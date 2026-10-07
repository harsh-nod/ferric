"""Data-only actual Full-parent clone with four parent-only timing overlays."""
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
PROPOSAL_ROOT = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/proposals/guarded-mlp-model-interface-v228-v1/model-guarded-full2303-v1/host-timing-proposal-v1')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/guarded-mlp-full2303-v1/parent-cpu-v1'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-full2303-parent-cpu-v228-v1'
ROOT = E / 'guarded-mlp-readiness40-host-timing-parent-cpu-v228-v1'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-readiness40-host-timing-parent-input-v228-v1.tar.gz'
CONTROLLER_PIN = dict(bytes=33129, sha256='94686ea4fc4d1653af2308a89cde72d116836a5ced016dc3abb56a1e7dd1e868')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13')
BASE_COMPLETE = dict(bytes=3870404, sha256='67949bbd5a662efc4d56a65df8b9988dbaf56cbd10b0e006981bed7f4fffbf13')
BASE_SOURCES = dict(bytes=485790, sha256='2d6613ee1d0267416adcc496d7806e9fdad9317b641a96e900d4b04a5a697df4')
PROPOSAL_PIN = dict(bytes=7307, sha256='669cdf0e4f2f1de9e3ea42eb2ea2de1610f1aef8a39f219ebd57eb4497b24b99')
CACHE_PIN = dict(bytes=96226, sha256='984b2a893ba64f35cda1cdceb5d379028416c00c36815c678200480c7e8904ad')
WORKER_PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
PARENT_REL = 'adapters/m1-engineering-execution-v1'
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


def contract(packed):
    require(CONTROLLER_PIN is not None, 'reviewed controller pin must be bound')
    require(pin(packed['run_cpu.py']) == CONTROLLER_PIN
            and pin(packed['supervisor.py']) == SUPERVISOR_PIN
            and pin(packed['qualification_support.py']) == SUPPORT_PIN, 'three exact reviewed harness inputs')
    fixed = {'parent-complete.json': BASE_COMPLETE, 'parent-sources.json': BASE_SOURCES,
             'host-timing-source-manifest.json': PROPOSAL_PIN}
    bodies = {n.removeprefix('inputs/'): raw for n, raw in packed.items() if n.startswith('inputs/')}
    require(all(pin(bodies[n]) == v for n, v in fixed.items()), 'literal actual parent/source identities')
    base = parse(bodies['parent-complete.json'])
    old_map = parse(bodies['parent-sources.json'])
    proposal = parse(bodies['host-timing-source-manifest.json'])
    require(base['schema'] == 'ferric-guarded-mlp-full2303-parent-cpu-v1'
            and len(old_map) == 1254 and len(base['phases']) == 63 and len(base['tests']) == 53
            and len(base['inventory']) == 928 and sum(v['passed'] for v in base['tests'].values()) == 455
            and all(v['failed'] == v['ignored'] == 0 for v in base['tests'].values())
            and base['full2303_parent_route_added'] is True
            and base['full2303_native_execution'] is False
            and base['full2303_launch_feasibility'] is False
            and base['full2303_source_abort_ms'] == 3600000
            and len(base['artifacts']) == 7, 'actual selected Full parent baseline')
    require(base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['input_sources'] == base['final_sources'] == old_map
            and base['source_unchanged'] is True and base['gpu_execution'] is False
            and compact(base['raw']['sources-after.json']) == BASE_SOURCES
            and all(p['exit_code'] == 0 and p['natural_exit'] is True and p['reaped'] is True
                and p['process_group_absent'] is True and p['forced_cleanup'] is False
                and p['timed_out'] is False and p['exception'] is None and p['storage_failure'] is None
                for p in base['phases']), 'actual clean source/CPU lifecycle')
    require(proposal['schema'] == 'ferric-readiness40-position5-parent-host-timing-source-v1'
            and proposal['base']['full_parent_paths_disjoint'] is True
            and proposal['scope'] == dict(compiled=False, config_struct_changed=False,
                deadline_changed=False, event_count=124, explicit_parent_host_timing_source_added=True,
                forward_rows=40, full2303_feasibility=False, kernel_images_changed=False,
                native_execution=False, numerical_acceptance=False, old_entry_signatures_changed=False,
                ordinary_observation_schema_changed=False, performance_claim=False,
                retention_cap_changed=False, runtime_changed=False, sidecar_max_bytes=65536,
                tests_executed=False, wire_changed=False, worker_changed=False),
            'reviewed parent-only timing source with unchanged defaults and no authority')
    old = {n: compact(v) for n, v in old_map.items() if n.startswith('ferric/')}
    require(len(old) == 1251 and sum(n.startswith(WORKER_PREFIX) for n in old) == 203
            and all(ordinary(n) and old_map[n]['path'] == str(BASE / n) for n in old),
            'one actual parent source closure with203 inherited worker bodies')
    expected = dict(old)
    parent_rows = {'ferric/' + r['path']: r for r in proposal['files']}
    require(len(parent_rows) == len(proposal['files']) == 4
            and all(r['repository'] == 'ferric' and n.startswith('ferric/' + PARENT_REL + '/')
                    and n.endswith('.rs') for n, r in parent_rows.items())
            and sum(r['before'] is None for r in parent_rows.values()) == 2,
            'four parent-only Rust overlays and two additions')
    for n, row in parent_rows.items():
        require(expected.get(n) == row['before'], 'actual parent proposal preimage ' + n)
        expected[n] = row['after']
    require(len(expected) == 1253
            and all(expected[n] == row for n, row in old.items() if n.startswith(WORKER_PREFIX)),
            'complete parent Ferric closure without worker substitutions')
    for row in proposal['unchanged_contracts']:
        require(old['ferric/' + row['path']] == expected['ferric/' + row['path']] == compact(row),
                'unchanged evidence/wire/native source boundary')
    names = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    names |= {label + suffix for label in base['tests'] for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == names and len(names) == 111, 'closed111 direct lineage bodies')
    require(all(pin(bodies[n]) == compact(base['raw'][n]) for n in names - set(fixed)),
            'actual current recipes and raw named outcomes')
    require(set(packed) == set(parent_rows) | HELPERS | {'inputs/' + n for n in names},
            'closed118 original package bodies')
    for n, row in parent_rows.items():
        require(ordinary(n) and pin(packed[n]) == row['after'], 'exact parent overlay body ' + n)
    for n in HELPERS:
        expected[n] = pin(packed[n])
    require(len(expected) == 1256 and sum(v['bytes'] for v in expected.values()) <= 64 << 20,
            'bounded1256 source map')
    inputs = dict(schema='ferric-guarded-mlp-readiness40-host-timing-parent-cpu-input-v1',
        files=dict(sorted(expected.items())), lineage={n: pin(bodies[n]) for n in sorted(names)},
        parent_overlay=sorted(parent_rows), cache_manifest=CACHE_PIN, git_revision=base['git_revision'])
    return inputs, old


def pack():
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'parent-input-manifest.json'),
            'fresh package outputs')
    paths = {'run_cpu.py': P / 'parent_cpu.py', 'supervisor.py': Q / 'supervisor.py',
             'qualification_support.py': Q / 'qualification_support.py',
             'inputs/host-timing-source-manifest.json': PROPOSAL_ROOT / 'source-manifest.json',
             'inputs/parent-complete.json': Q / 'evidence/complete.json',
             'inputs/parent-sources.json': Q / 'evidence/sources-after.json'}
    base = parse(read(paths['inputs/parent-complete.json']))
    proposal = parse(read(paths['inputs/host-timing-source-manifest.json']))
    for n in {'parent-lib-list.stdout', 'metadata.stdout'} | {label + suffix for label in base['tests']
            for suffix in ('.stdout', '.command.json')}:
        paths['inputs/' + n] = Q / 'evidence' / n
    for row in proposal['files']:
        paths['ferric/' + row['path']] = PROPOSAL_ROOT / 'ferric' / row['path']
    bodies = {n: read(path) for n, path in paths.items()}
    inputs, old = contract(bodies)
    for n, row in old.items():
        require(pin(read(F / n.removeprefix('ferric/'))) == row,
                'all1251 actual qualified canonical Ferric bodies')
    for row in proposal['files']:
        if row['before'] is None:
            require(not os.path.lexists(F / row['path']), 'new source path is absent in canonical')
    require(all(read(path) == bodies[n] for n, path in paths.items()), 'all packaging inputs unchanged')
    raw_input = encoded(inputs)
    bodies['input-manifest.json'] = raw_input
    require(len(bodies) == 119 and sum(map(len, bodies.values())) <= 32 << 20, 'bounded119-member package')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive_stream:
            for n, raw in sorted(bodies.items()):
                item = tarfile.TarInfo(n); item.size = len(raw); item.mode = 0o600
                archive_stream.addfile(item, io.BytesIO(raw))
    with (P / 'parent-input-manifest.json').open('xb') as stream:
        stream.write(raw_input)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))),
                         input=pin(raw_input), members=119, source_files=1256), sort_keys=True))


def stage(archive_sha, input_sha):
    require(re.fullmatch('[0-9a-f]{64}', archive_sha) and re.fullmatch('[0-9a-f]{64}', input_sha),
            'observed two SHA arguments')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact staging host/UID')
    require(E.resolve(strict=True) == E and not os.path.lexists(ROOT), 'fresh exact parent root')
    require(shutil.disk_usage(E).free >= 40 << 30, '40GiB initial free floor')
    archive_raw = read(E / BASENAME)
    require(pin(archive_raw)['sha256'] == archive_sha and len(archive_raw) <= 32 << 20,
            'actual bounded source archive')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as archive_stream:
        members = archive_stream.getmembers()
        require(len(members) == len({m.name for m in members}) == 119
                and all(m.isfile() and ordinary(m.name) and not m.pax_headers
                        and 0 <= m.size <= 8 << 20 for m in members)
                and sum(m.size for m in members) <= 32 << 20, 'closed regular USTAR package')
        bodies = {m.name: archive_stream.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'complete member reads')
    raw_input = bodies.pop('input-manifest.json')
    require(pin(raw_input)['sha256'] == input_sha, 'actual source input')
    expected, old = contract(bodies)
    require(parse(raw_input) == expected, 'entire reconstructed input contract')
    require(read(BASE / 'evidence/complete.json') == bodies['inputs/parent-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/parent-sources.json'],
            'immutable actual parent baseline receipts')
    actual_paths = set()
    for directory, dirs, names in os.walk(BASE / 'ferric', followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(not any((Path(directory) / n).is_symlink() for n in dirs), 'no baseline directory aliases')
        actual_paths.update(str((Path(directory) / n).relative_to(BASE)) for n in names)
    require(actual_paths == set(old), 'exact original source subtree roster')
    require(all(pin(read(BASE / n)) == row for n, row in old.items()), 'all1251 actual parent prehashes')
    os.umask(0o077); ROOT.mkdir(mode=0o700)
    def write(n, raw):
        require(ordinary(n), 'ordinary destination')
        path = ROOT / n; path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
    for n in old:
        if n not in bodies:
            write(n, read(BASE / n))
    for n, raw in bodies.items():
        write(n, raw)
    write('input-manifest.json', raw_input)
    require(all(pin(read(ROOT / n)) == row for n, row in expected['files'].items()),
            'all1256 staged source rows')
    require(all(pin(read(BASE / n)) == row for n, row in old.items()), 'immutable baseline body posthashes')
    require(read(E / BASENAME) == archive_raw, 'archive unchanged during staging')
    require(read(BASE / 'evidence/complete.json') == bodies['inputs/parent-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/parent-sources.json'],
            'immutable baseline receipt posthashes')
    receipt = dict(schema='ferric-guarded-mlp-readiness40-host-timing-parent-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=1256, ferric_files=1253,
        overlay_files=4, inherited_worker_files=203, worker_substitutions=0, helpers=3, lineage_files=111,
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
    if len(sys.argv) == 2 and sys.argv[1] == 'pack':
        pack()
    elif len(sys.argv) == 4 and sys.argv[1] == 'stage':
        stage(sys.argv[2], sys.argv[3])
    else:
        raise ValueError('parent_transport.py pack | stage ARCHIVE_SHA INPUT_SHA')
    signal.alarm(0)


if __name__ == '__main__':
    main()

