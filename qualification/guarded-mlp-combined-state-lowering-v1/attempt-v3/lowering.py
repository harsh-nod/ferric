"""One bounded checked generic lowering; no runtime loading or GPU execution."""
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

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
CPU = E / 'guarded-mlp-combined-state-cpu-v228-v3'
TOOLS = E / 'guarded-mlp-memory-bounds-dag-compiler-tools-v228-v1'
AUDIT = E / 'guarded-mlp-memory-bounds-dag-compiler-tools-audit-v228-v1'
VENDOR_ROOT = E / 'guarded-mlp-combined-state-vendor-v228-v3'
VENDOR = VENDOR_ROOT / 'vendor'
OUT = E / 'guarded-mlp-combined-state-lowering-v228-v3'
SCRATCH = OUT / 'scratch'
SCRIPT = E / 'lower_guarded_mlp_combined_state_v228_v3.py'
TOOLCHAIN = Path('/home/harmenon/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu')
RUST_SOURCE = TOOLCHAIN / 'lib/rustlib/src/rust'
CPU_SHA = 'ebdcb5f36dcdadd77614f85b91f0ed510d1ea6a44be407ab18d73a1aa56af696'  # Bind only the actual successful combined-state CPU receipt.
HELPER_SHA = 'd59335b65cbde4b04ac003ad56e08c0f72fe58b3cf4bf4cfe65d88f8134970e3'  # Bind only the reviewed combined-state CPU controller.
AUDIT_SHA = 'bbcfc8228c4b8a34bfe1da740662bbee8fe81c878f8bfae5d1be0be699208d76'  # Requires actual successful fresh memory-bounds DAG loader receipt.
VENDOR_CONTROLLER_SHA = 'a8d77b106b89341f566af7b46a2d2dc2188b0fc537f9707e8bb812a229d38841'  # Bind only the reviewed combined-state vendor controller.
AUDIT_CONTROLLER_SHA = 'fe191e3cbc75dc584d41b1a4c725280a1c25ef9f9205b8e065b3d0470e00f4de'  # Bind the reviewed memory-bounds DAG loader source.
DRIVER_CONTROLLER_SHA = '8b72ba779a3df9125ecfd9170ce8964eefccd1b83f2e595fe09d21a2e44f50f5'
DRIVER = E / 'guarded-mlp-driver-cpu-v228-v1'
DRIVER_GENERATION = '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'
PREVIOUS_MANIFEST_SHA = '4d0b2f6220056573e2683e9057f7e716f6fcdf6ef35d70d30d9bfc212f2ca9f6'
PRODUCER_CONTROLLER_SHA = '151f0a91534ddf5aa7cec746545dce7a3eda0aae1764b917907075bfeed9b119'
PRODUCER_HELPER_SHA = 'ba127f1057546c0ce6e57fb78832c3e774c1108a4f7c5a2421f4aad7f51108be'
PRODUCER_HELPER_BYTES = 4777
PRODUCER = E / 'guarded-mlp-memory-bounds-dag-cpu-v228-v1'
MEMORY_BOUNDS_DAG_PROPOSAL_SHA = '7ebe693afa0b672c8e80c0d6c08fe9f7c6a8b0d6bcd478c393b8408dd890eeb8'
MEMORY_BOUNDS_DAG_BASE_COMPLETE_SHA = '7827d969f295a8a0ba709629b2679b71ecde9b93b1f0ad11c5b8be70f7259915'
MEMORY_BOUNDS_DAG_BASE_SOURCES_SHA = '0c5a0dce518d1d469d0f7cb34618245aedb74d9279a4aa50473f2f3e20a1de23'
MEMORY_BOUNDS_DAG_BASE_INPUT_SHA = '0bb52fac77acd6af93609e214a6117af4047c484547a9728a4f1f3f387da3981'
BASE_COMPLETE_SHA = 'e30712ef17c7094efce9bd13bee2bb172823e94f4817c812cfb8b1b20cefb847'
DRIVER_MANIFEST_SHA = 'b38ef216900209ea59fbd53a1cb571447f983eb8984ca4a7ce57c1fb10f521fb'
DRIVER_COMPLETE_SHA = 'a869f0f4aa0572c82492cb3d90bdb0dc9bb2625a7b441ff1f3e9a876a87079d9'
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

CFG_EXPANSION_PREFIX = 'production_ranked_projection_v1::tests::cfg_projected_block_limit_diagnostic_'
CFG_EXPANSION_NAMES = tuple(sorted(CFG_EXPANSION_PREFIX + name for name in (
    'context_is_bounded',
    'exact_and_next_boundaries',
    'expansion_counts_and_identity',
)))

PRODUCER_INPUT_SHA = '3c60b907c38c1b61d645ec303b6099843e28e8b46148920f7486a364f0faba12'

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

WORKER_BUILD = 'fe2o3-worker-v1-sha256-5e3a2d855dee71b322cb49e452e06a08f4603d866a8fdce246fb9ffa583dc2ed'
LLVM_BUILD = 'rocm7.2.1-packages-sha256:eb02c62693d6697017195f0abf5ebcf7e58f60e4d2acf8356de2e944bceec540'
CRATE = 'ferric_qwen3_tp_guarded_mlp_segment_kernels_device_v2'
KERNELS = ['ferric_qwen3_mlp_state_guard_v2',
           'ferric_qwen3_tp2_guarded_projection_residual_bf16_v2']
NAMES = ('cargo-fe2o3', 'clang-22', 'fe2o3-engineering-lld-proxy',
         'fe2o3-llvm-link-worker', 'fe2o3-rustc-extract', 'librustc_codegen_fe2o3.so', 'lld')
SIGNALS = (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM)
WHOLE_SECONDS, LEAF_SECONDS, CLEANUP_RESERVE = 720, 600, 50
AS_LIMIT, FILE_LIMIT, CACHE_LIMIT, STREAM_LIMIT = 12 << 30, 1 << 30, 6 << 30, 64 << 20
START_FREE, LIVE_FREE = 40 << 30, 38 << 30
INPUTS = {}


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

BASE_FAILURE_ROOT = E / 'guarded-mlp-ranked-cfg-linear-fusion-lowering-v228-v3'
BASE_FAILURE_SHA = '8d4cc1a0cfe0c9104282c3d20349638a616bd98e7e28f92598dcaf0110057ca2'
BASE_FAILURE_STDERR_SHA = 'c60dc2edc62bfe11ba7a8f79ce1465322d2c887aeb3fa5661dae8002a33c5095'
BASE_FAILURE_DIAGNOSTIC = (
    b'fe2o3 rustc extraction: production compilation general kernel verification failed: '
    b'production analysis resource limit exceeded [memory-bounds]: '
    b'memory-bounds work hard limit')

def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pairs(items):
    value = {}
    for key, item in items:
        require(key not in value, 'duplicate JSON key')
        value[key] = item
    return value


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
            value.st_uid, value.st_gid, value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def read(path, cap=64 << 20):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'noncanonical input')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= cap, 'bounded ordinary body required')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC), 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'body changed before read')
        body = stream.read()
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'body changed during read')
    require(stamp(path.lstat()) == stamp(before), 'body changed after read')
    return body


def helper():
    path = CPU / 'run_cpu.py'
    body = read(path, 1 << 20)
    require(hashlib.sha256(body).hexdigest() == HELPER_SHA, 'CPU helper generation changed')
    module = types.ModuleType('guarded_lowering_cpu_helpers')
    module.__file__ = str(path)
    exec(compile(body, str(path), 'exec'), module.__dict__)
    return module


def record(h, path, expected=None):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical pinned input required')
    value = h.pin(path)
    require(expected is None or value == expected or value['sha256'] == expected, 'input identity differs')
    require(str(path) not in INPUTS or INPUTS[str(path)] == value, 'conflicting input identity')
    INPUTS[str(path)] = value
    return value


def doc(h, path, expected=None):
    value = record(h, path, expected)
    body = read(path)
    require(len(body) == value['bytes'] and hashlib.sha256(body).hexdigest() == value['sha256'],
            'document changed after pin')
    return json.loads(body, object_pairs_hook=pairs)


def tree(h, root, cap=200000):
    require(root.is_dir() and root.resolve(strict=True) == root and not root.is_symlink(),
            'canonical tree required')
    result = {}
    for directory, dirs, names in os.walk(root, followlinks=False,
                                          onerror=lambda error: (_ for _ in ()).throw(error)):
        require(all(not (Path(directory) / name).is_symlink() for name in dirs), 'directory alias')
        for name in names:
            require(len(result) < cap, 'bounded tree roster')
            path = Path(directory) / name
            require(not path.is_symlink(), 'tree file alias')
            result[str(path.relative_to(root))] = h.pin(path)
    return result


def natural(row):
    require(row['natural_exit'] is True and row['exit_code'] == 0 and row['reaped'] is True
            and row['process_group_absent'] is True and row['forced_cleanup'] is False
            and row['timed_out'] is False and row['exception'] is None,
            'natural successful/reaped leaf required')


def raw_replay(h, value, directory):
    for name, item in value['raw'].items():
        require(Path(name).name == name and Path(item['path']) == directory / name, 'raw namespace')
        record(h, directory / name, item)
    for row in value['phases']:
        natural(row)
        name = row['label']
        require(doc(h, directory / (name + '.result.json')) == row, 'leaf/result join')
        for key in ('command', 'stdout', 'stderr'):
            require(Path(row[key]['path']).parent == directory, 'leaf body namespace')
            record(h, Path(row[key]['path']), row[key])


def configurations(h):
    paths = {parent / '.cargo' / name
             for root in (OUT, CPU / 'candidate', RUST_SOURCE / 'library')
             for parent in (root, *root.parents) for name in ('config', 'config.toml')}
    paths |= {Path('/home/harmenon/.cargo') / name for name in ('config', 'config.toml')}
    values = {str(path): h.pin(path) if os.path.lexists(path) else None for path in sorted(paths)}
    require(not any(values.values()), 'inherited Cargo configuration refused')
    return values


def cpu_admission(h):
    value = doc(h, CPU / 'evidence/complete.json', CPU_SHA)
    require(value['schema'] == 'ferric-guarded-mlp-combined-state-cpu-v1'
            and value['passed'] is True and value['failure'] is None
            and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['dependency_commit'] == h.COMMIT and value['controller']['sha256'] == HELPER_SHA
            and value['gpu_execution'] is False and value['compiler_hsaco_reproduced'] is False,
            'actual successful combined-state CPU required')
    raw_replay(h, value, CPU / 'evidence')
    require([row['label'] for row in value['phases']] == ['rustc-version', 'format', 'format-check',
            'metadata', 'host-check', 'build-tests', 'lib-list', 'lib-ignored', 'lib-tests', 'host-build'],
            'exact ten CPU phases')
    source = doc(h, Path(value['final_sources']['path']), value['final_sources'])
    require(source == doc(h, Path(value['tested_sources']['path']), value['tested_sources'])
            == h.sources(), 'CPU-qualified source/lock drift')
    manifest = doc(h, Path(value['input_manifest']['path']), value['input_manifest'])
    expected = manifest['expected_tests']
    require(type(h.EXPECTED_TESTS) is tuple and len(h.EXPECTED_TESTS) > 27
            and expected == list(h.EXPECTED_TESTS) and sorted(set(expected)) == expected
            and sorted(h.inventory(read(CPU / 'evidence/lib-list.stdout').decode())) == expected
            and not h.inventory(read(CPU / 'evidence/lib-ignored.stdout').decode())
            and h.test_outcomes(CPU / 'evidence/lib-tests.stdout', expected) == value['tests'],
            'all exact reviewed combined-state CPU tests, no ignores')
    for item in value['tool_pins'].values():
        record(h, Path(item['path']), item)
    for item in value['artifacts'].values():
        record(h, Path(item['pin']['path']), item['pin'])
    return value, source


def audit_admission(h):
    value = doc(h, AUDIT / 'complete.json', AUDIT_SHA)
    require(value['schema'] == 'ferric-guarded-mlp-memory-bounds-dag-compiler-tool-audit-v1'
            and value['passed'] is True and value['failure'] is None and value['postcheck_errors'] == []
            and value['actual_library_audits_replayed'] is True and value['compiler_invocation'] is False
            and value['producer_receipt_source_and_final_cargo_joins_replayed'] is True
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
            and value['borrow_lookup_test_filter'] == BORROW_LOOKUP_PREFIX
            and value['borrow_lookup_tests'] == list(BORROW_LOOKUP_NAMES)
            and value['cfg_expansion_test_filter'] == CFG_EXPANSION_PREFIX
            and value['cfg_expansion_tests'] == list(CFG_EXPANSION_NAMES)
            and value['census_test_filter'] == CENSUS_PREFIX
            and value['census_tests'] == list(CENSUS_NAMES)
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
            and value['dag_test_filter'] == DAG_PREFIX
            and value['dag_tests'] == list(DAG_NAMES)
            and value['graph_work_diagnostics'] is True
            and value['inherited_graph_work_diagnostics'] is True
            and value['membership_test_filter'] == MEMBERSHIP_PREFIX
            and value['membership_tests'] == list(MEMBERSHIP_NAMES)
            and value['graph_work_test_filter'] == GRAPH_WORK_PREFIX
            and value['graph_work_tests'] == list(GRAPH_WORK_NAMES)
            and value['cfg_diagnostics_retained'] is True
            and value['capacity_limits'] == CAPACITY_LIMITS
            and value['qualified_baseline_producer']['complete']['sha256'] == BASE_COMPLETE_SHA
            and value['driver_receipt_source_and_final_cargo_joins_replayed'] is True
            and value['rlib_deployed'] is False and value['rlib_readelf_or_ldd_invoked'] is False
            and value['tool_manifest']['path'] == str(TOOLS / 'manifest.json')
            and set(value['tools']) == set(NAMES)
            and [row['label'] for row in value['phases']]
                == [name + '-' + phase for name in NAMES for phase in ('readelf', 'ldd')],
            'actual successful fourteen-leaf S/RPO loader audit required')
    raw_replay(h, value, AUDIT)
    for path, item in value['inputs'].items():
        record(h, Path(path), item)
    for reported, canonical in value['resolved_paths'].items():
        require(str(Path(reported).resolve(strict=True)) == canonical, 'audited loader alias changed')
    audit_input = doc(h, Path(value['input_manifest']['path']), value['input_manifest'])
    require(audit_input['schema'] == 'ferric-guarded-mlp-memory-bounds-dag-tool-audit-input-v1'
            and audit_input['controller']['sha256'] == AUDIT_CONTROLLER_SHA
            and audit_input['tool_manifest'] == value['tool_manifest'],
            'reviewed actual loader input/controller generation')
    record(h, Path(audit_input['controller']['path']), audit_input['controller'])
    proof = value['qualified_producer']
    require(proof['complete']['path'] == str(PRODUCER / 'evidence/complete.json')
            and proof['omitted_artifact_bodies_rehashed'] is False
            and proof['diagnostic_build'] is False and proof['diagnostic_only'] is False
            and proof['admission_changed'] is True
            and proof['diagnostic_changes_admission'] is False
            and proof['cfg_expansion_diagnostics'] is True
            and proof['inherited_cfg_expansion_diagnostics'] is True
            and proof['memory_bounds_dag_optimization'] is True
            and proof['memory_bounds_dag_test_filter'] == MEMORY_BOUNDS_DAG_PREFIX
            and proof['memory_bounds_dag_tests'] == list(MEMORY_BOUNDS_DAG_NAMES)
            and proof['inherited_cfg_linear_fusion_optimization'] is True
            and proof['cfg_linear_fusion_optimization'] is True
            and proof['cfg_compaction_optimization'] is True
            and proof['inherited_cfg_compaction_optimization'] is True
            and proof['cfg_compaction_test_filter'] == CFG_COMPACTION_PREFIX
            and proof['cfg_compaction_tests'] == list(CFG_COMPACTION_NAMES)
            and proof['cfg_linear_fusion_test_filter'] == CFG_LINEAR_FUSION_PREFIX
            and proof['cfg_linear_fusion_tests'] == list(CFG_LINEAR_FUSION_NAMES)
            and proof['structural_capacity_expansion'] is False
            and proof['inherited_capacity_expansion'] is True
            and proof['structural_limits_changed'] is False
            and proof['graph_analysis_optimization'] is True
            and proof['inherited_graph_analysis_optimization'] is True
            and proof['resource_admission_may_change'] is True
            and proof['membership_lookup_optimization'] is True
            and proof['inherited_membership_lookup_optimization'] is True
            and proof['dead_cast_census_optimization'] is True
            and proof['compiler_scratch_added'] is True
            and proof['inherited_dead_cast_census_optimization'] is True
            and proof['borrow_lookup_optimization'] is True
            and proof['inherited_borrow_lookup_optimization'] is True
            and proof['use_lookup_optimization'] is True
            and proof['inherited_use_lookup_optimization'] is True
            and proof['use_lookup_test_filter'] == USE_LOOKUP_PREFIX
            and proof['use_lookup_tests'] == list(USE_LOOKUP_NAMES)
            and proof['borrow_lookup_test_filter'] == BORROW_LOOKUP_PREFIX
            and proof['borrow_lookup_tests'] == list(BORROW_LOOKUP_NAMES)
            and proof['cfg_expansion_test_filter'] == CFG_EXPANSION_PREFIX
            and proof['cfg_expansion_tests'] == list(CFG_EXPANSION_NAMES)
            and proof['census_test_filter'] == CENSUS_PREFIX
            and proof['census_tests'] == list(CENSUS_NAMES)
            and proof['semantic_predicates_changed'] is False
            and proof['actual_failure_caller_identified'] is False
            and proof['baseline_failure_caller_identified'] is True
            and proof['baseline_failure_block_count_observed'] is True
            and proof['baseline_memory_bounds_preflight_refusal'] is True
            and proof['baseline_rendered_blocks'] == 567
            and proof['baseline_rendered_edges'] == 1122
            and proof['baseline_rendered_operations'] == 2240
            and proof['baseline_guard_candidates'] == 552
            and proof['baseline_intersection_work_upper_bound'] == 9340170
            and proof['baseline_memory_bounds_work_limit'] == CAPACITY_LIMITS['ranked_work']
            and proof['baseline_runtime_work_exhaustion_observed'] is False
            and proof['baseline_edge_verdict_observed'] is False
            and proof['static_failure_site_identified'] is True
            and proof['actual_failure_block_count_observed'] is False
            and proof['dag_test_filter'] == DAG_PREFIX
            and proof['dag_tests'] == list(DAG_NAMES)
            and proof['graph_work_diagnostics'] is True
            and proof['inherited_graph_work_diagnostics'] is True
            and proof['membership_test_filter'] == MEMBERSHIP_PREFIX
            and proof['membership_tests'] == list(MEMBERSHIP_NAMES)
            and proof['graph_work_test_filter'] == GRAPH_WORK_PREFIX
            and proof['graph_work_tests'] == list(GRAPH_WORK_NAMES)
            and proof['cfg_diagnostics_retained'] is True
            and proof['capacity_limits'] == CAPACITY_LIMITS
            and proof['capacity_cohorts'] == CAPACITY_COHORTS,
            'actual S/RPO producer provenance required')
    producer = doc(h, Path(proof['complete']['path']), proof['complete'])
    require(producer['schema'] == 'ferric-guarded-mlp-memory-bounds-dag-cpu-v1'
            and producer['controller']['sha256'] == PRODUCER_CONTROLLER_SHA
            and producer['helper'] == dict(
                path=str(PRODUCER / 'qualification_helpers.py'),
                bytes=PRODUCER_HELPER_BYTES, sha256=PRODUCER_HELPER_SHA)
            and producer['passed'] is True and producer['failure'] is None
            and producer['diagnostic_build'] is False and producer['diagnostic_only'] is False
            and producer['admission_changed'] is True
            and producer['diagnostic_changes_admission'] is False
            and producer['cfg_expansion_diagnostics'] is True
            and producer['inherited_cfg_expansion_diagnostics'] is True
            and producer['memory_bounds_dag_optimization'] is True
            and producer['memory_bounds_dag_test_filter'] == MEMORY_BOUNDS_DAG_PREFIX
            and producer['memory_bounds_dag_tests'] == list(MEMORY_BOUNDS_DAG_NAMES)
            and producer['inherited_cfg_linear_fusion_optimization'] is True
            and producer['cfg_linear_fusion_optimization'] is True
            and producer['cfg_compaction_optimization'] is True
            and producer['inherited_cfg_compaction_optimization'] is True
            and producer['cfg_compaction_test_filter'] == CFG_COMPACTION_PREFIX
            and producer['cfg_compaction_tests'] == list(CFG_COMPACTION_NAMES)
            and producer['cfg_linear_fusion_test_filter'] == CFG_LINEAR_FUSION_PREFIX
            and producer['cfg_linear_fusion_tests'] == list(CFG_LINEAR_FUSION_NAMES)
            and producer['structural_capacity_expansion'] is False
            and producer['inherited_capacity_expansion'] is True
            and producer['structural_limits_changed'] is False
            and producer['graph_analysis_optimization'] is True
            and producer['inherited_graph_analysis_optimization'] is True
            and producer['resource_admission_may_change'] is True
            and producer['membership_lookup_optimization'] is True
            and producer['inherited_membership_lookup_optimization'] is True
            and producer['dead_cast_census_optimization'] is True
            and producer['compiler_scratch_added'] is True
            and producer['inherited_dead_cast_census_optimization'] is True
            and producer['borrow_lookup_optimization'] is True
            and producer['inherited_borrow_lookup_optimization'] is True
            and producer['use_lookup_optimization'] is True
            and producer['inherited_use_lookup_optimization'] is True
            and producer['use_lookup_test_filter'] == USE_LOOKUP_PREFIX
            and producer['use_lookup_tests'] == list(USE_LOOKUP_NAMES)
            and producer['borrow_lookup_test_filter'] == BORROW_LOOKUP_PREFIX
            and producer['borrow_lookup_tests'] == list(BORROW_LOOKUP_NAMES)
            and producer['cfg_expansion_test_filter'] == CFG_EXPANSION_PREFIX
            and producer['cfg_expansion_tests'] == list(CFG_EXPANSION_NAMES)
            and producer['census_test_filter'] == CENSUS_PREFIX
            and producer['census_tests'] == list(CENSUS_NAMES)
            and producer['semantic_predicates_changed'] is False
            and producer['actual_failure_caller_identified'] is False
            and producer['baseline_failure_caller_identified'] is True
            and producer['baseline_failure_block_count_observed'] is True
            and producer['baseline_memory_bounds_preflight_refusal'] is True
            and producer['baseline_rendered_blocks'] == 567
            and producer['baseline_rendered_edges'] == 1122
            and producer['baseline_rendered_operations'] == 2240
            and producer['baseline_guard_candidates'] == 552
            and producer['baseline_intersection_work_upper_bound'] == 9340170
            and producer['baseline_memory_bounds_work_limit'] == CAPACITY_LIMITS['ranked_work']
            and producer['baseline_runtime_work_exhaustion_observed'] is False
            and producer['baseline_edge_verdict_observed'] is False
            and producer['static_failure_site_identified'] is True
            and producer['actual_failure_block_count_observed'] is False
            and producer['dag_test_filter'] == DAG_PREFIX
            and producer['dag_tests'] == list(DAG_NAMES)
            and producer['graph_work_diagnostics'] is True
            and producer['inherited_graph_work_diagnostics'] is True
            and producer['membership_test_filter'] == MEMBERSHIP_PREFIX
            and producer['membership_tests'] == list(MEMBERSHIP_NAMES)
            and producer['graph_work_test_filter'] == GRAPH_WORK_PREFIX
            and producer['graph_work_tests'] == list(GRAPH_WORK_NAMES)
            and producer['cfg_diagnostics_retained'] is True
            and producer['capacity_limits'] == CAPACITY_LIMITS
            and producer['capacity_cohorts'] == CAPACITY_COHORTS
            and producer['source_lineage']['memory_bounds_dag_proposal']['sha256'] == MEMORY_BOUNDS_DAG_PROPOSAL_SHA
            and producer['source_lineage']['base_complete']['sha256'] == MEMORY_BOUNDS_DAG_BASE_COMPLETE_SHA
            and producer['source_lineage']['base_sources']['sha256'] == MEMORY_BOUNDS_DAG_BASE_SOURCES_SHA
            and producer['source_lineage']['base_input']['sha256'] == MEMORY_BOUNDS_DAG_BASE_INPUT_SHA
            and producer['input_manifest']['sha256'] == PRODUCER_INPUT_SHA
            and len(producer['phases']) == 45 and len(producer['tests']) == 32
            and producer['tests_passed'] == 2990 and producer['tests_ignored'] == 25
            and producer['source_unchanged'] is True and producer['postcheck_errors'] == []
            and producer['final_compiler_product_phase'] == 'compiler-tests-build'
            and producer['source_lineage'] == proof['source_lineage']
            and producer['artifacts']['backend-rlib'] == proof['rlib_cpu_provenance']
            and {role: producer['artifacts'][role] for role in
                 ('pliron-lib', 'compiler-lib', 'atomic-extraction', 'matrix-extraction')}
                == proof['test_elf_cpu_provenance'], 'actual final producer/artifact metadata join')
    lineage = producer['source_lineage']
    require(set(lineage) == {'base_complete', 'base_sources', 'base_input', 'base_metadata',
                            'base_dependencies', 'base_streams', 'memory_bounds_dag_proposal', 'memory_bounds_dag_overlay',
                            'baseline_failure', 'baseline_failure_stderr'},
            'closed qualified memory-bounds DAG source lineage')
    require(lineage['baseline_failure'] == dict(path=str(BASE_FAILURE_ROOT / 'failed.json'),
                bytes=17737, sha256=BASE_FAILURE_SHA)
            and lineage['baseline_failure_stderr'] == dict(
                path=str(BASE_FAILURE_ROOT / 'compile.stderr'), bytes=123245,
                sha256=BASE_FAILURE_STDERR_SHA), 'exact measured baseline failure inputs')
    baseline_failure = doc(h, Path(lineage['baseline_failure']['path']), lineage['baseline_failure'])
    stderr_pin = record(h, Path(lineage['baseline_failure_stderr']['path']), lineage['baseline_failure_stderr'])
    baseline_stderr = read(Path(stderr_pin['path']))
    require(len(baseline_stderr) == stderr_pin['bytes']
            and hashlib.sha256(baseline_stderr).hexdigest() == stderr_pin['sha256'],
            'baseline stderr changed after pin')
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
            and {key: baseline_failure['raw']['source-before.json'][key] for key in ('bytes', 'sha256')}
                == {key: baseline_failure['raw']['source-after.json'][key] for key in ('bytes', 'sha256')}
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
    for key, input_key, actual in (
            ('complete', 'memory_bounds_dag_complete', proof['complete']),
            ('input', 'memory_bounds_dag_input', producer['input_manifest']),
            ('sources', 'memory_bounds_dag_sources', producer['input_sources']),
            ('final_build', 'memory_bounds_dag_final_build', producer['raw']['compiler-tests-build.stdout'])):
        require(proof[key] == audit_input[input_key] == actual, 'audited producer readset join: ' + key)
        record(h, Path(proof[key]['path']), proof[key])
    manifest = doc(h, TOOLS / 'manifest.json', value['tool_manifest'])
    driver_manifest_pin = value['driver_tool_manifest']
    require(driver_manifest_pin == audit_input['driver_tool_manifest']
            and driver_manifest_pin['sha256'] == DRIVER_MANIFEST_SHA,
            'actual unchanged driver-generation manifest')
    driver_manifest = doc(h, Path(driver_manifest_pin['path']), driver_manifest_pin)
    driver_admission(h, value, audit_input, driver_manifest)
    require(set(driver_manifest) == set(manifest)
            and all(manifest['bin/' + name] == driver_manifest['bin/' + name]
                    for name in NAMES if name not in
                    ('fe2o3-rustc-extract', 'librustc_codegen_fe2o3.so')),
            'only memory-bounds-DAG-qualified extractor/backend may change')
    require(set(manifest) == {'bin/' + name for name in NAMES}, 'exact seven courier tool names')
    for name in NAMES:
        row = value['tools'][name]
        require(row['original'] == manifest['bin/' + name]
                and row['deployed']['path'] == str(TOOLS / 'bin' / name)
                and all(row['deployed'][key] == row['original'][key] for key in ('bytes', 'sha256')),
                'courier/original tool join')
        record(h, TOOLS / 'bin' / name, row['deployed'])
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        artifact = producer['artifacts'][role]['pin']
        require(manifest['bin/' + name] == dict(source=artifact['path'], bytes=artifact['bytes'],
                                               sha256=artifact['sha256']),
                'deployed replacement is not the final qualified S/RPO product')
    require(value['audits']['fe2o3-rustc-extract']['libraries']['librustc_codegen_fe2o3.so']['pin']
            == value['tools']['librustc_codegen_fe2o3.so']['deployed'], 'exact S/RPO backend linkage')
    return value


def driver_admission(h, value, audit_input, manifest):
    previous_pin = value['previous_tool_manifest']
    require(previous_pin == audit_input['previous_tool_manifest']
            and previous_pin['sha256'] == PREVIOUS_MANIFEST_SHA,
            'actual previous seven-tool manifest required')
    previous = doc(h, Path(previous_pin['path']), previous_pin)
    require(set(previous) == set(manifest) == {'bin/' + name for name in NAMES}
            and all(manifest['bin/' + name] == previous['bin/' + name]
                    for name in NAMES if name != 'cargo-fe2o3'),
            'only the driver may change from the actual S/RPO tool generation')
    proof = value['qualified_driver']
    require(proof['complete']['path'] == str(DRIVER / 'evidence/complete.json')
            and proof['omitted_test_elf_body_rehashed'] is False
            and proof['complete']['sha256'] == DRIVER_COMPLETE_SHA,
            'new driver receipt and metadata-only test ELF provenance required')
    driver = doc(h, Path(proof['complete']['path']), proof['complete'])
    require(driver['schema'] == 'ferric-guarded-mlp-driver-cpu-v1'
            and driver['controller']['sha256'] == DRIVER_CONTROLLER_SHA
            and driver['source_generation'] == DRIVER_GENERATION
            and driver['passed'] is True and driver['failure'] is None
            and driver['source_unchanged'] is True and driver['postcheck_errors'] == []
            and driver['final_artifact_phase'] == 'driver-build'
            and driver['test_artifact_phase'] == 'driver-tests-build'
            and driver['full_bin_tests_executed'] is False
            and driver['test_scope'] == 'engineering_hsaco::tests::'
            and set(driver['artifacts']) == {'driver', 'test'}
            and driver['artifacts']['driver'] == proof['driver']
            and driver['artifacts']['test'] == proof['test_elf_cpu_provenance'],
            'actual reviewed focused driver qualification and final artifact joins')
    selected = driver['engineering_tests']
    require(len(selected) == 18 and selected == sorted(set(selected))
            and all(name.startswith('engineering_hsaco::tests::') for name in selected)
            and driver['tests'] == {'driver': dict(names=selected, passed=18, failed=0, ignored=0,
                                                  filtered_out=len(driver['inventory']) - 18)},
            'exact focused driver result scope, not a full-bin suite claim')
    for key, input_key, actual in (
            ('complete', 'driver_complete', proof['complete']),
            ('input', 'driver_input', driver['input_manifest']),
            ('sources', 'driver_sources', driver['raw']['sources-before.json']),
            ('final_build', 'driver_final_build', driver['raw']['driver-build.stdout']),
            ('test_build', 'driver_test_build', driver['raw']['driver-tests-build.stdout'])):
        require(proof[key] == audit_input[input_key] == actual, 'audited driver readset join: ' + key)
        record(h, Path(proof[key]['path']), proof[key])
    sources = doc(h, Path(proof['sources']['path']), proof['sources'])
    require(sources == driver['input_sources'] == driver['final_sources'] and len(sources) == 5299,
            'actual unchanged complete driver source map')
    inputs = doc(h, Path(proof['input']['path']), proof['input'])
    require(inputs['schema'] == 'ferric-guarded-mlp-driver-cpu-input-v1'
            and inputs['source_generation'] == DRIVER_GENERATION
            and inputs['files'] == {name: {key: row[key] for key in ('bytes', 'sha256')}
                                   for name, row in sources.items()},
            'driver input/source map identity')
    record(h, Path(driver['controller']['path']), driver['controller'])
    artifact = driver['artifacts']['driver']['pin']
    require(manifest['bin/cargo-fe2o3'] == dict(source=artifact['path'], bytes=artifact['bytes'],
                                               sha256=artifact['sha256']),
            'deployed driver is not the final qualified executable')


def git_sources(config):
    require(set(config) == {'source'} and isinstance(config['source'], dict), 'closed vendor config')
    rows = []
    for name, entry in config['source'].items():
        if name == 'vendored-sources':
            require(entry == {'directory': str(VENDOR)}, 'selected vendor directory')
        elif name == 'crates-io':
            require(entry == {'replace-with': 'vendored-sources'}, 'crates.io replacement')
        else:
            require(set(entry) == {'git', 'rev', 'replace-with'}
                    and entry['replace-with'] == 'vendored-sources'
                    and re.fullmatch(r'https://[A-Za-z0-9./_-]+', entry['git'])
                    and re.fullmatch(r'[0-9a-f]{40}', entry['rev']), 'closed revision-pinned git source')
            rows.append(dict(url=entry['git'], rev=entry['rev']))
    rows.sort(key=lambda row: (row['url'], row['rev']))
    require('vendored-sources' in config['source'] and len(rows) <= 16
            and len({(r['url'], r['rev']) for r in rows}) == len(rows), 'vendor source roster')
    return rows


def vendor_admission(h, expected_sha, source):
    require(VENDOR_CONTROLLER_SHA is not None, 'reviewed combined-state vendor controller binding still pending')
    directory = VENDOR_ROOT / 'evidence'
    value = doc(h, directory / 'complete.json', expected_sha)
    require(value['schema'] == 'ferric-guarded-mlp-combined-state-vendor-preparation-v1' and value['passed'] is True
            and value['failure'] is None and value['postcheck_errors'] == []
            and value['qualified_cpu_complete'] == INPUTS[str(CPU / 'evidence/complete.json')]
            and value['controller']['sha256'] == VENDOR_CONTROLLER_SHA
            and value['input_sources_unchanged'] is True and value['config_installed'] is False
            and value['offline'] is True and value['host_fixture_crate_binding_used'] is False
            and value['vendor_directory'] == str(VENDOR)
            and [row['label'] for row in value['phases']] == ['vendor'], 'fresh qualified vendor required')
    raw_replay(h, value, directory)
    for path, item in value['inputs'].items():
        record(h, Path(path), item)
    require(doc(h, directory / 'cpu-sources-before.json')
            == doc(h, directory / 'cpu-sources-after.json') == source, 'vendor/CPU source join')
    rust = doc(h, directory / 'rust-src-before.json')
    require(rust == doc(h, directory / 'rust-src-after.json') == tree(h, RUST_SOURCE),
            'vendor nightly rust-src/lock drift')
    files = doc(h, Path(value['vendor_files']['path']), value['vendor_files'])
    require(files and files == tree(h, VENDOR), 'vendor body roster drift')
    return value, files, rust, git_sources(value['generated_config'])


def command(cpu, audit, git):
    tools = {name: row['deployed'] for name, row in audit['tools'].items()}
    for name in ('cargo', 'rustc'):
        matching = [row for row in cpu['tool_pins'].values() if row['path'] == str(TOOLCHAIN / 'bin' / name)]
        require(len(matching) == 1, 'unique CPU-qualified nightly executable')
        tools[name] = matching[0]
    argv = [tools['cargo-fe2o3']['path'], 'engineering', 'hsaco', '--crate', CRATE,
            '--output-root', str(OUT / 'fe2o3-engineering-v1'), '--target', 'gfx950:xnack-',
            '--code-object-version', '6']
    for flag, name in [('extractor', 'fe2o3-rustc-extract'),
                       ('extractor-backend', 'librustc_codegen_fe2o3.so'),
                       ('worker', 'fe2o3-llvm-link-worker'), ('cargo', 'cargo'), ('rustc', 'rustc'),
                       ('host-linker', 'clang-22'), ('host-lld', 'lld'),
                       ('host-lld-proxy', 'fe2o3-engineering-lld-proxy')]:
        argv += ['--' + flag, tools[name]['path'], '--' + flag + '-sha256', tools[name]['sha256']]
    argv += ['--worker-build-id', WORKER_BUILD, '--llvm-build-id', LLVM_BUILD, '--cargo-vendor', str(VENDOR)]
    for row in git:
        argv += ['--cargo-git-source', row['url'] + '@' + row['rev']]
    argv += ['--timeout-seconds', str(LEAF_SECONDS),
             '--mir-normalization', 'optimized-inline-v1', '--', '--manifest-path',
             str(CPU / 'candidate/Cargo.toml'), '--lib', '--no-default-features', '--features', 'gfx950']
    env = dict(PATH='/usr/bin:/bin', HOME=str(OUT), LANG='C', LC_ALL='C', TZ='UTC',
               TMPDIR=str(SCRATCH), CARGO_BUILD_JOBS='2', RUST_TEST_THREADS='2')
    return argv, env, tools


def scratch_bytes():
    require(SCRATCH.is_dir() and not SCRATCH.is_symlink(), 'private scratch root missing/aliased')
    total = count = 0
    def walk_error(error):
        path = Path(error.filename) if isinstance(error.filename, str) else None
        if (isinstance(error, FileNotFoundError) and path is not None and path.is_absolute()
                and '..' not in path.parts and path != SCRATCH and path.is_relative_to(SCRATCH)):
            return
        raise error
    for directory, _, names in os.walk(SCRATCH, followlinks=False, onerror=walk_error):
        for name in names:
            try:
                total += (Path(directory) / name).lstat().st_size
            except FileNotFoundError:
                continue
            count += 1
            require(count <= 200000, 'private scratch file-count bound')
    return total


def owned(h, argv, env, phases, hard_deadline):
    require(time.monotonic() + LEAF_SECONDS + CLEANUP_RESERVE < hard_deadline,
            'insufficient full compiler/cleanup reserve')
    require(shutil.disk_usage(E).free >= START_FREE, '40 GiB launch floor')
    wrapped = ['/usr/bin/prlimit', '--as=' + str(AS_LIMIT), '--cpu=' + str(LEAF_SECONDS),
               '--fsize=' + str(FILE_LIMIT), '--core=0', '--', *argv]
    command_pin = h.save('compile.command.json', dict(argv=wrapped, compiler_argv=argv,
        cwd=str(CPU / 'candidate'), env=env, wall_seconds=LEAF_SECONDS, cleanup_seconds=CLEANUP_RESERVE))
    stdout, stderr = OUT / 'compile.stdout', OUT / 'compile.stderr'
    start, child, exception = time.monotonic_ns(), None, None
    timed_out = forced = False
    group_absent, peak, minimum = False, 0, shutil.disk_usage(E).free
    deferred, reaped = [], []
    with stdout.open('xb') as so, stderr.open('xb') as se:
        previous = {sig: signal.getsignal(sig) for sig in SIGNALS}
        def defer(signum, _frame):
            if len(deferred) < 16:
                deferred.append(signum)
        for sig in SIGNALS:
            signal.signal(sig, defer)
        try:
            require(not deferred, 'termination before compiler spawn')
            child = subprocess.Popen(wrapped, cwd=CPU / 'candidate', env=env, stdin=subprocess.DEVNULL,
                                     stdout=so, stderr=se, start_new_session=True)
            require(os.getpgid(child.pid) == child.pid, 'owned compiler process-group identity')
            h.save('compile.started.json', dict(pid=child.pid, pgid=child.pid, argv=wrapped))
            deadline, next_storage = time.monotonic() + LEAF_SECONDS, 0.0
            while child.poll() is None:
                require(not deferred, 'termination during compiler: ' + repr(deferred))
                now = time.monotonic()
                if now >= deadline:
                    timed_out = True
                    break
                require(max(os.fstat(so.fileno()).st_size, os.fstat(se.fileno()).st_size) <= STREAM_LIMIT,
                        '64 MiB compiler stream cap')
                if now >= next_storage:
                    peak = max(peak, scratch_bytes())
                    minimum = min(minimum, shutil.disk_usage(E).free)
                    require(peak <= CACHE_LIMIT and minimum >= LIVE_FREE, 'scratch/live disk bound')
                    next_storage = now + 2
                time.sleep(0.1)
        except BaseException as error:
            exception = repr(error)
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            try:
                if child is not None:
                    child.poll()
                    if child.returncode is not None:
                        reaped.extend(h.reap_group(child.pid))
                    if child.returncode is None or h.group_exists(child.pid):
                        forced = True
                        reaped.extend(h.stop_group(child, hard_deadline))
                    try:
                        child.wait(timeout=max(0.001, min(5, hard_deadline - time.monotonic())))
                    except subprocess.TimeoutExpired:
                        exception = (exception or '') + ' leader not reaped'
                    if child.returncode is not None:
                        reaped.extend(h.reap_group(child.pid))
                    group_absent = not h.group_exists(child.pid)
            finally:
                for sig, handler in previous.items():
                    signal.signal(sig, handler)
    if deferred:
        exception = (exception or '') + ' deferred signals: ' + repr(deferred)
    row = dict(label='compile', command=command_pin, pid=child.pid if child else None,
        pgid=child.pid if child else None, exit_code=child.returncode if child else None,
        natural_exit=child is not None and child.returncode is not None and not timed_out
            and not forced and exception is None, reaped=child is not None and child.returncode is not None,
        timed_out=timed_out, forced_cleanup=forced, exception=exception, observed_signals=deferred,
        process_group_absent=group_absent, adopted_reaped=reaped, elapsed_ns=time.monotonic_ns() - start,
        peak_private_scratch_bytes=peak, minimum_free_bytes=minimum,
        stdout=h.pin(stdout), stderr=h.pin(stderr))
    phases.append(row)
    h.save('compile.result.json', row)
    natural(row)
    require(max(row['stdout']['bytes'], row['stderr']['bytes']) <= STREAM_LIMIT
            and scratch_bytes() <= CACHE_LIMIT and shutil.disk_usage(E).free >= LIVE_FREE,
            'post-compiler storage bounds')
    signal.setitimer(signal.ITIMER_REAL, max(0.001, hard_deadline - time.monotonic()))


def output(h, tools, git):
    lines = read(OUT / 'compile.stdout', STREAM_LIMIT).decode().splitlines()
    require(len(lines) == 1, 'one content-directory record required')
    folder = Path(lines[0])
    require(folder.parent == OUT / 'fe2o3-engineering-v1' and re.fullmatch(r'[0-9a-f]{64}', folder.name)
            and folder.resolve(strict=True) == folder
            and sorted(path.name for path in folder.iterdir()) == ['observation.hsaco', 'observation.json'],
            'closed ordinary content directory')
    manifest = read(folder / 'observation.json', 1 << 20)
    image = read(folder / 'observation.hsaco', STREAM_LIMIT)
    value = json.loads(manifest, object_pairs_hook=pairs)
    identity = lambda row: dict(sha256=row['sha256'], byte_len=row['bytes'])
    expected = {key: identity(tools[name]) for key, name in [('cargo', 'cargo'), ('rustc', 'rustc'),
        ('host_linker', 'clang-22'), ('host_lld', 'lld'), ('host_lld_proxy', 'fe2o3-engineering-lld-proxy'),
        ('extractor', 'fe2o3-rustc-extract'), ('extractor_backend', 'librustc_codegen_fe2o3.so')]}
    observed = value['tools']
    require(set(observed) == set(expected) | {'rustc_lib_tree_sha256', 'cargo_vendor', 'worker'}
            and all(observed[key] == row for key, row in expected.items())
            and observed['worker'] == dict(executable=identity(tools['fe2o3-llvm-link-worker']),
                worker_build_identity=WORKER_BUILD, llvm_build_identity=LLVM_BUILD)
            and re.fullmatch(r'[0-9a-f]{64}', observed['rustc_lib_tree_sha256'])
            and set(observed['cargo_vendor']) == {'tree_sha256', 'git_sources'}
            and re.fullmatch(r'[0-9a-f]{64}', observed['cargo_vendor']['tree_sha256'])
            and observed['cargo_vendor']['git_sources'] == git, 'exact selected engineering tool closure')
    require(value['schema'] == 'EngineeringHsacoObservationV1' and value['namespace'] == 'fe2o3-engineering-v1'
            and value['authority'] == 'none' and value['crate_name'] == CRATE and value['target'] == 'gfx950:xnack-'
            and value['code_object_version'] == 6 and value['providers'] == []
            and value['grants'] == dict(publication=False, load=False, launch=False)
            and value['options'] == dict(mir_normalization='optimized-inline-v1',
                extraction_rustflags='-Zalways-encode-mir -Zinline-mir=yes '
                    '-Zmir-enable-passes=-JumpThreading -Copt-level=3 -Ctarget-cpu=gfx950 '
                    '-Ctarget-feature=-wavefrontsize32,+wavefrontsize64,-xnack',
                optimization='O2', strip_debug=True, verify_each=True,
                timeout_seconds=LEAF_SECONDS, maximum_output_bytes=STREAM_LIMIT)
            and value['execution']['exact_output_replay'] is True
            and sorted(value['hsaco']['kernel_names']) == KERNELS
            and value['hsaco']['identity'] == dict(sha256=hashlib.sha256(image).hexdigest(), byte_len=len(image))
            and image.startswith(b'\x7fELF'), 'checked output/roots/options/non-authority')
    content = hashlib.sha256(b'FE2O3/ENGINEERING-HSACO-OBSERVATION-CONTENT/V1\0'
        + len(manifest).to_bytes(8, 'little') + manifest + len(image).to_bytes(8, 'little') + image).hexdigest()
    require(content == folder.name, 'content-addressed output differs')
    return dict(observation=h.pin(folder / 'observation.json'), image=h.pin(folder / 'observation.hsaco'), value=value)


def main():
    started = time.monotonic()
    hard_deadline = started + WHOLE_SECONDS
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]),
            'usage: python3 -B lower_guarded_mlp_combined_state_v228_v3.py ACTUAL_VENDOR_COMPLETE_SHA')
    require(all(type(value) is str and re.fullmatch(r'[0-9a-f]{64}', value)
                for value in (CPU_SHA, HELPER_SHA, AUDIT_SHA, VENDOR_CONTROLLER_SHA,
                              AUDIT_CONTROLLER_SHA, DRIVER_CONTROLLER_SHA,
                              PRODUCER_CONTROLLER_SHA, MEMORY_BOUNDS_DAG_PROPOSAL_SHA, PRODUCER_INPUT_SHA, PRODUCER_HELPER_SHA)),
            'actual CPU, loader receipt and vendor-controller bindings are still pending')
    require(Path(__file__).resolve() == SCRIPT and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'controller host/path/UID')
    require(not os.path.lexists(OUT) and E.resolve(strict=True) == E, 'fresh canonical output required')
    require(shutil.disk_usage(E).free >= START_FREE, '40 GiB setup floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    require(os.getpriority(os.PRIO_PROCESS, 0) in (0, 10), 'initial nice level')
    if os.getpriority(os.PRIO_PROCESS, 0) == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, AS_LIMIT), (resource.RLIMIT_FSIZE, FILE_LIMIT)):
        soft, hard = resource.getrlimit(kind)
        effective = min([cap] + [n for n in (soft, hard) if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (effective, effective))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup')
    h = helper()
    h.OUT = OUT
    OUT.mkdir(mode=0o700)
    SCRATCH.mkdir(mode=0o700)
    for sig in SIGNALS:
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, hard_deadline - CLEANUP_RESERVE - time.monotonic()))
    cpu = audit = vendor = source = vendor_files = rust_source = config = artifact = None
    phases, postchecks, failure = [], [], None
    try:
        record(h, SCRIPT)
        record(h, CPU / 'run_cpu.py', HELPER_SHA)
        record(h, Path('/usr/bin/prlimit'))
        cpu, source = cpu_admission(h)
        audit = audit_admission(h)
        vendor, vendor_files, rust_source, git = vendor_admission(h, sys.argv[1], source)
        config = configurations(h)
        argv, env, selected_tools = command(cpu, audit, git)
        h.save('inputs-before.json', INPUTS)
        h.save('source-before.json', source)
        h.save('configurations.json', config)
        owned(h, argv, env, phases, hard_deadline)
        artifact = output(h, selected_tools, git)
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    def check(label, action):
        try:
            remaining = hard_deadline - 5 - time.monotonic()
            require(remaining > 0, 'postcheck whole deadline')
            signal.setitimer(signal.ITIMER_REAL, remaining)
            require(action(), label + ' drift')
        except BaseException as error:
            postchecks.append(label + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    if source is not None:
        def source_after():
            actual = h.sources()
            h.save('source-after.json', actual)
            return actual == source
        check('qualified sources and lock', source_after)
    if vendor_files is not None:
        check('vendor tree', lambda: tree(h, VENDOR) == vendor_files)
    if rust_source is not None:
        check('nightly rust-src', lambda: tree(h, RUST_SOURCE) == rust_source)
    if config is not None:
        check('Cargo configurations', lambda: configurations(h) == config)
    check('all consumed bodies/tools/providers', lambda: all(h.pin(Path(path)) == value for path, value in INPUTS.items()))
    if audit is not None:
        check('provider aliases', lambda: all(str(Path(path).resolve(strict=True)) == actual
                                             for path, actual in audit['resolved_paths'].items()))
    check('seven-tool roster', lambda: {p.name for p in TOOLS.iterdir()} == {'bin', 'manifest.json'}
          and {p.name for p in (TOOLS / 'bin').iterdir()} == set(NAMES))
    if artifact is not None:
        check('produced bodies', lambda: all(h.pin(Path(artifact[key]['path'])) == artifact[key]
                                            for key in ('observation', 'image')))
    if time.monotonic() >= hard_deadline:
        postchecks.append('whole controller deadline exceeded')
    if shutil.disk_usage(E).free < LIVE_FREE:
        postchecks.append('38 GiB final storage floor')
    passed = failure is None and not postchecks and artifact is not None
    h.save('inputs-after.json', dict(inputs=INPUTS, postcheck_errors=postchecks))
    result = dict(schema='ferric-guarded-mlp-combined-state-lowering-result-v1', passed=passed, failure=failure,
        postcheck_errors=postchecks, phases=phases, artifact=artifact, controller=h.pin(SCRIPT),
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
        admission_changed=True, structural_capacity_expansion=False, inherited_capacity_expansion=True,
        structural_limits_changed=False, graph_analysis_optimization=True,
        inherited_graph_analysis_optimization=True,
        resource_admission_may_change=True, membership_lookup_optimization=True,
        inherited_membership_lookup_optimization=True,
        dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
        borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
        use_lookup_optimization=True,
        inherited_use_lookup_optimization=True, compiler_scratch_added=True,
        use_lookup_test_filter=USE_LOOKUP_PREFIX, use_lookup_tests=list(USE_LOOKUP_NAMES),
        borrow_lookup_test_filter=BORROW_LOOKUP_PREFIX, borrow_lookup_tests=list(BORROW_LOOKUP_NAMES),
        cfg_expansion_test_filter=CFG_EXPANSION_PREFIX, cfg_expansion_tests=list(CFG_EXPANSION_NAMES),
        census_test_filter=CENSUS_PREFIX, census_tests=list(CENSUS_NAMES),
        semantic_predicates_changed=False,
        actual_failure_caller_identified=False, baseline_failure_caller_identified=audit is not None,
        baseline_failure_block_count_observed=audit is not None,
        baseline_memory_bounds_preflight_refusal=audit is not None,
        baseline_rendered_blocks=567 if audit is not None else None,
        baseline_rendered_edges=1122 if audit is not None else None,
        baseline_rendered_operations=2240 if audit is not None else None,
        baseline_guard_candidates=552 if audit is not None else None,
        baseline_intersection_work_upper_bound=9340170 if audit is not None else None,
        baseline_memory_bounds_work_limit=8388608 if audit is not None else None,
        baseline_runtime_work_exhaustion_observed=False,
        baseline_edge_verdict_observed=False,
        dag_test_filter=DAG_PREFIX, dag_tests=list(DAG_NAMES),
        graph_work_diagnostics=True, inherited_graph_work_diagnostics=True,
        graph_work_test_filter=GRAPH_WORK_PREFIX,
        graph_work_tests=list(GRAPH_WORK_NAMES),
        membership_test_filter=MEMBERSHIP_PREFIX, membership_tests=list(MEMBERSHIP_NAMES),
        cfg_diagnostics_retained=True,
        capacity_limits=CAPACITY_LIMITS,
        cpu_complete=INPUTS.get(str(CPU / 'evidence/complete.json')),
        tool_audit=INPUTS.get(str(AUDIT / 'complete.json')),
        vendor_complete=INPUTS.get(str(VENDOR_ROOT / 'evidence/complete.json')),
        source_unchanged=source is not None and not postchecks,
        input_byte_maps_rechecked=source is not None and vendor_files is not None and not postchecks,
        compiler_internal_tree_digest_encoding_replayed=False,
        host_fixture_crate_binding_used=False, automatic_retries=0,
        elapsed_seconds=time.monotonic() - started,
        limits=dict(whole_seconds=WHOLE_SECONDS, compiler_seconds=LEAF_SECONDS,
            cleanup_reserve_seconds=CLEANUP_RESERVE, address_space_bytes=AS_LIMIT,
            private_scratch_bytes=CACHE_LIMIT, stream_and_output_bytes=STREAM_LIMIT,
            file_bytes=FILE_LIMIT, affinity=[8, 9], nice=10, initial_free_bytes=START_FREE, live_free_bytes=LIVE_FREE),
        gpu_execution=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False, production_authority=False, load_authority=False, launch_authority=False,
        retained_handoff_or_llvm=False, raw={p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()})
    receipt = h.save('complete.json' if passed else 'failed.json', result)
    print(json.dumps(receipt), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
