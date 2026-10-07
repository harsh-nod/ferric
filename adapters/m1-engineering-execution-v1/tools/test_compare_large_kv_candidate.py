"""Synthetic capacity accounting tests; not GPU or serving evidence."""

import copy
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

import compare_large_kv_candidate as large
import test_compare_competitiveness_candidate as fixtures


def fixture(root, pages=16384):
    records, expected = fixtures.fixture(root, budget=16, chunk=16)
    expected.update(schema=large.SCHEMA, candidate=large.CANDIDATE, physical_pages=pages,
                    kv_pool_profile=large.CANDIDATE, kv_pool_max_physical_pages=16384,
                    kv_pool_payload_bytes=pages * large.BYTES_PER_PAGE,
                    kv_pool_artifact=dict(fixtures.fixtures.PEER_PINS))
    records[0].update({key: copy.deepcopy(expected[key]) for key in large.EXTRA})
    for item in records:
        if "free_pages" in item:
            item["free_pages"] = pages - item["retained_pages"]
    return records, expected


class LargeKvCandidateTests(unittest.TestCase):
    def compare(self, root, expected):
        with mock.patch.object(large.CHECK, "REFERENCE_SHA256", expected["reference_sha256"]):
            return large.compare(root, root / "workload.json", root / "reference.json", expected)

    def test_capacity_translation_preserves_inputs_and_reference(self):
        for pages in (4, 512, 513, 8192, 8704, 16384):
            with self.subTest(pages=pages), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected = fixture(root, pages)
                raw = fixtures.write(root, records)
                prior = copy.deepcopy(expected)
                report = self.compare(root, expected)
                self.assertTrue(report["all_reference_tokens_and_bytes_match"])
                self.assertFalse(report["qualification"])
                self.assertEqual(report["actual_physical_pages"], pages)
                self.assertEqual(report["metrics"]["output_tokens"], 8)
                self.assertEqual(report["input_sha256"]["results.jsonl"], large.CHECK.sha256(raw))
                self.assertEqual((root / "results.jsonl").read_bytes(), raw)
                self.assertEqual(expected, prior)

    def test_invalid_expectations_reject(self):
        with tempfile.TemporaryDirectory() as directory:
            _, expected = fixture(Path(directory))
            for mutate in (
                lambda e: e.update(physical_pages=16385),
                lambda e: e.update(physical_pages=True),
                lambda e: e.update(kv_pool_payload_bytes=36 * 1024**3 + 1),
                lambda e: e.update(kv_pool_max_physical_pages=8192),
                lambda e: e.update(kv_pool_profile="legacy"),
                lambda e: e.update(world=8),
                lambda e: e.update(unreviewed_extension=True),
                lambda e: e["kv_pool_artifact"].update(unreviewed_extension=True),
                lambda e: e["kv_pool_artifact"].update(artifact_hsaco_id="bad"),
                lambda e: e["performance_profile"].update(attention="wave"),
                lambda e: e["performance_profile"].update(runtime_profiling=True),
                lambda e: e["performance_profile"].update(dispatch_sequences=True),
            ):
                changed = copy.deepcopy(expected)
                mutate(changed)
                with self.assertRaises((ValueError, KeyError, TypeError)):
                    large.expectation(changed)

    def test_normalization_cannot_hide_accounting_identity_or_token_errors(self):
        for mutate in (
            lambda r: r[0].update(physical_pages=512),
            lambda r: r[0].pop("kv_pool_profile"),
            lambda r: r[0].update(kv_pool_payload_bytes=0),
            lambda r: r[0]["kv_pool_artifact"].update(artifact_hsaco_id="f" * 64),
            lambda r: r[0].update(unreviewed_extension=True),
            lambda r: next(x for x in r if "free_pages" in x).update(free_pages=16384),
            lambda r: next(x for x in r if "free_pages" in x).update(free_pages=16382, retained_pages=2),
            lambda r: next(x for x in r if "free_pages" in x).update(free_pages=16383.0),
            lambda r: next(x for x in r if x.get("outputs"))["outputs"][0].update(token=9856),
        ):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                records, expected = fixture(root)
                mutate(records)
                fixtures.write(root, records)
                with self.assertRaises((ValueError, KeyError, TypeError)):
                    self.compare(root, expected)

    def test_cli_checks_frozen_sources_before_inputs(self):
        tools = Path(large.__file__).parent
        sources = ("compare_large_kv_candidate.py", "compare_competitiveness_candidate.py",
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
