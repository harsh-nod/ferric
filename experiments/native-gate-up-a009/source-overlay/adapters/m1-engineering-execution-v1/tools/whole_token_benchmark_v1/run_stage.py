#!/usr/bin/env python3
"""Guard predeclared V14 cells sharing fixed inputs; never signal discovered PIDs."""
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
definition = importlib.util.spec_from_file_location('v14_launch_contract', D / 'launch_contract.py')
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
    c.require(output.parent == stage / 'cells' and output.name in [row[0] for row in c.CELL_ORDER],
              'predeclared cell output path')
    for path in (output.parent, output):
        item = path.lstat()
        c.require(path.resolve(strict=True) == path and stat.S_ISDIR(item.st_mode) and item.st_uid == c.UID
                  and stat.S_IMODE(item.st_mode) == 0o700 and item.st_dev == stage.stat().st_dev,
                  'owned private cell directory in shared input stage')
    return {'st_dev': item.st_dev, 'st_ino': item.st_ino, 'st_uid': item.st_uid, 'mode': stat.S_IMODE(item.st_mode)}


def fresh_output(stage, output):
    c.require(output.parent == stage / 'cells' and output.name in [row[0] for row in c.CELL_ORDER],
              'predeclared fresh output root')
    output.parent.mkdir(mode=0o700, exist_ok=True)
    item = output.parent.lstat()
    c.require(output.parent.resolve(strict=True) == output.parent and stat.S_ISDIR(item.st_mode)
              and item.st_uid == c.UID and stat.S_IMODE(item.st_mode) == 0o700
              and item.st_dev == stage.stat().st_dev, 'owned real cell parent before creation')
    c.require(not os.path.lexists(output), 'fresh per-cell output root required; failed cells are never overwritten')
    output.mkdir(mode=0o700)
    return output_identity(stage, output)


def resources(stage, supervisor, expected, output=None, output_expected=None):
    c.require(stage_identity(stage) == expected, 'stage replaced during execution')
    if output is not None:
        c.require(output_identity(stage, output) == output_expected, 'cell output root replaced during execution')
    # The shared root contains all cell outputs, so the frozen cap is cumulative.
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
                        pids = sorted(set(kfd['stage_worker_pids']) | set(members))
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
                        sample['scan_returncode'] = observed.returncode
                        path = self.directory / (name + '-scan-' + str(attempt) + '.json')
                        save_new(path, sample)
                        binding = {'path': str(path), 'sha256': c.read(path)[1],
                                   'accepted': sample.get('accepted') is True}
                        result['samples'].append(binding)
                        observation['descriptor_scan'] = binding
                        if sample.get('accepted') is True and sample.get('complete') is True:
                            c.require(observed.returncode == 0, 'scan acceptance/status mismatch')
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


def execute(stage, plan, selected, loaded, counter, evidence, cell, options, identities):
    c.require(os.environ.get('FERRIC_V14_SUPERVISOR_PID') == str(os.getppid()), 'live owning supervisor required')
    _, old, legacy, support, profile, runner, supervisor, _ = loaded
    for number in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        signal.signal(number, runner.interrupted)
    output = Path(selected['output'])
    admission = Admission(stage, plan, supervisor, profile, output)
    save_new(output / 'wrapper.json', admission.activity.identity(Path('/proc'), os.getpid()))
    model = runner.model_identities(options)
    save_new(output / 'model-before.json', model)
    placement = support.inherited_placement()
    save_new(output / 'launch-placement.json', placement)
    admitted = evidence.admitted_executables(identities)
    endpoints, placements = [], []

    def check(phase, setup):
        result = admission.check(phase, setup)
        if phase == 'active':
            worker_row = evidence.process(setup['worker_pids'][0])['process']
            controller_row = evidence.process(worker_row['parent_pid'])['process']
            worker = {key: worker_row[key] for key in evidence.STABLE_PROCESS}
            controller = {key: controller_row[key] for key in evidence.STABLE_PROCESS}
            observed = {'worker': worker, 'controller': controller}
            c.require(controller['parent_pid'] == os.getpid()
                      and controller['process_group'] == controller['session'] == controller['process_id']
                      and worker['process_group'] == worker['session'] == controller['process_id'],
                      'owned controller group and worker parent')
            for row in observed.values():
                c.require(row['nice'] == placement['nice'] == 0
                          and support.affinity_list(row['cpus_allowed_list']) == placement['affinity'],
                          'fresh default process placement changed')
            if placements:
                legacy.stable_placement(placements[0], observed)
            placements.append(observed)
            endpoints.append(evidence.endpoint(observed, admitted, after_request=len(endpoints) > 0))
        return result

    result = cell.run_cell(selected['spec'], output / 'cell-results', runner=runner, legacy=legacy,
                           counter=counter, evidence=evidence, admission=check)
    c.require(len(endpoints) == 2 and evidence.admitted_executables(identities) == admitted, 'paired process endpoints')
    cost = evidence.cpu_cost(endpoints[0], endpoints[1], os.sysconf('SC_CLK_TCK'))
    count, _ = cell.shape(selected['spec'])
    for row in cost['roles'].values():
        row['cpu_seconds_per_token'] = row['cpu_seconds'] / (count * 128)
    cost['scope'] = 'All requests including excluded warmups; excludes setup/teardown; not GPU time'
    cost['output_tokens'] = count * 128
    save_new(output / 'process-endpoints.json', {'placements': placements, 'endpoints': endpoints, 'cpu_cost': cost})
    c.require(runner.model_identities(options) == model, 'model input identity changed')
    c.verify_files(stage, plan['files'])
    save_new(output / 'completion.json', {'accepted': result['accepted'], 'cell_id': selected['cell_id'],
        'cell_result_sha256': c.read(output / 'cell-results/result.json')[1],
        'cpu_cost': cost, 'model_stable': True, 'input_files_stable': True})
    return 0


def retained_cells(plan, rows, replay, plan_binding, runner, legacy, evidence):
    return [replay.load_retained_cell({'cell_id': row['cell_id'], 'spec': row['spec']}, Path(row['output']),
        plan_binding=plan_binding, runner=runner, legacy=legacy, evidence=evidence) for row in rows]


def preceding_cells(plan, cell_id):
    ids = [row['cell_id'] for row in plan['cells']]
    c.require(ids == [row[0] for row in c.CELL_ORDER] and cell_id in ids, 'closed ordered campaign roster')
    return plan['cells'][:ids.index(cell_id)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--plan-sha256', required=True)
    parser.add_argument('--cell-id', choices=[row[0] for row in c.CELL_ORDER])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--supervise', action='store_true')
    mode.add_argument('--execute', action='store_true')
    mode.add_argument('--summarize', action='store_true')
    args = parser.parse_args()
    os.umask(0o077)
    c.require(socket.gethostname() == c.HOST and os.getuid() == c.UID, 'authorized native host/UID')
    stage = c.stage_name(D)
    c.require(args.plan == stage / 'plan.json', 'one owned plan location')
    plan = c.decode(c.read(args.plan, args.plan_sha256, 32 * 1024**2)[0])
    c.require((args.cell_id is None) == args.summarize, 'cell ID required only for a predeclared execution')
    loaded, counter, evidence, cell, options, identities = c.validate_plan(plan, stage, args.cell_id or 'counter-A')
    _, _, legacy, _, profile, runner, supervisor, _ = loaded
    selected = None if args.summarize else c.selected_cell(plan, args.cell_id)
    if args.execute:
        return execute(stage, plan, selected, loaded, counter, evidence, cell, options, identities)
    lock = os.open(stage / 'native.lock', os.O_CREAT | os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
    try:
        item = os.fstat(lock)
        c.require(stat.S_ISREG(item.st_mode) and item.st_uid == c.UID and item.st_nlink == 1
                  and stat.S_IMODE(item.st_mode) == 0o600, 'owned native launch lock')
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        replay = c.module(stage / 'measurement/native_campaign_replay.py',
                          plan['sources']['measurement/native_campaign_replay.py'])
        plan_binding = {'path': str(args.plan), 'sha256': args.plan_sha256}
        before = c.read(args.plan, args.plan_sha256)[0]
        if args.summarize:
            rows = retained_cells(plan, plan['cells'], replay, plan_binding, runner, legacy, evidence)
            report = cell.evaluate_campaign(plan['comparison'], rows[2:], rows[:2])
            report['cpu_cost_by_cell'] = {row['cell_id']: row['completion']['cpu_cost'] for row in rows}
            c.require(c.read(args.plan, args.plan_sha256)[0] == before, 'plan changed during summary')
            c.verify_files(stage, plan['files'])
            save_new(stage / 'campaign-report.json', report)
            return 0
        prior = retained_cells(plan, preceding_cells(plan, args.cell_id), replay, plan_binding,
                               runner, legacy, evidence)
        mechanism = replay.validate_counter_pair(prior[:2]) if len(prior) >= 2 else None
        output = Path(selected['output'])
        fresh_output(stage, output)
        save_new(output / 'prior-cells.json', [{'cell_id': row['cell_id'],
            'completion_sha256': row['outer']['completion_sha256']} for row in prior])
        if mechanism is not None:
            save_new(output / 'mechanism-pair.json', mechanism)
        admission = Admission(stage, plan, supervisor, profile, output)
        os.environ.update(PATH='/usr/bin:/bin', LC_ALL='C', PYTHONDONTWRITEBYTECODE='1', PYTHONNOUSERSITE='1',
                          FERRIC_V14_SUPERVISOR_PID=str(os.getpid()))
        for key in ('LD_PRELOAD', 'LD_LIBRARY_PATH', 'PYTHONPATH'):
            os.environ.pop(key, None)
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        resource.setrlimit(resource.RLIMIT_AS, (128 * 1024**3, 128 * 1024**3))
        stopped = []
        for number in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            signal.signal(number, lambda value, _frame: stopped.append(value) if not stopped else None)
        argv = ['/usr/bin/python3', '-I', '-B', str(stage / 'run_stage.py'), '--execute', '--plan', str(args.plan),
                '--plan-sha256', args.plan_sha256, '--cell-id', args.cell_id]
        result = supervisor.supervise(argv, output,
            lambda initial: admission.check('preflight' if initial else 'monitor', initial=initial),
            lambda: stopped[0] if stopped else None, duration=1500, grace=20, kill_wait=5, interval=3)
        try:
            result['postflight'] = admission.check('postflight')
            c.require(c.read(args.plan, args.plan_sha256)[0] == before, 'plan changed during run')
            c.verify_files(stage, plan['files'])
            if result['status'] == 0:
                c.require(result['cleanup_ok'] and result['child_reaped'] and not result['term_sent']
                    and not result['kill_sent'] and not result['errors'], 'clean unsignaled outer completion')
                complete = c.decode(c.read(output / 'completion.json', maximum=1024**2)[0])
                c.require(complete.get('accepted') is True, 'complete accepted cell receipt required')
                result['completion_sha256'] = c.read(output / 'completion.json')[1]
        except BaseException as error:
            result['errors'].append('postflight: ' + type(error).__name__ + ': ' + str(error))
            result.update(status=125, cleanup_ok=False)
        runner.save(output / 'launch-supervisor.json', result)
        with (output / 'launch.status').open('x') as stream:
            stream.write(str(result['status']) + '\n')
        return result['status']
    finally:
        os.close(lock)


if __name__ == '__main__':
    raise SystemExit(main())
