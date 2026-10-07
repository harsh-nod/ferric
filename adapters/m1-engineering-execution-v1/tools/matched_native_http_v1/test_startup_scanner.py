"""Exercise the real finite-roster scanner through the HTTP observer.

Only proc fixture files, device stat and external commands are substituted.
No accepted descriptor-sample dictionary is manufactured by these fixtures.
"""
import copy
import importlib.util
import os
from pathlib import Path
import stat
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch


def module(name):
    spec = importlib.util.spec_from_file_location('startup_integration_' + name,
        Path(__file__).with_name(name + '.py'))
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


activity, probe, attribution = (module(name) for name in
    ('gpu_activity', 'gpu_probe', 'gpu_attribution'))


class StartupScannerTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.proc = Path(temporary.name)
        self.add_process(17)
        self.owned = {17: activity.identity(self.proc, 17)}
        self.gpu_pids = {17}
        self.read_identity = activity.identity
        self.read_pids = activity.pids
        self.endpoint_pids = []
        self.devices = {(234, 0): ['/dev/kfd'], (226, 128): ['/dev/dri/renderD128']}

    def add_process(self, pid):
        root = self.proc / str(pid)
        (root / 'fd').mkdir(parents=True)
        (root / 'ns').mkdir()
        (root / 'ns/pid').write_bytes(b'namespace fixture')
        (root / 'fd/3').write_bytes(b'descriptor fixture')
        (root / 'stat').write_text(str(pid) + ' (fixture worker) ' +
            ' '.join(['S'] + ['0'] * 18 + [str(pid * 100)]) + '\n')
        (root / 'status').write_text('Name:\tfixture\nUid:\t9661\t9661\t9661\t9661\n')
        (root / 'cgroup').write_text('0::/owned-fixture\n')

    def descriptor_stat(self, path):
        pid = int(Path(path).parent.parent.name)
        return SimpleNamespace(st_mode=stat.S_IFCHR if pid in self.gpu_pids else stat.S_IFREG,
            st_rdev=os.makedev(234, 0))

    def observe(self, phase='startup', owners=None):
        real_scan = activity.scan
        adapter = SimpleNamespace(device_identities=lambda _: self.devices,
            scan=lambda proc, devices, allowed, current_phase: real_scan(proc, devices, allowed,
                current_phase, identity_reader=self.read_identity, pid_reader=self.read_pids,
                descriptor_stat=self.descriptor_stat, effective_uid_reader=lambda: 0))
        runner = lambda *args, **kwargs: subprocess.CompletedProcess(args[0], 1, b'', b'')
        with patch.object(probe.os, 'geteuid', return_value=0):
            return probe.observe(adapter, attribution, phase,
                owners or (lambda: (copy.deepcopy(self.owned), None)),
                ['/dev/kfd', '/dev/dri/renderD128'], proc=self.proc, runner=runner,
                sysfs=lambda: self.endpoint_pids)

    def test_startup_before_worker_creation_accepts_empty_complete_sample(self):
        self.owned, self.gpu_pids = {}, set()
        result = self.observe()
        self.assertTrue(result['accepted'])
        self.assertTrue(result['descriptor_scan']['complete'])
        self.assertEqual(result['descriptor_scan']['phase'], 'startup')
        self.assertEqual(result['descriptor_scan']['device_users'], [])
        self.assertFalse(result['attribution']['timing_admitted'])

    def test_startup_existing_owned_worker_before_kfd_is_not_active(self):
        self.gpu_pids = set()
        result = self.observe()
        self.assertTrue(result['accepted'])
        self.assertFalse(result['attribution']['descriptor_positive'])
        self.assertFalse(result['attribution']['timing_admitted'])

    def test_startup_owned_worker_with_kfd_accepts_positive_real_scan(self):
        self.endpoint_pids = [17]
        result = self.observe()
        self.assertTrue(result['accepted'])
        self.assertTrue(result['attribution']['descriptor_positive'])
        self.assertFalse(result['attribution']['timing_admitted'])
        self.assertEqual(result['descriptor_scan']['owned_users'][0]['identity'], self.owned[17])
        self.assertEqual(result['attribution']['scanner_sha256'], attribution.SCANNER_SHA)

    def test_active_still_requires_positive_descriptors(self):
        self.gpu_pids = set()
        result = self.observe('active')
        self.assertFalse(result['accepted'])
        self.assertIn('no positive owned', result['descriptor_scan']['errors'][0])

    def test_preflight_and_postflight_still_require_empty_descriptors(self):
        for phase in ('preflight', 'postflight'):
            with self.subTest(phase=phase):
                sample = activity.scan(self.proc, self.devices, self.owned, phase,
                    identity_reader=self.read_identity, descriptor_stat=self.descriptor_stat)
                self.assertFalse(sample['accepted'])
                self.assertIn('lifecycle endpoint', sample['errors'][0])

    def test_startup_foreign_gpu_hit_remains_retained_and_terminal(self):
        self.add_process(18)
        self.gpu_pids.add(18)
        result = self.observe()
        self.assertFalse(result['accepted'])
        self.assertEqual(result['descriptor_scan']['foreign_users'][0]['identity']['pid'], 18)
        self.assertTrue(result['descriptor_scan']['complete'])

    def test_startup_owned_lifetime_namespace_and_credentials_drift_refuse(self):
        for field in ('start_time_ticks', 'pid_namespace', 'uids', 'proc_uid'):
            calls = []
            def reader(proc, pid):
                row = activity.identity(proc, pid)
                calls.append(pid)
                if len(calls) > 1:
                    row[field] = [99, 99] if field == 'pid_namespace' else ([99] * 4 if field == 'uids' else 99)
                return row
            self.read_identity = reader
            with self.subTest(field=field):
                result = self.observe()
                self.assertFalse(result['accepted'])
                self.assertTrue(result['descriptor_scan']['device_users'])

    def test_startup_live_permission_error_never_becomes_empty(self):
        def denied(_path):
            raise PermissionError('fixture live descriptor denied')
        self.descriptor_stat = denied
        result = self.observe()
        self.assertFalse(result['accepted'])
        self.assertIn('PermissionError', result['descriptor_scan']['errors'][0])
        self.assertFalse(result['descriptor_scan']['complete'])

    def test_startup_worker_birth_between_endpoints_refuses_even_without_gpu(self):
        self.gpu_pids = set()
        values = iter((({}, None), (self.owned, None)))
        result = self.observe(owners=lambda: next(values))
        self.assertFalse(result['accepted'])
        self.assertTrue(result['descriptor_scan']['accepted'])
        self.assertIn('identity changed across', result['errors'][0])
        self.assertEqual(result['before']['owned_identities'], [])
        self.assertEqual(result['after']['owned_identities'], [self.owned[17]])

    def test_startup_unbound_gpu_birth_in_frontier_is_not_reinterpreted(self):
        self.owned, self.gpu_pids = {}, set()
        calls = []
        def census(proc):
            calls.append(None)
            if len(calls) == 2:
                self.add_process(18)
                self.gpu_pids.add(18)
            return activity.pids(proc)
        self.read_pids = census
        result = self.observe()
        self.assertFalse(result['accepted'])
        self.assertEqual(result['descriptor_scan']['birth_frontier_pids'], [18])
        self.assertEqual(result['descriptor_scan']['foreign_users'][0]['identity']['pid'], 18)

    def test_startup_owner_replacement_at_final_endpoint_refuses(self):
        replacement = copy.deepcopy(self.owned)
        replacement[17]['start_time_ticks'] += 1
        values = iter(((self.owned, None), (replacement, None)))
        result = self.observe(owners=lambda: next(values))
        self.assertFalse(result['accepted'])
        self.assertTrue(result['descriptor_scan']['owned_users'])

    def test_old_preflight_sample_cannot_be_relabelled_startup(self):
        self.gpu_pids = set()
        result = self.observe()
        sample = result['descriptor_scan']
        sample['phase'] = 'preflight'
        with self.assertRaisesRegex(ValueError, 'qualified complete'):
            attribution.accept(sample, result['before'], result['after'], phase='startup',
                identities=result['before']['owned_identities'])


if __name__ == '__main__':
    unittest.main()
