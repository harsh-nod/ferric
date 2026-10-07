"""Synthetic new-profile fixtures only; no native or performance evidence."""

import copy
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest
from unittest import mock

import compare_competitiveness_candidate as candidate
import test_compare_tp_batch as fixtures


def fixture(root, kind="head32-v8", world=1, budget=32, chunk=32):
    if kind == "head32-v8":
        profile = dict(fixtures.PROFILE, projection="mfma", runtime_operational=True)
        records = fixtures.extended_fixture(1, True, False, candidate.check.LEGACY_COLLECTIVE,
                                           profile, "v5-mfma32", budget, chunk)
        records[0].update(head_precision="fp32-v8", fp32_head_artifact=dict(fixtures.PEER_PINS),
                          fp32_head_workspace_bytes=candidate.HEAD_WORKSPACE["fp32-v8"])
        extras = {"head_precision": "fp32-v8", "fp32_head_artifact": dict(fixtures.PEER_PINS),
                  "wide_kernel_profile": "v5-mfma32"}
        collective = None
    else:
        records = fixtures.peer_fixture(world)
        profile = dict(fixtures.PROFILE)
        collective = candidate.check.CONCURRENT_COLLECTIVE
        records[0].update(collective=collective, peer_shared_full_currentness=True)
        extras = {"peer_artifact": dict(fixtures.PEER_PINS), "peer_shared_full_currentness": True}
        budget = chunk = 16
    reference = fixtures.encoded(fixtures.synthetic_reference())
    workload = fixtures.encoded(candidate.check.expected_workload())
    expected = {key: records[0][key] for key in candidate.check.IDENTITY_FIELDS}
    expected.update(schema=candidate.SCHEMA, candidate=kind, world=world,
                    workload_sha256=candidate.check.sha256(workload),
                    reference_sha256=candidate.check.sha256(reference), physical_gpu_ids=fixtures.GPU_IDS,
                    prefix_cache=True, output_head_pruning=False, performance_profile=profile,
                    collective=collective, batch_tokens=budget, prefill_chunk=chunk, **extras)
    for name, data in {"status": b"0\n", "gpu-before.json": fixtures.encoded(fixtures.snapshots()),
                       "gpu-after.json": fixtures.encoded(fixtures.snapshots()),
                       "workload.json": workload, "reference.json": reference}.items():
        (root / name).write_bytes(data)
    return records, expected


def write(root, records):
    raw = b"".join(fixtures.encoded(record) for record in records)
    (root / "results.jsonl").write_bytes(raw)
    return raw


class CandidateTests(unittest.TestCase):
    def compare(self, root, expected):
        with mock.patch.object(candidate.check, "REFERENCE_SHA256", expected["reference_sha256"]):
            return candidate.compare(root, root / "workload.json", root / "reference.json", expected)

    def test_head32_supported_row_policies_keep_raw_trace_and_correct_metrics(self):
        for budget, chunk in ((16, 16), (32, 16), (32, 17), (32, 32)):
            with self.subTest(budget=budget, chunk=chunk), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected = fixture(root, budget=budget, chunk=chunk)
                raw = write(root, records)
                result = self.compare(root, expected)
                self.assertTrue(result["passed"])
                self.assertFalse(result["qualification"])
                self.assertEqual(result["metrics"]["output_tokens"], 8)
                self.assertEqual(result["input_sha256"]["results.jsonl"], candidate.check.sha256(raw))
                self.assertEqual((root / "results.jsonl").read_bytes(), raw)

    def test_shared_tp2_tp8_explicit_profile_preserves_reference(self):
        for world in (2, 8):
            with self.subTest(world=world), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected = fixture(root, "shared-full-currentness-v1", world)
                write(root, records)
                result = self.compare(root, expected)
                self.assertTrue(result["all_reference_tokens_and_bytes_match"])
                self.assertEqual(result["candidate"], "shared-full-currentness-v1")

    def test_bf16_control_has_zero_additional_workspace(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            records, expected = fixture(root)
            expected["head_precision"] = "bf16-v8-control"
            records[0].update(head_precision="bf16-v8-control", fp32_head_workspace_bytes=0)
            write(root, records)
            self.assertTrue(self.compare(root, expected)["passed"])

    def test_missing_or_wrong_extensions_and_diagnostics_reject(self):
        for kind, world, mutations in (
            ("head32-v8", 1, [lambda r: r[0].pop("head_precision"),
                              lambda r: r[0].update(head_precision="fp32-v7"),
                              lambda r: r[0].update(fp32_head_workspace_bytes=9723904),
                              lambda r: r[0].update(fp32_head_workspace_bytes=19447808.0),
                              lambda r: r[0]["fp32_head_artifact"].update(artifact_hsaco_id="f" * 64),
                              lambda r: r[0].update(batch_tokens=16),
                              lambda r: r[0].update(runtime_diagnostic_status="diagnostic")]),
            ("shared-full-currentness-v1", 8, [lambda r: r[0].pop("peer_shared_full_currentness"),
                                               lambda r: r[0].update(peer_shared_full_currentness=False),
                                               lambda r: r[0].update(peer_shared_full_currentness=1),
                                               lambda r: r[0].update(numerical_capture={})]),
        ):
            for index, mutate in enumerate(mutations):
                with self.subTest(kind=kind, index=index), tempfile.TemporaryDirectory() as directory:
                    root = Path(directory)
                    records, expected = fixture(root, kind, world)
                    mutate(records)
                    write(root, records)
                    with self.assertRaises((ValueError, KeyError)):
                        self.compare(root, expected)

    def test_wrong_tokens_failed_exit_and_nonidle_gpu_reject(self):
        for mutation in ("token", "status", "idle"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected = fixture(root)
                if mutation == "token":
                    next(r for r in records if r.get("outputs"))["outputs"][0]["token"] = 9856
                write(root, records)
                if mutation == "status":
                    (root / "status").write_bytes(b"1\n")
                if mutation == "idle":
                    snapshot = fixtures.snapshots()
                    snapshot["card0"]["GPU use (%)"] = "1"
                    (root / "gpu-after.json").write_bytes(fixtures.encoded(snapshot))
                with self.assertRaises(ValueError):
                    self.compare(root, expected)

    def test_expectations_cannot_broaden_scope(self):
        with tempfile.TemporaryDirectory() as directory:
            _, expected = fixture(Path(directory))
            for mutate in (lambda e: e.update(world=2),
                           lambda e: e.update(head_precision="fp32-v7"),
                           lambda e: e["performance_profile"].update(runtime_profiling=True),
                           lambda e: e["performance_profile"].update(attention="wave"),
                           lambda e: e["performance_profile"].update(dispatch_sequences=True),
                           lambda e: e.update(batch_tokens=33),
                           lambda e: e.update(peer_shared_full_currentness=True)):
                changed = copy.deepcopy(expected)
                mutate(changed)
                with self.assertRaises(ValueError):
                    candidate.expectation(changed)

    def test_cli_rejects_checker_or_extractor_pin_drift_before_reading_results(self):
        tools = Path(candidate.__file__).parent
        for changed, error in (("compare_tp_batch.py", "frozen semantic checker source"),
                               ("performance_ledger.py", "frozen metric extractor source")):
            with self.subTest(changed=changed), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                for name in ("compare_competitiveness_candidate.py", "compare_tp_batch.py", "performance_ledger.py"):
                    raw = (tools / name).read_bytes()
                    (root / name).write_bytes(raw + (b"\n# drift\n" if name == changed else b""))
                command = [sys.executable, "-B", str(root / "compare_competitiveness_candidate.py")]
                for name in ("run-dir", "workload", "reference", "expect", "output"):
                    command.extend(["--" + name, str(root / name)])
                result = subprocess.run(command, capture_output=True, text=True, timeout=10)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(error, result.stderr)
                self.assertFalse((root / "output").exists())


if __name__ == "__main__":
    unittest.main()
