"""Source-counted conditional prefix checks. No device, framework or fitted error."""
from fractions import Fraction
import math
import struct

HEAD_WIDTH = 128
HEAD_DEPTH = 129
HEAD_HALF_SUBNORMALS = 256
MAX_FIRST_CANDIDATES = 32
EPSILON_BITS = 897988541


def exact_square_sum(M, words):
    M.require(M.np.asarray(words).shape == (HEAD_WIDTH,), 'serial head width128')
    M.bf16(words)
    total = 0
    for word in words:
        magnitude = int(word) & 0x7fff
        exponent, fraction = magnitude >> 7, magnitude & 127
        units = fraction if exponent == 0 else (128 + fraction) << (exponent - 1)
        total += units * units
    return Fraction(total, 1 << 266)


def head_sum_interval(M, words):
    exact = exact_square_sum(M, words)
    center = float(exact)
    lower = max(0.0, math.nextafter(center, -math.inf))
    upper = math.nextafter(center, math.inf)
    M.require(math.isfinite(upper), 'finite exact head square sum')
    # Each path has at most one product and128 sequential additions. Count
    # all128 products and128 additions for gradual-underflow roundoff.
    relative = math.nextafter(M.gamma_up(HEAD_DEPTH, M.U32) * upper, math.inf)
    tiny = math.nextafter(HEAD_HALF_SUBNORMALS * M.HALF_SUBNORMAL /
                         math.nextafter(1.0 - HEAD_DEPTH * M.U32, -math.inf), math.inf)
    error = math.nextafter(relative + tiny, math.inf)
    lo = max(0.0, math.nextafter(lower - error, -math.inf))
    hi = math.nextafter(upper + error, math.inf)
    M.require(hi <= M.F32_MAX, 'serial head sum envelope overflows F32')
    return lo, hi


def head_inverse_interval(M, words):
    lower, upper = head_sum_interval(M, words)
    lower, upper = M._round_interval(*M._outward_pair(lower / 128, upper / 128))
    epsilon = struct.unpack('<f', struct.pack('<I', EPSILON_BITS))[0]
    lower, upper = M._round_interval(*M._outward_pair(lower + epsilon, upper + epsilon))
    M.require(lower > 0, 'positive head stabilized interval')
    lo, hi = M._round_interval(*M._outward_pair(math.sqrt(lower), math.sqrt(upper)))
    # Exactly the existing P218 prerequisite: a reviewed <=1 F32-ULP sqrt;
    # not a theorem about the selected artifact derived from these tests.
    lo = float(M.np.nextafter(M.np.float32(lo), M.np.float32(-M.np.inf)))
    hi = float(M.np.nextafter(M.np.float32(hi), M.np.float32(M.np.inf)))
    M.require(lo > 0 and math.isfinite(hi), 'head sqrt envelope')
    return M._round_interval(*M._outward_pair(1.0 / hi, 1.0 / lo))


def weighted_candidates(M, input_word, weight_word, inverse):
    x, w = M.scalar_bf16(int(input_word)), M.scalar_bf16(int(weight_word))
    if x == 0:
        return (int(input_word ^ weight_word) & 0x8000,)
    endpoints = [x * inverse[0], x * inverse[1]]
    lo, hi = M._round_interval(*M._outward_pair(min(endpoints), max(endpoints)))
    first = M._possible_narrowed(lo, hi)
    M.require(len(first) <= MAX_FIRST_CANDIDATES, 'fixed head candidate cap')
    second = set()
    for word in first:
        product = float(M.np.float32(M.scalar_bf16(word) * w))
        M.require(math.isfinite(product), 'head weight multiplication overflow')
        second.add(M.round_bf16_real(product))
    return tuple(sorted(second))


def rotate_pair(M, a_word, b_word, cosine, sine):
    a, b = M.scalar_bf16(int(a_word)), M.scalar_bf16(int(b_word))
    M.require(math.isfinite(float(cosine)) and math.isfinite(float(sine)), 'finite rotary inputs')
    f32 = M.np.float32
    ac, bs, bc, ass = f32(a * float(cosine)), f32(b * float(sine)), f32(b * float(cosine)), f32(a * float(sine))
    first, second = f32(ac - bs), f32(bc + ass)
    M.require(all(math.isfinite(float(x)) for x in (ac, bs, bc, ass, first, second)),
              'finite separately rounded RoPE operations')
    return M.round_bf16_real(float(first)), M.round_bf16_real(float(second))


def check_head_rope(M, qkv, weights, rotary, query, key, value):
    np = M.np
    M.require(np.asarray(qkv).shape == (3072,) and np.asarray(weights).shape == (256,)
              and np.asarray(rotary).shape == (128,) and np.asarray(rotary).dtype == np.dtype('<f4')
              and np.asarray(query).shape == (2048,) and np.asarray(key).shape == np.asarray(value).shape == (512,),
              'closed packed-QKV/headweights/rotary/output shapes')
    for array in (qkv, weights, query, key, value):
        M.bf16(array)
    M.require(np.isfinite(rotary).all(), 'finite rotary vector')
    M.require(np.array_equal(value, qkv[2560:3072]), 'exact current V append from QKV')
    max_candidates, max_pairs = 0, 0
    for head in range(20):
        source = qkv[head * 128:(head + 1) * 128]
        learned = weights[:128] if head < 16 else weights[128:]
        observed = query[head * 128:(head + 1) * 128] if head < 16 else key[(head - 16) * 128:(head - 15) * 128]
        inverse = head_inverse_interval(M, source)
        possible = [weighted_candidates(M, x, w, inverse) for x, w in zip(source, learned)]
        max_candidates = max(max_candidates, max(map(len, possible)))
        for lane in range(64):
            pairs = {(rotate_pair(M, a, b, rotary[lane], rotary[lane + 64]))
                     for a in possible[lane] for b in possible[lane + 64]}
            max_pairs = max(max_pairs, len(pairs))
            M.require((int(observed[lane]), int(observed[lane + 64])) in pairs,
                      'serial headnorm/staged RoPE mismatch head=%d lane=%d' % (head, lane))
    return dict(heads=20, query_words=2048, key_words=512, value_words=512,
                maximum_weighted_candidates=max_candidates, maximum_rotated_pairs=max_pairs,
                shared_inverse_correlations_overapproximated=True,
                exact_current_value_append=True)


def check_rank(M, inputs, stages, position):
    np = M.np
    M.require(type(position) is int and 0 <= position < 2304, 'causal position')
    x, norm_weight, qkv_weight, head_weight, rotary, metadata = inputs
    normalized, qkv, query, keys, values, _, _ = stages
    b16 = lambda raw: np.frombuffer(raw, dtype='<u2')
    meta = struct.unpack('<145I', metadata)
    M.require(meta[0] == position and list(meta[1:]) == [(5 * page + 7) % 144 for page in range(144)],
              'exact position/page mapping')
    slot = meta[1 + position // 16] * 16 + position % 16
    start, end = slot * 1024, (slot + 1) * 1024
    norm = M.check_norm(b16(x), b16(norm_weight), b16(normalized))
    projection = M.check_bf16_gemv(b16(qkv_weight).reshape(3072, 4096), b16(normalized), b16(qkv))
    post = check_head_rope(M, b16(qkv), b16(head_weight), np.frombuffer(rotary, dtype='<f4'),
                          b16(query), b16(keys[start:end]), b16(values[start:end]))
    return dict(normalization=norm, qkv_projection=projection, headnorm_rope_append=post,
                physical_slot=slot, conditional_on_actual_preceding_stage=True)
