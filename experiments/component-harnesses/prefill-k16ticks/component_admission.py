#!/usr/bin/env python3
"""Component-stage adaptation of qualified run_stage admission; class body unchanged."""
import argparse
import fcntl
import importlib.util
import os
from pathlib import Path
import re
import resource
import signal
import socket
import stat
import subprocess
import sys
import time

D = Path(__file__).resolve().parent
definition = importlib.util.spec_from_file_location('v14_launch_contract', D / 'component_contract.py')
c = importlib.util.module_from_spec(definition)
definition.loader.exec_module(c)


def mount_record(raw):
    matches = []
    for line in raw.splitlines():
        fields = line.split()
        if len(fields) < 10 or '-' not in fields:
            continue
        separator = fields.index('-')
        if fields[4] == '/dev/shm':
            c.require(separator >= 6 and len(fields) > separator + 3, 'complete shm mount record')
            matches.append((fields[separator + 1], set(fields[5].split(',')) | set(fields[separator + 3].split(','))))
    c.require(len(matches) == 1, 'one unambiguous shm mount')
    filesystem, options = matches[0]
    c.require(filesystem == 'tmpfs' and 'rw' in options and 'noexec' not in options, 'executable writable tmpfs required')
    return {'filesystem': filesystem, 'options': sorted(options)}


def tmpfs_parent():
    root = Path('/dev/shm')
    item = root.lstat()
    c.require(root.resolve(strict=True) == root and stat.S_ISDIR(item.st_mode) and item.st_uid == 0
              and stat.S_IMODE(item.st_mode) == 0o1777, 'root-owned sticky real tmpfs parent')
    mount = mount_record(Path('/proc/self/mountinfo').read_text())
    free = os.statvfs(root)
    available = free.f_bavail * free.f_frsize
    c.require(available >= 32 * 1024**3, 'shm free-space floor reached')
    return {'st_dev': item.st_dev, 'st_ino': item.st_ino, 'free_bytes': available, **mount}


def stage_identity(stage):
    c.stage_name(stage)
    item = stage.lstat()
    c.require(stage.resolve(strict=True) == stage and stat.S_ISDIR(item.st_mode) and item.st_uid == c.UID
              and stat.S_IMODE(item.st_mode) == 0o700 and item.st_dev == Path('/dev/shm').stat().st_dev,
              'owned private stage on approved tmpfs')
    return {'st_dev': item.st_dev, 'st_ino': item.st_ino, 'st_uid': item.st_uid, 'mode': stat.S_IMODE(item.st_mode)}


def output_identity(stage, output):
    c.require(output.parent == stage / 'outputs' and output.name in c.OUTPUT_NAMES,
              'predeclared component output path')
    for path in (output.parent, output):
        item = path.lstat()
        c.require(path.resolve(strict=True) == path and stat.S_ISDIR(item.st_mode) and item.st_uid == c.UID
                  and stat.S_IMODE(item.st_mode) == 0o700 and item.st_dev == stage.stat().st_dev,
                  'owned private component directory in shared input stage')
    return {'st_dev': item.st_dev, 'st_ino': item.st_ino, 'st_uid': item.st_uid, 'mode': stat.S_IMODE(item.st_mode)}


def fresh_output(stage, output):
    c.require(output.parent == stage / 'outputs' and output.name in c.OUTPUT_NAMES,
              'predeclared fresh output root')
    output.parent.mkdir(mode=0o700, exist_ok=True)
    item = output.parent.lstat()
    c.require(output.parent.resolve(strict=True) == output.parent and stat.S_ISDIR(item.st_mode)
              and item.st_uid == c.UID and stat.S_IMODE(item.st_mode) == 0o700
              and item.st_dev == stage.stat().st_dev, 'owned real component parent before creation')
    c.require(not os.path.lexists(output), 'fresh per-component output root required; failed cells are never overwritten')
    output.mkdir(mode=0o700)
    return output_identity(stage, output)


def resources(stage, supervisor, expected, output=None, output_expected=None):
    c.require(stage_identity(stage) == expected, 'stage replaced during execution')
    if output is not None:
        c.require(output_identity(stage, output) == output_expected, 'component output root replaced during execution')
    # The shared root contains all component outputs, so the frozen cap is cumulative.
    return {'tmpfs': tmpfs_parent(), 'bounded': supervisor.resource_snapshot(stage)}


def save_new(path, value):
    with Path(path).open('xb') as stream:
        stream.write(c.encoded(value))


def fuser_members(result):
    c.require(result.returncode in (0, 1) and len(result.stdout) <= 65536 and len(result.stderr) <= 65536,
              'bounded all-UID fuser command failed')
    stdout = result.stdout.decode('ascii')
    stderr = result.stderr.decode('ascii')
    c.require(re.fullmatch(r'\s*(?:[0-9]+\s*)*', stdout) is not None, 'unrecognized fuser PID output')
    values = [int(word) for word in stdout.split()]
    c.require(all(pid > 1 for pid in values) and len(values) == len(set(values)), 'distinct valid fuser PIDs')
    c.require((result.returncode == 0) == bool(values), 'fuser return/PID disagreement')
    if result.returncode == 1:
        c.require(not stdout and not stderr, 'empty fuser result must have no diagnostics')
    else:
        c.require(not stderr or re.fullmatch(r'/dev/kfd:\s*(?:m\s*)*', stderr) is not None,
                  'unrecognized fuser diagnostics')
        c.require(stderr.count('m') <= len(values), 'fuser access markers exceed PID count')
    return sorted(values)


def parent_pid(pid):
    raw = (Path('/proc') / str(pid) / 'stat').read_text()
    head, separator, tail = raw.rpartition(') ')
    words = tail.split()
    c.require(separator and head.startswith(str(pid) + ' (') and len(words) >= 20 and words[1].isdigit(),
              'bounded parent process identity')
    return int(words[1])


def descendant(pid, ancestor):
    seen = set()
    for _ in range(64):
        if pid == ancestor:
            return True
        c.require(pid > 1 and pid not in seen, 'process ancestry lost or cyclic')
        seen.add(pid)
        pid = parent_pid(pid)
    raise ValueError('process ancestry exceeds bound')


class Admission:
    def __init__(self, stage, plan, supervisor, profile, output):
        self.stage, self.plan, self.supervisor, self.profile = stage, plan, supervisor, profile
        self.output = output
        self.owner = stage_identity(stage)
        self.output_owner = output_identity(stage, output)
        self.activity = c.module(stage / 'measurement/gpu_activity.py', plan['sources']['measurement/gpu_activity.py'])
        self.devices = ['/dev/kfd'] + sorted(str(path) for path in Path('/dev/dri').glob('renderD*'))
        c.require(len(self.devices) >= 2, 'KFD and render device roster required')
        self.directory = output / 'admission'
        self.directory.mkdir(mode=0o700, exist_ok=True)

    def owned(self, pids):
        if not pids:
            return []
        wrapper = c.decode(c.read(self.output / 'wrapper.json', maximum=16384)[0])
        c.require(self.activity.identity(Path('/proc'), wrapper['pid']) == wrapper, 'live wrapper PID lifetime')
        expected = (self.stage / 'worker-candidate').stat()
        values = []
        for pid in pids:
            first = self.activity.identity(Path('/proc'), pid)
            c.require(first['uids'] == [c.UID] * 4 and first['proc_uid'] == c.UID, 'owned worker UID')
            path = Path('/proc') / str(pid) / 'exe'
            observed = path.stat()
            c.require(os.readlink(path) == str(self.stage / 'worker-candidate')
                and all(getattr(observed, key) == getattr(expected, key) for key in ('st_dev', 'st_ino', 'st_size')),
                'owned worker executable inode')
            c.require(descendant(pid, wrapper['pid']) and self.activity.identity(Path('/proc'), pid) == first,
                      'worker is not a stable descendant of the owned wrapper')
            values.append(first)
        return values

    def gpu_endpoint(self, observation):
        observation['started_ns'] = time.monotonic_ns()
        kfd = self.supervisor.kfd_snapshot(self.stage)
        observation['kfd'] = kfd
        c.require(not kfd['foreign_pids'], 'foreign KFD work appeared')
        observed = subprocess.run(['/usr/bin/sudo', '-n', '/usr/bin/fuser', '/dev/kfd'],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            timeout=5, check=False)
        observation['fuser'] = {'returncode': observed.returncode,
            'stdout': observed.stdout.decode('ascii'), 'stderr': observed.stderr.decode('ascii')}
        members = fuser_members(observed)
        observation['fuser']['pids'] = members
        observation['finished_ns'] = time.monotonic_ns()
        return sorted(set(kfd['stage_worker_pids']) | set(members))

    def confirm_scan_endpoint(self, phase, setup, initial, allowed, observation):
        endpoint = observation['gpu_endpoint_after'] = {}
        pids = self.gpu_endpoint(endpoint)
        if phase in ('preflight', 'postflight') or initial:
            c.require(not pids, 'GPU process appeared at final lifecycle endpoint')
        if setup is not None:
            c.require(pids == sorted(setup['worker_pids']),
                      'setup worker must remain positively visible at final GPU endpoint')
        current = self.owned(pids)
        endpoint['owned_identities'] = current
        before = {row['pid']: row for row in allowed}
        after = {row['pid']: row for row in current}
        c.require(all(before[pid] == after[pid] for pid in before.keys() & after.keys()),
                  'owned process lifetime changed across descriptor scan')
        if current == allowed:
            return True
        c.require(phase == 'monitor' and not initial and setup is None
                  and (not allowed or not current) and max(len(allowed), len(current)) == 1,
                  'unexpected owned GPU roster change across descriptor scan')
        observation['retry_reason'] = 'owned worker appeared or retired at final GPU endpoint; fresh scan required'
        return False

    def check(self, phase, setup=None, initial=False):
        c.require(phase in ('preflight', 'active', 'postflight', 'monitor'), 'closed admission phase')
        descriptor = os.open(self.output / 'admission.lock', os.O_CREAT | os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
        try:
            lock = os.fstat(descriptor)
            c.require(stat.S_ISREG(lock.st_mode) and lock.st_uid == c.UID and lock.st_nlink == 1
                and stat.S_IMODE(lock.st_mode) == 0o600, 'owned scan lock')
            # A bounded nonblocking lock avoids the outer watchdog and child
            # starting simultaneous all-process scans that invalidate each other.
            until = time.monotonic() + 90
            while True:
                try:
                    fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    break
                except BlockingIOError:
                    c.require(time.monotonic() < until, 'admission scan lock deadline')
                    time.sleep(0.05)
            result = {'accepted': False, 'phase': phase, 'samples': [], 'attempts': [],
                      'started_ns': time.monotonic_ns()}
            name = str(time.monotonic_ns()) + '-' + str(os.getpid()) + '-' + phase
            try:
                result['resources'] = resources(self.stage, self.supervisor, self.owner, self.output, self.output_owner)
                if phase in ('preflight', 'postflight') or initial:
                    self.profile.gpu_idle()
                for attempt in range(3):
                    observation = {'attempt': attempt, 'accepted': False, 'started_ns': time.monotonic_ns()}
                    result['attempts'].append(observation)
                    try:
                        observation['resources'] = resources(self.stage, self.supervisor, self.owner,
                                                             self.output, self.output_owner)
                        before_endpoint = observation['gpu_endpoint_before'] = {}
                        pids = self.gpu_endpoint(before_endpoint)
                        observation['kfd'], observation['fuser'] = before_endpoint['kfd'], before_endpoint['fuser']
                        if phase in ('preflight', 'postflight') or initial:
                            c.require(not pids, 'GPU process remains at lifecycle endpoint')
                        if setup is not None:
                            wanted = setup.get('worker_pids')
                            c.require(type(wanted) is list and len(wanted) == 1
                                      and all(type(pid) is int and pid > 1 for pid in wanted),
                                      'one positively identified setup worker')
                            c.require(not set(pids) - set(wanted), 'unexpected additional KFD process')
                            pids = wanted
                        allowed = self.owned(pids)
                        scan_phase = 'active' if allowed else ('postflight' if phase == 'postflight' else 'preflight')
                        c.require(phase != 'active' or allowed, 'active sample requires owned worker identity')
                        identities = self.directory / (name + '-owners-' + str(attempt) + '.json')
                        save_new(identities, allowed)
                        observation['owners'] = {'path': str(identities), 'sha256': c.read(identities)[1]}
                        argv = ['/usr/bin/sudo', '-n', '/usr/bin/python3', '-I', '-B',
                                str(self.stage / 'measurement/gpu_activity.py'), '--phase', scan_phase,
                                '--owned-identities', str(identities)]
                        for device in self.devices:
                            argv += ['--device', device]
                        observed = subprocess.run(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, timeout=20, check=False)
                        c.require(observed.returncode in (0, 1) and len(observed.stdout) <= 4 * 1024**2
                                  and not observed.stderr, 'bounded readonly privileged descriptor scan')
                        sample = c.decode(observed.stdout)
                        c.require(sample.get('schema') == 'FerricDeviceDescriptorSampleV2'
                                  and sample.get('method') == 'proc-fd-rdev-finite-roster-v1'
                                  and sample.get('sampling_policy') == 'initial-plus-one-birth-frontier-v1'
                                  and sample.get('root_visibility') is True,
                                  'explicit privileged finite-roster sampling policy required')
                        sample['scan_returncode'] = observed.returncode
                        path = self.directory / (name + '-scan-' + str(attempt) + '.json')
                        save_new(path, sample)
                        binding = {'path': str(path), 'sha256': c.read(path)[1],
                                   'accepted': sample.get('accepted') is True}
                        result['samples'].append(binding)
                        observation['descriptor_scan'] = binding
                        if sample.get('accepted') is True and sample.get('complete') is True:
                            c.require(observed.returncode == 0 and sample.get('errors') == []
                                      and sample.get('foreign_users') == [], 'scan acceptance/status mismatch')
                            if not self.confirm_scan_endpoint(phase, setup, initial, allowed, observation):
                                if attempt < 2:
                                    time.sleep(0.1)
                                continue
                            result['accepted'] = observation['accepted'] = True
                            break
                        foreign = sample.get('foreign_users', [])
                        if foreign:
                            c.require(phase == 'monitor' and not initial and setup is None,
                                      'foreign GPU descriptors at exact lifecycle sample')
                            observed_owners = [row['identity'] for row in foreign]
                            current = self.owned([row['pid'] for row in observed_owners])
                            c.require(current == observed_owners,
                                      'foreign GPU descriptors; process lifetime is not owned')
                            observation['retry_reason'] = 'exact owned worker appeared after owner snapshot'
                        else:
                            c.require(sample.get('complete') is False
                                      or (phase == 'monitor' and not initial and allowed
                                          and sample.get('complete') is True and not sample.get('device_users')),
                                      'complete descriptor refusal is terminal')
                            observation['retry_reason'] = 'incomplete sample or monitor worker retirement'
                    except FileNotFoundError as error:
                        # Every retry starts with new KFD, fuser and process identities.
                        # A disappearing process never turns a refused sample into acceptance.
                        observation['retry_reason'] = 'FileNotFoundError: ' + str(error)
                    except BaseException as error:
                        observation['error'] = type(error).__name__ + ': ' + str(error)
                        raise
                    finally:
                        observation['finished_ns'] = time.monotonic_ns()
                    if attempt < 2:
                        time.sleep(0.1)
                c.require(result['accepted'], 'three complete-scan attempts refused; no launch admission')
                result['resources_after'] = resources(self.stage, self.supervisor, self.owner,
                                                      self.output, self.output_owner)
            except BaseException as error:
                result['error'] = type(error).__name__ + ': ' + str(error)
                raise
            finally:
                result['finished_ns'] = time.monotonic_ns()
                save_new(self.directory / (name + '.json'), result)
            return result
        finally:
            os.close(descriptor)
