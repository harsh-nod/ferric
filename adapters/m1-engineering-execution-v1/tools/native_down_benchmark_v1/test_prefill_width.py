"""Authored CPU-only down fixtures; none is a native timing sample."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parent


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


c = load('width_contract', ROOT / 'launch_contract.py')
m = load('width_cell', ROOT / 'measurement/native_token_cell.py')
r = load('width_replay', ROOT / 'measurement/native_campaign_replay.py')
FROZEN = ROOT / 'fixtures/frozen_run_v17_native.py'
FROZEN_SHA = '810a22cb4e805662a7575e2e4ec261fe161a43405f675a7b639f45a62664301f'
if hashlib.sha256(FROZEN.read_bytes()).hexdigest() != FROZEN_SHA:
    raise ValueError('frozen chronological/request validation fixture changed')
frozen = load('frozen_native_event_checks', FROZEN)


def down_fixture():
    roster = [{'logical_name': 'compiler::partial', 'export_name': m.DOWN_EXPORTS[0]},
              {'logical_name': 'compiler::merge', 'export_name': m.DOWN_EXPORTS[1]}]
    binding = {'path':'/private/down', 'ids':dict(c.DOWN_IDS),
               'roster':{'path':'/private/down/roster.json', 'sha256':m.ledger.digest(roster)}}
    return c.down_metadata(binding, roster)


def fixture(arm='A', mode='latency'):
    count = 6 if mode == 'latency' else 1
    backend = m.BACKENDS[arm]
    mechanism = m.expected_mechanism(arm)
    instrumented = mode == 'counters'
    controller = {'path': '/private/controller-' + ('counters' if instrumented else 'live') + '-' + arm,
                  'sha256': ('b' if instrumented else 'a') * 64}
    worker = {'path': '/private/worker', 'sha256': 'c' * 64}
    down = down_fixture()
    flags = {'--native-prefill-rows': str(backend['rows']), '--worker': worker['path'],
             '--worker-sha256': worker['sha256'], '--device-unique-id': '123',
             '--max-batches': str(count * mechanism['model_batches']), '--context': '8192', '--pages': '512',
             '--submission': 'ordered', '--wave-target-mode': 'combined', '--layer-projection': 'c1-wave',
             '--prefill-kv-mode': 'parallel-prefill16-v27', '--split-attention-mode': 'split8-v21',
             '--c1-packet-mode': 'packed64-v29', '--gemv-mode': 'baseline',
             '--ordered64-kv-copy-mode': 'parallel-c1-v19',
             '--native-down':backend['selection'], '--down-artifact':down['artifact_path'],
             '--down-roster':down['compiler_roster']['path'],
             '--down-roster-sha256':down['compiler_roster']['sha256'],
             '--down-hsaco-sha256':m.DOWN_IMAGE['artifact_hsaco_id'],
             '--down-manifest-sha256':m.DOWN_IMAGE['artifact_manifest_id'],
             '--down-handoff-sha256':m.DOWN_IMAGE['artifact_handoff_id']}
    argv = [controller['path']]
    for key, value in flags.items():
        argv.extend((key, value))
    argv.extend(('--live-stdin', '--runtime-cache-admission', '--runtime-operational', '--queue-rollover',
                 '--disable-prefix-cache', '--prune-output-head'))
    spec = {'schema': 'FerricNativeDownTokenCellPlanR1', 'arm': arm, 'mode': mode, 'argv': argv,
            'controller': controller, 'worker': worker, 'device_unique_id': 123, 'prompt': 'frozen prompt',
            'down_expected': down,
            'reference': {'prompt_token_ids': list(range(128)), 'generated_token_ids': list(range(128)),
                          'generated_utf8_hex': '6162'},
            'setup_expected': {'model_bundle_id': 'd' * 64, 'target_model_id': 'e' * 64, 'artifact': 'same',
                               'requested_gemv_mode': 'baseline', 'gemv_mode': 'baseline'},
            'profile_expected': {'artifact': 'same', 'requested_gemv_mode': 'baseline', 'gemv_mode': 'baseline'},
            'closed_expected': {'artifact': 'same', 'requested_gemv_mode': 'baseline', 'gemv_mode': 'baseline'},
            'timeouts': {'setup_seconds': 600, 'request_seconds': 180, 'cell_seconds': 1200}}
    for scope in ('setup_expected', 'profile_expected', 'closed_expected'):
        spec[scope]['split_attention_policy'] = {
            'physical_rows':1, 'actual_context_min':128, 'actual_context_max':256,
            'fallback':'query-hoist-v14', 'fallback_packets_no_head':613, 'fallback_packets_with_head':616,
            'partitions':8, 'split_packets_no_head':649, 'split_packets_with_head':652,
            'packet_counts_scope':'baseline FFN only; selected composition counts are in native_down'}
    profile_name = backend['counter_profile'] if instrumented else backend['profile']
    common = {**m.COMMON, 'live_profile': profile_name, 'prefill_chunk': backend['rows'],
              'token_program': m.token_metadata(arm, instrumented),
              'prefill_program': m.prefill_metadata(arm, instrumented),
              'native_down':dict(down, enabled=backend['enabled'], decode_dispatches=backend['decode_commands'])}
    profile = {**copy.deepcopy(common), 'attention': 'query-hoist-v14', 'runtime_profiling': instrumented,
               'dispatch_sequences': False, 'runtime_cache_admission': True, 'runtime_operational': True,
               'queue_rollover': True, 'runtime_ordered_batches': True, 'projection': 'mfma',
               'argmax_mode': 'wave-v11', **spec['profile_expected']}
    setup = {**copy.deepcopy(common), 'schema': 'FerricQwen3TpBatchSetupV2', 'authority': 'none',
             'attention_mode': 'query-hoist-v14', 'tensor_parallel': 1, 'model': 'Qwen/Qwen3-8B',
             'dtype': 'BF16', 'target': 'gfx950:xnack-', 'head_precision': 'fp32-v8',
             'argmax_mode': 'wave-v11', 'runtime_ordered_batches': True, 'prefix_cache': False,
             'context_tokens': 8192, 'physical_pages': 512, 'batch_tokens': 32,
             'performance_qualified': False, 'serving_qualified': False, 'output_head_pruning': True,
             'max_batches': count * mechanism['model_batches'], 'controller_sha256': controller['sha256'],
             'worker_sha256': worker['sha256'], 'running_worker_sha256': [worker['sha256']],
             'device_unique_ids': [123], 'worker_pids': [42], 'performance_profile': profile,
             **spec['setup_expected']}
    closed = {**copy.deepcopy(common), 'schema': 'FerricQwen3TpBatchClosedV2', 'authority': 'none',
              'attention_mode': 'query-hoist-v14', 'performance_qualified': False, 'worker_pids': [42],
              'execution_completed': True, 'all_workers_exited': True,
              'rank_dispatch_counts': [count * mechanism['model_dispatches']], **spec['closed_expected']}
    return spec, setup, closed


def counters(spec, setup):
    arm = spec['arm']
    backend, expected = m.BACKENDS[arm], m.expected_mechanism(arm)
    rows = []
    for ordinal in (0, 1):
        values = {'executions': expected['program_executions'], 'dispatches': expected['program_dispatches'],
                  'publications': expected['program_publications'], 'final_waits': expected['program_final_waits'],
                  'retirement_signals': expected['program_dispatches'], 'staging_ns': 5000,
                  'kernarg_initialized_bytes': 1000}
        if not ordinal:
            values = dict.fromkeys(values, 0)
        rows.append({'schema': 'FerricNativeDownProgramCountersR1', 'authority': 'none',
                     'performance_qualified': False, 'runtime_profiling': True, 'latency_sample_admitted': False,
                     'backend': backend['backend'], 'worker_entry': backend['worker_entry'],
                     'live_profile': backend['counter_profile'],
                     'process_id': setup['worker_pids'][0], 'device_unique_id': 123, 'ordinal': ordinal,
                     'phase': 'worker_start' if not ordinal else 'before_close',
                     'scope': 'successful prefill649 and explicitly selected decode programs; excludes head singletons, registration and readback',
                     'down_splitk8':{'enabled':backend['enabled'], 'scratch_bytes':131072,
                                       'prefill_unchanged':True, 'decode_dispatches':backend['decode_commands']},
                     'staging_scope': 'host staging wall time; not GPU time',
                     'kernarg_bytes_scope': 'initialized stores; baseline includes slot clear and exact copy',
                     'program_phases': {
                         'prefill': {'rows': backend['rows'], 'executions': expected['prefill_chunks'] if ordinal else 0,
                                     'dispatches_per_execution': backend['commands'], 'dynamic_slots': backend['slots'],
                                     'command_family': backend['family']},
                         'decode_c1': {'executions': 127 if ordinal else 0, 'dispatches_per_execution': backend['decode_commands'],
                                       'dynamic_slots': 180, 'command_family': 'legacy256-v1'},
                         'registrations': 2 if ordinal else 0, 'releases': 2 if ordinal else 0,
                         'transition_policy': 'idle-explicit-shape-release-register-v1'}, 'counters': values})
    return rows


def raw(rows):
    return b'\n'.join(m.ledger.canonical(row) for row in rows) + b'\n'


def plan():
    profiles = {arm: m.profile_for_spec(fixture(arm)[0]) for arm in ('A', 'B')}
    return {'schema': 'FerricNativeDownChangePlanR1', 'change_id': 'native-down-control-vs-splitk8',
            'gates': dict(m.ledger.GATES), 'workload': {'input_tokens': 128, 'output_tokens': 128,
            'context_tokens': 8192, 'concurrency': 1, 'tensor_parallel': 1,
            'greedy': True, 'prefix_caching': False, 'speculation': False},
            'reference_sha256': m.ledger.digest(fixture()[0]['reference']), 'workload_sha256': '1' * 64,
            'client_sha256': '2' * 64, 'profiles': profiles,
            'allowed_profile_differences': m.ledger.differences(profiles['A'], profiles['B']),
            'mechanism': {arm: m.expected_mechanism(arm) for arm in ('A', 'B')},
            'timing_semantics': 'native-ingress-v1'}


def measured_cells(ttft_b=800, tpot_b=45):
    result = []
    for index, arm in enumerate(m.ledger.GATES['order'] * 3):
        ttft, tpot = (800, 50) if arm == 'A' else (ttft_b, tpot_b)
        requests = [{'ttft_ms': ttft, 'tpot_ms': tpot, 'output_tokens': 128,
                     'started_ns': index * 100_000_000_000 + j * 10_000_000_000,
                     'completed_ns': index * 100_000_000_000 + j * 10_000_000_000 + 8_000_000_000}
                    for j in range(4)]
        result.append({'arm': arm, 'cell_id': 'cell-' + str(index), 'requests': requests,
                       'median_tpot_ms': tpot, 'window_ns': 38_000_000_000})
    return result


def request_stream(arm):
    reference = {'prompt_token_ids': list(range(128)), 'generated_token_ids': list(range(128)),
                 'generated_utf8_hex': '6162'}
    identity = {'schema': frozen.EVENT, 'authority': 'none', 'request_id': 1, 'name': 'sample'}
    rows = [{**identity, 'event': 'queued', 'emission_started_ns': 2, 'arrival_ns': 1,
             'queued_ns': 2, 'prompt_tokens': 128},
            {**identity, 'event': 'admission', 'emission_started_ns': 3, 'arrival_ns': 1,
             'admitted_ns': 3, 'queue_wait_ns': 2, 'prompt_tokens': reference['prompt_token_ids'],
             'prompt_token_count': 128, 'cached_tokens': 0, 'cached_pages': 0}]
    expected = m.expected_mechanism(arm)
    stamps = []
    for index in range(expected['model_batches']):
        prefill = index < expected['prefill_chunks']
        outputs = 0 if index < expected['prefill_chunks'] - 1 else 1
        start, end = 100 + index * 100, 150 + index * 100
        if outputs:
            token = len(stamps)
            stamps.append(end)
            rows.append({**identity, 'event': 'token', 'emission_started_ns': end,
                         'completed_ns': end, 'index': token, 'token': token, 'finished': token == 127})
        rows.append({**identity, 'event': 'batch', 'emission_started_ns': end + 1,
                     'tick': index, 'batch_id': index + 1, 'started_ns': start, 'completed_ns': end,
                     'rows': m.BACKENDS[arm]['rows'] if prefill else 1, 'outputs': outputs,
                     'output_head_rows': outputs,
                     'rank_dispatch_counts': [m.BACKENDS[arm]['commands'] + 3 * outputs if prefill else m.BACKENDS[arm]['decode_commands']]})
    rows.append({**identity, 'event': 'request', 'emission_started_ns': stamps[-1] + 2,
                 'admitted': True, 'state': 'Completed', 'cancelled_ns': None, 'cached_prefix_tokens': 0,
                 'arrival_ns': 1, 'prompt_tokens': reference['prompt_token_ids'], 'prompt_token_count': 128,
                 'generated_tokens': reference['generated_token_ids'], 'output_timestamps_ns': stamps,
                 'generated_utf8_bytes': [97, 98], 'generated_text': 'ab', 'ttft_ns': stamps[0] - 1,
                 'tpot_ns': (stamps[-1] - stamps[0]) // 127,
                 'decode_intervals_ns': [b - a for a, b in zip(stamps, stamps[1:])]})
    return rows, reference


class Stream:
    def __init__(self, rows):
        self.rows = list(rows)

    def next(self, _deadline):
        if not self.rows:
            raise ValueError('truncated request stream')
        return self.rows.pop(0)


def replay_request(arm, rows, reference):
    events = m.WidthEvents(frozen, arm)
    result = m.collect_request(Stream(rows), events, 1, 'sample', reference, 0, runner=frozen, arm=arm)
    return result, events


class WidthContractTests(unittest.TestCase):
    def test_both_widths_accept_exact_cli_setup_and_close(self):
        for arm in ('A', 'B'):
            for mode in ('counters', 'latency'):
                with self.subTest(arm=arm, mode=mode):
                    spec, setup, closed = fixture(arm, mode)
                    count, _ = m.shape(spec)
                    m.check_setup(setup, spec)
                    m.check_closed(closed, setup, count * m.expected_mechanism(arm)['model_dispatches'], spec)

    def test_width_missing_duplicate_noncanonical_wrong_arm_refuse(self):
        for replacement in ([], ['--native-prefill-rows', '032'], ['--native-prefill-rows', '16'],
                            ['--native-prefill-rows', '16', '--native-prefill-rows', '16']):
            spec = fixture()[0]
            spec['argv'][1:3] = replacement
            with self.subTest(replacement=replacement), self.assertRaises(ValueError):
                m.shape(spec)

    def test_wrong_width_budget_refuses(self):
        spec = fixture('B')[0]
        spec['argv'][spec['argv'].index('--max-batches') + 1] = '810'
        with self.assertRaisesRegex(ValueError, 'max-batches'):
            m.shape(spec)

    def test_other_backend_or_fence_cli_refuses(self):
        for extra in (['--token-program-backend', 'native-whole-program-v1'],
                      ['--token-program-fence-mode', 'boundary'],
                      ['--native-gate-up', 'control'], ['--gate-up-artifact', '/private/old']):
            spec = fixture()[0]
            spec['argv'].extend(extra)
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                m.shape(spec)

    def test_gate_up_metadata_hybrid_refuses(self):
        for scope in ('setup','profile','closed'):
            spec,setup,closed = fixture('B')
            target = setup if scope == 'setup' else setup['performance_profile'] if scope == 'profile' else closed
            target['native_gate_up'] = {'enabled':False}
            with self.subTest(scope=scope), self.assertRaises(ValueError):
                if scope == 'closed':
                    m.check_closed(closed,setup,6*89975,spec)
                else:
                    m.check_setup(setup,spec)

    def test_old_backend_or_worker_entry_metadata_refuses(self):
        for key, value in (('backend', 'native-whole-program-v1'),
                           ('worker_entry', '--diagnostic-token-program-native-v1'), ('backend_identity_checked', False)):
            spec, setup, _ = fixture()
            setup['token_program'][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                m.check_setup(setup, spec)

    def test_prefill_family_slots_or_width_drift_refuses(self):
        for key, value in (('rows', 16), ('dynamic_slots', 216), ('dispatches', 613),
                           ('command_family', 'legacy256-v1'), ('decode_command_family', 'slots512-v1')):
            spec, setup, _ = fixture('B')
            setup['prefill_program'][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                m.check_setup(setup, spec)

    def test_profile_or_closed_width_drift_refuses(self):
        spec, setup, closed = fixture('B')
        setup['performance_profile']['prefill_chunk'] = 16
        with self.assertRaises(ValueError):
            m.check_setup(setup, spec)
        closed['prefill_chunk'] = 16
        with self.assertRaises(ValueError):
            m.check_closed(closed, setup, 6 * 89975, spec)

    def test_decode_or_kernel_composition_drift_refuses(self):
        for key, value in (('c1_dispatches', 649), ('dynamic_slots', 396), ('prefill_unchanged', False)):
            spec, setup, _ = fixture('B')
            setup['token_program'][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                m.check_setup(setup, spec)
        spec, setup, _ = fixture('B')
        setup['gemv_mode'] = 'other'
        with self.assertRaises(ValueError):
            m.check_setup(setup, spec)

    def test_mechanisms_include_three_head_singletons_only_in_model_total(self):
        for arm, chunks, programs, program_dispatches, model_dispatches in (
                ('A', 4, 131, 85400, 85403), ('B', 4, 131, 89972, 89975)):
            expected = m.expected_mechanism(arm)
            self.assertEqual((expected['prefill_chunks'], expected['program_executions'],
                              expected['program_dispatches'], expected['model_dispatches']),
                             (chunks, programs, program_dispatches, model_dispatches))
            self.assertEqual(expected['decode_program_executions'], 127)
            self.assertEqual(expected['decode_commands_per_program'], 652 if arm == 'A' else 688)

    def test_counter_pairs_exact_for_both_widths(self):
        for arm in ('A', 'B'):
            spec, setup, _ = fixture(arm, 'counters')
            result = m.counter_replay(raw(counters(spec, setup)), spec, setup)
            self.assertFalse(result['latency_admitted'])
            self.assertTrue(result['mechanism_qualified'])
            self.assertEqual(result['delta']['executions'], 131)

    def test_counter_wrong_schema_backend_worker_or_width_refuses(self):
        for key, value in (('schema', 'FerricPrefillProgramCountersV1'), ('backend', 'native-whole-program-v1'),
                           ('worker_entry', '--diagnostic-token-program-native-v1')):
            spec, setup, _ = fixture('B', 'counters')
            rows = counters(spec, setup)
            rows[1][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                m.counter_replay(raw(rows), spec, setup)

    def test_counter_phase_family_width_or_late_transition_refuses(self):
        for phase, key, value in (('prefill', 'rows', 16), ('prefill', 'dynamic_slots', 216),
                                  ('prefill', 'command_family', 'legacy256-v1'),
                                  ('decode_c1', 'command_family', 'slots512-v1'), ('decode_c1', 'executions', 126)):
            spec, setup, _ = fixture('B', 'counters')
            rows = counters(spec, setup)
            rows[1]['program_phases'][phase][key] = value
            with self.subTest(phase=phase, key=key), self.assertRaises(ValueError):
                m.counter_replay(raw(rows), spec, setup)
        rows = counters(spec, setup)
        rows[1]['program_phases']['releases'] = 1
        with self.assertRaises(ValueError):
            m.counter_replay(raw(rows), spec, setup)

    def test_counter_head_inclusion_or_old_counts_refuses(self):
        for key, value in (('executions', 135), ('dispatches', 89975), ('publications', 135),
                           ('final_waits', 135), ('retirement_signals', 89975),
                           ('dispatches', 85400), ('dispatches', 85403), ('retirement_signals', 85400)):
            spec, setup, _ = fixture('B', 'counters')
            rows = counters(spec, setup)
            rows[1]['counters'][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                m.counter_replay(raw(rows), spec, setup)

    def test_nonzero_initial_counter_or_extra_stderr_refuses(self):
        spec, setup, _ = fixture('A', 'counters')
        rows = counters(spec, setup)
        rows[0]['counters']['staging_ns'] = 1
        with self.assertRaises(ValueError):
            m.counter_replay(raw(rows), spec, setup)
        with self.assertRaises(ValueError):
            m.counter_replay(raw(counters(spec, setup)) + b'{}\n', spec, setup)

    def test_latency_never_accepts_counter_replay(self):
        spec, setup, _ = fixture('A', 'latency')
        with self.assertRaises(ValueError):
            m.counter_replay(raw(counters(spec, setup)), spec, setup)

    def test_common_argv_normalizes_only_validated_down_arm(self):
        a, b = fixture('A')[0], fixture('B')[0]
        self.assertEqual(m.common_argv(a), m.common_argv(b))
        b['argv'][b['argv'].index('--context') + 1] = '4096'
        with self.assertRaises(ValueError):
            m.common_argv(b)

    def test_counter_pair_rejects_different_binaries(self):
        entries = []
        for arm in ('A', 'B'):
            spec, setup, _ = fixture(arm, 'counters')
            entries.append({'cell_id': 'counter-' + arm, 'spec': spec,
                'result': {'accepted': True, 'latency_admitted': False, 'raw_replay_passed': True,
                           'setup': setup, 'mechanism': m.counter_replay(raw(counters(spec, setup)), spec, setup)}})
        self.assertTrue(r.validate_counter_pair(entries)['accepted'])
        entries[1]['spec']['controller']['sha256'] = 'f' * 64
        with self.assertRaisesRegex(ValueError, 'one exact'):
            r.validate_counter_pair(entries)

    def test_complete_fourteen_cell_order_is_unchanged(self):
        self.assertEqual(c.CELL_ORDER[:2], (('counter-A', 'A', 'counters'), ('counter-B', 'B', 'counters')))
        self.assertEqual([row[1] for row in c.CELL_ORDER[2:]], ['A', 'B', 'B', 'A'] * 3)
        self.assertEqual(len(c.CELL_ORDER), 14)

    def test_plan_rejects_old_mechanism_or_http_semantics(self):
        value = plan()
        m.ledger.validate_plan(value)
        value['mechanism']['B']['model_dispatches'] = 87711
        with self.assertRaises(ValueError):
            m.ledger.validate_plan(value)
        value = plan()
        value['timing_semantics'] = 'http-text-chunk-v1'
        with self.assertRaises(ValueError):
            m.ledger.validate_plan(value)

    def test_ttft_gain_does_not_fabricate_tpot_gain(self):
        report = m.ledger.compare_results(plan(), '3' * 64, measured_cells(ttft_b=720, tpot_b=50))
        self.assertEqual(report['status'], 'inconclusive')
        self.assertAlmostEqual(report['median_ttft_gain_percent'], 10)
        self.assertEqual(report['median_tpot_gain_percent'], 0)
        self.assertEqual(report['arms']['A']['requests'], 24)
        self.assertIn('not HTTP', report['latency_semantics'])

    def test_tpot_regression_blocks_ttft_only_gain(self):
        report = m.ledger.compare_results(plan(), '3' * 64, measured_cells(ttft_b=720, tpot_b=53))
        self.assertFalse(report['checks']['median_tpot_gain'])
        self.assertEqual(report['status'], 'inconclusive')

    def test_faster_decode_with_unchanged_prefill_passes_experimental_gate(self):
        report = m.ledger.compare_results(plan(), '3' * 64, measured_cells())
        self.assertTrue(report['checks']['median_tpot_gain'])
        self.assertEqual(report['status'], 'experimental-gates-passed')
        self.assertFalse(report['default_promotion'])

    def test_ttft_regression_blocks_faster_decode(self):
        report = m.ledger.compare_results(plan(), '3' * 64, measured_cells(ttft_b=850))
        self.assertTrue(report['checks']['median_tpot_gain'])
        self.assertFalse(report['checks']['regression_limits'])
        self.assertEqual(report['status'], 'inconclusive')

    def test_warmups_excluded_without_changing_width_dispatch_totals(self):
        records = [{'arrival_ns': 1 + index * 10_000_000_000, 'completed_ns': 8_000_000_001 + index * 10_000_000_000,
                    'ttft_ns': (7000 if index < 2 else 700) * 1_000_000, 'tpot_ns': 50_000_000,
                    'output_tokens': 128, 'batches': 131, 'dispatches': 89975} for index in range(6)]
        summary = m.summarize(records, 2, 'B')
        self.assertEqual(len(summary['requests']), 4)
        self.assertEqual([row['ttft_ms'] for row in summary['requests']], [700] * 4)
        self.assertEqual(summary['warmups_excluded'], 2)
        with self.assertRaises(ValueError):
            m.summarize(records, 2, 'A')

    def test_synthetic_smaller_sample_or_reordered_abba_refuses(self):
        values = measured_cells()
        values[0]['requests'].pop()
        with self.assertRaises(ValueError):
            m.ledger.compare_results(plan(), '3' * 64, values)
        values = measured_cells()
        values[0]['arm'] = 'B'
        with self.assertRaises(ValueError):
            m.ledger.compare_results(plan(), '3' * 64, values)

    def test_current_release_still_requires_complete_build_and_cpu_evidence(self):
        self.assertTrue(c.WIDTH_QUALIFIED)
        with self.assertRaises(ValueError):
            c.validate_build({})
        with self.assertRaises(ValueError):
            c.validate_cpu({}, {})

    def test_both_controller_roles_and_actual_images_retain_stdout(self):
        current = c.cpu_contract().current
        for row in current.catalog()['phases'].values():
            self.assertEqual(set(row['files']), current.FIELDS)
            self.assertIn('role_stdout', row['files'])
            self.assertIn('role_stderr', row['files'])

    def test_native_width_stage_is_explicitly_distinct(self):
        self.assertEqual(c.stage_name(Path('/dev/shm/ferric-native-down-a001')).name,
                         'ferric-native-down-a001')
        with self.assertRaises(ValueError):
            c.stage_name(Path('/dev/shm/ferric-v17-prefill-dcf3454-a004'))
        with self.assertRaises(ValueError):
            c.stage_name(Path('/dev/shm/ferric-native-prefill-width-a001'))

    def test_actual_frozen_event_validator_accepts_both_full_width_streams(self):
        for arm in ('A', 'B'):
            rows, reference = request_stream(arm)
            original = copy.deepcopy(rows)
            result, events = replay_request(arm, rows, reference)
            self.assertEqual(result['output_tokens'], 128)
            self.assertEqual(events.batches, 131)
            self.assertEqual(events.dispatches, 85403 if arm == 'A' else 89975)
            self.assertEqual(rows, original, 'projection must not mutate retained raw events')

    def test_native32_requires_width_aware_request_collector(self):
        rows, reference = request_stream('A')
        current, _ = replay_request('A', rows, reference)
        self.assertEqual(current['output_tokens'], 128)
        with self.assertRaises(ValueError):
            frozen.collect_request(Stream(rows), m.WidthEvents(frozen, 'A'), 1, 'sample', reference, 0)

    def test_b32_each_prefill_geometry_and_dispatch_count_is_checked(self):
        for ordinal in range(4):
            for field, value in (('rows', 16), ('rank_dispatch_counts', [613])):
                rows, reference = request_stream('B')
                event = [row for row in rows if row['event'] == 'batch'][ordinal]
                event[field] = value
                with self.subTest(ordinal=ordinal, field=field), self.assertRaises(ValueError):
                    replay_request('B', rows, reference)

    def test_wrong_terminal_prefill_position_or_head_count_refuses(self):
        for ordinal, outputs in ((2, 1), (3, 0)):
            rows, reference = request_stream('B')
            event = [row for row in rows if row['event'] == 'batch'][ordinal]
            event.update(outputs=outputs, output_head_rows=outputs)
            with self.subTest(ordinal=ordinal), self.assertRaises(ValueError):
                replay_request('B', rows, reference)
        rows, reference = request_stream('B')
        [row for row in rows if row['event'] == 'batch'][3]['rank_dispatch_counts'] = [649]
        with self.assertRaises(ValueError):
            replay_request('B', rows, reference)

    def test_missing_extra_and_duplicated_prefill_events_refuse(self):
        for mutation in ('missing', 'extra', 'duplicate'):
            rows, reference = request_stream('B')
            index = next(i for i, row in enumerate(rows) if row['event'] == 'batch')
            if mutation == 'missing':
                rows.pop(index)
            elif mutation == 'extra':
                rows.insert(index + 1, {**rows[index], 'tick': 1, 'batch_id': 2})
            else:
                rows.insert(index + 1, copy.deepcopy(rows[index]))
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                replay_request('B', rows, reference)

    def test_decode_row_or_packet_count_cannot_be_projected_away(self):
        for field, value in (('rows', 32), ('rank_dispatch_counts', [649]), ('rank_dispatch_counts', [616])):
            rows, reference = request_stream('B')
            [row for row in rows if row['event'] == 'batch'][4][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                replay_request('B', rows, reference)

    def test_wrong_final_token_or_latency_report_refuses(self):
        for kind, field, value in (('token', 'token', 999), ('request', 'tpot_ns', 999)):
            rows, reference = request_stream('B')
            [row for row in rows if row['event'] == kind][-1][field] = value
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                replay_request('B', rows, reference)

    def test_b32_emission_and_packet_sequence_remain_frozen(self):
        for field, value in (('emission_started_ns', 0), ('tick', 999), ('batch_id', 999)):
            rows, reference = request_stream('B')
            [row for row in rows if row['event'] == 'batch'][1][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                replay_request('B', rows, reference)

    def test_b32_batch_overlap_and_producing_token_time_refuse(self):
        rows, reference = request_stream('B')
        batches = [row for row in rows if row['event'] == 'batch']
        batches[1]['started_ns'] = batches[0]['completed_ns'] - 1
        with self.assertRaisesRegex(ValueError, 'batch timing'):
            replay_request('B', rows, reference)
        rows, reference = request_stream('B')
        [row for row in rows if row['event'] == 'token'][0]['completed_ns'] -= 1
        with self.assertRaisesRegex(ValueError, 'producing batch'):
            replay_request('B', rows, reference)

    def test_counter_wire_domain_is_exact_u64(self):
        spec, setup, _ = fixture('B', 'counters')
        for field in ('staging_ns', 'kernarg_initialized_bytes'):
            values = counters(spec, setup)
            values[1]['counters'][field] = (1 << 64) - 1
            self.assertTrue(m.counter_replay(raw(values), spec, setup)['mechanism_qualified'])
            for invalid in (1 << 64, -1, True):
                values[1]['counters'][field] = invalid
                with self.subTest(field=field, invalid=invalid), self.assertRaisesRegex(ValueError, 'u64'):
                    m.counter_replay(raw(values), spec, setup)

    def test_all_seven_down_selectors_are_required_and_unique(self):
        for key in ('--native-down', '--down-artifact', '--down-roster',
                    '--down-roster-sha256', '--down-hsaco-sha256',
                    '--down-manifest-sha256', '--down-handoff-sha256'):
            for mutation in ('missing', 'duplicate', 'changed'):
                spec = fixture('B')[0]
                index = spec['argv'].index(key)
                if mutation == 'missing':
                    del spec['argv'][index:index+2]
                elif mutation == 'duplicate':
                    spec['argv'].extend(spec['argv'][index:index+2])
                else:
                    spec['argv'][index+1] = 'wrong'
                with self.subTest(key=key, mutation=mutation), self.assertRaises(ValueError):
                    m.shape(spec)

    def test_selected_metadata_is_exact_in_setup_profile_and_closed(self):
        for scope in ('setup', 'profile', 'closed'):
            for key, value in (('enabled', False), ('decode_dispatches', 652),
                               ('scratch_bytes', 131071), ('loaded_image_count', 9),
                               ('additional_weight_bytes', 1), ('same_image_set_in_both_arms', False)):
                spec, setup, closed = fixture('B')
                target = setup if scope == 'setup' else setup['performance_profile'] if scope == 'profile' else closed
                target['native_down'][key] = value
                with self.subTest(scope=scope, key=key), self.assertRaises(ValueError):
                    if scope == 'closed':
                        m.check_closed(closed, setup, 6 * 89975, spec)
                    else:
                        m.check_setup(setup, spec)

    def test_candidate_cannot_reuse_control_decode_counts(self):
        rows, reference = request_stream('B')
        [row for row in rows if row['event'] == 'batch'][4]['rank_dispatch_counts'] = [652]
        with self.assertRaisesRegex(ValueError, 'dispatch count'):
            replay_request('B', rows, reference)
        spec, setup, _ = fixture('B', 'counters')
        values = counters(spec, setup)
        values[1]['program_phases']['decode_c1']['dispatches_per_execution'] = 652
        with self.assertRaises(ValueError):
            m.counter_replay(raw(values), spec, setup)

    def test_counter_selected_scratch_and_arm_metadata_are_required(self):
        for key, value in (('enabled', False), ('scratch_bytes', 131071),
                           ('prefill_unchanged', False), ('decode_dispatches', 652)):
            spec, setup, _ = fixture('B', 'counters')
            values = counters(spec, setup)
            values[1]['down_splitk8'][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                m.counter_replay(raw(values), spec, setup)

    def test_roster_and_image_mutations_refuse(self):
        for mutation in ('export', 'logical', 'extra', 'hsaco', 'missing'):
            spec = fixture('B')[0]
            gate = spec['down_expected']
            if mutation == 'export':
                gate['compiler_roster']['value'].reverse()
            elif mutation == 'logical':
                gate['compiler_roster']['value'][0]['logical_name'] = 'bad\nname'
            elif mutation == 'extra':
                gate['compiler_roster']['value'][0]['extra'] = True
            elif mutation == 'hsaco':
                gate['artifact']['artifact_hsaco_id'] = '0' * 64
            else:
                gate.pop('artifact')
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                m.shape(spec)

    def test_different_positive_staging_byte_counts_are_not_a_regression(self):
        entries = []
        for arm in ('A', 'B'):
            spec, setup, _ = fixture(arm, 'counters')
            values = counters(spec, setup)
            values[1]['counters']['kernarg_initialized_bytes'] = 1000 if arm == 'A' else 2000
            entries.append({'cell_id':'counter-' + arm, 'spec':spec,
                'result':{'accepted':True, 'latency_admitted':False, 'raw_replay_passed':True,
                    'setup':setup, 'mechanism':m.counter_replay(raw(values), spec, setup)}})
        self.assertTrue(r.validate_counter_pair(entries)['accepted'])

    def test_policy_scope_is_required_in_each_runtime_record(self):
        for scope in ('setup', 'profile', 'closed'):
            for replacement in (None, 'selected composition'):
                spec, setup, closed = fixture('B')
                target = setup if scope == 'setup' else setup['performance_profile'] if scope == 'profile' else closed
                target['split_attention_policy'] = copy.deepcopy(target['split_attention_policy'])
                if replacement is None:
                    target['split_attention_policy'].pop('packet_counts_scope')
                else:
                    target['split_attention_policy']['packet_counts_scope'] = replacement
                with self.subTest(scope=scope, replacement=replacement), self.assertRaises(ValueError):
                    if scope == 'closed':
                        m.check_closed(closed, setup, 6 * 89975, spec)
                    else:
                        m.check_setup(setup, spec)

    def test_nested_selected_metadata_and_extra_fields_refuse(self):
        for scope in ('setup', 'profile', 'closed'):
            for mutation in ('image', 'roster', 'extra'):
                spec, setup, closed = fixture('B')
                target = setup if scope == 'setup' else setup['performance_profile'] if scope == 'profile' else closed
                gate = target['native_down']
                if mutation == 'image':
                    gate['artifact']['artifact_hsaco_id'] = '0' * 64
                elif mutation == 'roster':
                    gate['compiler_roster']['value'][0]['logical_name'] = 'other'
                else:
                    gate['unrecognized'] = True
                with self.subTest(scope=scope, mutation=mutation), self.assertRaises(ValueError):
                    if scope == 'closed':
                        m.check_closed(closed, setup, 6 * 89975, spec)
                    else:
                        m.check_setup(setup, spec)

    def test_candidate_staging_preserves_compiler_directory_contract(self):
        root = Path(c.DOWN_SUFFIX)
        self.assertEqual(root.parent.name, 'fe2o3-engineering-v1')
        self.assertEqual(root.name, 'efe280b299a95408cd5326305190c1ae7471d7fd0da62226aa2fc528153556b7')
        self.assertFalse(Path(c.DOWN_ROSTER).is_relative_to(root))

    def test_candidate_file_custody_refuses_replacement_or_missing_membership(self):
        for mutation in ('none', 'roster-bytes', 'missing-member', 'handoff', 'symlink',
                         'extra-image-file', 'wrong-content-name', 'wrong-namespace', 'roster-inside-image'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                stage = Path(temporary).resolve()
                observation = c.encoded({'compiler_handoff':{'sha256':('0' if mutation == 'handoff' else '3') * 64}})
                roster = c.encoded(down_fixture()['compiler_roster']['value'])
                content = {'observation.hsaco':b'fixture-not-an-actual-HSACO',
                           'observation.json':observation, 'roster.json':roster}
                content_id = hashlib.sha256(b'FE2O3/ENGINEERING-HSACO-OBSERVATION-CONTENT/V1\0')
                for name in ('observation.json', 'observation.hsaco'):
                    content_id.update(len(content[name]).to_bytes(8, 'little'))
                    content_id.update(content[name])
                suffix = (Path('native-inputs/down-splitk8-r1')
                          / ('wrong-namespace' if mutation == 'wrong-namespace' else 'fe2o3-engineering-v1')
                          / ('0' * 64 if mutation == 'wrong-content-name' else content_id.hexdigest()))
                root = stage / suffix
                root.mkdir(parents=True)
                roster_path = stage / c.DOWN_ROSTER
                files = {}
                for name, raw_bytes in content.items():
                    path = roster_path if name == 'roster.json' else root / name
                    path.write_bytes(raw_bytes)
                    files[str(path.relative_to(stage))] = {'sha256':m.ledger.sha(raw_bytes)}
                ids = {'artifact_hsaco_id':m.ledger.sha(content['observation.hsaco']),
                       'artifact_manifest_id':m.ledger.sha(observation), 'artifact_handoff_id':'3' * 64}
                gate = {'path':str(root), 'ids':ids,
                        'roster':{'path':str(roster_path), 'sha256':m.ledger.sha(roster)}}
                if mutation == 'roster-bytes':
                    roster_path.write_bytes(roster + b' ')
                elif mutation == 'missing-member':
                    files.pop(str(roster_path.relative_to(stage)))
                elif mutation == 'symlink':
                    roster_path.rename(stage / 'replacement.json')
                    roster_path.symlink_to(stage / 'replacement.json')
                elif mutation == 'extra-image-file':
                    (root / 'unexpected').write_bytes(b'not an artifact member')
                elif mutation == 'roster-inside-image':
                    (root / 'roster.json').write_bytes(roster)
                    gate['roster']['path'] = str(root / 'roster.json')
                with mock.patch.object(c, 'DOWN_IDS', ids), mock.patch.object(c, 'DOWN_SUFFIX', str(suffix)):
                    if mutation == 'none':
                        self.assertEqual(c.validate_down(stage, gate, files)['compiler_roster']['value'],
                                         down_fixture()['compiler_roster']['value'])
                    else:
                        with self.assertRaises(ValueError):
                            c.validate_down(stage, gate, files)


if __name__ == '__main__':
    unittest.main()
