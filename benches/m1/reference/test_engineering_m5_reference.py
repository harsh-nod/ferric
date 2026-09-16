#!/usr/bin/env python3
"""CPU-only protocol/metric tests; no model or completed-queue evidence."""

import copy
import importlib.util
from pathlib import Path
import struct
import sys
import unittest

specification = importlib.util.spec_from_file_location(
    "ferric_engineering_m5_reference", Path(__file__).with_name("engineering_m5_reference.py")
)
assert specification is not None and specification.loader is not None
reference = importlib.util.module_from_spec(specification)
sys.modules[specification.name] = reference
specification.loader.exec_module(reference)


def fixture():
    rows = []
    descriptors = []
    for index in range(5):
        row = bytearray(reference.ROW_BYTES)
        struct.pack_into("<H", row, (11 + index) * 2, 0x3F80)
        rows.append(bytes(row))
        descriptors.append({
            "active_index": index, "position": 128 + index,
            "bytes": reference.ROW_BYTES, "offset_bytes": 64 + index * reference.ROW_BYTES,
            "sha256": reference.digest(row), "argmax_token": 11 + index,
        })
    logits = b"".join(rows)
    document = {
        "format": "FERRIC-ENGINEERING-S1-K4-ALL-LOGITS-V1", "authority": "none",
        "qualification": False, "benchmark_comparable": False,
        "numerical_comparison_performed": False,
        "scope": "single-sequence-k4-five-target-positions-with-real-paired-prefill",
        "reference_sequence_semantics": "full-128-token-prefix-plus-anchor-plus-four-actual-draft-proposals",
        "prefix_token_ids": list(range(128)), "target_token_ids": [31, 41, 43, 47, 53],
        "positions": [128, 129, 130, 131, 132], "shape": [1, 5, reference.core.VOCABULARY_SIZE],
        "dtype": "bf16-little-endian", "dispatch_generation": 17,
        "logits": {"path": "target-logits.bf16", "bytes": len(logits), "sha256": reference.digest(logits)},
        "rows": descriptors,
        "smoke": {
            "authority": "none", "artifact_authority": "none", "benchmark_comparable": False,
            "authenticated_AB_exercised": False, "compiler_origin_authenticated": False,
            "current_publication_selected": False, "worker_v3_authenticated": False,
            "hardware_completion_observed": True, "target": "gfx942:xnack-",
            "program_strategy": "AttributedMfma13", "program_count": 13,
            "identities": {"model_bundle_sha256": reference.core.PINNED_MODEL_IDENTITY,
                           "target_prepacked_sha256": reference.TARGET_PREPACKED},
            "prompt": {"physical_token_ids": list(range(128))},
            "paired_prefill": {"first_token_id": 31},
            "speculative_k4": {"draft_choices": [41, 43, 47, 53],
                               "target_choices": [11, 12, 13, 14, 15], "dispatch_generation": 17},
        },
    }
    return document, logits


class M5ProtocolTests(unittest.TestCase):
    def test_real_prefix_and_all_five_actual_positions_are_retained(self):
        document, logits = fixture()
        self.assertEqual(reference.validate_capture(document, logits), list(range(128)) + [31, 41, 43, 47, 53])

    def test_prefix_proposal_generation_and_model_substitutions_reject(self):
        original, logits = fixture()
        for path, value in [
            (("prefix_token_ids", 127), 37), (("target_token_ids", 4), 59),
            (("dispatch_generation",), 18),
            (("smoke", "identities", "model_bundle_sha256"), "a" * 64),
            (("smoke", "program_strategy"), "LegacyScalar12"),
            (("qualification",), True),
        ]:
            document = copy.deepcopy(original)
            target = document
            for key in path[:-1]:
                target = target[key]
            target[path[-1]] = value
            with self.subTest(path=path), self.assertRaises(reference.Failure):
                reference.validate_capture(document, logits)

    def test_tail_row_geometry_and_missing_or_nonfinite_bytes_reject(self):
        original, logits = fixture()
        for index in range(5):
            document = copy.deepcopy(original)
            document["rows"][index]["offset_bytes"] += 2
            with self.subTest(index=index), self.assertRaises(reference.Failure):
                reference.validate_capture(document, logits)
        with self.assertRaises(reference.Failure):
            reference.validate_capture(original, logits[:-reference.ROW_BYTES])
        damaged = bytearray(logits)
        struct.pack_into("<H", damaged, 4 * reference.ROW_BYTES, 0x7FC0)
        document = copy.deepcopy(original)
        document["logits"]["sha256"] = reference.digest(damaged)
        document["rows"][4]["sha256"] = reference.digest(damaged[4 * reference.ROW_BYTES:])
        with self.assertRaises(reference.Failure):
            reference.validate_capture(document, bytes(damaged))

    def test_metrics_report_mismatch_without_qualification_threshold(self):
        _, logits = fixture()
        exact = reference.row_metrics(logits[:reference.ROW_BYTES], logits[:reference.ROW_BYTES])
        self.assertEqual(exact["max_bf16_ulp"], 0)
        self.assertEqual(exact["rmse"], 0)
        self.assertFalse(exact["token_mismatch"])
        mismatch = reference.row_metrics(logits[:reference.ROW_BYTES], logits[reference.ROW_BYTES:2 * reference.ROW_BYTES])
        self.assertTrue(mismatch["token_mismatch"])
        self.assertGreater(mismatch["max_bf16_ulp"], 0)
        self.assertNotIn("accepted", mismatch)


if __name__ == "__main__":
    unittest.main()
