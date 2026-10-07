"""CPU-only real subprocess exits through the unchanged descriptor scanner."""
import copy
import importlib.util
import os
from pathlib import Path
import stat
import subprocess
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import patch


def module(name):
    spec = importlib.util.spec_from_file_location('departure_' + name,
        Path(__file__).with_name(name + '.py'))
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


activity, probe, attribution, cli = (module(name) for name in
    ('gpu_activity', 'gpu_probe', 'gpu_attribution', 'probe_cli'))


class StartupDepartureTests(unittest.TestCase):
    def exercise(self, *, mode='during', deny=None, positive=False, proof_mode=None,
                 change=None, mutate=None):
        child = subprocess.Popen([sys.executable, '-I', '-B', '-c',
            'import sys; sys.stdin.buffer.read()'], stdin=subprocess.PIPE,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        init_pid = os.getpid()
        child_pid = child.pid
        first = {pid: activity.identity(Path('/proc'), pid) for pid in (init_pid, child_pid)}
        identity = {'id': 'a' * 64, 'name': 'owned', 'image': 'sha256:' + 'b' * 64,
            'label_key': 'owner', 'label_value': 'owned', 'created': 'first'}
        runtime = {'id': identity['id'], 'init_pid': init_pid, 'started_at': 'first'}
        binding = {key: identity[key] for key in ('id', 'name', 'image', 'label_key', 'label_value')}
        endpoints, scans, proofs, attempts = [], [], [], []
        elapsed = [0.0]
        def stop():
            if child.poll() is None:
                child.terminate()
                child.wait(timeout=2)
        def owners():
            if len(endpoints) == 1 and mode == 'after':
                stop()
            allowed = copy.deepcopy(first if not endpoints else {init_pid: first[init_pid]})
            receipt = {'binding': copy.deepcopy(binding), 'state': copy.deepcopy(runtime),
                       'host_pids': sorted(allowed)}
            if change and len(endpoints) == 1:
                change(allowed, receipt, init_pid, child_pid)
            endpoints.append(copy.deepcopy((allowed, receipt)))
            return allowed, receipt
        class Supplier:
            def __call__(self):
                return owners()
        supplier = Supplier()
        supplier.identity, supplier.runtime = identity, runtime
        def scanner(*args, **kwargs):
            if not scans and mode == 'during':
                stop()
            scans.append(None)
            return activity.scan(*args, **kwargs, effective_uid_reader=lambda: 0)
        def census(_root):
            values = {init_pid}
            if not scans or len(scans) == 1 or (proof_mode == 'census-reuse' and proofs):
                values.add(child_pid)
            return values
        def identities(root, pid):
            if deny and pid == child_pid:
                if deny == 'permission':
                    raise PermissionError('live startup process unreadable')
                raise FileNotFoundError(2, 'injected missing member', str(root / str(pid) / 'stat'))
            return activity.identity(root, pid)
        def departure(root, pid):
            proofs.append(pid)
            self.assertEqual(pid, child_pid)
            if proof_mode == 'invalid':
                return {'pid': pid, 'method': 'FileNotFoundError-only'}
            if proof_mode == 'terminal-reuse' and len(proofs) == 2:
                raise ValueError('PID reused at terminal departure check')
            if proof_mode == 'live-pidfd':
                with patch.object(activity.os, 'pidfd_open', return_value=-1), \
                     patch.object(activity.os, 'close', return_value=None):
                    return activity.confirm_departure(root, pid)
            value = activity.confirm_departure(root, pid)
            if proof_mode == 'deadline':
                elapsed[0] = 16
            return value
        adapter = SimpleNamespace(device_identities=lambda _: {(999, 0): ['/dev/kfd'],
            (226, 128): ['/dev/dri/renderD128']}, scan=scanner, identity=identities, pids=census,
            confirm_departure=departure, MAX_SECONDS=15, MAX_PIDS=32768,
            MAX_FDS=131072, MAX_TOTAL_FDS=1000000)
        command = lambda argv, **kwargs: subprocess.CompletedProcess(argv, 1, b'', b'')
        budget = probe.StartupBudget(adapter, clock=lambda: elapsed[0], runner=command)
        def observe(current, rules, phase, current_owners, devices, **kwargs):
            value = probe.observe(current, rules, phase, current_owners, devices,
                sysfs=lambda: [], **kwargs)
            if mutate:
                mutate(value)
            attempts.append(copy.deepcopy(value))
            return value
        wrapped = SimpleNamespace(observe=observe, startup_birth=probe.startup_birth,
            startup_departures=probe.startup_departures)
        original_stat = os.stat
        def stat_path(path, *args, **kwargs):
            if positive and str(path) == '/proc/' + str(init_pid) + '/fd/0':
                return SimpleNamespace(st_mode=stat.S_IFCHR, st_rdev=os.makedev(999, 0))
            return original_stat(path, *args, **kwargs)
        try:
            with patch.object(probe.os, 'geteuid', return_value=0), \
                 patch.object(probe.os, 'stat', side_effect=stat_path):
                result = cli.startup_container(adapter, attribution, wrapped, supplier,
                    ['/dev/kfd', '/dev/dri/renderD128'], budget)
            self.assertEqual(result['startup_reconciliation']['attempts'], attempts)
            self.assertFalse(result['startup_reconciliation']['timing_admitted'])
            return result, attempts, proofs
        finally:
            stop()
            child.stdin.close()

    def test_real_noninit_child_exit_during_scan_requires_proof_and_fresh_acceptance(self):
        result, attempts, proofs = self.exercise()
        self.assertTrue(result['accepted'])
        self.assertEqual(len(attempts), 2)
        self.assertFalse(attempts[0]['accepted'])
        self.assertFalse(attempts[0]['descriptor_scan']['complete'])
        self.assertEqual(attempts[0]['descriptor_scan']['errors'],
                         ['ValueError: allowlisted worker disappeared during scan'])
        self.assertEqual(len(proofs), 2)
        self.assertEqual(proofs[0], proofs[1])
        self.assertTrue(all(row['accepted'] for row in result['startup_reconciliation']['departure_checks']))

    def test_real_noninit_exit_after_complete_scan_is_rechecked(self):
        result, attempts, proofs = self.exercise(mode='after')
        self.assertTrue(result['accepted'])
        self.assertTrue(attempts[0]['descriptor_scan']['complete'])
        self.assertTrue(attempts[0]['descriptor_scan']['accepted'])
        self.assertEqual(len(proofs), 2)

    def test_alive_but_missing_proc_member_is_not_departure_proof(self):
        result, attempts, proofs = self.exercise(mode='alive', deny='missing')
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 1)
        self.assertEqual(len(proofs), 1)
        self.assertFalse(result['startup_reconciliation']['departure_checks'][0]['accepted'])

    def test_live_permission_failure_never_requests_departure_proof(self):
        result, attempts, proofs = self.exercise(mode='alive', deny='permission')
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 1)
        self.assertEqual(proofs, [])

    def test_positive_gpu_hit_before_child_exit_is_sticky(self):
        result, attempts, proofs = self.exercise(positive=True)
        self.assertFalse(result['accepted'])
        self.assertTrue(attempts[0]['descriptor_scan']['device_users'])
        self.assertEqual(proofs, [])

    def test_init_loss_is_not_an_eligible_child_departure(self):
        def change(allowed, receipt, init, child):
            allowed.clear()
            receipt['host_pids'] = []
        result, _, proofs = self.exercise(change=change)
        self.assertFalse(result['accepted'])
        self.assertEqual(proofs, [])

    def test_container_restart_is_not_an_eligible_child_departure(self):
        def change(_allowed, receipt, _init, _child):
            receipt['state']['started_at'] = 'restart'
        result, _, proofs = self.exercise(change=change)
        self.assertFalse(result['accepted'])
        self.assertEqual(proofs, [])

    def test_surviving_owner_identity_change_is_terminal(self):
        def change(allowed, _receipt, init, _child):
            allowed[init]['start_time_ticks'] += 1
        result, _, proofs = self.exercise(change=change)
        self.assertFalse(result['accepted'])
        self.assertEqual(proofs, [])

    def test_pid_number_reappearance_in_fresh_census_is_terminal(self):
        result, attempts, proofs = self.exercise(proof_mode='census-reuse')
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 2)
        self.assertIn('departed startup PID reappeared', attempts[1]['descriptor_scan']['errors'][0])
        self.assertEqual(len(proofs), 1)

    def test_terminal_pid_reuse_invalidates_otherwise_stable_accepted_scan(self):
        result, attempts, proofs = self.exercise(proof_mode='terminal-reuse')
        self.assertFalse(result['accepted'])
        self.assertTrue(attempts[1]['accepted'])
        self.assertIn('startup final departure check', result['errors'][0])
        self.assertEqual(len(proofs), 2)

    def test_absent_directory_but_live_pidfd_refuses(self):
        result, attempts, proofs = self.exercise(proof_mode='live-pidfd')
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 1)
        self.assertEqual(len(proofs), 1)

    def test_file_not_found_only_proof_is_rejected(self):
        result, attempts, proofs = self.exercise(proof_mode='invalid')
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 1)
        self.assertEqual(len(proofs), 1)

    def test_departure_proof_consumes_shared_deadline(self):
        result, attempts, proofs = self.exercise(proof_mode='deadline')
        self.assertFalse(result['accepted'])
        self.assertEqual(len(attempts), 1)
        self.assertIn('deadline', result['startup_reconciliation']['departure_checks'][0]['error'])
        self.assertEqual(len(proofs), 1)

    def test_unrelated_coverage_failure_cannot_be_relabelled_departure(self):
        def mutate(value):
            value['descriptor_scan']['errors'] = ['ValueError: descriptor scan deadline exceeded']
        result, _, proofs = self.exercise(mutate=mutate)
        self.assertFalse(result['accepted'])
        self.assertEqual(proofs, [])


if __name__ == '__main__':
    unittest.main()
