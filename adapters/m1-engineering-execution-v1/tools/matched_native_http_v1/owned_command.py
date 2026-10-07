"""Bounded same-session helper commands; only the unreaped direct child is signaled."""
import os
from pathlib import Path
import signal
import subprocess
import time


def run(argv, output, label, *, lifecycle, save, timeout=30, limit=4 * 1024**2,
        monitor=None, check=True):
    require = lifecycle.require
    require(type(argv) is list and argv and all(type(word) is str and word for word in argv),
            'literal helper argv required')
    require(type(label) is str and label and all(letter.isalnum() or letter in '-_' for letter in label),
            'one local command label required')
    require(0 < timeout <= 5000 and 0 < limit <= 256 * 1024**2, 'bounded helper command required')
    require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, 'default SIGCHLD required')
    output = Path(output)
    stdout, stderr = output / (label + '.stdout'), output / (label + '.stderr')
    record = {'schema': 'FerricNativeHttpCommandV1', 'argv': argv, 'timeout_seconds': timeout,
        'limit_bytes': limit, 'start_new_session': False, 'started_ns': time.monotonic_ns(),
        'signals': [], 'errors': [], 'child_reaped': False, 'accepted': False}
    process, descriptor = None, None

    def status():
        require(process is not None and not record['child_reaped'], 'owned unreaped direct child required')
        return os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)

    def send(number):
        # Unreaped direct-child ownership prevents PID reuse even if pidfd_open
        # failed during construction. Never signal a process group here.
        if status() is not None:
            return
        if descriptor is not None:
            signal.pidfd_send_signal(descriptor, number)
        else:
            os.kill(process.pid, number)
        record['signals'].append(int(number))

    try:
        with stdout.open('xb') as out, stderr.open('xb') as err:
            with lifecycle.deferred_stop():
                process = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=out, stderr=err,
                    start_new_session=False, env={**os.environ, 'PYTHONDONTWRITEBYTECODE': '1'})
                record['pid'] = process.pid
                descriptor = os.pidfd_open(process.pid, 0)
                record['pidfd_opened'] = True
            deadline = time.monotonic() + timeout
            while status() is None:
                require(time.monotonic() < deadline, 'helper deadline exceeded')
                require(stdout.stat().st_size + stderr.stat().st_size <= limit, 'helper evidence budget exceeded')
                if monitor is not None:
                    monitor()
                time.sleep(0.05)
            require(stdout.stat().st_size + stderr.stat().st_size <= limit, 'helper evidence budget exceeded')
            record['returncode'] = process.wait(timeout=1)
            record['child_reaped'] = True
            require(not check or record['returncode'] == 0, 'helper command failed: ' + label)
            record['accepted'] = True
            return record['returncode'], stdout.read_bytes(), stderr.read_bytes()
    except BaseException as error:
        record['errors'].append(type(error).__name__ + ': ' + str(error)[:4096])
        raise
    finally:
        with lifecycle.deferred_stop(deliver=False):
            try:
                if process is not None and not record['child_reaped']:
                    send(signal.SIGTERM)
                    deadline = time.monotonic() + 1
                    while status() is None and time.monotonic() < deadline:
                        time.sleep(0.02)
                    send(signal.SIGKILL)
                    record['returncode'] = process.wait(timeout=3)
                    record['child_reaped'] = True
            except BaseException as error:
                record['errors'].append('cleanup: ' + type(error).__name__ + ': ' + str(error)[:4096])
                record['accepted'] = False
            finally:
                if descriptor is not None:
                    os.close(descriptor)
                record['finished_ns'] = time.monotonic_ns()
                record['scope'] = 'direct helper only; outer unreaped wrapper owns same-group descendants'
                save(output / (label + '.command.json'), record)
