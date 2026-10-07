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


def fixture(bank_step=10, census_step=5):
    metadata = {name: dict(path='/synthetic/' + name, bytes=1, sha256='a' * 64)
                for name in ('parent', 'parent_cpu', 'worker', 'worker_cpu')}
    cases = {}
    for mode, step in (('bank', bank_step), ('census', census_step)):
        timing = dict(file=dict(path='/synthetic/' + mode + '/host-timing.json', bytes=1, sha256='b' * 64),
            timeline=timeline(step), disjoint_spans=124, parent_host_measurement=True,
            gpu_timing=False, nested_control_timers_included=False, sidecar_publication_timed=False,
            full_long_workload=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
        checked = dict(schema=('ferric-readiness40-bank-scoped-warm-case-data-v2' if mode == 'bank'
                else 'ferric-readiness40-bank-scoped-census-case-data-v3'),
            mode='bank_timed' if mode == 'bank' else 'census_timed',
            policy=dict(name='bank_scoped_warm' if mode == 'bank' else 'bank_scoped_census',
                policy_record={'original': 'synthetic-'+mode+'-record'},
                no_policy_bytes_discarded=True, original_stderr_empty=False,
                temporal_equivalent_to_full=False, shared_full_currentness=False), timing=timing)
        if mode == 'census':
            checked.update(allocation_preflights_changed=True, census_counters_are_layer_subset=True)
        cases[mode] = dict(present=True, original_passed=True, retained_success_revalidated=True,
            readiness_completed_forwards=40, semantic_and_payload_parity_revalidated=True,
            matched_timing=checked, original_terminal=dict(name='complete.json', bytes=1, sha256='c' * 64))
    parity = dict(passed=True, records=list(range(40)), captures=[0, 5, 16, 39],
        all40_records_equal=True, all40_observation_pins_equal=True, all40_logit_pins_equal=True,
        all4_payloads_byte_equal=True, controls_individually_validated=True,
        controls_byte_equality_claimed=False, independent_numerical_reference=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
    pair = dict(schema='ferric-readiness40-bank-scoped-census-matched-timed-parity-v3', passed=True, parity=parity,
        same_elf_cpu_metadata=metadata, bank_timing=cases['bank']['matched_timing']['timing'],
        census_timing=cases['census']['matched_timing']['timing'],
        bank_policy=copy.deepcopy(cases['bank']['matched_timing']['policy']),
        census_policy=copy.deepcopy(cases['census']['matched_timing']['policy']),
        allocation_preflights_changed=True, census_counters_are_layer_subset=True, temporal_equivalent_to_full=False, shared_full_currentness=False)
    return dict(passed=True, selected_original_bodies=242, raw_files=147, baseline_raw_files=70,
        baseline_data_revalidated=True, both_original_owned_lineages_revalidated=True,
        matched_semantic_and_payload_parity_revalidated=True, parent_host_measurement=True,
        original_policy_bytes_retained=True, census_scoped_case_retained=True,
        bank_scoped_rearm_case_retained=True, allocation_preflights_changed=True, census_counters_are_layer_subset=True,
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
        self.assertEqual(result['total'], dict(bank_ns=1240, census_ns=620, difference_ns=-620,
            census_over_bank='0.500000', change_percent='-50.000000'))
        self.assertEqual(sum(row['bank_ns'] for row in result['categories']), 1240)
        self.assertEqual(sum(row['census_ns'] for row in result['categories']), 620)
        self.assertEqual([row['position'] for row in result['positions'] if row['captured']], [0, 5, 16, 39])
        self.assertFalse(result['gpu_timing'])
        self.assertFalse(result['general_speedup_claim'])
        self.assertFalse(result['temporal_equivalent_to_full'])
        self.assertFalse(result['shared_full_currentness'])
        self.assertEqual(result['currentness_policies'], dict(bank='bank_scoped_warm', census='bank_scoped_census'))

    def test_integer_nanoseconds_beyond_float_precision_and_decimal_ties(self):
        size = (1 << 53) + 1
        result = A.summarize(fixture(size, size + 1), T.timeline)
        self.assertEqual(result['total']['bank_ns'], 124 * size)
        self.assertEqual(result['total']['difference_ns'], 124)
        self.assertEqual(result['positions'][17]['prepare_write']['difference_ns'], 1)
        self.assertEqual(A.decimal_ratio(1, 2000000), '0.000000')
        self.assertEqual(A.decimal_ratio(3, 2000000), '0.000002')

    def test_zero_denominators_are_na_not_infinite_claims(self):
        result = A.summarize(fixture(0, 1), T.timeline)
        self.assertIsNone(result['total']['census_over_bank'])
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
            lambda v: v['matched_pair']['bank_policy']['policy_record'].__setitem__('original', 'changed'),
            lambda v: v['cases']['census']['matched_timing'].__setitem__('mode', 'census'),
            lambda v: v['cases']['census']['matched_timing']['policy'].__setitem__('name', 'shared_full'),
            lambda v: v['cases']['census']['matched_timing']['policy'].__setitem__('name', 'default_full')]
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
            edit(value['cases']['census']['matched_timing']['timing']['timeline'])
            self.assertNotEqual(before, A.encoded(value))
            with self.assertRaises((ValueError, KeyError, TypeError)):
                A.summarize(value, T.timeline)

    def test_csv_markdown_and_fixed_svg_preserve_original_values(self):
        value = fixture(); original = copy.deepcopy(value)
        outputs = A.render(value, T.timeline)
        self.assertEqual(value, original)
        self.assertEqual(set(outputs), {'summary.json', 'categories.csv', 'positions.csv', 'forward-groups.csv', 'table.md', 'waits.svg', 'groups.svg'})
        categories = list(csv.reader(io.StringIO(outputs['categories.csv'].decode())))
        positions = list(csv.reader(io.StringIO(outputs['positions.csv'].decode())))
        self.assertEqual(len(categories), 9)
        self.assertEqual(len(positions), 41)
        self.assertEqual(categories[-1][0:4], ['Total', '1240', '620', '-620'])
        self.assertEqual(positions[-1][:3], ['39', '40', 'True'])
        self.assertEqual(json.loads(outputs['summary.json'])['total']['bank_ns'], 1240)
        svg = ET.fromstring(outputs['waits.svg']); ns = {'s': 'http://www.w3.org/2000/svg'}
        self.assertEqual((svg.attrib['width'], svg.attrib['height']), ('1060', '540'))
        lines = svg.findall('.//s:polyline', ns)
        self.assertEqual(len(lines), 2)
        for line in lines:
            points = [tuple(map(float, pair.split(','))) for pair in line.attrib['points'].split()]
            self.assertEqual(len(points), 40)
            self.assertTrue(all(86 <= x <= 1026 and 108 <= y <= 464 for x, y in points))
        self.assertEqual(len(svg.findall('.//s:circle', ns)), 8)
        self.assertIn(b'Census V3', outputs['waits.svg'])
        self.assertNotIn(b'SharedFull', outputs['waits.svg'])

    def test_no_nested_timers_and_sidecar_timeline_mismatch_refused(self):
        value = fixture(); value['cases']['census']['matched_timing']['timing']['nested_control_timers_included'] = True
        with self.assertRaises(ValueError): A.summarize(value, T.timeline)
        value = fixture(); value['matched_pair']['census_timing'] = copy.deepcopy(value['matched_pair']['census_timing'])
        value['matched_pair']['census_timing']['timeline'] = timeline(9)
        with self.assertRaises(ValueError): A.summarize(value, T.timeline)


    def test_forward_groups_reconcile_without_adding_overlapping_totals(self):
        result = A.summarize(fixture(), T.timeline)
        first, warm, all_forwards = result['forward_groups']
        self.assertEqual((first['forwards'], warm['forwards'], all_forwards['forwards']), (2,38,40))
        self.assertEqual((first['first_position'], first['last_position']), (0,1))
        self.assertEqual((warm['first_position'], warm['last_position']), (2,39))
        self.assertEqual([g['total']['bank_ns'] for g in result['forward_groups']], [60,1140,1200])
        self.assertEqual([g['total']['census_ns'] for g in result['forward_groups']], [30,570,600])
        for mode in ('bank', 'census'):
            self.assertEqual(first['total'][mode+'_ns']+warm['total'][mode+'_ns'], all_forwards['total'][mode+'_ns'])
            self.assertLess(all_forwards['total'][mode+'_ns'], result['total'][mode+'_ns'])
        outputs = A.render(fixture(), T.timeline)
        rows = list(csv.reader(io.StringIO(outputs['forward-groups.csv'].decode())))
        self.assertEqual(len(rows), 4)
        self.assertEqual(rows[2][:4], ['Warm (2-39)', '2', '39', '38'])
        svg = ET.fromstring(outputs['groups.svg']); ns = {'s':'http://www.w3.org/2000/svg'}
        self.assertEqual((svg.attrib['width'], svg.attrib['height']), ('1060','370'))
        self.assertEqual(len(svg.findall('.//s:title', ns)), 8)
        self.assertIn(b'Rows overlap', outputs['groups.svg'])
        self.assertIn(b'not extra time', outputs['table.md'])

    def test_both_original_policies_and_bank_ablation_scope_are_mandatory(self):
        for mode in ('bank','census'):
            for key, wrong in (('no_policy_bytes_discarded',False),('original_stderr_empty',True),
                               ('temporal_equivalent_to_full',True),('shared_full_currentness',True)):
                value=fixture(); value['cases'][mode]['matched_timing']['policy'][key]=wrong
                value['matched_pair'][mode+'_policy'][key]=wrong
                with self.subTest(mode=mode,key=key), self.assertRaises(ValueError):
                    A.summarize(value,T.timeline)
            value=fixture(); value['matched_pair'][mode+'_policy']['policy_record']['original']='changed'
            with self.assertRaises(ValueError): A.summarize(value,T.timeline)
        for key, wrong in (('bank_scoped_rearm_case_retained',False),('allocation_preflights_changed',False)):
            value=fixture(); value[key]=wrong
            with self.assertRaises(ValueError): A.summarize(value,T.timeline)
        value=fixture(); value['matched_pair']['allocation_preflights_changed']=False
        with self.assertRaises(ValueError): A.summarize(value,T.timeline)


    def test_census_subset_and_temporal_change_are_explicit_at_every_admission_layer(self):
        for target in ('observation', 'pair', 'candidate'):
            for key in ('allocation_preflights_changed', 'census_counters_are_layer_subset'):
                for bad in (False, 1, None):
                    value = fixture()
                    node = value if target == 'observation' else value['matched_pair'] if target == 'pair' else value['cases']['census']['matched_timing']
                    node[key] = bad
                    with self.subTest(target=target, key=key, bad=bad), self.assertRaises(ValueError):
                        A.summarize(value, T.timeline)
        result = A.summarize(fixture(), T.timeline)
        self.assertTrue(result['allocation_preflights_changed'] and result['census_counters_are_layer_subset'])
        self.assertFalse(result['temporal_equivalent_to_full'] or result['full2303_feasibility'])

    def test_bank_v2_control_cannot_be_relabelled_as_census_v3_candidate(self):
        for mode, other in (('bank', 'census'), ('census', 'bank')):
            for key in ('schema', 'mode'):
                value = fixture()
                value['cases'][mode]['matched_timing'][key] = value['cases'][other]['matched_timing'][key]
                with self.subTest(mode=mode, key=key), self.assertRaises(ValueError):
                    A.summarize(value, T.timeline)
        value = fixture(); before = copy.deepcopy(value)
        result = A.summarize(value, T.timeline)
        self.assertEqual(value, before)
        self.assertEqual(result['policy_records'], {
            mode: before['cases'][mode]['matched_timing']['policy']['policy_record']
            for mode in ('bank', 'census')})
        self.assertEqual(result['total']['bank_ns'], 1240)
        self.assertEqual(result['total']['census_ns'], 620)
        self.assertEqual(len(result['categories']), 7)
        self.assertTrue(result['one_ordered_pair'])
        self.assertFalse(result['gpu_overlap_claim'] or result['tokens_per_second_claim'])


if __name__ == '__main__':
    unittest.main()
