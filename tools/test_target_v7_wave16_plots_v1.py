"""Synthetic plot-policy tests; real charts additionally revalidate raw captures."""

import copy
import csv
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock
from xml.etree import ElementTree as ET

import target_v7_wave16_plots_v1 as plots
from test_target_v7_wave16_decode_v1 import fixture


class V7Wave16PlotTests(unittest.TestCase):
    def setUp(self):
        self.checker = plots.load_checker()
        self.reports = []
        for variant in plots.VARIANTS:
            report = self.checker.validate_records(*fixture(variant))
            report["comparator_sha256"] = plots.CHECKER_SHA256
            self.reports.append(report)

    def test_exact_pair_generates_62_observed_points_and_keeps_source_reports(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "plots"
            raw = [json.dumps(report).encode() for report in self.reports]
            plots.generate(output, self.checker, self.reports, raw)
            svg = ET.parse(output / "intervals.svg")
            self.assertEqual(len(svg.findall(".//{http://www.w3.org/2000/svg}circle")), 62)
            rows = list(csv.DictReader(io.StringIO((output / "intervals.csv").read_text())))
            self.assertEqual(len(rows), 62)
            self.assertEqual({row["output_token_ordinal"] for row in rows}, {str(i) for i in range(2, 33)})
            self.assertEqual((output / "baseline-report.json").read_bytes(), raw[0])
            contrast = json.loads((output / "observed-contrast.json").read_text())
            self.assertFalse(contrast["causal_or_stable_speedup_claim"])
            self.assertFalse(contrast["gpu_overlap_measured"])

    def test_unmatched_cohorts_rejected_before_output(self):
        for key in ("identities", "head_artifact"):
            reports = copy.deepcopy(self.reports)
            reports[1][key]["artifact_hsaco_id"] = "a" * 64
            with self.subTest(key=key), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / "plots"
                with self.assertRaises(ValueError):
                    plots.generate(output, self.checker, reports, [b"{}", b"{}"])
                self.assertFalse(output.exists())

    def test_reference_packet_precision_and_type_drift_rejected(self):
        for key, value in (("generated_tokens", [42] * 32), ("dispatches", 22176.0),
                           ("logits_precision", "BF16"), ("benchmark_qualified", True),
                           ("comparator_sha256", "a" * 64), ("variant", plots.VARIANTS[0])):
            reports = copy.deepcopy(self.reports)
            reports[1][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                plots.rows_and_contrast(self.checker, reports)
        for value in (float("inf"), True, 0, 1.0):
            reports = copy.deepcopy(self.reports)
            reports[1]["timing"]["decode_intervals_ns"][0] = value
            with self.assertRaises(ValueError):
                plots.rows_and_contrast(self.checker, reports)

    def test_checker_source_pin_is_mandatory(self):
        with mock.patch.object(plots, "CHECKER_SHA256", "a" * 64):
            with self.assertRaises(ValueError):
                plots.load_checker()


if __name__ == "__main__":
    unittest.main()
