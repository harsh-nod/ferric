"""Bounded in-process tests for the data-only host attribution report."""
import contextlib
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import stat
import sys
import time
import traceback
import types
import unittest

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-currentness-duration-report-cpu-v228-v1')
SOURCES = {
    'validate_readiness.py': (19800, '0c319e99142b19909350d9a81404948b494c2e3ea0defbbc165c2e5529be032a'),
    'validate_timing.py': (10104, '27c652f5452676b4cc08a634241aff89b2653e85ee111133c1f1c4ab13efd93f'),
    'duration_report.py': (22586, 'dedca35993c62acce84f8d2286b2dbce30f2f6f046afcebd6427ff69c83d9b6e'),
    'test_duration_report.py': (12400, '9b9b7378b46f98a20160cf071b8e853dda6b26a75c05a336bbde6a4404b3c474'),
}


def require(ok, reason):
    if not ok:
        raise RuntimeError(reason)


def read(path):
    require(path.resolve(strict=True) == path, 'canonical ordinary source')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= 16 << 20, 'bounded ordinary file')
    stamp = lambda row: (row.st_dev, row.st_ino, row.st_mode, row.st_size, row.st_mtime_ns, row.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC), 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'changed at open')
        raw = stream.read((16 << 20) + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'changed during read')
    require(stamp(path.lstat()) == stamp(before) and len(raw) == before.st_size, 'changed after read')
    return raw, dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def save(path, value):
    raw = (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()
    require(len(raw) <= 1 << 20, 'bounded metadata')
    with path.open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())


def main():
    start = time.monotonic()
    require(__debug__ and sys.flags.isolated and sys.dont_write_bytecode, 'isolated ordinary Python')
    require(Path(__file__).resolve().parent == ROOT and ROOT.resolve(strict=True) == ROOT
            and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'exact unprivileged host/root')
    require({p.name for p in ROOT.iterdir()} == set(SOURCES) | {'run_cpu.py'}, 'fresh closed source root')
    require(shutil.disk_usage(ROOT).free >= 40 << 30, 'initial disk floor')
    os.umask(0o077); os.sched_setaffinity(0, {8, 9})
    require(os.getpriority(os.PRIO_PROCESS, 0) in (0, 10), 'expected priority')
    if os.getpriority(os.PRIO_PROCESS, 0) == 0:
        os.nice(10)
    for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
        os.environ[key] = ''
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_FSIZE, 8 << 20),
                      (resource.RLIMIT_CORE, 0), (resource.RLIMIT_CPU, 30)):
        soft, hard = resource.getrlimit(kind)
        cap = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (cap, cap))
    def stop(_number, _frame):
        raise RuntimeError('report test whole deadline or signal')
    for number in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, stop)
    signal.setitimer(signal.ITIMER_REAL, 60)
    evidence = ROOT / 'evidence'; evidence.mkdir(mode=0o700)
    temporary = ROOT / 'tmp'; temporary.mkdir(mode=0o700)
    os.environ['TMPDIR'] = str(temporary)
    sources = {}; after = {}; tests = []; result = None; failure = None
    python = Path(sys.executable).resolve(strict=True)
    save(evidence / 'started.json', dict(pid=os.getpid(), uid=os.getuid(), host=os.uname().nodename,
        affinity=sorted(os.sched_getaffinity(0)), nice=os.getpriority(os.PRIO_PROCESS, 0),
        python=dict(path=str(python), **read(python)[1]), isolated=True, no_bytecode=True,
        in_process=True, gpu_execution=False, child_processes_requested=False,
        whole_seconds=60, cpu_seconds=30, address_space_bytes=512 << 20,
        stream_bytes=4 << 20, file_bytes=8 << 20))
    with (evidence / 'stdout').open('x', encoding='utf-8') as stdout, \
            (evidence / 'stderr').open('x', encoding='utf-8') as stderr:
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            try:
                bodies = {}
                for name in (*SOURCES, 'run_cpu.py'):
                    bodies[name], sources[name] = read(ROOT / name)
                    if name in SOURCES:
                        require((sources[name]['bytes'], sources[name]['sha256']) == SOURCES[name], 'exact source pin')
                save(evidence / 'sources-before.json', sources)
                # The four authenticated modules load in dependency order, without search-path imports.
                for name in SOURCES:
                    module_name = Path(name).stem
                    module = types.ModuleType(module_name); module.__file__ = str(ROOT / name)
                    sys.modules[module_name] = module
                    exec(compile(bodies[name], module.__file__, 'exec'), module.__dict__)
                test_module = sys.modules['test_duration_report']
                names = unittest.defaultTestLoader.getTestCaseNames(test_module.DurationReportTests)
                tests = ['test_duration_report.DurationReportTests.' + name for name in names]
                require(len(tests) == 13 and len(set(tests)) == 13, 'exact thirteen selected tests')
                suite = unittest.defaultTestLoader.loadTestsFromModule(test_module)
                require(suite.countTestCases() == 13, 'no additional discovered tests')
                result = unittest.TextTestRunner(stream=stderr, verbosity=2).run(suite)
                require(result.testsRun == 13 and result.wasSuccessful() and not result.skipped
                        and not result.expectedFailures and not result.unexpectedSuccesses, 'all thirteen executed passes')
                require(list(temporary.iterdir()) == [], 'private temporary files retired')
                after = {name: read(ROOT / name)[1] for name in sources}
                require(after == sources, 'source postcheck')
                save(evidence / 'sources-after.json', after)
                require(shutil.disk_usage(ROOT).free >= 38 << 30, 'final disk floor')
                require(time.monotonic() - start < 60, 'whole deadline')
            except BaseException as error:
                failure = type(error).__name__ + ': ' + str(error)
                traceback.print_exc(file=stderr)
        stdout.flush(); stderr.flush()
        os.fsync(stdout.fileno()); os.fsync(stderr.fileno())
    raw = {p.name: read(p)[1] for p in evidence.iterdir()}
    if any(raw[n]['bytes'] > 4 << 20 for n in ('stdout', 'stderr')):
        failure = failure or 'stream cap exceeded'
    terminal = dict(schema='ferric-currentness-duration-report-cpu-v1', passed=failure is None,
        failure=failure, elapsed_seconds=time.monotonic() - start, tests=tests,
        tests_run=0 if result is None else result.testsRun,
        failures=[] if result is None else [test.id() for test, _ in result.failures],
        errors=[] if result is None else [test.id() for test, _ in result.errors],
        sources_before=sources, sources_after=after, raw=raw,
        in_process=True, child_processes_requested=False, gpu_execution=False,
        model_execution=False, native_execution=False, numerical_acceptance=False,
        performance_claim=False, actual_archive_tested=False)
    save(evidence / 'complete.json', terminal)
    signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps({k: terminal[k] for k in ('passed', 'failure', 'tests_run', 'elapsed_seconds')}, sort_keys=True))
    return 0 if terminal['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
