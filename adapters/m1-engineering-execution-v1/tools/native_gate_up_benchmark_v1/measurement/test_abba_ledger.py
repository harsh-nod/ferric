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


def fixture(root, candidate_tpot=50_000_000, candidate_ttft=720_000_000):
    ref = write_binding(root, "reference.json", reference())
    plan = {"schema": "FerricPrefillChangePlanV1", "change_id": "whole-program-r1", "gates": m.GATES,
            "reference_sha256": ref["sha256"], "client_sha256": "c" * 64, "workload_sha256": "d" * 64,
            "timing_semantics": "http-text-chunk-v1",
            "workload": {"input_tokens": 128, "output_tokens": 128, "context_tokens": 8192,
                         "concurrency": 1, "tensor_parallel": 1, "greedy": True,
                         "prefix_caching": False, "speculation": False},
            "profiles": {arm: {"artifacts": {"worker": ("a" if arm == "A" else "b") * 64},
                               "configuration": {"backend": "grouped" if arm == "A" else "whole",
                                                 "model": "Qwen3-8B", "packed_down": False}} for arm in ("A", "B")},
            "allowed_profile_differences": ["/artifacts/worker", "/configuration/backend"],
            "mechanism": {arm: {"prefill_chunks": 8, "prefill_commands_per_chunk": 613,
                  "prefill_program_dynamic_slots": 216 if arm == "B" else 0,
                  "decode_program_executions": 127, "decode_commands_per_program": 652,
                  "decode_dynamic_slots": 180, "program_executions": 135 if arm == "B" else 127,
                  "program_dispatches": 87708 if arm == "B" else 82804,
                  "program_publications": 135 if arm == "B" else 127,
                  "program_final_waits": 135 if arm == "B" else 127,
                  "model_dispatches": 87711, "model_batches": 135} for arm in ("A", "B")}}
    counter = {"schema": "FerricPrefillRequestMechanismV1", "plan_sha256": m.digest(plan), "arms": {}}
    for arm in ("A", "B"):
        expected = plan["mechanism"][arm]
        counter["arms"][arm] = {"profile_sha256": m.digest(plan["profiles"][arm]), "decode_tokens": 127,
                                "correctness_passed": True, "request_mechanism": expected}
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


WIDTH_SPEC = importlib.util.spec_from_file_location("width_metric_fixtures", Path(__file__).parent.parent / "test_prefill_width.py")
width = importlib.util.module_from_spec(WIDTH_SPEC)
WIDTH_SPEC.loader.exec_module(width)


class AbbaTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        # Historical HTTP utilities remain exercised, but no HTTP attempt can
        # enter this native-width campaign or acquire a promotion result.
        self.http_plan, self.attempt = fixture(self.root)
        self.plan = width.plan()
        self.results = width.measured_cells()

    def evaluate(self):
        return m.compare_results(self.plan, "f" * 64, self.results)

    def utility_cell(self):
        return m.cell_metrics(self.http_plan, self.attempt["cells"][0], self.root, reference())

    def replace(self, cell_number, field, mutate):
        cell = self.attempt["cells"][cell_number]
        bound = cell[field]
        value = m.binding(self.root, bound)
        mutate(value)
        cell[field] = write_binding(self.root, bound["path"], value)
        if field == "run":
            self.replace(cell_number, "correctness", lambda v: v.update(run_sha256=cell[field]["sha256"]))

    def test_http_evaluator_cannot_admit_width_or_historical_plan(self):
        for plan in (self.plan, self.http_plan, {**self.plan, "timing_semantics": "http-text-chunk-v1"}):
            with self.assertRaises(ValueError):
                m.evaluate(plan, self.attempt, self.root)

    def test_complete_repeatable_gain(self):
        result = self.evaluate()
        self.assertEqual(result["status"], "experimental-gates-passed")
        self.assertAlmostEqual(result["median_tpot_gain_percent"], 10)
        self.assertEqual(result["arms"]["A"]["requests"], 24)
        self.assertEqual(len(result["pairs"]), 6)

    def test_exact_five_percent_gain(self):
        self.results = width.measured_cells(tpot_b=47.5)
        self.assertEqual(self.evaluate()["status"], "experimental-gates-passed")

    def test_small_gain_not_promoted(self):
        self.results = width.measured_cells(tpot_b=49)
        self.assertEqual(self.evaluate()["status"], "inconclusive")

    def test_ttft_regression_not_promoted(self):
        self.results = width.measured_cells(ttft_b=850)
        self.assertFalse(self.evaluate()["checks"]["regression_limits"])

    def test_decode_gain_passes_without_claiming_prefill_gain(self):
        self.results = width.measured_cells(tpot_b=40, ttft_b=800)
        result = self.evaluate()
        self.assertEqual(result["status"], "experimental-gates-passed")
        self.assertEqual(result["median_ttft_gain_percent"], 0)
        self.assertFalse(result["default_promotion"])

    def test_decode_regression_blocks_ttft_gain(self):
        self.results = width.measured_cells(tpot_b=53)
        self.assertFalse(self.evaluate()["checks"]["median_tpot_gain"])

    def test_missing_cell_rejected(self):
        self.results.pop()
        with self.assertRaisesRegex(ValueError, "twelve"):
            self.evaluate()

    def test_duplicate_cell_rejected(self):
        self.results[1]["cell_id"] = self.results[0]["cell_id"]
        with self.assertRaisesRegex(ValueError, "distinct"):
            self.evaluate()

    def test_order_drift_rejected(self):
        self.results[0]["arm"] = "B"
        with self.assertRaisesRegex(ValueError, "ordered ABBA"):
            self.evaluate()

    def test_profile_drift_rejected(self):
        self.attempt["cells"][0]["profile_sha256"] = "f" * 64
        with self.assertRaisesRegex(ValueError, "profile"):
            self.utility_cell()

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
            self.utility_cell()

    def test_failed_warmup_rejected(self):
        self.replace(0, "run", lambda v: v["warmups"][0]["requests"][0].update(success=False))
        with self.assertRaisesRegex(ValueError, "request failed"):
            self.utility_cell()

    def test_missing_warmup_rejected(self):
        self.replace(0, "run", lambda v: v["warmups"].pop())
        with self.assertRaisesRegex(ValueError, "request counts"):
            self.utility_cell()

    def test_wrong_output_ids_rejected(self):
        self.replace(0, "correctness", lambda v: v["requests"][0]["generated_token_ids"].__setitem__(127, 999))
        with self.assertRaisesRegex(ValueError, "token/byte"):
            self.utility_cell()

    def test_failed_frontier_rejected(self):
        self.replace(0, "correctness", lambda v: v["requests"][0].update(completed_frontier=999))
        with self.assertRaisesRegex(ValueError, "frontier"):
            self.utility_cell()

    def test_failed_dispatch_count_rejected(self):
        self.replace(0, "correctness", lambda v: v["requests"][0].update(completed_dispatches=999))
        with self.assertRaisesRegex(ValueError, "dispatch"):
            self.utility_cell()

    def test_fallback_rejected(self):
        self.replace(0, "correctness", lambda v: v["requests"][0].update(fallback=True))
        with self.assertRaisesRegex(ValueError, "fallback"):
            self.utility_cell()

    def test_timing_summary_not_trusted(self):
        self.replace(0, "run", lambda v: v["samples"][0]["requests"][0].update(tpot_ns=1))
        with self.assertRaisesRegex(ValueError, "derived"):
            self.utility_cell()

    def test_chunk_time_not_trusted(self):
        self.replace(0, "run", lambda v: v["samples"][0]["requests"][0]["chunks"][0].update(received_ns=1))
        with self.assertRaisesRegex(ValueError, "clock"):
            self.utility_cell()

    def test_token_text_mismatch_rejected(self):
        self.replace(0, "run", lambda v: v["samples"][0]["requests"][0].update(text="wrong"))
        with self.assertRaisesRegex(ValueError, "streamed output"):
            self.utility_cell()

    def test_fuser_only_lifecycle_rejected(self):
        self.replace(0, "lifecycle", lambda v: v["device_samples"][1].update(method="fuser"))
        with self.assertRaisesRegex(ValueError, "fuser"):
            self.utility_cell()

    def test_empty_active_attribution_rejected(self):
        self.replace(0, "lifecycle", lambda v: v["device_samples"][1].update(owned_users=[]))
        with self.assertRaisesRegex(ValueError, "positive"):
            self.utility_cell()

    def test_unclean_teardown_rejected(self):
        self.replace(0, "lifecycle", lambda v: v.update(forced_signal=True))
        with self.assertRaisesRegex(ValueError, "unclean"):
            self.utility_cell()

    def test_mechanism_total_drift_rejected(self):
        self.plan["mechanism"]["B"]["program_publications"] = 1397
        with self.assertRaisesRegex(ValueError, "whole-request"):
            m.validate_plan(self.plan)

    def test_hash_mismatch_rejected(self):
        (self.root / "run-0.json").write_bytes(b"{}")
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            self.utility_cell()

    def test_raw_run_cannot_be_reused(self):
        spec = importlib.util.spec_from_file_location("native_raw_reuse", Path(__file__).with_name("test_native_token_cell.py"))
        native = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(native)
        plan, cells, counters = native.campaign_fixture()
        cells[1]["result"]["raw_sha256"]["stdout.raw"] = cells[0]["result"]["raw_sha256"]["stdout.raw"]
        native.rebind_outer(cells[1])
        with self.assertRaisesRegex(ValueError, "reused"):
            native.m.evaluate_campaign(plan, cells, counters)

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
            m.append_ledger(path, {"schema": "FerricNativeGateUpChangeEvaluationR1",
                                  "change_id": "width-r1", "plan_sha256": m.digest(self.plan),
                                  "status": status, "note": "not a speedup"})
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
