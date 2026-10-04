"""Captured SiLU-boundary diagnostics; no native admission or fitted tolerance."""
import math
import struct

import comparison as M
import diagnostic as S

C, K, B = M.C, M.K, M.B
SCHEMA = 'ferric-p228-silu-materialized-comparison-v1'
RESIDUAL_ADAPTER_SHA = 'f8074e6eaedbe51d26307b596401487c589c834688cd6c395ee8b3fac60ef149'
SILU_CONTROL_SHA = '19ac2cf361d5653b4438b77d5a7f5501961ffe1dcabb2606fbf681b5175ab3f8'


def request(summary, baseline, mlp_image, residual_image, worker):
    outer, previous = summary['request'], baseline['request']
    C.require(set(outer) == set(previous) == {'schema', 'layer', 'projection_residual_image'}
              and outer['schema'] == previous['schema'] == M.REQUEST
              and outer['projection_residual_image'] == previous['projection_residual_image']
              and K.identity(outer['projection_residual_image']) == M.selected_pin(residual_image),
              'unchanged separately selected projection-residual image')
    value, old = outer['layer'], previous['layer']
    C.require(set(value) == set(old), 'unchanged inner request fields')
    for key in set(old) - {'session', 'evidence_directory', 'worker', 'mlp_tiles_image'}:
        C.require(value[key] == old[key], 'unchanged layer input: ' + key)
    C.require(K.identity(value['worker']) == K.identity(old['worker']) == M.selected_pin(worker),
              'same actually qualified worker')
    C.require(K.identity(value['mlp_tiles_image']) == M.selected_pin(mlp_image)
              and K.identity(value['mlp_tiles_image']) != K.identity(old['mlp_tiles_image']),
              'only the separately selected MLP image changes')
    C.require(C.wire(value['session']) != '0' * 64 and value['session'] != old['session']
              and value['evidence_directory'] != old['evidence_directory'], 'fresh candidate session and output')
    return value


def unchanged_upstream(old, new, old_page, new_page):
    rows = []
    for rank in range(2):
        for name, _, _ in K.STAGES[:11]:
            left, right = old[rank][name], new[rank][name]
            if name in ('key-cache', 'value-cache'):
                left = left[old_page * 16 * 1024:old_page * 16 * 1024 + 1024]
                right = right[new_page * 16 * 1024:new_page * 16 * 1024 + 1024]
            C.require(left == right, 'unchanged pre-SwiGLU stage: ' + name)
            rows.append(dict(rank=rank, stage=name, bytes=len(right), sha256=C.sha(right), byte_equal=True))
    return rows


def partial_difference(left, right, rank):
    """Native-to-native FP32 change only; neither partial is a full dot oracle."""
    C.require(type(left) is bytes and type(right) is bytes and len(left) == len(right) == 16384,
              'two full native FP32 Down partials')
    a, b = list(struct.iter_unpack('<I', left)), list(struct.iter_unpack('<I', right))
    C.require(all(word & 0x7f800000 != 0x7f800000 for rows in (a, b) for (word,) in rows), 'finite Down partials')
    av, bv = struct.unpack('<4096f', left), struct.unpack('<4096f', right)
    error = [r - l for l, r in zip(av, bv)]
    norm, delta = math.sqrt(math.fsum(v * v for v in av)), math.sqrt(math.fsum(v * v for v in error))
    return dict(stage='down-partial', rank=rank, elements=4096, element_bytes=4,
                comparison='candidate-vs-corrected-native-baseline-fp32-partial',
                exact_elements=sum(l == r for l, r in zip(a, b)), byte_equal=left == right,
                max_absolute_error=max(map(abs, error)), relative_l2_error=delta / norm if norm else 0.0 if delta == 0 else None,
                zero_reference_norm=norm == 0, reference_sha256=C.sha(left), candidate_sha256=C.sha(right),
                independent_reference=False, compared_to_full_framework_bf16=False)


def compare_parts(framework, baseline, candidate, baseline_page, candidate_page, diagnostics):
    upstream = unchanged_upstream(baseline, candidate, baseline_page, candidate_page)
    residuals, conditioning = M.residual_comparisons(framework, candidate)
    rows, earliest = K.comparisons(framework, candidate, candidate_page, diagnostics)
    old_rows, old_earliest = K.comparisons(framework, baseline, baseline_page, diagnostics)
    oracle = B.load_oracle()
    genuine = {name: framework[name] for name in ('gate', 'silu-input', 'silu', 'up', 'product')}
    controls = {label: S.compare(genuine, [{name: part[name] for name in ('gate', 'up', 'activation')}
                                         for part in parts], oracle.narrow_bf16_rne)
                for label, parts in (('baseline', baseline), ('candidate', candidate))}
    changes = []
    for rank in range(2):
        for name in ('activation', 'final-hidden'):
            changes.append(dict(rank=rank, comparison='candidate-vs-corrected-native-baseline-bf16',
                                **C.diagnostic(diagnostics, name, baseline[rank][name], candidate[rank][name])))
        changes.append(partial_difference(baseline[rank]['down-partial'], candidate[rank]['down-partial'], rank))
    return dict(unchanged_pre_swiglu_arrays=upstream, unchanged_pre_swiglu_count=len(upstream),
                conditional_residual_comparisons=residuals, conditional_residual_conditioning=conditioning,
                conditional_residuals_exact=all(row['byte_equal'] for row in residuals),
                conditional_residual_words=sum(row['elements'] for row in residuals),
                comparisons=rows, comparable_rows=len(rows), earliest_observable_divergence=earliest,
                baseline_comparisons=old_rows, baseline_earliest_observable_divergence=old_earliest,
                silu_product_controls=controls, changed_native_stages=changes,
                framework_product_control_exact=all(row['framework_product_control']['byte_equal'] for row in controls.values()))


def compare_retained(framework_pin, original_framework_pin, current_tf4_pin,
                     corrected_capture_pin, candidate_capture_pin, read, diagnostics,
                     *, candidate_mlp_image_pin, projection_image_pin, candidate_worker_pin):
    """Outer ownership, selected images and terminal controls are caller-authenticated.

    Keeps the two genuine framework passes and all native captures unchanged.
    Conditional mismatches remain diagnostic; no acceptance threshold is applied.
    """
    primary, framework = C.compare_retained(framework_pin, original_framework_pin, current_tf4_pin, read, diagnostics)
    C.require(primary['original_framework_matches_new'] is True, 'genuine original framework reproduction')
    report = C.document(C.checked_read(read, framework_pin, 4 << 20))
    baseline = C.document(C.checked_read(read, corrected_capture_pin, 65536))
    candidate = C.document(C.checked_read(read, candidate_capture_pin, 65536))
    for value in (baseline, candidate):
        M.observation(value, M.OBSERVATION)
    inner = request(candidate, baseline, candidate_mlp_image_pin, projection_image_pin, candidate_worker_pin)
    K.framework_prompt(report, inner)
    old_bodies, old_pins = M.native_files(baseline, read)
    bodies, pins = M.native_files(candidate, read)
    old_page, page = M.bootstrap_page(baseline, old_bodies), M.bootstrap_page(candidate, bodies)
    old_parts = M.stage_bytes(baseline, old_bodies['candidate-capture.bin'], old_page, diagnostics)
    parts = M.stage_bytes(candidate, bodies['candidate-capture.bin'], page, diagnostics)
    result = compare_parts(framework, old_parts, parts, old_page, page, diagnostics)
    return dict(schema=SCHEMA, authority='none', input_token=9112, position=0,
                inputs=dict(framework_capture=framework_pin, original_framework=original_framework_pin,
                            current_tf4=current_tf4_pin, corrected_baseline=corrected_capture_pin,
                            candidate_capture=candidate_capture_pin, selected_mlp_image=candidate_mlp_image_pin,
                            projection_image=projection_image_pin, worker=candidate_worker_pin),
                baseline_files=old_pins, candidate_files=pins,
                physical_pages=dict(baseline=old_page, candidate=page), primary=primary, **result,
                oracle_sha256=B.ORACLE_SHA256, conditional_replay_performed=True,
                observation_order=list(K.ORDER), ordering_is_a_causal_proof=False,
                excluded_fp32_partials=[row for row in candidate['stages'] if row['element_bytes'] == 4],
                fp32_partials_compared_to_full_bf16=False, genuine_independent_framework_outputs=True,
                native_silu_intermediate_observed=False, exponential_evaluated=False,
                native_exp_error_measured=False, materialization_only_cause_proven=False,
                different_gate_predictions_performed=False, upstream_partial_numerics_checked=False,
                old_hidden_equality_required=False, receipt_authentication=False,
                arithmetic_prerequisites_verified=False, runtime_premises_discharged=False,
                numerical_acceptance=False, acceptance_threshold=None, full_layer_numerics_accepted=False,
                full_model_correctness=False, production_authority=False, performance_measured=False, gpu_execution=False)
