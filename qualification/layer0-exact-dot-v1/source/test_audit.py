"""Synthetic mapping/refusal tests; no model, capture, or oracle execution."""
import copy
import unittest

import audit as A


def fixtures():
    header, index = {}, {'weight_map': {}}
    for letter, _, count, start, end, _ in A.TENSORS:
        key = f'model.layers.0.self_attn.{letter}_proj.weight'
        header[key] = dict(dtype='BF16', shape=[count, 4096], data_offsets=[start, end])
        index['weight_map'][key] = A.SHARD
    return header, index


class AuditTests(unittest.TestCase):
    def test_exact_qkv_geometry_is_admitted(self):
        header, index = fixtures()
        self.assertIsNone(A.tensor_geometry(header, index))

    def test_both_rank_boundaries_and_qkv_capture_offsets(self):
        # Explicit values independently derived from the model header and [Q2048,K512,V512].
        expected = {
            ('q', 0, 0): (0, 1588618872, 0),
            ('q', 0, 168): (168, 1589995128, 168),
            ('q', 0, 2047): (2047, 1605387896, 2047),
            ('q', 1, 0): (2048, 1605396088, 0),
            ('q', 1, 2047): (4095, 1622165112, 2047),
            ('k', 0, 0): (0, 1546675576, 2048),
            ('k', 1, 0): (512, 1550869880, 2048),
            ('k', 1, 511): (1023, 1555055992, 2559),
            ('v', 0, 0): (0, 1622173304, 2560),
            ('v', 1, 0): (512, 1626367608, 2560),
            ('v', 1, 511): (1023, 1630553720, 3071),
        }
        for args, result in expected.items():
            with self.subTest(args=args):
                self.assertEqual(A.row_location(*args), result)

    def test_invalid_rank_row_and_projection_refused(self):
        for args in (('o', 0, 0), ('q', 2, 0), ('q', True, 0), ('q', 0, -1),
                     ('q', 0, 2048), ('k', 1, 512), ('v', 0, False)):
            with self.subTest(args=args), self.assertRaises(ValueError):
                A.row_location(*args)

    def test_wrong_dtype_shape_and_offset_refused(self):
        header, index = fixtures()
        key = 'model.layers.0.self_attn.q_proj.weight'
        for replacement in ({'dtype': 'F16'}, {'shape': [4096, 2048]},
                            {'data_offsets': [1588609538, 1622163970]}):
            bad = copy.deepcopy(header)
            bad[key].update(replacement)
            with self.subTest(replacement=replacement), self.assertRaises(ValueError):
                A.tensor_geometry(bad, index)

    def test_wrong_shard_or_bias_refused(self):
        header, index = fixtures()
        for change in ({'model.layers.0.self_attn.k_proj.weight': 'model-00002-of-00005.safetensors'},
                       {'model.layers.0.self_attn.v_proj.bias': A.SHARD}):
            bad = copy.deepcopy(index)
            bad['weight_map'].update(change)
            with self.subTest(change=change), self.assertRaises(ValueError):
                A.tensor_geometry(header, bad)

    def test_duplicate_json_and_invalid_wire_digest_refused(self):
        with self.assertRaises(ValueError):
            A.parse(b'{"row": 1, "row": 2}')
        with self.assertRaises(ValueError):
            A.parse(b'{"value": NaN}')
        for value in ([0] * 31, [0] * 31 + [True], [0] * 31 + [256]):
            with self.subTest(value=value), self.assertRaises(ValueError):
                A.wire_sha(value)
        self.assertEqual(A.wire_sha([0] * 32), '00' * 32)


if __name__ == '__main__':
    unittest.main(verbosity=2)
