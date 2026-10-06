"""Synthetic policy/ratio/chart tests, separate from native qualification."""
import copy
from fractions import Fraction
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

import compare as C
import test_analysis as T


def case(shared, multiplier=1):
    report, checked = T.fixture()
    if shared:
        report['schema'] = 'FerricGuardedMlpSharedFullHostObservationV1'
        checked['schema'] = 'ferric-guarded-mlp-model-host-shared-currentness-checked-v1'
        checked['shared_full_currentness'] = True
        for snapshot in report['snapshots']: snapshot['shared_full_currentness'] = True
    for interval in report['intervals']: interval['host_elapsed_ns'] *= multiplier
    for forward, row in enumerate(checked['forward_rows']):
        begin = 2 + 146 * forward
        row['bracket_host_ns'] *= multiplier
        for layer, values in enumerate(row['layers']):
            for offset, key in enumerate(('prefix', 'paired', 'hidden_read')):
                values[key] = copy.deepcopy(report['intervals'][begin + 4 * layer + offset + 1])
    return dict(report=report, checked=checked, payloads=[bytes(606976)] * 4, history=[9112, 67, 25, 576])


class CompareTests(unittest.TestCase):
    def test_exact_ratios_large_values_zero_and_decimal_ties(self):
        for numerator, denominator in ((1, 2), (2, 1), ((1 << 63) + 3, 7), (0, 7)):
            expected = Fraction(numerator, denominator)
            self.assertEqual(C.ratio(numerator, denominator),
                             dict(numerator=expected.numerator, denominator=expected.denominator))
        self.assertIsNone(C.ratio(7, 0)); self.assertIsNone(C.ratio(0, 0))
        for value in (True, 1.0, -1, 1 << 64):
            with self.assertRaises(ValueError): C.ratio(value, 1)
        self.assertEqual(C.decimal(1, 8, 2), '0.12')
        self.assertEqual(C.decimal(3, 8, 2), '0.38')
        self.assertEqual(C.decimal(-1, 8, 2), '-0.12')

    def test_policy_mapping_preserves_original_reports_and_partition(self):
        baseline, candidate = case(False), case(True)
        original = copy.deepcopy((baseline, candidate))
        result = C.compare(baseline, candidate)
        self.assertEqual((baseline, candidate), original)
        self.assertFalse(result['baseline']['observed_shared_full_currentness'])
        self.assertTrue(result['candidate']['observed_shared_full_currentness'])
        for row in result['rows']:
            self.assertEqual(row['wall']['bracket']['conservative_over_shared'], dict(numerator=1, denominator=1))
        for key in ('baseline', 'candidate'):
            for row in result[key]['forwards']:
                self.assertEqual(row['category_interval_counts'], dict(prefix=36, paired=36, hidden=36, other=37))
                self.assertEqual(sum(row['disjoint_wall_ns'].values()), row['bracket_host_ns'])

    def test_wrong_mode_mixed_policy_and_checked_policy_refuse(self):
        for source_shared, selected_shared in ((False, True), (True, False)):
            value = case(source_shared)
            with self.assertRaises(ValueError): C.account(value['report'], value['checked'], selected_shared)
        for policy in (False, 1, None):
            value = case(True); value['report']['snapshots'][293]['shared_full_currentness'] = policy
            with self.assertRaises(ValueError): C.account(value['report'], value['checked'], True)
        value = case(True); value['checked']['shared_full_currentness'] = False
        with self.assertRaises(ValueError): C.account(value['report'], value['checked'], True)

    def test_payload_or_genuine_history_mismatch_precedes_accounting(self):
        for field in ('payloads', 'history'):
            baseline, candidate = case(False), case(True)
            if field == 'payloads': candidate[field][2] = b'\x01' + candidate[field][2][1:]
            else: candidate[field][2] = 26
            with patch.object(C, 'account', side_effect=AssertionError('accounting must not run')):
                with self.assertRaisesRegex(ValueError, 'payload/history mismatch'):
                    C.compare(baseline, candidate)
        candidate = case(True); candidate['history'][0] = True
        with self.assertRaises(ValueError): C.compare(case(False), candidate)

    def test_slower_candidate_is_reported_without_performance_claim(self):
        result = C.compare(case(False), case(True, 2))
        for row in result['rows']:
            for value in row['wall'].values():
                self.assertEqual(value['conservative_over_shared'], dict(numerator=1, denominator=2))
                self.assertEqual(value['shared_minus_conservative_ns'], value['conservative_ns'])
        for key in ('controlled_repeated_benchmark', 'causal_improvement_established', 'speedup_claim',
                    'gpu_time', 'gpu_overlap', 'throughput', 'numerical_acceptance', 'performance_claim'):
            self.assertFalse(result[key])
        self.assertIn('not a controlled repeated benchmark', C.markdown(result))
        self.assertIn('0.500', C.markdown(result))

    def test_structured_svg_has_exact_eight_disjoint_stacks(self):
        result = C.compare(case(False), case(True, 2)); root = ET.fromstring(C.svg(result))
        ns = {'s': 'http://www.w3.org/2000/svg'}
        self.assertEqual((root.attrib['width'], root.attrib['height'], root.attrib['viewBox']),
                         ('1180', '720', '0 0 1180 720'))
        groups = root.findall('s:g', ns); self.assertEqual(len(groups), 8)
        for index, group in enumerate(groups):
            rects = group.findall('s:rect', ns); self.assertEqual(len(rects), 4)
            self.assertEqual([r.attrib['data-stage'] for r in rects], ['prefix', 'paired', 'hidden', 'other'])
            case_name = 'baseline' if index % 2 == 0 else 'candidate'
            self.assertEqual(sum(int(r.attrib['data-nanoseconds']) for r in rects),
                             result[case_name]['forwards'][index // 2]['bracket_host_ns'])
        self.assertIn('Not GPU time', ''.join(root.itertext()))
        bad = copy.deepcopy(result); bad['same_payloads'] = False
        with self.assertRaises(ValueError): C.svg(bad)
        bad = copy.deepcopy(result); bad['candidate']['forwards'][0]['disjoint_wall_ns']['other'] += 1
        with self.assertRaises(ValueError): C.svg(bad)


if __name__ == '__main__':
    unittest.main(verbosity=2)
