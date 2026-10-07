"""One bounded, explicitly reviewed two-forward engineering observation."""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import select
import shutil
import signal
import stat
import subprocess
import time
import types

ROOT = Path('/home/harmenon/ferric-asrock-42')
EVIDENCE = ROOT / 'evidence/finite-two-forward-v223'
MODEL = '/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model'
HOST = 'smci350-rck-g03-b19-03'
UID = 9661
DEVICES = [16366993098680759275, 10838076764495710945]
HELPER_SHA = '6016c30f46aa32abf9d175f329a0110e86cbf79d1363f1803dba3c86c1bad50a'
MODEL_ID = 'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a'
BUNDLE_ID = '6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b'
GIB = 1024**3
STREAM_CAP = 64 * 1024**2
DEADLINE = 4000
CHILD_DEADLINE_MS = 3600000
MARKER = re.compile(rb'finite engineering owned child pid=([1-9][0-9]*) pgid=([1-9][0-9]*); no native setup acknowledged')
ENV = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8',
    LC_ALL='C.UTF-8', TZ='UTC', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1',
    MKL_NUM_THREADS='1', HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def keys(value, expected):
    require(type(value) is dict and set(value) == set(expected.split()), 'closed JSON fields')


def uint(value, maximum=(1 << 64) - 1):
    require(type(value) is int and 0 <= value <= maximum, 'unsigned integer')
    return value


def digest(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value), 'SHA256 spelling')
    return value


def byte_array(value, count=None):
    require(type(value) is list and (count is None or len(value) == count), 'byte array shape')
    require(all(type(v) is int and 0 <= v <= 255 for v in value), 'byte array type')
    return bytes(value)


def parse(data):
    def object_pairs(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, 'duplicate JSON field')
            result[key] = value
        return result
    return json.loads(data, object_pairs_hook=object_pairs,
        parse_constant=lambda _: require(False, 'nonfinite JSON number'))


def identity(value):
    return (value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def read_file(path, maximum, retain=False):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical regular path')
    result, total, output = hashlib.sha256(), 0, bytearray()
    with path.open('rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_size <= maximum, 'regular-file byte limit')
        while True:
            block = stream.read(1024 * 1024)
            if not block:
                break
            total += len(block)
            require(total <= maximum, 'growing input limit')
            result.update(block)
            if retain:
                output.extend(block)
        after = os.fstat(stream.fileno())
    final = path.lstat()
    require(stat.S_ISREG(final.st_mode) and identity(before) == identity(after) == identity(final)
        and total == before.st_size, 'input changed during hash')
    return dict(path=str(path), bytes=total, sha256=result.hexdigest()), bytes(output)


def checked(record, maximum=512 * 1024**2, retain=False):
    keys(record, 'path bytes sha256')
    require(type(record['path']) is str and Path(record['path']).is_relative_to(ROOT), 'input root')
    uint(record['bytes'], maximum)
    digest(record['sha256'])
    observed, data = read_file(record['path'], maximum, retain)
    require(observed == record, 'pinned file mismatch: ' + record['path'])
    return data


def rust_pin(value):
    keys(value, 'path bytes sha256')
    return dict(path=value['path'], bytes=uint(value['bytes'], 512 * 1024**2),
        sha256=byte_array(value['sha256'], 32).hex())


def save(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, sort_keys=True, indent=2, allow_nan=False)
        stream.write('\n')


def pin(path):
    return read_file(path, STREAM_CAP)[0]


def progress(value):
    try:
        print(json.dumps(value), flush=True)
    except OSError:
        pass


def validate_request(request):
    keys(request, 'schema source worker images expected_bundle_id expected_model_id device_ids session mode input_tokens dispatch_timeout_ms child_deadline_ms')
    require(request['schema'] == 'FerricFiniteTwoForwardRequestV1' and request['source'] == MODEL,
        'request schema/model source')
    require(byte_array(request['expected_model_id'], 32).hex() == MODEL_ID
        and byte_array(request['expected_bundle_id'], 32).hex() == BUNDLE_ID, 'authentic model/bundle')
    require(byte_array(request['session'], 32) != bytes(32), 'nonzero session')
    require(type(request['device_ids']) is list and request['device_ids'] == DEVICES
        and all(type(v) is int for v in request['device_ids']), 'exact selected devices')
    require(request['mode'] in ('teacher_forced', 'autoregressive'), 'input mode')
    require(type(request['input_tokens']) is list and len(request['input_tokens']) ==
        (2 if request['mode'] == 'teacher_forced' else 1), 'two-forward input count')
    for token in request['input_tokens']:
        uint(token, 151935)
    require(1 <= uint(request['dispatch_timeout_ms'], 10000), 'dispatch deadline')
    require(uint(request['child_deadline_ms']) == CHILD_DEADLINE_MS, 'fixed child deadline')
    keys(request['images'], 'prefix mlp residual tail')
    return [rust_pin(request['worker'])] + [rust_pin(request['images'][name])
        for name in ('prefix', 'mlp', 'residual', 'tail')]


def available_ram():
    values = dict(line.split(':', 1) for line in Path('/proc/meminfo').read_text().splitlines())
    value, units = values['MemAvailable'].split()
    require(units == 'kB', 'MemAvailable units')
    return int(value) * 1024


def resources(initial=False):
    result = dict(free_bytes=shutil.disk_usage(ROOT).free, mem_available_bytes=available_ram())
    require(result['free_bytes'] >= (40 if initial else 38) * GIB, 'free-space floor')
    require(result['mem_available_bytes'] >= (64 if initial else 32) * GIB, 'available-RAM floor')
    return result


def limits():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_AS, (32 * GIB, 32 * GIB))
    resource.setrlimit(resource.RLIMIT_FSIZE, (STREAM_CAP, STREAM_CAP))


def proc(pid):
    try:
        path = Path('/proc') / str(pid)
        raw = (path / 'stat').read_text()
        fields = raw[raw.rfind(')') + 2:].split()
        return dict(pid=pid, uid=path.stat().st_uid, state=fields[0], ppid=int(fields[1]),
            pgid=int(fields[2]), sid=int(fields[3]), starttime=int(fields[19]))
    except (FileNotFoundError, ProcessLookupError, PermissionError):
        return None


def processes():
    return {row['pid']: row for path in Path('/proc').iterdir() if path.name.isdecimal()
        if (row := proc(int(path.name))) is not None}


def token(row):
    return row['pid'], row['starttime']


def same_process(left, right):
    return right is not None and all(left[key] == right[key] for key in ('pid', 'uid', 'starttime'))


def subreaper():
    require(hasattr(os, 'pidfd_open') and hasattr(signal, 'pidfd_send_signal')
        and hasattr(os, 'WNOWAIT'), 'Linux pidfd/waitid required')
    require(not any(row['ppid'] == os.getpid() for row in processes().values()),
        'supervisor must start without children')
    require(len(list(Path('/proc/self/task').iterdir())) == 1
        and signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL,
        'single exclusive waiter with ordinary zombie retention required')
    probe = os.pidfd_open(os.getpid(), 0)
    os.close(probe)
    libc = ctypes.CDLL(None, use_errno=True)
    require(libc.prctl(36, 1, 0, 0, 0) == 0, 'set child subreaper')


class OwnedProcesses:
    """Only observed descendants/adoptees; kernel pidfds, never numeric signals."""
    def __init__(self):
        self.parent = None
        self.records = {}
        self.events = []
        self.supervisor = os.getpid()
        self.root = None

    def attach(self, parent):
        self.parent = parent
        self.root = proc(parent.pid)
        require(self.root is not None and self.root['ppid'] == self.supervisor
            and self.root['pgid'] == self.root['sid'] == parent.pid, 'new parent session')
        self.add(self.root, 'spawned-parent')

    def add(self, row, reason):
        key = token(row)
        if key in self.records:
            return False
        fd = os.pidfd_open(row['pid'], 0)
        current = proc(row['pid'])
        if not same_process(row, current):
            os.close(fd)
            return False
        require(row['uid'] == self.root['uid'], 'owned process changed UID')
        self.records[key] = dict(identity=row, fd=fd, reason=reason, reaped=False)
        self.events.append(dict(event='owned', reason=reason, identity=row))
        return True

    def discover(self):
        rows = processes()
        changed = True
        while changed:
            changed = False
            live = {pid for (pid, _), record in self.records.items()
                if same_process(record['identity'], rows.get(pid))}
            for row in rows.values():
                if token(row) in self.records or row['pid'] == self.supervisor:
                    continue
                inherited = row['ppid'] in live
                adopted = row['ppid'] == self.supervisor and row['starttime'] >= self.root['starttime']
                if inherited or adopted:
                    # The standalone subreaper has no pre-existing children and
                    # creates only this parent. Adoption therefore retains lineage.
                    require(row['uid'] == self.root['uid'], 'descendant UID changed')
                    try:
                        added = self.add(row, 'ancestry' if inherited else 'subreaper-adoption')
                    except ProcessLookupError:
                        continue
                    changed |= added
        return rows

    def alive(self, record):
        return not select.select([record['fd']], [], [], 0)[0]

    def signal_all(self, number):
        self.discover()
        for record in self.records.values():
            if self.alive(record):
                try:
                    signal.pidfd_send_signal(record['fd'], number)
                    self.events.append(dict(event='signal', signal=number, identity=record['identity']))
                except ProcessLookupError:
                    pass

    def reap_adopted(self):
        for record in self.records.values():
            row = proc(record['identity']['pid'])
            if (record['identity']['pid'] != self.parent.pid and same_process(record['identity'], row)
                    and row['ppid'] == self.supervisor):
                try:
                    pid, status = os.waitpid(row['pid'], os.WNOHANG)
                except ChildProcessError:
                    continue
                if pid:
                    record['reaped'] = True
                    self.events.append(dict(event='reaped-adoptee', identity=record['identity'], status=status))

    def cleanup(self):
        # Parent remains unreaped until its separate-group descendants have been
        # discovered. Readiness on a pidfd cannot refer to a subsequently reused PID.
        self.discover()
        forced = any(self.alive(record) for record in self.records.values())
        if forced:
            self.signal_all(signal.SIGTERM)
        start = time.monotonic()
        while True:
            self.discover()
            self.reap_adopted()
            alive = any(self.alive(record) for record in self.records.values())
            if not alive:
                break
            if time.monotonic() - start >= 2:
                self.signal_all(signal.SIGKILL)
            require(time.monotonic() - start < 60, 'owned cleanup stalled; no quiescence/complete claim')
            time.sleep(.05)
        code = self.parent.wait(timeout=1)
        for _ in range(100):
            self.discover()
            self.reap_adopted()
            rows = processes()
            owned_present = any(same_process(record['identity'], rows.get(record['identity']['pid']))
                for record in self.records.values())
            if not owned_present:
                break
            time.sleep(.05)
        groups = sorted({record['identity']['pgid'] for record in self.records.values()})
        rows = processes()
        require(not any(same_process(record['identity'], rows.get(record['identity']['pid']))
            for record in self.records.values()), 'owned process remains after reaping')
        require(not any(row['ppid'] == self.supervisor or row['pgid'] in groups
            for row in rows.values()), 'owned group absent check failed (no signalling of unknown members)')
        return dict(exit_code=code, cleanup_signalled=forced, owned_groups=groups,
            owned_groups_absent=True, owned_processes_reaped=True, lineage=self.events)

    def close_fds(self):
        for record in self.records.values():
            os.close(record['fd'])


def announcements(data):
    rows = []
    for line in data.splitlines():
        if b'finite engineering owned child ' in line:
            match = MARKER.fullmatch(line)
            require(match is not None, 'malformed child process announcement')
            pid, pgid = map(int, match.groups())
            require(pid == pgid, 'child must lead its own process group')
            rows.append(pid)
    require(len(rows) <= 1, 'multiple child process announcements')
    return rows


def emergency_reap(parent, events):
    """Last-resort cleanup after pidfd/tracker initialization failure.

    Only direct children are signalled. This standalone, single-threaded
    subreaper is their exclusive waiter and has not reaped them: their PIDs
    cannot be reused between waitid(WNOWAIT) and kill. No historical PGID or
    non-child PID is ever used. Orphan descendants become direct children.
    """
    start, groups = time.monotonic(), {parent.pid}
    while True:
        for row in processes().values():
            if row['ppid'] != os.getpid():
                continue
            require(row['uid'] == os.getuid(), 'emergency child UID')
            try:
                exited = os.waitid(os.P_PID, row['pid'], os.WEXITED | os.WNOHANG | os.WNOWAIT)
            except ChildProcessError:
                continue
            current = proc(row['pid'])
            require(same_process(row, current) and current['ppid'] == os.getpid(),
                'emergency direct-child identity')
            groups.add(row['pgid'])
            if exited is None:
                os.kill(row['pid'], signal.SIGKILL)
                events.append(dict(event='emergency-unreaped-direct-child-signal',
                    signal=signal.SIGKILL, identity=row))
            elif row['pid'] == parent.pid:
                parent.wait()
                events.append(dict(event='emergency-parent-reaped', identity=row))
            else:
                _, status = os.waitpid(row['pid'], 0)
                events.append(dict(event='emergency-adoptee-reaped', identity=row, status=status))
        try:
            os.waitid(os.P_ALL, 0, os.WEXITED | os.WNOHANG | os.WNOWAIT)
        except ChildProcessError:
            break
        require(time.monotonic() - start < 60, 'emergency cleanup stalled; no complete claim')
        time.sleep(.05)
    require(parent.returncode is not None, 'emergency parent must be reaped')
    require(not any(row['pgid'] in groups for row in processes().values()), 'emergency group remains')
    return dict(exit_code=parent.returncode, cleanup_signalled=True, owned_groups=sorted(groups),
        owned_groups_absent=True, owned_processes_reaped=True, emergency_cleanup=True, lineage=events)


def run_owned(out, argv, environment, monitor, deadline=DEADLINE, apply_limits=True):
    """CPU fake-process tests call this same supervisor without any GPU helpers."""
    start = time.monotonic()
    reason, tracker, outcome = None, OwnedProcesses(), None
    with (out / 'stdout').open('xb') as stdout, (out / 'stderr').open('xb') as stderr:
        parent = subprocess.Popen(argv, cwd=ROOT, env=environment, stdin=subprocess.DEVNULL,
            stdout=stdout, stderr=stderr, start_new_session=True,
            preexec_fn=limits if apply_limits else None)
        try:
            tracker.attach(parent)
            save(out / 'started.json', dict(parent=tracker.root, supervisor_pid=os.getpid()))
            progress(dict(phase='owned-parent-started', pid=parent.pid))
            while True:
                tracker.discover()
                require(time.monotonic() - start < deadline, 'root execution deadline')
                require(max(os.fstat(stdout.fileno()).st_size, os.fstat(stderr.fileno()).st_size)
                    < STREAM_CAP, 'stream cap')
                monitor()
                # Read only complete lines while the parent may still append.
                current_stderr = (out / 'stderr').read_bytes()
                complete_lines = current_stderr[:current_stderr.rfind(b'\n') + 1]
                ids = announcements(complete_lines)
                if ids:
                    # The worker may have spawned after the earlier snapshot.
                    tracker.discover()
                    actual = proc(ids[0])
                    if actual is not None:
                        record = tracker.records.get(token(actual))
                        require(record is not None and actual['pgid'] == actual['pid']
                            and actual['pid'] != parent.pid
                            and same_process(actual, proc(actual['pid'])),
                            'announced worker live lineage/group')
                # WNOWAIT observes exit without releasing the leader PID.
                if os.waitid(os.P_PID, parent.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT):
                    break
                time.sleep(.2)
        except BaseException as error:
            reason = type(error).__name__ + ': ' + str(error)
        finally:
            # Repeated transport/operator signals cannot interrupt owned cleanup.
            saved_handlers = {number: signal.signal(number, signal.SIG_IGN)
                for number in (signal.SIGTERM, signal.SIGINT)}
            try:
                try:
                    require(tracker.root is not None and token(tracker.root) in tracker.records,
                        'parent pidfd initialization incomplete')
                    outcome = tracker.cleanup()
                except BaseException as primary_cleanup_error:
                    reason = reason or ('primary cleanup failed: ' + repr(primary_cleanup_error))
                    outcome = emergency_reap(parent, tracker.events)
            except BaseException as error:
                save(out / 'cleanup-failure.json', dict(error=repr(error), lineage=tracker.events,
                    complete=False, owned_groups_absent=False))
                raise
            finally:
                tracker.close_fds()
                for number, handler in saved_handlers.items():
                    signal.signal(number, handler)
    outcome.update(reason=reason, elapsed_seconds=time.monotonic() - start,
        stdout=pin(out / 'stdout'), stderr=pin(out / 'stderr'))
    save(out / 'result.json', outcome)
    return outcome


def terminal(words, tasks):
    require(type(words) is list and len(words) == tasks + 6, 'terminal state extent')
    for word in words:
        uint(word, (1 << 32) - 1)
    require(words[:4] == [1, 0, (1 << tasks) - 1, (1 << tasks) - 1]
        and words[5] == 0 and words[6:] == [64] * tasks, 'terminal state flags/arrivals')
    require(all(((words[4] >> (2 * i)) & 3) in (1, 2) for i in range(tasks)), 'terminal owners')
    require(tasks == 16 or words[4] >> (2 * tasks) == 0, 'terminal owner upper bits')


def validate_observation(observed, request, child_pid):
    keys(observed, 'schema request child_pid registration_sha256 source_program_sha256 upload_manifest_sha256 setup_commands forwards close child_stderr child_exit_zero process_group_absent native_closed gpu_execution numerical_acceptance performance_claim production_authority')
    require(observed['schema'] == 'FerricFiniteTwoForwardObservationV1' and observed['request'] == request
        and uint(observed['child_pid']) == child_pid, 'observation request/child')
    for key in ('registration_sha256', 'source_program_sha256', 'upload_manifest_sha256'):
        require(byte_array(observed[key], 32) != bytes(32), 'observation identity')
    require(uint(observed['setup_commands']) > 0, 'setup command count')
    require(len(byte_array(observed['child_stderr'])) <= 2 * 1024**2, 'child stderr bound')
    for name in ('child_exit_zero', 'process_group_absent', 'native_closed', 'gpu_execution'):
        require(observed[name] is True, 'successful observation lifecycle')
    for name in ('numerical_acceptance', 'performance_claim', 'production_authority'):
        require(observed[name] is False, 'observation authority boundary')
    require(type(observed['forwards']) is list and len(observed['forwards']) == 2, 'two forwards')
    output_tokens, payload_pins = [], []
    for index, row in enumerate(observed['forwards']):
        keys(row, 'response payload')
        response = row['response']
        validate_response(response, request, observed['registration_sha256'], index + 1, False)
        event = response['event']
        keys(event, 'status generation position input_token output_token embedding_ns layers tail_ns payload')
        token_in = request['input_tokens'][index] if request['mode'] == 'teacher_forced' or index == 0 else output_tokens[0]
        require(event['status'] == 'completed' and uint(event['generation']) == index + 1
            and uint(event['position']) == index and uint(event['input_token']) == token_in, 'forward ordering/tokens')
        output_tokens.append(uint(event['output_token'], 151935))
        require(type(event['layers']) is list and len(event['layers']) == 36, 'all layer states')
        for layer in event['layers']:
            keys(layer, 'prefix_states mlp_states paired_ns')
            for name, tasks in (('prefix_states', 16), ('mlp_states', 5)):
                require(type(layer[name]) is list and len(layer[name]) == 2, 'rank states')
                for words in layer[name]:
                    terminal(words, tasks)
            require(type(layer['paired_ns']) is list and len(layer['paired_ns']) == 4, 'paired timing shape')
            for pair in layer['paired_ns']:
                timings(pair, 2)
        timings(event['embedding_ns'], 2)
        timings(event['tail_ns'], 3)
        raw = byte_array(row['payload'], 606976)
        require(all((raw[i + 1] & 0x7f) != 0x7f or not (raw[i] & 0x80)
            for i in range(0, len(raw), 2)), 'nonfinite BF16 payload')
        def part(data):
            return dict(bytes=len(data), sha256=list(hashlib.sha256(data).digest()))
        expected = dict(layer_hidden=[part(raw[i * 8192:(i + 1) * 8192]) for i in range(36)],
            final_normalized=part(raw[36 * 8192:37 * 8192]), logits=part(raw[37 * 8192:]), total=part(raw))
        require(event['payload'] == expected, 'all payload digest/extent joins')
        payload_pins.append(expected['total'])
    validate_response(observed['close'], request, observed['registration_sha256'], 3, True)
    require(observed['close']['event'] == dict(status='closed', completed_forwards=2), 'healthy explicit Close')
    return dict(completed_forwards=2, completed_layer_rank_pairs=144, checked_terminal_states=288,
        output_tokens=output_tokens, payloads=payload_pins,
        structural_observation_only=True, numerical_acceptance=False, performance_claim=False,
        production_authority=False)


def timings(value, count):
    require(type(value) is list and len(value) == count, 'timing extent')
    for item in value:
        uint(item)


def validate_response(value, request, registration, sequence, closed):
    keys(value, 'protocol id device_ids session registration event native_closed gpu_execution numerical_acceptance performance_claim production_authority')
    require(uint(value['protocol']) == 1 and uint(value['id']) == sequence
        and value['device_ids'] == request['device_ids'] and value['session'] == request['session']
        and value['registration'] == registration and value['native_closed'] is closed
        and value['gpu_execution'] is True, 'response identity/lifecycle')
    require(all(value[name] is False for name in ('numerical_acceptance', 'performance_claim', 'production_authority')),
        'response authority boundary')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('plan', type=Path)
    parser.add_argument('sha256')
    args = parser.parse_args()
    os.umask(0o077)
    require(os.getuid() == UID and os.uname().nodename == HOST, 'supervisor host/UID')
    require(os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'affinity/nice')
    plan_pin, raw = read_file(args.plan, 65536, True)
    require(plan_pin['sha256'] == digest(args.sha256), 'plan SHA')
    plan = parse(raw)
    keys(plan, 'schema output parent request topology_helper evidence')
    require(plan['schema'] == 'ferric-p223-gpu-supervisor-plan-v1', 'plan schema')
    out = Path(plan['output'])
    require(out.parent == EVIDENCE and out.parent.resolve(strict=True) == out.parent
        and re.fullmatch('[a-z][a-z0-9-]{1,90}', out.name), 'exclusive output path')
    checked(plan['parent'])
    require(os.access(plan['parent']['path'], os.X_OK), 'parent executable')
    request = parse(checked(plan['request'], 65536, True))
    request_pins = validate_request(request)
    pins = [plan_pin, plan['parent'], plan['request'], plan['topology_helper'], *request_pins]
    require(type(plan['evidence']) is list and 1 <= len(plan['evidence']) <= 32, 'explicit build/review evidence')
    pins.extend(plan['evidence'])
    for record in pins:
        checked(record)
    require(os.access(request_pins[0]['path'], os.X_OK), 'worker executable')
    require(plan['topology_helper']['path'] == str(ROOT / 'evidence/resident-output-tp2-v217/run_p217_mi350.py')
        and plan['topology_helper']['sha256'] == HELPER_SHA, 'exact reviewed topology helper')
    helper = types.ModuleType('p223_pinned_topology')
    helper.__file__ = plan['topology_helper']['path']
    exec(compile(checked(plan['topology_helper'], STREAM_CAP, True), helper.__file__, 'exec'), helper.__dict__)
    require(helper.ROOT == ROOT and [row[2] for row in helper.DEVICES] == DEVICES, 'topology helper identity')
    subreaper()
    initial = resources(True)
    out.mkdir()
    self_pin = pin(Path(__file__).resolve())
    save(out / 'input-pins.json', dict(controller=self_pin, pins=pins, initial_resources=initial))
    argv = [plan['parent']['path'], '--request', plan['request']['path'], '--allow-unauthenticated-machine-code']
    save(out / 'command.json', dict(argv=argv, env=ENV, cwd=str(ROOT), root_deadline_seconds=DEADLINE,
        child_deadline_ms=CHILD_DEADLINE_MS, address_space_bytes=32 * GIB, stream_file_cap_bytes=STREAM_CAP,
        core_bytes=0, affinity=[8, 9], nice=10, initial_disk_floor_bytes=40 * GIB,
        ongoing_disk_floor_bytes=38 * GIB, initial_ram_floor_bytes=64 * GIB,
        ongoing_ram_floor_bytes=32 * GIB, gpu_execution_requested=True,
        numerical_acceptance=False, performance_claim=False, production_authority=False))
    before = []
    for index in range(3):
        before.append(helper.topology_sample())
        save(out / f'topology-before-{index}.json', before[-1])
        if index < 2:
            time.sleep(5)
    helper.require_idle(before)
    resources(True)
    for record in pins:
        checked(record)
    def interrupted(number, _frame):
        raise RuntimeError('supervisor interrupted by signal ' + str(number))
    old = {number: signal.signal(number, interrupted) for number in (signal.SIGTERM, signal.SIGINT)}
    try:
        result = run_owned(out, argv, ENV, resources)
    finally:
        for number, handler in old.items():
            signal.signal(number, handler)
    save(out / 'topology-after.json', helper.topology_sample())
    require(result['reason'] is None and result['exit_code'] == 0 and not result['cleanup_signalled'],
        'one-shot failed; no retry or successful observation')
    observed = parse(checked(result['stdout'], STREAM_CAP, True))
    child_ids = announcements(checked(result['stderr'], STREAM_CAP, True))
    require(len(child_ids) == 1, 'exactly one announced worker')
    # A fast child can be reaped by its parent between scans. In that case no
    # pidfd or live-executable observation is fabricated; the parent receipt and
    # group-absence check still bind its announced identifier.
    require(child_ids[0] != result['lineage'][0]['identity']['pid'], 'separate parent/worker identifiers')
    require(not any(row['pgid'] == child_ids[0] for row in processes().values()), 'announced worker group remains')
    structural = validate_observation(observed, request, child_ids[0])
    for record in pins + [self_pin, result['stdout'], result['stderr']]:
        checked(record)
    save(out / 'complete.json', dict(schema='ferric-p223-gpu-supervisor-complete-v1',
        status='CLOSED_TWO_FORWARD_STRUCTURAL_OBSERVATION', plan=plan_pin, controller=self_pin,
        pins=pins, command=pin(out / 'command.json'), started=pin(out / 'started.json'),
        result=pin(out / 'result.json'), stdout=result['stdout'], stderr=result['stderr'],
        topology=[pin(out / f'topology-before-{i}.json') for i in range(3)] + [pin(out / 'topology-after.json')],
        structural=structural, gpu_execution=True, numerical_acceptance=False,
        performance_claim=False, production_authority=False))
    progress(dict(complete=pin(out / 'complete.json')))


if __name__ == '__main__':
    main()
