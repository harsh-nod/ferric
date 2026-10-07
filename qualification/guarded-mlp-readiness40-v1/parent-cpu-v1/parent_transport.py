"""Data-only current-parent clone and reviewed readiness source overlay."""
import ast
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import signal
import stat
import sys
import tarfile

P = Path(__file__).resolve().parent
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/guarded-mlp-peer-read-pair-v1/parent-cpu-v2'
WQ = F / 'qualification/guarded-mlp-peer-read-pair-v1/worker-cpu-v3'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-paired-read-parent-cpu-v228-v2'
WORKER_BASE = E / 'guarded-mlp-peer-read-pair-worker-cpu-v228-v3'
ROOT = E / 'guarded-mlp-readiness-cli-parent-cpu-v228-v1'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-readiness-cli-parent-input-v228-v1.tar.gz'
CONTROLLER_PIN = dict(bytes=34672, sha256='08a826dfe12e51c671069a12df08f4139df0c2b441a759dbb793d4775251fb4b')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13')
COMPLETE = dict(bytes=3792547, sha256='143132b2d4e147aec77865cd7a60a076e1f663088ee9fe4d69ede4cfbf0ed76c')
SOURCES = dict(bytes=476997, sha256='9b11f4bb7185f7c435af7347ed6b4f7f6f82b359b034dbd4b3b6d4ef2e7dabb2')
WORKER_COMPLETE = dict(bytes=1641412, sha256='f69a1d54adba68602a2bc33a1802a71310cd4a63a8f64cdfd234558e47674d98')
WORKER_SOURCES = dict(bytes=408952, sha256='91af0a905be9d2d320187e8e3d876ec4b8f6d3570b3092cf59cedc639969a205')
PROPOSAL = dict(bytes=43037, sha256='cc6f6c46886c28f102e4f2b05d822be120d19673d2703e71596ca9b54ddbcbd4')
CACHE_PIN = dict(bytes=96226, sha256='984b2a893ba64f35cda1cdceb5d379028416c00c36815c678200480c7e8904ad')
WORKER = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
PARENT = 'ferric/adapters/m1-engineering-execution-v1/'
HELPERS = {'run_cpu.py', 'supervisor.py', 'qualification_support.py'}


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
            and pin(bodies['supervisor.py']) == SUPERVISOR_PIN
            and pin(bodies['qualification_support.py']) == SUPPORT_PIN, 'three exact reviewed harness inputs')
    fixed = {'parent-complete.json': COMPLETE, 'parent-sources.json': SOURCES,
             'worker-complete.json': WORKER_COMPLETE, 'worker-sources.json': WORKER_SOURCES,
             'composed-source-manifest.json': PROPOSAL}
    require(all(pin(bodies['inputs/' + n]) == v for n, v in fixed.items()), 'literal actual baseline/source identities')
    base = parse(bodies['inputs/parent-complete.json'])
    source_map = parse(bodies['inputs/parent-sources.json'])
    worker = parse(bodies['inputs/worker-complete.json'])
    worker_map = parse(bodies['inputs/worker-sources.json'])
    proposal = parse(bodies['inputs/composed-source-manifest.json'])
    require(base['schema'] == 'ferric-guarded-mlp-paired-read-parent-cpu-v2' and base['passed'] is True
            and base['failure'] is None and base['postcheck_errors'] == [] and base['source_unchanged'] is True
            and base['input_sources'] == base['final_sources'] == source_map and len(source_map) == 1225
            and compact(base['raw']['sources-after.json']) == SOURCES and len(base['phases']) == 55
            and len(base['tests']) == 47 and sum(v['passed'] for v in base['tests'].values()) == 400
            and base['gpu_execution'] is False, 'actual selected parent CPU')
    require(worker['schema'] == 'ferric-guarded-mlp-peer-read-pair-worker-cpu-v3' and worker['passed'] is True
            and worker['failure'] is None and worker['postcheck_errors'] == [] and worker['source_unchanged'] is True
            and worker['input_sources'] == worker['final_sources'] == worker_map and len(worker_map) == 999
            and compact(worker['raw']['sources-after.json']) == WORKER_SOURCES and worker['gpu_execution'] is False,
            'actual V3 worker source baseline')
    require(proposal['schema'] == 'ferric-guarded-mlp-readiness-cli-composed-source-v1'
            and proposal['composition_order'] == ['pure-long', 'native-owner', 'readiness-cli']
            and len(proposal['layers']) == 3 and proposal['parent_source_preimage_discrepancies'] == [],
            'closed reviewed readiness composition')
    layer_pins = {r['name'] + '-source-manifest.json': compact(r['manifest']) for r in proposal['layers']}
    require(all(pin(bodies['inputs/' + n]) == row for n, row in layer_pins.items()), 'original three source manifests')
    names = set(fixed) | set(layer_pins) | {'parent-lib-list.stdout', 'metadata.stdout'}
    names |= {label + suffix for label in base['tests'] for suffix in ('.stdout', '.command.json')}
    require(len(names) == 104, 'current104 lineage census')
    require(all(pin(bodies['inputs/' + n]) == compact(base['raw'][n]) for n in names - set(fixed) - set(layer_pins)),
            'actual current recipes and raw named outcomes')
    old = {n: compact(v) for n, v in source_map.items() if n.startswith('ferric/')}
    worker_files = {n: compact(v) for n, v in worker_map.items() if n.startswith(WORKER)}
    require(len(old) == 1222 and len(worker_files) == 184 and set(worker_files) <= set(old)
            and all(ordinary(n) and source_map[n]['path'] == str(BASE / n) for n in old)
            and all(worker_map[n]['path'] == str(WORKER_BASE / n) for n in worker_files), 'actual canonical source roots')
    transitions = {'ferric/' + row['path']: row for row in proposal['parent_worker_transitions']}
    require(len(transitions) == len(proposal['parent_worker_transitions']) == 10
            and set(transitions) == {n for n in worker_files if old[n] != worker_files[n]}
            and all(row['before'] == old[n] and row['after'] == worker_files[n] for n, row in transitions.items()),
            'explicit ten inherited worker transitions')
    files = dict(old)
    files.update(worker_files)
    overlay = []
    for row in proposal['files']:
        n = 'ferric/' + row['path']
        require(row['repository'] == 'ferric' and ordinary(n) and n not in overlay
                and files.get(n) == row['before'] and pin(bodies[n]) == row['after'], 'exact composed preimage/postimage')
        files[n] = row['after']
        overlay.append(n)
    require(len(overlay) == 23 and sum(n.startswith(WORKER) for n in overlay) == 16
            and sum(n.startswith(PARENT) for n in overlay) == 7
            and sum(r['before'] is None for r in proposal['files']) == 15, 'closed23-row source boundary')
    for n in HELPERS:
        files[n] = pin(bodies[n])
    require(set(bodies) == set(overlay) | HELPERS | {'inputs/' + n for n in names}, 'closed130 source/lineage bodies')
    require(len(files) == 1240 and sum(v['bytes'] for v in files.values()) <= 64 << 20, 'bounded1240 source rows')
    inputs = dict(schema='ferric-guarded-mlp-readiness-cli-parent-cpu-input-v1',
        files=dict(sorted(files.items())), lineage={n: pin(bodies['inputs/' + n]) for n in sorted(names)},
        parent_overlay=sorted(n for n in overlay if n.startswith(PARENT) and n.endswith('.rs')),
        cache_manifest=CACHE_PIN, git_revision=base['git_revision'])
    require(len(inputs['parent_overlay']) == 6, 'six parent Rust formatting inputs')
    return inputs, old, worker_files


def pack():
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'parent-input-manifest.json'), 'fresh package outputs')
    paths = {'run_cpu.py': P / 'parent_cpu.py', 'supervisor.py': Q / 'supervisor.py',
             'qualification_support.py': Q / 'qualification_support.py',
             'inputs/composed-source-manifest.json': P.parent / 'composed-v1/source-manifest.json'}
    for prefix, directory in [('parent', Q), ('worker', WQ)]:
        paths['inputs/' + prefix + '-complete.json'] = directory / 'evidence/complete.json'
        paths['inputs/' + prefix + '-sources.json'] = directory / 'evidence/sources-after.json'
    base = parse(read(paths['inputs/parent-complete.json']))
    proposal = parse(read(paths['inputs/composed-source-manifest.json']))
    for row in proposal['layers']:
        paths['inputs/' + row['name'] + '-source-manifest.json'] = Path(row['manifest']['path'])
    for name in {'parent-lib-list.stdout', 'metadata.stdout'} | {label + suffix for label in base['tests']
            for suffix in ('.stdout', '.command.json')}:
        paths['inputs/' + name] = Q / 'evidence' / name
    for row in proposal['files']:
        paths['ferric/' + row['path']] = P.parent / 'composed-v1/ferric' / row['path']
    bodies = {n: read(p) for n, p in paths.items()}
    inputs, old, worker_files = contract(bodies)
    for n, row in worker_files.items():
        require(pin(read(F / n.removeprefix('ferric/'))) == row, 'all184 current canonical worker preimages')
    require(all(read(p) == bodies[n] for n, p in paths.items()), 'all packaging inputs unchanged')
    raw_input = encoded(inputs)
    bodies['input-manifest.json'] = raw_input
    require(len(bodies) == 131 and sum(map(len, bodies.values())) <= 32 << 20, 'bounded131-member source package')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as t:
            for n, raw in sorted(bodies.items()):
                item = tarfile.TarInfo(n); item.size = len(raw); item.mode = 0o600
                t.addfile(item, io.BytesIO(raw))
    with (P / 'parent-input-manifest.json').open('xb') as stream:
        stream.write(raw_input)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))),
                         input=pin(raw_input), members=131, source_files=1240), sort_keys=True))


def stage(archive_sha, input_sha):
    require(re.fullmatch('[0-9a-f]{64}', archive_sha) and re.fullmatch('[0-9a-f]{64}', input_sha), 'observed two SHA arguments')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'exact staging host/UID')
    require(E.resolve(strict=True) == E and not os.path.lexists(ROOT), 'fresh exact parent root')
    require(shutil.disk_usage(E).free >= 40 << 30, '40GiB initial free floor')
    archive_raw = read(E / BASENAME)
    require(pin(archive_raw)['sha256'] == archive_sha and len(archive_raw) <= 32 << 20, 'actual bounded source archive')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as t:
        members = t.getmembers()
        require(len(members) == len({m.name for m in members}) == 131
                and all(m.isfile() and ordinary(m.name) and not m.pax_headers and 0 <= m.size <= 8 << 20 for m in members)
                and sum(m.size for m in members) <= 32 << 20, 'closed regular USTAR package')
        bodies = {m.name: t.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'complete member reads')
    raw_input = bodies.pop('input-manifest.json')
    require(pin(raw_input)['sha256'] == input_sha, 'actual source input')
    expected, old, worker_files = contract(bodies)
    require(parse(raw_input) == expected, 'entire reconstructed input contract')
    for prefix, base in [('parent', BASE), ('worker', WORKER_BASE)]:
        require(read(base / 'evidence/complete.json') == bodies['inputs/' + prefix + '-complete.json']
                and read(base / 'evidence/sources-after.json') == bodies['inputs/' + prefix + '-sources.json'],
                'immutable actual baseline receipts')
    actual_paths = set()
    for directory, dirs, names in os.walk(BASE / 'ferric', followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(not any((Path(directory) / n).is_symlink() for n in dirs), 'no baseline directory aliases')
        actual_paths.update(str((Path(directory) / n).relative_to(BASE)) for n in names)
    require(actual_paths == set(old), 'exact1222-body original parent source tree')
    require(all(pin(read(BASE / n)) == row for n, row in old.items())
            and all(pin(read(WORKER_BASE / n)) == row for n, row in worker_files.items()), 'all baseline body prehashes')
    os.umask(0o077); ROOT.mkdir(mode=0o700)
    def write(n, raw):
        require(ordinary(n), 'ordinary destination')
        path = ROOT / n; path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
    for n in old:
        if n not in bodies:
            write(n, read((WORKER_BASE if n in worker_files else BASE) / n))
    for n, raw in bodies.items():
        write(n, raw)
    write('input-manifest.json', raw_input)
    require(all(pin(read(ROOT / n)) == row for n, row in expected['files'].items()), 'all1240 staged source rows')
    require(all(pin(read(BASE / n)) == row for n, row in old.items())
            and all(pin(read(WORKER_BASE / n)) == row for n, row in worker_files.items()), 'immutable baseline body posthashes')
    require(read(E / BASENAME) == archive_raw, 'archive unchanged during staging')
    for prefix, base in [('parent', BASE), ('worker', WORKER_BASE)]:
        require(read(base / 'evidence/complete.json') == bodies['inputs/' + prefix + '-complete.json']
                and read(base / 'evidence/sources-after.json') == bodies['inputs/' + prefix + '-sources.json'],
                'immutable baseline receipt posthashes')
    receipt = dict(schema='ferric-guarded-mlp-readiness-cli-parent-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=1240, ferric_files=1237,
        overlay_files=23, inherited_worker_transitions=10, helpers=3, lineage_files=104,
        project_execution=False, canonical_changed=False, shared_cache_changed=False, lockfiles_changed=False,
        root=str(ROOT), controller=pin(read(Path(__file__).resolve())))
    write('stage.json', encoded(receipt))
    print(json.dumps(receipt, sort_keys=True))


def main():
    require(__debug__ and sys.dont_write_bytecode, 'nonoptimized python3 -B')
    def timeout(_number, _frame):
        raise RuntimeError('bounded parent source transport deadline')
    signal.signal(signal.SIGALRM, timeout)
    signal.alarm(180)
    if sys.argv[1:] == ['pack']:
        pack()
    elif len(sys.argv) == 4 and sys.argv[1] == 'stage':
        stage(sys.argv[2], sys.argv[3])
    else:
        raise ValueError('parent_transport.py pack | stage ARCHIVE_SHA INPUT_SHA')
    signal.alarm(0)


if __name__ == '__main__':
    main()
