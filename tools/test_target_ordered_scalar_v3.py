"""Synthetic policy tests only; no GPU/model or performance evidence."""

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import target_ordered_scalar_v3 as check
from test_target_batch_profile_v1 import fixture as profile_fixture


def fixture(ordered=True, profiled=False):
    records, diagnostics, plan, reference = profile_fixture()
    plan.update(schema=check.EXPECTATION_SCHEMA, variant="ordered-scalar-v3" if ordered else "serial-control",
                runtime_ordered_batches=ordered, ordered_batch_profile=check.ORDERED_PROFILE if ordered else None,
                counter_source_sha256=check.COUNTER_SOURCE_SHA256, worker_sha256=check.WORKER_SHA256,
                controller_sha256="9" * 64, **check.ARTIFACT)
    plan["performance_profile"]["runtime_profiling"] = profiled
    for key in check.core.IDENTITIES | {"performance_profile"}:
        records[0][key] = copy.deepcopy(plan[key])
    records[0]["running_worker_sha256"] = [plan["worker_sha256"]]
    if ordered:
        records[0].update(runtime_ordered_batches=True, ordered_batch_profile=check.ORDERED_PROFILE)
    if not profiled:
        del records[0]["runtime_diagnostic_status"]
        records[0]["numerical_status"] = check.counter_source.COMMON_NUMERICAL_STATUS
        diagnostics = None
    else:
        before = diagnostics[0]["ranks"][0]["counters"]
        after = diagnostics[1]["ranks"][0]["counters"]
        frontiers = check.execution_counts(plan)["completion_frontiers"]
        after["completion_polls"] = before["completion_polls"] + frontiers
        after["commands"] = before["commands"] + frontiers + 10 + 11 + 1
    return records, plan, reference, diagnostics


class OrderedScalarPolicyTests(unittest.TestCase):
    def test_both_submission_modes_with_separate_optional_counters(self):
        for ordered in (False, True):
            for profiled in (False, True):
                values = fixture(ordered, profiled)
                unchanged = copy.deepcopy(values)
                result = check.validate_records(*values)
                self.assertEqual(values, unchanged)
                self.assertEqual(result["dispatches"], 22176)
                self.assertEqual(result["generated_tokens"], check.core.TOKENS)
                self.assertEqual(result["execution_counts"]["completion_frontiers"], 2736 if ordered else 22176)
                self.assertEqual(result["configuration"]["runtime_ordered_batches"], ordered)
                self.assertEqual(result["runtime_profiling"], profiled)
                self.assertEqual("runtime_counters" in result, profiled)
                self.assertFalse(result["performance_qualified"])
                self.assertFalse(result["gpu_timestamps"])

    def test_optional_ordered_metadata_is_exact_before_adaptation(self):
        for ordered in (False, True):
            for key, value in (("runtime_ordered_batches", not ordered), ("ordered_batch_profile", "v5-wide"),
                               ("head_precision", "fp32-v7"), ("peer_artifact", {})):
                records, plan, reference, diagnostics = fixture(ordered)
                records[0][key] = value
                with self.subTest(ordered=ordered, key=key), self.assertRaises(ValueError):
                    check.validate_records(records, plan, reference, diagnostics)
        for key in ("runtime_ordered_batches", "ordered_batch_profile"):
            values = fixture()
            del values[0][0][key]
            with self.assertRaises(ValueError):
                check.validate_records(*values)

    def test_strict_plan_and_frozen_original_worker_artifact(self):
        for key, value in (("variant", "wide-ordered"), ("runtime_ordered_batches", False),
                           ("ordered_batch_profile", None), ("worker_sha256", "a" * 64),
                           ("artifact_hsaco_id", "a" * 64), ("artifact_manifest_id", "a" * 64),
                           ("artifact_handoff_id", "a" * 64), ("new_tokens", 2),
                           ("collective", "host-staged-reuse-v3"), ("kernel_profile", "v3-mfma"),
                           ("host_timing_enabled", True), ("common_comparator_sha256", "a" * 64),
                           ("counter_source_sha256", "a" * 64), ("max_batches", 100)):
            values = fixture()
            values[1][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(*values)
        for key, value in (("projection", "wave"), ("attention", "wave"), ("dispatch_sequences", True),
                           ("queue_rollover", True), ("runtime_cache_admission", False),
                           ("runtime_operational", False), ("runtime_profiling", 1)):
            values = fixture()
            values[1]["performance_profile"][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(*values)

    def test_controller_identity_stays_independently_bound_to_plan(self):
        values = fixture()
        values[0][0]["controller_sha256"] = "a" * 64
        with self.assertRaises(ValueError):
            check.validate_records(*values)

    def test_every_reference_output_remains_mandatory(self):
        for index in range(32):
            values = fixture()
            values[0][index + 6]["outputs"][0]["token"] = 42
            with self.subTest(index=index), self.assertRaises(ValueError):
                check.validate_records(*values)

    def test_common_schedule_bytes_page_timing_and_closure_guards_remain(self):
        for index, key, value in ((0, "dtype", "FP8"), (0, "prefix_cache", True),
                                  (0, "output_head_pruning", True), (2, "rank_dispatch_counts", [76]),
                                  (2, "free_pages", 4), (2, "started_ns", 0),
                                  (-2, "generated_utf8_bytes", []), (-2, "tpot_ns", 0),
                                  (-1, "all_workers_exited", False)):
            values = fixture()
            values[0][index][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(*values)

    def test_counter_presence_is_explicit_not_an_ordinary_observation(self):
        records, plan, reference, _ = fixture(profiled=True)
        with self.assertRaises(ValueError):
            check.validate_records(records, plan, reference)
        records, plan, reference, _ = fixture()
        with self.assertRaises(ValueError):
            check.validate_records(records, plan, reference, fixture(profiled=True)[3])

    def test_packet_count_never_becomes_frontier_count(self):
        values = fixture(profiled=True)
        values[3][1]["ranks"][0]["counters"]["dispatches"] = 2736
        with self.assertRaisesRegex(ValueError, "packet count"):
            check.validate_records(*values)

    def test_frontier_minima_are_profile_specific_without_mutating_old_checker(self):
        ordered = fixture(profiled=True)
        result = check.validate_records(*ordered)
        self.assertEqual(result["runtime_counters"]["workload_delta"]["completion_polls"], 2736)
        with self.assertRaises(ValueError):
            check.counter_source.snapshots(ordered[3], ordered[0][0], ordered[1])
        for is_ordered in (False, True):
            for key in ("completion_polls", "commands"):
                values = fixture(is_ordered, True)
                values[3][1]["ranks"][0]["counters"][key] -= 1
                with self.subTest(ordered=is_ordered, key=key), self.assertRaises(ValueError):
                    check.validate_records(*values)
        self.assertEqual(check.core.sha256(Path(check.core.__file__).read_bytes()), check.CORE_SHA256)
        self.assertEqual(check.core.sha256(Path(check.counter_source.__file__).read_bytes()), check.COUNTER_SOURCE_SHA256)

    def test_counter_types_roster_monotonicity_and_phase_coherence(self):
        for key in check.counter_source.COUNTERS:
            for replacement in (True, 1.0, -1, 2**64):
                values = fixture(profiled=True)
                values[3][1]["ranks"][0]["counters"][key] = replacement
                with self.subTest(key=key, value=replacement), self.assertRaises(ValueError):
                    check.validate_records(*values)
        for key, value in (("reads", 0), ("command_ns", 10001), ("kernel_admissions", 14),
                           ("kernel_admission_ns", 131), ("operational_currentness_ns", 200)):
            values = fixture(profiled=True)
            values[3][1]["ranks"][0]["counters"][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(*values)

    def test_counter_identity_phase_and_raw_stderr_contract(self):
        for key, value in (("process_id", 1), ("device_unique_id", 1), ("ordinal", 2),
                           ("rank", 1), ("scope", "GPU")):
            values = fixture(profiled=True)
            values[3][1]["ranks"][0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(*values)
        values = fixture(profiled=True)
        with self.assertRaises(ValueError):
            check.validate_records(values[0], values[1], values[2], values[3][::-1])
        raw = check.counter_source.STDERR_PREFIX + b"".join(json.dumps(row).encode() + b"\n" for row in values[3])
        self.assertEqual(check.counter_source.diagnostic_records_from_bytes(raw), values[3])
        for bad in (raw[len(check.counter_source.STDERR_PREFIX):], raw + b"extra\n", raw[:-1]):
            with self.assertRaises(ValueError):
                check.counter_source.diagnostic_records_from_bytes(bad)

    def test_private_identities_do_not_enter_public_reports(self):
        values = fixture(profiled=True)
        device = 1956390207832604050
        values[1]["device_unique_id"] = device
        values[0][0]["device_unique_ids"] = [device]
        for record in values[3]:
            record["ranks"][0]["device_unique_id"] = device
        report = check.validate_records(*values)
        text = json.dumps(report)
        for private in (str(device), "87654321", "worker_pids", "device_unique_id", "session_id", "/home/"):
            self.assertNotIn(private, text)
        values[1]["device_unique_id"] = float(device)
        with self.assertRaises(ValueError):
            check.validate_records(*values)

    def test_verified_source_loading_rejects_unpinned_code_and_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            fake = Path(directory) / "bad.py"
            fake.write_text("raise RuntimeError('must not execute')\n")
            with mock.patch.object(check, "__file__", str(Path(directory) / "checker.py")):
                with self.assertRaises(ValueError):
                    check.load_pinned("bad.py", check.CORE_SHA256)
                link = Path(directory) / "link.py"
                link.symlink_to(Path(check.core.__file__).resolve())
                with self.assertRaises(OSError):
                    check.load_pinned("link.py", check.CORE_SHA256)

    def test_compare_keeps_frozen_reference_status_workload_and_original_hashes(self):
        for profiled in (False, True):
            records, plan, reference, diagnostics = fixture(profiled=profiled)
            capture = b"".join(json.dumps(row).encode() + b"\n" for row in records)
            workload = json.dumps(check.expected_workload(plan)).encode()
            expected = json.dumps(plan).encode()
            diagnostic = None if diagnostics is None else check.counter_source.STDERR_PREFIX + b"".join(
                json.dumps(row).encode() + b"\n" for row in diagnostics)
            with self.assertRaisesRegex(ValueError, "reference digest"):
                check.compare(capture, b"0\n", workload, json.dumps(reference).encode(), expected, diagnostic)
            with mock.patch.object(check.core, "load_reference", return_value=reference):
                result = check.compare(capture, b"0\n", workload, b"synthetic", expected, diagnostic)
                self.assertEqual(result["input_sha256"]["capture"], check.core.sha256(capture))
                for raw, status, work in ((capture[:-1], b"0\n", workload), (capture, b"1\n", workload),
                                          (capture, b"0\n", b"{}")):
                    with self.assertRaises(ValueError):
                        check.compare(raw, status, work, b"synthetic", expected, diagnostic)


if __name__ == "__main__":
    unittest.main()
