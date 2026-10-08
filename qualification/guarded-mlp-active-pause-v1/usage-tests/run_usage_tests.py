"""Run fourteen pure usage tests in one bounded CPU process; retain originals."""
import ast
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import sys
import time
import traceback
import types
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
D = E / 'guarded-mlp-active-pause-native-source-v228-v1'
OUT = E / 'guarded-mlp-active-pause-usage-tests-v228-v1'


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def main():
    start = time.monotonic()
    assert __debug__ and sys.dont_write_bytecode and len(sys.argv) == 4
    assert os.getuid() == os.geteuid() == 9661
    assert os.uname().nodename == 'smci350-rck-g03-b19-03'
    assert os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
    assert all(os.environ.get(k) == '' for k in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'))
    assert not os.path.lexists(OUT) and shutil.disk_usage(E).free >= 40 << 30
    os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 1 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (cap, cap))
    def stop(_number, _frame):
        raise KeyboardInterrupt('90-second pure-test deadline')
    for number in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, stop)
    signal.alarm(90)
    originals = {}
    for name, digest in zip(('run_layer_model_gpu.py', 'layer_retention_tool.py', 'test_layer_cpu_usage.py'), sys.argv[1:]):
        path = D / name
        assert path.resolve(strict=True) == path and path.is_file() and path.stat().st_size <= 1 << 20
        raw = path.read_bytes()
        assert pin(raw)['sha256'] == digest
        originals[name] = raw
    expected = sorted('test_layer_cpu_usage.' + node.name + '.' + method.name
        for node in ast.parse(originals['test_layer_cpu_usage.py']).body if isinstance(node, ast.ClassDef)
        for method in node.body if isinstance(method, ast.FunctionDef) and method.name.startswith('test_'))
    assert len(expected) == len(set(expected)) == 14
    OUT.mkdir(mode=0o700)
    for name, raw in originals.items():
        with (OUT / name).open('xb') as stream:
            stream.write(raw)
    stdout, stderr, successes = io.StringIO(), io.StringIO(), []
    failure, result = None, None
    class Result(unittest.TextTestResult):
        def addSuccess(self, test):
            successes.append(test.id())
            super().addSuccess(test)
    try:
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            name = 'test_layer_cpu_usage.py'
            module = types.ModuleType(Path(name).stem)
            module.__file__ = str(OUT / name)
            sys.modules[module.__name__] = module
            exec(compile(originals[name], module.__file__, 'exec'), module.__dict__)
            suite = unittest.defaultTestLoader.loadTestsFromModule(module)
            result = unittest.TextTestRunner(stream=stderr, verbosity=2, resultclass=Result).run(suite)
            assert result.testsRun == 14 and result.wasSuccessful() and not result.skipped
            assert sorted(successes) == expected
    except BaseException:
        failure = traceback.format_exc()
    signal.alarm(0)
    unchanged = all((D / n).read_bytes() == raw == (OUT / n).read_bytes() for n, raw in originals.items())
    streams = {'stdout': stdout.getvalue().encode(), 'stderr': stderr.getvalue().encode()}
    assert all(len(raw) <= 1 << 20 for raw in streams.values())
    for name, raw in streams.items():
        with (OUT / name).open('xb') as stream:
            stream.write(raw)
    passed = failure is None and unchanged and sorted(successes) == expected
    receipt = dict(schema='ferric-active-pause-usage-pure-tests-v1', passed=passed, failure=failure,
        controller=pin(Path(__file__).read_bytes()), sources={n: pin(b) for n, b in originals.items()},
        source_unchanged=unchanged, raw={n: pin(b) for n, b in streams.items()},
        expected_names=expected, passed_names=sorted(successes), tests_run=None if result is None else result.testsRun,
        elapsed_seconds=time.monotonic() - start, single_process=True, child_processes_started=0,
        gpu_execution=False, native_execution=False, actual_archive_read=False, actual_rusage_read=False,
        numerical_acceptance=False,
        limits=dict(wall_seconds=90, cpu_seconds=120, address_space_bytes=512 << 20,
                    affinity=[8, 9], nice=10), argv=sys.argv)
    raw = (json.dumps(receipt, sort_keys=True, indent=2) + '\n').encode()
    destination = OUT / ('complete.json' if passed else 'failed.json')
    with destination.open('xb') as stream:
        stream.write(raw)
    assert destination.read_bytes() == raw and shutil.disk_usage(E).free >= 38 << 30
    print(json.dumps(dict(passed=passed, terminal=str(destination), **pin(raw))))
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
