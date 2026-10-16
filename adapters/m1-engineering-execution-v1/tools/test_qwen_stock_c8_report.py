import copy
import unittest

import qwen_stock_c8_report as report


class StockC8Test(unittest.TestCase):
    def fixture(self):
        ids = ['r' + str(i) for i in range(8)]
        records = [{'id': name, 'success': True, 'started_ns': i + 1, 'completed_ns': 900,
                    'token_ids': [1] * 256, 'ttft_ns': 10, 'tpot_ns': 1, 'e2e_ns': 899 - i,
                    'usage': {'prompt_tokens': 128, 'completion_tokens': 256, 'total_tokens': 384}}
                   for i, name in enumerate(ids)]
        run = {'schema': 'FerricCompetitiveTokenStreamingRunV1', 'engine': 'vllm',
                'concurrency': 8, 'arrival_policy': 'bounded-closed-loop-windows', 'completed': True,
                'workload': {'schema': 'FerricCompetitiveTokenWorkloadV1', 'requests':
                             [{'id': name, 'prompt': [1] * 128, 'max_tokens': 256} for name in ids]},
                **{phase: [{'index': i, 'started_ns': 0, 'completed_ns': 1000,
                            'requests': copy.deepcopy(records)} for i in range(n)]
                   for phase, n in [('warmups', 10), ('samples', 30)]}}
        ordinal = 0
        for phase in ['warmups', 'samples']:
            for w in run[phase]:
                start = ordinal * 1001
                w.update(started_ns=start, completed_ns=start + 1000)
                for record in w['requests']:
                    record['started_ns'] += start
                    record['completed_ns'] += start
                ordinal += 1
        return run

    def test_full_counts_overlap_and_mismatch(self):
        s = report.summarize(self.fixture(), [2] * 256, [1] * 256)
        self.assertTrue(s['complete_declared_baseline'])
        self.assertEqual(s['counts']['warmups']['successful_requests'], 80)
        self.assertEqual(s['successful_measured_requests'], 240)
        self.assertEqual(s['successful_output_tokens'], 61440)
        self.assertFalse(s['performance_qualified'])
        self.assertTrue(all(not p['reference_equal'] and p['retained_C1_stock_equal'] for p in s['parity']))

    def test_sequential_intervals_not_C8(self):
        r = self.fixture()
        for w in r['samples']:
            for i, p in enumerate(w['requests']):
                p.update(started_ns=w['started_ns'] + i * 100,
                         completed_ns=w['started_ns'] + i * 100 + 99, e2e_ns=99, tpot_ns=.1)
        self.assertFalse(report.summarize(r, [1] * 256, [1] * 256)['complete_declared_baseline'])

    def test_partial_failure_retained(self):
        r = self.fixture(); r['completed'] = False; r['samples'] = r['samples'][:1]
        r['samples'][0]['requests'][0].update(success=False, error='timeout')
        s = report.summarize(r, [1] * 256, [1] * 256)
        self.assertEqual(s['counts']['samples']['failed_requests'], 1)
        self.assertEqual(s['counts']['samples']['missing_requests'], 232)
        self.assertEqual(s['successful_measured_requests'], 7)

    def test_duplicate_ids_rejected(self):
        r = self.fixture(); r['workload']['requests'][1]['id'] = 'r0'
        with self.assertRaises(ValueError): report.summarize(r, [1] * 256, [1] * 256)

    def test_ties_end_before_start(self):
        self.assertEqual(report.overlap([{'started_ns': 0, 'completed_ns': 1},
                                        {'started_ns': 1, 'completed_ns': 2}]), 1)

    def test_bad_usage_rejected(self):
        r = self.fixture(); r['samples'][0]['requests'][0]['usage']['completion_tokens'] = 255
        with self.assertRaises(ValueError): report.summarize(r, [1] * 256, [1] * 256)

    def test_variation_is_disclosed(self):
        r = self.fixture(); r['samples'][0]['requests'][0]['token_ids'] = [3] * 256
        self.assertFalse(report.summarize(r, [2] * 256, [1] * 256)['parity'][80]['retained_C1_stock_equal'])

    def test_malformed_window_and_request_endpoints(self):
        for endpoint in ['started_ns', 'completed_ns']:
            for value in [float('nan'), float('inf'), True, False, 1.5, '1', None, -1, 2**63]:
                for target in ['window', 'request']:
                    with self.subTest(endpoint=endpoint, value=value, target=target):
                        r = self.fixture(); w = r['samples'][0]
                        (w if target == 'window' else w['requests'][0])[endpoint] = value
                        with self.assertRaises(ValueError): report.summarize(r, [1] * 256, [1] * 256)

    def test_outside_and_reversed_request_intervals(self):
        for failure in ['before', 'after', 'zero', 'reverse']:
            for success in [True, False]:
                with self.subTest(failure=failure, success=success):
                    r = self.fixture(); w = r['samples'][0]; p = w['requests'][0]; p['success'] = success
                    if failure == 'before': p['started_ns'] = w['started_ns'] - 1
                    if failure == 'after': p['completed_ns'] = w['completed_ns'] + 1
                    if failure == 'zero': p['completed_ns'] = p['started_ns']
                    if failure == 'reverse': p['completed_ns'] = p['started_ns'] - 1
                    with self.assertRaises(ValueError): report.summarize(r, [1] * 256, [1] * 256)

    def test_positive_but_clipped_window_rejected(self):
        r = self.fixture(); w = r['samples'][0]
        w['completed_ns'] = w['started_ns'] + 10
        with self.assertRaises(ValueError): report.summarize(r, [1] * 256, [1] * 256)

    def test_overlapping_reordered_and_cross_phase_windows(self):
        for failure in ['overlap', 'reorder', 'cross_phase', 'zero', 'reverse']:
            with self.subTest(failure=failure):
                r = self.fixture(); first, second = r['samples'][:2]
                if failure == 'overlap':
                    for k in ['started_ns', 'completed_ns']:
                        second[k] -= 2
                        for p in second['requests']: p[k] -= 2
                if failure == 'reorder':
                    for k in ['started_ns', 'completed_ns', 'requests']:
                        first[k], second[k] = second[k], first[k]
                if failure == 'cross_phase':
                    for k in ['started_ns', 'completed_ns', 'requests']:
                        first[k] = copy.deepcopy(r['warmups'][-1][k])
                if failure == 'zero': first['completed_ns'] = first['started_ns']
                if failure == 'reverse': first['completed_ns'] = first['started_ns'] - 1
                with self.assertRaises(ValueError): report.summarize(r, [1] * 256, [1] * 256)

    def test_run_error_prevents_complete_without_erasing_samples(self):
        for error in ['failed after final window', '', False, 0]:
            r = self.fixture(); r['run_error'] = error
            s = report.summarize(r, [1] * 256, [1] * 256)
            self.assertFalse(s['complete_declared_baseline'])
            self.assertEqual(s['run_error'], error)
            self.assertEqual(s['successful_measured_requests'], 240)

    def test_touching_windows_allowed(self):
        r = self.fixture(); w = r['samples'][0]
        for k in ['started_ns', 'completed_ns']:
            w[k] -= 1
            for p in w['requests']: p[k] -= 1
        self.assertTrue(report.summarize(r, [1] * 256, [1] * 256)['complete_declared_baseline'])
