"""Synthetic policy tests, not actual receipt or GPU qualification."""
import copy
import hashlib
import json
import types
import unittest
from unittest.mock import patch

import run as R

NAMES = ['commands', 'command_ns', 'full_currentness_checks', 'full_currentness_ns',
    'operational_currentness_checks', 'operational_currentness_ns', 'kernel_admissions',
    'kernel_admission_ns', 'dispatches', 'dispatch_prepare_ns', 'dispatch_publish_ns',
    'dispatch_wait_ns', 'completion_polls', 'reads', 'read_bytes', 'read_ns', 'writes',
    'write_bytes', 'write_ns']


def hosts():
    ranks = [[0, 0, 676, 1100, 0, 0, 148, 3, 148, 4, 5, 6, 70, 39, 606980, 8, 3, 1096, 9],
             [0, 0, 656, 1200, 0, 0, 145, 3, 145, 4, 5, 6, 80, 36, 294912, 8, 2, 1092, 9]]
    old = dict(policy='shared-full-currentness', inclusive_nested_host_scopes=True, gpu_time=False,
        calibrated_device_time=False, native_closed=True, counter_names=NAMES,
        shared_counter_names=R.SHARED_NAMES, forward_host_ns=[7000000000] * 4,
        intervals=[dict(shared=[count, 300, 288, 40], ranks=copy.deepcopy(ranks))
                   for count in [0, 2196, 2196, 2484, 2484, 0]])
    new = copy.deepcopy(old)
    new['forward_host_ns'] = [6000000000] * 4
    for interval in new['intervals'][1:5]:
        interval['shared'][0] -= 576
        interval['shared'][1] = 200
        for rank in interval['ranks']:
            rank[NAMES.index('completion_polls')] += 17
            rank[NAMES.index('full_currentness_ns')] += 19
    return old, new


def cases():
    receipt = {key: key for key in ('parent_cpu_complete', 'parent_cpu_review', 'image_deployment',
               'standalone_prepared', 'standalone_cases', 'numericals')}
    receipt['deployment'] = 'CPU475 deployment'
    old = dict(request=dict(worker='oldworker', session=[1], evidence_directory='/old',
        device_ids=[10, 11], mode='teacher_forced', dispatch_timeout_ms=10000, prompt={'sha256': 'prompt'}),
        runtime=dict(parent='parent633', image='V7', worker='oldworker'), receipt=receipt, files={}, records=[])
    for position in range(4):
        old['files'][f'request-{position}.json'] = json.dumps(dict(id=position + 1, protocol='v1',
            device_ids=[10, 11], profile_sha256=[8], session=[1], registration=[2],
            command=dict(op='forward', generation=position + 1, cache_metadata=[position], rotary_bits=[3]))).encode()
        old['files'][f'observation-{position}.bin'] = bytes(606976)
        old['records'].append(dict(generation=position + 1, position=position, input_token=4, output_token=0))
    new = copy.deepcopy(old)
    new['request'].update(worker='newworker', session=[2], evidence_directory='/new')
    new['runtime']['worker'] = 'newworker'
    new['receipt'].update(prior_deployment='CPU475 deployment', deployment='CPU522 deployment')
    for case, profile, registration in ((old, [8], [2]), (new, [9], [3])):
        case['observed'] = dict(profile_sha256=profile, bootstrap=dict(registration=registration,
            scope=dict(session=case['request']['session'])))
        for position in range(4):
            request = json.loads(case['files'][f'request-{position}.json'])
            request.update(profile_sha256=profile, registration=registration, session=case['request']['session'])
            case['files'][f'request-{position}.json'] = json.dumps(request).encode()
    return old, new


C = types.SimpleNamespace(document=json.loads)
H = types.SimpleNamespace(same=lambda a, b: json.dumps(a, sort_keys=True) == json.dumps(b, sort_keys=True))


class FakeDiagnostics:
    """Only a partition/routing double; real arithmetic validators are frozen."""
    def validate_case(self, record, raw, position):
        R.require(record['position'] == position, 'position')
        rows = {f'layer{index}-hidden': raw[index * 8192:(index + 1) * 8192] for index in range(36)}
        rows.update({'final-norm': raw[294912:303104], 'logits': raw[303104:]})
        return rows


class WorkloadTests(unittest.TestCase):
    def test_fresh_session_bound_profiles_need_not_match(self):
        a, b = cases()
        self.assertNotEqual(a['observed']['profile_sha256'], b['observed']['profile_sha256'])
        R.same_workload(a, b, C, H)

    def test_each_forward_must_bind_its_own_profile_registration_and_session(self):
        for key in ('profile_sha256', 'registration', 'session'):
            a, b = cases()
            request = json.loads(b['files']['request-3.json'])
            request[key] = [99]
            b['files']['request-3.json'] = json.dumps(request).encode()
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, 'own authenticated'):
                R.same_workload(a, b, C, H)

    def test_only_expected_changes_are_allowed(self):
        a, b = cases()
        before = copy.deepcopy((a, b))
        R.same_workload(a, b, C, H)
        self.assertEqual((a, b), before)

    def test_same_session_is_refused(self):
        a, b = cases()
        b['request']['session'] = a['request']['session']
        with self.assertRaisesRegex(ValueError, 'fresh candidate'):
            R.same_workload(a, b, C, H)

    def test_workload_and_parent_image_drift_are_refused(self):
        for key in ('prompt', 'device_ids', 'dispatch_timeout_ms', 'parent', 'image'):
            a, b = cases()
            if key in ('parent', 'image'):
                b['runtime'][key] = 'changed'
            else:
                b['request'][key] = 'changed'
            with self.subTest(key=key), self.assertRaises(ValueError):
                R.same_workload(a, b, C, H)

    def test_request_field_and_bool_drift_are_refused(self):
        for mutation in ('extra', 'type'):
            a, b = cases()
            if mutation == 'extra':
                b['request']['unexpected'] = 0
            else:
                a['request']['dispatch_timeout_ms'] = 0
                b['request']['dispatch_timeout_ms'] = False
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                R.same_workload(a, b, C, H)

    def test_prerequisite_or_prior_deployment_drift_refused(self):
        for key in ('parent_cpu_complete', 'numericals', 'standalone_cases', 'prior_deployment'):
            a, b = cases()
            b['receipt'][key] = 'changed'
            with self.subTest(key=key), self.assertRaises(ValueError):
                R.same_workload(a, b, C, H)

    def test_forward_command_and_mapping_drift_refused(self):
        a, b = cases()
        row = json.loads(b['files']['request-3.json'])
        row['command']['cache_metadata'] = [100]
        b['files']['request-3.json'] = json.dumps(row).encode()
        with self.assertRaisesRegex(ValueError, 'forward input'):
            R.same_workload(a, b, C, H)


class TensorTests(unittest.TestCase):
    def test_all152_actual_slice_hashes_and_records(self):
        a, b = cases()
        rows = R.tensor_comparison(a, b, FakeDiagnostics())
        self.assertEqual(sum(len(row['tensors']) for row in rows), 152)
        self.assertTrue(all(row['payload_byte_equal'] and row['record_equal'] for row in rows))
        self.assertEqual(rows[0]['tensors'][0]['old_sha256'], hashlib.sha256(bytes(8192)).hexdigest())

    def test_single_tensor_difference_is_retained(self):
        a, b = cases()
        raw = bytearray(b['files']['observation-2.bin'])
        raw[294912] = 1
        b['files']['observation-2.bin'] = bytes(raw)
        rows = R.tensor_comparison(a, b, FakeDiagnostics())
        self.assertFalse(rows[2]['payload_byte_equal'])
        changed = [item['name'] for row in rows for item in row['tensors'] if not item['byte_equal']]
        self.assertEqual(changed, ['final-norm'])

    def test_token_difference_is_retained(self):
        a, b = cases()
        b['records'][2]['output_token'] = 8
        rows = R.tensor_comparison(a, b, FakeDiagnostics())
        self.assertFalse(rows[2]['record_equal'])
        self.assertTrue(rows[2]['payload_byte_equal'])

    def test_truncated_payload_or_missing_forward_refused(self):
        for mutation in ('truncate', 'missing'):
            a, b = cases()
            if mutation == 'truncate':
                b['files']['observation-3.bin'] = bytes(606974)
            else:
                b['records'].pop()
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                R.tensor_comparison(a, b, FakeDiagnostics())

    def test_real_validator_failure_is_propagated(self):
        a, b = cases()
        with patch.object(FakeDiagnostics, 'validate_case', side_effect=RuntimeError('capture pin failure')):
            with self.assertRaisesRegex(RuntimeError, 'capture pin failure'):
                R.tensor_comparison(a, b, FakeDiagnostics())


class CounterTests(unittest.TestCase):
    def test_expected576_and_rank_invariance_with_variable_polls(self):
        rows = R.host_comparison(*hosts())
        self.assertEqual([row['new_shared']['group_full_checks'] for row in rows], [1620, 1620, 1908, 1908])
        self.assertTrue(all(row['expected_removal_observed'] and row['publication_counts_equal'] for row in rows))
        self.assertTrue(all(rank['invariant_counts_equal'] for row in rows for rank in row['ranks']))
        self.assertEqual(rows[0]['old_over_new_host_duration_ratio'], 7 / 6)

    def test_wrong_removal_and_publication_counts_remain_failed_results(self):
        old, new = hosts()
        new['intervals'][2]['shared'][0] += 1
        new['intervals'][3]['shared'][2] -= 1
        rows = R.host_comparison(old, new)
        self.assertFalse(rows[1]['expected_removal_observed'])
        self.assertEqual(rows[1]['removed_group_full_checks'], 575)
        self.assertFalse(rows[2]['publication_counts_equal'])

    def test_each_rank_count_read_write_or_dispatch_change_is_flagged(self):
        for key in R.INVARIANT_COUNTS:
            old, new = hosts()
            new['intervals'][1]['ranks'][1][NAMES.index(key)] += 1
            row = R.host_comparison(old, new)[0]['ranks'][1]
            with self.subTest(key=key):
                self.assertFalse(row['invariant_counts_equal'])
                self.assertEqual(row['differing_invariant_counts'], [key])

    def test_bool_negative_duration_and_wrong_layout_refused(self):
        for mutation in ('bool', 'negative', 'duration', 'layout'):
            old, new = hosts()
            if mutation == 'bool':
                new['intervals'][1]['shared'][0] = True
            elif mutation == 'negative':
                new['intervals'][1]['ranks'][0][0] = -1
            elif mutation == 'duration':
                new['forward_host_ns'][0] = 0
            else:
                new['shared_counter_names'] = list(reversed(R.SHARED_NAMES))
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                R.host_comparison(old, new)

    def test_policy_and_device_time_claim_refused(self):
        for key, value in (('policy', 'baseline'), ('gpu_time', True), ('calibrated_device_time', True)):
            old, new = hosts()
            new[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                R.host_comparison(old, new)


class ReportTests(unittest.TestCase):
    def test_summary_requires_every_invariance(self):
        tensors = R.tensor_comparison(*cases(), FakeDiagnostics())
        counters = R.host_comparison(*hosts())
        self.assertTrue(R.summarize(tensors, counters)['observed_invariance_passed'])
        for mutation in ('token', 'payload', 'tensor', 'removal', 'publication', 'rank'):
            t, c = copy.deepcopy((tensors, counters))
            if mutation == 'token':
                t[0]['record_equal'] = False
            elif mutation == 'payload':
                t[0]['payload_byte_equal'] = False
            elif mutation == 'tensor':
                t[0]['tensors'][0]['byte_equal'] = False
            elif mutation == 'removal':
                c[0]['expected_removal_observed'] = False
            elif mutation == 'publication':
                c[0]['publication_counts_equal'] = False
            else:
                c[0]['ranks'][0]['invariant_counts_equal'] = False
            with self.subTest(mutation=mutation):
                self.assertFalse(R.summarize(t, c)['observed_invariance_passed'])

    def test_table_is_explicitly_host_only_and_uses_supplied_measurements(self):
        value = R.table(R.host_comparison(*hosts()))
        self.assertIn('not GPU time or a qualified speedup', value)
        self.assertIn('7000.000 | 6000.000 | 2196/1620 | 576', value)
        self.assertIn('must not be summed', value)

    def test_main_publishes_pin_before_nonzero_invariance_exit(self):
        with patch.object(R, 'execute', return_value=({'sha256': 'result'}, False)), \
             patch.object(R.sys, 'argv', ['run.py', '--candidate-complete', '/candidate',
                 '--candidate-sha', 'a' * 64, '--output', '/output']), patch('builtins.print') as printed:
            self.assertEqual(R.main(), 1)
            printed.assert_called_once_with(json.dumps({'sha256': 'result'}, sort_keys=True))


if __name__ == '__main__':
    unittest.main()
