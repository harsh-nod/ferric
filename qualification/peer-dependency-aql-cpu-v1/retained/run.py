"""Bounded CPU-only packet ABI checks in an isolated task-owned workspace."""
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parent
OUT = ROOT / 'evidence'
TOOLCHAIN = Path('/home/harmenon/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/bin')


def pin(path):
    raw = path.read_bytes()
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def sources():
    files = [ROOT / 'Cargo.toml', ROOT / 'run.py']
    files.extend(p for p in (ROOT / 'crates/fe2o3-aql').rglob('*') if p.is_file())
    return {str(p.relative_to(ROOT)): pin(p) for p in sorted(files)}


def save(name, value):
    with (OUT / name).open('x') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')


def group_exists(pid):
    try:
        os.killpg(pid, 0)
        return True
    except ProcessLookupError:
        return False


def stop_group(pid):
    for sig in (signal.SIGTERM, signal.SIGKILL):
        if not group_exists(pid):
            break
        try:
            os.killpg(pid, sig)
        except ProcessLookupError:
            break
        deadline = time.monotonic() + 2
        while group_exists(pid) and time.monotonic() < deadline:
            time.sleep(0.02)


def interrupted(signum, _frame):
    raise RuntimeError('interrupted by signal ' + str(signum))


def main():
    assert os.getuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
    assert ROOT.name == 'peer-dependency-aql-cpu-v228-v1' and not OUT.exists()
    assert shutil.disk_usage(ROOT).free >= 4 << 30
    os.sched_setaffinity(0, {8, 9})
    os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 2 << 30),
                      (resource.RLIMIT_CPU, 600), (resource.RLIMIT_FSIZE, 512 << 20)):
        soft, hard = resource.getrlimit(kind)
        cap = min([cap] + [n for n in (soft, hard) if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (cap, cap))
    OUT.mkdir(mode=0o700)
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        signal.signal(sig, interrupted)
    env = dict(HOME='/home/harmenon', PATH=str(TOOLCHAIN) + ':/usr/bin:/bin',
               CARGO_HOME='/home/harmenon/.cargo', CARGO_TARGET_DIR=str(ROOT / 'target'),
               CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', RUST_BACKTRACE='1',
               ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
    before = sources()
    save('sources-input.json', before)
    results = []
    formatted = None
    failure = None

    def run(label, argv):
        started = time.monotonic_ns()
        stdout, stderr = OUT / (label + '.stdout'), OUT / (label + '.stderr')
        timed_out = False
        forced_cleanup = False
        exception = None
        with stdout.open('xb') as so, stderr.open('xb') as se:
            child = subprocess.Popen(argv, cwd=ROOT, env=env, stdin=subprocess.DEVNULL,
                                     stdout=so, stderr=se, start_new_session=True)
            try:
                save(label + '.started.json', dict(pid=child.pid, argv=argv))
                code = child.wait(timeout=300)
            except subprocess.TimeoutExpired:
                timed_out = True
            except BaseException as error:
                exception = repr(error)
            finally:
                if child.poll() is None or group_exists(child.pid):
                    forced_cleanup = True
                    stop_group(child.pid)
                code = child.wait(timeout=5)
                group_absent = not group_exists(child.pid)
        row = dict(label=label, argv=argv, pid=child.pid, exit_code=code,
                   timed_out=timed_out, elapsed_ns=time.monotonic_ns() - started,
                   forced_cleanup=forced_cleanup, process_group_absent=group_absent,
                   exception=exception,
                   stdout=pin(stdout), stderr=pin(stderr))
        results.append(row)
        save(label + '.json', row)
        if code != 0 or timed_out or forced_cleanup or not group_absent or exception is not None:
            raise RuntimeError(label + ' did not finish successfully')

    try:
        run('rustc-version', [str(TOOLCHAIN / 'rustc'), '--version', '--verbose'])
        new_sources = ['crates/fe2o3-aql/src/peer_dependency.rs', 'crates/fe2o3-aql/tests/peer_dependency.rs']
        run('format', [str(TOOLCHAIN / 'rustfmt'), '--edition', '2024', *new_sources])
        formatted = sources()
        save('sources-tested.json', formatted)
        run('format-check', [str(TOOLCHAIN / 'rustfmt'), '--edition', '2024', '--check', *new_sources])
        run('lock', [str(TOOLCHAIN / 'cargo'), 'generate-lockfile', '--offline'])
        run('tests', [str(TOOLCHAIN / 'cargo'), 'test', '--offline', '--locked',
                      '-p', 'fe2o3-aql', '--', '--test-threads=1'])
        header = Path('/opt/rocm/include/hsa/hsa.h')
        header_pin = pin(header)
        save('rocm-header.json', header_pin)
        assert header_pin['sha256'] == '51ea864cc3e83a9ce824c294dd98a5724eeec87b76fafded1a01d406206ce0f5'
        run('oracle-build', ['/usr/bin/cc', '-std=c11', '-Wall', '-Wextra', '-Werror',
                             '-I/opt/rocm/include/hsa',
                             'crates/fe2o3-aql/tests/oracles/aql_peer_dependency.c',
                             '-o', str(OUT / 'peer-dependency-oracle')])
        run('oracle-run', [str(OUT / 'peer-dependency-oracle')])
        assert pin(header) == header_pin, 'ROCm header changed during oracle'
        assert sources() == formatted, 'tested source changed during checks'
    except Exception as error:
        failure = repr(error)
    result = dict(schema='ferric-peer-dependency-aql-cpu-v1', passed=failure is None,
                  failure=failure, phases=results, input_sources=before,
                  tested_sources=formatted, final_sources=sources(),
                  host=os.uname().nodename, boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                  gpu_execution=False, native_peer_ordering_qualified=False)
    if (ROOT / 'Cargo.lock').is_file():
        result['cargo_lock'] = pin(ROOT / 'Cargo.lock')
    name = 'complete.json' if failure is None else 'failed.json'
    save(name, result)
    print(json.dumps(pin(OUT / name)))
    if failure is not None:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
