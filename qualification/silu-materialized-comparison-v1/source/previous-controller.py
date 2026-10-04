"""CPU-only replay of one new capture and the unchanged numerical adapter."""
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import stat
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
OLD = E / 'p228-layer0-current-comparison-v1/run.py'
OLD_SHA = '9c3db491d4f8c20890b045474bd1ab5bd8565d42916a8eb7772c0eb07c44717e'
OLD_PLAN_SHA = '67f8d16e5f9fc305330247a7946fd309fd684cdf758d53a218a02cfa49005a62'
GPU = E / 'p228-projection-residual-capture-gpu-v1'
GPU_SHA = '4bcfdd5c53c78e2c6e41cebb889a2fb39cc113d8c5aa0799afb45c1396cc92fc'
GPU_PURE_SHA = '763050083ca48d606e3e76e94978ecb792a944fd568247eab454176b5926f955'
MATH_PURE_SHA = '00f8b30490a7566d449ffe4b0dde0fe6a53fc035b31162c5445ef9ea1b1ed3f3'
OUT = E / 'projection-residual-comparison-v228-v1'


def require(ok, message):
    if not ok:
        raise ValueError(message)


def bootstrap():
    require(not sys.flags.optimize and sys.dont_write_bytecode
        and all(k not in os.environ for k in ('PYTHONOPTIMIZE', 'PYTHONPATH', 'PYTHONHOME')),
        'ordinary isolated -B Python before imports')
    require(OLD.resolve(strict=True) == OLD and OLD.is_file(), 'canonical retained helper')
    with os.fdopen(os.open(OLD, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_size <= 1 << 20, 'bounded replay helper')
        raw = stream.read((1 << 20) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) == stamp(OLD.lstat()) and len(raw) == before.st_size
        and hashlib.sha256(raw).hexdigest() == OLD_SHA, 'exact tested replay helper')
    value = types.ModuleType('retained_current_replay'); value.__file__ = str(OLD)
    exec(compile(raw, str(OLD), 'exec'), value.__dict__)
    return value


def package(J, reader, path, digest, schema):
    pin = J.actual(path / 'manifest.json', digest)[0]
    value = reader.doc(pin)
    rows = {row['path']: dict(row, path=str(path / row['path'])) for row in value['files']}
    require(value['schema'] == schema and len(rows) == len(value['files']) == 10,
            'exact frozen capture package')
    for record in rows.values(): reader(record)
    return pin, rows


def parent_leaf(J, reader, outer, directory, environment):
    entry = outer['leaves']['parent']; records = entry['retained_files']
    require(set(entry) == {'result', 'retained_files'} and set(records) == J.RAW
        and records['result.json'] == entry['result'], 'exact parent leaf records')
    for name, pin in records.items():
        require(pin['path'] == str(directory / 'parent' / name), 'case-contained parent leaf')
        reader(pin)
    value = reader.doc(entry['result'])
    require(type(value['exit_code']) is int and value['exit_code'] == 0 and value['reason'] is None
        and value['cleanup_signalled'] is False and value['owned_groups_absent'] is True
        and value['owned_processes_reaped'] is True and value['gpu_execution_requested'] is True,
        'candidate natural zero and fully reaped ownership')
    for key in ('command', 'started', 'stdout', 'stderr'):
        require(value[key] == records[key + '.json' if key in ('command', 'started') else key], 'parent raw join')
    require(reader.doc(value['command']) == dict(argv=[outer['parent']['path'], '--request', outer['request']['path'],
        '--capture-projection-residual-layer-zero', '--allow-unauthenticated-machine-code'], env=environment,
        cwd=str(J.R), deadline_seconds=4000, affinity=[8, 9], nice=10, address_space_bytes=32 << 30,
        file_cap_bytes=64 << 20, stream_cap_bytes=8 << 20, gpu_execution_requested=True),
        'actual new selector and unchanged bounded parent command')
    start = reader.doc(value['started']); parent = start['parent']
    require(set(start) == {'parent', 'supervisor_pid', 'command_sha256'}
        and start['command_sha256'] == value['command']['sha256']
        and parent['pid'] == parent['pgid'] == parent['sid'] and parent['uid'] == 9661
        and parent['ppid'] == start['supervisor_pid']
        and parent in [row['identity'] for row in value['lineage'] if row.get('event') == 'owned'],
        'candidate started identity and lineage')
    return value, start


def candidate(J, reader, pin, prior, platform_pin, manifest, sources, I, CV, runner, audit, topology):
    value = reader.doc(pin); directory = Path(pin['path']).parent
    require(directory.parent == E and Path(pin['path']).name == 'complete.json'
        and re.fullmatch(r'prefix-projection-residual-capture-gpu-v228-v[1-9][0-9]{0,8}', directory.name),
        'genuine new capture namespace')
    require(value['schema'] == 'ferric-p228-projection-residual-capture-gpu-v1' and value['passed'] is True
        and value['failures'] == [] and type(value['native_attempts']) is int and value['native_attempts'] == 1
        and type(value['retries']) is int and value['retries'] == 0 and value['gpu_execution_requested'] is True
        and value['current_tf4_hidden_equal'] is None and value['old_hidden_equality_required'] is False
        and value['pre_residual_arrays_equal'] is True and value['conditional_residual_checks_performed'] is False
        and value['captured_arrays'] == 28 and all(value[k] is False for k in J.FALSE), 'new diagnostic-only completion')
    require(value['controller'] == sources['run.py'] and value['supervisor_manifest'] == manifest,
        'actual frozen new supervisor')
    plan = reader.doc(value['plan']); I.input_shape(plan)
    require(plan['output_label'] == directory.name and value['baseline'] == plan['baseline'] == J.BASELINE
        and value['baseline_capture'] == plan['baseline_capture'] == prior['pin'], 'genuine TF4 and old layer capture')
    for key in ('parent', 'worker', 'request', 'projection_image', 'lowering_complete', 'inspection_complete',
                'parent_runtime_review', 'worker_runtime_review', 'capture_review'):
        require(value[key] == plan[key], 'actual candidate plan binding: ' + key)
    require(value['selected_runtime'] == dict(parent=value['parent'], worker=value['worker'],
        image=prior['outer']['selected_runtime']['image']), 'actual selected upstream and new executables')
    require(value['parent_cpu_complete'] == value['worker_cpu_complete'] == plan['parent_cpu'] == plan['worker_cpu']
        and I.content(plan['parent_cpu']) == I.JOINT_CPU, 'one actually qualified new parent/worker generation')
    cpu = reader.doc(plan['parent_cpu']); selected = I.qualified_cpu(cpu)
    require(cpu['controller']['sha256'] == I.CPU_CONTROLLER_SHA
        and cpu['prior_parent_cpu'] == prior['outer']['parent_cpu_complete']
        and cpu['prior_worker_cpu'] == prior['outer']['worker_cpu_complete'], 'actual CPU lineage')
    reader(cpu['controller'])
    require(reader.doc(cpu['raw']['sources-before.json']) == reader.doc(cpu['raw']['sources-after.json']), 'CPU source postchecks')
    for role, phase in (('parent', 'parent-builds'), ('worker', 'worker-build')):
        require(I.content(value[role]) == I.content(selected[role]['binary']), 'selected actual Cargo ELF')
        raw = reader(cpu['raw'][phase + '-stdout'])
        require(sum(J.parse(line) == selected[role]['artifact'] for line in raw.splitlines() if line.strip()) == 1
            and hashlib.sha256(raw).hexdigest() == cpu['phases'][phase]['stdout_sha256'], 'actual Cargo artifact stream')
    require(I.content(value['lowering_complete']) == I.LOWERING
        and value['inspection_complete']['sha256'] == I.INSPECTION_SHA
        and I.content(value['projection_image']) == I.IMAGE, 'actual selected checked image')
    lower, inspected = reader.doc(value['lowering_complete']), reader.doc(value['inspection_complete'])
    require(lower['passed'] is inspected['passed'] is True
        and lower['errors'] == lower['postcheck_errors'] == inspected['postcheck_errors'] == []
        and inspected['error'] is None and inspected['lowering_complete'] == value['lowering_complete']
        and inspected['image'] == lower['artifact']['image']
        and I.content(inspected['image']) == I.IMAGE, 'actual retained checked lowering and inspection')
    reader(value['projection_image'])
    ledger = dict(value['input_pins']); ledger.update(value['standalone_input_pins'])
    require(ledger.get(platform_pin['path']) == platform_pin, 'recorded platform admission input')
    platform = reader.doc(platform_pin)
    for role in ('parent', 'worker'):
        review = reader.doc(value[role + '_runtime_review'])
        require(review['reviewed'] is True and review['authority'] == 'none' and review['binary'] == value[role]
            and review['host'] == platform['host'] and review['boot_id'] == platform['boot_id']
            and review['production_authority'] is False, 'own original selected runtime review')
    review = reader.doc(value['capture_review'])
    require(review['schema'] == I.REVIEW_SCHEMA and review['reviewed'] is True and review['authority'] == 'none'
        and review['output_label'] == directory.name and review['gpu_attempts'] == 1
        and all(review[k] == plan[k] for k in I.REVIEW_BINDINGS)
        and all(review[k] is False for k in I.REVIEW_FALSE), 'exact original engineering review/nonclaims')
    pure = reader.doc(plan['supervisor_tests'])
    require(plan['supervisor_tests']['sha256'] == GPU_PURE_SHA and pure['schema'] == I.PURE_SCHEMA
        and pure['passed'] is True and pure['tests'] == 34 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
        and pure['manifest_sha256'] == manifest['sha256'] and pure['controller_sha256'] == I.TEST_RUNNER_SHA
        and pure['source_postchecks_passed'] is True
        and pure['sources_before'] == plan['supervisor_test_sources']
        and reader.doc(pure['sources_before']) == reader.doc(pure['sources_after']) == sources,
        'actual source-bound 34-test supervisor qualification')
    reader(pure['transcript'])
    reader(J.actual(E / 'run_projection_residual_capture_gpu_pure_p228_v1.py', I.TEST_RUNNER_SHA)[0])
    require(set(value['leaves']) == set(J.LEAVES), 'candidate seven owned leaves')
    environment = reader.doc(value['leaves']['parent']['retained_files']['command.json'])['env']
    require(type(environment) is dict and all(environment.get(k) == '' for k in
        ('CUDA_VISIBLE_DEVICES', 'HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES')), 'recorded KFD-only environment')
    leaves = {name: (parent_leaf(J, reader, value, directory, environment) if name == 'parent' else
        J.owned_leaf(reader, value, directory, name, environment)) for name in J.LEAVES}
    require(len({start['supervisor_pid'] for _, start in leaves.values()}) == 1
        and len({(start['parent']['pid'], start['parent']['starttime']) for _, start in leaves.values()}) == 7,
        'one supervisor and seven actual process incarnations')
    J.audit_samples(reader, value, directory, leaves, platform, audit, topology)
    records = value['retained_native']
    require(set(records) == CV.BODY | {'summary.json'}, 'actual candidate twelve-file roster')
    for name, record in records.items():
        require(record['path'] == str(directory / 'native' / name), 'actual candidate body path')
    files = {name: reader(records[name]) for name in CV.BODY}; summary = reader(records['summary.json'])
    parent, _ = leaves['parent']
    require(reader(parent['stdout']) == summary + b'\n', 'actual candidate parent stdout')
    request = reader.doc(value['request'])
    require(reader.doc(records['request.json']) == request, 'outer and actual retained candidate request')
    require(value['baseline_capture_payload'] == prior['outer']['retained_native']['candidate-capture.bin'],
        'actual unchanged baseline payload')
    baseline_body = reader(value['baseline_capture_payload'])
    baseline_input = reader.doc(prior['outer']['retained_native']['candidate-bootstrap.json'])['input']
    checked = CV.validate(summary, files, request, baseline_body, baseline_input)
    require(checked == value['checked'] == reader.doc(value['observation']), 'actual new structural replay')
    CV.child_marker(reader(parent['stderr']), checked['closed_child_pids'][0])
    # Reuse the actual frozen supervisor's data-only lineage function, supplying
    # only its reader interface. No context/live platform probe is invoked.
    class Pins:
        def read(self, path, digest=None, retain=True, maximum=16 << 20):
            p = parent['started']
            require(str(path) == p['path'] and digest == p['sha256'], 'closed lineage started-record read')
            raw = reader(p)
            require(len(raw) <= maximum, 'lineage read bound')
            return p, raw
    require(runner.lineage(Pins(), parent, checked) == value['owned_children'], 'actual candidate child identity replay')
    return value, records['summary.json']


def main(args):
    J = bootstrap()
    require(len(args) == 4, 'NEW_COMPLETE_PATH NEW_SHA TRANSPORT_ADDITIONS_PATH TRANSPORT_ADDITIONS_SHA')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
        and all(os.environ.get(k) == '' for k in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')),
        'fixed bounded ASROCK CPU-only run')
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        limits = [v for v in (*resource.getrlimit(kind), cap) if v != resource.RLIM_INFINITY]
        resource.setrlimit(kind, (min(limits), min(limits)))
    require(not os.path.lexists(OUT), 'fresh comparison output')
    old_plan_pin, raw = J.actual(E / 'layer0-current-comparison-inputs-v228-v1/plan.json', OLD_PLAN_SHA)
    old_plan = J.parse(raw)
    additions_pin, raw = J.actual(Path(args[2]), args[3]); additions = J.parse(raw)
    require(type(additions) is dict, 'explicit original-to-retained transport additions')
    mapping = dict(old_plan['transport'])
    for key, value in additions.items():
        require(key not in mapping or mapping[key] == value, 'no changed old transport pin')
        mapping[key] = value
    reader = J.Reader(mapping)
    own = J.actual(Path(__file__).resolve())[0]
    for p in (own, J.actual(OLD, OLD_SHA)[0], old_plan_pin, additions_pin): reader(p)
    new_pin = J.actual(Path(args[0]), args[1])[0]; reader(new_pin)
    old_manifest, old_sources = package(J, reader, J.GPU, J.GPU_MANIFEST_SHA,
        'ferric-p228-layer0-native-capture-gpu-package-v1')
    V = J.module(reader, old_sources['layer_validation.py'], 'retained_projection_layer_validation')
    old_cv = J.module(reader, old_sources['capture_validation.py'], 'retained_old_capture_validation', {'layer_validation': V})
    audit_pin = J.actual(E / 'p228-independent-gpu-observation-v1/validation.py',
        'cdaf6dc53208bbca8f23b2a3fa3eca9d28fe00ca4cfc74b36d225f3ebe8d28d1')[0]
    topology_pin = J.actual(J.R / 'evidence/resident-output-tp2-v217/run_p217_mi350.py',
        '6016c30f46aa32abf9d175f329a0110e86cbf79d1363f1803dba3c86c1bad50a')[0]
    audit = J.module(reader, audit_pin, 'retained_audit_validation')
    topology = J.module(reader, topology_pin, 'retained_topology_validation')
    old_outer, old_summary = J.native(reader, old_plan['native_outer'], old_plan['platform'], old_cv, audit, topology)
    framework_outer, reference = J.framework(reader)
    manifest, sources = package(J, reader, GPU, GPU_SHA, 'ferric-p228-projection-residual-capture-gpu-package-v1')
    require(sources['layer_validation.py']['sha256'] == old_sources['layer_validation.py']['sha256'], 'unchanged layer contract')
    I = J.module(reader, sources['intake.py'], 'retained_projection_intake', {'layer_validation': V})
    CV = J.module(reader, sources['capture_validation.py'], 'retained_projection_capture', {'layer_validation': V})
    runner = J.module(reader, sources['run.py'], 'retained_projection_capture_run', {'intake': I, 'capture_validation': CV})
    new_outer, summary = candidate(J, reader, new_pin,
        dict(pin=old_plan['native_outer'], outer=old_outer), old_plan['platform'], manifest, sources, I, CV, runner, audit, topology)
    math_pure = J.actual(E / 'projection-residual-comparison-pure-v228-v1/complete.json', MATH_PURE_SHA)[0]
    tested = reader.doc(math_pure); math_sources = reader.doc(tested['sources_before'])
    require(len(math_sources) == 8, 'actual eight-body tested numerical dependency closure')
    J.pure(reader, math_pure, 'ferric-p228-projection-residual-comparison-pure-v1', 20, math_sources)
    for p in math_sources.values(): reader(p)
    selected = lambda suffix: next(p for key, p in math_sources.items() if key == suffix)
    C = J.module(reader, selected('p228-layer0-current-diagnostic-v1/compare.py'), 'retained_projection_compare')
    K = J.module(reader, selected('p228-layer0-current-diagnostic-v1/current.py'), 'retained_projection_current', {'compare': C})
    B = J.module(reader, selected('p228-output-residual-boundary-v1/boundary.py'), 'retained_projection_boundary')
    M = J.module(reader, selected('p228-projection-residual-comparison-v1/comparison.py'), 'retained_projection_math',
        {'current': K, 'compare': C, 'boundary': B})
    diagnostics = C.load_diagnostics(reader, selected('p228-layer0-current-diagnostic-v1/diagnostics.py'))
    original = J.actual(E / 'framework-rearm-v224-v1/reference/reference.json',
        '2edddf40cc6195fdd622e479e8e46f2a659f1b569106f5f4e51afdb83bdc2416')[0]
    result = M.compare_retained(reference, original, J.BASELINE, old_summary, summary, reader, diagnostics,
        candidate_image_pin=new_outer['projection_image'], candidate_worker_pin=new_outer['worker'])
    require(result['comparable_rows'] == 24 and len(result['conditional_residual_comparisons']) == 4
        and result['conditional_residual_words'] == 16384 and result['unchanged_pre_residual_count'] == 14
        and result['numerical_acceptance'] is False and result['acceptance_threshold'] is None, 'complete diagnostic census')
    reader.recheck()
    report = dict(schema='ferric-p228-projection-residual-comparison-observation-v1', completed=True,
        controller=own, old_plan=old_plan_pin, transport_additions=additions_pin, native_outer=new_pin,
        baseline_outer=old_plan['native_outer'], framework_outer=framework_outer,
        comparison_tests=math_pure, comparison_sources=math_sources, native_supervisor_manifest=manifest,
        candidate_owned_leaves_replayed=7, candidate_audits_replayed=6, baseline_owned_leaves_replayed=7,
        baseline_audits_replayed=6, framework_owned_leaf_results_rechecked=21, native_structural_replayed=True,
        comparison=result, consumed=list(reader.consumed.values()), source_postchecks_passed=True,
        all_transitive_admission_inputs_replayed=False, current_gpu_audits_performed=False,
        gpu_execution=False, native_process_launched=False, numerical_acceptance=False,
        full_model_correctness=False, performance_claim=False, production_authority=False)
    raw = (json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    require(len(raw) <= 16 << 20, 'bounded comparison output')
    OUT.mkdir(mode=0o700)
    with (OUT / 'complete.json').open('xb') as stream: stream.write(raw)
    print(json.dumps(dict(complete=J.actual(OUT / 'complete.json')[0], comparable_rows=24,
        conditional_residuals_exact=result['conditional_residuals_exact'],
        earliest_observable_divergence=result['earliest_observable_divergence'])), flush=True)


if __name__ == '__main__':
    main(sys.argv[1:])
