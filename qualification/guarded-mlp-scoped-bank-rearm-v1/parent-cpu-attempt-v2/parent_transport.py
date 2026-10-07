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
PROPOSAL_ROOT = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/proposals/guarded-mlp-model-interface-v228-v1/model-scoped-bank-rearm-v1/parent-v2')
FULL = PROPOSAL_ROOT.parent
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/guarded-mlp-full2303-scoped-currentness-v1/parent-cpu-v1'
QWORKER = F / 'qualification/guarded-mlp-scoped-bank-rearm-v1/cpu-attempt-v2'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-full2303-scoped-warm-parent-cpu-v228-v2'
ROOT = E / 'guarded-mlp-readiness40-bank-scoped-warm-parent-cpu-v228-v2'
OLD_ROOT = str(BASE)
WORKER_ROOT = str(E / 'guarded-mlp-scoped-bank-rearm-cpu-v228-v2')
BASE_WORKER_COMPLETE = {'bytes': 2402339, 'sha256': 'a44623c8586dd5ba00eb55f0d549eb11b01c7c53b96f363749246eef1037d2a1'}
WORKER_COMPLETE = {'bytes': 2395639, 'sha256': 'e522aafde4204c24be8fac9b1ecf8d49f8e3389819ad8cfda765f2145647c432'}
BASE_WORKER_SOURCES = {'bytes': 430941, 'sha256': 'c3dbb35ea085d4166e4b5ee58b29732ebfe2f14c63599f693ceefdc5a8959d37'}
WORKER_SOURCES = {'bytes': 423235, 'sha256': '73e3ad534a3d48bddd717780215142dd588f4a776dd25f003305854f70d7c285'}
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-readiness40-bank-scoped-warm-parent-input-v228-v2.tar.gz'
CONTROLLER_PIN = {'bytes': 44921, 'sha256': 'fb2afc34876ae1a775e2e77198f5fc17c610a14995ff010700bfa8edc4fc5861'}
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13')
BASE_COMPLETE = {'bytes': 3997921, 'sha256': 'c145839bb36b2dc437590bb1bcdc6e4977f0fabc74c4889cfcb5345b57f5b6d8'}
BASE_SOURCES = {'bytes': 508077, 'sha256': '50e41ee555b648ccf93aec5aae95b1b1040b936ce426d56cd620dd524106f5b9'}
PROPOSAL_PINS = {'parent-proposal.json': {'bytes': 19134, 'sha256': '072568d76e5055b934aa51bf50c7d8d1bad4b0d10fc19441df23de15fd8eb643'}, 'worker-proposal.json': {'bytes': 15970, 'sha256': 'bb28743219931b14202c725650ff45841e3857a3cc5112390896dfc163c6626c'}, 'runtime-proposal.json': {'bytes': 10029, 'sha256': '2798cf16b04d0c77c5a1df155513a99c226907c1db46558e4d882da9ddf2e7ff'}, 'worker-repair.json': {'bytes': 17360, 'sha256': '70f9d4e91c0c2d22bdc647f6cac67aa889fd7e9972c21050caa19cb5d4845b53'}}
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
    worker_proposal = parse(bodies['worker-proposal.json'])
    require(base['schema'] == 'ferric-guarded-mlp-full2303-scoped-warm-parent-cpu-v1'
            and len(old_map) == 1270 and len(base['phases']) == 66 and len(base['tests']) == 56
            and len(base['inventory']) == 972 and sum(v['passed'] for v in base['tests'].values()) == 504
            and all(v['failed'] == v['ignored'] == 0 for v in base['tests'].values())
            and base['full2303_scoped_warm_parent_source_added'] is True
            and base['full2303_scoped_policy_prepublication_checked'] is True
            and base['readiness40_scoped_warm_parent_source_added'] is True
            and base['scoped_warm_host_timing_parent_source_added'] is True
            and base['qualified_worker_sources_preserved'] is True
            and base['worker_qualification'] == BASE_WORKER_COMPLETE
            and base['worker_source_manifest'] == BASE_WORKER_SOURCES
            and base['parent_host_timing_rows'] == 40 and base['parent_host_timing_disjoint_spans'] == 124
            and len(base['artifacts']) == 7, 'actual504 selected Full-scoped parent baseline')
    for receipt, source_map, expected_pin, phase_count in (
            (base, old_map, BASE_SOURCES, 66), (worker, worker_map, WORKER_SOURCES, 26)):
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
    require(worker['schema'] == 'ferric-guarded-mlp-scoped-bank-rearm-cpu-v1'
            and worker['source_generation'] == 'scoped-bank-rearm-coupled-v2'
            and len(worker_map) == 1042 and len(worker['artifacts']) == 11
            and (worker['tests']['worker-tests']['passed'], worker['tests']['worker-tests']['failed'],
                 worker['tests']['worker-tests']['ignored']) == (760, 0, 4)
            and (worker['tests']['kfd-tests']['passed'], worker['tests']['kfd-tests']['failed'],
                 worker['tests']['kfd-tests']['ignored']) == (1152, 0, 8)
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and worker['runtime_source_changed'] is True
            and worker['scoped_bank_rearm_runtime_source_added'] is True
            and worker['readiness40_bank_scoped_warm_source_added'] is True
            and worker['full2303_scoped_warm_source_added'] is True
            and worker['inherited_scoped_currentness_runtime_preserved'] is True
            and all(worker[k] is False for k in ('full2303_scoped_warm_native_execution',
                'readiness40_bank_scoped_warm_native_execution', 'allocation_preflights_changed',
                'full2303_launch_admitted',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'numerical_acceptance', 'performance_claim')),
            'actual coupled bank runtime and worker with tested real executable')
    require(compact(worker['readset']['worker-proposal.json']) == fixed['worker-proposal.json']
            and compact(worker['readset']['runtime-proposal.json']) == fixed['runtime-proposal.json']
            and compact(worker['readset']['worker-repair.json']) == fixed['worker-repair.json']
            and compact(worker['readset']['baseline-complete.json']) == BASE_WORKER_COMPLETE
            and compact(worker['readset']['baseline-sources.json']) == BASE_WORKER_SOURCES,
            'actual coupled direct proposal and predecessor readset')
    require(proposal['schema'] == 'ferric-bank-scoped-warm-readiness40-parent-source-v1'
            and proposal['source_generation'] == 'bank-scoped-warm-parent-v2-full-parent-rebase'
            and compact(proposal['base']['parent_receipt']) == BASE_COMPLETE
            and compact(proposal['base']['parent_sources']) == BASE_SOURCES
            and compact(proposal['requires']['worker_proposal']) == fixed['worker-proposal.json']
            and compact(proposal['requires']['runtime_proposal']) == fixed['runtime-proposal.json']
            and all(proposal[k] is False for k in ('canonical_changed', 'compiled', 'tested',
                'native_execution', 'default_routes_changed', 'default_group_policy_changed',
                'temporal_equivalent_to_full', 'performance_claim', 'numerical_acceptance')),
            'exact rebased bank-scoped parent proposal')
    original_worker = worker_proposal
    repaired_raw = bodies['worker-repair.json']
    worker_proposal = parse(repaired_raw)
    repair = worker_proposal['repair']
    repair_path = "adapters/tp-peer-finite-engineering-worker-v1/src/native_guarded_mlp_readiness_cli_v1_tests.rs"
    require(worker_proposal['source_generation'] == 'bank-scoped-warm-worker-v2-local-unsafe-test-allowance'
            and {k: worker_proposal['predecessor_manifest'][k] for k in ('bytes', 'sha256')}
                == dict(bytes=15970, sha256='bb28743219931b14202c725650ff45841e3857a3cc5112390896dfc163c6626c')
            and {k: v for k, v in original_worker.items() if k not in ('files', 'readme', 'patch')}
                == {k: v for k, v in worker_proposal.items() if k not in
                    ('files', 'readme', 'patch', 'source_generation', 'predecessor_manifest', 'repair')}
            and repair['path'] == repair_path
            and repair['before'] == {"bytes":19939,"sha256":"27c422af23b93adc05c3a8619015c9a0af0506e9f20d2b2d8ff319e85e460e33"}
            and repair['after'] == {"bytes":20118,"sha256":"8b97a5446abdab7071d02587cbe9fcea071ff8c383f5e435ca202fdc8fb56545"}
            and all(repair[key] is False for key in ('production_changed', 'test_names_changed',
                'assertions_changed', 'crate_or_module_lint_changed', 'observed_worker_attempt_passed')),
            'original worker proposal retained with sole reviewed test-local repair')
    old_rows = {row['path']: row for row in original_worker['files']}
    new_rows = {row['path']: row for row in worker_proposal['files']}
    require(len(old_rows) == len(new_rows) == len(original_worker['files']) == len(worker_proposal['files']) == 11
            and set(old_rows) == set(new_rows)
            and all(new_rows[name] == (dict(row, after=repair['after']) if name == repair_path else row)
                    for name, row in old_rows.items())
            and old_rows[repair_path]['after'] == repair['before'],
            'eleven unchanged preimages and exact one-test postimage transition')

    require(worker_proposal['schema'] == 'ferric-readiness40-bank-scoped-warm-worker-source-v2'
            and len(worker_proposal['files']) == 11
            and compact(worker_proposal['base']['worker_receipt']) == BASE_WORKER_COMPLETE
            and compact(worker_proposal['base']['worker_sources']) == BASE_WORKER_SOURCES
            and worker_proposal['conditional'] == dict(worker_files=216, worker_passed=760,
                worker_ignored=4, worker_inventory=764, coupled_runtime_files=822,
                coupled_source_files_including_four_helpers=1042)
            and all(worker_proposal[k] is False for k in ('compiled', 'tested', 'canonical_changed',
                'native_execution', 'runtime_changed', 'default_group_policy_changed',
                'currentness_temporal_equivalence_claim', 'performance_claim',
                'numerical_acceptance', 'full_launch_admitted', 'allocation_preflights_changed')),
            'qualified worker source lineage')
    runtime_proposal = parse(bodies['runtime-proposal.json'])
    require(runtime_proposal['schema'] == 'ferric-scoped-bank-rearm-runtime-source-v1'
            and compact(worker_proposal['requires']['runtime_manifest']) == fixed['runtime-proposal.json']
            and runtime_proposal['conditional']['runtime_map_rows'] == 822
            and runtime_proposal['conditional']['runtime_passed'] == 1152
            and runtime_proposal['scope']['ordinary_rearm_unchanged'] is True
            and runtime_proposal['scope']['temporal_equivalence_claim'] is False,
            'same reviewed bank runtime is qualified by the coupled worker result')
    worker_rows = {'ferric/' + r['path']: r for r in worker_proposal['files']}
    require(len(worker_rows) == 11 and sum(r['before'] is None for r in worker_rows.values()) == 4
            and all(n.startswith(WORKER_PREFIX) and n.endswith('.rs') and r['repository'] == 'ferric'
                    for n, r in worker_rows.items()), 'eleven closed worker paths with four additions')
    require(all(worker_rows.get('ferric/' + r['path']) == {k: r[k] for k in ('repository', 'path', 'before', 'after')}
                for r in proposal['requires']['imported_policy_rows'])
            and len(proposal['requires']['imported_policy_rows']) == 2,
            'exact imported worker policy and test source rows')
    old = {n: compact(v) for n, v in old_map.items() if n.startswith('ferric/')}
    require(len(old) == 1267 and sum(n.startswith(WORKER_PREFIX) for n in old) == 212
            and all(old_map[n]['path'] == OLD_ROOT + '/' + n for n in old),
            'one actual1267 parent source closure with212 worker bodies')
    pre_worker = {n: r for n, r in old.items() if n.startswith(WORKER_PREFIX)}
    for n, row in worker_rows.items():
        require(pre_worker.get(n) == row['before'], 'actual worker preimage or absence ' + n)
        pre_worker[n] = row['after']
    qualified_worker = {n: compact(r) for n, r in worker_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(qualified_worker) == 216 and set(qualified_worker) == set(pre_worker)
            and {n: compact(r) for n, r in worker['preformat_sources'].items() if n.startswith(WORKER_PREFIX)}
                == pre_worker
            and all(qualified_worker[n] == r for n, r in pre_worker.items() if n not in worker_rows)
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n for n in qualified_worker)
            and sum(n.startswith('fe2o3/') for n in worker_map) == 822,
            'only eleven qualified worker format paths; all216 final worker bodies preserved')
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
    require(len(expected) == 1273 and not set(parent_rows) & set(worker_rows),
            '1273 Ferric bodies with disjoint ownership')
    rows = dict(worker_rows, **parent_rows)
    require(all(ordinary(n) for n in old) and all(ordinary(n) for n in rows), 'ordinary closed source paths')
    names = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    names |= {label + suffix for label in base['tests'] for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == names and len(names) == 122, 'closed122 direct lineage bodies')
    require(all(pin(bodies[n]) == compact(base['raw'][n]) for n in names - set(fixed)),
            'actual current recipes and raw named outcomes')
    require(set(packed) == set(rows) | HELPERS | {'inputs/' + n for n in names},
            'closed143 original package bodies')
    for n in rows:
        require(pin(packed[n]) == expected[n], 'exact authored parent or actually qualified worker overlay ' + n)
    for n in HELPERS:
        expected[n] = pin(packed[n])
    require(len(expected) == 1276 and sum(v['bytes'] for v in expected.values()) <= 64 << 20,
            'bounded1276 source map')
    inputs = dict(schema='ferric-guarded-mlp-readiness40-bank-scoped-warm-parent-cpu-input-v1',
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
             'inputs/worker-proposal.json': FULL / 'worker-v1/source-manifest.json',
             'inputs/worker-repair.json': FULL / 'worker-v2/source-manifest.json',
             'inputs/runtime-proposal.json': FULL / 'runtime-v1/source-manifest.json'}
    base = parse(read(paths['inputs/parent-complete.json']))
    proposal = parse(read(paths['inputs/parent-proposal.json']))
    for n in {'parent-lib-list.stdout', 'metadata.stdout'} | {label + suffix for label in base['tests']
            for suffix in ('.stdout', '.command.json')}:
        paths['inputs/' + n] = Q / 'evidence' / n
    for row in proposal['files']:
        n = 'ferric/' + row['path']
        paths[n] = PROPOSAL_ROOT / n
    for name in ('worker-repair.json',):
        for row in parse(read(paths['inputs/' + name]))['files']:
            n = 'ferric/' + row['path']
            require(n not in paths, 'disjoint parent and worker overlays')
            paths[n] = QWORKER / n
    bodies = {n: read(path) for n, path in paths.items()}
    inputs, old = contract(bodies)
    canonical = {n: row for n, row in old.items() if not n.startswith(WORKER_PREFIX)}
    canonical.update({n: row for n, row in inputs['files'].items() if n.startswith(WORKER_PREFIX)})
    require(len(canonical) == 1271 and sum(n.startswith(WORKER_PREFIX) for n in canonical) == 216,
            'old1055 nonworker plus216 actually qualified worker preparation closure')
    for n, row in canonical.items():
        require(pin(read(F / n.removeprefix('ferric/'))) == row,
                'canonical predecessor and actual qualified worker body ' + n)
    for row in proposal['files']:
        if row['before'] is None:
            require(not os.path.lexists(F / row['path']), 'new parent source is absent')
    require(all(read(path) == bodies[n] for n, path in paths.items()), 'all packaging inputs unchanged')
    raw_input = encoded(inputs)
    bodies['input-manifest.json'] = raw_input
    require(len(bodies) == 144 and sum(map(len, bodies.values())) <= 32 << 20, 'bounded144-member package')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive_stream:
            for n, raw in sorted(bodies.items()):
                item = tarfile.TarInfo(n); item.size = len(raw); item.mode = 0o600
                archive_stream.addfile(item, io.BytesIO(raw))
    with (P / 'parent-input-manifest.json').open('xb') as stream:
        stream.write(raw_input)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))),
                         input=pin(raw_input), members=144, source_files=1276), sort_keys=True))


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
        require(len(members) == len({m.name for m in members}) == 144
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
    require(all(pin(read(BASE / n)) == row for n, row in old.items()), 'all1267 actual parent prehashes')
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
            'all1276 staged source rows')
    require(all(pin(read(BASE / n)) == row for n, row in old.items()), 'immutable baseline body posthashes')
    require(read(E / BASENAME) == archive_raw, 'archive unchanged during staging')
    require(read(BASE / 'evidence/complete.json') == bodies['inputs/parent-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/parent-sources.json'],
            'immutable baseline receipt posthashes')
    receipt = dict(schema='ferric-guarded-mlp-readiness40-bank-scoped-warm-parent-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=1276, ferric_files=1273,
        overlay_files=18, parent_format_paths=7, qualified_worker_files=216, qualified_worker_sources_preserved=True,
        helpers=3, lineage_files=122,
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
