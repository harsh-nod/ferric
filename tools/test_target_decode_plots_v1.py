"""Plot-contract tests using synthetic fixtures, never GPU performance evidence."""

import copy
import json
from pathlib import Path
import tempfile
import unittest
from xml.etree import ElementTree as ET

import target_batch_decode_v1 as batch_gate
import target_decode_plots_v1 as plot
import target_decode_profile_v1 as legacy_gate
from test_target_batch_decode_v1 import fixture as batch_fixture
from test_target_decode_profile_v1 import fixture as legacy_fixture


def legacy(variant="baseline"):
    records, reference, plan = legacy_fixture()
    for key, value in zip(("runtime_cache_admission", "runtime_operational"), plot.LEGACY[variant]):
        plan[key] = value
        records[1][key] = value
    report = legacy_gate.validate(records, reference, plan)
    report["input_sha256"] = {"capture": "a" * 64, "validator": "b" * 64}
    return report


def batch(variant="wave-device"):
    projection, collective, sequences = plot.BATCH[variant]
    records, plan, reference = batch_fixture(device_residual=collective == "device-tp1-v3")
    plan["performance_profile"]["projection"] = projection
    plan["performance_profile"]["dispatch_sequences"] = sequences
    records[0]["performance_profile"] = copy.deepcopy(plan["performance_profile"])
    report = batch_gate.validate_records(records, plan, reference)
    report["input_sha256"] = {"capture": "c" * 64}
    report["comparator_sha256"] = "d" * 64
    return report


class TargetPlotTests(unittest.TestCase):
    def test_both_report_contracts_preserve_31_actual_intervals(self):
        for family, variants, producer in [("legacy", plot.LEGACY, legacy), ("batch", plot.BATCH, batch)]:
            for variant in variants:
                row = plot.observation(producer(variant), family, variant, "e" * 64)
                self.assertEqual(len(row["decode_intervals_seconds"]), 31)
                self.assertEqual(row["generated_tokens"], 32)
                self.assertFalse(row["performance_qualified"])
                self.assertTrue(row["reference_tokens_and_bytes_match"])

    def test_unmatched_or_duplicate_families_are_not_combined(self):
        first = plot.observation(legacy(), "legacy", "baseline", "e" * 64)
        second = plot.observation(legacy("cache"), "legacy", "cache", "e" * 64)
        plot.matched([first, second])
        with self.assertRaises(ValueError):
            plot.matched([first, first])
        for key in plot.IDENTITIES:
            changed = copy.deepcopy(second)
            changed["identities"][key] = "f" * 64
            with self.assertRaises(ValueError):
                plot.matched([first, changed])
        changed = plot.observation(batch(), "batch", "wave-device", "e" * 64)
        with self.assertRaises(ValueError):
            plot.matched([first, changed])

    def test_rate_arithmetic_and_numerical_status_are_checked(self):
        report = legacy()
        for field, value in [("all_reference_checks_passed", False), ("precision", "FP8"),
                             ("measured_runs", 2), ("warmup_runs", 10), ("runtime_profile", True)]:
            changed = copy.deepcopy(report)
            changed[field] = value
            with self.assertRaises(ValueError):
                plot.observation(changed, "legacy", "baseline", "e" * 64)
        report["summary"]["post_first_tokens_per_second"] = 700
        with self.assertRaises(ValueError):
            plot.observation(report, "legacy", "baseline", "e" * 64)

    def test_variant_label_cannot_override_the_actual_runtime_flags(self):
        with self.assertRaises(ValueError):
            plot.observation(legacy(), "legacy", "operational", "e" * 64)
        with self.assertRaises(ValueError):
            plot.observation(batch(), "batch", "wave-device-sequences", "e" * 64)

    def test_no_comparison_bar_for_only_one_observation(self):
        row = plot.observation(legacy(), "legacy", "baseline", "e" * 64)
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "single"
            plot.generate(destination, {"legacy": [row], "batch": []})
            self.assertTrue((destination / "legacy-intervals.svg").is_file())
            self.assertFalse((destination / "legacy-rates.svg").exists())
            svg = ET.parse(destination / "legacy-intervals.svg")
            self.assertEqual(len(svg.findall(".//{http://www.w3.org/2000/svg}circle")), 31)
            self.assertEqual(json.loads((destination / "observed-contrasts.json").read_text())["contrasts"], [])

    def test_matched_rates_csv_tables_and_distinct_plot_families(self):
        rows = [plot.observation(legacy(variant), "legacy", variant, "e" * 64) for variant in ("baseline", "cache")]
        batch_row = plot.observation(batch(), "batch", "wave-device", "e" * 64)
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "matched"
            plot.generate(destination, {"legacy": rows, "batch": [batch_row]})
            self.assertTrue((destination / "legacy-rates.svg").is_file())
            self.assertFalse((destination / "batch-rates.svg").exists())
            self.assertEqual(len((destination / "legacy-intervals.csv").read_text().splitlines()), 63)
            self.assertEqual(len((destination / "batch-intervals.csv").read_text().splitlines()), 32)
            for family in ("legacy", "batch"):
                raw = (destination / f"{family}-intervals.csv").read_bytes()
                self.assertNotIn(b"\r", raw)
                self.assertTrue(raw.endswith(b"\n"))
            contrast = json.loads((destination / "observed-contrasts.json").read_text())["contrasts"]
            self.assertEqual(len(contrast), 1)
            self.assertFalse(contrast[0]["causal_or_statistical_speedup_claim"])
            with self.assertRaises(FileExistsError):
                plot.generate(destination, {"legacy": rows, "batch": []})

    def test_public_normalization_does_not_copy_private_or_unknown_fields(self):
        report = legacy()
        report["private_path"] = "/private/example"
        report["worker_pids"] = [123456789]
        row = plot.observation(report, "legacy", "baseline", "e" * 64)
        encoded = json.dumps(row)
        for private in ("private_path", "/private/", "worker_pids", "123456789"):
            self.assertNotIn(private, encoded)

    def test_derived_overflow_rejects_before_any_output_directory(self):
        first = plot.observation(legacy(), "legacy", "baseline", "e" * 64)
        second = plot.observation(legacy("cache"), "legacy", "cache", "e" * 64)
        for intervals in ([1e290, 1e-308], [1e200, 1e-200], [1e307, 1e307]):
            rows = copy.deepcopy([first, second])
            for row, value in zip(rows, intervals):
                row["decode_intervals_seconds"] = [value] * 31
                row["tpot_seconds"] = value
                row["post_first_tokens_per_second"] = 1 / value
            with tempfile.TemporaryDirectory() as directory:
                destination = Path(directory) / "must-not-exist"
                with self.assertRaises(ValueError):
                    plot.generate(destination, {"legacy": rows, "batch": []})
                self.assertFalse(destination.exists())


if __name__ == "__main__":
    unittest.main()
