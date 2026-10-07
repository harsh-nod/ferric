import copy
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location('http_attribution_test', Path(__file__).with_name('gpu_attribution.py'))
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)


def fixture(container=True):
    identities = [{'pid': 17, 'start_time_ticks': 88, 'pid_namespace': 'pid:[77]'}]
    user = {'identity': identities[0], 'descriptors': [{'fd': 3, 'major': 234, 'minor': 0}]}
    sample = {'schema': 'FerricDeviceDescriptorSampleV2', 'method': 'proc-fd-rdev-finite-roster-v1',
        'sampling_policy': 'initial-plus-one-birth-frontier-v1', 'root_visibility': True,
        'accepted': True, 'complete': True, 'phase': 'active', 'errors': [], 'foreign_users': [],
        'started_monotonic_ns': 20, 'completed_monotonic_ns': 30,
        'device_users': [user], 'owned_users': [user]}
    before = {'root_visibility': True, 'fuser_pids': [], 'sysfs_pids': [], 'owned_identities': identities,
              'started_ns': 10, 'finished_ns': 19}
    after = {**copy.deepcopy(before), 'started_ns': 31, 'finished_ns': 40}
    binding = None
    if container:
        binding = {'binding': {'id': 'c' * 64, 'image': 'sha256:' + 'd' * 64,
                    'name': 'owned', 'label_key': 'owned', 'label_value': 'owned'},
                   'state': {'init_pid': 17, 'started_at': 'fixed'}, 'host_pids': [17]}
        for item in (sample, before, after):
            item['container'] = copy.deepcopy(binding)
    return sample, before, after, identities, binding


class AttributionTests(unittest.TestCase):
    def test_empty_host_fuser_requires_real_positive_container_descriptors(self):
        sample, before, after, identities, container = fixture()
        result = m.accept(sample, before, after, phase='active', identities=identities, container=container)
        self.assertTrue(result['descriptor_positive'])
        self.assertTrue(result['host_fuser_empty'])
        sample['device_users'] = sample['owned_users'] = []
        with self.assertRaisesRegex(ValueError, 'positive owned'):
            m.accept(sample, before, after, phase='active', identities=identities, container=container)

    def test_namespace_lifetime_or_container_restart_refused(self):
        for mutation in ('namespace', 'lifetime', 'container'):
            sample, before, after, identities, container = fixture()
            if mutation == 'container':
                after['container']['state']['started_at'] = 'restarted'
            else:
                after['owned_identities'][0][{'namespace': 'pid_namespace', 'lifetime': 'start_time_ticks'}[mutation]] = 'changed'
            with self.assertRaises(ValueError):
                m.accept(sample, before, after, phase='active', identities=identities, container=container)

    def test_foreign_endpoint_and_descriptor_hits_refused(self):
        for key in ('sysfs_pids', 'fuser_pids', 'foreign_users'):
            sample, before, after, identities, container = fixture()
            if key == 'foreign_users':
                sample[key] = [{'identity': {'pid': 19}}]
            else:
                after[key] = [19]
            with self.assertRaises(ValueError):
                m.accept(sample, before, after, phase='active', identities=identities, container=container)

    def test_partial_permission_or_old_policy_cannot_pass(self):
        for key, value in (('accepted', False), ('complete', False), ('root_visibility', False),
                           ('errors', ['PermissionError']), ('sampling_policy', 'old'), ('accepted', 1)):
            sample, before, after, identities, container = fixture()
            sample[key] = value
            with self.assertRaises(ValueError):
                m.accept(sample, before, after, phase='active', identities=identities, container=container)

    def test_stale_external_endpoint_refused(self):
        sample, before, after, identities, container = fixture()
        after['started_ns'] = 21
        with self.assertRaisesRegex(ValueError, 'bracket'):
            m.accept(sample, before, after, phase='active', identities=identities, container=container)


if __name__ == '__main__':
    unittest.main()
