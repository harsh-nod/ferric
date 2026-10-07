"""Strict SharedFull admission of original policy bytes; no execution authority."""
import hashlib
from pathlib import Path
import validate_readiness as V
from validate_readiness import (CAPTURES, CONTROL_BYTES, PAYLOAD_BYTES, keys, octets,
    parse, part, require, rust_pin, same, uint)

OBSERVATION = 'ferric-guarded-mlp-readiness40-position5-data-observation-v1'
SCHEMA = 'ferric-readiness40-position5-shared-full-data-v1'
FIELDS = ('schema session device_ids child_pid worker_sha256 profile_sha256 registration_sha256 '
    'transcript_sha256 completed_forwards generated_tokens capture_positions shared_full_currentness '
    'cache_kernel_admission operational_currentness legacy_profile host_observer paired_hidden_reads '
    'paired_terminal native_closed full_long_workload numerical_acceptance performance_claim production_authority')


def validate_record(raw, summary):
    require(type(raw) is bytes and 0 < len(raw) <= 4096 and raw.endswith(b'\n'),
            'one bounded original shared policy record')
    value = parse(raw)
    keys(value, FIELDS)
    b = summary['bootstrap']['sequence']
    require(b['profile'] == 'readiness40_position5'
            and summary['schema'] == 'FerricGuardedMlpReadiness40Position5ObservationV1'
            and summary['request']['schema'] == 'FerricGuardedMlpReadiness40Position5RequestV1'
            and all(summary[k] is True for k in ('native_closed', 'child_exit_zero', 'process_group_absent')),
            'shared only after ordinary Position5 healthy Close')
    expected = dict(schema='FerricReadiness40Position5SharedFullPolicyV1',
        session=b['scope']['session'], device_ids=b['device_ids'], child_pid=b['scope']['child_identity'],
        worker_sha256=summary['request']['base']['worker']['sha256'],
        profile_sha256=summary['profile_sha256'], registration_sha256=b['registration'],
        transcript_sha256=summary['transcript_sha256'], completed_forwards=40, generated_tokens=[],
        capture_positions=[0, 5, 16, 39], shared_full_currentness=True,
        cache_kernel_admission=False, operational_currentness=False, legacy_profile=False,
        host_observer=False, paired_hidden_reads=False, paired_terminal=False, native_closed=True,
        full_long_workload=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False)

    require(same(value, expected) and V.encoded(expected) + b'\n' == raw,
            'exact canonical policy flags/identity/worker/transcript/framing')
    require(uint(value['child_pid'], 0x7fffffff) == summary['child_pid']
            and uint(summary['completed_forwards']) == 40 and summary['generated_tokens'] == []
            and octets(value['profile_sha256']) != bytes(32)
            and octets(value['worker_sha256']) != bytes(32)
            and octets(value['transcript_sha256']) != bytes(32), 'actual complete scope with nonzero identity')
    return value


def validate(stdout, summary_raw, request, directory, read, prompt):
    require(type(stdout) is bytes and 0 < len(stdout) <= 128 << 10, 'bounded original shared stdout')
    wrapper = parse(stdout)
    keys(wrapper, 'schema observation currentness_policy')
    summary = parse(summary_raw)
    require(wrapper['schema'] == 'FerricReadiness40Position5SharedFullObservationV1'
            and same(wrapper['observation'], summary), 'shared wrapper preserves original ordinary summary')
    # The only native entry fixes this callback. It receives unsanitized pinned
    # stderr only after every ordinary frame, payload, Close and byte-budget gate.
    checked = V.validate(summary_raw, request, Path(directory), read, prompt,
                         policy_validator=validate_record)
    record = checked['shared_full_policy']
    require(same(wrapper['currentness_policy'], record), 'wrapper joins original worker policy bytes')
    pin = rust_pin(summary['files']['child_stderr'])
    return checked, dict(schema=SCHEMA, policy_record=record, file=pin,
        shared_full_currentness=True, ordinary_wire_unchanged=True, no_policy_bytes_discarded=True,
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
    return dict(schema='ferric-readiness40-shared-full-same-side-parity-v1', passed=True,
        records=records, captures=payloads, all40_records_equal=True, all40_observation_pins_equal=True,
        all40_logit_pins_equal=True, all4_payloads_byte_equal=True, independent_numerical_reference=False,
        controls_individually_validated=True, controls_byte_equality_claimed=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
