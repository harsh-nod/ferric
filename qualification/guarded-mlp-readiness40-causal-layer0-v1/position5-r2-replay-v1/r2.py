"""Captured-operand R2 boundary replay, not a model or dot-product reference."""
import hashlib
import json
import struct

import capture
import fp32_replay as F


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def parse(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON member')
            result[key] = value
        return result
    def invalid(value):
        raise ValueError('nonfinite JSON: ' + value)
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def same(left, right):
    return json.dumps(left, sort_keys=True, allow_nan=False) == json.dumps(right, sort_keys=True, allow_nan=False)


class Fields:
    PAYLOAD_BYTES = 606976
    parse = staticmethod(parse)

    @staticmethod
    def keys(value, names):
        require(type(value) is dict and set(value) == set(names.split()), 'closed capture fields')

    @staticmethod
    def uint(value, maximum=(1 << 64) - 1):
        require(type(value) is int and 0 <= value <= maximum, 'strict unsigned integer')
        return value

    @staticmethod
    def octets(values, count=32):
        require(type(values) is list and len(values) == count
                and all(type(v) is int and 0 <= v <= 255 for v in values), 'exact octet array')
        return bytes(values)

    @staticmethod
    def rust_pin(row):
        require(type(row['path']) is str, 'native original path')
        return dict(path=row['path'], bytes=Fields.uint(row['bytes']),
                    sha256=Fields.octets(row['sha256']).hex())


def admit(terminal, numerical, summary_raw, read_body, saved_checked):
    require(terminal['schema'] == 'ferric-guarded-mlp-model-stage-capture-gpu-v1'
            and terminal['passed'] is True and terminal['errors'] == []
            and terminal['capture_requested'] is terminal['capture_verified'] is True
            and terminal['native_attempts'] == 1 and terminal['retries'] == 0, 'qualified actual capture')
    require(numerical['schema'] == 'ferric-guarded-mlp-model-stage-numerical-report-v1'
            and numerical['passed'] is True and numerical['error'] is None
            and numerical['postcheck_errors'] == [] and numerical['input_posthashes_complete'] is True,
            'authenticated prior complete stage admission')
    for record in (terminal, numerical):
        require(all(record[key] is False for key in
                    ('numerical_acceptance', 'full_model_acceptance', 'performance_claim', 'production_authority')),
                'preserved original nonclaims')
    summary = parse(summary_raw)
    require(summary['schema'] == 'FerricFiniteGuardedMlpDecodeObservationV1'
            and summary['native_closed'] is summary['child_exit_zero'] is summary['process_group_absent'] is True
            and summary['completed_forwards'] == 4 and summary['input_tokens'][0] == 9112,
            'actual same-run native completion')
    checked = capture.capture_admission(summary_raw, read_body, Fields)
    previous = numerical['diagnostic']['candidate']
    require(same(checked, terminal['capture_observation']) and same(checked, saved_checked)
            and same(checked, previous['capture_observation'])
            and same(previous['observation'], terminal['observation']), 'independent saved capture joins')
    require(numerical['diagnostic']['captured_dedicated_guarded_down'] is True
            and numerical['diagnostic']['position'] == numerical['diagnostic']['layer'] == 0
            and numerical['diagnostic']['input_token'] == 9112, 'actual dedicated Down and selected location')
    envelope = parse(read_body(checked['source']))
    raw = Fields.octets(envelope['capture']['payload'], 256136)
    selected, source_parts = {}, []
    for part in envelope['capture']['parts']:
        if part['role'] in ('down_partial', 'first_residual', 'final_hidden'):
            key = (part['rank'], part['role'])
            require(key not in selected, 'unique selected operand')
            body = raw[part['offset']:part['offset'] + part['bytes']]
            selected[key] = body
            source_parts.append(dict(rank=part['rank'], role=part['role'], offset=part['offset'],
                                     source_byte_offset=part['source_byte_offset'], **pin(body)))
    require(set(selected) == {(rank, role) for rank in (0, 1)
                             for role in ('down_partial', 'first_residual', 'final_hidden')},
            'six original input/output slices')
    return selected, dict(capture_observation=checked, selected_parts=source_parts,
        original_native_attempts=1, previous_full_model_validator_admission=True,
        full_model_validator_rerun=False, capture_admission_repeated=True)


def replay(selected):
    require(set(selected) == {(rank, role) for rank in (0, 1)
                             for role in ('down_partial', 'first_residual', 'final_hidden')}, 'exact R2 operand roles')
    for (rank, role), raw in selected.items():
        require(type(raw) is bytes and len(raw) == (16384 if role == 'down_partial' else 8192),
                'exact R2 operand extent')
    partials = [struct.unpack('<4096I', selected[(rank, 'down_partial')]) for rank in (0, 1)]
    residuals = [struct.unpack('<4096H', selected[(rank, 'first_residual')]) for rank in (0, 1)]
    observed = [struct.unpack('<4096H', selected[(rank, 'final_hidden')]) for rank in (0, 1)]
    rows, sums, projections, expected = [], [], [], [[], []]
    matches = [0, 0]
    for index in range(4096):
        values = [F.combined(partials[0][index], partials[1][index], residuals[rank][index]) for rank in (0, 1)]
        require(values[0]['sum_word'] == values[1]['sum_word']
                and values[0]['projection'] == values[1]['projection'], 'rank-independent ordered projection')
        sums.append(values[0]['sum_word']); projections.append(values[0]['projection'])
        for rank in (0, 1):
            F.bf16_f32(observed[rank][index])
            expected[rank].append(values[rank]['residual'])
            matches[rank] += observed[rank][index] == values[rank]['residual']
        rows.append(dict(row=index, down_partial_f32=[f'{values[index]:08x}' for values in partials],
            ordered_sum_f32=f'{sums[-1]:08x}', derived_down_bf16=f'{projections[-1]:04x}',
            first_residual_bf16=[f'{values[index]:04x}' for values in residuals],
            replay_final_bf16=[f'{values[-1]:04x}' for values in expected],
            observed_final_bf16=[f'{values[index]:04x}' for values in observed],
            matches=[expected[rank][-1] == observed[rank][index] for rank in (0, 1)]))
    outputs = {'derived-ordered-sum.f32': struct.pack('<4096I', *sums),
               'derived-down.bf16': struct.pack('<4096H', *projections)}
    outputs.update({'replayed-final-rank%d.bf16' % rank: struct.pack('<4096H', *expected[rank]) for rank in (0, 1)})
    return dict(schema='ferric-guarded-mlp-current-r2-replay-v1', rows=rows, rows_per_rank=4096,
        observed_words=8192, matched_words=sum(matches), matched_words_per_rank=matches,
        all_final_encodings_match=matches == [4096, 4096],
        mismatch_rows=[row['row'] for row in rows if not all(row['matches'])],
        rank_residual_inputs_byte_equal=selected[(0, 'first_residual')] == selected[(1, 'first_residual')],
        rank_final_outputs_byte_equal=selected[(0, 'final_hidden')] == selected[(1, 'final_hidden')],
        derived_outputs={name: pin(body) for name, body in outputs.items()},
        order='FP32(+0 + rank0), FP32(+ rank1), BF16-RNE, FP32(+ BF16 residual), BF16-RNE',
        source_contract_replayed=True, current_machine_instruction_order_independently_proven=False,
        dedicated_down_dot_products_replayed=False, materialized_down_directly_captured=False,
        model_weights_read=False, framework_execution=False, native_execution=False, gpu_execution=False,
        independent_model_reference=False, numerical_acceptance=False, acceptance_threshold=None,
        full_model_acceptance=False, performance_claim=False, production_authority=False), outputs


def framework_control(projection, residual, final):
    require(all(type(v) is bytes and len(v) == 8192 for v in (projection, residual, final)),
            'framework boundary extents')
    p, r, f = (struct.unpack('<4096H', v) for v in (projection, residual, final))
    expected = [F.narrow_f32(F.add_f32(F.bf16_f32(a), F.bf16_f32(b))) for a, b in zip(p, r)]
    for value in f: F.bf16_f32(value)
    return dict(rows=4096, matched_words=sum(a == b for a, b in zip(expected, f)),
                all_encodings_match=expected == list(f), framework_inputs_not_substituted_for_native=True)
