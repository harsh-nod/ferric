import copy
import hashlib
import io
import json
from pathlib import Path
import tarfile
import tempfile
import types
import unittest
from unittest.mock import patch

import component_contract as contract
import prepare_ordered_bundle as bundle
import runtime_binding as runtime


class BundleTests(unittest.TestCase):
    def test_default_disabled_before_any_input_read(self):
        self.assertFalse(bundle.ENABLED)
        with patch.object(bundle, 'input_bytes', side_effect=AssertionError('input read')):
            with self.assertRaisesRegex(ValueError, 'not qualified/enabled'):
                bundle.prepare({}, Path('/unused'))

    def test_closed_json_and_nonfinite_rejected(self):
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}'):
            with self.assertRaises(ValueError):
                bundle.decode(raw)

    def test_relative_names_reject_escape_and_noncanonical(self):
        for name in ('/tmp/a', '../a', 'a/../b', 'a//b', '.', ''):
            with self.subTest(name=name), self.assertRaises(ValueError):
                bundle.relative(name)

    def test_archive_duplicate_symlink_and_traversal_rejected(self):
        for variant in ('duplicate', 'symlink', 'traversal', 'non-python'):
            buffer = io.BytesIO()
            with tarfile.open(fileobj=buffer, mode='w:gz') as packed:
                for index in range(2 if variant == 'duplicate' else 1):
                    name = '../evil.py' if variant == 'traversal' else ('a.txt' if variant == 'non-python' else 'a.py')
                    item = tarfile.TarInfo(name)
                    if variant == 'symlink':
                        item.type, item.linkname = tarfile.SYMTYPE, '/tmp/evil'
                        packed.addfile(item)
                    else:
                        item.size = 1
                        packed.addfile(item, io.BytesIO(b'x'))
            with self.subTest(variant=variant), self.assertRaises(ValueError):
                bundle.source_files(buffer.getvalue())

    def test_input_reader_hash_size_and_symlink(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / 'input'
            path.write_bytes(b'abc')
            binding = {'path': str(path), 'sha256': bundle.digest(b'abc')}
            self.assertEqual(bundle.input_bytes(binding), b'abc')
            with self.assertRaises(ValueError):
                bundle.input_bytes(binding, maximum=2)
            with self.assertRaises(ValueError):
                bundle.input_bytes({**binding, 'sha256': '0' * 64})
            (root / 'link').symlink_to(path)
            with self.assertRaises(ValueError):
                bundle.input_bytes({**binding, 'path': str(root / 'link')})

    def test_inherited_compiler_and_images_not_old_runtime_or_sources(self):
        names = ['compiler/worker', 'images/v5.hsaco', 'qualification/component-tests/stdout',
                 'qualification/supervisor-tests/result', 'worker-candidate', 'review.json', 'old.py']
        base = {'files': dict.fromkeys(names), 'sources': {'old.py': 'pin'}}
        self.assertEqual(bundle.inherited_names(base), names[:3])

    def test_remap_only_exact_path_prefix(self):
        value = {'a': ['/old/a', '/older/a', '/old', 3]}
        self.assertEqual(bundle.remap(value, '/old', '/new'), {'a': ['/new/a', '/older/a', '/new', 3]})

    def test_mode_namespace_and_legacy_namespace_refused(self):
        for mode in contract.MODES:
            self.assertEqual(contract.stage_name('/dev/shm/ferric-v16-splitk-ordered-' + mode + '-a1').parent,
                             Path('/dev/shm'))
        for name in ('/dev/shm/ferric-v16-splitk-component-a1',
                     '/dev/shm/ferric-v16-splitk-ordered-ordinary-a1'):
            with self.assertRaises(ValueError):
                contract.stage_name(name)

    def test_portable_preparation_mode_runtime_and_compiler_preservation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, observed = Path(temporary), []
            base_root = root / 'base'
            base_root.mkdir()
            files = {}

            def put(parent, name, raw):
                path = parent / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(raw)
                path.chmod(0o600)
                return {'path': str(path), 'sha256': bundle.digest(raw)}

            for name, raw in {'compiler/llvm-worker-proof': b'compiler unchanged',
                              'images/v5.hsaco': b'v5', 'images/candidate.hsaco': b'candidate'}.items():
                item = put(base_root, name, raw)
                files[name] = {'sha256': item['sha256'], 'bytes': len(raw), 'mode': 0o600}
            old_stage = '/dev/shm/ferric-v16-splitk-component-old'
            image_pins = {name: files['images/' + name + '.hsaco']['sha256'] for name in ('v5', 'candidate')}
            review = {'images': {}, 'cpu_phases': {'component-tests': {'unchanged': True}}}
            review_binding = put(base_root, 'review.json', bundle.encoded(review))
            base = {'schema': 'FerricSplitKComponentLaunchPlanV1', 'stage': old_stage,
                'files': files, 'sources': {}, 'review': review_binding, 'python': {'path': '/usr/bin/python3'},
                'images': {name: {'path': old_stage + '/images/' + name + '.hsaco', 'sha256': digest}
                           for name, digest in image_pins.items()}}
            base_binding = put(base_root, 'plan.json', bundle.encoded(base))
            source = put(root, 'source.tar.gz', b'synthetic source archive')
            worker = put(root, 'new-worker', b'new runtime')
            evidence = put(root, 'proof', b'exact retained proof')
            phase = {name: put(root, 'phase/' + name, b'' if name == 'stdout' else b'fixture')
                     for name in ('status', 'result', 'stdout', 'stderr', 'helper', 'inner')}
            phase.update(source_archive=source, source_directory='/cpu/source', argv_sha256='0' * 64)
            config = {'schema': 'FerricOrderedComponentPortableInputsV1', 'base_payload': str(base_root),
                'source_archive': source, 'stage': '/dev/shm/ferric-v16-splitk-ordered-latency-a1',
                'mode': 'latency', 'worker': worker, 'runtime_evidence': {'proof': evidence},
                'supervisor_phase': phase}
            runtime_stub = types.SimpleNamespace(WORKER_SHA=worker['sha256'], WORKER_BYTES=11,
                PINS={'proof': evidence['sha256']}, REVISION=runtime.REVISION)
            c = types.SimpleNamespace(PINS={}, OWN_SOURCES=('component_contract.py', 'runtime_binding.py'),
                DEVICE=31, stage_name=contract.stage_name, verify_files=lambda *args: observed.append('files'),
                validate_review=lambda value, plan: observed.append((value['mode'], value['worker_sha256'])))
            with patch.object(bundle, 'ENABLED', True), patch.object(bundle, 'BASE_PLAN_SHA', base_binding['sha256']), \
                    patch.object(bundle, 'BASE_REVIEW_SHA', review_binding['sha256']), \
                    patch.object(bundle, 'IMAGE_PINS', image_pins), \
                    patch.object(bundle, 'source_files', return_value={name: b'fixture' for name in c.OWN_SOURCES}), \
                    patch.object(bundle, 'load_module', side_effect=lambda path: c if path.name == 'component_contract.py' else runtime_stub):
                result = bundle.prepare(config, root / 'prepared')
                with self.assertRaises(FileExistsError):
                    bundle.prepare(config, root / 'prepared')
            payload = root / 'prepared/payload'
            self.assertEqual((payload / 'compiler/llvm-worker-proof').read_bytes(), b'compiler unchanged')
            self.assertEqual((payload / 'worker-candidate').read_bytes(), b'new runtime')
            plan = json.loads((payload / 'plan.json').read_bytes())
            self.assertEqual(plan['mode'], 'latency')
            self.assertEqual(plan['worker']['path'], config['stage'] + '/worker-candidate')
            self.assertEqual(observed, ['files', ('latency', worker['sha256'])])
            self.assertFalse(result['launched'])
            self.assertFalse(result['native_admitted'])


class RuntimeTests(unittest.TestCase):
    def fixture(self):
        clean = {'status': 0, 'reason': 'completed', 'returncode': 0, 'cleanup_ok': True,
            'child_reaped': True, 'errors': [], 'term_sent': False, 'kill_sent': False,
            'log_limit_exceeded': False}
        raw = {'full-result': bundle.encoded(clean), 'full-status': b'0\n',
               'full-stdout': b'test result: ok. 734 passed; 0 failed; 3 ignored;\n'
                              b'test result: ok. 9 passed; 0 failed; 0 ignored;\n'}
        pins = {name: bundle.digest(value) for name, value in raw.items()}
        evidence = {name: {'path': '/stage/runtime/' + name, 'sha256': value} for name, value in pins.items()}
        plan = {'stage': '/stage', 'worker': {'sha256': runtime.WORKER_SHA},
                'files': {'worker-candidate': {'bytes': runtime.WORKER_BYTES}}}
        return raw, pins, evidence, plan

    def check(self, raw, pins, evidence, plan):
        with patch.object(runtime, 'PINS', pins):
            runtime.validate_review(evidence, plan,
                read=lambda path, *args: (raw[Path(path).name], ''), staged_binding=lambda value, _: value,
                decode=bundle.decode, require=bundle.require)

    def test_exact_worker_and_clean_raw_evidence(self):
        self.check(*self.fixture())
        raw, pins, evidence, plan = self.fixture()
        plan['worker']['sha256'] = '0' * 64
        with self.assertRaises(ValueError):
            self.check(raw, pins, evidence, plan)

    def test_runtime_stale_evidence_status_and_roster_rejected(self):
        for mutation in ('missing', 'raw', 'status', 'path', 'size'):
            raw, pins, evidence, plan = self.fixture()
            if mutation == 'missing':
                evidence.pop('full-status')
            elif mutation == 'raw':
                raw['full-stdout'] += b'changed'
            elif mutation == 'status':
                raw['full-status'] = b'1\n'
                pins['full-status'] = evidence['full-status']['sha256'] = bundle.digest(b'1\n')
            elif mutation == 'path':
                evidence['full-status']['path'] = '/other/full-status'
            else:
                plan['files']['worker-candidate']['bytes'] += 1
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.check(raw, pins, evidence, plan)

    def test_unclean_terminal_and_incomplete_raw_tests_rejected(self):
        for name, raw_value in (('full-result', bundle.encoded({'status': 0})),
                                ('full-stdout', b'test result: ok. 9 passed; 0 failed; 0 ignored;')):
            raw, pins, evidence, plan = self.fixture()
            raw[name] = raw_value
            pins[name] = evidence[name]['sha256'] = bundle.digest(raw_value)
            with self.assertRaises(ValueError):
                self.check(raw, pins, evidence, plan)
