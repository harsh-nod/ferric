"""Data-only scoped parent clone using actually qualified worker postimages."""
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
PROPOSAL_ROOT = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/proposals/guarded-mlp-model-interface-v228-v1/model-guarded-full2303-v1/scoped-parent-v1')
FULL = PROPOSAL_ROOT.parent
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/guarded-mlp-readiness40-shared-full-v1/parent-cpu-attempt-v3'
QWORKER = F / 'qualification/guarded-mlp-scoped-currentness-v1/cpu-attempt-v3'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-readiness40-shared-full-parent-cpu-v228-v3'
ROOT = E / 'guarded-mlp-readiness40-scoped-warm-parent-cpu-v228-v3'
OLD_ROOT = str(BASE)
WORKER_ROOT = str(E / 'guarded-mlp-scoped-currentness-cpu-v228-v3')
BASE_WORKER_COMPLETE = dict(bytes=1746409, sha256='4a016b7e09b0cc6f9b4bd32c98a5713f24709bd6509564a589c29b0478f7e337')
WORKER_COMPLETE = dict(bytes=2364691, sha256='335cf93cc109390f3b7590f7dbe35ed852adca223a042894dcc18418a738ffbd')
BASE_WORKER_SOURCES = dict(bytes=428240, sha256='2566997ffb73fb40812688178f4494e48c1db4c8e7522717d54b9176994fd3e1')
WORKER_SOURCES = dict(bytes=420283, sha256='e5ca47a688908d4c78d1e3157211aee4c29bf12f626c01258f06a8ca8eefbda3')
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-readiness40-scoped-warm-parent-input-v228-v3.tar.gz'
CONTROLLER_PIN = dict(bytes=45154, sha256='39fd25f0feecf5204363a0942615f5f6cd543df7a52075597210a35e53df00cf')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13')
BASE_COMPLETE = dict(bytes=3983845, sha256='f7134c5f22d75c8295b8ab99ad8625fac40aa615ac1682340c0b8e6ffde1f71e')
BASE_SOURCES = dict(bytes=507332, sha256='9e70e32da52ebba96a8354ecd5fe60bbb3add9a7d1b93ba0d218572349f129c4')
PROPOSAL_PINS = {
    "selector-repair.json": {
        "bytes": 16911,
        "sha256": "2659a6e7d2dd71a22fa5579be3a70be948b81c806cb511c79633e8739eb375a0"
    },
    "consumer-repair.json": {
        "bytes": 4565,
        "sha256": "c2be6fe83fba6c4bfef971bdf1e1535b88167ec8685f6faf6aa163abbdaa93e0"
    },
    "parent-proposal.json": {
        "bytes": 11484,
        "sha256": "687fa059a6eb2744ad55618fe7b59830ec36a4f096e6cf5ee456bf4ee5056db8"
    },
    "consumer-proposal.json": {
        "bytes": 1677,
        "sha256": "6b4dffbe4032e52f6dddb80f255553c420e95a3fb7af84f2b1da311daf1168bb"
    },
    "selector-proposal.json": {
        "bytes": 15169,
        "sha256": "cfec625935042c416a329cfb0e02c1fc8836f12820967e1d3c0257ed2d6a8235"
    },
    "runtime-proposal.json": {
        "bytes": 13373,
        "sha256": "437948cd334c9f95dcf77de4e3217c92f4d8e6d4cabbcf5010aff182f53ae5b2"
    },
    "primitive-proposal.json": {
        "bytes": 3820,
        "sha256": "9abcd59d4c3fd23d532bd1b9e86592a191e62960d8a901fbd31d8e61974bdec7"
    }
}
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
    require(CONTROLLER_PIN is not None and WORKER_COMPLETE is not None and WORKER_SOURCES is not None,
            'reviewed controller and actual coupled worker pins must be bound')
    require(pin(packed['run_cpu.py']) == CONTROLLER_PIN
            and pin(packed['supervisor.py']) == SUPERVISOR_PIN
            and pin(packed['qualification_support.py']) == SUPPORT_PIN, 'three exact reviewed harness inputs')
    fixed = dict(PROPOSAL_PINS, **{'parent-complete.json': BASE_COMPLETE,
        'parent-sources.json': BASE_SOURCES, 'worker-complete.json': WORKER_COMPLETE,
        'worker-sources.json': WORKER_SOURCES})
    bodies = {n.removeprefix('inputs/'): raw for n, raw in packed.items() if n.startswith('inputs/')}
    require(all(pin(bodies[n]) == v for n, v in fixed.items()), 'literal actual parent/worker/source identities')
    base = parse(bodies['parent-complete.json'])
    old_map = parse(bodies['parent-sources.json'])
    proposal = parse(bodies['parent-proposal.json'])
    worker = parse(bodies['worker-complete.json'])
    worker_map = parse(bodies['worker-sources.json'])
    selector = parse(bodies['selector-proposal.json'])
    selector_repaired = parse(bodies['selector-repair.json'])
    consumer = parse(bodies['consumer-proposal.json'])
    repaired = parse(bodies['consumer-repair.json'])
    runtime = parse(bodies['runtime-proposal.json'])
    primitive = parse(bodies['primitive-proposal.json'])
    require(base['schema'] == 'ferric-guarded-mlp-readiness40-shared-full-host-timing-parent-cpu-v1'
            and len(old_map) == 1260 and len(base['phases']) == 64 and len(base['tests']) == 54
            and len(base['inventory']) == 948 and sum(v['passed'] for v in base['tests'].values()) == 478
            and all(v['failed'] == v['ignored'] == 0 for v in base['tests'].values())
            and base['shared_full_host_timing_parent_source_added'] is True
            and base['readiness40_shared_full_parent_route_added'] is True
            and base['readiness40_shared_full_native_execution'] is False
            and base['qualified_worker_sources_preserved'] is True
            and base['worker_qualification'] == BASE_WORKER_COMPLETE
            and base['worker_source_manifest'] == BASE_WORKER_SOURCES
            and base['parent_host_timing_rows'] == 40 and base['parent_host_timing_disjoint_spans'] == 124
            and len(base['artifacts']) == 7, 'actual478 selected timed Shared parent baseline')
    for receipt, source_map, expected_pin, phase_count in (
            (base, old_map, BASE_SOURCES, 64), (worker, worker_map, WORKER_SOURCES, 26)):
        require(receipt['passed'] is True and receipt['failure'] is None
                and receipt['postcheck_errors'] == [] and receipt['source_unchanged'] is True
                and receipt['gpu_execution'] is False
                and receipt['input_sources'] == receipt['final_sources'] == source_map
                and compact(receipt['raw']['sources-after.json']) == expected_pin
                and len(receipt['phases']) == phase_count
                and all(p['exit_code'] == 0 and p['natural_exit'] is True and p['reaped'] is True
                    and p['process_group_absent'] is True and p['forced_cleanup'] is False
                    and p['timed_out'] is False and p['exception'] is None and p['storage_failure'] is None
                    for p in receipt['phases']), 'actual clean source/CPU lifecycle')
    require(worker['schema'] == 'ferric-guarded-mlp-scoped-currentness-cpu-v1'
            and worker['source_generation'] == 'scoped-currentness-coupled-v3'
            and len(worker_map) == 1033 and len(worker['artifacts']) == 11
            and (worker['tests']['worker-tests']['passed'], worker['tests']['worker-tests']['failed'],
                 worker['tests']['worker-tests']['ignored']) == (726, 0, 4)
            and (worker['tests']['kfd-tests']['passed'], worker['tests']['kfd-tests']['failed'],
                 worker['tests']['kfd-tests']['ignored']) == (1143, 0, 8)
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and worker['scoped_currentness_runtime_source_added'] is True
            and worker['scoped_currentness_worker_source_added'] is True
            and worker['explicit_scoped_readiness_selector_source_added'] is True
            and all(worker[k] is False for k in ('scoped_currentness_native_execution',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'numerical_acceptance', 'performance_claim')),
            'actual coupled routed runtime and real worker executable qualification')
    for name in ('runtime-proposal.json', 'primitive-proposal.json', 'consumer-proposal.json',
                 'consumer-repair.json', 'selector-proposal.json', 'selector-repair.json'):
        require(compact(worker['readset'][name]) == fixed[name], 'actual coupled proposal readset join')
    require(proposal['schema'] == 'ferric-scoped-warm-readiness40-parent-source-v1'
            and compact(proposal['base']['parent_receipt']) == BASE_COMPLETE
            and compact(proposal['base']['parent_sources']) == BASE_SOURCES
            and compact(proposal['requires']['worker_selector']) == fixed['selector-proposal.json']
            and compact(proposal['requires']['consumer']) == fixed['consumer-proposal.json']
            and compact(proposal['requires']['runtime']) == fixed['runtime-proposal.json']
            and all(proposal[k] is False for k in ('canonical_changed', 'compiled', 'tested',
                'native_execution', 'default_routes_changed', 'default_group_policy_changed',
                'temporal_equivalent_to_full', 'performance_claim', 'numerical_acceptance')),
            'exact source-only explicit scoped parent proposal')
    require(selector['schema'] == 'ferric-scoped-warm-readiness40-worker-selector-source-v1'
            and len(selector['files']) == 9 and len(consumer['files']) == 4
            and compact(selector['requires']['consumer']) == fixed['consumer-proposal.json']
            and compact(selector['requires']['runtime']) == fixed['runtime-proposal.json']
            and compact(selector['base']['worker_receipt']) == BASE_WORKER_COMPLETE
            and compact(selector['base']['worker_sources']) == BASE_WORKER_SOURCES
            and runtime['primitive_manifest'] == fixed['primitive-proposal.json']
            and len(runtime['files']) == 22 and len(primitive['files']) == 6,
            'coupled worker and routed runtime source lineage')
    require(repaired['schema'] == consumer['schema']
            and compact(repaired['predecessor_manifest']) == fixed['consumer-proposal.json']
            and repaired['source_generation'] == 'scoped-consumer-v2-module-path-repair'
            and repaired['declared_tests'] == consumer['declared_tests'] == 7
            and all(repaired[k] is False for k in ('compiled', 'tested', 'native_execution',
                'canonical_changed', 'profile_enabled'))
            and len(repaired['files']) == len(consumer['files']) == 4,
            'explicit consumer repair preserves original parent and selector dependencies')
    repair = repaired['repair']
    require(repair == dict(
        path='adapters/tp-peer-finite-engineering-worker-v1/src/state_roster/guarded_mlp_decode_v1/scoped_currentness_v1.rs',
        before_call='crate::native_forward::checked_layer_hidden_pair',
        after_call='crate::native_catalog::forward::checked_layer_hidden_pair',
        changed_occurrences=1, other_three_postimages_unchanged=True, tests_unchanged=True),
        'one registered-module call repair, not a route or test change')
    require(sorted(repaired['new_tests']['worker_library']) == sorted(
        name for names in selector['consumer_tests'].values() for name in names),
        'all seven original consumer test names')
    for old_row, new_row in zip(consumer['files'], repaired['files']):
        require(old_row['path'] == new_row['path'] and old_row['before'] == new_row['before']
                and (new_row['after'] != old_row['after'] if old_row['path'] == repair['path']
                     else new_row == old_row), 'one exact cumulative consumer postimage transition')
    require(selector_repaired['schema'] == selector['schema']
            and compact(selector_repaired['predecessor_manifest']) == fixed['selector-proposal.json']
            and selector_repaired['source_generation'] == 'scoped-selector-v2-owned-reader-signature-repair'
            and len(selector_repaired['files']) == len(selector['files']) == 9
            and all(selector_repaired[k] == selector[k] for k in (
                'base', 'requires', 'new_tests', 'consumer_tests', 'composed_new_tests',
                'conditional_composition', 'route', 'canonical_changed', 'compiled', 'tested',
                'native_execution', 'profile_enabled', 'old_routes_preserved',
                'default_group_policy_changed', 'full2303_profile_enabled',
                'performance_claim', 'numerical_acceptance')),
            'explicit selector test repair preserves original source and policy dependencies')
    selector_repair = selector_repaired['repair']
    require(selector_repair == dict(
        path='adapters/tp-peer-finite-engineering-worker-v1/src/native_guarded_mlp_readiness_cli_v1_tests.rs',
        before_reader='&mut &[u8]', after_reader='&mut io::Cursor<Vec<u8>>',
        changed_occurrences=2,
        changed_test='scoped_warm_publication_deadline_requires_both_pre_and_post_write_to_be_inside',
        other_eight_postimages_unchanged=True, production_bodies_unchanged=True,
        test_names_unchanged=True, original_consumer_requirement_unchanged=True),
        'two concrete owned-reader test types only, no production or assertion change')
    for old_row, new_row in zip(selector['files'], selector_repaired['files']):
        require(old_row['path'] == new_row['path'] and old_row['before'] == new_row['before']
                and ({k: v for k, v in new_row.items() if k != 'after'}
                     == {k: v for k, v in old_row.items() if k != 'after'})
                and (new_row['after'] != old_row['after'] if old_row['path'] == selector_repair['path']
                     else new_row == old_row), 'one exact cumulative selector test postimage transition')
    old = {n: compact(v) for n, v in old_map.items() if n.startswith('ferric/')}
    require(len(old) == 1257 and sum(n.startswith(WORKER_PREFIX) for n in old) == 205
            and all(old_map[n]['path'] == OLD_ROOT + '/' + n for n in old),
            'one actual1257 parent source closure with205 worker bodies')
    worker_rows = {'ferric/' + r['path']: r for source in (repaired, selector_repaired) for r in source['files']}
    require(len(worker_rows) == 13 and all(n.startswith(WORKER_PREFIX) and n.endswith('.rs')
            for n in worker_rows), 'thirteen closed worker overlay paths')
    old_worker = {n: r for n, r in old.items() if n.startswith(WORKER_PREFIX)}
    pre_worker = dict(old_worker)
    for n, row in worker_rows.items():
        require(pre_worker.get(n) == row['before'], 'actual worker preimage or absence ' + n)
        pre_worker[n] = row['after']
    qualified_worker = {n: compact(r) for n, r in worker_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(qualified_worker) == 209 and set(qualified_worker) == set(pre_worker)
            and {n: compact(r) for n, r in worker['preformat_sources'].items() if n.startswith(WORKER_PREFIX)}
                == pre_worker
            and all(qualified_worker[n] == r for n, r in pre_worker.items() if n not in worker_rows)
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n for n in qualified_worker)
            and sum(n.startswith('fe2o3/') for n in worker_map) == 820,
            'only thirteen actually qualified worker formatting paths; all209 final bodies retained')
    parent_rows = {'ferric/' + r['path']: r for r in proposal['files']}
    require(len(parent_rows) == len(proposal['files']) == 7
            and sum(r['before'] is None for r in parent_rows.values()) == 2
            and all(n.startswith('ferric/' + PARENT_REL + '/') and n.endswith('.rs')
                    and r['repository'] == 'ferric' for n, r in parent_rows.items()),
            'exact seven parent paths with two additions')
    expected = {n: r for n, r in old.items() if not n.startswith(WORKER_PREFIX)}
    expected.update(qualified_worker)
    for n, row in parent_rows.items():
        require(expected.get(n) == row['before'], 'actual parent preimage or absence ' + n)
        expected[n] = row['after']
    require(len(expected) == 1263 and not set(parent_rows) & set(worker_rows),
            '1263 Ferric bodies with disjoint parent and worker ownership')
    rows = dict(worker_rows, **parent_rows)
    require(all(ordinary(n) for n in old) and all(ordinary(n) for n in rows), 'ordinary closed source paths')
    names = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    names |= {label + suffix for label in base['tests'] for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == names and len(names) == 121, 'closed121 direct lineage bodies')
    require(all(pin(bodies[n]) == compact(base['raw'][n]) for n in names - set(fixed)),
            'actual current recipes and raw named outcomes')
    require(set(packed) == set(rows) | HELPERS | {'inputs/' + n for n in names},
            'closed144 original package bodies')
    for n in rows:
        require(pin(packed[n]) == expected[n], 'exact authored parent or actually qualified worker overlay ' + n)
    for n in HELPERS:
        expected[n] = pin(packed[n])
    require(len(expected) == 1266 and sum(v['bytes'] for v in expected.values()) <= 64 << 20,
            'bounded1266 source map')
    inputs = dict(schema='ferric-guarded-mlp-readiness40-scoped-warm-parent-cpu-input-v1',
        files=dict(sorted(expected.items())), lineage={n: pin(bodies[n]) for n in sorted(names)},
        parent_overlay=sorted(parent_rows), cache_manifest=CACHE_PIN, git_revision=base['git_revision'])
    return inputs, old


def pack():
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'parent-input-manifest.json'),
            'fresh package outputs')
    paths = {'run_cpu.py': P / 'parent_cpu.py', 'supervisor.py': Q / 'supervisor.py',
             'qualification_support.py': Q / 'qualification_support.py',
             'inputs/parent-proposal.json': PROPOSAL_ROOT / 'source-manifest.json',
             'inputs/parent-complete.json': Q / 'evidence/complete.json',
             'inputs/parent-sources.json': Q / 'evidence/sources-after.json',
             'inputs/worker-complete.json': QWORKER / 'evidence/complete.json',
             'inputs/worker-sources.json': QWORKER / 'evidence/sources-after.json',
             'inputs/consumer-proposal.json': FULL / 'scoped-worker-v1/source-manifest.json',
             'inputs/consumer-repair.json': FULL / 'scoped-worker-v2/source-manifest.json',
             'inputs/selector-proposal.json': FULL / 'scoped-worker-selector-v1/source-manifest.json',
             'inputs/selector-repair.json': FULL / 'scoped-worker-selector-v2/source-manifest.json',
             'inputs/runtime-proposal.json': FULL / 'scoped-currentness-routed-v1/source-manifest.json',
             'inputs/primitive-proposal.json': FULL / 'scoped-currentness-primitive-v1/source-manifest.json'}
    base = parse(read(paths['inputs/parent-complete.json']))
    proposal = parse(read(paths['inputs/parent-proposal.json']))
    for n in {'parent-lib-list.stdout', 'metadata.stdout'} | {label + suffix for label in base['tests']
            for suffix in ('.stdout', '.command.json')}:
        paths['inputs/' + n] = Q / 'evidence' / n
    for row in proposal['files']:
        n = 'ferric/' + row['path']
        paths[n] = PROPOSAL_ROOT / n
    for name in ('consumer-repair.json', 'selector-repair.json'):
        for row in parse(read(paths['inputs/' + name]))['files']:
            n = 'ferric/' + row['path']
            require(n not in paths, 'disjoint parent and worker overlays')
            paths[n] = QWORKER / n
    bodies = {n: read(path) for n, path in paths.items()}
    inputs, old = contract(bodies)
    canonical = {n: row for n, row in old.items() if not n.startswith(WORKER_PREFIX)}
    canonical.update({n: row for n, row in inputs['files'].items() if n.startswith(WORKER_PREFIX)})
    require(len(canonical) == 1261 and sum(n.startswith(WORKER_PREFIX) for n in canonical) == 209,
            'old1052 nonworker plus209 actually qualified worker preparation closure')
    for n, row in canonical.items():
        require(pin(read(F / n.removeprefix('ferric/'))) == row,
                'canonical predecessor and actual qualified worker body ' + n)
    for row in proposal['files']:
        if row['before'] is None:
            require(not os.path.lexists(F / row['path']), 'new parent source is absent')
    require(all(read(path) == bodies[n] for n, path in paths.items()), 'all packaging inputs unchanged')
    raw_input = encoded(inputs)
    bodies['input-manifest.json'] = raw_input
    require(len(bodies) == 145 and sum(map(len, bodies.values())) <= 32 << 20, 'bounded145-member package')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive_stream:
            for n, raw in sorted(bodies.items()):
                item = tarfile.TarInfo(n); item.size = len(raw); item.mode = 0o600
                archive_stream.addfile(item, io.BytesIO(raw))
    with (P / 'parent-input-manifest.json').open('xb') as stream:
        stream.write(raw_input)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))),
                         input=pin(raw_input), members=145, source_files=1266), sort_keys=True))


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
        require(len(members) == len({m.name for m in members}) == 145
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
            'all1266 staged source rows')
    require(all(pin(read(BASE / n)) == row for n, row in old.items()), 'immutable baseline body posthashes')
    require(read(E / BASENAME) == archive_raw, 'archive unchanged during staging')
    require(read(BASE / 'evidence/complete.json') == bodies['inputs/parent-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/parent-sources.json'],
            'immutable baseline receipt posthashes')
    receipt = dict(schema='ferric-guarded-mlp-readiness40-scoped-warm-parent-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=1266, ferric_files=1263,
        overlay_files=20, parent_format_paths=7, qualified_worker_files=209, qualified_worker_sources_preserved=True,
        helpers=3, lineage_files=121,
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
