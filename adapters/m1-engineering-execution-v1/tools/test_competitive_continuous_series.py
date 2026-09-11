import hashlib
import json
from pathlib import Path
import resource
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

import competitive_benchmark as client
import competitive_continuous as continuous
import competitive_continuous_series as series
import test_competitive_continuous as timeline_fixture
import test_competitive_series as old_fixture


def fixture(root, starts=3, windows=10, warmups=10):
    old, manifest, _ = old_fixture.fixture(root, starts=starts, windows=windows, warmups=warmups)
    settings = timeline_fixture.config(warmup_windows=warmups, measurement_windows=windows)
    binding = old_fixture.save(root, 'continuous-settings.json', settings)
    frozen = {key: old[key] for key in ('cell', 'comparison_scope', 'workload_sha256',
        'tuning_policy_sha256', 'engines', 'baseline', 'starts', 'engine_order',
        'bootstrap_seed', 'bootstrap_samples')}
    frozen.update(schema=series.PLAN_SCHEMA, settings=settings, settings_sha256=binding['sha256'],
                  **series.source_hashes())
    plan_binding = old_fixture.save(root, 'continuous-plan.json', frozen)
    for entry in manifest['runs']:
        original = json.loads((root / entry['run']['path']).read_bytes())
        value = original['workload']
        plan = continuous.schedule(value, settings)
        origin = 10**12
        records = []
        for item in plan['arrivals']:
            intended = origin + item['offset_ns']
            record = timeline_fixture.success(continuous.request_item(value, item), intended,
                                                intended + 1, intended + 100_000_000)
            records.append(record | {'sequence': item['sequence'], 'original_id': item['original_id']})
        run = {'origin_ns': origin, 'completed_ns': origin + plan['timeline']['guard_end_ns'] + 100_000_000,
               'records': records, 'completed': True, 'drain_expired': False, 'boundaries': [],
               'worker_errors': [], 'response_bytes_observed': 10, 'response_budget_exhausted': False}
        timeline_fixture.refresh_boundaries(run, plan, settings)
        report = {key: original[key] for key in ('authority', 'qualification', 'engine', 'identity',
            'identity_sha256', 'workload', 'workload_sha256', 'start_evidence', 'start_evidence_sha256',
            'tuning_policy', 'tuning_policy_sha256', 'started_unix_ns')}
        report.update(schema=continuous.SCHEMA, completed=True, endpoint='http://127.0.0.1:18980/v1/completions',
            settings=settings, settings_sha256=binding['sha256'], plan=plan, run=run,
            completed_unix_ns=report['started_unix_ns'] + run['completed_ns'] - origin,
            deadline_semantics=client.DEADLINE_SEMANTICS,
            window_semantics='adjacent-fixed-time-windows-under-one-continuous-arrival-schedule',
            goodput_semantics='fixed-output-completion-usage-credited-once-at-DONE', token_itl_available=False,
            **{key: value for key, value in series.source_hashes().items() if key != 'aggregator_sha256'})
        report['reduction'] = continuous.reduce(value, settings, plan, run)
        entry['settings'] = binding
        entry['run'] = old_fixture.save(root, entry['run']['path'], report)
    manifest.update(schema=series.RUNS_SCHEMA, plan_sha256=plan_binding['sha256'])
    return frozen, manifest, plan_binding['sha256']


class ContinuousSeriesTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.reset()

    def tearDown(self):
        self.directory.cleanup()

    def reset(self, **changes):
        self.plan, self.manifest, self.plan_sha = fixture(self.root, **changes)

    def analyze(self):
        return series.analyze(self.plan, self.manifest, self.root, self.plan_sha)

    def mutate_run(self, mutate, index=0, refresh=False):
        entry = self.manifest['runs'][index]
        report = json.loads((self.root / entry['run']['path']).read_bytes())
        mutate(report)
        if refresh:
            timeline_fixture.refresh_boundaries(report['run'], report['plan'], report['settings'])
            report['reduction'] = continuous.reduce(report['workload'], report['settings'], report['plan'], report['run'])
        entry['run'] = old_fixture.save(self.root, entry['run']['path'], report)

    def fail(self, kind, sequence=20, late=False):
        def change(report):
            occurrence = report['plan']['arrivals'][sequence]
            intended = report['run']['origin_ns'] + occurrence['offset_ns']
            if late:
                record = {'id': occurrence['id'], 'success': False, 'intended_arrival_ns': intended,
                          'started_ns': intended + 1, 'send_started_ns': intended + 1, 'send_delay_ns': 1,
                          'completed_ns': intended + 1_100_000_000, 'failure_kind': kind,
                          'error': kind, 'chunks': []}
            else:
                record = client.arrival_failure(occurrence, intended, intended + 1, kind)
            report['run']['records'][sequence] = record | {
                'sequence': sequence, 'original_id': occurrence['original_id']}
        self.mutate_run(change, refresh=True)

    def test_exact_replay_has_descriptive_statistics_without_qualification(self):
        result = self.analyze()
        self.assertTrue(result['comparison_valid'])
        self.assertTrue(result['sampling_preconditions_satisfied'])
        self.assertEqual(result['paired']['median_ratio'], 1)
        self.assertEqual(result['paired']['ratio_ci95'], [1, 1])
        self.assertEqual(result['paired']['bootstrap_unit'], 'whole-paired-start-timeline')
        self.assertEqual(result['paired']['independent_pair_count'], 3)
        self.assertEqual(result['prompt_token_counts_by_original'], {'r': 1})
        self.assertEqual(len(result['windows']), 6)
        self.assertTrue(all(len(item['goodputs']) == 10 for item in result['windows']))
        self.assertFalse(result['qualification'])
        self.assertFalse(result['qualification_preconditions_satisfied'])
        self.assertFalse(result['framework_win_claim'])
        self.assertEqual(result['aggregator_sha256'], series.source_hashes()['aggregator_sha256'])
        self.assertEqual(result['resource_policy']['collector_response_bytes_unchanged'], 64 * 1024**2)

    def test_every_imported_source_and_aggregator_must_match_frozen_plan_before_loading_runs(self):
        for key in series.source_hashes():
            with self.subTest(key=key):
                original = self.plan[key]
                self.plan[key] = '0' * 64
                with mock.patch.object(series.EvidenceBudget, 'load') as load:
                    with self.assertRaisesRegex(ValueError, 'local source differs'):
                        self.analyze()
                    load.assert_not_called()
                self.plan[key] = original

    def test_less_than_three_pairs_never_emits_a_bootstrap(self):
        self.reset(starts=2)
        result = self.analyze()
        self.assertTrue(result['comparison_valid'])
        self.assertIsNone(result['paired'])
        self.assertFalse(result['sampling_preconditions_satisfied'])
        with self.assertRaisesRegex(ValueError, 'three fresh'):
            series.paired_start_bootstrap([[(1, 1)], [(1, 1)]], 1000, 7)

    def test_all_raw_windows_occurrences_and_boundaries_are_required(self):
        for mutate in (lambda report: report['reduction']['windows'].pop(),
                       lambda report: report['run']['records'].pop(),
                       lambda report: report['run']['boundaries'].pop(),
                       lambda report: report['plan']['arrivals'].pop(),
                       lambda report: report['run']['records'][0].update(original_id='other')):
            with self.subTest(mutate=mutate):
                self.reset()
                self.mutate_run(mutate)
                with self.assertRaises(ValueError):
                    self.analyze()

    def test_derived_metric_raw_text_source_and_semantic_tampering_are_rejected(self):
        for mutate in (lambda report: report['reduction']['windows'][0].update(accepted_output_goodput_per_second=999),
                       lambda report: report['run']['records'][0].update(text='forged'),
                       lambda report: report.update(collector_sha256='0' * 64),
                       lambda report: report.update(token_itl_available=True),
                       lambda report: report.update(deadline_semantics='soft')):
            with self.subTest(mutate=mutate):
                self.reset()
                self.mutate_run(mutate)
                with self.assertRaises(ValueError):
                    self.analyze()

    def test_hashed_workload_settings_and_tuning_scope_are_frozen(self):
        for key in ('workload', 'settings', 'tuning_policy', 'identity'):
            with self.subTest(key=key):
                self.reset()
                entry = self.manifest['runs'][0]
                data = json.loads((self.root / entry[key]['path']).read_bytes())
                data['unfrozen'] = True
                entry[key] = old_fixture.save(self.root, 'drifted.json', data)
                with self.assertRaisesRegex(ValueError, 'frozen .* hash drifted'):
                    self.analyze()

    def test_prompt_usage_across_starts_is_replayed_then_compared(self):
        def change(report):
            for record in report['run']['records']:
                record['usage']['prompt_tokens'] = 2
                for chunk in record['chunks']:
                    if 'usage' in chunk['event']:
                        chunk['event']['usage']['prompt_tokens'] = 2
        self.mutate_run(change, index=4, refresh=True)
        with self.assertRaisesRegex(ValueError, 'prompt token usage changed across'):
            self.analyze()

    def test_fixed_completion_usage_and_late_success_are_not_relabelled(self):
        for mutate in (lambda report: report['run']['records'][0]['usage'].update(completion_tokens=1),
                       lambda report: report['run']['records'][0].update(
                           completed_ns=report['run']['origin_ns'] + 2 * 10**9)):
            self.reset()
            self.mutate_run(mutate)
            with self.assertRaises(ValueError):
                self.analyze()

    def test_ordinary_failure_stays_zero_and_late_failure_charges_origin_window(self):
        self.fail('client_overload')
        result = self.analyze()
        self.assertTrue(result['comparison_valid'])
        self.assertIsNotNone(result['paired'])
        self.assertEqual(result['windows'][0]['goodputs'][0], 0)
        self.assertFalse(result['sampling_preconditions_satisfied'])
        self.reset()
        self.fail('deadline', sequence=39, late=True)
        result = self.analyze()
        self.assertTrue(result['comparison_valid'])
        self.assertEqual(result['windows'][0]['goodputs'][-1], 0)
        self.assertEqual(len(result['windows'][0]['goodputs']), 10)

    def test_client_warmup_guard_and_drain_faults_never_rank_an_engine(self):
        for reason in ('client_budget', 'client_cancelled', 'warmup', 'guard', 'budget', 'drain'):
            with self.subTest(reason=reason):
                self.reset()
                if reason in ('client_budget', 'client_cancelled'):
                    self.fail(reason)
                elif reason in ('warmup', 'guard'):
                    self.fail('client_overload', sequence=0 if reason == 'warmup' else 40)
                else:
                    field = 'response_budget_exhausted' if reason == 'budget' else 'drain_expired'
                    self.mutate_run(lambda report: report['run'].update({field: True}), refresh=True)
                result = self.analyze()
                self.assertFalse(result['comparison_valid'])
                self.assertIsNone(result['paired'])
                self.assertTrue(result['comparison_invalid_reasons'])

    def test_external_fault_drift_and_gate_failure_nullify_statistics(self):
        for update in ({'clock_drift_percent': 4}, {'thermal_drift_percent': 4},
                       {'environment_unchanged': False}, {'link_errors': 1}, {'ecc_errors': 1},
                       {'faults': ['client host fault']}, {'admission_gates_passed': False}):
            with self.subTest(update=update):
                self.reset()
                entry = self.manifest['runs'][0]
                value = json.loads((self.root / entry['observations']['path']).read_bytes())
                entry['observations'] = old_fixture.save(self.root, 'fault.json', value | update)
                result = self.analyze()
                self.assertFalse(result['comparison_valid'])
                self.assertIsNone(result['paired'])

    def test_missing_reordered_and_reused_start_evidence_fails_closed(self):
        self.manifest['runs'].pop()
        with self.assertRaisesRegex(ValueError, 'missing paired'):
            self.analyze()
        self.reset()
        self.manifest['runs'].reverse()
        with self.assertRaisesRegex(ValueError, 'order or pairing'):
            self.analyze()
        self.reset()
        entry = self.manifest['runs'][4]
        first = json.loads((self.root / self.manifest['runs'][0]['start_evidence']['path']).read_bytes())
        later = json.loads((self.root / entry['start_evidence']['path']).read_bytes())
        for key in ('boot_id', 'pid', 'process_start_ticks'):
            later[key] = first[key]
        entry['start_evidence'] = old_fixture.save(self.root, 'reused-start.json', later)
        self.mutate_run(lambda report: report.update(start_evidence=later,
            start_evidence_sha256=entry['start_evidence']['sha256']), index=4)
        with self.assertRaisesRegex(ValueError, 'reused server start'):
            self.analyze()

    def test_readiness_wall_order_and_incomplete_run_are_checked(self):
        for mutate in (lambda report: report.update(started_unix_ns=1),
                       lambda report: report.update(completed_unix_ns=10**18),
                       lambda report: report.update(completed=False),
                       lambda report: report.update(run_error='interrupted')):
            self.reset()
            self.mutate_run(mutate)
            with self.assertRaises(ValueError):
                self.analyze()

    def test_bootstrap_resamples_whole_timelines_without_within_start_resampling(self):
        pairs = [[(0, 0), (10, 5), (20, 10)], [(0, 0), (10, 5), (20, 10)], [(0, 0), (10, 5), (20, 10)]]
        result = series.paired_start_bootstrap(pairs, 1000, 7)
        self.assertEqual(result['ratio_ci95'], [2, 2])
        self.assertEqual(result['difference_ci95'], [5, 5])
        self.assertEqual(result, series.paired_start_bootstrap(pairs, 1000, 7))
        zeros = series.paired_start_bootstrap([[(0, 0)], [(0, 0)], [(0, 0)]], 1000, 7)
        self.assertIsNone(zeros['ratio_ci95'])
        self.assertIsNone(zeros['median_ratio'])
        self.assertEqual(zeros['undefined_ratio_resamples'], 1000)
        with self.assertRaisesRegex(ValueError, 'invalid window'):
            series.paired_start_bootstrap([[(None, 0)]] * 3, 1000, 7)

    def test_file_aggregate_hash_and_path_bounds(self):
        budget = series.EvidenceBudget(self.root)
        binding = old_fixture.save(self.root, 'bounded.json', {'a': 'b' * 20})
        with self.assertRaisesRegex(ValueError, 'bounded evidence file'):
            budget.load(binding, limit=16)
        with mock.patch.object(series, 'SERIES_FILE_LIMIT', 16):
            with self.assertRaisesRegex(ValueError, 'series evidence byte budget'):
                budget.load(binding)
        with self.assertRaisesRegex(ValueError, 'hash drifted'):
            budget.load(binding | {'sha256': '0' * 64})
        with self.assertRaisesRegex(ValueError, 'escapes manifest'):
            budget.load(binding | {'path': '../outside.json'})
        (self.root / 'duplicate.json').write_bytes(b'{"x":1,"x":2}')
        duplicate = {'path': 'duplicate.json', 'sha256': hashlib.sha256((self.root / 'duplicate.json').read_bytes()).hexdigest()}
        with self.assertRaisesRegex(ValueError, 'duplicate JSON key'):
            budget.load(duplicate)

    def test_owned_cli_enforces_memory_limit_and_retains_failure_report(self):
        manifest = old_fixture.save(self.root, 'runs.json', self.manifest)
        args = [sys.executable, '-B', series.__file__, '--plan', str(self.root / 'continuous-plan.json'),
                '--plan-sha256', self.plan_sha, '--runs', str(self.root / manifest['path'])]
        output = self.root / 'accepted.json'
        result = subprocess.run(args + ['--output', str(output)], capture_output=True, text=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(output.read_bytes())
        self.assertTrue(report['completed'])
        self.assertLessEqual(report['effective_cli_address_space_bytes'], series.ADDRESS_SPACE_LIMIT)
        self.assertFalse(report['qualification'])
        again = subprocess.run(args + ['--output', str(output)], capture_output=True, text=True, timeout=15)
        self.assertNotEqual(again.returncode, 0)
        self.assertEqual(report, json.loads(output.read_bytes()))
        (self.root / self.manifest['runs'][0]['run']['path']).write_text('{}')
        failed = self.root / 'failed.json'
        result = subprocess.run(args + ['--output', str(failed)], capture_output=True, text=True, timeout=15)
        self.assertNotEqual(result.returncode, 0)
        report = json.loads(failed.read_bytes())
        self.assertFalse(report['completed'])
        self.assertFalse(report['comparison_valid'])
        self.assertIsNone(report['paired'])
        self.assertIn('hash drifted', report['analysis_error'])

    def test_memory_guard_never_raises_an_existing_process_limit(self):
        with mock.patch.object(resource, 'getrlimit', return_value=(1024, 2048)), \
             mock.patch.object(resource, 'setrlimit') as set_limit:
            self.assertEqual(series.limit_owned_process(), 1024)
            set_limit.assert_called_once_with(resource.RLIMIT_AS, (1024, 1024))

    def test_actual_owned_process_memory_exhaustion_emits_no_statistics(self):
        manifest = old_fixture.save(self.root, 'runs.json', self.manifest)
        output = self.root / 'memory-failure.json'
        code = ('import competitive_continuous_series as s; '
                's.ADDRESS_SPACE_LIMIT = 64 * 1024**2; '
                's.analyze = lambda *args: bytearray(128 * 1024**2); s.main()')
        before = resource.getrlimit(resource.RLIMIT_AS)
        result = subprocess.run([sys.executable, '-B', '-c', code,
            '--plan', str(self.root / 'continuous-plan.json'), '--plan-sha256', self.plan_sha,
            '--runs', str(self.root / manifest['path']), '--output', str(output)],
            cwd=Path(series.__file__).parent, capture_output=True, text=True, timeout=15)
        self.assertNotEqual(result.returncode, 0)
        report = json.loads(output.read_bytes())
        self.assertFalse(report['completed'])
        self.assertFalse(report['comparison_valid'])
        self.assertIsNone(report['paired'])
        self.assertIn('MemoryError', report['analysis_error'])
        self.assertEqual(report['effective_cli_address_space_bytes'], 64 * 1024**2)
        self.assertEqual(resource.getrlimit(resource.RLIMIT_AS), before)


if __name__ == '__main__':
    unittest.main()
