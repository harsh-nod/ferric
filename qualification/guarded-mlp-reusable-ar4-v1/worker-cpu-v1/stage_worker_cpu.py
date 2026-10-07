"""Data-only reusable-AR4 worker overlay over the actual runtime V2 tree."""
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
Q = F / 'qualification/guarded-mlp-reusable-arena-v1/cpu-attempt-v2'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-reusable-arena-cpu-v228-v2'
ROOT = E / 'guarded-mlp-reusable-ar4-worker-cpu-v228-v1'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-reusable-ar4-worker-input-v228-v1.tar.gz'
CONTROLLER_PIN = dict(bytes=33459, sha256='0e373bb466649e669a593cfd8d860a621e106f0a4079ec4c561648862e16496a')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
COMPLETE = dict(bytes=2187577, sha256='032c4ac98bee0b6c551f1b98fb12d7966bd4f9fadc6b0b8570ab2488634c7814')
SOURCES = dict(bytes=401732, sha256='efe370a940d49aaeb173fdf9a245c6a19f5561852d2fb535d929fae93db30da7')
PROPOSAL = dict(bytes=14597, sha256='f3394e81b8b5d177d77db90e2bcfd8f2658c72685d39c710c129e7dfe71fc230')
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
    require(pin(bodies['inputs/runtime-complete.json']) == COMPLETE
            and pin(bodies['inputs/runtime-sources.json']) == SOURCES
            and pin(bodies['inputs/reuse-source-manifest.json']) == PROPOSAL, 'actual base and reviewed route pins')
    base = parse(bodies['inputs/runtime-complete.json'])
    source_map = parse(bodies['inputs/runtime-sources.json'])
    proposal = parse(bodies['inputs/reuse-source-manifest.json'])
    require(base['schema'] == 'ferric-guarded-mlp-reusable-arena-cpu-v1'
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['source_unchanged'] is True and base['input_sources'] == base['final_sources'] == source_map
            and len(source_map) == 999 and compact(base['raw']['sources-after.json']) == SOURCES
            and base['gpu_execution'] is False, 'actual qualified reusable runtime and worker')
    for name in ('worker-tests.stdout', 'worker-list.stdout'):
        require(pin(bodies['inputs/' + name]) == compact(base['raw'][name]), 'actual worker raw baseline')
    require(proposal['schema'] == 'ferric-guarded-mlp-reusable-ar4-source-v1' and len(proposal['files']) == 15
            and proposal['runtime_source_manifest']['sha256'] == '4c08a6338231a98310ff63ea34d0ff89775c670fecc5069bbfac3eb5e85e8015',
            'exact reusable route source proposal')
    old = {name: compact(row) for name, row in source_map.items() if name.startswith(('fe2o3/', WORKER))}
    require(len(old) == 995 and sum(name.startswith('fe2o3/') for name in old) == 811
            and all(ordinary(name) and source_map[name]['path'] == str(BASE / name) for name in old), 'qualified811+184 source closure')
    files, overlay = dict(old), []
    for row in proposal['files']:
        name = 'ferric/' + row['path']
        if not name.startswith(WORKER):
            continue
        require(ordinary(name) and old.get(name) == row['before'] and row['before'] is not None
                and pin(bodies[name]) == row['after'], 'exact worker replacement bytes')
        files[name] = row['after']
        overlay.append(name)
    require(len(overlay) == len(set(overlay)) == 12, 'twelve worker replacements, no new files')
    for name in ('run_cpu.py', 'supervisor.py'):
        files[name] = pin(bodies[name])
    roles = {'runtime_complete': 'runtime-complete.json', 'runtime_sources': 'runtime-sources.json',
             'worker_tests': 'worker-tests.stdout', 'worker_list': 'worker-list.stdout', 'reuse_proposal': 'reuse-source-manifest.json'}
    require(set(bodies) == set(overlay) | {'run_cpu.py', 'supervisor.py'} | {'inputs/' + name for name in roles.values()},
            'closed nineteen transport source/lineage bodies')
    require(len(files) == 997 and sum(row['bytes'] for row in files.values()) <= 64 << 20, 'full worker source bounds')
    inputs = dict(schema='ferric-guarded-mlp-reusable-ar4-worker-cpu-input-v1', source_generation=GENERATION,
        files=dict(sorted(files.items())), tool_pins=base['tool_pins'],
        source_lineage={role: dict(path=str(ROOT / 'inputs' / name), **pin(bodies['inputs/' + name])) for role, name in roles.items()},
        worker_overlay=sorted(overlay), new_tests={'worker-lib': proposal['new_tests']['worker_library'],
                                                  'worker-bin-test': [], 'worker-wire-test': []})
    require(len(inputs['new_tests']['worker-lib']) == 8, 'eight source-declared new tests')
    return inputs, old


def pack():
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'worker-input-manifest.json'), 'fresh package outputs')
    paths = {'run_cpu.py': P / 'worker_cpu.py', 'supervisor.py': Q / 'supervisor.py',
             'inputs/reuse-source-manifest.json': P.parent / 'source-manifest.json'}
    for name, original in (('runtime-complete.json', 'complete.json'), ('runtime-sources.json', 'sources-after.json'),
                           ('worker-tests.stdout', 'worker-tests.stdout'), ('worker-list.stdout', 'worker-list.stdout')):
        paths['inputs/' + name] = Q / 'evidence' / original
    proposal = parse(read(paths['inputs/reuse-source-manifest.json']))
    for row in proposal['files']:
        name = 'ferric/' + row['path']
        if name.startswith(WORKER):
            paths[name] = P.parent / 'ferric' / row['path']
    bodies = {name: read(path) for name, path in paths.items()}
    inputs, old = contract(bodies)
    for name, row in old.items():
        if name.startswith(WORKER):
            require(pin(read(F / name.removeprefix('ferric/'))) == row, 'all184 canonical preimages match actual runtime worker')
    require(all(read(path) == bodies[name] for name, path in paths.items()), 'package source posthash')
    input_raw = encoded(inputs)
    bodies['input-manifest.json'] = input_raw
    require(len(bodies) == 20 and sum(map(len, bodies.values())) <= 16 << 20, 'bounded twenty-member input')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(raw), 0o600, 0
                tar.addfile(info, io.BytesIO(raw))
    with (P / 'worker-input-manifest.json').open('xb') as stream:
        stream.write(input_raw)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))), input=pin(input_raw),
                         members=20, source_files=997), sort_keys=True))


def stage(archive_sha, input_sha):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(E.resolve(strict=True) == E and BASE.resolve(strict=True) == BASE and not os.path.lexists(ROOT), 'fresh exact worker root')
    require(os.statvfs(E).f_bavail * os.statvfs(E).f_frsize >= 40 << 30, 'initial40GiB floor')
    archive_raw = read(E / BASENAME)
    require(pin(archive_raw)['sha256'] == archive_sha, 'actual source archive')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == len({m.name for m in members}) == 20
                and all(m.isfile() and ordinary(m.name) and not m.pax_headers and 0 <= m.size <= 4 << 20 for m in members)
                and sum(m.size for m in members) <= 16 << 20, 'closed bounded regular USTAR bodies')
        bodies = {m.name: tar.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'member extents')
    raw_input = bodies.pop('input-manifest.json')
    require(pin(raw_input)['sha256'] == input_sha, 'actual source input hash')
    expected, old = contract(bodies)
    require(parse(raw_input) == expected, 'entire reconstructed worker input')
    require(read(BASE / 'evidence/complete.json') == bodies['inputs/runtime-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/runtime-sources.json'], 'immutable qualified runtime receipts')
    paths = []
    for subtree in ('fe2o3', 'ferric'):
        for directory, dirs, names in os.walk(BASE / subtree, followlinks=False,
                onerror=lambda error: (_ for _ in ()).throw(error)):
            require(all(not (Path(directory) / name).is_symlink() and name not in ('.git', 'target') for name in dirs), 'ordinary source directory')
            paths.extend(Path(directory) / name for name in names)
    require({str(path.relative_to(BASE)) for path in paths} == set(old), 'full995 predecessor source roster')
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
    require(all(pin(read(ROOT / name)) == row for name, row in expected['files'].items()), 'all997 staged sources')
    require(all(pin(read(BASE / name)) == row for name, row in old.items())
            and read(BASE / 'evidence/complete.json') == bodies['inputs/runtime-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/runtime-sources.json'], 'immutable base posthash')
    receipt = dict(schema='ferric-guarded-mlp-reusable-ar4-worker-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=997, runtime_files=811, worker_files=184,
        copied_unchanged_files=983, overlay_files=12, helpers=2, project_execution=False,
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
