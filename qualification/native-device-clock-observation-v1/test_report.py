"""Synthetic tests of added raw-counter reporting only; no native capture fixture."""
import copy
import unittest

import report as R


def fixture():
    device = dict(group_incarnation=41, ranks=[dict(unique_id=11, queue_epoch=21),
                                            dict(unique_id=12, queue_epoch=22)])
    samples = []
    for index in range(16):
        position, slot = divmod(index, 4)
        rank, post = slot % 2, slot >= 2
        samples.append(dict(generation=position + 1, position=position,
            endpoint='post' if post else 'pre', rank=rank,
            row_boundary=(position + int(post)) * 293, group_incarnation=41,
            unique_id=11 + rank, queue_epoch=21 + rank, gpu_id=101 + rank,
            gpu_clock_counter=1000 + index * 7, cpu_clock_counter=2000 + index * 11,
            system_clock_counter=3000 + index * 13, system_clock_frequency_hz=1000000000,
            host_started_ns=index * 10, host_finished_ns=index * 10 + 5))
    value = dict(schema='FerricPrefixDecodeDeviceClockObservationV2', raw=device,
                 samples=samples, raw_clock_counters=True, **{key: False for key in R.CLOCK_FALSE})
    return value, device


class RawClockReportTests(unittest.TestCase):
    def test_exact_sixteen_samples_produce_eight_same_rank_integer_differences(self):
        value, device = fixture()
        samples, deltas = R.clock_samples(value, device)
        self.assertIs(samples, value['samples'])
        self.assertEqual(len(deltas), 8)
        self.assertEqual([(r['position'], r['rank']) for r in deltas],
                         [(position, rank) for position in range(4) for rank in range(2)])
        for row in deltas:
            self.assertEqual(row['post_sample'], row['pre_sample'] + 2)
            self.assertEqual(row['gpu_clock_counter_signed_difference'], 14)
            self.assertEqual(row['cpu_clock_counter_signed_difference'], 22)
            self.assertEqual(row['system_clock_counter_signed_difference'], 26)
            self.assertEqual(row['host_bracket_span_ns'], 25)
            self.assertEqual(row['pre_sample_host_span_ns'], 5)
            self.assertEqual(row['post_sample_host_span_ns'], 5)

    def test_zero_decreasing_and_full_width_raw_values_are_not_converted_or_repaired(self):
        value, device = fixture()
        for index, sample in enumerate(value['samples']):
            sample.update(gpu_clock_counter=15 - index, cpu_clock_counter=0,
                          system_clock_counter=(1 << 64) - 1 if index < 2 else 0)
        _, deltas = R.clock_samples(value, device)
        for row in deltas:
            self.assertEqual(row['gpu_clock_counter_signed_difference'], -2)
            self.assertIs(row['gpu_clock_counter_decreased'], True)
            self.assertEqual(row['cpu_clock_counter_signed_difference'], 0)
            self.assertIs(row['cpu_clock_counter_decreased'], False)
        self.assertEqual(deltas[0]['system_clock_counter_signed_difference'], -((1 << 64) - 1))
        self.assertIs(type(deltas[0]['system_clock_counter_signed_difference']), int)
        self.assertIs(value['clock_domain_validated'], False)

    def test_integer_types_exact_roster_and_native_identity_are_required(self):
        value, device = fixture()
        for key in set(value['samples'][0]) - {'endpoint'}:
            for invalid in (True, 0.0, -1, 1 << 64):
                bad = copy.deepcopy(value); bad['samples'][0][key] = invalid
                with self.subTest(key=key, invalid=invalid), self.assertRaises(ValueError):
                    R.clock_samples(bad, device)
        for key in ('generation', 'position', 'rank', 'row_boundary', 'group_incarnation',
                    'unique_id', 'queue_epoch', 'gpu_id'):
            bad = copy.deepcopy(value); bad['samples'][7][key] += 1
            with self.subTest(identity=key), self.assertRaises(ValueError):
                R.clock_samples(bad, device)
        for changed in (value['samples'][:-1], value['samples'] + [value['samples'][0]],
                        [value['samples'][1], value['samples'][0], *value['samples'][2:]]):
            with self.assertRaises(ValueError):
                R.clock_samples(dict(value, samples=changed), device)

    def test_closed_flags_frequencies_and_host_bracket_order_are_required(self):
        value, device = fixture()
        for key in R.CLOCK_FALSE:
            with self.subTest(flag=key), self.assertRaises(ValueError):
                R.clock_samples(dict(value, **{key: True}), device)
        for index, key, invalid in ((0, 'system_clock_frequency_hz', 0),
                (4, 'system_clock_frequency_hz', 999), (1, 'gpu_id', 101),
                (0, 'gpu_id', 1 << 32), (7, 'host_started_ns', 0), (7, 'host_finished_ns', 69)):
            bad = copy.deepcopy(value); bad['samples'][index][key] = invalid
            with self.subTest(index=index, key=key), self.assertRaises(ValueError):
                R.clock_samples(bad, device)
        with self.assertRaises(ValueError): R.clock_samples(dict(value, extra=0), device)
        bad = copy.deepcopy(value); bad['samples'][0]['extra'] = 0
        with self.assertRaises(ValueError): R.clock_samples(bad, device)

    def test_markdown_keeps_counter_differences_and_host_spans_explicit(self):
        value, device = fixture()
        samples, deltas = R.clock_samples(value, device)
        text = '\n'.join(R.clock_markdown(samples, deltas))
        self.assertIn('Signed post-minus-pre integer differences only', text)
        self.assertIn('not a GPU tick frequency', text)
        self.assertIn('not GPU compute alone', text)
        self.assertIn('KFD GPU ID 101', text)
        self.assertIn('| 0 | 0 | 14 | 22 | 26 | 25 |', text)


if __name__ == '__main__': unittest.main()
