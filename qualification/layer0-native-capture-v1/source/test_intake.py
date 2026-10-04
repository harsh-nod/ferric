"""Synthetic policy tests only; no CPU build, native call, or GPU observation."""
import copy
import hashlib
from pathlib import Path
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import intake as I


def pin(path='/example', body=b'body'):
    return dict(path=path, bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def plan():
    value = {name: pin('/' + name) for name in I.PLAN_FIELDS.split()}
    value.update(schema=I.INPUT_SCHEMA, output_label='prefix-layer0-native-capture-gpu-v228-v1')
    return value


def cpu(role):
    parent = role == 'parent'
    name = I.PARENT_NAME if parent else I.WORKER_NAME
    binary = pin('/qualified/target/debug/' + name)
    actual = I.PARENT_BINARY if parent else I.WORKER_BINARY
    binary.update(bytes=actual[0], sha256=actual[1])
    artifact = dict(reason='compiler-artifact', executable=binary['path'], filenames=[binary['path']],
        target=dict(name=name, kind=['bin'], crate_types=['bin']),
        features=['default', 'tp-batch-engineering'] if parent else [],
        profile=dict(opt_level='2', test=False, debug_assertions=True, overflow_checks=True))
    return dict(schema='ferric-p228-layer0-native-capture-cpu-result-v1' if parent
        else 'ferric-p228-gfx950-clock-recorder-cpu-result-v1', passed=True, error=None, postcheck_errors=[],
        source_unchanged=True, empty_initial_target=True, tests_passed=289 if parent else 669,
        tests_ignored=0 if parent else 4, parent_rebuilt=parent, worker_rebuilt=not parent,
        compiler_hsaco_reproduced=False, sibling_runtime_rebuilt=False, gpu_execution=False, numerical_acceptance=False,
        performance_claim=False, timestamp_calibration=False, production_authority=False,
        phases={str(i): dict(exit_code=0, reason=None, group_absent=True) for i in range(46 if parent else 33)},
        binaries={name: dict(artifact=artifact, binary=binary)})


def previous_fixture(fail=None):
    base = Path('/synthetic-prior')
    bodies = {
        'layer_validation': 'marker = object()\n',
        'host_validation': 'import layer_validation as V\n',
        'raw_validation': 'import host_validation as H\n',
        'down2': 'marker = object()\n',
        'device_validation': 'import raw_validation as RV\nimport down2 as D2\n',
        'intake': 'import layer_validation as V\nimport down2 as D2\n',
    }
    raw = {name + '.py': body.encode() for name, body in bodies.items()}
    manifest = dict(files=[dict(pin(name, body), path=name) for name, body in raw.items()])
    calls = []
    def read(path, expected, retain, maximum):
        calls.append(path.name)
        if path.stem == fail:
            raise RuntimeError('synthetic authenticated load failure')
        body = raw[path.name]
        if expected != hashlib.sha256(body).hexdigest():
            raise RuntimeError('synthetic pin mismatch')
        return pin(str(path), body), body
    return base, SimpleNamespace(read=read, json=lambda *args: (manifest, pin())), manifest, calls


def request_fixture():
    worker, prefix, mlp = pin('/worker'), pin('/prefix'), pin('/mlp')
    request = dict(schema='FerricFinitePrefixLayerCaptureRequestV1', source='/model', worker=worker,
        images={name: pin('/' + name) for name in ('prefix', 'mlp', 'residual', 'tail')},
        expected_bundle_id=[1] * 32, expected_model_id=[2] * 32, device_ids=[3, 4], session=[5] * 32,
        prompt={name: pin('/' + name) for name in ('manifest', 'text', 'tokens')},
        prefix_tiles_image=prefix, mlp_tiles_image=mlp, evidence_directory=str(I.E / 'case/native'),
        dispatch_timeout_ms=10000, child_deadline_ms=3600000)
    old = {key: value for key, value in request.items() if key not in ('prefix_tiles_image', 'mlp_tiles_image')}
    old.update(schema='FerricFinitePrefixDecodeRequestV1', mode='teacher_forced',
               prefix_image=prefix, tiles_image=mlp, session=[6] * 32, evidence_directory='/old')
    runtime = dict(parent=pin('/parent'), worker=worker, image=prefix)
    prior = dict(request=old, runtime=runtime, L=SimpleNamespace(rust_pin=lambda v: v),
                 plan=dict(down2_image=mlp))
    return request, prior, runtime


def review_fixture():
    p = plan()
    runtime = dict(parent=p['parent'], worker=p['worker'], image=pin('/prefix'))
    prior = dict(plan=dict(down2_image=pin('/down2')), baseline_hidden=dict(source=pin('/payload'),
        offset=0, bytes=8192, sha256=I.HIDDEN_SHA), standalone=dict(provenance={'actual': 'V7'}),
        down2_provenance={'actual': 'Down2'})
    value = {name: p[name] for name in I.REVIEW_BINDINGS}
    value.update(schema=I.REVIEW_SCHEMA, reviewed=True, authority='none', gpu_attempts=1,
        output_label=p['output_label'], image=runtime['image'], down2_image=prior['plan']['down2_image'],
        baseline_hidden=prior['baseline_hidden'], image_provenance=prior['standalone']['provenance'],
        down2_provenance=prior['down2_provenance'],
        notes='Root records the exact finite engineering limitations here.',
        review_topics={key: 'Root reviews this exact source and runtime evidence topic.' for key in I.TOPICS},
        **{key: False for key in I.REVIEW_FALSE})
    return value, p, runtime, prior


class IntakeTests(unittest.TestCase):
    def test_previous_loader_keeps_dependency_bindings_and_restores_aliases(self):
        aliases = ('layer_validation', 'host_validation', 'raw_validation', 'down2', 'device_validation')
        for state in ('absent', 'object', 'none'):
            base, pins, _, calls = previous_fixture()
            with patch.dict(sys.modules):
                for name in aliases:
                    if state == 'absent': sys.modules.pop(name, None)
                    else: sys.modules[name] = None if state == 'none' else object()
                before = {name: sys.modules[name] for name in aliases if name in sys.modules}
                with patch.object(I.D, 'package', return_value=base):
                    old, dv, _, _ = I.previous_package(pins)
                self.assertIs(dv.RV.H.V, old.V)
                self.assertIs(dv.D2, old.D2)
                self.assertEqual(calls, [name + '.py' for name in (*aliases, 'intake')])
                self.assertEqual({name: sys.modules[name] for name in aliases if name in sys.modules}, before)

    def test_previous_loader_restores_aliases_on_each_authenticated_failure(self):
        aliases = ('layer_validation', 'host_validation', 'raw_validation', 'down2', 'device_validation')
        for fail in (*aliases, 'intake'):
            base, pins, _, _ = previous_fixture(fail)
            with patch.dict(sys.modules):
                for index, name in enumerate(aliases):
                    if index % 2: sys.modules.pop(name, None)
                    else: sys.modules[name] = object()
                before = {name: sys.modules[name] for name in aliases if name in sys.modules}
                with patch.object(I.D, 'package', return_value=base), self.assertRaisesRegex(RuntimeError, 'authenticated load'):
                    I.previous_package(pins)
                self.assertEqual({name: sys.modules[name] for name in aliases if name in sys.modules}, before)

    def test_closed_capture_plan_refuses_policy_modes_missing_and_extra_pins(self):
        p = plan(); I.input_shape(p)
        for change in ('extra', 'policy', 'mode', 'missing', 'type'):
            bad = copy.deepcopy(p)
            if change == 'extra': bad['down2_image'] = pin()
            elif change == 'policy': bad['policy'] = 'shared-full-currentness'
            elif change == 'mode': bad['output_label'] = 'prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-v1'
            elif change == 'type': bad['request'] = None
            else: del bad['parent_cpu']
            with self.assertRaises(RuntimeError): I.input_shape(bad)

    def test_qualified_cpu_uses_new_capture_parent_and_unchanged_worker(self):
        with patch.object(I, 'PARENT_BINARY', (20, 'a' * 64)):
            for role in ('parent', 'worker'):
                self.assertEqual(I.qualified_cpu(cpu(role), role), cpu(role)['binaries'][I.PARENT_NAME if role == 'parent' else I.WORKER_NAME])

    def test_cpu_refuses_old_schema_wrong_counts_or_false_success(self):
        with patch.object(I, 'PARENT_BINARY', (20, 'a' * 64)):
            for key, value in (('schema', 'ferric-p228-gfx950-clock-parent-cpu-result-v1'),
                    ('tests_passed', 275), ('tests_passed', True), ('tests_ignored', 1),
                    ('passed', False), ('error', 'failure'), ('postcheck_errors', ['changed']),
                    ('source_unchanged', False), ('parent_rebuilt', False), ('sibling_runtime_rebuilt', True)):
                bad = cpu('parent'); bad[key] = value
                with self.assertRaises(RuntimeError): I.qualified_cpu(bad, 'parent')

    def test_cpu_refuses_incomplete_forced_or_nonzero_phases(self):
        for change in ('missing', 'exit', 'reason', 'group'):
            bad = cpu('worker')
            if change == 'missing': del bad['phases']['0']
            elif change == 'exit': bad['phases']['0']['exit_code'] = False
            elif change == 'reason': bad['phases']['0']['reason'] = 'timeout'
            else: bad['phases']['0']['group_absent'] = False
            with self.assertRaises(RuntimeError): I.qualified_cpu(bad, 'worker')

    def test_cpu_artifact_profile_identity_and_authority_are_not_relaxed(self):
        for key, value in (('test', True), ('opt_level', '0'), ('debug_assertions', False), ('overflow_checks', False)):
            bad = cpu('worker'); bad['binaries'][I.WORKER_NAME]['artifact']['profile'][key] = value
            with self.assertRaises(RuntimeError): I.qualified_cpu(bad, 'worker')
        for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'timestamp_calibration', 'production_authority'):
            bad = cpu('worker'); bad[key] = True
            with self.assertRaises(RuntimeError): I.qualified_cpu(bad, 'worker')

    def test_pending_parent_and_package_fail_before_reading_inputs(self):
        with patch.object(I, 'PARENT_CPU', None), patch.object(I, 'doc') as read:
            with self.assertRaises(RuntimeError): I.cpu_evidence(None, pin(), 'parent')
            read.assert_not_called()
        with patch.object(I, 'TEST_RUNNER_SHA', None):
            pins = SimpleNamespace(json=lambda *args: self.fail('unfrozen package was read'))
            with self.assertRaises(RuntimeError): I.package_record(pins)

    def test_baseline_requires_exact_actual_down2_receipt_before_replay(self):
        with patch.object(I, 'previous_package') as load:
            with self.assertRaises(RuntimeError): I.baseline(None, pin(str(I.E / I.BASELINE_LABEL / 'complete.json')))
            load.assert_not_called()

    def test_actual_down2_clock_and_legacy_baseline_fields_are_not_conflated(self):
        names = ('request', 'parent', 'worker', 'parent_runtime_review', 'worker_runtime_review')
        p = {name: pin('/' + name) for name in (*names, 'baseline', 'clock_baseline', 'parent_cpu', 'worker_cpu')}
        value = {name: p[name] for name in names}
        value.update(baseline=p['clock_baseline'], legacy_baseline=p['baseline'],
                     parent_cpu_complete=p['parent_cpu'], worker_cpu_complete=p['worker_cpu'])
        I.baseline_bindings(p, value)
        for key in ('baseline', 'legacy_baseline'):
            bad = copy.deepcopy(value)
            bad[key] = value['legacy_baseline' if key == 'baseline' else 'baseline']
            with self.assertRaises(RuntimeError): I.baseline_bindings(p, bad)

    def test_deployed_binary_rehashes_actual_elf_without_build_host_alias(self):
        raw = b'\x7fELF\x02\x01' + bytes(12) + b'\x3e\x00'
        original = pin('/build/' + I.PARENT_NAME, raw)
        deployed = dict(original, path=str(I.E / 'capture-runtime-v228-v1' / I.PARENT_NAME))
        with patch.object(I, 'read', return_value=raw):
            self.assertEqual(I.deployed_binary(None, deployed, original), deployed)
            for bad in (dict(deployed, sha256='a' * 64), dict(deployed, path='/tmp/' + I.PARENT_NAME)):
                with self.assertRaises(RuntimeError): I.deployed_binary(None, bad, original)
        with patch.object(I, 'read', return_value=b'not elf'):
            with self.assertRaises(RuntimeError): I.deployed_binary(None, deployed, original)

    def test_request_preserves_genuine_inputs_and_reads_each_image_and_prompt(self):
        request, prior, runtime = request_fixture()
        with patch.object(I, 'read', return_value=b'body') as read:
            I.request_check(None, request, prior, runtime, I.E / 'case')
        self.assertEqual(read.call_count, 9)

    def test_request_rejects_each_input_or_image_substitution_and_policy_injection(self):
        request, prior, runtime = request_fixture()
        for key in ('source', 'images', 'expected_bundle_id', 'expected_model_id', 'device_ids',
                    'prompt', 'dispatch_timeout_ms', 'child_deadline_ms', 'prefix_tiles_image', 'mlp_tiles_image', 'worker'):
            bad = copy.deepcopy(request); bad[key] = 'changed'
            with patch.object(I, 'read'), self.assertRaises((RuntimeError, TypeError)):
                I.request_check(None, bad, prior, runtime, I.E / 'case')
        for field in ('policy', 'mode', 'input_token', 'hidden'):
            with self.assertRaises(RuntimeError):
                I.request_check(None, dict(request, **{field: 'injected'}), prior, runtime, I.E / 'case')

    def test_request_requires_new_schema_session_and_exclusive_output(self):
        request, prior, runtime = request_fixture()
        for key, value in (('schema', 'FerricFinitePrefixLayerComparisonRequestV1'),
                           ('session', [0] * 32), ('session', prior['request']['session']),
                           ('evidence_directory', '/old')):
            with self.assertRaises(RuntimeError):
                I.request_check(None, dict(request, **{key: value}), prior, runtime, I.E / 'case')

    def test_review_binds_all_provenance_and_never_grants_authority(self):
        value, p, runtime, prior = review_fixture()
        I.engineering_review(value, p, runtime, prior)
        for key, changed in [('reviewed', False), ('gpu_attempts', 2), ('gpu_attempts', True),
                             *[(key, True) for key in I.REVIEW_FALSE]]:
            with self.assertRaises(RuntimeError): I.engineering_review(dict(value, **{key: changed}), p, runtime, prior)

    def test_review_refuses_rebinding_old_hidden_provenance_and_empty_notes(self):
        value, p, runtime, prior = review_fixture()
        for key in (*I.REVIEW_BINDINGS, 'image', 'down2_image', 'baseline_hidden', 'image_provenance', 'down2_provenance'):
            with self.assertRaises(RuntimeError):
                I.engineering_review(dict(value, **{key: {'changed': True}}), p, runtime, prior)
        for key in ('notes', 'review_topics'):
            bad = dict(value, **{key: '' if key == 'notes' else dict(value['review_topics'], formal='')})
            with self.assertRaises(RuntimeError): I.engineering_review(bad, p, runtime, prior)

    def test_owned_success_never_treats_killed_or_unreaped_child_as_natural(self):
        good = dict(exit_code=0, reason=None, cleanup_signalled=False, owned_groups_absent=True, owned_processes_reaped=True)
        I.owned_success(good)
        for key, value in (('exit_code', False), ('reason', 'timeout'), ('cleanup_signalled', True),
                           ('owned_groups_absent', False), ('owned_processes_reaped', False)):
            with self.assertRaises(RuntimeError): I.owned_success(dict(good, **{key: value}))


if __name__ == '__main__':
    unittest.main()
