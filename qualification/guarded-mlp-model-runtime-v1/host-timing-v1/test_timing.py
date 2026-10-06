import struct
import unittest

from timing import CONTROL_BYTES, LAYER_BYTES, STATE_BYTES, counters


class TimingTests(unittest.TestCase):
    def test_wire_extent(self):
        self.assertEqual(CONTROL_BYTES, 242824)
        for raw in (b'', bytes(CONTROL_BYTES - 1), bytes(CONTROL_BYTES + 1),
                    bytearray(CONTROL_BYTES)):
            with self.assertRaises(ValueError):
                counters(raw)

    def test_offsets_do_not_decode_states_as_timing(self):
        raw = bytearray(b'\xff' * CONTROL_BYTES)
        struct.pack_into('<2Q', raw, 0, 11, 12)
        for layer in range(36):
            struct.pack_into('<3Q', raw, 16 + layer * LAYER_BYTES + STATE_BYTES,
                             100 + layer, 200 + layer, 300 + layer)
        struct.pack_into('<3Q', raw, len(raw) - 24, 41, 42, 43)
        row = counters(bytes(raw))
        self.assertEqual(row['embedding_host_ns'], [11, 12])
        self.assertEqual(row['tail_host_ns'], [41, 42, 43])
        self.assertEqual(row['prefix_rank_host_ns'], [sum(range(100, 136)), sum(range(200, 236))])
        self.assertEqual(row['guarded_segments_host_ns'], sum(range(300, 336)))
        self.assertEqual(row['layers'][35]['guarded_segment_host_ns'], 335)

    def test_u64_counters_remain_integers(self):
        row = counters(bytes(b'\xff' * CONTROL_BYTES))
        self.assertEqual(row['guarded_segments_host_ns'], 36 * ((1 << 64) - 1))
        self.assertIs(type(row['guarded_segments_host_ns']), int)

    def test_no_invented_overlap_or_throughput(self):
        row = counters(bytes(CONTROL_BYTES))
        for flag in ('gpu_duration_measured', 'overlap_measured', 'throughput_measured'):
            self.assertIs(row[flag], False)
        self.assertNotIn('total_ns', row)
        self.assertNotIn('tokens_per_second', row)


if __name__ == '__main__':
    unittest.main()
