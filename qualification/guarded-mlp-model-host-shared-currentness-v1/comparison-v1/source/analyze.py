"""Exact integer accounting of already captured inclusive host observations."""
import json

MAX_U64 = (1 << 64) - 1
COUNTERS = ('commands', 'command_ns', 'full_currentness_checks', 'full_currentness_ns',
    'operational_currentness_checks', 'operational_currentness_ns', 'kernel_admissions',
    'kernel_admission_ns', 'dispatches', 'dispatch_prepare_ns', 'dispatch_publish_ns',
    'dispatch_wait_ns', 'completion_polls', 'reads', 'read_bytes', 'read_ns', 'writes', 'write_bytes', 'write_ns')
SHARED = ('group_full_checks', 'group_full_ns', 'publication_full_checks', 'publication_full_ns')
IDS = (16366993098680759275, 10838076764495710945)


def require(ok, reason):
    if not ok:
        raise ValueError(reason)


def uint(value):
    require(type(value) is int and 0 <= value <= MAX_U64, 'strict u64')
    return value


def total(values):
    result = sum(uint(value) for value in values)
    require(result <= MAX_U64, 'u64 sum overflow')
    return result


def vector(value, count):
    require(type(value) is list and len(value) == count, 'counter vector extent')
    return [uint(v) for v in value]


def same(left, right):
    return json.dumps(left, sort_keys=True, allow_nan=False) == json.dumps(right, sort_keys=True, allow_nan=False)


def phases():
    result = ['fresh_enabled', 'setup_sealed']
    for forward in range(4):
        result.append('forward_%d/begin' % forward)
        for layer in range(36):
            result.extend('forward_%d/layer_%02d/%s' % (forward, layer, phase)
                          for phase in ('begin', 'prefix', 'paired', 'hidden'))
        result.append('forward_%d/done' % forward)
    return result + ['before_close']


def analyze(report, checked):
    require(report['schema'] == 'FerricGuardedMlpHostObservationV1'
            and checked['schema'] == 'ferric-guarded-mlp-model-host-observation-checked-v1'
            and report['native_closed'] is checked['native_close_confirmed'] is True
            and report['inclusive_nested_host_scopes'] is checked['inclusive_nested_host_scopes'] is True,
            'host report identity, Close and inclusive scope')
    for key in ('paired_generic_dispatch_timers_complete', 'tensor_stage_capture', 'gpu_time', 'gpu_overlap',
                'numerical_acceptance', 'full_model_acceptance', 'performance_claim', 'production_authority'):
        require(report[key] is checked[key] is False, 'host-only incomplete-timer nonclaims')
    require(checked['counter_names'] == list(COUNTERS) and checked['shared_counter_names'] == list(SHARED),
            'fixed counter names')
    snapshots, intervals = report['snapshots'], report['intervals']
    require(type(snapshots) is list and len(snapshots) == uint(checked['snapshots']) == 587
            and type(intervals) is list and len(intervals) == uint(checked['intervals']) == 586,
            'complete snapshot and interval census')
    require([row['phase'] for row in snapshots] == phases(), 'closed phase schedule')
    first = snapshots[0]
    require(uint(first['group_incarnation']) > 0 and vector(first['shared'], 4) == [0] * 4,
            'fresh group/shared baseline')
    for index, snapshot in enumerate(snapshots):
        require(uint(snapshot['group_incarnation']) == first['group_incarnation']
                and snapshot['shared_full_currentness'] is False
                and type(snapshot['ranks']) is list and len(snapshot['ranks']) == 2,
                'fixed group and conservative policy')
        vector(snapshot['shared'], 4)
        for rank, row in enumerate(snapshot['ranks']):
            values = vector(row['counters'], 19)
            require(uint(row['rank']) == rank and uint(row['unique_id']) == IDS[rank]
                    and uint(row['queue_epoch']) == uint(first['ranks'][rank]['queue_epoch'])
                    and row['cache_kernel_admission'] is row['raw_timestamp_queue'] is False
                    and values[4:6] == [0, 0], 'fixed rank/epoch and no policy change')
            if index == 0: require(values == [0] * 19, 'fresh rank baseline')
        if index:
            delta, previous = intervals[index - 1], snapshots[index - 1]
            uint(delta['host_elapsed_ns'])
            require(type(delta['ranks']) is list and len(delta['ranks']) == 2, 'two rank delta vectors')
            require(vector(delta['shared'], 4) == [a - b for a, b in zip(snapshot['shared'], previous['shared'])],
                    'shared exact subtraction/decrease')
            for rank in range(2):
                require(vector(delta['ranks'][rank], 19) == [a - b for a, b in zip(
                    snapshot['ranks'][rank]['counters'], previous['ranks'][rank]['counters'])],
                    'rank exact subtraction/decrease')
    require(same(checked['final_rank_counters'], [row['counters'] for row in snapshots[-1]['ranks']])
            and same(checked['final_shared_counters'], snapshots[-1]['shared']), 'checked final counters')
    forwards = vector(report['forward_host_ns'], 4)
    close = uint(report['close_host_ns'])
    require(uint(checked['close_host_ns']) == close and type(checked['forward_rows']) is list
            and len(checked['forward_rows']) == 4, 'checked forward/Close geometry')
    rows, claimed = [], set()
    for forward in range(4):
        start = 2 + 146 * forward
        indices = set(range(start, start + 145))
        groups = {name: {start + 1 + 4 * layer + offset for layer in range(36)}
                  for offset, name in enumerate(('prefix', 'paired', 'hidden'))}
        groups['other'] = indices - set.union(*groups.values())
        require(sum(len(g) for g in groups.values()) == len(set.union(*groups.values())) == 145
                and set.union(*groups.values()) == indices and not claimed & indices,
                'disjoint exhaustive forward interval partition')
        claimed |= indices
        walls = {name: total(intervals[i]['host_elapsed_ns'] for i in sorted(group))
                 for name, group in groups.items()}
        bracket = total(intervals[i]['host_elapsed_ns'] for i in sorted(indices))
        require(total(walls.values()) == bracket and forwards[forward] <= bracket, 'forward wall reconciliation')
        actual_checked = checked['forward_rows'][forward]
        layers = [dict(layer=layer, prefix=intervals[start + 1 + 4 * layer],
                       paired=intervals[start + 2 + 4 * layer], hidden_read=intervals[start + 3 + 4 * layer])
                  for layer in range(36)]
        require(same(actual_checked, dict(position=forward, forward_host_ns=forwards[forward],
                                         bracket_host_ns=bracket, layers=layers)), 'checked per-layer interval joins')
        rank_values = [[total(intervals[i]['ranks'][rank][column] for i in sorted(indices))
                        for column in range(19)] for rank in range(2)]
        shared_values = [total(intervals[i]['shared'][column] for i in sorted(indices)) for column in range(4)]
        for rank in range(2):
            require(rank_values[rank] == [a - b for a, b in zip(snapshots[start + 145]['ranks'][rank]['counters'],
                                                               snapshots[start]['ranks'][rank]['counters'])],
                    'rank telescoping sum')
        require(shared_values == [a - b for a, b in zip(snapshots[start + 145]['shared'], snapshots[start]['shared'])],
                'shared telescoping sum')
        rows.append(dict(position=forward, first_interval=start, interval_count=145,
            category_interval_counts={name: len(group) for name, group in groups.items()},
            disjoint_wall_ns=walls, bracket_host_ns=bracket, owner_run_host_ns=forwards[forward],
            inclusive_rank_counters=[dict(zip(COUNTERS, row)) for row in rank_values],
            inclusive_shared_counters=dict(zip(SHARED, shared_values))))
    outside = sorted(set(range(586)) - claimed)
    require(outside == [0, 1, 147, 293, 439, 585], 'exact nonforward snapshot intervals')
    outside_ns = total(intervals[i]['host_elapsed_ns'] for i in outside)
    snapshot_ns = total(v['host_elapsed_ns'] for v in intervals)
    require(total([outside_ns] + [row['bracket_host_ns'] for row in rows]) == snapshot_ns,
            'whole snapshot interval reconciliation')
    return dict(schema='ferric-guarded-mlp-model-host-analysis-v1', forwards=rows,
        snapshots=587, intervals=586, forward_intervals=580, nonforward_intervals=outside,
        nonforward_wall_ns=outside_ns, snapshot_wall_ns=snapshot_ns, close_host_ns=close,
        close_is_outside_snapshot_intervals=True, disjoint_wall_reconciled=True,
        counter_scopes_inclusive_and_nested=True, nested_counters_added_to_elapsed=False,
        paired_generic_dispatch_timers_complete=False, gpu_time=False, gpu_overlap=False,
        throughput=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False, production_authority=False)


def seconds(value):
    value = uint(value)
    return '%d.%09d' % divmod(value, 1000000000)


def markdown(result):
    lines = ['# Guarded Host Observation', '',
        'Host-only diagnostic. Wall categories are disjoint snapshot intervals; counters below are inclusive and may nest.',
        'No GPU duration, overlap, throughput, numerical acceptance or performance claim follows.', '',
        '| Forward | Prefix (s) | Paired (s) | Hidden Read (s) | Other (s) | Bracket (s) | Owner Run (s) |',
        '| ---: | ---: | ---: | ---: | ---: | ---: | ---: |']
    for row in result['forwards']:
        values = [row['position']] + [seconds(row['disjoint_wall_ns'][k]) for k in ('prefix', 'paired', 'hidden', 'other')]
        values += [seconds(row[k]) for k in ('bracket_host_ns', 'owner_run_host_ns')]
        lines.append('| ' + ' | '.join(map(str, values)) + ' |')
    lines += ['', '## Inclusive Rank Counters', '',
        'Do not sum these nested scopes into wall time. Generic paired-dispatch timers are incomplete.', '',
        '| Forward | Rank | Full Checks | Full Check (s) | Admissions | Admission (s) | Commands | Command (s) |',
        '| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |']
    for row in result['forwards']:
        for rank, counters in enumerate(row['inclusive_rank_counters']):
            values = [row['position'], rank, counters['full_currentness_checks'], seconds(counters['full_currentness_ns']),
                counters['kernel_admissions'], seconds(counters['kernel_admission_ns']), counters['commands'], seconds(counters['command_ns'])]
            lines.append('| ' + ' | '.join(map(str, values)) + ' |')
    lines += ['', '## Inclusive Shared Counters', '',
        '| Forward | Group Checks | Group Check (s) | Publication Checks | Publication Check (s) |',
        '| ---: | ---: | ---: | ---: | ---: |']
    for row in result['forwards']:
        c = row['inclusive_shared_counters']
        values = [row['position'], c['group_full_checks'], seconds(c['group_full_ns']),
                  c['publication_full_checks'], seconds(c['publication_full_ns'])]
        lines.append('| ' + ' | '.join(map(str, values)) + ' |')
    lines += ['', 'Nonforward snapshot intervals: ' + seconds(result['nonforward_wall_ns']) + ' s.',
        'All snapshot intervals: ' + seconds(result['snapshot_wall_ns']) + ' s.',
        'Close host wall, outside those intervals: ' + seconds(result['close_host_ns']) + ' s.',
        'Other includes layer-entry/forward-boundary host work. Snapshot wall also includes waits and process gaps.', '']
    return '\n'.join(lines)
