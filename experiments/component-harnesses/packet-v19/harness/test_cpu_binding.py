"""Synthetic current55c custody fixtures; never actual qualification evidence."""
import copy
import hashlib
import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest import mock


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


HERE = Path(__file__).parent
c = module('baseline_contract', HERE / 'launch_contract.py')
p = module('baseline_policy', HERE / 'controller_binding.py')
q = module('baseline_qualifier', HERE / 'controller_qualifier.py')
v = module('v19_qualifier', HERE / 'v19_qualifier.py')
r = module('baseline_runtime', HERE / 'runtime_binding.py')
stager = module('baseline_stager', HERE / 'prepare_stage.py')
binder = module('baseline_binder', HERE.parent / 'bind_build.py')


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


class Fixture:
    def __init__(self):
        self.raw, self.results = {}, {}
        self.before = c.decode((HERE / 'fixtures/source-files.json').read_bytes())
        self.modes = c.decode((HERE / 'fixtures/source-modes.json').read_bytes())
        self.after = dict(self.before)
        self.after.update({name: pair[1] for name, pair in q.OVERLAYS.items()})
        self.build = {'schema': 'FerricV19Packet55cBuildBindingV1',
            'runtime_main': c.RUNTIME_MAIN,
            'runtime_source': self.file('runtime', b'runtime archive', c.RUNTIME_SOURCE_SHA),
            'controller_source': self.file('source', b'candidate archive'),
            'worker': self.file('worker', b'\x7fELF' + bytes(r.WORKER_BYTES - 4), c.WORKER_SHA),
            'controllers': {'diagnostic': self.file('controller', b'\x7fELFsynthetic controller')},
            'cpu_qualification': self.file('cpu', b'not emitted')}
        self.cpu = {'schema': 'FerricV19Packet55cCpuQualificationV1',
            'runtime_source_sha256': c.RUNTIME_SOURCE_SHA,
            'controller_source_sha256': self.build['controller_source']['sha256'],
            'worker_sha256': c.WORKER_SHA,
            'controllers': {'diagnostic': self.build['controllers']['diagnostic']['sha256']},
            'harness_sources': {name: digest((HERE / name).read_bytes()) for name in
                (*c.OWN_SOURCES, *('measurement/' + name for name in c.MEASUREMENT_SOURCES))},
            'external_sources': {name: digest((HERE.parent / name).read_bytes()) for name in c.EXTERNAL_SOURCES},
            'guard_sources': {profile: {name: self.file('guard/' + name, b'guard', sha)
                for name, sha in pins.items()} for profile, pins in c.GUARD_SOURCE_PINS.items()},
            'phases': {}, 'runtime_evidence': {}, 'controller_evidence': {}, 'harness_evidence': {},
            'v19_evidence': {'qualifier': self.file('v19/helper', b'V19 helper', p.V19_QUALIFIER_SHA),
                             'baseline_controller': self.file('baseline-controller', b'\x7fELFbaseline controller')}}
        self.harness_sources = {'harness/' + name: value for name, value in self.cpu['harness_sources'].items()}
        self.harness_sources.update(self.cpu['external_sources'])
        target = io.BytesIO()
        with tarfile.open(fileobj=target, mode='w:gz') as archive:
            for name in self.harness_sources:
                raw = (HERE.parent / name).read_bytes()
                item = tarfile.TarInfo(name)
                item.size = len(raw)
                archive.addfile(item, io.BytesIO(raw))
        helper = (HERE.parent / 'prepare_qualification.py').read_bytes().replace(
            b'ENABLED = False\n', b'ENABLED = True\n', 1)
        self.cpu['harness_evidence'] = {'archive': self.file('harness/archive', target.getvalue()),
                                         'helper': self.file('harness/helper', helper)}
        clean = {'status': 0, 'reason': 'completed', 'returncode': 0, 'cleanup_ok': True,
                 'child_reaped': True, 'errors': [], 'term_sent': False, 'kill_sent': False,
                 'log_limit_exceeded': False}
        for name, sha in r.PINS.items():
            raw = (b'0\n' if name.endswith('-status') else c.encoded(clean) if name.endswith('-result')
                   else b'test result: ok. 734 passed; 0 failed; 3 ignored;\n'
                   b'test result: ok. 9 passed; 0 failed; 0 ignored;\n' if name == 'full-stdout'
                   else b'fixed existing evidence')
            self.cpu['runtime_evidence'][name] = self.file('runtime/' + name, raw, sha)
        evidence = {name: self.file('evidence/' + name, b'fixed helper', sha)
                    for name, sha in p.HELPERS.items()}
        evidence.update(base_source=self.file('base', b'base archive', p.BASE_SHA),
            source_roster=self.json('roster', self.after), source_modes=self.json('modes', self.modes))
        self.cpu['controller_evidence'] = evidence
        source_sha = evidence['source_roster']['sha256']
        for name in sorted(c.PHASES):
            argv = c.expected_command(name, self.cpu['harness_evidence']['archive']['sha256'])
            resources = {key: c.CPU_LIMITS[key] + 1 for key in
                         ('memory_available_bytes', 'root_free_bytes', 'shm_free_bytes')}
            resources['stage_bytes'] = 1
            result = {**clean, 'profile': c.PACKET_PROFILE, 'cwd': c.D, 'limits': dict(c.CPU_LIMITS),
                'cpus': [0, 1, 2, 3], 'nice': 19, 'build_jobs': 4, 'rust_test_threads': 1,
                'peak_observed_rss_bytes': 1, 'admission': dict(resources), 'final_resources': dict(resources),
                'argv': argv, 'launch_environment': {'CARGO_BUILD_JOBS': '4', 'RUST_TEST_THREADS': '1',
                    'FERRIC_CPU_PROFILE': c.PACKET_PROFILE, 'CUDA_VISIBLE_DEVICES': '-1',
                    'HIP_VISIBLE_DEVICES': '-1', 'HSA_VISIBLE_DEVICES': '-1', 'ROCR_VISIBLE_DEVICES': '-1'}}
            phase = {'profile': c.PACKET_PROFILE, 'argv_sha256': digest(c.encoded(argv)),
                     'status': self.file(name + '/status', b'0\n'), 'result': self.json(name + '/result', result)}
            self.cpu['phases'][name] = phase
            for field in set(c.phase_receipt_fields(name)) - {'status', 'result'}:
                phase[field] = self.file(name + '/' + field, b'')
            if name in c.CLIENT_COMMANDS:
                role = c.CLIENT_COMMANDS[name]
                phase['helper'] = copy.deepcopy(evidence['qualifier'])
                phase['inner'] = self.json(name + '/inner', {
                    'schema': 'FerricBaselinePacket55cCpuRoleV1', 'role': role,
                    'source': str(q.S), 'source_files': 1433, 'source_roster_sha256': source_sha,
                    'base_archive_sha256': p.BASE_SHA, 'runtime_revision': c.RUNTIME_MAIN,
                    'helper_sha256': p.QUALIFIER_SHA, 'profile': c.PACKET_PROFILE, 'returncode': 0,
                    'source_verified_after': True, 'native_executed': False, 'latency_sample_admitted': False,
                    'commands': q.commands(role), 'planning_allowance_bytes': p.ALLOWANCES[role] * 1024**2,
                    'overlays': {key: list(value) for key, value in q.OVERLAYS.items()},
                    'allocation_before': self.allocation(p.ALLOWANCES[role] * 1024**2),
                    'allocation_after': self.allocation(0), 'artifacts': {},
                    'scope': 'CPU qualification only; prior test roles bind actual raw passing counts'})
            if name in c.RUST_TEST_ROLES:
                self.output(name, ('test result: ok. %d passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'
                                  % p.TEST_COUNTS[name.split('-', 1)[1]]).encode())
            elif name in c.PYTHON_COMMANDS:
                self.output(name, ('Ran %d tests in 0.001s\n\nOK\n' % c.PYTHON_COUNTS[name]).encode())
                directory, pattern = c.PYTHON_COMMANDS[name]
                phase['helper'] = copy.deepcopy(self.cpu['harness_evidence']['helper'])
                phase['inner'] = self.json(name + '/inner', {
                    'schema': 'FerricV19PacketHarnessCustodyV1', 'action': name,
                    'argv': ['/usr/bin/python3', '-I', '-B', '-m', 'unittest', 'discover',
                             '-s', directory, '-p', pattern, '-v'],
                    'source_before': dict(self.harness_sources), 'source_after': dict(self.harness_sources),
                    'source_root': c.HARNESS_ROOT,
                    'archive_sha256': self.cpu['harness_evidence']['archive']['sha256'],
                    'helper_sha256': phase['helper']['sha256'], 'authored_test_count': c.PYTHON_COUNTS[name],
                    'returncode': 0, 'native_executed': False, 'native_qualified': False,
                    'allocation_before': self.allocation(64 * 1024**2), 'allocation_after': self.allocation(0)})
        binary = self.cpu['v19_evidence']['baseline_controller']
        controller = {'path': str(q.TARGET / 'release' / q.BINARY), 'sha256': binary['sha256'],
                      'size_bytes': len(self.raw[binary['path']][0])}
        self.inner('packet-release')['artifacts'] = {'controller': controller}
        self.inner('packet-retention')['artifacts'] = {
            'controller': {**controller, 'path': str(q.O / 'retained' / q.BINARY)},
            'source_archive': {'path': str(q.O / 'retained/ferric-packet-baseline-55c1a9b6-a004.tar.gz'),
                'sha256': self.build['controller_source']['sha256'], 'size_bytes': len(b'candidate archive')},
            'phases': {role: {'inner': copy.deepcopy(self.cpu['phases']['packet-' + role]['inner']),
                'result': copy.deepcopy(self.cpu['phases']['packet-' + role]['result']),
                'stdout_sha256': self.cpu['phases']['packet-' + role]['stdout']['sha256'],
                'stderr_sha256': self.cpu['phases']['packet-' + role]['stderr']['sha256']} for role in p.ROLES[:-1]}}
        for role in v.ROLES:
            name = 'v19-' + role
            phase = self.cpu['phases'][name]
            phase['helper'] = copy.deepcopy(self.cpu['v19_evidence']['qualifier'])
            phase['inner'] = self.json(name + '/inner', {
                'schema': 'FerricV19Packet55cCpuRoleV1', 'role': role, 'source': str(q.S),
                'source_files': 1433, 'source_roster_sha256': v.SOURCE_ROSTER_SHA,
                'runtime_revision': c.RUNTIME_MAIN, 'helper_sha256': p.V19_QUALIFIER_SHA,
                'base_helper_sha256': p.QUALIFIER_SHA, 'profile': c.PACKET_PROFILE,
                'commands': v.commands(q, role), 'returncode': 0, 'source_verified_after': True,
                'planning_allowance_bytes': v.ALLOWANCES[role] * 1024**2,
                'reused_test_roles': {'timestamps': 32, 'source-policy': 58},
                'native_executed': False, 'latency_sample_admitted': False,
                'scope': 'same A004 source and features; V19 kernel composition, not native publication timing',
                'allocation_before': self.allocation(v.ALLOWANCES[role] * 1024**2),
                'allocation_after': self.allocation(0), 'artifacts': {},
                'baseline_phases': self.custody(p.ROLES, 'packet-')})
        binary = self.build['controllers']['diagnostic']
        controller = {'path': str(q.TARGET / 'release' / v.BINARY), 'sha256': binary['sha256'],
                      'size_bytes': len(self.raw[binary['path']][0])}
        self.inner('v19-release')['artifacts'] = {'controller': controller}
        self.inner('v19-retention')['artifacts'] = {
            'controller': {**controller, 'path': str(v.O / 'retained' / v.BINARY)},
            'source_archive': copy.deepcopy(self.inner('packet-retention')['artifacts']['source_archive']),
            'phases': self.custody(v.ROLES[:-1], 'v19-')}

    def custody(self, roles, prefix):
        return {role: {'inner': copy.deepcopy(self.cpu['phases'][prefix + role]['inner']),
            'result': copy.deepcopy(self.cpu['phases'][prefix + role]['result']),
            'stdout_sha256': self.cpu['phases'][prefix + role]['stdout']['sha256'],
            'stderr_sha256': self.cpu['phases'][prefix + role]['stderr']['sha256']} for role in roles}

    @staticmethod
    def allocation(planned):
        return {'stage_allocated_bytes': 1024, 'stage_cap_bytes': 38654705664,
                'stage_reserve_bytes': 536870912, 'remaining_reserved_bytes': 38117833728,
                'planned_increment_bytes': planned}

    def file(self, name, raw, sha=None):
        item = {'path': '/synthetic/' + name, 'sha256': sha or digest(raw)}
        self.raw[item['path']] = (raw, item['sha256'])
        return item

    def json(self, name, value):
        item = self.file(name, c.encoded(value))
        self.results[item['path']] = value
        return item

    def read_raw(self, path, expected, maximum, empty=False):
        raw, sha = self.raw[str(path)]
        c.require(sha == expected and len(raw) <= maximum and (raw or empty), 'synthetic raw identity')
        return raw, sha

    def read_bound(self, item):
        return self.results[item['path']]

    def result(self, name):
        return self.read_bound(self.cpu['phases'][name]['result'])

    def inner(self, name):
        return self.read_bound(self.cpu['phases'][name]['inner'])

    def output(self, name, raw):
        key = 'stdout' if name in c.RUST_TEST_ROLES else 'stderr'
        self.cpu['phases'][name][key] = self.file(name + '/' + key, raw)

    def validate(self):
        def archives(raw, _):
            if raw == b'base archive':
                return copy.deepcopy(self.before), copy.deepcopy(self.modes)
            c.require(raw == b'candidate archive', 'known synthetic archive')
            return copy.deepcopy(self.after), copy.deepcopy(self.modes)
        actual_module = c.module
        def modules(path, expected):
            if str(path) == self.cpu['controller_evidence']['qualifier']['path']:
                c.require(expected == p.QUALIFIER_SHA, 'fixed qualifier')
                return q
            if str(path) == self.cpu['v19_evidence']['qualifier']['path']:
                c.require(expected == p.V19_QUALIFIER_SHA, 'fixed V19 qualifier')
                return v
            return actual_module(path, expected)
        c.validate_build_inputs(self.build)
        with mock.patch.object(c, 'controller_binding_module', return_value=p), \
                mock.patch.object(c, 'module', side_effect=modules), \
                mock.patch.object(p, 'archive_state', side_effect=archives):
            return c.validate_cpu(self.cpu, self.build, self.read_bound, self.read_raw)


class PacketCpuBindingTests(unittest.TestCase):
    def test_v19_roles_cannot_reuse_baseline_graph_or_parser_coverage(self):
        for key, value in (('commands', q.commands('graph')), ('reused_test_roles', {'graph': 3}),
                           ('helper_sha256', p.QUALIFIER_SHA), ('source_roster_sha256', 'f' * 64)):
            f = Fixture()
            f.inner('v19-graph')[key] = value
            with self.assertRaises(ValueError):
                f.validate()
        f = Fixture()
        f.inner('v19-parser')['baseline_phases']['timestamps']['stdout_sha256'] = 'f' * 64
        with self.assertRaises(ValueError):
            f.validate()

    def test_v19_retained_binary_cannot_be_the_baseline_or_different_source(self):
        for field in ('controller', 'source_archive', 'phases'):
            f = Fixture()
            actual = f.inner('v19-retention')['artifacts']
            if field == 'controller':
                actual[field]['sha256'] = f.cpu['v19_evidence']['baseline_controller']['sha256']
            elif field == 'source_archive':
                actual[field]['sha256'] = 'f' * 64
            else:
                actual[field]['graph']['inner']['sha256'] = 'f' * 64
            with self.assertRaises(ValueError):
                f.validate()

    def test_complete_current_controller_and_runtime(self):
        counts = Fixture().validate()
        self.assertEqual(sum(counts['packet-' + name]['passed'] for name in p.TEST_COUNTS), 97)
        self.assertEqual(set(counts), c.RUST_TEST_ROLES | set(c.PYTHON_COMMANDS))

    def test_qualifier_copy_is_exact_reviewed_source(self):
        self.assertEqual(digest((HERE / 'controller_qualifier.py').read_bytes()), p.QUALIFIER_SHA)
        self.assertEqual(c.PACKET_QUALIFIER_SHA, p.QUALIFIER_SHA)
        self.assertEqual(digest((HERE / 'v19_qualifier.py').read_bytes()), p.V19_QUALIFIER_SHA)

    def test_role_omission_and_old_schema_reject(self):
        for role in c.PHASES:
            f = Fixture()
            f.cpu['phases'].pop(role)
            with self.assertRaises(ValueError):
                f.validate()
        f = Fixture()
        f.cpu['schema'] = 'FerricPacketTicksCpuQualificationV1'
        with self.assertRaises(ValueError):
            f.validate()

    def test_limits_placement_and_outcome_reject(self):
        for key, value in (('status', 125), ('cleanup_ok', False), ('child_reaped', False),
                           ('term_sent', True), ('kill_sent', True), ('errors', ['failure']),
                           ('nice', 0), ('cpus', [0]), ('cwd', '/other')):
            f = Fixture()
            f.result('packet-release')[key] = value
            with self.assertRaises(ValueError):
                f.validate()
        for key in c.CPU_LIMITS:
            f = Fixture()
            f.result('packet-release')['limits'][key] += 1
            with self.assertRaises(ValueError):
                f.validate()

    def test_inner_source_helper_commands_and_overlays_reject(self):
        for key, value in (('source_files', 1432), ('helper_sha256', 'f' * 64),
                           ('source_verified_after', False), ('native_executed', True),
                           ('commands', []), ('overlays', {}), ('returncode', 1)):
            f = Fixture()
            f.inner('packet-parser')[key] = value
            with self.assertRaises(ValueError):
                f.validate()

    def test_allocation_envelope_and_reserve_reject(self):
        for field in Fixture.allocation(0):
            f = Fixture()
            f.inner('packet-parser')['allocation_before'][field] += 1
            with self.assertRaises(ValueError):
                f.validate()

    def test_actual_test_counts_skips_and_duplicate_footers_reject(self):
        for role in c.RUST_TEST_ROLES:
            for raw in (b'', b'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n',
                        b'test result: ok. 32 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out;\n'):
                f = Fixture()
                f.output(role, raw)
                with self.assertRaises(ValueError):
                    f.validate()

    def test_filtered_policy_is_not_a_complete_suite(self):
        f = Fixture()
        f.output('packet-source-policy', b'test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out;\n')
        with self.assertRaises(ValueError):
            f.validate()

    def test_python_zero_skip_failure_and_missing_footer_reject(self):
        for raw in (b'Ran 0 tests in 0.1s\n\nOK\n', b'Ran 3 tests in 0.1s\n\nOK (skipped=1)\n',
                    b'Ran 3 tests in 0.1s\n\nFAILED\n', b'Ran 3 tests in 0.1s\n'):
            f = Fixture()
            f.output('packet-replay-tests', raw)
            with self.assertRaises(ValueError):
                f.validate()

    def test_self_rehashed_outer_argv_cannot_change_command(self):
        for role in c.PHASES:
            f = Fixture()
            f.result(role)['argv'].append('--list')
            f.cpu['phases'][role]['argv_sha256'] = digest(c.encoded(f.result(role)['argv']))
            with self.assertRaises(ValueError):
                f.validate()

    def test_full_source_closure_preimages_and_retained_roster_reject(self):
        for change in ('before', 'after', 'extra', 'retained'):
            f = Fixture()
            name = next(iter(q.OVERLAYS))
            if change == 'before':
                f.before[name] = 'f' * 64
            elif change == 'after':
                f.after[name] = 'f' * 64
            elif change == 'extra':
                f.after['unexpected'] = 'f' * 64
            else:
                f.read_bound(f.cpu['controller_evidence']['source_roster'])[name] = 'f' * 64
            with self.assertRaises(ValueError):
                f.validate()

    def test_runtime_exact_roster_clean_receipts_and_full_counts(self):
        for change in ('missing', 'hash', 'status', 'counts'):
            f = Fixture()
            name = 'full-stdout' if change == 'counts' else next(name for name in r.PINS if name.endswith('-status'))
            evidence = f.cpu['runtime_evidence']
            if change == 'missing':
                evidence.pop(name)
            elif change == 'hash':
                evidence[name]['sha256'] = 'f' * 64
            else:
                item = evidence[name]
                f.raw[item['path']] = (b'1\n' if change == 'status' else b'no tests', item['sha256'])
            with self.assertRaises(ValueError):
                f.validate()

    def test_retention_binary_size_source_and_raw_custody_reject(self):
        for change in ('size', 'source', 'phase', 'raw'):
            f = Fixture()
            retained = f.inner('packet-retention')['artifacts']
            if change == 'size':
                retained['controller']['size_bytes'] += 1
            elif change == 'source':
                retained['source_archive']['sha256'] = 'f' * 64
            elif change == 'phase':
                retained['phases'].pop('stage')
            else:
                retained['phases']['parser']['stdout_sha256'] = 'f' * 64
            with self.assertRaises(ValueError):
                f.validate()

    def test_native_emit_is_disabled_before_output_creation(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'uncreated'
            with mock.patch.object(binder.c, 'PACKET_TICKS_QUALIFIED', False):
                with self.assertRaises(ValueError):
                    binder.emit({}, HERE, output)
            self.assertFalse(output.exists())

    def test_every_raw_role_runtime_and_source_is_staged(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            stage = root / 'stage'
            stage.mkdir()
            cpu = Fixture().cpu
            records = [phase[key] for name, phase in cpu['phases'].items() for key in c.phase_receipt_fields(name)]
            records += [item for group in cpu['guard_sources'].values() for item in group.values()]
            records += list(cpu['controller_evidence'].values()) + list(cpu['runtime_evidence'].values())
            records += list(cpu['harness_evidence'].values())
            records += list(cpu['v19_evidence'].values())
            for index, item in enumerate(records):
                path = root / str(index)
                raw = str(index).encode()
                path.write_bytes(raw)
                item.update(path=str(path), sha256=digest(raw))
            files = {}
            stager.stage_cpu_receipts(stage, cpu, files)
            c.validate_cpu_stage(stage, files, cpu)
            files.pop('evidence/packet-parser/stdout')
            with self.assertRaises(ValueError):
                c.validate_cpu_stage(stage, files, cpu)

    def test_tar_reader_full_roster_and_unsafe_members(self):
        def archive(names):
            target = io.BytesIO()
            with tarfile.open(fileobj=target, mode='w:gz') as stream:
                for name in names:
                    item = tarfile.TarInfo(name)
                    item.mode = 0o600
                    stream.addfile(item, io.BytesIO(b''))
            return target.getvalue()
        raw = archive(['source/' + str(index) for index in range(1433)])
        files, modes = p.archive_state(raw, c.require)
        self.assertEqual(len(files), 1433)
        self.assertEqual(set(modes.values()), {0o600})
        for names in (['../escape'], ['/absolute'], ['same', './same'], ['one']):
            with self.assertRaises(ValueError):
                p.archive_state(archive(names), c.require)


if __name__ == '__main__':
    unittest.main()
