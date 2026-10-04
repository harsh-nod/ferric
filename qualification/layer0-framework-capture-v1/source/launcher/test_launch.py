"""Pure launcher policy tests; no retained helper, framework, subprocess or GPU execution."""
import copy
from pathlib import Path
import tempfile
import types
import unittest
from unittest.mock import patch

import launch as M


def pin(path, digest='a' * 64):
    return dict(path=str(path), bytes=1, sha256=digest)


def inputs():
    root = M.E / 'p224-rearm-framework/reference'
    return dict(schema='ferric-p228-layer0-framework-launch-inputs-v1', launcher_sha256='b' * 64,
        owned_helper=pin(M.E / 'owned.py', M.OLD_SHA),
        capture_package={name: pin(M.E / 'p228-layer0-framework-capture-v1' / name, sha)
                         for name, sha in M.CAPTURE.items()}, reference_helper=pin(root / 'framework_reference.py', M.BASE_SHA),
        reference_support=dict(diagnostics=pin(root / 'diagnostics.py'), policy=pin(root / 'policy.json'),
            long_reference=pin(root / 'helpers/long_reference.py')), legacy_plan=pin(M.E / 'legacy.json'),
        source_authentication=pin(M.E / 'source.json'), token_provenance=dict(manifest=pin(M.E / 'tokens.json'),
            prompt_ids=pin(M.E / 'tokens.bin')), topology=dict(host='asrock-1w300-g2-2b', card='card1', uid='1' * 16),
        platform_monitor=None, implementation_sources={name: pin('/venv/' + name + '.py')
            for name in ('modeling_qwen3', 'activation', 'sdpa', 'torch_functional')},
        output_label='layer0-framework-launch-v228-v1', capture_label='layer0-framework-capture-v228-v1',
        execution_review=pin(M.E / 'review.json'))


def review(value):
    return dict(schema='ferric-p228-layer0-framework-launch-review-v1', reviewed=True,
        inputs_projection_sha256=M.sha(M.encoded({k: v for k, v in value.items() if k != 'execution_review'})),
        resources=M.LIMITS, gpu_execution_authorized=True, **{key: False for key in M.FALSE})


def monitor():
    return dict(schema='ferric-p223-platform-monitor-v1', host='host', boot_id='boot',
        process=dict(pid=77, uid=0, ppid=1, pgid=77, session=77, start_ticks=456,
                     comm='gpuagent', cmdline_hex=b'/usr/local/bin/gpuagent\0'.hex()),
        executable=pin('/usr/local/bin/gpuagent'), filesystem_uid=1001, filesystem_gid=1001,
        read_only_attestation_reviewed=True)


def inventory(total=0, files=0):
    return dict(total=total, cache=0, tmp=0, other=total, output=0, files=files,
                maximum_file=total, maximum_provider_file=0)


def report_fixture(directory):
    shapes = {'stage' + str(n): (1,) for n in range(33)}
    policy = types.SimpleNamespace(SHAPES=shapes, stage_bytes=lambda _: 2,
                                  validate_values=lambda values: M.require(len(values) == 33, 'stages'))
    plan = pin('/synthetic/plan.json')
    report = dict(schema='ferric-p228-layer0-framework-capture-v1', status='PASS',
        repeat_passes_byte_equal=True, genuine_framework_chain=True, position=0, input_token=9112,
        captured_stages_per_pass=33, input_pins=[plan], gpu_execution=True,
        **{key: False for key in ('candidate_gpu_execution', 'candidate_intermediate_inputs',
           'conditional_replay_performed', 'numerical_acceptance', 'full_model_correctness',
           'production_authority', 'performance_measured')})
    report['passes'] = [dict(ordinal=index, position=0, input_token=9112, fresh_cache=True,
        stages={name: dict(dtype='bfloat16', shape=[1], pin=dict(path=str(directory / f'{index}-{name}.bf16'),
            bytes=2, sha256='1' * 64)) for name in shapes}) for index in (1, 2)]
    return policy, plan, report


class LaunchPolicy(unittest.TestCase):
    def test_exact_package_and_closed_inputs(self):
        M.input_shape(inputs())
        for name in M.CAPTURE:
            value = inputs(); value['capture_package'][name]['sha256'] = 'f' * 64
            with self.assertRaises(ValueError): M.input_shape(value)
        value = inputs(); value['unknown'] = 1
        with self.assertRaises(ValueError): M.input_shape(value)

    def test_original_namespace_and_wrong_host_refused(self):
        for key, value in (('output_label', 'framework-rearm-v224-v1'),
                           ('capture_label', 'layer0-framework-capture-v228-v0')):
            row = inputs(); row[key] = value
            with self.assertRaises(ValueError): M.input_shape(row)
        row = inputs(); row['topology']['host'] = 'mi350'
        with self.assertRaises(ValueError): M.input_shape(row)

    def test_package_cannot_mix_source_directories(self):
        row = inputs(); row['capture_package']['test_run.py']['path'] = str(M.E / 'other/test_run.py')
        with self.assertRaises(ValueError): M.input_shape(row)

    def test_reference_support_is_local_and_complete(self):
        row = inputs(); row['reference_support']['long_reference']['path'] = '/elsewhere/long_reference.py'
        with self.assertRaises(ValueError): M.input_shape(row)
        row = inputs(); del row['reference_support']['policy']
        with self.assertRaises(ValueError): M.input_shape(row)

    def test_launch_review_binds_projection_resources_and_false_claims(self):
        row = inputs(); approved = review(row); M.review_inputs(row, approved)
        for field in ('reviewed', 'gpu_execution_authorized', 'numerical_acceptance'):
            value = copy.deepcopy(approved); value[field] = not value[field]
            with self.assertRaises(ValueError): M.review_inputs(row, value)
        changed = copy.deepcopy(row); changed['capture_label'] = 'layer0-framework-capture-v228-v2'
        with self.assertRaises(ValueError): M.review_inputs(changed, approved)
        changed = copy.deepcopy(approved); changed['resources']['timeout_seconds'] += 1
        with self.assertRaises(ValueError): M.review_inputs(row, changed)

    def test_monitor_uses_fresh_reviewed_process_not_historical_pid(self):
        old = types.SimpleNamespace(); row = monitor()
        topology = dict(host='host', boot_id='boot', uid='f' * 16)
        with patch.object(M, 'read', return_value=b'x'):
            M.configure_monitor(old, row, topology)
        self.assertEqual(old.MONITOR_PID, 77)
        self.assertEqual(old.MONITOR_PROCESS['start_ticks'], 456)
        old.monitor_document(row, topology)
        changed = copy.deepcopy(row); changed['process']['start_ticks'] += 1
        with self.assertRaises(ValueError): old.monitor_document(changed, topology)

    def test_monitor_wrong_executable_identity_or_process_shape_refused(self):
        topology = dict(host='host', boot_id='boot', uid='f' * 16)
        for key, value in (('uid', 9661), ('pid', True), ('pgid', 3), ('comm', 'other')):
            row = monitor(); row['process'][key] = value
            with self.assertRaises(ValueError): M.configure_monitor(types.SimpleNamespace(), row, topology)
        row = monitor(); row['executable']['path'] = '/tmp/gpuagent'
        with self.assertRaises(ValueError): M.configure_monitor(types.SimpleNamespace(), row, topology)

    def test_no_monitor_does_not_create_a_monitor_exception(self):
        old = types.SimpleNamespace()
        M.configure_monitor(old, None, dict(uid='c' * 16))
        self.assertEqual(vars(old), {'UID': 'c' * 16})

    def test_reference_plan_binds_actual_direct_supervisor_and_no_candidate_inputs(self):
        row = inputs(); row['topology']['boot_id'] = 'boot'
        old = types.SimpleNamespace(PYTHON=Path('/venv/bin/python'))
        with patch.object(M, 'read', return_value=b'{"model_root":"/original/model"}'):
            value = M.reference_plan(row, old, M.E / row['capture_label'], 1234)
        self.assertEqual(value['supervisor_pid'], 1234)
        self.assertEqual(value['input_tokens'], [9112, 2190, 3772, 220])
        self.assertEqual(value['model_root'], '/original/model')
        self.assertEqual(value['execution_review'], row['execution_review'])
        self.assertEqual(value['resources'], M.LIMITS)

    def test_first_pin_read_refuses_wrong_bytes_without_importing_module(self):
        with tempfile.TemporaryDirectory() as name:
            path = Path(name) / 'body.py'; path.write_bytes(b'raise RuntimeError("do not import")')
            value = dict(path=str(path), bytes=path.stat().st_size, sha256='0' * 64)
            with self.assertRaises(ValueError): M.load(value, '0' * 64, 'never_loaded')

    def test_approval_wait_timeout_never_creates_review(self):
        old = types.SimpleNamespace(ONGOING_FREE=0, inventory=lambda _: {}, storage_ok=lambda _: None)
        with tempfile.TemporaryDirectory() as name:
            with patch.object(M.time, 'monotonic', side_effect=[0, 301]):
                with self.assertRaisesRegex(ValueError, 'approval timeout'):
                    M.approval(old, Path(name), {}, None, [])
            self.assertFalse((Path(name) / 'approval.json').exists())

    def test_late_approval_file_does_not_bypass_wait_deadline(self):
        with tempfile.TemporaryDirectory() as name:
            (Path(name) / 'approval.json').write_bytes(b'{}')
            with patch.object(M.time, 'monotonic', side_effect=[0, 301]):
                with self.assertRaisesRegex(ValueError, 'after deadline'):
                    M.approval(None, Path(name), {}, None, [])

    def test_post_audits_and_single_attempt_are_required_for_success(self):
        value = dict(failures=[], native_attempts=1, reference={'captured': True},
                     before_audits=[1, 2, 3], after_audits=[1, 2, 3])
        self.assertTrue(M.completed_status(value))
        for key, item in (('failures', ['post-audit failed']), ('native_attempts', 0),
                          ('native_attempts', 2), ('reference', None), ('after_audits', [1, 2])):
            changed = dict(value); changed[key] = item
            self.assertFalse(M.completed_status(changed))

    def test_all_three_post_audits_attempted_even_after_each_failure(self):
        for failure in range(3):
            calls = []; result = dict(after_audits=[], failures=['earlier native failure'])
            def observe(index):
                calls.append(index)
                if index == failure: raise ValueError('audit failed')
                return index
            M.post_audits(result, observe)
            self.assertEqual(calls, [0, 1, 2])
            self.assertEqual(len(result['after_audits']), 2)
            self.assertEqual(len(result['failures']), 2)

    def test_separate_capture_cache_and_output_keep_original_aggregate_categories(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name); owner = root / 'owner'; output = root / 'capture'
            owner.mkdir(); output.mkdir(); (output / 'private-cache').mkdir()
            (output / 'result').write_bytes(b'out')
            (output / 'private-cache' / 'cache').write_bytes(b'cache')
            values = {owner: inventory(7, 1), output: inventory(8, 3), output / 'private-cache': inventory(5, 1)}
            actual = M.joined_inventory(lambda path: dict(values[path]), owner, output)
            self.assertEqual((actual['total'], actual['cache'], actual['other'], actual['output']), (15, 5, 10, 3))
            self.assertEqual(actual['maximum_provider_file'], 5)
            self.assertEqual(actual['files'], 4)

    def test_capture_directory_or_cache_symlinks_are_refused(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name); owner = root / 'owner'; actual = root / 'actual'; actual.mkdir()
            alias = root / 'alias'; alias.symlink_to(actual, target_is_directory=True)
            with self.assertRaises(ValueError): M.joined_inventory(lambda _: inventory(), owner, alias)
            (actual / 'private-cache').symlink_to(root, target_is_directory=True)
            with self.assertRaises(ValueError): M.joined_inventory(lambda _: inventory(), owner, actual)

    def test_complete_two_pass_stage_report_has_no_candidate_acceptance(self):
        directory = Path('/synthetic/capture'); policy, plan, report = report_fixture(directory)
        old = types.SimpleNamespace(checked=lambda _: b'\0\0')
        M.validate_report(old, policy, report, directory, plan)
        report['numerical_acceptance'] = True
        with self.assertRaises(ValueError): M.validate_report(old, policy, report, directory, plan)

    def test_stage_missing_extent_path_and_repeat_mismatch_are_refused(self):
        directory = Path('/synthetic/capture'); policy, plan, original = report_fixture(directory)
        old = types.SimpleNamespace(checked=lambda row: b'\0\0')
        for failure in ('missing', 'extent', 'path', 'dtype'):
            report = copy.deepcopy(original); stage = report['passes'][1]['stages']['stage0']
            if failure == 'missing': del report['passes'][1]['stages']['stage0']
            if failure == 'extent': stage['pin']['bytes'] = 4
            if failure == 'path': stage['pin']['path'] = '/elsewhere/file'
            if failure == 'dtype': stage['dtype'] = 'float32'
            with self.assertRaises(ValueError): M.validate_report(old, policy, report, directory, plan)
        old.checked = lambda row: b'\1\0' if '/2-' in row['path'] else b'\0\0'
        with self.assertRaises(ValueError): M.validate_report(old, policy, original, directory, plan)


if __name__ == '__main__':
    unittest.main()
