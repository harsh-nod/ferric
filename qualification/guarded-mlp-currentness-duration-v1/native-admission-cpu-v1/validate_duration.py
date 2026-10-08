"""Read-only admission of an instrumented Tail V4 case, never performance authority."""
import hashlib
from pathlib import Path

import validate_readiness as V
import validate_shared as S
import validate_matched as M
import validate_tail as T
from validate_readiness import keys, octets, ordered, parse, require, rust_pin, same, uint

SCHEMA = 'ferric-readiness40-tail-currentness-duration-case-v1'
RECORD_SCHEMA = 'FerricReadiness40TailCurrentnessDurationsV1'
MAX_BYTES = 65_536
STDERR_MAX_BYTES = 4096 + MAX_BYTES
WHOLE_NS = 3_600_000_000_000
U64_MAX = (1 << 64) - 1
CATEGORIES = ('before', 'discover', 'after', 'root_generation')
COUNTS = ('before_calls', 'full_discoveries', 'after_calls', 'generation_probes')
GROUPS = ('bank', 'layers', 'tail')
RECORD_FIELDS = ('schema instrumented policy_sha256 session worker_sha256 transcript_sha256 '
                 'forwards host_elapsed_nanoseconds bank_guarded_body_includes_callbacks '
                 'numerical_acceptance performance_claim execution_authority')


def durations(value):
    result = ordered(value, ' '.join(CATEGORIES))
    for key in CATEGORIES:
        pair = ordered(result[key], 'calls elapsed_ns')
        require(uint(pair['calls']) > 0, 'nonzero successful callback count')
        uint(pair['elapsed_ns'], WHOLE_NS)
        result[key] = pair
    require(result['before']['calls'] == result['after']['calls'],
            'balanced participant callback counts')
    return result


def measured(value):
    result = ordered(value, 'bank layers tail bank_guarded_body_ns')
    for group, discoveries in zip(GROUPS, (2, 72, 2)):
        result[group] = durations(result[group])
        require(result[group]['discover']['calls'] == discoveries,
                'exact per-forward scoped discoveries')
    require([result['bank'][key]['calls'] for key in CATEGORIES] == [726, 2, 726, 725],
            'fixed per-forward bank participant census')
    body = uint(result['bank_guarded_body_ns'], WHOLE_NS)
    subtotal = uint(sum(result['bank'][key]['elapsed_ns'] for key in CATEGORIES))
    require(subtotal <= body, 'bank callbacks contained in guarded-body duration')
    return result


def validate_record(raw, summary):
    require(type(raw) is bytes and 0 < len(raw) <= STDERR_MAX_BYTES,
            'bounded entire original diagnostic stderr')
    first, separator, second = raw.partition(b'\n')
    require(separator == b'\n' and second.endswith(b'\n') and b'\n' not in second[:-1]
            and 0 < len(second) <= MAX_BYTES, 'exactly two canonical newline records')
    first += separator
    policy = T.validate_record(first, summary)
    record = ordered(parse(second), RECORD_FIELDS)
    require(type(record['forwards']) is list and len(record['forwards']) == 40,
            'forty ordered original forwards')
    rows = []
    totals = {group: {key: dict(calls=0, elapsed_ns=0) for key in CATEGORIES} for group in GROUPS}
    for position, value in enumerate(record['forwards']):
        row = ordered(value, 'position measured')
        require(uint(row['position'], 0xffffffff) == position, 'original forward order')
        if position < 2:
            require(row['measured'] is None, 'first two ordinary forwards are unmeasured')
        else:
            row['measured'] = measured(row['measured'])
            for group in GROUPS:
                for key in CATEGORIES:
                    for field in ('calls', 'elapsed_ns'):
                        totals[group][key][field] = uint(totals[group][key][field]
                            + row['measured'][group][key][field], U64_MAX)
        rows.append(row)
    for group, policy_group in zip(GROUPS, ('banks', 'layers', 'tails')):
        for key, count in zip(CATEGORIES, COUNTS):
            require(totals[group][key]['calls'] == policy['counts'][policy_group][count],
                    'each category joins original closed policy counts')
            uint(totals[group][key]['elapsed_ns'], WHOLE_NS)
    expected = dict(schema=RECORD_SCHEMA, instrumented=True,
        policy_sha256=list(hashlib.sha256(first).digest()), session=policy['session'],
        worker_sha256=policy['worker_sha256'], transcript_sha256=policy['transcript_sha256'],
        forwards=rows, host_elapsed_nanoseconds=True, bank_guarded_body_includes_callbacks=True,
        numerical_acceptance=False, performance_claim=False, execution_authority=False)
    for name in ('policy_sha256', 'session', 'worker_sha256', 'transcript_sha256'):
        octets(record[name])
    require(same(record, expected) and V.encoded(expected) + b'\n' == second,
            'exact typed identity, flags, fields and canonical diagnostic bytes')
    return dict(policy=policy, diagnostic=expected)


def validate(mode, stdout, summary_raw, request, native_root, read, prompt):
    require(mode == 'tail_duration', 'explicit instrumented Tail diagnostic mode')
    require(type(stdout) is bytes and 0 < len(stdout) <= 128 << 10
            and type(summary_raw) is bytes and 0 < len(summary_raw) <= 128 << 10,
            'bounded original instrumented parent stdout and summary')
    wrapper, summary = parse(stdout), parse(summary_raw)
    keys(wrapper, 'schema observation currentness_policy host_timing')
    require(wrapper['schema'] == T.WRAPPERS['tail_timed']
            and same(wrapper['observation'], summary), 'unchanged original parent wrapper')
    checked = V.validate(summary_raw, request, Path(native_root), read, prompt,
                         policy_validator=validate_record)
    # The legacy callback slot receives the entire original stderr. Both parsed
    # records remain in this distinct result; no file or pin is substituted.
    records = checked.pop('shared_full_policy')
    checked['bank_scoped_census_tail_policy'] = records['policy']
    checked['currentness_duration_diagnostic'] = records['diagnostic']
    require(same(wrapper['currentness_policy'], records['policy']),
            'parent wrapper joins original first policy record')
    timing = M.shared_timing(wrapper, summary_raw, checked, native_root, read)
    return dict(schema=SCHEMA, mode=mode, ordinary=checked,
        policy=dict(name='bank_scoped_census_tail_duration', file=rust_pin(summary['files']['child_stderr']),
            policy_record=records['policy'], diagnostic_record=records['diagnostic'],
            no_policy_bytes_discarded=True, original_stderr_empty=False,
            temporal_equivalent_to_full=False, shared_full_currentness=False),
        timing=timing, summary=M.summary_pin(summary_raw, native_root), stdout=V.part(stdout),
        instrumented=True, parent_host_measurement=True, gpu_timing=False,
        bank_guarded_body_includes_callbacks=True, full_long_workload=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False,
        outer_owned_lineage_checked=False, cpu_qualification_checked=False)


def compare_same_side(current_raw, baseline_raw, checked, baseline_checked, read):
    require(checked['schema'] == SCHEMA and checked['mode'] == 'tail_duration'
            and checked['instrumented'] is True and checked['parent_host_measurement'] is True
            and checked['bank_guarded_body_includes_callbacks'] is True
            and checked['policy']['name'] == 'bank_scoped_census_tail_duration'
            and checked['policy']['no_policy_bytes_discarded'] is True
            and checked['policy']['original_stderr_empty'] is False
            and checked['policy']['temporal_equivalent_to_full'] is False
            and checked['policy']['shared_full_currentness'] is False
            and 'shared_full_policy' not in checked['ordinary']
            and all(checked[key] is False for key in ('gpu_timing', 'full_long_workload',
                'numerical_acceptance', 'performance_claim', 'production_authority')),
            'independently admitted diagnostic without numerical or performance claim')
    current = parse(current_raw)
    require(same(checked['summary'], M.summary_pin(current_raw,
                current['request']['base']['evidence_directory']))
            and same(checked['policy']['file'], rust_pin(current['files']['child_stderr'])),
            'same-side comparison uses original summary and whole stderr pin')
    pin = checked['policy']['file']
    raw = read(pin)
    require(type(raw) is bytes and len(raw) == pin['bytes']
            and hashlib.sha256(raw).hexdigest() == pin['sha256'], 'whole original diagnostic pin')
    records = validate_record(raw, current)
    require(same(records['policy'], checked['policy']['policy_record'])
            and same(records['policy'], checked['ordinary']['bank_scoped_census_tail_policy'])
            and same(records['diagnostic'], checked['policy']['diagnostic_record'])
            and same(records['diagnostic'], checked['ordinary']['currentness_duration_diagnostic']),
            'revalidate both original records before same-side comparison')
    parity = S.compare_same_side(current_raw, baseline_raw, checked['ordinary'], baseline_checked, read)
    parity.update(schema='ferric-readiness40-tail-currentness-duration-same-side-parity-v1',
        currentness_duration_policy=checked['policy'], instrumented=True,
        bank_guarded_body_includes_callbacks=True, temporal_equivalent_to_full=False,
        shared_full_currentness=False)
    return parity
