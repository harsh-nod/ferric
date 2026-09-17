import copy
import json
from pathlib import Path
import struct
import tempfile
import unittest

import bind_evidence as binding
import extract_checkpoint as checkpoint
import reference


class BindingTests(unittest.TestCase):
    def fixture(self, root):
        # Synthetic host-only transport receipts. Nothing here invokes a GPU
        # or substitutes for the separately hashed real-checkpoint extraction.
        weights = root / "weights"
        weights.mkdir()
        tensor = struct.pack("<H", 0x3F80) * (1024 * 1024)
        (weights / "weights.bf16le").write_bytes(tensor)
        checkpoint_record = {
            "checkpoint_sha256": checkpoint.CHECKPOINT_SHA256, "revision": checkpoint.REVISION,
            "tensor_key": checkpoint.TENSOR, "shape": [1024, 1024], "dtype": "BF16",
            "tensor_bytes": len(tensor), "tensor_sha256": reference.digest(tensor),
            "extractor_sha256": binding.file_hash(Path(checkpoint.__file__)),
        }
        (weights / "checkpoint.json").write_text(json.dumps(checkpoint_record))
        case = root / "zero"
        reference.generate(weights, "zero", case)
        identity = binding.fixture_identity(weights, case)
        preflight = root / "preflight.json"
        cases = [dict(identity, case=name) for name in reference.CASES]
        preflight.write_text(json.dumps({
            "schema": "ferric-qwen3-kproj-preflight-v1", "policy": reference.POLICY,
            "reference_sha256": binding.file_hash(Path(reference.__file__)), "cases": cases,
            "extractor_sha256": binding.file_hash(Path(checkpoint.__file__)),
            "binder_sha256": binding.file_hash(Path(binding.__file__)),
        }))
        probe, worker = root / "probe", root / "worker"
        probe.write_bytes(b"synthetic host-test probe, not executable")
        worker.write_bytes(b"synthetic host-test worker, not executable")
        artifact = {"schema": "ferric-qwen3-kproj-artifact-v1",
                    "source_sha256": "1" * 64, "object_sha256": "2" * 64}
        artifact_path = root / "artifact.json"
        artifact_path.write_text(json.dumps(artifact))
        run = root / "run"
        run.mkdir()
        (run / "output.f32le").write_bytes(bytes(4096))
        report = {
            "schema": "ferric-qwen3-kproj-probe-v1", "authority": "none", "artifact": artifact,
            "probe_sha256": binding.file_hash(probe), "worker_sha256": binding.file_hash(worker),
            "input_sha256": identity["input_sha256"], "weights_sha256": identity["weights_sha256"],
            "output_sha256": binding.file_hash(run / "output.f32le"),
            "workgroup": [128, 1, 1], "grid_work_items": [1024, 1, 1], "completed_dispatches": 1,
            "input_immutability_and_all_allocation_guards_passed": True,
            "free_close_and_worker_exit_passed": True,
        }
        (run / "report.json").write_text(json.dumps(report))
        return (weights, case, run, artifact_path, probe, worker, preflight), report

    def test_success_binds_independent_comparison_and_report(self):
        with tempfile.TemporaryDirectory() as temporary:
            paths, _ = self.fixture(Path(temporary))
            evidence = binding.bind(*paths)
            self.assertEqual(evidence["values_checked"], 1024)
            self.assertEqual(evidence["exact_rows_checked"], 1024)
            self.assertEqual(evidence["report_sha256"], binding.file_hash(paths[2] / "report.json"))
            self.assertEqual(evidence["preflight_sha256"], binding.file_hash(paths[-1]))

    def test_report_identity_geometry_and_cleanup_mutations_fail_closed(self):
        with tempfile.TemporaryDirectory() as temporary:
            paths, valid = self.fixture(Path(temporary))
            run = paths[2]
            for mutation in range(9):
                report = copy.deepcopy(valid)
                if mutation == 0:
                    report["probe_sha256"] = "0" * 64
                elif mutation == 1:
                    report["weights_sha256"] = "0" * 64
                elif mutation == 2:
                    report["output_sha256"] = "0" * 64
                elif mutation == 3:
                    report["artifact"]["source_sha256"] = "0" * 64
                elif mutation == 4:
                    report["grid_work_items"] = [1024, 2, 1]
                elif mutation == 5:
                    report["completed_dispatches"] = 0
                elif mutation == 6:
                    report["free_close_and_worker_exit_passed"] = False
                elif mutation == 7:
                    report["input_immutability_and_all_allocation_guards_passed"] = False
                else:
                    report["authority"] = "production"
                (run / "report.json").write_text(json.dumps(report))
                with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                    binding.bind(*paths)
                self.assertFalse((run / "dispatch-numerical.json").exists())
                self.assertFalse((run / "comparison.json").exists())

    def test_preflight_substitution_is_rejected_before_comparison(self):
        with tempfile.TemporaryDirectory() as temporary:
            paths, _ = self.fixture(Path(temporary))
            data = binding.load(paths[-1])
            data["cases"][0]["bounds_sha256"] = "0" * 64
            paths[-1].write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, "before dispatch"):
                binding.bind(*paths)
            self.assertFalse((paths[2] / "comparison.json").exists())

    def test_extractor_source_identity_is_required(self):
        with tempfile.TemporaryDirectory() as temporary:
            paths, _ = self.fixture(Path(temporary))
            record_path = paths[0] / "checkpoint.json"
            record = binding.load(record_path)
            record["extractor_sha256"] = "0" * 64
            record_path.write_text(json.dumps(record))
            with self.assertRaisesRegex(ValueError, "extractor source changed"):
                binding.bind(*paths)
            self.assertFalse((paths[2] / "comparison.json").exists())


if __name__ == "__main__":
    unittest.main()
