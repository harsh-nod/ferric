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
PROPOSAL_ROOT = Path("/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/proposals/guarded-mlp-model-interface-v228-v1/model-full-scoped-tail-v1/parent-v1")
FULL = PROPOSAL_ROOT.parent
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/guarded-mlp-scoped-tail-v1/parent-cpu-v2'
QWORKER = F / 'qualification/guarded-mlp-full2303-scoped-tail-v1/cpu-v1'
QBASEWORKER = F / 'qualification/guarded-mlp-scoped-tail-v1/cpu-attempt-v2'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-v228-v2'
ROOT = E / 'guarded-mlp-full2303-scoped-tail-parent-cpu-v228-v1'
OLD_ROOT = str(BASE)
WORKER_ROOT = "/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-full2303-scoped-tail-cpu-v228-v1"
BASE_WORKER_COMPLETE = {"bytes":2448697,"sha256":"3c4fe3a6c4942b20ec61fd7ea5c3409526e2167c43d24a62fb8660c34b0e713e"}
WORKER_COMPLETE = {"bytes":2498847,"sha256":"f25bd6d9f582a4e632c05e385961c260c0c95d2a119bac5af4927af6dc3b61ed"}
BASE_WORKER_SOURCES = {"bytes":424416,"sha256":"5de4929b2832eda55a53f99945fc4b64a9fbda28b4d4f8bef07f2e8d5e000d6a"}
WORKER_SOURCES = {"bytes":435848,"sha256":"eda7f5f696a5b93d048923cd5f8bd83b51a5ca9cb77efab8322da97f4286e928"}
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-full2303-scoped-tail-parent-input-v228-v1.tar.gz'
CONTROLLER_PIN = {"bytes":47115,"sha256":"52e5d03a2f31ab6944e58fffd3ae92943ef7162f2e8529e170c4e7e89723dd92"}
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13')
BASE_COMPLETE = {"bytes":4163125,"sha256":"a59c39cfa2be0ec11d0ffa6a352576814917243627ce9a73aca1d4f6501266e9"}
BASE_SOURCES = {"bytes":538263,"sha256":"f8b92a7b220c9733e75aa7745498d915217241369ea2a003edfbdda55f52d4b2"}
PROPOSAL_PINS = {"parent-proposal.json":{"bytes":18956,"sha256":"721425e289ae2e26fc9ed178cfa8f5d6f84f645ea9094cfedfbf3a232e169f4e"},"worker-proposal.json":{"bytes":23469,"sha256":"e53136db6e443f768f92d374395cce3867a160a90994d768555573606e8e8e9a"},"interface.md":{"bytes":4970,"sha256":"0f83da4e18dce6beacadf565035f5e6b5ed7c32d5f44303826db93ff06fbd589"}}
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
        'baseline-worker-sources.json': BASE_WORKER_SOURCES,
        'baseline-worker-complete.json': BASE_WORKER_COMPLETE})
    bodies = {n.removeprefix('inputs/'): raw for n, raw in packed.items() if n.startswith('inputs/')}
    require(all(pin(bodies[n]) == v for n, v in fixed.items()), 'literal actual parent/worker/source identities')
    base = parse(bodies['parent-complete.json'])
    old_map = parse(bodies['parent-sources.json'])
    proposal = parse(bodies['parent-proposal.json'])
    worker = parse(bodies['worker-complete.json'])
    worker_map = parse(bodies['worker-sources.json'])
    worker_proposal = parse(bodies['worker-proposal.json'])
    base_worker_map = parse(bodies['baseline-worker-sources.json'])
    baseline_worker = parse(bodies['baseline-worker-complete.json'])
    require(base['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-v1'
            and base['source_generation'] == 'scoped-tail-parent-v2'
            and base['shared_test_fixture_repaired'] is True
            and len(old_map) == 1293 and len(base['phases']) == 68 and len(base['tests']) == 58
            and len(base['inventory']) == 1032 and sum(v['passed'] for v in base['tests'].values()) == 568
            and all(v['failed'] == v['ignored'] == 0 for v in base['tests'].values())
            and base['full2303_scoped_warm_parent_source_added'] is True
            and base['full2303_scoped_policy_prepublication_checked'] is True
            and base['readiness40_scoped_warm_parent_source_added'] is True
            and base['scoped_warm_host_timing_parent_source_added'] is True
            and base['readiness40_bank_scoped_warm_parent_source_added'] is True
            and base['allocation_preflights_changed'] is True
            and base['readiness40_bank_scoped_census_parent_source_added'] is True
            and base['readiness40_bank_scoped_census_tail_parent_source_added'] is True
            and base['readiness40_bank_scoped_census_tail_policy_prepublication_checked'] is True
            and base['full2303_bank_scoped_census_parent_source_added'] is True
            and base['full2303_bank_scoped_census_policy_prepublication_checked'] is True
            and base['qualified_worker_sources_preserved'] is True
            and base['worker_qualification'] == BASE_WORKER_COMPLETE
            and base['worker_source_manifest'] == BASE_WORKER_SOURCES
            and base['parent_host_timing_rows'] == 40 and base['parent_host_timing_disjoint_spans'] == 124
            and len(base['artifacts']) == 7, 'actual568 selected Tail V2 parent baseline')
    for receipt, source_map, expected_pin, phase_count in (
            (base, old_map, BASE_SOURCES, 68),
            (baseline_worker, base_worker_map, BASE_WORKER_SOURCES, 27),
            (worker, worker_map, WORKER_SOURCES, 27)):
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
    require(worker['schema'] == 'ferric-guarded-mlp-full2303-scoped-tail-cpu-v1'
            and worker['source_generation'] == 'full2303-scoped-tail-coupled-v1'
            and len(worker_map) == 1063 and len(worker['artifacts']) == 11
            and (worker['tests']['worker-tests']['passed'], worker['tests']['worker-tests']['failed'],
                 worker['tests']['worker-tests']['ignored']) == (838, 0, 4)
            and (worker['tests']['kfd-tests']['passed'], worker['tests']['kfd-tests']['failed'],
                 worker['tests']['kfd-tests']['ignored']) == (1180, 0, 8)
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and worker['runtime_source_changed'] is False
            and worker['scoped_tail_runtime_source_added'] is True
            and worker['readiness40_bank_scoped_census_tail_source_added'] is True
            and worker['full2303_scoped_tail_source_added'] is True
            and worker['inherited_readiness40_tail_preserved'] is True
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
                'full2303_scoped_tail_native_execution', 'full2303_launch_admitted',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'numerical_acceptance', 'performance_claim')),
            'actual Full Tail worker and unchanged runtime with tested real executable')
    require(baseline_worker['schema'] == 'ferric-guarded-mlp-scoped-tail-cpu-v1'
            and baseline_worker['source_generation'] == 'scoped-tail-coupled-v2'
            and len(base_worker_map) == 1059
            and (baseline_worker['tests']['worker-tests']['passed'], baseline_worker['tests']['worker-tests']['ignored']) == (820, 4)
            and (baseline_worker['tests']['kfd-tests']['passed'], baseline_worker['tests']['kfd-tests']['ignored']) == (1180, 8),
            'actual direct Tail V2 coupled baseline')
    require(compact(worker['readset']['worker-proposal.json']) == fixed['worker-proposal.json']
            and compact(worker['readset']['baseline-complete.json']) == BASE_WORKER_COMPLETE
            and compact(worker['readset']['baseline-sources.json']) == BASE_WORKER_SOURCES,
            'actual coupled direct proposal and predecessor readset')
    require(proposal['schema'] == 'ferric-full2303-bank-scoped-census-tail-parent-source-v1'
            and compact(proposal['base']['parent_receipt']) == BASE_COMPLETE
            and compact(proposal['base']['parent_sources']) == BASE_SOURCES
            and compact(proposal['requires']['worker_source_manifest']) == fixed['worker-proposal.json']
            and compact(proposal['requires']['interface']) == fixed['interface.md']
            and compact(proposal['base']['worker_receipt']) == BASE_WORKER_COMPLETE
            and compact(proposal['base']['worker_sources']) == BASE_WORKER_SOURCES
            and all(proposal[k] is False for k in ('canonical_changed', 'compiled', 'tested',
                'native_execution', 'default_routes_changed', 'default_group_policy_changed',
                'runtime_api_changed', 'deadline_changed', 'reference_feedback',
                'temporal_equivalent_to_full', 'performance_claim', 'numerical_acceptance',
                'full2303_launch_admitted')), 'exact Full bank-census parent proposal')
    require(worker_proposal['schema'] == 'ferric-full2303-bank-scoped-census-tail-worker-source-v1'
            and len(worker_proposal['files']) == 12
            and worker_proposal['source_generation'] == 'full2303-bank-scoped-census-tail-worker-v1'
            and worker_proposal['new_explicit_full2303_route'] is True
            and worker_proposal['first_two_forwards_unchanged'] is True
            and worker_proposal['full_child_abort_ms'] == 3600000
            and worker_proposal['per_dispatch_timeout_max_ms'] == 10000
            and compact(worker_proposal['base']['worker_receipt']) == BASE_WORKER_COMPLETE
            and compact(worker_proposal['base']['worker_sources']) == BASE_WORKER_SOURCES
            and compact(worker_proposal['interface']) == fixed['interface.md']
            and worker_proposal['conditional_qualification'] == dict(worker_source_files=232,
                passed=838, ignored=4, inventory=842, new_library_methods=17,
                new_readiness_cli_methods=1, library_passed=810, library_ignored=4,
                binary_passed=0, readiness_cli_passed=12, wire_passed=16,
                parent_imported_new_methods=9, shared_sequence_new_methods=0,
                existing_targets_and_old_named_statuses_must_be_preserved=True)
            and all(worker_proposal[k] is False for k in ('compiled', 'tested', 'canonical_changed',
                'native_execution', 'runtime_changed', 'default_group_policy_changed',
                'existing_selectors_changed', 'currentness_temporal_equivalence_claim',
                'performance_claim', 'numerical_acceptance', 'full_launch_admitted')),
            'qualified Full bank-census worker source lineage')
    old_names = {r['name']: r['outcome'] for r in baseline_worker['tests']['worker-tests']['named']}
    new_names = worker_proposal['new_tests']['library'] + worker_proposal['new_tests']['readiness_cli']
    require(len(old_names) == 824 and len(new_names) == len(set(new_names)) == 18
            and not set(old_names) & set(new_names)
            and {r['name']: r['outcome'] for r in worker['tests']['worker-tests']['named']}
                == dict(old_names, **{n: 'ok' for n in new_names})
            and all(worker['tests'][name] == result for name, result in baseline_worker['tests'].items()
                    if name != 'worker-tests'),
            'all old runtime, focused and worker named outcomes preserved with exactly18 additions')
    old_runtime = {n: compact(r) for n, r in base_worker_map.items() if n.startswith('fe2o3/')}
    require(len(old_runtime) == 827
            and {n: compact(r) for n, r in worker_map.items() if n.startswith('fe2o3/')} == old_runtime
            and {n: compact(r) for n, r in worker['preformat_sources'].items()
                 if n.startswith('fe2o3/')} == old_runtime,
            'all827 actual Tail runtime bodies unchanged through worker qualification')
    worker_rows = {'ferric/' + r['path']: r for r in worker_proposal['files']}
    require(len(worker_rows) == 12 and sum(r['before'] is None for r in worker_rows.values()) == 4
            and all(n.startswith(WORKER_PREFIX) and n.endswith('.rs') and r['repository'] == 'ferric'
                    for n, r in worker_rows.items()), 'twelve closed worker paths with four additions')
    require(all(worker_rows.get('ferric/' + r['path']) == {k: r[k] for k in ('repository', 'path', 'before', 'after')}
                for r in proposal['requires']['imported_policy_rows'])
            and len(proposal['requires']['imported_policy_rows']) == 2,
            'exact imported worker policy and test source rows')
    old = {n: compact(v) for n, v in old_map.items() if n.startswith('ferric/')}
    require(len(old) == 1290 and sum(n.startswith(WORKER_PREFIX) for n in old) == 228
            and all(old_map[n]['path'] == OLD_ROOT + '/' + n for n in old),
            'one actual1290 parent source closure with228 worker bodies')
    pre_worker = {n: r for n, r in old.items() if n.startswith(WORKER_PREFIX)}
    require(pre_worker == {n: compact(r) for n, r in base_worker_map.items()
                           if n.startswith(WORKER_PREFIX)},
            'all228 actual parent worker bodies equal the coupled baseline')
    for n, row in worker_rows.items():
        require(pre_worker.get(n) == row['before'], 'actual worker preimage or absence ' + n)
        pre_worker[n] = row['after']
    qualified_worker = {n: compact(r) for n, r in worker_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(qualified_worker) == 232 and set(qualified_worker) == set(pre_worker)
            and {n: compact(r) for n, r in worker['preformat_sources'].items() if n.startswith(WORKER_PREFIX)}
                == pre_worker
            and all(qualified_worker[n] == r for n, r in pre_worker.items() if n not in worker_rows)
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n for n in qualified_worker)
            and sum(n.startswith('fe2o3/') for n in worker_map) == 827,
            'only twelve qualified worker format paths; all232 final worker bodies preserved')
    parent_rows = {'ferric/' + r['path']: r for r in proposal['files']}
    require(len(parent_rows) == len(proposal['files']) == 6
            and sum(r['before'] is None for r in parent_rows.values()) == 1
            and all(n.startswith('ferric/' + PARENT_REL + '/') and n.endswith('.rs')
                    and r['repository'] == 'ferric' for n, r in parent_rows.items()),
            'exact six parent paths with one addition')
    expected = {n: r for n, r in old.items() if not n.startswith(WORKER_PREFIX)}
    expected.update(qualified_worker)
    for n, row in parent_rows.items():
        require(expected.get(n) == row['before'], 'actual parent preimage or absence ' + n)
        expected[n] = row['after']
    require(len(expected) == 1295 and not set(parent_rows) & set(worker_rows),
            '1295 Ferric bodies with disjoint ownership')
    rows = dict(worker_rows, **parent_rows)
    require(all(ordinary(n) for n in old) and all(ordinary(n) for n in rows), 'ordinary closed source paths')
    names = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    names |= {label + suffix for label in base['tests'] for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == names and len(names) == 127, 'closed127 direct lineage bodies')
    require(all(pin(bodies[n]) == compact(base['raw'][n]) for n in names - set(fixed)),
            'actual current recipes and raw named outcomes')
    require(set(packed) == set(rows) | HELPERS | {'inputs/' + n for n in names},
            'closed148 original package bodies')
    for n in rows:
        require(pin(packed[n]) == expected[n], 'exact authored parent or actually qualified worker overlay ' + n)
    for n in HELPERS:
        expected[n] = pin(packed[n])
    require(len(expected) == 1298 and sum(v['bytes'] for v in expected.values()) <= 64 << 20,
            'bounded1298 source map')
    inputs = dict(schema='ferric-guarded-mlp-full2303-scoped-tail-parent-cpu-input-v1',
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
             'inputs/interface.md': FULL / 'worker-v1/INTERFACE.md',
             'inputs/baseline-worker-complete.json': QBASEWORKER / 'evidence/complete.json',
             'inputs/baseline-worker-sources.json': QBASEWORKER / 'evidence/sources-after.json'}
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
    require(len(canonical) == 1294 and sum(n.startswith(WORKER_PREFIX) for n in canonical) == 232,
            'old1062 nonworker plus232 actually qualified worker preparation closure')
    for n, row in canonical.items():
        require(pin(read(F / n.removeprefix('ferric/'))) == row,
                'canonical predecessor and actual qualified worker body ' + n)
    for row in proposal['files']:
        if row['before'] is None:
            require(not os.path.lexists(F / row['path']), 'new parent source is absent')
    require(all(read(path) == bodies[n] for n, path in paths.items()), 'all packaging inputs unchanged')
    raw_input = encoded(inputs)
    bodies['input-manifest.json'] = raw_input
    require(len(bodies) == 149 and sum(map(len, bodies.values())) <= 32 << 20, 'bounded149-member package')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive_stream:
            for n, raw in sorted(bodies.items()):
                item = tarfile.TarInfo(n); item.size = len(raw); item.mode = 0o600
                archive_stream.addfile(item, io.BytesIO(raw))
    with (P / 'parent-input-manifest.json').open('xb') as stream:
        stream.write(raw_input)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))),
                         input=pin(raw_input), members=149, source_files=1298), sort_keys=True))


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
        require(len(members) == len({m.name for m in members}) == 149
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
    require(all(pin(read(BASE / n)) == row for n, row in old.items()), 'all1290 actual parent prehashes')
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
            'all1298 staged source rows')
    require(all(pin(read(BASE / n)) == row for n, row in old.items()), 'immutable baseline body posthashes')
    require(read(E / BASENAME) == archive_raw, 'archive unchanged during staging')
    require(read(BASE / 'evidence/complete.json') == bodies['inputs/parent-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/parent-sources.json'],
            'immutable baseline receipt posthashes')
    receipt = dict(schema='ferric-guarded-mlp-full2303-scoped-tail-parent-source-stage-v1', passed=True,
        archive=pin(archive_raw), input=pin(raw_input), source_files=1298, ferric_files=1295,
        overlay_files=18, parent_format_paths=6, qualified_worker_files=232, qualified_worker_sources_preserved=True,
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
