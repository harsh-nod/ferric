"""Synthetic same-ELF scoped pair gates; no native or timing measurements."""
import copy
import unittest

import test_matched as K
import test_scoped as C
import validate_scoped_pair as P


def setup_pair():
    left, right = copy.deepcopy(K.fixture('default')), copy.deepcopy(C.fixture('scoped_timed'))
    a, b = K.check(left), C.check(right)
    worker = P.rust_pin(left['value'][1]['base']['worker'])
    common = dict(worker=worker,
        worker_cpu=dict(path='/qualified/coupled.json', bytes=123, sha256='a' * 64),
        parent=dict(path='/qualified/parent', bytes=456, sha256='b' * 64),
        parent_cpu=dict(path='/qualified/parent.json', bytes=789, sha256='c' * 64))
    pins = dict(default=copy.deepcopy(common), scoped=copy.deepcopy(common))
    bodies = {**left['value'][2], **right['value'][2]}
    return left, right, a, b, pins, bodies


def compare(value):
    left, right, a, b, pins, bodies = value
    return P.compare_pair(left['summary'], right['summary'], a, b, pins,
        lambda pin: bodies[pin['path']])


class ScopedPairTests(unittest.TestCase):
    def test_same_binary_pair_preserves_original_policy_and_host_timing_without_authority(self):
        value = setup_pair()
        before = copy.deepcopy(value[-1])
        result = compare(value)
        self.assertEqual(before, value[-1])
        self.assertTrue(result['passed'])
        self.assertTrue(result['parity']['all40_records_equal'])
        self.assertTrue(result['parity']['all4_payloads_byte_equal'])
        self.assertEqual(result['scoped_policy']['name'], 'scoped_warm')
        self.assertEqual(result['default_timing']['disjoint_spans'], 124)
        self.assertEqual(result['scoped_timing']['disjoint_spans'], 124)
        self.assertFalse(result['temporal_equivalent_to_full'] or result['shared_full_currentness'])
        for key in ('gpu_timing', 'full_long_workload', 'numerical_acceptance',
            'performance_claim', 'production_authority', 'cpu_qualification_checked',
            'outer_owned_lineage_checked', 'fresh_processes_independently_checked'):
            self.assertIs(result[key], False)

    def test_pair_requires_each_same_actual_cpu_and_product_pin(self):
        for role in ('worker', 'worker_cpu', 'parent', 'parent_cpu'):
            value = setup_pair()
            value[4]['scoped'][role]['sha256'] = 'd' * 64
            with self.subTest(role=role), self.assertRaises(ValueError): compare(value)
        for field, bad in [('path', 'relative'), ('bytes', True), ('bytes', 0), ('sha256', 'x')]:
            value = setup_pair()
            value[4]['default']['parent'][field] = bad
            value[4]['scoped']['parent'][field] = bad
            with self.subTest(field=field), self.assertRaises(ValueError): compare(value)

    def test_pair_rejects_wrong_mode_summary_timeline_and_original_policy_body(self):
        for change in ('mode', 'schema', 'policy', 'summary', 'timeline', 'stderr', 'authority'):
            value = setup_pair()
            checked = value[3]
            if change == 'mode': checked['mode'] = 'scoped'
            elif change == 'schema': checked['schema'] = P.M.SCHEMA
            elif change == 'policy': checked['policy']['name'] = 'shared_full'
            elif change == 'summary': checked['summary']['sha256'] = '0' * 64
            elif change == 'timeline': checked['timing']['timeline']['total_ns'] += 1
            elif change == 'stderr': value[-1][checked['policy']['file']['path']] += b'\n'
            else: checked['performance_claim'] = True
            with self.subTest(change=change), self.assertRaises(ValueError): compare(value)
