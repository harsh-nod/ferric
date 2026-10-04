"""Data-only Four capture observation, distinct from paired numerical comparison."""

INPUT_SCHEMA = 'ferric-p228-prefix-decode-independent-observation-inputs-v1'
RESULT_SCHEMA = 'ferric-p228-prefix-decode-independent-observation-v1'


def observe(C, plan, read, helpers, host):
    C.keys(plan, 'schema mode policy request prefix_image candidate')
    C.require(plan['schema'] == INPUT_SCHEMA
        and type(plan['mode']) is str and plan['mode'] in C.MODES
        and type(plan['policy']) is str and plan['policy'] in host.H.POLICIES,
        'explicit independent Four observation, never legacy parity')
    C.require(C.pin(plan['request']) == C.pin(plan['candidate']['request']),
              'selected request is the actual owned candidate request')
    expected_image = C.pin(plan['prefix_image'])

    # Reuse the complete native structure, owned-leaf, Close, and sidecar checks.
    observed, _, structural, ownership, diagnostic = host.candidate(
        C, read, plan['candidate'], helpers)
    C.require(observed['request']['mode'] == plan['mode']
        and diagnostic['policy'] == plan['policy'], 'selected mode and host policy')
    C.require(C.pin(observed['request']['prefix_image']) == expected_image,
              'captured request binds the explicitly selected prefix image')

    return dict(schema=RESULT_SCHEMA, status='FINITE_FOUR_FORWARD_OBSERVED',
        structural_observation_complete=True, mode=plan['mode'], policy=plan['policy'],
        request=C.pin(plan['request']), prefix_image=expected_image,
        structural=structural, owned_record_checks=ownership, host_observation=diagnostic,
        captured_tensor_rows=4 * (36 + 1 + 1), inputs=plan,
        recorded_close_and_owner_reap_checked=True, gpu_launched=False,
        native_baseline_comparison_performed=False, independent_framework_comparison_performed=False,
        numerical_acceptance=False, independent_tensor_acceptance=False,
        independent_tensor_threshold=None, full_model_acceptance=False,
        current_source_binary_image_authority_verified=False,
        current_platform_idle_audits_verified=False, top_level_observer_reaping_verified=False,
        sustained_2048_256=False, performance_claim=False, production_authority=False)
