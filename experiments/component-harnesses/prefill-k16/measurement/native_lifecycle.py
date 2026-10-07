"""Campaign-specific ownership: one unreaped outer session leader, no detached children."""
import collections
from contextlib import contextmanager
import os
from pathlib import Path
import selectors
import signal
import subprocess
import time

STOP_SIGNALS = (signal.SIGTERM, signal.SIGINT, signal.SIGHUP)


def require(value, message):
    if not value:
        raise ValueError(message)


class StopRequested(BaseException):
    """Not InterruptedError: selectors catches and suppresses that exception."""


def interrupted(number, _frame):
    raise StopRequested("native cell signal " + str(number))


@contextmanager
def handling_stop():
    previous = {number: signal.signal(number, interrupted) for number in STOP_SIGNALS}
    try:
        yield
    finally:
        for number, handler in previous.items():
            signal.signal(number, handler)


@contextmanager
def deferred_stop(*, deliver=True):
    # A recording handler, not a blocked mask inherited by the spawned child.
    pending = []
    previous = {number: signal.signal(number, lambda sig, _frame: pending.append(sig))
                for number in STOP_SIGNALS}
    try:
        yield pending
    finally:
        for number, handler in previous.items():
            signal.signal(number, handler)
    if pending and deliver:
        interrupted(pending[0], None)


def process(pid):
    root = Path('/proc') / str(pid)
    raw = (root / 'stat').read_bytes()
    require(len(raw) < 4096, 'bounded process identity')
    fields = raw.rsplit(b')', 1)[1].split()
    return {'pid': pid, 'parent': int(fields[1]), 'group': int(fields[2]),
            'session': int(fields[3]), 'start': int(fields[19]), 'uid': root.stat().st_uid}


def unreaped(proc, anchor):
    # waitid without WNOWAIT would release the PID/PGID before the last signal.
    status = os.waitid(os.P_PID, proc.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
    require(process(proc.pid) == anchor, 'owned child lifetime/credentials changed')
    return status


def members(anchor):
    require(process(anchor['pid']) == anchor, 'owned session leader identity changed')
    require(anchor['pid'] == anchor['group'] == anchor['session'], 'owned session leader required')
    result = []
    deadline = time.monotonic() + 2
    with os.scandir('/proc') as entries:
        for count, entry in enumerate((entry for entry in entries if entry.name.isdecimal()), 1):
            require(count <= 32768 and time.monotonic() < deadline, 'bounded owned-group census')
            try:
                row = process(int(entry.name))
            except (FileNotFoundError, ProcessLookupError):
                continue
            if row['group'] == anchor['group']:
                require(row['session'] == anchor['session'], 'owned group session changed')
                result.append(row)
    require(any(row == anchor for row in result), 'owned session leader disappeared')
    return sorted(result, key=lambda row: row['pid'])


def signal_member(row, wrapper, number):
    require(row['pid'] != wrapper['pid'] and row['group'] == row['session'] == wrapper['pid']
            and row['uid'] == wrapper['uid'], 'only a non-wrapper owned-group member may be signaled')
    require(process(wrapper['pid']) == wrapper, 'wrapper identity changed before member signal')
    descriptor = os.pidfd_open(row['pid'], 0)
    try:
        require(process(row['pid']) == row, 'member lifetime changed while opening pidfd')
        signal.pidfd_send_signal(descriptor, number)
    finally:
        os.close(descriptor)


def controller_class(runner):
    class Controller(runner.Controller):
        def __init__(self, argv, output, deadline):
            self.output, self.deadline = Path(output), deadline
            self.queue, self.pending = collections.deque(), bytearray()
            self.counts = {'stdout': 0, 'stderr': 0}
            self.logs = {name: (self.output / (name + '.raw')).open('xb')
                         for name in ('stdin', 'stdout', 'stderr')}
            self.received = (self.output / 'received.jsonl').open('x')
            self.proc, self.reaped, self.clean_exit = None, False, False
            self.selector = selectors.DefaultSelector()
            self.wrapper, self.anchor, self.cleanup = process(os.getpid()), None, None
            require(self.wrapper['group'] == self.wrapper['session'] == os.getpid(), 'wrapper must own session')
            require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, 'default SIGCHLD required')
            try:
                self.proc = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE, bufsize=0, start_new_session=False)
                self.anchor = process(self.proc.pid)
                require(self.anchor['parent'] == os.getpid()
                        and self.anchor['group'] == self.anchor['session'] == self.wrapper['pid']
                        and self.anchor['uid'] == self.wrapper['uid'], 'direct same-group controller required')
                unreaped(self.proc, self.anchor)
                runner.save(self.output / 'process.json', {'pid': self.proc.pid,
                    'owned_pgid': self.wrapper['pid'], 'argv': argv, 'start_new_session': False,
                    'controller_identity': self.anchor, 'wrapper_identity': self.wrapper})
                for name in ('stdin', 'stdout', 'stderr'):
                    os.set_blocking(getattr(self.proc, name).fileno(), False)
                for name in ('stdout', 'stderr'):
                    self.selector.register(getattr(self.proc, name), selectors.EVENT_READ, name)
            except BaseException:
                self.close()
                raise

        def owned_group_members(self):
            require(not self.reaped and self.anchor is not None, 'unreaped controller required')
            unreaped(self.proc, self.anchor)
            return [{'pid': row['pid']} for row in members(self.wrapper) if row['pid'] != self.wrapper['pid']]

        def close(self):
            if self.cleanup is not None:
                return self.cleanup
            cleanup = {'owned_pgid': self.wrapper['pid'], 'term_sent': False, 'kill_sent': False,
                       'errors': [], 'child_reaped': False, 'owned_descendants_absent': False,
                       'group_scope': 'wrapper-owned-session; controller descendants exclude wrapper'}
            with deferred_stop(deliver=False):
                try:
                    if self.proc is not None:
                        require(not self.reaped and self.anchor is not None, 'owned unreaped controller required')
                        status = unreaped(self.proc, self.anchor)
                        if self.clean_exit:
                            require(status is not None and status.si_code == os.CLD_EXITED and status.si_status == 0,
                                    'clean controller wait status')
                            require([row['pid'] for row in self.owned_group_members()] == [self.proc.pid],
                                    'clean controller left descendants')
                        else:
                            # Each pass is fresh, so a worker forked after TERM is included before KILL.
                            for number, label in ((signal.SIGTERM, 'term_sent'), (signal.SIGKILL, 'kill_sent')):
                                unreaped(self.proc, self.anchor)
                                for row in members(self.wrapper):
                                    if row['pid'] != self.wrapper['pid'] and row['uid'] == self.wrapper['uid']:
                                        try:
                                            signal_member(row, self.wrapper, number)
                                            cleanup[label] = True
                                        except ProcessLookupError:
                                            pass
                                if number == signal.SIGTERM:
                                    time.sleep(0.2)
                        self.proc.wait(timeout=3)
                        self.reaped = True
                        cleanup['child_reaped'] = True
                        cleanup['returncode'] = self.proc.returncode
                        cleanup['owned_descendants_absent'] = all(
                            row['pid'] == self.wrapper['pid'] for row in members(self.wrapper))
                except BaseException as error:
                    cleanup['errors'].append(type(error).__name__ + ': ' + str(error))
                finally:
                    if self.proc is not None:
                        for name in ('stdin', 'stdout', 'stderr'):
                            getattr(self.proc, name).close()
                    self.selector.close()
                    for handle in self.logs.values():
                        handle.close()
                    self.received.close()
                    cleanup['cleanup_ok'] = (cleanup['child_reaped'] and cleanup['owned_descendants_absent']
                                             and not cleanup['errors'])
                    self.cleanup = cleanup
                    runner.save(self.output / 'cleanup.json', cleanup)
            return cleanup
    return Controller


def supervise_group(base, argv, output, check, stop_signal=lambda: None, *, duration=1500,
                    grace=20, kill_wait=5, interval=3):
    """Never reap the wrapper session leader until all possible group signals finish."""
    receipt = {'started': base.timestamp(), 'argv': argv, 'term_sent': False, 'kill_sent': False,
               'child_reaped': False, 'errors': [], 'termination_reason': 'not_started',
               'duration_seconds': duration, 'term_grace_seconds': grace, 'cleanup_ok': False,
               'group_policy': 'one-unreaped-wrapper-owned-session-v1'}
    child, anchor, descriptor = None, None, None
    status = 125

    def observe():
        return unreaped(child, anchor)

    def group():
        observe()
        return members(anchor)

    def send(number, label):
        observe()
        require(not receipt['child_reaped'], 'cannot signal reaped wrapper group')
        os.killpg(child.pid, number)
        receipt[label] = True

    def await_group(seconds):
        deadline = time.monotonic() + seconds
        while True:
            observed = observe()
            rows = group()
            if observed is not None and [row['pid'] for row in rows] == [child.pid]:
                receipt['group_before_reap'] = rows
                return True
            if time.monotonic() >= deadline:
                return False
            time.sleep(0.05)

    def pre_anchor_cleanup():
        # Popen(start_new_session=True) already established ownership. Keep the
        # direct child unreaped; kernel PGID/SID checks do not depend on /proc.
        for number, label in ((signal.SIGTERM, 'term_sent'), (signal.SIGKILL, 'kill_sent')):
            os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            require(os.getpgid(child.pid) == os.getsid(child.pid) == child.pid,
                    'kernel-owned direct child session required')
            os.killpg(child.pid, number)
            receipt[label] = True
            if number == signal.SIGTERM:
                time.sleep(0.2)
        receipt['returncode'] = child.wait(timeout=kill_wait)
        receipt['child_reaped'] = True
        receipt['owned_group_absent'] = False
        receipt['errors'].append('pre-anchor cleanup signaled owned session; group absence unverified')

    try:
        base.require_facilities()
        receipt['admission'] = check(True)
        require(stop_signal() is None, 'stop requested before child launch')
        with (output / 'launch.stdout').open('xb') as stdout, (output / 'launch.stderr').open('xb') as stderr:
            # Outer handlers only record a stop; assignment cannot be interrupted.
            child = subprocess.Popen(argv, cwd=output, stdout=stdout, stderr=stderr,
                                     stdin=subprocess.DEVNULL, start_new_session=True)
        anchor = process(child.pid)
        require(anchor['parent'] == os.getpid() and anchor['uid'] == os.getuid()
                and anchor['group'] == anchor['session'] == child.pid, 'owned wrapper session required')
        observe()
        descriptor = os.pidfd_open(child.pid, 0)
        receipt.update(child_pid=child.pid, pidfd_opened=True, wrapper_identity=anchor)
        base.save(output / 'launch-supervisor.json', receipt)
        deadline, next_check = time.monotonic() + duration, time.monotonic()
        while observe() is None:
            number = stop_signal()
            if number is not None:
                receipt.update(termination_reason='signal', stop_signal=number)
                break
            if time.monotonic() >= deadline:
                receipt['termination_reason'] = 'timeout'
                break
            if time.monotonic() >= next_check:
                try:
                    receipt['last_admission'] = check(False)
                except Exception as error:
                    receipt['termination_reason'] = 'resource_or_foreign_kfd'
                    receipt['errors'].append(type(error).__name__ + ': ' + str(error))
                    (output / 'launch-resource-stop.txt').write_text(base.timestamp() + '\n')
                    break
                next_check = time.monotonic() + interval
            time.sleep(0.05)
        else:
            receipt['termination_reason'] = 'completed'
    except BaseException as error:
        receipt['errors'].append(type(error).__name__ + ': ' + str(error))
        receipt['termination_reason'] = 'supervisor_error'
    finally:
        with deferred_stop(deliver=False):
            if child is not None:
                try:
                    if anchor is None:
                        pre_anchor_cleanup()
                    else:
                        if not await_group(0):
                            send(signal.SIGTERM, 'term_sent')
                            if not await_group(grace):
                                send(signal.SIGKILL, 'kill_sent')
                                require(await_group(kill_wait), 'owned group did not retire after KILL')
                        receipt['returncode'] = child.wait(timeout=1)
                        receipt['child_reaped'] = True
                        receipt['owned_group_absent'] = True
                except BaseException as error:
                    receipt['errors'].append('cleanup: ' + type(error).__name__ + ': ' + str(error))
                finally:
                    if descriptor is not None:
                        os.close(descriptor)
            returncode = receipt.get('returncode')
            receipt['cleanup_ok'] = child is None or (receipt['child_reaped']
                and receipt.get('owned_group_absent') is True and returncode is not None
                and returncode >= 0 and not receipt['kill_sent'])
            if receipt['termination_reason'] == 'completed':
                status = returncode if returncode is not None and returncode >= 0 else 125
                if not receipt['cleanup_ok']:
                    status = 125
            elif receipt['termination_reason'] == 'timeout':
                status = 124
            elif receipt['termination_reason'] == 'signal':
                status = 128 + receipt['stop_signal']
            receipt.update(status=status, finished=base.timestamp())
            base.save(output / 'launch-supervisor.json', receipt)
    return receipt
