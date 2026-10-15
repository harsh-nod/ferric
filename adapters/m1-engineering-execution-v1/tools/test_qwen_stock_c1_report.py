import copy
import unittest

import qwen_stock_c1_report as report


class StockReportTest(unittest.TestCase):
    def fixture(self):
        record = {"success": True, "token_ids": [1] * 256, "ttft_ns": 1000000,
                  "tpot_ns": 2000000, "e2e_ns": 511000000,
                  "usage": {"prompt_tokens": 128, "completion_tokens": 256, "total_tokens": 384}}
        run = {"schema": "FerricCompetitiveTokenStreamingRunV1", "engine": "vllm",
                "concurrency": 1, "arrival_policy": "bounded-closed-loop-windows",
                "completed": True, "workload": {"schema": "FerricCompetitiveTokenWorkloadV1", "requests": [{"prompt": [1] * 128,
                "max_tokens": 256}]}, **{phase: [{"index": i, "started_ns": 0,
                "completed_ns": 512000000, "requests": [copy.deepcopy(record)]}
                for i in range(n)] for phase, n in (("warmups", 10), ("samples", 30))}}
        ordinal = 0
        for phase in ("warmups", "samples"):
            for window in run[phase]:
                start = ordinal * 512000001
                window.update(started_ns=start, completed_ns=start + 512000000)
                window["requests"][0].update(started_ns=start + 1000,
                                             completed_ns=start + 511001000)
                ordinal += 1
        return run

    def test_mismatch_does_not_grant_equivalence(self):
        out = report.summarize(self.fixture(), [2] * 256, [1] * 256)
        self.assertTrue(out["complete_declared_baseline"])
        self.assertFalse(out["performance_qualified"])
        self.assertFalse(out["ferric_comparison"])
        self.assertTrue(all(not r["reference_equal"] and r["retained_stock_equal"] for r in out["parity"]))
        self.assertEqual(out["ttft_ms"]["mean"], 1)
        self.assertEqual(out["successful_output_tokens"], 7680)

    def test_partial_and_failed_preserved(self):
        run = self.fixture(); run["completed"] = False
        run["samples"] = run["samples"][:2]
        run["samples"][1]["requests"][0].update(success=False, error="timeout")
        out = report.summarize(run, [2] * 256, [1] * 256)
        self.assertEqual(out["counts"]["samples"], {"planned": 30, "recorded": 2,
                         "successful": 1, "failed": 1, "missing": 28})
        self.assertFalse(out["complete_declared_baseline"])
        self.assertEqual(out["closed_loop_output_tokens_per_second"], 250)
        self.assertEqual(out["parity"][-1]["error"], "timeout")

    def test_no_measured_samples(self):
        run = self.fixture(); run["samples"] = []; run["completed"] = False
        out = report.summarize(run, [1] * 256, [1] * 256)
        self.assertIsNone(out["ttft_ms"])
        self.assertIsNone(out["closed_loop_output_tokens_per_second"])

    def test_warmups_excluded(self):
        run = self.fixture(); run["warmups"][0]["requests"][0].update(ttft_ns=2000000, tpot_ns=1000000)
        self.assertEqual(report.summarize(run, [1] * 256, [1] * 256)["ttft_ms"]["mean"], 1)

    def test_variation_retained(self):
        run = self.fixture(); run["samples"][0]["requests"][0]["token_ids"] = [3] * 256
        out = report.summarize(run, [2] * 256, [1] * 256)
        self.assertFalse(out["parity"][10]["retained_stock_equal"])

    def test_bad_accounting_rejected(self):
        run = self.fixture(); run["samples"][0]["requests"][0]["usage"]["completion_tokens"] = 255
        with self.assertRaises(ValueError): report.summarize(run, [1] * 256, [1] * 256)

    def test_additional_usage_details_allowed(self):
        run = self.fixture()
        run["samples"][0]["requests"][0]["usage"]["prompt_tokens_details"] = None
        self.assertTrue(report.summarize(run, [1] * 256, [1] * 256)["complete_declared_baseline"])

    def test_nonfinite_rejected(self):
        run = self.fixture(); run["samples"][0]["requests"][0]["tpot_ns"] = float("nan")
        with self.assertRaises(ValueError): report.summarize(run, [1] * 256, [1] * 256)

    def test_wrong_mode_rejected(self):
        run = self.fixture(); run["concurrency"] = 2
        with self.assertRaises(ValueError): report.summarize(run, [1] * 256, [1] * 256)

    def test_malformed_window_and_request_endpoints(self):
        for endpoint in ("started_ns", "completed_ns"):
            for value in (float("nan"), float("inf"), True, False, 1.5, "1", None, -1, 2**63):
                for target in ("window", "request"):
                    with self.subTest(endpoint=endpoint, value=value, target=target):
                        run = self.fixture()
                        window = run["samples"][0]
                        record = window if target == "window" else window["requests"][0]
                        record[endpoint] = value
                        with self.assertRaises(ValueError):
                            report.summarize(run, [1] * 256, [1] * 256)

    def test_outside_and_reversed_request_intervals(self):
        for failure in ("before", "after", "zero", "reverse"):
            for success in (True, False):
                with self.subTest(failure=failure, success=success):
                    run = self.fixture(); w = run["samples"][0]; r = w["requests"][0]
                    r["success"] = success
                    if failure == "before": r["started_ns"] = w["started_ns"] - 1
                    if failure == "after": r["completed_ns"] = w["completed_ns"] + 1
                    if failure == "zero": r["completed_ns"] = r["started_ns"]
                    if failure == "reverse": r["completed_ns"] = r["started_ns"] - 1
                    with self.assertRaises(ValueError): report.summarize(run, [1] * 256, [1] * 256)

    def test_positive_but_clipped_window_rejected(self):
        run = self.fixture(); w = run["samples"][0]
        w["completed_ns"] = w["started_ns"] + 10
        with self.assertRaises(ValueError): report.summarize(run, [1] * 256, [1] * 256)

    def test_overlapping_reordered_and_cross_phase_windows(self):
        for failure in ("overlap", "reorder", "cross_phase", "zero", "reverse"):
            with self.subTest(failure=failure):
                run = self.fixture(); first, second = run["samples"][:2]
                if failure == "overlap":
                    for k in ("started_ns", "completed_ns"):
                        second[k] -= 2
                        second["requests"][0][k] -= 2
                if failure == "reorder":
                    for k in ("started_ns", "completed_ns", "requests"):
                        first[k], second[k] = second[k], first[k]
                if failure == "cross_phase":
                    for k in ("started_ns", "completed_ns", "requests"):
                        first[k] = copy.deepcopy(run["warmups"][-1][k])
                if failure == "zero": first["completed_ns"] = first["started_ns"]
                if failure == "reverse": first["completed_ns"] = first["started_ns"] - 1
                with self.assertRaises(ValueError): report.summarize(run, [1] * 256, [1] * 256)

    def test_run_error_prevents_complete_without_erasing_samples(self):
        for error in ("failed after final window", "", False, 0):
            run = self.fixture(); run["run_error"] = error
            out = report.summarize(run, [1] * 256, [1] * 256)
            self.assertFalse(out["complete_declared_baseline"])
            self.assertEqual(out["run_error"], error)
            self.assertEqual(out["counts"]["samples"]["successful"], 30)

    def test_touching_windows_allowed(self):
        run = self.fixture(); w = run["samples"][0]
        for k in ("started_ns", "completed_ns"):
            w[k] -= 1; w["requests"][0][k] -= 1
        self.assertTrue(report.summarize(run, [1] * 256, [1] * 256)["complete_declared_baseline"])


if __name__ == "__main__":
    unittest.main()
