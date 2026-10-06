"""Observe exact formatter equivalence; do not edit sources or run project code."""
import ctypes
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import signal
import stat
import subprocess
import sys
import tarfile
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PARENT = E / 'guarded-mlp-parent-cpu-v228-v3'
WORKER = E / 'guarded-mlp-worker-cpu-v228-v6'
OUT = E / 'guarded-mlp-parent-shared-format-v228-v1'
ARCHIVE = E / 'guarded-mlp-parent-shared-format-evidence-v228-v1.tar.gz'
PARENT_PIN = dict(bytes=3697764, sha256='cfe7adb9e0d375191f41c37e36c9712eeed72c1b7879cd6f96a69ba2359e1c2a')
WORKER_PIN = dict(bytes=1559924, sha256='927e6519923ab44aa5f5616886ce9b539ed5b5f77804a48fe5d42f6a37974cd2')
TOOL = Path('/home/harmenon/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/bin/rustfmt')
TOOL_PIN = dict(bytes=5281712, sha256='a9137d0c198ceb6c72193d517d3c9007b3ec7a90d3d10ec6889773eca48261b4')
PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/src/'
SOURCES = {
    'finite_guarded_mlp_decode_wire_v1.rs': (
        dict(bytes=17323, sha256='ec9d3b4f6bfa85688250a3b390acd3ae2fe9958f0a99f1bf2165745e4772fb2e'),
        dict(bytes=19298, sha256='d42d5e297f92a7594e8472cb112acca96cff987a67d40f0490e5869606dfeb99')),
    'finite_guarded_mlp_decode_wire_v1_tests.rs': (
        dict(bytes=9057, sha256='ca24ac6ecafe7a6e59d1ddb9e8509c0608bd13a6fe31e382b73cfbd7d7274217'),
        dict(bytes=10779, sha256='a7c6d8d92640c1429f5f27c870ec36d0b969fb71298c620f3b339cf8f63a3e11')),
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
            value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def file_pin(path):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and 0 <= before.st_size <= 256 << 20, 'ordinary bounded input: ' + str(path))
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'opened input changed')
        for block in iter(lambda: stream.read(1 << 20), b''):
            digest.update(block)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed during hash')
    require(stamp(path.lstat()) == stamp(before), 'input changed after hash')
    return dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())


def read(path, expected):
    before = file_pin(path)
    require(compact(before) == compact(expected) and before['bytes'] <= 8 << 20, 'bound input pin')
    body = path.read_bytes()
    require(pin(body) == compact(before) and file_pin(path) == before, 'bounded input read drift')
    return body, before


def save(name, value):
    body = value if isinstance(value, bytes) else (json.dumps(value, indent=2, sort_keys=True) + '\n').encode()
    require('/' not in name and 0 <= len(body) <= 8 << 20, 'ordinary bounded output')
    with (OUT / name).open('xb') as stream:
        stream.write(body)
    return file_pin(OUT / name)


def group_exists(pid):
    try:
        os.killpg(pid, 0)
        return True
    except ProcessLookupError:
        return False


def group_members(pgid):
    result = []
    entries = list(Path('/proc').iterdir())
    require(len(entries) <= 32768, 'bounded process observation')
    for entry in entries:
        if not entry.name.isdecimal():
            continue
        try:
            body = (entry / 'stat').read_text()
        except (FileNotFoundError, ProcessLookupError):
            continue
        require(len(body) <= 8192 and ') ' in body, 'bounded process stat')
        fields = body.rsplit(') ', 1)[1].split()
        require(len(fields) >= 4, 'complete process stat')
        if int(fields[2]) == pgid:
            result.append(int(entry.name))
    return sorted(result)


def interrupted(signum, frame):
    raise RuntimeError('formatter observation interrupted: ' + str(signum))


def run(label, body, env, input_path):
    argv = ['/usr/bin/prlimit', '--as=1073741824', '--cpu=30', '--fsize=1048576', '--core=0', '--',
            str(TOOL), '--edition', '2024', '--config', 'skip_children=true', '--emit', 'stdout']
    command = save(label + '.command.json', dict(argv=argv, cwd=str(OUT), env=env,
        stdin=pin(body), wall_timeout_seconds=30, address_space_bytes=1 << 30,
        stream_bytes=1 << 20, source_paths_supplied=False))
    started, proc, failure, forced, timed_out = time.monotonic(), None, None, False, False
    leader_reaped = False
    adopted, deferred = [], []
    handled = (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM)
    previous = {sig: signal.getsignal(sig) for sig in handled}

    def defer(signum, frame):
        deferred.append(signum)

    with input_path.open('rb') as stdin, (OUT / (label + '.stdout')).open('xb') as stdout, (OUT / (label + '.stderr')).open('xb') as stderr:
        require(stdin.read() == body, 'retained exact stdin body')
        stdin.seek(0)
        for sig in handled:
            signal.signal(sig, defer)
        try:
            require(not deferred, 'termination requested before formatter spawn')
            proc = subprocess.Popen(argv, cwd=OUT, env=env, stdin=stdin,
                stdout=stdout, stderr=stderr, start_new_session=True, close_fds=True)
            require(os.getpgid(proc.pid) == proc.pid, 'owned formatter process group')
            save(label + '.started.json', dict(pid=proc.pid, pgid=proc.pid, argv=argv))
            while True:
                require(not deferred, 'termination requested during formatter')
                terminal = os.waitid(os.P_PID, proc.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                if terminal is not None:
                    break
                if time.monotonic() - started >= 30:
                    timed_out = True
                    raise RuntimeError('formatter30-second deadline')
                time.sleep(0.02)
        except BaseException as error:
            failure = repr(error)
        finally:
            try:
                if proc is not None:
                    # Keep the leader waitable until no signaling can remain necessary.
                    terminal = os.waitid(os.P_PID, proc.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                    members = group_members(proc.pid)
                    require(proc.pid in members, 'unreaped owned leader missing')
                    if terminal is None or members != [proc.pid]:
                        forced = True
                        os.killpg(proc.pid, signal.SIGKILL)
                    until = time.monotonic() + 5
                    while True:
                        members = group_members(proc.pid)
                        require(proc.pid in members, 'owned leader lost before group retirement')
                        for child_pid in members:
                            if child_pid == proc.pid:
                                continue
                            try:
                                pid, status = os.waitpid(child_pid, os.WNOHANG)
                            except ChildProcessError:
                                continue
                            if pid:
                                adopted.append(dict(pid=pid, status=status))
                        terminal = os.waitid(os.P_PID, proc.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                        if terminal is not None and group_members(proc.pid) == [proc.pid]:
                            break
                        require(time.monotonic() < until, 'owned formatter group failed to retire')
                        time.sleep(0.01)
                    proc.wait(timeout=max(0.001, until - time.monotonic()))
                    leader_reaped = True
                    require(not group_exists(proc.pid), 'formatter group remains after final reap')
            except BaseException as error:
                failure = (failure or '') + ' retirement failure: ' + repr(error)
                if proc is not None and not leader_reaped:
                    # A failed /proc observation must not bypass owned cleanup.
                    forced = True
                    try:
                        os.killpg(proc.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    try:
                        proc.wait(timeout=5)
                        leader_reaped = True
                        until = time.monotonic() + 5
                        while True:
                            try:
                                pid, status = os.waitpid(-proc.pid, os.WNOHANG)
                            except ChildProcessError:
                                break
                            if pid:
                                adopted.append(dict(pid=pid, status=status))
                            else:
                                require(time.monotonic() < until, 'fallback adopted-child reap deadline')
                                time.sleep(0.01)
                    except BaseException as cleanup_error:
                        failure += ' fallback cleanup failure: ' + repr(cleanup_error)
            finally:
                for sig, handler in previous.items():
                    signal.signal(sig, handler)
    if deferred:
        failure = (failure or '') + ' deferred signals: ' + repr(deferred)
    require(proc is not None, 'formatter process was not spawned: ' + str(failure))
    row = dict(label=label, argv=argv, command=command, pid=proc.pid, pgid=proc.pid,
        exit_code=proc.returncode, timed_out=timed_out, forced_cleanup=forced,
        reaped=proc.returncode is not None, process_group_absent=not group_exists(proc.pid),
        natural_exit=not forced and not timed_out and failure is None and proc.returncode is not None,
        adopted_reaped=adopted, observed_signals=deferred,
        exception=failure, elapsed_seconds=time.monotonic() - started,
        stdout=file_pin(OUT / (label + '.stdout')), stderr=file_pin(OUT / (label + '.stderr')))
    save(label + '.result.json', row)
    return row


def main():
    started = time.monotonic()
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B verify_parent_shared_format_v1.py')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'qualified host and UID')
    require(E.resolve(strict=True) == E and not os.path.lexists(OUT) and not os.path.lexists(ARCHIVE),
            'fresh exact diagnostic namespace')
    os.umask(0o077)
    resource.setrlimit(resource.RLIMIT_AS, (1 << 30, 1 << 30))
    resource.setrlimit(resource.RLIMIT_FSIZE, (16 << 20, 16 << 20))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    os.sched_setaffinity(0, {8, 9})
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'formatter subreaper')
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, interrupted)
    signal.alarm(180)
    parent_body, parent_pin = read(PARENT / 'evidence/complete.json', PARENT_PIN)
    worker_body, worker_pin = read(WORKER / 'evidence/complete.json', WORKER_PIN)
    parent, worker = json.loads(parent_body), json.loads(worker_body)
    require(all(value['passed'] is True and value['postcheck_errors'] == []
            and value['source_unchanged'] is True for value in (parent, worker)), 'two actual qualified CPU source parents')
    require(compact(worker['tool_pins']['rustfmt']) == TOOL_PIN
            and worker['tool_pins']['rustfmt']['path'] == str(TOOL), 'qualified nightly formatter')
    observed = {parent_pin['path']: parent_pin, worker_pin['path']: worker_pin}
    for row in worker['tool_pins'].values():
        require(file_pin(Path(row['path'])) == row, 'qualified formatter/tool-library pin')
        observed[row['path']] = row
    source_bodies, source_pins = {}, {}
    for name, expected in SOURCES.items():
        key = PREFIX + name
        source_bodies[name] = []
        for root, receipt, bound in ((PARENT, parent, expected[0]), (WORKER, worker, expected[1])):
            row = receipt['final_sources'][key]
            require(compact(row) == bound and row['path'] == str(root / key), 'exact qualified shared source row')
            body, actual = read(root / key, bound)
            source_bodies[name].append(body)
            source_pins[str(root / key)] = actual
            observed[str(root / key)] = actual
    config_paths = [path / name for path in (OUT, *OUT.parents) for name in ('rustfmt.toml', '.rustfmt.toml')]
    require(all(not os.path.lexists(path) for path in config_paths), 'no ambient formatter configuration')
    OUT.mkdir(mode=0o700)
    save('parent-complete.json', parent_body)
    save('worker-complete.json', worker_body)
    controller_body, controller_pin = read(Path(__file__).resolve(), file_pin(Path(__file__).resolve()))
    observed[controller_pin['path']] = controller_pin
    save('verify_parent_shared_format_v1.py', controller_body)
    env = dict(HOME=str(OUT), PATH=str(TOOL.parent) + ':/usr/bin:/bin',
        LD_LIBRARY_PATH=str(TOOL.parent.parent / 'lib'), TMPDIR=str(OUT),
        ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
    phases, comparisons, errors, failure = [], [], [], None
    try:
        for index, (name, bodies) in enumerate(source_bodies.items()):
            original, expected = bodies
            save('parent-' + name, original)
            save('worker-' + name, expected)
            row = run('format-' + str(index), original, env, OUT / ('parent-' + name))
            phases.append(row)
            require(row['exit_code'] == 0 and row['natural_exit'] and row['reaped']
                    and row['process_group_absent'] and not row['timed_out'] and not row['forced_cleanup']
                    and row['exception'] is None and row['adopted_reaped'] == [], 'natural clean formatter process')
            output, _ = read(OUT / ('format-' + str(index) + '.stdout'), pin(expected))
            require(output == expected, 'formatter stdout differs from qualified worker body')
            comparisons.append(dict(source=name, parent=pin(original), worker=pin(expected),
                formatted_stdout=pin(output), byte_equal=True))
    except BaseException as error:
        failure = repr(error)
    remaining = started + 180 - time.monotonic()
    require(remaining > 0, 'whole formatter observation deadline before postchecks')
    signal.setitimer(signal.ITIMER_REAL, remaining)
    for path, expected in observed.items():
        require(time.monotonic() < started + 180, 'whole formatter postcheck deadline')
        try:
            require(file_pin(Path(path)) == expected, 'source/receipt/tool drift')
        except BaseException as error:
            errors.append(str(path) + ': ' + repr(error))
    require(time.monotonic() < started + 180, 'whole formatter retention deadline')
    require(all(not os.path.lexists(path) for path in config_paths), 'formatter configuration appeared')
    failure = failure or ('postcheck failed' if errors else None)
    passed = failure is None and len(comparisons) == len(phases) == 2
    result = dict(schema='ferric-guarded-mlp-parent-shared-formatter-equivalence-v1',
        passed=passed, failure=failure, postcheck_errors=errors, parent_terminal=PARENT_PIN,
        worker_terminal=WORKER_PIN, tool_pins=worker['tool_pins'], source_inputs=source_pins,
        phases=phases, comparisons=comparisons, elapsed_seconds=time.monotonic() - started,
        formatter_equivalence_only=True, source_unchanged=not errors, formatter_execution=True,
        sources_edited=False, build_execution=False, test_execution=False, gpu_execution=False,
        full_model_acceptance=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False)
    save('complete.json' if passed else 'failed.json', result)
    files = {p.name: compact(file_pin(p)) for p in sorted(OUT.iterdir())}
    require(len(files) <= 18 and sum(row['bytes'] for row in files.values()) <= 16 << 20,
            'bounded formatter-only retention closure')
    save('manifest.json', dict(schema='ferric-guarded-mlp-parent-shared-formatter-retained-v1',
        files=files, passed=passed, formatter_equivalence_only=True, sources_edited=False,
        build_execution=False, test_execution=False, gpu_execution=False, performance_claim=False))
    with ARCHIVE.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive:
            for path in sorted(OUT.iterdir()):
                body, _ = read(path, file_pin(path))
                info = tarfile.TarInfo(path.name)
                info.size, info.mode, info.mtime = len(body), 0o600, 0
                archive.addfile(info, io.BytesIO(body))
    signal.alarm(0)
    print(json.dumps(dict(passed=passed, archive=file_pin(ARCHIVE), members=len(files) + 1,
        formatter_equivalence_only=True, sources_edited=False, gpu_execution=False), sort_keys=True))
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
