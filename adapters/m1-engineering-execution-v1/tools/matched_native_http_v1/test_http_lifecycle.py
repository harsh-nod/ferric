import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import types
import unittest
from unittest import mock

ROOT = Path(__file__).parent


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


m = load('private_http_lifecycle_tests', ROOT / 'http_lifecycle.py')
life = load('qualified_http_lifecycle_tests', ROOT / 'frozen/native_lifecycle.py')
serving = load('frozen_http_serving_tests', ROOT / 'frozen/serve_ferric.py')


class BackendOwnershipTests(unittest.TestCase):
    def test_teardown_success_is_not_normal_completion(self):
        receipt = {'cleanup_ok': True, 'child_reaped': True, 'owned_descendants_absent': True,
            'threads_joined': True, 'closed_receipt': True, 'returncode': 0,
            'signals': [], 'errors': [], 'privileged_members_observed': []}
        self.assertTrue(m.normal_completion(receipt))
        for key, value in (('closed_receipt', False), ('returncode', -15),
                           ('signals', [{'signal': 15}]), ('errors', ['failed']),
                           ('child_reaped', 1), ('privileged_members_observed', [{'pid': 10}])):
            self.assertFalse(m.normal_completion({**receipt, key: value}))

    def test_backend_never_signals_wrapper_group(self):
        backend = m.backend_class(serving, life, lambda *_: None).__new__(
            m.backend_class(serving, life, lambda *_: None))
        with mock.patch.object(os, 'killpg') as signal_group:
            with self.assertRaisesRegex(ValueError, 'cannot signal'):
                backend._signal_group(signal.SIGKILL)
            signal_group.assert_not_called()

    def test_privileged_helper_is_retained_not_signaled(self):
        cls = m.backend_class(serving, life, lambda *_: None)
        backend = cls.__new__(cls)
        backend.wrapper = {'pid': 10, 'uid': 1000}
        ordinary = {'pid': 11, 'uid': 1000}
        privileged = {'pid': 12, 'uid': 0}
        receipt = {'signals': [], 'privileged_members_observed': []}
        with mock.patch.object(backend, '_members', return_value=[ordinary, privileged]), \
             mock.patch.object(life, 'signal_member') as send:
            backend._signal_members(signal.SIGTERM, receipt)
        send.assert_called_once_with(ordinary, backend.wrapper, signal.SIGTERM)
        self.assertEqual(receipt['privileged_members_observed'], [privileged])

    def test_identity_refusal_is_not_ignored(self):
        cls = m.backend_class(serving, life, lambda *_: None)
        backend = cls.__new__(cls)
        backend.wrapper = {'pid': 10, 'uid': 1000}
        receipt = {'signals': [], 'privileged_members_observed': []}
        with mock.patch.object(backend, '_members', return_value=[{'pid': 11, 'uid': 1000}]), \
             mock.patch.object(life, 'signal_member', side_effect=ValueError('PID reused')):
            with self.assertRaisesRegex(ValueError, 'PID reused'):
                backend._signal_members(signal.SIGKILL, receipt)
        self.assertEqual(receipt['signals'], [])

    def test_running_does_not_poll_or_reap(self):
        cls = m.backend_class(serving, life, lambda *_: None)
        backend = cls.__new__(cls)
        backend.process, backend.anchor, backend.reaped = mock.Mock(), {'pid': 10}, False
        with mock.patch.object(life, 'unreaped', return_value=None):
            self.assertTrue(backend.running())
        backend.process.poll.assert_not_called()
        backend.process.wait.assert_not_called()
        backend.reaped = True
        with self.assertRaises(ValueError):
            backend.running()


FIXTURE = r'''import importlib.util, json, os, pathlib, signal, sys, time, types
def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec); sys.modules[name] = value
    spec.loader.exec_module(value); return value
root, output, mode = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3]
life = load('fixture_life', root/'frozen/native_lifecycle.py')
serving = load('fixture_serving', root/'frozen/serve_ferric.py')
http = load('fixture_private', root/'http_lifecycle.py')
def save(path, value): path.write_text(json.dumps(value))
owner = types.SimpleNamespace(backend=None)
original = http.subprocess.Popen
thread_start = http.threading.Thread.start
started_threads = []
def start_thread(thread):
    started_threads.append(thread.name)
    if len(started_threads) == 2: raise RuntimeError('injected second-thread start failure')
    return thread_start(thread)
def spawn(*args, **kwargs):
    child = original(*args, **kwargs)
    os.kill(os.getpid(), signal.SIGTERM)
    return child
try:
    with life.handling_stop():
        if mode == 'startup': http.subprocess.Popen = spawn
        if mode == 'thread-start': http.threading.Thread.start = start_thread
        config = serving.Config(argv=(sys.executable, '-I', '-B', '-c', 'import time; time.sleep(30)'),
            ready_timeout_seconds=10, request_timeout_seconds=1, shutdown_timeout_seconds=0.1)
        backend = http.backend_class(serving, life, save)(config, owner=owner, output=output)
        (output/'ready').write_text('ready')
        if mode == 'hung':
            def late_child(_number, _frame):
                original([sys.executable, '-I', '-B', '-c', 'import time; time.sleep(30)'])
                signal.signal(signal.SIGTERM, signal.SIG_IGN)
            signal.signal(signal.SIGTERM, late_child)
            time.sleep(30)
        else: backend.wait_ready()
except life.StopRequested:
    pass
except RuntimeError:
    if mode != 'thread-start': raise
finally:
    if owner.backend is not None: owner.backend.close()
'''


class RealCpuProcessTests(unittest.TestCase):
    """Only short Python fake controllers. No model, GPU or network operation."""
    def run_stop(self, mode):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            script = output / 'fixture.py'
            script.write_text(FIXTURE)
            child = subprocess.Popen([sys.executable, '-I', '-B', str(script), str(ROOT), str(output), mode],
                start_new_session=True, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            reaped = False
            try:
                if mode == 'ready':
                    deadline = time.monotonic() + 5
                    while not (output / 'ready').exists():
                        self.assertLess(time.monotonic(), deadline)
                        time.sleep(0.01)
                    os.kill(child.pid, signal.SIGTERM)
                deadline = time.monotonic() + 10
                while os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT) is None:
                    self.assertLess(time.monotonic(), deadline, 'HTTP fake wrapper did not retire')
                    time.sleep(0.01)
                receipt = json.loads((output / 'http-controller-cleanup.json').read_text())
                self.assertTrue(receipt['cleanup_ok'], receipt)
                self.assertTrue(receipt['child_reaped'])
                self.assertTrue(receipt['owned_descendants_absent'])
                self.assertTrue(receipt['threads_joined'])
            finally:
                if not reaped:
                    os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                    try:
                        os.killpg(child.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    child.wait(timeout=5)
                    reaped = True
                for stream in (child.stdout, child.stderr):
                    stream.close()
            self.assertEqual(child.returncode, 0)

    def test_term_during_http_backend_constructor(self):
        self.run_stop('startup')

    def test_second_backend_thread_start_failure_reaps_controller(self):
        self.run_stop('thread-start')

    def test_term_during_ready_wait_and_blocked_reader(self):
        self.run_stop('ready')

    def test_outer_kill_contains_http_controller_and_late_child(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            script = output / 'fixture.py'
            script.write_text(FIXTURE)
            def save(path, value): path.write_text(json.dumps(value))
            base = types.SimpleNamespace(timestamp=lambda: 'fixture', save=save, require_facilities=lambda: None)
            receipt = life.supervise_group(base,
                [sys.executable, '-I', '-B', str(script), str(ROOT), str(output), 'hung'], output,
                lambda _initial: {'accepted': True}, duration=0.7, grace=0.3, kill_wait=3, interval=0.1)
            try:
                self.assertTrue(receipt['kill_sent'], receipt)
                self.assertTrue(receipt['child_reaped'], receipt)
                self.assertTrue(receipt['owned_group_absent'], receipt)
                self.assertFalse(receipt['cleanup_ok'])
            finally:
                if not receipt['child_reaped']:
                    pid = receipt['child_pid']
                    os.waitid(os.P_PID, pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                    os.killpg(pid, signal.SIGKILL)
                    os.waitpid(pid, 0)


if __name__ == '__main__':
    unittest.main()
