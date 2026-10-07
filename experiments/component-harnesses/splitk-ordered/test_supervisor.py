import ast
from contextlib import ExitStack, nullcontext
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import signal
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
        self.assertEqual([row['symbol'] for row in component.fixtures.CONTROL_KERNELS.values()], list(c.SYMBOLS['v5']))
        self.assertEqual(len(list(component.schedule())), 30)

    def test_stage_cannot_impersonate_fence_or_v14_campaign(self):
        self.assertEqual(c.stage_name('/dev/shm/ferric-v16-splitk-ordered-latency-a001').parent, Path('/dev/shm'))
        for value in ('/tmp/ferric-v16-splitk-component-a001', '/dev/shm/ferric-v16-fences-dcf3454-a001',
                      '/dev/shm/ferric-v14-native-a001', '/dev/shm/ferric-v16-splitk-component-'):
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
        for _ in range(60):
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
            for name in ('worker', 'v5', 'candidate', 'python', 'source'):
                path = root / name
                path.write_bytes(name.encode())
                paths[name] = {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
            plan = {'device_unique_id': c.DEVICE, 'worker': paths['worker'], 'python': paths['python'], 'mode': 'latency',
                    'images': {'v5': paths['v5'], 'candidate': paths['candidate']},
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
                for _ in range(60 if fault != 'short-schedule' else 58):
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
            driver = types.SimpleNamespace(run_component=lambda *args: run(*args))
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
                         [('held', 'worker', 62), ('held', 'v5', 224), ('held', 'candidate', 224)])
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


class ReviewTests(unittest.TestCase):
    def fixture(self):
        stage = Path('/dev/shm/ferric-v16-splitk-component-fixture')
        files, inputs = {}, {}
        def put(name, value):
            digest = hashlib.sha256(c.encoded(value)).hexdigest()
            files[name] = {'sha256': digest}
            binding = {'path': str(stage / name), 'sha256': digest}
            inputs[binding['path']] = value
            return binding
        images = {}
        for role in ('v5', 'candidate'):
            sha = ('1' if role == 'v5' else '2') * 64
            images[role] = {'sha256': sha, 'source_sha256': '3' * 64, 'emitter_sha256': '4' * 64,
                'symbols': {symbol: {'wavefront_size': 64, 'private_segment_bytes': 0, 'group_segment_bytes': 0}
                            for symbol in c.SYMBOLS[role]},
                'isa_review': put('reviews/' + role + '.json', {'schema': 'FerricSplitKSymbolIsaReviewV1',
                    'image_sha256': sha, 'accepted': True, 'symbols': list(c.SYMBOLS[role])})}
        phases = {}
        for name in ('component-tests', 'supervisor-tests'):
            phases[name] = {'status': put(name + '.status', 0), 'result': put(name + '.json', {'clean': True})}
        sources = {'runtime_binding.py': '7' * 64}
        review = {'schema': 'FerricOrderedSplitKComponentReviewedInputsV1', 'engineering_only': True,
            'worker_sha256': '5' * 64, 'images': images, 'source_hashes': sources, 'cpu_phases': phases,
            'mode': 'latency', 'runtime_evidence': {}}
        plan = {'stage': str(stage), 'files': files, 'worker': {'sha256': '5' * 64}, 'sources': sources,
                'mode': 'latency',
                'images': {role: {'sha256': row['sha256']} for role, row in images.items()}}
        return review, plan, inputs

    def check(self, review, plan, inputs):
        with patch.object(c, 'bound', side_effect=lambda value: inputs[value['path']]), \
                patch.object(c, 'read', return_value=(b'0\n', '0' * 64)), patch.object(c, 'validate_phase'), \
                patch.object(c, 'module', return_value=types.SimpleNamespace(validate_review=lambda *_, **__: None)):
            c.validate_review(review, plan)

    def test_review_binds_same_emitter_and_resource_gate(self):
        review, plan, inputs = self.fixture()
        self.check(review, plan, inputs)
        review['images']['candidate']['emitter_sha256'] = '6' * 64
        with self.assertRaises(ValueError):
            self.check(review, plan, inputs)
        review['images']['candidate']['emitter_sha256'] = '4' * 64
        review['images']['candidate']['symbols'][c.SYMBOLS['candidate'][0]]['group_segment_bytes'] = 64
        with self.assertRaises(ValueError):
            self.check(review, plan, inputs)

    def test_review_refuses_unaccepted_isa_and_missing_cpu_phase(self):
        review, plan, inputs = self.fixture()
        inputs[review['images']['v5']['isa_review']['path']]['accepted'] = False
        with self.assertRaises(ValueError):
            self.check(review, plan, inputs)
        review, plan, inputs = self.fixture()
        del review['cpu_phases']['supervisor-tests']
        with self.assertRaises(ValueError):
            self.check(review, plan, inputs)

    def test_review_bound_file_cannot_escape_stage(self):
        with self.assertRaises(ValueError):
            c.staged_binding({'path': '/tmp/unbounded-review', 'sha256': '0' * 64},
                             {'stage': '/dev/shm/ferric-v16-splitk-component-fixture', 'files': {}})

    def test_review_rejects_worker_source_or_extra_phase_drift(self):
        for change in ('worker', 'source', 'phase'):
            review, plan, inputs = self.fixture()
            if change == 'worker':
                review['worker_sha256'] = '9' * 64
            elif change == 'source':
                review['source_hashes'] = {'unexpected.py': '9' * 64}
            else:
                review['cpu_phases']['extra'] = review['cpu_phases']['component-tests']
            with self.assertRaises(ValueError):
                self.check(review, plan, inputs)


class PhaseTests(unittest.TestCase):
    def fixture(self, role):
        stage = Path('/dev/shm/ferric-v16-splitk-component-test')
        sources = {name: hashlib.sha256((D / name).read_bytes()).hexdigest()
                   for name in set(c.PINS) | set(c.OWN_SOURCES)}
        expected = c.phase_source_roster(role, sources)
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode='w:gz') as packed:
            for name in sorted(expected):
                source = D / ('frozen-tests/' + name if role == 'component-tests'
                              and name.startswith('test_') else name)
                raw = source.read_bytes()
                item = tarfile.TarInfo(name)
                item.size = len(raw)
                packed.addfile(item, io.BytesIO(raw))
        archive = buffer.getvalue()
        tests = c.qualified_test_lines(role, archive, expected)
        stderr = ('\n'.join(tests) + '\n\n' + '-' * 70 + '\nRan ' + str(len(tests)) +
                  ' tests in 0.001s\n\nOK\n').encode()
        cwd = '/tmp/ferric-v16-emitter-b95a642-r1'
        source = cwd + '/source/' + role if role == 'component-tests' else str(c.CPU_SOURCE)
        inner_argv = ['/usr/bin/python3', '-I', '-B',
                '-m', 'unittest', 'discover', '-s', source, '-p',
                'test_*.py', '-v']
        argv = (['/bin/bash', cwd + '/owner/cpu-env.sh', *inner_argv] if role == 'component-tests' else
                ['/bin/bash', cwd + '/owner/cpu-env-36g-emitter.sh', '/usr/bin/python3', '-I', '-B',
                 str(c.CPU_HELPER), 'harness', '--archive-sha256', hashlib.sha256(archive).hexdigest()])
        result = {'argv': argv, 'cwd': cwd,
                  'profile': c.G36_PROFILE if role == 'supervisor-tests' else 'FerricCpuFourCore24GiBEmitterV1'}
        raw_files = {'status': b'0\n', 'result': c.encoded(result), 'stdout': b'',
                     'stderr': stderr, 'source_archive': archive}
        if role == 'supervisor-tests':
            def allocation(allowance):
                return {'stage_allocated_bytes': 1, 'stage_cap_bytes': c.G36_LIMITS['stage_bytes'],
                    'stage_reserve_bytes': c.G36_LIMITS['stage_reserve_bytes'],
                    'remaining_reserved_bytes': c.G36_LIMITS['stage_bytes'] - c.G36_LIMITS['stage_reserve_bytes'] - 1,
                    'planned_increment_bytes': allowance}
            raw_files['helper'] = (D / 'qualify_source.py').read_bytes().replace(b'ENABLED = False', b'ENABLED = True', 1)
            raw_files['inner'] = c.encoded({'schema': 'FerricOrderedComponentSourceCustodyV1',
                'action': 'harness', 'argv': inner_argv, 'returncode': 0, 'source_root': source,
                'archive_sha256': hashlib.sha256(archive).hexdigest(), 'helper_sha256': c.QUALIFIER_SHA,
                'source_before': sources, 'source_after': sources, 'authored_test_count': len(tests),
                'native_executed': False, 'native_qualified': False,
                'allocation_before': allocation(64 * 1024**2), 'allocation_after': allocation(0)})
        phase, files, by_path = {}, {}, {}
        for key, raw in raw_files.items():
            name = 'qualification/' + role + '/' + key if key in ('stdout', 'helper') else role + '-' + key
            digest = hashlib.sha256(raw).hexdigest()
            phase[key] = {'path': str(stage / name), 'sha256': digest}
            files[name] = {'sha256': digest}
            by_path[str(stage / name)] = raw
        phase.update(source_directory=source, argv_sha256=hashlib.sha256(c.encoded(argv)).hexdigest())
        return phase, {'stage': str(stage), 'files': files, 'sources': sources}, by_path, result

    def check(self, role, phase, plan, by_path, result):
        with patch.object(c, 'read', side_effect=lambda path, *_, **__: (by_path[str(path)], '0' * 64)), \
                patch.object(c, 'bound', side_effect=lambda item: result if item == phase['result']
                             else json.loads(by_path[item['path']])), patch.object(c, 'clean_cpu'):
            c.validate_phase(role, phase, plan)

    def test_exact_role_command_roster_and_source_accept(self):
        for role in ('component-tests', 'supervisor-tests'):
            self.check(role, *self.fixture(role))

    def test_unrelated_clean_command_or_swapped_role_reject(self):
        phase, plan, raw, result = self.fixture('component-tests')
        result['argv'] = ['/bin/true']
        phase['argv_sha256'] = hashlib.sha256(c.encoded(result['argv'])).hexdigest()
        with self.assertRaises(ValueError):
            self.check('component-tests', phase, plan, raw, result)
        with self.assertRaises(ValueError):
            self.check('supervisor-tests', *self.fixture('component-tests'))

    def test_missing_test_output_or_changed_source_reject(self):
        phase, plan, raw, result = self.fixture('supervisor-tests')
        raw[phase['stderr']['path']] = raw[phase['stderr']['path']].replace(b' ... ok\n', b' ... skipped\n', 1)
        with self.assertRaises(ValueError):
            self.check('supervisor-tests', phase, plan, raw, result)
        phase, plan, raw, result = self.fixture('supervisor-tests')
        plan['sources']['component_entry.py'] = '0' * 64
        with self.assertRaises(ValueError):
            self.check('supervisor-tests', phase, plan, raw, result)

    def test_real_staged_empty_stdout_and_phase_validation(self):
        phase, plan, raw, result = self.fixture('supervisor-tests')
        result.update(status=0, reason='completed', returncode=0, cleanup_ok=True,
            child_reaped=True, errors=[], term_sent=False, kill_sent=False, log_limit_exceeded=False,
            profile=c.G36_PROFILE, limits=dict(c.G36_LIMITS),
            cpus=[0, 1, 2, 3], nice=19, build_jobs=4, rust_test_threads=1)
        raw[phase['result']['path']] = c.encoded(result)
        with tempfile.TemporaryDirectory() as temporary:
            stage = Path(temporary)
            old_stage = Path(plan['stage'])
            plan['stage'], plan['files'] = str(stage), {}
            for key in ('status', 'result', 'stdout', 'stderr', 'source_archive', 'helper', 'inner'):
                old = phase[key]['path']
                path = stage / Path(old).relative_to(old_stage)
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(raw[old])
                path.chmod(0o600)
                digest = hashlib.sha256(raw[old]).hexdigest()
                phase[key] = {'path': str(path), 'sha256': digest}
                plan['files'][str(path.relative_to(stage))] = {
                    'sha256': digest, 'bytes': len(raw[old]), 'mode': 0o600}
            c.verify_files(stage, plan['files'])
            c.validate_phase('supervisor-tests', phase, plan)
            bad = stage / 'worker-candidate'
            bad.write_bytes(b'')
            bad.chmod(0o600)
            plan['files']['worker-candidate'] = {'sha256': hashlib.sha256(b'').hexdigest(),
                                                'bytes': 0, 'mode': 0o600}
            with self.assertRaises(ValueError):
                c.verify_files(stage, plan['files'])


if __name__ == '__main__':
    unittest.main()
