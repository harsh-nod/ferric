"""Bounded CPU-only comparison of two retained GPU observations."""
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
P = E / 'p228-resident-state-runtime-comparison-v2'
EXPECTED = {
    'run.py': '76ef8db98601677c055d52fd332c92e75b1d14674f2e2fc34217b171aa7e9032',
    'test_run.py': '981c6ec6bafa72721b503106cbdc79ff254876c0c5ee2fb1859a94419612763f',
    'README.md': '5c7b4daeb38122f9d88326dac48fdf99fd9582be66cc98634fe3a2cd60f60e09',
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical source/result')
    require(path.stat().st_size <= 16 << 20, 'bounded source/result')
    raw = path.read_bytes()
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def snapshot():
    require({p.name for p in P.iterdir()} == set(EXPECTED), 'closed comparison adapter')
    result = {name: pin(P / name) for name in EXPECTED}
    require(all(result[name]['sha256'] == sha for name, sha in EXPECTED.items()), 'frozen source bytes')
    return result


def save(path, value):
    with path.open('x', encoding='ascii') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
    return pin(path)


def main():
    require(len(sys.argv) == 3 and sys.argv[1] in ('tests', 'compare')
        and re.fullmatch(r'resident-state-comparison-(pure|cpu)-v228-v[1-9][0-9]*', sys.argv[2]),
        'MODE FRESH_LABEL')
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ
        and 'PYTHONPATH' not in os.environ and 'PYTHONHOME' not in os.environ, 'isolated ordinary Python')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
        'bounded MI350 CPU identity')
    require(all(os.environ.get(k) == '' for k in
        ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'no visible GPU')
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
        require(suite.countTestCases() == 21, 'authored comparison test census')
        stream = io.StringIO()
        result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
        with (out / 'tests.log').open('x') as log:
            log.write(stream.getvalue())
        passed = result.wasSuccessful() and result.testsRun == 21 and not result.skipped
        details = dict(tests=result.testsRun, errors=len(result.errors), failures=len(result.failures),
            skipped=len(result.skipped), transcript=pin(out / 'tests.log'), synthetic_policy_tests_only=True)
        print(stream.getvalue(), end='', flush=True)
    else:
        import run as R
        require(Path(R.__file__).resolve() == P / 'run.py', 'actual comparison module')
        record, passed = R.execute(SimpleNamespace(
            diagnostic_reader=E / 'p228-independent-decode-diagnostic-v1/run.py',
            old_complete=E / R.OLD_LABEL / 'complete.json',
            candidate_complete=E / 'prefix-resident-state-decode-tf4-shared-full-currentness-gpu-v228-v1/complete.json',
            candidate_sha='f8e36cf2ed13435e7fb52b3a691f15f4135565d444038ee0d91292d94225fcba',
            output=out / 'comparison'))
        require(pin(Path(record['path'])) == record, 'actual retained comparison receipt')
        value = json.loads(Path(record['path']).read_bytes())
        require(value['comparison_completed'] is True and value['numerical_acceptance'] is False
            and value['performance_claim'] is False and value['gpu_time'] is False, 'invariance only')
        details = dict(comparison=record, summary=value['summary'])
    after = snapshot()
    require(before == after and pin(Path(__file__).resolve()) == controller, 'unchanged executed sources')
    value = dict(schema='ferric-p228-resident-state-comparison-cpu-v2', mode=sys.argv[1], passed=passed,
        controller=controller, sources_before=before_pin, sources_after=save(out / 'sources-after.json', after),
        source_postchecks_passed=True, details=details, elapsed_seconds=time.monotonic() - start,
        gpu_execution=False, numerical_acceptance=False, full_model_acceptance=False,
        production_authority=False, performance_claim=False)
    receipt = save(out / ('complete.json' if passed else 'failed.json'), value)
    print(json.dumps(dict(passed=passed, receipt=receipt)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
