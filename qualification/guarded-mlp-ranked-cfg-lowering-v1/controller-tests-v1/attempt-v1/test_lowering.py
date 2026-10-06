"""Synthetic new-audit joins only; no compiler, filesystem or auditor is run."""
import copy
import unittest
from unittest import mock

import lowering as L


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
    producer = dict(schema='ferric-guarded-mlp-ranked-cfg-diagnostic-cpu-v1',
                    controller=pin(L.PRODUCER / 'run_cpu.py', L.PRODUCER_CONTROLLER_SHA),
                    passed=True, failure=None, source_unchanged=True, postcheck_errors=[],
                    final_compiler_product_phase='compiler-tests-build',
                    source_lineage=dict(diagnostic_proposal=pin('/synthetic/proposal.json', L.DIAGNOSTIC_PROPOSAL_SHA),
                                        base_complete=pin('/synthetic/base.json', L.BASE_COMPLETE_SHA)),
                    diagnostic_build=True, diagnostic_changes_admission=False,
                    phases=[{} for _ in range(33)], tests={str(i): {} for i in range(20)},
                    tests_passed=2814, tests_ignored=25,
                    artifacts=artifacts, input_manifest=pin(L.PRODUCER / 'input-manifest.json'),
                    input_sources=pin(L.PRODUCER / 'evidence/sources-before.json'),
                    raw={'compiler-tests-build.stdout': pin(L.PRODUCER / 'evidence/compiler-tests-build.stdout')})
    proof = dict(complete=pin(L.PRODUCER / 'evidence/complete.json'),
                 input=producer['input_manifest'], sources=producer['input_sources'],
                 final_build=producer['raw']['compiler-tests-build.stdout'], source_lineage=copy.deepcopy(producer['source_lineage']),
                 diagnostic_build=True, diagnostic_changes_admission=False,
                 rlib_cpu_provenance=artifacts['backend-rlib'],
                 test_elf_cpu_provenance={role: artifacts[role] for role in
                     ('pliron-lib', 'compiler-lib', 'atomic-extraction', 'matrix-extraction')},
                 omitted_artifact_bodies_rehashed=False)
    audit_input = dict(schema='ferric-guarded-mlp-ranked-cfg-tool-audit-input-v1',
                       controller=pin(L.E / 'audit_guarded_mlp_ranked_cfg_compiler_tools_v228_v1.py', L.AUDIT_CONTROLLER_SHA),
                       tool_manifest=pin(L.TOOLS / 'manifest.json'),
                       diagnostic_complete=proof['complete'], diagnostic_input=proof['input'],
                       diagnostic_sources=proof['sources'], diagnostic_final_build=proof['final_build'])
    value = dict(schema='ferric-guarded-mlp-ranked-cfg-compiler-tool-audit-v1', passed=True,
                 failure=None, postcheck_errors=[], actual_library_audits_replayed=True,
                 compiler_invocation=False, producer_receipt_source_and_final_cargo_joins_replayed=True,
                 rlib_deployed=False, rlib_readelf_or_ldd_invoked=False,
                 tool_manifest=audit_input['tool_manifest'], tools=tools,
                 phases=[dict(label=name + '-' + phase) for name in L.NAMES for phase in ('readelf', 'ldd')],
                 inputs={}, resolved_paths={}, qualified_producer=proof,
                 diagnostic_changes_admission=False,
                 qualified_baseline_producer=dict(complete=pin('/synthetic/base.json', L.BASE_COMPLETE_SHA)),
                 input_manifest=pin(L.E / 'guarded-mlp-ranked-cfg-compiler-tools-audit-inputs-v228-v1.json'),
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
    return value, audit_input, producer, docs


class AuditBindingTests(unittest.TestCase):
    def admit(self, items):
        value, audit_input, producer, docs = items
        with mock.patch.object(L, 'doc', side_effect=lambda _h, path, _expected=None: docs[str(path)]), \
                mock.patch.object(L, 'record') as record, mock.patch.object(L, 'raw_replay') as replay:
            result = L.audit_admission(None)
            replay.assert_called_once_with(None, value, L.AUDIT)
            recorded = [str(call.args[1]) for call in record.call_args_list]
            self.assertNotIn(producer['artifacts']['backend-rlib']['pin']['path'], recorded)
            for field in ('diagnostic_complete', 'diagnostic_input', 'diagnostic_sources', 'diagnostic_final_build'):
                self.assertIn(audit_input[field]['path'], recorded)
            return result

    def test_new_filepin_and_final_producer_join(self):
        items = fixture()
        self.assertIs(self.admit(items), items[0])

    def test_old_audit_schema_or_missing_producer_replay_refused(self):
        for field, bad in (('schema', 'ferric-guarded-mlp-compiler-tool-audit-v1'),
                           ('producer_receipt_source_and_final_cargo_joins_replayed', False),
                           ('rlib_deployed', True), ('rlib_readelf_or_ldd_invoked', True)):
            items = fixture()
            items[0][field] = bad
            with self.subTest(field=field), self.assertRaises(RuntimeError):
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
        for field in ('diagnostic_complete', 'diagnostic_input', 'diagnostic_sources', 'diagnostic_final_build'):
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
                      'PRODUCER_CONTROLLER_SHA', 'DIAGNOSTIC_PROPOSAL_SHA'):
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

    def test_diagnostic_nonadmission_lineage_and_census_are_required(self):
        for field, bad in (('diagnostic_build', False), ('diagnostic_changes_admission', True),
                           ('tests_passed', 2798), ('tests_ignored', 24), ('phases', [{}] * 32)):
            items = fixture()
            items[2][field] = bad
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                self.admit(items)
        for section, key in (('source_lineage', 'diagnostic_proposal'), ('source_lineage', 'base_complete')):
            items = fixture()
            items[2][section][key]['sha256'] = '9' * 64
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                self.admit(items)
        items = fixture()
        items[0]['diagnostic_changes_admission'] = True
        with self.assertRaises(RuntimeError):
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


