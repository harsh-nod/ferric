"""Synthetic source/shape/hook tests only; no torch import or framework execution."""
import copy
import types
import unittest

import run as M


def plan():
    return dict(schema='ferric-p228-layer0-framework-capture-plan-v1', harness_sha256='1' * 64,
                reference_helper=dict(path='/retained/framework_reference.py', bytes=1, sha256=M.BASE_SHA),
                reference_plan=dict(path='/retained/environment.json', bytes=1, sha256='2' * 64),
                output_root=str(M.E / 'layer0-framework-capture-v228-v1'),
                implementation_sources={name: dict(path='/retained/' + name + '.py', bytes=1,
                    sha256=M.MODEL_SHA if name == 'modeling_qwen3' else '3' * 64) for name in M.SOURCES},
                execution_review=dict(path='/retained/review.json', bytes=1, sha256='4' * 64))


def review(value):
    return dict(schema='ferric-p228-layer0-framework-execution-review-v1', reviewed=True,
                plan_projection_sha256=M.digest(M.encoded({key: row for key, row in value.items()
                                                          if key != 'execution_review'})),
                resources={'test_bound': 1}, gpu_execution_authorized=True,
                numerical_acceptance=False, performance_claim=False, production_authority=False)


def values():
    return {name: bytes(M.stage_bytes(name)) for name in M.SHAPES}


def passes():
    return [dict(ordinal=index, position=0, input_token=9112, fresh_cache=True,
                 stages={name: dict(dtype='bfloat16', shape=list(shape),
                    pin=dict(path=f'/synthetic/pass{index}-{name}', bytes=M.stage_bytes(name), sha256='a' * 64))
                    for name, shape in M.SHAPES.items()}) for index in (1, 2)]


class FakeStages:
    def __init__(self):
        self.calls = []

    def add(self, name, tensor):
        self.calls.append((name, tensor))


class CapturePolicy(unittest.TestCase):
    def test_closed_plan_and_original_model_source_are_required(self):
        M.plan_shape(plan())
        for key in ('schema', 'implementation_sources', 'harness_sha256'):
            value = plan()
            if key == 'implementation_sources':
                value[key]['modeling_qwen3']['sha256'] = 'b' * 64
            else:
                value[key] = 'wrong'
            with self.assertRaises(ValueError):
                M.plan_shape(value)

    def test_new_namespace_cannot_overwrite_old_reference(self):
        for path in (str(M.E / 'framework-rearm-v224-v1'), '/tmp/layer0-framework-capture-v228-v1',
                     str(M.E / 'layer0-framework-capture-v228-v0')):
            value = plan()
            value['output_root'] = path
            with self.assertRaises(ValueError):
                M.plan_shape(value)

    def test_unknown_plan_fields_and_missing_source_roles_fail(self):
        value = plan()
        value['candidate_capture'] = '/not-allowed'
        with self.assertRaises(ValueError):
            M.plan_shape(value)
        value = plan()
        del value['implementation_sources']['sdpa']
        with self.assertRaises(ValueError):
            M.plan_shape(value)

    def test_separate_review_binds_entire_plan_projection(self):
        value = plan()
        accepted = review(value)
        M.reviewed(value, accepted, {'test_bound': 1})
        value['harness_sha256'] = 'f' * 64
        with self.assertRaises(ValueError):
            M.reviewed(value, accepted, {'test_bound': 1})

    def test_review_never_grants_numerical_or_performance_authority(self):
        value = plan()
        for key in ('reviewed', 'gpu_execution_authorized', 'numerical_acceptance',
                    'performance_claim', 'production_authority'):
            row = review(value)
            row[key] = not row[key]
            with self.assertRaises(ValueError):
                M.reviewed(value, row, {'test_bound': 1})

    def test_duplicate_and_nonfinite_json_are_rejected(self):
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}', b'{"a":Infinity}'):
            with self.assertRaises(ValueError):
                M.parse(raw)

    def test_exact_stage_roster_and_bf16_extents(self):
        data = values()
        self.assertEqual(len(data), 33)
        M.validate_values(data)
        del data['silu']
        with self.assertRaises(ValueError):
            M.validate_values(data)
        data = values()
        data['silu'] += b'\0\0'
        with self.assertRaises(ValueError):
            M.validate_values(data)

    def test_all_producer_consumer_joins_are_checked(self):
        for left, right in M.JOINS:
            data = values()
            data[right] = b'\1' + data[right][1:]
            with self.subTest(left=left, right=right), self.assertRaises(ValueError):
                M.validate_values(data)

    def test_stage_observer_uses_real_tensor_reader_once(self):
        calls = []
        def read(tensor, torch, shape):
            calls.append((tensor, torch, shape))
            return bytes(8192)
        torch, tensor = object(), object()
        stages = M.Stages(types.SimpleNamespace(bf16_raw=read), torch, lambda: calls.append('bounded'))
        stages.pre('embedding')(None, (tensor,))
        self.assertEqual(calls, [(tensor, torch, (1, 1, 4096)), 'bounded'])
        with self.assertRaises(ValueError):
            stages.post('embedding')(None, (), tensor)

    def test_stage_observer_propagates_dtype_or_finite_reader_refusal(self):
        def refuse(*_arguments):
            raise ValueError('real reader rejected dtype or nonfinite values')
        stages = M.Stages(types.SimpleNamespace(bf16_raw=refuse), object(), lambda: None)
        with self.assertRaises(ValueError):
            stages.add('silu', object())
        self.assertEqual(stages.values, {})

    def test_rotary_observer_returns_original_objects_and_selects_only_layer0(self):
        result = (object(), object())
        original = lambda *_args, **_kwargs: result
        module = types.SimpleNamespace(apply_rotary_pos_emb=original)
        stages = FakeStages()
        observer = M.RotaryObserver(module, stages)
        observer.install()
        self.assertIs(module.apply_rotary_pos_emb(), result)
        observer.enter(None, ())
        self.assertIs(module.apply_rotary_pos_emb('q', 'k'), result)
        observer.leave(None, (), result)
        observer.restore()
        self.assertIs(module.apply_rotary_pos_emb, original)
        self.assertEqual(observer.calls, 2)
        self.assertEqual(observer.selected, 1)
        self.assertEqual(stages.calls, [('rotary-q', result[0]), ('rotary-k', result[1])])

    def test_rotary_refuses_duplicate_selected_pair_and_nested_attention(self):
        module = types.SimpleNamespace(apply_rotary_pos_emb=lambda: (object(), object()))
        observer = M.RotaryObserver(module, FakeStages())
        observer.enter(None, ())
        with self.assertRaises(ValueError):
            observer.enter(None, ())
        observer.observe()
        with self.assertRaises(ValueError):
            observer.observe()

    def test_rotary_original_failure_restores_identity_in_finally(self):
        def original():
            raise ValueError('original failure')
        module = types.SimpleNamespace(apply_rotary_pos_emb=original)
        observer = M.RotaryObserver(module, FakeStages())
        observer.install()
        with self.assertRaisesRegex(ValueError, 'original failure'):
            try:
                module.apply_rotary_pos_emb()
            finally:
                observer.restore()
        self.assertIs(module.apply_rotary_pos_emb, original)

    def test_rotary_replacement_is_restored_but_refused(self):
        original = lambda: None
        module = types.SimpleNamespace(apply_rotary_pos_emb=original)
        observer = M.RotaryObserver(module, FakeStages())
        observer.install()
        module.apply_rotary_pos_emb = lambda: 'unexpected'
        with self.assertRaises(ValueError):
            observer.restore()
        self.assertIs(module.apply_rotary_pos_emb, original)

    def test_repeat_equality_needs_all_actual_stage_hashes(self):
        observed = passes()
        self.assertTrue(M.repeat_equal(observed))
        observed[1]['stages']['silu']['pin']['sha256'] = 'b' * 64
        self.assertFalse(M.repeat_equal(observed))

    def test_repeat_refuses_wrong_history_dtype_shape_and_missing_stage(self):
        for change in ('position', 'bool-position', 'dtype', 'shape', 'missing'):
            observed = copy.deepcopy(passes())
            if change in ('position', 'bool-position'):
                observed[1]['position'] = 1 if change == 'position' else False
            elif change == 'dtype':
                observed[1]['stages']['silu']['dtype'] = 'float32'
            elif change == 'shape':
                observed[1]['stages']['silu']['shape'] = [12288]
            else:
                del observed[1]['stages']['silu']
            with self.subTest(change=change), self.assertRaises(ValueError):
                M.repeat_equal(observed)


if __name__ == '__main__':
    unittest.main(verbosity=2)
