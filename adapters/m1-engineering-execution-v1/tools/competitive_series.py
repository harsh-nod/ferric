#!/usr/bin/env python3
"""Strict paired serving evidence analysis; never authenticates or qualifies a run."""

import argparse
import hashlib
import json
import math
from pathlib import Path
import random
import re
import statistics

import competitive_benchmark as client

require = client.require
integer = client.integer
SETTINGS = {'concurrency', 'pending_capacity', 'arrival_policy', 'arrival_rate', 'arrival_seed',
            'timeout_seconds', 'ttft_slo_ms', 'tpot_slo_ms'}
SCOPE = {'model_sha256', 'tokenizer_sha256', 'config_sha256', 'weights_sha256',
         'hardware_topology_sha256', 'environment_policy_sha256',
         'cache_policy', 'sampling_policy', 'speculation_policy'}


def fields(value, expected, name):
    require(type(value) is dict and set(value) == expected, f'{name} fields drifted')


def sha(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value), 'invalid SHA-256')
    return value


def finite(value, low, high, label):
    require(type(value) in (int, float) and math.isfinite(value) and low <= value <= high,
            f'invalid {label}')
    return value


def load(path, expected, limit=1024 * 1024):
    with path.open('rb') as source:
        raw = source.read(limit + 1)
    require(0 < len(raw) <= limit, f'bounded evidence size exceeded: {path.name}')
    require(hashlib.sha256(raw).hexdigest() == sha(expected), f'evidence hash drifted: {path.name}')
    return client.json_value(raw)


def plan(value):
    fields(value, {'schema', 'cell', 'comparison_scope', 'workload_sha256', 'client_sha256',
                   'tuning_policy_sha256', 'settings', 'engines', 'baseline', 'starts',
                   'windows', 'warmups', 'engine_order', 'bootstrap_seed', 'bootstrap_samples'}, 'plan')
    require(value['schema'] == 'FerricCompetitiveSeriesPlanV1', 'unknown series plan')
    require(type(value['cell']) is str and 0 < len(value['cell']) <= 128, 'invalid cell')
    require(value['baseline'] in ('vllm', 'sglang'), 'baseline must be explicitly frozen')
    fields(value['engines'], {'ferric', value['baseline']}, 'engine identities')
    for identity in value['engines'].values():
        sha(identity)
    for key in ('workload_sha256', 'client_sha256', 'tuning_policy_sha256'):
        sha(value[key])
    fields(value['comparison_scope'], SCOPE, 'comparison scope')
    for key, entry in value['comparison_scope'].items():
        if key.endswith('_sha256'):
            sha(entry)
        else:
            require(type(entry) is str and 0 < len(entry) <= 256, f'invalid scope {key}')
    fields(value['settings'], SETTINGS, 'settings')
    settings = value['settings']
    integer(settings['concurrency'], 1, 32, 'concurrency')
    integer(settings['pending_capacity'], 0, 256, 'pending capacity')
    client.arrival_offsets(1, settings['arrival_policy'], settings['arrival_rate'], settings['arrival_seed'])
    finite(settings['timeout_seconds'], 1e-9, 3600, 'timeout')
    for key in ('ttft_slo_ms', 'tpot_slo_ms'):
        finite(settings[key], 1e-9, 3600000, key)
    integer(value['starts'], 1, 32, 'start count')
    integer(value['windows'], 1, 100, 'window count')
    integer(value['warmups'], 0, 100, 'warmup count')
    integer(value['bootstrap_seed'], 0, 2**64 - 1, 'bootstrap seed')
    integer(value['bootstrap_samples'], 1000, 20000, 'bootstrap samples')
    orders = value['engine_order']
    require(type(orders) is list and len(orders) == value['starts']
            and all(type(order) is list and len(order) == 2 and set(order) == set(value['engines'])
                    for order in orders), 'invalid frozen engine order')
    return value


def start_evidence(value, engine, identity):
    fields(value, {'schema', 'engine', 'identity_sha256', 'fresh_start', 'boot_id', 'pid',
                   'process_start_ticks', 'ready_unix_ns', 'launch_evidence_sha256'}, 'start evidence')
    require(value['schema'] == 'FerricCompetitiveServerStartEvidenceV1'
            and value['engine'] == engine and value['identity_sha256'] == identity,
            'start evidence identity mismatch')
    require(value['fresh_start'] is True, 'server start was not fresh')
    require(type(value['boot_id']) is str
            and re.fullmatch('[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}', value['boot_id']),
            'invalid server boot identity')
    integer(value['pid'], 1, 2**31 - 1, 'server PID')
    integer(value['process_start_ticks'], 1, 2**64 - 1, 'process start ticks')
    integer(value['ready_unix_ns'], 1, 2**64 - 1, 'server readiness timestamp')
    sha(value['launch_evidence_sha256'])
    return (value['boot_id'], value['pid'], value['process_start_ticks'])


def observations(value):
    fields(value, {'schema', 'steady_state_windows', 'clock_drift_percent', 'thermal_drift_percent',
                   'environment_unchanged', 'link_errors', 'ecc_errors', 'faults',
                   'admission_gates_passed', 'evidence_sha256'}, 'external observations')
    require(value['schema'] == 'FerricCompetitiveExternalObservationsV1', 'unknown observation schema')
    for field in ('steady_state_windows', 'environment_unchanged', 'admission_gates_passed'):
        require(type(value[field]) is bool, f'invalid observation {field}')
    for field in ('clock_drift_percent', 'thermal_drift_percent'):
        finite(value[field], 0, 10000, field)
    for field in ('link_errors', 'ecc_errors'):
        integer(value[field], 0, 2**64 - 1, field)
    require(type(value['faults']) is list and len(value['faults']) <= 100
            and all(type(fault) is str and len(fault) <= 1024 for fault in value['faults']), 'invalid faults')
    sha(value['evidence_sha256'])
    return (value['clock_drift_percent'] <= 3 and value['thermal_drift_percent'] <= 3
            and value['environment_unchanged'] and value['admission_gates_passed']
            and value['link_errors'] == value['ecc_errors'] == 0 and not value['faults'])


def verify_record(record, item, origin, offset, end, timeout):
    require(type(record) is dict and record.get('id') == item['id']
            and type(record.get('success')) is bool, 'missing, duplicate or reordered request')
    intended = origin + offset
    require(record.get('intended_arrival_ns') == intended, 'intended arrival drifted')
    completed = integer(record.get('completed_ns'), intended, end, 'completion clock')
    sent = record.get('send_started_ns')
    if sent is None:
        require(not record['success'] and record.get('started_ns') is None
                and record.get('send_delay_ns') is None and record.get('chunks') == []
                and record.get('failure_kind') in ('client_overload', 'queue_timeout', 'client_budget'),
                'invalid unsent failure record')
        if record['failure_kind'] == 'queue_timeout':
            require(completed >= intended + int(timeout * 1e9), 'premature queue timeout')
        return
    integer(sent, intended, completed, 'actual send clock')
    require(record.get('started_ns') == sent and record.get('send_delay_ns') == sent - intended,
            'send-delay clock drifted')
    if not record['success']:
        require(record.get('failure_kind') in ('deadline', 'request_error', 'client_budget'), 'invalid request failure')
        require(type(record.get('chunks')) is list, 'missing partial SSE evidence')
        return
    require(completed <= intended + int(timeout * 1e9), 'late successful request')
    chunks = record.get('chunks')
    require(type(chunks) is list and len(chunks) <= 100000, 'invalid raw SSE chunks')
    events = [(json.dumps(chunk['event'], allow_nan=False).encode(), chunk['received_ns']) for chunk in chunks]
    replay = client.summarize_stream(events + [(b'[DONE]', completed)], sent, item['max_tokens'])
    expected = dict(replay, ttft_ns=replay['first_text_ns'] - intended,
                    e2e_ns=completed - intended, wire_ttft_ns=replay['ttft_ns'], wire_e2e_ns=replay['e2e_ns'])
    for key, value in expected.items():
        require(record.get(key) == value, f'raw SSE/derived request field drifted: {key}')


def window(value, index, workload, settings, offsets):
    fields(value, {'index', 'started_ns', 'completed_ns', 'metrics', 'requests'}, 'window')
    require(integer(value['index'], 0, 99, 'window index') == index, 'missing or reordered window')
    origin = integer(value['started_ns'], 1, 2**64 - 1, 'window start')
    end = integer(value['completed_ns'], origin + 1, 2**64 - 1, 'window end')
    records = value['requests']
    require(type(records) is list and len(records) == len(workload['requests']), 'missing request records')
    for record, item, offset in zip(records, workload['requests'], offsets):
        verify_record(record, item, origin, offset, end, settings['timeout_seconds'])
    measured = client.aggregate(records, origin, end, settings['ttft_slo_ms'], settings['tpot_slo_ms'])
    measured['failure_counts'] = {kind: sum(record.get('failure_kind') == kind for record in records)
                                 for kind in client.FAILURE_KINDS}
    require(value['metrics'] == measured, 'derived window metrics drifted')
    # A failed window remains paired, with zero accepted goodput, never omitted.
    goodput = measured['output_goodput_per_second'] if measured['all_requests_succeeded'] else 0.0
    return goodput, measured['all_requests_succeeded']


def paired_bootstrap(pairs, samples, seed):
    rng = random.Random(seed)
    candidate = [left for group in pairs for left, _ in group]
    baseline = [right for group in pairs for _, right in group]
    point_left, point_right = statistics.median(candidate), statistics.median(baseline)
    ratios = []
    differences = []
    undefined = 0
    for _ in range(samples):
        left, right = [], []
        # Resample starts, then paired windows within each selected start.
        for _ in pairs:
            group = rng.choice(pairs)
            for _ in group:
                a, b = rng.choice(group)
                left.append(a)
                right.append(b)
        a, b = statistics.median(left), statistics.median(right)
        differences.append(a - b)
        if b > 0:
            ratios.append(a / b)
        else:
            undefined += 1
    return {'candidate_median': point_left, 'baseline_median': point_right,
            'median_ratio': point_left / point_right if point_right > 0 else None,
            'ratio_ci95': [client.percentile(ratios, .025), client.percentile(ratios, .975)]
                          if undefined == 0 else None,
            'median_difference': point_left - point_right,
            'difference_ci95': [client.percentile(differences, .025), client.percentile(differences, .975)],
            'undefined_ratio_resamples': undefined, 'bootstrap_samples': samples,
            'bootstrap_seed': seed, 'bootstrap_unit': 'paired-start-then-paired-window'}


def analyze(frozen, manifest, root, plan_sha):
    frozen = plan(frozen)
    require(hashlib.sha256(Path(client.__file__).read_bytes()).hexdigest() == frozen['client_sha256'],
            'local replay client implementation differs from frozen plan')
    fields(manifest, {'schema', 'plan_sha256', 'runs'}, 'run manifest')
    require(manifest['schema'] == 'FerricCompetitiveSeriesRunsV1'
            and manifest['plan_sha256'] == plan_sha, 'run manifest plan mismatch')
    entries = manifest['runs']
    require(type(entries) is list and len(entries) == 2 * frozen['starts'], 'missing paired starts')
    expected_order = [(engine, index) for index, order in enumerate(frozen['engine_order']) for engine in order]
    seen_starts, seen_hashes = set(), set()
    results, reasons = {}, []
    invalid_comparison = []
    total_bytes = 0
    previous_run_end = 0
    observed_prompt_counts = {}
    for entry, (engine, start_index) in zip(entries, expected_order):
        fields(entry, {'engine', 'start_index', 'run', 'identity', 'workload', 'tuning_policy',
                       'start_evidence', 'observations'}, 'run entry')
        require((entry['engine'], entry['start_index']) == (engine, start_index), 'engine order or pairing drifted')
        data = {}
        for kind in ('run', 'identity', 'workload', 'tuning_policy', 'start_evidence', 'observations'):
            binding = entry[kind]
            fields(binding, {'path', 'sha256'}, f'{kind} binding')
            require(type(binding['path']) is str and 0 < len(binding['path']) <= 4096, 'invalid evidence path')
            path = root / binding['path']
            total_bytes += path.stat().st_size
            require(total_bytes <= 512 * 1024 * 1024, 'series evidence exceeds 512 MiB')
            data[kind] = load(path, binding['sha256'], 64 * 1024 * 1024 if kind == 'run' else 1024 * 1024)
        identity_sha = frozen['engines'][engine]
        for kind, wanted in (('identity', identity_sha), ('workload', frozen['workload_sha256']),
                             ('tuning_policy', frozen['tuning_policy_sha256'])):
            require(entry[kind]['sha256'] == wanted, f'frozen {kind} identity drifted')
        require(data['identity'].get('engine') == engine
                and data['identity'].get('comparison_scope') == frozen['comparison_scope'], 'comparison scope drifted')
        tuning = data['tuning_policy']
        fields(tuning, {'schema', 'calibration_workload_sha256', 'heldout_workload_sha256',
                        'trials_per_engine', 'selection_rule'}, 'tuning policy')
        require(tuning['schema'] == 'FerricCompetitiveTuningPolicyV1', 'unknown tuning policy')
        require(tuning['heldout_workload_sha256'] == frozen['workload_sha256']
                and sha(tuning['calibration_workload_sha256']) != tuning['heldout_workload_sha256'],
                'held-out/calibration workload mismatch')
        fields(tuning['trials_per_engine'], set(frozen['engines']), 'tuning budgets')
        budgets = [integer(n, 1, 10000, 'tuning trial budget') for n in tuning['trials_per_engine'].values()]
        require(len(set(budgets)) == 1 and type(tuning['selection_rule']) is str
                and 0 < len(tuning['selection_rule']) <= 4096, 'unequal or undefined tuning policy')
        instance = start_evidence(data['start_evidence'], engine, identity_sha)
        require(instance not in seen_starts and entry['start_evidence']['sha256'] not in seen_hashes,
                'reused server start evidence')
        seen_starts.add(instance)
        seen_hashes.add(entry['start_evidence']['sha256'])
        if not observations(data['observations']):
            reason = f'{engine}/{start_index}: external fault, gate or drift check failed'
            reasons.append(reason)
            invalid_comparison.append(reason)
        run = data['run']
        require(run.get('schema') == 'FerricCompetitiveStreamingRunV2' and run.get('authority') == 'none'
                and run.get('qualification') is False and run.get('engine') == engine,
                'run schema/authority/engine mismatch')
        for kind in ('identity', 'workload', 'tuning_policy', 'start_evidence'):
            require(run.get(kind) == data[kind] and run.get(kind + '_sha256') == entry[kind]['sha256'],
                    f'run {kind} binding drifted')
        require(run.get('client_sha256') == frozen['client_sha256'], 'client identity drifted')
        for key, expected in frozen['settings'].items():
            require(run.get(key) == expected, f'run setting drifted: {key}')
        require(run.get('completed') is True and 'run_error' not in run, 'incomplete or interrupted run')
        wall_start = integer(run.get('started_unix_ns'), data['start_evidence']['ready_unix_ns'],
                             2**64 - 1, 'run wall start')
        require(wall_start >= previous_run_end, 'engine run order overlapped or drifted')
        previous_run_end = integer(run.get('completed_unix_ns'), max(wall_start + 1, previous_run_end + 1),
                                   2**64 - 1, 'run wall completion')
        require(run.get('ttft_semantics') == 'intended-arrival-to-first-nonempty-text-chunk'
                and run.get('e2e_semantics') == 'intended-arrival-to-DONE'
                and run.get('deadline_semantics') == 'soft-absolute-checks-with-per-read-socket-timeout'
                and run.get('tpot_semantics') == 'first-to-last-text-chunk-divided-by-usage-tokens-minus-one'
                and run.get('token_itl_available') is False, 'timing semantics drifted')
        require(type(run.get('response_budget_exhausted')) is bool, 'missing response-budget status')
        client_budget_fault = run['response_budget_exhausted']
        require(run.get('window_semantics') == 'finite-arrival-cohort-including-drain-not-steady-state',
                'window semantics drifted')
        values = client.workload(data['workload'])
        settings = frozen['settings']
        offsets = client.arrival_offsets(len(values['requests']), settings['arrival_policy'],
                                         settings['arrival_rate'], settings['arrival_seed'])
        require(run.get('arrival_offsets_ns') == offsets, 'arrival schedule drifted')
        goodputs, all_ok, previous_end = [], True, 0
        for kind, count in (('warmups', frozen['warmups']), ('samples', frozen['windows'])):
            windows = run.get(kind)
            require(type(windows) is list and len(windows) == count, f'missing {kind} windows')
            for index, record in enumerate(windows):
                require(record['started_ns'] >= previous_end, 'overlapping or reordered windows')
                goodput, ok = window(record, index, values, settings, offsets)
                for request in record['requests']:
                    client_budget_fault = client_budget_fault or request.get('failure_kind') == 'client_budget'
                    if request['success']:
                        count = request['usage']['prompt_tokens']
                        expected = observed_prompt_counts.setdefault(request['id'], count)
                        require(count == expected, 'prompt token usage changed across paired runs/windows')
                previous_end = record['completed_ns']
                all_ok = all_ok and ok
                if kind == 'samples':
                    goodputs.append(goodput)
        if not all_ok:
            reasons.append(f'{engine}/{start_index}: request failures retained at zero window goodput')
        if client_budget_fault:
            reason = f'{engine}/{start_index}: client response-evidence budget exhausted; not engine performance'
            reasons.append(reason)
            invalid_comparison.append(reason)
        mean = statistics.mean(goodputs)
        cv = statistics.pstdev(goodputs) / mean if mean > 0 else None
        if cv is None or cv > .05:
            reasons.append(f'{engine}/{start_index}: serving goodput coefficient of variation exceeds 5% or is undefined')
        results[(engine, start_index)] = {'goodputs': goodputs, 'coefficient_of_variation': cv,
                                         'all_requests_succeeded': all_ok}
    pairs = [list(zip(results[('ferric', index)]['goodputs'],
                      results[(frozen['baseline'], index)]['goodputs'])) for index in range(frozen['starts'])]
    if frozen['starts'] < 3 or frozen['windows'] < 10 or frozen['warmups'] < 10:
        reasons.append('requires at least 3 fresh starts, 10 recorded windows and 10 warmups per start')
    if len({tuple(order) for order in frozen['engine_order']}) < 2:
        reasons.append('engine order did not rotate')
    # Cohort windows and text-chunk TPOT cannot establish the release steady-state/ITL gates.
    method_failures = ['finite arrival cohorts do not establish steady-state windows or token-level ITL SLOs',
                       'soft HTTP read deadlines do not establish hard timeout qualification']
    return {'schema': 'FerricCompetitivePairedSeriesReportV1', 'authority': 'none',
            'qualification': False, 'framework_win_claim': False, 'cell': frozen['cell'],
            'baseline': frozen['baseline'], 'plan_sha256': plan_sha,
            'aggregator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            'comparison_valid': not invalid_comparison,
            'comparison_invalid_reasons': invalid_comparison,
            'statistic': 'median per-window SLO-filtered output tokens/s; failed windows contribute zero',
            'paired': paired_bootstrap(pairs, frozen['bootstrap_samples'], frozen['bootstrap_seed'])
                      if not invalid_comparison else None,
            'windows': [{'engine': engine, 'start_index': index, **result}
                        for (engine, index), result in results.items()],
            'sampling_preconditions_satisfied': not reasons,
            'sampling_precondition_failures': reasons,
            'qualification_preconditions_satisfied': not (reasons + method_failures),
            'qualification_precondition_failures': reasons + method_failures,
            'external_evidence_status': 'hash-bound operator evidence, not authenticated or independently proved'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--plan-sha256', required=True)
    parser.add_argument('--runs', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--require-preconditions', action='store_true')
    args = parser.parse_args()
    frozen = load(args.plan, args.plan_sha256)
    raw, manifest = client.read_input(args.runs)
    report = analyze(frozen, manifest, args.runs.resolve().parent, args.plan_sha256)
    report['runs_manifest_sha256'] = hashlib.sha256(raw).hexdigest()
    with args.output.open('x') as output:
        json.dump(report, output, indent=2, sort_keys=True, allow_nan=False)
        output.write('\n')
    if args.require_preconditions:
        require(report['qualification_preconditions_satisfied'], 'qualification preconditions are not satisfied')


if __name__ == '__main__':
    main()
