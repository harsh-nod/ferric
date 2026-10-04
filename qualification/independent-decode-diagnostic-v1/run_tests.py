"""CPU-only qualification of the retained all-layer diagnostic reader."""
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import sys
import time
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-independent-decode-diagnostic-v1'
EXPECTED = {
    'run.py': '259f6f233be23da36eac213bfb5e0c905afa43461461fbb0305efcc45c08d930',
    'test_run.py': '2e3df90d54d4055682122c2405e234065ebd6e58973f1a1ebe4b5673e162889f',
    'README.md': '584601f0ef4e0c0a02ac618f3258d9fbc1568927ad65c619e7e7feb422ef0fbf',
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical source')
    raw = path.read_bytes()
    require(len(raw) <= 4 << 20, 'bounded source')
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def snapshot():
    require({p.name for p in P.iterdir()} == set(EXPECTED), 'closed tested package')
    result = {name: pin(P / name) for name in EXPECTED}
    require(all(result[n]['sha256'] == sha for n, sha in EXPECTED.items()), 'exact reviewed sources')
    return result


def write(path, value):
    with path.open('x', encoding='ascii') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
    return pin(path)


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ
            and 'PYTHONPATH' not in os.environ and 'PYTHONHOME' not in os.environ, 'isolated Python')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'bounded MI350 CPU identity')
    require(all(os.environ.get(k) == '' for k in
                ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'CPU-only visibility')
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        cap = min([cap] + [n for n in resource.getrlimit(kind) if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (cap, cap))
    before = snapshot(); controller = pin(Path(__file__).resolve())
    out = E / 'independent-decode-diagnostic-pure-v228-v1'
    require(not os.path.lexists(out), 'fresh pure result'); out.mkdir(mode=0o700)
    sources_before = write(out / 'sources-before.json', before)
    sys.path.insert(0, str(P))
    import test_run as T
    require(Path(T.__file__).resolve() == P / 'test_run.py', 'actual tests')
    suite = unittest.defaultTestLoader.loadTestsFromModule(T)
    require(suite.countTestCases() == 12, 'exact test count')
    start = time.monotonic(); stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
    after = snapshot()
    require(before == after and pin(Path(__file__).resolve()) == controller, 'unchanged source closure')
    with (out / 'tests.log').open('x') as log:
        log.write(stream.getvalue())
    passed = result.wasSuccessful() and result.testsRun == 12 and not result.skipped
    value = dict(schema='ferric-p228-independent-decode-diagnostic-pure-v1', passed=passed,
        tests=result.testsRun, errors=len(result.errors), failures=len(result.failures), skipped=len(result.skipped),
        elapsed_seconds=time.monotonic() - start, controller=controller,
        sources_before=sources_before, sources_after=write(out / 'sources-after.json', after),
        transcript=pin(out / 'tests.log'), source_postchecks_passed=True,
        gpu_execution=False, numerical_acceptance=False, full_model_acceptance=False,
        production_authority=False, performance_claim=False)
    receipt = write(out / ('complete.json' if passed else 'failed.json'), value)
    print(stream.getvalue(), end='', flush=True)
    print(json.dumps(dict(passed=passed, receipt=receipt)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
