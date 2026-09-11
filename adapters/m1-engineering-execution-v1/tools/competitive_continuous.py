#!/usr/bin/env python3
"""Bounded continuous-timeline engineering collector, not serving qualification."""

import argparse
from collections import deque
from concurrent.futures import FIRST_COMPLETED, wait
import hashlib
import json
import math
from pathlib import Path
import random
import time

import competitive_benchmark as client
import competitive_series as replay

require = client.require
integer = client.integer
SCHEMA = 'FerricCompetitiveContinuousRunV3'
SETTINGS = {'schema', 'arrival_policy', 'arrival_rate', 'arrival_seed', 'concurrency',
            'pending_capacity', 'timeout_seconds', 'window_seconds', 'warmup_windows',
            'measurement_windows', 'max_requests', 'ttft_slo_ms', 'tpot_slo_ms',
            'workload_repetition'}


def settings(value):
    require(type(value) is dict and set(value) == SETTINGS, 'continuous settings fields drifted')
    require(value['schema'] == 'FerricContinuousTimelineSettingsV3', 'unknown continuous settings')
    require(value['workload_repetition'] == 'cyclic-in-file-order', 'workload repetition must be explicit')
    integer(value['concurrency'], 1, 32, 'concurrency')
    integer(value['pending_capacity'], 0, 256, 'pending capacity')
    integer(value['max_requests'], 1, 16384, 'total request cap')
    integer(value['warmup_windows'], 1, 100, 'warmup windows')
    integer(value['measurement_windows'], 1, 100, 'measurement windows')
    client.arrival_offsets(1, value['arrival_policy'], value['arrival_rate'], value['arrival_seed'])
    for key in ('timeout_seconds', 'window_seconds', 'ttft_slo_ms', 'tpot_slo_ms'):
        number = value[key]
        upper = 3600000 if key.endswith('_ms') else 3600
        require(type(number) in (int, float) and math.isfinite(number) and 0 < number <= upper,
                f'invalid {key}')
    duration = round(value['window_seconds'] * 1e9)
    timeout = int(value['timeout_seconds'] * 1e9)
    require(duration > 0 and timeout > 0, 'window and timeout must be at least one nanosecond')
    warmup_end = value['warmup_windows'] * duration
    measure_end = warmup_end + value['measurement_windows'] * duration
    require(warmup_end >= timeout, 'warmup duration must cover a complete request deadline')
    require(measure_end + 2 * timeout <= 3600 * 10**9, 'complete timeline exceeds one hour')
    return {'window_ns': duration, 'timeout_ns': timeout, 'warmup_end_ns': warmup_end,
            'measurement_end_ns': measure_end, 'guard_end_ns': measure_end + timeout,
            'drain_deadline_ns': measure_end + 2 * timeout}


def schedule(value, config):
    client.workload(value)
    timeline = settings(config)
    rng = random.Random(config['arrival_seed'])
    arrivals, elapsed = [], 0.0
    while True:
        sequence = len(arrivals)
        elapsed = (sequence / config['arrival_rate'] if config['arrival_policy'] == 'constant'
                   else elapsed + rng.expovariate(config['arrival_rate']))
        offset = round(elapsed * 1e9)
        if offset >= timeline['guard_end_ns']:
            break
        require(sequence < config['max_requests'], 'complete timeline exceeds total request cap')
        item = value['requests'][sequence % len(value['requests'])]
        arrivals.append({'sequence': sequence, 'id': f'occurrence-{sequence}',
                         'original_id': item['id'], 'offset_ns': offset,
                         'max_tokens': item['max_tokens']})
    require(arrivals, 'timeline contains no offered requests')
    return {'timeline': timeline, 'arrivals': arrivals,
            'planned_output_tokens': sum(item['max_tokens'] for item in arrivals),
            'response_byte_budget': client.RUN_RESPONSE_LIMIT}


def request_item(value, occurrence):
    original = value['requests'][occurrence['sequence'] % len(value['requests'])]
    return dict(original, id=occurrence['id'])


def collect(url, value, config, plan, run, request_fn=None):
    request_fn = client.arrival_request if request_fn is None else request_fn
    require(plan == schedule(value, config), 'continuous schedule drifted before admission')
    origin = time.monotonic_ns()
    timeline, arrivals = plan['timeline'], plan['arrivals']
    boundaries = [index * timeline['window_ns'] for index in
                  range(config['warmup_windows'] + config['measurement_windows'] + 1)]
    boundaries.append(timeline['guard_end_ns'])
    records = [None] * len(arrivals)
    run.update(origin_ns=origin, completed=False, drain_expired=False, records=records,
               boundaries=[], worker_errors=[])
    budget = client.ResponseBudget()
    pending, active = deque(), {}
    next_arrival, next_boundary = 0, 0

    def finish(future, sequence):
        result = future.result()
        records[sequence] = dict(result, sequence=sequence, original_id=arrivals[sequence]['original_id'])

    try:
        with client.request_workers(config['concurrency']) as (executor, cancel):
            while True:
                now = time.monotonic_ns()
                for future in [future for future in active if future.done()]:
                    sequence = active.pop(future)
                    finish(future, sequence)
                while pending and now >= origin + arrivals[pending[0]]['offset_ns'] + timeline['timeout_ns']:
                    sequence = pending.popleft()
                    occurrence = arrivals[sequence]
                    records[sequence] = dict(client.arrival_failure(
                        occurrence, origin + occurrence['offset_ns'], now, 'queue_timeout'),
                        sequence=sequence, original_id=occurrence['original_id'])

                def dispatch(sequence):
                    occurrence = arrivals[sequence]
                    active[executor.submit(request_fn, url, value['model'], request_item(value, occurrence),
                                           config['timeout_seconds'], origin + occurrence['offset_ns'],
                                           budget, cancel)] = sequence

                while pending and len(active) < config['concurrency']:
                    dispatch(pending.popleft())
                while next_arrival < len(arrivals) and origin + arrivals[next_arrival]['offset_ns'] <= now:
                    sequence = next_arrival
                    next_arrival += 1
                    occurrence = arrivals[sequence]
                    intended = origin + occurrence['offset_ns']
                    failure = None
                    if now >= intended + timeline['timeout_ns']:
                        failure = 'queue_timeout'
                    elif len(active) < config['concurrency']:
                        dispatch(sequence)
                    elif len(pending) < config['pending_capacity']:
                        pending.append(sequence)
                    else:
                        failure = 'client_overload'
                    if failure:
                        records[sequence] = dict(client.arrival_failure(occurrence, intended, now, failure),
                                                 sequence=sequence, original_id=occurrence['original_id'])
                while next_boundary < len(boundaries) and origin + boundaries[next_boundary] <= now:
                    planned = origin + boundaries[next_boundary]
                    observed = time.monotonic_ns()
                    finished = [record for record in records if record is not None]
                    run['boundaries'].append({'offset_ns': boundaries[next_boundary], 'planned_ns': planned,
                        'observed_ns': observed, 'observation_lag_ns': observed - planned,
                        'active': len(active), 'pending': len(pending), 'offered': next_arrival,
                        'completed': len(finished), 'failed': sum(not record['success'] for record in finished)})
                    next_boundary += 1
                if now >= origin + timeline['drain_deadline_ns'] and (active or pending):
                    run['drain_expired'] = True
                    cancel.set()
                if next_arrival == len(arrivals) and not pending and not active and next_boundary == len(boundaries):
                    run['completed'] = True
                    break
                deadlines = [now + 50_000_000]
                if next_arrival < len(arrivals):
                    deadlines.append(origin + arrivals[next_arrival]['offset_ns'])
                if next_boundary < len(boundaries):
                    deadlines.append(origin + boundaries[next_boundary])
                if pending:
                    deadlines.append(origin + arrivals[pending[0]]['offset_ns'] + timeline['timeout_ns'])
                if not run['drain_expired']:
                    deadlines.append(origin + timeline['drain_deadline_ns'])
                delay = max(0, (min(deadlines) - time.monotonic_ns()) / 1e9)
                if active:
                    wait(active, timeout=delay, return_when=FIRST_COMPLETED)
                else:
                    time.sleep(delay)
    finally:
        # The request_workers context signals cancellation and joins before this
        # final harvest. Keep partial worker evidence even after a dispatcher error.
        for future, sequence in active.items():
            try:
                finish(future, sequence)
            except BaseException as error:
                run['worker_errors'].append({'sequence': sequence, 'error': f'{type(error).__name__}: {error}'})
        run.update(completed_ns=time.monotonic_ns(), response_bytes_observed=budget.used,
                   response_budget_exhausted=budget.exhausted)


def phase(offset, timeline):
    for name, end in (('warmup', timeline['warmup_end_ns']),
                      ('measurement', timeline['measurement_end_ns']), ('guard', timeline['guard_end_ns'])):
        if offset < end:
            return name
    return 'drain'


def reduce(value, config, plan, run):
    require(plan == schedule(value, config), 'continuous schedule drifted at replay')
    require(run.get('completed') is True and not run.get('worker_errors'), 'incomplete continuous collection')
    origin = integer(run.get('origin_ns'), 1, 2**64 - 1, 'origin clock')
    end = integer(run.get('completed_ns'), origin + plan['timeline']['guard_end_ns'], 2**64 - 1, 'final clock')
    records = run.get('records')
    require(type(records) is list and len(records) == len(plan['arrivals']), 'missing occurrence records')
    require(type(run.get('response_budget_exhausted')) is bool and type(run.get('drain_expired')) is bool,
            'missing budget/drain status')
    integer(run.get('response_bytes_observed'), 0, client.RUN_RESPONSE_LIMIT, 'observed response bytes')
    timeline = plan['timeline']
    expected_boundaries = [index * timeline['window_ns'] for index in
                          range(config['warmup_windows'] + config['measurement_windows'] + 1)]
    expected_boundaries.append(timeline['guard_end_ns'])
    boundaries = run.get('boundaries')
    require(type(boundaries) is list and len(boundaries) == len(expected_boundaries), 'missing counter boundaries')
    previous_observed = origin
    for record, offset in zip(boundaries, expected_boundaries):
        require(record.get('offset_ns') == offset and record.get('planned_ns') == origin + offset,
                'boundary clock drifted')
        observed = integer(record.get('observed_ns'), max(previous_observed, origin + offset), end, 'boundary observation')
        require(record.get('observation_lag_ns') == observed - origin - offset, 'boundary lag drifted')
        active = integer(record.get('active'), 0, config['concurrency'], 'active boundary count')
        pending = integer(record.get('pending'), 0, config['pending_capacity'], 'pending boundary count')
        offered = integer(record.get('offered'), 0, len(records), 'offered boundary count')
        completed = integer(record.get('completed'), 0, offered, 'completed boundary count')
        integer(record.get('failed'), 0, completed, 'failed boundary count')
        require(offered == active + pending + completed, 'boundary conservation failed')
        previous_observed = observed
    totals = {name: 0 for name in ('warmup', 'measurement', 'guard', 'drain')}
    failures = {name: 0 for name in totals}
    prompt_counts = {}
    warmup_completions = 0
    global_faults = []
    if run['response_budget_exhausted']:
        global_faults.append('client response-evidence budget exhausted')
    if run['drain_expired'] or end > origin + timeline['drain_deadline_ns']:
        global_faults.append('final drain deadline was exceeded')
    for record, occurrence in zip(records, plan['arrivals']):
        require(type(record) is dict and record.get('sequence') == occurrence['sequence']
                and record.get('original_id') == occurrence['original_id'], 'occurrence identity drifted')
        replay.verify_record(record, request_item(value, occurrence), origin, occurrence['offset_ns'],
                             end, config['timeout_seconds'])
        arrival_phase = phase(occurrence['offset_ns'], timeline)
        if record['success']:
            count = record['usage']['prompt_tokens']
            require(prompt_counts.setdefault(occurrence['original_id'], count) == count,
                    'original prompt token usage drifted between occurrences')
            completion_phase = phase(record['completed_ns'] - origin, timeline)
            totals[completion_phase] += record['usage']['completion_tokens']
            warmup_completions += completion_phase == 'warmup'
        else:
            failures[arrival_phase] += 1
            if record['failure_kind'] in ('client_budget', 'client_cancelled'):
                global_faults.append(record['failure_kind'])
            if arrival_phase != 'measurement':
                global_faults.append(f'{arrival_phase} request failure')
    if warmup_completions < config['warmup_windows']:
        global_faults.append('declared warmup completion count was not reached before measurement')
    windows = []
    for index in range(config['measurement_windows']):
        start = origin + timeline['warmup_end_ns'] + index * timeline['window_ns']
        stop = start + timeline['window_ns']
        completions = [record for record in records if start <= record['completed_ns'] < stop]
        arrivals = [record for record in records if start <= record['intended_arrival_ns'] < stop]
        metrics = client.aggregate(completions, start, stop, config['ttft_slo_ms'], config['tpot_slo_ms'])
        cohort = client.aggregate(arrivals, start, stop, config['ttft_slo_ms'], config['tpot_slo_ms'])
        failed = any(not record['success'] for record in completions + arrivals)
        windows.append({'index': index, 'started_ns': start, 'completed_ns': stop,
                        'completion_metrics': metrics,
                        'arrival_outcomes': {'requests': len(arrivals),
                            'successful_requests': cohort['successful_requests'],
                            'failed_requests': cohort['failed_requests'],
                            'completed_after_window': sum(record['completed_ns'] >= stop for record in arrivals),
                            **{key: cohort[key] for key in ('ttft_ms', 'tpot_ms', 'e2e_ms')}},
                        'request_failure': failed,
                        'accepted_output_goodput_per_second': (None if global_faults else
                            0.0 if failed else metrics['output_goodput_per_second'])})
    return {'measurement_valid': not global_faults and not any(window['request_failure'] for window in windows),
            'global_invalid_reasons': sorted(set(global_faults)), 'windows': windows,
            'completed_output_token_ledger': totals, 'arrival_failure_counts': failures,
            'prompt_token_counts_by_original': prompt_counts,
            'warmup_completed_requests': warmup_completions,
            'completed_output_tokens_total': sum(totals.values()),
            'drain_seconds': max(0, end - origin - timeline['guard_end_ns']) / 1e9,
            'qualification': False, 'framework_win_claim': False, 'paired_series_admitted': False,
            'remaining_limits': ['fixed windows do not prove empirical stationarity',
                                 'no token-level ITL evidence', 'V3 paired-series contract is not implemented']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--endpoint', type=client.endpoint, required=True)
    parser.add_argument('--engine', choices=('ferric', 'vllm', 'sglang'), required=True)
    for name in ('workload', 'identity', 'settings', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--start-evidence', type=Path)
    parser.add_argument('--tuning-policy', type=Path)
    args = parser.parse_args()
    report = {'schema': SCHEMA, 'authority': 'none', 'qualification': False, 'completed': False,
              'engine': args.engine,
              'endpoint': args.endpoint, 'deadline_semantics': client.DEADLINE_SEMANTICS,
              'window_semantics': 'adjacent-fixed-time-windows-under-one-continuous-arrival-schedule',
              'goodput_semantics': 'fixed-output-completion-usage-credited-once-at-DONE',
              'token_itl_available': False, 'run': {}}
    for name in ('workload', 'identity', 'settings', 'start_evidence', 'tuning_policy'):
        path = getattr(args, name)
        if path is not None:
            raw, value = client.read_input(path)
            report[name] = value
            report[name + '_sha256'] = hashlib.sha256(raw).hexdigest()
    require(type(report['identity']) is dict and report['identity'].get('engine') == args.engine,
            'engine identity mismatch')
    for name, path in (('collector', Path(__file__)), ('client', Path(client.__file__)),
                       ('replay_verifier', Path(replay.__file__))):
        report[name + '_sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
    report['plan'] = schedule(report['workload'], report['settings'])
    with args.output.open('x', encoding='utf-8') as output:
        report['started_unix_ns'] = time.time_ns()
        try:
            collect(args.endpoint, report['workload'], report['settings'], report['plan'], report['run'])
            report['reduction'] = reduce(report['workload'], report['settings'], report['plan'], report['run'])
            report['completed'] = True
        except BaseException as error:
            report['run_error'] = f'{type(error).__name__}: {error}'
            raise
        finally:
            report['completed_unix_ns'] = time.time_ns()
            json.dump(report, output, indent=2, allow_nan=False)
            output.write('\n')
    print(json.dumps(report['reduction'], allow_nan=False))


if __name__ == '__main__':
    main()
