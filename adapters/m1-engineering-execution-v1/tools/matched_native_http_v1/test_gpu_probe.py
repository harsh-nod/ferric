import copy
import importlib.util
import json
from pathlib import Path
import subprocess
from types import SimpleNamespace
import unittest
from unittest.mock import patch


def module(name):
    spec = importlib.util.spec_from_file_location('http_probe_' + name, Path(__file__).with_name(name + '.py'))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


m, attribution = module('gpu_probe'), module('gpu_attribution')


class ProbeTests(unittest.TestCase):
    def fixture(self):
        ticks = iter(range(10, 100))
        clock = lambda: next(ticks)
        identity = {'pid': 17, 'start_time_ticks': 1, 'uids': [9661] * 4, 'proc_uid': 9661}
        user = {'identity': identity, 'descriptors': [{'fd': 3, 'major': 234, 'minor': 0}]}
        owners = lambda: ({17: identity}, None)
        def scan(_proc, _devices, _allowed, phase):
            return {'schema': 'FerricDeviceDescriptorSampleV2', 'method': 'proc-fd-rdev-finite-roster-v1',
                    'sampling_policy': 'initial-plus-one-birth-frontier-v1', 'root_visibility': True,
                    'phase': phase, 'accepted': True, 'complete': True, 'errors': [], 'foreign_users': [],
                    'device_users': [user], 'owned_users': [user],
                    'started_monotonic_ns': clock(), 'completed_monotonic_ns': clock()}
        activity = SimpleNamespace(device_identities=lambda _: {
            (234, 0): ['/dev/kfd'], (226, 128): ['/dev/dri/renderD128']}, scan=scan)
        runner = lambda *args, **kwargs: subprocess.CompletedProcess(args[0], 1, b'', b'')
        return activity, owners, runner, clock

    def call(self, activity, owners, runner, clock, **kwargs):
        with patch.object(m.os, 'geteuid', return_value=0):
            return m.observe(activity, attribution, 'active', owners,
                ['/dev/kfd', '/dev/dri/renderD128'], runner=runner,
                sysfs=kwargs.pop('sysfs', lambda: []), clock=clock, **kwargs)

    def test_positive_descriptors_admit_empty_host_fuser(self):
        result = self.call(*self.fixture())
        self.assertTrue(result['accepted'])
        self.assertTrue(result['attribution']['host_fuser_empty'])
        self.assertTrue(result['attribution']['descriptor_positive'])

    def test_real_tuple_device_map_is_losslessly_serialized_without_changing_scan_input(self):
        activity, owners, runner, clock = self.fixture()
        expected = activity.device_identities(None)
        original = activity.scan
        def scan(proc, devices, allowed, phase):
            self.assertEqual(devices, expected)
            self.assertTrue(all(type(pair) is tuple for pair in devices))
            return original(proc, devices, allowed, phase)
        activity.scan = scan
        result = self.call(activity, owners, runner, clock)
        restored = json.loads(json.dumps(result, sort_keys=True, allow_nan=False))
        self.assertTrue(restored['accepted'])
        self.assertEqual(restored['devices'], [
            {'major': 226, 'minor': 128, 'paths': ['/dev/dri/renderD128']},
            {'major': 234, 'minor': 0, 'paths': ['/dev/kfd']}])

    def test_refused_probe_keeps_json_serializable_device_and_raw_endpoint_evidence(self):
        result = self.call(*self.fixture(), sysfs=lambda: [99])
        restored = json.loads(json.dumps(result, sort_keys=True, allow_nan=False))
        self.assertFalse(restored['accepted'])
        self.assertEqual(restored['before']['sysfs_pids'], [99])
        self.assertEqual(restored['before']['fuser_raw']['stdout_hex'], '')
        self.assertEqual(len(restored['devices']), 2)

    def test_foreign_positive_hit_is_retained_and_refused(self):
        activity, owners, runner, clock = self.fixture()
        original = activity.scan
        def scan(*args):
            value = original(*args)
            value['accepted'] = False
            value['foreign_users'] = [{'identity': {'pid': 99}, 'descriptors': [{'fd': 2}]}]
            return value
        activity.scan = scan
        result = self.call(activity, owners, runner, clock)
        self.assertFalse(result['accepted'])
        self.assertEqual(result['descriptor_scan']['foreign_users'][0]['identity']['pid'], 99)

    def test_owner_lifetime_drift_refuses_but_preserves_scan(self):
        activity, owners, runner, clock = self.fixture()
        first = owners()
        second = copy.deepcopy(first)
        second[0][17]['start_time_ticks'] += 1
        sequence = iter((first, second))
        result = self.call(activity, lambda: next(sequence), runner, clock)
        self.assertFalse(result['accepted'])
        self.assertTrue(result['descriptor_scan']['device_users'])

    def test_empty_active_scan_never_admitted(self):
        activity, owners, runner, clock = self.fixture()
        original = activity.scan
        def scan(*args):
            value = original(*args)
            value['device_users'] = value['owned_users'] = []
            return value
        activity.scan = scan
        self.assertFalse(self.call(activity, owners, runner, clock)['accepted'])

    def test_startup_complete_empty_descriptors_is_not_active_attribution(self):
        activity, owners, runner, clock = self.fixture()
        original = activity.scan
        def scan(*args):
            value = original(*args)
            value['device_users'] = value['owned_users'] = []
            return value
        activity.scan = scan
        with patch.object(m.os, 'geteuid', return_value=0):
            result = m.observe(activity, attribution, 'startup', owners,
                ['/dev/kfd', '/dev/dri/renderD128'], runner=runner, sysfs=lambda: [], clock=clock)
        self.assertTrue(result['accepted'])
        self.assertEqual(result['descriptor_scan']['phase'], 'startup')
        self.assertFalse(result['attribution']['descriptor_positive'])
        self.assertFalse(result['attribution']['timing_admitted'])

    def test_startup_foreign_descriptor_cannot_be_ignored(self):
        activity, owners, runner, clock = self.fixture()
        original = activity.scan
        def scan(*args):
            value = original(*args)
            value['foreign_users'] = [{'identity': {'pid': 99}, 'descriptors': [{'fd': 2}]}]
            return value
        activity.scan = scan
        with patch.object(m.os, 'geteuid', return_value=0):
            result = m.observe(activity, attribution, 'startup', owners,
                ['/dev/kfd', '/dev/dri/renderD128'], runner=runner, sysfs=lambda: [], clock=clock)
        self.assertFalse(result['accepted'])
        self.assertTrue(result['descriptor_scan']['foreign_users'])

    def test_foreign_endpoint_refuses_before_scan(self):
        activity, owners, runner, clock = self.fixture()
        activity.scan = lambda *args: self.fail('foreign endpoint must not progress to scan')
        result = self.call(activity, owners, runner, clock, sysfs=lambda: [99])
        self.assertFalse(result['accepted'])
        self.assertEqual(result['before']['sysfs_pids'], [99])

    def test_permission_failure_cannot_become_empty_endpoint(self):
        activity, owners, runner, clock = self.fixture()
        def sysfs():
            raise PermissionError('denied')
        result = self.call(activity, owners, runner, clock, sysfs=sysfs)
        self.assertFalse(result['accepted'])
        self.assertIn('PermissionError', result['errors'][0])
        self.assertEqual(result['before']['fuser_raw']['returncode'], 1)
        self.assertEqual(result['before']['fuser_raw']['stdout_hex'], '')
        self.assertGreater(result['before']['finished_ns'], result['before']['started_ns'])

    def test_malformed_fuser_retains_exact_raw_before_refusal(self):
        activity, owners, _, clock = self.fixture()
        raw = b'not-a-pid\xff'
        runner = lambda *args, **kwargs: subprocess.CompletedProcess(args[0], 0, raw, b'diagnostic')
        result = self.call(activity, owners, runner, clock)
        self.assertFalse(result['accepted'])
        self.assertEqual(result['before']['fuser_raw']['stdout_hex'], raw.hex())
        self.assertEqual(result['before']['fuser_raw']['stderr_hex'], b'diagnostic'.hex())
        self.assertGreater(result['before']['finished_ns'], result['before']['started_ns'])

    def test_owner_failure_retains_prior_endpoint_fields(self):
        activity, _, runner, clock = self.fixture()
        def owners():
            raise ProcessLookupError('owner exited')
        result = self.call(activity, owners, runner, clock, sysfs=lambda: [17])
        self.assertFalse(result['accepted'])
        self.assertEqual(result['before']['fuser_raw']['returncode'], 1)
        self.assertEqual(result['before']['sysfs_pids'], [17])
        self.assertGreater(result['before']['finished_ns'], result['before']['started_ns'])

    def test_final_owner_failure_retains_positive_scan_and_partial_after(self):
        activity, original, runner, clock = self.fixture()
        calls = []
        def owners():
            calls.append(None)
            if len(calls) == 2:
                raise ProcessLookupError('owner exited after scan')
            return original()
        result = self.call(activity, owners, runner, clock)
        self.assertFalse(result['accepted'])
        self.assertTrue(result['descriptor_scan']['device_users'])
        self.assertEqual(result['after']['fuser_raw']['returncode'], 1)
        self.assertGreater(result['after']['finished_ns'], result['after']['started_ns'])

    def test_nonroot_visibility_refused(self):
        activity, owners, runner, clock = self.fixture()
        with patch.object(m.os, 'geteuid', return_value=9661):
            result = m.observe(activity, attribution, 'active', owners,
                ['/dev/kfd', '/dev/dri/renderD128'], runner=runner, clock=clock)
        self.assertFalse(result['accepted'])

    def test_fuser_errors_are_not_empty_evidence(self):
        for status, stdout, stderr in ((1, b'', b'denied'), (0, b'', b''),
                (0, b'17 17', b'/dev/kfd:'), (0, b'17', b'/dev/kfd: mm'),
                (2, b'', b''), (1, b'\n', b'')):
            with self.subTest(status=status, stdout=stdout, stderr=stderr):
                with self.assertRaises(ValueError):
                    m.fuser_members(subprocess.CompletedProcess([], status, stdout, stderr))

    def test_container_restart_preserves_positive_scan_and_refuses(self):
        activity, original, runner, clock = self.fixture()
        binding = {'binding': {'id': 'c' * 64, 'image': 'sha256:' + 'd' * 64,
                    'name': 'owned', 'label_key': 'owned', 'label_value': 'owned'},
                   'state': {'init_pid': 17, 'started_at': 'first'}, 'host_pids': [17]}
        restarted = copy.deepcopy(binding)
        restarted['state']['started_at'] = 'later'
        records = iter((binding, restarted))
        result = self.call(activity, lambda: (original()[0], next(records)), runner, clock)
        self.assertFalse(result['accepted'])
        self.assertTrue(result['descriptor_scan']['device_users'])
        self.assertEqual(result['after']['container'], restarted)

    def test_stable_container_membership_with_empty_fuser_passes(self):
        activity, original, runner, clock = self.fixture()
        binding = {'binding': {'id': 'c' * 64, 'image': 'sha256:' + 'd' * 64,
                    'name': 'owned', 'label_key': 'owned', 'label_value': 'owned'},
                   'state': {'init_pid': 17, 'started_at': 'first'}, 'host_pids': [17]}
        result = self.call(activity, lambda: (original()[0], binding), runner, clock)
        self.assertTrue(result['accepted'])
        self.assertEqual(result['descriptor_scan']['container'], binding)

    def test_device_inode_or_major_minor_replacement_refuses(self):
        activity, owners, runner, clock = self.fixture()
        records = iter(({(234, 0): ['/dev/kfd'], (226, 128): ['/dev/dri/renderD128']},
                        {(235, 0): ['/dev/kfd'], (226, 128): ['/dev/dri/renderD128']}))
        activity.device_identities = lambda _: next(records)
        result = self.call(activity, owners, runner, clock)
        self.assertFalse(result['accepted'])
        self.assertTrue(result['descriptor_scan']['device_users'])


class NativeOwnerTests(unittest.TestCase):
    def fixture(self):
        wrapper = {'pid': 10, 'parent': 1, 'group': 10, 'session': 10, 'start': 100, 'uid': 9661}
        controller = {'pid': 20, 'parent': 10, 'group': 10, 'session': 10, 'start': 200, 'uid': 9661}
        worker_process = {'pid': 30, 'parent': 20, 'group': 10, 'session': 10, 'start': 300, 'uid': 9661}
        identity = {'pid': 30, 'start_time_ticks': 300, 'uids': [9661] * 4, 'proc_uid': 9661}
        binding = {'wrapper': wrapper, 'controller': controller,
            'worker': {'identity': identity, 'executable': {'path': '/owned/worker', 'dev': 1, 'ino': 2, 'size': 100}}}
        processes = copy.deepcopy({10: wrapper, 20: controller, 30: worker_process})
        observed = copy.deepcopy(identity)
        activity = SimpleNamespace(identity=lambda _proc, _pid: observed)
        lifecycle = SimpleNamespace(process=lambda pid: processes[pid])
        return m.NativeOwners(activity, lifecycle, binding), processes, observed

    def call(self, owners, *, inode=2, path='/owned/worker'):
        with patch.object(m.Path, 'stat', return_value=SimpleNamespace(st_dev=1, st_ino=inode, st_size=100)):
            with patch.object(m.os, 'readlink', return_value=path):
                return owners()

    def test_exact_worker_under_live_controller_and_wrapper(self):
        owners, _, observed = self.fixture()
        self.assertEqual(self.call(owners), ({30: observed}, None))

    def test_foreign_executable_or_inode_rejected(self):
        for kwargs in ({'inode': 99}, {'path': '/foreign/worker'}):
            owners, _, _ = self.fixture()
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                self.call(owners, **kwargs)

    def test_reparenting_or_wrapper_reuse_rejected(self):
        for pid, field, value in ((30, 'parent', 99), (30, 'session', 99), (10, 'start', 999),
                                  (20, 'uid', 0), (30, 'uid', 0)):
            owners, processes, _ = self.fixture()
            processes[pid][field] = value
            with self.subTest(pid=pid, field=field), self.assertRaises(ValueError):
                self.call(owners)

    def test_worker_lifetime_or_credentials_rejected(self):
        for field, value in (('start_time_ticks', 999), ('uids', [0] * 4), ('proc_uid', 0)):
            owners, _, observed = self.fixture()
            observed[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.call(owners)


if __name__ == '__main__':
    unittest.main()
