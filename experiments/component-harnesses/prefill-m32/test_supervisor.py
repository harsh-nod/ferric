import ast
from contextlib import ExitStack, nullcontext
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import signal
import sys
import tempfile
import tarfile
import types
import unittest
from unittest.mock import patch

D = Path(__file__).resolve().parent


def load(name):
    spec = importlib.util.spec_from_file_location('component_test_' + name, D / (name + '.py'))
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


c, entry, outer, admission_module = (load(name) for name in
    ('component_contract', 'component_entry', 'supervise_component', 'component_admission'))


class SourceTests(unittest.TestCase):
    def test_admission_class_is_exact_qualified_source(self):
        def body(path):
            text = path.read_text()
            node = next(item for item in ast.parse(text).body
                        if isinstance(item, ast.ClassDef) and item.name == 'Admission')
            return ast.get_source_segment(text, node)
        self.assertEqual(body(D / 'component_admission.py'), body(D / 'frozen/run_stage.py'))

    def test_frozen_pins_and_original_component_unchanged(self):
        for name, expected in c.PINS.items():
            self.assertEqual(hashlib.sha256((D / name).read_bytes()).hexdigest(), expected)
        component = c.load_component(D)
        self.assertTrue(component.PARTIAL.endswith('partial_f32_r1'))
        self.assertTrue(component.MERGE.endswith('merge_f32_r1'))
        self.assertNotIn(component.PARTIAL, c.SYMBOLS['m2'])
        self.assertEqual(len(list(component.schedule())), 30)

    def test_stage_cannot_impersonate_fence_or_v14_campaign(self):
        self.assertTrue(c.NATIVE_ENABLED)
        with patch.object(c, 'NATIVE_ENABLED', False), \
                patch.object(c, 'verify_files', side_effect=AssertionError('file read')):
            with self.assertRaisesRegex(ValueError, 'not qualified/enabled'):
                c.validate_plan({}, Path('/unused'))
        self.assertEqual(c.stage_name('/dev/shm/ferric-prefill-m32-component-latency-a001').parent, Path('/dev/shm'))
        for value in ('/tmp/ferric-v16-splitk-component-a001', '/dev/shm/ferric-v16-fences-dcf3454-a001',
                      '/dev/shm/ferric-v14-native-a001', '/dev/shm/ferric-v16-splitk-component-',
                      '/dev/shm/ferric-v16-splitk-ordered-latency-a001'):
            with self.assertRaises(ValueError):
                c.stage_name(value)

    def test_fuser_refusals_remain_fail_closed(self):
        for rc, out, err in ((1, b'', b'/dev/kfd: error'), (0, b'123', b'warning'), (1, b'123', b'')):
            with self.assertRaises(ValueError):
                admission_module.fuser_members(types.SimpleNamespace(returncode=rc, stdout=out, stderr=err))

    def test_cpu_failure_and_relaxed_limits_rejected(self):
        value = {'status': 0, 'reason': 'completed', 'returncode': 0, 'cleanup_ok': True,
            'child_reaped': True, 'errors': [], 'term_sent': False, 'kill_sent': False,
            'log_limit_exceeded': False, 'profile': 'FerricCpuFourCore24GiBEmitterV1',
            'limits': dict(c.custody.CPU_PROFILES['FerricCpuFourCore24GiBEmitterV1']),
            'cpus': [0, 1, 2, 3], 'nice': 19, 'build_jobs': 4, 'rust_test_threads': 1}
        c.clean_cpu(value)
        for key, bad in (('status', 1), ('child_reaped', False), ('cleanup_ok', False),
                         ('errors', ['error']), ('term_sent', True), ('cpus', [0, 1, 2, 3, 4])):
            with self.assertRaises(ValueError):
                c.clean_cpu({**value, key: bad})
        with self.assertRaises(ValueError):
            c.clean_cpu({**value, 'limits': {**value['limits'], 'stage_bytes': 1 << 40}})


class EndpointTests(unittest.TestCase):
    def test_only_first_and_last_callback_admit(self):
        events = []
        callbacks = entry.SampleEndpoints(lambda: events.append('alive'),
            lambda: events.append('admit') or {'accepted': True},
            lambda after: events.append(('endpoint', after)) or {'after': after})
        for _ in range(32):
            callbacks()
        callbacks.finish()
        self.assertEqual(events[:3], ['alive', 'admit', ('endpoint', False)])
        self.assertEqual(events[-3:], ['alive', 'admit', ('endpoint', True)])
        self.assertEqual(events.count('admit'), 2)
        with self.assertRaises(ValueError):
            callbacks()

    def test_missing_or_refused_endpoint_is_terminal(self):
        callbacks = entry.SampleEndpoints(lambda: None, lambda: {'accepted': False}, lambda _: {})
        with self.assertRaises(ValueError):
            callbacks()
        with self.assertRaises(ValueError):
            callbacks.finish()


class EntryTests(unittest.TestCase):
    def exercise(self, *, fault=None):
        events = []
        with tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
            root = Path(temporary)
            output = root / 'outputs/component'
            output.mkdir(parents=True)
            paths = {}
            for name in ('worker', 'v5', 'm2', 'python', 'source'):
                path = root / name
                path.write_bytes(name.encode())
                paths[name] = {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
            plan = {'device_unique_id': c.DEVICE, 'worker': paths['worker'], 'python': paths['python'], 'mode': 'latency',
                    'images': {'v5': paths['v5'], 'm2': paths['m2']},
                    'sources': {'source': paths['source']['sha256']}, 'files': {}}
            wrapper = {'pid': 101, 'parent': 100, 'group': 101, 'session': 101, 'start': 7, 'uid': os.getuid()}
            parent = {**wrapper, 'pid': 100, 'group': 100, 'session': 100, 'parent': 99, 'start': 6}
            child = {**wrapper, 'pid': 102, 'parent': 101, 'start': 8}
            if fault == 'worker-group':
                child = {**child, 'group': 102}
            fd = os.open(root / 'source', os.O_RDONLY)
            stack.callback(lambda: os.close(fd))
            lifecycle = types.SimpleNamespace(
                process=lambda pid: {100: parent, 101: wrapper, 102: child}[pid],
                handling_stop=nullcontext, deferred_stop=lambda **_: nullcontext(),
                members=lambda _: [wrapper] if fault != 'descendants' else [wrapper, child])
            worker = types.SimpleNamespace(process=types.SimpleNamespace(pid=102),
                finish=lambda: events.append('finish'), abort=lambda: events.append('abort'))

            def held(path, expected, limit, machine):
                events.append(('held', path.name, machine))
                if fault == 'held':
                    raise ValueError('held rejected')
                raw = path.read_bytes()
                descriptor = os.open(path, os.O_RDONLY)
                return descriptor, os.fstat(descriptor), raw

            core = types.SimpleNamespace(held_file=held,
                identity=lambda value: (value.st_dev, value.st_ino, value.st_size),
                Worker=lambda *_: events.append('spawn') or worker)

            def verify(*_):
                events.append('parent-live')
                if fault == 'parent':
                    raise ValueError('parent gone')

            def run(*args):
                for _ in range(32 if fault != 'short-schedule' else 30):
                    args[-1]()
                if fault == 'sample':
                    raise ValueError('corrupt sample')
                return {'samples': [], 'inputs_before': {}, 'inputs_after': {}}

            def save(path, value):
                path.write_text(json.dumps(value))

            def entry_gate(*_):
                events.append('entry-gate')
                if fault == 'parent-identity':
                    raise ValueError('wrong parent')

            component = types.SimpleNamespace(HELPER_SHA256=c.PINS['frozen/probe.py'],
                LIFECYCLE_SHA256=c.PINS['measurement/native_lifecycle.py'],
                load_frozen=lambda *_: events.append('helper-pin') or core,
                entry_identity=entry_gate, verify_supervisor=verify,
                fixtures=types.SimpleNamespace(make_fixture=lambda: events.append('fixture') or object()),
                run_component=run, save=save)
            admission = types.SimpleNamespace(
                activity=types.SimpleNamespace(identity=lambda *_: {'pid': 101}),
                check=lambda *args: events.append(('admission', args)) or {'accepted': fault != 'admission'})
            evidence = types.SimpleNamespace(admitted_executables=lambda _: {'bound': True},
                endpoint=lambda *_, after_request=False: {'after': after_request},
                cpu_cost=lambda *_: {'roles': {role: {'cpu_seconds_per_token': 1, 'cpu_seconds': 1}
                                               for role in ('controller', 'worker')}})
            def stable(*_):
                events.append('source-recheck')
                if fault == 'source':
                    raise ValueError('source changed')
            stack.enter_context(patch.multiple(entry.os, getpid=lambda: 101, getppid=lambda: 100,
                getpgrp=lambda: 102 if fault == 'entry-session' else 101, getsid=lambda _: 101,
                pidfd_open=lambda *_: os.dup(fd), umask=lambda _: events.append('umask')))
            stack.enter_context(patch.object(entry.signal, 'getsignal', return_value=signal.SIG_DFL))
            stack.enter_context(patch.object(entry, 'owned_placement', return_value={'bound': True}))
            stack.enter_context(patch.object(entry.c, 'verify_files', side_effect=stable))
            driver = types.SimpleNamespace(run_component=lambda *args: run(*args),
                fixtures=types.SimpleNamespace(build_fixture=lambda: events.append('fixture') or object()))
            stack.enter_context(patch.object(entry.c, 'load_ordered', return_value=(driver, object())))
            try:
                value = entry.execute(root, plan, types.SimpleNamespace(save_new=save), admission,
                                      component, output, lifecycle, evidence)
            except ValueError:
                self.assertIsNotNone(fault)
                self.assertFalse((output / 'completion.json').exists())
                return events
            self.assertIsNone(fault)
            self.assertEqual(value, 0)
            self.assertTrue((output / 'completion.json').exists())
            observed = json.loads((output / 'process-endpoints.json').read_text())
            self.assertNotIn('cpu_seconds_per_token', observed['cpu_cost']['roles']['worker'])
            self.assertEqual(len(observed['admissions']), 2)
            return events

    def test_actual_entry_retains_all_success_gates(self):
        events = self.exercise()
        self.assertEqual(events[0], 'umask')
        for value in ('helper-pin', 'entry-gate', 'parent-live', 'fixture', 'spawn', 'finish', 'source-recheck'):
            self.assertIn(value, events)
        self.assertEqual([row for row in events if isinstance(row, tuple) and row[0] == 'held'],
                         [('held', 'worker', 62), ('held', 'v5', 224), ('held', 'm2', 224)])
        self.assertNotIn('abort', events)

    def test_entry_rejects_before_worker_launch(self):
        for fault in ('entry-session', 'parent-identity', 'parent', 'held'):
            with self.subTest(fault=fault):
                self.assertNotIn('spawn', self.exercise(fault=fault))

    def test_worker_failure_abort_and_no_completion(self):
        for fault in ('worker-group', 'admission', 'sample', 'short-schedule'):
            with self.subTest(fault=fault):
                self.assertIn('abort', self.exercise(fault=fault))

    def test_postfinish_failure_never_accepts(self):
        for fault in ('descendants', 'source'):
            with self.subTest(fault=fault):
                events = self.exercise(fault=fault)
                self.assertIn('finish', events)
                self.assertNotIn('abort', events)


class CompletionTests(unittest.TestCase):
    def test_outer_refuses_false_postflight_return(self):
        result = {'status': 0, 'cleanup_ok': True, 'child_reaped': True, 'owned_group_absent': True,
                  'term_sent': False, 'kill_sent': False, 'errors': []}
        admission = types.SimpleNamespace(check=lambda _: {'accepted': False})
        outer.clean_completion(result, admission, Path('/unused'), {}, Path('/unused/plan'), '0' * 64, Path('/unused'))
        self.assertEqual(result['status'], 125)
        self.assertIn('positive idle postflight', result['errors'][0])

    def test_outer_refuses_failed_postflight_even_if_inner_was_successful(self):
        result = {'status': 0, 'cleanup_ok': True, 'child_reaped': True, 'owned_group_absent': True,
                  'term_sent': False, 'kill_sent': False, 'errors': []}
        admission = types.SimpleNamespace(check=lambda _: (_ for _ in ()).throw(ValueError('foreign GPU')))
        outer.clean_completion(result, admission, Path('/unused'), {}, Path('/unused/plan'), '0' * 64, Path('/unused'))
        self.assertEqual(result['status'], 125)
        self.assertFalse(result['cleanup_ok'])
        self.assertIn('foreign GPU', result['errors'][0])

    def test_signaled_or_unreaped_outer_cannot_accept(self):
        for key, value in (('term_sent', True), ('kill_sent', True), ('child_reaped', False),
                           ('owned_group_absent', False), ('errors', ['retirement error'])):
            result = {'status': 0, 'cleanup_ok': True, 'child_reaped': True, 'owned_group_absent': True,
                      'term_sent': False, 'kill_sent': False, 'errors': []}
            result[key] = value
            admission = types.SimpleNamespace(check=lambda _: {'accepted': True})
            with patch.object(outer.c, 'read', return_value=(b'{}', '0' * 64)), \
                    patch.object(outer.c, 'verify_files'):
                outer.clean_completion(result, admission, Path('/unused'), {'files': {}},
                                       Path('/unused/plan'), '0' * 64, Path('/unused'))
            self.assertEqual(result['status'], 125)



class LoaderTests(unittest.TestCase):
    def test_loader_restores_existing_modules_and_keeps_distinct_fixtures(self):
        original = {name: sys.modules.get(name) for name in ('fixtures', 'slice_views')}
        old = c.load_component(D)
        sources = {name: hashlib.sha256((D / name).read_bytes()).hexdigest()
                   for name in ('fixtures.py', 'slice_views.py', 'm32_component.py', 'ordered_dispatch.py')}
        driver, _ = c.load_ordered(D, {'sources': sources})
        self.assertIsNot(old.fixtures, driver.fixtures)
        self.assertTrue(hasattr(driver.fixtures, 'build_fixture'))
        self.assertTrue(hasattr(old.fixtures, 'make_fixture'))
        for name, value in original.items():
            self.assertIs(sys.modules.get(name), value)

    def test_partial_import_failure_restores_identity_and_absence(self):
        old = object()
        with patch.dict(sys.modules, {'fixtures': old}):
            absent = 'ferric_test_absent'
            self.assertNotIn(absent, sys.modules)
            def fake_read(path, *_):
                if path.name == 'bad.py':
                    return b'raise ValueError("injected")', '0' * 64
                return b'value = 1', '0' * 64
            with patch.object(c, 'read', side_effect=fake_read):
                with self.assertRaisesRegex(ValueError, 'injected'):
                    c.load_local(D, {'fixtures': ('good.py', '0' * 64), absent: ('bad.py', '0' * 64)})
            self.assertIs(sys.modules['fixtures'], old)
            self.assertNotIn(absent, sys.modules)


class PhaseTests(unittest.TestCase):
    def fixture(self):
        stage = Path('/dev/shm/ferric-prefill-m32-component-latency-fixture')
        sources = {name: hashlib.sha256((D / name).read_bytes()).hexdigest()
                   for name in set(c.PINS) | set(c.OWN_SOURCES)}
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode='w:gz') as packed:
            for name in sorted(sources):
                raw = (D / name).read_bytes()
                info = tarfile.TarInfo(name)
                info.size = len(raw)
                packed.addfile(info, io.BytesIO(raw))
        archive = buffer.getvalue()
        tests = c.qualified_test_lines(archive, sources)
        inner_argv = ['/usr/bin/python3', '-I', '-B', '-m', 'unittest', 'discover',
                      '-s', str(c.CPU_SOURCE), '-p', 'test_*.py', '-v']
        result = {'argv': ['/bin/bash', str(c.CPU_STAGE / 'owner/cpu-env-40g-emitter.sh'),
                  '/usr/bin/python3', '-I', '-B', str(c.CPU_HELPER), hashlib.sha256(archive).hexdigest()],
                  'cwd': str(c.CPU_STAGE), 'status': 0, 'reason': 'completed', 'returncode': 0,
                  'cleanup_ok': True, 'child_reaped': True, 'errors': [], 'term_sent': False,
                  'kill_sent': False, 'log_limit_exceeded': False, 'profile': c.CPU_PROFILE,
                  'limits': dict(c.CPU_LIMITS), 'cpus': [0, 1, 2, 3], 'nice': 19,
                  'build_jobs': 4, 'rust_test_threads': 1}
        inner = {'schema': 'FerricM32HarnessQualificationV1', 'accepted': True, 'returncode': 0,
            'argv': inner_argv, 'source_root': str(c.CPU_SOURCE),
            'source_archive_sha256': hashlib.sha256(archive).hexdigest(), 'helper_sha256': c.QUALIFIER_SHA,
            'source_before': sources, 'source_after': sources, 'tests_expected': len(tests),
            'native_executed': False, 'native_qualified': False,
            'stage_before_bytes': 1, 'stage_after_bytes': 2, 'planning_increment_bytes': 64 * 1024**2}
        raw = {'status': b'0\n', 'result': c.encoded(result), 'stdout': b'',
            'stderr': ('\n'.join(tests) + '\n\n' + '-' * 70 + '\nRan ' + str(len(tests)) +
                       ' tests in 0.001s\n\nOK\n').encode(),
            'source_archive': archive, 'helper': (D / 'qualify_source.py').read_bytes(), 'inner': c.encoded(inner)}
        phase, files = {}, {}
        for name, content in raw.items():
            relative = 'qualification/harness/' + name
            phase[name] = {'path': str(stage / relative), 'sha256': hashlib.sha256(content).hexdigest()}
            files[relative] = {'sha256': phase[name]['sha256'], 'bytes': len(content), 'mode': 0o600}
        return phase, {'stage': str(stage), 'files': files, 'sources': sources}, raw

    def check(self, phase, plan, raw):
        with patch.object(c, 'read', side_effect=lambda path, *_, **__: (raw[Path(path).name], '0' * 64)), \
                patch.object(c, 'bound', side_effect=lambda item: c.decode(raw[Path(item['path']).name])):
            c.validate_phase(phase, plan)

    def test_full_unified_phase_and_complete_roster(self):
        self.check(*self.fixture())

    def test_wrong_profile_command_or_stage_growth_rejects(self):
        for field, value in (('profile', 'FerricCpuFourCore36GiBEmitterV1'),
                             ('argv', ['/bin/true']), ('cleanup_ok', False), ('term_sent', True)):
            phase, plan, raw = self.fixture()
            obj = c.decode(raw['result'])
            obj[field] = value
            raw['result'] = c.encoded(obj)
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.check(phase, plan, raw)
        phase, plan, raw = self.fixture()
        obj = c.decode(raw['inner'])
        obj['stage_after_bytes'] = c.CPU_LIMITS['stage_bytes']
        raw['inner'] = c.encoded(obj)
        with self.assertRaises(ValueError):
            self.check(phase, plan, raw)

    def test_stale_source_archive_test_roster_or_schema_rejects(self):
        for fault in ('source', 'archive', 'roster', 'schema', 'helper'):
            phase, plan, raw = self.fixture()
            if fault == 'source':
                plan['sources']['component_entry.py'] = '0' * 64
            elif fault == 'archive':
                phase['source_archive']['sha256'] = '0' * 64
            elif fault == 'roster':
                raw['stderr'] = raw['stderr'].replace(b' ... ok\n', b' ... skipped\n', 1)
            elif fault == 'schema':
                obj = c.decode(raw['inner'])
                obj['schema'] = 'FerricPairedComponentSourceCustodyV1'
                raw['inner'] = c.encoded(obj)
            else:
                phase['helper']['sha256'] = '0' * 64
            with self.subTest(fault=fault), self.assertRaises(ValueError):
                self.check(phase, plan, raw)

    def test_real_staged_zero_stdout_is_only_empty_exception(self):
        phase, plan, raw = self.fixture()
        with tempfile.TemporaryDirectory() as temporary:
            stage = Path(temporary)
            plan['stage'], plan['files'] = str(stage), {}
            for name, content in raw.items():
                relative = 'qualification/harness/' + name
                path = stage / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
                path.chmod(0o600)
                phase[name] = {'path': str(path), 'sha256': hashlib.sha256(content).hexdigest()}
                plan['files'][relative] = {'sha256': phase[name]['sha256'], 'bytes': len(content), 'mode': 0o600}
            c.verify_files(stage, plan['files'])
            c.validate_phase(phase, plan)
            path = stage / 'worker-candidate'
            path.write_bytes(b'')
            path.chmod(0o600)
            plan['files']['worker-candidate'] = {'sha256': hashlib.sha256(b'').hexdigest(),
                                                'bytes': 0, 'mode': 0o600}
            with self.assertRaises(ValueError):
                c.verify_files(stage, plan['files'])


class ReviewTests(unittest.TestCase):
    def fixture(self):
        sources = {'runtime_binding.py': '7' * 64, 'm32_provenance.py': '8' * 64}
        plan = {'stage': '/dev/shm/ferric-prefill-m32-component-latency-test', 'sources': sources,
                'worker': {'sha256': '5' * 64}, 'mode': 'latency'}
        review = {'schema': 'FerricM32ComponentReviewedInputsV1', 'engineering_only': True,
            'worker_sha256': '5' * 64, 'source_hashes': sources, 'cpu_phase': {}, 'mode': 'latency',
            'runtime_evidence': {}, 'image_evidence': {}, 'control_compiler_matches_candidate': False}
        return review, plan

    def check(self, review, plan):
        validator = types.SimpleNamespace(validate_review=lambda *_, **__: None)
        with patch.object(c, 'validate_phase') as cpu, patch.object(c, 'module', return_value=validator) as image:
            c.validate_review(review, plan)
        self.assertEqual(cpu.call_count, 1)
        self.assertEqual(image.call_count, 2)

    def test_fresh_phase_and_independent_provenance_required(self):
        self.check(*self.fixture())
        for key, value in (('control_compiler_matches_candidate', True), ('mode', 'ticks'),
                           ('worker_sha256', '0' * 64), ('source_hashes', {})):
            review, plan = self.fixture()
            review[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.check(review, plan)
        review, plan = self.fixture()
        del review['image_evidence']
        with self.assertRaises(ValueError):
            self.check(review, plan)

    def test_review_cannot_escape_stage(self):
        with self.assertRaises(ValueError):
            c.staged_binding({'path': '/tmp/escape', 'sha256': '0' * 64}, {**self.fixture()[1], 'files': {}})
