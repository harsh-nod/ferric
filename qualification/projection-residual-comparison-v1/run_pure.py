"""Bounded comparison tests; synthetic data only, no GPU execution."""
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import sys
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-projection-residual-comparison-v1'
OUT = E / 'projection-residual-comparison-pure-v228-v1'
SOURCES = {
    'p228-projection-residual-comparison-v1/comparison.py': 'f8074e6eaedbe51d26307b596401487c589c834688cd6c395ee8b3fac60ef149',
    'p228-projection-residual-comparison-v1/test_comparison.py': '261ed6237c0be76ccd6bcdcf4ad9071e7d2fe78a9578d3990684791b1c95bcde',
    'p228-layer0-current-diagnostic-v1/current.py': 'eb103b681c54b0c880bc6e692363208dae837eb322845bab7e593bdea6330d7b',
    'p228-layer0-current-diagnostic-v1/compare.py': '1598e22a3460a9ed2c350fb5d2b5fec6b8c37648b53bb2f1fc714c1fb3506d3f',
    'p228-layer0-current-diagnostic-v1/diagnostics.py': '38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf',
    'p228-layer0-current-diagnostic-v1/test_current.py': '55260e410cb601e9af38cb6e7c35c6c153c7a0f7d11269a40d581cad1c56b3b4',
    'p228-output-residual-boundary-v1/boundary.py': 'a5fac7b587739999623167d81c163fc18ebe9a2baf986da1b192b597e7fb0b9d',
    'p228-independent-layer-reference-v1/helpers/residual_oracle.py': '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3',
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical input')
    raw = path.read_bytes()
    require(len(raw) <= 1 << 20, 'bounded source or report')
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def save(name, value):
    with (OUT / name).open('x', encoding='ascii') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode
        and all(k not in os.environ for k in ('PYTHONOPTIMIZE', 'PYTHONPATH', 'PYTHONHOME')),
        'ordinary isolated Python')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
        'bounded ASROCK CPU identity')
    require(all(os.environ.get(k) == '' for k in
        ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'no visible GPU')
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        values = [cap] + [v for v in resource.getrlimit(kind) if v != resource.RLIM_INFINITY]
        resource.setrlimit(kind, (min(values), min(values)))
    controller = pin(Path(__file__).resolve())
    require(len(sys.argv) == 2 and controller['sha256'] == sys.argv[1], 'selected controller bytes')
    before = {name: pin(E / name) for name in SOURCES}
    require(all(before[name]['sha256'] == sha for name, sha in SOURCES.items()), 'selected test source bytes')
    OUT.mkdir(mode=0o700)
    save('sources-before.json', before)
    sys.path[:0] = [str(P), str(E / 'p228-layer0-current-diagnostic-v1'),
                   str(E / 'p228-output-residual-boundary-v1')]
    import test_comparison as tests
    import comparison
    import current
    import compare
    import diagnostics
    import test_current
    import boundary
    modules = (tests, comparison, current, compare, diagnostics, test_current, boundary)
    require({str(Path(m.__file__).resolve().relative_to(E)) for m in modules}
        == set(SOURCES) - {'p228-independent-layer-reference-v1/helpers/residual_oracle.py'},
        'actual imported module paths')
    suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
    require(suite.countTestCases() == 20, 'closed twenty-test census')
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
    after = {name: pin(E / name) for name in SOURCES}
    require(before == after and pin(Path(__file__).resolve()) == controller, 'source postchecks')
    save('sources-after.json', after)
    with (OUT / 'tests.log').open('x', encoding='utf-8') as log:
        log.write(stream.getvalue())
    passed = result.wasSuccessful() and result.testsRun == 20 and not result.skipped
    record = dict(schema='ferric-p228-projection-residual-comparison-pure-v1', passed=passed,
        tests=result.testsRun, errors=len(result.errors), failures=len(result.failures),
        skipped=len(result.skipped), controller=controller, source_postchecks_passed=True,
        sources_before=pin(OUT / 'sources-before.json'), sources_after=pin(OUT / 'sources-after.json'),
        transcript=pin(OUT / 'tests.log'), synthetic_policy_tests_only=True,
        compiler_execution=False, framework_execution=False, runtime_audit_executed=False,
        gpu_execution=False, numerical_acceptance=False, production_authority=False, performance_claim=False)
    filename = 'complete.json' if passed else 'failed.json'
    save(filename, record)
    print(stream.getvalue(), end='', flush=True)
    print(json.dumps(dict(passed=passed, receipt=pin(OUT / filename))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
