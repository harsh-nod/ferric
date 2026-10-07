import io
import json
from pathlib import Path
import tarfile
import tempfile
import types
import unittest
from unittest.mock import patch

import component_contract as c
import prepare_gate_up_bundle as p

D = Path(__file__).resolve().parent


def archive(files):
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode='w:gz') as packed:
        for name, raw in files.items():
            item = tarfile.TarInfo(name)
            item.size = len(raw)
            packed.addfile(item, io.BytesIO(raw))
    return buffer.getvalue()


class BundleTests(unittest.TestCase):
    def fixture(self, root):
        files = {name: (D / name).read_bytes() for name in set(c.PINS) | set(c.OWN_SOURCES)}
        def put(name, raw):
            path = root / name
            path.write_bytes(raw)
            return {'path': str(path), 'sha256': p.digest(raw)}
        source = put('source.tar.gz', archive(files))
        evidence = put('evidence', b'evidence')
        worker = put('worker', b'worker')
        runtime = types.SimpleNamespace(WORKER_SHA=worker['sha256'], WORKER_BYTES=6,
            REVISION='fixture-only', PINS={'fixture': evidence['sha256']})
        provenance = types.SimpleNamespace(PINS={'fixture': evidence['sha256']})
        images = {arm: put(arm + '.hsaco', arm.encode()) for arm in ('wave', 'splitk4')}
        pins = {arm: item['sha256'] for arm, item in images.items()}
        phase = {key: put(key, b'' if key == 'stdout' else b'fixture')
                 for key in ('status', 'result', 'stdout', 'stderr', 'helper', 'inner')}
        phase['source_archive'] = source
        config = {'schema': 'FerricSplitK4GateUpComponentPortableInputsV1', 'source_archive': source,
            'stage': '/dev/shm/ferric-c1-splitk4-component-latency-fixture', 'mode': 'latency',
            'worker': worker, 'python': {'path': '/usr/bin/python3.12', 'sha256': '0' * 64},
            'runtime_evidence': {'fixture': evidence}, 'image_evidence': {'fixture': evidence},
            'cpu_phase': phase, 'images': images}
        return config, runtime, provenance, pins

    def run_preparation(self, root, config, runtime, provenance, pins):
        def load(path):
            return {'component_contract.py': c, 'runtime_binding.py': runtime,
                    'gate_up_provenance.py': provenance}[path.name]
        with patch.object(p, 'load_module', side_effect=load), patch.object(c, 'IMAGE_PINS', pins), \
                patch.object(c, 'validate_review') as review:
            receipt = p.prepare(config, root / 'out')
        self.assertEqual(review.call_count, 1)
        return receipt

    def test_explicit_roster_remap_and_review_gate(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config, *mocks = self.fixture(root)
            result = self.run_preparation(root, config, *mocks)
            plan = json.loads((root / 'out/payload/plan.json').read_bytes())
            review = json.loads((root / 'out/payload/review.json').read_bytes())
            self.assertFalse(result['launched'])
            self.assertFalse(result['native_admitted'])
            self.assertFalse(result['control_compiler_matches_candidate'])
            self.assertEqual(set(plan['images']), {'wave', 'splitk4'})
            self.assertEqual(plan['worker']['path'], config['stage'] + '/worker-candidate')
            self.assertEqual(review['cpu_phase']['source_archive']['path'],
                             config['stage'] + '/qualification/harness/source_archive')
            self.assertNotIn('cpu_phases', review)
            self.assertFalse(any(name.startswith(('paired/', 'history/', 'qualification/component-tests'))
                                 for name in plan['files']))
            with self.assertRaises(FileExistsError):
                self.run_preparation(root, config, *mocks)

    def test_image_source_worker_and_evidence_changes_refuse(self):
        for fault in ('mode', 'image', 'source', 'worker', 'evidence', 'phase'):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                config, *mocks = self.fixture(root)
                if fault == 'mode':
                    config['mode'] = 'ticks'
                elif fault == 'image':
                    config['images']['wave'] = {'path': config['images']['wave']['path'], 'sha256': '0' * 64}
                elif fault == 'source':
                    config['source_archive'] = {**config['source_archive'], 'sha256': '0' * 64}
                elif fault == 'worker':
                    config['worker'] = {**config['worker'], 'sha256': '0' * 64}
                elif fault == 'evidence':
                    config['image_evidence'] = {}
                else:
                    config['cpu_phase']['unexpected_old_credit'] = {}
                with self.assertRaises(ValueError):
                    self.run_preparation(root, config, *mocks)

    def test_input_byte_hash_and_symlink_reject(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / 'input'
            path.write_bytes(b'input')
            item = {'path': str(path), 'sha256': p.digest(b'input')}
            self.assertEqual(p.input_bytes(item), b'input')
            path.write_bytes(b'drift')
            with self.assertRaises(ValueError):
                p.input_bytes(item)
            alias = root / 'alias'
            alias.symlink_to(path)
            with self.assertRaises(ValueError):
                p.input_bytes({'path': str(alias), 'sha256': p.digest(b'drift')})

    def test_source_archive_refuses_escape_duplicate_and_non_python(self):
        self.assertEqual(p.source_files(archive({'ok.py': b'value = 1'})), {'ok.py': b'value = 1'})
        for name in ('../escape.py', '/absolute.py', 'file.txt'):
            with self.subTest(name=name), self.assertRaises(ValueError):
                p.source_files(archive({name: b'value = 1'}))
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode='w:gz') as packed:
            for _ in range(2):
                info = tarfile.TarInfo('duplicate.py')
                info.size = 1
                packed.addfile(info, io.BytesIO(b'x'))
        with self.assertRaises(ValueError):
            p.source_files(buffer.getvalue())

    def test_json_duplicates_and_nonfinite_refuse(self):
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}'):
            with self.assertRaises(ValueError):
                p.decode(raw)
