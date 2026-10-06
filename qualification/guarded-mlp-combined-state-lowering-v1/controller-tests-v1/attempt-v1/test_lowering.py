"""Synthetic audit joins and retained-source checks; no compiler or auditor is run."""
import ast
import copy
import hashlib
import json
from pathlib import Path
import unittest
from unittest import mock

import lowering as L

# Adjacent bodies are mechanically copied from retained evidence by the fixture packager.
_FIXTURE_ROOT = Path(__file__).resolve().parent
BASELINE_FAILURE_BODY = (_FIXTURE_ROOT / 'baseline-failed.json').read_bytes()
BASELINE_STDERR = (_FIXTURE_ROOT / 'baseline-compile.stderr').read_bytes()
if ((len(BASELINE_FAILURE_BODY), hashlib.sha256(BASELINE_FAILURE_BODY).hexdigest())
        != (17737, '8d4cc1a0cfe0c9104282c3d20349638a616bd98e7e28f92598dcaf0110057ca2')
        or (len(BASELINE_STDERR), hashlib.sha256(BASELINE_STDERR).hexdigest())
        != (123245, 'c60dc2edc62bfe11ba7a8f79ce1465322d2c887aeb3fa5661dae8002a33c5095')):
    raise RuntimeError('packaged baseline fixture pins differ')
BASELINE_FAILURE = json.loads(BASELINE_FAILURE_BODY)


def fixture():
    def pin(path, digest='1' * 64):
        return dict(path=str(path), bytes=10, sha256=digest)

    manifest = {'bin/' + name: dict(source='/synthetic/old/' + name, bytes=10, sha256='1' * 64)
                for name in L.NAMES}
    artifacts = {role: dict(pin=pin(L.PRODUCER / 'target' / role), cargo_artifact={}) for role in
                 ('backend', 'extractor', 'backend-rlib', 'pliron-lib', 'compiler-lib',
                  'atomic-extraction', 'matrix-extraction')}
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        product = artifacts[role]['pin']
        manifest['bin/' + name] = dict(source=product['path'], bytes=product['bytes'], sha256=product['sha256'])
    tools = {name: dict(original=manifest['bin/' + name], deployed=pin(L.TOOLS / 'bin' / name))
             for name in L.NAMES}
    producer = dict(schema='ferric-guarded-mlp-memory-bounds-dag-cpu-v1',
                    controller=pin(L.PRODUCER / 'run_cpu.py', L.PRODUCER_CONTROLLER_SHA),
                    helper=dict(path=str(L.PRODUCER / 'qualification_helpers.py'),
                                bytes=L.PRODUCER_HELPER_BYTES, sha256=L.PRODUCER_HELPER_SHA),
                    passed=True, failure=None, source_unchanged=True, postcheck_errors=[],
                    final_compiler_product_phase='compiler-tests-build',
                    source_lineage=dict(memory_bounds_dag_proposal=pin('/synthetic/proposal.json', L.MEMORY_BOUNDS_DAG_PROPOSAL_SHA),
                                        base_complete=pin('/synthetic/cap.json', L.MEMORY_BOUNDS_DAG_BASE_COMPLETE_SHA),
                                        base_sources=pin('/synthetic/cap-sources.json', L.MEMORY_BOUNDS_DAG_BASE_SOURCES_SHA),
                                        base_input=pin('/synthetic/cap-input.json', L.MEMORY_BOUNDS_DAG_BASE_INPUT_SHA),
                                        base_metadata={}, base_dependencies={}, base_streams={},
                                        memory_bounds_dag_overlay={},
                                        baseline_failure=dict(path=str(L.BASE_FAILURE_ROOT / 'failed.json'),
                                                              bytes=17737, sha256=L.BASE_FAILURE_SHA),
                                        baseline_failure_stderr=dict(path=str(L.BASE_FAILURE_ROOT / 'compile.stderr'),
                                                                     bytes=123245, sha256=L.BASE_FAILURE_STDERR_SHA)),
                    diagnostic_build=False, diagnostic_only=False,
                    diagnostic_changes_admission=False, cfg_expansion_diagnostics=True,
                    inherited_cfg_expansion_diagnostics=True, memory_bounds_dag_optimization=True,
                    inherited_cfg_linear_fusion_optimization=True, cfg_linear_fusion_optimization=True,
                    cfg_compaction_optimization=True, inherited_cfg_compaction_optimization=True,
                    cfg_compaction_test_filter=L.CFG_COMPACTION_PREFIX, cfg_compaction_tests=list(L.CFG_COMPACTION_NAMES),
                    cfg_linear_fusion_test_filter=L.CFG_LINEAR_FUSION_PREFIX, cfg_linear_fusion_tests=list(L.CFG_LINEAR_FUSION_NAMES),
                    memory_bounds_dag_test_filter=L.MEMORY_BOUNDS_DAG_PREFIX, memory_bounds_dag_tests=list(L.MEMORY_BOUNDS_DAG_NAMES),
                    static_failure_site_identified=True, actual_failure_block_count_observed=False, admission_changed=True,
                    structural_capacity_expansion=False, inherited_capacity_expansion=True,
                    structural_limits_changed=False, graph_analysis_optimization=True, inherited_graph_analysis_optimization=True,
                    resource_admission_may_change=True, membership_lookup_optimization=True, inherited_membership_lookup_optimization=True,
                    dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                    borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
                    use_lookup_optimization=True,
                    inherited_use_lookup_optimization=True, compiler_scratch_added=True,
                    borrow_lookup_test_filter=L.BORROW_LOOKUP_PREFIX, borrow_lookup_tests=list(L.BORROW_LOOKUP_NAMES),
                    cfg_expansion_test_filter=L.CFG_EXPANSION_PREFIX, cfg_expansion_tests=list(L.CFG_EXPANSION_NAMES),
                    use_lookup_test_filter=L.USE_LOOKUP_PREFIX, use_lookup_tests=list(L.USE_LOOKUP_NAMES),
                    census_test_filter=L.CENSUS_PREFIX, census_tests=list(L.CENSUS_NAMES),
                    semantic_predicates_changed=False,
                    actual_failure_caller_identified=False, baseline_failure_caller_identified=True, baseline_failure_block_count_observed=True,
                    baseline_memory_bounds_preflight_refusal=True,
                    baseline_rendered_blocks=567, baseline_rendered_edges=1122,
                    baseline_rendered_operations=2240, baseline_guard_candidates=552,
                    baseline_intersection_work_upper_bound=9340170, baseline_memory_bounds_work_limit=8388608,
                    baseline_runtime_work_exhaustion_observed=False,
                    baseline_edge_verdict_observed=False,
                    membership_test_filter=L.MEMBERSHIP_PREFIX, membership_tests=list(L.MEMBERSHIP_NAMES),
                    inherited_graph_work_diagnostics=True, dag_test_filter=L.DAG_PREFIX,
                    dag_tests=list(L.DAG_NAMES), graph_work_diagnostics=True,
                    graph_work_test_filter=L.GRAPH_WORK_PREFIX, graph_work_tests=list(L.GRAPH_WORK_NAMES), cfg_diagnostics_retained=True,
                    capacity_limits=copy.deepcopy(L.CAPACITY_LIMITS),
                    capacity_cohorts=copy.deepcopy(L.CAPACITY_COHORTS),
                    phases=[{} for _ in range(45)], tests={str(i): {} for i in range(32)},
                    tests_passed=2990, tests_ignored=25,
                    artifacts=artifacts, input_manifest=pin(L.PRODUCER / 'input-manifest.json', L.PRODUCER_INPUT_SHA),
                    input_sources=pin(L.PRODUCER / 'evidence/sources-before.json'),
                    raw={'compiler-tests-build.stdout': pin(L.PRODUCER / 'evidence/compiler-tests-build.stdout')})
    proof = dict(complete=pin(L.PRODUCER / 'evidence/complete.json'),
                 input=producer['input_manifest'], sources=producer['input_sources'],
                 final_build=producer['raw']['compiler-tests-build.stdout'], source_lineage=copy.deepcopy(producer['source_lineage']),
                 diagnostic_build=False, diagnostic_only=False,
                    diagnostic_changes_admission=False, cfg_expansion_diagnostics=True,
                    inherited_cfg_expansion_diagnostics=True, memory_bounds_dag_optimization=True,
                    inherited_cfg_linear_fusion_optimization=True, cfg_linear_fusion_optimization=True,
                    cfg_compaction_optimization=True, inherited_cfg_compaction_optimization=True,
                    cfg_compaction_test_filter=L.CFG_COMPACTION_PREFIX, cfg_compaction_tests=list(L.CFG_COMPACTION_NAMES),
                    cfg_linear_fusion_test_filter=L.CFG_LINEAR_FUSION_PREFIX, cfg_linear_fusion_tests=list(L.CFG_LINEAR_FUSION_NAMES),
                    memory_bounds_dag_test_filter=L.MEMORY_BOUNDS_DAG_PREFIX, memory_bounds_dag_tests=list(L.MEMORY_BOUNDS_DAG_NAMES),
                    static_failure_site_identified=True, actual_failure_block_count_observed=False, admission_changed=True,
                 structural_capacity_expansion=False, inherited_capacity_expansion=True,
                    structural_limits_changed=False, graph_analysis_optimization=True, inherited_graph_analysis_optimization=True,
                    resource_admission_may_change=True, membership_lookup_optimization=True, inherited_membership_lookup_optimization=True,
                    dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                    borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
                    use_lookup_optimization=True,
                    inherited_use_lookup_optimization=True, compiler_scratch_added=True,
                    borrow_lookup_test_filter=L.BORROW_LOOKUP_PREFIX, borrow_lookup_tests=list(L.BORROW_LOOKUP_NAMES),
                    cfg_expansion_test_filter=L.CFG_EXPANSION_PREFIX, cfg_expansion_tests=list(L.CFG_EXPANSION_NAMES),
                    use_lookup_test_filter=L.USE_LOOKUP_PREFIX, use_lookup_tests=list(L.USE_LOOKUP_NAMES),
                    census_test_filter=L.CENSUS_PREFIX, census_tests=list(L.CENSUS_NAMES),
                    semantic_predicates_changed=False,
                    actual_failure_caller_identified=False, baseline_failure_caller_identified=True, baseline_failure_block_count_observed=True,
                    baseline_memory_bounds_preflight_refusal=True,
                    baseline_rendered_blocks=567, baseline_rendered_edges=1122,
                    baseline_rendered_operations=2240, baseline_guard_candidates=552,
                    baseline_intersection_work_upper_bound=9340170, baseline_memory_bounds_work_limit=8388608,
                    baseline_runtime_work_exhaustion_observed=False,
                    baseline_edge_verdict_observed=False,
                    membership_test_filter=L.MEMBERSHIP_PREFIX, membership_tests=list(L.MEMBERSHIP_NAMES),
                    inherited_graph_work_diagnostics=True, dag_test_filter=L.DAG_PREFIX,
                    dag_tests=list(L.DAG_NAMES), graph_work_diagnostics=True,
                    graph_work_test_filter=L.GRAPH_WORK_PREFIX, graph_work_tests=list(L.GRAPH_WORK_NAMES), cfg_diagnostics_retained=True,
                 capacity_limits=copy.deepcopy(L.CAPACITY_LIMITS),
                 capacity_cohorts=copy.deepcopy(L.CAPACITY_COHORTS),
                 rlib_cpu_provenance=artifacts['backend-rlib'],
                 test_elf_cpu_provenance={role: artifacts[role] for role in
                     ('pliron-lib', 'compiler-lib', 'atomic-extraction', 'matrix-extraction')},
                 omitted_artifact_bodies_rehashed=False)
    audit_input = dict(schema='ferric-guarded-mlp-memory-bounds-dag-tool-audit-input-v1',
                       controller=pin(L.E / 'audit_guarded_mlp_memory_bounds_dag_compiler_tools_v228_v1.py', '4' * 64),
                       tool_manifest=pin(L.TOOLS / 'manifest.json'),
                       memory_bounds_dag_complete=proof['complete'], memory_bounds_dag_input=proof['input'],
                       memory_bounds_dag_sources=proof['sources'], memory_bounds_dag_final_build=proof['final_build'])
    value = dict(schema='ferric-guarded-mlp-memory-bounds-dag-compiler-tool-audit-v1', passed=True,
                 failure=None, postcheck_errors=[], actual_library_audits_replayed=True,
                 compiler_invocation=False, producer_receipt_source_and_final_cargo_joins_replayed=True,
                 rlib_deployed=False, rlib_readelf_or_ldd_invoked=False,
                 tool_manifest=audit_input['tool_manifest'], tools=tools,
                 phases=[dict(label=name + '-' + phase) for name in L.NAMES for phase in ('readelf', 'ldd')],
                 inputs={}, resolved_paths={}, qualified_producer=proof,
                 diagnostic_build=False, diagnostic_only=False,
                    diagnostic_changes_admission=False, cfg_expansion_diagnostics=True,
                    inherited_cfg_expansion_diagnostics=True, memory_bounds_dag_optimization=True,
                    inherited_cfg_linear_fusion_optimization=True, cfg_linear_fusion_optimization=True,
                    cfg_compaction_optimization=True, inherited_cfg_compaction_optimization=True,
                    cfg_compaction_test_filter=L.CFG_COMPACTION_PREFIX, cfg_compaction_tests=list(L.CFG_COMPACTION_NAMES),
                    cfg_linear_fusion_test_filter=L.CFG_LINEAR_FUSION_PREFIX, cfg_linear_fusion_tests=list(L.CFG_LINEAR_FUSION_NAMES),
                    memory_bounds_dag_test_filter=L.MEMORY_BOUNDS_DAG_PREFIX, memory_bounds_dag_tests=list(L.MEMORY_BOUNDS_DAG_NAMES),
                    static_failure_site_identified=True, actual_failure_block_count_observed=False,
                 admission_changed=True, structural_capacity_expansion=False, inherited_capacity_expansion=True,
                    structural_limits_changed=False, graph_analysis_optimization=True, inherited_graph_analysis_optimization=True,
                    resource_admission_may_change=True, membership_lookup_optimization=True, inherited_membership_lookup_optimization=True,
                    dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                    borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
                    use_lookup_optimization=True,
                    inherited_use_lookup_optimization=True, compiler_scratch_added=True,
                    borrow_lookup_test_filter=L.BORROW_LOOKUP_PREFIX, borrow_lookup_tests=list(L.BORROW_LOOKUP_NAMES),
                    cfg_expansion_test_filter=L.CFG_EXPANSION_PREFIX, cfg_expansion_tests=list(L.CFG_EXPANSION_NAMES),
                    use_lookup_test_filter=L.USE_LOOKUP_PREFIX, use_lookup_tests=list(L.USE_LOOKUP_NAMES),
                    census_test_filter=L.CENSUS_PREFIX, census_tests=list(L.CENSUS_NAMES),
                    semantic_predicates_changed=False,
                    actual_failure_caller_identified=False, baseline_failure_caller_identified=True, baseline_failure_block_count_observed=True,
                    baseline_memory_bounds_preflight_refusal=True,
                    baseline_rendered_blocks=567, baseline_rendered_edges=1122,
                    baseline_rendered_operations=2240, baseline_guard_candidates=552,
                    baseline_intersection_work_upper_bound=9340170, baseline_memory_bounds_work_limit=8388608,
                    baseline_runtime_work_exhaustion_observed=False,
                    baseline_edge_verdict_observed=False,
                    membership_test_filter=L.MEMBERSHIP_PREFIX, membership_tests=list(L.MEMBERSHIP_NAMES),
                    inherited_graph_work_diagnostics=True, dag_test_filter=L.DAG_PREFIX,
                    dag_tests=list(L.DAG_NAMES), graph_work_diagnostics=True,
                    graph_work_test_filter=L.GRAPH_WORK_PREFIX, graph_work_tests=list(L.GRAPH_WORK_NAMES),
                 cfg_diagnostics_retained=True, capacity_limits=copy.deepcopy(L.CAPACITY_LIMITS),
                 qualified_baseline_producer=dict(complete=pin('/synthetic/base.json', L.BASE_COMPLETE_SHA)),
                 input_manifest=pin(L.E / 'guarded-mlp-memory-bounds-dag-compiler-tools-audit-inputs-v228-v1.json'),
                 audits={'fe2o3-rustc-extract': {'libraries': {'librustc_codegen_fe2o3.so': {
                     'pin': tools['librustc_codegen_fe2o3.so']['deployed']}}}})
    docs = {str(L.AUDIT / 'complete.json'): value, value['input_manifest']['path']: audit_input,
            proof['complete']['path']: producer, str(L.TOOLS / 'manifest.json'): manifest}

    previous = copy.deepcopy(manifest)
    previous_pin = pin('/synthetic/previous-manifest.json', L.PREVIOUS_MANIFEST_SHA)
    source_rows = {'fe2o3/file-' + str(i): pin(L.DRIVER / ('fe2o3/file-' + str(i)))
                   for i in range(5299)}
    driver_artifacts = {role: dict(pin=pin(L.DRIVER / 'target' / role), cargo_artifact={})
                        for role in ('driver', 'test')}
    driver_sources_pin = pin(L.DRIVER / 'evidence/sources-before.json')
    selected = sorted('engineering_hsaco::tests::test_' + str(i) for i in range(18))
    driver = dict(schema='ferric-guarded-mlp-driver-cpu-v1',
                  controller=pin(L.DRIVER / 'run_cpu.py', L.DRIVER_CONTROLLER_SHA),
                  source_generation=L.DRIVER_GENERATION, passed=True, failure=None,
                  postcheck_errors=[], source_unchanged=True, final_artifact_phase='driver-build',
                  test_artifact_phase='driver-tests-build', full_bin_tests_executed=False,
                  test_scope='engineering_hsaco::tests::', engineering_tests=selected,
                  inventory=sorted([*selected, 'other::test']),
                  tests={'driver': dict(names=selected, passed=18, failed=0, ignored=0, filtered_out=1)},
                  input_manifest=pin(L.DRIVER / 'input-manifest.json'),
                  input_sources=source_rows, final_sources=source_rows, artifacts=driver_artifacts,
                  raw={'sources-before.json': driver_sources_pin,
                       'driver-build.stdout': pin(L.DRIVER / 'evidence/driver-build.stdout'),
                       'driver-tests-build.stdout': pin(L.DRIVER / 'evidence/driver-tests-build.stdout')})
    driver_proof = dict(complete=pin(L.DRIVER / 'evidence/complete.json', L.DRIVER_COMPLETE_SHA),
                        input=driver['input_manifest'], sources=driver_sources_pin,
                        final_build=driver['raw']['driver-build.stdout'],
                        test_build=driver['raw']['driver-tests-build.stdout'],
                        driver=driver_artifacts['driver'], test_elf_cpu_provenance=driver_artifacts['test'],
                        omitted_test_elf_body_rehashed=False)
    product = driver_artifacts['driver']['pin']
    manifest['bin/cargo-fe2o3'] = dict(source=product['path'], bytes=product['bytes'], sha256=product['sha256'])
    tools['cargo-fe2o3']['original'] = manifest['bin/cargo-fe2o3']
    driver_manifest = copy.deepcopy(manifest)
    driver_manifest_pin = pin('/synthetic/driver-manifest.json', L.DRIVER_MANIFEST_SHA)
    audit_input['driver_tool_manifest'] = driver_manifest_pin
    value['driver_tool_manifest'] = driver_manifest_pin
    docs[driver_manifest_pin['path']] = driver_manifest
    audit_input.update(previous_tool_manifest=previous_pin, driver_complete=driver_proof['complete'],
                       driver_input=driver_proof['input'], driver_sources=driver_proof['sources'],
                       driver_final_build=driver_proof['final_build'], driver_test_build=driver_proof['test_build'])
    value.update(previous_tool_manifest=previous_pin, qualified_driver=driver_proof,
                 driver_receipt_source_and_final_cargo_joins_replayed=True)
    driver_input = dict(schema='ferric-guarded-mlp-driver-cpu-input-v1',
                        source_generation=L.DRIVER_GENERATION,
                        files={name: {key: row[key] for key in ('bytes', 'sha256')}
                               for name, row in source_rows.items()})
    docs.update({previous_pin['path']: previous, driver_proof['complete']['path']: driver,
                 driver_sources_pin['path']: source_rows, driver['input_manifest']['path']: driver_input})
    lineage = producer['source_lineage']
    docs[lineage['baseline_failure']['path']] = copy.deepcopy(BASELINE_FAILURE)
    docs['baseline_stderr_body'] = BASELINE_STDERR
    return value, audit_input, producer, docs


class AuditBindingTests(unittest.TestCase):
    def admit(self, items):
        value, audit_input, producer, docs = items
        with mock.patch.object(L, 'AUDIT_CONTROLLER_SHA', '4' * 64), \
                mock.patch.object(L, 'doc', side_effect=lambda _h, path, _expected=None: docs[str(path)]), \
                mock.patch.object(L, 'record', side_effect=lambda _h, _path, expected=None: expected) as record, \
                mock.patch.object(L, 'read', side_effect=lambda path: docs['baseline_stderr_body']
                                  if str(path) == str(L.BASE_FAILURE_ROOT / 'compile.stderr')
                                  else (_ for _ in ()).throw(AssertionError('unexpected raw read'))) as raw_read, \
                mock.patch.object(L, 'raw_replay') as replay:
            result = L.audit_admission(None)
            replay.assert_called_once_with(None, value, L.AUDIT)
            recorded = [str(call.args[1]) for call in record.call_args_list]
            self.assertNotIn(producer['artifacts']['backend-rlib']['pin']['path'], recorded)
            for field in ('memory_bounds_dag_complete', 'memory_bounds_dag_input', 'memory_bounds_dag_sources', 'memory_bounds_dag_final_build'):
                self.assertIn(audit_input[field]['path'], recorded)
            self.assertIn(producer['source_lineage']['baseline_failure_stderr']['path'], recorded)
            raw_read.assert_called_once_with(L.BASE_FAILURE_ROOT / 'compile.stderr')
            return result

    def test_new_filepin_and_final_producer_join(self):
        items = fixture()
        self.assertEqual(items[1]['schema'],
                         'ferric-guarded-mlp-memory-bounds-dag-tool-audit-input-v1')
        self.assertEqual(items[2]['helper'], dict(
            path=str(L.PRODUCER / 'qualification_helpers.py'),
            bytes=L.PRODUCER_HELPER_BYTES, sha256=L.PRODUCER_HELPER_SHA))
        self.assertIs(self.admit(items), items[0])

    def test_old_audit_schema_or_missing_producer_replay_refused(self):
        for field, bad in (('schema', 'ferric-guarded-mlp-compiler-tool-audit-v1'),
                           ('producer_receipt_source_and_final_cargo_joins_replayed', False),
                           ('rlib_deployed', True), ('rlib_readelf_or_ldd_invoked', True)):
            items = fixture()
            items[0][field] = bad
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                self.admit(items)

        items = fixture()
        items[1]['schema'] = 'ferric-guarded-mlp-indexed-atomic-borrow-lookup-tool-audit-input-v1'
        with self.assertRaisesRegex(RuntimeError, 'reviewed actual loader input/controller generation'):
            self.admit(items)

    def test_unreviewed_loader_or_producer_controller_refused(self):
        for index in (1, 2):
            items = fixture()
            items[index]['controller']['sha256'] = '2' * 64
            with self.subTest(index=index), self.assertRaises(RuntimeError):
                self.admit(items)

        for field, bad in (
                ('sha256', 'a77cc7f82ebdd9518185b30b837e250abc68cc32e4858ab1ad3dc83a0ec8efe6'),
                ('sha256', '0' * 64), ('bytes', L.PRODUCER_HELPER_BYTES - 1),
                ('path', '/earlier/qualification_helpers.py')):
            items = fixture()
            self.assertNotEqual(items[2]['helper'][field], bad)
            items[2]['helper'][field] = bad
            with self.subTest(helper_field=field, bad=bad), self.assertRaises(RuntimeError):
                self.admit(items)

    def test_failed_or_earlier_producer_refused(self):
        for field, bad in (('passed', False), ('source_unchanged', False),
                           ('final_compiler_product_phase', 'compiler-products')):
            items = fixture()
            items[2][field] = bad
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                self.admit(items)

    def test_loader_input_readset_drift_refused(self):
        for field in ('memory_bounds_dag_complete', 'memory_bounds_dag_input', 'memory_bounds_dag_sources', 'memory_bounds_dag_final_build'):
            items = fixture()
            items[1][field] = dict(items[1][field], sha256='2' * 64)
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                self.admit(items)

    def test_deployed_body_must_match_final_not_earlier_product(self):
        for role in ('backend', 'extractor'):
            items = fixture()
            items[2]['artifacts'][role]['pin']['sha256'] = '2' * 64
            with self.subTest(role=role), self.assertRaises(RuntimeError):
                self.admit(items)

    def test_pending_bindings_refuse_before_any_helper_or_effect(self):
        for field in ('CPU_SHA', 'HELPER_SHA', 'AUDIT_SHA', 'VENDOR_CONTROLLER_SHA', 'AUDIT_CONTROLLER_SHA', 'DRIVER_CONTROLLER_SHA',
                      'PRODUCER_CONTROLLER_SHA', 'MEMORY_BOUNDS_DAG_PROPOSAL_SHA', 'PRODUCER_INPUT_SHA',
                      'PRODUCER_HELPER_SHA'):
            with mock.patch.object(L.sys, 'argv', ['lowering.py', '1' * 64]), \
                    mock.patch.object(L.sys, 'dont_write_bytecode', True), \
                    mock.patch.object(L, 'CPU_SHA', '6' * 64), \
                    mock.patch.object(L, 'HELPER_SHA', '7' * 64), \
                    mock.patch.object(L, 'AUDIT_SHA', '2' * 64), \
                    mock.patch.object(L, 'VENDOR_CONTROLLER_SHA', '3' * 64), \
                    mock.patch.object(L, 'AUDIT_CONTROLLER_SHA', '4' * 64), \
                    mock.patch.object(L, 'PRODUCER_INPUT_SHA', '5' * 64), \
                    mock.patch.object(L, field, None), mock.patch.object(L, 'helper') as helper, \
                    mock.patch.object(L.os, 'umask') as umask:
                with self.subTest(field=field), self.assertRaisesRegex(RuntimeError, 'bindings are still pending'):
                    L.main()
                helper.assert_not_called()
                umask.assert_not_called()


    def test_new_driver_replay_and_metadata_only_scope_are_required(self):
        for field, bad in (('driver_receipt_source_and_final_cargo_joins_replayed', False),
                           ('previous_tool_manifest', dict(path='/bad', bytes=1, sha256='0'*64))):
            items = fixture()
            items[0][field] = bad
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                self.admit(items)
        items = fixture()
        items[0]['qualified_driver']['omitted_test_elf_body_rehashed'] = True
        with self.assertRaises(RuntimeError):
            self.admit(items)

    def test_driver_failure_controller_phase_or_full_suite_claim_refused(self):
        for field, bad in (('passed', False), ('source_unchanged', False),
                           ('final_artifact_phase', 'driver-tests-build'),
                           ('full_bin_tests_executed', True), ('test_scope', 'all')):
            items = fixture()
            items[3][items[0]['qualified_driver']['complete']['path']][field] = bad
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                self.admit(items)
        items = fixture()
        items[3][items[0]['qualified_driver']['complete']['path']]['controller']['sha256'] = '9'*64
        with self.assertRaises(RuntimeError):
            self.admit(items)

    def test_driver_readset_or_final_body_drift_refused(self):
        for key in ('driver_complete', 'driver_input', 'driver_sources',
                    'driver_final_build', 'driver_test_build'):
            items = fixture()
            items[1][key] = dict(items[1][key], sha256='9'*64)
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                self.admit(items)
        items = fixture()
        driver = items[3][items[0]['qualified_driver']['complete']['path']]
        driver['artifacts']['driver']['pin']['sha256'] = '9'*64
        with self.assertRaises(RuntimeError):
            self.admit(items)

    def test_each_of_six_other_tools_stays_exact(self):
        for name in set(L.NAMES) - {'cargo-fe2o3'}:
            items = fixture()
            items[3][str(L.TOOLS / 'manifest.json')]['bin/' + name]['source'] = '/wrong'
            with self.subTest(name=name), self.assertRaises(RuntimeError):
                self.admit(items)

    def test_cfg_expansion_lineage_and_compaction_are_required(self):
        for field, bad in (('diagnostic_build', True), ('diagnostic_only', True), ('admission_changed', False),
                           ('structural_capacity_expansion', True), ('cfg_diagnostics_retained', False),
                           ('inherited_capacity_expansion', False), ('structural_limits_changed', True),
                           ('graph_analysis_optimization', False), ('inherited_graph_analysis_optimization', False), ('resource_admission_may_change', False), ('membership_lookup_optimization', False),
                             ('inherited_membership_lookup_optimization', False),
                             ('dead_cast_census_optimization', False), ('compiler_scratch_added', False),
                             ('inherited_dead_cast_census_optimization', False), ('borrow_lookup_optimization', False), ('inherited_borrow_lookup_optimization', False),
                             ('use_lookup_optimization', False), ('inherited_use_lookup_optimization', False),
                             ('use_lookup_test_filter', 'wrong'), ('use_lookup_tests', []),
                             ('borrow_lookup_test_filter', 'wrong'), ('borrow_lookup_tests', []),
                             ('cfg_expansion_test_filter', 'wrong'), ('cfg_expansion_tests', []),
                             ('cfg_expansion_diagnostics', False), ('diagnostic_changes_admission', True),
                             ('static_failure_site_identified', False), ('actual_failure_block_count_observed', True),
                             ('census_test_filter', 'wrong'), ('census_tests', []),
                           ('semantic_predicates_changed', True), ('actual_failure_caller_identified', True),
                           ('baseline_failure_caller_identified', False),
                             ('baseline_failure_block_count_observed', False), ('baseline_memory_bounds_preflight_refusal', False),
                             ('baseline_rendered_blocks', 566), ('baseline_guard_candidates', 551),
                             ('baseline_rendered_operations', 2239), ('baseline_intersection_work_upper_bound', 9340169),
                             ('baseline_memory_bounds_work_limit', 8388607), ('baseline_runtime_work_exhaustion_observed', True),
                         ('baseline_rendered_edges', 1121), ('baseline_edge_verdict_observed', True),
                         ('cfg_compaction_optimization', False), ('inherited_cfg_compaction_optimization', False),
                         ('cfg_compaction_test_filter', 'wrong'), ('cfg_compaction_tests', []),
                             ('cfg_linear_fusion_optimization', False), ('inherited_cfg_linear_fusion_optimization', False),
                             ('memory_bounds_dag_optimization', False), ('memory_bounds_dag_test_filter', 'wrong'), ('memory_bounds_dag_tests', []), ('inherited_cfg_expansion_diagnostics', False),
                             ('cfg_linear_fusion_test_filter', 'wrong'), ('cfg_linear_fusion_tests', []), ('inherited_graph_work_diagnostics', False),
                           ('membership_test_filter', 'wrong'), ('membership_tests', []),
                           ('dag_test_filter', 'wrong'), ('dag_tests', []), ('graph_work_diagnostics', False),
                           ('graph_work_test_filter', 'wrong'), ('graph_work_tests', []),
                           ('tests_passed', 2912), ('tests_ignored', 24), ('phases', [{}] * 40)):
            items = fixture()
            self.assertNotEqual(items[2][field], bad, field)
            items[2][field] = bad
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                self.admit(items)
        for section, key in (('source_lineage', 'memory_bounds_dag_proposal'), ('source_lineage', 'base_complete'),
                             ('source_lineage', 'base_sources'), ('source_lineage', 'base_input'),
                             ('source_lineage', 'baseline_failure'), ('source_lineage', 'baseline_failure_stderr')):
            items = fixture()
            items[2][section][key]['sha256'] = '9' * 64
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                self.admit(items)
        items = fixture()
        items[0]['admission_changed'] = False
        with self.assertRaises(RuntimeError):
            self.admit(items)

        for key, bad in (('passed', True), ('artifact', {}), ('source_unchanged', False),
                         ('postcheck_errors', ['drift']), ('automatic_retries', 1),
                         ('input_byte_maps_rechecked', False), ('gpu_execution', True)):
            items = fixture()
            prior = items[3][str(L.BASE_FAILURE_ROOT / 'failed.json')]
            prior[key] = bad
            with self.subTest(baseline_receipt=key), self.assertRaises(RuntimeError):
                self.admit(items)
        for key, bad in (('exit_code', 0), ('natural_exit', False), ('reaped', False),
                         ('process_group_absent', False), ('forced_cleanup', True),
                         ('timed_out', True), ('exception', 'failure'), ('observed_signals', [15])):
            items = fixture()
            prior = items[3][str(L.BASE_FAILURE_ROOT / 'failed.json')]
            prior['phases'][0][key] = bad
            with self.subTest(baseline_phase=key), self.assertRaises(RuntimeError):
                self.admit(items)
        for body in (b'', BASELINE_STDERR.replace(b'memory-bounds work hard limit', b'wrong failure'),
                     BASELINE_STDERR + L.BASE_FAILURE_DIAGNOSTIC):
            items = fixture()
            items[3]['baseline_stderr_body'] = body
            with self.subTest(baseline_stderr=body), self.assertRaises(RuntimeError):
                self.admit(items)

    def test_capacity_limits_and_cohorts_cannot_drift(self):
        source = ast.parse(Path(L.__file__).read_bytes())
        main = next(node for node in source.body if isinstance(node, ast.FunctionDef)
                    and node.name == 'main')
        result = next(node.value for node in main.body if isinstance(node, ast.Assign)
                      and any(isinstance(target, ast.Name) and target.id == 'result'
                              for target in node.targets))
        self.assertIsInstance(result, ast.Call)
        self.assertEqual(result.func.id, 'dict')
        fields = {field.arg: field.value for field in result.keywords}
        flags = dict(diagnostic_build=False, diagnostic_only=False,
                    diagnostic_changes_admission=False, cfg_expansion_diagnostics=True,
                    inherited_cfg_expansion_diagnostics=True, memory_bounds_dag_optimization=True,
                    inherited_cfg_linear_fusion_optimization=True, cfg_linear_fusion_optimization=True,
                    cfg_compaction_optimization=True, inherited_cfg_compaction_optimization=True,
                    static_failure_site_identified=True, actual_failure_block_count_observed=False, admission_changed=True,
                     structural_capacity_expansion=False, inherited_capacity_expansion=True,
                     structural_limits_changed=False, graph_analysis_optimization=True,
                     inherited_graph_analysis_optimization=True, resource_admission_may_change=True,
                     membership_lookup_optimization=True, inherited_membership_lookup_optimization=True,
                    dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                    borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
                    use_lookup_optimization=True,
                    inherited_use_lookup_optimization=True, compiler_scratch_added=True,
                    semantic_predicates_changed=False,
                     actual_failure_caller_identified=False,
                     baseline_edge_verdict_observed=False, baseline_runtime_work_exhaustion_observed=False,
                     inherited_graph_work_diagnostics=True, graph_work_diagnostics=True,
                     cfg_diagnostics_retained=True)
        for field, expected in flags.items():
            self.assertIs(ast.literal_eval(fields[field]), expected, field)
        for name in ('baseline_failure_caller_identified', 'baseline_failure_block_count_observed', 'baseline_memory_bounds_preflight_refusal'):
            self.assertEqual(ast.dump(fields[name]), ast.dump(ast.parse('audit is not None', mode='eval').body))
        for name, count in (('baseline_rendered_blocks', 567), ('baseline_rendered_edges', 1122),
                            ('baseline_rendered_operations', 2240), ('baseline_guard_candidates', 552),
                            ('baseline_intersection_work_upper_bound', 9340170), ('baseline_memory_bounds_work_limit', 8388608)):
            self.assertEqual(ast.dump(fields[name]),
                             ast.dump(ast.parse(str(count) + ' if audit is not None else None', mode='eval').body))
        self.assertEqual(fields['memory_bounds_dag_test_filter'].id, 'MEMORY_BOUNDS_DAG_PREFIX')
        self.assertEqual(ast.dump(fields['memory_bounds_dag_tests']),
                         ast.dump(ast.parse('list(MEMORY_BOUNDS_DAG_NAMES)', mode='eval').body))
        self.assertEqual(fields['cfg_linear_fusion_test_filter'].id, 'CFG_LINEAR_FUSION_PREFIX')
        self.assertEqual(ast.dump(fields['cfg_linear_fusion_tests']),
                         ast.dump(ast.parse('list(CFG_LINEAR_FUSION_NAMES)', mode='eval').body))
        self.assertEqual(fields['cfg_compaction_test_filter'].id, 'CFG_COMPACTION_PREFIX')
        self.assertEqual(ast.dump(fields['cfg_compaction_tests']),
                         ast.dump(ast.parse('list(CFG_COMPACTION_NAMES)', mode='eval').body))
        self.assertEqual(fields['cfg_expansion_test_filter'].id, 'CFG_EXPANSION_PREFIX')
        self.assertEqual(ast.dump(fields['cfg_expansion_tests']),
                         ast.dump(ast.parse('list(CFG_EXPANSION_NAMES)', mode='eval').body))
        self.assertEqual(fields['borrow_lookup_test_filter'].id, 'BORROW_LOOKUP_PREFIX')
        self.assertEqual(ast.dump(fields['borrow_lookup_tests']),
                         ast.dump(ast.parse('list(BORROW_LOOKUP_NAMES)', mode='eval').body))
        self.assertEqual(fields['use_lookup_test_filter'].id, 'USE_LOOKUP_PREFIX')
        self.assertEqual(ast.dump(fields['use_lookup_tests']),
                         ast.dump(ast.parse('list(USE_LOOKUP_NAMES)', mode='eval').body))
        self.assertEqual(fields['census_test_filter'].id, 'CENSUS_PREFIX')
        self.assertEqual(ast.dump(fields['census_tests']),
                         ast.dump(ast.parse('list(CENSUS_NAMES)', mode='eval').body))
        self.assertEqual(fields['membership_test_filter'].id, 'MEMBERSHIP_PREFIX')
        self.assertEqual(ast.dump(fields['membership_tests']),
                         ast.dump(ast.parse('list(MEMBERSHIP_NAMES)', mode='eval').body))
        self.assertEqual(fields['graph_work_test_filter'].id, 'GRAPH_WORK_PREFIX')
        self.assertEqual(ast.dump(fields['graph_work_tests']),
                         ast.dump(ast.parse('list(GRAPH_WORK_NAMES)', mode='eval').body))
        for section in ('producer', 'proof', 'audit'):
            for key in L.CAPACITY_LIMITS:
                items = fixture()
                row = {'producer': items[2], 'proof': items[0]['qualified_producer'],
                       'audit': items[0]}[section]
                row['capacity_limits'][key] += 1
                with self.subTest(section=section, key=key), self.assertRaises(RuntimeError):
                    self.admit(items)
            for key, bad in (('graph_analysis_optimization', False), ('inherited_graph_analysis_optimization', False),
                             ('resource_admission_may_change', False), ('membership_lookup_optimization', False),
                             ('inherited_membership_lookup_optimization', False),
                             ('dead_cast_census_optimization', False), ('compiler_scratch_added', False),
                             ('inherited_dead_cast_census_optimization', False), ('borrow_lookup_optimization', False), ('inherited_borrow_lookup_optimization', False),
                             ('use_lookup_optimization', False), ('inherited_use_lookup_optimization', False),
                             ('use_lookup_test_filter', 'wrong'), ('use_lookup_tests', []),
                             ('borrow_lookup_test_filter', 'wrong'), ('borrow_lookup_tests', []),
                             ('cfg_expansion_test_filter', 'wrong'), ('cfg_expansion_tests', []),
                             ('cfg_expansion_diagnostics', False), ('diagnostic_changes_admission', True),
                             ('static_failure_site_identified', False), ('actual_failure_block_count_observed', True),
                             ('census_test_filter', 'wrong'), ('census_tests', []),
                             ('semantic_predicates_changed', True),
                             ('actual_failure_caller_identified', True), ('baseline_failure_caller_identified', False),
                             ('baseline_failure_block_count_observed', False), ('baseline_memory_bounds_preflight_refusal', False),
                             ('baseline_rendered_blocks', 566), ('baseline_guard_candidates', 551),
                             ('baseline_rendered_operations', 2239), ('baseline_intersection_work_upper_bound', 9340169),
                             ('baseline_memory_bounds_work_limit', 8388607), ('baseline_runtime_work_exhaustion_observed', True),
                         ('baseline_rendered_edges', 1121), ('baseline_edge_verdict_observed', True),
                         ('cfg_compaction_optimization', False), ('inherited_cfg_compaction_optimization', False),
                         ('cfg_compaction_test_filter', 'wrong'), ('cfg_compaction_tests', []),
                             ('cfg_linear_fusion_optimization', False), ('inherited_cfg_linear_fusion_optimization', False),
                             ('memory_bounds_dag_optimization', False), ('memory_bounds_dag_test_filter', 'wrong'), ('memory_bounds_dag_tests', []), ('inherited_cfg_expansion_diagnostics', False),
                             ('cfg_linear_fusion_test_filter', 'wrong'), ('cfg_linear_fusion_tests', []),
                             ('inherited_graph_work_diagnostics', False),
                             ('membership_test_filter', 'wrong'), ('membership_tests', []),
                             ('dag_test_filter', 'wrong'), ('dag_tests', []), ('graph_work_diagnostics', False),
                           ('graph_work_test_filter', 'wrong'), ('graph_work_tests', [])):
                items = fixture()
                row = {'producer': items[2], 'proof': items[0]['qualified_producer'],
                       'audit': items[0]}[section]
                row[key] = bad
                with self.subTest(section=section, key=key), self.assertRaises(RuntimeError):
                    self.admit(items)
        for section in ('producer', 'proof'):
            items = fixture()
            row = items[2] if section == 'producer' else items[0]['qualified_producer']
            row['capacity_cohorts']['cfg-block-cap-pliron']['names'].pop()
            with self.subTest(section=section), self.assertRaises(RuntimeError):
                self.admit(items)

    def test_earlier_generation_cannot_supply_memory_bounds_dag_provenance(self):
        for section, field, bad in (
                ('producer', 'schema', 'ferric-guarded-mlp-indexed-atomic-borrow-lookup-cpu-v1'),
                ('audit', 'schema', 'ferric-guarded-mlp-indexed-atomic-borrow-lookup-compiler-tool-audit-v1'),
                ('proof', 'diagnostic_build', True), ('proof', 'diagnostic_only', True), ('proof', 'admission_changed', False),
                ('proof', 'structural_capacity_expansion', True),
                ('proof', 'cfg_diagnostics_retained', False)):
            items = fixture()
            row = {'producer': items[2], 'proof': items[0]['qualified_producer'],
                   'audit': items[0]}[section]
            row[field] = bad
            with self.subTest(section=section, field=field), self.assertRaises(RuntimeError):
                self.admit(items)

    def test_actual_previous_driver_generation_is_immutable(self):
        for where in ('manifest', 'receipt', 'baseline'):
            items = fixture()
            if where == 'manifest':
                items[0]['driver_tool_manifest']['sha256'] = '9' * 64
            elif where == 'receipt':
                items[0]['qualified_driver']['complete']['sha256'] = '9' * 64
            else:
                items[0]['qualified_baseline_producer']['complete']['sha256'] = '9' * 64
            with self.subTest(where=where), self.assertRaises(RuntimeError):
                self.admit(items)



def combined_cpu_fixture():
    names = tuple('synthetic::case_' + str(i).zfill(2) for i in range(39))
    helper = mock.Mock(COMMIT='synthetic-reviewed-commit', EXPECTED_TESTS=names)
    source = {'candidate/src/lib.rs': dict(path='/synthetic/lib.rs', bytes=10, sha256='1' * 64)}
    sources_pin = dict(path='/synthetic/final-sources.json', bytes=10, sha256='2' * 64)
    tested_pin = dict(path='/synthetic/tested-sources.json', bytes=10, sha256='3' * 64)
    input_pin = dict(path='/synthetic/input.json', bytes=10, sha256='4' * 64)
    tests = dict(names=list(names), passed=39, failed=0, ignored=0)
    value = dict(
        schema='ferric-guarded-mlp-combined-state-cpu-v1', passed=True, failure=None,
        postcheck_errors=[], source_unchanged=True, dependency_commit=helper.COMMIT,
        controller=dict(sha256='7' * 64), gpu_execution=False, compiler_hsaco_reproduced=False,
        phases=[dict(label=name) for name in (
            'rustc-version', 'format', 'format-check', 'metadata', 'host-check',
            'build-tests', 'lib-list', 'lib-ignored', 'lib-tests', 'host-build')],
        final_sources=sources_pin, tested_sources=tested_pin, input_manifest=input_pin,
        tests=copy.deepcopy(tests), tool_pins={}, artifacts={})
    docs = {str(L.CPU / 'evidence/complete.json'): value,
            sources_pin['path']: copy.deepcopy(source),
            tested_pin['path']: copy.deepcopy(source),
            input_pin['path']: dict(expected_tests=list(names))}
    helper.sources.return_value = copy.deepcopy(source)
    helper.inventory.side_effect = lambda text: list(names) if text == 'inventory' else []
    helper.test_outcomes.return_value = tests
    return helper, value, docs, source


def combined_vendor_fixture():
    helper = mock.Mock()
    cpu_pin = dict(path=str(L.CPU / 'evidence/complete.json'), bytes=10, sha256='1' * 64)
    source = {'candidate/src/lib.rs': dict(bytes=10, sha256='2' * 64)}
    rust = {'library/core/src/lib.rs': dict(bytes=10, sha256='3' * 64)}
    files = {'package/src/lib.rs': dict(bytes=10, sha256='4' * 64)}
    files_pin = dict(path='/synthetic/vendor-files.json', bytes=10, sha256='5' * 64)
    value = dict(
        schema='ferric-guarded-mlp-combined-state-vendor-preparation-v1',
        passed=True, failure=None, postcheck_errors=[], qualified_cpu_complete=cpu_pin,
        controller=dict(sha256='6' * 64), input_sources_unchanged=True,
        config_installed=False, offline=True, host_fixture_crate_binding_used=False,
        vendor_directory=str(L.VENDOR), phases=[dict(label='vendor')], inputs={},
        vendor_files=files_pin, generated_config='synthetic-closed-config')
    directory = L.VENDOR_ROOT / 'evidence'
    docs = {str(directory / 'complete.json'): value,
            str(directory / 'cpu-sources-before.json'): copy.deepcopy(source),
            str(directory / 'cpu-sources-after.json'): copy.deepcopy(source),
            str(directory / 'rust-src-before.json'): copy.deepcopy(rust),
            str(directory / 'rust-src-after.json'): copy.deepcopy(rust),
            files_pin['path']: copy.deepcopy(files)}
    return helper, value, docs, source, rust, files, cpu_pin


class CombinedCandidateAdmissionTests(unittest.TestCase):
    def admit_cpu(self, items):
        helper, value, docs, source = items
        with mock.patch.object(L, 'HELPER_SHA', '7' * 64), \
                mock.patch.object(L, 'doc', side_effect=lambda h, path, *args: docs[str(path)]), \
                mock.patch.object(L, 'raw_replay') as replay, \
                mock.patch.object(L, 'record'), \
                mock.patch.object(L, 'read', side_effect=lambda path: (
                    b'inventory' if Path(path).name == 'lib-list.stdout' else b'ignored')):
            result = L.cpu_admission(helper)
            replay.assert_called_once_with(helper, value, L.CPU / 'evidence')
            return result

    def admit_vendor(self, items):
        helper, value, docs, source, rust, files, cpu_pin = items
        with mock.patch.object(L, 'VENDOR_CONTROLLER_SHA', '6' * 64), \
                mock.patch.object(L, 'INPUTS', {str(L.CPU / 'evidence/complete.json'): cpu_pin}), \
                mock.patch.object(L, 'doc', side_effect=lambda h, path, *args: docs[str(path)]), \
                mock.patch.object(L, 'raw_replay') as replay, \
                mock.patch.object(L, 'record'), \
                mock.patch.object(L, 'tree', side_effect=lambda h, path: (
                    rust if path == L.RUST_SOURCE else files)), \
                mock.patch.object(L, 'git_sources', return_value=['synthetic-git']) as git:
            result = L.vendor_admission(helper, '8' * 64, source)
            replay.assert_called_once_with(helper, value, L.VENDOR_ROOT / 'evidence')
            git.assert_called_once_with(value['generated_config'])
            return result

    def test_exact_combined_cpu_generation_and_named_census(self):
        items = combined_cpu_fixture()
        self.assertEqual(self.admit_cpu(items), (items[1], items[3]))
        self.assertEqual(len(items[0].EXPECTED_TESTS), 39)

    def test_cpu_refuses_old_generation_helper_and_named_census(self):
        for field, bad in (
                ('schema', 'ferric-guarded-mlp-segment-cpu-v1'), ('passed', False),
                ('failure', 'failed'), ('postcheck_errors', ['drift']),
                ('source_unchanged', False), ('dependency_commit', 'wrong'),
                ('controller', dict(sha256='0' * 64)), ('gpu_execution', True),
                ('compiler_hsaco_reproduced', True)):
            items = combined_cpu_fixture()
            items[1][field] = bad
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                self.admit_cpu(items)
        for change in ('legacy-census', 'mutable-census', 'manifest-name', 'compiled-name', 'outcomes'):
            items = combined_cpu_fixture()
            helper, value, docs, _ = items
            if change == 'legacy-census':
                helper.EXPECTED_TESTS = helper.EXPECTED_TESTS[:27]
            elif change == 'mutable-census':
                helper.EXPECTED_TESTS = list(helper.EXPECTED_TESTS)
            elif change == 'manifest-name':
                docs[value['input_manifest']['path']]['expected_tests'][0] = 'substituted'
            elif change == 'compiled-name':
                helper.inventory.side_effect = lambda text: ['substituted'] if text == 'inventory' else []
            else:
                helper.test_outcomes.return_value = dict(value['tests'], passed=38)
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                self.admit_cpu(items)

    def test_cpu_refuses_phase_source_and_ignore_drift(self):
        for change in ('phase-order', 'phase-missing', 'final-source', 'tested-source', 'actual-source', 'ignored'):
            items = combined_cpu_fixture()
            helper, value, docs, _ = items
            if change == 'phase-order':
                value['phases'][0], value['phases'][1] = value['phases'][1], value['phases'][0]
            elif change == 'phase-missing':
                value['phases'].pop()
            elif change in ('final-source', 'tested-source'):
                field = 'final_sources' if change == 'final-source' else 'tested_sources'
                docs[value[field]['path']]['candidate/src/lib.rs']['sha256'] = '9' * 64
            elif change == 'actual-source':
                helper.sources.return_value = {}
            else:
                helper.inventory.side_effect = lambda text: list(helper.EXPECTED_TESTS)
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                self.admit_cpu(items)

    def test_exact_combined_vendor_generation_and_source_rosters(self):
        items = combined_vendor_fixture()
        self.assertEqual(self.admit_vendor(items), (items[1], items[5], items[4], ['synthetic-git']))

    def test_vendor_refuses_old_generation_cpu_controller_and_pending_binding(self):
        for field, bad in (
                ('schema', 'ferric-guarded-mlp-vendor-preparation-v1'), ('passed', False),
                ('failure', 'failed'), ('postcheck_errors', ['drift']),
                ('qualified_cpu_complete', dict(path='/wrong', bytes=10, sha256='0' * 64)),
                ('controller', dict(sha256='0' * 64)), ('input_sources_unchanged', False),
                ('config_installed', True), ('offline', False),
                ('host_fixture_crate_binding_used', True), ('vendor_directory', '/wrong'),
                ('phases', [])):
            items = combined_vendor_fixture()
            items[1][field] = bad
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                self.admit_vendor(items)
        with mock.patch.object(L, 'VENDOR_CONTROLLER_SHA', None), mock.patch.object(L, 'doc') as doc:
            with self.assertRaisesRegex(RuntimeError, 'combined-state vendor controller binding still pending'):
                L.vendor_admission(mock.Mock(), '8' * 64, {})
            doc.assert_not_called()

    def test_vendor_refuses_cpu_rust_and_vendor_snapshot_drift(self):
        for suffix in ('cpu-sources-before.json', 'cpu-sources-after.json',
                       'rust-src-before.json', 'rust-src-after.json', 'vendor-files'):
            items = combined_vendor_fixture()
            _, value, docs, _, _, _, _ = items
            path = value['vendor_files']['path'] if suffix == 'vendor-files' else str(
                L.VENDOR_ROOT / 'evidence' / suffix)
            docs[path] = {}
            with self.subTest(snapshot=suffix), self.assertRaises(RuntimeError):
                self.admit_vendor(items)

if __name__ == '__main__':
    unittest.main()

