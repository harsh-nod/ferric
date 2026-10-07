"""Synthetic CPU-only fixtures in the retained matched HTTP summary schemas."""

import contextlib
import copy
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest

import compare_paired_starts as paired


class PairedStartsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.reference = {"prompt_token_ids": list(range(128)),
                          "generated_token_ids": list(range(128, 256)),
                          "generated_utf8_hex": "answer".encode().hex(),
                          "workload_sha256": "a" * 64}
        self.reference_sha = self.write(self.root / "reference.json", self.reference)
        self.plan = {"settings": paired.SETTINGS}
        self.manifest = {"schema": paired.MANIFEST_SCHEMA, "reference": "reference.json", "pairs": []}
        self.manifest_path = self.root / "series.json"

    @staticmethod
    def write(path, value):
        raw = (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + "\n").encode()
        path.write_bytes(raw)
        return hashlib.sha256(raw).hexdigest()

    def final(self, request_id):
        return {"event": "request", "request_id": request_id, "state": "Completed", "admitted": True,
                "cancelled_ns": None, "cached_prefix_tokens": 0,
                "prompt_tokens": self.reference["prompt_token_ids"],
                "generated_tokens": self.reference["generated_token_ids"],
                "generated_utf8_bytes": list(b"answer")}

    def engine(self, index, engine, start):
        directory = self.root / f"pair-{index}-{engine}"
        directory.mkdir()
        hashes = {}

        def retain(name, value):
            hashes[name] = self.write(directory / name, value)

        retain("frozen-plan.json", self.plan)
        identity = {"engine": engine, "plan_sha256": hashes["frozen-plan.json"],
                    "settings": paired.SETTINGS}
        retain("identity.json", identity)
        receipt = {"schema": "FerricCandidateMatched128ReceiptV2", "engine": engine,
                   "plan_sha256": hashes["frozen-plan.json"], "settings": paired.SETTINGS,
                   "timing_admitted": True, "cleanup_completed": True, "errors": [],
                   "qualification": False, "framework_win_claim": False,
                   "numerical_diagnostics": [{"admitted": True, "classification": "exact_match",
                                              "first_token_mismatch": None}] * 2,
                   "timing_replay": {"timing_admitted": True, "classification": "exact_utf8_and_usage",
                                     "token_itl_available": False, "qualification": False,
                                     "framework_win_claim": False}}
        inputs = {"model_files": {"target/config.json": {"sha256": "b" * 64}}}
        retain("input-identities-before.json", inputs)
        retain("input-identities-after.json", inputs)
        if engine == "ferric":
            receipt["timed_stream_checks"] = {"timed_final_token_ids_checked": 40,
                                              "timed_http_request_identities_checked": 40}
            retain("ferric-final-events.json", [self.final(value) for value in range(1, 43)])
            cleanup = {"controller_status": 0, "signals": [], "threads_joined": True,
                       "worker_close_receipt": True}
        else:
            diagnostic = {"choices": [{"prompt_token_ids": self.reference["prompt_token_ids"],
                                      "token_ids": self.reference["generated_token_ids"],
                                      "text": "answer", "finish_reason": "length"}]}
            retain("diagnostic-before.json", diagnostic)
            retain("diagnostic-after.json", diagnostic)
            cleanup = {"host_exit_clean": True, "internal_force_kill_observed": False,
                       "owned_container_absence_check_status": 1}
        retain("receipt.json", receipt)
        report = {"schema": "FerricCompetitiveStreamingRunV1", "authority": "none",
                  "engine": engine, "completed": True, "qualification": False,
                  "ttft_semantics": paired.TTFT, "tpot_semantics": paired.TPOT,
                  "token_itl_available": False, "concurrency": 1,
                  "arrival_policy": "bounded-closed-loop-windows",
                  "identity": identity, "identity_sha256": hashes["identity.json"],
                  "client_sha256": "c" * 64, "workload_sha256": self.reference["workload_sha256"],
                  "warmups": [], "samples": []}
        tpot = 100_000 if engine == "ferric" else 200_000
        ttft = 1_000_000 if engine == "ferric" else 2_000_000
        for phase, count in (("warmups", 10), ("samples", 30)):
            for number in range(count):
                sent = start + 100
                first = sent + ttft
                last = first + 127 * tpot
                done = last + 1000
                record = {"success": True, "started_ns": sent, "first_text_ns": first,
                          "last_text_ns": last, "completed_ns": done, "ttft_ns": ttft,
                          "tpot_ns": float(tpot), "e2e_ns": done - sent,
                          "usage": {"prompt_tokens": 128, "completion_tokens": 128, "total_tokens": 256},
                          "text": "answer", "token_itl_ns": None}
                report[phase].append({"index": number, "started_ns": start,
                                      "completed_ns": done + 100, "requests": [record]})
                start = done + 10000
        retain("timed-raw.json", report)
        measured = [report["samples"][0]["started_ns"], report["samples"][-1]["completed_ns"]]
        metrics = paired.bench.aggregate([row["requests"][0] for row in report["samples"]],
                                         *measured, 10000, 1000)
        return {"engine": engine, "output_directory": str(directory), "input_hashes": hashes,
                "warmup_requests": 10, "measured_requests": 30, "untimed_diagnostics": 2,
                "measured_output_tokens": 3840, "metrics": metrics, "cleanup": cleanup,
                "gpu_postflight_idle": True, "measured_window_ns": measured}

    def add_pair(self, order=None):
        index = len(self.manifest["pairs"])
        order = order or (["vllm", "ferric"] if index % 2 == 0 else ["ferric", "vllm"])
        engines = {engine: self.engine(index, engine, 10**12 + index * 10**12 + rank * 10**11)
                   for rank, engine in enumerate(order)}
        summary = {"schema": "FerricCandidateMatchedPairSummaryV2", "accepted": True,
                   "qualification": False, "framework_win_claim": False,
                   "plan_sha256": engines["ferric"]["input_hashes"]["frozen-plan.json"],
                   "settings": paired.SETTINGS, "reference_sha256": self.reference_sha,
                   "client_sha256": "c" * 64,
                   "ferric_build": {"controller_sha256": "d" * 64, "worker_sha256": "e" * 64},
                   "vllm_image": {"digest": "vllm@sha256:" + "f" * 64},
                   "cohort_order": order, "throughput_formula": paired.THROUGHPUT, "engines": engines}
        name = f"pair-{index}.json"
        self.write(self.root / name, summary)
        self.manifest["pairs"].append({"summary": name, "ferric": f"pair-{index}-ferric",
                                       "vllm": f"pair-{index}-vllm"})
        return summary

    def result(self):
        self.write(self.manifest_path, self.manifest)
        return paired.summarize(self.manifest_path)

    def add_v3_pair(self):
        index = len(self.manifest["pairs"])
        summary = self.add_pair()
        summary["schema"] = "FerricCandidateMatchedPairSummaryV3"
        summary["ferric_build"].update(
            variant="prefill16-decode-composed-v28", mode="composed",
            live_profile="prefill16-decode-composed-v28-live-v1",
            selectors={"prefill_kv_mode": "parallel-prefill16-v27",
                       "split_attention_mode": "split8-v21", "c1_packet_mode": "packed16-v22"},
            images={"FAKE-IMAGE-" + str(number): {"manifest_sha256": "a" * 64, "hsaco_sha256": "b" * 64}
                    for number in range(7)})
        self.write(self.root / f"pair-{index}.json", summary)
        for engine in ("ferric", "vllm"):
            self.mutate(index, engine, "receipt.json",
                        lambda value: value.update(schema="FerricCandidateMatched128ReceiptV3"))

    def mutate(self, index, engine, name, change):
        path = self.root / f"pair-{index}-{engine}" / name
        value = json.loads(path.read_bytes())
        change(value)
        digest = self.write(path, value)
        summary_path = self.root / f"pair-{index}.json"
        summary = json.loads(summary_path.read_bytes())
        summary["engines"][engine]["input_hashes"][name] = digest
        self.write(summary_path, summary)

    def test_three_alternating_pairs_report_ratios_not_qualification(self):
        for _ in range(3):
            self.add_pair()
        result = self.result()
        self.assertTrue(result["accepted"])
        self.assertEqual(result["classification"], "repeated-pair-screening")
        self.assertEqual(result["ferric_over_vllm_spread"]["mean_tpot_ms"]["mean"], .5)
        self.assertEqual(result["ferric_over_vllm_spread"]["mean_ttft_ms"]["sample_stdev"], 0)
        self.assertFalse(result["competitiveness_accepted"])
        self.assertFalse(result["framework_win_claim"])
        self.assertFalse(result["qualification"])
        self.assertFalse(result["token_itl_available"])
        self.assertGreater(result["attempts"][0]["ferric_over_vllm"]["output_tokens_per_second"], 1)

    def test_single_pair_is_only_limited_screening(self):
        self.add_pair()
        result = self.result()
        self.assertEqual(result["classification"], "limited-pair-screening")
        self.assertIsNone(result["ferric_over_vllm_spread"]["mean_tpot_ms"]["sample_stdev"])

    def test_old_summary_schema_is_supported_without_mixing_profiles(self):
        summary = self.add_pair()
        summary["schema"] = "FerricV17MatchedPairSummaryV1"
        self.write(self.root / "pair-0.json", summary)
        for engine in ("ferric", "vllm"):
            self.mutate(0, engine, "receipt.json", lambda value: value.update(schema="FerricV17Matched128ReceiptV1"))
        self.assertTrue(self.result()["accepted"])

    def test_v3_composed_pairs_remain_descriptive_screening(self):
        for _ in range(3):
            self.add_v3_pair()
        result = self.result()
        self.assertTrue(result["accepted"])
        self.assertEqual(result["classification"], "repeated-pair-screening")
        self.assertEqual(result["ferric_over_vllm_spread"]["mean_tpot_ms"]["mean"], 0.5)
        self.assertFalse(result["competitiveness_accepted"])
        self.assertFalse(result["qualification"])
        self.assertFalse(result["framework_win_claim"])

    def test_v3_summary_requires_v3_receipt_on_both_engines(self):
        self.add_v3_pair()
        for engine in ("ferric", "vllm"):
            self.mutate(0, engine, "receipt.json",
                        lambda value: value.update(schema="FerricCandidateMatched128ReceiptV2"))
            self.assertFalse(self.result()["accepted"])
            self.mutate(0, engine, "receipt.json",
                        lambda value: value.update(schema="FerricCandidateMatched128ReceiptV3"))
        self.assertTrue(self.result()["accepted"])

    def test_v3_selector_or_image_drift_rejects_the_whole_series(self):
        self.add_v3_pair()
        self.add_v3_pair()
        path = self.root / "pair-1.json"
        original = json.loads(path.read_bytes())
        for mutate in (
                lambda value: value["ferric_build"]["selectors"].update(c1_packet_mode="baseline"),
                lambda value: value["ferric_build"]["images"]["FAKE-IMAGE-6"].update(hsaco_sha256="c" * 64)):
            changed = copy.deepcopy(original)
            mutate(changed)
            self.write(path, changed)
            result = self.result()
            self.assertFalse(result["accepted"])
            self.assertIn("profile/build", result["errors"][0]["error"])
            self.assertIsNone(result["ferric_over_vllm_spread"])

    def test_v3_failed_attempt_is_not_dropped_from_aggregate(self):
        for _ in range(3):
            self.add_v3_pair()
        self.mutate(1, "ferric", "receipt.json", lambda value: value.update(timing_admitted=False))
        result = self.result()
        self.assertFalse(result["accepted"])
        self.assertEqual(len(result["attempts"]), 3)
        self.assertFalse(result["attempts"][1]["accepted"])
        self.assertIsNone(result["ferric_over_vllm_spread"])

    def test_non_alternating_series_is_rejected(self):
        self.add_pair(["vllm", "ferric"])
        self.add_pair(["vllm", "ferric"])
        result = self.result()
        self.assertFalse(result["accepted"])
        self.assertIn("alternate", result["errors"][0]["error"])
        self.assertIsNone(result["ferric_over_vllm_spread"])

    def test_failed_pair_is_retained_and_entire_aggregate_is_rejected(self):
        self.add_pair()
        summary = self.add_pair()
        summary["accepted"] = False
        self.write(self.root / "pair-1.json", summary)
        self.add_pair()
        result = self.result()
        self.assertEqual(len(result["attempts"]), 3)
        self.assertFalse(result["attempts"][1]["accepted"])
        self.assertIsNone(result["ferric_over_vllm_spread"])

    def test_missing_pair_is_not_silently_omitted(self):
        self.add_pair()
        self.manifest["pairs"].append({"summary": "missing.json", "ferric": "missing-f", "vllm": "missing-v"})
        result = self.result()
        self.assertFalse(result["accepted"])
        self.assertEqual(len(result["attempts"]), 2)

    def test_profile_or_compiler_changes_cannot_be_pooled(self):
        self.add_pair()
        summary = self.add_pair()
        summary["ferric_build"]["worker_sha256"] = "1" * 64
        self.write(self.root / "pair-1.json", summary)
        self.assertIn("profile/build", self.result()["errors"][0]["error"])

    def test_incompatible_precision_or_boolean_settings_rejected(self):
        summary = self.add_pair()
        for key, value in (("head_dtype", "bfloat16"), ("concurrency", True)):
            modified = copy.deepcopy(summary)
            modified["settings"][key] = value
            self.write(self.root / "pair-0.json", modified)
            self.assertFalse(self.result()["accepted"])

    def test_bad_diagnostic_receipt_is_rejected_even_if_summary_accepted(self):
        self.add_pair()
        self.mutate(0, "vllm", "receipt.json", lambda value: value["numerical_diagnostics"][0].update(admitted=False))
        self.assertIn("diagnostic correctness", self.result()["errors"][0]["error"])

    def test_wrong_actual_ids_are_rejected_even_if_receipt_claims_parity(self):
        self.add_pair()
        self.mutate(0, "ferric", "ferric-final-events.json", lambda rows: rows[15]["generated_tokens"].__setitem__(0, 999))
        self.assertIn("output IDs", self.result()["errors"][0]["error"])

    def test_wrong_vllm_diagnostic_ids_are_rejected(self):
        self.add_pair()
        self.mutate(0, "vllm", "diagnostic-after.json", lambda value: value["choices"][0]["token_ids"].__setitem__(0, 999))
        self.assertIn("diagnostic output", self.result()["errors"][0]["error"])

    def test_request_failure_and_wrong_output_are_rejected(self):
        self.add_pair()
        self.mutate(0, "ferric", "timed-raw.json", lambda value: value["samples"][0]["requests"][0].update(success=False))
        self.assertIn("failed request", self.result()["errors"][0]["error"])
        self.mutate(0, "ferric", "timed-raw.json", lambda value: value["samples"][0]["requests"][0].update(success=True, text="wrong"))
        self.assertIn("correctness mismatch", self.result()["errors"][0]["error"])

    def test_wrong_tpot_denominator_is_rejected(self):
        self.add_pair()
        self.mutate(0, "ferric", "timed-raw.json", lambda value: value["samples"][0]["requests"][0].update(tpot_ns=100000 * 127 / 128))
        self.assertIn("TPOT definition", self.result()["errors"][0]["error"])

    def test_incomplete_cohort_is_not_trimmed_to_match(self):
        self.add_pair()
        self.mutate(0, "ferric", "timed-raw.json", lambda value: value["samples"].pop())
        self.assertIn("incomplete samples", self.result()["errors"][0]["error"])

    def test_summary_metrics_are_recomputed(self):
        summary = self.add_pair()
        summary["engines"]["ferric"]["metrics"]["ttft_ms"]["mean"] = .01
        self.write(self.root / "pair-0.json", summary)
        self.assertIn("summary metrics", self.result()["errors"][0]["error"])

    def test_modified_raw_file_requires_hash_match(self):
        self.add_pair()
        path = self.root / "pair-0-ferric" / "timed-raw.json"
        path.write_bytes(path.read_bytes() + b" ")
        self.assertIn("retained hash differs", self.result()["errors"][0]["error"])

    def test_reusing_a_pair_does_not_make_independent_starts(self):
        self.add_pair()
        self.manifest["pairs"].append(copy.deepcopy(self.manifest["pairs"][0]))
        self.assertFalse(self.result()["accepted"])

    def test_roster_must_be_chronological_not_sorted_for_a_win(self):
        for _ in range(3):
            self.add_pair()
        self.manifest["pairs"].reverse()
        self.assertIn("chronological", self.result()["errors"][0]["error"])

    def test_json_duplicate_keys_and_nonfinite_are_rejected(self):
        path = self.root / "bad.json"
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}', b'{"a":1e999}'):
            path.write_bytes(raw)
            with self.assertRaises(ValueError):
                paired.read_json(path)

    def test_cli_returns_nonzero_and_retains_rejected_attempt(self):
        summary = self.add_pair()
        summary["accepted"] = False
        self.write(self.root / "pair-0.json", summary)
        self.write(self.manifest_path, self.manifest)
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            status = paired.main(["--manifest", str(self.manifest_path)])
        self.assertEqual(status, 1)
        self.assertEqual(len(json.loads(output.getvalue())["attempts"]), 1)


if __name__ == "__main__":
    unittest.main()
