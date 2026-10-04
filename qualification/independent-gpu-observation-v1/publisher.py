"""Publish retained observer policy evidence, never execute its controller or tests."""
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys

L = Path(__file__).resolve().parent
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
NAME = 'p228-independent-gpu-observation-v1'
PACKAGE = L / 'proposals' / NAME
PURE = L / 'independent-observer-pure-v228-v1'
README = L / 'proposals/public-independent-observer-v1/README.md'
OUT = Path('/home/harsh/ferric-p227-integration/qualification/independent-gpu-observation-v1')
MANIFEST = dict(bytes=2964, sha256='9b06b7b929b5fddf36d73ce262abe01daf65e4eb1c2c810d2f61d1e1e3acf1a3')
EVIDENCE = {
    'complete.json': dict(bytes=362, sha256='732e0542d87eddcd52d3552f34a1ec82870afd3995ec839378d119a601b45323'),
    'sources-before.json': dict(bytes=4906, sha256='0aaa7bd88585a1a39d39701ce9879ddaab62b44eb3e89be63f09532f39ac5071'),
    'tests.log': dict(bytes=12590, sha256='367196f92ea7d7b3fa36151f71da0bac69eea9ceb2dfa2e06876796803788102'),
}
CONTROLLER_NAME = 'run_independent_observer_pure_p228_v1.py'
CONTROLLER = dict(bytes=3241, sha256='ea64c61c58cd0e86f7a23f36a17b2ec32673aefbad971c3c8f65960a00a3948a')
COUNTS = {'test_prepare.py': 18, 'test_run.py': 15, 'test_validation.py': 15,
          'test_children.py': 10, 'test_observe.py': 23}
MEMBERS = {'README.md', 'baseline-inputs.json', 'validation.py', 'prepare.py', 'run_case.py',
    'test_validation.py', 'test_run.py', 'test_prepare.py', 'run_row_facts_v2.py',
    'frozen_owned.py', 'observe.py', 'test_observe.py', 'preimage/prepare.py', 'preimage/run_case.py',
    'fixtures/actual-readelf.stdout', 'fixtures/actual-ldd.stdout', 'child_evidence.py', 'test_children.py'}
HELPERS = {
    'validation.py': 'cdaf6dc53208bbca8f23b2a3fa3eca9d28fe00ca4cfc74b36d225f3ebe8d28d1',
    'child_evidence.py': '2cbb74ada0950d1767c8b526008e9d9c7efb48d84649e1aea8f3d6661309edd3',
    'observe.py': '3ed69ac9af12d0442b0d7c6acef8f6081c923dd410de934df6fbf64784441ec6',
    'test_observe.py': '198f63e7c296982a0a99642256f6698036aa2cc39135a1f5b5ec1beb901fd6d5',
    'frozen_owned.py': 'ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583',
    'run_row_facts_v2.py': '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820',
    'baseline-inputs.json': 'fee39b8bf6343d27044ef2564366b842c1c5f23a251627b89794e1df42fb425f',
    'preimage/prepare.py': 'b6a043f470d28c3a68bdc76b432c646f7a0e10079d20e5281441c08a006a2582',
    'preimage/run_case.py': '6a83616bde6dbd50eae376fd74de513c51c7a26b12cc91b4155c5cf83def8847',
}


def require(value, message):
    if not value:
        raise RuntimeError(message)


def identity(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def read(path):
    require(path.resolve(strict=True) == path, 'canonical source: ' + str(path))
    with path.open('rb') as stream:
        before = os.fstat(stream.fileno())
        raw = stream.read((8 << 20) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda value: (value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns, value.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and len(raw) == before.st_size <= 8 << 20
        and stamp(before) == stamp(after) == stamp(path.lstat()), 'stable bounded source: ' + str(path))
    return raw


def parse(raw):
    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
        parse_constant=lambda _: (_ for _ in ()).throw(ValueError('nonfinite JSON')))


def census(directory):
    require(directory.resolve(strict=True) == directory and directory.is_dir(), 'canonical source directory')
    files = set()
    for path in directory.rglob('*'):
        require(not path.is_symlink() and (path.is_dir() or path.is_file()), 'no source aliases/special files')
        if path.is_file():
            files.add(str(path.relative_to(directory)))
    return files


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'unoptimized publisher')
    require(len(sys.argv) == 1, 'publisher has no execution or path override arguments')
    retained, outputs = {}, {}

    def keep(path, expected=None):
        raw = read(path)
        require(expected is None or identity(raw) == expected, 'exact pinned bytes: ' + str(path))
        retained[path] = raw
        return raw

    raw = keep(PACKAGE / 'manifest.json', MANIFEST)
    manifest = parse(raw)
    require(set(manifest) == {'files', 'gpu_execution', 'numerical_acceptance', 'production_authority',
        'pure_tests', 'schema', 'tests_executed'}
        and manifest['schema'] == 'ferric-p228-independent-gpu-observation-contracts-v1'
        and type(manifest['pure_tests']) is int and manifest['pure_tests'] == 81
        and all(manifest[key] is False for key in ('gpu_execution', 'numerical_acceptance',
            'production_authority', 'tests_executed')), 'frozen pre-execution manifest, not a test receipt')
    rows = manifest['files']
    require(type(rows) is list and len(rows) == 18 and {row['path'] for row in rows} == MEMBERS
        and census(PACKAGE) == MEMBERS | {'manifest.json'}, 'exact eighteen-file package census')
    originals = {}
    outputs['source/manifest.json'] = raw
    for row in rows:
        require(set(row) == {'path', 'bytes', 'sha256'}, 'exact frozen member')
        raw = keep(PACKAGE / row['path'], {key: row[key] for key in ('bytes', 'sha256')})
        originals[row['path']] = dict(row, path=str(E / NAME / row['path']))
        outputs['source/' + row['path']] = raw
        if row['path'] in HELPERS:
            require(row['sha256'] == HELPERS[row['path']], 'unchanged pinned custody/reference helper')
    require(census(PURE) == set(EVIDENCE), 'exact retained pure-test output census')
    for name, expected in EVIDENCE.items():
        outputs['evidence/' + name] = keep(PURE / name, expected)
    pure = parse(outputs['evidence/complete.json'])
    require(pure == dict(passed=True, tests=81, manifest_sha256=MANIFEST['sha256'],
        source_sha256=EVIDENCE['sources-before.json']['sha256'], controller_sha256=CONTROLLER['sha256'],
        gpu_execution=False, numerical_acceptance=False), 'actual closed passing pure receipt')
    require(parse(outputs['evidence/sources-before.json']) == originals, 'all eighteen tested source identities')
    outputs['evidence/' + CONTROLLER_NAME] = keep(L / CONTROLLER_NAME, CONTROLLER)

    expected_names = set()
    for filename, count in COUNTS.items():
        tree = ast.parse(outputs['source/' + filename], filename=filename)
        names = {(filename.removesuffix('.py'), node.name, method.name)
            for node in tree.body if isinstance(node, ast.ClassDef)
            for method in node.body if isinstance(method, ast.FunctionDef) and method.name.startswith('test_')}
        require(len(names) == count, 'complete authored test roster: ' + filename)
        expected_names.update(names)
    lines = outputs['evidence/tests.log'].decode('utf-8').splitlines()
    require(len(lines) == 86 and lines[81:] == ['', '-' * 70, 'Ran 81 tests in 5.328s', '', 'OK'],
            'actual complete unittest transcript, not mocked controller stdout')
    actual_names = []
    for line in lines[:81]:
        match = re.fullmatch(r'(test_[a-z0-9_]+) \((test_[a-z0-9_]+)\.([A-Za-z0-9_]+)\.(test_[a-z0-9_]+)\) \.\.\. ok', line)
        require(match is not None and match[1] == match[4], 'actual passing named test row')
        actual_names.append((match[2], match[3], match[4]))
    require(len(set(actual_names)) == 81 and set(actual_names) == expected_names,
            'every authored test passed exactly once, no skips/substitutions')
    assignments = {node.targets[0].id: ast.literal_eval(node.value)
        for node in ast.parse(outputs['source/prepare.py']).body
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name)
        and node.targets[0].id in {'TEST_RUNNER_SHA', 'PURE_TESTS'}}
    require(assignments == {'TEST_RUNNER_SHA': CONTROLLER['sha256'], 'PURE_TESTS': 81},
            'published observer binds the actual tested controller/count')

    outputs['README.md'] = keep(README)
    outputs['publisher.py'] = keep(Path(__file__).resolve())
    summary = dict(schema='FerricIndependentGpuObserverPolicyQualificationV1', date='2026-10-03',
        passed=True, authority='none', test_host='smci350-rck-g03-b19-03', tests_passed=81,
        tests_failed=0, tests_ignored=0, elapsed_test_seconds=5.328, test_counts=COUNTS,
        frozen_package=dict(path=str(E / NAME / 'manifest.json'), **MANIFEST),
        pure_receipt=dict(path=str(E / PURE.name / 'complete.json'), **EVIDENCE['complete.json']),
        pure_source_snapshot=dict(path=str(E / PURE.name / 'sources-before.json'), **EVIDENCE['sources-before.json']),
        pure_transcript=dict(path=str(E / PURE.name / 'tests.log'), **EVIDENCE['tests.log']),
        pure_controller=dict(path=str(E / CONTROLLER_NAME), **CONTROLLER),
        tested_source_pins=originals, source_files=18, synthetic_policy_tests=True,
        tested_source_snapshot_matches=True, unchanged_helper_sha256=HELPERS,
        published_files={name: identity(body) for name, body in sorted(outputs.items())},
        limitations=[
            'These 81 in-process CPU tests qualify controller policies and synthetic fixtures, not native execution.',
            'The frozen test runner checked all eighteen package members both before and after the suite.',
            'Test time is not a GPU latency, kernel comparison, or model throughput measurement.',
            'Actual image/binary deployment, current runtime/source/ISA reviews and six GPU cases remain separate gates.',
            'Independent numerical comparison must run in a separate bounded CPU leaf after all GPU post-audits and reaping.',
            'No full-prefix, full-model, production, or 700 tokens/s acceptance follows from this publication.',
        ], gpu_execution=False, native_execution=False, actual_capture_replay=False,
        independent_numerical_acceptance=False, full_model_acceptance=False,
        production_authority=False, performance_claim=False)
    outputs['result.json'] = (json.dumps(summary, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')
    for path, raw in retained.items():
        require(read(path) == raw, 'publication source changed after validation')
    require(OUT.parent.resolve(strict=True) == OUT.parent, 'canonical existing qualification parent')
    if os.path.lexists(OUT):
        require(census(OUT) == set(outputs), 'existing publication census differs')
        for name, raw in outputs.items():
            require(read(OUT / name) == raw, 'existing publication differs: ' + name)
    else:
        OUT.mkdir(mode=0o755)
        for name, raw in sorted(outputs.items()):
            path = OUT / name
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('xb') as stream:
                stream.write(raw)
                stream.flush()
                os.fsync(stream.fileno())
        require(census(OUT) == set(outputs), 'complete published file census')
        for name, raw in outputs.items():
            require(read(OUT / name) == raw, 'published bytes differ')
    print(json.dumps(dict(output=str(OUT), files=len(outputs), result=identity(outputs['result.json']))))


if __name__ == '__main__':
    main()
