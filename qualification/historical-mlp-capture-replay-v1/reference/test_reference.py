"""Pure CPU/unit fixtures; these tests make no device execution claim."""
import hashlib
import io
import json
import math
import os
from pathlib import Path
import struct
import tempfile
import unittest
from unittest import mock

import numpy as np

import extract as ex
import reference as ref


class ExtractionTests(unittest.TestCase):
    def test_duplicate_and_nonfinite_json(self):
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}', b'{"x":Infinity}'):
            with self.assertRaises(ValueError):
                ex.json_bytes(raw)

    def test_row_and_column_shards_have_distinct_rank_order(self):
        matrix = np.arange(24, dtype='<u2').reshape(4, 6)
        raw = matrix.tobytes()
        with tempfile.TemporaryDirectory() as temp:
            for axis in ('rows', 'columns'):
                for rank in (0, 1):
                    ranges, shape = ex.shard_ranges([4, 6], rank, axis)
                    pin = ex.write_ranges(io.BytesIO(raw), 0, ranges,
                                          Path(temp) / f'{axis}-{rank}.bf16', shape)
                    wanted = matrix[rank * 2:(rank + 1) * 2] if axis == 'rows' else matrix[:, rank * 3:(rank + 1) * 3]
                    self.assertEqual(Path(pin['path']).read_bytes(), wanted.tobytes())
                    self.assertEqual(pin['sha256'], hashlib.sha256(wanted.tobytes()).hexdigest())
            self.assertNotEqual((Path(temp) / 'rows-1.bf16').read_bytes(),
                                (Path(temp) / 'columns-1.bf16').read_bytes())

    def test_rank_shape_axis_rejected(self):
        for shape, rank, axis in (([4, 6], True, 'rows'), ([4, 6], -1, 'rows'),
                                  ([4, 6], 2, 'columns'), ([3, 6], 0, 'rows'),
                                  ([4, 5], 0, 'columns'), ([4], 0, 'rows'),
                                  ([False, 6], 0, 'rows'), ([4, 6], 0, 'transpose')):
            with self.assertRaises(ValueError):
                ex.shard_ranges(shape, rank, axis)

    def test_header_shape_dtype_extent_overlap_mutations(self):
        good = {'weight': {'dtype': 'BF16', 'shape': [2, 4], 'data_offsets': [8, 24]},
                'other': {'dtype': 'U8', 'shape': [8], 'data_offsets': [0, 8]}}
        self.assertEqual(ex.tensor_entry(good, 'weight', [2, 4], 24), (8, 24))
        for field, value in [('dtype', 'F16'), ('shape', [4, 2]), ('shape', [True, 8]),
                             ('data_offsets', [8, 22]), ('data_offsets', [9, 25]),
                             ('data_offsets', [True, 17]), ('data_offsets', [8, 26])]:
            bad = json.loads(json.dumps(good))
            bad['weight'][field] = value
            with self.assertRaises(ValueError):
                ex.tensor_entry(bad, 'weight', [2, 4], 24)
        bad = json.loads(json.dumps(good))
        bad['other']['data_offsets'] = [0, 10]
        with self.assertRaises(ValueError):
            ex.tensor_entry(bad, 'weight', [2, 4], 24)

    def test_pin_mutation_and_symlink(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'payload'
            path.write_bytes(b'original')
            pin = ex.file_pin(path, b'original')
            self.assertEqual(ex.pinned_bytes(pin, 8), b'original')
            for key, value in [('bytes', True), ('bytes', 7), ('sha256', '0' * 64)]:
                with self.assertRaises(ValueError):
                    ex.pinned_bytes(dict(pin, **{key: value}), 8)
            path.write_bytes(b'mutation')
            with self.assertRaises(ValueError):
                ex.pinned_bytes(pin, 8)
            alias = Path(temp) / 'link'
            alias.symlink_to(path)
            with self.assertRaises(ValueError):
                ex.pinned_bytes(dict(pin, path=str(alias)), 8)

    def test_nonfinite_and_partial_read_refusal(self):
        for word in (0x7f80, 0xff80, 0x7fc1):
            with self.assertRaises(ValueError):
                ex.finite_bf16(struct.pack('<H', word))
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'incomplete'
            with self.assertRaises(ValueError):
                ex.write_ranges(io.BytesIO(b'\0\0'), 0, [(0, 4)], path, [2])
            self.assertTrue(path.exists())
            with self.assertRaises(FileExistsError):
                ex.write_ranges(io.BytesIO(b'\0' * 4), 0, [(0, 4)], path, [2])


class NumericalTests(unittest.TestCase):
    def test_direct_bf16_rounding_avoids_double_rounding(self):
        middle = (ref.scalar_bf16(0x3f81) + ref.scalar_bf16(0x3f82)) * 0.5
        below = math.nextafter(middle, -math.inf)
        self.assertEqual(ref.round_bf16_real(below), 0x3f81)
        self.assertEqual(ref.round_bf16_real(float(np.float32(below))), 0x3f82)
        self.assertEqual(ref.round_bf16_real(middle), 0x3f82)
        self.assertEqual(ref.round_bf16_real(-below), 0xbf81)
        self.assertEqual(ref.round_bf16_real(-0.0), 0x8000)
        self.assertEqual(ref.round_bf16_real(2.0**-134), 0)
        self.assertEqual(ref.round_bf16_real(math.nextafter(2.0**-134, math.inf)), 1)

    def test_rounding_cells_ties_extremes_and_signed_zero(self):
        middle = (ref.scalar_bf16(0x3f81) + ref.scalar_bf16(0x3f82)) * 0.5
        self.assertFalse(ref.cell_intersects(0x3f81, middle, middle))
        self.assertTrue(ref.cell_intersects(0x3f82, middle, middle))
        self.assertFalse(ref.cell_intersects(0, -2.0**-135, -2.0**-135))
        self.assertTrue(ref.cell_intersects(0x8000, -2.0**-135, -2.0**-135))
        _, upper, _ = ref.cell(0x7f7f)
        with self.assertRaises(ValueError):
            ref.round_bf16_real(upper)
        self.assertEqual(ref.round_bf16_real(math.nextafter(upper, -math.inf)), 0x7f7f)

    def test_word_types_nonfinite_and_length(self):
        for values in ([True], [65536], [-1], [0x7f80], [0x7fc0], []):
            with self.assertRaises(ValueError):
                ref.words(values, 1)
        self.assertEqual(ref.words([0x8000], 1).tolist(), [0x8000])

    def test_gemv_ones_and_absolute_cancellation_bound(self):
        inputs = np.full(4096, 0x3f80, dtype='<u2')
        weights = np.full((2, 4096), 0x3f80, dtype='<u2')
        center, bound = ref.gemv_reference(weights, inputs, 71)
        np.testing.assert_array_equal(center, [4096, 4096])
        self.assertTrue(np.all((bound > 0) & (bound < 0.05)))
        ref.check_bf16_gemv(weights, inputs, np.full(2, 0x4580, dtype='<u2'))
        weights[:] = 0
        weights[:, 0], weights[:, 64], weights[:, 128] = 0x4b80, 0x3f80, 0xcb80
        center, bound = ref.gemv_reference(weights, inputs, 71)
        np.testing.assert_array_equal(center, [1, 1])
        self.assertTrue(np.all(bound > 1))
        ref.check_bf16_gemv(weights, inputs, np.zeros(2, dtype='<u2'))
        with self.assertRaises(ValueError):
            ref.check_bf16_gemv(weights, inputs, np.full(2, 0x4780, dtype='<u2'))

    def test_underflow_allowance_and_down_partial(self):
        zero_bound = ref.error_envelope(np.array([0.0]), 6144, 103)[0]
        self.assertGreater(zero_bound, 12351 * 2.0**-150)
        weight = np.full((1, 6144), 0x3f80, dtype='<u2')
        activation = np.full(6144, 0x3f80, dtype='<u2')
        actual = np.asarray([6144.0], dtype='<f4').view('<u4')
        self.assertEqual(ref.check_f32_gemv(weight, activation, actual)['max_absolute_error'], 0)
        with self.assertRaises(ValueError):
            ref.check_f32_gemv(weight, activation, np.asarray([6150.0], dtype='<f4').view('<u4'))

    def test_dual_rounding_is_not_fused_weighting(self):
        normalized, weight = 1.00390625, ref.scalar_bf16(0x3f81)
        first = ref.round_bf16_real(normalized)
        staged = ref.round_bf16_real(float(np.float32(ref.scalar_bf16(first) * weight)))
        collapsed = ref.round_bf16_real(float(np.float32(normalized * weight)))
        self.assertEqual(staged, 0x3f81)
        self.assertEqual(collapsed, 0x3f82)

    def test_norm_independent_interval_accepts_staged_model_and_rejects_mutation(self):
        inputs = np.full(4096, 0x3f80, dtype='<u2')
        inputs[0], inputs[1], inputs[2] = 0x8000, 0x3f81, 0xbf00
        weight = np.full(4096, 0x3f81, dtype='<u2')
        expected = ref.norm_diagnostic(inputs, weight)
        report = ref.check_norm(inputs, weight, expected)
        self.assertEqual(report['mismatches'], 0)
        self.assertEqual(int(expected[0]), 0x8000)
        bad = expected.copy()
        bad[0] = 0
        with self.assertRaises(ValueError):
            ref.check_norm(inputs, weight, bad)
        bad = expected.copy()
        bad[100] = 0x4000
        with self.assertRaises(ValueError):
            ref.check_norm(inputs, weight, bad)

    def test_norm_uncertainty_refuses_instead_of_widening(self):
        with self.assertRaises(ValueError):
            ref._possible_narrowed(-1.0, 1.0)
        with self.assertRaises(ValueError):
            ref.norm_inverse_interval(np.full(4096, 0x7f7f, dtype='<u2'))

    def test_stable_silu_extremes_zero_sign_and_fixed_tolerance(self):
        gate = np.asarray([0, 0x8000, 0x3f80, 0xbf80, 0x42c8, 0xc2c8], dtype='<u2')
        up = np.asarray([0xbf80, 0xbf80, 0x3f80, 0x3f80, 0x3f80, 0x3f80], dtype='<u2')
        expected = ref.swiglu_reference(gate, up)
        self.assertEqual(int(expected[0]), 0x8000)
        self.assertEqual(int(expected[1]), 0)
        self.assertEqual(ref.check_swiglu(gate, up, expected)['exact'], 6)
        adjacent = expected.copy()
        adjacent[2] += 1
        self.assertEqual(ref.check_swiglu(gate, up, adjacent)['tolerated'], 1)
        adjacent[2] += 1
        with self.assertRaises(ValueError):
            ref.check_swiglu(gate, up, adjacent)
        wrong_zero = expected.copy()
        wrong_zero[0] = 0
        with self.assertRaises(ValueError):
            ref.check_swiglu(gate, up, wrong_zero)

    def test_oracle_requires_frozen_exact_source(self):
        with tempfile.TemporaryDirectory() as temp:
            fake = Path(temp) / 'reference.py'
            fake.write_text('raise RuntimeError("must never execute")\n')
            with self.assertRaises(ValueError):
                ref.load_oracle(fake)

    def test_frozen_integer_residual_order_zero_overflow_and_inputs_unchanged(self):
        default = '/home/harmenon/ferric-asrock-42/evidence/wave-output-lowering-v216/tp2-residual-reference/reference.py'
        oracle = ref.load_oracle(os.environ.get('P218_INTEGER_ORACLE', default))
        rank0 = struct.pack('<4096I', *([0x4b800000] * 4096))
        rank1 = struct.pack('<4096I', *([0x3f800000] * 4096))
        residual = struct.pack('<4096H', *([0xcb80] * 4096))
        original = (rank0, rank1, residual)
        result = oracle(rank0, rank1, residual)
        self.assertEqual(result, bytes(8192))
        self.assertEqual((rank0, rank1, residual), original)
        self.assertEqual(oracle(bytes(16384), bytes(16384), struct.pack('<4096H', *([0x8000] * 4096))),
                         bytes(8192))
        overflow = struct.pack('<4096I', *([0x7f7fffff] * 4096))
        with self.assertRaises(ValueError):
            oracle(overflow, overflow, bytes(8192))
        with self.assertRaises(ValueError):
            oracle(rank0[:-4], rank1, residual)

    def test_raw_observation_status_rejected_before_loading_files(self):
        for observed in ({}, {'completed_and_closed': False, 'numerical_acceptance': False},
                         {'completed_and_closed': True, 'numerical_acceptance': True},
                         {'completed_and_closed': 1, 'numerical_acceptance': False}):
            with self.assertRaises(ValueError):
                ref.check_observation('/not/a/fixture', '0' * 64, observed, '/not/an/oracle')

    def test_policy_literals_match_bound_and_do_not_allow_adaptation(self):
        policy = ex.json_bytes((Path(__file__).parent / 'policy.json').read_bytes())
        self.assertFalse(policy['adaptive_tolerance'])
        self.assertEqual(policy['norm']['epsilon_f32_bits'],
                         struct.unpack('<I', struct.pack('<f', 1e-6))[0])
        for role, k, depth in [('gate_up', 4096, 71), ('down', 6144, 103)]:
            self.assertEqual(policy[role]['gamma_f32'], depth)
            self.assertEqual(policy[role]['half_subnormal_allowances'], 2 * k + 63)
        self.assertEqual(policy['residual']['oracle_sha256'], ref.ORACLE_SHA)


if __name__ == '__main__':
    unittest.main()
