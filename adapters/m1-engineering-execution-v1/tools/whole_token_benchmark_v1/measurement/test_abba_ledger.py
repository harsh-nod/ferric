import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SPEC = importlib.util.spec_from_file_location("abba_ledger", Path(__file__).with_name("abba_ledger.py"))
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)


def write_binding(root, name, value):
    raw = m.canonical(value)
    (root / name).write_bytes(raw)
    return {"path": name, "sha256": m.sha(raw)}


def reference():
    return {"generated_token_ids": list(range(128)), "generated_utf8_hex": b"ab".hex()}


def device_sample(phase):
    active = phase == "active"
    users = [{"identity": {"pid": 10}}] if active else []
    return {"schema": "FerricDeviceDescriptorSampleV1", "method": "proc-fd-rdev-all-pids",
            "phase": phase, "accepted": True, "complete": True, "errors": [],
            "foreign_users": [], "device_users": users, "owned_users": users}


def lifecycle():
    return {"schema": "FerricMeasuredCellLifecycleV1", "exit_status": 0, "clean_teardown": True,
            "forced_signal": False, "errors": [],
            "device_samples": [device_sample(v) for v in ("preflight", "active", "postflight")]}


def raw_request(start, tpot=50_000_000, ttft=800_000_000):
    first, last = start + ttft, start + ttft + 127 * tpot
    usage = {"prompt_tokens": 128, "completion_tokens": 128, "total_tokens": 256}
    return {"id": "prompt-0", "success": True, "started_ns": start, "first_text_ns": first,
            "last_text_ns": last, "completed_ns": last + 1000,
            "ttft_ns": ttft, "tpot_ns": tpot, "e2e_ns": last + 1000 - start,
            "usage": usage, "text": "ab", "chunks": [
                {"received_ns": first, "event": {"choices": [{"index": 0, "text": "a"}]}},
                {"received_ns": last, "event": {"choices": [{"index": 0, "text": "b", "finish_reason": "length"}]}},
                {"received_ns": last + 500, "event": {"choices": [], "usage": usage}}]}


def fixture(root, candidate_tpot=45_000_000, candidate_ttft=800_000_000):
    ref = write_binding(root, "reference.json", reference())
    plan = {"schema": "FerricChangePlanV1", "change_id": "whole-program-r1", "gates": m.GATES,
            "reference_sha256": ref["sha256"], "client_sha256": "c" * 64, "workload_sha256": "d" * 64,
            "timing_semantics": "http-text-chunk-v1",
            "workload": {"input_tokens": 128, "output_tokens": 128, "context_tokens": 8192,
                         "concurrency": 1, "tensor_parallel": 1, "greedy": True,
                         "prefix_caching": False, "speculation": False},
            "profiles": {arm: {"artifacts": {"worker": ("a" if arm == "A" else "b") * 64},
                               "configuration": {"backend": "grouped" if arm == "A" else "whole",
                                                 "model": "Qwen3-8B", "packed_down": False}} for arm in ("A", "B")},
            "allowed_profile_differences": ["/artifacts/worker", "/configuration/backend"],
            "mechanism": {arm: {"dispatches_per_decode_token": 652, "publications_per_decode_token": 11 if arm == "A" else 1,
                                "waits_per_decode_token": 11 if arm == "A" else 1,
                                "retired_signals_per_decode_token": 652} for arm in ("A", "B")}}
    counter = {"schema": "FerricDecodeMechanismV1", "plan_sha256": m.digest(plan), "arms": {}}
    for arm in ("A", "B"):
        expected = plan["mechanism"][arm]
        counter["arms"][arm] = {"profile_sha256": m.digest(plan["profiles"][arm]), "decode_tokens": 127,
                                "correctness_passed": True, "per_decode_token": expected,
                                "totals": {k.removesuffix("_per_decode_token"): v * 127 for k, v in expected.items()}}
    attempt = {"schema": "FerricChangeAttemptV1", "plan_sha256": m.digest(plan), "reference": ref,
               "mechanism": write_binding(root, "mechanism.json", counter), "cells": []}
    for number in range(12):
        arm = m.GATES["order"][number % 4]
        profile = m.digest(plan["profiles"][arm])
        run = {"schema": "FerricCompetitiveStreamingRunV1", "completed": True, "concurrency": 1,
               "arrival_policy": "bounded-closed-loop-windows", "workload_sha256": plan["workload_sha256"],
               "client_sha256": plan["client_sha256"], "identity": {"profile_sha256": profile},
               "ttft_semantics": "client-send-to-first-nonempty-text-chunk",
               "tpot_semantics": "first-to-last-text-chunk-divided-by-usage-tokens-minus-one",
               "warmups": [], "samples": []}
        proof = {"schema": "FerricCellCorrectnessV1", "requests": []}
        start = 1000000000000 * (number + 1)
        for phase, count in (("warmups", 2), ("samples", 4)):
            for index in range(count):
                record = raw_request(start + 1000,
                                     candidate_tpot if arm == "B" else 50_000_000,
                                     candidate_ttft if arm == "B" else 800_000_000)
                run[phase].append({"index": index, "started_ns": start, "completed_ns": record["completed_ns"] + 1000,
                                   "requests": [record], "metrics": {"deliberately_ignored": "derived again"}})
                proof["requests"].append({"phase": phase, "index": index, "request_id": record["id"],
                                          **reference(), "expected_frontier": 100000, "completed_frontier": 100000,
                                          "expected_dispatches": 82804, "completed_dispatches": 82804,
                                          "timeout": False, "poison": False, "fallback": False, "leak": False})
                start = record["completed_ns"] + 2000
        run_binding = write_binding(root, f"run-{number}.json", run)
        proof["run_sha256"] = run_binding["sha256"]
        attempt["cells"].append({"cell_id": f"cell-{number}", "block": number // 4, "position": number % 4,
                                 "arm": arm, "profile_sha256": profile, "instrumented": False,
                                 "run": run_binding, "correctness": write_binding(root, f"proof-{number}.json", proof),
                                 "lifecycle": write_binding(root, f"life-{number}.json", lifecycle())})
    return plan, attempt


class AbbaTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.plan, self.attempt = fixture(self.root)

    def evaluate(self):
        return m.evaluate(self.plan, self.attempt, self.root)

    def replace(self, cell_number, field, mutate):
        cell = self.attempt["cells"][cell_number]
        bound = cell[field]
        value = m.binding(self.root, bound)
        mutate(value)
        cell[field] = write_binding(self.root, bound["path"], value)
        if field == "run":
            self.replace(cell_number, "correctness", lambda v: v.update(run_sha256=cell[field]["sha256"]))

    def test_complete_repeatable_gain(self):
        result = self.evaluate()
        self.assertEqual(result["status"], "promotable")
        self.assertAlmostEqual(result["median_tpot_gain_percent"], 10)
        self.assertEqual(result["arms"]["A"]["requests"], 24)
        self.assertEqual(len(result["pairs"]), 6)

    def test_exact_five_percent_gain(self):
        self.plan, self.attempt = fixture(self.root, candidate_tpot=47_500_000)
        self.assertEqual(self.evaluate()["status"], "promotable")

    def test_small_gain_not_promoted(self):
        self.plan, self.attempt = fixture(self.root, candidate_tpot=49_000_000)
        self.assertEqual(self.evaluate()["status"], "inconclusive")

    def test_ttft_regression_not_promoted(self):
        self.plan, self.attempt = fixture(self.root, candidate_ttft=850_000_000)
        self.assertFalse(self.evaluate()["checks"]["regression_limits"])

    def test_missing_cell_rejected(self):
        self.attempt["cells"].pop()
        with self.assertRaisesRegex(ValueError, "twelve"):
            self.evaluate()

    def test_duplicate_cell_rejected(self):
        self.attempt["cells"][1]["cell_id"] = "cell-0"
        with self.assertRaisesRegex(ValueError, "duplicate cell"):
            self.evaluate()

    def test_order_drift_rejected(self):
        self.attempt["cells"][0]["arm"] = "B"
        with self.assertRaisesRegex(ValueError, "ABBA order"):
            self.evaluate()

    def test_profile_drift_rejected(self):
        self.attempt["cells"][0]["profile_sha256"] = "f" * 64
        with self.assertRaisesRegex(ValueError, "profile"):
            self.evaluate()

    def test_composition_difference_must_be_explicit(self):
        self.plan["profiles"]["B"]["configuration"]["packed_down"] = True
        with self.assertRaisesRegex(ValueError, "difference"):
            m.validate_plan(self.plan)

    def test_gate_tuning_rejected(self):
        plan = copy.deepcopy(self.plan)
        plan["gates"]["minimum_faster_pairs"] = 4
        with self.assertRaisesRegex(ValueError, "gates"):
            m.validate_plan(plan)

    def test_instrumented_timing_rejected(self):
        self.attempt["cells"][0]["instrumented"] = True
        with self.assertRaisesRegex(ValueError, "instrumented"):
            self.evaluate()

    def test_failed_warmup_rejected(self):
        self.replace(0, "run", lambda v: v["warmups"][0]["requests"][0].update(success=False))
        with self.assertRaisesRegex(ValueError, "request failed"):
            self.evaluate()

    def test_missing_warmup_rejected(self):
        self.replace(0, "run", lambda v: v["warmups"].pop())
        with self.assertRaisesRegex(ValueError, "request counts"):
            self.evaluate()

    def test_wrong_output_ids_rejected(self):
        self.replace(0, "correctness", lambda v: v["requests"][0]["generated_token_ids"].__setitem__(127, 999))
        with self.assertRaisesRegex(ValueError, "token/byte"):
            self.evaluate()

    def test_failed_frontier_rejected(self):
        self.replace(0, "correctness", lambda v: v["requests"][0].update(completed_frontier=999))
        with self.assertRaisesRegex(ValueError, "frontier"):
            self.evaluate()

    def test_failed_dispatch_count_rejected(self):
        self.replace(0, "correctness", lambda v: v["requests"][0].update(completed_dispatches=999))
        with self.assertRaisesRegex(ValueError, "dispatch"):
            self.evaluate()

    def test_fallback_rejected(self):
        self.replace(0, "correctness", lambda v: v["requests"][0].update(fallback=True))
        with self.assertRaisesRegex(ValueError, "fallback"):
            self.evaluate()

    def test_timing_summary_not_trusted(self):
        self.replace(0, "run", lambda v: v["samples"][0]["requests"][0].update(tpot_ns=1))
        with self.assertRaisesRegex(ValueError, "derived"):
            self.evaluate()

    def test_chunk_time_not_trusted(self):
        self.replace(0, "run", lambda v: v["samples"][0]["requests"][0]["chunks"][0].update(received_ns=1))
        with self.assertRaisesRegex(ValueError, "clock"):
            self.evaluate()

    def test_token_text_mismatch_rejected(self):
        self.replace(0, "run", lambda v: v["samples"][0]["requests"][0].update(text="wrong"))
        with self.assertRaisesRegex(ValueError, "streamed output"):
            self.evaluate()

    def test_fuser_only_lifecycle_rejected(self):
        self.replace(0, "lifecycle", lambda v: v["device_samples"][1].update(method="fuser"))
        with self.assertRaisesRegex(ValueError, "fuser"):
            self.evaluate()

    def test_empty_active_attribution_rejected(self):
        self.replace(0, "lifecycle", lambda v: v["device_samples"][1].update(owned_users=[]))
        with self.assertRaisesRegex(ValueError, "positive"):
            self.evaluate()

    def test_unclean_teardown_rejected(self):
        self.replace(0, "lifecycle", lambda v: v.update(forced_signal=True))
        with self.assertRaisesRegex(ValueError, "unclean"):
            self.evaluate()

    def test_mechanism_total_drift_rejected(self):
        bound = self.attempt["mechanism"]
        value = m.binding(self.root, bound)
        value["arms"]["B"]["totals"]["publications"] = 1397
        self.attempt["mechanism"] = write_binding(self.root, bound["path"], value)
        with self.assertRaisesRegex(ValueError, "total invariant"):
            self.evaluate()

    def test_hash_mismatch_rejected(self):
        (self.root / "run-0.json").write_bytes(b"{}")
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            self.evaluate()

    def test_raw_run_cannot_be_reused(self):
        self.attempt["cells"][3]["run"] = self.attempt["cells"][0]["run"]
        with self.assertRaisesRegex(ValueError, "reused"):
            self.evaluate()

    def test_json_duplicates_and_nonfinite_rejected(self):
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}'):
            with self.assertRaises(ValueError):
                m.parse(raw)

    def test_ledger_chain_and_tamper_detection(self):
        path = self.root / "ledger.jsonl"
        entry = self.evaluate()
        first = m.append_ledger(path, entry)
        second = m.append_ledger(path, entry)
        self.assertEqual(second["previous_sha256"], first["sha256"])
        raw = path.read_bytes().replace(b'"sequence":0', b'"sequence":9', 1)
        path.write_bytes(raw)
        with self.assertRaisesRegex(ValueError, "chain"):
            m.append_ledger(path, entry)

    def test_pending_and_failed_outcomes_retained(self):
        path = self.root / "ledger.jsonl"
        for status in ("pending", "failed"):
            m.append_ledger(path, {"schema": "FerricChangeEvaluationV1", "change_id": "whole-program-r1",
                                  "plan_sha256": m.digest(self.plan), "status": status, "note": "not a speedup"})
        self.assertEqual(len(path.read_bytes().splitlines()), 2)

    def test_interrupted_ledger_not_extended(self):
        path = self.root / "ledger.jsonl"
        path.write_bytes(b'{"unfinished":')
        with self.assertRaisesRegex(ValueError, "interrupted"):
            m.append_ledger(path, self.evaluate())

    def test_percentile_uses_predeclared_linear_method(self):
        self.assertAlmostEqual(m.percentile([1, 2, 3, 4], .95), 3.85)


if __name__ == "__main__":
    unittest.main()
