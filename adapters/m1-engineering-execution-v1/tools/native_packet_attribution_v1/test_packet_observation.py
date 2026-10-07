import copy
import importlib.util
import json
from pathlib import Path
import unittest

path = Path(__file__).with_name('packet_observation.py')
p = importlib.util.module_from_spec(importlib.util.spec_from_file_location('packet', path))
p.__spec__.loader.exec_module(p)


def record(count=3, first=0):
    return {'schema': p.SCHEMA, 'clock': 'raw_gpu_clock_ticks', 'device_unique_id': 7,
            'queue_epoch': 0, 'first_write': first, 'next_write': first + count,
            'symbols': {'1': 'a.kd', '2': 'b.kd'},
            'packets': [[1 + i % 2, 100 + i * 10, 107 + i * 10] for i in range(count)]}


def campaign():
    rows = [record(649, i * 649) for i in range(4)]
    rows += [record(652, 2599 + i * 652) for i in range(127)]
    waits = [{'queue_epoch': r['queue_epoch'], 'next_write': r['next_write'],
              'dispatches': len(r['packets'])} for r in rows]
    return rows, waits


class PacketTests(unittest.TestCase):
    def test_disjoint_conservation(self):
        value = p.validate(record())
        self.assertEqual([value[k] for k in ('window_ticks', 'packet_sum_ticks',
            'packet_union_ticks', 'uncovered_ticks', 'overlap_ticks')], [27, 21, 21, 6, 0])

    def test_overlap_union_not_sum(self):
        row = record()
        row['packets'] = [[1, 100, 120], [2, 110, 130], [1, 90, 150]]
        value = p.validate(row)
        self.assertEqual([value[k] for k in ('window_ticks', 'packet_sum_ticks',
            'packet_union_ticks', 'uncovered_ticks', 'overlap_ticks')], [60, 100, 60, 0, 40])

    def test_fields_and_clock(self):
        for field, value in [('schema', 'wrong'), ('clock', 'ns'), ('queue_epoch', True),
                             ('next_write', 2), ('device_unique_id', -1)]:
            row = record()
            row[field] = value
            with self.assertRaises(ValueError): p.validate(row)
        row = record()
        row['extra'] = 1
        with self.assertRaises(ValueError): p.validate(row)

    def test_invalid_ticks(self):
        for packet in ([1, 0, 2], [1, 2, 2], [1, 3, 2], [1, 1, 2**64], [1, True, 2], [1, 2]):
            row = record()
            row['packets'][0] = packet
            with self.assertRaises(ValueError): p.validate(row)

    def test_symbol_roster(self):
        for symbols in ({}, {'01': 'a', '2': 'b'}, {'1': '', '2': 'b'},
                        {'1': 'a', '2': 'b', '3': 'unused'}, {'1': 'a' * 1025, '2': 'b'}):
            row = record()
            row['symbols'] = symbols
            with self.assertRaises(ValueError): p.validate(row)

    def test_bounded_count(self):
        for count in (0, 1025):
            with self.assertRaises(ValueError): p.validate(record(count))

    def test_duplicate_and_nonfinite(self):
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}'):
            with self.assertRaises(ValueError): p.decode(raw)

    def test_exact_stream_preservation(self):
        rows, _ = campaign()
        raw = b'{"legacy":1}\n' + b''.join(json.dumps(r).encode() + b'\n' for r in rows)
        parsed, remaining = p.split_stream(raw)
        self.assertEqual(parsed, rows)
        self.assertEqual(remaining, b'{"legacy":1}\n')
        with self.assertRaises(ValueError): p.split_stream(raw[:-1])

    def test_missing_execution(self):
        with self.assertRaises(ValueError): p.split_stream(json.dumps(record()).encode() + b'\n')

    def test_summary(self):
        rows, waits = campaign()
        result = p.summarize(rows, waits, 7)
        self.assertEqual(result['decode']['executions'], 127)
        self.assertEqual(sum(v['packets'] for v in result['decode']['kernels'].values()), 127 * 652)
        self.assertFalse(result['performance_qualified'])
        self.assertEqual(result['decode']['intervals']['overlap_ticks']['sum'], 0)

    def test_identity_binding(self):
        for field in ('queue_epoch', 'next_write', 'dispatches'):
            rows, waits = campaign()
            waits[8][field] += 1
            with self.assertRaises(ValueError): p.summarize(rows, waits, 7)
        rows, waits = campaign()
        with self.assertRaises(ValueError): p.summarize(rows, waits, 8)

    def test_kernel_roster_drift(self):
        rows, waits = campaign()
        rows[7]['packets'][0][0] = 2
        with self.assertRaises(ValueError): p.summarize(rows, waits, 7)
        rows, waits = campaign()
        rows[7]['symbols']['1'] = 'changed'
        with self.assertRaises(ValueError): p.summarize(rows, waits, 7)

    def test_u64_frontier(self):
        row = record(3, 2**64 - 2)
        with self.assertRaises(ValueError): p.validate(row)


if __name__ == '__main__':
    unittest.main(verbosity=2)
