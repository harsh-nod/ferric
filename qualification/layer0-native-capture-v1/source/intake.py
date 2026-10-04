"""One candidate-only layer0 capture with unchanged worker and retained TF4 inputs."""
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys
import types

if sys.flags.optimize or 'PYTHONOPTIMIZE' in os.environ:
    raise RuntimeError('ordinary Python required before loading custody helpers')

import layer_validation as V

HELPER_SHA = '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820'
PRIOR_PACKAGE = ('p228-down2-clock-gpu-v1', '09cb794c667a89f033128a588d6cf558e7306f0e1f7200bd341995113bc64b57')
BASELINE_LABEL = 'prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-v1'
BASELINE = (963187, '00e1af86b1a61894797dc1eeb3202a113d62bd3d11c8069ba6299d63f8875073')
WORKER_CPU = (213924, '41ddcf8a9cb48b970d4f9a4187de6f6de1fd0506ba098d524e82d17335030f1d')
WORKER_BINARY = (5018856, 'd2af909e7fef88af5b20f4ae8f2a5779a7aee93a187169e97a939bec7d3a0fed')
# Root binds only actual completed CPU artifacts before package qualification.
PARENT_CPU = (326552, '78c12f822e95d50a1239f56411c5b8da9411a3fbda7510fbbf38d5644e1dffbb')
PARENT_BINARY = (13480088, 'e7fb0571cfb8114ef86987525a6b882c4fdc84329022db5eabfdb131f3e3e12d')
PARENT_NAME = 'ferric-qwen3-finite-prefix-layer-capture-engineering'
WORKER_NAME = 'ferric-tp-peer-finite-engineering-worker-v1'
HIDDEN_SHA = 'faa56202578a3d3497bbe779137736439957e473775bd6f677dbce466a9d6979'
INPUT_SCHEMA = 'ferric-p228-layer0-native-capture-inputs-v1'
REVIEW_SCHEMA = 'ferric-p228-layer0-native-capture-engineering-review-v1'
PACKAGE_SCHEMA = 'ferric-p228-layer0-native-capture-gpu-package-v1'
PURE_SCHEMA = 'ferric-p228-layer0-native-capture-gpu-pure-v1'
PLAN_FIELDS = ('schema output_label baseline parent_cpu worker_cpu parent worker request capture_review '
               'parent_runtime_review worker_runtime_review supervisor_tests supervisor_test_sources')
REVIEW_BINDINGS = ('baseline parent_cpu worker_cpu parent worker request '
                   'parent_runtime_review worker_runtime_review').split()
REVIEW_FALSE = ('production_authority', 'full_model_acceptance', 'independent_numerical_acceptance',
    'numerical_acceptance', 'arithmetic_prerequisites_verified', 'runtime_premises_discharged', 'performance_claim',
    'timestamp_calibration', 'clock_domain_validated', 'cross_device_clock_alignment', 'overlap_claim',
    'paired_comparison_performed', 'full_forward')
TOPICS = ('source_lineage', 'formal', 'isa', 'coherence', 'lifecycle', 'selected_device')
PACKAGE_FILES = {'INTAKE.md', 'README.md', 'capture_validation.py', 'intake.py', 'layer_validation.py',
    'run.py', 'run_row_facts_v2.py', 'test_capture_validation.py', 'test_intake.py', 'test_run.py'}
PURE_TESTS = 40
TEST_RUNNER_SHA = 'd4a5ed455602f34754ad3d6ff4541c902cc205f1157fecd57c8595786a9925cc'


def bootstrap():
    path = Path(__file__).resolve().with_name('run_row_facts_v2.py')
    with path.open('rb') as stream:
        before = os.fstat(stream.fileno()); raw = stream.read((1 << 20) + 1); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    V.require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
        and len(raw) == before.st_size <= 1 << 20 and stamp(before) == stamp(after) == stamp(path.lstat())
        and hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'frozen custody helper')
    module = types.ModuleType('device_timing_custody'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


D = bootstrap()
E, R = D.E, D.E.parents[1]


def read(pins, record, maximum=16 << 20, retain=True):
    V.keys(record, 'path bytes sha256'); V.uint(record['bytes'], maximum)
    V.require(type(record['sha256']) is str and re.fullmatch('[0-9a-f]{64}', record['sha256']), 'FilePin SHA')
    actual, raw = pins.read(Path(record['path']), record['sha256'], retain, maximum)
    V.require(actual == record, 'actual pin extent')
    return raw


def doc(pins, record, maximum=16 << 20):
    return D.parse(read(pins, record, maximum))


def save(path, value):
    raw = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    V.require(len(raw) <= 8 << 20, 'bounded supervisor JSON')
    with path.open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    return D.read_file(path, maximum=8 << 20)[0]


def owned_success(value):
    V.require(type(value.get('exit_code')) is int and value['exit_code'] == 0 and value.get('reason') is None
        and value.get('cleanup_signalled') is False and value.get('owned_groups_absent') is True
        and value.get('owned_processes_reaped') is True, 'successful independently reaped owned execution')


def input_shape(plan):
    V.keys(plan, PLAN_FIELDS)
    V.require(plan['schema'] == INPUT_SCHEMA and type(plan['output_label']) is str
        and re.fullmatch(r'prefix-layer0-native-capture-gpu-v228-v[1-9][0-9]{0,8}', plan['output_label']),
        'closed candidate-only capture plan')
    for name in PLAN_FIELDS.split():
        if name not in ('schema', 'output_label'):
            V.require(type(plan[name]) is dict, 'actual input FilePin')


def package_record(pins):
    V.require(type(PACKAGE_FILES) is set and PACKAGE_FILES and type(PURE_TESTS) is int and PURE_TESTS > 0
        and type(TEST_RUNNER_SHA) is str and re.fullmatch('[0-9a-f]{64}', TEST_RUNNER_SHA),
        'root must freeze package members and actual pure runner before context')
    base = Path(__file__).resolve().parent
    manifest, record = pins.json(base / 'manifest.json')
    V.require(manifest['schema'] == PACKAGE_SCHEMA and type(manifest['pure_tests']) is int
        and manifest['pure_tests'] == PURE_TESTS and type(manifest['files']) is list
        and len(manifest['files']) == len(PACKAGE_FILES)
        and {row['path'] for row in manifest['files']} == PACKAGE_FILES, 'closed candidate capture supervisor package')
    for row in manifest['files']:
        V.keys(row, 'path bytes sha256')
        V.require(pins.pin(base / row['path'], row['sha256'])['bytes'] == row['bytes'], 'frozen supervisor member')
    return manifest, record


def supervisor_tests(pins, plan, manifest, manifest_pin):
    value = doc(pins, plan['supervisor_tests'])
    V.require(value['schema'] == PURE_SCHEMA and value['passed'] is True
        and type(value['tests']) is int and value['tests'] == manifest['pure_tests']
        and value['manifest_sha256'] == manifest_pin['sha256'] and value['controller_sha256'] == TEST_RUNNER_SHA
        and value['source_sha256'] == plan['supervisor_test_sources']['sha256']
        and value['sources_before'] == plan['supervisor_test_sources']
        and value['source_postchecks_passed'] is True and value['synthetic_policy_tests_only'] is True,
        'actual new bounded pure-test receipt')
    for key in ('errors', 'failures', 'skipped'):
        V.require(type(value[key]) is int and value[key] == 0, 'complete passing pure census')
    for key in ('native_execution', 'gpu_execution', 'numerical_acceptance', 'full_model_acceptance',
                'production_authority', 'performance_claim'):
        V.require(value[key] is False, 'pure-test scope only')
    directory = Path(plan['supervisor_tests']['path']).parent
    V.require(directory.parent == E and Path(plan['supervisor_tests']['path']).name == 'complete.json'
        and re.fullmatch(r'layer0-native-capture-gpu-pure-v228-v[1-9][0-9]{0,8}', directory.name), 'new pure namespace')
    for key, filename in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'),
                          ('transcript', 'tests.log')):
        V.require(value[key]['path'] == str(directory / filename), 'actual retained pure output path')
        read(pins, value[key])
    before, after = doc(pins, value['sources_before']), doc(pins, value['sources_after'])
    expected = {row['path']: dict(row, path=str(Path(__file__).resolve().parent / row['path']))
                for row in manifest['files']}
    V.require(before == after == expected, 'exact tested package before and after')
    pins.pin(E / 'run_layer0_native_capture_gpu_pure_p228_v1.py', TEST_RUNNER_SHA)


def content(value):
    return value['bytes'], value['sha256']


def previous_package(pins):
    base = D.package(pins, *PRIOR_PACKAGE)
    manifest, record = pins.json(base / 'manifest.json', PRIOR_PACKAGE[1])
    hashes = {row['path']: row['sha256'] for row in manifest['files']}
    names = ('layer_validation', 'host_validation', 'raw_validation', 'down2', 'device_validation')
    absent = object()
    previous = {name: sys.modules.get(name, absent) for name in names}
    loaded = {}
    try:
        for name in names:
            loaded[name] = D.load_module(pins, base / (name + '.py'), hashes[name + '.py'],
                                        'layer0_capture_previous_' + name)
            sys.modules[name] = loaded[name]
        intake = D.load_module(pins, base / 'intake.py', hashes['intake.py'], 'layer0_capture_previous_intake')
    finally:
        for name, value in previous.items():
            if value is absent:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = value
    return intake, loaded['device_validation'], manifest, record


def replay_leaves(pins, value, platform, O, environment):
    directory = E / BASELINE_LABEL
    expected = {'parent', *[side + '-' + str(i) for side in ('before', 'after') for i in range(3)]}
    V.require(set(value['leaves']) == expected, 'actual one native and six audit leaves')
    for name, leaf in value['leaves'].items():
        result = doc(pins, leaf['result'])
        owned_success(result)
        files = leaf['retained_files']
        V.require(set(files) == {'command.json', 'started.json', 'stdout', 'stderr', 'result.json'}
            and files['result.json'] == leaf['result'], 'complete original owned leaf')
        for filename, pin in files.items():
            V.require(pin['path'] == str(directory / name / filename), 'case-contained original leaf')
            read(pins, pin, 8 << 20, False)
        for key in ('command', 'started', 'stdout', 'stderr'):
            filename = key + '.json' if key in ('command', 'started') else key
            V.require(result[key] == files[filename], 'actual command and stream identity')
        V.require(doc(pins, result['command'])['env'] == environment, 'unchanged reviewed baseline environment')
    for side in ('before', 'after'):
        audits = value[side + '_audits']
        V.require(type(audits) is list and len(audits) == 3, 'three actual audits per side')
        for index, audit in enumerate(audits):
            name = side + '-' + str(index)
            V.require(audit['process_result'] == value['leaves'][name]['result']
                and audit['topology']['path'] == str(directory / (name + '-topology.json')),
                'actual audit/owned-leaf joins')
            O.check_platform(platform, doc(pins, audit['topology']))
            result = doc(pins, audit['process_result'])
            V.require(result['gpu_execution_requested'] is False and result['stderr']['bytes'] == 0,
                      'CPU-only quiet process audit')
            O.process_audit(read(pins, result['stdout']), platform)


def baseline_bindings(plan, value):
    for key, receipt_key in (('baseline', 'legacy_baseline'), ('clock_baseline', 'baseline'),
                             ('request', 'request'), ('parent', 'parent'), ('worker', 'worker'),
                             ('parent_cpu', 'parent_cpu_complete'), ('worker_cpu', 'worker_cpu_complete'),
                             ('parent_runtime_review', 'parent_runtime_review'), ('worker_runtime_review', 'worker_runtime_review')):
        V.require(plan[key] == value[receipt_key], 'actual baseline plan/receipt binding: ' + key)


def baseline(pins, record):
    V.require(content(record) == BASELINE
        and record['path'] == str(E / BASELINE_LABEL / 'complete.json'), 'actual Down2 TF4 baseline')
    old, DV, package, package_pin = previous_package(pins)
    value = doc(pins, record)
    V.require(value['schema'] == 'ferric-p228-down2-clock-gpu-v1' and value['passed'] is True
        and value['failures'] == [] and value['native_attempts'] == 1 and value['retries'] == 0
        and value['supervisor_manifest'] == package_pin
        and value['policy'] == 'shared-full-currentness'
        and all(value[key] is False for key in DV.FALSE), 'actual conditional native TF4 observation only')
    V.require(value['controller'] == pins.pin(Path(old.__file__).with_name('run.py')),
              'actual frozen baseline controller')
    plan = doc(pins, value['plan'], 1 << 20)
    old.input_shape(plan)
    old.supervisor_tests(pins, plan, package, package_pin)
    baseline_bindings(plan, value)
    modules, prior = old.baseline(pins, plan['baseline'])
    B, _, C, H, _ = modules
    P, O, standalone, packages, cli_tests = B.standalone(pins, plan)
    for key in ('image_deployment', 'standalone_prepared', 'standalone_cases', 'numericals'):
        V.require(plan[key] == prior['plan'][key], 'unchanged six original prerequisite cases')
    parent_cpu, parent_artifact, parent_sources = old.cpu_evidence(pins, plan['parent_cpu'], 'parent')
    worker_cpu, worker_artifact, worker_sources = old.cpu_evidence(pins, plan['worker_cpu'], 'worker')
    old.shared_source(parent_sources, worker_sources)
    runtime = dict(parent=old.deployed_binary(pins, plan['parent'], parent_artifact['binary']),
        worker=old.deployed_binary(pins, plan['worker'], worker_artifact['binary']),
        image=standalone['verified']['image'])
    V.require(runtime == value['selected_runtime'] and runtime['image'] == prior['runtime']['image'],
              'actual baseline CPU generations and V7 image')
    api = types.SimpleNamespace(E=E, doc=doc, read=read, owned_success=owned_success)
    clock = old.D2.clock_baseline(api, DV, pins, plan['clock_baseline'], prior, C, H, O,
                                 standalone['platform'], dict(P.ENV))
    V.require(runtime == clock['runtime'], 'unchanged actual baseline clock runtime')
    for key in ('parent_cpu', 'worker_cpu', 'parent_runtime_review', 'worker_runtime_review'):
        V.require(plan[key] == clock['plan'][key], 'original clock runtime proof binding')
    provenance = old.D2.lowering(api, pins, plan['down2_lowering'], plan['down2_image'])
    legacy = old.legacy(pins, B)
    L = legacy.prior_helpers(pins)
    request = old.device_request(doc(pins, plan['request'], 64 << 10))
    legacy.request_check(request, E / BASELINE_LABEL, runtime, standalone, pins, L)
    old.D2.workload(request, clock['request'], plan['down2_image'], L.rust_pin, B.HC.H.same)
    for role in ('parent', 'worker'):
        P.runtime_review(doc(pins, plan[role + '_runtime_review']), runtime[role], standalone['platform'], pins)
    old.engineering_review(doc(pins, plan['decode_review'], 192 << 10), plan, runtime, clock, standalone)
    old.D2.review(doc(pins, plan['down2_review'], 192 << 10), plan, provenance, clock, runtime, old.REVIEW_FALSE)
    replay_leaves(pins, value, standalone['platform'], O, dict(P.ENV))
    native = doc(pins, value['leaves']['parent']['result'])
    c = dict(pins=pins, plan=plan, runtime=runtime, baseline=clock, C=C, H=H, read=read,
             rust_pin=L.rust_pin, environment=dict(P.ENV))
    checked = DV.observe(c, value['retained_native'], value['device_sidecar'],
                         value['leaves']['parent']['result'], native)
    V.require(checked == value['checked'] == doc(pins, value['observation']),
              'all four payloads and 152 tensors replay against genuine unchanged receipt')
    pin = value['retained_native']['observation-0.bin']
    raw = read(pins, pin, 606976)
    V.require(len(raw) == 606976 and hashlib.sha256(raw[:8192]).hexdigest() == HIDDEN_SHA,
              'exact current native token9112 position0 layer0 hidden slice')
    pins.recheck()
    P.guard(standalone)
    return dict(receipt_pin=record, receipt=value, plan=plan, request=request, runtime=runtime,
        standalone=standalone, P=P, O=O, L=L, old=old, parent_cpu=parent_cpu, worker_cpu=worker_cpu,
        worker_sources=worker_sources, down2_provenance=provenance, expected_hidden=raw[:8192],
        baseline_hidden=dict(source=pin, offset=0, bytes=8192, sha256=HIDDEN_SHA))


def qualified_cpu(value, role):
    V.require(role in ('parent', 'worker'), 'closed CPU role')
    is_parent = role == 'parent'
    schema = 'ferric-p228-layer0-native-capture-cpu-result-v1' if is_parent else 'ferric-p228-gfx950-clock-recorder-cpu-result-v1'
    V.require(value['schema'] == schema and value['passed'] is True and value['error'] is None
        and value['postcheck_errors'] == [] and value['source_unchanged'] is True
        and value['empty_initial_target'] is True
        and value['tests_passed'] == (289 if is_parent else 669)
        and type(value['tests_passed']) is int and value['tests_ignored'] == (0 if is_parent else 4)
        and type(value['tests_ignored']) is int, 'actual successful qualified CPU generation')
    V.require(value['parent_rebuilt'] is is_parent, 'separate parent and worker qualification')
    if is_parent:
        V.require(value['worker_rebuilt'] is False and value['compiler_hsaco_reproduced'] is False
            and value['sibling_runtime_rebuilt'] is False,
                  'parent-only CPU qualification')
    for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim',
                'timestamp_calibration', 'production_authority'):
        V.require(value[key] is False, 'CPU receipt conveys no GPU or numerical authority')
    phases = value['phases']
    V.require(type(phases) is dict and len(phases) == (46 if is_parent else 33), 'complete CPU phase census')
    for phase in phases.values():
        V.require(type(phase['exit_code']) is int and phase['exit_code'] == 0
            and phase['reason'] is None and phase['group_absent'] is True, 'natural successful CPU phases')
    name = PARENT_NAME if is_parent else WORKER_NAME
    selected = value['binaries'][name]
    V.keys(selected, 'artifact binary')
    artifact, binary = selected['artifact'], selected['binary']
    V.keys(binary, 'path bytes sha256')
    V.require(artifact['reason'] == 'compiler-artifact' and artifact['target']['name'] == name
        and artifact['target']['kind'] == ['bin'] and artifact['target']['crate_types'] == ['bin']
        and artifact['executable'] == binary['path'] and artifact['filenames'] == [binary['path']]
        and artifact['profile']['test'] is False and artifact['profile']['opt_level'] == '2'
        and artifact['profile']['debug_assertions'] is True and artifact['profile']['overflow_checks'] is True,
        'exact ordinary optimized CPU Cargo executable with assertions and overflow checks')
    V.require(('tp-batch-engineering' in artifact['features']) if is_parent else artifact['features'] == [],
              'selected executable feature gate')
    V.require(content(binary) == (PARENT_BINARY if is_parent else WORKER_BINARY),
              'actual capture parent or unchanged CPU669 worker digest and extent')
    return selected


def cpu_evidence(pins, record, role):
    expected = PARENT_CPU if role == 'parent' else WORKER_CPU
    V.require(type(expected) is tuple and len(expected) == 2 and type(expected[0]) is int
        and expected[0] > 0 and type(expected[1]) is str and re.fullmatch('[0-9a-f]{64}', expected[1]),
        'root must bind actual successful CPU receipt before admission')
    V.require(content(record) == expected, 'exact root-verified CPU completion bytes')
    value = doc(pins, record)
    selected = qualified_cpu(value, role)
    before = doc(pins, value['raw']['sources-before.json'])
    after = doc(pins, value['raw']['sources-after.json'])
    V.require(type(before) is dict and before and before == after, 'retained qualified source generation unchanged')
    phase = 'parent-builds' if role == 'parent' else 'worker-build'
    raw = read(pins, value['raw'][phase + '-stdout'], 32 << 20)
    rows = [D.parse(line) for line in raw.splitlines() if line.strip()]
    V.require(sum(row == selected['artifact'] for row in rows) == 1,
              'selected artifact occurs exactly once in actual retained Cargo output')
    V.require(hashlib.sha256(raw).hexdigest() == value['phases'][phase]['stdout_sha256'],
              'selected build phase and actual Cargo output join')
    return value, selected, before


def shared_source(parent, worker):
    prefix = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
    a = {path: value for path, value in parent.items() if path.startswith(prefix)}
    b = {path: value for path, value in worker.items() if path.startswith(prefix)}
    V.require(set(a) == set(b), 'same complete qualified worker subtree roster')
    # CPU669 publication changed only this noncompiled README after its build.
    # Keep every Rust, test, Cargo, config and any other member in the join.
    a.pop(prefix + 'README.md', None); b.pop(prefix + 'README.md', None)
    required = {prefix + 'src/' + name for name in (
        'prefix_decode_device_observation_v1.rs', 'prefix_decode_device_observation_v1_tests.rs',
        'prefix_decode_device_clock_observation_v2.rs', 'prefix_decode_device_clock_observation_v2_tests.rs',
        'finite_prefix_decode_wire_v1.rs', 'finite_setup_wire_v1.rs', 'finite_forward_wire_v1.rs')}
    V.require(a == b and required <= set(a), 'parent aliases read the exact qualified worker source generation')


def deployed_binary(pins, record, original):
    V.keys(record, 'path bytes sha256')
    V.require(content(record) == content(original), 'transported executable differs from qualified Cargo artifact')
    path = Path(record['path'])
    V.require(path.is_relative_to(E) and path.name == Path(original['path']).name,
              'explicit transported executable under session evidence')
    raw = read(pins, record, 128 << 20)
    V.require(len(raw) >= 20 and raw[:6] == b'\x7fELF\x02\x01' and raw[18:20] == b'\x3e\x00',
              'actual little-endian x86_64 ELF artifact')
    return record


def request_check(pins, value, prior, runtime, out):
    V.keys(value, 'schema source worker images expected_bundle_id expected_model_id device_ids session prompt '
        'prefix_tiles_image mlp_tiles_image evidence_directory dispatch_timeout_ms child_deadline_ms')
    V.require(value['schema'] == 'FerricFinitePrefixLayerCaptureRequestV1',
              'distinct candidate-only layer0 request')
    previous, L = prior['request'], prior['L']
    for key in ('source', 'images', 'expected_bundle_id', 'expected_model_id', 'device_ids', 'prompt',
                'dispatch_timeout_ms', 'child_deadline_ms'):
        V.require(value[key] == previous[key], 'unchanged genuine TF4 input field: ' + key)
    V.require(previous['mode'] == 'teacher_forced'
        and value['prefix_tiles_image'] == previous['prefix_image']
        and value['mlp_tiles_image'] == previous['tiles_image'],
        'exact V7 prefix and Down2 images, not bootstrap images')
    V.require(V.digest(value['session']) != bytes(32) and value['session'] != previous['session']
        and value['evidence_directory'] == str(out / 'native'), 'fresh private capture output and session')
    V.require(L.rust_pin(value['worker']) == runtime['worker'] == prior['runtime']['worker'],
              'unchanged actual qualified worker')
    V.require(L.rust_pin(value['prefix_tiles_image']) == runtime['image']
        and L.rust_pin(value['mlp_tiles_image']) == prior['plan']['down2_image'],
        'exact selected image paths and bytes')
    V.require(V.uint(value['dispatch_timeout_ms']) == 10000 and V.uint(value['child_deadline_ms']) == 3600000,
              'unchanged bounded child and dispatch deadlines')
    for pin in [*value['images'].values(), value['prefix_tiles_image'], value['mlp_tiles_image']]:
        read(pins, L.rust_pin(pin), 32 << 20, False)
    for pin in value['prompt'].values():
        read(pins, L.rust_pin(pin), 32 << 10, False)


def engineering_review(value, plan, runtime, prior):
    V.keys(value, 'schema reviewed authority gpu_attempts output_label image down2_image '
        'baseline_hidden image_provenance down2_provenance review_topics notes '
        + ' '.join(REVIEW_BINDINGS) + ' ' + ' '.join(REVIEW_FALSE))
    V.require(value['schema'] == REVIEW_SCHEMA and value['reviewed'] is True and value['authority'] == 'none'
        and type(value['gpu_attempts']) is int and value['gpu_attempts'] == 1
        and value['output_label'] == plan['output_label']
        and all(value[key] is False for key in REVIEW_FALSE), 'one engineering capture, no numerical authority')
    for key in REVIEW_BINDINGS:
        V.require(value[key] == plan[key], 'root review binds actual input: ' + key)
    V.require(value['parent'] == runtime['parent'] and value['worker'] == runtime['worker']
        and value['image'] == runtime['image'] and value['down2_image'] == prior['plan']['down2_image']
        and value['baseline_hidden'] == prior['baseline_hidden']
        and value['image_provenance'] == prior['standalone']['provenance']
        and value['down2_provenance'] == prior['down2_provenance'], 'current capture runtime and arithmetic lineage')
    V.keys(value['review_topics'], ' '.join(TOPICS))
    for text in [value['notes'], *value['review_topics'].values()]:
        V.require(type(text) is str and len(text.strip()) >= 32 and len(text.encode()) <= 16384,
                  'substantive root-authored source/formal/ISA/coherence/lifecycle/device notes')


def context(plan_pin):
    pins = D.Pins()
    plan = doc(pins, plan_pin, 1 << 20)
    input_shape(plan)
    out = E / plan['output_label']
    V.require(not os.path.lexists(out), 'exclusive one-attempt capture directory')
    manifest, manifest_pin = package_record(pins)
    supervisor_tests(pins, plan, manifest, manifest_pin)
    prior = baseline(pins, plan['baseline'])
    V.require(plan['worker_cpu'] == prior['plan']['worker_cpu'], 'unchanged CPU669 receipt')
    parent_cpu, parent_artifact, parent_sources = cpu_evidence(pins, plan['parent_cpu'], 'parent')
    worker_cpu, worker_artifact, worker_sources = cpu_evidence(pins, plan['worker_cpu'], 'worker')
    shared_source(parent_sources, worker_sources)
    V.require(parent_cpu['prior_cpu_complete'] == prior['plan']['parent_cpu']
        and parent_cpu['source_commits']['fe2o3'] == worker_cpu['source_commits']['fe2o3'],
        'new parent descends from actual baseline275 and retains runtime archive generation')
    runtime = dict(parent=deployed_binary(pins, plan['parent'], parent_artifact['binary']),
        worker=deployed_binary(pins, plan['worker'], worker_artifact['binary']), image=prior['runtime']['image'])
    request = doc(pins, plan['request'], 64 << 10)
    request_check(pins, request, prior, runtime, out)
    standalone, P = prior['standalone'], prior['P']
    reviews = {}
    for role in ('parent', 'worker'):
        reviews[role] = doc(pins, plan[role + '_runtime_review'])
        P.runtime_review(reviews[role], runtime[role], standalone['platform'], pins)
    engineering_review(doc(pins, plan['capture_review'], 192 << 10), plan, runtime, prior)
    c = dict(pins=pins, plan=plan, plan_pin=plan_pin, request=request, out=out, runtime=runtime,
        standalone=standalone, P=P, O=prior['O'], platform=standalone['platform'], topology=standalone['topology'],
        owned=standalone['owned'], environment=dict(P.ENV), runtime_reviews=reviews,
        expected_hidden=prior['expected_hidden'], baseline_hidden=prior['baseline_hidden'],
        supervisor_manifest=manifest_pin, baseline=prior, parent_cpu=parent_cpu, worker_cpu=worker_cpu)
    guard(c)
    return c


def guard(c):
    c['pins'].recheck(); c['P'].guard(c['standalone'])
    for review in c['runtime_reviews'].values():
        for row in review['libraries']:
            V.require(Path(row['path']).resolve(strict=True) == Path(row['resolved']['path']),
                      'runtime library alias unchanged')
