"""CPU-only selector/composition tests; no controller or worker is launched."""
import copy
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest import mock


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


ROOT = Path(__file__).parent
c = module('packet_launch_contract', ROOT / 'launch_contract.py')
cell = module('packet_cell', ROOT / 'measurement/native_token_cell.py')


def fixture():
    stage = Path('/dev/shm/ferric-packet-v19-55c-test')
    build = {'controllers': {'diagnostic': {'sha256': 'a' * 64}}, 'worker': {'sha256': c.WORKER_SHA}}
    images, observations = {}, {}
    for index, option in enumerate(c.IMAGE_FIELDS, 1):
        digest = f'{index:064x}'
        images[option] = {'path': str(stage / 'images' / str(index)),
                         'manifest_sha256': 'b' * 64, 'hsaco_sha256': digest}
        observations[option] = {'compiler_handoff': {'sha256': 'c' * 64},
            'hsaco': {'identity': {'sha256': digest}, 'kernel_names': [f'kernel_{index}']}}
    common = ['--worker', str(stage / 'worker-candidate'), '--worker-sha256', c.WORKER_SHA,
              '--device-unique-id', str(c.DEVICE), '--max-batches', '135', '--context', '8192',
              '--pages', '512', '--submission', 'ordered', '--layer-projection', 'c1-wave',
              '--live-stdin', '--runtime-cache-admission', '--runtime-operational', '--queue-rollover',
              '--disable-prefix-cache', '--prune-output-head']
    reference = {'generated_token_ids': list(range(128)), 'generated_utf8_hex': '41'}
    arguments = (stage, 'A', 'counters', common, images, observations, build, {'prompt': 'prompt'}, reference)
    return c.cell_spec(*arguments), arguments


def setup(spec):
    profile = {**cell.COMMON, **spec['profile_expected'], 'live_profile': cell.ticks.PROFILE,
        'attention': 'query-hoist-v14', 'runtime_profiling': False, 'dispatch_sequences': False,
        'runtime_cache_admission': True, 'runtime_operational': True, 'queue_rollover': True,
        'runtime_ordered_batches': True, 'projection': 'mfma', 'argmax_mode': 'wave-v11',
        'model_timestamps': cell.ticks.METADATA}
    return {**cell.COMMON, **spec['setup_expected'], 'live_profile': cell.ticks.PROFILE,
        'schema': 'FerricQwen3TpBatchSetupV2', 'authority': 'none', 'attention_mode': 'query-hoist-v14',
        'tensor_parallel': 1, 'model': 'Qwen/Qwen3-8B', 'dtype': 'BF16', 'target': 'gfx950:xnack-',
        'head_precision': 'fp32-v8', 'argmax_mode': 'wave-v11', 'runtime_ordered_batches': True,
        'prefix_cache': False, 'context_tokens': 8192, 'physical_pages': 512, 'prefill_chunk': 16,
        'batch_tokens': 32, 'performance_qualified': False, 'serving_qualified': False,
        'output_head_pruning': True, 'max_batches': 135, 'controller_sha256': spec['controller']['sha256'],
        'worker_sha256': c.WORKER_SHA, 'running_worker_sha256': [c.WORKER_SHA],
        'device_unique_ids': [c.DEVICE], 'worker_pids': [123], 'performance_profile': profile}


class PacketSelectorTests(unittest.TestCase):
    def test_exact_selector_private_sidecar_and_catalog(self):
        spec, _ = fixture()
        self.assertEqual(cell.shape(spec), (1, 0))
        self.assertEqual(spec['argv'][-2:], ['--ordered64-packet-ticks', spec['packet_sidecar']])
        self.assertEqual(len(spec['kernel_catalog']), 9)
        self.assertEqual(cell.check_setup(setup(spec), spec),
                         {'model_bundle_id': c.MODEL_BUNDLE, 'target_model_id': c.MODEL_TARGET})

    def test_image_hash_or_duplicate_export_rejects(self):
        _, arguments = fixture()
        for mutation in ('digest', 'symbol'):
            changed = copy.deepcopy(arguments)
            values = list(changed[5].values())
            if mutation == 'digest':
                values[0]['hsaco']['identity']['sha256'] = '0' * 64
            else:
                values[1]['hsaco']['kernel_names'] = values[0]['hsaco']['kernel_names']
            with self.assertRaises(ValueError):
                c.cell_spec(*changed)

    def test_no_native_or_fence_selector(self):
        for flag in ('--token-program-backend', '--token-program-fence-mode'):
            spec, _ = fixture()
            spec['argv'] += [flag, 'native']
            with self.assertRaises(ValueError):
                cell.shape(spec)

    def test_sidecar_path_and_single_request_are_fixed(self):
        for key, value in (('packet_sidecar', '/tmp/packet-ticks.json'), ('mode', 'latency'), ('arm', 'B')):
            spec, _ = fixture()
            spec[key] = value
            with self.assertRaises(ValueError):
                cell.shape(spec)

    def test_clock_metadata_cannot_claim_shader_time_or_nanoseconds(self):
        spec, _ = fixture()
        for key, value in (('unit', 'nanoseconds'), ('scope', 'shader_only'), ('performance_qualified', True)):
            actual = copy.deepcopy(setup(spec))
            actual['performance_profile']['model_timestamps'][key] = value
            with self.assertRaises(ValueError):
                cell.check_setup(actual, spec)

    def test_unrelated_diagnostics_and_top_level_timestamp_metadata_reject(self):
        spec, _ = fixture()
        for key in ('token_program', 'host_breakdown', 'prefill_program', 'model_timestamps'):
            actual = setup(spec)
            actual[key] = {}
            with self.assertRaises(ValueError):
                cell.check_setup(actual, spec)

    def test_stderr_counter_or_host_record_rejects(self):
        spec, _ = fixture()
        self.assertIsNone(cell.counter_replay(b'', spec, setup(spec)))
        with self.assertRaises(ValueError):
            cell.counter_replay(b'{}\n', spec, setup(spec))

    def test_binding_stays_disabled_without_actual_qualification(self):
        with mock.patch.object(c, 'PACKET_TICKS_QUALIFIED', False):
            with self.assertRaisesRegex(ValueError, 'actual controller and CPU evidence not qualified'):
                c.validate_build({})

    def test_staged_qualifiers_remain_importable_python_files(self):
        prepare = module('packet_prepare_stage', ROOT / 'prepare_stage.py')
        raw = b'VALUE = 55\n'
        digest = hashlib.sha256(raw).hexdigest()
        binding = {'path': '/retained/qualifier.py', 'sha256': digest}
        cpu = {'phases': {}, 'guard_sources': {}, 'runtime_evidence': {}, 'harness_evidence': {},
               'controller_evidence': {'qualifier': dict(binding)},
               'v19_evidence': {'qualifier': dict(binding)}}
        with tempfile.TemporaryDirectory() as temporary:
            stage = Path(temporary).resolve()
            files = {}
            with mock.patch.object(prepare.c, 'read', return_value=(raw, digest)):
                prepare.stage_cpu_receipts(stage, cpu, files)
            for field in ('controller_evidence', 'v19_evidence'):
                item = cpu[field]['qualifier']
                self.assertEqual(Path(item['path']).suffix, '')
                self.assertEqual(prepare.c.module(item['path'], item['sha256']).VALUE, 55)

    def check_module_name(self, name):
        raw = b'VALUE = 55\n'
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary).resolve() / name
            path.write_bytes(raw)
            loaded = c.module(path, hashlib.sha256(raw).hexdigest())
            self.assertEqual(loaded.VALUE, 55)
            self.assertEqual(loaded.__file__, str(path))

    def test_module_python_suffix(self):
        self.check_module_name('qualifier.py')

    def test_module_extensionless_hash_name(self):
        self.check_module_name('a' * 64)

    def test_module_bad_hash_rejects_before_execution(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary).resolve() / ('a' * 64)
            path.write_bytes(b'raise AssertionError("must not execute")\n')
            with self.assertRaisesRegex(ValueError, 'input hash mismatch'):
                c.module(path, '0' * 64)

    def test_prepare_summary_reports_validated_image_count(self):
        import ast
        tree = ast.parse((ROOT / 'prepare_stage.py').read_text())
        values = [value for node in ast.walk(tree) if isinstance(node, ast.Dict)
                  for key, value in zip(node.keys, node.values)
                  if isinstance(key, ast.Constant) and key.value == 'images'
                  and isinstance(value, ast.Call)]
        self.assertEqual(len(values), 1)
        self.assertEqual(ast.dump(values[0]), ast.dump(ast.parse('len(images)', mode='eval').body))


if __name__ == '__main__':
    unittest.main()
