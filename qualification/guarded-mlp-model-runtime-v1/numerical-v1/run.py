"""Bounded data-only TF4/AR4 diagnostic; never launches a process or GPU work."""
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
OUT = E / 'guarded-mlp-model-numerical-comparison-v228-v1'
TF4 = dict(path=str(E / 'guarded-mlp-model-gpu-v228-v2/tf4/failed.json'), bytes=91510,
    sha256='8ced1ef7f167348fbcb68ce56cca8fc8e2238273ebe18fd737127563779871ce')
TF4_REVALIDATION = dict(path=str(E / 'guarded-mlp-model-tf4-revalidation-v228-v3/complete.json'), bytes=79327,
    sha256='d0551f310a57f63dfe98af8c887b5996752e3b62128a3087c4a03849082e6a3f')
AR4 = dict(path=str(E / 'guarded-mlp-model-gpu-v228-v3/ar4/complete.json'), bytes=93497,
    sha256='edf05cf2dd19a9934dab9762b328ea3e6160cf01c15606a3df3299ce967e4991')
REFERENCES = {
    'tf4': dict(path=str(E / 'framework-rearm-v224-v1/complete.json'), bytes=8288,
        sha256='cac5d79969c2e17a19630a581b5e21c594ea65b452806630ee87bf7855396036'),
    'ar4': dict(path=str(E / 'framework-tiles-ar4-v225-v1/complete.json'), bytes=8347,
        sha256='385dc945b9bcd7e6c93d24cddde9afea24c7ed93c2708f5453aeac9dd9e2a69f'),
}
SOURCES = {
    'compare.py': '8154580de7f5ad40fd4897ce264fc3d92c9a109808ea7ea5dab6d0486f01e622',
    'diagnostics.py': '38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf',
    'validate_observation.py': '367d1904f15741138db708c1f10e5a777905b62d9440a961e952a6cc9125b055',
    'guarded_announcement.py': '96f84389071ef2c1a9e354705ebb7144e9ff97e30c242bd9b494f03d2aea6b80',
    'adapter.py': '8d61aadc0da31c118b31eaeabe7cc3b650e9995be43140d384548a8f08d1314a',
    'test_adapter.py': '3b3ab40b15b0fb340e7882dfea1a8cc5f2b17fcd3f53e2ab7d3d5ace6977a1fc',
}
TESTS = ['test_acceptance_and_unobserved_gpu_flags_refuse', 'test_ar4_requires_actual_clean_success',
         'test_history_divergence_never_resumes_tensor_comparison', 'test_json_tuple_u64_join_preserves_numbers',
         'test_original_failed_tf4_cannot_be_promoted']
DEADLINE = float('inf')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_uid, value.st_gid,
            value.st_nlink, value.st_size, value.st_mtime_ns, value.st_ctime_ns)


class Reader:
    def __init__(self):
        self.pins = {}
        self.c = None

    def read_path(self, path, expected=None, maximum=64 << 20, retain=True):
        require(time.monotonic() < DEADLINE, 'whole diagnostic deadline')
        path = Path(path)
        require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical regular input path')
        before = path.lstat()
        require(stat.S_ISREG(before.st_mode) and before.st_nlink >= 1
                and 0 <= before.st_size <= maximum <= 128 << 20, 'bounded ordinary input')
        raw = []
        digest = hashlib.sha256()
        size = 0
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
        with os.fdopen(fd, 'rb') as stream:
            require(stamp(before) == stamp(os.fstat(stream.fileno())), 'input open identity')
            while True:
                require(time.monotonic() < DEADLINE, 'whole diagnostic read deadline')
                chunk = stream.read(1 << 20)
                if not chunk:
                    break
                size += len(chunk)
                require(size <= maximum, 'growing input bound')
                digest.update(chunk)
                if retain:
                    raw.append(chunk)
            require(size == before.st_size and stamp(before) == stamp(os.fstat(stream.fileno()))
                    == stamp(path.lstat()), 'input changed during read')
        pin = dict(path=str(path), bytes=size, sha256=digest.hexdigest())
        require(expected is None or pin == expected, 'actual input pin mismatch')
        require(str(path) not in self.pins or self.pins[str(path)] == pin, 'input changed across reads')
        self.pins[str(path)] = pin
        require(len(self.pins) <= 384 and sum(row['bytes'] for row in self.pins.values()) <= 128 << 20,
                '384 inputs/128 MiB total readset bound')
        return b''.join(raw) if retain else None

    def read(self, pin, maximum=64 << 20, retain=True):
        pin = self.c.pin(pin)
        require(pin['bytes'] <= maximum, 'declared input bound')
        return self.read_path(pin['path'], pin, maximum, retain)

    def tree(self, root, expected):
        actual = set()
        require(root.resolve(strict=True) == root and stat.S_ISDIR(root.lstat().st_mode), 'canonical case directory')
        for directory, dirs, files in os.walk(root, followlinks=False,
                                            onerror=lambda error: (_ for _ in ()).throw(error)):
            require(time.monotonic() < DEADLINE, 'case inventory deadline')
            for name in dirs + files:
                path = Path(directory) / name
                value = path.lstat()
                require(path.resolve(strict=True) == path and
                        (stat.S_ISDIR(value.st_mode) or stat.S_ISREG(value.st_mode)), 'no special case members')
                if stat.S_ISREG(value.st_mode):
                    require(value.st_nlink == 1 and value.st_size <= 8 << 20, 'ordinary bounded case file')
            actual.update(str((Path(directory) / name).relative_to(root)) for name in files)
            require(len(actual) <= 78, 'bounded closed case inventory')
        require(actual == expected, 'exact original case members, no missing or extra file')

    def recheck(self):
        for row in list(self.pins.values()):
            self.read(row, retain=False)


def load(reader, name):
    path = Path(__file__).resolve().parent / name
    raw = reader.read_path(path, maximum=1 << 20)
    require(hashlib.sha256(raw).hexdigest() == SOURCES[name], 'frozen pure source identity')
    module = types.ModuleType('guarded_diagnostic_' + name.replace('.', '_'))
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def save(name, value):
    raw = (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()
    require(len(raw) <= 4 << 20, 'diagnostic output bound')
    with (OUT / name).open('xb') as stream:
        stream.write(raw)
    return dict(path=str(OUT / name), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def interrupted(number, _frame):
    raise RuntimeError('diagnostic interrupted by signal ' + str(number))


def main():
    global DEADLINE
    started_at = time.monotonic()
    DEADLINE = started_at + 300
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B run.py only')
    require(AR4 is not None, 'actual AR4 terminal remains pending; no diagnostic output created')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged data-only qualification host')
    require(OUT.parent.resolve(strict=True) == OUT.parent and not os.path.lexists(OUT), 'fresh diagnostic output only')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'diagnostic nice level')
    if priority == 0:
        os.nice(10)
    for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
        os.environ[key] = ''
    for kind, maximum in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_FSIZE, 4 << 20),
                          (resource.RLIMIT_CPU, 300), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        maximum = min([maximum] + [value for value in (soft, hard) if value != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (maximum, maximum))
    handlers = {number: signal.getsignal(number) for number in
                (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)}
    for number in handlers:
        signal.signal(number, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 300)
    OUT.mkdir(mode=0o700)
    reader = Reader()
    error = None
    posterrors = []
    results = {}
    tests = None
    try:
        reader.read_path(Path(__file__).resolve(), maximum=1 << 20)
        c = load(reader, 'compare.py')
        reader.c = c
        d = load(reader, 'diagnostics.py')
        v = load(reader, 'validate_observation.py')
        a = load(reader, 'guarded_announcement.py')
        adapter = load(reader, 'adapter.py')
        t = load(reader, 'test_adapter.py')
        t.C, t.A, t.D = c, adapter, d
        require(unittest.defaultTestLoader.getTestCaseNames(t.AdapterTests) == TESTS, 'exact five adapter test names')
        transcript = io.StringIO()
        tested = unittest.TextTestRunner(stream=transcript, verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(t.AdapterTests))
        tests = dict(names=TESTS, run=tested.testsRun, failures=len(tested.failures), errors=len(tested.errors),
            skipped=len(tested.skipped), expected_failures=len(tested.expectedFailures),
            unexpected_successes=len(tested.unexpectedSuccesses), transcript=transcript.getvalue())
        require(tested.wasSuccessful() and tests['run'] == 5 and all(tests[key] == 0 for key in
                ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes')), 'five adapter regressions')
        results['tf4'] = adapter.compare_mode(reader, c, d, v, a, TF4, REFERENCES['tf4'], 'tf4', TF4_REVALIDATION)
        results['ar4'] = adapter.compare_mode(reader, c, d, v, a, AR4, REFERENCES['ar4'], 'ar4')
        require(c.pin(results['tf4']['candidate']['parent']) == c.pin(results['ar4']['candidate']['parent'])
                and c.pin(results['tf4']['candidate']['worker']) == c.pin(results['ar4']['candidate']['worker']),
                'same qualified parent and worker across modes')
        for tag in ('tf4', 'ar4'):
            root = Path(REFERENCES[tag]['path']).parent
            expected = {str(root / f'reference/pass{repeat}-pos{position}.bf16')
                        for repeat in (1, 2) for position in range(4)}
            require(expected <= set(reader.pins) and all(reader.pins[path]['bytes'] == 606976 for path in expected),
                    'all eight unchanged reference payloads consumed per mode')
    except BaseException as caught:
        error = type(caught).__name__ + ': ' + str(caught)
    try:
        reader.recheck()
    except BaseException as caught:
        posterrors.append(type(caught).__name__ + ': ' + str(caught))
    require(time.monotonic() < DEADLINE, 'whole deadline expired; no completed diagnostic accepted')
    passed = error is None and not posterrors
    result = dict(schema='ferric-guarded-mlp-model-numerical-diagnostic-v1', passed=passed,
        status='DIAGNOSTIC_COMPLETE' if passed else 'DIAGNOSTIC_FAILED', error=error, postcheck_errors=posterrors,
        elapsed_seconds=time.monotonic() - started_at, selftests=tests, modes=results,
        inputs=reader.pins, input_posthashes_complete=not posterrors, sources=SOURCES,
        comparison_completed=passed, independent_framework_reference_compared=passed,
        original_tf4_failure_preserved=True, source_model_shards_rehashed=False,
        gpu_execution=False, native_rerun=False, subprocess_execution=False,
        acceptance_threshold=None, numerical_acceptance=False, independent_tensor_acceptance=False,
        full_model_acceptance=False, full_model_correctness=False, performance_claim=False,
        production_authority=False, full_long_workload=False, sustained_2048_256=False)
    save('complete.json' if passed else 'failed.json', result)
    print(json.dumps(dict(passed=passed, error=error, numerical_acceptance=False), sort_keys=True))
    signal.setitimer(signal.ITIMER_REAL, 0)
    for number, handler in handlers.items():
        signal.signal(number, handler)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
