"""Portable synthetic norm-policy tests, not native or model-performance evidence."""
import copy
import json
import unittest
from unittest import mock

import target_batch_decode_v1 as batch
import target_full_forward_rmsnorm_v15_v1 as check
from test_target_full_forward_argmax_v11_v1 import fixture as argmax_fixture


def fixture(variant="baseline"):
    records, plan, reference = argmax_fixture("wave-v11")
    plan.update(schema=check.EXPECTATION_SCHEMA, variant=variant, full_forward_profile=check.PROFILE,
                controller_sha256=check.CONTROLLER_SHA256, worker_sha256=check.WORKER_SHA256,
                rmsnorm=variant, rmsnorm_artifact=copy.deepcopy(check.RMSNORM_ARTIFACT),
                rmsnorm_canonical_descriptor_sha256=check.RMSNORM_CANONICAL_DESCRIPTOR_SHA256,
                rmsnorm_admission_sha256=check.RMSNORM_ADMISSION_SHA256, **check.SOURCE_PINS)
    for key in batch.IDENTITIES | check.EXTRA_SETUP | check.HEAD_SETUP | check.ARGMAX_SETUP | check.RMSNORM_SETUP:
        records[0][key] = copy.deepcopy(plan[key])
    records[0]["running_worker_sha256"] = [check.WORKER_SHA256]
    return records, plan, reference


class RmsNormV15Tests(unittest.TestCase):
    def test_both_modes_same_four_images_packets_precision_and_nonclaims(self):
        for variant in check.VARIANTS:
            report = check.validate_records(*fixture(variant))
            self.assertEqual(report["rmsnorm"], variant)
            self.assertEqual(report["rmsnorm_artifact"], check.RMSNORM_ARTIFACT)
            self.assertEqual(report["head_artifact"], check.HEAD_ARTIFACT)
            self.assertEqual(report["fp32_argmax_artifact"], check.ARGMAX_ARTIFACT)
            self.assertEqual(report["fp32_argmax"], "wave-v11")
            self.assertEqual(report["changed_norm_packets_per_forward"], 73)
            self.assertEqual(report["dispatches"], 22176)
            self.assertEqual(report["execution_counts"]["packets_per_forward"], 616)
            self.assertEqual(report["timing"]["decode_interval_count"], 31)
            self.assertEqual(report["generated_tokens"], batch.TOKENS)
            self.assertEqual((report["weight_precision"], report["activation_precision"], report["logits_precision"]), ("BF16", "BF16", "FP32"))
            self.assertEqual((report["rmsnorm_source_row_capacity"], report["controller_row_capacity"], report["observed_rows"]), (32, 16, 1))
            for key in ("performance_qualified", "benchmark_qualified", "runtime_profiling", "gpu_timestamps", "overlap_measured", "persistent_kernel"):
                self.assertIs(report[key], False)
            self.assertIn("both BF16 rounding", " ".join(report["nonclaims"]))
            self.assertIn("FP32 sum association", " ".join(report["nonclaims"]))

    def test_norm_modes_must_match_plan_and_setup(self):
        for variant in check.VARIANTS:
            other = "wave-v15" if variant == "baseline" else "baseline"
            for plan_change, setup_change in ((True, False), (False, True), (True, True)):
                records, plan, reference = fixture(variant)
                if plan_change: plan["rmsnorm"] = other
                if setup_change: records[0]["rmsnorm"] = other
                with self.assertRaises(ValueError): check.validate_records(records, plan, reference)
        for value in (None, True, [], "wave-v11"):
            records, plan, reference = fixture(); plan["variant"] = value
            with self.assertRaises(ValueError): check.validate_records(records, plan, reference)

    def test_every_source_pin_and_norm_admission_are_fixed(self):
        for key in list(check.SOURCE_PINS) + ["rmsnorm_admission_sha256", "rmsnorm_canonical_descriptor_sha256", "common_comparator_sha256"]:
            records, plan, reference = fixture(); plan[key] = "a" * 64
            with self.subTest(key=key), self.assertRaises(ValueError): check.validate_records(records, plan, reference)

    def test_four_images_and_controller_worker_cannot_be_replaced(self):
        for key in batch.IDENTITIES:
            records, plan, reference = fixture(); plan[key] = records[0][key] = "a" * 64
            if key == "worker_sha256": records[0]["running_worker_sha256"] = ["a" * 64]
            with self.subTest(key=key), self.assertRaises(ValueError): check.validate_records(records, plan, reference)
        for field in ("fp32_head_artifact", "fp32_argmax_artifact", "rmsnorm_artifact"):
            for key in check.RMSNORM_ARTIFACT:
                records, plan, reference = fixture()
                plan[field][key] = records[0][field][key] = "a" * 64
                with self.subTest(field=field, key=key), self.assertRaises(ValueError): check.validate_records(records, plan, reference)

    def test_every_required_setup_extension_is_closed(self):
        for key in check.EXTRA_SETUP | check.HEAD_SETUP | check.ARGMAX_SETUP | check.RMSNORM_SETUP:
            records, plan, reference = fixture(); del records[0][key]
            with self.subTest(key=key), self.assertRaises(ValueError): check.validate_records(records, plan, reference)
        for key in ("native_full_forward_timestamps", "runtime_diagnostic_status", "unknown"):
            records, plan, reference = fixture(); records[0][key] = True
            with self.assertRaises(ValueError): check.validate_records(records, plan, reference)

    def test_unchanged_argmax_attention_and_runtime_policy(self):
        for key, value in (("fp32_argmax", "serial-v7"), ("head_precision", "bf16"), ("host_timing_enabled", True),
                           ("runtime_full_forward", False), ("full_forward_profile", "old"), ("new_tokens", 31), ("new_tokens", 32.0)):
            records, plan, reference = fixture(); plan[key] = value
            if key in records[0]: records[0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError): check.validate_records(records, plan, reference)
        for key, value in (("attention", "baseline"), ("projection", "baseline"), ("runtime_profiling", True),
                           ("runtime_cache_admission", False), ("runtime_operational", False), ("queue_rollover", True), ("dispatch_sequences", True)):
            records, plan, reference = fixture()
            plan["performance_profile"][key] = records[0]["performance_profile"][key] = value
            with self.assertRaises(ValueError): check.validate_records(records, plan, reference)

    def test_all_32_tokens_and_utf8_bytes_still_checked(self):
        for index in range(32):
            records, plan, reference = fixture("wave-v15"); records[index + 6]["outputs"][0]["token"] = 42
            with self.subTest(index=index), self.assertRaises(ValueError): check.validate_records(records, plan, reference)
        records, plan, reference = fixture(); records[-2]["generated_utf8_bytes"] = [0]
        with self.assertRaises(ValueError): check.validate_records(records, plan, reference)

    def test_schedule_timing_types_pages_and_cleanup_not_adapted(self):
        for index, key, value in ((2, "rank_dispatch_counts", [543]), (2, "rank_dispatch_counts", [616.0]),
                                  (2, "started_ns", 0), (2, "free_pages", 4), (-2, "tpot_ns", 0),
                                  (-1, "all_workers_exited", False), (0, "prefix_cache", True)):
            records, plan, reference = fixture(); records[index][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError): check.validate_records(records, plan, reference)

    def test_adaptation_is_inert_to_existing_inputs_and_checker(self):
        values = fixture(); before = copy.deepcopy(values); old = copy.deepcopy((batch.PLAN, batch.SETUP))
        check.validate_records(*values)
        self.assertEqual(values, before); self.assertEqual((batch.PLAN, batch.SETUP), old)
        with self.assertRaises(ValueError): batch.validate_records(*values)

    def test_unreleased_image_and_core_drift_rejected(self):
        for field in ("RMSNORM_CANONICAL_DESCRIPTOR_SHA256", "RMSNORM_ADMISSION_SHA256"):
            with mock.patch.object(check, field, ""):
                with self.assertRaises(ValueError): check.checked_plan(fixture()[1])
        with mock.patch.object(check, "CORE_SHA256", "0" * 64):
            with self.assertRaises(ValueError): check.validate_records(*fixture())

    def test_compare_success_and_nonzero_stderr_truncation_failures(self):
        records, plan, reference = fixture()
        capture = b"".join(json.dumps(row).encode() + b"\n" for row in records)
        workload = json.dumps(check.expected_workload(plan)).encode(); expectation = json.dumps(plan).encode()
        with mock.patch.object(check.core, "load_reference", return_value=reference):
            report = check.compare(capture, b"0\n", workload, b"synthetic", expectation, check.STDERR)
            self.assertEqual(report["variant"], "baseline")
            for raw, status, stderr in ((capture, b"1\n", check.STDERR), (capture, b"0\n", b"unexpected\n"), (capture[:-1], b"0\n", check.STDERR)):
                with self.assertRaises(ValueError): check.compare(raw, status, workload, b"synthetic", expectation, stderr)


if __name__ == "__main__": unittest.main()
