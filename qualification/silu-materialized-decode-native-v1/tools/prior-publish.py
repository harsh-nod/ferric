"""Publish retained TF4 structure and ownership, without numerical acceptance."""
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
Q = F / 'qualification/projection-residual-decode-native-v1'
HELPER = F / 'qualification/layer0-native-capture-v1/tools/publish.py'
HELPER_SHA = '3181b2f41a51ee94c331b137f2807580c326f7f6dc53c429163d8465309e2cb8'
PACKAGE = 'p228-projection-residual-decode-gpu-v1'
PACKAGE_SHA = '6dfd492f7dc8da8bd8eaa2f8e9da9bcbe36cd94756ba321bda538b3abb6fbb35'
PLAN_SHA = 'c14d6abf8f5b14f578d3664f62a465f4a1537a1e750539e57b80e2f8498a2e92'
PURE_SHA = 'e3162fa0b19b7370c81acdc1a902eaf76a61fac2e33b2ccbb04b6f71ac53ac89'
BASE_SHA = '4f25030567c470062fd50862876bc35e93d080f1778c0be4ca36f2b39af4199a'
BINDINGS = ('baseline parent_cpu worker_cpu parent worker request projection_image lowering_complete '
    'inspection_complete parent_runtime_review worker_runtime_review').split()
FALSE = ('numerical_acceptance', 'independent_numerical_acceptance', 'independent_tensor_acceptance',
    'full_model_acceptance', 'full_model_correctness', 'performance_claim', 'production_authority',
    'paired_comparison_performed', 'native_baseline_comparison_performed', 'conditional_residual_checks_performed',
    'independent_framework_comparison_performed', 'sustained_2048_256', 'gpu_time', 'calibrated_nanoseconds',
    'cross_device_clock_alignment', 'overlap_claim')


def require(ok, message):
    if not ok: raise RuntimeError(message)


def helper():
    require(HELPER.resolve(strict=True) == HELPER, 'canonical authenticated data helper')
    before = HELPER.lstat(); raw = HELPER.read_bytes()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 1 << 20
        and before == HELPER.lstat() and len(raw) == before.st_size
        and hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'published data helper identity')
    module = types.ModuleType('retained_publication_data'); module.__file__ = str(HELPER)
    exec(compile(raw, str(HELPER), 'exec'), module.__dict__)
    module.body(HELPER)
    return module


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
    require(original(H, path) == record, 'actual original/retained FilePin')
    return H.body(path)


def doc(H, record):
    return H.parse(read(H, record))


def package(H, value, plan):
    require(value['supervisor_manifest']['path'] == str(E / PACKAGE / 'manifest.json')
        and value['supervisor_manifest']['sha256'] == PACKAGE_SHA, 'actual frozen supervisor')
    manifest = doc(H, value['supervisor_manifest'])
    sources = {r['path']: dict(r, path=str(E / PACKAGE / r['path'])) for r in manifest['files']}
    require(len(sources) == len(manifest['files']) == 12 and value['controller'] == sources['run.py'], 'closed package')
    for record in sources.values(): read(H, record)
    K = {}
    for row in ast.parse(read(H, sources['intake.py'])).body:
        if isinstance(row, ast.Assign) and len(row.targets) == 1 and isinstance(row.targets[0], ast.Name):
            try: K[row.targets[0].id] = ast.literal_eval(row.value)
            except (ValueError, TypeError): pass
    require(manifest['schema'] == K['PACKAGE_SCHEMA'] and set(sources) == K['PACKAGE_FILES']
        and manifest['pure_tests'] == K['PURE_TESTS'] == 35, 'frozen twelve-file35-test contract')
    tested = doc(H, plan['supervisor_tests'])
    require(plan['supervisor_tests']['sha256'] == PURE_SHA and tested['passed'] is True and tested['tests'] == 35
        and tested['schema'] == K['PURE_SCHEMA'] and tested['manifest_sha256'] == PACKAGE_SHA
        and tested['controller_sha256'] == K['TEST_RUNNER_SHA']
        and tested['errors'] == tested['failures'] == tested['skipped'] == 0
        and tested['source_postchecks_passed'] is True and tested['sources_before'] == plan['supervisor_test_sources']
        and tested['test_inventory_before'] == tested['test_inventory_after']
        and doc(H, tested['sources_before']) == doc(H, tested['sources_after']) == sources, 'actual pure35 qualification')
    read(H, tested['transcript'])
    # Only authenticated data validators are loaded, never intake or the supervisor.
    absent = object(); names = ('stage_core', 'smoke_validation', 'decode_validation')
    previous = {name: sys.modules.get(name, absent) for name in names}
    try:
        for name in names:
            module = types.ModuleType(name); module.__file__ = sources[name + '.py']['path']
            sys.modules[name] = module
            exec(compile(read(H, sources[name + '.py']), module.__file__, 'exec'), module.__dict__)
    finally:
        for name, old in previous.items():
            if old is absent: sys.modules.pop(name, None)
            else: sys.modules[name] = old
    return module, K, sources


def owned(H, value, plan, summary, checked, V, baseline):
    directory = E / plan['output_label']; starts, results = {}, {}
    names = {'parent', *(side + '-' + str(i) for side in ('before', 'after') for i in range(3))}
    require(set(value['leaves']) == names, 'seven owned leaves')
    for name in sorted(names):
        leaf = value['leaves'][name]; records = leaf['retained_files']; row = doc(H, leaf['result']); H.natural(row)
        require(set(leaf) == {'result', 'retained_files'} and set(records) ==
            {'command.json', 'started.json', 'stdout', 'stderr', 'result.json'}
            and records['result.json'] == leaf['result'], 'complete owned leaf roster')
        for filename, record in records.items():
            require(record['path'] == str(directory / name / filename), 'case-contained owned record'); read(H, record)
        require(all(row[k] == records[k + '.json' if k in ('command', 'started') else k]
            for k in ('command', 'started', 'stdout', 'stderr')), 'actual owned pin joins')
        start, command = doc(H, row['started']), doc(H, row['command']); gpu = name == 'parent'
        argv = [plan['parent']['path'], '--request', plan['request']['path'], '--observe-projection-residual-decode',
            '--allow-unauthenticated-machine-code'] if gpu else H.SMI
        require(command == dict(argv=argv, env=H.ENV, cwd=str(E.parents[1]), deadline_seconds=4000 if gpu else 30,
            affinity=[8, 9], nice=10, address_space_bytes=(32 if gpu else 12) << 30,
            file_cap_bytes=64 << 20, stream_cap_bytes=8 << 20, gpu_execution_requested=gpu)
            and row['gpu_execution_requested'] is gpu and start['command_sha256'] == row['command']['sha256'],
            'unchanged bounded command and new native selector')
        identity = start['parent']; actual = [r['identity'] for r in row['lineage'] if r.get('event') == 'owned']
        require(identity in actual and identity['pid'] == identity['pgid'] == identity['sid']
            and identity['uid'] == 9661 and identity['ppid'] == start['supervisor_pid']
            and identity['starttime'] > 0 and len(actual) == len({(r['pid'], r['starttime']) for r in actual})
            and set(row['owned_groups']) == {r['pgid'] for r in actual}, 'actual owned incarnations')
        if gpu:
            require(read(H, row['stdout']) == summary, 'native stdout includes exactly the retained summary LF')
            V.child_marker(read(H, row['stderr']), checked['closed_child_pids'][0])
        else:
            require(actual == [identity] and read(H, row['stderr']) == b'' and doc(H, row['stdout']) ==
                [dict(gpu=i, process_list=[dict(process_info='No running processes detected')]) for i in range(2)],
                'single CPU audit process and recorded empty devices')
        starts[name], results[name] = start, row
    require(len({s['supervisor_pid'] for s in starts.values()}) == 1
        and len({(s['parent']['pid'], s['parent']['starttime']) for s in starts.values()}) == 7, 'one supervisor/seven leaf incarnations')
    identity = lambda sample: [{k: v for k, v in r.items() if k not in ('gpu_busy', 'memory_busy', 'vram_used')}
        for r in sample['devices']]
    old = doc(H, baseline['before_audits'][0]['topology'])
    for side in ('before', 'after'):
        require(len(value[side + '_audits']) == 3, 'three audits per side'); samples = []
        for i, audit in enumerate(value[side + '_audits']):
            require(audit['process_result'] == value['leaves'][f'{side}-{i}']['result']
                and audit['topology']['path'] == str(directory / f'{side}-{i}-topology.json'), 'audit process/topology joins')
            sample = doc(H, audit['topology']); samples.append(sample)
            require(identity(sample) == identity(old) and len(sample['devices']) == 2
                and all(r['gpu_busy'] == r['memory_busy'] == 0 for r in sample['devices']), 'same recorded idle devices')
        require(all(len({s['devices'][rank]['vram_used'] for s in samples}) == 1 for rank in range(2)), 'stable recorded VRAM')
    child = value['owned_children']; parent = starts['parent']['parent']
    actual = [r['identity'] for r in results['parent']['lineage'] if r.get('event') == 'owned']
    require(child['parent'] == parent and child['synthesized_child_identity'] is False
        and child['parent_asserted_close_and_reap'] is True and child['outer_groups_absent'] is True
        and len(child['workers']) == 1 and len(actual) <= 2, 'one naturally closed worker')
    worker = child['workers'][0]
    require(worker['pid'] == checked['closed_child_pids'][0] and worker['pid'] != parent['pid']
        and worker['outer_pidfd_observed'] is (worker['identity'] is not None)
        and {r['pid'] for r in actual} <= {parent['pid'], worker['pid']}
        and (worker['identity'] in actual if worker['outer_pidfd_observed'] else not any(r['pid'] == worker['pid'] for r in actual)),
        'honest recorded worker identity')
    if worker['outer_pidfd_observed']:
        row = worker['identity']
        require(row['pid'] == row['pgid'] and row['ppid'] == parent['pid'] and row['sid'] == parent['sid']
            and row['uid'] == parent['uid'] and row['starttime'] >= parent['starttime'], 'worker ownership lineage')


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary data-only Python')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('complete_path'); parser.add_argument('complete_sha256'); args = parser.parse_args()
    path = Path(args.complete_path); directory = path.parent
    require(path.name == 'complete.json' and directory.parent == L
        and re.fullmatch(r'prefix-projection-residual-decode-gpu-v228-v[1-9][0-9]{0,8}', directory.name)
        and re.fullmatch('[0-9a-f]{64}', args.complete_sha256), 'actual retained case path and digest')
    H = helper(); value = H.document(path, args.complete_sha256); gpu_pin = original(H, path)
    case_files, case_bytes = H.tree(directory)
    plan = doc(H, value['plan']); request = doc(H, plan['request'])
    require(value['plan']['sha256'] == PLAN_SHA and plan['output_label'] == directory.name
        and value['schema'] == 'ferric-p228-projection-residual-decode-gpu-v1' and value['passed'] is True
        and value['failures'] == [] and type(value['native_attempts']) is int and value['native_attempts'] == 1
        and type(value['retries']) is int and value['retries'] == 0 and value['gpu_execution_requested'] is True
        and value['full_forward'] is True and value['old_native_equality_required'] is False
        and all(value[k] is False for k in FALSE), 'actual TF4 structural execution only')
    for key in BINDINGS + ['decode_review']:
        require((value[key + '_complete'] if key in ('parent_cpu', 'worker_cpu') else value[key]) == plan[key], 'plan/receipt binding')
    V, K, sources = package(H, value, plan)
    cpu = doc(H, plan['parent_cpu']); public_path = F / 'qualification/projection-residual-decode-cpu-v1/result.json'
    public = H.document(public_path)
    require(plan['parent_cpu'] == plan['worker_cpu'] == public['cpu_receipt']['original']
        and (plan['parent_cpu']['bytes'], plan['parent_cpu']['sha256']) == K['JOINT_CPU']
        and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
        and cpu['tests_passed'] == 1022 and cpu['tests_ignored'] == 4 and len(cpu['phases']) == 87
        and public['controller'] == cpu['controller'] and cpu['controller']['sha256'] == K['CPU_CONTROLLER_SHA'],
        'published actual CPU1022 generation')
    for role in ('parent', 'worker'):
        require(value[role] == value['selected_runtime'][role] == public['selected_binaries'][K[role.upper() + '_NAME']]['binary']
            and (value[role]['bytes'], value[role]['sha256']) == K['BINARIES'][role], 'selected qualified ELF identity')
        read(H, value[role])
    baseline = doc(H, value['baseline'])
    require(value['baseline']['sha256'] == BASE_SHA and baseline['passed'] is True
        and value['selected_runtime']['image'] == baseline['selected_runtime']['image'], 'prior capture provenance only')
    require((value['projection_image']['bytes'], value['projection_image']['sha256']) == K['IMAGE']
        and (value['lowering_complete']['bytes'], value['lowering_complete']['sha256']) == K['LOWERING']
        and value['inspection_complete']['sha256'] == K['INSPECTION_SHA'], 'checked candidate image generation')
    for key in ('projection_image', 'lowering_complete', 'inspection_complete'): read(H, value[key])
    native = value['retained_native']; require(set(native) == V.FILES | {'complete.json'}, 'fourteen native files')
    for name, record in native.items(): require(record['path'] == str(E / directory.name / 'native' / name), 'native case path')
    bodies = {name: read(H, native[name]) for name in V.FILES}; summary = read(H, native['complete.json'])
    checked = V.validate(summary, bodies, request)
    require(checked == value['checked'] == doc(H, value['observation'])
        and value['captured_tensor_rows'] == checked['captured_tensor_rows'] == 152
        and value['captured_payloads'] == checked['captured_payloads'] == 4, 'all4 payloads/152 finite structural tensor rows')
    owned(H, value, plan, summary, checked, V, baseline)
    review = doc(H, plan['decode_review'])
    require(review['schema'] == K['REVIEW_SCHEMA'] and review['reviewed'] is True and review['authority'] == 'none'
        and review['gpu_attempts'] == 1 and review['output_label'] == directory.name
        and all(review[k] == plan[k] for k in BINDINGS) and all(review[k] is False for k in K['REVIEW_FALSE']), 'root engineering review')
    for role in ('parent', 'worker'):
        runtime = doc(H, plan[role + '_runtime_review']); prior = doc(H, baseline[role + '_runtime_review'])
        require(runtime['reviewed'] is True and runtime['authority'] == 'none' and runtime['binary'] == value[role]
            and runtime['gpu_execution'] is runtime['production_authority'] is False
            and runtime['host'] == prior['host'] and runtime['boot_id'] == prior['boot_id'], 'recorded root runtime review')
    publisher = pin(H, Path(__file__).resolve()); source = H.body(Path(__file__).resolve())
    result = dict(schema='ferric-p228-projection-residual-decode-native-publication-v1', authority='none',
        gpu_observation=gpu_pin, plan=value['plan'], request=plan['request'], decode_review=plan['decode_review'],
        parent_runtime_review=plan['parent_runtime_review'], worker_runtime_review=plan['worker_runtime_review'],
        supervisor_manifest=value['supervisor_manifest'], sources=sources, pure=plan['supervisor_tests'], pure_tests=35,
        cpu=plan['parent_cpu'], cpu_publication=pin(H, public_path), data_reader=pin(H, HELPER), publisher=publisher,
        selected_runtime=value['selected_runtime'], projection_image=value['projection_image'], lowering=value['lowering_complete'],
        inspection=value['inspection_complete'], baseline_provenance=value['baseline'], mode='teacher_forced',
        native_attempts=1, retries=0, completed_forwards=4, captured_tensor_rows=152, captured_payloads=4,
        structural=checked, retained_native=native, leaves=value['leaves'],
        audits=dict(before=value['before_audits'], after=value['after_audits']), owned_children=value['owned_children'],
        recorded_gpu_execution=True, recorded_idle_audits_replayed=True, structural_validator_replayed=True,
        case_file_count=len(case_files), case_bytes=case_bytes,
        locally_rehashed=[dict(path=str(p), **record) for p, record in sorted(H.CHECKED.items())],
        binary_captures_in_git=False, all_transitive_inputs_replayed=False, selected_executable_bodies_locally_rehashed=True,
        dynamic_library_bodies_locally_rehashed=False, runtime_audits_reexecuted=False, frozen_controller_reexecuted=False,
        top_level_observer_reaping_independently_verified=False, old_native_equality_required=False,
        numerical_comparison_performed=False, full_forward_acceptance=False, **{k: False for k in FALSE})
    require(Q.parent.resolve(strict=True) == Q.parent, 'canonical publication parent')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} <= {'README.md'}, 'fresh output/root README only')
        if os.path.lexists(Q / 'README.md'): H.body(Q / 'README.md')
    for p in list(H.CHECKED): H.body(p)
    outputs = {'publish.py': source, 'result.json': H.json_bytes(result)}; Q.mkdir(mode=0o755, exist_ok=True)
    for name, raw in outputs.items():
        with (Q / name).open('xb') as stream: stream.write(raw)
        require(H.body(Q / name) == raw, 'published byte identity')
    for p in list(H.CHECKED): H.body(p)
    print(H.json_bytes(dict(result=pin(H, Q / 'result.json'), files=2)).decode(), end='')


if __name__ == '__main__': main()
