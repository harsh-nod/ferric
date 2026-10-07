import copy
from contextlib import redirect_stdout
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch


definition = importlib.util.spec_from_file_location('http_probe_cli_tests', Path(__file__).with_name('probe_cli.py'))
m = importlib.util.module_from_spec(definition)
definition.loader.exec_module(m)


class DiscoveryTests(unittest.TestCase):
    def fixture(self):
        wrapper = {'pid': 10, 'parent': 9, 'uid': 9661, 'group': 10, 'session': 10, 'start': 1}
        controller = {**wrapper, 'pid': 11, 'parent': 10, 'start': 2}
        worker = {**wrapper, 'pid': 12, 'parent': 11, 'start': 3}
        identity = {'pid': 12, 'start_time_ticks': 3, 'proc_uid': 9661, 'uids': [9661] * 4}
        rows = [wrapper, controller, worker]
        life = SimpleNamespace(members=lambda _: rows)
        activity = SimpleNamespace(identity=lambda *_: identity)
        probe = SimpleNamespace(NativeOwners=lambda *args: lambda: ({12: identity}, None))
        binding = {'wrapper': wrapper, 'controller_executable': {'path': 'controller'},
            'worker_executable': {'path': 'worker'}, 'controller': None, 'worker': None}
        matcher = lambda pid, expected: pid == (11 if expected['path'] == 'controller' else 12)
        return m.NativeDiscovery(activity, life, probe, binding), rows, matcher

    def test_first_discovery_pins_both_endpoints(self):
        owner, rows, matcher = self.fixture()
        with patch.object(m, 'executable', side_effect=matcher):
            first = owner()
            self.assertEqual(first, owner())
        self.assertEqual(owner.controller, rows[1])
        self.assertEqual(owner.worker['pid'], 12)

    def test_controller_lifetime_reuse_refused(self):
        owner, rows, matcher = self.fixture()
        with patch.object(m, 'executable', side_effect=matcher):
            owner()
            rows[1] = {**rows[1], 'start': 99}
            with self.assertRaisesRegex(ValueError, 'controller exited or changed'):
                owner()

    def test_previous_worker_cannot_disappear_or_restart(self):
        owner, rows, matcher = self.fixture()
        with patch.object(m, 'executable', side_effect=matcher):
            owner()
            rows.pop()
            with self.assertRaisesRegex(ValueError, 'worker exited'):
                owner()

    def test_wrong_parent_cannot_be_selected_controller(self):
        owner, rows, matcher = self.fixture()
        rows[1]['parent'] = 99
        with patch.object(m, 'executable', side_effect=matcher):
            with self.assertRaisesRegex(ValueError, 'direct wrapper child'):
                owner()

    def test_live_unreadable_executable_is_terminal(self):
        owner, _, _ = self.fixture()
        with patch.object(m, 'executable', side_effect=FileNotFoundError('exe')), \
            patch.object(m.Path, 'exists', return_value=True):
            with self.assertRaisesRegex(ValueError, 'live unreadable'):
                owner()

    def test_empty_before_creation_is_not_a_replacement_allowance(self):
        owner, rows, matcher = self.fixture()
        rows[:] = rows[:1]
        with patch.object(m, 'executable', side_effect=matcher):
            self.assertEqual(owner(), ({}, None))
        self.assertIsNone(owner.controller)
        self.assertIsNone(owner.worker)

    def test_container_restart_between_samples_is_not_readmitted(self):
        identity = {'id': 'a' * 64, 'name': 'owned', 'image': 'sha256:' + 'b' * 64,
                    'label_key': 'owner', 'label_value': 'owned', 'created': 'first'}
        activity = SimpleNamespace(docker_json=lambda _, **kwargs: b'[{"State":{"Running":true}}]',
            container_members=lambda *_, **kwargs: ({17: {'pid': 17}},
                {'state': {'init_pid': 17, 'started_at': 'new'}}))
        custody = SimpleNamespace(validate_intent=lambda _: None,
            list_owned=lambda *_: [{'ID': 'a' * 64}], observe=lambda *_: identity)
        owner = m.ContainerDiscovery(activity, custody, {'intent': {}, 'identity': identity,
            'runtime': {'init_pid': 17, 'started_at': 'old'}})
        with self.assertRaisesRegex(ValueError, 'restarted between samples'):
            owner()


class CliSerializationTests(unittest.TestCase):
    def exercise(self, foreign):
        root = Path(__file__).parent
        def load(name, filename):
            definition = importlib.util.spec_from_file_location(name, root / filename)
            value = importlib.util.module_from_spec(definition)
            definition.loader.exec_module(value)
            return value
        activity = load('http_real_device_shape', 'gpu_activity.py')
        probe = load('http_cli_real_observer', 'gpu_probe.py')
        attribution = load('http_cli_real_attribution', 'gpu_attribution.py')
        expected = {(234, 0): ['/dev/kfd'], (226, 128): ['/dev/dri/renderD128']}
        def scan(_proc, devices, allowed, phase):
            self.assertEqual(devices, expected)
            self.assertEqual(allowed, {})
            now = time.monotonic_ns()
            hits = [{'identity': {'pid': 99}, 'descriptors': [{'fd': 3, 'major': 234, 'minor': 0}]}] if foreign else []
            return {'schema': 'FerricDeviceDescriptorSampleV2', 'method': 'proc-fd-rdev-finite-roster-v1',
                'sampling_policy': 'initial-plus-one-birth-frontier-v1', 'root_visibility': True,
                'phase': phase, 'accepted': not foreign, 'complete': True, 'errors': [],
                'foreign_users': hits, 'device_users': hits, 'owned_users': [],
                'started_monotonic_ns': now, 'completed_monotonic_ns': now + 1}
        runner = lambda *args, **kwargs: subprocess.CompletedProcess(args[0], 1, b'', b'')
        wrapped = SimpleNamespace(observe=lambda activity, attribution, phase, owners, devices:
            probe.observe(activity, attribution, phase, owners, devices, runner=runner, sysfs=lambda: []))
        names = ('gpu_activity.py', 'gpu_attribution.py', 'gpu_probe.py',
                 'frozen/native_lifecycle.py', 'container_custody.py')
        value = {'schema': 'FerricNativeHttpGpuProbeInputV1', 'phase': 'preflight',
            'devices': ['/dev/kfd', '/dev/dri/renderD128'], 'owner': {'kind': 'empty', 'binding': None},
            'sources': {name: 'a' * 64 for name in names}}
        real_stat = Path.stat
        def fake_device_stat(path, *args, **kwargs):
            if str(path) in value['devices']:
                pair = (234, 0) if str(path) == '/dev/kfd' else (226, 128)
                return SimpleNamespace(st_mode=stat.S_IFCHR | 0o660, st_rdev=os.makedev(*pair))
            return real_stat(path, *args, **kwargs)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'probe-input.json'
            raw = json.dumps(value, sort_keys=True).encode()
            path.write_bytes(raw)
            output = io.StringIO()
            argv = ['probe_cli.py', '--input', str(path), '--input-sha256', hashlib.sha256(raw).hexdigest()]
            with patch.object(m.sys, 'argv', argv), patch.object(m.os, 'geteuid', return_value=0), \
                 patch.object(Path, 'stat', fake_device_stat), patch.object(activity, 'scan', side_effect=scan), \
                 patch.object(m, 'load', side_effect=[activity, attribution, wrapped, None, None]), \
                 redirect_stdout(output), self.assertRaises(SystemExit) as stopped:
                m.main()
            self.assertEqual(stopped.exception.code, 125 if foreign else 0)
            lines = output.getvalue().splitlines()
            self.assertEqual(len(lines), 1)
            result = json.loads(lines[0])
            self.assertEqual(result['accepted'], not foreign)
            self.assertEqual(result['devices'], [
                {'major': 226, 'minor': 128, 'paths': ['/dev/dri/renderD128']},
                {'major': 234, 'minor': 0, 'paths': ['/dev/kfd']}])
            return result

    def test_actual_cli_prints_one_json_record_with_frozen_scanner_device_map(self):
        self.assertIsNone(self.exercise(False)['retained_owner'])

    def test_actual_cli_preserves_foreign_hits_in_serialized_refusal(self):
        result = self.exercise(True)
        self.assertEqual(result['descriptor_scan']['foreign_users'][0]['identity']['pid'], 99)
        self.assertTrue(result['errors'])


if __name__ == '__main__':
    unittest.main()
