"""Adapter-only tests; the unchanged M1 validator owns actual receipt checking."""
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import Mock, patch

from common import canonical, LEDGER
import report


class M1AdapterTests(unittest.TestCase):
    def test_delegates_to_existing_validator_then_only_exports_whitelisted_fields(self):
        equivalence = {key: 16 for key in LEDGER.EQUIVALENCE_FIELDS}
        equivalence.update(model="Qwen/Qwen3-8B", dtype="bf16", target="gfx950", tensor_parallel=1,
            device_unique_ids=[987654321], model_bundle_id="a" * 64, workload_sha256="b" * 64,
            reference_sha256="c" * 64)
        expected = {key: "d" * 64 for key in LEDGER.CHECK.IDENTITY_FIELDS}
        expected["performance_profile"] = None
        metrics = dict(requests={"one": {"ttft_seconds": 0.1, "tpot_seconds": 0.2}},
            output_tokens_per_second=3, comparison_sha256="e" * 64, input_sha256={"results.jsonl": "f" * 64},
            run_dir="/private/path", worker_pid=44444)
        original = dict(measurement="host-wall-latency-not-gpu-duration", baseline="base", equivalence=equivalence,
            nonclaims=["SYNTHETIC CPU ADAPTER TEST ONLY"], variants=[dict(name="base", kind="baseline",
                expected=expected, repetitions=1, observations=[], runs=[dict(metrics=metrics, timing_sha256="1" * 64)])])
        aggregate = Mock(return_value=original)
        host = SimpleNamespace(aggregate=aggregate, LEDGER=LEDGER, CHECK=LEDGER.CHECK)
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            path = base / "manifest.json"
            value = {"synthetic": "adapter-only test; never a hardware receipt"}
            path.write_bytes(canonical(value))
            with patch.object(report, "load_module", return_value=host):
                result = report.m1_report(path, base / "output")
            aggregate.assert_called_once_with(value)
            self.assertEqual(result["variants"][0]["output_tokens_per_second"]["mean"], 3)
            self.assertEqual(result["equivalence"]["model"], "Qwen/Qwen3-8B")
            for private in (b"987654321", b"worker_pid", b"device_unique_ids", b"/private/path"):
                self.assertNotIn(private, canonical(result))
            self.assertFalse((base / "output" / "host-timeline.svg").exists())
            self.assertFalse(result["performance_qualified"])

    def test_existing_m1_rejection_is_not_bypassed_or_rendered(self):
        aggregate = Mock(side_effect=ValueError("original M1 validation failure"))
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            path = base / "manifest.json"
            path.write_bytes(canonical({"synthetic": "invalid CPU fixture"}))
            with patch.object(report, "load_module", return_value=SimpleNamespace(aggregate=aggregate)):
                with self.assertRaisesRegex(ValueError, "original M1"):
                    report.m1_report(path, base / "output")
            self.assertFalse((base / "output").exists())


if __name__ == "__main__":
    unittest.main()
