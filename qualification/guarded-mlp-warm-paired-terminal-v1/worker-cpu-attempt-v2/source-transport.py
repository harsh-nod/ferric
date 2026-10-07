"""Data-only warm terminal transport over two separately qualified source maps."""
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
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/guarded-mlp-readiness40-position5-v1/worker-cpu-v1'
RQ = F / 'qualification/guarded-mlp-terminal-pair-v1/cpu-v1'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-readiness40-position5-worker-cpu-v228-v1'
RUNTIME_BASE = E / 'guarded-mlp-terminal-pair-cpu-v228-v1'
ROOT = E / 'guarded-mlp-warm-paired-terminal-worker-cpu-v228-v2'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-warm-paired-terminal-worker-input-v228-v2.tar.gz'
CONTROLLER_PIN = dict(bytes=39374, sha256='23e70479adaee68c480213d3c57fbfd8d80bdb573f11d91362eda7fe6afa7ac7')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
COMPLETE = dict(bytes=1704432, sha256='728339a954436b3e191e5efac8c42146521f7acd57c73ac81dc0d4ad55cdd421')
SOURCES = dict(bytes=420819, sha256='c75c35bd7c987a4e52f1897a3d7afd2923e32b6d4768e5420473aa3c4527ce6c')
PROPOSAL = dict(bytes=10821, sha256='dcf31abd141f0a6f15e0585a2660e28f35d50084dc8d0a54916dbd925fae8e55')
RUNTIME_COMPLETE = dict(bytes=2250677, sha256='bf0fa20a37ff3bf748ef5984df0dceb5cd1c453ef5c05c7111779b31cda33720')
RUNTIME_SOURCES = dict(bytes=407038, sha256='f8024397177d83cdd1d07040b55c97afc5190988676bb80f67a9af4ae0554718')
TEST_MODULES = {
    'src/finite_guarded_mlp_decode_wire_v1_tests.rs': 'finite_guarded_mlp_decode_wire_v1::tests::',
    'src/native_guarded_mlp_decode_v1/tests.rs': 'native_catalog::forward::guarded_mlp_decode_v1::tests::',
    'src/state_roster/guarded_mlp_decode_v1/tests.rs': 'state_roster::guarded_mlp_decode_v1::tests::',
    'src/native_guarded_mlp_decode_cli_v1_tests.rs': 'native_guarded_mlp_decode_cli_v1::tests::',
    'tests/shared_wire.rs': '',
}
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
            and pin(bodies['inputs/terminal-source-manifest.json']) == PROPOSAL, 'actual base and reviewed source pins')
    require(pin(bodies['inputs/runtime-complete.json']) == RUNTIME_COMPLETE
            and pin(bodies['inputs/runtime-sources.json']) == RUNTIME_SOURCES, 'actual terminal runtime pins')
    runtime = parse(bodies['inputs/runtime-complete.json'])
    runtime_map = parse(bodies['inputs/runtime-sources.json'])
    base = parse(bodies['inputs/worker-complete.json'])
    source_map = parse(bodies['inputs/worker-sources.json'])
    proposal = parse(bodies['inputs/terminal-source-manifest.json'])
    require(base['schema'] == 'ferric-guarded-mlp-readiness40-position5-worker-cpu-v1'
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['source_unchanged'] is True and base['input_sources'] == base['final_sources'] == source_map
            and len(source_map) == 1010 and compact(base['raw']['sources-after.json']) == SOURCES
            and base['gpu_execution'] is False and base['runtime_source_qualified_by_paired_read_cpu'] is True
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
    require(proposal['schema'] == 'ferric-guarded-mlp-warm-paired-terminal-source-v1'
            and len(proposal['files']) == 14 and proposal['base_commit'] == '0586e3852b2198703f4280264de7e89bf9034606'
            and proposal['paired_terminal_dispatches'] == [0, 0, 36, 36]
            and proposal['selected_terminal_cadence_changed'] is True
            and all(proposal[key] is False for key in ('default_profile_changed', 'global_currentness_policy_changed',
                'hidden_read_policy_changed', 'runtime_source_changed', 'long_readiness_enabled',
                'project_execution', 'native_execution', 'numerical_acceptance', 'performance_claim')),
            'reviewed isolated warm terminal source')
    old = {name: compact(row) for name, row in runtime_map.items() if name.startswith('fe2o3/')}
    require(len(old) == 815 and all(ordinary(name) and runtime_map[name]['path'] == str(RUNTIME_BASE / name)
            for name in old), 'qualified815 runtime closure')
    worker = {name: compact(row) for name, row in source_map.items() if name.startswith(WORKER)}
    require(len(worker) == 195 and all(ordinary(name) and source_map[name]['path'] == str(BASE / name)
            for name in worker), 'qualified195 current worker closure')
    old.update(worker)
    rows = [row for row in proposal['files'] if row['repository'] == 'ferric'
            and ('ferric/' + row['path']).startswith(WORKER)]
    require(len(rows) == 11 and all(row['before'] is not None for row in rows), 'eleven worker replacement rows')
    files, overlay = dict(old), []
    for row in rows:
        name = 'ferric/' + row['path']
        require(ordinary(name) and name not in overlay and old.get(name) == row['before']
                and pin(bodies[name]) == row['after'], 'exact worker preimage and postimage')
        files[name] = row['after']
        overlay.append(name)
    require(WORKER + 'tests/shared_wire.rs' in overlay, 'additive executable regression in existing target')
    for name in ('run_cpu.py', 'supervisor.py'):
        files[name] = pin(bodies[name])
    roles = {'worker_complete': 'worker-complete.json', 'worker_sources': 'worker-sources.json',
             'worker_tests': 'worker-tests.stdout', 'worker_list': 'worker-list.stdout',
             'terminal_proposal': 'terminal-source-manifest.json',
             'runtime_complete': 'runtime-complete.json', 'runtime_sources': 'runtime-sources.json'}
    require(set(bodies) == set(overlay) | {'run_cpu.py', 'supervisor.py'} | {'inputs/' + name for name in roles.values()},
            'closed20 transport source/lineage bodies; parent sources excluded')
    require(len(files) == 1012 and sum(row['bytes'] for row in files.values()) <= 64 << 20, 'full worker source bounds')
    library = proposal['new_tests']['worker_library']
    executable = proposal['new_tests']['worker_integration']
    require(len(library) == 9 and len(executable) == 1, 'ten exact source test additions')
    names = library + executable
    prior = set(re.findall(r'^([A-Za-z0-9_:]+): test$', bodies['inputs/worker-list.stdout'].decode(), re.M))
    actual_added = []
    for relative, prefix in TEST_MODULES.items():
        tests = re.findall(r'#\[test\]\s*fn\s+(\w+)', bodies[WORKER + relative].decode())
        actual_added += [prefix + name for name in tests if prefix + name not in prior]
    require(sorted(actual_added) == sorted(names) and len(names) == len(set(names)) == 10
            and all(type(name) is str and re.fullmatch('[A-Za-z0-9_:]+', name) for name in names),
            'ten unique actual source-declared new names')
    inputs = dict(schema='ferric-guarded-mlp-warm-paired-terminal-worker-cpu-input-v1', source_generation=GENERATION,
        files=dict(sorted(files.items())), tool_pins=base['tool_pins'],
        source_lineage={role: dict(path=str(ROOT / 'inputs' / name), **pin(bodies['inputs/' + name])) for role, name in roles.items()},
        worker_overlay=sorted(overlay), new_tests={'worker-lib': library, 'worker-bin-test': [],
                                                'worker-wire-test': executable, 'worker-readiness-test': []})
    return inputs, old

def pack():
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'worker-input-manifest.json'), 'fresh package outputs')
    paths = {'run_cpu.py': P / 'worker_cpu.py', 'supervisor.py': Q / 'supervisor.py',
             'inputs/terminal-source-manifest.json': P.parent / 'source-manifest.json'}
    for name, original in (('worker-complete.json', 'complete.json'), ('worker-sources.json', 'sources-after.json'),
                           ('worker-tests.stdout', 'worker-tests.stdout'), ('worker-list.stdout', 'worker-list.stdout')):
        paths['inputs/' + name] = Q / 'evidence' / original
    for name, original in (('runtime-complete.json', 'complete.json'), ('runtime-sources.json', 'sources-after.json')):
        paths['inputs/' + name] = RQ / 'evidence' / original
    proposal = parse(read(paths['inputs/terminal-source-manifest.json']))
    for row in proposal['files']:
        if row['repository'] != 'ferric' or not ('ferric/' + row['path']).startswith(WORKER):
            continue
        name = 'ferric/' + row['path']
        require(name not in paths, 'disjoint source rows')
        paths[name] = P.parent / 'ferric' / row['path']
    bodies = {name: read(path) for name, path in paths.items()}
    inputs, old = contract(bodies)
    for name, row in old.items():
        if name.startswith(WORKER):
            require(pin(read(F / name.removeprefix('ferric/'))) == row, 'all195 canonical preimages match actual Readiness40 worker')
    require(all(read(path) == bodies[name] for name, path in paths.items()), 'package source posthash')
    input_raw = encoded(inputs)
    bodies['input-manifest.json'] = input_raw
    require(len(bodies) == 21 and sum(map(len, bodies.values())) <= 16 << 20, 'bounded21-member input')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(raw), 0o600, 0
                tar.addfile(info, io.BytesIO(raw))
    with (P / 'worker-input-manifest.json').open('xb') as stream:
        stream.write(input_raw)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))), input=pin(input_raw),
                         members=21, source_files=1012), sort_keys=True))


def stage(archive_sha, input_sha):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(E.resolve(strict=True) == E and BASE.resolve(strict=True) == BASE
            and RUNTIME_BASE.resolve(strict=True) == RUNTIME_BASE and not os.path.lexists(ROOT), 'fresh exact worker root')
    require(os.statvfs(E).f_bavail * os.statvfs(E).f_frsize >= 40 << 30, 'initial40GiB floor')
    archive_raw = read(E / BASENAME)
    require(pin(archive_raw)['sha256'] == archive_sha, 'actual source archive')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == len({m.name for m in members}) == 21
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
    require(set(paths) == set(old) and len(paths) == len(old), 'full1010 composed predecessor source roster')
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
    require(all(pin(read(ROOT / name)) == row for name, row in expected['files'].items()), 'all1012 staged sources')
    require(all(pin(read(source(name))) == row for name, row in old.items())
            and read(BASE / 'evidence/complete.json') == bodies['inputs/worker-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/worker-sources.json']
            and read(RUNTIME_BASE / 'evidence/complete.json') == bodies['inputs/runtime-complete.json']
            and read(RUNTIME_BASE / 'evidence/sources-after.json') == bodies['inputs/runtime-sources.json'], 'immutable two-base posthash')
    receipt = dict(schema='ferric-guarded-mlp-warm-paired-terminal-worker-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=1012, runtime_files=815, worker_files=195,
        copied_unchanged_files=999, overlay_files=11, helpers=2, project_execution=False,
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
