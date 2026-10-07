"""Bounded in-process MI350 checks for the data-only hidden-read analyzer."""
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import signal
import sys
import time
import types
import unittest

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-peer-read-analysis-cpu-v228-v1')
SOURCES = {
    'analyze_hidden_reads.py': dict(bytes=11498, sha256='f9e3ee4f8072532b71ddf7172f4b7b90725b4d824b511a186d37d094ce17b8ca'),
    'test_hidden_reads.py': dict(bytes=7050, sha256='43929c9582085637ca1fe194a2680f43a30e1482738a6ba2340652c56cb256d0'),
}


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1,
            'python3 -I -S -B run_analysis_tests.py')
    require(Path(__file__).resolve().parent == ROOT and ROOT.resolve(strict=True) == ROOT
            and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'fixed root/host/UID')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    os.nice(10)
    for kind, bound in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 30),
                        (resource.RLIMIT_FSIZE, 1 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        bound = min([bound] + [value for value in (soft, hard) if value != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (bound, bound))
    for name in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
        os.environ[name] = ''
    started = time.monotonic()
    deadline = started + 30
    def timed_out(_number, _frame):
        raise TimeoutError('analysis CPU deadline')
    signal.signal(signal.SIGALRM, timed_out)
    signal.setitimer(signal.ITIMER_REAL, 30, 1)
    output = ROOT / 'evidence'
    require(not os.path.lexists(output), 'fresh CPU result')
    bodies = {}
    for name, expected in SOURCES.items():
        path = ROOT / name
        require(path.resolve(strict=True) == path and path.is_file()
                and path.stat().st_size < 1 << 20, 'bounded ordinary source')
        bodies[name] = path.read_bytes()
        require(expected is not None and pin(bodies[name]) == expected, 'bound source pin')
    output.mkdir(mode=0o700)
    transcript = io.StringIO()
    error, result = None, None
    try:
        modules = {}
        for name in SOURCES:
            module = types.ModuleType(name.removesuffix('.py'))
            module.__file__ = str(ROOT / name)
            sys.modules[module.__name__] = module
            exec(compile(bodies[name], module.__file__, 'exec'), module.__dict__)
            modules[name] = module
        suite = unittest.defaultTestLoader.loadTestsFromModule(modules['test_hidden_reads.py'])
        require(suite.countTestCases() == 5, 'five declared tests')
        result = unittest.TextTestRunner(stream=transcript, verbosity=2).run(suite)
        require(time.monotonic() < deadline, 'absolute deadline after tests')
        require(result.testsRun == 5 and result.wasSuccessful() and not result.skipped,
                'complete passing analysis suite')
    except BaseException as failure:
        error = repr(failure)
    after = {name: pin((ROOT / name).read_bytes()) for name in SOURCES}
    if after != SOURCES:
        error = error or 'source drift'
    raw = transcript.getvalue().encode()
    require(len(raw) <= 65536, 'bounded unittest transcript')
    with (output / 'stdout.txt').open('xb') as stream:
        stream.write(raw)
    terminal = dict(schema='ferric-peer-read-analysis-cpu-v1', passed=error is None,
        error=error, host=os.uname().nodename, python=sys.version,
        controller=pin(Path(__file__).read_bytes()), sources_before=SOURCES, sources_after=after,
        tests_run=result.testsRun if result else 0, failures=len(result.failures) if result else None,
        errors=len(result.errors) if result else None, stdout=pin(raw),
        elapsed_seconds=time.monotonic() - started, affinity=[8, 9],
        subprocesses_started=0, gpu_execution=False, numerical_acceptance=False,
        performance_claim=False, synthetic_fixtures_only=True)
    with (output / ('complete.json' if error is None else 'failed.json')).open('x') as stream:
        json.dump(terminal, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
    signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(terminal, sort_keys=True))
    return int(error is not None)


if __name__ == '__main__':
    raise SystemExit(main())
