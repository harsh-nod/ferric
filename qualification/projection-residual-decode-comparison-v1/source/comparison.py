"""Retained TF4/framework diagnostics; no launch or numerical acceptance policy."""
from pathlib import Path

GPU_SCHEMA = 'ferric-p228-projection-residual-decode-gpu-v1'
GPU_MANIFEST_SHA = '6dfd492f7dc8da8bd8eaa2f8e9da9bcbe36cd94756ba321bda538b3abb6fbb35'
GPU_CONTROLLER_SHA = '42890b417528192928392feb905121ec40904471989fe3f4e60e3197fbb2cde0'
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
    return dict(schema='ferric-p228-projection-residual-decode-framework-diagnostic-v1',
        mode='teacher_forced', candidate_complete=outer_pin, framework_complete=reference_owner_pin,
        projection_image=outer['projection_image'], structural=checked, recorded_ownership=ownership,
        **metrics, gpu_launched=False, recorded_close_and_owner_reap_checked=True,
        recorded_six_audit_leaves_rehashed=True, current_platform_idle_audits_verified=False,
        source_binary_image_admission_replayed=False, numerical_acceptance=False, acceptance_threshold=None,
        independent_tensor_acceptance=False, full_model_acceptance=False, full_model_correctness=False,
        conditional_residual_checks_performed=False, candidate_intermediate_inputs=False,
        sustained_2048_256=False, causal_attribution_proven=False, performance_claim=False,
        production_authority=False)
