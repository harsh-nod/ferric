"""Run the frozen numerical unit suite on MI350 without opening a GPU."""
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import resource
import sys
import time
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
P = E / 'p228-prefix-profile-numerical-v1'
OUT = E / 'prefix-profile-tests-mi350-v228-v1'
MANIFEST = 'd99d27e9912b486ad90b212d08fb297d894033da9845330bea36f199ed4f0122'


def pin(path):
    raw = path.read_bytes()
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def verify():
    path = P / 'source-manifest.json'
    assert pin(path)['sha256'] == MANIFEST
    manifest = json.loads(path.read_bytes())
    assert manifest['tests_expected'] == 20 and len(manifest['files']) == 30
    for row in manifest['files']:
        path = E / row['path']
        assert path.resolve(strict=True) == path and not path.is_symlink()
        assert pin(path) == {key: row[key] for key in ('bytes', 'sha256')}
    return manifest


def main():
    if sys.flags.optimize or not sys.dont_write_bytecode:
        raise RuntimeError('Requires normal assertions and bytecode disabled')
    assert platform.node() == 'smci350-rck-g03-b19-03' and os.getuid() == 9661
    assert sorted(os.sched_getaffinity(0)) == [8, 9]
    assert os.getpriority(os.PRIO_PROCESS, 0) == 10
    assert os.environ['OPENBLAS_NUM_THREADS'] == os.environ['OMP_NUM_THREADS'] == '1'
    assert resource.getrlimit(resource.RLIMIT_AS) == (2 << 30, 2 << 30)
    verify()
    OUT.mkdir(mode=0o700)
    sys.path.insert(0, str(P))
    import test_compare_profile
    assert test_compare_profile.C.helpers()[2].np.__version__ == '2.2.6'
    start = time.monotonic()
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromModule(test_compare_profile))
    elapsed = time.monotonic() - start
    verify()
    (OUT / 'tests.log').write_text(stream.getvalue(), encoding='utf-8')
    passed = result.wasSuccessful() and result.testsRun == 20 and not result.skipped
    body = dict(schema='ferric-prefix-profile-cpu-tests-v1', passed=passed,
                tests=result.testsRun, skipped=len(result.skipped), failures=len(result.failures),
                errors=len(result.errors), host=platform.node(), python=platform.python_version(),
                numpy=test_compare_profile.ProfileTests.original[2].np.__version__,
                elapsed_host_seconds=elapsed, cpu_affinity=sorted(os.sched_getaffinity(0)), nice=10,
                source_manifest=pin(P / 'source-manifest.json'), source_postchecks_passed=True,
                controller=pin(Path(__file__).resolve()), transcript=pin(OUT / 'tests.log'),
                gpu_execution=False, numerical_acceptance=False, full_model_acceptance=False,
                production_authority=False, performance_claim=False,
                synthetic_fixtures_only=True, fixed_reference_bounds=True)
    path = OUT / ('complete.json' if passed else 'failed.json')
    with path.open('x', encoding='ascii') as output:
        json.dump(body, output, indent=2, sort_keys=True)
        output.write('\n')
    print(stream.getvalue(), end='', flush=True)
    print(json.dumps(dict(output=str(path), **pin(path))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
