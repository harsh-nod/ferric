"""Host-only replica protocol/process tests. No GPU or real controller execution."""
import copy
import os
from pathlib import Path
import tempfile
import signal
import subprocess
import sys
import time
import unittest
from unittest import mock

import replica_benchmark as launch
import test_compare_tp_batch as fixture


class ReplicaTests(unittest.TestCase):
    def setUp(self):
        self.reference = launch.encoded(fixture.synthetic_reference())
        patch = mock.patch.object(launch.reference, "REFERENCE_SHA256", launch.sha(self.reference))
        patch.start()
        self.addCleanup(patch.stop)

    def plan(self, layout="8xTP1"):
        return launch.plan(fixture.GPU_IDS, layout, self.reference, [])

    def test_three_layouts_preserve_workload_hash_and_all_requests(self):
        plans = [self.plan(layout) for layout in launch.LAYOUTS]
        self.assertEqual(len({value["workload_sha256"] for value in plans}), 1)
        for value in plans:
            launch.validate_plan(value)
            self.assertEqual(value["workload"]["total_output_tokens"], 64)
            self.assertEqual(sorted(name for replica in value["replicas"] for name in replica["request_names"]), launch.NAMES)
            self.assertEqual(value["total_row_budget"], 16 * len(value["replicas"]))

    def test_duplicate_missing_devices_requests_and_mutations_rejected(self):
        for devices in (fixture.GPU_IDS[:-1], [100] * 8, [True] + fixture.GPU_IDS[1:]):
            with self.assertRaises(ValueError):
                launch.partitions(devices, "8xTP1")
        mutations = [lambda p: p["replicas"].pop(),
                     lambda p: p["replicas"][1].update(request_names=[launch.NAMES[0]]),
                     lambda p: p["replicas"][1].update(device_unique_ids=[100]),
                     lambda p: p["workload"].update(total_output_tokens=63),
                     lambda p: p["replicas"][0]["requests"]["requests"][0].update(cancel_tick=1)]
        for mutation in mutations:
            value = self.plan()
            mutation(value)
            with self.assertRaises(ValueError):
                launch.validate_plan(value)

    def test_controlled_options_cannot_override_assignment_or_cache(self):
        for values in (["--devices", "100"], ["--benchmark-control", "/x"], ["--requests", "/x"],
                       ["--collective", "device-peer-serial-v4"], ["--batch-tokens"],
                       ["--queue-rollover", "--queue-rollover"]):
            with self.assertRaises(ValueError):
                launch.controller_options(values)

    def test_memory_headroom_and_bool_clock_bounds_reject(self):
        with self.assertRaises(ValueError):
            launch.memory_check(b"MemAvailable: 1 kB\n", launch.TARGET_BYTES * 8, 32 * 1024**3)
        with self.assertRaises(ValueError):
            launch.memory_check(b"MemAvailable: 1 kB\nMemAvailable: 2 kB\n", 1, 32 * 1024**3)
        settings = self.settings()
        settings["run_timeout_ns"] = True
        with self.assertRaises(ValueError):
            launch.validate_settings(settings)

    @staticmethod
    def settings():
        return {"ready_timeout_ns": 1_000_000_000, "run_timeout_ns": 1_000_000_000,
                "start_lead_ns": 50_000_000, "max_lateness_ns": 100_000_000}

    def run_fixture(self, mode="ok", layout="8xTP1", memory=None, failed_post=False):
        directory = tempfile.TemporaryDirectory(prefix="frc-")
        self.addCleanup(directory.cleanup)
        parent = Path(directory.name)
        controller = parent / "controller"
        controller.write_bytes(Path(__file__).with_name("replica_benchmark_test_worker.py").read_bytes())
        controller.chmod(0o700)
        output = parent / "cohort"
        output.mkdir(mode=0o700)
        calls = []

        def snapshot(command, root, phase, devices):
            self.assertEqual(devices, fixture.GPU_IDS)
            calls.append(phase)
            launch.write_new(root / f"gpu-{phase}.json", launch.encoded(fixture.snapshots()))
            if failed_post and phase == "after":
                raise ValueError("synthetic post snapshot failure")

        with mock.patch.dict(os.environ, {"FERRIC_REPLICA_TEST_MODE": mode}):
            result = launch.run(self.plan(layout), output, controller, launch.sha(controller.read_bytes()),
                                controller, launch.sha(controller.read_bytes()), parent, parent, ["test-only-snapshot"],
                                self.settings(), 32 * 1024**3, snapshot_fn=snapshot,
                                memory_raw=b"MemAvailable: 2000000000 kB\n" if memory is None else memory)
        for replica in result.get("control", {}).get("replicas", []):
            with self.assertRaises(ChildProcessError):
                os.waitpid(replica["pid"], os.WNOHANG)
        self.assertFalse((output / "control.sock").exists())
        self.assertFalse(result["model_parity_qualified"])
        self.assertTrue((output / "cohort.json").is_file())
        for path in output.glob("*/descendant.pid"):
            self.assert_terminated(int(path.read_text()))
        return result, calls

    def assert_terminated(self, pid):
        try:
            self.assertIn(launch.process_identity(pid)["state"], ("Z", "X"))
        except FileNotFoundError:
            pass

    def test_successful_eight_children_share_future_epoch_and_reap(self):
        result, calls = self.run_fixture()
        self.assertEqual(result["status"], "unvalidated-complete", result)
        self.assertEqual(calls, ["before", "after"])
        control = result["control"]
        self.assertEqual(len(control["replicas"]), 8)
        self.assertLess(control["all_ready_ns"], control["epoch_ns"])
        self.assertEqual(result["exit_codes"], [0] * 8)
        self.assertTrue(result["group_termination"]["confirmed"])
        for replica in control["replicas"]:
            self.assertEqual(replica["started"]["epoch_ns"], control["epoch_ns"])
            self.assertGreaterEqual(replica["eof_ns"], replica["closed"]["closed_ns"])

    def test_ready_run_close_failures_abort_and_reap_entire_cohort(self):
        for mode in ("early_exit", "no_ready", "wrong_nonce", "duplicate_ready", "oversized",
                     "late_start", "no_close", "wrong_close", "trailing_partial", "duplicate_after_close",
                     "float_ready", "float_start_epoch", "float_close_epoch", "descendant"):
            with self.subTest(mode=mode):
                result, calls = self.run_fixture(mode, "4xTP2")
                self.assertEqual(result["status"], "failed", result)
                self.assertEqual(calls, ["before", "after"])
                self.assertEqual(len(result["exit_codes"]), 4)
                self.assertTrue(result["group_termination"]["confirmed"])

    def test_sigterm_launcher_aborts_and_reaps_descendants(self):
        with tempfile.TemporaryDirectory(prefix="frsig-") as temporary:
            parent = Path(temporary)
            controller = parent / "controller"
            controller.write_bytes(Path(__file__).with_name("replica_benchmark_test_worker.py").read_bytes())
            controller.chmod(0o700)
            code = """
import os, sys
from pathlib import Path
import replica_benchmark as launch
import test_compare_tp_batch as fixture
root = Path(sys.argv[1]); output = root / 'cohort'; output.mkdir(mode=0o700)
raw = launch.encoded(fixture.synthetic_reference()); launch.reference.REFERENCE_SHA256 = launch.sha(raw)
plan = launch.plan(fixture.GPU_IDS, '4xTP2', raw, [])
def snapshot(command, output, phase, devices):
    launch.write_new(output / ('gpu-' + phase + '.json'), launch.encoded(fixture.snapshots()))
controller = root / 'controller'; digest = launch.sha(controller.read_bytes())
settings = {'ready_timeout_ns': 5000000000, 'run_timeout_ns': 5000000000, 'start_lead_ns': 50000000, 'max_lateness_ns': 100000000}
result = launch.run(plan, output, controller, digest, controller, digest, root, root, ['test-only'], settings,
                    32*1024**3, snapshot_fn=snapshot, memory_raw=b'MemAvailable: 2000000000 kB\\n')
raise SystemExit(0 if result['status'] == 'unvalidated-complete' else 1)
"""
            env = dict(os.environ, PYTHONPATH=str(Path(__file__).parent), FERRIC_REPLICA_TEST_MODE="descendant")
            child = subprocess.Popen([sys.executable, "-c", code, str(parent)], env=env,
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
            try:
                deadline = time.monotonic() + 10
                pid_path = parent / "cohort/replica-00/descendant.pid"
                while not pid_path.exists():
                    self.assertLess(time.monotonic(), deadline)
                    self.assertIsNone(child.poll())
                    time.sleep(0.01)
                child.send_signal(signal.SIGTERM)
                stdout, stderr = child.communicate(timeout=15)
                self.assertEqual(child.returncode, 1, (stdout, stderr))
                result = launch.reference.json_value((parent / "cohort/cohort.json").read_bytes())
                self.assertEqual(result["status"], "failed")
                self.assertIn("signal 15", result["error"])
                self.assertTrue(result["group_termination"]["confirmed"])
                self.assert_terminated(int(pid_path.read_text()))
                for state in result["control"]["replicas"]:
                    self.assert_terminated(state["pid"])
            finally:
                if child.poll() is None:
                    child.kill()
                    child.wait(timeout=5)

    def test_failed_post_snapshot_is_not_retried(self):
        result, calls = self.run_fixture(layout="1xTP8", failed_post=True)
        self.assertEqual(result["status"], "failed")
        self.assertEqual(calls, ["before", "after"])

    def test_insufficient_memory_is_retained_and_launches_nothing(self):
        result, calls = self.run_fixture(memory=b"MemAvailable: 1 kB\n")
        self.assertEqual(result["status"], "failed")
        self.assertEqual(calls, [])
        self.assertNotIn("control", result)


if __name__ == "__main__":
    unittest.main()
