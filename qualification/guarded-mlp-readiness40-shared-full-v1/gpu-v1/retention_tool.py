"""Bounded export/retention of original Readiness40 SharedFull policy evidence; no GPU or model replay."""
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import struct
import sys
import tarfile
import time
import types

REMOTE = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-shared-full-gpu-v228-v1')
CASE = REMOTE / 'readiness'
BASELINE_ROOT = REMOTE.parent / 'guarded-mlp-readiness40-position5-gpu-v228-v1'
BASELINE = dict(path=str(BASELINE_ROOT / 'readiness/complete.json'), bytes=171456,
    sha256='5b9617aba0b588c33923c5cc458b013d0929828be5635ea75f3fd702a12c2cd9')
ROOT_PINS = {
    "frozen_owned.py": {
        "bytes": 30433,
        "sha256": "ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583"
    },
    "library_audit.py": {
        "bytes": 25872,
        "sha256": "b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d"
    },
    "validate_readiness.py": {
        "bytes": 19800,
        "sha256": "0c319e99142b19909350d9a81404948b494c2e3ea0defbbc165c2e5529be032a"
    },
    "readiness_announcement.py": {
        "bytes": 3246,
        "sha256": "b974ac6b6e936d8239639ac0c595c7db36700e3b1e2cff101224699fd789a9b1"
    },
    "validate_shared.py": {
        "bytes": 8015,
        "sha256": "6557fe5c082b2c92bae15274dd0d19bba3da8c3c36b9fc74757b4c1b74a87eca"
    },
    "run_model_gpu.py": {
        "bytes": 38455,
        "sha256": "07db0d2f819d98c447e8a0dfc572d070bab42aae9325b44feb8a4a1cc0287105"
    },
    "prepare_model_inputs.py": {
        "bytes": 16438,
        "sha256": "aa7b4f59d3b2c1f18deb43015bbfde9ff94f4b5eaf9f32fd233da65198feb19a"
    },
    "readiness-input.json": {
        "bytes": 1651,
        "sha256": "c3a0914e49f73eed73f9f4910a0198b4dd69fe3ec65a135cec7d843f95e59421"
    },
    "readiness-request.json": {
        "bytes": 9172,
        "sha256": "2c72171d03d4de579735687a7fd730f366bafc31e9cc44248031d26d8cc447db"
    },
    "prepared-inputs.json": {
        "bytes": 12949,
        "sha256": "210cbb65ece505b7e701ab5e082701e381ef9fc8ff6709553e7629ef76046d86"
    }
}
ADMISSION = {
    "worker": {
        "bytes": 6255224,
        "sha256": "9dbebbb014562f1bb704e8d504e46b8b719745abff5e0c7674af18412541bda8"
    },
    "worker_cpu": {
        "bytes": 1746409,
        "sha256": "4a016b7e09b0cc6f9b4bd32c98a5713f24709bd6509564a589c29b0478f7e337"
    },
    "parent": {
        "bytes": 13852264,
        "sha256": "682d5455b9c5966fa6a3ad02f0e755117b7d7e69241ead372ee22fbd703de804"
    },
    "parent_cpu": {
        "bytes": 3983617,
        "sha256": "7e4ad4324ea3b0302e1f7cf738b699e0e0721025963e659f0ba7f5bc35d70a8d"
    }
}
CHECKER = {
    "path": "/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-shared-full-checker-cpu-v228-v1/evidence/complete.json",
    "bytes": 13382,
    "sha256": "7a9029ae0e864809fa98062f4f15a13c954ce67d8b1d20e8c2cb36baa888850c"
}
LABELS = ('parent-readelf', 'parent-ldd', 'worker-readelf', 'worker-ldd',
          'before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2')
BODY_CAP, CASE_CAP, TOTAL_CAP, FILE_CAP = 8 << 20, 64 << 20, 72 << 20, 342
DEADLINE = float('inf')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def tick():
    require(time.monotonic() < DEADLINE, 'data-only whole deadline')


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def compact(value):
    return {key: value[key] for key in ('bytes', 'sha256')}


def same(a, b):
    return json.dumps(a, sort_keys=True, separators=(',', ':'), allow_nan=False) == json.dumps(
        b, sort_keys=True, separators=(',', ':'), allow_nan=False)


def parse(raw):
    def pairs(rows):
        out = {}
        for key, value in rows:
            require(key not in out, 'duplicate JSON field')
            out[key] = value
        return out
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def ordinary(name):
    return (type(name) is str and name not in ('', '.') and not Path(name).is_absolute()
            and Path(name).as_posix() == name and '..' not in Path(name).parts)


def stamp(row):
    return (row.st_dev, row.st_ino, row.st_mode, row.st_nlink, row.st_size, row.st_mtime_ns, row.st_ctime_ns)


def read(path, expected=None, limit=BODY_CAP):
    tick()
    path = Path(path); before = path.lstat()
    require(path.is_absolute() and path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and 1 <= before.st_nlink and 0 <= before.st_size <= limit, 'ordinary bounded immutable input: ' + str(path))
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    with os.fdopen(fd, 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed at open')
        body = stream.read(limit + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed during read')
    require(stamp(path.lstat()) == stamp(before) and len(body) == before.st_size
            and (expected is None or pin(body) == compact(expected)), 'input pin/stamp drift: ' + str(path))
    tick()
    return body


def roster(root, entry_cap=256):
    require(root.resolve(strict=True) == root and root.is_dir(), 'canonical source directory')
    result = set(); entries = 0
    for parent, dirs, names in os.walk(root, followlinks=False,
            onerror=lambda e: (_ for _ in ()).throw(e)):
        tick()
        for name in dirs + names:
            path = Path(parent) / name; row = path.lstat(); entries += 1
            require(path.resolve(strict=True) == path
                    and (stat.S_ISDIR(row.st_mode) if name in dirs else stat.S_ISREG(row.st_mode)),
                    'ordinary directory/member')
        for name in names:
            relative = str((Path(parent) / name).relative_to(root))
            require(ordinary(relative) and relative not in result, 'unique ordinary member')
            result.add(relative)
        require(entries <= entry_cap, 'bounded source entry cap')
    return result


def module(body, expected, name):
    require(pin(body) == expected, 'exact qualified pure validation body')
    result = types.ModuleType(name)
    result.__file__ = '<authenticated-data-only-' + name + '>'
    exec(compile(body, result.__file__, 'exec'), result.__dict__)
    return result


def success_names():
    names = {label + '/' + name for label in LABELS for name in
             ('command.json', 'started.json', 'stdout', 'stderr', 'result.json')}
    names |= {'initial-topology.json', 'observation.json', 'native/complete.json',
              'native/frames.ndjson', 'native/child-stderr.bin'}
    names |= {side + '-' + str(i) + '-topology.json' for side in ('before', 'after') for i in range(3)}
    names |= {'native/capture-%d.bin' % i for i in (0, 5, 16, 39)}
    names |= {'parity.json'}
    require(len(names) == 71, 'fixed successful shared-full raw census')
    return names



def baseline_names():
    return {'readiness/' + name for name in success_names() - {'parity.json'}} | {
        'readiness/complete.json', 'readiness-input.json', 'readiness-request.json'}


def verify_baseline(bodies, current):
    selected = {name[len('baseline/'):]: body for name, body in bodies.items() if name.startswith('baseline/')}
    require(set(selected) == baseline_names() and len(selected) == 73, 'closed original baseline bodies')
    terminal_raw = selected['readiness/complete.json']
    require(pin(terminal_raw) == compact(BASELINE), 'actual prior position5 terminal bytes')
    baseline = parse(terminal_raw)
    require(baseline['schema'] == 'ferric-guarded-mlp-readiness40-position5-gpu-v1'
            and baseline['passed'] is True and baseline['errors'] == []
            and baseline['native_attempts'] == 1 and baseline['retries'] == 0
            and baseline['owned_worker_lineage_verified'] is True
            and baseline['default_full_currentness_requested'] is True
            and all(baseline[k] is False for k in ('shared_full_currentness_requested', 'paired_read_requested',
                'full_model_acceptance', 'numerical_acceptance', 'performance_claim', 'production_authority',
                'full_long_workload')), 'fixed successful historical baseline')
    require(set(baseline['raw']) == success_names() - {'parity.json'}
            and [p['label'] for p in baseline['phases']] == list(LABELS), 'closed original baseline raw and leaves')
    for name, expected in baseline['raw'].items():
        require(expected['path'] == str(BASELINE_ROOT / 'readiness' / name)
                and pin(selected['readiness/' + name]) == compact(expected)
                and current['readset'][expected['path']] == expected, 'all baseline raw/current readset pins')
    require(current['readset'][BASELINE['path']] == BASELINE, 'baseline original terminal in current readset')
    plan_pin = baseline['plan']; plan = parse(selected['readiness-input.json'])
    require(plan_pin['path'] == str(BASELINE_ROOT / 'readiness-input.json')
            and pin(selected['readiness-input.json']) == compact(plan_pin)
            and current['readset'][plan_pin['path']] == plan_pin, 'baseline actual plan pin')
    request_pin = plan['request']; request = parse(selected['readiness-request.json'])
    require(request_pin['path'] == str(BASELINE_ROOT / 'readiness-request.json')
            and pin(selected['readiness-request.json']) == compact(request_pin)
            and current['readset'][request_pin['path']] == request_pin, 'baseline actual request pin')
    for phase in baseline['phases']:
        label = phase['label']
        require(type(phase['exit_code']) is int and phase['exit_code'] == 0 and phase['reason'] is None
                and phase['cleanup_signalled'] is False and phase['owned_groups_absent'] is True
                and phase['owned_processes_reaped'] is True, 'original baseline natural clean owned leaf')
        require(same(parse(selected['readiness/' + label + '/result.json']),
                     {k: v for k, v in phase.items() if k != 'label'}), 'baseline original leaf result')
        for key, suffix in [('command', 'command.json'), ('started', 'started.json'),
                            ('stdout', 'stdout'), ('stderr', 'stderr')]:
            require(phase[key] == baseline['raw'][label + '/' + suffix], 'baseline leaf pin')
    initial = parse(selected['readiness/initial-topology.json'])
    require(same(initial, baseline['platform']), 'baseline original topology')
    for label in [name for name in LABELS if name.startswith(('before-', 'after-'))]:
        require(same(parse(selected['readiness/' + label + '-topology.json']), initial), 'baseline six stable topology bodies')
        idle = parse(selected['readiness/' + label + '/stdout'])
        require(type(idle) is list and len(idle) == 8
                and all(set(row) == {'gpu', 'process_list'} and type(row['gpu']) is int for row in idle)
                and {row['gpu'] for row in idle} == set(range(8))
                and all(row['process_list'] == [{'process_info': 'No running processes detected'}] for row in idle),
                'baseline six all-eight-device idle observations')
    validator = module(bodies['validate_readiness.py'], ROOT_PINS['validate_readiness.py'], 'original_position5_data')
    marker = module(bodies['readiness_announcement.py'], ROOT_PINS['readiness_announcement.py'], 'original_lineage')
    summary_raw = selected['readiness/native/complete.json']; summary = parse(summary_raw)
    require(same(summary, parse(selected['readiness/parent/stdout'])), 'baseline parent stdout/summary')
    prompt = summary['bootstrap']['sequence']['prompt_tokens']
    require(type(prompt) is list and len(prompt) == 2048
            and all(type(v) is int and 0 <= v < 151936 for v in prompt)
            and hashlib.sha256(struct.pack('<2048I', *prompt)).hexdigest() ==
                '2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02',
            'original authentic2048 prompt bits')
    def read_body(row):
        path = Path(row['path'])
        require(path.is_absolute() and path.is_relative_to(BASELINE_ROOT / 'readiness/native'), 'baseline native-only body')
        name = str(path.relative_to(BASELINE_ROOT))
        require(ordinary(name) and name in selected and pin(selected[name]) == compact(row), 'baseline original body join')
        return selected[name]
    checked = validator.validate(summary_raw, request, BASELINE_ROOT / 'readiness/native', read_body, prompt)
    require(same(checked, baseline['observation']) and same(checked, parse(selected['readiness/observation.json'])),
            'same independently repeated original position5 admission')
    native = next(row for row in baseline['phases'] if row['label'] == 'parent')
    marker.validate_lineage(selected['readiness/parent/stderr'], native,
                            parse(selected['readiness/parent/started.json']), checked['child_pid'])
    return summary_raw, checked


def verify(bodies, terminal_name, terminal_sha):
    tick()
    require(all(type(v) is dict and set(v) == {'bytes', 'sha256'} for v in
                [*ROOT_PINS.values(), *ADMISSION.values()])
            and type(CHECKER) is dict and set(CHECKER) == {'path', 'bytes', 'sha256'},
            'all actual source/input/CPU/product/checker bindings required')
    require(terminal_name in ('complete.json', 'failed.json')
            and re.fullmatch('[0-9a-f]{64}', terminal_sha), 'explicit observed terminal binding')
    require(all(ordinary(name) and type(body) is bytes and len(body) <= BODY_CAP
                and not body.startswith(b'\x7fELF') for name, body in bodies.items()), 'bounded nonbinary bodies only')
    require(len(bodies) <= FILE_CAP and sum(map(len, bodies.values())) <= TOTAL_CAP, 'capsule extent bound')
    for name, expected in ROOT_PINS.items():
        require(name in bodies and pin(bodies[name]) == expected, 'exact deployed/prepared root body: ' + name)
    own = read(Path(__file__).resolve())
    require(bodies.get('retention_tool.py') == own, 'same reviewed retention tool')
    case_names = {name[len('readiness/'):] for name in bodies if name.startswith('readiness/')}
    require(set(bodies) == set(ROOT_PINS) | {'retention_tool.py'} | {'readiness/' + n for n in case_names}
            | {'baseline/' + n for n in baseline_names()},
            'closed root/case/tool selection; no external model or executable bodies')
    require(terminal_name in case_names and ({'complete.json', 'failed.json'} & case_names) == {terminal_name},
            'one original terminal, failure not replaced')
    require(len(case_names) <= 256 and sum(len(bodies['readiness/' + n]) for n in case_names) <= CASE_CAP,
            'original case member/64MiB cap')
    terminal_body = bodies['readiness/' + terminal_name]
    require(pin(terminal_body)['sha256'] == terminal_sha, 'observed original terminal hash')
    t = parse(terminal_body)
    require(t['schema'] == 'ferric-guarded-mlp-readiness40-shared-full-gpu-v1' and t['case'] == 'readiness'
            and t['profile'] == 'readiness40_position5' and type(t['passed']) is bool
            and terminal_name == ('complete.json' if t['passed'] else 'failed.json')
            and type(t['errors']) is list and (t['errors'] == [] if t['passed'] else bool(t['errors']))
            and type(t['native_attempts']) is int and t['native_attempts'] in (0, 1) and t['retries'] == 0,
            'original one-shot readiness outcome')
    require(all(t[k] is False for k in ('full_model_acceptance', 'numerical_acceptance', 'performance_claim',
        'production_authority', 'full_long_workload', 'full2303_native_enabled',
        'independent_full_model_reference', 'ar4_parity_transferred', 'default_full_currentness_requested',
        'paired_read_requested', 'outer_model_shards_rehashed', 'gpu_timing', 'parent_host_timing_requested',
        'host_observer_requested', 'paired_terminal_requested', 'cache_kernel_admission_requested', 'operational_currentness_requested'))
        and t['shared_full_currentness_requested'] is True
        and t['prompt_tokens'] == 2048 and t['prompt_positions_requested'] == 40
        and t['generated_tokens_requested'] == 0 and t['capture_positions'] == [0, 5, 16, 39]
        and t['allocation_pages'] == 144
        and t['artifact_contains_full2303_route'] is True
        and t['baseline'] == BASELINE, 'closed scope and nonclaims')
    require(t['limits'] == dict(native_seconds=4000, whole_seconds=4300, address_space_bytes=32 << 30,
        stream_bytes=8 << 20, case_bytes=64 << 20, case_members=256, affinity=[8, 9]), 'unchanged case bounds')
    require(t['controller'] == dict(path=str(REMOTE / 'run_model_gpu.py'), **ROOT_PINS['run_model_gpu.py']),
            'exact actual controller')
    require(set(t['raw']) == case_names - {terminal_name}, 'all original raw bodies retained')
    for name, expected in t['raw'].items():
        require(ordinary(name) and expected['path'] == str(CASE / name)
                and pin(bodies['readiness/' + name]) == compact(expected), 'original raw pin: ' + name)
    plan = parse(bodies['readiness-input.json']); request = parse(bodies['readiness-request.json'])
    prepared = parse(bodies['prepared-inputs.json'])
    require(plan['schema'] == 'ferric-guarded-mlp-readiness40-shared-full-gpu-input-v1'
            and plan['case'] == 'readiness' and plan['profile'] == 'readiness40_position5'
            and t['plan'] == dict(path=str(REMOTE / 'readiness-input.json'), **ROOT_PINS['readiness-input.json'])
            and plan['request'] == dict(path=str(REMOTE / 'readiness-request.json'), **ROOT_PINS['readiness-request.json'])
            and request['schema'] == 'FerricGuardedMlpReadiness40Position5RequestV1'
            and request['base']['evidence_directory'] == str(CASE / 'native'), 'exact prepared readiness request')
    require(prepared['passed'] is True and prepared['native_execution'] is False
            and prepared['gpu_execution'] is False and prepared['full_long_workload'] is False
            and prepared['parent_host_timing_requested'] is False and prepared['gpu_timing'] is False
            and prepared['shared_full_currentness_requested'] is True
            and prepared['cache_kernel_admission_requested'] is False
            and prepared['operational_currentness_requested'] is False
            and prepared['ordinary_wire_schema_changed'] is False
            and prepared['ordinary_observation_schema_changed'] is False
            and prepared['plan'] == t['plan'] and prepared['request'] == plan['request']
            and prepared['controller'] == dict(path=str(REMOTE / 'prepare_model_inputs.py'), **ROOT_PINS['prepare_model_inputs.py']),
            'actual data-only preparation joins')
    require(set(t['admission']) == set(ADMISSION), 'closed CPU/product metadata')
    for name, expected in ADMISSION.items():
        require(compact(plan[name]) == expected and plan[name] == t['admission'][name]
                and t['readset'][plan[name]['path']] == plan[name], 'actual qualified CPU/product reference')
    require(t['checker_cpu'] == CHECKER and t['readset'][CHECKER['path']] == CHECKER,
            'actual twenty-nine-test pure checker reference')
    root_inputs = {name: row for name, row in t['readset'].items() if name.startswith(str(REMOTE) + '/')}
    required_inputs = set(ROOT_PINS) - {'prepare_model_inputs.py', 'prepared-inputs.json'}
    require(set(root_inputs) == {str(REMOTE / n) for n in required_inputs}, 'exact eight current root readset bodies')
    for name in required_inputs:
        require(root_inputs[str(REMOTE / name)] == dict(path=str(REMOTE / name), **ROOT_PINS[name]), 'root readset pin')
    for row in t['readset'].values():
        require(type(row) is dict and set(row) == {'path', 'bytes', 'sha256'}
                and type(row['bytes']) is int and row['bytes'] >= 0
                and re.fullmatch('[0-9a-f]{64}', row['sha256']), 'closed metadata-only external readset')
    labels = [row['label'] for row in t['phases']]
    require(len(labels) == len(set(labels)) <= 11 and all(n in LABELS for n in labels)
            and labels == sorted(labels, key=LABELS.index), 'ordered original phase prefix')
    for row in t['phases']:
        label = row['label']
        for key, suffix in [('command', 'command.json'), ('started', 'started.json'),
                            ('stdout', 'stdout'), ('stderr', 'stderr')]:
            if row[key] is not None:
                require(t['raw'][label + '/' + suffix] == row[key], 'phase original pin')
        require(same(parse(bodies['readiness/' + label + '/result.json']),
            {k: v for k, v in row.items() if k != 'label'}), 'phase original result')
        command = parse(bodies['readiness/' + label + '/command.json'])
        require(command['cwd'] == str(REMOTE.parent) and command['affinity'] == [8, 9] and command['nice'] == 10
                and command['stream_cap_bytes'] == 8 << 20 and command['file_cap_bytes'] == 64 << 20,
                'original leaf resource bounds')
        expected_argv = ([plan['parent']['path'], '--request', plan['request']['path'],
            '--observe-guarded-readiness40-position5-shared-full', '--allow-unauthenticated-machine-code'] if label == 'parent' else
            ['/opt/rocm/bin/amd-smi', 'process', '--json'] if label.startswith(('before-', 'after-')) else
            ['/usr/bin/readelf', '-l', '-d', plan[label.split('-')[0]]['path']] if label.endswith('-readelf') else
            ['/usr/bin/ldd', plan[label.split('-')[0]]['path']])
        require(command['argv'] == expected_argv, 'actual native/audit argv')
    baseline_raw, baseline_checked = verify_baseline(bodies, t)
    confirmed = None; parity = None; shared_checked = None
    if t['passed']:
        require(labels == list(LABELS) and set(t['raw']) == success_names()
                and t['native_attempts'] == 1 and t['owned_worker_lineage_verified'] is True
                and all(t[k] is True for k in ('native_spawn_observed', 'gpu_execution_requested',
                    'gpu_execution', 'gpu_execution_confirmed', 'model_source_authenticated_by_qualified_parent'))
                and t['partial_gpu_execution_possible'] is False
                and type(t['shared_policy']) is dict, 'successful closed execution scope')
        for row in t['phases']:
            require(type(row['exit_code']) is int and row['exit_code'] == 0 and row['reason'] is None
                    and row['cleanup_signalled'] is False and row['owned_groups_absent'] is True
                    and row['owned_processes_reaped'] is True, 'natural successful owned phase')
        initial = parse(bodies['readiness/initial-topology.json'])
        require(same(initial, t['platform']) and initial['host'] == 'smci350-rck-g03-b19-03'
                and re.fullmatch('[0-9a-f-]{36}', initial['boot'])
                and len(initial['devices']) == 2, 'recorded selected host/topology')
        for row, node, identity in zip(initial['devices'], (2, 3),
                (16366993098680759275, 10838076764495710945)):
            require(row['node'] == node and type(row['gpu_id']) is int and row['gpu_id'] > 0
                    and int(row['properties']['unique_id']) == identity
                    and int(row['properties']['gfx_target_version']) == 90500, 'recorded gfx950 unique identity')
        for label in [n for n in LABELS if n.startswith(('before-', 'after-'))]:
            require(same(parse(bodies['readiness/' + label + '-topology.json']), initial), 'six stable topology snapshots')
            idle = parse(bodies['readiness/' + label + '/stdout'])
            require(type(idle) is list and len(idle) == 8
                    and all(set(r) == {'gpu', 'process_list'} and type(r['gpu']) is int for r in idle)
                    and {r['gpu'] for r in idle} == set(range(8))
                    and all(r['process_list'] == [{'process_info': 'No running processes detected'}] for r in idle),
                    'six original all-eight-device idle observations')
        validator = module(bodies['validate_readiness.py'], ROOT_PINS['validate_readiness.py'], 'ordinary_position5_data')
        prior_module = sys.modules.get('validate_readiness')
        sys.modules['validate_readiness'] = validator
        try:
            shared = module(bodies['validate_shared.py'], ROOT_PINS['validate_shared.py'], 'readiness_shared_full_data')
        finally:
            if prior_module is None: sys.modules.pop('validate_readiness', None)
            else: sys.modules['validate_readiness'] = prior_module
        guard = module(bodies['readiness_announcement.py'], ROOT_PINS['readiness_announcement.py'], 'readiness_lineage')
        summary_raw = bodies['readiness/native/complete.json']; summary = parse(summary_raw)
        prompt = summary['bootstrap']['sequence']['prompt_tokens']
        require(type(prompt) is list and len(prompt) == 2048
                and all(type(v) is int and 0 <= v < 151936 for v in prompt)
                and hashlib.sha256(struct.pack('<2048I', *prompt)).hexdigest() ==
                '2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02',
                'all2048 original authentic prompt bits without copying model bulk')
        def read_body(row):
            path = Path(row['path'])
            if path.is_relative_to(CASE / 'native'):
                name = 'readiness/' + str(path.relative_to(CASE))
            else:
                require(path.is_relative_to(BASELINE_ROOT / 'readiness/native'), 'retained two-run native-only read closure')
                name = 'baseline/' + str(path.relative_to(BASELINE_ROOT))
            require(path.is_absolute() and ordinary(name) and name in bodies
                    and pin(bodies[name]) == compact(row), 'retained native body join')
            return bodies[name]
        confirmed, shared_checked = shared.validate(bodies['readiness/parent/stdout'], summary_raw,
                                                   request, CASE / 'native', read_body, prompt)
        require(same(confirmed, t['observation']) and same(confirmed, parse(bodies['readiness/observation.json'])),
                'same independently checked40 transcript/four captures')
        require(same(shared_checked, t['shared_policy']), 'same original policy bytes and qualified shared admission')
        native = next(row for row in t['phases'] if row['label'] == 'parent')
        started = parse(bodies['readiness/parent/started.json'])
        guard.validate_lineage(bodies['readiness/parent/stderr'], native, started, confirmed['child_pid'])
        require(t['native_started'] == t['raw']['parent/started.json'], 'recorded native registration')
        parity = shared.compare_same_side(summary_raw, baseline_raw, confirmed, baseline_checked, read_body)
        require(same(parity, t['same_side_parity']) and same(parity, parse(bodies['readiness/parity.json'])),
                'same independently revalidated semantic and full-payload parity')
    tick()
    return dict(original_passed=t['passed'], original_terminal=dict(name=terminal_name, **pin(terminal_body)),
        original_raw_files=len(t['raw']), selected_original_bodies=len(bodies),
        retained_success_revalidated=confirmed is not None,
        readiness_completed_forwards=confirmed['completed_forwards'] if confirmed else None,
        cpu_and_elf_metadata=ADMISSION, checker_cpu=CHECKER,
        historical_baseline=BASELINE, baseline_raw_files=70, baseline_data_revalidated=True,
        shared_full_policy_revalidated=shared_checked is not None,
        original_policy_bytes_retained=True, parent_host_measurement=False, gpu_timing=False,
        semantic_and_payload_parity_revalidated=parity is not None,
        numerical_reference_replayed=False,
        data_only=True, native_rerun=False, gpu_execution=False, model_execution=False,
        numerical_acceptance=False, full_long_workload=False, performance_claim=False, production_authority=False)


def export(terminal_name, terminal_sha, archive):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged export host')
    require(archive.is_absolute() and archive.parent.resolve(strict=True) == archive.parent
            and archive.parent != REMOTE and REMOTE not in archive.parents, 'archive outside original source tree')
    require({p.name for p in REMOTE.iterdir()} == set(ROOT_PINS) | {'readiness'}, 'closed original input/case root')
    names = roster(CASE); inputs = {n: REMOTE / n for n in ROOT_PINS}
    inputs.update({'readiness/' + n: CASE / n for n in names})
    inputs.update({'baseline/' + n: BASELINE_ROOT / n for n in baseline_names()})
    inputs['retention_tool.py'] = Path(__file__).resolve()
    bodies = {name: read(path, ROOT_PINS.get(name)) for name, path in inputs.items()}
    observation = verify(bodies, terminal_name, terminal_sha)
    manifest = dict(schema='ferric-guarded-mlp-readiness40-shared-full-retention-v1',
        source_root=str(REMOTE), files={n: pin(b) for n, b in sorted(bodies.items())},
        observation=observation, terminal_name=terminal_name, terminal_sha256=terminal_sha)
    for name, path in inputs.items(): require(read(path, pin(bodies[name])) == bodies[name], 'export posthash')
    require(roster(CASE) == names, 'source roster drift')
    all_bodies = dict(bodies, **{'manifest.json': (json.dumps(manifest, sort_keys=True, indent=2) + '\n').encode()})
    require(len(all_bodies) <= FILE_CAP and sum(map(len, all_bodies.values())) <= TOTAL_CAP, 'archive closed caps')
    partial = archive.with_name(archive.name + '.partial')
    require(not os.path.lexists(archive) and not os.path.lexists(partial), 'fresh archive, no overwrite')
    with partial.open('xb') as stream:
        with gzip.GzipFile(fileobj=stream, mode='wb', compresslevel=1, filename='', mtime=0) as gz:
            with tarfile.open(fileobj=gz, mode='w', format=tarfile.USTAR_FORMAT) as tar:
                for name, body in sorted(all_bodies.items()):
                    tick(); row = tarfile.TarInfo(name); row.size = len(body); row.mode = 0o600
                    tar.addfile(row, io.BytesIO(body))
        stream.flush(); os.fsync(stream.fileno())
    for name, path in inputs.items(): read(path, pin(bodies[name]))
    require(roster(CASE) == names, 'source roster final drift')
    archive_pin = pin(read(partial, limit=TOTAL_CAP))
    os.link(partial, archive); partial.unlink()
    return dict(archive=dict(path=str(archive), **archive_pin), members=len(all_bodies),
                expanded_bytes=sum(map(len, all_bodies.values())), **observation)


def retain(archive, archive_sha, destination):
    require(re.fullmatch('[0-9a-f]{64}', archive_sha), 'observed archive SHA')
    raw = read(archive, limit=TOTAL_CAP); require(pin(raw)['sha256'] == archive_sha, 'actual archive hash')
    bodies = {}; total = 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        for row in tar:
            tick()
            require(row.isfile() and ordinary(row.name) and row.name not in bodies
                    and not row.pax_headers and 0 <= row.size <= BODY_CAP, 'ordinary unique bounded tar body')
            total += row.size
            require(len(bodies) < FILE_CAP and total <= TOTAL_CAP, 'expanded archive cap')
            with tar.extractfile(row) as stream:
                body = stream.read(row.size + 1)
            require(len(body) == row.size, 'exact archive body extent')
            bodies[row.name] = body
    require('manifest.json' in bodies, 'closed capsule manifest')
    manifest_body = bodies.pop('manifest.json'); manifest = parse(manifest_body)
    require(set(manifest) == {'schema', 'source_root', 'files', 'observation', 'terminal_name', 'terminal_sha256'}
            and manifest['schema'] == 'ferric-guarded-mlp-readiness40-shared-full-retention-v1'
            and manifest['source_root'] == str(REMOTE)
            and set(manifest['files']) == set(bodies), 'closed manifest member map')
    for name, body in bodies.items(): require(pin(body) == manifest['files'][name], 'all archive member hashes')
    observation = verify(bodies, manifest['terminal_name'], manifest['terminal_sha256'])
    require(same(observation, manifest['observation']), 'unchanged independent data checks')
    require(destination.is_absolute() and destination.parent.resolve(strict=True) == destination.parent
            and not os.path.lexists(destination), 'fresh canonical retention target')
    destination.mkdir(mode=0o700)
    bodies['manifest.json'] = manifest_body
    for name, body in sorted(bodies.items()):
        tick(); target = destination / name; target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(body); stream.flush(); os.fsync(stream.fileno())
        require(read(target, pin(body)) == body, 'retained posthash')
    require(roster(destination, FILE_CAP + 32) == set(bodies), 'retained exact member closure')
    return dict(archive=dict(path=str(archive), **pin(raw)), destination=str(destination),
                members=len(bodies), expanded_bytes=sum(map(len, bodies.values())), **observation)


def main():
    global DEADLINE
    start = time.monotonic(); DEADLINE = start + 180
    require(__debug__ and sys.dont_write_bytecode, 'python3 -B data-only tool')
    os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 768 << 20), (resource.RLIMIT_FSIZE, TOTAL_CAP),
                       (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        cap = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (cap, cap))
    def stop(number, _frame): raise RuntimeError('data-only signal ' + str(number))
    for number in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP): signal.signal(number, stop)
    signal.setitimer(signal.ITIMER_REAL, 180)
    if len(sys.argv) == 5 and sys.argv[1] == 'export':
        result = export(sys.argv[2], sys.argv[3], Path(sys.argv[4]))
    elif len(sys.argv) == 5 and sys.argv[1] == 'retain':
        result = retain(Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]))
    else:
        raise RuntimeError('export complete.json|failed.json OBSERVED_TERMINAL_SHA ARCHIVE; or retain ARCHIVE OBSERVED_ARCHIVE_SHA FRESH_DIRECTORY')
    tick()
    print(json.dumps(result, sort_keys=True, allow_nan=False))
    signal.setitimer(signal.ITIMER_REAL, 0)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
