"""Bounded parent-only qualification; no model, worker, or GPU execution."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import sys
import time
import tomllib
import types

ROOT = Path(__file__).resolve().parent
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-v228-v2')
FERRIC = ROOT / 'ferric'
PARENT_REL = 'adapters/m1-engineering-execution-v1'
PARENT = FERRIC / PARENT_REL
CARGO_HOME = ROOT / 'cargo-home'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
SUPPORT_SHA = '71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13'
BASE_COMPLETE = {"bytes":4092678,"sha256":"6580942cb6b39d69b6fdac91a48ac6e0a25cd4e7648c4175152f3620ff7bacb6"}
BASE_SOURCES = {"bytes":525041,"sha256":"cf69d8bb33a1c9bf2fa0238e18da01ef70bba182a488388de378d1f611bc35fb"}
PROPOSAL_PINS = {"parent-proposal.json":{"bytes":25554,"sha256":"cfc7add994656a5418fbc77abb0f6d16d820c3f1763ba8b5835b1e5df89bd3b2"},"worker-proposal.json":{"bytes":27229,"sha256":"96463bf9a70d6020e28c2d44260f589c38e7f61229f8c8e77f28242f1893ca71"},"runtime-proposal.json":{"bytes":10858,"sha256":"71da4afd43eb9f7de8fa1cac47cf8e9b2f0ececc86cfb98877b94678d500df83"},"interface.md":{"bytes":24881,"sha256":"749fd470a27cf7630f292b04c5464567e4e7d398ee7855b1fbd152824421a8e4"},"parent-predecessor.json":{"bytes":23473,"sha256":"840417429d3357155672cc5d096d592329461dd742d6d2476580759495dd116a"},"worker-repair.json":{"bytes":4324,"sha256":"8e5ffe3f2b4d87b9967ef993e31385c42461dbdd92a49c1f63a2578e1b8e6128"}}
BASE_WORKER_COMPLETE = {"bytes":2475794,"sha256":"a7fcea3f09cfcda84ba556f60b5f1578ed378a45fc84bb81fbfa6ed31273cd64"}
WORKER_COMPLETE = {"bytes":2448697,"sha256":"3c4fe3a6c4942b20ec61fd7ea5c3409526e2167c43d24a62fb8660c34b0e713e"}
BASE_WORKER_SOURCES = {"bytes":438675,"sha256":"5372e185504d11ed89d45cfb3d3e32672f93e8d1bbf3d04e44932b280300528c"}
WORKER_SOURCES = {"bytes":424416,"sha256":"5de4929b2832eda55a53f99945fc4b64a9fbda28b4d4f8bef07f2e8d5e000d6a"}
WORKER_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-scoped-tail-cpu-v228-v2'
TAIL_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-scoped-tail-cpu-v228-v1'
TAIL_COMPLETE = {"bytes":2449865,"sha256":"a2e3d99f94b5b01223f42f3d8917c2cf6a1fc25f9227d4c341bc2b9a7cc9285c"}
TAIL_SOURCES = {"bytes":424416,"sha256":"a2a74ed023e9b2aac224621d37f328d097838c317c901a361c1496064e162ec4"}
WORKER_PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
LOCK_SHA = 'ec06e964ed72dc9b97d9f5769bd867bf05398781b6e17a9f605137742d6e19ea'
CACHE_PIN = dict(bytes=96226, sha256='984b2a893ba64f35cda1cdceb5d379028416c00c36815c678200480c7e8904ad')
PARENT_TOOLS_SHA = 'c38e83e58d01e60b7311efa1ff53cb94e69dbbcd227ee30bde6a04c7928590d9'
PARENT_TOOLCHAIN = Path('/home/harmenon/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin')
PACKAGE = 'ferric-m1-engineering-execution-v1'
BINARY = 'ferric-qwen3-finite-guarded-mlp-decode-engineering'
ADDED_BINARY = 'ferric-qwen3-guarded-mlp-full2303-engineering'
OLD_FEATURE = 'guarded-mlp-readiness-engineering'
FULL_FEATURE = 'guarded-mlp-full2303-engineering'
FEATURE = OLD_FEATURE + ',' + FULL_FEATURE
OLD_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-full2303-bank-scoped-census-parent-cpu-v228-v1'
WHOLE_WALL, LEAF_WALL, CLEANUP_RESERVE = 3600, 1200, 50


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def load_support():
    path = ROOT / 'qualification_support.py'
    require(path.resolve(strict=True) == path and path.stat().st_size <= 65536, 'ordinary support')
    body = path.read_bytes()
    require(hashlib.sha256(body).hexdigest() == SUPPORT_SHA, 'reviewed worker parsing support')
    module = types.ModuleType('parent_qualification_support')
    module.__file__ = str(path)
    exec(compile(body, str(path), 'exec'), module.__dict__)
    module.ROOT, module.WORKER = ROOT, PARENT
    module.OUT, module.TARGET, module.TMP = OUT, TARGET, TMP
    return module


def snapshot(h):
    paths = [ROOT / name for name in ('run_cpu.py', 'qualification_support.py', 'supervisor.py')]
    paths += h.files_below(FERRIC)
    require(1000 <= len(paths) <= 3000, 'bounded parent source closure')
    rows = {str(p.relative_to(ROOT)): h.pin(p) for p in sorted(paths)}
    require(len(rows) == len(paths) and sum(p['bytes'] for p in rows.values()) <= 64 << 20,
            'parent source extent')
    return rows


def configurations(h):
    directories = {ROOT, FERRIC, PARENT, CARGO_HOME, *ROOT.parents, *PARENT.parents}
    directories |= {p.parent for p in FERRIC.rglob('Cargo.toml')}
    paths = {d / '.cargo' / n for d in directories for n in ('config', 'config.toml')}
    paths |= {CARGO_HOME / n for n in ('config', 'config.toml')}
    paths |= {Path('/home/harmenon/.cargo') / n for n in ('config', 'config.toml')}
    result = {str(p): h.pin(p) if os.path.lexists(p) else None for p in sorted(paths)}
    require(not any(result.values()), 'inherited Cargo configuration refused')
    return result


def read(h, name, expected, readset):
    path = ROOT / 'inputs' / name
    require(path.resolve(strict=True) == path, 'lineage alias')
    pin = h.pin(path)
    require(compact(pin) == expected and pin['bytes'] <= 16 << 20, 'lineage identity ' + name)
    body = path.read_bytes()
    require(h.pin(path) == pin, 'lineage changed while reading')
    readset[name] = pin
    return body


def lineage(h, core, inputs, before, readset):
    fixed = dict(PROPOSAL_PINS, **{'parent-complete.json': BASE_COMPLETE,
        'parent-sources.json': BASE_SOURCES, 'worker-complete.json': WORKER_COMPLETE,
        'worker-sources.json': WORKER_SOURCES,
        'baseline-worker-sources.json': BASE_WORKER_SOURCES,
        'tail-worker-complete.json': TAIL_COMPLETE, 'tail-worker-sources.json': TAIL_SOURCES})
    bodies = {name: read(h, name, row, readset) for name, row in inputs['lineage'].items()}
    require(all(compact(readset[name]) == row for name, row in fixed.items()), 'literal actual parent/worker/source pins')
    base = json.loads(bodies['parent-complete.json'])
    old_map = json.loads(bodies['parent-sources.json'])
    proposal = json.loads(bodies['parent-proposal.json'])
    worker = json.loads(bodies['worker-complete.json'])
    worker_map = json.loads(bodies['worker-sources.json'])
    worker_proposal = json.loads(bodies['worker-proposal.json'])
    runtime_proposal = json.loads(bodies['runtime-proposal.json'])
    base_worker_map = json.loads(bodies['baseline-worker-sources.json'])
    tail = json.loads(bodies['tail-worker-complete.json'])
    tail_map = json.loads(bodies['tail-worker-sources.json'])
    repair = json.loads(bodies['worker-repair.json'])
    parent_predecessor = json.loads(bodies['parent-predecessor.json'])
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
            (base, old_map, BASE_SOURCES, 68), (tail, tail_map, TAIL_SOURCES, 27),
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
    require(tail['schema'] == 'ferric-guarded-mlp-scoped-tail-cpu-v1'
            and tail['source_generation'] == 'scoped-tail-coupled-v1'
            and len(tail_map) == 1059 and len(tail['artifacts']) == 11
            and (tail['tests']['worker-tests']['passed'], tail['tests']['worker-tests']['failed'],
                 tail['tests']['worker-tests']['ignored']) == (820, 0, 4)
            and (tail['tests']['kfd-tests']['passed'], tail['tests']['kfd-tests']['failed'],
                 tail['tests']['kfd-tests']['ignored']) == (1180, 0, 8)
            and tail['cli_executable_unchanged_across_tests'] is True
            and tail['cli_executable_before_tests']['pin'] == tail['artifacts']['worker']['pin']
            and tail['runtime_source_changed'] is True
            and tail['scoped_tail_runtime_source_added'] is True
            and tail['readiness40_bank_scoped_census_tail_source_added'] is True
            and tail['scoped_bank_rearm_runtime_source_added'] is True
            and tail['readiness40_bank_scoped_warm_source_added'] is True
            and tail['full2303_scoped_warm_source_added'] is True
            and tail['inherited_scoped_currentness_runtime_preserved'] is True
            and tail['scoped_capacity_census_runtime_source_added'] is True
            and tail['readiness40_bank_scoped_census_source_added'] is True
            and tail['allocation_preflights_changed'] is True
            and tail['full2303_bank_scoped_census_source_added'] is True
            and tail['inherited_readiness40_bank_scoped_census_preserved'] is True
            and tail['full2303_deadline_changed'] is False
            and all(tail[k] is False for k in ('full2303_scoped_warm_native_execution',
                'readiness40_bank_scoped_warm_native_execution', 'readiness40_bank_scoped_census_native_execution',
                'full2303_bank_scoped_census_native_execution',
                'readiness40_bank_scoped_census_tail_native_execution',
                'full2303_launch_admitted',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'numerical_acceptance', 'performance_claim')),
            'actual Tail runtime and worker qualification with tested real executable')
    require(compact(tail['readset']['worker-proposal.json']) == fixed['worker-proposal.json']
            and compact(tail['readset']['runtime-proposal.json']) == fixed['runtime-proposal.json']
            and compact(tail['readset']['baseline-complete.json']) == BASE_WORKER_COMPLETE
            and compact(tail['readset']['baseline-sources.json']) == BASE_WORKER_SOURCES,
            'actual coupled direct proposal and predecessor readset')
    require(proposal['schema'] == 'ferric-bank-scoped-census-tail-readiness40-parent-source-v1'
            and proposal['source_generation'] == 'bank-scoped-census-tail-parent-v2'
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
    qualified_runtime = {n: compact(r) for n, r in tail_map.items() if n.startswith('fe2o3/')}
    require(len(qualified_runtime) == 827 and set(qualified_runtime) == set(pre_runtime)
            and {n: compact(r) for n, r in tail['preformat_sources'].items()
                 if n.startswith('fe2o3/')} == pre_runtime
            and all(qualified_runtime[n] == r for n, r in pre_runtime.items() if n not in runtime_rows)
            and all(tail_map[n]['path'] == TAIL_ROOT + '/' + n for n in qualified_runtime),
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
    qualified_worker = {n: compact(r) for n, r in tail_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(qualified_worker) == 228 and set(qualified_worker) == set(pre_worker)
            and {n: compact(r) for n, r in tail['preformat_sources'].items() if n.startswith(WORKER_PREFIX)}
                == pre_worker
            and all(qualified_worker[n] == r for n, r in pre_worker.items() if n not in worker_rows)
            and all(tail_map[n]['path'] == TAIL_ROOT + '/' + n for n in qualified_worker)
            and sum(n.startswith('fe2o3/') for n in tail_map) == 827,
            'only thirteen qualified worker format paths; all228 final worker bodies preserved')
    require(proposal['parent_rust_unchanged_from_v1'] is True
            and compact(proposal['predecessor_manifest']) == fixed['parent-predecessor.json']
            and proposal['base'] == parent_predecessor['base']
            and proposal['files'] == parent_predecessor['files']
            and proposal['requires']['imported_policy_rows']
                == parent_predecessor['requires']['imported_policy_rows']
            and compact(proposal['requires']['worker_repair']) == fixed['worker-repair.json']
            and proposal['requires']['worker_qualification_generation'] == 'scoped-tail-coupled-v2'
            and proposal['requires']['qualified_worker_baseline']
                == dict(terminal=TAIL_COMPLETE, sources=TAIL_SOURCES)
            and proposal['conditional_parent_projection']['selected_passes'] == 568
            and proposal['conditional_parent_projection']['library_inventory'] == 1032,
            'parent V2 preserves seven Rust rows and adds explicit repaired-worker provenance')
    added_shared = dict(
        name='long_sequence_completed_warm_tail_then_worker_reject_is_terminal_without_commit_or_close',
        qualified_name='guarded_mlp_long_sequence_v2::tests::long_sequence_completed_warm_tail_then_worker_reject_is_terminal_without_commit_or_close',
        path='adapters/tp-peer-finite-engineering-worker-v1/src/guarded_mlp_long_sequence_v2_tests.rs',
        target='parent_library', imported=True)
    require(proposal['test_roster'] == parent_predecessor['test_roster'] + [added_shared],
            'only previously omitted shared-sequence parent test added to the projection')
    require(repair['schema'] == 'ferric-scoped-tail-shared-test-repair-v2'
            and repair['source_generation'] == 'scoped-tail-shared-test-repair-v2'
            and repair['status'] == 'source-only-uncompiled-unexecuted'
            and repair['base'] == dict(terminal=TAIL_COMPLETE, sources=TAIL_SOURCES,
                                      runtime_files=827, worker_files=228)
            and all(compact(repair['requires'][name]) == fixed[alias] for name, alias in (
                ('worker-v1/source-manifest.json', 'worker-proposal.json'),
                ('runtime-v1/source-manifest.json', 'runtime-proposal.json'),
                ('INTERFACE.md', 'interface.md')))
            and all(repair[k] is False for k in ('production_changed', 'runtime_changed',
                'test_assertions_changed', 'tests_executed', 'canonical_changed',
                'native_execution', 'numerical_acceptance', 'performance_claim'))
            and repair['new_test_names'] == [] and len(repair['unchanged_test_names']) == 9,
            'source-only one-test repair with unchanged production and test names')
    repair_path = 'ferric/' + added_shared['path']
    require(len(repair['files']) == 1 and repair['files'][0]['repository'] == 'ferric'
            and repair['files'][0]['path'] == added_shared['path']
            and repair_path in worker_rows
            and repair['files'][0]['before'] == qualified_worker[repair_path],
            'repair preimage is the actual formatted Tail V1 shared test')
    require(worker['schema'] == 'ferric-guarded-mlp-scoped-tail-cpu-v1'
            and worker['source_generation'] == 'scoped-tail-coupled-v2'
            and worker['runtime_source_changed'] is False
            and worker['shared_test_fixture_repaired'] is True
            and len(worker_map) == 1059 and len(worker['artifacts']) == 11
            and worker['tests'] == tail['tests']
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and all(worker[k] is tail[k] for k in (
                'scoped_tail_runtime_source_added', 'readiness40_bank_scoped_census_tail_source_added',
                'scoped_bank_rearm_runtime_source_added', 'readiness40_bank_scoped_warm_source_added',
                'full2303_scoped_warm_source_added', 'inherited_scoped_currentness_runtime_preserved',
                'scoped_capacity_census_runtime_source_added', 'readiness40_bank_scoped_census_source_added',
                'allocation_preflights_changed', 'full2303_bank_scoped_census_source_added',
                'inherited_readiness40_bank_scoped_census_preserved', 'full2303_deadline_changed',
                'full2303_scoped_warm_native_execution', 'readiness40_bank_scoped_warm_native_execution',
                'readiness40_bank_scoped_census_native_execution', 'full2303_bank_scoped_census_native_execution',
                'readiness40_bank_scoped_census_tail_native_execution', 'full2303_launch_admitted',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'numerical_acceptance', 'performance_claim')),
            'fresh V2 coupled qualification retains every old named outcome and source authority')
    require(compact(worker['readset']['baseline-complete.json']) == TAIL_COMPLETE
            and compact(worker['readset']['baseline-sources.json']) == TAIL_SOURCES
            and all(compact(worker['readset'][n]) == fixed[n] for n in (
                'worker-proposal.json', 'runtime-proposal.json', 'worker-repair.json')),
            'V2 actual direct baseline and both original and repaired worker provenance')
    require({n: compact(r) for n, r in worker['preformat_sources'].items()
                if n.startswith('fe2o3/')} == qualified_runtime
            and {n: compact(r) for n, r in worker_map.items()
                if n.startswith('fe2o3/')} == qualified_runtime,
            'all827 actual Tail V1 runtime bodies unchanged in V2')
    repaired_pre = dict(qualified_worker)
    repaired_pre[repair_path] = repair['files'][0]['after']
    qualified_worker = {n: compact(r) for n, r in worker_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(qualified_worker) == 228 and set(qualified_worker) == set(repaired_pre)
            and {n: compact(r) for n, r in worker['preformat_sources'].items()
                 if n.startswith(WORKER_PREFIX)} == repaired_pre
            and all(qualified_worker[n] == r for n, r in repaired_pre.items() if n != repair_path)
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n
                    for n in (*qualified_worker, *qualified_runtime)),
            'only the repaired test may format; all228 actual V2 worker bodies preserved')
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
    scopes = base['tests']
    expected_lineage = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    expected_lineage |= {name + suffix for name in scopes for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == expected_lineage and len(bodies) == 131, 'closed131 direct lineage bodies')
    for name in expected_lineage - set(fixed):
        require(compact(readset[name]) == compact(base['raw'][name]), 'actual baseline raw ' + name)
    listed = core.inventory(bodies['parent-lib-list.stdout'].decode())
    require(listed == base['inventory'], 'actual1014-name parent inventory')
    for name, prior in scopes.items():
        require(core.outcomes(bodies[name + '.stdout'].decode()) == prior
                and prior['failed'] == prior['ignored'] == 0, 'preserved raw selected outcomes ' + name)
        command = json.loads(bodies[name + '.command.json'])
        phase = next(p for p in base['phases'] if p['label'] == name)
        require(command['argv'] == phase['argv'], 'actual selected command join')
    require({n: compact(r) for n, r in before.items() if n.startswith('ferric/')} == expected
            and len(before) == 1293, 'complete actual baseline, qualified worker and seven parent overlays')
    require(inputs['parent_overlay'] == sorted(parent_rows), 'only seven parent Rust formatting inputs')
    require(h.pin(PARENT / 'Cargo.lock')['sha256'] == LOCK_SHA, 'unchanged parent lock')
    test_groups = (
        (PARENT / 'src/tp_finite_client/long/readiness_bank_scoped_census_tail_tests.rs',
         'tp_finite_client::long::readiness::bank_scoped_census_tail::tests::', 4, 4),
        (PARENT / 'src/tp_finite_client/long/readiness_host_timing_tests.rs',
         'tp_finite_client::long::readiness::host_timing::tests::', 20, 2),
        (FERRIC / 'adapters/tp-peer-finite-engineering-worker-v1/src/finite_guarded_mlp_readiness_bank_scoped_census_tail_v4_tests.rs',
         'finite_guarded_mlp_readiness_bank_scoped_census_tail_v4::tests::', 11, 11),
        (FERRIC / 'adapters/tp-peer-finite-engineering-worker-v1/src/guarded_mlp_long_sequence_v2_tests.rs',
         'guarded_mlp_long_sequence_v2::tests::', 9, 1))
    additions = []
    for path, prefix, total, added in test_groups:
        declared = [prefix + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', path.read_text())]
        new = [n for n in declared if n not in listed]
        require(len(declared) == len(set(declared)) == total and len(new) == added,
                'exact source-declared new and inherited methods')
        additions.extend(new)
    additions.sort()
    declared_additions = proposal['test_roster']
    library_names = sorted(r['qualified_name'] for r in declared_additions if r['target'] == 'parent_library')
    bin_names = sorted(r['qualified_name'] for r in declared_additions if r['target'] == 'parent_readiness_binary')
    require(len(declared_additions) == 19 and len(additions) == len(set(additions)) == 18
            and additions == library_names and len(set(library_names + bin_names)) == 19,
            'exact eighteen additive library names and one readiness binary name')
    source = (PARENT / 'src/bin/ferric-qwen3-guarded-mlp-readiness-engineering.rs').read_text()
    declared_bin = sorted('tests::' + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', source))
    old_bin = set(core.statuses(scopes['readiness-bin-tests']))
    require(len(bin_names) == 1 and not set(bin_names) & old_bin
            and declared_bin == sorted(old_bin | set(bin_names)), 'exact one additive Readiness40 CLI test')
    parent_tools = base['parent_toolchain_observation']
    require(parent_tools['schema'] == 'ferric-guarded-mlp-parent-toolchain-v1'
            and parent_tools['toolchain'] == str(PARENT_TOOLCHAIN)
            and parent_tools['tool_pins'] == base['tool_pins'] and len(base['tool_pins']) == 8,
            'actual unchanged parent toolchain')
    for row in parent_tools['tool_pins'].values():
        require(h.pin(Path(row['path'])) == row, 'parent tool drift')
    return base, parent_tools, proposal, bodies, scopes, listed, additions, bin_names


def relocate(value):
    if isinstance(value, str):
        return value.replace(OLD_ROOT, str(ROOT))
    if isinstance(value, list):
        return [relocate(v) for v in value]
    if isinstance(value, dict):
        return {k: relocate(v) for k, v in value.items()}
    return value


def metadata_contract(h, old):
    current = json.loads((OUT / 'metadata.stdout').read_bytes())
    require(current == relocate(old), 'unchanged locked dependency/feature/target graph')
    require(current['workspace_root'] == str(PARENT) and current['target_directory'] == str(TARGET)
            and len(current['packages']) == 209, 'standalone parent metadata')
    external = {}
    external_packages = 0
    for row in current['packages']:
        path = Path(row['manifest_path'])
        require(path.resolve(strict=True) == path, 'dependency ordinary path')
        if row['source'] is None:
            require(path.is_relative_to(FERRIC), 'local dependency escapes source closure')
        else:
            require(path.is_relative_to(CARGO_HOME), 'external dependency escapes private cache')
            external_packages += 1
            root = path.parent
            if row['source'].startswith('git+'):
                parts = path.relative_to(CARGO_HOME).parts
                require(parts[:2] == ('git', 'checkouts') and len(parts) >= 5, 'private Git checkout source')
                root = CARGO_HOME.joinpath(*parts[:4])
            if str(root) not in external:
                external[str(root)] = dependency_snapshot(h, root)
    require(external_packages == 181 and len(external) == 121,
            '181 external packages in118 registry plus3 complete Git source roots')
    return current, external


def dependency_snapshot(h, root):
    return {str(p.relative_to(root)): h.pin(p) for p in h.files_below(root, packed=False)
            if '.git' not in p.relative_to(root).parts}


def cache_contract(h, inputs):
    path = ROOT / 'cargo-cache-manifest.json'
    row = h.pin(path)
    require(compact(row) == inputs['cache_manifest'] == CACHE_PIN, 'root-bound private parent cache')
    value = json.loads(path.read_bytes())
    stage_path = ROOT / 'cargo-cache-stage-complete.json'
    stage_pin = h.pin(stage_path)
    require(stage_pin['bytes'] <= 1 << 20, 'bounded private cache stage receipt')
    stage = json.loads(stage_path.read_bytes())
    require(value['schema'] == 'ferric-guarded-mlp-parent-cache-v1'
            and value['lock'] == compact(h.pin(PARENT / 'Cargo.lock'))
            and len(value['packages']) == 118 and len(value['files']) == 247
            and len(value['git_commits']) == 3 and value['cargo_execution'] is False,
            'closed parent cache lock/packages')
    lock = tomllib.loads((PARENT / 'Cargo.lock').read_text())['package']
    registry = sorted((p['name'], p['version'], p['checksum']) for p in lock
                      if p.get('source', '').startswith('registry+'))
    require(registry == sorted((p['name'], p['version'], p['checksum']) for p in value['packages'])
            and {p['source'] for p in lock if p.get('source', '').startswith('git+')}
                == {p['source'] for p in value['git_commits']}, 'cache exact locked package/revision roster')
    require(stage['schema'] == 'ferric-guarded-mlp-parent-cache-stage-v1'
            and stage['passed'] is True and stage['manifest'] == compact(row)
            and stage['cargo_home'] == str(CARGO_HOME) and stage['lock'] == value['lock']
            and stage['files'] == value['files'] and stage['git_commits'] == value['git_commits']
            and stage['packages'] == 118 and stage['cache_files'] == 247
            and stage['inner_members'] == value['inner_members'] == 5023
            and stage['inner_expanded_bytes'] == value['inner_expanded_bytes'] == 85992791
            and stage['git_objects'] == value['git_objects'] == 7723
            and stage['git_object_bytes'] == value['git_object_bytes'] == 105759011
            and all(stage[k] is False for k in ('shared_cache_changed', 'lock_changed',
                'project_code_executed', 'crate_sources_extracted', 'git_sources_checked_out')),
            'authenticated cache staging contract')
    files = {}
    for name, expected in value['files'].items():
        require(type(name) is str and Path(name).as_posix() == name and not name.startswith('/')
                and '..' not in Path(name).parts, 'ordinary cache input')
        actual = h.pin(CARGO_HOME / name)
        require(compact(actual) == expected, 'private cache input changed')
        files[name] = actual
    require(h.pin(path) == row and h.pin(stage_path) == stage_pin, 'cache provenance changed during read')
    return dict(manifest=row, stage=stage_pin, files=files)


def main():
    started = time.monotonic()
    deadline = started + WHOLE_WALL
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1])
            and PARENT_TOOLS_SHA is not None and WORKER_COMPLETE is not None and WORKER_SOURCES is not None, 'python3 -B run_cpu.py INPUT_SHA; bind actual parent tools first')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT
            and not any(os.path.lexists(p) for p in (OUT, TARGET, TMP)), 'fresh exact parent root')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'parent qualification host/UID')
    require(shutil.disk_usage(ROOT).free >= 40 << 30, 'initial free floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    nice = os.getpriority(os.PRIO_PROCESS, 0)
    require(nice in (0, 10), 'parent nice')
    if nice == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, 1 << 30)):
        old = resource.getrlimit(kind)
        limit = min([cap] + [v for v in old if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper')
    core = load_support()
    h = core.load_supervisor()
    h.TOOLCHAIN, h.TOOLCHAIN_LIB = PARENT_TOOLCHAIN, PARENT_TOOLCHAIN.parent / 'lib'
    h.sources = lambda: snapshot(h)
    original_scratch = h.scratch_bytes
    def scratch():
        paths = h.files_below(CARGO_HOME, packed=False)
        require(len(paths) <= 150000, 'bounded private cache files')
        total = original_scratch()
        for p in paths:
            try:
                total += p.lstat().st_size
            except FileNotFoundError:
                pass
        return total
    h.scratch_bytes = scratch
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - CLEANUP_RESERVE - time.monotonic()))
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    phases, tests, artifacts, readset, tool_pins, dependencies, errors = [], {}, {}, {}, {}, {}, []
    inputs = input_pin = before = formatted = after = config = cache = metadata = parent_tools = None
    failure = None
    changed, listed = [], []
    try:
        input_pin = h.pin(ROOT / 'input-manifest.json')
        require(input_pin['sha256'] == sys.argv[1], 'root-bound parent input')
        inputs = json.loads((ROOT / 'input-manifest.json').read_bytes())
        require(set(inputs) == {'schema', 'files', 'lineage', 'parent_overlay', 'cache_manifest', 'git_revision'}
                and inputs['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-input-v1', 'closed parent input')
        before = snapshot(h)
        require(inputs['files'] == {k: compact(v) for k, v in before.items()}, 'exact parent source input')
        h.save('sources-preformat.json', before)
        base, parent_tools, proposal, bodies, scopes, old_list, additions, bin_names = lineage(h, core, inputs, before, readset)
        h.SHARED_LIBRARIES = {name: row['bytes'] for name, row in parent_tools['tool_pins'].items()
                              if name not in {'rustc', 'rustdoc', 'rustfmt', 'cargo', 'prlimit'}}
        config = configurations(h)
        cache = cache_contract(h, inputs)
        require({str(p.relative_to(CARGO_HOME)) for p in h.files_below(CARGO_HOME, packed=False)}
                == set(cache['files']), 'fresh private cache contains only staged immutable inputs')
        tool_pins = {name: h.pin(h.TOOLCHAIN / name) for name in ('rustc', 'rustdoc', 'rustfmt', 'cargo')}
        tool_pins['prlimit'] = h.pin(Path('/usr/bin/prlimit'))
        tool_pins.update({name: h.pin(h.TOOLCHAIN_LIB / name) for name in h.SHARED_LIBRARIES})
        require(tool_pins == parent_tools['tool_pins'], 'actual bound parent1.97.1 toolchain')
        env = dict(HOME='/home/harmenon', PATH=str(h.TOOLCHAIN) + ':/usr/bin:/bin',
            CARGO_HOME=str(CARGO_HOME), CARGO_TARGET_DIR=str(TARGET), TMPDIR=str(TMP),
            LD_LIBRARY_PATH=str(TARGET / 'debug/deps') + ':' + str(h.TOOLCHAIN_LIB),
            RUSTC=str(h.TOOLCHAIN / 'rustc'), RUSTDOC=str(h.TOOLCHAIN / 'rustdoc'),
            CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true',
            CARGO_CACHE_AUTO_CLEAN_FREQUENCY='never', MALLOC_ARENA_MAX='2', RUST_BACKTRACE='1',
            RUSTC_BOOTSTRAP='fe2o3_device,fe2o3_macros',
            CARGO_PROFILE_DEV_OPT_LEVEL='2', CARGO_PROFILE_DEV_DEBUG='0',
            CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
            CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_TEST_DEBUG='0',
            CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
            ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
        def leaf(name, argv, seconds=LEAF_WALL, source=formatted):
            h.run(name, argv, env, phases, deadline, source, seconds, PARENT)
        fmt = [str(h.TOOLCHAIN / 'rustfmt'), '--edition', '2024', '--config', 'skip_children=true']
        paths = [str(ROOT / n) for n in inputs['parent_overlay']]
        leaf('rustfmt', fmt + paths, 120)
        formatted = snapshot(h)
        require(set(formatted) == set(before), 'formatter source roster')
        changed = [n for n in formatted if formatted[n] != before[n]]
        require(set(changed) <= set(inputs['parent_overlay']), 'formatter scope')
        h.save('sources-before.json', formatted)
        leaf('rustfmt-check', fmt + ['--check'] + paths, 120, formatted)
        leaf('rustc-version', [str(h.TOOLCHAIN / 'rustc'), '--version', '--verbose'], 60, formatted)
        require((OUT / 'rustc-version.stdout').read_text() == parent_tools['rustc_version'],
                'current parent rustc version differs from its actual observation')
        cargo = str(h.TOOLCHAIN / 'cargo')
        common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(PARENT / 'Cargo.toml')]
        feature = ['--features', FEATURE]
        leaf('metadata', [cargo, 'metadata', '--offline', '--locked', '--manifest-path', str(PARENT / 'Cargo.toml'),
                         *feature, '--format-version', '1'], 120, formatted)
        metadata, dependencies = metadata_contract(h, json.loads(bodies['metadata.stdout']))
        h.save('dependencies-before.json', dependencies)
        leaf('parent-lib-list', [cargo, 'test', *common, *feature, '--lib', '--', '--list', '--format=terse'], source=formatted)
        listed = core.inventory((OUT / 'parent-lib-list.stdout').read_text())
        require(listed == sorted(old_list + additions) and len(listed) == 1032, 'old1014 plus exact18 library names')
        selected = set()
        list_labels = {'guarded-bin-list': 'guarded-bin-tests', 'readiness-bin-list': 'readiness-bin-tests',
                       'full2303-bin-list': 'full2303-bin-tests'}
        ordered_scopes = [p['label'] for p in base['phases'] if p['label'] in scopes or p['label'] in list_labels]
        require(len(ordered_scopes) == 61 and set(ordered_scopes) == set(scopes) | set(list_labels),
                'all58 original selected and three inventory recipes')
        for label in ordered_scopes:
            original = next(p for p in base['phases'] if p['label'] == label)
            argv = relocate(original['argv'][6:])
            require(argv[:2] == [cargo, 'test'] and argv[argv.index('--features') + 1] == FEATURE,
                    'unchanged original owned Cargo recipe')
            leaf(label, argv, source=formatted)
            if label in list_labels:
                names = set(core.statuses(scopes[list_labels[label]]))
                if label == 'readiness-bin-list':
                    names |= set(bin_names)
                require(core.inventory((OUT / (label + '.stdout')).read_text()) == sorted(names),
                        'preserved named binary inventory plus explicit addition')
                continue
            names = set(core.statuses(scopes[label]))
            if label == 'readiness-bin-tests':
                names |= set(bin_names)
            if '--lib' in argv:
                selector = argv[argv.index('--lib') + 1]
                names |= {n for n in additions if n == selector or ('--exact' not in argv and selector in n)}
                require(not selected & names, 'inherited library selections overlap')
                selected |= names
            value = core.outcomes((OUT / (label + '.stdout')).read_text())
            require(core.statuses(value) == {n: 'ok' for n in names}
                    and value['failed'] == value['ignored'] == 0, 'all inherited and added named outcomes ' + label)
            tests[label] = value
        require(set(additions) <= selected, 'every new library method selected exactly once')
        build_bins = sorted(base['artifacts'])
        require(len(build_bins) == 7 and BINARY in build_bins and ADDED_BINARY in build_bins,
                'seven unchanged parent products')
        args = [item for n in build_bins for item in ('--bin', n)]
        leaf('parent-builds', [cargo, 'build', '--profile', 'test', *common, *feature, *args, '--message-format=json'], source=formatted)
        records = h.build_records(OUT / 'parent-builds.stdout')
        for name in build_bins:
            candidates = [r for r in records if r.get('reason') == 'compiler-artifact'
                and r.get('manifest_path') == str(PARENT / 'Cargo.toml') and r.get('target', {}).get('name') == name
                and r['target']['kind'] == ['bin'] and r.get('profile', {}).get('test') is False]
            require(len(candidates) == 1, 'exact selected parent product')
            row = candidates[0]
            path = Path(row['executable'])
            require(path.resolve(strict=True) == path and path.is_relative_to(TARGET)
                    and {OLD_FEATURE, FULL_FEATURE} <= set(row['features']) and row['filenames'].count(str(path)) == 1, 'new target artifact')
            with path.open('rb') as stream:
                require(stream.read(4) == b'\x7fELF', 'parent ELF')
            artifacts[name] = dict(pin=h.pin(path), cargo_artifact=row)
        leaf('parent-default-check', [cargo, 'check', *common, '--lib', '--message-format=json'], source=formatted)
        require(len(phases) == 68 and len(tests) == 58 and sum(v['passed'] for v in tests.values()) == 568
                and sum(v['failed'] + v['ignored'] for v in tests.values()) == 0, '68 phases and568 selected passes')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    def check(name, action):
        try:
            require(deadline - time.monotonic() > 5, 'postcheck reserve')
            signal.setitimer(signal.ITIMER_REAL, deadline - time.monotonic() - 5)
            action()
        except BaseException as error:
            errors.append(name + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    def source_check():
        nonlocal after
        after = snapshot(h)
        h.save('sources-after.json', after)
        if formatted is not None:
            require(after == formatted, 'parent source/lock changed')
        elif before is not None:
            require(set(after) == set(before) and all(after[n] == r for n, r in before.items()
                if n not in (inputs or {}).get('parent_overlay', [])), 'failed formatting escaped scope')
    check('sources', source_check)
    if config is not None:
        check('configuration', lambda: require(configurations(h) == config, 'Cargo config drift'))
    if cache is not None:
        check('cache', lambda: require(cache_contract(h, inputs) == cache, 'immutable cache drift'))
    if input_pin is not None:
        check('input', lambda: require(h.pin(ROOT / 'input-manifest.json') == input_pin, 'input drift'))
    for name, row in dict(readset, **tool_pins).items():
        check('input ' + name, lambda r=row: require(h.pin(Path(r['path'])) == r, 'readset drift'))
    for directory, expected in dependencies.items():
        check('dependency ' + directory, lambda d=directory, e=expected: require(
            dependency_snapshot(h, Path(d)) == e, 'dependency drift'))
    for name, row in artifacts.items():
        check('artifact ' + name, lambda r=row: require(h.pin(Path(r['pin']['path'])) == r['pin'], 'artifact drift'))
    check('scratch', lambda: require(h.scratch_bytes() <= h.CACHE_LIMIT, 'final scratch cap'))
    raw = {}
    def raw_check():
        nonlocal raw
        raw = {p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()}
        for row in phases:
            for key in ('command', 'stdout', 'stderr'):
                require(raw.get(Path(row[key]['path']).name) == row[key], 'raw phase join')
    check('raw evidence', raw_check)
    failure = failure or ('postcheck failed' if errors else None)
    if time.monotonic() >= deadline:
        failure = failure or 'whole deadline exceeded'
    result = dict(schema='ferric-guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-v1', passed=failure is None, failure=failure,
        source_generation='scoped-tail-parent-v2', shared_test_fixture_repaired=True,
        postcheck_errors=errors, controller=h.pin(ROOT / 'run_cpu.py'), supervisor=h.pin(ROOT / 'supervisor.py'),
        input_manifest=input_pin, readset=readset, git_revision=inputs['git_revision'] if inputs else None,
        preformat_sources=before, input_sources=formatted, final_sources=after,
        source_unchanged=formatted is not None and after == formatted, format_changed_paths=changed,
        phases=phases, tests=tests, inventory=listed, artifacts=artifacts, metadata=metadata,
        tool_pins=tool_pins, parent_toolchain_observation=parent_tools,
        cache_provenance=cache, configurations=config, raw=raw,
        elapsed_seconds=time.monotonic() - started,
        limits=dict(whole_seconds=WHOLE_WALL, leaf_seconds=LEAF_WALL, cleanup_reserve_seconds=CLEANUP_RESERVE,
            address_space_bytes=h.AS_LIMIT, cache_bytes=h.CACHE_LIMIT, stream_bytes=h.STREAM_LIMIT,
            initial_free_bytes=h.START_FREE, live_free_bytes=h.LIVE_FREE, affinity=[8, 9], cargo_jobs=2),
        full_parent_library_suite_executed=False,
        all_selected_parent_tests_executed=len(tests) == 58 and sum(v['passed'] for v in tests.values()) == 568,
        readiness40_bank_scoped_warm_parent_source_added=True,
        readiness40_bank_scoped_warm_native_execution=False, allocation_preflights_changed=True,
        readiness40_bank_scoped_census_tail_parent_source_added=True,
        readiness40_bank_scoped_census_tail_policy_prepublication_checked=True,
        readiness40_bank_scoped_census_tail_native_execution=False,
        readiness40_bank_scoped_census_parent_source_added=True,
        readiness40_bank_scoped_census_native_execution=False,
        allocation_preflights_changed_only_for_explicit_warm_position5=False,
        allocation_preflights_changed_only_for_explicit_warm_position5_or_full=True,
        full2303_bank_scoped_census_parent_source_added=True,
        full2303_bank_scoped_census_native_execution=False, full2303_deadline_changed=False,
        full2303_bank_scoped_census_policy_prepublication_checked=True,
        readiness40_scoped_warm_parent_source_added=True, readiness40_scoped_warm_native_execution=False,
        scoped_warm_host_timing_parent_source_added=True, currentness_temporal_equivalence_claim=False,
        readiness40_shared_full_parent_route_added=True, readiness40_shared_full_native_execution=False,
        shared_full_host_timing_parent_source_added=True, shared_full_host_timing_native_execution=False,
        selected_readiness_currentness_policy_changed=True, qualified_worker_sources_preserved=True,
        worker_qualification=WORKER_COMPLETE, worker_source_manifest=WORKER_SOURCES,
        parent_host_timing_source_added=True, parent_host_timing_native_execution=False,
        parent_host_timing_rows=40, parent_host_timing_disjoint_spans=124,
        ordinary_wire_schema_changed=False, ordinary_observation_schema_changed=False,
        full2303_scoped_warm_parent_source_added=True, full2303_scoped_warm_native_execution=False,
        full2303_scoped_policy_prepublication_checked=True, full2303_launch_admitted=False,
        full2303_parent_route_added=True, full2303_native_execution=False,
        full2303_launch_feasibility=False, full2303_source_abort_ms=3600000,
        inherited_causal_layer0_parent_route_preserved=True,
        causal_layer0_parent_route_added=True, causal_layer0_native_execution=False,
        parent_causal_file_reads_retained=True,
        causal_capture_positions=list(range(6)), causal_layer=0,
        inherited_warm_paired_terminal_parent_route_preserved=True,
        warm_paired_terminal_parent_route_added=True, warm_paired_terminal_native_execution=False,
        selected_terminal_cadence_changed=True, global_currentness_policy_changed=False,
        hidden_read_policy_changed=False,
        position5_diagnostic_parent_route_added=True, position5_native_execution=False,
        inherited_readiness_parent_route_preserved=True, readiness_native_execution=False,
        inherited_paired_read_parent_route_preserved=True, paired_read_native_execution=False,
        method_local_currentness_cadence_changed=True, read_ns_scope_changed=False,
        inherited_reusable_arena_parent_route_added=True, reusable_arena_native_execution=False,
        default_arena_policy_changed=False, full_long_workload=False,
        inherited_shared_full_currentness_source_added=True,
        inherited_host_observation_source_added=True, inherited_capture_source_added=True,
        performance_policy_changed=False, default_policy_changed=False,
        runtime_suite_rerun=False, worker_suite_rerun=False, gpu_execution=False,
        full_model_acceptance=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - time.monotonic()))
    try:
        h.save('complete.json' if failure is None else 'failed.json', result)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(passed=failure is None, failure=failure, phases=len(phases),
        tests_passed=sum(v['passed'] for v in tests.values()), output=str(OUT)), sort_keys=True))
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
