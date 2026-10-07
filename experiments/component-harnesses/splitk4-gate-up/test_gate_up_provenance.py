import copy
import hashlib
import json
import unittest
from unittest.mock import patch

import gate_up_provenance as p


def require(value, message):
    if not value:
        raise ValueError(message)


class ProvenanceTests(unittest.TestCase):
    def fixture(self):
        plan = {'images': {arm: {'sha256': digest} for arm, digest in p.IMAGE_PINS.items()},
                'files': {'images/' + arm + '.hsaco': {'bytes': size} for arm, size in p.SIZES.items()}}
        docs, payloads = {}, {}
        for arm in p.IMAGE_PINS:
            docs[arm + '/observation.json'] = {
                'schema': 'EngineeringHsacoObservationV1', 'authority': 'none',
                'target': 'gfx950:xnack-', 'code_object_version': 6,
                'grants': {'publication': False, 'load': False, 'launch': False},
                'hsaco': {'identity': {'sha256': p.IMAGE_PINS[arm], 'byte_len': p.SIZES[arm]},
                          'kernel_names': sorted(p.SYMBOLS[arm])},
                'execution': {'exact_output_replay': True}, 'tools': {'fixture_arm': arm},
                'compiler_handoff': {'sha256': '0' * 64, 'byte_len': 54782}}
        capture = {'state': 'payload-complete', 'requires_cli_completion_acknowledgement': True,
            'observation': {'sha256': p.PINS['splitk4/observation.json'], 'byte_len': 2836},
            'grants': docs['splitk4/observation.json']['grants'],
            'source_authentication': False, 'proof_authority': False}
        payload_ids = {}
        for name, (field, size) in p.PAYLOADS.items():
            payloads['splitk4/payload/' + name] = b'x' * size
            payload_ids[name] = {'sha256': hashlib.sha256(b'x' * size).hexdigest(), 'byte_len': size}
            capture[field] = payload_ids[name]
        docs['splitk4/observation.json']['compiler_handoff'] = capture['compiler_handoff']
        docs['splitk4/capture.json'] = capture
        source = {name.removeprefix('splitk4/source/'): value for name, value in p.PINS.items()
                  if name.startswith('splitk4/source/')}
        docs['splitk4/isa/receipt.json'] = {
            'schema': 'FerricSplitK4GateUpIsaInspectionInputsV1', 'accepted': True,
            'abi_admitted': False, 'native_correctness': False, 'native_performance': False,
            'capture_state': 'payload-complete', 'payloads': payload_ids,
            'image': docs['splitk4/observation.json']['hsaco']['identity'],
            'emission_inner_sha256': p.PINS['splitk4/emit/inner.json'],
            'emission_outer_sha256': p.PINS['splitk4/emit/outer.json'], 'source': source}
        docs['splitk4/emit/inner.json'] = {
            'schema': 'FerricSplitK4GateUpStandardEmissionPhaseV1', 'accepted': True,
            'native_executed': False, 'sdk_revision': '55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9',
            'source_before': source, 'source_after': source,
            'compiler_provenance': '79a plus published private-access and witness-rank fixes',
            'vendor_receipt_sha256': p.PINS['splitk4/vendor/inner.json'],
            'vendor_outer_sha256': p.PINS['splitk4/vendor/outer.json'],
            'cpu_receipts': {role: [p.PINS['splitk4/' + role + '/inner.json'],
                                  p.PINS['splitk4/' + role + '/outer.json']] for role in p.HOST_ROLES}}
        for role in p.HOST_ROLES:
            docs['splitk4/' + role + '/inner.json'] = {
                'schema': 'FerricSplitK4GateUpHostQualificationV1', 'accepted': True, 'role': role,
                'native_executed': False, 'source_after': source,
                'source_before': {k: v for k, v in source.items() if role != 'prepare' or k != 'Cargo.lock'}}
        for role in (*p.HOST_ROLES, 'vendor', 'emit', 'inspect'):
            docs['splitk4/' + role + '/outer.json'] = {'status': 0, 'reason': 'completed', 'returncode': 0,
                'cleanup_ok': True, 'child_reaped': True, 'errors': [], 'term_sent': False,
                'kill_sent': False, 'log_limit_exceeded': False,
                'profile': 'FerricCpuFourCore40GiBEmitterV1', 'cpus': [0, 1, 2, 3],
                'nice': 19, 'build_jobs': 4, 'rust_test_threads': 1}
        return docs, payloads, plan

    def check(self, docs, payloads, plan):
        pins = {key: hashlib.sha256(b'x' * size).hexdigest()
                for name, (_, size) in p.PAYLOADS.items() for key in ['splitk4/payload/' + name]}
        with patch.dict(p.PINS, pins):
            p.validate_documents({**{name: json.dumps(value).encode() for name, value in docs.items()},
                                  **payloads}, plan, decode=json.loads, require=require)

    def test_distinct_emission_histories_and_complete_non_authoritative_capture(self):
        self.check(*self.fixture())

    def test_image_root_size_grants_and_compiler_drift(self):
        for fault in ('image', 'size', 'root', 'grants', 'replay', 'same-compiler'):
            docs, payloads, plan = self.fixture()
            obs = docs['splitk4/observation.json']
            if fault == 'image':
                plan['images']['wave']['sha256'] = '0' * 64
            elif fault == 'size':
                plan['files']['images/splitk4.hsaco']['bytes'] += 1
            elif fault == 'root':
                obs['hsaco']['kernel_names'] = [p.SYMBOLS['splitk4'][0]]
            elif fault == 'grants':
                obs['grants']['launch'] = True
            elif fault == 'replay':
                obs['execution']['exact_output_replay'] = False
            else:
                obs['tools'] = docs['wave/observation.json']['tools']
            with self.subTest(fault=fault), self.assertRaises(ValueError):
                self.check(docs, payloads, plan)

    def test_missing_corrupt_or_unacknowledged_capture_rejects(self):
        for name in p.PAYLOADS:
            docs, payloads, plan = self.fixture()
            payloads['splitk4/payload/' + name] = b'y' * len(payloads['splitk4/payload/' + name])
            with self.subTest(payload=name), self.assertRaises(ValueError):
                self.check(docs, payloads, plan)
        for field, value in (('state', 'omitted-ineligible'), ('proof_authority', True),
                             ('requires_cli_completion_acknowledgement', False)):
            docs, payloads, plan = self.fixture()
            docs['splitk4/capture.json'][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.check(docs, payloads, plan)

    def test_host_failure_source_drift_and_authority_promotion_reject(self):
        for name, field, value in (('test-enabled/inner.json', 'accepted', False),
                ('emit/inner.json', 'source_after', {}), ('emit/outer.json', 'term_sent', True),
                ('inspect/outer.json', 'profile', 'FerricCpuFourCore36GiBEmitterV1'),
                ('isa/receipt.json', 'abi_admitted', True), ('isa/receipt.json', 'native_correctness', True),
                ('isa/receipt.json', 'native_performance', True), ('prepare/inner.json', 'source_before', {})):
            docs, payloads, plan = self.fixture()
            docs['splitk4/' + name][field] = value
            with self.subTest(name=name, field=field), self.assertRaises(ValueError):
                self.check(docs, payloads, plan)

    def test_binding_checks_bytes_location_and_complete_roster(self):
        raw = {name: name.encode() for name in p.PINS}
        pins = {name: hashlib.sha256(value).hexdigest() for name, value in raw.items()}
        stage = '/dev/shm/ferric-c1-splitk4-component-latency-fixture'
        evidence = {name: {'path': stage + '/provenance/' + name, 'sha256': value} for name, value in pins.items()}

        def check(items):
            p.validate_review(items, {'stage': stage},
                read=lambda path, *args: (raw[path.removeprefix(stage + '/provenance/')], '0' * 64),
                staged_binding=lambda item, _: item, decode=json.loads, require=require)

        with patch.object(p, 'PINS', pins), patch.object(p, 'validate_documents') as semantic:
            check(evidence)
            self.assertEqual(semantic.call_count, 1)
            missing = dict(evidence)
            del missing['splitk4/capture.json']
            with self.assertRaises(ValueError):
                check(missing)
            changed = copy.deepcopy(evidence)
            changed['splitk4/capture.json']['path'] = '/tmp/escape'
            with self.assertRaises(ValueError):
                check(changed)
            raw['splitk4/capture.json'] += b'changed'
            with self.assertRaises(ValueError):
                check(evidence)


if __name__ == '__main__':
    unittest.main()
