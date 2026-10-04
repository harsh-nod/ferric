"""Bounded CPU-only tests for the explicit raw-device observation package."""
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
P = E / 'p228-device-timing-gpu-v2'
RUNNER = E / 'run_device_timing_gpu_pure_p228_v2.py'
FILES = {'INTAKE.md', 'README.md', 'baseline_comparison.py', 'baseline_diagnostic.py',
    'device_validation.py', 'host_validation.py', 'intake.py', 'layer_validation.py',
    'run.py', 'run_row_facts_v2.py', 'test_device_validation.py', 'test_intake.py', 'test_run.py'}
TEST_CENSUS = {'test_run.py': 13, 'test_device_validation.py': 17, 'test_intake.py': 17}
COPIED_SHA = {
    'run_row_facts_v2.py': '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820',
    'layer_validation.py': '757c077979b7ee02d4c0e7d222e75292cd9d417927f84ae9a8134344f8e217b9',
    'host_validation.py': '2f32426a80aca05125112ee3f023c075caea2f9360cbbde1fcb6c3b54cc4f73f',
    'baseline_comparison.py': '413e8bb4ea5637cb3ca67003433b62bf7cb90832c4f88101d2c9ea9fa7bcc512',
    'baseline_diagnostic.py': '259f6f233be23da36eac213bfb5e0c905afa43461461fbb0305efcc45c08d930',
}


def require(value, message):
    if not value:
        raise RuntimeError(message)


def pin(path):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical source')
    raw = path.read_bytes()
    require(len(raw) <= 4 << 20, 'bounded source')
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def write(path, value):
    with path.open('x', encoding='ascii') as stream:
        json.dump(value, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')


def verify(expected):
    manifest = pin(P / 'manifest.json')
    require(manifest['sha256'] == expected, 'actual frozen package')
    value = json.loads((P / 'manifest.json').read_bytes())
    require(value['schema'] == 'ferric-p228-device-timing-package-v1', 'package schema')
    sources = {}
    for row in value['files']:
        name = row['path']
        require(type(name) is str and name in FILES and name not in sources, 'unique closed flat member')
        actual = pin(P / name)
        require(all(actual[key] == row[key] for key in ('bytes', 'sha256')), 'member hash/extent')
        sources[name] = actual
    require(set(sources) == FILES and {path.name for path in P.iterdir()} == FILES | {'manifest.json'},
            'exact thirteen-file package census')
    for name, sha in COPIED_SHA.items():
        require(sources[name]['sha256'] == sha, 'unchanged copied helper: ' + name)
    require(value['test_census'] == TEST_CENSUS
            and all(type(n) is int for n in value['test_census'].values())
            and type(value['pure_tests']) is int and value['pure_tests'] == sum(TEST_CENSUS.values()) == 47,
            'closed 13+17+17 test census')
    return value, manifest, sources


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and sys.dont_write_bytecode and 'PYTHONPATH' not in os.environ
            and 'PYTHONHOME' not in os.environ, 'unoptimized isolated module path')
    require(len(sys.argv) == 3 and re.fullmatch('[0-9a-f]{64}', sys.argv[1])
            and re.fullmatch(r'device-timing-gpu-pure-v228-v[1-9][0-9]{0,8}', sys.argv[2]),
            'MANIFEST_SHA FRESH_LABEL')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'bounded MI350 CPU identity')
    require(all(os.environ.get(k) == '' for k in
                ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'no visible GPU')
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        old = resource.getrlimit(kind)
        ceiling = min([cap] + [n for n in old if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (ceiling, ceiling))
    start = time.monotonic()
    require(Path(__file__).resolve() == RUNNER, 'exact final retained controller path')
    controller = pin(RUNNER)
    value, manifest, before = verify(sys.argv[1])
    out = E / sys.argv[2]
    require(out.parent.resolve(strict=True) == out.parent and not os.path.lexists(out), 'fresh result')
    out.mkdir(mode=0o700)
    write(out / 'sources-before.json', before)
    sys.path.insert(0, str(P))
    suite = unittest.TestSuite()
    for filename, count in sorted(value['test_census'].items()):
        module = __import__(filename[:-3])
        require(Path(module.__file__).resolve() == P / filename, 'actual test module')
        selected = unittest.defaultTestLoader.loadTestsFromModule(module)
        require(selected.countTestCases() == count, 'actual per-module test count')
        suite.addTests(selected)
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
    after_value, after_manifest, after = verify(sys.argv[1])
    require((after_value, after_manifest, after) == (value, manifest, before)
            and pin(RUNNER) == controller, 'unchanged tested sources')
    write(out / 'sources-after.json', after)
    with (out / 'tests.log').open('x', encoding='utf-8') as log:
        log.write(stream.getvalue())
    passed = result.wasSuccessful() and result.testsRun == value['pure_tests'] and not result.skipped
    record = dict(schema='ferric-p228-device-timing-pure-v1', passed=passed,
        tests=result.testsRun, errors=len(result.errors), failures=len(result.failures), skipped=len(result.skipped),
        manifest_sha256=manifest['sha256'], controller_sha256=controller['sha256'],
        source_sha256=pin(out / 'sources-before.json')['sha256'],
        sources_before=pin(out / 'sources-before.json'), sources_after=pin(out / 'sources-after.json'),
        transcript=pin(out / 'tests.log'), source_postchecks_passed=True,
        elapsed_seconds=time.monotonic() - start, synthetic_policy_tests_only=True,
        native_execution=False, gpu_execution=False, numerical_acceptance=False,
        full_model_acceptance=False, production_authority=False, performance_claim=False)
    path = out / ('complete.json' if passed else 'failed.json')
    write(path, record)
    print(stream.getvalue(), end='', flush=True)
    print(json.dumps(dict(passed=passed, receipt=pin(path))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
