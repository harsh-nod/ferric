"""Synthetic report-consumer tests; not native or retained-pair qualification."""
import copy
import csv
import io
import json
import unittest
import xml.etree.ElementTree as ET

import paired_analysis as A
import validate_timing as T


def timeline(step):
    cursor = 0
    def span():
        nonlocal cursor
        start = cursor; cursor += step
        return dict(start_ns=start, end_ns=cursor, elapsed_ns=step)
    value = dict(source_preparation=span(), spawn_to_setup_seal=span(), forwards=[])
    for position in range(40):
        value['forwards'].append(dict(position=position, generation=position + 1,
            prepare_write=span(), flush_to_frame_read=span(), validate_retain_commit=span(), elapsed_ns=3 * step))
    value.update(close_and_retirement=span(), postcheck_and_ordinary_publication=span(), total_ns=cursor)
    return value


def fixture(default_step=10, scoped_step=5):
    metadata = {name: dict(path='/synthetic/' + name, bytes=1, sha256='a' * 64)
                for name in ('parent', 'parent_cpu', 'worker', 'worker_cpu')}
    cases = {}
    for mode, step in (('default', default_step), ('scoped', scoped_step)):
        timing = dict(file=dict(path='/synthetic/' + mode + '/host-timing.json', bytes=1, sha256='b' * 64),
            timeline=timeline(step), disjoint_spans=124, parent_host_measurement=True,
            gpu_timing=False, nested_control_timers_included=False, sidecar_publication_timed=False,
            full_long_workload=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
        checked = dict(schema=('ferric-readiness40-matched-timed-case-data-v1' if mode == 'default'
                else 'ferric-readiness40-scoped-warm-case-data-v1'),
            mode='default' if mode == 'default' else 'scoped_timed',
            policy=dict(name='default_full') if mode == 'default' else dict(name='scoped_warm',
                policy_record={'original': 'synthetic-scoped-record'},
                no_policy_bytes_discarded=True, temporal_equivalent_to_full=False,
                shared_full_currentness=False), timing=timing)
        cases[mode] = dict(present=True, original_passed=True, retained_success_revalidated=True,
            readiness_completed_forwards=40, semantic_and_payload_parity_revalidated=True,
            matched_timing=checked, original_terminal=dict(name='complete.json', bytes=1, sha256='c' * 64))
    parity = dict(passed=True, records=list(range(40)), captures=[0, 5, 16, 39],
        all40_records_equal=True, all40_observation_pins_equal=True, all40_logit_pins_equal=True,
        all4_payloads_byte_equal=True, controls_individually_validated=True,
        controls_byte_equality_claimed=False, independent_numerical_reference=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
    pair = dict(schema='ferric-readiness40-scoped-matched-timed-parity-v1', passed=True, parity=parity,
        same_elf_cpu_metadata=metadata, default_timing=cases['default']['matched_timing']['timing'],
        scoped_timing=cases['scoped']['matched_timing']['timing'],
        scoped_policy=copy.deepcopy(cases['scoped']['matched_timing']['policy']),
        temporal_equivalent_to_full=False, shared_full_currentness=False)
    return dict(passed=True, selected_original_bodies=239, raw_files=147, baseline_raw_files=70,
        baseline_data_revalidated=True, both_original_owned_lineages_revalidated=True,
        matched_semantic_and_payload_parity_revalidated=True, parent_host_measurement=True,
        original_policy_bytes_retained=True, scoped_warm_case_retained=True,
        shared_full_currentness_requested=False, currentness_temporal_equivalence_claim=False,
        gpu_timing=False, native_rerun=False, gpu_execution=False, model_execution=False,
        numerical_reference_replayed=False, numerical_acceptance=False, full_long_workload=False,
        performance_claim=False, production_authority=False, cases=cases, matched_pair=pair,
        cpu_and_elf_metadata={name: {key: pin[key] for key in ('bytes', 'sha256')} for name, pin in metadata.items()},
        checker_cpu=dict(path='/synthetic/checker', bytes=1, sha256='d' * 64))


class PairedAnalysisTests(unittest.TestCase):
    def test_disjoint_categories_positions_and_observed_ratio(self):
        result = A.summarize(fixture(), T.timeline)
        self.assertEqual(len(result['categories']), 7)
        self.assertEqual(len(result['positions']), 40)
        self.assertEqual(result['total'], dict(default_ns=1240, scoped_ns=620, difference_ns=-620,
            scoped_over_default='0.500000', change_percent='-50.000000'))
        self.assertEqual(sum(row['default_ns'] for row in result['categories']), 1240)
        self.assertEqual(sum(row['scoped_ns'] for row in result['categories']), 620)
        self.assertEqual([row['position'] for row in result['positions'] if row['captured']], [0, 5, 16, 39])
        self.assertFalse(result['gpu_timing'])
        self.assertFalse(result['general_speedup_claim'])
        self.assertFalse(result['temporal_equivalent_to_full'])
        self.assertFalse(result['shared_full_currentness'])
        self.assertEqual(result['currentness_policy'], 'scoped_warm')

    def test_integer_nanoseconds_beyond_float_precision_and_decimal_ties(self):
        size = (1 << 53) + 1
        result = A.summarize(fixture(size, size + 1), T.timeline)
        self.assertEqual(result['total']['default_ns'], 124 * size)
        self.assertEqual(result['total']['difference_ns'], 124)
        self.assertEqual(result['positions'][17]['prepare_write']['difference_ns'], 1)
        self.assertEqual(A.decimal_ratio(1, 2000000), '0.000000')
        self.assertEqual(A.decimal_ratio(3, 2000000), '0.000002')

    def test_zero_denominators_are_na_not_infinite_claims(self):
        result = A.summarize(fixture(0, 1), T.timeline)
        self.assertIsNone(result['total']['scoped_over_default'])
        self.assertIsNone(result['total']['change_percent'])
        outputs = A.render(fixture(0, 0), T.timeline)
        self.assertIn(b'n/a', outputs['table.md'])
        self.assertNotIn(b'Infinity', outputs['summary.json'])
        ET.fromstring(outputs['waits.svg'])

    def test_refuses_missing_pair_parity_identity_or_false_authority(self):
        edits = [lambda v: v.__setitem__('passed', False),
            lambda v: v.__setitem__('both_original_owned_lineages_revalidated', False),
            lambda v: v['matched_pair']['parity'].__setitem__('all4_payloads_byte_equal', False),
            lambda v: v['matched_pair']['same_elf_cpu_metadata']['parent'].__setitem__('sha256', 'e' * 64),
            lambda v: v.__setitem__('gpu_timing', True),
            lambda v: v.__setitem__('selected_original_bodies', 237),
            lambda v: v.__setitem__('shared_full_currentness_requested', True),
            lambda v: v.__setitem__('currentness_temporal_equivalence_claim', True),
            lambda v: v['matched_pair'].__setitem__('temporal_equivalent_to_full', True),
            lambda v: v['matched_pair']['scoped_policy']['policy_record'].__setitem__('original', 'changed'),
            lambda v: v['cases']['scoped']['matched_timing'].__setitem__('mode', 'scoped'),
            lambda v: v['cases']['scoped']['matched_timing']['policy'].__setitem__('name', 'shared_full'),
            lambda v: v['cases']['scoped']['matched_timing']['policy'].__setitem__('name', 'default_full')]
        for edit in edits:
            with self.subTest(edit=edit):
                value = fixture(); before = A.encoded(value); edit(value)
                self.assertNotEqual(before, A.encoded(value))
                with self.assertRaises((ValueError, KeyError, TypeError)):
                    A.summarize(value, T.timeline)

    def test_rechecks_timeline_order_bool_gap_and_total(self):
        for edit in (lambda t: t.__setitem__('total_ns', True),
                     lambda t: t['forwards'][4]['flush_to_frame_read'].__setitem__('start_ns', 0),
                     lambda t: t['forwards'][39].__setitem__('generation', 39),
                     lambda t: t.__setitem__('total_ns', 999)):
            value = fixture(); before = A.encoded(value)
            edit(value['cases']['scoped']['matched_timing']['timing']['timeline'])
            self.assertNotEqual(before, A.encoded(value))
            with self.assertRaises((ValueError, KeyError, TypeError)):
                A.summarize(value, T.timeline)

    def test_csv_markdown_and_fixed_svg_preserve_original_values(self):
        value = fixture(); original = copy.deepcopy(value)
        outputs = A.render(value, T.timeline)
        self.assertEqual(value, original)
        self.assertEqual(set(outputs), {'summary.json', 'categories.csv', 'positions.csv', 'table.md', 'waits.svg'})
        categories = list(csv.reader(io.StringIO(outputs['categories.csv'].decode())))
        positions = list(csv.reader(io.StringIO(outputs['positions.csv'].decode())))
        self.assertEqual(len(categories), 9)
        self.assertEqual(len(positions), 41)
        self.assertEqual(categories[-1][0:4], ['Total', '1240', '620', '-620'])
        self.assertEqual(positions[-1][:3], ['39', '40', 'True'])
        self.assertEqual(json.loads(outputs['summary.json'])['total']['default_ns'], 1240)
        svg = ET.fromstring(outputs['waits.svg']); ns = {'s': 'http://www.w3.org/2000/svg'}
        self.assertEqual((svg.attrib['width'], svg.attrib['height']), ('1060', '540'))
        lines = svg.findall('.//s:polyline', ns)
        self.assertEqual(len(lines), 2)
        for line in lines:
            points = [tuple(map(float, pair.split(','))) for pair in line.attrib['points'].split()]
            self.assertEqual(len(points), 40)
            self.assertTrue(all(86 <= x <= 1026 and 108 <= y <= 464 for x, y in points))
        self.assertEqual(len(svg.findall('.//s:circle', ns)), 8)
        self.assertIn(b'ScopedWarm', outputs['waits.svg'])
        self.assertNotIn(b'SharedFull', outputs['waits.svg'])

    def test_no_nested_timers_and_sidecar_timeline_mismatch_refused(self):
        value = fixture(); value['cases']['scoped']['matched_timing']['timing']['nested_control_timers_included'] = True
        with self.assertRaises(ValueError): A.summarize(value, T.timeline)
        value = fixture(); value['matched_pair']['scoped_timing'] = copy.deepcopy(value['matched_pair']['scoped_timing'])
        value['matched_pair']['scoped_timing']['timeline'] = timeline(9)
        with self.assertRaises(ValueError): A.summarize(value, T.timeline)


if __name__ == '__main__':
    unittest.main()
