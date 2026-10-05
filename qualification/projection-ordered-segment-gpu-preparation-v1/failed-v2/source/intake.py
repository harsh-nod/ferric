"""Explicit default/shared AR4 arms with one qualified generation and unchanged images."""
import contextlib
import copy
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
PRIOR_PACKAGE = ('p228-projection-residual-capture-gpu-v1', '4bcfdd5c53c78e2c6e41cebb889a2fb39cc113d8c5aa0799afb45c1396cc92fc')
CAPTURE_LABEL = 'prefix-projection-residual-capture-gpu-v228-v1'
CAPTURE = (955163, '4f25030567c470062fd50862876bc35e93d080f1778c0be4ca36f2b39af4199a')
LOWERING = (8652, '4f46acb4eaaadefc2c1426146424bda91a7e455ab3b57b23c935401e9c4d79bb')
INSPECTION_SHA = 'dbeb034c2f7520966e365ef5cb97751a075f70eb83cbcdfb272b82063db25df9'
IMAGE = (10864, '25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25')
SILU_PACKAGE = ('p228-silu-materialized-capture-gpu-v1',
    '71cae69a25ac82c53bdf2976ab53b9d9af615c5af4b8337c2e386beb44a8a18b')
SILU_CAPTURE = (1024009, '67a235280b48cb5f6a2c51bcca534c182c843857c4c4a98e86718ed368ef12e4')
SILU_CPU = (84947, 'e139729d8fa95715ef8017679ee21f1175ba302b681785789e3dc84137508a30')
SILU_LOWERING = (25125, 'fd7d4c792dc7ddaa831b3372549fe3896a18e8c94b96598b553df94156d949b4')
SILU_OWNER = (21726, '3ee5600755d5b77846d16429d58e2395df0616c86fbe705ea24af07fd9f9e464')
SILU_IMAGE = (33320, 'b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589')
COMPARISON_PACKAGE_SHA = '2678c7e43318eb56388154bea3f5a4e5f6aaac3b307b7bd9865e5642a97adba2'
COMPARISON_CONTROLLER_SHA = 'fa179f6c9b1f7de3330c1e31cbbe0e22e9aa8e4456f0535abe19ac4a3b47c482'
COMPARISON_PURE_SHA = '278f480dbce3fa04383ecda1366f11fd7ba51cef42592d0a48cebce20ee1479a'
LAYER_COMPARISON = (1508927, '828f8fdb4fc4194b5d7d5c65135222b472d2ec76080667fae8f0b8febba169aa')
PREFIX_STAGES = ('fixture-metadata', 'checked-lowering', 'actual-replay', 'actual-inert-join',
                 'emit', 'extract-retained', 'descriptor-metadata', 'elf-notes', 'disassembly')
PREFIX_RUST = {'src/lib.rs', 'src/wave_numerics_v1.rs', 'src/head_rope_numerics_v3.rs',
    'src/prefix_reciprocal_numerics_v1.rs', 'src/attention_online.rs', 'src/output_projection_numerics_v5.rs',
    'src/prefix_tiles_numerics_v6.rs', 'src/prefix_rope_materialized_numerics_v1.rs'}
PREFIX_CHANGED = {'src/lib.rs', 'src/prefix_rope_materialized_numerics_v1.rs',
                  'tests/rope_materialized_v1.rs', 'tests/fixtures/v7_lib.rs'}
# Actual completed AR4 CPU record; deployment still rehashes both ELF bodies.
JOINT_CPU = (583779, '7a3c170ca6ffe6000517588280fcdb99cb58c0cbb22cc3f4be910232f7d51c54')
BINARIES = {
    'parent': (13827528, '22d2cb744117af5232e29001f49ff265c25d94f9326ee48a1eb3ddc5aeab6319'),
    'worker': (5133800, 'a3079acbd8ff05fbc18503fb7f7a840a4732bc0ab60717b86c03c186198bee7d'),
}
CPU_LABEL = 'projection-ar4-cpu-v228-v1'
CPU_CONTROLLER_SHA = 'a4f9fe5df6d6203003eaffd11ba0a7dee0586e16a5b0fc419599c8bab16b57b3'
CPU_INPUT_SHA = 'f2cb8e4b233c6960139cd326c440f0682b575685c1647975781702e341750f0f'
CPU_PROPOSAL_SHA = '58ae10d372b552190a6c811e1a0e53591171a6d8682cafd3bca55279d2eb3d7f'
PRIOR_CPU = (584169, '1f4365064da1a0884385035d1f2d280e6bba66afc2b5bb4265bf7d60b1418c83')
PARENT_NAME = 'ferric-qwen3-finite-projection-residual-decode-engineering'
WORKER_NAME = 'ferric-tp-peer-finite-engineering-worker-v1'
INPUT_SCHEMA = 'ferric-p228-projection-ordered-segment-inputs-v1'
REVIEW_SCHEMA = 'ferric-p228-projection-ordered-segment-engineering-review-v1'
PACKAGE_SCHEMA = 'ferric-p228-projection-ordered-segment-gpu-package-v2'
PURE_SCHEMA = 'ferric-p228-projection-ordered-segment-gpu-pure-v2'
PLAN_FIELDS = ('schema output_label route baseline parent_cpu worker_cpu parent worker request '
    'projection_image lowering_complete inspection_complete decode_review parent_runtime_review '
    'worker_runtime_review supervisor_tests supervisor_test_sources layer_comparison '
    'mlp_image mlp_cpu mlp_lowering_complete mlp_lowering_owner '
    'prefix_image prefix_cpu prefix_emission_complete prefix_emission_owner')
REVIEW_BINDINGS = ('route baseline parent_cpu worker_cpu parent worker request '
    'projection_image lowering_complete inspection_complete parent_runtime_review worker_runtime_review '
    'layer_comparison mlp_image mlp_cpu mlp_lowering_complete mlp_lowering_owner '
    'prefix_image prefix_cpu prefix_emission_complete prefix_emission_owner').split()
REVIEW_FALSE = ('production_authority', 'full_model_acceptance', 'independent_numerical_acceptance',
    'numerical_acceptance', 'arithmetic_prerequisites_verified', 'runtime_premises_discharged', 'performance_claim',
    'timestamp_calibration', 'clock_domain_validated', 'cross_device_clock_alignment', 'overlap_claim',
    'paired_comparison_performed', 'full_forward_acceptance', 'conditional_residual_checks_performed')
TOPICS = ('source_lineage', 'formal', 'isa', 'coherence', 'lifecycle', 'selected_device')
PACKAGE_FILES = {'INTAKE.md', 'README.md', 'decode_validation.py', 'intake.py', 'layer_validation.py',
    'run.py', 'run_row_facts_v2.py', 'stage_core.py', 'smoke_validation.py',
    'test_decode_validation.py', 'test_intake.py', 'test_run.py', 'test_silu_admission.py',
    'prefix_contracts.py', 'test_rope_admission.py', 'test_linked_admission.py',
    'host_validation.py', 'observer_cpu.py', 'test_host_validation.py', 'test_observer_cpu.py',
    'run_projection_ordered_segment_gpu_pure_p228_v2.py', 'shared_cpu.py', 'pair.py',
    'test_shared_cpu.py', 'test_shared_host.py', 'test_pair.py', 'ordered_cpu.py', 'ordered_validation.py', 'ordered_pair.py',
    'test_ordered_cpu.py', 'test_ordered_validation.py', 'test_ordered_host.py', 'test_ordered_pair.py'}
PURE_TESTS = 174
TEST_RUNNER_SHA = '20bea4bf63e1fb0cb606930128cce34e1d25a3c1504345cae1eb160585ee9463'

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

RPO_PREREQUISITES = {
    'cpu': dict(path=str(E / 'rpo-compiler-cpu-v228-v2/complete.json'), bytes=375781,
        sha256='56fc51fc326980e00156d550d0a7052f44bb481ded7b9948c06217653fb246c1'),
    'cpu_owner': dict(path=str(E / 'rpo-compiler-cpu-owner-v228-v2/complete.json'), bytes=57128,
        sha256='afca99d8799f910ce607873c320b8f244be66f9ffdb3df4dce4545eb1552ea66'),
    'tools': dict(path=str(E / 'rpo-finalizer-tools-v228-v2/complete.json'), bytes=44858,
        sha256='39eab92ede34991b08c168e827c7111c13533627cbf2efd3d248c6742a92660b'),
    'tools_owner': dict(path=str(E / 'rpo-finalizer-tools-owner-v228-v2/complete.json'), bytes=11103,
        sha256='e919fb520672be2a909fd0ec8301bc9923210680f818e411b7db459ffc3059dd'),
}
PREFIX_PACKAGE = dict(path=str(E / 'p228-rope-materialized-rpo-lowering-v1/manifest.json'), bytes=2312,
    sha256='6ed02271fa204a9d2f00f81751425bd26860456ff753c360b079167b1d4d87b0')
PRODUCER = E / 'row-rope-materialized-rpo-checked-probe-v228-v1'
PRODUCER_PIN = dict(path=str(PRODUCER / 'failed.json'), bytes=21555,
    sha256='1fbbbd9a8889e1b33b724b3f8911c4ecc1074fffe8fc29377f6840686375ba1f')
PRODUCER_OWNER_PIN = dict(path=str(E / 'rope-materialized-rpo-checked-probe-owner-v228-v1/failed.json'), bytes=28117,
    sha256='8fb2f02cde719c08fd41535d59d39980d8c6328f27cf952770c481f5b48edbbf')
CONSUMER = E / 'kir-indexed-formal-join-cpu-v228-v2'
CONSUMER_PIN = dict(path=str(CONSUMER / 'complete.json'), bytes=300367,
    sha256='4548c7ee32206f6fc94bf54b5c50e13c500d2f4381e58092094893d9d80d4bfa')
CONSUMER_OWNER_PIN = dict(path=str(E / 'kir-indexed-formal-join-cpu-owner-v228-v2/complete.json'), bytes=35109,
    sha256='21205fbed2b56d5a27b4c1fbb2f4f3d5f0e810b822a518e3f875e3083553954e')
CONSUMER_SOURCE_SHA = 'f16b193aa6a505ab2b28d98334e86b60ac5bcb6cfbd09d885cecd33d5cc27854'
EMISSION_PACKAGE_SHA = 'b46ad9dbfb7d4a9f922adc80a2862df47e3801e4b3acd9d455357cc959a15233'
EMISSION_DIR = E / 'rope-indexed-checked-emission-v228-v1'
EMISSION_OWNER_DIR = E / 'rope-indexed-checked-emission-owner-v228-v1'
EMISSION_PIN = dict(path=str(EMISSION_DIR / 'complete.json'), bytes=197494,
    sha256='d45dd1a2ed5b88e767b14bbb8c7d2728211bb49aeaa14b1f8b4cc5a5b4f80c49')
EMISSION_OWNER_PIN = dict(path=str(EMISSION_OWNER_DIR / 'complete.json'), bytes=31010,
    sha256='d9e4794a60a7d48daa953701ae764e51e99b70270310537c9f93d21afb8aa991')
PREFIX_IMAGE = (54344, '29fd58e7b09fee003ed31660e6201c20a7eb5da4b873d0b4d954993f7018f2f8')
PRODUCER_STAGES = PREFIX_STAGES[:3]
EMISSION_STAGES = ('metadata', 'tool-build', *PREFIX_STAGES[3:])
EMISSION_ARTIFACTS = {'emitted/source.handoff-v3', 'emitted/compiler.handoff-v2',
    'emitted/artifact.hsaco', 'emitted/receipt.txt', 'extracted/formal.archive', 'extracted/module.ll'}
EMISSION_FALSE = ('fresh_checked_lowering', 'fresh_checked_replay', 'fresh_compiler_built',
    'full_compiler_cohort_requalified', 'gpu_execution', 'numerical_acceptance', 'performance_claim',
    'production_authority', 'launch_authority', 'runtime_requirements_discharged', 'full_model_acceptance')
HANDOFF = dict(path=str(PRODUCER / 'prefix-tiles.handoff-v3'), bytes=4078537,
    sha256='ad4b31ee88efa54702dc8dff13e1e3c331c296734f7cdcfa52e8249ed9fa37fc')


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
        and type(plan['route']) is str and plan['route'] in ('default', 'shared', 'ordered')
        and re.fullmatch(r'prefix-projection-ordered-segment-' + plan['route'] + r'-gpu-v228-v[1-9][0-9]{0,8}', plan['output_label']),
        'closed candidate-only capture plan')
    for name in PLAN_FIELDS.split():
        if name not in ('schema', 'output_label', 'route'):
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
        and re.fullmatch(r'projection-ordered-segment-gpu-pure-v228-v[1-9][0-9]{0,8}', directory.name), 'new pure namespace')
    for key, filename in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'),
                          ('transcript', 'tests.log')):
        V.require(value[key]['path'] == str(directory / filename), 'actual retained pure output path')
        read(pins, value[key])
    before, after = doc(pins, value['sources_before']), doc(pins, value['sources_after'])
    expected = {row['path']: dict(row, path=str(Path(__file__).resolve().parent / row['path']))
                for row in manifest['files']}
    V.require(before == after == expected, 'exact tested package before and after')
    pins.pin(E / 'run_projection_ordered_segment_gpu_pure_p228_v2.py', TEST_RUNNER_SHA)



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


def baseline_bindings(plan, value):
    for key, receipt_key in (('baseline', 'baseline'), ('baseline_capture', 'baseline_capture'),
            ('request', 'request'), ('parent', 'parent'), ('worker', 'worker'),
            ('parent_cpu', 'parent_cpu_complete'), ('worker_cpu', 'worker_cpu_complete'),
            ('projection_image', 'projection_image'), ('lowering_complete', 'lowering_complete'),
            ('inspection_complete', 'inspection_complete'), ('parent_runtime_review', 'parent_runtime_review'),
            ('worker_runtime_review', 'worker_runtime_review'), ('capture_review', 'capture_review')):
        V.require(plan[key] == value[receipt_key], 'original capture plan binding: ' + key)


def corrected_baseline(pins, record):
    V.require(content(record) == CAPTURE and record['path'] == str(E / CAPTURE_LABEL / 'complete.json'),
              'actual passed projection layer capture, not a fabricated TF4 baseline')
    C, validator, runner, manifest_pin = previous(pins)
    value = doc(pins, record)
    V.require(value['schema'] == 'ferric-p228-projection-residual-capture-gpu-v1'
        and value['passed'] is True and value['failures'] == [] and value['native_attempts'] == 1
        and value['retries'] == 0 and value['supervisor_manifest'] == manifest_pin
        and value['full_forward'] is False and value['numerical_acceptance'] is False,
        'actual closed prior layer-only observation')
    V.require(value['controller'] == pins.pin(Path(C.__file__).with_name('run.py')), 'actual prior controller')
    plan = doc(pins, value['plan']); C.input_shape(plan); baseline_bindings(plan, value)
    manifest, _ = pins.json(Path(C.__file__).with_name('manifest.json'), PRIOR_PACKAGE[1])
    C.supervisor_tests(pins, plan, manifest, manifest_pin)
    old, old_validator, old_runner, old_manifest = C.previous(pins)
    prior = old.baseline(pins, plan['baseline'])
    capture = C.replay_capture(pins, plan['baseline_capture'], prior, old, old_validator, old_runner, old_manifest)
    cpu, selected = C.cpu_evidence(pins, plan, capture, prior)
    runtime = dict(parent=C.deployed_binary(pins, plan['parent'], selected['parent']['binary']),
        worker=C.deployed_binary(pins, plan['worker'], selected['worker']['binary']), image=prior['runtime']['image'])
    V.require(runtime == value['selected_runtime'], 'actual prior runtime selection')
    image = C.image_evidence(pins, plan)
    request = doc(pins, plan['request'], 64 << 10)
    C.request_check(pins, request, prior, runtime, E / CAPTURE_LABEL, plan['projection_image'])
    C.engineering_review(doc(pins, plan['capture_review'], 192 << 10), plan, runtime, prior, image)
    P, standalone = prior['P'], prior['standalone']
    for role in ('parent', 'worker'):
        P.runtime_review(doc(pins, plan[role + '_runtime_review']), runtime[role], standalone['platform'], pins)
    C.replay_leaves(pins, value, standalone['platform'], prior['O'], dict(P.ENV), E / CAPTURE_LABEL)
    records = value['retained_native']
    V.require(set(records) == validator.BODY | {'summary.json'}, 'complete original layer capture')
    files = {}
    for name, pin in records.items():
        V.require(pin['path'] == str(E / CAPTURE_LABEL / 'native' / name), 'original capture file namespace')
        files[name] = read(pins, pin, V.LIMIT)
    raw = files.pop('summary.json')
    checked = validator.validate(raw, files, request, capture['body'], capture['input'])
    V.require(checked == value['checked'] == doc(pins, value['observation']), 'original layer structure replay')
    native = doc(pins, value['leaves']['parent']['result'])
    V.require(read(pins, native['stdout']) == raw + b'\n', 'prior summary/stdout join')
    validator.child_marker(read(pins, native['stderr']), checked['closed_child_pids'][0])
    V.require(runner.lineage(pins, native, checked) == value['owned_children'], 'prior owned identity replay')
    pins.recheck(); P.guard(standalone)
    return dict(receipt_pin=record, receipt=value, plan=plan, cpu=cpu, prior=prior,
                image_provenance=image)


def actual_tuple(value, label):
    V.require(type(value) is tuple and len(value) == 2 and type(value[0]) is int and value[0] > 0
        and type(value[1]) is str and re.fullmatch('[0-9a-f]{64}', value[1]),
        'root must bind actual completed ' + label)


@contextlib.contextmanager
def module_aliases(values):
    absent = object(); previous = {name: sys.modules.get(name, absent) for name in values}
    try:
        sys.modules.update(values)
        yield
    finally:
        for name, value in previous.items():
            if value is absent:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = value


def silu_helpers(pins):
    base = D.package(pins, *SILU_PACKAGE)
    manifest, record = pins.json(base / 'manifest.json', SILU_PACKAGE[1])
    hashes = {row['path']: row['sha256'] for row in manifest['files']}
    S = D.load_module(pins, base / 'intake.py', hashes['intake.py'], 'silu_layer_previous_intake')
    validator = D.load_module(pins, base / 'capture_validation.py', hashes['capture_validation.py'],
                              'silu_layer_previous_validator')
    contracts = D.load_module(pins, base / 'mlp_contracts.py', hashes['mlp_contracts.py'], 'silu_mlp_contracts')
    with module_aliases({'intake': S, 'capture_validation': validator}):
        runner = D.load_module(pins, base / 'run.py', hashes['run.py'], 'silu_layer_previous_runner')
    return S, validator, runner, contracts, manifest, record


def silu_plan_bindings(plan, capture_plan):
    for key, expected in (('mlp_cpu', SILU_CPU), ('mlp_lowering_complete', SILU_LOWERING),
                          ('mlp_lowering_owner', SILU_OWNER), ('mlp_image', SILU_IMAGE)):
        V.require(content(plan[key]) == expected and plan[key] == capture_plan[key],
                  'exact CPU38/checked image already selected in genuine SiLU layer: ' + key)
    for key in ('projection_image', 'lowering_complete', 'inspection_complete'):
        V.require(plan[key] == capture_plan[key], 'unchanged separately selected residual image: ' + key)


def comparison_contract(value, plan, capture):
    V.require(value['schema'] == 'ferric-p228-silu-materialized-comparison-observation-v1'
        and value['completed'] is True and value['source_postchecks_passed'] is True
        and value['native_structural_replayed'] is True and value['candidate_owned_leaves_replayed'] == 7
        and value['candidate_audits_replayed'] == 6 and value['framework_owned_leaf_results_rechecked'] == 21
        and value['native_outer'] == plan['baseline'] and content(value['baseline_outer']) == CAPTURE
        and value['native_supervisor_manifest'] == capture['supervisor_manifest']
        and value['manifest']['sha256'] == COMPARISON_PACKAGE_SHA
        and value['controller']['sha256'] == COMPARISON_CONTROLLER_SHA
        and value['comparison_tests']['sha256'] == COMPARISON_PURE_SHA,
        'actual source-bound genuine SiLU layer comparison')
    for key in ('gpu_execution', 'native_process_launched', 'numerical_acceptance',
                'full_model_correctness', 'performance_claim', 'production_authority'):
        V.require(value[key] is False, 'comparison grants no model or execution authority')
    m = value['comparison']; inputs = m['inputs']
    V.require(m['schema'] == 'ferric-p228-silu-materialized-comparison-v1'
        and m['input_token'] == 9112 and m['position'] == 0 and m['comparable_rows'] == 24
        and len(m['comparisons']) == len(m['baseline_comparisons']) == 24
        and m['unchanged_pre_swiglu_count'] == 22 and m['conditional_residual_words'] == 16384
        and len(m['conditional_residual_comparisons']) == 4
        and m['conditional_residuals_exact'] is True
        and all(row['byte_equal'] is True and row['elements'] == 4096 for row in m['conditional_residual_comparisons'])
        and m['framework_product_control_exact'] is True and m['genuine_independent_framework_outputs'] is True
        and m['numerical_acceptance'] is False and m['full_layer_numerics_accepted'] is False
        and m['full_model_correctness'] is False and m['acceptance_threshold'] is None,
        'completed independent diagnostics and exact conditional residual contract, not full-model acceptance')
    V.require(inputs['candidate_capture'] == capture['retained_native']['summary.json']
        and inputs['selected_mlp_image'] == plan['mlp_image'] and inputs['projection_image'] == plan['projection_image']
        and inputs['worker'] == capture['worker'] and inputs['current_tf4'] == capture['baseline']
        and inputs['framework_capture']['sha256'] == 'cf7512025bb469e06f87c32f788da807b4e6607297b98bb0b33c4135d1e2c78e'
        and inputs['original_framework']['sha256'] == '2edddf40cc6195fdd622e479e8e46f2a659f1b569106f5f4e51afdb83bdc2416',
        'actual same images/native capture/genuine framework')


def baseline(pins, plan):
    record = plan['baseline']; path = Path(record['path'])
    V.require(content(record) == SILU_CAPTURE and path == E / 'prefix-silu-materialized-capture-gpu-v228-v1/complete.json',
              'actual completed SiLU layer capture')
    corrected = corrected_baseline(pins, dict(path=str(E / CAPTURE_LABEL / 'complete.json'), bytes=CAPTURE[0], sha256=CAPTURE[1]))
    S, validator, runner, contracts, manifest, manifest_pin = silu_helpers(pins)
    value = doc(pins, record)
    V.require(value['schema'] == 'ferric-p228-silu-materialized-capture-gpu-v1'
        and value['passed'] is True and value['failures'] == [] and value['native_attempts'] == 1 and value['retries'] == 0
        and value['supervisor_manifest'] == manifest_pin and value['controller'] == pins.pin(Path(S.__file__).with_name('run.py'))
        and value['captured_arrays'] == 28 and value['pre_swiglu_arrays_equal'] is True and value['pre_swiglu_array_count'] == 22
        and value['conditional_residual_checks_performed'] is False and value['numerical_acceptance'] is False
        and value['full_forward'] is False, 'actual closed one-layer SiLU capture')
    previous_plan = doc(pins, value['plan']); S.input_shape(previous_plan)
    baseline_bindings(previous_plan, value)
    V.require(previous_plan['baseline_capture'] == corrected['receipt_pin'], 'same actual corrected layer predecessor')
    for key in ('mlp_image', 'mlp_cpu', 'mlp_lowering_complete', 'mlp_lowering_owner'):
        V.require(previous_plan[key] == value[key], 'actual SiLU layer receipt binding: ' + key)
    silu_plan_bindings(plan, previous_plan)
    S.supervisor_tests(pins, previous_plan, manifest, manifest_pin)
    C, _, _, _ = previous(pins)
    runtime = S.selected_runtime(pins, previous_plan, corrected, C)
    V.require(runtime == value['selected_runtime'], 'same CPU988 layer runtime')
    with module_aliases({'mlp_contracts': contracts}):
        mlp = S.mlp_evidence(pins, previous_plan, corrected['prior'])
    request = doc(pins, previous_plan['request'], 64 << 10)
    S.request_check(pins, request, corrected, runtime, path.parent, plan['mlp_image'])
    S.engineering_review(doc(pins, previous_plan['capture_review'], 192 << 10), previous_plan, runtime, corrected, mlp)
    prior = corrected['prior']; P, standalone = prior['P'], prior['standalone']
    for role in ('parent', 'worker'):
        P.runtime_review(doc(pins, previous_plan[role + '_runtime_review']), runtime[role], standalone['platform'], pins)
    replay_leaves(pins, value, standalone['platform'], prior['O'], dict(P.ENV), path.parent)
    records = value['retained_native']; V.require(set(records) == validator.BODY | {'summary.json'}, 'twelve original SiLU capture files')
    files = {}
    for name, pin in records.items():
        V.require(pin['path'] == str(path.parent / 'native' / name), 'original SiLU layer namespace')
        files[name] = read(pins, pin, V.LIMIT)
    summary = files.pop('summary.json'); old_records = corrected['receipt']['retained_native']
    checked = validator.validate(summary, files, request, read(pins, old_records['candidate-capture.bin'], V.LIMIT),
        doc(pins, old_records['candidate-bootstrap.json'])['layer']['input'])
    V.require(checked == value['checked'] == doc(pins, value['observation']), 'actual SiLU layer structural replay')
    native = doc(pins, value['leaves']['parent']['result'])
    V.require(read(pins, native['stdout']) == summary + b'\n'
        and doc(pins, native['command'])['argv'] == [runtime['parent']['path'], '--request', previous_plan['request']['path'],
            '--capture-projection-residual-layer-zero', '--allow-unauthenticated-machine-code'], 'actual one-layer selector/stdout')
    validator.child_marker(read(pins, native['stderr']), checked['closed_child_pids'][0])
    V.require(runner.lineage(pins, native, checked) == value['owned_children'], 'actual SiLU child Close and owned lineage')
    V.require(content(plan['layer_comparison']) == LAYER_COMPARISON
        and plan['layer_comparison']['path'] == str(E / 'silu-materialized-comparison-v228-v1/complete.json'),
        'actual completed independent SiLU comparison')
    comparison = doc(pins, plan['layer_comparison'], 8 << 20); comparison_contract(comparison, plan, value)
    for key in ('controller', 'manifest', 'comparison_tests'):
        read(pins, comparison[key])
    tested = doc(pins, comparison['comparison_tests'])
    V.require(tested['passed'] is True and tested['tests'] == 12 and tested['errors'] == tested['failures'] == tested['skipped'] == 0
        and tested['controller'] == comparison['controller'] and tested['manifest'] == comparison['manifest']
        and tested['source_postchecks_passed'] is True
        and doc(pins, tested['sources_before']) == doc(pins, tested['sources_after']) == comparison['comparison_sources'],
        'actual twelve-test comparison source closure')
    read(pins, tested['transcript'])
    for pin in comparison['comparison_sources'].values():
        read(pins, pin)
    pins.recheck(); P.guard(standalone)
    return dict(receipt_pin=record, receipt=value, plan=previous_plan, cpu=corrected['cpu'], prior=prior,
                image_provenance=corrected['image_provenance'], mlp_provenance=mlp,
                layer_comparison=plan['layer_comparison'])


def qualified_cpu(value):
    V.require(value['schema'] == 'ferric-projection-ar4-cpu-result-v1'
        and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
        and value['source_unchanged'] is True and value['empty_initial_target'] is True,
        'actual fresh decode CPU qualification')
    V.require(type(value['phases']) is dict and len(value['phases']) == 87
        and len(value['binaries']) == 17 and set(value['metadata']) == {'parent', 'worker'},
        'new joint phase/artifact/metadata roster')
    for phase in value['phases'].values():
        V.require(type(phase['exit_code']) is int and phase['exit_code'] == 0 and phase['reason'] is None
            and phase['group_absent'] is True, 'natural reaped CPU phase')
    for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority'):
        V.require(value[key] is False, 'CPU qualification grants no native authority')
    tests = value['tests']
    V.require(type(tests) is dict and tests, 'actual flat test records')
    for row in tests.values():
        V.require(type(row['passed']) is int and row['passed'] >= 0
            and type(row['ignored']) is int and row['ignored'] >= 0
            and len(row['names']) == len(set(row['names'])) == row['passed'] + row['ignored']
            and row['summaries'] and all(len(s) == 3 and all(type(n) is int and n >= 0 for n in s)
                and s[1] == 0 for s in row['summaries'])
            and sum(s[0] for s in row['summaries']) == row['passed']
            and sum(s[2] for s in row['summaries']) == row['ignored'], 'actual passing and ignored name census')
    V.require(type(value['tests_passed']) is int and value['tests_passed'] == 1037
        and sum(row['passed'] for row in tests.values()) == value['tests_passed']
        and type(value['tests_ignored']) is int
        and sum(row['ignored'] for row in tests.values()) == value['tests_ignored'] == 4, 'actual test aggregate')
    selected = {}
    for role, name in (('parent', PARENT_NAME), ('worker', WORKER_NAME)):
        actual_tuple(BINARIES[role], role + ' ELF')
        entry = value['binaries'][name]; V.keys(entry, 'artifact binary')
        a, b = entry['artifact'], entry['binary']
        root = E / CPU_LABEL
        source_dir = 'm1-engineering-execution-v1' if role == 'parent' else 'tp-peer-finite-engineering-worker-v1'
        source = root / 'sources/ferric/adapters' / source_dir
        V.require(content(b) == BINARIES[role] and b['path'] == str(root / 'target' / role / 'debug' / name)
            and a['reason'] == 'compiler-artifact' and a['target']['name'] == name
            and a['target']['kind'] == a['target']['crate_types'] == ['bin']
            and a['executable'] == b['path'] and a['filenames'] == [b['path']]
            and a['manifest_path'] == str(source / 'Cargo.toml')
            and a['target']['src_path'] == str(source / 'src' / ('main.rs' if role == 'worker' else 'bin/' + name + '.rs'))
            and a['profile'] == dict(test=False, opt_level='2', debug_assertions=True,
                overflow_checks=True, debuginfo=0), 'actual new Cargo artifact')
        V.require(('tp-batch-engineering' in a['features']) if role == 'parent' else a['features'] == [],
                  'separate executable feature gates')
        selected[role] = entry
    return selected



def cpu_lineage(value, previous_cpu, previous_capture):
    V.require(content(value['prior_completion']) == PRIOR_CPU
        and value['prior_completion']['path'] == str(E / 'projection-residual-decode-cpu-v228-v1/complete.json')
        and value['controller']['sha256'] == CPU_CONTROLLER_SHA
        and value['controller']['path'] == str(E / 'p228-projection-ar4-cpu-v1/run.py'),
        'actual AR4 controller with exact CPU1022 predecessor')
    V.require(previous_cpu['schema'] == 'ferric-projection-residual-decode-cpu-result-v1'
        and previous_cpu['passed'] is True and previous_cpu['error'] is None
        and previous_cpu['postcheck_errors'] == [] and previous_cpu['source_unchanged'] is True
        and previous_cpu['tests_passed'] == 1022 and previous_cpu['tests_ignored'] == 4
        and previous_cpu['prior_completion'] == previous_capture['receipt']['parent_cpu_complete']
            == previous_capture['receipt']['worker_cpu_complete'],
        'CPU1022 descends from CPU988 layer runtime without equating rebuilt executables')


def prior_cpu_evidence(pins, plan, previous_capture):
    actual_tuple(JOINT_CPU, 'decode CPU receipt')
    V.require(plan['parent_cpu'] == plan['worker_cpu'] and content(plan['parent_cpu']) == JOINT_CPU
        and plan['parent_cpu']['path'] == str(E / CPU_LABEL / 'complete.json'), 'same actual new CPU generation')
    value = doc(pins, plan['parent_cpu'])
    selected = qualified_cpu(value)
    previous_cpu = doc(pins, value['prior_completion'])
    cpu_lineage(value, previous_cpu, previous_capture)
    read(pins, value['controller'])
    proposal = doc(pins, value['proposal'])
    V.require(value['proposal']['path'] == str(E / 'p228-projection-ar4-runtime-v1/source-manifest.json')
        and value['proposal']['sha256'] == CPU_PROPOSAL_SHA
        and proposal['schema'] == 'ferric-p228-projection-ar4-runtime-source-proposal-v1'
        and len(proposal['files']) == 9 and proposal['authored_added_tests'] == 13
        and value['declared_test_additions'] == proposal['added_tests']
        and value['declared_test_renames'] == proposal['renamed_tests'],
        'exact reviewed AR4 source and named test delta')
    inputs = [v for v in value['inputs'] if v['path'] == str(E / 'projection-ar4-cpu-inputs-v228-v1.json')]
    V.require(len(inputs) == 1 and inputs[0]['sha256'] == CPU_INPUT_SHA, 'exact paired source/overlay input manifest')
    read(pins, inputs[0])
    for name in ('sources-before.json', 'sources-after.json'):
        V.require(value['raw'][name]['path'] == str(E / CPU_LABEL / name), 'actual source snapshot namespace')
    before = doc(pins, value['raw']['sources-before.json'])
    V.require(before and before == doc(pins, value['raw']['sources-after.json']), 'actual compiled source custody')
    for role, phase in (('parent', 'parent-builds'), ('worker', 'worker-build')):
        pin = value['raw'][phase + '-stdout']
        V.require(pin['path'] == str(E / CPU_LABEL / (phase + '-stdout')), 'selected actual build stream')
        raw = read(pins, pin, 32 << 20)
        rows = [D.parse(line) for line in raw.splitlines() if line.strip()]
        V.require(sum(row == selected[role]['artifact'] for row in rows) == 1
            and hashlib.sha256(raw).hexdigest() == value['phases'][phase]['stdout_sha256'],
            'actual selected Cargo artifact occurs once in successful stream')
    return value, selected


def cpu_evidence(pins, plan, previous_capture):
    import ordered_cpu
    return ordered_cpu.evidence(types.SimpleNamespace(**globals()), pins, plan, previous_capture)


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



def prefix_generation(pins, generation, predecessor):
    V.keys(generation, 'source target roles prerequisites compiler_products patches qualified_generation '
        'predecessor compiler_package finalizer_package backend_dynamic_library')
    V.require(generation['prerequisites'] == RPO_PREREQUISITES and generation['predecessor'] == predecessor,
              'actual RPO compiler has the exact ordinary V7 predecessor')
    cpu, owner, tools, tools_owner = [doc(pins, RPO_PREREQUISITES[key])
        for key in ('cpu', 'cpu_owner', 'tools', 'tools_owner')]
    schemas = ('fe2o3-p228-rpo-compiler-cpu-result-v1', 'fe2o3-p228-rpo-compiler-owned-result-v1',
        'fe2o3-p228-rpo-finalizer-tools-result-v1', 'fe2o3-p228-rpo-finalizer-tools-owned-result-v1')
    for value, schema in zip((cpu, owner, tools, tools_owner), schemas):
        V.require(value['schema'] == schema and value['passed'] is True and value['error'] is None
            and value['postcheck_errors'] == [] and value['gpu_execution'] is False
            and value['production_authority'] is False, 'actual successful CPU-only compiler qualification')
    for value, record in ((owner, RPO_PREREQUISITES['cpu']), (tools_owner, RPO_PREREQUISITES['tools'])):
        owned_success(value['owned'])
        V.require(value['completion'] == record, 'actual qualified owned completion')
    V.require(cpu['source_unchanged'] is True
        and cpu['qualified_generation']['completion'] == predecessor['prerequisites']['cpu']
        and cpu['qualified_generation']['owner'] == predecessor['prerequisites']['cpu_owner']
        and [(cpu['tests'][key]['passed'], cpu['tests'][key]['ignored']) for key in ('pliron', 'compiler')]
            == [(1504, 1), (1196, 24)]
        and (tools['tests']['passed'], tools['tests']['ignored']) == (190, 15),
        'actual RPO2700/25 and finalizer190/15 metadata, not new tool execution')
    for value in (tools, tools_owner):
        V.require(value['compiler_cpu'] == RPO_PREREQUISITES['cpu']
            and value['compiler_owner'] == RPO_PREREQUISITES['cpu_owner']
            and value['qualified_generation'] == cpu['qualified_generation']
            and value['patches']['rpo'] == cpu['patch'], 'one exact RPO source generation')
    V.require(tools['compiler_artifacts'] == cpu['artifacts']
        and tools['source_snapshot'] == cpu['raw']['sources-before.json']
        and tools['compiler_required_test_names'] == cpu['required_test_names'],
        'finalizer preserves actual compiler products and source/test roster')
    source_pins = (cpu['raw']['sources-before.json'], cpu['raw']['sources-after.json'],
                   tools['raw']['sources-after.json'])
    snapshots = [doc(pins, record) for record in source_pins]
    V.require(snapshots[0] and snapshots[0] == snapshots[1] == snapshots[2],
              'retained compiled source snapshots, without opening build-host source or targets')
    for record in (cpu['package'], tools['package'], cpu['patch']):
        doc(pins, record)
    V.require(set(cpu['artifacts']) == {'pliron-tests', 'compiler-tests', 'fe2o3-rustc-extract',
            'librustc_codegen_fe2o3.so', 'librustc_codegen_fe2o3.rlib'}
        and set(tools['artifacts']) == {'finalizer', 'finalizer-tests', 'metadata'},
        'eight qualified product identities')
    roles = {name: cpu['artifacts'][name]
        for name in ('compiler-tests', 'fe2o3-rustc-extract', 'librustc_codegen_fe2o3.so')}
    roles.update(tools['artifacts'])
    root = E / 'rpo-compiler-cpu-v228-v2'
    expected = dict(source=str(root / 'source/fe2o3'), target=str(root / 'target'), roles=roles,
        prerequisites=RPO_PREREQUISITES, compiler_products=cpu['artifacts'], patches=tools['patches'],
        qualified_generation=cpu['qualified_generation'], predecessor=predecessor,
        compiler_package=cpu['package'], finalizer_package=tools['package'],
        backend_dynamic_library=dict(cpu['artifacts']['librustc_codegen_fe2o3.so'],
            path=str(root / 'target/debug/deps/librustc_codegen_fe2o3.so')))
    V.require(generation == expected, 'exact selected compiler roles, packages, backend alias and source lineage')
    return generation


def prefix_cpu_contract(cpu):
    V.require(cpu['schema'] == 'ferric-p228-rope-materialized-cpu-result-v1'
        and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
        and cpu['source_unchanged'] is True and cpu['cpu_arithmetic_only'] is True
        and type(cpu['tests_passed']) is int and cpu['tests_passed'] == 33
        and type(cpu['tests_ignored']) is int and cpu['tests_ignored'] == 1
        and len(cpu['phases']) == 11 and len(cpu['raw']) == 61
        and set(cpu['tests']) == {'prefix_reciprocal_v1', 'rope_materialized_v1', 'exhaustive'},
        'actual isolated RoPE CPU33, including explicit exhaustive execution')
    for name, passed, ignored in (('prefix_reciprocal_v1', 12, 1), ('rope_materialized_v1', 20, 0), ('exhaustive', 1, 0)):
        row = cpu['tests'][name]
        V.require(type(row['passed']) is int and type(row['ignored']) is int
            and (row['passed'], row['ignored']) == (passed, ignored)
            and len(row['names']) == len(set(row['names'])) == passed + ignored, 'named RoPE CPU tests')
    V.require(cpu['tests']['exhaustive']['names'] == ['exhaustive_all_significands_in_one_binade'], 'explicit exhaustive case')
    for row in cpu['phases'].values():
        V.require(type(row['exit_code']) is int and row['exit_code'] == 0 and row['reason'] is None
            and row['group_absent'] is True, 'natural reaped prefix CPU phase')
    for key in ('kernel_entry_host_compiled', 'gpu_execution', 'compiler_hsaco_reproduced',
                'full_model_acceptance', 'numerical_acceptance', 'performance_claim', 'production_authority'):
        V.require(cpu[key] is False, 'CPU arithmetic-only qualification')


def prefix_path(record, label, filename):
    path = Path(record['path'])
    V.require(path.parent.parent == E and path.name == filename
        and re.fullmatch(re.escape(label) + r'[1-9][0-9]{0,8}', path.parent.name),
        'explicit actual prefix receipt/source namespace')
    return path


def linked_contract(value, owner, plan, producer, producer_owner, consumer, consumer_owner):
    V.require(value['schema'] == 'ferric-p228-rope-indexed-checked-emission-result-v1'
        and owner['schema'] == 'ferric-p228-rope-indexed-checked-emission-owned-result-v1'
        and owner['completion'] == plan['prefix_emission_complete'], 'linked continuation schemas and completion')
    for row in (value, owner):
        V.require(row['passed'] is True and row['error'] is None and row['postcheck_errors'] == []
            and row['fresh_hsaco_emitted'] is True and all(row[key] is False for key in EMISSION_FALSE),
            'successful emission is not fresh lowering, full compiler qualification, or GPU authority')
        V.require(row['producer'] == PRODUCER_PIN and row['producer_owner'] == PRODUCER_OWNER_PIN
            and row['consumer'] == CONSUMER_PIN and row['consumer_owner'] == CONSUMER_OWNER_PIN
            and row['package'] == value['package'], 'separate exact producer and consumer lineage')
    owned_success(owner['owned'])
    V.require(value['package']['path'] == str(E / 'p228-rope-indexed-checked-emission-v1/manifest.json')
        and value['package']['sha256'] == EMISSION_PACKAGE_SHA, 'reviewed linked-emission controller package')
    V.require(producer['schema'] == 'ferric-p228-rope-materialized-rpo-lowering-result-v1'
        and producer_owner['schema'] == 'ferric-p228-rope-materialized-rpo-lowering-owned-result-v1'
        and producer['passed'] is producer_owner['passed'] is False
        and producer['postcheck_errors'] == producer_owner['postcheck_errors'] == []
        and producer['error'] == 'AssertionError: actual-inert-join'
        and producer_owner['completion'] is None
        and all(producer[key] is False for key in ('fresh_checked_lowering', 'fresh_checked_replay', 'fresh_hsaco_emitted')),
        'historical aggregate remains failed')
    failed_owner = producer_owner['owned']
    V.require(type(failed_owner['exit_code']) is int and failed_owner['exit_code'] == 1
        and failed_owner['reason'] is None and failed_owner['cleanup_signalled'] is False
        and failed_owner['owned_groups_absent'] is failed_owner['owned_processes_reaped'] is True,
        'historical producer owner failed naturally and was reaped')
    V.require(tuple(row['name'] for row in producer['commands']) == PRODUCER_STAGES
        and value['producer_commands'] == producer['commands']
        and value['producer_captures'] == producer['artifacts']
        and value['producer_generation'] == producer['compiler_generation'] == producer_owner['compiler_generation']
        and producer['candidate_cpu'] == producer_owner['candidate_cpu'] == plan['prefix_cpu'],
        'retain original three checked producer stages and arithmetic-source qualification')
    V.require(consumer['schema'] == 'ferric-p228-kir-indexed-formal-join-cpu-result-v2'
        and consumer_owner['schema'] == 'ferric-p228-kir-indexed-formal-join-owned-result-v2'
        and consumer_owner['completion'] == CONSUMER_PIN, 'actual indexed V2 qualification, not V1 attempt')
    for row in (consumer, consumer_owner):
        V.require(row['passed'] is True and row['error'] is None and row['postcheck_errors'] == []
            and row['actual_capture_join_passed'] is True and row['fresh_hsaco_emitted'] is False,
            'consumer qualification is successful without prior emission')
    owned_success(consumer_owner['owned'])
    V.require(consumer['source_unchanged'] is True and consumer['limits_changed'] is False
        and consumer['fresh_compiler_built'] is consumer['full_compiler_cohort_requalified'] is False
        and consumer['compiler_cpu'] == RPO_PREREQUISITES['cpu']
        and consumer['raw']['sources-before.json']['sha256'] == CONSUMER_SOURCE_SHA
        and content(consumer['raw']['sources-before.json']) == content(consumer['raw']['sources-after.json'])
        and value['consumer_source'] == consumer['raw']['sources-before.json']
        and value['consumer_proposal'] == consumer['proposal']
        and value['consumer_tests'] == consumer['tests']
        and value['preserved_consumer_test_artifacts'] == consumer['artifacts'],
        'exact tested consumer source and artifact metadata, not substituted producer tools')
    for name, passed, ignored in (('lower', 785, 0), ('indexed_subset_repeat', 20, 0), ('finalizer', 190, 15)):
        V.require(consumer['tests'][name]['passed'] == passed and consumer['tests'][name]['ignored'] == ignored,
                  'actual distinct consumer test cohorts')
    V.require(value['retained_handoff'] == producer['artifacts']['prefix-tiles.handoff-v3']
        == consumer['retained_handoff'] == HANDOFF
        and content(value['artifacts']['emitted/source.handoff-v3']) == content(HANDOFF),
        'identical retained producer handoff enters the new consumer')
    V.require(value['retained_checked_producer_replay_authenticated'] is True
        and value['producer_consumer_generations_distinct'] is True and value['source_unchanged'] is True
        and value['fresh_consumer_tools_built'] is value['fresh_actual_inert_join_passed'] is True
        and value['unresolved_runtime_requirements'] == 8
        and tuple(row['name'] for row in value['commands']) == EMISSION_STAGES
        and set(value['phases']) == set(EMISSION_STAGES) and set(value['tools']) == {'finalizer', 'metadata'}
        and set(value['artifacts']) == EMISSION_ARTIFACTS
        and value['actual_join'] == consumer['actual_join'], 'complete eight-stage continuation with the full retained join')
    for role, name in (('finalizer', 'finite_join_engineering_hsaco_v1'), ('metadata', 'finite_join_request_metadata_v1')):
        V.require(value['tools'][role]['path'] == str(CONSUMER / 'target/debug/examples' / name),
                  'actual freshly built consumer tool location')
    V.require(content(plan['prefix_image']) == content(value['artifacts']['emitted/artifact.hsaco']),
              'transported image is exactly the actual emitted image')


def prefix_leaf(pins, row, directory, phase=None):
    V.keys(row, 'name command started result stdout stderr')
    for key in ('command', 'started', 'result', 'stdout', 'stderr'):
        suffix = key + '.json' if key in ('command', 'started', 'result') else key
        V.require(row[key]['path'] == str(directory / (row['name'] + '-' + suffix)), 'closed retained stage path')
        read(pins, row[key], 32 << 20)
    result, command = doc(pins, row['result']), doc(pins, row['command'])
    V.require(type(result['exit_code']) is int and result['exit_code'] == 0 and result['reason'] is None
        and result['group_absent'] is True and (phase is None or result == phase)
        and all(result[key + '_sha256'] == row[key]['sha256'] for key in ('stdout', 'stderr')),
        'natural successful stage with authenticated streams')
    V.require(command['expected_exit'] == 0 and command['gpu_execution'] is False
        and command['affinity'] == [8, 9] and command['nice'] == 10
        and command['cache_cap_bytes'] == 6 << 30
        and all(command['env'][key] == '' for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'))
        and 'FE2O3_KIR_JOIN_WORK_DIAGNOSTIC_V1' not in command['env'], 'bounded GPU-hidden actual stage')
    return command


def linked_command(template, substitutions, value):
    result = copy.deepcopy(template)
    name = result.pop('name'); result.pop('cwd')
    replacements = {str(PRODUCER / key): str(EMISSION_DIR / key) for key in ('emitted', 'extracted')}
    result['argv'] = [next((new + item[len(old):] for old, new in replacements.items()
        if item == old or item.startswith(old + '/')), item) for item in result['argv']]
    role = {'actual-inert-join': 'finalizer-tests', 'emit': 'finalizer', 'descriptor-metadata': 'metadata'}.get(name)
    if role:
        roles = dict(value['tools'], **{'finalizer-tests': value['preserved_consumer_test_artifacts']['finalizer-tests']})
        result['argv'][0] = roles[role]['path']
    result['env']['CARGO_TARGET_DIR'] = str(CONSUMER / 'target')
    result['env']['TMPDIR'] = str(EMISSION_DIR / 'tmp')
    libraries = result['env']['LD_LIBRARY_PATH'].split(':')
    V.require(len(libraries) == 2, 'original qualified two-part loader path')
    result['env']['LD_LIBRARY_PATH'] = str(CONSUMER / 'target/debug/deps') + ':' + libraries[1]
    return substitute_record(result, substitutions)


def substitute_record(value, substitutions):
    if isinstance(value, str):
        return str(substitutions[value]) if value in substitutions else value
    if isinstance(value, list):
        return [substitute_record(item, substitutions) for item in value]
    if isinstance(value, dict):
        return {key: substitute_record(item, substitutions) for key, item in value.items()}
    return value


def consumer_builds(raw, tools):
    names = {'finalizer': 'finite_join_engineering_hsaco_v1', 'metadata': 'finite_join_request_metadata_v1'}
    observed = {role: set() for role in names}
    finished = []
    for line in raw.decode('utf-8').splitlines():
        row = D.parse(line.encode())
        if row.get('reason') == 'build-finished':
            finished.append(row['success'])
        if row.get('reason') != 'compiler-artifact':
            continue
        for role, name in names.items():
            if row['target']['name'] == name:
                V.require(row['target']['kind'] == ['example'] and row['profile']['test'] is False
                    and 'fe2o3-hsaco-finalize' in row['package_id'], 'actual consumer example, not a test executable')
                observed[role].add(row['executable'])
    V.require(finished == [True] and all(observed[role] == {tools[role]['path']} for role in names),
              'both emitted consumer tool identities came from the actual Cargo stream')


def prefix_evidence(pins, plan, prior):
    import prefix_contracts as C
    V.require(plan['prefix_emission_complete'] == EMISSION_PIN
        and plan['prefix_emission_owner'] == EMISSION_OWNER_PIN
        and content(plan['prefix_image']) == PREFIX_IMAGE,
        'exact retained successful linked continuation, owner, and image')
    V.require(Path(plan['prefix_image']['path']).is_relative_to(E), 'transported image under session evidence')
    cpu, value, owner = (doc(pins, plan[key]) for key in ('prefix_cpu', 'prefix_emission_complete', 'prefix_emission_owner'))
    producer, producer_owner = doc(pins, PRODUCER_PIN), doc(pins, PRODUCER_OWNER_PIN)
    consumer, consumer_owner = doc(pins, CONSUMER_PIN), doc(pins, CONSUMER_OWNER_PIN)
    prefix_cpu_contract(cpu)
    linked_contract(value, owner, plan, producer, producer_owner, consumer, consumer_owner)
    generation = prefix_generation(pins, value['producer_generation'], prior['standalone']['provenance']['compiler_generation'])
    V.require(cpu['runner']['path'] == str(E / 'p228-rope-materialized-cpu-v2/run.py')
        and cpu['overlay'] == producer['source_manifest']
        and cpu['overlay']['path'] == str(E / 'p228-rope-materialized-source-v2/source-manifest.json')
        and producer['package_manifest'] == producer_owner['package_manifest'] == PREFIX_PACKAGE
        and producer['prior_lowering'] == cpu['prior_lowering'] == prior['standalone']['provenance']['compiler_complete']
        and cpu['prior_cpu'] == prior['standalone']['provenance']['candidate_cpu_receipt'],
        'unchanged V2 arithmetic source, original checked producer, and V7 predecessor')
    V.require(cpu['fixture'] == str(Path(plan['prefix_cpu']['path']).parent / 'fixture'), 'isolated tested CPU fixture')
    read(pins, cpu['runner'])
    V.require(doc(pins, cpu['overlay'])['schema'] == 'ferric-p228-rope-materialized-source-proposal-v2', 'exact source family')
    source = doc(pins, cpu['raw']['sources-before.json'])
    V.require(source == doc(pins, cpu['raw']['sources-after.json']) and len(source['fixture']) == 15
        and set(cpu['formatted_sources']) == PREFIX_CHANGED and set(cpu['lowering_sources']) == PREFIX_RUST,
        'exact tested fifteen-file arithmetic fixture and eight lowered Rust bodies')
    for record in [*cpu['formatted_sources'].values(), *cpu['lowering_sources'].values()]:
        relative = str(Path(record['path']).relative_to(Path(cpu['fixture'])))
        V.require(content(record) == (source['fixture'][relative]['bytes'], source['fixture'][relative]['sha256']), 'tested arithmetic body')
        read(pins, record, 4 << 20)
    V.require(doc(pins, PREFIX_PACKAGE)['schema'] == 'ferric-p228-rope-materialized-rpo-lowering-package-v1', 'producer package')
    package = doc(pins, value['package'])
    V.require(package['schema'] == 'ferric-p228-rope-indexed-checked-emission-package-v1'
        and len(package['files']) == 4 and {row['path'] for row in package['files']} == {'run.py', 'test_run.py', 'contracts.py', 'README.md'},
        'exact reviewed continuation source package')
    for row in package['files']:
        read(pins, dict(row, path=str(Path(value['package']['path']).parent / row['path'])))
    consumer_source = doc(pins, value['consumer_source'])
    V.require(consumer_source == doc(pins, consumer['raw']['sources-after.json']) and len(consumer_source) == 5785
        and len(consumer['formatted_sources']) == 5, 'actual qualified consumer source snapshot')
    for relative, record in consumer['formatted_sources'].items():
        V.require(record['path'] == str(CONSUMER / 'source/fe2o3' / relative)
            and record == consumer_source[record['path']]['pin'], 'formatted consumer source identity')
        read(pins, record, 4 << 20)
    doc(pins, consumer['package']); doc(pins, consumer['proposal'])
    recipe = doc(pins, value['producer_recipe'])
    V.require(value['producer_recipe']['path'] == str(PRODUCER / 'recipe.json')
        and recipe['schema'] == 'ferric-p228-rope-materialized-rpo-lowering-recipe-v1'
        and recipe['compiler_generation'] == generation['prerequisites']
        and tuple(row['name'] for row in recipe['commands']) == PREFIX_STAGES
        and len(recipe['fixture']) == 10 and {row['destination'] for row in recipe['fixture']} == PREFIX_RUST | {'Cargo.toml', 'Cargo.lock'},
        'original nine-stage recipe is provenance, not nine new successful stages')
    for row in recipe['fixture']:
        if row['destination'] in PREFIX_RUST:
            V.require(row['source'] == cpu['lowering_sources'][row['destination']], 'producer uses the tested arithmetic source')
        read(pins, dict(row['source'], path=str(PRODUCER / 'fixture' / row['destination'])), 4 << 20)
    substitutions = {'@candidate.semantic.sha256': producer['artifacts']['prefix-tiles-semantic.bin']['sha256'],
        '@candidate.handoff.sha256': HANDOFF['sha256'], '@candidate.handoff.bytes': str(HANDOFF['bytes']),
        '@candidate.image.sha256': plan['prefix_image']['sha256'], '@candidate.image.bytes': str(plan['prefix_image']['bytes'])}
    for template, row in zip(recipe['commands'][:3], producer['commands']):
        command = prefix_leaf(pins, row, PRODUCER)
        expected = substitute_record({key: item for key, item in template.items() if key not in ('name', 'cwd')}, substitutions)
        V.require(command == expected, 'actual preserved checked producer recipe')
    templates = {row['name']: row for row in recipe['commands']}
    prior_build = doc(pins, consumer['raw']['finalizer-build-tests-command.json'])
    build_env = dict(prior_build['env'], TMPDIR=str(EMISSION_DIR / 'tmp'))
    cargo = prior_build['argv'][0]
    descriptor = None
    for row in value['commands']:
        name = row['name']; command = prefix_leaf(pins, row, EMISSION_DIR, value['phases'][name])
        if name in PREFIX_STAGES[3:]:
            V.require(command == linked_command(templates[name], substitutions, value), 'only explicit linked consumer substitutions')
        if name in ('metadata', 'tool-build'):
            V.require(command['env'] == build_env and command['tools'] == prior_build['tools'], 'same qualified consumer build environment')
            prefix = [cargo, 'metadata', '--offline', '--locked'] if name == 'metadata' else [cargo, 'build', '--offline', '--locked', '--jobs', '2']
            suffix = ['--format-version', '1'] if name == 'metadata' else [
                '-p', 'fe2o3-hsaco-finalize', '--example', 'finite_join_engineering_hsaco_v1',
                '--example', 'finite_join_request_metadata_v1', '--message-format=json']
            V.require(command['argv'] == prefix + ['--manifest-path', str(CONSUMER / 'source/fe2o3/Cargo.toml')] + suffix
                and command['deadline_seconds'] == (120 if name == 'metadata' else 1800), 'only the two actual consumer tool build stages')
            if name == 'metadata':
                V.require(D.parse(read(pins, row['stdout'])) == doc(pins, consumer['raw']['metadata-stdout']), 'same qualified locked Cargo metadata')
            else:
                consumer_builds(read(pins, row['stdout']), value['tools'])
        elif name == 'actual-inert-join':
            C.exact_test(read(pins, row['stdout']), command['argv'][2])
        elif name == 'emit':
            worker = next(pin for pin in recipe['toolchain_and_retained_tool_pins'] if pin['path'] == command['argv'][5])
            config = {Path(pin['path']).name: pin['sha256'] for pin in recipe['toolchain_and_retained_tool_pins']
                      if '/gfx950-reviewed-libs/' in pin['path']}
            emitted = {Path(name).name: pin for name, pin in value['artifacts'].items() if name.startswith('emitted/')}
            C.emission(read(pins, emitted['receipt.txt']), emitted, worker, command['argv'][8], command['argv'][9], config)
        elif name == 'descriptor-metadata':
            descriptor = C.metadata(read(pins, row['stdout']), plan['prefix_image'])
        elif name == 'elf-notes':
            V.require(all(marker in read(pins, row['stdout']) for marker in (b'.group_segment_fixed_size: 512',
                b'.private_segment_fixed_size: 0', b'.wavefront_size: 64')), 'unchanged prefix launch resources')
    for records, directory in ((producer['artifacts'], PRODUCER), (value['artifacts'], EMISSION_DIR)):
        for name, record in records.items():
            V.require(record['path'] == str(directory / name), 'exact original artifact path')
            read(pins, record, 32 << 20)
    owner_result_pin, _ = pins.read(EMISSION_OWNER_DIR / 'owned-result.json')
    V.require(doc(pins, owner_result_pin) == owner['owned'], 'actual independently retained continuation owner outcome')
    read(pins, plan['prefix_image'], 32 << 20, False)
    return dict(cpu=plan['prefix_cpu'], emission=plan['prefix_emission_complete'], owner=plan['prefix_emission_owner'],
        source_manifest=cpu['overlay'], original_image=value['artifacts']['emitted/artifact.hsaco'], descriptor=descriptor,
        producer=PRODUCER_PIN, producer_owner=PRODUCER_OWNER_PIN, producer_generation=generation,
        producer_recipe=value['producer_recipe'], retained_handoff=HANDOFF,
        consumer=CONSUMER_PIN, consumer_owner=CONSUMER_OWNER_PIN, consumer_source=value['consumer_source'],
        consumer_proposal=value['consumer_proposal'], consumer_tools=value['tools'],
        compiler_binary_bodies_replayed=False, consumer_binary_bodies_replayed=False,
        fresh_checked_lowering=False, fresh_checked_replay=False, full_compiler_cohort_requalified=False,
        numerical_acceptance=False, runtime_requirements_discharged=False,
        native_table_generation_unchanged=True, table_generation_equality=False)


def request_check(pins, value, prior, runtime, out, image, mlp, route='default'):
    V.keys(value, 'schema decode projection_residual_image')
    V.require(route in ('default', 'shared', 'ordered') and value['schema'] ==
        ('FerricFiniteProjectionResidualMlpOrderedRequestV1' if route == 'ordered' else
         'FerricFiniteProjectionResidualDecodeRequestV1'), 'distinct selected AR4 request')
    decode, previous, L = value['decode'], prior['request'], prior['L']
    V.keys(decode, 'schema source worker images expected_bundle_id expected_model_id device_ids session prompt '
        'prefix_image tiles_image evidence_directory mode dispatch_timeout_ms child_deadline_ms')
    V.require(decode['schema'] == previous['schema'] == 'FerricFinitePrefixDecodeRequestV1'
        and decode['mode'] == 'autoregressive' and previous['mode'] == 'teacher_forced',
        'AR4 mode changes only recurrence from the authentic TF4 provenance')
    for key in ('source', 'images', 'expected_bundle_id', 'expected_model_id', 'device_ids', 'prompt',
                'dispatch_timeout_ms', 'child_deadline_ms'):
        V.require(decode[key] == previous[key], 'unchanged genuine model/prompt input: ' + key)
    V.require(L.rust_pin(decode['worker']) == runtime['worker']
        and L.rust_pin(value['projection_residual_image']) == image
        and L.rust_pin(decode['prefix_image']) == runtime['image']
        and content(runtime['image']) != content(L.rust_pin(previous['prefix_image']))
        and L.rust_pin(decode['tiles_image']) == mlp and content(mlp) == SILU_IMAGE
        and content(mlp) != content(prior['plan']['down2_image']), 'new checked prefix with unchanged qualified SiLU and projection images')
    V.require(V.digest(decode['session']) != bytes(32) and decode['session'] != previous['session']
        and decode['evidence_directory'] == str(out / 'native'), 'fresh session and case output')
    V.require(V.uint(decode['dispatch_timeout_ms']) == 10000 and V.uint(decode['child_deadline_ms']) == 3600000,
              'unchanged dispatch and one-hour child bounds')
    for pin in [*decode['images'].values(), decode['prefix_image'], decode['tiles_image'],
                value['projection_residual_image']]:
        read(pins, L.rust_pin(pin), 32 << 20, False)
    for pin in decode['prompt'].values():
        read(pins, L.rust_pin(pin), 32 << 10, False)


def engineering_review(value, plan, runtime, prior, image, mlp, prefix):
    V.keys(value, 'schema reviewed authority gpu_attempts output_label image down2_image '
        'image_provenance down2_provenance projection_provenance mlp_provenance prefix_provenance review_topics notes '
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
        and value['projection_provenance'] == image and value['mlp_provenance'] == mlp
        and value['prefix_provenance'] == prefix and value['image'] == plan['prefix_image'],
        'reviewed RPO prefix and original V7 predecessor, unchanged SiLU/residual lineage')
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
    capture = baseline(pins, plan)
    prior = capture['prior']
    cpu, selected = cpu_evidence(pins, plan, capture)
    prefix = prefix_evidence(pins, plan, prior)
    runtime = dict(parent=deployed_binary(pins, plan['parent'], selected['parent']['binary']),
        worker=deployed_binary(pins, plan['worker'], selected['worker']['binary']), image=plan['prefix_image'])
    image = image_evidence(pins, plan)
    V.require(image == capture['image_provenance'] and plan['projection_image'] == capture['plan']['projection_image'],
              'same already-inspected additive image, no implicit replacement')
    request = doc(pins, plan['request'], 64 << 10)
    request_check(pins, request, prior, runtime, out, plan['projection_image'], plan['mlp_image'], plan['route'])
    P, standalone = prior['P'], prior['standalone']
    reviews = {}
    for role in ('parent', 'worker'):
        reviews[role] = doc(pins, plan[role + '_runtime_review'])
        P.runtime_review(reviews[role], runtime[role], standalone['platform'], pins)
    engineering_review(doc(pins, plan['decode_review'], 192 << 10), plan, runtime, prior, image, capture['mlp_provenance'], prefix)
    c = dict(pins=pins, plan=plan, plan_pin=plan_pin, request=request, out=out, runtime=runtime, route=plan['route'],
        standalone=standalone, P=P, O=prior['O'], platform=standalone['platform'], topology=standalone['topology'],
        owned=standalone['owned'], environment=dict(P.ENV), runtime_reviews=reviews,
        supervisor_manifest=manifest_pin, baseline=capture, parent_cpu=cpu, worker_cpu=cpu,
        prefix_provenance=prefix)
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
