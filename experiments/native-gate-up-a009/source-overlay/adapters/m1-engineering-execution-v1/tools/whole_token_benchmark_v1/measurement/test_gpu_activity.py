import importlib.util
import json
import os
from pathlib import Path
import stat
import tempfile
import types
import unittest


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
        self.assertTrue(row["complete"])

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

    def test_census_churn_rejected(self):
        calls = []
        def census(_root):
            calls.append(1)
            return {10} if len(calls) == 1 else {10, 11}
        row = self.scan(pid_reader=census)
        self.assertFalse(row["accepted"])
        self.assertIn("census changed", row["errors"][0])

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


if __name__ == "__main__":
    unittest.main()
