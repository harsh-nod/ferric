"""Synthetic host ledger tests; no GPU timing or model-performance evidence."""

import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock


def module(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + ".py"))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


LEDGER = module("performance_ledger")
FIXTURE = module("test_compare_tp_batch")
CHECK = LEDGER.CHECK
PINS = {**FIXTURE.PINS, "artifact_manifest_id": "4" * 64, "artifact_handoff_id": "5" * 64}


def expected(reference_hash=CHECK.REFERENCE_SHA256, workload_hash="7" * 64):
    return {**PINS, "world": 8, "prefix_cache": True, "output_head_pruning": None,
            "collective": None, "performance_profile": None,
            "workload_sha256": workload_hash, "reference_sha256": reference_hash}


def run(identifier):
    return {"id": identifier, "run_dir": "/tmp/" + identifier, "workload": "/tmp/workload.json",
            "reference": "/tmp/reference.json", "comparison": "/tmp/" + identifier + ".json",
            "comparison_sha256": "8" * 64}


def manifest():
    return {"schema": LEDGER.SCHEMA, "baseline": "baseline", "warmup_policy": "fresh-worker-no-warmup",
            "variants": [{"name": "baseline", "kind": "baseline", "expect": expected(),
                          "runs": [run("baseline-r1"), run("baseline-r2")]},
                         {"name": "optimized", "kind": "standalone", "expect": expected(),
                          "runs": [run("optimized-r1"), run("optimized-r2")]}]}


def synthetic_loader(run_value, _expected):
    metrics = LEDGER.extract_metrics(FIXTURE.fixture())
    scale = 0.5 if run_value["id"].startswith("optimized") else 1.0
    if run_value["id"].endswith("r2"):
        scale *= 2
    for key in LEDGER.GLOBAL_METRICS:
        metrics[key] *= 1 / scale if key == "output_tokens_per_second" else scale
    for request in metrics["requests"].values():
        for key in LEDGER.REQUEST_METRICS:
            if request[key] is not None:
                request[key] *= 1 / scale if key == "output_tokens_per_second" else scale
    metrics["id"] = run_value["id"]
    return metrics, {"tensor_parallel": 8, "context_tokens": 128}


class LedgerTests(unittest.TestCase):
    def test_nearest_rank_is_explicit_and_small_p95_is_worst(self):
        stats = LEDGER.statistics([4.0, 1.0, 3.0, 2.0])
        self.assertEqual(stats, {"n": 4, "mean": 2.5, "min": 1.0, "max": 4.0, "p50": 2.0, "p95": 4.0})
        self.assertIsNone(LEDGER.statistics([None, None]))
        for values in ([], [1, None], [True], [0], [float("inf")]):
            with self.assertRaises(ValueError):
                LEDGER.statistics(values)

    def test_request_and_workload_windows_use_terminal_events_not_sum_rates(self):
        records = FIXTURE.fixture()
        metrics = LEDGER.extract_metrics(records)
        self.assertEqual(metrics["output_tokens"], 8)
        self.assertAlmostEqual(metrics["workload_window_seconds"], 4.104)
        self.assertAlmostEqual(metrics["output_tokens_per_second"], 8 / 4.104)
        cancelled = metrics["requests"]["cancel-between-batches"]
        self.assertIsNone(cancelled["tpot_seconds"])
        self.assertEqual(cancelled["decode_interval_count"], 0)
        self.assertEqual(cancelled["terminal_window_ns"], 1_001_000_000)
        self.assertAlmostEqual(cancelled["output_tokens_per_second"], 1 / 1.001)
        self.assertNotAlmostEqual(metrics["output_tokens_per_second"],
                                  sum(item["output_tokens_per_second"] for item in metrics["requests"].values()))
        next(record for record in records if record.get("state") == "Cancelled")["cancelled_ns"] = 9_000_000_000
        self.assertAlmostEqual(LEDGER.extract_metrics(records)["workload_window_seconds"], 8.999)

    def test_aggregation_keeps_request_identities_and_correct_ratio_direction(self):
        report = LEDGER.aggregate(manifest(), synthetic_loader)
        optimized = report["variants"][1]
        self.assertEqual(optimized["repetitions"], 2)
        self.assertEqual(set(optimized["requests"]), set(CHECK.NAMES))
        self.assertAlmostEqual(optimized["baseline_relative"]["output_tokens_per_second"]["p50"]["improvement_percent"], 100)
        for name, request in optimized["requests"].items():
            self.assertAlmostEqual(request["baseline_relative"]["ttft_seconds"]["p50"]["improvement_percent"], 50)
            self.assertEqual(request["metrics"]["ttft_seconds"]["n"], 2)
            if name == "cancel-between-batches":
                self.assertIsNone(request["metrics"]["tpot_seconds"])
        text = LEDGER.markdown(report)
        self.assertIn("Decode Gaps/Rep", text)
        self.assertIn("not a stable tail", text)
        self.assertIn("n/a", text)

    def test_equivalence_and_request_interval_drift_reject(self):
        def different_world(run_value, expectation):
            metrics, equivalence = synthetic_loader(run_value, expectation)
            if run_value["id"].startswith("optimized"):
                equivalence["tensor_parallel"] = 1
            return metrics, equivalence
        with self.assertRaisesRegex(ValueError, "equivalence"):
            LEDGER.aggregate(manifest(), different_world)
        def different_intervals(run_value, expectation):
            metrics, equivalence = synthetic_loader(run_value, expectation)
            if run_value["id"] == "optimized-r2":
                metrics["requests"]["seed-prefix"]["decode_interval_count"] = 2
            return metrics, equivalence
        with self.assertRaisesRegex(ValueError, "interval count"):
            LEDGER.aggregate(manifest(), different_intervals)

    def test_manifest_rejects_missing_pins_reused_runs_unknown_modes_and_warmup(self):
        mutations = [
            lambda value: value["variants"][0]["expect"].pop("artifact_manifest_id"),
            lambda value: value["variants"][1]["runs"][0].update(id="baseline-r1"),
            lambda value: value["variants"][1]["runs"][0].update(run_dir="/tmp/baseline-r1"),
            lambda value: value["variants"][1].update(kind="untrusted"),
            lambda value: value["variants"][1]["expect"].update(collective="peer-device"),
            lambda value: value["variants"][1]["expect"].update(output_head_pruning=1),
            lambda value: value.update(warmup_policy="warm"),
            lambda value: value.update(baseline="missing"),
        ]
        for mutation in mutations:
            changed = manifest()
            mutation(changed)
            with self.assertRaises(ValueError):
                LEDGER.validate_manifest(changed)

    def test_canonical_comparison_does_not_equate_booleans_and_numbers(self):
        self.assertNotEqual(LEDGER.canonical({"passed": True}), LEDGER.canonical({"passed": 1}))

    def test_file_loader_revalidates_raw_status_idle_and_exact_current_report(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            reference = FIXTURE.encoded(FIXTURE.synthetic_reference())
            workload = FIXTURE.encoded(CHECK.expected_workload())
            files = {"status": b"0\n", "gpu-before.json": FIXTURE.encoded(FIXTURE.snapshots()),
                     "gpu-after.json": FIXTURE.encoded(FIXTURE.snapshots()),
                     "results.jsonl": b"".join(FIXTURE.encoded(row) for row in FIXTURE.fixture()),
                     "workload.json": workload, "reference.json": reference}
            for filename, data in files.items():
                (root / filename).write_bytes(data)
            expected_value = expected(CHECK.sha256(reference), CHECK.sha256(workload))
            run_value = {"id": "r1", "run_dir": str(root), "workload": str(root / "workload.json"),
                         "reference": str(root / "reference.json"), "comparison": str(root / "comparison.json"),
                         "comparison_sha256": "8" * 64}
            with mock.patch.object(CHECK, "REFERENCE_SHA256", CHECK.sha256(reference)):
                report = CHECK.compare(root, root / "workload.json", root / "reference.json", 8, PINS, True,
                                       expected_workload_hash=CHECK.sha256(workload),
                                       expected_reference_hash=CHECK.sha256(reference))
                comparison = FIXTURE.encoded(report)
                (root / "comparison.json").write_bytes(comparison)
                run_value["comparison_sha256"] = CHECK.sha256(comparison)
                metrics, equivalence = LEDGER.load_run(run_value, expected_value)
                self.assertEqual(metrics["output_tokens"], 8)
                self.assertEqual(equivalence["tensor_parallel"], 8)
                for filename, bad in (("status", b"1\n"), ("gpu-after.json", b"{}\n"),
                                      ("results.jsonl", files["results.jsonl"] + b"\n")):
                    (root / filename).write_bytes(bad)
                    with self.assertRaises(ValueError):
                        LEDGER.load_run(run_value, expected_value)
                    (root / filename).write_bytes(files[filename])
                stale = copy.deepcopy(report)
                stale["comparator_sha256"] = "9" * 64
                stale_bytes = FIXTURE.encoded(stale)
                (root / "comparison.json").write_bytes(stale_bytes)
                run_value["comparison_sha256"] = CHECK.sha256(stale_bytes)
                with self.assertRaisesRegex(ValueError, "stale"):
                    LEDGER.load_run(run_value, expected_value)

    def test_changed_artifact_hashes_are_permitted_but_expected_profile_is_not_free_form(self):
        value = manifest()
        value["variants"][1]["expect"]["artifact_hsaco_id"] = "9" * 64
        value["variants"][1]["expect"]["performance_profile"] = copy.deepcopy(FIXTURE.PROFILE)
        LEDGER.validate_manifest(value)
        value["variants"][1]["expect"]["performance_profile"]["runtime_profiling"] = "off"
        with self.assertRaises(ValueError):
            LEDGER.validate_manifest(value)


if __name__ == "__main__":
    unittest.main()
