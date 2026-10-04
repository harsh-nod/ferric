"""Publish an actual retained SiLU TF4 capture, never a projected native result."""
import ast
import hashlib
import os
from pathlib import Path
import re
import stat
import sys
import types

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/silu-materialized-decode-native-v1'
PRIOR = L / 'proposals/p228-projection-residual-decode-native-publication-v1/publish.py'
PRIOR_SHA = 'cb4974eb7fda4281df656106aba83f7ff1c44a2b66dfe3f56e7b9a6ad6a961ca'
PACKAGE = 'p228-silu-materialized-decode-gpu-v1'
PACKAGE_SHA = '47afc73e3333ff0705002a16b1a296780d22c119676bba142f07c2eb9fe80c31'
PURE_SHA = 'cd95c4739bfab5f2bc3d01c53218302577d218c2ae9e25ef9fc7ed5d30cb93cf'
PLAN_SHA = '30f7a284aeb1ea0674e9f0b4828cb718189ff2038f741823a69cbfbe97eb9baa'
ASSEMBLER_SHA = '56c94333ccf3e2fc8057b16bf9e4066fdae1cf5637b2f74505aa344462ed7ec3'
WRAPPER = 'run_silu_materialized_decode_gpu_pure_p228_v1.py'
COPIES = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def helpers():
    require(PRIOR.resolve(strict=True) == PRIOR, 'canonical prior publication helper')
    before = PRIOR.lstat(); raw = PRIOR.read_bytes()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 1 << 20
        and before == PRIOR.lstat() and len(raw) == before.st_size
        and hashlib.sha256(raw).hexdigest() == PRIOR_SHA, 'unchanged authenticated ownership/data helper')
    P = types.ModuleType('silu_tf4_previous_publication'); P.__file__ = str(PRIOR)
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
            'actual frozen SiLU TF4 package')
    manifest = P.doc(H, record)
    sources = {row['path']: dict(row, path=str(E / PACKAGE / row['path'])) for row in manifest['files']}
    require(len(sources) == len(manifest['files']) == 14 and value['controller'] == sources['run.py'], 'fourteen frozen members')
    for name, pin in sources.items():
        add(H, 'source/' + name, P.read(H, pin), pin)
    add(H, 'source/manifest.json', P.read(H, record), record)
    K = {}
    for row in ast.parse(P.read(H, sources['intake.py'])).body:
        if isinstance(row, ast.Assign) and len(row.targets) == 1 and isinstance(row.targets[0], ast.Name):
            try: K[row.targets[0].id] = ast.literal_eval(row.value)
            except (ValueError, TypeError): pass
    require(manifest['schema'] == K['PACKAGE_SCHEMA'] and set(sources) == K['PACKAGE_FILES']
        and manifest['pure_tests'] == K['PURE_TESTS'] == 43, 'actual closed43 package contract')
    test_pin = plan['supervisor_tests']; tested = P.doc(H, test_pin)
    require(test_pin['sha256'] == PURE_SHA and tested['schema'] == K['PURE_SCHEMA']
        and tested['passed'] is True and tested['tests'] == 43
        and tested['manifest_sha256'] == PACKAGE_SHA and tested['controller_sha256'] == K['TEST_RUNNER_SHA']
        and tested['errors'] == tested['failures'] == tested['skipped'] == 0
        and tested['source_postchecks_passed'] is True and tested['sources_before'] == plan['supervisor_test_sources']
        and tested['source_sha256'] == tested['sources_before']['sha256']
        and tested['test_inventory_before'] == tested['test_inventory_after']
        and P.doc(H, tested['sources_before']) == P.doc(H, tested['sources_after']) == sources,
        'actual pure43 source and outcome joins')
    require(all(tested[key] is False for key in ('native_execution', 'gpu_execution', 'numerical_acceptance',
        'full_model_acceptance', 'production_authority', 'performance_claim')), 'policy tests only')
    for name, pin in [('complete.json', test_pin), ('sources-before.json', tested['sources_before']),
                      ('sources-after.json', tested['sources_after']), ('tests.log', tested['transcript'])]:
        require(pin['path'] == str(Path(test_pin['path']).parent / name), 'pure output namespace')
        add(H, 'pure/' + name, P.read(H, pin), pin)
    wrapper = dict(sources[WRAPPER], path=str(E / WRAPPER))
    require(wrapper['sha256'] == K['TEST_RUNNER_SHA'], 'exact tested external wrapper')
    add(H, 'tools/' + WRAPPER, P.read(H, wrapper), wrapper)
    # Load only the three immutable data validators, not intake or the supervisor.
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


def assembly(P, H, value, plan):
    base = Path(value['plan']['path']).parent
    local = L / base.name / 'assembly.json'; record = P.original(H, local); assembled = P.doc(H, record)
    require(assembled['schema'] == 'ferric-p228-silu-materialized-decode-root-assembly-v1'
        and assembled['plan'] == value['plan'] and assembled['assembler']['sha256'] == ASSEMBLER_SHA
        and assembled['explicit_root_decisions_copied'] is True and assembled['automatic_approval'] is False
        and assembled['gpu_execution'] is assembled['numerical_acceptance'] is assembled['performance_claim'] is False,
        'actual root-reviewed data-only assembly')
    expected = {'plan.json': value['plan'], 'request.json': plan['request'], 'decode-review.json': plan['decode_review'],
        'parent-runtime-review.json': plan['parent_runtime_review'], 'worker-runtime-review.json': plan['worker_runtime_review']}
    require(assembled['outputs'] == expected, 'exact five assembled output pins')
    for name, pin in expected.items():
        require(pin['path'] == str(base / name), 'closed input directory')
        add(H, 'inputs/' + name, P.read(H, pin), pin)
    add(H, 'inputs/assembly.json', P.read(H, record), record)
    for key in ('configuration', 'root_notes', 'assembler', 'replay_dependency'):
        pin = assembled[key]; path = Path(pin['path'])
        require(path.is_relative_to(L) and P.pin(H, path) == pin, 'authentic local-only assembly dependency')
        suffix = '.py' if key in ('assembler', 'replay_dependency') else '.json'
        add(H, 'assembly-source/' + key + suffix, H.body(path), pin)
    return record


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary data-only Python')
    require(len(sys.argv) == 3 and re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'LOCAL_COMPLETE_PATH ACTUAL_SHA')
    path = Path(sys.argv[1]); directory = path.parent
    require(path.name == 'complete.json' and directory.parent == L
        and re.fullmatch(r'prefix-silu-materialized-decode-gpu-v228-v[1-9][0-9]{0,8}', directory.name), 'actual retained case namespace')
    P, H = helpers(); value = H.document(path, sys.argv[2]); gpu_pin = P.original(H, path)
    case_files, case_bytes = H.tree(directory)
    plan = P.doc(H, value['plan']); request = P.doc(H, plan['request'])
    require(value['plan']['sha256'] == PLAN_SHA and plan['output_label'] == directory.name
        and value['schema'] == 'ferric-p228-silu-materialized-decode-gpu-v1' and value['passed'] is True
        and value['failures'] == [] and type(value['native_attempts']) is int and value['native_attempts'] == 1
        and type(value['retries']) is int and value['retries'] == 0 and value['gpu_execution_requested'] is True
        and value['full_forward'] is True and value['old_native_equality_required'] is False
        and all(value[k] is False for k in P.FALSE), 'actual completed TF4 structural capture only')
    bindings = P.BINDINGS + ['decode_review', 'layer_comparison', 'mlp_image', 'mlp_cpu', 'mlp_lowering_complete', 'mlp_lowering_owner']
    for key in bindings:
        require((value[key + '_complete'] if key in ('parent_cpu', 'worker_cpu') else value[key]) == plan[key], 'plan/receipt identity: ' + key)
    V, K, sources = package(P, H, value, plan)
    cpu = P.doc(H, plan['parent_cpu']); cpu_public_path = F / 'qualification/projection-residual-decode-cpu-v1/result.json'
    public = H.document(cpu_public_path)
    require(plan['parent_cpu'] == plan['worker_cpu'] == public['cpu_receipt']['original']
        and (plan['parent_cpu']['bytes'], plan['parent_cpu']['sha256']) == K['JOINT_CPU']
        and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
        and cpu['tests_passed'] == 1022 and cpu['tests_ignored'] == 4 and len(cpu['phases']) == 87
        and public['controller'] == cpu['controller'] and cpu['controller']['sha256'] == K['CPU_CONTROLLER_SHA'], 'actual CPU1022')
    for role in ('parent', 'worker'):
        require(value[role] == value['selected_runtime'][role] == public['selected_binaries'][K[role.upper() + '_NAME']]['binary']
            and (value[role]['bytes'], value[role]['sha256']) == K['BINARIES'][role], 'unchanged qualified selected ELF')
        P.read(H, value[role])
    baseline = P.doc(H, value['baseline']); comparison = P.doc(H, value['layer_comparison'])
    require((value['baseline']['bytes'], value['baseline']['sha256']) == K['SILU_CAPTURE']
        and baseline['passed'] is True and baseline['failures'] == [] and baseline['pre_swiglu_arrays_equal'] is True
        and baseline['pre_swiglu_array_count'] == 22 and baseline['captured_arrays'] == 28
        and value['selected_runtime']['image'] == baseline['selected_runtime']['image']
        and (value['layer_comparison']['bytes'], value['layer_comparison']['sha256']) == K['LAYER_COMPARISON']
        and comparison['completed'] is True and comparison['native_outer'] == value['baseline']
        and comparison['numerical_acceptance'] is False, 'authentic completed SiLU layer and separate diagnostics')
    for key, constant in (('projection_image', 'IMAGE'), ('lowering_complete', 'LOWERING'),
            ('mlp_cpu', 'SILU_CPU'), ('mlp_lowering_complete', 'SILU_LOWERING'),
            ('mlp_lowering_owner', 'SILU_OWNER'), ('mlp_image', 'SILU_IMAGE')):
        require((value[key]['bytes'], value[key]['sha256']) == K[constant] and value[key] == baseline[key], 'actual already-selected image provenance: ' + key)
        P.read(H, value[key])
    require(value['inspection_complete']['sha256'] == K['INSPECTION_SHA']
        and value['inspection_complete'] == baseline['inspection_complete'], 'unchanged projection inspection')
    P.read(H, value['inspection_complete'])
    old = P.doc(H, baseline['request'])['layer']; decode = request['decode']
    for key in ('source', 'images', 'expected_bundle_id', 'expected_model_id', 'device_ids', 'prompt',
                'dispatch_timeout_ms', 'child_deadline_ms'):
        require(decode[key] == old[key], 'unchanged original workload/bootstrap: ' + key)
    rust_pin = lambda p: dict(p, sha256=list(bytes.fromhex(p['sha256'])))
    require(decode['prefix_image'] == old['prefix_tiles_image'] == rust_pin(value['selected_runtime']['image'])
        and decode['tiles_image'] == old['mlp_tiles_image'] == rust_pin(value['mlp_image'])
        and decode['worker'] == rust_pin(value['worker']) and request['projection_residual_image'] == rust_pin(value['projection_image'])
        and decode['mode'] == 'teacher_forced' and decode['evidence_directory'] == str(E / directory.name / 'native')
        and decode['session'] != old['session'], 'existing plain TF4 route selects exact SiLU and residual images')
    native = value['retained_native']; require(set(native) == V.FILES | {'complete.json'}, 'fourteen retained native files')
    for name, pin in native.items():
        require(pin['path'] == str(E / directory.name / 'native' / name), 'native case namespace')
    bodies = {name: P.read(H, native[name]) for name in V.FILES}; summary = P.read(H, native['complete.json'])
    checked = V.validate(summary, bodies, request)
    require(checked == value['checked'] == P.doc(H, value['observation'])
        and value['captured_tensor_rows'] == checked['captured_tensor_rows'] == 152
        and value['captured_payloads'] == checked['captured_payloads'] == 4, 'four actual payloads/152 structural tensor rows')
    P.owned(H, value, plan, summary, checked, V, baseline)
    review = P.doc(H, plan['decode_review'])
    require(review['schema'] == K['REVIEW_SCHEMA'] and review['reviewed'] is True and review['authority'] == 'none'
        and review['gpu_attempts'] == 1 and review['output_label'] == directory.name
        and all(review[k] == plan[k] for k in bindings if k != 'decode_review')
        and all(review[k] is False for k in K['REVIEW_FALSE']), 'explicit root engineering review, no transferred math authority')
    for role in ('parent', 'worker'):
        runtime = P.doc(H, plan[role + '_runtime_review']); prior = P.doc(H, baseline[role + '_runtime_review'])
        require(runtime['reviewed'] is True and runtime['authority'] == 'none' and runtime['binary'] == value[role]
            and runtime['gpu_execution'] is runtime['production_authority'] is False
            and runtime['host'] == prior['host'] and runtime['boot_id'] == prior['boot_id'], 'recorded executable runtime review')
    assembly_pin = assembly(P, H, value, plan)
    add(H, 'complete.json', H.body(path), gpu_pin)
    add(H, 'observation.json', P.read(H, value['observation']), value['observation'])
    for name, leaf in value['leaves'].items():
        for filename, pin in leaf['retained_files'].items():
            add(H, 'leaves/' + name + '/' + filename, P.read(H, pin), pin)
    for side in ('before', 'after'):
        for index, audit in enumerate(value[side + '_audits']):
            pin = audit['topology']; add(H, f'audits/{side}-{index}-topology.json', P.read(H, pin), pin)
    for name, pin in native.items():
        if name.endswith('.json'):
            add(H, 'native/' + name, P.read(H, pin), pin)
    for source, target in ((Path(__file__).resolve(), 'publish.py'), (PRIOR, 'tools/prior-publish.py'), (P.HELPER, 'tools/data-reader.py')):
        add(H, target, H.body(source), P.pin(H, source))
    result = dict(schema='ferric-p228-silu-materialized-decode-native-publication-v1', authority='none',
        gpu_observation=gpu_pin, plan=value['plan'], request=plan['request'], decode_review=plan['decode_review'], assembly=assembly_pin,
        parent_runtime_review=plan['parent_runtime_review'], worker_runtime_review=plan['worker_runtime_review'],
        supervisor_manifest=value['supervisor_manifest'], sources=sources, pure=plan['supervisor_tests'], pure_tests=43,
        cpu=plan['parent_cpu'], cpu_publication=P.pin(H, cpu_public_path), selected_runtime=value['selected_runtime'],
        projection_image=value['projection_image'], lowering=value['lowering_complete'], inspection=value['inspection_complete'],
        baseline_provenance=value['baseline'], layer_comparison=value['layer_comparison'],
        **{key: value[key] for key in ('mlp_image', 'mlp_cpu', 'mlp_lowering_complete', 'mlp_lowering_owner')},
        mode='teacher_forced', native_attempts=1, retries=0, completed_forwards=4, captured_tensor_rows=152, captured_payloads=4,
        structural=checked, retained_native=native, leaves=value['leaves'],
        audits=dict(before=value['before_audits'], after=value['after_audits']), owned_children=value['owned_children'],
        recorded_gpu_execution=True, recorded_idle_audits_replayed=True, structural_validator_replayed=True,
        case_file_count=len(case_files), case_bytes=case_bytes,
        locally_rehashed=[dict(path=str(p), **record) for p, record in sorted(H.CHECKED.items())],
        published_files={name: dict(bytes=len(raw), sha256=H.digest(raw), source_pin=source)
            for name, (raw, source) in sorted(COPIES.items())},
        binary_captures_in_git=False, all_transitive_inputs_replayed=False, selected_executable_bodies_locally_rehashed=True,
        dynamic_library_bodies_locally_rehashed=False, runtime_audits_reexecuted=False, frozen_controller_reexecuted=False,
        top_level_observer_reaping_independently_verified=False, old_native_equality_required=False,
        numerical_comparison_performed=False, full_forward_acceptance=False, **{key: False for key in P.FALSE})
    require(Q.parent.resolve(strict=True) == Q.parent, 'canonical publication parent')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} <= {'README.md'}, 'fresh publication/root README only')
        if os.path.lexists(Q / 'README.md'): H.body(Q / 'README.md')
    for path in list(H.CHECKED): H.body(path)
    Q.mkdir(mode=0o755, exist_ok=True)
    outputs = {name: raw for name, (raw, _) in COPIES.items()}; outputs['result.json'] = H.json_bytes(result)
    for name, raw in outputs.items():
        target = Q / name; target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream: stream.write(raw)
        require(H.body(target) == raw, 'published byte identity')
    for path in list(H.CHECKED): H.body(path)
    require({str(p.relative_to(Q)) for p in Q.rglob('*') if p.is_file()} == set(outputs)
        | ({'README.md'} if (Q / 'README.md').exists() else set()), 'closed published tree')
    print(H.json_bytes(dict(result=P.pin(H, Q / 'result.json'), files=len(outputs))).decode(), end='')


if __name__ == '__main__':
    main()
