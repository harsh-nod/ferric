"""Current-worker projection fixtures; no server or native execution."""
import copy
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).parent


def module(name):
    spec = importlib.util.spec_from_file_location('current_down_' + name, ROOT / (name + '.py'))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


m = module('selected_native')
old = module('test_down_selection')


def fixture(arm='B'):
    values = old.fixture(arm)
    values[1]['worker'] = {'path': '/private/current-worker', 'sha256': m.CURRENT_WORKER_SHA}
    values[4]['worker_refresh'] = copy.deepcopy(m.CURRENT_REFRESH)
    return values


def project(values=None):
    return m.project_arm(*(values or fixture()), campaign='Down1736')


class CurrentDownTests(unittest.TestCase):
    def test_both_arms_keep_exact_geometry_and_explicit_ancestry(self):
        for arm, dispatches in [('A', 3586926), ('B', 3778950)]:
            values = fixture(arm)
            before = copy.deepcopy(values)
            selected = project(values)
            self.assertEqual(values, before)
            self.assertEqual(m.selected_geometry(selected), (5502, dispatches))
            self.assertEqual(selected['schema'], 'FerricDown1736SelectedHttpArmV1')
            self.assertEqual(selected['worker_refresh'], m.CURRENT_REFRESH)
            self.assertTrue(selected['runtime_source_is_historical_ancestry'])
            self.assertEqual(selected['worker']['sha256'], m.CURRENT_WORKER_SHA)
            self.assertFalse(selected['default_promotion'])
            self.assertFalse(selected['vendor_comparison_performed'])

    def test_old_worker_is_not_current(self):
        values = fixture()
        values[1]['worker']['sha256'] = m.DOWN_SOURCES['worker']
        with self.assertRaises(ValueError):
            project(values)

    def test_current_worker_cannot_be_relabelled_historical(self):
        with self.assertRaises(ValueError):
            m.project_arm(*fixture(), campaign='DownDa6b')

    def test_refresh_report_is_mandatory(self):
        values = fixture()
        del values[4]['worker_refresh']
        with self.assertRaises(ValueError):
            project(values)

    def test_every_refresh_field_is_exact(self):
        for key in m.CURRENT_REFRESH:
            values = fixture()
            values[4]['worker_refresh'][key] = None
            with self.subTest(key=key), self.assertRaises(ValueError):
                project(values)

    def test_historical_controller_and_build_are_not_rewritten(self):
        for key in ('runtime_main', 'worker', 'runtime_source', 'controller_source'):
            values = fixture()
            values[3][key] = 'current' if key == 'runtime_main' else {'sha256': m.CURRENT_WORKER_SHA}
            with self.subTest(key=key), self.assertRaises(ValueError):
                project(values)

    def test_failing_campaign_is_not_admitted(self):
        for key, value in [('status', 'inconclusive'), ('default_promotion', True),
                           ('vendor_comparison_performed', True)]:
            values = fixture()
            values[4][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                project(values)

    def test_current_must_not_compose_gate_up(self):
        values = fixture()
        values[1]['argv'] += ['--native-gate-up', 'splitk4']
        with self.assertRaises(ValueError):
            project(values)

    def test_current_diagnostics_cannot_enter_timing(self):
        for scope, field in [('result', 'instrumented'), ('token', 'counter_diagnostic'),
                             ('profile', 'runtime_profiling')]:
            values = fixture()
            target = values[2] if scope == 'result' else values[2]['setup'][
                'token_program' if scope == 'token' else 'performance_profile']
            target[field] = True
            with self.subTest(scope=scope), self.assertRaises(ValueError):
                project(values)

    def test_current_uses_same_images_and_down_decoder(self):
        self.assertEqual(m.ADAPTERS['Down1736'], m.ADAPTERS['DownDa6b'])
        values = fixture()
        values[1]['down_expected']['scratch_bytes'] = 0
        with self.assertRaises(ValueError):
            project(values)


if __name__ == '__main__':
    unittest.main()
