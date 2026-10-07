import math
import struct
import unittest

import fixtures
import slice_views as views


def metadata(buffers):
    arguments = []
    for i in range(buffers):
        arguments.extend(({'offset': i * 16, 'bytes': 8, 'global_buffer': True,
                           'pointee_alignment': 2, 'access': 'read' if i < 2 else 'write'},
                          {'offset': i * 16 + 8, 'bytes': 8, 'global_buffer': False}))
    arguments.extend({'offset': buffers * 16 + i * 4, 'bytes': 4, 'global_buffer': False}
                     for i in range(5))
    implicit = (buffers * 16 + 20 + 7) // 8 * 8
    return {'wavefront_size': 64, 'private_segment_bytes': 0, 'group_segment_bytes': 0,
            'kernarg_alignment': 8, 'explicit_arguments': arguments,
            'kernarg_bytes': implicit + 256, 'implicit_argument_offset': implicit,
            'implicit_argument_bytes': 256}


class ComponentInputs(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixture = fixtures.build_fixture()
        cls.records = [dict(id=i + 1, data=data, element_bytes=2) for i, data in enumerate(
            (cls.fixture.input_bf16, cls.fixture.weights_kn_bf16, b'\xa5' * 786432))]

    def plans(self, paired):
        a, b, out = self.records
        result = [views.view(a, 'read'), views.view(b, 'read')]
        return result + ([views.view(out, 'write', 0, 393216), views.view(out, 'write', 393216, 393216)]
                         if paired else [views.view(out, 'write')])

    def test_fixture_lengths_and_exact_inputs(self):
        f = self.fixture
        self.assertEqual((len(f.input_bf16), len(f.weights_kn_bf16), len(f.output_bf16)),
                         (262144, 100663296, 786432))
        for row in (0, 15, 16, 31):
            for inner in (0, 16, 17, 2048, 4095):
                offset = (row * 4096 + inner) * 2
                self.assertEqual(f.input_bf16[offset:offset + 2], fixtures.bf16((row + 1) * (inner % 17 - 8) / 16))
        for inner in (0, 18, 19, 2048, 4095):
            for col in (0, 18, 19, 6143, 12287):
                offset = (inner * 12288 + col) * 2
                self.assertEqual(f.weights_kn_bf16[offset:offset + 2], fixtures.bf16(((3 * inner + 5 * col) % 19 - 9) / 16))

    def test_reference_and_half_boundary(self):
        for row in (0, 15, 16, 31):
            for column in (0, 18, 19, 12287):
                # Decode the actual input bytes, not the periodic reference helper.
                total = 0.0
                for inner in range(4096):
                    left = self.fixture.input_bf16[2 * (row * 4096 + inner):2 * (row * 4096 + inner) + 2]
                    right = self.fixture.weights_kn_bf16[2 * (inner * 12288 + column):2 * (inner * 12288 + column) + 2]
                    total += struct.unpack('<f', b'\0\0' + left)[0] * struct.unpack('<f', b'\0\0' + right)[0]
                offset = 2 * (row * 12288 + column)
                self.assertEqual(self.fixture.output_bf16[offset:offset + 2], fixtures.bf16(total))
        low, high = self.fixture.output_bf16[:393216], self.fixture.output_bf16[393216:]
        self.assertNotEqual(low, high)
        with self.assertRaises(ValueError):
            fixtures.validate_output(low + low, self.fixture.output_bf16, 'duplicated half')

    def test_bf16_ties_finite_and_pattern_validation(self):
        self.assertEqual(fixtures.bf16(1.00390625), struct.pack('<H', 0x3f80))
        self.assertEqual(fixtures.bf16(1.01171875), struct.pack('<H', 0x3f82))
        self.assertEqual(fixtures.bf16(-1.00390625), struct.pack('<H', 0xbf80))
        self.assertEqual(fixtures.bf16(-1.01171875), struct.pack('<H', 0xbf82))
        self.assertEqual(fixtures.bf16(0.0), b'\0\0')
        self.assertEqual(fixtures.bf16(-0.0), b'\0\x80')
        with self.assertRaises(ValueError):
            fixtures.bf16(struct.unpack('<f', struct.pack('<I', 0x7f7fffff))[0])
        for value in (math.inf, -math.inf, math.nan):
            with self.assertRaises(ValueError):
                fixtures.bf16(value)
        for pattern, size in ((b'', 1), (b'a', 1), (b'aa', True), (b'aa', 0)):
            with self.assertRaises(ValueError):
                fixtures.repeat(pattern, size)

    def test_full_and_split_abi_both_projection_roles(self):
        for split in (False, True):
            count = 4 if split else 3
            for tag in (4, 5):
                data, pointers = views.encode(metadata(count), self.plans(split), (32, 12288, 4096, 1, tag), 768 if split else 1536)
                self.assertEqual(len(data), 344 if split else 328)
                self.assertEqual([p['buffer'] for p in pointers], [1, 2, 3, 3] if split else [1, 2, 3])
                self.assertEqual([p['buffer_offset'] for p in pointers], [64, 64, 64, 393280] if split else [64, 64, 64])
                self.assertEqual(struct.unpack_from('<Q', data, 8)[0], 131072)
                self.assertEqual(struct.unpack_from('<Q', data, 24)[0], 50331648)
                self.assertEqual(struct.unpack_from('<Q', data, 40)[0], 196608 if split else 393216)
                if split:
                    self.assertEqual(struct.unpack_from('<Q', data, 56)[0], 196608)
                self.assertEqual(data[(88 if split else 72):], bytes(256))
                for index in range(count):
                    self.assertEqual(data[index * 16:index * 16 + 8], bytes(8))
                self.assertEqual(struct.unpack_from('<IIIII', data, count * 16), (32, 12288, 4096, 1, tag))

    def test_overlap_gap_reversal_and_independent_outputs_rejected(self):
        for change in ('overlap', 'gap', 'reverse', 'separate'):
            plan = self.plans(True)
            if change == 'overlap':
                plan[3]['offset'] = 0
            elif change == 'gap':
                plan[3]['offset'] += 2
                plan[3]['extent'] -= 2
            elif change == 'reverse':
                plan[2:] = list(reversed(plan[2:]))
            else:
                plan[3]['record'] = dict(plan[3]['record'], id=4)
            with self.subTest(change=change), self.assertRaises(ValueError):
                views.encode(metadata(4), plan, (32, 12288, 4096, 1, 4), 768)

    def test_invalid_views_rejected(self):
        for offset, extent in ((-2, 2), (1, 2), (0, 1), (0, 0), (786432, 2), (True, 2), (0, True)):
            with self.subTest(offset=offset, extent=extent), self.assertRaises(ValueError):
                views.view(self.records[2], 'write', offset, extent)

    def test_input_alias_and_access_rejected(self):
        for mutate in ('alias', 'access', 'padded'):
            plan = self.plans(True)
            if mutate == 'alias':
                plan[0]['record'] = dict(plan[0]['record'], id=3)
            elif mutate == 'access':
                plan[0]['access'] = 'write'
            else:
                plan[0]['record'] = dict(plan[0]['record'], data=plan[0]['record']['data'] + b'\0\0')
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                views.encode(metadata(4), plan, (32, 12288, 4096, 1, 4), 768)

    def test_stale_or_malformed_abi_rejected(self):
        for key, value in (('kernarg_bytes', 340), ('implicit_argument_offset', 84),
                           ('wavefront_size', 32), ('private_segment_bytes', 4),
                           ('group_segment_bytes', 4), ('kernarg_alignment', 4)):
            info = metadata(4)
            info[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                views.encode(info, self.plans(True), (32, 12288, 4096, 1, 4), 768)
        for key, value in (('offset', 48), ('bytes', 4), ('global_buffer', True)):
            info = metadata(4)
            info['explicit_arguments'][7][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                views.encode(info, self.plans(True), (32, 12288, 4096, 1, 4), 768)
        for key, value in (('offset', 40), ('bytes', 4), ('access', 'read'), ('pointee_alignment', 4)):
            info = metadata(4)
            info['explicit_arguments'][6][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                views.encode(info, self.plans(True), (32, 12288, 4096, 1, 4), 768)

    def test_wrong_scalars_and_launch_rejected(self):
        for scalars in ((16, 12288, 4096, 1, 4), (32, 12288, 4096, 2, 4),
                        (32, 12288, 4096, 1, 2), (32, 12288, 4096, True, 4)):
            with self.subTest(scalars=scalars), self.assertRaises(ValueError):
                views.encode(metadata(4), self.plans(True), scalars, 768)
        for groups in (1536, 767, 769, True):
            with self.subTest(groups=groups), self.assertRaises(ValueError):
                views.encode(metadata(4), self.plans(True), (32, 12288, 4096, 1, 4), groups)


if __name__ == '__main__':
    unittest.main()
