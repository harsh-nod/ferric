"""Four exact real BF16 dots, not an MFMA or framework accumulation emulator."""
import hashlib
import json
import struct

WIDTH, VOCAB = 4096, 151936
TOKENS = (2, 9112)
PAYLOAD_BYTES, CONTROL_BYTES = 606976, 242824
FINAL_OFFSET, LOGIT_OFFSET = 36 * 8192, 37 * 8192


def require(ok, message):
    if not ok:
        raise ValueError(message)


def parse(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    def invalid(_):
        raise ValueError('nonfinite JSON value')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def units(word):
    require(type(word) is int and 0 <= word <= 65535, 'BF16 word')
    exponent, fraction = (word >> 7) & 255, word & 127
    require(exponent != 255, 'finite BF16 operand')
    value = fraction if exponent == 0 else (128 + fraction) << (exponent - 1)
    return -value if word & 32768 else value


def words(raw, count):
    require(type(count) is int and count >= 0 and type(raw) is bytes
            and len(raw) == count * 2, 'BF16 extent')
    result = struct.unpack('<%dH' % count, raw)
    for word in result:
        units(word)
    return result


def exact_dot(left, right):
    require(len(left) == len(right) == WIDTH, '4096-term dot')
    return sum(units(a) * units(b) for a, b in zip(left, right))


def decimal_dyadic(value, power=266):
    require(type(value) is int and type(power) is int and 0 <= power <= 266, 'bounded exact dyadic')
    digits = str(abs(value) * 5 ** power).rjust(power + 1, '0')
    if power:
        digits = (digits[:-power] + '.' + digits[-power:]).rstrip('0').rstrip('.')
    return ('-' if value < 0 else '') + digits


def nearest_bf16(value):
    """RNE from an exact signed integer in units 2^-266, including overflow."""
    require(type(value) is int, 'exact integer dot')
    sign, magnitude = (32768 if value < 0 else 0), abs(value)
    maximum = units(0x7f7f) << 133
    overflow_midpoint = maximum + (1 << (252 + 133))
    if magnitude >= overflow_midpoint:
        return sign | 0x7f80
    if magnitude >= maximum:
        return sign | 0x7f7f
    low, high = 0, 0x7f7f
    while low + 1 < high:
        middle = (low + high) // 2
        if units(middle) << 133 <= magnitude:
            low = middle
        else:
            high = middle
    distance_low = magnitude - (units(low) << 133)
    distance_high = (units(high) << 133) - magnitude
    chosen = low if distance_low < distance_high else high
    if distance_low == distance_high:
        chosen = low if low % 2 == 0 else high
    return sign | chosen


def tensor_layout(header, header_bytes, file_bytes):
    require(type(header) is dict and set(header) in
            ({'lm_head.weight'}, {'lm_head.weight', '__metadata__'}), 'closed last-shard header')
    if '__metadata__' in header:
        metadata = header['__metadata__']
        require(type(metadata) is dict and all(type(k) is str and type(v) is str
                for k, v in metadata.items()), 'safetensors metadata')
    value = header['lm_head.weight']
    require(type(value) is dict and set(value) == {'dtype', 'shape', 'data_offsets'}, 'tensor header fields')
    require(value['dtype'] == 'BF16' and type(value['shape']) is list
            and all(type(n) is int for n in value['shape'])
            and value['shape'] == [VOCAB, WIDTH], 'original row-major BF16 head')
    offsets = value['data_offsets']
    require(type(offsets) is list and len(offsets) == 2 and all(type(n) is int for n in offsets)
            and offsets == [0, VOCAB * WIDTH * 2], 'complete original head tensor extent')
    require(type(header_bytes) is int and 2 <= header_bytes <= 1 << 20
            and type(file_bytes) is int and 8 + header_bytes + offsets[1] == file_bytes,
            'bounded header and exact no-hole/no-trailer shard')
    start = 8 + header_bytes
    return start, {token: (start + token * WIDTH * 2, WIDTH * 2) for token in TOKENS}


def payload(raw):
    require(type(raw) is bytes and len(raw) == PAYLOAD_BYTES, 'full selected payload extent')
    words(raw, PAYLOAD_BYTES // 2)
    final = raw[FINAL_OFFSET:LOGIT_OFFSET]
    logits = raw[LOGIT_OFFSET:]
    observed = {token: struct.unpack_from('<H', logits, 2 * token)[0] for token in TOKENS}
    return words(final, WIDTH), observed, dict(final_norm=pin(final), logits=pin(logits), payload=pin(raw))


def analyze(native, reference, rows):
    require(type(rows) is dict and set(rows) == set(TOKENS), 'exact two head rows')
    weights = {token: words(rows[token], WIDTH) for token in TOKENS}
    inputs = dict(native=payload(native), reference=payload(reference))
    midpoint = (units(0x419d) + units(0x419e)) << 132
    outputs, exact = {}, {}
    for name, (vector, observed, pins) in inputs.items():
        dots = {}
        for token in TOKENS:
            value = exact_dot(weights[token], vector)
            rounded = nearest_bf16(value)
            dots[str(token)] = dict(exact_units_2_pow_minus266=str(value), exact_decimal=decimal_dyadic(value),
                nearest_bf16_word=rounded, nearest_bf16_is_finite=(rounded & 0x7f80) != 0x7f80,
                observed_bf16_word=observed[token], observed_matches_nearest=observed[token] == rounded,
                observed_decimal=decimal_dyadic(units(observed[token]), 133),
                observed_minus_exact_units_2_pow_minus266=str((units(observed[token]) << 133) - value),
                exact_minus_19_6875_midpoint_units_2_pow_minus266=str(value - midpoint))
        exact[name] = {token: int(dots[str(token)]['exact_units_2_pow_minus266']) for token in TOKENS}
        margin = exact[name][2] - exact[name][9112]
        outputs[name] = dict(input_pins=pins, dots=dots,
            exact_token2_minus_token9112_units_2_pow_minus266=str(margin),
            exact_token2_minus_token9112_decimal=decimal_dyadic(margin),
            observed_token2_minus_token9112_units_2_pow_minus266=str((units(observed[2]) - units(observed[9112])) << 133),
            exact_dot_winner_among_two=min(TOKENS, key=lambda token: (-exact[name][token], token)))
    delta = {str(token): str(exact['native'][token] - exact['reference'][token]) for token in TOKENS}
    vector_delta = [units(a) - units(b) for a, b in zip(inputs['native'][0], inputs['reference'][0])]
    for token in TOKENS:
        require(int(delta[str(token)]) == sum(units(w) * d for w, d in zip(weights[token], vector_delta)),
                'independent exact input-delta decomposition')
    return dict(schema='ferric-position5-exact-head-diagnostic-v1', position=5, tokens=list(TOKENS),
        terms_per_dot=WIDTH, dot_count=4, exact_product_unit='2^-266',
        midpoint_units_2_pow_minus266=str(midpoint), inputs=outputs,
        row_pins={str(token): pin(rows[token]) for token in TOKENS},
        native_minus_reference_exact_units_2_pow_minus266=delta,
        margin_change_units_2_pow_minus266=str(int(delta['2']) - int(delta['9112'])),
        mfma_emulation=False, framework_accumulation_emulation=False, tolerance_applied=False,
        numerical_acceptance=False, full_model_acceptance=False, performance_claim=False)
