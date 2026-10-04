"""Data-only Down2 intake delta; no process creation, review generation, or native calls."""
from pathlib import Path

LOWERING = (23895, '0ba363b9b4106e5293e4e6152d719c795c2d58f05e062b6a67570f194e0eb68e')
IMAGE = (33112, '65a76f917c12476f4814174ee39d515642a643a0352c6609ee514955a6a97449')
CLOCK = (902162, 'e34189597dc7db7a7c325040f5381932e84390c1a2cf32055830858cb9278ddb')
CLOCK_LABEL = 'prefix-device-clock-tf4-shared-full-currentness-gpu-v228-v1'
SYMBOL = 'ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2'
STAGES = ('fixture-metadata', 'checked-lowering', 'actual-replay', 'actual-inert-join',
          'emit', 'extract-retained', 'descriptor-metadata', 'elf-notes', 'disassembly')
LOWERING_FALSE = ('gpu_execution', 'production_authority', 'launch_authority', 'numerical_acceptance',
                  'performance_claim', 'runtime_requirements_discharged', 'full_model_acceptance')
REVIEW_TOPICS = ('source_lineage', 'formal', 'isa', 'coherence', 'lifecycle', 'selected_device')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def content(pin):
    return pin['bytes'], pin['sha256']


def lowering_shape(value):
    require(value['schema'] == 'ferric-p228-down2-lowering-result-v1'
            and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
            and all(value[key] is True for key in ('fresh_checked_lowering', 'fresh_checked_replay', 'fresh_hsaco_emitted'))
            and all(value[key] is False for key in LOWERING_FALSE)
            and value['unresolved_runtime_requirements'] == 8, 'actual inert checked Down2 lowering only')
    require(tuple(row['name'] for row in value['commands']) == STAGES, 'exact nine completed compiler phases')
    require(content(value['artifacts']['emitted/artifact.hsaco']) == IMAGE, 'exact newly emitted Down2 image')


def lowering(I, pins, record, image):
    """Read retained build evidence, never tools or original compiler targets on MI350."""
    require(content(record) == LOWERING, 'exact root-verified successful Down2 completion')
    value = I.doc(pins, record)
    lowering_shape(value)
    for row in value['commands']:
        command = I.doc(pins, row['command'])
        started = I.doc(pins, row['started'])
        result = I.doc(pins, row['result'])
        require(command['gpu_execution'] is False and command['expected_exit'] == 0
                and command['affinity'] == [8, 9] and command['nice'] == 10
                and command['cache_cap_bytes'] == 6 << 30,
                'retained bounded CPU compiler phase')
        require(type(started['pid']) is int and started['pid'] > 1 and started['pgid'] == started['pid']
                and result['exit_code'] == 0 and result['reason'] is None and result['group_absent'] is True,
                'actual naturally completed and reaped compiler phase')
        for name in ('stdout', 'stderr'):
            I.read(pins, row[name], 64 << 20, False)
            require(result[name + '_sha256'] == row[name]['sha256'], 'compiler result/stream join')
    for artifact in value['artifacts'].values():
        I.read(pins, artifact, 32 << 20, False)
    require(content(image) == IMAGE and Path(image['path']).is_relative_to(I.E), 'exact transported Down2 image')
    raw = I.read(pins, image, 32 << 20)
    require(raw[:6] == b'\x7fELF\x02\x01' and raw[18:20] == b'\xe0\x00', 'AMDGPU ELF object')
    by_name = {row['name']: row for row in value['commands']}
    return dict(lowering=record, original_image=value['artifacts']['emitted/artifact.hsaco'],
                transported_image=image, candidate_cpu=value['candidate_cpu'],
                compiler_generation=value['compiler_generation'],
                source_handoff=value['artifacts']['emitted/source.handoff-v3'],
                formal_archive=value['artifacts']['extracted/formal.archive'],
                llvm=value['artifacts']['extracted/module.ll'],
                emission_receipt=value['artifacts']['emitted/receipt.txt'],
                descriptor=by_name['descriptor-metadata']['stdout'],
                isa=by_name['disassembly']['stdout'], elf_notes=by_name['elf-notes']['stdout'],
                unresolved_runtime_requirements=8, runtime_requirements_discharged=False,
                production_authority=False, numerical_acceptance=False, performance_claim=False)


def workload(request, prior, image, rust_pin, same):
    """Exactly one image replacement; not an allowance for a different workload or runtime."""
    ignored = {'session', 'evidence_directory', 'tiles_image'}
    require(set(request) == set(prior) and ignored <= set(request)
            and same({key: value for key, value in request.items() if key not in ignored},
                     {key: value for key, value in prior.items() if key not in ignored}),
            'same clock TF4 workload, worker and bootstrap images except the explicit tiles_image')
    require(request['mode'] == prior['mode'] == 'teacher_forced' and request['session'] != prior['session'],
            'fresh teacher-forced Down2 session')
    require(rust_pin(request['tiles_image']) == image and content(image) == IMAGE
            and content(rust_pin(prior['tiles_image'])) != IMAGE,
            'new exact Down2 tiles_image replaces the old explicit MLP image')


def review(value, plan, provenance, clock, runtime, false_flags):
    required = {'schema', 'reviewed', 'authority', 'gpu_attempts', 'down2_lowering', 'down2_image',
                'clock_baseline', 'request', 'parent', 'worker', 'provenance', 'review_topics',
                'notes', 'unresolved_runtime_requirements', *false_flags}
    require(set(value) == required and value['schema'] == 'ferric-p228-down2-image-engineering-review-v1'
            and value['reviewed'] is True and value['authority'] == 'none'
            and type(value['gpu_attempts']) is int and value['gpu_attempts'] == 1
            and value['unresolved_runtime_requirements'] == 8
            and all(value[key] is False for key in false_flags), 'explicit single-attempt Down2 engineering review')
    for key in ('down2_lowering', 'down2_image', 'clock_baseline', 'request'):
        require(value[key] == plan[key], 'review exact input binding: ' + key)
    require(value['clock_baseline'] == clock['receipt_pin'] and value['provenance'] == provenance
            and value['parent'] == runtime['parent'] and value['worker'] == runtime['worker'],
            'new image review belongs to actual unchanged clock runtime and retained build evidence')
    require(set(value['review_topics']) == set(REVIEW_TOPICS), 'all existing image review topics remain required')
    for text in [value['notes'], *value['review_topics'].values()]:
        require(type(text) is str and 32 <= len(text.strip()) and len(text.encode()) <= 16384,
                'substantive separately authored source/formal/ISA/coherence/lifecycle/device review')


def clock_shape(value, old_receipt, false_flags):
    require(value['schema'] == 'ferric-p228-device-clock-gpu-v1' and value['passed'] is True
            and value['failures'] == [] and value['native_attempts'] == 1 and value['retries'] == 0
            and value['baseline'] == old_receipt and value['policy'] == 'shared-full-currentness'
            and all(value[key] is False for key in false_flags)
            and len(value['before_audits']) == len(value['after_audits']) == 3,
            'actual single closed raw-clock baseline')


def clock_baseline(I, DV, pins, record, old, C, H, O, platform, environment):
    """Replay the real clock-enabled TF4 row, retaining its original CPU553 comparison lineage."""
    require(content(record) == CLOCK and record['path'] == str(I.E / CLOCK_LABEL / 'complete.json'),
            'exact actual clock-enabled TF4 baseline')
    value = I.doc(pins, record)
    clock_shape(value, old['receipt_pin'], DV.FALSE)
    plan = I.doc(pins, value['plan'], 1 << 20)
    require(plan['baseline'] == old['receipt_pin'] and plan['parent'] == value['parent']
            and plan['worker'] == value['worker'] and plan['request'] == value['request'],
            'actual clock plan/receipt/runtime bindings')
    for key in ('image_deployment', 'standalone_prepared', 'standalone_cases', 'numericals'):
        require(plan[key] == old['plan'][key], 'unchanged V7 and six actual numerical prerequisites')
    require(set(value['leaves']) == {'parent', *[side + '-' + str(i) for side in ('before', 'after') for i in range(3)]},
            'one native leaf and six audit leaves')
    for name, leaf in value['leaves'].items():
        outcome = I.doc(pins, leaf['result'])
        I.owned_success(outcome)
        require(set(leaf['retained_files']) == {'command.json', 'started.json', 'stdout', 'stderr', 'result.json'},
                'full retained owned leaf')
        for filename, pin in leaf['retained_files'].items():
            require(pin['path'] == str(I.E / CLOCK_LABEL / name / filename), 'case-contained owned evidence')
            I.read(pins, pin, 8 << 20, False)
        require(leaf['retained_files']['result.json'] == leaf['result'], 'actual owned result pin')
        for key in ('command', 'started', 'stdout', 'stderr'):
            require(outcome[key] == leaf['retained_files'][key + ('.json' if key in ('command', 'started') else '')],
                    'owned leaf stream/command record identity')
    for side in ('before', 'after'):
        audits = value[side + '_audits']
        require(len(audits) == 3, 'three full process/topology audits on each side')
        for index, audit in enumerate(audits):
            require(audit['process_result'] == value['leaves'][side + '-' + str(index)]['result'], 'audit/owner join')
            O.check_platform(platform, I.doc(pins, audit['topology']))
            outcome = I.doc(pins, audit['process_result'])
            require(outcome['gpu_execution_requested'] is False and outcome['stderr']['bytes'] == 0, 'quiet CPU process audit')
            O.process_audit(I.read(pins, outcome['stdout']), platform)
    leaf = value['leaves']['parent']
    native = I.doc(pins, leaf['result'])
    read = lambda pin, maximum: I.read(pins, pin, maximum)
    item = dict(native_files=value['retained_native'], device_sidecar=value['device_sidecar'],
                request=value['request'], parent=value['parent'], owner=leaf['result'],
                **{key: native[key] for key in ('command', 'started', 'stdout', 'stderr')})
    require(I.doc(pins, native['command'])['env'] == environment, 'same reviewed clock native environment')
    observed, files, structural, ownership, device = DV.candidate(C, read, item, H)
    rows = DV.invariance(C, old, observed, files, H)
    checked = value['checked']
    require(checked == I.doc(pins, value['observation']) and checked['tensor_comparison'] == rows
            and checked['structural'] == structural and checked['owned_record_checks'] == ownership
            and checked['raw_rows'] == 1172 and checked['clock_samples'] == 16
            and checked['rank_packets'] == [592, 580]
            and checked['all_payloads_tokens_and_tensors_equal'] is True, 'actual clock baseline replay agrees with retained result')
    return dict(receipt_pin=record, receipt=value, plan=plan, observed=observed, request=observed['request'],
                files=files, runtime=value['selected_runtime'], structural=structural,
                ownership=ownership, device=device)


def invariance(C, baseline, observed, files, helpers, image, rust_pin, same):
    # The old raw validator remains unchanged for replaying the baseline. Only
    # this new candidate route permits the exact, separately reviewed MLP image.
    old, prior = baseline['observed'], baseline['files']
    workload(observed['request'], old['request'], image, rust_pin, same)
    for position in range(4):
        a = C.document(prior[f'request-{position}.json'])
        b = C.document(files[f'request-{position}.json'])
        require(same(a['command'], b['command']) and a['device_ids'] == b['device_ids']
                and a['id'] == b['id'] and a['protocol'] == b['protocol'], 'identical full native forward input')
    old_records, new_records = C.records(old), C.records(observed)
    require(len(old_records) == len(new_records) == 4, 'four complete token trajectories')
    rows = C.compare_rows(old_records, [prior[f'observation-{i}.bin'] for i in range(4)],
        new_records, [files[f'observation-{i}.bin'] for i in range(4)], helpers['diagnostics'])
    require(len(rows) == 4 and sum(len(row['tensors']) for row in rows) == 152, 'all 152 typed tensor comparisons')
    require(same(old_records, new_records)
        and all(len(files[f'observation-{i}.bin']) == len(prior[f'observation-{i}.bin']) == 606976
                and files[f'observation-{i}.bin'] == prior[f'observation-{i}.bin'] for i in range(4))
        and all(row['same_input_history'] and all(t['byte_equal'] is True for t in row['tensors']) for row in rows),
        'Down2 must preserve all four payloads, tokens and 152 tensors exactly')
    return rows
