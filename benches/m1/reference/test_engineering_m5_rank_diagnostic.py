#!/usr/bin/env python3
"""Host-only ranking regressions; no model or GPU execution."""

import copy
import importlib.util
from pathlib import Path
import struct
import sys
import unittest


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


diagnostic = load("m5_rank_diagnostic", "engineering_m5_rank_diagnostic.py")
fixtures = load("m5_rank_fixtures", "test_engineering_m5_reference.py")


def row(values):
    data = bytearray(diagnostic.reference.ROW_BYTES)
    for token, bits in values.items():
        struct.pack_into("<H", data, token * 2, bits)
    return bytes(data)


class RankTests(unittest.TestCase):
    def test_strict_winner_reversal_reports_both_signed_errors_and_ranks(self):
        result = diagnostic.row_diagnostic(row({16: 0x3F80, 576: 0x4000}),
                                           row({16: 0x4000, 576: 0x3F80}))
        self.assertTrue(result["strict_winner_reversal"])
        self.assertEqual(result["ferric_winner_minus_reference_winner"], 1.0)
        self.assertEqual(result["reference_winner_minus_ferric_winner"], 1.0)
        self.assertEqual(result["winner_comparison"], [
            {"token": 16, "ferric_logit": 1.0, "reference_logit": 2.0,
             "signed_error": -1.0, "ferric_rank": 2, "reference_rank": 1},
            {"token": 576, "ferric_logit": 2.0, "reference_logit": 1.0,
             "signed_error": 1.0, "ferric_rank": 1, "reference_rank": 2},
        ])

    def test_tie_uses_lowest_id_and_is_not_strict_reversal(self):
        result = diagnostic.row_diagnostic(row({16: 0x4000, 576: 0x4000}),
                                           row({16: 0x3F80, 576: 0x4000}))
        self.assertTrue(result["token_mismatch"])
        self.assertFalse(result["strict_winner_reversal"])
        self.assertEqual(result["ferric"]["argmax_token"], 16)
        self.assertEqual(result["ferric"]["maximum_tie_count"], 2)
        self.assertEqual(result["ferric"]["top2_gap"], 0.0)

    def test_identical_rows_do_not_claim_a_mismatch(self):
        data = row({31: 0x4000})
        result = diagnostic.row_diagnostic(data, data)
        self.assertFalse(result["token_mismatch"])
        self.assertFalse(result["strict_winner_reversal"])
        self.assertEqual(len(result["winner_comparison"]), 1)

    def test_signed_zero_ties_and_negative_values_follow_numeric_order(self):
        values = [-1.0] * diagnostic.core.VOCABULARY_SIZE
        values[17], values[18] = -0.0, 0.0
        result = diagnostic.ranking(values)
        self.assertEqual(result["argmax_token"], 17)
        self.assertEqual(result["maximum_tie_count"], 2)

    def test_wrong_extent_and_nonfinite_rows_reject(self):
        for data in (b"", row({})[:-2], row({7: 0x7F80}), row({7: 0xFF80}), row({7: 0x7FC0})):
            with self.subTest(length=len(data)), self.assertRaises(diagnostic.Failure):
                diagnostic.decode_row(data)

    def fixture(self):
        capture, actual = fixtures.fixture()
        metrics = [dict(position=128 + index, **diagnostic.reference.row_metrics(
            actual[index * diagnostic.reference.ROW_BYTES:(index + 1) * diagnostic.reference.ROW_BYTES],
            actual[index * diagnostic.reference.ROW_BYTES:(index + 1) * diagnostic.reference.ROW_BYTES],
        )) for index in range(5)]
        return capture, actual, {
            "format": "FERRIC-ENGINEERING-S1-K4-DIFFERENTIAL-V1", "authority": "none",
            "qualification": False, "benchmark_comparable": False,
            "tolerance_reviewed": False, "numerical_pass_claimed": False,
            "reference_execution": "independent-full-133-token-sequence-without-ferric-kv-two-byte-identical-runs",
            "reference_model": {"repository": diagnostic.core.PINNED_REPOSITORY,
                                "revision": diagnostic.core.PINNED_REVISION},
            "input_token_ids": capture["prefix_token_ids"] + capture["target_token_ids"],
            "ferric_logits_sha256": diagnostic.reference.digest(actual),
            "reference_logits_sha256": diagnostic.reference.digest(actual),
            "rows": metrics, "token_mismatch_count": 0, "max_bf16_ulp": 0,
        }

    def test_complete_analysis_retains_all_five_positions_and_nonclaims(self):
        capture, actual, comparison = self.fixture()
        result = diagnostic.analyze(capture, actual, comparison, actual)
        self.assertEqual([entry["position"] for entry in result["rows"]], list(range(128, 133)))
        for key in ("qualification", "benchmark_comparable", "new_model_execution",
                    "numerical_pass_claimed", "cause_established"):
            self.assertIs(result[key], False)

    def test_source_substitution_and_authority_promotion_reject(self):
        capture, actual, original = self.fixture()
        for key, value in (("qualification", True), ("qualification", 0),
                           ("reference_logits_sha256", "a" * 64),
                           ("input_token_ids", [0] * 133)):
            comparison = copy.deepcopy(original)
            comparison[key] = value
            with self.subTest(key=key), self.assertRaises(diagnostic.Failure):
                diagnostic.analyze(capture, actual, comparison, actual)

    def test_changed_metrics_and_truncated_reference_reject(self):
        capture, actual, original = self.fixture()
        comparison = copy.deepcopy(original)
        comparison["rows"][3]["rmse"] = 1.0
        with self.assertRaisesRegex(diagnostic.Failure, "metrics"):
            diagnostic.analyze(capture, actual, comparison, actual)
        with self.assertRaises(diagnostic.Failure):
            diagnostic.analyze(capture, actual, original, actual[:-2])


if __name__ == "__main__":
    unittest.main()
