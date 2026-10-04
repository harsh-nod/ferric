"""Independent FP64 projections with explicit FP32 output boundaries, revision2.

The original FP64-direct diagnostic remains unchanged. This separate diagnostic
rounds QKV/gate/up FP64 sums to FP32 before BF16, and retains every FP64 dot.
It does not copy lane reductions or introduce whole-layer acceptance tolerance.
Norm and SiLU still use the original independent FP64 formulas.
"""
import hashlib
import json
import math
import os
from pathlib import Path
import stat
import struct
import sys
import types

import numpy as np

ORIGINAL_SOURCE_SHA256 = 'ebf4a216e36b4e74f390757923efb6f8654a747c9d72973c3dbd774f7a7afe44'
REFERENCE_MODE = 'independent-fp64-projection-f32-boundary-v2'

TOKENS = (785, 6722, 315, 9625, 374)
MODEL = 'Qwen/Qwen3-8B'
REVISION = 'b968826d9c46dd6066d109eabc6255188de91218'
QKV_SHA = '9516d017300facd7c481149f865c6a4817045d083702d05558297a82619b0eda'
GENUINE_SHA = 'fbf766e1ded38055e03bfd512184eb4b54f362eedc08c2ea9388b00171a041ea'
CONFIG_SHA = 'f7c4eadfbbf522470667b797a3c89be2524832d2d599797248dc304fff447c30'
SOURCE_SHA = '31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f'
O_SHAS = ('695a205027c01cc6bd7238b76d8ca2e503f508f4be7d6ad7f6dfe427a0c2fb8e',
          '79bca2f8b04191dd004a3248160b1e97848d5b1613b72bfed63ef8a944ec4b4c')
HELPERS = {
    'extract.py': '76872f3558d6f90c21b9eee3ec0ba4ebdd79168b620338faa6bdbdd8b78bdf4a',
    'reference.py': '2e41d6e5715cc561bf818fdee794e4ced74a77f93acd4961d8b15f427f196c6e',
    'policy.json': '9497e55e70a42630d50b74ea33b0856a385d051fb60bc7f23124ee5cbd46e44b',
    'attention_reference.py': 'b0aaa60d86ea3ef940a82cf910753f4480a71adbd816073ad5406465c31b486a',
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def bootstrap_read(path, maximum, digest):
    """Bound the first trusted read before the authenticated helper is loaded."""
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'bootstrap canonical path')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= maximum, 'bootstrap regular extent')
        raw = stream.read(maximum + 1)
        require(len(raw) == before.st_size and hashlib.sha256(raw).hexdigest() == digest, 'bootstrap digest/extent')
        fields = ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')
        for after in (os.fstat(stream.fileno()), os.stat(path, follow_symlinks=False)):
            require(all(getattr(before, key) == getattr(after, key) for key in fields), 'bootstrap file changed')
        return raw


def preliminary_json(raw):
    def pairs(entries):
        value = {}
        for key, item in entries:
            require(key not in value, 'duplicate bootstrap JSON key')
            value[key] = item
        return value
    def invalid(_):
        raise ValueError('nonfinite bootstrap JSON')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def bootstrap(p218_directory, attention_module):
    """Load exact reviewed helper bytes once; no import-path substitution."""
    directory = Path(p218_directory)
    require(directory.is_absolute() and directory.resolve(strict=True) == directory, 'helper directory')
    def load(name, path, digest):
        require(name not in sys.modules, 'helper module name already populated')
        path = Path(path)
        raw = bootstrap_read(path, 65536, digest)
        module = types.ModuleType(name)
        module.__file__ = str(path)
        sys.modules[name] = module
        try:
            exec(compile(raw, str(path), 'exec'), module.__dict__)
        except BaseException:
            del sys.modules[name]
            raise
        return module
    ex = load('extract', directory / 'extract.py', HELPERS['extract.py'])
    policy_path = directory / 'policy.json'
    ex.pinned_bytes({'path': str(policy_path), 'bytes': 2799, 'sha256': HELPERS['policy.json']}, 32768)
    mlp = load('reference', directory / 'reference.py', HELPERS['reference.py'])
    attention = load('attention_reference', attention_module, HELPERS['attention_reference.py'])
    return ex, mlp, attention


def read_document(ex, pin, maximum=131072):
    return ex.json_bytes(ex.pinned_bytes(pin, maximum))


def child_words(ex, mlp, directory, name, row, shape):
    require(Path(name).name == name and row.get('file', name) == name, 'fixture child name')
    require(row['bytes'] == 2 * math.prod(shape), 'fixture extent')
    path = Path(directory) / name
    raw = ex.pinned_bytes({'path': str(path), 'bytes': row['bytes'], 'sha256': row['sha256']}, 50331648)
    array = np.frombuffer(raw, dtype='<u2').reshape(shape)
    mlp.bf16(array)
    return array


def dense_project(mlp, weights, input_words):
    """Independent FP64 row chunks; no source lane-strided accumulation."""
    require(weights.ndim == 2 and input_words.shape == (weights.shape[1],), 'dense projection shape')
    x = mlp.bf16(input_words).astype(np.float64)
    result = []
    for first in range(0, weights.shape[0], 64):
        rows = mlp.bf16(weights[first:first + 64]).astype(np.float64)
        result.extend((rows * x).sum(axis=1, dtype=np.float64))
    result = np.asarray(result, dtype='<f8')
    require(np.isfinite(result).all(), 'dense projection nonfinite')
    return result


def narrow_dense(mlp, values):
    """Independent direct FP64->BF16 RNE at declared BF16 boundaries."""
    return np.asarray([mlp.round_bf16_real(float(value)) for value in values], dtype='<u2')


def partial_f32(values):
    """The declared FP32 partial boundary, not a claim of wave accumulation equality."""
    require(np.isfinite(values).all() and (np.abs(values) <= float(np.finfo(np.float32).max)).all(),
            'finite partial boundary')
    return np.asarray(values, dtype='<f4')


def narrow_projection(mlp, values):
    """Independent FP64 sum -> FP32 output -> BF16 RNE; not wave-tree replay."""
    return narrow_dense(mlp, partial_f32(values))


def dense_norm(mlp, input_words, weight_words):
    require(input_words.ndim == 1 and input_words.shape == weight_words.shape and
            input_words.size in (128, 4096), 'closed norm geometry')
    x, w = mlp.bf16(input_words).astype(np.float64), mlp.bf16(weight_words).astype(np.float64)
    total = math.fsum(float(value) * float(value) for value in x)
    epsilon = struct.unpack('<f', struct.pack('<I', 897988541))[0]
    inverse = 1.0 / math.sqrt(total / x.size + epsilon)
    first = narrow_dense(mlp, x * inverse)
    return narrow_dense(mlp, mlp.bf16(first).astype(np.float64) * w)


def post_qkv(mlp, raw, query_norm, key_norm, position):
    require(raw.shape == (3072,) and query_norm.shape == key_norm.shape == (128,) and
            type(position) is int and 0 <= position < 5, 'post QKV geometry')
    cos = np.asarray([math.cos(position * math.pow(1000000.0, -pair / 64.0))
                      for pair in range(64)], dtype=np.float32)
    sin = np.asarray([math.sin(position * math.pow(1000000.0, -pair / 64.0))
                      for pair in range(64)], dtype=np.float32)
    rotated = []
    for head in range(20):
        normalized = dense_norm(mlp, raw[head * 128:(head + 1) * 128],
                                query_norm if head < 16 else key_norm)
        x = mlp.bf16(normalized)
        # RoPE's FP32 input coefficients and separate multiply/add boundaries
        # remain explicit. Independent norms/dots are still FP64 above.
        lo = np.subtract(np.multiply(x[:64], cos, dtype=np.float32),
                         np.multiply(x[64:], sin, dtype=np.float32), dtype=np.float32)
        hi = np.add(np.multiply(x[64:], cos, dtype=np.float32),
                    np.multiply(x[:64], sin, dtype=np.float32), dtype=np.float32)
        rotated.extend(narrow_dense(mlp, np.concatenate((lo, hi))).tolist())
    return (np.asarray(rotated[:2048], dtype='<u2'), np.asarray(rotated[2048:], dtype='<u2'),
            raw[2560:].copy())


def causal_history(history, rank, position, role):
    require(type(rank) is int and rank in (0, 1) and type(position) is int and 0 <= position < 5,
            'causal history identity')
    require(role in ('key', 'value') and len(history[rank]) == 5, 'history role/length')
    rows = [history[rank][token][role] for token in range(position + 1)]
    require(all(row.shape == (512,) for row in rows), 'causal history row shape')
    return np.concatenate(rows).tolist()


def inputs(ex, mlp, attention, roster):
    require(roster['schema'] == 'ferric-genuine-layer-reference-inputs-v219' and
            set(roster) == {'schema', 'p218_directory', 'attention_module', 'qkv_manifest',
                            'genuine_manifest', 'output_manifest', 'mlp_manifest', 'config', 'oracle'},
            'input roster fields')
    require(roster['qkv_manifest']['sha256'] == QKV_SHA and
            roster['genuine_manifest']['sha256'] == GENUINE_SHA and
            roster['config']['sha256'] == CONFIG_SHA, 'retained authentic source pins')
    qkv, genuine, output = [read_document(ex, roster[key]) for key in
                            ('qkv_manifest', 'genuine_manifest', 'output_manifest')]
    config = read_document(ex, roster['config'])
    for key, value in {'hidden_size': 4096, 'intermediate_size': 12288, 'num_attention_heads': 32,
                       'num_key_value_heads': 8, 'head_dim': 128, 'rope_theta': 1000000.0,
                       'rms_norm_eps': 1e-6}.items():
        require(config[key] == value, 'model config ' + key)
    for report in (qkv, genuine, output):
        require(report['model'] == MODEL and report['revision'] == REVISION, 'cross-model fixture')
    require(qkv['source_shard']['sha256'] == SOURCE_SHA and
            genuine['source']['sha256'] == SOURCE_SHA and output['source']['sha256'] == SOURCE_SHA,
            'source shard identity')
    require(genuine['token_ids'] == list(TOKENS) and genuine['gpu_execution'] is False and
            genuine['schema'] == 'ferric-genuine-first-layer-prompt-v214', 'genuine five-token reference')
    require(qkv['packing'] == [['query_weight', 2048, 0], ['key_weight', 512, 2048],
                               ['value_weight', 512, 2560]], 'QKV packing identity')
    qdir, gdir, odir = [Path(roster[key]['path']).parent for key in
                       ('qkv_manifest', 'genuine_manifest', 'output_manifest')]
    embedding_rows = genuine['source']['embedding_rows']
    require([row['token'] for row in embedding_rows] == list(TOKENS), 'original embedding token order')
    embeddings = []
    for position, token in enumerate(TOKENS):
        name = f'input-pos{position}.bf16'
        row = genuine['files'][name]
        require(row['sha256'] == embedding_rows[position]['sha256'] and
                embedding_rows[position]['bytes'] == 8192 and
                embedding_rows[position]['absolute_byte_offset'] == 9336 + token * 8192,
                'original embedding byte binding')
        embeddings.append(child_words(ex, mlp, gdir, name, row, [4096]))
    norm = child_words(ex, mlp, qdir, 'norm-weight.bf16', qkv['files']['norm-weight.bf16'], [4096])
    qnorm = child_words(ex, mlp, qdir, 'query-norm.bf16', qkv['files']['query-norm.bf16'], [128])
    knorm = child_words(ex, mlp, qdir, 'key-norm.bf16', qkv['files']['key-norm.bf16'], [128])
    packed, output_weights = [], []
    require([row['rank'] for row in output['shards']] == [0, 1], 'O rank roster')
    for rank in (0, 1):
        name = f'packed-qkv-rank{rank}.bf16'
        packed.append(child_words(ex, mlp, qdir, name, qkv['files'][name], [3072, 4096]))
        row = output['shards'][rank]
        require(row['sha256'] == O_SHAS[rank] and row['shape'] == [4096, 2048], 'authentic O shard')
        output_weights.append(child_words(ex, mlp, odir, row['file'], row, [4096, 2048]))
    ex.pinned_bytes(roster['mlp_manifest'], 131072)
    _, postnorm, mlp_weights = mlp.fixture(roster['mlp_manifest']['path'], roster['mlp_manifest']['sha256'])
    require(roster['oracle']['sha256'] == mlp.ORACLE_SHA, 'residual oracle pin')
    ex.pinned_bytes(roster['oracle'], 32768)
    residual = mlp.load_oracle(roster['oracle']['path'])
    return {'embedding': embeddings, 'norm': norm, 'query_norm': qnorm, 'key_norm': knorm,
            'packed': packed, 'output': output_weights, 'post_norm': postnorm,
            'mlp': mlp_weights, 'residual_oracle': residual}


def write_array(out, name, values):
    raw = values.tobytes()
    with (out / name).open('xb') as stream:
        stream.write(raw)
    return {'file': name, 'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest(),
            'dtype': values.dtype.str, 'shape': list(values.shape)}


def generate(ex, mlp, attention, data, out):
    """No observed GPU array is accepted by this interface."""
    normalized = [dense_norm(mlp, embedding, data['norm']) for embedding in data['embedding']]
    history = [[], []]
    for rank in (0, 1):
        for position in range(5):
            raw_fp64 = dense_project(mlp, data['packed'][rank], normalized[position])
            raw = narrow_projection(mlp, raw_fp64)
            query, key, value = post_qkv(mlp, raw, data['query_norm'], data['key_norm'], position)
            history[rank].append({'raw': raw, 'raw_fp64': raw_fp64,
                                 'query': query, 'key': key, 'value': value})
    cases = []
    for position, token in enumerate(TOKENS):
        outputs, partials, per_rank = {}, [], []
        prefix = f'genuine-pos{position}'
        outputs['input'] = write_array(out, prefix + '-input.bf16', data['embedding'][position])
        outputs['input_norm'] = write_array(out, prefix + '-input-norm.bf16', normalized[position])
        for rank in (0, 1):
            state = history[rank][position]
            keys = causal_history(history, rank, position, 'key')
            values = causal_history(history, rank, position, 'value')
            dense, _ = attention.dense_reference(state['query'].tolist(), keys, values, position)
            attended = np.asarray([attention.bf16(value) for value in dense], dtype='<u2')
            partial_fp64 = dense_project(mlp, data['output'][rank], attended)
            partial = partial_f32(partial_fp64)
            partials.append(partial)
            rows = {role: write_array(out, f'{prefix}-rank{rank}-{role}.bf16', state[role])
                    for role in ('raw', 'query', 'key', 'value')}
            rows['raw_fp64'] = write_array(out, f'{prefix}-rank{rank}-raw.f64', state['raw_fp64'])
            rows['output_partial_fp64'] = write_array(out, f'{prefix}-rank{rank}-output-partial.f64', partial_fp64)
            rows['attention'] = write_array(out, f'{prefix}-rank{rank}-attention.bf16', attended)
            rows['output_partial'] = write_array(out, f'{prefix}-rank{rank}-output-partial.f32', partial)
            rows['logical_key_sha256'] = hashlib.sha256(struct.pack(f'<{len(keys)}H', *keys)).hexdigest()
            rows['logical_value_sha256'] = hashlib.sha256(struct.pack(f'<{len(values)}H', *values)).hexdigest()
            per_rank.append(rows)
        first_residual = np.frombuffer(data['residual_oracle'](partials[0].tobytes(), partials[1].tobytes(),
                                                             data['embedding'][position].tobytes()), dtype='<u2')
        outputs['attention_residual'] = write_array(out, prefix + '-attention-residual.bf16', first_residual)
        mlp_norm = dense_norm(mlp, first_residual, data['post_norm'])
        outputs['post_norm'] = write_array(out, prefix + '-post-norm.bf16', mlp_norm)
        down = []
        for rank in (0, 1):
            weights = data['mlp'][rank]
            gate_fp64 = dense_project(mlp, weights['gate'], mlp_norm)
            up_fp64 = dense_project(mlp, weights['up'], mlp_norm)
            gate = narrow_projection(mlp, gate_fp64)
            up = narrow_projection(mlp, up_fp64)
            activation = mlp.swiglu_reference(gate, up)
            down_fp64 = dense_project(mlp, weights['down'], activation)
            partial = partial_f32(down_fp64)
            down.append(partial)
            for name, array in [('gate_fp64', gate_fp64), ('up_fp64', up_fp64),
                                ('down_partial_fp64', down_fp64)]:
                per_rank[rank][name] = write_array(out, f'{prefix}-rank{rank}-{name}.f64', array)
            for name, array in [('gate', gate), ('up', up), ('activation', activation)]:
                per_rank[rank][name] = write_array(out, f'{prefix}-rank{rank}-{name}.bf16', array)
            per_rank[rank]['down_partial'] = write_array(out, f'{prefix}-rank{rank}-down-partial.f32', partial)
        final = np.frombuffer(data['residual_oracle'](down[0].tobytes(), down[1].tobytes(), first_residual.tobytes()),
                              dtype='<u2')
        outputs['layer_output'] = write_array(out, prefix + '-layer-output.bf16', final)
        cases.append({'history_kind': 'genuine', 'position': position, 'token_id': token,
                      'causal_token_ids': list(TOKENS[:position + 1]), 'outputs': outputs, 'ranks': per_rank})
    return cases


def export(roster_path, roster_sha, out):
    roster_path = Path(roster_path)
    raw = bootstrap_read(roster_path, 131072, roster_sha)
    # Only locate exact hash-pinned helpers from this preliminary parse. The
    # complete roster is then decoded with duplicate/nonfinite rejection.
    locate = preliminary_json(raw)
    ex, mlp, attention = bootstrap(locate['p218_directory'], locate['attention_module'])
    roster = ex.json_bytes(raw)
    data = inputs(ex, mlp, attention, roster)
    out = Path(out)
    require(out.is_absolute() and out.parent.resolve(strict=True) == out.parent and not out.exists() and
            not out.is_symlink(), 'create-exclusive output directory')
    out.mkdir(mode=0o700)
    cases = generate(ex, mlp, attention, data, out)
    report = {'schema': 'ferric-genuine-first-layer-reference-projection-f32-v2', 'model': MODEL, 'revision': REVISION,
              'token_ids': list(TOKENS), 'layer': 0, 'tensor_parallel': 2,
              'reference_mode': REFERENCE_MODE, 'original_reference_source_sha256': ORIGINAL_SOURCE_SHA256,
              'input_roster_sha256': roster_sha, 'input_roster': roster,
              'helper_sha256': HELPERS, 'numpy_version': np.__version__, 'cases': cases,
              'gpu_execution': False, 'actual_gpu_intermediate_substitution': False,
              'whole_layer_numerical_acceptance': False, 'acceptance_threshold': None,
              'full_model_correctness': False, 'performance_measured': False,
              'precision': {'projection': 'Independent FP64 row sums retained; QKV/gate/up rounded FP64 -> FP32 -> BF16 RNE',
                            'norm': 'Unchanged independent FP64 reduction/inverse; two direct BF16 RNE boundaries, not source-exact FP32 intermediates',
                            'head_norm': 'Unchanged independent FP64 sum/inverse and two direct BF16 RNE boundaries',
                            'swiglu': 'Unchanged independent FP64 stable SiLU and direct BF16 output; not OCML replay',
                            'rope': 'source FP32 cos/sin and separate multiply/add, then BF16 RNE',
                            'attention': 'retained unpaged FP64 two-pass causal oracle; FP32 then BF16 RNE output',
                            'partial_boundary': 'FP64 O/down dot rounded once to FP32; not device lane-tree equality',
                            'residual': 'frozen integer-bit ordered FP32 TP2 plus residual, one BF16 RNE'},
              'scope': 'Independent first-layer projection-boundary revision2 from authentic embeddings and weights for genuine positions0..4. FP64 sums are retained. This isolates a declared FP32 projection output boundary; it is not an exact source reduction, norm, OCML, framework, whole-layer acceptance or full-model claim.'}
    payload = (json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    with (out / 'manifest.json').open('xb') as stream:
        stream.write(payload)
    return {'path': str(out / 'manifest.json'), 'bytes': len(payload),
            'sha256': hashlib.sha256(payload).hexdigest(), 'cases': 5, 'reference_mode': REFERENCE_MODE,
            'whole_layer_numerical_acceptance': False, 'gpu_execution': False}


if __name__ == '__main__':
    require(len(sys.argv) == 4, 'usage: layer_reference_v2.py INPUT_ROSTER EXPECTED_SHA NEW_OUTPUT_DIR')
    print(json.dumps(export(*sys.argv[1:]), sort_keys=True))
