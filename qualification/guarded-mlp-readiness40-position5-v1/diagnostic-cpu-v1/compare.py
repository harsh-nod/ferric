"""Conditional position-5 data diagnosis; upstream owned/native admission is required."""
import hashlib
import json
import struct

import diagnostics as D

OLD = (0, 15, 16, 39)
NEW = (0, 5, 16, 39)
COMMON = (0, 16, 39)


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(raw):
    require(type(raw) is bytes, 'original byte body')
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def normalized_pin(value, extent):
    require(type(value) is dict and set(value) == {'bytes', 'sha256'}
            and type(value['bytes']) is int and value['bytes'] == extent, 'exact extent pin')
    digest = value['sha256']
    if type(digest) is list:
        require(len(digest) == 32 and all(type(n) is int and 0 <= n <= 255 for n in digest), 'digest octets')
        digest = bytes(digest).hex()
    require(type(digest) is str and len(digest) == 64
            and all(c in '0123456789abcdef' for c in digest), 'digest encoding')
    return dict(bytes=extent, sha256=digest)


def uint(value, limit):
    require(type(value) is int and 0 <= value < limit, 'unsigned integer')
    return value


def native_records(raw, tokens, selected, payloads):
    require(type(raw) is bytes and 0 < len(raw) <= 40 * 8193 and raw.endswith(b'\n'), 'forty bounded frame lines')
    lines = raw.splitlines()
    require(len(lines) == 40 and all(0 < len(line) <= 8192 for line in lines), 'forty frame census')
    require(type(payloads) is dict and set(payloads) == set(selected)
            and all(type(p) is int for p in payloads), 'closed four native payloads')
    expected = 'readiness40' if selected == OLD else 'readiness40_position5'
    records = []
    for position, line in enumerate(lines):
        frame = json.loads(line)
        c, request = frame['completion'], frame['request']
        require(frame['schema'] == 'FerricGuardedMlpLongResponseV2' and frame['profile'] == expected
                and uint(c['position'], 40) == position and uint(c['generation'], 41) == position + 1
                and uint(c['input_token'], D.VOCABULARY) == tokens[position]
                and request['command']['token'] == tokens[position]
                and type(c['captured']) is bool and c['captured'] == (position in selected), 'native ordered authentic history/profile')
        row = dict(position=position, input_token=c['input_token'], output_token=uint(c['output_token'], D.VOCABULARY),
                   logits=normalized_pin(c['logits'], 303872), observation=normalized_pin(c['observation'], 606976))
        if position in selected:
            parts = D.split_payload(payloads[position])
            require(pin(payloads[position]) == row['observation'] and pin(parts['logits']) == row['logits']
                    and D.argmax(parts['logits']) == row['output_token'], 'native selected bytes and own argmax')
        records.append(row)
    return records


def reference_records(passes, tokens, selected, payloads):
    require(type(passes) is list and len(passes) == 2 and type(payloads) is list and len(payloads) == 2,
            'two independent reference passes')
    result = []
    for ordinal, (value, bodies) in enumerate(zip(passes, payloads), 1):
        require(type(value) is dict and set(value) == {'ordinal', 'fresh_cache', 'cases'}
                and type(value['ordinal']) is int and value['ordinal'] == ordinal and value['fresh_cache'] is True
                and type(value['cases']) is list and len(value['cases']) == 40
                and type(bodies) is dict and set(bodies) == set(selected)
                and all(type(p) is int for p in bodies), 'closed reference pass/selection')
        rows = []
        for position, case in enumerate(value['cases']):
            require(uint(case['position'], 40) == position and uint(case['generation'], 41) == position + 1
                    and uint(case['cache_length'], 41) == position + 1
                    and uint(case['input_token'], D.VOCABULARY) == tokens[position], 'reference authentic own-cache history')
            row = dict(position=position, input_token=case['input_token'], output_token=uint(case['predicted_token'], D.VOCABULARY),
                       logits=normalized_pin(case['logits'], 303872))
            if position in selected:
                parts = D.split_payload(bodies[position])
                require(pin(bodies[position]) == normalized_pin(case['payload'], 606976)
                        and pin(parts['logits']) == row['logits'] and D.argmax(parts['logits']) == row['output_token'],
                        'reference selected bytes and own argmax')
            else:
                require(case['payload'] is None, 'no unselected reference payload')
            rows.append(row)
        result.append(rows)
    require(passes[0]['cases'] == passes[1]['cases'] and payloads[0] == payloads[1], 'exact reference repeat gate')
    return result[0]


def exact_units(word):
    require(type(word) is int and 0 <= word < 65536 and word & 0x7f80 != 0x7f80, 'finite BF16 word')
    exponent, fraction = (word >> 7) & 255, word & 127
    magnitude = fraction if exponent == 0 else (128 + fraction) << (exponent - 1)
    return -magnitude if word & 0x8000 else magnitude


def logit_diagnostic(raw):
    words = D.words(raw, D.VOCABULARY)
    units = [exact_units(word) for word in words]
    order = sorted(range(len(words)), key=lambda token: (-units[token], token))
    def row(token):
        return dict(token=token, rank=order.index(token) + 1, bf16_word=words[token],
                    value=D.scalar(words[token]), signed_units_2_pow_minus133=units[token])
    return dict(top_two=[row(token) for token in order[:2]],
                top_two_margin_units_2_pow_minus133=units[order[0]] - units[order[1]],
                tie_break='lowest token ID among equal finite values; signed zeros equal',
                targets=[row(2), row(9112)],
                token2_minus_token9112_units_2_pow_minus133=units[2] - units[9112])


def compare(tokens_raw, old_native_raw, new_native_raw, old_reference, new_reference,
            old_native_payloads, new_native_payloads, old_reference_payloads, new_reference_payloads):
    """Bodies must first pass the separately pinned original/diagnostic admission tools.

    This pure function authenticates byte consistency, not receipts, model identity,
    process custody or native success. It does not read files or launch processes.
    """
    require(type(tokens_raw) is bytes and len(tokens_raw) == 8192, 'full 2048-token prompt body')
    tokens = list(struct.unpack('<2048I', tokens_raw))
    require(all(t < D.VOCABULARY for t in tokens), 'all authentic input tokens in vocabulary')
    old_n = native_records(old_native_raw, tokens, OLD, old_native_payloads)
    new_n = native_records(new_native_raw, tokens, NEW, new_native_payloads)
    old_r = reference_records(old_reference, tokens, OLD, old_reference_payloads)
    new_r = reference_records(new_reference, tokens, NEW, new_reference_payloads)
    require(old_n == new_n, 'all40 native input/output/logit/observation pins unchanged')
    require(old_r == new_r, 'all40 reference input/output/logit pins unchanged')
    for position in COMMON:
        require(old_native_payloads[position] == new_native_payloads[position], 'common native full payload bytes unchanged')
        require(old_reference_payloads[0][position] == new_reference_payloads[0][position], 'common reference full payload bytes unchanged')
    left, right = D.split_payload(new_reference_payloads[0][5]), D.split_payload(new_native_payloads[5])
    metrics = {name: D.compare_tensor(raw, right[name]) for name, raw in left.items()}
    differing = [layer for layer in range(36) if left['layer%d-hidden' % layer] != right['layer%d-hidden' % layer]]
    return dict(schema='ferric-readiness40-position5-conditional-diagnostic-v1', position=5, input_token=tokens[5],
                original_native_frames=pin(old_native_raw), diagnostic_native_frames=pin(new_native_raw), prompt=pin(tokens_raw),
                all40_native_pins_unchanged=True, all40_reference_pins_unchanged=True, common_payloads_equal=list(COMMON),
                native=logit_diagnostic(right['logits']), reference=logit_diagnostic(left['logits']),
                tensors=metrics, first_nonidentical_layer_hidden=differing[0] if differing else None,
                note='First byte difference is not by itself an arithmetic bug; association can differ.',
                receipt_authentication=False, native_structural_admission=False, numerical_acceptance=False,
                acceptance_threshold=None, full_model_acceptance=False, performance_claim=False)
