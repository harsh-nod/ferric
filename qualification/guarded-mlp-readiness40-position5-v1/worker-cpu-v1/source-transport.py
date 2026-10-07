"""Data-only position5 transport over the actual qualified Readiness40 worker."""
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
Q = F / 'qualification/guarded-mlp-readiness40-v1/worker-cpu-v1'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-readiness-cli-worker-cpu-v228-v1'
ROOT = E / 'guarded-mlp-readiness40-position5-worker-cpu-v228-v1'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-readiness40-position5-worker-input-v228-v1.tar.gz'
CONTROLLER_PIN = dict(bytes=37504, sha256='7d1e718862c3aecc05d90a22a849566b5d367adb9a781c5646b15624954b639a')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
COMPLETE = dict(bytes=1675803, sha256='2b785c1b8384358f11d547be93614660171c94108aba5e50cdb8db7e4b462b77')
SOURCES = dict(bytes=412739, sha256='8fd9a73ff5954e82a45505e9e3cdd0b072565f2560403f58e6474093fa14ef3c')
PROPOSAL = dict(bytes=7948, sha256='7c0c940e37f519995126a0d8850dcdafcb323a1d1c8dc5573d37bd79cbbf2772')
TEST_MODULES = {
    'src/finite_guarded_mlp_long_wire_v2_tests.rs': 'finite_guarded_mlp_long_wire_v2::tests::',
    'src/finite_guarded_mlp_readiness_wire_v1_tests.rs': 'finite_guarded_mlp_readiness_wire_v1::tests::',
    'src/native_guarded_mlp_readiness_cli_v1_tests.rs': 'native_guarded_mlp_readiness_cli_v1::tests::',
    'src/native_guarded_mlp_readiness_v1_tests.rs': 'native_catalog::forward::guarded_mlp_decode_v1::readiness::tests::',
    'tests/readiness_cli.rs': '',
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
            and pin(bodies['inputs/position5-source-manifest.json']) == PROPOSAL, 'actual base and reviewed source pins')
    base = parse(bodies['inputs/worker-complete.json'])
    source_map = parse(bodies['inputs/worker-sources.json'])
    proposal = parse(bodies['inputs/position5-source-manifest.json'])
    require(base['schema'] == 'ferric-guarded-mlp-readiness-cli-worker-cpu-v1'
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
    require(proposal['schema'] == 'ferric-readiness40-position5-source-v1'
            and proposal['source_only'] is True and len(proposal['files']) == 14
            and proposal['base_commit'] == 'b68a737335508493fbc469c48726ff21af27c002'
            and proposal['old_selected'] == [0, 15, 16, 39] and proposal['diagnostic_selected'] == [0, 5, 16, 39]
            and proposal['forwards'] == 40 and proposal['generated_tokens'] == 0
            and all(proposal[key] is False for key in ('default_profile_changed', 'runtime_changed',
                'kernels_changed', 'scope_limits_changed', 'gpu_execution', 'tests_executed')),
            'reviewed isolated position5 source profile')
    old = {name: compact(row) for name, row in source_map.items() if name.startswith(('fe2o3/', WORKER))}
    require(len(old) == 1008 and sum(name.startswith('fe2o3/') for name in old) == 813
            and all(ordinary(name) and source_map[name]['path'] == str(BASE / name) for name in old),
            'qualified813+195 source closure')
    rows = [row for row in proposal['files'] if row['repository'] == 'ferric'
            and ('ferric/' + row['path']).startswith(WORKER)]
    require(len(rows) == 10 and all(row['before'] is not None for row in rows), 'ten worker replacement rows')
    files, overlay = dict(old), []
    for row in rows:
        name = 'ferric/' + row['path']
        require(ordinary(name) and name not in overlay and old.get(name) == row['before']
                and pin(bodies[name]) == row['after'], 'exact worker preimage and postimage')
        files[name] = row['after']
        overlay.append(name)
    require(WORKER + 'tests/shared_wire.rs' not in overlay, 'existing paired executable regression preserved')
    for name in ('run_cpu.py', 'supervisor.py'):
        files[name] = pin(bodies[name])
    roles = {'worker_complete': 'worker-complete.json', 'worker_sources': 'worker-sources.json',
             'worker_tests': 'worker-tests.stdout', 'worker_list': 'worker-list.stdout',
             'position5_proposal': 'position5-source-manifest.json'}
    require(set(bodies) == set(overlay) | {'run_cpu.py', 'supervisor.py'} | {'inputs/' + name for name in roles.values()},
            'closed17 transport source/lineage bodies; parent sources excluded')
    require(len(files) == 1010 and sum(row['bytes'] for row in files.values()) <= 64 << 20, 'full worker source bounds')
    source_tests = {name.removeprefix(WORKER): names for path, names in proposal['new_tests'].items()
                    if (name := 'ferric/' + path).startswith(WORKER)}
    require(set(source_tests) == set(TEST_MODULES) and all(type(names) is list and len(names) == 1
            for names in source_tests.values()), 'five source test addition rows')
    library = sorted(TEST_MODULES[path] + name for path, names in source_tests.items()
                     if path.startswith('src/') for name in names)
    executable = source_tests['tests/readiness_cli.rs']
    names = library + executable
    require(len(names) == len(set(names)) == 5
            and all(type(name) is str and re.fullmatch('[A-Za-z0-9_:]+', name) for name in names),
            'five unique source-declared test names')
    inputs = dict(schema='ferric-guarded-mlp-readiness40-position5-worker-cpu-input-v1', source_generation=GENERATION,
        files=dict(sorted(files.items())), tool_pins=base['tool_pins'],
        source_lineage={role: dict(path=str(ROOT / 'inputs' / name), **pin(bodies['inputs/' + name])) for role, name in roles.items()},
        worker_overlay=sorted(overlay), new_tests={'worker-lib': library, 'worker-bin-test': [],
                                                'worker-wire-test': [], 'worker-readiness-test': executable})
    return inputs, old

def pack():
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'worker-input-manifest.json'), 'fresh package outputs')
    paths = {'run_cpu.py': P / 'worker_cpu.py', 'supervisor.py': Q / 'supervisor.py',
             'inputs/position5-source-manifest.json': P.parent / 'source-manifest.json'}
    for name, original in (('worker-complete.json', 'complete.json'), ('worker-sources.json', 'sources-after.json'),
                           ('worker-tests.stdout', 'worker-tests.stdout'), ('worker-list.stdout', 'worker-list.stdout')):
        paths['inputs/' + name] = Q / 'evidence' / original
    proposal = parse(read(paths['inputs/position5-source-manifest.json']))
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
    require(len(bodies) == 18 and sum(map(len, bodies.values())) <= 16 << 20, 'bounded18-member input')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(raw), 0o600, 0
                tar.addfile(info, io.BytesIO(raw))
    with (P / 'worker-input-manifest.json').open('xb') as stream:
        stream.write(input_raw)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))), input=pin(input_raw),
                         members=18, source_files=1010), sort_keys=True))


def stage(archive_sha, input_sha):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(E.resolve(strict=True) == E and BASE.resolve(strict=True) == BASE and not os.path.lexists(ROOT), 'fresh exact worker root')
    require(os.statvfs(E).f_bavail * os.statvfs(E).f_frsize >= 40 << 30, 'initial40GiB floor')
    archive_raw = read(E / BASENAME)
    require(pin(archive_raw)['sha256'] == archive_sha, 'actual source archive')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == len({m.name for m in members}) == 18
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
    paths = []
    for subtree in ('fe2o3', 'ferric'):
        for directory, dirs, names in os.walk(BASE / subtree, followlinks=False,
                onerror=lambda error: (_ for _ in ()).throw(error)):
            require(all(not (Path(directory) / name).is_symlink() and name not in ('.git', 'target') for name in dirs), 'ordinary source directory')
            paths.extend(Path(directory) / name for name in names)
    require({str(path.relative_to(BASE)) for path in paths} == set(old), 'full1008 predecessor source roster')
    require(all(pin(read(BASE / name)) == row for name, row in old.items()), 'all predecessor source hashes')
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
            raw = read(BASE / name)
            require(pin(raw) == row, 'preimage stable during copy')
            write(name, raw)
    for name, raw in sorted(bodies.items()):
        write(name, raw)
    write('input-manifest.json', raw_input)
    require(all(pin(read(ROOT / name)) == row for name, row in expected['files'].items()), 'all1010 staged sources')
    require(all(pin(read(BASE / name)) == row for name, row in old.items())
            and read(BASE / 'evidence/complete.json') == bodies['inputs/worker-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/worker-sources.json'], 'immutable base posthash')
    receipt = dict(schema='ferric-guarded-mlp-readiness40-position5-worker-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=1010, runtime_files=813, worker_files=195,
        copied_unchanged_files=998, overlay_files=10, helpers=2, project_execution=False,
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

