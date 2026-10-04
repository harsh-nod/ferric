"""Bounded original arithmetic tests and historical MLP capture replay on MI350."""
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import sys
import time
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-historical-mlp-capture-replay-v1'
REF = E.parent / 'resident-layer-tp2-v218/reference'
ORACLE = E.parent / 'wave-output-lowering-v216/tp2-residual-reference/reference.py'
EXPECTED = {
    P / 'run.py': 'a537be4b19098180907ff41023ed2fd7ca19f12213a5fd4c33e9ef38f59aeb89',
    P / 'test_run.py': '81ed1719d232046eac5a8bea25af33627f79ad1de6c4303cf9cfe31f05aebc37',
    P / 'README.md': '397429d9375120ad349bd41925f839d8c1976d70ea465adf75d88784fe267682',
    REF / 'reference.py': '2e41d6e5715cc561bf818fdee794e4ced74a77f93acd4961d8b15f427f196c6e',
    REF / 'extract.py': '76872f3558d6f90c21b9eee3ec0ba4ebdd79168b620338faa6bdbdd8b78bdf4a',
    REF / 'policy.json': '9497e55e70a42630d50b74ea33b0856a385d051fb60bc7f23124ee5cbd46e44b',
    REF / 'test_reference.py': '4de379796dc83561809aaf070a1e8f129a72e7a664d8a3b113d0d278280b524d',
    ORACLE: '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3',
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
    require({p.name for p in P.iterdir()} == {'run.py', 'test_run.py', 'README.md'}, 'closed adapter')
    result = {str(path): pin(path) for path in EXPECTED}
    require(all(result[str(path)]['sha256'] == sha for path, sha in EXPECTED.items()), 'frozen sources')
    return result


def save(path, value):
    with path.open('x', encoding='ascii') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
    return pin(path)


def main():
    require(len(sys.argv) == 3 and sys.argv[1] in ('tests', 'replay')
            and re.fullmatch(r'historical-mlp-(pure|replay)-v228-v[1-9][0-9]*', sys.argv[2]),
            'MODE FRESH_LABEL')
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ
            and 'PYTHONPATH' not in os.environ and 'PYTHONHOME' not in os.environ, 'isolated Python')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'bounded MI350 CPU identity')
    require(all(os.environ.get(k) == '' for k in
                ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'CPU-only visibility')
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
    sys.path[:0] = [str(P), str(REF)]
    start = time.monotonic()
    if sys.argv[1] == 'tests':
        import test_run as T
        import test_reference as A
        require(Path(T.__file__).resolve() == P / 'test_run.py'
                and Path(A.__file__).resolve() == REF / 'test_reference.py', 'actual test modules')
        suites = [unittest.defaultTestLoader.loadTestsFromModule(m) for m in (T, A)]
        require([s.countTestCases() for s in suites] == [22, 19], 'adapter and original arithmetic census')
        stream = io.StringIO()
        result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.TestSuite(suites))
        with (out / 'tests.log').open('x') as log:
            log.write(stream.getvalue())
        passed = result.wasSuccessful() and result.testsRun == 41 and not result.skipped
        details = dict(tests=result.testsRun, adapter_tests=22, original_arithmetic_tests=19,
            errors=len(result.errors), failures=len(result.failures), skipped=len(result.skipped),
            transcript=pin(out / 'tests.log'))
        print(stream.getvalue(), end='', flush=True)
    else:
        import run as R
        require(Path(R.__file__).resolve() == P / 'run.py', 'actual replay source')
        value = R.replay(E, E / 'p228-residual-capture-replay-v1/run.py',
            E / 'p227-prefix-layer-replay-v1/layer_validation.py', REF,
            E.parent / 'resident-layer-tp2-v218/reference-fixtures-v1/fixture',
            E.parent / 'p225-tiles-tf4-runtime-v1/tiles.hsaco')
        passed = value['passed'] is True and value['conditional_stage_checks_passed'] is True
        details = dict(historical_result=save(out / 'mlp.json', value))
    after = snapshot()
    require(before == after and pin(Path(__file__).resolve()) == controller, 'unchanged executed sources')
    value = dict(schema='ferric-p228-historical-mlp-cpu-v1', mode=sys.argv[1], passed=passed,
        controller=controller, sources_before=before_pin, sources_after=save(out / 'sources-after.json', after),
        source_postchecks_passed=True, details=details, elapsed_seconds=time.monotonic() - start,
        gpu_execution=False, new_v7_image_checked=False, full_layer_acceptance=False,
        full_model_acceptance=False, production_authority=False, performance_claim=False)
    receipt = save(out / ('complete.json' if passed else 'failed.json'), value)
    print(json.dumps(dict(passed=passed, receipt=receipt)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
