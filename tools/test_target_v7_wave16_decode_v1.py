"""Synthetic strict v7-wave16 checker tests; not GPU or numerical evidence."""

import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import target_batch_decode_v1 as batch
import target_v7_wave16_decode_v1 as check
from test_target_batch_decode_v1 import fixture as base_fixture


def fixture(variant="mfma-device-baselineattention"):
    records, plan, reference = base_fixture(device_residual=True)
    projection, precision = "mfma", "fp32-v7"
    plan.update(schema="FerricTargetV7Wave16ExpectationV1", variant=variant, kernel_profile="v3-mfma",
                head_precision=precision, fp32_head_workspace_bytes=9_723_904 if precision == "fp32-v7" else 0,
                fp32_head_artifact={key: str(index + 6) * 64
                                    for index, key in enumerate(sorted(check.HEAD_IDENTITIES))})
    plan["worker_sha256"] = check.WORKER_SHA256
    plan["controller_sha256"] = check.CONTROLLER_SHA256
    records[0]["controller_sha256"] = check.CONTROLLER_SHA256
    records[0]["worker_sha256"] = check.WORKER_SHA256
    records[0]["running_worker_sha256"] = [check.WORKER_SHA256]
    plan["performance_profile"]["projection"] = projection
    plan["performance_profile"]["attention"] = check.VARIANTS[variant]
    for key in check.HEAD_SETUP | {"performance_profile"}:
        records[0][key] = copy.deepcopy(plan[key])
    return records, plan, reference


class TargetV7Wave16PolicyTests(unittest.TestCase):
    def test_exact_two_variants_keep_full_reference_and_dispatches(self):
        for variant in check.VARIANTS:
            with self.subTest(variant=variant):
                records, plan, reference = fixture(variant)
                report = check.validate_records(records, plan, reference)
                self.assertEqual(report["schema"], "FerricTargetV7Wave16ObservationV1")
                self.assertEqual(report["dispatches"], 22176)
                self.assertEqual(report["generated_tokens"], batch.TOKENS)
                self.assertEqual(report["timing"]["decode_interval_count"], 31)
                self.assertEqual(report["logits_precision"], "BF16" if "control" in variant else "FP32")
                self.assertEqual(report["weight_precision"], "BF16")
                self.assertEqual(report["activation_precision"], "BF16")
                self.assertEqual(report["configuration"]["performance_profile"], plan["performance_profile"])
                self.assertFalse(report["benchmark_qualified"])

    def test_normalization_never_mutates_inputs_or_old_policy(self):
        records, plan, reference = fixture()
        original = copy.deepcopy((records, plan, reference))
        old_sets = (copy.deepcopy(batch.SETUP), copy.deepcopy(batch.PLAN))
        check.validate_records(records, plan, reference)
        self.assertEqual((records, plan, reference), original)
        self.assertEqual((batch.SETUP, batch.PLAN), old_sets)
        with self.assertRaises(ValueError):
            batch.validate_records(records, plan, reference)

    def test_unknown_or_unpinned_head_setup_rejected(self):
        for key, value in (("head_precision", "fp32-v8"), ("fp32_head_workspace_bytes", 0),
                           ("runtime_ordered_batches", False), ("kernel_profile", "v3-mfma"),
                           ("numerical_capture", {}), ("runtime_diagnostic_status", "enabled")):
            records, plan, reference = fixture()
            records[0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)
        for key in check.HEAD_IDENTITIES:
            records, plan, reference = fixture()
            records[0]["fp32_head_artifact"][key] = "f" * 64
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_incoherent_or_extended_expectations_rejected(self):
        for key, value in (("schema", "FerricTargetBatchDecodeExpectationV1"), ("variant", "mfma-bf16-v7-control"),
                           ("kernel_profile", "v3-wave"), ("head_precision", "bf16-v7-control"),
                           ("fp32_head_workspace_bytes", 19_447_808), ("new_tokens", 2),
                           ("collective", "host-staged-reuse-v3"), ("host_timing_enabled", True),
                           ("max_batches", 100), ("extra", "ignored")):
            records, plan, reference = fixture()
            plan[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_runtime_flags_and_projection_are_not_normalized_before_validation(self):
        for key, value in (("runtime_cache_admission", False), ("runtime_operational", False),
                           ("dispatch_sequences", True), ("queue_rollover", True),
                           ("runtime_profiling", True), ("projection", "baseline"), ("attention", "wave")):
            records, plan, reference = fixture()
            plan["performance_profile"][key] = value
            records[0]["performance_profile"][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_full_reference_stays_strict_at_every_output(self):
        for index in range(32):
            records, plan, reference = fixture()
            records[index + 6]["outputs"][0]["token"] = 9856 if index == 12 else 42
            with self.subTest(index=index), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_common_schedule_bytes_timing_and_closure_checks_still_apply(self):
        for index, key, value in ((0, "dtype", "FP8"), (0, "prefix_cache", True),
                                  (0, "output_head_pruning", True), (2, "rank_dispatch_counts", [544]),
                                  (2, "free_pages", 4), (2, "started_ns", 0),
                                  (-2, "generated_utf8_bytes", [0]), (-2, "tpot_ns", 0),
                                  (-1, "all_workers_exited", False), (-1, "whole_seconds", 0.1)):
            records, plan, reference = fixture()
            records[index][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_exact_u64_identity_is_preserved_and_float_rejected(self):
        records, plan, reference = fixture()
        value = 1956390207832604050
        plan["device_unique_id"] = value
        records[0]["device_unique_ids"] = [value]
        copied_plan = batch.json_value(json.dumps(plan).encode())
        self.assertEqual(copied_plan["device_unique_id"], value)
        check.validate_records(records, copied_plan, reference)
        for bad in (float(value), True):
            copied_plan["device_unique_id"] = bad
            with self.assertRaises(ValueError):
                check.validate_records(records, copied_plan, reference)

    def test_shared_checker_drift_fails_closed(self):
        with mock.patch.object(check, "BASE_CHECKER_SHA256", "a" * 64):
            with self.assertRaisesRegex(ValueError, "shared checker"):
                check.validate_records(*fixture())

    def test_shared_loader_uses_verified_bytes_and_rejects_symlink(self):
        self.assertIsNot(check.batch, batch)
        source = Path(batch.__file__).read_bytes()
        with tempfile.TemporaryDirectory() as directory:
            head_path = Path(directory) / "target_head_decode_v1.py"
            shared = Path(directory) / "target_batch_decode_v1.py"
            with mock.patch.object(check, "__file__", str(head_path)):
                shared.write_bytes(source)
                self.assertEqual(check.load_shared_checker().REFERENCE_SHA256, batch.REFERENCE_SHA256)
                shared.write_bytes(source + b"\n")
                with self.assertRaises(ValueError):
                    check.load_shared_checker()
                shared.unlink()
                shared.symlink_to(Path(batch.__file__).resolve())
                with self.assertRaises(OSError):
                    check.load_shared_checker()

    def test_report_has_only_public_identities_and_explicit_precision(self):
        report = check.validate_records(*fixture())
        text = json.dumps(report)
        for private in ("device_unique_id", "worker_pids", "session_id", "987654321", "87654321", "/home/"):
            self.assertNotIn(private, text)
        self.assertEqual(report["head_artifact"], fixture()[1]["fp32_head_artifact"])
        self.assertFalse(report["configuration"]["speculation"])

    def test_compare_pins_reference_status_workload_and_capture(self):
        records, plan, reference = fixture()
        capture = b"".join(json.dumps(row).encode() + b"\n" for row in records)
        workload = json.dumps(batch.expected_workload(plan)).encode()
        expected = json.dumps(plan).encode()
        with self.assertRaisesRegex(ValueError, "reference digest"):
            check.compare(capture, b"0\n", workload, json.dumps(reference).encode(), expected, check.STDERR)
        with mock.patch.object(check.batch, "load_reference", return_value=reference):
            report = check.compare(capture, b"0\n", workload, b"synthetic", expected, check.STDERR)
            self.assertEqual(report["input_sha256"]["capture"], batch.sha256(capture))
            for raw, status, work in ((capture[:-1], b"0\n", workload), (capture, b"1\n", workload),
                                      (capture, b"0\n", b"{}")):
                with self.assertRaises(ValueError):
                    check.compare(raw, status, work, b"synthetic", expected, check.STDERR)

    def test_cli_rejection_does_not_create_output(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source"
            source.write_bytes(b"invalid")
            output = Path(directory) / "report"
            arguments = [arg for key in ("capture", "status", "workload", "reference", "expect", "stderr")
                         for arg in ("--" + key, str(source))] + ["--output", str(output)]
            with self.assertRaises(SystemExit) as error:
                check.main(arguments)
            self.assertEqual(error.exception.code, 1)
            self.assertFalse(output.exists())

    def test_worker_and_unprofiled_stderr_are_exact(self):
        records, plan, reference = fixture()
        plan["controller_sha256"] = "a" * 64
        records[0]["controller_sha256"] = "a" * 64
        with self.assertRaises(ValueError):
            check.validate_records(records, plan, reference)
        records, plan, reference = fixture()
        plan["worker_sha256"] = "a" * 64
        records[0]["worker_sha256"] = "a" * 64
        records[0]["running_worker_sha256"] = ["a" * 64]
        with self.assertRaises(ValueError):
            check.validate_records(records, plan, reference)
        records, plan, reference = fixture()
        capture = b"".join(json.dumps(row).encode() + b"\n" for row in records)
        workload = json.dumps(batch.expected_workload(plan)).encode()
        expected = json.dumps(plan).encode()
        with mock.patch.object(check.batch, "load_reference", return_value=reference):
            report = check.compare(capture, b"0\n", workload, b"synthetic", expected, check.STDERR)
            self.assertEqual(report["input_sha256"]["stderr"], batch.sha256(check.STDERR))
            for stderr in (b"", check.STDERR + b"{}\n", check.STDERR.replace(b"15136194560", b"0")):
                with self.assertRaises(ValueError):
                    check.compare(capture, b"0\n", workload, b"synthetic", expected, stderr)


if __name__ == "__main__":
    unittest.main()
