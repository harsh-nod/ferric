"""Data-only three-file publication of one actual projection-residual capture."""
import argparse
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
Q = F / 'qualification/projection-residual-native-capture-v1'
HELPER = F / 'qualification/layer0-native-capture-v1/tools/publish.py'
HELPER_SHA = '3181b2f41a51ee94c331b137f2807580c326f7f6dc53c429163d8465309e2cb8'
PACKAGE = 'p228-projection-residual-capture-gpu-v1'
PACKAGE_SHA = '4bcfdd5c53c78e2c6e41cebb889a2fb39cc113d8c5aa0799afb45c1396cc92fc'
PURE_SHA = '763050083ca48d606e3e76e94978ecb792a944fd568247eab454176b5926f955'
BASE_CAPTURE_SHA = '632a779159c60bae46aa7952c99519943fd83ca11b83de7d22bd1b9a9cb5742a'
COMPARISON_SHA = '08bb62113ff319662dea2b65d1c04b77b023c9397495e205b582c2bafb32cd0c'
MATH_PURE_SHA = '00f8b30490a7566d449ffe4b0dde0fe6a53fc035b31162c5445ef9ea1b1ed3f3'
BINDINGS = ('baseline baseline_capture parent_cpu worker_cpu parent worker request projection_image '
    'lowering_complete inspection_complete parent_runtime_review worker_runtime_review').split()


def require(ok, message):
    if not ok: raise RuntimeError(message)


def helper():
    require(HELPER.resolve(strict=True) == HELPER, 'canonical published data reader')
    before = HELPER.lstat(); raw = HELPER.read_bytes()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 1 << 20
        and before == HELPER.lstat() and len(raw) == before.st_size
        and hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'exact published data reader')
    h = types.ModuleType('retained_publication_data'); h.__file__ = str(HELPER)
    exec(compile(raw, str(HELPER), 'exec'), h.__dict__)
    h.body(HELPER)
    return h


def pin(H, path):
    raw = H.body(path)
    return dict(path=str(path), bytes=len(raw), sha256=H.digest(raw))


def original(H, path):
    relative = path.relative_to(L)
    if relative.parts[0] == 'proposals': relative = Path(*relative.parts[1:])
    return dict(pin(H, path), path=str(E / relative))


def read(H, record):
    record = H.normalize(record); relative = Path(record['path']).relative_to(E)
    path = L / ('proposals' if relative.parts[0].startswith(('p228-', 'p227-')) else '') / relative
    require(original(H, path) == record, 'retained original FilePin: ' + record['path'])
    return H.body(path)


def doc(H, record):
    return H.parse(read(H, record))


def package(H, value, plan):
    manifest = doc(H, value['supervisor_manifest'])
    require(value['supervisor_manifest']['path'] == str(E / PACKAGE / 'manifest.json')
        and value['supervisor_manifest']['sha256'] == PACKAGE_SHA
        and manifest['schema'] == 'ferric-p228-projection-residual-capture-gpu-package-v1'
        and manifest['pure_tests'] == 34, 'frozen supervisor')
    sources = {r['path']: dict(r, path=str(E / PACKAGE / r['path'])) for r in manifest['files']}
    require(len(sources) == len(manifest['files']) == 10 and value['controller'] == sources['run.py'], 'closed package')
    for record in sources.values(): read(H, record)
    constants = {}
    for s in ast.parse(read(H, sources['intake.py'])).body:
        if isinstance(s, ast.Assign) and len(s.targets) == 1 and isinstance(s.targets[0], ast.Name):
            try: constants[s.targets[0].id] = ast.literal_eval(s.value)
            except (ValueError, TypeError): pass
    tested = doc(H, plan['supervisor_tests'])
    require(plan['supervisor_tests']['sha256'] == PURE_SHA and tested['passed'] is True and tested['tests'] == 34
        and tested['schema'] == constants['PURE_SCHEMA'] and tested['manifest_sha256'] == PACKAGE_SHA
        and tested['controller_sha256'] == constants['TEST_RUNNER_SHA']
        and tested['errors'] == tested['failures'] == tested['skipped'] == 0
        and tested['source_postchecks_passed'] is True and tested['sources_before'] == plan['supervisor_test_sources']
        and doc(H, tested['sources_before']) == doc(H, tested['sources_after']) == sources, 'actual 34-test qualification')
    read(H, tested['transcript'])
    # Only authenticated data validators are evaluated, never intake or GPU controllers.
    missing = object(); previous = sys.modules.get('layer_validation', missing)
    try:
        for name in ('layer_validation', 'capture_validation'):
            module = types.ModuleType('retained_projection_' + name); module.__file__ = sources[name + '.py']['path']
            exec(compile(read(H, sources[name + '.py']), module.__file__, 'exec'), module.__dict__)
            if name == 'layer_validation': sys.modules[name] = module
    finally:
        if previous is missing: sys.modules.pop('layer_validation', None)
        else: sys.modules['layer_validation'] = previous
    return module, constants, sources


def owned(H, value, plan, summary, checked, CV, baseline):
    directory = E / plan['output_label']; starts, results = {}, {}
    names = {'parent', *(s + '-' + str(i) for s in ('before', 'after') for i in range(3))}
    require(set(value['leaves']) == names, 'seven owned leaves')
    for name in sorted(names):
        leaf = value['leaves'][name]; retained = leaf['retained_files']; row = doc(H, leaf['result']); H.natural(row)
        require(set(retained) == {'command.json', 'started.json', 'stdout', 'stderr', 'result.json'}
            and retained['result.json'] == leaf['result'], 'five retained leaf records')
        for filename, record in retained.items():
            require(record['path'] == str(directory / name / filename), 'case-contained leaf'); read(H, record)
        require(all(row[k] == retained[k + '.json' if k in ('command', 'started') else k]
            for k in ('command', 'started', 'stdout', 'stderr')), 'owned record joins')
        start, command = doc(H, row['started']), doc(H, row['command']); gpu = name == 'parent'
        argv = [plan['parent']['path'], '--request', plan['request']['path'], '--capture-projection-residual-layer-zero',
            '--allow-unauthenticated-machine-code'] if gpu else H.SMI
        require(command == dict(argv=argv, env=H.ENV, cwd=str(E.parents[1]), deadline_seconds=4000 if gpu else 30,
            affinity=[8, 9], nice=10, address_space_bytes=(32 if gpu else 12) << 30,
            file_cap_bytes=64 << 20, stream_cap_bytes=8 << 20, gpu_execution_requested=gpu)
            and row['gpu_execution_requested'] is gpu and start['command_sha256'] == row['command']['sha256'], 'bounded command')
        require(start['parent'] in [r['identity'] for r in row['lineage'] if r.get('event') == 'owned'], 'actual owned parent')
        if gpu:
            require(read(H, row['stdout']) == summary + b'\n', 'actual parent stdout')
            CV.child_marker(read(H, row['stderr']), checked['closed_child_pids'][0])
        else:
            require(read(H, row['stderr']) == b'' and doc(H, row['stdout']) == [dict(gpu=i,
                process_list=[dict(process_info='No running processes detected')]) for i in range(2)], 'empty audit')
        starts[name], results[name] = start, row
    require(len({s['supervisor_pid'] for s in starts.values()}) == 1
        and len({(s['parent']['pid'], s['parent']['starttime']) for s in starts.values()}) == 7, 'one supervisor/seven incarnations')
    identity = lambda s: [{k: v for k, v in r.items() if k not in ('gpu_busy', 'memory_busy', 'vram_used')} for r in s['devices']]
    old = doc(H, baseline['before_audits'][0]['topology'])
    for side in ('before', 'after'):
        require(len(value[side + '_audits']) == 3, 'three audits per side'); samples = []
        for i, audit in enumerate(value[side + '_audits']):
            require(audit['process_result'] == value['leaves'][f'{side}-{i}']['result']
                and audit['topology']['path'] == str(directory / f'{side}-{i}-topology.json'), 'audit joins')
            sample = doc(H, audit['topology']); samples.append(sample)
            require(identity(sample) == identity(old) and len(sample['devices']) == 2
                and all(r['gpu_busy'] == r['memory_busy'] == 0 for r in sample['devices']), 'same idle recorded devices')
        require(all(len({s['devices'][r]['vram_used'] for s in samples}) == 1 for r in range(2)), 'stable recorded VRAM')
    child = value['owned_children']; actual = [r['identity'] for r in results['parent']['lineage'] if r.get('event') == 'owned']
    require(child['parent'] == starts['parent']['parent'] and child['synthesized_child_identity'] is False
        and child['parent_asserted_close_and_reap'] is True and child['outer_groups_absent'] is True
        and len(child['workers']) == 1, 'one naturally closed child')
    worker = child['workers'][0]
    require(worker['pid'] == checked['closed_child_pids'][0] and worker['outer_pidfd_observed'] is (worker['identity'] is not None)
        and (worker['identity'] in actual if worker['outer_pidfd_observed'] else not any(r['pid'] == worker['pid'] for r in actual)),
        'honest child identity')


def comparison(H, path, sha, gpu_pin, gpu):
    value = H.document(path, sha); math = value['comparison']; inputs = math['inputs']
    require(value['schema'] == 'ferric-p228-projection-residual-comparison-observation-v1' and value['completed'] is True
        and value['native_outer'] == gpu_pin and value['baseline_outer'] == gpu['baseline_capture']
        and value['native_supervisor_manifest'] == gpu['supervisor_manifest'] and value['source_postchecks_passed'] is True
        and value['controller']['sha256'] == COMPARISON_SHA and value['native_structural_replayed'] is True, 'actual comparison joins')
    read(H, value['controller']); tested = doc(H, value['comparison_tests'])
    require(value['comparison_tests']['sha256'] == MATH_PURE_SHA and tested['passed'] is True and tested['tests'] == 20
        and tested['errors'] == tested['failures'] == tested['skipped'] == 0 and tested['source_postchecks_passed'] is True
        and doc(H, tested['sources_before']) == doc(H, tested['sources_after']) == value['comparison_sources'], 'actual math20 qualification')
    consumed = {r['original']['path']: r for r in value['consumed']}
    require(len(consumed) == len(value['consumed']) and all(r['original']['bytes'] == r['retained']['bytes']
        and r['original']['sha256'] == r['retained']['sha256'] for r in consumed.values()), 'unique transport identities')
    # The comparator joins the worker identity through CPU/capture records; it
    # consumes tensors and the image, not the executable body.
    consumed_inputs = [record for key, record in inputs.items() if key != 'selected_worker']
    for record in [gpu_pin, value['controller'], value['comparison_tests'], *consumed_inputs, *math['candidate_files'].values()]:
        require(consumed.get(record['path'], {}).get('original') == record, 'actual consumed selected input')
    require(inputs['candidate_capture'] == gpu['retained_native']['summary.json'] and inputs['current_tf4'] == gpu['baseline']
        and inputs['selected_image'] == gpu['projection_image'] and inputs['selected_worker'] == gpu['worker']
        and math['candidate_files'] == {k: v for k, v in gpu['retained_native'].items() if k != 'summary.json'}, 'exact candidate body joins')
    residuals, rows = math['conditional_residual_comparisons'], math['comparisons']
    require(math['schema'] == 'ferric-p228-projection-residual-comparison-v1' and math['authority'] == 'none'
        and math['position'] == 0 and math['input_token'] == 9112 and math['conditional_replay_performed'] is True
        and math['genuine_independent_framework_outputs'] is True and math['unchanged_pre_residual_count'] == 14
        and math['conditional_residual_words'] == 16384 and math['comparable_rows'] == len(rows) == 24
        and len(residuals) == 4 and {(r['stage'], r['rank']) for r in residuals} == {('output', 0), ('output', 1), ('down', 0), ('down', 1)}
        and all(type(r['byte_equal']) is bool and r['elements'] == 4096 for r in residuals)
        and math['conditional_residuals_exact'] is all(r['byte_equal'] for r in residuals), 'four residual rows and 24 measured rows')
    require(all(value[k] is False for k in ('gpu_execution', 'native_process_launched', 'numerical_acceptance',
        'full_model_correctness', 'performance_claim', 'production_authority')) and math['acceptance_threshold'] is None
        and all(math[k] is False for k in ('ordering_is_a_causal_proof', 'projection_gemm_equivalence_proven',
        'numerical_acceptance', 'full_layer_numerics_accepted', 'full_model_correctness', 'performance_measured', 'gpu_execution')),
        'no numerical or performance authority')
    return dict(receipt=original(H, path), controller=value['controller'], tests=value['comparison_tests'],
        sources=value['comparison_sources'], framework_outer=value['framework_outer'], consumed_pairs=len(consumed),
        residual_rows=residuals, framework_rows=rows, conditioning=math['conditional_residual_conditioning'],
        conditional_residuals_exact=math['conditional_residuals_exact'], arithmetic_premises=math['arithmetic_premises'],
        earliest_observable_divergence=math['earliest_observable_divergence'],
        candidate_hidden_matches_old_tf4=math['candidate_hidden_matches_old_tf4'], numerical_acceptance=False)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python required')
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('case_label'); parser.add_argument('complete_sha256')
    parser.add_argument('--comparison', nargs=2, metavar=('LOCAL_COMPLETE', 'SHA256')); args = parser.parse_args()
    require(re.fullmatch(r'prefix-projection-residual-capture-gpu-v228-v[1-9][0-9]{0,8}', args.case_label)
        and re.fullmatch('[0-9a-f]{64}', args.complete_sha256), 'actual completed case and digest')
    H = helper(); directory = L / args.case_label; value = H.document(directory / 'complete.json', args.complete_sha256)
    gpu_pin = original(H, directory / 'complete.json'); plan = doc(H, value['plan']); request = doc(H, plan['request'])
    require(value['schema'] == 'ferric-p228-projection-residual-capture-gpu-v1' and value['passed'] is True and value['failures'] == []
        and type(value['native_attempts']) is int and value['native_attempts'] == 1 and type(value['retries']) is int and value['retries'] == 0
        and value['gpu_execution_requested'] is True and value['current_tf4_hidden_equal'] is None
        and value['old_hidden_equality_required'] is False and value['pre_residual_arrays_equal'] is True
        and value['conditional_residual_checks_performed'] is False and value['captured_arrays'] == 28
        and all(value[k] is False for k in H.FALSE) and plan['output_label'] == args.case_label, 'actual structural capture only')
    for key in BINDINGS + ['capture_review']:
        require((value[key + '_complete'] if key in ('parent_cpu', 'worker_cpu') else value[key]) == plan[key], 'case/plan binding: ' + key)
    CV, constants, sources = package(H, value, plan)
    cpu = doc(H, plan['parent_cpu']); public_path = F / 'qualification/projection-residual-runtime-v1/result.json'; public = H.document(public_path)
    require(plan['parent_cpu'] == plan['worker_cpu'] == public['cpu_receipt']['original']
        and (plan['parent_cpu']['bytes'], plan['parent_cpu']['sha256']) == constants['JOINT_CPU']
        and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == [] and cpu['tests_passed'] == 988
        and cpu['tests_ignored'] == 4 and public['controller'] == cpu['controller']
        and cpu['controller']['sha256'] == constants['CPU_CONTROLLER_SHA'], 'published actual CPU988')
    for role in ('parent', 'worker'):
        require(value[role] == value['selected_runtime'][role] == public['selected_binaries'][constants[role.upper() + '_NAME']]['binary']
            and (value[role]['bytes'], value[role]['sha256']) == constants['BINARIES'][role], 'selected qualified ELF identity')
    baseline, old = doc(H, value['baseline']), doc(H, value['baseline_capture'])
    require(value['baseline']['sha256'] == H.BASE_SHA and baseline['passed'] is True
        and value['baseline_capture']['sha256'] == BASE_CAPTURE_SHA and old['passed'] is True
        and old['baseline'] == value['baseline'] and old['current_tf4_hidden_equal'] is True
        and value['selected_runtime']['image'] == old['selected_runtime']['image'], 'unchanged genuine baseline lineage')
    require((value['projection_image']['bytes'], value['projection_image']['sha256']) == constants['IMAGE']
        and (value['lowering_complete']['bytes'], value['lowering_complete']['sha256']) == constants['LOWERING']
        and value['inspection_complete']['sha256'] == constants['INSPECTION_SHA'], 'checked image identity')
    for key in ('projection_image', 'lowering_complete', 'inspection_complete'): read(H, value[key])
    native = value['retained_native']; require(set(native) == CV.BODY | {'summary.json'}, 'twelve native files')
    for name, record in native.items(): require(record['path'] == str(E / args.case_label / 'native' / name), 'native path')
    bodies = {name: read(H, native[name]) for name in CV.BODY}; summary = read(H, native['summary.json'])
    require(value['baseline_capture_payload'] == old['retained_native']['candidate-capture.bin'], 'prior full capture binding')
    checked = CV.validate(summary, bodies, request, read(H, value['baseline_capture_payload']),
        doc(H, old['retained_native']['candidate-bootstrap.json'])['input'])
    require(checked == value['checked'] == doc(H, value['observation']), 'full structural replay')
    owned(H, value, plan, summary, checked, CV, baseline)
    review = doc(H, plan['capture_review'])
    require(review['schema'] == constants['REVIEW_SCHEMA'] and review['reviewed'] is True and review['authority'] == 'none'
        and review['gpu_attempts'] == 1 and review['output_label'] == args.case_label
        and all(review[k] == plan[k] for k in BINDINGS) and all(review[k] is False for k in constants['REVIEW_FALSE']), 'root review')
    for role in ('parent', 'worker'):
        runtime = doc(H, plan[role + '_runtime_review']); prior = doc(H, baseline[role + '_runtime_review'])
        require(runtime['reviewed'] is True and runtime['authority'] == 'none' and runtime['binary'] == value[role]
            and runtime['gpu_execution'] is runtime['production_authority'] is False
            and runtime['host'] == prior['host'] and runtime['boot_id'] == prior['boot_id'], 'original runtime review')
    table = dict(schema='ferric-p228-projection-residual-native-table-v1', capture=gpu_pin, comparison=None, numerical_acceptance=False)
    if args.comparison: table['comparison'] = comparison(H, Path(args.comparison[0]), args.comparison[1], gpu_pin, value)
    outputs = {'table.json': H.json_bytes(table), 'publish.py': H.body(Path(__file__).resolve())}
    result = dict(schema='ferric-p228-projection-residual-native-publication-v1', authority='none', gpu_observation=gpu_pin,
        plan=value['plan'], capture_review=plan['capture_review'], parent_runtime_review=plan['parent_runtime_review'],
        worker_runtime_review=plan['worker_runtime_review'], supervisor_manifest=value['supervisor_manifest'], sources=sources,
        pure=plan['supervisor_tests'], pure_tests=34, cpu=plan['parent_cpu'], cpu_publication=pin(H, public_path),
        data_reader=pin(H, HELPER), publisher=pin(H, Path(__file__).resolve()), selected_runtime=value['selected_runtime'],
        projection_image=value['projection_image'], lowering=value['lowering_complete'], inspection=value['inspection_complete'],
        baseline=value['baseline'], baseline_capture=value['baseline_capture'], native_attempts=1, retries=0,
        layer=0, position=0, token=9112, captured_arrays=28, retained_native=native, stages=checked['stages'],
        leaves=value['leaves'], audits=dict(before=value['before_audits'], after=value['after_audits']), owned_children=value['owned_children'],
        pre_residual_arrays_equal=True, full_kv_checked=True, gpu_execution_observed=True, structural_validator_replayed=True,
        retained={name: dict(bytes=len(raw), sha256=H.digest(raw)) for name, raw in outputs.items()},
        locally_rehashed=[dict(path=str(path), **record) for path, record in H.CHECKED.items()],
        binary_captures_in_git=False, all_transitive_inputs_replayed=False, all_comparison_consumed_bodies_rehashed_here=False,
        selected_executable_bodies_locally_rehashed=False, dynamic_library_bodies_locally_rehashed=False,
        runtime_audits_reexecuted=False, frozen_controller_reexecuted=False, numerical_comparison_reexecuted=False,
        top_level_observer_reaping_independently_verified=False, **{k: False for k in H.FALSE})
    require(Q.parent.resolve(strict=True) == Q.parent, 'canonical publication parent')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} == {'README.md'}, 'fresh output or root README only')
        H.body(Q / 'README.md')
    for path in list(H.CHECKED): H.body(path)
    outputs['result.json'] = H.json_bytes(result); Q.mkdir(mode=0o755, exist_ok=True)
    for name, raw in outputs.items():
        with (Q / name).open('xb') as stream: stream.write(raw)
        require(H.body(Q / name) == raw, 'published byte identity')
    for path in list(H.CHECKED): H.body(path)
    print(H.json_bytes(dict(result=pin(H, Q / 'result.json'), files=3)).decode(), end='')


if __name__ == '__main__': main()
