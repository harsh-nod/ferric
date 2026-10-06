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
        != (16503, '1bf26b28162414dfb4afbf6dc5ba5a70a60d1c1fe1fd55624c251733840c560b')
        or (len(BASELINE_STDERR), hashlib.sha256(BASELINE_STDERR).hexdigest())
        != (164209, '9a309b8f23c66ad29e98a464bcb3ed8480766e2c7fd807415616dc99d6121a05')):
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
    producer = dict(schema='ferric-guarded-mlp-ranked-cfg-linear-fusion-cpu-v1',
                    controller=pin(L.PRODUCER / 'run_cpu.py', L.PRODUCER_CONTROLLER_SHA),
                    passed=True, failure=None, source_unchanged=True, postcheck_errors=[],
                    final_compiler_product_phase='compiler-tests-build',
                    source_lineage=dict(cfg_linear_fusion_proposal=pin('/synthetic/proposal.json', L.CFG_LINEAR_FUSION_PROPOSAL_SHA),
                                        base_complete=pin('/synthetic/cap.json', L.CFG_LINEAR_FUSION_BASE_COMPLETE_SHA),
                                        base_sources=pin('/synthetic/cap-sources.json', L.CFG_LINEAR_FUSION_BASE_SOURCES_SHA),
                                        base_input=pin('/synthetic/cap-input.json', L.CFG_LINEAR_FUSION_BASE_INPUT_SHA),
                                        base_metadata={}, base_dependencies={}, base_streams={},
                                        cfg_linear_fusion_overlay={},
                                        baseline_failure=dict(path=str(L.BASE_FAILURE_ROOT / 'failed.json'),
                                                              bytes=16503, sha256=L.BASE_FAILURE_SHA),
                                        baseline_failure_stderr=dict(path=str(L.BASE_FAILURE_ROOT / 'compile.stderr'),
                                                                     bytes=164209, sha256=L.BASE_FAILURE_STDERR_SHA)),
                    diagnostic_build=False, diagnostic_only=False,
                    diagnostic_changes_admission=False, cfg_expansion_diagnostics=True,
                    inherited_cfg_expansion_diagnostics=True, cfg_linear_fusion_optimization=True,
                    cfg_compaction_optimization=True, inherited_cfg_compaction_optimization=True,
                    cfg_compaction_test_filter=L.CFG_COMPACTION_PREFIX, cfg_compaction_tests=list(L.CFG_COMPACTION_NAMES),
                    cfg_linear_fusion_test_filter=L.CFG_LINEAR_FUSION_PREFIX, cfg_linear_fusion_tests=list(L.CFG_LINEAR_FUSION_NAMES),
                    static_failure_site_identified=True, actual_failure_block_count_observed=False, admission_changed=True,
                    structural_capacity_expansion=False, inherited_capacity_expansion=True,
                    structural_limits_changed=False, graph_analysis_optimization=False, inherited_graph_analysis_optimization=True,
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
                    baseline_identity_first_refusal_blocks=1025, baseline_identity_block_limit=1024,
                    baseline_rendered_blocks=1675, baseline_rendered_edges=2230,
                    baseline_edge_verdict_observed=False,
                    membership_test_filter=L.MEMBERSHIP_PREFIX, membership_tests=list(L.MEMBERSHIP_NAMES),
                    inherited_graph_work_diagnostics=True, dag_test_filter=L.DAG_PREFIX,
                    dag_tests=list(L.DAG_NAMES), graph_work_diagnostics=True,
                    graph_work_test_filter=L.GRAPH_WORK_PREFIX, graph_work_tests=list(L.GRAPH_WORK_NAMES), cfg_diagnostics_retained=True,
                    capacity_limits=copy.deepcopy(L.CAPACITY_LIMITS),
                    capacity_cohorts=copy.deepcopy(L.CAPACITY_COHORTS),
                    phases=[{} for _ in range(44)], tests={str(i): {} for i in range(31)},
                    tests_passed=2972, tests_ignored=25,
                    artifacts=artifacts, input_manifest=pin(L.PRODUCER / 'input-manifest.json', L.PRODUCER_INPUT_SHA),
                    input_sources=pin(L.PRODUCER / 'evidence/sources-before.json'),
                    raw={'compiler-tests-build.stdout': pin(L.PRODUCER / 'evidence/compiler-tests-build.stdout')})
    proof = dict(complete=pin(L.PRODUCER / 'evidence/complete.json'),
                 input=producer['input_manifest'], sources=producer['input_sources'],
                 final_build=producer['raw']['compiler-tests-build.stdout'], source_lineage=copy.deepcopy(producer['source_lineage']),
                 diagnostic_build=False, diagnostic_only=False,
                    diagnostic_changes_admission=False, cfg_expansion_diagnostics=True,
                    inherited_cfg_expansion_diagnostics=True, cfg_linear_fusion_optimization=True,
                    cfg_compaction_optimization=True, inherited_cfg_compaction_optimization=True,
                    cfg_compaction_test_filter=L.CFG_COMPACTION_PREFIX, cfg_compaction_tests=list(L.CFG_COMPACTION_NAMES),
                    cfg_linear_fusion_test_filter=L.CFG_LINEAR_FUSION_PREFIX, cfg_linear_fusion_tests=list(L.CFG_LINEAR_FUSION_NAMES),
                    static_failure_site_identified=True, actual_failure_block_count_observed=False, admission_changed=True,
                 structural_capacity_expansion=False, inherited_capacity_expansion=True,
                    structural_limits_changed=False, graph_analysis_optimization=False, inherited_graph_analysis_optimization=True,
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
                    baseline_identity_first_refusal_blocks=1025, baseline_identity_block_limit=1024,
                    baseline_rendered_blocks=1675, baseline_rendered_edges=2230,
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
    audit_input = dict(schema='ferric-guarded-mlp-ranked-cfg-linear-fusion-tool-audit-input-v1',
                       controller=pin(L.E / 'audit_guarded_mlp_ranked_cfg_linear_fusion_compiler_tools_v228_v2.py', '4' * 64),
                       tool_manifest=pin(L.TOOLS / 'manifest.json'),
                       cfg_linear_fusion_complete=proof['complete'], cfg_linear_fusion_input=proof['input'],
                       cfg_linear_fusion_sources=proof['sources'], cfg_linear_fusion_final_build=proof['final_build'])
    value = dict(schema='ferric-guarded-mlp-ranked-cfg-linear-fusion-compiler-tool-audit-v1', passed=True,
                 failure=None, postcheck_errors=[], actual_library_audits_replayed=True,
                 compiler_invocation=False, producer_receipt_source_and_final_cargo_joins_replayed=True,
                 rlib_deployed=False, rlib_readelf_or_ldd_invoked=False,
                 tool_manifest=audit_input['tool_manifest'], tools=tools,
                 phases=[dict(label=name + '-' + phase) for name in L.NAMES for phase in ('readelf', 'ldd')],
                 inputs={}, resolved_paths={}, qualified_producer=proof,
                 diagnostic_build=False, diagnostic_only=False,
                    diagnostic_changes_admission=False, cfg_expansion_diagnostics=True,
                    inherited_cfg_expansion_diagnostics=True, cfg_linear_fusion_optimization=True,
                    cfg_compaction_optimization=True, inherited_cfg_compaction_optimization=True,
                    cfg_compaction_test_filter=L.CFG_COMPACTION_PREFIX, cfg_compaction_tests=list(L.CFG_COMPACTION_NAMES),
                    cfg_linear_fusion_test_filter=L.CFG_LINEAR_FUSION_PREFIX, cfg_linear_fusion_tests=list(L.CFG_LINEAR_FUSION_NAMES),
                    static_failure_site_identified=True, actual_failure_block_count_observed=False,
                 admission_changed=True, structural_capacity_expansion=False, inherited_capacity_expansion=True,
                    structural_limits_changed=False, graph_analysis_optimization=False, inherited_graph_analysis_optimization=True,
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
                    baseline_identity_first_refusal_blocks=1025, baseline_identity_block_limit=1024,
                    baseline_rendered_blocks=1675, baseline_rendered_edges=2230,
                    baseline_edge_verdict_observed=False,
                    membership_test_filter=L.MEMBERSHIP_PREFIX, membership_tests=list(L.MEMBERSHIP_NAMES),
                    inherited_graph_work_diagnostics=True, dag_test_filter=L.DAG_PREFIX,
                    dag_tests=list(L.DAG_NAMES), graph_work_diagnostics=True,
                    graph_work_test_filter=L.GRAPH_WORK_PREFIX, graph_work_tests=list(L.GRAPH_WORK_NAMES),
                 cfg_diagnostics_retained=True, capacity_limits=copy.deepcopy(L.CAPACITY_LIMITS),
                 qualified_baseline_producer=dict(complete=pin('/synthetic/base.json', L.BASE_COMPLETE_SHA)),
                 input_manifest=pin(L.E / 'guarded-mlp-ranked-cfg-linear-fusion-compiler-tools-audit-inputs-v228-v2.json'),
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
            for field in ('cfg_linear_fusion_complete', 'cfg_linear_fusion_input', 'cfg_linear_fusion_sources', 'cfg_linear_fusion_final_build'):
                self.assertIn(audit_input[field]['path'], recorded)
            self.assertIn(producer['source_lineage']['baseline_failure_stderr']['path'], recorded)
            raw_read.assert_called_once_with(L.BASE_FAILURE_ROOT / 'compile.stderr')
            return result

    def test_new_filepin_and_final_producer_join(self):
        items = fixture()
        self.assertEqual(items[1]['schema'],
                         'ferric-guarded-mlp-ranked-cfg-linear-fusion-tool-audit-input-v1')
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

    def test_failed_or_earlier_producer_refused(self):
        for field, bad in (('passed', False), ('source_unchanged', False),
                           ('final_compiler_product_phase', 'compiler-products')):
            items = fixture()
            items[2][field] = bad
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                self.admit(items)

    def test_loader_input_readset_drift_refused(self):
        for field in ('cfg_linear_fusion_complete', 'cfg_linear_fusion_input', 'cfg_linear_fusion_sources', 'cfg_linear_fusion_final_build'):
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
        for field in ('AUDIT_SHA', 'VENDOR_CONTROLLER_SHA', 'AUDIT_CONTROLLER_SHA', 'DRIVER_CONTROLLER_SHA',
                      'PRODUCER_CONTROLLER_SHA', 'CFG_LINEAR_FUSION_PROPOSAL_SHA', 'PRODUCER_INPUT_SHA'):
            with mock.patch.object(L.sys, 'argv', ['lowering.py', '1' * 64]), \
                    mock.patch.object(L.sys, 'dont_write_bytecode', True), \
                    mock.patch.object(L, 'AUDIT_SHA', '2' * 64), \
                    mock.patch.object(L, 'VENDOR_CONTROLLER_SHA', '3' * 64), \
                    mock.patch.object(L, 'AUDIT_CONTROLLER_SHA', '4' * 64), \
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
                           ('graph_analysis_optimization', True), ('inherited_graph_analysis_optimization', False), ('resource_admission_may_change', False), ('membership_lookup_optimization', False),
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
                             ('baseline_failure_block_count_observed', False), ('baseline_identity_first_refusal_blocks', 1024),
                             ('baseline_rendered_blocks', 1674), ('baseline_identity_block_limit', 1023),
                         ('baseline_rendered_edges', 2229), ('baseline_edge_verdict_observed', True),
                         ('cfg_compaction_optimization', False), ('inherited_cfg_compaction_optimization', False),
                         ('cfg_compaction_test_filter', 'wrong'), ('cfg_compaction_tests', []),
                             ('cfg_linear_fusion_optimization', False), ('inherited_cfg_expansion_diagnostics', False),
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
        for section, key in (('source_lineage', 'cfg_linear_fusion_proposal'), ('source_lineage', 'base_complete'),
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
        for body in (b'', BASELINE_STDERR.replace(b'blocks count 1025', b'blocks count 1024'),
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
                    inherited_cfg_expansion_diagnostics=True, cfg_linear_fusion_optimization=True,
                    cfg_compaction_optimization=True, inherited_cfg_compaction_optimization=True,
                    static_failure_site_identified=True, actual_failure_block_count_observed=False, admission_changed=True,
                     structural_capacity_expansion=False, inherited_capacity_expansion=True,
                     structural_limits_changed=False, graph_analysis_optimization=False,
                     inherited_graph_analysis_optimization=True, resource_admission_may_change=True,
                     membership_lookup_optimization=True, inherited_membership_lookup_optimization=True,
                    dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                    borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
                    use_lookup_optimization=True,
                    inherited_use_lookup_optimization=True, compiler_scratch_added=True,
                    semantic_predicates_changed=False,
                     actual_failure_caller_identified=False,
                     baseline_edge_verdict_observed=False,
                     inherited_graph_work_diagnostics=True, graph_work_diagnostics=True,
                     cfg_diagnostics_retained=True)
        for field, expected in flags.items():
            self.assertIs(ast.literal_eval(fields[field]), expected, field)
        for name in ('baseline_failure_caller_identified', 'baseline_failure_block_count_observed'):
            self.assertEqual(ast.dump(fields[name]), ast.dump(ast.parse('audit is not None', mode='eval').body))
        for name, count in (('baseline_identity_first_refusal_blocks', 1025),
                            ('baseline_identity_block_limit', 1024),
                            ('baseline_rendered_blocks', 1675), ('baseline_rendered_edges', 2230)):
            self.assertEqual(ast.dump(fields[name]),
                             ast.dump(ast.parse(str(count) + ' if audit is not None else None', mode='eval').body))
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
            for key, bad in (('graph_analysis_optimization', True), ('inherited_graph_analysis_optimization', False),
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
                             ('baseline_failure_block_count_observed', False), ('baseline_identity_first_refusal_blocks', 1024),
                             ('baseline_rendered_blocks', 1674), ('baseline_identity_block_limit', 1023),
                         ('baseline_rendered_edges', 2229), ('baseline_edge_verdict_observed', True),
                         ('cfg_compaction_optimization', False), ('inherited_cfg_compaction_optimization', False),
                         ('cfg_compaction_test_filter', 'wrong'), ('cfg_compaction_tests', []),
                             ('cfg_linear_fusion_optimization', False), ('inherited_cfg_expansion_diagnostics', False),
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

    def test_earlier_generation_cannot_supply_cfg_linear_fusion_provenance(self):
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


if __name__ == '__main__':
    unittest.main()
