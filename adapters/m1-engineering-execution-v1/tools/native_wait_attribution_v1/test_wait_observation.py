"""CPU-only negative and boundary fixtures for native wait replay."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

PATH = Path(__file__).with_name('wait_observation.py')
SPEC = importlib.util.spec_from_file_location('wait_observation', PATH)
w = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(w)


def record():
    return {
        'schema': w.SCHEMA, 'clock': w.CLOCK, 'queue_epoch': 0,
        'next_write': 649, 'dispatches': 649, 'execution_succeeded': True,
        'observation_valid': True, 'publication_return_ns': 1000,
        'execution_return_ns': 50200, 'signal_reads': 2, 'pending_reads': 1,
        'completed_reads': 1, 'unexpected_reads': 0,
        'last_pending_read_ns': [49000, 49010],
        'first_completed_read_ns': [50000, 50010],
        'completion_bracket_ns': [49000, 50010], 'max_inter_read_ns': 1010,
        'signal_read_ns': 20, 'pause_count': 1, 'pause_elapsed_ns': 900,
        'max_pause_ns': 900, 'post_read_ns': 100,
        'currentness_nested_ns': 10, 'retirement_signals_ns': 20,
        'poll_ready_ns': 50100,
    }


def records():
    rows = []
    for count, next_write in ([(649, 649 * (i + 1)) for i in range(4)] +
                              [(652, 2599 + 652 * (i + 1)) for i in range(127)]):
        row = record()
        row.update(dispatches=count, next_write=next_write)
        rows.append(row)
    return rows


def encode(rows):
    return b''.join(json.dumps(row).encode() + b'\n' for row in rows)


class WaitObservationTests(unittest.TestCase):
    def test_expected_intervals(self):
        result = w.validate(record())
        self.assertEqual(result['publication_to_signal_lower_ns'], 48000)
        self.assertEqual(result['publication_to_signal_upper_ns'], 49010)
        self.assertEqual(result['signal_observation_uncertainty_ns'], 1010)
        self.assertEqual(result['signal_to_execution_return_lower_ns'], 190)
        self.assertEqual(result['signal_to_execution_return_upper_ns'], 1200)

    def test_first_read_completed_is_conservative(self):
        row = record()
        row.update(signal_reads=1, pending_reads=0, last_pending_read_ns=None,
                   completion_bracket_ns=[0, 50010], max_inter_read_ns=0,
                   pause_count=0, pause_elapsed_ns=0, max_pause_ns=0, signal_read_ns=10)
        result = w.validate(row)
        self.assertEqual(result['publication_to_signal_lower_ns'], 0)
        self.assertEqual(result['signal_to_execution_return_upper_ns'], 50200)
        row['completion_bracket_ns'][0] = row['publication_return_ns']
        with self.assertRaises(ValueError):
            w.validate(row)

    def test_invalid_or_failed_records_reject(self):
        for key in ('execution_succeeded', 'observation_valid'):
            for value in (False, 0, 1, 'true', None):
                row = record()
                row[key] = value
                with self.assertRaises(ValueError):
                    w.validate(row)

    def test_closed_schema_and_clock(self):
        for change in ({'extra': 0}, {'schema': 'v2'}, {'clock': 'gpu_nanoseconds'}):
            with self.assertRaises(ValueError):
                w.validate({**record(), **change})
        row = record()
        del row['retirement_signals_ns']
        with self.assertRaises(ValueError):
            w.validate(row)

    def test_integer_types_and_bounds(self):
        for key in w.NUMBERS | {'publication_return_ns', 'poll_ready_ns'}:
            for bad in (True, -1, 2**64, 1.5, '1', None):
                row = record()
                row[key] = bad
                with self.assertRaises(ValueError, msg=key):
                    w.validate(row)

    def test_json_duplicate_nonfinite_and_malformed(self):
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}', b'{"a":Infinity}', b'{'):
            with self.assertRaises(ValueError):
                w.decode(raw)

    def test_windows_and_event_order(self):
        for key in ('last_pending_read_ns', 'first_completed_read_ns', 'completion_bracket_ns'):
            for bad in (None, [1], [1, 2, 3], [2, 1], [True, 2], [0, 50201]):
                row = record()
                row[key] = bad
                with self.assertRaises(ValueError):
                    w.validate(row)
        for key, value in (('publication_return_ns', 49900), ('poll_ready_ns', 49999)):
            with self.assertRaises(ValueError):
                w.validate({**record(), key: value})

    def test_completion_bracket_and_inter_read_bound(self):
        for key, value in (('completion_bracket_ns', [49001, 50010]),
                           ('completion_bracket_ns', [49000, 50009]),
                           ('max_inter_read_ns', 1009), ('max_inter_read_ns', 50201)):
            with self.assertRaises(ValueError):
                w.validate({**record(), key: value})

    def test_poll_counts_and_pauses(self):
        for key, value in (('signal_reads', 3), ('completed_reads', 2),
                           ('unexpected_reads', 1), ('pause_count', 0),
                           ('pause_elapsed_ns', 899), ('pause_elapsed_ns', 901)):
            with self.assertRaises(ValueError):
                w.validate({**record(), key: value})

    def test_buckets_must_fit_execution(self):
        for key, value in (('signal_read_ns', 19), ('post_read_ns', 89),
                           ('currentness_nested_ns', 101), ('retirement_signals_ns', 101),
                           ('signal_read_ns', 50200)):
            with self.assertRaises(ValueError):
                w.validate({**record(), key: value})

    def test_pause_is_not_subtracted_from_device_interval(self):
        first = w.validate(record())
        row = record()
        row.update(pause_elapsed_ns=10000, max_pause_ns=10000)
        second = w.validate(row)
        for key in first:
            if key != 'pause_elapsed_ns':
                self.assertEqual(first[key], second[key])

    def test_summary_exact_geometry_and_scope(self):
        rows = records()
        before = copy.deepcopy(rows)
        result = w.summarize(rows, 0)
        self.assertEqual(rows, before)
        self.assertEqual(result['decode']['execution_return_ns']['sum'], 127 * 50200)
        self.assertEqual(result['prefill']['execution_return_ns']['sum'], 4 * 50200)
        for key in ('performance_qualified', 'latency_sample_admitted', 'gpu_time_measured'):
            self.assertIs(result[key], False)

    def test_wrong_geometry_epoch_or_record_count(self):
        for index in (0, 3, 4, 130):
            for key in ('queue_epoch', 'dispatches', 'next_write'):
                rows = records()
                rows[index][key] += 1
                with self.assertRaises(ValueError):
                    w.summarize(rows, 0)
        for rows in (records()[:-1], records() + [record()], list(reversed(records()))):
            with self.assertRaises(ValueError):
                w.summarize(rows, 0)

    def test_no_cross_execution_clock_comparison(self):
        rows = records()
        rows[20]['execution_return_ns'] += 100
        self.assertEqual(w.summarize(rows, 0)['decode']['execution_return_ns']['max'], 50300)

    def test_split_preserves_legacy_bytes_and_order(self):
        legacy = [b'{ "start" : true }\n', b'{"breakdown":1}\n', b'{"end":true}\n']
        source = legacy[0] + encode(records()) + b''.join(legacy[1:])
        rows, old = w.split_stream(source)
        self.assertEqual(rows, records())
        self.assertEqual(old, b''.join(legacy))

    def test_split_rejects_partial_extra_or_missing_records(self):
        source = b'{}\n' + encode(records()) + b'{}\n{}\n'
        for bad in (source[:-1], source + b'{}\n', encode(records()), source + encode([record()]),
                    b'\n' + source, b'not-json\n' + source, b'[]\n' + source,
                    b'x' * (w.MAX_STREAM + 1)):
            with self.assertRaises(ValueError):
                w.split_stream(bad)

    def test_oversized_record_rejected(self):
        rows = records()
        huge = json.dumps(rows[0]).encode() + b' ' * w.MAX_RECORD + b'\n'
        with self.assertRaises(ValueError):
            w.split_stream(huge + encode(rows[1:]) + b'{}\n{}\n{}\n')

    def test_nearest_rank_statistics(self):
        result = w.describe(list(range(1, 21)))
        self.assertEqual(result, {'sum': 210, 'mean': 10.5, 'median': 10.5,
                                  'max': 20, 'p95_nearest_rank': 19})


if __name__ == '__main__':
    unittest.main(verbosity=2)
