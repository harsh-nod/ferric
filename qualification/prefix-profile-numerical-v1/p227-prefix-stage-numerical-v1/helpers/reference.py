"""Independent, preregistered MLP stage qualification; never a GPU launcher.

FP64 dots do not reproduce the device's lane tree. Later stages are explicitly
conditional on actual accepted preceding arrays. The final residual uses the
already frozen integer-bit oracle, not FP64 additions masquerading as exact F32.
"""
import hashlib
import json
import math
from pathlib import Path
import struct
import sys

import numpy as np

from extract import (MODEL, REVISION, SOURCE_SHA, HEADER_SHA, INDEX_SHA, canonical,
                     file_pin, json_bytes, pinned_bytes, require)

ORACLE_SHA = '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3'
U32, U64, HALF_SUBNORMAL = 2.0**-24, 2.0**-53, 2.0**-150
F32_MAX = float(np.finfo(np.float32).max)


def words(values, count, bits=16):
    require(isinstance(values, list) and len(values) == count and
            all(type(x) is int and 0 <= x < 1 << bits for x in values), 'exact word array')
    array = np.asarray(values, dtype='<u2' if bits == 16 else '<u4')
    mask = 0x7f80 if bits == 16 else 0x7f800000
    require(not np.any((array & mask) == mask), 'nonfinite input/output word')
    return array


def bf16(array):
    array = np.asarray(array)
    require(sys.byteorder == 'little', 'little-endian reference host required')
    require(array.dtype == np.dtype('<u2'), 'BF16 storage must be little-endian uint16')
    require(not np.any((array & 0x7f80) == 0x7f80), 'nonfinite BF16')
    return (array.astype(np.uint32) << 16).view(np.float32)


def scalar_bf16(word):
    require(type(word) is int and 0 <= word <= 65535 and word & 0x7f80 != 0x7f80,
            'finite BF16 word')
    return struct.unpack('<f', struct.pack('<I', word << 16))[0]


def round_bf16_real(value):
    """Round one finite binary64 directly to BF16, avoiding double rounding."""
    value = float(value)
    require(math.isfinite(value), 'nonfinite real before BF16')
    sign = 0x8000 if math.copysign(1.0, value) < 0 else 0
    if value == 0:
        return sign
    numerator, denominator = abs(value).as_integer_ratio()
    quantum = max(-133, math.frexp(abs(value))[1] - 8)
    if quantum < 0:
        numerator <<= -quantum
    else:
        denominator <<= quantum
    significand, remainder = divmod(numerator, denominator)
    if 2 * remainder > denominator or (2 * remainder == denominator and significand & 1):
        significand += 1
    if quantum == -133 and significand < 128:
        return sign | significand
    if significand == 256:
        significand >>= 1
        quantum += 1
    exponent = quantum + 134
    require(1 <= exponent <= 254, 'BF16 narrowing overflow')
    return sign | (exponent << 7) | (significand - 128)


def ordered(word):
    return 0x8000 - (word & 0x7fff) if word & 0x8000 else 0x8000 + word


def cell(word):
    """Finite BF16 RNE cell, including even-tie ownership and signed-zero side."""
    value = scalar_bf16(word)
    magnitude = word & 0x7fff
    if magnitude == 0:
        half = 2.0**-134
        return (-half, -0.0, True) if word & 0x8000 else (0.0, half, True)
    previous = scalar_bf16(magnitude - 1)
    following = 2.0**128 if magnitude == 0x7f7f else scalar_bf16(magnitude + 1)
    lower = (previous + abs(value)) * 0.5
    upper = (abs(value) + following) * 0.5
    if word & 0x8000:
        return -upper, -lower, (word & 1) == 0
    return lower, upper, (word & 1) == 0


def cell_intersects(word, lower, upper):
    require(math.isfinite(lower) and math.isfinite(upper) and lower <= upper, 'finite interval')
    left, right, even = cell(word)
    lo, hi = max(left, lower), min(right, upper)
    if lo < hi:
        return True
    if lo > hi:
        return False
    return even or (left < lo < right)


def gamma_up(n, unit):
    require(type(n) is int and n > 0 and n * unit < 1, 'gamma domain')
    return math.nextafter(n * unit / math.nextafter(1.0 - n * unit, -math.inf), math.inf)


def error_envelope(absolute_sum, k, depth):
    absolute_sum = np.asarray(absolute_sum, dtype=np.float64)
    require(np.isfinite(absolute_sum).all() and (absolute_sum >= 0).all(), 'absolute product sum')
    g32, g64 = gamma_up(depth, U32), gamma_up(k, U64)
    upper = np.nextafter(absolute_sum / math.nextafter(1.0 - g64, -math.inf), np.inf)
    relative = np.nextafter(math.nextafter(g32 + g64, math.inf) * upper, np.inf)
    # K products + K lane additions + 63 wave-tree additions, each with a
    # half-subnormal absolute error; gamma depth inflates propagated roundoff.
    underflow = math.nextafter((2 * k + 63) * HALF_SUBNORMAL /
                              math.nextafter(1.0 - depth * U32, -math.inf), math.inf)
    return np.nextafter(relative + underflow, np.inf)


def gemv_reference(weight_words, input_words, depth):
    weights = np.asarray(weight_words)
    inputs = np.asarray(input_words)
    require(weights.dtype == inputs.dtype == np.dtype('<u2') and weights.ndim == 2 and
            inputs.ndim == 1 and weights.shape[1] == inputs.size and inputs.size > 0,
            'GEMV shape/type')
    k = inputs.size
    require(k in (4096, 6144) and depth == (71 if k == 4096 else 103), 'closed GEMV policy')
    a = bf16(inputs).astype(np.float64)
    exact, bound = [], []
    for first in range(0, weights.shape[0], 64):
        w = bf16(weights[first:first + 64]).astype(np.float64)
        products = w * a
        center = products.sum(axis=1, dtype=np.float64)
        absolute = np.abs(products).sum(axis=1, dtype=np.float64)
        exact.append(center)
        bound.append(error_envelope(absolute, k, depth))
    exact, bound = np.concatenate(exact), np.concatenate(bound)
    require(np.isfinite(exact).all() and np.isfinite(bound).all(), 'GEMV reference finite')
    return exact, bound


def check_bf16_gemv(weight_words, input_words, observed):
    require(np.asarray(weight_words).shape[1] == 4096, 'gate/up K4096')
    require(np.asarray(observed).shape == (np.asarray(weight_words).shape[0],), 'gate/up output shape')
    bf16(observed)
    center, bound = gemv_reference(weight_words, input_words, 71)
    failures = []
    for i, word in enumerate(observed):
        lo = math.nextafter(float(center[i] - bound[i]), -math.inf)
        hi = math.nextafter(float(center[i] + bound[i]), math.inf)
        if not cell_intersects(int(word), lo, hi):
            failures.append(i)
    require(not failures, 'BF16 GEMV envelope mismatch at ' + repr(failures[:8]))
    return {'elements': len(observed), 'mismatches': 0, 'max_bound': float(bound.max())}


def check_f32_gemv(weight_words, input_words, observed_bits):
    require(np.asarray(weight_words).shape[1] == 6144, 'down K6144')
    require(np.asarray(observed_bits).dtype == np.dtype('<u4') and
            np.asarray(observed_bits).shape == (np.asarray(weight_words).shape[0],), 'down F32 bits')
    observed = observed_bits.view(np.float32).astype(np.float64)
    require(np.isfinite(observed).all(), 'down finite FP32')
    center, bound = gemv_reference(weight_words, input_words, 103)
    error = np.abs(observed - center)
    upper = np.where(error == 0, 0, np.nextafter(error, np.inf))
    require((upper <= bound).all(), 'FP32 down envelope mismatch')
    return {'elements': len(observed_bits), 'mismatches': 0,
            'max_absolute_error': float(error.max()), 'max_bound_ratio': float((upper / bound).max())}


def _round_interval(lower, upper):
    require(math.isfinite(lower) and math.isfinite(upper) and lower <= upper and
            -F32_MAX <= lower <= F32_MAX and -F32_MAX <= upper <= F32_MAX,
            'FP32 intermediate interval overflow')
    return float(np.float32(lower)), float(np.float32(upper))


def _outward_pair(lower, upper):
    return math.nextafter(lower, -math.inf), math.nextafter(upper, math.inf)


def norm_inverse_interval(input_words):
    require(np.asarray(input_words).shape == (4096,), 'norm width4096')
    x = bf16(input_words).astype(np.float64)
    squares = x * x
    total = float(squares.sum(dtype=np.float64))
    error = float(error_envelope(np.array([total]), 4096, 71)[0])
    lower = max(0.0, math.nextafter(total - error, -math.inf))
    upper = math.nextafter(total + error, math.inf)
    lower, upper = _round_interval(*_outward_pair(lower / 4096, upper / 4096))
    epsilon = struct.unpack('<f', struct.pack('<I', 897988541))[0]
    lower, upper = _round_interval(*_outward_pair(lower + epsilon, upper + epsilon))
    require(lower > 0, 'norm positive stabilized interval')
    root_lo, root_hi = _outward_pair(math.sqrt(lower), math.sqrt(upper))
    root_lo, root_hi = _round_interval(root_lo, root_hi)
    # Preregistered artifact obligation: sqrt differs by at most one F32 ULP
    # from correctly rounded sqrt. This is not inferred from host math tests.
    root_lo = float(np.nextafter(np.float32(root_lo), np.float32(-np.inf)))
    root_hi = float(np.nextafter(np.float32(root_hi), np.float32(np.inf)))
    require(root_lo > 0 and math.isfinite(root_hi), 'norm sqrt interval')
    return _round_interval(*_outward_pair(1.0 / root_hi, 1.0 / root_lo))


def _possible_narrowed(lower, upper):
    left, right = round_bf16_real(lower), round_bf16_real(upper)
    low_order, high_order = ordered(left), ordered(right)
    require(high_order >= low_order and high_order - low_order <= 30,
            'norm uncertainty exceeds preregistered candidate cap')
    candidates = []
    for number in range(max(1, low_order - 1), min(65534, high_order + 1) + 1):
        word = 0x8000 | (0x8000 - number) if number < 0x8000 else number - 0x8000
        if word & 0x7f80 != 0x7f80 and cell_intersects(word, lower, upper):
            candidates.append(word)
    # Ordered numbering merges +/-zero. Keep both only when the interval can
    # contain zero; check_norm handles exact-zero inputs separately.
    if lower <= 0 <= upper and 0x8000 not in candidates:
        candidates.append(0x8000)
    require(0 < len(candidates) <= 32, 'norm candidate set')
    return candidates


def check_norm(input_words, weight_words, observed):
    require(np.asarray(weight_words).shape == np.asarray(observed).shape == (4096,), 'norm arrays')
    x, w = bf16(input_words), bf16(weight_words)
    bf16(observed)
    inverse_low, inverse_high = norm_inverse_interval(input_words)
    failures, maximum = [], 0
    for index in range(4096):
        xi, wi = float(x[index]), float(w[index])
        if xi == 0:
            allowed = {((int(input_words[index]) ^ int(weight_words[index])) & 0x8000)}
        else:
            endpoints = (xi * inverse_low, xi * inverse_high)
            lower, upper = _round_interval(*_outward_pair(min(endpoints), max(endpoints)))
            normalized = _possible_narrowed(lower, upper)
            maximum = max(maximum, len(normalized))
            allowed = {round_bf16_real(float(np.float32(scalar_bf16(word) * wi)))
                       for word in normalized}
        if int(observed[index]) not in allowed:
            failures.append(index)
    require(not failures, 'dual-round RMSNorm mismatch at ' + repr(failures[:8]))
    return {'elements': 4096, 'mismatches': 0, 'maximum_first_narrow_candidates': maximum,
            'inverse_interval': [inverse_low, inverse_high]}


def norm_diagnostic(input_words, weight_words):
    """Supporting CPU F32 lane-tree model, not the independent acceptance oracle."""
    x, w = bf16(input_words), bf16(weight_words)
    require(x.shape == w.shape == (4096,), 'norm diagnostic width')
    partial = np.zeros(64, dtype=np.float32)
    for step in range(64):
        segment = x[step * 64:(step + 1) * 64]
        partial = np.add(partial, np.multiply(segment, segment, dtype=np.float32), dtype=np.float32)
    lanes = np.arange(64)
    for offset in (1, 2, 4, 8, 16, 32):
        partial = np.add(partial, partial[lanes ^ offset], dtype=np.float32)
    epsilon = np.asarray([897988541], dtype=np.uint32).view(np.float32)[0]
    denominator = np.sqrt(np.add(np.divide(partial[0], np.float32(4096)), epsilon), dtype=np.float32)
    inverse = np.divide(np.float32(1), denominator, dtype=np.float32)
    first = np.asarray([round_bf16_real(float(value)) for value in
                        np.multiply(x, inverse, dtype=np.float32)], dtype='<u2')
    second = np.multiply(bf16(first), w, dtype=np.float32)
    return np.asarray([round_bf16_real(float(value)) for value in second], dtype='<u2')


def swiglu_reference(gate_words, up_words):
    gate, up = bf16(gate_words), bf16(up_words)
    require(gate.shape == up.shape and gate.ndim == 1, 'SwiGLU shape')
    expected = []
    for g, u in zip(gate, up):
        g, u = float(g), float(u)
        exponential = math.exp(-abs(g))
        sigmoid = (1.0 if g >= 0 else exponential) / (1.0 + exponential)
        expected.append(round_bf16_real((g * sigmoid) * u))
    return np.asarray(expected, dtype='<u2')


def check_swiglu(gate_words, up_words, observed):
    expected = swiglu_reference(gate_words, up_words)
    require(np.asarray(observed).shape == expected.shape, 'SwiGLU output shape')
    gate, up = bf16(gate_words), bf16(up_words)
    bf16(observed)
    exact = tolerated = maximum = 0
    for index, (actual, wanted) in enumerate(zip(observed, expected)):
        distance = abs(ordered(int(actual)) - ordered(int(wanted)))
        if gate[index] == 0 or up[index] == 0:
            require(actual == wanted, 'SwiGLU signed zero')
        require(distance <= 1, 'SwiGLU exceeds preregistered one-step policy')
        exact += int(actual == wanted)
        tolerated += int(actual != wanted)
        maximum = max(maximum, distance)
    return {'elements': len(observed), 'exact': exact, 'tolerated': tolerated,
            'mismatches': 0, 'max_ordered_bf16_steps': maximum}


def load_oracle(path):
    path = canonical(path)
    raw = pinned_bytes({'path': str(path), 'bytes': path.stat().st_size, 'sha256': ORACLE_SHA}, 32768)
    namespace = {'__name__': 'frozen_p216_residual_oracle', '__file__': str(path)}
    exec(compile(raw, str(path), 'exec'), namespace)
    return namespace['residual_vector']


def unconditioned_mlp_diagnostic(residual_words, norm_weight, weight_ranks, actual_output):
    """FP64 suffix diagnostic with BF16 boundaries, no GPU intermediate inputs.

    This intentionally does not claim the wave-F32 association. It cannot make
    a failed stage pass and has no aggregate acceptance threshold.
    """
    x, weight = bf16(residual_words).astype(np.float64), bf16(norm_weight).astype(np.float64)
    epsilon = struct.unpack('<f', struct.pack('<I', 897988541))[0]
    inverse = 1.0 / math.sqrt(float((x * x).sum(dtype=np.float64)) / 4096 + epsilon)
    first = np.asarray([round_bf16_real(value * inverse) for value in x], dtype='<u2')
    normalized = np.asarray([round_bf16_real(value) for value in
                             bf16(first).astype(np.float64) * weight], dtype='<u2')
    partials, stage_hashes = [], []
    for weights_rank in weight_ranks:
        gate64, _ = gemv_reference(weights_rank['gate'], normalized, 71)
        up64, _ = gemv_reference(weights_rank['up'], normalized, 71)
        gate = np.asarray([round_bf16_real(value) for value in gate64], dtype='<u2')
        up = np.asarray([round_bf16_real(value) for value in up64], dtype='<u2')
        activation = swiglu_reference(gate, up)
        down64, _ = gemv_reference(weights_rank['down'], activation, 103)
        partials.append(down64)
        stage_hashes.append({name: hashlib.sha256(array.tobytes()).hexdigest()
                             for name, array in [('gate', gate), ('up', up), ('activation', activation),
                                                 ('down_f64', down64.astype('<f8'))]})
    final64 = (partials[0] + partials[1]) + x
    expected = np.asarray([round_bf16_real(value) for value in final64], dtype='<u2')
    error = np.abs(bf16(actual_output).astype(np.float64) - bf16(expected).astype(np.float64))
    steps = [abs(ordered(int(a)) - ordered(int(b))) for a, b in zip(actual_output, expected)]
    return {'schema': 'ferric-layer0-mlp-unconditioned-fp64-diagnostic-v218',
            'conditional_on_actual_first_residual_only': True, 'gpu_intermediate_substitution': False,
            'dot_and_final_sum_precision': 'FP64', 'bf16_boundaries_retained': True,
            'acceptance_threshold': None, 'affects_stage_acceptance': False,
            'full_attention_reference': False, 'full_model_reference': False,
            'exact_outputs': int(np.count_nonzero(expected == actual_output)),
            'different_outputs': int(np.count_nonzero(expected != actual_output)),
            'max_absolute_difference': float(error.max()), 'max_ordered_bf16_steps': max(steps),
            'normalized_sha256': hashlib.sha256(normalized.tobytes()).hexdigest(),
            'rank_stage_sha256': stage_hashes,
            'output_sha256': hashlib.sha256(expected.tobytes()).hexdigest(),
            'output_words': expected.tolist()}


def fixture(manifest_path, manifest_sha):
    manifest_path = canonical(manifest_path)
    pin = {'path': str(manifest_path), 'bytes': manifest_path.stat().st_size, 'sha256': manifest_sha}
    manifest = json_bytes(pinned_bytes(pin, 128 * 1024))
    require(manifest['schema'] == 'ferric-layer0-mlp-fixture-v218' and
            manifest['model'] == MODEL and manifest['revision'] == REVISION and
            manifest['gpu_execution'] is False and manifest['production_authority'] is False,
            'fixture identity')
    require(manifest['source']['sha256'] == SOURCE_SHA and
            manifest['source']['header_sha256'] == HEADER_SHA and
            manifest['index']['sha256'] == INDEX_SHA, 'authentic source pins')
    policy_raw = pinned_bytes(manifest['numerical_policy'], 32768)
    require(policy_raw == (Path(__file__).parent / 'policy.json').read_bytes(), 'preregistered policy mismatch')
    require([row['rank'] for row in manifest['ranks']] == [0, 1] and
            all(type(row['rank']) is int for row in manifest['ranks']), 'rank roster')
    def read(row, shape):
        size = 2 * math.prod(shape)
        require(row['shape'] == shape and row['dtype'] == 'BF16' and row['bytes'] == size,
                'fixture matrix shape/type')
        path = Path(row['path'])
        require(path.parent == manifest_path.parent, 'fixture file must be local to manifest')
        data = np.frombuffer(pinned_bytes(row, 50331648), dtype='<u2').reshape(shape)
        bf16(data)
        return data
    norm = read(manifest['post_norm'], [4096])
    ranks = [{role: read(row[role], [4096, 6144] if role == 'down' else [6144, 4096])
              for role in ('gate', 'up', 'down')} for row in manifest['ranks']]
    paths = [manifest['post_norm']['path']] + [row[role]['path'] for row in manifest['ranks']
             for role in ('gate', 'up', 'down')]
    require(len(set(paths)) == 7, 'fixture aliased paths')
    return manifest, norm, ranks


def check_observation(manifest_path, manifest_sha256, observed, oracle_path):
    """Qualify suffix numerics of an already authenticated, closed raw observation.

    Caller owns request/image/roster/prefix receipt validation. This API checks
    explicit closure/raw-status flags but cannot establish GPU provenance from
    a JSON object. Preserve the raw receipt and this result as separate evidence.
    """
    require(isinstance(observed, dict) and observed.get('completed_and_closed') is True and
            observed.get('numerical_acceptance') is False, 'closed unqualified raw observation required')
    require(isinstance(observed.get('ranks'), list) and len(observed['ranks']) == 2,
            'exact two-rank observation')
    manifest, norm_weight, weight_ranks = fixture(manifest_path, manifest_sha256)
    results, residuals, partials, outputs = [], [], [], []
    for rank, (record, weights_rank) in enumerate(zip(observed['ranks'], weight_ranks)):
        require(record.get('rank') == rank and type(record.get('rank')) is int, 'observation rank ordering')
        residual = words(record['prefix']['residual_words'], 4096)
        normalized = words(record['post_norm_words'], 4096)
        gate = words(record['gate_words'], 6144)
        up = words(record['up_words'], 6144)
        activation = words(record['activation_words'], 6144)
        down = words(record['down_partial_bits'], 4096, 32)
        final = words(record['layer_output_words'], 4096)
        result = {'rank': rank, 'norm': check_norm(residual, norm_weight, normalized),
                  'gate': check_bf16_gemv(weights_rank['gate'], normalized, gate),
                  'up': check_bf16_gemv(weights_rank['up'], normalized, up),
                  'swiglu': check_swiglu(gate, up, activation),
                  'down': check_f32_gemv(weights_rank['down'], activation, down)}
        result['actual_input_sha256'] = hashlib.sha256(residual.tobytes()).hexdigest()
        result['stage_sha256'] = {name: hashlib.sha256(array.tobytes()).hexdigest()
                                 for name, array in [('norm', normalized), ('gate', gate), ('up', up),
                                                     ('activation', activation), ('down', down), ('output', final)]}
        results.append(result)
        residuals.append(residual.tobytes())
        partials.append(down.tobytes())
        outputs.append(final.tobytes())
    require(residuals[0] == residuals[1], 'replicated first residual must match exactly')
    expected = load_oracle(oracle_path)(partials[0], partials[1], residuals[0])
    require(outputs[0] == outputs[1] == expected, 'ordered TP2 final residual mismatch')
    diagnostic = unconditioned_mlp_diagnostic(np.frombuffer(residuals[0], dtype='<u2'), norm_weight,
                                            weight_ranks, np.frombuffer(outputs[0], dtype='<u2'))
    return {'schema': 'ferric-layer0-mlp-conditional-comparison-v218', 'passed': True,
            'adaptive_tolerance': False, 'numerical_acceptance': True, 'gpu_execution_by_comparator': False,
            'production_authority': False, 'gpu_receipt_authenticated_by_comparator': False,
            'prefix_requires_separate_acceptance': True, 'independent_end_to_end_model_reference': False,
            'full_model_correctness': False, 'performance_measured': False,
            'artifact_arithmetic_prerequisites_require_separate_review': True,
            'numpy_version': np.__version__,
            'manifest_sha256': manifest_sha256, 'policy_sha256': manifest['numerical_policy']['sha256'],
            'oracle_sha256': ORACLE_SHA, 'ranks': results,
            'unconditioned_mlp_diagnostic': diagnostic,
            'final_output_sha256': hashlib.sha256(expected).hexdigest()}


def compare_cli(manifest_path, manifest_sha, observation_path, observation_sha, oracle_path, output_path):
    observation_path = canonical(observation_path)
    observation_pin = {'path': str(observation_path), 'bytes': observation_path.stat().st_size,
                       'sha256': observation_sha}
    observed = json_bytes(pinned_bytes(observation_pin, 128 * 1024**2))
    result = check_observation(manifest_path, manifest_sha, observed, oracle_path)
    result['observation'] = observation_pin
    result['comparator'] = file_pin(canonical(__file__), Path(__file__).read_bytes())
    output_path = Path(output_path)
    require(output_path.is_absolute() and output_path.parent.resolve(strict=True) == output_path.parent,
            'comparison output parent')
    raw = (json.dumps(result, indent=2, sort_keys=True) + '\n').encode()
    with output_path.open('xb') as stream:
        stream.write(raw)
    return file_pin(output_path, raw)


if __name__ == '__main__':
    require(len(sys.argv) == 7,
            'usage: reference.py MANIFEST SHA OBSERVATION SHA ORACLE NEW_RESULT_JSON')
    print(json.dumps(compare_cli(*sys.argv[1:]), sort_keys=True))
