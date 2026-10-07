"""CPU-only fixtures; no SSH, device, sudo, controller or compiler launches."""
import copy
import hashlib
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
gate_fixture = load('test_prefill_width').gate_up_fixture


def receipt(path, digit='a'):
    return {'path': path, 'sha256': digit * 64}


def scan_sample(value):
    return {'schema': 'FerricDeviceDescriptorSampleV2', 'method': 'proc-fd-rdev-finite-roster-v1',
        'sampling_policy': 'initial-plus-one-birth-frontier-v1', 'root_visibility': True, **value}


EVIDENCE = None


def build():
    return copy.deepcopy(EVIDENCE.build)


def cpu():
    return copy.deepcopy(EVIDENCE.cpu)


def clean_result(role=None):
    role = role or 'model-library'
    return EVIDENCE.bound(EVIDENCE.cpu['checkpoint']['phases'][role]['result'])


def result_for_binding(binding):
    return EVIDENCE.bound(binding)


def raw_receipt(path, *_args):
    return EVIDENCE.read(path, empty=True)


class ContractTests(unittest.TestCase):
    def setUp(self):
        global EVIDENCE
        fixture = load('test_cpu_binding')
        EVIDENCE = fixture.Evidence(c).install(self)

    def test_current_build_has_distinct_entries_and_one_exact_worker(self):
        c.validate_build(build())
        for mutate in (
            lambda value: value.update(runtime_main='0' * 40),
            lambda value: value['controllers'].update({'counters-A': value['controllers']['live-A']}),
            lambda value: value['worker'].update(sha256='unknown'),
            lambda value: value.update(extra=True),
        ):
            value = build()
            mutate(value)
            with self.assertRaises(ValueError):
                c.validate_build(value)

    def test_cpu_rejects_failed_dirty_or_wrong_source_receipts(self):
        EVIDENCE.validate()
        validator = EVIDENCE.current
        expected = EVIDENCE.expected['phases']['model-library']
        for key, value in [('status', 1), ('status', False), ('returncode', -9), ('reason', 'timeout'),
            ('cleanup_ok', False), ('child_reaped', False), ('errors', ['failure']), ('term_sent', True),
            ('kill_sent', True), ('log_limit_exceeded', True), ('profile', 'other')]:
            row = clean_result()
            row[key] = value
            with self.assertRaises(ValueError):
                validator.validate_outer(row, expected)
        value = cpu()
        value['worker_sha256'] = 'd' * 64
        with self.assertRaises(ValueError):
            c.validate_cpu(value, build(), result_for_binding, raw_receipt)

    def test_cpu_limits_and_visibility_cannot_be_weakened_by_same_profile_label(self):
        current = EVIDENCE.current
        for mutate in (
            lambda row: row['limits'].update(stage_bytes=43 * 1024**3),
            lambda row: row.update(cpus=[0, 1, 2, 3, 4]),
            lambda row: row.update(nice=0),
            lambda row: row.update(build_jobs=8),
            lambda row: row.update(rust_test_threads=4),
            lambda row: row['launch_environment'].update(HIP_VISIBLE_DEVICES='0'),
            lambda row: row['final_resources'].update(root_free_bytes=1),
            lambda row: row['final_resources'].update(stage_bytes=current.LIMITS['stage_bytes']),
            lambda row: row.update(peak_observed_rss_bytes=16 * 1024**3),
        ):
            row = clean_result()
            mutate(row)
            with self.assertRaises(ValueError):
                current.validate_outer(row,EVIDENCE.expected['phases']['model-library'])

    def test_new_roles_reject_historical_profiles_and_cross_profile_labels(self):
        for profile, cap in (('FerricCpuFourCore36GiBEmitterV1',36*1024**3),
                             ('FerricCpuFourCore28GiBBuildV1',28*1024**3)):
            row = clean_result()
            row['profile'] = profile
            row['launch_environment']['FERRIC_CPU_PROFILE'] = profile
            row['limits']['stage_bytes'] = cap
            with self.assertRaises(ValueError):
                EVIDENCE.current.validate_outer(row,EVIDENCE.expected['phases']['model-library'])

    def test_prefill_binding_rejects_v14_rosters_and_unqualified_worker(self):
        for mutate in (
            lambda value: value.update(schema='FerricV14NativeBuildBindingV1'),
            lambda value: value.update(runtime_main='cea62c79d1622ca5fa53a727611a102e067c4ea2'),
            lambda value: value['worker'].update(sha256='c' * 64),
            lambda value: value['controllers'].update(A=value['controllers']['live-A']),
        ):
            value = build()
            mutate(value)
            with self.assertRaises(ValueError):
                c.validate_build(value)

    def test_aggregate_receipt_cannot_replace_closed_individual_roles(self):
        value = cpu()
        value['checkpoint']['phases']['actual-abi-candidate'] = copy.deepcopy(value['checkpoint']['phases']['actual-abi-control'])
        with self.assertRaises(ValueError):
            c.validate_cpu(value,build(),result_for_binding,raw_receipt)

    def test_fresh_harness_requires_all_roles_and_exact_source_manifest(self):
        for name in EVIDENCE.contract.ROLES:
            value = cpu()
            del value['harness']['roles'][name]
            with self.assertRaises(ValueError):
                c.validate_cpu(value,build(),result_for_binding,raw_receipt)
        value = cpu()
        value['harness_sources']['launch_contract.py'] = 'b'*64
        with self.assertRaises(ValueError):
            c.validate_cpu(value,build(),result_for_binding,raw_receipt)

    def test_harness_raw_counts_and_actual_binding_are_required(self):
        value = cpu()
        phase = value['harness']['roles']['contract']
        phase['role_stderr'] = EVIDENCE.put('/input/zero-tests',b'Ran 0 tests in 0.001s\n\nOK\n')
        with self.assertRaises(ValueError):
            c.validate_cpu(value,build(),result_for_binding,raw_receipt)
        value = cpu()
        original = EVIDENCE.bound(value['harness']['original_actual_binding'])
        original['custody'].pop(next(iter(original['custody'])))
        value['harness']['original_actual_binding'] = EVIDENCE.put('/input/incomplete-actual',c.encoded(original))
        with self.assertRaises(ValueError):
            c.validate_cpu(value,build(),result_for_binding,raw_receipt)

    def test_empty_streams_are_verified_by_the_staged_file_roster(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory)
            path = stage/'stderr'
            path.write_bytes(b'')
            path.chmod(0o600)
            files = {'stderr':{'bytes':0,'mode':0o600,'sha256':hashlib.sha256(b'').hexdigest()}}
            c.verify_files(stage,files)
            path.write_bytes(b'changed')
            with self.assertRaises(ValueError):
                c.verify_files(stage,files)

    def test_image_gate_requires_exact_ignored_test_and_prefill_binary(self):
        current = EVIDENCE.current
        for name in ('actual-abi-control','actual-abi-candidate','actual-image'):
            phase = EVIDENCE.cpu['checkpoint']['phases'][name]
            streams = {key:EVIDENCE.read(item['path'],empty=True)[0] for key,item in phase.items()}
            for mutate in (lambda argv:argv.__setitem__(1,'different_filter'),
                lambda argv:argv.__setitem__(0,'/bin/true'),lambda argv:argv.remove('--ignored'),
                lambda argv:argv.remove('--nocapture'),lambda argv:argv.append('--list')):
                inner = current.decode(streams['inner'])
                mutate(inner['argv'])
                with self.assertRaises(ValueError):
                    current.validate_inner(name,inner,EVIDENCE.expected['phases'][name],streams)

    def test_driver_gate_requires_executed_closed_default_filter(self):
        name = 'model-library'
        phase = EVIDENCE.cpu['checkpoint']['phases'][name]
        streams = {key:EVIDENCE.read(item['path'],empty=True)[0] for key,item in phase.items()}
        for mutate in (lambda argv:argv.append('--list'),lambda argv:argv.append('--ignored'),
            lambda argv:argv.__setitem__(argv.index('splitk'),'different_filter'),
            lambda argv:argv.__setitem__(argv.index('--features')+1,'other_feature')):
            inner = EVIDENCE.current.decode(streams['inner'])
            mutate(inner['argv'])
            with self.assertRaises(ValueError):
                EVIDENCE.current.validate_inner(name,inner,EVIDENCE.expected['phases'][name],streams)

    def test_image_and_driver_receipts_bind_actual_nonzero_pass_count(self):
        for name in EVIDENCE.current.TEST_COUNTS:
            phase = EVIDENCE.cpu['checkpoint']['phases'][name]
            streams = {key:EVIDENCE.read(item['path'],empty=True)[0] for key,item in phase.items()}
            inner = EVIDENCE.current.decode(streams['inner'])
            for output in (b'',b'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\\n',
                           streams['role_stdout']*2):
                with self.assertRaisesRegex(ValueError,'executed test counts'):
                    EVIDENCE.current.validate_inner(name,inner,EVIDENCE.expected['phases'][name],
                        {**streams,'role_stdout':output})

    def test_stage_namespace_and_json_are_closed(self):
        self.assertEqual(c.stage_name('/dev/shm/ferric-native-gate-up-slots512-a001').parent, Path('/dev/shm'))
        for path in ('/tmp/ferric-native-gate-up-slots512-a001', '/dev/shm/foreign',
                     '/dev/shm/../shm/ferric-native-gate-up-slots512-a001'):
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
        stage = Path('/dev/shm/ferric-native-gate-up-slots512-a001')
        for arm, count, maximum in (('A', 1, '131'), ('A', 6, '786'), ('B', 1, '131'), ('B', 6, '786')):
            values = c.remap_common(old, stage, 'a' * 64, count, arm)
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

    def test_cpu_receipts_are_copied_with_stdout_into_closed_stage(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            inputs, stage = root / 'inputs', root / 'stage'
            inputs.mkdir(mode=0o700)
            stage.mkdir(mode=0o700)
            value, files, original = cpu(), {}, {}
            for name,item in c.iter_cpu_bindings(value):
                raw = raw_receipt(item['path'])[0]
                source = inputs / name
                source.parent.mkdir(parents=True,exist_ok=True)
                source.write_bytes(raw)
                item.update(path=str(source),sha256=hashlib.sha256(raw).hexdigest())
                original[name] = raw
            prepare.stage_cpu_receipts(stage,value,files)
            c.validate_cpu_stage(stage,files,value)
            for name,item in c.iter_cpu_bindings(value):
                target = stage / 'evidence' / name
                self.assertEqual(item['path'],str(target))
                self.assertEqual(target.read_bytes(),original[name])
                self.assertEqual(files[str(target.relative_to(stage))]['sha256'],item['sha256'])
            saved = copy.deepcopy(value)
            key = 'checkpoint/actual-image/role_stdout'
            value['checkpoint']['phases']['actual-image']['role_stdout']['path'] = str(inputs/key)
            with self.assertRaisesRegex(ValueError,'outside immutable stage'):
                c.validate_cpu_stage(stage,files,value)
            files.pop('evidence/'+key)
            with self.assertRaisesRegex(ValueError,'closed staged file roster'):
                c.validate_cpu_stage(stage,files,saved)

    def test_exact_cell_modes_share_worker_images_and_arguments(self):
        stage = Path('/dev/shm/ferric-native-gate-up-slots512-a001')
        images = {option: {'path': str(stage / ('image-' + str(index))), 'manifest_sha256': '1' * 64,
                          'hsaco_sha256': '2' * 64} for index, option in enumerate(c.IMAGE_FIELDS)}
        observations = {key: {'compiler_handoff': {'sha256': '3' * 64}} for key in images}
        old = {'stage': '/tmp/frozen', 'common_args': ['--worker', '/tmp/frozen/worker-candidate',
                '--worker-sha256', '0' * 64, '--max-batches', '256']}
        common = c.remap_common(old, stage, 'c' * 64, 1, 'A')
        common_b = c.remap_common(old, stage, 'c' * 64, 1, 'B')
        a = c.cell_spec(stage, 'A', 'correctness', common, images, observations, build(), {'prompt': 'hello'}, {}, gate_fixture())
        b = c.cell_spec(stage, 'B', 'correctness', common_b, images, observations, build(), {'prompt': 'hello'}, {}, gate_fixture())
        self.assertNotEqual(a['controller'], b['controller'])
        self.assertEqual(a['argv'][1:3], ['--native-prefill-rows', '32'])
        self.assertEqual(b['argv'][1:3], ['--native-prefill-rows', '32'])
        self.assertEqual(a['controller']['sha256'], b['controller']['sha256'])
        self.assertNotIn('--token-program-fence-mode', a['argv'] + b['argv'])
        self.assertEqual(a['worker'], b['worker'])
        self.assertEqual(a['setup_expected'], b['setup_expected'])
        counter = c.cell_spec(stage, 'B', 'counters', common_b, images, observations, build(), {'prompt': 'hello'}, {}, gate_fixture())
        self.assertEqual(counter['argv'][1:], b['argv'][1:])
        control = c.cell_spec(stage, 'A', 'counters', common, images, observations, build(), {'prompt': 'hello'}, {}, gate_fixture())
        self.assertEqual(control['argv'][1:3], ['--native-prefill-rows', '32'])
        self.assertEqual(control['argv'][3:], a['argv'][3:])
        self.assertEqual(counter['setup_expected']['kv_copy_artifact']['artifact_manifest_id'], '1' * 64)

    def test_campaign_predeclares_two_counters_and_three_abba_blocks_with_shared_inputs(self):
        stage = Path('/dev/shm/ferric-native-gate-up-slots512-a001')
        images = {option: {'path': str(stage / ('image-' + str(index))), 'manifest_sha256': '1' * 64,
                          'hsaco_sha256': '2' * 64} for index, option in enumerate(c.IMAGE_FIELDS)}
        observations = {key: {'compiler_handoff': {'sha256': '3' * 64}} for key in images}
        old = {'stage': '/tmp/frozen', 'common_args': ['--worker', '/tmp/frozen/worker-candidate',
            '--worker-sha256', '0' * 64, '--max-batches', '256']}
        cells = c.campaign_cells(stage, old, images, observations, build(), {'prompt': 'hello'}, {}, gate_fixture())
        self.assertEqual(len(cells), 14)
        self.assertEqual([row['spec']['mode'] for row in cells], ['counters'] * 2 + ['latency'] * 12)
        self.assertEqual([row['spec']['arm'] for row in cells], ['A', 'B'] + ['A', 'B', 'B', 'A'] * 3)
        self.assertEqual(len({row['output'] for row in cells}), 14)
        self.assertEqual({row['spec']['worker']['path'] for row in cells}, {str(stage / 'worker-candidate')})
        for arm in ('A', 'B'):
            self.assertEqual(len({tuple(row['spec']['argv'][3:]) for row in cells[2:] if row['spec']['arm'] == arm}), 1)
        self.assertEqual(len({row['spec']['controller']['sha256'] for row in cells[2:]}), 1)
        self.assertEqual(len({row['spec']['controller']['path'] for row in cells[2:]}), 2)
        self.assertEqual({row['common_args'][-1] for row in cells[:2]}, {'131'})
        self.assertEqual({row['common_args'][-1] for row in cells[2:]}, {'786'})
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
        stage = Path('/dev/shm/ferric-native-gate-up-slots512-a001')
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
                subprocess.CompletedProcess([], 1, c.encoded(scan_sample(refused)), b''),
                subprocess.CompletedProcess([], 1, b'', b''),
                subprocess.CompletedProcess([], 0, c.encoded(scan_sample(accepted)), b''),
                subprocess.CompletedProcess([], 1, b'', b'')]
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
            monitor = self.monitor(stage, [empty, owned, owned])
            identity = {'pid': 123, 'start_time_ticks': 99}
            user = {'identity': identity, 'descriptors': [{'fd': 5, 'major': 1, 'minor': 2}]}
            refused = {'accepted': False, 'complete': True, 'errors': ['unbound GPU owner'],
                       'foreign_users': [user], 'owned_users': [], 'device_users': [user]}
            accepted = {'accepted': True, 'complete': True, 'errors': [], 'foreign_users': [],
                        'owned_users': [user], 'device_users': [user]}
            responses = [subprocess.CompletedProcess([], 1, b'', b''),
                subprocess.CompletedProcess([], 1, c.encoded(scan_sample(refused)), b''),
                subprocess.CompletedProcess([], 0, b'123', b'/dev/kfd: m'),
                subprocess.CompletedProcess([], 0, c.encoded(scan_sample(accepted)), b''),
                subprocess.CompletedProcess([], 0, b'123', b'/dev/kfd: m')]
            with patch.object(guard.c, 'UID', os.getuid()), patch.object(guard, 'resources', return_value={}), \
                 patch.object(guard.subprocess, 'run', side_effect=responses), patch.object(guard.time, 'sleep'):
                result = monitor.check('monitor')
            self.assertTrue(result['accepted'])
            self.assertEqual(monitor.supervisor.kfd_snapshot.call_count, 3)
            self.assertEqual([row['accepted'] for row in result['samples']], [False, True])
            self.assertEqual([c.decode(c.read(row['owners']['path'])[0]) for row in result['attempts']],
                             [[], [identity]])
            self.assertIn('exact owned worker appeared', result['attempts'][0]['retry_reason'])

    def test_fuser_new_worker_requires_owned_lifetime_before_scan(self):
        with tempfile.TemporaryDirectory() as directory:
            monitor = self.monitor(Path(directory), [{'foreign_pids': [], 'stage_worker_pids': []}] * 2)
            accepted = {'accepted': True, 'complete': True, 'errors': [], 'foreign_users': []}
            responses = [subprocess.CompletedProcess([], 0, b'123', b'/dev/kfd: m'),
                         subprocess.CompletedProcess([], 0, c.encoded(scan_sample(accepted)), b''),
                         subprocess.CompletedProcess([], 0, b'123', b'/dev/kfd: m')]
            with patch.object(guard.c, 'UID', os.getuid()), patch.object(guard, 'resources', return_value={}), \
                 patch.object(guard.subprocess, 'run', side_effect=responses):
                result = monitor.check('monitor')
            self.assertTrue(result['accepted'])
            self.assertEqual([call.args for call in monitor.owned.call_args_list], [([123],), ([123],)])

    def test_foreign_descriptor_lifetime_is_terminal_and_receipt_is_retained(self):
        with tempfile.TemporaryDirectory() as directory:
            monitor = self.monitor(Path(directory), [{'foreign_pids': [], 'stage_worker_pids': []}])
            monitor.owned = Mock(side_effect=[[], ValueError('wrong executable inode')])
            refused = {'accepted': False, 'complete': True, 'errors': ['unbound owner'],
                       'foreign_users': [{'identity': {'pid': 777, 'start_time_ticks': 99}}]}
            responses = [subprocess.CompletedProcess([], 1, b'', b''),
                         subprocess.CompletedProcess([], 1, c.encoded(scan_sample(refused)), b'')]
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
                {'foreign_pids': [], 'stage_worker_pids': []},
                {'foreign_pids': [], 'stage_worker_pids': []}])
            monitor.owned = Mock(side_effect=[FileNotFoundError('retired worker'), [], []])
            accepted = {'accepted': True, 'complete': True, 'errors': [], 'foreign_users': []}
            responses = [subprocess.CompletedProcess([], 0, b'123', b'/dev/kfd: m'),
                subprocess.CompletedProcess([], 1, b'', b''),
                subprocess.CompletedProcess([], 0, c.encoded(scan_sample(accepted)), b''),
                subprocess.CompletedProcess([], 1, b'', b'')]
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

    def test_postscan_exact_new_worker_requires_a_fresh_complete_sample(self):
        with tempfile.TemporaryDirectory() as directory:
            empty = {'foreign_pids': [], 'stage_worker_pids': []}
            owned = {'foreign_pids': [], 'stage_worker_pids': [123]}
            monitor = self.monitor(Path(directory), [empty, owned, owned, owned])
            sample = scan_sample({'accepted': True, 'complete': True, 'errors': [], 'foreign_users': []})
            no_users = subprocess.CompletedProcess([], 1, b'', b'')
            worker = subprocess.CompletedProcess([], 0, b'123', b'/dev/kfd: m')
            scan = subprocess.CompletedProcess([], 0, c.encoded(sample), b'')
            with patch.object(guard.c, 'UID', os.getuid()), patch.object(guard, 'resources', return_value={}), \
                 patch.object(guard.subprocess, 'run', side_effect=[no_users, scan, worker, worker, scan, worker]), \
                 patch.object(guard.time, 'sleep'):
                result = monitor.check('monitor')
            self.assertTrue(result['accepted'])
            self.assertEqual([row['accepted'] for row in result['attempts']], [False, True])
            self.assertEqual(len(result['samples']), 2)
            self.assertIn('fresh scan required', result['attempts'][0]['retry_reason'])
            self.assertEqual(monitor.supervisor.kfd_snapshot.call_count, 4)
            self.assertEqual(result['attempts'][1]['gpu_endpoint_after']['owned_identities'],
                             [{'pid': 123, 'start_time_ticks': 99}])

    def test_postscan_foreign_fuser_process_refuses_even_after_clean_scan(self):
        with tempfile.TemporaryDirectory() as directory:
            empty = {'foreign_pids': [], 'stage_worker_pids': []}
            monitor = self.monitor(Path(directory), [empty, empty])
            monitor.owned = Mock(side_effect=[[], ValueError('wrong executable inode')])
            sample = scan_sample({'accepted': True, 'complete': True, 'errors': [], 'foreign_users': []})
            responses = [subprocess.CompletedProcess([], 1, b'', b''),
                subprocess.CompletedProcess([], 0, c.encoded(sample), b''),
                subprocess.CompletedProcess([], 0, b'777', b'/dev/kfd: m')]
            with patch.object(guard.c, 'UID', os.getuid()), patch.object(guard, 'resources', return_value={}), \
                 patch.object(guard.subprocess, 'run', side_effect=responses):
                with self.assertRaisesRegex(ValueError, 'wrong executable inode'):
                    monitor.check('monitor')
            records = [c.decode(c.read(path)[0]) for path in monitor.directory.glob('*-monitor.json')]
            self.assertFalse(records[0]['accepted'])
            self.assertEqual(records[0]['attempts'][0]['gpu_endpoint_after']['fuser']['pids'], [777])

    def test_postscan_sysfs_foreign_process_is_terminal(self):
        with tempfile.TemporaryDirectory() as directory:
            monitor = self.monitor(Path(directory), [
                {'foreign_pids': [], 'stage_worker_pids': []},
                {'foreign_pids': [777], 'stage_worker_pids': []}])
            sample = scan_sample({'accepted': True, 'complete': True, 'errors': [], 'foreign_users': []})
            responses = [subprocess.CompletedProcess([], 1, b'', b''),
                subprocess.CompletedProcess([], 0, c.encoded(sample), b'')]
            with patch.object(guard.c, 'UID', os.getuid()), patch.object(guard, 'resources', return_value={}), \
                 patch.object(guard.subprocess, 'run', side_effect=responses) as run:
                with self.assertRaisesRegex(ValueError, 'foreign KFD'):
                    monitor.check('monitor')
            self.assertEqual(run.call_count, 2)

    def test_postscan_owned_pid_lifetime_drift_is_terminal(self):
        with tempfile.TemporaryDirectory() as directory:
            monitor = self.monitor(Path(directory), [{'foreign_pids': [], 'stage_worker_pids': [123]}])
            monitor.owned = Mock(return_value=[{'pid': 123, 'start_time_ticks': 100}])
            with patch.object(guard.subprocess, 'run', return_value=
                              subprocess.CompletedProcess([], 0, b'123', b'/dev/kfd: m')):
                with self.assertRaisesRegex(ValueError, 'lifetime changed'):
                    monitor.confirm_scan_endpoint('monitor', None, False,
                        [{'pid': 123, 'start_time_ticks': 99}], {})

    def test_postscan_replacement_worker_is_not_an_appearance_retry(self):
        with tempfile.TemporaryDirectory() as directory:
            monitor = self.monitor(Path(directory), [{'foreign_pids': [], 'stage_worker_pids': [124]}])
            with patch.object(guard.subprocess, 'run', return_value=
                              subprocess.CompletedProcess([], 0, b'124', b'/dev/kfd: m')):
                with self.assertRaisesRegex(ValueError, 'unexpected owned GPU roster'):
                    monitor.confirm_scan_endpoint('monitor', None, False,
                        [{'pid': 123, 'start_time_ticks': 99}], {})

    def test_postscan_retirement_only_retries_in_monitor_without_setup(self):
        for phase, setup, initial in [('monitor', None, False), ('active', {'worker_pids': [123]}, False),
                                     ('monitor', None, True)]:
            with self.subTest(phase=phase, setup=setup, initial=initial), tempfile.TemporaryDirectory() as directory:
                monitor = self.monitor(Path(directory), [{'foreign_pids': [], 'stage_worker_pids': []}])
                with patch.object(guard.subprocess, 'run', return_value=subprocess.CompletedProcess([], 1, b'', b'')):
                    if phase == 'monitor' and not initial:
                        observation = {}
                        self.assertFalse(monitor.confirm_scan_endpoint(phase, setup, initial,
                            [{'pid': 123, 'start_time_ticks': 99}], observation))
                        self.assertIn('fresh scan required', observation['retry_reason'])
                    else:
                        with self.assertRaises(ValueError):
                            monitor.confirm_scan_endpoint(phase, setup, initial,
                                [{'pid': 123, 'start_time_ticks': 99}], {})

    def test_postscan_gpu_birth_refuses_at_lifecycle_endpoints(self):
        for phase, initial in [('preflight', False), ('postflight', False), ('monitor', True)]:
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as directory:
                monitor = self.monitor(Path(directory), [{'foreign_pids': [], 'stage_worker_pids': [123]}])
                with patch.object(guard.subprocess, 'run', return_value=
                                  subprocess.CompletedProcess([], 0, b'123', b'/dev/kfd: m')):
                    with self.assertRaisesRegex(ValueError, 'final lifecycle endpoint'):
                        monitor.confirm_scan_endpoint(phase, None, initial, [], {})
                monitor.owned.assert_not_called()

    def test_postscan_fuser_diagnostics_cannot_be_ignored(self):
        with tempfile.TemporaryDirectory() as directory:
            monitor = self.monitor(Path(directory), [{'foreign_pids': [], 'stage_worker_pids': []}])
            with patch.object(guard.subprocess, 'run', return_value=
                              subprocess.CompletedProcess([], 1, b'', b'Permission denied')):
                with self.assertRaisesRegex(ValueError, 'diagnostics'):
                    monitor.confirm_scan_endpoint('monitor', None, False, [], {})
            monitor.owned.assert_not_called()

    def test_sample_policy_and_privileged_visibility_are_required(self):
        for key, value in [('schema', 'FerricDeviceDescriptorSampleV1'), ('method', 'proc-fd-rdev-all-pids'),
                           ('sampling_policy', 'unknown'), ('root_visibility', False), ('root_visibility', 1)]:
            with self.subTest(key=key, value=value), tempfile.TemporaryDirectory() as directory:
                monitor = self.monitor(Path(directory), [{'foreign_pids': [], 'stage_worker_pids': []}])
                sample = scan_sample({'accepted': True, 'complete': True, 'errors': [], 'foreign_users': []})
                sample[key] = value
                responses = [subprocess.CompletedProcess([], 1, b'', b''),
                    subprocess.CompletedProcess([], 0, c.encoded(sample), b'')]
                with patch.object(guard.c, 'UID', os.getuid()), patch.object(guard, 'resources', return_value={}), \
                     patch.object(guard.subprocess, 'run', side_effect=responses):
                    with self.assertRaisesRegex(ValueError, 'sampling policy'):
                        monitor.check('preflight')

    def test_complete_sample_with_errors_is_not_accepted(self):
        with tempfile.TemporaryDirectory() as directory:
            monitor = self.monitor(Path(directory), [{'foreign_pids': [], 'stage_worker_pids': []}])
            sample = scan_sample({'accepted': True, 'complete': True, 'errors': ['permission'], 'foreign_users': []})
            responses = [subprocess.CompletedProcess([], 1, b'', b''),
                subprocess.CompletedProcess([], 0, c.encoded(sample), b'')]
            with patch.object(guard.c, 'UID', os.getuid()), patch.object(guard, 'resources', return_value={}), \
                 patch.object(guard.subprocess, 'run', side_effect=responses):
                with self.assertRaisesRegex(ValueError, 'acceptance/status mismatch'):
                    monitor.check('preflight')


if __name__ == '__main__':
    unittest.main()
