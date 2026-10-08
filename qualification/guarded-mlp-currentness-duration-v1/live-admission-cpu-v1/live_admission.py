"""Exercise diagnostic admission against original MI350 products, without launch."""
import hashlib
import json
import os
from pathlib import Path
import signal
import stat
import sys
import time
import types


ROOT = Path(__file__).resolve().parent
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
WORKER_ROOT = E / 'guarded-mlp-currentness-duration-diagnostic-cpu-v228-v2'
PARENT_ROOT = E / 'guarded-mlp-currentness-duration-diagnostic-parent-cpu-v228-v2'
SOURCES = {
    'run_model_gpu.py': (54215, '2803e5783d9afaf2b58c704b2bf7dddc053ee44aca29a5c1fbf1a845c887daba'),
    'library_audit.py': (25872, 'b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'),
}
PRODUCTS = {
    'worker_cpu': (WORKER_ROOT / 'evidence/complete.json', 2569774,
        '5a27b9983ecd0d523226cbf2db4070108cd6deaf75d2ee6fe528274d2f8a5ac2'),
    'worker_sources': (WORKER_ROOT / 'evidence/sources-after.json', 450252,
        '74bcd0c27c264faf5991e8b13f8dcd2b9f52d42ad1d9b3e9a30350ee2c1baa0c'),
    'worker': (WORKER_ROOT / 'target/debug/ferric-tp-peer-finite-engineering-worker-v1', 6598384,
        '10c7432d29f8ca780ce9338c3217bd93c75b04cd3463d7cd0921c0904382606d'),
    'parent_cpu': (PARENT_ROOT / 'evidence/complete.json', 4165800,
        'f253584fcc0fde7087614c2d9469f37ff950aeb48825fc64d1be1614d6596602'),
    'parent_sources': (PARENT_ROOT / 'evidence/sources-after.json', 537481,
        '2737318f2f1e13ac89eabf9e1ad603fc11c11f6a86adfbd2e0115a9bcb6a0a1b'),
    'parent': (PARENT_ROOT / 'target/debug/ferric-qwen3-guarded-mlp-readiness-engineering', 14281472,
        'c1c90ba29f3e77b8111dee1a04d9d35f91cbff6abb3e914a9b75f599378bc7e4'),
}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def source(name):
    path = ROOT / name
    require(path.resolve(strict=True) == path, 'canonical reviewed source')
    size, sha = SOURCES[name]
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size == size, 'reviewed ordinary source extent')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC), 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'source changed at open')
        raw = stream.read(size + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'source changed while reading')
    require(stamp(path.lstat()) == stamp(before) and len(raw) == size
            and hashlib.sha256(raw).hexdigest() == sha, 'exact reviewed source bytes')
    return raw


def load(name):
    raw = source(name)
    module = types.ModuleType('live_cpu_' + name.removesuffix('.py'))
    module.__file__ = str(ROOT / name)
    exec(compile(raw, module.__file__, 'exec'), module.__dict__)
    return module


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1,
            'ordinary python3 -I -B live_admission.py only')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'exact CPU qualification host')
    require(os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'inherited low-priority CPU lane')
    require(all(os.environ.get(name) == '' for name in
            ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')),
            'GPU-hidden CPU-only qualification environment')
    start = time.monotonic()
    deadline = start + 60
    def stop(_number, _frame):
        raise RuntimeError('bounded live CPU admission deadline')
    signal.signal(signal.SIGALRM, stop)
    signal.setitimer(signal.ITIMER_REAL, 60)
    runner = load('run_model_gpu.py')
    audit = load('library_audit.py')
    audit.HARD_DEADLINE = deadline
    plan = {name: dict(path=str(path), bytes=size, sha256=sha)
            for name, (path, size, sha) in PRODUCTS.items()}
    # Only observed product constants are bound; no acceptance predicate is replaced.
    for name, row in plan.items():
        setattr(runner, name.upper(), {key: row[key] for key in ('bytes', 'sha256')})
    admitted = runner.cpu_admission(plan, audit)
    require(admitted == plan, 'complete original six-way CPU/product/source-map admission')
    checker = runner.checker_admission(audit)
    require(checker == runner.CHECKER_CPU, 'original independently exercised checker admission')
    for path, pin in list(audit.INPUTS.items()):
        audit.read(path, pin, retain=False, limit=128 << 20)
    for name in SOURCES:
        source(name)
    require(time.monotonic() < deadline, 'live admission completed within bound')
    result = dict(schema='ferric-currentness-duration-live-cpu-admission-v1', passed=True,
        admission=admitted, checker_cpu=checker, readset=audit.INPUTS,
        source_pins={name: dict(bytes=size, sha256=sha) for name, (size, sha) in SOURCES.items()},
        elapsed_seconds=time.monotonic() - start, data_only=True,
        synthetic_fixture=False, original_products_admitted=True,
        native_launch_admitted=False, native_parent_execution=False, gpu_execution=False,
        model_execution=False, numerical_acceptance=False, performance_claim=False)
    print(json.dumps(result, sort_keys=True, allow_nan=False))
    signal.setitimer(signal.ITIMER_REAL, 0)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
