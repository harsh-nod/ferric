"""Small synthetic host timelines only; no archive imports, model or GPU work."""
import copy
import csv
import io
from pathlib import Path
import tempfile
import unittest
import xml.etree.ElementTree as XML

import duration_report as R
import validate_timing as T


def fixture():
    cursor = 0
    def span(value):
        nonlocal cursor
        row = dict(start_ns=cursor, end_ns=cursor + value, elapsed_ns=value)
        cursor += value
        return row
    timeline = dict(source_preparation=span(3), spawn_to_setup_seal=span(5), forwards=[])
    for position in range(40):
        timeline['forwards'].append(dict(position=position, generation=position + 1,
            prepare_write=span(2), flush_to_frame_read=span(1),
            validate_retain_commit=span(17), elapsed_ns=20))
    timeline.update(close_and_retirement=span(7), postcheck_and_ordinary_publication=span(11), total_ns=cursor)
    # These call counts match the qualified DurationTests fixture divided over
    # 38 warm forwards. The elapsed values and complete timeline are synthetic.
    calls = dict(bank=(726, 2, 726, 725), layers=(972, 72, 972, 1620), tail=(43, 2, 43, 57))
    measured = {group: {key: dict(calls=count, elapsed_ns=1)
                        for key, count in zip(R.CATEGORIES, counts)} for group, counts in calls.items()}
    measured['bank_guarded_body_ns'] = 5
    rows = [dict(position=position, measured=None if position < 2 else copy.deepcopy(measured))
            for position in range(40)]
    T.timeline(timeline)
    return rows, timeline


def admitted_shape():
    """Reducer result shape only; not a substitute for original archive admission."""
    rows, timeline = fixture()
    diagnostic = dict(forwards=rows)
    policy = dict(counts='synthetic reducer fixture; no original-policy admission claim')
    timing = dict(timeline=timeline, disjoint_spans=124, parent_host_measurement=True,
        file=dict(bytes=1, sha256='0' * 64), **{k: False for k in T.FALSE_FIELDS})
    checked = dict(schema='ferric-readiness40-tail-currentness-duration-case-v1',
        mode='tail_duration', instrumented=True, timing=timing,
        ordinary=dict(currentness_duration_diagnostic=diagnostic, bank_scoped_census_tail_policy=policy),
        policy=dict(name='bank_scoped_census_tail_duration', no_policy_bytes_discarded=True,
            original_stderr_empty=False, diagnostic_record=diagnostic, policy_record=policy,
            file=dict(bytes=1, sha256='1' * 64)))
    case = dict(present=True, original_passed=True, retained_success_revalidated=True,
        readiness_completed_forwards=40, semantic_and_payload_parity_revalidated=True,
        matched_timing=checked, original_terminal=dict(name='synthetic-only', bytes=1, sha256='2' * 64))
    yes = ('baseline_data_revalidated', 'original_owned_lineage_revalidated',
        'semantic_and_payload_parity_revalidated', 'parent_host_measurement',
        'original_policy_bytes_retained', 'instrumented', 'currentness_duration_diagnostic',
        'bank_guarded_body_includes_callbacks')
    no = ('gpu_timing', 'matched_speed_comparison', 'native_rerun', 'gpu_execution',
        'model_execution', 'numerical_reference_replayed', 'numerical_acceptance',
        'full_long_workload', 'performance_claim', 'production_authority')
    return dict(passed=True, selected_original_bodies=166, raw_files=73, baseline_raw_files=70,
        cases={'tail_duration': case}, cpu_and_elf_metadata={}, checker_cpu={},
        **{k: True for k in yes}, **{k: False for k in no})


class DurationReportTests(unittest.TestCase):
    def test_small_valid_timeline_and_twelve_callback_categories_reconcile(self):
        rows, timeline = fixture()
        before = copy.deepcopy((rows, timeline))
        got = R.reduce_durations(rows, timeline, T.timeline)
        self.assertEqual((rows, timeline), before)
        self.assertEqual((len(got['positions']), len(got['callbacks']), len(got['parent_spans'])), (40, 12, 124))
        self.assertEqual(got['parent_total_ns'], 826)
        self.assertEqual(got['warm']['parent_forward_ns'], 760)
        self.assertEqual([row['elapsed_ns'] for row in got['forward_groups']], [40, 760, 800])
        self.assertEqual(sum(row['elapsed_ns'] for row in got['parent_spans']), 826)
        self.assertEqual(sum(row['elapsed_ns'] for row in got['callbacks']), 456)
        self.assertEqual(next(row for row in got['callbacks'] if row['group'] == 'bank'
                              and row['category'] == 'before')['calls'], 726 * 38)

    def test_residual_uses_complete_forward_not_flush_wait(self):
        rows, timeline = fixture()
        got = R.reduce_durations(rows, timeline, T.timeline)['positions'][2]
        self.assertGreater(got['expanded_disjoint_ns'], timeline['forwards'][2]['flush_to_frame_read']['elapsed_ns'])
        self.assertEqual((got['expanded_disjoint_ns'], got['parent_forward_ns'], got['unattributed_parent_ns']), (13, 20, 7))

    def test_bank_callbacks_are_never_added_to_the_containing_bank_body(self):
        rows, timeline = fixture()
        got = R.reduce_durations(rows, timeline, T.timeline)['warm']
        self.assertEqual((got['bank_callbacks_ns'], got['bank_guarded_body_ns'], got['bank_other_ns']), (152, 190, 38))
        self.assertEqual(got['expanded_disjoint_ns'], 494)
        self.assertEqual(sum(got[k] for k in R.PARTITION), got['parent_forward_ns'])
        self.assertNotEqual(got['bank_guarded_body_ns'] + got['all_callbacks_ns'], got['expanded_disjoint_ns'])

    def test_each_forward_rejects_negative_residual_even_if_aggregate_would_fit(self):
        rows, timeline = fixture()
        rows[2]['measured']['bank_guarded_body_ns'] = 30
        self.assertLess(30 + 37 * 5 + 38 * 8, 760)
        with self.assertRaisesRegex(ValueError, 'negative complete-parent-forward residual'):
            R.reduce_durations(rows, timeline, T.timeline)

    def test_bank_callback_subtotal_must_fit_its_interval(self):
        rows, timeline = fixture()
        rows[2]['measured']['bank_guarded_body_ns'] = 3
        with self.assertRaisesRegex(ValueError, 'contained'):
            R.reduce_durations(rows, timeline, T.timeline)

    def test_scalar_types_bounds_closed_fields_and_bank_census_are_strict(self):
        for wrong in (True, -1, 1.0, 1 << 64):
            for key in ('calls', 'elapsed_ns'):
                rows, timeline = fixture()
                rows[2]['measured']['tail']['before'][key] = wrong
                with self.subTest(key=key, wrong=wrong), self.assertRaises(ValueError):
                    R.reduce_durations(rows, timeline, T.timeline)
        for mutation in ('extra', 'zero_calls', 'balance', 'bank_census', 'discoveries'):
            rows, timeline = fixture(); m = rows[2]['measured']
            if mutation == 'extra': m['other'] = 0
            elif mutation == 'zero_calls': m['tail']['before']['calls'] = 0
            elif mutation == 'balance': m['layers']['after']['calls'] += 1
            elif mutation == 'bank_census':
                m['bank']['before']['calls'] += 1; m['bank']['after']['calls'] += 1
            else: m['layers']['discover']['calls'] += 1
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                R.reduce_durations(rows, timeline, T.timeline)

    def test_first_two_unmeasured_and_all_positions_are_exact(self):
        for mutation in ('ordinary', 'missing', 'extra', 'position', 'bool_position', 'warm_none'):
            rows, timeline = fixture()
            if mutation == 'ordinary': rows[0]['measured'] = rows[2]['measured']
            elif mutation == 'missing': rows.pop()
            elif mutation == 'extra': rows.append(copy.deepcopy(rows[-1]))
            elif mutation == 'position': rows[3]['position'] = 2
            elif mutation == 'bool_position': rows[0]['position'] = False
            else: rows[2]['measured'] = None
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                R.reduce_durations(rows, timeline, T.timeline)

    def test_original_timing_gaps_overlaps_totals_and_generation_are_rejected(self):
        for mutation in ('start', 'elapsed', 'forward', 'total', 'generation'):
            rows, timeline = fixture(); forward = timeline['forwards'][2]
            if mutation == 'start': forward['prepare_write']['start_ns'] += 1
            elif mutation == 'elapsed': forward['flush_to_frame_read']['elapsed_ns'] += 1
            elif mutation == 'forward': forward['elapsed_ns'] += 1
            elif mutation == 'total': timeline['total_ns'] += 1
            else: forward['generation'] += 1
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                R.reduce_durations(rows, timeline, T.timeline)

    def test_zero_elapsed_callbacks_are_not_confused_with_unmeasured_rows(self):
        rows, timeline = fixture()
        for row in rows[2:]:
            row['measured']['bank_guarded_body_ns'] = 0
            for group in R.GROUPS:
                for pair in row['measured'][group].values(): pair['elapsed_ns'] = 0
        got = R.reduce_durations(rows, timeline, T.timeline)
        self.assertIsNone(got['positions'][0]['all_callbacks_ns'])
        self.assertEqual(got['positions'][2]['all_callbacks_ns'], 0)
        self.assertEqual(got['warm']['unattributed_parent_ns'], 760)

    def test_integer_overflow_and_decimal_half_even_are_exact(self):
        self.assertEqual(R.checked_sum((1 << 53, 1)), (1 << 53) + 1)
        with self.assertRaises(ValueError): R.checked_sum(((1 << 64) - 1, 1))
        self.assertEqual(R.decimal_ratio(1, 2000000), '0.000000')
        self.assertEqual(R.decimal_ratio(3, 2000000), '0.000002')
        self.assertIsNone(R.decimal_ratio(0, 0))

    def test_summarizer_preserves_single_case_scope_and_rejects_false_admission(self):
        observation = admitted_shape()
        got = R.summarize(observation, T.timeline)
        self.assertEqual(got['parent_total_ns'], 826)
        self.assertTrue(got['first_two_callbacks_unmeasured'])
        self.assertFalse(got['gpu_timing'] or got['gpu_overlap_claim'] or got['matched_speed_comparison'])
        for key in ('passed', 'instrumented', 'original_owned_lineage_revalidated'):
            changed = copy.deepcopy(observation); changed[key] = False
            with self.subTest(key=key), self.assertRaises(ValueError): R.summarize(changed, T.timeline)
        changed = copy.deepcopy(observation); changed['performance_claim'] = True
        with self.assertRaises(ValueError): R.summarize(changed, T.timeline)
        changed = copy.deepcopy(observation)
        changed['cases']['tail_duration']['matched_timing']['ordinary']['currentness_duration_diagnostic'] = {}
        with self.assertRaises(ValueError): R.summarize(changed, T.timeline)

    def test_csv_svg_and_tables_preserve_integers_blank_unmeasured_and_no_double_sum(self):
        summary = R.summarize(admitted_shape(), T.timeline)
        outputs = R.render(summary)
        self.assertEqual(set(outputs), {'summary.json', 'table.md', 'callbacks.csv',
            'forwards.csv', 'parent-spans.csv', 'attribution.svg'})
        rows = list(csv.DictReader(io.StringIO(outputs['forwards.csv'].decode())))
        self.assertEqual((rows[0]['measured'], rows[0]['bank_callbacks_ns'], rows[2]['bank_callbacks_ns']), ('False', '', '4'))
        self.assertEqual(sum(int(row['elapsed_ns']) for row in csv.DictReader(
            io.StringIO(outputs['parent-spans.csv'].decode()))), 826)
        self.assertEqual(len(list(csv.DictReader(io.StringIO(outputs['callbacks.csv'].decode())))), 12)
        svg = XML.fromstring(outputs['attribution.svg'])
        self.assertEqual(svg.attrib['viewBox'], '0 0 1040 370')
        self.assertIn(b'not GPU time', outputs['attribution.svg'])
        self.assertIn(b'Cbank + (B - Cbank)', outputs['table.md'])
        self.assertIn(b'forward groups are subsets', outputs['table.md'])

    def test_standalone_verifier_bytes_must_match_before_any_load(self):
        with tempfile.TemporaryDirectory(prefix='duration-report-source-') as directory:
            path = Path(directory).resolve() / 'not-the-verifier.py'
            path.write_bytes(b'x' * R.VERIFIER['bytes'])
            with self.assertRaisesRegex(ValueError, 'exact final retention source'):
                R.source_bytes(path)


if __name__ == '__main__':
    unittest.main()
