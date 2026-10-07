"""Pure parent-wall timing admission after unchanged ordinary Position5 admission."""
import hashlib
from pathlib import Path

from validate_readiness import (CAPTURES, CONTROL_BYTES, PAYLOAD_BYTES, keys, octets,
    parse, part, require, rust_pin, same, uint)

OBSERVATION = 'ferric-guarded-mlp-readiness40-position5-data-observation-v1'
FALSE_FIELDS = ('gpu_timing', 'nested_control_timers_included', 'sidecar_publication_timed',
    'full_long_workload', 'numerical_acceptance', 'performance_claim', 'production_authority')


def u64(value):
    require(type(value) is int and 0 <= value < 1 << 64, 'strict timing u64')
    return value


def timeline(value):
    keys(value, 'source_preparation spawn_to_setup_seal forwards close_and_retirement postcheck_and_ordinary_publication total_ns')
    require(type(value['forwards']) is list and len(value['forwards']) == 40, 'forty timing rows')
    cursor = total = count = 0
    def span(row):
        nonlocal cursor, total, count
        keys(row, 'start_ns end_ns elapsed_ns')
        start, end, elapsed = (u64(row[k]) for k in ('start_ns', 'end_ns', 'elapsed_ns'))
        require(start == cursor and end >= start and end - start == elapsed,
                'timing gap/overlap/regression/duration')
        total = u64(total + elapsed); cursor = end; count += 1
    span(value['source_preparation']); span(value['spawn_to_setup_seal'])
    for position, row in enumerate(value['forwards']):
        keys(row, 'position generation prepare_write flush_to_frame_read validate_retain_commit elapsed_ns')
        require(u64(row['position']) == position and u64(row['generation']) == position + 1,
                'ordered timing position/generation')
        before = cursor
        for name in ('prepare_write', 'flush_to_frame_read', 'validate_retain_commit'):
            span(row[name])
        require(u64(row['elapsed_ns']) == cursor - before, 'forward disjoint extent')
    span(value['close_and_retirement']); span(value['postcheck_and_ordinary_publication'])
    require(count == 124 and total == cursor == u64(value['total_ns']), 'complete disjoint timing reconciliation')
    return value


def validate(stdout, summary_raw, summary_pin, checked, native_root, read):
    require(type(stdout) is bytes and len(stdout) <= 256 << 10
            and type(summary_raw) is bytes and len(summary_raw) <= 128 << 10, 'bounded timing stdout/summary')
    wrapper, summary = parse(stdout), parse(summary_raw)
    keys(wrapper, 'schema observation host_timing')
    require(wrapper['schema'] == 'FerricReadiness40Position5TimedObservationV1'
            and same(wrapper['observation'], summary), 'timed stdout preserves ordinary observation')
    require(checked['schema'] == OBSERVATION and checked['completed_forwards'] == 40
            and checked['generated_tokens'] == [] and checked['all40_transcript_checked'] is True
            and checked['selected_payloads_independently_checked'] == 4,
            'ordinary admission must precede timing admission')
    root = Path(native_root)
    keys(summary_pin, 'path bytes sha256')
    require(summary_pin == dict(path=str(root / 'complete.json'), bytes=len(summary_raw),
            sha256=hashlib.sha256(summary_raw).hexdigest()), 'actual ordinary completion pin')
    status = wrapper['host_timing']
    keys(status, 'complete file ordinary_retained_bytes retained_bytes_with_timing supervisor_metadata_allowance parent_host_measurement gpu_timing numerical_acceptance performance_claim')
    require(status['complete'] is True and status['parent_host_measurement'] is True
            and all(status[k] is False for k in ('gpu_timing', 'numerical_acceptance', 'performance_claim')),
            'timing status complete with false authority')
    pin = rust_pin(status['file'])
    require(pin['path'] == str(root / 'host-timing.json') and 0 < pin['bytes'] <= 64 << 10,
            'closed timing sidecar path/extent')
    raw = read(pin)
    require(type(raw) is bytes and len(raw) == pin['bytes']
            and hashlib.sha256(raw).hexdigest() == pin['sha256'], 'timing original bytes')
    report = parse(raw)
    keys(report, 'schema ordinary_complete profile_sha256 transcript_sha256 completed_forwards generated_tokens capture_positions timeline native_closed child_exit_zero process_group_absent parent_host_measurement ' + ' '.join(FALSE_FIELDS))
    require(report['schema'] == 'FerricReadiness40Position5ParentHostTimingV1'
            and rust_pin(report['ordinary_complete']) == summary_pin
            and octets(report['profile_sha256']) == octets(summary['profile_sha256'])
            and octets(report['transcript_sha256']) == octets(summary['transcript_sha256'])
            and u64(report['completed_forwards']) == 40 and u64(report['generated_tokens']) == 0
            and same(report['capture_positions'], [0, 5, 16, 39]), 'timing actual ordinary scope/hash join')
    require(all(report[k] is True and summary[k] is True
                for k in ('native_closed', 'child_exit_zero', 'process_group_absent'))
            and report['parent_host_measurement'] is True
            and all(report[k] is False for k in FALSE_FIELDS), 'closed timing scope and false authority')
    ordinary = u64(status['ordinary_retained_bytes']); reserve = u64(status['supervisor_metadata_allowance'])
    total = u64(status['retained_bytes_with_timing'])
    require(ordinary == u64(summary['files']['total_bytes'])
            and reserve == u64(summary['files']['supervisor_metadata_allowance'])
            and total == u64(ordinary + len(raw)) and u64(total + reserve) <= 32 << 20,
            'timing unchanged aggregate retention cap and exact original byte accounting')
    checked_timeline = timeline(report['timeline'])
    return dict(schema='ferric-readiness40-position5-parent-host-timing-data-v1',
        file=pin, ordinary_complete=summary_pin, timeline=checked_timeline, disjoint_spans=124,
        ordinary_retained_bytes=ordinary, retained_bytes_with_timing=total,
        supervisor_metadata_allowance=reserve, parent_host_measurement=True,
        gpu_timing=False, nested_control_timers_included=False, sidecar_publication_timed=False,
        full_long_workload=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False)


def compare_same_side(current_raw, baseline_raw, current_checked, baseline_checked, read):
    require(current_checked['schema'] == OBSERVATION
            and baseline_checked['schema'] == 'ferric-guarded-mlp-readiness40-position5-data-observation-v1',
            'separately admitted ordinary current and original position5 summaries')
    for checked in (current_checked, baseline_checked):
        require(checked['completed_forwards'] == 40 and checked['generated_tokens'] == []
                and checked['all40_transcript_checked'] is True
                and checked['selected_payloads_independently_checked'] == 4
                and checked['unselected_payloads_independently_checked'] is False
                and all(checked[k] is False for k in ('full_long_workload', 'numerical_acceptance',
                    'performance_claim', 'production_authority')), 'parity admission has no numerical authority')
    current, baseline = parse(current_raw), parse(baseline_raw)
    for value in (current, baseline):
        require(value['native_closed'] is True and value['child_exit_zero'] is True
                and value['process_group_absent'] is True, 'both original healthy Close')
    cb, bb = current['request']['base'], baseline['request']['base']
    immutable = ('source', 'images', 'expected_bundle_id', 'expected_model_id', 'device_ids',
                 'prompt', 'dispatch_timeout_ms', 'child_deadline_ms')
    require(all(same(cb[k], bb[k]) for k in immutable)
            and cb['session'] != bb['session']
            and all(same(current['request'][k], baseline['request'][k])
                for k in ('tiles_image', 'prefix_image', 'projection_image', 'guarded_image')),
            'same immutable model/images/fullprompt but distinct sessions')
    def body(rust_value):
        p = rust_pin(rust_value); raw = read(p)
        require(type(raw) is bytes and len(raw) == p['bytes']
                and hashlib.sha256(raw).hexdigest() == p['sha256'], 'parity retained body pin')
        return raw
    frames = []
    for value in (current, baseline):
        rows = [parse(line) for line in body(value['files']['frames']).splitlines()]
        require(len(rows) == 40, 'forty admitted parity records')
        frames.append(rows)
    projection = ('generation', 'position', 'input_token', 'output_token', 'bank', 'captured', 'observation', 'logits')
    records = []
    for position, (a, b) in enumerate(zip(*frames)):
        left = {k: a['completion'][k] for k in projection}
        right = {k: b['completion'][k] for k in projection}
        require(uint(left['position']) == position and same(left, right), 'all40 genuine histories/observation/logit pins equal')
        records.append(left)
    payloads = []
    for position, a, b in zip(CAPTURES, current['files']['captures'], baseline['files']['captures']):
        require(a['position'] == b['position'] == position, 'four exact parity positions')
        left, right = body(a['file']), body(b['file'])
        require(len(left) == len(right) == CONTROL_BYTES + PAYLOAD_BYTES
                and left[CONTROL_BYTES:] == right[CONTROL_BYTES:], 'all four complete model payload bytes equal')
        payloads.append(dict(position=position, payload=part(left[CONTROL_BYTES:]),
            current_capture=a['file'], baseline_capture=b['file']))
    return dict(schema='ferric-readiness40-host-timing-same-side-parity-v1', passed=True,
        records=records, captures=payloads, all40_records_equal=True, all40_observation_pins_equal=True,
        all40_logit_pins_equal=True, all4_payloads_byte_equal=True, independent_numerical_reference=False,
        controls_individually_validated=True, controls_byte_equality_claimed=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
