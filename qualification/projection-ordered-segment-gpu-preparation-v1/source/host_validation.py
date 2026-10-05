"""Projection AR4 host counters; unchanged native decode validation runs first."""
import hashlib
import json
from pathlib import Path
import re

import layer_validation as V
from smoke_validation import normalized_pin

MAX_BYTES = 65536
U64 = (1 << 64) - 1
PHASES = ('fresh_enabled', 'setup_sealed', 'forward_0', 'forward_1', 'forward_2', 'forward_3', 'before_close')
COUNTERS = ('commands', 'command_ns', 'full_currentness_checks', 'full_currentness_ns',
    'operational_currentness_checks', 'operational_currentness_ns', 'kernel_admissions', 'kernel_admission_ns',
    'dispatches', 'dispatch_prepare_ns', 'dispatch_publish_ns', 'dispatch_wait_ns', 'completion_polls',
    'reads', 'read_bytes', 'read_ns', 'writes', 'write_bytes', 'write_ns')
SHARED = ('group_full_checks', 'group_full_ns', 'publication_full_checks', 'publication_full_ns')
FALSE = ('gpu_time', 'numerical_acceptance', 'performance_claim', 'production_authority')


def pin(value):
    V.keys(value, 'path bytes sha256')
    digest = value['sha256']
    if type(digest) is str:
        V.require(re.fullmatch('[0-9a-f]{64}', digest), 'canonical FilePin hex digest')
        digest = list(bytes.fromhex(digest))
    else:
        V.digest(digest)
    return normalized_pin(dict(value, sha256=digest), 512 << 20)


def same(a, b):
    # Unlike Python equality, this distinguishes booleans from integer words.
    return json.dumps(a, sort_keys=True, separators=(',', ':'), allow_nan=False) == \
        json.dumps(b, sort_keys=True, separators=(',', ':'), allow_nan=False)


def member(raw, selected):
    """Return the exact JSON value bytes using decoder boundaries, never reserialize."""
    V.parse(raw)
    text = raw.decode('utf-8'); decoder = json.JSONDecoder(); i = 0; result = None
    def space(at):
        while at < len(text) and text[at] in ' \t\r\n': at += 1
        return at
    i = space(i); V.require(i < len(text) and text[i] == '{', 'top-level object required'); i += 1
    while True:
        i = space(i)
        if i < len(text) and text[i] == '}': break
        key, i = decoder.raw_decode(text, i); i = space(i)
        V.require(type(key) is str and i < len(text) and text[i] == ':', 'JSON object member')
        start = space(i + 1); _, i = decoder.raw_decode(text, start)
        if key == selected: result = text[start:i].encode('utf-8')
        i = space(i)
        V.require(i < len(text) and text[i] in ',}', 'JSON object delimiter')
        if text[i] == '}': break
        i += 1
    V.require(result is not None, 'required exact JSON member')
    return result


def report(raw, observed):
    return _report(raw, observed, False)


def shared_report(raw, observed):
    V.require(type(raw) is bytes and 0 < len(raw) <= MAX_BYTES, 'shared envelope64KiB')
    envelope = V.parse(raw)
    V.keys(envelope, 'schema policy configuration_host_ns observation')
    V.require(envelope['schema'] == 'FerricProjectionResidualDecodeSharedHostEnvelopeV1'
        and envelope['policy'] == 'shared-full', 'explicit shared-full envelope')
    configuration = V.uint(envelope['configuration_host_ns'])
    value = _report(member(raw, 'observation'), observed, True)
    return value, configuration


def _report(raw, observed, shared, ordered=False):
    V.require(type(raw) is bytes and 0 < len(raw) <= MAX_BYTES, 'host sidecar64KiB')
    value = V.parse(raw)
    V.keys(value, 'schema bootstrap worker_sha256 child_pid profile_sha256 snapshots intervals forward_host_ns '
        'close_host_ns serialization_host_ns completions transcript_sha256 native_closed inclusive_nested_host_scopes '
        'gpu_time numerical_acceptance performance_claim production_authority')
    V.require(value['schema'] == ('FerricProjectionResidualMlpOrderedHostObservationV1' if ordered else
        'FerricProjectionResidualDecodeSharedHostObservationV1' if shared else
        'FerricProjectionResidualDecodeHostObservationV1')
        and observed['request']['decode']['mode'] == 'autoregressive'
        and value['native_closed'] is True and value['inclusive_nested_host_scopes'] is True
        and all(value[k] is False for k in FALSE), 'closed host diagnostic scope')
    V.require(0 < V.uint(value['child_pid'], 0xffffffff) == observed['child_pid']
        and same(value['bootstrap'], observed['bootstrap'])
        and V.digest(value['worker_sha256']) == V.digest(observed['request']['decode']['worker']['sha256']) != bytes(32)
        and V.digest(value['profile_sha256']) == V.digest(observed['profile_sha256'])
        and V.digest(value['transcript_sha256']) == V.digest(observed['transcript_sha256']),
        'host report matches actual validated Four identity')
    V.require(type(value['completions']) is list and len(value['completions']) == 4,
              'four actual completion bindings')
    for actual, frame in zip(value['completions'], observed['files']['frames']):
        expected = dict(frame['response']['event']); V.require(expected.pop('status') == 'completed', 'actual completion')
        V.require(same(actual, expected), 'host report actual control/payload/transcript binding')
    snapshots, intervals = value['snapshots'], value['intervals']
    V.require(type(snapshots) is list and len(snapshots) == 7 and type(intervals) is list and len(intervals) == 6,
              'seven snapshots and six checked deltas')
    V.array(value['forward_host_ns'], 4, U64); V.uint(value['close_host_ns'])
    V.array(value['serialization_host_ns'], 4, U64)
    devices = observed['request']['decode']['device_ids']
    V.require(len(devices) == 2 and devices[0] != devices[1] and all(V.uint(d) > 0 for d in devices), 'two devices')
    for index, current in enumerate(snapshots):
        V.keys(current, 'phase group_incarnation shared_full_currentness ranks shared')
        V.require(current['phase'] == PHASES[index] and V.uint(current['group_incarnation']) > 0
            and current['group_incarnation'] == snapshots[0]['group_incarnation']
            and current['shared_full_currentness'] is shared, 'phase/group/exact selected full policy')
        V.array(current['shared'], 4, U64)
        V.require(type(current['ranks']) is list and len(current['ranks']) == 2, 'two ranked snapshots')
        for rank, row in enumerate(current['ranks']):
            V.keys(row, 'rank unique_id queue_epoch cache_kernel_admission raw_timestamp_queue counters')
            V.require(V.uint(row['rank'], 1) == rank and V.uint(row['unique_id']) == devices[rank]
                and V.uint(row['queue_epoch']) == snapshots[0]['ranks'][rank]['queue_epoch']
                and row['cache_kernel_admission'] is False and row['raw_timestamp_queue'] is False,
                'stable ranked queue and no cached/raw policy')
            V.array(row['counters'], 19, U64)
            V.require(row['counters'][4:6] == [0, 0], 'operational currentness remains off')
        if index == 0:
            V.require(current['shared'] == [0] * 4
                and all(row['counters'] == [0] * 19 for row in current['ranks']), 'fresh enabled zero baseline')
            continue
        delta = intervals[index - 1]; old = snapshots[index - 1]
        V.keys(delta, 'host_elapsed_ns ranks shared'); V.uint(delta['host_elapsed_ns'])
        V.array(delta['shared'], 4, U64)
        V.require(type(delta['ranks']) is list and len(delta['ranks']) == 2, 'two ranked deltas')
        V.require(delta['shared'] == [n - p for n, p in zip(current['shared'], old['shared'])], 'shared monotone delta')
        for rank in range(2):
            V.array(delta['ranks'][rank], 19, U64)
            V.require(delta['ranks'][rank] == [n - p for n, p in
                zip(current['ranks'][rank]['counters'], old['ranks'][rank]['counters'])], 'rank monotone delta')
        if 2 <= index <= 5:
            V.require(value['forward_host_ns'][index - 2] <= delta['host_elapsed_ns'], 'forward wall interval bound')
    for index, elapsed in enumerate(value['serialization_host_ns']):
        V.require(elapsed <= intervals[index + 2]['host_elapsed_ns'], 'serialization following interval bound')
    return value


def validate(stdout, sidecar_raw, sidecar_pin, summary_raw, observed, parent):
    return _validate(stdout, sidecar_raw, sidecar_pin, summary_raw, observed, parent, False)


def validate_shared(stdout, sidecar_raw, sidecar_pin, summary_raw, observed, parent):
    return _validate(stdout, sidecar_raw, sidecar_pin, summary_raw, observed, parent, True)


def _validate(stdout, sidecar_raw, sidecar_pin, summary_raw, observed, parent, shared, ordered=False):
    V.require(type(stdout) is bytes and 0 < len(stdout) <= MAX_BYTES and stdout.endswith(b'\n')
        and not stdout.endswith(b'\n\n'), 'bounded single diagnostic wrapper stdout')
    wrapper = V.parse(stdout)
    V.keys(wrapper, 'schema parent_binary request_projection_sha256 observation host_sidecar '
        'inclusive_nested_host_scopes gpu_time performance_claim numerical_acceptance production_authority')
    V.require(wrapper['schema'] == ('FerricFiniteProjectionResidualMlpOrderedHostDiagnosticV1' if ordered else
        'FerricFiniteProjectionResidualDecodeSharedHostDiagnosticV1' if shared else
        'FerricFiniteProjectionResidualDecodeHostDiagnosticV1')
        and wrapper['inclusive_nested_host_scopes'] is True and all(wrapper[k] is False for k in FALSE),
        'explicit host wrapper scope')
    V.require(pin(wrapper['parent_binary']) == pin(parent)
        and pin(wrapper['host_sidecar']) == pin(sidecar_pin), 'actual diagnostic parent and sidecar pins')
    expected_path = Path(observed['request']['decode']['evidence_directory'])
    expected_path = expected_path.with_name(expected_path.name + ('-projection-residual-mlp-ordered-observation.json' if ordered else '-projection-shared-host-observation.json' if shared else '-projection-host-observation.json'))
    V.require(pin(sidecar_pin)['path'] == str(expected_path), 'separate exact host sidecar path')
    V.require(pin(sidecar_pin)['bytes'] == len(sidecar_raw)
        and pin(sidecar_pin)['sha256'] == hashlib.sha256(sidecar_raw).hexdigest(), 'actual sidecar bytes')
    V.require(member(stdout, 'observation') + b'\n' == summary_raw
        and same(wrapper['observation'], observed), 'exact nested original Four summary bytes')
    V.require(V.digest(wrapper['request_projection_sha256']) ==
        hashlib.sha256(member(summary_raw, 'request')).digest(), 'actual serde Config projection, not raw request file')
    value, configuration = (ordered_report(sidecar_raw, observed) if ordered else
        shared_report(sidecar_raw, observed) if shared else (report(sidecar_raw, observed), None))
    V.require(observed['files']['bytes_before_summary'] + len(sidecar_raw) + MAX_BYTES <= 8 << 20,
              'unchanged combined native8MiB bound including reserved summary')
    result = dict(schema=('ferric-p228-projection-ordered-segment-host-counters-v1' if ordered else
        'ferric-p228-projection-ar4-shared-host-counters-v1' if shared else
        'ferric-p228-projection-ar4-host-counters-v1'), sidecar=pin(sidecar_pin),
        counter_names=list(COUNTERS), shared_counter_names=list(SHARED), snapshots=value['snapshots'],
        intervals=value['intervals'], forward_host_ns=value['forward_host_ns'], close_host_ns=value['close_host_ns'],
        serialization_host_ns=value['serialization_host_ns'],
        native_closed=True, inclusive_nested_host_scopes=True, gpu_time=False, calibrated_device_time=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
    if shared:
        result.update(policy='ordered-shared-full' if ordered else 'shared-full', configuration_host_ns=configuration,
                      configuration_time_in_snapshots=False, fresh_full_currentness_preserved=True)
    if ordered:
        result.update(intermediate_host_fence_removed=True, original_per_kernel_policy_unchanged=False,
            dispatch_packet_count_preserved=True, per_kernel_publish_wait_poll_comparable=False,
            ordered_wait_spans_overlap=True, segment_host_ns_in_control=True)
    return result

def ordered_report(raw, observed):
    V.require(type(raw) is bytes and 0 < len(raw) <= MAX_BYTES, 'ordered envelope64KiB')
    envelope = V.parse(raw)
    V.keys(envelope, 'schema policy configuration_host_ns observation')
    V.require(envelope['schema'] == 'FerricProjectionResidualMlpOrderedHostEnvelopeV1'
        and envelope['policy'] == 'ordered-shared-full', 'explicit ordered shared-full envelope')
    V.require(observed['schema'] == 'FerricFiniteProjectionResidualMlpOrderedObservationV1'
        and observed['request']['schema'] == 'FerricFiniteProjectionResidualMlpOrderedRequestV1'
        and observed['bootstrap']['schema'] == 'FerricProjectionResidualMlpOrderedBootstrapV1',
        'ordered report requires validated ordered summary')
    return _report(member(raw, 'observation'), observed, True, True), V.uint(envelope['configuration_host_ns'])


def validate_ordered(stdout, sidecar_raw, sidecar_pin, summary_raw, observed, parent):
    return _validate(stdout, sidecar_raw, sidecar_pin, summary_raw, observed, parent, True, True)
