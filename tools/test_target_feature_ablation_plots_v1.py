"""Synthetic feature-observation fixtures; never GPU or performance evidence."""

import copy
import csv
import hashlib
import io
import json
import statistics
import tempfile
import unittest
from pathlib import Path
from unittest import mock
from xml.etree import ElementTree as ET

import target_feature_ablation_plots_v1 as plots
from test_target_mfma_paired_prefetch_v1 import fixture as paired_fixture
from test_target_full_forward_preparation_v1 import fixture as preparation_fixture
from test_target_full_forward_argmax_v11_v1 import fixture as argmax_fixture
from test_target_full_forward_rmsnorm_v15_v1 import fixture as rmsnorm_fixture
from test_target_full_forward_parallel_kv_v16_v1 import fixture as kv_fixture


FIXTURES = {"paired-prefetch": paired_fixture, "preparation-worker": preparation_fixture,
            "argmax-v11": argmax_fixture, "rmsnorm-v15": rmsnorm_fixture, "parallel-kv-v16": kv_fixture}


def fixture(family):
    checker = plots.load_checker(family)
    reports = []
    for variant in plots.COHORTS[family]["variants"]:
        report = checker.validate_records(*FIXTURES[family](variant))
        report["comparator_sha256"] = plots.COHORTS[family]["sha256"]
        reports.append(report)
    return checker, reports


def synthetic_capture(checker, family, directory, variant):
    """Only the reference-byte digest is mocked by callers, never receipts."""
    records, plan, reference = FIXTURES[family](variant)
    directory.mkdir()
    inputs = {
        "capture.ndjson": b"".join(json.dumps(row).encode() + b"\n" for row in records),
        "exit-status.txt": b"0\n",
        "workload.json": json.dumps(checker.expected_workload(plan)).encode(),
        "predeclared-expectation.json": json.dumps(plan).encode(),
        "stderr.log": checker.STDERR,
    }
    if family in ("rmsnorm-v15", "parallel-kv-v16"):
        inputs["postcheck-status.json"] = json.dumps(plots.COHORTS[family]["postchecks"]).encode()
    else:
        inputs["postcheck-status.txt"] = plots.COHORTS[family]["postchecks"]
    with mock.patch.object(checker.core, "load_reference", return_value=reference):
        report = checker.compare(inputs["capture.ndjson"], inputs["exit-status.txt"], inputs["workload.json"],
                                 b"synthetic-only-reference", inputs["predeclared-expectation.json"], inputs["stderr.log"])
    report["comparator_sha256"] = plots.COHORTS[family]["sha256"]
    inputs["report-public.json"] = json.dumps(report).encode()
    for name, raw in inputs.items():
        (directory / name).write_bytes(raw)
    return reference


class FeatureAblationPlotTests(unittest.TestCase):
    def test_public_assets_present_and_byte_reproducible(self):
        root = Path(__file__).resolve().parents[1] / "docs/assets/asrock-target8b-feature-ablations-v1"
        names = {"control-report.json", "candidate-report.json", "observed-contrast.json",
                 "intervals.svg", "rates.svg", "intervals.csv", "table.md", "SHA256SUMS"}
        for family in plots.COHORTS:
            with self.subTest(family=family), tempfile.TemporaryDirectory() as directory:
                published = root / family
                self.assertEqual({path.name for path in published.iterdir()}, names)
                checker = plots.load_checker(family)
                raw = [(published / name).read_bytes() for name in ("control-report.json", "candidate-report.json")]
                for data in raw:
                    timing = json.loads(data)["timing"]
                    self.assertEqual(statistics.median(timing["decode_intervals_ns"]), timing["median_tpot_ns"])
                output = Path(directory) / "rebuilt"
                plots.generate(output, checker, family, [json.loads(data) for data in raw], raw)
                for name in names:
                    self.assertEqual((published / name).read_bytes(), (output / name).read_bytes())
                for line in (published / "SHA256SUMS").read_text().splitlines():
                    digest, name = line.split("  ")
                    self.assertEqual(digest, hashlib.sha256((published / name).read_bytes()).hexdigest())
                self.assertEqual(len(ET.parse(published / "intervals.svg").findall(".//{http://www.w3.org/2000/svg}circle")), 62)
                points = list(csv.DictReader(io.StringIO((published / "intervals.csv").read_text())))
                self.assertEqual(len(points), 62)
                for name in names:
                    data = (published / name).read_bytes()
                    for private in (b"/home/", b"/tmp/", b"mi350-2", b"harmenon", b'"pid"'):
                        self.assertNotIn(private, data)

    def test_each_family_preserves_actual_different_identities_and62_points(self):
        for family in plots.COHORTS:
            checker, reports = fixture(family)
            original = copy.deepcopy(reports)
            raw = [json.dumps(report).encode() for report in reports]
            with self.subTest(family=family), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / "synthetic-only"
                with mock.patch.object(plots.drawing, "matched", side_effect=AssertionError("must not normalize")):
                    plots.generate(output, checker, family, reports, raw)
                self.assertEqual(reports, original)
                svg = ET.parse(output / "intervals.svg")
                self.assertEqual(len(svg.findall(".//{http://www.w3.org/2000/svg}circle")), 62)
                points = list(csv.DictReader(io.StringIO((output / "intervals.csv").read_text())))
                self.assertEqual(len(points), 62)
                self.assertEqual({row["output_token_ordinal"] for row in points}, {str(i) for i in range(2, 33)})
                self.assertEqual((output / "control-report.json").read_bytes(), raw[0])
                self.assertEqual((output / "candidate-report.json").read_bytes(), raw[1])
                contrast = json.loads((output / "observed-contrast.json").read_text())
                self.assertEqual(contrast["source_completion_frontiers"], {r["variant"]: 36 for r in reports})
                self.assertEqual(contrast["identities_by_variant"], {r["variant"]: r["identities"] for r in reports})
                if family == "argmax-v11":
                    self.assertEqual(reports[0]["identities"], reports[1]["identities"])
                    self.assertEqual(contrast["differing_identity_fields"], [])
                    self.assertEqual(contrast["differing_configuration_fields"], ["fp32_argmax"])
                    self.assertEqual(contrast["fp32_argmax_artifact"], checker.ARGMAX_ARTIFACT)
                    self.assertEqual(contrast["argmax_roots_by_variant"], checker.ARGMAX_ROOTS)
                    self.assertIs(contrast["gpu_argmax_duration_measured"], False)
                elif family == "rmsnorm-v15":
                    self.assertEqual(reports[0]["identities"], reports[1]["identities"])
                    self.assertEqual(contrast["differing_identity_fields"], [])
                    self.assertEqual(contrast["differing_configuration_fields"], ["rmsnorm"])
                    self.assertEqual(contrast["rmsnorm_artifact"], checker.RMSNORM_ARTIFACT)
                    self.assertEqual(contrast["fp32_argmax_artifact"], checker.ARGMAX_ARTIFACT)
                    self.assertEqual(contrast["fp32_argmax"], "wave-v11")
                    self.assertEqual(contrast["changed_norm_packets_per_forward"], 73)
                    self.assertEqual(contrast["bf16_rounding_boundaries_retained"], 2)
                    self.assertIs(contrast["fp32_sum_association_changed"], True)
                    self.assertIs(contrast["gpu_rmsnorm_duration_measured"], False)
                elif family == "parallel-kv-v16":
                    self.assertEqual(reports[0]["identities"], reports[1]["identities"])
                    self.assertEqual(contrast["differing_identity_fields"], [])
                    self.assertEqual(contrast["differing_configuration_fields"], ["kv_append"])
                    self.assertEqual(contrast["kv_append_artifact"], checker.KV_ARTIFACT)
                    self.assertEqual(contrast["kv_roots_by_variant"], checker.KV_ROOTS)
                    self.assertEqual(contrast["kv_workgroups_per_packet_by_variant"], {"baseline": 1, "parallel-v16": 64})
                    self.assertEqual(contrast["changed_kv_packets_per_forward"], 36)
                    self.assertEqual(contrast["unchanged_wave_norm_packets_per_forward"], 73)
                    self.assertEqual(contrast["loaded_images_per_variant"], 5)
                    self.assertEqual(contrast["rmsnorm"], "wave-v15")
                    self.assertEqual(contrast["fp32_argmax"], "wave-v11")
                    self.assertIs(contrast["logical_explicit_arguments_unchanged"], True)
                    self.assertIs(contrast["buffer_extents_and_access_modes_unchanged"], True)
                    self.assertIs(contrast["physical_pointer_bytes_compared"], False)
                    self.assertIs(contrast["hidden_launch_fields_unchanged"], False)
                    self.assertIs(contrast["gpu_kv_duration_measured"], False)
                else:
                    self.assertNotEqual(reports[0]["identities"], reports[1]["identities"])
                self.assertEqual(contrast["differing_axis"], plots.COHORTS[family]["axis"])
                self.assertEqual(contrast["logits_precision"], "FP32")
                for key in ("causal_or_stable_speedup_claim", "gpu_overlap_measured", "persistent_gpu_kernel",
                            "performance_qualified", "target_700_tokens_per_second_claimed"):
                    self.assertIs(contrast[key], False)
                if family == "paired-prefetch":
                    self.assertEqual(len(contrast["changed_kernel_roots"]), 6)
                    self.assertEqual(len(contrast["secondary_codegen_roots"]), 4)
                    self.assertEqual(contrast["codegen_catalog_sha256"], checker.CODEGEN_CATALOG_SHA256)
                    self.assertIs(contrast["pure_prefetch_causal_gain_claimed"], False)
                    self.assertEqual(contrast["main_canonical_descriptors_by_variant"], checker.MAIN_CANONICAL_DESCRIPTORS)
                else:
                    self.assertIs(contrast["currentness_check_count_measured"], False)

    def test_missing_duplicate_reversed_and_cross_family_pairs_rejected(self):
        checker, paired = fixture("paired-prefetch")
        _, preparation = fixture("preparation-worker")
        for reports in (paired[:1], [paired[0], paired[0]], list(reversed(paired)), [paired[0], preparation[1]]):
            with self.assertRaises(ValueError):
                plots.rows_and_contrast(checker, "paired-prefetch", reports)

    def test_all_individual_intervals_and_rate_definition_are_retained(self):
        checker, reports = fixture("preparation-worker")
        for index, report in enumerate(reports):
            values = [(index + 1) * 1_000_000_000 + ordinal * 1_234_567 for ordinal in range(31)]
            timing = report["timing"]
            timing.update(decode_intervals_ns=values, mean_tpot_ns=sum(values) / 31,
                          post_first_tokens_per_second=31 * 1e9 / sum(values),
                          generation_ns=timing["admission_ttft_ns"] + sum(values))
        rows, contrast = plots.rows_and_contrast(checker, "preparation-worker", reports)
        for report, row in zip(reports, rows):
            values = report["timing"]["decode_intervals_ns"]
            self.assertEqual(row["decode_intervals_seconds"], [value / 1e9 for value in values])
            self.assertEqual(row["post_first_tokens_per_second"], 31 * 1e9 / sum(values))
        self.assertAlmostEqual(contrast["observed_rate_ratio"],
                               sum(reports[0]["timing"]["decode_intervals_ns"]) /
                               sum(reports[1]["timing"]["decode_intervals_ns"]))

    def test_actual_axis_and_non_axis_identities_cannot_be_forged_or_dropped(self):
        for family in plots.COHORTS:
            checker, original = fixture(family)
            for key in checker.core.IDENTITIES:
                for replacement in ("a" * 64, original[0]["identities"][key], None):
                    if replacement == original[1]["identities"][key]:
                        continue
                    reports = copy.deepcopy(original)
                    if replacement is None:
                        del reports[1]["identities"][key]
                    else:
                        reports[1]["identities"][key] = replacement
                    with self.subTest(family=family, key=key), tempfile.TemporaryDirectory() as directory:
                        output = Path(directory) / "not-created"
                        with self.assertRaises(ValueError):
                            plots.generate(output, checker, family, reports, [b"{}", b"{}"])
                        self.assertFalse(output.exists())

    def test_head_configuration_scope_and_exact_types_remain_strict(self):
        for family in plots.COHORTS:
            checker, original = fixture(family)
            for key, value in (("generated_tokens", [42] * 32), ("dispatches", 22176.0),
                               ("controller_cohort", "rank-wrapper-fixed-v6"), ("logits_precision", "BF16"),
                               ("runtime_profiling", True), ("persistent_kernel", True),
                               ("tensor_parallel", True), ("comparator_sha256", "a" * 64)):
                reports = copy.deepcopy(original)
                reports[1][key] = value
                with self.subTest(family=family, key=key), self.assertRaises(ValueError):
                    plots.rows_and_contrast(checker, family, reports)
            for mutate in (
                lambda r: r["head_artifact"].update(artifact_hsaco_id="a" * 64),
                lambda r: r["configuration"]["performance_profile"].update(attention="baseline"),
                lambda r: r["configuration"].update(runtime_full_forward=False),
                lambda r: r["configuration"].update(physical_pages=8),
                lambda r: r["execution_counts"].update(completion_frontiers=22176),
            ):
                reports = copy.deepcopy(original)
                mutate(reports[1])
                with self.assertRaises(ValueError):
                    plots.rows_and_contrast(checker, family, reports)

    def test_paired_catalog_secondary_codegen_and_default_off_scope_retained(self):
        checker, original = fixture("paired-prefetch")
        for key, value in (("changed_kernel_roots", checker.CHANGED_KERNEL_ROOTS[:2]),
                           ("secondary_codegen_roots", []), ("codegen_catalog_sha256", "a" * 64),
                           ("pure_prefetch_causal_gain_claimed", True),
                           ("main_canonical_descriptor_sha256", "a" * 64),
                           ("comparison_scope", "pure-prefetch"),
                           ("paired_source_manifest_sha256", "a" * 64)):
            reports = copy.deepcopy(original)
            reports[1][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                plots.rows_and_contrast(checker, "paired-prefetch", reports)
        for key, value in (("default_enabled", True), ("selected", False), ("observed_rows", 16),
                           ("multirow_gpu_validated", True)):
            reports = copy.deepcopy(original)
            reports[1]["paired_prefetch_feature"][key] = value
            with self.assertRaises(ValueError):
                plots.rows_and_contrast(checker, "paired-prefetch", reports)

    def test_preparation_policy_and_source_are_exact(self):
        checker, original = fixture("preparation-worker")
        for key, value in (("worker_source_manifest_sha256", "a" * 64),
                           ("currentness_check_count_measured", True), ("transaction_preparation_fence", False)):
            reports = copy.deepcopy(original)
            reports[1][key] = value
            with self.assertRaises(ValueError):
                plots.rows_and_contrast(checker, "preparation-worker", reports)

    def test_argmax_selector_sidecar_admission_sources_and_scope_are_exact(self):
        checker, original = fixture("argmax-v11")
        mutations = [(key, "a" * 64) for key in checker.SOURCE_PINS]
        mutations += [("fp32_argmax", "serial-v7"), ("argmax_root", checker.ARGMAX_ROOTS["serial-v7"]),
                      ("fp32_argmax_artifact", {}), ("argmax_admission_sha256", "a" * 64),
                      ("argmax_admission_provenance_sha256", "a" * 64),
                      ("argmax_canonical_descriptor_sha256", "a" * 64),
                      ("sidecar_loaded_both_variants", False), ("observed_rows", 16),
                      ("controller_row_capacity", 32), ("argmax_source_row_capacity", 16),
                      ("transaction_preparation_fence", False)]
        for key, value in mutations:
            reports = copy.deepcopy(original)
            reports[1][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                plots.rows_and_contrast(checker, "argmax-v11", reports)
        reports = copy.deepcopy(original)
        reports[1]["configuration"]["fp32_argmax"] = "serial-v7"
        with self.assertRaises(ValueError):
            plots.rows_and_contrast(checker, "argmax-v11", reports)

    def test_argmax_pair_is_distinct_from_previous_worker_and_image_cohorts(self):
        checker, original = fixture("argmax-v11")
        for family in ("paired-prefetch", "preparation-worker"):
            _, earlier = fixture(family)
            for reports in ([earlier[0], original[1]], [original[0], earlier[1]]):
                with self.assertRaises(ValueError):
                    plots.rows_and_contrast(checker, "argmax-v11", reports)

    def test_norm_selector_four_images_sources_count_and_capacity_are_exact(self):
        checker, original = fixture("rmsnorm-v15")
        mutations = [(key, "a" * 64) for key in checker.SOURCE_PINS]
        mutations += [("rmsnorm", "baseline"), ("rmsnorm_artifact", {}),
                      ("rmsnorm_canonical_descriptor_sha256", "a" * 64),
                      ("rmsnorm_admission_sha256", "a" * 64), ("changed_norm_packets_per_forward", 72),
                      ("changed_norm_packets_per_forward", 73.0), ("rmsnorm_source_row_capacity", 16),
                      ("fp32_argmax", "serial-v7"), ("fp32_argmax_artifact", {}),
                      ("controller_row_capacity", 32), ("observed_rows", 16),
                      ("sidecar_loaded_both_variants", False)]
        for key, value in mutations:
            reports = copy.deepcopy(original); reports[1][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                plots.rows_and_contrast(checker, "rmsnorm-v15", reports)
        for key, value in (("rmsnorm", "baseline"), ("fp32_argmax", "serial-v7")):
            reports = copy.deepcopy(original); reports[1]["configuration"][key] = value
            with self.assertRaises(ValueError): plots.rows_and_contrast(checker, "rmsnorm-v15", reports)
        for family in ("paired-prefetch", "preparation-worker", "argmax-v11"):
            _, earlier = fixture(family)
            with self.assertRaises(ValueError): plots.rows_and_contrast(checker, "rmsnorm-v15", [earlier[0], original[1]])

    def test_intervals_rates_boundaries_and_nonfinite_values_rejected(self):
        checker, original = fixture("paired-prefetch")
        for value in (True, 0, 1.0, float("inf"), 2**64):
            reports = copy.deepcopy(original)
            reports[1]["timing"]["decode_intervals_ns"][0] = value
            with self.assertRaises(ValueError):
                plots.rows_and_contrast(checker, "paired-prefetch", reports)
        for key, value in (("decode_intervals_ns", [1] * 30), ("mean_tpot_ns", 1.0),
                           ("post_first_tokens_per_second", 700.0), ("generation_ns", 1),
                           ("setup_seconds", float("nan"))):
            reports = copy.deepcopy(original)
            reports[1]["timing"][key] = value
            with self.assertRaises(ValueError):
                plots.rows_and_contrast(checker, "paired-prefetch", reports)

    def test_kv_selector_five_images_sources_geometry_and_unchanged_norm_are_exact(self):
        checker, original = fixture("parallel-kv-v16")
        mutations = [(key, "a" * 64) for key in checker.SOURCE_PINS]
        mutations += [("kv_append", "baseline"), ("kv_append_artifact", {}),
                      ("kv_root", checker.KV_ROOTS["baseline"]),
                      ("kv_canonical_descriptor_sha256", "a" * 64),
                      ("kv_admission_sha256", "a" * 64), ("kv_cli_admission_sha256", "a" * 64),
                      ("kv_fixtures_sha256", "a" * 64), ("changed_kv_packets_per_forward", 35),
                      ("changed_kv_packets_per_forward", 36.0), ("kv_workgroups_per_packet", 1),
                      ("kv_workgroups_per_packet", 64.0), ("unchanged_wave_norm_packets_per_forward", 72),
                      ("rmsnorm", "baseline"), ("rmsnorm_artifact", {}),
                      ("fp32_argmax", "serial-v7"), ("fp32_argmax_artifact", {}),
                      ("observed_rows", 16), ("sidecar_loaded_both_variants", False)]
        for key, value in mutations:
            reports = copy.deepcopy(original); reports[1][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                plots.rows_and_contrast(checker, "parallel-kv-v16", reports)
        for key, value in (("kv_append", "baseline"), ("rmsnorm", "baseline"), ("fp32_argmax", "serial-v7")):
            reports = copy.deepcopy(original); reports[1]["configuration"][key] = value
            with self.assertRaises(ValueError): plots.rows_and_contrast(checker, "parallel-kv-v16", reports)
        for family in ("paired-prefetch", "preparation-worker", "argmax-v11", "rmsnorm-v15"):
            _, earlier = fixture(family)
            with self.assertRaises(ValueError): plots.rows_and_contrast(checker, "parallel-kv-v16", [earlier[0], original[1]])
    def test_output_source_bytes_must_match_report_before_directory_creation(self):
        checker, reports = fixture("paired-prefetch")
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "not-created"
            with self.assertRaises(ValueError):
                plots.generate(output, checker, "paired-prefetch", reports, [b"{}", b"{}"])
            self.assertFalse(output.exists())

    def test_loader_requires_pinned_regular_source_and_rejects_symlink(self):
        for family in plots.COHORTS:
            with mock.patch.dict(plots.COHORTS[family], {"sha256": "a" * 64}):
                with self.assertRaises(ValueError):
                    plots.load_checker(family)
            with tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / plots.COHORTS[family]["checker"]
                path.symlink_to(Path(plots.__file__).with_name(path.name))
                with mock.patch.object(plots, "__file__", str(Path(directory) / "reporter.py")):
                    with self.assertRaises(OSError):
                        plots.load_checker(family)

    def test_raw_capture_is_revalidated_and_regenerated_report_must_equal(self):
        family = "paired-prefetch"
        checker = plots.load_checker(family)
        variant = plots.COHORTS[family]["variants"][0]
        with tempfile.TemporaryDirectory() as directory:
            capture = Path(directory) / "synthetic"
            reference = synthetic_capture(checker, family, capture, variant)
            with mock.patch.object(checker.core, "load_reference", return_value=reference):
                plots.revalidate(checker, family, capture, variant, b"synthetic-only-reference")
                records = [json.loads(line) for line in (capture / "capture.ndjson").read_bytes().splitlines()]
                original = (capture / "capture.ndjson").read_bytes()
                records[6]["outputs"][0]["token"] = 42
                (capture / "capture.ndjson").write_bytes(b"".join(json.dumps(r).encode() + b"\n" for r in records))
                with self.assertRaises(ValueError):
                    plots.revalidate(checker, family, capture, variant, b"synthetic-only-reference")
                (capture / "capture.ndjson").write_bytes(original)
                report = json.loads((capture / "report-public.json").read_bytes())
                report["timing"]["post_first_tokens_per_second"] = 700.0
                (capture / "report-public.json").write_text(json.dumps(report))
                with self.assertRaises(ValueError):
                    plots.revalidate(checker, family, capture, variant, b"synthetic-only-reference")

    def test_all_cohort_postchecks_required_before_any_pipeline_output(self):
        for family in plots.COHORTS:
            checker = plots.load_checker(family)
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                captures = [root / "synthetic-control", root / "synthetic-candidate"]
                for capture, variant in zip(captures, plots.COHORTS[family]["variants"]):
                    reference = synthetic_capture(checker, family, capture, variant)
                reference_path = root / "synthetic-reference"
                reference_path.write_bytes(b"synthetic-only-reference")
                expected = plots.COHORTS[family]["postchecks"]
                if family in ("rmsnorm-v15", "parallel-kv-v16"):
                    variants = [json.dumps(dict(expected, **{field: value})).encode() for field in expected for value in (1, False)]
                    variants += [b"{}", json.dumps(dict(expected, extra=0)).encode(),
                                 json.dumps({key: value for key, value in expected.items() if key != "controller"}).encode()]
                    status_name = "postcheck-status.json"
                else:
                    variants = [expected.replace(field, field.replace(b"=0", b"=1")) for field in expected.split()]
                    variants += [b" ".join(expected.split()[:6]) + b"\n", expected.rstrip(), expected + b"extra=0\n"]
                    other = "preparation-worker" if family == "paired-prefetch" else "paired-prefetch"
                    variants.append(plots.COHORTS[other]["postchecks"])
                    status_name = "postcheck-status.txt"
                for status in variants:
                    (captures[1] / status_name).write_bytes(status)
                    output = root / "not-created"
                    with mock.patch.object(plots, "load_checker", return_value=checker), \
                         mock.patch.object(checker.core, "load_reference", return_value=reference):
                        with self.subTest(family=family, status=status), self.assertRaises(ValueError):
                            plots.run(family, *captures, reference_path, output)
                    self.assertFalse(output.exists())

    def test_failed_or_incomplete_capture_never_creates_output(self):
        family = "preparation-worker"
        checker = plots.load_checker(family)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            captures = [root / "synthetic-control", root / "synthetic-candidate"]
            for capture, variant in zip(captures, plots.COHORTS[family]["variants"]):
                reference = synthetic_capture(checker, family, capture, variant)
            reference_path = root / "synthetic-reference"
            reference_path.write_bytes(b"synthetic-only-reference")
            for name, bad in (("exit-status.txt", b"1\n"), ("capture.ndjson", b""), ("stderr.log", b"unexpected\n")):
                path = captures[1] / name
                original = path.read_bytes()
                path.write_bytes(bad)
                with mock.patch.object(plots, "load_checker", return_value=checker), \
                     mock.patch.object(checker.core, "load_reference", return_value=reference):
                    with self.assertRaises(ValueError):
                        plots.run(family, *captures, reference_path, root / "not-created")
                self.assertFalse((root / "not-created").exists())
                path.write_bytes(original)

    def test_production_pipeline_rejects_synthetic_reference_without_mock(self):
        family = "paired-prefetch"
        checker = plots.load_checker(family)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            captures = [root / "synthetic-control", root / "synthetic-candidate"]
            for capture, variant in zip(captures, plots.COHORTS[family]["variants"]):
                synthetic_capture(checker, family, capture, variant)
            reference_path = root / "synthetic-reference"
            reference_path.write_bytes(b"synthetic-only-reference")
            with self.assertRaisesRegex(ValueError, "reference digest"):
                plots.run(family, *captures, reference_path, root / "not-created")
            self.assertFalse((root / "not-created").exists())


if __name__ == "__main__":
    unittest.main()
