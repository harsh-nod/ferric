"""Synthetic closed-input and generation refusals; no native execution."""
import copy
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import intake as I


def pin(name='record', size=1, digest='ab' * 32):
    return dict(path=str(I.E / name), bytes=size, sha256=digest)


def cpu():
    names = ['worker::' + str(i) for i in range(509)]
    value = dict(schema='ferric-projection-ar4-cpu-result-v1', passed=True,
        error=None, postcheck_errors=[], source_unchanged=True, empty_initial_target=True,
        metadata=dict(parent={}, worker={}),
        phases={str(i): dict(exit_code=0, reason=None, group_absent=True) for i in range(87)},
        tests={'worker-tests': dict(passed=505, ignored=4, names=names, summaries=[[505, 0, 4]]),
               'parent-client': dict(passed=324, ignored=0, names=['parent::' + str(i) for i in range(324)], summaries=[[324, 0, 0]]),
               'runtime': dict(passed=208, ignored=0, names=['runtime::' + str(i) for i in range(208)], summaries=[[208, 0, 0]])},
        tests_passed=1037, tests_ignored=4, binaries={})
    for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority'):
        value[key] = False
    for role, name in (('parent', I.PARENT_NAME), ('worker', I.WORKER_NAME)):
        root = I.E / I.CPU_LABEL
        source = root / 'sources/ferric/adapters' / ('m1-engineering-execution-v1' if role == 'parent'
                                                   else 'tp-peer-finite-engineering-worker-v1')
        binary = dict(path=str(root / 'target' / role / 'debug' / name), bytes=1, sha256='ab' * 32)
        artifact = dict(reason='compiler-artifact',
            target=dict(name=name, kind=['bin'], crate_types=['bin'],
                        src_path=str(source / 'src' / ('main.rs' if role == 'worker' else 'bin/' + name + '.rs'))),
            executable=binary['path'], filenames=[binary['path']], manifest_path=str(source / 'Cargo.toml'),
            features=['tp-batch-engineering'] if role == 'parent' else [],
            profile=dict(test=False, opt_level='2', debug_assertions=True, overflow_checks=True, debuginfo=0))
        value['binaries'][name] = dict(artifact=artifact, binary=binary)
    value['binaries'].update({'old-bin-' + str(i): {} for i in range(15)})
    return value


def request_fixture():
    image = pin('image', *I.IMAGE); worker = pin('new-worker'); v7 = pin('v7'); down = pin('down2')
    mlp = pin('materialized-silu', *I.SILU_IMAGE)
    previous = dict(schema='FerricFinitePrefixDecodeRequestV1', mode='teacher_forced', source='/model',
        worker=pin('old-worker'), images={k: pin(k) for k in ('prefix', 'mlp', 'tail', 'residual')},
        expected_bundle_id=[1] * 32, expected_model_id=[2] * 32, device_ids=[7, 9],
        prompt={k: pin(k) for k in ('tokens', 'manifest', 'text')}, dispatch_timeout_ms=10000,
        child_deadline_ms=3600000, session=[3] * 32, prefix_image=v7, tiles_image=down,
        evidence_directory=str(I.E / 'old/native'))
    decode = copy.deepcopy(previous)
    decode.update(mode='autoregressive', worker=worker, session=[4] * 32, evidence_directory=str(I.E / 'case/native'), tiles_image=mlp)
    request = dict(schema='FerricFiniteProjectionResidualDecodeRequestV1', decode=decode,
                   projection_residual_image=image)
    prior = dict(request=previous, L=SimpleNamespace(rust_pin=lambda v: v), plan=dict(down2_image=down))
    return request, prior, dict(parent=pin('parent'), worker=worker, image=v7), I.E / 'case', image, mlp


class IntakeTests(unittest.TestCase):
    def test_plan_is_separate_closed_ar4_namespace(self):
        plan = {key: pin(key) for key in I.PLAN_FIELDS.split()}
        plan.update(schema=I.INPUT_SCHEMA, output_label='prefix-projection-ar4-decode-gpu-v228-v1')
        I.input_shape(plan)
        for change in ({'policy': 'shared-full-currentness'}, {'baseline_capture': pin()},
                       {'output_label': 'prefix-projection-residual-capture-gpu-v228-v1'},
                       {'schema': 'ferric-p228-projection-residual-capture-inputs-v1'}):
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                I.input_shape(dict(plan, **change))

    def test_pending_actual_cpu_and_package_inputs_fail_closed(self):
        for value in (None, (1, 'unknown'), (True, 'ab' * 32)):
            with self.assertRaises(RuntimeError): I.actual_tuple(value, 'fixture')
        with patch.object(I, 'PURE_TESTS', None), self.assertRaises(RuntimeError): I.package_record(None)
        with patch.object(I, 'JOINT_CPU', None), patch.object(I, 'doc') as doc:
            with self.assertRaises(RuntimeError): I.cpu_evidence(None, {}, {})
            doc.assert_not_called()

    def test_new_flat_cpu_accounting_and_both_roles(self):
        with patch.object(I, 'BINARIES', dict(parent=(1, 'ab' * 32), worker=(1, 'ab' * 32))):
            self.assertEqual(set(I.qualified_cpu(cpu())), {'parent', 'worker'})
            for key, bad in (('source_unchanged', False), ('tests_passed', 1), ('tests_ignored', 0),
                             ('numerical_acceptance', True), ('gpu_execution', True)):
                with self.subTest(key=key), self.assertRaises(RuntimeError):
                    I.qualified_cpu(dict(cpu(), **{key: bad}))

    def test_cpu_name_summary_and_phase_mutations_refuse(self):
        with patch.object(I, 'BINARIES', dict(parent=(1, 'ab' * 32), worker=(1, 'ab' * 32))):
            for mode in ('bool-exit', 'forced', 'phase-count', 'duplicate', 'failed', 'bool-count', 'artifact-count'):
                value = cpu()
                if mode == 'bool-exit': value['phases']['0']['exit_code'] = False
                elif mode == 'forced': value['phases']['0']['reason'] = 'timeout'
                elif mode == 'phase-count': value['phases'].pop('0')
                elif mode == 'duplicate': value['tests']['worker-tests']['names'][-1] = 'worker::0'
                elif mode == 'failed': value['tests']['worker-tests']['summaries'][0][1] = 1
                elif mode == 'bool-count': value['tests']['parent-client']['passed'] = True
                else: value['binaries'].pop('old-bin-0')
                with self.subTest(mode=mode), self.assertRaises(RuntimeError): I.qualified_cpu(value)

    def test_selected_artifact_requires_exact_new_source_target_and_profile(self):
        with patch.object(I, 'BINARIES', dict(parent=(1, 'ab' * 32), worker=(1, 'ab' * 32))):
            for mode in ('feature', 'digest', 'overflow', 'namespace', 'source', 'manifest'):
                value = cpu(); target = value['binaries'][I.WORKER_NAME]
                if mode == 'feature': target['artifact']['features'] = ['live-validation']
                elif mode == 'digest': target['binary']['sha256'] = 'cd' * 32
                elif mode == 'overflow': target['artifact']['profile']['overflow_checks'] = False
                elif mode == 'namespace': target['binary']['path'] = str(I.E / I.WORKER_NAME)
                elif mode == 'source': target['artifact']['target']['src_path'] += '.old'
                else: target['artifact']['manifest_path'] += '.old'
                with self.subTest(mode=mode), self.assertRaises(RuntimeError): I.qualified_cpu(value)

    def test_nested_ar4_preserves_original_copy_v7_and_selects_silu_with_all_input_reads(self):
        with patch.object(I, 'read', return_value=b'') as read:
            I.request_check(None, *request_fixture())
            self.assertEqual(read.call_count, 10)

    def test_request_refuses_workload_image_session_and_bound_changes(self):
        request, prior, runtime, out, image, mlp = request_fixture()
        for mode in ('copy', 'prefix', 'down', 'worker', 'token', 'session', 'output', 'deadline', 'extra-image'):
            bad = copy.deepcopy(request)
            if mode == 'copy': bad['decode']['images']['residual'] = image
            elif mode == 'prefix': bad['decode']['prefix_image'] = image
            elif mode == 'down': bad['decode']['tiles_image'] = image
            elif mode == 'worker': bad['decode']['worker'] = pin('old-worker')
            elif mode == 'token': bad['decode']['prompt']['tokens'] = pin('changed-token')
            elif mode == 'session': bad['decode']['session'] = prior['request']['session']
            elif mode == 'output': bad['decode']['evidence_directory'] = '/tmp/other'
            elif mode == 'deadline': bad['decode']['child_deadline_ms'] = 7200000
            else: bad['projection_residual_image'] = pin('wrong-image')
            with self.subTest(mode=mode), patch.object(I, 'read', return_value=b''), self.assertRaises(RuntimeError):
                I.request_check(None, bad, prior, runtime, out, image, mlp)

    def test_request_cannot_enable_tf_clocks_host_policy_or_old_schema(self):
        request, prior, runtime, out, image, mlp = request_fixture()
        for mode in ('tf', 'clock', 'policy', 'old-schema', 'old-inner'):
            bad = copy.deepcopy(request)
            if mode == 'tf': bad['decode']['mode'] = 'teacher_forced'
            elif mode == 'clock': bad['observe_device_clocks'] = True
            elif mode == 'policy': bad['decode']['policy'] = 'shared-full-currentness'
            elif mode == 'old-schema': bad['schema'] = 'FerricFiniteProjectionResidualLayerCaptureRequestV1'
            else: bad['decode']['schema'] = 'FerricFinitePrefixDecodeDeviceClockRequestV2'
            with self.subTest(mode=mode), patch.object(I, 'read', return_value=b''), self.assertRaises(RuntimeError):
                I.request_check(None, bad, prior, runtime, out, image, mlp)

    def test_root_review_binds_new_cpu_and_input_without_numerical_authority(self):
        plan = {key: pin(key) for key in I.REVIEW_BINDINGS}; plan['output_label'] = 'new-case'
        runtime = dict(parent=plan['parent'], worker=plan['worker'], image=pin('v7'))
        prior = dict(plan=dict(down2_image=pin('down2')), standalone=dict(provenance={'source': 'v7'}),
                     down2_provenance={'source': 'down2'})
        image = dict(lowering=plan['lowering_complete'], inspection=plan['inspection_complete'])
        mlp = dict(cpu=plan['mlp_cpu'], lowering=plan['mlp_lowering_complete'])
        review = dict(schema=I.REVIEW_SCHEMA, reviewed=True, authority='none', gpu_attempts=1,
            output_label=plan['output_label'], image=runtime['image'], down2_image=prior['plan']['down2_image'],
            image_provenance=prior['standalone']['provenance'], down2_provenance=prior['down2_provenance'],
            projection_provenance=image, mlp_provenance=mlp,
            review_topics={k: 'Actual substantive scoped root review notes.' for k in I.TOPICS},
            notes='Actual substantive scoped root review notes.', **{k: plan[k] for k in I.REVIEW_BINDINGS},
            **{k: False for k in I.REVIEW_FALSE})
        I.engineering_review(review, plan, runtime, prior, image, mlp)
        for key in I.REVIEW_FALSE:
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                I.engineering_review(dict(review, **{key: True}), plan, runtime, prior, image, mlp)
        for key in I.REVIEW_BINDINGS:
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                I.engineering_review(dict(review, **{key: pin('changed')}), plan, runtime, prior, image, mlp)

    def test_prior_capture_bindings_do_not_relabel_old_tf4_as_new_capture(self):
        pairs = [('baseline', 'baseline'), ('baseline_capture', 'baseline_capture'), ('request', 'request'),
            ('parent', 'parent'), ('worker', 'worker'), ('parent_cpu', 'parent_cpu_complete'),
            ('worker_cpu', 'worker_cpu_complete'), ('projection_image', 'projection_image'),
            ('lowering_complete', 'lowering_complete'), ('inspection_complete', 'inspection_complete'),
            ('parent_runtime_review', 'parent_runtime_review'), ('worker_runtime_review', 'worker_runtime_review'),
            ('capture_review', 'capture_review')]
        plan = {a: pin(a) for a, _ in pairs}; value = {b: plan[a] for a, b in pairs}
        I.baseline_bindings(plan, value)
        for _, key in pairs:
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                I.baseline_bindings(plan, dict(value, **{key: pin('wrong')}))
        with patch.object(I, 'previous') as previous, self.assertRaises(RuntimeError):
            I.corrected_baseline(None, pin('wrong', *I.CAPTURE))
        previous.assert_not_called()

    def test_unqualified_image_or_inspection_refuses_before_read(self):
        plan = dict(lowering_complete=pin('lower', *I.LOWERING),
            inspection_complete=pin('inspection', 1, I.INSPECTION_SHA), projection_image=pin('image', *I.IMAGE))
        for key in plan:
            bad = copy.deepcopy(plan); bad[key]['sha256'] = 'ff' * 32
            with patch.object(I, 'doc') as doc, self.assertRaises(RuntimeError): I.image_evidence(None, bad)
            doc.assert_not_called()

    def test_transport_cannot_substitute_other_executable_identity(self):
        original = pin(I.PARENT_NAME, 20)
        for record in (pin('different', 20), pin(I.PARENT_NAME, 21)):
            with patch.object(I, 'read') as read, self.assertRaises(RuntimeError):
                I.deployed_binary(None, record, original)
            read.assert_not_called()

    def test_ar4_cpu_lineage_has_cpu1022_between_new_runtime_and_cpu988_layer(self):
        layer = dict(receipt=dict(parent_cpu_complete=pin('layer-cpu'), worker_cpu_complete=pin('layer-cpu')))
        previous = dict(schema='ferric-projection-residual-decode-cpu-result-v1', passed=True,
            error=None, postcheck_errors=[], source_unchanged=True, tests_passed=1022, tests_ignored=4,
            prior_completion=pin('layer-cpu'))
        value = dict(prior_completion=pin('projection-residual-decode-cpu-v228-v1/complete.json', *I.PRIOR_CPU),
            controller=pin('p228-projection-ar4-cpu-v1/run.py', 1, I.CPU_CONTROLLER_SHA))
        I.cpu_lineage(value, previous, layer)
        for mode in ('skip-predecessor', 'old-controller', 'wrong-layer', 'failed-predecessor'):
            candidate, old = copy.deepcopy(value), copy.deepcopy(previous)
            if mode == 'skip-predecessor': candidate['prior_completion'] = pin('layer-cpu')
            elif mode == 'old-controller': candidate['controller']['path'] = str(I.E / 'p228-projection-residual-decode-cpu-v1/run.py')
            elif mode == 'wrong-layer': old['prior_completion'] = pin('unrelated-cpu')
            else: old['passed'] = False
            with self.subTest(mode=mode), self.assertRaises(RuntimeError): I.cpu_lineage(candidate, old, layer)

    def test_old_tf_cpu_receipt_cannot_admit_ar4_even_with_new_artifact_fixtures(self):
        with patch.object(I, 'BINARIES', dict(parent=(1, 'ab' * 32), worker=(1, 'ab' * 32))):
            value = cpu(); value['schema'] = 'ferric-projection-residual-decode-cpu-result-v1'
            with self.assertRaises(RuntimeError): I.qualified_cpu(value)

    def test_ar4_cpu_requires_complete_1037_named_census_not_only_positive_total(self):
        with patch.object(I, 'BINARIES', dict(parent=(1, 'ab' * 32), worker=(1, 'ab' * 32))):
            value = cpu(); row = value['tests']['parent-client']
            row['names'].pop(); row['passed'] -= 1; row['summaries'][0][0] -= 1
            value['tests_passed'] -= 1
            with self.assertRaises(RuntimeError): I.qualified_cpu(value)

    def test_ar_admission_keeps_authentic_tf_input_provenance_but_not_its_recurrence(self):
        request, prior, runtime, out, image, mlp = request_fixture()
        self.assertEqual((request['decode']['mode'], prior['request']['mode']), ('autoregressive', 'teacher_forced'))
        with patch.object(I, 'read', return_value=b''):
            I.request_check(None, request, prior, runtime, out, image, mlp)
        prior['request']['mode'] = 'autoregressive'
        with patch.object(I, 'read', return_value=b''), self.assertRaises(RuntimeError):
            I.request_check(None, request, prior, runtime, out, image, mlp)


if __name__ == '__main__':
    unittest.main()
