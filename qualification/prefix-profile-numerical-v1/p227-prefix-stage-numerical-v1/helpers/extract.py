"""Bounded authentic Qwen3 layer0 MLP extraction. No device or subprocess use."""
import hashlib
import json
import os
from pathlib import Path
import stat
import struct
import sys

MODEL = 'Qwen/Qwen3-8B'
REVISION = 'b968826d9c46dd6066d109eabc6255188de91218'
SOURCE_SHA = '31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f'
INDEX_SHA = 'f9fdbcb91c23971c13ec5d5f2573d2349e8f61f2f049371ec699281748fdb1bc'
HEADER_SHA = '979bbeed365485ddaa67a1ed41d0289e15e2f3ba0b3388cb93e42d31f346d1df'
KEYS = {
    'post_norm': ('model.layers.0.post_attention_layernorm.weight', [4096]),
    'gate': ('model.layers.0.mlp.gate_proj.weight', [12288, 4096]),
    'up': ('model.layers.0.mlp.up_proj.weight', [12288, 4096]),
    'down': ('model.layers.0.mlp.down_proj.weight', [4096, 12288]),
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def strict_object(pairs):
    value = {}
    for key, item in pairs:
        require(key not in value, 'duplicate JSON field: ' + key)
        value[key] = item
    return value


def json_bytes(raw):
    return json.loads(raw, object_pairs_hook=strict_object,
                      parse_constant=lambda _: (_ for _ in ()).throw(ValueError('nonfinite JSON')))


def identity(info):
    return (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns)


def canonical(path):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'noncanonical path')
    return path


def opened(path, expected_bytes, maximum):
    require(type(expected_bytes) is int and 0 < expected_bytes <= maximum, 'file extent')
    path = canonical(path)
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    stream = os.fdopen(fd, 'rb')
    try:
        info = os.fstat(fd)
        require(stat.S_ISREG(info.st_mode) and info.st_size == expected_bytes, 'regular exact-size file')
        return stream, identity(info), path
    except BaseException:
        stream.close()
        raise


def hash_stream(stream):
    stream.seek(0)
    digest = hashlib.sha256()
    while True:
        block = stream.read(1024 * 1024)
        if not block:
            return digest.hexdigest()
        digest.update(block)


def unchanged(stream, before, path):
    require(identity(os.fstat(stream.fileno())) == before, 'open file changed')
    require(identity(os.stat(path, follow_symlinks=False)) == before, 'path replaced')
    require(canonical(path) == path, 'path no longer canonical')


def pinned_bytes(pin, maximum):
    require(isinstance(pin, dict) and {'path', 'bytes', 'sha256'} <= pin.keys(), 'file pin')
    stream, before, path = opened(pin['path'], pin['bytes'], maximum)
    with stream:
        raw = stream.read(maximum + 1)
        require(len(raw) == pin['bytes'], 'short or oversized read')
        require(hashlib.sha256(raw).hexdigest() == pin['sha256'], 'file hash')
        unchanged(stream, before, path)
        return raw


def file_pin(path, raw):
    return {'path': str(path), 'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}


def tensor_entry(header, key, shape, payload_bytes):
    require(key in header and isinstance(header[key], dict), 'tensor missing')
    entry = header[key]
    require(set(entry) == {'dtype', 'shape', 'data_offsets'}, 'tensor fields')
    require(entry['dtype'] == 'BF16' and entry['shape'] == shape, 'tensor dtype/shape')
    require(all(type(x) is int and x > 0 for x in entry['shape']), 'shape integer')
    offsets = entry['data_offsets']
    require(isinstance(offsets, list) and len(offsets) == 2 and
            all(type(x) is int and x >= 0 for x in offsets), 'tensor offsets')
    count = 1
    for extent in shape:
        count *= extent
    lo, hi = offsets
    require(lo % 2 == 0 and hi - lo == count * 2 and hi <= payload_bytes, 'tensor byte extent')
    for other_key, other in header.items():
        if other_key in (key, '__metadata__'):
            continue
        require(isinstance(other, dict), 'other tensor schema')
        span = other.get('data_offsets')
        require(isinstance(span, list) and len(span) == 2 and
                all(type(x) is int for x in span) and 0 <= span[0] <= span[1] <= payload_bytes,
                'other tensor extent')
        require(not (lo < span[1] and span[0] < hi), 'overlapping tensors')
    return lo, hi


def finite_bf16(raw):
    require(len(raw) % 2 == 0, 'odd BF16 byte length')
    require(all(word & 0x7f80 != 0x7f80 for (word,) in struct.iter_unpack('<H', raw)),
            'nonfinite BF16 weight')


def shard_ranges(shape, rank, axis):
    """Byte ranges relative to one row-major BF16 tensor; testable on small data."""
    require(type(rank) is int and rank in (0, 1), 'rank must be exact 0 or 1')
    require(len(shape) == 2 and all(type(x) is int and x > 0 for x in shape), 'matrix shape')
    require(axis in ('rows', 'columns'), 'shard axis')
    rows, columns = shape
    if axis == 'rows':
        require(rows % 2 == 0, 'rows not divisible by two')
        length = rows // 2 * columns * 2
        return [(rank * length, length)], [rows // 2, columns]
    require(columns % 2 == 0, 'columns not divisible by two')
    length = columns
    return [(row * columns * 2 + rank * length, length) for row in range(rows)], [rows, columns // 2]


def write_ranges(stream, tensor_start, ranges, destination, shape):
    digest, count = hashlib.sha256(), 0
    with destination.open('xb') as output:
        for offset, length in ranges:
            stream.seek(tensor_start + offset)
            remaining = length
            while remaining:
                raw = stream.read(min(1024 * 1024, remaining))
                require(raw and len(raw) % 2 == 0, 'short tensor read')
                finite_bf16(raw)
                output.write(raw)
                digest.update(raw)
                count += len(raw)
                remaining -= len(raw)
        output.flush()
        os.fsync(output.fileno())
    expected = 2
    for extent in shape:
        expected *= extent
    require(count == expected, 'shard output extent')
    return {'path': str(destination), 'bytes': count, 'sha256': digest.hexdigest(),
            'shape': shape, 'dtype': 'BF16'}


def export(inventory_path, inventory_sha, out):
    inventory_path = canonical(inventory_path)
    inv_pin = {'path': str(inventory_path), 'bytes': inventory_path.stat().st_size, 'sha256': inventory_sha}
    inventory = json_bytes(pinned_bytes(inv_pin, 128 * 1024))
    require(inventory['schema'] == 'ferric-layer0-mlp-source-inventory-v218' and
            inventory['model'] == MODEL and inventory['revision'] == REVISION and
            inventory['gpu_execution'] is False, 'inventory identity')
    source, index_pin = inventory['source'], inventory['index']
    require(source['bytes'] == 3996250744 and source['sha256'] == SOURCE_SHA and
            source['header_bytes'] == 9328 and source['header_sha256'] == HEADER_SHA,
            'unrecognized authentic model shard')
    require(index_pin['bytes'] == 32878 and index_pin['sha256'] == INDEX_SHA, 'model index pin')
    index = json_bytes(pinned_bytes(index_pin, 128 * 1024))
    require(Path(source['path']).parent == Path(index_pin['path']).parent, 'index/source parent')
    for key, _ in KEYS.values():
        require(index['weight_map'][key] == Path(source['path']).name, 'index tensor-to-shard binding')
    policy_path = canonical(Path(__file__).parent / 'policy.json')
    policy_raw = policy_path.read_bytes()
    policy = json_bytes(policy_raw)
    require(policy['schema'] == 'ferric-layer0-mlp-numerical-policy-v218' and
            policy['adaptive_tolerance'] is False, 'fixed policy')
    out = Path(out)
    require(out.is_absolute() and out.parent.resolve(strict=True) == out.parent, 'output parent')
    require(not out.exists() and not out.is_symlink(), 'output must not exist')
    stream, before, source_path = opened(source['path'], source['bytes'], 4 * 1024**3)
    with stream:
        require(hash_stream(stream) == SOURCE_SHA, 'model source hash before extraction')
        stream.seek(0)
        header_size = struct.unpack('<Q', stream.read(8))[0]
        require(header_size == source['header_bytes'], 'header length')
        header_raw = stream.read(header_size)
        require(hashlib.sha256(header_raw).hexdigest() == HEADER_SHA, 'header hash')
        header = json_bytes(header_raw)
        base = 8 + header_size
        entries = {role: tensor_entry(header, key, shape, source['bytes'] - base)
                   for role, (key, shape) in KEYS.items()}
        require(inventory['tensors'] == {key: header[key] for key, _ in KEYS.values()},
                'inventory/header tensor mismatch')
        out.mkdir(mode=0o700)
        post_norm = write_ranges(stream, base + entries['post_norm'][0], [(0, 8192)],
                                 out / 'post-norm.bf16', [4096])
        ranks = []
        for rank in (0, 1):
            row = {'rank': rank}
            for role in ('gate', 'up', 'down'):
                ranges, shape = shard_ranges(KEYS[role][1], rank, 'columns' if role == 'down' else 'rows')
                row[role] = write_ranges(stream, base + entries[role][0], ranges,
                                         out / f'{role}-rank{rank}.bf16', shape)
            ranks.append(row)
        require(hash_stream(stream) == SOURCE_SHA, 'model source hash after extraction')
        unchanged(stream, before, source_path)
    with (out / 'policy.json').open('xb') as output:
        output.write(policy_raw)
    manifest = {'schema': 'ferric-layer0-mlp-fixture-v218', 'model': MODEL, 'revision': REVISION,
                'source': source, 'index': index_pin, 'source_inventory': inv_pin,
                'post_norm': post_norm, 'ranks': ranks,
                'numerical_policy': file_pin(out / 'policy.json', policy_raw),
                'extractor': file_pin(canonical(__file__), Path(__file__).read_bytes()),
                'gpu_execution': False, 'production_authority': False,
                'source_identity_during_extraction': list(before)}
    raw = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()
    with (out / 'manifest.json').open('xb') as output:
        output.write(raw)
    return file_pin(out / 'manifest.json', raw)


if __name__ == '__main__':
    require(len(sys.argv) == 4, 'usage: extract.py INVENTORY EXPECTED_SHA NEW_OUTPUT_DIR')
    print(json.dumps(export(*sys.argv[1:]), sort_keys=True))
