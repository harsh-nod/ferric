"""Actual diagnostic envelope shape; arithmetic calls stay frozen and unchanged."""
import copy
import unittest
from unittest.mock import patch

import host_comparison as M
from test_host_validation import example, encode, refresh


def run_candidate(f):
    return M.candidate(f.C, lambda p, _: f.bodies[p['path']], f.value, f.helpers)


class HostComparisonTests(unittest.TestCase):
    def test_three_policy_candidates_use_same_frozen_native_validator(self):
        for policy in M.H.POLICIES:
            f = example('teacher_forced', policy)
            observed, files, structural, owner, host = run_candidate(f)
            self.assertEqual(host['policy'], policy)
            self.assertEqual(observed, f.observed)
            self.assertEqual(len(files), 13)
            self.assertTrue(owner['outer_pidfd_observed'])
            self.assertIsInstance(structural, dict)

    def test_real_diagnostic_candidate_and_original_six_progress_lines(self):
        for mode in ('teacher_forced', 'autoregressive'):
            f = example(mode); observed, files, structural, owner, host = run_candidate(f)
            self.assertEqual(observed, f.observed); self.assertEqual(len(files), 13)
            self.assertEqual(owner['child_pid'], 17); self.assertTrue(owner['outer_pidfd_observed'])
            self.assertTrue(host['native_closed']); self.assertFalse(host['gpu_time'])
            self.assertIsInstance(structural, dict)

    def test_plain_argv_extra_option_and_wrong_owned_bounds_refuse(self):
        for key in ('plain', 'extra', 'deadline_seconds', 'address_space_bytes', 'stream_cap_bytes'):
            f = example(); command = copy.deepcopy(f.command)
            if key == 'plain': command['argv'].pop()
            elif key == 'extra': command['argv'].append('--cache')
            else: command[key] += 1
            f.value['command'] = f.retain('/task/command.json', encode(command))
            f.started['command_sha256'] = f.value['command']['sha256']
            f.value['started'] = f.retain('/task/started.json', encode(f.started)); refresh(f)
            with self.subTest(key=key), self.assertRaises(ValueError): run_candidate(f)

    def test_owner_close_lineage_stderr_and_sidecar_loss_refuse(self):
        for key in ('owner', 'lineage', 'stderr', 'sidecar', 'control', 'raw-summary'):
            f = example()
            if key == 'owner': f.owner['owned_processes_reaped'] = False; refresh(f)
            elif key == 'lineage': f.owner['lineage'][-1]['identity']['ppid'] = 999; refresh(f)
            elif key == 'stderr': f.value['stderr'] = f.retain('/task/stderr', b'unexpected\n'); refresh(f)
            elif key == 'sidecar': f.bodies[f.value['host_sidecar']['path']] = b'{}'
            elif key == 'control': f.bodies[f.value['native_files']['control-2.bin']['path']] = b'bad'
            else:
                f.value['stdout'] = f.retain('/task/stdout', f.raw)
                f.owner['stdout'] = f.value['stdout']; f.value['owner'] = f.retain('/task/result.json', encode(f.owner))
            with self.subTest(key=key), self.assertRaises((RuntimeError, ValueError)): run_candidate(f)

    def test_comparison_calls_frozen_math_twice_and_keeps_independent_scope_false(self):
        f = example(); read = lambda p, _: f.bodies[p['path']]
        observed, files, _, _, _ = run_candidate(f)
        plan = dict(schema='ferric-p228-prefix-decode-host-policy-comparison-inputs-v2', mode='autoregressive', policy='baseline',
            candidate=f.value, baseline={}, reference={})
        rows = [dict(same_input_history=True, tensors=[dict(byte_equal=True)] * 38) for _ in range(4)]
        with patch.object(f.C, 'baseline', return_value=(copy.deepcopy(observed), files)), \
             patch.object(f.C, 'reference', return_value=(f.C.records(observed), [files[f'observation-{i}.bin'] for i in range(4)])), \
             patch.object(f.C, 'compare_rows', return_value=rows) as compare:
            result = M.compare(f.C, plan, read, f.helpers)
            self.assertEqual(compare.call_count, 2)
            self.assertTrue(result['full152_tensor_rows_bitwise_equal'])
            self.assertFalse(result['independent_tensor_acceptance']); self.assertIsNone(result['independent_tensor_threshold'])
            self.assertFalse(result['production_authority']); self.assertIn('host_observation', result)
            compare.side_effect = [[dict(same_input_history=False, tensors=None)] * 4, rows]
            self.assertFalse(M.compare(f.C, plan, read, f.helpers)['full152_tensor_rows_bitwise_equal'])

    def test_baseline_image_and_rotary_identity_refuse_before_comparison(self):
        f = example(); read = lambda p, _: f.bodies[p['path']]
        observed, files, _, _, _ = run_candidate(f)
        plan = dict(schema='ferric-p228-prefix-decode-host-policy-comparison-inputs-v2', mode='autoregressive', policy='baseline',
            candidate=f.value, baseline={}, reference={})
        for key in ('image', 'rotary'):
            previous = copy.deepcopy(observed); bodies = dict(files)
            if key == 'image': previous['request']['tiles_image']['sha256'] = [0] * 32
            else:
                q = f.C.document(bodies['request-2.json']); q['command']['rotary_bits'][0] = 0
                bodies['request-2.json'] = encode(q)
            with patch.object(f.C, 'baseline', return_value=(previous, bodies)), \
                 patch.object(f.C, 'compare_rows') as compare, self.assertRaises(ValueError):
                M.compare(f.C, plan, read, f.helpers)
            compare.assert_not_called()


if __name__ == '__main__': unittest.main()
