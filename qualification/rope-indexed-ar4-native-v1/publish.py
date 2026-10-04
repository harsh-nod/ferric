"""Publish an actual retained AR4 observation; never launch or admit a native run."""
import ast
import hashlib
import math
import os
from pathlib import Path
import re
import stat
import sys
import types

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/rope-indexed-ar4-native-v1'
PRIOR = L / 'proposals/p228-projection-residual-decode-native-publication-v1/publish.py'
PRIOR_SHA = 'cb4974eb7fda4281df656106aba83f7ff1c44a2b66dfe3f56e7b9a6ad6a961ca'
PACKAGE = 'p228-rope-indexed-ar4-gpu-v1'
PACKAGE_SHA = 'aa1e8cd787581c6b218e2eaaf358a8b468dc3fe75ff5eb85f183bb6ada8ceddd'
POLICY = F / 'qualification/rope-indexed-ar4-supervisor-v1'
POLICY_SHA = 'f596362f38f73f280b5d8cb03ec3deac00cd02a26cde9168cd426c6439a89d46'
PLAN_SHA = '4e9b2e9ce3f1d43c8e242d75508f42def544b675a54ba09e80d25ae67ccd285c'
RUNTIME_PUBLIC = F / 'qualification/projection-ar4-native-v1'
RUNTIME_SHA = '010c82b5b47023a2f0a75a01eb857f11d35962282a6410e2ca8b9e126eb0bdc8'
EMISSION_PUBLIC = F / 'qualification/rope-indexed-checked-emission-v1'
EMISSION_SHA = '551622fde0afd6765fb8ce7abeb346f0717471658f37d53d7b16a6b133f8c0f7'
COPIES = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def helpers():
    require(PRIOR.resolve(strict=True) == PRIOR, 'canonical prior data helper')
    before = PRIOR.lstat(); raw = PRIOR.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns, s.st_mode, s.st_nlink)
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 1 << 20
        and stamp(before) == stamp(PRIOR.lstat()) and len(raw) == before.st_size
        and hashlib.sha256(raw).hexdigest() == PRIOR_SHA, 'unchanged authenticated owned validator')
    P = types.ModuleType('ar4_previous_publication'); P.__file__ = str(PRIOR)
    exec(compile(raw, str(PRIOR), 'exec'), P.__dict__)
    H = P.helper(); H.body(PRIOR)
    return P, H


def add(H, name, raw, source):
    path = Path(name)
    require(not path.is_absolute() and '..' not in path.parts and name not in COPIES
        and len(raw) == source['bytes'] and H.digest(raw) == source['sha256'], 'unique byte-exact copy')
    COPIES[name] = (raw, source)


def package(P, H, value, plan):
    record = value['supervisor_manifest']
    require(record['path'] == str(E / PACKAGE / 'manifest.json') and record['sha256'] == PACKAGE_SHA,
            'actual frozen AR4 package')
    manifest = P.doc(H, record)
    sources = {row['path']: dict(row, path=str(E / PACKAGE / row['path'])) for row in manifest['files']}
    require(len(sources) == len(manifest['files']) == 17 and value['controller'] == sources['run.py'], 'seventeen members')
    K = {}
    for row in ast.parse(P.read(H, sources['intake.py'])).body:
        if isinstance(row, ast.Assign) and len(row.targets) == 1 and isinstance(row.targets[0], ast.Name):
            try: K[row.targets[0].id] = ast.literal_eval(row.value)
            except (ValueError, TypeError): pass
    require(manifest['schema'] == K['PACKAGE_SCHEMA'] and set(sources) == K['PACKAGE_FILES']
        and manifest['pure_tests'] == K['PURE_TESTS'] == 74, 'closed actual policy74 source contract')
    public = H.document(POLICY / 'result.json', POLICY_SHA)
    for name, pin in {**sources, 'manifest.json': record}.items():
        raw = P.read(H, pin); retained = POLICY / 'source' / name
        require(H.body(retained) == raw and public['files']['source/' + name]['sha256'] == pin['sha256'],
                'same already-published source, not a new source qualification')
    tested = P.doc(H, plan['supervisor_tests'])
    require(H.body(POLICY / 'pure/complete.json') == P.read(H, plan['supervisor_tests'])
        and tested['schema'] == K['PURE_SCHEMA'] and tested['passed'] is True and tested['tests'] == 74
        and tested['manifest_sha256'] == PACKAGE_SHA and tested['controller_sha256'] == K['TEST_RUNNER_SHA']
        and tested['errors'] == tested['failures'] == tested['skipped'] == 0
        and tested['source_postchecks_passed'] is True and tested['sources_before'] == plan['supervisor_test_sources']
        and tested['test_inventory_before'] == tested['test_inventory_after']
        and P.doc(H, tested['sources_before']) == P.doc(H, tested['sources_after']) == sources,
        'same published bounded policy74 receipt')
    require(public['suite']['receipt'] == plan['supervisor_tests'] and public['suite']['passed'] == 74,
        'same published actual policy receipt')
    P.read(H, tested['transcript'])
    # Load only authenticated record parsers; never intake, run.py or policy tests.
    absent = object(); names = ('stage_core', 'smoke_validation', 'decode_validation')
    previous = {name: sys.modules.get(name, absent) for name in names}
    try:
        for name in names:
            module = types.ModuleType(name); module.__file__ = sources[name + '.py']['path']
            sys.modules[name] = module
            exec(compile(P.read(H, sources[name + '.py']), module.__file__, 'exec'), module.__dict__)
    finally:
        for name, old in previous.items():
            if old is absent: sys.modules.pop(name, None)
            else: sys.modules[name] = old
    return module, K, sources


def provenance(P, H, value, plan, review):
    prior = H.document(RUNTIME_PUBLIC / 'result.json', RUNTIME_SHA)
    for role in ('parent', 'worker'):
        name = 'inputs/' + role + '-runtime-review.json'
        pin = plan[role + '_runtime_review']
        require(pin == prior['published_files'][name]['source_pin']
            and P.read(H, pin) == H.body(RUNTIME_PUBLIC / name), 'unchanged published CPU1037 runtime review')
        runtime = P.doc(H, pin)
        require(runtime['reviewed'] is True and runtime['authority'] == 'none'
            and runtime['binary'] == value[role] == prior['selected_runtime'][role]
            and runtime['gpu_execution'] is runtime['production_authority'] is False,
            'recorded separate root runtime review, not new audit execution')
    emission = H.document(EMISSION_PUBLIC / 'result.json', EMISSION_SHA)
    raw = H.body(EMISSION_PUBLIC / 'complete.json')
    require(H.digest(raw) == plan['prefix_emission_complete']['sha256']
        and len(raw) == plan['prefix_emission_complete']['bytes'], 'published actual emission body')
    completed = H.parse(raw)
    prefix = value['prefix_provenance']
    require(prefix == review['prefix_provenance'] and emission['publication_passed'] is True
        and plan['prefix_emission_complete'] == prefix['emission'] == emission['complete']
        and plan['prefix_emission_owner'] == prefix['owner'] == emission['owner']
        and plan['prefix_image'] == prefix['original_image'] == emission['artifacts']['emitted/artifact.hsaco']
        and value['selected_runtime']['image'] == plan['prefix_image']
        and prefix['cpu'] == plan['prefix_cpu'] and prefix['producer_recipe'] == completed['producer_recipe'],
        'distinct actual checked producer and indexed emission selection')
    for key in ('producer', 'producer_owner', 'producer_generation', 'retained_handoff',
                'consumer', 'consumer_owner', 'consumer_source', 'consumer_proposal', 'consumer_tools', 'descriptor'):
        require(prefix[key] == emission[key], 'published linked generation: ' + key)
    cpu = P.doc(H, plan['prefix_cpu'])
    require((plan['prefix_cpu']['bytes'], plan['prefix_cpu']['sha256']) ==
        (68573, '8dcb4f2b326ab909c52039273515f44eb4f88cd91a967c8816f866d5514c864a')
        and prefix['source_manifest'] == cpu['overlay'], 'unchanged arithmetic CPU source identity')
    require(prefix['native_table_generation_unchanged'] is True
        and all(prefix[key] is False for key in ('compiler_binary_bodies_replayed', 'consumer_binary_bodies_replayed',
            'fresh_checked_lowering', 'fresh_checked_replay', 'full_compiler_cohort_requalified',
            'numerical_acceptance', 'runtime_requirements_discharged', 'table_generation_equality'))
        and emission['producer_aggregate_passed'] is False and emission['producer_consumer_generations_distinct'] is True,
        'historical producer remains failed; no merged or fresh compiler generation')
    return prior['runtime_audits']


def seconds(value):
    require(type(value) in (int, float) and math.isfinite(value) and value >= 0, 'recorded finite host elapsed seconds')
    return value


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary data-only Python')
    require(len(sys.argv) == 3 and re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'LOCAL_COMPLETE_PATH ACTUAL_SHA')
    path = Path(sys.argv[1]); directory = path.parent
    require(path.name == 'complete.json' and directory.parent == L
        and re.fullmatch(r'prefix-rope-indexed-ar4-gpu-v228-v[1-9][0-9]{0,8}', directory.name), 'actual case namespace')
    P, H = helpers(); value = H.document(path, sys.argv[2]); gpu_pin = P.original(H, path)
    case_files, case_bytes = H.tree(directory)
    plan = P.doc(H, value['plan']); request = P.doc(H, plan['request'])
    require(value['plan']['sha256'] == PLAN_SHA and plan['output_label'] == directory.name
        and value['schema'] == 'ferric-p228-rope-indexed-ar4-gpu-v1' and value['passed'] is True
        and value['failures'] == [] and type(value['native_attempts']) is int and value['native_attempts'] == 1
        and type(value['retries']) is int and value['retries'] == 0 and value['gpu_execution_requested'] is True
        and value['full_forward'] is True and value['own_output_trajectory_checked'] is True
        and value['old_native_equality_required'] is value['teacher_forced_token_parity_required'] is False
        and all(value[k] is False for k in P.FALSE), 'actual passed own-output AR4 structural observation only')
    bindings = P.BINDINGS + ['decode_review', 'layer_comparison', 'mlp_image', 'mlp_cpu', 'mlp_lowering_complete', 'mlp_lowering_owner',
        'prefix_image', 'prefix_cpu', 'prefix_emission_complete', 'prefix_emission_owner']
    for key in bindings:
        require((value[key + '_complete'] if key in ('parent_cpu', 'worker_cpu') else value[key]) == plan[key],
                'plan/receipt identity: ' + key)
    V, K, sources = package(P, H, value, plan)
    cpu_public_path = F / 'qualification/projection-ar4-cpu-v1/result.json'
    cpu_path = cpu_public_path.with_name('complete.json')
    cpu = H.document(cpu_path, plan['parent_cpu']['sha256'])
    require(len(H.body(cpu_path)) == plan['parent_cpu']['bytes'], 'published CPU completion exact extent')
    public = H.document(cpu_public_path)
    require(plan['parent_cpu'] == plan['worker_cpu'] == public['cpu_receipt']['original']
        and (plan['parent_cpu']['bytes'], plan['parent_cpu']['sha256']) == K['JOINT_CPU']
        and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
        and cpu['tests_passed'] == 1037 and cpu['tests_ignored'] == 4 and len(cpu['phases']) == 87
        and public['controller'] == cpu['controller'] and cpu['controller']['sha256'] == K['CPU_CONTROLLER_SHA'], 'actual CPU1037')
    for role in ('parent', 'worker'):
        require(value[role] == value['selected_runtime'][role] == public['selected_binaries'][K[role.upper() + '_NAME']]['binary']
            and (value[role]['bytes'], value[role]['sha256']) == K['BINARIES'][role], 'qualified selected ELF identity only')
    baseline = P.doc(H, value['baseline']); comparison = P.doc(H, value['layer_comparison'])
    require((value['baseline']['bytes'], value['baseline']['sha256']) == K['SILU_CAPTURE']
        and baseline['passed'] is True and baseline['failures'] == [] and baseline['pre_swiglu_arrays_equal'] is True
        and baseline['pre_swiglu_array_count'] == 22 and baseline['captured_arrays'] == 28
        and value['selected_runtime']['image'] == plan['prefix_image']
        and value['selected_runtime']['image'] != baseline['selected_runtime']['image']
        and (value['layer_comparison']['bytes'], value['layer_comparison']['sha256']) == K['LAYER_COMPARISON']
        and comparison['completed'] is True and comparison['native_outer'] == value['baseline']
        and comparison['numerical_acceptance'] is False, 'prior layer provenance, not AR4 output equality')
    for key, constant in (('projection_image', 'IMAGE'), ('lowering_complete', 'LOWERING'),
            ('mlp_cpu', 'SILU_CPU'), ('mlp_lowering_complete', 'SILU_LOWERING'),
            ('mlp_lowering_owner', 'SILU_OWNER'), ('mlp_image', 'SILU_IMAGE')):
        require((value[key]['bytes'], value[key]['sha256']) == K[constant] and value[key] == baseline[key],
                'unchanged image/source provenance: ' + key)
    require(value['inspection_complete']['sha256'] == K['INSPECTION_SHA']
        and value['inspection_complete'] == baseline['inspection_complete'], 'unchanged checked projection inspection')
    old = P.doc(H, baseline['request'])['layer']; decode = request['decode']
    for key in ('source', 'images', 'expected_bundle_id', 'expected_model_id', 'device_ids', 'prompt',
                'dispatch_timeout_ms', 'child_deadline_ms'):
        require(decode[key] == old[key], 'unchanged original workload/bootstrap: ' + key)
    rust_pin = lambda p: dict(p, sha256=list(bytes.fromhex(p['sha256'])))
    require(decode['prefix_image'] == rust_pin(value['prefix_image']) == rust_pin(value['selected_runtime']['image'])
        and (value['prefix_image']['bytes'], value['prefix_image']['sha256']) == K['PREFIX_IMAGE']
        and decode['prefix_image'] != old['prefix_tiles_image']
        and decode['tiles_image'] == old['mlp_tiles_image'] == rust_pin(value['mlp_image'])
        and decode['worker'] == rust_pin(value['worker']) and request['projection_residual_image'] == rust_pin(value['projection_image'])
        and decode['mode'] == 'autoregressive' and decode['evidence_directory'] == str(E / directory.name / 'native')
        and decode['session'] != old['session'], 'AR4 selection preserves current images and original copy')
    native = value['retained_native']; require(set(native) == V.FILES | {'complete.json'}, 'fourteen native files')
    for name, pin in native.items():
        require(pin['path'] == str(E / directory.name / 'native' / name), 'native case namespace')
    bodies = {name: P.read(H, native[name]) for name in V.FILES}; summary = P.read(H, native['complete.json'])
    checked = V.validate(summary, bodies, request)
    require(checked == value['checked'] == P.doc(H, value['observation'])
        and value['captured_tensor_rows'] == checked['captured_tensor_rows'] == 152
        and value['captured_payloads'] == checked['captured_payloads'] == 4
        and checked['terminal_state_count'] == 576 and checked['own_output_trajectory_checked'] is True,
        'actual152 slices/576 states and own lowest-index argmax recurrence')
    P.owned(H, value, plan, summary, checked, V, baseline)
    review = P.doc(H, plan['decode_review'])
    require(review['schema'] == K['REVIEW_SCHEMA'] and review['reviewed'] is True and review['authority'] == 'none'
        and review['gpu_attempts'] == 1 and review['output_label'] == directory.name
        and all(review[k] == plan[k] for k in bindings if k != 'decode_review')
        and all(review[k] is False for k in K['REVIEW_FALSE']), 'root engineering review, not numerical acceptance')
    audits = provenance(P, H, value, plan, review)
    expected_case = {'complete.json', 'observation.json'} | {'native/' + name for name in native}
    expected_case |= {f'{name}/{filename}' for name, leaf in value['leaves'].items() for filename in leaf['retained_files']}
    expected_case |= {f'{side}-{index}-topology.json' for side in ('before', 'after') for index in range(3)}
    require({str(p.relative_to(directory)) for p in case_files} == expected_case and len(expected_case) == 57,
            'closed successful case with all native buffers and seven owned leaves')
    inputs = {'plan.json': value['plan'], 'request.json': plan['request'], 'decode-review.json': plan['decode_review'],
        'parent-runtime-review.json': plan['parent_runtime_review'], 'worker-runtime-review.json': plan['worker_runtime_review']}
    for name, pin in inputs.items(): add(H, 'inputs/' + name, P.read(H, pin), pin)
    add(H, 'complete.json', H.body(path), gpu_pin)
    add(H, 'observation.json', P.read(H, value['observation']), value['observation'])
    for name, leaf in value['leaves'].items():
        for filename, pin in leaf['retained_files'].items(): add(H, 'leaves/' + name + '/' + filename, P.read(H, pin), pin)
    for side in ('before', 'after'):
        for index, audit in enumerate(value[side + '_audits']):
            pin = audit['topology']; add(H, f'audits/{side}-{index}-topology.json', P.read(H, pin), pin)
    for name, pin in native.items():
        if name.endswith('.json'): add(H, 'native/' + name, P.read(H, pin), pin)
    self_path = Path(__file__).resolve(); add(H, 'publish.py', H.body(self_path), P.pin(H, self_path))
    exporter = self_path.with_name('export.py')
    add(H, 'export.py', H.body(exporter), P.pin(H, exporter))
    timings = dict(scope='recorded host wall-clock elapsed seconds, including control and setup; not device time',
        supervisor=seconds(value['elapsed_seconds']),
        owned_parent=seconds(P.doc(H, value['leaves']['parent']['result'])['elapsed_seconds']))
    rows = [dict(position=i, generation=i + 1, input_token=checked['input_tokens'][i], output_token=checked['output_tokens'][i],
        observation=native[f'observation-{i}.bin'], control=native[f'control-{i}.bin']) for i in range(4)]
    result = dict(schema='ferric-p228-rope-indexed-ar4-native-publication-v1', authority='none', gpu_observation=gpu_pin,
        plan=value['plan'], request=plan['request'], decode_review=plan['decode_review'], runtime_audits=audits,
        reused_runtime_publication=P.pin(H, RUNTIME_PUBLIC / 'result.json'),
        linked_emission_publication=P.pin(H, EMISSION_PUBLIC / 'result.json'), prefix_provenance=value['prefix_provenance'],
        supervisor_manifest=value['supervisor_manifest'], supervisor_sources=sources,
        policy_publication=P.pin(H, POLICY / 'result.json'), pure=plan['supervisor_tests'], pure_tests=74,
        cpu=plan['parent_cpu'], cpu_publication=P.pin(H, cpu_public_path), selected_runtime=value['selected_runtime'],
        image_provenance={k: value[k] for k in ('projection_image', 'lowering_complete', 'inspection_complete',
            'mlp_image', 'mlp_cpu', 'mlp_lowering_complete', 'mlp_lowering_owner',
            'prefix_image', 'prefix_cpu', 'prefix_emission_complete', 'prefix_emission_owner')},
        baseline_provenance=value['baseline'], prior_layer_comparison=value['layer_comparison'],
        mode='autoregressive', seed_token=9112, native_attempts=1, retries=0, completed_forwards=4,
        captured_tensor_rows=152, captured_payloads=4, terminal_state_count=576,
        own_output_trajectory_checked=True, trajectory=rows, host_elapsed_seconds=timings,
        structural=checked, retained_native=native, owned_children=value['owned_children'],
        recorded_gpu_execution=True, recorded_idle_audits_replayed=True, structural_validator_replayed=True,
        case_file_count=len(case_files), case_bytes=case_bytes,
        locally_rehashed=[dict(path=str(p), **record) for p, record in sorted(H.CHECKED.items())],
        published_files={name: dict(bytes=len(raw), sha256=H.digest(raw), source_pin=source)
            for name, (raw, source) in sorted(COPIES.items())},
        binary_captures_in_git=False, executable_or_weight_bodies_in_git=False,
        all_transitive_inputs_replayed=False, selected_executable_bodies_locally_rehashed=False,
        dynamic_library_bodies_locally_rehashed=False, source_image_bodies_locally_rehashed=False,
        runtime_tool_records_replayed=False, runtime_audits_reexecuted=False, frozen_controller_reexecuted=False,
        linked_compiler_source_and_tool_bodies_replayed=False, emission_reexecuted=False,
        top_level_observer_reaping_independently_verified=False, old_native_equality_required=False,
        teacher_forced_token_parity_required=False, numerical_comparison_performed=False,
        full_forward_acceptance=False, **{key: False for key in P.FALSE})
    table = '| Position | Input Token | Own Output Argmax |\n| ---: | ---: | ---: |\n'
    table += ''.join(f"| {r['position']} | {r['input_token']} | {r['output_token']} |\n" for r in rows)
    table += '\nNative trajectory only; no framework token agreement or numerical acceptance is implied.\n'
    require(Q.parent.resolve(strict=True) == Q.parent, 'canonical publication parent')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} <= {'README.md'}, 'fresh publication/root README only')
        if os.path.lexists(Q / 'README.md'): H.body(Q / 'README.md')
    for checked_path in list(H.CHECKED): H.body(checked_path)
    outputs = {name: raw for name, (raw, _) in COPIES.items()}; outputs['trajectory.md'] = table.encode()
    result['generated_files'] = {'trajectory.md': dict(bytes=len(outputs['trajectory.md']), sha256=H.digest(outputs['trajectory.md']))}
    outputs['result.json'] = H.json_bytes(result)
    Q.mkdir(mode=0o755, exist_ok=True)
    for name, raw in outputs.items():
        target = Q / name; target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream: stream.write(raw)
        require(H.body(target) == raw, 'published byte identity')
    for checked_path in list(H.CHECKED): H.body(checked_path)
    require({str(p.relative_to(Q)) for p in Q.rglob('*') if p.is_file()} == set(outputs)
        | ({'README.md'} if (Q / 'README.md').exists() else set()), 'closed published tree')
    print(H.json_bytes(dict(result=P.pin(H, Q / 'result.json'), files=len(outputs))).decode(), end='')


if __name__ == '__main__':
    main()
