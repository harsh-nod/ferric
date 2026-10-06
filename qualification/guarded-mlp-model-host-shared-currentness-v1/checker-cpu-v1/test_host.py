"""Synthetic additional-checker tests; ordinary model admission is a precondition."""
import copy
import hashlib
import json
import unittest

import run_model_gpu as R
import validate_observation as V


def encode(value):
    return json.dumps(value, separators=(',', ':'), allow_nan=False).encode()


def pin(path, raw):
    return dict(path=path, bytes=len(raw), sha256=list(hashlib.sha256(raw).digest()))


def fixture():
    bootstrap = dict(schema='FerricGuardedMlpDecodeBootstrapV1', decode=dict(
        device_ids=list(R.IDS), scope=dict(child_identity=12345, session=[9] * 32)))
    events = [dict(generation=i + 1, position=i, input_token=[9112, 67, 25, 576][i],
                   output_token=[67, 25, 576, 2701][i], chain=[i + 1] * 32) for i in range(4)]
    summary = dict(bootstrap=bootstrap, profile_sha256=[7] * 32, child_pid=12345,
        request=dict(decode=dict(worker=dict(sha256=[8] * 32))), input_tokens=[9112, 67, 25, 576],
        files=dict(frames=[dict(response=dict(event=dict(status='completed', **event))) for event in events]))
    phases = ['fresh_enabled', 'setup_sealed']
    for forward in range(4):
        phases.append('forward_%d/begin' % forward)
        for layer in range(36):
            phases.extend('forward_%d/layer_%02d/%s' % (forward, layer, step)
                          for step in ('begin', 'prefix', 'paired', 'hidden'))
        phases.append('forward_%d/done' % forward)
    phases.append('before_close')
    snapshots = []
    for index, phase in enumerate(phases):
        ranks = []
        for rank in range(2):
            counters = [index * (column + 1) * (rank + 1) for column in range(19)]
            counters[4:6] = [0, 0]
            ranks.append(dict(rank=rank, unique_id=R.IDS[rank], queue_epoch=(1 << 53) + rank,
                cache_kernel_admission=False, raw_timestamp_queue=False, counters=counters))
        snapshots.append(dict(phase=phase, group_incarnation=(1 << 63) + 7,
            shared_full_currentness=True, ranks=ranks, shared=[index * (i + 1) for i in range(4)]))
    intervals = []
    for index in range(586):
        previous, current = snapshots[index:index + 2]
        intervals.append(dict(host_elapsed_ns=index + 1,
            ranks=[[a - b for a, b in zip(current['ranks'][rank]['counters'], previous['ranks'][rank]['counters'])]
                   for rank in range(2)], shared=[a - b for a, b in zip(current['shared'], previous['shared'])]))
    report = dict(schema='FerricGuardedMlpSharedFullHostObservationV1', bootstrap=copy.deepcopy(bootstrap),
        worker_sha256=[8] * 32, child_pid=12345, profile_sha256=[7] * 32,
        snapshots=snapshots, intervals=intervals, forward_host_ns=[1, 2, 3, 4], close_host_ns=5,
        completions=events, native_closed=True, inclusive_nested_host_scopes=True,
        paired_generic_dispatch_timers_complete=False, tensor_stage_capture=False, gpu_time=False,
        gpu_overlap=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False, production_authority=False)
    return report, summary


def check(data, mutate_raw=None):
    report, summary = copy.deepcopy(data)
    raw = encode(report)
    summary['files']['child_stderr'] = pin('/synthetic/host.json', raw)
    return R.host_admission(encode(summary), lambda _: raw if mutate_raw is None else mutate_raw(raw), V)


class HostTests(unittest.TestCase):
    def refuse(self, data):
        with self.assertRaises((RuntimeError, ValueError)):
            check(data)

    def test_valid_closed_report_preserves_u64_and_layer_interval_indices(self):
        data = fixture(); result = check(data)
        self.assertEqual((result['snapshots'], result['intervals']), (587, 586))
        self.assertEqual(len(result['counter_names']), 19)
        self.assertEqual(result['final_rank_counters'], [r['counters'] for r in data[0]['snapshots'][-1]['ranks']])
        for forward, row in enumerate(result['forward_rows']):
            self.assertEqual(len(row['layers']), 36)
            for layer, values in enumerate(row['layers']):
                start = 3 + forward * 146 + layer * 4
                for offset, key in enumerate(('prefix', 'paired', 'hidden_read')):
                    self.assertEqual(values[key], data[0]['intervals'][start + offset])
        self.assertFalse(result['gpu_time']); self.assertFalse(result['throughput'])
        self.assertTrue(result['shared_full_currentness'])

    def test_conservative_schema_and_cross_mode_policy_refuse(self):
        for shared in (False, True):
            data = fixture()
            data[0]['schema'] = 'FerricGuardedMlpHostObservationV1'
            for snapshot in data[0]['snapshots']:
                snapshot['shared_full_currentness'] = shared
            self.refuse(data)
        data = fixture()
        for snapshot in data[0]['snapshots']:
            snapshot['shared_full_currentness'] = False
        self.refuse(data)
        for value in (False, 1, None):
            data = fixture(); data[0]['snapshots'][293]['shared_full_currentness'] = value
            self.refuse(data)

    def test_snapshot_and_interval_rosters_refuse(self):
        for field in ('snapshots', 'intervals'):
            for append in (False, True):
                data = fixture()
                if append: data[0][field].append(copy.deepcopy(data[0][field][-1]))
                else: data[0][field].pop()
                self.refuse(data)
        data = fixture(); data[0]['snapshots'][4]['phase'] = 'forward_0/layer_00/paired'; self.refuse(data)

    def test_exact_report_hash_and_extent_refuse(self):
        with self.assertRaises(RuntimeError): check(fixture(), lambda raw: raw + b' ')
        with self.assertRaises(RuntimeError): check(fixture(), lambda raw: raw[:-1] + b' ')

    def test_actual_worker_bootstrap_profile_pid_and_completion_joins_refuse(self):
        for key, value in [('worker_sha256', [2] * 32), ('profile_sha256', [2] * 32), ('child_pid', True),
                           ('child_pid', 12346), ('bootstrap', {}), ('completions', [])]:
            data = fixture(); data[0][key] = value; self.refuse(data)
        data = fixture(); data[0]['completions'][0]['generation'] = True; self.refuse(data)
        data = fixture(); data[0]['bootstrap']['decode']['scope']['child_identity'] = False; self.refuse(data)

    def test_group_epoch_device_and_policy_drift_refuse(self):
        for key, value in [('group_incarnation', 0), ('group_incarnation', 7), ('shared_full_currentness', False)]:
            data = fixture(); data[0]['snapshots'][5][key] = value; self.refuse(data)
        for key, value in [('rank', True), ('unique_id', 39903), ('queue_epoch', 0),
                           ('cache_kernel_admission', True), ('raw_timestamp_queue', True)]:
            data = fixture(); data[0]['snapshots'][5]['ranks'][0][key] = value; self.refuse(data)

    def test_fresh_baseline_counter_monotonicity_and_exact_deltas_refuse(self):
        data = fixture(); data[0]['snapshots'][0]['shared'][0] = 1; self.refuse(data)
        data = fixture(); data[0]['snapshots'][0]['ranks'][0]['counters'][0] = 1; self.refuse(data)
        data = fixture(); data[0]['snapshots'][5]['ranks'][0]['counters'][0] = 0; self.refuse(data)
        data = fixture(); data[0]['intervals'][4]['ranks'][0][0] += 1; self.refuse(data)
        data = fixture(); data[0]['intervals'][4]['shared'][0] += 1; self.refuse(data)
        data = fixture(); data[0]['snapshots'][5]['ranks'][0]['counters'][4] = 1; self.refuse(data)

    def test_strict_u64_vectors_and_closed_nested_fields_refuse(self):
        for value in (True, -1, 1 << 64, 1.0):
            data = fixture(); data[0]['intervals'][4]['host_elapsed_ns'] = value; self.refuse(data)
            data = fixture(); data[0]['snapshots'][4]['ranks'][0]['counters'][1] = value; self.refuse(data)
        for target in ('report', 'snapshot', 'rank', 'interval'):
            data = fixture(); report = data[0]
            row = {'report': report, 'snapshot': report['snapshots'][0],
                   'rank': report['snapshots'][0]['ranks'][0], 'interval': report['intervals'][0]}[target]
            row['extra'] = 0; self.refuse(data)

    def test_forward_wall_bound_and_sum_overflow_refuse(self):
        data = fixture(); data[0]['forward_host_ns'][0] = 1 << 63; self.refuse(data)
        data = fixture(); data[0]['intervals'][2]['host_elapsed_ns'] = (1 << 64) - 1; self.refuse(data)
        data = fixture(); data[0]['close_host_ns'] = True; self.refuse(data)

    def test_healthy_close_and_all_nonclaims_required(self):
        for key in ('native_closed', 'inclusive_nested_host_scopes', 'paired_generic_dispatch_timers_complete',
                    'tensor_stage_capture', 'gpu_time', 'gpu_overlap', 'numerical_acceptance',
                    'full_model_acceptance', 'performance_claim', 'production_authority'):
            data = fixture(); data[0][key] = not data[0][key]; self.refuse(data)

    def test_output_comparison_reports_differences_and_history_without_acceptance(self):
        _, summary = fixture(); bodies = {}; old = []
        for index, frame in enumerate(summary['files']['frames']):
            raw = bytes(V.PAYLOAD_BYTES); path = '/synthetic/observation-%d.bin' % index
            frame['observation'] = pin(path, raw); bodies[path] = raw
            old.append((V.rust_pin(frame['observation']), raw))
        prior = ({'sha256': 'prior'}, list(summary['input_tokens']), old)
        read = lambda p: bodies[p['path']]
        result = R.compare_payloads(encode(summary), read, V, prior)
        self.assertTrue(result['all_payloads_equal']); self.assertTrue(result['all_histories_equal'])
        path = summary['files']['frames'][0]['observation']['path']; bodies[path] = b'\x01' + bodies[path][1:]
        summary['files']['frames'][0]['observation'] = pin(path, bodies[path])
        summary['input_tokens'][1] = 68
        result = R.compare_payloads(encode(summary), read, V, prior)
        self.assertTrue(result['observed_payload_difference']); self.assertFalse(result['all_payloads_equal'])
        self.assertEqual([r['same_history'] for r in result['frames']], [True, False, False, False])
        for key in ('causal_effect_established', 'independent_accuracy_reference', 'numerical_acceptance', 'performance_claim'):
            self.assertFalse(result[key])


if __name__ == '__main__':
    unittest.main(verbosity=2)
