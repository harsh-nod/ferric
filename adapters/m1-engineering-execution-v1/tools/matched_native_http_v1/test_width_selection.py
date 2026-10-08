import copy
import hashlib
import importlib.util
from pathlib import Path
import types
import unittest
from unittest import mock


ROOT = Path(__file__).parent


def module(name):
    spec = importlib.util.spec_from_file_location('width_test_' + name, ROOT / (name + '.py'))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


m = module('selected_native')
c = module('matched128_contract')
base = module('test_selected_native')


def fixture(arm='B'):
    _, spec, result, _, report = base.fixture(arm)
    rows, batches, _, commands, slots = m.WIDTH_GEOMETRY[arm]
    controller = {'path': '/private/current55c-live', 'sha256': m.WIDTH_LIVE}
    worker = {'path': '/private/current55c-worker', 'sha256': m.WIDTH_SOURCES['worker']}
    spec.update(controller=controller, worker=worker,
                argv=[controller['path'], '--native-prefill-rows', str(rows), '--max-batches', str(6 * batches)])
    spec['closed_expected'] = {'head_precision': 'fp32-v8'}
    result['setup'].update(max_batches=6 * batches, prefill_chunk=rows,
        live_profile=f'prefill{rows}-native{commands}-slots512-worker-decode652-v1',
        prefill_program={'rows': rows, 'dispatches': commands, 'dynamic_slots': slots})
    result['setup']['token_program']['backend'] = 'native-whole-program-slots512-v1'
    build = {'schema': 'FerricPrefillWidthBuildBindingV1', 'runtime_main': m.WIDTH_CORE,
             **{key: {'path': '/private/' + key, 'sha256': sha} for key, sha in m.WIDTH_SOURCES.items()},
             'controllers': {'live-A': controller, 'live-B': controller}}
    report.update(experiment='native-prefill-width16-vs32-ttft-v1', status='promotable',
                  promotion_scope='prefill-width-within-slots512-native-backend-only',
                  ordered64_comparison_performed=False)
    return arm, spec, result, build, report


def project(values=None):
    return m.project_arm(*(values or fixture()), campaign='Width55c')


class WidthSelectionTests(unittest.TestCase):
    def test_exact_width_budgets_preserve_all_other_argv_and_setup_fields(self):
        for arm, budget, dispatches in [('A', 5670, 3683862), ('B', 5502, 3586926)]:
            values = fixture(arm)
            before = copy.deepcopy(values)
            selected = project(values)
            self.assertEqual(values, before)
            expected = list(values[1]['argv'])
            expected[expected.index('--max-batches') + 1] = str(budget)
            self.assertEqual(selected['argv'], expected)
            setup = {key: value for key, value in values[2]['setup'].items() if key not in m.SESSION_FIELDS}
            setup['max_batches'] = budget
            self.assertEqual(selected['expected_setup'], setup)
            self.assertEqual(m.selected_geometry(selected), (budget, dispatches))
            self.assertEqual(selected['schema'], 'FerricWidth55cSelectedHttpArmV1')

    def test_stale_core_source_or_worker_cannot_be_relabelled(self):
        for key in ['runtime_main', *m.WIDTH_SOURCES]:
            values = fixture()
            values[3][key] = 'historical' if key == 'runtime_main' else {'sha256': 'f' * 64}
            with self.assertRaises(ValueError):
                project(values)

    def test_both_live_aliases_must_be_the_exact_qualified_controller(self):
        for arm in ('A', 'B'):
            values = fixture()
            values[3]['controllers']['live-' + arm] = {'sha256': 'f' * 64}
            with self.assertRaises(ValueError):
                project(values)

    def test_historical_campaign_cannot_borrow_width_scope(self):
        for campaign in ('V17', 'V19'):
            with self.assertRaises(ValueError):
                m.project_arm(*fixture(), campaign=campaign)
        with self.assertRaises(ValueError):
            project(base.fixture())

    def test_incomplete_or_inconclusive_report_cannot_select_width(self):
        for status in ('inconclusive', 'pending', 'failed', None):
            values = fixture()
            values[4]['status'] = status
            with self.assertRaises(ValueError):
                project(values)

    def test_wrong_promotion_or_ordered_scope_refused(self):
        for key, value in [('promotion_scope', 'composed'), ('ordered64_comparison_performed', True),
                           ('default_promotion', True), ('vendor_comparison_performed', True)]:
            values = fixture()
            values[4][key] = value
            with self.assertRaises(ValueError):
                project(values)

    def test_wrong_or_duplicated_width_selector_refused(self):
        for argv in [
                ['/private/current55c-live', '--native-prefill-rows', '16', '--max-batches', '786'],
                ['/private/current55c-live', '--max-batches', '786', '--native-prefill-rows', '32'],
                fixture()[1]['argv'] + ['--native-prefill-rows', '32']]:
            values = fixture()
            values[1]['argv'] = argv
            with self.assertRaises(ValueError):
                project(values)

    def test_stale_six_request_budget_in_argv_or_setup_refused(self):
        for target in ('argv', 'setup'):
            values = fixture()
            if target == 'argv':
                values[1]['argv'][-1] = '810'
            else:
                values[2]['setup']['max_batches'] = 810
            with self.assertRaises(ValueError):
                project(values)

    def test_width_setup_profile_or_slots_drift_refused(self):
        for key in ('prefill_chunk', 'live_profile', 'rows', 'dispatches', 'dynamic_slots', 'backend'):
            values = fixture()
            setup = values[2]['setup']
            if key in ('prefill_chunk', 'live_profile'):
                setup[key] = 'wrong'
            elif key == 'backend':
                setup['token_program'][key] = 'native-whole-program-v1'
            else:
                setup['prefill_program'][key] = -1
            with self.assertRaises(ValueError):
                project(values)

    def test_selected_geometry_cannot_use_forged_http_counters(self):
        for key in ('requests', 'model_batches', 'model_dispatches', 'schema', 'arm'):
            selected = project()
            selected[key] = 'wrong'
            with self.assertRaises(ValueError):
                m.selected_geometry(selected)

    def test_width_closed_dispatch_count_is_42_times_selected_geometry(self):
        for arm, dispatches in [('A', 3683862), ('B', 3586926)]:
            selected = project(fixture(arm))
            setup = {'worker_pids': [100]}
            def exact(actual, expected, message):
                m.require(all(actual.get(key) == value for key, value in expected.items()), message)
            cell = types.SimpleNamespace(composition=mock.Mock(), exact=exact)
            closed = {'schema': 'FerricQwen3TpBatchClosedV2', 'authority': 'none',
                      'execution_completed': True, 'all_workers_exited': True,
                      'worker_pids': [100], 'rank_dispatch_counts': [dispatches], 'head_precision': 'fp32-v8'}
            m.validate_closed([closed], setup, selected, cell)
            cell.composition.assert_called_once_with(closed, selected['native_spec'])
            with self.assertRaises(ValueError):
                m.validate_closed([{**closed, 'rank_dispatch_counts': [dispatches + 1]}], setup, selected, cell)

    def test_actual_width_setup_cannot_change_any_extra_field(self):
        selected = project()
        actual = {**selected['expected_setup'], 'worker_pids': [100], 'session_id': 'new', 'setup_seconds': 0.5}
        m.validate_setup(actual, selected)
        with self.assertRaises(ValueError):
            m.validate_setup({**actual, 'packed_down_mode': 'split-k'}, selected)

    def test_width_adapter_pins_exact_enabled_contract_parser_and_runtime(self):
        self.assertEqual(m.ADAPTERS['Width55c'][:2], (
            '9bbc328a6aa3fc1c8a351a7bcde0223f5dda2a5c923f320d8a0f74e1047a84c0',
            'b08dcaf72398c6778c21bbdfae7726e7476acc3281d927a96ae5e2bb8ea6a07b'))
        self.assertEqual(m.WIDTH_CELL_SHA, '866218ffa64d6e0078fee7f4088b2150527fbee233461116fd64f42cbc5dd2e4')
        self.assertEqual(m.WIDTH_CORE, '55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9')

    def test_r6_settings_client_and_precision_policy_unchanged(self):
        self.assertEqual(c.SETTINGS, {'prompt_tokens': 128, 'completion_tokens': 128, 'concurrency': 1,
            'warmups': 10, 'samples': 30, 'temperature': 0, 'seed': 0, 'ignore_eos': True,
            'speculation': False, 'prefix_cache': False, 'context': 8192, 'dtype': 'bfloat16',
            'head_dtype': 'float32', 'arrival': 'closed-loop', 'request_timeout_seconds': 120,
            'ttft_slo_ms': 10000, 'tpot_slo_ms': 1000})
        self.assertEqual(c.CLIENT_SHA, '979136caea4f134f33f19c62b82a8ac9537205eaa11d11a43b7a3af466af0a2d')
        self.assertEqual(c.SERVING_SHA, '347754cb8d88da4639beb0163f54c0ca091288e9c11d6e47e29e47db1dddaf7d')
        self.assertEqual(c.POLICY, 'exact-greedy-token-ids-and-decoded-utf8-v1')

    def test_shared_client_and_exact_startup_reconciler_source_pins(self):
        pins = {
            'gpu_activity.py': '4cc1bb26e5eea7bd235ca4c014fd205d4d9fffdf35d96c1b9a2cad1233cd03e2',
            'gpu_attribution.py': 'a077d7d1309166b7a216643b10fcbddb6e7a729142ea2c23af1513990a515f51',
            'http_lifecycle.py': '075cd929cd6df73c2768959eabdab7548f5611b84c8f54200d8b71494ed6f5d8',
            'owned_command.py': 'cd6159e8a86358335ee7122956590e539b5fcf52ac1f79e839f72811b378d245',
            'gpu_probe.py': '40a2795d5126ba9a8b29007a2134fbaf8a80c52af2cc97b85cc88a9c2ba01364',
            'probe_cli.py': 'ee85acf4bff53b1f317f89e85a88b2b20d80f693f0a5e4fec2ff5058f0c1d0ff',
            'container_custody.py': 'e37085a7a4078a33337b05e8a6155b71dbae5d05869819588048049e65109570',
            'sitecustomize.py': '540c105f4a10e474c98a0d54a1900afc963d6c6fae139ab2613bbb8a4eb329a5',
            'competitive_benchmark.py': c.CLIENT_SHA, 'frozen/serve_ferric.py': c.SERVING_SHA,
            'frozen/gpu_activity.py': m.SCANNER_SHA, 'frozen/native_lifecycle.py': m.LIFECYCLE_SHA}
        for name, expected in pins.items():
            self.assertEqual(hashlib.sha256((ROOT / name).read_bytes()).hexdigest(), expected, name)

    def test_vllm_image_and_complete_server_argument_tail_unchanged(self):
        argv = c.baseline_argv('vllm', 'owned', '/cache', '/dev/dri/renderD128', '/observer', '/evidence')
        self.assertEqual(c.IMAGES['vllm']['id'],
            'sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba')
        self.assertEqual(argv[argv.index('--entrypoint'):], ['--entrypoint', 'vllm',
            'vllm/vllm-openai-rocm@sha256:e0a3b2bd3fe7ec563916c3a5d949898d133458c18d6b2f460c906885cfb32032',
            'serve', '/model', '--host', '127.0.0.1', '--port', '18981', '--served-model-name', 'Qwen/Qwen3-8B',
            '--dtype', 'bfloat16', '--tensor-parallel-size', '1', '--max-model-len', '8192', '--max-num-seqs', '1',
            '--gpu-memory-utilization', '0.25', '--kv-cache-dtype', 'auto', '--no-enable-prefix-caching',
            '--generation-config', 'vllm', '--hf-overrides', '{"head_dtype":"float32"}', '--shutdown-timeout', '20'])

    def test_http_contract_uses_selected_width_budget_not_historical_constant(self):
        for arm in ('A', 'B'):
            selected = project(fixture(arm))
            selected['native_evidence'] = {'test_only': True}
            selected['argv'] += ['--source', str(Path(c.TARGET).parent), '--device-unique-id', str(c.GPU_IDS[0]),
                                 '--worker', selected['worker']['path'], '--worker-sha256', selected['worker']['sha256'],
                                 '--context', '8192', '--pages', '512']
            adapter = types.SimpleNamespace(admit=lambda _: selected, selected_geometry=m.selected_geometry)
            digests = {selected[key]['path']: selected[key]['sha256'] for key in ('controller', 'worker')}
            with mock.patch.object(c, 'selection_adapter', return_value=adapter), \
                    mock.patch.object(c, 'digest', side_effect=digests.__getitem__):
                self.assertEqual(c.validate_ferric(selected, {}), selected)
                selected['argv'][selected['argv'].index('--max-batches') + 1] = '1'
                with self.assertRaises(ValueError):
                    c.validate_ferric(selected, {})

    def test_enabled_checkpoint_preserves_disabled_gate_before_any_plan_read(self):
        driver = module('run_matched128')
        self.assertTrue(driver.NATIVE_EXECUTION_ENABLED)
        for flag in ('--execute', '--supervise'):
            argv = ['run_matched128.py', flag, '--engine', 'ferric',
                    '--plan', '/private/not-read/plan.json', '--plan-sha256', 'a' * 64,
                    '--output-dir', '/dev/shm/ferric-native-http-ferric-test']
            with mock.patch.object(driver, 'NATIVE_EXECUTION_ENABLED', False), \
                    mock.patch.object(driver.sys, 'argv', argv), \
                    mock.patch.object(driver.os, 'umask'), \
                    mock.patch.object(driver, 'read') as reader:
                with self.assertRaisesRegex(ValueError, 'current55c width HTTP execution disabled'):
                    driver.main()
                reader.assert_not_called()


if __name__ == '__main__':
    unittest.main()
