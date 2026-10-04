"""Separate checked-image capture with joint-qualified executables; no mathematical acceptance."""
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
PRIOR_PACKAGE = ('p228-layer0-native-capture-gpu-v1', '957115945b1e1a12a6ee6b7fd815a98a33b9d7f1bdc99b77293cb5556b560535')
CAPTURE_LABEL = 'prefix-layer0-native-capture-gpu-v228-v1'
CAPTURE = (911582, '632a779159c60bae46aa7952c99519943fd83ca11b83de7d22bd1b9a9cb5742a')
LOWERING = (8652, '4f46acb4eaaadefc2c1426146424bda91a7e455ab3b57b23c935401e9c4d79bb')
INSPECTION_SHA = 'dbeb034c2f7520966e365ef5cb97751a075f70eb83cbcdfb272b82063db25df9'
IMAGE = (10864, '25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25')
# Actual completed joint CPU and its two selected Cargo ELF artifacts.
JOINT_CPU = (569967, 'bf1a12f78981d9ff9b8157e1dec6dca300752b238680e16380b98e9d1260bafb')
CPU_CONTROLLER_SHA = '6fc90b8cbdda07c52c761767c7c463a3ea0ebecb244c5ace7a0fc3e274ac2d94'
BINARIES = {
    'parent': (13542344, '37f20372c91e2ac6229b747d287e902809a165d6def5da7677eeeeabf565cc4d'),
    'worker': (5082304, 'ca9236967f8ae38666c46499040d55c1df86353dbb8dc4905ce41579ae9d4a8b'),
}
PARENT_NAME = 'ferric-qwen3-finite-projection-residual-layer-capture-engineering'
WORKER_NAME = 'ferric-tp-peer-finite-engineering-worker-v1'
INPUT_SCHEMA = 'ferric-p228-projection-residual-capture-inputs-v1'
REVIEW_SCHEMA = 'ferric-p228-projection-residual-capture-engineering-review-v1'
PACKAGE_SCHEMA = 'ferric-p228-projection-residual-capture-gpu-package-v1'
PURE_SCHEMA = 'ferric-p228-projection-residual-capture-gpu-pure-v1'
PLAN_FIELDS = ('schema output_label baseline baseline_capture parent_cpu worker_cpu parent worker request '
    'projection_image lowering_complete inspection_complete capture_review parent_runtime_review '
    'worker_runtime_review supervisor_tests supervisor_test_sources')
REVIEW_BINDINGS = ('baseline baseline_capture parent_cpu worker_cpu parent worker request '
    'projection_image lowering_complete inspection_complete parent_runtime_review worker_runtime_review').split()
REVIEW_FALSE = ('production_authority', 'full_model_acceptance', 'independent_numerical_acceptance',
    'numerical_acceptance', 'arithmetic_prerequisites_verified', 'runtime_premises_discharged', 'performance_claim',
    'timestamp_calibration', 'clock_domain_validated', 'cross_device_clock_alignment', 'overlap_claim',
    'paired_comparison_performed', 'full_forward', 'conditional_residual_checks_performed')
TOPICS = ('source_lineage', 'formal', 'isa', 'coherence', 'lifecycle', 'selected_device')
PACKAGE_FILES = {'INTAKE.md', 'README.md', 'capture_validation.py', 'intake.py', 'layer_validation.py',
    'run.py', 'run_row_facts_v2.py', 'test_capture_validation.py', 'test_intake.py', 'test_run.py'}
PURE_TESTS = 34
TEST_RUNNER_SHA = '99c322d21de6ca7c451bdc4bf629dca187894d6466a95a4230c858207727c2c4'

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
        and re.fullmatch(r'prefix-projection-residual-capture-gpu-v228-v[1-9][0-9]{0,8}', plan['output_label']),
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
        and re.fullmatch(r'projection-residual-capture-gpu-pure-v228-v[1-9][0-9]{0,8}', directory.name), 'new pure namespace')
    for key, filename in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'),
                          ('transcript', 'tests.log')):
        V.require(value[key]['path'] == str(directory / filename), 'actual retained pure output path')
        read(pins, value[key])
    before, after = doc(pins, value['sources_before']), doc(pins, value['sources_after'])
    expected = {row['path']: dict(row, path=str(Path(__file__).resolve().parent / row['path']))
                for row in manifest['files']}
    V.require(before == after == expected, 'exact tested package before and after')
    pins.pin(E / 'run_projection_residual_capture_gpu_pure_p228_v1.py', TEST_RUNNER_SHA)



def content(value):
    return value['bytes'], value['sha256']



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



def guard(c):
    c['pins'].recheck(); c['P'].guard(c['standalone'])
    for review in c['runtime_reviews'].values():
        for row in review['libraries']:
            V.require(Path(row['path']).resolve(strict=True) == Path(row['resolved']['path']),
                      'runtime library alias unchanged')


def previous(pins):
    base = D.package(pins, *PRIOR_PACKAGE)
    manifest, record = pins.json(base / 'manifest.json', PRIOR_PACKAGE[1])
    hashes = {row['path']: row['sha256'] for row in manifest['files']}
    old = D.load_module(pins, base / 'intake.py', hashes['intake.py'], 'projection_capture_previous_intake')
    validator = D.load_module(pins, base / 'capture_validation.py', hashes['capture_validation.py'],
                              'projection_capture_previous_validator')
    absent = object()
    previous_modules = {name: sys.modules.get(name, absent) for name in ('intake', 'capture_validation')}
    try:
        sys.modules['intake'], sys.modules['capture_validation'] = old, validator
        runner = D.load_module(pins, base / 'run.py', hashes['run.py'], 'projection_capture_previous_run')
    finally:
        for name, module in previous_modules.items():
            if module is absent:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = module
    return old, validator, runner, record


def replay_capture(pins, record, prior, old, validator, runner, manifest):
    V.require(content(record) == CAPTURE and record['path'] == str(E / CAPTURE_LABEL / 'complete.json'),
              'actual genuine prior one-layer capture')
    value = doc(pins, record)
    V.require(value['schema'] == 'ferric-p228-layer0-native-capture-gpu-v1'
        and value['passed'] is True and value['failures'] == [] and value['native_attempts'] == 1
        and value['retries'] == 0 and value['supervisor_manifest'] == manifest
        and value['baseline'] == prior['receipt_pin'], 'actual prior capture completion')
    V.require(value['controller'] == pins.pin(Path(old.__file__).with_name('run.py')), 'prior capture controller')
    plan = doc(pins, value['plan'])
    old.input_shape(plan)
    old_manifest, _ = pins.json(Path(old.__file__).with_name('manifest.json'), PRIOR_PACKAGE[1])
    old.supervisor_tests(pins, plan, old_manifest, manifest)
    for key, selected in (('request', 'request'), ('parent_cpu', 'parent_cpu_complete'),
                          ('worker_cpu', 'worker_cpu_complete'), ('parent_runtime_review', 'parent_runtime_review'),
                          ('worker_runtime_review', 'worker_runtime_review'), ('capture_review', 'capture_review')):
        V.require(plan[key] == value[selected], 'prior capture plan binding: ' + key)
        read(pins, plan[key])
    V.require(value['selected_runtime']['parent'] == plan['parent']
        and value['selected_runtime']['worker'] == plan['worker']
        and value['selected_runtime']['image'] == prior['runtime']['image'], 'prior capture selected runtime')
    request = doc(pins, plan['request'], 64 << 10)
    old.request_check(pins, request, prior, value['selected_runtime'], E / CAPTURE_LABEL)
    old.engineering_review(doc(pins, plan['capture_review']), plan, value['selected_runtime'], prior)
    replay_leaves(pins, value, prior['standalone']['platform'], prior['O'], dict(prior['P'].ENV),
                  E / CAPTURE_LABEL)
    records = value['retained_native']
    V.require(set(records) == set(validator.NAMES) | {'summary.json'}, 'complete prior capture bodies')
    files = {}
    for name, pin in records.items():
        V.require(pin['path'] == str(E / CAPTURE_LABEL / 'native' / name), 'prior capture body path')
        files[name] = read(pins, pin, V.LIMIT)
    checked = validator.validate(files.pop('summary.json'), files, request, prior['expected_hidden'])
    V.require(checked == value['checked'] == doc(pins, value['observation']), 'old genuine capture replay')
    parent = doc(pins, value['leaves']['parent']['result'])
    V.require(read(pins, parent['stdout']) == read(pins, records['summary.json']) + b'\n',
              'actual parent stdout and closed capture summary')
    validator.child_marker(read(pins, parent['stderr']), checked['closed_child_pids'][0])
    V.require(runner.lineage(pins, parent, checked) == value['owned_children'],
              'unchanged genuine prior ownership and child identity replay')
    return dict(body=files['candidate-capture.bin'],
                input=V.parse(files['candidate-bootstrap.json'])['input'],
                payload=records['candidate-capture.bin'], receipt=value)


def actual_tuple(value, label):
    V.require(type(value) is tuple and len(value) == 2 and type(value[0]) is int and value[0] > 0
        and type(value[1]) is str and re.fullmatch('[0-9a-f]{64}', value[1]),
        'root must bind actual completed ' + label)


def qualified_cpu(value):
    V.require(value['schema'] == 'ferric-p228-projection-residual-runtime-cpu-result-v1'
        and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
        and value['source_unchanged'] is True and value['parent_rebuilt'] is True
        and value['worker_rebuilt'] is True and value['empty_initial_target'] is True, 'actual joint CPU qualification')
    V.require(type(value['phases']) is dict and len(value['phases']) == 84, 'all actual joint CPU phases')
    for phase in value['phases'].values():
        V.require(type(phase['exit_code']) is int and phase['exit_code'] == 0 and phase['reason'] is None
            and phase['group_absent'] is True, 'natural reaped CPU phase')
    for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'timestamp_calibration',
                'production_authority', 'compiler_hsaco_reproduced'):
        V.require(value[key] is False, 'CPU qualification grants no native authority')
    tests = [value['tests']['worker'], *value['tests']['runtime'].values(), *value['tests']['parent'].values()]
    V.require(sum(row['passed'] for row in tests) == value['tests_passed']
        and sum(row['ignored'] for row in tests) == value['tests_ignored'] == 4, 'actual test census joins')
    V.require(len(value['added_worker_tests']) == 16 and len(set(value['added_worker_tests'])) == 16
        and len(value['added_parent_tests']['lib']) == 13
        and len(value['added_parent_tests']['bin']) == 1, 'declared additions actually qualified')
    selected = {}
    for role, name in (('parent', PARENT_NAME), ('worker', WORKER_NAME)):
        actual_tuple(BINARIES[role], role + ' ELF')
        entry = value['binaries'][name]
        V.keys(entry, 'artifact binary')
        a, b = entry['artifact'], entry['binary']
        V.require(content(b) == BINARIES[role] and a['reason'] == 'compiler-artifact'
            and a['target']['name'] == name and a['target']['kind'] == ['bin']
            and a['target']['crate_types'] == ['bin'] and a['executable'] == b['path']
            and a['filenames'] == [b['path']] and a['profile']['test'] is False
            and a['profile']['opt_level'] == '2' and a['profile']['debug_assertions'] is True
            and a['profile']['overflow_checks'] is True, 'actual joint Cargo executable')
        V.require(('tp-batch-engineering' in a['features']) if role == 'parent' else a['features'] == [],
                  'separate executable feature gates')
        selected[role] = entry
    return selected


def cpu_evidence(pins, plan, capture, prior):
    actual_tuple(JOINT_CPU, 'joint CPU receipt')
    V.require(type(CPU_CONTROLLER_SHA) is str and re.fullmatch('[0-9a-f]{64}', CPU_CONTROLLER_SHA)
        and type(BINARIES) is dict and set(BINARIES) == {'parent', 'worker'}, 'actual root-bound CPU generation')
    V.require(plan['parent_cpu'] == plan['worker_cpu'] and content(plan['parent_cpu']) == JOINT_CPU,
              'same actual joint source/build generation for both executables')
    value = doc(pins, plan['parent_cpu'])
    selected = qualified_cpu(value)
    V.require(value['controller']['sha256'] == CPU_CONTROLLER_SHA
        and value['prior_parent_cpu'] == capture['receipt']['parent_cpu_complete']
        and value['prior_worker_cpu'] == prior['plan']['worker_cpu'], 'joint CPU prior-generation and controller joins')
    read(pins, value['controller'])
    before = doc(pins, value['raw']['sources-before.json'])
    V.require(before and before == doc(pins, value['raw']['sources-after.json']), 'same actual formatted source snapshot')
    for role, phase in (('parent', 'parent-builds'), ('worker', 'worker-build')):
        raw = read(pins, value['raw'][phase + '-stdout'], 32 << 20)
        rows = [D.parse(line) for line in raw.splitlines() if line.strip()]
        V.require(sum(row == selected[role]['artifact'] for row in rows) == 1
            and hashlib.sha256(raw).hexdigest() == value['phases'][phase]['stdout_sha256'],
            'actual Cargo selected artifact and phase stream')
    return value, selected


def image_evidence(pins, plan):
    V.require(content(plan['lowering_complete']) == LOWERING
        and plan['inspection_complete']['sha256'] == INSPECTION_SHA
        and content(plan['projection_image']) == IMAGE, 'exact checked generic candidate and static inspection')
    lower = doc(pins, plan['lowering_complete'])
    inspected = doc(pins, plan['inspection_complete'])
    V.require(lower['schema'] == 'ferric-p228-projection-residual-lowering-result-v1'
        and lower['passed'] is True and lower['errors'] == [] and lower['postcheck_errors'] == []
        and lower['exit_code'] == lower['natural_exit_code'] == 0 and lower['owned_group_empty'] is True
        and lower['natural_group_absent_before_cleanup'] is True and lower['source_unchanged'] is True,
        'actual checked generic lowering, naturally closed')
    V.require(inspected['schema'] == 'ferric-p228-projection-residual-inspection-result-v1'
        and inspected['passed'] is True and inspected['error'] is None and inspected['postcheck_errors'] == []
        and inspected['source_unchanged'] is True and inspected['lowering_complete'] == plan['lowering_complete']
        and inspected['cpu_complete'] == lower['cpu_complete']
        and inspected['image'] == lower['artifact']['image']
        and content(inspected['image']) == IMAGE, 'actual static inspected image and CPU lineage')
    V.require(doc(pins, lower['artifact']['observation']) == lower['artifact']['value'],
              'actual generic emission record')
    for complete in (lower, inspected):
        for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority',
                    'load_authority', 'launch_authority'):
            V.require(complete[key] is False, 'checked compile/static inspection does not authorize execution')
    for name in ('descriptor-metadata', 'elf-notes', 'disassembly'):
        row = inspected['phases'][name]
        V.require(type(row['exit_code']) is int and row['exit_code'] == 0 and row['reason'] is None
            and row['group_absent'] is True, 'natural retained static inspection')
        for suffix in ('command.json', 'started.json', 'stdout', 'stderr', 'result.json'):
            pin = inspected['raw'][name + '-' + suffix]
            data = read(pins, pin)
            if suffix == 'result.json':
                V.require(D.parse(data) == row, 'inspection phase result join')
            elif suffix in ('stdout', 'stderr'):
                V.require(pin['sha256'] == row[suffix + '_sha256'], 'inspection phase stream join')
    read(pins, plan['projection_image'], 32 << 20, False)
    return dict(lowering=plan['lowering_complete'], inspection=plan['inspection_complete'],
                cpu_complete=lower['cpu_complete'], original_image=inspected['image'],
                observation=lower['artifact']['observation'], descriptor=inspected['inspection']['descriptor'])


def request_check(pins, value, prior, runtime, out, image):
    V.keys(value, 'schema layer projection_residual_image')
    V.require(value['schema'] == 'FerricFiniteProjectionResidualLayerCaptureRequestV1',
              'distinct projection capture request')
    layer, previous, L = value['layer'], prior['request'], prior['L']
    V.keys(layer, 'schema source worker images expected_bundle_id expected_model_id device_ids session prompt '
        'prefix_tiles_image mlp_tiles_image evidence_directory dispatch_timeout_ms child_deadline_ms')
    V.require(layer['schema'] == 'FerricFinitePrefixLayerCaptureRequestV1', 'unchanged inner capture schema')
    for key in ('source', 'images', 'expected_bundle_id', 'expected_model_id', 'device_ids', 'prompt',
                'dispatch_timeout_ms', 'child_deadline_ms'):
        V.require(layer[key] == previous[key], 'same genuine input: ' + key)
    V.require(previous['mode'] == 'teacher_forced' and layer['prefix_tiles_image'] == previous['prefix_image']
        and layer['mlp_tiles_image'] == previous['tiles_image']
        and L.rust_pin(value['projection_residual_image']) == image
        and L.rust_pin(layer['worker']) == runtime['worker'], 'only explicit new worker and extra image route')
    V.require(V.digest(layer['session']) != bytes(32) and layer['session'] != previous['session']
        and layer['evidence_directory'] == str(out / 'native'), 'fresh session and output')
    V.require(V.uint(layer['dispatch_timeout_ms']) == 10000 and V.uint(layer['child_deadline_ms']) == 3600000,
              'unchanged bounded deadlines')
    V.require(L.rust_pin(layer['prefix_tiles_image']) == runtime['image']
        and L.rust_pin(layer['mlp_tiles_image']) == prior['plan']['down2_image'], 'V7 and Down2 unchanged')
    for pin in [*layer['images'].values(), layer['prefix_tiles_image'], layer['mlp_tiles_image'],
                value['projection_residual_image']]:
        read(pins, L.rust_pin(pin), 32 << 20, False)
    for pin in layer['prompt'].values():
        read(pins, L.rust_pin(pin), 32 << 10, False)


def engineering_review(value, plan, runtime, prior, image):
    V.keys(value, 'schema reviewed authority gpu_attempts output_label image down2_image '
        'image_provenance down2_provenance projection_provenance review_topics notes '
        + ' '.join(REVIEW_BINDINGS) + ' ' + ' '.join(REVIEW_FALSE))
    V.require(value['schema'] == REVIEW_SCHEMA and value['reviewed'] is True and value['authority'] == 'none'
        and type(value['gpu_attempts']) is int and value['gpu_attempts'] == 1
        and value['output_label'] == plan['output_label'] and all(value[k] is False for k in REVIEW_FALSE),
        'one explicitly reviewed engineering attempt only')
    for key in REVIEW_BINDINGS:
        V.require(value[key] == plan[key], 'root review binds exact input: ' + key)
    V.require(value['parent'] == runtime['parent'] and value['worker'] == runtime['worker']
        and value['image'] == runtime['image'] and value['down2_image'] == prior['plan']['down2_image']
        and value['image_provenance'] == prior['standalone']['provenance']
        and value['down2_provenance'] == prior['down2_provenance']
        and value['projection_provenance'] == image, 'reviewed unchanged upstream and new arithmetic lineage')
    V.keys(value['review_topics'], ' '.join(TOPICS))
    for text in [value['notes'], *value['review_topics'].values()]:
        V.require(type(text) is str and len(text.strip()) >= 32 and len(text.encode()) <= 16384,
                  'substantive root review notes, not generated authority')


def context(plan_pin):
    pins = D.Pins()
    plan = doc(pins, plan_pin, 1 << 20); input_shape(plan)
    out = E / plan['output_label']
    V.require(not os.path.lexists(out), 'exclusive one-attempt output')
    manifest, manifest_pin = package_record(pins)
    supervisor_tests(pins, plan, manifest, manifest_pin)
    old, validator, runner, old_manifest = previous(pins)
    prior = old.baseline(pins, plan['baseline'])
    capture = replay_capture(pins, plan['baseline_capture'], prior, old, validator, runner, old_manifest)
    cpu, selected = cpu_evidence(pins, plan, capture, prior)
    runtime = dict(parent=deployed_binary(pins, plan['parent'], selected['parent']['binary']),
        worker=deployed_binary(pins, plan['worker'], selected['worker']['binary']), image=prior['runtime']['image'])
    image = image_evidence(pins, plan)
    request = doc(pins, plan['request'], 64 << 10)
    request_check(pins, request, prior, runtime, out, plan['projection_image'])
    P, standalone = prior['P'], prior['standalone']
    reviews = {}
    for role in ('parent', 'worker'):
        reviews[role] = doc(pins, plan[role + '_runtime_review'])
        P.runtime_review(reviews[role], runtime[role], standalone['platform'], pins)
    engineering_review(doc(pins, plan['capture_review'], 192 << 10), plan, runtime, prior, image)
    c = dict(pins=pins, plan=plan, plan_pin=plan_pin, request=request, out=out, runtime=runtime,
        standalone=standalone, P=P, O=prior['O'], platform=standalone['platform'], topology=standalone['topology'],
        owned=standalone['owned'], environment=dict(P.ENV), runtime_reviews=reviews,
        baseline_capture_body=capture['body'], baseline_input=capture['input'], baseline_capture_payload=capture['payload'],
        supervisor_manifest=manifest_pin, baseline=prior, parent_cpu=cpu, worker_cpu=cpu)
    guard(c)
    return c

def replay_leaves(pins, value, platform, O, environment, directory):
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
