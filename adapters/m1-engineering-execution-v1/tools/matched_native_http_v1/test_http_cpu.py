import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest

definition = importlib.util.spec_from_file_location('http_cpu_tests', Path(__file__).with_name('http_cpu.py'))
m = importlib.util.module_from_spec(definition)
definition.loader.exec_module(m)


def fixture():
    sources = {'run_matched128.py': 'a' * 64}
    binding = lambda name, digest='b' * 64: {'path': '/input/' + name, 'sha256': digest}
    argv = ['/bin/bash', '/original/owner/' + m.PROFILES[m.PROFILE][3], '/usr/bin/python3',
            '-I', '-B', '-m', 'unittest', 'discover', '-s', '/original/source', '-p', 'test_*.py', '-v']
    encoded = json.dumps(argv, sort_keys=True, separators=(',', ':')).encode() + b'\n'
    value = {'schema': 'FerricNativeHttpCpuQualificationV1', 'driver_sources': sources,
        'test_sources': {name: 'c' * 64 for name in m.TEST_FILES},
        'guard': binding('guard', m.GUARD_SHA), 'environment': binding('environment', m.ENV_SHA),
        'phase': {'status': binding('status'), 'result': binding('result'),
                  'stderr': binding('stderr'), 'argv_sha256': hashlib.sha256(encoded).hexdigest()},
        'source_root': '/original/source', 'servers_launched': False}
    result = {'profile': m.PROFILE, 'status': 0, 'reason': 'completed', 'returncode': 0,
        'cleanup_ok': True, 'child_reaped': True, 'errors': [], 'term_sent': False, 'kill_sent': False,
        'log_limit_exceeded': False, 'build_jobs': 4, 'rust_test_threads': 1, 'nice': 19,
        'cpus': [0, 1, 2, 3], 'limits': dict(m.LIMITS), 'peak_observed_rss_bytes': 1, 'argv': argv,
        'launch_environment': {'CARGO_BUILD_JOBS': '4', 'RUST_TEST_THREADS': '1',
            'FERRIC_CPU_PROFILE': m.PROFILE, 'CUDA_VISIBLE_DEVICES': '-1', 'HIP_VISIBLE_DEVICES': '-1',
            'HSA_VISIBLE_DEVICES': '-1', 'ROCR_VISIBLE_DEVICES': '-1'}}
    for name in ('admission', 'final_resources'):
        result[name] = {key: m.LIMITS[key] for key in ('memory_available_bytes', 'root_free_bytes', 'shm_free_bytes')}
        result[name]['stage_bytes'] = 1
    return value, sources, result


class CpuTests(unittest.TestCase):
    def validate(self, value, sources, result, output=None):
        output = output if output is not None else ('Ran ' + str(m.EXPECTED_TESTS) + ' tests in 2.0s\n\nOK\n').encode()
        def digest(path):
            name = Path(path).name
            return value['guard']['sha256'] if name == 'guard' else value['environment']['sha256'] if name == 'environment' else 'c' * 64
        return m.validate(value, sources, bound=lambda _: result,
            raw=lambda item, _: b'0\n' if item['path'].endswith('status') else output,
            digest=digest, root=Path('/qualified'))

    def test_exact_profile_raw_command_and_all_fixtures_admitted(self):
        self.assertTrue(self.validate(*fixture())['accepted'])

    def test_failed_cleanup_or_signals_cannot_be_requalified(self):
        for key, item in (('cleanup_ok', False), ('term_sent', True), ('child_reaped', False), ('status', 125)):
            value, sources, result = fixture()
            result[key] = item
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.validate(value, sources, result)

    def test_wrong_guard_profile_or_stage_limit_rejected(self):
        value, sources, result = fixture()
        for profile in ('FerricCpuFourCore24GiBEmitterV1', 'FerricCpuFourCore28GiBEmitterV1'):
            result['profile'] = profile
            with self.subTest(profile=profile), self.assertRaises(ValueError):
                self.validate(value, sources, result)
        result['profile'] = m.PROFILE
        result['limits']['stage_bytes'] += 1
        with self.assertRaises(ValueError):
            self.validate(value, sources, result)

    def test_historical_g30_g32_require_their_own_exact_guard_cap_and_wrapper(self):
        for profile in ('FerricCpuFourCore30GiBEmitterV1', 'FerricCpuFourCore32GiBEmitterV1',
                        'FerricCpuFourCore42GiBEmitterV1', 'FerricCpuFourCore44GiBEmitterV1',
                        'FerricCpuFourCore56GiBEmitterV1'):
            value, sources, result = fixture()
            guard, environment, cap, wrapper = m.PROFILES[profile]
            result['profile'] = result['launch_environment']['FERRIC_CPU_PROFILE'] = profile
            result['limits']['stage_bytes'] = cap
            result['argv'][1] = '/original/owner/' + wrapper
            value['guard']['sha256'], value['environment']['sha256'] = guard, environment
            raw = json.dumps(result['argv'], sort_keys=True, separators=(',', ':')).encode() + b'\n'
            value['phase']['argv_sha256'] = hashlib.sha256(raw).hexdigest()
            with self.subTest(profile=profile):
                self.assertTrue(self.validate(value, sources, result)['accepted'])

    def test_g44_mismatched_guard_environment_wrapper_or_cap_refused(self):
        profile = 'FerricCpuFourCore44GiBEmitterV1'
        guard, environment, cap, wrapper = m.PROFILES[profile]
        for field in ('guard', 'environment', 'wrapper', 'cap'):
            value, sources, result = fixture()
            result['profile'] = result['launch_environment']['FERRIC_CPU_PROFILE'] = profile
            result['limits']['stage_bytes'] = cap
            result['argv'][1] = '/original/owner/' + wrapper
            value['guard']['sha256'], value['environment']['sha256'] = guard, environment
            if field == 'cap':
                result['limits']['stage_bytes'] += 1
            elif field == 'wrapper':
                result['argv'][1] = '/original/owner/' + m.PROFILES[m.PROFILE][3]
            else:
                value[field]['sha256'] = m.PROFILES[m.PROFILE][0 if field == 'guard' else 1]
            encoded = json.dumps(result['argv'], sort_keys=True, separators=(',', ':')).encode() + b'\n'
            value['phase']['argv_sha256'] = hashlib.sha256(encoded).hexdigest()
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.validate(value, sources, result)

    def test_cross_profile_guard_or_wrapper_mixture_is_rejected(self):
        for field in ('guard', 'environment', 'wrapper'):
            value, sources, result = fixture()
            old = m.PROFILES['FerricCpuFourCore30GiBEmitterV1']
            if field == 'wrapper':
                result['argv'][1] = '/original/owner/' + old[3]
            else:
                value[field]['sha256'] = old[0 if field == 'guard' else 1]
            raw = json.dumps(result['argv'], sort_keys=True, separators=(',', ':')).encode() + b'\n'
            value['phase']['argv_sha256'] = hashlib.sha256(raw).hexdigest()
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.validate(value, sources, result)

    def test_unknown_profile_or_relaxed_reserve_is_rejected(self):
        for field, item in (('profile', 'FerricCpuFourCore34GiBEmitterV1'),
                            ('stage_reserve_bytes', 384 * 1024**2)):
            value, sources, result = fixture()
            if field == 'profile':
                result[field] = item
            else:
                result['limits'][field] = item
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.validate(value, sources, result)

    def test_list_filtered_empty_or_skipped_test_output_rejected(self):
        for output in (b'Ran 0 tests in 0.0s\nOK\n', b'Ran 75 tests in 1.0s\nOK (skipped=1)\n',
                       b'Ran 75 tests in 1.0s\nFAILED (errors=1)\n'):
            with self.subTest(output=output), self.assertRaises(ValueError):
                self.validate(*fixture(), output=output)

    def test_source_or_fixture_roster_drift_rejected(self):
        value, sources, result = fixture()
        value['driver_sources'] = {'run_matched128.py': 'f' * 64}
        with self.assertRaises(ValueError):
            self.validate(value, sources, result)
        value['driver_sources'] = sources
        del value['test_sources'][m.TEST_FILES[0]]
        with self.assertRaises(ValueError):
            self.validate(value, sources, result)

    def test_filtered_command_rejected_even_if_argv_hash_is_updated(self):
        value, sources, result = fixture()
        result['argv'] += ['-k', 'one']
        raw = json.dumps(result['argv'], sort_keys=True, separators=(',', ':')).encode() + b'\n'
        value['phase']['argv_sha256'] = hashlib.sha256(raw).hexdigest()
        with self.assertRaises(ValueError):
            self.validate(value, sources, result)

    def test_gpu_visibility_or_insufficient_resources_refused(self):
        value, sources, result = fixture()
        result['launch_environment']['HIP_VISIBLE_DEVICES'] = '0'
        with self.assertRaises(ValueError):
            self.validate(value, sources, result)
        result['launch_environment']['HIP_VISIBLE_DEVICES'] = '-1'
        result['final_resources']['stage_bytes'] = m.LIMITS['stage_bytes']
        with self.assertRaises(ValueError):
            self.validate(value, sources, result)


if __name__ == '__main__':
    unittest.main()
