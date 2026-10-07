"""CPU-only negative fixtures for the worker-only campaign transformation."""
import copy
import importlib.util
from pathlib import Path
import unittest

D = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('current_binding', D / 'current_binding.py')
b = importlib.util.module_from_spec(spec)
spec.loader.exec_module(b)
OLD = Path('/dev/shm/ferric-native-down-historical-a001')
NEW = Path('/dev/shm/ferric-native-down-current-a001')


def cells():
    result = []
    for cell_id, arm, mode in b.CELL_ORDER:
        common = ['--worker', str(OLD / 'worker-candidate'), '--worker-sha256', 'a' * 64,
                  '--max-batches', '786' if mode == 'latency' else '131']
        result.append({'cell_id': cell_id, 'output': str(OLD / 'cells' / cell_id),
            'common_args': common, 'spec': {'arm': arm, 'mode': mode,
                'controller': {'path': str(OLD / ('controller-' + arm)), 'sha256': 'b' * 64},
                'worker': {'path': str(OLD / 'worker-candidate'), 'sha256': 'a' * 64},
                'argv': [str(OLD / ('controller-' + arm))] + common + ['--native-down', arm],
                'reference': {'tokens': [1, 2, 3]}, 'timeouts': {'cell_seconds': 1200},
                'down_expected': {'scratch_bytes': 131072, 'loaded_image_count': 10}}})
    return result


def closure():
    return {'status': 0, 'returncode': 0, 'cleanup_ok': True, 'child_reaped': True,
            'term_sent': False, 'kill_sent': False, 'errors': [], 'reason': 'completed',
            'profile': 'FerricCpuFourCore45GiBEmitterV1'}


def receipt():
    return {'schema': 'FerricNativePacketCpuV1', 'mode': 'baseline', 'returncode': 0,
            'source_unchanged': True, 'native_executed': False, 'source_sha256': b.SOURCE_SHA,
            'binary': {'sha256': b.WORKER_SHA, 'bytes': b.WORKER_BYTES}, 'roles': [{
                'returncode': 0, 'role': 'build', 'argv': ['/cargo', 'build', '--release',
                    '--locked', '--offline', '--manifest-path', '/source/Cargo.toml', '-p',
                    'fe2o3-kfd', '--no-default-features', '--features', 'engineering-gfx950',
                    '--bin', 'fe2o3-gfx950-engineering-worker']}]}


def rosters():
    def item(pin, size=100, mode=0o600):
        return {'sha256': pin, 'bytes': size, 'mode': mode}
    previous = {'run_stage.py': item(b.GUARD_SHA), 'launch_contract.py': item(b.CONTRACT_SHA),
                'worker-candidate': item('a' * 64, 100, 0o700), 'image.hsaco': item('b' * 64),
                'measurement/native_token_cell.py': item('c' * 64)}
    sources = {name: value['sha256'] for name, value in previous.items() if name.endswith('.py')}
    files = copy.deepcopy(previous)
    files.update({name: item('d' * 64) for name in b.ADDED_FILES})
    files['worker-candidate'] = item(b.WORKER_SHA, b.WORKER_BYTES, 0o700)
    files['run_stage.py'] = item('e' * 64)
    files['historical/run_stage.py'] = copy.deepcopy(previous['run_stage.py'])
    files['historical/launch_contract.py'] = copy.deepcopy(previous['launch_contract.py'])
    return ({'files': files, 'sources': {**sources, **{name: files[name]['sha256'] for name in b.SOURCES}}},
            {'files': previous, 'sources': sources})


class CurrentBindingTests(unittest.TestCase):
    def setUp(self):
        self.original = cells()
        self.current = b.derive_cells(self.original, OLD, NEW)

    def rejected(self):
        with self.assertRaises(ValueError):
            b.validate_cells(self.current, self.original, OLD, NEW)

    def test_same_worker_all_fourteen_cells(self):
        b.validate_cells(self.current, self.original, OLD, NEW)
        self.assertEqual(len(self.current), 14)
        self.assertTrue(all(row['spec']['worker'] == {
            'path': str(NEW / 'worker-candidate'), 'sha256': b.WORKER_SHA} for row in self.current))

    def test_original_remains_immutable(self):
        self.assertEqual(self.original, cells())
        self.current[0]['spec']['reference']['tokens'].append(4)
        self.assertEqual(self.original, cells())

    def test_same_stage_rejected(self):
        with self.assertRaisesRegex(ValueError, 'fresh campaign'):
            b.derive_cells(self.original, OLD, OLD)

    def test_partial_old_campaign_rejected(self):
        with self.assertRaisesRegex(ValueError, 'fourteen'):
            b.derive_cells(self.original[:-4], OLD, NEW)

    def test_reordered_campaign_rejected(self):
        self.current[2], self.current[3] = self.current[3], self.current[2]
        self.rejected()

    def test_mixed_worker_rejected(self):
        self.current[3]['spec']['worker']['sha256'] = 'f' * 64
        self.rejected()

    def test_worker_argv_drift_rejected(self):
        self.current[3]['spec']['argv'][2] = str(OLD / 'worker-candidate')
        self.rejected()

    def test_duplicate_worker_selector_rejected(self):
        self.original[0]['common_args'].extend(['--worker', '/other'])
        with self.assertRaisesRegex(ValueError, 'one complete worker selector'):
            b.derive_cells(self.original, OLD, NEW)

    def test_incomplete_worker_selector_rejected(self):
        self.original[0]['common_args'] = ['--worker']
        with self.assertRaisesRegex(ValueError, 'one complete worker selector'):
            b.derive_cells(self.original, OLD, NEW)

    def test_controller_drift_rejected(self):
        self.current[0]['spec']['controller']['sha256'] = 'f' * 64
        self.rejected()

    def test_kernel_scratch_drift_rejected(self):
        self.current[0]['spec']['down_expected']['scratch_bytes'] *= 2
        self.rejected()

    def test_reference_drift_rejected(self):
        self.current[0]['spec']['reference']['tokens'][0] = 100
        self.rejected()

    def test_reused_old_output_rejected(self):
        self.current[0]['output'] = self.original[0]['output']
        self.rejected()

    def test_timeout_drift_rejected(self):
        self.current[0]['spec']['timeouts']['cell_seconds'] = 2400
        self.rejected()

    def test_valid_current_worker_receipt(self):
        b.validate_worker(receipt(), closure())

    def test_diagnostic_feature_rejected(self):
        value = receipt()
        value['roles'][0]['argv'][11] = 'engineering-native-packet-diagnostics'
        with self.assertRaisesRegex(ValueError, 'diagnostics excluded'):
            b.validate_worker(value, closure())

    def test_stale_worker_receipt_rejected(self):
        value = receipt()
        value['binary']['sha256'] = 'a' * 64
        with self.assertRaisesRegex(ValueError, 'exact current'):
            b.validate_worker(value, closure())

    def test_changed_source_rejected(self):
        value = receipt()
        value['source_unchanged'] = False
        with self.assertRaisesRegex(ValueError, 'exact current'):
            b.validate_worker(value, closure())

    def test_signaled_outer_rejected(self):
        for key, wrong in (('term_sent', True), ('kill_sent', True),
                           ('profile', 'FerricCpuFourCore48GiBEmitterV1')):
            with self.subTest(key=key):
                value = closure()
                value[key] = wrong
                with self.assertRaisesRegex(ValueError, 'unsignaled'):
                    b.validate_worker(receipt(), value)

    def test_unreaped_outer_rejected(self):
        value = closure()
        value['child_reaped'] = False
        with self.assertRaisesRegex(ValueError, 'closure'):
            b.validate_worker(receipt(), value)

    def test_exact_input_roster_accepted(self):
        b.validate_rosters(*rosters())

    def test_kernel_file_drift_rejected(self):
        current, original = rosters()
        current['files']['image.hsaco']['sha256'] = 'e' * 64
        with self.assertRaisesRegex(ValueError, 'historical artifact changed'):
            b.validate_rosters(current, original)

    def test_extra_unqualified_input_rejected(self):
        current, original = rosters()
        current['files']['new-kernel.hsaco'] = current['files']['image.hsaco']
        with self.assertRaisesRegex(ValueError, 'closed historical inputs'):
            b.validate_rosters(current, original)

    def test_changed_measurement_source_rejected(self):
        current, original = rosters()
        current['sources']['measurement/native_token_cell.py'] = 'f' * 64
        with self.assertRaisesRegex(ValueError, 'measurement remains historical'):
            b.validate_rosters(current, original)


if __name__ == '__main__':
    unittest.main()
