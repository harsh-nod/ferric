"""Synthetic runtime-profile policy tests, not GPU measurements."""

import copy
import json
from pathlib import Path
import tempfile
import unittest

import target_batch_profile_v1 as check
from test_target_batch_decode_v1 import fixture as numerical_fixture


def fixture():
    records, plan, reference = copy.deepcopy(numerical_fixture())
    plan["schema"] = check.EXPECTATION_SCHEMA
    plan["common_comparator_sha256"] = check.CORE_SHA256
    plan["controller_sha256"] = check.CONTROLLER_SHA256
    plan["performance_profile"]["projection"] = "baseline"
    plan["performance_profile"]["runtime_profiling"] = True
    records[0]["controller_sha256"] = plan["controller_sha256"]
    records[0]["performance_profile"] = copy.deepcopy(plan["performance_profile"])
    records[0]["runtime_diagnostic_status"] = check.DIAGNOSTIC_STATUS
    records[0]["numerical_status"] = check.NUMERICAL_STATUS
    before = dict.fromkeys(check.COUNTERS, 0)
    before.update(commands=100, command_ns=10_000, kernel_admissions=13, kernel_admission_ns=130,
                  full_currentness_checks=10, full_currentness_ns=100,
                  operational_currentness_checks=20, operational_currentness_ns=200,
                  reads=3, read_bytes=6, read_ns=30, writes=4, write_bytes=8, write_ns=40)
    delta = dict.fromkeys(check.COUNTERS, 0)
    delta.update(commands=22176 + 10 + 11 + 1, command_ns=10_000,
                 operational_currentness_checks=70000, operational_currentness_ns=700,
                 dispatches=22176, dispatch_prepare_ns=1000, dispatch_publish_ns=2000,
                 dispatch_wait_ns=3000, completion_polls=30000,
                 reads=10, read_bytes=20, read_ns=100, writes=11, write_bytes=22, write_ns=110)
    after = {key: before[key] + delta[key] for key in check.COUNTERS}
    diagnostics = []
    for ordinal, (phase, counters) in enumerate(zip(("before_workload", "after_workload"), (before, after))):
        diagnostics.append({"schema": "FerricQwen3TpRuntimeDiagnosticV1", "authority": "none",
            "phase": phase, "measurement": check.MEASUREMENT, "performance_qualified": False,
            "ranks": [{"schema": "FerricRuntimeDiagnosticSnapshotV1", "authority": "none",
                "performance_qualified": False, "scope": check.RANK_SCOPE,
                "process_id": records[0]["worker_pids"][0], "device_unique_id": plan["device_unique_id"],
                "rank": 0, "ordinal": ordinal, "counters": counters}]})
    return records, diagnostics, plan, reference


def validate(values):
    return check.validate_records(*values)


class TargetRuntimeProfileTests(unittest.TestCase):
    def test_valid_profile_preserves_exact_work_and_truthful_diagnostic_scope(self):
        values = fixture()
        unchanged = copy.deepcopy(values)
        result = validate(values)
        self.assertEqual(values, unchanged)
        self.assertEqual(result["schema"], "FerricTargetBatchRuntimeProfileDiagnosticV1")
        self.assertEqual(result["dispatches"], 22176)
        self.assertTrue(result["reference_tokens_and_bytes_match"])
        self.assertTrue(result["runtime_profiling"])
        self.assertTrue(result["configuration"]["performance_profile"]["runtime_profiling"])
        self.assertTrue(result["completion_polls_measured"])
        self.assertFalse(result["performance_qualified"])
        self.assertFalse(result["gpu_timestamps"])
        self.assertEqual(result["runtime_counters"]["workload_delta"]["completion_polls"], 30000)

    def test_normalization_never_mutates_or_weakens_frozen_core_policy(self):
        records, _, plan, reference = fixture()
        with self.assertRaises(ValueError):
            check.core.checked_plan(check.common_plan(plan) | {"performance_profile": plan["performance_profile"]})
        with self.assertRaises(ValueError):
            check.core.validate_records(records, check.common_plan(plan), reference)
        self.assertEqual(check.core.sha256(Path(check.core.__file__).read_bytes()), check.CORE_SHA256)

    def test_common_source_pin_rejects_substitution_and_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "core.py"
            path.write_text("raise RuntimeError('must not execute')\n")
            with self.assertRaises(ValueError):
                check.load_core(path)
            link = Path(directory) / "link.py"
            link.symlink_to(check.core.__file__)
            with self.assertRaises(OSError):
                check.load_core(link)

    def test_requires_exact_profile_plan_and_controller(self):
        for key, value in (("schema", "FerricTargetBatchDecodeExpectationV1"),
                           ("common_comparator_sha256", "a" * 64), ("controller_sha256", "b" * 64),
                           ("new_tokens", 2), ("collective", "host-staged-reuse-v3"),
                           ("host_timing_enabled", True)):
            values = fixture()
            values[2][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate(values)
        for key, value in (("runtime_profiling", False), ("runtime_cache_admission", False),
                           ("runtime_operational", False), ("dispatch_sequences", True),
                           ("projection", "wave"), ("attention", "wave"), ("queue_rollover", True)):
            values = fixture()
            values[2]["performance_profile"][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate(values)

    def test_setup_profile_extensions_and_status_must_match_exactly(self):
        for key, value in (("runtime_diagnostic_status", "different"), ("numerical_status", check.COMMON_NUMERICAL_STATUS),
                           ("head_precision", "fp32"), ("private_path", "/private/example")):
            values = fixture()
            values[0][0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate(values)

    def test_requires_two_ordered_single_rank_snapshots(self):
        for replacement in ([], fixture()[1][:1], fixture()[1][::-1], fixture()[1] * 2):
            records, _, plan, reference = fixture()
            with self.assertRaises(ValueError):
                check.validate_records(records, replacement, plan, reference)
        values = fixture()
        values[1][0]["ranks"] *= 2
        with self.assertRaises(ValueError):
            validate(values)

    def test_every_envelope_and_rank_identity_is_bound(self):
        for key, value in (("phase", "other"), ("measurement", "GPU"), ("performance_qualified", True),
                           ("authority", "protected"), ("extra", 1)):
            values = fixture()
            values[1][0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate(values)
        for key, value in (("process_id", 1), ("device_unique_id", 1), ("rank", 1), ("ordinal", 2),
                           ("scope", "GPU"), ("performance_qualified", True), ("extra", 1)):
            values = fixture()
            values[1][1]["ranks"][0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate(values)

    def test_counters_are_complete_exact_u64_and_monotonic(self):
        for name in check.COUNTERS:
            for replacement in (-1, 1 << 64, True, 1.0):
                values = fixture()
                values[1][1]["ranks"][0]["counters"][name] = replacement
                with self.subTest(name=name, value=replacement), self.assertRaises(ValueError):
                    validate(values)
            values = fixture()
            del values[1][1]["ranks"][0]["counters"][name]
            with self.subTest(name=name), self.assertRaises(ValueError):
                validate(values)
        values = fixture()
        values[1][1]["ranks"][0]["counters"]["reads"] = 0
        with self.assertRaises(ValueError):
            validate(values)

    def test_large_device_and_counter_integers_remain_exact(self):
        values = fixture()
        device = (1 << 63) + 12345
        values[2]["device_unique_id"] = device
        values[0][0]["device_unique_ids"] = [device]
        for envelope in values[1]:
            rank = envelope["ranks"][0]
            rank["device_unique_id"] = device
            rank["counters"]["commands"] += 1 << 63
        result = validate(values)
        self.assertEqual(result["runtime_counters"]["workload_delta"]["commands"], 22198)
        values[1][1]["ranks"][0]["device_unique_id"] = float(device)
        with self.assertRaises(ValueError):
            validate(values)

    def test_exact_dispatch_polls_admissions_and_phase_relationships(self):
        changes = [("dispatches", 22175), ("completion_polls", 22175), ("kernel_admissions", 14),
                   ("kernel_admission_ns", 131), ("commands", 22275), ("command_ns", 10001),
                   ("dispatch_wait_ns", 0), ("read_ns", 30)]
        for key, value in changes:
            values = fixture()
            values[1][1]["ranks"][0]["counters"][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate(values)

    def test_common_token_page_identity_and_closure_rejections_are_preserved(self):
        for kind in ("token", "pages", "worker", "closure"):
            values = fixture()
            if kind == "token":
                values[0][-2]["generated_tokens"][12] = 9856
            elif kind == "pages":
                values[0][2]["free_pages"] = 4
            elif kind == "worker":
                values[0][0]["running_worker_sha256"] = ["a" * 64]
            else:
                values[0][-1]["all_workers_exited"] = False
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                validate(values)

    def test_public_report_redacts_all_process_device_and_session_fields(self):
        text = json.dumps(validate(fixture()))
        for private in ("87654321", "987654321", "process_id", "device_unique_id", "worker_pids", "session_id", "/home/", "/private/"):
            self.assertNotIn(private, text)

    def test_ndjson_bounds_duplicate_keys_and_nonfinite_values_reject(self):
        for raw in (b"{}\n", b"{}\n{}", b"{}\n\n", b"{\"x\":1,\"x\":2}\n{}\n", b"{\"x\":NaN}\n{}\n"):
            with self.assertRaises(ValueError):
                check.records_from_bytes(raw, 2, "diagnostic")

    def test_diagnostic_stderr_requires_exact_status_then_two_snapshots(self):
        snapshots = fixture()[1]
        raw = b"".join(json.dumps(value).encode() + b"\n" for value in snapshots)
        self.assertEqual(check.diagnostic_records_from_bytes(check.STDERR_PREFIX + raw), snapshots)
        for changed in (raw, b"\n" + check.STDERR_PREFIX + raw,
                        check.STDERR_PREFIX.replace(b": 0\n", b": 1\n") + raw,
                        check.STDERR_PREFIX + check.STDERR_PREFIX + raw,
                        check.STDERR_PREFIX + raw + b"unexpected\n",
                        check.STDERR_PREFIX + b"unexpected\n" + raw,
                        raw + check.STDERR_PREFIX):
            with self.subTest(changed=changed[:60]), self.assertRaises(ValueError):
                check.diagnostic_records_from_bytes(changed)

    def test_failed_status_and_unpinned_reference_fail_before_output(self):
        with self.assertRaises(ValueError):
            check.compare(b"", b"", b"1\n", b"", b"", b"")
        _, _, plan, reference = fixture()
        with self.assertRaisesRegex(ValueError, "digest"):
            check.compare(b"", b"", b"0\n", json.dumps(check.expected_workload(plan)).encode(),
                          json.dumps(reference).encode(), json.dumps(plan).encode())


if __name__ == "__main__":
    unittest.main()
