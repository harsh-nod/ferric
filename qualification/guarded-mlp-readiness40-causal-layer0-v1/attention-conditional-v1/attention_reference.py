"""CPU-only P214 attention fixtures and an unpaged FP64 two-pass oracle."""
import argparse
from array import array
import hashlib
import json
import math
from pathlib import Path
import struct
import sys

if not __debug__:
    raise RuntimeError('reference validation requires assertions enabled')
sys.dont_write_bytecode = True
CONTEXT, Q_HEADS, KV_HEADS, HEAD_DIM, PAGE = 2304, 16, 4, 128, 16
POSITIONS = (0, 15, 16, 2047, 2048, 2303)
TOKENS = (785, 6722, 315, 9625, 374)
POISON = 0x7fc1
SCALE_BITS = 0x3db504f3
SCALE = struct.unpack('<f', struct.pack('<I', SCALE_BITS))[0]
QKV_MANIFEST_SHA = '9516d017300facd7c481149f865c6a4817045d083702d05558297a82619b0eda'
POST_REPORT_SHA = 'd6b2fdba3e4b9a1d24860d083928cfdd65faaf60d00edd53f5432c9c5b3e7f6b'
POLICY = dict(schema='ferric-attention-tolerance-v214', exact_position_zero=True,
    bf16_steps=1, cancellation_coefficient=5e-5,
    acceptance='exact at position 0; otherwise steps <= 1 OR abs_error <= 5e-5 * per_kv_head_max_abs_V',
    reference_rounding='FP64 oracle -> FP32 ties-to-even -> BF16 ties-to-even',
    max_abs_V='maximum magnitude over all causal value words for the mapped KV head',
    nonfinite='reject query, every causal K/V word, scores and outputs; future poison is never read',
    gpu_execution=False, adaptive_tolerance=False)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def pinned(path, expected=None, size=None):
    path = Path(path).absolute()
    assert path.resolve() == path and path.is_file() and not path.is_symlink()
    before = path.stat()
    raw = path.read_bytes()
    after = path.stat()
    fields = ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')
    assert all(getattr(before, key) == getattr(after, key) for key in fields)
    assert size is None or len(raw) == size
    assert expected is None or digest(raw) == expected
    return raw


def f32(value):
    result = struct.unpack('<f', struct.pack('<f', value))[0]
    if not math.isfinite(result):
        raise ValueError('nonfinite FP32')
    return result


def decode(word):
    if type(word) is not int or not 0 <= word <= 65535:
        raise ValueError('BF16 word')
    result = struct.unpack('<f', struct.pack('<I', word << 16))[0]
    if not math.isfinite(result):
        raise ValueError('nonfinite causal BF16 word')
    return result


def bf16(value):
    raw = struct.unpack('<I', struct.pack('<f', f32(value)))[0]
    word = ((raw + 0x7fff + ((raw >> 16) & 1)) >> 16) & 65535
    decode(word)
    return word


def unpack(raw):
    if len(raw) % 2:
        raise ValueError('BF16 byte extent')
    return [word for word, in struct.iter_unpack('<H', raw)]


def pack(words):
    return b''.join(struct.pack('<H', word) for word in words)


def ordered(word):
    decode(word)
    return word if word & 0x8000 == 0 else -(word & 0x7fff)


def table():
    return [(5 * logical + 7) % 144 for logical in range(144)]


def validate_table(pages):
    if len(pages) != 144 or any(type(page) is not int for page in pages) or sorted(pages) != list(range(144)):
        raise ValueError('complete physical page permutation')


def slot(position, pages):
    validate_table(pages)
    if type(position) is not int or not 0 <= position < CONTEXT:
        raise ValueError('position')
    return pages[position // PAGE] * PAGE + position % PAGE


def pattern_word(kind, rank, token, head, dim):
    assert kind in ('key', 'value') and rank in (0, 1)
    assert 0 <= token < CONTEXT and 0 <= head < KV_HEADS and 0 <= dim < HEAD_DIM
    if kind == 'key':
        value = ((token * 7 + head * 11 + dim * 3 + rank * 13) % 61 - 30) / 64.0
    else:
        value = ((token * 17 + head * 5 + dim * 13 + rank * 19) % 97 - 48) / 32.0
    return bf16(value)


def patterned_history(rank, position, current_key, current_value):
    assert rank in (0, 1) and position in POSITIONS
    assert len(current_key) == len(current_value) == KV_HEADS * HEAD_DIM
    outputs = []
    for kind, current in [('key', current_key), ('value', current_value)]:
        result = array('H', (pattern_word(kind, rank, token, head, dim)
            for token in range(position) for head in range(KV_HEADS) for dim in range(HEAD_DIM)))
        result.extend(current)
        outputs.append(result)
    return outputs


def physical_cache(logical, position, pages, include_current=True):
    """Materializer only. The independent oracle never calls this function."""
    validate_table(pages)
    if len(logical) != (position + 1) * 512:
        raise ValueError('logical cache shape')
    result = array('H', [POISON]) * (CONTEXT * 512)
    for token in range(position + int(include_current)):
        start = (pages[token // PAGE] * PAGE + token % PAGE) * 512
        result[start:start + 512] = array('H', logical[token * 512:(token + 1) * 512])
    return result


def dense_reference(query, keys, values, position):
    """No page table, online recurrence, FP32 reduction, or device helper."""
    if type(position) is not int or not 0 <= position < CONTEXT:
        raise ValueError('position')
    if len(query) != 2048 or len(keys) != len(values) or len(keys) != (position + 1) * 512:
        raise ValueError('exact logical attention shapes')
    q, k, v = [list(map(decode, words)) for words in (query, keys, values)]
    output, head_maxima = [], []
    for head in range(Q_HEADS):
        # Independent proportional mapping, not the runtime's integer /4.
        kv = head * KV_HEADS // Q_HEADS
        vh = [v[token * 512 + kv * 128:(token * 512 + kv * 128 + 128)]
              for token in range(position + 1)]
        head_maxima.append(max(abs(x) for row in vh for x in row))
        scores = [math.fsum(q[head * 128 + d] * k[token * 512 + kv * 128 + d]
                           for d in range(128)) * SCALE for token in range(position + 1)]
        if not all(math.isfinite(score) for score in scores):
            raise ValueError('nonfinite dense score')
        maximum = max(scores)
        weights = [math.exp(score - maximum) for score in scores]
        denominator = math.fsum(weights)
        if not math.isfinite(denominator) or denominator <= 0:
            raise ValueError('invalid dense softmax denominator')
        # Preserve the exact BF16 value, including signed zero, for one token.
        output.extend(vh[0] if position == 0 else
            [math.fsum(weight * row[d] for weight, row in zip(weights, vh)) / denominator
             for d in range(128)])
    assert all(math.isfinite(value) for value in output)
    return output, head_maxima


def compare(actual_words, reference, maxima, position):
    if len(actual_words) != 2048 or len(reference) != 2048 or len(maxima) != 16:
        raise ValueError('comparison shapes')
    exact = tolerated = max_steps = 0
    max_error = 0.0
    for index, (word, expected) in enumerate(zip(actual_words, reference)):
        actual = decode(word)
        if not math.isfinite(expected) or not math.isfinite(maxima[index // 128]) or maxima[index // 128] < 0:
            raise ValueError('nonfinite reference or bound')
        expected_word = bf16(expected)
        steps = abs(ordered(word) - ordered(expected_word))
        error = abs(actual - expected)
        if position == 0:
            valid = word == expected_word
        else:
            valid = steps <= 1 or error <= POLICY['cancellation_coefficient'] * maxima[index // 128]
        if not valid:
            raise ValueError(f'attention tolerance index={index} steps={steps} error={error}')
        exact += word == expected_word
        tolerated += word != expected_word
        max_steps, max_error = max(max_steps, steps), max(max_error, error)
    return dict(exact=exact, tolerated=tolerated, max_bf16_steps=max_steps, max_abs_error=max_error)


def cpu_wave_model(query, physical_keys, physical_values, position, pages,
                   kv_mapping=None, omit_current=False, include_future=False):
    """Diagnostic FP32 Wave64/XOR + online model, not the acceptance oracle.

    Every lane's distinct XOR ordering is retained. NumPy only vectorizes
    elementwise FP32 operations; host math.exp is rounded separately to FP32.
    This does not certify the GPU exponential approximation or instruction ISA.
    """
    import numpy as np
    validate_table(pages)
    assert len(query) == 2048 and len(physical_keys) == len(physical_values) == CONTEXT * 512
    end = position + 1 - int(omit_current) + int(include_future)
    if not 0 < end <= CONTEXT:
        raise ValueError('nonempty causal sequence')
    q = np.array([decode(x) for x in query], dtype=np.float32).reshape(16, 128)
    token_slots = [pages[token // 16] * 16 + token % 16 for token in range(end)]
    output = []
    for head in range(16):
        kv = head // 4 if kv_mapping is None else kv_mapping(head)
        if type(kv) is not int or not 0 <= kv < 4:
            raise ValueError('KV head')
        keys = np.array([[decode(int(physical_keys[physical * 512 + kv * 128 + dim]))
            for dim in range(128)] for physical in token_slots], dtype=np.float32)
        values = np.array([[decode(int(physical_values[physical * 512 + kv * 128 + dim]))
            for dim in range(128)] for physical in token_slots], dtype=np.float32)
        product = keys * q[head]
        partial = product[:, :64] + product[:, 64:]
        for distance in (1, 2, 4, 8, 16, 32):
            partial = partial + partial[:, np.arange(64) ^ distance]
        scores = partial * np.float32(SCALE)
        if not np.isfinite(scores).all():
            raise ValueError('nonfinite modeled score')
        maximum, denominator = scores[0].copy(), np.ones(64, dtype=np.float32)
        lo, hi = values[0, :64].copy(), values[0, 64:].copy()
        for token in range(1, end):
            next_max = np.maximum(maximum, scores[token])
            previous = np.array([math.exp(float(x)) for x in maximum - next_max], dtype=np.float32)
            current = np.array([math.exp(float(x)) for x in scores[token] - next_max], dtype=np.float32)
            denominator = denominator * previous + current
            lo = lo * previous + values[token, :64] * current
            hi = hi * previous + values[token, 64:] * current
            maximum = next_max
            if not all(np.isfinite(x).all() for x in (denominator, lo, hi)) or not (denominator > 0).all():
                raise ValueError('nonfinite modeled recurrence')
        output.extend(bf16(float(value)) for value in np.concatenate((lo / denominator, hi / denominator)))
    return output


def write_file(destination, name, raw):
    path = destination / name
    with path.open('xb') as stream:
        stream.write(raw)
    return dict(file=name, bytes=len(raw), sha256=digest(raw))


def json_bytes(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()


def make_case(out, label, rank, position, query, keys, values, recipe):
    expected, maxima = dense_reference(query, keys, values, position)
    pages = table()
    physical_keys, physical_values = [physical_cache(words, position, pages) for words in (keys, values)]
    modeled = cpu_wave_model(query, physical_keys, physical_values, position, pages)
    metrics = compare(modeled, expected, maxima, position)
    prefix = f'{label}-rank{rank}-pos{position}'
    row = dict(label=label, rank=rank, position=position, token_count=position + 1,
        recipe=recipe, metadata=[position] + pages, future_poison_word=POISON,
        logical_key_sha256=digest(pack(keys)), logical_value_sha256=digest(pack(values)),
        physical_key_sha256=digest(pack(physical_keys)), physical_value_sha256=digest(pack(physical_values)),
        query_sha256=digest(pack(query)), per_query_head_max_abs_v=maxima,
        cpu_wave_model_comparison=metrics, cpu_model_is_gpu_math_proof=False,
        reference=write_file(out, prefix + '.f64', struct.pack('<2048d', *expected)),
        reference_bf16=write_file(out, prefix + '.bf16', pack([bf16(x) for x in expected])))
    write_file(out, prefix + '.json', json_bytes(row))
    return row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--qkv-dir', type=Path, required=True)
    parser.add_argument('--post-dir', type=Path, required=True)
    parser.add_argument('--genuine-dir', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert sys.byteorder == 'little' and not args.output.exists()
    qkv_manifest = pinned(args.qkv_dir / 'fixture-manifest.json', QKV_MANIFEST_SHA, 7827)
    post_raw = pinned(args.post_dir / 'report.json', POST_REPORT_SHA, 11138)
    post = json.loads(post_raw)
    assert post['positions'] == list(POSITIONS) and not post['gpu_execution']
    args.output.mkdir(parents=True, mode=0o700)
    # This immutable policy is written before generating any numerical result.
    policy = write_file(args.output, 'preregistered-policy.json', json_bytes(POLICY))
    cases = []
    for rank in (0, 1):
        for position in POSITIONS:
            current = {}
            for role in ('query', 'key', 'value'):
                name = f'rank{rank}-pos{position}-{role}.bf16'
                row = post['files'][name]
                current[role] = unpack(pinned(args.post_dir / name, row['sha256'], row['bytes']))
            keys, values = patterned_history(rank, position, current['key'], current['value'])
            recipe = dict(kind='patterned-signed-history-v214', rank=rank,
                prior_positions=[0, position], current_post_report_sha256=POST_REPORT_SHA,
                current_prefix=f'rank{rank}-pos{position}',
                pattern='key=((7*t+11*h+3*d+13*r)%61-30)/64; value=((17*t+5*h+13*d+19*r)%97-48)/32',
                scope='Authentic token785 current QKV at a synthetic position; deterministic prior history, not model decode.')
            cases.append(make_case(args.output, 'patterned', rank, position, current['query'], keys, values, recipe))
    genuine_pin = None
    if args.genuine_dir is not None:
        raw = pinned(args.genuine_dir / 'manifest.json')
        genuine = json.loads(raw)
        assert genuine['schema'] == 'ferric-genuine-first-layer-prompt-v214'
        assert genuine['token_ids'] == list(TOKENS) and genuine['token_zero_matches_retained'] is True
        assert genuine['gpu_execution'] is False and genuine['full_model_correctness'] is False
        genuine_pin = digest(raw)
        for rank in (0, 1):
            keys, values = array('H'), array('H')
            for position in range(5):
                current = {}
                for role in ('query', 'key', 'value'):
                    name = f'rank{rank}-pos{position}-{role}.bf16'
                    row = genuine['files'][name]
                    current[role] = unpack(pinned(args.genuine_dir / name, row['sha256'], row['bytes']))
                keys.extend(current['key'])
                values.extend(current['value'])
                recipe = dict(kind='genuine-first-layer-five-token-v214', manifest_sha256=genuine_pin,
                    token_ids=list(TOKENS[:position + 1]), rank=rank,
                    scope='Real prompt embeddings and first-layer weights with staged CPU QKV/Post; not later layers, generated tokens or full-model output.')
                cases.append(make_case(args.output, 'genuine', rank, position, current['query'], keys, values, recipe))
    import numpy as np
    report = dict(schema='ferric-attention-reference-v214', policy=policy,
        qkv_manifest_sha256=digest(qkv_manifest), post_report_sha256=digest(post_raw),
        genuine_manifest_sha256=genuine_pin, cases=cases,
        dimensions=dict(context=CONTEXT, query_heads=Q_HEADS, kv_heads=KV_HEADS, head_dim=HEAD_DIM, tensor_parallel=2),
        scale_f32_bits=SCALE_BITS, cpu_model='all 64 XOR lanes, separate rounded FP32 operations, host exp',
        numpy_version=np.__version__, python=sys.version, oracle='unpaged FP64 stable dot and two-pass softmax',
        gpu_execution=False, gpu_math_qualified=False, full_layer_correctness=False,
        full_model_correctness=False, performance_measured=False, adaptive_tolerance=False)
    write_file(args.output, 'report.json', json_bytes(report))
    print(json.dumps(dict(passed=True, cases=len(cases), gpu_execution=False, report_sha256=digest(json_bytes(report)))))


if __name__ == '__main__':
    main()
