"""Synthetic CPU-only fixtures; unchanged frozen native validation is exercised."""
import copy
import unittest
from unittest.mock import patch

import host_comparison as HC
from test_host_validation import example, encode, refresh

import observation as M


def plan(f):
    return dict(schema=M.INPUT_SCHEMA, mode=f.observed['request']['mode'], policy=f.policy,
        request=f.value['request'], prefix_image=f.C.pin(f.observed['request']['prefix_image']),
        candidate=f.value)


def observe(f, value=None):
    return M.observe(f.C, plan(f) if value is None else value,
        lambda item, _: f.bodies[item['path']], f.helpers, HC)


class IndependentObservationTests(unittest.TestCase):
    def test_two_modes_and_three_policies_keep_all_authority_false(self):
        for mode in ('teacher_forced', 'autoregressive'):
            for policy in HC.H.POLICIES:
                f = example(mode, policy)
                with self.subTest(mode=mode, policy=policy):
                    result = observe(f)
                    self.assertEqual(result['schema'], M.RESULT_SCHEMA)
                    self.assertEqual(result['status'], 'FINITE_FOUR_FORWARD_OBSERVED')
                    self.assertIs(result['structural_observation_complete'], True)
                    self.assertEqual(result['captured_tensor_rows'], 152)
                    self.assertEqual(result['structural']['positions'], [0, 1, 2, 3])
                    self.assertIs(result['recorded_close_and_owner_reap_checked'], True)
                    for name in ('gpu_launched', 'native_baseline_comparison_performed',
                            'independent_framework_comparison_performed', 'numerical_acceptance',
                            'independent_tensor_acceptance', 'full_model_acceptance',
                            'current_source_binary_image_authority_verified',
                            'current_platform_idle_audits_verified', 'top_level_observer_reaping_verified',
                            'sustained_2048_256', 'performance_claim', 'production_authority'):
                        self.assertIs(result[name], False, name)
                    self.assertIsNone(result['independent_tensor_threshold'])
                    self.assertNotIn('full152_tensor_rows_bitwise_equal', result)

    def test_no_paired_baseline_reference_or_math_is_called(self):
        f = example()
        with patch.object(f.C, 'baseline', side_effect=AssertionError('paired baseline called')) as baseline, \
             patch.object(f.C, 'reference', side_effect=AssertionError('reference called')) as reference, \
             patch.object(f.C, 'compare_rows', side_effect=AssertionError('math called')) as arithmetic:
            result = observe(f)
        baseline.assert_not_called()
        reference.assert_not_called()
        arithmetic.assert_not_called()
        self.assertFalse(result['numerical_acceptance'])

    def test_old_comparison_schema_and_extra_authority_fields_refuse(self):
        f = example()
        for change in ('schema', 'numerical_acceptance', 'baseline', 'reference'):
            value = plan(f)
            if change == 'schema':
                value['schema'] = 'ferric-p228-prefix-decode-host-policy-comparison-inputs-v2'
            else:
                value[change] = True
            with self.subTest(change=change), self.assertRaises(ValueError):
                observe(f, value)

    def test_mode_policy_mismatch_and_unsupported_policy_refuse(self):
        f = example('teacher_forced', 'baseline')
        for key, replacement in (('mode', 'autoregressive'),
                ('policy', 'shared-full-currentness'), ('policy', 'both'), ('mode', True)):
            value = plan(f)
            value[key] = replacement
            with self.subTest(key=key, replacement=replacement), self.assertRaises(ValueError):
                observe(f, value)

    def test_exact_request_pin_mismatch_refuses_before_candidate(self):
        f = example()
        value = plan(f)
        value['request'] = dict(value['request'], path='/task/other-request.json')
        with patch.object(HC, 'candidate') as candidate, self.assertRaises(ValueError):
            observe(f, value)
        candidate.assert_not_called()

    def test_selected_prefix_path_extent_and_digest_all_bind(self):
        f = example()
        for key, replacement in (('path', '/inputs/other-image'), ('bytes', 2), ('sha256', '02' * 32)):
            value = plan(f)
            value['prefix_image'][key] = replacement
            with self.subTest(key=key), self.assertRaises(ValueError):
                observe(f, value)

    def test_missing_capture_control_corruption_and_missing_sidecar_refuse(self):
        for change in ('missing', 'control', 'sidecar'):
            f = example()
            if change == 'missing':
                del f.value['native_files']['observation-3.bin']
            elif change == 'control':
                f.bodies[f.value['native_files']['control-2.bin']['path']] = b'bad'
            else:
                f.bodies[f.value['host_sidecar']['path']] = b'{}'
            with self.subTest(change=change), self.assertRaises((ValueError, RuntimeError)):
                observe(f)

    def test_forced_cleanup_and_unreaped_owner_are_not_observations(self):
        for key in ('cleanup_signalled', 'owned_processes_reaped'):
            f = example()
            f.owner[key] = key == 'cleanup_signalled'
            refresh(f)
            with self.subTest(key=key), self.assertRaises(ValueError):
                observe(f)

    def test_wrong_leaf_argv_and_resource_envelope_still_refuse(self):
        for change in ('argv', 'deadline_seconds'):
            f = example()
            command = copy.deepcopy(f.command)
            if change == 'argv':
                command['argv'].pop()
            else:
                command['deadline_seconds'] += 1
            f.value['command'] = f.retain('/task/command.json', encode(command))
            f.started['command_sha256'] = f.value['command']['sha256']
            f.value['started'] = f.retain('/task/started.json', encode(f.started))
            refresh(f)
            with self.subTest(change=change), self.assertRaises(ValueError):
                observe(f)

    def test_host_sidecar_cannot_grant_numerical_authority(self):
        f = example()
        f.report['numerical_acceptance'] = True
        refresh(f)
        with self.assertRaises(RuntimeError):
            observe(f)


if __name__ == '__main__':
    unittest.main()
