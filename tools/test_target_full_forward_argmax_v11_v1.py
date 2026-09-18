"""Synthetic full-forward FP32 argmax policy fixtures, not GPU or reference evidence."""

import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import target_batch_decode_v1 as batch
import target_full_forward_argmax_v11_v1 as check
from test_target_batch_decode_v1 import fixture as base_fixture


def fixture(variant="serial-v7"):
    records, plan, reference = base_fixture(device_residual=True)
    plan.update(schema=check.EXPECTATION_SCHEMA, variant=variant,
                runtime_full_forward=True, full_forward_profile=check.PROFILE,
                common_comparator_sha256=check.CORE_SHA256,
                **check.SOURCE_PINS,
                fp32_argmax=variant, fp32_argmax_artifact=copy.deepcopy(check.ARGMAX_ARTIFACT),
                argmax_canonical_descriptor_sha256=check.ARGMAX_CANONICAL_DESCRIPTOR_SHA256,
                argmax_admission_sha256=check.ARGMAX_ADMISSION_SHA256,
                argmax_admission_provenance_sha256=check.ARGMAX_ADMISSION_PROVENANCE_SHA256,
                controller_sha256=check.CONTROLLER_SHA256, worker_sha256=check.WORKER_SHA256,
                kernel_profile="v3-mfma", head_precision="fp32-v7", fp32_head_workspace_bytes=9_723_904,
                fp32_head_artifact=copy.deepcopy(check.HEAD_ARTIFACT), **check.ARTIFACT)
    for key in batch.IDENTITIES:
        records[0][key] = plan[key]
    plan["performance_profile"]["projection"] = "mfma"
    plan["performance_profile"]["attention"] = "wave"
    for key in check.HEAD_SETUP:
        records[0][key] = copy.deepcopy(plan[key])
    records[0]["performance_profile"] = copy.deepcopy(plan["performance_profile"])
    records[0]["running_worker_sha256"] = [check.WORKER_SHA256]
    for key in check.EXTRA_SETUP | check.ARGMAX_SETUP:
        records[0][key] = plan[key]
    return records, plan, reference


class FullForwardArgmaxV11PolicyTests(unittest.TestCase):
    def test_both_variants_retain_packets_reference_and_timing_scope(self):
        for variant in check.VARIANTS:
            with self.subTest(variant=variant):
                report = check.validate_records(*fixture(variant))
                self.assertEqual(report["schema"], "FerricTargetFullForwardArgmaxV11ObservationV1")
                self.assertEqual(report["controller_cohort"], "full-forward-best-stack-fp32-argmax-v11-controller-v1")
                self.assertEqual(report["dispatches"], 22176)
                self.assertEqual(report["execution_counts"]["completion_frontiers"], 36)
                self.assertEqual(report["generated_tokens"], batch.TOKENS)
                self.assertEqual(report["timing"]["decode_interval_count"], 31)
                for key in ("weight_precision", "activation_precision"):
                    self.assertEqual(report[key], "BF16")
                self.assertEqual(report["logits_precision"], "FP32")
                self.assertEqual(report["head_artifact"], check.HEAD_ARTIFACT)
                for key, value in check.SOURCE_PINS.items():
                    self.assertEqual(report[key], value)
                self.assertEqual(report["fp32_argmax"], variant)
                self.assertEqual(report["fp32_argmax_artifact"], check.ARGMAX_ARTIFACT)
                self.assertEqual(report["argmax_root"], check.ARGMAX_ROOTS[variant])
                self.assertIs(report["sidecar_loaded_both_variants"], True)
                self.assertEqual(report["controller_row_capacity"], 16)
                self.assertEqual(report["argmax_source_row_capacity"], 32)
                self.assertEqual(report["observed_rows"], 1)
                self.assertIs(report["transaction_preparation_fence"], True)
                self.assertIs(report["currentness_check_count_measured"], False)
                self.assertEqual(report["configuration"]["performance_profile"]["attention"], "wave")
                self.assertIn("700 tokens/s", " ".join(report["nonclaims"]))
                for key in ("performance_qualified", "benchmark_qualified", "runtime_profiling",
                            "gpu_timestamps", "overlap_measured", "completion_polls_measured", "persistent_kernel"):
                    self.assertIs(report[key], False)

    def test_metadata_adapter_never_mutates_inputs_or_existing_checker(self):
        values = fixture("wave-v11")
        original = copy.deepcopy(values)
        old_policy = copy.deepcopy((batch.PLAN, batch.SETUP))
        check.validate_records(*values)
        self.assertEqual(values, original)
        self.assertEqual((batch.PLAN, batch.SETUP), old_policy)
        with self.assertRaises(ValueError):
            batch.validate_records(*values)

    def test_setup_extras_are_exact_and_required_for_both_argmax_modes(self):
        for variant in check.VARIANTS:
            for key, value in (("runtime_ordered_batches", False), ("runtime_diagnostic_status", "enabled"),
                               ("head_precision", "bf16-v7-control"), ("full_forward_profile", "wrong"),
                               ("runtime_full_forward", False)):
                records, plan, reference = fixture(variant)
                records[0][key] = value
                with self.subTest(variant=variant, key=key), self.assertRaises(ValueError):
                    check.validate_records(records, plan, reference)
        for variant in check.VARIANTS:
            for key in check.EXTRA_SETUP | check.ARGMAX_SETUP:
                records, plan, reference = fixture(variant)
                del records[0][key]
                with self.subTest(variant=variant, missing=key), self.assertRaises(ValueError):
                    check.validate_records(records, plan, reference)

    def test_plan_profile_and_types_are_fail_closed(self):
        for key, value in (("schema", "FerricTargetBatchDecodeExpectationV1"), ("variant", "ordered-scalar-v3"),
                           ("variant", []), ("runtime_full_forward", 0), ("full_forward_profile", None),
                           ("new_tokens", 2), ("new_tokens", 32.0), ("collective", "host-staged-reuse-v3"),
                           ("kernel_profile", "v3-wave"), ("host_timing_enabled", True), ("max_batches", 37),
                           ("common_comparator_sha256", "a" * 64), ("unknown", 0)):
            records, plan, reference = fixture()
            plan[key] = value
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)
        for key, value in (("runtime_cache_admission", False), ("runtime_operational", False),
                           ("runtime_profiling", True), ("dispatch_sequences", True),
                           ("queue_rollover", True), ("projection", "baseline"), ("attention", "baseline")):
            records, plan, reference = fixture()
            plan["performance_profile"][key] = value
            records[0]["performance_profile"][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_fixed_controller_worker_and_native_identities(self):
        for key in batch.IDENTITIES:
            records, plan, reference = fixture()
            plan[key] = records[0][key] = "a" * 64
            if key == "worker_sha256":
                records[0]["running_worker_sha256"] = ["a" * 64]
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)
        with mock.patch.object(check, "CONTROLLER_SHA256", ""):
            with self.assertRaises(ValueError):
                check.validate_records(*fixture())

    def test_mode_sidecar_and_source_cannot_be_substituted(self):
        for variant in check.VARIANTS:
            other = "wave-v11" if variant == "serial-v7" else "serial-v7"
            for change_plan, change_setup in ((True, False), (False, True), (True, True)):
                records, plan, reference = fixture(variant)
                if change_plan:
                    plan["fp32_argmax"] = other
                if change_setup:
                    records[0]["fp32_argmax"] = other
                with self.subTest(variant=variant, plan=change_plan, setup=change_setup), self.assertRaises(ValueError):
                    check.validate_records(records, plan, reference)
            for key in check.SOURCE_PINS:
                for value in ("", "a" * 64, False):
                    records, plan, reference = fixture(variant)
                    plan[key] = value
                    with self.subTest(variant=variant, source=key, value=value), self.assertRaises(ValueError):
                        check.validate_records(records, plan, reference)
            for key in check.ARGMAX_ARTIFACT:
                records, plan, reference = fixture(variant)
                plan["fp32_argmax_artifact"][key] = "a" * 64
                records[0]["fp32_argmax_artifact"][key] = "a" * 64
                with self.subTest(variant=variant, artifact=key), self.assertRaises(ValueError):
                    check.validate_records(records, plan, reference)
            for key in ("argmax_canonical_descriptor_sha256", "argmax_admission_sha256", "argmax_admission_provenance_sha256"):
                records, plan, reference = fixture(variant)
                plan[key] = "a" * 64
                with self.assertRaises(ValueError):
                    check.validate_records(records, plan, reference)
            records, plan, reference = fixture(variant)
            records[0]["running_worker_sha256"] = ["a" * 64]
            with self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_unreleased_sidecar_descriptor_or_admission_never_passes(self):
        for key in check.ARGMAX_ARTIFACT:
            with mock.patch.dict(check.ARGMAX_ARTIFACT, {key: ""}):
                with self.assertRaises(ValueError):
                    check.checked_plan(fixture()[1])
        for key in ("ARGMAX_CANONICAL_DESCRIPTOR_SHA256", "ARGMAX_ADMISSION_SHA256", "ARGMAX_ADMISSION_PROVENANCE_SHA256"):
            with mock.patch.object(check, key, ""):
                with self.assertRaises(ValueError):
                    check.checked_plan(fixture()[1])

    def test_real_worker_identities_and_numeric_receipts_reach_core_unchanged(self):
        for variant in check.VARIANTS:
            records, plan, reference = fixture(variant)
            with mock.patch.object(check.core, "validate_records", wraps=check.core.validate_records) as validate:
                report = check.validate_records(records, plan, reference)
            observed_records, observed_plan, observed_reference = validate.call_args.args
            self.assertEqual(observed_records[1:], records[1:])
            self.assertEqual(observed_reference, reference)
            self.assertEqual(observed_records[0]["running_worker_sha256"], [check.WORKER_SHA256])
            for key in batch.IDENTITIES:
                self.assertEqual(observed_records[0][key], records[0][key])
                self.assertEqual(observed_plan[key], plan[key])
                self.assertEqual(report["identities"][key], plan[key])

    def test_baseline_cohort_variants_profile_and_controller_are_rejected(self):
        for variant in check.VARIANTS:
            for key, value in (
                ("schema", "FerricTargetFullForwardMfmaV7ExpectationV2"),
                ("variant", "serial-mfma-v7-control"),
                ("variant", "full-forward-mfma-v7"),
                ("variant", "full-forward-mfma-v7-wave"),
                ("variant", "transaction-checks"),
                ("controller_sha256", "3b69635b90d26659a21aab50600640910c3ba637e0713e69c68809ddd7089838"),
            ):
                records, plan, reference = fixture(variant)
                plan[key] = value
                if key == "controller_sha256":
                    records[0][key] = value
                with self.subTest(variant=variant, key=key, value=value), self.assertRaises(ValueError):
                    check.validate_records(records, plan, reference)
        records, plan, reference = fixture("wave-v11")
        plan["full_forward_profile"] = records[0]["full_forward_profile"] = "mfma-v3-fp32-v7-wave-attention-tp1-616"
        with self.assertRaises(ValueError):
            check.validate_records(records, plan, reference)

    def test_plan_and_record_must_both_select_wave_attention(self):
        for variant in check.VARIANTS:
            for change_plan, change_setup in ((True, False), (False, True), (True, True)):
                records, plan, reference = fixture(variant)
                if change_plan:
                    plan["performance_profile"]["attention"] = "baseline"
                if change_setup:
                    records[0]["performance_profile"]["attention"] = "baseline"
                with self.subTest(variant=variant, plan=change_plan, setup=change_setup), self.assertRaises(ValueError):
                    check.validate_records(records, plan, reference)

    def test_head_image_precision_workspace_and_setup_are_exact(self):
        for key, value in (("head_precision", "bf16-v7-control"),
                           ("fp32_head_workspace_bytes", 19_447_808),
                           ("fp32_head_workspace_bytes", 9_723_904.0),
                           ("fp32_head_artifact", {})):
            records, plan, reference = fixture()
            plan[key] = records[0][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)
        for key in check.HEAD_ARTIFACT:
            records, plan, reference = fixture()
            plan["fp32_head_artifact"][key] = "a" * 64
            records[0]["fp32_head_artifact"][key] = "a" * 64
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)
        for key in check.HEAD_SETUP:
            records, plan, reference = fixture()
            del records[0][key]
            with self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_every_generated_token_remains_independently_checked(self):
        for index in range(32):
            records, plan, reference = fixture("wave-v11")
            records[index + 6]["outputs"][0]["token"] = 42
            with self.subTest(index=index), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_schedule_pages_bytes_timestamps_and_cleanup_remain_strict(self):
        for index, key, value in ((0, "dtype", "FP8"), (0, "prefix_cache", True),
                                  (0, "output_head_pruning", True), (2, "rank_dispatch_counts", [1]),
                                  (2, "rank_dispatch_counts", [616.0]), (2, "free_pages", 4),
                                  (2, "started_ns", 0), (-2, "generated_utf8_bytes", [0]),
                                  (-2, "tpot_ns", 0), (-1, "all_workers_exited", False),
                                  (-1, "whole_seconds", 0.1)):
            records, plan, reference = fixture("wave-v11")
            records[index][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_exact_u64_identity_and_public_redaction(self):
        records, plan, reference = fixture()
        value = 2**63 + 123
        plan["device_unique_id"] = value
        records[0]["device_unique_ids"] = [value]
        plan = batch.json_value(json.dumps(plan).encode())
        report = check.validate_records(records, plan, reference)
        text = json.dumps(report)
        for private in (str(value), "device_unique_id", "worker_pids", "session_id", "/home/", "87654321"):
            self.assertNotIn(private, text)
        for bad in (float(value), True):
            plan["device_unique_id"] = bad
            with self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_loader_executes_verified_bytes_and_rejects_mutation_symlink(self):
        self.assertIsNot(check.core, batch)
        with mock.patch.object(check, "CORE_SHA256", "a" * 64):
            with self.assertRaisesRegex(ValueError, "shared source"):
                check.validate_records(*fixture())
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "target_batch_decode_v1.py"
            with mock.patch.object(check, "__file__", str(Path(directory) / "checker.py")):
                source = Path(batch.__file__).read_bytes()
                path.write_bytes(source)
                self.assertEqual(check.load_pinned().REFERENCE_SHA256, batch.REFERENCE_SHA256)
                path.write_bytes(source + b"\n")
                with self.assertRaises(ValueError):
                    check.load_pinned()
                path.unlink()
                path.symlink_to(Path(batch.__file__).resolve())
                with self.assertRaises(OSError):
                    check.load_pinned()

    def test_compare_pins_status_stderr_workload_reference_and_capture(self):
        records, plan, reference = fixture()
        capture = b"".join(json.dumps(row).encode() + b"\n" for row in records)
        workload = json.dumps(check.expected_workload(plan)).encode()
        expected = json.dumps(plan).encode()
        with self.assertRaisesRegex(ValueError, "reference digest"):
            check.compare(capture, b"0\n", workload, b"synthetic", expected, check.STDERR)
        with mock.patch.object(check.core, "load_reference", return_value=reference):
            report = check.compare(capture, b"0\n", workload, b"synthetic", expected, check.STDERR)
            self.assertEqual(report["input_sha256"]["stderr"], batch.sha256(check.STDERR))
            for cap, status, work, stderr in ((capture[:-1], b"0\n", workload, check.STDERR),
                                              (capture, b"1\n", workload, check.STDERR),
                                              (capture, b"0\n", b"{}", check.STDERR),
                                              (capture, b"0\n", workload, b""),
                                              (capture, b"0\n", workload, check.STDERR + b"{}\n")):
                with self.assertRaises(ValueError):
                    check.compare(cap, status, work, b"synthetic", expected, stderr)

    def test_cli_failure_and_nonfinite_report_never_create_output(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source"
            source.write_bytes(b"invalid")
            output = Path(directory) / "report"
            arguments = [arg for key in ("capture", "status", "workload", "reference", "expect", "stderr")
                         for arg in ("--" + key, str(source))] + ["--output", str(output)]
            with self.assertRaises(SystemExit):
                check.main(arguments)
            self.assertFalse(output.exists())
            with mock.patch.object(check, "compare", return_value={"bad": float("inf")}):
                with self.assertRaises(SystemExit):
                    check.main(arguments)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
