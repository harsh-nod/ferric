"""Selected original position-five Down dots; exact dyadics, not an ISA emulator."""
import struct

import exact_bf16 as E
import fp32_replay as F
import gateup as G
import head as H
import position5 as P

WIDTH, HALF, POSITION = 4096, 6144, 5
TENSOR = 'model.layers.0.mlp.down_proj.weight'
DIFFERING_ROWS = (219, 955, 1352, 1408, 1953, 2040, 2070, 2208, 2210,
                  2328, 2706, 2850, 3704, 4017)
ROWS = (0,) + DIFFERING_ROWS
PRODUCT_DIFFERENCES = ((0, 553, 0xbb2b, 0xbb29), (0, 2575, 0x3864, 0x3863),
    (0, 2603, 0x39ca, 0x39c9), (0, 2840, 0x3746, 0x3747),
    (1, 1719, 0xb5d1, 0xb5d2), (1, 2063, 0x39dc, 0x39dd), (1, 5310, 0x35c9, 0x35ca))
R2_ORDER = 'FP32(+0 + rank0), FP32(+ rank1), BF16-RNE, FP32(+ BF16 residual), BF16-RNE'


def tensor_layout(header, index, header_bytes, file_bytes):
    # The qualified Gate/Up helper checks the entire indexed shard, not just selected tensors.
    G.tensor_layout(header, index, header_bytes, file_bytes)
    H.require(TENSOR in header and header[TENSOR]['shape'] == [WIDTH, 2 * HALF],
              'original row-major 4096 by 12288 Down tensor')
    return 8 + header_bytes + header[TENSOR]['data_offsets'][0]


def row_offset(start, row, rank):
    H.require(type(start) is int and start >= 0 and type(row) is int and 0 <= row < WIDTH
              and type(rank) is int and rank in (0, 1), 'strict Down row/rank coordinates')
    return start + (row * 2 * HALF + rank * HALF) * 2


def registration_upload_pins(original_raw, summary, uploads_raw):
    _, joined = G.registration_upload_pins(original_raw, summary, uploads_raw)
    original, uploads = H.parse(original_raw), H.parse(uploads_raw)
    result, seen = {}, set()
    for rank in (0, 1):
        layer = next(row for row in original['layers'] if (row['rank'], row['layer']) == (rank, 0))
        candidates = [row for row in layer['weights'] if row['kind'] == 'down_projection']
        H.require(len(candidates) == 1, 'one original Down weight role')
        buffer = candidates[0]['buffer']
        H.require(set(buffer) == {'rank', 'id', 'elements', 'element_bytes'}
                  and all(type(buffer[k]) is int for k in buffer)
                  and buffer['rank'] == rank and 0 <= buffer['id'] < 1 << 64
                  and buffer['elements'] == WIDTH * HALF and buffer['element_bytes'] == 2
                  and (rank, buffer['id']) not in seen, 'rank-local column-sharded Down geometry')
        H.require(sum(weight['buffer']['rank'] == rank and weight['buffer']['id'] == buffer['id']
                      for weight in layer['weights']) == 1, 'Down buffer does not alias another weight role')
        seen.add((rank, buffer['id']))
        key = dict(kind='source', rank=rank, id=buffer['id'])
        matches = [row for row in uploads['uploads'] if row['key'] == key]
        H.require(len(matches) == 1 and set(matches[0]) == {'key', 'bytes', 'sha256'}
                  and type(matches[0]['key']['rank']) is int
                  and type(matches[0]['key']['id']) is int, 'one exact Down source upload')
        value = G.normalized(matches[0])
        H.require(value['bytes'] == WIDTH * HALF * 2, 'entire uploaded Down partition extent')
        result[rank] = dict(key=key, **value)
    return result, joined


def check_partition(actual, expected):
    H.require(G.normalized(actual) == G.normalized(expected),
              'current native Down partition equals original checkpoint columns')


def select_products(nc, np, fc, fp):
    H.require(type(nc['position']) is int and nc['position'] == POSITION
              and type(nc['generation']) is int and nc['generation'] == 6
              and type(nc['layer']) is int and nc['layer'] == 0
              and type(fc['position']) is int and fc['position'] == POSITION
              and type(fc['input_token']) is int and fc['input_token'] == 271,
              'original position-five product scope')
    products, pins = {}, []
    for rank in (0, 1):
        selected = [p for p in nc['parts'] if p['rank'] == rank and p['role'] == 'activation']
        H.require(len(selected) == 1, 'unique original native activation')
        part = selected[0]
        H.require(type(part['rank']) is int and part['rank'] == rank
                  and part['boundary'] == 'after_mlp' and part['scalar'] == 'bf16'
                  and type(part['elements']) is int and part['elements'] == HALF
                  and type(part['source_byte_offset']) is int and part['source_byte_offset'] == 0,
                  'native Down input geometry and boundary')
        value = P.slice_part(np, part)
        H.words(value, HALF)
        products['native', rank] = value
        pins.append(dict(side='native', rank=rank, role='activation', **H.pin(value)))
    selected = [p for p in fc['parts'] if p['name'] == 'product']
    H.require(len(selected) == 1, 'unique independent framework product')
    part = selected[0]
    H.require(part['dtype'] == 'bfloat16' and part['shape'] == [1, 1, 2 * HALF]
              and all(type(n) is int for n in part['shape']), 'framework full Down input geometry')
    full = P.slice_part(fp, part)
    H.words(full, 2 * HALF)
    pins.append(dict(side='framework', role='product', **H.pin(full)))
    for rank in (0, 1):
        value = full[rank * HALF * 2:(rank + 1) * HALF * 2]
        products['framework', rank] = value
        pins.append(dict(side='framework', rank=rank, role='product-column-half', **H.pin(value)))
    return products, pins


def product_differences(products):
    H.require(set(products) == {(side, rank) for side in ('native', 'framework') for rank in (0, 1)},
              'four own-input product halves')
    result = []
    for rank in (0, 1):
        a, b = (H.words(products[side, rank], HALF) for side in ('native', 'framework'))
        result.extend((rank, i, x, y) for i, (x, y) in enumerate(zip(a, b)) if x != y)
    return tuple(result)


def extract(bodies):
    comparison = H.parse(bodies['comparison.json'])
    captured = P.extract(bodies['native-sidecar.bin'], bodies['reference-sidecar1.bin'],
                         bodies['reference-sidecar2.bin'], comparison)
    nh, np = G.envelope(bodies['native-sidecar.bin'], b'FCAP061\0')
    fh, fp = G.envelope(bodies['reference-sidecar1.bin'], b'FREF061\0')
    base = sum(c['payload_bytes'] for c in nh['captures'][:POSITION])
    nc = nh['captures'][POSITION]
    products, pins = select_products(nc, np[base:base + nc['payload_bytes']], fh['captures'][POSITION], fp)
    H.require(product_differences(products) == PRODUCT_DIFFERENCES,
              'exact seven original own-input product differences')
    captured.update(products=products, product_pins=pins)
    return captured


def reuse_r2(captured, terminal_raw, replay_raw, down_raw, sum_raw):
    terminal, replay = H.parse(terminal_raw), H.parse(replay_raw)
    H.require(terminal['schema'] == 'ferric-causal-position5-r2-replay-cpu-v1'
              and terminal['passed'] is True and terminal['error'] is None
              and terminal['postcheck_errors'] == [] and terminal['tests']['passed'] == 26
              and terminal['tests']['failures'] == terminal['tests']['errors'] == terminal['tests']['skipped'] == 0
              and terminal['dedicated_down_dot_products_replayed'] is False
              and terminal['numerical_acceptance'] is False, 'actual original successful boundary-only R2')
    diagnostic = terminal['diagnostic']
    H.require(diagnostic['order'] == R2_ORDER and diagnostic['all_observed_boundaries_exact'] is True
              and diagnostic['native_matched_words'] == 8192 and diagnostic['framework_matched_words'] == WIDTH
              and diagnostic['original_sidecars'] == captured['original_sidecars']
              and diagnostic['native_reference_different_rows'] == [list(DIFFERING_ROWS)] * 2,
              'existing R2 exact result and selected original rows')
    for name, raw in [('replay.json', replay_raw), ('derived-down.bf16', down_raw),
                      ('derived-ordered-sum.f32', sum_raw)]:
        H.require(diagnostic['retained_outputs'][name] == H.pin(raw), 'original retained R2 output pin')
    H.require(replay['schema'] == 'ferric-causal-position5-r2-original-replay-rows-v1'
              and type(replay['position']) is int and replay['position'] == POSITION
              and type(replay['layer']) is int and replay['layer'] == 0
              and replay['native']['order'] == R2_ORDER
              and len(replay['native']['rows']) == WIDTH, 'original complete R2 row ledger')
    native, reference = captured['native'], captured['reference']
    down = H.words(down_raw, WIDTH)
    H.require(type(sum_raw) is bytes and len(sum_raw) == WIDTH * 4, 'original derived FP32 sum extent')
    sums = struct.unpack('<4096I', sum_raw)
    partials = [struct.unpack('<4096I', native[r, 'down_partial']) for r in (0, 1)]
    residuals = [H.words(native[r, 'first_residual'], WIDTH) for r in (0, 1)]
    final = [H.words(native[r, 'final_hidden'], WIDTH) for r in (0, 1)]
    reference_final = H.words(reference['layer0-hidden'], WIDTH)
    different = tuple(i for i in range(WIDTH) if final[0][i] != reference_final[i])
    H.require(different == DIFFERING_ROWS and final[0][0] == reference_final[0],
              'fourteen original rows plus predetermined matching row-zero control')
    for i, row in enumerate(replay['native']['rows']):
        for rank in (0, 1):
            F.f32_units(partials[rank][i])
        F.f32_units(sums[i])
        H.require(type(row['row']) is int and row['row'] == i
                  and row['down_partial_f32'] == ['%08x' % partials[r][i] for r in (0, 1)]
                  and row['first_residual_bf16'] == ['%04x' % residuals[r][i] for r in (0, 1)]
                  and row['observed_final_bf16'] == ['%04x' % final[r][i] for r in (0, 1)]
                  and row['replay_final_bf16'] == row['observed_final_bf16']
                  and row['matches'] == [True, True]
                  and row['ordered_sum_f32'] == '%08x' % sums[i]
                  and row['derived_down_bf16'] == '%04x' % down[i],
                  'byte-joined original R2 row, not a replacement history')
    return dict(partials=partials, sums=sums, down=down, residual=residuals[0], final=final[0],
        framework_down=H.words(reference['down-projection'], WIDTH), framework_final=reference_final,
        evidence=dict(terminal=H.pin(terminal_raw), replay=H.pin(replay_raw),
                      derived_down=H.pin(down_raw), derived_sum=H.pin(sum_raw)))


def bf16_units(word):
    return E.decode_units(word) << 133


def decompose(native_exact, framework_exact, partial_words, sum_word, native_down,
              framework_down, residual, native_final, framework_final):
    H.require(len(native_exact) == len(framework_exact) == len(partial_words) == 2
              and all(type(x) is int for x in native_exact + framework_exact), 'two exact rank dots per side')
    partials = [F.f32_units(x) for x in partial_words]
    ordered = F.add_f32(F.add_f32(0, partial_words[0]), partial_words[1])
    H.require(ordered == sum_word and F.narrow_f32(sum_word) == native_down,
              'selected original R2 sum/narrow boundary join')
    n, f = sum(native_exact), sum(framework_exact)
    rank_errors = [actual - exact for actual, exact in zip(partials, native_exact)]
    summed = F.f32_units(sum_word)
    nd, fd, r = bf16_units(native_down), bf16_units(framework_down), bf16_units(residual)
    nf, ff = bf16_units(native_final), bf16_units(framework_final)
    terms = dict(upstream_own_product_delta=n-f, native_rank0_accumulation_error=rank_errors[0],
        native_rank1_accumulation_error=rank_errors[1], native_tp_sum_rounding=summed-sum(partials),
        native_projection_bf16_rounding=nd-summed, framework_projection_error=fd-f,
        observed_projection_difference=nd-fd, native_residual_boundary_error=nf-nd-r,
        framework_residual_boundary_error=ff-fd-r, observed_final_hidden_difference=nf-ff)
    projection_rhs = (n-f) + sum(rank_errors) + (summed-sum(partials)) + (nd-summed) - (fd-f)
    H.require(nd-fd == projection_rhs and nf-ff == projection_rhs + (nf-nd-r) - (ff-fd-r),
              'exact projection and common-residual error decomposition reconciles')
    ideal = [E.round_dot_units(x) for x in (n, f)]
    H.require(all((x & 0x7f80) != 0x7f80 for x in ideal), 'finite selected once-rounded exact dots')
    hypothetical_final = []
    for word in ideal:
        added = F.add_f32(F.bf16_f32(word), F.bf16_f32(residual))
        hypothetical_final.append(F.narrow_f32(added))
    H.require(all((x & 0x7f80) != 0x7f80 for x in hypothetical_final),
              'finite diagnostic own-input ideal residual boundaries')
    return dict(unit='2^-266', terms={k: str(v) for k, v in terms.items()},
        exact_native_rank_dots=[str(x) for x in native_exact],
        exact_framework_rank_dots=[str(x) for x in framework_exact],
        exact_native_full_dot=str(n), exact_framework_full_dot=str(f),
        native_partials_f32=['%08x' % x for x in partial_words], original_sum_f32='%08x' % sum_word,
        native_projection_bf16='%04x' % native_down, framework_projection_bf16='%04x' % framework_down,
        common_residual_bf16='%04x' % residual, native_final_bf16='%04x' % native_final,
        framework_final_bf16='%04x' % framework_final,
        own_input_ideal_projection_bf16=['%04x' % x for x in ideal],
        own_input_ideal_projection_encodings_equal=ideal[0] == ideal[1],
        observed_projection_matches_own_ideal=[native_down == ideal[0], framework_down == ideal[1]],
        counterfactual_own_ideal_projection_plus_common_residual_bf16=['%04x' % x for x in hypothetical_final],
        counterfactual_own_ideal_final_encodings_equal=hypothetical_final[0] == hypothetical_final[1],
        counterfactual_not_model_execution=True, exact_decomposition_reconciled=True)


def analyze(captured, reused, weight_rows):
    H.require(set(weight_rows) == {(row, rank) for row in ROWS for rank in (0, 1)},
              'closed fourteen difference rows and one fixed control')
    vectors = {k: H.words(v, HALF) for k, v in captured['products'].items()}
    rows = []
    for row in ROWS:
        weights = [H.words(weight_rows[row, rank], HALF) for rank in (0, 1)]
        exact = {side: [E.dot_units(vectors[side, rank], weights[rank]) for rank in (0, 1)]
                 for side in ('native', 'framework')}
        sparse = []
        for rank, local, native, framework in PRODUCT_DIFFERENCES:
            value = (E.decode_units(native) - E.decode_units(framework)) * E.decode_units(weights[rank][local])
            sparse.append(dict(rank=rank, local_column=local, global_column=rank * HALF + local,
                native_product_bf16='%04x' % native, framework_product_bf16='%04x' % framework,
                weight_bf16='%04x' % weights[rank][local], delta_units_2_pow_minus266=str(value)))
        H.require(sum(int(r['delta_units_2_pow_minus266']) for r in sparse)
                  == sum(exact['native']) - sum(exact['framework']), 'sparse upstream delta equals full-dot delta')
        H.require(all(sum(int(r['delta_units_2_pow_minus266']) for r in sparse if r['rank'] == rank)
                      == exact['native'][rank] - exact['framework'][rank] for rank in (0, 1)),
                  'each rank sparse upstream delta equals its exact own-input dot difference')
        analysis = decompose(exact['native'], exact['framework'],
            [reused['partials'][rank][row] for rank in (0, 1)], reused['sums'][row], reused['down'][row],
            reused['framework_down'][row], reused['residual'][row], reused['final'][row], reused['framework_final'][row])
        rows.append(dict(row=row, fixed_matching_control=row == 0, weight_halves=[H.pin(weight_rows[row, r]) for r in (0, 1)],
                         sparse_upstream_delta=sparse, **analysis))
    return dict(schema='ferric-position5-selected-down-exact-error-v1', position=POSITION, layer=0,
        input_token=271, selected_differing_rows=list(DIFFERING_ROWS), fixed_control_row=0,
        rows=rows, exact_rank_dot_count=len(ROWS) * 4, terms_per_rank_dot=HALF,
        full_dot_terms=2 * HALF, product_difference_count=len(PRODUCT_DIFFERENCES),
        products=captured['product_pins'], original_r2_reused=reused['evidence'],
        exact_unit='2^-266', exact_integers_encoded_as_decimal_strings=True,
        framework_rank_dots_are_exact_diagnostic_partitions_not_observed_accumulators=True,
        native_projection_is_original_r2_derived_not_direct_capture=True,
        native_fp32_accumulation_tree_emulated=False, framework_accumulation_emulated=False,
        current_machine_instruction_order_independently_proven=False,
        input_substitution_in_model=False, r2_full_vector_reexecution=False,
        selected_rows_only=True, all_layer_down_acceptance=False,
        gate_up_or_silu_replayed=False, mismatch_explains_position5_argmax=False,
        independent_model_reference=False, acceptance_threshold=None,
        numerical_acceptance=False, full_model_acceptance=False, policy_changed=False,
        semantic_bug_claimed=False, performance_claim=False, gpu_execution=False, model_execution=False)
