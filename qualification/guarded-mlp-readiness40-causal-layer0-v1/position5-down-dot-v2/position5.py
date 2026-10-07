"""Position-five captured R2 boundary only; no Down dot or model reference."""
import struct

import head as H
import qkv as Q
import r2 as R

normalized = Q.normalized
POSITION, GENERATION, INPUT_TOKEN, WIDTH = 5, 6, 271, 4096
NATIVE = {
    'first_residual': ('after_first_residual', 'bf16', 8192),
    'down_partial': ('after_mlp', 'f32', 16384),
    'final_hidden': ('after_final_residual', 'bf16', 8192),
}
FRAMEWORK = ('down-projection', 'mlp-output', 'first-residual', 'layer0-hidden')


def uint(value, maximum):
    H.require(type(value) is int and 0 <= value <= maximum, 'strict bounded integer')
    return value


def slice_part(payload, part):
    offset = uint(part['offset'], len(payload))
    size = uint(part['bytes'], len(payload))
    H.require(size > 0 and offset + size <= len(payload), 'complete in-bounds part')
    value = payload[offset:offset + size]
    H.require(H.pin(value) == normalized(part), 'original part hash and extent')
    return value


def select(nc, native_payload, fc, framework_payload, rows):
    H.require([uint(nc[k], 40) for k in ('position', 'generation', 'layer')]
              == [POSITION, GENERATION, 0]
              and uint(fc['position'], 40) == POSITION
              and uint(fc['input_token'], 151935) == INPUT_TOKEN, 'exact position-five layer-zero scope')
    H.require(type(nc['parts']) is list and len(nc['parts']) == 34
              and type(fc['parts']) is list and len(fc['parts']) == 33, 'original full stage counts')
    H.require(H.pin(native_payload) == normalized(
        dict(bytes=nc['payload_bytes'], sha256=nc['payload_sha256'])), 'original native capture hash')
    native, reference, selected_pins = {}, {}, []
    seen, cursor = set(), 0
    for part in nc['parts']:
        rank = uint(part['rank'], 1)
        key = rank, part['role']
        H.require(type(part['role']) is str and key not in seen
                  and uint(part['offset'], len(native_payload)) == cursor, 'unique contiguous native roles')
        seen.add(key)
        value = slice_part(native_payload, part)
        cursor += len(value)
        if part['role'] in NATIVE:
            boundary, scalar, size = NATIVE[part['role']]
            H.require((part['boundary'], part['scalar'], part['bytes']) == (boundary, scalar, size)
                      and uint(part['elements'], WIDTH) == WIDTH
                      and uint(part['source_byte_offset'], 0) == 0, 'exact selected native geometry')
            native[key] = value
            selected_pins.append(dict(side='native', rank=rank, role=part['role'],
                offset=part['offset'], **H.pin(value)))
    H.require(cursor == len(native_payload)
              and set(native) == {(rank, role) for rank in (0, 1) for role in NATIVE},
              'complete native capture and six selected operands')
    seen, previous_end = set(), None
    for part in fc['parts']:
        name = part['name']
        H.require(type(name) is str and name not in seen, 'unique framework role')
        seen.add(name)
        offset = uint(part['offset'], len(framework_payload))
        H.require(previous_end is None or offset == previous_end, 'contiguous framework capture')
        value = slice_part(framework_payload, part)
        previous_end = offset + len(value)
        if name in FRAMEWORK:
            H.require(part['dtype'] == 'bfloat16' and part['shape'] == [1, 1, WIDTH]
                      and all(type(x) is int for x in part['shape']) and len(value) == 8192,
                      'exact selected framework geometry')
            H.words(value, WIDTH)
            reference[name] = value
            selected_pins.append(dict(side='framework', role=name, offset=offset, **H.pin(value)))
    H.require(set(reference) == set(FRAMEWORK)
              and reference['down-projection'] == reference['mlp-output'],
              'original framework Down producer/consumer bytes')
    H.require(type(rows) is list and len(rows) == 204 and len({r['id'] for r in rows}) == 204,
              'original full comparison roster')
    recorded = {row['id']: row for row in rows}
    for rank in (0, 1):
        for native_role, reference_role in (('first_residual', 'first-residual'),
                                           ('final_hidden', 'layer0-hidden')):
            row = recorded['5:%d:%s' % (rank, native_role)]
            H.require(row['position'] == POSITION and type(row['position']) is int
                      and row['rank'] == rank and type(row['rank']) is int
                      and row['role'] == native_role
                      and uint(row['input_token'], 151935) == INPUT_TOKEN
                      and row['native'] == H.pin(native[rank, native_role])
                      and row['reference'] == H.pin(reference[reference_role]),
                      'original comparison output/residual pin join')
    H.require(native[0, 'first_residual'] == native[1, 'first_residual']
              == reference['first-residual'], 'actual common position-five residual input')
    H.require(native[0, 'final_hidden'] == native[1, 'final_hidden'],
              'actual rank-equal final hidden, not framework equality')
    return native, reference, selected_pins


def extract(native_raw, reference_raw, repeat_raw, comparison):
    nh, np = Q.envelope(native_raw, b'FCAP061\0')
    fh, fp = Q.envelope(reference_raw, b'FREF061\0')
    rh, rp = Q.envelope(repeat_raw, b'FREF061\0')
    H.require(type(fh['ordinal']) is int and fh['ordinal'] == 1
              and type(rh['ordinal']) is int and rh['ordinal'] == 2
              and fp == rp and fh['captures'] == rh['captures'],
              'two original reference passes repeat stage bytes')
    H.require(nh['native_close_confirmed'] is True
              and comparison['checks']['both_same_side_parity_gates_rechecked'] is True,
              'original healthy Close and prior full same-side admission')
    base, chosen = 0, None
    for position, nc in enumerate(nh['captures']):
        H.require(uint(nc['position'], 40) == position
                  and uint(nc['generation'], 40) == position + 1
                  and uint(nc['layer'], 40) == 0, 'ordered native scope')
        size = uint(nc['payload_bytes'], len(np))
        H.require(base + size <= len(np), 'complete original native position extent')
        value = np[base:base + size]
        H.require(H.pin(value) == normalized(dict(bytes=size, sha256=nc['payload_sha256'])),
                  'every original native capture hash')
        if position == POSITION:
            chosen = select(nc, value, fh['captures'][position], fp,
                            comparison['checks']['diagnostic']['comparable_rows'])
        base += size
    H.require(base == len(np) and chosen is not None, 'complete six-position native payload')
    native, reference, pins = chosen
    return dict(native=native, reference=reference, selected_parts=pins,
                original_sidecars=dict(native=H.pin(native_raw),
                    reference=H.pin(reference_raw), repeat=H.pin(repeat_raw)))


def analyze(captured):
    native, reference = captured['native'], captured['reference']
    replay, outputs = R.replay(native)
    framework = R.framework_control(reference['down-projection'],
                                    reference['first-residual'], reference['layer0-hidden'])
    H.require(replay['rows_per_rank'] == WIDTH and replay['observed_words'] == 2 * WIDTH
              and len(replay['rows']) == WIDTH and framework['rows'] == WIDTH, 'all rows evaluated')
    words = [struct.unpack('<4096H', native[rank, 'final_hidden']) for rank in (0, 1)]
    reference_words = struct.unpack('<4096H', reference['layer0-hidden'])
    cross_rows = [[i for i, (a, b) in enumerate(zip(row, reference_words)) if a != b] for row in words]
    full_rows = dict(schema='ferric-causal-position5-r2-original-replay-rows-v1',
                     position=POSITION, layer=0, native=replay, framework=framework)
    report = dict(schema='ferric-causal-position5-r2-boundary-diagnostic-v1',
        position=POSITION, input_token=INPUT_TOKEN, layer=0, rows_per_rank=WIDTH, native_words_checked=2 * WIDTH,
        native_matched_words=replay['matched_words'], native_mismatch_rows=replay['mismatch_rows'],
        native_boundary_exact=replay['all_final_encodings_match'],
        framework_words_checked=WIDTH, framework_matched_words=framework['matched_words'],
        framework_boundary_exact=framework['all_encodings_match'],
        all_observed_boundaries_exact=replay['all_final_encodings_match'] and framework['all_encodings_match'],
        native_reference_different_rows=cross_rows, selected_parts=captured['selected_parts'],
        original_sidecars=captured['original_sidecars'], order=replay['order'],
        framework_inputs_not_substituted_for_native=True,
        native_projection_is_derived_not_directly_captured=True,
        source_boundary_replayed=True, current_machine_instruction_order_independently_proven=False,
        dedicated_down_dot_products_replayed=False, gate_up_or_silu_replayed=False,
        mismatch_explains_position5_argmax=False, independent_model_reference=False,
        numerical_acceptance=False, full_model_acceptance=False, acceptance_threshold=None,
        semantic_bug_claimed=False, performance_claim=False, gpu_execution=False, model_execution=False)
    return report, full_rows, outputs
