"""Source-only fixtures; no native launch or performance samples."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parent


def load(name, path, digest=None):
    if digest is not None and hashlib.sha256(path.read_bytes()).hexdigest() != digest:
        raise ValueError('frozen chronological fixture changed')
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


m = load('splitk_selection', ROOT / 'measurement/splitk_selection.py')
runner = load('splitk_frozen_runner', ROOT / 'fixtures/frozen_run_v17_native.py',
              '810a22cb4e805662a7575e2e4ec261fe161a43405f675a7b639f45a62664301f')
legacy = load('splitk_frozen_split_events', ROOT / 'fixtures/frozen_run_v25_native.py',
              '8c3f6cf9677df9530bc59ebc0a3cbba2355215e002a60a01f9d002c456ea0920')


def spec(arm='A'):
    value = {'arm': arm, 'splitk': {'image': {'path': '/private/splitk', **m.IMAGE},
        'roster': {'path': '/private/roster.json', 'sha256': 'a' * 64,
                   'value': [{'logical_name': name, 'export_name': name} for name in m.ROOTS]}},
        'argv': ['/private/same-controller']}
    options = {'--splitk-down-mode': m.ARMS[arm], '--splitk-down-artifact': '/private/splitk',
        '--splitk-down-roster': '/private/roster.json', '--splitk-down-roster-sha256': 'a' * 64,
        '--splitk-down-hsaco-sha256': m.IMAGE['hsaco'],
        '--splitk-down-manifest-sha256': m.IMAGE['manifest'],
        '--splitk-down-handoff-sha256': m.IMAGE['handoff']}
    for name, item in options.items():
        value['argv'].extend((name, item))
    return value


def stream(arm):
    reference = {'prompt_token_ids': list(range(128)), 'generated_token_ids': list(range(128)),
                 'generated_utf8_hex': '6162'}
    identity = {'schema': runner.EVENT, 'authority': 'none', 'request_id': 1, 'name': 'sample'}
    rows = [{**identity, 'event': 'queued', 'emission_started_ns': 2, 'arrival_ns': 1,
             'queued_ns': 2, 'prompt_tokens': 128},
            {**identity, 'event': 'admission', 'emission_started_ns': 3, 'arrival_ns': 1,
             'admitted_ns': 3, 'queue_wait_ns': 2, 'prompt_tokens': reference['prompt_token_ids'],
             'prompt_token_count': 128, 'cached_tokens': 0, 'cached_pages': 0}]
    stamps = []
    for index in range(135):
        prefill, outputs = index < 8, int(index >= 7)
        start, end = 100 + index * 100, 150 + index * 100
        if outputs:
            token = len(stamps)
            stamps.append(end)
            rows.append({**identity, 'event': 'token', 'emission_started_ns': end,
                'completed_ns': end, 'index': token, 'token': token, 'finished': token == 127})
        rows.append({**identity, 'event': 'batch', 'emission_started_ns': end + 1,
            'tick': index, 'batch_id': index + 1, 'started_ns': start, 'completed_ns': end,
            'rows': 16 if prefill else 1, 'outputs': outputs, 'output_head_rows': outputs,
            'rank_dispatch_counts': [613 + 3 * outputs if prefill else (688 if arm == 'B' else 652)]})
    rows.append({**identity, 'event': 'request', 'emission_started_ns': stamps[-1] + 2,
        'admitted': True, 'state': 'Completed', 'cancelled_ns': None, 'cached_prefix_tokens': 0,
        'arrival_ns': 1, 'prompt_tokens': reference['prompt_token_ids'], 'prompt_token_count': 128,
        'generated_tokens': reference['generated_token_ids'], 'output_timestamps_ns': stamps,
        'generated_utf8_bytes': [97, 98], 'generated_text': 'ab', 'ttft_ns': stamps[0] - 1,
        'tpot_ns': (stamps[-1] - stamps[0]) // 127,
        'decode_intervals_ns': [after - before for before, after in zip(stamps, stamps[1:])]})
    return rows, reference


class Stream:
    def __init__(self, rows):
        self.rows = list(rows)

    def next(self, _deadline):
        if not self.rows:
            raise ValueError('truncated stream')
        return self.rows.pop(0)


def replay(arm, rows, reference):
    events = m.Events(runner, legacy, arm)
    result = runner.collect_request(Stream(rows), events, 1, 'sample', reference, 0)
    return result, events


cell = load('splitk_test_cell', ROOT / 'measurement/native_token_cell.py')
frozen = runner
request_stream = stream


def fixture(arm='A', mode='latency'):
    count = 6 if mode == 'latency' else 1
    mechanism = cell.expected_mechanism(arm)
    selection_spec = spec(arm)
    controller = {'path': '/private/same-controller', 'sha256': 'a' * 64}
    worker = {'path': '/private/worker', 'sha256': 'c' * 64}
    flags = {'--worker': worker['path'],
             '--worker-sha256': worker['sha256'], '--device-unique-id': '123',
             '--max-batches': str(count * mechanism['model_batches']), '--context': '8192', '--pages': '512',
             '--submission': 'ordered', '--wave-target-mode': 'combined', '--layer-projection': 'c1-wave',
             '--prefill-kv-mode': 'parallel-prefill16-v27', '--split-attention-mode': 'split8-v21',
             '--c1-packet-mode': 'packed64-v29', '--gemv-mode': 'baseline',
             '--ordered64-kv-copy-mode': 'parallel-c1-v19'}
    argv = selection_spec['argv']
    for key, value in flags.items():
        argv.extend((key, value))
    argv.extend(('--live-stdin', '--runtime-cache-admission', '--runtime-operational', '--queue-rollover',
                 '--disable-prefix-cache', '--prune-output-head'))
    value = {'schema': 'FerricSplitKModelCellPlanV1', 'arm': arm, 'mode': mode, 'argv': argv,
            'splitk': selection_spec['splitk'],
            'experimental_retention': {'path': '/private/retention.json', 'sha256': 'f' * 64},
            'controller': controller, 'worker': worker, 'device_unique_id': 123, 'prompt': 'frozen prompt',
            'reference': {'prompt_token_ids': list(range(128)), 'generated_token_ids': list(range(128)),
                          'generated_utf8_hex': '6162'},
            'setup_expected': {'model_bundle_id': 'd' * 64, 'target_model_id': 'e' * 64, 'artifact': 'same',
                               'requested_gemv_mode': 'baseline', 'gemv_mode': 'baseline'},
            'profile_expected': {'artifact': 'same', 'requested_gemv_mode': 'baseline', 'gemv_mode': 'baseline'},
            'closed_expected': {'artifact': 'same', 'requested_gemv_mode': 'baseline', 'gemv_mode': 'baseline'},
            'timeouts': {'setup_seconds': 600, 'request_seconds': 180, 'cell_seconds': 1200}}
    common = {**cell.COMMON, 'live_profile': m.PROFILE, 'prefill_chunk': 16,
              'splitk_down': m.expected_metadata(value)}
    profile = {**copy.deepcopy(common), 'attention': 'query-hoist-v14', 'runtime_profiling': False,
               'dispatch_sequences': False, 'runtime_cache_admission': True, 'runtime_operational': True,
               'queue_rollover': True, 'runtime_ordered_batches': True, 'projection': 'mfma',
               'argmax_mode': 'wave-v11', **value['profile_expected']}
    setup = {**copy.deepcopy(common), 'schema': 'FerricQwen3TpBatchSetupV2', 'authority': 'none',
             'attention_mode': 'query-hoist-v14', 'tensor_parallel': 1, 'model': 'Qwen/Qwen3-8B',
             'dtype': 'BF16', 'target': 'gfx950:xnack-', 'head_precision': 'fp32-v8',
             'argmax_mode': 'wave-v11', 'runtime_ordered_batches': True, 'prefix_cache': False,
             'context_tokens': 8192, 'physical_pages': 512, 'batch_tokens': 32,
             'performance_qualified': False, 'serving_qualified': False, 'output_head_pruning': True,
             'max_batches': count * mechanism['model_batches'], 'controller_sha256': controller['sha256'],
             'worker_sha256': worker['sha256'], 'running_worker_sha256': [worker['sha256']],
             'device_unique_ids': [123], 'worker_pids': [42], 'performance_profile': profile,
             **value['setup_expected']}
    closed = {**copy.deepcopy(common), 'schema': 'FerricQwen3TpBatchClosedV2', 'authority': 'none',
              'attention_mode': 'query-hoist-v14', 'performance_qualified': False, 'worker_pids': [42],
              'execution_completed': True, 'all_workers_exited': True,
              'rank_dispatch_counts': [count * mechanism['model_dispatches']], **value['closed_expected']}
    return value, setup, closed


def plan():
    profiles = {arm: cell.profile_for_spec(fixture(arm)[0]) for arm in ('A', 'B')}
    return {'schema': 'FerricSplitKModelChangePlanV1', 'change_id': 'ordinary-v19-splitk-down-r1',
        'gates': dict(cell.ledger.GATES), 'workload': {'input_tokens': 128, 'output_tokens': 128,
        'context_tokens': 8192, 'concurrency': 1, 'tensor_parallel': 1, 'greedy': True,
        'prefix_caching': False, 'speculation': False},
        'reference_sha256': cell.ledger.digest(fixture()[0]['reference']), 'workload_sha256': '1' * 64,
        'client_sha256': '2' * 64, 'profiles': profiles,
        'allowed_profile_differences': cell.ledger.differences(profiles['A'], profiles['B']),
        'mechanism': {arm: m.expected_mechanism(arm) for arm in ('A', 'B')},
        'timing_semantics': 'native-ingress-v1'}


def measured_cells(ttft_b=800, tpot_b=45):
    result = []
    for index, arm in enumerate(cell.ledger.GATES['order'] * 3):
        ttft, tpot = (800, 50) if arm == 'A' else (ttft_b, tpot_b)
        requests = [{'ttft_ms': ttft, 'tpot_ms': tpot, 'output_tokens': 128,
            'started_ns': index * 100_000_000_000 + j * 10_000_000_000,
            'completed_ns': index * 100_000_000_000 + j * 10_000_000_000 + 8_000_000_000}
            for j in range(4)]
        result.append({'arm': arm, 'cell_id': 'cell-' + str(index), 'requests': requests,
                       'median_tpot_ms': tpot, 'window_ns': 38_000_000_000})
    return result


class SelectionTests(unittest.TestCase):
    def test_exact_both_metadata_and_cli(self):
        for arm in m.ARMS:
            value = spec(arm)
            m.cli(value)
            m.metadata(m.expected_metadata(value), value)

    def test_only_selection_changes_in_same_asset_metadata(self):
        self.assertEqual(m.stable_metadata(m.expected_metadata(spec('A'))),
                         m.stable_metadata(m.expected_metadata(spec('B'))))
        self.assertEqual(m.common_argv(spec('A')), m.common_argv(spec('B')))

    def test_cli_missing_duplicate_wrong_mode(self):
        for replacement in ([], ['--splitk-down-mode', 'other'],
                            ['--splitk-down-mode', 'baseline', '--splitk-down-mode', 'baseline']):
            value = spec()
            value['argv'][1:3] = replacement
            with self.subTest(replacement=replacement), self.assertRaises(ValueError):
                m.cli(value)

    def test_cli_rejects_unrelated_program_width_or_instrumentation(self):
        for option in ('--native-prefill-rows', '--token-program-backend', '--token-program-fence-mode',
                       '--model-timestamps', '--ordered64-packet-ticks', '--ordered64-runtime-counters'):
            value = spec()
            value['argv'].append(option)
            with self.subTest(option=option), self.assertRaises(ValueError):
                m.cli(value)

    def test_metadata_storage_backend_phase_and_counts_cannot_drift(self):
        value = spec('B')
        for key, wrong in (('activation_scratch_bytes', 0), ('resident_transposed_down_bytes', 0),
                           ('additional_weight_bytes', 1), ('loaded_image_count', 9),
                           ('worker_backend', 'native-whole-program-v1'), ('selected_phase', 'prefill'),
                           ('prefill_unchanged', False), ('decode_packets', 652), ('model_dispatches_128_128', 87711)):
            changed = m.expected_metadata(value)
            changed[key] = wrong
            with self.subTest(key=key), self.assertRaises(ValueError):
                m.metadata(changed, value)

    def test_image_and_roster_drift(self):
        for key in m.IMAGE:
            value = spec()
            value['splitk']['image'][key] = 'f' * 64
            with self.subTest(key=key), self.assertRaises(ValueError):
                m.expected_metadata(value)
        value = spec()
        value['splitk']['roster']['value'].reverse()
        with self.assertRaises(ValueError):
            m.expected_metadata(value)

    def test_metadata_unknown_field_and_bool_integer_refuse(self):
        for key, wrong in (('extra', False), ('selected_rows', True)):
            value = m.expected_metadata(spec())
            value[key] = wrong
            with self.subTest(key=key), self.assertRaises(ValueError):
                m.metadata(value, spec())

    def test_both_complete_raw_requests_replay_without_mutation(self):
        for arm in m.ARMS:
            rows, reference = stream(arm)
            before = copy.deepcopy(rows)
            result, events = replay(arm, rows, reference)
            self.assertEqual(rows, before)
            self.assertEqual(events.batches, 135)
            self.assertEqual(events.dispatches, m.expected_mechanism(arm)['model_dispatches'])
            self.assertEqual(result['output_tokens'], 128)

    def test_late_decode_hybrid_refuses(self):
        for arm, wrong in (('A', 688), ('B', 652)):
            rows, reference = stream(arm)
            [row for row in rows if row['event'] == 'batch'][-1]['rank_dispatch_counts'] = [wrong]
            with self.subTest(arm=arm), self.assertRaises(ValueError):
                replay(arm, rows, reference)

    def test_prefill_packets_or_rows_cannot_change(self):
        for index, field, wrong in ((0, 'rank_dispatch_counts', [649]),
                                    (7, 'rank_dispatch_counts', [652]), (0, 'rows', 32)):
            rows, reference = stream('B')
            [row for row in rows if row['event'] == 'batch'][index][field] = wrong
            with self.subTest(index=index, field=field), self.assertRaises(ValueError):
                replay('B', rows, reference)

    def test_wrong_tick_or_batch_id_refuses(self):
        for field in ('tick', 'batch_id'):
            rows, reference = stream('B')
            [row for row in rows if row['event'] == 'batch'][-1][field] += 1
            with self.subTest(field=field), self.assertRaises(ValueError):
                replay('B', rows, reference)

    def test_nonmonotonic_emission_or_overlapping_batch_refuses(self):
        for field in ('emission_started_ns', 'started_ns'):
            rows, reference = stream('B')
            [row for row in rows if row['event'] == 'batch'][-1][field] = 1
            with self.subTest(field=field), self.assertRaises(ValueError):
                replay('B', rows, reference)

    def test_producing_token_timestamp_mismatch_refuses(self):
        rows, reference = stream('B')
        [row for row in rows if row['event'] == 'token'][-1]['completed_ns'] -= 1
        with self.assertRaises(ValueError):
            replay('B', rows, reference)

    def test_missing_or_duplicate_batch_refuses(self):
        for duplicate in (False, True):
            rows, reference = stream('B')
            index = next(index for index, row in enumerate(rows) if row['event'] == 'batch')
            if duplicate:
                rows.insert(index, copy.deepcopy(rows[index]))
            else:
                rows.pop(index)
            with self.subTest(duplicate=duplicate), self.assertRaises(ValueError):
                replay('B', rows, reference)

    def test_common_argv_does_not_hide_different_controller(self):
        a, b = spec('A'), spec('B')
        b['argv'][0] = '/private/other-controller'
        self.assertNotEqual(m.common_argv(a), m.common_argv(b))

    def test_failed_lint_provenance_is_separate_from_raw_setup(self):
        raw = json.dumps({'schema': 'actual-raw-setup'}).encode()
        saved = bytes(raw)
        value = m.setup_provenance(raw, {'path': '/evidence/retention.json', 'sha256': 'a' * 64})
        self.assertEqual(raw, saved)
        self.assertEqual(value['strict_clippy'], 'failed')
        self.assertEqual(value['binary_lint_coverage'], 'incomplete')
        self.assertFalse(value['production_qualified'])
        self.assertEqual(value['raw_setup_sha256'], hashlib.sha256(raw).hexdigest())


if __name__ == '__main__':
    unittest.main()
