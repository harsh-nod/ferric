"""Refresh only the worker in a fully qualified, immutable native down campaign."""
import copy
from pathlib import Path

RUNTIME_MAIN = '1736eff451d445f1f194abe145af242cf51ec322'
WORKER_SHA = '8f764849a5a1c6a23c567f8aefe253de5e6ef2320977a79db7c472ad3417a8b5'
WORKER_BYTES = 2448360
SOURCE_SHA = '5e3ec3e9e7c568fb3459120b9b7dc2171b55163389e56dc531da59095420b295'
RECEIPT_SHA = '40ed1047ec98e024af2b5cd0609ff67b2f1a80aa92eb14b7054816da3f29ed32'
OUTER_SHA = '1652d371b6b3fa01bae76b6d9d3409467fda45ac997f0ce292fa2c0a23cf6e98'
CONTRACT_SHA = '207af7b843cefba73da7beff658a213ee280bd21d2ea55a7ec6e6d84c5508739'
GUARD_SHA = '36d8eef5890c13c3a0ed6d1a99d327796ca9277bb3265cab0f5b21d3fe66fbed'
SOURCES = ('current_binding.py', 'run_stage.py', 'test_current_binding.py', 'qualify_cpu.py')
CPU_FILES = ('worker-cpu/receipt.json', 'worker-cpu/result.json', 'worker-cpu/stdout',
             'worker-cpu/stderr', 'worker-cpu/source.json', 'harness-cpu/receipt.json',
             'harness-cpu/result.json', 'harness-cpu/stdout', 'harness-cpu/stderr')
ADDED_FILES = set(SOURCES) - {'run_stage.py'} | set(CPU_FILES) | {
    'historical/run_stage.py', 'historical/launch_contract.py'}
PLAN_KEYS = {'schema', 'stage', 'cells', 'images', 'inputs', 'files', 'build', 'cpu',
             'sources', 'engineering_only', 'comparison', 'down', 'worker_refresh'}
CELL_ORDER = (('counter-A', 'A', 'counters'), ('counter-B', 'B', 'counters')) + tuple(
    (f'block{block}-{suffix}', arm, 'latency') for block in range(1, 4)
    for suffix, arm in (('A1', 'A'), ('B1', 'B'), ('B2', 'B'), ('A2', 'A')))
EXPECTED_TESTS = 24


def require(ok, why):
    if not ok:
        raise ValueError(why)


def relocate(value, old, new):
    if type(value) is dict:
        return {key: relocate(item, old, new) for key, item in value.items()}
    if type(value) is list:
        return [relocate(item, old, new) for item in value]
    if type(value) is str and (value == str(old) or value.startswith(str(old) + '/')):
        return str(new) + value[len(str(old)):]
    return copy.deepcopy(value)


def derive_cells(original, old, stage):
    require(old != stage, 'fresh campaign stage differs from historical ancestry')
    require(type(original) is list and len(original) == 14, 'exact fourteen fresh cells')
    cells = relocate(original, old, stage)
    for row, (cell_id, arm, mode) in zip(cells, CELL_ORDER):
        require(row['cell_id'] == cell_id and row['spec']['arm'] == arm
                and row['spec']['mode'] == mode
                and row['output'] == str(stage / 'cells' / cell_id), 'ordered fresh campaign roster')
        worker = {'path': str(stage / 'worker-candidate'), 'sha256': WORKER_SHA}
        for argv in (row['common_args'], row['spec']['argv']):
            for flag, value in (('--worker', worker['path']), ('--worker-sha256', WORKER_SHA)):
                require(argv.count(flag) == 1 and argv.index(flag) + 1 < len(argv),
                        'one complete worker selector')
                argv[argv.index(flag) + 1] = value
        row['spec']['worker'] = worker
    return cells


def validate_cells(observed, original, old, stage):
    require(observed == derive_cells(original, old, stage),
            'only private path relocation and one common current worker may change')


def clean_cpu(value, expected_profile):
    require(value.get('status') == value.get('returncode') == 0
            and value.get('cleanup_ok') is True and value.get('child_reaped') is True
            and value.get('term_sent') is False and value.get('kill_sent') is False
            and value.get('errors') == [] and value.get('reason') == 'completed'
            and value.get('profile') == expected_profile,
            'clean bounded unsignaled CPU closure')


def validate_worker(receipt, outer):
    require(receipt.get('schema') == 'FerricNativePacketCpuV1'
            and receipt.get('mode') == 'baseline' and receipt.get('returncode') == 0
            and receipt.get('source_unchanged') is True and receipt.get('native_executed') is False
            and receipt.get('source_sha256') == SOURCE_SHA
            and receipt.get('binary') == {'sha256': WORKER_SHA, 'bytes': WORKER_BYTES},
            'exact current uninstrumented worker build')
    roles = receipt.get('roles')
    require(type(roles) is list and len(roles) == 1 and roles[0].get('returncode') == 0
            and roles[0].get('role') == 'build', 'one successful ordinary worker build')
    argv = roles[0]['argv']
    require(len(argv) == 14 and argv[1:6] == ['build', '--release', '--locked', '--offline', '--manifest-path']
            and argv[7:] == ['-p', 'fe2o3-kfd', '--no-default-features', '--features',
                              'engineering-gfx950', '--bin', 'fe2o3-gfx950-engineering-worker'],
            'ordinary engineering feature only; diagnostics excluded')
    clean_cpu(outer, 'FerricCpuFourCore45GiBEmitterV1')


def validate_rosters(plan, original):
    files, previous = plan['files'], original['files']
    require(not (ADDED_FILES & set(previous)) and set(files) == set(previous) | ADDED_FILES,
            'closed historical inputs plus explicit refresh inputs')
    for name, item in previous.items():
        if name not in ('worker-candidate', 'run_stage.py'):
            require(files[name] == item, 'historical artifact changed: ' + name)
    require(files['worker-candidate'] == {'sha256': WORKER_SHA, 'bytes': WORKER_BYTES, 'mode': 0o700},
            'current worker staged executable identity')
    for name, pin in (('historical/run_stage.py', GUARD_SHA),
                      ('historical/launch_contract.py', CONTRACT_SHA)):
        require(files[name] == previous[name.split('/')[-1]] and files[name]['sha256'] == pin,
                'unchanged historical admission implementation')
    wanted = dict(original['sources'])
    wanted.update({name: files[name]['sha256'] for name in SOURCES})
    require(plan['sources'] == wanted, 'closed source roster; measurement remains historical')


def refresh_envelope(parent):
    return {'schema': 'FerricNativeDownWorkerRefreshV1', 'runtime_main': RUNTIME_MAIN,
            'parent_plan': copy.deepcopy(parent), 'worker_sha256': WORKER_SHA,
            'source_sha256': SOURCE_SHA, 'historical_runtime_is_ancestry_only': True,
            'wait_diagnostics': False, 'packet_diagnostics': False}


def derive_plan(c, original, stage, parent_binding, files, cell):
    old = Path(original['stage'])
    plan = {key: relocate(original[key], old, stage)
            for key in ('schema', 'stage', 'images', 'inputs', 'down', 'engineering_only')}
    # These describe the immutable controller/kernel ancestry, not the new worker.
    plan.update(build=copy.deepcopy(original['build']), cpu=copy.deepcopy(original['cpu']),
                files=copy.deepcopy(files), worker_refresh=refresh_envelope(parent_binding))
    plan['sources'] = dict(original['sources'])
    plan['sources'].update({name: files[name]['sha256'] for name in SOURCES})
    plan['cells'] = derive_cells(original['cells'], old, stage)
    plan['comparison'] = c.comparison_plan(plan['cells'], plan['inputs'], plan['sources'], cell)
    return plan


def validate(c, stage, plan, cell_id):
    require(type(plan) is dict and set(plan) == PLAN_KEYS, 'closed current-worker campaign plan')
    refresh = plan['worker_refresh']
    require(type(refresh) is dict and 'parent_plan' in refresh
            and refresh == refresh_envelope(refresh['parent_plan']), 'closed explicit worker refresh')
    c.binding(refresh['parent_plan'])
    old = c.stage_name(Path(refresh['parent_plan']['path']).parent)
    require(refresh['parent_plan']['path'] == str(old / 'plan.json') and old != stage,
            'distinct canonical historical plan')
    original = c.bound(refresh['parent_plan'])
    c.read(old / 'launch_contract.py', CONTRACT_SHA)
    c.read(old / 'run_stage.py', GUARD_SHA)
    c.validate_plan(original, old, cell_id)
    c.stage_name(stage)
    validate_rosters(plan, original)
    c.verify_files(stage, plan['files'])
    receipt = c.decode(c.read(stage / 'worker-cpu/receipt.json', RECEIPT_SHA)[0])
    outer = c.decode(c.read(stage / 'worker-cpu/result.json', OUTER_SHA)[0])
    validate_worker(receipt, outer)
    c.read(stage / 'worker-cpu/source.json', SOURCE_SHA)
    for name in ('stdout', 'stderr'):
        c.read(stage / 'worker-cpu' / name, receipt['roles'][0][name + '_sha256'], empty=True)
    cpu = c.decode(c.read(stage / 'harness-cpu/receipt.json')[0])
    require(cpu.get('schema') == 'FerricNativeDownCurrentCpuV1' and cpu.get('accepted') is True
            and cpu.get('returncode') == 0 and cpu.get('native_executed') is False
            and cpu.get('expected_tests') == EXPECTED_TESTS
            and cpu.get('source_before') == cpu.get('source_after') ==
                {name: plan['sources'][name] for name in SOURCES}, 'current launcher CPU qualification')
    clean_cpu(c.decode(c.read(stage / 'harness-cpu/result.json')[0]), 'FerricCpuFourCore48GiBEmitterV1')
    for name in ('stdout', 'stderr'):
        c.read(stage / 'harness-cpu' / name, cpu[name + '_sha256'], empty=True)
    counter = c.module(stage / 'counter-support/run_counter_diagnostic.py', c.COUNTER_PINS['run_counter_diagnostic.py'])
    evidence = c.module(stage / 'counter-support/process_evidence.py', c.COUNTER_PINS['process_evidence.py'])
    loaded = counter.load(stage)
    _, _, legacy, _, _, runner, supervisor, _ = loaded
    require(supervisor.ROOT_FREE_BYTES == 64 * 1024**3
            and supervisor.STAGE_BYTES == 2 * 1024**3
            and supervisor.MEMORY_AVAILABLE_BYTES == 128 * 1024**3, 'frozen native resource limits')
    cell = c.module(stage / 'measurement/native_token_cell.py', plan['sources']['measurement/native_token_cell.py'])
    require(plan == derive_plan(c, original, stage, refresh['parent_plan'], plan['files'], cell),
            'worker-only reconstructed campaign differs')
    validate_cells(plan['cells'], original['cells'], old, stage)
    for row in plan['cells']:
        cell.shape(row['spec'])
    c.validate_down(stage, plan['down'], plan['files'])
    selected = c.selected_cell(plan, cell_id)
    options = c.parse_common(selected['common_args'], runner)
    identities = legacy.inputs({**plan['inputs'], 'controller': selected['spec']['controller'],
                                'images': plan['images']}, options, runner)
    return original, loaded, counter, evidence, cell, options, identities
