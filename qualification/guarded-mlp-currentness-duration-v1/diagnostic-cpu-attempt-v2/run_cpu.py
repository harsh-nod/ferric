"""Coupled routed-runtime and activated-worker CPU qualification using owned leaves."""
from collections import Counter
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
import types

ROOT = Path(__file__).resolve().parent
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
EXPECTED_ROOT = E / 'guarded-mlp-currentness-duration-diagnostic-cpu-v228-v2'
RUNTIME = ROOT / 'fe2o3'
WORKER_REL = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
WORKER = ROOT / WORKER_REL
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
CARGO_HOME = ROOT / 'cargo-home'
SUPPORT_SHA = '4d12ba86c4014adaaea1c5c49c3790dffcca78af8326ae09cb085840347a518a'
CACHE_SHA = 'c05cc2043d38a78f4bb307ec7693d682b05c8a47cb98ab898fb848a7f0262545'
WORKER_PROPOSAL_SHA = 'c6224930fef73622d36b2634f5ef88846db8c1b55e9a7e3fdd550247e5024e52'
SELECTOR_SCHEMA = 'ferric-readiness40-tail-currentness-duration-worker-parent-source-v1'
SELECTOR_ROWS = 12
WORKER_FILES = 236
WORKER_SUMMARY_ADDITIONS = [15, 0, 0, 0]
TOTAL_SOURCES = 1069
GENERATION = 'currentness-duration-diagnostic-coupled-v2'

MODE = 'diagnostic'
DIAGNOSTIC = True
FEATURE = 'engineering-currentness-duration-diagnostics'
RUNTIME_PROPOSAL_SHA = '85cd4a641b0e7429f3eeb8aebe310e56a64487b467ca5e81431002c4a7ffc8aa'
RUNTIME_DIAGNOSTIC_NAMES = ['device::gfx950::scoped_currentness::tests::duration_tests::duration_backwards_clock_refuses_after_callback_without_later_observation', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_clock_unwind_before_and_after_each_callback_never_commits', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_count_mismatch_and_category_sum_refuse_before_finish_commit', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_each_call_and_elapsed_overflow_refuses_under_engine_guard', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_elapsed_and_bank_subtotal_bounds_are_checked_integer_data', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_every_callback_fault_keeps_original_prefix_and_quarantine', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_every_callback_unwind_keeps_original_prefix_and_quarantine', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_final_deadline_is_rechecked_after_reconciliation', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_production_hooks_and_armed_publication_boundaries_are_source_bound', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_real_adapter_matches_unmeasured_predicate_and_deadline_order', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_selected_rank_and_group_checkpoints_share_one_recorder', 'engineering_gfx950::peer::combined_mlp_state_v1::paired::retained::mixed_bank::scoped::custody_tests::scoped_bank_duration_refusal_and_unwind_preserve_outer_custody']
WORKER_DIAGNOSTIC_NAMES = ['finite_guarded_mlp_readiness_currentness_durations_v1::tests::duration_record_checks_bank_subtotal_overflow_and_body_containment', 'finite_guarded_mlp_readiness_currentness_durations_v1::tests::duration_record_does_not_accept_changed_original_policy_or_out_of_bound_ns', 'finite_guarded_mlp_readiness_currentness_durations_v1::tests::duration_record_joins_each_scope_to_original_policy_counts', 'finite_guarded_mlp_readiness_currentness_durations_v1::tests::duration_record_joins_policy_worker_transcript_and_session_identity', 'finite_guarded_mlp_readiness_currentness_durations_v1::tests::duration_record_refuses_bank_row_drift_even_when_global_totals_cancel', 'finite_guarded_mlp_readiness_currentness_durations_v1::tests::duration_record_refuses_claim_type_unknown_duplicate_and_noncanonical_bytes', 'finite_guarded_mlp_readiness_currentness_durations_v1::tests::duration_record_refuses_missing_duplicate_reordered_or_measured_first_use_rows', 'finite_guarded_mlp_readiness_currentness_durations_v1::tests::duration_record_requires_exact_two_canonical_bounded_records', 'finite_guarded_mlp_readiness_currentness_durations_v1::tests::duration_record_roundtrips_two_original_lines_without_policy_laundering', 'native_catalog::forward::guarded_mlp_decode_v1::readiness::tail_scoped::durations::tests::duration_state_bad_bank_duration_is_terminal_before_any_layer', 'native_catalog::forward::guarded_mlp_decode_v1::readiness::tail_scoped::durations::tests::duration_state_layer_count_mismatch_and_overflow_cannot_commit', 'native_catalog::forward::guarded_mlp_decode_v1::readiness::tail_scoped::durations::tests::duration_state_missing_metrics_first_use_and_wrong_positions_refuse_without_fallback', 'native_catalog::forward::guarded_mlp_decode_v1::readiness::tail_scoped::durations::tests::duration_state_records_exact_forty_rows_and_two_unmeasured_forwards', 'native_catalog::forward::guarded_mlp_decode_v1::readiness::tail_scoped::durations::tests::duration_state_tail_metric_error_prevents_forward_and_close_publication', 'native_catalog::forward::guarded_mlp_decode_v1::readiness::tail_scoped::durations::tests::duration_state_unwind_at_each_collection_boundary_stays_terminal']
RUNTIME_FEATURES = ['default', 'engineering-currentness-duration-diagnostics', 'engineering-gfx950'] if DIAGNOSTIC else ['default', 'engineering-gfx950']
WORKER_FEATURES = [FEATURE] if DIAGNOSTIC else []
WHOLE_WALL, LEAF_WALL, CLEANUP_RESERVE = 3600, 1800, 50
HISTORY = {'baseline-complete.json': [2498847, 'f25bd6d9f582a4e632c05e385961c260c0c95d2a119bac5af4927af6dc3b61ed'], 'baseline-sources.json': [435848, 'eda7f5f696a5b93d048923cd5f8bd83b51a5ca9cb77efab8322da97f4286e928'], 'runtime-tests.stdout': [138625, '32717a9bd5e4d4297cceb875d09e5e0d69f965baa4946b94239ba5ebd5e39cee'], 'runtime-list.stdout': [130138, 'a0950df723eff88f08d77f6b24f34823a85be015721c42b0d7a0725cbf381381'], 'worker-tests.stdout': [103009, 'e60b1a9ba0d2705b515280d6fed67533b68120db28619ab579655926879468ef'], 'worker-list.stdout': [97213, '43d187849c8ee68a38ff1b075e64f8e3ce833907b0b21e1d5d170b47ae817457'], 'runtime-docs.stdout': [2610, 'ae232b746889e201c68b98493db857a97d110dabb9dd32cee7fdaa11a5d0b8d8'], 'legacy-docs.stdout': [2161, '95b8958d401e6e40ab05df7310915287a68270d301e7164826349c1c068c52d5']}
KFD_TARGETS = {
    'kfd-lib': ('fe2o3_kfd', 'lib', 'src/lib.rs'),
    'engineering-worker-test': ('fe2o3-gfx950-engineering-worker', 'bin', 'src/bin/gfx950_engineering_worker.rs'),
    'guarded-facade-test': ('guarded_mlp_facade_v1', 'test', 'tests/guarded_mlp_facade_v1.rs'),
    'debug-trap-test': ('kfd_debug_trap_live', 'test', 'tests/kfd_debug_trap_live.rs'),
    'telemetry-env-test': ('target_debug_telemetry_env_v1', 'test', 'tests/target_debug_telemetry_env_v1.rs'),
    'telemetry-test': ('target_debug_telemetry_v1', 'test', 'tests/target_debug_telemetry_v1.rs'),
}
PAIRED = 'engineering_gfx950::peer::combined_mlp_state_v1::paired::'
FOCUS = {
    'terminal-pair-tests': PAIRED + 'terminal_pair::tests::',
    'peer-read-pair-tests': 'engineering_gfx950::peer::read_pair_v1::tests::',
    'arena-reuse-tests': PAIRED + 'arena::retired::reuse::tests::',
    'arena-memory-tests': 'memory_linux::paired_arena_reuse_v1::tests::',
    'retained-tests': PAIRED + 'retained::tests::',
    'mixed-bank-tests': PAIRED + 'retained::mixed_bank::',
    'scoped-checkpoint-tests': 'device::gfx950::scoped_currentness::tests::',
    'scoped-group-tests': 'engineering_gfx950::peer::scoped_currentness::tests::',
    'scoped-layer-tests': PAIRED + 'retained::scoped_layer::',
    'scoped-tail-tests': 'engineering_gfx950::peer::scoped_tail_v1::tests::',
}
RUNTIME_NEW_NAMES = ['device::gfx950::scoped_currentness::tests::duration_tests::duration_backwards_clock_refuses_after_callback_without_later_observation', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_clock_unwind_before_and_after_each_callback_never_commits', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_count_mismatch_and_category_sum_refuse_before_finish_commit', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_each_call_and_elapsed_overflow_refuses_under_engine_guard', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_elapsed_and_bank_subtotal_bounds_are_checked_integer_data', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_every_callback_fault_keeps_original_prefix_and_quarantine', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_every_callback_unwind_keeps_original_prefix_and_quarantine', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_final_deadline_is_rechecked_after_reconciliation', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_production_hooks_and_armed_publication_boundaries_are_source_bound', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_real_adapter_matches_unmeasured_predicate_and_deadline_order', 'device::gfx950::scoped_currentness::tests::duration_tests::duration_selected_rank_and_group_checkpoints_share_one_recorder', 'engineering_gfx950::peer::combined_mlp_state_v1::paired::retained::mixed_bank::scoped::custody_tests::scoped_bank_duration_refusal_and_unwind_preserve_outer_custody']
DOC_FILTER = 'engineering_gfx950_peer_combined_mlp_paired_facade_v1.rs'
PHASES = ('rustfmt', 'rustfmt-check', 'rustc-version', 'runtime-metadata', 'default-check',
    'kfd-tests-build', 'kfd-list', 'kfd-ignored', 'kfd-tests', *FOCUS,
    'interface-doc-list', 'interface-doc-tests', 'metadata', 'worker-tests-build',
    'worker-list', 'worker-ignored', 'worker-tests', 'worker-build')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def load_exact(name, filename, digest):
    path = ROOT / filename
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical helper')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    before = path.stat()
    require(before.st_size <= 1 << 20, 'helper extent')
    raw = path.read_bytes()
    require(stamp(before) == stamp(path.stat()) and hashlib.sha256(raw).hexdigest() == digest,
            'authenticated helper body')
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def normalized(text):
    for name in ('payload_release_failure_after_event_destroy_is_process_terminal',
                 'unpublished_custody_cleanup_failure_is_process_terminal'):
        full = 'queue_linux::tests::' + name
        text, count = re.subn('^' + re.escape('test ' + full + ' ... \nrunning 1 test\nok')
                             + r'(?=\n|\Z)', 'test ' + full + ' ... ok', text, flags=re.M)
        require(count <= 1, 'duplicate known nested child progress')
    return text


def sources(h):
    paths = [ROOT / name for name in ('run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py')]
    paths += h.files_below(ROOT / 'ferric') + h.files_below(RUNTIME)
    require(len(paths) == len(set(paths)) == TOTAL_SOURCES, '829 runtime + final worker +4 controller/helper sources')
    rows = {str(path.relative_to(ROOT)): h.pin(path) for path in sorted(paths)}
    require(sum(row['bytes'] for row in rows.values()) <= 64 << 20
            and sum(name.startswith('fe2o3/') for name in rows) == 829
            and sum(name.startswith(WORKER_REL) for name in rows) == WORKER_FILES, 'closed source tree')
    return rows


def lineage(h, w, c, inputs, before, readset):
    proposals = {'runtime-proposal.json': RUNTIME_PROPOSAL_SHA,
                 'worker-proposal.json': WORKER_PROPOSAL_SHA}
    require(set(inputs['lineage']) == set(HISTORY) | set(proposals), 'closed direct lineage')
    bodies = {}
    for name in inputs['lineage']:
        path = ROOT / 'inputs' / name
        raw, actual = c.read(path), h.pin(path)
        require(w.compact({'x': actual})['x'] == inputs['lineage'][name], 'lineage input pin')
        if name in HISTORY:
            size, digest = HISTORY[name]
            require(c.pin(raw) == dict(bytes=size, sha256=digest), 'literal actual predecessor')
        else:
            require(c.pin(raw)['sha256'] == proposals[name], 'literal reviewed source proposal')
        bodies[name], readset[name] = raw, actual
    receipt = c.parse(bodies['baseline-complete.json'])
    source_map = c.parse(bodies['baseline-sources.json'])
    require(receipt['schema'] == 'ferric-guarded-mlp-full2303-scoped-tail-cpu-v1'
            and receipt['source_generation'] == 'full2303-scoped-tail-coupled-v1'
            and receipt['passed'] is True and receipt['failure'] is None
            and receipt['postcheck_errors'] == [] and receipt['source_unchanged'] is True
            and receipt['gpu_execution'] is False
            and receipt['input_sources'] == receipt['final_sources'] == source_map
            and len(source_map) == 1063
            and w.compact({'x': receipt['raw']['sources-after.json']})['x']
                == c.pin(bodies['baseline-sources.json'])
            and [row['label'] for row in receipt['phases']] == list(PHASES)
            and all(row['exit_code'] == 0 and row['natural_exit'] is True
                and row['reaped'] is True and row['process_group_absent'] is True
                and row['forced_cleanup'] is False for row in receipt['phases']),
            'actual clean Full Tail coupled predecessor')
    require(receipt['tool_pins'] == inputs['tool_pins']
            and receipt['limits'] == dict(
                whole_seconds=WHOLE_WALL, leaf_seconds=LEAF_WALL,
                cleanup_reserve_seconds=CLEANUP_RESERVE, cpu_seconds=h.CPU_LIMIT,
                address_space_bytes=h.AS_LIMIT, file_bytes=h.FILE_LIMIT,
                cache_bytes=h.CACHE_LIMIT, stream_bytes=h.STREAM_LIMIT,
                initial_free_bytes=h.START_FREE, live_free_bytes=h.LIVE_FREE,
                affinity=[8, 9], nice=10, cargo_jobs=2),
            'unchanged toolchain and resource contract')
    old = {name: row for name, row in w.compact(source_map).items()
           if name.startswith(('fe2o3/', WORKER_REL))}
    require(len(old) == 1059 and sum(name.startswith('fe2o3/') for name in old) == 827
            and sum(name.startswith(WORKER_REL) for name in old) == 232,
            'actual 827-runtime/232-worker source base')
    require(receipt['cli_executable_unchanged_across_tests'] is True
            and receipt['cli_executable_before_tests']['pin'] == receipt['artifacts']['worker']['pin']
            and set(receipt['artifacts']) == set(KFD_TARGETS) | set(w.TARGETS) | {'worker'}
            and all(receipt[name] is True for name in (
                'full_runtime_tests_executed', 'full_worker_tests_executed',
                'scoped_currentness_runtime_source_added', 'scoped_currentness_worker_source_added',
                'scoped_bank_rearm_runtime_source_added', 'scoped_capacity_census_runtime_source_added',
                'scoped_tail_runtime_source_added', 'shared_test_fixture_repaired',
                'readiness40_bank_scoped_census_tail_source_added', 'full2303_scoped_tail_source_added'))
            and all(receipt[name] is False for name in (
                'global_currentness_policy_changed', 'currentness_temporal_equivalence_claim',
                'gpu_execution', 'numerical_acceptance', 'performance_claim',
                'runtime_source_changed', 'readiness40_bank_scoped_census_tail_native_execution',
                'full2303_scoped_tail_native_execution', 'full2303_launch_admitted')),
            'actual qualified runtime and Full Tail predecessor without native authority')
    runtime = c.parse(bodies['runtime-proposal.json'])
    worker = c.parse(bodies['worker-proposal.json'])
    require(runtime['schema'] == 'ferric-currentness-duration-runtime-source-v1'
            and runtime['source_only'] is True and runtime['feature'] == FEATURE
            and runtime['feature_dependencies'] == ['engineering-gfx950']
            and len(runtime['files']) == 10
            and runtime['test_contract']['feature_off_new_tests'] == []
            and runtime['test_contract']['feature_on_new_tests'] == RUNTIME_DIAGNOSTIC_NAMES
            and all(runtime['guarantees'][key] is False for key in (
                'canonical_changed', 'compiled', 'formatted', 'native_execution', 'tests_executed',
                'new_currentness_policy', 'new_execution_facade', 'cached_admission_enabled',
                'ambient_profile_enabled', 'performance_claim', 'full2303_feasibility')),
            'exact reviewed feature-gated runtime proposal')
    require(worker['schema'] == SELECTOR_SCHEMA
            and worker['features']['worker'] == dict(name=FEATURE, dependencies=['fe2o3-kfd/' + FEATURE])
            and worker['features']['default_features_changed'] is False
            and worker['features']['locks_changed'] is False
            and all(worker['guarantees'][key] is False for key in (
                'canonical_changed', 'compiled', 'formatted', 'tests_executed',
                'currentness_policy_changed', 'new_execution_authority', 'new_runtime_facade',
                'new_selector', 'native_execution', 'numerical_acceptance', 'performance_claim'))
            and worker['test_contract']['feature_off_new_tests'] == []
            and worker['test_contract']['feature_on_new_tests'] == WORKER_DIAGNOSTIC_NAMES
            and worker['test_contract']['worker_readiness_cli_new_tests'] == [],
            'exact reviewed diagnostic-only worker additions')
    worker_rows = [row for row in worker['files']
                   if row['repository'] == 'ferric'
                   and ('ferric/' + row['path']).startswith(WORKER_REL)]
    require(len(worker_rows) == SELECTOR_ROWS
            and len(worker['files']) == SELECTOR_ROWS + 4,
            'twelve worker rows; four parent rows are authenticated but never staged')
    expected, overlay = dict(old), set()
    for repository, rows, count, additions in (
            ('fe2o3', runtime['files'], 10, 2), ('ferric', worker_rows, SELECTOR_ROWS, 4)):
        require(len({row['path'] for row in rows}) == count
                and sum(row['before'] is None for row in rows) == additions,
                'exact unique runtime/worker rows and additions')
        for row in rows:
            path, name = row['path'], repository + '/' + row['path']
            manifest = name in ('fe2o3/crates/fe2o3-kfd/Cargo.toml', WORKER_REL + 'Cargo.toml')
            require(w.ordinary_relative(path) and (path.endswith('.rs') or manifest)
                    and (name.startswith('fe2o3/crates/fe2o3-kfd/') if repository == 'fe2o3'
                         else name.startswith(WORKER_REL))
                    and row['repository'] == repository and expected.get(name) == row['before'],
                    'exact scoped Rust or crate-feature manifest preimage/absence')
            expected[name] = row['after']
            overlay.add(name)
    require(sum(name.startswith('fe2o3/') for name in expected) == 829
            and sum(name.startswith(WORKER_REL) for name in expected) == WORKER_FILES
            and inputs['overlay'] == sorted(overlay)
            and {name: row for name, row in inputs['files'].items()
                 if name.startswith(('fe2o3/', WORKER_REL))} == expected
            and w.compact(before) == inputs['files'], 'closed source map with both reviewed overlays')
    baselines = {}
    for kind, label in (('runtime', 'kfd-tests'), ('worker', 'worker-tests')):
        raw = bodies[kind + '-tests.stdout'].decode()
        previous = w.outcomes(normalized(raw) if kind == 'runtime' else raw)
        require(previous == receipt['tests'][label]
                and w.inventory(bodies[kind + '-list.stdout'].decode()) == sorted(w.statuses(previous)),
                'all original named outcomes and inventory')
        for suffix, raw_label in (('-tests.stdout', label + '.stdout'),
                                  ('-list.stdout', label.replace('-tests', '-list') + '.stdout')):
            require(w.compact({'x': receipt['raw'][raw_label]})['x']
                    == c.pin(bodies[kind + suffix]), 'actual historical raw receipt join')
        baselines[kind] = previous
    new_worker = WORKER_DIAGNOSTIC_NAMES if DIAGNOSTIC else []
    require(len(RUNTIME_DIAGNOSTIC_NAMES) == len(set(RUNTIME_DIAGNOSTIC_NAMES)) == 12
            and len(WORKER_DIAGNOSTIC_NAMES) == len(set(WORKER_DIAGNOSTIC_NAMES)) == 15
            and not set(RUNTIME_DIAGNOSTIC_NAMES) & set(w.statuses(baselines['runtime']))
            and not set(WORKER_DIAGNOSTIC_NAMES) & set(w.statuses(baselines['worker'])),
            'exact disjoint feature-only additions')
    declared = set()
    for name in overlay:
        if name.endswith('.rs'):
            declared.update(re.findall(r'#\[test\]\s*fn\s+(\w+)\s*\(', c.read(ROOT / name).decode()))
    require({name.rsplit('::', 1)[-1] for name in [*RUNTIME_DIAGNOSTIC_NAMES, *WORKER_DIAGNOSTIC_NAMES]} <= declared,
            'source-declared additions for both modes')
    require((baselines['runtime']['passed'], baselines['runtime']['ignored']) == (1180, 8)
            and (baselines['worker']['passed'], baselines['worker']['ignored']) == (838, 4),
            'actual Full Tail runtime and worker baseline outcomes')
    require(c.pin(bodies['runtime-docs.stdout'])
            == w.compact({'x': receipt['raw']['interface-doc-tests.stdout']})['x'], 'actual ten-doc raw join')
    return dict(test_names=list(RUNTIME_NEW_NAMES), worker_test_names=new_worker), baselines, \
        bodies['legacy-docs.stdout'].decode(), bodies['runtime-docs.stdout'].decode()
def cache_contract(h, w, c, inputs):
    raw = c.read(ROOT / 'cargo-cache-manifest.json')
    require(c.pin(raw) == inputs['cache_manifest'], 'actual two-lock cache manifest binding')
    manifest = c.parse(raw)
    stage = c.parse(c.read(ROOT / 'cargo-cache-stage-complete.json'))
    locks = {role: c.read(ROOT / row[0]) for role, row in c.LOCKS.items()}
    bodies = {name: c.read(CARGO_HOME / name) for name in manifest['files']}
    require(len(bodies) == 73 and all(c.relative(name) and name.startswith('registry/') for name in bodies),
            'exact immutable private cache inputs')
    c.validate(manifest, bodies, locks)
    require(stage['schema'] == 'ferric-guarded-mlp-reusable-arena-cache-stage-v1' and stage['passed'] is True
            and stage['manifest'] == inputs['cache_manifest'] and stage['locks'] == manifest['locks']
            and stage['cargo_home'] == str(CARGO_HOME) and stage['files'] == manifest['files']
            and stage['packages'] == 39 and stage['cache_files'] == 73
            and stage['controller'] == inputs['files']['cache.py']
            and all(stage[key] is False for key in ('shared_cache_changed', 'lock_changed',
                        'project_code_executed', 'crate_sources_extracted')), 'closed private cache staging')
    return dict(manifest=h.pin(ROOT / 'cargo-cache-manifest.json'),
        stage=h.pin(ROOT / 'cargo-cache-stage-complete.json'),
        files={name: h.pin(CARGO_HOME / name) for name in bodies}, locks=manifest['locks'],
        locked_packages=manifest['locked_packages'], archive=stage['archive'])


def runtime_metadata(h, w, c):
    value = c.parse(c.read(OUT / 'runtime-metadata.stdout'))
    require(value['workspace_root'] == str(RUNTIME) and value['target_directory'] == str(TARGET),
            'runtime workspace/target')
    packages = {row['id']: row for row in value['packages']}
    require(len(packages) == len(value['packages']) <= 256, 'unique bounded runtime package graph')
    expected_local = w.CRATES - {w.PACKAGE}
    local, external = {}, {}
    for row in packages.values():
        path = Path(row['manifest_path'])
        require(path.resolve(strict=True) == path, 'dependency alias')
        if row['source'] is None:
            require(row['name'] in expected_local and row['name'] not in local
                    and path == RUNTIME / 'crates' / row['name'] / 'Cargo.toml', 'closed runtime package')
            local[row['name']] = str(path)
        else:
            require(row['source'] == c.REGISTRY and path.is_relative_to(CARGO_HOME / 'registry/src'),
                    'locked private registry dependency')
            external[str(path.parent)] = {str(p.relative_to(path.parent)): h.pin(p)
                                          for p in h.files_below(path.parent, packed=False)}
    require(set(local) == expected_local and all(packages[key]['source'] is None
            for key in value['workspace_members']), 'nine runtime workspace crates')
    kfd = next(row for row in packages.values() if row['name'] == 'fe2o3-kfd')
    for name, kind, source in KFD_TARGETS.values():
        require(sum(row['name'] == name and row['kind'] == [kind]
                    and row['src_path'] == str(RUNTIME / 'crates/fe2o3-kfd' / source)
                    for row in kfd['targets']) == 1, 'selected KFD target')
    return external, local


def resolved_features(c, label, package, expected):
    value = c.parse(c.read(OUT / (label + '.stdout')))
    packages = [row for row in value['packages'] if row['name'] == package]
    require(len(packages) == 1 and packages[0]['source'] is None, 'one exact local feature owner')
    nodes = [row for row in value['resolve']['nodes'] if row['id'] == packages[0]['id']]
    require(len(nodes) == 1 and nodes[0]['features'] == expected,
            'exact selected resolved feature set for ' + package)


def selected_artifact(h, records, name, kind, source, testing):
    rows = [row for row in records if row.get('reason') == 'compiler-artifact'
            and row.get('manifest_path') == str(WORKER / 'Cargo.toml')
            and row.get('target', {}).get('name') == name
            and row['target']['kind'] == [kind] and row.get('profile', {}).get('test') is testing]
    require(len(rows) == 1, 'unique worker artifact')
    row = rows[0]
    require(row['target']['src_path'] == str(WORKER / source) and row['features'] == WORKER_FEATURES
            and type(row.get('executable')) is str, 'worker artifact source/features')
    path = Path(row['executable'])
    require(path.is_relative_to(TARGET) and path.resolve(strict=True) == path
            and row['filenames'].count(str(path)) == 1, 'fresh target worker artifact')
    with path.open('rb') as stream:
        require(stream.read(4) == b'\x7fELF', 'worker artifact ELF magic')
    return dict(pin=h.pin(path), cargo_artifact=row)



def doc_results(text):
    rows, active, summaries = [], [], []
    # Rustdoc emits separate compiled-example and compile-fail sections.
    for line in text.splitlines():
        row = re.fullmatch(r'test (.+?) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?', line)
        summary = re.fullmatch(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; '
                              r'(\d+) measured; (\d+) filtered out; finished in [0-9.]+s', line)
        if row:
            name, status = row.groups()
            require(name not in {name for name, _ in rows} and status == 'ok', 'unique successful doc result')
            rows.append((name, status))
            active.append((name, status))
        elif summary:
            values = summary.groups()
            require(active and values[:5] == ('ok', str(len(active)), '0', '0', '0'), 'exact doc section census')
            summaries.append(values)
            active = []
        else:
            require(not line.startswith('test '), 'malformed or unsupported doc result')
    require(not active and 1 <= len(summaries) <= 2 and rows
            and sum(int(row[1]) for row in summaries) == len(rows), 'complete successful selected doc census')
    return dict(named=rows, summaries=summaries, passed=len(rows), failed=0, ignored=0)


def doc_regressions(actual, historical):
    cases = []
    def accepted(name, body, count, sections):
        value = doc_results(body)
        require(value['passed'] == count and len(value['summaries']) == sections, 'doc regression ' + name)
        cases.append(dict(name=name, outcome='ok'))
    def refused(name, body):
        require(body != actual, 'doc regression mutation changed bytes')
        try:
            doc_results(body)
        except RuntimeError:
            cases.append(dict(name=name, outcome='ok'))
        else:
            raise RuntimeError('doc regression accepted ' + name)
    accepted('actual_two_sections', actual, 10, 2)
    accepted('historical_one_section', historical, 9, 1)
    first = next(line for line in actual.splitlines() if line.startswith('test ') and ' ... ' in line)
    refused('missing_named_result', actual.replace(first + '\n', '', 1))
    refused('duplicate_named_result', actual.replace(first, first + '\n' + first, 1))
    refused('wrong_section_subtotal', actual.replace('ok. 1 passed;', 'ok. 2 passed;', 1))
    refused('failed_named_result', actual.replace(' ... ok', ' ... FAILED', 1))
    summary = next(line for line in actual.splitlines() if line.startswith('test result:'))
    refused('duplicate_section_summary', actual.replace(summary, summary + '\n' + summary, 1))
    final_summary = [line for line in actual.splitlines() if line.startswith('test result:')][-1]
    refused('missing_final_summary', actual.replace(final_summary + '\n', '', 1))
    require(len(cases) == 8, 'eight exact doc parser regressions')
    return dict(passed=8, failed=0, cases=cases, source='pinned current ten-doc and historical nine-doc raw stdout',
                project_execution=False, gpu_execution=False)


def main():
    require(all(value is not None for value in (WORKER_PROPOSAL_SHA, SELECTOR_SCHEMA,
            SELECTOR_ROWS, WORKER_FILES, WORKER_DIAGNOSTIC_NAMES, WORKER_SUMMARY_ADDITIONS, TOTAL_SOURCES)),
            'exact reviewed selector and composition bindings are required')
    started = time.monotonic()
    deadline = started + WHOLE_WALL
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch('[0-9a-f]{64}', sys.argv[1]), 'python3 -B run_cpu.py INPUT_SHA')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT
            and not any(os.path.lexists(path) for path in (OUT, TARGET, TMP)), 'fresh CPU namespace')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'qualified host and UID')
    require(shutil.disk_usage(ROOT).free >= 40 << 30, '40 GiB initial free floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'nice level')
    if priority == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, 1 << 30)):
        bounds = resource.getrlimit(kind)
        value = min([cap] + [n for n in bounds if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper')
    w = load_exact('qualified_worker_support', 'worker_support.py', SUPPORT_SHA)
    c = load_exact('private_cache_data', 'cache.py', CACHE_SHA)
    w.ROOT, w.FERRIC, w.RUNTIME, w.WORKER = ROOT, ROOT / 'ferric', RUNTIME, WORKER
    w.OUT, w.TARGET, w.TMP, w.CARGO_HOME = OUT, TARGET, TMP, CARGO_HOME
    h = w.load_supervisor()
    h.SOURCE = RUNTIME
    h.sources = lambda: sources(h)
    old_scratch = h.scratch_bytes
    def scratch():
        total = old_scratch()
        count = 0
        for directory, dirs, names in os.walk(CARGO_HOME, followlinks=False):
            require(all(not (Path(directory) / name).is_symlink() for name in dirs), 'cache directory alias')
            for name in names:
                count += 1
                try:
                    total += (Path(directory) / name).lstat().st_size
                except FileNotFoundError:
                    pass
        require(count <= 15000, 'bounded generated cache roster')
        return total
    h.scratch_bytes = scratch
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, deadline - CLEANUP_RESERVE - time.monotonic())
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    inputs = input_pin = preformat = formatted = final = config = cache = proposal = baselines = doc_parser_tests = None
    readset, tool_pins, external, external_after, local, artifacts, tests, inventories = {}, {}, {}, {}, {}, {}, {}, {}
    phases, errors, changed = [], [], []
    failure = None
    cli_executable_pretest, cli_executable_unchanged = None, False
    try:
        input_pin = h.pin(ROOT / 'input-manifest.json')
        require(input_pin['sha256'] == sys.argv[1], 'literal input pin')
        inputs = c.parse(c.read(ROOT / 'input-manifest.json'))
        require(set(inputs) == {'schema', 'source_generation', 'qualification_mode', 'files', 'tool_pins', 'lineage', 'overlay', 'cache_manifest'}
                and inputs['schema'] == 'ferric-guarded-mlp-currentness-duration-cpu-input-v1'
                and inputs['source_generation'] == GENERATION and inputs['qualification_mode'] == MODE, 'closed input schema')
        preformat = sources(h)
        h.save('sources-preformat.json', preformat)
        require((WORKER / '../../../fe2o3').resolve(strict=True) == RUNTIME, 'unchanged worker dependency layout')
        proposal, baselines, old_docs, current_docs = lineage(h, w, c, inputs, preformat, readset)
        doc_parser_tests = doc_regressions(current_docs, old_docs)
        h.save('doc-parser-regression.json', doc_parser_tests)
        config = w.configurations(h)
        tool_pins = {name: h.pin(h.TOOLCHAIN / name) for name in ('rustc', 'rustdoc', 'rustfmt', 'cargo')}
        tool_pins['prlimit'] = h.pin(Path('/usr/bin/prlimit'))
        for name, size in h.SHARED_LIBRARIES.items():
            tool_pins[name] = h.pin(h.TOOLCHAIN_LIB / name)
            require(tool_pins[name]['bytes'] == size, 'qualified compiler library extent')
        require(tool_pins == inputs['tool_pins'], 'qualified immutable compiler toolchain')
        cache = cache_contract(h, w, c, inputs)
        require({str(path.relative_to(CARGO_HOME)) for path in h.files_below(CARGO_HOME, packed=False)}
                == set(cache['files']), 'fresh private Cargo home, no preexisting extracted sources')
        env = dict(HOME='/home/harmenon', PATH=str(h.TOOLCHAIN) + ':/usr/bin:/bin', CARGO_HOME=str(CARGO_HOME),
            CARGO_TARGET_DIR=str(TARGET), LD_LIBRARY_PATH=str(TARGET / 'debug/deps') + ':' + str(h.TOOLCHAIN_LIB),
            TMPDIR=str(TMP), RUSTC=str(h.TOOLCHAIN / 'rustc'), RUSTDOC=str(h.TOOLCHAIN / 'rustdoc'),
            CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true',
            CARGO_CACHE_AUTO_CLEAN_FREQUENCY='never', MALLOC_ARENA_MAX='2', RUST_BACKTRACE='1',
            CARGO_PROFILE_DEV_OPT_LEVEL='2', CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true',
            CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true', CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_TEST_DEBUG='0',
            CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
            ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
        def leaf(label, argv, seconds=LEAF_WALL, tested=None, cwd=RUNTIME):
            h.run(label, argv, env, phases, deadline, tested, seconds, cwd)
        fmt = [str(h.TOOLCHAIN / 'rustfmt'), '--edition', '2024', '--config', 'skip_children=true']
        overlay = [str(ROOT / name) for name in inputs['overlay'] if name.endswith('.rs')]
        leaf('rustfmt', [*fmt, *overlay], 120)
        formatted = sources(h)
        changed = sorted(name for name in formatted if formatted[name] != preformat[name])
        require(set(formatted) == set(preformat) and set(changed) <= {name for name in inputs['overlay'] if name.endswith('.rs')}, 'format only reviewed Rust overlay paths, never Cargo manifests')
        h.save('sources-before.json', formatted)
        leaf('rustfmt-check', [*fmt, '--check', *overlay], 120, formatted)
        leaf('rustc-version', [str(h.TOOLCHAIN / 'rustc'), '--version', '--verbose'], 60, formatted)
        cargo = str(h.TOOLCHAIN / 'cargo')
        common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(RUNTIME / 'Cargo.toml'), '-p', 'fe2o3-kfd']
        runtime_features = ['--features', ','.join(RUNTIME_FEATURES[1:])]
        worker_features = ['--features', FEATURE] if DIAGNOSTIC else []
        selected = [*common, *runtime_features, '--lib', '--tests']
        leaf('runtime-metadata', [cargo, 'metadata', '--offline', '--locked', '--format-version', '1',
                                 '--manifest-path', str(RUNTIME / 'Cargo.toml'),
                                 '--features', ','.join('fe2o3-kfd/' + f for f in RUNTIME_FEATURES[1:])], 120, formatted)
        external, local['runtime'] = runtime_metadata(h, w, c)
        resolved_features(c, 'runtime-metadata', 'fe2o3-kfd', RUNTIME_FEATURES)
        h.save('runtime-dependencies-before.json', external)
        leaf('default-check', [cargo, 'check', *common, '--no-default-features', *worker_features], tested=formatted)
        leaf('kfd-tests-build', [cargo, 'test', *selected, '--no-run', '--message-format=json'], tested=formatted)
        records = h.build_records(OUT / 'kfd-tests-build.stdout')
        require(sum(row.get('reason') == 'compiler-artifact' and row.get('manifest_path')
                == str(RUNTIME / 'crates/fe2o3-kfd/Cargo.toml') and row.get('profile', {}).get('test') is True
                for row in records) == 6, 'exact six runtime test products')
        for role, (name, kind, source) in KFD_TARGETS.items():
            artifact = h.select_artifact(records, 'fe2o3-kfd', name, kind, True)
            row = artifact['cargo_artifact']
            require(row['target']['kind'] == [kind] and row['target']['src_path'] == str(RUNTIME / 'crates/fe2o3-kfd' / source)
                    and row['features'] == RUNTIME_FEATURES
                    and row['filenames'].count(artifact['pin']['path']) == 1, 'runtime product source/features')
            artifacts[role] = artifact
        expected = w.statuses(baselines['runtime']) | {name: 'ok' for name in proposal['test_names']}
        summaries = [dict(row) for row in baselines['runtime']['summaries']]
        summaries[0]['passed'] += len(RUNTIME_NEW_NAMES)
        leaf('kfd-list', [cargo, 'test', *selected, '--', '--list', '--format=terse'], 120, formatted)
        leaf('kfd-ignored', [cargo, 'test', *selected, '--', '--ignored', '--list', '--format=terse'], 120, formatted)
        inventories['runtime'] = w.inventory((OUT / 'kfd-list.stdout').read_text())
        require(inventories['runtime'] == sorted(expected) and w.inventory((OUT / 'kfd-ignored.stdout').read_text())
                == sorted(name for name, status in expected.items() if status == 'ignored'), 'runtime exact named inventory')
        leaf('kfd-tests', [cargo, 'test', *selected, '--', '--test-threads=1'], tested=formatted)
        value = w.outcomes(normalized((OUT / 'kfd-tests.stdout').read_text()))
        require(value['summaries'] == summaries and w.statuses(value) == expected
                and (value['passed'], value['failed'], value['ignored']) == (1180 + len(RUNTIME_NEW_NAMES), 0, 8), 'all old runtime outcomes plus exact mode-selected additions')
        tests['kfd-tests'] = value
        for label, prefix in FOCUS.items():
            leaf(label, [cargo, 'test', *common, *runtime_features, '--lib', prefix,
                         '--', '--test-threads=1'], 180, formatted)
            value = w.outcomes(normalized((OUT / (label + '.stdout')).read_text()))
            named = {name: status for name, status in expected.items() if name.startswith(prefix)}
            require(named and w.statuses(value) == named and len(value['summaries']) == 1
                    and value['summaries'][0]['filtered_out'] == 1165 + len(RUNTIME_NEW_NAMES) - len(named)
                    and value['failed'] == value['ignored'] == 0, 'focused exact library scope')
            tests[label] = value
        docs = [*common, *runtime_features, '--doc', DOC_FILTER]
        leaf('interface-doc-list', [cargo, 'test', *docs, '--', '--list', '--format=terse'], 120, formatted)
        doc_names = [line[:-6] for line in (OUT / 'interface-doc-list.stdout').read_text().splitlines() if line.endswith(': test')]
        require(len(doc_names) == len(set(doc_names)) == 10 and all(DOC_FILTER in name for name in doc_names), 'ten selected facade docs')
        leaf('interface-doc-tests', [cargo, 'test', *docs, '--', '--test-threads=1'], 180, formatted)
        value = doc_results((OUT / 'interface-doc-tests.stdout').read_text())
        normalize = lambda name: re.sub(r' \(line \d+\)', '', name)
        old = doc_results(old_docs)
        require(value['passed'] == 10 and Counter(normalize(name) for name, _ in value['named'] if name.endswith(' - compile fail'))
                == Counter(normalize(name) for name, _ in old['named'])
                and sum('bind_guarded_mlp_pair_exact_own_residual_reusable_unchecked_v1' in name
                        and not name.endswith(' - compile fail') for name, _ in value['named']) == 1
                and {re.sub(r' - compile(?: fail)?$', '', name) for name, _ in value['named']} == set(doc_names),
                'nine inherited privacy docs and one new compiled public-signature example')
        tests['interface-doc-tests'] = dict(value, inventory=doc_names)
        wc = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(WORKER / 'Cargo.toml'), *worker_features]
        ws = [*wc, '--lib', '--bin', w.PACKAGE, '--test', 'readiness_cli', '--test', 'shared_wire']
        leaf('metadata', [cargo, 'metadata', '--offline', '--locked', '--format-version', '1',
                         '--manifest-path', str(WORKER / 'Cargo.toml'), *worker_features], 120, formatted, WORKER)
        worker_external, local['worker'] = w.metadata_contract(h)
        resolved_features(c, 'metadata', w.PACKAGE, WORKER_FEATURES)
        resolved_features(c, 'metadata', 'fe2o3-kfd', RUNTIME_FEATURES)
        for path, rows in worker_external.items():
            require(path not in external or external[path] == rows, 'shared dependency unchanged between resolutions')
            external[path] = rows
        h.save('dependencies-before.json', external)
        leaf('worker-tests-build', [cargo, 'test', *ws, '--no-run', '--message-format=json'], tested=formatted, cwd=WORKER)
        records = h.build_records(OUT / 'worker-tests-build.stdout')
        require(sum(row.get('reason') == 'compiler-artifact' and row.get('manifest_path') == str(WORKER / 'Cargo.toml')
                    and row.get('profile', {}).get('test') is True for row in records) == 4, 'four worker test products')
        for role, (name, kind, source) in w.TARGETS.items():
            artifacts[role] = selected_artifact(h, records, name, kind, source, True)
        cli_executable_pretest = selected_artifact(h, records, w.PACKAGE, 'bin', 'src/main.rs', False)
        require(cli_executable_pretest['pin']['path'] == str(TARGET / 'debug' / w.PACKAGE),
                'actual worker executable used by both CLI integration targets')
        leaf('worker-list', [cargo, 'test', *ws, '--', '--list', '--format=terse'], 120, formatted, WORKER)
        leaf('worker-ignored', [cargo, 'test', *ws, '--', '--ignored', '--list', '--format=terse'], 120, formatted, WORKER)
        expected_worker = w.statuses(baselines['worker']) | {name: 'ok' for name in proposal['worker_test_names']}
        worker_summaries = [dict(row) for row in baselines['worker']['summaries']]
        require(len(worker_summaries) == len(WORKER_SUMMARY_ADDITIONS) == 4, 'four worker test targets')
        for row, additions in zip(worker_summaries, WORKER_SUMMARY_ADDITIONS):
            row['passed'] += additions
        inventories['worker'] = w.inventory((OUT / 'worker-list.stdout').read_text())
        require(inventories['worker'] == sorted(expected_worker) and w.inventory((OUT / 'worker-ignored.stdout').read_text())
                == sorted(name for name, status in expected_worker.items() if status == 'ignored'), 'all old worker names plus mode-selected diagnostics additions')
        require(h.pin(Path(cli_executable_pretest['pin']['path'])) == cli_executable_pretest['pin'],
                'real executable unchanged immediately before integration tests')
        leaf('worker-tests', [cargo, 'test', *ws, '--', '--test-threads=1'], tested=formatted, cwd=WORKER)
        require(h.pin(Path(cli_executable_pretest['pin']['path'])) == cli_executable_pretest['pin'],
                'real executable unchanged across actual EOF CLI probes')
        cli_executable_unchanged = True
        value = w.outcomes((OUT / 'worker-tests.stdout').read_text())
        require(w.statuses(value) == expected_worker and value['summaries'] == worker_summaries
                and (value['passed'], value['failed'], value['ignored'])
                    == (838 + sum(WORKER_SUMMARY_ADDITIONS), 0, 4),
                'all actual old worker outcomes plus exact reviewed additions')
        tests['worker-tests'] = value
        leaf('worker-build', [cargo, 'build', *wc, '--bin', w.PACKAGE, '--message-format=json'], tested=formatted, cwd=WORKER)
        artifacts['worker'] = selected_artifact(h, h.build_records(OUT / 'worker-build.stdout'),
                                                w.PACKAGE, 'bin', 'src/main.rs', False)
        require(artifacts['worker']['pin'] == cli_executable_pretest['pin'],
                'final worker ELF equals executable exercised by integration tests')
        require(len({row['pin']['path'] for row in artifacts.values()}) == 11
                and [row['label'] for row in phases] == list(PHASES), 'eleven distinct products and exact 27 phases')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    def check(label, action):
        try:
            remaining = deadline - 5 - time.monotonic()
            require(remaining > 0, 'postcheck deadline')
            signal.setitimer(signal.ITIMER_REAL, remaining)
            action()
        except BaseException as error:
            errors.append(label + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    def source_check():
        nonlocal final
        final = sources(h)
        h.save('sources-after.json', final)
        if formatted is not None:
            require(final == formatted, 'qualified source/lock drift')
        elif preformat is not None:
            require(set(final) == set(preformat) and all(final[name] == row for name, row in preformat.items()
                    if name not in (inputs or {}).get('overlay', [])), 'failed format changed outside overlay')
    check('sources', source_check)
    if input_pin:
        check('input', lambda: require(h.pin(ROOT / 'input-manifest.json') == input_pin, 'input drift'))
    if config is not None:
        check('configurations', lambda: require(w.configurations(h) == config, 'Cargo configuration drift'))
    if cache is not None:
        check('cache', lambda: require(cache_contract(h, w, c, inputs) == cache, 'immutable cache input drift'))
    for label, row in dict(tool_pins, **readset).items():
        check('readset ' + label, lambda row=row: require(h.pin(Path(row['path'])) == row, 'readset drift'))
    for path, expected in external.items():
        def dependency_check(path=path, expected=expected):
            root = Path(path)
            actual = {str(p.relative_to(root)): h.pin(p) for p in h.files_below(root, packed=False)}
            external_after[path] = actual
            require(actual == expected, 'resolved dependency drift')
        check('dependency ' + path, dependency_check)
    check('dependency ledger', lambda: h.save('dependencies-after.json', external_after))
    for label, artifact in artifacts.items():
        check('product ' + label, lambda artifact=artifact: require(h.pin(Path(artifact['pin']['path'])) == artifact['pin'], 'product drift'))
    raw = {}
    def raw_check():
        nonlocal raw
        raw = {path.name: h.pin(path) for path in sorted(OUT.iterdir()) if path.is_file()}
        for row in phases:
            for key in ('command', 'stdout', 'stderr'):
                require(raw[Path(row[key]['path']).name] == row[key], 'original phase bytes')
    check('raw', raw_check)
    failure = failure or ('postcheck failed' if errors else None)
    if time.monotonic() >= deadline:
        failure = failure or 'whole deadline exceeded'
    result = dict(schema='ferric-guarded-mlp-currentness-duration-cpu-v1', passed=failure is None,
        failure=failure, postcheck_errors=errors, source_generation=GENERATION, qualification_mode=MODE,
        selected_runtime_features=RUNTIME_FEATURES, selected_worker_features=WORKER_FEATURES,
        default_check_no_default_features=True, default_check_diagnostic_feature_requested=DIAGNOSTIC,
        currentness_duration_diagnostic_build=DIAGNOSTIC, currentness_duration_runtime_source_added=True,
        currentness_duration_worker_source_added=True, currentness_duration_native_execution=False,
        input_manifest=input_pin,
        controller=h.pin(ROOT / 'run_cpu.py'), supervisor=h.pin(ROOT / 'supervisor.py'),
        support=h.pin(ROOT / 'worker_support.py'), cache_helper=h.pin(ROOT / 'cache.py'),
        readset=readset, baseline_tests=baselines, preformat_sources=preformat, input_sources=formatted,
        final_sources=final, source_unchanged=formatted is not None and formatted == final,
        format_changed_paths=changed, phases=phases, artifacts=artifacts, tests=tests, inventories=inventories,
        tool_pins=tool_pins, local_dependencies=local, cache_provenance=cache, configurations=config, raw=raw,
        elapsed_seconds=time.monotonic() - started,
        doc_parser_tests=doc_parser_tests,
        cli_executable_before_tests=cli_executable_pretest,
        cli_executable_unchanged_across_tests=cli_executable_unchanged,
        limits=dict(whole_seconds=WHOLE_WALL, leaf_seconds=LEAF_WALL, cleanup_reserve_seconds=CLEANUP_RESERVE,
            cpu_seconds=h.CPU_LIMIT, address_space_bytes=h.AS_LIMIT, file_bytes=h.FILE_LIMIT,
            cache_bytes=h.CACHE_LIMIT, stream_bytes=h.STREAM_LIMIT, initial_free_bytes=h.START_FREE,
            live_free_bytes=h.LIVE_FREE, affinity=[8, 9], nice=10, cargo_jobs=2),
        full_runtime_tests_executed='kfd-tests' in tests, full_worker_tests_executed='worker-tests' in tests,
        selected_facade_doctests_executed='interface-doc-tests' in tests, all_crate_doctests_executed=False,
        inherited_terminal_pair_runtime_preserved=True, explicit_per_dispatch_opt_in=True,
        legacy_terminal_dispatch_preserved=True, fresh_terminal_opt_in_refused=True,
        inherited_peer_read_pair_runtime_preserved=True, opt_in_retired_arena_reuse_preserved=True,
        arena_policy_changed=False, opt_in_terminal_currentness_cadence_changed=True,
        default_fresh_arena_policy_changed=False, global_currentness_policy_changed=False,
        method_local_currentness_cadence_changed=True, ordinary_read_api_changed=False,
        public_runtime_limits_changed=False,
        scoped_currentness_runtime_source_added=True, scoped_currentness_worker_source_added=True,
        explicit_scoped_readiness_selector_source_added=True, scoped_currentness_native_execution=False,
        currentness_temporal_equivalence_claim=False,
        worker_source_changed=True, lockfiles_changed=False, shared_cache_changed=False,
        full_model_long_request_enabled=True, gpu_execution=False, gpu_qualified=False,
        inherited_scoped_currentness_runtime_preserved=True, runtime_source_changed=True,
        full2303_scoped_warm_source_added=True, full2303_scoped_warm_native_execution=False,
        full2303_launch_admitted=False,
        inherited_readiness40_bank_scoped_census_preserved=True,
        scoped_bank_rearm_runtime_source_added=True, scoped_capacity_census_runtime_source_added=True,
        readiness40_bank_scoped_warm_source_added=True, readiness40_bank_scoped_census_source_added=True,
        readiness40_bank_scoped_warm_native_execution=False, readiness40_bank_scoped_census_native_execution=False,
        full2303_bank_scoped_census_source_added=True, full2303_bank_scoped_census_native_execution=False,
        allocation_preflights_changed=True, full2303_deadline_changed=False,
        scoped_tail_runtime_source_added=True, shared_test_fixture_repaired=True,
        readiness40_bank_scoped_census_tail_source_added=True,
        readiness40_bank_scoped_census_tail_native_execution=False,
        inherited_bank_scoped_currentness_preserved=True, inherited_readiness40_tail_preserved=True,
        full2303_scoped_tail_source_added=True, full2303_scoped_tail_native_execution=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - time.monotonic()))
    try:
        h.save('complete.json' if failure is None else 'failed.json', result)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(passed=failure is None, failure=failure, phases=len(phases),
                         output=str(OUT)), sort_keys=True))
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
