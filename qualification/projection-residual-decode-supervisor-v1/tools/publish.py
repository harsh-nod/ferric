"""Publish authenticated pure-test evidence only; no tested module is imported."""
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PL = L / 'proposals'
Q = Path('/home/harsh/ferric-p227-integration/qualification/projection-residual-decode-supervisor-v1')
PACKAGE = 'p228-projection-residual-decode-gpu-v1'
SELECTOR = 'p228-projection-residual-decode-runtime-v1'
WRAPPERS = PL / 'p228-projection-residual-decode-pure-wrappers-v1'
SELF = PL / 'p228-projection-residual-decode-supervisor-publication-v1/publish.py'
MANIFEST = (2021, '6dfd492f7dc8da8bd8eaa2f8e9da9bcbe36cd94756ba321bda538b3abb6fbb35')
CENSUS = {'test_run.py': 11, 'test_decode_validation.py': 12, 'test_intake.py': 12}
MEMBERS = {'INTAKE.md', 'README.md', 'decode_validation.py', 'intake.py', 'layer_validation.py',
    'run.py', 'run_row_facts_v2.py', 'stage_core.py', 'smoke_validation.py', *CENSUS}
HELPERS = {
    'run_row_facts_v2.py': '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820',
    'layer_validation.py': '757c077979b7ee02d4c0e7d222e75292cd9d417927f84ae9a8134344f8e217b9',
    'stage_core.py': '7e3d64e80e66bacbbba05ac3236189249949bb95ea3f717150822cf45740eec2',
    'smoke_validation.py': '9bf460451e722452da9c49168c730615d32e9b73b200c579f003e9e45238ea26',
}
SELECTOR_SHA = {
    'audit_projection_residual_decode_runtime.py': '4bd93966b539e053f09b0b7d535ccf0d5c4abcdb75f0cb8e4190e8ed3e98a52e',
    'test_audit_projection_residual_decode_runtime.py': 'fdd05276ee8053689016fc4c54f9d6c836484f62b5e1fed34e4db474cce7e2cb',
}
WRAPPER_SHA = {
    'gpu': 'a8f1d7d647c736c11050d28251fee8245413344a4c50e834fd5103a23427147c',
    'runtime': '283b6fc0db593fad46b80c72b3b117a31a85f0117d01eeb643dc5c48bd99df77',
}
RESULTS = {
    'gpu': ('projection-residual-decode-gpu-pure-v228-v1', 8514,
        'e3162fa0b19b7370c81acdc1a902eaf76a61fac2e33b2ccbb04b6f71ac53ac89'),
    'runtime': ('projection-residual-decode-runtime-audit-pure-v228-v1', 1510,
        '1b403c2270b9b5a3058f16a9133651a832a7f363b3582f5fb830d848356fe921'),
}
RAW = {'complete.json', 'sources-before.json', 'sources-after.json', 'tests.log'}
READS, COPIES = {}, {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def body(path):
    require(path.resolve(strict=True) == path, 'canonical source: ' + str(path))
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= 4 << 20,
            'bounded unaliased regular file')
    raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(path.lstat()) and len(raw) == before.st_size, 'stable source read')
    identity = (len(raw), digest(raw))
    require(path not in READS or READS[path] == identity, 'input changed during publication')
    READS[path] = identity
    return raw


def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    def invalid(_):
        raise RuntimeError('nonfinite JSON')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def add(path, original, target, expected=None):
    raw = body(path)
    require(expected is None or (len(raw), digest(raw)) == expected, 'exact published input: ' + str(path))
    relative = Path(target)
    require(not relative.is_absolute() and '..' not in relative.parts and target not in COPIES,
            'unique closed relative destination')
    pin = dict(path=str(original), bytes=len(raw), sha256=digest(raw))
    COPIES[target] = (raw, pin)
    return pin, raw


def names(raw, filename):
    result = [filename[:-3] + '.' + cls.name + '.' + method.name
        for cls in ast.parse(raw).body if isinstance(cls, ast.ClassDef)
        for method in cls.body if isinstance(method, ast.FunctionDef) and method.name.startswith('test_')]
    require(result and len(result) == len(set(result)), 'unique authored test declarations')
    return sorted(result)


def suite(role, sources, inventory, controller):
    label, size, sha = RESULTS[role]
    directory = L / label
    require(directory.resolve(strict=True) == directory and {p.name for p in directory.iterdir()} == RAW,
            'exact four retained pure files')
    records, data = {}, {}
    for filename in sorted(RAW):
        records[filename], data[filename] = add(directory / filename, E / label / filename,
            'tests/' + role + '/' + filename, (size, sha) if filename == 'complete.json' else None)
    value = parse(data['complete.json'])
    count = sum(len(v) for v in inventory.values())
    schema = 'ferric-p228-projection-residual-decode-' + ('gpu' if role == 'gpu' else 'runtime-audit') + '-pure-v1'
    require(value['schema'] == schema and value['passed'] is True and type(value['tests']) is int
        and value['tests'] == count and value['source_postchecks_passed'] is True
        and value['synthetic_policy_tests_only'] is True, 'actual passing pure suite')
    require(all(type(value[k]) is int and value[k] == 0 for k in ('errors', 'failures', 'skipped'))
        and all(value[k] is False for k in ('gpu_execution', 'numerical_acceptance',
            'production_authority', 'performance_claim')), 'no failures or native/numerical authority')
    for key, filename in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'),
                          ('transcript', 'tests.log')):
        require(value[key] == records[filename], 'complete joins exact retained pure body')
    require(parse(data['sources-before.json']) == parse(data['sources-after.json']) == sources,
            'exact before/after source roster and bytes')
    expected = sorted(name for rows in inventory.values() for name in rows)
    log = data['tests.log'].decode('utf-8')
    passes = [m.group(1) for m in re.finditer(r'^test_[^\s]+ \(([^()\n]+)\) \.\.\. ok$', log, re.M)]
    require(sorted(passes) == expected and len(passes) == len(set(passes)) == count
        and re.search(r'\nRan ' + str(count) + r' tests in [0-9.]+s\n\nOK\n?\Z', log)
        and not re.search(r'\.\.\. (?:FAIL|ERROR|skipped)|^FAILED', log, re.M), 'exact named unittest transcript')
    if role == 'gpu':
        require(value['manifest_sha256'] == MANIFEST[1] and value['controller_sha256'] == controller['sha256']
            and value['source_sha256'] == records['sources-before.json']['sha256']
            and value['test_inventory_before'] == value['test_inventory_after'] == inventory
            and value['native_execution'] is False and value['full_model_acceptance'] is False,
            '35-test manifest/controller/named-inventory joins')
    else:
        require(value['controller'] == controller and value['runtime_audit_executed'] is False,
                '9-test controller identity; no runtime audit performed')
    return dict(receipt=records['complete.json'], controller=controller, passed=count,
        errors=0, failures=0, skipped=0, inventory=inventory, sources=records['sources-before.json'],
        transcript=records['tests.log'], synthetic_policy_tests_only=True, native_execution=False)


def main():
    require(len(sys.argv) == 1 and not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ,
            'no arguments; ordinary data-only Python')
    require(Path(__file__).resolve() == SELF, 'exact publication source path')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} <= {'README.md'},
                'fresh publication with optional root README only')
        if (Q / 'README.md').exists():
            body(Q / 'README.md')
    else:
        require(Q.parent.resolve(strict=True) == Q.parent, 'canonical publication parent')
    manifest_pin, raw = add(PL / PACKAGE / 'manifest.json', E / PACKAGE / 'manifest.json',
                            'source/manifest.json', MANIFEST)
    manifest = parse(raw)
    require(manifest['schema'] == 'ferric-p228-projection-residual-decode-gpu-package-v1'
        and type(manifest['pure_tests']) is int and manifest['pure_tests'] == 35
        and manifest['test_census'] == CENSUS and len(manifest['files']) == 12
        and {r['path'] for r in manifest['files']} == MEMBERS, 'exact frozen twelve-member package')
    sources, inventory = {}, {}
    for row in manifest['files']:
        require(set(row) == {'path', 'bytes', 'sha256'}, 'closed source row')
        name = row['path']
        sources[name], raw = add(PL / PACKAGE / name, E / PACKAGE / name, 'source/' + name,
                                  (row['bytes'], row['sha256']))
        if name in HELPERS:
            require(row['sha256'] == HELPERS[name], 'unchanged helper identity')
        if name in CENSUS:
            inventory[name] = names(raw, name)
            require(len(inventory[name]) == CENSUS[name], 'actual per-module test declarations')
    selector_sources, selector_inventory = {}, {}
    for name, sha in SELECTOR_SHA.items():
        raw = body(PL / SELECTOR / name)
        selector_sources[name], _ = add(PL / SELECTOR / name, E / SELECTOR / name,
                                       'runtime/' + name, (len(raw), sha))
        if name.startswith('test_'):
            selector_inventory[name] = names(raw, name)
            require(len(selector_inventory[name]) == 9, 'nine selector tests')
    controllers = {}
    for role, sha in WRAPPER_SHA.items():
        name = 'run_projection_residual_decode_' + role + '_pure_p228_v1.py'
        raw = body(WRAPPERS / name)
        controllers[role], _ = add(WRAPPERS / name, E / name, 'tools/' + name, (len(raw), sha))
    gpu = suite('gpu', sources, inventory, controllers['gpu'])
    runtime = suite('runtime', selector_sources, selector_inventory, controllers['runtime'])
    publisher, _ = add(SELF, SELF, 'tools/publish.py')
    for path in list(READS):
        body(path)
    Q.mkdir(exist_ok=True)
    for target, (raw, _) in COPIES.items():
        path = Q / target
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
        require(path.read_bytes() == raw, 'published body rehash')
    for path in list(READS):
        body(path)
    result = dict(schema='ferric-p228-projection-residual-decode-supervisor-publication-v1',
        package=manifest_pin, publisher=publisher, suites=dict(supervisor=gpu, runtime_selector=runtime),
        files={name: dict(original=pin, bytes=len(raw), sha256=digest(raw))
               for name, (raw, pin) in sorted(COPIES.items())},
        source_postchecks_passed=True, data_only_publication=True, tests_rerun=False,
        tested_modules_imported=False, native_execution=False, gpu_execution=False,
        runtime_audit_executed=False, numerical_acceptance=False, full_model_acceptance=False,
        production_authority=False, performance_claim=False,
        root_readme_not_part_of_authenticated_copies=True,
        limitations=['Synthetic CPU policy tests only; no native run or runtime audit is performed by these suites.',
            'External CPU artifacts, devices, models, dependencies and prior GPU admissions are not revalidated here.'])
    output = (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    with (Q / 'result.json').open('xb') as stream:
        stream.write(output)
    expected = set(COPIES) | {'result.json'} | ({'README.md'} if (Q / 'README.md').exists() else set())
    require({str(p.relative_to(Q)) for p in Q.rglob('*') if p.is_file()} == expected,
            'closed published file roster')
    print(json.dumps(dict(path=str(Q / 'result.json'), bytes=len(output), sha256=digest(output),
        published_files=len(COPIES) + 1, supervisor_tests=35, runtime_selector_tests=9,
        native_execution=False), sort_keys=True))


if __name__ == '__main__':
    main()
