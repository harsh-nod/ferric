"""Synthetic acceptance and rejection tests, not native evidence."""

import copy
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

import compare_wave_head32_candidate as wave
import test_compare_competitiveness_candidate as fixtures


def fixture(root, budget=32):
    records, expected = fixtures.fixture(root, budget=budget, chunk=budget)
    expected.update(schema=wave.SCHEMA, candidate=wave.CANDIDATE)
    expected["performance_profile"]["attention"] = "wave"
    records[0]["performance_profile"]["attention"] = "wave"
    return records, expected


class WaveCandidateTests(unittest.TestCase):
    def compare(self, root, expected):
        with mock.patch.object(wave.CHECK, "REFERENCE_SHA256", expected["reference_sha256"]):
            return wave.compare(root, root / "workload.json", root / "reference.json", expected)

    def test_full_reference_preserves_raw_wave_policy_and_inputs(self):
        for budget in (16, 32):
            with self.subTest(budget=budget), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected = fixture(root, budget)
                raw = fixtures.write(root, records)
                prior = copy.deepcopy(expected)
                report = self.compare(root, expected)
                self.assertTrue(report["passed"])
                self.assertTrue(report["all_reference_tokens_and_bytes_match"])
                self.assertFalse(report["qualification"])
                self.assertEqual(report["metrics"]["output_tokens"], 8)
                self.assertEqual(report["expected_candidate"]["performance_profile"]["attention"], "wave")
                self.assertEqual(report["input_sha256"]["results.jsonl"], wave.CHECK.sha256(raw))
                self.assertEqual((root / "results.jsonl").read_bytes(), raw)
                self.assertEqual(expected, prior)

    def test_unsupported_expectations_reject(self):
        with tempfile.TemporaryDirectory() as directory:
            _, expected = fixture(Path(directory))
            for mutate in (
                lambda e: e.update(schema=wave.base.SCHEMA),
                lambda e: e.update(candidate="head32-v8"),
                lambda e: e.update(world=2),
                lambda e: e.update(head_precision="fp32-v7"),
                lambda e: e.update(batch_tokens=33),
                lambda e: e.update(large_kv_profile="v9"),
                lambda e: e["performance_profile"].update(attention="baseline"),
                lambda e: e["performance_profile"].update(projection="auto"),
                lambda e: e["performance_profile"].update(dispatch_sequences=True),
                lambda e: e["performance_profile"].update(runtime_profiling=True),
            ):
                changed = copy.deepcopy(expected)
                mutate(changed)
                with self.assertRaises((ValueError, KeyError, TypeError)):
                    wave.expectation(changed)

    def test_raw_policy_identity_workspace_and_tokens_cannot_be_normalized_away(self):
        mutations = (
            lambda r: r[0]["performance_profile"].update(attention="baseline"),
            lambda r: r[0]["performance_profile"].update(runtime_profiling=True),
            lambda r: r[0].update(controller_sha256="f" * 64),
            lambda r: r[0].update(fp32_head_workspace_bytes=9723904),
            lambda r: r[0].update(large_kv_profile="v9"),
            lambda r: r[0]["fp32_head_artifact"].update(artifact_hsaco_id="f" * 64),
            lambda r: next(item for item in r if item.get("outputs"))["outputs"][0].update(token=9856),
        )
        for index, mutate in enumerate(mutations):
            with self.subTest(index=index), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected = fixture(root)
                mutate(records)
                fixtures.write(root, records)
                with self.assertRaises((ValueError, KeyError)):
                    self.compare(root, expected)

    def test_failed_status_nonidle_and_truncated_trace_reject(self):
        for bad in ("status", "gpu-after.json", "results.jsonl"):
            with self.subTest(bad=bad), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected = fixture(root)
                raw = fixtures.write(root, records)
                if bad == "status":
                    (root / bad).write_bytes(b"1\n")
                elif bad == "results.jsonl":
                    (root / bad).write_bytes(raw[:-1])
                else:
                    snapshot = fixtures.fixtures.snapshots()
                    snapshot["card0"]["GPU use (%)"] = "1"
                    (root / bad).write_bytes(fixtures.fixtures.encoded(snapshot))
                with self.assertRaises(ValueError):
                    self.compare(root, expected)

    def test_frozen_sources_checked_before_cli_inputs(self):
        tools = Path(wave.__file__).parent
        sources = ("compare_wave_head32_candidate.py", "compare_competitiveness_candidate.py",
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
