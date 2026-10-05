"""Synthetic assembly boundary tests; no frozen intake or native execution."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import types
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location('observer_prepare', Path(__file__).with_name('prepare.py'))
P = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(P)
TOPICS = ('source_lineage', 'formal', 'isa', 'coherence', 'lifecycle', 'selected_device')


def keys(value, names):
    P.require(type(value) is dict and set(value) == set(names.split()), 'closed synthetic object')


def pin(name, digest='ab' * 32):
    return dict(path=str(P.E / name), bytes=123, sha256=digest)


def fixture():
    config = dict(schema='ferric-p228-projection-ar4-host-observation-assembly-inputs-v1',
        input_label='prefix-projection-ar4-host-observation-inputs-v228-v1',
        output_label='prefix-projection-ar4-host-observation-gpu-v228-v1', session='12' * 32,
        supervisor_tests=copy.deepcopy(P.PURE), parent_runtime_review=pin('parent-review.json'),
        worker_runtime_review=pin('worker-review.json'))
    notes = dict(schema='ferric-p228-projection-ar4-host-observation-root-notes-v1',
        configuration=copy.deepcopy(config), reviewed=True, authority='none', gpu_attempts=1,
        review_topics={k: 'Synthetic substantive review text for boundary tests only.' for k in TOPICS},
        notes='Synthetic root-authored test fixture; not a real approval.')
    I = types.SimpleNamespace(V=types.SimpleNamespace(keys=keys), TOPICS=TOPICS)
    return I, config, notes


def request():
    old_worker = pin('old-worker', '34' * 32)
    old_worker['sha256'] = list(bytes.fromhex(old_worker['sha256']))
    return dict(schema='FerricFiniteProjectionResidualDecodeRequestV1',
        projection_residual_image=pin('unchanged-projection'), decode=dict(
            schema='FerricFinitePrefixDecodeRequestV1', mode='autoregressive',
            device_ids=list(P.DEVICE_IDS), worker=old_worker, session=[1] * 32,
            evidence_directory=str(P.E / 'old-case/native'), source={'model_id': [8] * 32},
            prompt={'tokens': [9112, 67, 198]}, images={'copy': pin('unchanged-copy')},
            prefix_image=pin('unchanged-prefix'), tiles_image=pin('unchanged-silu'),
            dispatch_timeout_ms=10000, child_deadline_ms=3600000))


class AssemblyTests(unittest.TestCase):
    def test_request_only_three_fields_and_lossless_u64(self):
        original = request(); before = copy.deepcopy(original)
        worker = pin('new-worker'); out = P.E / 'new-case'
        value = P.request_copy(original, worker, '12' * 32, out)
        self.assertEqual(original, before)
        self.assertEqual({k for k in original['decode'] if original['decode'][k] != value['decode'][k]},
                         {'worker', 'session', 'evidence_directory'})
        self.assertEqual(value['projection_residual_image'], original['projection_residual_image'])
        self.assertEqual(value['decode']['worker']['sha256'], list(bytes.fromhex(worker['sha256'])))
        restored = json.loads(json.dumps(value, sort_keys=True, allow_nan=False))
        self.assertEqual(restored['decode']['device_ids'], [16366993098680759275, 10838076764495710945])
        self.assertTrue(all(type(n) is int for n in restored['decode']['device_ids']))
        value['decode']['images']['copy']['bytes'] = 999
        self.assertEqual(original, before)
        self.assertNotIn('parent', value)

    def test_request_rejects_invalid_modes_ids_and_sessions(self):
        for field, value in [('mode', 'teacher_forced'), ('device_ids', [float(n) for n in P.DEVICE_IDS]),
                             ('device_ids', [True, P.DEVICE_IDS[1]]),
                             ('device_ids', [int(float(n)) for n in P.DEVICE_IDS])]:
            with self.subTest(field=field, value=value):
                old = request(); old['decode'][field] = value
                with self.assertRaises(RuntimeError):
                    P.request_copy(old, pin('new-worker'), '12' * 32, P.E / 'new-case')
        for session in ['0' * 64, 'G' * 64, '12', None]:
            with self.subTest(session=session), self.assertRaises(RuntimeError):
                P.request_copy(request(), pin('new-worker'), session, P.E / 'new-case')

    def test_request_requires_all_three_fresh_fields(self):
        old = request()
        same_worker = dict(old['decode']['worker'], sha256=bytes(old['decode']['worker']['sha256']).hex())
        for worker, session, out in [(same_worker, '12' * 32, P.E / 'new-case'),
                (pin('new-worker'), '01' * 32, P.E / 'new-case'),
                (pin('new-worker'), '12' * 32, P.E / 'old-case')]:
            with self.subTest(worker=worker, session=session, out=out), self.assertRaises(RuntimeError):
                P.request_copy(old, worker, session, out)

    def test_settings_accept_actual_pure_and_root_reviews(self):
        I, config, notes = fixture()
        self.assertIsNone(P.settings(I, config, notes))
        config['input_label'] = 'prefix-projection-ar4-host-observation-inputs-v228-v2'
        config['output_label'] = 'prefix-projection-ar4-host-observation-gpu-v228-v2'
        notes['configuration'] = copy.deepcopy(config)
        self.assertIsNone(P.settings(I, config, notes))

    def test_settings_refuses_relabelled_or_unspecified_reviews(self):
        cases = [('input_label', 'prefix-rope-indexed-ar4-inputs-v228-v1'),
                 ('output_label', 'prefix-rope-indexed-ar4-gpu-v228-v1'),
                 ('supervisor_tests', pin('wrong-pure.json')),
                 ('parent_runtime_review', dict(pin('parent-review.json'), bytes=True)),
                 ('parent_runtime_review', dict(pin('parent-review.json'), path='/tmp/review.json')),
                 ('parent_runtime_review', dict(pin('parent-review.json'), path=str(P.E / '../review.json'))),
                 ('parent_runtime_review', dict(pin('parent-review.json'), sha256='A' * 64)),
                 ('parent_runtime_review', pin('worker-review.json'))]
        for key, value in cases:
            I, config, notes = fixture(); config[key] = value; notes['configuration'] = copy.deepcopy(config)
            with self.subTest(key=key, value=value), self.assertRaises(RuntimeError):
                P.settings(I, config, notes)
        I, config, notes = fixture(); del config['worker_runtime_review']
        with self.assertRaises(RuntimeError): P.settings(I, config, notes)

    def test_notes_explicit_and_substantive(self):
        for key, value in [('reviewed', False), ('authority', 'production'), ('gpu_attempts', True),
                           ('gpu_attempts', 2), ('configuration', {}), ('notes', 'approved')]:
            I, config, notes = fixture(); notes[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError): P.settings(I, config, notes)
        for topics in [{}, {k: 'short' for k in TOPICS}]:
            I, config, notes = fixture(); notes['review_topics'] = topics
            with self.assertRaises(RuntimeError): P.settings(I, config, notes)

    def test_intake_aliases_restore_on_success_and_failed_import(self):
        aliases = ('layer_validation', 'prefix_contracts', 'observer_cpu', 'intake')
        absent = object(); initial = {k: sys.modules.get(k, absent) for k in aliases}
        try:
            for failure in (False, True):
                sys.modules.pop('layer_validation', None)
                sys.modules['prefix_contracts'] = None
                old_observer, old_intake = object(), object()
                sys.modules['observer_cpu'], sys.modules['intake'] = old_observer, old_intake
                bodies = {name + '.py': b'VALUE = 17\n' for name in aliases}
                bodies['intake.py'] = b'import observer_cpu\nVALUE = observer_cpu.VALUE\n'
                if failure:
                    bodies['observer_cpu.py'] = b'raise RuntimeError("synthetic dependency failure")\n'
                bodies.update({'extra%d.txt' % i: b'bounded fixture\n' for i in range(17)})
                rows = [dict(path=name, bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
                        for name, raw in bodies.items()]
                manifest = json.dumps(dict(schema='ferric-p228-projection-ar4-host-observation-gpu-package-v1',
                    pure_tests=91, files=rows)).encode()
                bodies['manifest.json'] = manifest

                def read(path):
                    raw = bodies[path.name]
                    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest()), raw

                with mock.patch.object(P, 'bytes_at', side_effect=read), mock.patch.object(
                        P, 'PACKAGE_SHA', hashlib.sha256(manifest).hexdigest()):
                    if failure:
                        with self.assertRaisesRegex(RuntimeError, 'synthetic dependency failure'):
                            with P.intake(): self.fail('failed dependency must not yield')
                    else:
                        with P.intake() as (module, pins):
                            self.assertEqual(module.VALUE, 17)
                            self.assertEqual(len(pins), 22)
                            self.assertIsNot(sys.modules['observer_cpu'], old_observer)
                self.assertNotIn('layer_validation', sys.modules)
                self.assertIsNone(sys.modules['prefix_contracts'])
                self.assertIs(sys.modules['observer_cpu'], old_observer)
                self.assertIs(sys.modules['intake'], old_intake)
        finally:
            for name, value in initial.items():
                if value is absent: sys.modules.pop(name, None)
                else: sys.modules[name] = value

    def test_config_notes_bytes_explicitly_retained(self):
        I, config, notes = fixture(); reader = mock.Mock()
        reader.read.side_effect = [(pin('config.json'), json.dumps(config).encode()),
                                   (pin('notes.json'), json.dumps(notes).encode())]
        I.D = types.SimpleNamespace(Pins=lambda: reader, parse=json.loads)
        with mock.patch.object(P, 'settings', side_effect=RuntimeError('synthetic stop before admission')):
            with self.assertRaisesRegex(RuntimeError, 'synthetic stop before admission'):
                P.assemble(I, {}, Path('/config'), '11' * 32, Path('/notes'), '22' * 32)
        self.assertEqual(reader.read.call_args_list, [
            mock.call(Path('/config'), '11' * 32, retain=True, maximum=64 << 10),
            mock.call(Path('/notes'), '22' * 32, retain=True, maximum=192 << 10)])


if __name__ == '__main__':
    unittest.main(verbosity=2)
