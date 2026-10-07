import importlib.util
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import tempfile
import time
import types
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location('lifecycle_fixture', Path(__file__).with_name('native_lifecycle.py'))
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)


def row(pid, parent=1, group=None, start=None, uid=1000):
    return {'pid': pid, 'parent': parent, 'group': group or pid, 'session': group or pid,
            'start': start or pid * 100, 'uid': uid}


class StopTests(unittest.TestCase):
    def test_stop_is_not_io_error(self):
        self.assertFalse(issubclass(m.StopRequested, OSError))

    def test_selector_swallows_old_handler_error(self):
        with selectors.PollSelector() as selector:
            with mock.patch.object(selector, '_selector') as poll:
                poll.poll.side_effect = InterruptedError('old signal handler')
                self.assertEqual(selector.select(0), [])

    def test_selector_propagates_new_stop(self):
        with selectors.PollSelector() as selector:
            with mock.patch.object(selector, '_selector') as poll:
                poll.poll.side_effect = m.StopRequested('new handler')
                with self.assertRaises(m.StopRequested):
                    selector.select(0)

    def test_installed_handler_restored(self):
        previous = signal.getsignal(signal.SIGTERM)
        with m.handling_stop():
            with self.assertRaises(m.StopRequested):
                signal.raise_signal(signal.SIGTERM)
        self.assertEqual(signal.getsignal(signal.SIGTERM), previous)

    def test_startup_defers_until_assignment(self):
        assigned = []
        with self.assertRaises(m.StopRequested), m.handling_stop():
            with m.deferred_stop():
                signal.raise_signal(signal.SIGTERM)
                assigned.append('owned')
        self.assertEqual(assigned, ['owned'])

    def test_cleanup_defers_all_stop_signals(self):
        with m.handling_stop(), m.deferred_stop(deliver=False) as observed:
            for number in m.STOP_SIGNALS:
                signal.raise_signal(number)
        self.assertEqual(observed, list(m.STOP_SIGNALS))


class IdentityTests(unittest.TestCase):
    def test_reused_member_is_not_signaled(self):
        wrapper, child = row(10), row(11, 10, 10)
        replacement = dict(child, start=999)
        with mock.patch.object(m, 'process', side_effect=[wrapper, replacement]), \
             mock.patch.object(m.os, 'pidfd_open', return_value=99), \
             mock.patch.object(m.os, 'close') as close, \
             mock.patch.object(m.signal, 'pidfd_send_signal') as send:
            with self.assertRaisesRegex(ValueError, 'lifetime changed'):
                m.signal_member(child, wrapper, signal.SIGKILL)
            send.assert_not_called()
            close.assert_called_once_with(99)

    def test_foreign_group_uid_and_wrapper_never_signaled(self):
        wrapper = row(10)
        for candidate in (wrapper, row(11, group=20), row(11, group=10, uid=7)):
            with self.subTest(candidate=candidate), mock.patch.object(m.os, 'pidfd_open') as opened:
                with self.assertRaises(ValueError):
                    m.signal_member(candidate, wrapper, signal.SIGTERM)
                opened.assert_not_called()

    def test_reaped_child_refuses_identity_and_signal(self):
        proc = types.SimpleNamespace(pid=11)
        with mock.patch.object(m.os, 'waitid', side_effect=ChildProcessError()), \
             mock.patch.object(m, 'process') as identity:
            with self.assertRaises(ChildProcessError):
                m.unreaped(proc, row(11))
            identity.assert_not_called()

    def test_unreaped_identity_drift_refuses(self):
        with mock.patch.object(m.os, 'waitid', return_value=None), \
             mock.patch.object(m, 'process', return_value=row(11, start=999)):
            with self.assertRaisesRegex(ValueError, 'lifetime/credentials'):
                m.unreaped(types.SimpleNamespace(pid=11), row(11))


class ControllerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.wrapper, self.child, self.worker = row(10), row(11, 10, 10), row(12, 11, 10)
        self.proc = mock.Mock(pid=11, returncode=0)
        self.proc.wait.return_value = 0
        self.runner = types.SimpleNamespace(Controller=object, save=mock.Mock())
        self.patches = [mock.patch.object(m.os, 'getpid', return_value=10),
            mock.patch.object(m, 'process', side_effect=lambda pid: self.wrapper if pid == 10 else self.child),
            mock.patch.object(m, 'unreaped', return_value=None),
            mock.patch.object(m, 'members', return_value=[self.wrapper, self.child]),
            mock.patch.object(m.os, 'set_blocking'), mock.patch.object(m.selectors, 'DefaultSelector'),
            mock.patch.object(m.subprocess, 'Popen', return_value=self.proc),
            mock.patch.object(m.time, 'sleep'), mock.patch.object(m, 'signal_member')]
        self.mocks = [patch.start() for patch in self.patches]
        for patch in reversed(self.patches):
            self.addCleanup(patch.stop)

    def make(self):
        return m.controller_class(self.runner)(['fake-controller'], self.root, 100)

    def test_constructor_shares_wrapper_group(self):
        controller = self.make()
        self.assertFalse(m.subprocess.Popen.call_args.kwargs['start_new_session'])
        controller.close()

    def test_term_during_popen_preserves_assigned_controller(self):
        def spawn(*_args, **_kwargs):
            signal.raise_signal(signal.SIGTERM)
            return self.proc
        m.subprocess.Popen.side_effect = spawn
        controller = None
        with self.assertRaises(m.StopRequested), m.handling_stop():
            with m.deferred_stop():
                controller = self.make()
        self.assertIsNotNone(controller)
        controller.close()
        self.assertTrue(controller.reaped)

    def test_clean_close_is_unsignaled_and_idempotent(self):
        controller = self.make()
        controller.clean_exit = True
        m.unreaped.return_value = types.SimpleNamespace(si_code=m.os.CLD_EXITED, si_status=0)
        m.members.side_effect = [[self.wrapper, self.child], [self.wrapper]]
        cleanup = controller.close()
        self.assertTrue(cleanup['cleanup_ok'])
        self.assertIs(controller.close(), cleanup)
        m.signal_member.assert_not_called()
        self.proc.wait.assert_called_once()

    def test_failure_close_signals_only_children_not_wrapper(self):
        controller = self.make()
        m.members.side_effect = [[self.wrapper, self.child], [self.wrapper, self.child], [self.wrapper]]
        self.assertTrue(controller.close()['cleanup_ok'])
        self.assertEqual([call.args[0]['pid'] for call in m.signal_member.call_args_list], [11, 11])
        self.assertEqual([call.args[2] for call in m.signal_member.call_args_list], [signal.SIGTERM, signal.SIGKILL])

    def test_late_worker_is_included_in_fresh_kill_pass(self):
        controller = self.make()
        m.members.side_effect = [[self.wrapper, self.child],
                                [self.wrapper, self.child, self.worker], [self.wrapper]]
        self.assertTrue(controller.close()['cleanup_ok'])
        self.assertEqual([call.args[0]['pid'] for call in m.signal_member.call_args_list], [11, 11, 12])
        self.assertEqual(m.signal_member.call_args_list[-1].args[2], signal.SIGKILL)

    def test_missing_lifetime_does_not_signal(self):
        controller = self.make()
        m.unreaped.side_effect = ChildProcessError('unexpected reap')
        cleanup = controller.close()
        self.assertFalse(cleanup['cleanup_ok'])
        m.signal_member.assert_not_called()
        self.proc.wait.assert_not_called()

    def test_cleanup_signal_does_not_abort_cleanup(self):
        controller = self.make()
        def during_cleanup(*_args):
            signal.raise_signal(signal.SIGHUP)
        m.signal_member.side_effect = during_cleanup
        m.members.side_effect = [[self.wrapper, self.child], [self.wrapper, self.child], [self.wrapper]]
        self.assertTrue(controller.close()['cleanup_ok'])

    def test_privileged_owned_helper_does_not_block_gpu_child_signals(self):
        controller = self.make()
        helper = row(15, 10, 10, uid=0)
        m.members.side_effect = [[self.wrapper, self.child, helper],
                                [self.wrapper, self.child, helper], [self.wrapper, helper]]
        cleanup = controller.close()
        self.assertFalse(cleanup['cleanup_ok'])
        self.assertTrue(cleanup['child_reaped'])
        self.assertEqual([call.args[0]['pid'] for call in m.signal_member.call_args_list], [11, 11])


class SupervisorTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.wrapper, self.worker = row(10, parent=1), row(12, parent=11, group=10)
        self.proc = mock.Mock(pid=10)
        self.proc.wait.return_value = 0
        self.base = types.SimpleNamespace(timestamp=lambda: 'fixed', require_facilities=mock.Mock(), save=mock.Mock())
        self.status = types.SimpleNamespace(si_code=m.os.CLD_EXITED, si_status=0)
        self.patches = [mock.patch.object(m.os, 'getpid', return_value=1),
            mock.patch.object(m.os, 'getuid', return_value=1000),
            mock.patch.object(m, 'process', return_value=self.wrapper),
            mock.patch.object(m, 'unreaped', return_value=self.status),
            mock.patch.object(m, 'members', return_value=[self.wrapper]),
            mock.patch.object(m.subprocess, 'Popen', return_value=self.proc),
            mock.patch.object(m.os, 'pidfd_open', return_value=99), mock.patch.object(m.os, 'close'),
            mock.patch.object(m.os, 'killpg'), mock.patch.object(m.time, 'sleep')]
        for patch in self.patches:
            patch.start()
            self.addCleanup(patch.stop)

    def run_supervisor(self, check=lambda _initial: {'accepted': True}, **kwargs):
        return m.supervise_group(self.base, ['fake-wrapper'], self.root, check, grace=0, kill_wait=0, **kwargs)

    def test_normal_exit_never_signals(self):
        receipt = self.run_supervisor()
        self.assertTrue(receipt['cleanup_ok'])
        self.assertEqual(receipt['status'], 0)
        m.os.killpg.assert_not_called()
        self.proc.wait.assert_called_once()

    def test_leftover_worker_after_wrapper_exit_is_killed_before_reap(self):
        m.members.side_effect = [[self.wrapper, self.worker], [self.wrapper, self.worker], [self.wrapper]]
        receipt = self.run_supervisor()
        self.assertTrue(receipt['child_reaped'])
        self.assertFalse(receipt['cleanup_ok'])
        self.assertEqual(m.os.killpg.call_args_list, [mock.call(10, signal.SIGTERM), mock.call(10, signal.SIGKILL)])

    def test_stop_before_launch_creates_no_child(self):
        dead = [False]
        def observe(*_args):
            return self.status if dead[0] else None
        def group_signal(_pid, number):
            self.proc.wait.assert_not_called()
            if number == signal.SIGKILL:
                dead[0] = True
        m.unreaped.side_effect = observe
        m.os.killpg.side_effect = group_signal
        m.members.side_effect = lambda _anchor: [self.wrapper] if dead[0] else [self.wrapper, self.worker]
        receipt = self.run_supervisor(stop_signal=lambda: signal.SIGTERM)
        # The prelaunch stop must not launch a child.
        self.assertFalse(receipt['child_reaped'])
        m.os.killpg.assert_not_called()

    def test_live_hung_wrapper_fallback_after_admission_failure(self):
        dead = [False]
        m.unreaped.side_effect = lambda *_args: self.status if dead[0] else None
        def group_signal(_pid, number):
            self.proc.wait.assert_not_called()
            if number == signal.SIGKILL:
                dead[0] = True
        m.os.killpg.side_effect = group_signal
        m.members.side_effect = lambda _anchor: [self.wrapper] if dead[0] else [self.wrapper, self.worker]
        def check(initial):
            if not initial:
                raise ValueError('forced monitor refusal')
            return {'accepted': True}
        receipt = self.run_supervisor(check)
        self.assertTrue(receipt['child_reaped'])
        self.assertTrue(receipt['kill_sent'])
        self.assertEqual(receipt['termination_reason'], 'resource_or_foreign_kfd')

    def test_foreign_replacement_prevents_group_signal(self):
        m.unreaped.side_effect = ValueError('lifetime drift')
        receipt = self.run_supervisor()
        self.assertFalse(receipt['cleanup_ok'])
        m.os.killpg.assert_not_called()
        self.proc.wait.assert_not_called()

    def test_group_permission_failure_is_not_ignored(self):
        m.members.side_effect = PermissionError('denied')
        receipt = self.run_supervisor()
        self.assertFalse(receipt['cleanup_ok'])
        m.os.killpg.assert_not_called()
        self.proc.wait.assert_not_called()

    def test_anchor_read_failure_still_cleans_kernel_owned_session(self):
        m.process.side_effect = PermissionError('proc unavailable')
        with mock.patch.object(m.os, 'waitid', return_value=None), \
             mock.patch.object(m.os, 'getpgid', return_value=10), \
             mock.patch.object(m.os, 'getsid', return_value=10):
            receipt = self.run_supervisor()
        self.assertTrue(receipt['child_reaped'])
        self.assertFalse(receipt['cleanup_ok'])
        self.assertEqual(m.os.killpg.call_args_list, [mock.call(10, signal.SIGTERM), mock.call(10, signal.SIGKILL)])

    def test_anchor_failure_with_lost_child_ownership_never_signals(self):
        m.process.side_effect = PermissionError('proc unavailable')
        with mock.patch.object(m.os, 'waitid', side_effect=ChildProcessError()):
            receipt = self.run_supervisor()
        self.assertFalse(receipt['child_reaped'])
        m.os.killpg.assert_not_called()


class RealFakeProcessTests(unittest.TestCase):
    """CPU-only Python stand-ins, never a model, GPU binary, or shared process."""

    def run_fixture(self, mode):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            source = '''import importlib.util, json, os, pathlib, selectors, signal, subprocess, sys, time, types
spec = importlib.util.spec_from_file_location('fixture_lifecycle', sys.argv[1])
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
out = pathlib.Path(sys.argv[2]); mode = sys.argv[3]
def save(path, value): path.write_text(json.dumps(value))
runner = types.SimpleNamespace(Controller=object, save=save)
controller = None
original = m.subprocess.Popen
def spawn(*args, **kwargs):
    child = original(*args, **kwargs)
    os.kill(os.getpid(), signal.SIGTERM)
    return child
try:
    with m.handling_stop():
        if mode == 'startup': m.subprocess.Popen = spawn
        with m.deferred_stop():
            controller = m.controller_class(runner)([sys.executable, '-I', '-B', '-c',
                'import time; time.sleep(40)'], out, time.monotonic()+45)
        (out/'ready').write_text('ready')
        with selectors.DefaultSelector() as selector:
            selector.register(controller.proc.stdout, selectors.EVENT_READ)
            selector.select(30)
except m.StopRequested:
    pass
finally:
    if controller is not None: controller.close()
'''
            script = output / 'fake.py'
            script.write_text(source)
            child = subprocess.Popen([sys.executable, '-I', '-B', str(script), str(Path(m.__file__)),
                                      str(output), mode], start_new_session=True,
                                      stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                if mode == 'selector':
                    deadline = time.monotonic() + 5
                    while not (output / 'ready').exists():
                        self.assertLess(time.monotonic(), deadline, 'fake wrapper startup timed out')
                        time.sleep(0.01)
                    os.kill(child.pid, signal.SIGTERM)
                # Keep the leader unreaped until all safety signals are finished.
                deadline = time.monotonic() + 10
                while os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT) is None:
                    self.assertLess(time.monotonic(), deadline, 'fake wrapper did not stop')
                    time.sleep(0.01)
                cleanup = json.loads((output / 'cleanup.json').read_text())
                self.assertTrue(cleanup['child_reaped'])
                self.assertTrue(cleanup['owned_descendants_absent'])
                self.assertTrue(cleanup['cleanup_ok'])
            finally:
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                child.wait(timeout=5)
                for stream in (child.stdout, child.stderr):
                    stream.close()
            self.assertEqual(child.returncode, 0)

    def test_real_term_during_constructor_is_delivered_after_ownership(self):
        self.run_fixture('startup')

    def test_real_term_breaks_selector_and_reaps_controller(self):
        self.run_fixture('selector')

    def test_real_outer_kill_contains_unresponsive_wrapper_and_late_worker(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            script = output / 'hung.py'
            script.write_text('''import signal, subprocess, sys, time
def late_worker(_number, _frame):
    subprocess.Popen([sys.executable, '-I', '-B', '-c', 'import time; time.sleep(30)'])
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
signal.signal(signal.SIGTERM, late_worker)
time.sleep(30)
''')
            def save(path, value):
                path.write_text(json.dumps(value))
            base = types.SimpleNamespace(timestamp=lambda: 'fixture', require_facilities=lambda: None, save=save)
            receipt = m.supervise_group(base, [sys.executable, '-I', '-B', str(script)], output,
                lambda _initial: {'accepted': True}, duration=0.3, grace=0.3, kill_wait=3, interval=0.1)
            try:
                self.assertTrue(receipt['kill_sent'])
                self.assertTrue(receipt['child_reaped'], receipt['errors'])
                self.assertTrue(receipt['owned_group_absent'])
                self.assertFalse(receipt['cleanup_ok'])
                self.assertEqual(receipt['returncode'], -signal.SIGKILL)
            finally:
                if not receipt['child_reaped']:
                    pid = receipt['child_pid']
                    os.waitid(os.P_PID, pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                    os.killpg(pid, signal.SIGKILL)
                    os.waitpid(pid, 0)


if __name__ == '__main__':
    unittest.main()
