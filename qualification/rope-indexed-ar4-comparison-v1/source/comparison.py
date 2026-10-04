"""Retained genuine AR4 diagnostics; no controller, model or GPU execution."""
import hashlib
import json
from pathlib import Path

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
REFERENCE = dict(path=str(E / 'projection-ar4-framework-reference-v228-v1/reference.json'),
    bytes=201239, sha256='00952244362ad51d241d179f741ae5ae61ff8acfcb3fd160b9dc64ce3b5f699e')
REFERENCE_OWNER = dict(path=str(E / 'projection-ar4-framework-launch-v228-v1/complete.json'),
    bytes=17692, sha256='94403d351120c0eb756f5660e333682ca0f47c4c6c1ac0f3cf6e9c10db896771')
BASELINE = dict(path=str(E / 'prefix-projection-ar4-decode-gpu-v228-v1/complete.json'),
    bytes=1054985, sha256='15938580d218f855883a589c819d532bbf941a02c4677cb29928f7bf7106d1cb')
EMISSION = dict(path=str(E / 'rope-indexed-checked-emission-v228-v1/complete.json'),
    bytes=197494, sha256='d45dd1a2ed5b88e767b14bbb8c7d2728211bb49aeaa14b1f8b4cc5a5b4f80c49')
IMAGE = (54344, '29fd58e7b09fee003ed31660e6201c20a7eb5da4b873d0b4d954993f7018f2f8')
PACKAGE_SHA = 'aa1e8cd787581c6b218e2eaaf358a8b468dc3fe75ff5eb85f183bb6ada8ceddd'
MODEL_ID = 'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a'
BUNDLE_ID = '6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b'


def require(ok, message):
    if not ok:
        raise ValueError(message)


def parse(raw):
    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'duplicate JSON field')
            result[key] = value
        return result
    def invalid(_):
        raise ValueError('nonfinite JSON number')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def request_delta(old, new):
    require(set(old) == set(new) == {'schema', 'decode', 'projection_residual_image'}
        and old['schema'] == new['schema'] == 'FerricFiniteProjectionResidualDecodeRequestV1'
        and old['projection_residual_image'] == new['projection_residual_image'], 'same additive residual request')
    left, right = old['decode'], new['decode']
    require(set(left) == set(right) and left['mode'] == right['mode'] == 'autoregressive', 'AR4 request mode')
    changes = {name for name in left if left[name] != right[name]}
    require(changes == {'prefix_image', 'session', 'evidence_directory'},
        'only selected prefix, fresh session and output may change')
    require(right['prefix_image']['bytes'] == IMAGE[0]
        and bytes(right['prefix_image']['sha256']).hex() == IMAGE[1], 'actual emitted RoPE image')
    require(bytes(right['expected_model_id']).hex() == MODEL_ID
        and bytes(right['expected_bundle_id']).hex() == BUNDLE_ID, 'genuine common model and bundle')


def native(read, pin, validator, candidate):
    outer = parse(read(pin))
    expected = 'ferric-p228-' + ('rope-indexed-ar4' if candidate else 'projection-ar4-decode') + '-gpu-v1'
    require(outer['schema'] == expected and outer['passed'] is True and outer['failures'] == []
        and type(outer['native_attempts']) is int and outer['native_attempts'] == 1
        and type(outer['retries']) is int and outer['retries'] == 0
        and outer['captured_payloads'] == 4 and outer['captured_tensor_rows'] == 152
        and outer['own_output_trajectory_checked'] is True and outer['numerical_acceptance'] is False,
        'actual complete four-forward native observation')
    require(outer['old_native_equality_required'] is False
        and outer['teacher_forced_token_parity_required'] is False, 'no substituted teacher-forced parity')
    if candidate:
        require(outer['supervisor_manifest']['sha256'] == PACKAGE_SHA
            and outer['prefix_emission_complete'] == EMISSION
            and (outer['prefix_image']['bytes'], outer['prefix_image']['sha256']) == IMAGE,
            'qualified linked consumer package and actual emitted image metadata')
    request = parse(read(outer['request']))
    roster = outer['retained_native']
    require(type(roster) is dict and set(roster) == validator.FILES | {'complete.json'}, 'fourteen actual native files')
    root = Path(request['decode']['evidence_directory'])
    require(root == Path(pin['path']).parent / 'native', 'original native output namespace')
    bodies = {}
    for name, record in roster.items():
        require(record['path'] == str(root / name), 'native file original path')
        bodies[name] = read(record)
    checked = validator.validate(bodies.pop('complete.json'), bodies, request)
    require(checked == outer['checked'], 'actual payload/Control/profile/Close replay equals recorded validation')
    cases = [(dict(generation=pos + 1, position=pos, input_token=checked['input_tokens'][pos],
        output_token=checked['output_tokens'][pos]), bodies[f'observation-{pos}.bin']) for pos in range(4)]
    return outer, request, cases


def framework(read, diagnostics):
    owner, value = parse(read(REFERENCE_OWNER)), parse(read(REFERENCE))
    require(owner['schema'] == 'ferric-p228-projection-ar4-framework-launch-complete-v1'
        and owner['passed'] is True and owner['failures'] == [] and owner['reference'] == REFERENCE
        and owner['native_attempts'] == 1 and owner['retries'] == 0 and owner['numerical_acceptance'] is False,
        'pinned completed genuine framework owner, not a new lifecycle replay')
    require(value['schema'] == 'ferric-p228-projection-ar4-framework-reference-v1'
        and value['status'] == 'PASS' and value['model_id'] == MODEL_ID and value['bundle_id'] == BUNDLE_ID
        and value['seed'] == 9112 and value['native_complete'] == BASELINE
        and value['genuine_framework_chain'] is True and value['candidate_intermediate_inputs'] is False
        and value['repeat_passes_byte_equal'] is True and value['framework_forward_count'] == 8
        and value['captured_tensors_per_forward'] == 38 and value['conditional_passes'] == []
        and value['conditional_replay_performed'] is False and value['numerical_acceptance'] is False,
        'exact retained genuine AR4 reference, no invented conditional later-position reference')
    require(len(value['genuine_passes']) == 2, 'two genuine fresh-cache passes')
    passes = []
    for ordinal, entry in enumerate(value['genuine_passes'], 1):
        require(set(entry) == {'ordinal', 'kind', 'fresh_cache', 'cases'} and entry['ordinal'] == ordinal
            and entry['kind'] == 'genuine_ar' and entry['fresh_cache'] is True and len(entry['cases']) == 4,
            'four positions in each genuine fresh-cache pass')
        cases = []
        for pos, case in enumerate(entry['cases']):
            require(set(case) == {'record', 'payload', 'tensors', 'cache_sha256'}
                and case['payload']['path'] == str(Path(REFERENCE['path']).parent / f'genuine-ar-pass{ordinal}-pos{pos}.bf16')
                and case['payload']['bytes'] == 606976, 'actual framework body geometry and original path')
            raw = read(case['payload'])
            slices = diagnostics.validate_case(case['record'], raw, pos)
            require(case['tensors'] == {name: dict(bytes=len(body), sha256=digest(body)) for name, body in slices.items()},
                'all 38 genuine framework tensor slices rehashed')
            cases.append((case['record'], raw))
        diagnostics.compare_four_forwards('autoregressive', [9112], cases, cases)
        passes.append(cases)
    require(passes[0] == passes[1], 'both four-payload genuine repeat passes byte-equal')
    for first, second in zip(value['genuine_passes'][0]['cases'], value['genuine_passes'][1]['cases']):
        require(first['cache_sha256'] == second['cache_sha256'], 'recorded repeat cache digest equality')
    return value, passes[0]


def metric_rows(diagnostics, reference, baseline, candidate):
    before = diagnostics.compare_four_forwards('autoregressive', [9112], reference, baseline)
    after = diagnostics.compare_four_forwards('autoregressive', [9112], reference, candidate)
    rows, positions = [], []
    for left, right in zip(before['comparisons'], after['comparisons']):
        pos = right['position']
        comparable = left['same_input_history'] and right['same_input_history']
        positions.append(dict(position=pos, baseline_same_history=left['same_input_history'],
            candidate_same_history=right['same_input_history'], before_after_comparable=comparable,
            needs_conditional_reference=not right['same_input_history'],
            framework_input_token=right['reference_input_token'], baseline_input_token=left['candidate_input_token'],
            candidate_input_token=right['candidate_input_token'], framework_output_token=right['reference_output_token'],
            baseline_output_token=left['candidate_output_token'], candidate_output_token=right['candidate_output_token']))
        names = [*(f'layer{layer}-hidden' for layer in range(36)), 'final-norm', 'logits']
        for name in names:
            old = left['tensors'][name] if left['same_input_history'] else None
            new = right['tensors'][name] if right['same_input_history'] else None
            deltas = None
            if comparable:
                deltas = {key: (new[key] - old[key] if new[key] is not None and old[key] is not None else None)
                    for key in ('max_abs_error', 'relative_l2', 'rmse', 'exact_words', 'max_bf16_steps')}
            rows.append(dict(position=pos, tensor=name, baseline=old, candidate=new,
                same_history_before_after=comparable, candidate_minus_baseline=deltas,
                needs_conditional_reference=not right['same_input_history']))
    return dict(positions=positions, tensors=rows, total_tensor_slots=152,
        candidate_tensor_rows=sum(row['candidate'] is not None for row in rows),
        baseline_tensor_rows=sum(row['baseline'] is not None for row in rows),
        comparable_delta_rows=sum(row['same_history_before_after'] for row in rows),
        all_four_output_tokens_equal=all(row['framework_output_token'] == row['candidate_output_token'] for row in positions),
        genuine_framework=after, recorded_old_native_recomputed=before,
        comparison_scope='Independent own-output AR histories; once input history diverges, comparability never recovers.',
        reference_reused_conditionally=False, conditional_reference_generated=False,
        causal_attribution=False, numerical_acceptance=False, acceptance_threshold=None)


def compare_retained(read, native_pin, validator, diagnostics):
    reference, reference_cases = framework(read, diagnostics)
    old, old_request, old_cases = native(read, BASELINE, validator, False)
    new, new_request, new_cases = native(read, native_pin, validator, True)
    request_delta(old_request, new_request)
    require(old['parent'] == new['parent'] and old['worker'] == new['worker']
        and old['parent_cpu_complete'] == new['parent_cpu_complete']
        and old['worker_cpu_complete'] == new['worker_cpu_complete'], 'same actual CPU1037 runtime metadata')
    require(old['checked'] == reference['native_checked'], 'old native is exactly the reference baseline')
    result = metric_rows(diagnostics, reference_cases, old_cases, new_cases)
    result.update(native_complete=native_pin, baseline_complete=BASELINE, framework_reference=REFERENCE,
        framework_owner=REFERENCE_OWNER, candidate_prefix_image=new['prefix_image'],
        retained_framework_payloads_rehashed=8, retained_native_payloads_rehashed=8,
        native_structure_replayed=True, full_native_owner_audits_replayed=False,
        full_framework_owner_audits_replayed=False, model_or_image_bodies_rehashed=False,
        source_image_admission_replayed=False, gpu_execution=False, model_execution=False,
        full_model_correctness=False, performance_claim=False, production_authority=False)
    return result
