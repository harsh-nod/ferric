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
Q = F / 'qualification/projection-ar4-native-v1'
PRIOR = L / 'proposals/p228-projection-residual-decode-native-publication-v1/publish.py'
PRIOR_SHA = 'cb4974eb7fda4281df656106aba83f7ff1c44a2b66dfe3f56e7b9a6ad6a961ca'
PACKAGE = 'p228-projection-ar4-decode-gpu-v1'
PACKAGE_SHA = '7526f2ad568de0759c6f3bae544608f25033c664d779542fb85484a62400d731'
POLICY = F / 'qualification/projection-ar4-supervisor-v1'
POLICY_SHA = '7de93a2d6b49da3dd8162a9efd3742404bba3fc723657678a6fa9cad9ec6b342'
PLAN_SHA = 'c6b1fe024354f76ad545683fd792b89ca07a7f9b2a6eb2e06cea1aa1a4c61f0c'
AUDITS = {
    'parent': 'dcd49b291d045a20a505b7853e27035b1ca7d4dc8712da417530540637e45b92',
    'worker': '1a216653d6e7798286f52b504584b41f35fd933f47967b50e096f3b7c6b97255',
}
COPIES = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def helpers():
    require(PRIOR.resolve(strict=True) == PRIOR, 'canonical prior data helper')
    before = PRIOR.lstat(); raw = PRIOR.read_bytes()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 1 << 20
        and before == PRIOR.lstat() and len(raw) == before.st_size
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
    require(len(sources) == len(manifest['files']) == 14 and value['controller'] == sources['run.py'], 'fourteen members')
    K = {}
    for row in ast.parse(P.read(H, sources['intake.py'])).body:
        if isinstance(row, ast.Assign) and len(row.targets) == 1 and isinstance(row.targets[0], ast.Name):
            try: K[row.targets[0].id] = ast.literal_eval(row.value)
            except (ValueError, TypeError): pass
    require(manifest['schema'] == K['PACKAGE_SCHEMA'] and set(sources) == K['PACKAGE_FILES']
        and manifest['pure_tests'] == K['PURE_TESTS'] == 52, 'closed actual policy52 source contract')
    public = H.document(POLICY / 'result.json', POLICY_SHA)
    for name, pin in {**sources, 'manifest.json': record}.items():
        raw = P.read(H, pin); retained = POLICY / 'supervisor' / name
        require(H.body(retained) == raw and public['files']['supervisor/' + name]['sha256'] == pin['sha256'],
                'same already-published source, not a new source qualification')
    tested = P.doc(H, plan['supervisor_tests'])
    require(H.body(POLICY / 'tests/supervisor/complete.json') == P.read(H, plan['supervisor_tests'])
        and tested['schema'] == K['PURE_SCHEMA'] and tested['passed'] is True and tested['tests'] == 52
        and tested['manifest_sha256'] == PACKAGE_SHA and tested['controller_sha256'] == K['TEST_RUNNER_SHA']
        and tested['errors'] == tested['failures'] == tested['skipped'] == 0
        and tested['source_postchecks_passed'] is True and tested['sources_before'] == plan['supervisor_test_sources']
        and tested['test_inventory_before'] == tested['test_inventory_after']
        and P.doc(H, tested['sources_before']) == P.doc(H, tested['sources_after']) == sources,
        'same published bounded policy52 receipt')
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


def runtime_audits(P, H, value, plan):
    result = {}
    for role, sha in AUDITS.items():
        label = 'projection-ar4-decode-runtime-' + role + '-v228-v1'
        directory = L / label; path = directory / 'complete.json'
        audit = H.document(path, sha); record = P.original(H, path)
        review = P.doc(H, plan[role + '_runtime_review'])
        require(audit['schema'] == 'ferric-p227-prefix-runtime-audit-v1'
            and audit['binary'] == review['binary'] == value[role]
            and audit['authority'] == review['authority'] == 'none'
            and audit['reviewed'] is False and review['reviewed'] is True
            and audit['runtime_premises_discharged'] is False
            and audit['gpu_execution'] is audit['numerical_acceptance'] is audit['production_authority'] is False
            and review['gpu_execution'] is review['production_authority'] is False,
            'recorded audit and separate explicit root review, no transferred authority')
        for key in ('host', 'boot_id', 'readelf', 'ldd', 'libraries'):
            require(review[key] == audit[key], 'actual reviewed runtime field: ' + key)
        expected = {'complete.json', 'inputs.json', 'software.json', 'topology-before.json', 'topology-after.json'}
        expected |= {f'{tool}/{name}' for tool in ('readelf', 'ldd')
            for name in ('command.json', 'started.json', 'owner.json', 'audit.json', 'stdout', 'stderr')}
        files, _ = H.tree(directory)
        require({str(p.relative_to(directory)) for p in files} == expected, 'closed seventeen-file runtime tree')
        originals = {str(p.relative_to(directory)): P.original(H, p) for p in files}
        for key, filename in (('software_audit', 'software.json'), ('topology_before', 'topology-before.json'),
                              ('topology_after', 'topology-after.json')):
            require(audit[key] == originals[filename], 'runtime selected auxiliary record')
        initial = P.doc(H, originals['inputs.json'])
        require(initial['binary'] == value[role], 'runtime initial selected binary')
        source_pins = dict(initial['source_pins'])
        for library in audit['libraries']:
            library_pin = library['resolved']
            require(source_pins.setdefault(library_pin['path'], library_pin) == library_pin, 'unchanged library source pin')
        require(source_pins == audit['source_pins'], 'initial and resolved-library pin closure, bodies not rehashed here')
        for index, tool in enumerate(('readelf', 'ldd')):
            leaf = P.doc(H, audit[tool]); owner = P.doc(H, audit['owners'][index])
            H.natural(owner['outcome'])
            require(audit[tool] == originals[f'{tool}/audit.json']
                and audit['owners'][index] == originals[f'{tool}/owner.json']
                and audit['commands'][index] == owner['command'] == originals[f'{tool}/command.json']
                and owner['started'] == originals[f'{tool}/started.json'] and owner['gpu_execution'] is False
                and leaf['exit_code'] == 0 and leaf['deadline_seconds'] == 30
                and leaf['stdout'] == originals[f'{tool}/stdout'] and leaf['stderr'] == originals[f'{tool}/stderr']
                and leaf['stderr']['bytes'] == 0
                and leaf['argv'] == (['/usr/bin/readelf', '-d', value[role]['path']] if tool == 'readelf'
                                    else ['/usr/bin/ldd', value[role]['path']]), 'natural retained runtime tool leaves')
            command = P.doc(H, owner['command']); started = P.doc(H, owner['started'])
            identities = [entry['identity'] for entry in owner['outcome']['lineage'] if entry.get('event') == 'owned']
            identity = started['parent']
            require(command == dict(argv=['/usr/bin/prlimit', '--as=2147483648', '--cpu=30', '--fsize=1048576',
                '--core=0', '--', *leaf['argv']], audit_argv=leaf['argv'], cwd=str(E.parents[1]),
                deadline_seconds=30, env=H.ENV, gpu_execution_requested=False, stream_cap_bytes=1 << 20)
                and identities == [identity] and identity['pid'] == identity['pgid'] == identity['sid']
                and identity['uid'] == 9661 and identity['ppid'] == started['supervisor_pid']
                and owner['outcome']['owned_groups'] == [identity['pgid']], 'actual bounded runtime command and owner')
        selector_name = 'audit_projection_ar4_decode_runtime.py'
        selector = audit['source_pins'][str(E / 'p228-projection-ar4-decode-runtime-v1' / selector_name)]
        raw = P.read(H, selector)
        if 'runtime/selector.py' not in COPIES: add(H, 'runtime/selector.py', raw, selector)
        else: require(COPIES['runtime/selector.py'] == (raw, selector), 'both roles used same actual selector')
        for name, pin in originals.items(): add(H, 'runtime/' + role + '/' + name, P.read(H, pin), pin)
        result[role] = record
    return result


def selector_test_observation(P, H):
    path = L / 'projection-ar4-runtime-pure-root-v228-v1.json'
    value = H.document(path, '29f4d99b133b709093eea892576abf8045703acce6f9f4259bcac108d3482390')
    require(value['schema'] == 'ferric-p228-projection-ar4-runtime-root-test-observation-v1'
        and value['observation_kind'] == 'Primary-agent observation of actual SSH tool output; not a remote supervisor receipt'
        and value['exit_code'] == 0 and value['tests'] == 12
        and value['errors'] == value['failures'] == value['skipped'] == 0
        and value['separate_test_source_postcheck_observed'] is False
        and value['full_transcript_retained_in_this_record'] is False
        and value['test_output_excerpt'] == 'Ran 12 tests in 0.016s\n\nOK\n'
        and all(value[k] is False for k in ('native_execution', 'gpu_execution', 'numerical_acceptance',
                                           'performance_claim', 'production_authority')), 'modest actual selector test observation')
    for name, sha in value['expected_transported_source_sha256'].items():
        require(name in {'audit_projection_ar4_decode_runtime.py', 'test_audit_projection_ar4_decode_runtime.py', 'README.md'},
                'selector source basename')
        source = L / 'proposals/p228-projection-ar4-decode-runtime-v1' / name
        raw = H.body(source); require(H.digest(raw) == sha, 'current local source matches expected transported bytes')
        if name == 'audit_projection_ar4_decode_runtime.py':
            require(COPIES['runtime/selector.py'][0] == raw, 'same selector recorded by actual audits')
        else: add(H, 'runtime/source/' + name, raw, P.original(H, source))
    add(H, 'runtime/test-primary-observation.json', H.body(path), P.pin(H, path))
    return dict(observation=P.pin(H, path), observed_tests=12, evidence_kind=value['observation_kind'],
        excerpt_only=True, separate_test_source_postcheck_observed=False, full_named_transcript_replayed=False)


def seconds(value):
    require(type(value) in (int, float) and math.isfinite(value) and value >= 0, 'recorded finite host elapsed seconds')
    return value


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary data-only Python')
    require(len(sys.argv) == 3 and re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'LOCAL_COMPLETE_PATH ACTUAL_SHA')
    path = Path(sys.argv[1]); directory = path.parent
    require(path.name == 'complete.json' and directory.parent == L
        and re.fullmatch(r'prefix-projection-ar4-decode-gpu-v228-v[1-9][0-9]{0,8}', directory.name), 'actual case namespace')
    P, H = helpers(); value = H.document(path, sys.argv[2]); gpu_pin = P.original(H, path)
    case_files, case_bytes = H.tree(directory)
    plan = P.doc(H, value['plan']); request = P.doc(H, plan['request'])
    require(value['plan']['sha256'] == PLAN_SHA and plan['output_label'] == directory.name
        and value['schema'] == 'ferric-p228-projection-ar4-decode-gpu-v1' and value['passed'] is True
        and value['failures'] == [] and type(value['native_attempts']) is int and value['native_attempts'] == 1
        and type(value['retries']) is int and value['retries'] == 0 and value['gpu_execution_requested'] is True
        and value['full_forward'] is True and value['own_output_trajectory_checked'] is True
        and value['old_native_equality_required'] is value['teacher_forced_token_parity_required'] is False
        and all(value[k] is False for k in P.FALSE), 'actual passed own-output AR4 structural observation only')
    bindings = P.BINDINGS + ['decode_review', 'layer_comparison', 'mlp_image', 'mlp_cpu', 'mlp_lowering_complete', 'mlp_lowering_owner']
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
        and value['selected_runtime']['image'] == baseline['selected_runtime']['image']
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
    require(decode['prefix_image'] == old['prefix_tiles_image'] == rust_pin(value['selected_runtime']['image'])
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
    audits = runtime_audits(P, H, value, plan)
    selector_tests = selector_test_observation(P, H)
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
    timings = dict(scope='recorded host wall-clock elapsed seconds, including control and setup; not device time',
        supervisor=seconds(value['elapsed_seconds']),
        owned_parent=seconds(P.doc(H, value['leaves']['parent']['result'])['elapsed_seconds']))
    rows = [dict(position=i, generation=i + 1, input_token=checked['input_tokens'][i], output_token=checked['output_tokens'][i],
        observation=native[f'observation-{i}.bin'], control=native[f'control-{i}.bin']) for i in range(4)]
    result = dict(schema='ferric-p228-projection-ar4-native-publication-v1', authority='none', gpu_observation=gpu_pin,
        plan=value['plan'], request=plan['request'], decode_review=plan['decode_review'], runtime_audits=audits,
        runtime_selector_tests=selector_tests,
        supervisor_manifest=value['supervisor_manifest'], supervisor_sources=sources,
        policy_publication=P.pin(H, POLICY / 'result.json'), pure=plan['supervisor_tests'], pure_tests=52,
        cpu=plan['parent_cpu'], cpu_publication=P.pin(H, cpu_public_path), selected_runtime=value['selected_runtime'],
        image_provenance={k: value[k] for k in ('projection_image', 'lowering_complete', 'inspection_complete',
            'mlp_image', 'mlp_cpu', 'mlp_lowering_complete', 'mlp_lowering_owner')},
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
        runtime_audits_reexecuted=False, frozen_controller_reexecuted=False,
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
