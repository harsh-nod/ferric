"""Synthetic arithmetic tests; no model, runtime or GPU execution."""
import copy
import unittest

import analyze as A


def fixture():
    snapshots = []
    time_columns = {1, 3, 7, 9, 10, 11, 15, 18}
    for index in range(587):
        if index == 0: phase = 'fresh_enabled'
        elif index == 1: phase = 'setup_sealed'
        elif index == 586: phase = 'before_close'
        else:
            forward, offset = divmod(index - 2, 146)
            if offset in (0, 145): phase = 'forward_%d/%s' % (forward, 'begin' if offset == 0 else 'done')
            else:
                layer, step = divmod(offset - 1, 4)
                phase = 'forward_%d/layer_%02d/%s' % (forward, layer, ('begin', 'prefix', 'paired', 'hidden')[step])
        ranks = []
        for rank, identity in enumerate((16366993098680759275, 10838076764495710945)):
            values = [index * (rank + 1) * (column + 1) * (1000 if column in time_columns else 1)
                      for column in range(19)]
            values[4:6] = [0, 0]
            ranks.append(dict(rank=rank, unique_id=identity, queue_epoch=(1 << 53) + rank,
                              cache_kernel_admission=False, raw_timestamp_queue=False, counters=values))
        snapshots.append(dict(phase=phase, group_incarnation=(1 << 63) + 7,
            shared_full_currentness=False, ranks=ranks, shared=[index * x for x in (1, 1000, 2, 2000)]))
    intervals = []
    for index, (previous, current) in enumerate(zip(snapshots, snapshots[1:])):
        intervals.append(dict(host_elapsed_ns=index + 1,
            ranks=[[a - b for a, b in zip(current['ranks'][rank]['counters'], previous['ranks'][rank]['counters'])]
                   for rank in range(2)], shared=[a - b for a, b in zip(current['shared'], previous['shared'])]))
    flags = dict(inclusive_nested_host_scopes=True, paired_generic_dispatch_timers_complete=False,
        tensor_stage_capture=False, gpu_time=False, gpu_overlap=False, numerical_acceptance=False,
        full_model_acceptance=False, performance_claim=False, production_authority=False)
    report = dict(schema='FerricGuardedMlpHostObservationV1', snapshots=snapshots, intervals=intervals,
                  forward_host_ns=[1, 2, 3, 4], close_host_ns=5, native_closed=True, **flags)
    rows = []
    for forward in range(4):
        begin = 2 + 146 * forward
        layers = [dict(layer=layer, prefix=intervals[begin + 4 * layer + 1],
                       paired=intervals[begin + 4 * layer + 2], hidden_read=intervals[begin + 4 * layer + 3])
                  for layer in range(36)]
        rows.append(dict(position=forward, forward_host_ns=forward + 1,
                         bracket_host_ns=sum(range(begin + 1, begin + 146)), layers=layers))
    checked = dict(schema='ferric-guarded-mlp-model-host-observation-checked-v1', snapshots=587, intervals=586,
        counter_names=list(A.COUNTERS), shared_counter_names=list(A.SHARED), forward_rows=rows, close_host_ns=5,
        final_rank_counters=[r['counters'] for r in snapshots[-1]['ranks']],
        final_shared_counters=snapshots[-1]['shared'], native_close_confirmed=True, **flags)
    return copy.deepcopy(report), copy.deepcopy(checked)


class AnalysisTests(unittest.TestCase):
    def reject(self, report, checked):
        with self.assertRaises((ValueError, KeyError, TypeError)):
            A.analyze(report, checked)

    def test_exact_disjoint_wall_partition_and_total(self):
        result = A.analyze(*fixture())
        self.assertEqual(result['snapshot_wall_ns'], 586 * 587 // 2)
        self.assertEqual(result['nonforward_intervals'], [0, 1, 147, 293, 439, 585])
        self.assertEqual(result['nonforward_wall_ns'], 1 + 2 + 148 + 294 + 440 + 586)
        self.assertEqual(result['forward_intervals'], 580)
        for forward, row in enumerate(result['forwards']):
            first = 2 + 146 * forward
            self.assertEqual(row['category_interval_counts'], dict(prefix=36, paired=36, hidden=36, other=37))
            expected = {key: 36 * (first + offset + 2) + 4 * 35 * 36 // 2
                        for offset, key in enumerate(('prefix', 'paired', 'hidden'))}
            expected['other'] = 37 * (first + 1) + 4 * 36 * 37 // 2
            self.assertEqual(row['disjoint_wall_ns'], expected)
            self.assertEqual(sum(expected.values()), row['bracket_host_ns'])

    def test_inclusive_counters_telescope_without_wall_addition(self):
        result = A.analyze(*fixture())
        for row in result['forwards']:
            for rank, counters in enumerate(row['inclusive_rank_counters']):
                self.assertEqual(counters['full_currentness_checks'], 145 * 3 * (rank + 1))
                self.assertEqual(counters['full_currentness_ns'], 145 * 4000 * (rank + 1))
                self.assertEqual(counters['kernel_admissions'], 145 * 7 * (rank + 1))
                self.assertGreater(counters['full_currentness_ns'], row['bracket_host_ns'])
            self.assertEqual(row['inclusive_shared_counters'], dict(group_full_checks=145, group_full_ns=145000,
                                                                  publication_full_checks=290, publication_full_ns=290000))
        self.assertFalse(result['nested_counters_added_to_elapsed'])

    def test_closed_phase_schedule_and_census(self):
        for key in ('snapshots', 'intervals'):
            for delta in (-1, 1):
                report, checked = fixture()
                if delta < 0: report[key].pop()
                else: report[key].append(copy.deepcopy(report[key][-1]))
                self.reject(report, checked)
        report, checked = fixture(); report['snapshots'][3]['phase'] = 'forward_0/layer_00/paired'
        self.reject(report, checked)

    def test_counter_decrease_and_wrong_delta(self):
        for shared in (False, True):
            report, checked = fixture()
            row = report['snapshots'][5]['shared'] if shared else report['snapshots'][5]['ranks'][0]['counters']
            row[0] = 0
            self.reject(report, checked)
            report, checked = fixture()
            row = report['intervals'][5]['shared'] if shared else report['intervals'][5]['ranks'][0]
            row[0] += 1
            self.reject(report, checked)

    def test_strict_u64_not_bool_float_or_overflow(self):
        for value in (True, 1.0, -1, 1 << 64):
            report, checked = fixture(); report['intervals'][0]['host_elapsed_ns'] = value
            self.reject(report, checked)
            report, checked = fixture(); report['snapshots'][5]['ranks'][1]['counters'][1] = value
            self.reject(report, checked)
        report, checked = fixture(); checked['snapshots'] = 587.0
        self.reject(report, checked)
        report, checked = fixture(); checked['close_host_ns'] = 5.0
        self.reject(report, checked)

    def test_forward_and_whole_sum_overflow_refuse(self):
        report, checked = fixture(); report['intervals'][2]['host_elapsed_ns'] = (1 << 64) - 1
        self.reject(report, checked)
        report, checked = fixture(); report['intervals'][0]['host_elapsed_ns'] = (1 << 64) - 1
        self.reject(report, checked)
        report, checked = fixture(); report['forward_host_ns'][0] = 1 << 63
        self.reject(report, checked)

    def test_checked_raw_cross_joins(self):
        report, checked = fixture(); checked['final_rank_counters'][0][0] += 1
        self.reject(report, checked)
        report, checked = fixture(); checked['forward_rows'][0]['layers'][0]['prefix']['host_elapsed_ns'] += 1
        self.reject(report, checked)
        report, checked = fixture(); checked['forward_rows'][1]['position'] = True
        self.reject(report, checked)
        report, checked = fixture(); checked['shared_counter_names'].reverse()
        self.reject(report, checked)

    def test_policy_identity_and_fresh_baseline(self):
        for key, value in (('unique_id', 39903), ('queue_epoch', 1), ('cache_kernel_admission', True)):
            report, checked = fixture(); report['snapshots'][4]['ranks'][0][key] = value
            self.reject(report, checked)
        report, checked = fixture(); report['snapshots'][0]['shared'][0] = 1
        self.reject(report, checked)
        report, checked = fixture(); report['snapshots'][5]['shared_full_currentness'] = True
        self.reject(report, checked)

    def test_close_and_no_authority_upgrade(self):
        for key in ('native_closed', 'inclusive_nested_host_scopes', 'gpu_time', 'gpu_overlap',
                    'numerical_acceptance', 'full_model_acceptance', 'performance_claim', 'production_authority'):
            report, checked = fixture(); report[key] = not report[key]
            self.reject(report, checked)
        result = A.analyze(*fixture())
        for key in ('gpu_time', 'gpu_overlap', 'throughput', 'numerical_acceptance', 'full_model_acceptance',
                    'performance_claim', 'production_authority', 'paired_generic_dispatch_timers_complete'):
            self.assertFalse(result[key])

    def test_markdown_exact_integer_seconds_and_scope(self):
        self.assertEqual(A.seconds((1 << 64) - 1), '18446744073.709551615')
        text = A.markdown(A.analyze(*fixture()))
        self.assertIn('inclusive and may nest', text)
        self.assertIn('outside those intervals', text)
        self.assertIn('0.000000005', text)
        self.assertEqual(len([line for line in text.splitlines() if line.startswith('| 0 |')]), 4)


if __name__ == '__main__':
    unittest.main(verbosity=2)
