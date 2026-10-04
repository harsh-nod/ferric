"""UNFROZEN: no-launch CPU comparison after a separately completed GPU observer."""
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import stat
import sys
import time
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
PYTHON = E / 'row-reference-env-v226-mi350-v1/venv/bin/python3'
HOST = 'smci350-rck-g03-b19-03'
HELPER = E / 'p227-prefix-parity-observation-v4/run_row_facts_v2.py'
HELPER_SHA = '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820'
OBSERVER = 'p228-independent-gpu-observation-v1'
# Root must replace these only after freezing and actually testing the observer.
OBSERVER_MANIFEST_SHA = '9b06b7b929b5fddf36d73ce262abe01daf65e4eb1c2c810d2f61d1e1e3acf1a3'
OBSERVER_TESTS = dict(path=str(E / 'independent-observer-pure-v228-v1/complete.json'),
    bytes=362, sha256='732e0542d87eddcd52d3552f34a1ec82870afd3995ec839378d119a601b45323')
ADAPTER = 'p228-independent-profile-observation-v1'
ADAPTER_MANIFEST_SHA = '4c49bffced2b40b52547000f0d1b1efeabd569fc59abc1a6a1cd94b7d462952e'
ADAPTER_TESTS_SHA = '0730c9f9cc846c97c82ed9b6341e6b85acee4577e2d0a10318f93c9efad96921'
PROFILE = 'p228-prefix-profile-numerical-v1'
NUMERICAL_MANIFEST_SHA = 'd99d27e9912b486ad90b212d08fb297d894033da9845330bea36f199ed4f0122'
NUMERICAL_TESTS_SHA = '189bd2fdc8c946777a1216c3578316c1b318b69e8c92ac5ef05ca7c15575f992'
STAGE = 'p227-prefix-stage-numerical-v1'
SIDECAR = 'p227-prefix-numerical-sidecar-v1'
SCHEMA = 'ferric-p228-independent-profile-numerical-cli-result-v1'
INPUT_SCHEMA = 'ferric-p228-independent-profile-numerical-cli-inputs-v1'
REVIEW_SCHEMA = 'ferric-p228-independent-profile-arithmetic-assumptions-review-v1'
FALSE = ('gpu_execution', 'production_authority', 'runtime_premises_discharged',
    'arithmetic_prerequisites_verified', 'independent_numerical_acceptance',
    'full_prefix_acceptance', 'full_model_acceptance', 'performance_claim',
    'top_level_observer_reaping_verified', 'paired_comparison_performed')
REVIEW_FALSE = ('production_authority', 'runtime_premises_discharged',
    'arithmetic_prerequisites_verified', 'numerical_acceptance')
LIMITATIONS = [
    'Root must serialize this CPU leaf after the GPU observer exits and its task-owned descendants are reaped; this wrapper does not independently prove top-level observer reaping.',
    'Checks are conditional on actual preceding stages, supplied rotary values and the explicitly recorded arithmetic assumptions; they do not discharge every arithmetic premise.',
    'Both profiles are checked independently. Neither matching profiles nor an old source/arithmetic review is used as numerical acceptance.',
    'No full-model correctness, production authority, GPU timing or 700 tokens/s claim follows from this result.',
]


def require(value, message):
    if not value:
        raise RuntimeError(message)


def keys(value, names):
    require(type(value) is dict and set(value) == set(names.split()), 'closed fields: ' + names)


def filepin(value):
    keys(value, 'path bytes sha256')
    require(type(value['path']) is str and Path(value['path']).is_absolute()
        and '..' not in Path(value['path']).parts and str(Path(value['path'])) == value['path']
        and type(value['bytes']) is int and value['bytes'] > 0
        and type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']), 'exact FilePin')
    return value


def frozen_observer():
    require(type(OBSERVER_MANIFEST_SHA) is str
        and re.fullmatch('[0-9a-f]{64}', OBSERVER_MANIFEST_SHA)
        and OBSERVER_TESTS is not None, 'root must freeze the observer and bind actual passing pure-test evidence')
    filepin(OBSERVER_TESTS)


def bootstrap():
    require(HELPER.resolve(strict=True) == HELPER, 'canonical frozen custody helper')
    with HELPER.open('rb') as stream:
        before = os.fstat(stream.fileno())
        raw = stream.read((1 << 20) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda row: (row.st_dev, row.st_ino, row.st_size, row.st_mtime_ns, row.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and len(raw) == before.st_size <= 1 << 20
        and stamp(before) == stamp(after) == stamp(HELPER.lstat())
        and hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'unchanged custody helper bytes')
    module = types.ModuleType('independent_numerical_custody')
    module.__file__ = str(HELPER)
    exec(compile(raw, str(HELPER), 'exec'), module.__dict__)
    return module


def read(D, pins, record, maximum=8 << 20, retain=False):
    filepin(record)
    require(record['bytes'] <= maximum, 'bounded input')
    actual, raw = pins.read(Path(record['path']), record['sha256'], retain, maximum)
    require(actual == record, 'exact input extent and identity')
    return raw


def document(D, pins, record, maximum=8 << 20):
    return D.parse(read(D, pins, record, maximum, True))


def package(D, pins, root, filename, digest, roots):
    manifest, record = pins.json(root / filename, digest)
    rows = manifest['files']
    require(type(rows) is list and 0 < len(rows) <= 256, 'bounded frozen package roster')
    members = {}
    for row in rows:
        relative = Path(row['path'])
        require(not relative.is_absolute() and '..' not in relative.parts
            and str(relative) == row['path'] and row['path'] not in members
            and relative.parts[0] in roots, 'closed unique package member')
        pin = dict(row, path=str(root / relative))
        read(D, pins, pin, 8 << 20)
        members[row['path']] = pin
    return manifest, record, members


def package_tests(D, pins, adapter_manifest, numerical_manifest):
    adapter, adapter_pin = pins.json(E / 'independent-profile-observation-pure-v228-v1/complete.json', ADAPTER_TESTS_SHA)
    require(adapter['passed'] is True and type(adapter['tests']) is int and adapter['tests'] == 23
        and adapter['manifest_sha256'] == adapter_manifest['sha256']
        and adapter['synthetic_policy_tests'] is True
        and all(adapter[key] is False for key in ('actual_capture_replay', 'gpu_execution',
            'native_execution', 'production_authority', 'real_numerical_reference_execution')),
        'actual adapter policy tests, not GPU or numerical evidence')
    read(D, pins, dict(adapter['transcript'], path=str(Path(adapter_pin['path']).with_name('tests.log'))))
    numerical, numerical_pin = pins.json(E / 'prefix-profile-tests-v228-v1/complete.json', NUMERICAL_TESTS_SHA)
    require(numerical['schema'] == 'ferric-prefix-profile-cpu-tests-v1'
        and numerical['passed'] is True and type(numerical['tests']) is int and numerical['tests'] == 20
        and all(type(numerical[key]) is int and numerical[key] == 0 for key in ('errors', 'failures', 'skipped'))
        and numerical['source_manifest'] == {key: numerical_manifest[key] for key in ('bytes', 'sha256')}
        and numerical['source_postchecks_passed'] is True and numerical['fixed_reference_bounds'] is True
        and numerical['synthetic_fixtures_only'] is True and numerical['numpy'] == '2.2.6'
        and all(numerical[key] is False for key in ('gpu_execution', 'numerical_acceptance',
            'production_authority', 'full_model_acceptance', 'performance_claim')),
        'actual fixed-reference CPU tests')
    read(D, pins, dict(numerical['transcript'], path=str(Path(numerical_pin['path']).with_name('tests.log'))))
    return dict(adapter=adapter_pin, numerical=numerical_pin)


def loaded_modules(D, pins):
    _, observer_pin, observer = package(D, pins, E / OBSERVER, 'manifest.json',
        OBSERVER_MANIFEST_SHA, {row.name for row in (E / OBSERVER).iterdir()})
    _, adapter_pin, adapter = package(D, pins, E / ADAPTER, 'manifest.json',
        ADAPTER_MANIFEST_SHA, {'README.md', 'observe.py', 'validation.py', 'child_evidence.py', 'test_observe.py'})
    _, numerical_pin, _ = package(D, pins, E, PROFILE + '/source-manifest.json',
        NUMERICAL_MANIFEST_SHA, {PROFILE, STAGE, SIDECAR})
    tests = package_tests(D, pins, adapter_pin, numerical_pin)
    aliases = {}
    modules = {}
    try:
        for alias, row in (('validation', adapter['validation.py']),
            ('child_evidence', adapter['child_evidence.py']), ('observe', adapter['observe.py']),
            ('prepare', observer['prepare.py'])):
            aliases[alias] = sys.modules.get(alias)
            module = D.load_module(pins, Path(row['path']), row['sha256'], 'independent_cpu_' + alias)
            modules[alias] = module
            sys.modules[alias] = module
        row = observer['run_case.py']
        modules['run_case'] = D.load_module(pins, Path(row['path']), row['sha256'], 'independent_cpu_replay')
    finally:
        for alias, old in aliases.items():
            if old is None:
                sys.modules.pop(alias, None)
            else:
                sys.modules[alias] = old
    return modules, dict(observer=observer_pin, adapter=adapter_pin, numerical=numerical_pin, tests=tests)


def input_shape(value):
    keys(value, 'schema prepared observation arithmetic_review output_weights')
    require(value['schema'] == INPUT_SCHEMA, 'explicit independent numerical input')
    for key in ('prepared', 'observation', 'arithmetic_review'):
        filepin(value[key])
    require(type(value['output_weights']) is list and len(value['output_weights']) == 2,
        'two rank-local output-weight pins')
    for row in value['output_weights']:
        filepin(row)
    require(value['output_weights'][0]['path'] != value['output_weights'][1]['path'], 'distinct rank weight files')


def arithmetic_review(value, c, replay, frozen):
    S, _, _, N, _, _, _, _ = frozen
    keys(value, 'schema authority reviewed case request baseline_image_sha256 tiles_image_sha256 '
        'artifact_review source_lineage_review isa_review compiler_complete candidate_cpu_receipt '
        'arithmetic_evidence prefix_policy_sha256 attention_policy_sha256 output_reference_sha256 '
        'prefix_assumptions attention_output_assumptions remaining_limitations notes ' + ' '.join(REVIEW_FALSE))
    requested, baseline = replay['request'], replay['baseline']
    verified = c['verified']
    require(value['schema'] == REVIEW_SCHEMA and value['authority'] == 'none' and value['reviewed'] is True
        and value['case'] == replay['receipt']['case'] and value['request'] == replay['request_pin']
        and value['baseline_image_sha256'] == baseline['producer']['sha256']
        and value['tiles_image_sha256'] == requested['tiles']['object']['sha256'], 'case and actual image review scope')
    require(value['artifact_review'] == c['inputs']['artifact_review']
        and value['source_lineage_review'] == requested['reviews'][0]
        and value['isa_review'] == requested['reviews'][2]
        and value['compiler_complete'] == verified['compiler_receipt']
        and value['candidate_cpu_receipt'] == verified['candidate_cpu_receipt']
        and value['arithmetic_evidence'] == verified['arithmetic_evidence'], 'actual reviewed V7 provenance, not old source labels')
    require(set(value['arithmetic_evidence']) == {'source_entry', 'reciprocal_source', 'llvm', 'isa', 'elf_notes'},
        'five actual source/LLVM/ISA evidence pins')
    for row in value['arithmetic_evidence'].values():
        filepin(row)
    require(value['prefix_policy_sha256'] == S.POLICY_SHA
        and value['attention_policy_sha256'] == N.ATTENTION_POLICY
        and value['output_reference_sha256'] == N.HELPERS['output_reference.py']
        and value['prefix_assumptions'] == S.PREREQUISITES
        and value['attention_output_assumptions'] == N.PREREQUISITES,
        'unchanged prerequisite lists and numerical bounds')
    require(type(value['notes']) is str and len(value['notes'].strip()) >= 32
        and type(value['remaining_limitations']) is list and 0 < len(value['remaining_limitations']) <= 32
        and all(type(item) is str and 0 < len(item.strip()) <= 4096 for item in value['remaining_limitations'])
        and all(value[key] is False for key in REVIEW_FALSE), 'substantive assumptions review without premise discharge')
    return value


def numerical_plan(replay, weights):
    return dict(schema='ferric-p228-independent-profile-observation-inputs-v1', case=replay['receipt']['case'],
        case_directory=str(replay['case_directory']), request=replay['request_pin'], binary=replay['binary'],
        inspection_result=replay['inspection_result'], native_result=replay['native_result'], output_weights=weights)


def compare(c, replay, plan, review, O, frozen, guard):
    require(type(replay['receipt']['native_attempts']) is int and replay['receipt']['native_attempts'] == 1
        and type(replay['receipt']['retries']) is int and replay['receipt']['retries'] == 0
        and replay['receipt']['failures'] == [] and len(replay['receipt']['before_audits']) == 3
        and len(replay['receipt']['after_audits']) == 3, 'one completed native attempt and all six audits')
    arithmetic_review(review, c, replay, frozen)
    P, pins = c['P'], c['pins']
    reader = frozen[3].Reader(frozen[1])
    guard()
    result = O.compare_retained(plan, P, pins, reader, E / PROFILE, E / STAGE, E / SIDECAR)
    require(result['schema'] == O.RESULT_SCHEMA and result['case'] == plan['case']
        and result['request'] == plan['request'] and result['binary'] == plan['binary']
        and result['inspection_result'] == plan['inspection_result']
        and result['native_result'] == plan['native_result']
        and [row['profile'] for row in result['profiles']] == list(O.PROFILES)
        and all(result[key] is False for key in O.FALSE_FIELDS), 'unchanged conditional adapter result')
    for row in reader.records.values():
        P.read(pins, row, maximum=max(row['bytes'], 64 << 10))
    reader.recheck()
    pins.recheck()
    guard()
    return result


def conditional_exit_status(result):
    passed = result['conditional_operator_checks_passed']
    require(type(passed) is bool, 'explicit conditional comparison outcome')
    return 0 if passed else 1


def envelope():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
        and 'PYTHONPATH' not in os.environ and 'PYTHONHOME' not in os.environ,
        'unoptimized isolated Python environment')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == HOST
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
        and Path(sys.executable) == PYTHON, 'existing numerical Python and bounded host identity')
    require(all(os.environ.get(key) == '1' for key in ('OMP_NUM_THREADS', 'OPENBLAS_NUM_THREADS',
        'MKL_NUM_THREADS', 'NUMEXPR_NUM_THREADS', 'VECLIB_MAXIMUM_THREADS', 'BLIS_NUM_THREADS'))
        and all(os.environ.get(key) == '' for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES',
        'CUDA_VISIBLE_DEVICES')), 'single-threaded CPU-only numerical environment')
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
        (resource.RLIMIT_FSIZE, 64 << 20), (resource.RLIMIT_CORE, 0)):
        old = resource.getrlimit(kind)
        value = min([cap] + [item for item in old if item != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))
    require(shutil.disk_usage(R).free >= 40 << 30, 'initial free-space floor')
    sys.dont_write_bytecode = True


def main(args):
    require(len(args) == 4, 'INPUT_PATH SHA256 OUTPUT_LABEL CLI_MANIFEST_SHA256')
    path, digest, label, manifest_sha = args
    require(re.fullmatch(r'prefix-independent-numerical-v228-v[1-9][0-9]{0,8}', label), 'fresh CPU result label')
    frozen_observer()
    envelope()
    started = time.monotonic()
    out = E / label
    require(not os.path.lexists(out), 'no overwrite or retry')
    D = bootstrap()
    initial = D.Pins()
    initial.pin(HELPER, HELPER_SHA)
    self_root = E / 'p228-independent-profile-numerical-cli-v1'
    require(Path(__file__).resolve().parent == self_root, 'frozen deployed numerical CLI package')
    _, self_pin, _ = package(D, initial, self_root, 'manifest.json', manifest_sha,
        {'run.py', 'test_run.py', 'README.md'})
    value, input_pin = initial.json(Path(path), digest)
    input_shape(value)
    modules, packages = loaded_modules(D, initial)
    packages['cli'] = self_pin
    P, O = modules['prepare'], modules['observe']
    c = P.load(value['prepared'])
    c['P'] = P
    require(c['inputs']['observer_manifest'] == packages['observer']
        and c['inputs']['observer_tests'] == OBSERVER_TESTS, 'root-frozen actual observer qualification')
    pins = c['pins']
    for row in initial.records.values():
        P.read(pins, row, maximum=max(row['bytes'], 8 << 20))
    def guard():
        require(time.monotonic() - started < 120 and shutil.disk_usage(R).free >= 38 << 30,
            'unchanged 120-second interval and free-space floor')
        initial.recheck()
        P.guard(c)
    guard()
    replay = modules['run_case'].replay_case(c, value['observation'])
    require(replay['receipt_pin'] == value['observation'], 'exact terminal observer receipt replayed')
    _, frozen = O.load_numerical(E / PROFILE, E / STAGE, E / SIDECAR)
    review = P.document(pins, value['arithmetic_review'], 64 << 10)
    plan = numerical_plan(replay, value['output_weights'])
    os.umask(0o077)
    out.mkdir(mode=0o700)
    try:
        result = compare(c, replay, plan, review, O, frozen, guard)
        guard()
        record = P.save(out / 'complete.json', dict(schema=SCHEMA, authority='none', case=plan['case'],
            inputs=input_pin, observation=value['observation'], arithmetic_review=value['arithmetic_review'],
            packages=packages, controller=pins.pin(Path(__file__).resolve()), conditional=result,
            conditional_operator_checks_passed=result['conditional_operator_checks_passed'],
            assumptions=dict(prefix=review['prefix_assumptions'], attention_output=review['attention_output_assumptions'],
                remaining_limitations=review['remaining_limitations']), native_attempts_replayed=1,
            profile_attempts_replayed=2, retries=0, input_pins=dict(pins.records),
            elapsed_host_seconds=time.monotonic() - started, limitations=LIMITATIONS, **{key: False for key in FALSE}))
        print(json.dumps(dict(result=record, conditional_operator_checks_passed=result['conditional_operator_checks_passed'],
            gpu_execution=False, full_model_acceptance=False)), flush=True)
        return conditional_exit_status(result)
    except Exception as failure:
        P.save(out / 'failed.json', dict(schema=SCHEMA, authority='none', inputs=input_pin,
            observation=value['observation'], error=type(failure).__name__ + ': ' + str(failure),
            limitations=LIMITATIONS, **{key: False for key in FALSE}))
        raise


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
