"""Selector/loader mocks only; no imports of controllers or process execution."""
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

import audit_device_clock_runtime as A


class AuditSelectorTests(unittest.TestCase):
    def fixture(self, role='worker'):
        I = SimpleNamespace(E=A.E, PARENT_NAME='ferric-qwen3-finite-prefix-decode-device-clock-engineering',
            WORKER_NAME='ferric-tp-peer-finite-engineering-worker-v1')
        name = I.PARENT_NAME if role == 'parent' else I.WORKER_NAME
        label = 'device-clock-runtime-' + role + '-v228-v1'
        cpu_label = 'gfx950-clock-parent-cpu-v228-v1' if role == 'parent' else 'gfx950-clock-recorder-cpu-v228-v1'
        cpu = dict(path=str(A.E / cpu_label / 'complete.json'), bytes=100, sha256='a' * 64)
        binary = dict(path=str(A.E / 'device-clock-runtime-v228-v1' / name), bytes=1000, sha256='b' * 64)
        original = dict(binary, path='/qualified/target/debug/' + name)
        records = {cpu['path']: cpu, binary['path']: binary}
        def read(path, digest, **options):
            record = records[str(path)]
            A.require(record['sha256'] == digest, 'synthetic digest mismatch')
            return record, None
        pins = SimpleNamespace(read=Mock(side_effect=read))
        I.cpu_evidence = Mock(return_value=({}, dict(binary=original), {}))
        I.deployed_binary = Mock(side_effect=lambda pins, record, expected: record)
        args = [label, role, cpu['path'], cpu['sha256'], binary['path'], binary['sha256']]
        return I, pins, args, cpu, binary, original

    def test_both_roles_join_actual_cpu_and_derived_extent_before_audit(self):
        for role in ('parent', 'worker'):
            I, pins, args, cpu, binary, original = self.fixture(role)
            with self.subTest(role=role), patch.object(A.os, 'access', return_value=True):
                self.assertEqual(A.selected(I, args, pins), (A.E / args[0], binary))
            I.cpu_evidence.assert_called_once_with(pins, cpu, role)
            I.deployed_binary.assert_called_once_with(pins, binary, original)
            self.assertEqual(pins.read.call_count, 2)

    def test_closed_argv_role_label_and_digest_refuse_before_reads(self):
        I, pins, args, *_ = self.fixture()
        cases = [args[:-1], args + ['1000']]
        for index, value in ((0, 'device-clock-runtime-parent-v228-v1'), (1, 'image'),
                              (3, 'x' * 64), (5, 'B' * 64)):
            changed = list(args); changed[index] = value; cases.append(changed)
        for bad in cases:
            with self.assertRaises(RuntimeError): A.selected(I, bad, pins)
        pins.read.assert_not_called()

    def test_cpu_and_binary_paths_cannot_escape_or_select_other_generation(self):
        I, pins, args, *_ = self.fixture()
        cases = []
        for index, value in ((2, str(A.E / 'device-routing-cpu-v228-v1/complete.json')),
                             (2, '/tmp/complete.json'), (2, args[2].replace('/complete.json', '/../complete.json')),
                             (4, args[4].replace('runtime-v228-v1/', 'runtime-v228-v2/')),
                             (4, '/tmp/' + I.WORKER_NAME), (4, args[4] + '/..')):
            changed = list(args); changed[index] = value; cases.append(changed)
        for bad in cases:
            with self.assertRaises(RuntimeError): A.selected(I, bad, pins)
        pins.read.assert_not_called()

    def test_unqualified_cpu_cannot_reach_binary_selection(self):
        I, pins, args, *_ = self.fixture()
        I.cpu_evidence.side_effect = RuntimeError('actual CPU qualification refused')
        with self.assertRaises(RuntimeError): A.selected(I, args, pins)
        self.assertEqual(pins.read.call_count, 1)
        I.deployed_binary.assert_not_called()

    def test_digest_or_nonexecutable_or_artifact_mismatch_refuses(self):
        for failure in ('digest', 'mode', 'artifact'):
            I, pins, args, *_ = self.fixture()
            if failure == 'digest': args[5] = 'c' * 64
            if failure == 'artifact': I.deployed_binary.side_effect = RuntimeError('wrong qualified artifact')
            with self.subTest(failure=failure), patch.object(A.os, 'access', return_value=failure != 'mode'), \
                 self.assertRaises(RuntimeError): A.selected(I, args, pins)

    def test_pending_intake_pin_fails_before_module_loading(self):
        D = SimpleNamespace(package=Mock())
        with patch.object(A, 'GPU_PACKAGE', ('p228-device-clock-gpu-v1', None)), self.assertRaises(RuntimeError):
            A.load_intake(D, object())
        D.package.assert_not_called()

    def test_layer_alias_is_restored_after_success_and_failure(self):
        for failure in (False, True):
            prior = object(); validation = object(); intake = SimpleNamespace(package_record=Mock())
            pins = SimpleNamespace(json=Mock(return_value=(dict(files=[
                dict(path='layer_validation.py', sha256='a' * 64), dict(path='intake.py', sha256='b' * 64)]), {})))
            def load(_pins, path, digest, alias):
                if path.name == 'layer_validation.py': return validation
                self.assertIs(A.sys.modules['layer_validation'], validation)
                if failure: raise RuntimeError('intake load failed')
                return intake
            D = SimpleNamespace(package=Mock(return_value=Path('/frozen')), load_module=Mock(side_effect=load))
            with self.subTest(failure=failure), patch.dict(A.sys.modules, {'layer_validation': prior}), \
                 patch.object(A, 'GPU_PACKAGE', ('p228-device-clock-gpu-v1', 'c' * 64)):
                if failure:
                    with self.assertRaises(RuntimeError): A.load_intake(D, pins)
                else:
                    self.assertIs(A.load_intake(D, pins), intake)
                    intake.package_record.assert_called_once_with(pins)
                self.assertIs(A.sys.modules['layer_validation'], prior)


if __name__ == '__main__': unittest.main()
