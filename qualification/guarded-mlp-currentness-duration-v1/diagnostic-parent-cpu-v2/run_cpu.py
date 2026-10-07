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
EXPECTED_ROOT = Path("/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-currentness-duration-diagnostic-parent-cpu-v228-v2")
FERRIC = ROOT / 'ferric'
PARENT_REL = 'adapters/m1-engineering-execution-v1'
PARENT = FERRIC / PARENT_REL
CARGO_HOME = ROOT / 'cargo-home'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
SUPPORT_SHA = '71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13'
BASE_COMPLETE = {"bytes":4077579,"sha256":"6e61fbbd1b1e82d97102e311272a947744858a7123ca7c73062b656bc8bdaea2"}
BASE_SOURCES = {"bytes":521263,"sha256":"3f451da9914922109ffc0b232fe276f99aca98ecce3723fc7285faeb9fa6feea"}
PROPOSAL_PINS = {"worker-proposal.json":{"bytes":25694,"sha256":"c6224930fef73622d36b2634f5ef88846db8c1b55e9a7e3fdd550247e5024e52"},"worker-proposal-v1.json":{"bytes":22283,"sha256":"e7a0c23826acc56b83b53fb254b7f17fef9e6f245b04d80ea98b66d71f6d7e9f"},"worker-failed-v1.json":{"bytes":2267201,"sha256":"a3f48e67451287afca3ee91000209a2c3d9c65b693659a4afa73882ab2615d8b"},"runtime-proposal.json":{"bytes":14821,"sha256":"85cd4a641b0e7429f3eeb8aebe310e56a64487b467ca5e81431002c4a7ffc8aa"},"interface.md":{"bytes":3488,"sha256":"9c724086a502043303601f7fe32036f67abf84ebb4ff76c4e524b1544e315303"}}
BASE_WORKER_COMPLETE = {"bytes":2498847,"sha256":"f25bd6d9f582a4e632c05e385961c260c0c95d2a119bac5af4927af6dc3b61ed"}
WORKER_COMPLETE = {"bytes":2569774,"sha256":"5a27b9983ecd0d523226cbf2db4070108cd6deaf75d2ee6fe528274d2f8a5ac2"}
BASE_WORKER_SOURCES = {"bytes":435848,"sha256":"eda7f5f696a5b93d048923cd5f8bd83b51a5ca9cb77efab8322da97f4286e928"}
WORKER_SOURCES = {"bytes":450252,"sha256":"74bcd0c27c264faf5991e8b13f8dcd2b9f52d42ad1d9b3e9a30350ee2c1baa0c"}
WORKER_ROOT = "/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-currentness-duration-diagnostic-cpu-v228-v2"
WORKER_PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
LOCK_SHA = 'ec06e964ed72dc9b97d9f5769bd867bf05398781b6e17a9f605137742d6e19ea'
CACHE_PIN = dict(bytes=96226, sha256='984b2a893ba64f35cda1cdceb5d379028416c00c36815c678200480c7e8904ad')
PARENT_TOOLS_SHA = 'c38e83e58d01e60b7311efa1ff53cb94e69dbbcd227ee30bde6a04c7928590d9'
PARENT_TOOLCHAIN = Path('/home/harmenon/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin')
PACKAGE = 'ferric-m1-engineering-execution-v1'
BINARY = 'ferric-qwen3-finite-guarded-mlp-decode-engineering'
ADDED_BINARY = 'ferric-qwen3-guarded-mlp-full2303-engineering'
MODE = "diagnostic"
DIAGNOSTIC = MODE == 'diagnostic'
DIAGNOSTIC_FEATURE = 'engineering-currentness-duration-diagnostics'
EXPECTED_PHASES, EXPECTED_SCOPES = (69, 59) if DIAGNOSTIC else (68, 58)
EXPECTED_PASSED, EXPECTED_INVENTORY = (595, 1058) if DIAGNOSTIC else (584, 1047)
OLD_FEATURE = 'guarded-mlp-readiness-engineering'
FULL_FEATURE = 'guarded-mlp-full2303-engineering'
BASE_FEATURE = OLD_FEATURE + ',' + FULL_FEATURE
FEATURE = BASE_FEATURE + (',' + DIAGNOSTIC_FEATURE if DIAGNOSTIC else '')
OLD_ROOT = "/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-full2303-scoped-tail-parent-cpu-v228-v1"
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
        'baseline-worker-complete.json': BASE_WORKER_COMPLETE})
    bodies = {name: read(h, name, row, readset) for name, row in inputs['lineage'].items()}
    require(all(compact(readset[name]) == row for name, row in fixed.items()), 'literal actual parent/worker/source pins')
    base = json.loads(bodies['parent-complete.json'])
    old_map = json.loads(bodies['parent-sources.json'])
    proposal = json.loads(bodies['worker-proposal.json'])
    runtime_proposal = json.loads(bodies['runtime-proposal.json'])
    worker = json.loads(bodies['worker-complete.json'])
    worker_map = json.loads(bodies['worker-sources.json'])
    baseline_worker = json.loads(bodies['baseline-worker-complete.json'])
    base_worker_map = json.loads(bodies['baseline-worker-sources.json'])
    require(base['schema'] == 'ferric-guarded-mlp-full2303-scoped-tail-parent-cpu-v1'
            and base['source_generation'] == 'full2303-scoped-tail-parent-v1'
            and len(old_map) == 1298 and len(base['phases']) == 68 and len(base['tests']) == 58
            and len(base['inventory']) == 1047 and sum(v['passed'] for v in base['tests'].values()) == 584
            and base['worker_qualification'] == BASE_WORKER_COMPLETE
            and base['worker_source_manifest'] == BASE_WORKER_SOURCES
            and base['full2303_scoped_tail_parent_source_added'] is True
            and base['full2303_scoped_tail_policy_prepublication_checked'] is True
            and base['readiness40_bank_scoped_census_tail_parent_source_added'] is True
            and base['readiness40_bank_scoped_census_tail_policy_prepublication_checked'] is True
            and base['qualified_worker_sources_preserved'] is True
            and base['parent_host_timing_rows'] == 40 and base['parent_host_timing_disjoint_spans'] == 124
            and len(base['artifacts']) == 7, 'actual584 selected Full Tail parent baseline')
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
                    and p['observed_signals'] == [] for p in receipt['phases']),
                'actual clean source/CPU lifecycle')
    require(baseline_worker['schema'] == 'ferric-guarded-mlp-full2303-scoped-tail-cpu-v1'
            and baseline_worker['source_generation'] == 'full2303-scoped-tail-coupled-v1'
            and len(base_worker_map) == 1063
            and (baseline_worker['tests']['worker-tests']['passed'], baseline_worker['tests']['worker-tests']['ignored']) == (838, 4)
            and (baseline_worker['tests']['kfd-tests']['passed'], baseline_worker['tests']['kfd-tests']['ignored']) == (1180, 8),
            'actual direct Full Tail coupled baseline')
    require(proposal['schema'] == 'ferric-readiness40-tail-currentness-duration-worker-parent-source-v1'
            and proposal['source_only'] is True and len(proposal['files']) == 16
            and proposal['features']['worker'] == dict(name=DIAGNOSTIC_FEATURE,
                dependencies=['fe2o3-kfd/' + DIAGNOSTIC_FEATURE])
            and proposal['features']['parent'] == dict(name=DIAGNOSTIC_FEATURE,
                dependencies=[OLD_FEATURE])
            and proposal['features']['default_features_changed'] is False
            and proposal['features']['locks_changed'] is False
            and all(proposal['guarantees'][k] is False for k in ('new_execution_authority',
                'currentness_policy_changed', 'native_execution', 'numerical_acceptance',
                'performance_claim', 'canonical_changed', 'compiled', 'tests_executed',
                'formatted', 'ambient_environment_flag', 'new_selector', 'new_runtime_facade')),
            'fixed duration source and feature-only nonauthority')
    require(any(compact(p) == BASE_COMPLETE for p in proposal['requires'])
            and any(compact(p) == BASE_SOURCES for p in proposal['requires'])
            and any(compact(p) == BASE_WORKER_COMPLETE for p in proposal['requires'])
            and any(compact(p) == BASE_WORKER_SOURCES for p in proposal['requires'])
            and any(compact(p) == PROPOSAL_PINS['runtime-proposal.json'] for p in proposal['requires'])
            and compact(proposal['interface']) == PROPOSAL_PINS['interface.md'],
            'original parent/worker/runtime/interface dependency joins')
    prior_proposal = json.loads(bodies['worker-proposal-v1.json'])
    failed_v1 = json.loads(bodies['worker-failed-v1.json'])
    repair = proposal['repair']
    repair_path = 'adapters/tp-peer-finite-engineering-worker-v1/src/native_guarded_mlp_readiness_cli_v1.rs'
    require(proposal['source_generation'] == 'currentness-duration-worker-parent-v2-cfg-assignment-block'
            and compact(proposal['predecessor_manifest']) == PROPOSAL_PINS['worker-proposal-v1.json']
            and compact(repair['failed_attempt']) == PROPOSAL_PINS['worker-failed-v1.json']
            and prior_proposal['schema'] == proposal['schema']
            and prior_proposal['source_only'] is True
            and all(proposal[k] == prior_proposal[k] for k in
                ('features', 'source_census', 'test_contract', 'guarantees', 'unchanged_contracts'))
            and compact(prior_proposal['interface']) == PROPOSAL_PINS['interface.md'],
            'immutable original source and exact cumulative syntax repair')
    old_rows = {r['path']: r for r in prior_proposal['files']}
    new_rows = {r['path']: r for r in proposal['files']}
    require(len(old_rows) == len(prior_proposal['files']) == len(new_rows) == len(proposal['files']) == 16
            and set(old_rows) == set(new_rows)
            and all(new_rows[n]['before'] == old_rows[n]['before']
                    and new_rows[n]['repository'] == old_rows[n]['repository'] for n in old_rows)
            and [n for n in old_rows if old_rows[n]['after'] != new_rows[n]['after']] == [repair_path]
            and repair['path'] == repair_path
            and compact(repair['v1_after']) == old_rows[repair_path]['after']
            and compact(repair['v2_after']) == new_rows[repair_path]['after']
            and repair['before_text'] == "            #[cfg(not(feature = \"engineering-currentness-duration-diagnostics\"))]\n            self.tail_counts = Some(owner.close_with_tail_scoped(request, digest)?);"
            and repair['after_text'] == "            #[cfg(not(feature = \"engineering-currentness-duration-diagnostics\"))]\n            {\n                self.tail_counts = Some(owner.close_with_tail_scoped(request, digest)?);\n            }"
            and repair['test_contract_identical_to_predecessor'] is True
            and repair['runtime_changed_from_predecessor'] is False
            and repair['unstable_language_feature_added'] is False,
            'only the attributed assignment gains a block; other fifteen rows and all tests unchanged')
    failed_phases = failed_v1['phases']
    require(failed_v1['schema'] == 'ferric-guarded-mlp-currentness-duration-cpu-v1'
            and failed_v1['qualification_mode'] == 'default'
            and failed_v1['source_generation'] == 'currentness-duration-default-coupled-v1'
            and failed_v1['passed'] is False and type(failed_v1['failure']) is str
            and failed_v1['postcheck_errors'] == []
            and len(failed_phases) == 23
            and [p['exit_code'] for p in failed_phases] == [0] * 22 + [101]
            and failed_phases[-1]['label'] == 'worker-tests-build'
            and all(p['natural_exit'] is True and p['reaped'] is True
                    and p['process_group_absent'] is True and p['forced_cleanup'] is False
                    and p['timed_out'] is False and p['exception'] is None
                    and p['storage_failure'] is None and p['observed_signals'] == [] for p in failed_phases)
            and compact(failed_v1['readset']['worker-proposal.json']) == PROPOSAL_PINS['worker-proposal-v1.json']
            and repair['compiler_stderr_from_terminal'] == failed_phases[-1]['stderr']
            and 'worker-tests' not in failed_v1['tests'],
            'original default failure retained as ancestry, never successful qualification')
    require(runtime_proposal['schema'] == 'ferric-currentness-duration-runtime-source-v1'
            and runtime_proposal['feature'] == DIAGNOSTIC_FEATURE
            and len(runtime_proposal['files']) == 10
            and runtime_proposal['test_contract']['feature_off_new_tests'] == []
            and len(runtime_proposal['test_contract']['feature_on_new_tests']) == 12,
            'fixed duration runtime source and two-mode test contract')
    require(worker['schema'] == 'ferric-guarded-mlp-currentness-duration-cpu-v1'
            and worker['qualification_mode'] == MODE
            and worker['source_generation'] == 'currentness-duration-' + MODE + '-coupled-v2'
            and len(worker_map) == 1069 and len(worker['artifacts']) == 11
            and worker['selected_runtime_features'] == sorted(['default', 'engineering-gfx950']
                + ([DIAGNOSTIC_FEATURE] if DIAGNOSTIC else []))
            and worker['selected_worker_features'] == ([DIAGNOSTIC_FEATURE] if DIAGNOSTIC else [])
            and worker['currentness_duration_diagnostic_build'] is DIAGNOSTIC
            and worker['default_check_no_default_features'] is True
            and worker['default_check_diagnostic_feature_requested'] is DIAGNOSTIC
            and worker['currentness_duration_runtime_source_added'] is True
            and worker['currentness_duration_worker_source_added'] is True
            and worker['currentness_duration_native_execution'] is False
            and worker['runtime_source_changed'] is True
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and worker['artifacts']['worker']['cargo_artifact']['features']
                == ([DIAGNOSTIC_FEATURE] if DIAGNOSTIC else [])
            and (worker['tests']['worker-tests']['passed'], worker['tests']['worker-tests']['failed'],
                 worker['tests']['worker-tests']['ignored']) == (853 if DIAGNOSTIC else 838, 0, 4)
            and (worker['tests']['kfd-tests']['passed'], worker['tests']['kfd-tests']['failed'],
                 worker['tests']['kfd-tests']['ignored']) == (1192 if DIAGNOSTIC else 1180, 0, 8)
            and all(worker[k] is False for k in ('full2303_scoped_tail_native_execution',
                'readiness40_bank_scoped_census_tail_native_execution', 'full2303_launch_admitted',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'numerical_acceptance', 'performance_claim')), 'matched actual mode and tested executable')
    require(compact(worker['readset']['worker-proposal.json']) == PROPOSAL_PINS['worker-proposal.json']
            and compact(worker['readset']['runtime-proposal.json']) == PROPOSAL_PINS['runtime-proposal.json']
            and compact(worker['readset']['baseline-complete.json']) == BASE_WORKER_COMPLETE
            and compact(worker['readset']['baseline-sources.json']) == BASE_WORKER_SOURCES,
            'actual duration coupled direct proposal and predecessor readset')
    for label, added in (('worker-tests', proposal['test_contract']['feature_on_new_tests']),
                        ('kfd-tests', runtime_proposal['test_contract']['feature_on_new_tests'])):
        old_names = {r['name']: r['outcome'] for r in baseline_worker['tests'][label]['named']}
        new_names = added if DIAGNOSTIC else []
        require(len(new_names) == len(set(new_names)) and not set(old_names) & set(new_names)
                and {r['name']: r['outcome'] for r in worker['tests'][label]['named']}
                    == dict(old_names, **{n: 'ok' for n in new_names}),
                'all actual old runtime/worker names and exact diagnostic additions')
    runtime_rows = {'fe2o3/' + r['path']: r for r in runtime_proposal['files']}
    prior_runtime = {n: compact(r) for n, r in base_worker_map.items() if n.startswith('fe2o3/')}
    require(len(prior_runtime) == 827 and len(runtime_rows) == 10
            and sum(r['before'] is None for r in runtime_rows.values()) == 2, 'runtime source extent')
    for n, r in runtime_rows.items():
        require(prior_runtime.get(n) == r['before'] and r['repository'] == 'fe2o3', 'runtime preimage')
        prior_runtime[n] = r['after']
    require({n: compact(r) for n, r in worker['preformat_sources'].items() if n.startswith('fe2o3/')}
                == prior_runtime and len(prior_runtime) == 829
            and {n for n in worker_map if n.startswith('fe2o3/')} == set(prior_runtime)
            and all(compact(worker_map[n]) == r for n, r in prior_runtime.items()
                    if n not in runtime_rows or not n.endswith('.rs')),
            'only reviewed runtime Rust formatting, exact Cargo features')
    worker_rows = {'ferric/' + r['path']: r for r in proposal['files']
                   if ('ferric/' + r['path']).startswith(WORKER_PREFIX)}
    parent_rows = {'ferric/' + r['path']: r for r in proposal['files']
                   if r['path'].startswith(PARENT_REL + '/')}
    require(len(worker_rows) == 12 and len(parent_rows) == 4
            and set(worker_rows) | set(parent_rows) == {'ferric/' + r['path'] for r in proposal['files']}
            and sum(r['before'] is None for r in worker_rows.values()) == 4
            and all(r['before'] is not None for r in parent_rows.values())
            and all(r['repository'] == 'ferric' for r in proposal['files'])
            and all(n.endswith('.rs') or n.endswith('/Cargo.toml') for n in (*worker_rows, *parent_rows)),
            'exact twelve worker and four parent rows including two Cargo features')
    old = {n: compact(r) for n, r in old_map.items() if n.startswith('ferric/')}
    require(len(old) == 1295 and sum(n.startswith(WORKER_PREFIX) for n in old) == 232
            and all(old_map[n]['path'] == OLD_ROOT + '/' + n for n in old),
            'actual1295 parent baseline with232 worker bodies')
    pre_worker = {n: r for n, r in old.items() if n.startswith(WORKER_PREFIX)}
    require(pre_worker == {n: compact(r) for n, r in base_worker_map.items() if n.startswith(WORKER_PREFIX)},
            'entire actual parent and coupled worker source agreement')
    for n, r in worker_rows.items():
        require(pre_worker.get(n) == r['before'], 'worker preimage or absence')
        pre_worker[n] = r['after']
    qualified_worker = {n: compact(r) for n, r in worker_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(pre_worker) == len(qualified_worker) == 236 and set(pre_worker) == set(qualified_worker)
            and {n: compact(r) for n, r in worker['preformat_sources'].items() if n.startswith(WORKER_PREFIX)} == pre_worker
            and all(qualified_worker[n] == r for n, r in pre_worker.items()
                    if n not in worker_rows or not n.endswith('.rs'))
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n for n in qualified_worker),
            'all236 actually qualified worker bodies, only eleven reviewed Rust formatting paths')
    expected = {n: r for n, r in old.items() if not n.startswith(WORKER_PREFIX)}
    expected.update(qualified_worker)
    for n, r in parent_rows.items():
        require(expected.get(n) == r['before'], 'parent preimage')
        expected[n] = r['after']
    require(len(expected) == 1299 and not set(worker_rows) & set(parent_rows), '1299 closed Ferric composition')
    rows = dict(worker_rows, **parent_rows)
    scopes = base['tests']
    expected_lineage = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    expected_lineage |= {name + suffix for name in scopes for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == expected_lineage and len(bodies) == 129, 'closed129 direct lineage bodies')
    for name in expected_lineage - set(fixed):
        require(compact(readset[name]) == compact(base['raw'][name]), 'actual baseline raw ' + name)
    listed = core.inventory(bodies['parent-lib-list.stdout'].decode())
    require(listed == base['inventory'], 'actual1047-name parent inventory')
    for name, prior in scopes.items():
        require(core.outcomes(bodies[name + '.stdout'].decode()) == prior
                and prior['failed'] == prior['ignored'] == 0, 'preserved raw selected outcomes ' + name)
        command = json.loads(bodies[name + '.command.json'])
        phase = next(p for p in base['phases'] if p['label'] == name)
        require(command['argv'] == phase['argv'], 'actual selected command join')
    require({n: compact(r) for n, r in before.items() if n.startswith('ferric/')} == expected
            and len(before) == 1302, 'complete actual baseline, qualified worker and four parent overlays')
    require(inputs['parent_overlay'] == sorted(n for n in parent_rows if n.endswith('.rs')),
            'only three parent Rust formatting inputs; Cargo manifest stays exact')
    require(h.pin(PARENT / 'Cargo.lock')['sha256'] == LOCK_SHA, 'unchanged parent lock')
    test_groups = (
        (PARENT / 'src/tp_finite_client/long/readiness_bank_scoped_census_tail_tests.rs',
         'tp_finite_client::long::readiness::bank_scoped_census_tail::tests::', 6, 2),
        (FERRIC / 'adapters/tp-peer-finite-engineering-worker-v1/src/finite_guarded_mlp_readiness_currentness_durations_v1_tests.rs',
         'finite_guarded_mlp_readiness_currentness_durations_v1::tests::', 9, 9))
    declared_new = []
    for path, prefix, total, added in test_groups:
        declared = [prefix + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', path.read_text())]
        new = [n for n in declared if n not in listed]
        require(len(declared) == len(set(declared)) == total and len(new) == added,
                'exact source-declared old and feature-gated methods')
        declared_new.extend(new)
    require(len(declared_new) == len(set(declared_new)) == 11
            and sorted(declared_new) == proposal['test_contract']['parent_feature_on_new_tests'],
            'exact eleven diagnostic library instances')
    additions = sorted(declared_new) if DIAGNOSTIC else []
    bin_names = []
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
    expected = relocate(old)
    packages = [p for p in expected['packages'] if p['name'] == PACKAGE]
    require(len(packages) == 1 and DIAGNOSTIC_FEATURE not in packages[0]['features'],
            'one predecessor parent package without diagnostic feature')
    packages[0]['features'][DIAGNOSTIC_FEATURE] = [OLD_FEATURE]
    nodes = [n for n in expected['resolve']['nodes'] if n['id'] == packages[0]['id']]
    require(len(nodes) == 1 and DIAGNOSTIC_FEATURE not in nodes[0]['features'], 'one parent resolution node')
    if DIAGNOSTIC:
        nodes[0]['features'] = sorted(nodes[0]['features'] + [DIAGNOSTIC_FEATURE])
    require(current == expected, 'only diagnostic feature declaration/activation; locked graph unchanged')
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
        require(set(inputs) == {'schema', 'qualification_mode', 'files', 'lineage', 'parent_overlay', 'cache_manifest', 'git_revision'}
                and inputs['schema'] == 'ferric-guarded-mlp-currentness-duration-parent-cpu-input-v1'
                and inputs['qualification_mode'] == MODE, 'closed matched-mode parent input')
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
        require(listed == sorted(old_list + additions) and len(listed) == EXPECTED_INVENTORY, 'old1047 plus exact mode-specific library names')
        selected = set()
        list_labels = {'guarded-bin-list': 'guarded-bin-tests', 'readiness-bin-list': 'readiness-bin-tests',
                       'full2303-bin-list': 'full2303-bin-tests'}
        ordered_scopes = [p['label'] for p in base['phases'] if p['label'] in scopes or p['label'] in list_labels]
        require(len(ordered_scopes) == 61 and set(ordered_scopes) == set(scopes) | set(list_labels),
                'all58 original selected and three inventory recipes')
        for label in ordered_scopes:
            original = next(p for p in base['phases'] if p['label'] == label)
            argv = relocate(original['argv'][6:])
            require(argv[:2] == [cargo, 'test'] and argv[argv.index('--features') + 1] == BASE_FEATURE,
                    'unchanged original owned Cargo recipe')
            argv[argv.index('--features') + 1] = FEATURE
            leaf(label, argv, source=formatted)
            if label in list_labels:
                names = set(core.statuses(scopes[list_labels[label]]))
                if label == 'full2303-bin-list':
                    names |= set(bin_names)
                require(core.inventory((OUT / (label + '.stdout')).read_text()) == sorted(names),
                        'preserved named binary inventory plus explicit addition')
                continue
            names = set(core.statuses(scopes[label]))
            if label == 'full2303-bin-tests':
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
        if DIAGNOSTIC:
            label = 'parent-currentness-duration-policy'
            selector = 'finite_guarded_mlp_readiness_currentness_durations_v1::tests::'
            names = {n for n in additions if n.startswith(selector)}
            require(len(names) == 9 and not selected & names, 'closed new pure diagnostic scope')
            leaf(label, [cargo, 'test', *common, *feature, '--lib', selector,
                         '--', '--nocapture', '--test-threads=1'], source=formatted)
            value = core.outcomes((OUT / (label + '.stdout')).read_text())
            require(core.statuses(value) == {n: 'ok' for n in names}
                    and value['failed'] == value['ignored'] == 0, 'nine exact diagnostic pure cases')
            tests[label] = value
            selected |= names
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
                    and row['features'] == sorted(base['artifacts'][name]['cargo_artifact']['features']
                        + ([DIAGNOSTIC_FEATURE] if DIAGNOSTIC else [])) and row['filenames'].count(str(path)) == 1, 'new target artifact')
            with path.open('rb') as stream:
                require(stream.read(4) == b'\x7fELF', 'parent ELF')
            artifacts[name] = dict(pin=h.pin(path), cargo_artifact=row)
        leaf('parent-default-check', [cargo, 'check', *common, '--lib', '--message-format=json'], source=formatted)
        require(len(phases) == EXPECTED_PHASES and len(tests) == EXPECTED_SCOPES and sum(v['passed'] for v in tests.values()) == EXPECTED_PASSED
                and sum(v['failed'] + v['ignored'] for v in tests.values()) == 0, 'complete matched-mode phase and selected-test census')
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
    result = dict(schema="ferric-guarded-mlp-currentness-duration-parent-cpu-v1", passed=failure is None, failure=failure,
        source_generation='currentness-duration-' + MODE + '-parent-v2', qualification_mode=MODE,
        currentness_duration_parent_source_added=True, currentness_duration_diagnostic_build=DIAGNOSTIC,
        currentness_duration_native_execution=False, selected_parent_features=FEATURE.split(','),
        shared_test_fixture_repaired=True,
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
        all_selected_parent_tests_executed=len(tests) == EXPECTED_SCOPES and sum(v['passed'] for v in tests.values()) == EXPECTED_PASSED,
        readiness40_bank_scoped_warm_parent_source_added=True,
        readiness40_bank_scoped_warm_native_execution=False, allocation_preflights_changed=True,
        full2303_scoped_tail_parent_source_added=True,
        full2303_scoped_tail_policy_prepublication_checked=True,
        full2303_scoped_tail_native_execution=False,
        inherited_readiness40_tail_parent_preserved=True,
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
