"""Data-only qualified Shared parent clone with four timed-composition overlays."""
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
PROPOSAL_ROOT = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/proposals/guarded-mlp-model-interface-v228-v1/model-guarded-full2303-v1/shared-full-readiness40-v1/timed-followup-v1')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/guarded-mlp-readiness40-shared-full-v1/parent-cpu-attempt-v2'
QWORKER = F / 'qualification/guarded-mlp-readiness40-shared-full-v1/worker-cpu-v1'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-readiness40-shared-full-parent-cpu-v228-v2'
ROOT = E / 'guarded-mlp-readiness40-shared-full-parent-cpu-v228-v3'
OLD_ROOT = str(BASE)
WORKER_ROOT = str(E / 'guarded-mlp-readiness40-shared-full-worker-cpu-v228-v1')
WORKER_COMPLETE = dict(bytes=1746409, sha256='4a016b7e09b0cc6f9b4bd32c98a5713f24709bd6509564a589c29b0478f7e337')
WORKER_SOURCES = dict(bytes=428240, sha256='2566997ffb73fb40812688178f4494e48c1db4c8e7522717d54b9176994fd3e1')
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-readiness40-shared-full-parent-input-v228-v3.tar.gz'
CONTROLLER_PIN = dict(bytes=34941, sha256='b6cdc0a1918e7b5391ede2121a43a5d540b02b5c7b79cf428cab001c85ae3fe6')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13')
BASE_COMPLETE = dict(bytes=3983617, sha256='7e4ad4324ea3b0302e1f7cf738b699e0e0721025963e659f0ba7f5bc35d70a8d')
BASE_SOURCES = dict(bytes=507330, sha256='82c6cdd0f0af07173777b44aad9a457f8092234f3ae0415eb778bad8492f1118')
PROPOSAL_PIN = dict(bytes=6675, sha256='44729f6171a55af6d666fe91c70652dd39eb5c54e7560faa9f24f273dbf3bf6f')
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
             'timed-shared-source-manifest.json': PROPOSAL_PIN}
    bodies = {n.removeprefix('inputs/'): raw for n, raw in packed.items() if n.startswith('inputs/')}
    require(all(pin(bodies[n]) == v for n, v in fixed.items()), 'literal actual parent/source identities')
    base = parse(bodies['parent-complete.json'])
    old_map = parse(bodies['parent-sources.json'])
    proposal = parse(bodies['timed-shared-source-manifest.json'])
    require(base['schema'] == 'ferric-guarded-mlp-readiness40-shared-full-parent-cpu-v1'
            and len(old_map) == 1260 and len(base['phases']) == 64 and len(base['tests']) == 54
            and len(base['inventory']) == 946 and sum(v['passed'] for v in base['tests'].values()) == 475
            and all(v['failed'] == v['ignored'] == 0 for v in base['tests'].values())
            and base['readiness40_shared_full_parent_route_added'] is True
            and base['readiness40_shared_full_native_execution'] is False
            and base['qualified_worker_sources_preserved'] is True
            and base['worker_qualification'] == WORKER_COMPLETE
            and base['worker_source_manifest'] == WORKER_SOURCES
            and base['parent_host_timing_source_added'] is True
            and base['parent_host_timing_native_execution'] is False
            and base['parent_host_timing_rows'] == 40 and base['parent_host_timing_disjoint_spans'] == 124
            and len(base['artifacts']) == 7, 'actual475 selected Shared parent and unchanged205 worker baseline')
    require(base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['input_sources'] == base['final_sources'] == old_map
            and base['source_unchanged'] is True and base['gpu_execution'] is False
            and compact(base['raw']['sources-after.json']) == BASE_SOURCES
            and all(p['exit_code'] == 0 and p['natural_exit'] is True and p['reaped'] is True
                and p['process_group_absent'] is True and p['forced_cleanup'] is False
                and p['timed_out'] is False and p['exception'] is None and p['storage_failure'] is None
                for p in base['phases']), 'actual clean source/CPU lifecycle')
    require(proposal['schema'] == 'ferric-readiness40-position5-shared-full-host-timing-source-v1'
            and proposal['base']['parent_terminal'] == BASE_COMPLETE
            and proposal['base']['parent_source_map'] == BASE_SOURCES
            and proposal['scope']['combined_parent_entry_added'] is True
            and proposal['scope']['private_shared_recorder_prohibition_removed'] is True
            and proposal['scope']['event_count'] == 124 and proposal['scope']['forward_rows'] == 40
            and proposal['scope']['generated_tokens'] == 0
            and proposal['scope']['capture_positions'] == [0, 5, 16, 39]
            and proposal['scope']['explicit_selector']
                == '--observe-guarded-readiness40-position5-shared-full-host-timing'
            and proposal['scope']['wrapper_schema']
                == 'FerricReadiness40Position5SharedFullTimedObservationV1'
            and all(proposal['scope'][k] is False for k in (
                'compiled', 'tests_executed', 'native_execution', 'old_entry_signatures_changed',
                'config_struct_changed', 'default_policy_changed', 'shared_policy_changed',
                'recorder_scope_changed', 'wire_changed', 'ordinary_observation_schema_changed',
                'worker_changed', 'runtime_changed', 'kernel_images_changed', 'deadline_changed',
                'retention_cap_changed', 'features_changed', 'dependency_graph_changed',
                'numerical_acceptance', 'performance_claim', 'full2303_feasibility')),
            'exact source-only timed Shared composition without policy or bound changes')
    old = {n: compact(v) for n, v in old_map.items() if n.startswith('ferric/')}
    require(len(old) == 1257 and sum(n.startswith(WORKER_PREFIX) for n in old) == 205
            and all(old_map[n]['path'] == OLD_ROOT + '/' + n for n in old),
            'one actual parent closure with205 unchanged worker bodies')
    rows = {'ferric/' + r['path']: r for r in proposal['files']}
    parent_rows = dict(rows)
    require(len(rows) == len(proposal['files']) == 4
            and all(n.startswith('ferric/' + PARENT_REL + '/') and n.endswith('.rs')
                    and r['repository'] == 'ferric' and r['before'] is not None for n, r in rows.items()),
            'exact four existing parent paths, no worker overlay or addition')
    expected = dict(old)
    for n, row in rows.items():
        require(expected.get(n) == row['before'], 'actual formatted parent preimage ' + n)
        expected[n] = row['after']
    require({n: r for n, r in expected.items() if n.startswith(WORKER_PREFIX)}
            == {n: r for n, r in old.items() if n.startswith(WORKER_PREFIX)},
            'all205 qualified worker bodies preserved without substitution')

    require(all(ordinary(n) for n in old) and all(ordinary(n) for n in rows), 'ordinary closed source paths')
    names = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    names |= {label + suffix for label in base['tests'] for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == names and len(names) == 113, 'closed113 direct lineage bodies')
    require(all(pin(bodies[n]) == compact(base['raw'][n]) for n in names - set(fixed)),
            'actual current recipes and raw named outcomes')
    require(set(packed) == set(rows) | HELPERS | {'inputs/' + n for n in names},
            'closed120 original package bodies')
    for n in rows:
        require(pin(packed[n]) == expected[n], 'exact authored parent overlay ' + n)
    for n in HELPERS:
        expected[n] = pin(packed[n])
    require(len(expected) == 1260 and sum(v['bytes'] for v in expected.values()) <= 64 << 20,
            'bounded1260 source map')
    inputs = dict(schema='ferric-guarded-mlp-readiness40-shared-full-host-timing-parent-cpu-input-v1',
        files=dict(sorted(expected.items())), lineage={n: pin(bodies[n]) for n in sorted(names)},
        parent_overlay=sorted(parent_rows), cache_manifest=CACHE_PIN, git_revision=base['git_revision'])
    return inputs, old


def pack():
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'parent-input-manifest.json'),
            'fresh package outputs')
    paths = {'run_cpu.py': P / 'parent_cpu.py', 'supervisor.py': Q / 'supervisor.py',
             'qualification_support.py': Q / 'qualification_support.py',
             'inputs/timed-shared-source-manifest.json': PROPOSAL_ROOT / 'source-manifest.json',
             'inputs/parent-complete.json': Q / 'evidence/complete.json',
             'inputs/parent-sources.json': Q / 'evidence/sources-after.json'}
    base = parse(read(paths['inputs/parent-complete.json']))
    proposal = parse(read(paths['inputs/timed-shared-source-manifest.json']))
    for n in {'parent-lib-list.stdout', 'metadata.stdout'} | {label + suffix for label in base['tests']
            for suffix in ('.stdout', '.command.json')}:
        paths['inputs/' + n] = Q / 'evidence' / n
    for row in proposal['files']:
        n = 'ferric/' + row['path']
        paths[n] = PROPOSAL_ROOT / n
    bodies = {n: read(path) for n, path in paths.items()}
    inputs, old = contract(bodies)
    require(len(old) == 1257 and sum(n.startswith(WORKER_PREFIX) for n in old) == 205,
            'canonical qualified parent and worker preparation closure')
    for n, row in old.items():
        require(pin(read(F / n.removeprefix('ferric/'))) == row,
                'all1257 canonical bodies equal the actual predecessor including205 worker files')
    require(all(read(path) == bodies[n] for n, path in paths.items()), 'all packaging inputs unchanged')
    raw_input = encoded(inputs)
    bodies['input-manifest.json'] = raw_input
    require(len(bodies) == 121 and sum(map(len, bodies.values())) <= 32 << 20, 'bounded121-member package')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive_stream:
            for n, raw in sorted(bodies.items()):
                item = tarfile.TarInfo(n); item.size = len(raw); item.mode = 0o600
                archive_stream.addfile(item, io.BytesIO(raw))
    with (P / 'parent-input-manifest.json').open('xb') as stream:
        stream.write(raw_input)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))),
                         input=pin(raw_input), members=121, source_files=1260), sort_keys=True))


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
        require(len(members) == len({m.name for m in members}) == 121
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
    require(all(pin(read(BASE / n)) == row for n, row in old.items()), 'all1257 actual parent prehashes')
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
            'all1260 staged source rows')
    require(all(pin(read(BASE / n)) == row for n, row in old.items()), 'immutable baseline body posthashes')
    require(read(E / BASENAME) == archive_raw, 'archive unchanged during staging')
    require(read(BASE / 'evidence/complete.json') == bodies['inputs/parent-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/parent-sources.json'],
            'immutable baseline receipt posthashes')
    receipt = dict(schema='ferric-guarded-mlp-readiness40-shared-full-host-timing-parent-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=1260, ferric_files=1257,
        overlay_files=4, parent_format_paths=4, qualified_worker_files=205, qualified_worker_sources_preserved=True,
        helpers=3, lineage_files=113,
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
