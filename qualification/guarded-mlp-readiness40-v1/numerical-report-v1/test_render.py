"""Synthetic presentation-contract tests; no actual model or reference imports."""
import copy
import csv
import io
import json
from pathlib import Path
import tempfile
import unittest
import xml.etree.ElementTree as ET

import render


def fixture():
    points = []
    for position in range(40):
        selected = position in (0, 15, 16, 39)
        points.append(dict(position=position, input_token=271 if position == 5 else 100 + position,
            candidate_output_token=9112 if position == 5 else 20 + position,
            reference_output_token=2 if position == 5 else 20 + position,
            output_token_equal=position != 5, selected_capture=selected,
            candidate_argmax_recomputed=selected, reference_argmax_recomputed=selected,
            candidate_logits=dict(bytes=303872, sha256='a' * 64),
            reference_logits=dict(bytes=303872, sha256='b' * 64),
            unselected_argmax_scope=None if selected else 'authenticated original records; logits body not retained'))
    comparisons = []
    for position in (0, 15, 16, 39):
        tensors = {}
        for index, name in enumerate([*('layer%d-hidden' % i for i in range(36)), 'final-norm', 'logits']):
            count = 151936 if name == 'logits' else 4096
            exact = position == 0 and index == 0
            tensors[name] = dict(elements=count, exact_words=count if exact else count - 1,
                max_abs_error=0.0 if exact else 0.125, max_bf16_steps=0 if exact else 1,
                relative_l2=0.0 if exact else (position + index + 1) / 10000,
                rmse=0.0 if exact else 0.01, zero_reference_norm=False)
        comparisons.append({**{k: points[position][k] for k in
            ('position', 'input_token', 'candidate_output_token', 'reference_output_token', 'output_token_equal')}, 'tensors': tensors})
    selected = dict(schema='ferric-readiness40-selected-tensor-diagnostic-v1', acceptance_threshold=None,
        comparisons=comparisons, input_history='authentic first40 prompt IDs; independent own KV; no generated feedback',
        tensor_rows=152, receipt_authentication=False, numerical_acceptance=False,
        full_model_acceptance=False, full_long_workload=False, performance_claim=False)
    mismatch_keys = ('position', 'input_token', 'candidate_output_token', 'reference_output_token',
                     'selected_capture', 'candidate_argmax_recomputed', 'reference_argmax_recomputed')
    return dict(schema='ferric-readiness40-actual-reference-diagnostic-v1', acceptance_threshold=None,
        argmax_diagnostics=points, argmax_matches=39,
        argmax_mismatches=[{k: points[5][k] for k in mismatch_keys}], argmax_positions=40,
        bundle_id='6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b',
        candidate_generated_tokens=0, reference_generated_tokens=0, own_kv_caches=True,
        full_prompt_sha256='2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02',
        model_id='f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a',
        selected=selected, selected_positions=[0, 15, 16, 39], tensor_rows=152,
        numerical_acceptance=False, full_model_acceptance=False, full_long_workload=False,
        performance_claim=False, production_authority=False, gpu_execution=False, model_execution=False)


class ReportTests(unittest.TestCase):
    def test_csv_preserves_all152_metrics_in_numeric_layer_order(self):
        d = fixture()
        outputs = render.render(d)
        rows = list(csv.DictReader(io.StringIO(outputs['tensor-metrics.csv'].decode())))
        self.assertEqual(len(rows), 152)
        expected_roles = [*('layer%d-hidden' % i for i in range(36)), 'final-norm', 'logits']
        for point, block in zip(d['selected']['comparisons'], [rows[i:i + 38] for i in range(0, 152, 38)]):
            self.assertEqual([row['tensor'] for row in block], expected_roles)
            for row in block:
                self.assertEqual(int(row['position']), point['position'])
                for key, value in point['tensors'][row['tensor']].items():
                    self.assertEqual(row[key], str(value))

    def test_all40_argmax_and_uncaptured_mismatch_survive(self):
        outputs = render.render(fixture())
        rows = list(csv.DictReader(io.StringIO(outputs['argmax-diagnostics.csv'].decode())))
        self.assertEqual([int(row['position']) for row in rows], list(range(40)))
        self.assertEqual(rows[5]['candidate_output_token'], '9112')
        self.assertEqual(rows[5]['reference_output_token'], '2')
        self.assertEqual(rows[5]['selected_capture'], 'False')
        self.assertEqual(rows[5]['candidate_argmax_recomputed'], 'False')
        summary = outputs['summary.md'].decode()
        self.assertIn('39/40 argmax matches; 1/152 selected tensors bit exact', summary)
        self.assertIn('| 5 | 271 | 9112 | 2 | NOT captured |', summary)
        self.assertIn('does not establish numerical acceptance', summary)

    def test_svg_four_distinct_series_stable_geometry_and_percent_axis(self):
        d = fixture()
        root = ET.fromstring(render.render(d)['hidden-relative-l2.svg'])
        ns = {'s': 'http://www.w3.org/2000/svg'}
        self.assertEqual((root.get('width'), root.get('height'), root.get('viewBox')), ('1000', '540', '0 0 1000 540'))
        lines = root.findall('.//s:polyline', ns)
        self.assertEqual([int(row.get('data-position')) for row in lines], [0, 15, 16, 39])
        self.assertEqual(len({row.get('stroke') for row in lines}), 4)
        self.assertEqual(len({row.get('stroke-dasharray') for row in lines}), 4)
        for row in lines:
            points = [tuple(map(float, point.split(','))) for point in row.get('points').split()]
            self.assertEqual(len(points), 36)
            self.assertEqual((points[0][0], points[-1][0]), (90, 955))
            self.assertTrue(all(90 <= x <= 955 and 115 <= y <= 405 for x, y in points))
        texts = [row.text for row in root.findall('.//s:text', ns)]
        self.assertIn('Relative L2 error (%)', texts)
        self.assertIn('39/40 argmax matches; position 5 was NOT captured. No numerical acceptance.', texts)

    def test_rejects_authority_identity_and_extra_fields(self):
        mutations = [('numerical_acceptance', True), ('gpu_execution', True), ('performance_claim', True),
                     ('candidate_generated_tokens', 1), ('own_kv_caches', False), ('acceptance_threshold', 0.01),
                     ('model_id', '0' * 64), ('bundle_id', '0' * 64), ('full_prompt_sha256', '0' * 64), ('extra', False)]
        for key, value in mutations:
            with self.subTest(key=key):
                d = fixture()
                d[key] = value
                with self.assertRaises(ValueError):
                    render.render(d)

    def test_rejects_metric_shape_bool_negative_and_nonfinite(self):
        mutations = [('elements', 4095), ('exact_words', True), ('max_bf16_steps', -1), ('rmse', float('nan')),
                     ('max_abs_error', float('inf')), ('relative_l2', -1), ('relative_l2', True),
                     ('relative_l2', None), ('relative_l2', 1e308), ('zero_reference_norm', 0), ('extra', 0)]
        for key, value in mutations:
            with self.subTest(key=key, value=value):
                d = fixture()
                d['selected']['comparisons'][0]['tensors']['layer1-hidden'][key] = value
                with self.assertRaises(ValueError):
                    render.render(d)

    def test_rejects_missing_reordered_or_duplicated_capture_roles(self):
        d = fixture()
        del d['selected']['comparisons'][0]['tensors']['layer35-hidden']
        with self.assertRaises(ValueError):
            render.render(d)
        d = fixture()
        d['selected']['comparisons'].reverse()
        with self.assertRaises(ValueError):
            render.render(d)
        d = fixture()
        d['selected']['comparisons'][1] = copy.deepcopy(d['selected']['comparisons'][0])
        with self.assertRaises(ValueError):
            render.render(d)

    def test_rejects_argmax_scope_history_and_mismatch_loss(self):
        for key, value in [('position', True), ('input_token', True), ('candidate_argmax_recomputed', True),
                           ('output_token_equal', True), ('unselected_argmax_scope', None)]:
            with self.subTest(key=key):
                d = fixture()
                d['argmax_diagnostics'][5][key] = value
                with self.assertRaises(ValueError):
                    render.render(d)
        d = fixture()
        d['argmax_mismatches'] = []
        with self.assertRaises(ValueError):
            render.render(d)
        d = fixture()
        d['selected']['comparisons'][0]['input_token'] += 1
        with self.assertRaises(ValueError):
            render.render(d)

    def test_zero_curve_has_finite_nonclipped_axis(self):
        d = fixture()
        for row in d['selected']['comparisons']:
            for name, metric in row['tensors'].items():
                if name.startswith('layer'):
                    metric.update(relative_l2=0.0, rmse=0.0, zero_reference_norm=True)
        raw = render.render(d)['hidden-relative-l2.svg']
        root = ET.fromstring(raw)
        for row in root.findall('.//{http://www.w3.org/2000/svg}polyline'):
            self.assertTrue(all(point.endswith(',405.000') for point in row.get('points').split()))
        self.assertNotIn(b'nan', raw)

    def test_exact_input_pin_duplicate_keys_and_nonfinite_json_refused(self):
        with self.assertRaises(ValueError):
            render.authenticate(json.dumps(fixture()).encode())
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}', b'{"a":Infinity}'):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                render.parse(raw)

    def test_reader_refuses_symlink_and_oversized_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            body = root / 'body'
            body.write_bytes(b'ordinary')
            self.assertEqual(render.read(body), b'ordinary')
            link = root / 'link'
            link.symlink_to(body)
            with self.assertRaises(ValueError):
                render.read(link)
            body.write_bytes(b'x' * (render.CAP + 1))
            with self.assertRaises(ValueError):
                render.read(body)
