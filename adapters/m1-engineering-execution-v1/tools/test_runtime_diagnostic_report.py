"""CPU-only synthetic diagnostics; no GPU or performance qualification."""

import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

import runtime_diagnostic_report as D
import test_compare_competitiveness_candidate as C
import test_compare_tp_batch as F


def fixture(root, budget=32, chunk=32):
    records, expected = C.fixture(root, budget=budget, chunk=chunk)
    records[0]["performance_profile"]["runtime_profiling"] = True
    records[0].update(runtime_diagnostic_status=D.RUNTIME_STATUS, numerical_status=D.NUMERICAL_STATUS)
    dispatches = records[-1]["rank_dispatch_counts"][0]
    before = dict.fromkeys(D.COUNTERS, 100)
    before["dispatches"] = 0
    after = {key: value + 10000 for key, value in before.items()}
    after["dispatches"] = dispatches
    envelopes = []
    for ordinal, (phase, counters) in enumerate((("before_workload", before), ("after_workload", after))):
        rank = {"schema": D.SNAPSHOT_SCHEMA, "authority": "none", "performance_qualified": False,
                "scope": D.COUNTER_SCOPE, "process_id": records[0]["worker_pids"][0],
                "device_unique_id": records[0]["device_unique_ids"][0], "rank": 0,
                "ordinal": ordinal, "counters": counters}
        envelopes.append({"schema": D.ENVELOPE_SCHEMA, "authority": "none", "phase": phase,
                          "measurement": D.MEASUREMENT, "performance_qualified": False, "ranks": [rank]})
    return records, expected, envelopes


def write(root, records, envelopes):
    C.write(root, records)
    (root / "progress.log").write_bytes(b"model setup complete\n" + F.encoded(envelopes[0]) +
                                        b"batch diagnostics\n" + b"".join(F.encoded(e) for e in envelopes[1:]))


class RuntimeDiagnosticTests(unittest.TestCase):
    def diagnose(self, root, expected):
        with mock.patch.object(D.CHECK, "REFERENCE_SHA256", expected["reference_sha256"]):
            return D.diagnose(root, root / "workload.json", root / "reference.json", expected)

    def test_valid_tp1_v8_boundaries_report_only_cumulative_and_delta_counters(self):
        for budget, chunk in ((16, 16), (32, 16), (32, 17), (32, 32)):
            with self.subTest(budget=budget, chunk=chunk), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected, envelopes = fixture(root, budget, chunk)
                write(root, records, envelopes)
                untouched = {p.name: p.read_bytes() for p in root.iterdir()}
                with mock.patch.object(D.candidate, "compare", side_effect=AssertionError("ordinary report")), \
                        mock.patch.object(D.candidate.ledger, "extract_metrics", side_effect=AssertionError("metrics")):
                    result = self.diagnose(root, expected)
                self.assertEqual(result["schema"], "FerricRuntimeDiagnosticReportV1")
                self.assertTrue(result["fixed_reference_passed"])
                self.assertFalse(result["performance_qualified"])
                self.assertFalse(result["qualification"])
                self.assertTrue(result["diagnostic_profile"]["runtime_profiling"])
                self.assertFalse(result["expected_candidate"]["performance_profile"]["runtime_profiling"])
                self.assertEqual(result["snapshot_ordinals"], [0, 1])
                self.assertEqual(result["stderr_snapshot_lines"], [2, 4])
                self.assertEqual(result["delta_counters"]["dispatches"], records[-1]["rank_dispatch_counts"][0])
                self.assertEqual(result["delta_counters"]["command_ns"], 10000)
                self.assertEqual(result["cumulative_counters"]["before_workload"]["commands"], 100)
                self.assertEqual(result["counts"]["generated_tokens"], 8)
                self.assertEqual(result["input_sha256"]["progress.log"], D.CHECK.sha256(untouched["progress.log"]))
                self.assertEqual(untouched, {p.name: p.read_bytes() for p in root.iterdir()})
                self.assertFalse(set(result) & {"metrics", "requests", "setup_seconds", "whole_seconds", "ttft_ns", "tpot_ns", "batch_durations_ns"})

    def test_every_snapshot_binding_and_schema_mutation_rejects(self):
        mutations = [
            lambda e: e[0].update(phase="after_workload"),
            lambda e: e.reverse(),
            lambda e: e[1].update(extra=True),
            lambda e: e[1].update(schema="unknown"),
            lambda e: e[1].update(measurement="GPU timestamps"),
            lambda e: e[1].update(authority="protected"),
            lambda e: e[1].update(performance_qualified=True),
            lambda e: e[1]["ranks"].append(copy.deepcopy(e[1]["ranks"][0])),
            lambda e: e[1].update(ranks=[]),
        ]
        for key, value in (("process_id", 17), ("device_unique_id", 17), ("rank", 1),
                           ("rank", False), ("ordinal", 0), ("ordinal", 1.0),
                           ("scope", "GPU duration"), ("authority", "protected"),
                           ("schema", "unknown"), ("performance_qualified", 0), ("extra", 1)):
            mutations.append(lambda e, key=key, value=value: e[1]["ranks"][0].update({key: value}))
        for index, mutate in enumerate(mutations):
            with self.subTest(index=index), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected, envelopes = fixture(root)
                mutate(envelopes)
                write(root, records, envelopes)
                with self.assertRaises(ValueError):
                    self.diagnose(root, expected)

    def test_counter_schema_unsigned_monotonicity_and_dispatch_consistency_reject(self):
        mutations = [
            lambda a, b: b.pop("read_ns"),
            lambda a, b: b.update(extra=0),
            lambda a, b: b.update(read_ns=99),
            lambda a, b: b.update(read_ns=-1),
            lambda a, b: b.update(read_ns=1 << 64),
            lambda a, b: b.update(read_ns=100.0),
            lambda a, b: b.update(read_ns=True),
            lambda a, b: b.update(dispatches=b["dispatches"] - 1),
            lambda a, b: a.update(dispatches=1),
            lambda a, b: b.update(commands=a["commands"] + b["dispatches"]),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(index=index), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected, envelopes = fixture(root)
                mutate(envelopes[0]["ranks"][0]["counters"], envelopes[1]["ranks"][0]["counters"])
                write(root, records, envelopes)
                with self.assertRaises(ValueError):
                    self.diagnose(root, expected)

    def test_diagnostic_setup_v8_identity_and_full_reference_mutations_reject(self):
        mutations = [
            lambda r: r[0].pop("runtime_diagnostic_status"),
            lambda r: r[0].update(runtime_diagnostic_status="unqualified"),
            lambda r: r[0].update(numerical_status=D.ORDINARY_STATUS),
            lambda r: r[0]["performance_profile"].update(runtime_profiling=False),
            lambda r: r[0]["performance_profile"].update(runtime_cache_admission=True),
            lambda r: r[0].update(head_precision="fp32-v7"),
            lambda r: r[0].update(fp32_head_workspace_bytes=0),
            lambda r: r[0]["fp32_head_artifact"].update(artifact_hsaco_id="f" * 64),
            lambda r: r[0].update(kernel_row_capacity=16),
            lambda r: r[0].update(numerical_capture={}),
            lambda r: r[0].update(extra=True),
            lambda r: r[-1].update(extra=True),
            lambda r: r[-1].update(all_workers_exited=False),
            lambda r: r[-1].update(rank_dispatch_counts=[1]),
            lambda r: next(v for v in r if v.get("outputs"))["outputs"][0].update(token=9856),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(index=index), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected, envelopes = fixture(root)
                mutate(records)
                write(root, records, envelopes)
                with self.assertRaises(ValueError):
                    self.diagnose(root, expected)

    def test_missing_extra_truncated_or_duplicate_json_snapshots_reject(self):
        for variant in ("one", "three", "truncated", "malformed", "duplicate", "invalid_utf8"):
            with self.subTest(variant=variant), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected, envelopes = fixture(root)
                write(root, records, envelopes)
                data = (root / "progress.log").read_bytes()
                if variant == "one":
                    data = F.encoded(envelopes[0])
                elif variant == "three":
                    data += F.encoded(envelopes[1])
                elif variant == "truncated":
                    data = data[:-1]
                elif variant == "malformed":
                    data += b"invalid FerricRuntimeDiagnosticSnapshotV1\n"
                elif variant == "duplicate":
                    data = data.replace(b'"phase":', b'"phase":"before_workload","phase":', 1)
                else:
                    data += b"\xff\n"
                (root / "progress.log").write_bytes(data)
                with self.assertRaises(ValueError):
                    self.diagnose(root, expected)

    def test_failed_exit_idle_or_external_expectations_cannot_broaden_scope(self):
        for variant in ("status", "idle", "world", "profile", "input", "worker", "extra"):
            with self.subTest(variant=variant), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected, envelopes = fixture(root)
                write(root, records, envelopes)
                if variant == "status":
                    (root / "status").write_bytes(b"1\n")
                elif variant == "idle":
                    cards = F.snapshots()
                    cards["card0"]["GPU use (%)"] = "1"
                    (root / "gpu-after.json").write_bytes(F.encoded(cards))
                elif variant == "world":
                    expected["world"] = 2
                elif variant == "profile":
                    expected["performance_profile"]["runtime_profiling"] = True
                elif variant == "input":
                    expected["workload_sha256"] = "f" * 64
                elif variant == "worker":
                    expected["worker_sha256"] = "f" * 64
                else:
                    expected["unexpected"] = True
                with self.assertRaises(ValueError):
                    self.diagnose(root, expected)

    def test_cli_writes_only_explicit_diagnostic_output_and_never_overwrites(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            records, expected, envelopes = fixture(root)
            write(root, records, envelopes)
            (root / "expectation.json").write_bytes(F.encoded(expected))
            args = ["--run-dir", str(root), "--workload", str(root / "workload.json"),
                    "--reference", str(root / "reference.json"), "--expect", str(root / "expectation.json"),
                    "--output", str(root / "diagnostic.json")]
            with mock.patch.object(D.CHECK, "REFERENCE_SHA256", expected["reference_sha256"]):
                self.assertEqual(D.main(args), 0)
                result = json.loads((root / "diagnostic.json").read_bytes())
                self.assertEqual(result["expectation_sha256"], D.CHECK.sha256(F.encoded(expected)))
                original = (root / "diagnostic.json").read_bytes()
                with self.assertRaises(SystemExit):
                    D.main(args)
                self.assertEqual((root / "diagnostic.json").read_bytes(), original)
                records[-1]["all_workers_exited"] = False
                write(root, records, envelopes)
                args[-1] = str(root / "rejected.json")
                with self.assertRaises(SystemExit):
                    D.main(args)
                self.assertFalse((root / "rejected.json").exists())

    def test_cli_rejects_any_frozen_dependency_source_drift_before_reading_inputs(self):
        tools = Path(D.__file__).parent
        names = ("runtime_diagnostic_report.py", "compare_competitiveness_candidate.py",
                 "compare_tp_batch.py", "performance_ledger.py")
        for altered in names[1:]:
            with self.subTest(altered=altered), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                for name in names:
                    (root / name).write_bytes((tools / name).read_bytes() + (b"\n# drift\n" if name == altered else b""))
                args = [sys.executable, "-B", str(root / names[0])]
                for name in ("run-dir", "workload", "reference", "expect", "output"):
                    args.extend(["--" + name, str(root / name)])
                result = subprocess.run(args, capture_output=True, text=True, timeout=10)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("frozen", result.stderr)
                self.assertFalse((root / "output").exists())


if __name__ == "__main__":
    unittest.main()
