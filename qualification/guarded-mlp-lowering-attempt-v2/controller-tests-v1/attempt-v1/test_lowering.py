"""Synthetic new-audit joins only; no compiler, filesystem or auditor is run."""
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
    producer = dict(schema='ferric-guarded-mlp-s-rpo-qualification-cpu-v1',
                    controller=pin(L.PRODUCER / 'run_cpu.py', L.PRODUCER_CONTROLLER_SHA),
                    passed=True, failure=None, source_unchanged=True, postcheck_errors=[],
                    final_compiler_product_phase='compiler-tests-build', source_lineage={},
                    artifacts=artifacts, input_manifest=pin(L.PRODUCER / 'input-manifest.json'),
                    input_sources=pin(L.PRODUCER / 'evidence/sources-before.json'),
                    raw={'compiler-tests-build.stdout': pin(L.PRODUCER / 'evidence/compiler-tests-build.stdout')})
    proof = dict(complete=pin(L.PRODUCER / 'evidence/complete.json'),
                 input=producer['input_manifest'], sources=producer['input_sources'],
                 final_build=producer['raw']['compiler-tests-build.stdout'], source_lineage={},
                 rlib_cpu_provenance=artifacts['backend-rlib'],
                 test_elf_cpu_provenance={role: artifacts[role] for role in
                     ('pliron-lib', 'compiler-lib', 'atomic-extraction', 'matrix-extraction')},
                 omitted_artifact_bodies_rehashed=False)
    audit_input = dict(schema='ferric-guarded-mlp-s-rpo-tool-audit-input-v1',
                       controller=pin(L.E / 'audit_guarded_mlp_s_rpo_compiler_tools_v228_v1.py', L.AUDIT_CONTROLLER_SHA),
                       tool_manifest=pin(L.TOOLS / 'manifest.json'),
                       producer_complete=proof['complete'], producer_input=proof['input'],
                       producer_sources=proof['sources'], producer_final_build=proof['final_build'])
    value = dict(schema='ferric-guarded-mlp-s-rpo-compiler-tool-audit-v1', passed=True,
                 failure=None, postcheck_errors=[], actual_library_audits_replayed=True,
                 compiler_invocation=False, producer_receipt_source_and_final_cargo_joins_replayed=True,
                 rlib_deployed=False, rlib_readelf_or_ldd_invoked=False,
                 tool_manifest=audit_input['tool_manifest'], tools=tools,
                 phases=[dict(label=name + '-' + phase) for name in L.NAMES for phase in ('readelf', 'ldd')],
                 inputs={}, resolved_paths={}, qualified_producer=proof,
                 input_manifest=pin(L.E / 'guarded-mlp-s-rpo-compiler-tools-audit-inputs-v228-v1.json'),
                 audits={'fe2o3-rustc-extract': {'libraries': {'librustc_codegen_fe2o3.so': {
                     'pin': tools['librustc_codegen_fe2o3.so']['deployed']}}}})
    docs = {str(L.AUDIT / 'complete.json'): value, value['input_manifest']['path']: audit_input,
            proof['complete']['path']: producer, str(L.TOOLS / 'manifest.json'): manifest}
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
            for field in ('producer_complete', 'producer_input', 'producer_sources', 'producer_final_build'):
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
        for field in ('producer_complete', 'producer_input', 'producer_sources', 'producer_final_build'):
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
        for field in ('AUDIT_SHA', 'VENDOR_CONTROLLER_SHA'):
            with mock.patch.object(L.sys, 'argv', ['lowering.py', '1' * 64]), \
                    mock.patch.object(L.sys, 'dont_write_bytecode', True), \
                    mock.patch.object(L, 'AUDIT_SHA', '2' * 64), \
                    mock.patch.object(L, 'VENDOR_CONTROLLER_SHA', '3' * 64), \
                    mock.patch.object(L, field, None), mock.patch.object(L, 'helper') as helper, \
                    mock.patch.object(L.os, 'umask') as umask:
                with self.subTest(field=field), self.assertRaisesRegex(RuntimeError, 'bindings are still pending'):
                    L.main()
                helper.assert_not_called()
                umask.assert_not_called()


if __name__ == '__main__':
    unittest.main()
