"""Explicit device-tick intake; qualified CPU artifacts, unchanged V7 prerequisites."""
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
BASELINE_COMPARISON_SHA = '413e8bb4ea5637cb3ca67003433b62bf7cb90832c4f88101d2c9ea9fa7bcc512'
BASELINE_DIAGNOSTIC_SHA = '259f6f233be23da36eac213bfb5e0c905afa43461461fbb0305efcc45c08d930'
BASELINE_LABEL = 'prefix-state-bank-batch-tf4-shared-full-currentness-gpu-v228-v1'
BASELINE_SHA = '9917a07aba38ecdf4ca1c9289774c9b9c040bdf5bf850157126c87ca1ab7595c'
WORKER_CPU = (183804, '407990eeac9332f4807ff71fcc7c884a0f14a92dffd44f97fd19ea7fa0b7bbf7')
WORKER_BINARY = (4872432, 'eda6a2450636d5fdc8ceca0f39b84769cdbcf47a74b73bb66310eb5cc8233d03')
# Root fills this only after the bounded parent qualification actually passes.
PARENT_CPU = (298774, '3b51d61268bee492dd6ded7623742153513c46cd3d246ebc60a6bc9a2555077b')
PARENT_NAME = 'ferric-qwen3-finite-prefix-decode-device-engineering'
WORKER_NAME = 'ferric-tp-peer-finite-engineering-worker-v1'
INPUT_SCHEMA = 'ferric-p228-device-timing-inputs-v1'
REVIEW_SCHEMA = 'ferric-p228-device-timing-engineering-review-v1'
PACKAGE_SCHEMA = 'ferric-p228-device-timing-package-v1'
PURE_SCHEMA = 'ferric-p228-device-timing-pure-v1'
PLAN_FIELDS = ('schema output_label policy baseline parent_cpu worker_cpu parent worker image_deployment '
    'standalone_prepared standalone_cases numericals request decode_review parent_runtime_review '
    'worker_runtime_review supervisor_tests supervisor_test_sources')
REVIEW_BINDINGS = ('baseline parent_cpu worker_cpu parent worker image_deployment standalone_prepared '
    'standalone_cases numericals request parent_runtime_review worker_runtime_review').split()
REVIEW_FALSE = ('production_authority', 'full_model_acceptance', 'independent_numerical_acceptance',
    'arithmetic_prerequisites_verified', 'runtime_premises_discharged', 'performance_claim',
    'timestamp_calibration', 'cross_device_clock_alignment', 'overlap_claim')
TOPICS = ('source_lineage', 'formal', 'isa', 'coherence', 'lifecycle', 'selected_device')
BODY = {'child-stderr.bin'} | {f'{kind}-{p}.{suffix}' for p in range(4)
    for kind, suffix in (('request', 'json'), ('control', 'bin'), ('observation', 'bin'))}
# Root binds the finished package and actual bounded pure runner before use.
PACKAGE_FILES = {'INTAKE.md', 'README.md', 'baseline_comparison.py', 'baseline_diagnostic.py',
    'device_validation.py', 'host_validation.py', 'intake.py', 'layer_validation.py',
    'run.py', 'run_row_facts_v2.py', 'test_device_validation.py', 'test_intake.py', 'test_run.py'}
PURE_TESTS = 44
TEST_RUNNER_SHA = '667f62cd755605d0b5758b568f5e38f8575f5ea9863d9573484db7247939794f'


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
    V.require(plan['schema'] == INPUT_SCHEMA and plan['policy'] == 'shared-full-currentness'
        and type(plan['output_label']) is str
        and re.fullmatch(r'prefix-device-timing-tf4-shared-full-currentness-gpu-v228-v[1-9][0-9]{0,8}',
                         plan['output_label']), 'closed explicit TF4 raw-device input')
    for name in ('standalone_cases', 'numericals'):
        V.require(type(plan[name]) is list and len(plan[name]) == 6
            and len({row['path'] for row in plan[name]}) == 6, 'six distinct ordered actual prerequisites')
    for name in PLAN_FIELDS.split():
        if name not in ('schema', 'output_label', 'policy', 'standalone_cases', 'numericals'):
            V.require(type(plan[name]) is dict, 'actual input FilePin')


def device_request(value):
    V.keys(value, 'schema decode')
    V.require(value['schema'] == 'FerricFinitePrefixDecodeDeviceRequestV1'
        and type(value['decode']) is dict and value['decode'].get('mode') == 'teacher_forced',
        'new explicit device request and supported baseline mode')
    return value['decode']


def package_record(pins):
    V.require(type(PACKAGE_FILES) is set and PACKAGE_FILES and type(PURE_TESTS) is int and PURE_TESTS > 0
        and type(TEST_RUNNER_SHA) is str and re.fullmatch('[0-9a-f]{64}', TEST_RUNNER_SHA),
        'root must freeze package members and actual pure runner before context')
    base = Path(__file__).resolve().parent
    manifest, record = pins.json(base / 'manifest.json')
    V.require(manifest['schema'] == PACKAGE_SCHEMA and type(manifest['pure_tests']) is int
        and manifest['pure_tests'] == PURE_TESTS and type(manifest['files']) is list
        and len(manifest['files']) == len(PACKAGE_FILES)
        and {row['path'] for row in manifest['files']} == PACKAGE_FILES, 'closed device supervisor package')
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
        and re.fullmatch(r'device-timing-gpu-pure-v228-v[1-9][0-9]{0,8}', directory.name), 'new pure namespace')
    for key, filename in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'),
                          ('transcript', 'tests.log')):
        V.require(value[key]['path'] == str(directory / filename), 'actual retained pure output path')
        read(pins, value[key])
    before, after = doc(pins, value['sources_before']), doc(pins, value['sources_after'])
    expected = {row['path']: dict(row, path=str(Path(__file__).resolve().parent / row['path']))
                for row in manifest['files']}
    V.require(before == after == expected, 'exact tested package before and after')
    pins.pin(E / 'run_device_timing_gpu_pure_p228_v1.py', TEST_RUNNER_SHA)


def baseline(pins, record):
    V.require(record['path'] == str(E / BASELINE_LABEL / 'complete.json')
        and record['sha256'] == BASELINE_SHA, 'actual CPU553 TF4 baseline only')
    base = Path(__file__).resolve().parent
    reader = D.load_module(pins, base / 'baseline_diagnostic.py', BASELINE_DIAGNOSTIC_SHA,
                           'device_timing_baseline_diagnostic')
    comparison = D.load_module(pins, base / 'baseline_comparison.py', BASELINE_COMPARISON_SHA,
                               'device_timing_baseline_comparison')
    modules = comparison.loaded(D, pins, reader, 'new')
    result = comparison.replay(reader, D, pins, 'new', record, doc(pins, record), modules)
    return modules, result


def content(value):
    return value['bytes'], value['sha256']


def qualified_cpu(value, role):
    V.require(role in ('parent', 'worker'), 'closed CPU role')
    is_parent = role == 'parent'
    schema = 'ferric-p228-device-parent-cpu-result-v1' if is_parent else 'ferric-p228-device-routing-cpu-result-v1'
    V.require(value['schema'] == schema and value['passed'] is True and value['error'] is None
        and value['postcheck_errors'] == [] and value['source_unchanged'] is True
        and value['empty_initial_target'] is True
        and value['tests_passed'] == (257 if is_parent else 609)
        and type(value['tests_passed']) is int and value['tests_ignored'] == (0 if is_parent else 4)
        and type(value['tests_ignored']) is int, 'actual successful qualified CPU generation')
    V.require(value['parent_rebuilt'] is is_parent, 'separate parent and worker qualification')
    if is_parent:
        V.require(value['worker_rebuilt'] is False and value['compiler_hsaco_reproduced'] is False,
                  'parent-only CPU qualification')
    for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim',
                'timestamp_calibration', 'production_authority'):
        V.require(value[key] is False, 'CPU receipt conveys no GPU or numerical authority')
    phases = value['phases']
    V.require(type(phases) is dict and len(phases) == (41 if is_parent else 25), 'complete CPU phase census')
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
    if not is_parent:
        V.require(content(binary) == WORKER_BINARY, 'actual CPU609 worker digest and extent')
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
    # CPU609 publication changed only this noncompiled README after its build.
    # Keep every Rust, test, Cargo, config and any other member in the join.
    a.pop(prefix + 'README.md', None); b.pop(prefix + 'README.md', None)
    required = {prefix + 'src/' + name for name in (
        'prefix_decode_device_observation_v1.rs', 'prefix_decode_device_observation_v1_tests.rs',
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


def same_workload(request, prior, H):
    previous = prior['request']
    ignored = {'worker', 'session', 'evidence_directory'}
    V.require(set(request) == set(previous) and ignored <= set(request)
        and H.same({k: v for k, v in request.items() if k not in ignored},
                   {k: v for k, v in previous.items() if k not in ignored}),
        'same complete authenticated TF4 workload except worker/session/output')
    V.require(request['session'] != previous['session'], 'fresh device-observation session')


def engineering_review(value, plan, runtime, prior, s):
    V.keys(value, 'schema reviewed authority policy image historical_runtime image_provenance '
        'review_topics notes gpu_attempts ' + ' '.join(REVIEW_BINDINGS) + ' ' + ' '.join(REVIEW_FALSE))
    V.require(value['schema'] == REVIEW_SCHEMA and value['reviewed'] is True and value['authority'] == 'none'
        and value['policy'] == plan['policy'] and type(value['gpu_attempts']) is int and value['gpu_attempts'] == 1
        and all(value[key] is False for key in REVIEW_FALSE), 'explicit single-attempt engineering review only')
    for key in REVIEW_BINDINGS:
        V.require(value[key] == plan[key], 'review binds exact prerequisite: ' + key)
    V.require(value['parent'] == runtime['parent'] and value['worker'] == runtime['worker']
        and value['image'] == runtime['image'] and value['historical_runtime'] == prior['runtime']
        and value['image_provenance'] == s['provenance'], 'selected CPU generations and unchanged V7 provenance')
    V.keys(value['review_topics'], ' '.join(TOPICS))
    for text in [value['notes'], *value['review_topics'].values()]:
        V.require(type(text) is str and 32 <= len(text.strip()) and len(text.encode()) <= 16384,
                  'substantive root-authored engineering review')


def context(plan_pin):
    pins = D.Pins(); plan = doc(pins, plan_pin, 1 << 20); input_shape(plan)
    out = E / plan['output_label']; V.require(not os.path.lexists(out), 'exclusive one-attempt output')
    manifest, manifest_pin = package_record(pins)
    supervisor_tests(pins, plan, manifest, manifest_pin)
    modules, prior = baseline(pins, plan['baseline'])
    B, _, C, H, _ = modules
    P, O, s, packages, cli_tests = B.standalone(pins, plan)
    for key in ('image_deployment', 'standalone_prepared', 'standalone_cases', 'numericals'):
        V.require(plan[key] == prior['plan'][key], 'unchanged actual baseline prerequisite: ' + key)
    parent_cpu, parent_artifact, parent_sources = cpu_evidence(pins, plan['parent_cpu'], 'parent')
    worker_cpu, worker_artifact, worker_sources = cpu_evidence(pins, plan['worker_cpu'], 'worker')
    shared_source(parent_sources, worker_sources)
    V.require(parent_cpu['source_commits']['fe2o3'] == worker_cpu['source_commits']['fe2o3'],
              'unchanged qualified runtime source commit')
    runtime = dict(parent=deployed_binary(pins, plan['parent'], parent_artifact['binary']),
        worker=deployed_binary(pins, plan['worker'], worker_artifact['binary']), image=s['verified']['image'])
    V.require(runtime['image'] == s['prepared']['object'] == prior['runtime']['image'], 'unchanged actual V7 image')
    request = device_request(doc(pins, plan['request'], 64 << 10))
    old = B.legacy(pins); L = old.prior_helpers(pins)
    old.request_check(request, out, runtime, s, pins, L)
    old.baseline_images(request, prior['observed'], L)
    same_workload(request, prior, B.HC.H)
    reviews = {}
    for role in ('parent', 'worker'):
        reviews[role] = doc(pins, plan[role + '_runtime_review'])
        P.runtime_review(reviews[role], runtime[role], s['platform'], pins)
    engineering_review(doc(pins, plan['decode_review'], 192 << 10), plan, runtime, prior, s)
    c = dict(pins=pins, plan=plan, plan_pin=plan_pin, request=request, policy=plan['policy'], out=out,
        runtime=runtime, selected_runtime=runtime, historical_runtime=prior['runtime'], baseline=prior,
        standalone=s, P=P, O=O, platform=s['platform'], topology=s['topology'], owned=s['owned'],
        runtime_reviews=reviews, environment=dict(P.ENV), parent_cpu=parent_cpu, worker_cpu=worker_cpu,
        parent_artifact=parent_artifact, worker_artifact=worker_artifact,
        image_deployment=s['deployment'], image_provenance=s['provenance'],
        standalone_receipts=plan['standalone_cases'], numerical_packages=packages,
        numerical_cli_tests=cli_tests, supervisor_manifest=manifest_pin, C=C, H=H, read=read)
    guard(c)
    return c


def guard(c):
    c['pins'].recheck(); c['P'].guard(c['standalone'])
    for review in c['runtime_reviews'].values():
        for row in review['libraries']:
            V.require(Path(row['path']).resolve(strict=True) == Path(row['resolved']['path']),
                      'runtime library alias unchanged')
