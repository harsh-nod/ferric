"""Own-input conditional attention checks using an unchanged historical policy."""
from decimal import Decimal, localcontext
import struct

import attention_reference as A
import head as H

require = H.require
TARGETS = ((2, 1, 1455, 0xbb36, 0xbb37), (2, 1, 1820, 0xb50b, 0xb50c),
           (3, 1, 216, 0xb96a, 0xb969), (3, 1, 1804, 0xb97b, 0xb97a),
           (4, 1, 1334, 0xb3cf, 0xb3d5), (5, 1, 1901, 0xb79e, 0xb79d))


def normalized(row):
    H.require(type(row) is dict and type(row.get('bytes')) is int and row['bytes'] >= 0,
              'data pin extent')
    digest = row.get('sha256')
    if type(digest) is list:
        H.require(len(digest) == 32 and all(type(x) is int and 0 <= x <= 255 for x in digest),
                  'digest octets')
        digest = bytes(digest).hex()
    H.require(type(digest) is str and len(digest) == 64
              and all(c in '0123456789abcdef' for c in digest), 'SHA256')
    return dict(bytes=row['bytes'], sha256=digest)


def envelope(raw, magic):
    H.require(type(raw) is bytes and 16 <= len(raw) <= 2 << 20 and raw[:8] == magic,
              'bounded captured sidecar envelope')
    header_bytes, payload_bytes = struct.unpack_from('<II', raw, 8)
    H.require(2 <= header_bytes <= 128 << 10 and 16 + header_bytes + payload_bytes == len(raw),
              'complete sidecar extent')
    header = H.parse(raw[16:16 + header_bytes])
    payload = raw[16 + header_bytes:]
    expected = (dict(bytes=header['payload_bytes'], sha256=header['payload_sha256'])
                if magic == b'FCAP061\0' else header['payload'])
    H.require(H.pin(payload) == normalized(expected), 'entire sidecar payload hash')
    H.require(len(header['captures']) == 6
              and [c['position'] for c in header['captures']] == list(range(6)), 'six ordered positions')
    return header, payload


def cache_rows(raw, position, rank):
    """Byte permutation only: framework head/token/channel to native token/head/channel."""
    require(type(position) is int and 0 <= position < 6 and type(rank) is int and rank in (0,1)
            and type(raw) is bytes and len(raw) == 8 * (position + 1) * 256, 'closed cache dimensions')
    return b''.join(raw[(head * (position+1) + token)*256:(head * (position+1) + token+1)*256]
                    for token in range(position+1) for head in range(rank*4, rank*4+4))


def extract(native_raw, reference_raw, repeat_raw, comparison):
    nh, np = envelope(native_raw, b'FCAP061\0')
    fh, fp = envelope(reference_raw, b'FREF061\0')
    rh, rp = envelope(repeat_raw, b'FREF061\0')
    require(fh['ordinal'] == 1 and rh['ordinal'] == 2 and fp == rp
            and fh['captures'] == rh['captures'], 'genuine repeated framework stages')
    metrics = comparison['checks']['diagnostic']['comparable_rows']
    require(len(metrics) == 204 and len({r['id'] for r in metrics}) == 204, 'original comparison roster')
    by_id = {r['id']: r for r in metrics}
    cases, differences, base = [], [], 0
    for nc, fc in zip(nh['captures'], fh['captures']):
        position = nc['position']
        body = np[base:base + nc['payload_bytes']]
        base += len(body)
        require(H.pin(body) == normalized(dict(bytes=nc['payload_bytes'], sha256=nc['payload_sha256']))
                and nc['generation'] == position + 1 and nc['layer'] == 0, 'native scope and payload')
        nr, fr = {}, {}
        for part in nc['parts']:
            key = (part['rank'], part['role'])
            raw = body[part['offset']:part['offset'] + part['bytes']]
            require(key not in nr and H.pin(raw) == normalized(part), 'unique native part pin')
            nr[key] = raw
        for part in fc['parts']:
            raw = fp[part['offset']:part['offset'] + part['bytes']]
            require(part['name'] not in fr and H.pin(raw) == normalized(part), 'unique reference part pin')
            fr[part['name']] = raw
        for rank in (0, 1):
            native = {role: nr[rank, role] for role in ('query', 'used_key', 'used_value', 'attention')}
            reference = dict(query=fr['rotary-q'][rank * 4096:(rank + 1) * 4096],
                used_key=cache_rows(fr['cache-key'], position, rank),
                used_value=cache_rows(fr['cache-value'], position, rank),
                attention=fr['attention-output'][rank * 4096:(rank + 1) * 4096])
            for role in native:
                row = by_id['%d:%d:%s' % (position, rank, role)]
                require(row['native'] == H.pin(native[role]) and row['reference'] == H.pin(reference[role]),
                        'actual full comparison tensor pin')
            same = all(native[k] == reference[k] for k in ('query', 'used_key', 'used_value'))
            require(by_id['%d:%d:attention' % (position, rank)]['same_captured_inputs'] is same,
                    'recorded actual attention prerequisites')
            nw, rw = H.words(native['attention'], 2048), H.words(reference['attention'], 2048)
            found = [(position, rank, i, a, b) for i, (a, b) in enumerate(zip(nw, rw)) if a != b]
            require(len(found) == by_id['%d:%d:attention' % (position, rank)]['different_words'],
                    'whole attention difference census')
            require(not found or same, 'six diagnostic differences must have same captured inputs')
            differences.extend(found)
            cases.append(dict(position=position, rank=rank, same_captured_inputs=same,
                native=native, framework=reference))
    require(base == len(np) and tuple(differences) == TARGETS, 'exact six observed attention differences')
    return cases


def state_words(value, position):
    require(type(position) is int and 0 <= position < 6 and type(value) is dict
            and set(value) == {'query', 'used_key', 'used_value', 'attention'}, 'closed own-input attention state')
    return (H.words(value['query'], 2048), H.words(value['used_key'], (position + 1) * 512),
            H.words(value['used_value'], (position + 1) * 512), H.words(value['attention'], 2048))


def conditional(value, position):
    query, keys, values, observed = state_words(value, position)
    expected, maxima = A.dense_reference(query, keys, values, position)
    error, metrics = None, None
    try:
        metrics = A.compare(observed, expected, maxima, position)
    except ValueError as failure:
        error = str(failure)
    return dict(passed=error is None, error=error, metrics=metrics,
        input_pins={k: H.pin(v) for k, v in value.items()}, per_query_head_max_abs_v=maxima,
        reference_f64=H.pin(struct.pack('<2048d', *expected)),
        reference_bf16=H.pin(A.pack([A.bf16(x) for x in expected]))), expected


def decimal_bf16(value):
    require(type(value) is Decimal and value.is_finite(), 'finite Decimal output')
    numerator, denominator = value.copy_abs().as_integer_ratio()
    scaled = numerator << 133
    require(scaled <= H.units(0x7f7f) * denominator, 'finite BF16 diagnostic range')
    low, high = 0, 0x7f7f
    while low + 1 < high:
        middle = (low + high) // 2
        if H.units(middle) * denominator <= scaled:
            low = middle
        else:
            high = middle
    left = scaled - H.units(low) * denominator
    right = H.units(high) * denominator - scaled
    chosen = low if left < right or (left == right and low % 2 == 0) else high
    return chosen | (0x8000 if value.is_signed() else 0)


def high_precision_scalar(value, position, index, precision):
    query, keys, values, _ = state_words(value, position)
    require(type(index) is int and 0 <= index < 2048 and type(precision) is int
            and precision in (80, 160), 'bounded scalar diagnostic')
    head, channel = divmod(index, 128)
    kv = head * 4 // 16
    exact_scores = [sum(H.units(query[head * 128 + d]) * H.units(keys[token * 512 + kv * 128 + d])
                        for d in range(128)) for token in range(position + 1)]
    scale_num, scale_den = A.SCALE.as_integer_ratio()
    with localcontext() as context:
        context.prec = precision
        scale = Decimal(scale_num) / Decimal(scale_den)
        scores = [Decimal(v) / Decimal(2 ** 266) * scale for v in exact_scores]
        maximum = max(scores)
        weights = [(score - maximum).exp() for score in scores]
        causal_values = [Decimal(H.units(values[token * 512 + kv * 128 + channel])) / Decimal(2 ** 133)
                         for token in range(position + 1)]
        result = sum((w * v for w, v in zip(weights, causal_values)), Decimal(0)) / sum(weights, Decimal(0))
        require(result.is_finite(), 'finite high precision softmax result')
        return dict(decimal_precision=precision, value_decimal=str(result), direct_bf16_word=decimal_bf16(result),
            native_scale_bits=A.SCALE_BITS, local_query_head=head, local_kv_head=kv, channel=channel,
            exact_qk_dot_units_2_pow_minus266=[str(x) for x in exact_scores])


def analyze(cases):
    require(type(cases) is list and [(c['position'], c['rank']) for c in cases]
            == [(p, r) for p in range(6) for r in (0, 1)], 'all twelve original attention states')
    states, selected = [], []
    for case in cases:
        row = dict(position=case['position'], rank=case['rank'], same_captured_inputs=case['same_captured_inputs'])
        for side in ('native', 'framework'):
            row[side], _ = conditional(case[side], case['position'])
        states.append(row)
    for position, rank, index, native, framework in TARGETS:
        case = cases[2 * position + rank]
        require(case['same_captured_inputs'] is True
                and all(case['native'][k] == case['framework'][k] for k in ('query', 'used_key', 'used_value')),
                'same inputs for precision diagnostics')
        require(H.words(case['native']['attention'], 2048)[index] == native
                and H.words(case['framework']['attention'], 2048)[index] == framework,
                'observed diagnostic words remain the original six')
        low = high_precision_scalar(case['native'], position, index, 80)
        high = high_precision_scalar(case['native'], position, index, 160)
        selected.append(dict(position=position, rank=rank, index=index, native_word=native,
            framework_word=framework, precision80=low, precision160=high,
            rounded_word_stable=low['direct_bf16_word'] == high['direct_bf16_word'],
            rigorous_interval_or_correct_rounding_proof=False))
    return dict(schema='ferric-causal-attention-own-input-conditional-v1', states=states,
        state_count_per_side=12, outputs_per_side=24576,
        all_conditional_checks_passed=all(r[s]['passed'] for r in states for s in ('native', 'framework')),
        policy=A.POLICY, policy_changed=False, reference_scale_bits=A.SCALE_BITS,
        higher_precision_diagnostics=selected, precision_diagnostics_are_acceptance=False,
        framework_internal_reduction_emulated=False, current_native_exp_ulp_bound_proven=False,
        full_capsule_revalidated=False, current_kernel_full_numerical_contract_proven=False,
        numerical_acceptance=False, full_model_acceptance=False, performance_claim=False,
        semantic_bug_claimed=False, arithmetic_patch_recommended=False)
