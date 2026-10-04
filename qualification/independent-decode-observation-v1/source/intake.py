"""Explicit V7 image selection over the unchanged qualified Four runtime."""
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys
import types

import layer_validation as V
import host_comparison as HC
import observation as OBS

HELPER_SHA = '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820'
OWNED_SHA = 'ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'
LEGACY = ('p228-group-fence-gpu-v3', '2ed179c671075b19cda4ca1e5c411d20591be3a0c95c03095039e4f10b070384')
LEGACY_INTAKE_SHA = '951e9451edc973a7e2b8ca7ec740cf677c058325671c460973f7079ee93fefec'
COMPARISON = ('p227-prefix-decode-gpu-qualification-v2', 'ee82ba07eea0e87d20e4fe46eb880c446531d521e04c591a7a8811be051a3f1c')
NUMERICAL_CLI = ('p228-independent-profile-numerical-cli-v1', 'bbd9d3e89f0036b06d3080750f01c8003284ed9e1a48570632d27a731a371464')
NUMERICAL_CLI_SHA = '17a963b528fa7e91b4c9496ff03dd96181e712d57b3139175668d1cb659bba65'
NUMERICAL_PURE_SHA = 'b712dd14bc46d4736e6615e394369ab90c85d51e8ae699ec114844baabfdda3b'
NUMERICAL_SHAS = (
    '6185d809002e61f53edb1a47f3fbf58cfa815e23daae70c108a7d12dfeeec62e',
    '24b4a5ea414858e362ae0a6c2e6e1bc5cb1ad1d144c3e0ff8cac7289d232504a',
    'eec4e9dd9d501775ec1d1c9dfd2fb778d666d91631256994402a9618fb97b9f7',
    'c4a35d01f6ec4f838e3fdd4d2fbf6327000caf425049e9b419a1125690411c1c',
    'd3df53b2979917e1b72c03003c5407bcd1ddd43be65438942cc631c4588f0c66',
    'f4cf48160c02c7acbbcafc785f57234cd7cffe80ee8fc5c5c2cac170e959b0f5')
INPUT_SCHEMA = 'ferric-p228-independent-decode-observation-inputs-v1'
REVIEW_SCHEMA = 'ferric-p228-independent-decode-engineering-review-v1'
PACKAGE_SCHEMA = 'ferric-p228-independent-decode-observation-package-v1'
PLAN_FIELDS = ('schema output_label policy deployment image_deployment standalone_prepared '
    'standalone_cases numericals request decode_review parent_runtime_review worker_runtime_review '
    'comparison_baseline comparison_reference comparison_tests comparison_test_sources '
    'supervisor_tests supervisor_test_sources')
REVIEW_BINDINGS = ('deployment image_deployment standalone_prepared standalone_cases numericals request '
    'parent_runtime_review worker_runtime_review comparison_baseline comparison_reference').split()
REVIEW_FALSE = ('production_authority', 'full_model_acceptance', 'independent_numerical_acceptance',
    'arithmetic_prerequisites_verified', 'runtime_premises_discharged', 'performance_claim')
TOPICS = ('source_lineage', 'formal', 'isa', 'coherence', 'lifecycle', 'selected_device')
BODY = {'child-stderr.bin'} | {f'{kind}-{p}.{suffix}' for p in range(4)
    for kind, suffix in (('request', 'json'), ('control', 'bin'), ('observation', 'bin'))}

# Root fills these from the actual frozen package and its bounded pure runner.
PACKAGE_FILES = {
    'README.md', 'INTAKE.md', 'RUN.md', 'export_group_fence.py', 'frozen_owned.py',
    'group_fence_portable.py', 'host_comparison.py', 'host_validation.py', 'intake.py',
    'layer_validation.py', 'observation.py', 'policy_portable.py', 'run.py', 'run_row_facts_v2.py',
    'test_group_fence_portable.py', 'test_host_comparison.py', 'test_host_validation.py',
    'test_intake.py', 'test_observation.py', 'test_policy_portable.py', 'test_run.py',
}
PURE_TESTS = 84
TEST_RUNNER_SHA = 'c18b9fa391cfd27ba8ecac50bc23150d85261c031364d077e2fa7cd5951af696'


def bootstrap():
    path = Path(__file__).resolve().with_name('run_row_facts_v2.py')
    with path.open('rb') as stream:
        before = os.fstat(stream.fileno()); raw = stream.read((1 << 20) + 1); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    V.require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
        and len(raw) == before.st_size <= 1 << 20 and stamp(before) == stamp(after) == stamp(path.lstat())
        and hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'frozen custody helper')
    module = types.ModuleType('independent_decode_custody'); module.__file__ = str(path)
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


def legacy(pins):
    base = D.package(pins, *LEGACY)
    return D.load_module(pins, base / 'intake.py', LEGACY_INTAKE_SHA, 'independent_decode_legacy_intake')


def comparator(pins):
    return legacy(pins).comparator(pins)


def historical_deployment(pins, record):
    return legacy(pins).historical_deployment(pins, record)


def input_shape(plan):
    V.keys(plan, PLAN_FIELDS)
    V.require(plan['schema'] == INPUT_SCHEMA and type(plan['policy']) is str
        and plan['policy'] in HC.H.POLICIES and type(plan['output_label']) is str
        and re.fullmatch(r'prefix-independent-decode-(tf4|ar4)-(baseline|immutable-admission-cache|shared-full-currentness)-gpu-v228-v[1-9][0-9]{0,8}',
                         plan['output_label']), 'closed explicit independent Four input')
    for name in ('standalone_cases', 'numericals'):
        V.require(type(plan[name]) is list and len(plan[name]) == 6
            and len({row['path'] for row in plan[name]}) == 6, 'six distinct ordered actual prerequisites')
    for name in PLAN_FIELDS.split():
        if name not in ('schema', 'output_label', 'policy', 'standalone_cases', 'numericals'):
            V.require(type(plan[name]) is dict, 'actual input FilePin')


def package_record(pins):
    V.require(type(PACKAGE_FILES) is set and PACKAGE_FILES and type(PURE_TESTS) is int and PURE_TESTS > 0
        and type(TEST_RUNNER_SHA) is str and re.fullmatch('[0-9a-f]{64}', TEST_RUNNER_SHA),
        'root must freeze package members and the actual pure-test runner before context')
    base = Path(__file__).resolve().parent
    manifest, record = pins.json(base / 'manifest.json')
    V.require(manifest['schema'] == PACKAGE_SCHEMA and type(manifest['pure_tests']) is int
        and manifest['pure_tests'] == PURE_TESTS and type(manifest['files']) is list
        and len(manifest['files']) == len(PACKAGE_FILES)
        and {row['path'] for row in manifest['files']} == PACKAGE_FILES, 'closed independent supervisor package')
    for row in manifest['files']:
        V.keys(row, 'path bytes sha256')
        V.require(pins.pin(base / row['path'], row['sha256'])['bytes'] == row['bytes'], 'frozen supervisor member')
    return manifest, record


def supervisor_tests(pins, plan, manifest, manifest_pin):
    value = doc(pins, plan['supervisor_tests'])
    V.require(value['schema'] == 'ferric-p228-independent-decode-pure-v1'
        and value['passed'] is True and type(value['tests']) is int and value['tests'] == manifest['pure_tests']
        and value['manifest_sha256'] == manifest_pin['sha256'] and value['controller_sha256'] == TEST_RUNNER_SHA
        and value['source_sha256'] == plan['supervisor_test_sources']['sha256']
        and value['sources_before'] == plan['supervisor_test_sources']
        and value['source_postchecks_passed'] is True and value['synthetic_policy_tests_only'] is True,
        'actual new bounded pure-test receipt, not the historical runner')
    for key in ('errors', 'failures', 'skipped'):
        V.require(type(value[key]) is int and value[key] == 0, 'complete passing test census')
    for key in ('native_execution', 'gpu_execution', 'numerical_acceptance', 'full_model_acceptance',
                'production_authority', 'performance_claim'):
        V.require(value[key] is False, 'pure-test scope only')
    directory = Path(plan['supervisor_tests']['path']).parent
    V.require(directory.parent == E and Path(plan['supervisor_tests']['path']).name == 'complete.json'
        and re.fullmatch(r'independent-decode-pure-v228-v[1-9][0-9]{0,8}', directory.name), 'new pure-result namespace')
    for key, filename in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'),
                          ('transcript', 'tests.log')):
        V.require(value[key]['path'] == str(directory / filename), 'actual retained pure output path')
        read(pins, value[key])
    before, after = doc(pins, value['sources_before']), doc(pins, value['sources_after'])
    expected = {row['path']: dict(row, path=str(Path(__file__).resolve().parent / row['path']))
                for row in manifest['files']}
    V.require(before == after == expected, 'exact tested current package source map before and after')
    pins.pin(E / 'run_independent_decode_pure_p228_v1.py', TEST_RUNNER_SHA)


def numerical_modules(pins):
    base = D.package(pins, *NUMERICAL_CLI)
    module = D.load_module(pins, base / 'run.py', NUMERICAL_CLI_SHA, 'independent_decode_numerical_cli')
    cli_pin = pins.pin(base / 'manifest.json', NUMERICAL_CLI[1])
    pure, pure_pin = pins.json(E / 'independent-numerical-cli-pure-v228-v1/complete.json', NUMERICAL_PURE_SHA)
    V.require(pure['schema'] == 'ferric-p228-independent-profile-numerical-cli-pure-v1'
        and pure['passed'] is True and type(pure['tests']) is int and pure['tests'] == 17
        and pure['package_manifest'] == cli_pin and pure['gpu_execution'] is False
        and pure['numerical_acceptance'] is False, 'actual unchanged numerical CLI pure qualification')
    for row in [pure['controller'], pure['transcript'], pure['test_source'], *pure['source_pins'].values()]:
        read(pins, row, 8 << 20, False)
    modules, packages = module.loaded_modules(D, pins)
    packages['cli'] = cli_pin
    return module, modules, packages, pins.pin(base / 'run.py', NUMERICAL_CLI_SHA), pure_pin


def ledger(pins, values):
    V.require(type(values) is dict and 0 < len(values) <= 4096, 'bounded actual input ledger')
    for path, row in values.items():
        V.require(type(path) is str and path == row['path'], 'exact ledger path key')
        read(pins, row, 64 << 20, False)


def numerical_receipt(pins, record, index, s, replay, N, O, packages, controller):
    V.require(record['path'] == str(E / f'prefix-independent-numerical-v228-v{index + 1}/complete.json')
        and record['sha256'] == NUMERICAL_SHAS[index], 'actual completed six-case numerical cohort')
    value = doc(pins, record)
    V.keys(value, 'schema authority case inputs observation arithmetic_review packages controller conditional '
        'conditional_operator_checks_passed assumptions native_attempts_replayed profile_attempts_replayed retries '
        'input_pins elapsed_host_seconds limitations ' + ' '.join(N.FALSE))
    V.require(value['schema'] == N.SCHEMA and value['authority'] == 'none'
        and value['case'] == replay['receipt']['case'] and value['observation'] == replay['receipt_pin']
        and value['packages'] == packages and value['controller'] == controller
        and value['conditional_operator_checks_passed'] is True
        and all(value[key] is False for key in N.FALSE) and value['limitations'] == N.LIMITATIONS,
        'authenticated conditional numerical result, never full acceptance')
    for key, count in (('native_attempts_replayed', 1), ('profile_attempts_replayed', 2), ('retries', 0)):
        V.require(type(value[key]) is int and value[key] == count, 'actual finite numerical attempt census')
    inputs = doc(pins, value['inputs']); N.input_shape(inputs)
    V.require(inputs['prepared'] == s['prepared_pin'] and inputs['observation'] == replay['receipt_pin']
        and inputs['arithmetic_review'] == value['arithmetic_review'], 'numerical input and observation identities')
    review = doc(pins, value['arithmetic_review'], 64 << 10)
    V.require(review['schema'] == N.REVIEW_SCHEMA and review['authority'] == 'none'
        and review['reviewed'] is True and review['case'] == value['case']
        and review['request'] == replay['request_pin']
        and review['artifact_review'] == s['inputs']['artifact_review']
        and review['source_lineage_review'] == replay['request']['reviews'][0]
        and review['isa_review'] == replay['request']['reviews'][2]
        and review['compiler_complete'] == s['verified']['compiler_receipt']
        and review['candidate_cpu_receipt'] == s['verified']['candidate_cpu_receipt']
        and review['arithmetic_evidence'] == s['verified']['arithmetic_evidence']
        and review['tiles_image_sha256'] == s['verified']['image']['sha256']
        and review['baseline_image_sha256'] == replay['baseline']['producer']['sha256']
        and all(review[key] is False for key in N.REVIEW_FALSE), 'same recorded V7 arithmetic assumptions')
    V.require(value['assumptions'] == dict(prefix=review['prefix_assumptions'],
        attention_output=review['attention_output_assumptions'], remaining_limitations=review['remaining_limitations']),
        'unchanged recorded assumptions, no premise discharge')
    conditional = value['conditional']
    V.require(conditional['schema'] == O.RESULT_SCHEMA and conditional['authority'] == 'none'
        and conditional['case'] == value['case'] and conditional['request'] == replay['request_pin']
        and conditional['binary'] == replay['binary']
        and conditional['inspection_result'] == replay['inspection_result']
        and conditional['native_result'] == replay['native_result']
        and conditional['closed_capture_checks'] == replay['receipt']['checked']
        and conditional['profile_children'] == replay['receipt']['profile_children']
        and conditional['retained_profile_files'] == replay['receipt']['retained_profile_files']
        and conditional['conditional_operator_checks_passed'] is True
        and all(conditional[key] is False for key in O.FALSE_FIELDS), 'independently replayed conditional case binding')
    profiles = conditional['profiles']
    V.require(type(profiles) is list and len(profiles) == 2
        and [row['profile'] for row in profiles] == list(O.PROFILES), 'both actual independent profile results')
    for profile_index, row in enumerate(profiles):
        V.keys(row, 'profile checked error conditional_operator_checks_passed')
        checked = row['checked']
        V.require(row['error'] is None and row['conditional_operator_checks_passed'] is True
            and checked['schema'] == 'ferric-p228-prefix-profile-conditional-numerical-v1'
            and checked['case'] == value['case'] and checked['profile'] == row['profile']
            and checked['authority'] == 'none' and checked['conditional_operator_checks_passed'] is True
            and checked['current_kv_append_checked'] is True and checked['full_capture_bytes_hashed'] is True,
            'two actual successful conditional profile checks')
        for key in ('arithmetic_prerequisites_verified', 'capture_provenance_verified', 'full_model_acceptance',
                'full_prefix_acceptance', 'gpu_execution_verified', 'historical_kv_numerics_checked',
                'independent_numerical_acceptance', 'paired_comparison_performed', 'performance_claim',
                'production_authority', 'runtime_premises_discharged', 'untouched_kv_bytes_checked'):
            V.require(checked[key] is False, 'profile result does not enlarge conditional authority')
        for name in ('prefix_policy_sha256', 'attention_policy_sha256', 'output_reference_sha256'):
            V.require(checked[name] == review[name], 'unchanged numerical policy identities')
        V.require(checked['required_prefix_prerequisites'] == review['prefix_assumptions']
            and checked['required_attention_output_prerequisites'] == review['attention_output_assumptions'],
            'unchanged recorded numerical premises')
        rows = checked['rows']
        V.require(type(rows) is list and len(rows) == 2, 'two actual rank results')
        for rank, tensor in enumerate(rows):
            prefix = ('baseline-v5', 'tiles-v6')[profile_index]
            V.require(type(tensor['rank']) is int and tensor['rank'] == rank and tensor['profile'] == row['profile']
                and tensor['capture'] == replay['receipt']['retained_captures'][f'{prefix}-rank{rank}.bin']
                and tensor['stage_sha256'] == replay['receipt']['checked']['stage_sha256'][profile_index][rank]
                and tensor['conditioning']['output_weights'] == inputs['output_weights'][rank],
                'rank, stage, capture and output-weight identities')
        ledger(pins, checked['input_pins'])
    ledger(pins, conditional['original_input_pins'])
    ledger(pins, value['input_pins'])
    return value


def standalone(pins, plan):
    N, modules, packages, controller, cli_tests = numerical_modules(pins)
    P, O = modules['prepare'], modules['run_case']
    s = P.load(plan['standalone_prepared']); s['P'] = P
    V.require(s['inputs']['deployment'] == plan['image_deployment']
        and s['inputs']['observer_manifest'] == packages['observer']
        and s['inputs']['observer_tests'] == N.OBSERVER_TESTS, 'same qualified new image and observer')
    for index, case in enumerate(P.V.CASES):
        record = plan['standalone_cases'][index]
        V.require(record['path'] == str(E / s['inputs']['matrix_label'] / case / 'complete.json'),
                  'six exact ordered independent GPU cases')
        replay = O.replay_case(s, record)
        numerical_receipt(pins, plan['numericals'][index], index, s, replay, N,
                          modules['observe'], packages, controller)
    pins.recheck(); P.guard(s)
    return P, O, s, packages, cli_tests


def select_runtime(historical, standalone_context):
    V.keys(historical, 'parent worker image')
    selected = dict(parent=dict(historical['parent']), worker=dict(historical['worker']),
                    image=dict(standalone_context['verified']['image']))
    V.require(selected['image'] == standalone_context['prepared']['object']
        and selected['image'] != historical['image'], 'explicit separate V7 image selection')
    return selected


def engineering_review(value, plan, runtime, historical, s):
    V.keys(value, 'schema reviewed authority policy parent worker image historical_runtime image_provenance '
        'review_topics notes gpu_attempts ' + ' '.join(REVIEW_BINDINGS) + ' ' + ' '.join(REVIEW_FALSE))
    V.require(value['schema'] == REVIEW_SCHEMA and value['reviewed'] is True and value['authority'] == 'none'
        and value['policy'] == plan['policy'] and type(value['gpu_attempts']) is int and value['gpu_attempts'] == 1
        and all(value[key] is False for key in REVIEW_FALSE), 'scoped engineering observation, no proof authority')
    for name in REVIEW_BINDINGS:
        V.require(value[name] == plan[name], 'review binds exact observation prerequisite: ' + name)
    for name in ('parent', 'worker', 'image'):
        V.require(value[name] == runtime[name], 'review binds selected runtime object')
    V.require(value['historical_runtime'] == historical and value['image_provenance'] == s['provenance'],
              'separate historical runtime and actual V7 compiler/native provenance')
    V.keys(value['review_topics'], ' '.join(TOPICS))
    for text in [value['notes'], *value['review_topics'].values()]:
        V.require(type(text) is str and 32 <= len(text.strip()) and len(text.encode()) <= 16384,
                  'substantive explicit all-layer engineering review notes')


def context(plan_pin):
    pins = D.Pins(); plan = doc(pins, plan_pin, 1 << 20); input_shape(plan)
    out = E / plan['output_label']; V.require(not os.path.lexists(out), 'exclusive one-attempt output')
    manifest, manifest_pin = package_record(pins)
    old = legacy(pins); L = old.prior_helpers(pins); C, H = old.comparator(pins)
    supervisor_tests(pins, plan, manifest, manifest_pin)
    old.pure_tests(pins, plan['comparison_tests'], plan['comparison_test_sources'], COMPARISON[1],
                   27, L.TEST_RUNNER_SHA)
    deployed, historical = old.deployment(pins, plan['deployment'])
    base_deployed = doc(pins, deployed['base_deployment'])
    P, O, s, packages, cli_tests = standalone(pins, plan)
    runtime = select_runtime(historical, s)
    wrapper = doc(pins, plan['request'], 64 << 10); request, policy = HC.H.request(wrapper)
    V.require(policy == plan['policy'], 'exact request and plan policy')
    old.request_check(request, out, runtime, s, pins, L)
    tag = 'tf4' if request['mode'] == 'teacher_forced' else 'ar4'
    V.require(plan['output_label'].startswith('prefix-independent-decode-' + tag + '-' + policy + '-gpu-'),
              'output mode and policy identity')
    reader = lambda item, maximum: read(pins, item, maximum)
    previous, _ = C.baseline(reader, plan['comparison_baseline'], request['mode'], H)
    old.baseline_images(request, previous, L)
    C.reference(reader, plan['comparison_reference'], request['mode'], H)
    reviews = {}
    for role in ('parent', 'worker'):
        reviews[role] = doc(pins, plan[role + '_runtime_review'])
        P.runtime_review(reviews[role], runtime[role], s['platform'], pins)
    review = doc(pins, plan['decode_review'], 192 << 10)
    engineering_review(review, plan, runtime, historical, s)
    c = dict(pins=pins, plan=plan, plan_pin=plan_pin, request=request, policy=policy, out=out,
        runtime=runtime, selected_runtime=runtime, historical_runtime=historical,
        standalone=s, P=P, O=O, platform=s['platform'], topology=s['topology'], owned=s['owned'],
        runtime_reviews=reviews, environment=dict(P.ENV), deployment=deployed, base_deployment=base_deployed,
        image_deployment=s['deployment'], image_provenance=s['provenance'], standalone_receipts=plan['standalone_cases'],
        numerical_packages=packages, numerical_cli_tests=cli_tests, supervisor_manifest=manifest_pin, C=C, H=H)
    guard(c)
    return c


def compare_native(c, records, host_sidecar, leaf_record, value):
    candidate = dict(native_files=records, host_sidecar=host_sidecar, request=c['plan']['request'],
        parent=c['runtime']['parent'], owner=leaf_record,
        **{key: value[key] for key in ('command', 'started', 'stdout', 'stderr')})
    plan = dict(schema=OBS.INPUT_SCHEMA, mode=c['request']['mode'], policy=c['policy'],
        request=c['plan']['request'], prefix_image=c['selected_runtime']['image'], candidate=candidate)
    return OBS.observe(c['C'], plan, lambda item, maximum: read(c['pins'], item, maximum), c['H'], HC)


def guard(c):
    c['pins'].recheck(); c['P'].guard(c['standalone'])
    for review in c['runtime_reviews'].values():
        for row in review['libraries']:
            V.require(Path(row['path']).resolve(strict=True) == Path(row['resolved']['path']),
                      'all-layer runtime library alias unchanged')
