import contextlib
import copy
import io
import json
from pathlib import Path
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock

import competitive_benchmark as client
import competitive_continuous as continuous
import competitive_series as series
import test_competitive_series as series_fixture
import test_serve_ferric as serving_fixture


def workload():
    return {'schema': client.SCHEMA, 'model': 'Qwen/Qwen3-8B', 'requests': [
        {'id': 'original', 'prompt': 'hello', 'max_tokens': 2}]}


def config(**changes):
    result = {'schema': 'FerricContinuousTimelineSettingsV3', 'arrival_policy': 'constant',
              'arrival_rate': 2, 'arrival_seed': 7, 'concurrency': 32, 'pending_capacity': 8,
              'timeout_seconds': 1, 'window_seconds': 1, 'warmup_windows': 1,
              'measurement_windows': 2, 'max_requests': 100, 'ttft_slo_ms': 1000,
              'tpot_slo_ms': 1000, 'workload_repetition': 'cyclic-in-file-order'}
    return result | changes


def success(item, intended, sent, done):
    event = {'choices': [{'index': 0, 'text': 'x', 'finish_reason': 'length'}],
             'usage': {'prompt_tokens': 1, 'completion_tokens': item['max_tokens']}}
    raw = client.summarize_stream([(json.dumps(event).encode(), done), (b'[DONE]', done)],
                                  sent, item['max_tokens'])
    return {'id': item['id'], 'success': True, **raw, 'intended_arrival_ns': intended,
            'send_started_ns': sent, 'send_delay_ns': sent - intended,
            'wire_ttft_ns': raw['ttft_ns'], 'wire_e2e_ns': raw['e2e_ns'],
            'ttft_ns': done - intended, 'e2e_ns': done - intended}


def fixture(options=None):
    value, settings = workload(), config() if options is None else options
    plan = continuous.schedule(value, settings)
    origin = 10**12
    records = []
    for occurrence in plan['arrivals']:
        intended = origin + occurrence['offset_ns']
        record = success(continuous.request_item(value, occurrence), intended, intended + 1,
                         intended + 100_000_000)
        records.append(record | {'sequence': occurrence['sequence'], 'original_id': occurrence['original_id']})
    run = {'origin_ns': origin, 'completed_ns': origin + plan['timeline']['guard_end_ns'] + 100_000_000,
           'records': records, 'completed': True, 'drain_expired': False, 'boundaries': [],
           'worker_errors': [], 'response_bytes_observed': 10, 'response_budget_exhausted': False}
    refresh_boundaries(run, plan, settings)
    return value, settings, plan, run


def refresh_boundaries(run, plan, settings):
    timeline = plan['timeline']
    offsets = [index * timeline['window_ns'] for index in
               range(settings['warmup_windows'] + settings['measurement_windows'] + 1)]
    run['boundaries'] = []
    for offset in offsets + [timeline['guard_end_ns']]:
        when = run['origin_ns'] + offset
        offered = sum(item['offset_ns'] <= offset for item in plan['arrivals'])
        finished = [item for item in run['records'] if item['completed_ns'] <= when]
        run['boundaries'].append({'offset_ns': offset, 'planned_ns': when, 'observed_ns': when,
            'observation_lag_ns': 0, 'active': offered - len(finished), 'pending': 0,
            'offered': offered, 'completed': len(finished), 'failed': sum(not item['success'] for item in finished)})


class TimelineTests(unittest.TestCase):
    def test_one_seeded_schedule_has_unique_occurrences_and_unchanged_original_requests(self):
        value = workload()
        value['requests'].append({'id': 'second', 'prompt': 'another', 'max_tokens': 3})
        frozen = copy.deepcopy(value)
        plan = continuous.schedule(value, config())
        self.assertEqual(value, frozen)
        self.assertEqual([item['offset_ns'] for item in plan['arrivals']], list(range(0, 4 * 10**9, 500_000_000)))
        self.assertEqual(len({item['id'] for item in plan['arrivals']}), 8)
        self.assertEqual([item['original_id'] for item in plan['arrivals']], ['original', 'second'] * 4)
        self.assertEqual(plan['planned_output_tokens'], 20)
        self.assertEqual(plan['response_byte_budget'], 64 * 1024 * 1024)
        poisson = config(arrival_policy='poisson', arrival_rate=4)
        self.assertEqual(continuous.schedule(value, poisson), continuous.schedule(value, poisson))
        self.assertNotEqual(continuous.schedule(value, poisson), continuous.schedule(value, poisson | {'arrival_seed': 8}))

    def test_preflight_rejects_infeasible_or_unbounded_timeline_without_truncation(self):
        for change in ({'max_requests': 7}, {'concurrency': 33}, {'pending_capacity': 257},
                       {'max_requests': 16385}, {'workload_repetition': 'implicit'},
                       {'timeout_seconds': 2}, {'window_seconds': 3600},
                       {'arrival_rate': float('nan')}, {'arrival_rate': 1000001},
                       {'window_seconds': 1e-12}, {'arrival_seed': True}, {'ttft_slo_ms': 3600001}):
            with self.subTest(change=change), self.assertRaises(ValueError):
                continuous.schedule(workload(), config(**change))
        continuous.schedule(workload(), config(ttft_slo_ms=10000))

    def test_half_open_done_windows_credit_fixed_usage_once_including_loaded_guard_and_drain(self):
        value, settings, plan, run = fixture()
        for sequence, done_offset in ((1, 10**9), (3, 2 * 10**9), (5, 3 * 10**9), (7, 4 * 10**9)):
            item = plan['arrivals'][sequence]
            intended = run['origin_ns'] + item['offset_ns']
            run['records'][sequence] = success(continuous.request_item(value, item), intended, intended + 1,
                run['origin_ns'] + done_offset) | {'sequence': sequence, 'original_id': item['original_id']}
        refresh_boundaries(run, plan, settings)
        result = continuous.reduce(value, settings, plan, run)
        self.assertEqual(result['completed_output_token_ledger'],
                         {'warmup': 2, 'measurement': 8, 'guard': 4, 'drain': 2})
        self.assertEqual(result['completed_output_tokens_total'], 16)
        self.assertEqual([item['completion_metrics']['output_tokens_per_second'] for item in result['windows']], [4, 4])
        self.assertEqual([item['arrival_outcomes']['completed_after_window'] for item in result['windows']], [1, 1])
        self.assertFalse(result['qualification'])
        self.assertFalse(result['framework_win_claim'])
        self.assertFalse(result['paired_series_admitted'])

    def test_failure_observed_later_is_charged_to_original_arrival_window(self):
        value, settings, plan, run = fixture()
        occurrence = plan['arrivals'][3]
        intended = run['origin_ns'] + occurrence['offset_ns']
        run['records'][3] = {'id': occurrence['id'], 'original_id': occurrence['original_id'], 'sequence': 3,
            'success': False, 'intended_arrival_ns': intended, 'started_ns': intended + 1,
            'send_started_ns': intended + 1, 'send_delay_ns': 1, 'completed_ns': intended + 1100000000,
            'failure_kind': 'deadline', 'error': 'deadline', 'chunks': []}
        refresh_boundaries(run, plan, settings)
        result = continuous.reduce(value, settings, plan, run)
        self.assertEqual(result['windows'][0]['arrival_outcomes']['failed_requests'], 1)
        self.assertEqual([window['accepted_output_goodput_per_second'] for window in result['windows']], [0, 0])
        self.assertFalse(result['measurement_valid'])
        self.assertEqual(len(result['windows']), 2)

    def test_warmup_guard_budget_cancel_and_incomplete_drain_fail_closed(self):
        for reason in ('warmup', 'guard', 'budget', 'cancel', 'drain'):
            with self.subTest(reason=reason):
                value, settings, plan, run = fixture()
                if reason in ('warmup', 'guard', 'cancel'):
                    sequence = 6 if reason == 'guard' else 0
                    occurrence = plan['arrivals'][sequence]
                    intended = run['origin_ns'] + occurrence['offset_ns']
                    run['records'][sequence] = client.arrival_failure(
                        occurrence, intended, intended + 1,
                        'client_cancelled' if reason == 'cancel' else 'client_overload') | {
                            'sequence': sequence, 'original_id': occurrence['original_id']}
                    refresh_boundaries(run, plan, settings)
                elif reason == 'budget':
                    run['response_budget_exhausted'] = True
                else:
                    run['drain_expired'] = True
                result = continuous.reduce(value, settings, plan, run)
                self.assertFalse(result['measurement_valid'])
                self.assertTrue(result['global_invalid_reasons'])
                self.assertTrue(all(window['accepted_output_goodput_per_second'] is None
                                    for window in result['windows']))

    def test_missing_identity_boundary_usage_and_late_success_are_rejected(self):
        for mutation in (lambda run: run['records'].pop(),
                         lambda run: run['records'][0].update(original_id='wrong'),
                         lambda run: run['boundaries'].pop(),
                         lambda run: run['boundaries'][0].update(active=33),
                         lambda run: run['records'][0]['usage'].update(completion_tokens=1),
                         lambda run: run['records'][0].update(completed_ns=run['origin_ns'] + 2 * 10**9)):
            with self.subTest(mutation=mutation):
                value, settings, plan, run = fixture()
                mutation(run)
                with self.assertRaises(ValueError):
                    continuous.reduce(value, settings, plan, run)

    def test_existing_paired_series_rejects_v3_even_with_rebound_file_hash(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            frozen, manifest, sha = series_fixture.fixture(root, starts=1, windows=1, warmups=1)
            entry = manifest['runs'][0]
            report = json.loads((root / entry['run']['path']).read_bytes())
            report['schema'] = continuous.SCHEMA
            entry['run'] = series_fixture.save(root, entry['run']['path'], report)
            with self.assertRaisesRegex(ValueError, 'schema'):
                series.analyze(frozen, manifest, root, sha)

    def test_repeated_canonical_prompt_usage_must_be_consistent(self):
        value, settings, plan, run = fixture()
        run['records'][1]['usage']['prompt_tokens'] = 2
        with self.assertRaisesRegex(ValueError, 'prompt token usage drifted'):
            continuous.reduce(value, settings, plan, run)

    def test_warmup_time_alone_does_not_replace_declared_completion_count(self):
        value, settings, plan, run = fixture(config(arrival_rate=.1, warmup_windows=3))
        result = continuous.reduce(value, settings, plan, run)
        self.assertEqual(result['warmup_completed_requests'], 1)
        self.assertFalse(result['measurement_valid'])
        self.assertTrue(any('warmup completion count' in reason for reason in result['global_invalid_reasons']))


class CollectorTests(unittest.TestCase):
    def test_final_drain_expiry_cancels_and_joins_remaining_owned_worker(self):
        value = workload()
        settings = config(window_seconds=.02, timeout_seconds=.02, measurement_windows=1, arrival_rate=1)
        plan, run = continuous.schedule(value, settings), {}
        threads = {thread.ident for thread in threading.enumerate()}

        def blocked(_url, _model, item, _timeout, intended, _budget, cancel):
            self.assertTrue(cancel.wait(1))
            return client.arrival_failure(item, intended, time.monotonic_ns(), 'client_cancelled')

        started = time.monotonic()
        continuous.collect('unused', value, settings, plan, run, request_fn=blocked)
        self.assertLess(time.monotonic() - started, .6)
        self.assertTrue(run['drain_expired'])
        self.assertEqual(run['records'][0]['failure_kind'], 'client_cancelled')
        self.assertFalse(continuous.reduce(value, settings, plan, run)['measurement_valid'])
        self.assertEqual({thread.ident for thread in threading.enumerate()}, threads)

    def test_dispatcher_exception_harvests_cancelled_worker_before_propagating(self):
        value, settings, run = workload(), config(), {}
        plan = continuous.schedule(value, settings)
        threads = {thread.ident for thread in threading.enumerate()}

        def blocked(_url, _model, item, _timeout, intended, _budget, cancel):
            self.assertTrue(cancel.wait(1))
            return client.arrival_failure(item, intended, time.monotonic_ns(), 'client_cancelled')

        with mock.patch.object(continuous, 'wait', side_effect=RuntimeError('dispatcher interrupted')), \
                self.assertRaisesRegex(RuntimeError, 'dispatcher interrupted'):
            continuous.collect('unused', value, settings, plan, run, request_fn=blocked)
        self.assertFalse(run['completed'])
        self.assertEqual(run['records'][0]['failure_kind'], 'client_cancelled')
        self.assertEqual(len(run['records']), len(plan['arrivals']))
        self.assertEqual({thread.ident for thread in threading.enumerate()}, threads)

    def test_window_boundary_does_not_drain_or_stop_new_admissions(self):
        settings = config(window_seconds=.1, warmup_windows=2, timeout_seconds=.2,
                          arrival_rate=50, concurrency=2, pending_capacity=2)
        value = workload()
        plan = continuous.schedule(value, settings)
        starts, finishes = {}, {}

        def request(_url, _model, item, _timeout, intended, _budget, cancel):
            sequence = int(item['id'].split('-')[1])
            starts[sequence] = time.monotonic_ns()
            if sequence == 11:
                cancel.wait(.14)
            finishes[sequence] = time.monotonic_ns()
            return success(item, intended, starts[sequence], finishes[sequence])

        run = {}
        continuous.collect('unused', value, settings, plan, run, request_fn=request)
        result = continuous.reduce(value, settings, plan, run)
        self.assertLess(starts[16], finishes[11])
        self.assertEqual(len(run['records']), len(plan['arrivals']))
        self.assertTrue(result['measurement_valid'], result)
        self.assertTrue(all(boundary['active'] <= 2 and boundary['pending'] <= 2 for boundary in run['boundaries']))

    def test_overload_retains_all_occurrences_with_bounded_pending_and_active(self):
        settings = config(window_seconds=.1, timeout_seconds=.1, measurement_windows=1,
                          arrival_rate=100, concurrency=1, pending_capacity=1)
        value, run = workload(), {}
        plan = continuous.schedule(value, settings)

        def request(_url, _model, item, _timeout, intended, _budget, cancel):
            start = time.monotonic_ns()
            cancel.wait(.02)
            return success(item, intended, start, time.monotonic_ns())

        continuous.collect('unused', value, settings, plan, run, request_fn=request)
        self.assertTrue(run['completed'])
        self.assertEqual(len(run['records']), len(plan['arrivals']))
        self.assertTrue(any(record.get('failure_kind') == 'client_overload' for record in run['records']))
        self.assertTrue(all(boundary['active'] <= 1 and boundary['pending'] <= 1 for boundary in run['boundaries']))

    def test_exhausted_budget_does_not_drop_future_scheduled_occurrences(self):
        settings = config(window_seconds=.01, timeout_seconds=.01, arrival_rate=100, concurrency=1)
        value, run = workload(), {}
        plan = continuous.schedule(value, settings)

        def request(_url, _model, item, _timeout, intended, budget, _cancel):
            try:
                budget.consume(2)
            except ValueError:
                return client.arrival_failure(item, intended, time.monotonic_ns(), 'client_budget')
            raise AssertionError('fixture budget was not exhausted')

        original = client.ResponseBudget
        with mock.patch.object(client, 'ResponseBudget', side_effect=lambda: original(1)):
            continuous.collect('unused', value, settings, plan, run, request_fn=request)
        self.assertEqual(len(run['records']), len(plan['arrivals']))
        self.assertTrue(all(record['failure_kind'] == 'client_budget' for record in run['records']))
        self.assertFalse(continuous.reduce(value, settings, plan, run)['measurement_valid'])

    def test_actual_loopback_adapter_keeps_one_child_across_windows_and_guard(self):
        value = workload()
        settings = config(window_seconds=.1, timeout_seconds=.1, arrival_rate=10, concurrency=1)
        plan, run = continuous.schedule(value, settings), {}
        with serving_fixture.running() as (backend, server, trace):
            pid = backend.process.pid
            host, port = server.server_address
            continuous.collect(f'http://{host}:{port}/v1/completions', value, settings, plan, run)
            self.assertEqual(backend.process.pid, pid)
            self.assertEqual(len([entry for entry in serving_fixture.commands(trace) if entry['op'] == 'submit']),
                             len(plan['arrivals']))
        self.assertTrue(continuous.reduce(value, settings, plan, run)['measurement_valid'])
        self.assertGreater(run['response_bytes_observed'], 0)

    def test_partial_cli_report_is_retained_on_dispatcher_interruption(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, value in (('workload', workload()), ('settings', config()), ('identity', {'engine': 'ferric'})):
                (root / f'{name}.json').write_text(json.dumps(value))
            argv = ['continuous', '--endpoint', 'http://127.0.0.1:18980/v1/completions', '--engine', 'ferric',
                    '--workload', str(root / 'workload.json'), '--settings', str(root / 'settings.json'),
                    '--identity', str(root / 'identity.json'), '--output', str(root / 'report.json')]
            with mock.patch.object(sys, 'argv', argv), mock.patch.object(continuous, 'collect', side_effect=KeyboardInterrupt), \
                    self.assertRaises(KeyboardInterrupt), contextlib.redirect_stdout(io.StringIO()):
                continuous.main()
            result = json.loads((root / 'report.json').read_bytes())
            self.assertEqual(result['schema'], continuous.SCHEMA)
            self.assertIn('KeyboardInterrupt', result['run_error'])
            self.assertFalse(result['qualification'])
            self.assertFalse(result['completed'])
            self.assertEqual(result['plan']['response_byte_budget'], 64 * 1024 * 1024)
            self.assertNotIn('reduction', result)


if __name__ == '__main__':
    unittest.main()
