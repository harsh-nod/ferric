"""One bounded in-process CPU-only run of seven synthetic report contracts."""
import ast
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

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-peer-read-report-cpu-v228-v1')
SOURCES = {
    'render.py': (23281, '2534f8c96ecbc52e4d2579e1781ba8a14631bcc500ba9d34281af0cbc8e3345f'),
    'test_render.py': (9348, '0b15b42203cf035558f58cee47f513b020d582cc1daf22d89b0e73c63d128efd'),
}
NAMES = ('test_bad_wall_order_overflow_and_drift_refuse',
    'test_csv_plot_and_caption_keep_run_and_forward_units',
    'test_duplicate_fields_strict_integers_and_decimal_rounding',
    'test_exact_full_census_and_integer_forward_totals',
    'test_failure_payload_identity_and_claims_refuse',
    'test_incomplete_order_and_independence_refuse',
    'test_wrong_mode_counts_or_hidden_effects_refuse')


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1,
            'python3 -I -S -B run_tests.py')
    require(Path(__file__).resolve().parent == ROOT and ROOT.resolve(strict=True) == ROOT
            and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'exact root/host/UID')
    require({p.name for p in ROOT.iterdir()} == {'run_tests.py', *SOURCES}, 'exact three source files')
    started = time.monotonic(); deadline = started + 30
    def timed_out(_number, _frame): raise TimeoutError('report CPU deadline')
    def guard(): require(time.monotonic() < deadline, 'absolute report CPU deadline')
    signal.signal(signal.SIGALRM, timed_out); signal.setitimer(signal.ITIMER_REAL, 30, 1)
    os.umask(0o077); os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0); require(priority in (0, 10), 'expected nice level')
    if priority == 0: os.nice(10)
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 30),
                      (resource.RLIMIT_FSIZE, 1 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        bound = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (bound, bound))
    for name in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'): os.environ[name] = ''
    def read(name):
        guard(); path = ROOT / name
        require(path.resolve(strict=True) == path, 'canonical test source')
        stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
        with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
            before = os.fstat(stream.fileno())
            require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= 1 << 20, 'bounded regular source')
            raw = stream.read((1 << 20) + 1); after = os.fstat(stream.fileno())
        require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size, 'source changed')
        return raw
    bodies = {name: read(name) for name in ('run_tests.py', *SOURCES)}
    before = {name: pin(raw) for name, raw in bodies.items()}
    for name, extent in SOURCES.items():
        require((before[name]['bytes'], before[name]['sha256']) == extent, 'exact reviewed source')
    tree = ast.parse(bodies['test_render.py'])
    classes = [n for n in tree.body if isinstance(n, ast.ClassDef)]
    require(len(classes) == 1 and classes[0].name == 'RenderTests'
        and tuple(sorted(n.name for n in classes[0].body if isinstance(n, ast.FunctionDef)
                         and n.name.startswith('test_'))) == NAMES, 'exact seven named tests')
    out = ROOT / 'evidence'; out.mkdir(mode=0o700)
    transcript = io.StringIO(); error = None; result = None
    try:
        modules = {}
        for name in SOURCES:
            module = types.ModuleType(name.removesuffix('.py')); module.__file__ = str(ROOT / name)
            sys.modules[module.__name__] = module
            exec(compile(bodies[name], module.__file__, 'exec'), module.__dict__); modules[name] = module
        suite = unittest.defaultTestLoader.loadTestsFromModule(modules['test_render.py'])
        require(suite.countTestCases() == 7, 'seven loaded tests')
        result = unittest.TextTestRunner(stream=transcript, verbosity=2).run(suite)
        guard()
        require(result.testsRun == 7 and result.wasSuccessful() and not result.skipped, 'all seven tests passed')
    except BaseException as failure:
        error = repr(failure)
    guard()
    after = {name: pin(read(name)) for name in before}
    if after != before: error = error or 'source drift'
    raw = transcript.getvalue().encode(); require(len(raw) <= 65536, 'bounded raw unittest transcript')
    guard()
    with (out / 'stdout.txt').open('xb') as stream: stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    terminal = dict(schema='ferric-peer-read-report-cpu-v1', passed=error is None, error=error,
        controller=before['run_tests.py'], sources_before=before, sources_after=after,
        source_unchanged=after == before, test_names=list(NAMES), tests_run=result.testsRun if result else 0,
        failures=len(result.failures) if result else None, errors=len(result.errors) if result else None,
        skipped=len(result.skipped) if result else None, stdout=pin(raw),
        elapsed_seconds=time.monotonic() - started, affinity=[8, 9], whole_seconds=30,
        subprocesses_started=0, gpu_execution=False, model_execution=False,
        numerical_acceptance=False, performance_claim=False, synthetic_fixtures_only=True)
    guard()
    with (out / ('complete.json' if error is None else 'failed.json')).open('x') as stream:
        json.dump(terminal, stream, indent=2, sort_keys=True, allow_nan=False); stream.write('\n')
        stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(terminal, sort_keys=True)); signal.setitimer(signal.ITIMER_REAL, 0)
    return int(error is not None)


if __name__ == '__main__':
    raise SystemExit(main())
