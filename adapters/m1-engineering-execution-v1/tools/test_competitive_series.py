import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import competitive_benchmark as client
import competitive_series as series


def save(root, name, value):
    raw = json.dumps(value, sort_keys=True).encode()
    (root / name).write_bytes(raw)
    return {'path': name, 'sha256': hashlib.sha256(raw).hexdigest()}


def sample(index, origin, duration, failure=False):
    sent = origin + 10
    completed = origin + 50
    chunks = [
        {'received_ns': origin + 20, 'event': {'choices': [{'index': 0, 'text': 'a', 'finish_reason': None}]}},
        {'received_ns': origin + 30, 'event': {'choices': [{'index': 0, 'text': 'b', 'finish_reason': 'length'}],
                                             'usage': {'prompt_tokens': 1, 'completion_tokens': 2}}},
    ]
    replay = client.summarize_stream([(json.dumps(c['event']).encode(), c['received_ns']) for c in chunks]
                                      + [(b'[DONE]', completed)], sent, 2)
    record = {'id': 'r', 'success': True, **replay, 'intended_arrival_ns': origin,
              'send_started_ns': sent, 'send_delay_ns': 10, 'wire_ttft_ns': replay['ttft_ns'],
              'wire_e2e_ns': replay['e2e_ns'], 'ttft_ns': 20, 'e2e_ns': 50}
    if failure:
        record = client.arrival_failure({'id': 'r'}, origin, origin + 10, 'client_overload')
    metrics = client.aggregate([record], origin, origin + duration, 100, 100)
    metrics['failure_counts'] = {kind: int(failure and kind == 'client_overload') for kind in client.FAILURE_KINDS}
    return {'index': index, 'started_ns': origin, 'completed_ns': origin + duration,
            'metrics': metrics, 'requests': [record]}


def fixture(root, starts=3, windows=10, warmups=10):
    scope = {key: 'a' * 64 if key.endswith('_sha256') else 'fixed' for key in series.SCOPE}
    workload = {'schema': client.SCHEMA, 'model': 'Qwen/Qwen3-8B',
                'requests': [{'id': 'r', 'prompt': 'x', 'max_tokens': 2}]}
    workload_binding = save(root, 'workload.json', workload)
    tuning = {'schema': 'FerricCompetitiveTuningPolicyV1', 'calibration_workload_sha256': 'b' * 64,
              'heldout_workload_sha256': workload_binding['sha256'],
              'trials_per_engine': {'ferric': 4, 'vllm': 4}, 'selection_rule': 'highest calibration goodput'}
    tuning_binding = save(root, 'tuning.json', tuning)
    identity = {engine: {'engine': engine, 'comparison_scope': scope, 'artifact': engine}
                for engine in ('ferric', 'vllm')}
    identities = {engine: save(root, engine + '-identity.json', value) for engine, value in identity.items()}
    observations = {'schema': 'FerricCompetitiveExternalObservationsV1', 'steady_state_windows': False,
                    'clock_drift_percent': 0, 'thermal_drift_percent': 0, 'environment_unchanged': True,
                    'link_errors': 0, 'ecc_errors': 0, 'faults': [], 'admission_gates_passed': True,
                    'evidence_sha256': 'd' * 64}
    observation_binding = save(root, 'observations.json', observations)
    settings = {'concurrency': 1, 'pending_capacity': 0, 'arrival_policy': 'constant', 'arrival_rate': 1.0,
                'arrival_seed': 0, 'timeout_seconds': 1.0, 'ttft_slo_ms': 100, 'tpot_slo_ms': 100}
    order = [['ferric', 'vllm'] if index % 2 == 0 else ['vllm', 'ferric'] for index in range(starts)]
    frozen = {'schema': 'FerricCompetitiveSeriesPlanV1', 'cell': 'test', 'comparison_scope': scope,
              'workload_sha256': workload_binding['sha256'],
              'client_sha256': hashlib.sha256(Path(client.__file__).read_bytes()).hexdigest(),
              'tuning_policy_sha256': tuning_binding['sha256'], 'settings': settings,
              'engines': {engine: binding['sha256'] for engine, binding in identities.items()},
              'baseline': 'vllm', 'starts': starts, 'windows': windows, 'warmups': warmups,
              'engine_order': order, 'bootstrap_seed': 7, 'bootstrap_samples': 1000}
    plan_binding = save(root, 'plan.json', frozen)
    entries = []
    for index, engines in enumerate(order):
        for engine in engines:
            ordinal = len(entries) + 1
            wall = ordinal * 10**12
            start = {'schema': 'FerricCompetitiveServerStartEvidenceV1', 'engine': engine,
                     'identity_sha256': identities[engine]['sha256'], 'fresh_start': True,
                     'boot_id': '12345678-1234-1234-1234-123456789abc', 'pid': ordinal + 100,
                     'process_start_ticks': ordinal, 'ready_unix_ns': wall - 1,
                     'launch_evidence_sha256': 'e' * 64}
            start_binding = save(root, f'{engine}-{index}-start.json', start)
            duration = 500000000 if engine == 'ferric' else 1000000000
            report = {'schema': 'FerricCompetitiveStreamingRunV2', 'authority': 'none', 'qualification': False,
                      'engine': engine, 'identity': identity[engine], 'identity_sha256': identities[engine]['sha256'],
                      'workload': workload, 'workload_sha256': workload_binding['sha256'],
                      'client_sha256': frozen['client_sha256'], 'start_evidence': start,
                      'start_evidence_sha256': start_binding['sha256'], 'tuning_policy': tuning,
                      'tuning_policy_sha256': tuning_binding['sha256'], **settings, 'completed': True,
                      'ttft_semantics': 'intended-arrival-to-first-nonempty-text-chunk',
                      'tpot_semantics': 'first-to-last-text-chunk-divided-by-usage-tokens-minus-one',
                      'e2e_semantics': 'intended-arrival-to-DONE', 'token_itl_available': False,
                      'deadline_semantics': 'soft-absolute-checks-with-per-read-socket-timeout',
                      'response_budget_exhausted': False,
                      'window_semantics': 'finite-arrival-cohort-including-drain-not-steady-state',
                      'arrival_offsets_ns': [0], 'started_unix_ns': wall,
                      'completed_unix_ns': wall + (warmups + windows) * (duration + 1)}
            origin = 1000
            for phase, count in (('warmups', warmups), ('samples', windows)):
                report[phase] = []
                for n in range(count):
                    report[phase].append(sample(n, origin, duration))
                    origin += duration + 1
            entries.append({'engine': engine, 'start_index': index,
                            'run': save(root, f'{engine}-{index}.json', report),
                            'identity': identities[engine], 'workload': workload_binding,
                            'tuning_policy': tuning_binding, 'start_evidence': start_binding,
                            'observations': observation_binding})
    return frozen, {'schema': 'FerricCompetitiveSeriesRunsV1', 'plan_sha256': plan_binding['sha256'],
                    'runs': entries}, plan_binding['sha256']


class SeriesTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.plan, self.manifest, self.plan_sha = fixture(self.root)

    def tearDown(self):
        self.directory.cleanup()

    def analyze(self):
        return series.analyze(self.plan, self.manifest, self.root, self.plan_sha)

    def mutate_run(self, mutate, index=0):
        entry = self.manifest['runs'][index]
        value = json.loads((self.root / entry['run']['path']).read_bytes())
        mutate(value)
        entry['run'] = save(self.root, entry['run']['path'], value)

    def test_paired_starts_and_windows_pass_sampling_not_qualification(self):
        result = self.analyze()
        self.assertTrue(result['sampling_preconditions_satisfied'])
        self.assertFalse(result['qualification_preconditions_satisfied'])
        self.assertFalse(result['qualification'])
        self.assertFalse(result['framework_win_claim'])
        self.assertEqual(result['paired']['ratio_ci95'], [2, 2])
        self.assertEqual(result['paired']['median_ratio'], 2)
        self.assertTrue(result['comparison_valid'])
        self.assertEqual(result['aggregator_sha256'], hashlib.sha256(Path(series.__file__).read_bytes()).hexdigest())

    def test_local_replay_implementation_must_match_frozen_client(self):
        self.plan['client_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'local replay client'):
            self.analyze()

    def test_response_budget_failure_invalidates_ratios_instead_of_ranking_engine(self):
        self.mutate_run(lambda run: run.update(response_budget_exhausted=True))
        result = self.analyze()
        self.assertFalse(result['comparison_valid'])
        self.assertIsNone(result['paired'])
        self.assertTrue(any('client response-evidence budget' in reason
                            for reason in result['comparison_invalid_reasons']))

    def test_client_budget_record_alone_also_invalidates_comparison(self):
        def fail(run):
            window = run['samples'][0]
            item = run['workload']['requests'][0]
            record = client.arrival_failure(item, window['started_ns'], window['started_ns'] + 1, 'client_budget')
            window['requests'] = [record]
            window['metrics'] = client.aggregate([record], window['started_ns'], window['completed_ns'], 100, 100)
            window['metrics']['failure_counts'] = {kind: int(kind == 'client_budget') for kind in client.FAILURE_KINDS}
        self.mutate_run(fail)
        result = self.analyze()
        self.assertFalse(result['comparison_valid'])
        self.assertIsNone(result['paired'])

    def test_external_fault_or_drift_invalidates_comparison(self):
        entry = self.manifest['runs'][0]
        value = json.loads((self.root / entry['observations']['path']).read_bytes())
        value['clock_drift_percent'] = 4
        entry['observations'] = save(self.root, 'drifted-observations.json', value)
        result = self.analyze()
        self.assertFalse(result['comparison_valid'])
        self.assertIsNone(result['paired'])

    def test_missing_start_and_missing_window_fail_closed(self):
        self.manifest['runs'].pop()
        with self.assertRaisesRegex(ValueError, 'missing paired starts'):
            self.analyze()
        self.setUp_fresh_fixture()
        self.mutate_run(lambda run: run['samples'].pop())
        with self.assertRaisesRegex(ValueError, 'missing samples'):
            self.analyze()

    def setUp_fresh_fixture(self):
        self.plan, self.manifest, self.plan_sha = fixture(self.root)

    def test_identity_hash_drift_rejected_before_metrics(self):
        entry = self.manifest['runs'][0]
        (self.root / entry['identity']['path']).write_text('{}')
        with self.assertRaisesRegex(ValueError, 'hash drifted'):
            self.analyze()

    def test_reused_start_identity_rejected_even_with_new_hash(self):
        entry = self.manifest['runs'][4]
        first = json.loads((self.root / self.manifest['runs'][0]['start_evidence']['path']).read_bytes())
        value = json.loads((self.root / entry['start_evidence']['path']).read_bytes())
        for key in ('boot_id', 'pid', 'process_start_ticks'):
            value[key] = first[key]
        entry['start_evidence'] = save(self.root, entry['start_evidence']['path'], value)
        self.mutate_run(lambda run: run.update(start_evidence=value,
                         start_evidence_sha256=entry['start_evidence']['sha256']), index=4)
        with self.assertRaisesRegex(ValueError, 'reused server start'):
            self.analyze()

    def test_late_success_and_metric_tampering_rejected(self):
        self.mutate_run(lambda run: run['samples'][0]['requests'][0].update(completed_ns=2**63))
        with self.assertRaises(ValueError):
            self.analyze()
        self.setUp_fresh_fixture()
        self.mutate_run(lambda run: run['samples'][0]['metrics'].update(output_goodput_per_second=99))
        with self.assertRaisesRegex(ValueError, 'metrics drifted'):
            self.analyze()

    def test_arrival_scope_and_raw_text_tampering_rejected(self):
        self.mutate_run(lambda run: run.update(arrival_offsets_ns=[1]))
        with self.assertRaisesRegex(ValueError, 'arrival schedule'):
            self.analyze()
        self.setUp_fresh_fixture()
        self.mutate_run(lambda run: run['samples'][0]['requests'][0].update(text='altered'))
        with self.assertRaisesRegex(ValueError, 'raw SSE'):
            self.analyze()

    def test_failure_window_contributes_zero_and_is_not_dropped(self):
        def fail(run):
            old = run['samples'][0]
            run['samples'][0] = sample(0, old['started_ns'], old['completed_ns'] - old['started_ns'], True)
        self.mutate_run(fail)
        result = self.analyze()
        self.assertEqual(result['windows'][0]['goodputs'][0], 0)
        self.assertEqual(len(result['windows'][0]['goodputs']), 10)
        self.assertFalse(result['sampling_preconditions_satisfied'])

    def test_bootstrap_preserves_pairing_and_zero_denominator_samples(self):
        pairs = [[(2, 1), (20, 10)], [(6, 3), (60, 30)]]
        result = series.paired_bootstrap(pairs, 1000, 7)
        self.assertEqual(result['ratio_ci95'], [2, 2])
        self.assertEqual(result, series.paired_bootstrap(pairs, 1000, 7))
        zeros = series.paired_bootstrap([[(0, 0), (1, 0)]], 1000, 7)
        self.assertIsNone(zeros['ratio_ci95'])
        self.assertIsNone(zeros['median_ratio'])
        self.assertEqual(zeros['undefined_ratio_resamples'], 1000)

    def test_small_series_cannot_pass_sampling_gate(self):
        self.plan, self.manifest, self.plan_sha = fixture(self.root, starts=1, windows=2, warmups=1)
        result = self.analyze()
        self.assertFalse(result['sampling_preconditions_satisfied'])
        self.assertTrue(any('3 fresh' in item for item in result['sampling_precondition_failures']))

    def test_unknown_plan_fields_and_unequal_tuning_budget_rejected(self):
        altered = copy.deepcopy(self.plan)
        altered['ignore_failures'] = True
        with self.assertRaises(ValueError):
            series.plan(altered)
        entry = self.manifest['runs'][0]
        tuning = json.loads((self.root / entry['tuning_policy']['path']).read_bytes())
        tuning['trials_per_engine']['vllm'] = 5
        binding = save(self.root, 'tuning.json', tuning)
        self.plan['tuning_policy_sha256'] = binding['sha256']
        for record in self.manifest['runs']:
            record['tuning_policy'] = binding
        with self.assertRaisesRegex(ValueError, 'unequal'):
            self.analyze()


if __name__ == '__main__':
    unittest.main()
