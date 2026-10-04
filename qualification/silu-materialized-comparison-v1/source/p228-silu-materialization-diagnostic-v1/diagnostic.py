"""Exact captured-BF16 multiplication controls; no exponential approximation."""
import hashlib
import struct


def require(value, message):
    if not value:
        raise ValueError(message)


def finite_word(word, bits):
    require(type(word) is int and 0 <= word < 1 << bits, 'unsigned integer word')
    mask = 0x7f80 if bits == 16 else 0x7f800000
    require(word & mask != mask, 'nonfinite input')
    return word


def _round_shift(value, shift):
    if shift <= 0:
        return value << -shift
    quotient, remainder = divmod(value, 1 << shift)
    halfway = 1 << (shift - 1)
    return quotient + int(remainder > halfway or (remainder == halfway and quotient & 1))


def mul_f32_rne(left, right):
    """Integer-only finite binary32 multiply, RNE and gradual underflow.

    Decode exact dyadics, multiply their integer significands, then round once
    to the normal 24-bit lattice or fixed 2**-149 subnormal lattice.
    """
    finite_word(left, 32)
    finite_word(right, 32)
    sign = (left ^ right) & 0x80000000
    def dyadic(word):
        exponent, fraction = (word >> 23) & 255, word & 0x7fffff
        return (fraction, -149) if exponent == 0 else ((1 << 23) | fraction, exponent - 150)
    a, ae = dyadic(left)
    b, be = dyadic(right)
    product, scale = a * b, ae + be
    if product == 0:
        return sign
    leading_exponent = product.bit_length() - 1 + scale
    quantum = max(-149, leading_exponent - 23)
    rounded = _round_shift(product, quantum - scale)
    if rounded == 0:
        return sign
    if rounded >= 1 << 24:
        require(rounded == 1 << 24, 'one-bit rounding carry')
        rounded >>= 1
        quantum += 1
    if rounded < 1 << 23:
        require(quantum == -149, 'subnormal lattice')
        return sign | rounded
    exponent = quantum + 150
    require(1 <= exponent < 255, 'nonfinite binary32 product')
    return sign | exponent << 23 | (rounded - (1 << 23))


def product_word(silu, up, narrow):
    finite_word(silu, 16)
    finite_word(up, 16)
    return narrow(mul_f32_rne(silu << 16, up << 16))


def words(raw, count):
    require(type(raw) is bytes and len(raw) == count * 2, 'exact BF16 extent')
    values = list(struct.unpack('<' + str(count) + 'H', raw))
    for word in values:
        finite_word(word, 16)
    return values


def digest_words(values):
    return hashlib.sha256(struct.pack('<' + str(len(values)) + 'H', *values)).hexdigest()


def compare(framework, candidates, narrow):
    """Condition on genuine SiLU only where the native gate bits are identical.

    The genuine SiLU is not a reconstructed native FP32 intermediate. A differing
    native product therefore cannot be attributed solely to missing rounding.
    """
    require(set(framework) == {'gate', 'silu-input', 'silu', 'up', 'product'}, 'five genuine stages')
    fw = {name: words(raw, 12288) for name, raw in framework.items()}
    require(fw['gate'] == fw['silu-input'], 'genuine gate equals SiLU input')
    expected = [product_word(s, u, narrow) for s, u in zip(fw['silu'], fw['up'])]
    control_errors = [i for i, (a, b) in enumerate(zip(expected, fw['product'])) if a != b]
    require(not control_errors, 'framework BF16-SiLU product control failed: ' + repr(control_errors[:16]))
    require(type(candidates) is list and len(candidates) == 2, 'two native TP ranks')
    rows = []
    for rank, raw in enumerate(candidates):
        require(set(raw) == {'gate', 'up', 'activation'}, 'three native stages')
        native = {name: words(body, 6144) for name, body in raw.items()}
        offset = rank * 6144
        groups = {name: [] for name in ('same_gate_same_up', 'same_gate_different_up',
                                       'different_gate_same_up', 'different_gate_different_up')}
        predictions, indices, mismatches, comparable_errors = [], [], [], []
        for i in range(6144):
            j = offset + i
            same_gate = native['gate'][i] == fw['gate'][j]
            same_up = native['up'][i] == fw['up'][j]
            key = ('same_gate' if same_gate else 'different_gate') + ('_same_up' if same_up else '_different_up')
            groups[key].append(i)
            if same_gate:
                predicted = product_word(fw['silu'][j], native['up'][i], narrow)
                indices.append(i)
                predictions.append(predicted)
                if predicted != native['activation'][i]:
                    mismatches.append(i)
                if same_up:
                    require(predicted == fw['product'][j], 'same-input prediction joins genuine product')
                    if native['activation'][i] != fw['product'][j]:
                        comparable_errors.append(i)
        rows.append(dict(rank=rank, elements=6144,
            partition_counts={name: len(value) for name, value in groups.items()},
            partitions=groups, same_gate_elements=len(indices),
            prediction_indices=indices, predicted_bf16_sha256=digest_words(predictions),
            native_equal_materialized=len(indices) - len(mismatches),
            native_different_materialized=len(mismatches), mismatch_indices=mismatches,
            same_gate_same_up_native_different_framework_indices=comparable_errors,
            conditioning='genuine BF16 SiLU at identical gate; actual native BF16 up',
            native_silu_intermediate_observed=False))
    return dict(schema='ferric-p228-silu-materialization-diagnostic-v1', authority='none',
        framework_product_control=dict(elements=12288, exact_words=12288, byte_equal=True,
            predicted_sha256=digest_words(expected), silu_input_equals_gate=True),
        ranks=rows, conditional_replay_performed=True, exponential_evaluated=False,
        native_exp_error_measured=False, materialization_only_cause_proven=False,
        native_silu_intermediate_observed=False, different_gate_predictions_performed=False,
        numerical_acceptance=False, acceptance_threshold=None, full_model_correctness=False,
        gpu_execution=False, performance_claim=False, production_authority=False)
