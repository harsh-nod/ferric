"""Closed replay of one ordered64 packet-tick sidecar; never a wall-time model."""
import collections
import hashlib
import json
import os
from pathlib import Path
import re
import stat

CORE = '55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9'
PROFILE = 'prefill16-decode-ordered64-kv-copy-packet-ticks-v1'
MAX_BYTES = 12 * 1024**2
TICK_UNIT = 'raw_device_ticks_frequency_unspecified'
INTERVAL_SCOPE = 'end_tick_minus_start_tick_not_shader_only_not_wall_time'
TERMINAL_SCOPE = ('raw packet-processing ticks, frequency unspecified; not shader-only time, '
                  'calibrated nanoseconds, or performance qualification')
OPERATIONS = ['embedding', 'input_norm', 'query_projection', 'key_projection', 'value_projection',
              'query_norm', 'key_norm', 'rope', 'kv_append', 'attention', 'attention_output',
              'attention_residual', 'post_attention_norm', 'gate_projection', 'up_projection',
              'swiglu', 'down_projection', 'feed_forward_residual', 'final_norm',
              'head_projection', 'argmax']
METADATA = {'command': 'dispatch_ordered_batch64_profiled', 'max_group_packets': 64,
            'original_publication_boundaries_preserved': True, 'unit': TICK_UNIT,
            'performance_qualified': False,
            'scope': 'raw packet-processing end_tick_minus_start_tick; not shader-only, calibrated nanoseconds, or wall time'}
RECORD_COLUMNS = ['group', 'batch', 'layer_plus_one_zero_means_none', 'operation',
                  'kernel', 'epoch', 'packet', 'start_tick', 'end_tick']
GROUP_COLUMNS = ['group', 'original_publication_0_single_2_ordered64', 'packets']
COPY_KEYS = {'requested_kv_copy_mode', 'kv_copy_mode', 'kv_append_mode',
             'kv_copy_artifact_path', 'kv_copy_artifact'}


def expected_symbol(prefill, operation, attention_merge=False):
    """Exact fixed-graph dispatch route, not a timing-derived classification."""
    prefix = 'ferric_qwen3_tp_batch32_'
    if operation in (2, 3, 4, 13, 14):
        return prefix + ('mfma_gemm_bf16_v5' if prefill else 'wave_gemv_bf16_v5')
    if operation in (10, 16):
        return prefix + ('mfma_gemm_partial_f32_v5' if prefill else 'wave_gemv_partial_f32_v5')
    if operation == 8:
        return 'ferric_qwen3_tp_' + ('prefill16_kv_copy_bf16_v27' if prefill else 'c1_kv_copy_bf16_v19')
    if operation == 9:
        if prefill:
            return prefix + 'wave_paged_gqa_query_hoist_bf16_v14'
        return 'ferric_qwen3_tp_c1_split8_attention_' + ('merge_bf16_v21' if attention_merge else 'partial_f32_v21')
    fixed = {0: 'embedding_bf16_v5', 1: 'wave_rmsnorm_bf16_v15', 5: None, 6: None,
             7: 'rope_v5', 11: 'residual_bf16_v5', 12: 'wave_rmsnorm_bf16_v15',
             15: 'swiglu_bf16_f32_v5', 17: 'residual_bf16_v5', 18: 'wave_rmsnorm_bf16_v15',
             19: 'mfma_head_f32_v8', 20: 'wave_argmax_f32_v11'}
    require(operation in fixed, 'unknown fixed-graph operation')
    return prefix + fixed[operation] if fixed[operation] is not None else 'qwen3_rmsnorm_v1'


def require(value, message):
    if not value:
        raise ValueError(message)


def integer(value, minimum=0, maximum=2**64 - 1):
    require(type(value) is int and minimum <= value <= maximum, 'bounded exact integer required')
    return value


def digest(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value), 'exact SHA256 required')
    return value


def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate sidecar JSON key')
            result[key] = value
        return result
    require(type(raw) is bytes and 0 < len(raw) <= MAX_BYTES, 'bounded nonempty packet sidecar')
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON constant'))


def read_sidecar(path):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical sidecar path required')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    with os.fdopen(fd, 'rb') as source:
        before = os.fstat(source.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1
                and before.st_uid == os.getuid() and stat.S_IMODE(before.st_mode) == 0o600
                and 0 < before.st_size <= MAX_BYTES, 'private owned single-link bounded sidecar required')
        raw = source.read(MAX_BYTES + 1)
        after = os.fstat(source.fileno())
    current = path.lstat()
    require(len(raw) == before.st_size and all(getattr(before, key) == getattr(after, key)
            == getattr(current, key) for key in ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')),
            'sidecar changed during bounded read')
    return raw


def expected_batch(ordinal):
    integer(ordinal, 1, 135)
    prefill = ordinal <= 8
    roles = [(0, 0)]
    for layer in range(1, 37):
        operations = list(range(1, 18))
        if not prefill:
            operations.insert(9, 9)
        roles.extend((layer, operation) for operation in operations)
    if ordinal >= 8:
        roles.extend((0, operation) for operation in (18, 19, 20))
    groups = ([1] + [width for _ in range(36) for width in (11, 6)]
              + ([1, 1, 1] if ordinal == 8 else [])) if prefill else [64] * 10 + [12]
    return prefill, roles, groups


def summaries(partitions):
    result = []
    for (epoch, prefill, operation, kernel), values in sorted(partitions.items()):
        values.sort()
        count = len(values)
        result.append({'epoch': epoch, 'phase': 'prefill' if prefill else 'decode',
            'operation': OPERATIONS[operation], 'kernel': kernel, 'count': count,
            'min_raw_dispatch_interval_ticks': values[0],
            'p50_raw_dispatch_interval_ticks': values[(count - 1) // 2],
            'p95_raw_dispatch_interval_ticks': values[(95 * count + 99) // 100 - 1],
            'max_raw_dispatch_interval_ticks': values[-1],
            'sum_raw_dispatch_interval_ticks_nonadditive': str(sum(values))})
    return result


def validate(raw, terminal, *, sidecar_path, device, allowed_kernels, copy_identity):
    """Replay bytes and the post-Closed terminal event against bound image identities."""
    require(type(copy_identity) is dict and set(copy_identity) == COPY_KEYS
            and all(copy_identity[key] == 'parallel-c1-v19'
                    for key in ('requested_kv_copy_mode', 'kv_copy_mode', 'kv_append_mode')),
            'exact V19 terminal copy binding')
    captured = decode(raw)
    require(type(captured) is dict and set(captured) == {'schema', 'evidence_scope', 'core_revision',
        'tick_unit', 'interval_scope', 'original_max_group_packets', 'device_unique_id', 'operations',
        'kernels', 'batches', 'group_columns', 'groups', 'record_columns', 'records'}, 'closed sidecar schema')
    expected = {'schema': 'FerricOrdered64RawPacketIntervalsV1',
        'evidence_scope': 'transport_intervals_only_not_numerical_or_performance_qualification',
        'core_revision': CORE, 'tick_unit': TICK_UNIT, 'interval_scope': INTERVAL_SCOPE,
        'original_max_group_packets': 64, 'device_unique_id': integer(device, 1),
        'operations': OPERATIONS, 'group_columns': GROUP_COLUMNS, 'record_columns': RECORD_COLUMNS}
    require(all(type(captured.get(key)) is type(value) and captured[key] == value
                for key, value in expected.items()), 'sidecar attribution or clock labeling changed')
    require(type(allowed_kernels) is dict and 1 <= len(allowed_kernels) <= 256, 'bound image kernel catalog')
    for symbol, image in allowed_kernels.items():
        require(type(symbol) is str and re.fullmatch('[A-Za-z0-9_]{1,256}', symbol), 'bound kernel symbol')
        digest(image)
    kernels = captured['kernels']
    require(type(kernels) is list and len(kernels) == len(allowed_kernels), 'complete admitted kernel catalog')
    ids, observed, previous = {}, {}, 0
    for row in kernels:
        require(type(row) is dict and set(row) == {'kernel', 'symbol', 'object_sha256'}, 'closed kernel identity')
        ident = integer(row['kernel'], previous + 1)
        previous = ident
        require(type(row['object_sha256']) is list and len(row['object_sha256']) == 32,
                'exact object digest bytes')
        image = bytes(integer(value, 0, 255) for value in row['object_sha256']).hex()
        require(row['symbol'] in allowed_kernels and row['symbol'] not in observed
                and allowed_kernels[row['symbol']] == image, 'kernel image identity mismatch')
        ids[ident], observed[row['symbol']] = row['symbol'], image
    require(observed == allowed_kernels, 'missing admitted kernel')
    batches, groups, records = (captured[key] for key in ('batches', 'groups', 'records'))
    require(type(batches) is list and len(batches) == 135, 'exact 135 batches required')
    require(type(groups) is list and len(groups) == 1984, 'exact original publication count required')
    require(type(records) is list and len(records) == 87711, 'exact 87711 records required')
    packet, group, request, scheduler = 0, 0, None, -1
    partitions = collections.defaultdict(list)
    group_counts = collections.Counter()
    for ordinal, batch in enumerate(batches, 1):
        prefill, roles, widths = expected_batch(ordinal)
        require(type(batch) is dict and set(batch) == {'ordinal', 'scheduler_batch_id', 'request',
                'prefill_rows', 'decode_rows', 'expected_packets'}, 'closed batch identity')
        wanted = {'ordinal': ordinal, 'prefill_rows': 16 if prefill else 0,
                  'decode_rows': 0 if prefill else 1, 'expected_packets': len(roles)}
        require(all(type(batch.get(key)) is int and batch[key] == value for key, value in wanted.items()),
                'batch rows/packet count changed')
        scheduler = integer(batch['scheduler_batch_id'], scheduler + 1)
        identity = batch['request']
        require(type(identity) is list and len(identity) == 2, 'request slot/generation identity')
        integer(identity[0], 0, 31)
        integer(identity[1], 1)
        require(request is None or request == identity, 'multiple request identities in capture')
        request = identity
        role_index = 0
        for width in widths:
            publication = 0 if width == 1 else 2
            require(groups[group] == [group, publication, width]
                    and all(type(value) is int for value in groups[group]), 'original group boundary drift')
            group_counts['prefill_direct' if prefill and width == 1 else 'prefill_ordered' if prefill else 'decode_ordered'] += 1
            for _ in range(width):
                row = records[packet]
                require(type(row) is list and len(row) == 9, 'exact packet record width')
                for value in row:
                    integer(value)
                layer, operation = roles[role_index]
                require(row[:4] == [group, ordinal, layer, operation] and row[4] in ids
                        and row[5:7] == [0, packet] and 0 < row[7] < row[8],
                        'packet attribution/frontier/kernel/ticks changed')
                merge = not prefill and operation == 9 and roles[role_index - 1] == (layer, 9)
                require(ids[row[4]] == expected_symbol(prefill, operation, merge),
                        'admitted kernel does not match exact phase/operation/substep')
                partitions[(0, prefill, operation, row[4])].append(row[8] - row[7])
                role_index += 1
                packet += 1
            group += 1
        require(role_index == len(roles), 'incomplete batch role sequence')
    require(packet == 87711 and group == 1984 and group_counts == {
        'prefill_direct': 11, 'prefill_ordered': 576, 'decode_ordered': 1397}, 'capture totals differ')
    summary = summaries(partitions)
    require(len(summary) == 43, 'fixed graph phase/kernel summary count differs')
    expected_terminal = {'schema': 'FerricOrdered64PacketTicksClosedV1', 'authority': 'none',
        'live_profile': PROFILE, 'performance_qualified': False, 'serving_qualified': False,
        'raw_capture_path': str(sidecar_path), 'raw_capture_sha256': hashlib.sha256(raw).hexdigest(),
        'raw_capture_bytes': len(raw), 'core_wire_revision': CORE, 'scope': TERMINAL_SCOPE,
        'numerical_status': 'independently compare every emitted model token ID',
        'rank_dispatch_counts': [87711], 'all_workers_exited': True, 'summary': summary,
        **copy_identity}
    require(type(terminal) is dict and set(terminal) == set(expected_terminal)
            and all(type(terminal[key]) is type(value) and terminal[key] == value
                    for key, value in expected_terminal.items()), 'terminal sidecar binding or raw summary differs')
    return {'schema': 'FerricOrdered64PacketTickReplayV1', 'accepted': True,
        'performance_qualified': False, 'latency_admitted': False,
        'tick_unit': TICK_UNIT, 'interval_scope': INTERVAL_SCOPE,
        'sums_are_wall_time_shares': False, 'batches': 135, 'records': packet, 'groups': group,
        'group_counts': dict(group_counts), 'raw_capture_sha256': hashlib.sha256(raw).hexdigest(),
        'raw_capture_bytes': len(raw), 'kernels': kernels, 'summary': summary}
