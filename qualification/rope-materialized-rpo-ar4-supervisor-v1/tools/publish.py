"""Publish source and actual CPU policy-test evidence without importing tested code."""
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
Q = Path('/home/harsh/ferric-p227-integration/qualification/rope-materialized-rpo-ar4-supervisor-v1')
PACKAGE = 'p228-rope-materialized-rpo-ar4-gpu-v1'
SELF = PL / 'p228-rope-materialized-rpo-ar4-supervisor-publication-v1/publish.py'
WRAPPER = 'run_rope_materialized_rpo_ar4_gpu_pure_p228_v1.py'
WRAPPER_SHA = '770863f54cb0ea42967ce42df9c69a049ca8acbd3976b66e1082daa63260a955'
MANIFEST = (2741, 'be12154c8567c423531771df00c9d0d4211e733b88b5b798c6e602ae47dac9af')
CENSUS = {'test_run.py': 11, 'test_decode_validation.py': 17, 'test_intake.py': 16,
    'test_silu_admission.py': 8, 'test_rope_admission.py': 13}
MEMBERS = {'INTAKE.md', 'README.md', 'decode_validation.py', 'intake.py', 'layer_validation.py',
    'run.py', 'run_row_facts_v2.py', 'stage_core.py', 'smoke_validation.py',
    'prefix_contracts.py', WRAPPER, *CENSUS}
HELPERS = {
    'run_row_facts_v2.py': '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820',
    'layer_validation.py': '757c077979b7ee02d4c0e7d222e75292cd9d417927f84ae9a8134344f8e217b9',
    'stage_core.py': '7e3d64e80e66bacbbba05ac3236189249949bb95ea3f717150822cf45740eec2',
    'smoke_validation.py': '9bf460451e722452da9c49168c730615d32e9b73b200c579f003e9e45238ea26',
    'decode_validation.py': '0eb96d4ac5e10e7f6ac10b018692c56f969286949681be336cb6040d56f01422',
    'test_decode_validation.py': 'f36d5a8cc321cb3df26049a5941ffd52e0ebe401213cce9e03cc7fd987c3c253',
    'prefix_contracts.py': '694be9d199cd92b2d11ff4fe8401ad3728497bd8846b5d6924311330f3d9de1a',
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


def suite(complete, sha, sources, inventory, controller):
    directory = complete.parent
    require(directory.parent == L and complete.name == 'complete.json'
        and re.fullmatch(r'rope-materialized-rpo-ar4-gpu-pure-v228-v[1-9][0-9]{0,8}', directory.name)
        and directory.resolve(strict=True) == directory
        and {p.name for p in directory.iterdir()} == RAW, 'exact four retained pure files')
    records, data = {}, {}
    for filename in sorted(RAW):
        records[filename], data[filename] = add(directory / filename, E / directory.name / filename,
                                                'pure/' + filename)
    require(records['complete.json']['sha256'] == sha, 'caller-authenticated actual pure receipt')
    value = parse(data['complete.json'])
    count = sum(len(v) for v in inventory.values())
    require(value['schema'] == 'ferric-p228-rope-materialized-rpo-ar4-gpu-pure-v1'
        and value['passed'] is True and type(value['tests']) is int and value['tests'] == count == 65
        and value['source_postchecks_passed'] is True and value['synthetic_policy_tests_only'] is True,
        'actual passing pure suite')
    require(all(type(value[k]) is int and value[k] == 0 for k in ('errors', 'failures', 'skipped'))
        and all(value[k] is False for k in ('native_execution', 'gpu_execution', 'numerical_acceptance',
            'full_model_acceptance', 'production_authority', 'performance_claim')), 'pure-test scope only')
    for key, filename in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'),
                          ('transcript', 'tests.log')):
        require(value[key] == records[filename], 'complete joins exact retained pure body')
    require(parse(data['sources-before.json']) == parse(data['sources-after.json']) == sources,
            'exact before/after source roster and bytes')
    expected = sorted(name for rows in inventory.values() for name in rows)
    log = data['tests.log'].decode('utf-8')
    passes = []
    for match in re.finditer(r'^(test_[^\s]+) \(([^()\n]+)\) \.\.\. ok$', log, re.M):
        method, name = match.groups()
        passes.append(name if name.endswith('.' + method) else name + '.' + method)
    require(sorted(passes) == expected and len(passes) == len(set(passes)) == count
        and re.search(r'\nRan 65 tests in [0-9.]+s\n\nOK\n?\Z', log)
        and not re.search(r'\.\.\. (?:FAIL|ERROR|skipped)|^FAILED', log, re.M), 'exact named unittest transcript')
    require(value['manifest_sha256'] == MANIFEST[1] and value['controller_sha256'] == controller['sha256']
        and value['source_sha256'] == records['sources-before.json']['sha256']
        and value['test_inventory_before'] == value['test_inventory_after'] == inventory,
        '65-test manifest/controller/named-inventory joins')
    return dict(receipt=records['complete.json'], controller=controller, passed=count,
        errors=0, failures=0, skipped=0, inventory=inventory, sources=records['sources-before.json'],
        transcript=records['tests.log'], elapsed_seconds=value['elapsed_seconds'],
        synthetic_policy_tests_only=True, native_execution=False)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary data-only Python')
    require(len(sys.argv) == 3 and re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'LOCAL_COMPLETE_PATH ACTUAL_SHA')
    require(Path(__file__).resolve() == SELF, 'exact publication source path')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {p.name for p in Q.iterdir()} <= {'README.md'},
                'fresh publication with optional root README only')
        if (Q / 'README.md').exists():
            body(Q / 'README.md')
    else:
        require(Q.parent.resolve(strict=True) == Q.parent, 'canonical publication parent')
    base = PL / PACKAGE
    require({p.name for p in base.iterdir()} == MEMBERS | {'manifest.json'}, 'exact local package roster')
    manifest_pin, raw = add(base / 'manifest.json', E / PACKAGE / 'manifest.json', 'source/manifest.json', MANIFEST)
    manifest = parse(raw)
    require(manifest['schema'] == 'ferric-p228-rope-materialized-rpo-ar4-gpu-package-v1'
        and type(manifest['pure_tests']) is int and manifest['pure_tests'] == 65
        and manifest['test_census'] == CENSUS and len(manifest['files']) == 16
        and {r['path'] for r in manifest['files']} == MEMBERS, 'exact frozen sixteen-member package')
    sources, inventory = {}, {}
    for row in manifest['files']:
        require(set(row) == {'path', 'bytes', 'sha256'}, 'closed source row')
        name = row['path']
        sources[name], raw = add(base / name, E / PACKAGE / name, 'source/' + name,
                                  (row['bytes'], row['sha256']))
        if name in HELPERS:
            require(row['sha256'] == HELPERS[name], 'unchanged helper identity')
        if name in CENSUS:
            inventory[name] = names(raw, name)
            require(len(inventory[name]) == CENSUS[name], 'actual per-module test declarations')
    wrapper = body(L / WRAPPER)
    controller, _ = add(L / WRAPPER, E / WRAPPER, 'tools/' + WRAPPER, (len(wrapper), WRAPPER_SHA))
    require(body(base / WRAPPER) == wrapper, 'package and actual external wrapper bytes equal')
    measured = suite(Path(sys.argv[1]), sys.argv[2], sources, inventory, controller)
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
    result = dict(schema='ferric-p228-rope-materialized-rpo-ar4-supervisor-publication-v1',
        package=manifest_pin, publisher=publisher, suite=measured,
        files={name: dict(original=pin, bytes=len(raw), sha256=digest(raw))
               for name, (raw, pin) in sorted(COPIES.items())},
        source_postchecks_passed=True, data_only_publication=True, tests_rerun=False,
        tested_modules_imported=False, native_execution=False, gpu_execution=False,
        runtime_audit_executed=False, numerical_acceptance=False, full_model_acceptance=False,
        production_authority=False, performance_claim=False,
        root_readme_not_part_of_authenticated_copies=True,
        limitations=['Synthetic CPU policy tests only; no native run or runtime audit is performed by this suite.',
            'The existing CPU1037 executables and prior runtime audits are unchanged, not rerun by this publisher.',
            'External devices, models, prior GPU admissions and numerical comparisons are not revalidated here.'])
    output = (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    with (Q / 'result.json').open('xb') as stream:
        stream.write(output)
    expected = set(COPIES) | {'result.json'} | ({'README.md'} if (Q / 'README.md').exists() else set())
    require({str(p.relative_to(Q)) for p in Q.rglob('*') if p.is_file()} == expected,
            'closed published file roster')
    print(json.dumps(dict(path=str(Q / 'result.json'), bytes=len(output), sha256=digest(output),
        published_files=len(COPIES) + 1, supervisor_tests=65, native_execution=False), sort_keys=True))


if __name__ == '__main__':
    main()
