"""Closed ABI encoding for the C1 Wave and split-K4 gate/up experiment."""
import struct

GUARD_BYTES = 64
WAVE = 'ferric_qwen3_tp_batch32_wave_gemv_bf16_v5'
PARTIAL = 'ferric_qwen3_c1_gate_up_splitk4_mfma_partial_f32_r1'
MERGE = 'ferric_qwen3_c1_gate_up_splitk4_merge_bf16_r1'
SHAPES = {
    WAVE: (12288, (8192, 100663296, 786432), (2, 2, 2), ('read', 'read', 'write')),
    PARTIAL: (3072, (8192, 100663296, 196608), (2, 2, 4), ('read', 'read', 'write')),
    MERGE: (192, (196608, 786432), (4, 2), ('read', 'write')),
}


def require(value, message):
    if not value:
        raise ValueError(message)


def view(record, access):
    require(type(record) is dict and set(record) == {'id', 'data', 'element_bytes'}, 'closed buffer record')
    require(type(record['id']) is int and record['id'] > 0 and type(record['data']) is bytes
            and type(record['element_bytes']) is int and record['element_bytes'] in (2, 4),
            'allocated typed buffer required')
    require(access in ('read', 'write') and len(record['data']) > 0
            and len(record['data']) % record['element_bytes'] == 0, 'complete typed view')
    return {'record': record, 'access': access}


def encode(metadata, views, scalars, groups):
    require(type(metadata) is dict and metadata.get('symbol') in SHAPES, 'exact admitted kernel root')
    symbol = metadata['symbol']
    expected_groups, sizes, widths, accesses = SHAPES[symbol]
    require(type(groups) is int and groups == expected_groups, 'exact per-root grid')
    require(type(scalars) is tuple and all(type(value) is int for value in scalars), 'typed scalar tuple')
    if symbol == MERGE:
        require(scalars == (), 'merge has no scalar arguments')
    else:
        require(len(scalars) == 5 and scalars[:4] == (1, 12288, 4096, 1)
                and scalars[4] in (4, 5), 'fixed C1 gate/up argument shape')
    require(type(views) is list and len(views) == len(sizes), 'complete per-root view roster')
    checked = []
    for item in views:
        require(type(item) is dict and set(item) == {'record', 'access'}, 'closed complete view')
        checked.append(view(item['record'], item['access']))
    require(tuple(len(item['record']['data']) for item in checked) == sizes
            and tuple(item['record']['element_bytes'] for item in checked) == widths
            and tuple(item['access'] for item in checked) == accesses, 'exact typed extents and roles')
    require(len({item['record']['id'] for item in checked}) == len(checked), 'views may not alias')
    require(metadata['wavefront_size'] == 64 and metadata['private_segment_bytes'] == 0
            and metadata['group_segment_bytes'] == 0 and metadata['kernarg_alignment'] == 8,
            'fixed admitted resource shape')
    arguments = metadata['explicit_arguments']
    require(len(arguments) == len(views) * 2 + len(scalars), 'complete explicit argument roster')
    implicit = (16 * len(views) + 4 * len(scalars) + 7) // 8 * 8
    size = implicit + 256
    require(type(metadata['kernarg_bytes']) is int and metadata['kernarg_bytes'] == size
            and metadata['implicit_argument_offset'] == implicit
            and metadata['implicit_argument_bytes'] == 256, 'exact padded COV6 ABI')
    encoded, pointers = bytearray(size), []
    for index, item in enumerate(checked):
        pointer, count = arguments[2 * index:2 * index + 2]
        offset = index * 16
        width = widths[index]
        require(pointer['offset'] == offset and pointer['bytes'] == 8
                and pointer['global_buffer'] is True and pointer['pointee_alignment'] in (None, width)
                and pointer['access'] in (None, item['access']), 'typed pointer metadata')
        require(count['offset'] == offset + 8 and count['bytes'] == 8
                and count['global_buffer'] is False, 'typed slice length metadata')
        extent = sizes[index]
        struct.pack_into('<Q', encoded, offset + 8, extent // width)
        pointers.append({'kernarg_offset': offset, 'buffer': item['record']['id'],
            'buffer_offset': GUARD_BYTES, 'extent_bytes': extent, 'access': item['access']})
    for index, (argument, value) in enumerate(zip(arguments[len(views) * 2:], scalars, strict=True)):
        offset = len(views) * 16 + index * 4
        require(argument['offset'] == offset and argument['bytes'] == 4
                and argument['global_buffer'] is False, 'typed u32 metadata')
        struct.pack_into('<I', encoded, offset, value)
    return bytes(encoded), pointers
