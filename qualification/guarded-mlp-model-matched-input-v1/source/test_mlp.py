import copy
from pathlib import Path
import struct
import tempfile
import unittest

import common
import launch
import mlp


def tensor(count, word=0x3f80):
    return struct.pack('<%dH' % count, *([word] * count))


def fixture():
    original = {name: tensor(count) for name, count in mlp.STAGES.items()}
    matched = dict(original)
    matched_input = tensor(4096, 0x4000)
    for name in ('input', 'gate-input', 'up-input'):
        matched[name] = matched_input
    calls = {name: dict(original if name.startswith('control') else matched) for name in mlp.CALLS}
    native = {(rank, role): matched_input if role == 'post_normalized' else tensor(6144)
              for rank in (0, 1) for role in ('post_normalized', 'gate', 'up', 'activation')}
    return calls, native, original


class MlpTests(unittest.TestCase):
    def test_four_calls_and_six_rank_comparisons(self):
        calls, native, original = fixture()
        result = mlp.compare_calls(calls, native, original)
        self.assertTrue(result['control_and_repeat_gate_passed'])
        self.assertEqual(len(result['comparisons']), 6)
        self.assertTrue(all(row['matched_input']['exact_words'] == 6144 for row in result['comparisons']))
        self.assertEqual(result['module_calls'], 4)
        self.assertEqual(result['full_model_forward_calls'], 0)
        for qualified_method in (False, True):
            rows = [name + ' (test_mlp.MlpTests' + ('.' + name if qualified_method else '') + ') ... ok'
                    for name in launch.TEST_NAMES]
            raw = ('\n'.join(rows) + '\n\nRan 20 tests in 0.001s\n\nOK\n').encode()
            launch.test_census(raw)
            with self.assertRaises(ValueError):
                launch.test_census(raw.replace(rows[0].encode(), rows[1].encode()))
        with self.assertRaises(ValueError):
            launch.test_census(raw.replace(('.' + launch.TEST_NAMES[0] + ')').encode(), b'.test_wrong)'))

    def test_exact_rank_half_byte_units(self):
        first, second = tensor(6144), tensor(6144, 0x4000)
        self.assertEqual(mlp.rank_half(first + second, 0), first)
        self.assertEqual(mlp.rank_half(first + second, 1), second)
        with self.assertRaises(ValueError):
            mlp.rank_half(first, 0)

    def test_boolean_or_outside_rank_refused(self):
        for rank in (True, False, 2, -1, '0'):
            with self.subTest(rank=rank), self.assertRaises(ValueError):
                mlp.rank_half(tensor(12288), rank)

    def test_control_difference_retains_gate_without_comparisons(self):
        calls, native, original = fixture()
        for name in ('control-1', 'control-2'):
            calls[name]['down-projection'] = calls[name]['mlp-output'] = tensor(4096, 0x4000)
        result = mlp.compare_calls(calls, native, original)
        self.assertFalse(result['control_and_repeat_gate_passed'])
        self.assertIsNone(result['comparisons'])
        self.assertIsNone(result['framework_down_input_effect'])

    def test_repeated_native_difference_withholds_comparisons(self):
        calls, native, original = fixture()
        calls['native-2']['product'] = tensor(12288, 0x4000)
        result = mlp.compare_calls(calls, native, original)
        self.assertFalse(result['native_input_repeat_equal']['product'])
        self.assertIsNone(result['comparisons'])

    def test_swapped_native_control_input_refused(self):
        calls, native, original = fixture()
        calls['native-1'] = dict(original)
        with self.assertRaisesRegex(ValueError, 'actual module arguments'):
            mlp.compare_calls(calls, native, original)

    def test_product_not_silu_compared_to_activation(self):
        calls, native, original = fixture()
        for name in mlp.CALLS:
            calls[name]['silu'] = tensor(12288, 0x4040)
        original['silu'] = tensor(12288, 0x4040)
        result = mlp.compare_calls(calls, native, original)
        products = [row for row in result['comparisons'] if row['stage'] == 'product']
        self.assertEqual(len(products), 2)
        self.assertTrue(all(row['native_role'] == 'activation' and
                            row['matched_input']['exact_words'] == 6144 for row in products))
        self.assertFalse(result['native_standalone_silu_compared'])

    def test_diagnostic_authority_and_partial_exclusion(self):
        result = mlp.compare_calls(*fixture())
        for name in ('native_down_partials_compared_to_full_bf16', 'numerical_acceptance',
                     'full_model_acceptance', 'performance_claim', 'production_authority',
                     'matched_input_is_genuine_full_model_history'):
            self.assertIs(result[name], False)
        self.assertIsNone(result['acceptance_threshold'])

    def test_stage_extent_finiteness_and_semantic_joins(self):
        _, _, original = fixture()
        for name, replacement in (('up', b'\0'), ('silu', tensor(12288, 0x7f80)),
                                  ('silu-input', tensor(12288, 0x4000)),
                                  ('mlp-output', tensor(4096, 0x4000))):
            changed = dict(original); changed[name] = replacement
            with self.subTest(name=name), self.assertRaises(ValueError):
                mlp.validate_stages(changed, original['input'])

    def test_missing_stage_and_call_order_refused(self):
        calls, native, original = fixture()
        missing = dict(original); del missing['product']
        with self.assertRaises(ValueError):
            mlp.validate_stages(missing, original['input'])
        with self.assertRaises(ValueError):
            mlp.compare_calls(dict(reversed(list(calls.items()))), native, original)

    def test_hook_duplicate_and_closed_extent(self):
        saved = []
        hooks = mlp.Hooks(lambda raw, shape: raw, lambda name, raw: saved.append(name))
        hooks.add('input', tensor(4096))
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            hooks.add('input', tensor(4096))
        with self.assertRaises(ValueError):
            hooks.add('gate', tensor(4096))
        self.assertEqual(saved, ['input'])

    def test_all_actual_hook_handles_removed_in_reverse(self):
        removed = []
        class Handle:
            def __init__(self, number):
                self.number = number
            def remove(self):
                removed.append(self.number)
        class Module:
            count = 0
            def register_forward_pre_hook(self, callback):
                number = Module.count; Module.count += 1
                return Handle(number)
            register_forward_hook = register_forward_pre_hook
        module = Module()
        module.gate_proj, module.up_proj, module.act_fn, module.down_proj = (Module() for _ in range(4))
        hooks = mlp.Hooks(lambda raw, shape: raw, lambda name, raw: None)
        try:
            hooks.attach(module)
            raise ValueError('simulated module failure')
        except ValueError:
            pass
        finally:
            hooks.close()
        self.assertEqual(removed, list(reversed(range(9))))
        self.assertEqual(hooks.handles, [])

    def test_changed_source_pin_and_symlink_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'input'
            path.write_bytes(b'one')
            _, expected = common.read(path)
            path.write_bytes(b'two')
            with self.assertRaises(ValueError):
                common.read(path, expected)
            link = Path(directory) / 'link'
            link.symlink_to(path)
            with self.assertRaises(ValueError):
                common.read(link)

    def test_container_named_user_offline_and_single_gpu(self):
        value = launch.container_environment()
        self.assertEqual((value['USER'], value['LOGNAME'], value['HOME']),
                         ('harmenon', 'harmenon', '/scratch/home'))
        self.assertEqual(value['TORCHINDUCTOR_CACHE_DIR'], '/scratch/inductor')
        self.assertEqual(value['HIP_VISIBLE_DEVICES'], '0')
        self.assertNotIn('ROCR_VISIBLE_DEVICES', value)
        self.assertNotIn('CUDA_VISIBLE_DEVICES', value)
        for name in ('HF_HUB_OFFLINE', 'TRANSFORMERS_OFFLINE', 'HF_DATASETS_OFFLINE'):
            self.assertEqual(value[name], '1')

    def test_container_exact_readonly_mounts_and_no_network(self):
        argv = launch.build_command(Path('/private/packages'))
        mounts = [argv[i + 1] for i, item in enumerate(argv) if item == '--mount']
        for target in ('/model', '/source', '/inputs', '/packages'):
            self.assertEqual(sum(',dst=' + target + ',readonly' in item for item in mounts), 1)
        self.assertIn('--network=none', argv)
        self.assertIn('--read-only', argv)
        self.assertIn('--pull=never', argv)
        self.assertEqual(argv[-4:], ['/usr/bin/python3', '-I', '-B', '/source/run.py'])
        self.assertEqual(argv.count('/dev/dri/renderD128'), 1)

    def test_wrong_container_owner_or_image_refused(self):
        base = dict(Name='/' + launch.NAME, Config=dict(Image=launch.IMAGE,
                    Labels={'ferric.matched.owner': str(launch.ROOT)}), HostConfig=dict(NetworkMode='none'))
        self.assertEqual(launch.owned_container([base]), base)
        for field in ('owner', 'image', 'network', 'name'):
            changed = copy.deepcopy(base)
            if field == 'owner': changed['Config']['Labels']['ferric.matched.owner'] = '/other'
            if field == 'image': changed['Config']['Image'] = 'mutable:tag'
            if field == 'network': changed['HostConfig']['NetworkMode'] = 'host'
            if field == 'name': changed['Name'] = '/other'
            with self.subTest(field=field), self.assertRaises(ValueError):
                launch.owned_container([changed])

    def test_retirement_uses_separate_deadline(self):
        with self.assertRaises(ValueError):
            launch.leaf_guard(False, [], 1201, 1200, 30, 1200, lambda: None)
        launch.leaf_guard(True, [], 1201, 1200, 30, 1800, lambda: None)
        with self.assertRaises(ValueError):
            launch.leaf_guard(True, [], 1790, 1789, 30, 1800, lambda: None)

    def test_retirement_ignores_failed_workload_storage(self):
        def bad_storage():
            raise ValueError('scratch full')
        with self.assertRaisesRegex(ValueError, 'scratch full'):
            launch.leaf_guard(False, [], 1, 0, 30, 1200, bad_storage)
        launch.leaf_guard(True, [], 1, 0, 30, 1800, bad_storage)

    def test_retirement_signal_deferred_and_recorded(self):
        state = launch.OwnerSignals()
        state.retiring = True
        state(launch.signal.SIGTERM, None)
        state(launch.signal.SIGHUP, None)
        self.assertEqual(state.observed, [launch.signal.SIGTERM, launch.signal.SIGHUP])
        launch.leaf_guard(True, state.observed, 1, 0, 30, 1800, lambda: None)

    def test_work_signal_raises_between_cli_calls(self):
        state = launch.OwnerSignals()
        with self.assertRaises(InterruptedError):
            state(launch.signal.SIGTERM, None)
        self.assertEqual(state.observed, [launch.signal.SIGTERM])


if __name__ == '__main__':
    unittest.main()
