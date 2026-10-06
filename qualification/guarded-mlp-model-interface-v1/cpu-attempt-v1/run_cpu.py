"""Qualify the guarded model interface without executing ignored GPU tests."""

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
EXPECTED_ROOT = E / 'guarded-mlp-model-interface-cpu-v228-v1'
SOURCE = ROOT / 'fe2o3'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
GENERATION = '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'
BASE = E / 'guarded-mlp-deferred-binding-cpu-v228-v1'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
LINEAGE = {
    'base_complete': (BASE / 'evidence/complete.json', 1249931,
                      '76c7d14beaecec2f63fff67828ff40cbe5ac439c77f27b0e621da7659ab42ca1'),
    'base_sources': (BASE / 'evidence/sources-after.json', 320662,
                     '4b7ef5c6bc235dcd307927aaff30608758005cb323dfa8a6e1a2aa48f8efe013'),
    'base_stdout': (BASE / 'evidence/kfd-tests.stdout', 122389,
                    'ae006359e3b77edc5ab769329bdc1f67f87314681c7aa8be1a199846273616b8'),
    'base_controller': (BASE / 'run_cpu.py', 33240,
                        '5fb5766ccbd6c0001b6bdc9ec957dee5adfe3d13b2db46846c10b01fad1313e2'),
}
CRATES = ('fe2o3-amd-target', 'fe2o3-amdhsa-loader', 'fe2o3-aql', 'fe2o3-drm-uapi',
          'fe2o3-hsaco', 'fe2o3-kfd', 'fe2o3-kfd-uapi', 'fe2o3-runtime-model', 'fe2o3-target-spec')
OWNER_PREFIX = 'engineering_gfx950::peer::combined_mlp_state_v1::tests::'
MEMORY_PREFIX = 'memory_linux::combined_mlp_state_v1::tests::'
NATIVE_PREFIX = OWNER_PREFIX + 'native::'
NATIVE_TEST = NATIVE_PREFIX + 'native_stable_guarded_r2_v1'
PRIOR_ADDED = tuple(NATIVE_PREFIX + name for name in (
    'native_reference_anchors_preserve_both_roundings_and_special_finite_values',
    'native_case_matrix_covers_each_guard_rank_and_validator_boundary',
    'native_request_rejects_duplicate_devices_and_unknown_fields',
)) + (MEMORY_PREFIX + 'combined_memory_diagnostic_seed_preserves_objects_and_refuses_wrong_extent',)
PAIRED_PREFIX = 'engineering_gfx950::peer::combined_mlp_state_v1::paired::tests::'
PAIRED_PASSED = tuple(PAIRED_PREFIX + name for name in (
    'paired_coordinator_exposes_both_batches_before_waiting',
    'paired_every_failure_boundary_poisoned_before_and_after_effect',
    'paired_every_deadline_boundary_and_invalid_timeout_are_terminal',
    'paired_both_terminal_guards_precede_either_completion',
    'paired_publication_accepts_early_rank_zero_producers_only',
    'paired_all_ten_signals_required_without_fabricated_read_credit',
    'paired_real_packet_bytes_encode_five_packet_dependency_graph',
    'paired_packet_constructor_refuses_duplicate_completion_addresses',
    'paired_actual_barriers_block_r2_until_both_validators_complete',
    'paired_guard_arguments_preserve_generation_lengths_and_zero_padding',
    'paired_guard_metadata_is_exact_not_an_annotation_wildcard',
    'paired_genuine_extents_and_allocation_backings_are_checked',
    'paired_cross_stage_writer_aliases_and_output_aliases_are_refused',
    'paired_native_entry_refuses_wrong_world_before_gpu_operations',
    'paired_exact_residual_policy_requires_both_full_identities',
    'paired_exact_residual_policy_refuses_all_live_root_and_peer_aliases',
    'paired_exact_residual_policy_preserves_extents_and_backing_checks',
))
PAIRED_NATIVE_PREFIX = NATIVE_PREFIX + 'paired::'
PAIRED_NATIVE_TEST = PAIRED_NATIVE_PREFIX + 'native_paired_guarded_mlp_v1'
PRIOR_PAIRED_NATIVE = tuple(PAIRED_NATIVE_PREFIX + name for name in (
    'paired_native_fixture_matches_independent_staged_arithmetic',
    'paired_native_matrix_roles_are_distinct_with_every_row_nonzero',
    'paired_native_request_refuses_wrong_images_devices_and_unknown_fields',
))
SESSION_PREFIX = 'engineering_gfx950::peer::combined_mlp_state_v1::paired::session::tests::'
SESSION_PASSED = tuple(SESSION_PREFIX + name for name in (
    'session_rearm_checks_pair_before_any_reset',
    'session_rearm_each_failure_before_and_after_effect_is_terminal',
    'session_rearm_each_deadline_boundary_including_commit_is_terminal',
    'session_rearm_invalid_timeout_zero_and_overflow_do_not_touch_state',
    'session_rearm_peer_snapshot_or_generation_drift_refuses_before_stores',
    'session_rearm_propagates_quiescence_and_final_readback_refusals',
    'session_custody_does_not_accept_replay_or_caller_mutated_completion',
    'session_transfer_phase_policy_allows_writes_only_ready',
    'session_native_entry_replay_and_transfer_errors_quarantine_both_owners',
    'session_busy_drop_and_native_generation_drift_are_terminal',
))
REUSE_PREFIX = PAIRED_NATIVE_PREFIX + 'reuse::'
REUSE_TEST = REUSE_PREFIX + 'native_paired_guarded_mlp_reuse_v1'
PRIOR_REUSE_PASSED = tuple(REUSE_PREFIX + name for name in (
    'reuse_fixture_changes_every_final_word_and_preserves_exact_positive_zeros',
    'reuse_fixture_doubles_only_up_matrix_coefficients',
))
RETAINED_PREFIX = 'engineering_gfx950::peer::combined_mlp_state_v1::paired::retained::tests::'
RETIRED_PREFIX = 'engineering_gfx950::peer::combined_mlp_state_v1::paired::arena::retired::tests::'
RETAINED_PASSED = tuple(RETAINED_PREFIX + name for name in (
    'retained_batch_checks_every_pair_before_any_reset',
    'retained_batch_every_failure_boundary_is_terminal',
    'retained_batch_every_deadline_boundary_is_terminal',
    'retained_batch_uses_the_original_absolute_deadline',
    'retained_batch_refuses_invalid_bounds_or_any_generation_before_stores',
    'retained_binding_covers_every_ordered_role_kernel_and_image',
    'retained_native_refuses_before_gpu_and_quarantines_all_custody',
    'retained_operation_drop_and_unwind_poison_all_pairs',
    'retained_exact_residual_binding_keeps_policy_and_roles_immutable',
    'retained_exact_residual_constructor_refuses_invalid_custody',
))
RETIRED_PASSED = tuple(RETIRED_PREFIX + name for name in (
    'retired_gate_accepts_intervening_completed_work_without_read_credit',
    'retired_gate_refuses_every_nonzero_signal_and_unknown_kind',
    'retired_gate_refuses_busy_regressed_or_invented_frontiers',
))
OUTER_TEST = PAIRED_PREFIX + 'paired_outer_deadline_is_shared_before_preflight_and_publication'
INTERLEAVED_PREFIX = PAIRED_NATIVE_PREFIX + 'interleaved::'
INTERLEAVED_TEST = INTERLEAVED_PREFIX + 'native_paired_guarded_mlp_interleaved_v1'
EXACT_RESIDUAL_TEST = INTERLEAVED_PREFIX + 'native_paired_guarded_mlp_exact_residual_v1'
INTERLEAVED_PASSED = tuple(INTERLEAVED_PREFIX + name for name in (
    'interleaved_schedule_keeps_bank_generations_separate_from_forwards',
    'interleaved_all_eight_final_witnesses_match_independent_hashes',
    'interleaved_signed_up_preserves_positive_zero_dense_padding',
    'interleaved_rank_roots_preserve_shared_private_and_input_identities',
    'interleaved_rank_roots_refuse_shared_roles_before_private_allocation',
    'interleaved_rank_roots_refuse_private_roles_duplicates_and_errors',
    'interleaved_exact_residual_inputs_preserve_anchors_and_change_every_output',
    'interleaved_exact_residual_payload_roles_are_pair_private',
))
OBSERVER_TEST = RETAINED_PREFIX + 'retained_diagnostic_refuses_nonquiescent_or_empty_group_before_gpu'
PRIOR_DEFERRED_PASSED = tuple(RETAINED_PREFIX + name for name in (
    'retained_unbound_identity_requires_initial_private_owners',
    'retained_unbound_storage_refuses_invalid_custody',
    'retained_unbound_bind_consumes_and_quarantines_invalid_custody',
    'retained_partial_allocation_custody_poisoned_on_drop_and_unwind',
))
FACADE_PREFIX = 'engineering_gfx950::peer::combined_mlp_state_v1::paired::facade::tests::'
MIXED_PREFIX = 'engineering_gfx950::peer::combined_mlp_state_v1::paired::retained::mixed_bank::tests::'
FACADE_PASSED = tuple(FACADE_PREFIX + name for name in (
    'facade_inputs_preserve_every_role_and_kernel_reference',
    'facade_observations_preserve_all_words_and_lagging_read_frontiers',
))
MIXED_PASSED = tuple(MIXED_PREFIX + name for name in (
    'mixed_bank_validates_every_owner_before_stores_and_publishes_last',
    'mixed_bank_initial_validation_has_no_stores_or_retired_arenas',
    'mixed_bank_bounds_timeouts_and_outer_deadline_refuse_before_effects',
    'mixed_bank_last_invalid_prefix_or_pair_performs_zero_stores',
    'mixed_bank_cross_kind_identity_collisions_refuse_before_atomic_reads',
    'mixed_bank_generations_require_one_common_nonzero_successor',
    'mixed_bank_every_error_boundary_quarantines_without_later_effects',
    'mixed_bank_every_unwind_boundary_quarantines_without_publishing',
    'mixed_bank_every_deadline_boundary_blocks_the_next_operation',
    'mixed_bank_native_refuses_context_free_custody_and_quarantines_all_entries',
    'mixed_bank_native_prefix_metadata_rejects_wrong_phase_rank_group_and_extent',
    'mixed_bank_native_unwind_keeps_group_and_typed_owners_quarantined',
))
EXTERNAL_PASSED = (
    'guarded_facade_pair_types_have_no_borrowed_lifetime',
    'guarded_facade_group_signatures_preserve_linear_custody',
    'guarded_facade_mixed_bank_signatures_are_scoped_and_explicit',
)
LIB_ADDED = FACADE_PASSED + MIXED_PASSED
NEW_PASSED = LIB_ADDED + EXTERNAL_PASSED
LIB_COUNT = 1059 + len(LIB_ADDED)
TOTAL_COUNT = 1079 + len(NEW_PASSED)
DOC_FILTER = 'engineering_gfx950_peer_combined_mlp_paired_facade_v1.rs'
DOC_COUNT = 9
COHORTS = {
    'combined-owner-tests': (OWNER_PREFIX, tuple(OWNER_PREFIX + name for name in (
        'combined_binding_matrix_admits_only_exact_owner_and_suffix_regions',
        'combined_borrowed_regions_preserve_one_real_allocation_identity',
        'combined_cleanup_selects_queue_first_and_keeps_existing_kinds_unchanged',
        'combined_generation_and_activation_rules_refuse_replay_skip_and_overflow',
        'combined_native_submit_refuses_bad_phase_before_any_context_operation',
        'combined_owner_does_not_change_legacy_token_or_public_byte_policy',
        'combined_snapshot_predicates_check_every_prefix_word_and_guard_component',
        'combined_terminal_failures_poison_without_publishing_generation_or_losing_custody',
    ))),
    'combined-memory-tests': (MEMORY_PREFIX, tuple(MEMORY_PREFIX + name for name in (
        'combined_memory_bad_mapping_and_alignment_refuse_before_any_atomic_access',
        'combined_memory_cannot_be_observed_or_rearmed_as_legacy_2192_owner',
        'combined_memory_constructs_exact_552_genuine_atomics_and_preserves_tail',
        'combined_memory_quiescent_rearm_preserves_objects_and_replaces_every_word',
        'combined_memory_wrong_request_refuses_before_construction_or_stores',
        'combined_memory_zero_generation_does_not_publish_or_mutate',
    ))),
}
IGNORED = tuple(sorted((
    'queue::dispatch_binding::tests::real_gfx950_kernel_rejects_before_fixed_dispatch_data_preparation',
    'shared_memory::gfx950_observed::queue::finite_join::tests::retained_fixed_image_passes_same_engine_intake',
    'shared_memory::gfx950_observed::queue::multiwave_join::tests::actual_multiwave_image_passes_distinct_same_engine_intake',
    NATIVE_TEST,
    PAIRED_NATIVE_TEST,
    REUSE_TEST,
    INTERLEAVED_TEST,
    EXACT_RESIDUAL_TEST,
)))
COHORTS = {label: (prefix, tuple(sorted((*names, *(n for n in (*PRIOR_ADDED, NATIVE_TEST, *PRIOR_PAIRED_NATIVE, PAIRED_NATIVE_TEST, *PRIOR_REUSE_PASSED, REUSE_TEST, *INTERLEAVED_PASSED, INTERLEAVED_TEST, EXACT_RESIDUAL_TEST)
            if n.startswith(prefix)))))) for label, (prefix, names) in COHORTS.items()}
COHORTS['paired-tests'] = (PAIRED_PREFIX, tuple(sorted((*PAIRED_PASSED, OUTER_TEST))))
COHORTS['paired-native-tests'] = (PAIRED_NATIVE_PREFIX, tuple(sorted((*PRIOR_PAIRED_NATIVE, PAIRED_NATIVE_TEST, *PRIOR_REUSE_PASSED, REUSE_TEST, *INTERLEAVED_PASSED, INTERLEAVED_TEST, EXACT_RESIDUAL_TEST))))
COHORTS['session-tests'] = (SESSION_PREFIX, tuple(sorted(SESSION_PASSED)))
COHORTS['reuse-native-tests'] = (REUSE_PREFIX, tuple(sorted((*PRIOR_REUSE_PASSED, REUSE_TEST))))
COHORTS['retained-tests'] = (RETAINED_PREFIX, tuple(sorted((*RETAINED_PASSED, OBSERVER_TEST, *PRIOR_DEFERRED_PASSED))))
COHORTS['retired-tests'] = (RETIRED_PREFIX, tuple(sorted(RETIRED_PASSED)))
COHORTS['interleaved-native-tests'] = (INTERLEAVED_PREFIX, tuple(sorted((*INTERLEAVED_PASSED, INTERLEAVED_TEST, EXACT_RESIDUAL_TEST))))
COHORTS['facade-tests'] = (FACADE_PREFIX, tuple(sorted(FACADE_PASSED)))
COHORTS['mixed-bank-tests'] = (MIXED_PREFIX, tuple(sorted(MIXED_PASSED)))
OVERLAY = tuple('crates/fe2o3-kfd/src/' + name for name in (
    'lib.rs', 'engineering_gfx950.rs', 'engineering_gfx950_peer.rs',
    'engineering_gfx950_peer_combined_mlp_paired_v1.rs',
    'engineering_gfx950_peer_combined_mlp_paired_retained_v1.rs',
    'engineering_gfx950_peer_combined_mlp_paired_interleaved_native_v1_tests.rs',
    'engineering_gfx950_peer_combined_mlp_paired_mixed_bank_v1.rs',
    'engineering_gfx950_peer_combined_mlp_paired_mixed_bank_v1_tests.rs',
    'engineering_gfx950_peer_combined_mlp_paired_facade_v1.rs',
    'engineering_gfx950_peer_combined_mlp_paired_facade_v1_tests.rs',
)) + ('crates/fe2o3-kfd/tests/guarded_mlp_facade_v1.rs',)
ADDED_SOURCES = set(OVERLAY[-5:])
SOURCE_COUNT = 802 + len(ADDED_SOURCES)
TARGETS = {
    'kfd-lib': ('fe2o3_kfd', 'lib', 'src/lib.rs'),
    'engineering-worker-test': ('fe2o3-gfx950-engineering-worker', 'bin', 'src/bin/gfx950_engineering_worker.rs'),
    'guarded-facade-test': ('guarded_mlp_facade_v1', 'test', 'tests/guarded_mlp_facade_v1.rs'),
    'debug-trap-test': ('kfd_debug_trap_live', 'test', 'tests/kfd_debug_trap_live.rs'),
    'telemetry-env-test': ('target_debug_telemetry_env_v1', 'test', 'tests/target_debug_telemetry_env_v1.rs'),
    'telemetry-test': ('target_debug_telemetry_v1', 'test', 'tests/target_debug_telemetry_v1.rs'),
}
WHOLE_WALL, LEAF_WALL, CLEANUP_RESERVE = 3600, 1800, 50
PHASES = ('rustc-version', 'metadata', 'default-check', 'kfd-tests-build', 'kfd-list',
          'kfd-ignored', 'kfd-tests', 'combined-owner-tests', 'combined-memory-tests',
          'paired-tests', 'paired-native-tests', 'session-tests', 'reuse-native-tests',
          'retained-tests', 'retired-tests', 'interleaved-native-tests', 'facade-tests',
          'mixed-bank-tests', 'interface-doc-list', 'interface-doc-tests')


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def load_exact(name, path, digest):
    require(path.resolve(strict=True) == path and path.is_file() and not path.is_symlink(),
            'helper must be canonical ordinary source')
    before = path.stat()
    require(before.st_size <= 1 << 20, 'helper size bound')
    body = path.read_bytes()
    after = path.stat()
    require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns)
            == (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)
            and hashlib.sha256(body).hexdigest() == digest, 'helper source pin mismatch')
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(body, str(path), 'exec'), module.__dict__)
    return module


def compact(rows):
    return {name: {key: row[key] for key in ('bytes', 'sha256')} for name, row in rows.items()}


def normalized_libtest(text):
    names = (
        'queue_linux::tests::payload_release_failure_after_event_destroy_is_process_terminal',
        'queue_linux::tests::unpublished_custody_cleanup_failure_is_process_terminal',
    )
    for name in names:
        block = 'test ' + name + ' ... \nrunning 1 test\nok'
        text, count = re.subn('^' + re.escape(block) + r'(?=\n|\Z)',
                              'test ' + name + ' ... ok', text, flags=re.M)
        require(count <= 1, 'duplicate known abort-child parent block: ' + name)
    return text


def test_outcomes(path):
    text = normalized_libtest(path.read_text())
    summaries = [dict(status=status, passed=int(passed), failed=int(failed),
                      ignored=int(ignored), measured=int(measured), filtered_out=int(filtered))
                 for status, passed, failed, ignored, measured, filtered in re.findall(
                     r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; '
                     r'(\d+) measured; (\d+) filtered out;', text, re.M)]
    named = [dict(name=name, outcome=outcome) for name, outcome in re.findall(
        r'^test (.+?) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', text, re.M)]
    require(summaries, 'no actual libtest summaries: ' + str(path))
    for status, field in (('ok', 'passed'), ('FAILED', 'failed'), ('ignored', 'ignored')):
        require(sum(row['outcome'] == status for row in named)
                == sum(row[field] for row in summaries), 'named libtest census differs from summaries')
    return dict(summaries=summaries, named=named,
                passed=sum(row['passed'] for row in summaries),
                failed=sum(row['failed'] for row in summaries),
                ignored=sum(row['ignored'] for row in summaries))


def named_statuses(value):
    rows = value['named']
    result = {row['name']: row['outcome'] for row in rows}
    require(len(result) == len(rows) and all(re.fullmatch(r'[A-Za-z0-9_:]+', name)
            for name in result), 'closed unique test names required')
    return result


def full_outcomes(path, expected, old_summaries):
    value = test_outcomes(path)
    summaries = [dict(row) for row in old_summaries]
    summaries[0]['passed'] += len(LIB_ADDED)
    summaries.insert(2, dict(status='ok', passed=len(EXTERNAL_PASSED), failed=0, ignored=0,
                             measured=0, filtered_out=0))
    require(value['summaries'] == summaries and named_statuses(value) == expected
            and (value['passed'], value['failed'], value['ignored']) == (TOTAL_COUNT - 8, 0, 8),
            'full KFD six-target outcomes differ from exact baseline-plus-interface roster')
    return value


def lineage_contract(h, inputs, before, readset):
    lineage = inputs['source_lineage']
    require(set(lineage) == set(LINEAGE) | {'overlay'}, 'closed diagnostic source lineage')
    bodies = {}
    for name, (path, size, digest) in LINEAGE.items():
        require(path.resolve(strict=True) == path and size <= 16 << 20, 'lineage path/size')
        row = h.pin(path)
        require(row == lineage[name] and row['bytes'] == size and row['sha256'] == digest,
                'literal source lineage differs: ' + name)
        readset[name] = row
        bodies[name] = path.read_bytes()
        require(h.pin(path) == row, 'lineage changed while reading')
    base = json.loads(bodies['base_complete'])
    base_sources = json.loads(bodies['base_sources'])
    require(base['schema'] == 'ferric-guarded-mlp-interleaved-native-cpu-v1'
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['source_unchanged'] is True and base['gpu_execution'] is False
            and base['source_generation'] == GENERATION
            and base['input_sources'] == base['final_sources'] == base_sources
            and base['controller'] == readset['base_controller']
            and base['raw']['sources-after.json'] == readset['base_sources']
            and base['raw']['kfd-tests.stdout'] == readset['base_stdout'],
            'qualified combined-owner baseline')
    require(len(base['phases']) == 16 and all(row['exit_code'] == 0 and row['natural_exit'] is True
            and row['reaped'] is True and row['process_group_absent'] is True
            and row['forced_cleanup'] is False and row['timed_out'] is False
            and row['exception'] is None for row in base['phases']), 'baseline natural lifecycle')
    require(len(base_sources) == 804 and all(row['path'] == str(BASE / name)
            and Path(name).as_posix() == name and not Path(name).is_absolute()
            and '..' not in Path(name).parts for name, row in base_sources.items()),
            'baseline closed source roster')
    expected = {name: row for name, row in compact(base_sources).items() if name.startswith('fe2o3/')}
    require(len(expected) == 802 and set(lineage['overlay']) == set(OVERLAY)
            and {n for n, row in lineage['overlay'].items() if row['before'] is None} == ADDED_SOURCES,
            'closed guarded model interface overlay')
    for name, row in lineage['overlay'].items():
        require(set(row) == {'before', 'after'} and expected.get('fe2o3/' + name) == row['before'],
                'diagnostic preimage differs: ' + name)
        expected['fe2o3/' + name] = row['after']
        require(compact({'x': before['fe2o3/' + name]})['x'] == row['after'],
                'diagnostic postimage differs: ' + name)
    require(len(expected) == SOURCE_COUNT and compact({name: row for name, row in before.items()
            if name.startswith('fe2o3/')}) == expected, 'exact diagnostic source map differs')
    old_tests = test_outcomes(Path(readset['base_stdout']['path']))
    require(old_tests == base['tests']['kfd-tests'] and len(old_tests['summaries']) == 5
            and (old_tests['passed'], old_tests['failed'], old_tests['ignored']) == (1071, 0, 8),
            'exact baseline raw KFD census')
    names = named_statuses(old_tests)
    require(len(names) == 1079 and sorted(n for n, s in names.items() if s == 'ignored')
            == list(IGNORED), 'baseline old names/ignored roster')
    require(not set(NEW_PASSED) & set(names), 'deferred binding names collide')
    names.update({name: 'ok' for name in NEW_PASSED})
    return base, old_tests, names


def selected_tests(h, path):
    rows = h.build_records(path)
    test_rows = [row for row in rows if row.get('reason') == 'compiler-artifact'
                 and row.get('manifest_path') == str(SOURCE / 'crates/fe2o3-kfd/Cargo.toml')
                 and row.get('profile', {}).get('test') is True]
    require(len(test_rows) == 6, 'exact six KFD test artifacts required')
    artifacts = {}
    for role, (name, kind, source) in TARGETS.items():
        artifact = h.select_artifact(rows, 'fe2o3-kfd', name, kind, True)
        row = artifact['cargo_artifact']
        require(row['target']['kind'] == [kind]
                and row['target']['src_path'] == str(SOURCE / 'crates/fe2o3-kfd' / source)
                and row['filenames'].count(artifact['pin']['path']) == 1
                and set(row['features']) == {'default', 'engineering-gfx950'},
                'closed selected KFD test target/features')
        artifacts[role] = artifact
    require(len({row['pin']['path'] for row in artifacts.values()}) == 6, 'distinct selected test ELFs')
    return artifacts


def main():
    started = time.monotonic()
    deadline = started + WHOLE_WALL
    require(MIXED_PASSED and __debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]), 'python3 -B run_cpu.py INPUT_SHA')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT
            and not any(os.path.lexists(p) for p in (OUT, TARGET, TMP)), 'fresh exact outputs required')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID mismatch')
    require(shutil.disk_usage(ROOT).free >= 40 << 30, '40 GiB setup floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected nice level')
    if priority == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30),
                      (resource.RLIMIT_FSIZE, 1 << 30)):
        soft, hard = resource.getrlimit(kind)
        value = min([cap] + [x for x in (soft, hard) if x != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup')
    h = load_exact('owner_owned_supervisor', ROOT / 'supervisor.py', SUPERVISOR_SHA)
    h.ROOT, h.SOURCE, h.OUT, h.TARGET, h.TMP = ROOT, SOURCE, OUT, TARGET, TMP
    h.ROLES = {role: ('fe2o3-kfd', name, kind) for role, (name, kind, _) in TARGETS.items()}
    original_sources = h.sources
    def sources():
        return dict(original_sources(), **{'supervisor.py': h.pin(ROOT / 'supervisor.py')})
    h.sources = sources
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - CLEANUP_RESERVE - time.monotonic()))
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    before = final = input_pin = config = old_tests = None
    readset, tool_pins, external, external_after, local, artifacts, tests = {}, {}, {}, {}, {}, {}, {}
    inventory, ignored, phases, errors, failure = [], [], [], [], None
    try:
        input_pin = h.pin(ROOT / 'input-manifest.json')
        require(input_pin['sha256'] == sys.argv[1], 'literal input mismatch')
        inputs = json.loads((ROOT / 'input-manifest.json').read_bytes())
        require(set(inputs) == {'schema', 'source_generation', 'files', 'tool_pins', 'source_lineage'}
                and inputs['schema'] == 'ferric-guarded-mlp-interleaved-native-cpu-input-v1'
                and inputs['source_generation'] == GENERATION, 'input generation/fields')
        before = sources()
        require(len(before) == SOURCE_COUNT + 2 and inputs['files'] == compact(before), 'closed exact source map')
        h.save('sources-before.json', before)
        base, old_tests, expected_names = lineage_contract(h, inputs, before, readset)
        config = h.configurations()
        tool_pins = {name: h.pin(h.TOOLCHAIN / name) for name in ('rustc', 'rustdoc', 'rustfmt', 'cargo')}
        tool_pins['prlimit'] = h.pin(Path('/usr/bin/prlimit'))
        for name, size in h.SHARED_LIBRARIES.items():
            path = h.TOOLCHAIN_LIB / name
            require(path.resolve(strict=True) == path, 'tool library alias')
            tool_pins[name] = h.pin(path)
            require(tool_pins[name]['bytes'] == size, 'tool library extent')
        require(tool_pins == inputs['tool_pins'] == base['tool_pins'], 'exact qualified tool pins')
        env = dict(HOME='/home/harmenon', PATH=str(h.TOOLCHAIN) + ':/usr/bin:/bin',
                   CARGO_HOME='/home/harmenon/.cargo', CARGO_TARGET_DIR=str(TARGET),
                   LD_LIBRARY_PATH=str(TARGET / 'debug/deps') + ':' + str(h.TOOLCHAIN_LIB),
                   TMPDIR=str(TMP), RUSTC=str(h.TOOLCHAIN / 'rustc'), RUSTDOC=str(h.TOOLCHAIN / 'rustdoc'),
                   CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true',
                   CARGO_CACHE_AUTO_CLEAN_FREQUENCY='never', MALLOC_ARENA_MAX='2', RUST_BACKTRACE='1',
                   CARGO_PROFILE_DEV_OPT_LEVEL='2', CARGO_PROFILE_DEV_DEBUG='0',
                   CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
                   CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_TEST_DEBUG='0',
                   CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
                   ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
        def leaf(label, argv, seconds=LEAF_WALL):
            h.run(label, argv, env, phases, deadline, before, seconds, SOURCE)
        cargo = str(h.TOOLCHAIN / 'cargo')
        common = ['--offline', '--locked', '--jobs', '2', '-p', 'fe2o3-kfd']
        selected = [*common, '--features', 'engineering-gfx950', '--lib', '--tests']
        leaf('rustc-version', [str(h.TOOLCHAIN / 'rustc'), '--version', '--verbose'], 60)
        leaf('metadata', [cargo, 'metadata', '--offline', '--locked', '--format-version', '1'], 120)
        external, local = h.metadata_contract()
        require(set(local) == set(CRATES), 'exact nine-crate metadata closure')
        h.save('dependencies-before.json', external)
        leaf('default-check', [cargo, 'check', *common, '--no-default-features'])
        leaf('kfd-tests-build', [cargo, 'test', *selected, '--no-run', '--message-format=json'])
        artifacts = selected_tests(h, OUT / 'kfd-tests-build.stdout')
        leaf('kfd-list', [cargo, 'test', *selected, '--', '--list', '--format=terse'], 120)
        leaf('kfd-ignored', [cargo, 'test', *selected, '--', '--ignored', '--list', '--format=terse'], 120)
        inventory = h.inventory(OUT / 'kfd-list.stdout')
        ignored = h.inventory(OUT / 'kfd-ignored.stdout')
        require(inventory == sorted(expected_names) and len(inventory) == TOTAL_COUNT
                and ignored == list(IGNORED), 'compiled full/ignored KFD inventory differs')
        for prefix, names in COHORTS.values():
            require(sorted(name for name in inventory if name.startswith(prefix)) == sorted(names)
                    and set(names) & set(ignored) == ({NATIVE_TEST, PAIRED_NATIVE_TEST, REUSE_TEST, INTERLEAVED_TEST, EXACT_RESIDUAL_TEST} if prefix == OWNER_PREFIX
                        else {PAIRED_NATIVE_TEST, REUSE_TEST, INTERLEAVED_TEST, EXACT_RESIDUAL_TEST} if prefix == PAIRED_NATIVE_PREFIX
                        else {REUSE_TEST} if prefix == REUSE_PREFIX
                        else {INTERLEAVED_TEST, EXACT_RESIDUAL_TEST} if prefix == INTERLEAVED_PREFIX else set()),
                    'compiled focused inventory differs')
        leaf('kfd-tests', [cargo, 'test', *selected, '--', '--test-threads=1'])
        tests['kfd-tests'] = full_outcomes(OUT / 'kfd-tests.stdout', expected_names, old_tests['summaries'])
        for label, (prefix, names) in COHORTS.items():
            leaf(label, [cargo, 'test', *common, '--features', 'engineering-gfx950',
                         '--lib', prefix, '--', '--test-threads=1'])
            value = test_outcomes(OUT / (label + '.stdout'))
            expected = {n: expected_names[n] for n in names}
            require(named_statuses(value) == expected
                    and value['summaries'] == [dict(status='ok',
                        passed=sum(s == 'ok' for s in expected.values()), failed=0,
                        ignored=sum(s == 'ignored' for s in expected.values()), measured=0,
                        filtered_out=LIB_COUNT - len(names))], 'focused interface CPU outcomes differ')
            tests[label] = value
        doc_selected = [*common, '--features', 'engineering-gfx950', '--doc', DOC_FILTER]
        leaf('interface-doc-list', [cargo, 'test', *doc_selected, '--', '--list', '--format=terse'], 120)
        doc_names = [line.removesuffix(': test') for line in
                     (OUT / 'interface-doc-list.stdout').read_text().splitlines() if line.endswith(': test')]
        require(len(doc_names) == len(set(doc_names)) == DOC_COUNT
                and all(DOC_FILTER in name and '\\n' not in name for name in doc_names),
                'exact interface doc inventory')
        leaf('interface-doc-tests', [cargo, 'test', *doc_selected, '--', '--test-threads=1'], 180)
        doc_text = (OUT / 'interface-doc-tests.stdout').read_text()
        doc_rows = re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', doc_text, re.M)
        normalize = lambda name: name.removesuffix(' - compile fail')
        require(len(doc_rows) == DOC_COUNT and all(status == 'ok' for _, status in doc_rows)
                and {normalize(name) for name, _ in doc_rows} == {normalize(name) for name in doc_names}
                and re.findall(r'test result: (ok|FAILED)\. ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored;', doc_text)
                    == [('ok', str(DOC_COUNT), '0', '0')], 'all interface opacity doctests passed')
        tests['interface-doc-tests'] = dict(passed=DOC_COUNT, failed=0, ignored=0,
                                            inventory=doc_names, named=doc_rows)
        require([row['label'] for row in phases] == list(PHASES), 'closed twenty phases')
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
        final = sources()
        h.save('sources-after.json', final)
        require(before is not None and final == before, 'source/lock mutation')
    check('sources', source_check)
    if input_pin is not None:
        check('input', lambda: require(h.pin(ROOT / 'input-manifest.json') == input_pin, 'input drift'))
    if config is not None:
        check('configuration', lambda: require(h.configurations() == config, 'configuration drift'))
    for label, expected in dict(tool_pins, **readset).items():
        check('readset ' + label, lambda expected=expected: require(
            h.pin(Path(expected['path'])) == expected, 'authenticated input/tool drift'))
    for directory, expected in external.items():
        def dependency_check(directory=directory, expected=expected):
            path = Path(directory)
            actual = {str(p.relative_to(path)): h.pin(p) for p in h.files_below(path, packed=False)}
            external_after[directory] = actual
            require(actual == expected, 'external dependency drift')
        check('dependency ' + directory, dependency_check)
    check('dependency ledger', lambda: h.save('dependencies-after.json', external_after))
    for label, artifact in artifacts.items():
        check('artifact ' + label, lambda artifact=artifact: require(
            h.pin(Path(artifact['pin']['path'])) == artifact['pin'], 'selected test artifact drift'))
    raw = {}
    def raw_check():
        nonlocal raw
        raw = {p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()}
        for row in phases:
            for key in ('command', 'stdout', 'stderr'):
                require(raw[Path(row[key]['path']).name] == row[key], 'raw phase join')
    check('raw evidence', raw_check)
    failure = failure or ('postcheck failed' if errors else None)
    if time.monotonic() >= deadline:
        failure = failure or 'whole deadline exceeded'
    result = dict(schema='ferric-guarded-mlp-interleaved-native-cpu-v1', passed=failure is None,
                  failure=failure, postcheck_errors=errors, source_generation=GENERATION,
                  controller=h.pin(ROOT / 'run_cpu.py'), supervisor=h.pin(ROOT / 'supervisor.py'),
                  input_manifest=input_pin, source_lineage=inputs['source_lineage'] if input_pin and before else None,
                  readset=readset, baseline_tests=old_tests,
                  input_sources=before, final_sources=final, source_unchanged=before is not None and final == before,
                  phases=phases, artifacts=artifacts, tests=tests, inventory=inventory, ignored=ignored,
                  owner_cohorts={label: {'filter': prefix, 'names': sorted(names)}
                                for label, (prefix, names) in COHORTS.items()},
                  full_kfd_tests_executed='kfd-tests' in tests,
                  tool_pins=tool_pins, raw=raw, local_dependencies=local, configurations=config,
                  elapsed_seconds=time.monotonic() - started, test_artifact_phase='kfd-tests-build',
                  limits=dict(whole_seconds=WHOLE_WALL, leaf_seconds=LEAF_WALL,
                              cleanup_reserve_seconds=CLEANUP_RESERVE, cpu_seconds=h.CPU_LIMIT,
                              address_space_bytes=h.AS_LIMIT, file_bytes=h.FILE_LIMIT,
                              cache_bytes=h.CACHE_LIMIT, stream_bytes=h.STREAM_LIMIT,
                              initial_free_bytes=h.START_FREE, live_free_bytes=h.LIVE_FREE,
                              affinity=[8, 9], nice=10, cargo_jobs=2),
                  additive_private_owner=True, private_paired_coordinator_added=True,
                  private_retained_session_added=True, native_session_reuse_qualified=False,
                  private_borrow_free_pair_added=True, private_retired_proof_added=True,
                  private_deferred_binding_added=True,
                  public_engineering_guarded_interface_added=True, mixed_bank_implemented=True,
                  native_mixed_bank_qualified=False,
                  native_interleaving_qualified=False, whole_model_bank_integrated=False,
                  native_test_name=EXACT_RESIDUAL_TEST, native_test_executed=False,
                  legacy_state_v2_changed=False, legacy_profiles_changed=False,
                  gpu_execution=False, worker_integrated=False, coordinator_implemented=True,
                  native_peer_ordering_qualified=False, production_authority=False, performance_claim=False,
                  doctests_executed='interface-doc-tests' in tests, live_validation_enabled=False)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - time.monotonic()))
    try:
        h.save('complete.json' if failure is None else 'failed.json', result)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(passed=failure is None, failure=failure, phases=len(phases),
                          tests={k: {n: v[n] for n in ('passed', 'failed', 'ignored')}
                                 for k, v in tests.items()}, output=str(OUT)), sort_keys=True))
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
