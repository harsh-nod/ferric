"""Run the frozen prefix timestamp CPU policy suite, never a native/GPU leaf."""
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
P = E / 'p228-prefix-raw-timestamps-cpu-v1'


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
    require(value['schema'] == 'ferric-p228-prefix-raw-timestamps-cpu-package-v1', 'package schema')
    sources = {}
    for row in value['files']:
        name = row['path']
        require(type(name) is str and '/' not in name and name not in sources
                and name not in ('', '.', '..', 'manifest.json'), 'unique flat member')
        actual = pin(P / name)
        require(all(actual[key] == row[key] for key in ('bytes', 'sha256')), 'member hash/extent')
        sources[name] = actual
    require({path.name for path in P.iterdir()} == set(sources) | {'manifest.json'}, 'exact package census')
    tests = {name for name in sources if name.startswith('test_') and name.endswith('.py')}
    require(type(value['test_census']) is dict and set(value['test_census']) == tests
            and all(type(n) is int and n > 0 for n in value['test_census'].values())
            and type(value['pure_tests']) is int
            and value['pure_tests'] == sum(value['test_census'].values()), 'closed test census')
    return value, manifest, sources


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and sys.dont_write_bytecode and 'PYTHONPATH' not in os.environ
            and 'PYTHONHOME' not in os.environ, 'unoptimized isolated module path')
    require(len(sys.argv) == 3 and re.fullmatch('[0-9a-f]{64}', sys.argv[1])
            and re.fullmatch(r'prefix-raw-timestamps-pure-v228-v[1-9][0-9]{0,8}', sys.argv[2]),
            'MANIFEST_SHA FRESH_LABEL')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'bounded ASROCK CPU identity')
    require(all(os.environ.get(k) == '' for k in
                ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'no visible GPU')
    for kind, cap in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        old = resource.getrlimit(kind)
        ceiling = min([cap] + [n for n in old if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (ceiling, ceiling))
    start = time.monotonic()
    controller = pin(Path(__file__).resolve())
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
            and pin(Path(__file__).resolve()) == controller, 'unchanged tested sources')
    write(out / 'sources-after.json', after)
    with (out / 'tests.log').open('x', encoding='utf-8') as log:
        log.write(stream.getvalue())
    passed = result.wasSuccessful() and result.testsRun == value['pure_tests'] and not result.skipped
    record = dict(schema='ferric-p228-prefix-raw-timestamps-pure-v1', passed=passed,
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
