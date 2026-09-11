#!/usr/bin/env python3
"""Bounded paired V3 replay and descriptive statistics, never qualification."""

import argparse
import hashlib
import math
import os
from pathlib import Path
import random
import resource
import stat
import statistics
import json

import competitive_benchmark as client
import competitive_continuous as continuous
import competitive_series as replay

require = client.require
integer = client.integer
SMALL_FILE_LIMIT = 1024 * 1024
RUN_FILE_LIMIT = 256 * 1024 * 1024
SERIES_FILE_LIMIT = 2 * 1024**3
ADDRESS_SPACE_LIMIT = 2 * 1024**3
SOURCES = {'collector': continuous, 'client': client, 'replay_verifier': replay}
PLAN_SCHEMA = 'FerricCompetitiveContinuousSeriesPlanV3'
RUNS_SCHEMA = 'FerricCompetitiveContinuousSeriesRunsV3'
REPORT_SCHEMA = 'FerricCompetitiveContinuousSeriesReportV3'


def source_hashes():
    values = {name + '_sha256': hashlib.sha256(Path(module.__file__).read_bytes()).hexdigest()
              for name, module in SOURCES.items()}
    values['aggregator_sha256'] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    return values


def plan(value):
    replay.fields(value, {'schema', 'cell', 'comparison_scope', 'workload_sha256',
                         'tuning_policy_sha256', 'settings', 'settings_sha256', 'engines',
                         'baseline', 'starts', 'engine_order', 'bootstrap_seed', 'bootstrap_samples',
                         *source_hashes()}, 'continuous series plan')
    require(value['schema'] == PLAN_SCHEMA, 'unknown continuous series plan')
    require(type(value['cell']) is str and 0 < len(value['cell']) <= 128, 'invalid cell')
    require(value['baseline'] in ('vllm', 'sglang'), 'baseline must be explicitly frozen')
    replay.fields(value['engines'], {'ferric', value['baseline']}, 'engine identities')
    replay.fields(value['comparison_scope'], replay.SCOPE, 'comparison scope')
    for key, entry in value['comparison_scope'].items():
        if key.endswith('_sha256'):
            replay.sha(entry)
        else:
            require(type(entry) is str and 0 < len(entry) <= 256, f'invalid scope {key}')
    for identity in value['engines'].values():
        replay.sha(identity)
    for key in ('workload_sha256', 'tuning_policy_sha256', 'settings_sha256', *source_hashes()):
        replay.sha(value[key])
    continuous.settings(value['settings'])
    integer(value['starts'], 1, 32, 'start count')
    integer(value['bootstrap_seed'], 0, 2**64 - 1, 'bootstrap seed')
    integer(value['bootstrap_samples'], 1000, 20000, 'bootstrap samples')
    orders = value['engine_order']
    require(type(orders) is list and len(orders) == value['starts']
            and all(type(order) is list and len(order) == 2 and set(order) == set(value['engines'])
                    for order in orders), 'invalid frozen engine order')
    for key, actual in source_hashes().items():
        require(value[key] == actual, f'local source differs from frozen plan: {key}')
    return value


class EvidenceBudget:
    """One bounded parsed run at a time; count repeated inputs conservatively."""

    def __init__(self, root):
        self.root = root.resolve()
        self.used = 0

    def load(self, binding, limit=SMALL_FILE_LIMIT):
        replay.fields(binding, {'path', 'sha256'}, 'evidence binding')
        name = binding['path']
        require(type(name) is str and 0 < len(name) <= 4096, 'invalid evidence path')
        path = (self.root / name).resolve()
        require(path.is_relative_to(self.root), 'evidence path escapes manifest directory')
        expected = replay.sha(binding['sha256'])
        with path.open('rb') as source:
            info = os.fstat(source.fileno())
            require(stat.S_ISREG(info.st_mode) and 0 < info.st_size <= limit,
                    f'bounded evidence file size exceeded: {name}')
            require(self.used + info.st_size <= SERIES_FILE_LIMIT, 'series evidence byte budget exceeded')
            raw = source.read(limit + 1)
        require(0 < len(raw) <= limit and self.used + len(raw) <= SERIES_FILE_LIMIT,
                'evidence byte budget exceeded during read')
        self.used += len(raw)
        require(hashlib.sha256(raw).hexdigest() == expected, f'evidence hash drifted: {name}')
        return client.json_value(raw)


def tuning_policy(value, frozen):
    replay.fields(value, {'schema', 'calibration_workload_sha256', 'heldout_workload_sha256',
                          'trials_per_engine', 'selection_rule'}, 'tuning policy')
    require(value['schema'] == 'FerricCompetitiveTuningPolicyV1', 'unknown tuning policy')
    require(value['heldout_workload_sha256'] == frozen['workload_sha256']
            and replay.sha(value['calibration_workload_sha256']) != frozen['workload_sha256'],
            'held-out/calibration workload mismatch')
    replay.fields(value['trials_per_engine'], set(frozen['engines']), 'tuning budgets')
    budgets = [integer(n, 1, 10000, 'tuning trial budget') for n in value['trials_per_engine'].values()]
    require(len(set(budgets)) == 1 and type(value['selection_rule']) is str
            and 0 < len(value['selection_rule']) <= 4096, 'unequal or undefined tuning policy')


def replay_run(run, data, entry, frozen):
    require(type(run) is dict and run.get('schema') == continuous.SCHEMA
            and run.get('authority') == 'none' and run.get('qualification') is False
            and run.get('engine') == entry['engine'], 'run schema/authority/engine mismatch')
    require(run.get('completed') is True and 'run_error' not in run, 'incomplete or interrupted run')
    for kind in ('workload', 'identity', 'settings', 'start_evidence', 'tuning_policy'):
        require(run.get(kind) == data[kind] and run.get(kind + '_sha256') == entry[kind]['sha256'],
                f'run {kind} binding drifted')
    for key in (name + '_sha256' for name in SOURCES):
        require(run.get(key) == frozen[key], f'run source identity drifted: {key}')
    require(run.get('deadline_semantics') == client.DEADLINE_SEMANTICS
            and run.get('window_semantics') == 'adjacent-fixed-time-windows-under-one-continuous-arrival-schedule'
            and run.get('goodput_semantics') == 'fixed-output-completion-usage-credited-once-at-DONE'
            and run.get('token_itl_available') is False, 'run timing semantics drifted')
    client.endpoint(run.get('endpoint'))
    expected_plan = continuous.schedule(data['workload'], frozen['settings'])
    require(run.get('plan') == expected_plan, 'raw continuous schedule drifted')
    reduction = continuous.reduce(data['workload'], frozen['settings'], expected_plan, run.get('run'))
    require(run.get('reduction') == reduction, 'raw replay and saved reduction differ')
    return reduction


def paired_start_bootstrap(pairs, samples, seed):
    require(len(pairs) >= 3, 'descriptive bootstrap requires at least three fresh paired starts')
    require(all(group and len(group) == len(pairs[0]) for group in pairs), 'missing paired windows')
    require(all(type(value) in (int, float) and math.isfinite(value) and value >= 0
                for group in pairs for pair in group for value in pair), 'invalid window goodput')
    rng = random.Random(seed)
    left = [a for group in pairs for a, _ in group]
    right = [b for group in pairs for _, b in group]
    a, b = statistics.median(left), statistics.median(right)
    differences, ratios, undefined = [], [], 0
    for _ in range(samples):
        # Keep each complete paired timeline intact: adjacent windows can correlate.
        selected = [rng.choice(pairs) for _ in pairs]
        x = statistics.median(a for group in selected for a, _ in group)
        y = statistics.median(b for group in selected for _, b in group)
        differences.append(x - y)
        if y > 0:
            ratios.append(x / y)
        else:
            undefined += 1
    return {'candidate_median': a, 'baseline_median': b, 'median_ratio': a / b if b > 0 else None,
            'median_difference': a - b,
            'ratio_ci95': [client.percentile(ratios, .025), client.percentile(ratios, .975)]
                          if not undefined else None,
            'difference_ci95': [client.percentile(differences, .025), client.percentile(differences, .975)],
            'undefined_ratio_resamples': undefined, 'bootstrap_samples': samples, 'bootstrap_seed': seed,
            'bootstrap_unit': 'whole-paired-start-timeline', 'interpretation': 'descriptive-only',
            'independent_pair_count': len(pairs), 'windows_per_pair': len(pairs[0])}


def analyze(frozen, manifest, root, plan_sha):
    frozen = plan(frozen)
    replay.sha(plan_sha)
    replay.fields(manifest, {'schema', 'plan_sha256', 'runs'}, 'continuous run manifest')
    require(manifest['schema'] == RUNS_SCHEMA and manifest['plan_sha256'] == plan_sha,
            'run manifest plan mismatch')
    entries = manifest['runs']
    require(type(entries) is list and len(entries) == 2 * frozen['starts'], 'missing paired starts')
    expected_order = [(engine, index) for index, order in enumerate(frozen['engine_order']) for engine in order]
    budget = EvidenceBudget(root)
    seen_starts, seen_start_hashes, seen_run_hashes = set(), set(), set()
    observed_prompt_counts, results, reasons, invalid = {}, {}, [], []
    previous_wall_end = 0
    for entry, (engine, index) in zip(entries, expected_order):
        replay.fields(entry, {'engine', 'start_index', 'run', 'identity', 'workload', 'settings',
                             'tuning_policy', 'start_evidence', 'observations'}, 'run entry')
        require(entry['engine'] == engine and type(entry['start_index']) is int
                and entry['start_index'] == index, 'engine order or pairing drifted')
        data = {kind: budget.load(entry[kind]) for kind in
                ('identity', 'workload', 'settings', 'tuning_policy', 'start_evidence', 'observations')}
        for kind, expected in (('identity', frozen['engines'][engine]),
                               ('workload', frozen['workload_sha256']),
                               ('settings', frozen['settings_sha256']),
                               ('tuning_policy', frozen['tuning_policy_sha256'])):
            require(entry[kind]['sha256'] == expected, f'frozen {kind} hash drifted')
        require(data['identity'].get('engine') == engine
                and data['identity'].get('comparison_scope') == frozen['comparison_scope'],
                'comparison scope drifted')
        require(data['settings'] == frozen['settings'], 'frozen continuous settings drifted')
        tuning_policy(data['tuning_policy'], frozen)
        instance = replay.start_evidence(data['start_evidence'], engine, frozen['engines'][engine])
        require(instance not in seen_starts and entry['start_evidence']['sha256'] not in seen_start_hashes,
                'reused server start evidence')
        seen_starts.add(instance)
        seen_start_hashes.add(entry['start_evidence']['sha256'])
        prefix = f'{engine}/{index}: '
        if not replay.observations(data['observations']):
            invalid.append(prefix + 'external fault, gate or drift check failed')
        run = budget.load(entry['run'], RUN_FILE_LIMIT)
        require(entry['run']['sha256'] not in seen_run_hashes, 'reused raw run evidence')
        seen_run_hashes.add(entry['run']['sha256'])
        reduction = replay_run(run, data, entry, frozen)
        wall_start = integer(run.get('started_unix_ns'), data['start_evidence']['ready_unix_ns'],
                             2**64 - 1, 'run wall start')
        require(wall_start >= previous_wall_end, 'engine run order overlapped or drifted')
        previous_wall_end = integer(run.get('completed_unix_ns'), wall_start + 1, 2**64 - 1,
                                    'run wall completion')
        collection_duration = run['run']['completed_ns'] - run['run']['origin_ns']
        require(previous_wall_end - wall_start >= collection_duration,
                'wall interval is shorter than monotonic collection duration')
        for original, count in reduction['prompt_token_counts_by_original'].items():
            require(observed_prompt_counts.setdefault(original, count) == count,
                    'original prompt token usage changed across paired starts/engines')
        invalid.extend(prefix + reason for reason in reduction['global_invalid_reasons'])
        goodputs = [window['accepted_output_goodput_per_second'] for window in reduction['windows']]
        ordinary_failures = [window['request_failure'] for window in reduction['windows']]
        if any(ordinary_failures):
            reasons.append(prefix + 'request failures retained at zero window goodput')
        mean = statistics.mean(goodputs) if all(value is not None for value in goodputs) else None
        cv = statistics.pstdev(goodputs) / mean if mean else None
        if cv is None or cv > .05:
            reasons.append(prefix + 'goodput coefficient of variation exceeds 5% or is undefined')
        results[(engine, index)] = {'goodputs': goodputs, 'window_request_failures': ordinary_failures,
            'coefficient_of_variation': cv, 'measurement_valid': reduction['measurement_valid'],
            'warmup_completed_requests': reduction['warmup_completed_requests'],
            'completed_output_token_ledger': reduction['completed_output_token_ledger'],
            'arrival_failure_counts': reduction['arrival_failure_counts'],
            'run_sha256': entry['run']['sha256'], 'start_evidence_sha256': entry['start_evidence']['sha256']}
        # Do not retain parsed SSE runs across starts, including the last run during bootstrap.
        del run, reduction, data
    enough_pairs = frozen['starts'] >= 3
    if not enough_pairs:
        reasons.append('requires at least three fresh paired starts before descriptive bootstrap')
    settings = frozen['settings']
    if settings['measurement_windows'] < 10 or settings['warmup_windows'] < 10:
        reasons.append('requires at least ten measurement windows and ten actual warmup completions per start')
    if len({tuple(order) for order in frozen['engine_order']}) < 2:
        reasons.append('engine order did not rotate')
    method_failures = ['continuous fixed windows do not prove empirical stationarity',
                       'text-chunk TPOT does not establish token-level ITL SLOs',
                       'descriptive paired statistics do not establish a qualified framework win']
    paired = None
    if enough_pairs and not invalid:
        pairs = [list(zip(results[('ferric', index)]['goodputs'],
                          results[(frozen['baseline'], index)]['goodputs'])) for index in range(frozen['starts'])]
        paired = paired_start_bootstrap(pairs, frozen['bootstrap_samples'], frozen['bootstrap_seed'])
    return {'schema': REPORT_SCHEMA, 'authority': 'none', 'qualification': False, 'framework_win_claim': False,
            'cell': frozen['cell'], 'baseline': frozen['baseline'], 'plan_sha256': plan_sha,
            **source_hashes(), 'comparison_valid': not invalid, 'comparison_invalid_reasons': invalid,
            'statistic': 'median fixed-window SLO-filtered DONE output tokens/s; ordinary failed windows stay zero',
            'paired': paired, 'windows': [{'engine': engine, 'start_index': index, **result}
                                         for (engine, index), result in results.items()],
            'prompt_token_counts_by_original': observed_prompt_counts,
            'sampling_preconditions_satisfied': not (reasons or invalid),
            'sampling_precondition_failures': reasons + invalid,
            'qualification_preconditions_satisfied': False,
            'qualification_precondition_failures': reasons + invalid + method_failures,
            'resource_policy': {'run_json_bytes': RUN_FILE_LIMIT, 'aggregate_evidence_bytes': SERIES_FILE_LIMIT,
                                'cli_address_space_bytes': ADDRESS_SPACE_LIMIT,
                                'collector_response_bytes_unchanged': client.RUN_RESPONSE_LIMIT,
                                'evidence_bytes_read': budget.used},
            'external_evidence_status': 'hash-bound operator evidence, not authenticated or independently proved'}


def limit_owned_process():
    soft, hard = resource.getrlimit(resource.RLIMIT_AS)
    finite_limits = [value for value in (soft, hard) if value != resource.RLIM_INFINITY]
    limit = min(ADDRESS_SPACE_LIMIT, *finite_limits) if finite_limits else ADDRESS_SPACE_LIMIT
    resource.setrlimit(resource.RLIMIT_AS, (limit, limit))
    return limit


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('plan', 'runs', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--plan-sha256', required=True)
    args = parser.parse_args()
    memory_limit = limit_owned_process()
    report = {'schema': REPORT_SCHEMA, 'authority': 'none', 'qualification': False,
              'framework_win_claim': False, 'completed': False, 'comparison_valid': False, 'paired': None,
              'effective_cli_address_space_bytes': memory_limit}
    with args.output.open('x', encoding='utf-8') as output:
        try:
            frozen = replay.load(args.plan, args.plan_sha256)
            raw, manifest = client.read_input(args.runs)
            report.update(analyze(frozen, manifest, args.runs.resolve().parent, args.plan_sha256),
                          runs_manifest_sha256=hashlib.sha256(raw).hexdigest(), completed=True)
        except BaseException as error:
            report['analysis_error'] = f'{type(error).__name__}: {error}'
            raise
        finally:
            json.dump(report, output, indent=2, sort_keys=True, allow_nan=False)
            output.write('\n')


if __name__ == '__main__':
    main()
