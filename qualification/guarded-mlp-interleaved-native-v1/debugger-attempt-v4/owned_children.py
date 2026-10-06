"""Retire only this serial subreaper's adopted children, including escaped PGs."""
import ctypes
import json
import os
from pathlib import Path
import signal
import time

FLAGS = os.WEXITED | os.WNOHANG | os.WNOWAIT
SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)


def require(value, message):
    if not value:
        raise RuntimeError(message)


def child_state():
    try:
        return os.waitid(os.P_ALL, 0, FLAGS)
    except ChildProcessError:
        return False


def preconditions(empty=False):
    require(len(list(Path('/proc/self/task').iterdir())) == 1, 'single-threaded controller')
    require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, 'default SIGCHLD disposition')
    value = ctypes.c_int()
    require(ctypes.CDLL(None, use_errno=True).prctl(37, ctypes.byref(value), 0, 0, 0) == 0
            and value.value == 1, 'actual child subreaper')
    if empty:
        require(child_state() is False, 'no existing owned children before leaf')


def children():
    path = Path(f'/proc/self/task/{os.getpid()}/children')
    with path.open('rb') as stream:
        body = stream.read(4097)
    require(len(body) <= 4096, 'bounded child roster')
    values = [int(word) for word in body.split()]
    require(len(values) == len(set(values)) <= 32 and all(pid > 1 for pid in values),
            'bounded positive unique child PIDs')
    return values


def identity(pid):
    with Path(f'/proc/{pid}/stat').open('rb') as stream:
        body = stream.read(8193)
    require(len(body) <= 8192 and body.startswith(str(pid).encode() + b' ('), 'bounded process stat')
    fields = body.rsplit(b') ', 1)[1].split()
    require(len(fields) >= 20 and int(fields[1]) == os.getpid(), 'actual direct child')
    return dict(pid=pid, pgid=int(fields[2]), sid=int(fields[3]), start_ticks=int(fields[19]))


def authenticated_fd(pid):
    fd = os.pidfd_open(pid, 0)
    try:
        os.waitid(os.P_PIDFD, fd, FLAGS)
        return fd
    except BaseException:
        os.close(fd)
        raise


def retire(deadline, roster=children):
    preconditions()
    seen, reaped, deferred, failure = {}, [], set(), None
    old_mask = signal.pthread_sigmask(signal.SIG_BLOCK, SIGNALS)
    previous = {sig: signal.getsignal(sig) for sig in SIGNALS}
    try:
        for sig in SIGNALS:
            signal.signal(sig, lambda number, _frame: deferred.add(number))
    except BaseException:
        for sig, handler in previous.items():
            signal.signal(sig, handler)
        signal.pthread_sigmask(signal.SIG_SETMASK, old_mask)
        raise
    try:
        signal.pthread_sigmask(signal.SIG_SETMASK, old_mask)
        for _ in range(1000):
            state = child_state()
            if state is False:
                break
            require(time.monotonic() < deadline, 'adopted-child cleanup deadline')
            # An empty proc snapshot is not authoritative. waitid may expose a
            # zombie omitted by that snapshot; ECHILD alone closes ownership.
            candidates = set(roster())
            if state is not None:
                candidates.add(state.si_pid)
            require(len(candidates) <= 32 and len(seen) <= 64, 'adopted-child count bound')
            for pid in sorted(candidates):
                try:
                    fd = authenticated_fd(pid)
                except (ProcessLookupError, ChildProcessError):
                    continue
                try:
                    value = identity(pid)
                    key = (pid, value['start_ticks'])
                    require(key in seen or len(seen) < 64, 'adopted-child identity bound')
                    row = seen.setdefault(key, dict(**value, first_seen=time.monotonic(), signals=[]))
                    status = os.waitid(os.P_PIDFD, fd, FLAGS)
                    if status is not None:
                        consumed = os.waitid(os.P_PIDFD, fd, os.WEXITED | os.WNOHANG)
                        require(consumed is not None and consumed.si_pid == pid, 'same child reaped')
                        reaped.append(dict(**value, code=consumed.si_code, status=consumed.si_status))
                    else:
                        sig = signal.SIGKILL if time.monotonic() - row['first_seen'] >= 1 else signal.SIGTERM
                        if int(sig) not in row['signals']:
                            signal.pidfd_send_signal(fd, sig)
                            row['signals'].append(int(sig))
                except ProcessLookupError:
                    pass
                finally:
                    os.close(fd)
            time.sleep(0.01)
        else:
            raise RuntimeError('adopted-child scan bound')
        require(child_state() is False, 'unresolved owned child')
    except BaseException as error:
        failure = repr(error)
    finally:
        restore_mask = signal.pthread_sigmask(signal.SIG_BLOCK, SIGNALS)
        try:
            for sig, handler in previous.items():
                signal.signal(sig, handler)
        finally:
            signal.pthread_sigmask(signal.SIG_SETMASK, restore_mask)
    closed = child_state() is False
    return dict(complete=closed and failure is None, error=failure, no_children=closed,
                forced=any(row['signals'] for row in seen.values()),
                observed=list(seen.values()), reaped=reaped, deferred_signals=sorted(deferred))


def orphan(nested=False, exited=False):
    read_fd, write_fd = os.pipe()
    leader = os.fork()
    if leader == 0:
        os.close(read_fd)
        os.setsid()
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
        child = os.fork()
        if child == 0:
            if nested:
                grandchild = os.fork()
                if grandchild == 0:
                    os.close(write_fd)
                    os.setsid()
                    time.sleep(60)
                    os._exit(0)
                os.write(write_fd, (str(os.getpid()) + ' ' + str(grandchild)).encode())
            else:
                os.write(write_fd, str(os.getpid()).encode())
            os.close(write_fd)
            if not exited:
                time.sleep(60)
            os._exit(0)
        os.close(write_fd)
        os._exit(0)
    os.close(write_fd)
    try:
        expected = {int(word) for word in os.read(read_fd, 128).split()}
    finally:
        os.close(read_fd)
    require(os.waitpid(leader, 0)[0] == leader, 'fixture leader reaped')
    require(len(expected) == (2 if nested else 1), 'fixture child identities')
    return expected


def selftest():
    signal.alarm(45)
    signal.signal(signal.SIGCHLD, signal.SIG_DFL)
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'fixture subreaper')
    preconditions(empty=True)
    checks = []
    try:
        remaining = signal.getitimer(signal.ITIMER_REAL)[0]
        value = retire(time.monotonic() + 5)
        require(value['complete'] and not value['forced'] and not value['reaped'], 'empty case')
        require(0 < signal.getitimer(signal.ITIMER_REAL)[0] <= remaining, 'active timer preserved')
        checks.append('empty')
        for name, nested, exited in (('escaped_group', False, False),
                                     ('nested_adoption', True, False),
                                     ('zombie', False, True)):
            expected = orphan(nested, exited)
            if exited:
                until = time.monotonic() + 5
                while child_state() is None:
                    require(time.monotonic() < until, 'fixture zombie wait bound')
                    time.sleep(0.01)
            value = retire(time.monotonic() + 10)
            require(value['complete'] and {p['pid'] for p in value['reaped']} == expected,
                    'complete exact fixture descendants')
            require(all(p['pgid'] != os.getpgrp() for p in value['reaped']), 'escaped fixture PGs')
            if exited:
                require(not value['forced'] and all(p['code'] == os.CLD_EXITED and p['status'] == 0
                                                   for p in value['reaped']), 'natural zombie reaping')
            checks.append(name)
        expected = orphan()
        calls = 0
        def incomplete():
            nonlocal calls
            calls += 1
            return [] if calls <= 3 else children()
        value = retire(time.monotonic() + 10, incomplete)
        require(calls > 3 and value['complete']
                and {p['pid'] for p in value['reaped']} == expected, 'empty proc snapshot is not ECHILD')
        checks.append('incomplete_proc_snapshot')
        try:
            fd = authenticated_fd(os.getpid())
        except ChildProcessError:
            checks.append('nonchild_refused_without_signal')
        else:
            os.close(fd)
            raise RuntimeError('nonchild pidfd accepted')
        expected = orphan()
        value = retire(time.monotonic() - 1)
        require(not value['complete'] and not value['no_children'] and value['error']
                and not value['forced'], 'deadline retains unresolved state')
        value = retire(time.monotonic() + 10)
        require(value['complete'] and {p['pid'] for p in value['reaped']} == expected,
                'fixture deadline case cleanup')
        checks.append('deadline_exhaustion')
        expected = orphan()
        old_handler = signal.getsignal(signal.SIGTERM)
        old_mask = signal.pthread_sigmask(signal.SIG_BLOCK, [])
        remaining = signal.getitimer(signal.ITIMER_REAL)[0]
        injected = False
        def interrupted_roster():
            nonlocal injected
            if not injected:
                injected = True
                signal.raise_signal(signal.SIGTERM)
                signal.raise_signal(signal.SIGTERM)
            return children()
        value = retire(time.monotonic() + 10, interrupted_roster)
        require(value['complete'] and value['deferred_signals'] == [signal.SIGTERM]
                and {p['pid'] for p in value['reaped']} == expected, 'bounded deferred signals')
        require(signal.getsignal(signal.SIGTERM) == old_handler
                and signal.pthread_sigmask(signal.SIG_BLOCK, []) == old_mask
                and 0 < signal.getitimer(signal.ITIMER_REAL)[0] <= remaining,
                'handlers, mask and active timer preserved')
        checks.append('deferred_signal_and_timer')
        preconditions(empty=True)
        print(json.dumps(dict(passed=True, tests=checks, gpu_execution=False, project_execution=False)))
    finally:
        value = retire(time.monotonic() + 10)
        require(value['complete'], 'final fixture child retirement')
        signal.alarm(0)


if __name__ == '__main__':
    selftest()
