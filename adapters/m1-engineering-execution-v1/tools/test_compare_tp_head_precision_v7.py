"""Synthetic v7 wrapper tests; no model/GPU/performance evidence."""

import copy
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import compare_tp_head_precision_v7 as V7
import test_compare_tp_batch as F


def fixture(root, head="fp32-v7", projection="baseline"):
    profile = dict(F.PROFILE, projection=projection, runtime_operational=True)
    rows = F.extended_fixture(1, True, False, V7.CHECK.LEGACY_COLLECTIVE, profile)
    pins = dict(F.PEER_PINS)
    rows[0].update(head_precision=head, fp32_head_artifact=pins,
                   fp32_head_workspace_bytes=V7.PROFILES[head])
    reference, workload = F.encoded(F.synthetic_reference()), F.encoded(V7.CHECK.expected_workload())
    expected = {key: rows[0][key] for key in V7.CHECK.IDENTITY_FIELDS}
    expected.update(schema=V7.EXPECT_SCHEMA, workload_sha256=V7.CHECK.sha256(workload),
                    reference_sha256=V7.CHECK.sha256(reference), physical_gpu_ids=F.GPU_IDS,
                    prefix_cache=True, output_head_pruning=False, performance_profile=profile,
                    collective=None, head_precision=head, fp32_head_artifact=dict(pins))
    for name, data in {"status": b"0\n", "gpu-before.json": F.encoded(F.snapshots()),
                       "gpu-after.json": F.encoded(F.snapshots()), "reference.json": reference,
                       "workload.json": workload}.items():
        (root / name).write_bytes(data)
    return rows, expected


def write_trace(root, rows):
    data = b"".join(F.encoded(row) for row in rows)
    (root / "results.jsonl").write_bytes(data)
    return data


class HeadPrecisionTests(unittest.TestCase):
    def compare(self, root, expected):
        with mock.patch.object(V7.CHECK, "REFERENCE_SHA256", expected["reference_sha256"]):
            return V7.compare(root, root / "workload.json", root / "reference.json", expected)

    def test_explicit_control_and_both_candidates_have_distinct_schema_and_unchanged_raw(self):
        for head, projection in (("bf16-v7-control", "baseline"), ("fp32-v7", "baseline"), ("fp32-v7", "mfma")):
            with self.subTest(head=head, projection=projection), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                rows, expected = fixture(root, head, projection)
                raw = write_trace(root, rows)
                report = self.compare(root, expected)
                self.assertEqual(report["schema"], V7.SCHEMA)
                self.assertTrue(report["passed"])
                self.assertTrue(report["all_reference_tokens_and_bytes_match"])
                self.assertEqual(report["head_precision"], head)
                self.assertEqual(report["fp32_head_workspace_bytes"], V7.PROFILES[head])
                self.assertEqual(report["generated_tokens"], 8)
                self.assertEqual(report["rank_dispatch_counts"], [2720])
                self.assertEqual(report["input_sha256"]["results.jsonl"], V7.CHECK.sha256(raw))
                self.assertEqual(raw, (root / "results.jsonl").read_bytes())
                with self.assertRaises(ValueError):
                    V7.CHECK.validate_records(rows, F.GPU_IDS, 1, F.PINS, True, False, None, expected["performance_profile"])

    def test_missing_wrong_extra_profile_pins_extents_and_diagnostic_fields_reject(self):
        mutations = (
            lambda rows: rows[0].pop("head_precision"),
            lambda rows: rows[0].update(head_precision="bf16-v7-control"),
            lambda rows: rows[0].update(fp32_head_workspace_bytes=0),
            lambda rows: rows[0].update(fp32_head_workspace_bytes=9723904.0),
            lambda rows: rows[0]["fp32_head_artifact"].pop("artifact_manifest_id"),
            lambda rows: rows[0]["fp32_head_artifact"].update(artifact_handoff_id="f" * 64),
            lambda rows: rows[0]["fp32_head_artifact"].update(extra="a" * 64),
            lambda rows: rows[0].update(tensor_parallel=2),
            lambda rows: rows[0].update(batch_tokens=32),
            lambda rows: rows[0].update(kernel_profile="v5-mfma32", kernel_row_capacity=32),
            lambda rows: rows[0].update(numerical_capture={}),
            lambda rows: rows[0].update(replica_benchmark={}),
            lambda rows: rows[-1].update(numerical_capture={}),
            lambda rows: rows[-1].update(all_workers_exited=False),
            lambda rows: rows[-1].update(fp32_head_workspace_bytes=9723904),
        )
        for index, mutate in enumerate(mutations):
            with self.subTest(mutation=index), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                rows, expected = fixture(root)
                mutate(rows)
                write_trace(root, rows)
                with self.assertRaises((ValueError, KeyError)):
                    self.compare(root, expected)

    def test_failed_reference_exit_idle_or_external_identity_never_returns_metrics(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            rows, expected = fixture(root)
            next(value for value in rows if value.get("outputs"))["outputs"][0]["token"] = 9856
            write_trace(root, rows)
            with self.assertRaises(ValueError):
                self.compare(root, expected)
        for filename, data in (("status", b"1\n"), ("gpu-after.json", b"{}\n"), ("results.jsonl", b"{}")):
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                rows, expected = fixture(root)
                write_trace(root, rows)
                (root / filename).write_bytes(data)
                with self.assertRaises(ValueError):
                    self.compare(root, expected)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            rows, expected = fixture(root)
            write_trace(root, rows)
            for field in V7.CHECK.IDENTITY_FIELDS | {"workload_sha256", "reference_sha256"}:
                changed = copy.deepcopy(expected)
                changed[field] = "f" * 64
                with self.subTest(field=field), self.assertRaises(ValueError):
                    self.compare(root, changed)

    def test_unsupported_explicit_expectations_cannot_expand_scope(self):
        with tempfile.TemporaryDirectory() as temporary:
            rows, expected = fixture(Path(temporary))
            for mutate in (
                    lambda item: item.update(head_precision="bf16"),
                    lambda item: item["performance_profile"].update(projection="wave"),
                    lambda item: item["performance_profile"].update(projection="auto"),
                    lambda item: item["performance_profile"].update(attention="wave"),
                    lambda item: item["performance_profile"].update(dispatch_sequences=True),
                    lambda item: item.update(collective="device-peer-concurrent-round-v1"),
                    lambda item: item.update(fp32_head_artifact={}),
                    lambda item: item.update(physical_gpu_ids=[100] * 8)):
                changed = copy.deepcopy(expected)
                mutate(changed)
                with self.assertRaises(ValueError):
                    V7.expectation(changed)

    def test_cli_pins_both_sources_and_creates_only_an_exclusive_distinct_report(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            rows, expected = fixture(root)
            write_trace(root, rows)
            encoded = F.encoded(expected)
            (root / "expect.json").write_bytes(encoded)
            output = root / "v7-comparison.json"
            argv = ["compare_tp_head_precision_v7.py", "--run-dir", str(root),
                    "--workload", str(root / "workload.json"), "--reference", str(root / "reference.json"),
                    "--expect", str(root / "expect.json"), "--output", str(output)]
            with mock.patch("sys.argv", argv), \
                    mock.patch.object(V7.CHECK, "REFERENCE_SHA256", expected["reference_sha256"]):
                V7.main()
                report = V7.CHECK.json_value(output.read_bytes())
                self.assertEqual(report["schema"], V7.SCHEMA)
                self.assertEqual(report["semantic_checker_sha256"], V7.SEMANTIC_SHA256)
                self.assertEqual(report["comparator_sha256"], V7.CHECK.sha256(Path(V7.__file__).read_bytes()))
                self.assertEqual(report["expectation_sha256"], V7.CHECK.sha256(encoded))
                original = output.read_bytes()
                with self.assertRaises(FileExistsError):
                    V7.main()
                self.assertEqual(original, output.read_bytes())


if __name__ == "__main__":
    unittest.main()
