"""Authentic O shards and CPU references conditional on accepted P214 attention."""
import hashlib
import json
import math
import os
from pathlib import Path
import struct
import sys
import unittest
import numpy as np

R = Path('/home/harmenon/ferric-asrock-42')
D214 = R / 'evidence/wave-attention-v214'
SOURCE = R / 'models/qwen3-source-v1/target/model-00001-of-00005.safetensors'
SOURCE_SHA = '31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f'
HEADER_SHA = '979bbeed365485ddaa67a1ed41d0289e15e2f3ba0b3388cb93e42d31f346d1df'
SUITE_SHA = 'f999d11fd4d3313092d5fc7ac6f409e1dc5aa023bb54a4b366989aca2da333e4'
GENUINE_SHA = 'fbf766e1ded38055e03bfd512184eb4b54f362eedc08c2ea9388b00171a041ea'
SHARD_SHA = ['695a205027c01cc6bd7238b76d8ca2e503f508f4be7d6ad7f6dfe427a0c2fb8e',
    '79bca2f8b04191dd004a3248160b1e97848d5b1613b72bfed63ef8a944ec4b4c']

def sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        while block := stream.read(1024**2): digest.update(block)
    return digest.hexdigest()

def pinned_json(path, digest, size=None):
    raw = path.read_bytes()
    assert size is None or len(raw) == size
    assert hashlib.sha256(raw).hexdigest() == digest
    return json.loads(raw)

def checked_json(row):
    path = Path(row['path'])
    assert path.resolve() == path and path.is_relative_to(D214)
    return pinned_json(path, row['sha256'], row['bytes'])

def column_shard(matrix, rank, world=2):
    assert matrix.dtype == np.dtype('<u2') and matrix.ndim == 2
    assert type(rank) is int and type(world) is int and world > 0 and 0 <= rank < world
    assert matrix.shape[1] > 0 and matrix.shape[1] % world == 0
    width = matrix.shape[1] // world
    return np.ascontiguousarray(matrix[:, rank * width:(rank + 1) * width])

def decode(words):
    return (words.astype(np.uint32) << 16).view(np.float32)

def gamma_up(n, u):
    return math.nextafter(n * u / (1.0 - n * u), math.inf)

def reference(weights, attention):
    assert weights.shape == (4096, 2048) and attention.shape == (2048,)
    w, a = decode(weights), decode(attention)
    assert np.isfinite(w).all() and np.isfinite(a).all()
    # Independent dense FP64 reference. Do not reuse the lane-strided tree.
    w64, a64 = w.astype(np.float64), a.astype(np.float64)
    exact = w64 @ a64
    absolute_sum = (np.abs(w64) * np.abs(a64)).sum(axis=1, dtype=np.float64)
    g32, g64 = gamma_up(39, 2.0**-24), gamma_up(2048, 2.0**-53)
    upper = np.nextafter(absolute_sum / math.nextafter(1.0-g64, -math.inf), np.inf)
    relative = np.nextafter(math.nextafter(g32+g64, math.inf) * upper, np.inf)
    underflow = math.nextafter(4159 * 2.0**-150 / math.nextafter(1.0-39*2.0**-24, -math.inf), math.inf)
    bound = np.nextafter(relative + underflow, np.inf)
    # Separate FP32 multiply and add,32 sequential products in every lane.
    partial = np.zeros((4096, 64), dtype=np.float32)
    for step in range(32):
        product = w[:, step*64:(step+1)*64] * a[step*64:(step+1)*64]
        partial = partial + product
        assert np.isfinite(product).all() and np.isfinite(partial).all()
    lanes = np.arange(64)
    for offset in (1, 2, 4, 8, 16, 32):
        partial = partial + partial[:, lanes ^ offset]
        assert np.isfinite(partial).all()
    staged = np.ascontiguousarray(partial[:, 0])
    error = np.abs(staged.astype(np.float64)-exact)
    error_upper = np.where(error == 0, 0, np.nextafter(error, np.inf))
    assert np.isfinite(exact).all() and np.isfinite(bound).all() and (error_upper <= bound).all()
    return exact, bound, staged, float(np.max(error_upper / bound))

def write_bytes(out, name, raw):
    path = out / name
    with path.open('xb') as stream: stream.write(raw)
    return dict(file=name, bytes=len(raw), sha256=sha(path))

def export(out):
    assert np.__version__ == '2.2.6' and sys.byteorder == 'little'
    out.mkdir(mode=0o700)
    suite_path = D214 / 'gpu-suite-v2/complete.json'
    genuine_path = D214 / 'reference-v1/genuine/manifest.json'
    suite = pinned_json(suite_path, SUITE_SHA, 251917)
    genuine = pinned_json(genuine_path, GENUINE_SHA, 14862)
    assert genuine['source']['sha256'] == SOURCE_SHA
    assert genuine['revision'] == 'b968826d9c46dd6066d109eabc6255188de91218'
    before = SOURCE.stat()
    assert before.st_size == 3996250744 and sha(SOURCE) == SOURCE_SHA
    with SOURCE.open('rb') as stream:
        header_length = struct.unpack('<Q', stream.read(8))[0]
        assert header_length == 9328
        raw_header = stream.read(header_length)
        assert hashlib.sha256(raw_header).hexdigest() == HEADER_SHA
        header = json.loads(raw_header)
        tensor = header['model.layers.0.self_attn.o_proj.weight']
        assert tensor == dict(dtype='BF16', shape=[4096, 4096], data_offsets=[1555054848, 1588609280])
        start = 8 + header_length + tensor['data_offsets'][0]
        assert start == 1555064184
        stream.seek(start)
        raw = stream.read(4096 * 4096 * 2)
        assert len(raw) == 33554432
    full = np.frombuffer(raw, dtype='<u2').reshape(4096, 4096)
    shards = [column_shard(full, rank) for rank in (0, 1)]
    files, shard_rows = {}, []
    for rank, shard in enumerate(shards):
        assert np.isfinite(decode(shard)).all()
        name = f'output-weight-rank{rank}.bf16'
        row = write_bytes(out, name, shard.tobytes())
        assert row['sha256'] == SHARD_SHA[rank] and row['bytes'] == 16777216
        files[name] = row
        shard_rows.append(dict(rank=rank, shape=[4096, 2048], **row))
    expected = {(kind, rank, pos) for kind, positions in [('patterned', [0,15,16,2047,2048,2303]),
        ('genuine', range(5))] for rank in (0, 1) for pos in positions}
    seen, cases = set(), []
    for case in suite['cases']:
        key = tuple(case['case'])
        assert key in expected and key not in seen and case['completed_and_closed'] is True
        seen.add(key)
        complete = checked_json(case['complete'])
        assert complete['case'] == list(key) and complete['completed_and_closed'] is True
        comparison = complete['comparison']
        assert comparison['upstream_mismatches'] == [0, 0, 0] and comparison['cache_mismatches'] == [0, 0]
        assert comparison['attention']['mismatches'] == comparison['attention']['nonfinite'] == 0
        observation = checked_json(complete['observation'])
        words = observation['attention_words']
        assert len(words) == 2048 and all(type(word) is int and 0 <= word <= 65535 for word in words)
        packed = struct.pack('<2048H', *words)
        assert hashlib.sha256(packed).hexdigest() == comparison['output_sha256'][5]
        attention = np.frombuffer(packed, dtype='<u2')
        exact, bound, staged, ratio = reference(shards[key[1]], attention)
        prefix = f'{key[0]}-rank{key[1]}-pos{key[2]}'
        outputs = {}
        for suffix, payload in [('attention.bf16', packed), ('reference.f64', exact.astype('<f8').tobytes()),
            ('bound.f64', bound.astype('<f8').tobytes()), ('staged.f32', staged.astype('<f4').tobytes())]:
            row = write_bytes(out, prefix+'-'+suffix, payload)
            files[row['file']] = row
            outputs[suffix] = row
        cases.append(dict(case=list(key), predecessor=case['complete'], observation=complete['observation'],
            attention_sha256=hashlib.sha256(packed).hexdigest(), output_weight_sha256=SHARD_SHA[key[1]],
            outputs=outputs, staged_fp32_maximum_bound_ratio=ratio))
    assert seen == expected and sha(SOURCE) == SOURCE_SHA
    after = SOURCE.stat()
    assert (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) == (
        after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)
    report = dict(schema='ferric-conditional-output-reference-v215', conditional_on_accepted_p214_attention=True,
        gpu_execution=False, full_layer_correctness=False, full_model_correctness=False, performance_measured=False,
        fresh_v5_attention_requires_independent_acceptance_and_matching_input_hash=True,
        model=genuine['model'], revision=genuine['revision'], source=dict(path=str(SOURCE), sha256=SOURCE_SHA, header_sha256=HEADER_SHA),
        suite_sha256=SUITE_SHA, genuine_manifest_sha256=GENUINE_SHA, numpy_version=np.__version__,
        numerical_policy='gamma39+gamma2048_f64 times outward absolute-product sum plus4159 half-subnormal allowances; IEEE gradual FP32 underflow required',
        shards=shard_rows, cases=cases, files=files)
    with (out/'manifest.json').open('x') as stream: json.dump(report, stream, indent=2, sort_keys=True)
    print(json.dumps(dict(cases=len(cases), files=len(files), manifest_sha256=sha(out/'manifest.json'),
        maximum_staged_fp32_bound_ratio=max(case['staged_fp32_maximum_bound_ratio'] for case in cases))))

class ShardTests(unittest.TestCase):
    def test_json_parses_only_the_once_verified_buffer(self):
        class ChangingPath:
            def __init__(self): self.reads = 0
            def read_bytes(self):
                self.reads += 1
                return b'{"value":1}' if self.reads == 1 else b'{"value":2}'
        path = ChangingPath()
        digest = hashlib.sha256(b'{"value":1}').hexdigest()
        assert pinned_json(path, digest, 11) == {'value': 1}
        assert path.reads == 1
        with self.assertRaises(AssertionError): pinned_json(ChangingPath(), '00'*32, 11)
        with self.assertRaises(AssertionError): pinned_json(ChangingPath(), digest, 12)
    def test_column_rank_and_row_order(self):
        matrix = np.arange(24, dtype='<u2').reshape(4, 6)
        np.testing.assert_array_equal(column_shard(matrix, 0), [[0,1,2],[6,7,8],[12,13,14],[18,19,20]])
        np.testing.assert_array_equal(column_shard(matrix, 1), [[3,4,5],[9,10,11],[15,16,17],[21,22,23]])
    def test_rank_shape_dtype_and_divisibility_rejected(self):
        good = np.zeros((4, 6), dtype='<u2')
        for matrix, rank, world in [(good,-1,2), (good,2,2), (good,0,0), (good,0,4),
            (good.astype(np.float32),0,2), (good.ravel(),0,2)]:
            with self.assertRaises(AssertionError): column_shard(matrix, rank, world)
    def test_fp32_tree_and_independent_fp64_reference(self):
        weights = np.full((4096,2048), 0x3f80, dtype='<u2')
        attention = np.full(2048, 0x3f80, dtype='<u2')
        exact, bound, staged, ratio = reference(weights, attention)
        assert (exact == 2048).all() and (staged == 2048).all() and ratio == 0
        assert (bound > 0).all() and (bound < 0.01).all()
    def test_cancellation_is_not_a_relative_output_bound(self):
        weights = np.zeros((4096,2048), dtype='<u2')
        weights[:, 0], weights[:, 64], weights[:, 128] = 0x4b80, 0x3f80, 0xcb80
        exact, bound, staged, ratio = reference(weights, np.full(2048, 0x3f80, dtype='<u2'))
        assert (exact == 1).all() and (staged == 0).all() and (bound >= 1).all() and 0 < ratio < 1

if __name__ == '__main__':
    if sys.argv[1:] == ['--test']: unittest.main(argv=[sys.argv[0]])
    else: export(Path(sys.argv[1]))
