"""Bounded export/retention of original matched timed CensusV3/TailV4 Readiness40 evidence; no GPU or model replay."""
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

REMOTE = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-bank-scoped-census-tail-matched-timing-gpu-v228-v2')
WORKER_ROOT = REMOTE.parent / 'guarded-mlp-scoped-tail-cpu-v228-v2'
PARENT_ROOT = REMOTE.parent / 'guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-v228-v2'
ORDER = ('census', 'tail')
BASELINE_ROOT = REMOTE.parent / 'guarded-mlp-readiness40-position5-gpu-v228-v1'
BASELINE = dict(path=str(BASELINE_ROOT / 'readiness/complete.json'), bytes=171456,
    sha256='5b9617aba0b588c33923c5cc458b013d0929828be5635ea75f3fd702a12c2cd9')
ROOT_PINS = {
    "census-input.json": {"bytes":1685,"sha256":"ce4f889bb4c5514e32601ba85115bdef460c7215abae25c7fff689f5c1c71bf3"},
    "census-request.json": {"bytes":9169,"sha256":"df3020ea639bb99909d457bf2ebf726f1f1a67ff17218cbba751354b80121b1a"},
    "tail-input.json": {"bytes":1681,"sha256":"9c6eeb4259021879c82aae72e2c3f26359bbefc648f62829f3f24a739c49eefe"},
    "tail-request.json": {"bytes":9171,"sha256":"430c00268bd29dbd0861e0e75b5e4470f8041bc19849ec1d616f71f3864f0b11"},
    "prepared-inputs.json": {"bytes":15746,"sha256":"d024042be1fc79f3fa630751a06d692542484bf365d361c1d6479d75720aa5ec"},
    "run_model_gpu.py": {"bytes":60867,"sha256":"bba5e5a91609487cb28742a8fb8576567db6bda36bb7647b2791c791780911c4"},
    "prepare_model_inputs.py": {"bytes":22131,"sha256":"c3c13a7719bd7dc07488223ced65f4aaf1088316a7d7f3f7743b70c90073b4d3"},
    "frozen_owned.py": {
        "bytes": 30433,
        "sha256": "ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583"
    },
    "library_audit.py": {
        "bytes": 25872,
        "sha256": "b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d"
    },
    "readiness_announcement.py": {
        "bytes": 3246,
        "sha256": "b974ac6b6e936d8239639ac0c595c7db36700e3b1e2cff101224699fd789a9b1"
    },
    "validate_bank_scoped.py": {
        "bytes": 9534,
        "sha256": "2432fc14c0ba40fc934abcd7886459c12fc31ca6167ba4cf908a8c7974d26772"
    },
    "validate_census.py": {
        "bytes": 10392,
        "sha256": "090431e59a481d5b681311d5a015ddfd35705cc828c920118d4f0043b862ec9f"
    },
    "validate_matched.py": {
        "bytes": 9362,
        "sha256": "d31c94a4dd1f254d166be185fe1cca10bc4eed26e500a0cb1fc3c5a74a574437"
    },
    "validate_readiness.py": {
        "bytes": 19800,
        "sha256": "0c319e99142b19909350d9a81404948b494c2e3ea0defbbc165c2e5529be032a"
    },
    "validate_scoped.py": {
        "bytes": 9184,
        "sha256": "c2541e11e534cf1e5a4faade31fe4a734726552171fed9617d6ea938f680b6fd"
    },
    "validate_scoped_pair.py": {
        "bytes": 3233,
        "sha256": "495fc476454e4052467810b55e177f9d2b91d86d3496a6b90876963f7f60ac62"
    },
    "validate_shared.py": {
        "bytes": 8015,
        "sha256": "6557fe5c082b2c92bae15274dd0d19bba3da8c3c36b9fc74757b4c1b74a87eca"
    },
    "validate_tail.py": {
        "bytes": 10447,
        "sha256": "696e55d4bebea97ab82e5b613a76978c5e191087789d299551c304527e598d49"
    },
    "validate_tail_pair.py": {
        "bytes": 4883,
        "sha256": "ab69bcafcc245b8eca05d7e625c42eec7aaec2de4084e719090de1894f4d6995"
    },
    "validate_timing.py": {
        "bytes": 10104,
        "sha256": "27c652f5452676b4cc08a634241aff89b2653e85ee111133c1f1c4ab13efd93f"
    }
}
ADMISSION = {"worker":{"bytes":6506128,"sha256":"3b39b25d2363d656f2d75094faebd831b85ea544c4071332620e6019de5ef07d"},"worker_cpu":{"bytes":2448697,"sha256":"3c4fe3a6c4942b20ec61fd7ea5c3409526e2167c43d24a62fb8660c34b0e713e"},"parent":{"bytes":14246864,"sha256":"9cf24b52ffcb6b4d9cdeb6b58c3f97668f9ce20f550463d40451c21483a4e0b6"},"parent_cpu":{"bytes":4163125,"sha256":"a59c39cfa2be0ec11d0ffa6a352576814917243627ce9a73aca1d4f6501266e9"}}
CHECKER = {"path":"/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-bank-scoped-census-tail-checker-cpu-v228-v1/evidence/complete.json","bytes":33263,"sha256":"ecb5ea55c489d756614c91c7d394f08578c38342212c27c3be42c1bc2839472d"}
LABELS = ('parent-readelf', 'parent-ldd', 'worker-readelf', 'worker-ldd',
          'before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2')
BODY_CAP, CASE_CAP, TOTAL_CAP, FILE_CAP = 8 << 20, 64 << 20, 136 << 20, 601
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
                and (current is None or current['readset'][expected['path']] == expected), 'all baseline raw/current readset pins')
    require(current is None or current['readset'][BASELINE['path']] == BASELINE, 'baseline original terminal in current readset')
    plan_pin = baseline['plan']; plan = parse(selected['readiness-input.json'])
    require(plan_pin['path'] == str(BASELINE_ROOT / 'readiness-input.json')
            and pin(selected['readiness-input.json']) == compact(plan_pin)
            and (current is None or current['readset'][plan_pin['path']] == plan_pin), 'baseline actual plan pin')
    request_pin = plan['request']; request = parse(selected['readiness-request.json'])
    require(request_pin['path'] == str(BASELINE_ROOT / 'readiness-request.json')
            and pin(selected['readiness-request.json']) == compact(request_pin)
            and (current is None or current['readset'][request_pin['path']] == request_pin), 'baseline actual request pin')
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



def matched_names(case):
    require(case in ORDER, 'closed matched mode')
    names = success_names() | {'native/host-timing.json', 'matched.json'}
    if case == 'tail': names.add('pair.json')
    require(len(names) == (73 if case == 'census' else 74), 'fixed matched raw census')
    return names


def validation_modules(bodies):
    names = ('validate_readiness', 'validate_shared', 'validate_timing', 'validate_matched', 'validate_scoped', 'validate_bank_scoped', 'validate_census', 'validate_tail')
    prior = {name: sys.modules.get(name) for name in names}
    try:
        validator = module(bodies['validate_readiness.py'], ROOT_PINS['validate_readiness.py'], 'validate_readiness')
        sys.modules['validate_readiness'] = validator
        shared = module(bodies['validate_shared.py'], ROOT_PINS['validate_shared.py'], 'validate_shared')
        sys.modules['validate_shared'] = shared
        timing = module(bodies['validate_timing.py'], ROOT_PINS['validate_timing.py'], 'validate_timing')
        sys.modules['validate_timing'] = timing
        matched = module(bodies['validate_matched.py'], ROOT_PINS['validate_matched.py'], 'validate_matched')
        sys.modules['validate_matched'] = matched
        scoped = module(bodies['validate_scoped.py'], ROOT_PINS['validate_scoped.py'], 'validate_scoped')
        sys.modules['validate_scoped'] = scoped
        bank = module(bodies['validate_bank_scoped.py'], ROOT_PINS['validate_bank_scoped.py'], 'validate_bank_scoped')
        sys.modules['validate_bank_scoped'] = bank
        census = module(bodies['validate_census.py'], ROOT_PINS['validate_census.py'], 'validate_census')
        sys.modules['validate_census'] = census
        tail = module(bodies['validate_tail.py'], ROOT_PINS['validate_tail.py'], 'validate_tail')
        sys.modules['validate_tail'] = tail
        pair = module(bodies['validate_tail_pair.py'], ROOT_PINS['validate_tail_pair.py'], 'validate_tail_pair')
        return validator, shared, timing, matched, scoped, bank, census, tail, pair
    finally:
        for name, original in prior.items():
            if original is None: sys.modules.pop(name, None)
            else: sys.modules[name] = original


def native_body(bodies, row):
    path = Path(row['path'])
    require(path.is_absolute(), 'absolute original native body')
    for case in ORDER:
        if path.is_relative_to(REMOTE / case / 'native'):
            name = str(path.relative_to(REMOTE))
            break
    else:
        require(path.is_relative_to(BASELINE_ROOT / 'readiness/native'), 'retained native-only read closure')
        name = 'baseline/' + str(path.relative_to(BASELINE_ROOT))
    require(ordinary(name) and name in bodies and pin(bodies[name]) == compact(row), 'retained native body join')
    return bodies[name]


def verify_case(bodies, case, outcome):
    CASE = REMOTE / case
    terminal_name, terminal_sha = outcome['name'], outcome['sha256']
    case_names = {name[len(case) + 1:] for name in bodies if name.startswith(case + '/')}
    require(terminal_name in case_names and ({'complete.json', 'failed.json'} & case_names) == {terminal_name},
            'one original terminal, failure not replaced')
    require(len(case_names) <= 256 and sum(len(bodies[case + '/' + n]) for n in case_names) <= CASE_CAP,
            'original per-case member/64MiB cap')
    terminal_body = bodies[case + '/' + terminal_name]
    require(pin(terminal_body)['sha256'] == terminal_sha, 'observed original terminal hash')
    t = parse(terminal_body)
    require(t['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-tail-matched-timing-gpu-v1' and t['case'] == case
            and t['profile'] == 'readiness40_position5' and type(t['passed']) is bool
            and terminal_name == ('complete.json' if t['passed'] else 'failed.json')
            and type(t['errors']) is list and (t['errors'] == [] if t['passed'] else bool(t['errors']))
            and type(t['native_attempts']) is int and t['native_attempts'] in (0, 1) and t['retries'] == 0,
            'original one-shot readiness outcome')
    require(all(t[k] is False for k in ('full_model_acceptance', 'numerical_acceptance', 'performance_claim',
        'production_authority', 'full_long_workload', 'full2303_native_enabled',
        'independent_full_model_reference', 'ar4_parity_transferred',
        'paired_read_requested', 'outer_model_shards_rehashed', 'gpu_timing',
        'host_observer_requested', 'paired_terminal_requested', 'cache_kernel_admission_requested', 'operational_currentness_requested'))
        and t['shared_full_currentness_requested'] is False
        and t['scoped_warm_currentness_requested'] is True
        and t['bank_scoped_rearm_requested'] is True
        and t['scoped_capacity_census_requested'] is True
        and t['allocation_preflights_changed'] is True
        and t['census_counters_are_layer_subset'] is True
        and t['scoped_tail_requested'] is (case == 'tail')
        and t['tail_counters_are_independent'] is (case == 'tail')
        and t['currentness_temporal_equivalence_claim'] is False
        and t['default_full_currentness_requested'] is False
        and t['parent_host_timing_requested'] is True
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
                and pin(bodies[case + '/' + name]) == compact(expected), 'original raw pin: ' + name)
    plan = parse(bodies[case + '-input.json']); request = parse(bodies[case + '-request.json'])
    prepared = parse(bodies['prepared-inputs.json'])
    require(plan['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-tail-matched-timing-gpu-input-v1'
            and plan['case'] == case and plan['profile'] == 'readiness40_position5'
            and t['plan'] == dict(path=str(REMOTE / (case + '-input.json')), **ROOT_PINS[case + '-input.json'])
            and plan['request'] == dict(path=str(REMOTE / (case + '-request.json')), **ROOT_PINS[case + '-request.json'])
            and request['schema'] == 'FerricGuardedMlpReadiness40Position5RequestV1'
            and request['base']['evidence_directory'] == str(CASE / 'native'), 'exact prepared readiness request')
    require(prepared['passed'] is True and prepared['native_execution'] is False
            and prepared['gpu_execution'] is False and prepared['full_long_workload'] is False
            and prepared['parent_host_timing_requested'] is True and prepared['gpu_timing'] is False
            and prepared['modes'] == list(ORDER)
            and prepared['currentness_policies'] == dict(census='bank_scoped_census', tail='bank_scoped_census_tail')
            and prepared['temporal_equivalent_to_full'] is False
            and prepared['allocation_preflights_changed'] is True
            and prepared['allocation_preflights_changed_only_for_candidate'] is False
            and prepared['scoped_tail_requested_only_for_candidate'] is True
            and prepared['tail_counters_are_independent'] is True
            and prepared['census_counters_are_layer_subset'] is True
            and prepared['shared_full_currentness_requested'] is False
            and prepared['identical_parent_and_worker_products'] is True
            and prepared['fresh_case_namespaces'] is True
            and prepared['cache_kernel_admission_requested'] is False
            and prepared['operational_currentness_requested'] is False
            and prepared['ordinary_wire_schema_changed'] is False
            and prepared['ordinary_observation_schema_changed'] is False
            and prepared['plans'][case] == t['plan'] and prepared['requests'][case] == plan['request']
            and prepared['controller'] == dict(path=str(REMOTE / 'prepare_model_inputs.py'), **ROOT_PINS['prepare_model_inputs.py']),
            'actual data-only preparation joins')
    require(plan['worker_cpu']['path'] == str(WORKER_ROOT / 'evidence/complete.json')
            and plan['parent_cpu']['path'] == str(PARENT_ROOT / 'evidence/complete.json'),
            'actual successful census coupled and parent qualification roots')
    require(set(t['admission']) == set(ADMISSION), 'closed CPU/product metadata')
    for name, expected in ADMISSION.items():
        require(compact(plan[name]) == expected and plan[name] == t['admission'][name]
                and t['readset'][plan[name]['path']] == plan[name], 'actual qualified CPU/product reference')
    require(t['checker_cpu'] == CHECKER and t['readset'][CHECKER['path']] == CHECKER,
            'actual one-hundred-fourteen-test pure checker reference')
    root_inputs = {name: row for name, row in t['readset'].items() if name.startswith(str(REMOTE) + '/')}
    required_inputs = {name for name in ROOT_PINS if name.endswith('.py') and name != 'prepare_model_inputs.py'}
    required_inputs |= {case + '-input.json', case + '-request.json'}
    required = {str(REMOTE / name): dict(path=str(REMOTE / name), **ROOT_PINS[name]) for name in required_inputs}
    if case == 'tail':
        for name in ('census-input.json', 'census-request.json'):
            required[str(REMOTE / name)] = dict(path=str(REMOTE / name), **ROOT_PINS[name])
        for name, body in bodies.items():
            if name.startswith('census/'):
                required[str(REMOTE / name)] = dict(path=str(REMOTE / name), **pin(body))
    require(root_inputs == required, 'exact current root inputs plus authenticated prior CensusV3 control readset')
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
        require(same(parse(bodies[case + '/' + label + '/result.json']),
            {k: v for k, v in row.items() if k != 'label'}), 'phase original result')
        command = parse(bodies[case + '/' + label + '/command.json'])
        require(command['cwd'] == str(REMOTE.parent) and command['affinity'] == [8, 9] and command['nice'] == 10
                and command['stream_cap_bytes'] == 8 << 20 and command['file_cap_bytes'] == 64 << 20,
                'original leaf resource bounds')
        expected_argv = ([plan['parent']['path'], '--request', plan['request']['path'],
            ('--observe-guarded-readiness40-position5-bank-scoped-census-host-timing-v3' if case == 'census' else
             '--observe-guarded-readiness40-position5-bank-scoped-census-tail-host-timing-v4'), '--allow-unauthenticated-machine-code'] if label == 'parent' else
            ['/opt/rocm/bin/amd-smi', 'process', '--json'] if label.startswith(('before-', 'after-')) else
            ['/usr/bin/readelf', '-l', '-d', plan[label.split('-')[0]]['path']] if label.endswith('-readelf') else
            ['/usr/bin/ldd', plan[label.split('-')[0]]['path']])
        require(command['argv'] == expected_argv, 'actual native/audit argv')
    baseline_raw, baseline_checked = verify_baseline(bodies, t)
    confirmed = None; parity = None; matched_checked = None; summary_raw = None; parent_identity = None
    if t['passed']:
        require(labels == list(LABELS) and set(t['raw']) == matched_names(case)
                and t['native_attempts'] == 1 and t['owned_worker_lineage_verified'] is True
                and all(t[k] is True for k in ('native_spawn_observed', 'gpu_execution_requested',
                    'gpu_execution', 'gpu_execution_confirmed', 'model_source_authenticated_by_qualified_parent'))
                and t['partial_gpu_execution_possible'] is False
                and type(t['matched_timing']) is dict, 'successful closed execution scope')
        for row in t['phases']:
            require(type(row['exit_code']) is int and row['exit_code'] == 0 and row['reason'] is None
                    and row['cleanup_signalled'] is False and row['owned_groups_absent'] is True
                    and row['owned_processes_reaped'] is True, 'natural successful owned phase')
        initial = parse(bodies[case + '/initial-topology.json'])
        require(same(initial, t['platform']) and initial['host'] == 'smci350-rck-g03-b19-03'
                and re.fullmatch('[0-9a-f-]{36}', initial['boot'])
                and len(initial['devices']) == 2, 'recorded selected host/topology')
        for row, node, identity in zip(initial['devices'], (2, 3),
                (16366993098680759275, 10838076764495710945)):
            require(row['node'] == node and type(row['gpu_id']) is int and row['gpu_id'] > 0
                    and int(row['properties']['unique_id']) == identity
                    and int(row['properties']['gfx_target_version']) == 90500, 'recorded gfx950 unique identity')
        for label in [n for n in LABELS if n.startswith(('before-', 'after-'))]:
            require(same(parse(bodies[case + '/' + label + '-topology.json']), initial), 'six stable topology snapshots')
            idle = parse(bodies[case + '/' + label + '/stdout'])
            require(type(idle) is list and len(idle) == 8
                    and all(set(r) == {'gpu', 'process_list'} and type(r['gpu']) is int for r in idle)
                    and {r['gpu'] for r in idle} == set(range(8))
                    and all(r['process_list'] == [{'process_info': 'No running processes detected'}] for r in idle),
                    'six original all-eight-device idle observations')
        validator, shared, timing, matched, scoped, bank, census, tail, _ = validation_modules(bodies)
        guard = module(bodies['readiness_announcement.py'], ROOT_PINS['readiness_announcement.py'], 'readiness_lineage')
        summary_raw = bodies[case + '/native/complete.json']; summary = parse(summary_raw)
        prompt = summary['bootstrap']['sequence']['prompt_tokens']
        require(type(prompt) is list and len(prompt) == 2048
                and all(type(v) is int and 0 <= v < 151936 for v in prompt)
                and hashlib.sha256(struct.pack('<2048I', *prompt)).hexdigest() ==
                '2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02',
                'all2048 original authentic prompt bits without copying model bulk')
        def read_body(row):
            return native_body(bodies, row)
        matched_checked = (census.validate('census_timed', bodies[case + '/parent/stdout'], summary_raw,
                request, CASE / 'native', read_body, prompt) if case == 'census' else
            tail.validate('tail_timed', bodies[case + '/parent/stdout'], summary_raw,
                request, CASE / 'native', read_body, prompt))
        confirmed = matched_checked['ordinary']
        require(same(confirmed, t['observation']) and same(confirmed, parse(bodies[case + '/observation.json'])),
                'same independently checked40 transcript/four captures')
        require(same(matched_checked, t['matched_timing'])
                and same(matched_checked, parse(bodies[case + '/matched.json'])),
                'same original timed wrapper, policy bytes and complete 124-span admission')
        native = next(row for row in t['phases'] if row['label'] == 'parent')
        started = parse(bodies[case + '/parent/started.json'])
        guard.validate_lineage(bodies[case + '/parent/stderr'], native, started, confirmed['child_pid'])
        require(t['native_started'] == t['raw']['parent/started.json'], 'recorded native registration')
        parent_identity = started['parent']
        parity = (census.compare_same_side(summary_raw, baseline_raw, matched_checked, baseline_checked, read_body)
            if case == 'census' else tail.compare_same_side(summary_raw, baseline_raw,
                matched_checked, baseline_checked, read_body))
        require(same(parity, t['same_side_parity']) and same(parity, parse(bodies[case + '/parity.json'])),
                'same independently revalidated semantic and full-payload parity')
    require(type(t['started_monotonic']) in (int, float) and type(t['completed_monotonic']) in (int, float)
            and 0 < t['started_monotonic'] < t['completed_monotonic'] < float('inf'),
            'finite original case chronology')
    if case == 'census':
        require(t['prior_census'] is None and t['matched_pair'] is None, 'Census control has no earlier matched case')
    observation = dict(present=True, original_passed=t['passed'],
        original_terminal=dict(name=terminal_name, **pin(terminal_body)), original_raw_files=len(t['raw']),
        retained_success_revalidated=confirmed is not None,
        readiness_completed_forwards=confirmed['completed_forwards'] if confirmed else None,
        matched_timing=matched_checked, semantic_and_payload_parity_revalidated=parity is not None,
        original_policy_bytes_retained=True, parent_host_measurement=True, gpu_timing=False,
        numerical_reference_replayed=False, numerical_acceptance=False, performance_claim=False)
    return observation, t, summary_raw, matched_checked, parent_identity


def verify(bodies, outcomes):
    tick()
    require(type(outcomes) is dict and set(outcomes) == set(ORDER), 'explicit two-case outcomes')
    for outcome in outcomes.values():
        require(outcome is None or (type(outcome) is dict and set(outcome) == {'name', 'sha256'}
                and outcome['name'] in ('complete.json', 'failed.json')
                and type(outcome['sha256']) is str and re.fullmatch('[0-9a-f]{64}', outcome['sha256'])),
                'observed terminal name/hash or explicitly absent case')
    require(all(type(v) is dict and set(v) == {'bytes', 'sha256'} for v in
                [*ROOT_PINS.values(), *ADMISSION.values()])
            and type(CHECKER) is dict and set(CHECKER) == {'path', 'bytes', 'sha256'},
            'all observed source/input/CPU/product/checker bindings required')
    require(all(ordinary(name) and type(body) is bytes and len(body) <= BODY_CAP
                and not body.startswith(b'\x7fELF') for name, body in bodies.items()), 'bounded nonbinary bodies only')
    require(len(bodies) <= FILE_CAP and sum(map(len, bodies.values())) <= TOTAL_CAP, 'capsule extent bound')
    for name, expected in ROOT_PINS.items():
        require(name in bodies and pin(bodies[name]) == expected, 'exact actual deployed/prepared body: ' + name)
    require(bodies.get('retention_tool.py') == read(Path(__file__).resolve()), 'same reviewed retention tool')
    selected = {name for name in bodies if any(name.startswith(case + '/') for case in ORDER)}
    require(set(bodies) == set(ROOT_PINS) | {'retention_tool.py'} | selected
            | {'baseline/' + name for name in baseline_names()}, 'closed roots/cases/baseline/tool bodies')
    plans = {case: parse(bodies[case + '-input.json']) for case in ORDER}
    requests = {case: parse(bodies[case + '-request.json']) for case in ORDER}
    prepared = parse(bodies['prepared-inputs.json'])
    require(prepared['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-tail-matched-timing-input-preparation-v1'
            and prepared['passed'] is True and prepared['modes'] == list(ORDER)
            and prepared['parent_host_timing_requested'] is True
            and prepared['currentness_policies'] == dict(census='bank_scoped_census', tail='bank_scoped_census_tail')
            and prepared['temporal_equivalent_to_full'] is False
            and prepared['allocation_preflights_changed'] is True
            and prepared['allocation_preflights_changed_only_for_candidate'] is False
            and prepared['scoped_tail_requested_only_for_candidate'] is True
            and prepared['tail_counters_are_independent'] is True
            and prepared['census_counters_are_layer_subset'] is True
            and prepared['shared_full_currentness_requested'] is False
            and prepared['identical_parent_and_worker_products'] is True and prepared['fresh_case_namespaces'] is True
            and prepared['controller'] == dict(path=str(REMOTE / 'prepare_model_inputs.py'), **ROOT_PINS['prepare_model_inputs.py'])
            and all(prepared[k] is False for k in ('native_execution', 'gpu_execution', 'gpu_timing',
                'full_long_workload', 'numerical_acceptance', 'performance_claim', 'production_authority')),
            'original observed two-case data-only preparation')
    for case in ORDER:
        plan, request = plans[case], requests[case]
        require(set(plan) == {'schema', 'case', 'profile', 'parent_cpu', 'worker_cpu', 'parent', 'worker', 'request'}
                and plan['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-tail-matched-timing-gpu-input-v1'
                and plan['case'] == case and plan['profile'] == 'readiness40_position5'
                and plan['request'] == dict(path=str(REMOTE / (case + '-request.json')), **ROOT_PINS[case + '-request.json'])
                and prepared['requests'][case] == plan['request']
                and prepared['plans'][case] == dict(path=str(REMOTE / (case + '-input.json')), **ROOT_PINS[case + '-input.json'])
                and request['schema'] == 'FerricGuardedMlpReadiness40Position5RequestV1'
                and request['base']['evidence_directory'] == str(REMOTE / case / 'native'),
                'exact original prepared case request')
        for key, expected in ADMISSION.items():
            require(compact(plan[key]) == expected and plan[key] == plans['census'][key],
                    'both plans name the same observed qualified CPU/product pair')
        require(prepared['parent_cpu'] == plan['parent_cpu'] and prepared['worker_cpu'] == plan['worker_cpu'],
                'preparation CPU receipts')
    sessions = [requests[case]['base']['session'] for case in ORDER]
    require(prepared['sessions'] == sessions and sessions[0] != sessions[1]
            and all(type(row) is list and len(row) == 32
                and all(type(v) is int and 0 <= v <= 255 for v in row) and any(row) for row in sessions),
            'distinct actual nonzero 32-byte sessions')
    def immutable(request):
        base = dict(request['base'])
        for key in ('session', 'evidence_directory'): base.pop(key)
        return dict(request, base=base)
    require(same(immutable(requests['census']), immutable(requests['tail'])),
            'both original requests differ only in session and case root')
    verify_baseline(bodies, None)
    cases = {}; state = {}
    for case in ORDER:
        present = any(name.startswith(case + '/') for name in bodies)
        require(present is (outcomes[case] is not None), 'absent means no case evidence, never an invented terminal')
        if outcomes[case] is None:
            cases[case] = dict(present=False, original_passed=None, original_terminal=None,
                              original_raw_files=0, retained_success_revalidated=False, matched_timing=None)
            continue
        observation, terminal, summary, checked, identity = verify_case(bodies, case, outcomes[case])
        cases[case] = observation
        state[case] = dict(terminal=terminal, summary=summary, checked=checked, identity=identity)
    pair = None
    if 'tail' in state:
        require('census' in state and cases['census']['original_passed'] is True,
                'Tail case requires an original healthy Census control')
        control, candidate = state['census'], state['tail']
        control_pin = dict(path=str(REMOTE / 'census' / outcomes['census']['name']),
                           **pin(bodies['census/' + outcomes['census']['name']]))
        require(candidate['terminal']['prior_census'] == control_pin
                and candidate['terminal']['admission'] == control['terminal']['admission']
                and control['terminal']['completed_monotonic'] < candidate['terminal']['started_monotonic'],
                'actual Census control terminal/CPU/ELF and original sequential chronology')
        if cases['tail']['original_passed']:
            require(same(candidate['terminal']['platform'], control['terminal']['platform']),
                    'successful pair same host/boot/topology')
            require(control['identity']['pid'] != candidate['identity']['pid'], 'distinct actual native parent identities')
            _, _, _, _, _, _, _, _, pair_validator = validation_modules(bodies)
            pair = pair_validator.compare_pair(control['summary'], candidate['summary'], control['checked'], candidate['checked'],
                    dict(census=control['terminal']['admission'], tail=candidate['terminal']['admission']),
                    lambda row: native_body(bodies, row))
            require(same(pair, candidate['terminal']['matched_pair'])
                    and same(pair, parse(bodies['tail/pair.json'])), 'exact independently revalidated matched pair')
    passed = pair is not None and all(cases[case]['original_passed'] is True for case in ORDER)
    if passed:
        require(len(bodies) == 243, 'successful243 pinned original bodies before manifest')
    tick()
    return dict(passed=passed, cases=cases, matched_pair=pair, selected_original_bodies=len(bodies),
        raw_files=sum(case['original_raw_files'] for case in cases.values()),
        cpu_and_elf_metadata=ADMISSION, checker_cpu=CHECKER, historical_baseline=BASELINE,
        baseline_raw_files=70, baseline_data_revalidated=True,
        both_original_owned_lineages_revalidated=passed, matched_semantic_and_payload_parity_revalidated=passed,
        parent_host_measurement=True, original_policy_bytes_retained=True, gpu_timing=False,
        census_scoped_case_retained='census' in state, tail_scoped_case_retained='tail' in state,
        scoped_tail=True, tail_counters_are_independent=True,
        allocation_preflights_changed=True, census_counters_are_layer_subset=True,
        shared_full_currentness_requested=False,
        currentness_temporal_equivalence_claim=False,
        data_only=True, native_rerun=False, gpu_execution=False, model_execution=False,
        numerical_reference_replayed=False, numerical_acceptance=False, full_long_workload=False,
        performance_claim=False, production_authority=False)


def export(outcomes, archive):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged export host')
    require(archive.is_absolute() and archive.parent.resolve(strict=True) == archive.parent
            and archive.parent != REMOTE and REMOTE not in archive.parents, 'archive outside original source tree')
    present = {case for case in ORDER if outcomes[case] is not None}
    require({p.name for p in REMOTE.iterdir()} == set(ROOT_PINS) | present, 'closed original input/two-case root')
    names = {case: roster(REMOTE / case) for case in present}
    inputs = {name: REMOTE / name for name in ROOT_PINS}
    for case in present:
        inputs.update({case + '/' + name: REMOTE / case / name for name in names[case]})
    inputs.update({'baseline/' + name: BASELINE_ROOT / name for name in baseline_names()})
    inputs['retention_tool.py'] = Path(__file__).resolve()
    bodies = {name: read(path, ROOT_PINS.get(name)) for name, path in inputs.items()}
    observation = verify(bodies, outcomes)
    manifest = dict(schema='ferric-guarded-mlp-readiness40-bank-scoped-census-tail-matched-timing-retention-v1',
        source_root=str(REMOTE), files={name: pin(body) for name, body in sorted(bodies.items())},
        observation=observation, outcomes=outcomes)
    for name, path in inputs.items(): require(read(path, pin(bodies[name])) == bodies[name], 'export posthash')
    require(all(roster(REMOTE / case) == names[case] for case in present), 'source roster drift')
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
    require(all(roster(REMOTE / case) == names[case] for case in present), 'source roster final drift')
    require({p.name for p in REMOTE.iterdir()} == set(ROOT_PINS) | present, 'final two-case root drift')
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
    require(set(manifest) == {'schema', 'source_root', 'files', 'observation', 'outcomes'}
            and manifest['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-tail-matched-timing-retention-v1'
            and manifest['source_root'] == str(REMOTE)
            and set(manifest['files']) == set(bodies), 'closed manifest member map')
    for name, body in bodies.items(): require(pin(body) == manifest['files'][name], 'all archive member hashes')
    observation = verify(bodies, manifest['outcomes'])
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


def outcome_argument(value):
    if value == 'absent': return None
    parts = value.split(':')
    require(len(parts) == 2 and parts[0] in ('complete.json', 'failed.json')
            and re.fullmatch('[0-9a-f]{64}', parts[1]), 'OUTCOME is absent or complete.json|failed.json:OBSERVED_SHA')
    return dict(name=parts[0], sha256=parts[1])


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
        result = export(dict(zip(ORDER, map(outcome_argument, sys.argv[2:4]))), Path(sys.argv[4]))
    elif len(sys.argv) == 5 and sys.argv[1] == 'retain':
        result = retain(Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]))
    else:
        raise RuntimeError('export CENSUS_OUTCOME TAIL_OUTCOME ARCHIVE; or retain ARCHIVE OBSERVED_ARCHIVE_SHA FRESH_DIRECTORY')
    tick()
    print(json.dumps(result, sort_keys=True, allow_nan=False))
    signal.setitimer(signal.ITIMER_REAL, 0)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
