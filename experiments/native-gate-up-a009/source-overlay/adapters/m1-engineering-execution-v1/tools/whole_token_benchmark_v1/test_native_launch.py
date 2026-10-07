"""CPU-only fixtures; no SSH, device, sudo, controller or compiler launches."""
import copy
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

D = Path(__file__).resolve().parent


def load(name):
    spec = importlib.util.spec_from_file_location('test_' + name, D / (name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


c, guard, prepare = load('launch_contract'), load('run_stage'), load('prepare_stage')


def receipt(path, digit='a'):
    return {'path': path, 'sha256': digit * 64}


def build():
    return {'schema': 'FerricV14NativeBuildBindingV1', 'runtime_main': c.RUNTIME_MAIN,
        'runtime_source': receipt('/input/runtime.tar'), 'controller_source': receipt('/input/controller.tar', 'b'),
        'worker': receipt('/input/worker', 'c'), 'controllers': {
            'A': receipt('/input/a', 'd'), 'B': receipt('/input/b', 'e'), 'counters': receipt('/input/counters', 'f')},
        'cpu_qualification': receipt('/input/cpu.json', '1')}


def cpu():
    value = build()
    return {'schema': 'FerricV14CpuQualificationV1', 'runtime_source_sha256': 'a' * 64,
        'controller_source_sha256': 'b' * 64, 'worker_sha256': 'c' * 64,
        'controllers': {key: item['sha256'] for key, item in value['controllers'].items()},
        'phases': {name: {'status': receipt('/input/' + name + '/exit.status'),
                          'result': receipt('/input/' + name + '/result.json')} for name in c.PHASES},
        'harness_sources': {name: 'a' * 64 for name in (*c.OWN_SOURCES,
            *('measurement/' + name for name in c.MEASUREMENT_SOURCES))}}


def clean_result():
    return {'status': 0, 'reason': 'completed', 'returncode': 0, 'cleanup_ok': True, 'child_reaped': True,
        'errors': [], 'term_sent': False, 'kill_sent': False, 'log_limit_exceeded': False,
        'profile': 'FerricCpuFourCore28GiBBuildV1', 'limits': dict(c.CPU_LIMITS), 'cpus': [0, 1, 2, 3],
        'nice': 19, 'build_jobs': 4, 'rust_test_threads': 1, 'peak_observed_rss_bytes': 1024,
        'argv': ['/usr/bin/python3', '-I', '-B', '-m', 'unittest'],
        'launch_environment': {'CARGO_BUILD_JOBS': '4', 'RUST_TEST_THREADS': '1',
            'FERRIC_CPU_PROFILE': 'FerricCpuFourCore28GiBBuildV1', 'CUDA_VISIBLE_DEVICES': '-1',
            'HIP_VISIBLE_DEVICES': '-1', 'HSA_VISIBLE_DEVICES': '-1', 'ROCR_VISIBLE_DEVICES': '-1'},
        **{key: {'memory_available_bytes': c.CPU_LIMITS['memory_available_bytes'],
            'root_free_bytes': c.CPU_LIMITS['root_free_bytes'], 'shm_free_bytes': c.CPU_LIMITS['shm_free_bytes'],
            'stage_bytes': 1024} for key in ('admission', 'final_resources')}}


class ContractTests(unittest.TestCase):
    def test_current_build_has_distinct_entries_and_one_exact_worker(self):
        c.validate_build(build())
        for mutate in (
            lambda value: value.update(runtime_main='0' * 40),
            lambda value: value['controllers'].update(B=value['controllers']['A']),
            lambda value: value['worker'].update(sha256='unknown'),
            lambda value: value.update(extra=True),
        ):
            value = build()
            mutate(value)
            with self.assertRaises(ValueError):
                c.validate_build(value)

    def test_cpu_rejects_failed_dirty_or_wrong_source_receipts(self):
        c.validate_cpu(cpu(), build(), lambda _: clean_result(), lambda *_: (b'0\n', 'a' * 64))
        for key, value in [('status', 1), ('status', False), ('returncode', -9), ('reason', 'timeout'),
            ('cleanup_ok', False), ('child_reaped', False), ('errors', ['failure']), ('term_sent', True),
            ('kill_sent', True), ('log_limit_exceeded', True), ('profile', 'other')]:
            row = clean_result()
            row[key] = value
            with self.assertRaises(ValueError):
                c.validate_cpu(cpu(), build(), lambda _: row, lambda *_: (b'0\n', 'a' * 64))
        for status in (b'1\n', b'0', b'0\nextra'):
            with self.assertRaises(ValueError):
                c.validate_cpu(cpu(), build(), lambda _: clean_result(), lambda *_: (status, 'a' * 64))
        value = cpu()
        value['worker_sha256'] = 'd' * 64
        with self.assertRaises(ValueError):
            c.validate_cpu(value, build(), lambda _: clean_result(), lambda *_: (b'0\n', 'a' * 64))

    def test_cpu_limits_and_visibility_cannot_be_weakened_by_same_profile_label(self):
        for mutate in (
            lambda row: row['limits'].update(stage_bytes=32 * 1024**3),
            lambda row: row.update(cpus=[0, 1, 2, 3, 4]),
            lambda row: row.update(nice=0),
            lambda row: row.update(build_jobs=8),
            lambda row: row.update(rust_test_threads=4),
            lambda row: row['launch_environment'].update(HIP_VISIBLE_DEVICES='0'),
            lambda row: row['final_resources'].update(root_free_bytes=1),
            lambda row: row['final_resources'].update(stage_bytes=c.CPU_LIMITS['stage_bytes']),
            lambda row: row.update(peak_observed_rss_bytes=16 * 1024**3),
        ):
            row = clean_result()
            mutate(row)
            with self.assertRaises(ValueError):
                c.validate_cpu(cpu(), build(), lambda _: row, lambda *_: (b'0\n', 'a' * 64))

    def test_stage_namespace_and_json_are_closed(self):
        self.assertEqual(c.stage_name('/dev/shm/ferric-v14-native-40695ec-a001').parent, Path('/dev/shm'))
        for path in ('/tmp/ferric-v14-native-40695ec-a001', '/dev/shm/foreign',
                     '/dev/shm/../shm/ferric-v14-native-40695ec-a001'):
            with self.assertRaises(ValueError):
                c.stage_name(path)
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}'):
            with self.assertRaises(ValueError):
                c.decode(raw)
        for path in ('../escape', '/absolute', 'nested/../escape'):
            with self.assertRaises(ValueError):
                c.relative(path)

    def test_remap_changes_only_stage_worker_hash_and_batch_budget(self):
        old = {'stage': '/tmp/frozen', 'common_args': ['--source', '/borrowed/model',
            '--worker', '/tmp/frozen/worker-candidate', '--worker-sha256', '0' * 64,
            '--max-batches', '256', '--target-artifact', '/tmp/frozen/images/target', '--disable-prefix-cache']}
        stage = Path('/dev/shm/ferric-v14-native-40695ec-a001')
        for count, maximum in ((1, '135'), (6, '810')):
            values = c.remap_common(old, stage, 'a' * 64, count)
            self.assertEqual(values, ['--source', '/borrowed/model', '--worker', str(stage / 'worker-candidate'),
                '--worker-sha256', 'a' * 64, '--max-batches', maximum,
                '--target-artifact', str(stage / 'images/target'), '--disable-prefix-cache'])

    def test_file_copy_is_create_only_and_hash_bound(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory)
            files = {}
            binding = prepare.create(stage, 'data/file', b'bytes', files)
            self.assertEqual(c.read(binding['path'], binding['sha256'])[0], b'bytes')
            self.assertEqual(files['data/file']['mode'], 0o600)
            with self.assertRaises(FileExistsError):
                prepare.create(stage, 'data/file', b'other', files)
            with self.assertRaises(ValueError):
                c.read(binding['path'], '0' * 64)
            (stage / 'alias').symlink_to(stage / 'data/file')
            with self.assertRaises(ValueError):
                c.read(stage / 'alias')

    def test_exact_cell_modes_share_worker_images_and_arguments(self):
        stage = Path('/dev/shm/ferric-v14-native-40695ec-a001')
        images = {option: {'path': str(stage / ('image-' + str(index))), 'manifest_sha256': '1' * 64,
                          'hsaco_sha256': '2' * 64} for index, option in enumerate(c.IMAGE_FIELDS)}
        observations = {key: {'compiler_handoff': {'sha256': '3' * 64}} for key in images}
        old = {'stage': '/tmp/frozen', 'common_args': ['--worker', '/tmp/frozen/worker-candidate',
                '--worker-sha256', '0' * 64, '--max-batches', '256']}
        common = c.remap_common(old, stage, 'c' * 64, 1)
        a = c.cell_spec(stage, 'A', 'correctness', common, images, observations, build(), {'prompt': 'hello'}, {})
        b = c.cell_spec(stage, 'B', 'correctness', common, images, observations, build(), {'prompt': 'hello'}, {})
        self.assertEqual(a['argv'][1:], b['argv'][1:])
        self.assertEqual(a['worker'], b['worker'])
        self.assertEqual(a['setup_expected'], b['setup_expected'])
        counter = c.cell_spec(stage, 'B', 'counters', common, images, observations, build(), {'prompt': 'hello'}, {})
        self.assertEqual(counter['argv'][1:3], ['--token-program-backend', 'native-whole-program-v1'])
        self.assertEqual(counter['argv'][3:], a['argv'][1:])
        self.assertEqual(counter['setup_expected']['kv_copy_artifact']['artifact_manifest_id'], '1' * 64)

    def test_campaign_predeclares_two_counters_and_three_abba_blocks_with_shared_inputs(self):
        stage = Path('/dev/shm/ferric-v14-native-40695ec-a001')
        images = {option: {'path': str(stage / ('image-' + str(index))), 'manifest_sha256': '1' * 64,
                          'hsaco_sha256': '2' * 64} for index, option in enumerate(c.IMAGE_FIELDS)}
        observations = {key: {'compiler_handoff': {'sha256': '3' * 64}} for key in images}
        old = {'stage': '/tmp/frozen', 'common_args': ['--worker', '/tmp/frozen/worker-candidate',
            '--worker-sha256', '0' * 64, '--max-batches', '256']}
        cells = c.campaign_cells(stage, old, images, observations, build(), {'prompt': 'hello'}, {})
        self.assertEqual(len(cells), 14)
        self.assertEqual([row['spec']['mode'] for row in cells], ['counters'] * 2 + ['latency'] * 12)
        self.assertEqual([row['spec']['arm'] for row in cells], ['A', 'B'] + ['A', 'B', 'B', 'A'] * 3)
        self.assertEqual(len({row['output'] for row in cells}), 14)
        self.assertEqual({row['spec']['worker']['path'] for row in cells}, {str(stage / 'worker-candidate')})
        self.assertEqual({tuple(row['spec']['argv'][1:]) for row in cells[2:]}, {tuple(cells[2]['spec']['argv'][1:])})
        self.assertEqual({row['common_args'][-1] for row in cells[:2]}, {'135'})
        self.assertEqual({row['common_args'][-1] for row in cells[2:]}, {'810'})
        plan = {'cells': cells}
        for index, row in enumerate(cells):
            self.assertEqual(c.selected_cell(plan, row['cell_id']), row)
            self.assertEqual(guard.preceding_cells(plan, row['cell_id']), cells[:index])
        for bad in ('counter-C', 'block4-A1', '../block1-A1'):
            with self.assertRaises(ValueError):
                c.selected_cell(plan, bad)
        changed = copy.deepcopy(plan)
        changed['cells'][2:4] = reversed(changed['cells'][2:4])
        with self.assertRaises(ValueError):
            guard.preceding_cells(changed, 'block1-A1')

    def test_output_roots_are_create_only_closed_and_reject_symlink_parents(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory)
            output = stage / 'cells/counter-A'
            with patch.object(guard.c, 'UID', os.getuid()):
                identity = guard.fresh_output(stage, output)
                self.assertEqual(guard.output_identity(stage, output), identity)
                with self.assertRaises(ValueError):
                    guard.fresh_output(stage, output)
                with self.assertRaises(ValueError):
                    guard.fresh_output(stage, stage / 'cells/not-predeclared')
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory)
            (stage / 'external').mkdir()
            (stage / 'cells').symlink_to(stage / 'external', target_is_directory=True)
            with patch.object(guard.c, 'UID', os.getuid()), self.assertRaises(ValueError):
                guard.fresh_output(stage, stage / 'cells/counter-A')
            self.assertFalse((stage / 'external/counter-A').exists())

    def test_shared_resource_cap_counts_input_root_and_all_nested_outputs(self):
        stage = Path('/dev/shm/ferric-v14-native-40695ec-a001')
        output = stage / 'cells/block3-A2'
        supervisor = SimpleNamespace(resource_snapshot=Mock(return_value={'stage_bytes': 1234}))
        with patch.object(guard, 'stage_identity', return_value={'root': 1}), \
             patch.object(guard, 'output_identity', return_value={'cell': 2}), \
             patch.object(guard, 'tmpfs_parent', return_value={'free_bytes': 32 * 1024**3}):
            result = guard.resources(stage, supervisor, {'root': 1}, output, {'cell': 2})
        supervisor.resource_snapshot.assert_called_once_with(stage)
        self.assertEqual(result['bounded']['stage_bytes'], 1234)


class AdmissionTests(unittest.TestCase):
    def monitor(self, stage, snapshots):
        monitor = object.__new__(guard.Admission)
        monitor.stage, monitor.owner, monitor.plan = stage, {}, {}
        monitor.output, monitor.output_owner = stage, {}
        monitor.profile = SimpleNamespace(gpu_idle=Mock())
        monitor.supervisor = SimpleNamespace(kfd_snapshot=Mock(side_effect=snapshots))
        monitor.directory = stage / 'admission'
        monitor.directory.mkdir()
        monitor.devices = ['/dev/kfd']
        monitor.owned = Mock(side_effect=lambda pids: [{'pid': pid, 'start_time_ticks': 99} for pid in pids])
        return monitor

    def test_mount_noexec_duplicate_and_wrong_filesystem_fail_closed(self):
        row = '5 1 0:4 / /dev/shm rw,nosuid,nodev - tmpfs tmpfs rw,size=1048576k\n'
        self.assertEqual(guard.mount_record(row)['filesystem'], 'tmpfs')
        for value in (row.replace('rw,nosuid', 'rw,noexec,nosuid'), row.replace('tmpfs tmpfs', 'ext4 /dev/x'), row + row, ''):
            with self.assertRaises(ValueError):
                guard.mount_record(value)

    def test_all_uid_fuser_retains_normal_access_markers(self):
        for stderr in (b'/dev/kfd:           m\n', b'/dev/kfd:   m m\n'):
            row = subprocess.CompletedProcess([], 0, b' 123 456\n', stderr)
            self.assertEqual(guard.fuser_members(row), [123, 456])
        self.assertEqual(guard.fuser_members(subprocess.CompletedProcess([], 1, b'', b'')), [])
        for code, stdout, stderr in ((0, b'', b''), (1, b'123', b''), (0, b'123 123', b''),
            (0, b'123 bad', b''), (0, b'123', b'Permission denied'), (2, b'', b''),
            (1, b'', b'/dev/kfd: error'), (0, b'123', b'/dev/kfd: warning'),
            (0, b'123', b'/dev/kfd: m m'), (1, b' ', b'')):
            with self.assertRaises(ValueError):
                guard.fuser_members(subprocess.CompletedProcess([], code, stdout, stderr))

    def test_incomplete_samples_retained_before_new_complete_retry(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory)
            monitor = object.__new__(guard.Admission)
            monitor.stage = stage
            monitor.output = stage
            monitor.output_owner = {}
            monitor.owner = {}
            monitor.plan = {}
            monitor.profile = SimpleNamespace(gpu_idle=lambda: None)
            monitor.supervisor = SimpleNamespace(kfd_snapshot=lambda _: {'foreign_pids': [], 'stage_worker_pids': []})
            monitor.directory = stage / 'admission'
            monitor.directory.mkdir()
            monitor.devices = ['/dev/kfd']
            monitor.owned = lambda _: []
            refused = {'accepted': False, 'complete': False, 'errors': ['vanished PID'], 'foreign_users': []}
            accepted = {'accepted': True, 'complete': True, 'errors': [], 'owned_users': [], 'foreign_users': []}
            responses = [subprocess.CompletedProcess([], 1, b'', b''),
                subprocess.CompletedProcess([], 1, c.encoded(refused), b''),
                subprocess.CompletedProcess([], 1, b'', b''),
                subprocess.CompletedProcess([], 0, c.encoded(accepted), b'')]
            with patch.object(guard.c, 'UID', os.getuid()), patch.object(guard, 'resources', return_value={}), \
                 patch.object(guard.subprocess, 'run', side_effect=responses), patch.object(guard.time, 'sleep'):
                result = monitor.check('preflight')
            self.assertTrue(result['accepted'])
            self.assertEqual([row['accepted'] for row in result['samples']], [False, True])
            self.assertEqual(len(list(monitor.directory.glob('*-scan-*.json'))), 2)

    def test_new_exact_worker_in_descriptor_scan_gets_fresh_full_sample(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory)
            empty = {'foreign_pids': [], 'stage_worker_pids': []}
            owned = {'foreign_pids': [], 'stage_worker_pids': [123]}
            monitor = self.monitor(stage, [empty, owned])
            identity = {'pid': 123, 'start_time_ticks': 99}
            user = {'identity': identity, 'descriptors': [{'fd': 5, 'major': 1, 'minor': 2}]}
            refused = {'accepted': False, 'complete': True, 'errors': ['unbound GPU owner'],
                       'foreign_users': [user], 'owned_users': [], 'device_users': [user]}
            accepted = {'accepted': True, 'complete': True, 'errors': [], 'foreign_users': [],
                        'owned_users': [user], 'device_users': [user]}
            responses = [subprocess.CompletedProcess([], 1, b'', b''),
                subprocess.CompletedProcess([], 1, c.encoded(refused), b''),
                subprocess.CompletedProcess([], 0, b'123', b'/dev/kfd: m'),
                subprocess.CompletedProcess([], 0, c.encoded(accepted), b'')]
            with patch.object(guard.c, 'UID', os.getuid()), patch.object(guard, 'resources', return_value={}), \
                 patch.object(guard.subprocess, 'run', side_effect=responses), patch.object(guard.time, 'sleep'):
                result = monitor.check('monitor')
            self.assertTrue(result['accepted'])
            self.assertEqual(monitor.supervisor.kfd_snapshot.call_count, 2)
            self.assertEqual([row['accepted'] for row in result['samples']], [False, True])
            self.assertEqual([c.decode(c.read(row['owners']['path'])[0]) for row in result['attempts']],
                             [[], [identity]])
            self.assertIn('exact owned worker appeared', result['attempts'][0]['retry_reason'])

    def test_fuser_new_worker_requires_owned_lifetime_before_scan(self):
        with tempfile.TemporaryDirectory() as directory:
            monitor = self.monitor(Path(directory), [{'foreign_pids': [], 'stage_worker_pids': []}])
            accepted = {'accepted': True, 'complete': True, 'errors': [], 'foreign_users': []}
            responses = [subprocess.CompletedProcess([], 0, b'123', b'/dev/kfd: m'),
                         subprocess.CompletedProcess([], 0, c.encoded(accepted), b'')]
            with patch.object(guard.c, 'UID', os.getuid()), patch.object(guard, 'resources', return_value={}), \
                 patch.object(guard.subprocess, 'run', side_effect=responses):
                result = monitor.check('monitor')
            self.assertTrue(result['accepted'])
            monitor.owned.assert_called_once_with([123])

    def test_foreign_descriptor_lifetime_is_terminal_and_receipt_is_retained(self):
        with tempfile.TemporaryDirectory() as directory:
            monitor = self.monitor(Path(directory), [{'foreign_pids': [], 'stage_worker_pids': []}])
            monitor.owned = Mock(side_effect=[[], ValueError('wrong executable inode')])
            refused = {'accepted': False, 'complete': True, 'errors': ['unbound owner'],
                       'foreign_users': [{'identity': {'pid': 777, 'start_time_ticks': 99}}]}
            responses = [subprocess.CompletedProcess([], 1, b'', b''),
                         subprocess.CompletedProcess([], 1, c.encoded(refused), b'')]
            with patch.object(guard.c, 'UID', os.getuid()), patch.object(guard, 'resources', return_value={}), \
                 patch.object(guard.subprocess, 'run', side_effect=responses) as run:
                with self.assertRaisesRegex(ValueError, 'wrong executable inode'):
                    monitor.check('monitor')
            self.assertEqual(run.call_count, 2)
            self.assertEqual(monitor.supervisor.kfd_snapshot.call_count, 1)
            retained = [c.decode(c.read(path)[0]) for path in monitor.directory.glob('*-monitor.json')]
            self.assertEqual(len(retained), 1)
            self.assertFalse(retained[0]['accepted'])
            self.assertEqual(len(retained[0]['samples']), 1)

    def test_retired_worker_identity_retries_with_empty_new_kfd_sample(self):
        with tempfile.TemporaryDirectory() as directory:
            monitor = self.monitor(Path(directory), [
                {'foreign_pids': [], 'stage_worker_pids': [123]},
                {'foreign_pids': [], 'stage_worker_pids': []}])
            monitor.owned = Mock(side_effect=[FileNotFoundError('retired worker'), []])
            accepted = {'accepted': True, 'complete': True, 'errors': [], 'foreign_users': []}
            responses = [subprocess.CompletedProcess([], 0, b'123', b'/dev/kfd: m'),
                subprocess.CompletedProcess([], 1, b'', b''),
                subprocess.CompletedProcess([], 0, c.encoded(accepted), b'')]
            with patch.object(guard.c, 'UID', os.getuid()), patch.object(guard, 'resources', return_value={}), \
                 patch.object(guard.subprocess, 'run', side_effect=responses), patch.object(guard.time, 'sleep'):
                result = monitor.check('monitor')
            self.assertTrue(result['accepted'])
            self.assertEqual(len(result['attempts']), 2)
            self.assertIn('retired worker', result['attempts'][0]['retry_reason'])
            self.assertEqual(result['attempts'][1]['kfd']['stage_worker_pids'], [])

    def test_postflight_rechecks_utilization_and_refuses_remaining_worker(self):
        with tempfile.TemporaryDirectory() as directory:
            monitor = self.monitor(Path(directory), [{'foreign_pids': [], 'stage_worker_pids': [123]}])
            with patch.object(guard.c, 'UID', os.getuid()), patch.object(guard, 'resources', return_value={}), \
                 patch.object(guard.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, b'123', b'')):
                with self.assertRaisesRegex(ValueError, 'lifecycle endpoint'):
                    monitor.check('postflight')
            monitor.profile.gpu_idle.assert_called_once_with()
            monitor.owned.assert_not_called()


if __name__ == '__main__':
    unittest.main()
