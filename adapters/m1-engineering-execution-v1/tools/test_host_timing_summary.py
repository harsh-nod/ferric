"""Synthetic host instrumentation checks. These are not GPU observations."""

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


SUMMARY = module("host_timing_summary")
FIXTURE = module("test_compare_tp_batch")
LEDGER_FIXTURE = module("test_performance_ledger")
CHECK = SUMMARY.CHECK


def entry(label, elapsed=10, count=1, batch=None, rank=None, category="span", dispatches=0, phase=None):
    return {"batch": batch, "phase": phase or label, "category": category, "label": label, "rank": rank,
            "count": count, "failed": 0, "elapsed_ns": elapsed, "max_ns": elapsed // count,
            "request_payload_bytes": 0, "response_payload_bytes": 0, "dispatches": dispatches}


def sidecar(records, expected):
    batches = [r for r in records if r["schema"] == CHECK.SCHEMA_PREFIX + "CompletedV2"]
    rows = [entry("controller", 100_000), entry("setup", 1000), entry("workload", 10_000), entry("close", 1000),
            entry("controller_batch", 1000, len(batches))]
    rows += [entry("batch", batch=r["pool_batch_id"]) for r in batches]
    for rank, count in enumerate(records[-1]["rank_dispatch_counts"]):
        rows += [entry("dispatch", count * 2, count, rank=rank, category="ipc_send", phase="workload"),
                 entry("dispatch", count * 3, count, rank=rank, category="ipc_roundtrip", dispatches=count, phase="workload")]
    return {**SUMMARY.CONSTANTS, "controller_pid": 42, "setup": copy.deepcopy(records[0]),
            "closed": copy.deepcopy(records[-1]), "workload_sha256": expected["workload_sha256"], "records": rows}


class HostTimingTests(unittest.TestCase):
    def test_accepts_bound_whole_path_and_exact_dispatch_counts(self):
        records, expected = FIXTURE.fixture(), LEDGER_FIXTURE.expected()
        rows = SUMMARY.validate_sidecar(sidecar(records, expected), records, expected)
        self.assertEqual(sum(row["dispatches"] for row in rows), sum(records[-1]["rank_dispatch_counts"]))

    def test_failures_incomplete_identity_and_count_mutations_reject(self):
        records, expected = FIXTURE.fixture(), LEDGER_FIXTURE.expected()
        original = sidecar(records, expected)
        mutations = [lambda value: value.update(run_status="failed"),
                     lambda value: value.update(incomplete=True),
                     lambda value: value.update(active_records=1),
                     lambda value: value.update(workload_sha256="f" * 64),
                     lambda value: value["setup"].update(controller_sha256="f" * 64),
                     lambda value: value["setup"].update(tensor_parallel=8.0),
                     lambda value: value["closed"].update(all_workers_exited=False),
                     lambda value: value["records"][-1].update(failed=1),
                     lambda value: value["records"][-1].update(dispatches=1),
                     lambda value: value["records"][-1].update(rank=8),
                     lambda value: value["records"][-1].update(batch=999),
                     lambda value: value["records"].pop(),
                     lambda value: value["records"].append(value["records"][0]),
                     lambda value: value["records"][0].update(elapsed_ns=1, max_ns=1),
                     lambda value: value.update(active_records=False)]
        for mutate in mutations:
            changed = copy.deepcopy(original)
            mutate(changed)
            with self.subTest(mutation=mutate), self.assertRaises(ValueError):
                SUMMARY.validate_sidecar(changed, records, expected)

    def test_manifest_pins_timing_and_rejects_unpaired_policy(self):
        manifest = LEDGER_FIXTURE.manifest()
        manifest["schema"] = SUMMARY.SCHEMA
        for variant in manifest["variants"]:
            for run in variant["runs"]:
                run.update(timing=run["run_dir"] + "/host-timing.json", timing_sha256="a" * 64)
        SUMMARY.validate_manifest(manifest)

        def loader(run, expected):
            rate = 0.5 if run["id"].startswith("optimized") else 1
            key = ("workload", "span", "workload", None)
            return {"metrics": {}, "timing_sha256": run["timing_sha256"],
                    "aggregates": {key: entry("workload", int(100 * rate))}}, {"world": expected["world"]}

        report = SUMMARY.aggregate(manifest, loader)
        self.assertEqual(report["variants"][1]["observations"][0]["baseline_elapsed_ratio"], 0.5)
        self.assertIn("Host wall latency only", SUMMARY.markdown(report))
        changed = copy.deepcopy(manifest)
        changed["variants"][1]["expect"]["world"] = 2
        with self.assertRaisesRegex(ValueError, "incompatible"):
            SUMMARY.aggregate(changed, loader)
        del manifest["variants"][0]["runs"][0]["timing_sha256"]
        with self.assertRaises(ValueError):
            SUMMARY.validate_manifest(manifest)

    def test_file_loader_rechecks_reference_and_idle_before_accepting_timings(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            records = FIXTURE.fixture()
            reference = FIXTURE.encoded(FIXTURE.synthetic_reference())
            workload = FIXTURE.encoded(CHECK.expected_workload())
            expected = LEDGER_FIXTURE.expected(CHECK.sha256(reference), CHECK.sha256(workload))
            files = {"status": b"0\n", "gpu-before.json": FIXTURE.encoded(FIXTURE.snapshots()),
                     "gpu-after.json": FIXTURE.encoded(FIXTURE.snapshots()),
                     "results.jsonl": b"".join(FIXTURE.encoded(r) for r in records),
                     "workload.json": workload, "reference.json": reference,
                     "host-timing.json": FIXTURE.encoded(sidecar(records, expected))}
            for name, data in files.items():
                (root / name).write_bytes(data)
            run = {"id": "baseline", "run_dir": str(root), "workload": str(root / "workload.json"),
                   "reference": str(root / "reference.json"), "comparison": str(root / "comparison.json"),
                   "timing": str(root / "host-timing.json"), "timing_sha256": CHECK.sha256(files["host-timing.json"])}
            with mock.patch.object(CHECK, "REFERENCE_SHA256", CHECK.sha256(reference)):
                comparison = CHECK.compare(root, run["workload"], run["reference"], 8, LEDGER_FIXTURE.PINS, True,
                                           expected_workload_hash=CHECK.sha256(workload), expected_reference_hash=CHECK.sha256(reference))
                raw = FIXTURE.encoded(comparison)
                (root / "comparison.json").write_bytes(raw)
                run["comparison_sha256"] = CHECK.sha256(raw)
                actual, _ = SUMMARY.load_timed_run(run, expected)
                self.assertEqual(actual["metrics"]["output_tokens"], 8)
                (root / "status").write_bytes(b"1\n")
                with self.assertRaisesRegex(ValueError, "successfully"):
                    SUMMARY.load_timed_run(run, expected)
                (root / "status").write_bytes(b"0\n")
                bad_gpu = FIXTURE.snapshots()
                bad_gpu["card0"]["GPU use (%)"] = "1"
                (root / "gpu-after.json").write_bytes(FIXTURE.encoded(bad_gpu))
                with self.assertRaises(ValueError):
                    SUMMARY.load_timed_run(run, expected)
                (root / "gpu-after.json").write_bytes(files["gpu-after.json"])
                wrong_tokens = copy.deepcopy(records)
                request = next(row for row in wrong_tokens if row["schema"] == CHECK.SCHEMA_PREFIX + "RequestV2")
                request["generated_tokens"][0] = 9856
                (root / "results.jsonl").write_bytes(b"".join(FIXTURE.encoded(r) for r in wrong_tokens))
                with self.assertRaises(ValueError):
                    SUMMARY.load_timed_run(run, expected)
                (root / "results.jsonl").write_bytes(files["results.jsonl"])
                (root / "host-timing.json").write_bytes(files["host-timing.json"] + b" ")
                with self.assertRaisesRegex(ValueError, "sidecar drifted"):
                    SUMMARY.load_timed_run(run, expected)
                (root / "host-timing.json").write_bytes(files["host-timing.json"])
                records[0]["controller_sha256"] = "f" * 64
                (root / "results.jsonl").write_bytes(b"".join(FIXTURE.encoded(r) for r in records))
                with self.assertRaises(ValueError):
                    SUMMARY.load_timed_run(run, expected)


if __name__ == "__main__":
    unittest.main()
