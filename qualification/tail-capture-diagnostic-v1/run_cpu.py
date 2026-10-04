"""Bounded CPU-only tests and conditional final-norm/head replay on MI350."""
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import sys
import time
from types import SimpleNamespace
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-tail-capture-diagnostic-v1'
EXPECTED = {
    'run.py': '1b45818cff51f7ab802329dd83ef248370cc50945695a994a78e7b197694295c',
    'test_run.py': 'dbf7bfb54634ebe4842c2cf0abe9e5d139158dba58e8b4156563e10efa9386bb',
    'README.md': '09f6820ae758fb74c68bc72e7f5211f9c2c6ee6d22ae752ed6af48c3c4d7fb61',
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical source/result')
    raw = path.read_bytes()
    require(len(raw) <= 16 << 20, 'bounded source/result')
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def snapshot():
    require({p.name for p in P.iterdir()} == set(EXPECTED), 'closed tail adapter')
    result = {name: pin(P / name) for name in EXPECTED}
    require(all(result[name]['sha256'] == sha for name, sha in EXPECTED.items()), 'frozen source bytes')
    return result


def save(path, value):
    with path.open('x', encoding='ascii') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
    return pin(path)


def main():
    require(len(sys.argv) in (3, 4) and sys.argv[1] in ('tests', 'replay')
            and re.fullmatch(r'tail-capture-(pure|diagnostic)-v228-v[1-9][0-9]*', sys.argv[2])
            and (len(sys.argv) == 4) == (sys.argv[1] == 'replay'), 'MODE FRESH_LABEL [P222_DIRECTORY]')
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ
            and 'PYTHONPATH' not in os.environ and 'PYTHONHOME' not in os.environ, 'isolated ordinary Python')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'bounded MI350 CPU identity')
    require(all(os.environ.get(k) == '' for k in
                ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'no visible GPU')
    require(all(os.environ.get(k) == '1' for k in
                ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS', 'NUMEXPR_NUM_THREADS')),
            'single-threaded numerical libraries')
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        cap = min([cap] + [n for n in resource.getrlimit(kind) if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (cap, cap))
    before = snapshot(); controller = pin(Path(__file__).resolve())
    out = E / sys.argv[2]
    require(not os.path.lexists(out), 'fresh output'); out.mkdir(mode=0o700)
    before_pin = save(out / 'sources-before.json', before)
    sys.path.insert(0, str(P))
    start = time.monotonic()
    if sys.argv[1] == 'tests':
        import test_run as T
        require(Path(T.__file__).resolve() == P / 'test_run.py', 'actual test module')
        suite = unittest.defaultTestLoader.loadTestsFromModule(T)
        require(suite.countTestCases() == 22, 'authored adapter test census')
        stream = io.StringIO()
        result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
        with (out / 'tests.log').open('x') as log:
            log.write(stream.getvalue())
        passed = result.wasSuccessful() and result.testsRun == 22 and not result.skipped
        details = dict(tests=result.testsRun, errors=len(result.errors), failures=len(result.failures),
            skipped=len(result.skipped), transcript=pin(out / 'tests.log'), synthetic_policy_tests_only=True)
        print(stream.getvalue(), end='', flush=True)
    else:
        import run as R
        require(Path(R.__file__).resolve() == P / 'run.py', 'actual tail replay module')
        record = R.execute(SimpleNamespace(
            diagnostic_reader=E / 'p228-independent-decode-diagnostic-v1/run.py',
            observation=E / R.TF4 / 'complete.json',
            uploads=E / 'prefix-layer-gpu-v227-v1/native/baseline-uploads.json',
            program=E / 'prefix-layer-gpu-v227-v1/native/baseline-program.json',
            p222_directory=Path(sys.argv[3]),
            model_directory=Path('/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target'),
            output=out / 'diagnostic'))
        require(pin(Path(record['path'])) == record, 'actual retained diagnostic receipt')
        value = json.loads(Path(record['path']).read_bytes())
        passed = value['passed'] is True and value['compared_stage_rows'] == 8
        require(value['numerical_acceptance'] is False and value['acceptance_threshold'] is None,
                'diagnostic only; no invented numerical bound')
        details = dict(diagnostic=record, compared_stage_rows=8, acceptance_threshold=None)
    after = snapshot()
    require(before == after and pin(Path(__file__).resolve()) == controller, 'unchanged executed sources')
    value = dict(schema='ferric-p228-tail-capture-cpu-v1', mode=sys.argv[1], passed=passed,
        controller=controller, sources_before=before_pin, sources_after=save(out / 'sources-after.json', after),
        source_postchecks_passed=True, details=details, elapsed_seconds=time.monotonic() - start,
        gpu_execution=False, numerical_acceptance=False, full_model_acceptance=False,
        production_authority=False, performance_claim=False)
    receipt = save(out / ('complete.json' if passed else 'failed.json'), value)
    print(json.dumps(dict(passed=passed, receipt=receipt)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
