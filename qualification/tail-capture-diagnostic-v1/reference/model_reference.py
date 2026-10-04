"""Independent original-weight Qwen3-8B two-forward diagnostic, never acceptance.

The CLI accepts no observed hidden state, KV history, or GPU intermediate. It
maps the pinned original safetensors and computes every layer itself. Synthetic
unit fixtures use the same equations at smaller dimensions, not a GPU simulator.
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

MODEL = 'Qwen/Qwen3-8B'
REVISION = 'b968826d9c46dd6066d109eabc6255188de91218'
MODE = 'independent-original-weight-fp64-projection-f32-boundary-two-forward-v1'
HELPER_SHA = '3facbd495b6fd0dd01856fd389101ed7c7b0af3c28d94171d185d3a28335c60f'
ORACLE_SHA = '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3'
POLICY_SHA = '8978c56f1b0c108c9aee7cdbe7f8ef2de11181bf83ca8f68fa4c69ff247bf031'
ORIGINAL_PINS = {
    'config.json': (728, 'f7c4eadfbbf522470667b797a3c89be2524832d2d599797248dc304fff447c30'),
    'model.safetensors.index.json': (32878, 'f9fdbcb91c23971c13ec5d5f2573d2349e8f61f2f049371ec699281748fdb1bc'),
    'model-00001-of-00005.safetensors': (3996250744, '31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f'),
    'model-00002-of-00005.safetensors': (3993160032, '5991236cea6fe21f3d43cab0f0e84448734fbbe0789816202989f2ddc9d18282'),
    'model-00003-of-00005.safetensors': (3959604768, 'c5185c4794be2d8a9784d5753c9922db38df478ce11f9ed0b415b7304d896836'),
    'model-00004-of-00005.safetensors': (3187841392, 'b5ee7de71fbf17db3d5704e0c8f2bc7d005ca9e1d7ca2aeb19827b0cfcaa917a'),
    'model-00005-of-00005.safetensors': (1244659840, '20c2d6366ab85c90786ccdd829cd2b9e7d30ef3b2ebbb998280e7e4014b542ff'),
}
MAX_OUTPUT = 64 << 20
MAX_FILES = 4096
STAT_FIELDS = ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def json_bytes(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    def invalid(_):
        raise ValueError('nonfinite JSON')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def valid_pin(pin, maximum):
    require(isinstance(pin, dict) and set(pin) == {'path', 'bytes', 'sha256'}, 'file pin fields')
    path = Path(pin['path'])
    require(path.is_absolute() and path.resolve(strict=True) == path and not path.is_symlink(),
            'canonical file pin path')
    require(type(pin['bytes']) is int and 0 < pin['bytes'] <= maximum and
            isinstance(pin['sha256'], str) and len(pin['sha256']) == 64 and
            all(c in '0123456789abcdef' for c in pin['sha256']), 'pin extent/digest')
    return path


class PinnedFile:
    def __init__(self, pin, maximum):
        self.pin, self.path = pin, valid_pin(pin, maximum)
        self.stream = os.fdopen(os.open(self.path, os.O_RDONLY | os.O_NOFOLLOW), 'rb')
        try:
            self.before = os.fstat(self.stream.fileno())
            require(stat.S_ISREG(self.before.st_mode) and self.before.st_size == pin['bytes'],
                    'pinned regular file size')
            self.recheck()
        except BaseException:
            self.stream.close()
            raise

    def stable(self):
        for after in (os.fstat(self.stream.fileno()), os.stat(self.path, follow_symlinks=False)):
            require(all(getattr(self.before, key) == getattr(after, key) for key in STAT_FIELDS),
                    'pinned input changed')

    def recheck(self):
        self.stable()
        self.stream.seek(0)
        digest, total = hashlib.sha256(), 0
        while True:
            chunk = self.stream.read(4 << 20)
            if not chunk:
                break
            digest.update(chunk)
            total += len(chunk)
        require(total == self.pin['bytes'] and digest.hexdigest() == self.pin['sha256'],
                'pinned file digest')
        self.stable()

    def read(self, offset, size):
        require(type(offset) is int and type(size) is int and
                0 <= offset <= offset + size <= self.pin['bytes'], 'pinned read range')
        self.stable()
        self.stream.seek(offset)
        raw = self.stream.read(size)
        require(len(raw) == size, 'short pinned read')
        self.stable()
        return raw

    def close(self):
        self.stream.close()


def small_pin(pin, maximum):
    value = PinnedFile(pin, maximum)
    try:
        return value.read(0, pin['bytes'])
    finally:
        value.close()


def bootstrap(directory=None):
    directory = Path(directory or Path(__file__).parent / 'helpers').resolve(strict=True)
    path = directory / 'layer_reference_v2.py'
    raw = small_pin({'path': str(path), 'bytes': path.stat().st_size, 'sha256': HELPER_SHA}, 65536)
    module = types.ModuleType('_p222_frozen_layer_reference')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    ex, mlp, attention = module.bootstrap(directory, directory / 'attention_reference.py')
    oracle_path = directory / 'residual_oracle.py'
    oracle_raw = module.bootstrap_read(oracle_path, 32768, ORACLE_SHA)
    oracle = types.ModuleType('_p222_frozen_residual')
    exec(compile(oracle_raw, str(oracle_path), 'exec'), oracle.__dict__)
    return module, mlp, attention, oracle


class Geometry:
    def __init__(self, layers, hidden, intermediate, qheads, kvheads, head, vocabulary):
        values = (layers, hidden, intermediate, qheads, kvheads, head, vocabulary)
        require(all(type(value) is int and value > 0 for value in values), 'positive geometry')
        require(hidden == qheads * head and qheads % 2 == kvheads % 2 == intermediate % 2 == 0
                and qheads % kvheads == 0 and head % 2 == 0, 'TP2 geometry divisibility')
        self.layers, self.hidden, self.intermediate = layers, hidden, intermediate
        self.qheads, self.kvheads, self.head, self.vocabulary = qheads, kvheads, head, vocabulary


QWEN8B = Geometry(36, 4096, 12288, 32, 8, 128, 151936)


def tensor_shapes(g):
    out = {'model.embed_tokens.weight': (g.vocabulary, g.hidden),
           'model.norm.weight': (g.hidden,), 'lm_head.weight': (g.vocabulary, g.hidden)}
    for layer in range(g.layers):
        prefix = f'model.layers.{layer}.'
        for name, shape in {
            'input_layernorm.weight': (g.hidden,),
            'self_attn.q_proj.weight': (g.qheads * g.head, g.hidden),
            'self_attn.k_proj.weight': (g.kvheads * g.head, g.hidden),
            'self_attn.v_proj.weight': (g.kvheads * g.head, g.hidden),
            'self_attn.q_norm.weight': (g.head,), 'self_attn.k_norm.weight': (g.head,),
            'self_attn.o_proj.weight': (g.hidden, g.hidden),
            'post_attention_layernorm.weight': (g.hidden,),
            'mlp.gate_proj.weight': (g.intermediate, g.hidden),
            'mlp.up_proj.weight': (g.intermediate, g.hidden),
            'mlp.down_proj.weight': (g.hidden, g.intermediate),
        }.items():
            out[prefix + name] = shape
    return out


def decode_header(raw, payload_bytes):
    header = json_bytes(raw)
    require(isinstance(header, dict), 'safetensors header object')
    tensors, intervals = {}, []
    for name, row in header.items():
        if name == '__metadata__':
            require(isinstance(row, dict) and all(isinstance(k, str) and isinstance(v, str)
                    for k, v in row.items()), 'safetensors metadata')
            continue
        require(isinstance(name, str) and isinstance(row, dict) and
                set(row) == {'dtype', 'shape', 'data_offsets'} and row['dtype'] == 'BF16',
                'BF16 tensor header')
        shape, offsets = row['shape'], row['data_offsets']
        require(isinstance(shape, list) and len(shape) in (1, 2) and
                all(type(x) is int and x > 0 for x in shape), 'tensor shape')
        require(isinstance(offsets, list) and len(offsets) == 2 and
                all(type(x) is int for x in offsets) and
                0 <= offsets[0] < offsets[1] <= payload_bytes and
                offsets[1] - offsets[0] == 2 * math.prod(shape), 'tensor offsets')
        tensors[name] = (tuple(shape), offsets[0], offsets[1])
        intervals.append((offsets[0], offsets[1]))
    cursor = 0
    for first, end in sorted(intervals):
        require(first == cursor, 'overlapping or unclaimed safetensors data')
        cursor = end
    require(cursor == payload_bytes and tensors, 'complete safetensors data')
    return tensors


class OriginalWeights:
    def __init__(self, index_pin, shard_pins):
        original_pin(index_pin, 'model.safetensors.index.json')
        for number, pin in enumerate(shard_pins, 1):
            original_pin(pin, f'model-{number:05}-of-00005.safetensors')
        index = json_bytes(small_pin(index_pin, 1 << 20))
        require(set(index) == {'metadata', 'weight_map'} and isinstance(index['weight_map'], dict),
                'original index schema')
        self.files, self.arrays, self.pins = [], {}, [index_pin, *shard_pins]
        expected = tensor_shapes(QWEN8B)
        require(index['metadata'] == {'total_size': sum(2 * math.prod(shape) for shape in expected.values())},
                'original index tensor byte total')
        require(set(index['weight_map']) == set(expected), 'exact399 original tensors')
        names = [Path(pin['path']).name for pin in shard_pins]
        require(names == [f'model-{i:05}-of-00005.safetensors' for i in range(1, 6)],
                'ordered exact five original shards')
        require(set(index['weight_map'].values()) == set(names), 'index shard membership')
        try:
            for name, pin in zip(names, shard_pins):
                source = PinnedFile(pin, 5 << 30)
                self.files.append(source)
                header_bytes = struct.unpack('<Q', source.read(0, 8))[0]
                require(2 <= header_bytes <= 8 << 20 and 8 + header_bytes < pin['bytes'],
                        'bounded safetensors header')
                offset = 8 + header_bytes
                header = decode_header(source.read(8, header_bytes), pin['bytes'] - offset)
                require(set(header) == {key for key, value in index['weight_map'].items() if value == name},
                        'index/header tensor membership')
                for key, (shape, first, _) in header.items():
                    require(shape == expected[key] and key not in self.arrays, 'original tensor shape/identity')
                    self.arrays[key] = np.memmap(source.stream, mode='r', dtype='<u2',
                                                offset=offset + first, shape=shape, order='C')
            require(len(self.arrays) == 399, 'complete original model')
        except BaseException:
            self.close()
            raise

    def tensor(self, name):
        require(name in self.arrays, 'missing original tensor')
        return self.arrays[name]

    def recheck(self):
        for source in self.files:
            source.recheck()
        small_pin(self.pins[0], 1 << 20)

    def close(self):
        self.arrays.clear()
        for source in self.files:
            source.close()
        self.files.clear()


def dense_norm(helpers, words, weight):
    old, mlp, _, _ = helpers
    require(words.ndim == 1 and words.shape == weight.shape and words.size > 0, 'norm geometry')
    if words.size in (128, 4096):
        return old.dense_norm(mlp, words, weight)
    x, w = mlp.bf16(words).astype(np.float64), mlp.bf16(weight).astype(np.float64)
    epsilon = struct.unpack('<f', struct.pack('<I', 897988541))[0]
    inverse = 1.0 / math.sqrt(math.fsum(float(v) * float(v) for v in x) / len(x) + epsilon)
    first = old.narrow_dense(mlp, x * inverse)
    return old.narrow_dense(mlp, mlp.bf16(first).astype(np.float64) * w)


def rotate(helpers, words, weight, position, head):
    old, mlp, _, _ = helpers
    require(words.ndim == 1 and words.size % head == 0 and weight.shape == (head,), 'head geometry')
    half = head // 2
    cos = np.asarray([math.cos(position * math.pow(1_000_000.0, -pair / half))
                      for pair in range(half)], dtype='<f4')
    sin = np.asarray([math.sin(position * math.pow(1_000_000.0, -pair / half))
                      for pair in range(half)], dtype='<f4')
    out = []
    for first in range(0, len(words), head):
        normalized = mlp.bf16(dense_norm(helpers, words[first:first + head], weight))
        low = np.subtract(np.multiply(normalized[:half], cos, dtype=np.float32),
                          np.multiply(normalized[half:], sin, dtype=np.float32), dtype=np.float32)
        high = np.add(np.multiply(normalized[half:], cos, dtype=np.float32),
                      np.multiply(normalized[:half], sin, dtype=np.float32), dtype=np.float32)
        out.extend(old.narrow_dense(mlp, np.concatenate((low, high))))
    return np.asarray(out, dtype='<u2')


def dense_attention(helpers, g, query, keys, values):
    old, mlp, attention_oracle, _ = helpers
    qh, kh, dim = g.qheads // 2, g.kvheads // 2, g.head
    require(query.shape == (qh * dim,) and len(keys) == len(values) and len(keys) in (1, 2),
            'attention causal geometry')
    require(all(row.shape == (kh * dim,) for row in [*keys, *values]), 'attention cache geometry')
    if (g.qheads, g.kvheads, g.head) == (32, 8, 128):
        values64, _ = attention_oracle.dense_reference(
            query.tolist(), np.concatenate(keys).tolist(),
            np.concatenate(values).tolist(), len(keys) - 1)
        exact = np.asarray(values64, dtype='<f8')
        return old.narrow_projection(mlp, exact), exact
    q = mlp.bf16(query).astype(np.float64).reshape(qh, dim)
    k = np.asarray([mlp.bf16(row) for row in keys], dtype=np.float64).reshape(len(keys), kh, dim)
    v = np.asarray([mlp.bf16(row) for row in values], dtype=np.float64).reshape(len(values), kh, dim)
    scale = float(np.float32(1.0 / math.sqrt(dim)))
    out = []
    for head in range(qh):
        group = head // (qh // kh)
        logits = [math.fsum(float(a) * float(b) for a, b in zip(q[head], row[group])) * scale for row in k]
        maximum = max(logits)
        probabilities = [math.exp(value - maximum) for value in logits]
        denominator = math.fsum(probabilities)
        require(math.isfinite(denominator) and denominator > 0, 'attention denominator')
        if len(keys) == 1:
            out.extend(v[0, group])  # Retain the frozen oracle's exact single-token signed zeros.
        else:
            out.extend(math.fsum(probabilities[t] * float(v[t, group, lane]) for t in range(len(keys)))
                       / denominator for lane in range(dim))
    exact = np.asarray(out, dtype='<f8')
    return old.narrow_projection(mlp, exact), exact


def residual(helpers, first, second, original):
    oracle = helpers[3]
    require(first.dtype == second.dtype == np.dtype('<f4') and first.shape == second.shape == original.shape,
            'residual partial geometry')
    words = [oracle.ordered_residual_bits(int(a), int(b), int(c)) for a, b, c in
             zip(first.view('<u4'), second.view('<u4'), original)]
    return np.asarray(words, dtype='<u2')


def choose(words, mlp):
    values = mlp.bf16(words)
    require(values.ndim == 1 and values.size > 0, 'argmax geometry')
    return int(np.argmax(values))  # First index wins ties, matching the declared diagnostic policy.


def original_pin(pin, name):
    require(name in ORIGINAL_PINS and Path(pin['path']).name == name and
            (pin['bytes'], pin['sha256']) == ORIGINAL_PINS[name], 'authentic original file identity')


def run_model(weights, helpers, g, mode, tokens, emit):
    """Two forwards, original inputs only. `emit` records but cannot replace arrays."""
    old, mlp, _, _ = helpers
    require(mode in ('teacher_forced', 'autoregressive') and isinstance(tokens, list) and
            len(tokens) == (2 if mode == 'teacher_forced' else 1) and
            all(type(token) is int and 0 <= token < g.vocabulary for token in tokens), 'token mode/roster')
    destination = emit
    def emit(name, array):
        # A diagnostic sink receives owned copies, never a mutable recurrence alias.
        destination(name, np.array(array, copy=True))
    history = [[{'key': [], 'value': []} for _ in range(2)] for _ in range(g.layers)]
    cases = []
    for position in range(2):
        token = tokens[position] if mode == 'teacher_forced' or position == 0 else cases[0]['output_token']
        hidden = np.asarray(weights.tensor('model.embed_tokens.weight')[token], dtype='<u2').copy()
        mlp.bf16(hidden)
        emit(f'pos{position}-embedding', hidden)
        for layer in range(g.layers):
            prefix = f'model.layers.{layer}.'
            name = f'pos{position}-layer{layer}'
            normalized = dense_norm(helpers, hidden, weights.tensor(prefix + 'input_layernorm.weight'))
            emit(name + '-input-norm', normalized)
            partials = []
            for rank in (0, 1):
                raw = []
                for role, heads in [('q', g.qheads), ('k', g.kvheads), ('v', g.kvheads)]:
                    rows = heads * g.head // 2
                    weight = weights.tensor(prefix + f'self_attn.{role}_proj.weight')[rank * rows:(rank + 1) * rows]
                    exact = old.dense_project(mlp, weight, normalized)
                    emit(f'{name}-rank{rank}-{role}-fp64', exact)
                    raw.append(old.narrow_projection(mlp, exact))
                query = rotate(helpers, raw[0], weights.tensor(prefix + 'self_attn.q_norm.weight'), position, g.head)
                key = rotate(helpers, raw[1], weights.tensor(prefix + 'self_attn.k_norm.weight'), position, g.head)
                state = history[layer][rank]
                require(len(state['key']) == len(state['value']) == position, 'causal history recurrence')
                state['key'].append(key.copy())
                state['value'].append(raw[2].copy())
                attention, attention64 = dense_attention(helpers, g, query, state['key'], state['value'])
                qcols = g.hidden // 2
                weight = weights.tensor(prefix + 'self_attn.o_proj.weight')[:, rank * qcols:(rank + 1) * qcols]
                output64 = old.dense_project(mlp, weight, attention)
                output = old.partial_f32(output64)
                partials.append(output)
                for role, array in [('qkv', np.concatenate(raw)), ('query', query), ('key', key),
                                    ('value', raw[2]), ('attention', attention), ('attention-fp64', attention64),
                                    ('output-partial-fp64', output64), ('output-partial', output)]:
                    emit(f'{name}-rank{rank}-{role}', array)
            first = residual(helpers, partials[0], partials[1], hidden)
            emit(name + '-attention-residual', first)
            normalized = dense_norm(helpers, first, weights.tensor(prefix + 'post_attention_layernorm.weight'))
            emit(name + '-post-norm', normalized)
            partials = []
            for rank in (0, 1):
                rows = g.intermediate // 2
                projected = []
                for role in ('gate', 'up'):
                    weight = weights.tensor(prefix + f'mlp.{role}_proj.weight')[rank * rows:(rank + 1) * rows]
                    exact = old.dense_project(mlp, weight, normalized)
                    words = old.narrow_projection(mlp, exact)
                    projected.append(words)
                    emit(f'{name}-rank{rank}-{role}-fp64', exact)
                    emit(f'{name}-rank{rank}-{role}', words)
                activation = mlp.swiglu_reference(*projected)
                weight = weights.tensor(prefix + 'mlp.down_proj.weight')[:, rank * rows:(rank + 1) * rows]
                exact = old.dense_project(mlp, weight, activation)
                partial = old.partial_f32(exact)
                partials.append(partial)
                emit(f'{name}-rank{rank}-activation', activation)
                emit(f'{name}-rank{rank}-down-partial-fp64', exact)
                emit(f'{name}-rank{rank}-down-partial', partial)
            hidden = residual(helpers, partials[0], partials[1], first)
            emit(name + '-hidden', hidden)
        normalized = dense_norm(helpers, hidden, weights.tensor('model.norm.weight'))
        logits64 = old.dense_project(mlp, weights.tensor('lm_head.weight'), normalized)
        logits32 = old.partial_f32(logits64)
        logits = old.narrow_projection(mlp, logits64)
        choice = choose(logits, mlp)
        for role, array in [('final-hidden', hidden), ('final-norm', normalized),
                            ('logits-fp64', logits64), ('logits-fp32', logits32), ('logits', logits)]:
            emit(f'pos{position}-{role}', array)
        cases.append({'position': position, 'input_token': token, 'output_token': choice})
    return cases


class Output:
    def __init__(self, directory):
        self.directory, self.files, self.total = Path(directory), {}, 0
        require(self.directory.is_absolute() and self.directory.parent.resolve(strict=True) == self.directory.parent
                and not self.directory.exists() and not self.directory.is_symlink(), 'new canonical output directory')
        self.directory.mkdir(mode=0o700)

    def emit(self, name, array):
        require(isinstance(name, str) and name and all(c.isalnum() or c == '-' for c in name)
                and name not in self.files and len(self.files) < MAX_FILES, 'unique output name')
        require(array.dtype.str in ('<u2', '<f4', '<f8') and array.ndim == 1 and
                np.isfinite(array).all(), 'finite typed output array')
        if array.dtype.str == '<u2':
            require(not np.any((array & 0x7f80) == 0x7f80), 'nonfinite BF16 output')
        raw = array.tobytes()
        require(self.total + len(raw) <= MAX_OUTPUT, 'bounded output bytes')
        filename = name + { '<u2': '.bf16', '<f4': '.f32', '<f8': '.f64'}[array.dtype.str]
        with (self.directory / filename).open('xb') as stream:
            stream.write(raw)
        self.total += len(raw)
        self.files[name] = {'file': filename, 'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest(),
                            'shape': list(array.shape), 'dtype': array.dtype.str}


def export(roster_path, roster_sha, output):
    path = Path(roster_path)
    pin = {'path': str(path), 'bytes': path.stat().st_size, 'sha256': roster_sha}
    raw = small_pin(pin, 1 << 20)
    roster = json_bytes(raw)
    require(set(roster) == {'schema', 'model', 'revision', 'mode', 'tokens', 'token_provenance',
                          'source_authentication', 'config', 'index', 'shards'} and
            roster['schema'] == 'ferric-p222-original-model-reference-inputs-v1' and
            roster['model'] == MODEL and roster['revision'] == REVISION, 'reference roster')
    require(isinstance(roster['shards'], list) and len(roster['shards']) == 5, 'five shard pins')
    original_pin(roster['config'], 'config.json')
    config = json_bytes(small_pin(roster['config'], 1 << 20))
    for name, expected in {'model_type': 'qwen3', 'hidden_size': 4096, 'intermediate_size': 12288,
                           'num_hidden_layers': 36, 'num_attention_heads': 32, 'num_key_value_heads': 8,
                           'head_dim': 128, 'vocab_size': 151936, 'rope_theta': 1000000.0,
                           'rms_norm_eps': 1e-6, 'max_position_embeddings': 40960,
                           'tie_word_embeddings': False}.items():
        require(config.get(name) == expected, 'fixed Qwen config ' + name)
    # Provenance is retained exactly, not reinterpreted as an inference approval.
    small_pin(roster['token_provenance'], 16 << 20)
    small_pin(roster['source_authentication'], 16 << 20)
    helpers = bootstrap()
    require(np.__version__ == '2.2.6' and sys.byteorder == 'little', 'qualified CPU numerical environment')
    policy_path = (Path(__file__).parent / 'policy.json').resolve(strict=True)
    policy_pin = {'path': str(policy_path), 'bytes': policy_path.stat().st_size, 'sha256': POLICY_SHA}
    policy_raw = small_pin(policy_pin, 32768)
    policy = json_bytes(policy_raw)
    weights = OriginalWeights(roster['index'], roster['shards'])
    try:
        out = Output(output)
        cases = run_model(weights, helpers, QWEN8B, roster['mode'], roster['tokens'], out.emit)
        weights.recheck()
        require(small_pin(pin, 1 << 20) == raw, 'reference roster changed')
        for key, maximum in [('config', 1 << 20), ('token_provenance', 16 << 20),
                             ('source_authentication', 16 << 20)]:
            small_pin(roster[key], maximum)
        require(small_pin(policy_pin, 32768) == policy_raw, 'reference policy changed')
        report = {'schema': 'ferric-p222-original-model-two-forward-reference-v1',
                  'model': MODEL, 'revision': REVISION, 'reference_mode': MODE,
                  'input_roster': roster, 'input_roster_sha256': roster_sha,
                  'numpy_version': np.__version__, 'cases': cases, 'files': out.files,
                  'output_bytes': out.total, 'policy': policy,
                  'policy_sha256': POLICY_SHA, 'layer_helper_sha256': HELPER_SHA,
                  'residual_oracle_sha256': ORACLE_SHA,
                  'gpu_execution': False, 'actual_gpu_intermediate_substitution': False,
                  'numerical_acceptance': False, 'acceptance_threshold': None,
                  'full_model_correctness': False, 'performance_measured': False}
        payload = (json.dumps(report, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()
        require(len(payload) <= 4 << 20, 'bounded reference manifest')
        with (out.directory / 'manifest.json').open('xb') as stream:
            stream.write(payload)
        return {'path': str(out.directory / 'manifest.json'), 'bytes': len(payload),
                'sha256': hashlib.sha256(payload).hexdigest(), 'cases': cases,
                'numerical_acceptance': False, 'gpu_execution': False}
    finally:
        weights.close()


if __name__ == '__main__':
    require(len(sys.argv) == 4, 'model_reference.py ROSTER EXPECTED_SHA NEW_OUTPUT_DIR')
    print(json.dumps(export(*sys.argv[1:]), sort_keys=True))
