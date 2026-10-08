"""Strict three-record host-phase admission; no numerical or execution authority."""
import hashlib
from pathlib import Path

import validate_readiness as V
import validate_shared as S
import validate_matched as M
import validate_tail as T
import validate_duration as D
from validate_readiness import keys, octets, ordered, parse, require, rust_pin, same, uint

SCHEMA = 'ferric-readiness40-tail-forward-duration-case-v1'
RECORD_SCHEMA = 'FerricReadiness40ForwardPhaseDurationsV1'
MODE = 'tail_forward'
POLICY_NAME = 'bank_scoped_census_tail_forward'
MAX_BYTES = 32_768
STDERR_MAX_BYTES = D.STDERR_MAX_BYTES
WHOLE_NS = 3_600_000_000_000
PHASES = ('input', 'metadata', 'embedding', 'bank', 'layers', 'tail', 'frame', 'fence', 'commit')
RECORD_FIELDS = ('schema instrumented policy_sha256 currentness_record_sha256 session '
                 'worker_sha256 transcript_sha256 phase_order forwards host_elapsed_nanoseconds '
                 'disjoint_phases currentness_durations_nested gpu_timing numerical_acceptance '
                 'performance_claim execution_authority')


def validate_record(raw, summary):
    require(type(raw) is bytes and 0 < len(raw) <= STDERR_MAX_BYTES,
            'bounded entire original forward-phase stderr')
    lines = raw.split(b'\n')
    require(len(lines) == 4 and lines[-1] == b'' and all(lines[:3]),
            'exactly three LF-terminated original records')
    first, second, third = (line + b'\n' for line in lines[:3])
    require(len(third) <= MAX_BYTES, 'bounded third forward-phase record')
    # This is the actual original prefix, not a replacement file, pin or summary.
    # The outer Readiness validator authenticates the complete three-line file.
    original = D.validate_record(first + second, summary)
    policy, callbacks = original['policy'], original['diagnostic']
    record = ordered(parse(third), RECORD_FIELDS)
    require(type(record['phase_order']) is list and record['phase_order'] == list(PHASES),
            'fixed nine ordered serial phases')
    require(type(record['forwards']) is list and len(record['forwards']) == 40,
            'forty ordered original forward-phase rows')
    rows = []
    total = 0
    for position, value in enumerate(record['forwards']):
        row = ordered(value, 'position phase_ns forward_body_ns')
        require(uint(row['position'], 0xffffffff) == position, 'original forward-phase order')
        require(type(row['phase_ns']) is list and len(row['phase_ns']) == len(PHASES),
                'fixed phase extent')
        row['phase_ns'] = [uint(ns) for ns in row['phase_ns']]
        body = uint(row['forward_body_ns'], WHOLE_NS)
        require(uint(sum(row['phase_ns'])) == body, 'exact checked disjoint phase sum')
        total = uint(total + body)
        old = callbacks['forwards'][position]
        require(old['position'] == position, 'same original callback row')
        if position < 2:
            require(old['measured'] is None, 'first-use callbacks remain unmeasured')
        else:
            measured = old['measured']
            require(type(measured) is dict, 'warm callbacks required')
            require(measured['bank_guarded_body_ns'] <= row['phase_ns'][3]
                    and uint(sum(measured['layers'][key]['elapsed_ns'] for key in D.CATEGORIES))
                        <= row['phase_ns'][4]
                    and uint(sum(measured['tail'][key]['elapsed_ns'] for key in D.CATEGORIES))
                        <= row['phase_ns'][5], 'original callback containment in phase intervals')
        rows.append(row)
    require(total <= WHOLE_NS, 'whole forty-forward body duration bound')
    expected = dict(schema=RECORD_SCHEMA, instrumented=True,
        policy_sha256=list(hashlib.sha256(first).digest()),
        currentness_record_sha256=list(hashlib.sha256(second).digest()),
        session=policy['session'], worker_sha256=policy['worker_sha256'],
        transcript_sha256=policy['transcript_sha256'], phase_order=list(PHASES), forwards=rows,
        host_elapsed_nanoseconds=True, disjoint_phases=True, currentness_durations_nested=True,
        gpu_timing=False, numerical_acceptance=False, performance_claim=False, execution_authority=False)
    for name in ('policy_sha256', 'currentness_record_sha256', 'session',
                 'worker_sha256', 'transcript_sha256'):
        octets(record[name])
    require(same(record, expected) and V.encoded(expected) + b'\n' == third,
            'exact typed identity, flags, fields and canonical forward-phase bytes')
    return dict(policy=policy, diagnostic=callbacks, forward=expected)


def validate(mode, stdout, summary_raw, request, native_root, read, prompt):
    require(mode == MODE, 'explicit three-record forward-phase diagnostic mode')
    require(type(stdout) is bytes and 0 < len(stdout) <= 128 << 10
            and type(summary_raw) is bytes and 0 < len(summary_raw) <= 128 << 10,
            'bounded original instrumented parent stdout and summary')
    wrapper, summary = parse(stdout), parse(summary_raw)
    keys(wrapper, 'schema observation currentness_policy host_timing')
    require(wrapper['schema'] == T.WRAPPERS['tail_timed']
            and same(wrapper['observation'], summary), 'unchanged original parent wrapper')
    checked = V.validate(summary_raw, request, Path(native_root), read, prompt,
                         policy_validator=validate_record)
    records = checked.pop('shared_full_policy')
    checked['bank_scoped_census_tail_policy'] = records['policy']
    checked['currentness_duration_diagnostic'] = records['diagnostic']
    checked['forward_phase_diagnostic'] = records['forward']
    require(same(wrapper['currentness_policy'], records['policy']),
            'parent wrapper joins original first policy record')
    timing = M.shared_timing(wrapper, summary_raw, checked, native_root, read)
    return dict(schema=SCHEMA, mode=mode, ordinary=checked,
        policy=dict(name=POLICY_NAME, file=rust_pin(summary['files']['child_stderr']),
            policy_record=records['policy'], diagnostic_record=records['diagnostic'],
            forward_record=records['forward'], no_policy_bytes_discarded=True,
            original_stderr_empty=False, temporal_equivalent_to_full=False, shared_full_currentness=False),
        timing=timing, summary=M.summary_pin(summary_raw, native_root), stdout=V.part(stdout),
        instrumented=True, parent_host_measurement=True, gpu_timing=False,
        bank_guarded_body_includes_callbacks=True, forward_phase_durations=True,
        currentness_durations_nested=True, disjoint_forward_phases=True,
        full_long_workload=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False, outer_owned_lineage_checked=False, cpu_qualification_checked=False)


def compare_same_side(current_raw, baseline_raw, checked, baseline_checked, read):
    require(checked['schema'] == SCHEMA and checked['mode'] == MODE
            and all(checked[key] is True for key in ('instrumented', 'parent_host_measurement',
                'bank_guarded_body_includes_callbacks', 'forward_phase_durations',
                'currentness_durations_nested', 'disjoint_forward_phases'))
            and checked['policy']['name'] == POLICY_NAME
            and checked['policy']['no_policy_bytes_discarded'] is True
            and checked['policy']['original_stderr_empty'] is False
            and checked['policy']['temporal_equivalent_to_full'] is False
            and checked['policy']['shared_full_currentness'] is False
            and 'shared_full_policy' not in checked['ordinary']
            and all(checked[key] is False for key in ('gpu_timing', 'full_long_workload',
                'numerical_acceptance', 'performance_claim', 'production_authority')),
            'independently admitted forward phases without numerical or performance claim')
    current = parse(current_raw)
    require(same(checked['summary'], M.summary_pin(current_raw,
                current['request']['base']['evidence_directory']))
            and same(checked['policy']['file'], rust_pin(current['files']['child_stderr'])),
            'same-side comparison uses original summary and whole stderr pin')
    pin = checked['policy']['file']
    raw = read(pin)
    require(type(raw) is bytes and len(raw) == pin['bytes']
            and hashlib.sha256(raw).hexdigest() == pin['sha256'],
            'whole original three-record stderr pin')
    records = validate_record(raw, current)
    require(same(records['policy'], checked['policy']['policy_record'])
            and same(records['policy'], checked['ordinary']['bank_scoped_census_tail_policy'])
            and same(records['diagnostic'], checked['policy']['diagnostic_record'])
            and same(records['diagnostic'], checked['ordinary']['currentness_duration_diagnostic'])
            and same(records['forward'], checked['policy']['forward_record'])
            and same(records['forward'], checked['ordinary']['forward_phase_diagnostic']),
            'revalidate all three original records before same-side comparison')
    parity = S.compare_same_side(current_raw, baseline_raw, checked['ordinary'], baseline_checked, read)
    parity.update(schema='ferric-readiness40-tail-forward-duration-same-side-parity-v1',
        forward_phase_policy=checked['policy'], instrumented=True,
        bank_guarded_body_includes_callbacks=True, forward_phase_durations=True,
        currentness_durations_nested=True, disjoint_forward_phases=True,
        temporal_equivalent_to_full=False, shared_full_currentness=False)
    return parity

