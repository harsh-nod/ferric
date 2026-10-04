"""Synthetic retained-data policy tests; no GPU, process launch, or calibration."""
import copy
from fractions import Fraction
import unittest

import report as R


def fixture(image=R.DOWN2, delta=10):
    wire = list(bytes.fromhex(image[1]))
    request = dict(device_ids=[123, 456], session=[3] * 32, worker={'sha256': [4] * 32})
    raw = dict(schema='FerricPrefixDecodeDeviceObservationV1', native_closed=True,
        raw_completion_ticks=True, shared_full_currentness=True, cache_kernel_admission=False,
        operational_currentness=False, final_dispatches=[592, 580], group_incarnation=9,
        ranks=[dict(rank=rank, unique_id=request['device_ids'][rank], queue_epoch=rank + 1) for rank in range(2)],
        bootstrap=dict(device_ids=request['device_ids'], scope={'session': request['session']},
                       tiles_image=dict(bytes=image[0], sha256=wire)),
        worker_sha256=request['worker']['sha256'],
        images={name: wire for name in ('prefix', 'mlp', 'residual', 'tail', 'copy')}, rows=[],
        **{key: False for key in R.FALSE if key != 'clock_domain_validated'})
    packets = [0, 0]
    for i in range(1172):
        position, stage, layer, rank = R.coordinate(i)
        raw['rows'].append(dict(position=position, generation=position + 1, stage=stage, layer=layer,
            rank=rank, unique_id=request['device_ids'][rank], queue_epoch=rank + 1, group_incarnation=9,
            packet_id=packets[rank], signal_generation=packets[rank] + 1,
            image_sha256=wire, entry='ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2',
            start_tick=100, end_tick=100 + delta, host_elapsed_ns=1000 + delta))
        packets[rank] += 1
    return raw, request


class Report(unittest.TestCase):
    def test_full_1172_rows_include_288_exact_mlp_image_bindings(self):
        raw, request = fixture()
        ticks, host = R.selected_rows(raw, request, R.DOWN2)
        self.assertEqual(len(ticks), 13)
        self.assertEqual(len(ticks[0, 'mlp']), 144)
        self.assertEqual(len(ticks[1, 'mlp']), 144)
        self.assertEqual(sum(map(len, ticks.values())), 1172)
        self.assertEqual(host[0, 'mlp'], [1010] * 144)

    def test_zero_raw_deltas_are_retained(self):
        raw, request = fixture(delta=0)
        ticks, _ = R.selected_rows(raw, request, R.DOWN2)
        self.assertEqual(ticks[1, 'mlp'].count(0), 144)

    def test_one_wrong_mlp_image_fails_even_if_counts_match(self):
        raw, request = fixture()
        next(row for row in raw['rows'] if row['stage'] == 'mlp')['image_sha256'] = [1] * 32
        with self.assertRaises(ValueError):
            R.selected_rows(raw, request, R.DOWN2)

    def test_wrong_bootstrap_image_extent_or_hash_fails(self):
        for key, replacement in (('bytes', 1), ('sha256', [1] * 32)):
            raw, request = fixture()
            raw['bootstrap']['tiles_image'][key] = replacement
            with self.assertRaises(ValueError):
                R.selected_rows(raw, request, R.DOWN2)

    def test_missing_reordered_or_wrong_rank_packet_fails(self):
        for change in ('missing', 'order', 'packet'):
            raw, request = fixture()
            if change == 'missing': raw['rows'].pop()
            elif change == 'order': raw['rows'][2], raw['rows'][3] = raw['rows'][3], raw['rows'][2]
            else: raw['rows'][8]['packet_id'] += 1
            with self.assertRaises(ValueError):
                R.selected_rows(raw, request, R.DOWN2)

    def test_wrong_queue_group_worker_or_session_fails(self):
        for change in ('queue', 'group', 'worker', 'session'):
            raw, request = fixture()
            if change == 'queue': raw['rows'][8]['queue_epoch'] += 1
            elif change == 'group': raw['rows'][8]['group_incarnation'] += 1
            elif change == 'worker': raw['worker_sha256'] = [1] * 32
            else: raw['bootstrap']['scope']['session'] = [1] * 32
            with self.assertRaises(ValueError):
                R.selected_rows(raw, request, R.DOWN2)

    def test_decreasing_or_zero_endpoint_and_noninteger_host_time_fail(self):
        for key, value in (('start_tick', 0), ('end_tick', 99), ('host_elapsed_ns', True), ('host_elapsed_ns', 1.0)):
            raw, request = fixture()
            raw['rows'][0][key] = value
            with self.assertRaises(ValueError):
                R.selected_rows(raw, request, R.DOWN2)

    def test_numerical_or_performance_claim_is_rejected(self):
        for key in ('performance_claim', 'numerical_acceptance', 'production_authority'):
            raw, request = fixture()
            raw[key] = True
            with self.assertRaises(ValueError):
                R.selected_rows(raw, request, R.DOWN2)

    def test_signed_half_integer_display_does_not_floor_negative_differences(self):
        for value, expected in ((Fraction(-3, 2), '-1.5'), (Fraction(-1, 2), '-0.5'),
                                (Fraction(3, 2), '1.5'), (0, '0'), (-2, '-2')):
            self.assertEqual(R.number(value), expected)

    def test_descriptive_summary_contains_absolute_values_not_ratios(self):
        old = dict(ticks={(0, 'mlp'): [0, 1]}, host={(0, 'mlp'): [100, 101]})
        new = dict(ticks={(0, 'mlp'): [0, 0]}, host={(0, 'mlp'): [99, 99]})
        row = R.summary(old, new)[0]
        self.assertEqual(row['ticks']['signed_median_difference'], '-0.5')
        self.assertEqual(row['host']['signed_median_difference'], '-1.5')
        self.assertEqual(row['ticks']['candidate_zero_deltas'], 2)
        self.assertNotIn('speedup', row)
        text = R.markdown([row])
        self.assertIn('not a speedup', text)
        self.assertIn('not GPU durations', text)

    def test_same_case_join_allows_only_tiles_image_session_and_output(self):
        request = dict(mode='teacher_forced', session=[1], evidence_directory='/old',
                       tiles_image='old', worker='unchanged', model='unchanged')
        old = dict(pin={'sha256': 'old'}, complete={'selected_runtime': {'worker': 1}}, request=request,
                   ticks={(0, 'mlp'): [1]}, selected_image='old')
        new = dict(complete={'baseline': old['pin'], 'selected_runtime': old['complete']['selected_runtime']},
                   request=dict(request, session=[2], evidence_directory='/new', tiles_image='new'),
                   ticks={(0, 'mlp'): [1]}, selected_image='new')
        R.same_cases(old, new)
        for key in ('worker', 'model'):
            bad = copy.deepcopy(new)
            bad['request'][key] = 'changed'
            with self.assertRaises(ValueError): R.same_cases(old, bad)
        bad = copy.deepcopy(new); bad['complete']['baseline'] = {}
        with self.assertRaises(ValueError): R.same_cases(old, bad)

    def test_strict_sha_json_and_uint_parsers(self):
        self.assertEqual(R.digest([0] * 32), '0' * 64)
        for value in ([True] * 32, [256] * 32, 'not-a-sha'):
            with self.assertRaises(ValueError): R.digest(value)
        for value in (True, -1, 1 << 64, 1.0):
            with self.assertRaises(ValueError): R.uint(value)
        for value in ('{"a":1,"a":2}', '{"a":NaN}'):
            with self.assertRaises(ValueError): R.parse(value)


if __name__ == '__main__':
    unittest.main(verbosity=2)
