"""Private HTTP controller ownership; shared serving defaults are unchanged."""
import os
from pathlib import Path
import queue
import signal
import subprocess
import threading
import time


def normal_completion(receipt):
    """Successful cleanup does not turn an interrupted request into a sample."""
    required = {'cleanup_ok': True, 'child_reaped': True, 'owned_descendants_absent': True,
                'threads_joined': True, 'closed_receipt': True, 'returncode': 0,
                'signals': [], 'errors': [], 'privileged_members_observed': []}
    return type(receipt) is dict and all(type(receipt.get(key)) is type(value)
        and receipt[key] == value for key, value in required.items())


def backend_class(serving, lifecycle, save):
    """Use the frozen HTTP protocol with a controller in the outer owned group.

    The caller must itself be the outer supervisor's unreaped session leader.
    The outer supervisor is the only code allowed to signal the whole group.
    """
    require = lifecycle.require

    class Backend(serving.Backend):
        def __init__(self, config, *, owner, output):
            self.config, self.output = config, Path(output)
            self.lock = threading.Lock()
            self.pending, self.next_id = {}, 1
            self.commands = queue.Queue(serving.COMMAND_QUEUE)
            self.ready, self.stopping = threading.Event(), threading.Event()
            self.failure = self.setup = None
            self.closed_receipt = False
            self.stderr_tail = bytearray()
            self.process, self.anchor, self.pidfd = None, None, None
            self.reaped, self.cleanup, self.threads = False, None, []
            self.wrapper = lifecycle.process(os.getpid())
            require(self.wrapper['pid'] == self.wrapper['group'] == self.wrapper['session'],
                    'HTTP wrapper must own the outer supervised session')
            require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL,
                    'unreaped direct-child ownership requires default SIGCHLD')
            # Publish ownership before construction can spawn or defer a signal.
            owner.backend = self
            try:
                with lifecycle.deferred_stop():
                    self.process = subprocess.Popen(config.argv, stdin=subprocess.PIPE,
                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=65_536,
                        start_new_session=False)
                    self.pidfd = os.pidfd_open(self.process.pid, 0)
                    self.anchor = lifecycle.process(self.process.pid)
                    require(self.anchor['parent'] == self.wrapper['pid']
                            and self.anchor['group'] == self.anchor['session'] == self.wrapper['pid']
                            and self.anchor['uid'] == self.wrapper['uid'],
                            'direct same-group HTTP controller required')
                    lifecycle.unreaped(self.process, self.anchor)
                    save(self.output / 'owned-controller.json', {
                        'schema': 'FerricNativeHttpControllerOwnershipV1',
                        'argv': list(config.argv), 'controller': self.anchor,
                        'wrapper': self.wrapper, 'start_new_session': False,
                        'pidfd_opened': True})
                    for target, name in ((self._writer, 'ferric-command-writer'),
                                         (self._reader, 'ferric-event-reader'),
                                         (self._stderr, 'ferric-stderr-reader')):
                        thread = threading.Thread(target=target, daemon=True, name=name)
                        try:
                            thread.start()
                        finally:
                            if thread.ident is not None:
                                self.threads.append(thread)
            except BaseException:
                self.close()
                raise

        def running(self):
            require(self.process is not None and not self.reaped and self.anchor is not None,
                    'unreaped bound HTTP controller required')
            return lifecycle.unreaped(self.process, self.anchor) is None

        def _signal_group(self, _number):
            raise ValueError('HTTP backend cannot signal its wrapper group')

        def _members(self):
            return [row for row in lifecycle.members(self.wrapper)
                    if row['pid'] != self.wrapper['pid']]

        def _signal_members(self, number, receipt):
            for row in self._members():
                if row['uid'] != self.wrapper['uid']:
                    receipt['privileged_members_observed'].append(row)
                    continue
                try:
                    lifecycle.signal_member(row, self.wrapper, number)
                    receipt['signals'].append({'identity': row, 'signal': int(number)})
                except ProcessLookupError:
                    pass

        def _controller_status(self):
            if self.anchor is not None:
                return lifecycle.unreaped(self.process, self.anchor)
            # A successful Popen followed by /proc failure still owns an unreaped
            # direct child. The outer session remains the cleanup authority.
            status = os.waitid(os.P_PID, self.process.pid,
                               os.WEXITED | os.WNOHANG | os.WNOWAIT)
            require(os.getpgid(self.process.pid) == os.getsid(self.process.pid)
                    == self.wrapper['pid'], 'kernel-owned child escaped wrapper session')
            return status

        def _await_child_only(self, seconds):
            deadline = time.monotonic() + seconds
            while True:
                status = self._controller_status()
                rows = self._members()
                if status is not None and [row['pid'] for row in rows] == [self.process.pid]:
                    return status
                if time.monotonic() >= deadline:
                    return None
                time.sleep(0.02)

        def close(self):
            if self.cleanup is not None:
                return self.cleanup
            receipt = {'schema': 'FerricNativeHttpControllerCleanupV1',
                'wrapper': self.wrapper, 'controller': self.anchor, 'signals': [],
                'privileged_members_observed': [], 'errors': [], 'child_reaped': False,
                'owned_descendants_absent': False, 'threads_joined': False,
                'closed_receipt': False, 'cleanup_ok': False}
            with lifecycle.deferred_stop(deliver=False):
                try:
                    self.stopping.set()
                    self._fail('adapter shutting down')
                    if self.process is not None:
                        while True:
                            try:
                                self.commands.get_nowait()
                            except queue.Empty:
                                break
                        if self.threads:
                            self._enqueue({'op': 'shutdown'})
                            self.commands.put_nowait(None)
                        status = self._await_child_only(self.config.shutdown_timeout_seconds)
                        if status is None:
                            self._signal_members(signal.SIGTERM, receipt)
                            status = self._await_child_only(0.2)
                        if status is None:
                            # Re-census on each pass: a TERM handler can fork late.
                            deadline = time.monotonic() + 3
                            while status is None and time.monotonic() < deadline:
                                self._signal_members(signal.SIGKILL, receipt)
                                status = self._await_child_only(0.05)
                        require(status is not None,
                                'HTTP descendants survive; outer unreaped-group fallback required')
                        self.process.wait(timeout=1)
                        self.reaped = True
                        receipt['child_reaped'] = True
                        receipt['returncode'] = self.process.returncode
                        receipt['owned_descendants_absent'] = not self._members()
                    else:
                        receipt['owned_descendants_absent'] = not self._members()
                except BaseException as error:
                    receipt['errors'].append(type(error).__name__ + ': ' + str(error))
                finally:
                    # Never close live pipe objects while a reader holds their
                    # buffered lock. The outer process-group fallback is bounded.
                    if self.reaped:
                        for thread in self.threads:
                            thread.join(timeout=1)
                        receipt['threads_joined'] = all(not thread.is_alive() for thread in self.threads)
                        if receipt['threads_joined']:
                            for stream in (self.process.stdin, self.process.stdout, self.process.stderr):
                                if stream is not None:
                                    stream.close()
                    if self.pidfd is not None:
                        os.close(self.pidfd)
                        self.pidfd = None
                    receipt['closed_receipt'] = self.closed_receipt
                    receipt['cleanup_ok'] = (receipt['child_reaped']
                        and receipt['owned_descendants_absent'] and receipt['threads_joined']
                        and not receipt['errors'])
                    self.cleanup = receipt
                    save(self.output / 'http-controller-cleanup.json', receipt)
            return receipt
    return Backend
