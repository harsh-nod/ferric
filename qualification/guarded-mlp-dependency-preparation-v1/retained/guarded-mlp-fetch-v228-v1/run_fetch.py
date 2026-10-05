"""Explicit bounded locked nightly-source fetch, separate from offline vendoring."""
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
ROOT = E / 'guarded-mlp-fetch-v228-v1'
CPU = E / 'guarded-mlp-segment-cpu-v228-v3'
OUT, TMP, TARGET = (ROOT / name for name in ('evidence', 'tmp', 'target'))
CARGO_HOME = Path('/home/harmenon/.cargo')
REGISTRY = CARGO_HOME / 'registry'
CACHE_BASE_BYTES = 0
FAILED_VENDOR = E / 'guarded-mlp-vendor-v228-v1/evidence/failed.json'
FAILED_VENDOR_SHA = '254428ec279074de41a85da4b2585b40177b435ec04a17a06a88a5de8544179d'
HELPER_SHA = '98ab3cedf7d2d1ae283105260f1e40ee70da37cf5d69356d55938301b4f21cf2'
COMPLETE_SHA = 'd1565161acfe89c4ac9136767961c7b273cd1eb6562a4e9e666ea4a3e306de87'
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
    module = types.ModuleType('guarded_cpu_v3_vendor_helpers')
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
    total = cache_total = count = 0
    for root in (TMP, TARGET, REGISTRY):
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
                    size = (Path(directory) / name).lstat().st_size
                except FileNotFoundError:
                    continue
                if root == REGISTRY:
                    cache_total += size
                else:
                    total += size
                count += 1
                require(count <= 200000, 'scratch inventory bound')
    return total + max(0, cache_total - CACHE_BASE_BYTES)


def registry_snapshot(h):
    result = {}
    for section in ('src', 'cache', 'index'):
        root = REGISTRY / section
        result[section] = tree(h, root, 200000) if os.path.lexists(root) else {}
    require(sum(map(len, result.values())) <= 200000, 'registry snapshot file-count bound')
    return result


def registry_changes(before, after):
    result = {}
    for section in ('src', 'cache', 'index'):
        old, new = before[section], after[section]
        removed = sorted(set(old) - set(new))
        changed = sorted(name for name in set(old) & set(new) if old[name] != new[name])
        if section != 'index':
            require(not removed and not changed, 'existing registry ' + section + ' bodies changed')
        result[section] = dict(added=sorted(set(new) - set(old)), removed=removed, changed=changed)
    return result


def qualified_cache_sources(h, expected):
    for root, wanted in expected.items():
        path = Path(root)
        require(path.is_relative_to(CARGO_HOME), 'qualified dependency outside Cargo cache')
        actual = {str(p.relative_to(path)): h.pin(p) for p in h.files_below(path, packed=False)}
        require(actual == wanted, 'qualified external dependency changed: ' + root)
    return True


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
    require(time.monotonic() + LEAF_SECONDS < work_deadline, 'insufficient full fetch leaf reserve')
    require(shutil.disk_usage(ROOT).free >= LIVE_FREE and scratch_bytes() <= CACHE_LIMIT,
            'pre-fetch storage bound')
    argv = [str(TOOLCHAIN / 'bin/cargo'), 'fetch', '--locked',
            '--manifest-path', str(LIBRARY / 'Cargo.toml')]
    command = ['/usr/bin/prlimit', '--as=' + str(AS_LIMIT), '--cpu=' + str(LEAF_SECONDS),
               '--fsize=' + str(FILE_LIMIT), '--core=0', '--', *argv]
    command_pin = h.save('fetch.command.json', dict(argv=command, cwd=str(ROOT), env=env,
                                                   wall_timeout_seconds=LEAF_SECONDS))
    stdout, stderr = OUT / 'fetch.stdout', OUT / 'fetch.stderr'
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
            require(not deferred, 'termination before fetch spawn')
            child = subprocess.Popen(command, cwd=ROOT, env=env, stdin=subprocess.DEVNULL,
                                     stdout=so, stderr=se, start_new_session=True)
            require(os.getpgid(child.pid) == child.pid, 'owned fetch process-group identity')
            h.save('fetch.started.json', dict(pid=child.pid, pgid=child.pid, argv=command))
            deadline = time.monotonic() + LEAF_SECONDS
            next_storage = 0.0
            while child.poll() is None:
                require(not deferred, 'deferred fetch termination: ' + repr(deferred))
                now = time.monotonic()
                if now >= deadline:
                    timed_out = True
                    break
                require(max(os.fstat(so.fileno()).st_size, os.fstat(se.fileno()).st_size) <= STREAM_LIMIT,
                        'fetch stream cap')
                if now >= next_storage:
                    require(shutil.disk_usage(ROOT).free >= LIVE_FREE and scratch_bytes() <= CACHE_LIMIT,
                            'fetch storage floor/cache cap')
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
    row = dict(label='fetch', command=command_pin, argv=command,
               pid=child.pid if child else None, pgid=child.pid if child else None,
               exit_code=child.returncode if child else None, timed_out=timed_out,
               forced_cleanup=forced, exception=exception, observed_signals=deferred,
               adopted_reaped=reaped, reaped=child is not None and child.returncode is not None,
               natural_exit=child is not None and child.returncode is not None
                   and not timed_out and not forced and exception is None,
               process_group_absent=group_absent, elapsed_ns=time.monotonic_ns() - start,
               stdout=h.pin(stdout), stderr=h.pin(stderr))
    leaves.append(row)
    h.save('fetch.result.json', row)
    require(row['natural_exit'] and row['exit_code'] == 0 and row['reaped'] and group_absent,
            'fetch did not finish naturally, successfully and fully reaped')
    require(max(row['stdout']['bytes'], row['stderr']['bytes']) <= STREAM_LIMIT
            and shutil.disk_usage(ROOT).free >= LIVE_FREE and scratch_bytes() <= CACHE_LIMIT,
            'post-fetch storage bound')
    signal.setitimer(signal.ITIMER_REAL, max(0.001, work_deadline - time.monotonic()))


def main():
    global CACHE_BASE_BYTES
    started = time.monotonic()
    hard_deadline = started + WHOLE_SECONDS
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1,
            'usage: python3 -B run_fetch.py')
    require(Path(__file__).resolve() == ROOT / 'run_fetch.py'
            and ROOT.resolve() == ROOT and not ROOT.is_symlink(), 'exact fresh fetch namespace')
    require(not any(os.path.lexists(path) for path in (OUT, TMP, TARGET)), 'fetch outputs already exist')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'unexpected fetch host/UID')
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
    cache_before = cache_after = cache_delta = external_sources = None
    failure = None
    complete = None

    def read_pin(path, expected=None):
        value = h.pin(path)
        require(expected is None or value == expected, 'pinned input differs: ' + str(path))
        input_pins[str(path)] = value
        return value

    try:
        read_pin(ROOT / 'run_fetch.py')
        helper_pin = read_pin(CPU / 'run_cpu.py')
        require(helper_pin['sha256'] == HELPER_SHA, 'helper changed after load')
        complete_pin = read_pin(CPU / 'evidence/complete.json')
        require(complete_pin['sha256'] == COMPLETE_SHA, 'qualified CPU completion differs')
        complete = json.loads((CPU / 'evidence/complete.json').read_bytes())
        require(complete['passed'] is True and complete['failure'] is None
                and complete['postcheck_errors'] == [] and complete['source_unchanged'] is True
                and complete['tests']['passed'] == 27 and complete['tests']['failed'] == 0
                and complete['tests']['ignored'] == 0 and len(complete['phases']) == 10,
                'qualified CPU gate differs')
        failed_pin = read_pin(FAILED_VENDOR)
        require(failed_pin['sha256'] == FAILED_VENDOR_SHA, 'prior offline vendor failure differs')
        failed = json.loads(FAILED_VENDOR.read_bytes())
        require(failed['passed'] is False and failed['offline'] is True
                and failed['postcheck_errors'] == [] and len(failed['phases']) == 1
                and failed['phases'][0]['exit_code'] == 101
                and failed['phases'][0]['natural_exit'] is True,
                'actual separate offline refusal required')
        read_pin(Path(complete['final_sources']['path']), complete['final_sources'])
        cpu_before = h.sources()
        require(cpu_before == json.loads(Path(complete['final_sources']['path']).read_bytes()),
                'CPU qualified sources/lock have changed')
        read_pin(Path(complete['input_manifest']['path']), complete['input_manifest'])
        for value in complete['tool_pins'].values():
            read_pin(Path(value['path']), value)
        for value in complete['artifacts'].values():
            read_pin(Path(value['pin']['path']), value['pin'])
        dependency_pin = complete['raw']['dependencies-after.json']
        read_pin(Path(dependency_pin['path']), dependency_pin)
        external_sources = json.loads(Path(dependency_pin['path']).read_bytes())
        qualified_cache_sources(h, external_sources)
        rust_before = tree(h, RUST_SOURCE)
        for name, (count, sha) in [('Cargo.lock', RUST_LOCK), ('Cargo.toml', RUST_MANIFEST)]:
            value = rust_before['library/' + name]
            require((value['bytes'], value['sha256']) == (count, sha), 'nightly library input differs')
        config_before = configurations(h)
        h.save('cpu-sources-before.json', cpu_before)
        h.save('rust-src-before.json', rust_before)
        cache_before = registry_snapshot(h)
        CACHE_BASE_BYTES = sum(value['bytes'] for section in cache_before.values() for value in section.values())
        h.save('registry-before.json', cache_before)
        env = dict(HOME='/home/harmenon', PATH=str(TOOLCHAIN / 'bin') + ':/usr/bin:/bin',
                   CARGO_HOME=str(CARGO_HOME), CARGO_TARGET_DIR=str(TARGET),
                   CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='false',
                   CARGO_NET_RETRY='0', CARGO_HTTP_TIMEOUT='60',
                   LD_LIBRARY_PATH=str(TOOLCHAIN / 'lib'), TMPDIR=str(TMP),
                   RUSTC=str(TOOLCHAIN / 'bin/rustc'), RUSTDOC=str(TOOLCHAIN / 'bin/rustdoc'),
                   RUST_BACKTRACE='1', ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
        # Explicit fetch only: no host fixture binding, compilation or vendor retry.
        owned(h, env, leaves, hard_deadline)
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
        def cache_postcheck():
            nonlocal cache_after, cache_delta
            if cache_before is None:
                return True
            cache_after = registry_snapshot(h)
            h.save('registry-after.json', cache_after)
            cache_delta = registry_changes(cache_before, cache_after)
            h.save('registry-changes.json', cache_delta)
            return True
        for label, action in (
            ('CPU source/lock', lambda: source_postcheck('cpu-sources-after.json', h.sources, cpu_before)),
            ('nightly rust-src/lock', lambda: source_postcheck('rust-src-after.json',
                lambda: tree(h, RUST_SOURCE), rust_before)),
            ('configuration', lambda: configurations(h) == config_before if config_before is not None else True),
            ('all literal inputs/tools/products', lambda: all(h.pin(Path(path)) == value for path, value in input_pins.items())),
            ('registry source/archive custody and index delta', cache_postcheck),
            ('qualified external sources', lambda: qualified_cache_sources(h, external_sources)
                if external_sources is not None else True),
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
    result = dict(schema='ferric-guarded-mlp-nightly-fetch-v1', passed=passed,
                  failure=failure, postcheck_errors=postchecks, phases=leaves,
                  controller=h.pin(ROOT / 'run_fetch.py'), inputs=input_pins,
                  qualified_cpu_complete=complete_pin if complete is not None else None,
                  dependency_commit=h.COMMIT, cargo_home=str(CARGO_HOME),
                  prior_offline_failure=input_pins.get(str(FAILED_VENDOR)),
                  registry_changes=cache_delta, online_fetch=True, offline=False,
                  cargo_cache_metadata_mutation_allowed=True,
                  existing_registry_sources_and_archives_unchanged=cache_delta is not None and not postchecks,
                  config_installed=False, vendor_executed=False,
                  input_sources_unchanged=cpu_before is not None and rust_before is not None and not postchecks,
                  configurations=config_before,
                  host_fixture_crate_binding_used=False, compiler_hsaco_reproduced=False,
                  gpu_execution=False, full_model_acceptance=False, performance_claim=False,
                  elapsed_seconds=time.monotonic() - started,
                  limits=dict(whole_seconds=WHOLE_SECONDS, leaf_seconds=LEAF_SECONDS,
                              cleanup_reserve_seconds=CLEANUP_RESERVE, address_space_bytes=AS_LIMIT,
                              cpu_seconds=LEAF_SECONDS, additional_cache_bytes=CACHE_LIMIT,
                              initial_registry_bytes=CACHE_BASE_BYTES, stream_bytes=STREAM_LIMIT,
                              file_bytes=FILE_LIMIT, affinity=[8, 9], nice=10, cargo_jobs=2,
                              initial_free_bytes=START_FREE, live_free_bytes=LIVE_FREE),
                  raw={path.name: h.pin(path) for path in sorted(OUT.iterdir()) if path.is_file()})
    h.save('complete.json' if passed else 'failed.json', result)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
