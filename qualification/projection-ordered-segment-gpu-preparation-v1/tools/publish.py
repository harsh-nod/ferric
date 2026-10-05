"""Publish source and actual CPU policy-test evidence without importing tested code."""
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import shlex
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PL = L / 'proposals'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
Q = Path('/home/harsh/ferric-p227-integration/qualification/projection-ordered-segment-gpu-preparation-v1')
PACKAGE = 'p228-projection-ordered-segment-gpu-v3'
SELF = PL / 'p228-projection-ordered-segment-gpu-publication-v1/publish.py'
WRAPPER = 'run_projection_ordered_segment_gpu_pure_p228_v3.py'
WRAPPER_SHA = 'c75159850159feab4ae4092c09e649ef1b7b62907b37c741678b7c02a8fffddc'
MANIFEST = (5666, '054b11e3eae9389dad6dc42f216b2ccfc81115436340cab572fda7f93d8d4d69')
CENSUS = {'test_run.py': 15, 'test_decode_validation.py': 17, 'test_intake.py': 16,
    'test_silu_admission.py': 8, 'test_rope_admission.py': 7, 'test_linked_admission.py': 15,
    'test_host_validation.py': 8, 'test_observer_cpu.py': 8, 'test_shared_cpu.py': 8,
    'test_shared_host.py': 8, 'test_pair.py': 13,
    'test_ordered_validation.py': 20, 'test_ordered_cpu.py': 10,
    'test_ordered_host.py': 8, 'test_ordered_pair.py': 13}
MEMBERS = {'INTAKE.md', 'README.md', 'decode_validation.py', 'intake.py', 'layer_validation.py',
    'run.py', 'run_row_facts_v2.py', 'stage_core.py', 'smoke_validation.py',
    'prefix_contracts.py', 'host_validation.py', 'observer_cpu.py', 'shared_cpu.py',
    'pair.py', 'ordered_cpu.py', 'ordered_validation.py', 'ordered_pair.py', WRAPPER, *CENSUS}
HELPERS = {
    'run_row_facts_v2.py': '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820',
    'layer_validation.py': '757c077979b7ee02d4c0e7d222e75292cd9d417927f84ae9a8134344f8e217b9',
    'stage_core.py': '7e3d64e80e66bacbbba05ac3236189249949bb95ea3f717150822cf45740eec2',
    'smoke_validation.py': '9bf460451e722452da9c49168c730615d32e9b73b200c579f003e9e45238ea26',
    'decode_validation.py': '0eb96d4ac5e10e7f6ac10b018692c56f969286949681be336cb6040d56f01422',
    'test_decode_validation.py': 'f36d5a8cc321cb3df26049a5941ffd52e0ebe401213cce9e03cc7fd987c3c253',
    'test_silu_admission.py': '224e3d83ab56d62c0fc69b6e0d07e2baaa5423596d6013604a8dd64c499f72be',
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
    require(directory.parent in (L, W) and complete.name == 'complete.json'
        and re.fullmatch(r'projection-ordered-segment-gpu-pure-v228-v[1-9][0-9]{0,8}', directory.name)
        and directory.resolve(strict=True) == directory
        and {p.name for p in directory.iterdir()} == RAW, 'exact four retained pure files')
    records, data = {}, {}
    for filename in sorted(RAW):
        records[filename], data[filename] = add(directory / filename, E / directory.name / filename,
                                                'pure/' + filename)
    require(records['complete.json']['sha256'] == sha, 'caller-authenticated actual pure receipt')
    value = parse(data['complete.json'])
    count = sum(len(v) for v in inventory.values())
    require(value['schema'] == 'ferric-p228-projection-ordered-segment-gpu-pure-v3'
        and value['passed'] is True and type(value['tests']) is int and value['tests'] == count == 174
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
        and re.search(r'\nRan 174 tests in [0-9.]+s\n\nOK\n?\Z', log)
        and not re.search(r'\.\.\. (?:FAIL|ERROR|skipped)|^FAILED', log, re.M), 'exact named unittest transcript')
    require(value['manifest_sha256'] == MANIFEST[1] and value['controller_sha256'] == controller['sha256']
        and value['source_sha256'] == records['sources-before.json']['sha256']
        and value['test_inventory_before'] == value['test_inventory_after'] == inventory,
        '174-test manifest/controller/named-inventory joins')
    return dict(receipt=records['complete.json'], controller=controller, passed=count,
        errors=0, failures=0, skipped=0, inventory=inventory, sources=records['sources-before.json'],
        transcript=records['tests.log'], elapsed_seconds=value['elapsed_seconds'],
        synthetic_policy_tests_only=True, native_execution=False)



ADAPTERS = (
    ('p228-projection-ordered-segment-deployment-v3', 16,
     ('audit_projection_ordered_segment_runtime.py', '07b3d7e8cbb7890d310d7a69f21cf9a9ae76c67956739332dc54ed6fb216eb80'),
     ('test_audit_projection_ordered_segment_runtime.py', 'b69d15f8784278fc9995a2d944f01c517e843ce7a5efc8498542276a0288212f'),
     ('README.md', 'bd9bcd9ebced4efe5032ec1ffe643c3383e7bffd22cc10ac1302c845313fe0db')),
    ('p228-projection-ordered-segment-inputs-v3', 13,
     ('prepare.py', 'a9c7162c886ddd3a9cbf6b73c0d77f800b18722de2698d6cc1ae427ec7d409f0'),
     ('test_prepare.py', 'f77bbdacc37bc6d263cf1050be898a84535050e2b7e5821f6bfb1aba874fcdb4'),
     ('README.md', '00b87ea166a15236815db51658965b183a53a36aeb0f97c9e7c2de44830e0369')),
)


def adapters(path, sha):
    require(path.parent == L and path.name == 'ordered-segment-mi350-adapter-tests-primary-v228-v3.json',
            'exact root primary-observation location')
    primary, raw = add(path, path, 'primary/adapter-tests.json')
    require(primary['sha256'] == sha, 'caller-authenticated root observation')
    value = parse(raw)
    require(value['schema'] == 'ferric-ordered-segment-mi350-adapter-tests-primary-v1'
        and value['host'] == 'smci350-rck-g03-b19-03' and value['body_hashes_unchanged'] is True
        and len(value['tests']) == 2
        and all(value[k] is False for k in ('gpu_execution', 'numerical_acceptance', 'performance_claim')),
        'actual two-suite CPU-only primary observation')
    require(all(type(value[k]) is str and value[k] for k in
        ('scope', 'body_hashes_before_chunk', 'body_hashes_after_chunk')), 'primary observation source checks')
    observations = []
    for index, (package, count, controller, test, readme) in enumerate(ADAPTERS):
        rows, test_names = {}, None
        for role, (name, expected) in zip(('controller', 'test', 'readme'), (controller, test, readme)):
            rows[role], data = add(PL / package / name, E / package / name, 'adapters/' + package + '/' + name)
            require(rows[role]['sha256'] == expected, 'exact reviewed adapter source')
            if role == 'test':
                test_names = names(data, name)
                require(len(test_names) == count, 'authored adapter inventory count')
        observed = value['tests'][index]
        require(type(observed['exit_code']) is int and observed['exit_code'] == 0
            and type(observed['passed']) is int and observed['passed'] == count
            and type(observed['result_chunk']) is str and observed['result_chunk']
            and type(observed['reported_seconds']) in (int, float) and observed['reported_seconds'] >= 0
            and all(observed[role + '_sha256'] == rows[role]['sha256'] for role in rows),
            'recorded primary result and all source identities')
        expected_command = ['env', '-u', 'PYTHONPATH', '-u', 'PYTHONHOME', '-u', 'PYTHONOPTIMIZE',
            'HIP_VISIBLE_DEVICES=', 'ROCR_VISIBLE_DEVICES=', 'CUDA_VISIBLE_DEVICES=', 'GPU_DEVICE_ORDINAL=',
            'timeout', '--signal=TERM', '--kill-after=5s', '120s', 'prlimit', '--as=536870912',
            '--cpu=60', '--fsize=16777216', '--core=0', 'taskset', '-c', '8,9', 'nice', '-n', '10',
            '/usr/bin/python3', '-B', str(E / package / test[0]), '-v']
        require(shlex.split(observed['command']) == expected_command, 'exact bounded primary test command')
        observations.append(dict(passed=count, observation=observed, sources=rows,
            authored_inventory=test_names, full_named_transcript_retained=False))
    return dict(primary=primary, tests=observations, passed=29,
        root_primary_tool_observation=True, supervisor_receipt=False,
        runtime_audits_performed=False, native_execution=False)


def failed_v2(path, sha):
    package = 'p228-projection-ordered-segment-gpu-v2'
    wrapper = 'run_projection_ordered_segment_gpu_pure_p228_v2.py'
    manifest_expected = (5666, 'adceb1ffceef393ad5f06a797cec0e2d037453b31798fb59c86d29e0109a1177')
    failure_expected = (37877, 'e040e6da0ed04682841b92b4c229fbb86bca8949cb7a3e73c9d08966be9fe393')
    directory = path.parent
    require(directory.parent in (L, W) and directory.name == 'projection-ordered-segment-gpu-pure-v228-v2'
        and path.name == 'failed.json' and directory.resolve(strict=True) == directory
        and {p.name for p in directory.iterdir()} == (RAW - {'complete.json'}) | {'failed.json'},
        'exact preserved failed V2 tree')
    records, data = {}, {}
    for name in sorted((RAW - {'complete.json'}) | {'failed.json'}):
        records[name], data[name] = add(directory / name, E / directory.name / name, 'failed-v2/pure/' + name)
    require((records['failed.json']['bytes'], records['failed.json']['sha256']) == failure_expected
        and sha == failure_expected[1], 'actual immutable V2 failure')
    manifest_pin, raw = add(PL / package / 'manifest.json', E / package / 'manifest.json',
        'failed-v2/source/manifest.json', manifest_expected)
    manifest = parse(raw)
    old_members = (MEMBERS - {WRAPPER}) | {wrapper}
    require(manifest['schema'] == 'ferric-p228-projection-ordered-segment-gpu-package-v2'
        and manifest['test_census'] == CENSUS and manifest['pure_tests'] == 174
        and len(manifest['files']) == len(old_members) == 33
        and {r['path'] for r in manifest['files']} == old_members, 'frozen failed V2 package roster')
    sources, inventory = {}, {}
    for row in manifest['files']:
        require(set(row) == {'path', 'bytes', 'sha256'}, 'closed failed source row')
        name = row['path']; raw = body(PL / package / name)
        require((len(raw), digest(raw)) == (row['bytes'], row['sha256']), 'failed source body identity')
        sources[name] = dict(row, path=str(E / package / name))
        if name in CENSUS:
            inventory[name] = names(raw, name)
            require(len(inventory[name]) == CENSUS[name], 'failed package authored test census')
        if name in {'intake.py', 'test_linked_admission.py', wrapper}:
            add(PL / package / name, E / package / name, 'failed-v2/source/' + name,
                (row['bytes'], row['sha256']))
    value = parse(data['failed.json'])
    require(value['schema'] == 'ferric-p228-projection-ordered-segment-gpu-pure-v2'
        and value['passed'] is False and type(value['tests']) is int and value['tests'] == 174
        and value['source_postchecks_passed'] is True and value['synthetic_policy_tests_only'] is True
        and all(type(value[k]) is int and value[k] == n for k, n in
                (('errors', 1), ('failures', 0), ('skipped', 0)))
        and all(value[k] is False for k in ('native_execution', 'gpu_execution', 'numerical_acceptance',
            'full_model_acceptance', 'production_authority', 'performance_claim')), 'preserved failure, not qualification')
    for key, name in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'),
                      ('transcript', 'tests.log')):
        require(value[key] == records[name], 'failed receipt/raw join')
    require(parse(data['sources-before.json']) == parse(data['sources-after.json']) == sources
        and value['source_sha256'] == records['sources-before.json']['sha256']
        and value['manifest_sha256'] == manifest_expected[1]
        and value['controller_sha256'] == sources[wrapper]['sha256']
            == '20bea4bf63e1fb0cb606930128cce34e1d25a3c1504345cae1eb160585ee9463'
        and value['test_inventory_before'] == value['test_inventory_after'] == inventory,
        'failed source and full named inventory postchecks')
    error_name = 'test_linked_admission.LinkedAdmissionTests.test_all_eight_continuation_stages_and_six_artifacts_required'
    expected = sorted(name for rows in inventory.values() for name in rows if name != error_name)
    log = data['tests.log'].decode('utf-8')
    passes = []
    for method, name in re.findall(r'^(test_[^\s]+) \(([^()\n]+)\) \.\.\. ok$', log, re.M):
        passes.append(name if name.endswith('.' + method) else name + '.' + method)
    error_line = 'ERROR: ' + error_name.rsplit('.', 1)[1] + ' (' + error_name + ") (where='artifacts')"
    require(sorted(passes) == expected and len(passes) == len(set(passes)) == 173
        and [line for line in log.splitlines() if line.startswith('ERROR:')] == [error_line]
        and "KeyError: 'emitted/source.handoff-v3'" in log
        and re.search(r'\nRan 174 tests in [0-9.]+s\n\nFAILED \(errors=1\)\n?\Z', log),
        'actual V2 missing-handoff error and 173 other named outcomes')
    return dict(receipt=records['failed.json'], package=manifest_pin, tests_run=174,
        passed=False, errors=1, failures=0, skipped=0, error_test=error_name,
        error="KeyError: 'emitted/source.handoff-v3'", transcript=records['tests.log'],
        unchanged_sources=records['sources-before.json'], selected_source_bodies_retained=3,
        full_source_body_hashes_rechecked=True, native_execution=False, qualifies_v3=False)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary data-only Python')
    require(len(sys.argv) == 7 and all(re.fullmatch('[0-9a-f]{64}', sys.argv[n]) for n in (2, 4, 6)),
            'LOCAL_PURE_COMPLETE ACTUAL_SHA LOCAL_PRIMARY29 ACTUAL_SHA LOCAL_V2_FAILED ACTUAL_SHA')
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
    require(manifest['schema'] == 'ferric-p228-projection-ordered-segment-gpu-package-v3'
        and type(manifest['pure_tests']) is int and manifest['pure_tests'] == 174
        and manifest['test_census'] == CENSUS and len(manifest['files']) == 33
        and {r['path'] for r in manifest['files']} == MEMBERS, 'exact frozen thirty-three-member package')
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
    wrapper = body(base / WRAPPER)
    require(digest(wrapper) == WRAPPER_SHA, 'exact tested wrapper body retained in package')
    controller = dict(path=str(E / WRAPPER), bytes=len(wrapper), sha256=WRAPPER_SHA)
    add(base / WRAPPER, E / WRAPPER, 'tools/' + WRAPPER, (len(wrapper), WRAPPER_SHA))
    measured = suite(Path(sys.argv[1]), sys.argv[2], sources, inventory, controller)
    primary = adapters(Path(sys.argv[3]), sys.argv[4])
    prior_failure = failed_v2(Path(sys.argv[5]), sys.argv[6])
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
    result = dict(schema='ferric-p228-projection-ordered-segment-gpu-preparation-publication-v1',
        package=manifest_pin, publisher=publisher, suite=measured, primary_adapter_tests=primary,
        prior_failed_policy_run=prior_failure,
        files={name: dict(original=pin, bytes=len(raw), sha256=digest(raw))
               for name, (raw, pin) in sorted(COPIES.items())},
        source_postchecks_passed=True, data_only_publication=True, tests_rerun=False,
        tested_modules_imported=False, native_execution=False, gpu_execution=False,
        runtime_audit_executed=False, numerical_acceptance=False, full_model_acceptance=False,
        production_authority=False, performance_claim=False,
        root_readme_not_part_of_authenticated_copies=True,
        limitations=['Synthetic CPU policy tests only; no native run or runtime audit is performed by this suite.',
            'CPU compilation, executable runtime admission and native GPU execution are not performed or qualified by this publisher.',
            'The adapter29 observation is a root primary-tool summary, not a supervisor receipt or full named transcript.',
            'The V2 suite remains failed with one artifact-roster KeyError; its outcome is not relabeled.',
            'External devices, models, prior GPU admissions and numerical comparisons are not revalidated here.'])
    output = (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    with (Q / 'result.json').open('xb') as stream:
        stream.write(output)
    expected = set(COPIES) | {'result.json'} | ({'README.md'} if (Q / 'README.md').exists() else set())
    require({str(p.relative_to(Q)) for p in Q.rglob('*') if p.is_file()} == expected,
            'closed published file roster')
    print(json.dumps(dict(path=str(Q / 'result.json'), bytes=len(output), sha256=digest(output),
        published_files=len(COPIES) + 1, supervisor_tests=174, adapter_tests=29, native_execution=False), sort_keys=True))


if __name__ == '__main__':
    main()
