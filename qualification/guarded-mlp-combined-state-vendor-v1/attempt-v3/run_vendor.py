"""One bounded offline vendor preparation, not checked compilation or emission."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import stat
import subprocess
import sys
import time
import tomllib
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-combined-state-vendor-v228-v3'
CPU = E / 'guarded-mlp-combined-state-cpu-v228-v3'
OUT, VENDOR, TMP, TARGET = (ROOT / name for name in ('evidence', 'vendor', 'tmp', 'target'))
HELPER_SHA = 'd59335b65cbde4b04ac003ad56e08c0f72fe58b3cf4bf4cfe65d88f8134970e3'  # Bind only the reviewed combined-state CPU controller.
COMPLETE_SHA = 'ebdcb5f36dcdadd77614f85b91f0ed510d1ea6a44be407ab18d73a1aa56af696'  # Bind only the actual successful combined-state CPU receipt.
TOOLCHAIN = Path('/home/harmenon/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu')
RUST_SOURCE = TOOLCHAIN / 'lib/rustlib/src/rust'
LIBRARY = RUST_SOURCE / 'library'
RUST_LOCK = (12033, 'c7fbe8811bd7b2a3737deb1bc5d1ec2ee6d631bc00221d3d8a58d05814b3e965')
RUST_MANIFEST = (2955, 'd1c133e6fa50400a81cd4342658596d7dfdff18e5d0cacd3d31b69e8991ee451')
WHOLE_SECONDS, LEAF_SECONDS, CLEANUP_RESERVE = 720, 600, 50
AS_LIMIT, FILE_LIMIT, CACHE_LIMIT, STREAM_LIMIT = 12 << 30, 1 << 30, 6 << 30, 64 << 20
START_FREE, LIVE_FREE = 40 << 30, 38 << 30
SIGNALS = (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM)


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
            value.st_uid, value.st_gid, value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def helper():
    path = CPU / 'run_cpu.py'
    require(path.resolve(strict=True) == path, 'noncanonical qualified helper path')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size < 1 << 20, 'helper type/extent')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC), 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'helper changed before read')
        body = stream.read()
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'helper changed during read')
    require(stamp(path.lstat()) == stamp(before) and hashlib.sha256(body).hexdigest() == HELPER_SHA,
            'qualified helper identity changed')
    module = types.ModuleType('guarded_combined_state_cpu_v1_vendor_helpers')
    module.__file__ = str(path)
    exec(compile(body, str(path), 'exec'), module.__dict__)
    return module


def tree(h, root, cap=20000):
    require(root.is_dir() and not root.is_symlink() and root.resolve() == root,
            'noncanonical source/output tree: ' + str(root))
    result = {}
    for directory, dirs, names in os.walk(root, followlinks=False,
                                          onerror=lambda error: (_ for _ in ()).throw(error)):
        for name in dirs:
            require(not (Path(directory) / name).is_symlink(), 'directory alias in retained tree')
        for name in names:
            path = Path(directory) / name
            require(len(result) < cap, 'tree file-count bound')
            result[str(path.relative_to(root))] = h.pin(path)
    return result


def scratch_bytes():
    total = count = 0
    for root in (VENDOR, TMP, TARGET):
        if not root.exists():
            continue
        def walk_error(error):
            path = Path(error.filename) if isinstance(error.filename, str) else None
            if (isinstance(error, FileNotFoundError) and path is not None
                    and path.is_absolute() and '..' not in path.parts
                    and path != root and path.is_relative_to(root)):
                return
            raise error
        for directory, _, names in os.walk(root, followlinks=False, onerror=walk_error):
            for name in names:
                try:
                    total += (Path(directory) / name).lstat().st_size
                except FileNotFoundError:
                    continue
                count += 1
                require(count <= 200000, 'scratch inventory bound')
    return total


def configurations(h):
    paths = {parent / '.cargo' / name
             for root in (ROOT, CPU / 'candidate', LIBRARY)
             for parent in (root, *root.parents)
             for name in ('config', 'config.toml')}
    paths |= {Path('/home/harmenon/.cargo') / name for name in ('config', 'config.toml')}
    values = {str(path): h.pin(path) if os.path.lexists(path) else None for path in sorted(paths)}
    require(not any(values.values()), 'inherited Cargo config refused')
    return values


def owned(h, env, leaves, hard_deadline):
    work_deadline = hard_deadline - CLEANUP_RESERVE
    require(time.monotonic() + LEAF_SECONDS < work_deadline, 'insufficient full vendor leaf reserve')
    require(shutil.disk_usage(ROOT).free >= LIVE_FREE and scratch_bytes() <= CACHE_LIMIT,
            'pre-vendor storage bound')
    argv = [str(TOOLCHAIN / 'bin/cargo'), 'vendor', '--offline', '--locked', '--versioned-dirs',
            '--manifest-path', str(CPU / 'candidate/Cargo.toml'),
            '--sync', str(LIBRARY / 'Cargo.toml'), str(VENDOR)]
    command = ['/usr/bin/prlimit', '--as=' + str(AS_LIMIT), '--cpu=' + str(LEAF_SECONDS),
               '--fsize=' + str(FILE_LIMIT), '--core=0', '--', *argv]
    command_pin = h.save('vendor.command.json', dict(argv=command, cwd=str(ROOT), env=env,
                                                   wall_timeout_seconds=LEAF_SECONDS))
    stdout, stderr = OUT / 'vendor.stdout', OUT / 'vendor.stderr'
    start, child, exception = time.monotonic_ns(), None, None
    timed_out = forced = False
    group_absent = False
    reaped, deferred = [], []
    with stdout.open('xb') as so, stderr.open('xb') as se:
        previous = {sig: signal.getsignal(sig) for sig in SIGNALS}
        def defer(signum, _frame):
            if len(deferred) < 16:
                deferred.append(signum)
        for sig in SIGNALS:
            signal.signal(sig, defer)
        try:
            require(not deferred, 'termination before vendor spawn')
            child = subprocess.Popen(command, cwd=ROOT, env=env, stdin=subprocess.DEVNULL,
                                     stdout=so, stderr=se, start_new_session=True)
            require(os.getpgid(child.pid) == child.pid, 'owned vendor process-group identity')
            h.save('vendor.started.json', dict(pid=child.pid, pgid=child.pid, argv=command))
            deadline = time.monotonic() + LEAF_SECONDS
            next_storage = 0.0
            while child.poll() is None:
                require(not deferred, 'deferred vendor termination: ' + repr(deferred))
                now = time.monotonic()
                if now >= deadline:
                    timed_out = True
                    break
                require(max(os.fstat(so.fileno()).st_size, os.fstat(se.fileno()).st_size) <= STREAM_LIMIT,
                        'vendor stream cap')
                if now >= next_storage:
                    require(shutil.disk_usage(ROOT).free >= LIVE_FREE and scratch_bytes() <= CACHE_LIMIT,
                            'vendor storage floor/cache cap')
                    next_storage = now + 2
                time.sleep(0.1)
        except BaseException as error:
            exception = repr(error)
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            try:
                if child is not None:
                    child.poll()
                    if child.returncode is not None:
                        reaped.extend(h.reap_group(child.pid))
                    if child.returncode is None or h.group_exists(child.pid):
                        forced = True
                        reaped.extend(h.stop_group(child, hard_deadline))
                    try:
                        child.wait(timeout=max(0.001, min(5, hard_deadline - time.monotonic())))
                    except subprocess.TimeoutExpired:
                        exception = (exception or '') + ' leader not reaped within cleanup bound'
                    if child.returncode is not None:
                        reaped.extend(h.reap_group(child.pid))
                    group_absent = not h.group_exists(child.pid)
            finally:
                for sig, handler in previous.items():
                    signal.signal(sig, handler)
    if deferred:
        exception = (exception or '') + ' deferred signals: ' + repr(deferred)
    if time.monotonic() >= work_deadline:
        timed_out = True
    row = dict(label='vendor', command=command_pin, argv=command,
               pid=child.pid if child else None, pgid=child.pid if child else None,
               exit_code=child.returncode if child else None, timed_out=timed_out,
               forced_cleanup=forced, exception=exception, observed_signals=deferred,
               adopted_reaped=reaped, reaped=child is not None and child.returncode is not None,
               natural_exit=child is not None and child.returncode is not None
                   and not timed_out and not forced and exception is None,
               process_group_absent=group_absent, elapsed_ns=time.monotonic_ns() - start,
               stdout=h.pin(stdout), stderr=h.pin(stderr))
    leaves.append(row)
    h.save('vendor.result.json', row)
    require(row['natural_exit'] and row['exit_code'] == 0 and row['reaped'] and group_absent,
            'vendor did not finish naturally, successfully and fully reaped')
    require(max(row['stdout']['bytes'], row['stderr']['bytes']) <= STREAM_LIMIT
            and shutil.disk_usage(ROOT).free >= LIVE_FREE and scratch_bytes() <= CACHE_LIMIT,
            'post-vendor storage bound')
    signal.setitimer(signal.ITIMER_REAL, max(0.001, work_deadline - time.monotonic()))


def main():
    require(all(type(value) is str and re.fullmatch(r'[0-9a-f]{64}', value)
                for value in (HELPER_SHA, COMPLETE_SHA)),
            'actual combined-state CPU/controller bindings required')
    started = time.monotonic()
    hard_deadline = started + WHOLE_SECONDS
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1,
            'usage: python3 -B run_vendor.py')
    require(Path(__file__).resolve() == ROOT / 'run_vendor.py'
            and ROOT.resolve() == ROOT and not ROOT.is_symlink(), 'exact fresh vendor namespace')
    require(not any(os.path.lexists(path) for path in (OUT, VENDOR, TMP, TARGET)), 'vendor outputs already exist')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'unexpected vendor host/UID')
    require(shutil.disk_usage(ROOT).free >= START_FREE, 'initial40GiB floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected initial nice level')
    if priority == 0:
        os.nice(10)
    require(os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'affinity/nice mismatch')
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, AS_LIMIT),
                      (resource.RLIMIT_FSIZE, FILE_LIMIT)):
        soft, hard = resource.getrlimit(kind)
        effective = min([cap] + [n for n in (soft, hard) if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (effective, effective))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup failed')
    h = helper()
    h.OUT = OUT
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    for sig in SIGNALS:
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, hard_deadline - CLEANUP_RESERVE - time.monotonic()))
    leaves, postchecks, input_pins = [], [], {}
    cpu_before = rust_before = config_before = None
    vendor_snapshot = config_output = None
    failure = None
    complete = None

    def read_pin(path, expected=None):
        value = h.pin(path)
        require(expected is None or value == expected, 'pinned input differs: ' + str(path))
        input_pins[str(path)] = value
        return value

    try:
        read_pin(ROOT / 'run_vendor.py')
        helper_pin = read_pin(CPU / 'run_cpu.py')
        require(helper_pin['sha256'] == HELPER_SHA, 'helper changed after load')
        complete_pin = read_pin(CPU / 'evidence/complete.json')
        require(complete_pin['sha256'] == COMPLETE_SHA, 'qualified CPU completion differs')
        complete = json.loads((CPU / 'evidence/complete.json').read_bytes())
        require(complete['schema'] == 'ferric-guarded-mlp-combined-state-cpu-v1'
                and complete['passed'] is True and complete['failure'] is None
                and complete['postcheck_errors'] == [] and complete['source_unchanged'] is True
                and type(h.EXPECTED_TESTS) is tuple and len(h.EXPECTED_TESTS) > 27
                and complete['tests']['passed'] == len(h.EXPECTED_TESTS)
                and complete['tests']['names'] == list(h.EXPECTED_TESTS)
                and complete['tests']['failed'] == 0
                and complete['tests']['ignored'] == 0 and len(complete['phases']) == 10,
                'qualified CPU gate differs')
        read_pin(Path(complete['final_sources']['path']), complete['final_sources'])
        cpu_before = h.sources()
        require(cpu_before == json.loads(Path(complete['final_sources']['path']).read_bytes()),
                'CPU qualified sources/lock have changed')
        read_pin(Path(complete['input_manifest']['path']), complete['input_manifest'])
        for value in complete['tool_pins'].values():
            read_pin(Path(value['path']), value)
        for value in complete['artifacts'].values():
            read_pin(Path(value['pin']['path']), value['pin'])
        rust_before = tree(h, RUST_SOURCE)
        for name, (count, sha) in [('Cargo.lock', RUST_LOCK), ('Cargo.toml', RUST_MANIFEST)]:
            value = rust_before['library/' + name]
            require((value['bytes'], value['sha256']) == (count, sha), 'nightly library input differs')
        config_before = configurations(h)
        h.save('cpu-sources-before.json', cpu_before)
        h.save('rust-src-before.json', rust_before)
        env = dict(HOME='/home/harmenon', PATH=str(TOOLCHAIN / 'bin') + ':/usr/bin:/bin',
                   CARGO_HOME='/home/harmenon/.cargo', CARGO_TARGET_DIR=str(TARGET),
                   CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true',
                   LD_LIBRARY_PATH=str(TOOLCHAIN / 'lib'), TMPDIR=str(TMP),
                   RUSTC=str(TOOLCHAIN / 'bin/rustc'), RUSTDOC=str(TOOLCHAIN / 'bin/rustdoc'),
                   RUST_BACKTRACE='1', ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
        # Vendoring is not host compilation; no synthetic crate-binding fixture is passed.
        owned(h, env, leaves, hard_deadline)
        config_output = tomllib.loads((OUT / 'vendor.stdout').read_text())
        require(config_output.get('source', {}).get('vendored-sources', {}).get('directory') == str(VENDOR),
                'vendor stdout does not bind the selected fresh directory')
        vendor_snapshot = tree(h, VENDOR, 200000)
        require(vendor_snapshot and any(name.endswith('/.cargo-checksum.json') for name in vendor_snapshot),
                'empty or unrecognizable vendor tree')
        h.save('vendor-files.json', vendor_snapshot)
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        def source_postcheck(label, observe, expected):
            if expected is None:
                return True
            after = observe()
            h.save(label, after)
            return after == expected
        for label, action in (
            ('CPU source/lock', lambda: source_postcheck('cpu-sources-after.json', h.sources, cpu_before)),
            ('nightly rust-src/lock', lambda: source_postcheck('rust-src-after.json',
                lambda: tree(h, RUST_SOURCE), rust_before)),
            ('configuration', lambda: configurations(h) == config_before if config_before is not None else True),
            ('all literal inputs/tools/products', lambda: all(h.pin(Path(path)) == value for path, value in input_pins.items())),
            ('vendor output', lambda: tree(h, VENDOR, 200000) == vendor_snapshot if vendor_snapshot is not None else True),
        ):
            try:
                remaining = hard_deadline - 5 - time.monotonic()
                require(remaining > 0, 'postcheck whole deadline')
                signal.setitimer(signal.ITIMER_REAL, remaining)
                require(action(), label + ' changed')
            except BaseException as error:
                postchecks.append(label + ': ' + repr(error))
            finally:
                signal.setitimer(signal.ITIMER_REAL, 0)
    passed = failure is None and not postchecks
    result = dict(schema='ferric-guarded-mlp-combined-state-vendor-preparation-v1', passed=passed,
                  failure=failure, postcheck_errors=postchecks, phases=leaves,
                  controller=h.pin(ROOT / 'run_vendor.py'), inputs=input_pins,
                  qualified_cpu_complete=complete_pin if complete is not None else None,
                  dependency_commit=h.COMMIT, vendor_directory=str(VENDOR),
                  vendor_files=h.pin(OUT / 'vendor-files.json') if (OUT / 'vendor-files.json').exists() else None,
                  generated_config=config_output, config_installed=False, offline=True,
                  input_sources_unchanged=cpu_before is not None and rust_before is not None and not postchecks,
                  configurations=config_before,
                  host_fixture_crate_binding_used=False, compiler_hsaco_reproduced=False,
                  gpu_execution=False, full_model_acceptance=False, performance_claim=False,
                  elapsed_seconds=time.monotonic() - started,
                  limits=dict(whole_seconds=WHOLE_SECONDS, leaf_seconds=LEAF_SECONDS,
                              cleanup_reserve_seconds=CLEANUP_RESERVE, address_space_bytes=AS_LIMIT,
                              cpu_seconds=LEAF_SECONDS, cache_bytes=CACHE_LIMIT, stream_bytes=STREAM_LIMIT,
                              file_bytes=FILE_LIMIT, affinity=[8, 9], nice=10, cargo_jobs=2,
                              initial_free_bytes=START_FREE, live_free_bytes=LIVE_FREE),
                  raw={path.name: h.pin(path) for path in sorted(OUT.iterdir()) if path.is_file()})
    h.save('complete.json' if passed else 'failed.json', result)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
