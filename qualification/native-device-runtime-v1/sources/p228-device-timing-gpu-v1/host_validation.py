"""V2 single-factor policies; native Four validation and all host limits stay closed."""
import hashlib
import json
from pathlib import Path

import layer_validation as V

MAX_BYTES = 65536
U64 = (1 << 64) - 1
PHASES = ('fresh_enabled', 'setup_sealed', 'forward_0', 'forward_1', 'forward_2', 'forward_3', 'before_close')
COUNTERS = ('commands', 'command_ns', 'full_currentness_checks', 'full_currentness_ns',
    'operational_currentness_checks', 'operational_currentness_ns', 'kernel_admissions', 'kernel_admission_ns',
    'dispatches', 'dispatch_prepare_ns', 'dispatch_publish_ns', 'dispatch_wait_ns', 'completion_polls',
    'reads', 'read_bytes', 'read_ns', 'writes', 'write_bytes', 'write_ns')
SHARED = ('group_full_checks', 'group_full_ns', 'publication_full_checks', 'publication_full_ns')
FALSE = ('gpu_time', 'numerical_acceptance', 'performance_claim', 'production_authority')
POLICIES = {'baseline': (False, False), 'immutable-admission-cache': (True, False),
            'shared-full-currentness': (False, True)}
REQUEST_SCHEMA = 'FerricFinitePrefixDecodeHostPolicyRequestV2'


def request(value):
    V.keys(value, 'schema policy decode')
    V.require(value['schema'] == REQUEST_SCHEMA and type(value['policy']) is str
        and value['policy'] in POLICIES and type(value['decode']) is dict,
        'closed required V2 single-factor request')
    return value['decode'], value['policy']


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


def report(raw, observed, policy):
    V.require(type(policy) is str and policy in POLICIES, 'required requested policy')
    cache, shared = POLICIES[policy]
    V.require(type(raw) is bytes and 0 < len(raw) <= MAX_BYTES, 'host sidecar64KiB')
    value = V.parse(raw)
    V.keys(value, 'schema policy bootstrap worker_sha256 child_pid profile_sha256 snapshots intervals forward_host_ns '
        'close_host_ns completions transcript_sha256 native_closed inclusive_nested_host_scopes '
        'gpu_time numerical_acceptance performance_claim production_authority')
    V.require(value['schema'] == 'FerricPrefixDecodeHostObservationV2' and value['policy'] == policy
        and value['native_closed'] is True and value['inclusive_nested_host_scopes'] is True
        and all(value[k] is False for k in FALSE), 'closed host diagnostic scope')
    V.require(0 < V.uint(value['child_pid'], 0xffffffff) == observed['child_pid']
        and same(value['bootstrap'], observed['bootstrap'])
        and V.digest(value['worker_sha256']) == V.digest(observed['request']['worker']['sha256']) != bytes(32)
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
    devices = observed['request']['device_ids']
    V.require(len(devices) == 2 and devices[0] != devices[1] and all(V.uint(d) > 0 for d in devices), 'two devices')
    for index, current in enumerate(snapshots):
        V.keys(current, 'phase group_incarnation shared_full_currentness ranks shared')
        V.require(current['phase'] == PHASES[index] and V.uint(current['group_incarnation']) > 0
            and current['group_incarnation'] == snapshots[0]['group_incarnation']
            and current['shared_full_currentness'] is shared, 'phase/group/exact requested shared policy')
        V.array(current['shared'], 4, U64)
        V.require(type(current['ranks']) is list and len(current['ranks']) == 2, 'two ranked snapshots')
        for rank, row in enumerate(current['ranks']):
            V.keys(row, 'rank unique_id queue_epoch cache_kernel_admission raw_timestamp_queue counters')
            V.require(V.uint(row['rank'], 1) == rank and V.uint(row['unique_id']) == devices[rank]
                and V.uint(row['queue_epoch']) == snapshots[0]['ranks'][rank]['queue_epoch']
                and row['cache_kernel_admission'] is cache and row['raw_timestamp_queue'] is False,
                'stable ranked queue, exact requested cache policy and raw timing off')
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
    return value


def validate(C, stdout, sidecar_raw, sidecar_pin, summary_raw, observed, parent, external_request):
    decode, policy = request(external_request)
    V.require(same(decode, observed['request']), 'V2 request embeds exact validated Four config')
    V.require(type(stdout) is bytes and 0 < len(stdout) <= MAX_BYTES and stdout.endswith(b'\n')
        and not stdout.endswith(b'\n\n'), 'bounded single diagnostic wrapper stdout')
    wrapper = V.parse(stdout)
    V.keys(wrapper, 'schema policy parent_binary request_projection_sha256 observation host_sidecar '
        'inclusive_nested_host_scopes gpu_time performance_claim numerical_acceptance production_authority')
    V.require(wrapper['schema'] == 'FerricFinitePrefixDecodeHostDiagnosticV2' and wrapper['policy'] == policy
        and wrapper['inclusive_nested_host_scopes'] is True and all(wrapper[k] is False for k in FALSE),
        'explicit host wrapper scope')
    V.require(C.pin(wrapper['parent_binary']) == C.pin(parent)
        and C.pin(wrapper['host_sidecar']) == C.pin(sidecar_pin), 'actual diagnostic parent and sidecar pins')
    expected_path = Path(observed['request']['evidence_directory'])
    expected_path = expected_path.with_name(expected_path.name + '-host-policy-v2.json')
    V.require(C.pin(sidecar_pin)['path'] == str(expected_path), 'separate exact host sidecar path')
    V.require(member(stdout, 'observation') + b'\n' == summary_raw
        and same(wrapper['observation'], observed), 'exact nested original Four summary bytes')
    # Request's Rust struct field order is schema, policy, decode. Preserve the
    # exact already-emitted Config bytes, including serde string escaping.
    projection = (b'{"schema":"FerricFinitePrefixDecodeHostPolicyRequestV2","policy":'
                  + json.dumps(policy).encode('ascii') + b',"decode":'
                  + member(summary_raw, 'request') + b'}')
    V.require(V.digest(wrapper['request_projection_sha256']) == hashlib.sha256(projection).digest(),
              'actual complete serde V2 Request projection, not V1 Config or raw request formatting')
    value = report(sidecar_raw, observed, policy)
    V.require(observed['files']['bytes_before_summary'] + len(sidecar_raw) + MAX_BYTES <= 8 << 20,
              'unchanged combined native8MiB bound including reserved summary')
    return dict(schema='ferric-p228-prefix-decode-host-policy-counters-v2', policy=policy, sidecar=C.pin(sidecar_pin),
        counter_names=list(COUNTERS), shared_counter_names=list(SHARED), snapshots=value['snapshots'],
        intervals=value['intervals'], forward_host_ns=value['forward_host_ns'], close_host_ns=value['close_host_ns'],
        native_closed=True, inclusive_nested_host_scopes=True, gpu_time=False, calibrated_device_time=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
