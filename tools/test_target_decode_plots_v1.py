"""Plot-contract tests using synthetic fixtures, never GPU performance evidence."""

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock
from xml.etree import ElementTree as ET

import target_batch_decode_v1 as batch_gate
import target_decode_plots_v1 as plot
import target_decode_profile_v1 as legacy_gate
import target_head_decode_v1 as head_gate
import target_ordered_scalar_v3 as ordered_gate
from test_target_batch_decode_v1 import fixture as batch_fixture
from test_target_decode_profile_v1 import fixture as legacy_fixture
from test_target_head_decode_v1 import fixture as head_fixture
from test_target_ordered_scalar_v3 import fixture as ordered_fixture


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


def head(variant="baseline-fp32-v7"):
    report = head_gate.validate_records(*head_fixture(variant))
    report["input_sha256"] = {"capture": "c" * 64}
    report["comparator_sha256"] = "d" * 64
    return report


def ordered(variant="ordered-scalar-v3", profiled=False):
    records, plan, reference, counters = ordered_fixture(plot.ORDERED[variant], profiled)
    plan["controller_sha256"] = plot.ORDERED_CONTROLLER_SHA256
    records[0]["controller_sha256"] = plan["controller_sha256"]
    report = ordered_gate.validate_records(records, plan, reference, counters)
    report["input_sha256"] = {"capture": "c" * 64}
    report["comparator_sha256"] = plot.ORDERED_CHECKER_SHA256
    return report


class TargetPlotTests(unittest.TestCase):
    def test_both_report_contracts_preserve_31_actual_intervals(self):
        for family, variants, producer in [("legacy", plot.LEGACY, legacy), ("batch", plot.BATCH, batch),
                                           ("head", plot.HEAD, head), ("ordered", plot.ORDERED, ordered)]:
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
        with self.assertRaises(ValueError):
            plot.observation(batch("baseline-device"), "batch", "wave-device", "e" * 64)

    def test_scalar_device_residual_comparison_keeps_worker_identity_pinned(self):
        rows = [plot.observation(batch(variant), "batch", variant, "e" * 64)
                for variant in ("baseline-host", "baseline-device")]
        plot.matched(rows)
        self.assertEqual([row["completed_dispatches"] for row in rows], [19584, 22176])
        self.assertEqual([row["configuration"]["projection"] for row in rows], ["baseline", "baseline"])
        changed = copy.deepcopy(rows)
        changed[1]["identities"]["worker_sha256"] = "f" * 64
        with self.assertRaises(ValueError):
            plot.matched(changed)

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

    def test_head_reports_preserve_explicit_precision_and_distinct_identity(self):
        for variant, (_, precision, logits) in plot.HEAD.items():
            raw = head(variant)
            raw["private_path"] = "/private/diagnostic"
            raw["worker_pids"] = [123456789]
            row = plot.observation(raw, "head", variant, "e" * 64)
            self.assertEqual(row["head_precision"], precision)
            self.assertEqual(row["logits_precision"], logits)
            self.assertEqual(row["weight_precision"], "BF16")
            self.assertEqual(row["activation_precision"], "BF16")
            self.assertEqual(row["head_artifact"], raw["head_artifact"])
            self.assertEqual(row["completed_dispatches"], 19584)
            self.assertIn(logits + " logits", row["label"])
            for forbidden in ("private_path", "/private/", "worker_pids", "123456789"):
                self.assertNotIn(forbidden, json.dumps(row))

    def test_head_comparison_pins_main_and_sidecar_artifacts_workers_and_reference(self):
        rows = [plot.observation(head(variant), "head", variant, "e" * 64) for variant in plot.HEAD]
        plot.matched(rows)
        for container, keys in (("identities", plot.IDENTITIES), ("head_artifact", rows[0]["head_artifact"])):
            for key in keys:
                changed = copy.deepcopy(rows)
                changed[1][container][key] = "f" * 64
                with self.subTest(container=container, key=key), self.assertRaises(ValueError):
                    plot.matched(changed)
        for key, value in (("reference_sha256", "f" * 64), ("model_revision", "different")):
            changed = copy.deepcopy(rows)
            changed[1][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                plot.matched(changed)
        other = plot.observation(batch("baseline-host"), "batch", "baseline-host", "e" * 64)
        other["identities"] = copy.deepcopy(rows[0]["identities"])
        with self.assertRaises(ValueError):
            plot.matched([rows[0], other])

    def test_head_failed_reference_mislabeling_or_unsupported_options_reject(self):
        for key, value in (("passed", False), ("reference_tokens_and_bytes_match", False),
                           ("variant", "mfma-fp32-v7"), ("head_precision", "bf16-v7-control"),
                           ("logits_precision", "BF16"), ("weight_precision", "FP8"),
                           ("activation_precision", "FP32"), ("fp32_head_workspace_bytes", 0),
                           ("shared_checker_sha256", "f" * 64), ("warmup_requests", False),
                           ("generated_tokens", [42] * 32), ("generated_utf8_bytes", [])):
            report = head()
            report[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                plot.observation(report, "head", "baseline-fp32-v7", "e" * 64)
        for key, value in (("target_only", False), ("speculation", True), ("kernel_profile", "v3-wave"),
                           ("collective", "device-tp1-v3"), ("batch_tokens", True)):
            report = head()
            report["configuration"][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                plot.observation(report, "head", "baseline-fp32-v7", "e" * 64)

    def test_head_only_family_has_no_baseline_or_unobserved_bars(self):
        rows = [plot.observation(head(variant), "head", variant, "e" * 64) for variant in plot.HEAD]
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "synthetic-head-only"
            plot.generate(destination, {"legacy": [], "batch": [], "head": rows})
            self.assertTrue((destination / "head-rates.svg").is_file())
            for family in ("legacy", "batch"):
                self.assertFalse((destination / f"{family}-rates.svg").exists())
            root = ET.parse(destination / "head-intervals.svg")
            self.assertEqual(len(root.findall(".//{http://www.w3.org/2000/svg}circle")), 93)
            text = " ".join(root.getroot().itertext())
            self.assertIn("BF16 weights/activations", text)
            self.assertIn("Scalar + BF16 logits", text)
            self.assertIn("MFMA + FP32 logits", text)
            single = Path(directory) / "synthetic-head-single"
            plot.generate(single, {"legacy": [], "batch": [], "head": rows[:1]})
            self.assertFalse((single / "head-rates.svg").exists())

    def test_head_integral_floats_are_not_integer_receipts(self):
        for key in ("generated_tokens", "generated_utf8_bytes", "dispatches"):
            report = head()
            value = report[key]
            report[key] = [float(item) for item in value] if type(value) is list else float(value)
            with self.subTest(key=key), self.assertRaises(ValueError):
                plot.observation(report, "head", "baseline-fp32-v7", "e" * 64)

        for key in ("decode_intervals_ns", "admission_ttft_ns", "generation_ns", "decode_interval_count"):
            report = head()
            value = report["timing"][key]
            report["timing"][key] = [float(item) for item in value] if type(value) is list else float(value)
            with self.subTest(key=key), self.assertRaises(ValueError):
                plot.observation(report, "head", "baseline-fp32-v7", "e" * 64)

    def test_runtime_counter_diagnostics_cannot_be_rate_plot_observations(self):
        for family, variant, producer in (("legacy", "baseline", legacy),
                                          ("batch", "baseline-device", batch),
                                          ("head", "baseline-fp32-v7", head)):
            report = producer(variant)
            report["schema"] = "FerricTargetBatchRuntimeProfileDiagnosticV1"
            with self.subTest(family=family), self.assertRaises(ValueError):
                plot.observation(report, family, variant, "e" * 64)

    def test_no_head_output_for_failed_capture_or_missing_observations(self):
        with tempfile.TemporaryDirectory() as directory:
            report = head()
            report["passed"] = False
            source = Path(directory) / "synthetic-rejected.json"
            source.write_text(json.dumps(report))
            destination = Path(directory) / "must-not-exist"
            with mock.patch("sys.argv", ["plot", "--head", "baseline-fp32-v7=" + str(source),
                                          "--output", str(destination)]):
                with self.assertRaises(ValueError):
                    plot.main()
            self.assertFalse(destination.exists())
            with self.assertRaises(ValueError):
                plot.generate(destination, {"legacy": [], "batch": [], "head": []})
            self.assertFalse(destination.exists())

    def test_ordered_cohort_has_exact_packets_and_separate_source_frontiers(self):
        for variant, enabled in plot.ORDERED.items():
            report = ordered(variant)
            report["worker_pids"] = [123456789]
            report["private_path"] = "/private/example"
            row = plot.observation(report, "ordered", variant, "e" * 64)
            self.assertEqual(row["completed_dispatches"], 22176)
            self.assertEqual(row["execution_counts"]["packets"], 22176)
            self.assertEqual(row["execution_counts"]["completion_frontiers"], 2736 if enabled else 22176)
            self.assertEqual(row["configuration"]["runtime_ordered_batches"], enabled)
            self.assertEqual(row["logits_precision"], "BF16")
            self.assertFalse(row["gpu_overlap_measured"])
            self.assertFalse(row["completion_polls_measured"])
            self.assertIn("not a measured GPU event count", row["execution_counts"]["frontier_count_scope"])
            for forbidden in ("worker_pids", "123456789", "private_path", "/private/"):
                self.assertNotIn(forbidden, json.dumps(row))

    def test_ordered_cohort_pins_controller_worker_image_and_checker(self):
        for key in plot.IDENTITIES:
            report = ordered()
            report["identities"][key] = "f" * 64
            with self.subTest(identity=key), self.assertRaises(ValueError):
                plot.observation(report, "ordered", "ordered-scalar-v3", "e" * 64)
        for key in ("reference_sha256", "comparator_sha256", "common_comparator_sha256", "counter_source_sha256"):
            report = ordered()
            report[key] = "f" * 64
            with self.subTest(pin=key), self.assertRaises(ValueError):
                plot.observation(report, "ordered", "ordered-scalar-v3", "e" * 64)
        rows = [plot.observation(ordered(variant), "ordered", variant, "e" * 64) for variant in plot.ORDERED]
        plot.matched(rows)
        old = plot.observation(batch("baseline-device"), "batch", "baseline-device", "e" * 64)
        old["identities"] = copy.deepcopy(rows[0]["identities"])
        with self.assertRaises(ValueError):
            plot.matched([old, rows[0]])

    def test_ordered_profiling_results_never_enter_uninstrumented_rate_family(self):
        for variant in plot.ORDERED:
            report = ordered(variant, profiled=True)
            with self.assertRaises(ValueError):
                plot.observation(report, "ordered", variant, "e" * 64)
            report["schema"] = "FerricTargetOrderedScalarObservationV1"
            with self.assertRaises(ValueError):
                plot.observation(report, "ordered", variant, "e" * 64)
        for key in ("runtime_counters", "derived_host_observations"):
            report = ordered()
            report[key] = {}
            with self.subTest(key=key), self.assertRaises(ValueError):
                plot.observation(report, "ordered", "ordered-scalar-v3", "e" * 64)

    def test_ordered_numerical_flags_geometry_and_finite_receipts_fail_closed(self):
        for key, value in (("passed", False), ("reference_tokens_and_bytes_match", False),
                           ("variant", "serial-control"), ("dispatches", 2736),
                           ("generated_tokens", [42] * 32), ("generated_utf8_bytes", []),
                           ("performance_qualified", True), ("gpu_timestamps", True),
                           ("completion_polls_measured", True), ("warmup_requests", False)):
            report = ordered()
            report[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                plot.observation(report, "ordered", "ordered-scalar-v3", "e" * 64)
        for key, value in (("runtime_ordered_batches", False), ("ordered_batch_profile", "v5-wide"),
                           ("head_precision", "FP32"), ("target_only", False), ("speculation", True),
                           ("physical_pages", 8), ("host_timing_enabled", True)):
            report = ordered()
            report["configuration"][key] = value
            with self.subTest(config=key), self.assertRaises(ValueError):
                plot.observation(report, "ordered", "ordered-scalar-v3", "e" * 64)
        for key in ("projection", "attention", "runtime_profiling", "dispatch_sequences", "queue_rollover"):
            report = ordered()
            report["configuration"]["performance_profile"][key] = "mfma" if key in ("projection", "attention") else True
            with self.subTest(profile=key), self.assertRaises(ValueError):
                plot.observation(report, "ordered", "ordered-scalar-v3", "e" * 64)

    def test_ordered_exact_integer_accounting_cannot_be_float_or_inferred_overlap(self):
        for key in ("generated_tokens", "generated_utf8_bytes", "dispatches", "processed_kv_tokens"):
            report = ordered()
            value = report[key]
            report[key] = [float(item) for item in value] if type(value) is list else float(value)
            with self.subTest(key=key), self.assertRaises(ValueError):
                plot.observation(report, "ordered", "ordered-scalar-v3", "e" * 64)
        for key in ("packets", "completion_frontiers", "ordered_groups", "serial_frontiers", "forwards"):
            report = ordered()
            report["execution_counts"][key] = float(report["execution_counts"][key])
            with self.subTest(count=key), self.assertRaises(ValueError):
                plot.observation(report, "ordered", "ordered-scalar-v3", "e" * 64)
        for key in ("decode_intervals_ns", "admission_ttft_ns", "generation_ns", "decode_interval_count"):
            report = ordered()
            value = report["timing"][key]
            report["timing"][key] = [float(item) for item in value] if type(value) is list else float(value)
            with self.subTest(timing=key), self.assertRaises(ValueError):
                plot.observation(report, "ordered", "ordered-scalar-v3", "e" * 64)

    def test_ordered_only_generates_actual_observations_not_other_cohort_bars(self):
        rows = [plot.observation(ordered(variant), "ordered", variant, "e" * 64) for variant in plot.ORDERED]
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "synthetic-ordered-only"
            families = {family: [] for family in plot.FAMILIES}
            families["ordered"] = rows
            plot.generate(destination, families)
            self.assertTrue((destination / "ordered-rates.svg").exists())
            self.assertEqual(len((destination / "ordered-intervals.csv").read_text().splitlines()), 63)
            self.assertNotIn(b"\r", (destination / "ordered-intervals.csv").read_bytes())
            document = ET.parse(destination / "ordered-intervals.svg")
            self.assertEqual(len(document.findall(".//{http://www.w3.org/2000/svg}circle")), 62)
            self.assertIn("no GPU-overlap claim", " ".join(document.getroot().itertext()))
            for family in ("legacy", "batch", "head"):
                self.assertFalse((destination / (family + "-rates.svg")).exists())
            contrasts = json.loads((destination / "observed-contrasts.json").read_text())["contrasts"]
            self.assertEqual([entry["family"] for entry in contrasts], ["ordered"])
            families["ordered"] = rows[:1]
            single = Path(directory) / "synthetic-serial-only"
            plot.generate(single, families)
            self.assertFalse((single / "ordered-rates.svg").exists())
            families["ordered"] = []
            missing = Path(directory) / "no-planned-bars"
            with self.assertRaises(ValueError):
                plot.generate(missing, families)
            self.assertFalse(missing.exists())


if __name__ == "__main__":
    unittest.main()
