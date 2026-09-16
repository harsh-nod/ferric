"""Host-only report-linkage tests; synthetic reports are not GPU evidence."""

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


def load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


reference = load("reference")
summary = load("summarize")


class SummaryTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        for case in summary.CASES:
            case_dir = self.root / "cases" / case
            manifest = reference.generate(case_dir, case)
            run_dir = self.root / "runs" / case
            run_dir.mkdir(parents=True)
            expected = reference.decode((case_dir / "expected.f64le").read_bytes(), 10240, "d")
            output = reference.encode(expected, "f")
            (run_dir / "output.f32le").write_bytes(output)
            comparison = reference.check(case_dir, run_dir / "output.f32le")
            reference.save_json(run_dir / "comparison.json", comparison)
            dispatch = {
                "schema": "ferric-finite-decoder-probe-v1", "authority": "none",
                "artifact": {"source_sha256": "synthetic-source", "object_sha256": "0" * 64,
                             "metadata": {"object_sha256": [0] * 32, "wavefront_size": 64,
                                          "private_segment_bytes": 0, "group_segment_bytes": 0,
                                          "symbol": "ferric_gfx950_decoder_layer_f32_v1"}},
                "worker_sha256": "synthetic-worker", "completed_dispatches": 1,
                "input_immutability_and_output_guards_passed": True,
                "free_close_and_worker_exit_passed": True,
                "grid_work_items": [256, 1, 1], "workgroup": [128, 1, 1],
                "input_sha256": manifest["inputs_sha256"],
                "weights_sha256": manifest["weights_sha256"],
                "output_sha256": reference.sha(output),
            }
            reference.save_json(run_dir / "report.json", dispatch)

    def mutate_dispatch(self, key, value):
        path = self.root / "runs" / "mixed" / "report.json"
        report = summary.read_json(path)
        report[key] = value
        path.write_text(json.dumps(report), encoding="ascii")

    def test_synthetic_linkage_preserves_unqualified_scope(self):
        report = summary.summarize(self.root)
        self.assertTrue(report["passed"])
        self.assertEqual(report["compared_values"], 40960)
        self.assertFalse(report["full_model"])
        self.assertFalse(report["persistent_scheduler"])
        self.assertIsNone(report["performance_claim"])

    def test_output_digest_mismatch_rejected(self):
        self.mutate_dispatch("output_sha256", "0" * 64)
        with self.assertRaisesRegex(ValueError, "output digest"):
            summary.summarize(self.root)

    def test_input_digest_mismatch_rejected(self):
        self.mutate_dispatch("input_sha256", "0" * 64)
        with self.assertRaisesRegex(ValueError, "input digest"):
            summary.summarize(self.root)

    def test_failed_cleanup_rejected(self):
        self.mutate_dispatch("free_close_and_worker_exit_passed", False)
        with self.assertRaisesRegex(ValueError, "runtime checks"):
            summary.summarize(self.root)

    def test_workgroup_count_not_workitem_count_rejected(self):
        self.mutate_dispatch("grid_work_items", [2, 1, 1])
        with self.assertRaisesRegex(ValueError, "launch geometry"):
            summary.summarize(self.root)

    def test_duplicate_checkpoint_stage_rejected(self):
        path = self.root / "runs" / "mixed" / "comparison.json"
        comparison = summary.read_json(path)
        comparison["stages"][1] = comparison["stages"][0]
        path.write_text(json.dumps(comparison), encoding="ascii")
        with self.assertRaisesRegex(ValueError, "checkpoint coverage"):
            summary.summarize(self.root)

    def test_metadata_object_mismatch_rejected(self):
        path = self.root / "runs" / "mixed" / "report.json"
        dispatch = summary.read_json(path)
        dispatch["artifact"]["metadata"]["object_sha256"][0] = 1
        path.write_text(json.dumps(dispatch), encoding="ascii")
        with self.assertRaisesRegex(ValueError, "metadata object digest"):
            summary.summarize(self.root)

    def test_changed_artifact_between_cases_rejected(self):
        self.mutate_dispatch("worker_sha256", "different-synthetic-worker")
        with self.assertRaisesRegex(ValueError, "artifact changed"):
            summary.summarize(self.root)


if __name__ == "__main__":
    unittest.main()
