"""Synthetic mapping and diagnostic classification; no Torch/model/native imports."""
import struct
import unittest

import compare as C
import observer as O


def fixture():
    native, framework = [], []
    pages = list(reversed(range(144)))
    for position in range(6):
        stages = {}
        for name, shape in O.shapes(position).items():
            size = 2
            for dimension in shape:
                size *= dimension
            stages[name] = bytes(size)
        ranks = []
        for _ in range(2):
            rank = {name: bytes(size if size is not None else 1024 * (position + 1))
                    for name, size in C.NATIVE_EXTENTS.items()}
            rank['cache_metadata'] = struct.pack('<145I', position, *pages)
            ranks.append(rank)
        native.append(dict(position=position, input_token=900 + position, ranks=ranks))
        framework.append(dict(position=position, input_token=900 + position, stages=stages))
    return native, framework


def word(raw, offset, value=0x3f80):
    result = bytearray(raw)
    struct.pack_into('<H', result, offset, value)
    return bytes(result)


def row(result, position, rank, role):
    return next(value for value in result['comparable_rows']
                if (value['position'], value['rank'], value['role']) == (position, rank, role))


class ComparisonTests(unittest.TestCase):
    def test_closed204_rows_and_no_acceptance_or_partial_synthesis(self):
        result = C.compare(*fixture())
        self.assertEqual(len(result['comparable_rows']), 204)
        self.assertEqual(len(result['noncomparable_rows']), 36)
        self.assertEqual(result['exact_rows'], 204)
        self.assertIsNone(result['earliest_observed_difference'])
        self.assertIsNone(result['earliest_same_captured_input_difference'])
        for key in ('receipt_authentication', 'same_side_parity_verified_here', 'partials_summed_or_rounded',
                    'numerical_acceptance', 'semantic_bug_claimed', 'full_model_correctness', 'performance_measured'):
            self.assertIs(result[key], False)
        self.assertIsNone(result['acceptance_threshold'])

    def test_qkv_rank_slices_and_gate_up_halves_are_not_interleaved(self):
        native, framework = fixture()
        stages = framework[0]['stages']
        for name, count, values in [('q-projection', 4096, (0x3f80, 0x4000)),
                                    ('k-projection', 1024, (0x4040, 0x4080)),
                                    ('v-projection', 1024, (0x40a0, 0x40c0)),
                                    ('gate', 12288, (0x40e0, 0x4100)),
                                    ('up', 12288, (0x4110, 0x4120))]:
            stages[name] = b''.join(struct.pack('<H', value) * (count // 2) for value in values)
        stages['q-norm-input'] = stages['q-projection']
        stages['k-norm-input'] = stages['k-projection']
        stages['silu-input'] = stages['gate']
        # Keep the synthetic framework/native own-cache prefix valid through all six positions.
        for p in range(6):
            fs = framework[p]['stages']
            fs['cache-value'] = b''.join((struct.pack('<H', 0x40a0 if head < 4 else 0x40c0)
                if token == 0 else b'\0\0') * 128 for head in range(8) for token in range(p + 1))
            for rank in range(2):
                native[p]['ranks'][rank]['used_value'] = struct.pack('<H', 0x40a0 if rank == 0 else 0x40c0) * 512 + bytes(p * 1024)
        for rank, values in enumerate(((0x3f80, 0x4040, 0x40a0, 0x40e0, 0x4110),
                                       (0x4000, 0x4080, 0x40c0, 0x4100, 0x4120))):
            n = native[0]['ranks'][rank]
            n['raw_qkv'] = b''.join(struct.pack('<H', value) * count for value, count in zip(values[:3], (2048, 512, 512)))
            n['gate'], n['up'] = (struct.pack('<H', value) * 6144 for value in values[3:])
        result = C.compare(native, framework)
        for rank in range(2):
            for name in ('q_projection', 'k_projection', 'v_projection', 'gate', 'up'):
                self.assertEqual(row(result, 0, rank, name)['different_words'], 0)
        native[0]['ranks'][1]['raw_qkv'] = native[0]['ranks'][0]['raw_qkv']
        result = C.compare(native, framework)
        self.assertEqual(row(result, 0, 1, 'q_projection')['different_words'], 2048)
        self.assertEqual(row(result, 0, 1, 'k_projection')['different_words'], 512)

    def test_framework_head_major_cache_permutation_and_current_row(self):
        native, framework = fixture()
        for position in range(6):
            stages = framework[position]['stages']
            for key, current, shift in [('cache-key', 'rotary-k', 0), ('cache-value', 'v-projection', 256)]:
                stages[key] = b''.join(struct.pack('<H', 0x3f80 + shift + 16 * head + token) * 128
                    for head in range(8) for token in range(position + 1))
                stages[current] = b''.join(struct.pack('<H', 0x3f80 + shift + 16 * head + position) * 128
                    for head in range(8))
                role = 'used_key' if key == 'cache-key' else 'used_value'
                for rank in range(2):
                    native[position]['ranks'][rank][role] = b''.join(
                        struct.pack('<H', 0x3f80 + shift + 16 * head + token) * 128
                        for token in range(position + 1) for head in range(rank * 4, rank * 4 + 4))
        result = C.compare(native, framework)
        for position in range(6):
            for rank in range(2):
                for role in ('used_key', 'used_value', 'current_key', 'current_value'):
                    self.assertEqual(row(result, position, rank, role)['different_words'], 0)
        stages = framework[5]['stages']
        expected = b''.join(struct.pack('<H', 0x3f80 + 16 * head + token) * 128
                           for token in range(6) for head in range(4, 8))
        self.assertEqual(C.reference_rank(stages, 5, 1)['used_key'], expected)
        self.assertNotEqual(expected, stages['cache-key'][4 * 6 * 256:])

    def test_same_captured_inputs_are_distinct_from_propagation(self):
        native, framework = fixture()
        n = native[0]['ranks'][0]
        n['raw_qkv'] = word(n['raw_qkv'], 0)
        n['query'] = word(n['query'], 0)
        n['attention'] = word(n['attention'], 0)
        result = C.compare(native, framework)
        q = row(result, 0, 0, 'q_projection')
        self.assertTrue(q['same_captured_inputs'])
        self.assertEqual(q['operation_scope'], 'q_projection')
        query = row(result, 0, 0, 'query')
        self.assertFalse(query['same_captured_inputs'])
        self.assertIn('0:0:q_projection', query['differing_prerequisites'])
        self.assertIn('comparable_rotary_values', query['unresolved_prerequisites'])
        self.assertEqual(row(result, 0, 0, 'attention')['input_relation'], 'propagated_input_difference_possible')
        self.assertEqual(result['earliest_observed_difference']['stage'], 'qkv_projection')
        self.assertEqual(result['earliest_same_captured_input_difference']['row_ids'], ['0:0:q_projection'])

    def test_tp_composite_dependencies_include_both_ranks(self):
        native, framework = fixture()
        native[0]['ranks'][1]['attention'] = word(native[0]['ranks'][1]['attention'], 0)
        native[0]['ranks'][0]['first_residual'] = word(native[0]['ranks'][0]['first_residual'], 0)
        native[0]['ranks'][1]['activation'] = word(native[0]['ranks'][1]['activation'], 0)
        native[0]['ranks'][0]['final_hidden'] = word(native[0]['ranks'][0]['final_hidden'], 0)
        result = C.compare(native, framework)
        self.assertIn('0:1:attention', row(result, 0, 0, 'first_residual')['differing_prerequisites'])
        self.assertIn('0:1:activation', row(result, 0, 0, 'final_hidden')['differing_prerequisites'])
        self.assertFalse(row(result, 0, 0, 'first_residual')['same_captured_inputs'])
        self.assertFalse(row(result, 0, 0, 'final_hidden')['same_captured_inputs'])

    def test_unavailable_rotary_and_partial_rows_never_become_full_projections(self):
        native, framework = fixture()
        for rank in native[0]['ranks']:
            rank['output_partial'] = struct.pack('<f', 100.0) * 4096
            rank['down_partial'] = struct.pack('<f', -100.0) * 4096
            rank['rotary'] = struct.pack('<f', 1.0) * 128
        result = C.compare(native, framework)
        self.assertEqual(result['exact_rows'], 204)
        self.assertTrue(all(value['metrics'] is None and value['comparable'] is False
                            for value in result['noncomparable_rows']))
        self.assertFalse(row(result, 0, 0, 'query')['same_captured_inputs'])
        self.assertIn('o-projection', result['framework_only_uncompared'])

    def test_product_is_composite_and_requires_both_gate_and_up(self):
        native, framework = fixture()
        n = native[0]['ranks'][0]; n['activation'] = word(n['activation'], 0)
        result = C.compare(native, framework)
        self.assertTrue(row(result, 0, 0, 'activation')['same_captured_inputs'])
        self.assertEqual(row(result, 0, 0, 'activation')['operation_scope'], 'silu_times_up_composite')
        n['up'] = word(n['up'], 0)
        result = C.compare(native, framework)
        self.assertFalse(row(result, 0, 0, 'activation')['same_captured_inputs'])
        self.assertEqual(row(result, 0, 0, 'activation')['differing_prerequisites'], ['0:0:up'])

    def test_earliest_stage_groups_parallel_qkv_and_preserves_record_order(self):
        native, framework = fixture()
        for offset in (0, 4096):
            native[2]['ranks'][1]['raw_qkv'] = word(native[2]['ranks'][1]['raw_qkv'], offset)
        native[3]['ranks'][0]['input'] = word(native[3]['ranks'][0]['input'], 0)
        result = C.compare(native, framework)
        first = result['earliest_observed_difference']
        self.assertEqual(first['position'], 2)
        self.assertEqual(set(first['row_ids']), {'2:1:q_projection', '2:1:k_projection'})
        self.assertEqual(first['stage'], 'qkv_projection')

    def test_closed_scope_extents_finiteness_and_cache_ancestry(self):
        edits = [lambda n, f: n.pop(), lambda n, f: n[0].__setitem__('position', True),
            lambda n, f: f[1].__setitem__('input_token', 123),
            lambda n, f: n[0]['ranks'][0].__setitem__('extra', b''),
            lambda n, f: n[0]['ranks'][0].__setitem__('raw_qkv', bytes(6142)),
            lambda n, f: n[0]['ranks'][0].__setitem__('input', b'\x80\x7f' + bytes(8190)),
            lambda n, f: n[0]['ranks'][0].__setitem__('output_partial', struct.pack('<I', 0x7fc00000) + bytes(16380)),
            lambda n, f: n[5]['ranks'][1].__setitem__('used_key', word(n[5]['ranks'][1]['used_key'], 0)),
            lambda n, f: f[0]['stages'].__setitem__('input-norm', bytes(8190))]
        for edit in edits:
            with self.subTest(edit=edits.index(edit)):
                n, f = fixture(); edit(n, f)
                with self.assertRaises(ValueError): C.compare(n, f)

    def test_frozen_metrics_keep_zero_norm_signed_zero_and_exact_word_counts(self):
        native, framework = fixture()
        n = native[0]['ranks'][0]
        n['input_normalized'] = word(n['input_normalized'], 0, 0x8000)
        n['gate'] = word(n['gate'], 0, 0x3f80)
        result = C.compare(native, framework)
        zero = row(result, 0, 0, 'input_normalized')['metrics']
        self.assertEqual(zero['exact_words'], 4095)
        self.assertEqual(zero['max_abs_error'], 0.0)
        self.assertEqual(zero['relative_l2'], 0.0)
        metric = row(result, 0, 0, 'gate')['metrics']
        self.assertTrue(metric['zero_reference_norm'])
        self.assertIsNone(metric['relative_l2'])
        self.assertEqual(metric['exact_words'], 6143)
        self.assertEqual(metric['max_abs_error'], 1.0)


if __name__ == '__main__':
    unittest.main()
