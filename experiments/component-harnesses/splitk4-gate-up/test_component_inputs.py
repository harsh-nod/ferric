import copy
import dataclasses
import struct
import unittest

import fixtures as f
import slice_views as s


def metadata(symbol):
    counts = 2 if symbol == s.MERGE else 3
    scalar_count = 0 if symbol == s.MERGE else 5
    arguments = []
    for index in range(counts):
        arguments.extend([
            {'offset': index * 16, 'bytes': 8, 'global_buffer': True,
             'pointee_alignment': None, 'access': None},
            {'offset': index * 16 + 8, 'bytes': 8, 'global_buffer': False},
        ])
    for index in range(scalar_count):
        arguments.append({'offset': counts * 16 + index * 4, 'bytes': 4, 'global_buffer': False})
    implicit = 32 if symbol == s.MERGE else 72
    return {'symbol': symbol, 'wavefront_size': 64, 'private_segment_bytes': 0,
        'group_segment_bytes': 0, 'kernarg_alignment': 8, 'explicit_arguments': arguments,
        'kernarg_bytes': implicit + 256, 'implicit_argument_offset': implicit,
        'implicit_argument_bytes': 256}


class Inputs(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixture = f.build_fixture()
        fixture = cls.fixture
        cls.records = {
            'a': {'id': 1, 'data': fixture.input_bf16, 'element_bytes': 2},
            'nk': {'id': 2, 'data': fixture.weights_nk_bf16, 'element_bytes': 2},
            'kn': {'id': 3, 'data': fixture.weights_kn_bf16, 'element_bytes': 2},
            'scratch': {'id': 4, 'data': fixture.partials_f32, 'element_bytes': 4},
            'output': {'id': 5, 'data': b'\xa5' * f.OUTPUT_CAPACITY_BYTES, 'element_bytes': 2},
        }

    def views(self, symbol):
        keys = {s.WAVE: ('a', 'nk', 'output'), s.PARTIAL: ('a', 'kn', 'scratch'),
                s.MERGE: ('scratch', 'output')}[symbol]
        return [s.view(self.records[key], 'write' if index == len(keys) - 1 else 'read')
                for index, key in enumerate(keys)]

    def encode(self, symbol, tag=4, meta=None, views=None, groups=None, scalars=None):
        return s.encode(metadata(symbol) if meta is None else meta,
            self.views(symbol) if views is None else views,
            (() if symbol == s.MERGE else (1, 12288, 4096, 1, tag)) if scalars is None else scalars,
            s.SHAPES[symbol][0] if groups is None else groups)

    def test_complete_extents(self):
        self.assertEqual(tuple(map(len, dataclasses.astuple(self.fixture))),
                         (8192, 100663296, 100663296, 196608, 24576))
        self.assertEqual(f.OUTPUT_CAPACITY_BYTES, 786432)

    def test_both_weight_layouts_match_independent_coordinates(self):
        for inner in (0, 15, 16, 18, 19, 1023, 1024, 2047, 2048, 3071, 3072, 4095):
            for column in (0, 15, 16, 18, 19, 255, 256, 4095, 4096, 8192, 12287):
                expected = f.bf16(((3 * inner + 5 * column) % 19 - 9) / 16)
                kn = 2 * (inner * 12288 + column)
                nk = 2 * (column * 4096 + inner)
                self.assertEqual(self.fixture.weights_kn_bf16[kn:kn + 2], expected)
                self.assertEqual(self.fixture.weights_nk_bf16[nk:nk + 2], expected)
        self.assertNotEqual(self.fixture.weights_kn_bf16, self.fixture.weights_nk_bf16)

    def test_partials_and_output_match_independent_integer_accumulation(self):
        for column in (0, 15, 16, 255, 256, 4095, 4096, 8191, 8192, 12287):
            numerator = 0
            for partition in range(4):
                expected = 0
                for inner in range(partition * 1024, partition * 1024 + 1024):
                    expected += (inner % 17 - 8) * ((3 * inner + 5 * column) % 19 - 9)
                self.assertEqual(struct.unpack_from('<f', self.fixture.partials_f32,
                                 4 * (partition * 12288 + column))[0], expected / 256)
                numerator += expected
            self.assertEqual(self.fixture.output_bf16[2 * column:2 * column + 2], f.bf16(numerator / 256))

    def test_input_is_exact_row_zero(self):
        for inner in range(4096):
            self.assertEqual(self.fixture.input_bf16[2 * inner:2 * inner + 2], f.bf16((inner % 17 - 8) / 16))

    def test_invalid_fixture_coordinates_and_patterns_fail(self):
        for partition, column in ((-1, 0), (4, 0), (True, 0), (0, -1), (0, 12288), (0, 1.0)):
            with self.assertRaises(ValueError):
                f.partial_numerator(partition, column)
        for pattern, elements, width in ((b'', 1, 2), (b'x', 1, 2), (b'xx', 1, 4),
                (b'xxxx', 0, 4), (b'xxxx', True, 4), (b'xxxx', 1, True), (b'xxxx', 1, 8)):
            with self.assertRaises(ValueError):
                f.repeat(pattern, elements, width)
        self.assertEqual(f.repeat(b'abcd', 3, 2), b'abcdab')
        self.assertEqual(f.repeat(b'abcdefgh', 3, 4), b'abcdefghabcd')

    def test_bf16_rounding_and_nonfinite_rejection(self):
        for value, expected in ((1 + 1 / 256, 0x3f80), (1 + 3 / 256, 0x3f82),
                (-1 - 1 / 256, 0xbf80), (-1 - 3 / 256, 0xbf82)):
            self.assertEqual(f.bf16(value), struct.pack('<H', expected))
        for value in (float('nan'), float('inf'), -float('inf'), struct.unpack('<f', bytes.fromhex('ffff7f7f'))[0]):
            with self.assertRaises(ValueError):
                f.bf16(value)

    def test_exact_output_validation_rejects_corruption(self):
        f.validate_output(self.fixture.output_bf16, self.fixture.output_bf16, 'output')
        for actual in (self.fixture.output_bf16[:-1], b'', bytearray(self.fixture.output_bf16),
                       b'xx' + self.fixture.output_bf16[2:]):
            with self.assertRaises(ValueError):
                f.validate_output(actual, self.fixture.output_bf16, 'output')

    def test_all_root_encodings_have_exact_lengths_offsets_and_zero_hidden_bytes(self):
        for symbol in (s.WAVE, s.PARTIAL, s.MERGE):
            for tag in (4, 5):
                encoded, pointers = self.encode(symbol, tag)
                self.assertEqual(len(encoded), 288 if symbol == s.MERGE else 328)
                self.assertEqual(encoded[-256:], bytes(256))
                for index, (pointer, item) in enumerate(zip(pointers, self.views(symbol), strict=True)):
                    self.assertEqual(pointer, {'kernarg_offset': 16 * index,
                        'buffer': item['record']['id'], 'buffer_offset': 64,
                        'extent_bytes': len(item['record']['data']), 'access': item['access']})
                    self.assertEqual(encoded[16 * index:16 * index + 8], bytes(8))
                    self.assertEqual(struct.unpack_from('<Q', encoded, 16 * index + 8)[0],
                                     len(item['record']['data']) // item['record']['element_bytes'])
                if symbol != s.MERGE:
                    self.assertEqual(struct.unpack_from('<5I', encoded, 48), (1, 12288, 4096, 1, tag))
                    self.assertEqual(encoded[68:72], bytes(4))

    def test_closed_root_geometry_and_projection_tags(self):
        for symbol in (s.WAVE, s.PARTIAL, s.MERGE):
            for groups in (0, 4096, True, s.SHAPES[symbol][0] + 1):
                with self.assertRaises(ValueError):
                    self.encode(symbol, groups=groups)
        for symbol in (s.WAVE, s.PARTIAL):
            for tag in (0, 2, 3, 6, True, 4.0):
                with self.assertRaises(ValueError):
                    self.encode(symbol, tag)
        altered = metadata(s.WAVE)
        altered['symbol'] += '_unknown'
        with self.assertRaises(ValueError):
            self.encode(s.WAVE, meta=altered)

    def test_exact_scalar_rosters(self):
        for scalars in ((), (2, 12288, 4096, 1, 4), (1, 4096, 12288, 1, 4),
                        (1, 12288, 4096, 2, 4), [1, 12288, 4096, 1, 4]):
            with self.assertRaises(ValueError):
                self.encode(s.PARTIAL, scalars=scalars)
        with self.assertRaises(ValueError):
            self.encode(s.MERGE, scalars=(4,))

    def test_alias_element_width_extent_and_access_drift(self):
        for field, value in (('id', self.records['kn']['id']), ('element_bytes', 2), ('data', b'\0' * 4)):
            views = self.views(s.PARTIAL)
            views[2] = {'record': {**views[2]['record'], field: value}, 'access': 'write'}
            with self.assertRaises(ValueError):
                self.encode(s.PARTIAL, views=views)
        for index in range(3):
            views = self.views(s.PARTIAL)
            views[index]['access'] = 'read' if index == 2 else 'write'
            with self.assertRaises(ValueError):
                self.encode(s.PARTIAL, views=views)

    def test_loaded_resource_metadata_drift(self):
        for field, value in (('wavefront_size', 32), ('private_segment_bytes', 16),
                ('group_segment_bytes', 256), ('kernarg_alignment', 4), ('kernarg_bytes', 327),
                ('implicit_argument_offset', 68), ('implicit_argument_bytes', 248)):
            altered = metadata(s.PARTIAL)
            altered[field] = value
            with self.assertRaises(ValueError):
                self.encode(s.PARTIAL, meta=altered)

    def test_pointer_length_and_scalar_metadata_drift(self):
        for slot, field, value in ((0, 'offset', 8), (0, 'global_buffer', False),
                (0, 'access', 'write'), (0, 'pointee_alignment', 4), (1, 'bytes', 4),
                (1, 'offset', 9), (4, 'pointee_alignment', 2), (6, 'global_buffer', True),
                (6, 'offset', 49), (6, 'bytes', 8)):
            altered = metadata(s.PARTIAL)
            altered['explicit_arguments'][slot][field] = value
            with self.assertRaises(ValueError):
                self.encode(s.PARTIAL, meta=altered)
        altered = metadata(s.PARTIAL)
        altered['explicit_arguments'].pop()
        with self.assertRaises(ValueError):
            self.encode(s.PARTIAL, meta=altered)

    def test_views_are_closed_and_records_remain_unchanged(self):
        before = copy.deepcopy(self.records['scratch'])
        self.encode(s.MERGE)
        self.assertEqual(self.records['scratch'], before)
        for record in ({**before, 'extra': 1}, {**before, 'id': True},
                       {**before, 'data': bytearray(before['data'])}):
            with self.assertRaises(ValueError):
                s.view(record, 'read')
        views = self.views(s.MERGE)
        views[0]['offset'] = 4
        with self.assertRaises(ValueError):
            self.encode(s.MERGE, views=views)


if __name__ == '__main__':
    unittest.main()
