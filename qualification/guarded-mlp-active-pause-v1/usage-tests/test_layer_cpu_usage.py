"""Pure child-usage arithmetic and source-placement tests; no process or GPU launch."""
import ast
import copy
from pathlib import Path
import types
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
FIELDS = ('ru_utime', 'ru_stime', 'ru_nvcsw', 'ru_nivcsw', 'ru_minflt', 'ru_majflt')


def load(name):
    path = HERE / name
    module = types.ModuleType('usage_test_' + path.stem)
    module.__file__ = str(path)
    exec(compile(path.read_bytes(), str(path), 'exec'), module.__dict__)
    return module


def snapshot():
    return dict(ru_utime=12.5, ru_stime=3.25, ru_nvcsw=100, ru_nivcsw=20,
                ru_minflt=300, ru_majflt=4)


def record():
    before = snapshot()
    after = dict(ru_utime=13.75, ru_stime=3.75, ru_nvcsw=109, ru_nivcsw=22,
                 ru_minflt=317, ru_majflt=5)
    return dict(who='RUSAGE_CHILDREN', before=before, after=after,
                delta=dict(ru_utime=1.25, ru_stime=0.5, ru_nvcsw=9, ru_nivcsw=2,
                           ru_minflt=17, ru_majflt=1), error=None)


def function(tree, name):
    return next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == name)


def calls(node, name):
    return [item for item in ast.walk(node) if isinstance(item, ast.Call)
            and ((isinstance(item.func, ast.Name) and item.func.id == name)
                 or (isinstance(item.func, ast.Attribute) and item.func.attr == name))]


class LayerCpuUsageTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.runner = load('run_layer_model_gpu.py')
        cls.retainer = load('layer_retention_tool.py')
        cls.modules = (cls.runner, cls.retainer)
        cls.runner_tree = ast.parse((HERE / 'run_layer_model_gpu.py').read_bytes())
        cls.retainer_tree = ast.parse((HERE / 'layer_retention_tool.py').read_bytes())

    def test_nonzero_cumulative_baseline_is_subtracted_exactly(self):
        value = record()
        for module in self.modules:
            with self.subTest(module=module.__name__):
                self.assertEqual(module.children_rusage_delta(value['before'], value['after']), value['delta'])
                self.assertIsNone(module.validate_children_rusage(value))
                self.assertEqual(value, record())

    def test_zero_delta_preserves_float_seconds_and_integer_counts(self):
        before = snapshot()
        for module in self.modules:
            delta = module.children_rusage_delta(before, copy.deepcopy(before))
            self.assertEqual(delta, dict.fromkeys(FIELDS, 0))
            for name in FIELDS:
                self.assertIs(type(delta[name]), float if name in FIELDS[:2] else int)

    def test_snapshot_has_exact_selected_fields_and_ignores_maxrss(self):
        raw = types.SimpleNamespace(**snapshot(), ru_maxrss=999999, ru_inblock=123)
        with patch.object(self.runner.resource, 'getrusage', return_value=raw) as sample:
            self.assertEqual(self.runner.children_rusage_snapshot(), snapshot())
            sample.assert_called_once_with(self.runner.resource.RUSAGE_CHILDREN)
        raw.ru_maxrss = 1
        with patch.object(self.runner.resource, 'getrusage', return_value=raw):
            self.assertEqual(self.runner.children_rusage_snapshot(), snapshot())
        self.assertNotIn('ru_maxrss', record()['delta'])

    def test_snapshot_refuses_bad_shape_and_nonexact_numeric_types(self):
        bad_rows = [None, [], tuple(snapshot().items())]
        for field in FIELDS:
            missing = snapshot(); del missing[field]; bad_rows.append(missing)
            for value in ([True, 1, '1', None] if field in FIELDS[:2] else [True, 1.0, 0.5, '1', None]):
                row = snapshot(); row[field] = value; bad_rows.append(row)
        extra = snapshot(); extra['ru_maxrss'] = 99; bad_rows.append(extra)
        for module in self.modules:
            for row in bad_rows:
                with self.subTest(module=module.__name__, row=row):
                    with self.assertRaises(RuntimeError):
                        module.children_rusage_delta(row, snapshot())
                    with self.assertRaises(RuntimeError):
                        module.children_rusage_delta(snapshot(), row)

    def test_snapshot_refuses_nonfinite_negative_and_decreasing_values(self):
        for module in self.modules:
            for field in FIELDS:
                bad = [float('nan'), float('inf'), -float('inf'), -0.5] if field in FIELDS[:2] else [-1]
                for value in bad:
                    row = snapshot(); row[field] = value
                    with self.subTest(module=module.__name__, field=field, value=value):
                        with self.assertRaises(RuntimeError):
                            module.children_rusage_delta(row, row)
                after = snapshot(); after[field] -= 0.25 if field in FIELDS[:2] else 1
                with self.assertRaises(RuntimeError):
                    module.children_rusage_delta(snapshot(), after)

    def test_record_rejects_wrong_delta_values_types_or_fields(self):
        for module in self.modules:
            for field in FIELDS:
                for value in (True, '0', None, record()['delta'][field] + 1,
                              int(record()['delta'][field]) if field in FIELDS[:2]
                              else float(record()['delta'][field])):
                    changed = record(); changed['delta'][field] = value
                    with self.subTest(module=module.__name__, field=field, value=value):
                        with self.assertRaises(RuntimeError):
                            module.validate_children_rusage(changed)
                changed = record(); del changed['delta'][field]
                with self.assertRaises(RuntimeError):
                    module.validate_children_rusage(changed)
            changed = record(); changed['delta']['ru_maxrss'] = 0
            with self.assertRaises(RuntimeError):
                module.validate_children_rusage(changed)

    def test_record_rejects_unknown_scope_keys_and_missing_snapshots(self):
        mutations = []
        for field in ('who', 'before', 'after', 'delta', 'error'):
            changed = record(); del changed[field]; mutations.append(changed)
        changed = record(); changed['extra'] = False; mutations.append(changed)
        changed = record(); changed['who'] = 'RUSAGE_SELF'; mutations.append(changed)
        for field in ('before', 'after', 'delta'):
            changed = record(); changed[field] = None; mutations.append(changed)
        for module in self.modules:
            for changed in mutations:
                with self.assertRaises(RuntimeError):
                    module.validate_children_rusage(changed)

    def test_incomplete_sample_is_failure_only_and_preserves_valid_raw_snapshot(self):
        for module in self.modules:
            for before, after in ((None, None), (snapshot(), None), (snapshot(), snapshot())):
                value = dict(who='RUSAGE_CHILDREN', before=before, after=after, delta=None,
                             error='after child CPU usage: synthetic sampling failure')
                preserved = copy.deepcopy(value)
                self.assertIsNone(module.validate_children_rusage(value, complete=False))
                self.assertEqual(value, preserved)
                with self.assertRaises(RuntimeError):
                    module.validate_children_rusage(value)

    def test_incomplete_record_rejects_hidden_delta_and_malformed_failure(self):
        invalid = []
        for error in ('', 'x' * 1025, 1, True):
            invalid.append(dict(who='RUSAGE_CHILDREN', before=None, after=None, delta=None, error=error))
        invalid.append(dict(who='RUSAGE_CHILDREN', before=None, after=snapshot(), delta=None, error='failed'))
        invalid.append(dict(who='RUSAGE_CHILDREN', before=snapshot(), after=None,
                            delta=record()['delta'], error='failed'))
        invalid.append(dict(who='RUSAGE_CHILDREN', before={'ru_utime': 1.0}, after=None,
                            delta=None, error='failed'))
        for module in self.modules:
            for changed in invalid:
                with self.assertRaises(RuntimeError):
                    module.validate_children_rusage(changed, complete=False)

    def test_sampling_error_or_malformed_kernel_tuple_is_not_coerced(self):
        for error in (OSError('synthetic sample failure'), KeyboardInterrupt('synthetic interruption')):
            with patch.object(self.runner.resource, 'getrusage', side_effect=error):
                with self.assertRaises(type(error)):
                    self.runner.children_rusage_snapshot()
        invalid = types.SimpleNamespace(**snapshot())
        invalid.ru_utime = 1
        with patch.object(self.runner.resource, 'getrusage', return_value=invalid):
            with self.assertRaises(RuntimeError):
                self.runner.children_rusage_snapshot()

    def test_runner_and_retainer_use_identical_pure_validation(self):
        for name in ('children_rusage_delta', 'validate_children_rusage'):
            self.assertEqual(ast.dump(function(self.runner_tree, name), include_attributes=False),
                             ast.dump(function(self.retainer_tree, name), include_attributes=False))

    def test_native_samples_bracket_spawn_and_complete_owned_cleanup_before_save(self):
        bounded = function(self.runner_tree, 'bounded')
        outer = next(node for node in bounded.body if isinstance(node, ast.Try) and calls(node, 'Popen'))
        spawn, = calls(outer, 'Popen')
        samples = sorted(calls(bounded, 'children_rusage_snapshot'), key=lambda node: node.lineno)
        self.assertEqual(len(samples), 2)
        cleanup, = calls(outer, 'cleanup')
        emergency, = calls(outer, 'emergency_reap')
        close, = calls(outer, 'close_fds')
        self.assertLess(samples[0].lineno, spawn.lineno)
        self.assertTrue(any(cleanup in list(ast.walk(node)) for node in outer.finalbody))
        self.assertLess(max(cleanup.lineno, emergency.lineno, close.lineno, outer.end_lineno),
                        samples[1].lineno)
        result_saves = [node for node in calls(bounded, 'save')
                        if any(isinstance(value, ast.Constant) and value.value == 'result.json'
                               for value in ast.walk(node.args[0]))]
        self.assertEqual(len(result_saves), 1)
        self.assertLess(samples[1].lineno, result_saves[0].lineno)
        condition = ast.dump(ast.parse('seconds == NATIVE_SECONDS', mode='eval').body)
        for sample in samples:
            self.assertTrue(any(isinstance(node, ast.If) and ast.dump(node.test) == condition
                                and sample in list(ast.walk(node)) for node in ast.walk(bounded)))

    def test_sampling_failure_handlers_cannot_bypass_owned_finally(self):
        bounded = function(self.runner_tree, 'bounded')
        sample_tries = [node for node in ast.walk(bounded) if isinstance(node, ast.Try)
                       and any(calls(body, 'children_rusage_snapshot') for body in node.body)]
        before = min(sample_tries, key=lambda node: node.end_lineno)
        after = max(sample_tries, key=lambda node: node.lineno)
        self.assertTrue(any(isinstance(node, ast.Raise) for node in ast.walk(before.handlers[0])))
        self.assertFalse(any(isinstance(node, (ast.Raise, ast.Return)) for node in ast.walk(after.handlers[0])))
        self.assertEqual(ast.dump(after.handlers[0].type), ast.dump(ast.Name(id='BaseException', ctx=ast.Load())))
        self.assertTrue(any(isinstance(node, ast.Assign) and any(isinstance(target, ast.Name)
                            and target.id == 'reason' for target in node.targets)
                            for node in ast.walk(after.handlers[0])))
        self.assertTrue(any(isinstance(node, ast.Assign) and isinstance(node.value, ast.Constant)
                            and node.value.value is None and any(isinstance(target, ast.Subscript)
                            and isinstance(target.slice, ast.Constant) and target.slice.value == 'delta'
                            for target in node.targets) for node in ast.walk(after.handlers[0])))

    def test_retainer_checks_current_parent_only_and_preserves_historical_function(self):
        checks = calls(self.retainer_tree, 'validate_children_rusage')
        self.assertEqual(len(checks), 1)
        owner = next(node for node in ast.walk(self.retainer_tree) if isinstance(node, ast.If)
                     and checks[0] in list(ast.walk(node))
                     and ast.dump(node.test) == ast.dump(ast.parse("label == 'parent'", mode='eval').body))
        self.assertTrue(any(isinstance(node, ast.Constant)
                            and node.value == 'child CPU usage only covers the native parent phase'
                            for item in owner.orelse for node in ast.walk(item)))
        baseline = function(self.retainer_tree, 'verify_baseline')
        self.assertFalse(calls(baseline, 'validate_children_rusage'))
        self.assertFalse(any(isinstance(node, ast.Constant) and node.value == 'children_rusage'
                             for node in ast.walk(baseline)))


if __name__ == '__main__':
    unittest.main()
