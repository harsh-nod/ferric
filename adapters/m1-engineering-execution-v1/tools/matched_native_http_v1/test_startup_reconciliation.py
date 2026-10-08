"""Real descriptor scans and endpoints through the bounded startup retry path."""
import copy
import importlib.util
import os
from pathlib import Path
import subprocess
from types import SimpleNamespace
import unittest
from unittest.mock import patch


def module(name):
    spec = importlib.util.spec_from_file_location('reconcile_' + name,
        Path(__file__).with_name(name + '.py'))
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


base, cli = module('test_startup_scanner'), module('probe_cli')
probe, activity, attribution = base.probe, base.activity, base.attribution


class StartupReconciliationTests(unittest.TestCase):
    def setUp(self):
        self.fixture = base.StartupScannerTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.fixture.gpu_pids = set()
        self.identity = {'id': 'a' * 64, 'name': 'owned', 'image': 'sha256:' + 'b' * 64,
            'label_key': 'owner', 'label_value': 'owned', 'created': 'first'}
        self.state = {'id': 'a' * 64, 'init_pid': 17, 'started_at': 'first'}
        self.elapsed = 0.0
        self.calls = []

    def row(self, pids):
        owners = {pid: activity.identity(self.fixture.proc, pid) for pid in pids}
        receipt = {'binding': {key: self.identity[key] for key in
            ('id', 'name', 'image', 'label_key', 'label_value')},
            'state': copy.deepcopy(self.state), 'host_pids': sorted(pids)}
        return owners, receipt

    def exercise(self, rows, *, endpoint_pids=None, mutate=None, advance=0, max_fds=None):
        rows = iter(copy.deepcopy(rows))
        class Supplier:
            identity = copy.deepcopy(self.identity)
            runtime = copy.deepcopy(self.state)
            def __call__(supplier):
                self.calls.append(None)
                return next(rows)
        supplier = Supplier()
        def command(argv, **kwargs):
            self.assertLessEqual(kwargs['timeout'], 15)
            self.elapsed += advance
            return subprocess.CompletedProcess(argv, 1, b'', b'')
        adapter = SimpleNamespace(device_identities=lambda _: self.fixture.devices,
            scan=activity.scan, identity=activity.identity, pids=activity.pids,
            MAX_PIDS=activity.MAX_PIDS, MAX_FDS=activity.MAX_FDS,
            MAX_TOTAL_FDS=activity.MAX_TOTAL_FDS if max_fds is None else max_fds,
            MAX_SECONDS=activity.MAX_SECONDS)
        budget = probe.StartupBudget(adapter, clock=lambda: self.elapsed, runner=command)
        attempts = []
        def observe(current, rules, phase, owners, devices, **kwargs):
            value = probe.observe(current, rules, phase, owners, devices,
                proc=self.fixture.proc, sysfs=lambda: endpoint_pids or [], **kwargs)
            if mutate:
                mutate(value, len(attempts))
            attempts.append(copy.deepcopy(value))
            return value
        wrapped = SimpleNamespace(observe=observe, startup_birth=probe.startup_birth,
            startup_departures=probe.startup_departures)
        original_stat = os.stat
        def stat_path(path, *args, **kwargs):
            if Path(path).parent.name == 'fd':
                return self.fixture.descriptor_stat(path)
            return original_stat(path, *args, **kwargs)
        with patch.object(probe.os, 'geteuid', return_value=0), \
             patch.object(activity, 'os', SimpleNamespace(**{key: getattr(os, key) for key in dir(os)})), \
             patch.object(probe.os, 'stat', side_effect=stat_path):
            # activity.scan's default effective_uid_reader is captured on import.
            real_scan = adapter.scan
            adapter.scan = lambda *args, **kwargs: real_scan(*args, **kwargs, effective_uid_reader=lambda: 0)
            result = cli.startup_container(adapter, attribution, wrapped, supplier,
                ['/dev/kfd', '/dev/dri/renderD128'], budget)
        self.assertEqual(result['startup_reconciliation']['attempts'], attempts)
        self.assertFalse(result['startup_reconciliation']['timing_admitted'])
        return result, attempts

    def initial(self):
        owned = self.row([17])
        return [({}, None), owned, owned, owned]

    def test_initial_container_birth_retains_refusal_then_accepts_fresh_real_scan(self):
        result, attempts = self.exercise(self.initial())
        self.assertTrue(result['accepted'])
        self.assertEqual(len(attempts), 2)
        self.assertFalse(attempts[0]['accepted'])
        self.assertTrue(attempts[0]['descriptor_scan']['accepted'])
        self.assertIn('identity changed across', attempts[0]['errors'][0])
        self.assertEqual(result['startup_reconciliation']['accepted_attempt'], 1)

    def test_engine_child_birth_is_same_container_monotonic_growth(self):
        self.fixture.add_process(18)
        old, new = self.row([17]), self.row([17, 18])
        result, attempts = self.exercise([old, new, new, new])
        self.assertTrue(result['accepted'])
        self.assertEqual(len(attempts), 2)

    def test_three_stable_complete_samples_are_not_required_when_first_stable(self):
        owned = self.row([17])
        result, attempts = self.exercise([owned, owned])
        self.assertTrue(result['accepted'])
        self.assertEqual(len(attempts), 1)

    def test_third_birth_exhausts_fixed_attempt_count(self):
        for pid in (18, 19):
            self.fixture.add_process(pid)
        a, b, c = self.row([17]), self.row([17, 18]), self.row([17, 18, 19])
        result, attempts = self.exercise([({}, None), a, a, b, b, c])
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 3)
        self.assertIsNone(result['startup_reconciliation']['accepted_attempt'])

    def test_positive_gpu_birth_is_terminal_even_when_final_owner_is_exact(self):
        self.fixture.gpu_pids = {17}
        result, attempts = self.exercise(self.initial())
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 1)
        self.assertTrue(attempts[0]['descriptor_scan']['foreign_users'])

    def test_positive_owned_descriptor_with_new_child_requires_fresh_scan(self):
        self.fixture.add_process(18)
        self.fixture.gpu_pids = {17}
        old, new = self.row([17]), self.row([17, 18])
        result, attempts = self.exercise([old, new, new, new], endpoint_pids=[17])
        self.assertTrue(result['accepted'])
        self.assertFalse(attempts[0]['accepted'])
        self.assertTrue(attempts[0]['descriptor_scan']['owned_users'])
        self.assertEqual(len(attempts), 2)

    def test_positive_startup_requires_complete_scan_and_stable_gpu_endpoints(self):
        self.fixture.add_process(18)
        self.fixture.gpu_pids = {17}
        old, new = self.row([17]), self.row([17, 18])
        mutations = [
            lambda x: x['descriptor_scan'].update(complete=False),
            lambda x: x['descriptor_scan'].update(accepted=False),
            lambda x: x['descriptor_scan'].update(errors=['incomplete']),
            lambda x: x['descriptor_scan'].update(foreign_users=[{'pid': 99}]),
            lambda x: x['after'].update(sysfs_pids=[18]),
            lambda x: x['before'].update(fuser_pids=[99]),
            lambda x: x['after'].update(root_visibility=False),
        ]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                result, attempts = self.exercise([old, new], endpoint_pids=[17],
                    mutate=lambda value, _: mutate(value))
                self.assertFalse(result['accepted'])
                self.assertEqual(len(attempts), 1)

    def test_positive_gpu_owner_lifetime_change_never_retries(self):
        self.fixture.add_process(18)
        self.fixture.gpu_pids = {17}
        old, new = self.row([17]), self.row([17, 18])
        new[0][17]['start_time_ticks'] += 1
        result, attempts = self.exercise([old, new], endpoint_pids=[17])
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 1)

    def test_nonempty_external_endpoint_is_terminal(self):
        result, attempts = self.exercise(self.initial(), endpoint_pids=[17])
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 1)

    def test_foreign_descriptor_never_retries(self):
        self.fixture.add_process(18)
        self.fixture.gpu_pids = {18}
        result, attempts = self.exercise(self.initial())
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 1)
        self.assertTrue(attempts[0]['descriptor_scan']['foreign_users'])

    def test_permission_failure_never_retries(self):
        def denied(_):
            raise PermissionError('live fd denied')
        self.fixture.descriptor_stat = denied
        result, attempts = self.exercise(self.initial())
        self.assertFalse(result['accepted'])
        self.assertFalse(attempts[0]['descriptor_scan']['complete'])
        self.assertEqual(len(attempts), 1)

    def test_credential_or_lifetime_change_is_not_monotonic_birth(self):
        self.fixture.add_process(18)
        for field, changed in (('start_time_ticks', 99), ('uids', [99] * 4),
                               ('pid_namespace', [99, 99]), ('proc_uid', 99), ('cgroup', 'other')):
            old, new = self.row([17]), self.row([17, 18])
            new[0][17][field] = changed
            with self.subTest(field=field):
                result, attempts = self.exercise([old, new])
                self.assertFalse(result['accepted'])
                self.assertEqual(len(attempts), 1)

    def test_owner_disappearance_between_attempts_never_becomes_new_baseline(self):
        result, attempts = self.exercise(self.initial()[:2] + [({}, None)])
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 2)
        self.assertIn('disappeared', attempts[1]['errors'][0])

    def test_owner_reuse_between_attempts_is_terminal(self):
        changed = self.row([17])
        changed[0][17]['start_time_ticks'] += 1
        result, attempts = self.exercise(self.initial()[:2] + [changed])
        self.assertFalse(result['accepted'])
        self.assertIn('changed across reconciliation', attempts[1]['errors'][0])

    def test_container_restart_is_terminal(self):
        self.fixture.add_process(18)
        old, new = self.row([17]), self.row([17, 18])
        new[1]['state']['started_at'] = 'restart'
        result, attempts = self.exercise([old, new])
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 1)

    def test_container_binding_change_is_terminal(self):
        self.fixture.add_process(18)
        for field in ('id', 'name', 'image', 'label_key', 'label_value'):
            old, new = self.row([17]), self.row([17, 18])
            new[1]['binding'][field] = 'foreign'
            with self.subTest(field=field):
                result, attempts = self.exercise([old, new])
                self.assertFalse(result['accepted'])
                self.assertEqual(len(attempts), 1)

    def test_active_or_preflight_or_postflight_refusal_is_not_retried(self):
        for phase in ('active', 'preflight', 'postflight'):
            with self.subTest(phase=phase):
                result, attempts = self.exercise(self.initial(),
                    mutate=lambda value, _: value.update(phase=phase))
                self.assertFalse(result['accepted'])
                self.assertEqual(len(attempts), 1)

    def test_shared_deadline_expiring_inside_endpoint_refuses_and_stops(self):
        result, attempts = self.exercise(self.initial(), advance=5.1)
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 2)
        self.assertIn('shared startup sampling deadline', attempts[1]['errors'][0])
        self.assertEqual(attempts[1]['before']['fuser_raw']['returncode'], 1)
        self.assertEqual(attempts[1]['before']['fuser_raw']['stdout_hex'], '')
        self.assertIn('finished_ns', attempts[1]['before'])

    def test_descriptor_budget_is_shared_across_complete_rescans(self):
        result, attempts = self.exercise(self.initial(), max_fds=1)
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 2)
        self.assertIn('shared startup descriptor bound', attempts[1]['descriptor_scan']['errors'][0])

    def test_missing_or_malformed_refusal_evidence_does_not_retry(self):
        for key in ('descriptor_scan', 'before', 'after'):
            with self.subTest(key=key):
                result, attempts = self.exercise(self.initial(),
                    mutate=lambda value, _, key=key: value.pop(key))
                self.assertFalse(result['accepted'])
                self.assertEqual(len(attempts), 1)


if __name__ == '__main__':
    unittest.main()
