"""Data-only composed readiness transport over the actual corrected V3 worker."""
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
Q = F / 'qualification/guarded-mlp-peer-read-pair-v1/worker-cpu-v3'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-peer-read-pair-worker-cpu-v228-v3'
ROOT = E / 'guarded-mlp-readiness-cli-worker-cpu-v228-v1'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-readiness-cli-worker-input-v228-v1.tar.gz'
CONTROLLER_PIN = dict(bytes=37922, sha256='640f8c694d3b4cbd1795e060916d548dfbc82b1fc944713637bd36dfad5ac6ce')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
COMPLETE = dict(bytes=1641412, sha256='f69a1d54adba68602a2bc33a1802a71310cd4a63a8f64cdfd234558e47674d98')
SOURCES = dict(bytes=408952, sha256='91af0a905be9d2d320187e8e3d876ec4b8f6d3570b3092cf59cedc639969a205')
PROPOSAL = dict(bytes=43037, sha256='cc6f6c46886c28f102e4f2b05d822be120d19673d2703e71596ca9b54ddbcbd4')
COMPONENTS = {
    'pure_long': ('pure-long-source-manifest.json', dict(bytes=8538, sha256='7e27f89d1258df78e955f820f72cee556f1905f4aa709030eac8cb1881e3d7a0')),
    'native_owner': ('native-owner-source-manifest.json', dict(bytes=4749, sha256='3cb914f184a61a3ac9b830bdf2bf0ab23c205c79d5764e904fe9eb188027ab98')),
    'readiness_cli': ('readiness-cli-source-manifest.json', dict(bytes=8699, sha256='d669a05f166ef2c1beebe34b7c5a620c2d075da33e4dc7b248d1f4b1757d1816')),
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
    require(PROPOSAL is not None and pin(bodies['inputs/worker-complete.json']) == COMPLETE
            and pin(bodies['inputs/worker-sources.json']) == SOURCES
            and pin(bodies['inputs/composed-source-manifest.json']) == PROPOSAL, 'actual base and reviewed composed source pins')
    base = parse(bodies['inputs/worker-complete.json'])
    source_map = parse(bodies['inputs/worker-sources.json'])
    proposal = parse(bodies['inputs/composed-source-manifest.json'])
    require(base['schema'] == 'ferric-guarded-mlp-peer-read-pair-worker-cpu-v3'
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['source_unchanged'] is True and base['input_sources'] == base['final_sources'] == source_map
            and len(source_map) == 999 and compact(base['raw']['sources-after.json']) == SOURCES
            and base['gpu_execution'] is False and base['runtime_source_qualified_by_paired_read_cpu'] is True
            and base['paired_read_cli_dispatch_fixed'] is True and base['cli_executable_unchanged_across_tests'] is True
            and base['cli_executable_before_tests']['pin'] == base['artifacts']['worker']['pin'],
            'actual qualified V3 runtime/worker and executable CLI regression')
    for name in ('worker-tests.stdout', 'worker-list.stdout'):
        require(pin(bodies['inputs/' + name]) == compact(base['raw'][name]), 'actual worker raw baseline')
    require(proposal['schema'] == 'ferric-guarded-mlp-readiness-cli-composed-source-v1'
            and proposal['source_only'] is True and len(proposal['files']) == 23
            and compact(proposal['base']['worker_complete']) == COMPLETE
            and compact(proposal['base']['worker_sources']) == SOURCES
            and proposal['base']['worker_elf'] == base['artifacts']['worker']['pin'],
            'reviewed composed readiness source ancestry')
    old = {name: compact(row) for name, row in source_map.items() if name.startswith(('fe2o3/', WORKER))}
    require(len(old) == 997 and sum(name.startswith('fe2o3/') for name in old) == 813
            and all(ordinary(name) and source_map[name]['path'] == str(BASE / name) for name in old), 'qualified813+184 source closure')
    chain, components = dict(old), {}
    require([layer['name'] for layer in proposal['layers']] == ['pure-long', 'native-owner', 'readiness-cli'],
            'closed composition order')
    for (role, (filename, wanted)), layer in zip(COMPONENTS.items(), proposal['layers']):
        require(pin(bodies['inputs/' + filename]) == compact(layer['manifest']) == wanted,
                'exact retained component manifest')
        component = parse(bodies['inputs/' + filename])
        components[role] = component
        for row in component['files']:
            name = 'ferric/' + row['path']
            if name.startswith(WORKER):
                require(chain.get(name) == row['before'], 'exact intermediate worker preimage or absence')
                chain[name] = row['after']
    rows = [row for row in proposal['files'] if row['repository'] == 'ferric'
            and ('ferric/' + row['path']).startswith(WORKER)]
    require(len(rows) == 16 and sum(row['before'] is None for row in rows) == 11, '16 worker rows, five replacements and eleven additions')
    files, overlay = dict(old), []
    for row in rows:
        name = 'ferric/' + row['path']
        require(ordinary(name) and name not in overlay and old.get(name) == row['before']
                and pin(bodies[name]) == row['after'], 'exact worker preimage/absence and postimage')
        files[name] = row['after']
        overlay.append(name)
    require(WORKER + 'tests/shared_wire.rs' not in overlay, 'existing real executable regression source preserved')
    require(files == chain, 'flattened source equals all three frozen components')
    for name in ('run_cpu.py', 'supervisor.py'):
        files[name] = pin(bodies[name])
    roles = {'worker_complete': 'worker-complete.json', 'worker_sources': 'worker-sources.json',
             'worker_tests': 'worker-tests.stdout', 'worker_list': 'worker-list.stdout',
             'composed_proposal': 'composed-source-manifest.json'}
    roles.update({role: row[0] for role, row in COMPONENTS.items()})
    require(set(bodies) == set(overlay) | {'run_cpu.py', 'supervisor.py'} | {'inputs/' + name for name in roles.values()},
            'closed26 transport source/lineage bodies; parent sources excluded')
    require(len(files) == 1010 and sum(row['bytes'] for row in files.values()) <= 64 << 20, 'full composed worker source bounds')
    library = proposal['new_tests']['worker_library']
    executable = proposal['new_tests']['worker_readiness_cli_integration']
    require(type(library) is list and type(executable) is list and len(library) == 34 and len(executable) == 1,
            'closed target-specific additions')
    names = library + executable
    require(sorted(library) == sorted(components['pure_long']['new_tests']['worker_lib']
            + components['native_owner']['new_tests']['worker_lib']
            + components['readiness_cli']['new_tests']['worker_library'])
            and executable == components['readiness_cli']['new_tests']['worker_readiness_cli_integration'],
            'all exact component test additions preserved')
    require(len(names) == len(set(names)) == 35
            and all(type(name) is str and re.fullmatch('[A-Za-z0-9_:]+', name) for name in names),
            '35 unique source-declared test names')
    inputs = dict(schema='ferric-guarded-mlp-readiness-cli-worker-cpu-input-v1', source_generation=GENERATION,
        files=dict(sorted(files.items())), tool_pins=base['tool_pins'],
        source_lineage={role: dict(path=str(ROOT / 'inputs' / name), **pin(bodies['inputs/' + name])) for role, name in roles.items()},
        worker_overlay=sorted(overlay), new_tests={'worker-lib': library, 'worker-bin-test': [],
                                                'worker-wire-test': [], 'worker-readiness-test': executable})
    return inputs, old

def pack():
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'worker-input-manifest.json'), 'fresh package outputs')
    paths = {'run_cpu.py': P / 'worker_cpu.py', 'supervisor.py': Q / 'supervisor.py',
             'inputs/composed-source-manifest.json': P.parent / 'composed-v1/source-manifest.json',
             'inputs/pure-long-source-manifest.json': P.parent.parent / 'source-manifest.json',
             'inputs/native-owner-source-manifest.json': P.parent.parent / 'native-readiness-v1/source-manifest.json',
             'inputs/readiness-cli-source-manifest.json': P.parent / 'source-manifest.json'}
    for name, original in (('worker-complete.json', 'complete.json'), ('worker-sources.json', 'sources-after.json'),
                           ('worker-tests.stdout', 'worker-tests.stdout'), ('worker-list.stdout', 'worker-list.stdout')):
        paths['inputs/' + name] = Q / 'evidence' / original
    proposal = parse(read(paths['inputs/composed-source-manifest.json']))
    for row in proposal['files']:
        if row['repository'] != 'ferric' or not ('ferric/' + row['path']).startswith(WORKER):
            continue
        name = 'ferric/' + row['path']
        require(name not in paths, 'disjoint source rows')
        paths[name] = P.parent / 'composed-v1/ferric' / row['path']
    bodies = {name: read(path) for name, path in paths.items()}
    inputs, old = contract(bodies)
    for name, row in old.items():
        if name.startswith(WORKER):
            require(pin(read(F / name.removeprefix('ferric/'))) == row, 'all184 canonical preimages match actual V3 worker')
    require(all(read(path) == bodies[name] for name, path in paths.items()), 'package source posthash')
    input_raw = encoded(inputs)
    bodies['input-manifest.json'] = input_raw
    require(len(bodies) == 27 and sum(map(len, bodies.values())) <= 16 << 20, 'bounded27-member input')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(raw), 0o600, 0
                tar.addfile(info, io.BytesIO(raw))
    with (P / 'worker-input-manifest.json').open('xb') as stream:
        stream.write(input_raw)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))), input=pin(input_raw),
                         members=27, source_files=1010), sort_keys=True))


def stage(archive_sha, input_sha):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(E.resolve(strict=True) == E and BASE.resolve(strict=True) == BASE and not os.path.lexists(ROOT), 'fresh exact worker root')
    require(os.statvfs(E).f_bavail * os.statvfs(E).f_frsize >= 40 << 30, 'initial40GiB floor')
    archive_raw = read(E / BASENAME)
    require(pin(archive_raw)['sha256'] == archive_sha, 'actual source archive')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == len({m.name for m in members}) == 27
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
    require({str(path.relative_to(BASE)) for path in paths} == set(old), 'full997 predecessor source roster')
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
    receipt = dict(schema='ferric-guarded-mlp-readiness-cli-worker-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=1010, runtime_files=813, worker_files=195,
        copied_unchanged_files=992, overlay_files=16, helpers=2, project_execution=False,
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
