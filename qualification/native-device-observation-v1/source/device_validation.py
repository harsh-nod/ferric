"""Retained raw ticks plus exact output invariance; no native calls or clock scale."""
import hashlib
import struct
from pathlib import Path

import layer_validation as V
import host_validation as H

MAX_BYTES = 2 << 20
PER_FORWARD, MAX_ROWS, RANK_PACKETS = 293, 1172, [592, 580]
REQUEST_SCHEMA = 'FerricFinitePrefixDecodeDeviceRequestV1'
OBSERVATION_SCHEMA = 'ferric-p228-device-timing-observation-v1'
FALSE = ('calibrated_nanoseconds', 'cross_device_clock_alignment', 'overlap_claim',
         'performance_claim', 'numerical_acceptance', 'full_model_acceptance', 'production_authority')
ENTRIES = {
    'embedding': 'ferric_qwen3_tp_batch_embedding_bf16_v2',
    'copy': 'ferric_qwen3_tp_peer_copy_bf16_v4',
    'prefix': 'ferric_qwen3_claimed_rmsnorm_qkv_attention_output_tiles_bf16_f32_v6',
    'post-attention-residual': 'ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18',
    'mlp': 'ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2',
    'post-mlp-residual': 'ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18',
    'final-norm': 'qwen3_rmsnorm_v1',
    'head': 'ferric_qwen3_tp_mfma_gemm_bf16_v3',
    'argmax': 'ferric_qwen3_tp_batch_argmax_bf16_v2',
}


def request(value):
    V.keys(value, 'schema decode')
    V.require(value['schema'] == REQUEST_SCHEMA and type(value['decode']) is dict,
              'closed explicit device request')
    return value['decode']


def expected(index):
    V.require(type(index) is int and 0 <= index < MAX_ROWS, 'bounded raw packet index')
    position, slot = divmod(index, PER_FORWARD)
    if slot == 0: stage, layer, rank = 'embedding', None, 0
    elif slot == 1: stage, layer, rank = 'copy', None, 1
    elif slot < 290:
        layer, local = divmod(slot - 2, 8)
        stage = ('prefix', 'post-attention-residual', 'mlp', 'post-mlp-residual')[local // 2]
        rank = local % 2
    else: stage, layer, rank = ('final-norm', 'head', 'argmax')[slot - 290], None, 0
    return position + 1, position, stage, layer, rank


def control_times(raw, validator):
    # The frozen validator authenticates all terminal words before we extract
    # timing fields using the unchanged wire layout, not guessed JSON offsets.
    V.require(validator.control(raw) == 144, 'all typed prefix/MLP states validated')
    times = list(struct.unpack_from('<2Q', raw))
    offset = 16
    for _ in range(36):
        offset += 2 * (284 + 548) * 4
        times.extend(struct.unpack_from('<8Q', raw, offset)); offset += 64
    times.extend(struct.unpack_from('<3Q', raw, offset)); offset += 24
    V.require(offset == len(raw) == 241960 and len(times) == PER_FORWARD,
              'exact raw Control timing census')
    return times


def report(raw, observed, files, validator):
    V.require(type(raw) is bytes and 0 < len(raw) <= MAX_BYTES, 'raw sidecar 2 MiB bound')
    value = V.parse(raw)
    V.keys(value, 'schema bootstrap worker_sha256 child_pid profile_sha256 group_incarnation ranks images rows '
        'final_dispatches completions transcript_sha256 raw_timestamp_queue shared_full_currentness '
        'cache_kernel_admission operational_currentness native_closed raw_completion_ticks ' + ' '.join(FALSE))
    V.require(value['schema'] == 'FerricPrefixDecodeDeviceObservationV1'
        and all(value[k] is True for k in ('raw_timestamp_queue', 'shared_full_currentness',
                                         'native_closed', 'raw_completion_ticks'))
        and value['cache_kernel_admission'] is False and value['operational_currentness'] is False
        and all(value[k] is False for k in FALSE), 'closed raw-only policy and claims')
    V.require(0 < V.uint(value['child_pid'], 0xffffffff) == observed['child_pid']
        and H.same(value['bootstrap'], observed['bootstrap'])
        and V.digest(value['worker_sha256']) == V.digest(observed['request']['worker']['sha256']) != bytes(32)
        and V.digest(value['profile_sha256']) == V.digest(observed['profile_sha256'])
        and V.digest(value['transcript_sha256']) == V.digest(observed['transcript_sha256']),
        'raw report bound to actual closed native identity')
    group = V.uint(value['group_incarnation']); V.require(group > 0, 'actual raw group incarnation')
    ranks = value['ranks']
    V.require(type(ranks) is list and len(ranks) == 2, 'two ranked raw queues')
    for rank, item in enumerate(ranks):
        V.keys(item, 'rank unique_id queue_epoch')
        V.require(V.uint(item['rank'], 1) == rank
            and V.uint(item['unique_id']) == observed['request']['device_ids'][rank], 'actual ranked device')
        V.uint(item['queue_epoch'])
    images = value['images']; V.keys(images, 'prefix mlp residual tail copy')
    bootstrap = observed['bootstrap']; begin = bootstrap['begin']
    image_parts = dict(prefix=bootstrap['prefix_image'], mlp=bootstrap['tiles_image'],
        residual=begin['residual_image'], tail=begin['tail_image'], copy=begin['residual_image'])
    for name, part in image_parts.items():
        V.require(type(part) is dict and V.digest(images[name]) == V.digest(part['sha256']) != bytes(32),
                  'actual selected code-object identity: ' + name)
    rows = value['rows']; completions = value['completions']
    V.require(type(rows) is list and len(rows) == MAX_ROWS
        and type(completions) is list and len(completions) == 4, 'complete four-forward raw census')
    V.array(value['final_dispatches'], 2, H.U64)
    V.require(value['final_dispatches'] == RANK_PACKETS, 'actual final dispatch counter census')
    packets = [0, 0]
    timings = []
    for position, frame in enumerate(observed['files']['frames']):
        completion = dict(frame['response']['event'])
        V.require(completion.pop('status') == 'completed' and H.same(completions[position], completion),
                  'raw report actual completion/control/capture/chain')
        timings.extend(control_times(files[f'control-{position}.bin'], validator))
    V.require(len(timings) == MAX_ROWS, 'all actual per-packet host intervals')
    for index, row in enumerate(rows):
        V.keys(row, 'generation position stage layer rank entry image_sha256 group_incarnation unique_id '
            'queue_epoch packet_id signal_generation start_tick end_tick host_elapsed_ns')
        generation, position, stage, layer, rank = expected(index)
        V.require(V.uint(row['generation']) == generation and V.uint(row['position'], 3) == position
            and row['stage'] == stage and V.uint(row['rank'], 1) == rank
            and ((row['layer'] is None) if layer is None else V.uint(row['layer'], 35) == layer)
            and row['entry'] == ENTRIES[stage], 'exact actual raw stage/layer/rank order')
        image = ('residual' if stage.endswith('-residual') else stage if stage in ('prefix', 'mlp', 'copy') else 'tail')
        V.require(V.digest(row['image_sha256']) == V.digest(images[image])
            and V.uint(row['group_incarnation']) == group
            and V.uint(row['unique_id']) == ranks[rank]['unique_id']
            and V.uint(row['queue_epoch']) == ranks[rank]['queue_epoch']
            and V.uint(row['packet_id']) == packets[rank]
            and V.uint(row['signal_generation']) == packets[rank] + 1,
            'actual selected kernel and contiguous packet/signal identity')
        start, end = V.uint(row['start_tick']), V.uint(row['end_tick'])
        V.require(0 < start <= end and V.uint(row['host_elapsed_ns']) == timings[index],
                  'raw ticks and actual retained Control interval')
        packets[rank] += 1
    V.require(packets == RANK_PACKETS, 'all actual raw dispatches accounted for')
    return value


def validate(C, stdout, sidecar, sidecar_pin, summary, observed, parent, external, files, validator):
    V.require(H.same(request(external), observed['request']), 'device request exact decode projection')
    V.require(type(stdout) is bytes and 0 < len(stdout) <= 65536 and stdout.endswith(b'\n')
        and not stdout.endswith(b'\n\n'), 'bounded single parent diagnostic JSON')
    wrapper = V.parse(stdout)
    V.keys(wrapper, 'schema parent_binary request_projection_sha256 observation device_sidecar raw_completion_ticks '
        + ' '.join(FALSE))
    V.require(wrapper['schema'] == 'FerricFinitePrefixDecodeDeviceDiagnosticV1'
        and wrapper['raw_completion_ticks'] is True and all(wrapper[k] is False for k in FALSE),
        'raw parent wrapper grants no calibrated timing or numerical authority')
    V.require(C.pin(wrapper['parent_binary']) == C.pin(parent)
        and C.pin(wrapper['device_sidecar']) == C.pin(sidecar_pin), 'actual parent and sidecar pins')
    directory = Path(observed['request']['evidence_directory'])
    V.require(C.pin(sidecar_pin)['path'] == str(directory.with_name(directory.name + '-device-v1.json')),
              'case-contained raw sidecar')
    V.require(H.member(stdout, 'observation') + b'\n' == summary
        and H.same(wrapper['observation'], observed), 'exact nested closed native summary')
    projection = (b'{"schema":"FerricFinitePrefixDecodeDeviceRequestV1","decode":'
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
        '--allow-unauthenticated-machine-code', '--observe-device-ticks']
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


def invariance(C, baseline, observed, files, helpers):
    old = baseline['observed']; prior = baseline['files']
    omitted = {'worker', 'session', 'evidence_directory'}
    left, right = old['request'], observed['request']
    V.require(set(left) == set(right) and omitted <= set(left)
        and H.same({k: v for k, v in left.items() if k not in omitted},
                   {k: v for k, v in right.items() if k not in omitted})
        and left['session'] != right['session'], 'same complete TF4 workload with fresh session')
    V.require(left['mode'] == right['mode'] == 'teacher_forced', 'fixed actual TF4 timing experiment')
    for position in range(4):
        a = C.document(prior[f'request-{position}.json'])
        b = C.document(files[f'request-{position}.json'])
        V.require(H.same(a['command'], b['command']) and a['device_ids'] == b['device_ids']
            and a['id'] == b['id'] and a['protocol'] == b['protocol'], 'identical full native forward input')
    old_records, new_records = C.records(old), C.records(observed)
    V.require(len(old_records) == len(new_records) == 4, 'four complete token trajectories')
    rows = C.compare_rows(old_records, [prior[f'observation-{i}.bin'] for i in range(4)],
        new_records, [files[f'observation-{i}.bin'] for i in range(4)], helpers['diagnostics'])
    V.require(len(rows) == 4 and sum(len(row['tensors']) for row in rows) == 152,
              'all 152 typed tensor comparisons')
    V.require(H.same(old_records, new_records)
        and all(len(files[f'observation-{i}.bin']) == len(prior[f'observation-{i}.bin']) == 606976
                and files[f'observation-{i}.bin'] == prior[f'observation-{i}.bin'] for i in range(4))
        and all(row['same_input_history'] and all(t['byte_equal'] is True for t in row['tensors']) for row in rows),
        'raw instrumentation must preserve all four payloads, tokens and 152 tensors')
    return rows


def observe(c, records, sidecar, leaf, value):
    read = lambda pin, maximum: c['read'](c['pins'], pin, maximum)
    command = c['C'].doc(read, value['command'])
    V.require(H.same(command['env'], c['environment']), 'actual exact reviewed native environment')
    item = dict(native_files=records, device_sidecar=sidecar, request=c['plan']['request'],
        parent=c['runtime']['parent'], owner=leaf,
        **{key: value[key] for key in ('command', 'started', 'stdout', 'stderr')})
    observed, files, structural, ownership, device = candidate(c['C'], read, item, c['H'])
    rows = invariance(c['C'], c['baseline'], observed, files, c['H'])
    return dict(schema=OBSERVATION_SCHEMA, status='RAW_TICKS_AND_EXACT_OUTPUT_INVARIANCE_OBSERVED',
        request=c['plan']['request'], mode='teacher_forced', parent=c['runtime']['parent'],
        worker=c['runtime']['worker'], prefix_image=c['runtime']['image'], device_sidecar=sidecar,
        baseline=c['baseline']['receipt_pin'], structural=structural, owned_record_checks=ownership,
        raw_rows=len(device['rows']), rank_packets=device['final_dispatches'], captured_payloads=4,
        compared_tensor_rows=152, all_payloads_tokens_and_tensors_equal=True, tensor_comparison=rows,
        recorded_close_and_owner_reap_checked=True, raw_completion_ticks=True,
        **{key: False for key in FALSE})
