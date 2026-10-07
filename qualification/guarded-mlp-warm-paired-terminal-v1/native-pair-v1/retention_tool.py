"""Bounded original warm-terminal control/candidate retention; no native launch."""
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import tarfile
import time
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-warm-paired-terminal-gpu-v228-v1'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
DEST = Path('/home/harsh/ferric-p227-integration/qualification/guarded-mlp-warm-paired-terminal-v1')
ROOT_PINS = {
    "run_model_gpu.py": {
        "bytes": 43699,
        "sha256": "3ce5be14d4c4024aed049a8884165cd84d57f63852412bf3a57014322aa8352c"
    },
    "prepare_model_inputs.py": {
        "bytes": 15592,
        "sha256": "648f1e1d910121152bfe03743fde53b2a618e0750e87737079e19268e00466b5"
    },
    "validate_observation.py": {
        "bytes": 17983,
        "sha256": "7224877439ec966254d7604317f1f47b5d6345af698e47e332f6cb36c3abc19b"
    },
    "frozen_owned.py": {
        "bytes": 30433,
        "sha256": "ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583"
    },
    "library_audit.py": {
        "bytes": 25872,
        "sha256": "b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d"
    },
    "guarded_announcement.py": {
        "bytes": 3251,
        "sha256": "96f84389071ef2c1a9e354705ebb7144e9ff97e30c242bd9b494f03d2aea6b80"
    },
    "prepared-inputs.json": {
        "bytes": 14481,
        "sha256": "a29a53e9935cc80984049fdbdf1acd55c82436ccac6c3a8cbc0dcc05bb9ba675"
    },
    "control-input.json": {
        "bytes": 1640,
        "sha256": "c8e2b42a1a16d98e2d0fdb10b7fbefcbdf22e6b559582e41d158d157953c3c9d"
    },
    "candidate-input.json": {
        "bytes": 1643,
        "sha256": "9490f915d3d71a575f8dd727f1afa1745a258e4ce05a2454b3ad613aff06fe7e"
    },
    "control-request.json": {
        "bytes": 9345,
        "sha256": "c173559e545aa92881302f69ec518228e216c6d8144cbddba87e306eebe99f2b"
    },
    "candidate-request.json": {
        "bytes": 9359,
        "sha256": "a6a70d0285f0680ecf03106d373456f8d92e4580068e21ecfb7fedc1d6bfb517"
    }
}
ADMISSION = {
    'worker_cpu': {'bytes':1709799, 'sha256':'28e110ab7ebd6d28e9f31499165f364bac729e7e6d0a408b6cb3a369044a340b'},
    'worker': {'bytes':6135752, 'sha256':'240c62c1c3eca809ed63bb00f3612e0059df20786bd1a21d30e04aa628ad97c6'},
    'parent_cpu': {'bytes':3901680, 'sha256':'29af2c2ef9e4eeb93bdc0f57a5ae463c2baf751f2824e86430260d8a29ce1f3c'},
    'parent': {'bytes':13920552, 'sha256':'e82139705d4eb861ceecc75aba6036939a14ed4889f9dd616e57d08c8e8c0f15'},
}
CHECKER = {'bytes':4147, 'sha256':'ca0ddb8f614a17e09c787d75f3a5d0d912456642be59b179cc57ea766fff6adc'}
PRIOR = str(E / 'guarded-mlp-model-gpu-v228-v3/ar4')
REFERENCE_PINS = {
    'reference/ordinary-ar4/complete.json': {'path':PRIOR+'/complete.json', 'bytes':93497, 'sha256':'edf05cf2dd19a9934dab9762b328ea3e6160cf01c15606a3df3299ce967e4991'},
    'reference/ordinary-ar4/observation-0.bin': {'path':PRIOR+'/native/observation-0.bin', 'bytes':606976, 'sha256':'1324a6056ea6c0308e912c63ed1b6a6d27e103c915c6dde11ff600d97cf9b1b0'},
    'reference/ordinary-ar4/observation-1.bin': {'path':PRIOR+'/native/observation-1.bin', 'bytes':606976, 'sha256':'61f56a5b97e4f35922bf7733a74b36b8dec9544a15d66980757bf9ef5aefc6e4'},
    'reference/ordinary-ar4/observation-2.bin': {'path':PRIOR+'/native/observation-2.bin', 'bytes':606976, 'sha256':'87d3799724693386909e97fd3ea31fab065de021824dea276218614f2a0fc85a'},
    'reference/ordinary-ar4/observation-3.bin': {'path':PRIOR+'/native/observation-3.bin', 'bytes':606976, 'sha256':'7c40126b76a7eeb7a64ef67950636359ba951cafcdce2ddc42636388de3c5588'},
}
LABELS = ('parent-readelf', 'parent-ldd', 'worker-readelf', 'worker-ldd',
          'before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2')
IDS = [16366993098680759275, 10838076764495710945]
FALSE_FLAGS = ('full_model_acceptance', 'numerical_acceptance', 'performance_claim',
    'production_authority', 'full_long_workload', 'independent_full_model_reference',
    'host_observation_requested', 'shared_full_currentness_requested', 'hidden_read_policy_changed',
    'gpu_latency_claim', 'timing_comparison_performed', 'speedup_claim', 'outer_model_shards_rehashed')
MAX_FILES, MAX_BODY, MAX_TOTAL = 531, 8 << 20, 144 << 20
DEADLINE = None
DOCUMENTATION = {'bytes':4774, 'sha256':'21bab0e425421940bb3644017dbd41e995486c2a003f89dd5fd1b6a4647c6cb1'}

def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def ordinary(name):
    return (type(name) is str and Path(name).as_posix() == name and name not in ('', '.')
            and not Path(name).is_absolute() and '..' not in Path(name).parts)


def stamp(row):
    return (row.st_dev, row.st_ino, row.st_mode, row.st_nlink,
            row.st_size, row.st_mtime_ns, row.st_ctime_ns)


def read(path, expected=None):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_nlink == 1 and 0 <= before.st_size <= MAX_BODY,
            'ordinary bounded unlinked body: ' + str(path))
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'opened file changed')
        body = stream.read(MAX_BODY + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'open file changed while reading')
    require(stamp(path.lstat()) == stamp(before) and len(body) == before.st_size
            and (expected is None or pin(body) == compact(expected)), 'file pin or identity changed: ' + str(path))
    return body


def roster(root):
    require(root.resolve(strict=True) == root and root.is_dir(), 'ordinary evidence directory')
    result = set()
    for directory, dirs, names in os.walk(root, followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(all(not (Path(directory) / name).is_symlink() for name in dirs), 'directory symlink')
        for name in names:
            path = Path(directory) / name
            relative = str(path.relative_to(root))
            require(ordinary(relative) and relative not in result, 'ordinary unique source member')
            require(path.resolve(strict=True) == path and stat.S_ISREG(path.lstat().st_mode), 'nonregular member')
            result.add(relative)
        require(len(result) <= MAX_FILES, 'bounded member census')
    return result


def wire_pin(row):
    require(type(row['sha256']) is list and len(row['sha256']) == 32
            and all(type(value) is int and 0 <= value <= 255 for value in row['sha256']), 'exact wire digest bytes')
    return dict(bytes=row['bytes'], sha256=bytes(row['sha256']).hex())


def same(left, right):
    return json.dumps(left, sort_keys=True, separators=(',', ':'), allow_nan=False) == json.dumps(
        right, sort_keys=True, separators=(',', ':'), allow_nan=False)


def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON member')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def validate_topology(snapshot):
    require(snapshot['host'] == 'smci350-rck-g03-b19-03'
            and re.fullmatch('[0-9a-f-]{36}', snapshot['boot']), 'selected host/boot shape')
    require(len(snapshot['devices']) == 2, 'selected topology extent')
    for row, node, unique_id in zip(snapshot['devices'], (2, 3), IDS):
        require(row['node'] == node and type(row['gpu_id']) is int and row['gpu_id'] > 0
                and int(row['properties']['unique_id']) == unique_id
                and int(row['properties']['gfx_target_version']) == 90500,
                'exact selected gfx950 unique_id; gpu_id is a separate KFD handle')


def compare_payloads(summary_raw, read_body, validator, prior):
    summary = validator.parse(summary_raw)
    prior_pin, prior_inputs, values = prior
    rows = []
    for index, (old_pin, old) in enumerate(values):
        current = validator.rust_pin(summary['files']['frames'][index]['observation'])
        raw = read_body(current)
        require(len(raw) == current['bytes'] == len(old) == validator.PAYLOAD_BYTES
                and hashlib.sha256(raw).hexdigest() == current['sha256'], 'current observation body')
        rows.append(dict(position=index, current=current, ordinary=old_pin, byte_equal=raw == old,
                         same_history=summary['input_tokens'][:index + 1] == prior_inputs[:index + 1]))
    return dict(schema='ferric-guarded-mlp-warm-paired-terminal-payload-comparison-v1', reference_terminal=prior_pin,
        frames=rows, all_payloads_equal=all(row['byte_equal'] for row in rows),
        all_histories_equal=all(row['same_history'] for row in rows),
        observed_payload_difference=any(not row['byte_equal'] for row in rows),
        causal_effect_established=False,
        comparison_completed=True, independent_accuracy_reference=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)


def guard():
    require(DEADLINE is not None and time.monotonic() < DEADLINE, 'retention whole deadline')


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def load_pure(bodies, name, module_name):
    require(name in ('validate_observation.py', 'guarded_announcement.py')
            and pin(bodies[name]) == ROOT_PINS[name], 'closed qualified pure module')
    module = types.ModuleType(module_name)
    module.__file__ = str(ROOT / name)
    exec(compile(bodies[name], module.__file__, 'exec'), module.__dict__)
    return module


def success_names():
    names = {label + '/' + name for label in LABELS
             for name in ('command.json', 'started.json', 'stdout', 'stderr', 'result.json')}
    names |= {'initial-topology.json', 'observation.json', 'payload-comparison.json',
              'native/complete.json', 'native/child-stderr.bin'}
    names |= {side + '-' + str(i) + '-topology.json' for side in ('before', 'after') for i in range(3)}
    names |= {'native/%s-%d.%s' % (stem, i, suffix) for i in range(4)
              for stem, suffix in (('request', 'json'), ('control', 'bin'), ('observation', 'bin'))}
    require(len(names) == 78, 'fixed successful raw roster')
    return names


def mapped_read(bodies, row):
    path = row['path']
    require(type(path) is str and path.startswith(str(ROOT) + '/'), 'owned retained input path')
    name = path[len(str(ROOT)) + 1:]
    require(ordinary(name) and name in bodies and pin(bodies[name]) == compact(row), 'retained input hash')
    return bodies[name]


def preparation(bodies):
    for name, expected in ROOT_PINS.items():
        require(pin(bodies[name]) == expected, 'exact prepared source/data: ' + name)
    prepared = parse(bodies['prepared-inputs.json'])
    require(prepared['schema'] == 'ferric-guarded-mlp-warm-paired-terminal-input-preparation-v1'
            and prepared['passed'] is True and prepared['data_only'] is True
            and prepared['cpu_tests_executed'] is False and prepared['native_execution'] is False
            and prepared['gpu_execution'] is False and prepared['numerical_acceptance'] is False
            and prepared['performance_claim'] is False and set(prepared['generated']) == {'control', 'candidate'}
            and prepared['controller'] == dict(path=str(ROOT / 'prepare_model_inputs.py'),
                                               **ROOT_PINS['prepare_model_inputs.py']), 'original data preparation')
    require(compact(prepared['historical_request']) == {'bytes':8818,
        'sha256':'44e9a717750c70ea4c6316df1aae4fcd9abd386759188fb59b443f04a6a6c3ab'}, 'historical request ancestry')
    plans, requests = {}, {}
    for case in ('control', 'candidate'):
        plan = parse(bodies[case + '-input.json'])
        request = parse(bodies[case + '-request.json'])
        paired = case == 'candidate'
        require(set(plan) == {'schema', 'mode', 'case', 'paired_terminal', 'parent_cpu', 'worker_cpu',
                             'parent', 'worker', 'request'}
                and plan['schema'] == 'ferric-guarded-mlp-warm-paired-terminal-gpu-input-v1'
                and plan['mode'] == 'ar4' and plan['case'] == case and plan['paired_terminal'] is paired,
                'closed actual case plan')
        for role in ADMISSION:
            require(compact(plan[role]) == ADMISSION[role]
                    and prepared['readset'][plan[role]['path']] == plan[role], 'actual CPU/product metadata')
        require(request['schema'] == ('FerricFiniteGuardedMlpReusableAr4PairedTerminalRequestV1'
                if paired else 'FerricFiniteGuardedMlpReusableAr4RequestV1')
                and request['decode']['mode'] == 'autoregressive'
                and request['decode']['evidence_directory'] == str(ROOT / case / 'native'), 'closed selected request')
        for role, suffix in (('plan', 'input'), ('request', 'request')):
            row = prepared['generated'][case][role]
            require(row == dict(path=str(ROOT / (case + '-' + suffix + '.json')),
                                **ROOT_PINS[case + '-' + suffix + '.json']), 'actual preparation output')
        require(plan['request'] == prepared['generated'][case]['request']
                and request['decode']['session'] == prepared['generated'][case]['session'], 'session/output join')
        plans[case], requests[case] = plan, request
    require(all(plans['control'][key] == plans['candidate'][key] for key in ADMISSION),
            'identical actually qualified ELF pair')
    require(requests['control']['decode']['session'] != requests['candidate']['decode']['session'],
            'distinct actual sessions')
    for key in requests['control']['decode']:
        if key not in ('session', 'evidence_directory'):
            require(same(requests['control']['decode'][key], requests['candidate']['decode'][key]),
                    'same control/candidate source and setup')
    require(same(requests['control']['projection_image'], requests['candidate']['projection_image'])
            and same(requests['control']['guarded_image'], requests['candidate']['guarded_image']), 'same selected images')
    return plans, requests


def case_admission(bodies, case, outcome, plans, requests, validator, announcement):
    require(case in ('control', 'candidate') and set(outcome) == {'name', 'sha256'}, 'closed case outcome')
    names = {name[len(case) + 1:] for name in bodies if name.startswith(case + '/')}
    require(len(names) <= 256 and sum(len(bodies[case + '/' + name]) for name in names) <= 64 << 20,
            'original per-case member/byte limits')
    terminal_name = outcome['name']
    if terminal_name == 'absent':
        require(outcome['sha256'] is None and not ({'complete.json', 'failed.json'} & names),
                'original missing-terminal prefix')
        return dict(passed=False, terminal=None, terminal_name='absent', original_files=len(names),
                    raw_files=len(names), observation=None, terminal_missing=True)
    require(terminal_name in ('complete.json', 'failed.json')
            and type(outcome['sha256']) is str and re.fullmatch('[0-9a-f]{64}', outcome['sha256'])
            and names & {'complete.json', 'failed.json'} == {terminal_name}, 'one original observed terminal')
    raw = bodies[case + '/' + terminal_name]
    require(pin(raw)['sha256'] == outcome['sha256'], 'observed original terminal SHA')
    value = parse(raw)
    paired = case == 'candidate'
    require(value['schema'] == 'ferric-guarded-mlp-warm-paired-terminal-gpu-v1'
            and value['case'] == case and value['mode'] == 'ar4'
            and type(value['passed']) is bool and value['paired_terminal_requested'] is paired
            and value['same_elf_control_required'] is paired and value['retries'] == 0
            and type(value['native_attempts']) is int and value['native_attempts'] in (0, 1)
            and all(value[key] is False for key in FALSE_FLAGS)
            and value['default_full_currentness_requested'] is True
            and value['reusable_arena_requested'] is True and value['baseline_payload_equality_required'] is True,
            'original scope and unchanged false authority')
    require(terminal_name == ('complete.json' if value['passed'] else 'failed.json')
            and type(value['errors']) is list and (not value['errors'] if value['passed'] else bool(value['errors'])),
            'original pass/failure preserved')
    require(value['controller'] == dict(path=str(ROOT / 'run_model_gpu.py'), **ROOT_PINS['run_model_gpu.py'])
            and value['plan'] == dict(path=str(ROOT / (case + '-input.json')), **ROOT_PINS[case + '-input.json'])
            and value['admission'] == {key: plans[case][key] for key in ADMISSION},
            'actual controller/plan/CPU products')
    require(value['checker_cpu'] == dict(path=str(E / 'guarded-mlp-warm-paired-terminal-checker-cpu-v228-v1/complete.json'), **CHECKER),
            'actual twenty-test gate')
    for key, expected in (
        ('parser_cpu', {'bytes':8872, 'sha256':'170ba440cdf33cd88fc55c2ddff709624fd819d3d461d54daad239a719efdf5a'}),
        ('tf4_data_revalidation', {'bytes':79327, 'sha256':'d0551f310a57f63dfe98af8c887b5996752e3b62128a3087c4a03849082e6a3f'}),
        ('checker_cpu', CHECKER)):
        require(compact(value[key]) == expected and value['readset'][value[key]['path']] == value[key],
                'original prior-gate metadata')
    require(value['limits'] == dict(native_seconds=4000, whole_seconds=4300, address_space_bytes=32 << 30,
        stream_bytes=8 << 20, case_bytes=64 << 20, case_members=256, affinity=[8, 9]), 'original execution bounds')
    require(names - {terminal_name} == set(value['raw']), 'complete original case raw closure')
    for name, row in value['raw'].items():
        require(ordinary(name) and row['path'] == str(ROOT / case / name)
                and pin(bodies[case + '/' + name]) == compact(row), 'original raw pin: ' + name)
    for path, row in value['readset'].items():
        require(path == row['path'], 'readset identity')
        if path.startswith(str(ROOT) + '/'):
            mapped_read(bodies, row)
    for role in ADMISSION:
        require(value['readset'][plans[case][role]['path']] == plans[case][role], 'original product readset')
    for name, row in REFERENCE_PINS.items():
        require(value['readset'][row['path']] == row and pin(bodies[name]) == compact(row),
                'ordinary reference original path and content')
    labels = [row['label'] for row in value['phases']]
    require(len(labels) == len(set(labels)) and all(label in LABELS for label in labels), 'unique known original leaves')
    for phase in value['phases']:
        label = phase['label']
        require(same(parse(bodies[case + '/' + label + '/result.json']),
                     {key:item for key,item in phase.items() if key != 'label'}), 'original result body')
        for key, filename in (('command','command.json'), ('started','started.json'), ('stdout','stdout'), ('stderr','stderr')):
            if phase[key] is not None:
                require(phase[key] == value['raw'][label + '/' + filename], 'phase stream pin join')
    result = dict(passed=value['passed'], terminal=pin(raw), terminal_name=terminal_name,
                  original_files=len(names), raw_files=len(value['raw']), observation=None, terminal_missing=False)
    if not value['passed']:
        return result
    require(set(value['raw']) == success_names() and labels == list(LABELS)
            and value['native_attempts'] == 1 and value['errors'] == []
            and all(value[key] is True for key in ('native_spawn_observed', 'gpu_execution_requested',
                'gpu_execution', 'gpu_execution_confirmed', 'actual_arena_plateau_verified'))
            and value['partial_gpu_execution_possible'] is False, 'complete original successful case')
    for phase in value['phases']:
        label = phase['label']
        command = parse(bodies[case + '/' + label + '/command.json'])
        started = parse(bodies[case + '/' + label + '/started.json'])
        require(phase['exit_code'] == 0 and phase['reason'] is None
                and phase['cleanup_signalled'] is False and phase['owned_groups_absent'] is True
                and phase['owned_processes_reaped'] is True
                and phase['gpu_execution_requested'] is (label == 'parent')
                and command['gpu_execution_requested'] is (label == 'parent')
                and started['command_sha256'] == phase['command']['sha256']
                and phase['lineage'][0]['event'] == 'owned'
                and phase['lineage'][0]['reason'] == 'spawned-parent'
                and phase['lineage'][0]['identity'] == started['parent'], 'clean natural original leaf')
    native_root = ROOT / case / 'native'
    summary = bodies[case + '/native/complete.json']
    require(same(parse(summary), parse(bodies[case + '/parent/stdout'])), 'original native stdout/summary')
    read_body = lambda row: mapped_read(bodies, row)
    checked = validator.validate(summary, requests[case], native_root, read_body, paired)
    require(same(checked, value['observation'])
            and same(checked, parse(bodies[case + '/observation.json']))
            and same(checked['segment_host_ns'], value['segment_host_ns'])
            and checked['paired_terminal_dispatches'] == value['paired_terminal_dispatches'] ==
                ([0, 0, 36, 36] if paired else [0, 0, 0, 0]), 'exact qualified numerical/control/census replay')
    native_phase = value['phases'][7]
    announcement.validate_lineage(bodies[case + '/parent/stderr'], native_phase,
        parse(bodies[case + '/parent/started.json']), checked['child_pid'])
    require(value['native_started'] == value['raw']['parent/started.json'], 'actual native spawn original body')
    platform = parse(bodies[case + '/initial-topology.json'])
    validate_topology(platform)
    require(same(platform, value['platform']), 'initial recorded platform')
    for side in ('before', 'after'):
        for index in range(3):
            label = side + '-' + str(index)
            require(same(parse(bodies[case + '/' + label + '-topology.json']), platform), 'recorded topology stability')
            require(parse(bodies[case + '/' + label + '/stdout']) == [
                dict(gpu=i, process_list=[dict(process_info='No running processes detected')]) for i in range(8)],
                'six original all-eight-device idle observations')
    old = parse(bodies['reference/ordinary-ar4/complete.json'])
    prior = (REFERENCE_PINS['reference/ordinary-ar4/complete.json'], old['observation']['input_tokens'], [
        (REFERENCE_PINS['reference/ordinary-ar4/observation-%d.bin' % i],
         bodies['reference/ordinary-ar4/observation-%d.bin' % i]) for i in range(4)])
    ordinary_comparison = compare_payloads(summary, read_body, validator, prior)
    require(same(ordinary_comparison, value['ordinary_comparison'])
            and ordinary_comparison['all_payloads_equal'] is True
            and ordinary_comparison['all_histories_equal'] is True, 'all four ordinary payloads and own histories')
    control_comparison = None
    if paired:
        control = parse(bodies['control/complete.json'])
        control_ref = (dict(path=str(ROOT / 'control/complete.json'), **pin(bodies['control/complete.json'])),
            control['observation']['input_tokens'], [
                (control['raw']['native/observation-%d.bin' % i], bodies['control/native/observation-%d.bin' % i])
                for i in range(4)])
        control_comparison = compare_payloads(summary, read_body, validator, control_ref)
        require(control_comparison['all_payloads_equal'] is True
                and control_comparison['all_histories_equal'] is True, 'all four same-ELF control payloads and own histories')
    require(same(control_comparison, value['control_comparison'])
            and same(parse(bodies[case + '/payload-comparison.json']),
                     dict(ordinary=ordinary_comparison, control=control_comparison)), 'original complete comparison record')
    result['observation'] = checked
    return result


def verify(bodies, outcomes):
    require(type(outcomes) is dict and set(outcomes) in ({'control'}, {'control','candidate'}), 'closed requested case set')
    own = read(Path(__file__).resolve())
    require(bodies['retention_tool.py'] == own and pin(bodies['RETENTION.md']) == DOCUMENTATION,
            'same reviewed retention source/docs')
    selected = set(ROOT_PINS) | set(REFERENCE_PINS) | {'retention_tool.py','RETENTION.md'}
    case_names = {n for n in bodies if any(n.startswith(c + '/') for c in outcomes)}
    require(set(bodies) == selected | case_names, 'no unselected original or invented body')
    require(len(bodies) <= MAX_FILES and sum(map(len,bodies.values())) <= MAX_TOTAL, 'bounded total original bodies')
    plans, requests = preparation(bodies)
    for name, expected in REFERENCE_PINS.items():
        require(pin(bodies[name]) == compact(expected), 'exact ordinary reference body')
    old = parse(bodies['reference/ordinary-ar4/complete.json'])
    require(old['passed'] is True and old['errors'] == [] and old['native_attempts'] == 1 and old['retries'] == 0,
            'original reference result metadata only, not independent accuracy')
    validator = load_pure(bodies, 'validate_observation.py', 'warm_terminal_retained_validator')
    announcement = load_pure(bodies, 'guarded_announcement.py', 'warm_terminal_retained_announcement')
    results = {}
    for case in ('control','candidate'):
        if case in outcomes:
            if case == 'candidate':
                require(results['control']['passed'] is True, 'candidate requires fully revalidated actual control')
            results[case] = case_admission(bodies,case,outcomes[case],plans,requests,validator,announcement)
    all_passed = set(outcomes) == {'control','candidate'} and all(r['passed'] for r in results.values())
    if all_passed:
        require(len(bodies) == 176 and sum(r['raw_files'] for r in results.values()) == 156,
                'exact successful pair176 body/156raw census')
    return dict(cases=results, all_cases_passed=all_passed, original_files=len(bodies),
        root_files=len(ROOT_PINS), reference_files=len(REFERENCE_PINS),
        qualified_pure_validator_rechecked=any(r['passed'] for r in results.values()),
        independent_accuracy_reference=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False, gpu_timing_claim=False, production_authority=False,
        new_native_execution=False, external_model_or_elf_bodies_rehashed=False)


def specs(args):
    require(len(args) in (3,5) and args[0] in ('control','pair'), 'control NAME SHA | pair CONTROL_NAME SHA CANDIDATE_NAME SHA')
    cases = ('control',) if args[0] == 'control' else ('control','candidate')
    require(len(args) == 1 + 2 * len(cases), 'exact observed outcome arguments')
    outcomes = {}
    for index, case in enumerate(cases):
        name, sha = args[1+2*index:3+2*index]
        require(name in ('complete.json','failed.json','absent'), 'original terminal name or missing terminal')
        require((name == 'absent' and sha == '-') or re.fullmatch('[0-9a-f]{64}',sha), 'observed SHA or explicit missing terminal')
        outcomes[case] = dict(name=name,sha256=None if name=='absent' else sha)
    return args[0], outcomes


def archive_name(scope):
    return 'guarded-mlp-warm-paired-terminal-native-' + scope + '-evidence-v228-v1.tar.gz'


def export(scope, outcomes):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'original native host')
    require(ROOT.resolve(strict=True) == ROOT and E.resolve(strict=True) == E, 'canonical original roots')
    bodies = {name:read(ROOT/name,expected) for name,expected in ROOT_PINS.items()}
    for name, expected in REFERENCE_PINS.items():
        bodies[name] = read(Path(expected['path']),expected)
    for case in outcomes:
        directory = ROOT/case
        require(directory.resolve(strict=True) == directory, 'actual spent case directory')
        names = roster(directory)
        require(len(names) <= 256, 'original case file count')
        for name in names:
            guard(); bodies[case+'/'+name] = read(directory/name)
    if scope == 'control':
        require(not os.path.lexists(ROOT/'candidate'), 'control-only export before any candidate')
    own = Path(__file__).resolve()
    bodies['retention_tool.py'] = read(own)
    bodies['RETENTION.md'] = read(own.parent/'RETENTION.md',DOCUMENTATION)
    result = verify(bodies,outcomes)
    manifest = dict(schema='ferric-warm-paired-terminal-native-export-v1', outcomes=outcomes,
                    verification=result, files={n:pin(b) for n,b in sorted(bodies.items())})
    bodies['manifest.json'] = encoded(manifest)
    require(sum(map(len,bodies.values())) <= MAX_TOTAL and len(bodies) <= MAX_FILES, 'closed expanded archive')
    output=E/archive_name(scope)
    require(not os.path.lexists(output) and os.statvfs(E).f_bavail*os.statvfs(E).f_frsize >= 1<<30, 'fresh archive/space reserve')
    with output.open('xb') as stream:
        with tarfile.open(fileobj=stream,mode='w:gz',format=tarfile.USTAR_FORMAT) as tar:
            for name,body in sorted(bodies.items()):
                guard(); row=tarfile.TarInfo(name);row.size=len(body);row.mode=0o644;row.mtime=0
                tar.addfile(row,io.BytesIO(body))
        stream.flush();os.fsync(stream.fileno())
    for name,body in bodies.items():
        if name=='manifest.json':continue
        path=(own if name=='retention_tool.py' else own.parent/'RETENTION.md' if name=='RETENTION.md'
              else Path(REFERENCE_PINS[name]['path']) if name in REFERENCE_PINS else ROOT/name)
        guard();require(read(path)==body,'all selected original posthashes')
    for case in outcomes:
        require(roster(ROOT/case)=={n[len(case)+1:] for n in bodies if n.startswith(case+'/')}, 'final case roster unchanged')
    raw=read_archive(output)
    print(json.dumps(dict(archive=dict(path=str(output),**pin(raw)),members=len(bodies),
        expanded_bytes=sum(map(len,bodies.values())),all_cases_passed=result['all_cases_passed']),sort_keys=True))


def read_archive(path):
    guard()
    require(path.resolve(strict=True)==path and stat.S_ISREG(path.lstat().st_mode)
            and path.lstat().st_size<=MAX_TOTAL, 'bounded ordinary archive')
    before=path.lstat()
    with os.fdopen(os.open(path,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK),'rb') as stream:
        require(stamp(os.fstat(stream.fileno()))==stamp(before),'archive changed at open')
        raw=stream.read(MAX_TOTAL+1)
        require(stamp(os.fstat(stream.fileno()))==stamp(before),'archive changed while read')
    require(stamp(path.lstat())==stamp(before) and len(raw)==before.st_size,'archive changed')
    return raw


def retain(scope,outcomes,sha):
    require(re.fullmatch('[0-9a-f]{64}',sha),'observed archive SHA')
    archive=W/archive_name(scope);raw=read_archive(archive)
    require(pin(raw)['sha256']==sha,'observed original archive')
    bodies={};total=0
    with tarfile.open(fileobj=io.BytesIO(raw),mode='r:gz') as tar:
        for row in tar:
            guard();require(row.isfile() and not row.pax_headers and ordinary(row.name)
                and '\\' not in row.name and row.name not in bodies and len(bodies)<MAX_FILES
                and 0<=row.size<=MAX_BODY,'bounded ordinary unique archive member')
            total+=row.size;require(total<=MAX_TOTAL,'expanded archive cap')
            body=tar.extractfile(row).read(row.size+1);require(len(body)==row.size,'complete member')
            bodies[row.name]=body
    manifest_raw=bodies.pop('manifest.json');manifest=parse(manifest_raw)
    require(manifest['schema']=='ferric-warm-paired-terminal-native-export-v1'
            and manifest['outcomes']==outcomes
            and manifest['files']=={n:pin(b) for n,b in sorted(bodies.items())},'all original archive pins')
    checked=verify(bodies,outcomes)
    require(same(checked,manifest['verification']),'same original verification')
    destination=DEST/('native-'+scope+'-v1')
    require(destination.parent.resolve(strict=True)==destination.parent and not os.path.lexists(destination),'fresh retention destination')
    require(read_archive(archive)==raw,'archive prepublication posthash')
    bodies['manifest.json']=manifest_raw
    destination.mkdir(mode=0o755)
    for name,body in sorted(bodies.items()):
        guard();path=destination/name;path.parent.mkdir(parents=True,exist_ok=True)
        with path.open('xb') as stream:stream.write(body);stream.flush();os.fsync(stream.fileno())
        path.chmod(0o644);require(read(path)==body,'retained original readback')
    require(read_archive(archive)==raw,'archive final posthash')
    receipt=dict(schema='ferric-warm-paired-terminal-native-retention-v1',archive=pin(raw),
        original_files=len(bodies),files={n:pin(b) for n,b in sorted(bodies.items())},
        verification=checked,original_terminals_unchanged=True,new_native_execution=False,
        external_model_or_elf_bodies_rehashed=False,numerical_acceptance=False,performance_claim=False)
    with (destination/'retention.json').open('xb') as stream:
        stream.write(encoded(receipt));stream.flush();os.fsync(stream.fileno())
    print(json.dumps(dict(destination=str(destination),original_files=len(bodies),
                         all_cases_passed=checked['all_cases_passed']),sort_keys=True))


def main():
    global DEADLINE
    require(__debug__ and sys.dont_write_bytecode,'isolated Python with bytecode disabled')
    DEADLINE=time.monotonic()+180
    for number in (signal.SIGINT,signal.SIGTERM,signal.SIGHUP,signal.SIGALRM):
        signal.signal(number,lambda n,_: (_ for _ in ()).throw(RuntimeError('retention signal '+str(n))))
    signal.setitimer(signal.ITIMER_REAL,180)
    resource.setrlimit(resource.RLIMIT_AS,(768<<20,768<<20))
    resource.setrlimit(resource.RLIMIT_CPU,(120,120))
    resource.setrlimit(resource.RLIMIT_FSIZE,(MAX_TOTAL,MAX_TOTAL))
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    try:
        require(len(sys.argv)>=5 and sys.argv[1] in ('export','retain'),'closed data-only retention CLI')
        if sys.argv[1]=='export':
            scope,outcomes=specs(sys.argv[2:]);export(scope,outcomes)
        else:
            scope,outcomes=specs(sys.argv[3:]);retain(scope,outcomes,sys.argv[2])
    finally:
        signal.setitimer(signal.ITIMER_REAL,0)


if __name__=='__main__':
    main()
