"""Synthetic ordered-batch contract tests, not native or timing evidence."""

import copy
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

import compare_ordered_batch_candidate as ordered
import test_compare_competitiveness_candidate as fixtures


def fixture(root, budget=32, chunk=32, cache=True):
    _, expected = fixtures.fixture(root, budget=budget, chunk=chunk)
    expected.update(schema=ordered.SCHEMA, candidate=ordered.CANDIDATE,
                    runtime_ordered_batches=True, output_head_pruning=True,
                    collective="device-tp1-v3", prefix_cache=cache)
    records = fixtures.fixtures.extended_fixture(
        1, cache, True, "device-tp1-v3", expected["performance_profile"],
        "v5-mfma32", budget, chunk)
    records[0].update(
        runtime_ordered_batches=True, head_precision="fp32-v8",
        fp32_head_artifact=copy.deepcopy(expected["fp32_head_artifact"]),
        fp32_head_workspace_bytes=ordered.base.HEAD_WORKSPACE["fp32-v8"])
    return records, expected


class OrderedCandidateTests(unittest.TestCase):
    def compare(self, root, expected):
        with mock.patch.object(ordered.CHECK, "REFERENCE_SHA256", expected["reference_sha256"]):
            return ordered.compare(root, root / "workload.json", root / "reference.json", expected)

    def test_supported_schedules_preserve_exact_raw_reference_and_packet_counts(self):
        for budget, chunk in ((16, 16), (17, 17), (32, 16), (32, 17), (32, 32)):
            for cache in (False, True):
                with self.subTest(budget=budget, chunk=chunk, cache=cache), tempfile.TemporaryDirectory() as directory:
                    root = Path(directory)
                    records, expected = fixture(root, budget, chunk, cache)
                    raw = fixtures.write(root, records)
                    before = copy.deepcopy(expected)
                    report = self.compare(root, expected)
                    self.assertTrue(report["passed"])
                    self.assertTrue(report["all_reference_tokens_and_bytes_match"])
                    self.assertFalse(report["qualification"])
                    self.assertEqual(report["metrics"]["output_tokens"], 8)
                    self.assertEqual(report["expected_candidate"]["performance_profile"],
                                     records[0]["performance_profile"])
                    self.assertIs(report["expected_candidate"]["performance_profile"]["dispatch_sequences"], False)
                    self.assertEqual(report["rank_dispatch_counts"], records[-1]["rank_dispatch_counts"])
                    self.assertEqual(report["input_sha256"]["results.jsonl"], ordered.CHECK.sha256(raw))
                    self.assertEqual((root / "results.jsonl").read_bytes(), raw)
                    self.assertEqual(expected, before)
                    for record in records:
                        if "output_head_rows" in record:
                            self.assertEqual(record["rank_dispatch_counts"],
                                             [616 if record["output_head_rows"] else 613])

    def test_expectations_are_exact_and_cannot_broaden_scope(self):
        with tempfile.TemporaryDirectory() as directory:
            _, expected = fixture(Path(directory))
            changes = (
                lambda e: e.update(schema=ordered.base.SCHEMA),
                lambda e: e.update(candidate="head32-v8"),
                lambda e: e.pop("runtime_ordered_batches"),
                lambda e: e.update(runtime_ordered_batches=False),
                lambda e: e.update(runtime_ordered_batches=1),
                lambda e: e.update(runtime_ordered_batches="true"),
                lambda e: e.update(world=2),
                lambda e: e.update(world=1.0),
                lambda e: e.update(wide_kernel_profile="v5-wave32"),
                lambda e: e.update(head_precision="bf16-v8-control"),
                lambda e: e.update(head_precision="fp32-v7"),
                lambda e: e.update(collective="host-staged-reuse-v3"),
                lambda e: e.update(output_head_pruning=False),
                lambda e: e.update(batch_tokens=33),
                lambda e: e.update(kv_pool_profile="large-kv-v9"),
                lambda e: e.update(peer_shared_full_currentness=True),
                lambda e: e.update(replica_benchmark={}),
                lambda e: e.update(numerical_capture={}),
                lambda e: e["performance_profile"].update(projection="baseline"),
                lambda e: e["performance_profile"].update(attention="wave"),
                lambda e: e["performance_profile"].update(dispatch_sequences=True),
                lambda e: e["performance_profile"].update(runtime_profiling=True),
                lambda e: e["performance_profile"].update(runtime_ordered_batches=True),
            )
            for index, mutate in enumerate(changes):
                with self.subTest(index=index):
                    changed = copy.deepcopy(expected)
                    mutate(changed)
                    with self.assertRaises((ValueError, KeyError, TypeError)):
                        ordered.expectation(changed)

    def test_raw_extensions_profiles_and_identity_cannot_be_normalized_away(self):
        changes = (
            lambda r: r[0].pop("runtime_ordered_batches"),
            lambda r: r[0].update(runtime_ordered_batches=False),
            lambda r: r[0].update(runtime_ordered_batches=1),
            lambda r: r[0].update(runtime_ordered_batches=1.0),
            lambda r: r[0].update(runtime_ordered_batches="true"),
            lambda r: r[-1].update(runtime_ordered_batches=True),
            lambda r: r[0]["performance_profile"].update(dispatch_sequences=True),
            lambda r: r[0]["performance_profile"].update(attention="wave"),
            lambda r: r[0]["performance_profile"].update(runtime_profiling=True),
            lambda r: r[0]["performance_profile"].update(runtime_cache_admission=True),
            lambda r: r[0].update(output_head_pruning=False),
            lambda r: r[0].update(collective="host-staged-reuse-v3"),
            lambda r: r[0].update(head_precision="bf16-v8-control"),
            lambda r: r[0].update(fp32_head_workspace_bytes=9723904),
            lambda r: r[0].update(fp32_head_workspace_bytes=19447808.0),
            lambda r: r[0].update(controller_sha256="f" * 64),
            lambda r: r[0].update(worker_sha256="f" * 64),
            lambda r: r[0].update(running_worker_sha256=["f" * 64]),
            lambda r: r[0].update(artifact_manifest_id="f" * 64),
            lambda r: r[0]["fp32_head_artifact"].update(artifact_handoff_id="f" * 64),
        )
        for index, mutate in enumerate(changes):
            with self.subTest(index=index), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected = fixture(root)
                mutate(records)
                fixtures.write(root, records)
                with self.assertRaises((ValueError, KeyError, TypeError)):
                    self.compare(root, expected)

    def test_unrelated_extensions_remain_rejected(self):
        for field in ("kv_pool_profile", "kv_pool_artifact", "peer_artifact",
                      "peer_shared_full_currentness", "replica_benchmark", "numerical_capture",
                      "runtime_diagnostic_status", "live_protocol"):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected = fixture(root)
                records[0][field] = {}
                fixtures.write(root, records)
                with self.assertRaises((ValueError, KeyError, TypeError)):
                    self.compare(root, expected)

    def test_tokens_bytes_rows_counts_and_close_are_not_relaxed(self):
        def batch(records):
            return next(record for record in records if record.get("outputs"))

        def request(records):
            return next(record for record in records if record.get("generated_utf8_bytes"))

        changes = (
            lambda r: batch(r)["outputs"][0].update(token=9856),
            lambda r: batch(r)["rows"][0].update(position=999),
            lambda r: batch(r).update(rank_dispatch_counts=[72]),
            lambda r: batch(r).update(output_head_rows=0),
            lambda r: request(r)["generated_utf8_bytes"].__setitem__(0, 33),
            lambda r: r[-1].update(rank_dispatch_counts=[72]),
            lambda r: r[-1].update(all_workers_exited=False),
            lambda r: r[-1].update(all_workers_exited=1),
            lambda r: r[-1].update(worker_pids=[9999]),
            lambda r: r.pop(-1),
        )
        for index, mutate in enumerate(changes):
            with self.subTest(index=index), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected = fixture(root)
                mutate(records)
                fixtures.write(root, records)
                with self.assertRaises((ValueError, KeyError, TypeError)):
                    self.compare(root, expected)

    def test_failed_status_nonidle_truncation_and_input_hash_reject(self):
        for bad in ("status", "gpu-after.json", "results.jsonl", "workload", "reference"):
            with self.subTest(bad=bad), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected = fixture(root)
                raw = fixtures.write(root, records)
                if bad == "status":
                    (root / bad).write_bytes(b"1\n")
                elif bad == "results.jsonl":
                    (root / bad).write_bytes(raw[:-1])
                elif bad == "gpu-after.json":
                    snapshot = fixtures.fixtures.snapshots()
                    snapshot["card0"]["GPU use (%)"] = "1"
                    (root / bad).write_bytes(fixtures.fixtures.encoded(snapshot))
                else:
                    expected[bad + "_sha256"] = "f" * 64
                with self.assertRaises(ValueError):
                    self.compare(root, expected)

    def test_frozen_checker_rejects_ordered_extension(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            records, expected = fixture(root)
            fixtures.write(root, records)
            shadow = ordered.expectation(expected)
            with mock.patch.object(ordered.CHECK, "REFERENCE_SHA256", expected["reference_sha256"]):
                with self.assertRaises(ValueError):
                    ordered.base.compare(root, root / "workload.json", root / "reference.json", shadow)

    def test_frozen_sources_checked_before_cli_inputs(self):
        tools = Path(ordered.__file__).parent
        sources = ("compare_ordered_batch_candidate.py", "compare_competitiveness_candidate.py",
                   "compare_tp_batch.py", "performance_ledger.py")
        for changed in sources[1:]:
            with self.subTest(changed=changed), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                for name in sources:
                    (root / name).write_bytes((tools / name).read_bytes() +
                                             (b"\n# drift\n" if name == changed else b""))
                command = [sys.executable, "-B", str(root / sources[0])]
                for name in ("run-dir", "workload", "reference", "expect", "output"):
                    command.extend(["--" + name, str(root / name)])
                result = subprocess.run(command, capture_output=True, text=True, timeout=10)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("frozen", result.stderr)
                self.assertFalse((root / "output").exists())


if __name__ == "__main__":
    unittest.main()
