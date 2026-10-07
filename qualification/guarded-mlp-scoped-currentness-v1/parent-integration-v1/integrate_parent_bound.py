"""Emit seven tested scoped parent rows; preserve the entire qualified worker tree."""
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import time

F = Path('/home/harsh/ferric-p227-integration')
RT = Path('/home/harsh/fe2o3-p228-runtime')
Q = F / 'qualification/guarded-mlp-scoped-currentness-v1'
ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-scoped-warm-parent-cpu-v228-v3'
PARENT = 'ferric/adapters/m1-engineering-execution-v1/'
WORKER = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
PROPOSAL = dict(bytes=11484, sha256='687fa059a6eb2744ad55618fe7b59830ec36a4f096e6cf5ee456bf4ee5056db8')
BASE_PARENT = dict(bytes=3983845, sha256='f7134c5f22d75c8295b8ab99ad8625fac40aa615ac1682340c0b8e6ffde1f71e')
BASE_PARENT_MAP = dict(bytes=507332, sha256='9e70e32da52ebba96a8354ecd5fe60bbb3add9a7d1b93ba0d218572349f129c4')
CONTROLLER = dict(bytes=45154, sha256='39fd25f0feecf5204363a0942615f5f6cd543df7a52075597210a35e53df00cf')
INPUT = dict(bytes=265721, sha256='30d16317a92cdd1477a10c87921a19196d6a109894ad6729f994353feaecf372')
WORKER_COMPLETE = dict(bytes=2364691, sha256='335cf93cc109390f3b7590f7dbe35ed852adca223a042894dcc18418a738ffbd')
WORKER_SOURCES = dict(bytes=420283, sha256='e5ca47a688908d4c78d1e3157211aee4c29bf12f626c01258f06a8ca8eefbda3')
PROPOSAL_SHAS = {
    'parent-proposal.json': '687fa059a6eb2744ad55618fe7b59830ec36a4f096e6cf5ee456bf4ee5056db8',
    'runtime-proposal.json': '437948cd334c9f95dcf77de4e3217c92f4d8e6d4cabbcf5010aff182f53ae5b2',
    'primitive-proposal.json': '9abcd59d4c3fd23d532bd1b9e86592a191e62960d8a901fbd31d8e61974bdec7',
    'consumer-proposal.json': '6b4dffbe4032e52f6dddb80f255553c420e95a3fb7af84f2b1da311daf1168bb',
    'consumer-repair.json': 'c2be6fe83fba6c4bfef971bdf1e1535b88167ec8685f6faf6aa163abbdaa93e0',
    'selector-proposal.json': 'cfec625935042c416a329cfb0e02c1fc8836f12820967e1d3c0257ed2d6a8235',
    'selector-repair.json': '2659a6e7d2dd71a22fa5579be3a70be948b81c806cb511c79633e8739eb375a0',
}
FULL_PATHS = tuple(PARENT + n for n in (
    'Cargo.toml', 'src/lib.rs', 'src/tp_finite_client/long.rs',
    'src/bin/ferric-qwen3-guarded-mlp-full2303-engineering.rs',
    'src/tp_finite_client/long/full2303.rs',
    'src/tp_finite_client/long/full2303_evidence.rs',
    'src/tp_finite_client/long/full2303_tests.rs'))
MAX_FILE, MAX_TOTAL = 16 << 20, 96 << 20
DEADLINE = None
READSET = {}

def require(ok, message):
    if not ok:
        raise ValueError(message)


def guard():
    require(DEADLINE is not None and time.monotonic() < DEADLINE, 'whole integration deadline')


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {k: row[k] for k in ('bytes', 'sha256')}


def parse(raw):
    def pairs(rows):
        result = {}
        for name, value in rows:
            require(name not in result, 'duplicate JSON key')
            result[name] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON'))


def relative(name):
    require(type(name) is str and name and Path(name).as_posix() == name
            and not Path(name).is_absolute() and '..' not in Path(name).parts
            and name not in ('.', '') and '\\' not in name, 'ordinary relative name')
    return name


def read(path):
    guard()
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical data path')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1
                and 0 <= before.st_size <= MAX_FILE, 'bounded ordinary data body')
        raw = stream.read(MAX_FILE + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size,
            'data changed during read')
    current = pin(raw)
    require(str(path) not in READSET or READSET[str(path)] == current, 'conflicting input pin')
    READSET[str(path)] = current
    require(len(READSET) <= 2200 and sum(p['bytes'] for p in READSET.values()) <= MAX_TOTAL,
            'bounded entire integration readset')
    return raw


def capsule(directory, count):
    manifest = parse(read(directory / 'manifest.json'))
    expected = manifest['files']
    require(type(expected) is dict and len(expected) == count, 'closed actual capsule manifest')
    names = set()
    for parent, dirs, files in os.walk(directory, followlinks=False,
            onerror=lambda e: (_ for _ in ()).throw(e)):
        guard()
        require(not any((Path(parent) / n).is_symlink() for n in dirs), 'capsule directory alias')
        names.update(relative(str((Path(parent) / n).relative_to(directory))) for n in files)
        require(len(names) <= count + 2, 'capsule file count bound')
    require(names == set(expected) | {'manifest.json', 'retention.json'}, 'exact retained capsule closure')
    bodies = {relative(n): read(directory / n) for n in expected}
    require(all(pin(raw) == expected[n] for n, raw in bodies.items()), 'all original capsule pins')
    read(directory / 'retention.json')
    return bodies


def actual(result, schema, phases, sources):
    require(result['schema'] == schema and result['passed'] is True and result['failure'] is None
            and result['postcheck_errors'] == [] and result['source_unchanged'] is True
            and result['input_sources'] == result['final_sources'] and len(result['final_sources']) == sources
            and len(result['phases']) == phases and result['gpu_execution'] is False,
            'actual successful CPU/source admission')
    require(all(type(p['exit_code']) is int and p['exit_code'] == 0 and p['natural_exit'] is True
                and p['reaped'] is True and p['process_group_absent'] is True
                and p['forced_cleanup'] is False and p['timed_out'] is False
                and p['exception'] is None and p['storage_failure'] is None for p in result['phases']),
            'actual clean natural CPU lifecycles')


def main():
    global DEADLINE
    DEADLINE = time.monotonic() + 180
    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError('integration deadline')))
    signal.setitimer(signal.ITIMER_REAL, 180)
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_CPU, (120, 120))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 3
            and re.fullmatch('[0-9a-f]{64}', sys.argv[1]),
            'python3 -B integrate_parent.py OBSERVED_PARENT_SHA FRESH_OUTPUT')
    output = Path(sys.argv[2])
    require(output.is_absolute() and output.parent.resolve(strict=True) == output.parent
            and not any(output.is_relative_to(root) for root in (F, RT))
            and not os.path.lexists(output), 'fresh output outside canonical repositories')
    require(CONTROLLER is not None and INPUT is not None, 'observed controller/input bindings required')
    own = read(Path(__file__).resolve())
    directory = Q / 'parent-cpu-attempt-v3'
    pb = capsule(directory, 482)
    manifest = parse(read(directory / 'manifest.json'))
    require('evidence/failed.json' not in pb
            and pin(pb['evidence/complete.json'])['sha256'] == sys.argv[1]
            and manifest['schema'] == 'ferric-readiness40-scoped-warm-parent-cpu-selected-evidence-v1'
            and manifest['passed'] is True and manifest['terminal_name'] == 'complete.json'
            and manifest['terminal'] == pin(pb['evidence/complete.json'])
            and manifest['all_raw_and_lineage_retained'] is True
            and manifest['original_receipts_unchanged'] is True,
            'observed successful parent original; never integrate failure')
    parent = parse(pb['evidence/complete.json'])
    actual(parent, 'ferric-guarded-mlp-readiness40-scoped-warm-parent-cpu-v1', 65, 1266)
    require(len(parent['tests']) == 55 and sum(v['passed'] for v in parent['tests'].values()) == 492
            and all(v['failed'] == v['ignored'] == 0 for v in parent['tests'].values())
            and len(parent['inventory']) == 961 and len(parent['artifacts']) == 7
            and len(parent['raw']) == 329 and parent['all_selected_parent_tests_executed'] is True,
            '65 phases/55 scopes/492 selected/961 inventory/seven products')
    require(all(parent[k] is True for k in (
                'readiness40_scoped_warm_parent_source_added', 'scoped_warm_host_timing_parent_source_added',
                'shared_full_host_timing_parent_source_added', 'qualified_worker_sources_preserved',
                'parent_host_timing_source_added', 'full2303_parent_route_added'))
            and parent['parent_host_timing_rows'] == 40 and parent['parent_host_timing_disjoint_spans'] == 124
            and parent['full2303_source_abort_ms'] == 3600000
            and parent['worker_qualification'] == WORKER_COMPLETE
            and parent['worker_source_manifest'] == WORKER_SOURCES
            and all(parent[k] is False for k in (
                'readiness40_scoped_warm_native_execution', 'currentness_temporal_equivalence_claim',
                'shared_full_host_timing_native_execution', 'parent_host_timing_native_execution',
                'ordinary_wire_schema_changed', 'ordinary_observation_schema_changed',
                'full2303_native_execution', 'full2303_launch_feasibility', 'numerical_acceptance',
                'performance_claim', 'production_authority', 'worker_suite_rerun', 'runtime_suite_rerun',
                'global_currentness_policy_changed', 'full_parent_library_suite_executed')),
            'explicit parent-only source qualification and unchanged authority')
    require(pin(pb['inputs/parent-proposal.json']) == PROPOSAL
            and pin(pb['inputs/parent-complete.json']) == BASE_PARENT
            and pin(pb['inputs/parent-sources.json']) == BASE_PARENT_MAP
            and pin(pb['inputs/worker-complete.json']) == WORKER_COMPLETE
            and pin(pb['inputs/worker-sources.json']) == WORKER_SOURCES
            and pin(pb['input-manifest.json']) == compact(parent['input_manifest']) == INPUT
            and pin(pb['run_cpu.py']) == compact(parent['controller']) == CONTROLLER,
            'observed qualified baselines and bound controller/input')
    base = parse(pb['inputs/parent-complete.json'])
    actual(base, 'ferric-guarded-mlp-readiness40-shared-full-host-timing-parent-cpu-v1', 64, 1260)
    old = parse(pb['inputs/parent-sources.json'])
    require(old == base['final_sources'] and len(base['tests']) == 54
            and sum(v['passed'] for v in base['tests'].values()) == 478
            and len(base['inventory']) == 948 and set(parent['artifacts']) == set(base['artifacts']),
            'actual timed parent baseline and unchanged product roles')
    worker = parse(pb['inputs/worker-complete.json'])
    actual(worker, 'ferric-guarded-mlp-scoped-currentness-cpu-v1', 26, 1033)
    worker_map = parse(pb['inputs/worker-sources.json'])
    require(worker['source_generation'] == 'scoped-currentness-coupled-v3'
            and worker_map == worker['final_sources']
            and (worker['tests']['kfd-tests']['passed'], worker['tests']['kfd-tests']['ignored']) == (1143, 8)
            and (worker['tests']['worker-tests']['passed'], worker['tests']['worker-tests']['ignored']) == (726, 4)
            and len(worker['artifacts']) == 11
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin'],
            'actual complete coupled V3 qualification, not a failed predecessor')
    inputs = parse(pb['input-manifest.json'])
    require(inputs['schema'] == 'ferric-guarded-mlp-readiness40-scoped-warm-parent-cpu-input-v1'
            and len(inputs['lineage']) == 121 and set(parent['readset']) == set(inputs['lineage']),
            'closed121 direct lineage')
    for name, expected in inputs['lineage'].items():
        require(pin(pb['inputs/' + relative(name)]) == expected == compact(parent['readset'][name]),
                'original direct lineage readset')
    for name, sha in PROPOSAL_SHAS.items():
        require(pin(pb['inputs/' + name])['sha256'] == sha, 'held source generation')
        if name != 'parent-proposal.json':
            require(compact(worker['readset'][name]) == pin(pb['inputs/' + name]),
                    'coupled qualification carries both original and repaired manifests')
    proposal = parse(pb['inputs/parent-proposal.json'])
    cp, cr, sp, sr = (parse(pb['inputs/' + n]) for n in (
        'consumer-proposal.json', 'consumer-repair.json', 'selector-proposal.json', 'selector-repair.json'))
    require(compact(proposal['requires']['consumer']) == compact(sp['requires']['consumer'])
                == compact(sr['requires']['consumer']) == pin(pb['inputs/consumer-proposal.json'])
            and compact(proposal['requires']['worker_selector']) == pin(pb['inputs/selector-proposal.json'])
            and compact(cr['predecessor_manifest']) == pin(pb['inputs/consumer-proposal.json'])
            and compact(sr['predecessor_manifest']) == pin(pb['inputs/selector-proposal.json']),
            'original requirements and explicit cumulative repair chains')
    require(proposal['schema'] == 'ferric-scoped-warm-readiness40-parent-source-v1', 'closed parent proposal')
    rows = {'ferric/' + row['path']: row for row in proposal['files']}
    parent_names = set(rows)
    require(len(rows) == len(proposal['files']) == 7
            and all(n.startswith(PARENT) and n.endswith('.rs') and r['repository'] == 'ferric'
                    for n, r in rows.items())
            and sum(r['before'] is None for r in rows.values()) == 2
            and inputs['parent_overlay'] == sorted(parent_names), 'seven parent Rust rows, five replace/two add')
    for name, expected in parent['raw'].items():
        require('/' not in relative(name) and pin(pb['evidence/' + name]) == compact(expected),
                'all329 original raw bodies')
    for phase in parent['phases']:
        label = phase['label']
        saved = parse(pb['evidence/' + label + '.result.json'])
        command = parse(pb['evidence/' + label + '.command.json'])
        started = parse(pb['evidence/' + label + '.started.json'])
        require(saved == phase and phase['observed_signals'] == []
                and command['argv'] == started['argv'] == phase['argv']
                and phase['pid'] == phase['pgid'] == started['pid'] == started['pgid'],
                'original owned lifecycle and command joins')
    parent_pre = parse(pb['evidence/sources-preformat.json'])
    parent_map = parse(pb['evidence/sources-after.json'])
    require(parent_pre == parent['preformat_sources'] and parent_map == parent['final_sources']
            and set(parent_pre) == set(parent_map)
            and inputs['files'] == {n: compact(v) for n, v in parent_pre.items()}
            and all(v['path'] == ROOT + '/' + n for n, v in parent_map.items()),
            'complete actual source maps and correct V3 namespace')
    changed = {n for n in parent_pre if compact(parent_pre[n]) != compact(parent_map[n])}
    require(changed <= parent_names and changed == set(parent['format_changed_paths'])
            and len(changed) == len(parent['format_changed_paths']), 'only seven parent formatting paths')
    work = {n: compact(v) for n, v in worker_map.items() if n.startswith(WORKER)}
    baseline = {n: compact(v) for n, v in old.items() if n.startswith('ferric/')}
    require(len(work) == 209 and len(baseline) == 1257
            and sum(n.startswith(WORKER) for n in baseline) == 205, 'full old parent and new worker extents')
    expected = {n: v for n, v in baseline.items() if not n.startswith(WORKER)}
    require(len(expected) == 1052, 'all unchanged historical nonworker bodies')
    expected.update(work)
    for name, row in rows.items():
        require(expected.get(name) == row['before'], 'actual parent canonical preimage or absence')
        expected[name] = row['after']
    desired = {n: compact(v) for n, v in parent_map.items() if n.startswith('ferric/')}
    require(expected == {n: compact(v) for n, v in parent_pre.items() if n.startswith('ferric/')}
            and len(expected) == len(desired) == 1263, 'complete1263 Ferric composition')
    require(work == {n: compact(v) for n, v in parent_pre.items() if n.startswith(WORKER)}
            and work == {n: compact(v) for n, v in parent_map.items() if n.startswith(WORKER)},
            'all209 qualified worker bodies byte-identical through parent compile')
    worker_overlays = {'ferric/' + row['path'] for prop in (cr, sr) for row in prop['files']}
    require(len(worker_overlays) == 13 and all(n.startswith(WORKER) and pin(pb[n]) == work[n]
            for n in worker_overlays), 'thirteen actual coupled worker postimages retained unchanged')
    unchanged_full = set(FULL_PATHS) - parent_names
    require(len(unchanged_full) == 6 and all(baseline[n] == expected[n] == desired[n] for n in unchanged_full)
            and PARENT + 'src/lib.rs' in parent_names, 'six Full paths unchanged; lib registration is reviewed overlay')
    for name in (PARENT + 'Cargo.toml', PARENT + 'Cargo.lock'):
        require(baseline[name] == expected[name] == desired[name], 'parent manifest and lock unchanged')
    require(pin(pb[PARENT + 'Cargo.lock']) == desired[PARENT + 'Cargo.lock'], 'retained original parent lock')
    test_groups = (
        (PARENT + 'src/tp_finite_client/long/readiness_host_timing_tests.rs',
         'tp_finite_client::long::readiness::host_timing::tests::', 14, 2),
        (PARENT + 'src/tp_finite_client/long/readiness_scoped_tests.rs',
         'tp_finite_client::long::readiness::scoped::tests::', 4, 4),
        (WORKER + 'src/finite_guarded_mlp_readiness_scoped_v1_tests.rs',
         'finite_guarded_mlp_readiness_scoped_v1::tests::', 7, 7))
    additions = []
    for path, prefix, total, added in test_groups:
        declared = [prefix + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', pb[path].decode())]
        new = [n for n in declared if n not in base['inventory']]
        require(len(declared) == len(set(declared)) == total and len(new) == added, 'source-declared methods')
        additions.extend(new)
    policy = {n for n in additions if n.startswith('finite_guarded_mlp_readiness_scoped_v1::tests::')}
    client = set(additions) - policy
    bin_names = {'tests::' + n for n in proposal['new_tests']['parent_readiness_binary']}
    require(len(additions) == len(set(additions)) == 13 and len(policy) == 7 and len(client) == 6
            and len(bin_names) == 1
            and sorted(n.rsplit('::', 1)[-1] for n in additions) == sorted(proposal['new_tests']['parent_library'])
            and parent['inventory'] == sorted(base['inventory'] + additions)
            and set(parent['tests']) == set(base['tests']) | {'parent-readiness-scoped-policy'},
            'thirteen library additions and one new selected scope')
    expected_tests = {}
    for label, prior in base['tests'].items():
        names = {r['name']: r['outcome'] for r in prior['named']}
        require(len(names) == len(prior['named']) and all(v == 'ok' for v in names.values()), 'old478 named passes')
        extra = client if label == 'parent-client' else bin_names if label == 'readiness-bin-tests' else set()
        require(not set(names) & extra, 'additive names')
        expected_tests[label] = names | {n: 'ok' for n in extra}
    expected_tests['parent-readiness-scoped-policy'] = dict.fromkeys(policy, 'ok')
    require(sum(map(len, expected_tests.values())) == 492, 'exact complete selected names')
    for label, names in expected_tests.items():
        value = parent['tests'][label]
        observed = {r['name']: r['outcome'] for r in value['named']}
        raw_rows = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$',
                              pb['evidence/' + label + '.stdout'].decode(), re.M)
        require(observed == names and len(observed) == len(value['named']) == value['passed']
                and len(raw_rows) == len(dict(raw_rows)) and dict(raw_rows) == names,
                'original raw selected outcomes exactly preserve/add reviewed names')
    cargo = [parse(line) for line in pb['evidence/parent-builds.stdout'].splitlines() if line.startswith(b'{')]
    for value in parent['artifacts'].values():
        record = value['cargo_artifact']
        require(cargo.count(record) == 1 and record['reason'] == 'compiler-artifact'
                and record['executable'] == value['pin']['path']
                and record['target']['kind'] == ['bin'] and record['profile']['test'] is False,
                'actual seven Cargo product rows')
    current, absent = {}, []
    for name, after in sorted(desired.items()):
        path = F / relative(name.removeprefix('ferric/'))
        if name in parent_names and rows[name]['before'] is None:
            require(not os.path.lexists(path), 'two new parent paths absent')
            current[name] = None
            absent.append(path)
        else:
            raw = read(path)
            require(pin(raw) == (rows[name]['before'] if name in parent_names else after),
                    'entire canonical before/composed worker identity: ' + name)
            current[name] = raw
    patch = ['*** Begin Patch\n']; emitted = []
    for name in sorted(parent_names):
        before, after = current[name], pb[name]
        require(pin(after) == desired[name] and after.endswith(b'\n')
                and (before is None or before.endswith(b'\n')), 'actual formatted parent text')
        path = str(F / name.removeprefix('ferric/'))
        if before is None:
            patch += ['*** Add File: ' + path + '\n']
        else:
            patch += ['*** Update File: ' + path + '\n', '@@\n']
            patch += ['-' + line + '\n' for line in before.decode()[:-1].split('\n')]
        patch += ['+' + line + '\n' for line in after.decode()[:-1].split('\n')]
        emitted.append(dict(path=name.removeprefix('ferric/'),
                            before=pin(before) if before is not None else None, after=pin(after)))
    patch.append('*** End Patch\n')
    patch_raw = ''.join(patch).encode()
    require(len(patch_raw) <= MAX_FILE and len(emitted) == 7 and len(absent) == 2, 'seven-row bounded patch')
    for name, expected_pin in list(READSET.items()):
        require(pin(read(Path(name))) == expected_pin, 'entire integration readset posthash')
    require(read(Path(__file__).resolve()) == own and all(not os.path.lexists(p) for p in absent),
            'helper unchanged and new parent absences retained')
    report = dict(schema='ferric-readiness40-scoped-warm-parent-integration-plan-v3',
        parent_terminal=pin(pb['evidence/complete.json']), parent_source_map=pin(pb['evidence/sources-after.json']),
        source_manifest=PROPOSAL, baseline_parent_terminal=BASE_PARENT, baseline_parent_source_map=BASE_PARENT_MAP,
        controller=CONTROLLER, input_manifest=INPUT, emitter=pin(own),
        capsule_manifest=pin(read(directory / 'manifest.json')), patch=pin(patch_raw), files=emitted,
        composed_ferric_sources={n.removeprefix('ferric/'): row for n, row in sorted(desired.items())},
        source_files=1263, worker_files=209, worker_terminal=WORKER_COMPLETE, worker_source_map=WORKER_SOURCES,
        unchanged_full_parent_files={n.removeprefix('ferric/'): desired[n] for n in sorted(unchanged_full)},
        parent_worker_formatter_transitions=[], qualified_worker_bytes_identical_across_compilations=True,
        canonical_changed=False, patch_applied=False, project_execution=False, gpu_execution=False,
        currentness_temporal_equivalence_claim=False, full2303_launch_feasibility=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
    guard(); output.mkdir(mode=0o700)
    for name, raw in [('integrate-qualified.patch', patch_raw),
                      ('integrate-qualified.json', (json.dumps(report, sort_keys=True, indent=2) + '\n').encode())]:
        guard()
        with (output / name).open('xb') as stream:
            stream.write(raw); stream.flush(); os.fsync(stream.fileno())
        require(read(output / name) == raw, 'integration output exact readback')
    guard()
    print(json.dumps(dict(output=str(output), rows=7, composed_sources=1263, worker_files=209,
                          patch=pin(patch_raw), canonical_changed=False)))
    signal.setitimer(signal.ITIMER_REAL, 0)


if __name__ == '__main__':
    main()
