import copy
import hashlib
import json
import unittest
from unittest.mock import patch

import m32_provenance as p


def require(value, message):
    if not value:
        raise ValueError(message)


class ProvenanceTests(unittest.TestCase):
    def fixture(self):
        plan = {'images': {arm: {'sha256': digest} for arm, digest in p.IMAGE_PINS.items()},
                'files': {'images/' + arm + '.hsaco': {'bytes': size} for arm, size in p.SIZES.items()}}
        docs = {}
        for arm in ('v5', 'm2'):
            docs[arm + '/observation.json'] = {
                'schema': 'EngineeringHsacoObservationV1', 'authority': 'none',
                'target': 'gfx950:xnack-', 'code_object_version': 6,
                'grants': {'publication': False, 'load': False, 'launch': False},
                'hsaco': {'identity': {'sha256': p.IMAGE_PINS[arm], 'byte_len': p.SIZES[arm]},
                          'kernel_names': [p.SYMBOLS[arm]]},
                'execution': {'exact_output_replay': True}, 'tools': {'fixture_arm': arm},
                'compiler_handoff': {'sha256': '0' * 64, 'byte_len': 79085}}
        docs['m2/capture.json'] = {'state': 'omitted-ineligible', 'kir': None, 'llvm': None,
            'observation': {'sha256': p.PINS['m2/observation.json'], 'byte_len': 2781},
            'compiler_handoff': docs['m2/observation.json']['compiler_handoff'],
            'grants': docs['m2/observation.json']['grants'], 'source_authentication': False,
            'proof_authority': False}
        source = {name.removeprefix('m2/source/'): value for name, value in p.PINS.items()
                  if name.startswith('m2/source/')}
        docs['m2/isa/receipt.json'] = {'schema': 'FerricM32IsaInspectionInputsV1', 'accepted': True,
            'abi_admitted': False, 'native_correctness': False, 'native_performance': False,
            'capture_state': 'omitted-ineligible', 'payloads': {},
            'image': docs['m2/observation.json']['hsaco']['identity'],
            'emission_inner_sha256': p.PINS['m2/emit/inner.json'],
            'emission_outer_sha256': p.PINS['m2/emit/outer.json'], 'source': source}
        docs['m2/emit/inner.json'] = {'schema': 'FerricM32StandardEmissionPhaseV1', 'accepted': True,
            'native_executed': False, 'sdk_revision': '55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9',
            'source_before': source, 'source_after': source,
            'compiler_provenance': '79a plus published private-access and witness-rank fixes',
            'cpu_receipts': {role + '-a002': [p.PINS['m2/' + role + '/inner.json'],
                            p.PINS['m2/' + role + '/outer.json']] for role in p.HOST_ROLES}}
        for role in p.HOST_ROLES:
            docs['m2/' + role + '/inner.json'] = {
                'schema': 'FerricM32HostQualificationV1', 'accepted': True, 'role': role,
                'native_executed': False, 'cold_attempt_accepted': False, 'warm_followup': True,
                'source_before': source, 'source_after': source}
        for role in (*p.HOST_ROLES, 'emit', 'inspect'):
            docs['m2/' + role + '/outer.json'] = {'status': 0, 'reason': 'completed', 'returncode': 0,
                'cleanup_ok': True, 'child_reaped': True, 'errors': [], 'term_sent': False,
                'kill_sent': False, 'log_limit_exceeded': False,
                'profile': 'FerricCpuFourCore40GiBEmitterV1', 'cpus': [0, 1, 2, 3],
                'nice': 19, 'build_jobs': 4, 'rust_test_threads': 1}
        return docs, plan

    def check(self, docs, plan):
        p.validate_documents({name: json.dumps(value).encode() for name, value in docs.items()},
                             plan, decode=json.loads, require=require)

    def test_distinct_emission_histories_and_non_authority(self):
        self.check(*self.fixture())

    def test_wrong_image_size_root_grants_and_replay_reject(self):
        for fault in ('image', 'size', 'root', 'grants', 'replay', 'same-compiler'):
            docs, plan = self.fixture()
            obs = docs['m2/observation.json']
            if fault == 'image':
                plan['images']['v5']['sha256'] = 'bbe68bc260c94dbfdd1b76a0e0ac5c44d0a182996d7b6cd94a5f66cfb5fc4b7c'
            elif fault == 'size':
                plan['files']['images/m2.hsaco']['bytes'] += 1
            elif fault == 'root':
                obs['hsaco']['kernel_names'] = ['partial_only']
            elif fault == 'grants':
                obs['grants']['launch'] = True
            elif fault == 'replay':
                obs['execution']['exact_output_replay'] = False
            else:
                obs['tools'] = docs['v5/observation.json']['tools']
            with self.subTest(fault=fault), self.assertRaises(ValueError):
                self.check(docs, plan)

    def test_capture_and_inspection_cannot_be_promoted(self):
        for name, field, value in (('m2/capture.json', 'kir', {'fake': True}),
                ('m2/capture.json', 'proof_authority', True),
                ('m2/isa/receipt.json', 'abi_admitted', True),
                ('m2/isa/receipt.json', 'native_correctness', True),
                ('m2/isa/receipt.json', 'native_performance', True)):
            docs, plan = self.fixture()
            docs[name][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.check(docs, plan)

    def test_host_failure_source_drift_and_profile_relabel_reject(self):
        for name, field, value in (('m2/test-enabled/inner.json', 'cold_attempt_accepted', True),
                ('m2/clippy-default/inner.json', 'accepted', False),
                ('m2/emit/inner.json', 'source_after', {}),
                ('m2/emit/outer.json', 'term_sent', True),
                ('m2/inspect/outer.json', 'profile', 'FerricCpuFourCore36GiBEmitterV1')):
            docs, plan = self.fixture()
            docs[name][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.check(docs, plan)

    def test_binding_checks_real_bytes_location_and_complete_roster(self):
        raw = {name: name.encode() for name in p.PINS}
        pins = {name: hashlib.sha256(value).hexdigest() for name, value in raw.items()}
        stage = '/dev/shm/ferric-prefill-m32-component-latency-fixture'
        evidence = {name: {'path': stage + '/provenance/' + name, 'sha256': value} for name, value in pins.items()}
        def check(items):
            p.validate_review(items, {'stage': stage},
                read=lambda path, *args: (raw[path.removeprefix(stage + '/provenance/')], '0' * 64),
                staged_binding=lambda item, _: item, decode=json.loads, require=require)
        with patch.object(p, 'PINS', pins), patch.object(p, 'validate_documents') as semantic:
            check(evidence)
            self.assertEqual(semantic.call_count, 1)
            missing = dict(evidence)
            del missing['m2/capture.json']
            with self.assertRaises(ValueError):
                check(missing)
            changed = copy.deepcopy(evidence)
            changed['m2/capture.json']['path'] = '/tmp/escape'
            with self.assertRaises(ValueError):
                check(changed)
            raw['m2/capture.json'] += b'changed'
            with self.assertRaises(ValueError):
                check(evidence)
