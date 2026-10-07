import importlib.util
from pathlib import Path
import struct
import unittest


HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("perf_probe", HERE / "probe.py")
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class ProbeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.core = probe.load_helper(HERE.parents[2] / "proofs/tensor-parallel-kernels-v1/probe.py")
        cls.fixtures = probe.self_test(cls.core)

    def test_closed_full_and_wave_only_rosters(self):
        self.assertEqual(len(self.fixtures), 36)
        self.assertEqual(len({case["symbol"] for case in self.fixtures}), 13)
        wave = [case for case in self.fixtures if "_mfma_" not in case["symbol"]]
        self.assertEqual(len(wave), 26)
        self.assertEqual(len({case["symbol"] for case in wave}), 11)

    def test_transpose_is_not_a_relabel_and_round_trips(self):
        source = probe.pack_bf16(range(6))
        transposed = probe.transpose_bf16(source, 2, 3)
        self.assertNotEqual(source, transposed)
        self.assertEqual(transposed, probe.pack_bf16([0, 3, 1, 4, 2, 5]))
        self.assertEqual(probe.transpose_bf16(transposed, 3, 2), source)

    def test_projection_grid_contracts_match_actual_abi(self):
        for case in self.fixtures:
            if case["symbol"].startswith("ferric_qwen3_tp_wave_gemv_"):
                rows, n, _, _, _ = case["scalars"]
                self.assertEqual(case["groups"], rows * n)
            if case["symbol"].startswith("ferric_qwen3_tp_mfma_gemm_"):
                rows, n, k, _, _ = case["scalars"]
                self.assertEqual(case["groups"], n // 16)
                self.assertEqual(len(case["buffers"][0]["data"]), rows * k * 2)
                self.assertEqual(len(case["buffers"][1]["data"]), n * k * 2)

    def test_dense_projection_preserves_distinct_rows_and_columns(self):
        case = next(c for c in self.fixtures if c["name"] == "wave_dense_partial_rows_3")
        data = case["buffers"][2]["expected"]
        factor = (512 // 16) * sum((d - 5) * (d - 7) for d in range(16)) / 4096.0
        for row in range(3):
            for column in (0, 1, 3, 6, 4095):
                value = struct.unpack_from("<f", data, (row * 4096 + column) * 4)[0]
                self.assertEqual(value, (row + 1) * ((column % 7) - 3) * factor)
        self.assertEqual(data[3 * 4096 * 4:], b"\xA5" * (13 * 4096 * 4))

    def test_dense_attention_reads_both_halves_but_preserves_unused_nan_pages(self):
        original = next(c for c in self.fixtures if c["name"] == "attention_nonzero_qk_mixed_positions")
        dense = next(c for c in self.fixtures if c["name"] == "wave_attention_nonzero_qk_mixed_positions_dense")
        self.assertEqual(original["buffers"][5]["expected"], dense["buffers"][5]["expected"])
        self.assertNotEqual(dense["buffers"][0]["data"][254:256], b"\0\0")
        self.assertEqual(dense["buffers"][1]["data"][:256], self.core.bf16(0x7FC0, 128))

    def test_residual_cases_use_only_active_extents(self):
        cases = [c for c in self.fixtures if c["name"].startswith("residual_rows_")]
        self.assertEqual([c["scalars"][0] for c in cases], [1, 3, 16])
        for case in cases:
            rows = case["scalars"][0]
            self.assertEqual(case["groups"], rows * 64)
            self.assertEqual(len(case["buffers"][0]["data"]), rows * 4096 * 4)
            self.assertEqual(len(case["buffers"][2]["expected"]), rows * 4096 * 2)


if __name__ == "__main__":
    unittest.main()
