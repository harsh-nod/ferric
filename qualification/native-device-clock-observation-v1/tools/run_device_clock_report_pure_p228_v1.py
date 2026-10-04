"""Bounded raw-clock report tests; no native execution or clock calibration."""
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import sys
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-device-clock-report-v1'
OUT = E / 'device-clock-report-pure-v228-v1'
FILES = ('report.py', 'test_report.py')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical input')
    raw = path.read_bytes()
    require(len(raw) <= 1 << 20, 'bounded source')
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def save(name, value):
    with (OUT / name).open('x', encoding='ascii') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode
        and all(k not in os.environ for k in ('PYTHONOPTIMIZE', 'PYTHONPATH', 'PYTHONHOME')),
        'ordinary isolated Python')
    require(len(sys.argv) == 3 and all(re.fullmatch('[0-9a-f]{64}', s) for s in sys.argv[1:]),
        'REPORT_SHA TEST_SHA')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
        'bounded MI350 CPU identity')
    require(all(os.environ.get(k) == '' for k in
        ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'no visible GPU')
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        values = [cap] + [v for v in resource.getrlimit(kind) if v != resource.RLIM_INFINITY]
        resource.setrlimit(kind, (min(values), min(values)))
    controller = pin(Path(__file__).resolve())
    before = {name: pin(P / name) for name in FILES}
    require([before[name]['sha256'] for name in FILES] == sys.argv[1:], 'selected test source bytes')
    OUT.mkdir(mode=0o700)
    save('sources-before.json', before)
    sys.path.insert(0, str(P))
    import test_report as tests
    import report as selected
    require(Path(tests.__file__).resolve() == P / FILES[1]
        and Path(selected.__file__).resolve() == P / FILES[0], 'actual module paths')
    suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
    require(suite.countTestCases() == 5, 'closed five-test census')
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
    after = {name: pin(P / name) for name in FILES}
    require(before == after and pin(Path(__file__).resolve()) == controller, 'source postchecks')
    save('sources-after.json', after)
    with (OUT / 'tests.log').open('x', encoding='utf-8') as log:
        log.write(stream.getvalue())
    passed = result.wasSuccessful() and result.testsRun == 5 and not result.skipped
    record = dict(schema='ferric-p228-device-clock-report-pure-v1', passed=passed,
        tests=result.testsRun, errors=len(result.errors), failures=len(result.failures),
        skipped=len(result.skipped), controller=controller, source_postchecks_passed=True,
        sources_before=pin(OUT / 'sources-before.json'), sources_after=pin(OUT / 'sources-after.json'),
        transcript=pin(OUT / 'tests.log'), synthetic_policy_tests_only=True,
        runtime_audit_executed=False, gpu_execution=False, numerical_acceptance=False,
        production_authority=False, performance_claim=False)
    filename = 'complete.json' if passed else 'failed.json'
    save(filename, record)
    print(stream.getvalue(), end='', flush=True)
    print(json.dumps(dict(passed=passed, receipt=pin(OUT / filename))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
