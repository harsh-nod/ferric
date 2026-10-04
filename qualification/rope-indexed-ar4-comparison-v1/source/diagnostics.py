"""Pure BF16 diagnostics, not a candidate numerical acceptance policy."""
import math
import struct

HIDDEN = 4096
LAYERS = 36
VOCABULARY = 151936
PAYLOAD_BYTES = (LAYERS * HIDDEN + HIDDEN + VOCABULARY) * 2
INPUT_TOKENS = (9112,)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def token_inputs(mode, tokens):
    require(mode == 'autoregressive' and isinstance(tokens, list), 'four-forward own-output autoregressive mode')
    require(tuple(tokens) == INPUT_TOKENS and all(type(x) is int for x in tokens), 'closed token roster')
    return tuple(tokens)


def next_input(mode, tokens, records):
    initial = token_inputs(mode, tokens)
    require(isinstance(records, list) and len(records) < 4, 'bounded prior own records')
    expected = initial[0]
    for position, record in enumerate(records):
        require(isinstance(record, dict) and set(record) ==
                {'generation', 'position', 'input_token', 'output_token'} and
                all(type(record[key]) is int for key in record) and
                record['generation'] == position + 1 and record['position'] == position and
                record['input_token'] == expected and 0 <= record['output_token'] < VOCABULARY,
                'causal own-output trajectory')
        expected = record['output_token']
    return expected


def words(raw, elements):
    require(isinstance(raw, bytes) and len(raw) == 2 * elements, 'BF16 extent')
    result = [word for word, in struct.iter_unpack('<H', raw)]
    require(all(word & 0x7f80 != 0x7f80 for word in result), 'nonfinite BF16')
    return result


def scalar(word):
    return struct.unpack('<f', struct.pack('<I', word << 16))[0]


def ordered(word):
    return -(word & 0x7fff) if word & 0x8000 else word


def argmax(raw):
    row = words(raw, VOCABULARY)
    return max(range(VOCABULARY), key=lambda i: scalar(row[i]))


def split_payload(raw):
    require(isinstance(raw, bytes) and len(raw) == PAYLOAD_BYTES, 'forward payload extent')
    result, offset = {}, 0
    for name, count in [*(('layer%d-hidden' % layer, HIDDEN) for layer in range(LAYERS)),
                        ('final-norm', HIDDEN), ('logits', VOCABULARY)]:
        result[name] = raw[offset:offset + count * 2]
        words(result[name], count)
        offset += count * 2
    require(offset == len(raw), 'exact payload partition')
    return result


def compare_tensor(reference, actual):
    require(len(reference) == len(actual) and len(reference) % 2 == 0, 'comparison geometry')
    left, right = words(reference, len(reference) // 2), words(actual, len(actual) // 2)
    errors = [scalar(b) - scalar(a) for a, b in zip(left, right)]
    square = math.fsum(value * value for value in errors)
    norm = math.fsum(scalar(a) ** 2 for a in left)
    return {'elements': len(left), 'exact_words': sum(a == b for a, b in zip(left, right)),
            'max_abs_error': max(map(abs, errors), default=0.0),
            'rmse': math.sqrt(square / len(left)) if left else 0.0,
            'relative_l2': math.sqrt(square / norm) if norm else (0.0 if not square else None),
            'zero_reference_norm': norm == 0,
            'max_bf16_steps': max((abs(ordered(a) - ordered(b)) for a, b in zip(left, right)), default=0)}


def validate_case(record, raw, position):
    require(isinstance(record, dict) and set(record) ==
            {'generation', 'position', 'input_token', 'output_token'}, 'case record fields')
    require(all(type(record[key]) is int for key in record) and record['position'] == position and
            0 <= position < 4 and record['generation'] == position + 1 and
            0 <= record['input_token'] < VOCABULARY and
            0 <= record['output_token'] < VOCABULARY, 'case identity')
    rows = split_payload(raw)
    require(record['output_token'] == argmax(rows['logits']), 'case argmax does not match own logits')
    return rows


def compare_four_forwards(mode, tokens, reference_cases, candidate_cases):
    """Each case is (exact four-field record, raw606976B); no receipt authority."""
    token_inputs(mode, tokens)
    require(len(reference_cases) == len(candidate_cases) == 4, 'four complete trajectories')
    validated = []
    for cases in (reference_cases, candidate_cases):
        rows = [validate_case(record, raw, position) for position, (record, raw) in enumerate(cases)]
        records = []
        for record, _raw in cases:
            require(record['input_token'] == next_input(mode, tokens, records),
                    'four own-output autoregressive inputs')
            records.append(record)
        validated.append(rows)
    comparisons = []
    for position in range(4):
        left, right = reference_cases[position][0], candidate_cases[position][0]
        same_input = left['input_token'] == right['input_token']
        same_history = all(reference_cases[earlier][0]['input_token'] ==
                           candidate_cases[earlier][0]['input_token'] for earlier in range(position + 1))
        comparisons.append({'position': position, 'same_input': same_input,
                            'same_input_history': same_history,
                            'reference_input_token': left['input_token'],
                            'candidate_input_token': right['input_token'],
                            'output_token_equal': left['output_token'] == right['output_token'],
                            'reference_output_token': left['output_token'],
                            'candidate_output_token': right['output_token'],
                            'comparison_scope': 'same autoregressive input history, independent own KV' if same_history
                                else 'different-input autoregressive trajectories; no same-input tensor comparison',
                            'tensors': {name: compare_tensor(raw, validated[1][position][name])
                                        for name, raw in validated[0][position].items()} if same_history else None})
    return {'schema': 'ferric-p225-tiles-ar4-framework-tensor-diagnostics-v1', 'mode': mode,
            'comparisons': comparisons, 'receipt_authentication': False,
            'numerical_acceptance': False, 'acceptance_threshold': None,
            'full_model_correctness': False, 'performance_measured': False}
