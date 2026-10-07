"""Data-only SharedFull readiness transport over actual Full worker/runtime source maps."""
import ast
import hashlib
import io
import json
import os
from pathlib import Path
import re
import signal
import stat
import sys
import tarfile

P = Path(__file__).resolve().parent
PROPOSAL_ROOT = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/proposals/guarded-mlp-model-interface-v228-v1/model-guarded-full2303-v1/shared-full-readiness40-v1')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/guarded-mlp-full2303-v1/worker-cpu-v1'
RQ = F / 'qualification/guarded-mlp-terminal-pair-v1/cpu-v1'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-full2303-worker-cpu-v228-v1'
RUNTIME_BASE = E / 'guarded-mlp-terminal-pair-cpu-v228-v1'
ROOT = E / 'guarded-mlp-readiness40-shared-full-worker-cpu-v228-v1'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-readiness40-shared-full-worker-input-v228-v1.tar.gz'
CONTROLLER_PIN = dict(bytes=41316, sha256='4d12ba86c4014adaaea1c5c49c3790dffcca78af8326ae09cb085840347a518a')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
COMPLETE = dict(bytes=1691615, sha256='0b91b220824f093b0297134ff21e4ddc2dde64fd17909439bb15421fb862e55f')
SOURCES = dict(bytes=412019, sha256='0fc0a33c61dd0e6f70903bb246654bfeb99095fd515be53c828d8d91f646e7cb')
PROPOSAL = dict(bytes=12074, sha256='fec2fadc94454afb00f538f158f65bde4526e93da2cdda699a2ee3972b511c85')
RUNTIME_COMPLETE = dict(bytes=2250677, sha256='bf0fa20a37ff3bf748ef5984df0dceb5cd1c453ef5c05c7111779b31cda33720')
RUNTIME_SOURCES = dict(bytes=407038, sha256='f8024397177d83cdd1d07040b55c97afc5190988676bb80f67a9af4ae0554718')
TEST_MODULES = {
    'src/finite_guarded_mlp_readiness_shared_v1_tests.rs': 'finite_guarded_mlp_readiness_shared_v1::tests::',
    'src/native_guarded_mlp_readiness_cli_v1_tests.rs': 'native_guarded_mlp_readiness_cli_v1::tests::',
    'tests/readiness_cli.rs': '',
}
SOURCE_REVISION = '2acc55e4d509f8fadc70beb7aa3f39e79660580c'
WORKER = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
GENERATION = '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def parse(raw):
    def pairs(rows):
        result = {}
        for name, value in rows:
            require(name not in result, 'duplicate JSON key')
            result[name] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical source')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and 0 <= before.st_size <= 64 << 20,
                'bounded ordinary source')
        body = stream.read((64 << 20) + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(body) == before.st_size, 'source read drift')
    return body


def ordinary(name):
    return type(name) is str and Path(name).as_posix() == name and not Path(name).is_absolute() \
        and '..' not in Path(name).parts and name not in ('', '.')


def contract(bodies):
    require(CONTROLLER_PIN is not None and pin(bodies['run_cpu.py']) == CONTROLLER_PIN
            and pin(bodies['supervisor.py']) == SUPERVISOR_PIN, 'exact reviewed controller and supervisor')
    require(pin(bodies['inputs/worker-complete.json']) == COMPLETE
            and pin(bodies['inputs/worker-sources.json']) == SOURCES
            and pin(bodies['inputs/shared-source-manifest.json']) == PROPOSAL, 'actual base and reviewed source pins')
    require(pin(bodies['inputs/runtime-complete.json']) == RUNTIME_COMPLETE
            and pin(bodies['inputs/runtime-sources.json']) == RUNTIME_SOURCES, 'actual terminal runtime pins')
    runtime = parse(bodies['inputs/runtime-complete.json'])
    runtime_map = parse(bodies['inputs/runtime-sources.json'])
    base = parse(bodies['inputs/worker-complete.json'])
    source_map = parse(bodies['inputs/worker-sources.json'])
    proposal = parse(bodies['inputs/shared-source-manifest.json'])
    require(base['schema'] == 'ferric-guarded-mlp-full2303-worker-cpu-v1'
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['source_unchanged'] is True and base['input_sources'] == base['final_sources'] == source_map
            and len(source_map) == 1020 and compact(base['raw']['sources-after.json']) == SOURCES
            and base['gpu_execution'] is False and base['runtime_source_qualified_by_terminal_cpu'] is True
            and base['full2303_source_added'] is True and base['full2303_native_enabled'] is True
            and base['full2303_native_execution'] is False and base['full2303_launch_feasibility'] is False
            and base['full2303_source_abort_ms'] == 3600000
            and base['causal_layer0_source_added'] is True
            and base['causal_layer0_native_execution'] is False
            and base['causal_capture_positions'] == list(range(6)) and base['causal_layer'] == 0
            and base['inherited_warm_paired_terminal_route_preserved'] is True
            and base['warm_paired_terminal_source_added'] is True
            and base['readiness_cli_source_added'] is True and base['readiness40_native_execution'] is False
            and base['paired_read_cli_dispatch_fixed'] is True and base['cli_executable_unchanged_across_tests'] is True
            and base['cli_executable_before_tests']['pin'] == base['artifacts']['worker']['pin'],
            'actual qualified Readiness40 runtime/worker and executable CLI regression')
    for name in ('worker-tests.stdout', 'worker-list.stdout'):
        require(pin(bodies['inputs/' + name]) == compact(base['raw'][name]), 'actual worker raw baseline')
    require(runtime['schema'] == 'ferric-guarded-mlp-terminal-pair-cpu-v1'
            and runtime['passed'] is True and runtime['failure'] is None and runtime['postcheck_errors'] == []
            and runtime['input_sources'] == runtime['final_sources'] == runtime_map
            and runtime['source_unchanged'] is True and len(runtime_map) == 1014
            and compact(runtime['raw']['sources-after.json']) == RUNTIME_SOURCES
            and runtime['full_runtime_tests_executed'] is True and runtime['terminal_pair_runtime_added'] is True
            and runtime['global_currentness_policy_changed'] is False and runtime['gpu_execution'] is False
            and runtime['tool_pins'] == base['tool_pins'], 'actual815 terminal runtime qualification')
    require(proposal['schema'] == 'ferric-readiness40-position5-shared-full-source-v1'
            and len(proposal['files']) == 12 and proposal['base']['ferric_commit'] == SOURCE_REVISION
            and proposal['conditional_census']['worker'] == dict(ignored=4, inventory=708,
                new_library=9, new_readiness_integration=1, passed=704, phases=9,
                runtime_sources=815, source_rows=1022, worker_sources=205)
            and proposal['scope']['policy_record_max_bytes'] == 4096
            and proposal['scope']['shared_full_currentness'] is True
            and proposal['scope']['selected_currentness_policy_changed'] is True
            and all(proposal['scope'][key] is False for key in ('cache_kernel_admission', 'compiled', 'deadline_changed', 'default_policy_changed', 'full2303_native_enabled_by_this_proposal', 'host_observer', 'native_execution', 'numerical_acceptance', 'operational_currentness', 'ordinary_observation_schema_changed', 'paired_hidden_reads', 'paired_terminal', 'performance_claim', 'pure_long_profile_changed', 'retention_cap_changed', 'runtime_changed', 'tests_executed', 'wire_changed')),
            'reviewed explicit SharedFull readiness source only; default and runtime unchanged')
    require({role: compact(proposal['base'][role]) for role in ('worker_complete', 'worker_sources')} ==
            {'worker_complete': COMPLETE, 'worker_sources': SOURCES},
            'literal actual Full worker predecessor receipt and source map')
    old = {name: compact(row) for name, row in runtime_map.items() if name.startswith('fe2o3/')}
    require(len(old) == 815 and all(ordinary(name) and runtime_map[name]['path'] == str(RUNTIME_BASE / name)
            for name in old), 'qualified815 runtime closure')
    worker = {name: compact(row) for name, row in source_map.items() if name.startswith(WORKER)}
    require(len(worker) == 203 and all(ordinary(name) and source_map[name]['path'] == str(BASE / name)
            for name in worker), 'qualified203 current worker closure')
    old.update(worker)
    rows = [row for row in proposal['files'] if ('ferric/' + row['path']).startswith(WORKER)]
    require(len(rows) == 7 and sum(row['before'] is None for row in rows) == 2, 'five worker replacements and two additions')
    files, overlay = dict(old), []
    for row in rows:
        name = 'ferric/' + row['path']
        require(ordinary(name) and name not in overlay and old.get(name) == row['before']
                and pin(bodies[name]) == row['after'], 'exact worker preimage and postimage')
        files[name] = row['after']
        overlay.append(name)
    require(WORKER + 'tests/readiness_cli.rs' in overlay, 'additive executable regression in existing target')
    for name in ('run_cpu.py', 'supervisor.py'):
        files[name] = pin(bodies[name])
    roles = {'worker_complete': 'worker-complete.json', 'worker_sources': 'worker-sources.json',
             'worker_tests': 'worker-tests.stdout', 'worker_list': 'worker-list.stdout',
             'shared_proposal': 'shared-source-manifest.json',
             'runtime_complete': 'runtime-complete.json', 'runtime_sources': 'runtime-sources.json'}
    require(set(bodies) == set(overlay) | {'run_cpu.py', 'supervisor.py'} | {'inputs/' + name for name in roles.values()},
            'closed16 transport source/lineage bodies; parent sources excluded')
    require(len(files) == 1022 and sum(row['bytes'] for row in files.values()) <= 64 << 20, 'full worker source bounds')
    declared_library = proposal['new_tests']['worker_library']
    executable = proposal['new_tests']['worker_readiness_cli_integration']
    prior = set(re.findall(r'^([A-Za-z0-9_:]+): test$', bodies['inputs/worker-list.stdout'].decode(), re.M))
    actual_added = []
    for relative, prefix in TEST_MODULES.items():
        tests = re.findall(r'#\[test\]\s*fn\s+(\w+)', bodies[WORKER + relative].decode())
        actual_added += [prefix + name for name in tests if prefix + name not in prior]
    library = sorted(name for name in actual_added if '::' in name)
    names = library + executable
    require(sorted(name.rsplit('::', 1)[-1] for name in library) == sorted(declared_library)
            and len(library) == 9 and len(executable) == 1
            and sorted(actual_added) == sorted(names) and len(names) == len(set(names)) == 10
            and all(type(name) is str and re.fullmatch('[A-Za-z0-9_:]+', name) for name in names),
            'ten unique actual source-declared SharedFull readiness additions')
    inputs = dict(schema='ferric-guarded-mlp-readiness40-shared-full-worker-cpu-input-v1', source_generation=GENERATION,
        files=dict(sorted(files.items())), tool_pins=base['tool_pins'],
        source_lineage={role: dict(path=str(ROOT / 'inputs' / name), **pin(bodies['inputs/' + name])) for role, name in roles.items()},
        worker_overlay=sorted(overlay), new_tests={'worker-lib': library, 'worker-bin-test': [],
                                                'worker-wire-test': [], 'worker-readiness-test': executable})
    return inputs, old

def pack():
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'worker-input-manifest.json'), 'fresh package outputs')
    paths = {'run_cpu.py': P / 'worker_cpu.py', 'supervisor.py': Q / 'supervisor.py',
             'inputs/shared-source-manifest.json': PROPOSAL_ROOT / 'source-manifest.json'}
    for name, original in (('worker-complete.json', 'complete.json'), ('worker-sources.json', 'sources-after.json'),
                           ('worker-tests.stdout', 'worker-tests.stdout'), ('worker-list.stdout', 'worker-list.stdout')):
        paths['inputs/' + name] = Q / 'evidence' / original
    for name, original in (('runtime-complete.json', 'complete.json'), ('runtime-sources.json', 'sources-after.json')):
        paths['inputs/' + name] = RQ / 'evidence' / original
    proposal = parse(read(paths['inputs/shared-source-manifest.json']))
    for row in proposal['files']:
        if not ('ferric/' + row['path']).startswith(WORKER):
            continue
        name = 'ferric/' + row['path']
        require(name not in paths, 'disjoint source rows')
        paths[name] = PROPOSAL_ROOT / 'ferric' / row['path']
    bodies = {name: read(path) for name, path in paths.items()}
    inputs, old = contract(bodies)
    for name, row in old.items():
        if name.startswith(WORKER):
            require(pin(read(F / name.removeprefix('ferric/'))) == row, 'all203 canonical preimages match actual Full worker')
    require(all(read(path) == bodies[name] for name, path in paths.items()), 'package source posthash')
    input_raw = encoded(inputs)
    bodies['input-manifest.json'] = input_raw
    require(len(bodies) == 17 and sum(map(len, bodies.values())) <= 16 << 20, 'bounded17-member input')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(raw), 0o600, 0
                tar.addfile(info, io.BytesIO(raw))
    with (P / 'worker-input-manifest.json').open('xb') as stream:
        stream.write(input_raw)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))), input=pin(input_raw),
                         members=17, source_files=1022), sort_keys=True))


def stage(archive_sha, input_sha):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(E.resolve(strict=True) == E and BASE.resolve(strict=True) == BASE
            and RUNTIME_BASE.resolve(strict=True) == RUNTIME_BASE and not os.path.lexists(ROOT), 'fresh exact worker root')
    require(os.statvfs(E).f_bavail * os.statvfs(E).f_frsize >= 40 << 30, 'initial40GiB floor')
    archive_raw = read(E / BASENAME)
    require(pin(archive_raw)['sha256'] == archive_sha, 'actual source archive')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == len({m.name for m in members}) == 17
                and all(m.isfile() and ordinary(m.name) and not m.pax_headers and 0 <= m.size <= 4 << 20 for m in members)
                and sum(m.size for m in members) <= 16 << 20, 'closed bounded regular USTAR bodies')
        bodies = {m.name: tar.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'member extents')
    raw_input = bodies.pop('input-manifest.json')
    require(pin(raw_input)['sha256'] == input_sha, 'actual source input hash')
    expected, old = contract(bodies)
    require(parse(raw_input) == expected, 'entire reconstructed worker input')
    require(read(BASE / 'evidence/complete.json') == bodies['inputs/worker-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/worker-sources.json'], 'immutable qualified worker receipts')
    require(read(RUNTIME_BASE / 'evidence/complete.json') == bodies['inputs/runtime-complete.json']
            and read(RUNTIME_BASE / 'evidence/sources-after.json') == bodies['inputs/runtime-sources.json'], 'immutable qualified runtime receipts')
    paths = []
    for source_root, subtree in ((RUNTIME_BASE, 'fe2o3'), (BASE, 'ferric')):
        for directory, dirs, names in os.walk(source_root / subtree, followlinks=False,
                onerror=lambda error: (_ for _ in ()).throw(error)):
            require(all(not (Path(directory) / name).is_symlink() and name not in ('.git', 'target') for name in dirs), 'ordinary source directory')
            paths.extend(str((Path(directory) / name).relative_to(source_root)) for name in names)
    require(set(paths) == set(old) and len(paths) == len(old), 'full1018 composed predecessor source roster')
    source = lambda name: (RUNTIME_BASE if name.startswith('fe2o3/') else BASE) / name
    require(all(pin(read(source(name))) == row for name, row in old.items()), 'all composed predecessor source hashes')
    ROOT.mkdir(mode=0o700)
    def write(name, raw):
        require(ordinary(name), 'ordinary stage destination')
        path = ROOT / name
        path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        require(path.parent.resolve(strict=True) == path.parent, 'stage parent alias')
        with path.open('xb') as stream:
            stream.write(raw)
    for name, row in old.items():
        if name not in bodies:
            raw = read(source(name))
            require(pin(raw) == row, 'preimage stable during copy')
            write(name, raw)
    for name, raw in sorted(bodies.items()):
        write(name, raw)
    write('input-manifest.json', raw_input)
    require(all(pin(read(ROOT / name)) == row for name, row in expected['files'].items()), 'all1022 staged sources')
    require(all(pin(read(source(name))) == row for name, row in old.items())
            and read(BASE / 'evidence/complete.json') == bodies['inputs/worker-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/worker-sources.json']
            and read(RUNTIME_BASE / 'evidence/complete.json') == bodies['inputs/runtime-complete.json']
            and read(RUNTIME_BASE / 'evidence/sources-after.json') == bodies['inputs/runtime-sources.json'], 'immutable two-base posthash')
    receipt = dict(schema='ferric-guarded-mlp-readiness40-shared-full-worker-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=1022, runtime_files=815, worker_files=205,
        copied_unchanged_files=1013, overlay_files=7, helpers=2, project_execution=False,
        canonical_changed=False, shared_cache_changed=False, lockfiles_changed=False,
        root=str(ROOT), controller=pin(read(Path(__file__).resolve())))
    write('stage-complete.json', encoded(receipt))
    print(json.dumps(receipt, sort_keys=True))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode, 'python3 -B worker_transport.py pack | stage ARCHIVE_SHA INPUT_SHA')
    os.umask(0o077)
    def interrupted(number, _frame):
        raise RuntimeError('worker transport signal ' + str(number))
    for number in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(number, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 180)
    try:
        if sys.argv[1:] == ['pack']:
            pack()
        else:
            require(len(sys.argv) == 4 and sys.argv[1] == 'stage'
                    and all(re.fullmatch('[0-9a-f]{64}', value) for value in sys.argv[2:]), 'closed transport CLI')
            stage(*sys.argv[2:])
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)

