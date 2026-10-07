"""Read-only acquisition around the explicit-startup finite-roster scanner.

The outer benchmark must hash-bind these modules and its immutable stage first.
No launch, signal, container mutation, or timing admission occurs here.
Only an exact pre-GPU container startup birth may request a fresh complete sample.
"""
import os
from pathlib import Path
import re
import subprocess
import time


def require(condition, message):
    if not condition:
        raise ValueError(message)


def fuser_members(result):
    require(result.returncode in (0, 1) and len(result.stdout) <= 65536
            and len(result.stderr) <= 65536, 'bounded all-UID fuser command failed')
    stdout, stderr = result.stdout.decode('ascii'), result.stderr.decode('ascii')
    require(re.fullmatch(r'\s*(?:[0-9]+\s*)*', stdout) is not None,
            'unrecognized fuser PID output')
    values = [int(word) for word in stdout.split()]
    require(all(pid > 1 for pid in values) and len(values) == len(set(values)),
            'distinct valid fuser PIDs required')
    require((result.returncode == 0) == bool(values), 'fuser return/PID disagreement')
    if result.returncode == 1:
        require(not stdout and not stderr, 'empty fuser result must have no diagnostics')
    else:
        require(not stderr or re.fullmatch(r'/dev/kfd:\s*(?:m\s*)*', stderr) is not None,
                'unrecognized fuser diagnostics')
        require(stderr.count('m') <= len(values), 'fuser access markers exceed PID count')
    return sorted(values)


def sysfs_members(root=Path('/sys/class/kfd/kfd/proc'), *, check=lambda: None):
    values = []
    deadline = time.monotonic() + 3
    with os.scandir(root) as entries:
        for count, entry in enumerate(entries, 1):
            check()
            require(count <= 32768 and time.monotonic() < deadline, 'KFD sysfs process census bound')
            require(entry.name.isdecimal() and int(entry.name) > 1
                    and entry.is_dir(follow_symlinks=False), 'unrecognized KFD process entry')
            values.append(int(entry.name))
    require(len(values) == len(set(values)), 'duplicate KFD process number')
    return sorted(values)


class StartupBudget:
    """One original scan budget shared by at most three complete observations."""
    def __init__(self, activity, *, clock=time.monotonic, runner=subprocess.run):
        self.activity, self.clock, self.runner = activity, clock, runner
        self.started = clock()
        self.observed_pids, self.fd_counts, self.descriptors = set(), {}, 0
        self.retired_pids = set()

    def check(self):
        require(self.clock() - self.started < self.activity.MAX_SECONDS,
                'shared startup sampling deadline exceeded')

    def run(self, argv, **kwargs):
        self.check()
        kwargs['timeout'] = min(kwargs['timeout'],
            self.activity.MAX_SECONDS - (self.clock() - self.started))
        require(kwargs['timeout'] > 0, 'shared startup sampling deadline exceeded')
        # The endpoint must retain acquired bytes before checking the clock.
        return self.runner(argv, **kwargs)

    def scan(self, proc, devices, allowed, phase):
        self.check()
        first_clock = True
        def scan_clock():
            nonlocal first_clock
            self.check()
            if first_clock:
                first_clock = False
                return self.started
            return self.clock()
        def identities(root, pid):
            self.check()
            require(pid not in self.retired_pids, 'departed startup PID reappeared')
            value = self.activity.identity(root, pid)
            self.check()
            return value
        def census(root):
            self.check()
            values = self.activity.pids(root)
            require(not self.retired_pids.intersection(values), 'departed startup PID reappeared')
            self.observed_pids.update(values)
            require(len(self.observed_pids) <= self.activity.MAX_PIDS,
                    'shared startup PID census bound exceeded')
            self.check()
            return values
        def descriptor(path):
            self.check()
            pid = int(Path(path).parent.parent.name)
            self.descriptors += 1
            self.fd_counts[pid] = self.fd_counts.get(pid, 0) + 1
            require(self.descriptors <= self.activity.MAX_TOTAL_FDS
                    and self.fd_counts[pid] <= self.activity.MAX_FDS,
                    'shared startup descriptor bound exceeded')
            result = os.stat(path)
            self.check()
            return result
        # The first clock value anchors the unchanged scanner deadline to the
        # entire observation series, not to this attempt's start.
        return self.activity.scan(proc, devices, allowed, phase, clock=scan_clock,
            identity_reader=identities, pid_reader=census, descriptor_stat=descriptor)


def startup_birth(result, *, identity, runtime):
    """Classify only a retained refusal; never convert it to acceptance."""
    require(result.get('schema') == 'FerricNativeHttpGpuProbeV1'
            and result.get('phase') == 'startup' and result.get('accepted') is False
            and result.get('errors') == ['ValueError: owner or container identity changed across descriptor scan'],
            'only exact startup membership refusal may be reconciled')
    scan = result['descriptor_scan']
    expected = {'schema': 'FerricDeviceDescriptorSampleV2', 'method': 'proc-fd-rdev-finite-roster-v1',
        'sampling_policy': 'initial-plus-one-birth-frontier-v1', 'root_visibility': True,
        'accepted': True, 'complete': True, 'phase': 'startup', 'errors': [],
        'device_users': [], 'owned_users': [], 'foreign_users': []}
    require(all(type(scan.get(key)) is type(value) and scan[key] == value
                for key, value in expected.items()), 'complete no-GPU startup scan required')
    before, after = result['before'], result['after']
    for endpoint in (before, after):
        require(endpoint.get('root_visibility') is True and endpoint.get('fuser_pids') == []
                and endpoint.get('sysfs_pids') == [], 'empty root-visible startup GPU endpoints required')
    old_rows, new_rows = before['owned_identities'], after['owned_identities']
    old, new = ({row['pid']: row for row in rows} for rows in (old_rows, new_rows))
    require(len(old) == len(old_rows) and len(new) == len(new_rows) and set(old) < set(new)
            and all(new[pid] == row for pid, row in old.items()),
            'only monotonic exact-owned startup births may be reconciled')
    current = after['container']
    require(type(current) is dict and set(current) == {'binding', 'state', 'host_pids'}
            and current['host_pids'] == sorted(new) and current['state'] == runtime
            and current['binding'] == {key: identity[key] for key in
                ('id', 'name', 'image', 'label_key', 'label_value')}
            and current['state']['init_pid'] in new,
            'exact retained container identity/runtime required')
    previous = before.get('container')
    if previous is None:
        require(not old, 'only initial container creation may lack a prior binding')
    else:
        require(previous == {**current, 'host_pids': sorted(old)},
                'container restart or binding change cannot be reconciled')
    require(result['before']['finished_ns'] <= scan['started_monotonic_ns']
            < scan['completed_monotonic_ns'] <= result['after']['started_ns'],
            'startup endpoints must bracket the complete scan')
    return True


def startup_departures(result, *, identity, runtime, budget, evidence, proc=Path('/proc')):
    """Prove a specific non-init roster loss; the failed scan stays refused."""
    require(result.get('schema') == 'FerricNativeHttpGpuProbeV1'
            and result.get('phase') == 'startup' and result.get('accepted') is False
            and result.get('errors') == ['ValueError: owner or container identity changed across descriptor scan'],
            'only exact startup membership refusal may be reconciled')
    before, after, scan = result['before'], result['after'], result['descriptor_scan']
    expected = {'schema': 'FerricDeviceDescriptorSampleV2', 'method': 'proc-fd-rdev-finite-roster-v1',
        'sampling_policy': 'initial-plus-one-birth-frontier-v1', 'root_visibility': True,
        'phase': 'startup', 'device_users': [], 'owned_users': [], 'foreign_users': []}
    require(all(type(scan.get(key)) is type(value) and scan[key] == value
                for key, value in expected.items()), 'zero-GPU startup descriptor evidence required')
    for endpoint in (before, after):
        require(endpoint.get('root_visibility') is True and endpoint.get('fuser_pids') == []
                and endpoint.get('sysfs_pids') == [], 'empty root-visible startup GPU endpoints required')
    old_rows, new_rows = before['owned_identities'], after['owned_identities']
    old, new = ({row['pid']: row for row in rows} for rows in (old_rows, new_rows))
    lost = set(old) - set(new)
    require(len(old) == len(old_rows) and len(new) == len(new_rows) and lost
            and all(new[pid] == old[pid] for pid in set(old) & set(new)),
            'unchanged surviving startup owner identities required')
    current, previous = after['container'], before['container']
    require(type(current) is dict and set(current) == {'binding', 'state', 'host_pids'}
            and current['host_pids'] == sorted(new) and current['state'] == runtime
            and current['binding'] == {key: identity[key] for key in
                ('id', 'name', 'image', 'label_key', 'label_value')}
            and previous == {**current, 'host_pids': sorted(old)}
            and runtime['init_pid'] in old and runtime['init_pid'] in new
            and runtime['init_pid'] not in lost,
            'same live init and exact container identity/runtime required')
    if scan.get('accepted') is True:
        require(scan.get('complete') is True and scan.get('errors') == [],
                'complete scan required for post-scan owner departure')
    else:
        require(scan.get('accepted') is False and scan.get('complete') is False
                and scan.get('errors') == ['ValueError: allowlisted worker disappeared during scan'],
                'only the specific allowlisted departure failure is eligible')
        diagnostics = scan.get('failure_diagnostics')
        require(type(diagnostics) is list and len(diagnostics) == 1,
                'exact scanner departure context required')
        context = diagnostics[0].get('context', {})
        pid = context.get('pid')
        require(context.get('operation') == 'departure_proof' and pid in lost
                and context.get('allowed_identity') == old[pid]
                and diagnostics[0].get('error', {}).get('message') ==
                    'allowlisted worker disappeared during scan',
                'scanner failure must identify an exact lost non-init lifetime')
    require(before['finished_ns'] <= scan['started_monotonic_ns']
            < scan['completed_monotonic_ns'] <= after['started_ns'],
            'startup endpoints must bracket the refused scan')
    proofs = []
    for pid in sorted(lost):
        row = {'identity': old[pid], 'accepted': False, 'terminal': False}
        evidence.append(row)
        try:
            budget.check()
            proof = budget.activity.confirm_departure(proc, pid)
            row['proof'] = proof
            require(proof == {'pid': pid, 'method': 'proc-directory-absent-and-pidfd-esrch',
                    'proc_directory_absent_checks': 2, 'pidfd_esrch_checks': 2},
                    'dual absence and pidfd ESRCH proof required for startup departure')
            budget.check()
            row['accepted'] = True
        except (OSError, ValueError, KeyError, TypeError) as error:
            row['error'] = type(error).__name__ + ': ' + str(error)
            raise
        proofs.append({'identity': old[pid], 'proof': proof})
    return proofs


class NativeOwners:
    """Exact setup worker, direct controller child, in the live wrapper session.

    The executable inode binding is derived from the independently hash-checked
    selected native artifact, not an observed arbitrary process executable.
    """
    def __init__(self, activity, lifecycle, binding, proc=Path('/proc')):
        require(type(binding) is dict and set(binding) == {'wrapper', 'controller', 'worker'},
                'closed native owner binding required')
        self.activity, self.lifecycle, self.binding, self.proc = activity, lifecycle, binding, proc

    def __call__(self):
        wrapper, controller, worker = (self.binding[key] for key in ('wrapper', 'controller', 'worker'))
        require(type(worker) is dict and set(worker) == {'identity', 'executable'},
                'closed worker binding required')
        require(self.lifecycle.process(wrapper['pid']) == wrapper
                and wrapper['pid'] == wrapper['group'] == wrapper['session']
                and wrapper['uid'] == 9661, 'live exact owned wrapper required')
        require(self.lifecycle.process(controller['pid']) == controller
                and controller['parent'] == wrapper['pid']
                and controller['group'] == controller['session'] == wrapper['pid']
                and controller['uid'] == 9661, 'live direct owned controller required')
        first = self.activity.identity(self.proc, worker['identity']['pid'])
        require(first == worker['identity'] and first['uids'] == [9661] * 4
                and first['proc_uid'] == 9661, 'exact worker credentials and lifetime required')
        row = self.lifecycle.process(first['pid'])
        require(row['parent'] == controller['pid'] and row['uid'] == 9661
                and row['group'] == row['session'] == wrapper['pid'],
                'worker must remain a direct child in the owned session')
        executable = worker['executable']
        require(type(executable) is dict and set(executable) == {'path', 'dev', 'ino', 'size'},
                'closed selected executable inode binding required')
        path = self.proc / str(first['pid']) / 'exe'
        observed = path.stat()
        require(os.readlink(path) == executable['path']
                and all(getattr(observed, 'st_' + key) == executable[key]
                        for key in ('dev', 'ino', 'size')), 'selected worker executable changed')
        require(self.activity.identity(self.proc, first['pid']) == first
                and self.lifecycle.process(first['pid']) == row
                and self.lifecycle.process(controller['pid']) == controller
                and self.lifecycle.process(wrapper['pid']) == wrapper, 'owned ancestry changed during sample')
        return {first['pid']: first}, None


def container_owners(activity, binding, *, runner=subprocess.run, proc=Path('/proc')):
    def observe():
        return activity.container_members(binding, proc, runner=runner)
    return observe


def endpoint(owners, row, *, runner=subprocess.run, sysfs=sysfs_members, clock=time.monotonic_ns,
             check=lambda: None):
    row.update(started_ns=clock(), root_visibility=os.geteuid() == 0)
    try:
        require(row['root_visibility'], 'root visibility required')
        observed = runner(['/usr/bin/fuser', '/dev/kfd'], stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5, check=False)
        # Store bytes before parsing: malformed text or a later sysfs/owner error
        # must not erase the preceding external endpoint evidence.
        row['fuser_raw'] = {'returncode': observed.returncode,
            'stdout_hex': observed.stdout[:65536].hex(), 'stderr_hex': observed.stderr[:65536].hex(),
            'stdout_bytes': len(observed.stdout), 'stderr_bytes': len(observed.stderr),
            'truncated': len(observed.stdout) > 65536 or len(observed.stderr) > 65536}
        check()
        row['fuser_pids'] = fuser_members(observed)
        row['fuser_raw'].update(stdout=observed.stdout.decode('ascii'), stderr=observed.stderr.decode('ascii'))
        row['sysfs_pids'] = sysfs()
        check()
        allowed, container = owners()
        require(type(allowed) is dict and all(type(pid) is int and pid > 1
                and type(value) is dict and value.get('pid') == pid for pid, value in allowed.items()),
                'exact owned PID identities required')
        row['owned_identities'] = [allowed[pid] for pid in sorted(allowed)]
        if container is not None:
            row['container'] = container
        return allowed, container
    finally:
        row['finished_ns'] = clock()


def observe(activity, attribution, phase, owners, device_paths, *, runner=subprocess.run,
            sysfs=sysfs_members, clock=time.monotonic_ns, proc=Path('/proc'), budget=None,
            native_owned_fd_rescan=False):
    """Retain every acquired endpoint and descriptor hit, including on refusal.

    This is a single bounded sample, with no implicit retry or continuous-isolation
    claim. The caller stores this entire result and stops on terminal evidence.
    """
    result = {'schema': 'FerricNativeHttpGpuProbeV1', 'phase': phase, 'accepted': False,
              'started_ns': clock(), 'errors': [],
              'scope': 'finite descriptor interval bracketed by GPU endpoints; not continuous isolation'}
    try:
        check = budget.check if budget is not None else lambda: None
        check()
        if budget is not None:
            runner = budget.run
            if sysfs is sysfs_members:
                sysfs = lambda: sysfs_members(check=check)
        require(os.geteuid() == 0, 'root visibility required')
        require(phase in ('preflight', 'startup', 'active', 'postflight'), 'closed probe phase required')
        require(type(device_paths) is list and len(device_paths) >= 2
                and device_paths[0] == '/dev/kfd'
                and device_paths[1:] == sorted(set(device_paths[1:]))
                and all(re.fullmatch(r'/dev/dri/renderD[0-9]+', path) for path in device_paths[1:]),
                'KFD and closed render device roster required')
        devices = activity.device_identities(device_paths)
        check()
        result['devices'] = [{'major': pair[0], 'minor': pair[1], 'paths': paths}
                             for pair, paths in sorted(devices.items())]
        before = result['before'] = {}
        allowed, container = endpoint(owners, before, runner=runner, sysfs=sysfs, clock=clock, check=check)
        check()
        require(set(before['fuser_pids'] + before['sysfs_pids']) <= set(allowed),
                'foreign GPU process at initial endpoint')
        require(phase == 'startup' or (phase == 'active') == bool(allowed),
                'owner roster disagrees with lifecycle phase')
        require(type(native_owned_fd_rescan) is bool, 'explicit native FD rescan selection required')
        require(not native_owned_fd_rescan or (phase == 'active' and budget is None
                and container is None and len(allowed) == 1), 'native-only active FD rescan selection')
        rescan = native_owned_fd_rescan and before['fuser_pids'] == before['sysfs_pids'] == sorted(allowed)
        scan = budget.scan if budget is not None else activity.scan
        sample = scan(proc, devices, allowed, phase, **({'native_owned_fd_rescan': True} if rescan else {}))
        result['descriptor_scan'] = sample
        check()
        after = result['after'] = {}
        final_allowed, final_container = endpoint(owners, after, runner=runner, sysfs=sysfs, clock=clock, check=check)
        check()
        require(activity.device_identities(device_paths) == devices, 'device identity changed across scan')
        require(final_allowed == allowed and final_container == container,
                'owner or container identity changed across descriptor scan')
        if rescan:
            require(after['fuser_pids'] == after['sysfs_pids'] == sorted(allowed),
                    'same positive native GPU endpoints required after FD rescan')
        if container is not None:
            sample['container'] = container
        result['attribution'] = attribution.accept(sample, before, after, phase=phase,
            identities=before['owned_identities'], container=container)
        check()
        result['accepted'] = True
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        result['errors'].append(type(error).__name__ + ': ' + str(error)[:4096])
    finally:
        result['finished_ns'] = clock()
    return result
