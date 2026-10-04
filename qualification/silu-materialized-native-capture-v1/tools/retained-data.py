"""Publish retained candidate-only layer0 bytes; no controller or native execution."""
import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys
import types

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/layer0-native-capture-v1'
PACKAGE = 'p228-layer0-native-capture-gpu-v1'
PACKAGE_SHA = '957115945b1e1a12a6ee6b7fd815a98a33b9d7f1bdc99b77293cb5556b560535'
PUBLISHER = 'p228-layer0-native-observation-publication-v1'
RUNNER = 'run_layer0_native_capture_gpu_pure_p228_v1.py'
RUNNER_SHA = 'd4a5ed455602f34754ad3d6ff4541c902cc205f1157fecd57c8595786a9925cc'
BASE = 'prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-v1'
BASE_SHA = '00e1af86b1a61894797dc1eeb3202a113d62bd3d11c8069ba6299d63f8875073'
PARENT_CPU = (326552, '78c12f822e95d50a1239f56411c5b8da9411a3fbda7510fbbf38d5644e1dffbb')
PARENT = (13480088, 'e7fb0571cfb8114ef86987525a6b882c4fdc84329022db5eabfdb131f3e3e12d')
WORKER_CPU = (213924, '41ddcf8a9cb48b970d4f9a4187de6f6de1fd0506ba098d524e82d17335030f1d')
WORKER = (5018856, 'd2af909e7fef88af5b20f4ae8f2a5779a7aee93a187169e97a939bec7d3a0fed')
SMI = ['/opt/rocm/bin/amd-smi', 'process', '--gpu', '0000:05:00.0', '0000:15:00.0', '--json']
ENV = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
    OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1', HIP_VISIBLE_DEVICES='',
    ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
FALSE = ('paired_comparison_performed', 'full_model_correctness', 'independent_numerical_acceptance',
    'numerical_acceptance', 'independent_tensor_acceptance', 'full_model_acceptance',
    'independent_framework_comparison_performed', 'full_forward', 'sustained_2048_256',
    'gpu_time', 'calibrated_nanoseconds', 'cross_device_clock_alignment', 'overlap_claim',
    'performance_claim', 'production_authority')
CHECKED = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON member')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: (_ for _ in ()).throw(ValueError('nonfinite JSON')))


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def body(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical retained file')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 64 << 20,
            'bounded unaliased regular evidence')
    raw = path.read_bytes(); after = path.lstat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_nlink, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) and len(raw) == before.st_size, 'stable retained bytes')
    value = dict(bytes=len(raw), sha256=digest(raw))
    require(CHECKED.setdefault(path, value) == value, 'retained file drift')
    return raw


def normalize(record):
    require(type(record) is dict and set(record) == {'path', 'bytes', 'sha256'}, 'closed FilePin')
    value = dict(record); path = Path(value['path'])
    require(path.is_relative_to(E) and '..' not in path.parts and str(path) == value['path']
        and type(value['bytes']) is int and 0 <= value['bytes'] <= 64 << 20, 'bounded evidence pin')
    if type(value['sha256']) is list:
        require(len(value['sha256']) == 32 and all(type(n) is int and 0 <= n <= 255 for n in value['sha256']),
                'Rust digest bytes')
        value['sha256'] = bytes(value['sha256']).hex()
    require(type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']), 'SHA256')
    return value


def remote_pin(path):
    relative = path.relative_to(L)
    if relative.parts[0] == 'proposals': relative = Path(*relative.parts[1:])
    raw = body(path)
    return dict(path=str(E / relative), bytes=len(raw), sha256=digest(raw))


def verify(record):
    record = normalize(record); relative = Path(record['path']).relative_to(E)
    path = L / relative
    if relative.parts[0] in (PACKAGE, PUBLISHER): path = L / 'proposals' / relative
    require(remote_pin(path) == record, 'actual retained pin equality')
    return path


def document(path, sha=None):
    raw = body(path)
    require(sha is None or digest(raw) == sha, 'explicit actual completion SHA')
    return parse(raw)


def referenced(value, root):
    if type(value) is dict:
        if set(value) == {'path', 'bytes', 'sha256'} and Path(value['path']).is_relative_to(root): verify(value)
        for item in value.values(): referenced(item, root)
    elif type(value) is list:
        for item in value: referenced(item, root)


def tree(directory):
    require(directory.resolve(strict=True) == directory and directory.is_dir(), 'canonical case directory')
    files, size, count = [], 0, 0
    for path in directory.rglob('*'):
        row = path.lstat(); count += 1
        require(path.resolve(strict=True) == path and (stat.S_ISREG(row.st_mode) or stat.S_ISDIR(row.st_mode)),
                'no special or aliased case members')
        if path.is_file(): size += len(body(path)); files.append(path)
        require(count <= 255 and size <= 64 << 20, 'bounded retained case tree')
    return sorted(files), size


def natural(value):
    require(type(value['exit_code']) is int and value['exit_code'] == 0 and value['reason'] is None
        and value['cleanup_signalled'] is False and value['owned_groups_absent'] is True
        and value['owned_processes_reaped'] is True, 'natural zero exit and owned reap')


def json_bytes(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')


def data_validators(package):
    # Only these authenticated, data-only modules are evaluated; never intake or run.
    hashes = {row['path']: row['sha256'] for row in package['files']}
    modules = {}
    previous = sys.modules.get('layer_validation')
    try:
        for name in ('layer_validation', 'capture_validation'):
            path = L / 'proposals' / PACKAGE / (name + '.py'); raw = body(path)
            require(digest(raw) == hashes[name + '.py'], 'exact structural data validator')
            module = types.ModuleType('retained_layer0_' + name); module.__file__ = str(path)
            exec(compile(raw, str(path), 'exec'), module.__dict__)
            modules[name] = module
            if name == 'layer_validation': sys.modules[name] = module
    finally:
        if previous is None: sys.modules.pop('layer_validation', None)
        else: sys.modules['layer_validation'] = previous
    return modules['capture_validation']


def pure_sources(plan, manifest, copies):
    pure_path = verify(plan['supervisor_tests']); pure = document(pure_path)
    require(pure['schema'] == 'ferric-p228-layer0-native-capture-gpu-pure-v1' and pure['passed'] is True
        and type(pure['tests']) is int and pure['tests'] == 40
        and pure['errors'] == pure['failures'] == pure['skipped'] == 0
        and pure['source_postchecks_passed'] is True and pure['synthetic_policy_tests_only'] is True
        and pure['manifest_sha256'] == PACKAGE_SHA and pure['controller_sha256'] == RUNNER_SHA,
        'actual passing bounded40 suite')
    before = document(verify(pure['sources_before'])); after = document(verify(pure['sources_after']))
    require(before == after and pure['sources_before'] == plan['supervisor_test_sources']
        and pure['source_sha256'] == pure['sources_before']['sha256']
        and len(manifest['files']) == len(before) == 10 and manifest['pure_tests'] == 40, 'tested source closure')
    expected, expected_by_module = set(), {}
    for member in manifest['files']:
        record = dict(member, path=str(E / PACKAGE / member['path']))
        require(before[member['path']] == record, 'manifest/tested source join')
        source = verify(record); copies.append((source, 'source/' + member['path']))
        if member['path'].startswith('test_'):
            syntax = ast.parse(body(source), filename=member['path'])
            names = {member['path'][:-3] + '.' + cls.name + '.' + method.name for cls in syntax.body
                if isinstance(cls, ast.ClassDef) for method in cls.body
                if isinstance(method, ast.FunctionDef) and method.name.startswith('test_')}
            require(len(names) == manifest['test_census'][member['path']], 'authored named test census')
            expected_by_module[member['path']] = sorted(names)
            expected.update(names)
    log = body(verify(pure['transcript'])).decode('utf-8')
    rows = re.findall(r'^(test_\w+) \(([^\n]+)\) \.\.\. ok$', log, re.MULTILINE)
    actual = {label if label.endswith('.' + name) else label + '.' + name for name, label in rows}
    require(len(rows) == len(expected) == 40 and actual == expected
        and pure['test_inventory_before'] == pure['test_inventory_after'] == expected_by_module
        and re.search(r'^Ran 40 tests in [^\n]+\n\nOK\s*$', log, re.MULTILINE), 'actual named passing inventory')
    for key in ('sources_before', 'sources_after', 'transcript'):
        path = verify(pure[key]); copies.append((path, 'pure/' + path.name))
    copies.append((pure_path, 'pure/complete.json'))
    runner = L / RUNNER
    require(digest(body(runner)) == RUNNER_SHA, 'actual bounded test runner')
    copies.append((runner, 'tools/' + RUNNER))


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python required')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('case_label'); parser.add_argument('complete_sha256')
    args = parser.parse_args()
    require(re.fullmatch(r'prefix-layer0-native-capture-gpu-v228-v[1-9][0-9]{0,8}', args.case_label)
        and re.fullmatch('[0-9a-f]{64}', args.complete_sha256), 'actual terminal case and SHA')
    directory = L / args.case_label; complete = document(directory / 'complete.json', args.complete_sha256)
    require(complete['schema'] == 'ferric-p228-layer0-native-capture-gpu-v1' and complete['passed'] is True
        and complete['failures'] == [] and type(complete['native_attempts']) is int
        and complete['native_attempts'] == 1 and complete['retries'] == 0
        and complete['gpu_execution_requested'] is True and complete['current_tf4_hidden_equal'] is True
        and complete['captured_arrays'] == 28 and all(complete[key] is False for key in FALSE),
        'actual one-layer capture only, without arithmetic authority')
    files, retained_bytes = tree(directory)
    for path in files:
        if path.suffix == '.json': referenced(document(path), E / args.case_label)
    manifest_path = verify(complete['supervisor_manifest']); manifest = document(manifest_path, PACKAGE_SHA)
    require(complete['supervisor_manifest']['path'] == str(E / PACKAGE / 'manifest.json')
        and manifest['schema'] == 'ferric-p228-layer0-native-capture-gpu-package-v1', 'actual frozen supervisor')
    run = next(row for row in manifest['files'] if row['path'] == 'run.py')
    require(complete['controller'] == dict(run, path=str(E / PACKAGE / 'run.py')), 'actual executed supervisor bytes')
    plan_path = verify(complete['plan']); plan = document(plan_path)
    request = document(verify(plan['request']))
    require(plan['schema'] == 'ferric-p228-layer0-native-capture-inputs-v1'
        and plan['output_label'] == args.case_label
        and complete['request'] == plan['request'] and complete['baseline'] == plan['baseline'], 'plan/case joins')
    require(complete['baseline']['path'] == str(E / BASE / 'complete.json')
        and complete['baseline']['sha256'] == BASE_SHA, 'actual Down2 TF4 baseline')
    baseline = document(verify(complete['baseline']))
    require(baseline['passed'] is True and baseline['failures'] == [], 'successful actual baseline')
    old_request = document(verify(baseline['request']))['decode']
    for key in ('source', 'images', 'expected_bundle_id', 'expected_model_id', 'device_ids', 'prompt',
                'dispatch_timeout_ms', 'child_deadline_ms', 'worker'):
        require(request[key] == old_request[key], 'unchanged genuine TF4 input: ' + key)
    require(request['prefix_tiles_image'] == old_request['prefix_image']
        and request['mlp_tiles_image'] == old_request['tiles_image']
        and request['evidence_directory'] == str(E / args.case_label / 'native')
        and request['session'] != old_request['session'], 'exact images and fresh capture output')
    for role, cpu, binary in (('parent', PARENT_CPU, PARENT), ('worker', WORKER_CPU, WORKER)):
        require(complete[role + '_cpu_complete'] == plan[role + '_cpu']
            and (plan[role + '_cpu']['bytes'], plan[role + '_cpu']['sha256']) == cpu
            and complete[role] == complete['selected_runtime'][role] == plan[role]
            and (plan[role]['bytes'], plan[role]['sha256']) == binary
            and complete[role + '_runtime_review'] == plan[role + '_runtime_review'], 'qualified runtime identity')
    require(complete['worker'] == baseline['worker']
        and complete['selected_runtime']['image'] == baseline['selected_runtime']['image'], 'unchanged worker and V7')
    public_parent = document(F / 'qualification/layer0-native-capture-parent-v1/result.json')
    require(public_parent['passed'] is True and public_parent['cpu_receipt'] == dict(plan['parent_cpu'],
        path=str(L / Path(plan['parent_cpu']['path']).relative_to(E))),
            'existing published actual parent CPU qualification')
    payload_pin = baseline['retained_native']['observation-0.bin']; payload = body(verify(payload_pin))
    require(len(payload) == 606976 and complete['baseline_hidden'] == dict(source=payload_pin, offset=0,
        bytes=8192, sha256=digest(payload[:8192])), 'actual current native hidden provenance')
    copies = [(manifest_path, 'source/manifest.json')]
    pure_sources(plan, manifest, copies)
    CV = data_validators(manifest)
    native = complete['retained_native']
    require(set(native) == CV.BODY | {'summary.json'}, 'all native body pins')
    bodies = {}
    for name, record in native.items():
        require(record['path'] == str(E / args.case_label / 'native' / name), 'closed capture path')
        raw = body(verify(record))
        if name != 'summary.json': bodies[name] = raw
    summary = body(verify(native['summary.json']))
    checked = CV.validate(summary, bodies, request, payload[:8192])
    require(checked == complete['checked'] == document(verify(complete['observation'])), 'actual structural replay')
    leaves = {'parent', *(side + '-' + str(i) for side in ('before', 'after') for i in range(3))}
    require(set(complete['leaves']) == leaves and len(complete['before_audits']) == len(complete['after_audits']) == 3,
            'one native leaf and six audits')
    native_leaf = None
    for name in sorted(leaves):
        leaf = complete['leaves'][name]; record = leaf['result']
        require(record['path'] == str(E / args.case_label / name / 'result.json'), 'case-contained leaf')
        value = document(verify(record)); natural(value)
        require(set(leaf['retained_files']) == {'command.json', 'started.json', 'stdout', 'stderr', 'result.json'}
            and leaf['retained_files']['result.json'] == record, 'all five owned records')
        for key in ('command', 'started', 'stdout', 'stderr'):
            filename = key + ('.json' if key in ('command', 'started') else '')
            require(value[key] == leaf['retained_files'][filename]
                and value[key]['path'] == str(E / args.case_label / name / filename), 'owned raw identity')
            verify(value[key])
        command = document(verify(value['command'])); started = document(verify(value['started']))
        gpu = name == 'parent'
        argv = ([plan['parent']['path'], '--request', plan['request']['path'], '--capture-layer-zero',
                 '--allow-unauthenticated-machine-code'] if gpu else SMI)
        require(command == dict(argv=argv, env=ENV, cwd=str(E.parents[1]), deadline_seconds=4000 if gpu else 30,
            affinity=[8, 9], nice=10, address_space_bytes=(32 if gpu else 12) << 30,
            file_cap_bytes=64 << 20, stream_cap_bytes=8 << 20, gpu_execution_requested=gpu)
            and started['command_sha256'] == value['command']['sha256']
            and value['gpu_execution_requested'] is gpu, 'exact bounded leaf command and start')
        if gpu:
            native_leaf = value
            require(body(verify(value['stdout'])) == summary + b'\n', 'actual stdout/summary bytes')
            CV.child_marker(body(verify(value['stderr'])), checked['closed_child_pids'][0])
        else:
            require(body(verify(value['stderr'])) == b'' and document(verify(value['stdout'])) == [
                dict(gpu=i, process_list=[dict(process_info='No running processes detected')]) for i in range(2)],
                'empty selected GPUs, no monitor exception')
    topologies = []
    for side in ('before', 'after'):
        for index, audit in enumerate(complete[side + '_audits']):
            name = side + '-' + str(index)
            require(audit['process_result'] == complete['leaves'][name]['result']
                and audit['topology']['path'] == str(E / args.case_label / (name + '-topology.json')), 'six audit joins')
            topologies.append(document(verify(audit['topology'])))
    baseline_topology = document(verify(baseline['before_audits'][0]['topology']))
    identity = lambda sample: [{key: value for key, value in row.items()
        if key not in ('gpu_busy', 'memory_busy', 'vram_used')} for row in sample['devices']]
    for sample in topologies:
        require(identity(sample) == identity(baseline_topology)
            and all(row['gpu_busy'] == row['memory_busy'] == 0 for row in sample['devices']),
            'same selected platform and idle counters in all six samples')
    for samples in (topologies[:3], topologies[3:]):
        for rank in range(2):
            require(len({sample['devices'][rank]['vram_used'] for sample in samples}) == 1,
                    'unchanged three-sample idle VRAM requirement')
    owned = complete['owned_children']; start = document(verify(native_leaf['started']))
    require(owned['parent'] == start['parent'] and owned['synthesized_child_identity'] is False
        and owned['parent_asserted_close_and_reap'] is True and owned['outer_groups_absent'] is True
        and len(owned['workers']) == 1 and owned['workers'][0]['pid'] == checked['closed_child_pids'][0],
        'actual parent and one child, no synthesized identity')
    actual_owned = [row['identity'] for row in native_leaf['lineage'] if row.get('event') == 'owned']
    require(owned['parent'] in actual_owned, 'owned parent observed')
    child = owned['workers'][0]
    require(child['outer_pidfd_observed'] is (child['identity'] is not None)
        and (child['identity'] in actual_owned if child['outer_pidfd_observed'] else
             not any(row['pid'] == child['pid'] for row in actual_owned)), 'honest outer child observation')
    review = document(verify(plan['capture_review']))
    require(complete['capture_review'] == plan['capture_review']
        and review['schema'] == 'ferric-p228-layer0-native-capture-engineering-review-v1'
        and review['reviewed'] is True and review['authority'] == 'none'
        and review['gpu_attempts'] == 1 and review['output_label'] == args.case_label
        and review['baseline_hidden'] == complete['baseline_hidden'], 'actual root capture review')
    for key in ('baseline', 'parent_cpu', 'worker_cpu', 'parent', 'worker', 'request',
                'parent_runtime_review', 'worker_runtime_review'):
        require(review[key] == plan[key], 'exact reviewed input binding')
    for key in ('production_authority', 'full_model_acceptance', 'independent_numerical_acceptance',
                'numerical_acceptance', 'arithmetic_prerequisites_verified', 'runtime_premises_discharged',
                'performance_claim', 'timestamp_calibration', 'clock_domain_validated',
                'cross_device_clock_alignment', 'overlap_claim', 'paired_comparison_performed', 'full_forward'):
        require(review[key] is False, 'engineering review grants no acceptance')
    for role in ('parent', 'worker'):
        runtime = document(verify(plan[role + '_runtime_review']))
        prior_runtime = document(verify(baseline[role + '_runtime_review']))
        require(runtime['schema'] == 'ferric-p227-prefix-parity-runtime-review-v1'
            and runtime['reviewed'] is True and runtime['authority'] == 'none'
            and runtime['binary'] == plan[role] and runtime['gpu_execution'] is False
            and runtime['production_authority'] is False
            and runtime['host'] == prior_runtime['host'] and runtime['boot_id'] == prior_runtime['boot_id'],
            'separate compatibility review on the same reviewed host/boot')
    copies.extend((path, 'capture/' + str(path.relative_to(directory))) for path in files if path.suffix != '.bin')
    for key in ('request', 'capture_review', 'parent_runtime_review', 'worker_runtime_review'):
        copies.append((verify(plan[key]), 'inputs/' + key.replace('_', '-') + '.json'))
    copies.extend(((plan_path, 'inputs/plan.json'), (Path(__file__).resolve(), 'tools/publish.py')))
    payloads, ledger = {}, {}
    for source, name in copies:
        require(name not in payloads and not Path(name).is_absolute() and '..' not in Path(name).parts, 'unique public path')
        payloads[name] = body(source); ledger[name] = dict(source=remote_pin(source), **CHECKED[source])
    stage_ledger = dict(schema='ferric-p228-layer0-native-stage-ledger-v1', capture=native['candidate-capture.bin'],
        captured_arrays=28, stages=checked['stages'], full_kv_checked=True,
        current_tf4_hidden_equal=True, baseline_hidden=complete['baseline_hidden'],
        binary_capture_in_git=False, numerical_acceptance=False, independent_framework_comparison_performed=False)
    payloads['stage-ledger.json'] = json_bytes(stage_ledger)
    ledger['stage-ledger.json'] = dict(bytes=len(payloads['stage-ledger.json']), sha256=digest(payloads['stage-ledger.json']),
        derived_from=[native['candidate-capture.bin'], payload_pin])
    require(Q.parent.resolve(strict=True) == Q.parent, 'canonical publication parent')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} == {'README.md'},
                'fresh publication with optional root README only')
        body(Q / 'README.md')
    for path in list(CHECKED): body(path)
    result = dict(schema='ferric-p228-layer0-native-capture-publication-v1', authority='none',
        gpu_observation=remote_pin(directory / 'complete.json'), baseline=complete['baseline'],
        parent_cpu=plan['parent_cpu'], worker_cpu=plan['worker_cpu'], selected_runtime=complete['selected_runtime'],
        parent_publication='../layer0-native-capture-parent-v1/result.json',
        supervisor_manifest=complete['supervisor_manifest'], pure=plan['supervisor_tests'], pure_tests=40,
        native_attempts=1, retries=0, natural_owned_leaf_exits=7, device_audits=6,
        layer=0, generation=1, position=0, token=9112, captured_arrays=28,
        capture=native['candidate-capture.bin'], current_tf4_hidden_equal=True,
        current_tf4_hidden_sha256=CV.HIDDEN_SHA, full_kv_checked=True,
        retained_case=dict(files=len(files), bytes=retained_bytes), retained=ledger,
        gpu_execution_observed=True, binary_captures_in_git=False,
        structural_validator_replayed=True, all_transitive_inputs_replayed=False,
        selected_executable_bodies_locally_rehashed=False, dynamic_library_bodies_locally_rehashed=False,
        runtime_audits_reexecuted=False, frozen_controller_reexecuted=False,
        top_level_observer_reaping_independently_verified=False, **{key: False for key in FALSE})
    Q.mkdir(mode=0o755, exist_ok=True)
    for name, raw in payloads.items():
        path = Q / name; path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream: stream.write(raw)
        require(body(path) == raw, 'published byte equality')
    for path in list(CHECKED): body(path)
    raw = json_bytes(result)
    with (Q / 'result.json').open('xb') as stream: stream.write(raw)
    print(json.dumps(dict(published_files=len(ledger), retained_case_files=len(files),
        result_bytes=len(raw), result_sha256=digest(raw))), flush=True)


if __name__ == '__main__':
    main()
