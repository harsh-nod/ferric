"""Synthetic report arithmetic and refusal tests, not a native qualification."""
import csv
import io
import unittest
import xml.etree.ElementTree as ET

import render as R


def fixture():
    result = {}
    for mode in R.MODES:
        times = [[1_000_000 * (position + 1) + layer for layer in range(36)] for position in range(4)]
        if mode == 'candidate':
            times = [[v + (100 if position < 2 else -100) for v in row]
                     for position, row in enumerate(times)]
        cadence = [0, 0, 36, 36] if mode == 'candidate' else [0, 0, 0, 0]
        observation = dict(host_timing_only=True, gpu_latency_claim=False, segment_host_ns=times,
            paired_terminal_dispatches=cadence, local_bank_generations=[1, 1, 2, 2],
            input_tokens=[9112, 67, 25, 576], output_tokens=[67, 25, 576, 2701])
        result[mode] = dict(passed=True, errors=[], case=mode, mode='ar4', native_attempts=1, retries=0,
            paired_terminal_requested=mode == 'candidate', default_full_currentness_requested=True,
            shared_full_currentness_requested=False, host_observation_requested=False,
            hidden_read_policy_changed=False, paired_terminal_dispatches=cadence,
            segment_host_ns=times, observation=observation, numerical_acceptance=False,
            performance_claim=False, production_authority=False, controller={'sha256':'a' * 64},
            admission={'worker':{'sha256':'b' * 64}, 'parent':{'sha256':'c' * 64}},
            ordinary_comparison=dict(all_payloads_equal=True, all_histories_equal=True),
            same_elf_control_required=mode == 'candidate',
            control_comparison=dict(all_payloads_equal=True, all_histories_equal=True))
    return result


class RenderTests(unittest.TestCase):
    def test_exact_integer_layers_positions_and_first_use_warm_sums(self):
        data = R.summarize(fixture())
        self.assertEqual(len(data['layers']), 288)
        self.assertEqual(len(data['summaries']), 6)
        self.assertEqual([v['candidate_minus_control_ns'] for v in data['summaries']],
                         [3600, 3600, -3600, -3600, 7200, -7200])
        first = data['summaries'][4]; warm = data['summaries'][5]
        self.assertEqual(first['positions'], [0, 1])
        self.assertEqual(warm['positions'], [2, 3])
        self.assertEqual(first['intervals_per_mode'], 72)
        self.assertEqual(first['control_ns'], 108_001_260)
        self.assertEqual(warm['control_ns'], 252_001_260)
        self.assertFalse(data['controlled_benchmark'])

    def test_wrong_extent_boolean_negative_and_u64_overflow_refuse(self):
        for value in (True, -1, R.U64_MAX + 1, 1.0):
            with self.subTest(value=value):
                case = fixture(); case['candidate']['observation']['segment_host_ns'][0][0] = value
                with self.assertRaises(ValueError): R.summarize(case)
        for rows in ([[1] * 36] * 3, [[1] * 35] * 4, [[1] * 37] * 4):
            case = fixture(); case['candidate']['observation']['segment_host_ns'] = rows
            case['candidate']['segment_host_ns'] = rows
            with self.assertRaises(ValueError): R.summarize(case)

    def test_failure_policy_cadence_and_authority_mutations_refuse(self):
        mutations = [('passed', False), ('errors', ['failed']), ('retries', 1),
            ('paired_terminal_requested', False), ('default_full_currentness_requested', False),
            ('shared_full_currentness_requested', True), ('hidden_read_policy_changed', True),
            ('paired_terminal_dispatches', [36, 36, 36, 36]), ('performance_claim', True)]
        for key, value in mutations:
            with self.subTest(key=key):
                cases = fixture(); cases['candidate'][key] = value
                with self.assertRaises(ValueError): R.summarize(cases)

    def test_same_elf_payload_history_and_timer_joins_are_required(self):
        variants = []
        cases = fixture(); cases['candidate']['admission']['worker']['sha256'] = 'd' * 64; variants.append(cases)
        cases = fixture(); cases['candidate']['control_comparison']['all_payloads_equal'] = False; variants.append(cases)
        cases = fixture(); cases['candidate']['ordinary_comparison']['all_histories_equal'] = False; variants.append(cases)
        cases = fixture(); cases['candidate']['observation']['output_tokens'][2] = 0; variants.append(cases)
        cases = fixture(); cases['candidate']['segment_host_ns'] = [[0] * 36 for _ in range(4)]; variants.append(cases)
        cases = fixture(); cases['candidate']['observation']['local_bank_generations'] = [1, 2, 3, 4]; variants.append(cases)
        for i, cases in enumerate(variants):
            with self.subTest(mutation=i):
                with self.assertRaises(ValueError): R.summarize(cases)

    def test_zero_denominator_and_decimal_ties_are_explicit(self):
        cases = fixture()
        for case in cases.values():
            times = [[0] * 36 for _ in range(4)]
            case['observation']['segment_host_ns'] = times; case['segment_host_ns'] = times
        data = R.summarize(cases)
        self.assertTrue(all(v['relative_change_denominator'] is None for v in data['summaries']))
        self.assertIn(b'| n/a |', R.markdown(data))
        self.assertEqual(R.decimal_ratio(25, 10, 0), '2')
        self.assertEqual(R.decimal_ratio(35, 10, 0), '4')
        self.assertEqual(R.decimal_ratio(-25, 10, 0), '-2')
        ET.fromstring(R.plot(data))

    def test_csv_preserves_all_original_unsigned_nanoseconds(self):
        cases = fixture(); cases['candidate']['observation']['segment_host_ns'][3][35] = R.U64_MAX
        data = R.summarize(cases); bodies = R.outputs(data)
        rows = list(csv.DictReader(io.StringIO(bodies['layers.csv'].decode())))
        self.assertEqual(len(rows), 288)
        self.assertEqual(int(rows[-1]['segment_host_ns']), R.U64_MAX)
        self.assertEqual([int(r['segment_host_ns']) for r in rows],
                         [r['segment_host_ns'] for r in data['layers']])
        summaries = list(csv.DictReader(io.StringIO(bodies['summary.csv'].decode())))
        self.assertEqual(len(summaries), 6)
        self.assertEqual(summaries[-1]['positions'], '2,3')

    def test_structured_svg_has_four_panels_eight_lines_and_scope_notes(self):
        data = R.summarize(fixture()); svg = ET.fromstring(R.plot(data)); ns = {'s': R.SVG}
        self.assertEqual(svg.attrib['viewBox'], '0 0 1120 760')
        lines = svg.findall('s:polyline', ns); self.assertEqual(len(lines), 8)
        for line in lines:
            self.assertEqual(len(line.attrib['points'].split()), 36)
            self.assertEqual(line.attrib['fill'], 'none')
        texts = [n.text for n in svg.findall('s:text', ns)]
        self.assertEqual(sum(t.startswith('Position ') for t in texts), 4)
        self.assertTrue(any('One pair only' in t for t in texts))
        self.assertTrue(any('Not GPU or full-forward time' in t for t in texts))
        self.assertIn(b'not a controlled repeated benchmark', R.markdown(data))


if __name__ == '__main__':
    unittest.main()
