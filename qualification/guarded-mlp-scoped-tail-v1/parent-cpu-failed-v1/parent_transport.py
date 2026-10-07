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
PROPOSAL_ROOT = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/proposals/guarded-mlp-model-interface-v228-v1/model-scoped-tail-v1/parent-v1')
FULL = PROPOSAL_ROOT.parent
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/guarded-mlp-full2303-bank-scoped-census-v1/parent-cpu-v1'
QWORKER = F / 'qualification/guarded-mlp-scoped-tail-v1/cpu-v1'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-full2303-bank-scoped-census-parent-cpu-v228-v1'
ROOT = E / 'guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-v228-v1'
OLD_ROOT = str(BASE)
WORKER_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-scoped-tail-cpu-v228-v1'
BASE_WORKER_COMPLETE = {"bytes":2475794,"sha256":"a7fcea3f09cfcda84ba556f60b5f1578ed378a45fc84bb81fbfa6ed31273cd64"}
WORKER_COMPLETE = {"bytes":2449865,"sha256":"a2e3d99f94b5b01223f42f3d8917c2cf6a1fc25f9227d4c341bc2b9a7cc9285c"}
BASE_WORKER_SOURCES = {"bytes":438675,"sha256":"5372e185504d11ed89d45cfb3d3e32672f93e8d1bbf3d04e44932b280300528c"}
WORKER_SOURCES = {"bytes":424416,"sha256":"a2a74ed023e9b2aac224621d37f328d097838c317c901a361c1496064e162ec4"}
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-readiness40-bank-scoped-census-tail-parent-input-v228-v1.tar.gz'
CONTROLLER_PIN = {"bytes":46865,"sha256":"db1f0c44ba309b99068dc42275f640c23004c3435e1fdf9e0440d2d82bdd9313"}
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13')
BASE_COMPLETE = {"bytes":4092678,"sha256":"6580942cb6b39d69b6fdac91a48ac6e0a25cd4e7648c4175152f3620ff7bacb6"}
BASE_SOURCES = {"bytes":525041,"sha256":"cf69d8bb33a1c9bf2fa0238e18da01ef70bba182a488388de378d1f611bc35fb"}
PROPOSAL_PINS = {"parent-proposal.json":{"bytes":23473,"sha256":"840417429d3357155672cc5d096d592329461dd742d6d2476580759495dd116a"},"worker-proposal.json":{"bytes":27229,"sha256":"96463bf9a70d6020e28c2d44260f589c38e7f61229f8c8e77f28242f1893ca71"},"runtime-proposal.json":{"bytes":10858,"sha256":"71da4afd43eb9f7de8fa1cac47cf8e9b2f0ececc86cfb98877b94678d500df83"},"interface.md":{"bytes":24881,"sha256":"749fd470a27cf7630f292b04c5464567e4e7d398ee7855b1fbd152824421a8e4"}}
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
        'worker-sources.json': WORKER_SOURCES,
        'baseline-worker-sources.json': BASE_WORKER_SOURCES})
    bodies = {n.removeprefix('inputs/'): raw for n, raw in packed.items() if n.startswith('inputs/')}
    require(all(pin(bodies[n]) == v for n, v in fixed.items()), 'literal actual parent/worker/source identities')
    base = parse(bodies['parent-complete.json'])
    old_map = parse(bodies['parent-sources.json'])
    proposal = parse(bodies['parent-proposal.json'])
    worker = parse(bodies['worker-complete.json'])
    worker_map = parse(bodies['worker-sources.json'])
    worker_proposal = parse(bodies['worker-proposal.json'])
    runtime_proposal = parse(bodies['runtime-proposal.json'])
    base_worker_map = parse(bodies['baseline-worker-sources.json'])
    require(base['schema'] == 'ferric-guarded-mlp-full2303-bank-scoped-census-parent-cpu-v1'
            and len(old_map) == 1287 and len(base['phases']) == 68 and len(base['tests']) == 58
            and len(base['inventory']) == 1014 and sum(v['passed'] for v in base['tests'].values()) == 549
            and all(v['failed'] == v['ignored'] == 0 for v in base['tests'].values())
            and base['full2303_bank_scoped_census_parent_source_added'] is True
            and base['full2303_bank_scoped_census_policy_prepublication_checked'] is True
            and base['full2303_scoped_warm_parent_source_added'] is True
            and base['full2303_scoped_policy_prepublication_checked'] is True
            and base['readiness40_scoped_warm_parent_source_added'] is True
            and base['scoped_warm_host_timing_parent_source_added'] is True
            and base['readiness40_bank_scoped_warm_parent_source_added'] is True
            and base['allocation_preflights_changed'] is True
            and base['readiness40_bank_scoped_census_parent_source_added'] is True
            and base['qualified_worker_sources_preserved'] is True
            and base['worker_qualification'] == BASE_WORKER_COMPLETE
            and base['worker_source_manifest'] == BASE_WORKER_SOURCES
            and base['parent_host_timing_rows'] == 40 and base['parent_host_timing_disjoint_spans'] == 124
            and len(base['artifacts']) == 7, 'actual549 selected Full bank-census parent baseline')
    for receipt, source_map, expected_pin, phase_count in (
            (base, old_map, BASE_SOURCES, 68), (worker, worker_map, WORKER_SOURCES, 27)):
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
    require(worker['schema'] == 'ferric-guarded-mlp-scoped-tail-cpu-v1'
            and worker['source_generation'] == 'scoped-tail-coupled-v1'
            and len(worker_map) == 1059 and len(worker['artifacts']) == 11
            and (worker['tests']['worker-tests']['passed'], worker['tests']['worker-tests']['failed'],
                 worker['tests']['worker-tests']['ignored']) == (820, 0, 4)
            and (worker['tests']['kfd-tests']['passed'], worker['tests']['kfd-tests']['failed'],
                 worker['tests']['kfd-tests']['ignored']) == (1180, 0, 8)
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and worker['runtime_source_changed'] is True
            and worker['scoped_tail_runtime_source_added'] is True
            and worker['readiness40_bank_scoped_census_tail_source_added'] is True
            and worker['scoped_bank_rearm_runtime_source_added'] is True
            and worker['readiness40_bank_scoped_warm_source_added'] is True
            and worker['full2303_scoped_warm_source_added'] is True
            and worker['inherited_scoped_currentness_runtime_preserved'] is True
            and worker['scoped_capacity_census_runtime_source_added'] is True
            and worker['readiness40_bank_scoped_census_source_added'] is True
            and worker['allocation_preflights_changed'] is True
            and worker['full2303_bank_scoped_census_source_added'] is True
            and worker['inherited_readiness40_bank_scoped_census_preserved'] is True
            and worker['full2303_deadline_changed'] is False
            and all(worker[k] is False for k in ('full2303_scoped_warm_native_execution',
                'readiness40_bank_scoped_warm_native_execution', 'readiness40_bank_scoped_census_native_execution',
                'full2303_bank_scoped_census_native_execution',
                'readiness40_bank_scoped_census_tail_native_execution',
                'full2303_launch_admitted',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'numerical_acceptance', 'performance_claim')),
            'actual Tail runtime and worker qualification with tested real executable')
    require(compact(worker['readset']['worker-proposal.json']) == fixed['worker-proposal.json']
            and compact(worker['readset']['runtime-proposal.json']) == fixed['runtime-proposal.json']
            and compact(worker['readset']['baseline-complete.json']) == BASE_WORKER_COMPLETE
            and compact(worker['readset']['baseline-sources.json']) == BASE_WORKER_SOURCES,
            'actual coupled direct proposal and predecessor readset')
    require(proposal['schema'] == 'ferric-bank-scoped-census-tail-readiness40-parent-source-v1'
            and proposal['source_generation'] == 'bank-scoped-census-tail-parent-v1'
            and compact(proposal['base']['parent_receipt']) == BASE_COMPLETE
            and compact(proposal['base']['parent_sources']) == BASE_SOURCES
            and compact(proposal['base']['worker_receipt']) == BASE_WORKER_COMPLETE
            and compact(proposal['base']['worker_sources']) == BASE_WORKER_SOURCES
            and all(compact(proposal['requires'][key]) == fixed[name] for key, name in
                (('worker_proposal', 'worker-proposal.json'), ('runtime_proposal', 'runtime-proposal.json'),
                 ('interface', 'interface.md')))
            and all(proposal[k] is False for k in ('canonical_changed', 'compiled', 'tested',
                'native_execution', 'default_routes_changed', 'default_group_policy_changed',
                'temporal_equivalent_to_full', 'performance_claim', 'numerical_acceptance')),
            'exact timed-only Readiness40 Tail parent proposal')
    require(worker_proposal['schema'] == 'ferric-readiness40-bank-scoped-census-tail-worker-source-v4'
            and len(worker_proposal['files']) == 13
            and worker_proposal['source_generation'] == 'readiness40-bank-scoped-census-tail-worker-v4'
            and worker_proposal['new_explicit_readiness40_tail_route'] is True
            and worker_proposal['first_two_forwards_unchanged'] is True
            and compact(worker_proposal['base']['worker_receipt']) == BASE_WORKER_COMPLETE
            and compact(worker_proposal['base']['worker_sources']) == BASE_WORKER_SOURCES
            and compact(worker_proposal['requires']['runtime']) == fixed['runtime-proposal.json']
            and compact(worker_proposal['requires']['interface']) == fixed['interface.md']
            and worker_proposal['conditional_qualification'] == {"worker_files":228,"passed":820,"ignored":4,"inventory":824,"new_library_tests":20,"new_readiness_cli_tests":1,"targets":{"library":{"passed":793,"ignored":4},"binary":{"passed":0,"ignored":0},"readiness_cli":{"passed":11,"ignored":0},"wire":{"passed":16,"ignored":0}},"coupled_sources":1059,"runtime_sources":827,"runtime_passed":1180,"runtime_ignored":8,"helper_sources":4,"phases":27,"elf_products":11,"qualified":False}
            and all(worker_proposal[k] is False for k in ('compiled', 'tested', 'canonical_changed',
                'native_execution', 'runtime_changed', 'default_group_policy_changed',
                'existing_selectors_changed', 'full2303_route_changed', 'allocation_preflights_changed',
                'currentness_temporal_equivalence_claim', 'production_authority',
                'performance_claim', 'numerical_acceptance', 'full_launch_admitted')),
            'qualified Tail worker source lineage')
    require(runtime_proposal['schema'] == 'ferric-scoped-tail-runtime-source-v1'
            and runtime_proposal['status'] == 'source-only-uncompiled-unexecuted'
            and compact(runtime_proposal['interface']) == fixed['interface.md']
            and runtime_proposal['baseline']['runtime_map_rows'] == 825
            and runtime_proposal['conditional']['runtime_map_rows'] == 827
            and runtime_proposal['conditional']['runtime_passed'] == 1180
            and runtime_proposal['conditional']['runtime_ignored'] == 8
            and all(value is False for value in runtime_proposal['claims'].values()),
            'exact separately qualified closed tail runtime source')
    old_runtime = {n: compact(r) for n, r in base_worker_map.items() if n.startswith('fe2o3/')}
    runtime_rows = {'fe2o3/' + r['path']: r for r in runtime_proposal['files']}
    require(len(old_runtime) == 825 and len(runtime_rows) == len(runtime_proposal['files']) == 6
            and sum(r['before'] is None for r in runtime_rows.values()) == 2
            and all(n.startswith('fe2o3/crates/fe2o3-kfd/src/') and n.endswith('.rs')
                    and r['repository'] == 'fe2o3' for n, r in runtime_rows.items()),
            'six closed runtime paths with two additions')
    pre_runtime = dict(old_runtime)
    for n, row in runtime_rows.items():
        require(pre_runtime.get(n) == row['before'], 'actual runtime preimage or absence ' + n)
        pre_runtime[n] = row['after']
    qualified_runtime = {n: compact(r) for n, r in worker_map.items() if n.startswith('fe2o3/')}
    require(len(qualified_runtime) == 827 and set(qualified_runtime) == set(pre_runtime)
            and {n: compact(r) for n, r in worker['preformat_sources'].items()
                 if n.startswith('fe2o3/')} == pre_runtime
            and all(qualified_runtime[n] == r for n, r in pre_runtime.items() if n not in runtime_rows)
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n for n in qualified_runtime),
            'only six declared runtime format paths within actual coupled qualification')
    worker_rows = {'ferric/' + r['path']: r for r in worker_proposal['files']}
    require(len(worker_rows) == 13 and sum(r['before'] is None for r in worker_rows.values()) == 4
            and all(n.startswith(WORKER_PREFIX) and n.endswith('.rs') and r['repository'] == 'ferric'
                    for n, r in worker_rows.items()), 'thirteen closed worker paths with four additions')
    require(all(worker_rows.get('ferric/' + r['path']) == {k: r[k] for k in ('repository', 'path', 'before', 'after')}
                for r in proposal['requires']['imported_policy_rows'])
            and len(proposal['requires']['imported_policy_rows']) == 2,
            'exact imported worker policy and test source rows')
    old = {n: compact(v) for n, v in old_map.items() if n.startswith('ferric/')}
    require(len(old) == 1284 and sum(n.startswith(WORKER_PREFIX) for n in old) == 224
            and all(old_map[n]['path'] == OLD_ROOT + '/' + n for n in old),
            'one actual1284 parent source closure with224 worker bodies')
    pre_worker = {n: r for n, r in old.items() if n.startswith(WORKER_PREFIX)}
    require(pre_worker == {n: compact(r) for n, r in base_worker_map.items()
                           if n.startswith(WORKER_PREFIX)},
            'all224 actual parent worker bodies equal the coupled baseline')
    for n, row in worker_rows.items():
        require(pre_worker.get(n) == row['before'], 'actual worker preimage or absence ' + n)
        pre_worker[n] = row['after']
    qualified_worker = {n: compact(r) for n, r in worker_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(qualified_worker) == 228 and set(qualified_worker) == set(pre_worker)
            and {n: compact(r) for n, r in worker['preformat_sources'].items() if n.startswith(WORKER_PREFIX)}
                == pre_worker
            and all(qualified_worker[n] == r for n, r in pre_worker.items() if n not in worker_rows)
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n for n in qualified_worker)
            and sum(n.startswith('fe2o3/') for n in worker_map) == 827,
            'only thirteen qualified worker format paths; all228 final worker bodies preserved')
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
    require(len(expected) == 1290 and not set(parent_rows) & set(worker_rows),
            '1290 Ferric bodies with disjoint ownership')
    rows = dict(worker_rows, **parent_rows)
    require(all(ordinary(n) for n in old) and all(ordinary(n) for n in rows), 'ordinary closed source paths')
    names = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    names |= {label + suffix for label in base['tests'] for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == names and len(names) == 127, 'closed127 direct lineage bodies')
    require(all(pin(bodies[n]) == compact(base['raw'][n]) for n in names - set(fixed)),
            'actual current recipes and raw named outcomes')
    require(set(packed) == set(rows) | HELPERS | {'inputs/' + n for n in names},
            'closed150 original package bodies')
    for n in rows:
        require(pin(packed[n]) == expected[n], 'exact authored parent or actually qualified worker overlay ' + n)
    for n in HELPERS:
        expected[n] = pin(packed[n])
    require(len(expected) == 1293 and sum(v['bytes'] for v in expected.values()) <= 64 << 20,
            'bounded1293 source map')
    inputs = dict(schema='ferric-guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-input-v1',
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
             'inputs/runtime-proposal.json': FULL / 'runtime-v1/source-manifest.json',
             'inputs/interface.md': FULL / 'INTERFACE.md',
             'inputs/baseline-worker-sources.json': F / 'qualification/guarded-mlp-full2303-bank-scoped-census-v1/cpu-v1/evidence/sources-after.json'}
    base = parse(read(paths['inputs/parent-complete.json']))
    proposal = parse(read(paths['inputs/parent-proposal.json']))
    for n in {'parent-lib-list.stdout', 'metadata.stdout'} | {label + suffix for label in base['tests']
            for suffix in ('.stdout', '.command.json')}:
        paths['inputs/' + n] = Q / 'evidence' / n
    for row in proposal['files']:
        n = 'ferric/' + row['path']
        paths[n] = PROPOSAL_ROOT / n
    for name in ('worker-proposal.json',):
        for row in parse(read(paths['inputs/' + name]))['files']:
            n = 'ferric/' + row['path']
            require(n not in paths, 'disjoint parent and worker overlays')
            paths[n] = QWORKER / n
    bodies = {n: read(path) for n, path in paths.items()}
    inputs, old = contract(bodies)
    canonical = {n: row for n, row in old.items() if not n.startswith(WORKER_PREFIX)}
    canonical.update({n: row for n, row in inputs['files'].items() if n.startswith(WORKER_PREFIX)})
    require(len(canonical) == 1288 and sum(n.startswith(WORKER_PREFIX) for n in canonical) == 228,
            'old1060 nonworker plus228 actually qualified worker preparation closure')
    for n, row in canonical.items():
        require(pin(read(F / n.removeprefix('ferric/'))) == row,
                'canonical predecessor and actual qualified worker body ' + n)
    for row in proposal['files']:
        if row['before'] is None:
            require(not os.path.lexists(F / row['path']), 'new parent source is absent')
    require(all(read(path) == bodies[n] for n, path in paths.items()), 'all packaging inputs unchanged')
    raw_input = encoded(inputs)
    bodies['input-manifest.json'] = raw_input
    require(len(bodies) == 151 and sum(map(len, bodies.values())) <= 32 << 20, 'bounded151-member package')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive_stream:
            for n, raw in sorted(bodies.items()):
                item = tarfile.TarInfo(n); item.size = len(raw); item.mode = 0o600
                archive_stream.addfile(item, io.BytesIO(raw))
    with (P / 'parent-input-manifest.json').open('xb') as stream:
        stream.write(raw_input)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))),
                         input=pin(raw_input), members=151, source_files=1293), sort_keys=True))


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
        require(len(members) == len({m.name for m in members}) == 151
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
    require(all(pin(read(BASE / n)) == row for n, row in old.items()), 'all1284 actual parent prehashes')
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
            'all1293 staged source rows')
    require(all(pin(read(BASE / n)) == row for n, row in old.items()), 'immutable baseline body posthashes')
    require(read(E / BASENAME) == archive_raw, 'archive unchanged during staging')
    require(read(BASE / 'evidence/complete.json') == bodies['inputs/parent-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/parent-sources.json'],
            'immutable baseline receipt posthashes')
    receipt = dict(schema='ferric-guarded-mlp-readiness40-bank-scoped-census-tail-parent-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=1293, ferric_files=1290,
        overlay_files=20, parent_format_paths=7, qualified_worker_files=228, qualified_worker_sources_preserved=True,
        helpers=3, lineage_files=127,
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
