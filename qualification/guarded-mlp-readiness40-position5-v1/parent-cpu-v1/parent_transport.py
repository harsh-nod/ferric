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
Q = F / 'qualification/guarded-mlp-readiness40-v1/parent-cpu-v1'
WQ = F / 'qualification/guarded-mlp-readiness40-v1/worker-cpu-v1'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-readiness-cli-parent-cpu-v228-v1'
WORKER_BASE = E / 'guarded-mlp-readiness-cli-worker-cpu-v228-v1'
ROOT = E / 'guarded-mlp-readiness40-position5-parent-cpu-v228-v1'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-readiness40-position5-parent-input-v228-v1.tar.gz'
CONTROLLER_PIN = dict(bytes=33355, sha256='977e14cd85432be4e690e992c0b105b648afa0e074ddda50c170e53f2bba405f')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13')
COMPLETE = dict(bytes=3854671, sha256='12728ea998174d0645f7b70ddfb46af8454ff50f2856ad2df3fd6c31ac22fc66')
SOURCES = dict(bytes=485952, sha256='c3786b554db70c059fcf77ba2afd4547fd451f820823cca9cd32f4afbea6ad87')
WORKER_COMPLETE = dict(bytes=1675803, sha256='2b785c1b8384358f11d547be93614660171c94108aba5e50cdb8db7e4b462b77')
WORKER_SOURCES = dict(bytes=412739, sha256='8fd9a73ff5954e82a45505e9e3cdd0b072565f2560403f58e6474093fa14ef3c')
PROPOSAL = dict(bytes=7948, sha256='7c0c940e37f519995126a0d8850dcdafcb323a1d1c8dc5573d37bd79cbbf2772')
TRANSITIONS_PIN = dict(bytes=6494, sha256='6ba7ed245b85945d922a57e9327151a98a2f67dd8b74943d5641dd9016d40de0')
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
             'source-manifest.json': PROPOSAL, 'parent-worker-transitions.json': TRANSITIONS_PIN}
    require(all(pin(bodies['inputs/' + n]) == v for n, v in fixed.items()), 'literal actual baseline/source identities')
    base = parse(bodies['inputs/parent-complete.json'])
    source_map = parse(bodies['inputs/parent-sources.json'])
    worker = parse(bodies['inputs/worker-complete.json'])
    worker_map = parse(bodies['inputs/worker-sources.json'])
    proposal = parse(bodies['inputs/source-manifest.json'])
    require(base['schema'] == 'ferric-guarded-mlp-readiness-cli-parent-cpu-v1' and base['passed'] is True
            and base['failure'] is None and base['postcheck_errors'] == [] and base['source_unchanged'] is True
            and base['input_sources'] == base['final_sources'] == source_map and len(source_map) == 1240
            and compact(base['raw']['sources-after.json']) == SOURCES and len(base['phases']) == 60
            and len(base['tests']) == 51 and sum(v['passed'] for v in base['tests'].values()) == 428
            and base['gpu_execution'] is False, 'actual selected parent CPU')
    require(worker['schema'] == 'ferric-guarded-mlp-readiness-cli-worker-cpu-v1' and worker['passed'] is True
            and worker['failure'] is None and worker['postcheck_errors'] == [] and worker['source_unchanged'] is True
            and worker['input_sources'] == worker['final_sources'] == worker_map and len(worker_map) == 1010
            and compact(worker['raw']['sources-after.json']) == WORKER_SOURCES and worker['gpu_execution'] is False,
            'actual V3 worker source baseline')
    require(proposal['schema'] == 'ferric-readiness40-position5-source-v1'
            and proposal['base_commit'] == 'b68a737335508493fbc469c48726ff21af27c002'
            and proposal['diagnostic_selected'] == [0, 5, 16, 39]
            and proposal['old_selected'] == [0, 15, 16, 39]
            and proposal['forwards'] == 40 and proposal['generated_tokens'] == 0
            and all(proposal[k] is False for k in ('runtime_changed', 'kernels_changed',
                'scope_limits_changed', 'default_profile_changed')), 'closed reviewed position5 replacements')
    transition_doc = parse(bodies['inputs/parent-worker-transitions.json'])
    require(transition_doc['parent_source'] == SOURCES and transition_doc['worker_source'] == WORKER_SOURCES
            and transition_doc['worker_files'] == 195, 'actual sixteen transition provenance')
    names = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    names |= {label + suffix for label in base['tests'] for suffix in ('.stdout', '.command.json')}
    require(len(names) == 110, 'current110 lineage census')
    require(all(pin(bodies['inputs/' + n]) == compact(base['raw'][n]) for n in names - set(fixed)),
            'actual current recipes and raw named outcomes')
    old = {n: compact(v) for n, v in source_map.items() if n.startswith('ferric/')}
    worker_files = {n: compact(v) for n, v in worker_map.items() if n.startswith(WORKER)}
    require(len(old) == 1237 and len(worker_files) == 195 and set(worker_files) <= set(old)
            and all(ordinary(n) and source_map[n]['path'] == str(BASE / n) for n in old)
            and all(worker_map[n]['path'] == str(WORKER_BASE / n) for n in worker_files), 'actual canonical source roots')
    transitions = {'ferric/' + row['path']: row for row in transition_doc['rows']}
    require(len(transitions) == len(transition_doc['rows']) == 16
            and set(transitions) == {n for n in worker_files if old[n] != worker_files[n]}
            and all(row['before'] == old[n] and row['after'] == worker_files[n] for n, row in transitions.items()),
            'explicit sixteen inherited worker transitions')
    files = dict(old)
    files.update(worker_files)
    overlay = []
    for row in proposal['files']:
        n = 'ferric/' + row['path']
        require(row['repository'] == 'ferric' and ordinary(n) and n not in overlay
                and files.get(n) == row['before'] and pin(bodies[n]) == row['after'], 'exact composed preimage/postimage')
        files[n] = row['after']
        overlay.append(n)
    require(len(overlay) == 14 and sum(n.startswith(WORKER) for n in overlay) == 10
            and sum(n.startswith(PARENT) for n in overlay) == 4
            and sum(r['before'] is None for r in proposal['files']) == 0, 'closed14-row source boundary')
    for n in HELPERS:
        files[n] = pin(bodies[n])
    require(set(bodies) == set(overlay) | HELPERS | {'inputs/' + n for n in names}, 'closed127 source/lineage bodies')
    require(len(files) == 1240 and sum(v['bytes'] for v in files.values()) <= 64 << 20, 'bounded1240 source rows')
    inputs = dict(schema='ferric-guarded-mlp-readiness40-position5-parent-cpu-input-v1',
        files=dict(sorted(files.items())), lineage={n: pin(bodies['inputs/' + n]) for n in sorted(names)},
        parent_overlay=sorted(n for n in overlay if n.startswith(PARENT) and n.endswith('.rs')),
        cache_manifest=CACHE_PIN, git_revision=base['git_revision'])
    require(len(inputs['parent_overlay']) == 4, 'four parent Rust formatting inputs')
    return inputs, old, worker_files


def pack():
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'parent-input-manifest.json'), 'fresh package outputs')
    paths = {'run_cpu.py': P / 'parent_cpu.py', 'supervisor.py': Q / 'supervisor.py',
             'qualification_support.py': Q / 'qualification_support.py',
             'inputs/source-manifest.json': P.parent / 'source-manifest.json',
             'inputs/parent-worker-transitions.json': P / 'parent-worker-transitions.json'}
    for prefix, directory in [('parent', Q), ('worker', WQ)]:
        paths['inputs/' + prefix + '-complete.json'] = directory / 'evidence/complete.json'
        paths['inputs/' + prefix + '-sources.json'] = directory / 'evidence/sources-after.json'
    base = parse(read(paths['inputs/parent-complete.json']))
    proposal = parse(read(paths['inputs/source-manifest.json']))
    for name in {'parent-lib-list.stdout', 'metadata.stdout'} | {label + suffix for label in base['tests']
            for suffix in ('.stdout', '.command.json')}:
        paths['inputs/' + name] = Q / 'evidence' / name
    for row in proposal['files']:
        paths['ferric/' + row['path']] = P.parent / 'ferric' / row['path']
    bodies = {n: read(p) for n, p in paths.items()}
    inputs, old, worker_files = contract(bodies)
    for n, row in worker_files.items():
        require(pin(read(F / n.removeprefix('ferric/'))) == row, 'all195 current canonical worker preimages')
    require(all(read(p) == bodies[n] for n, p in paths.items()), 'all packaging inputs unchanged')
    raw_input = encoded(inputs)
    bodies['input-manifest.json'] = raw_input
    require(len(bodies) == 128 and sum(map(len, bodies.values())) <= 32 << 20, 'bounded128-member source package')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as t:
            for n, raw in sorted(bodies.items()):
                item = tarfile.TarInfo(n); item.size = len(raw); item.mode = 0o600
                t.addfile(item, io.BytesIO(raw))
    with (P / 'parent-input-manifest.json').open('xb') as stream:
        stream.write(raw_input)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))),
                         input=pin(raw_input), members=128, source_files=1240), sort_keys=True))


def stage(archive_sha, input_sha):
    require(re.fullmatch('[0-9a-f]{64}', archive_sha) and re.fullmatch('[0-9a-f]{64}', input_sha), 'observed two SHA arguments')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'exact staging host/UID')
    require(E.resolve(strict=True) == E and not os.path.lexists(ROOT), 'fresh exact parent root')
    require(shutil.disk_usage(E).free >= 40 << 30, '40GiB initial free floor')
    archive_raw = read(E / BASENAME)
    require(pin(archive_raw)['sha256'] == archive_sha and len(archive_raw) <= 32 << 20, 'actual bounded source archive')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as t:
        members = t.getmembers()
        require(len(members) == len({m.name for m in members}) == 128
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
    require(actual_paths == set(old), 'exact1237-body original parent source tree')
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
    receipt = dict(schema='ferric-guarded-mlp-readiness40-position5-parent-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=1240, ferric_files=1237,
        overlay_files=14, inherited_worker_transitions=16, helpers=3, lineage_files=110,
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
