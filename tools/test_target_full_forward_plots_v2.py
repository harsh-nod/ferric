"""Synthetic plot-policy fixtures; never evidence of GPU execution or speed."""

import copy
import csv
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock
from xml.etree import ElementTree as ET

import target_full_forward_plots_v2 as plots
from test_target_full_forward_scalar_v2 import fixture as scalar_fixture
from test_target_full_forward_mfma_v7_v2 import fixture as mfma_fixture
from test_target_full_forward_mfma_wave_v1 import fixture as wave_fixture


def fixture(family):
    checker = plots.load_checker(family)
    source = {"scalar": scalar_fixture, "mfma-v7": mfma_fixture, "mfma-v7-wave": wave_fixture}[family]
    reports = []
    for variant in plots.COHORTS[family]["variants"]:
        report = checker.validate_records(*source(variant))
        report["comparator_sha256"] = plots.COHORTS[family]["sha256"]
        reports.append(report)
    return checker, reports


class FullForwardPlotPolicyTests(unittest.TestCase):
    def test_each_family_generates_exact62_points_only_from_its_pair(self):
        for family in plots.COHORTS:
            checker, reports = fixture(family)
            raw = [json.dumps(report).encode() for report in reports]
            with self.subTest(family=family), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / "synthetic"
                plots.generate(output, checker, family, reports, raw)
                svg = ET.parse(output / "intervals.svg")
                self.assertEqual(len(svg.findall(".//{http://www.w3.org/2000/svg}circle")), 62)
                points = list(csv.DictReader(io.StringIO((output / "intervals.csv").read_text())))
                self.assertEqual(len(points), 62)
                self.assertEqual({row["output_token_ordinal"] for row in points}, {str(i) for i in range(2, 33)})
                self.assertEqual((output / "control-report.json").read_bytes(), raw[0])
                self.assertEqual((output / "full-forward-report.json").read_bytes(), raw[1])
                contrast = json.loads((output / "observed-contrast.json").read_text())
                self.assertEqual(contrast["source_completion_frontiers"], {"serial": 22176, "full_forward": 36})
                self.assertFalse(contrast["causal_or_stable_speedup_claim"])
                self.assertFalse(contrast["gpu_overlap_measured"])
                self.assertFalse(contrast["persistent_gpu_kernel"])
                self.assertEqual(contrast["logits_precision"], "BF16" if family == "scalar" else "FP32")

    def test_missing_duplicate_or_cross_family_pairs_are_rejected(self):
        checker, scalar = fixture("scalar")
        _, mfma = fixture("mfma-v7")
        for reports in (scalar[:1], [scalar[0], scalar[0]], [scalar[0], mfma[1]]):
            with self.assertRaises(ValueError):
                plots.rows_and_contrast(checker, "scalar", reports)

    def test_unmatched_identities_or_head_image_rejected_before_output(self):
        for family in plots.COHORTS:
            checker, original = fixture(family)
            for key in ("controller_sha256", "worker_sha256", "artifact_hsaco_id"):
                reports = copy.deepcopy(original)
                reports[1]["identities"][key] = "a" * 64
                with self.subTest(family=family, key=key), tempfile.TemporaryDirectory() as directory:
                    output = Path(directory) / "not-created"
                    with self.assertRaises(ValueError):
                        plots.generate(output, checker, family, reports, [b"{}", b"{}"])
                    self.assertFalse(output.exists())
        checker, reports = fixture("mfma-v7")
        reports[1]["head_artifact"]["artifact_hsaco_id"] = "a" * 64
        with self.assertRaises(ValueError):
            plots.rows_and_contrast(checker, "mfma-v7", reports)

    def test_scope_types_counts_and_reference_remain_strict(self):
        checker, original = fixture("scalar")
        for key, value in (("generated_tokens", [42] * 32), ("dispatches", 22176.0),
                           ("controller_cohort", "rank-wrapper-fixed-v5"), ("logits_precision", "FP32"),
                           ("runtime_profiling", True), ("persistent_kernel", True),
                           ("tensor_parallel", True), ("comparator_sha256", "a" * 64)):
            reports = copy.deepcopy(original)
            reports[1][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                plots.rows_and_contrast(checker, "scalar", reports)
        for value in (True, 0, 1.0, float("inf"), 2**64):
            reports = copy.deepcopy(original)
            reports[1]["timing"]["decode_intervals_ns"][0] = value
            with self.assertRaises(ValueError):
                plots.rows_and_contrast(checker, "scalar", reports)

    def test_projection_and_frontier_metadata_not_normalized_away(self):
        checker, reports = fixture("mfma-v7")
        reports[1]["configuration"]["performance_profile"]["attention"] = "wave"
        with self.assertRaises(ValueError):
            plots.rows_and_contrast(checker, "mfma-v7", reports)
        checker, reports = fixture("scalar")
        reports[1]["execution_counts"]["completion_frontiers"] = 22176
        with self.assertRaises(ValueError):
            plots.rows_and_contrast(checker, "scalar", reports)

    def test_wave_cohort_rejects_baseline_attention_and_other_controller(self):
        for mutate in (
            lambda report: report["configuration"]["performance_profile"].update(attention="baseline"),
            lambda report: report.update(controller_cohort="rank-wrapper-fixed-v6"),
            lambda report: report["head_artifact"].update(artifact_hsaco_id="a" * 64),
        ):
            checker, reports = fixture("mfma-v7-wave")
            mutate(reports[1])
            with self.assertRaises(ValueError):
                plots.rows_and_contrast(checker, "mfma-v7-wave", reports)
        checker, reports = fixture("mfma-v7-wave")
        _, baseline = fixture("mfma-v7")
        with self.assertRaises(ValueError):
            plots.rows_and_contrast(checker, "mfma-v7-wave", [reports[0], baseline[1]])

    def test_output_source_bytes_must_match_validated_report(self):
        checker, reports = fixture("scalar")
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "not-created"
            with self.assertRaises(ValueError):
                plots.generate(output, checker, "scalar", reports, [b"{}", b"{}"])
            self.assertFalse(output.exists())

    def test_loader_requires_exact_pinned_checker_bytes(self):
        for family in plots.COHORTS:
            with mock.patch.dict(plots.COHORTS[family], {"sha256": "a" * 64}):
                with self.assertRaises(ValueError):
                    plots.load_checker(family)

    def test_failed_empty_capture_never_becomes_an_observation(self):
        checker, _ = fixture("scalar")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            (path / "capture.ndjson").write_bytes(b"")
            with self.assertRaises(ValueError):
                plots.revalidate(checker, "scalar", path, "full-forward-scalar-v3", b"synthetic")
            self.assertEqual([item.name for item in path.iterdir()], ["capture.ndjson"])


if __name__ == "__main__":
    unittest.main()
