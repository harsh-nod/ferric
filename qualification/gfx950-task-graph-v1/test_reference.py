import copy
import json
from pathlib import Path
import struct
import tempfile
import unittest

import reference


class ReferenceTests(unittest.TestCase):
    def test_independent_topological_values_and_input_bounds(self):
        self.assertEqual(reference.reference([1] * 896), [128, 256, 256, 640, 768, 768, 1664])
        self.assertEqual(reference.reference([1024] * 896),
                         [value * 1024 for value in [128, 256, 256, 640, 768, 768, 1664]])
        for values in ([0] * 895, [1025] * 896, [-1] * 896, [True] * 896):
            with self.assertRaises(ValueError):
                reference.reference(values)
        with self.assertRaises(ValueError):
            reference.decode_inputs(bytes(3583))

    def test_all_cases_are_deterministic_and_exclusive(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in reference.CASES:
                first = reference.generate(name, root / (name + "1"))
                second = reference.generate(name, root / (name + "2"))
                self.assertEqual(first, second)
                self.assertEqual(first["expected_payloads"], reference.reference(
                    reference.decode_inputs((root / (name + "1") / "inputs.u32le").read_bytes())))
                with self.assertRaises(FileExistsError):
                    reference.generate(name, root / (name + "1"))

    def fixture(self, root, cross_workgroup=False):
        case = root / "case"
        expected = reference.generate("ramp", case)
        run = root / "run"
        run.mkdir()
        artifact = {"source_sha256": "1" * 64, "object_sha256": "2" * 64}
        artifact_path = root / "artifact.json"
        artifact_path.write_text(json.dumps(artifact))
        probe = root / "probe"
        worker = root / "worker"
        probe.write_bytes(b"host-test probe identity, not executable")
        worker.write_bytes(b"host-test worker identity, not executable")
        epochs = []
        for expected_epoch in range(1, 6):
            stale = expected_epoch == 5
            owners = [0] * 7 if stale else ([1, 2, 1, 2, 1, 2, 1] if cross_workgroup else [1] * 7)
            packed = sum(owner << (2 * task) for task, owner in enumerate(owners))
            state = ([4, 1, 0, 0, 0, 1] + [0] * 7 if stale else
                     [expected_epoch, 0, 127, 127, packed, 0] + expected["expected_payloads"])
            edges = [[parent, task] for task, parents in enumerate(reference.DEPENDENCIES)
                     for parent in parents if owners[parent] and owners[task]
                     and owners[parent] != owners[task]]
            epochs.append({
                "expected_epoch": expected_epoch, "initialized_epoch": min(expected_epoch, 4),
                "stale_epoch_negative": stale, "state": state, "task_owners": owners,
                "distinct_observed_workgroups": len(set(owners) - {0}),
                "cross_workgroup_dependency_edges": edges,
                "worker_dispatch_interval_ns": 100, "host_dispatch_roundtrip_ns": 200,
                "host_epoch_reset_dispatch_readback_ns": 300,
            })
        states = b"".join(struct.pack("<13I", *epoch["state"]) for epoch in epochs)
        report = {
            "schema": "ferric-task-graph-probe-v1", "authority": "none", "artifact": artifact,
            "probe_sha256": reference.digest(probe.read_bytes()),
            "worker_sha256": reference.digest(worker.read_bytes()),
            "input_sha256": reference.digest((case / "inputs.u32le").read_bytes()),
            "states_sha256": reference.digest(states), "completed_dispatches": 5,
            "reused_worker_queue_and_allocations": True,
            "input_immutability_and_all_guards_passed": True,
            "free_close_and_worker_exit_passed": True, "host_lifecycle_ns": 2000,
            "epochs": epochs,
        }
        (run / "states.u32le").write_bytes(states)
        (run / "report.json").write_text(json.dumps(report))
        return (case, run, artifact_path, probe, worker), report

    def test_complete_report_binds_raw_state_and_distinguishes_actual_handoffs(self):
        for cross_workgroup in (False, True):
            with tempfile.TemporaryDirectory() as temporary:
                paths, _ = self.fixture(Path(temporary), cross_workgroup)
                result = reference.check(*paths)
                self.assertEqual(result["status"], "pass")
                self.assertEqual(result["independent_payload_values_checked"], 28)
                self.assertEqual(result["state_words_validated"], 65)
                self.assertEqual(result["deterministic_state_words_compared"], 61)
                self.assertEqual(result["cross_workgroup_dependency_observed"], cross_workgroup)

    def test_report_mutations_fail_without_numerical_receipt(self):
        for mutation in range(12):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                paths, original = self.fixture(Path(temporary))
                case, run, artifact, probe, worker = paths
                report = copy.deepcopy(original)
                if mutation == 0:
                    report["epochs"][0]["state"][6] += 1
                elif mutation == 1:
                    report["epochs"][4]["state"][6] = 1
                elif mutation == 2:
                    report["epochs"][0]["state"][4] |= 1 << 14
                elif mutation == 3:
                    report["free_close_and_worker_exit_passed"] = False
                elif mutation == 4:
                    report["input_sha256"] = "0" * 64
                elif mutation == 5:
                    report["artifact"]["source_sha256"] = "0" * 64
                elif mutation == 6:
                    worker.write_bytes(b"substituted worker")
                elif mutation == 7:
                    (run / "states.u32le").write_bytes(bytes(260))
                elif mutation == 8:
                    report["epochs"][0]["host_dispatch_roundtrip_ns"] = 99
                elif mutation == 9:
                    report["epochs"][0]["cross_workgroup_dependency_edges"] = [[0, 1]]
                elif mutation == 10:
                    report["authority"] = "production"
                else:
                    report["host_lifecycle_ns"] = 1499
                (run / "report.json").write_text(json.dumps(report))
                with self.assertRaises(ValueError):
                    reference.check(case, run, artifact, probe, worker)
                self.assertFalse((run / "numerical.json").exists())


if __name__ == "__main__":
    unittest.main()
