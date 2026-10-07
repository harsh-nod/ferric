import copy
import hashlib
import json
import unittest
from unittest.mock import patch

import k16_provenance as p


def require(value, message):
    if not value:
        raise ValueError(message)


class ProvenanceTests(unittest.TestCase):
    def fixture(self):
        plan = {'images': {arm: {'sha256': digest} for arm, digest in p.IMAGE_PINS.items()},
                'files': {'images/' + arm + '.hsaco': {'bytes': size} for arm, size in p.SIZES.items()}}
        source = {name.removeprefix('source/'): digest for name, digest in p.PINS.items()
                  if name.startswith('source/')}
        obs = {'schema': 'EngineeringHsacoObservationV1', 'authority': 'none',
            'namespace': 'fe2o3-engineering-v1', 'crate_name': 'ferric_qwen3_tp_prefill_k2_kernels_device_v1',
            'target': 'gfx950:xnack-', 'code_object_version': 6,
            'grants': {'publication': False, 'load': False, 'launch': False}, 'providers': [],
            'hsaco': {'identity': {'sha256': p.IMAGE_SHA, 'byte_len': 23272},
                      'kernel_names': sorted(p.SYMBOLS.values())},
            'execution': {'exact_output_replay': True},
            'options': {'optimization': 'O2', 'strip_debug': True, 'verify_each': True,
                        'timeout_seconds': 600, 'maximum_output_bytes': 4194304},
            'tools': {'extractor_backend': {'sha256': '94d663de7150d25e3449778df1618e5b274e5d1a8b981c47ad01c7d7dd9a51bf'},
                      'worker': {'executable': {'sha256': 'c6c92db6158bdab5a87f46c5a7b08907d51ded378babd88dc2257c0852415fe1'}}},
            'compiler_handoff': {'sha256': 'f52669142a51851087a699f393786da5ca3f0ca574a225cfcb9a91990e31ccb4',
                                 'byte_len': 128408}}
        docs = {'observation.json': obs}
        docs['capture.json'] = {'schema': 'EngineeringDiagnosticCaptureV1',
            'state': 'omitted-ineligible', 'kir': None, 'llvm': None,
            'observation': {'sha256': p.PINS['observation.json'], 'byte_len': 2815},
            'compiler_handoff': obs['compiler_handoff'], 'grants': obs['grants'],
            'source_authentication': False, 'proof_authority': False}
        authored = dict(source)
        authored['device/qwen3-tp-prefill-k2-kernels-v1/Cargo.lock'] = '0' * 64
        docs['source-frozen.json'] = {'formatted_locked': source, 'authored': authored,
            'source_archive_sha256': 'fdf979240b5a5963e315fd293205c59656b1395a70669620324dfb5040ce4387'}
        docs['isa/receipt.json'] = {'schema': 'FerricMatchedK16PrefillIsaInspectionInputsV1',
            'accepted': True, 'abi_admitted': False, 'native_correctness': False, 'native_performance': False,
            'helper_sha256': p.INSPECTOR_HELPER_SHA, 'capture_state': 'omitted-ineligible', 'payloads': {},
            'image': obs['hsaco']['identity'], 'observation': docs['capture.json']['observation'],
            'emission_inner_sha256': p.PINS['emit/inner.json'],
            'emission_outer_sha256': p.PINS['emit/outer.json'], 'source': source,
            'outputs': {name: p.PINS['isa/' + name] for name in ('disassembly.txt', 'metadata.txt', 'rodata.txt')}}
        docs['emit/inner.json'] = {'schema': 'FerricPrefillK2CurrentStandardEmissionPhaseV1',
            'accepted': True, 'role': 'emit', 'helper_sha256': p.EMITTER_HELPER_SHA,
            'native_executed': False, 'sdk_revision': p.REVISION, 'source_before': source, 'source_after': source,
            'compiler_provenance': '79a plus published private-access and witness-rank fixes',
            'vendor_receipt_sha256': p.PINS['vendor/inner.json'], 'vendor_outer_sha256': p.PINS['vendor/outer.json'],
            'cpu_receipts': {role: [p.PINS[role + '/inner.json'], p.PINS[role + '/outer.json']]
                             for role in p.HOST_ROLES}}
        docs['vendor/inner.json'] = {'accepted': True, 'role': 'vendor', 'helper_sha256': p.EMITTER_HELPER_SHA,
            'native_executed': False, 'source_before': source, 'source_after': source}
        for role in p.HOST_ROLES:
            docs[role + '/inner.json'] = {'schema': 'FerricMatchedK16PrefillHostQualificationV1',
                'accepted': True, 'role': role, 'helper_sha256': p.HOST_HELPER_SHA, 'native_executed': False,
                'source_before': authored if role == 'prepare' else source, 'source_after': source}
        for role in (*p.HOST_ROLES, 'vendor', 'emit', 'inspect'):
            docs[role + '/outer.json'] = {'status': 0, 'reason': 'completed', 'returncode': 0,
                'cleanup_ok': True, 'child_reaped': True, 'errors': [], 'term_sent': False,
                'kill_sent': False, 'log_limit_exceeded': False,
                'profile': 'FerricCpuFourCore40GiBEmitterV1', 'cpus': [0, 1, 2, 3],
                'nice': 19, 'build_jobs': 4, 'rust_test_threads': 1}
        raw = {name: json.dumps(value).encode() for name, value in docs.items()}
        for role, count in (('test-default', 7), ('test-enabled', 8)):
            raw[role + '/stdout'] = f'test result: ok. {count} passed; 0 failed; 0 ignored;\n'.encode()
        return raw, plan

    def check(self, raw, plan):
        p.validate_documents(raw, plan, decode=json.loads, require=require)

    def mutate(self, raw, name, fn):
        doc = json.loads(raw[name])
        fn(doc)
        raw[name] = json.dumps(doc).encode()

    def test_same_image_two_root_emission_and_prepare_transition(self):
        self.check(*self.fixture())

    def test_either_image_size_or_bytes_mismatch_rejects(self):
        for arm in p.SYMBOLS:
            for field in ('sha', 'size'):
                raw, plan = self.fixture()
                if field == 'sha':
                    plan['images'][arm]['sha256'] = '0' * 64
                else:
                    plan['files']['images/' + arm + '.hsaco']['bytes'] += 1
                with self.subTest(arm=arm, field=field), self.assertRaises(ValueError):
                    self.check(raw, plan)

    def test_missing_extra_duplicate_swapped_and_stale_roots_reject(self):
        roots = sorted(p.SYMBOLS.values())
        for bad in (roots[:1], roots + ['extra'], [roots[0]] * 2, roots[::-1], ['v5', 'm2']):
            raw, plan = self.fixture()
            self.mutate(raw, 'observation.json', lambda doc: doc['hsaco'].update(kernel_names=bad))
            with self.subTest(roots=bad), self.assertRaises(ValueError):
                self.check(raw, plan)

    def test_capture_inspection_and_compiler_authority_reject(self):
        for name, field, bad in (('capture.json', 'kir', {'fake': True}),
                ('capture.json', 'state', 'payload-complete'), ('capture.json', 'proof_authority', True),
                ('isa/receipt.json', 'abi_admitted', True), ('isa/receipt.json', 'native_correctness', True),
                ('isa/receipt.json', 'native_performance', True), ('isa/receipt.json', 'payloads', {'fake': {}}),
                ('emit/inner.json', 'compiler_provenance', 'current main rebuild')):
            raw, plan = self.fixture()
            self.mutate(raw, name, lambda doc: doc.update({field: bad}))
            with self.subTest(name=name, field=field), self.assertRaises(ValueError):
                self.check(raw, plan)

    def test_host_failure_source_drift_and_old_receipts_reject(self):
        for name, field, bad in (('test-enabled/inner.json', 'schema', 'FerricM32HostQualificationV1'),
                ('prepare/inner.json', 'source_before', {}), ('test-enabled/inner.json', 'source_before', {}),
                ('clippy-default/inner.json', 'accepted', False), ('emit/inner.json', 'source_after', {}),
                ('vendor/inner.json', 'source_after', {}), ('emit/outer.json', 'term_sent', True),
                ('inspect/outer.json', 'profile', 'FerricCpuFourCore36GiBEmitterV1')):
            raw, plan = self.fixture()
            self.mutate(raw, name, lambda doc: doc.update({field: bad}))
            with self.subTest(name=name, field=field), self.assertRaises(ValueError):
                self.check(raw, plan)

    def test_failed_skipped_or_incomplete_tests_reject(self):
        for bad in (b'test result: ok. 7 passed; 0 failed; 0 ignored;',
                    b'test result: ok. 8 passed; 1 failed; 0 ignored;',
                    b'test result: ok. 8 passed; 0 failed; 1 ignored;', b''):
            raw, plan = self.fixture()
            raw['test-enabled/stdout'] = bad
            with self.subTest(raw=bad), self.assertRaises(ValueError):
                self.check(raw, plan)

    def test_binding_checks_bytes_location_and_complete_roster(self):
        raw = {name: name.encode() for name in p.PINS}
        pins = {name: hashlib.sha256(value).hexdigest() for name, value in raw.items()}
        stage = '/dev/shm/ferric-prefill-k16-component-latency-fixture'
        evidence = {name: {'path': stage + '/provenance/' + name, 'sha256': value} for name, value in pins.items()}
        def check(items):
            p.validate_review(items, {'stage': stage},
                read=lambda path, *args: (raw[path.removeprefix(stage + '/provenance/')], '0' * 64),
                staged_binding=lambda item, _: item, decode=json.loads, require=require)
        with patch.object(p, 'PINS', pins), patch.object(p, 'validate_documents') as semantic:
            check(evidence)
            self.assertEqual(semantic.call_count, 1)
            missing = dict(evidence)
            del missing['capture.json']
            with self.assertRaises(ValueError):
                check(missing)
            changed = copy.deepcopy(evidence)
            changed['capture.json']['path'] = '/tmp/escape'
            with self.assertRaises(ValueError):
                check(changed)
            raw['capture.json'] += b'changed'
            with self.assertRaises(ValueError):
                check(evidence)
