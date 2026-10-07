"""Pure binding fixtures use retained actual controller/runtime raw evidence."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

D = Path(__file__).resolve().parent


def load():
    spec = importlib.util.spec_from_file_location('splitk_cpu_test_contract', D / 'launch_contract.py')
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


class Evidence:
    def __init__(self, c):
        self.c = c
        data = json.loads((D / 'fixtures/retained_cpu.json').read_bytes())
        self.raw = {path: value.encode() for path, value in data['raw'].items()}
        self.cpu = {key: copy.deepcopy(data[key]) for key in ('runtime_evidence', 'experimental_retention', 'phases')}
        self.cpu.update(schema='FerricSplitKExperimentalCpuV1',
            harness_sources={name: 'a' * 64 for name in
                (*c.OWN_SOURCES, *('measurement/' + name for name in c.MEASUREMENT_SOURCES))})
        self.helper = b'synthetic wrapper only for pure receipt fixtures\n'
        self.helper_sha = hashlib.sha256(self.helper).hexdigest()
        for role in c.experimental.HARNESS_ROLES:
            helper_path = c.experimental.D + '/inputs/splitk-model-harness-a003/qualify.py'
            root = c.experimental.D + '/splitk-model-harness-a003/harness'
            inner = {'schema': 'FerricSplitKHarnessCpuV1', 'role': role,
                'profile': c.experimental.G36, 'returncode': 0, 'helper_sha256': self.helper_sha,
                'source_before': self.cpu['harness_sources'], 'source_after': self.cpu['harness_sources'],
                'tests': 1, 'native_executed': False, 'accepted': True, 'error': None, 'test_stdout': '',
                'test_stderr': 'test_fixture ... ok\n\nRan 1 test in 0.001s\n\nOK\n',
                'command': ['/usr/bin/python3', '-I', '-B', '-m', 'unittest', 'discover', '-s',
                    root if role == 'launch-tests' else root + '/measurement', '-p', 'test_*.py', '-v']}
            result = self.bound(self.cpu['phases']['experimental-retention']['result'])
            result['argv'] = ['/bin/bash', c.experimental.D + '/owner/cpu-env-36g-emitter.sh',
                              '/usr/bin/python3', '-I', '-B', helper_path, role]
            phase = {field: self.put('/fixture/' + role + '/' + field, raw) for field, raw in {
                'status': b'0\n', 'result': c.encoded(result), 'stdout': c.encoded(inner),
                'stderr': b'', 'inner': c.encoded(inner), 'helper': self.helper}.items()}
            phase.update(profile=c.experimental.G36, argv_sha256=hashlib.sha256(c.encoded(result['argv'])).hexdigest())
            self.cpu['phases'][role] = phase
        image = {'path': '/private/splitk', **c.selection.IMAGE}
        roster = {'path': '/private/roots.json', 'sha256': 'b' * 64,
                  'value': [{'logical_name': key, 'export_name': key} for key in c.selection.ROOTS]}
        self.build = {'schema': 'FerricSplitKExperimentalBuildV1', 'runtime_main': c.RUNTIME_MAIN,
            'runtime_source': {'path': '/private/core.tar.gz', 'sha256': c.experimental.RUNTIME_SOURCE},
            'controller_source': {'path': '/private/client.tar.gz', 'sha256': c.experimental.CLIENT_SOURCE},
            'controller': {'path': '/private/controller', 'sha256': c.experimental.CONTROLLER},
            'worker': {'path': '/private/worker', 'sha256': c.experimental.WORKER},
            'experimental_retention': self.cpu['experimental_retention'],
            'cpu_qualification': {'path': '/private/cpu.json', 'sha256': 'f' * 64},
            'splitk': {'image': image, 'roster': roster}}

    def install(self, test):
        for name, value in {'SPLITK_QUALIFIED': True, 'HARNESS_HELPER_SHA': self.helper_sha,
                            'HARNESS_TEST_COUNTS': {'launch-tests': 1, 'measurement-tests': 1}}.items():
            p = patch.object(self.c, name, value)
            p.start()
            test.addCleanup(p.stop)
        return self

    def put(self, path, raw):
        self.raw[path] = raw
        return {'path': path, 'sha256': hashlib.sha256(raw).hexdigest()}

    def read(self, path, expected=None, maximum=32 * 1024**2, empty=False):
        raw = self.raw[str(path)]
        digest = hashlib.sha256(raw).hexdigest()
        self.c.require(expected is None or expected == digest, 'fixture actual raw hash mismatch')
        self.c.require(len(raw) <= maximum and (raw or empty), 'fixture raw bound')
        return raw, digest

    def bound(self, item):
        self.c.binding(item)
        return self.c.decode(self.read(item['path'], item['sha256'])[0])

    def validate(self):
        self.c.validate_build(self.build)
        return self.c.validate_cpu(self.cpu, self.build, self.bound, self.read)

    def change_result(self, role, action, rehash=True):
        phase = self.cpu['phases'][role]
        value = self.bound(phase['result'])
        action(value)
        item = self.put(phase['result']['path'], self.c.encoded(value))
        if rehash:
            phase['result'] = item
            phase['argv_sha256'] = hashlib.sha256(self.c.encoded(value['argv'])).hexdigest()


class CpuBindingTests(unittest.TestCase):
    def setUp(self):
        self.c = load()
        self.e = Evidence(self.c).install(self)

    def test_actual91_and_explicit_failed_clippy_with_runtime_only_reuse(self):
        result = self.e.validate()
        self.assertEqual(result['controller_test_passes'], 91)
        self.assertEqual(result['width_controller_passes_credited'], 0)
        self.assertEqual(result['strict_clippy'], 'failed')
        self.assertFalse(result['production_qualified'])

    def test_native_disabled_even_with_all_artifacts(self):
        self.c.SPLITK_QUALIFIED = False
        with self.assertRaisesRegex(ValueError, 'disabled'):
            self.e.validate()

    def test_source_worker_and_controller_pins_are_closed(self):
        for field in ('runtime_source', 'controller_source', 'worker', 'controller', 'experimental_retention'):
            saved = copy.deepcopy(self.e.build)
            self.e.build[field]['sha256'] = '0' * 64
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.e.validate()
            self.e.build = saved

    def test_borrowed_width_controller_test_cannot_replace_splitk_role(self):
        self.e.cpu['phases']['focused'] = copy.deepcopy(self.e.cpu['phases']['runtime-tests'])
        with self.assertRaises(ValueError):
            self.e.validate()

    def test_unknown_or_missing_role_rejected(self):
        self.e.cpu['phases']['other'] = self.e.cpu['phases'].pop('focused')
        with self.assertRaisesRegex(ValueError, 'roster'):
            self.e.validate()

    def test_wrong_historical_runtime_build_or_cpu(self):
        self.e.cpu['runtime_evidence']['cpu']['sha256'] = '1' * 64
        with self.assertRaisesRegex(ValueError, 'exact accepted'):
            self.e.validate()

    def test_dirty_guard_or_nonzero_source_test_cannot_be_rehashed(self):
        for field, value in (('status', 1), ('cleanup_ok', False), ('child_reaped', False),
                             ('term_sent', True), ('kill_sent', True), ('errors', ['failure'])):
            saved = copy.deepcopy(self.e.cpu), copy.deepcopy(self.e.raw)
            self.e.change_result('focused', lambda row: row.update({field: value}))
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.e.validate()
            self.e.cpu, self.e.raw = saved

    def test_changed_guard_cap_reserve_cpu_or_visibility(self):
        for action in (lambda row: row['limits'].update(stage_bytes=40 * 1024**3),
                       lambda row: row['limits'].update(stage_reserve_bytes=0),
                       lambda row: row.update(cpus=[0, 1, 2, 3, 4]),
                       lambda row: row['launch_environment'].update(HIP_VISIBLE_DEVICES='0')):
            saved = copy.deepcopy(self.e.cpu), copy.deepcopy(self.e.raw)
            self.e.change_result('focused', action)
            with self.assertRaises(ValueError):
                self.e.validate()
            self.e.cpu, self.e.raw = saved

    def test_clippy_failure_cannot_be_promoted(self):
        self.e.change_result('failed-clippy', lambda row: row.update(status=0, returncode=0))
        with self.assertRaises(ValueError):
            self.e.validate()

    def test_clippy_raw_stderr_cannot_be_omitted(self):
        self.e.cpu['phases']['failed-clippy']['stderr'] = self.e.put('/fake/empty', b'')
        with self.assertRaises(ValueError):
            self.e.validate()

    def test_failed_clippy_cannot_be_replaced_with_unrelated_failure(self):
        self.e.change_result('failed-clippy', lambda row: row.update(argv=['/bin/false']))
        with self.assertRaisesRegex(ValueError, 'Clippy raw custody'):
            self.e.validate()

    def test_image_gate_cannot_be_replaced_with_list_or_true(self):
        self.e.change_result('actual-image', lambda row: row.update(argv=['/bin/true']))
        with self.assertRaisesRegex(ValueError, 'retained split-K role'):
            self.e.validate()

    def test_outer_stdout_binds_inner_and_source(self):
        role = 'launch-tests'
        inner = self.e.bound(self.e.cpu['phases'][role]['inner'])
        inner['source_after'] = {}
        self.e.cpu['phases'][role]['inner'] = self.e.put('/changed/inner', self.c.encoded(inner))
        with self.assertRaisesRegex(ValueError, 'stdout/inner'):
            self.e.validate()

    def test_noop_harness_footer_and_skip_rejected(self):
        role = 'launch-tests'
        for tail in ('Ran 0 tests in 0.001s\n\nOK\n', 'Ran 1 test in 0.001s\n\nOK (skipped=1)\n'):
            saved = copy.deepcopy(self.e.cpu), copy.deepcopy(self.e.raw)
            inner = self.e.bound(self.e.cpu['phases'][role]['inner'])
            inner['test_stderr'] = '\n' + tail
            raw = self.c.encoded(inner)
            for name in ('inner', 'stdout'):
                self.e.cpu['phases'][role][name] = self.e.put('/changed/' + name, raw)
            with self.assertRaisesRegex(ValueError, 'footer'):
                self.e.validate()
            self.e.cpu, self.e.raw = saved

    def test_harness_command_cannot_change(self):
        self.e.change_result('measurement-tests', lambda row: row.update(argv=['/bin/true']))
        with self.assertRaisesRegex(ValueError, 'harness test invocation'):
            self.e.validate()

    def test_helper_must_be_the_exact_tested_source(self):
        self.e.cpu['phases']['launch-tests']['helper']['sha256'] = '0' * 64
        with self.assertRaises(ValueError):
            self.e.validate()

    def test_staged_evidence_requires_all_portable_roots(self):
        with self.assertRaisesRegex(ValueError, 'outside immutable'):
            self.c.validate_cpu_stage(Path('/stage'), {}, self.e.cpu)

    def test_same_measured_unpaired_image_is_mandatory(self):
        self.e.build['splitk']['image']['hsaco'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'measured unpaired'):
            self.e.validate()


if __name__ == '__main__':
    unittest.main()
