"""Bounded readelf/ldd audit of the seven actually couriered compiler tools."""
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

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
TOOLS = E / 'guarded-mlp-compiler-tools-v228-v1'
OUT = E / 'guarded-mlp-compiler-tools-audit-v228-v2'
SCRIPT = E / 'audit_guarded_mlp_compiler_tools_v228_v2.py'
MANIFEST_SHA = 'e22e8654ff094cc449ad09a9457bcbe5c07e311e3eb46635e40cf7f3cd5888e3'
NIGHTLY_LIB = Path('/home/harmenon/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/lib')
NAMES = ('cargo-fe2o3', 'clang-22', 'fe2o3-engineering-lld-proxy',
         'fe2o3-llvm-link-worker', 'fe2o3-rustc-extract',
         'librustc_codegen_fe2o3.so', 'lld')
NIGHTLY = {
    'libLLVM.so.22.1-rust-1.96.0-nightly': (199520544, '8af284bb5ae923ac175ddb3f7b9ad16f1f733f7f5f2779e0c9e8c68ef9ba162b'),
    'librustc_driver-7bb70639c3ace5a4.so': (152936640, 'a0aa61a461841224222b0064f9d77a84fe6d3410745d3d96ceaade6ee679cade'),
    'libLLVM-22-rust-1.96.0-nightly.so': (43, 'd43c716e9a7f6e673b4021e1ced69208a21e2433f0f90b49a8701f8be7c056be'),
}
SIGNALS = (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM)
AS_LIMIT, STREAM_LIMIT, FILE_LIMIT = 12 << 30, 1 << 20, 1 << 30
FREE_FLOOR, WHOLE_SECONDS, CLEANUP_RESERVE = 40 << 30, 600, 50
INPUTS, ALIASES = {}, {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_uid,
            value.st_gid, value.st_nlink, value.st_size,
            value.st_mtime_ns, value.st_ctime_ns)


def pin(path, track=True, cap=FILE_LIMIT):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path,
            'noncanonical input: ' + str(path))
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= cap,
            'not a bounded ordinary file: ' + str(path))
    digest = hashlib.sha256()
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    with os.fdopen(fd, 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed before read')
        for block in iter(lambda: stream.read(1 << 20), b''):
            digest.update(block)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed during read')
    require(stamp(path.lstat()) == stamp(before), 'input changed after read')
    value = dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())
    if track:
        require(str(path) not in INPUTS or INPUTS[str(path)] == value, 'conflicting input pin')
        INPUTS[str(path)] = value
    return value


def resolved(path):
    original = str(path)
    path = Path(path)
    require(path.is_absolute(), 'nonabsolute library/tool path')
    actual = path.resolve(strict=True)
    require(original not in ALIASES or ALIASES[original] == str(actual), 'resolution changed')
    ALIASES[original] = str(actual)
    return pin(actual)


def save(name, value):
    path = OUT / name
    with path.open('x') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')
    return pin(path, track=False)


def group_exists(pgid):
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False


def reap_adopted(child):
    rows = []
    child.poll()
    if child.returncode is None:
        return rows
    while True:
        try:
            pid, status = os.waitpid(-child.pid, os.WNOHANG)
        except ChildProcessError:
            break
        if pid == 0:
            break
        rows.append(dict(pid=pid, wait_status=status))
    return rows


def interrupted(signum, _frame):
    raise RuntimeError('interrupted by signal ' + str(signum))


def owned(label, argv, env, leaves, hard_deadline):
    require(shutil.disk_usage(E).free >= FREE_FLOOR, '40 GiB live storage floor')
    require(time.monotonic() + 30 + CLEANUP_RESERVE < hard_deadline,
            'insufficient whole-run reserve for leaf and cleanup')
    command = ['/usr/bin/prlimit', '--as=' + str(AS_LIMIT), '--cpu=30',
               '--fsize=' + str(STREAM_LIMIT), '--core=0', '--', *argv]
    command_pin = save(label + '.command.json', dict(argv=command, cwd=str(OUT),
                                                    env=env, wall_seconds=30))
    stdout, stderr = OUT / (label + '.stdout'), OUT / (label + '.stderr')
    start, child, exception = time.monotonic_ns(), None, None
    timed_out = forced = False
    reaped, deferred = [], []
    group_absent = False
    with stdout.open('xb') as so, stderr.open('xb') as se:
        previous = {sig: signal.getsignal(sig) for sig in SIGNALS}
        def defer(signum, _frame):
            if len(deferred) < 16:
                deferred.append(signum)
        for sig in SIGNALS:
            signal.signal(sig, defer)
        try:
            require(not deferred, 'termination requested before spawn')
            child = subprocess.Popen(command, cwd=OUT, env=env, stdin=subprocess.DEVNULL,
                                     stdout=so, stderr=se, start_new_session=True)
            require(os.getpgid(child.pid) == child.pid, 'owned process group identity')
            save(label + '.started.json', dict(pid=child.pid, pgid=child.pid, argv=command))
            deadline = time.monotonic() + 30
            while child.poll() is None:
                require(not deferred, 'termination during owned leaf: ' + repr(deferred))
                require(shutil.disk_usage(E).free >= FREE_FLOOR, '40 GiB live storage floor')
                if time.monotonic() >= deadline:
                    timed_out = True
                    break
                time.sleep(0.05)
        except BaseException as error:
            exception = repr(error)
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            try:
                if child is not None:
                    reaped.extend(reap_adopted(child))
                    if child.returncode is None or group_exists(child.pid):
                        forced = True
                        for sig, allowance in ((signal.SIGTERM, 10), (signal.SIGKILL, 20)):
                            try:
                                os.killpg(child.pid, sig)
                            except ProcessLookupError:
                                pass
                            deadline = min(hard_deadline - 5, time.monotonic() + allowance)
                            while time.monotonic() < deadline:
                                reaped.extend(reap_adopted(child))
                                if child.returncode is not None and not group_exists(child.pid):
                                    break
                                time.sleep(0.02)
                            if child.returncode is not None and not group_exists(child.pid):
                                break
                    try:
                        child.wait(timeout=max(0.001, min(5, hard_deadline - time.monotonic())))
                    except subprocess.TimeoutExpired:
                        exception = (exception or '') + ' leader not reaped'
                    reaped.extend(reap_adopted(child))
                    group_absent = not group_exists(child.pid)
            finally:
                for sig, handler in previous.items():
                    signal.signal(sig, handler)
    if deferred:
        exception = (exception or '') + ' deferred signals: ' + repr(deferred)
    row = dict(label=label, command=command_pin, pid=child.pid if child else None,
               pgid=child.pid if child else None, exit_code=child.returncode if child else None,
               timed_out=timed_out, forced_cleanup=forced, exception=exception,
               observed_signals=deferred, adopted_reaped=reaped,
               natural_exit=child is not None and child.returncode is not None
                   and not timed_out and not forced and exception is None,
               reaped=child is not None and child.returncode is not None,
               process_group_absent=group_absent, elapsed_ns=time.monotonic_ns() - start,
               stdout=pin(stdout, False, STREAM_LIMIT), stderr=pin(stderr, False, STREAM_LIMIT))
    leaves.append(row)
    save(label + '.result.json', row)
    require(row['natural_exit'] and row['exit_code'] == 0 and row['reaped'] and group_absent,
            label + ' did not exit naturally, successfully and fully reaped')
    require(shutil.disk_usage(E).free >= FREE_FLOOR, '40 GiB post-leaf storage floor')
    signal.setitimer(signal.ITIMER_REAL, max(0.001, hard_deadline - CLEANUP_RESERVE - time.monotonic()))
    return stdout.read_text()


def libraries(readelf, ldd, shared_object):
    needed = re.findall(r'\(NEEDED\).*Shared library: \[([^\]\n]+)\]', readelf)
    interpreters = re.findall(r'\[Requesting program interpreter: ([^\]\n]+)\]', readelf)
    require(needed and len(needed) == len(set(needed)), 'missing or duplicate DT_NEEDED')
    require(len(interpreters) == (0 if shared_object else 1), 'unexpected PT_INTERP count')
    require(ldd.strip() and 'not found' not in ldd and 'not a dynamic executable' not in ldd,
            'empty or unresolved ldd output')
    named, direct, virtual = {}, [], []
    for line in ldd.splitlines():
        if not line.strip():
            continue
        library = re.fullmatch(r'\s*(\S+) => (/\S+) \(0x[0-9a-fA-F]+\)\s*', line)
        absolute = re.fullmatch(r'\s*(/\S+) \(0x[0-9a-fA-F]+\)\s*', line)
        vdso = re.fullmatch(r'\s*linux-vdso\.so\.1 \(0x[0-9a-fA-F]+\)\s*', line)
        if library:
            name, path = library.groups()
            require(name not in named, 'duplicate resolved SONAME')
            named[name] = dict(reported_path=path, pin=resolved(path))
        elif absolute:
            direct.append(absolute.group(1))
        elif vdso:
            virtual.append('linux-vdso.so.1')
        else:
            raise RuntimeError('unrecognized ldd line: ' + line)
    require(len(virtual) == 1 and len(direct) <= 1, 'ambiguous direct loader/vDSO')
    loader = None
    if direct:
        path = direct[0]
        loader = dict(reported_path=path, pin=resolved(path))
        if Path(path).name in needed:
            name = Path(path).name
            require(name not in named or named[name]['pin'] == loader['pin'],
                    'named/direct loader disagreement')
            named[name] = loader
    interpreter = None
    if interpreters:
        path = interpreters[0]
        interpreter = dict(reported_path=path, pin=resolved(path))
        require(loader is not None and loader['pin'] == interpreter['pin'],
                'PT_INTERP does not match ldd direct loader')
    require(set(needed) <= set(named), 'unresolved DT_NEEDED closure')
    return dict(needed=needed, libraries=named, interpreter=interpreter,
                direct_loader=loader, virtual_objects=virtual, all_needed_resolved=True,
                shared_object_without_interpreter=shared_object)


def main():
    started = time.monotonic()
    hard_deadline = started + WHOLE_SECONDS
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1,
            'usage: python3 -B audit_guarded_mlp_compiler_tools_v228_v2.py')
    require(Path(__file__).resolve() == SCRIPT and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'unexpected controller host/path/UID')
    require(E.resolve(strict=True) == E and TOOLS.resolve(strict=True) == TOOLS
            and not os.path.lexists(OUT), 'canonical inputs and fresh output required')
    require(shutil.disk_usage(E).free >= FREE_FLOOR, '40 GiB setup storage floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected nice value')
    if priority == 0:
        os.nice(10)
    require(os.sched_getaffinity(0) == {8, 9}
            and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'CPU affinity/nice mismatch')
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, AS_LIMIT),
                      (resource.RLIMIT_FSIZE, STREAM_LIMIT)):
        soft, hard = resource.getrlimit(kind)
        effective = min([cap] + [x for x in (soft, hard) if x != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (effective, effective))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup')
    OUT.mkdir(mode=0o700)
    for sig in SIGNALS:
        signal.signal(sig, interrupted)
    signal.setitimer(signal.ITIMER_REAL, WHOLE_SECONDS - CLEANUP_RESERVE)
    env = dict(HOME='/home/harmenon', PATH='/usr/bin:/bin', LANG='C', LC_ALL='C', TZ='UTC',
               LD_LIBRARY_PATH=str(TOOLS / 'bin') + ':' + str(NIGHTLY_LIB), ROCR_VISIBLE_DEVICES='',
               HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
    leaves, audits, manifest, tool_rows, audit_tools = [], {}, None, {}, {}
    failure, postcheck_errors = None, []
    try:
        pin(SCRIPT)
        audit_tools = {name: resolved('/usr/bin/' + name) for name in ('readelf', 'ldd', 'prlimit')}
        audit_tools['bash'] = resolved('/bin/bash')
        require(NIGHTLY_LIB.resolve(strict=True) == NIGHTLY_LIB, 'canonical nightly lib directory')
        for name, (size, sha) in NIGHTLY.items():
            value = pin(NIGHTLY_LIB / name)
            require((value['bytes'], value['sha256']) == (size, sha), 'nightly compiler library identity')
        manifest_pin = pin(TOOLS / 'manifest.json', cap=64 << 10)
        require(manifest_pin['sha256'] == MANIFEST_SHA, 'actual seven-tool manifest identity')
        manifest = json.loads((TOOLS / 'manifest.json').read_bytes())
        require(set(manifest) == {'bin/' + name for name in NAMES}, 'exact seven-tool roster')
        require({p.name for p in TOOLS.iterdir()} == {'bin', 'manifest.json'}
                and {p.name for p in (TOOLS / 'bin').iterdir()} == set(NAMES), 'closed transported tree')
        for name in NAMES:
            original = manifest['bin/' + name]
            require(set(original) == {'source', 'bytes', 'sha256'}
                    and Path(original['source']).is_absolute(), 'original tool record shape')
            path = TOOLS / 'bin' / name
            value = pin(path)
            require((value['bytes'], value['sha256']) == (original['bytes'], original['sha256']),
                    'transported tool identity differs')
            require(name == 'librustc_codegen_fe2o3.so' or os.access(path, os.X_OK), 'tool executable mode')
            with path.open('rb') as stream:
                require(stream.read(4) == b'\x7fELF', 'tool is not an ELF object')
            tool_rows[name] = dict(original=original, deployed=value)
        save('inputs-before.json', INPUTS)
        for name in NAMES:
            path = str(TOOLS / 'bin' / name)
            dynamic = owned(name + '-readelf', ['/usr/bin/readelf', '-l', '-d', path], env, leaves, hard_deadline)
            linkage = owned(name + '-ldd', ['/usr/bin/ldd', path], env, leaves, hard_deadline)
            audits[name] = libraries(dynamic, linkage, name == 'librustc_codegen_fe2o3.so')
            if name == 'fe2o3-rustc-extract':
                require(audits[name]['libraries']['librustc_codegen_fe2o3.so']['pin']
                        == tool_rows['librustc_codegen_fe2o3.so']['deployed'],
                        'extractor must resolve the exact deployed backend')
        require(len(leaves) == 14 and set(audits) == set(NAMES), 'fourteen successful audit leaves required')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    for name, expected in list(INPUTS.items()):
        if time.monotonic() >= hard_deadline:
            postcheck_errors.append('whole deadline reached before completing input postchecks')
            break
        try:
            signal.setitimer(signal.ITIMER_REAL, max(0.001, hard_deadline - time.monotonic()))
            require(pin(Path(name), track=False) == expected, 'input/provider posthash drift')
        except BaseException as error:
            postcheck_errors.append(name + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    for name, expected in ALIASES.items():
        try:
            require(str(Path(name).resolve(strict=True)) == expected, 'reported alias changed')
        except BaseException as error:
            postcheck_errors.append(name + ': ' + repr(error))
    try:
        require({p.name for p in TOOLS.iterdir()} == {'bin', 'manifest.json'}
                and {p.name for p in (TOOLS / 'bin').iterdir()} == set(NAMES),
                'transported tree roster changed')
    except BaseException as error:
        postcheck_errors.append('tool roster: ' + repr(error))
    if postcheck_errors:
        failure = failure or 'input/provider postcheck failed'
    if time.monotonic() >= hard_deadline:
        failure = failure or 'whole audit deadline exceeded'
    if shutil.disk_usage(E).free < FREE_FLOOR:
        failure = failure or '40 GiB final storage floor'
    save('inputs-after.json', dict(inputs=INPUTS, resolved_paths=ALIASES, errors=postcheck_errors))
    result = dict(schema='ferric-guarded-mlp-compiler-tool-audit-v1', passed=failure is None,
                  failure=failure, postcheck_errors=postcheck_errors, tool_manifest_sha256=MANIFEST_SHA,
                  tools=tool_rows, audit_tools=audit_tools, phases=leaves, audits=audits,
                  inputs=INPUTS, resolved_paths=ALIASES, environment=env,
                  host=os.uname().nodename, boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                  elapsed_seconds=time.monotonic() - started,
                  limits=dict(whole_wall_seconds=WHOLE_SECONDS, cleanup_reserve_seconds=CLEANUP_RESERVE,
                              leaf_wall_seconds=30, leaf_cpu_seconds=30, address_space_bytes=AS_LIMIT,
                              stream_bytes=STREAM_LIMIT, free_bytes=FREE_FLOOR, affinity=[8, 9], nice=10),
                  raw={p.name: pin(p, False) for p in sorted(OUT.iterdir()) if p.is_file()},
                  actual_library_audits_replayed=failure is None, tool_execution_limited_to_readelf_ldd=True,
                  compiler_invocation=False, vendor_created=False, gpu_execution=False,
                  production_authority=False, load_authority=False, launch_authority=False)
    receipt = save('complete.json' if failure is None else 'failed.json', result)
    print(json.dumps(receipt), flush=True)
    if failure is not None:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
