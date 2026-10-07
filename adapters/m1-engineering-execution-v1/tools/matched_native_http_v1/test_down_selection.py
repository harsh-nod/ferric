"""Synthetic HTTP projection fixtures, not native or HTTP timing evidence."""
import copy
import importlib.util
from pathlib import Path
import types
import unittest
from unittest import mock


ROOT = Path(__file__).parent


def module(name):
    definition = importlib.util.spec_from_file_location('down_http_' + name, ROOT / (name + '.py'))
    value = importlib.util.module_from_spec(definition)
    definition.loader.exec_module(value)
    return value


m = module('selected_native')
base = module('test_selected_native')


def fixture(arm='B'):
    _, spec, result, _, report = base.fixture(arm)
    choice, commands = ('control', 652) if arm == 'A' else ('splitk8', 688)
    controller = {'path': '/private/down-live', 'sha256': m.DOWN_LIVE}
    worker = {'path': '/private/down-worker', 'sha256': m.DOWN_SOURCES['worker']}
    gate = {'artifact': dict(m.DOWN_IMAGE), 'artifact_path': '/private/image',
            'compiler_roster': {'path': '/private/roster.json', 'sha256': m.DOWN_ROSTER_SHA},
            'loaded_image_count': 10, 'scratch_bytes': 131072, 'prefill_unchanged': True}
    flags = {'--max-batches': '786', '--native-down': choice, '--down-artifact': gate['artifact_path'],
             '--down-roster': gate['compiler_roster']['path'], '--down-roster-sha256': m.DOWN_ROSTER_SHA,
             '--down-hsaco-sha256': m.DOWN_IMAGE['artifact_hsaco_id'],
             '--down-manifest-sha256': m.DOWN_IMAGE['artifact_manifest_id'],
             '--down-handoff-sha256': m.DOWN_IMAGE['artifact_handoff_id']}
    argv = [controller['path'], '--native-prefill-rows', '32']
    for flag, value in flags.items():
        argv += [flag, value]
    spec.update(controller=controller, worker=worker, argv=argv, down_expected=gate,
                closed_expected={'head_precision': 'fp32-v8'})
    selected = {**gate, 'enabled': arm == 'B', 'decode_dispatches': commands}
    result['setup'].update(max_batches=786, prefill_chunk=32,
        live_profile=f'prefill32-native649-decode{commands}-down-{choice}-r1',
        prefill_program={'rows': 32, 'dispatches': 649, 'dynamic_slots': 396}, native_down=selected)
    result['setup']['token_program']['backend'] = 'native-whole-program-slots512-v1'
    result['setup']['performance_profile']['native_down'] = copy.deepcopy(selected)
    build = {'schema': 'FerricNativeDownBuildBindingR1', 'runtime_main': m.DOWN_CORE,
             **{key: {'path': '/private/' + key, 'sha256': value} for key, value in m.DOWN_SOURCES.items()},
             'controllers': {'live-A': controller, 'live-B': controller}}
    report.update(experiment='native-down-control-vs-splitk8-tpot-r1', status='experimental-gates-passed',
        promotion_scope='explicit-down-selector-native32-backend-only', ordered64_comparison_performed=False)
    return arm, spec, result, build, report


def project(values=None):
    return m.project_arm(*(values or fixture()), campaign='DownDa6b')


class DownSelectionTests(unittest.TestCase):
    def test_exact_both_arm_budgets_change_only_request_budget(self):
        for arm, dispatches in [('A', 3586926), ('B', 3778950)]:
            values = fixture(arm)
            before = copy.deepcopy(values)
            selected = project(values)
            self.assertEqual(values, before)
            expected = list(values[1]['argv'])
            expected[expected.index('--max-batches') + 1] = '5502'
            self.assertEqual(selected['argv'], expected)
            self.assertEqual(m.selected_geometry(selected), (5502, dispatches))
            self.assertEqual(selected['schema'], 'FerricDownDa6bSelectedHttpArmV1')
            self.assertFalse(selected['default_promotion'])

    def test_stale_sources_worker_or_either_controller_refused(self):
        for key in ['runtime_main', *m.DOWN_SOURCES, 'live-A', 'live-B']:
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
        for flag in ['--native-prefill-rows', '--native-down', '--down-artifact', '--down-roster',
                     '--down-roster-sha256', '--down-hsaco-sha256', '--down-manifest-sha256', '--down-handoff-sha256']:
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
            values[1]['down_expected'][key] = {} if key in ('artifact', 'compiler_roster') else -1
            with self.subTest(key=key), self.assertRaises(ValueError):
                project(values)

    def test_selected_setup_and_profile_do_not_admit_old_decode_counts(self):
        for scope in ('setup', 'profile'):
            for key, value in [('decode_dispatches', 652), ('enabled', False), ('scratch_bytes', 0)]:
                values = fixture()
                setup = values[2]['setup']
                target = setup if scope == 'setup' else setup['performance_profile']
                target['native_down'][key] = value
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
        for arm, count in [('A', 3586926), ('B', 3778950)]:
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

    def test_gate_up_hybrids_refused_even_with_valid_down_fields(self):
        for kind in ('expected', 'flag', 'equals_flag', 'setup', 'profile'):
            values = fixture()
            if kind == 'expected':
                values[1]['gate_up_expected'] = {}
            elif kind == 'flag':
                values[1]['argv'] += ['--native-gate-up', 'splitk4']
            elif kind == 'equals_flag':
                values[1]['argv'] += ['--native-gate-up=splitk4']
            elif kind == 'setup':
                values[2]['setup']['native_gate_up'] = {}
            else:
                values[2]['setup']['performance_profile']['native_gate_up'] = {}
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                project(values)

    def test_new_campaign_cannot_borrow_old_width_or_prefill_report(self):
        for campaign in ('V17', 'V19', 'Width55c', 'GateUpDa6b'):
            with self.subTest(campaign=campaign), self.assertRaises(ValueError):
                m.project_arm(*fixture(), campaign=campaign)
        self.assertEqual(m.ADAPTERS['DownDa6b'][0], '207af7b843cefba73da7beff658a213ee280bd21d2ea55a7ec6e6d84c5508739')
        self.assertEqual(m.DOWN_CELL_SHA, 'e59c15758711f40b0f0dac40af246f9471c8dc8938803c430775712fc548e9a3')


if __name__ == '__main__':
    unittest.main()
