"""Synthetic HTTP projection fixtures, not native or HTTP timing evidence."""
import copy
import importlib.util
from pathlib import Path
import types
import unittest
from unittest import mock


ROOT = Path(__file__).parent


def module(name):
    definition = importlib.util.spec_from_file_location('gate_up_http_' + name, ROOT / (name + '.py'))
    value = importlib.util.module_from_spec(definition)
    definition.loader.exec_module(value)
    return value


m = module('selected_native')
base = module('test_selected_native')


def fixture(arm='B'):
    _, spec, result, _, report = base.fixture(arm)
    choice, commands = ('control', 652) if arm == 'A' else ('splitk4', 724)
    controller = {'path': '/private/gate-up-live', 'sha256': m.GATE_UP_LIVE}
    worker = {'path': '/private/gate-up-worker', 'sha256': m.GATE_UP_SOURCES['worker']}
    gate = {'artifact': dict(m.GATE_UP_IMAGE), 'artifact_path': '/private/image',
            'compiler_roster': {'path': '/private/roster.json', 'sha256': m.GATE_UP_ROSTER_SHA},
            'loaded_image_count': 10, 'scratch_bytes': 196608, 'prefill_unchanged': True}
    flags = {'--max-batches': '786', '--native-gate-up': choice, '--gate-up-artifact': gate['artifact_path'],
             '--gate-up-roster': gate['compiler_roster']['path'], '--gate-up-roster-sha256': m.GATE_UP_ROSTER_SHA,
             '--gate-up-hsaco-sha256': m.GATE_UP_IMAGE['artifact_hsaco_id'],
             '--gate-up-manifest-sha256': m.GATE_UP_IMAGE['artifact_manifest_id'],
             '--gate-up-handoff-sha256': m.GATE_UP_IMAGE['artifact_handoff_id']}
    argv = [controller['path'], '--native-prefill-rows', '32']
    for flag, value in flags.items():
        argv += [flag, value]
    spec.update(controller=controller, worker=worker, argv=argv, gate_up_expected=gate,
                closed_expected={'head_precision': 'fp32-v8'})
    selected = {**gate, 'enabled': arm == 'B', 'decode_dispatches': commands}
    result['setup'].update(max_batches=786, prefill_chunk=32,
        live_profile=f'prefill32-native649-decode{commands}-gate-up-{choice}-r1',
        prefill_program={'rows': 32, 'dispatches': 649, 'dynamic_slots': 396}, native_gate_up=selected)
    result['setup']['token_program']['backend'] = 'native-whole-program-slots512-v1'
    result['setup']['performance_profile']['native_gate_up'] = copy.deepcopy(selected)
    build = {'schema': 'FerricNativeGateUpBuildBindingR1', 'runtime_main': m.GATE_UP_CORE,
             **{key: {'path': '/private/' + key, 'sha256': value} for key, value in m.GATE_UP_SOURCES.items()},
             'controllers': {'live-A': controller, 'live-B': controller}}
    report.update(experiment='native-gate-up-control-vs-splitk4-tpot-r1', status='experimental-gates-passed',
        promotion_scope='explicit-gate-up-selector-native32-backend-only', ordered64_comparison_performed=False)
    return arm, spec, result, build, report


def project(values=None):
    return m.project_arm(*(values or fixture()), campaign='GateUpDa6b')


class GateUpSelectionTests(unittest.TestCase):
    def test_exact_both_arm_budgets_change_only_request_budget(self):
        for arm, dispatches in [('A', 3586926), ('B', 3970974)]:
            values = fixture(arm)
            before = copy.deepcopy(values)
            selected = project(values)
            self.assertEqual(values, before)
            expected = list(values[1]['argv'])
            expected[expected.index('--max-batches') + 1] = '5502'
            self.assertEqual(selected['argv'], expected)
            self.assertEqual(m.selected_geometry(selected), (5502, dispatches))
            self.assertEqual(selected['schema'], 'FerricGateUpDa6bSelectedHttpArmV1')
            self.assertFalse(selected['default_promotion'])

    def test_stale_sources_worker_or_either_controller_refused(self):
        for key in ['runtime_main', *m.GATE_UP_SOURCES, 'live-A', 'live-B']:
            values = fixture()
            if key.startswith('live-'):
                values[3]['controllers'][key] = {'sha256': 'f' * 64}
            else:
                values[3][key] = 'historical' if key == 'runtime_main' else {'sha256': 'f' * 64}
            with self.subTest(key=key), self.assertRaises(ValueError):
                project(values)

    def test_partial_inconclusive_or_wrong_scope_campaign_refused(self):
        for key, value in [('status', 'inconclusive'), ('status', 'pending'), ('status', 'promotable'),
                           ('promotion_scope', 'width-only'), ('experiment', 'other'),
                           ('default_promotion', True), ('vendor_comparison_performed', True),
                           ('ordered64_comparison_performed', True)]:
            values = fixture()
            values[4][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                project(values)

    def test_wrong_missing_or_duplicate_selected_flags_refused(self):
        for flag in ['--native-prefill-rows', '--native-gate-up', '--gate-up-artifact', '--gate-up-roster',
                     '--gate-up-roster-sha256', '--gate-up-hsaco-sha256', '--gate-up-manifest-sha256', '--gate-up-handoff-sha256']:
            for kind in ('wrong', 'missing', 'duplicate'):
                values = fixture()
                argv = values[1]['argv']
                index = argv.index(flag)
                if kind == 'wrong':
                    argv[index + 1] = 'wrong'
                elif kind == 'missing':
                    del argv[index:index + 2]
                else:
                    argv += argv[index:index + 2]
                with self.subTest(flag=flag, kind=kind), self.assertRaises(ValueError):
                    project(values)

    def test_image_roster_scratch_or_prefill_mutation_refused(self):
        for key in ('artifact', 'compiler_roster', 'loaded_image_count', 'scratch_bytes', 'prefill_unchanged'):
            values = fixture()
            values[1]['gate_up_expected'][key] = {} if key in ('artifact', 'compiler_roster') else -1
            with self.subTest(key=key), self.assertRaises(ValueError):
                project(values)

    def test_selected_setup_and_profile_do_not_admit_old_decode_counts(self):
        for scope in ('setup', 'profile'):
            for key, value in [('decode_dispatches', 652), ('enabled', False), ('scratch_bytes', 0)]:
                values = fixture()
                setup = values[2]['setup']
                target = setup if scope == 'setup' else setup['performance_profile']
                target['native_gate_up'][key] = value
                with self.subTest(scope=scope, key=key), self.assertRaises(ValueError):
                    project(values)

    def test_wrong_prefill_backend_or_instrumented_result_refused(self):
        for kind in ('rows', 'dispatches', 'dynamic_slots', 'backend', 'instrumented', 'runtime_profiling'):
            values = fixture()
            setup = values[2]['setup']
            if kind == 'instrumented':
                values[2][kind] = True
            elif kind == 'runtime_profiling':
                setup['performance_profile'][kind] = True
            elif kind == 'backend':
                setup['token_program'][kind] = 'ordinary-fallback'
            else:
                setup['prefill_program'][kind] = -1
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                project(values)

    def test_selected_http_accounting_cannot_reuse_old_control_total(self):
        selected = project()
        for key, value in [('model_batches', 5670), ('model_dispatches', 3586926), ('requests', 30)]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                m.selected_geometry({**selected, key: value})

    def test_closed_counts_are_exact_for_both_arms(self):
        for arm, count in [('A', 3586926), ('B', 3970974)]:
            selected = project(fixture(arm))
            def exact(actual, expected, message):
                m.require(all(actual.get(key) == value for key, value in expected.items()), message)
            cell = types.SimpleNamespace(composition=mock.Mock(), exact=exact)
            closed = {'schema': 'FerricQwen3TpBatchClosedV2', 'authority': 'none',
                      'execution_completed': True, 'all_workers_exited': True,
                      'worker_pids': [100], 'rank_dispatch_counts': [count], 'head_precision': 'fp32-v8'}
            m.validate_closed([closed], {'worker_pids': [100]}, selected, cell)
            with self.assertRaises(ValueError):
                m.validate_closed([{**closed, 'rank_dispatch_counts': [count - 1]}], {'worker_pids': [100]}, selected, cell)

    def test_new_campaign_cannot_borrow_old_width_or_prefill_report(self):
        for campaign in ('V17', 'V19', 'Width55c'):
            with self.subTest(campaign=campaign), self.assertRaises(ValueError):
                m.project_arm(*fixture(), campaign=campaign)
        self.assertEqual(m.ADAPTERS['GateUpDa6b'][0], '97f83d65302fd0d8fd276c57dc1454e4daf743259f733b0907c16b44eb542815')
        self.assertEqual(m.GATE_UP_CELL_SHA, 'f2536da4ad7f6906c1ca027cfdfe46b88bdf245a6b47d1b070e1089e5e61f35b')


if __name__ == '__main__':
    unittest.main()
