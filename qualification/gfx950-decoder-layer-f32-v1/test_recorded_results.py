"""Recheck saved GPU outputs on the CPU; this test does not launch a GPU."""

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("reference", HERE / "reference.py")
reference = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reference)


class RecordedResultsTests(unittest.TestCase):
    def test_saved_gpu_outputs_match_independently_regenerated_reference(self):
        results = HERE / "results"
        summary = json.loads((results / "summary.json").read_text())
        source = HERE.parents[1] / "device/gfx950-decoder-layer-f32-v1/src/lib.rs"
        self.assertEqual(reference.sha(source.read_bytes()), summary["source_sha256"])
        self.assertEqual(reference.sha((HERE / "reference.py").read_bytes()),
                         summary["reference_sha256"])
        self.assertEqual([row["case"] for row in summary["observations"]],
                         ["mixed", "zero", "saturation", "skewed-attention"])
        self.assertEqual(summary["compared_values"], 40960)
        self.assertFalse(summary["production_qualified"])
        self.assertFalse(summary["full_model"])
        with tempfile.TemporaryDirectory() as directory:
            for row in summary["observations"]:
                case = row["case"]
                with self.subTest(case=case):
                    saved = results / case
                    regenerated = Path(directory) / case
                    generated = reference.generate(regenerated, case)
                    manifest = json.loads((saved / "case.json").read_text())
                    dispatch = json.loads((saved / "report.json").read_text())
                    for field in ("inputs_sha256", "weights_sha256", "reference_sha256"):
                        self.assertEqual(generated[field], manifest[field])
                    for filename, field in (("case.json", "case_sha256"),
                                            ("report.json", "dispatch_report_sha256"),
                                            ("comparison.json", "comparison_report_sha256"),
                                            ("output.f32le", "output_sha256")):
                        self.assertEqual(reference.sha((saved / filename).read_bytes()), row[field])
                    self.assertEqual(dispatch["output_sha256"], row["output_sha256"])
                    self.assertEqual(dispatch["artifact"]["object_sha256"], summary["object_sha256"])
                    self.assertEqual(dispatch["worker_sha256"], summary["worker_sha256"])
                    self.assertTrue(dispatch["input_immutability_and_output_guards_passed"])
                    self.assertTrue(dispatch["free_close_and_worker_exit_passed"])
                    comparison = reference.check(regenerated, saved / "output.f32le")
                    self.assertTrue(comparison["passed"], comparison["first_failures"])
                    self.assertEqual(comparison["compared_values"], 10240)


if __name__ == "__main__":
    unittest.main()
