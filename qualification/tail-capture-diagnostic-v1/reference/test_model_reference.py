import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import numpy as np

import model_reference as ref


class Weights:
    def __init__(self, arrays):
        self.arrays = arrays
    def tensor(self, name):
        return self.arrays[name]


class WholeReferenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.helpers = ref.bootstrap()
        cls.old, cls.mlp, cls.attention, cls.oracle = cls.helpers
        cls.geometry = ref.Geometry(2, 8, 16, 4, 2, 2, 7)

    def words(self, values):
        return self.old.narrow_dense(self.mlp, np.asarray(values, dtype='<f8'))

    def weights(self):
        g = self.geometry
        arrays = {name: np.zeros(shape, dtype='<u2') for name, shape in ref.tensor_shapes(g).items()}
        for name, values in arrays.items():
            if values.ndim == 1:
                values[:] = 0x3f80
            elif '.v_proj.' in name or '.o_proj.' in name:
                for row in range(values.shape[0]):
                    values[row, row % values.shape[1]] = 0x3f80
            elif '.mlp.' in name:
                values[:] = 0x3c80  # 1/64, keeping every two-layer intermediate finite.
        arrays['model.embed_tokens.weight'][:] = np.asarray([
            self.words([((token + lane) % 7 + 1) / 8 for lane in range(g.hidden)])
            for token in range(g.vocabulary)], dtype='<u2')
        arrays['lm_head.weight'][6] = 0x3f80
        return Weights(arrays)

    def run_case(self, tokens, mode='teacher_forced', weights=None, mutate_sink=False):
        result = {}
        def emit(name, array):
            result[name] = array.copy()
            if mutate_sink:
                array[:] = 0
        cases = ref.run_model(weights or self.weights(), self.helpers, self.geometry, mode, tokens, emit)
        return cases, result

    def test_two_layers_and_two_positions_use_previous_layer_hidden(self):
        cases, arrays = self.run_case([0, 1])
        self.assertEqual([case['input_token'] for case in cases], [0, 1])
        weight = self.weights().tensor('model.layers.1.input_layernorm.weight')
        for position in (0, 1):
            expected = ref.dense_norm(self.helpers, arrays[f'pos{position}-layer0-hidden'], weight)
            np.testing.assert_array_equal(arrays[f'pos{position}-layer1-input-norm'], expected)
        self.assertEqual(arrays['pos1-logits'].shape, (7,))

    def test_no_future_token_can_modify_first_forward(self):
        _, a = self.run_case([0, 1])
        _, b = self.run_case([0, 2])
        for key in a:
            if key.startswith('pos0-'):
                np.testing.assert_array_equal(a[key], b[key], err_msg=key)

    def test_second_attention_uses_own_previous_original_token_history(self):
        _, a = self.run_case([0, 1])
        _, b = self.run_case([2, 1])
        self.assertFalse(np.array_equal(a['pos1-layer0-rank0-attention'], b['pos1-layer0-rank0-attention']))
        # Q/K are all zero, so the exact causal FP64 result is the mean of two V rows.
        expected = (self.mlp.bf16(a['pos0-layer0-rank0-value']).astype(np.float64) +
                    self.mlp.bf16(a['pos1-layer0-rank0-value']).astype(np.float64)) / 2
        expected = np.tile(expected, 2)  # Two local query heads share one KV head.
        np.testing.assert_array_equal(a['pos1-layer0-rank0-attention'],
                                      self.old.narrow_projection(self.mlp, expected))

    def test_autoregressive_uses_cpu_winner_not_an_extra_caller_token(self):
        cases, _ = self.run_case([0], 'autoregressive')
        self.assertEqual(cases[0]['output_token'], 6)
        self.assertEqual(cases[1]['input_token'], 6)
        with self.assertRaises(ValueError):
            self.run_case([0, 1], 'autoregressive')

    def test_invalid_token_modes_and_out_of_range_are_rejected(self):
        for mode, tokens in [('unknown', [0, 1]), ('teacher_forced', [0]),
                             ('teacher_forced', [0, 7]), ('teacher_forced', [0, True]),
                             ('autoregressive', [])]:
            with self.subTest(mode=mode, tokens=tokens), self.assertRaises(ValueError):
                self.run_case(tokens, mode)

    def test_diagnostic_sink_cannot_mutate_recurrence_and_originals_are_unchanged(self):
        weights = self.weights()
        before = {name: array.copy() for name, array in weights.arrays.items()}
        _, a = self.run_case([0, 1], weights=weights)
        _, b = self.run_case([0, 1], weights=weights, mutate_sink=True)
        for name in a:
            np.testing.assert_array_equal(a[name], b[name])
        for name in before:
            np.testing.assert_array_equal(before[name], weights.arrays[name])

    def test_production_head_norm_and_rope_match_frozen_p219_equations(self):
        raw = np.asarray([(0x3e00 + i % 31) for i in range(3072)], dtype='<u2')
        norm = np.full(128, 0x3f81, dtype='<u2')
        for position in (0, 1):
            oldq, oldk, _ = self.old.post_qkv(self.mlp, raw, norm, norm, position)
            np.testing.assert_array_equal(ref.rotate(self.helpers, raw[:2048], norm, position, 128), oldq)
            np.testing.assert_array_equal(ref.rotate(self.helpers, raw[2048:2560], norm, position, 128), oldk)

    def test_production_attention_matches_retained_unpaged_fp64_oracle(self):
        query = np.asarray([0x3e00 + i % 7 for i in range(2048)], dtype='<u2')
        keys = [np.asarray([0x3d00 + (i + p) % 13 for i in range(512)], dtype='<u2') for p in (0, 1)]
        values = [np.asarray([0x3e00 + (i + p) % 11 for i in range(512)], dtype='<u2') for p in (0, 1)]
        result, _ = ref.dense_attention(self.helpers, ref.QWEN8B, query, keys, values)
        original, _ = self.attention.dense_reference(query.tolist(), np.concatenate(keys).tolist(),
                                                    np.concatenate(values).tolist(), 1)
        expected = np.asarray([self.attention.bf16(x) for x in original], dtype='<u2')
        np.testing.assert_array_equal(result, expected)
        values[0][0] = 0x8000
        single, _ = ref.dense_attention(self.helpers, ref.QWEN8B, query, keys[:1], values[:1])
        self.assertEqual(int(single[0]), 0x8000)

    def test_residual_preserves_exact_f32_order_and_does_not_mutate_input(self):
        p0 = np.asarray([2.0**24], dtype='<f4')
        p1 = np.asarray([-2.0**24], dtype='<f4')
        original = np.asarray([0x3f80], dtype='<u2')
        np.testing.assert_array_equal(ref.residual(self.helpers, p0, p1, original), [0x3f80])
        self.assertEqual(original.tolist(), [0x3f80])

    def test_projection_f32_boundary_and_first_index_argmax_are_explicit(self):
        value = 72209139159 / 549755813888
        self.assertEqual(self.old.narrow_projection(self.mlp, np.asarray([value], dtype='<f8')).tolist(), [0x3e06])
        self.assertEqual(ref.choose(np.asarray([0x3f80, 0x3f80, 0], dtype='<u2'), self.mlp), 0)
        self.assertEqual(ref.choose(np.asarray([0x8000, 0], dtype='<u2'), self.mlp), 0)

    def test_header_geometry_overlap_gap_and_duplicate_mutations(self):
        valid = {'a': {'dtype': 'BF16', 'shape': [2], 'data_offsets': [0, 4]},
                 'b': {'dtype': 'BF16', 'shape': [2], 'data_offsets': [4, 8]}}
        self.assertEqual(set(ref.decode_header(json.dumps(valid).encode(), 8)), {'a', 'b'})
        for mutate in ('overlap', 'gap', 'width', 'dtype', 'shape'):
            bad = copy.deepcopy(valid)
            if mutate == 'overlap': bad['b']['data_offsets'] = [2, 6]
            if mutate == 'gap': bad['b']['data_offsets'] = [6, 10]
            if mutate == 'width': bad['a']['data_offsets'] = [0, 2]
            if mutate == 'dtype': bad['a']['dtype'] = 'F16'
            if mutate == 'shape': bad['a']['shape'] = [True]
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                ref.decode_header(json.dumps(bad).encode(), 8)
        with self.assertRaises(ValueError):
            ref.json_bytes(b'{"x":1,"x":2}')

    def test_pinned_file_detects_hash_path_and_size_substitution(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory).resolve() / 'original'
            path.write_bytes(b'original')
            pin = {'path': str(path), 'bytes': 8, 'sha256': hashlib.sha256(b'original').hexdigest()}
            self.assertEqual(ref.small_pin(pin, 8), b'original')
            for field, value in [('bytes', 7), ('sha256', '0' * 64)]:
                bad = dict(pin, **{field: value})
                with self.subTest(field=field), self.assertRaises(ValueError):
                    ref.small_pin(bad, 8)
            link = Path(directory) / 'alias'
            link.symlink_to(path)
            with self.assertRaises(ValueError):
                ref.small_pin(dict(pin, path=str(link)), 8)

    def test_fixed_production_roster_and_policy_deny_acceptance(self):
        self.assertEqual(len(ref.tensor_shapes(ref.QWEN8B)), 399)
        self.assertEqual(sum(2 * np.prod(shape) for shape in ref.tensor_shapes(ref.QWEN8B).values()), 16381470720)
        policy = ref.json_bytes((Path(ref.__file__).parent / 'policy.json').read_bytes())
        self.assertFalse(policy['numerical_acceptance'])
        self.assertFalse(policy['actual_gpu_intermediate_substitution'])
        self.assertFalse(policy['full_model_correctness'])
        self.assertIsNone(policy['acceptance_threshold'])

    def test_authentic_pin_substitution_and_output_lifecycle_bounds(self):
        size, digest = ref.ORIGINAL_PINS['config.json']
        pin = {'path': '/model/config.json', 'bytes': size, 'sha256': digest}
        ref.original_pin(pin, 'config.json')
        with self.assertRaises(ValueError):
            ref.original_pin(dict(pin, sha256='0' * 64), 'config.json')
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory).resolve() / 'outputs'
            out = ref.Output(path)
            out.emit('finite', np.asarray([0x3f80], dtype='<u2'))
            with self.assertRaises(ValueError):
                out.emit('finite', np.asarray([0], dtype='<u2'))
            with self.assertRaises(ValueError):
                out.emit('nan', np.asarray([0x7fc0], dtype='<u2'))
            with self.assertRaises(ValueError):
                out.emit('../escape', np.asarray([0], dtype='<u2'))
            out.total = ref.MAX_OUTPUT
            with self.assertRaises(ValueError):
                out.emit('overflow', np.asarray([0], dtype='<u2'))
            with self.assertRaises(ValueError):
                ref.Output(path)


if __name__ == '__main__':
    unittest.main()
