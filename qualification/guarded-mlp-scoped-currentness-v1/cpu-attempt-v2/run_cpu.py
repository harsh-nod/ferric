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
EXPECTED_ROOT = E / 'guarded-mlp-scoped-currentness-cpu-v228-v2'
RUNTIME = ROOT / 'fe2o3'
WORKER_REL = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
WORKER = ROOT / WORKER_REL
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
CARGO_HOME = ROOT / 'cargo-home'
SUPPORT_SHA = '4d12ba86c4014adaaea1c5c49c3790dffcca78af8326ae09cb085840347a518a'
CACHE_SHA = '35ffa957a6dff429572b9b4d9038e81a61712c1f8d28d678bbe36a58946d2c35'
RUNTIME_PROPOSAL_SHA = '437948cd334c9f95dcf77de4e3217c92f4d8e6d4cabbcf5010aff182f53ae5b2'
PRIMITIVE_PROPOSAL_SHA = '9abcd59d4c3fd23d532bd1b9e86592a191e62960d8a901fbd31d8e61974bdec7'
CONSUMER_PROPOSAL_SHA = '6b4dffbe4032e52f6dddb80f255553c420e95a3fb7af84f2b1da311daf1168bb'
CONSUMER_REPAIR_SHA = 'c2be6fe83fba6c4bfef971bdf1e1535b88167ec8685f6faf6aa163abbdaa93e0'
# These are source-review bindings, not runtime caller parameters.
SELECTOR_PROPOSAL_SHA = 'cfec625935042c416a329cfb0e02c1fc8836f12820967e1d3c0257ed2d6a8235'
SELECTOR_SCHEMA = 'ferric-scoped-warm-readiness40-worker-selector-source-v1'
SELECTOR_FIELDS = {
    'canonical_changed': False,
    'compiled': False,
    'tested': False,
    'native_execution': False,
    'profile_enabled': True,
    'old_routes_preserved': True,
    'default_group_policy_changed': False,
    'full2303_profile_enabled': False,
    'performance_claim': False,
    'numerical_acceptance': False,
}
SELECTOR_ROWS = 9
WORKER_FILES = 209
WORKER_NEW_NAMES = {
    'worker-lib': [
        'finite_guarded_mlp_readiness_scoped_v1::tests::scoped_warm_policy_checks_every_call_count_and_unbounded_counter_overflow',
        'finite_guarded_mlp_readiness_scoped_v1::tests::scoped_warm_policy_exact_closed_record_roundtrips',
        'finite_guarded_mlp_readiness_scoped_v1::tests::scoped_warm_policy_refuses_boolean_integer_substitution',
        'finite_guarded_mlp_readiness_scoped_v1::tests::scoped_warm_policy_refuses_every_policy_and_authority_drift',
        'finite_guarded_mlp_readiness_scoped_v1::tests::scoped_warm_policy_refuses_other_profiles_and_missing_identity',
        'finite_guarded_mlp_readiness_scoped_v1::tests::scoped_warm_policy_refuses_scope_worker_transcript_and_counts_drift',
        'finite_guarded_mlp_readiness_scoped_v1::tests::scoped_warm_policy_requires_one_exact_bounded_record',
        'native_catalog::forward::guarded_mlp_decode_v1::readiness::scoped::tests::scoped_route_calls_every_layer_with_two_ordinary_first_uses_and_no_fallback',
        'native_catalog::forward::guarded_mlp_decode_v1::readiness::scoped::tests::scoped_route_post_runtime_completion_counter_rejection_cannot_retry_or_commit',
        'native_catalog::forward::guarded_mlp_decode_v1::readiness::scoped::tests::scoped_route_rejects_first_use_scoped_and_warm_ordinary_results_fatally',
        'native_catalog::forward::guarded_mlp_decode_v1::readiness::scoped::tests::scoped_route_rejects_wrong_profile_causal_order_and_incomplete_close',
        'native_catalog::forward::guarded_mlp_decode_v1::readiness::scoped::tests::scoped_route_runtime_error_and_unwind_keep_original_failure_terminal',
        'native_guarded_mlp_readiness_cli_v1::tests::scoped_warm_cli_is_explicit_and_refuses_other_profiles_and_policy_combinations',
        'native_guarded_mlp_readiness_cli_v1::tests::scoped_warm_publication_deadline_requires_both_pre_and_post_write_to_be_inside',
        'state_roster::guarded_mlp_decode_v1::scoped_currentness_v1::tests::scoped_warm_last_layer_advances_to_tail_not_the_next_forward',
        'state_roster::guarded_mlp_decode_v1::scoped_currentness_v1::tests::scoped_warm_slot_commits_only_after_closed_call_and_keeps_allocation_plateau',
        'state_roster::guarded_mlp_decode_v1::scoped_currentness_v1::tests::scoped_warm_slot_error_leaves_no_commit_or_reusable_ledger',
        'state_roster::guarded_mlp_decode_v1::scoped_currentness_v1::tests::scoped_warm_slot_missing_owner_roster_refuses_without_execution',
        'state_roster::guarded_mlp_decode_v1::scoped_currentness_v1::tests::scoped_warm_slot_refuses_first_use_fresh_extent_and_paired_mode_before_dispatch',
        'state_roster::guarded_mlp_decode_v1::scoped_currentness_v1::tests::scoped_warm_slot_refuses_stale_bank_generation_and_wrong_layer',
        'state_roster::guarded_mlp_decode_v1::scoped_currentness_v1::tests::scoped_warm_slot_unwind_quarantines_ledger_without_advancing_generation',
    ],
    'worker-bin-test': [

    ],
    'worker-readiness-test': [
        'scoped_warm_executable_routes_selector_without_fallback_or_stdin_relock',
    ],
    'worker-wire-test': [

    ],
}
WORKER_SUMMARY_ADDITIONS = [21, 0, 1, 0]
TOTAL_SOURCES = 1033
GENERATION = 'scoped-currentness-coupled-v2'
WHOLE_WALL, LEAF_WALL, CLEANUP_RESERVE = 3600, 1800, 50
HISTORY = {
    'runtime-complete.json': (2250677, 'bf0fa20a37ff3bf748ef5984df0dceb5cd1c453ef5c05c7111779b31cda33720'),
    'runtime-sources.json': (407038, 'f8024397177d83cdd1d07040b55c97afc5190988676bb80f67a9af4ae0554718'),
    'runtime-tests.stdout': (129591, 'a058eaeeeb020ed35b4d9e538b33987cd469f3129ad5e42fc15721adf2b0d70d'),
    'runtime-list.stdout': (121470, 'bf162ed908a2a2ffed364dd23c00d2c7fba30773d3707e93c8afd7f724ec9534'),
    'worker-complete.json': (1746409, '4a016b7e09b0cc6f9b4bd32c98a5713f24709bd6509564a589c29b0478f7e337'),
    'worker-sources.json': (428240, '2566997ffb73fb40812688178f4494e48c1db4c8e7522717d54b9176994fd3e1'),
    'worker-tests.stdout': (83994, 'cd102e0f2698b75a1978b16f43492db8575ce63889819bbe09e45c7804a43ab2'),
    'worker-list.stdout': (79004, '6e7482203e0f3fba139d94107ce5952a41ca70f45e7c35a76ca589acad0c32da'),
    'runtime-docs.stdout': (2610, 'ae232b746889e201c68b98493db857a97d110dabb9dd32cee7fdaa11a5d0b8d8'),
    'legacy-docs.stdout': (2161, '95b8958d401e6e40ab05df7310915287a68270d301e7164826349c1c068c52d5'),
}
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
    'mixed-bank-tests': PAIRED + 'retained::mixed_bank::tests::',
    'scoped-checkpoint-tests': 'device::gfx950::scoped_currentness::tests::',
    'scoped-group-tests': 'engineering_gfx950::peer::scoped_currentness::tests::',
    'scoped-layer-tests': PAIRED + 'retained::scoped_layer::tests::',
}
RUNTIME_NEW_NAMES = (
    'device::gfx950::scoped_currentness::tests::scoped_checkpoint_keeps_local_checks_between_two_root_probes_without_discovery',
    'device::gfx950::scoped_currentness::tests::scoped_entry_and_exit_each_discover_and_compare_both_retained_participants',
    'device::gfx950::scoped_currentness::tests::every_scoped_engine_fault_poisons_both_and_stops_at_the_original_boundary',
    'device::gfx950::scoped_currentness::tests::every_scoped_engine_unwind_poisons_both_without_later_observation',
    'device::gfx950::scoped_currentness::tests::scoped_deadlines_refuse_entry_and_expiry_after_every_observed_effect',
    'device::gfx950::scoped_currentness::tests::scoped_roster_count_duplicate_zero_and_substitution_are_terminal',
    'device::gfx950::scoped_currentness::tests::scoped_root_or_generation_drift_rejects_before_local_predicates',
    'device::gfx950::scoped_currentness::tests::scoped_persistent_content_change_without_generation_is_caught_at_full_exit',
    'device::gfx950::scoped_currentness::tests::scoped_transient_content_change_and_revert_is_an_explicit_temporal_limitation',
    'device::gfx950::scoped_currentness::tests::scoped_exit_is_bound_to_entry_even_if_retained_snapshots_are_replaced',
    'device::gfx950::scoped_currentness::tests::scoped_closed_or_poisoned_windows_never_resume',
    'device::gfx950::scoped_currentness::tests::scoped_counter_overflow_is_terminal_before_unaccounted_work',
    'device::gfx950::scoped_currentness::tests::scoped_rank_checkpoint_preserves_exact_one_rank_predicates_and_full_exit',
    'device::gfx950::scoped_currentness::tests::scoped_rank_substitution_and_out_of_range_poison_window_and_selected_device',
    'device::gfx950::scoped_currentness::tests::scoped_rank_every_fault_and_unwind_stops_and_invalidates_whole_window',
    'device::gfx950::scoped_currentness::tests::scoped_rank_deadline_and_generation_drift_are_terminal_before_later_effects',
    'engineering_gfx950::peer::scoped_currentness::tests::scoped_group_identity_rejects_each_substitution_without_native_contexts',
    'engineering_gfx950::peer::combined_mlp_state_v1::paired::retained::scoped_layer::tests::scoped_closed_layer_uses_one_native_and_fake_ordering_engine',
    'engineering_gfx950::peer::combined_mlp_state_v1::paired::retained::scoped_layer::tests::scoped_closed_layer_every_boundary_failure_quarantines_before_publication',
    'engineering_gfx950::peer::combined_mlp_state_v1::paired::retained::scoped_layer::tests::scoped_closed_layer_every_boundary_unwind_quarantines_before_publication',
    'engineering_gfx950::peer::combined_mlp_state_v1::paired::retained::scoped_layer::tests::scoped_warm_admission_requires_ready_consumed_reuse_without_completed_proof',
    'engineering_gfx950::peer::combined_mlp_state_v1::paired::retained::scoped_layer::tests::scoped_hidden_validation_requires_exact_finite_rank_pair',
    'engineering_gfx950::peer::combined_mlp_state_v1::paired::retained::scoped_layer::tests::scoped_public_counts_are_data_only_and_preserve_rank_call_totals',
    'engineering_gfx950::peer::combined_mlp_state_v1::paired::retained::scoped_layer::tests::scoped_closed_call_keeps_original_paired_timeout_cap',
)
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
    require(len(paths) == len(set(paths)) == TOTAL_SOURCES, '820 runtime + final worker +4 controller/helper sources')
    rows = {str(path.relative_to(ROOT)): h.pin(path) for path in sorted(paths)}
    require(sum(row['bytes'] for row in rows.values()) <= 64 << 20
            and sum(name.startswith('fe2o3/') for name in rows) == 820
            and sum(name.startswith(WORKER_REL) for name in rows) == WORKER_FILES, 'closed source tree')
    return rows


def lineage(h, w, c, inputs, before, readset):
    proposals = {'runtime-proposal.json': RUNTIME_PROPOSAL_SHA,
        'primitive-proposal.json': PRIMITIVE_PROPOSAL_SHA,
        'consumer-proposal.json': CONSUMER_PROPOSAL_SHA,
        'selector-proposal.json': SELECTOR_PROPOSAL_SHA,
        'consumer-repair.json': CONSUMER_REPAIR_SHA}
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
    worker, runtime = (c.parse(bodies[kind + '-complete.json']) for kind in ('worker', 'runtime'))
    maps = {kind: c.parse(bodies[kind + '-sources.json']) for kind in ('worker', 'runtime')}
    for kind, receipt, count, schema in (
            ('worker', worker, 9, 'ferric-guarded-mlp-readiness40-shared-full-worker-cpu-v1'),
            ('runtime', runtime, 23, 'ferric-guarded-mlp-terminal-pair-cpu-v1')):
        require(receipt['schema'] == schema and receipt['passed'] is True and receipt['failure'] is None
                and receipt['postcheck_errors'] == [] and receipt['source_unchanged'] is True
                and receipt['gpu_execution'] is False
                and receipt['input_sources'] == receipt['final_sources'] == maps[kind]
                and w.compact({'x': receipt['raw']['sources-after.json']})
                    == w.compact({'x': readset[kind + '-sources.json']})
                and len(receipt['phases']) == count and all(row['exit_code'] == 0 and row['natural_exit'] is True
                    and row['reaped'] is True and row['process_group_absent'] is True
                    and row['forced_cleanup'] is False for row in receipt['phases']),
                'actual clean qualified predecessor')
    require(worker['tool_pins'] == runtime['tool_pins'] == inputs['tool_pins']
            and worker['limits'] == runtime['limits'], 'unchanged toolchain and resource contract')
    old = w.compact(maps['worker'])
    runtime_map = {name: row for name, row in w.compact(maps['runtime']).items() if name.startswith('fe2o3/')}
    require(len(runtime_map) == 815 and runtime_map == {name: row for name, row in old.items()
            if name.startswith('fe2o3/')}, 'same 815 qualified runtime bodies')
    old = {name: row for name, row in old.items() if name.startswith(('fe2o3/', WORKER_REL))}
    require(len(old) == 1020 and sum(name.startswith(WORKER_REL) for name in old) == 205,
            'actual 205-worker source base')
    require(worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and set(worker['artifacts']) == set(w.TARGETS) | {'worker'}
            and worker['readiness40_shared_full_source_added'] is True
            and worker['global_currentness_policy_changed'] is False
            and worker['default_policy_changed'] is False, 'actual previous worker contracts')
    parsed = {name: c.parse(bodies[name]) for name in proposals}
    rp, pp, cp, sp = (parsed[name] for name in ('runtime-proposal.json',
        'primitive-proposal.json', 'consumer-proposal.json', 'selector-proposal.json'))
    require(rp['schema'] == 'ferric-scoped-currentness-routed-source-v1'
            and rp['primitive_manifest'] == c.pin(bodies['primitive-proposal.json'])
            and len(rp['files']) == 22 and rp['replacement_rows'] == 17 and rp['addition_rows'] == 5
            and rp['runtime_total_sources_conditional'] == 820
            and rp['runtime_ordinary_tests_added'] == 24 and rp['doctest_change'] is False
            and rp['runtime_outer_owner_wiring_present'] is True
            and rp['individual_context_wiring_present'] is True
            and rp['native_execution'] is False and rp['performance_claim'] is False
            and rp['currentness_equivalence_claim'] is False, 'closed real routed runtime proposal')
    require(pp['schema'] == 'ferric-scoped-currentness-primitive-source-v1'
            and len(pp['files']) == 6
            and cp['schema'] == 'ferric-scoped-warm-layer-worker-source-proposal-v1'
            and len(cp['files']) == 4 and cp['declared_tests'] == 7 and cp['profile_enabled'] is False,
            'exact primitive lineage and genuine worker consumer')
    require(sp['schema'] == SELECTOR_SCHEMA and len(sp['files']) == SELECTOR_ROWS
            and all(sp.get(key) == value for key, value in SELECTOR_FIELDS.items()),
            'reviewed explicit activated selector contract')
    require(w.compact({'x': sp['requires']['runtime']})['x'] == c.pin(bodies['runtime-proposal.json'])
            and w.compact({'x': sp['requires']['consumer']})['x'] == c.pin(bodies['consumer-proposal.json'])
            and [{key: row[key] for key in ('path', 'before', 'after')}
                 for row in sp['requires']['consumer']['files']] == cp['files']
            and w.compact({'x': sp['base']['worker_receipt']})['x'] == c.pin(bodies['worker-complete.json'])
            and w.compact({'x': sp['base']['worker_sources']})['x'] == c.pin(bodies['worker-sources.json']),
            'selector requires this exact runtime, consumer and actual worker base')
    repaired = parsed['consumer-repair.json']
    require(repaired['schema'] == cp['schema']
            and w.compact({'x': repaired['predecessor_manifest']})['x'] == c.pin(bodies['consumer-proposal.json'])
            and repaired['source_generation'] == 'scoped-consumer-v2-module-path-repair'
            and repaired['declared_tests'] == cp['declared_tests'] == 7
            and all(repaired[key] is False for key in ('compiled', 'tested', 'native_execution',
                'canonical_changed', 'profile_enabled'))
            and len(repaired['files']) == len(cp['files']) == 4,
            'explicit repaired consumer with original selector lineage retained')
    repair = repaired['repair']
    require(repair == dict(
        path='adapters/tp-peer-finite-engineering-worker-v1/src/state_roster/guarded_mlp_decode_v1/scoped_currentness_v1.rs',
        before_call='crate::native_forward::checked_layer_hidden_pair',
        after_call='crate::native_catalog::forward::checked_layer_hidden_pair',
        changed_occurrences=1, other_three_postimages_unchanged=True, tests_unchanged=True),
        'one declared registered-module path repair only')
    require(sorted(repaired['new_tests']['worker_library']) == sorted(
        name for names in sp['consumer_tests'].values() for name in names),
        'all seven original consumer names preserved')
    for old_row, new_row in zip(cp['files'], repaired['files']):
        require(old_row['path'] == new_row['path'] and old_row['before'] == new_row['before']
                and (new_row['after'] != old_row['after'] if old_row['path'] == repair['path']
                     else new_row == old_row), 'one cumulative postimage transition')
    repaired_body = c.read(ROOT / ('ferric/' + repair['path']))
    before_call, after_call = repair['before_call'].encode(), repair['after_call'].encode()
    old_pin = next(row['after'] for row in cp['files'] if row['path'] == repair['path'])
    require(repaired_body.count(after_call) == 1 and before_call not in repaired_body
            and c.pin(repaired_body.replace(after_call, before_call)) == old_pin,
            'byte-exact single-call repair reverses to original consumer postimage')
    primitive = {row['path']: row for row in pp['files']}
    expected, overlay = dict(old), set()
    for source, prefix in ((rp, 'fe2o3/'), (repaired, 'ferric/'), (sp, 'ferric/')):
        paths = [row['path'] for row in source['files']]
        require(len(paths) == len(set(paths)), 'unique proposal paths')
        for row in source['files']:
            path, name = row['path'], prefix + row['path']
            require(w.ordinary_relative(path) and path.endswith('.rs')
                    and (path.startswith('crates/fe2o3-kfd/') if prefix == 'fe2o3/'
                         else name.startswith(WORKER_REL))
                    and expected.get(name) == row['before'], 'ordered exact source preimage/absence')
            if source is rp and path in primitive:
                require(row['primitive_postimage'] == primitive[path]['after']
                        and row['before'] == primitive[path]['before'], 'primitive source chain')
            expected[name] = row['after']
            overlay.add(name)
    require(sum(name.startswith('fe2o3/') for name in expected) == 820
            and sum(name.startswith(WORKER_REL) for name in expected) == WORKER_FILES
            and inputs['overlay'] == sorted(overlay)
            and {name: row for name, row in inputs['files'].items() if name.startswith(('fe2o3/', WORKER_REL))}
                == expected and w.compact(before) == inputs['files'], 'closed composed source map')
    baselines = {}
    for kind, receipt, label in (('runtime', runtime, 'kfd-tests'), ('worker', worker, 'worker-tests')):
        raw = bodies[kind + '-tests.stdout'].decode()
        previous = w.outcomes(normalized(raw) if kind == 'runtime' else raw)
        require(previous == receipt['tests'][label] and w.inventory(bodies[kind + '-list.stdout'].decode())
                == sorted(w.statuses(previous)), 'all original named outcomes and inventory')
        for suffix, raw_label in (('-tests.stdout', label + '.stdout'),
                                  ('-list.stdout', label.replace('-tests', '-list') + '.stdout')):
            require(w.compact({'x': receipt['raw'][raw_label]})
                    == w.compact({'x': readset[kind + suffix]}), 'actual historical raw receipt join')
        baselines[kind] = previous
    new_runtime = list(RUNTIME_NEW_NAMES)
    target_order = ('worker-lib', 'worker-bin-test', 'worker-readiness-test', 'worker-wire-test')
    require(set(WORKER_NEW_NAMES) == set(w.TARGETS)
            and WORKER_SUMMARY_ADDITIONS == [len(WORKER_NEW_NAMES[name]) for name in target_order],
            'exact target-owned additive worker census')
    require(WORKER_NEW_NAMES == sp['composed_new_tests']
            and set(sp['new_tests']) == set(sp['consumer_tests']) == set(w.TARGETS)
            and sum(map(len, sp['consumer_tests'].values())) == cp['declared_tests'] == 7
            and all(sorted(sp['new_tests'][key] + sp['consumer_tests'][key]) == WORKER_NEW_NAMES[key]
                    for key in target_order), 'exact declaration-owned worker test composition')
    new_worker = [name for names in WORKER_NEW_NAMES.values() for name in names]
    require(len(new_runtime) == len(set(new_runtime)) == 24
            and {name.rsplit('::', 1)[-1] for name in new_runtime} == set(rp['new_tests'])
            and len(new_worker) == len(set(new_worker)) == sum(WORKER_SUMMARY_ADDITIONS)
            and not set(new_runtime) & set(w.statuses(baselines['runtime']))
            and not set(new_worker) & set(w.statuses(baselines['worker'])), 'exact disjoint additive names')
    declared = {'runtime': set(), 'worker': set()}
    for name in overlay:
        kind = 'runtime' if name.startswith('fe2o3/') else 'worker'
        declared[kind].update(re.findall(r'#\[test\]\s*fn\s+(\w+)\s*\(', c.read(ROOT / name).decode()))
    for kind, names in (('runtime', new_runtime), ('worker', new_worker)):
        require({name.rsplit('::', 1)[-1] for name in names} <= declared[kind], 'source-declared additions')
    require((baselines['runtime']['passed'], baselines['runtime']['ignored']) == (1119, 8)
            and (baselines['worker']['passed'], baselines['worker']['ignored']) == (704, 4),
            'actual full-suite baselines')
    require(c.pin(bodies['runtime-docs.stdout'])
            == w.compact({'x': runtime['raw']['interface-doc-tests.stdout']})['x'], 'actual ten-doc raw join')
    return dict(test_names=new_runtime, worker_test_names=new_worker), baselines, \
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
    require(all(value is not None for value in (SELECTOR_PROPOSAL_SHA, SELECTOR_SCHEMA, SELECTOR_FIELDS,
            SELECTOR_ROWS, WORKER_FILES, WORKER_NEW_NAMES, WORKER_SUMMARY_ADDITIONS, TOTAL_SOURCES)),
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
        require(set(inputs) == {'schema', 'source_generation', 'files', 'tool_pins', 'lineage', 'overlay', 'cache_manifest'}
                and inputs['schema'] == 'ferric-guarded-mlp-scoped-currentness-cpu-input-v1'
                and inputs['source_generation'] == GENERATION, 'closed input schema')
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
        overlay = [str(ROOT / name) for name in inputs['overlay']]
        leaf('rustfmt', [*fmt, *overlay], 120)
        formatted = sources(h)
        changed = sorted(name for name in formatted if formatted[name] != preformat[name])
        require(set(formatted) == set(preformat) and set(changed) <= set(inputs['overlay']), 'format only the exact composed runtime/worker overlay')
        h.save('sources-before.json', formatted)
        leaf('rustfmt-check', [*fmt, '--check', *overlay], 120, formatted)
        leaf('rustc-version', [str(h.TOOLCHAIN / 'rustc'), '--version', '--verbose'], 60, formatted)
        cargo = str(h.TOOLCHAIN / 'cargo')
        common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(RUNTIME / 'Cargo.toml'), '-p', 'fe2o3-kfd']
        selected = [*common, '--features', 'engineering-gfx950', '--lib', '--tests']
        leaf('runtime-metadata', [cargo, 'metadata', '--offline', '--locked', '--format-version', '1',
                                 '--manifest-path', str(RUNTIME / 'Cargo.toml')], 120, formatted)
        external, local['runtime'] = runtime_metadata(h, w, c)
        h.save('runtime-dependencies-before.json', external)
        leaf('default-check', [cargo, 'check', *common, '--no-default-features'], tested=formatted)
        leaf('kfd-tests-build', [cargo, 'test', *selected, '--no-run', '--message-format=json'], tested=formatted)
        records = h.build_records(OUT / 'kfd-tests-build.stdout')
        require(sum(row.get('reason') == 'compiler-artifact' and row.get('manifest_path')
                == str(RUNTIME / 'crates/fe2o3-kfd/Cargo.toml') and row.get('profile', {}).get('test') is True
                for row in records) == 6, 'exact six runtime test products')
        for role, (name, kind, source) in KFD_TARGETS.items():
            artifact = h.select_artifact(records, 'fe2o3-kfd', name, kind, True)
            row = artifact['cargo_artifact']
            require(row['target']['kind'] == [kind] and row['target']['src_path'] == str(RUNTIME / 'crates/fe2o3-kfd' / source)
                    and row['features'] == ['default', 'engineering-gfx950']
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
                and (value['passed'], value['failed'], value['ignored']) == (1143, 0, 8), 'all old outcomes plus 24 scoped additions')
        tests['kfd-tests'] = value
        for label, prefix in FOCUS.items():
            leaf(label, [cargo, 'test', *common, '--features', 'engineering-gfx950', '--lib', prefix,
                         '--', '--test-threads=1'], 180, formatted)
            value = w.outcomes(normalized((OUT / (label + '.stdout')).read_text()))
            named = {name: status for name, status in expected.items() if name.startswith(prefix)}
            require(named and w.statuses(value) == named and len(value['summaries']) == 1
                    and value['summaries'][0]['filtered_out'] == 1128 - len(named)
                    and value['failed'] == value['ignored'] == 0, 'focused exact library scope')
            tests[label] = value
        docs = [*common, '--features', 'engineering-gfx950', '--doc', DOC_FILTER]
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
        wc = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(WORKER / 'Cargo.toml')]
        ws = [*wc, '--lib', '--bin', w.PACKAGE, '--test', 'readiness_cli', '--test', 'shared_wire']
        leaf('metadata', [cargo, 'metadata', '--offline', '--locked', '--format-version', '1',
                         '--manifest-path', str(WORKER / 'Cargo.toml')], 120, formatted, WORKER)
        worker_external, local['worker'] = w.metadata_contract(h)
        for path, rows in worker_external.items():
            require(path not in external or external[path] == rows, 'shared dependency unchanged between resolutions')
            external[path] = rows
        h.save('dependencies-before.json', external)
        leaf('worker-tests-build', [cargo, 'test', *ws, '--no-run', '--message-format=json'], tested=formatted, cwd=WORKER)
        records = h.build_records(OUT / 'worker-tests-build.stdout')
        require(sum(row.get('reason') == 'compiler-artifact' and row.get('manifest_path') == str(WORKER / 'Cargo.toml')
                    and row.get('profile', {}).get('test') is True for row in records) == 4, 'four worker test products')
        for role, (name, kind, source) in w.TARGETS.items():
            artifacts[role] = w.selected_artifact(h, records, name, kind, source, True)
        cli_executable_pretest = w.selected_artifact(h, records, w.PACKAGE, 'bin', 'src/main.rs', False)
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
                == sorted(name for name, status in expected_worker.items() if status == 'ignored'), 'all old worker names plus exact consumer/selector additions')
        require(h.pin(Path(cli_executable_pretest['pin']['path'])) == cli_executable_pretest['pin'],
                'real executable unchanged immediately before integration tests')
        leaf('worker-tests', [cargo, 'test', *ws, '--', '--test-threads=1'], tested=formatted, cwd=WORKER)
        require(h.pin(Path(cli_executable_pretest['pin']['path'])) == cli_executable_pretest['pin'],
                'real executable unchanged across actual EOF CLI probes')
        cli_executable_unchanged = True
        value = w.outcomes((OUT / 'worker-tests.stdout').read_text())
        require(w.statuses(value) == expected_worker and value['summaries'] == worker_summaries
                and (value['passed'], value['failed'], value['ignored'])
                    == (704 + sum(WORKER_SUMMARY_ADDITIONS), 0, 4),
                'all actual old worker outcomes plus exact reviewed additions')
        tests['worker-tests'] = value
        leaf('worker-build', [cargo, 'build', *wc, '--bin', w.PACKAGE, '--message-format=json'], tested=formatted, cwd=WORKER)
        artifacts['worker'] = w.selected_artifact(h, h.build_records(OUT / 'worker-build.stdout'),
                                                w.PACKAGE, 'bin', 'src/main.rs', False)
        require(artifacts['worker']['pin'] == cli_executable_pretest['pin'],
                'final worker ELF equals executable exercised by integration tests')
        require(len({row['pin']['path'] for row in artifacts.values()}) == 11
                and [row['label'] for row in phases] == list(PHASES), 'eleven distinct products and exact 26 phases')
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
    result = dict(schema='ferric-guarded-mlp-scoped-currentness-cpu-v1', passed=failure is None,
        failure=failure, postcheck_errors=errors, source_generation=GENERATION, input_manifest=input_pin,
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
        full_model_long_request_enabled=False, gpu_execution=False, gpu_qualified=False,
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
