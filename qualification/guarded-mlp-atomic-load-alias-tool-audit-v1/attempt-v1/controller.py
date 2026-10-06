"""Bounded seven-tool loader audit joined to an actual full S/RPO producer."""
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

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
TOOLS = E / 'guarded-mlp-atomic-load-alias-compiler-tools-v228-v1'
OUT = E / 'guarded-mlp-atomic-load-alias-compiler-tools-audit-v228-v1'
SCRIPT = E / 'audit_guarded_mlp_atomic_load_alias_compiler_tools_v228_v1.py'
AUDIT_INPUT = E / 'guarded-mlp-atomic-load-alias-compiler-tools-audit-inputs-v228-v1.json'
CPU_ROOT = E / 'guarded-mlp-s-rpo-qualification-cpu-v228-v1'
OLD_MANIFEST_SHA = 'e22e8654ff094cc449ad09a9457bcbe5c07e311e3eb46635e40cf7f3cd5888e3'
CPU_CONTROLLER_SHA = '083abf39c6a9d24ff379c238a7f41c56dfb71757a2678a08f65ca935fd2fc866'
CPU_HELPER_SHA = 'a77cc7f82ebdd9518185b30b837e250abc68cc32e4858ab1ad3dc83a0ec8efe6'
PROPOSAL_SHA = '88a30a490707881c73023b8359773ab271561f1442c51c15d314cea933ba94f2'
PREVIOUS_MANIFEST_SHA = '4d0b2f6220056573e2683e9057f7e716f6fcdf6ef35d70d30d9bfc212f2ca9f6'
DRIVER_ROOT = E / 'guarded-mlp-driver-cpu-v228-v1'
DRIVER_CONTROLLER_SHA = '8b72ba779a3df9125ecfd9170ce8964eefccd1b83f2e595fe09d21a2e44f50f5'
DRIVER_PHASES = ('rustc-version', 'metadata', 'driver-tests-build', 'driver-list',
                 'driver-ignored', 'driver-tests', 'driver-build')
DRIVER_GENERATION = '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'
DRIVER_BASE_CPU_SHA = '34df5002c9ca586b5c6dd7e6fc212341b857eedc7ce64bdd4ce0871ccfd3f34f'
DRIVER_BASE_SOURCES_SHA = '955e37e41e54df808221a7d3459f3f782ac7d0b6115ec35e48e14d5f834d4d78'
DRIVER_SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
DRIVER_ENGINEERING = tuple(sorted('engineering_hsaco::tests::' + name for name in (
    'parses_only_exact_target_profiles_cov_and_namespace',
    'rejects_hostile_identities_providers_limits_and_cargo_overrides',
    'manifest_profile_guard_rejects_both_cross_target_directions',
    'extraction_flags_follow_the_requested_profile_without_cargo_overrides',
    'optimized_inline_normalization_is_explicit_and_has_fixed_flags',
    'rejects_loader_and_cargo_selection_environment',
    'isolated_build_std_command_clears_caller_channel_and_owns_fixed_arguments',
    'complete_versioned_build_std_vendor_closure_is_admitted',
    'incomplete_build_std_vendor_fails_before_extraction_command_preparation',
    'build_std_vendor_checksum_substitution_is_rejected',
    'empty_build_std_registry_closure_is_rejected',
    'vendor_configuration_is_closed_and_injection_resistant',
    'claimed_host_linker_path_swap_cannot_change_executed_bytes',
    'substituted_scratch_path_is_never_removed',
    'output_namespace_is_fresh_and_never_uses_production_names',
    'publication_failure_retains_partial_output_without_cleanup',
    'source_has_no_production_or_supervisor_adoption_path',
    'content_namespace_binds_manifest_and_hsaco',
)))
DRIVER_IGNORED = tuple(sorted((
    'profile_command::tests::unauthorised_pc_probe_never_executes_caller_selected_avail',
    'profile_command::tests::real_pc_plan_probes_capability_without_running_beta_capture',
    'profile_command::tests::real_pc_plan_and_fake_collector_publish_the_exact_query_tuple',
    'production_source_isa_unit_matrix_v1::ordinary_source_units_round_trip_through_the_production_observer_on_both_targets',
    'production_source_isa_characteristic_matrix_v2::production_adapter_v2::ordinary_source_units_preserve_characteristic_facts_on_both_targets_v2',
)))
DRIVER_SOURCE_PINS = {
    'fe2o3/crates/cargo-fe2o3/src/engineering_hsaco.rs': '061c6b6e2f34c2f86ec4d37f987bd0a47c72968fc917d8f51899de03a495b94c',
    'fe2o3/crates/cargo-fe2o3/src/engineering_hsaco/execution.rs': 'c9deaaed38da78c4f95687d05f984feda2eac1924d2d9a3db8a45525d0c0be5c',
    'fe2o3/crates/cargo-fe2o3/src/engineering_hsaco/support.rs': 'abf7979e2988077f5aeddac10c5e4f561a7285bdd520736b61fab7971ef063d5',
}

DAG_ROOT = E / 'guarded-mlp-indexed-atomic-dag-cpu-v228-v1'
DAG_CONTROLLER_SHA = 'fe4292daa99ce61e5619333658552f081bc0cc78489eb31cb8ea1b37b8e71947'
DAG_COMPLETE_SHA = '5441c57333baff468a787e83da5852bc74e3286501b25dabc0be8503b8a1f9cf'
DAG_PROPOSAL_SHA = '4889df85ff563f94549d7f767820a5065d82f9e53da593952a96a5def2ad6bb3'
DAG_BASE_ROOT = E / 'guarded-mlp-ranked-cfg-cap-cpu-v228-v2'
DAG_BASE_COMPLETE_SHA = 'bd31abddedd2f7585d56e1e1807add8454e1047c914d1b9e6a9e63c036989e9c'
DAG_BASE_SOURCES_SHA = '130383014f517c1cd0754191361a2e28f52934de018f7b858100acac67ef9597'
DAG_BASE_INPUT_SHA = '1b49135394a395df4d7e36dcc855e6e38324e3a8619c4e17b1d8f058e51036f4'
DAG_BASE_CONTROLLER_SHA = '8f7ec46eb8726c85c7cc5651d426d6003351546c1c865ebef7e410246069e3c7'
DAG_PREFIX = 'production_ranked_projection_v1::tests::indexed_atomic_dag_certificate_'
DAG_NAMES = tuple(sorted(DAG_PREFIX + name for name in (
    'accepts_chains_and_diamonds',
    'bounds_large_acyclic_query_work',
    'charges_exact_budget_and_never_caches_failed_queries',
    'cycles_keep_per_definition_fallback',
    'matches_reference_on_all_small_graphs',
    'refuses_malformed_graph_without_publishing_cache',
    'rejects_invalid_query_before_cache_hit',
    'reuses_only_within_independent_proof_instances',
    'unreachable_cycles_do_not_grant_global_certificate',
)))
DIAGNOSTIC_ROOT = E / 'guarded-mlp-ranked-graph-work-diagnostic-cpu-v228-v2'
DIAGNOSTIC_CONTROLLER_SHA = 'f5179526b41d338f61f84d855b4c2bd301a6710d736c8c0e15ac73406d20652d'
DIAGNOSTIC_COMPLETE_SHA = '501a61fcfc7d660ab7d2c396a7050d04274553cf29e1688e81b0e168d0001e3c'
DAG_SOURCES_SHA = '7a008d22b63e2cb400eff50a596c77014a6d0ad5147955c9b8f725a96f3d698e'
DAG_INPUT_SHA = 'dda82d2befd9180c63b2b28ca5b9f5d7aa94915b620e37ebaac577467e3dd6cb'
GRAPH_WORK_PROPOSAL_SHA = '1a2dc0e17f6811eef3672fc4c86c27e535f2589329c3df695f0ad91c648b725a'
GRAPH_WORK_PREFIX = 'production_ranked_projection_v1::tests::graph_work_diagnostic_'
GRAPH_WORK_NAMES = tuple(sorted(GRAPH_WORK_PREFIX + name for name in (
    'atomic_forwarder_preserves_cell_on_failure',
    'body_context_uses_actual_body_not_root',
    'bounded_paths_names_and_exports_are_escaped',
    'context_preserves_other_errors',
    'direct_callers_are_distinct',
    'exact_limit_and_over_limit_preserve_counter',
    'first_function_context_is_retained',
    'overflow_preserves_counter',
    'proof_forwarder_keeps_caller_and_counter',
)))
MEMBERSHIP_ROOT = E / 'guarded-mlp-indexed-atomic-membership-cpu-v228-v1'
MEMBERSHIP_CONTROLLER_SHA = 'bb91990a900698267ad618ffbf4e24d8ac209dabda7113c960cd104c31f3799d'
MEMBERSHIP_INPUT_SHA = 'a349d29a3c3c4b999d5c2620b22344b2fcb790000a122403526eb32ec7febefb'
MEMBERSHIP_COMPLETE_SHA = 'd13a558cc990c4ebc0fe528362ecbf3c61877e21e3f7f239de69cf8d43a2e1cd'
MEMBERSHIP_SOURCES_SHA = 'ff47dcc22dd01745a93885da1abb083b013ec272f28622b336a7f03cec72cc21'
MEMBERSHIP_PROPOSAL_SHA = '0e98d81d3b437e6bdb25a1a8a70608cf81a2c03fba13864e0793556e9028e8c8'
DIAGNOSTIC_SOURCES_SHA = '31363efaa27ae8879705d76496dbfc74f4f0ece8ea296707932b018fa2008949'
DIAGNOSTIC_INPUT_SHA = '70d87dc213ff298317c6195a4e70d1f66a12d7f846ba4c1e5436007bdd73e90d'
MEMBERSHIP_PREFIX = 'production_ranked_projection_v1::tests::indexed_atomic_membership_'
MEMBERSHIP_NAMES = tuple(sorted(MEMBERSHIP_PREFIX + name for name in (
    'bounds_repeated_measured_size_queries',
    'charges_exact_bound_before_query',
    'failed_charges_do_not_commit_work',
    'handles_boundary_lengths',
    'keeps_independent_inventories',
    'matches_all_small_sorted_sets',
    'preserves_duplicate_and_extreme_ids',
    'preserves_guard_marker_and_escape_refusals',
    'real_inventory_keeps_statement_binding',
)))
MEMBERSHIP_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_membership_v1_tests.rs': True,
}

CENSUS_ROOT = E / 'guarded-mlp-indexed-atomic-dead-cast-census-cpu-v228-v1'
CENSUS_CONTROLLER_SHA = 'c027f1ee7a1837a6d8ba52872a4358fec0b90e963831eb013b16ddb6cba76117'
CENSUS_INPUT_SHA = 'bdf7c0c7c10786a2ea222451817e4febf458d141f19937f30b6ae6812251dac2'
CENSUS_COMPLETE_SHA = '3a5ae5b18b497282a05895b8beca033f316a261c5632f4fbecf02832cd552e3f'
CENSUS_SOURCES_SHA = '94d7a9ffede6584dbd1403273bbb6312fb99c24a767c8b6156c10c37c3fbc1ae'
CENSUS_PROPOSAL_SHA = '2b6050714c19c9661bb2fc6a7fff022833136be617c6cad0f90dcfc8cb4ff8fa'
CENSUS_PREFIX = 'production_ranked_projection_v1::tests::indexed_atomic_dead_cast_census_'
CENSUS_NAMES = tuple(sorted(CENSUS_PREFIX + name for name in (
    'bounds_scratch_before_allocation',
    'charges_exact_build_and_cached_query_bounds',
    'excludes_whole_definition_and_distinguishes_sites',
    'failed_build_never_publishes_partial_state',
    'ignores_storage_and_checks_roles_and_unreachable_blocks',
    'isolates_immutable_functions_and_retains_cast_gates',
    'many_candidates_share_one_complete_scan',
    'matches_legacy_for_all_statement_positions',
    'matches_legacy_for_all_terminator_positions',
)))
CENSUS_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_dead_cast_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_dead_cast_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_dead_cast_census_v1_tests.rs': True,
}

GRAPH_WORK_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/assertion_projected_range_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/cfg_block_limit_diagnostic_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/graph_work_diagnostic_v1.rs': True,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/graph_work_diagnostic_v1_tests.rs': True,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/index_singleton_invalid_query_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/index_singleton_switch_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/induction_body_predicate_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/loop_switch_domain_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/scalar_literal_range_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/zero_comparison_candidate_filter_v1_tests.rs': False,
}
BASE_COMPLETE_SHA = 'e30712ef17c7094efce9bd13bee2bb172823e94f4817c812cfb8b1b20cefb847'
BASE_SOURCES_SHA = 'fa4722605d557e1236809f610bc7af5ebb6e7f15a643ba736392be5b2177bb8a'
DRIVER_MANIFEST_SHA = 'b38ef216900209ea59fbd53a1cb571447f983eb8984ca4a7ce57c1fb10f521fb'
DRIVER_COMPLETE_SHA = 'a869f0f4aa0572c82492cb3d90bdb0dc9bb2625a7b441ff1f3e9a876a87079d9'
CFG_PREFIX = 'production_ranked_projection_v1::tests::cfg_block_limit_diagnostic_'
CFG_NAMES = tuple(sorted(CFG_PREFIX + name for name in (
    'zero_preserves_both_refusals',
    'exact_limit_preserves_graph_acceptance',
    'over_limit_preserves_both_refusals',
    'counts_unreachable_declared_blocks',
    'precedes_work_without_changing_budget',
    'callable_summary_has_no_fabricated_root',
    'root_is_bounded_and_escaped',
    'context_leaves_other_errors_unchanged',
)))
CFG_PATH = 'fe2o3/crates/rustc-codegen-fe2o3/src/'
DAG_FILES = {
    CFG_PATH + 'production_ranked_projection_v1.rs': False,
    CFG_PATH + 'production_ranked_projection_v1/indexed_atomic_v1.rs': False,
    CFG_PATH + 'production_ranked_projection_v1/indexed_atomic_dag_v1.rs': True,
    CFG_PATH + 'production_ranked_projection_v1/indexed_atomic_dag_v1_tests.rs': True,
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

NIGHTLY_LIB = Path('/home/harmenon/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/lib')
NAMES = ('cargo-fe2o3', 'clang-22', 'fe2o3-engineering-lld-proxy',
         'fe2o3-llvm-link-worker', 'fe2o3-rustc-extract',
         'librustc_codegen_fe2o3.so', 'lld')
NIGHTLY = {
    'libLLVM.so.22.1-rust-1.96.0-nightly': (199520544, '8af284bb5ae923ac175ddb3f7b9ad16f1f733f7f5f2779e0c9e8c68ef9ba162b'),
    'librustc_driver-7bb70639c3ace5a4.so': (152936640, 'a0aa61a461841224222b0064f9d77a84fe6d3410745d3d96ceaade6ee679cade'),
    'libLLVM-22-rust-1.96.0-nightly.so': (43, 'd43c716e9a7f6e673b4021e1ced69208a21e2433f0f90b49a8701f8be7c056be'),
}
SIGNALS = (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM)
AS_LIMIT, STREAM_LIMIT, FILE_LIMIT = 12 << 30, 1 << 20, 1 << 30
FREE_FLOOR, WHOLE_SECONDS, CLEANUP_RESERVE = 40 << 30, 600, 50
INPUTS, ALIASES = {}, {}


USE_LOOKUP_ROOT = E / 'guarded-mlp-indexed-atomic-use-lookup-cpu-v228-v1'
USE_LOOKUP_CONTROLLER_SHA = '2051a7e5821979a52d89c3716fef2920e0b1c34f4c091fbd7ed427c2b2ea8bd8'
USE_LOOKUP_INPUT_SHA = '4ae19d4bcb05eee71ba51ef7712a9c5cd70fbebd161f25604b9d1cd596eaad0f'
USE_LOOKUP_COMPLETE_SHA = 'e6d2f3d59d8dff63520be146a8e74cc79ead0baec29ea655e2c369232fa1dea9'
USE_LOOKUP_SOURCES_SHA = '4070da1f4c8ae61c34354b20092c5c1a26b080e43be52c3121551f3cccdbf083'
USE_LOOKUP_PROPOSAL_SHA = 'aeac5c853c40ec768e1b5add4a2decf6338d4a8a15c44abe3bc9962eab000f0e'
USE_LOOKUP_PREFIX = 'production_ranked_projection_v1::tests::indexed_atomic_use_lookup_'
USE_LOOKUP_NAMES = tuple(sorted(USE_LOOKUP_PREFIX + name for name in (
    'matches_all_small_sorted_sites',
    'handles_boundary_lengths',
    'preserves_first_duplicate_and_extreme_sites',
    'charges_exact_bound_before_query',
    'failed_charges_do_not_commit_work',
    'bounds_repeated_measured_size_queries',
    'keeps_independent_inventories',
    'real_inventory_keeps_exact_statement_binding',
    'real_inventory_keeps_atomic_refusals',
)))
USE_LOOKUP_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_use_lookup_v1_tests.rs': True,
}

BORROW_LOOKUP_ROOT = E / 'guarded-mlp-indexed-atomic-borrow-lookup-cpu-v228-v1'
BORROW_LOOKUP_CONTROLLER_SHA = '6c794a2e277596d3ff6e9453a7ecb485cd90fd4260ba67fc946c26f95693c9b8'
BORROW_LOOKUP_INPUT_SHA = '0737513f341124c4bfaf817065ee831ae68165a2d8b17ec13c438fad24e38a5f'
BORROW_LOOKUP_COMPLETE_SHA = '8d55b759a370369023c6692fd64511cfcd2610bf5917d8de23896d28eb2ea3cb'
BORROW_LOOKUP_SOURCES_SHA = '2cac3848055fb7d8cd41d22ba0e4fd29b0a6235e78a2fed179993ebc3c912d31'
BORROW_LOOKUP_PROPOSAL_SHA = '1a089d8bcee0b539e42b7870706da89c23ef60ed494f24e1c4a6ee2fa529d49b'
BORROW_LOOKUP_PREFIX = 'production_ranked_projection_v1::tests::indexed_atomic_borrow_lookup_'
BORROW_LOOKUP_NAMES = tuple(sorted(BORROW_LOOKUP_PREFIX + name for name in (
    'sorts_all_small_shuffled_inventories',
    'preserves_all_exact_place_components',
    'handles_block_and_length_boundaries',
    'charges_queries_transactionally',
    'sort_work_is_charged_and_failure_is_local',
    'bounds_repeated_bucket_queries',
    'keeps_independent_inventories',
    'real_inventory_preserves_provenance_sites',
    'real_inventory_keeps_atomic_refusals',
)))
BORROW_LOOKUP_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_v1_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/indexed_atomic_borrow_lookup_v1_tests.rs': True,
}

CFG_EXPANSION_ROOT = E / 'guarded-mlp-ranked-cfg-expansion-diagnostic-cpu-v228-v1'
CFG_EXPANSION_CONTROLLER_SHA = '69686f865ca9b7d54842889546779a033d47b11cddf262e76137cbcb9f2dcff8'
CFG_EXPANSION_INPUT_SHA = '09e2c31f7a045df0591ba42747d4cb345e21109fd697a497b238f7fa7f78c281'
CFG_EXPANSION_COMPLETE_SHA = 'bcf85b67b2b2d2a80f9b2e4a0fa990b038d848eafc0d8117d321dbb059efdf81'
CFG_EXPANSION_SOURCES_SHA = 'fd7b5ceab4fc2941437863cc03b8a857ab290e826e0e279757125095aeaf1472'
CFG_EXPANSION_PROPOSAL_SHA = '1fe62f3e0f9a892f211cd9b0f75db9eac93d4cfec7c320b46ef55e17a7330de4'
CFG_EXPANSION_PREFIX = 'production_ranked_projection_v1::tests::cfg_projected_block_limit_diagnostic_'
CFG_EXPANSION_NAMES = tuple(sorted(CFG_EXPANSION_PREFIX + name for name in (
    'context_is_bounded',
    'exact_and_next_boundaries',
    'expansion_counts_and_identity',
)))
CFG_EXPANSION_FILES = {
    CFG_PATH + 'production_ranked_projection_v1.rs': False,
    CFG_PATH + 'production_ranked_projection_v1/cfg_block_limit_diagnostic_v1.rs': False,
    CFG_PATH + 'production_ranked_projection_v1/cfg_block_limit_diagnostic_v1_tests.rs': False,
    CFG_PATH + 'production_ranked_projection_v1/analysis_multi_split_v1_tests.rs': False,
}

def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_uid,
            value.st_gid, value.st_nlink, value.st_size,
            value.st_mtime_ns, value.st_ctime_ns)


def pin(path, track=True, cap=FILE_LIMIT):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path,
            'noncanonical input: ' + str(path))
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= cap,
            'not a bounded ordinary file: ' + str(path))
    digest = hashlib.sha256()
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    with os.fdopen(fd, 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed before read')
        for block in iter(lambda: stream.read(1 << 20), b''):
            digest.update(block)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed during read')
    require(stamp(path.lstat()) == stamp(before), 'input changed after read')
    value = dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())
    if track:
        require(str(path) not in INPUTS or INPUTS[str(path)] == value, 'conflicting input pin')
        INPUTS[str(path)] = value
    return value


def resolved(path):
    original = str(path)
    path = Path(path)
    require(path.is_absolute(), 'nonabsolute library/tool path')
    actual = path.resolve(strict=True)
    require(original not in ALIASES or ALIASES[original] == str(actual), 'resolution changed')
    ALIASES[original] = str(actual)
    return pin(actual)


def save(name, value):
    path = OUT / name
    with path.open('x') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')
    return pin(path, track=False)


def group_exists(pgid):
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False


def reap_adopted(child):
    rows = []
    child.poll()
    if child.returncode is None:
        return rows
    while True:
        try:
            pid, status = os.waitpid(-child.pid, os.WNOHANG)
        except ChildProcessError:
            break
        if pid == 0:
            break
        rows.append(dict(pid=pid, wait_status=status))
    return rows


def interrupted(signum, _frame):
    raise RuntimeError('interrupted by signal ' + str(signum))


def owned(label, argv, env, leaves, hard_deadline):
    require(shutil.disk_usage(E).free >= FREE_FLOOR, '40 GiB live storage floor')
    require(time.monotonic() + 30 + CLEANUP_RESERVE < hard_deadline,
            'insufficient whole-run reserve for leaf and cleanup')
    command = ['/usr/bin/prlimit', '--as=' + str(AS_LIMIT), '--cpu=30',
               '--fsize=' + str(STREAM_LIMIT), '--core=0', '--', *argv]
    command_pin = save(label + '.command.json', dict(argv=command, cwd=str(OUT),
                                                    env=env, wall_seconds=30))
    stdout, stderr = OUT / (label + '.stdout'), OUT / (label + '.stderr')
    start, child, exception = time.monotonic_ns(), None, None
    timed_out = forced = False
    reaped, deferred = [], []
    group_absent = False
    with stdout.open('xb') as so, stderr.open('xb') as se:
        previous = {sig: signal.getsignal(sig) for sig in SIGNALS}
        def defer(signum, _frame):
            if len(deferred) < 16:
                deferred.append(signum)
        for sig in SIGNALS:
            signal.signal(sig, defer)
        try:
            require(not deferred, 'termination requested before spawn')
            child = subprocess.Popen(command, cwd=OUT, env=env, stdin=subprocess.DEVNULL,
                                     stdout=so, stderr=se, start_new_session=True)
            require(os.getpgid(child.pid) == child.pid, 'owned process group identity')
            save(label + '.started.json', dict(pid=child.pid, pgid=child.pid, argv=command))
            deadline = time.monotonic() + 30
            while child.poll() is None:
                require(not deferred, 'termination during owned leaf: ' + repr(deferred))
                require(shutil.disk_usage(E).free >= FREE_FLOOR, '40 GiB live storage floor')
                if time.monotonic() >= deadline:
                    timed_out = True
                    break
                time.sleep(0.05)
        except BaseException as error:
            exception = repr(error)
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            try:
                if child is not None:
                    reaped.extend(reap_adopted(child))
                    if child.returncode is None or group_exists(child.pid):
                        forced = True
                        for sig, allowance in ((signal.SIGTERM, 10), (signal.SIGKILL, 20)):
                            try:
                                os.killpg(child.pid, sig)
                            except ProcessLookupError:
                                pass
                            deadline = min(hard_deadline - 5, time.monotonic() + allowance)
                            while time.monotonic() < deadline:
                                reaped.extend(reap_adopted(child))
                                if child.returncode is not None and not group_exists(child.pid):
                                    break
                                time.sleep(0.02)
                            if child.returncode is not None and not group_exists(child.pid):
                                break
                    try:
                        child.wait(timeout=max(0.001, min(5, hard_deadline - time.monotonic())))
                    except subprocess.TimeoutExpired:
                        exception = (exception or '') + ' leader not reaped'
                    reaped.extend(reap_adopted(child))
                    group_absent = not group_exists(child.pid)
            finally:
                for sig, handler in previous.items():
                    signal.signal(sig, handler)
    if deferred:
        exception = (exception or '') + ' deferred signals: ' + repr(deferred)
    row = dict(label=label, command=command_pin, pid=child.pid if child else None,
               pgid=child.pid if child else None, exit_code=child.returncode if child else None,
               timed_out=timed_out, forced_cleanup=forced, exception=exception,
               observed_signals=deferred, adopted_reaped=reaped,
               natural_exit=child is not None and child.returncode is not None
                   and not timed_out and not forced and exception is None,
               reaped=child is not None and child.returncode is not None,
               process_group_absent=group_absent, elapsed_ns=time.monotonic_ns() - start,
               stdout=pin(stdout, False, STREAM_LIMIT), stderr=pin(stderr, False, STREAM_LIMIT))
    leaves.append(row)
    save(label + '.result.json', row)
    require(row['natural_exit'] and row['exit_code'] == 0 and row['reaped'] and group_absent,
            label + ' did not exit naturally, successfully and fully reaped')
    require(shutil.disk_usage(E).free >= FREE_FLOOR, '40 GiB post-leaf storage floor')
    signal.setitimer(signal.ITIMER_REAL, max(0.001, hard_deadline - CLEANUP_RESERVE - time.monotonic()))
    return stdout.read_text()


def libraries(readelf, ldd, shared_object):
    needed = re.findall(r'\(NEEDED\).*Shared library: \[([^\]\n]+)\]', readelf)
    interpreters = re.findall(r'\[Requesting program interpreter: ([^\]\n]+)\]', readelf)
    require(needed and len(needed) == len(set(needed)), 'missing or duplicate DT_NEEDED')
    require(len(interpreters) == (0 if shared_object else 1), 'unexpected PT_INTERP count')
    require(ldd.strip() and 'not found' not in ldd and 'not a dynamic executable' not in ldd,
            'empty or unresolved ldd output')
    named, direct, virtual = {}, [], []
    for line in ldd.splitlines():
        if not line.strip():
            continue
        library = re.fullmatch(r'\s*(\S+) => (/\S+) \(0x[0-9a-fA-F]+\)\s*', line)
        absolute = re.fullmatch(r'\s*(/\S+) \(0x[0-9a-fA-F]+\)\s*', line)
        vdso = re.fullmatch(r'\s*linux-vdso\.so\.1 \(0x[0-9a-fA-F]+\)\s*', line)
        if library:
            name, path = library.groups()
            require(name not in named, 'duplicate resolved SONAME')
            named[name] = dict(reported_path=path, pin=resolved(path))
        elif absolute:
            direct.append(absolute.group(1))
        elif vdso:
            virtual.append('linux-vdso.so.1')
        else:
            raise RuntimeError('unrecognized ldd line: ' + line)
    require(len(virtual) == 1 and len(direct) <= 1, 'ambiguous direct loader/vDSO')
    loader = None
    if direct:
        path = direct[0]
        loader = dict(reported_path=path, pin=resolved(path))
        if Path(path).name in needed:
            name = Path(path).name
            require(name not in named or named[name]['pin'] == loader['pin'],
                    'named/direct loader disagreement')
            named[name] = loader
    interpreter = None
    if interpreters:
        path = interpreters[0]
        interpreter = dict(reported_path=path, pin=resolved(path))
        require(loader is not None and loader['pin'] == interpreter['pin'],
                'PT_INTERP does not match ldd direct loader')
    require(set(needed) <= set(named), 'unresolved DT_NEEDED closure')
    return dict(needed=needed, libraries=named, interpreter=interpreter,
                direct_loader=loader, virtual_objects=virtual, all_needed_resolved=True,
                shared_object_without_interpreter=shared_object)


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def read_json(row, digest=None):
    require(type(row) is dict and set(row) == {'path', 'bytes', 'sha256'},
            'closed input FilePin')
    path = Path(row['path'])
    require(pin(path, cap=64 << 20) == row and (digest is None or row['sha256'] == digest),
            'actual input digest/extent mismatch')
    raw = path.read_bytes()
    require(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256']
            and pin(path, cap=64 << 20) == row, 'retained JSON changed during read')
    return json.loads(raw)


def producer_contract(value, inputs, sources, records, manifest, old):
    require(value['schema'] == 'ferric-guarded-mlp-s-rpo-qualification-cpu-v1'
            and value['passed'] is True and value['failure'] is None
            and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['diagnostic_build'] is False
            and value['controller']['sha256'] == CPU_CONTROLLER_SHA
            and value['helper']['sha256'] == CPU_HELPER_SHA,
            'actual successful reviewed full S/RPO producer required')
    require(inputs['schema'] == 'ferric-guarded-mlp-s-rpo-qualification-input-v1'
            and inputs['lineage'] == value['source_lineage']
            and inputs['lineage']['proposal']['sha256'] == PROPOSAL_SHA
            and value['final_compiler_product_phase'] == 'compiler-tests-build',
            'actual source lineage and final build phase')
    require(len(sources) == 5795 and {name: compact(row) for name, row in sources.items()} == inputs['files']
            and sources['run_cpu.py']['sha256'] == CPU_CONTROLLER_SHA
            and sources['qualification_helpers.py']['sha256'] == CPU_HELPER_SHA,
            'complete 5793-source/controller/helper map')
    phases = value['phases']
    require(len(phases) == len({row['label'] for row in phases}) == 32
            and all(row['exit_code'] == 0 and row['natural_exit'] is True
                    and row['reaped'] is True and row['process_group_absent'] is True
                    and row['forced_cleanup'] is False and row['timed_out'] is False
                    and row['exception'] is None and row['storage_failure'] is None
                    for row in phases), 'all 32 actual CPU leaves must pass naturally')
    require(len(value['tests']) == 19 and value['tests_passed'] == 2798
            and value['tests_ignored'] == 25
            and sum(row['passed'] for row in value['tests'].values()) == value['tests_passed']
            and sum(row['ignored'] for row in value['tests'].values()) == value['tests_ignored']
            and all(row['failed'] == 0 for row in value['tests'].values())
            and (value['tests']['compiler']['passed'], value['tests']['compiler']['ignored']) == (1239, 24)
            and (value['tests']['pliron']['passed'], value['tests']['pliron']['ignored']) == (1504, 1),
            'full named-suite and focused/extraction result census')
    require([row.get('success') for row in records if row.get('reason') == 'build-finished'] == [True],
            'actual final Cargo build-finished success')
    artifacts = value['artifacts']
    require(set(artifacts) == {'backend', 'backend-rlib', 'extractor', 'pliron-lib',
                              'compiler-lib', 'atomic-extraction', 'matrix-extraction'},
            'seven exact final CPU product roles')
    package = CPU_ROOT / 'fe2o3/crates/rustc-codegen-fe2o3'
    for role, name, kind, filename, source in (
            ('backend', 'rustc_codegen_fe2o3', ['dylib', 'rlib'], 'librustc_codegen_fe2o3.so', 'src/lib.rs'),
            ('backend-rlib', 'rustc_codegen_fe2o3', ['dylib', 'rlib'], 'librustc_codegen_fe2o3.rlib', 'src/lib.rs'),
            ('extractor', 'fe2o3-rustc-extract', ['bin'], 'fe2o3-rustc-extract', 'src/bin/fe2o3-rustc-extract.rs')):
        selected = artifacts[role]
        matches = [row for row in records if row.get('reason') == 'compiler-artifact'
                   and row.get('manifest_path') == str(package / 'Cargo.toml')
                   and row.get('target', {}).get('name') == name
                   and sorted(row['target'].get('kind', [])) == kind
                   and sorted(row['target'].get('crate_types', [])) == kind
                   and row['target'].get('src_path') == str(package / source)
                   and row.get('profile', {}).get('test') is False]
        selected_path = Path(selected['pin']['path'])
        require(len(matches) == 1 and selected['cargo_artifact'] == matches[0]
                and selected_path.is_absolute() and '..' not in selected_path.parts
                and selected_path.is_relative_to(CPU_ROOT / 'target')
                and selected_path.name == filename
                and selected['pin']['path'] in matches[0]['filenames'],
                'selected product must be from final checked Cargo records: ' + role)
        require(matches[0]['executable'] == (selected['pin']['path'] if role == 'extractor' else None),
                'selected library/executable Cargo role')
    require(artifacts['backend']['cargo_artifact'] == artifacts['backend-rlib']['cargo_artifact'],
            'backend ELF and rlib must share the final actual library record')
    require(set(manifest) == set(old) == {'bin/' + name for name in NAMES},
            'closed seven-tool deployment manifest')
    replacements = {'fe2o3-rustc-extract': 'extractor', 'librustc_codegen_fe2o3.so': 'backend'}
    for name in NAMES:
        row = manifest['bin/' + name]
        require(set(row) == {'source', 'bytes', 'sha256'}, 'closed deployment tool row')
        if name in replacements:
            product = artifacts[replacements[name]]['pin']
            require(row == dict(source=product['path'], **compact(product)),
                    'new extractor/backend must be actual final producer bodies')
        else:
            require(row == old['bin/' + name], 'legacy driver/linker/worker identity changed')
    return dict(rlib_cpu_provenance=artifacts['backend-rlib'],
                test_elf_cpu_provenance={role: artifacts[role] for role in
                    ('pliron-lib', 'compiler-lib', 'atomic-extraction', 'matrix-extraction')},
                omitted_artifact_bodies_rehashed=False)


def qualified_producer(audit_input, manifest):
    for key, relative in (('producer_complete', 'evidence/complete.json'),
                          ('producer_input', 'input-manifest.json'),
                          ('producer_sources', 'evidence/sources-before.json'),
                          ('producer_final_build', 'evidence/compiler-tests-build.stdout')):
        require(audit_input[key]['path'] == str(CPU_ROOT / relative),
                'exact current producer readset path: ' + key)
    old = read_json(audit_input['old_tool_manifest'], OLD_MANIFEST_SHA)
    value = read_json(audit_input['producer_complete'])
    require(value['input_manifest'] == audit_input['producer_input']
            and value['input_sources'] == audit_input['producer_sources']
            and compact(value['final_sources']) == compact(audit_input['producer_sources'])
            and value['raw']['compiler-tests-build.stdout'] == audit_input['producer_final_build'],
            'producer receipt/readset identity join')
    inputs = read_json(audit_input['producer_input'])
    sources = read_json(audit_input['producer_sources'])
    build_pin = audit_input['producer_final_build']
    path = Path(build_pin['path'])
    require(pin(path, cap=64 << 20) == build_pin, 'actual final Cargo stdout pin')
    raw = path.read_bytes()
    require(len(raw) == build_pin['bytes'] and hashlib.sha256(raw).hexdigest() == build_pin['sha256']
            and pin(path, cap=64 << 20) == build_pin, 'final Cargo stdout changed during read')
    records = [json.loads(line) for line in raw.decode().splitlines() if line.startswith('{')]
    proof = producer_contract(value, inputs, sources, records, manifest, old)
    return dict(complete=audit_input['producer_complete'], input=audit_input['producer_input'],
                sources=audit_input['producer_sources'], final_build=build_pin,
                source_lineage=value['source_lineage'], **proof)



def driver_contract(value, inputs, sources, build, tests_build, manifest, previous):
    require(value['schema'] == 'ferric-guarded-mlp-driver-cpu-v1'
            and value['passed'] is True and value['failure'] is None
            and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['controller']['sha256'] == DRIVER_CONTROLLER_SHA
            and value['supervisor']['sha256'] == DRIVER_SUPERVISOR_SHA
            and value['source_generation'] == DRIVER_GENERATION
            and value['base_cpu_complete']['sha256'] == DRIVER_BASE_CPU_SHA
            and value['base_cpu_sources']['sha256'] == DRIVER_BASE_SOURCES_SHA
            and value['final_artifact_phase'] == 'driver-build'
            and value['test_artifact_phase'] == 'driver-tests-build',
            'actual successful reviewed driver producer required')
    require(set(inputs) == {'schema', 'source_generation', 'files', 'tool_pins'}
            and inputs['schema'] == 'ferric-guarded-mlp-driver-cpu-input-v1'
            and inputs['source_generation'] == DRIVER_GENERATION
            and inputs['tool_pins'] == value['tool_pins']
            and len(sources) == 5299
            and {name: compact(row) for name, row in sources.items()} == inputs['files']
            and len([name for name in sources if name.startswith('fe2o3/')]) == 5297
            and sources['run_cpu.py'] == value['controller']
            and sources['supervisor.py'] == value['supervisor']
            and all(row['path'] == str(DRIVER_ROOT / name) for name, row in sources.items())
            and all(sources[name]['sha256'] == digest for name, digest in DRIVER_SOURCE_PINS.items()),
            'complete actual driver source map and normalized CLI bodies')
    require([row['label'] for row in value['phases']] == list(DRIVER_PHASES)
            and all(row['exit_code'] == 0 and row['natural_exit'] is True
                    and row['reaped'] is True and row['process_group_absent'] is True
                    and row['forced_cleanup'] is False and row['timed_out'] is False
                    and row['exception'] is None and row['storage_failure'] is None
                    for row in value['phases']), 'all seven actual driver leaves must pass naturally')
    require(set(value['tests']) == {'driver'} and value['full_bin_tests_executed'] is False
            and value['test_scope'] == 'engineering_hsaco::tests::'
            and value['engineering_tests'] == list(DRIVER_ENGINEERING),
            'only the exact engineering driver cohort is qualified')
    suite = value['tests']['driver']
    names, ignored = value['inventory'], value['ignored']
    require(names == sorted(set(names)) and ignored == list(DRIVER_IGNORED)
            and set(ignored) <= set(names) and not set(DRIVER_ENGINEERING) & set(ignored)
            and sorted(name for name in names if name.startswith('engineering_hsaco::tests::'))
                == list(DRIVER_ENGINEERING)
            and suite == dict(names=list(DRIVER_ENGINEERING), passed=18, failed=0, ignored=0,
                              filtered_out=len(names) - 18),
            'exact eighteen passing engineering names and compiled filtered census')
    require(set(value['artifacts']) == {'driver', 'test'}, 'exact driver/test artifact roles')
    package = DRIVER_ROOT / 'fe2o3/crates/cargo-fe2o3'
    for role, records, test in (('driver', build, False), ('test', tests_build, True)):
        require([row.get('success') for row in records if row.get('reason') == 'build-finished'] == [True],
                'actual successful final Cargo stream: ' + role)
        matches = [row for row in records if row.get('reason') == 'compiler-artifact'
                   and row.get('manifest_path') == str(package / 'Cargo.toml')
                   and row.get('target', {}).get('name') == 'cargo-fe2o3'
                   and row['target'].get('kind') == ['bin']
                   and row['target'].get('crate_types') == ['bin']
                   and row['target'].get('src_path') == str(package / 'src/main.rs')
                   and row.get('profile', {}).get('test') is test]
        artifact = value['artifacts'][role]
        path = Path(artifact['pin']['path'])
        require(len(matches) == 1 and artifact['cargo_artifact'] == matches[0]
                and path.is_absolute() and '..' not in path.parts
                and path.is_relative_to(DRIVER_ROOT / 'target')
                and str(path) == matches[0]['executable'] and str(path) in matches[0]['filenames']
                and (test or path == DRIVER_ROOT / 'target/debug/cargo-fe2o3'),
                'driver/test body must join its exact final Cargo product: ' + role)
    require(set(manifest) == set(previous) == {'bin/' + name for name in NAMES},
            'closed previous and new seven-tool rosters')
    product = value['artifacts']['driver']['pin']
    for name in NAMES:
        row = manifest['bin/' + name]
        require(set(row) == {'source', 'bytes', 'sha256'}, 'closed new deployment row')
        expected = dict(source=product['path'], **compact(product)) if name == 'cargo-fe2o3' else previous['bin/' + name]
        require(row == expected, 'only the exact final driver may replace a previous tool: ' + name)
    return dict(driver=value['artifacts']['driver'], test_elf_cpu_provenance=value['artifacts']['test'],
                omitted_test_elf_body_rehashed=False)


def qualified_driver(audit_input, manifest, previous):
    paths = (('driver_complete', 'evidence/complete.json'),
             ('driver_input', 'input-manifest.json'),
             ('driver_sources', 'evidence/sources-before.json'),
             ('driver_final_build', 'evidence/driver-build.stdout'),
             ('driver_test_build', 'evidence/driver-tests-build.stdout'))
    for key, relative in paths:
        require(audit_input[key]['path'] == str(DRIVER_ROOT / relative),
                'exact driver readset path: ' + key)
    value = read_json(audit_input['driver_complete'])
    require(value['input_manifest'] == audit_input['driver_input']
            and value['raw']['sources-before.json'] == audit_input['driver_sources']
            and compact(value['raw']['sources-after.json']) == compact(audit_input['driver_sources'])
            and value['raw']['driver-build.stdout'] == audit_input['driver_final_build']
            and value['raw']['driver-tests-build.stdout'] == audit_input['driver_test_build'],
            'actual driver receipt/readset join')
    require(pin(Path(value['controller']['path'])) == value['controller'],
            'actual reviewed driver controller body')
    inputs = read_json(audit_input['driver_input'])
    sources = read_json(audit_input['driver_sources'])
    require(value['input_sources'] == value['final_sources'] == sources,
            'actual initial/final driver source map equality')
    streams = []
    for key in ('driver_final_build', 'driver_test_build'):
        row = audit_input[key]
        path = Path(row['path'])
        require(pin(path, cap=64 << 20) == row, 'driver Cargo stdout pin')
        raw = path.read_bytes()
        require(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256']
                and pin(path, cap=64 << 20) == row, 'driver Cargo stdout changed during read')
        streams.append([json.loads(line) for line in raw.decode().splitlines() if line.startswith('{')])
    proof = driver_contract(value, inputs, sources, *streams, manifest, previous)
    return dict(complete=audit_input['driver_complete'], input=audit_input['driver_input'],
                sources=audit_input['driver_sources'], final_build=audit_input['driver_final_build'],
                test_build=audit_input['driver_test_build'], **proof)



CFG_COMPACTION_ROOT = E / 'guarded-mlp-ranked-cfg-compaction-cpu-v228-v1'
CFG_COMPACTION_CONTROLLER_SHA = 'f5dc942c8d6755d5abe8b2bf3e15881dbc303985b6255c723837b9c4b474d03d'
CFG_COMPACTION_INPUT_SHA = 'eee83b9d3f67eeb383b164e14ba4c8e5e8ee218d14492c3dbbde017135fff2ba'
CFG_COMPACTION_COMPLETE_SHA = '439d4b5bf10a1fe9f44d02fa1cbd28b03eec9601bb161806909b2f073beb9018'
CFG_COMPACTION_SOURCES_SHA = 'c00481ef4e62adf81ebce11b6a2a9ecb51823a709c28ccf87194c54c5fa7f420'
CFG_COMPACTION_PROPOSAL_SHA = 'a996f2ecae0ce3a29e091ecc50f23ef081e57feba005b621856cd55ce6889428'
CFG_COMPACTION_PREFIX = 'production_ranked_projection_v1::tests::cfg_compaction_'
CFG_COMPACTION_NAMES = tuple(sorted(CFG_COMPACTION_PREFIX + name for name in (
    'atomic_and_predicate_refusals_unchanged',
    'differential_checks_effects_and_traps',
    'edge_budget_remains_independent',
    'exact_block_boundary_and_next_refusal',
    'live_induction_layout_unchanged',
    'mixed_failure_layout',
    'multiple_semantic_blocks_preserve_traces',
    'source_wave_and_generated_identity',
    'unreachable_and_zero_eligible_unchanged',
)))
CFG_COMPACTION_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/projection_04_tests.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/cfg_compaction_v1_tests.rs': True,
}

CFG_LINEAR_FUSION_ROOT = E / 'guarded-mlp-ranked-cfg-linear-fusion-cpu-v228-v3'
CFG_LINEAR_FUSION_CONTROLLER_SHA = 'bdcd18f255a8c465adcb67e6831d62568535b6c4e7f83ad07c01a4f4339b6d9a'
CFG_LINEAR_FUSION_INPUT_SHA = '0bb52fac77acd6af93609e214a6117af4047c484547a9728a4f1f3f387da3981'
CFG_LINEAR_FUSION_HELPER_SHA = 'ba127f1057546c0ce6e57fb78832c3e774c1108a4f7c5a2421f4aad7f51108be'
CFG_LINEAR_FUSION_HELPER_BYTES = 4777
CFG_LINEAR_FUSION_COMPLETE_SHA = '7827d969f295a8a0ba709629b2679b71ecde9b93b1f0ad11c5b8be70f7259915'
CFG_LINEAR_FUSION_SOURCES_SHA = '0c5a0dce518d1d469d0f7cb34618245aedb74d9279a4aa50473f2f3e20a1de23'
CFG_LINEAR_FUSION_PROPOSAL_SHA = '8ac7037f7096c4a053876baef36f94dfffb1c7b0072b02bb199e5121e25f041e'
CFG_LINEAR_FUSION_PREFIX = 'production_ranked_projection_v1::tests::cfg_linear_fusion_'
CFG_LINEAR_FUSION_NAMES = tuple(sorted(CFG_LINEAR_FUSION_PREFIX + name for name in (
    'chain_and_identity_boundaries',
    'conditional_effect_traces',
    'cycles_and_argument_fallback',
    'duplicate_edges_and_entry_are_preserved',
    'malformed_references_fail_closed',
    'measured_guard_pattern',
    'production_projection_route',
    'source_wave_generated_coordinates',
    'work_storage_and_failure_boundaries',
)))
CFG_LINEAR_FUSION_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1.rs': False,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/cfg_linear_fusion_v1.rs': True,
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1/cfg_linear_fusion_v1_tests.rs': True,
    'fe2o3/crates/fe2o3-pliron/src/production/ranked.rs': False,
}


MEMORY_BOUNDS_DAG_ROOT = E / 'guarded-mlp-memory-bounds-dag-cpu-v228-v1'
MEMORY_BOUNDS_DAG_CONTROLLER_SHA = '151f0a91534ddf5aa7cec746545dce7a3eda0aae1764b917907075bfeed9b119'
MEMORY_BOUNDS_DAG_INPUT_SHA = '3c60b907c38c1b61d645ec303b6099843e28e8b46148920f7486a364f0faba12'
MEMORY_BOUNDS_DAG_COMPLETE_SHA = 'bd1fa4827483cdf6ca6a0470aacd6826ddab007e8f129611b9725d3e0cdf8445'  # Bind only the actual successful CPU receipt.
MEMORY_BOUNDS_DAG_HELPER_SHA = CFG_LINEAR_FUSION_HELPER_SHA
MEMORY_BOUNDS_DAG_HELPER_BYTES = CFG_LINEAR_FUSION_HELPER_BYTES
MEMORY_BOUNDS_DAG_PROPOSAL_SHA = '7ebe693afa0b672c8e80c0d6c08fe9f7c6a8b0d6bcd478c393b8408dd890eeb8'
MEMORY_BOUNDS_DAG_PREFIX = 'production_analysis::pliron_ranked_bounds::dag_schedule_v1_tests::'
MEMORY_BOUNDS_DAG_NAMES = (
    'production_analysis::pliron_ranked_bounds::dag_schedule_v1_tests::dag_schedule_certificate_mismatch_fails_closed',
    'production_analysis::pliron_ranked_bounds::dag_schedule_v1_tests::dag_schedule_cycles_and_unreachable_fallback',
    'production_analysis::pliron_ranked_bounds::dag_schedule_v1_tests::dag_schedule_dense_guard_production_route',
    'production_analysis::pliron_ranked_bounds::dag_schedule_v1_tests::dag_schedule_differential_dags_and_physical_orders',
    'production_analysis::pliron_ranked_bounds::dag_schedule_v1_tests::dag_schedule_duplicate_edges_and_entry',
    'production_analysis::pliron_ranked_bounds::dag_schedule_v1_tests::dag_schedule_forwarded_arguments_preserve_transport',
    'production_analysis::pliron_ranked_bounds::dag_schedule_v1_tests::dag_schedule_preserves_bounds_refusals',
    'production_analysis::pliron_ranked_bounds::dag_schedule_v1_tests::dag_schedule_preserves_previously_admitted_path',
    'production_analysis::pliron_ranked_bounds::dag_schedule_v1_tests::dag_schedule_work_storage_boundaries',
)
MEMORY_BOUNDS_DAG_FILES = {
    'fe2o3/crates/fe2o3-pliron/src/production_analysis/pliron_pipeline.rs': False,
    'fe2o3/crates/fe2o3-pliron/src/production_analysis/pliron_ranked_bounds/dag_schedule_v1.rs': True,
    'fe2o3/crates/fe2o3-pliron/src/production_analysis/pliron_ranked_bounds/dag_schedule_v1_tests.rs': True,
    'fe2o3/crates/fe2o3-pliron/src/production_analysis/pliron_ranked_bounds/execution_v1.rs': False,
    'fe2o3/crates/fe2o3-pliron/src/production_analysis/pliron_ranked_bounds.rs': False,
}

BASE_FAILURE_ROOT = E / 'guarded-mlp-ranked-cfg-linear-fusion-lowering-v228-v3'
BASE_FAILURE_SHA = '8d4cc1a0cfe0c9104282c3d20349638a616bd98e7e28f92598dcaf0110057ca2'
BASE_FAILURE_STDERR_SHA = 'c60dc2edc62bfe11ba7a8f79ce1465322d2c887aeb3fa5661dae8002a33c5095'
BASE_FAILURE_DIAGNOSTIC = (
    b'fe2o3 rustc extraction: production compilation general kernel verification failed: '
    b'production analysis resource limit exceeded [memory-bounds]: '
    b'memory-bounds work hard limit')

def memory_bounds_dag_contract(value, inputs, sources, records, manifest, previous,
                            base, base_sources, proposal, baseline_failure, baseline_stderr):
    require(base['schema'] == 'ferric-guarded-mlp-ranked-cfg-linear-fusion-cpu-v1'
            and base['passed'] is True and base['failure'] is None
            and base['postcheck_errors'] == [] and base['source_unchanged'] is True
            and base['diagnostic_build'] is False and base['diagnostic_only'] is False
            and base['admission_changed'] is True
            and base['diagnostic_changes_admission'] is False
            and base['cfg_expansion_diagnostics'] is True
            and base['inherited_cfg_expansion_diagnostics'] is True
            and base['cfg_linear_fusion_optimization'] is True
            and base['cfg_compaction_optimization'] is True
            and base['inherited_cfg_compaction_optimization'] is True
            and base['cfg_compaction_test_filter'] == CFG_COMPACTION_PREFIX
            and base['cfg_compaction_tests'] == list(CFG_COMPACTION_NAMES)
            and base['cfg_linear_fusion_test_filter'] == CFG_LINEAR_FUSION_PREFIX
            and base['cfg_linear_fusion_tests'] == list(CFG_LINEAR_FUSION_NAMES)
            and base['structural_capacity_expansion'] is False
            and base['inherited_capacity_expansion'] is True
            and base['structural_limits_changed'] is False
            and base['graph_analysis_optimization'] is False
            and base['inherited_graph_analysis_optimization'] is True
            and base['resource_admission_may_change'] is True
            and base['membership_lookup_optimization'] is True
            and base['inherited_membership_lookup_optimization'] is True
            and base['dead_cast_census_optimization'] is True
            and base['compiler_scratch_added'] is True
            and base['inherited_dead_cast_census_optimization'] is True
            and base['borrow_lookup_optimization'] is True
            and base['inherited_borrow_lookup_optimization'] is True
            and base['use_lookup_optimization'] is True
            and base['inherited_use_lookup_optimization'] is True
            and base['use_lookup_test_filter'] == USE_LOOKUP_PREFIX
            and base['use_lookup_tests'] == list(USE_LOOKUP_NAMES)
            and base['census_test_filter'] == CENSUS_PREFIX
            and base['census_tests'] == list(CENSUS_NAMES)
            and base['membership_test_filter'] == MEMBERSHIP_PREFIX
            and base['membership_tests'] == list(MEMBERSHIP_NAMES)
            and base['semantic_predicates_changed'] is False
            and base['actual_failure_caller_identified'] is False
            and base['baseline_failure_caller_identified'] is True
            and base['baseline_failure_block_count_observed'] is True
            and base['baseline_identity_first_refusal_blocks'] == 1025
            and base['baseline_identity_block_limit'] == 1024
            and base['baseline_rendered_blocks'] == 1675
            and base['baseline_rendered_edges'] == 2230
            and base['baseline_edge_verdict_observed'] is False
            and base['static_failure_site_identified'] is True
            and base['actual_failure_block_count_observed'] is False
            and base['cfg_diagnostics_retained'] is True
            and base['capacity_limits'] == CAPACITY_LIMITS
            and base['capacity_cohorts'] == CAPACITY_COHORTS
            and base['cfg_diagnostic_tests'] == list(CFG_NAMES)
            and base['dag_test_filter'] == DAG_PREFIX and base['dag_tests'] == list(DAG_NAMES)
            and base['graph_work_diagnostics'] is True and base['inherited_graph_work_diagnostics'] is True
            and base['graph_work_test_filter'] == GRAPH_WORK_PREFIX
            and base['graph_work_tests'] == list(GRAPH_WORK_NAMES)
            and base['borrow_lookup_test_filter'] == BORROW_LOOKUP_PREFIX
            and base['borrow_lookup_tests'] == list(BORROW_LOOKUP_NAMES)
            and base['cfg_expansion_test_filter'] == CFG_EXPANSION_PREFIX
            and base['cfg_expansion_tests'] == list(CFG_EXPANSION_NAMES)
            and base['controller']['sha256'] == CFG_LINEAR_FUSION_CONTROLLER_SHA
            and base['helper'] == dict(
                path=str(CFG_LINEAR_FUSION_ROOT / 'qualification_helpers.py'),
                bytes=CFG_LINEAR_FUSION_HELPER_BYTES, sha256=CFG_LINEAR_FUSION_HELPER_SHA)
            and base['final_compiler_product_phase'] == 'compiler-tests-build'
            and base['source_lineage']['cfg_linear_fusion_proposal']['sha256'] == CFG_LINEAR_FUSION_PROPOSAL_SHA
            and base['tests_passed'] == 2972 and base['tests_ignored'] == 25
            and len(base['phases']) == 44 and len(base['tests']) == 31
            and len(base_sources) == 5808
            and base_sources['run_cpu.py'] == base['controller']
            and base_sources['qualification_helpers.py'] == base['helper']
            and all(row['path'] == str(CFG_LINEAR_FUSION_ROOT / name)
                    for name, row in base_sources.items()),
            'actual direct qualified CFG linear fusion baseline required')
    require(value['schema'] == 'ferric-guarded-mlp-memory-bounds-dag-cpu-v1'
            and value['passed'] is True and value['failure'] is None
            and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['diagnostic_build'] is False and value['diagnostic_only'] is False
            and value['admission_changed'] is True
            and value['diagnostic_changes_admission'] is False
            and value['cfg_expansion_diagnostics'] is True
            and value['inherited_cfg_expansion_diagnostics'] is True
            and value['memory_bounds_dag_optimization'] is True
            and value['memory_bounds_dag_test_filter'] == MEMORY_BOUNDS_DAG_PREFIX
            and value['memory_bounds_dag_tests'] == list(MEMORY_BOUNDS_DAG_NAMES)
            and value['inherited_cfg_linear_fusion_optimization'] is True
            and value['cfg_linear_fusion_optimization'] is True
            and value['cfg_compaction_optimization'] is True
            and value['inherited_cfg_compaction_optimization'] is True
            and value['cfg_compaction_test_filter'] == CFG_COMPACTION_PREFIX
            and value['cfg_compaction_tests'] == list(CFG_COMPACTION_NAMES)
            and value['cfg_linear_fusion_test_filter'] == CFG_LINEAR_FUSION_PREFIX
            and value['cfg_linear_fusion_tests'] == list(CFG_LINEAR_FUSION_NAMES)
            and value['structural_capacity_expansion'] is False
            and value['inherited_capacity_expansion'] is True
            and value['structural_limits_changed'] is False
            and value['graph_analysis_optimization'] is True
            and value['inherited_graph_analysis_optimization'] is True
            and value['resource_admission_may_change'] is True
            and value['membership_lookup_optimization'] is True
            and value['inherited_membership_lookup_optimization'] is True
            and value['dead_cast_census_optimization'] is True
            and value['compiler_scratch_added'] is True
            and value['inherited_dead_cast_census_optimization'] is True
            and value['borrow_lookup_optimization'] is True
            and value['inherited_borrow_lookup_optimization'] is True
            and value['use_lookup_optimization'] is True
            and value['inherited_use_lookup_optimization'] is True
            and value['use_lookup_test_filter'] == USE_LOOKUP_PREFIX
            and value['use_lookup_tests'] == list(USE_LOOKUP_NAMES)
            and value['census_test_filter'] == CENSUS_PREFIX
            and value['census_tests'] == list(CENSUS_NAMES)
            and value['membership_test_filter'] == MEMBERSHIP_PREFIX
            and value['membership_tests'] == list(MEMBERSHIP_NAMES)
            and value['semantic_predicates_changed'] is False
            and value['actual_failure_caller_identified'] is False
            and value['baseline_failure_caller_identified'] is True
            and value['baseline_failure_block_count_observed'] is True
            and value['baseline_memory_bounds_preflight_refusal'] is True
            and value['baseline_rendered_blocks'] == 567
            and value['baseline_rendered_edges'] == 1122
            and value['baseline_rendered_operations'] == 2240
            and value['baseline_guard_candidates'] == 552
            and value['baseline_intersection_work_upper_bound'] == 9340170
            and value['baseline_memory_bounds_work_limit'] == CAPACITY_LIMITS['ranked_work']
            and value['baseline_runtime_work_exhaustion_observed'] is False
            and value['baseline_edge_verdict_observed'] is False
            and value['static_failure_site_identified'] is True
            and value['actual_failure_block_count_observed'] is False
            and value['cfg_diagnostics_retained'] is True
            and value['capacity_limits'] == CAPACITY_LIMITS
            and value['capacity_cohorts'] == CAPACITY_COHORTS
            and value['cfg_diagnostic_tests'] == list(CFG_NAMES)
            and value['compiler_cohorts'] == base['compiler_cohorts']
            and value['dag_test_filter'] == DAG_PREFIX and value['dag_tests'] == list(DAG_NAMES)
            and value['graph_work_diagnostics'] is True and value['inherited_graph_work_diagnostics'] is True
            and value['graph_work_test_filter'] == GRAPH_WORK_PREFIX
            and value['graph_work_tests'] == list(GRAPH_WORK_NAMES)
            and value['borrow_lookup_test_filter'] == BORROW_LOOKUP_PREFIX
            and value['borrow_lookup_tests'] == list(BORROW_LOOKUP_NAMES)
            and value['cfg_expansion_test_filter'] == CFG_EXPANSION_PREFIX
            and value['cfg_expansion_tests'] == list(CFG_EXPANSION_NAMES)
            and value['controller']['sha256'] == MEMORY_BOUNDS_DAG_CONTROLLER_SHA
            and value['helper'] == dict(
                path=str(MEMORY_BOUNDS_DAG_ROOT / 'qualification_helpers.py'),
                bytes=MEMORY_BOUNDS_DAG_HELPER_BYTES, sha256=MEMORY_BOUNDS_DAG_HELPER_SHA)
            and value['final_compiler_product_phase'] == 'compiler-tests-build',
            'actual successful memory-bounds DAG producer required')
    require(set(inputs) == {'schema', 'files', 'tool_pins', 'lineage',
                            'metadata_relocations', 'rust_src'}
            and inputs['schema'] == 'ferric-guarded-mlp-memory-bounds-dag-cpu-input-v1'
            and inputs['lineage'] == value['source_lineage']
            and len(sources) == 5810
            and {name: compact(row) for name, row in sources.items()} == inputs['files']
            and sources['run_cpu.py'] == value['controller']
            and sources['qualification_helpers.py'] == value['helper']
            and all(row['path'] == str(MEMORY_BOUNDS_DAG_ROOT / name) for name, row in sources.items()),
            'complete memory-bounds DAG source/controller/helper map')
    lineage = inputs['lineage']
    require(set(lineage) == {'base_complete', 'base_sources', 'base_input', 'base_metadata',
                            'base_dependencies', 'base_streams', 'memory_bounds_dag_proposal', 'memory_bounds_dag_overlay',
                            'baseline_failure', 'baseline_failure_stderr'}
            and lineage['base_complete']['sha256'] == CFG_LINEAR_FUSION_COMPLETE_SHA
            and lineage['base_sources']['sha256'] == CFG_LINEAR_FUSION_SOURCES_SHA
            and lineage['base_input']['sha256'] == CFG_LINEAR_FUSION_INPUT_SHA
            and lineage['memory_bounds_dag_proposal']['sha256'] == MEMORY_BOUNDS_DAG_PROPOSAL_SHA
            and proposal['schema'] == 'ferric-guarded-mlp-memory-bounds-dag-source-v1'
            and proposal['source_only'] is True
            and proposal['memory_bounds_dag_optimization'] is True
            and proposal['production_requires_zero_ownership_contracts'] is True
            and proposal['inherited_cfg_linear_fusion_optimization'] is True
            and proposal['cfg_linear_fusion_optimization'] is True
            and proposal['cfg_compaction_optimization'] is True
            and proposal['inherited_cfg_compaction_optimization'] is True
            and proposal['admission_changed'] is True
            and proposal['compiler_scratch_added'] is True
            and proposal['graph_analysis_optimization'] is True
            and proposal['semantic_predicates_changed'] is False
            and proposal['structural_limits_changed'] is False
            and proposal['resource_admission_may_change'] is True
            and proposal['actual_failure_caller_identified'] is False
            and proposal['actual_failure_block_count_observed'] is False
            and proposal['static_failure_site_identified'] is True
            and proposal['actual_failure'] == compact(lineage['baseline_failure'])
            and proposal['actual_failure_stderr'] == compact(lineage['baseline_failure_stderr'])
            and proposal['baseline_failure_caller_identified'] is True
            and proposal['baseline_failure_block_count_observed'] is True
            and proposal['baseline_memory_bounds_preflight_refusal'] is True
            and proposal['baseline_rendered_blocks'] == 567
            and proposal['baseline_rendered_edges'] == 1122
            and proposal['baseline_rendered_operations'] == 2240
            and proposal['baseline_guard_candidates'] == 552
            and proposal['baseline_intersection_work_upper_bound'] == 9340170
            and proposal['work_limit'] == CAPACITY_LIMITS['ranked_work']
            and proposal['baseline_runtime_work_exhaustion_observed'] is False
            and proposal['baseline_edge_verdict_observed'] is False
            and proposal['base_complete'] == compact(lineage['base_complete'])
            and proposal['base_sources'] == compact(lineage['base_sources'])
            and proposal['base_input'] == compact(lineage['base_input'])
            and proposal['base_controller'] == compact(base['controller'])
            and proposal['graph_work_limit'] == 3145728
            and proposal['block_limit'] == 2048 and proposal['edge_limit'] == 2048
            and proposal['fact_limit'] == CAPACITY_LIMITS['facts']
            and proposal['storage_limit'] == CAPACITY_LIMITS['ranked_storage']
            and proposal['base_source_count'] == 5806 and proposal['source_count'] == 5808
            and proposal['source_file_additions'] == 2 and proposal['new_test_count'] == 9
            and proposal['filter'] == MEMORY_BOUNDS_DAG_PREFIX
            and sorted(proposal['test_names']) == list(MEMORY_BOUNDS_DAG_NAMES),
            'exact qualified CFG linear fusion baseline and reviewed memory-bounds DAG proposal')
    require(lineage['baseline_failure'] == dict(path=str(BASE_FAILURE_ROOT / 'failed.json'),
                bytes=17737, sha256=BASE_FAILURE_SHA)
            and lineage['baseline_failure_stderr'] == dict(
                path=str(BASE_FAILURE_ROOT / 'compile.stderr'), bytes=123245,
                sha256=BASE_FAILURE_STDERR_SHA), 'exact measured baseline failure inputs')
    require(baseline_failure['schema'] == 'ferric-guarded-mlp-ranked-cfg-linear-fusion-lowering-result-v1'
            and baseline_failure['passed'] is False
            and baseline_failure['cfg_linear_fusion_optimization'] is True
            and baseline_failure['cfg_compaction_optimization'] is True
            and baseline_failure['diagnostic_build'] is False
            and baseline_failure['diagnostic_only'] is False
            and baseline_failure['compiler_scratch_added'] is True
            and baseline_failure['actual_failure_caller_identified'] is False
            and baseline_failure['actual_failure_block_count_observed'] is False
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
                 if line.startswith(b'fe2o3 rustc extraction:')] == [BASE_FAILURE_DIAGNOSTIC],
            'one exact memory-bounds preflight diagnostic without additional authority')
    # This is an inventory of the pinned rendered text, not an edge-admission verdict.
    rendered_parts = baseline_stderr.split(b'  = ranked PLIRON before rejected lowering:\n')
    require(len(rendered_parts) == 2, 'one retained ranked graph diagnostic required')
    rendered_end = rendered_parts[1].split(b'  = lowering stopped before target IR or artifact emission\n')
    require(len(rendered_end) == 2, 'one complete bounded ranked graph rendering required')
    rendered_graph = rendered_end[0]
    require(rendered_graph.startswith(b'    func @ferric_qwen3_mlp_state_guard_v1 {\n')
            and rendered_graph.endswith(b'    }\n')
            and rendered_graph.count(b'    func @') == 1,
            'rendered failure graph belongs to the exact guarded root')
    block_ids = re.findall(rb'^    \^bb([0-9]+):$', rendered_graph, re.MULTILINE)
    block_references = re.findall(rb'\^bb([0-9]+)', rendered_graph)
    operation_lines = [line for line in rendered_graph.splitlines() if line.startswith(b'      ')]
    guard_lines = re.findall(
        rb'^      kernel\.cond_br %[0-9]+ < %[0-9]+ \^bb[0-9]+, \^bb[0-9]+$',
        rendered_graph, re.MULTILINE)
    require(block_ids == [str(index).encode() for index in range(567)]
            and len(block_references) - len(block_ids) == 1122
            and set(block_references) == set(block_ids)
            and len(operation_lines) == 2240 and len(guard_lines) == 552,
            'exact rendered baseline census: 567 blocks, 1122 edges, 2240 operations, 552 guards')
    require((567 + 1122) * ((552 + 63) // 64 + 1) * (552 + 1) == 9340170
            and 9340170 > CAPACITY_LIMITS['ranked_work'],
            'known preflight intersection term exceeds unchanged hard work cap')
    expected = {name: compact(row) for name, row in base_sources.items()
                if name.startswith('fe2o3/')}
    overlay = {'fe2o3/' + name: row for name, row in proposal['files'].items()}
    require(len(expected) == 5806 and set(overlay) == set(MEMORY_BOUNDS_DAG_FILES)
            and lineage['memory_bounds_dag_overlay'] == overlay, 'closed five-file memory-bounds DAG overlay')
    for name, row in overlay.items():
        require(set(row) == {'before', 'after'}
                and row['before'] == expected.get(name)
                and (name not in expected) is MEMORY_BOUNDS_DAG_FILES[name]
                and set(row['after']) == {'bytes', 'sha256'}
                and row['after'] != row['before'], 'exact memory-bounds DAG preimage/addition')
        expected[name] = row['after']
    require(len(expected) == 5808
            and {name: compact(row) for name, row in sources.items()
                 if name.startswith('fe2o3/')} == expected,
            'all source bodies outside memory-bounds DAG overlay must remain exact')
    phases = value['phases']
    labels = [row['label'] for row in phases]
    old_labels = [row['label'] for row in base['phases']]
    extra = 'memory-bounds-dag'
    require(len(labels) == len(set(labels)) == 45
            and len(old_labels) == len(set(old_labels)) == 44
            and [label for label in labels if label != extra] == old_labels
            and labels[labels.index('cfg-linear-fusion')+1] == extra
            and labels[labels.index('atomic-extraction-0')-1] == extra
            and all(row['exit_code'] == 0 and row['natural_exit'] is True
                    and row['reaped'] is True and row['process_group_absent'] is True
                    and row['forced_cleanup'] is False and row['timed_out'] is False
                    and row['exception'] is None and row['storage_failure'] is None
                    for row in [*base['phases'], *phases]),
            'all inherited CPU leaves plus one exact memory-bounds DAG repeat')
    inventories = dict(base['test_inventories'])
    additions = {'compiler-lib': [], 'pliron-lib': list(MEMORY_BOUNDS_DAG_NAMES)}
    require(not set(MEMORY_BOUNDS_DAG_NAMES) & set(inventories['pliron-lib']), 'memory-bounds DAG names must be new')
    inventories['pliron-lib'] = sorted(inventories['pliron-lib'] + list(MEMORY_BOUNDS_DAG_NAMES))
    require(value['test_inventories'] == inventories
            and value['ignored_inventories'] == base['ignored_inventories'],
            'exact old compiled inventories and ignore identities plus nine names')
    tests = value['tests']
    require(set(tests) == set(base['tests']) | {extra} and len(tests) == 32
            and value['tests_passed'] == 2990 and value['tests_ignored'] == 25
            and sum(row['passed'] for row in tests.values()) == 2990
            and sum(row['ignored'] for row in tests.values()) == 25,
            'thirty-two scopes and actual passing/ignored memory-bounds DAG')
    pliron_repeats = {label for label, row in CAPACITY_COHORTS.items()
                      if row['role'] == 'pliron-lib'}
    for label, prior in base['tests'].items():
        wanted = dict(prior)
        if label in ('compiler', 'pliron'):
            role = label + '-lib'
            wanted['names'] = inventories[role]
            wanted['named_outcomes'] = dict(prior['named_outcomes'],
                                            **{name: 'ok' for name in additions[role]})
            wanted['passed'] += len(additions[role])
        elif label in pliron_repeats:
            wanted['filtered_out'] += len(MEMORY_BOUNDS_DAG_NAMES)
        require(tests[label] == wanted, 'inherited named outcomes changed: ' + label)
    require(tests[extra] == dict(names=list(MEMORY_BOUNDS_DAG_NAMES), passed=9, failed=0, ignored=0,
                                filtered_out=len(inventories['pliron-lib'])-9),
            'actual passing memory-bounds DAG names')
    require([row.get('success') for row in records if row.get('reason') == 'build-finished'] == [True],
            'actual final Cargo build-finished success')
    artifacts = value['artifacts']
    require(set(artifacts) == {'backend', 'backend-rlib', 'extractor', 'pliron-lib',
                              'compiler-lib', 'atomic-extraction', 'matrix-extraction'},
            'seven exact final CPU product roles')
    package = MEMORY_BOUNDS_DAG_ROOT / 'fe2o3/crates/rustc-codegen-fe2o3'
    for role, name, kind, filename, source in (
            ('backend', 'rustc_codegen_fe2o3', ['dylib', 'rlib'], 'librustc_codegen_fe2o3.so', 'src/lib.rs'),
            ('backend-rlib', 'rustc_codegen_fe2o3', ['dylib', 'rlib'], 'librustc_codegen_fe2o3.rlib', 'src/lib.rs'),
            ('extractor', 'fe2o3-rustc-extract', ['bin'], 'fe2o3-rustc-extract', 'src/bin/fe2o3-rustc-extract.rs')):
        selected = artifacts[role]
        matches = [row for row in records if row.get('reason') == 'compiler-artifact'
                   and row.get('manifest_path') == str(package / 'Cargo.toml')
                   and row.get('target', {}).get('name') == name
                   and sorted(row['target'].get('kind', [])) == kind
                   and sorted(row['target'].get('crate_types', [])) == kind
                   and row['target'].get('src_path') == str(package / source)
                   and row.get('profile', {}).get('test') is False]
        selected_path = Path(selected['pin']['path'])
        require(len(matches) == 1 and selected['cargo_artifact'] == matches[0]
                and selected_path.is_absolute() and '..' not in selected_path.parts
                and selected_path.is_relative_to(MEMORY_BOUNDS_DAG_ROOT / 'target')
                and selected_path.name == filename
                and selected['pin']['path'] in matches[0]['filenames'],
                'selected product must be from final checked Cargo records: ' + role)
        require(matches[0]['executable'] == (selected['pin']['path'] if role == 'extractor' else None),
                'selected library/executable Cargo role')
    require(artifacts['backend']['cargo_artifact'] == artifacts['backend-rlib']['cargo_artifact'],
            'backend ELF and rlib must share the final actual library record')
    require(set(manifest) == set(previous) == {'bin/' + name for name in NAMES},
            'closed seven-tool deployment manifest')
    replacements = {'fe2o3-rustc-extract': 'extractor', 'librustc_codegen_fe2o3.so': 'backend'}
    for name in NAMES:
        row = manifest['bin/' + name]
        require(set(row) == {'source', 'bytes', 'sha256'}, 'closed deployment tool row')
        if name in replacements:
            product = artifacts[replacements[name]]['pin']
            require(row == dict(source=product['path'], **compact(product)),
                    'new extractor/backend must be actual final producer bodies')
        else:
            require(row == previous['bin/' + name], 'unchanged qualified driver/linker/worker identity changed')
    return dict(rlib_cpu_provenance=artifacts['backend-rlib'],
                test_elf_cpu_provenance={role: artifacts[role] for role in
                    ('pliron-lib', 'compiler-lib', 'atomic-extraction', 'matrix-extraction')},
                omitted_artifact_bodies_rehashed=False)



def qualified_memory_bounds_dag_producer(audit_input, manifest, previous):
    for key, relative in (('memory_bounds_dag_complete', 'evidence/complete.json'),
                          ('memory_bounds_dag_input', 'input-manifest.json'),
                          ('memory_bounds_dag_sources', 'evidence/sources-before.json'),
                          ('memory_bounds_dag_final_build', 'evidence/compiler-tests-build.stdout')):
        require(audit_input[key]['path'] == str(MEMORY_BOUNDS_DAG_ROOT / relative),
                'exact memory-bounds DAG producer readset: ' + key)
    value = read_json(audit_input['memory_bounds_dag_complete'], MEMORY_BOUNDS_DAG_COMPLETE_SHA)
    require(value['input_manifest'] == audit_input['memory_bounds_dag_input']
            and value['input_sources'] == audit_input['memory_bounds_dag_sources']
            and compact(value['final_sources']) == compact(audit_input['memory_bounds_dag_sources'])
            and value['raw']['compiler-tests-build.stdout'] == audit_input['memory_bounds_dag_final_build'],
            'actual memory-bounds DAG receipt/readset join')
    inputs = read_json(audit_input['memory_bounds_dag_input'], MEMORY_BOUNDS_DAG_INPUT_SHA)
    sources = read_json(audit_input['memory_bounds_dag_sources'])
    lineage = inputs['lineage']
    base = read_json(lineage['base_complete'], CFG_LINEAR_FUSION_COMPLETE_SHA)
    base_sources = read_json(lineage['base_sources'], CFG_LINEAR_FUSION_SOURCES_SHA)
    base_input = read_json(lineage['base_input'], CFG_LINEAR_FUSION_INPUT_SHA)
    require(compact(base['input_manifest']) == compact(lineage['base_input'])
            and compact(base['input_sources']) == compact(lineage['base_sources'])
            and compact(base['final_sources']) == compact(lineage['base_sources'])
            and base_input['files'] == {name: compact(row) for name, row in base_sources.items()}
            and base_input['schema'] == 'ferric-guarded-mlp-ranked-cfg-linear-fusion-cpu-input-v1'
            and inputs['tool_pins'] == base_input['tool_pins'] == base['tool_pins'] == value['tool_pins']
            and compact(inputs['rust_src']) == compact(base_input['rust_src'])
                == compact(base['rust_src']) == compact(value['rust_src'])
            and base['source_lineage']['base_complete']['sha256'] == CFG_COMPACTION_COMPLETE_SHA
            and base['source_lineage']['base_sources']['sha256'] == CFG_COMPACTION_SOURCES_SHA
            and base['source_lineage']['base_input']['sha256'] == CFG_COMPACTION_INPUT_SHA,
            'direct linear-fusion baseline/input/tool/source and inherited compaction joins')
    compaction = read_json(base['source_lineage']['base_complete'], CFG_COMPACTION_COMPLETE_SHA)
    require(compaction['source_lineage']['base_complete']['sha256'] == CFG_EXPANSION_COMPLETE_SHA
            and compaction['source_lineage']['base_sources']['sha256'] == CFG_EXPANSION_SOURCES_SHA
            and compaction['source_lineage']['base_input']['sha256'] == CFG_EXPANSION_INPUT_SHA,
            'inherited compaction generation retains exact CFG-expansion lineage')
    cfg_expansion = read_json(compaction['source_lineage']['base_complete'], CFG_EXPANSION_COMPLETE_SHA)
    require(cfg_expansion['source_lineage']['base_complete']['sha256'] == BORROW_LOOKUP_COMPLETE_SHA
            and cfg_expansion['source_lineage']['base_sources']['sha256'] == BORROW_LOOKUP_SOURCES_SHA
            and cfg_expansion['source_lineage']['base_input']['sha256'] == BORROW_LOOKUP_INPUT_SHA,
            'inherited CFG-expansion generation retains exact borrow-lookup lineage')
    borrow_lookup = read_json(cfg_expansion['source_lineage']['base_complete'], BORROW_LOOKUP_COMPLETE_SHA)
    require(borrow_lookup['source_lineage']['base_complete']['sha256'] == USE_LOOKUP_COMPLETE_SHA
            and borrow_lookup['source_lineage']['base_sources']['sha256'] == USE_LOOKUP_SOURCES_SHA
            and borrow_lookup['source_lineage']['base_input']['sha256'] == USE_LOOKUP_INPUT_SHA,
            'inherited borrow-lookup generation retains exact use-lookup lineage')
    use_lookup = read_json(borrow_lookup['source_lineage']['base_complete'], USE_LOOKUP_COMPLETE_SHA)
    require(use_lookup['source_lineage']['base_complete']['sha256'] == CENSUS_COMPLETE_SHA
            and use_lookup['source_lineage']['base_sources']['sha256'] == CENSUS_SOURCES_SHA
            and use_lookup['source_lineage']['base_input']['sha256'] == CENSUS_INPUT_SHA,
            'inherited use-lookup generation retains exact census lineage')
    census = read_json(use_lookup['source_lineage']['base_complete'], CENSUS_COMPLETE_SHA)
    require(census['source_lineage']['base_complete']['sha256'] == MEMBERSHIP_COMPLETE_SHA
            and census['source_lineage']['base_sources']['sha256'] == MEMBERSHIP_SOURCES_SHA
            and census['source_lineage']['base_input']['sha256'] == MEMBERSHIP_INPUT_SHA,
            'inherited census generation retains exact membership lineage')
    membership = read_json(census['source_lineage']['base_complete'], MEMBERSHIP_COMPLETE_SHA)
    require(membership['source_lineage']['base_complete']['sha256'] == DIAGNOSTIC_COMPLETE_SHA
            and membership['source_lineage']['base_sources']['sha256'] == DIAGNOSTIC_SOURCES_SHA
            and membership['source_lineage']['base_input']['sha256'] == DIAGNOSTIC_INPUT_SHA,
            'inherited membership generation retains exact diagnostic lineage')
    diagnostic = read_json(membership['source_lineage']['base_complete'], DIAGNOSTIC_COMPLETE_SHA)
    require(diagnostic['source_lineage']['base_complete']['sha256'] == DAG_COMPLETE_SHA
            and diagnostic['source_lineage']['base_sources']['sha256'] == DAG_SOURCES_SHA
            and diagnostic['source_lineage']['base_input']['sha256'] == DAG_INPUT_SHA,
            'inherited diagnostic generation retains exact DAG lineage')
    dag = read_json(diagnostic['source_lineage']['base_complete'], DAG_COMPLETE_SHA)
    require(dag['source_lineage']['base_complete']['sha256'] == DAG_BASE_COMPLETE_SHA
            and dag['source_lineage']['base_sources']['sha256'] == DAG_BASE_SOURCES_SHA
            and dag['source_lineage']['base_input']['sha256'] == DAG_BASE_INPUT_SHA,
            'inherited DAG generation retains exact capacity lineage')
    capacity = read_json(dag['source_lineage']['base_complete'], DAG_BASE_COMPLETE_SHA)
    require(compact(capacity['source_lineage']['base_complete']) == compact(audit_input['producer_complete'])
            and compact(capacity['source_lineage']['base_sources']) == compact(audit_input['producer_sources']),
            'inherited cap baseline joins independently replayed full S producer')
    proposal = read_json(lineage['memory_bounds_dag_proposal'], MEMORY_BOUNDS_DAG_PROPOSAL_SHA)
    baseline_failure = read_json(lineage['baseline_failure'], BASE_FAILURE_SHA)
    failure_stderr_pin = lineage['baseline_failure_stderr']
    failure_stderr_path = Path(failure_stderr_pin['path'])
    require(pin(failure_stderr_path, cap=64 << 20) == failure_stderr_pin
            and failure_stderr_pin['sha256'] == BASE_FAILURE_STDERR_SHA,
            'exact measured baseline stderr pin')
    baseline_stderr = failure_stderr_path.read_bytes()
    require(len(baseline_stderr) == failure_stderr_pin['bytes']
            and hashlib.sha256(baseline_stderr).hexdigest() == BASE_FAILURE_STDERR_SHA
            and pin(failure_stderr_path, cap=64 << 20) == failure_stderr_pin,
            'measured baseline stderr changed during read')
    require(pin(Path(value['controller']['path'])) == value['controller'],
            'actual memory-bounds DAG producer controller body')
    row = audit_input['memory_bounds_dag_final_build']
    path = Path(row['path'])
    require(pin(path, cap=64 << 20) == row, 'actual final memory-bounds DAG Cargo stdout')
    raw = path.read_bytes()
    require(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256']
            and pin(path, cap=64 << 20) == row, 'memory-bounds DAG Cargo stdout changed')
    records = [json.loads(line) for line in raw.decode().splitlines() if line.startswith('{')]
    proof = memory_bounds_dag_contract(value, inputs, sources, records, manifest, previous,
                                base, base_sources, proposal, baseline_failure, baseline_stderr)
    return dict(complete=audit_input['memory_bounds_dag_complete'], input=audit_input['memory_bounds_dag_input'],
                sources=audit_input['memory_bounds_dag_sources'], final_build=audit_input['memory_bounds_dag_final_build'],
                source_lineage=value['source_lineage'], diagnostic_build=False, diagnostic_only=False,
                diagnostic_changes_admission=False, cfg_expansion_diagnostics=True,
                inherited_cfg_expansion_diagnostics=True, memory_bounds_dag_optimization=True,
                inherited_cfg_linear_fusion_optimization=True, cfg_linear_fusion_optimization=True,
                memory_bounds_dag_test_filter=MEMORY_BOUNDS_DAG_PREFIX,
                memory_bounds_dag_tests=list(MEMORY_BOUNDS_DAG_NAMES),
                cfg_compaction_optimization=True, inherited_cfg_compaction_optimization=True,
                cfg_compaction_test_filter=CFG_COMPACTION_PREFIX, cfg_compaction_tests=list(CFG_COMPACTION_NAMES),
                cfg_linear_fusion_test_filter=CFG_LINEAR_FUSION_PREFIX, cfg_linear_fusion_tests=list(CFG_LINEAR_FUSION_NAMES),
                static_failure_site_identified=True, actual_failure_block_count_observed=False,
                admission_changed=True, structural_capacity_expansion=False,
                inherited_capacity_expansion=True, structural_limits_changed=False,
                graph_analysis_optimization=True, inherited_graph_analysis_optimization=True,
                resource_admission_may_change=True, membership_lookup_optimization=True,
                inherited_membership_lookup_optimization=True,
                dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
                use_lookup_optimization=True,
                inherited_use_lookup_optimization=True, compiler_scratch_added=True,
                use_lookup_test_filter=USE_LOOKUP_PREFIX, use_lookup_tests=list(USE_LOOKUP_NAMES),
                census_test_filter=CENSUS_PREFIX, census_tests=list(CENSUS_NAMES),
                membership_test_filter=MEMBERSHIP_PREFIX, membership_tests=list(MEMBERSHIP_NAMES),
                semantic_predicates_changed=False, actual_failure_caller_identified=False,
                baseline_failure_caller_identified=True, baseline_failure_block_count_observed=True,
                baseline_memory_bounds_preflight_refusal=True,
                baseline_rendered_blocks=567, baseline_rendered_edges=1122,
                baseline_rendered_operations=2240, baseline_guard_candidates=552,
                baseline_intersection_work_upper_bound=9340170,
                baseline_memory_bounds_work_limit=8388608,
                baseline_runtime_work_exhaustion_observed=False, baseline_edge_verdict_observed=False,
                dag_test_filter=DAG_PREFIX, dag_tests=list(DAG_NAMES),
                graph_work_diagnostics=True, inherited_graph_work_diagnostics=True,
                graph_work_test_filter=GRAPH_WORK_PREFIX,
                graph_work_tests=list(GRAPH_WORK_NAMES),
                borrow_lookup_test_filter=BORROW_LOOKUP_PREFIX, borrow_lookup_tests=list(BORROW_LOOKUP_NAMES),
                cfg_expansion_test_filter=CFG_EXPANSION_PREFIX, cfg_expansion_tests=list(CFG_EXPANSION_NAMES),
                cfg_diagnostics_retained=True, capacity_limits=value['capacity_limits'],
                capacity_cohorts=value['capacity_cohorts'], **proof)


ALIAS_ROOT = E / 'guarded-mlp-atomic-load-alias-cpu-v228-v1'
ALIAS_CONTROLLER_SHA = 'd3dfe30c4dd2fd216d6d5527fae0ec573083d7c032651bfb368c7ab7156342d4'
ALIAS_INPUT_SHA = '7e15a68f3a7e0e6d521db06906af4f8bbbf9b538ebd05c071763de6a0a8740d4'
ALIAS_PROPOSAL_SHA = 'aedc4e4595c843c87586414da9fe120a5228e3655469e73ad878f0ff7a4d5a6d'
ALIAS_COMPLETE_SHA = '18925ee6216977c561523f84a1899db379fcaaff99a050cc750ef32befebb708'  # Bind only the actual successful alias CPU receipt.
ALIAS_BASE_SOURCES_SHA = '37355332ab519512f28ebe58f5d9f55f7c9774f8d14796db2519a5b5168f25ba'
ALIAS_FAILURE_ROOT = E / 'guarded-mlp-combined-state-lowering-v228-v3'
ALIAS_FAILURE_SHA = 'd072928318581d8f492d296dfa4cc4a1f42865fbd3a7d95fdac01dddac7e5ab0'
ALIAS_FAILURE_STDERR_SHA = '5d3adcd162d465f76cca68e5590494692be075905070ae9a9795d11e60fe9b75'
ALIAS_FAILURE_DIAGNOSTIC = (
    b'fe2o3 rustc extraction: production compilation compiler-module handoff failed: '
    b'compiler descriptor construction failed: production descriptor evidence has an internal '
    b'formal alias obligation not discharged by Rust ownership mismatch')
MEMORY_BOUNDS_DAG_TOOL_MANIFEST = E / 'guarded-mlp-memory-bounds-dag-compiler-tools-v228-v1/manifest.json'
MEMORY_BOUNDS_DAG_TOOL_MANIFEST_SHA = '3e9e05f830711ff4c93f2694683a66618e218fb2f792acc16d93c42a274c7949'
ALIAS_PREFIX = 'atomic_load_alias_'
ALIAS_NAMES = tuple(sorted(ALIAS_PREFIX + name for name in (
    'load_only_allocations_have_no_alias_requirements',
    'r2_readers_only_require_output_separation',
    'every_atomic_writer_retains_alias_requirements',
    'writes_dominate_loads_in_both_orders',
    'bounds_conflicts_and_receipt_kind_are_preserved',
    'ordinary_ranges_and_empty_accesses_are_unchanged',
    'unreachable_writers_do_not_pollute_readers',
    'malformed_atomics_and_unknown_extents_fail_closed',
    'opcode_and_metadata_remain_separate_from_receipt_authority',
)))
ALIAS_FILES = {
    'fe2o3/crates/fe2o3-kernel-ir/src/formal_memory_obligations.rs': False,
    'fe2o3/crates/fe2o3-kernel-ir/tests/formal_atomic_load_alias_v1.rs': True,
}
ALIAS_DESCRIPTOR_NAMES = (
    'compiler_descriptor::tests::atomic_slice_descriptor_does_not_discharge_aliases_from_shared_rust_ownership',
    'compiler_descriptor::tests::atomic_slice_descriptor_requires_exact_readwrite_u32_global_slice',
)
ALIAS_IR_TARGETS = (
    'fe2o3_kernel_ir', 'amdgpu_diagnostics', 'canonical_kir_v6', 'cast_verification',
    'checked_binary_v6', 'control_flow_bounds', 'device_constants_v2', 'effect_extraction',
    'float_operations', 'formal_memory_obligations', 'frozen_wire_compatibility',
    'g4_synchronization', 'inert_v12', 'inline_assembly', 'integer_switch',
    'interprocedural_effects', 'launch_kernel_v2', 'matrix_operations',
    'memory_intrinsics_v10', 'memory_safety_v2', 'operand_visitation',
    'pointer_access_cast_v11', 'region_effect_analysis', 'region_effect_model',
    'scalar_ops_v2', 'semantic_operations', 'standard_atomics', 'synchronization_v2',
    'terminator_operand_visitation', 'v12_effect_consumer_closure', 'vector_v12',
    'verification', 'verification_contract_v12', 'wave_operations', 'wave_operations_v9',
    'wire', 'wire_preflight_order', 'wire_v5', 'wire_v6', 'wire_v7', 'write_only_v9',
    'formal_atomic_load_alias_v1',
)
ALIAS_TRUE = (
    'atomic_load_alias_refinement', 'alias_predicates_changed', 'semantic_predicates_changed',
    'admission_changed', 'resource_admission_may_change', 'compiler_scratch_lifetime_changed',
    'inherited_graph_analysis_optimization', 'memory_bounds_dag_optimization',
    'inherited_memory_bounds_dag_optimization', 'cfg_linear_fusion_optimization',
    'inherited_cfg_linear_fusion_optimization', 'cfg_compaction_optimization',
    'inherited_cfg_compaction_optimization',
)
ALIAS_FALSE = (
    'diagnostic_build', 'diagnostic_only', 'compiler_scratch_added',
    'receipt_serialization_changed', 'structural_limits_changed',
    'graph_analysis_optimization', 'actual_failure_caller_identified',
    'actual_failure_block_count_observed', 'baseline_failure_caller_identified',
    'baseline_failure_block_count_observed', 'baseline_failure_root_observed',
    'baseline_alias_pair_observed',
)


def atomic_load_alias_contract(value, inputs, sources, records, manifest, previous,
                               base, base_sources, proposal, failure, stderr):
    require(value['schema'] == 'ferric-guarded-mlp-atomic-load-alias-cpu-v1'
            and value['passed'] is True and value['failure'] is None
            and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and all(value[key] is True for key in ALIAS_TRUE)
            and all(value[key] is False for key in ALIAS_FALSE)
            and value['baseline_descriptor_alias_refusal_observed'] is True
            and value['controller']['sha256'] == ALIAS_CONTROLLER_SHA
            and value['helper'] == dict(path=str(ALIAS_ROOT / 'qualification_helpers.py'),
                bytes=4777, sha256=MEMORY_BOUNDS_DAG_HELPER_SHA)
            and value['capacity_limits'] == CAPACITY_LIMITS
            and value['capacity_cohorts'] == CAPACITY_COHORTS
            and value['limits'] == base['limits']
            and value['final_compiler_product_phase'] == 'compiler-tests-build'
            and value['kernel_ir_historical_raw_census_available'] is False
            and value['focused_repeats_are_not_unique_tests'] is True
            and value['historical_product_pins_postchecked'] is False
            and all(value[key] is False for key in ('gpu_execution', 'guarded_candidate_hsaco_emitted',
                'numerical_acceptance', 'full_model_acceptance', 'performance_claim')),
            'successful semantic alias CPU generation required')
    require(set(inputs) == {'schema', 'files', 'tool_pins', 'lineage', 'metadata_relocations', 'rust_src'}
            and inputs['schema'] == 'ferric-guarded-mlp-atomic-load-alias-cpu-input-v1'
            and inputs['lineage'] == value['source_lineage']
            and inputs['tool_pins'] == value['tool_pins'] == base['tool_pins']
            and len(sources) == 5811
            and {name: compact(row) for name, row in sources.items()} == inputs['files']
            and sources['run_cpu.py'] == value['controller']
            and sources['qualification_helpers.py'] == value['helper']
            and all(row['path'] == str(ALIAS_ROOT / name) for name, row in sources.items()),
            'complete current alias source/controller/helper map')
    lineage = inputs['lineage']
    require(set(lineage) == {'base_complete', 'base_sources', 'base_input', 'base_metadata',
                'base_dependencies', 'base_streams', 'atomic_load_alias_proposal',
                'atomic_load_alias_overlay', 'baseline_failure', 'baseline_failure_stderr'}
            and lineage['base_complete']['sha256'] == MEMORY_BOUNDS_DAG_COMPLETE_SHA
            and lineage['base_sources']['sha256'] == ALIAS_BASE_SOURCES_SHA
            and lineage['base_input']['sha256'] == MEMORY_BOUNDS_DAG_INPUT_SHA
            and proposal['schema'] == 'ferric-guarded-mlp-atomic-load-alias-source-v1'
            and proposal['source_only'] is True
            and all(proposal[key] is True for key in ALIAS_TRUE)
            and all(proposal[key] is False for key in ALIAS_FALSE if key not in ('diagnostic_build', 'diagnostic_only'))
            and proposal['baseline_descriptor_alias_refusal_observed'] is True
            and proposal['base_complete'] == compact(lineage['base_complete'])
            and proposal['base_sources'] == compact(lineage['base_sources'])
            and proposal['base_input'] == compact(lineage['base_input'])
            and proposal['base_controller'] == compact(base['controller'])
            and proposal['actual_failure'] == compact(lineage['baseline_failure'])
            and proposal['actual_failure_stderr'] == compact(lineage['baseline_failure_stderr'])
            and proposal['base_source_count'] == 5808 and proposal['source_count'] == 5809
            and proposal['source_file_additions'] == 1 and proposal['new_test_count'] == 9
            and proposal['filter'] == value['atomic_load_alias_test_filter'] == ALIAS_PREFIX
            and proposal['test_names'] == value['atomic_load_alias_tests'] == list(ALIAS_NAMES)
            and proposal['test_target'] == 'formal_atomic_load_alias_v1'
            and proposal['descriptor_regression_test_names'] == value['descriptor_atomic_negative_tests']
                == list(ALIAS_DESCRIPTOR_NAMES), 'exact semantic alias proposal and direct DAG lineage')
    require(lineage['baseline_failure'] == dict(path=str(ALIAS_FAILURE_ROOT / 'failed.json'),
                bytes=19077, sha256=ALIAS_FAILURE_SHA)
            and lineage['baseline_failure_stderr'] == dict(path=str(ALIAS_FAILURE_ROOT / 'compile.stderr'),
                bytes=8480, sha256=ALIAS_FAILURE_STDERR_SHA)
            and failure['schema'] == 'ferric-guarded-mlp-combined-state-lowering-result-v1'
            and failure['passed'] is False and failure['source_unchanged'] is True
            and failure['postcheck_errors'] == [] and failure['artifact'] is None
            and failure['actual_failure_caller_identified'] is False
            and failure['actual_failure_block_count_observed'] is False
            and failure['raw']['compile.stderr'] == lineage['baseline_failure_stderr']
            and len(failure['phases']) == 1,
            'actual generic descriptor refusal, without attributed root or pair')
    phase = failure['phases'][0]
    require(phase['label'] == 'compile' and phase['exit_code'] == 1
            and phase['natural_exit'] is True and phase['reaped'] is True
            and phase['process_group_absent'] is True and phase['forced_cleanup'] is False
            and phase['timed_out'] is False and phase['exception'] is None
            and phase['observed_signals'] == [] and phase['stderr'] == lineage['baseline_failure_stderr']
            and [line for line in stderr.splitlines() if line.startswith(b'fe2o3 rustc extraction:')]
                == [ALIAS_FAILURE_DIAGNOSTIC], 'exact clean descriptor alias failure')
    expected = {name: compact(row) for name, row in base_sources.items() if name.startswith('fe2o3/')}
    overlay = {'fe2o3/' + name: row for name, row in proposal['files'].items()}
    require(len(expected) == 5808 and set(overlay) == set(ALIAS_FILES)
            and lineage['atomic_load_alias_overlay'] == overlay, 'closed two-file alias overlay')
    for name, row in overlay.items():
        require(set(row) == {'before', 'after'} and row['before'] == expected.get(name)
                and (name not in expected) is ALIAS_FILES[name]
                and set(row['after']) == {'bytes', 'sha256'} and row['after'] != row['before'],
                'exact alias preimage/addition')
        expected[name] = row['after']
    require(len(expected) == 5809 and expected == {name: compact(row) for name, row in sources.items()
            if name.startswith('fe2o3/')}, 'no source changes outside the reviewed alias overlay')
    inherited_keys = ['compiler_cohorts', 'cfg_diagnostic_tests']
    for prefix in ('memory_bounds_dag', 'cfg_linear_fusion', 'cfg_compaction', 'cfg_expansion',
                   'borrow_lookup', 'use_lookup', 'census', 'membership', 'graph_work', 'dag'):
        inherited_keys += [prefix + '_test_filter', prefix + '_tests']
    require(all(value[key] == base[key] for key in inherited_keys),
            'unchanged inherited cohort/filter metadata')
    roles = ['kernel-ir-' + name for name in ALIAS_IR_TARGETS]
    labels = ['rustc-version', 'alias-rustfmt-check', 'metadata', 'kernel-ir-build-tests']
    labels += [role + suffix for role in roles for suffix in ('-list', '-ignored', '-tests')]
    labels += ['atomic-load-alias']
    for phase in base['phases'][2:]:
        labels.append(phase['label'])
        if phase['label'] == 'compiler-tests':
            labels += ['descriptor-atomic-negative-0', 'descriptor-atomic-negative-1']
    require(len(labels) == len(set(labels)) == 176
            and [row['label'] for row in value['phases']] == labels
            and all(row['exit_code'] == 0 and row['natural_exit'] is True and row['reaped'] is True
                and row['process_group_absent'] is True and row['forced_cleanup'] is False
                and row['timed_out'] is False and row['exception'] is None and row['storage_failure'] is None
                for row in value['phases']), 'all 176 exact clean qualification phases')
    inventories, ignored, tests = value['test_inventories'], value['ignored_inventories'], value['tests']
    require(value['kernel_ir_target_names'] == sorted(ALIAS_IR_TARGETS)
            and set(inventories) == set(ignored) == set(base['test_inventories']) | set(roles)
            and all(inventories[role] == base['test_inventories'][role]
                and ignored[role] == base['ignored_inventories'][role] for role in base['test_inventories'])
            and set(tests) == set(base['tests']) | set(roles)
                | {'atomic-load-alias', 'descriptor-atomic-negative-0', 'descriptor-atomic-negative-1'}
            and len(tests) == 77 and all(tests[name] == result for name, result in base['tests'].items()),
            'unchanged historical names/outcomes plus exact fresh IR scopes')
    for role in roles:
        names, skip = inventories[role], ignored[role]
        require(names and names == sorted(set(names)) and skip == sorted(set(skip))
                and set(skip) <= set(names)
                and tests[role] == dict(names=names, passed=len(names)-len(skip), failed=0,
                    ignored=len(skip), filtered_out=0, ignored_names=skip,
                    named_outcomes={name: 'ignored' if name in skip else 'ok' for name in names}),
                'complete dynamic named IR census: ' + role)
    alias_role = 'kernel-ir-formal_atomic_load_alias_v1'
    require(inventories[alias_role] == list(ALIAS_NAMES) and ignored[alias_role] == []
            and tests['atomic-load-alias'] == dict(names=list(ALIAS_NAMES), passed=9, failed=0,
                ignored=0, filtered_out=0), 'nine exact nonignored alias cases and repeat')
    for index, name in enumerate(ALIAS_DESCRIPTOR_NAMES):
        require(name in inventories['compiler-lib'] and name not in ignored['compiler-lib']
                and tests['descriptor-atomic-negative-' + str(index)] == dict(names=[name],
                    passed=1, failed=0, ignored=0, filtered_out=len(inventories['compiler-lib'])-1),
                'unchanged descriptor rejection test repeated exactly')
    ir_passed = sum(tests[role]['passed'] for role in roles)
    ir_ignored = sum(tests[role]['ignored'] for role in roles)
    require(value['kernel_ir_tests_passed'] == ir_passed and value['kernel_ir_tests_ignored'] == ir_ignored
            and value['tests_passed'] == sum(row['passed'] for row in tests.values()) == 3001 + ir_passed
            and value['tests_ignored'] == sum(row['ignored'] for row in tests.values()) == 25 + ir_ignored,
            'measured dynamic aggregate, not an inherited IR census')
    artifacts = value['artifacts']
    require(set(artifacts) == set(base['artifacts']) | set(roles) and len(artifacts) == 49
            and [row.get('success') for row in records if row.get('reason') == 'build-finished'] == [True],
            'exact final artifact roles and Cargo success')
    package = ALIAS_ROOT / 'fe2o3/crates/rustc-codegen-fe2o3'
    for role, name, kind, filename, source in (
            ('backend', 'rustc_codegen_fe2o3', ['dylib', 'rlib'], 'librustc_codegen_fe2o3.so', 'src/lib.rs'),
            ('backend-rlib', 'rustc_codegen_fe2o3', ['dylib', 'rlib'], 'librustc_codegen_fe2o3.rlib', 'src/lib.rs'),
            ('extractor', 'fe2o3-rustc-extract', ['bin'], 'fe2o3-rustc-extract', 'src/bin/fe2o3-rustc-extract.rs')):
        selected = artifacts[role]
        matches = [row for row in records if row.get('reason') == 'compiler-artifact'
            and row.get('manifest_path') == str(package / 'Cargo.toml')
            and row.get('target', {}).get('name') == name
            and sorted(row['target'].get('kind', [])) == kind
            and sorted(row['target'].get('crate_types', [])) == kind
            and row['target'].get('src_path') == str(package / source)
            and row.get('profile', {}).get('test') is False]
        path = Path(selected['pin']['path'])
        require(len(matches) == 1 and selected['cargo_artifact'] == matches[0]
                and path.is_absolute() and '..' not in path.parts
                and path.is_relative_to(ALIAS_ROOT / 'target') and path.name == filename
                and selected['pin']['path'] in matches[0]['filenames']
                and matches[0]['executable'] == (str(path) if role == 'extractor' else None),
                'current final Cargo provenance: ' + role)
    require(artifacts['backend']['cargo_artifact'] == artifacts['backend-rlib']['cargo_artifact']
            and set(manifest) == set(previous) == {'bin/' + name for name in NAMES},
            'paired final library provenance and seven-tool manifest')
    for name in NAMES:
        row = manifest['bin/' + name]
        require(set(row) == {'source', 'bytes', 'sha256'}, 'closed current tool row')
        role = {'fe2o3-rustc-extract': 'extractor', 'librustc_codegen_fe2o3.so': 'backend'}.get(name)
        require(row == (dict(source=artifacts[role]['pin']['path'], **compact(artifacts[role]['pin']))
                        if role else previous['bin/' + name]), 'exact current/unchanged tool identity')
    return dict(rlib_cpu_provenance=artifacts['backend-rlib'],
        test_elf_cpu_provenance={role: artifacts[role] for role in
            ('pliron-lib', 'compiler-lib', 'atomic-extraction', 'matrix-extraction')},
        kernel_ir_test_elf_cpu_provenance={role: artifacts[role] for role in roles},
        omitted_artifact_bodies_rehashed=False)


def qualified_atomic_load_alias_producer(audit_input, manifest, previous):
    for key, relative in (('atomic_load_alias_complete', 'evidence/complete.json'),
                         ('atomic_load_alias_input', 'input-manifest.json'),
                         ('atomic_load_alias_sources', 'evidence/sources-before.json'),
                         ('atomic_load_alias_final_build', 'evidence/compiler-tests-build.stdout')):
        require(audit_input[key]['path'] == str(ALIAS_ROOT / relative), 'exact current alias readset path')
    value = read_json(audit_input['atomic_load_alias_complete'], ALIAS_COMPLETE_SHA)
    require(value['input_manifest'] == audit_input['atomic_load_alias_input']
            and value['input_sources'] == audit_input['atomic_load_alias_sources']
            and compact(value['final_sources']) == compact(audit_input['atomic_load_alias_sources'])
            and value['raw']['compiler-tests-build.stdout'] == audit_input['atomic_load_alias_final_build'],
            'actual current CPU receipt/readset join')
    inputs = read_json(audit_input['atomic_load_alias_input'], ALIAS_INPUT_SHA)
    sources = read_json(audit_input['atomic_load_alias_sources'])
    lineage = inputs['lineage']
    base = read_json(lineage['base_complete'], MEMORY_BOUNDS_DAG_COMPLETE_SHA)
    base_sources = read_json(lineage['base_sources'], ALIAS_BASE_SOURCES_SHA)
    base_input = read_json(lineage['base_input'], MEMORY_BOUNDS_DAG_INPUT_SHA)
    require(lineage['base_complete'] == audit_input['memory_bounds_dag_complete']
            and compact(lineage['base_sources']) == compact(audit_input['memory_bounds_dag_sources'])
            and lineage['base_input'] == audit_input['memory_bounds_dag_input']
            and base_input['files'] == {name: compact(row) for name, row in base_sources.items()}
            and inputs['tool_pins'] == base_input['tool_pins']
            and compact(inputs['rust_src']) == compact(base_input['rust_src'])
                == compact(base['rust_src']) == compact(value['rust_src']), 'direct qualified DAG parent joins')
    proposal = read_json(lineage['atomic_load_alias_proposal'], ALIAS_PROPOSAL_SHA)
    failure = read_json(lineage['baseline_failure'], ALIAS_FAILURE_SHA)
    raw_inputs = []
    for row, digest in ((lineage['baseline_failure_stderr'], ALIAS_FAILURE_STDERR_SHA),
                        (audit_input['atomic_load_alias_final_build'], None)):
        path = Path(row['path'])
        require(pin(path, cap=64 << 20) == row and (digest is None or row['sha256'] == digest),
                'exact alias observed raw input')
        body = path.read_bytes()
        require(compact(row) == dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())
                and pin(path, cap=64 << 20) == row, 'alias raw input changed while reading')
        raw_inputs.append(body)
    require(pin(Path(value['controller']['path'])) == value['controller']
            and pin(Path(value['helper']['path'])) == value['helper'], 'current controller/helper body')
    records = [json.loads(line) for line in raw_inputs[1].decode().splitlines() if line.startswith('{')]
    proof = atomic_load_alias_contract(value, inputs, sources, records, manifest, previous,
                                      base, base_sources, proposal, failure, raw_inputs[0])
    return dict(complete=audit_input['atomic_load_alias_complete'], input=audit_input['atomic_load_alias_input'],
        sources=audit_input['atomic_load_alias_sources'], final_build=audit_input['atomic_load_alias_final_build'],
        source_lineage=value['source_lineage'],
        **{key: True for key in ALIAS_TRUE}, **{key: False for key in ALIAS_FALSE},
        baseline_descriptor_alias_refusal_observed=True,
        atomic_load_alias_test_filter=ALIAS_PREFIX, atomic_load_alias_tests=list(ALIAS_NAMES),
        descriptor_atomic_negative_tests=list(ALIAS_DESCRIPTOR_NAMES),
        kernel_ir_target_names=value['kernel_ir_target_names'],
        kernel_ir_tests_passed=value['kernel_ir_tests_passed'], kernel_ir_tests_ignored=value['kernel_ir_tests_ignored'],
        capacity_limits=value['capacity_limits'], capacity_cohorts=value['capacity_cohorts'], **proof)


def main():
    started = time.monotonic()
    hard_deadline = started + WHOLE_SECONDS
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]),
            'python3 -B audit_guarded_mlp_atomic_load_alias_compiler_tools_v228_v1.py ACTUAL_INPUT_SHA')
    require(all(type(value) is str and re.fullmatch(r'[0-9a-f]{64}', value)
                for value in (MEMORY_BOUNDS_DAG_CONTROLLER_SHA, MEMORY_BOUNDS_DAG_PROPOSAL_SHA, MEMORY_BOUNDS_DAG_INPUT_SHA, MEMORY_BOUNDS_DAG_COMPLETE_SHA, MEMORY_BOUNDS_DAG_HELPER_SHA)),
            'actual memory-bounds DAG producer bindings remain pending')
    require(all(type(value) is str and re.fullmatch(r'[0-9a-f]{64}', value)
                for value in (ALIAS_CONTROLLER_SHA, ALIAS_INPUT_SHA, ALIAS_PROPOSAL_SHA, ALIAS_COMPLETE_SHA)),
            'actual atomic-load alias producer bindings remain pending')
    require(type(DRIVER_CONTROLLER_SHA) is str and re.fullmatch(r'[0-9a-f]{64}', DRIVER_CONTROLLER_SHA),
            'reviewed driver producer controller binding is pending')
    require(Path(__file__).resolve() == SCRIPT and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'unexpected controller host/path/UID')
    require(E.resolve(strict=True) == E and TOOLS.resolve(strict=True) == TOOLS
            and not os.path.lexists(OUT), 'canonical inputs and fresh output required')
    require(shutil.disk_usage(E).free >= FREE_FLOOR, '40 GiB setup storage floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected nice value')
    if priority == 0:
        os.nice(10)
    require(os.sched_getaffinity(0) == {8, 9}
            and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'CPU affinity/nice mismatch')
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, AS_LIMIT),
                      (resource.RLIMIT_FSIZE, STREAM_LIMIT)):
        soft, hard = resource.getrlimit(kind)
        effective = min([cap] + [x for x in (soft, hard) if x != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (effective, effective))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup')
    OUT.mkdir(mode=0o700)
    for sig in SIGNALS:
        signal.signal(sig, interrupted)
    signal.setitimer(signal.ITIMER_REAL, WHOLE_SECONDS - CLEANUP_RESERVE)
    env = dict(HOME='/home/harmenon', PATH='/usr/bin:/bin', LANG='C', LC_ALL='C', TZ='UTC',
               LD_LIBRARY_PATH=str(TOOLS / 'bin') + ':' + str(NIGHTLY_LIB), ROCR_VISIBLE_DEVICES='',
               HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
    leaves, audits, manifest, tool_rows, audit_tools = [], {}, None, {}, {}
    failure, postcheck_errors = None, []
    audit_input_pin, manifest_pin, producer, driver, previous_pin = None, None, None, None, None
    baseline_producer, driver_manifest_pin = None, None
    historical_dag, historical_dag_manifest_pin = None, None
    try:
        script_pin = pin(SCRIPT)
        audit_input_pin = pin(AUDIT_INPUT, cap=64 << 10)
        audit_input = read_json(audit_input_pin, sys.argv[1])
        require(set(audit_input) == {'schema', 'controller', 'tool_manifest', 'old_tool_manifest',
                    'producer_complete', 'producer_input', 'producer_sources', 'producer_final_build',
                    'previous_tool_manifest', 'driver_complete', 'driver_input', 'driver_sources',
                    'driver_final_build', 'driver_test_build', 'driver_tool_manifest',
                    'memory_bounds_dag_complete', 'memory_bounds_dag_input', 'memory_bounds_dag_sources',
                    'memory_bounds_dag_final_build', 'memory_bounds_dag_tool_manifest',
                    'atomic_load_alias_complete', 'atomic_load_alias_input',
                    'atomic_load_alias_sources', 'atomic_load_alias_final_build'}
                and audit_input['schema'] == 'ferric-guarded-mlp-atomic-load-alias-tool-audit-input-v1'
                and audit_input['controller'] == script_pin,
                'closed caller-pinned audit admission')
        audit_tools = {name: resolved('/usr/bin/' + name) for name in ('readelf', 'ldd', 'prlimit')}
        audit_tools['bash'] = resolved('/bin/bash')
        require(NIGHTLY_LIB.resolve(strict=True) == NIGHTLY_LIB, 'canonical nightly lib directory')
        for name, (size, sha) in NIGHTLY.items():
            value = pin(NIGHTLY_LIB / name)
            require((value['bytes'], value['sha256']) == (size, sha), 'nightly compiler library identity')
        manifest_pin = pin(TOOLS / 'manifest.json', cap=64 << 10)
        require(manifest_pin == audit_input['tool_manifest'], 'actual new seven-tool manifest identity')
        manifest = read_json(manifest_pin)
        previous_pin = audit_input['previous_tool_manifest']
        previous = read_json(previous_pin, PREVIOUS_MANIFEST_SHA)
        require(audit_input['producer_complete']['sha256'] == BASE_COMPLETE_SHA
                and audit_input['driver_complete']['sha256'] == DRIVER_COMPLETE_SHA,
                'exact actual baseline and unchanged focused driver receipts')
        baseline_producer = qualified_producer(audit_input, previous)
        driver_manifest_pin = audit_input['driver_tool_manifest']
        driver_manifest = read_json(driver_manifest_pin, DRIVER_MANIFEST_SHA)
        driver = qualified_driver(audit_input, driver_manifest, previous)
        historical_dag_manifest_pin = audit_input['memory_bounds_dag_tool_manifest']
        require(historical_dag_manifest_pin['path'] == str(MEMORY_BOUNDS_DAG_TOOL_MANIFEST),
                'explicit historical DAG deployment manifest path')
        historical_manifest = read_json(historical_dag_manifest_pin, MEMORY_BOUNDS_DAG_TOOL_MANIFEST_SHA)
        historical_dag = qualified_memory_bounds_dag_producer(audit_input, historical_manifest, driver_manifest)
        producer = qualified_atomic_load_alias_producer(audit_input, manifest, historical_manifest)
        require(set(manifest) == {'bin/' + name for name in NAMES}, 'exact seven-tool roster')
        require({p.name for p in TOOLS.iterdir()} == {'bin', 'manifest.json'}
                and {p.name for p in (TOOLS / 'bin').iterdir()} == set(NAMES), 'closed transported tree')
        for name in NAMES:
            original = manifest['bin/' + name]
            require(set(original) == {'source', 'bytes', 'sha256'}
                    and Path(original['source']).is_absolute(), 'original tool record shape')
            path = TOOLS / 'bin' / name
            value = pin(path)
            require((value['bytes'], value['sha256']) == (original['bytes'], original['sha256']),
                    'transported tool identity differs')
            require(name == 'librustc_codegen_fe2o3.so' or os.access(path, os.X_OK), 'tool executable mode')
            with path.open('rb') as stream:
                require(stream.read(4) == b'\x7fELF', 'tool is not an ELF object')
            tool_rows[name] = dict(original=original, deployed=value)
        save('inputs-before.json', INPUTS)
        for name in NAMES:
            path = str(TOOLS / 'bin' / name)
            dynamic = owned(name + '-readelf', ['/usr/bin/readelf', '-l', '-d', path], env, leaves, hard_deadline)
            linkage = owned(name + '-ldd', ['/usr/bin/ldd', path], env, leaves, hard_deadline)
            audits[name] = libraries(dynamic, linkage, name == 'librustc_codegen_fe2o3.so')
            if name == 'fe2o3-rustc-extract':
                require(audits[name]['libraries']['librustc_codegen_fe2o3.so']['pin']
                        == tool_rows['librustc_codegen_fe2o3.so']['deployed'],
                        'extractor must resolve the exact deployed backend')
        require(len(leaves) == 14 and set(audits) == set(NAMES), 'fourteen successful audit leaves required')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    for name, expected in list(INPUTS.items()):
        if time.monotonic() >= hard_deadline:
            postcheck_errors.append('whole deadline reached before completing input postchecks')
            break
        try:
            signal.setitimer(signal.ITIMER_REAL, max(0.001, hard_deadline - time.monotonic()))
            require(pin(Path(name), track=False) == expected, 'input/provider posthash drift')
        except BaseException as error:
            postcheck_errors.append(name + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    for name, expected in ALIASES.items():
        try:
            require(str(Path(name).resolve(strict=True)) == expected, 'reported alias changed')
        except BaseException as error:
            postcheck_errors.append(name + ': ' + repr(error))
    try:
        require({p.name for p in TOOLS.iterdir()} == {'bin', 'manifest.json'}
                and {p.name for p in (TOOLS / 'bin').iterdir()} == set(NAMES),
                'transported tree roster changed')
    except BaseException as error:
        postcheck_errors.append('tool roster: ' + repr(error))
    if postcheck_errors:
        failure = failure or 'input/provider postcheck failed'
    if time.monotonic() >= hard_deadline:
        failure = failure or 'whole audit deadline exceeded'
    if shutil.disk_usage(E).free < FREE_FLOOR:
        failure = failure or '40 GiB final storage floor'
    save('inputs-after.json', dict(inputs=INPUTS, resolved_paths=ALIASES, errors=postcheck_errors))
    result = dict(schema='ferric-guarded-mlp-atomic-load-alias-compiler-tool-audit-v1', passed=failure is None,
                  failure=failure, postcheck_errors=postcheck_errors, tool_manifest=manifest_pin,
                  input_manifest=audit_input_pin, qualified_producer=producer,
                  previous_tool_manifest=previous_pin, qualified_driver=driver,
                  driver_tool_manifest=driver_manifest_pin,
                  qualified_baseline_producer=baseline_producer,
                  qualified_memory_bounds_dag_producer=historical_dag,
                  memory_bounds_dag_tool_manifest=historical_dag_manifest_pin,
                  atomic_load_alias_refinement=True, alias_predicates_changed=True,
                  compiler_scratch_lifetime_changed=True, receipt_serialization_changed=False,
                  inherited_memory_bounds_dag_optimization=True,
                  atomic_load_alias_test_filter=ALIAS_PREFIX, atomic_load_alias_tests=list(ALIAS_NAMES),
                  descriptor_atomic_negative_tests=list(ALIAS_DESCRIPTOR_NAMES),
                  kernel_ir_target_names=sorted(ALIAS_IR_TARGETS),
                  kernel_ir_tests_passed=producer['kernel_ir_tests_passed'] if producer is not None else None,
                  kernel_ir_tests_ignored=producer['kernel_ir_tests_ignored'] if producer is not None else None,
                  diagnostic_build=False, diagnostic_only=False,
                diagnostic_changes_admission=False, cfg_expansion_diagnostics=True,
                inherited_cfg_expansion_diagnostics=True, memory_bounds_dag_optimization=True,
                inherited_cfg_linear_fusion_optimization=True, cfg_linear_fusion_optimization=True,
                memory_bounds_dag_test_filter=MEMORY_BOUNDS_DAG_PREFIX,
                memory_bounds_dag_tests=list(MEMORY_BOUNDS_DAG_NAMES),
                cfg_compaction_optimization=True, inherited_cfg_compaction_optimization=True,
                cfg_compaction_test_filter=CFG_COMPACTION_PREFIX, cfg_compaction_tests=list(CFG_COMPACTION_NAMES),
                cfg_linear_fusion_test_filter=CFG_LINEAR_FUSION_PREFIX, cfg_linear_fusion_tests=list(CFG_LINEAR_FUSION_NAMES),
                static_failure_site_identified=True, actual_failure_block_count_observed=False,
                  admission_changed=True, structural_capacity_expansion=False,
                  inherited_capacity_expansion=True, structural_limits_changed=False,
                  graph_analysis_optimization=False, inherited_graph_analysis_optimization=True,
                  resource_admission_may_change=True, membership_lookup_optimization=True,
                  inherited_membership_lookup_optimization=True,
                  dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                  borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
                  use_lookup_optimization=True,
                  inherited_use_lookup_optimization=True, compiler_scratch_added=False,
                  use_lookup_test_filter=USE_LOOKUP_PREFIX, use_lookup_tests=list(USE_LOOKUP_NAMES),
                  census_test_filter=CENSUS_PREFIX, census_tests=list(CENSUS_NAMES),
                  membership_test_filter=MEMBERSHIP_PREFIX, membership_tests=list(MEMBERSHIP_NAMES),
                  semantic_predicates_changed=True, actual_failure_caller_identified=False,
                  baseline_failure_caller_identified=False,
                  baseline_failure_block_count_observed=False,
                  baseline_descriptor_alias_refusal_observed=producer is not None,
                  baseline_failure_root_observed=False, baseline_alias_pair_observed=False,
                  dag_test_filter=DAG_PREFIX, dag_tests=list(DAG_NAMES),
                  graph_work_diagnostics=True, inherited_graph_work_diagnostics=True,
                graph_work_test_filter=GRAPH_WORK_PREFIX,
                  graph_work_tests=list(GRAPH_WORK_NAMES),
                borrow_lookup_test_filter=BORROW_LOOKUP_PREFIX, borrow_lookup_tests=list(BORROW_LOOKUP_NAMES),
                cfg_expansion_test_filter=CFG_EXPANSION_PREFIX, cfg_expansion_tests=list(CFG_EXPANSION_NAMES),
                  cfg_diagnostics_retained=True, capacity_limits=CAPACITY_LIMITS,
                  driver_receipt_source_and_final_cargo_joins_replayed=driver is not None,
                  producer_receipt_source_and_final_cargo_joins_replayed=producer is not None,
                  rlib_deployed=False, rlib_readelf_or_ldd_invoked=False,
                  tools=tool_rows, audit_tools=audit_tools, phases=leaves, audits=audits,
                  inputs=INPUTS, resolved_paths=ALIASES, environment=env,
                  host=os.uname().nodename, boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                  elapsed_seconds=time.monotonic() - started,
                  limits=dict(whole_wall_seconds=WHOLE_SECONDS, cleanup_reserve_seconds=CLEANUP_RESERVE,
                              leaf_wall_seconds=30, leaf_cpu_seconds=30, address_space_bytes=AS_LIMIT,
                              stream_bytes=STREAM_LIMIT, free_bytes=FREE_FLOOR, affinity=[8, 9], nice=10),
                  raw={p.name: pin(p, False) for p in sorted(OUT.iterdir()) if p.is_file()},
                  actual_library_audits_replayed=failure is None, tool_execution_limited_to_readelf_ldd=True,
                  compiler_invocation=False, vendor_created=False, gpu_execution=False,
                  production_authority=False, load_authority=False, launch_authority=False)
    receipt = save('complete.json' if failure is None else 'failed.json', result)
    print(json.dumps(receipt), flush=True)
    if failure is not None:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
