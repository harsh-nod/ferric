"""Synthetic matched-v7 timing tests; not native performance evidence."""

import copy
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import summarize_tp_head_precision_v7 as S
import test_compare_tp_head_precision_v7 as F
import test_host_timing_summary as T


def manifest(root):
    result = {"schema": S.SCHEMA, "warmup_policy": "fresh-worker-no-warmup", "variants": [],
              "pairs": [{"baseline": "bf16-control", "candidate": "fp32-baseline"},
                        {"baseline": "fp32-baseline", "candidate": "fp32-mfma"}]}
    for name, (head, projection) in S.PROFILES.items():
        directory = root / name
        directory.mkdir()
        records, expected = F.fixture(directory, head, projection)
        F.write_trace(directory, records)
        expectation_raw = F.F.encoded(expected)
        expectation_hash = S.CHECK.sha256(expectation_raw)
        (directory / "expect.json").write_bytes(expectation_raw)
        with mock.patch.object(S.CHECK, "REFERENCE_SHA256", expected["reference_sha256"]):
            report = S.V7.compare(directory, directory / "workload.json", directory / "reference.json", expected)
        report.update(expectation_sha256=expectation_hash,
                      comparator_sha256=S.SOURCES["compare_tp_head_precision_v7.py"])
        raw = F.F.encoded(report)
        (directory / "comparison.json").write_bytes(raw)
        sidecar = F.F.encoded(T.sidecar(records, expected))
        (directory / "host-timing.json").write_bytes(sidecar)
        result["variants"].append({"name": name, "expectation": str(directory / "expect.json"),
            "expectation_sha256": expectation_hash, "runs": [{"id": name + "-r1", "run_dir": str(directory),
            "workload": str(directory / "workload.json"), "reference": str(directory / "reference.json"),
            "comparison": str(directory / "comparison.json"), "comparison_sha256": S.CHECK.sha256(raw),
            "timing": str(directory / "host-timing.json"), "timing_sha256": S.CHECK.sha256(sidecar)}]})
    return result


class MatchedTimingTests(unittest.TestCase):
    def aggregate(self, value):
        reference_hash = S.CHECK.sha256(F.F.encoded(F.F.synthetic_reference()))
        with mock.patch.object(S.CHECK, "REFERENCE_SHA256", reference_hash):
            return S.aggregate(value)

    def test_three_matched_profiles_keep_two_isolated_pairs_and_request_identity(self):
        with tempfile.TemporaryDirectory() as temporary:
            value = manifest(Path(temporary).resolve())
            report = self.aggregate(value)
            self.assertEqual(report["schema"], "FerricTpHeadPrecisionTimingSummaryV1")
            self.assertEqual(len(report["pairs"]), 2)
            self.assertEqual(report["sidecar_normalization"], "none; original v7 Setup/Closed bind directly")
            for variant in report["variants"]:
                self.assertEqual(variant["repetitions"], 1)
                self.assertAlmostEqual(variant["metrics"]["output_tokens_per_second"]["p50"], 8 / 4.104)
                self.assertIsNone(variant["requests"]["cancel-between-batches"]["metrics"]["tpot_seconds"])
                self.assertEqual(variant["requests"]["arriving-short"]["decode_interval_count"], 2)
                self.assertIn("fp32_head_artifact", variant["runs"][0]["identities"])
                self.assertEqual(variant["runs"][0]["batch_count"], 5)
                self.assertEqual(variant["runs"][0]["physical_token_rows"], 34)
                self.assertEqual(variant["runs"][0]["rank_dispatch_counts"], [2720])
            for pair in report["pairs"]:
                self.assertEqual(pair["metrics"]["output_tokens_per_second"]["p50"]["improvement_percent"], 0)
            self.assertIn("Decode gaps/rep", S.markdown(report))
            self.assertIn("not GPU duration", S.markdown(report))

    def test_sidecar_setup_head_identity_close_and_failure_mutations_reject_even_when_rehashed(self):
        mutations = (
            lambda sidecar: sidecar["setup"].update(head_precision="bf16-v7-control"),
            lambda sidecar: sidecar["setup"]["fp32_head_artifact"].update(artifact_hsaco_id="f" * 64),
            lambda sidecar: sidecar["closed"].update(all_workers_exited=False),
            lambda sidecar: sidecar["records"][-1].update(dispatches=1),
            lambda sidecar: sidecar.update(incomplete=True),
        )
        for index, mutate in enumerate(mutations):
            with self.subTest(mutation=index), tempfile.TemporaryDirectory() as temporary:
                value = manifest(Path(temporary).resolve())
                run = value["variants"][1]["runs"][0]
                sidecar = S.CHECK.json_value(Path(run["timing"]).read_bytes())
                mutate(sidecar)
                raw = F.F.encoded(sidecar)
                Path(run["timing"]).write_bytes(raw)
                run["timing_sha256"] = S.CHECK.sha256(raw)
                with self.assertRaises(ValueError):
                    self.aggregate(value)

    def test_raw_hash_comparison_status_and_reference_failure_reject_before_metrics(self):
        for field in ("raw-hash", "comparison", "status", "tokens"):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temporary:
                value = manifest(Path(temporary).resolve())
                run = value["variants"][1]["runs"][0]
                directory = Path(run["run_dir"])
                if field == "raw-hash":
                    (directory / "results.jsonl").write_bytes(b" " + (directory / "results.jsonl").read_bytes())
                elif field == "comparison":
                    prior = S.CHECK.json_value(Path(run["comparison"]).read_bytes())
                    prior["input_sha256"]["results.jsonl"] = "f" * 64
                    raw = F.F.encoded(prior)
                    Path(run["comparison"]).write_bytes(raw)
                    run["comparison_sha256"] = S.CHECK.sha256(raw)
                elif field == "status":
                    (directory / "status").write_bytes(b"1\n")
                else:
                    records = [S.CHECK.json_value(line) for line in (directory / "results.jsonl").read_bytes().splitlines()]
                    next(row for row in records if row.get("outputs"))["outputs"][0]["token"] = 9856
                    F.write_trace(directory, records)
                with self.assertRaises(ValueError):
                    self.aggregate(value)

    def test_pairs_reject_simultaneous_precision_projection_change_and_unmatched_sources(self):
        with tempfile.TemporaryDirectory() as temporary:
            value = manifest(Path(temporary).resolve())
            value["pairs"] = [{"baseline": "bf16-control", "candidate": "fp32-mfma"}]
            with self.assertRaisesRegex(ValueError, "pair must change only"):
                S.validate_manifest(value)
        mutations = (
            lambda item: item.update(controller_sha256="f" * 64),
            lambda item: item.update(worker_sha256="f" * 64),
            lambda item: item.update(artifact_hsaco_id="f" * 64),
            lambda item: item["fp32_head_artifact"].update(artifact_manifest_id="f" * 64),
            lambda item: item.update(workload_sha256="f" * 64),
            lambda item: item.update(reference_sha256="f" * 64),
            lambda item: item.update(prefix_cache=False),
            lambda item: item["performance_profile"].update(runtime_operational=False),
        )
        for index, mutate in enumerate(mutations):
            with self.subTest(mutation=index), tempfile.TemporaryDirectory() as temporary:
                value = manifest(Path(temporary).resolve())
                variant = value["variants"][1]
                expected = S.CHECK.json_value(Path(variant["expectation"]).read_bytes())
                mutate(expected)
                raw = F.F.encoded(expected)
                Path(variant["expectation"]).write_bytes(raw)
                variant["expectation_sha256"] = S.CHECK.sha256(raw)
                with self.assertRaisesRegex(ValueError, "unmatched"):
                    S.validate_manifest(value)


if __name__ == "__main__":
    unittest.main()
