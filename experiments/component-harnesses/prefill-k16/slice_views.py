"""Checked encoding of three full BF16 views for matched K16 roots."""
import struct

GUARD_BYTES = 64


def require(value, message):
    if not value:
        raise ValueError(message)


def view(record, access, offset=0, extent=None):
    require(type(record) is dict and set(record) == {'id', 'data', 'element_bytes'}, 'closed buffer record')
    require(type(record['id']) is int and record['id'] > 0 and type(record['data']) is bytes
            and type(record['element_bytes']) is int and record['element_bytes'] == 2,
            'allocated BF16 buffer required')
    length = len(record['data'])
    extent = length if extent is None else extent
    require(access in ('read', 'write') and type(offset) is int and type(extent) is int
            and offset >= 0 and extent > 0 and offset % 2 == extent % 2 == 0
            and offset + extent <= length, 'aligned nonempty in-allocation BF16 view')
    return {'record': record, 'access': access, 'offset': offset, 'extent': extent}


def encode(metadata, views, scalars, groups):
    require(type(views) is list and len(views) == 3 and type(scalars) is tuple
            and len(scalars) == 5 and all(type(value) is int for value in scalars)
            and scalars[:4] == (32, 12288, 4096, 1) and scalars[4] in (4, 5),
            'fixed K16 gate/up argument shape')
    require(type(groups) is int and groups == 1536,
            'exact per-arm grid')
    checked = []
    for item in views:
        require(type(item) is dict and set(item) == {'record', 'access', 'offset', 'extent'}, 'closed view')
        checked.append(view(item['record'], item['access'], item['offset'], item['extent']))
    require([item['access'] for item in checked] == ['read', 'read'] + ['write'] * (len(views) - 2),
            'exact pointer access roles')
    require([(item['offset'], item['extent']) for item in checked[:2]] == [(0, 262144), (0, 100663296)],
            'complete fixed input views')
    require([len(item['record']['data']) for item in checked[:2]] == [262144, 100663296],
            'no padded input allocation')
    outputs = checked[2:]
    require(all(len(item['record']['data']) == 786432 for item in outputs), 'one exact K16 output extent')
    require(len({item['record']['id'] for item in outputs}) == 1
            and [(item['offset'], item['extent']) for item in outputs]
            == [(0, 786432)], 'one complete output view')
    require(len({checked[0]['record']['id'], checked[1]['record']['id'], outputs[0]['record']['id']}) == 3,
            'input and output allocations may not alias')
    require(metadata['wavefront_size'] == 64 and metadata['private_segment_bytes'] == 0
            and metadata['group_segment_bytes'] == 0 and metadata['kernarg_alignment'] == 8,
            'fixed admitted kernel resource shape')
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
        require(pointer['offset'] == offset and pointer['bytes'] == 8
                and pointer['global_buffer'] is True and pointer['pointee_alignment'] in (None, 2)
                and pointer['access'] in (None, item['access']), 'typed pointer metadata')
        require(count['offset'] == offset + 8 and count['bytes'] == 8
                and count['global_buffer'] is False, 'typed slice length metadata')
        struct.pack_into('<Q', encoded, offset + 8, item['extent'] // 2)
        pointers.append({'kernarg_offset': offset, 'buffer': item['record']['id'],
            'buffer_offset': GUARD_BYTES + item['offset'], 'extent_bytes': item['extent'],
            'access': item['access']})
    for index, (argument, value) in enumerate(zip(arguments[len(views) * 2:], scalars, strict=True)):
        offset = len(views) * 16 + index * 4
        require(argument['offset'] == offset and argument['bytes'] == 4
                and argument['global_buffer'] is False, 'typed u32 metadata')
        struct.pack_into('<I', encoded, offset, value)
    return bytes(encoded), pointers
