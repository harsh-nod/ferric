import csv
import hashlib
import json
from pathlib import Path
import re
import tempfile
import unittest
from xml.etree import ElementTree as ET

import target_bf16_argmax_report_v1 as report

ASSETS = Path(__file__).resolve().parents[1] / "docs/assets/asrock-bf16-argmax-v1"


class ArgmaxReportTests(unittest.TestCase):
    def load(self, variant="control"):
        return json.loads((ASSETS / f"{variant}-report-public.json").read_bytes())

    def test_fixed_pair_recomputes_31_interval_rates(self):
        for variant, expected in [("control", 0.8229240459383007), ("cooperative", 0.8411994654918412)]:
            raw, row = report.read_report(ASSETS / f"{variant}-report-public.json", variant)
            self.assertEqual(hashlib.sha256(raw).hexdigest(), report.REPORT_SHA[variant])
            self.assertEqual(len(row["intervals_ns"]), 31)
            self.assertAlmostEqual(row["post_first_tokens_per_second"], expected, places=14)

    def test_public_field_roster_and_claims_are_closed(self):
        for key, value in [
            ("private_path", "/private"), ("passed", False), ("benchmark_qualified", True),
            ("reference_tokens_and_bytes_match", False), ("warmup_requests", 1),
            ("measured_requests", 2), ("tensor_parallel", True), ("dispatches", 616),
            ("precision", "FP8"), ("model", "different"), ("revision", "different"),
        ]:
            with self.subTest(key=key):
                data = self.load()
                data[key] = value
                with self.assertRaises(ValueError):
                    report.validate_report(data, "control")

    def test_model_tokens_bytes_and_all_image_identities_are_required(self):
        for key in report.IMAGES["control"]:
            data = self.load()
            data["identities"][key] = report.IMAGES["cooperative"][key]
            with self.assertRaises(ValueError):
                report.validate_report(data, "control")
        for key in ["controller_sha256", "worker_sha256"]:
            data = self.load()
            data["identities"][key] = "a" * 64
            with self.assertRaises(ValueError):
                report.validate_report(data, "control")
        for key in ["generated_tokens", "generated_utf8_bytes"]:
            data = self.load()
            data[key][-1] += 1
            with self.assertRaises(ValueError):
                report.validate_report(data, "control")
        with self.assertRaises(ValueError):
            report.validate_report(self.load("cooperative"), "control")

    def test_profile_and_typed_geometry_cannot_change(self):
        for key in ["host_timing_enabled", "runtime_ordered_batches", "prefix_cache", "output_head_pruning"]:
            data = self.load()
            data["configuration"][key] = True
            with self.assertRaises(ValueError):
                report.validate_report(data, "control")
        for key in ["dispatch_sequences", "queue_rollover", "runtime_profiling"]:
            data = self.load()
            data["configuration"]["performance_profile"][key] = True
            with self.assertRaises(ValueError):
                report.validate_report(data, "control")
        data = self.load()
        data["configuration"]["batch_tokens"] = True
        with self.assertRaises(ValueError):
            report.validate_report(data, "control")

    def test_invalid_integer_intervals_and_derived_rates_fail(self):
        for bad in [True, 0, -1, 1.5, 2**64, float("nan"), float("inf")]:
            with self.subTest(interval=bad):
                data = self.load()
                data["timing"]["decode_intervals_ns"][0] = bad
                with self.assertRaises(ValueError):
                    report.validate_report(data, "control")
        for key in ["post_first_tokens_per_second", "mean_tpot_ns", "median_tpot_ns", "generation_ns"]:
            data = self.load()
            data["timing"][key] *= 2
            with self.assertRaises(ValueError):
                report.validate_report(data, "control")
        data = self.load()
        data["timing"]["decode_intervals_ns"].pop()
        with self.assertRaises(ValueError):
            report.validate_report(data, "control")

    def test_changed_or_swapped_raw_reports_publish_nothing(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            bad = path / "bad.json"
            bad.write_bytes((ASSETS / "control-report-public.json").read_bytes() + b" ")
            for control in [bad, ASSETS / "cooperative-report-public.json"]:
                with self.assertRaises(ValueError):
                    report.generate(control, ASSETS / "cooperative-report-public.json", path / "output")
                self.assertFalse((path / "output").exists())

    def test_generated_assets_are_reproducible_and_keep_62_actual_points(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "output"
            report.generate(ASSETS / "control-report-public.json", ASSETS / "cooperative-report-public.json", output)
            for expected in ASSETS.iterdir():
                self.assertEqual((output / expected.name).read_bytes(), expected.read_bytes(), expected.name)
            with (output / "intervals.csv").open(newline="") as stream:
                rows = list(csv.DictReader(stream))
            self.assertEqual(len(rows), 62)
            for variant in report.VARIANTS:
                selected = [row for row in rows if row["variant"] == variant]
                self.assertEqual([int(row["output_token_ordinal"]) for row in selected], list(range(2, 33)))
                self.assertEqual([int(row["host_interval_ns"]) for row in selected],
                                 self.load(variant)["timing"]["decode_intervals_ns"])
            svg = ET.parse(output / "intervals.svg")
            self.assertEqual(len(svg.findall(".//{http://www.w3.org/2000/svg}circle")), 62)
            summary = json.loads((output / "summary.json").read_text())
            for key in ["benchmark_qualified", "causal_or_statistical_speedup_claim", "gpu_timestamps",
                        "gpu_overlap_measured", "standalone_argmax_timing"]:
                self.assertIs(summary[key], False)
            with self.assertRaises(FileExistsError):
                report.generate(ASSETS / "control-report-public.json", ASSETS / "cooperative-report-public.json", output)

    def test_existing_four_families_and_310_points_are_unchanged(self):
        self.assertEqual(set(report.charts.FAMILIES), {"legacy", "batch", "head", "ordered"})
        old = ASSETS.parent / "asrock-target8b-ablations-v2"
        points = 0
        for family in report.charts.FAMILIES:
            with (old / f"{family}-intervals.csv").open(newline="") as stream:
                points += len(list(csv.DictReader(stream)))
        self.assertEqual(points, 310)

    def test_lesson_table_and_relative_links_match_published_assets(self):
        lesson = ASSETS.parents[1] / "GFX950_BF16_ARGMAX_V1.md"
        text = lesson.read_text()
        self.assertIn((ASSETS / "table.md").read_text().strip(), text)
        for target in re.findall(r"\]\(([^)]+)\)", text):
            self.assertTrue((lesson.parent / target.split("#", 1)[0]).exists(), target)


if __name__ == "__main__":
    unittest.main()
