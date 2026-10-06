"""Closed fixture geometry and conditional residual tests, without model I/O."""
import copy
import hashlib
import io
from types import SimpleNamespace
import struct
import unittest
from unittest.mock import patch

import exact_bf16
import fp32_replay
import replay


class ReplayInputsTests(unittest.TestCase):
    def geometry(self):
        capture = SimpleNamespace(SHARD='model.safetensors', HEADER_EXTENT=(128, 'unused'),
                                  SHARD_EXTENT=(136 + 4096 * 4096 * 2, 'unused'))
        header = {replay.WEIGHT_KEY: dict(dtype='BF16', shape=[4096, 4096],
                                        data_offsets=[0, 4096 * 4096 * 2])}
        index = dict(weight_map={replay.WEIGHT_KEY: capture.SHARD})
        return capture, header, index

    def test_full_o_geometry_and_exact_source_offset(self):
        capture, header, index = self.geometry()
        self.assertEqual(replay.geometry(header, index, capture), 136)
        header[replay.WEIGHT_KEY]['data_offsets'] = [8192, 8192 + 4096 * 4096 * 2]
        capture.SHARD_EXTENT = (capture.SHARD_EXTENT[0] + 8192, 'unused')
        self.assertEqual(replay.geometry(header, index, capture), 8328)

    def test_wrong_o_shard_dtype_shape_bias_and_bounds_refuse(self):
        capture, header, index = self.geometry()
        mutations = [dict(dtype='F32'), dict(shape=[2048, 4096]), dict(shape=[4096, 2048]),
                     dict(data_offsets=[-1, 4096 * 4096 * 2 - 1]),
                     dict(data_offsets=[True, 4096 * 4096 * 2 + 1]),
                     dict(data_offsets=[1, 4096 * 4096 * 2 + 1]),
                     dict(data_offsets=[0, 4096 * 4096 * 2 - 2]), dict(extra=0)]
        for fields in mutations:
            bad = copy.deepcopy(header)
            bad[replay.WEIGHT_KEY].update(fields)
            with self.subTest(fields=fields), self.assertRaises(ValueError):
                replay.geometry(bad, index, capture)
        for mapping in ({replay.WEIGHT_KEY: 'wrong'},
                        {**index['weight_map'], 'model.layers.0.self_attn.o_proj.bias': capture.SHARD}):
            with self.assertRaises(ValueError):
                replay.geometry(header, dict(weight_map=mapping), capture)

    def uploads(self):
        registration = dict(layers=[dict(rank=rank, layer=0, weights=[dict(kind='output_projection',
            buffer=dict(rank=rank, id=17 + rank, elements=4096 * 2048, element_bytes=2))]) for rank in (0, 1)])
        uploads = dict(uploads=[dict(key=dict(kind='source', rank=rank, id=17 + rank),
                                    bytes=4096 * 2048 * 2, sha256=str(rank) * 64) for rank in (0, 1)])
        return registration, uploads, SimpleNamespace(wire_sha=lambda value: value)

    def test_rank_columns_join_exact_registered_upload_keys(self):
        registration, uploads, capture = self.uploads()
        result = replay.uploaded_o(registration, uploads, capture)
        self.assertEqual([row['key']['rank'] for row in result], [0, 1])
        self.assertEqual([row['key']['id'] for row in result], [17, 18])
        self.assertEqual([row['sha256'] for row in result], ['0' * 64, '1' * 64])

    def test_duplicate_missing_wrong_rank_and_wrong_extent_upload_refuse(self):
        registration, uploads, capture = self.uploads()
        bad = copy.deepcopy(uploads)
        bad['uploads'].append(copy.deepcopy(bad['uploads'][0]))
        with self.assertRaises(ValueError):
            replay.uploaded_o(registration, bad, capture)
        for field, wrong in (('rank', 1), ('elements', 4096 * 2048 - 1), ('element_bytes', 4)):
            bad = copy.deepcopy(registration)
            bad['layers'][0]['weights'][0]['buffer'][field] = wrong
            with self.subTest(field=field), self.assertRaises(ValueError):
                replay.uploaded_o(bad, uploads, capture)
        for changed in (dict(uploads=uploads['uploads'][:1]),
                        dict(uploads=[{**uploads['uploads'][0], 'bytes': 8192}, uploads['uploads'][1]])):
            with self.assertRaises(ValueError):
                replay.uploaded_o(registration, changed, capture)

    def row_fixture(self):
        attention = struct.pack('<2048H', 0x3f80, *([0] * 2047))
        selected = {}
        for rank, partial in enumerate((0x3f800000, 0x3b800000)):
            selected[(rank, 'attention')] = attention
            selected[(rank, 'output-partial')] = struct.pack('<4096I', partial, *([0] * 4095))
            selected[(rank, 'first-residual')] = bytes(8192)
        reference = dict(embedding=struct.pack('<4096H', 0xbf80, *([0] * 4095)),
                         **{'o-projection': struct.pack('<4096H', 0x3f80, *([0] * 4095)),
                            'first-residual': bytes(8192)})
        weights = struct.pack('<4096H', 0x3f80, *([0] * 2047), 0x3b80, *([0] * 2047))
        uploads = [dict(key=dict(kind='source', rank=rank, id=rank + 1), bytes=4096,
                        sha256=hashlib.sha256(weights[4096 * rank:4096 * (rank + 1)]).hexdigest())
                   for rank in (0, 1)]
        return selected, reference, weights, uploads

    def test_one_row_retains_partial_projection_and_residual_separately(self):
        selected, reference, weights, uploads = self.row_fixture()
        with patch.object(replay, 'ROWS', 1):
            rows, slices = replay.audit_rows(io.BytesIO(weights), 0, selected, reference,
                                            uploads, exact_bf16, fp32_replay)
        row = rows[0]
        self.assertTrue(all(rank['native_matches_fixed_order'] for rank in row['ranks']))
        self.assertEqual(row['native_tp_sum_f32'], '3f808000')
        self.assertEqual(row['native_projection_bf16'], '3f80')
        self.assertEqual(row['conditional_residual_bf16'], '0000')
        self.assertEqual(row['native_residual_matches_conditional_boundary'], [True, True])
        self.assertEqual(len(slices), 2)

    def test_row_refuses_wrong_columns_truncation_and_residual_control(self):
        selected, reference, weights, uploads = self.row_fixture()
        with patch.object(replay, 'ROWS', 1):
            for body, pins in ((weights[:-1], uploads), (weights[4096:] + weights[:4096], uploads),
                               (weights, list(reversed(uploads)))):
                with self.assertRaises(ValueError):
                    replay.audit_rows(io.BytesIO(body), 0, selected, reference, pins, exact_bf16, fp32_replay)
            wrong = dict(reference, **{'first-residual': struct.pack('<4096H', 0x3b80, *([0] * 4095))})
            with self.assertRaises(ValueError):
                replay.audit_rows(io.BytesIO(weights), 0, selected, wrong, uploads, exact_bf16, fp32_replay)


if __name__ == '__main__':
    unittest.main()
