"""CPU engineering checks. These are neither native evidence nor Verus proofs."""
from fractions import Fraction
import hashlib
import importlib.util
import os
from pathlib import Path
import struct
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("tp1_attention_probe_under_test", HERE / "probe.py")
probe = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(probe)


def number(data, element):
    bits = struct.unpack_from("<H", data, element * 2)[0]
    return int(struct.unpack("<f", struct.pack("<I", bits << 16))[0])


def independent_head(case, row, head, *, wrong_page=False, future=False, dimensions=128,
                     dropped_qk=False, wrong_head=False, wrong_slot=False):
    """Read physical bytes and integer dot products, not the fixture's formula."""
    query, keys, values, positions, table, _ = [record["data"] for record in case["buffers"]]
    count = struct.unpack_from("<I", positions, row * 4)[0] + 1
    if future:
        count = 32
    kv_head = (head // 4 + (1 if wrong_head else 0)) % 8
    q = [number(query, (row * 32 + head) * 128 + dimension) for dimension in range(dimensions)]
    scores, starts = [], []
    for token in range(count):
        page = 64 if wrong_page else struct.unpack_from("<I", table, (row * 2 + token // 16) * 4)[0]
        slot = (token % 16 + (1 if wrong_slot else 0)) % 16
        start = ((page * 16 + slot) * 8 + kv_head) * 128
        starts.append(start)
        scores.append(0 if dropped_qk else sum(q[dimension] * number(keys, start + dimension)
                                              for dimension in range(dimensions)))
    maximum = max(scores)
    # Intended images clamp every nonzero logit difference in these fixtures.
    if any(score != maximum and maximum - score < 2048 for score in scores):
        raise AssertionError("mutation is outside the controlled score classes")
    selected = [start for start, score in zip(starts, scores) if score == maximum]
    return [Fraction(sum(number(values, start + dimension) for start in selected), len(selected))
            for dimension in range(128)]


class AttentionFixtures(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        helper = Path(os.environ.get("FERRIC_ATTENTION_HELPER", HERE.parent / "tensor-parallel-kernels-v1" / "probe.py"))
        cls.core = probe.load_helper(helper)
        cls.cases = {spec: probe.make_case(cls.core, spec) for spec in probe.specifications() if spec[2] == "baseline"}

    def test_closed_roster_and_launch_shapes(self):
        expected = [(family, rows, mode) for family, rows in
                    (("uniform", 1), ("uniform", 17), ("uniform", 31), ("selector", 32))
                    for mode in ("baseline", "wave")]
        self.assertEqual(list(probe.specifications()), expected)
        self.assertEqual(probe.ROOTS, ("ferric_qwen3_tp_batch32_paged_gqa_bf16_f32_v5",
                                      "ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5"))
        for spec in expected:
            case = probe.make_case(self.core, spec)
            probe.validate_case(case)
            self.assertEqual(case["scalars"], [spec[1], 1, 2, 68, 32])
            self.assertEqual(case["groups"], spec[1] * 32)
            self.assertEqual([len(record["data"]) for record in case["buffers"]],
                             [262144, 2228224, 2228224, 128, 256, 262144])
            self.assertTrue(all(len(record["data"]) + 128 < self.core.LIMIT for record in case["buffers"]))

    def test_both_modes_have_identical_full_buffer_oracles(self):
        for (family, rows, _), baseline in self.cases.items():
            wave = probe.make_case(self.core, (family, rows, "wave"))
            self.assertEqual(baseline["buffers"], wave["buffers"])
            self.assertEqual(baseline["scalars"], wave["scalars"])
            self.assertNotEqual(baseline["symbol"], wave["symbol"])

    def test_integer_encoding_and_dense_orthogonality(self):
        data = probe.exact_bf16(range(-256, 257))
        self.assertEqual([number(data, index) for index in range(513)], list(range(-256, 257)))
        for value in (257, -257, 0.5, True):
            with self.assertRaises(RuntimeError):
                probe.exact_bf16([value])
        patterns = [[probe.pattern(index, dimension) for dimension in range(128)] for index in range(4)]
        self.assertTrue(all(abs(value) == 1 for pattern in patterns for value in pattern))
        self.assertEqual([[sum(a * b for a, b in zip(left, right)) for right in patterns]
                          for left in patterns], [[128 if a == b else 0 for b in range(4)] for a in range(4)])
        self.assertEqual(sum(patterns[0][dimension] * patterns[1][dimension] for dimension in range(64)), 64)
        scale = struct.unpack("<f", struct.pack("<I", 0x3db504f3))[0]
        clamp = struct.unpack("<f", struct.pack("<I", 0xc2ce8ed0))[0]
        self.assertGreater(4096 * scale, 256)
        self.assertLess(-2048 * scale, clamp)

    def test_complete_independent_physical_oracle(self):
        for (_, rows, _), case in self.cases.items():
            expected = bytearray(case["buffers"][-1]["data"])
            for row in range(rows):
                for head in range(32):
                    values = independent_head(case, row, head)
                    self.assertTrue(all(value.denominator == 1 for value in values))
                    encoded = probe.exact_bf16([value.numerator for value in values])
                    offset = (row * 32 + head) * 256
                    expected[offset:offset + 256] = encoded
            self.assertEqual(bytes(expected), case["buffers"][-1]["expected"])

    def test_pages_masks_decoys_and_inactive_tails(self):
        for (_, rows, _), case in self.cases.items():
            table = struct.unpack("<64I", case["buffers"][4]["data"])
            self.assertEqual(table, tuple(value for row in range(32) for value in (63 - 2 * row, 62 - 2 * row)))
            self.assertEqual(set(table), set(range(64)))
            positions = struct.unpack("<32I", case["buffers"][3]["data"])
            expected_positions = (1,) * 32 if rows == 1 else (0, 1, 14, 15, 16, 17, 30, 31) * 4
            self.assertEqual(positions, expected_positions)
            output = case["buffers"][-1]
            self.assertEqual(output["expected"][rows * 8192:], probe.SENTINEL * ((32 - rows) * 4096))
            for page in range(64, 68):
                start = page * 16 * 8 * 128
                self.assertEqual(number(case["buffers"][1]["data"], start), 3)
                self.assertEqual(number(case["buffers"][2]["data"], start), -64)
            for row in range(rows):
                for token in range(32):
                    page = table[row * 2 + token // 16]
                    start = (page * 16 + token % 16) * 8 * 128
                    value = number(case["buffers"][2]["data"], start)
                    self.assertEqual(value, 2 * token + 1 + 2 * row if token <= positions[row] else 224)

    def test_wrong_page_mask_head_and_slot_are_detectable(self):
        case = self.cases[("uniform", 17, "baseline")]
        correct = independent_head(case, 1, 0)
        for mutation in ("wrong_page", "future", "wrong_head", "wrong_slot"):
            with self.subTest(mutation=mutation):
                self.assertNotEqual(correct, independent_head(case, 1, 0, **{mutation: True}))
        self.assertNotEqual(independent_head(case, 4, 0), independent_head(case, 4, 0, wrong_page=True))

    def test_dropped_qk_and_half_dot_are_detectable(self):
        case = self.cases[("selector", 32, "baseline")]
        correct = independent_head(case, 7, 0)
        self.assertNotEqual(correct, independent_head(case, 7, 0, dropped_qk=True))
        self.assertNotEqual(correct, independent_head(case, 7, 0, dimensions=64))
        self.assertNotEqual(independent_head(case, 1, 0), independent_head(case, 1, 0, future=True))

    def test_no_match_selector_uses_all_visible_tokens(self):
        case = self.cases[("selector", 32, "baseline")]
        for head in (1, 2, 3):
            self.assertEqual(independent_head(case, 0, head), independent_head(case, 0, head, dropped_qk=True))
        self.assertNotEqual(independent_head(case, 7, 0), independent_head(case, 7, 1))

    def test_case_contract_rejects_launch_output_and_tail_mutations(self):
        case = self.cases[("uniform", 1, "baseline")]
        for key, value in (("groups", True), ("groups", 64), ("symbol", "other"),
                           ("scalars", [1, 8, 2, 68, 32]), ("name", "unknown")):
            with self.subTest(key=key, value=value), self.assertRaises(RuntimeError):
                probe.validate_case({**case, key: value})
        for offset in (0, 8192, len(case["buffers"][-1]["expected"]) - 1):
            buffers = [dict(record) for record in case["buffers"]]
            changed = bytearray(buffers[-1]["expected"])
            changed[offset] ^= 1
            buffers[-1]["expected"] = bytes(changed)
            with self.assertRaises(RuntimeError):
                probe.validate_case({**case, "buffers": buffers})
        for spec in (("uniform", True, "baseline"), ("uniform", 32, "baseline"), ("selector", 32, "other")):
            with self.assertRaises(RuntimeError):
                probe.make_case(self.core, spec)

    def test_helper_pin_rejects_changed_source(self):
        with tempfile.TemporaryDirectory(prefix="tp1-attention-pin-") as directory:
            path = Path(directory) / "helper.py"
            path.write_bytes(b"raise RuntimeError('must not execute')\n")
            self.assertNotEqual(hashlib.sha256(path.read_bytes()).hexdigest(), probe.HELPER_SHA256)
            with self.assertRaisesRegex(RuntimeError, "helper identity drift"):
                probe.load_helper(path)


if __name__ == "__main__":
    unittest.main()
