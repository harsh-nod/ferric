"""Synthetic trajectory/policy tests; no Torch import or framework execution."""
import copy
import importlib.util
from pathlib import Path
import sys
import tempfile
import types
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('ar4_framework_policy', Path(__file__).with_name('run.py'))
R = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(R)


def records(outputs=(7, 8, 9, 10), inputs=None):
    inputs = [9112, *outputs[:3]] if inputs is None else inputs
    return [dict(generation=n + 1, position=n, input_token=inputs[n], output_token=outputs[n])
            for n in range(4)]


def pin(name, size=1, sha='a' * 64):
    return dict(path='/retained/' + name, bytes=size, sha256=sha)


def native():
    return dict(complete=pin('complete.json'), transport={},
                validators={name: pin(name, sha=sha) for name, sha in R.VALIDATORS.items()})


def plan():
    return dict(schema='ferric-p228-projection-ar4-framework-plan-v1', harness_sha256='b' * 64,
                reference_helper=pin('framework_reference.py', sha=R.BASE_SHA),
                reference_plan=pin('reference-plan.json'), execution_review=pin('review.json'),
                output_root=str(R.E / 'projection-ar4-framework-reference-v228-v1'),
                implementation_sources={name: pin(name + '.py', sha=R.MODEL_SHA if name == 'modeling_qwen3'
                    else 'c' * 64) for name in R.SOURCES}, native=native())


def passes(outputs=(7, 8, 9, 10), forced=None):
    names = [*(f'layer{n}-hidden' for n in range(36)), 'final-norm', 'logits']
    result = []
    for ordinal in (1, 2):
        cases = []
        for position, record in enumerate(records(outputs, forced)):
            cases.append(dict(record=record, payload=pin(f'pass{ordinal}-pos{position}.bf16', 606976),
                tensors={name: dict(bytes=303872 if name == 'logits' else 8192, sha256='d' * 64) for name in names},
                cache_sha256=[dict(key='e' * 64, value='f' * 64) for _ in range(36)]))
        result.append(dict(ordinal=ordinal, fresh_cache=True,
                           kind='genuine_ar' if forced is None else 'native_input_conditional', cases=cases))
    return result


class FakeDiag:
    """Only comparison scheduling is mocked; not numerical or receipt validation."""
    def validate_case(self, record, raw, position):
        R.require(record['position'] == position and raw == b'finite', 'fixture case')
        return {**{f'layer{n}-hidden': raw for n in range(36)}, 'final-norm': raw, 'logits': raw}

    def compare_tensor(self, left, right):
        return dict(exact=left == right)


class PolicyTests(unittest.TestCase):
    def test_genuine_uses_own_previous_output(self):
        self.assertEqual(R.trajectory(records()), [9112, 7, 8, 9])

    def test_wrong_seed_rejected(self):
        value = records(); value[0]['input_token'] = 785
        with self.assertRaises(ValueError): R.trajectory(value)

    def test_tf_history_not_autoregressive(self):
        with self.assertRaises(ValueError): R.trajectory(records(inputs=[9112, 2190, 3772, 220]))

    def test_conditional_does_not_require_own_output_recurrence(self):
        forced = [9112, 2190, 3772, 220]
        self.assertEqual(R.trajectory(records(inputs=forced), forced), forced)

    def test_conditional_wrong_actual_input_rejected(self):
        with self.assertRaises(ValueError): R.trajectory(records(), [9112, 7, 0, 9])

    def test_repeated_and_zero_outputs_are_valid(self):
        self.assertEqual(R.trajectory(records((0, 0, 0, 0))), [9112, 0, 0, 0])

    def test_bool_and_out_of_range_tokens_refused(self):
        for token in (True, -1, 151936):
            value = records(); value[3]['output_token'] = token
            with self.subTest(token=token), self.assertRaises(ValueError): R.trajectory(value)

    def test_position_generation_and_census_refused(self):
        for key in ('generation', 'position'):
            value = records(); value[2][key] += 1
            with self.subTest(key=key), self.assertRaises(ValueError): R.trajectory(value)
        with self.assertRaises(ValueError): R.trajectory(records()[:3])

    def test_prefix_comparability_never_recovers(self):
        self.assertEqual(R.comparable_prefix(records(), records((99, 8, 9, 10))), [True, False, False, False])

    def test_last_output_difference_keeps_all_input_histories_comparable(self):
        self.assertEqual(R.comparable_prefix(records(), records((7, 8, 9, 100))), [True] * 4)

    def test_genuine_comparison_skips_all_post_divergence_tensors(self):
        left = [(row, b'finite') for row in records()]
        right = [(row, b'finite') for row in records((99, 8, 9, 10))]
        value = R.compare_cases(FakeDiag(), left, right)
        self.assertEqual(value['tensor_rows'], 38)
        self.assertTrue(all(row['tensors'] is None for row in value['comparisons'][1:]))
        self.assertFalse(value['conditional']); self.assertFalse(value['numerical_acceptance'])

    def test_conditional_comparison_covers_all_native_inputs(self):
        forced = [9112, 99, 8, 9]
        left = [(row, b'finite') for row in records(inputs=forced)]
        right = [(row, b'finite') for row in records((99, 8, 9, 10))]
        value = R.compare_cases(FakeDiag(), left, right, True)
        self.assertEqual(value['tensor_rows'], 152)
        self.assertTrue(value['conditional']); self.assertIsNone(value['acceptance_threshold'])

    def test_repeat_hashes_and_own_cache_checked(self):
        self.assertTrue(R.repeat_equal(passes()))
        for key in ('tensors', 'cache_sha256'):
            value = passes()
            if key == 'tensors': value[1]['cases'][2][key]['logits']['sha256'] = 'b' * 64
            else: value[1]['cases'][2][key][0]['key'] = 'b' * 64
            self.assertFalse(R.repeat_equal(value))

    def test_conditional_pass_cannot_claim_genuine(self):
        forced = [9112, 99, 8, 9]
        value = passes(forced=forced)
        self.assertTrue(R.repeat_equal(value, forced))
        with self.assertRaises(ValueError): R.repeat_equal(value)

    def test_missing_tensor_and_nonfresh_cache_refused(self):
        value = passes(); del value[0]['cases'][0]['tensors']['final-norm']
        with self.assertRaises(ValueError): R.repeat_equal(value)
        value = passes(); value[0]['fresh_cache'] = False
        with self.assertRaises(ValueError): R.repeat_equal(value)

    def test_plan_closed_namespace_and_sources(self):
        R.plan_shape(plan())
        for key, item in (('schema', 'ferric-p228-layer0-framework-capture-plan-v1'),
                          ('output_root', str(R.E / 'layer0-framework-capture-v228-v1'))):
            value = plan(); value[key] = item
            with self.subTest(key=key), self.assertRaises(ValueError): R.plan_shape(value)
        value = plan(); value['extra'] = False
        with self.assertRaises(ValueError): R.plan_shape(value)

    def test_native_validator_generation_and_unused_roles_refused(self):
        R.native_shape(native())
        value = native(); value['validators']['decode_validation.py']['sha256'] = '0' * 64
        with self.assertRaises(ValueError): R.native_shape(value)
        value = native(); value['validators']['intake.py'] = pin('intake.py')
        with self.assertRaises(ValueError): R.native_shape(value)

    def test_review_binds_native_complete_and_false_authority(self):
        value = plan(); limits = {'test_only': True}
        review = dict(schema='ferric-p228-projection-ar4-framework-execution-review-v1', reviewed=True,
            plan_projection_sha256=R.digest(R.encoded({k: v for k, v in value.items() if k != 'execution_review'})),
            resources=limits, gpu_execution_authorized=True, numerical_acceptance=False,
            performance_claim=False, production_authority=False)
        R.reviewed(value, review, limits)
        changed = copy.deepcopy(value); changed['native']['complete']['sha256'] = 'f' * 64
        with self.assertRaises(ValueError): R.reviewed(changed, review, limits)
        review['numerical_acceptance'] = True
        with self.assertRaises(ValueError): R.reviewed(value, review, limits)

    def test_duplicate_json_and_nonfinite_refused(self):
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}'):
            with self.subTest(raw=raw), self.assertRaises(ValueError): R.parse(raw)

    def test_native_failed_outer_refused_before_any_validator_load(self):
        base = type('Base', (), {'read_pin': lambda self, _pin, _cap: R.encoded(dict(
            schema='ferric-p228-projection-ar4-decode-gpu-v1', passed=False))})()
        with self.assertRaises(ValueError): R.native_context(native(), base, FakeDiag())
        # Real scoped source loading, but explicitly synthetic validator semantics.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def write(name, raw):
                path = root / name; path.write_bytes(raw)
                return dict(path=str(path), bytes=len(raw), sha256=R.digest(raw))
            def file_pin(path):
                raw = path.read_bytes()
                return dict(path=str(path), bytes=len(raw), sha256=R.digest(raw))
            def read_pin(row, cap):
                R.require(row['bytes'] <= cap and file_pin(Path(row['path'])) == row, 'synthetic stable pin')
                return Path(row['path']).read_bytes()
            base = types.SimpleNamespace(file_pin=file_pin, read_pin=read_pin)
            checked = dict(input_tokens=[9112, 7, 8, 9], output_tokens=[7, 8, 9, 10])
            names = {'child-stderr.bin'} | {f'{kind}-{n}.{suffix}' for n in range(4)
                for kind, suffix in [('request', 'json'), ('control', 'bin'), ('observation', 'bin')]}
            roster = {name: write(name, b'' if name == 'child-stderr.bin' else b'finite') for name in names}
            roster['complete.json'] = write('summary.json', R.encoded(checked))
            request = write('request.json', b'{}')
            outer = dict(schema='ferric-p228-projection-ar4-decode-gpu-v1', passed=True, failures=[],
                native_attempts=1, retries=0, captured_tensor_rows=152, captured_payloads=4,
                own_output_trajectory_checked=True, numerical_acceptance=False, request=request,
                retained_native=roster, checked=checked)
            value = dict(complete=write('outer.json', R.encoded(outer)), transport={}, validators={})
            bodies = {'stage_core.py': b'marker = object()\n',
                'smoke_validation.py': b'from stage_core import marker\n',
                'decode_validation.py': ('import json\nfrom stage_core import marker\n'
                    'from smoke_validation import marker as other\nassert marker is other\n'
                    f'FILES = {names!r}\n'
                    'def validate(raw, files, request):\n    assert set(files) == FILES\n'
                    '    return json.loads(raw)\n').encode()}
            missing = object()
            saved = {name: sys.modules.get(name, missing) for name in ('stage_core', 'smoke_validation')}
            try:
                for old_alias in (missing, None, types.ModuleType('preexisting')):
                    for name in saved:
                        if old_alias is missing: sys.modules.pop(name, None)
                        else: sys.modules[name] = old_alias
                    value['validators'] = {name: write(name, raw) for name, raw in bodies.items()}
                    hashes = {name: row['sha256'] for name, row in value['validators'].items()}
                    with patch.object(R, 'VALIDATORS', hashes):
                        result = R.native_context(value, base, FakeDiag())
                    self.assertEqual(len(result['consumed']), 16)
                    self.assertFalse(result['external_native_lifecycle_replayed'])
                    for name in saved:
                        self.assertIs(sys.modules.get(name, missing), old_alias)
                for failed in bodies:
                    changed = dict(bodies); changed[failed] = b'raise ValueError("synthetic load failure")\n'
                    value['validators'] = {name: write(name, raw) for name, raw in changed.items()}
                    hashes = {name: row['sha256'] for name, row in value['validators'].items()}
                    with patch.object(R, 'VALIDATORS', hashes), self.assertRaisesRegex(ValueError, 'synthetic load failure'):
                        R.native_context(value, base, FakeDiag())
                    for name in saved:
                        self.assertIs(sys.modules.get(name, missing), old_alias)
            finally:
                for name, before in saved.items():
                    if before is missing: sys.modules.pop(name, None)
                    else: sys.modules[name] = before


if __name__ == '__main__':
    unittest.main()
