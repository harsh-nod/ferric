"""Closed clock V2 report plus unchanged raw-Control and full-output joins."""
import hashlib
from pathlib import Path

import layer_validation as V
import host_validation as H
import raw_validation as RV

MAX_BYTES, PER_FORWARD, MAX_ROWS, RANK_PACKETS = RV.MAX_BYTES, RV.PER_FORWARD, RV.MAX_ROWS, RV.RANK_PACKETS
SAMPLE_COUNT = 16
REQUEST_SCHEMA = 'FerricFinitePrefixDecodeDeviceClockRequestV2'
OBSERVATION_SCHEMA = 'ferric-p228-device-clock-observation-v1'
FALSE = ('clock_domain_validated', *RV.FALSE)
ENTRIES, expected, control_times, invariance = RV.ENTRIES, RV.expected, RV.control_times, RV.invariance


def request(value):
    V.keys(value, 'schema decode')
    V.require(value['schema'] == REQUEST_SCHEMA and type(value['decode']) is dict,
              'closed explicit device request')
    return value['decode']


def report(raw, observed, files, validator):
    V.require(type(raw) is bytes and 0 < len(raw) <= MAX_BYTES, 'clock sidecar 2 MiB bound')
    value = V.parse(raw)
    V.keys(value, 'schema raw samples raw_clock_counters ' + ' '.join(FALSE))
    V.require(value['schema'] == 'FerricPrefixDecodeDeviceClockObservationV2'
        and value['raw_clock_counters'] is True and all(value[key] is False for key in FALSE),
        'closed raw-clock-only schema and claims')
    # Reuse all raw-report checks directly on the authenticated embedded object.
    # Do not create a legacy receipt, change its bytes or relax its policy.
    RV.report_value(value['raw'], observed, files, validator)
    samples, raw_report = value['samples'], value['raw']
    V.require(type(samples) is list and len(samples) == SAMPLE_COUNT, 'exact sixteen clock samples')
    previous_finished = 0
    for index, sample in enumerate(samples):
        V.keys(sample, 'generation position endpoint rank row_boundary group_incarnation unique_id '
            'queue_epoch gpu_id gpu_clock_counter cpu_clock_counter system_clock_counter '
            'system_clock_frequency_hz host_started_ns host_finished_ns')
        position, slot = divmod(index, 4)
        rank, post = slot % 2, slot >= 2
        V.require(V.uint(sample['generation']) == position + 1
            and V.uint(sample['position'], 3) == position
            and sample['endpoint'] == ('post' if post else 'pre')
            and V.uint(sample['rank'], 1) == rank
            and V.uint(sample['row_boundary'], MAX_ROWS) == (position + int(post)) * PER_FORWARD,
            'exact forward/pre-post/rank clock sample order')
        V.require(V.uint(sample['group_incarnation']) == raw_report['group_incarnation']
            and V.uint(sample['unique_id']) == raw_report['ranks'][rank]['unique_id']
            and V.uint(sample['queue_epoch']) == raw_report['ranks'][rank]['queue_epoch'],
            'clock sample actual native group and queue identity')
        gpu_id = V.uint(sample['gpu_id'], (1 << 32) - 1)
        frequency = V.uint(sample['system_clock_frequency_hz'])
        started, finished = V.uint(sample['host_started_ns']), V.uint(sample['host_finished_ns'])
        V.require(frequency > 0 and previous_finished <= started <= finished,
                  'positive system frequency and ordered host sampling brackets')
        for key in ('gpu_clock_counter', 'cpu_clock_counter', 'system_clock_counter'):
            V.uint(sample[key])
        if index == 1:
            V.require(gpu_id != samples[0]['gpu_id'], 'distinct selected KFD GPU IDs')
        if index >= 2:
            V.require(gpu_id == samples[rank]['gpu_id']
                and frequency == samples[rank]['system_clock_frequency_hz'],
                'stable same-device KFD GPU ID and system clock frequency')
        previous_finished = finished
    # Raw zero or decreasing counters are not an error or a clock-domain proof.
    return value


def validate(C, stdout, sidecar, sidecar_pin, summary, observed, parent, external, files, validator):
    V.require(H.same(request(external), observed['request']), 'device request exact decode projection')
    V.require(type(stdout) is bytes and 0 < len(stdout) <= 65536 and stdout.endswith(b'\n')
        and not stdout.endswith(b'\n\n'), 'bounded single parent diagnostic JSON')
    wrapper = V.parse(stdout)
    V.keys(wrapper, 'schema parent_binary request_projection_sha256 observation device_sidecar raw_completion_ticks raw_clock_counters '
        + ' '.join(FALSE))
    V.require(wrapper['schema'] == 'FerricFinitePrefixDecodeDeviceClockDiagnosticV2'
        and wrapper['raw_completion_ticks'] is True and wrapper['raw_clock_counters'] is True and all(wrapper[k] is False for k in FALSE),
        'raw parent wrapper grants no calibrated timing or numerical authority')
    V.require(C.pin(wrapper['parent_binary']) == C.pin(parent)
        and C.pin(wrapper['device_sidecar']) == C.pin(sidecar_pin), 'actual parent and sidecar pins')
    directory = Path(observed['request']['evidence_directory'])
    V.require(C.pin(sidecar_pin)['path'] == str(directory.with_name(directory.name + '-device-clock-v2.json')),
              'case-contained raw sidecar')
    V.require(H.member(stdout, 'observation') + b'\n' == summary
        and H.same(wrapper['observation'], observed), 'exact nested closed native summary')
    projection = (b'{"schema":"FerricFinitePrefixDecodeDeviceClockRequestV2","decode":'
                  + H.member(summary, 'request') + b'}')
    V.require(V.digest(wrapper['request_projection_sha256']) == hashlib.sha256(projection).digest(),
              'exact serde Device request projection')
    V.require(observed['files']['bytes_before_summary'] + len(sidecar) + 65536 <= 8 << 20,
              'unchanged aggregate native/sidecar/summary 8 MiB cap')
    return report(sidecar, observed, files, validator)


def candidate(C, read, value, helpers):
    C.keys(value, 'native_files device_sidecar request parent owner command started stdout stderr')
    validator = helpers['decode_validation']
    raw, files, observed = C.native_files(read, value['native_files'], validator)
    structural = validator.validate(raw, files)
    external = C.doc(read, value['request'], 65536)
    C.require(H.same(request(external), observed['request']), 'external actual Device request')
    C.get(read, value['parent'], 128 << 20)
    owner = C.doc(read, value['owner']); C.clean_owner(owner)
    C.require(owner['gpu_execution_requested'] is True, 'actual owned native leaf')
    for key in ('command', 'started', 'stdout', 'stderr'):
        C.require(C.pin(owner[key]) == C.pin(value[key]), 'owned native raw record join')
    command = C.doc(read, value['command']); start = C.doc(read, value['started'])
    C.keys(command, 'argv env cwd deadline_seconds affinity nice address_space_bytes file_cap_bytes '
        'stream_cap_bytes gpu_execution_requested')
    C.require(command['argv'] == [C.pin(value['parent'])['path'], '--request', C.pin(value['request'])['path'],
        '--allow-unauthenticated-machine-code', '--observe-device-clocks']
        and command['cwd'] == C.ROOT and command['deadline_seconds'] == 4000
        and command['address_space_bytes'] == 32 << 30 and command['file_cap_bytes'] == 64 << 20
        and command['stream_cap_bytes'] == 8 << 20 and command['affinity'] == [8, 9]
        and command['nice'] == 10 and command['gpu_execution_requested'] is True,
        'unchanged owned envelope with exact device selector')
    C.require(start['command_sha256'] == C.pin(value['command'])['sha256'], 'owned command identity')
    sidecar = C.get(read, value['device_sidecar'], MAX_BYTES)
    C.require(sum(C.pin(pin)['bytes'] for pin in value['native_files'].values()) + len(sidecar) <= 8 << 20,
              'native and device sidecar aggregate cap')
    device = validate(C, C.get(read, value['stdout'], 65536), sidecar, value['device_sidecar'], raw,
                      observed, value['parent'], external, files, validator)
    C.parent_stderr(C.get(read, value['stderr']), observed['child_pid'], observed['request']['mode'])
    return observed, files, structural, C.lineage(owner, start, observed['child_pid']), device


def observe(c, records, sidecar, leaf, value):
    read = lambda pin, maximum: c['read'](c['pins'], pin, maximum)
    command = c['C'].doc(read, value['command'])
    V.require(H.same(command['env'], c['environment']), 'actual exact reviewed native environment')
    item = dict(native_files=records, device_sidecar=sidecar, request=c['plan']['request'],
        parent=c['runtime']['parent'], owner=leaf,
        **{key: value[key] for key in ('command', 'started', 'stdout', 'stderr')})
    observed, files, structural, ownership, device = candidate(c['C'], read, item, c['H'])
    rows = invariance(c['C'], c['baseline'], observed, files, c['H'])
    return dict(schema=OBSERVATION_SCHEMA, status='RAW_CLOCKS_AND_EXACT_OUTPUT_INVARIANCE_OBSERVED',
        request=c['plan']['request'], mode='teacher_forced', parent=c['runtime']['parent'],
        worker=c['runtime']['worker'], prefix_image=c['runtime']['image'], device_sidecar=sidecar,
        baseline=c['baseline']['receipt_pin'], structural=structural, owned_record_checks=ownership,
        raw_rows=len(device['raw']['rows']), rank_packets=device['raw']['final_dispatches'],
        clock_samples=len(device['samples']), captured_payloads=4,
        compared_tensor_rows=152, all_payloads_tokens_and_tensors_equal=True, tensor_comparison=rows,
        recorded_close_and_owner_reap_checked=True, raw_completion_ticks=True, raw_clock_counters=True,
        **{key: False for key in FALSE})
