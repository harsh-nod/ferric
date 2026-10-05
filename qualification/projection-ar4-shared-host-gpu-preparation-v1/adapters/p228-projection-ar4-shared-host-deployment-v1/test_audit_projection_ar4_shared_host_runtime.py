"""Synthetic selection/API tests; no real ELF audit or CPU qualification."""
import contextlib
import copy
import importlib.util
from pathlib import Path
import sys
import types
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location('shared_selector',
    Path(__file__).with_name('audit_projection_ar4_shared_host_runtime.py'))
A = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(A)


class SelectorTests(unittest.TestCase):
    def setUp(self):
        self.root = A.E / 'projection-ar4-shared-host-cpu-v228-v1'
        self.cpu = dict(path=str(self.root / 'complete.json'), bytes=900, sha256='c' * 64)
        self.header = b'\x7fELF\x02\x01' + bytes(12) + b'\x3e\x00'

    def invocation(self, role='default-parent'):
        _, directory, name = A.ROLES[role]
        binary = dict(path=str(self.root / 'target' / directory / 'debug' / name),
                      bytes=len(self.header), sha256='e' * 64)
        args = ['a' * 64, 'projection-ar4-shared-host-runtime-' + role + '-v228-v1',
                role, self.cpu['path'], self.cpu['sha256'], binary['path'], binary['sha256']]
        return args, binary

    def select(self, args, binary, header=None, actual=None, executable=True):
        pins = types.SimpleNamespace(read=mock.Mock(side_effect=[(self.cpu, b''),
            (binary, self.header if header is None else header)]))
        with mock.patch.object(A, 'cpu_artifact', return_value=binary if actual is None else actual), \
                mock.patch.object(A.os, 'access', return_value=executable):
            return A.selected(None, args, pins, None)

    def test_three_roles_select_only_their_actual_product(self):
        for role in A.ROLES:
            args, binary = self.invocation(role)
            self.assertEqual(self.select(args, binary), (A.E / args[1], binary))

    def test_original_header_bytes_not_escaped_text(self):
        args, binary = self.invocation()
        self.assertEqual(len(self.header), 20)
        self.select(args, binary)
        with self.assertRaises(RuntimeError):
            self.select(args, binary, header=br'\x7fELF\x02\x01' + bytes(20))

    def test_role_and_label_cannot_cross(self):
        args, binary = self.invocation(); args[2] = 'shared-parent'
        with self.assertRaises(RuntimeError): self.select(args, binary)
        args[2] = 'parent'
        with self.assertRaises(RuntimeError): self.select(args, binary)

    def test_old_cpu_namespace_rejected(self):
        args, binary = self.invocation(); args[3] = A.BASELINE['path']
        with self.assertRaises(RuntimeError): self.select(args, binary)

    def test_wrong_role_directory_or_copied_elf_rejected(self):
        args, binary = self.invocation()
        for replacement in (args[5].replace('/parent/', '/worker/'), str(A.E / Path(args[5]).name)):
            changed = list(args); changed[5] = replacement
            with self.assertRaises(RuntimeError): self.select(changed, binary)

    def test_actual_binary_extent_digest_and_mode_required(self):
        args, binary = self.invocation()
        for change in ({'bytes': 21}, {'sha256': 'f' * 64}):
            with self.assertRaises(RuntimeError): self.select(args, binary, actual=dict(binary, **change))
        with self.assertRaises(RuntimeError): self.select(args, binary, executable=False)

    def test_x86_64_little_endian_header_required(self):
        args, binary = self.invocation()
        for bad in (b'', bytes(20), self.header[:18] + b'\xb7\x00'):
            with self.assertRaises(RuntimeError): self.select(args, binary, header=bad)

    def test_explicit_package_and_actual_digests_required(self):
        args, binary = self.invocation()
        for index in (0, 4, 6):
            changed = list(args); changed[index] = 'F' * 64
            with self.assertRaises(RuntimeError): self.select(changed, binary)
        with self.assertRaises(RuntimeError): A.selected(None, args[:-1], None, None)

    def contract_fixture(self):
        prior = {'raw': {'sources-after.json': {'path': 'base', 'bytes': 1, 'sha256': 'b' * 64}}}
        value = {'prior_completion': copy.deepcopy(A.BASELINE)}
        proposal = {'base_sources_after': prior['raw']['sources-after.json']}
        selected = {key: {'artifact': {'target': {'name': name}}, 'binary': {'role': key}}
                    for key, _, name in A.ROLES.values()}
        B = types.SimpleNamespace(contract=mock.Mock())
        S = types.SimpleNamespace(contract=mock.Mock(return_value=selected),
            sources=mock.Mock(return_value=proposal), test_delta=mock.Mock())
        previous = types.SimpleNamespace(read_pin=mock.Mock(side_effect=[value, prior]))
        return value, prior, proposal, B, S, previous

    def run_contract(self, fixture, role='shared-parent'):
        _, _, _, B, S, previous = fixture
        with mock.patch.object(A, 'contracts', return_value=contextlib.nullcontext((B, S))):
            return A.cpu_artifact(types.SimpleNamespace(parse=lambda x: x), object(), previous,
                                  self.cpu, role, 'a' * 64)

    def test_contract_uses_source_and_exact_test_delta_without_gpu_context(self):
        fixture = self.contract_fixture()
        self.assertEqual(self.run_contract(fixture), {'role': 'shared'})
        fixture[3].contract.assert_called_once()
        fixture[4].sources.assert_called_once()
        fixture[4].test_delta.assert_called_once_with(fixture[0], fixture[1], fixture[2])

    def test_wrong_predecessor_or_base_map_refuses(self):
        fixture = self.contract_fixture(); fixture[0]['prior_completion']['sha256'] = '0' * 64
        with self.assertRaises(RuntimeError): self.run_contract(fixture)
        fixture = self.contract_fixture(); fixture[2]['base_sources_after'] = {}
        with self.assertRaises(RuntimeError): self.run_contract(fixture)

    def test_contract_source_or_named_test_refusal_propagates(self):
        for key in ('contract', 'sources', 'test_delta'):
            fixture = self.contract_fixture()
            getattr(fixture[4], key).side_effect = ValueError(key)
            with self.assertRaises(ValueError): self.run_contract(fixture)

    def loader(self):
        names = ('layer_validation', 'observer_cpu', 'shared_cpu')
        hashes = (A.VALIDATOR_SHA, A.BASE_CPU_SHA, 'f' * 64)
        rows = [{'path': n + '.py', 'bytes': 1, 'sha256': h} for n, h in zip(names, hashes)]
        manifest = {'schema': A.PACKAGE_SCHEMA, 'files': rows}
        D = types.SimpleNamespace(package=mock.Mock(return_value=A.E / A.PACKAGE),
            json=None, load_module=mock.Mock(side_effect=[types.ModuleType(n) for n in names]))
        pins = types.SimpleNamespace(json=None)
        D.json = None
        pins.json = mock.Mock(return_value=(manifest, {}))
        return D, pins, manifest

    def test_aliases_restore_existing_none_and_absent_entries(self):
        D, pins, _ = self.loader()
        old = types.ModuleType('sentinel')
        with mock.patch.dict(sys.modules, {'layer_validation': old, 'observer_cpu': None}):
            previous = sys.modules.pop('shared_cpu', None)
            try:
                with A.contracts(D, pins, 'a' * 64):
                    self.assertIsNot(sys.modules['layer_validation'], old)
                self.assertIs(sys.modules['layer_validation'], old)
                self.assertIsNone(sys.modules['observer_cpu'])
                self.assertNotIn('shared_cpu', sys.modules)
            finally:
                if previous is not None: sys.modules['shared_cpu'] = previous

    def test_loader_failure_restores_aliases(self):
        D, pins, _ = self.loader()
        D.load_module.side_effect = [types.ModuleType('v'), ValueError('load')]
        old = types.ModuleType('old')
        with mock.patch.dict(sys.modules, {'layer_validation': old, 'observer_cpu': None}):
            with self.assertRaises(ValueError):
                with A.contracts(D, pins, 'a' * 64): pass
            self.assertIs(sys.modules['layer_validation'], old)
            self.assertIsNone(sys.modules['observer_cpu'])

    def test_predecessor_package_or_mutated_helper_refuses(self):
        for change in ('schema', 'helper'):
            D, pins, manifest = self.loader()
            if change == 'schema': manifest['schema'] = 'old-package'
            else: manifest['files'][0]['sha256'] = '0' * 64
            with self.assertRaises(RuntimeError):
                with A.contracts(D, pins, 'a' * 64): pass


if __name__ == '__main__':
    unittest.main()
