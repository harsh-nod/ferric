"""Bounded CPU-only policy tests for the full-currentness projection AR4 observer."""
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
P = E / 'p228-projection-ordered-segment-gpu-v3'
RUNNER = E / 'run_projection_ordered_segment_gpu_pure_p228_v3.py'
FILES = {'INTAKE.md', 'README.md', 'decode_validation.py', 'intake.py', 'layer_validation.py',
    'run.py', 'run_row_facts_v2.py', 'stage_core.py', 'smoke_validation.py',
    'test_decode_validation.py', 'test_intake.py', 'test_run.py', 'test_silu_admission.py',
    'prefix_contracts.py', 'test_rope_admission.py', 'test_linked_admission.py',
    'run_projection_ordered_segment_gpu_pure_p228_v3.py',
    'host_validation.py', 'observer_cpu.py', 'test_host_validation.py', 'test_observer_cpu.py', 'shared_cpu.py', 'pair.py',
    'test_shared_cpu.py', 'test_shared_host.py', 'test_pair.py', 'ordered_cpu.py', 'ordered_validation.py', 'ordered_pair.py',
    'test_ordered_cpu.py', 'test_ordered_validation.py', 'test_ordered_host.py', 'test_ordered_pair.py'}
TEST_CENSUS = {'test_run.py': 15, 'test_decode_validation.py': 17, 'test_intake.py': 16,
    'test_silu_admission.py': 8, 'test_rope_admission.py': 7, 'test_linked_admission.py': 15,
    'test_host_validation.py': 8, 'test_observer_cpu.py': 8,
    'test_shared_cpu.py': 8, 'test_shared_host.py': 8, 'test_pair.py': 13,
    'test_ordered_validation.py': 20, 'test_ordered_cpu.py': 10,
    'test_ordered_host.py': 8, 'test_ordered_pair.py': 13}
COPIED_SHA = {
    'run_row_facts_v2.py': '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820',
    'layer_validation.py': '757c077979b7ee02d4c0e7d222e75292cd9d417927f84ae9a8134344f8e217b9',
    'stage_core.py': '7e3d64e80e66bacbbba05ac3236189249949bb95ea3f717150822cf45740eec2',
    'smoke_validation.py': '9bf460451e722452da9c49168c730615d32e9b73b200c579f003e9e45238ea26',
    'decode_validation.py': '0eb96d4ac5e10e7f6ac10b018692c56f969286949681be336cb6040d56f01422',
    'test_decode_validation.py': 'f36d5a8cc321cb3df26049a5941ffd52e0ebe401213cce9e03cc7fd987c3c253',
    'prefix_contracts.py': '694be9d199cd92b2d11ff4fe8401ad3728497bd8846b5d6924311330f3d9de1a',
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
    require(value['schema'] == 'ferric-p228-projection-ordered-segment-gpu-package-v3', 'package schema')
    sources = {}
    for row in value['files']:
        name = row['path']
        require(type(name) is str and name in FILES and name not in sources, 'unique closed flat member')
        actual = pin(P / name)
        require(all(actual[key] == row[key] for key in ('bytes', 'sha256')), 'member hash/extent')
        sources[name] = actual
    require(set(sources) == FILES and {path.name for path in P.iterdir()} == FILES | {'manifest.json'},
            'exact thirty-three-file package census')
    for name, sha in COPIED_SHA.items():
        require(sources[name]['sha256'] == sha, 'unchanged copied helper: ' + name)
    require(value['test_census'] == TEST_CENSUS
            and all(type(n) is int for n in value['test_census'].values())
            and type(value['pure_tests']) is int and value['pure_tests'] == sum(TEST_CENSUS.values()) == 174,
            'closed retained122 plus52 additive test census')
    require(sources[RUNNER.name] == dict(pin(RUNNER), path=str(P / RUNNER.name)),
            'package and external wrapper bytes match')
    return value, manifest, sources


def named_cases(suite):
    names = []
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            names.extend(named_cases(item))
        else:
            require(isinstance(item, unittest.TestCase), 'actual unittest case')
            names.append(item.id())
    require(len(names) == len(set(names)), 'unique named tests within suite')
    return sorted(names)


def selected_tests(value):
    suite, inventory = unittest.TestSuite(), {}
    for filename, count in sorted(value['test_census'].items()):
        module = __import__(filename[:-3])
        require(Path(module.__file__).resolve() == P / filename, 'actual test module')
        selected = unittest.defaultTestLoader.loadTestsFromModule(module)
        names = named_cases(selected)
        require(selected.countTestCases() == len(names) == count
                and all(name.startswith(filename[:-3] + '.') for name in names),
                'actual named per-module inventory')
        inventory[filename] = names
        suite.addTests(selected)
    require(len(named_cases(suite)) == value['pure_tests'], 'exact whole named test inventory')
    return suite, inventory


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and sys.dont_write_bytecode and 'PYTHONPATH' not in os.environ
            and 'PYTHONHOME' not in os.environ, 'unoptimized isolated module path')
    require(len(sys.argv) == 3 and re.fullmatch('[0-9a-f]{64}', sys.argv[1])
            and re.fullmatch(r'projection-ordered-segment-gpu-pure-v228-v[1-9][0-9]{0,8}', sys.argv[2]),
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
    suite, names_before = selected_tests(value)
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
    after_value, after_manifest, after = verify(sys.argv[1])
    require((after_value, after_manifest, after) == (value, manifest, before)
            and pin(RUNNER) == controller, 'unchanged tested sources')
    _, names_after = selected_tests(after_value)
    require(names_before == names_after, 'unchanged complete named test inventory')
    write(out / 'sources-after.json', after)
    with (out / 'tests.log').open('x', encoding='utf-8') as log:
        log.write(stream.getvalue())
    passed = result.wasSuccessful() and result.testsRun == value['pure_tests'] and not result.skipped
    record = dict(schema='ferric-p228-projection-ordered-segment-gpu-pure-v3', passed=passed,
        tests=result.testsRun, errors=len(result.errors), failures=len(result.failures), skipped=len(result.skipped),
        manifest_sha256=manifest['sha256'], controller_sha256=controller['sha256'],
        source_sha256=pin(out / 'sources-before.json')['sha256'],
        sources_before=pin(out / 'sources-before.json'), sources_after=pin(out / 'sources-after.json'),
        transcript=pin(out / 'tests.log'), source_postchecks_passed=True,
        test_inventory_before=names_before, test_inventory_after=names_after,
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
