"""Data-only actual-parent clone, actual Full worker map, and seven parent overlays."""
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
PROPOSAL_ROOT = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/proposals/guarded-mlp-model-interface-v228-v1/model-guarded-full2303-v1')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/guarded-mlp-readiness40-causal-layer0-v1/parent-cpu-attempt-v2'
WQ = F / 'qualification/guarded-mlp-readiness40-causal-layer0-v1/worker-cpu-v1'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-readiness40-causal-layer0-parent-cpu-v228-v2'
WORKER_BASE = E / 'guarded-mlp-readiness40-causal-layer0-worker-cpu-v228-v1'
QUALIFIED_WORKER_BASE = E / 'guarded-mlp-full2303-worker-cpu-v228-v1'
ROOT = E / 'guarded-mlp-full2303-parent-cpu-v228-v1'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-full2303-parent-input-v228-v1.tar.gz'
CONTROLLER_PIN = dict(bytes=40651, sha256='28554bf90a97579450066fecfb4d4b69ab4aa51b7a22615519546684b53d8c0b')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13')
BASE_COMPLETE = dict(bytes=3942470, sha256='f2c161753e64f9f2b8b4049ae75cac13332b7eb14217eaad247c8c2e9e13d4ea')
BASE_SOURCES = dict(bytes=502636, sha256='54b516e6cd3d95973c725923d69d1fd7b9e6fa4ff7b3d4f73272282ed3ba2ae2')
WORKER_COMPLETE = dict(bytes=1731960, sha256='5c4ffff4b06c7ad1d73c6985c6bcb0193b596ab6618bbda44010347a9b26aa1a')
WORKER_SOURCES = dict(bytes=426647, sha256='00b15b5478ebbeb6fbd00a2de68e24bb6381a6f28a521fc566b95539aa3e35a5')
QUALIFIED_WORKER = dict(bytes=1691615, sha256='0b91b220824f093b0297134ff21e4ddc2dde64fd17909439bb15421fb862e55f')
QUALIFIED_WORKER_SOURCES = dict(bytes=412019, sha256='0fc0a33c61dd0e6f70903bb246654bfeb99095fd515be53c828d8d91f646e7cb')
PROPOSAL_PIN = dict(bytes=4291, sha256='e29ee6f6fd37ede4f9b4c990e0427bb287ce8e066de01f136fb4ab82961f6793')
WORKER_PROPOSAL_PIN = dict(bytes=11547, sha256='8472f1749c10257cadb01f585c18e39fb76c195381ded1408be8254a01627a6b')
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
    require(CONTROLLER_PIN is not None and QUALIFIED_WORKER is not None and QUALIFIED_WORKER_SOURCES is not None,
            'actual Full worker and reviewed controller pins must be bound')
    require(pin(packed['run_cpu.py']) == CONTROLLER_PIN
            and pin(packed['supervisor.py']) == SUPERVISOR_PIN
            and pin(packed['qualification_support.py']) == SUPPORT_PIN, 'three exact reviewed harness inputs')
    fixed = {'parent-complete.json': BASE_COMPLETE, 'parent-sources.json': BASE_SOURCES,
             'worker-complete.json': WORKER_COMPLETE, 'worker-sources.json': WORKER_SOURCES,
             'qualified-worker-complete.json': QUALIFIED_WORKER,
             'qualified-worker-sources.json': QUALIFIED_WORKER_SOURCES,
             'parent-source-manifest.json': PROPOSAL_PIN, 'worker-source-manifest.json': WORKER_PROPOSAL_PIN}
    bodies = {n.removeprefix('inputs/'): raw for n, raw in packed.items() if n.startswith('inputs/')}
    require(all(pin(bodies[n]) == v for n, v in fixed.items()), 'literal actual baseline/source identities')
    base = parse(bodies['parent-complete.json'])
    old_map = parse(bodies['parent-sources.json'])
    worker = parse(bodies['worker-complete.json'])
    worker_map = parse(bodies['worker-sources.json'])
    qualified = parse(bodies['qualified-worker-complete.json'])
    qualified_map = parse(bodies['qualified-worker-sources.json'])
    proposal = parse(bodies['parent-source-manifest.json'])
    worker_proposal = parse(bodies['worker-source-manifest.json'])
    require(base['schema'] == 'ferric-guarded-mlp-readiness40-causal-layer0-parent-cpu-v2'
            and len(old_map) == 1244 and len(base['phases']) == 60 and len(base['tests']) == 51
            and len(base['inventory']) == 918 and sum(v['passed'] for v in base['tests'].values()) == 444
            and all(v['failed'] == v['ignored'] == 0 for v in base['tests'].values())
            and base['parent_causal_file_reads_retained'] is True,
            'actual selected causal parent V2 baseline')
    require(worker['schema'] == 'ferric-guarded-mlp-readiness40-causal-layer0-worker-cpu-v1'
            and len(worker_map) == 1014 and len(worker['phases']) == 9
            and len(worker['inventory']) == 684
            and worker['tests']['worker-tests']['passed'] == 680
            and worker['tests']['worker-tests']['failed'] == 0
            and worker['tests']['worker-tests']['ignored'] == 4
            and worker['cli_executable_unchanged_across_tests'] is True, 'actual causal worker baseline')
    require(qualified['schema'] == 'ferric-guarded-mlp-full2303-worker-cpu-v1'
            and len(qualified_map) == 1020 and len(qualified['phases']) == 9
            and len(qualified['inventory']) == 698
            and qualified['tests']['worker-tests']['passed'] == 694
            and qualified['tests']['worker-tests']['failed'] == 0
            and qualified['tests']['worker-tests']['ignored'] == 4
            and qualified['full2303_source_added'] is True
            and qualified['full2303_native_execution'] is False
            and qualified['full2303_launch_feasibility'] is False
            and qualified['full2303_source_abort_ms'] == 3600000
            and qualified['inherited_causal_layer0_route_preserved'] is True
            and qualified['cli_executable_unchanged_across_tests'] is True
            and qualified['cli_executable_before_tests']['pin'] == qualified['artifacts']['worker']['pin']
            and set(qualified['artifacts']) == {'worker-lib', 'worker-bin-test', 'worker-wire-test',
                'worker-readiness-test', 'worker'}
            and len({r['pin']['path'] for r in qualified['artifacts'].values()}) == 5,
            'observed full-worker CPU/real-executable contract')
    for result, source_map, expected_pin in ((base, old_map, BASE_SOURCES),
            (worker, worker_map, WORKER_SOURCES), (qualified, qualified_map, QUALIFIED_WORKER_SOURCES)):
        require(result['passed'] is True and result['failure'] is None and result['postcheck_errors'] == []
                and result['input_sources'] == result['final_sources'] == source_map
                and result['source_unchanged'] is True and result['gpu_execution'] is False
                and compact(result['raw']['sources-after.json']) == expected_pin
                and all(p['exit_code'] == 0 and p['natural_exit'] is True and p['reaped'] is True
                    and p['process_group_absent'] is True and p['forced_cleanup'] is False
                    and p['timed_out'] is False and p['exception'] is None and p['storage_failure'] is None
                    for p in result['phases']), 'actual clean source/CPU lifecycle')
    require(proposal['schema'] == 'ferric-guarded-full2303-parent-source-proposal-v1'
            and proposal['source_only'] is True
            and proposal['base_revision'] == 'b61588712af31738d3a744fae81b6afcfdd9ea9a'
            and all(v is False for v in proposal['claims'].values())
            and proposal['unchanged_contracts'] == dict(pure_long_wire=True, pure_long_sequence=True,
                readiness40_native_entries=True, ar4_entries=True, stream_bytes=67108864,
                evidence_bytes=33554432, full_deadline_ms=3600000, launch_feasibility_established=False),
            'frozen parent source scope, not launch authority')
    require(worker_proposal['schema'] == 'ferric-guarded-mlp-full2303-worker-source-v1'
            and worker_proposal['source_only'] is True
            and compact(worker_proposal['base']['worker_complete']) == WORKER_COMPLETE
            and compact(worker_proposal['base']['worker_sources']) == WORKER_SOURCES
            and worker_proposal['canonical_commit'] == proposal['base_revision']
            and all(worker_proposal[k] is False for k in ('tests_executed', 'native_execution',
                'launch_feasibility', 'numerical_acceptance', 'performance_claim',
                'currentness_policy_changed', 'existing_ar4_cap_changed', 'existing_readiness_cap_changed')),
            'frozen full-worker source and unchanged old limits')
    old = {n: compact(v) for n, v in old_map.items() if n.startswith('ferric/')}
    baseline_worker = {n: compact(v) for n, v in worker_map.items() if n.startswith(WORKER_PREFIX)}
    actual_worker = {n: compact(v) for n, v in qualified_map.items() if n.startswith(WORKER_PREFIX)}
    authored_worker = {n: compact(v) for n, v in qualified['preformat_sources'].items()
                       if n.startswith(WORKER_PREFIX)}
    require(len(old) == 1241 and len(baseline_worker) == 197 and set(baseline_worker) <= set(old)
            and len(authored_worker) == len(actual_worker) == 203, 'explicit parent/worker source boundaries')
    transitions = {n: dict(before=old[n], after=r) for n, r in baseline_worker.items() if old[n] != r}
    require(len(transitions) == 8, 'eight directly authenticated causal-worker formatter transitions')
    expected_authored = dict(baseline_worker)
    worker_rows = {'ferric/' + r['path']: r for r in worker_proposal['files']}
    require(len(worker_rows) == len(worker_proposal['files']) == 13
            and all(n.startswith(WORKER_PREFIX) for n in worker_rows)
            and sum(r['before'] is None for r in worker_rows.values()) == 6,
            'thirteen worker overlays and six additions')
    for n, row in worker_rows.items():
        require(expected_authored.get(n) == row['before'], 'actual worker proposal preimage ' + n)
        expected_authored[n] = row['after']
    require(expected_authored == authored_worker and set(authored_worker) == set(actual_worker),
            'future worker actual preformat map exactly equals reviewed overlay')
    worker_changes = {n for n in actual_worker if actual_worker[n] != authored_worker[n]}
    require(worker_changes == set(qualified['format_changed_paths'])
            and worker_changes <= {n for n in worker_rows if n.endswith('.rs')},
            'qualified worker formatting changes remain explicit')
    expected = dict(old)
    expected.update(actual_worker)
    parent_rows = {'ferric/' + r['path']: r for r in proposal['files']}
    require(len(parent_rows) == len(proposal['files']) == 7
            and all(n.startswith('ferric/' + PARENT_REL + '/') for n in parent_rows)
            and sum(r['before'] is None for r in parent_rows.values()) == 4,
            'seven parent overlays and four additions')
    for n, row in parent_rows.items():
        require(expected.get(n) == row['before'], 'actual parent proposal preimage ' + n)
        expected[n] = row['after']
    require(len(expected) == 1251, 'complete parent Ferric closure')
    wire_name = WORKER_PREFIX + 'src/finite_guarded_mlp_full2303_wire_v1.rs'
    require(worker_rows[wire_name]['after'] == proposal['worker_wire_api']['pin'],
            'parent API is the reviewed worker wire preformat body')
    names = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    names |= {label + suffix for label in base['tests'] for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == names and len(names) == 112, 'closed112 direct lineage bodies')
    require(all(pin(bodies[n]) == compact(base['raw'][n]) for n in names - set(fixed)),
            'actual current recipes and raw named outcomes')
    require(all(ordinary(n) and old_map[n]['path'] == str(BASE / n) for n in old)
            and all(worker_map[n]['path'] == str(WORKER_BASE / n) for n in baseline_worker)
            and all(qualified_map[n]['path'] == str(QUALIFIED_WORKER_BASE / n) for n in actual_worker),
            'exact original immutable source roots')
    require(set(packed) == set(parent_rows) | HELPERS | {'inputs/' + n for n in names},
            'closed122 original package bodies')
    for n, row in parent_rows.items():
        require(ordinary(n) and pin(packed[n]) == row['after'], 'exact parent overlay body ' + n)
    for n in HELPERS:
        expected[n] = pin(packed[n])
    require(len(expected) == 1254 and sum(v['bytes'] for v in expected.values()) <= 64 << 20,
            'bounded1254 source map')
    inputs = dict(schema='ferric-guarded-mlp-full2303-parent-cpu-input-v1',
        files=dict(sorted(expected.items())), lineage={n: pin(bodies[n]) for n in sorted(names)},
        parent_overlay=sorted(n for n in parent_rows if n.endswith('.rs')),
        cache_manifest=CACHE_PIN, git_revision=base['git_revision'])
    require(len(inputs['parent_overlay']) == 6, 'six parent Rust formatting inputs')
    return inputs, old, actual_worker


def pack(worker_capsule):
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'parent-input-manifest.json'),
            'fresh package outputs')
    require(worker_capsule.is_absolute() and worker_capsule.resolve(strict=True) == worker_capsule,
            'explicit actual retained Full-worker capsule')
    paths = {'run_cpu.py': P / 'parent_cpu.py', 'supervisor.py': Q / 'supervisor.py',
             'qualification_support.py': Q / 'qualification_support.py',
             'inputs/parent-source-manifest.json': PROPOSAL_ROOT / 'parent-v1/source-manifest.json',
             'inputs/worker-source-manifest.json': PROPOSAL_ROOT / 'worker-v1/source-manifest.json'}
    for prefix, directory in [('parent', Q), ('worker', WQ), ('qualified-worker', worker_capsule)]:
        paths['inputs/' + prefix + '-complete.json'] = directory / 'evidence/complete.json'
        paths['inputs/' + prefix + '-sources.json'] = directory / 'evidence/sources-after.json'
    base = parse(read(paths['inputs/parent-complete.json']))
    proposal = parse(read(paths['inputs/parent-source-manifest.json']))
    for n in {'parent-lib-list.stdout', 'metadata.stdout'} | {label + suffix for label in base['tests']
            for suffix in ('.stdout', '.command.json')}:
        paths['inputs/' + n] = Q / 'evidence' / n
    for row in proposal['files']:
        paths['ferric/' + row['path']] = PROPOSAL_ROOT / 'parent-v1/ferric' / row['path']
    bodies = {n: read(path) for n, path in paths.items()}
    inputs, old, worker_files = contract(bodies)
    # The capsule retains overlay bodies only; canonical bodies require the entire actual map join.
    for n, row in worker_files.items():
        require(pin(read(F / n.removeprefix('ferric/'))) == row, 'all203 actual qualified canonical worker bodies')
    for n, row in old.items():
        if not n.startswith(WORKER_PREFIX):
            require(pin(read(F / n.removeprefix('ferric/'))) == row, 'unchanged actual canonical parent closure')
    require(all(read(path) == bodies[n] for n, path in paths.items()), 'all packaging inputs unchanged')
    raw_input = encoded(inputs)
    bodies['input-manifest.json'] = raw_input
    require(len(bodies) == 123 and sum(map(len, bodies.values())) <= 32 << 20, 'bounded123-member package')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive_stream:
            for n, raw in sorted(bodies.items()):
                item = tarfile.TarInfo(n); item.size = len(raw); item.mode = 0o600
                archive_stream.addfile(item, io.BytesIO(raw))
    with (P / 'parent-input-manifest.json').open('xb') as stream:
        stream.write(raw_input)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))),
                         input=pin(raw_input), members=123, source_files=1254), sort_keys=True))


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
        require(len(members) == len({m.name for m in members}) == 123
                and all(m.isfile() and ordinary(m.name) and not m.pax_headers
                        and 0 <= m.size <= 8 << 20 for m in members)
                and sum(m.size for m in members) <= 32 << 20, 'closed regular USTAR package')
        bodies = {m.name: archive_stream.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'complete member reads')
    raw_input = bodies.pop('input-manifest.json')
    require(pin(raw_input)['sha256'] == input_sha, 'actual source input')
    expected, old, worker_files = contract(bodies)
    require(parse(raw_input) == expected, 'entire reconstructed input contract')
    bases = [('parent', BASE), ('worker', WORKER_BASE), ('qualified-worker', QUALIFIED_WORKER_BASE)]
    for prefix, base in bases:
        require(read(base / 'evidence/complete.json') == bodies['inputs/' + prefix + '-complete.json']
                and read(base / 'evidence/sources-after.json') == bodies['inputs/' + prefix + '-sources.json'],
                'immutable actual baseline receipts')
    for base, subtree, expected_names in ((BASE, BASE / 'ferric', set(old)),
            (QUALIFIED_WORKER_BASE, QUALIFIED_WORKER_BASE / WORKER_PREFIX, set(worker_files))):
        actual_paths = set()
        for directory, dirs, names in os.walk(subtree, followlinks=False,
                onerror=lambda error: (_ for _ in ()).throw(error)):
            require(not any((Path(directory) / n).is_symlink() for n in dirs), 'no baseline directory aliases')
            actual_paths.update(str((Path(directory) / n).relative_to(base)) for n in names)
        require(actual_paths == expected_names, 'exact original source subtree roster')
    require(all(pin(read(BASE / n)) == row for n, row in old.items())
            and all(pin(read(QUALIFIED_WORKER_BASE / n)) == row for n, row in worker_files.items()),
            'all1241 original parent and203 actual worker prehashes')
    os.umask(0o077); ROOT.mkdir(mode=0o700)
    def write(n, raw):
        require(ordinary(n), 'ordinary destination')
        path = ROOT / n; path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
    for n in old:
        if not n.startswith(WORKER_PREFIX) and n not in bodies:
            write(n, read(BASE / n))
    for n in worker_files:
        write(n, read(QUALIFIED_WORKER_BASE / n))
    for n, raw in bodies.items():
        write(n, raw)
    write('input-manifest.json', raw_input)
    require(all(pin(read(ROOT / n)) == row for n, row in expected['files'].items()),
            'all1254 staged source rows')
    require(all(pin(read(BASE / n)) == row for n, row in old.items())
            and all(pin(read(QUALIFIED_WORKER_BASE / n)) == row for n, row in worker_files.items()),
            'immutable baseline body posthashes')
    require(read(E / BASENAME) == archive_raw, 'archive unchanged during staging')
    for prefix, base in bases:
        require(read(base / 'evidence/complete.json') == bodies['inputs/' + prefix + '-complete.json']
                and read(base / 'evidence/sources-after.json') == bodies['inputs/' + prefix + '-sources.json'],
                'immutable baseline receipt posthashes')
    receipt = dict(schema='ferric-guarded-mlp-full2303-parent-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=1254, ferric_files=1251,
        overlay_files=7, qualified_worker_files=203, inherited_worker_transitions=8,
        helpers=3, lineage_files=112,
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
    if len(sys.argv) == 3 and sys.argv[1] == 'pack':
        pack(Path(sys.argv[2]))
    elif len(sys.argv) == 4 and sys.argv[1] == 'stage':
        stage(sys.argv[2], sys.argv[3])
    else:
        raise ValueError('parent_transport.py pack ACTUAL_WORKER_CAPSULE | stage ARCHIVE_SHA INPUT_SHA')
    signal.alarm(0)


if __name__ == '__main__':
    main()
