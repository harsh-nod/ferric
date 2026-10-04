"""Retained TF4/framework diagnostics; no launch or numerical acceptance policy."""
from pathlib import Path

GPU_SCHEMA = 'ferric-p228-silu-materialized-decode-gpu-v1'
GPU_MANIFEST_SHA = '47afc73e3333ff0705002a16b1a296780d22c119676bba142f07c2eb9fe80c31'
GPU_CONTROLLER_SHA = '100acf80caba09a7080d72b9680059c0d63c143978a7b6f4f19c8f61e3405e9b'
SILU_IMAGE = (33320, 'b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589')
SILU_LAYER = (1024009, '67a235280b48cb5f6a2c51bcca534c182c843857c4c4a98e86718ed368ef12e4')
LAYER_COMPARISON = (1508927, '828f8fdb4fc4194b5d7d5c65135222b472d2ec76080667fae8f0b8febba169aa')
FRAMEWORK_REPORT = (128641, '2edddf40cc6195fdd622e479e8e46f2a659f1b569106f5f4e51afdb83bdc2416')
BASELINE_COMPARISON = (206798, '2a5b98ae6a3a2ba4a034dbea0662504988d388b12a5447eaa395f82e3a41dd0c')
SILU_FIELDS = ('baseline', 'layer_comparison', 'mlp_image', 'mlp_cpu',
    'mlp_lowering_complete', 'mlp_lowering_owner')
RAW = {'command.json', 'started.json', 'stdout', 'stderr', 'result.json'}
LEAVES = {'parent', *[side + '-' + str(i) for side in ('before', 'after') for i in range(3)]}
TOKENS = [9112, 2190, 3772, 220]
FALSE = ('numerical_acceptance', 'independent_tensor_acceptance', 'full_model_acceptance',
    'full_model_correctness', 'performance_claim', 'production_authority',
    'paired_comparison_performed', 'native_baseline_comparison_performed',
    'conditional_residual_checks_performed', 'independent_framework_comparison_performed')


def require(ok, message):
    if not ok:
        raise ValueError(message)


def content(C, record):
    pin = C.pin(record)
    return pin['bytes'], pin['sha256']


def silu_provenance(read, outer, plan, request, C):
    for name in SILU_FIELDS:
        require(outer[name] == plan[name], 'SiLU outer/plan binding: ' + name)
    require(content(C, outer['mlp_image']) == SILU_IMAGE
        and C.pin(request['decode']['tiles_image']) == outer['mlp_image'], 'actual selected SiLU image')
    require(content(C, outer['baseline']) == SILU_LAYER
        and content(C, outer['layer_comparison']) == LAYER_COMPARISON, 'actual layer and mathematical predecessor')
    layer = C.doc(read, outer['baseline'])
    math = C.doc(read, outer['layer_comparison'])
    require(layer['schema'] == 'ferric-p228-silu-materialized-capture-gpu-v1'
        and layer['passed'] is True and layer['failures'] == [], 'actual completed SiLU layer')
    for name in ('mlp_image', 'mlp_cpu', 'mlp_lowering_complete', 'mlp_lowering_owner', 'projection_image'):
        require(layer[name] == outer[name], 'same layer-selected image provenance: ' + name)
    require(math['schema'] == 'ferric-p228-silu-materialized-comparison-observation-v1'
        and math['completed'] is True and math['source_postchecks_passed'] is True
        and math['native_outer'] == outer['baseline'], 'actual layer comparison binds actual native predecessor')
    for record in (layer, math):
        require(all(record[key] is False for key in
            ('numerical_acceptance', 'full_model_correctness', 'performance_claim', 'production_authority')),
            'predecessors confer no acceptance')
    inputs = math['comparison']['inputs']
    require(inputs['candidate_capture'] == layer['retained_native']['summary.json']
        and inputs['selected_mlp_image'] == outer['mlp_image']
        and inputs['projection_image'] == outer['projection_image']
        and content(C, inputs['original_framework']) == FRAMEWORK_REPORT,
        'same image, genuine layer capture and original framework identities')
    # Layer CPU988 and decode CPU1022 workers differ; do not conflate their identities.
    return {name: outer[name] for name in SILU_FIELDS}


def baseline_metrics(read, record, comparison, C):
    require(content(C, record) == BASELINE_COMPARISON, 'exact previously measured TF4 comparison')
    value = C.doc(read, record)
    require(value['schema'] == 'ferric-p228-projection-residual-decode-framework-comparison-observation-v1'
        and value['completed'] is True and value['errors'] == []
        and value['source_postchecks_passed'] is True
        and value['framework_outer'] == comparison['framework_complete'],
        'same actual completed baseline and framework owner')
    require(all(value[key] is False for key in ('gpu_execution', 'numerical_acceptance',
        'full_model_correctness', 'full_model_acceptance', 'performance_claim', 'production_authority')),
        'baseline diagnostic scope only')
    old, new = value['comparison']['comparisons'], comparison['comparisons']
    require(len(old) == len(new) == 4, 'four recorded comparison positions')
    for left, right in zip(old, new):
        require(all(left[key] == right[key] for key in
            ('position', 'reference_input', 'candidate_input', 'reference_output', 'same_input_history'))
            and left['same_input_history'] is True
            and len(left['tensors']) == len(right['tensors']) == 38, 'same teacher-forced reference histories')
        for a, b in zip(left['tensors'], right['tensors']):
            require(all(a[key] == b[key] for key in ('name', 'elements', 'reference_sha256')),
                'all152 exact reference tensor identities, not merely same model')
    return dict(receipt=record, comparisons=old, metrics_recomputed=False,
        baseline_payloads_rehashed=False, reference_tensor_identities_equal=True,
        causal_attribution_proven=False, numerical_acceptance=False, performance_claim=False)


def candidate(read, outer_pin, C, V):
    """Replay retained bytes. The caller authenticates the supplied actual outer pin."""
    outer = C.doc(read, outer_pin)
    require(outer['schema'] == GPU_SCHEMA and outer['passed'] is True and outer['failures'] == []
        and type(outer['native_attempts']) is int and outer['native_attempts'] == 1
        and type(outer['retries']) is int and outer['retries'] == 0
        and outer['full_forward'] is True and outer['gpu_execution_requested'] is True
        and outer['old_native_equality_required'] is False
        and all(outer[key] is False for key in FALSE), 'actual one-attempt diagnostic GPU outcome')
    require(outer['controller']['sha256'] == GPU_CONTROLLER_SHA
        and outer['supervisor_manifest']['sha256'] == GPU_MANIFEST_SHA,
        'qualified new GPU controller and package')
    C.get(read, outer['controller']); C.get(read, outer['supervisor_manifest'])
    plan = C.doc(read, outer['plan'])
    directory = Path(outer_pin['path']).parent
    require(Path(outer_pin['path']).name == 'complete.json'
        and directory.name == plan['output_label'], 'actual case namespace')
    for name in ('request', 'parent', 'worker', 'projection_image', 'lowering_complete',
                 'inspection_complete', 'parent_runtime_review', 'worker_runtime_review', 'decode_review'):
        require(outer[name] == plan[name], 'outer/plan binding: ' + name)
    request = C.doc(read, outer['request'], 65536)
    require(request['decode']['evidence_directory'] == str(directory / 'native'), 'actual nested output path')
    require(C.pin(request['decode']['worker']) == outer['worker']
        and C.pin(request['projection_residual_image']) == outer['projection_image'], 'selected candidate bindings')
    silu_provenance(read, outer, plan, request, C)
    records = outer['retained_native']
    require(type(records) is dict and set(records) == V.FILES | {'complete.json'}
        and sum(C.pin(p)['bytes'] for p in records.values()) <= 8 << 20, 'closed14 native files')
    require(all(C.pin(p)['path'] == str(directory / 'native' / name) for name, p in records.items()),
        'each native file belongs to actual case')
    summary = C.get(read, records['complete.json'], 65536)
    files = {name: C.get(read, records[name], 2 << 20) for name in V.FILES}
    checked = V.validate(summary, files, request)
    require(checked == outer['checked'] == C.doc(read, outer['observation'])
        and outer['captured_tensor_rows'] == checked['captured_tensor_rows'] == 152
        and outer['captured_payloads'] == checked['captured_payloads'] == 4,
        'recomputed152 rows and actual structural result')

    require(set(outer['leaves']) == LEAVES, 'one parent and six recorded audits')
    leaves = {}
    for name, item in outer['leaves'].items():
        require(set(item) == {'result', 'retained_files'} and set(item['retained_files']) == RAW,
            'complete recorded owned leaf')
        pins = item['retained_files']
        require(pins['result.json'] == item['result'], 'owned result pin join')
        for filename, pin in pins.items():
            require(C.pin(pin)['path'] == str(directory / name / filename), 'case-contained owned file')
            C.get(read, pin)
        result = C.doc(read, item['result']); C.clean_owner(result)
        for key in ('command', 'started', 'stdout', 'stderr'):
            require(result[key] == pins[key + '.json' if key in ('command', 'started') else key],
                'actual owned stream/command pin')
        start = C.doc(read, result['started'])
        require(start['command_sha256'] == result['command']['sha256'], 'started command digest')
        require(result['gpu_execution_requested'] is (name == 'parent'), 'one native leaf only')
        leaves[name] = (result, start)
    parent, start = leaves['parent']
    command = C.doc(read, parent['command'])
    require(command['argv'] == [outer['parent']['path'], '--request', outer['request']['path'],
        '--observe-projection-residual-decode', '--allow-unauthenticated-machine-code'], 'new exact parent selector')
    require(command['deadline_seconds'] == 4000 and command['affinity'] == [8, 9]
        and command['nice'] == 10 and command['address_space_bytes'] == 32 << 30
        and command['file_cap_bytes'] == 64 << 20 and command['stream_cap_bytes'] == 8 << 20
        and command['gpu_execution_requested'] is True, 'original bounded native command')
    require(C.get(read, parent['stdout']) == summary, 'actual parent stdout is retained summary including LF')
    observed = C.document(summary)
    V.child_marker(C.get(read, parent['stderr']), observed['child_pid'])
    ownership = C.lineage(parent, start, observed['child_pid'])
    for side in ('before', 'after'):
        audits = outer[side + '_audits']
        require(type(audits) is list and len(audits) == 3, 'three recorded audits per side')
        for i, row in enumerate(audits):
            name = side + '-' + str(i)
            require(row['process_result'] == outer['leaves'][name]['result']
                and row['topology']['path'] == str(directory / (name + '-topology.json')),
                'recorded topology/process join')
            C.doc(read, row['topology'])
            require(C.get(read, leaves[name][0]['stderr']) == b'', 'recorded audit stderr empty')
    return outer, observed, files, checked, ownership


def compare_payloads(reference_records, reference_rows, candidate_records, candidate_rows, C, D):
    for records in (reference_records, candidate_records):
        require(len(records) == 4 and [row['input_token'] for row in records] == TOKENS,
            'genuine same teacher-forced token sequence')
    rows = C.compare_rows(reference_records, reference_rows, candidate_records, candidate_rows, D)
    require(len(rows) == 4 and all(row['same_input_history'] and len(row['tensors']) == 38 for row in rows),
        'four comparable38-tensor payloads')
    growth = []
    for row in rows:
        hidden = row['tensors'][:36]
        require([t['name'] for t in hidden] == ['layer%d-hidden' % n for n in range(36)], 'ordered hidden layers')
        trajectory = []
        for layer, tensor in enumerate(hidden):
            previous = hidden[layer - 1] if layer else None
            relative, old_relative = tensor['relative_l2'], previous['relative_l2'] if previous else None
            trajectory.append(dict(layer=layer, exact_words=tensor['exact_words'], elements=tensor['elements'],
                max_abs_error=tensor['max_abs_error'], relative_l2=relative,
                max_abs_change_from_previous_layer=(tensor['max_abs_error'] - previous['max_abs_error'] if previous else None),
                relative_l2_change_from_previous_layer=(relative - old_relative
                    if relative is not None and old_relative is not None else None)))
        growth.append(dict(position=row['position'], layers=trajectory,
            first_nonexact_hidden_layer=next((i for i, t in enumerate(hidden) if not t['byte_equal']), None),
            causal_attribution_proven=False, monotonic_growth_required=False))
    return dict(comparisons=rows, layer_error_trajectories=growth,
        tensor_rows=152, exact_tensor_rows=sum(t['byte_equal'] for row in rows for t in row['tensors']),
        output_tokens_equal=all(row['output_equal'] for row in rows),
        reference_output_tokens=[row['reference_output'] for row in rows],
        candidate_output_tokens=[row['candidate_output'] for row in rows])


def compare_retained(outer_pin, reference_owner_pin, read, C, V, D):
    outer, observed, files, checked, ownership = candidate(read, outer_pin, C, V)
    reference_records, reference_rows = C.reference(read, reference_owner_pin, 'teacher_forced',
        {'diagnostics': D, 'decode_validation': V})
    metrics = compare_payloads(reference_records, reference_rows, C.records(observed),
        [files['observation-%d.bin' % n] for n in range(4)], C, D)
    return dict(schema='ferric-p228-silu-materialized-decode-framework-diagnostic-v1',
        mode='teacher_forced', candidate_complete=outer_pin, framework_complete=reference_owner_pin,
        projection_image=outer['projection_image'], selected_mlp_image=outer['mlp_image'],
        layer_capture=outer['baseline'], layer_comparison=outer['layer_comparison'],
        layer_predecessor_receipts_rehashed=True, layer_tensor_bodies_replayed=False,
        structural=checked, recorded_ownership=ownership,
        **metrics, gpu_launched=False, recorded_close_and_owner_reap_checked=True,
        recorded_six_audit_leaves_rehashed=True, current_platform_idle_audits_verified=False,
        source_binary_image_admission_replayed=False, numerical_acceptance=False, acceptance_threshold=None,
        independent_tensor_acceptance=False, full_model_acceptance=False, full_model_correctness=False,
        conditional_residual_checks_performed=False, candidate_intermediate_inputs=False,
        sustained_2048_256=False, causal_attribution_proven=False, performance_claim=False,
        production_authority=False)

