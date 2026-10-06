"""Bounded full-S CPU qualification of projected-CFG compaction."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import stat
import subprocess
import sys
import time
import types

ROOT = Path(__file__).resolve().parent
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-ranked-cfg-compaction-cpu-v228-v1')
HELPER_SHA = 'a77cc7f82ebdd9518185b30b837e250abc68cc32e4858ab1ad3dc83a0ec8efe6'
BASE_COMPLETE_SHA = 'bcf85b67b2b2d2a80f9b2e4a0fa990b038d848eafc0d8117d321dbb059efdf81'
BASE_SOURCES_SHA = 'fd7b5ceab4fc2941437863cc03b8a857ab290e826e0e279757125095aeaf1472'
BASE_INPUT_SHA = '09e2c31f7a045df0591ba42747d4cb345e21109fd697a497b238f7fa7f78c281'
BASE_CONTROLLER_SHA = '69686f865ca9b7d54842889546779a033d47b11cddf262e76137cbcb9f2dcff8'
BASE_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-ranked-cfg-expansion-diagnostic-cpu-v228-v1')
BASE_SOURCE = BASE_ROOT / 'fe2o3'
BASE_FAILURE_ROOT = ROOT.parent / 'guarded-mlp-ranked-cfg-expansion-diagnostic-lowering-v228-v1'
BASE_FAILURE_SHA = 'ba277bc9fbf3593b0b797a6bba3e7f8ee4fd073b8e28dc33d245b465d0c09a59'
BASE_FAILURE_STDERR_SHA = 'f806e763a124eb460d47de32531749744f9d24a03cd240115d9f744326ee374a'
BASE_FAILURE_DIAGNOSTIC = (
    b'fe2o3 rustc extraction: production compilation general kernel verification failed: '
    b'semantic-to-ranked projection rejected semantic CFG projection exceeds the ranked block limit; '
    b'cfg-block-diagnostic-v1 blocks=2778 limit=2048 reason=above-limit '
    b'site=projected-cfg-expansion semantic_blocks=1121 '
    b'function_sha256=72f186b0c49aafc62f29a7b77a5bbca4fc458c507019297c8bea173b272fe70c '
    b'role=KernelRoot; source=Rust source edb8c73e35fa:17:1; '
    b'kernel_export=ferric_qwen3_mlp_state_guard_v1 phase=root-projection; '
    b'root ferric_qwen3_mlp_state_guard_v1 (semantic root 1, body 1)')
DAG_PROPOSAL_SHA = '4889df85ff563f94549d7f767820a5065d82f9e53da593952a96a5def2ad6bb3'
GRAPH_WORK_PROPOSAL_SHA = '1a2dc0e17f6811eef3672fc4c86c27e535f2589329c3df695f0ad91c648b725a'
MEMBERSHIP_PROPOSAL_SHA = '0e98d81d3b437e6bdb25a1a8a70608cf81a2c03fba13864e0793556e9028e8c8'
CFG_COMPACTION_PROPOSAL_SHA = 'a996f2ecae0ce3a29e091ecc50f23ef081e57feba005b621856cd55ce6889428'
CFG_COMPACTION_PREFIX = 'production_ranked_projection_v1::tests::cfg_compaction_'
CFG_COMPACTION_TESTS = tuple(CFG_COMPACTION_PREFIX + name for name in (
    'atomic_and_predicate_refusals_unchanged',
    'differential_checks_effects_and_traps',
    'edge_budget_remains_independent',
    'exact_block_boundary_and_next_refusal',
    'live_induction_layout_unchanged',
    'mixed_failure_layout',
    'multiple_semantic_blocks_preserve_traces',
    'source_wave_and_generated_identity',
    'unreachable_and_zero_eligible_unchanged',
))
CFG_COMPACTION_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/projection_04_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/cfg_compaction_v1_tests.rs': True,
}
CFG_EXPANSION_PROPOSAL_SHA = '1fe62f3e0f9a892f211cd9b0f75db9eac93d4cfec7c320b46ef55e17a7330de4'
CFG_EXPANSION_PREFIX = 'production_ranked_projection_v1::tests::cfg_projected_block_limit_diagnostic_'
CFG_EXPANSION_TESTS = tuple(CFG_EXPANSION_PREFIX + name for name in (
    'exact_and_next_boundaries',
    'expansion_counts_and_identity',
    'context_is_bounded',
))
CFG_EXPANSION_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/cfg_block_limit_diagnostic_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/cfg_block_limit_diagnostic_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/analysis_multi_split_v1_tests.rs': False,
}
BORROW_LOOKUP_PROPOSAL_SHA = '1a089d8bcee0b539e42b7870706da89c23ef60ed494f24e1c4a6ee2fa529d49b'
BORROW_LOOKUP_PREFIX = 'production_ranked_projection_v1::tests::indexed_atomic_borrow_lookup_'
BORROW_LOOKUP_TESTS = tuple(BORROW_LOOKUP_PREFIX + name for name in (
    'sorts_all_small_shuffled_inventories',
    'preserves_all_exact_place_components',
    'handles_block_and_length_boundaries',
    'charges_queries_transactionally',
    'sort_work_is_charged_and_failure_is_local',
    'bounds_repeated_bucket_queries',
    'keeps_independent_inventories',
    'real_inventory_preserves_provenance_sites',
    'real_inventory_keeps_atomic_refusals',
))
BORROW_LOOKUP_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_borrow_lookup_v1_tests.rs': True,
}
USE_LOOKUP_PROPOSAL_SHA = 'aeac5c853c40ec768e1b5add4a2decf6338d4a8a15c44abe3bc9962eab000f0e'
USE_LOOKUP_PREFIX = 'production_ranked_projection_v1::tests::indexed_atomic_use_lookup_'
USE_LOOKUP_TESTS = tuple(USE_LOOKUP_PREFIX + name for name in (
    'matches_all_small_sorted_sites',
    'handles_boundary_lengths',
    'preserves_first_duplicate_and_extreme_sites',
    'charges_exact_bound_before_query',
    'failed_charges_do_not_commit_work',
    'bounds_repeated_measured_size_queries',
    'keeps_independent_inventories',
    'real_inventory_keeps_exact_statement_binding',
    'real_inventory_keeps_atomic_refusals',
))
USE_LOOKUP_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_use_lookup_v1_tests.rs': True,
}
CENSUS_PROPOSAL_SHA = '2b6050714c19c9661bb2fc6a7fff022833136be617c6cad0f90dcfc8cb4ff8fa'
CENSUS_PREFIX = 'production_ranked_projection_v1::tests::indexed_atomic_dead_cast_census_'
CENSUS_TESTS = tuple(CENSUS_PREFIX + name for name in (
    'bounds_scratch_before_allocation',
    'charges_exact_build_and_cached_query_bounds',
    'excludes_whole_definition_and_distinguishes_sites',
    'failed_build_never_publishes_partial_state',
    'ignores_storage_and_checks_roles_and_unreachable_blocks',
    'isolates_immutable_functions_and_retains_cast_gates',
    'many_candidates_share_one_complete_scan',
    'matches_legacy_for_all_statement_positions',
    'matches_legacy_for_all_terminator_positions',
))
CENSUS_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_dead_cast_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_dead_cast_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_dead_cast_census_v1_tests.rs': True,
}
MEMBERSHIP_PREFIX = 'production_ranked_projection_v1::tests::indexed_atomic_membership_'
MEMBERSHIP_TESTS = tuple(MEMBERSHIP_PREFIX + name for name in (
    'matches_all_small_sorted_sets',
    'handles_boundary_lengths',
    'preserves_duplicate_and_extreme_ids',
    'charges_exact_bound_before_query',
    'failed_charges_do_not_commit_work',
    'bounds_repeated_measured_size_queries',
    'keeps_independent_inventories',
    'real_inventory_keeps_statement_binding',
    'preserves_guard_marker_and_escape_refusals',
))
GRAPH_WORK_PREFIX = 'production_ranked_projection_v1::tests::graph_work_diagnostic_'
GRAPH_WORK_TESTS = tuple(GRAPH_WORK_PREFIX + name for name in (
    'atomic_forwarder_preserves_cell_on_failure',
    'body_context_uses_actual_body_not_root',
    'bounded_paths_names_and_exports_are_escaped',
    'context_preserves_other_errors',
    'direct_callers_are_distinct',
    'exact_limit_and_over_limit_preserve_counter',
    'first_function_context_is_retained',
    'overflow_preserves_counter',
    'proof_forwarder_keeps_caller_and_counter',
))
DAG_PREFIX = 'production_ranked_projection_v1::tests::indexed_atomic_dag_certificate_'
DAG_TESTS = tuple(DAG_PREFIX + name for name in (
    'accepts_chains_and_diamonds',
    'bounds_large_acyclic_query_work',
    'charges_exact_budget_and_never_caches_failed_queries',
    'cycles_keep_per_definition_fallback',
    'matches_reference_on_all_small_graphs',
    'refuses_malformed_graph_without_publishing_cache',
    'rejects_invalid_query_before_cache_hit',
    'reuses_only_within_independent_proof_instances',
    'unreachable_cycles_do_not_grant_global_certificate',
))
CFG_DIAGNOSTIC_PREFIX = 'production_ranked_projection_v1::tests::cfg_block_limit_diagnostic_'
CFG_DIAGNOSTIC_TESTS = tuple(CFG_DIAGNOSTIC_PREFIX + name for name in (
    'zero_preserves_both_refusals',
    'exact_limit_preserves_graph_acceptance',
    'over_limit_preserves_both_refusals',
    'counts_unreachable_declared_blocks',
    'precedes_work_without_changing_budget',
    'callable_summary_has_no_fabricated_root',
    'root_is_bounded_and_escaped',
    'context_leaves_other_errors_unchanged',
))
MEMBERSHIP_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_membership_v1_tests.rs': True,
}
CAPACITY_COHORTS = {
    'cfg-block-cap-compiler': dict(
        role='compiler-lib',
        prefix='production_ranked_projection_v1::tests::cfg_block_cap_',
        names=['production_ranked_projection_v1::tests::cfg_block_cap_preserves_fixed_graph_work_and_edge_caps']),
    'cfg-block-cap-pliron': dict(
        role='pliron-lib',
        prefix='production_analysis::pliron_ranked_bounds::resource_upper_bound_tests::memory_bounds_block_cap_',
        names=sorted('production_analysis::pliron_ranked_bounds::resource_upper_bound_tests::' + name for name in (
            'memory_bounds_block_cap_accepts_legacy_measured_and_2048',
            'memory_bounds_block_cap_does_not_expand_other_limits',
            'memory_bounds_block_cap_preserves_independent_work_storage_refusals'))),
}
CAPACITY_LIMITS = dict(previous_blocks=1024, blocks=2048, facts=1024, edges=2048,
                       projection_graph_work=3145728, operations=65536,
                       ranked_work=8388608, ranked_storage=131072, findings=4096)
INPUT_PINS = {}
BASELINE_FAILURE_VERIFIED = False
HELPERS = None
SOURCE = ROOT / 'fe2o3'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
TOOLCHAIN = Path('/home/harmenon/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/bin')
TOOLCHAIN_LIB = TOOLCHAIN.parent / 'lib'
SHARED_LIBRARIES = {
    'libLLVM.so.22.1-rust-1.96.0-nightly': 199520544,
    'librustc_driver-7bb70639c3ace5a4.so': 152936640,
    'libLLVM-22-rust-1.96.0-nightly.so': 43,
}
WHOLE_WALL, CLEANUP_RESERVE, LEAF_WALL = 7200, 50, 1800
AS_LIMIT, FILE_LIMIT, CPU_LIMIT = 12 << 30, 1 << 30, 1200
CACHE_LIMIT, STREAM_LIMIT = 6 << 30, 64 << 20
START_FREE, LIVE_FREE = 40 << 30, 38 << 30
ROLES = {
    'pliron-lib': ('fe2o3-pliron', 'fe2o3_pliron', 'lib'),
    'compiler-lib': ('rustc-codegen-fe2o3', 'rustc_codegen_fe2o3', 'dylib'),
    'atomic-extraction': ('rustc-codegen-fe2o3', 'production_extraction_driver_v1', 'test'),
    'matrix-extraction': ('rustc-codegen-fe2o3', 'production_general_matrix_driver_v1', 'test'),
}
ATOMIC = (
    'shared_atomic_u32_source_admission_has_a_scalar_positive_control',
    'shared_atomic_u32_source_admission_rejects_nominal_and_width_spoofs',
    'indexed_atomic_slice_two_argument_control_reaches_exact_system_scope_llvm',
    'indexed_atomic_slice_exclusive_input_reaches_exact_system_scope_llvm',
    'indexed_atomic_slice_shared_input_remains_fail_closed_at_alias_analysis',
    'ordinary_atomic_load_store_reaches_gfx950_llvm',
    'atomic_load_store_wrapper_rejects_untrusted_or_nonconstant_contracts',
    'wide_atomic_load_store_preserves_target_capability_rejection',
)
MATRIX = ('dynamic_matrix_kernel_reaches_gfx942_llvm', 'dynamic_attention_kernel_reaches_gfx942_llvm')
CORE_RESULT = 'trusted_device_items::core_result_control_v1::tests::'
CORE_RESULT_TESTS = tuple(CORE_RESULT + name for name in (
    'result_family_table_keeps_exact_source_pairs',
    'result_branch_real_core_bodies_and_nominal_refusals',
    'result_branch_real_body_rejects_count_and_local_mutations',
    'result_branch_real_body_rejects_scope_and_inline_origin_mutations',
    'result_branch_real_body_rejects_extra_operations_and_cleanup',
    'result_branch_real_body_rejects_changed_edges_and_discriminants',
    'result_branch_real_body_rejects_changed_payload_fields',
    'result_branch_real_body_rejects_changed_aggregate_identity_and_moves',
    'result_residual_real_core_bodies_and_nominal_refusals',
    'result_residual_real_body_rejects_count_and_local_mutations',
    'result_residual_real_body_rejects_scope_and_inline_origin_mutations',
    'result_residual_real_body_rejects_extra_operations_and_cleanup',
    'result_residual_real_body_rejects_changed_assume_and_discriminant',
    'result_residual_real_body_rejects_changed_conversion_call',
    'result_residual_real_body_rejects_changed_payload_and_output',
))
CORE_U32_WIDENING = 'trusted_device_items::core_u32_widening_v1::tests::'
CORE_U32_WIDENING_TESTS = tuple(CORE_U32_WIDENING + name for name in (
    'u32_widening_real_core_identity_and_signature_refusals',
    'u32_widening_real_body_rejects_shape_and_local_mutations',
    'u32_widening_real_body_rejects_source_scope_mutations',
    'u32_widening_real_body_rejects_extra_operations_and_cleanup',
    'u32_widening_real_body_rejects_cast_kind_type_and_operand_mutations',
    'u32_widening_real_body_rejects_places_and_terminators',
))
ADDITIONAL_COHORTS = {
    'core-checked-integer': dict(
        prefix='trusted_device_items::core_checked_integer_v1::tests::',
        names=tuple('trusted_device_items::core_checked_integer_v1::tests::' + name for name in (
            'checked_integer_real_core_identity_and_signature_refusals',
            'checked_integer_real_body_rejects_shape_and_local_mutations',
            'checked_integer_real_body_rejects_source_scope_and_inline_mutations',
            'checked_integer_real_body_rejects_extra_operations',
            'checked_integer_real_body_rejects_operands_constants_and_variants',
            'checked_integer_real_body_rejects_control_flow_and_unwind',
        )),
        source_paths=(
            'fe2o3/crates/rustc-codegen-fe2o3/src/trusted_device_items/core_checked_integer_v1.rs',
            'fe2o3/crates/rustc-codegen-fe2o3/src/trusted_device_items/core_checked_integer_v1_tests.rs',
        )),
    'core-attention-option': dict(
        prefix='trusted_device_items::core_attention_option_v1::tests::',
        names=tuple('trusted_device_items::core_attention_option_v1::tests::' + name for name in (
            'attention_option_real_provider_and_closure_positives',
            'attention_option_real_identity_payload_and_callback_refusals',
            'attention_option_real_shape_mutations',
            'attention_option_real_local_type_and_scope_mutations',
            'attention_option_real_source_scope_mutations',
            'attention_option_real_operations_and_cleanup_mutations',
            'attention_option_real_switch_and_return_mutations',
            'attention_option_real_payload_and_aggregate_mutations',
            'attention_option_real_drop_mutations',
            'attention_option_real_callback_call_mutations',
        )),
        source_paths=(
            'fe2o3/crates/rustc-codegen-fe2o3/src/trusted_device_items/core_attention_option_v1.rs',
            'fe2o3/crates/rustc-codegen-fe2o3/src/trusted_device_items/core_attention_option_v1_tests.rs',
        )),
    'core-kernel-error-identity': dict(
        prefix='trusted_device_items::core_kernel_error_identity_v1::tests::',
        names=tuple('trusted_device_items::core_kernel_error_identity_v1::tests::' + name for name in (
            'kernel_error_identity_real_core_provider_identity_and_signature_refusals',
            'kernel_error_identity_real_body_rejects_shape_mutations',
            'kernel_error_identity_real_body_rejects_local_type_and_scope_mutations',
            'kernel_error_identity_real_body_rejects_source_scope_mutations',
            'kernel_error_identity_real_body_rejects_extra_operations_and_cleanup',
            'kernel_error_identity_real_body_rejects_operands_places_and_returns',
        )),
        source_paths=(
            'fe2o3/crates/rustc-codegen-fe2o3/src/trusted_device_items/core_kernel_error_identity_v1.rs',
            'fe2o3/crates/rustc-codegen-fe2o3/src/trusted_device_items/core_kernel_error_identity_v1_tests.rs',
        )),
}
SOURCE_SAFETY_NEGATIVES = (
    'production_collector_rejects_reachable_unsafe_rust_with_rooted_diagnostics',
    'optimized_inlined_source_origins_retain_source_safety_checks',
)
DIAGNOSTIC_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/collector/core_kernel_error_identity_diagnostic_v1.rs',
    'fe2o3/crates/rustc-codegen-fe2o3/src/collector/core_kernel_error_identity_diagnostic_v1_tests.rs',
    'fe2o3/crates/rustc-codegen-fe2o3/src/collector/core_checked_attention_diagnostic_v1.rs',
    'fe2o3/crates/rustc-codegen-fe2o3/src/collector/core_checked_attention_diagnostic_v1_tests.rs',
    'fe2o3/crates/rustc-codegen-fe2o3/src/collector/core_u32_widening_diagnostic_v1.rs',
    'fe2o3/crates/rustc-codegen-fe2o3/src/collector/core_u32_widening_diagnostic_v1_tests.rs',
    'fe2o3/crates/rustc-codegen-fe2o3/src/collector/core_result_diagnostic_v1.rs',
    'fe2o3/crates/rustc-codegen-fe2o3/src/collector/core_result_diagnostic_v1_tests.rs',
    'fe2o3/crates/rustc-codegen-fe2o3/src/collector/core_result_residual_diagnostic_v1.rs',
    'fe2o3/crates/rustc-codegen-fe2o3/src/collector/core_result_residual_diagnostic_v1_tests.rs',
}

def option_fixture_metadata_observation():
    # Match the public derive_cargo_metadata_build_observation_v2 byte protocol.
    values = ('fe2o3_core_attention_option_fixture_v1',)
    digest = hashlib.sha256(b'FE2O3/CARGO-METADATA-BUILD-OBSERVATION/V2\0')
    digest.update(len(values).to_bytes(8, 'little'))
    for value in values:
        raw = value.encode('utf-8')
        digest.update(len(raw).to_bytes(8, 'little'))
        digest.update(raw)
    return digest.hexdigest()


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
            value.st_uid, value.st_gid, value.st_size, value.st_mtime_ns,
            value.st_ctime_ns)


def pin(path):
    path = Path(path)
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= FILE_LIMIT,
            'not a bounded ordinary file: ' + str(path))
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file changed before read')
        for chunk in iter(lambda: stream.read(1 << 20), b''):
            digest.update(chunk)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file changed during read')
    require(stamp(path.lstat()) == stamp(before), 'file changed after read')
    return dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())


def save(name, value):
    path = OUT / name
    with path.open('x') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')
    return pin(path)


def group_exists(pgid):
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False


def reap_group(pgid):
    reaped = []
    while True:
        try:
            pid, status = os.waitpid(-pgid, os.WNOHANG)
        except ChildProcessError:
            break
        if pid == 0:
            break
        reaped.append(dict(pid=pid, wait_status=status))
    return reaped


def stop_group(child, hard_deadline):
    reaped = []
    for sig, allowance in ((signal.SIGTERM, 10), (signal.SIGKILL, 20)):
        child.poll()
        if child.returncode is not None:
            reaped.extend(reap_group(child.pid))
        if not group_exists(child.pid):
            break
        try:
            os.killpg(child.pid, sig)
        except ProcessLookupError:
            break
        deadline = min(hard_deadline, time.monotonic() + allowance)
        while time.monotonic() < deadline:
            child.poll()
            if child.returncode is not None:
                reaped.extend(reap_group(child.pid))
            if not group_exists(child.pid):
                break
            time.sleep(0.02)
    return reaped


def interrupted(signum, _frame):
    raise RuntimeError('interrupted by signal ' + str(signum))


def files_below(root, packed=True):
    require(root.is_dir() and not root.is_symlink(), 'missing ordinary source directory')
    result = []
    for directory, dirs, names in os.walk(root, followlinks=False,
                                          onerror=lambda error: (_ for _ in ()).throw(error)):
        require(not packed or ('.git' not in dirs and 'target' not in dirs),
                'source pack contains Git/cache directory')
        for name in dirs:
            require(not (Path(directory) / name).is_symlink(), 'source directory alias')
        result.extend(Path(directory) / name for name in names)
    return result



def sources():
    files = [ROOT / 'run_cpu.py', ROOT / 'qualification_helpers.py', *files_below(SOURCE)]
    require(len(files) <= 20000, 'source file-count bound')
    return {str(p.relative_to(ROOT)): pin(p) for p in sorted(files)}


def configurations():
    paths = {parent / '.cargo' / name for parent in (SOURCE, ROOT, *ROOT.parents)
             for name in ('config', 'config.toml')}
    paths |= {Path('/home/harmenon/.cargo') / name for name in ('config', 'config.toml')}
    values = {str(p): pin(p) if os.path.lexists(p) else None for p in sorted(paths)}
    require(not any(values.values()), 'inherited Cargo configuration refused')
    return values


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def read_input(row, expected_sha=None):
    require(type(row) is dict and set(row) == {'path', 'bytes', 'sha256'}
            and type(row['bytes']) is int and 0 <= row['bytes'] <= FILE_LIMIT
            and type(row['sha256']) is str and re.fullmatch(r'[0-9a-f]{64}', row['sha256']),
            'closed ordinary input pin')
    path = Path(row['path'])
    require(path.is_absolute() and str(path) == row['path'] and '..' not in path.parts
            and path.resolve(strict=True) == path, 'canonical input path')
    require(pin(path) == row and (expected_sha is None or row['sha256'] == expected_sha),
            'input pin mismatch')
    raw = path.read_bytes()
    require(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256']
            and pin(path) == row, 'input changed during retained read')
    INPUT_PINS[str(path)] = row
    return raw


def json_input(row, expected_sha=None):
    return json.loads(read_input(row, expected_sha))


def load_helpers():
    path = ROOT / 'qualification_helpers.py'
    raw = read_input(pin(path), HELPER_SHA)
    module = types.ModuleType('ranked_cfg_qualification_helpers')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def cohort_contracts():
    return {
        'core-result-control': dict(prefix=CORE_RESULT, names=sorted(CORE_RESULT_TESTS)),
        'core-u32-widening': dict(prefix=CORE_U32_WIDENING, names=sorted(CORE_U32_WIDENING_TESTS)),
        **{label: dict(prefix=row['prefix'], names=sorted(row['names']))
           for label, row in ADDITIONAL_COHORTS.items()},
    }


def lineage_contract(value, before):
    global BASELINE_FAILURE_VERIFIED
    lineage = value['lineage']
    require(type(lineage) is dict and set(lineage) == {
        'base_complete', 'base_sources', 'base_input', 'base_metadata',
        'base_dependencies', 'base_streams', 'cfg_compaction_proposal', 'cfg_compaction_overlay',
        'baseline_failure', 'baseline_failure_stderr'},
        'closed qualified-CFG-expansion-base and CFG-compaction-overlay lineage')
    require(type(CFG_COMPACTION_PROPOSAL_SHA) is str
            and re.fullmatch(r'[0-9a-f]{64}', CFG_COMPACTION_PROPOSAL_SHA),
            'CFG compaction source proposal is not bound')
    prior = json_input(lineage['base_complete'], BASE_COMPLETE_SHA)
    require(prior['schema'] == 'ferric-guarded-mlp-ranked-cfg-expansion-diagnostic-cpu-v1'
            and prior['passed'] is True and prior['failure'] is None
            and prior['postcheck_errors'] == [] and prior['source_unchanged'] is True
            and prior['diagnostic_build'] is True and prior['diagnostic_only'] is True
            and prior['diagnostic_changes_admission'] is False
            and prior['cfg_expansion_diagnostics'] is True
            and prior['cfg_expansion_test_filter'] == CFG_EXPANSION_PREFIX
            and prior['cfg_expansion_tests'] == sorted(CFG_EXPANSION_TESTS)
            and prior['static_failure_site_identified'] is True
            and prior['actual_failure_block_count_observed'] is False
            and prior['admission_changed'] is False and prior['structural_capacity_expansion'] is False
            and prior['inherited_capacity_expansion'] is True
            and prior['structural_limits_changed'] is False
            and prior['graph_analysis_optimization'] is False
            and prior['inherited_graph_analysis_optimization'] is True
            and prior['resource_admission_may_change'] is False
            and prior['membership_lookup_optimization'] is True
            and prior['inherited_membership_lookup_optimization'] is True
            and prior['dead_cast_census_optimization'] is True
            and prior['inherited_dead_cast_census_optimization'] is True
            and prior['use_lookup_optimization'] is True
            and prior['use_lookup_test_filter'] == USE_LOOKUP_PREFIX
            and prior['use_lookup_tests'] == sorted(USE_LOOKUP_TESTS)
            and prior['inherited_use_lookup_optimization'] is True
            and prior['borrow_lookup_optimization'] is True
            and prior['inherited_borrow_lookup_optimization'] is True
            and prior['borrow_lookup_test_filter'] == BORROW_LOOKUP_PREFIX
            and prior['borrow_lookup_tests'] == sorted(BORROW_LOOKUP_TESTS)
            and prior['compiler_scratch_added'] is False
            and prior['census_test_filter'] == CENSUS_PREFIX
            and prior['census_tests'] == sorted(CENSUS_TESTS)
            and prior['semantic_predicates_changed'] is False
            and prior['actual_failure_caller_identified'] is False
            and prior['baseline_failure_caller_identified'] is False
            and prior['dag_test_filter'] == DAG_PREFIX and prior['dag_tests'] == sorted(DAG_TESTS)
            and prior['graph_work_diagnostics'] is True
            and prior['inherited_graph_work_diagnostics'] is True
            and prior['graph_work_test_filter'] == GRAPH_WORK_PREFIX
            and prior['graph_work_tests'] == sorted(GRAPH_WORK_TESTS)
            and prior['membership_test_filter'] == MEMBERSHIP_PREFIX
            and prior['membership_tests'] == sorted(MEMBERSHIP_TESTS)
            and prior['source_lineage']['cfg_expansion_proposal']['sha256'] == CFG_EXPANSION_PROPOSAL_SHA
            and prior['cfg_diagnostics_retained'] is True
            and prior['cfg_diagnostic_tests'] == sorted(CFG_DIAGNOSTIC_TESTS)
            and prior['capacity_cohorts'] == CAPACITY_COHORTS
            and prior['capacity_limits'] == CAPACITY_LIMITS
            and prior['compiler_cohorts'] == cohort_contracts()
            and prior['controller']['sha256'] == BASE_CONTROLLER_SHA
            and prior['tests_passed'] == 2936 and prior['tests_ignored'] == 25
            and len(prior['tests']) == 29, 'actual qualified CFG-expansion diagnostic baseline')
    phases = prior['phases']
    require(len(phases) == len({row['label'] for row in phases}) == 42
            and all(row['exit_code'] == 0 and row['natural_exit'] is True
                    and row['reaped'] is True and row['process_group_absent'] is True
                    and row['forced_cleanup'] is False and row['timed_out'] is False
                    and row['exception'] is None and row['storage_failure'] is None
                    for row in phases), 'baseline natural/reaped 42-phase CFG-expansion diagnostic')
    for key in ('input_sources', 'final_sources'):
        require(compact(prior[key]) == compact(lineage['base_sources']),
                'actual baseline before/after source join')
    prior_sources = json_input(lineage['base_sources'], BASE_SOURCES_SHA)
    require(type(prior_sources) is dict and len(prior_sources) == 5805,
            'actual baseline source/controller/helper census')
    for name, row in prior_sources.items():
        path = Path(name)
        require(type(row) is dict and set(row) == {'path', 'bytes', 'sha256'}
                and not path.is_absolute() and '..' not in path.parts
                and str(path) == name and row['path'] == str(BASE_ROOT / path),
                'baseline canonical relative source row')
    prior_input = json_input(lineage['base_input'], BASE_INPUT_SHA)
    require(compact(prior['input_manifest']) == compact(lineage['base_input'])
            and prior_input['schema'] == 'ferric-guarded-mlp-ranked-cfg-expansion-diagnostic-cpu-input-v1'
            and prior_input['files'] == {name: compact(row) for name, row in prior_sources.items()}
            and value['tool_pins'] == prior_input['tool_pins'] == prior['tool_pins']
            and compact(value['rust_src']) == compact(prior_input['rust_src'])
                == compact(prior['rust_src']),
            'baseline exact input/source/tool/rust-src joins')
    expected = {name: compact(row) for name, row in prior_sources.items()
                if name.startswith('fe2o3/')}
    require(len(expected) == 5803, 'qualified CFG-expansion diagnostic source census')
    require(lineage['baseline_failure'] == dict(path=str(BASE_FAILURE_ROOT / 'failed.json'),
                bytes=15422, sha256=BASE_FAILURE_SHA)
            and lineage['baseline_failure_stderr'] == dict(
                path=str(BASE_FAILURE_ROOT / 'compile.stderr'), bytes=8424,
                sha256=BASE_FAILURE_STDERR_SHA), 'exact measured baseline failure inputs')
    baseline_failure = json_input(lineage['baseline_failure'], BASE_FAILURE_SHA)
    baseline_stderr = read_input(lineage['baseline_failure_stderr'], BASE_FAILURE_STDERR_SHA)
    require(baseline_failure['schema'] == 'ferric-guarded-mlp-ranked-cfg-expansion-diagnostic-lowering-result-v1'
            and baseline_failure['passed'] is False
            and baseline_failure['failure'] == "RuntimeError('natural successful/reaped leaf required')"
            and baseline_failure['postcheck_errors'] == []
            and baseline_failure['source_unchanged'] is True
            and baseline_failure['input_byte_maps_rechecked'] is True
            and baseline_failure['automatic_retries'] == 0
            and baseline_failure['artifact'] is None
            and baseline_failure['retained_handoff_or_llvm'] is False
            and baseline_failure['capacity_limits'] == CAPACITY_LIMITS
            and baseline_failure['raw']['compile.stderr'] == lineage['baseline_failure_stderr']
            and compact(baseline_failure['raw']['source-before.json'])
                == compact(baseline_failure['raw']['source-after.json'])
            and len(baseline_failure['phases']) == 1,
            'actual clean failed guarded baseline and immutable source joins')
    baseline_phase = baseline_failure['phases'][0]
    require(baseline_phase['label'] == 'compile' and baseline_phase['exit_code'] == 1
            and baseline_phase['natural_exit'] is True and baseline_phase['reaped'] is True
            and baseline_phase['process_group_absent'] is True
            and baseline_phase['forced_cleanup'] is False and baseline_phase['timed_out'] is False
            and baseline_phase['exception'] is None and baseline_phase['observed_signals'] == []
            and baseline_phase['stderr'] == lineage['baseline_failure_stderr']
            and baseline_phase['stdout']['bytes'] == 0
            and baseline_phase['stdout'] == baseline_failure['raw']['compile.stdout'],
            'baseline compiler exited naturally with code one and retained exact stderr')
    require(all(baseline_failure[key] is False for key in (
                'production_authority', 'load_authority', 'launch_authority', 'gpu_execution',
                'numerical_acceptance', 'full_model_acceptance', 'performance_claim'))
            and [line for line in baseline_stderr.splitlines()
                 if b'cfg-block-diagnostic-v1' in line] == [BASE_FAILURE_DIAGNOSTIC],
            'one exact observed guard-root projected-CFG diagnostic without additional authority')
    BASELINE_FAILURE_VERIFIED = True
    proposal = json_input(lineage['cfg_compaction_proposal'], CFG_COMPACTION_PROPOSAL_SHA)
    overlay = lineage['cfg_compaction_overlay']
    require(proposal['schema'] == 'ferric-guarded-mlp-ranked-cfg-compaction-source-v1'
            and proposal['source_only'] is True
            and proposal['cfg_compaction_optimization'] is True
            and proposal['admission_changed'] is True
            and proposal['compiler_scratch_added'] is False
            and proposal['graph_analysis_optimization'] is False
            and proposal['semantic_predicates_changed'] is False
            and proposal['structural_limits_changed'] is False
            and proposal['resource_admission_may_change'] is True
            and proposal['static_failure_site_identified'] is True
            and proposal['actual_failure_caller_identified'] is False
            and proposal['actual_failure_block_count_observed'] is False
            and proposal['baseline_failure_caller_identified'] is True
            and proposal['baseline_failure_block_count_observed'] is True
            and proposal['baseline_projected_blocks'] == 2778
            and proposal['baseline_semantic_blocks'] == 1121
            and proposal['baseline_block_limit'] == CAPACITY_LIMITS['blocks']
            and proposal['graph_work_limit'] == CAPACITY_LIMITS['projection_graph_work']
            and proposal['block_limit'] == CAPACITY_LIMITS['blocks']
            and proposal['edge_limit'] == CAPACITY_LIMITS['edges']
            and proposal['base_source_count'] == 5803
            and proposal['source_count'] == 5803 + sum(CFG_COMPACTION_FILES.values())
            and proposal['source_file_additions'] == sum(CFG_COMPACTION_FILES.values())
            and proposal['new_test_count'] == len(CFG_COMPACTION_TESTS)
            and proposal['actual_failure'] == compact(lineage['baseline_failure'])
            and proposal['actual_failure_stderr'] == compact(lineage['baseline_failure_stderr'])
            and proposal['base_complete'] == compact(lineage['base_complete'])
            and proposal['base_sources'] == compact(lineage['base_sources'])
            and proposal['base_input'] == compact(lineage['base_input'])
            and proposal['base_controller'] == compact(prior['controller'])
            and type(overlay) is dict and set(overlay) == set(CFG_COMPACTION_FILES)
            and {'fe2o3/' + name: row for name, row in proposal['files'].items()} == overlay
            and proposal['filter'] == CFG_COMPACTION_PREFIX
            and sorted(proposal['test_names']) == sorted(CFG_COMPACTION_TESTS)
            and len(proposal['test_names']) == len(CFG_COMPACTION_TESTS),
            'closed CFG compaction proposal and exact authored test cohort')
    for name, is_new in CFG_COMPACTION_FILES.items():
        row = overlay[name]
        require(type(row) is dict and set(row) == {'before', 'after'}
                and type(row['after']) is dict and set(row['after']) == {'bytes', 'sha256'}
                and type(row['after']['bytes']) is int and 0 < row['after']['bytes'] <= FILE_LIMIT
                and type(row['after']['sha256']) is str
                and re.fullmatch(r'[0-9a-f]{64}', row['after']['sha256']),
                'closed bounded CFG compaction source row')
        if is_new:
            require(name not in expected and row['before'] is None,
                    'CFG compaction addition must be absent from qualified baseline')
        else:
            require(row['before'] == expected[name] and row['after'] != row['before'],
                    'CFG compaction change must match qualified preimage')
        expected[name] = row['after']
    actual = {name: compact(row) for name, row in before.items() if name.startswith('fe2o3/')}
    require(len(actual) == 5803 + sum(CFG_COMPACTION_FILES.values()) and actual == expected,
            'qualified CFG-expansion diagnostic map differs outside exact CFG compaction overlay')
    require(compact(before['qualification_helpers.py']) ==
            compact(prior_sources['qualification_helpers.py']), 'unchanged qualified parser/rlib helper')
    required_streams = {short + '-' + suffix + '-stdout'
                        for short in ('compiler', 'pliron')
                        for suffix in ('list', 'ignored-list', 'tests')}
    require(type(lineage['base_streams']) is dict
            and set(lineage['base_streams']) == required_streams, 'six historical full-suite streams')
    historical = {}
    for short, expected_counts in (('compiler', (1305, 24)), ('pliron', (1507, 1))):
        role = short + '-lib'
        for suffix in ('list', 'ignored-list', 'tests'):
            key = short + '-' + suffix + '-stdout'
            raw_name = (short + '-tests.stdout' if suffix == 'tests' else
                        role + ('-list.stdout' if suffix == 'list' else '-ignored.stdout'))
            row = lineage['base_streams'][key]
            require(compact(row) == compact(prior['raw'][raw_name]), 'baseline raw stream join')
            read_input(row)
        names = inventory(Path(lineage['base_streams'][short + '-list-stdout']['path']))
        ignored = inventory(Path(lineage['base_streams'][short + '-ignored-list-stdout']['path']))
        result = HELPERS.full_suite_outcomes(
            Path(lineage['base_streams'][short + '-tests-stdout']['path']), names, ignored)
        require((result['passed'], result['ignored']) == expected_counts
                and result == prior['tests'][short]
                and names == prior['test_inventories'][role]
                and ignored == prior['ignored_inventories'][role],
                'exact historical full-suite names, ignored identities and statuses')
        historical[short] = result
    require(len(CFG_COMPACTION_TESTS) == len(set(CFG_COMPACTION_TESTS))
            and not set(CFG_COMPACTION_TESTS) & set(historical['compiler']['names']),
            'CFG compaction names must be new nonoverlapping compiler additions')
    inherited_analysis = dict(CAPACITY_COHORTS)
    inherited_analysis['cfg-block-limit-diagnostic'] = dict(
        role='compiler-lib', prefix=CFG_DIAGNOSTIC_PREFIX, names=sorted(CFG_DIAGNOSTIC_TESTS))
    inherited_analysis['indexed-atomic-dag-certificate'] = dict(
        role='compiler-lib', prefix=DAG_PREFIX, names=sorted(DAG_TESTS))
    inherited_analysis['graph-work-diagnostic'] = dict(
        role='compiler-lib', prefix=GRAPH_WORK_PREFIX, names=sorted(GRAPH_WORK_TESTS))
    inherited_analysis['indexed-atomic-membership'] = dict(
        role='compiler-lib', prefix=MEMBERSHIP_PREFIX, names=sorted(MEMBERSHIP_TESTS))
    inherited_analysis['indexed-atomic-dead-cast-census'] = dict(
        role='compiler-lib', prefix=CENSUS_PREFIX, names=sorted(CENSUS_TESTS))
    inherited_analysis['indexed-atomic-use-lookup'] = dict(
        role='compiler-lib', prefix=USE_LOOKUP_PREFIX, names=sorted(USE_LOOKUP_TESTS))
    inherited_analysis['indexed-atomic-borrow-lookup'] = dict(
        role='compiler-lib', prefix=BORROW_LOOKUP_PREFIX, names=sorted(BORROW_LOOKUP_TESTS))
    inherited_analysis['cfg-expansion-diagnostic'] = dict(
        role='compiler-lib', prefix=CFG_EXPANSION_PREFIX, names=sorted(CFG_EXPANSION_TESTS))
    for label, cohort in inherited_analysis.items():
        result = prior['tests'][label]
        role = cohort['role']
        require(result['names'] == cohort['names']
                and result['passed'] == len(cohort['names'])
                and result['failed'] == result['ignored'] == 0
                and result['filtered_out'] == len(prior['test_inventories'][role]) - len(cohort['names'])
                and [name for name in prior['test_inventories'][role]
                     if name.startswith(cohort['prefix'])] == cohort['names']
                and not set(cohort['names']) & set(prior['ignored_inventories'][role]),
                'all inherited named diagnostic and capacity cohorts retained')
    for label, cohort in cohort_contracts().items():
        result = prior['tests'][label]
        require(result['names'] == cohort['names']
                and result['passed'] == len(cohort['names'])
                and result['failed'] == result['ignored'] == 0,
                'all inherited named wrapper cohorts retained')
    for label_prefix, names in (('atomic-extraction', ATOMIC),
                               ('source-safety-negative', SOURCE_SAFETY_NEGATIVES),
                               ('matrix-extraction', MATRIX)):
        for index, name in enumerate(names):
            result = prior['tests'][label_prefix + '-' + str(index)]
            require(result['names'] == [name] and result['passed'] == 1
                    and result['failed'] == result['ignored'] == 0,
                    'all twelve historical extraction controls retained')
    require(compact(lineage['base_metadata']) == compact(prior['raw']['metadata.stdout']),
            'baseline metadata stream join')
    metadata = json_input(lineage['base_metadata'])
    for name in ('dependencies-before.json', 'dependencies-after.json'):
        require(compact(lineage['base_dependencies']) == compact(prior['raw'][name]),
                'baseline unchanged dependency map join')
    dependencies = json_input(lineage['base_dependencies'])
    return dict(historical=historical, metadata=metadata, dependencies=dependencies,
                inventories=prior['test_inventories'], ignored=prior['ignored_inventories'])


def input_contract(digest):
    global HELPERS
    actual = pin(ROOT / 'input-manifest.json')
    require(actual['sha256'] == digest, 'literal input manifest mismatch')
    value = json_input(actual)
    require(set(value) == {'schema', 'files', 'tool_pins', 'lineage',
                           'metadata_relocations', 'rust_src'}
            and value['schema'] == 'ferric-guarded-mlp-ranked-cfg-compaction-cpu-input-v1',
            'closed CFG compaction CPU input fields')
    HELPERS = load_helpers()
    before = sources()
    require(value['files'] == {name: compact(row) for name, row in before.items()},
            'root-pinned source/controller/helper roster mismatch')
    require(not set(before) & DIAGNOSTIC_FILES, 'diagnostic sources are not production inputs')
    collector = (SOURCE / 'crates/rustc-codegen-fe2o3/src/collector.rs').read_text()
    require(all(token not in collector for token in (
        'core_kernel_error_identity_diagnostic_v1', 'FE2O3_DIAG_CORE_KERNEL_ERROR_IDENTITY_MIR_V1',
        'core_checked_attention_diagnostic_v1', 'FE2O3_DIAG_CORE_CHECKED_ATTENTION_MIR_V1',
        'core_result_diagnostic_v1', 'FE2O3_DIAG_CORE_RESULT_MIR_V1',
        'core_u32_widening_diagnostic_v1', 'FE2O3_DIAG_CORE_U32_WIDENING_MIR_V1')),
        'diagnostic hooks are not production inputs')
    lineage = lineage_contract(value, before)
    return value, actual, before, lineage


def relocated_metadata(value, relocations):
    require(type(relocations) is dict and len(relocations) == 2
            and relocations.get(str(BASE_SOURCE)) == str(SOURCE)
            and relocations.get(str(BASE_ROOT / 'target')) == str(TARGET),
            'explicit old source/target metadata relocation')
    for old, new in relocations.items():
        require(type(old) is str and type(new) is str
                and str(Path(old)) == old and str(Path(new)) == new
                and Path(old).is_absolute() and Path(new).is_absolute()
                and '..' not in Path(old).parts and '..' not in Path(new).parts
                and old in (str(BASE_SOURCE), str(BASE_ROOT / 'target')),
                'only qualified source/target metadata path relocations')
    require(len(set(relocations.values())) == len(relocations)
            and all(not Path(right).is_relative_to(Path(left))
                    for left in relocations for right in relocations if left != right),
            'metadata relocation roots must be distinct and nonoverlapping')
    def convert(item):
        if isinstance(item, str):
            for prefix in ('', 'path+file://'):
                for old in sorted(relocations, key=len, reverse=True):
                    start = prefix + old
                    if item == start or item.startswith(start + '/') or item.startswith(start + '#'):
                        return prefix + relocations[old] + item[len(start):]
            return item
        if isinstance(item, list):
            return [convert(member) for member in item]
        if isinstance(item, dict):
            return {key: convert(member) for key, member in item.items()}
        return item
    return convert(value)


def rust_sources(expected):
    root = TOOLCHAIN.parent / 'lib/rustlib/src/rust/library'
    actual = {str(path.relative_to(root)): compact(pin(path))
              for path in files_below(root, packed=False)}
    require(actual == expected and actual, 'complete pinned rust-src roster/body mismatch')
    return actual


def metadata_contract():
    value = json.loads((OUT / 'metadata.stdout').read_bytes())
    require(value['workspace_root'] == str(SOURCE) and value['target_directory'] == str(TARGET),
            'metadata workspace/target mismatch')
    packages = {row['id']: row for row in value['packages']}
    require(len(packages) == len(value['packages']), 'duplicate metadata package')
    require(all(Path(packages[k]['manifest_path']).is_relative_to(SOURCE)
                for k in value['workspace_members']), 'workspace member outside pinned source')
    local, external = {}, {}
    for row in packages.values():
        path = Path(row['manifest_path'])
        require(path.resolve(strict=True) == path, 'dependency path alias')
        if row['source'] is None:
            require(path.is_relative_to(SOURCE), 'unlisted local dependency')
            local[row['name']] = str(path)
        else:
            require(path.is_relative_to(Path('/home/harmenon/.cargo'))
                    and row['source'].startswith(('registry+', 'git+')), 'external dependency source')
            external[str(path.parent)] = {str(p.relative_to(path.parent)): pin(p)
                                         for p in files_below(path.parent, packed=False)}
    for package, name, kind in ROLES.values():
        rows = [p for p in packages.values() if p['manifest_path'] == str(SOURCE / 'crates' / package / 'Cargo.toml')]
        require(len(rows) == 1 and any(t['name'] == name and kind in t['kind']
                                      for t in rows[0]['targets']), 'selected Cargo target absent')
    return external, local


def build_records(path):
    rows = [json.loads(line) for line in path.read_text().splitlines() if line.startswith('{')]
    require([row.get('success') for row in rows if row.get('reason') == 'build-finished'] == [True],
            'one successful Cargo build-finished record required')
    return rows


def select_artifact(records, package, target, kind, testing, suffix=None):
    rows = [row for row in records if row.get('reason') == 'compiler-artifact'
            and row.get('manifest_path') == str(SOURCE / 'crates' / package / 'Cargo.toml')
            and row.get('target', {}).get('name') == target and kind in row['target'].get('kind', [])
            and row.get('profile', {}).get('test') is testing]
    require(len(rows) == 1, 'unique selected artifact required: ' + target)
    row = rows[0]
    if suffix:
        paths = [Path(p) for p in row['filenames'] if p.endswith(suffix)]
    else:
        paths = [Path(row['executable'])] if row.get('executable') else []
    require(len(paths) == 1, 'selected artifact extent/role')
    selected = paths[0]
    require(selected.is_relative_to(TARGET) and selected.resolve(strict=True) == selected,
            'selected artifact outside fresh target')
    with selected.open('rb') as stream:
        require(stream.read(4) == b'\x7fELF', 'selected product is not ELF')
    return dict(pin=pin(selected), cargo_artifact=row)


def inventory(path):
    text = path.read_text()
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', text, re.M)
    require(len(names) == len(set(names)) and ': benchmark' not in text, 'ambiguous libtest inventory')
    return sorted(names)


def outcomes(path, selected, total):
    text = path.read_text()
    named = re.findall(r'^test ([A-Za-z0-9_:]+)(?: - should panic)? \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', text, re.M)
    summaries = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
                           r'(\d+) ignored; (\d+) measured; (\d+) filtered out;', text, re.M)
    require(selected and len(named) == len(selected)
            and sorted(name for name, _ in named) == sorted(selected)
            and all(status == 'ok' for _, status in named)
            and summaries == [('ok', str(len(selected)), '0', '0', '0', str(total - len(selected)))],
            'selected named tests must all pass exactly once, with no hidden ignored cases')
    return dict(names=sorted(selected), passed=len(selected), failed=0, ignored=0,
                filtered_out=total-len(selected))


def scratch_bytes():
    total = 0
    count = 0
    for root in (TARGET, TMP):
        if not root.exists():
            continue
        def walk_error(error):
            path = Path(error.filename) if isinstance(error.filename, str) else None
            if (isinstance(error, FileNotFoundError) and path is not None
                    and path.is_absolute() and '..' not in path.parts
                    and path != root and path.is_relative_to(root)):
                # Cargo may remove an owned cache descendant between directory scans.
                return
            raise error
        for directory, _, names in os.walk(root, followlinks=False, onerror=walk_error):
            for name in names:
                try:
                    total += (Path(directory) / name).lstat().st_size
                except FileNotFoundError:
                    # Cargo removes/renames temporary files during this bounded inventory.
                    continue
                count += 1
                require(count <= 200000, 'scratch inventory bound')
    return total


def run(label, argv, env, phases, hard_deadline, tested, seconds=LEAF_WALL, cwd=ROOT):
    work_deadline = hard_deadline - CLEANUP_RESERVE
    require(shutil.disk_usage(ROOT).free >= LIVE_FREE and scratch_bytes() <= CACHE_LIMIT,
            'pre-leaf storage bound')
    remaining = work_deadline - time.monotonic()
    require(remaining > 0, 'whole-run work deadline reached')
    wall = min(seconds, remaining)
    command = ['/usr/bin/prlimit', '--as=' + str(AS_LIMIT), '--cpu=' + str(CPU_LIMIT),
               '--fsize=' + str(FILE_LIMIT), '--core=0', '--', *argv]
    command_pin = save(label + '.command.json', dict(argv=command, cwd=str(cwd), env=env,
                                                    wall_timeout_seconds=wall))
    leaf_started = time.monotonic_ns()
    stdout, stderr = OUT / (label + '.stdout'), OUT / (label + '.stderr')
    timed_out = forced_cleanup = False
    exception = storage_failure = None
    code = None
    reaped = []
    deferred = []
    def defer(signum, _frame):
        deferred.append(signum)
    with stdout.open('xb') as so, stderr.open('xb') as se:
        handled = (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM)
        previous_handlers = {sig: signal.getsignal(sig) for sig in handled}
        for sig in handled:
            signal.signal(sig, defer)
        child = None
        try:
            require(not deferred, 'termination requested before spawn')
            child = subprocess.Popen(command, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                     stdout=so, stderr=se, start_new_session=True)
            pgid = child.pid
            require(os.getpgid(child.pid) == pgid, 'leaf process group identity differs')
            save(label + '.started.json', dict(pid=child.pid, pgid=pgid, argv=command))
            leaf_deadline = time.monotonic() + wall
            next_storage = 0.0
            while child.poll() is None:
                now = time.monotonic()
                if deferred:
                    exception = 'deferred signals: ' + repr(deferred)
                    break
                if now >= leaf_deadline:
                    timed_out = True
                    break
                if max(os.fstat(so.fileno()).st_size, os.fstat(se.fileno()).st_size) > STREAM_LIMIT:
                    storage_failure = 'stream cap'
                    break
                if now >= next_storage:
                    if shutil.disk_usage(ROOT).free < LIVE_FREE or scratch_bytes() > CACHE_LIMIT:
                        storage_failure = 'storage floor/cache cap'
                        break
                    next_storage = now + 2
                time.sleep(0.1)
            code = child.poll()
        except BaseException as error:
            exception = repr(error)
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            try:
                if child is not None:
                    child.poll()
                    if child.returncode is not None:
                        reaped.extend(reap_group(pgid))
                    if child.poll() is None or group_exists(pgid):
                        forced_cleanup = True
                        reaped.extend(stop_group(child, hard_deadline))
                    try:
                        code = child.wait(timeout=max(0.001, min(5, hard_deadline - time.monotonic())))
                    except subprocess.TimeoutExpired:
                        exception = (exception or '') + ' leader not reaped within cleanup bound'
                    if child.returncode is not None:
                        reaped.extend(reap_group(pgid))
                    group_absent = not group_exists(pgid)
            finally:
                for sig, previous in previous_handlers.items():
                    signal.signal(sig, previous)
    require(child is not None, 'spawn failed before owned child registration: ' + str(exception))
    if deferred:
        exception = (exception or '') + ' deferred signals: ' + repr(deferred)
    if time.monotonic() >= work_deadline:
        timed_out = True
    row = dict(label=label, command=command_pin, argv=command, pid=child.pid, pgid=pgid,
               exit_code=code, timed_out=timed_out, elapsed_ns=time.monotonic_ns() - leaf_started,
               natural_exit=code is not None and not timed_out and not forced_cleanup
                   and exception is None and storage_failure is None,
               reaped=child.returncode is not None, forced_cleanup=forced_cleanup,
               process_group_absent=group_absent, adopted_reaped=reaped, exception=exception,
               observed_signals=deferred,
               storage_failure=storage_failure, stdout=pin(stdout), stderr=pin(stderr))
    phases.append(row)
    save(label + '.result.json', row)
    require(code == 0 and row['natural_exit'] and row['reaped'] and group_absent,
            label + ' did not finish naturally/reaped/successfully')
    require(max(row['stdout']['bytes'], row['stderr']['bytes']) <= STREAM_LIMIT
            and shutil.disk_usage(ROOT).free >= LIVE_FREE and scratch_bytes() <= CACHE_LIMIT,
            'post-leaf storage bound')
    if tested is not None:
        require(sources() == tested, 'qualified source/lock drift during phase')
    signal.setitimer(signal.ITIMER_REAL, max(0.001, work_deadline - time.monotonic()))


def main():
    require(type(CFG_COMPACTION_PROPOSAL_SHA) is str
            and re.fullmatch(r'[0-9a-f]{64}', CFG_COMPACTION_PROPOSAL_SHA)
            and type(CFG_COMPACTION_PREFIX) is str and CFG_COMPACTION_PREFIX
            and type(CFG_COMPACTION_TESTS) is tuple and CFG_COMPACTION_TESTS
            and len(CFG_COMPACTION_TESTS) == len(set(CFG_COMPACTION_TESTS))
            and all(type(name) is str and name.startswith(CFG_COMPACTION_PREFIX) for name in CFG_COMPACTION_TESTS)
            and type(CFG_COMPACTION_FILES) is dict and CFG_COMPACTION_FILES
            and all(type(name) is str and name.startswith('fe2o3/')
                    and '..' not in Path(name).parts and type(is_new) is bool
                    for name, is_new in CFG_COMPACTION_FILES.items()),
            'CFG compaction source proposal, exact paths and authored test roster are not bound')
    require(type(CORE_U32_WIDENING_TESTS) is tuple and len(CORE_U32_WIDENING_TESTS) == 6,
            'unbound exact widening test roster')
    started = time.monotonic()
    hard_deadline = started + WHOLE_WALL
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]), 'python3 -B run_cpu.py ACTUAL_INPUT_SHA')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve() == ROOT
            and not any(os.path.lexists(p) for p in (OUT, TARGET, TMP)), 'fresh exact namespace required')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'unexpected host/UID')
    require(shutil.disk_usage(ROOT).free >= START_FREE, 'initial40GiB floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected nice')
    if priority == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, AS_LIMIT), (resource.RLIMIT_FSIZE, FILE_LIMIT)):
        soft, hard = resource.getrlimit(kind)
        effective = min([cap] + [n for n in (soft, hard) if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (effective, effective))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup failed')
    inputs, input_pin, before, lineage = input_contract(sys.argv[1])
    rust_src_expected = json_input(inputs['rust_src'])
    rust_sources(rust_src_expected)
    config_before = configurations()
    tool_pins = {name: pin(TOOLCHAIN / name) for name in ('rustc', 'rustdoc', 'rustfmt', 'cargo')}
    tool_pins['prlimit'] = pin(Path('/usr/bin/prlimit'))
    require(TOOLCHAIN_LIB.resolve(strict=True) == TOOLCHAIN_LIB, 'toolchain library alias')
    for name, count in SHARED_LIBRARIES.items():
        path = TOOLCHAIN_LIB / name
        require(path.resolve(strict=True) == path, 'compiler shared-library alias')
        tool_pins[name] = pin(path)
        require(tool_pins[name]['bytes'] == count, 'compiler shared-library extent')
    require(tool_pins == inputs['tool_pins'], 'root-pinned tools differ')
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    env = dict(HOME='/home/harmenon', PATH=str(TOOLCHAIN) + ':/usr/bin:/bin',
               CARGO_HOME='/home/harmenon/.cargo', CARGO_TARGET_DIR=str(TARGET),
               LD_LIBRARY_PATH=str(TARGET / 'debug/deps') + ':' + str(TOOLCHAIN_LIB), TMPDIR=str(TMP),
               RUSTC=str(TOOLCHAIN / 'rustc'), RUSTDOC=str(TOOLCHAIN / 'rustdoc'),
               CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', RUST_BACKTRACE='1',
               CARGO_NET_OFFLINE='true', CARGO_CACHE_AUTO_CLEAN_FREQUENCY='never',
               MALLOC_ARENA_MAX='2',
               CARGO_PROFILE_DEV_OPT_LEVEL='0', CARGO_PROFILE_DEV_DEBUG='0',
               CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
               CARGO_PROFILE_TEST_OPT_LEVEL='0', CARGO_PROFILE_TEST_DEBUG='0',
               CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
               ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
    # No ambient or synthetic crate binding; the extraction fixtures establish their own contracts.
    source_pin = save('sources-before.json', before)
    phases, artifacts, external, local, inventories, ignored, tests = [], {}, {}, {}, {}, {}, {}
    compiler_products_before_test_build = {}
    failure, postchecks, final = None, [], None
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, hard_deadline-CLEANUP_RESERVE-time.monotonic()))
    def leaf(label, argv, seconds=LEAF_WALL, package=None):
        cwd = SOURCE / 'crates' / package if package else ROOT
        selected_env = dict(env)
        if label in ('compiler-tests', 'core-attention-option', 'core-kernel-error-identity'):
            selected_env['FE2O3_CARGO_METADATA_BUILD_OBSERVATION_V2'] = option_fixture_metadata_observation()
        if package:
            selected_env['CARGO_MANIFEST_DIR'] = str(cwd)
        run(label, argv, selected_env, phases, hard_deadline, before, seconds, cwd)
    cargo = str(TOOLCHAIN / 'cargo')
    common = ['--offline', '--locked', '--manifest-path', str(SOURCE / 'Cargo.toml')]
    try:
        leaf('rustc-version', [str(TOOLCHAIN / 'rustc'), '--version', '--verbose'], 60)
        leaf('metadata', [cargo, 'metadata', *common, '--format-version', '1'], 120)
        require(json.loads((OUT / 'metadata.stdout').read_bytes()) ==
                relocated_metadata(lineage['metadata'], inputs['metadata_relocations']),
                'full Cargo graph/metadata differs beyond declared path relocation')
        external, local = metadata_contract()
        save('dependencies-before.json', external)
        require(external == lineage['dependencies'],
                'resolved dependency source bodies differ from qualified full-S baseline')
        leaf('pliron-build-tests', [cargo, 'test', *common, '--jobs', '2', '-p', 'fe2o3-pliron',
                                   '--lib', '--no-run', '--message-format=json'])
        records = build_records(OUT / 'pliron-build-tests.stdout')
        artifacts['pliron-lib'] = select_artifact(records, *ROLES['pliron-lib'], True)
        leaf('compiler-products', [cargo, 'build', *common, '--jobs', '2', '-p', 'rustc-codegen-fe2o3',
                                   '--lib', '--bin', 'fe2o3-rustc-extract', '--message-format=json'])
        records = build_records(OUT / 'compiler-products.stdout')
        compiler_products_before_test_build['backend'] = select_artifact(records, 'rustc-codegen-fe2o3', 'rustc_codegen_fe2o3',
                                                'dylib', False, '.so')
        compiler_products_before_test_build['extractor'] = select_artifact(records, 'rustc-codegen-fe2o3', 'fe2o3-rustc-extract',
                                                  'bin', False)
        leaf('compiler-tests-build', [cargo, 'test', *common, '--jobs', '2', '-p', 'rustc-codegen-fe2o3',
                                      '--lib', '--test', 'production_extraction_driver_v1',
                                      '--test', 'production_general_matrix_driver_v1',
                                      '--no-run', '--message-format=json'])
        records = build_records(OUT / 'compiler-tests-build.stdout')
        # Cargo test feature unification can rebuild these same production paths.
        artifacts['backend'] = select_artifact(records, 'rustc-codegen-fe2o3', 'rustc_codegen_fe2o3',
                                                'dylib', False, '.so')
        artifacts['extractor'] = select_artifact(records, 'rustc-codegen-fe2o3', 'fe2o3-rustc-extract',
                                                  'bin', False)
        artifacts['backend-rlib'] = HELPERS.select_backend_rlib(records, SOURCE, TARGET, pin)
        for role in ('backend', 'extractor'):
            require(artifacts[role]['pin']['path'] == compiler_products_before_test_build[role]['pin']['path'],
                    'final Cargo product path differs from its prior build observation')
        for role in ('compiler-lib', 'atomic-extraction', 'matrix-extraction'):
            artifacts[role] = select_artifact(records, *ROLES[role], True)
        for role in ROLES:
            executable = artifacts[role]['pin']['path']
            leaf(role + '-list', [executable, '--list', '--format=terse'], 120, ROLES[role][0])
            leaf(role + '-ignored', [executable, '--ignored', '--list', '--format=terse'], 120, ROLES[role][0])
            inventories[role] = inventory(OUT / (role + '-list.stdout'))
            ignored[role] = inventory(OUT / (role + '-ignored.stdout'))
            require(inventories[role] and set(ignored[role]) <= set(inventories[role]), 'compiled inventory closure')
        for role, short in (('pliron-lib', 'pliron'), ('compiler-lib', 'compiler')):
            old = lineage['historical'][short]
            added = list(CFG_COMPACTION_TESTS) if short == 'compiler' else []
            require(inventories[role] == sorted(old['names'] + added)
                    and ignored[role] == old['ignored_names'], 'exact inherited full name/ignore census')
            leaf(short + '-tests', [artifacts[role]['pin']['path'], '--test-threads=2'],
                 package=ROLES[role][0])
            tests[short] = HELPERS.full_suite_outcomes(
                OUT / (short + '-tests.stdout'), inventories[role], ignored[role])
        for label, contract in cohort_contracts().items():
            selected = [name for name in inventories['compiler-lib']
                        if name.startswith(contract['prefix'])]
            require(selected == contract['names']
                    and not set(selected) & set(ignored['compiler-lib']),
                    'exact nonignored wrapper cohort: ' + label)
            leaf(label, [artifacts['compiler-lib']['pin']['path'], contract['prefix'], '--test-threads=1'],
                 package='rustc-codegen-fe2o3')
            tests[label] = outcomes(OUT / (label + '.stdout'), selected, len(inventories['compiler-lib']))
        selected = [name for name in inventories['compiler-lib']
                    if name.startswith(CFG_DIAGNOSTIC_PREFIX)]
        require(selected == sorted(CFG_DIAGNOSTIC_TESTS)
                and not set(selected) & set(ignored['compiler-lib']),
                'exact eight nonignored ranked-CFG diagnostic tests')
        leaf('cfg-block-limit-diagnostic',
             [artifacts['compiler-lib']['pin']['path'], CFG_DIAGNOSTIC_PREFIX, '--test-threads=1'],
             package='rustc-codegen-fe2o3')
        tests['cfg-block-limit-diagnostic'] = outcomes(
            OUT / 'cfg-block-limit-diagnostic.stdout', selected, len(inventories['compiler-lib']))
        for label, contract in CAPACITY_COHORTS.items():
            role = contract['role']
            selected = [name for name in inventories[role]
                        if name.startswith(contract['prefix'])]
            require(selected == contract['names'] and not set(selected) & set(ignored[role]),
                    'exact nonignored capacity cohort: ' + label)
            leaf(label, [artifacts[role]['pin']['path'], contract['prefix'], '--test-threads=1'],
                 package=ROLES[role][0])
            tests[label] = outcomes(OUT / (label + '.stdout'), selected, len(inventories[role]))
        selected = [name for name in inventories['compiler-lib'] if name.startswith(DAG_PREFIX)]
        require(selected == sorted(DAG_TESTS) and not set(selected) & set(ignored['compiler-lib']),
                'exact nine nonignored indexed-atomic DAG tests')
        leaf('indexed-atomic-dag-certificate',
             [artifacts['compiler-lib']['pin']['path'], DAG_PREFIX, '--test-threads=1'],
             package='rustc-codegen-fe2o3')
        tests['indexed-atomic-dag-certificate'] = outcomes(
            OUT / 'indexed-atomic-dag-certificate.stdout', selected, len(inventories['compiler-lib']))
        selected = [name for name in inventories['compiler-lib'] if name.startswith(GRAPH_WORK_PREFIX)]
        require(selected == sorted(GRAPH_WORK_TESTS) and not set(selected) & set(ignored['compiler-lib']),
                'exact nine nonignored graph-work diagnostic tests')
        leaf('graph-work-diagnostic',
             [artifacts['compiler-lib']['pin']['path'], GRAPH_WORK_PREFIX, '--test-threads=1'],
             package='rustc-codegen-fe2o3')
        tests['graph-work-diagnostic'] = outcomes(
            OUT / 'graph-work-diagnostic.stdout', selected, len(inventories['compiler-lib']))
        selected = [name for name in inventories['compiler-lib'] if name.startswith(MEMBERSHIP_PREFIX)]
        require(selected == sorted(MEMBERSHIP_TESTS) and not set(selected) & set(ignored['compiler-lib']),
                'exact nine nonignored indexed-atomic membership tests')
        leaf('indexed-atomic-membership',
             [artifacts['compiler-lib']['pin']['path'], MEMBERSHIP_PREFIX, '--test-threads=1'],
             package='rustc-codegen-fe2o3')
        tests['indexed-atomic-membership'] = outcomes(
            OUT / 'indexed-atomic-membership.stdout', selected, len(inventories['compiler-lib']))
        selected = [name for name in inventories['compiler-lib'] if name.startswith(CENSUS_PREFIX)]
        require(selected == sorted(CENSUS_TESTS) and not set(selected) & set(ignored['compiler-lib']),
                'exact nonignored indexed-atomic dead-cast census tests')
        leaf('indexed-atomic-dead-cast-census',
             [artifacts['compiler-lib']['pin']['path'], CENSUS_PREFIX, '--test-threads=1'],
             package='rustc-codegen-fe2o3')
        tests['indexed-atomic-dead-cast-census'] = outcomes(
            OUT / 'indexed-atomic-dead-cast-census.stdout', selected, len(inventories['compiler-lib']))
        selected = [name for name in inventories['compiler-lib'] if name.startswith(USE_LOOKUP_PREFIX)]
        require(selected == sorted(USE_LOOKUP_TESTS) and not set(selected) & set(ignored['compiler-lib']),
                'exact nonignored indexed-atomic use-lookup tests')
        leaf('indexed-atomic-use-lookup',
             [artifacts['compiler-lib']['pin']['path'], USE_LOOKUP_PREFIX, '--test-threads=1'],
             package='rustc-codegen-fe2o3')
        tests['indexed-atomic-use-lookup'] = outcomes(
            OUT / 'indexed-atomic-use-lookup.stdout', selected, len(inventories['compiler-lib']))
        selected = [name for name in inventories['compiler-lib'] if name.startswith(BORROW_LOOKUP_PREFIX)]
        require(selected == sorted(BORROW_LOOKUP_TESTS) and not set(selected) & set(ignored['compiler-lib']),
                'exact nonignored indexed-atomic borrow-lookup tests')
        leaf('indexed-atomic-borrow-lookup',
             [artifacts['compiler-lib']['pin']['path'], BORROW_LOOKUP_PREFIX, '--test-threads=1'],
             package='rustc-codegen-fe2o3')
        tests['indexed-atomic-borrow-lookup'] = outcomes(
            OUT / 'indexed-atomic-borrow-lookup.stdout', selected, len(inventories['compiler-lib']))
        selected = [name for name in inventories['compiler-lib'] if name.startswith(CFG_EXPANSION_PREFIX)]
        require(selected == sorted(CFG_EXPANSION_TESTS) and not set(selected) & set(ignored['compiler-lib']),
                'exact nonignored projected-CFG expansion diagnostic tests')
        leaf('cfg-expansion-diagnostic',
             [artifacts['compiler-lib']['pin']['path'], CFG_EXPANSION_PREFIX, '--test-threads=1'],
             package='rustc-codegen-fe2o3')
        tests['cfg-expansion-diagnostic'] = outcomes(
            OUT / 'cfg-expansion-diagnostic.stdout', selected, len(inventories['compiler-lib']))
        selected = [name for name in inventories['compiler-lib'] if name.startswith(CFG_COMPACTION_PREFIX)]
        require(selected == sorted(CFG_COMPACTION_TESTS) and not set(selected) & set(ignored['compiler-lib']),
                'exact nonignored projected-CFG compaction tests')
        leaf('cfg-compaction',
             [artifacts['compiler-lib']['pin']['path'], CFG_COMPACTION_PREFIX, '--test-threads=1'],
             package='rustc-codegen-fe2o3')
        tests['cfg-compaction'] = outcomes(
            OUT / 'cfg-compaction.stdout', selected, len(inventories['compiler-lib']))
        for role in ('atomic-extraction', 'matrix-extraction'):
            require(inventories[role] == lineage['inventories'][role]
                    and ignored[role] == lineage['ignored'][role],
                    'exact inherited extraction executable inventories')
        for role, label_prefix, names in (
            ('atomic-extraction', 'atomic-extraction', ATOMIC),
            ('atomic-extraction', 'source-safety-negative', SOURCE_SAFETY_NEGATIVES),
            ('matrix-extraction', 'matrix-extraction', MATRIX),
        ):
            require(set(names) <= set(ignored[role]), 'explicit historical ignored extraction controls absent')
            for index, name in enumerate(names):
                label = label_prefix + '-' + str(index)
                leaf(label, [artifacts[role]['pin']['path'], name, '--ignored', '--exact', '--test-threads=1'],
                     package='rustc-codegen-fe2o3')
                tests[label] = outcomes(OUT / (label + '.stdout'), [name], len(inventories[role]))
        require(len(phases) == 43 and len(tests) == 30 and len(artifacts) == 7
                and sum(t['passed'] for t in tests.values()) == 2936 + 2 * len(CFG_COMPACTION_TESTS)
                and sum(t['ignored'] for t in tests.values()) == 25,
                'closed43-phase/30-test-scope/seven-product/CFG-compaction-pass/25-ignore completion')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    def check(label, action):
        try:
            remaining = hard_deadline - 5 - time.monotonic()
            require(remaining > 0, 'postcheck whole deadline')
            signal.setitimer(signal.ITIMER_REAL, remaining)
            action()
        except BaseException as error:
            postchecks.append(label + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    def source_check():
        nonlocal final
        final = sources()
        save('sources-after.json', final)
        require(final == before, 'workspace sources/lock changed')
    check('sources', source_check)
    check('input manifest', lambda: require(pin(ROOT/'input-manifest.json') == input_pin, 'manifest drift'))
    check('tools', lambda: require({n:pin(Path(p['path'])) for n,p in tool_pins.items()} == tool_pins, 'tool drift'))
    check('configuration', lambda: require(configurations() == config_before, 'Cargo configuration drift'))
    check('rust-src', lambda: rust_sources(rust_src_expected))
    check('lineage inputs', lambda: require({name: pin(Path(name)) for name in INPUT_PINS} == INPUT_PINS,
                                           'lineage/helper/rust-src manifest drift'))
    external_after = {}
    for directory, expected in external.items():
        def dependency_check(directory=directory, expected=expected):
            path = Path(directory)
            actual = {str(p.relative_to(path)): pin(p) for p in files_below(path, packed=False)}
            external_after[directory] = actual
            require(actual == expected, 'dependency source changed')
        check('dependency:' + directory, dependency_check)
    if external:
        save('dependencies-after.json', external_after)
    for role, row in artifacts.items():
        check(role, lambda row=row: require(pin(Path(row['pin']['path'])) == row['pin'], 'artifact drift'))
    failure = failure or ('postcheck failed' if postchecks else None)
    if time.monotonic() >= hard_deadline:
        failure = failure or 'whole deadline exceeded'
    result = dict(schema='ferric-guarded-mlp-ranked-cfg-compaction-cpu-v1', passed=failure is None,
                  compiler_cohorts=cohort_contracts(), diagnostic_build=False, diagnostic_only=False,
                  cfg_compaction_optimization=True,
                  cfg_compaction_test_filter=CFG_COMPACTION_PREFIX,
                  cfg_compaction_tests=sorted(CFG_COMPACTION_TESTS),
                  cfg_expansion_diagnostics=True, diagnostic_changes_admission=False,
                  inherited_cfg_expansion_diagnostics=True,
                  cfg_expansion_test_filter=CFG_EXPANSION_PREFIX,
                  cfg_expansion_tests=sorted(CFG_EXPANSION_TESTS),
                  static_failure_site_identified=True,
                  actual_failure_block_count_observed=False,
                  cfg_diagnostics_retained=True, cfg_diagnostic_tests=sorted(CFG_DIAGNOSTIC_TESTS),
                  capacity_cohorts=CAPACITY_COHORTS, capacity_limits=CAPACITY_LIMITS,
                  admission_changed=True, structural_capacity_expansion=False,
                  inherited_capacity_expansion=True, structural_limits_changed=False,
                  graph_analysis_optimization=False, inherited_graph_analysis_optimization=True,
                  resource_admission_may_change=True, membership_lookup_optimization=True,
                  inherited_membership_lookup_optimization=True,
                  dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                  use_lookup_optimization=True, inherited_use_lookup_optimization=True,
                  borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
                  compiler_scratch_added=False,
                  borrow_lookup_test_filter=BORROW_LOOKUP_PREFIX,
                  borrow_lookup_tests=sorted(BORROW_LOOKUP_TESTS),
                  use_lookup_test_filter=USE_LOOKUP_PREFIX, use_lookup_tests=sorted(USE_LOOKUP_TESTS),
                  census_test_filter=CENSUS_PREFIX, census_tests=sorted(CENSUS_TESTS),
                  semantic_predicates_changed=False, actual_failure_caller_identified=False,
                  baseline_failure_caller_identified=BASELINE_FAILURE_VERIFIED,
                  baseline_failure_block_count_observed=BASELINE_FAILURE_VERIFIED,
                  baseline_projected_blocks=2778 if BASELINE_FAILURE_VERIFIED else None,
                  baseline_semantic_blocks=1121 if BASELINE_FAILURE_VERIFIED else None,
                  baseline_block_limit=2048 if BASELINE_FAILURE_VERIFIED else None,
                  dag_test_filter=DAG_PREFIX, dag_tests=sorted(DAG_TESTS),
                  graph_work_diagnostics=True, inherited_graph_work_diagnostics=True,
                  graph_work_test_filter=GRAPH_WORK_PREFIX,
                  graph_work_tests=sorted(GRAPH_WORK_TESTS),
                  membership_test_filter=MEMBERSHIP_PREFIX, membership_tests=sorted(MEMBERSHIP_TESTS),
                  failure=failure, postcheck_errors=postchecks, phases=phases, tests=tests,
                  tests_passed=sum(t['passed'] for t in tests.values()),
                  tests_ignored=sum(t['ignored'] for t in tests.values()),
                  focused_repeats_are_not_unique_tests=True,
                  test_inventories=inventories, ignored_inventories=ignored, artifacts=artifacts,
                  compiler_products_before_test_build=compiler_products_before_test_build,
                  historical_product_pins_postchecked=False,
                  final_compiler_product_phase='compiler-tests-build'
                      if {'backend', 'backend-rlib', 'extractor'} <= set(artifacts) else None,
                  source_lineage=inputs['lineage'], metadata_relocations=inputs['metadata_relocations'],
                  lineage_input_pins=INPUT_PINS, helper=pin(ROOT/'qualification_helpers.py'),
                  rust_src=inputs['rust_src'], input_manifest=input_pin,
                  input_sources=source_pin, final_sources=pin(OUT/'sources-after.json')
                      if (OUT/'sources-after.json').exists() else None,
                  source_unchanged=final==before, tool_pins=tool_pins, local_dependencies=local,
                  configurations=config_before, controller=pin(ROOT/'run_cpu.py'),
                  raw={p.name:pin(p) for p in sorted(OUT.iterdir()) if p.is_file()},
                  elapsed_seconds=time.monotonic()-started,
                  limits=dict(whole_seconds=WHOLE_WALL, leaf_seconds=LEAF_WALL,
                              cleanup_reserve_seconds=CLEANUP_RESERVE, address_space_bytes=AS_LIMIT,
                              cpu_seconds_per_leaf=CPU_LIMIT, file_bytes=FILE_LIMIT, cache_bytes=CACHE_LIMIT,
                              stream_bytes=STREAM_LIMIT, initial_free_bytes=START_FREE,
                              live_free_bytes=LIVE_FREE, affinity=[8,9], nice=10, cargo_jobs=2),
                  offline=True, cache_auto_clean='never', gpu_execution=False,
                  guarded_candidate_hsaco_emitted=False, full_model_acceptance=False,
                  numerical_acceptance=False, performance_claim=False)
    save('complete.json' if failure is None else 'failed.json', result)
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
