"""Run the eight synthetic census tests on MI350 without subprocesses or GPU use."""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import signal
import stat
import sys
import time
import types
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-reusable-ar4-census-cpu-v228-v1'
SOURCES = {
    'validate_observation': ('guarded-mlp-reusable-ar4-validator-v228-v1.py', 15574,
        '9d0f860d0174ef5574c3023253b4e220ff16bfba845ad13c7e0ce210cdc66ed7'),
    'test_census': ('guarded-mlp-reusable-ar4-census-tests-v228-v1.py', 4375,
        'd4f19ae2f4bcf9f68464a7b8951b82368183cec6db998a0ed55449d2349e7f64'),
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def read(path):
    require(path.resolve(strict=True) == path, 'canonical input')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1
                and before.st_uid == os.getuid() and before.st_size <= 1 << 20, 'ordinary bounded owned input')
        raw = stream.read((1 << 20) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size,
            'input changed while reading')
    return raw


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


class BoundedText(io.StringIO):
    def write(self, value):
        require(self.tell() + len(value) <= 65536, 'bounded test output')
        return super().write(value)


def save(name, raw):
    require(len(raw) <= 65536, 'bounded output')
    with (ROOT / name).open('xb') as stream:
        stream.write(raw)
    return dict(path=str(ROOT / name), **pin(raw))


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'isolated no-bytecode invocation')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'selected MI350 host and account')
    require(E.resolve(strict=True) == E and not os.path.lexists(ROOT), 'fresh census CPU namespace')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    os.nice(10)
    for kind, limit in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 30),
                        (resource.RLIMIT_FSIZE, 65536), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        limit = min([limit] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    def interrupted(number, _frame):
        raise RuntimeError('census CPU signal ' + str(number))
    for number in (signal.SIGALRM, signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        signal.signal(number, interrupted)
    started = time.monotonic()
    deadline = started + 30
    def wall_guard():
        remaining = deadline - time.monotonic()
        require(remaining > 0, 'absolute census CPU wall deadline')
        signal.setitimer(signal.ITIMER_REAL, remaining)
    wall_guard()
    ROOT.mkdir(mode=0o700)
    sources, modules, result, failure = {}, {}, None, None
    stdout, stderr = BoundedText(), BoundedText()
    controller = read(Path(__file__).resolve())
    try:
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            for name, (filename, size, sha) in SOURCES.items():
                path = E / filename
                raw = read(path)
                require(pin(raw) == dict(bytes=size, sha256=sha), 'reviewed source pin')
                sources[str(path)] = pin(raw)
                module = types.ModuleType(name)
                module.__file__ = str(path)
                require(name not in sys.modules, 'fresh test module')
                sys.modules[name] = modules[name] = module
                exec(compile(raw, str(path), 'exec'), module.__dict__)
            suite = unittest.defaultTestLoader.loadTestsFromTestCase(modules['test_census'].CensusTests)
            require(suite.countTestCases() == 8, 'exact eight synthetic tests')
            result = unittest.TextTestRunner(stream=stderr, verbosity=2).run(suite)
            require(result.testsRun == 8 and result.wasSuccessful() and not result.skipped
                    and not result.expectedFailures and not result.unexpectedSuccesses, 'all eight tests pass')
    except BaseException as error:
        failure = type(error).__name__ + ': ' + str(error)
    # Do not continue retention after a swallowed unittest timeout.
    wall_guard()
    post_errors = []
    for path, expected in sources.items():
        wall_guard()
        try:
            require(pin(read(Path(path))) == expected, 'source drift')
        except BaseException as error:
            post_errors.append(str(error))
    wall_guard()
    try:
        require(read(Path(__file__).resolve()) == controller, 'controller drift')
    except BaseException as error:
        post_errors.append(str(error))
    failure = failure or ('postcheck failed' if post_errors else None)
    wall_guard()
    record = dict(schema='ferric-guarded-mlp-reusable-ar4-census-cpu-v1', passed=failure is None,
        failure=failure, postcheck_errors=post_errors, controller=pin(controller), sources=sources,
        tests_run=result.testsRun if result is not None else 0,
        failures=len(result.failures) if result is not None else None,
        errors=len(result.errors) if result is not None else None,
        stdout=save('stdout', stdout.getvalue().encode()), stderr=save('stderr', stderr.getvalue().encode()),
        elapsed_seconds=time.monotonic() - started, synthetic_evidence_only=True,
        subprocesses_started=0, gpu_execution=False, numerical_acceptance=False, performance_claim=False)
    raw = (json.dumps(record, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()
    wall_guard()
    receipt = save('complete.json' if failure is None else 'failed.json', raw)
    wall_guard()
    signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(passed=failure is None, failure=failure, tests_run=record['tests_run'], receipt=receipt)))
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
