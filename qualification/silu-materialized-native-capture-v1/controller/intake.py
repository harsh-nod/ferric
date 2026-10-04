"""SiLU image admission with unchanged qualified executables and native capture protocol."""
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
BASELINE_PACKAGE = ('p228-projection-residual-decode-gpu-v1',
    '6dfd492f7dc8da8bd8eaa2f8e9da9bcbe36cd94756ba321bda538b3abb6fbb35')
BASELINE = (955163, '4f25030567c470062fd50862876bc35e93d080f1778c0be4ca36f2b39af4199a')
# Completed CPU generation; future lowering/owner/image are explicit root-pinned plan inputs.
MLP_CPU = (84947, 'e139729d8fa95715ef8017679ee21f1175ba302b681785789e3dc84137508a30')
INPUT_SCHEMA = 'ferric-p228-silu-materialized-capture-inputs-v1'
REVIEW_SCHEMA = 'ferric-p228-silu-materialized-capture-engineering-review-v1'
PACKAGE_SCHEMA = 'ferric-p228-silu-materialized-capture-gpu-package-v1'
PURE_SCHEMA = 'ferric-p228-silu-materialized-capture-gpu-pure-v1'
PLAN_FIELDS = ('schema output_label baseline baseline_capture parent_cpu worker_cpu parent worker request '
    'projection_image lowering_complete inspection_complete mlp_image mlp_cpu mlp_lowering_complete '
    'mlp_lowering_owner capture_review parent_runtime_review worker_runtime_review '
    'supervisor_tests supervisor_test_sources')
REVIEW_BINDINGS = [key for key in PLAN_FIELDS.split()
    if key not in ('schema', 'output_label', 'capture_review', 'supervisor_tests', 'supervisor_test_sources')]
REVIEW_FALSE = ('production_authority', 'full_model_acceptance', 'independent_numerical_acceptance',
    'numerical_acceptance', 'arithmetic_prerequisites_verified', 'runtime_premises_discharged', 'performance_claim',
    'timestamp_calibration', 'clock_domain_validated', 'cross_device_clock_alignment', 'overlap_claim',
    'paired_comparison_performed', 'full_forward', 'conditional_residual_checks_performed')
TOPICS = ('source_lineage', 'formal', 'isa', 'coherence', 'lifecycle', 'selected_device')
PACKAGE_FILES = {'INTAKE.md', 'README.md', 'capture_validation.py', 'intake.py', 'layer_validation.py',
    'run.py', 'run_row_facts_v2.py', 'test_capture_validation.py', 'test_intake.py', 'test_run.py',
    'mlp_contracts.py'}
PURE_TESTS = 36
TEST_RUNNER_SHA = '13c47f9f53496c88357aa05052b5429fb0b9bf2ee0501dfcc4dfd7f89e301e84'
STAGES = ('fixture-metadata', 'checked-lowering', 'actual-replay', 'actual-inert-join', 'emit',
          'extract-retained', 'descriptor-metadata', 'elf-notes', 'disassembly')
TARGET_COUNTS = {'mlp_tiles_numerics_v2': 4, 'mlp_down_two_row_v1': 10,
                 'mlp_claimed_numerics_v1': 8, 'mlp_silu_materialized_v1': 16}

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
        and re.fullmatch(r'prefix-silu-materialized-capture-gpu-v228-v[1-9][0-9]{0,8}', plan['output_label']),
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
        and re.fullmatch(r'silu-materialized-capture-gpu-pure-v228-v[1-9][0-9]{0,8}', directory.name), 'new pure namespace')
    for key, filename in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'),
                          ('transcript', 'tests.log')):
        V.require(value[key]['path'] == str(directory / filename), 'actual retained pure output path')
        read(pins, value[key])
    before, after = doc(pins, value['sources_before']), doc(pins, value['sources_after'])
    expected = {row['path']: dict(row, path=str(Path(__file__).resolve().parent / row['path']))
                for row in manifest['files']}
    V.require(before == after == expected, 'exact tested package before and after')
    pins.pin(E / 'run_silu_materialized_capture_gpu_pure_p228_v1.py', TEST_RUNNER_SHA)



def content(value):
    return value['bytes'], value['sha256']



def guard(c):
    c['pins'].recheck(); c['P'].guard(c['standalone'])
    for review in c['runtime_reviews'].values():
        for row in review['libraries']:
            V.require(Path(row['path']).resolve(strict=True) == Path(row['resolved']['path']),
                      'runtime library alias unchanged')


def actual_tuple(value, name):
    V.require(type(value) is tuple and len(value) == 2 and type(value[0]) is int and value[0] > 0
        and type(value[1]) is str and re.fullmatch('[0-9a-f]{64}', value[1]),
        'root must bind actual completed ' + name)


def dependencies(pins):
    base = D.package(pins, *BASELINE_PACKAGE)
    manifest, _ = pins.json(base / 'manifest.json', BASELINE_PACKAGE[1])
    hashes = {row['path']: row['sha256'] for row in manifest['files']}
    Q = D.load_module(pins, base / 'intake.py', hashes['intake.py'], 'silu_corrected_capture_replay')
    B, _, _, _ = Q.previous(pins)
    return Q, B


def selected_runtime(pins, plan, capture, B):
    for key in ('baseline', 'parent_cpu', 'worker_cpu', 'projection_image',
                'lowering_complete', 'inspection_complete'):
        V.require(plan[key] == capture['plan'][key], 'unchanged qualified baseline input: ' + key)
    selected = B.qualified_cpu(capture['cpu'])
    runtime = dict(image=capture['receipt']['selected_runtime']['image'])
    for role in ('parent', 'worker'):
        runtime[role] = B.deployed_binary(pins, plan[role], selected[role]['binary'])
        V.require(content(runtime[role]) == content(capture['receipt']['selected_runtime'][role]),
                  'no new executable or Rust route')
    return runtime


def request_shape(value, previous, runtime, out, image, rust_pin):
    V.keys(value, 'schema layer projection_residual_image')
    V.require(value['schema'] == previous['schema'] == 'FerricFiniteProjectionResidualLayerCaptureRequestV1'
        and value['projection_residual_image'] == previous['projection_residual_image'],
        'unchanged corrected residual route and image')
    layer, old = value['layer'], previous['layer']
    V.keys(layer, 'schema source worker images expected_bundle_id expected_model_id device_ids session prompt '
        'prefix_tiles_image mlp_tiles_image evidence_directory dispatch_timeout_ms child_deadline_ms')
    for key in set(layer) - {'worker', 'session', 'mlp_tiles_image', 'evidence_directory'}:
        V.require(layer[key] == old[key], 'unchanged genuine layer input: ' + key)
    V.require(rust_pin(layer['worker']) == runtime['worker']
        and rust_pin(layer['prefix_tiles_image']) == runtime['image']
        and rust_pin(layer['mlp_tiles_image']) == image
        and content(image) != content(rust_pin(old['mlp_tiles_image'])), 'only new selected MLP image')
    V.require(V.digest(layer['session']) != bytes(32) and layer['session'] != old['session']
        and layer['evidence_directory'] == str(out / 'native'), 'fresh session/output')
    V.require(V.uint(layer['dispatch_timeout_ms']) == 10000
        and V.uint(layer['child_deadline_ms']) == 3600000, 'unchanged native deadlines')


def request_check(pins, value, capture, runtime, out, image):
    previous = doc(pins, capture['plan']['request'], 64 << 10)
    L = capture['prior']['L']
    request_shape(value, previous, runtime, out, image, L.rust_pin)
    for pin in [*value['layer']['images'].values(), value['layer']['prefix_tiles_image'],
                value['layer']['mlp_tiles_image'], value['projection_residual_image']]:
        read(pins, L.rust_pin(pin), 32 << 20, False)
    for pin in value['layer']['prompt'].values():
        read(pins, L.rust_pin(pin), 32 << 10, False)


def cpu_contract(cpu):
    V.require(cpu['schema'] == 'ferric-p228-silu-materialized-cpu-result-v1'
        and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
        and cpu['source_unchanged'] is True and cpu['cpu_arithmetic_only'] is True
        and cpu['tests_passed'] == 38 and cpu['tests_ignored'] == 0
        and len(cpu['phases']) == 16 and set(cpu['tests']) == set(TARGET_COUNTS), 'actual SiLU CPU38')
    for name, count in TARGET_COUNTS.items():
        row = cpu['tests'][name]
        V.require(row['passed'] == count and row['ignored'] == 0
            and len(row['names']) == len(set(row['names'])) == count, 'actual named CPU cohort')
    for row in cpu['phases'].values():
        V.require(type(row['exit_code']) is int and row['exit_code'] == 0 and row['reason'] is None
            and row['group_absent'] is True, 'natural reaped CPU leaves')
    for key in ('gpu_execution', 'compiler_hsaco_reproduced', 'full_model_acceptance',
                'numerical_acceptance', 'performance_claim', 'production_authority'):
        V.require(cpu[key] is False, 'CPU does not authorize native arithmetic')


def lowering_contract(lower, owner, plan, cpu, prior):
    V.require(lower['schema'] == 'ferric-p228-silu-materialized-lowering-result-v1'
        and owner['schema'] == 'ferric-p228-silu-materialized-lowering-owned-result-v1',
        'separate checked SiLU lowering')
    for value in (lower, owner):
        V.require(value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
            and value['candidate_cpu'] == plan['mlp_cpu'] and value['source_manifest'] == cpu['overlay']
            and value['prior_lowering'] == cpu['prior_lowering'] == prior['down2_provenance']['lowering'],
            'actual CPU/source/Down2 lowering lineage')
        for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority'):
            V.require(value[key] is False, 'no compiler-granted runtime authority')
    for key in ('compiler_generation', 'package_manifest'):
        V.require(lower[key] == owner[key], 'same lowering owner generation')
    V.require(lower['compiler_generation'] == prior['down2_provenance']['compiler_generation'],
              'unchanged qualified compiler generation')
    V.require(owner['completion'] == plan['mlp_lowering_complete']
        and tuple(row['name'] for row in lower['commands']) == STAGES and len(lower['artifacts']) == 10
        and lower['fresh_checked_lowering'] is True and lower['fresh_checked_replay'] is True
        and lower['fresh_hsaco_emitted'] is True and lower['frontend_recipe_is_diagnostic'] is True
        and lower['unresolved_runtime_requirements'] == 8, 'all nine actual checked stages')
    owned_success(owner['owned'])
    V.require(content(lower['artifacts']['emitted/artifact.hsaco']) == content(plan['mlp_image']),
              'actual emitted SiLU image content')


def mlp_evidence(pins, plan, prior):
    import mlp_contracts as M
    actual_tuple(MLP_CPU, 'mlp_cpu')
    V.require(content(plan['mlp_cpu']) == MLP_CPU, 'actual root-bound SiLU CPU')
    for key, label in (('mlp_lowering_complete', 'row-silu-materialized-checked-probe-v228-v'),
                       ('mlp_lowering_owner', 'silu-materialized-checked-probe-owner-v228-v')):
        path = Path(plan[key]['path'])
        V.require(path.parent.parent == E and path.name == 'complete.json'
            and re.fullmatch(re.escape(label) + r'[1-9][0-9]{0,8}', path.parent.name),
            'explicit actual SiLU completion namespace')
    V.require(Path(plan['mlp_image']['path']).is_relative_to(E), 'explicit transported SiLU image')
    cpu, lower, owner = (doc(pins, plan[key]) for key in
                        ('mlp_cpu', 'mlp_lowering_complete', 'mlp_lowering_owner'))
    cpu_contract(cpu); lowering_contract(lower, owner, plan, cpu, prior)
    V.require(cpu['prior_cpu'] == prior['down2_provenance']['candidate_cpu'], 'unchanged CPU14 predecessor')
    read(pins, cpu['runner'])
    source = doc(pins, cpu['raw']['sources-before.json'])
    V.require(source == doc(pins, cpu['raw']['sources-after.json']), 'CPU source before/after equality')
    V.require(len(cpu['formatted_sources']) == 3 and len(cpu['lowering_sources']) == 5, 'exact CPU source bodies')
    for record in [*cpu['formatted_sources'].values(), *cpu['lowering_sources'].values()]:
        rel = str(Path(record['path']).relative_to(Path(cpu['fixture'])))
        V.require(content(record) == (source['fixture'][rel]['bytes'], source['fixture'][rel]['sha256']),
                  'compiled and lowering source identity')
        read(pins, record, 4 << 20)
    read(pins, cpu['overlay'])
    for key in ('source_manifest', 'package_manifest'):
        read(pins, lower[key])
    # Replay actual phase result/stream pins; large source maps stay under the authenticated owner receipt.
    descriptor = None
    for row in lower['commands']:
        for key in ('command', 'started', 'result', 'stdout', 'stderr'):
            read(pins, row[key], 32 << 20)
        result = doc(pins, row['result'])
        V.require(type(result['exit_code']) is int and result['exit_code'] == 0
            and result['reason'] is None and result['group_absent'] is True
            and all(result[k + '_sha256'] == row[k]['sha256'] for k in ('stdout', 'stderr')),
            'natural exact lowering command streams')
        if row['name'] == 'descriptor-metadata':
            descriptor = M.metadata(read(pins, row['stdout']), plan['mlp_image'])
        elif row['name'] == 'elf-notes':
            raw = read(pins, row['stdout'])
            V.require(all(marker in raw for marker in (b'.group_segment_fixed_size: 512',
                b'.private_segment_fixed_size: 0', b'.wavefront_size: 64')), 'unchanged MLP launch resources')
    for key in ('command.json', 'started.json', 'owned-result.json', 'stdout', 'stderr'):
        read(pins, owner['raw'][key])
    V.require(doc(pins, owner['raw']['owned-result.json']) == owner['owned'], 'actual owner result body')
    for record in lower['artifacts'].values():
        read(pins, record, 32 << 20)
    read(pins, plan['mlp_image'], 32 << 20, False)
    return dict(cpu=plan['mlp_cpu'], lowering=plan['mlp_lowering_complete'], owner=plan['mlp_lowering_owner'],
        source_manifest=cpu['overlay'], original_image=lower['artifacts']['emitted/artifact.hsaco'],
        descriptor=descriptor, numerical_acceptance=False, runtime_requirements_discharged=False)


def engineering_review(value, plan, runtime, capture, mlp):
    V.keys(value, 'schema reviewed authority gpu_attempts output_label image image_provenance '
        'projection_provenance prior_down2_provenance mlp_provenance review_topics notes '
        + ' '.join(REVIEW_BINDINGS) + ' ' + ' '.join(REVIEW_FALSE))
    V.require(value['schema'] == REVIEW_SCHEMA and value['reviewed'] is True and value['authority'] == 'none'
        and type(value['gpu_attempts']) is int and value['gpu_attempts'] == 1
        and value['output_label'] == plan['output_label'] and all(value[k] is False for k in REVIEW_FALSE),
        'explicitly root-reviewed single conditional engineering attempt')
    for key in REVIEW_BINDINGS:
        V.require(value[key] == plan[key], 'root review exact input: ' + key)
    V.require(value['image'] == runtime['image']
        and value['image_provenance'] == capture['prior']['standalone']['provenance']
        and value['projection_provenance'] == capture['image_provenance']
        and value['prior_down2_provenance'] == capture['prior']['down2_provenance']
        and value['mlp_provenance'] == mlp, 'unchanged upstream and exact new arithmetic lineage')
    V.keys(value['review_topics'], ' '.join(TOPICS))
    for text in [value['notes'], *value['review_topics'].values()]:
        V.require(type(text) is str and len(text.strip()) >= 32 and len(text.encode()) <= 16384,
                  'substantive root-authored notes')


def context(plan_pin):
    pins = D.Pins()
    plan = doc(pins, plan_pin, 1 << 20); input_shape(plan)
    out = E / plan['output_label']
    V.require(not os.path.lexists(out), 'exclusive one-attempt output')
    manifest, manifest_pin = package_record(pins)
    supervisor_tests(pins, plan, manifest, manifest_pin)
    Q, B = dependencies(pins)
    V.require(content(plan['baseline_capture']) == BASELINE, 'actual corrected-residual layer capture')
    capture = Q.baseline(pins, plan['baseline_capture'])
    runtime = selected_runtime(pins, plan, capture, B)
    mlp = mlp_evidence(pins, plan, capture['prior'])
    request = doc(pins, plan['request'], 64 << 10)
    request_check(pins, request, capture, runtime, out, plan['mlp_image'])
    P, standalone = capture['prior']['P'], capture['prior']['standalone']
    reviews = {}
    for role in ('parent', 'worker'):
        reviews[role] = doc(pins, plan[role + '_runtime_review'])
        P.runtime_review(reviews[role], runtime[role], standalone['platform'], pins)
    engineering_review(doc(pins, plan['capture_review'], 192 << 10), plan, runtime, capture, mlp)
    records = capture['receipt']['retained_native']
    body = read(pins, records['candidate-capture.bin'], V.LIMIT)
    bootstrap = doc(pins, records['candidate-bootstrap.json'])
    c = dict(pins=pins, plan=plan, plan_pin=plan_pin, request=request, out=out, runtime=runtime,
        standalone=standalone, P=P, O=capture['prior']['O'], platform=standalone['platform'],
        topology=standalone['topology'], owned=standalone['owned'], environment=dict(P.ENV),
        runtime_reviews=reviews, baseline_capture_body=body, baseline_input=bootstrap['layer']['input'],
        baseline_capture_payload=records['candidate-capture.bin'], supervisor_manifest=manifest_pin,
        baseline=capture['prior'], parent_cpu=capture['cpu'], worker_cpu=capture['cpu'])
    guard(c)
    return c
