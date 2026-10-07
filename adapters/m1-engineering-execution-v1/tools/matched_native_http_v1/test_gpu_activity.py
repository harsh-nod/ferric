import importlib.util
import errno
import json
import os
from pathlib import Path
import shutil
import stat
import tempfile
import types
import unittest
from unittest import mock


SPEC = importlib.util.spec_from_file_location("gpu_activity", Path(__file__).with_name("gpu_activity.py"))
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)


def process_identity(pid):
    return {"pid": pid, "start_time_ticks": 12345, "uids": [1000] * 4, "proc_uid": 1000,
            "pid_namespace": [1, 23], "cgroup": "0::/docker/test\n"}


class DeviceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.proc = Path(self.temp.name)
        (self.proc / "10/fd").mkdir(parents=True)
        (self.proc / "10/fd/5").write_bytes(b"fixture, not a GPU")
        self.devices = {(235, 0): ["/dev/kfd"]}
        self.allowed = {10: process_identity(10)}

    def scan(self, **kwargs):
        defaults = {"identity_reader": lambda _root, pid: process_identity(pid),
                    "pid_reader": lambda _root: {10},
                    "descriptor_stat": lambda _path: types.SimpleNamespace(st_mode=stat.S_IFCHR, st_rdev=os.makedev(235, 0))}
        defaults.update(kwargs)
        return m.scan(self.proc, self.devices, self.allowed, "active", **defaults)

    def test_positive_cross_namespace_rdev_attribution(self):
        row = self.scan()
        self.assertTrue(row["accepted"])
        self.assertEqual(row["owned_users"][0]["descriptors"], [{"fd": 5, "major": 235, "minor": 0}])

    def test_regular_file_named_like_kfd_not_accepted(self):
        row = self.scan(descriptor_stat=lambda _: types.SimpleNamespace(st_mode=stat.S_IFREG, st_rdev=os.makedev(235, 0)))
        self.assertFalse(row["accepted"])
        self.assertIn("positive", row["errors"][0])

    def test_different_character_device_not_accepted(self):
        row = self.scan(descriptor_stat=lambda _: types.SimpleNamespace(st_mode=stat.S_IFCHR, st_rdev=os.makedev(1, 3)))
        self.assertFalse(row["accepted"])

    def test_empty_scan_not_active_proof(self):
        row = self.scan(pid_reader=lambda _: set())
        self.assertFalse(row["accepted"])
        self.assertFalse(row["complete"])

    def test_foreign_user_rejected(self):
        self.allowed = {}
        row = self.scan()
        self.assertFalse(row["accepted"])
        self.assertEqual(len(row["foreign_users"]), 1)

    def test_reused_pid_rejected(self):
        self.allowed[10]["start_time_ticks"] = 1
        self.assertFalse(self.scan()["accepted"])

    def test_identity_changed_midscan_rejected(self):
        calls = []
        def changed(_root, pid):
            calls.append(pid)
            return {**process_identity(pid), "start_time_ticks": len(calls)}
        row = self.scan(identity_reader=changed)
        self.assertFalse(row["accepted"])
        self.assertIn("identity changed", row["errors"][0])

    def test_permission_error_not_idle(self):
        def denied(_path):
            raise PermissionError("not inspectable")
        row = self.scan(descriptor_stat=denied)
        self.assertFalse(row["accepted"])
        self.assertFalse(row["complete"])

    def test_fd_disappearance_not_idle(self):
        def vanished(_path):
            raise FileNotFoundError("descriptor retired")
        row = self.scan(descriptor_stat=vanished)
        self.assertFalse(row["accepted"])
        self.assertFalse(row["complete"])

    def test_new_census_member_is_scanned(self):
        calls = []
        def census(_root):
            calls.append(1)
            if len(calls) == 2:
                (self.proc / "11/fd").mkdir(parents=True)
                (self.proc / "11/fd/5").write_bytes(b"fixture")
            return {10} if len(calls) == 1 else {10, 11}
        row = self.scan(pid_reader=census, descriptor_stat=lambda path: types.SimpleNamespace(
            st_mode=stat.S_IFREG if "/11/" in path else stat.S_IFCHR, st_rdev=os.makedev(235, 0)))
        self.assertTrue(row["accepted"])
        self.assertEqual(row["pids_scanned"], 2)
        self.assertEqual(row["descriptors_scanned"], 2)

    def test_deadline_rejected(self):
        calls = []
        def clock():
            calls.append(1)
            return 0 if len(calls) == 1 else 16
        self.assertIn("deadline", self.scan(clock=clock)["errors"][0])

    def test_endpoint_empty_is_only_descriptor_observation(self):
        row = m.scan(self.proc, self.devices, {}, "preflight", pid_reader=lambda _: set())
        self.assertTrue(row["accepted"])
        self.assertIn("not continuous", row["caveat"])

    def test_endpoint_with_owned_descriptor_rejected(self):
        row = m.scan(self.proc, self.devices, self.allowed, "postflight",
                     pid_reader=lambda _: {10}, identity_reader=lambda _, pid: process_identity(pid),
                     descriptor_stat=lambda _: types.SimpleNamespace(st_mode=stat.S_IFCHR, st_rdev=os.makedev(235, 0)))
        self.assertFalse(row["accepted"])

    def test_identity_parser_comm_with_parentheses(self):
        directory = self.proc / "10"
        (directory / "stat").write_text("10 (worker (name)) " + " ".join(["S"] + ["0"] * 18 + ["12345"]) + "\n")
        (directory / "status").write_text("Name:\tworker\nUid:\t1000\t1000\t1000\t1000\n")
        (directory / "cgroup").write_text("0::/docker/test\n")
        (directory / "ns").mkdir()
        (directory / "ns/pid").write_text("fixture")
        value = m.identity(self.proc, 10)
        self.assertEqual(value["start_time_ticks"], 12345)


class ContainerTests(unittest.TestCase):
    def setUp(self):
        self.binding = {"id": "a" * 64, "image": "sha256:" + "b" * 64, "name": "ferric-v14-test",
                        "label_key": "ferric.v14", "label_value": "ferric-v14-test"}
        self.inspect = {"Id": self.binding["id"], "Image": self.binding["image"], "Name": "/ferric-v14-test",
                        "Config": {"Labels": {"ferric.v14": "ferric-v14-test"}},
                        "State": {"Running": True, "Paused": False, "OOMKilled": False,
                                  "Pid": 10, "StartedAt": "2026-10-05T00:00:00Z"}}
        self.calls = []

    def runner(self, argv, **_kwargs):
        self.calls.append(argv)
        raw = json.dumps([self.inspect]).encode() if argv[1] == "inspect" else b"PID\n10\n11\n"
        return types.SimpleNamespace(returncode=0, stdout=raw, stderr=b"")

    def members(self, **kwargs):
        return m.container_members(self.binding, Path("/unused"), runner=kwargs.get("runner", self.runner),
                                   identity_reader=lambda _, pid: process_identity(pid))

    def test_exact_container_membership(self):
        members, receipt = self.members()
        self.assertEqual(set(members), {10, 11})
        self.assertEqual(receipt["host_pids"], [10, 11])
        self.assertTrue(all(call[1] in ("inspect", "top") for call in self.calls))

    def test_wrong_image_rejected(self):
        self.inspect["Image"] = "sha256:" + "c" * 64
        with self.assertRaisesRegex(ValueError, "identity"):
            self.members()

    def test_wrong_label_rejected(self):
        self.inspect["Config"]["Labels"] = {}
        with self.assertRaisesRegex(ValueError, "identity"):
            self.members()

    def test_oom_rejected(self):
        self.inspect["State"]["OOMKilled"] = True
        with self.assertRaisesRegex(ValueError, "identity"):
            self.members()

    def test_container_restart_rejected(self):
        inspections = []
        def restarted(argv, **kwargs):
            if argv[1] == "inspect":
                inspections.append(1)
                self.inspect["State"]["StartedAt"] = str(len(inspections))
            return self.runner(argv, **kwargs)
        with self.assertRaisesRegex(ValueError, "restarted"):
            self.members(runner=restarted)

    def test_no_container_membership_from_empty_top(self):
        def empty(argv, **kwargs):
            result = self.runner(argv, **kwargs)
            if argv[1] == "top":
                result.stdout = b"PID\n"
            return result
        with self.assertRaisesRegex(ValueError, "nonempty"):
            self.members(runner=empty)

    def test_mutating_docker_command_forbidden(self):
        with self.assertRaisesRegex(ValueError, "read-only"):
            m.docker_json(["docker", "kill", self.binding["id"]], self.runner)


def gone():
    return FileNotFoundError(errno.ENOENT, "fixture process is absent")


class DepartureProofTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.proc = Path(self.temp.name)
        self.calls = []

    def esrch(self, pid, flags):
        self.calls.append((pid, flags))
        raise ProcessLookupError(errno.ESRCH, "fixture process does not exist")

    def test_fully_absent_process_requires_two_pidfd_esrch_observations(self):
        value = m.confirm_departure(self.proc, 10, pidfd_opener=self.esrch)
        self.assertEqual(value, {"pid": 10, "method": "proc-directory-absent-and-pidfd-esrch",
                                 "proc_directory_absent_checks": 2, "pidfd_esrch_checks": 2})
        self.assertEqual(self.calls, [(10, 0), (10, 0)])

    def test_existing_proc_directory_is_not_departure(self):
        (self.proc / "10").mkdir()
        with self.assertRaisesRegex(ValueError, "still-present"):
            m.confirm_departure(self.proc, 10, pidfd_opener=self.esrch)
        self.assertFalse(self.calls)

    def test_successful_pidfd_open_is_live_even_if_proc_is_hidden(self):
        descriptors = []
        def live(_pid, _flags):
            descriptor = os.open(os.devnull, os.O_RDONLY)
            descriptors.append(descriptor)
            return descriptor
        with self.assertRaisesRegex(ValueError, "still exists"):
            m.confirm_departure(self.proc, 10, pidfd_opener=live)
        with self.assertRaises(OSError):
            os.fstat(descriptors[0])

    def test_pidfd_permission_denied_is_not_departure(self):
        def denied(_pid, _flags):
            raise PermissionError(errno.EPERM, "not permitted")
        with self.assertRaises(PermissionError):
            m.confirm_departure(self.proc, 10, pidfd_opener=denied)

    def test_proc_permission_denied_is_not_departure(self):
        def denied(_path):
            raise PermissionError(errno.EACCES, "not inspectable")
        with self.assertRaises(PermissionError):
            m.confirm_departure(self.proc, 10, pidfd_opener=self.esrch, directory_stat=denied)
        self.assertFalse(self.calls)

    def test_proc_reappearance_between_checks_is_rejected(self):
        def exited(pid, flags):
            (self.proc / str(pid)).mkdir()
            return self.esrch(pid, flags)
        with self.assertRaisesRegex(ValueError, "still-present"):
            m.confirm_departure(self.proc, 10, pidfd_opener=exited)

    def test_pid_reuse_between_pidfd_checks_is_rejected(self):
        count = []
        def raced(pid, flags):
            count.append(1)
            if len(count) == 1:
                return self.esrch(pid, flags)
            return os.open(os.devnull, os.O_RDONLY)
        with self.assertRaisesRegex(ValueError, "still exists"):
            m.confirm_departure(self.proc, 10, pidfd_opener=raced)

    def test_unsupported_pidfd_is_not_departure(self):
        with mock.patch.object(m.os, "pidfd_open", None):
            with self.assertRaisesRegex(ValueError, "pidfd_open is required"):
                m.confirm_departure(self.proc, 10)

    def test_other_pidfd_error_is_not_departure(self):
        def failed(_pid, _flags):
            raise OSError(errno.ENOSYS, "unsupported")
        with self.assertRaises(OSError):
            m.confirm_departure(self.proc, 10, pidfd_opener=failed)

    def test_untyped_missing_directory_error_is_rejected(self):
        def missing(_path):
            raise FileNotFoundError("no exact errno")
        with self.assertRaisesRegex(ValueError, "exact absent"):
            m.confirm_departure(self.proc, 10, pidfd_opener=self.esrch, directory_stat=missing)


class ReconciliationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.proc = Path(self.temp.name)
        self.alive = {}
        self.gpu_pids = set()
        self.identity_calls = {}
        self.devices = {(235, 0): ["/dev/kfd"]}
        self.add(10)

    def add(self, pid, start=100):
        (self.proc / str(pid) / "fd").mkdir(parents=True)
        (self.proc / str(pid) / "fd/5").write_bytes(b"fixture")
        self.alive[pid] = {"pid": pid, "start_time_ticks": start, "uids": [1000] * 4,
                           "proc_uid": 1000, "pid_namespace": [1, 23], "cgroup": "0::/fixture\n"}

    def remove(self, pid):
        shutil.rmtree(self.proc / str(pid))
        self.alive.pop(pid)

    def identity(self, _proc, pid):
        self.identity_calls[pid] = self.identity_calls.get(pid, 0) + 1
        if pid not in self.alive:
            raise gone()
        return dict(self.alive[pid])

    def descriptor(self, path):
        pid = int(Path(path).parent.parent.name)
        return types.SimpleNamespace(st_mode=stat.S_IFCHR if pid in self.gpu_pids else stat.S_IFREG,
                                     st_rdev=os.makedev(235, 0))

    def departure(self, proc, pid):
        def exited(_pid, _flags):
            self.assertNotIn(pid, self.alive)
            raise ProcessLookupError(errno.ESRCH, "fixture process does not exist")
        return m.confirm_departure(proc, pid, pidfd_opener=exited)

    def scan(self, *, allowed=None, phase="preflight", **kwargs):
        values = {"identity_reader": self.identity, "descriptor_stat": self.descriptor,
                  "departure_reader": self.departure, "pid_reader": lambda _: set(self.alive),
                  "effective_uid_reader": lambda: 1000}
        values.update(kwargs)
        return m.scan(self.proc, self.devices, allowed or {}, phase, **values)

    def test_stable_process_is_scanned_and_lifetime_rechecked(self):
        row = self.scan()
        self.assertTrue(row["accepted"])
        self.assertEqual(self.identity_calls[10], 3)
        self.assertEqual(row["descriptors_scanned"], 1)
        self.assertEqual(row["departed_processes"], [])
        self.assertIn("not continuous", row["caveat"])

    def test_departure_before_initial_identity_is_retained(self):
        def missing(proc, pid):
            self.remove(pid)
            return self.identity(proc, pid)
        row = self.scan(identity_reader=missing)
        self.assertTrue(row["accepted"])
        self.assertIsNone(row["departed_processes"][0]["identity"])
        self.assertEqual(row["departed_processes"][0]["proof"]["pid"], 10)

    def test_departure_while_reading_fd_is_retained(self):
        def missing(_path):
            self.remove(10)
            raise gone()
        row = self.scan(descriptor_stat=missing)
        self.assertTrue(row["accepted"])
        self.assertEqual(row["departed_processes"][0]["identity"]["start_time_ticks"], 100)

    def test_departure_at_post_scan_identity_is_retained(self):
        def retiring(proc, pid):
            if self.identity_calls.get(pid, 0) == 1:
                self.remove(pid)
            return self.identity(proc, pid)
        row = self.scan(identity_reader=retiring)
        self.assertTrue(row["accepted"])
        self.assertEqual(len(row["departed_processes"]), 1)

    def test_departure_during_final_identity_reconciliation_is_retained(self):
        def retiring(proc, pid):
            if self.identity_calls.get(pid, 0) == 2:
                self.remove(pid)
            return self.identity(proc, pid)
        row = self.scan(identity_reader=retiring)
        self.assertTrue(row["accepted"])
        self.assertIn("identity-reconciliation", row["departed_processes"][0]["reason"])

    def test_departure_seen_in_followup_census_is_retained(self):
        calls = []
        def census(_proc):
            calls.append(1)
            if len(calls) == 2:
                self.remove(10)
            return set(self.alive)
        row = self.scan(pid_reader=census)
        self.assertTrue(row["accepted"])
        self.assertEqual(row["departed_processes"][0]["reason"], "absent-from-followup-census")

    def test_missing_fd_of_live_process_is_rejected(self):
        def missing(_path):
            raise gone()
        row = self.scan(descriptor_stat=missing)
        self.assertFalse(row["accepted"])
        self.assertIn("still-present", row["errors"][0])
        self.assertFalse(row["departed_processes"])

    def test_identity_access_denied_is_rejected_without_departure(self):
        def denied(_proc, _pid):
            raise PermissionError(errno.EACCES, "cannot inspect live identity")
        row = self.scan(identity_reader=denied)
        self.assertFalse(row["accepted"])
        self.assertIn("PermissionError", row["errors"][0])
        self.assertFalse(row["departed_processes"])

    def test_descriptor_access_denied_is_rejected_without_departure(self):
        def denied(_path):
            raise PermissionError(errno.EACCES, "cannot inspect live FD")
        row = self.scan(descriptor_stat=denied)
        self.assertFalse(row["accepted"])
        self.assertFalse(row["departed_processes"])

    def test_new_live_non_gpu_process_is_scanned(self):
        calls = []
        def census(_proc):
            calls.append(1)
            if len(calls) == 2:
                self.add(11)
            return set(self.alive)
        row = self.scan(pid_reader=census)
        self.assertTrue(row["accepted"])
        self.assertEqual(row["descriptors_scanned"], 2)
        self.assertEqual(row["pids_scanned"], 2)
        self.assertEqual(self.identity_calls[11], 3)

    def test_new_foreign_gpu_process_is_rejected(self):
        calls = []
        def census(_proc):
            calls.append(1)
            if len(calls) == 2:
                self.add(11)
                self.gpu_pids.add(11)
            return set(self.alive)
        row = self.scan(pid_reader=census)
        self.assertFalse(row["accepted"])
        self.assertEqual(row["foreign_users"][0]["identity"]["pid"], 11)

    def test_departed_pid_reappearance_is_rejected(self):
        calls = []
        def census(_proc):
            calls.append(1)
            if len(calls) == 2:
                self.add(10, start=200)
            return set(self.alive)
        def missing(proc, pid):
            self.remove(pid)
            return self.identity(proc, pid)
        row = self.scan(pid_reader=census, identity_reader=missing)
        self.assertFalse(row["accepted"])
        self.assertIn("reappeared", row["errors"][0])

    def test_surviving_pid_reuse_is_rejected(self):
        def reused(proc, pid):
            if self.identity_calls.get(pid, 0) == 2:
                self.alive[pid]["start_time_ticks"] = 200
            return self.identity(proc, pid)
        row = self.scan(identity_reader=reused)
        self.assertFalse(row["accepted"])
        self.assertIn("identity changed during reconciliation", row["errors"][0])

    def test_allowlisted_worker_departure_is_rejected(self):
        allowed = {10: dict(self.alive[10])}
        def missing(proc, pid):
            self.remove(pid)
            return self.identity(proc, pid)
        row = self.scan(allowed=allowed, phase="active", identity_reader=missing)
        self.assertFalse(row["accepted"])
        self.assertIn("allowlisted worker disappeared", row["errors"][0])
        self.assertFalse(row["departed_processes"])

    def test_allowlisted_pid_reused_before_first_read_fails_even_with_other_owned_gpu(self):
        self.add(20)
        self.gpu_pids.add(20)
        allowed = {pid: dict(value) for pid, value in self.alive.items()}
        self.alive[10]["start_time_ticks"] = 200
        row = self.scan(allowed=allowed, phase="active")
        self.assertFalse(row["accepted"])
        self.assertIn("allowlisted PID identity changed", row["errors"][0])

    def test_observed_foreign_gpu_owner_exit_still_fails_and_retains_hit(self):
        self.gpu_pids.add(10)
        def retiring(proc, pid):
            if self.identity_calls.get(pid, 0) == 1:
                self.remove(pid)
            return self.identity(proc, pid)
        row = self.scan(identity_reader=retiring)
        self.assertFalse(row["accepted"])
        self.assertIn("GPU descriptor owner disappeared", row["errors"][0])
        self.assertEqual(len(row["device_users"]), 1)
        self.assertEqual(len(row["foreign_users"]), 1)
        self.assertFalse(row["departed_processes"])

    def test_observed_gpu_owner_exit_at_census_still_fails(self):
        self.gpu_pids.add(10)
        calls = []
        def census(_proc):
            calls.append(1)
            if len(calls) == 2:
                self.remove(10)
            return set(self.alive)
        row = self.scan(pid_reader=census)
        self.assertFalse(row["accepted"])
        self.assertEqual(len(row["foreign_users"]), 1)

    def test_positive_owned_gpu_attribution_remains_required(self):
        row = self.scan(allowed={10: dict(self.alive[10])}, phase="active")
        self.assertFalse(row["accepted"])
        self.assertIn("positive owned", row["errors"][0])

    def test_normal_departure_does_not_remove_positive_owned_attribution(self):
        self.add(11)
        self.gpu_pids.add(10)
        def retiring(proc, pid):
            if pid == 11:
                self.remove(pid)
            return self.identity(proc, pid)
        row = self.scan(allowed={10: dict(self.alive[10])}, phase="active", identity_reader=retiring)
        self.assertTrue(row["accepted"])
        self.assertEqual(row["owned_users"][0]["identity"]["pid"], 10)
        self.assertEqual(row["departed_processes"][0]["proof"]["pid"], 11)

    def test_never_quiet_births_have_finite_roster_and_retained_late_births(self):
        calls = []
        def census(_proc):
            calls.append(1)
            if len(calls) > 1:
                self.add(9 + len(calls))
            return set(self.alive)
        row = self.scan(pid_reader=census)
        self.assertTrue(row["accepted"])
        self.assertEqual(len(calls), 3)
        self.assertEqual(row["roster_pids"], [10, 11])
        self.assertEqual(row["late_birth_pids"], [12])
        self.assertEqual(row["pids_scanned"], 2)
        self.assertEqual(row["pids_observed"], 3)
        self.assertNotIn(12, self.identity_calls)

    def test_pid_union_bound_is_not_reset_between_censuses(self):
        calls = []
        def census(_proc):
            calls.append(1)
            if len(calls) == 2:
                self.add(11)
            return set(self.alive)
        with mock.patch.object(m, "MAX_PIDS", 1):
            row = self.scan(pid_reader=census)
        self.assertFalse(row["accepted"])
        self.assertIn("union exceeds", row["errors"][0])

    def test_total_fd_bound_remains_in_force(self):
        self.add(11)
        with mock.patch.object(m, "MAX_TOTAL_FDS", 1):
            row = self.scan()
        self.assertFalse(row["accepted"])
        self.assertIn("count exceeds", row["errors"][0])

    def test_deadline_remains_in_force_during_reconciliation(self):
        elapsed = [0]
        def identity(proc, pid):
            value = self.identity(proc, pid)
            if self.identity_calls[pid] == 3:
                elapsed[0] = 16
            return value
        row = self.scan(clock=lambda: elapsed[0], identity_reader=identity)
        self.assertFalse(row["accepted"])
        self.assertIn("deadline", row["errors"][0])

    def test_forged_departure_receipt_is_rejected(self):
        def missing(proc, pid):
            self.remove(pid)
            return self.identity(proc, pid)
        row = self.scan(identity_reader=missing, departure_reader=lambda *_: {"pid": 10})
        self.assertFalse(row["accepted"])
        self.assertIn("exact positively departed", row["errors"][0])

    def test_descriptor_sample_never_claims_native_admission(self):
        row = self.scan()
        self.assertTrue(row["accepted"])
        self.assertFalse(row["diagnostic_only"])
        self.assertFalse(row["native_launch_admitted"])
        self.assertEqual(row["failure_diagnostics"], [])

    def test_reconciliation_cgroup_drift_rescans_and_retains_both_identities(self):
        def changed(proc, pid):
            if self.identity_calls.get(pid, 0) == 2:
                self.alive[pid]["cgroup"] = "0::/changed\n"
            return self.identity(proc, pid)
        row = self.scan(identity_reader=changed)
        self.assertTrue(row["accepted"])
        self.assertEqual(row["failure_diagnostics"], [])
        context = row["process_attempt_events"][0]["context"]
        self.assertEqual(context["pid"], 10)
        self.assertEqual(context["operation"], "identity_reconciliation")
        self.assertEqual(context["initial_identity"]["cgroup"], "0::/fixture\n")
        self.assertEqual(context["final_identity"]["cgroup"], "0::/changed\n")
        self.assertEqual(context["changed_identity_fields"], ["cgroup"])

    def test_midscan_uid_drift_retains_exact_changed_field(self):
        def changed(proc, pid):
            if self.identity_calls.get(pid, 0) == 1:
                self.alive[pid]["uids"] = [1001] * 4
            return self.identity(proc, pid)
        row = self.scan(identity_reader=changed)
        self.assertFalse(row["accepted"])
        context = row["failure_diagnostics"][0]["context"]
        self.assertEqual(context["operation"], "identity_after_descriptors")
        self.assertEqual(context["changed_identity_fields"], ["uids"])
        self.assertEqual(context["initial_identity"]["uids"], [1000] * 4)
        self.assertEqual(context["final_identity"]["uids"], [1001] * 4)

    def test_allowlist_mismatch_retains_expected_and_observed_identity(self):
        allowed = {10: dict(self.alive[10])}
        self.alive[10]["start_time_ticks"] = 200
        row = self.scan(allowed=allowed, phase="active")
        self.assertFalse(row["accepted"])
        context = row["failure_diagnostics"][0]["context"]
        self.assertEqual(context["operation"], "allowed_identity_validation")
        self.assertEqual(context["allowed_identity"]["start_time_ticks"], 100)
        self.assertEqual(context["initial_identity"]["start_time_ticks"], 200)
        self.assertEqual(context["allowed_changed_identity_fields"], ["start_time_ticks"])

    def diagnostic_stat(self, state):
        (self.proc / "10/stat").write_text("10 (fixture (name)) " + " ".join(
            [state, "1"] + ["0"] * 17 + ["100"]) + "\n")

    def test_live_fd_close_retains_original_missing_path_after_departure_refusal(self):
        self.diagnostic_stat("S")
        def missing(path):
            raise FileNotFoundError(errno.ENOENT, "descriptor closed", path)
        row = self.scan(descriptor_stat=missing)
        self.assertFalse(row["accepted"])
        diagnostic = row["failure_diagnostics"][0]
        context = diagnostic["context"]
        self.assertEqual(context["operation"], "departure_proof")
        self.assertEqual(context["pid"], 10)
        self.assertEqual(context["triggering_context"]["operation"], "descriptor_stat")
        self.assertEqual(context["triggering_missing_proc_error"]["filename"], str(self.proc / "10/fd/5"))
        self.assertEqual(context["triggering_missing_proc_error"]["errno"], errno.ENOENT)
        self.assertEqual(diagnostic["post_refusal_process"]["stat"]["state"], "S")
        self.assertEqual(diagnostic["post_refusal_process"]["stat"]["comm"], "fixture (name)")

    def test_zombie_missing_namespace_is_diagnosed_but_still_refused(self):
        self.diagnostic_stat("Z")
        def missing(_proc, _pid):
            raise FileNotFoundError(errno.ENOENT, "namespace unavailable", str(self.proc / "10/ns/pid"))
        row = self.scan(identity_reader=missing)
        self.assertFalse(row["accepted"])
        self.assertFalse(row["complete"])
        diagnostic = row["failure_diagnostics"][0]
        self.assertIsNone(diagnostic["context"]["initial_identity"])
        self.assertEqual(diagnostic["context"]["triggering_context"]["operation"], "identity_initial")
        self.assertEqual(diagnostic["context"]["triggering_missing_proc_error"]["filename"], str(self.proc / "10/ns/pid"))
        self.assertEqual(diagnostic["post_refusal_process"]["stat"]["state"], "Z")
        self.assertEqual(row["departed_processes"], [])

    def test_permission_failure_keeps_actual_fd_path_and_errno(self):
        def denied(path):
            raise PermissionError(errno.EACCES, "descriptor unreadable", path)
        row = self.scan(descriptor_stat=denied)
        self.assertFalse(row["accepted"])
        diagnostic = row["failure_diagnostics"][0]
        self.assertEqual(diagnostic["context"]["operation"], "descriptor_stat")
        self.assertEqual(diagnostic["context"]["pid"], 10)
        self.assertEqual(diagnostic["error"]["errno"], errno.EACCES)
        self.assertEqual(diagnostic["error"]["filename"], str(self.proc / "10/fd/5"))

    def test_exited_gpu_owner_diagnostics_do_not_erase_hit_or_change_refusal(self):
        self.gpu_pids.add(10)
        def retiring(proc, pid):
            if self.identity_calls.get(pid, 0) == 1:
                self.remove(pid)
            return self.identity(proc, pid)
        row = self.scan(identity_reader=retiring)
        self.assertFalse(row["accepted"])
        self.assertEqual(len(row["foreign_users"]), 1)
        diagnostic = row["failure_diagnostics"][0]
        self.assertEqual(diagnostic["context"]["pid"], 10)
        self.assertEqual(diagnostic["post_refusal_process"]["directory_error"]["errno"], errno.ENOENT)

    def test_post_refusal_observation_is_skipped_after_original_deadline(self):
        elapsed = [0]
        def denied(_proc, _pid):
            elapsed[0] = 16
            raise PermissionError(errno.EACCES, "identity unreadable")
        with mock.patch.object(m, "refusal_process_observation") as observer:
            row = self.scan(identity_reader=denied, clock=lambda: elapsed[0])
        self.assertFalse(row["accepted"])
        observer.assert_not_called()
        self.assertEqual(row["failure_diagnostics"][0]["post_refusal_process"]["skipped"],
                         "original scan deadline elapsed")

    def close_once(self, on_close=None):
        calls = []
        def descriptor(path):
            calls.append(path)
            if len(calls) == 1:
                if on_close is not None:
                    on_close()
                raise FileNotFoundError(errno.ENOENT, "descriptor closed", path)
            return self.descriptor(path)
        return descriptor, calls

    def test_fd_close_reopens_complete_directory_and_scans_new_fd(self):
        descriptor, calls = self.close_once(lambda: (self.proc / "10/fd/6").write_bytes(b"new"))
        row = self.scan(descriptor_stat=descriptor)
        self.assertTrue(row["accepted"])
        self.assertEqual(row["descriptors_scanned"], 3)
        self.assertEqual(sorted(Path(path).name for path in calls[1:]), ["5", "6"])
        self.assertTrue(row["process_attempt_events"][0]["retry_requested"])
        self.assertEqual(row["process_attempt_events"][-1]["reason"], "complete-stable-rescan")

    def test_retry_new_foreign_gpu_hit_refuses_and_retains_it(self):
        descriptor, _ = self.close_once(lambda: self.gpu_pids.add(10))
        row = self.scan(descriptor_stat=descriptor)
        self.assertFalse(row["accepted"])
        self.assertEqual(row["foreign_users"][0]["identity"]["pid"], 10)
        self.assertIn("foreign or unbound", row["errors"][0])

    def test_positive_gpu_hit_forbids_retry_after_cgroup_drift(self):
        self.gpu_pids.add(10)
        def changed(proc, pid):
            if self.identity_calls.get(pid, 0) == 1:
                self.alive[pid]["cgroup"] = "0::/new\n"
            return self.identity(proc, pid)
        row = self.scan(identity_reader=changed)
        self.assertFalse(row["accepted"])
        self.assertEqual(len(row["foreign_users"]), 1)
        self.assertFalse(row["process_attempt_events"][0]["retry_requested"])
        self.assertIn("owner cannot be rescanned", row["errors"][0])

    def test_positive_gpu_hit_before_missing_fd_is_never_retried(self):
        (self.proc / "10/fd/6").write_bytes(b"second")
        calls = []
        def descriptor(path):
            calls.append(path)
            if len(calls) == 1:
                return types.SimpleNamespace(st_mode=stat.S_IFCHR, st_rdev=os.makedev(235, 0))
            raise FileNotFoundError(errno.ENOENT, "descriptor closed", path)
        row = self.scan(descriptor_stat=descriptor)
        self.assertFalse(row["accepted"])
        self.assertEqual(len(calls), 2)
        self.assertEqual(len(row["foreign_users"]), 1)
        self.assertFalse(row["process_attempt_events"][0]["retry_requested"])
        self.assertIn("owner disappeared", row["errors"][0])

    def test_allowlisted_fd_close_is_not_retryable(self):
        descriptor, calls = self.close_once()
        row = self.scan(allowed={10: dict(self.alive[10])}, phase="active", descriptor_stat=descriptor)
        self.assertFalse(row["accepted"])
        self.assertEqual(len(calls), 1)
        self.assertFalse(row["process_attempt_events"][0]["retry_requested"])

    def test_midscan_cgroup_change_requires_complete_new_scan(self):
        def changed(proc, pid):
            if self.identity_calls.get(pid, 0) == 1:
                self.alive[pid]["cgroup"] = "0::/new\n"
            return self.identity(proc, pid)
        row = self.scan(identity_reader=changed)
        self.assertTrue(row["accepted"])
        self.assertEqual(row["descriptors_scanned"], 2)
        self.assertEqual(row["process_attempt_events"][0]["reason"], "cgroup-changed-during-descriptor-scan")
        self.assertEqual(row["process_attempt_events"][-1]["identity"]["cgroup"], "0::/new\n")

    def test_allowlisted_cgroup_change_is_not_retryable(self):
        allowed = {10: dict(self.alive[10])}
        def changed(proc, pid):
            if self.identity_calls.get(pid, 0) == 1:
                self.alive[pid]["cgroup"] = "0::/new\n"
            return self.identity(proc, pid)
        row = self.scan(allowed=allowed, phase="active", identity_reader=changed)
        self.assertFalse(row["accepted"])
        self.assertIn("allowlisted worker cannot be rescanned", row["errors"][0])

    def assert_retry_drift_rejected(self, field, value):
        descriptor, calls = self.close_once(lambda: self.alive[10].update({field: value}))
        row = self.scan(descriptor_stat=descriptor)
        self.assertFalse(row["accepted"])
        self.assertEqual(len(calls), 1)
        self.assertIn("changed before complete rescan", row["errors"][0])
        self.assertEqual(row["failure_diagnostics"][0]["context"]["anchor_changed_fields"], [field])

    def test_pid_reuse_before_retry_refuses(self):
        self.assert_retry_drift_rejected("start_time_ticks", 200)

    def test_namespace_change_before_retry_refuses(self):
        self.assert_retry_drift_rejected("pid_namespace", [1, 24])

    def test_uid_change_before_retry_refuses(self):
        self.assert_retry_drift_rejected("uids", [1001] * 4)

    def test_proc_owner_change_before_retry_refuses(self):
        self.assert_retry_drift_rejected("proc_uid", 1001)

    def test_access_denied_on_retry_refuses_without_skipping(self):
        first, calls = self.close_once()
        def descriptor(path):
            if calls:
                raise PermissionError(errno.EACCES, "unreadable FD", path)
            return first(path)
        row = self.scan(descriptor_stat=descriptor)
        self.assertFalse(row["accepted"])
        self.assertIn("PermissionError", row["errors"][0])
        self.assertEqual(row["descriptors_scanned"], 2)

    def test_persistent_missing_fd_exhausts_exactly_three_attempts(self):
        def missing(path):
            raise FileNotFoundError(errno.ENOENT, "descriptor closed", path)
        row = self.scan(descriptor_stat=missing)
        self.assertFalse(row["accepted"])
        self.assertEqual(row["descriptors_scanned"], 3)
        events = row["process_attempt_events"]
        self.assertEqual([event["attempt"] for event in events], [1, 2, 3])
        self.assertEqual([event["retry_requested"] for event in events], [True, True, False])
        self.assertIn("attempt bound", row["errors"][0])

    def test_wrong_missing_fd_filename_is_not_retryable(self):
        def missing(_path):
            raise FileNotFoundError(errno.ENOENT, "wrong filename", str(self.proc / "10/fd/99"))
        row = self.scan(descriptor_stat=missing)
        self.assertFalse(row["accepted"])
        self.assertEqual(row["descriptors_scanned"], 1)
        self.assertFalse(row["process_attempt_events"][0]["retry_requested"])

    def test_exit_before_retry_uses_complete_departure_proof(self):
        descriptor, calls = self.close_once()
        def identity(proc, pid):
            if calls:
                self.remove(pid)
            return self.identity(proc, pid)
        row = self.scan(descriptor_stat=descriptor, identity_reader=identity)
        self.assertTrue(row["accepted"])
        self.assertEqual(row["departed_processes"][0]["proof"]["pid"], 10)
        self.assertEqual(row["departed_processes"][0]["proof"]["pidfd_esrch_checks"], 2)

    def test_missing_identity_member_during_retry_is_not_retryable(self):
        descriptor, calls = self.close_once()
        def identity(proc, pid):
            if calls:
                raise FileNotFoundError(errno.ENOENT, "namespace gone", str(self.proc / "10/ns/pid"))
            return self.identity(proc, pid)
        row = self.scan(descriptor_stat=descriptor, identity_reader=identity)
        self.assertFalse(row["accepted"])
        self.assertEqual([event["retry_requested"] for event in row["process_attempt_events"]], [True, False])
        self.assertEqual(row["departed_processes"], [])

    def test_zombie_during_retry_is_not_exempted(self):
        self.diagnostic_stat("Z")
        descriptor, calls = self.close_once()
        def identity(proc, pid):
            if calls:
                raise FileNotFoundError(errno.ENOENT, "namespace gone", str(self.proc / "10/ns/pid"))
            return self.identity(proc, pid)
        row = self.scan(descriptor_stat=descriptor, identity_reader=identity)
        self.assertFalse(row["accepted"])
        self.assertEqual(row["failure_diagnostics"][0]["post_refusal_process"]["stat"]["state"], "Z")

    def test_per_pid_fd_count_is_cumulative_across_attempts(self):
        descriptor, calls = self.close_once()
        with mock.patch.object(m, "MAX_FDS", 1):
            row = self.scan(descriptor_stat=descriptor)
        self.assertFalse(row["accepted"])
        self.assertEqual(len(calls), 1)
        self.assertIn("count exceeds", row["errors"][0])

    def test_total_fd_count_is_cumulative_across_attempts(self):
        descriptor, calls = self.close_once()
        with mock.patch.object(m, "MAX_TOTAL_FDS", 1):
            row = self.scan(descriptor_stat=descriptor)
        self.assertFalse(row["accepted"])
        self.assertEqual(len(calls), 1)
        self.assertIn("count exceeds", row["errors"][0])

    def test_original_deadline_is_not_reset_by_retry(self):
        elapsed = [0]
        descriptor, calls = self.close_once(lambda: elapsed.__setitem__(0, 16))
        row = self.scan(descriptor_stat=descriptor, clock=lambda: elapsed[0])
        self.assertFalse(row["accepted"])
        self.assertEqual(len(calls), 1)
        self.assertIn("deadline", row["errors"][0])

    def test_retry_budget_is_shared_with_reconciliation(self):
        changes = {2, 4, 7}
        def identity(proc, pid):
            count = self.identity_calls.get(pid, 0) + 1
            if count in changes:
                self.alive[pid]["cgroup"] = "0::/change%d\n" % count
            return self.identity(proc, pid)
        row = self.scan(identity_reader=identity)
        self.assertFalse(row["accepted"])
        self.assertIn("attempt bound", row["errors"][0])
        self.assertEqual(row["descriptors_scanned"], 3)

    def test_resource_limits_unchanged_and_census_bound_reduced(self):
        self.assertEqual((m.MAX_PIDS, m.MAX_FDS, m.MAX_TOTAL_FDS, m.MAX_SECONDS,
                          m.MAX_CENSUS_ROUNDS, m.MAX_PROCESS_ATTEMPTS),
                         (32768, 131072, 1000000, 15, 3, 3))

    def test_malformed_diagnostic_stat_is_retained_without_changing_refusal(self):
        (self.proc / "10/stat").write_text("malformed fixture\n")
        def denied(_proc, _pid):
            raise PermissionError(errno.EACCES, "identity unreadable")
        row = self.scan(identity_reader=denied)
        self.assertFalse(row["accepted"])
        self.assertIn("PermissionError", row["errors"][0])
        self.assertEqual(row["failure_diagnostics"][0]["post_refusal_process"]["stat_error"]["type"], "ValueError")

    def test_original_resource_bounds_remain_unchanged(self):
        self.assertEqual((m.MAX_SECONDS, m.MAX_PIDS, m.MAX_FDS, m.MAX_TOTAL_FDS, m.MAX_CENSUS_ROUNDS),
                         (15, 32768, 131072, 1000000, 3))

    def credential_change(self, *, at=1, reuse=False, namespace=False):
        def identity(proc, pid):
            if self.identity_calls.get(pid, 0) == at:
                self.alive[pid].update(uids=[9661] * 4, proc_uid=9661)
                if reuse:
                    self.alive[pid]['start_time_ticks'] = 999
                if namespace:
                    self.alive[pid]['pid_namespace'] = [1, 99]
            return self.identity(proc, pid)
        return identity

    def test_schema_explicitly_declares_roster_not_atomic_host_coverage(self):
        row = self.scan()
        self.assertEqual(row['schema'], 'FerricDeviceDescriptorSampleV2')
        self.assertEqual(row['sampling_policy'], 'initial-plus-one-birth-frontier-v1')
        self.assertIn('not atomic', row['coverage_scope'])
        self.assertIn('paired external', row['caveat'])

    def test_root_visible_nonowned_credential_transition_restarts_full_scan(self):
        row = self.scan(identity_reader=self.credential_change(), effective_uid_reader=lambda: 0)
        self.assertTrue(row['accepted'])
        self.assertEqual(row['descriptors_scanned'], 2)
        event = row['process_attempt_events'][0]
        self.assertEqual(event['reason'], 'credentials-changed-during-descriptor-scan')
        self.assertEqual(event['initial_identity']['uids'], [1000] * 4)
        self.assertEqual(event['final_identity']['uids'], [9661] * 4)
        self.assertTrue(event['retry_requested'])

    def test_nonroot_credential_transition_never_retries(self):
        row = self.scan(identity_reader=self.credential_change())
        self.assertFalse(row['accepted'])
        self.assertIn('root visibility', row['errors'][0])
        self.assertEqual(row['descriptors_scanned'], 1)

    def test_root_reconciliation_credential_transition_restarts_full_scan(self):
        row = self.scan(identity_reader=self.credential_change(at=2), effective_uid_reader=lambda: 0)
        self.assertTrue(row['accepted'])
        self.assertEqual(row['descriptors_scanned'], 2)
        self.assertEqual(row['process_attempt_events'][0]['reason'], 'credentials-changed-during-reconciliation')

    def test_credential_transition_cannot_hide_pid_reuse(self):
        row = self.scan(identity_reader=self.credential_change(reuse=True), effective_uid_reader=lambda: 0)
        self.assertFalse(row['accepted'])
        self.assertEqual(row['descriptors_scanned'], 1)

    def test_credential_transition_cannot_hide_namespace_drift(self):
        row = self.scan(identity_reader=self.credential_change(namespace=True), effective_uid_reader=lambda: 0)
        self.assertFalse(row['accepted'])
        self.assertEqual(row['descriptors_scanned'], 1)

    def test_owned_credential_transition_never_retries(self):
        row = self.scan(allowed={10: dict(self.alive[10])}, phase='active',
                        identity_reader=self.credential_change(), effective_uid_reader=lambda: 0)
        self.assertFalse(row['accepted'])
        self.assertIn('allowlisted worker', row['errors'][0])

    def test_positive_gpu_hit_forbids_credential_restart(self):
        self.gpu_pids.add(10)
        row = self.scan(identity_reader=self.credential_change(), effective_uid_reader=lambda: 0)
        self.assertFalse(row['accepted'])
        self.assertEqual(len(row['foreign_users']), 1)
        self.assertIn('owner cannot be rescanned', row['errors'][0])

    def test_gpu_opened_after_credential_restart_is_retained_and_rejected(self):
        change = self.credential_change()
        def identity(proc, pid):
            value = change(proc, pid)
            if self.identity_calls[pid] == 2:
                self.gpu_pids.add(pid)
            return value
        row = self.scan(identity_reader=identity, effective_uid_reader=lambda: 0)
        self.assertFalse(row['accepted'])
        self.assertEqual(row['foreign_users'][0]['identity']['uids'], [9661] * 4)

    def test_owned_identity_has_an_exact_terminal_recheck(self):
        self.gpu_pids.add(10)
        row = self.scan(allowed={10: dict(self.alive[10])}, phase='active',
                        identity_reader=self.credential_change(at=3), effective_uid_reader=lambda: 0)
        self.assertFalse(row['accepted'])
        self.assertIn('terminal endpoint', row['errors'][0])
        self.assertEqual(row['failure_diagnostics'][0]['context']['pid'], 10)

    def test_late_birth_is_explicitly_not_scanned_or_claimed_gpu_idle(self):
        calls = []
        def census(_proc):
            calls.append(1)
            if len(calls) == 3:
                self.add(11)
                self.gpu_pids.add(11)
            return set(self.alive)
        row = self.scan(pid_reader=census)
        self.assertTrue(row['accepted'])
        self.assertEqual(row['roster_pids'], [10])
        self.assertEqual(row['late_birth_pids'], [11])
        self.assertEqual(row['pids_scanned'], 1)
        self.assertEqual(row['pids_observed'], 2)
        self.assertIn('not continuous isolation or proof of idle GPU', row['caveat'])

    def test_late_births_still_count_against_original_pid_union_bound(self):
        calls = []
        def census(_proc):
            calls.append(1)
            if len(calls) == 3:
                self.add(11)
            return set(self.alive)
        with mock.patch.object(m, 'MAX_PIDS', 1):
            row = self.scan(pid_reader=census)
        self.assertFalse(row['accepted'])
        self.assertIn('union exceeds', row['errors'][0])

    def test_owned_terminal_recheck_cannot_exceed_original_deadline(self):
        self.gpu_pids.add(10)
        elapsed = [0]
        def identity(proc, pid):
            value = self.identity(proc, pid)
            if self.identity_calls[pid] == 4:
                elapsed[0] = 16
            return value
        row = self.scan(allowed={10: dict(self.alive[10])}, phase='active',
                        identity_reader=identity, clock=lambda: elapsed[0])
        self.assertFalse(row['accepted'])
        self.assertIn('deadline', row['errors'][0])


if __name__ == "__main__":
    unittest.main()
