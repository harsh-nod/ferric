"""Synthetic host-span contract tests; no benchmark observations."""

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import target_head_host_timing_v1 as check
from test_target_head_decode_v1 import fixture as head_fixture


def entry(label, count=1, batch=None, category="span", rank=None, dispatches=0):
    duration = 100000 if label == "controller" else count * 3
    return {"batch": batch, "phase": label, "category": category, "label": label, "rank": rank,
            "count": count, "failed": 0, "elapsed_ns": duration, "max_ns": duration // count,
            "request_payload_bytes": 0, "response_payload_bytes": 0, "dispatches": dispatches}


def fixture(variant="mfma-fp32-v7"):
    records, plan, reference = head_fixture(variant)
    plan.update(schema="FerricTargetHeadHostTimingExpectationV1", host_timing_enabled=True)
    rows = []
    globals_ = {"controller", "setup", "workload", "close"}
    rows.extend(entry(label) for label in globals_)
    for record in records:
        if record["schema"] != "FerricQwen3TpBatchCompletedV2":
            continue
        for label in check.PUBLIC_SPANS - globals_:
            rows.append(entry(label, 36 if label in {"attention", "feed_forward"} else 1,
                              batch=record["pool_batch_id"]))
    rows.append(entry("controller_batch", 36))
    rows.extend([entry("dispatch", 19584, category="ipc_send", rank=0),
                 entry("dispatch", 19584, category="ipc_roundtrip", rank=0, dispatches=19584)])
    sidecar = {**check.load_timing().CONSTANTS, "controller_pid": 42,
               "setup": copy.deepcopy(records[0]), "closed": copy.deepcopy(records[-1]),
               "workload_sha256": "a" * 64, "records": rows}
    return records, plan, reference, sidecar, "a" * 64


class HostSpanTests(unittest.TestCase):
    def test_all_variants_keep_precision_tokens_and_instrumentation_explicit(self):
        for variant in check.head.VARIANTS:
            report = check.validate_records(*fixture(variant))
            self.assertEqual(report["generated_tokens"], check.batch.TOKENS)
            self.assertEqual(report["dispatches"], 19584)
            self.assertTrue(report["configuration"]["host_timing_enabled"])
            self.assertFalse(report["benchmark_qualified"])
            self.assertEqual(report["host_spans"]["attention"]["count"], 1296)
            self.assertEqual(report["host_spans"]["output_head_argmax"]["count"], 36)

    def test_no_mutation_or_uninstrumented_policy_widening(self):
        args = fixture()
        before = copy.deepcopy(args)
        check.validate_records(*args)
        self.assertEqual(args, before)
        with self.assertRaises(ValueError):
            check.head.checked_plan(args[1])

    def test_plan_stays_explicit(self):
        for key, value in (("host_timing_enabled", False), ("host_timing_enabled", 1),
                           ("schema", "FerricTargetHeadDecodeExpectationV1"),
                           ("collective", "device-tp1-v3"), ("new_tokens", 2)):
            args = fixture()
            args[1][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(*args)

    def test_all_reference_choices_remain_checked(self):
        for index in range(32):
            args = fixture()
            args[0][index + 6]["outputs"][0]["token"] = 42
            with self.subTest(index=index), self.assertRaises(ValueError):
                check.validate_records(*args)

    def test_sidecar_identity_failure_missing_counts_and_overflow_reject(self):
        mutations = [lambda s: s.update(run_status="failed"), lambda s: s.update(incomplete=True),
                     lambda s: s.update(active_records=1), lambda s: s.update(workload_sha256="b" * 64),
                     lambda s: s["setup"].update(controller_sha256="b" * 64),
                     lambda s: s["closed"].update(all_workers_exited=False),
                     lambda s: s["records"].pop(),
                     lambda s: s["records"][-1].update(dispatches=1),
                     lambda s: s["records"][-1].update(count=19584.0),
                     lambda s: s["records"][-1].update(elapsed_ns=2**64),
                     lambda s: s["records"].append(copy.deepcopy(s["records"][0])),
                     lambda s: s["records"].__setitem__(slice(None),
                                [r for r in s["records"] if r["label"] != "attention"])]
        for mutate in mutations:
            args = fixture()
            mutate(args[3])
            with self.subTest(mutation=mutate), self.assertRaises(ValueError):
                check.validate_records(*args)

    def test_no_raw_pids_paths_or_record_origins_in_public_report(self):
        text = json.dumps(check.validate_records(*fixture()))
        for private in ("controller_pid", "worker_pids", "session_id", "device_unique_id", "/home/"):
            self.assertNotIn(private, text)

    def test_dependency_drift_rejects(self):
        with mock.patch.object(check, "HEAD_SHA", "a" * 64):
            with self.assertRaises(ValueError):
                check.pinned_base()
        with mock.patch.dict(check.HELPER_SHA, {"host_timing_summary.py": "a" * 64}):
            with self.assertRaises(ValueError):
                check.load_timing()

    def test_per_batch_reassignment_missing_scope_rank_and_multiplicity_reject(self):
        for key, value in (("batch", None), ("batch", 999), ("phase", "wrong_phase"),
                           ("rank", 0), ("count", 36)):
            args = fixture()
            row = next(r for r in args[3]["records"] if r["label"] == "output_head")
            row[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(*args)

    def test_compare_hashes_original_instrumented_plan_and_sidecar(self):
        records, plan, reference, sidecar, _ = fixture()
        workload = json.dumps(check.batch.expected_workload(plan)).encode()
        sidecar["workload_sha256"] = check.batch.sha256(workload)
        capture = b"".join(json.dumps(row).encode() + b"\n" for row in records)
        expected = json.dumps(plan).encode()
        timing = json.dumps(sidecar).encode()
        with mock.patch.object(check.batch, "load_reference", return_value=reference):
            report = check.compare(capture, b"0\n", workload, b"synthetic", expected, timing)
            self.assertEqual(report["input_sha256"]["expectation"], check.batch.sha256(expected))
            self.assertEqual(report["input_sha256"]["host_timing"], check.batch.sha256(timing))
            for status, raw in ((b"1\n", capture), (b"0\n", capture[:-1])):
                with self.assertRaises(ValueError):
                    check.compare(raw, status, workload, b"synthetic", expected, timing)

    def test_cli_failure_creates_no_report(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "bad"
            source.write_bytes(b"invalid")
            output = Path(directory) / "report"
            args = [argument for key in ("capture", "status", "workload", "reference", "expect", "timing")
                    for argument in ("--" + key, str(source))] + ["--output", str(output)]
            with self.assertRaises(SystemExit):
                check.main(args)
            self.assertFalse(output.exists())

    def test_private_helper_snapshot_is_removed_after_loading(self):
        module = check.load_timing()
        snapshot = Path(module.__file__).parent
        self.assertFalse(snapshot.exists())
        self.assertEqual(module.CONSTANTS["aggregation"], "overlapping-not-additive")


if __name__ == "__main__":
    unittest.main()
