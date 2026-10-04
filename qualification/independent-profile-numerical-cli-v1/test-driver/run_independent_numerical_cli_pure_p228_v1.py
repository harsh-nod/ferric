"""Run 17 frozen pure wiring tests in-process; root supplies a 120s wall bound.

No observer, native process, reference engine, or GPU is launched by this runner.
Use python3 -B under the existing root-owned CPU8,9/nice10 bounded invocation.
"""
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
P = E / 'p228-independent-profile-numerical-cli-v1'
OUT = E / 'independent-numerical-cli-pure-v228-v1'
MEMBERS = {'run.py', 'test_run.py', 'README.md'}


def pin(path):
    assert path.resolve(strict=True) == path and path.is_file()
    raw = path.read_bytes()
    assert len(raw) <= 4 << 20
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def write(path, value):
    with path.open('x', encoding='ascii') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')


def verify(digest):
    assert P.resolve(strict=True) == P and P.is_dir()
    assert {path.name for path in P.iterdir()} == MEMBERS | {'manifest.json'}
    manifest = pin(P / 'manifest.json')
    assert manifest['sha256'] == digest
    value = json.loads((P / 'manifest.json').read_bytes())
    assert value['schema'] == 'ferric-p228-independent-profile-numerical-cli-package-v1'
    assert type(value['pure_tests']) is int and value['pure_tests'] == 17
    assert value['tests_executed'] is False and value['gpu_execution'] is False
    assert value['numerical_acceptance'] is False and value['production_authority'] is False
    assert type(value['files']) is list and len(value['files']) == 3
    assert {row['path'] for row in value['files']} == MEMBERS
    sources = {}
    for row in value['files']:
        assert set(row) == {'path', 'bytes', 'sha256'} and type(row['bytes']) is int
        actual = pin(P / row['path'])
        assert {key: actual[key] for key in ('bytes', 'sha256')} == {
            key: row[key] for key in ('bytes', 'sha256')}
        sources[row['path']] = actual
    return manifest, sources


def main():
    if sys.flags.optimize or 'PYTHONOPTIMIZE' in os.environ:
        raise RuntimeError('unoptimized Python required')
    assert len(sys.argv) == 2 and re.fullmatch('[0-9a-f]{64}', sys.argv[1])
    assert sys.dont_write_bytecode and 'PYTHONPATH' not in os.environ and 'PYTHONHOME' not in os.environ
    assert os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
    assert sorted(os.sched_getaffinity(0)) == [8, 9] and os.getpriority(os.PRIO_PROCESS, 0) == 10
    assert all(os.environ.get(name) == '' for name in
        ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'))
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        old = resource.getrlimit(kind)
        value = min([cap] + [word for word in old if word != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))
    started = time.monotonic()
    controller = pin(Path(__file__).resolve())
    manifest, before = verify(sys.argv[1])
    assert OUT.parent.resolve(strict=True) == OUT.parent and not os.path.lexists(OUT)
    OUT.mkdir(mode=0o700)
    write(OUT / 'sources-before.json', before)
    sys.path.insert(0, str(P))
    import test_run
    assert Path(test_run.__file__).resolve() == P / 'test_run.py'
    suite = unittest.defaultTestLoader.loadTestsFromModule(test_run)
    assert suite.countTestCases() == 17
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
    after_manifest, after = verify(sys.argv[1])
    assert manifest == after_manifest and before == after
    assert pin(Path(__file__).resolve()) == controller
    elapsed = time.monotonic() - started
    assert elapsed < 120
    write(OUT / 'sources-after.json', after)
    with (OUT / 'tests.log').open('x', encoding='utf-8') as transcript:
        transcript.write(stream.getvalue())
    passed = result.wasSuccessful() and result.testsRun == 17 and not result.skipped
    body = dict(schema='ferric-p228-independent-profile-numerical-cli-pure-v1',
        passed=passed, tests=result.testsRun, errors=len(result.errors), failures=len(result.failures),
        skipped=len(result.skipped), package_manifest=manifest, controller=controller,
        test_source=before['test_run.py'], transcript=pin(OUT / 'tests.log'),
        sources_before=pin(OUT / 'sources-before.json'), sources_after=pin(OUT / 'sources-after.json'),
        source_pins=after, source_postchecks_passed=True, elapsed_seconds=elapsed,
        synthetic_wiring_tests_only=True, actual_observer_replay=False,
        real_numerical_reference_execution=False, native_execution=False, gpu_execution=False,
        numerical_acceptance=False, production_authority=False, performance_claim=False)
    output = OUT / ('complete.json' if passed else 'failed.json')
    write(output, body)
    print(stream.getvalue(), end='', flush=True)
    print(json.dumps(dict(passed=passed, output=pin(output))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
