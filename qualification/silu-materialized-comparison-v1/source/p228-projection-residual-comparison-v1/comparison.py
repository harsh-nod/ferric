"""Data-only projection-materialized residual diagnostics, not GPU admission."""
import struct

import current as K
import compare as C
import boundary as B


CURRENT_SHA = 'eb103b681c54b0c880bc6e692363208dae837eb322845bab7e593bdea6330d7b'
BOUNDARY_SHA = 'a5fac7b587739999623167d81c163fc18ebe9a2baf986da1b192b597e7fb0b9d'
REQUEST = 'FerricFiniteProjectionResidualLayerCaptureRequestV1'
BOOTSTRAP = 'FerricProjectionResidualLayerBootstrapV1'
OBSERVATION = 'FerricFiniteProjectionResidualLayerCaptureObservationV1'
SCHEMA = 'ferric-p228-projection-residual-comparison-v1'


def observation(value, schema):
    C.require(value['schema'] == schema and value['native_attempts'] == 1
              and value['retries'] == 0 and value['completed_layers'] == 1
              and value['native_closed'] is True and value['gpu_execution'] is True
              and all(value[name] is False for name in ('paired_comparison_performed',
                  'numerical_acceptance', 'performance_claim', 'production_authority', 'full_forward'))
              and 'bitwise_equal' not in value and 'runs' not in value,
              'one-owner closed capture; outer ownership is caller-authenticated')


def selected_pin(value):
    C.require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}
              and type(value['bytes']) is int and value['bytes'] > 0
              and type(value['sha256']) is str and len(value['sha256']) == 64
              and all(c in '0123456789abcdef' for c in value['sha256'])
              and value['sha256'] != '0' * 64, 'explicit caller-authenticated selected FilePin')
    return value['bytes'], value['sha256']


def candidate_request(summary, baseline, image_pin, worker_pin):
    outer = summary['request']
    C.require(set(outer) == {'schema', 'layer', 'projection_residual_image'}
              and outer['schema'] == REQUEST, 'distinct projection-residual request')
    value, old = outer['layer'], baseline['request']
    C.require(set(value) == set(old), 'unchanged inner layer request fields')
    for key in set(old) - {'session', 'evidence_directory', 'worker'}:
        C.require(value[key] == old[key], 'unchanged workload and original image field: ' + key)
    C.require(K.identity(value['worker']) == selected_pin(worker_pin), 'actual newly qualified worker')
    C.require(K.identity(outer['projection_residual_image']) == selected_pin(image_pin)
              and K.identity(outer['projection_residual_image']) != K.identity(value['images']['residual']),
              'distinct selected residual; original residual/copy image stays unchanged')
    C.require(C.wire(value['session']) != '0' * 64, 'candidate scoped session')
    return value


def native_files(summary, read):
    # Same bounded 11-file format as K.native_files, with the new outer request.
    # Do not rewrite that authenticated request into the old request schema.
    outer = summary['request']
    records = summary['files']
    C.require(type(records) is list and len(records) == 11, 'complete candidate file roster')
    bodies, pins = {}, {}
    directory = K.Path(outer['layer']['evidence_directory'])
    C.require(directory.is_absolute() and '..' not in directory.parts, 'absolute candidate evidence directory')
    C.require([p['name'] for p in records] == list(K.FILES), 'exact candidate-only evidence roster')
    for record in records:
        C.require(set(record) == {'name', 'bytes', 'sha256'}, 'closed native evidence File')
        name = record['name']
        p = dict(path=str(directory / name), bytes=record['bytes'], sha256=C.wire(record['sha256']))
        maximum = (K.CAPTURE_BYTES if name == 'candidate-capture.bin' else
                   2 << 20 if name == 'candidate-stderr.bin' else
                   4 << 20 if name in K.FILES[1:4] else 65536)
        if name == 'candidate-stderr.bin' and p['bytes'] == 0:
            body = read(p)
            C.require(type(body) is bytes and body == b'' and C.sha(body) == p['sha256'], 'empty child stderr')
        else:
            body = C.checked_read(read, p, maximum)
        bodies[name], pins[name] = body, p
    C.require(C.document(bodies['request.json']) == outer, 'retained outer request unchanged')
    C.require(C.document(bodies['candidate-bootstrap.json']) == summary['run']['bootstrap'], 'retained outer bootstrap')
    C.require(C.document(bodies['candidate-response-2.json']) == summary['run']['close'], 'retained actual Close')
    return bodies, pins


def bootstrap_page(summary, bodies):
    outer = summary['run']['bootstrap']
    selected = summary['request']['projection_residual_image']
    C.require(set(outer) == {'schema', 'layer', 'projection_residual_image'}
              and outer['schema'] == BOOTSTRAP
              and outer['projection_residual_image'] == dict(bytes=selected['bytes'], sha256=selected['sha256']),
              'actual additional image part in distinct bootstrap')
    # The old validator checks the unchanged inner ABI, raw Run/Close and each
    # run's own profile. The outer caller validates the new profile domain hash.
    view = dict(summary, request=summary['request']['layer'],
                run=dict(summary['run'], bootstrap=outer['layer']))
    return K.bootstrap_page(view, bodies)


def stage_bytes(summary, raw, page, diagnostics):
    C.require(type(raw) is bytes and len(raw) == K.CAPTURE_BYTES
              and type(page) is int and 0 <= page < 144, 'capture extent/page')
    records = summary['stages']
    C.require(type(records) is list and len(records) == 28, 'exact candidate 28-stage roster')
    parts, offset = [], 0
    for rank in range(2):
        values = {}
        for ordinal, (name, count, width) in enumerate(K.STAGES):
            body = raw[offset:offset + count]
            expected = dict(rank=rank, stage=name, offset=offset, bytes=count,
                elements=count // width, element_bytes=width, sha256=list(bytes.fromhex(C.sha(body))))
            C.require(records[rank * 14 + ordinal] == expected, 'candidate stage shape/digest: ' + name)
            if width == 2:
                diagnostics.words(body, count // 2)
            else:
                C.require(all(v & 0x7f800000 != 0x7f800000 for (v,) in struct.iter_unpack('<I', body)),
                          'finite candidate FP32 partial')
            if name in ('key-cache', 'value-cache'):
                start = page * 16 * 1024
                C.require(not any(body[:start]) and not any(body[start + 1024:]), 'untouched candidate KV')
            values[name] = body
            offset += count
        parts.append(values)
    C.require(offset == len(raw), 'exact candidate capture partition')
    for name in ('first-residual', 'final-hidden'):
        C.require(parts[0][name] == parts[1][name], 'replicated candidate residual ranks agree: ' + name)
    return parts


def unchanged_prefix(baseline, candidate, baseline_page, candidate_page):
    rows = []
    for rank in range(2):
        for name, _, _ in K.STAGES[:7]:
            left, right = baseline[rank][name], candidate[rank][name]
            if name in ('key-cache', 'value-cache'):
                left = left[baseline_page * 16 * 1024:baseline_page * 16 * 1024 + 1024]
                right = right[candidate_page * 16 * 1024:candidate_page * 16 * 1024 + 1024]
            C.require(left == right, 'unchanged pre-residual captured stage: ' + name)
            rows.append(dict(rank=rank, stage=name, bytes=len(right), sha256=C.sha(right), byte_equal=True))
    return rows


def materialized_word(oracle, p0, p1, skip):
    first = oracle.add_f32_rne(0, p0, 'rank0')
    total = oracle.add_f32_rne(first, p1, 'rank1')
    projection = oracle.narrow_bf16_rne(total)
    summed = oracle.add_f32_rne(projection << 16, skip << 16, 'materialized-residual')
    return projection, oracle.narrow_bf16_rne(summed)


def residual_comparisons(framework, parts):
    oracle = B.load_oracle()
    rows, conditioning = [], []
    for stage, partial, result, skips in (
        ('output', 'output-partial', 'first-residual', [framework['embedding']] * 2),
        ('down', 'down-partial', 'final-hidden', [p['first-residual'] for p in parts])):
        p0, p1 = B.pair([p[partial] for p in parts], 4, partial)
        for rank, values in enumerate(parts):
            skip = B.words(skips[rank], 2, stage + '-skip')
            actual = B.words(values[result], 2, result)
            expected = []
            for index, (left, right, residual) in enumerate(zip(p0, p1, skip, strict=True)):
                try:
                    expected.append(materialized_word(oracle, left, right, residual)[1])
                except oracle.FinitePolicyError as error:
                    raise ValueError(f'{stage} rank {rank} element {index}: {error}') from error
            rows.append(dict(stage=stage, rank=rank,
                **B.compare_words(stage + '-materialized-vs-native-rank' + str(rank), expected, actual)))
            conditioning.append(dict(stage=stage, rank=rank, partials=[K.part(p[partial]) for p in parts],
                residual=K.part(skips[rank]), actual=K.part(values[result]),
                residual_source='genuine-framework-embedding' if stage == 'output' else 'candidate-first-residual'))
    return rows, conditioning


def compare_retained(framework_pin, original_framework_pin, current_tf4_pin,
                     baseline_capture_pin, candidate_capture_pin, read, diagnostics,
                     *, candidate_image_pin, candidate_worker_pin):
    """Caller authenticates both terminal native outers and genuine framework outer.

    All supplied readers must return the authenticated pin's original bytes.
    Conditional arithmetic disagreements are retained, never hidden by tolerance.
    """
    primary, framework = C.compare_retained(framework_pin, original_framework_pin, current_tf4_pin, read, diagnostics)
    C.require(primary['original_framework_matches_new'] is True, 'genuine framework reproduces original layer0')
    tf4 = C.document(C.checked_read(read, current_tf4_pin, 4 << 20))
    tf4_request = C.document(C.checked_read(read, tf4['request'], 1 << 20))
    hidden = C.current_hidden(tf4, tf4_request, read, diagnostics)
    baseline = C.document(C.checked_read(read, baseline_capture_pin, 65536))
    observation(baseline, 'FerricFinitePrefixLayerCaptureObservationV1')
    K.current_request(baseline, tf4_request)
    old_bodies, old_pins = K.native_files(baseline, read)
    old_page = K.bootstrap_page(baseline, old_bodies)
    old_parts = K.stage_bytes(baseline, old_bodies['candidate-capture.bin'], old_page, hidden, diagnostics)
    candidate = C.document(C.checked_read(read, candidate_capture_pin, 65536))
    observation(candidate, OBSERVATION)
    request = candidate_request(candidate, baseline, candidate_image_pin, candidate_worker_pin)
    K.framework_prompt(C.document(C.checked_read(read, framework_pin, 4 << 20)), request)
    bodies, pins = native_files(candidate, read)
    page = bootstrap_page(candidate, bodies)
    parts = stage_bytes(candidate, bodies['candidate-capture.bin'], page, diagnostics)
    unchanged = unchanged_prefix(old_parts, parts, old_page, page)
    residuals, conditioning = residual_comparisons(framework, parts)
    rows, earliest = K.comparisons(framework, parts, page, diagnostics)
    return dict(schema=SCHEMA, authority='none', position=0, input_token=9112,
        inputs=dict(framework_capture=framework_pin, original_framework=original_framework_pin,
            current_tf4=current_tf4_pin, baseline_capture=baseline_capture_pin, candidate_capture=candidate_capture_pin,
            selected_image=candidate_image_pin, selected_worker=candidate_worker_pin),
        baseline_files=old_pins, candidate_files=pins, physical_pages=dict(baseline=old_page, candidate=page),
        primary=primary, unchanged_pre_residual_arrays=unchanged, unchanged_pre_residual_count=len(unchanged),
        conditional_residual_comparisons=residuals, conditional_residual_conditioning=conditioning,
        conditional_residuals_exact=all(row['byte_equal'] for row in residuals),
        conditional_residual_words=sum(row['elements'] for row in residuals),
        oracle_sha256=B.ORACLE_SHA256, conditional_replay_performed=True,
        arithmetic_premises=['binary32-rne-gradual-underflow', 'separate-ordered-additions-no-reassociation',
                             'bf16-rne-finite-boundaries'],
        comparisons=rows, comparable_rows=len(rows), earliest_observable_divergence=earliest,
        observation_order=list(K.ORDER), ordering_is_a_causal_proof=False,
        old_hidden_equality_required=False, candidate_hidden_matches_old_tf4=parts[0]['final-hidden'] == hidden,
        both_candidate_residual_ranks_equal=True, genuine_independent_framework_outputs=True,
        excluded_fp32_partials=[row for row in candidate['stages'] if row['element_bytes'] == 4],
        fp32_partials_compared_to_full_bf16=False, upstream_partial_numerics_checked=False,
        projection_gemm_equivalence_proven=False, cumulative_chain_differences_not_isolated_operator_errors=True,
        receipt_authentication=False, arithmetic_prerequisites_verified=False, runtime_premises_discharged=False,
        numerical_acceptance=False, acceptance_threshold=None, full_layer_numerics_accepted=False,
        full_model_correctness=False, production_authority=False, performance_measured=False, gpu_execution=False)
