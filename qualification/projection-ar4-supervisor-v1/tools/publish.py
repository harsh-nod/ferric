"""Publish two actual CPU policy observations; never import or run tested code."""
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
Q = Path('/home/harsh/ferric-p227-integration/qualification/projection-ar4-supervisor-v1')
SELF = PL / 'p228-projection-ar4-policy-publication-v1/publish.py'
GPU = 'p228-projection-ar4-decode-gpu-v1'
FRAMEWORK = 'p228-projection-ar4-framework-v1'
PURE = 'projection-ar4-decode-gpu-pure-v228-v1'
MANIFEST = (2386, '7526f2ad568de0759c6f3bae544608f25033c664d779542fb85484a62400d731')
COMPLETE = (12441, 'c3409c69be7d17e922d1b46db6ab5106d7cd4771c953494ad75f9e03b16ee331')
WRAPPER = 'run_projection_ar4_decode_gpu_pure_p228_v1.py'
WRAPPER_SHA = '9b7093c02ed3868dc6af7272f38faec011f852e13e11b9091856e52feac5995f'
CENSUS = {'test_run.py': 11, 'test_decode_validation.py': 17, 'test_intake.py': 16,
          'test_silu_admission.py': 8}
MEMBERS = {'INTAKE.md', 'README.md', 'decode_validation.py', 'intake.py', 'layer_validation.py',
    'run.py', 'run_row_facts_v2.py', 'stage_core.py', 'smoke_validation.py', WRAPPER, *CENSUS}
FRAMEWORK_PINS = {
    'run.py': (30581, 'cba39ec416fe1d1ac5d586a44a9a73099598ff3600c51bd8df47f365983c3d33'),
    'test_run.py': (12897, 'b04e78325e4fc6bfe40e96a6e3b1ca4eca3c488c1c955c31f395f24977857675'),
    'launch.py': (20653, '2a0d60facae6f27855d6caa40a584e3257f5580c299527e0d367b824e0031304'),
    'test_launch.py': (12337, 'c83843cb9924b10490b9068963865ea38d817851f5c2a0784e123b1be904fe12'),
    'README.md': (5476, 'a246371c6c54002fa39b5665c0bafdc70d658a96402621cdeb6d6303d387969f'),
}
RAW = {'complete.json', 'sources-before.json', 'sources-after.json', 'tests.log'}
OBSERVATION_KIND = 'Primary-agent observation of actual SSH tool output; not a remote supervisor receipt'
FRAMEWORK_OBSERVATION_SHA = 'a54dfbfa2bf653a9507cf1a38f3d1ed85ecb7ca111a8ceda486beeed60372f4b'
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


def transcript(raw, inventory):
    expected = sorted(name for rows in inventory.values() for name in rows)
    text = raw.decode('utf-8')
    observed = []
    for match in re.finditer(r'^(test_\w+) \(([^()\n]+)\) \.\.\. ok$', text, re.M):
        method, owner = match.groups()
        observed.append(owner if owner.endswith('.' + method) else owner + '.' + method)
    require(sorted(observed) == expected and len(observed) == len(set(observed))
        and re.search(r'\nRan ' + str(len(expected)) + r' tests in [0-9.]+s\n\nOK\n?\Z', text)
        and not re.search(r'\.\.\. (?:FAIL|ERROR|skipped)|^FAILED', text, re.M), 'exact named passing transcript')


def gpu_suite():
    manifest_pin, raw = add(PL / GPU / 'manifest.json', E / GPU / 'manifest.json',
                            'supervisor/manifest.json', MANIFEST)
    manifest = parse(raw)
    require(manifest['schema'] == 'ferric-p228-projection-ar4-decode-gpu-package-v1'
        and type(manifest['pure_tests']) is int and manifest['pure_tests'] == 52
        and manifest['test_census'] == CENSUS and len(manifest['files']) == 14
        and {row['path'] for row in manifest['files']} == MEMBERS, 'frozen fourteen-member package')
    sources, inventory = {}, {}
    for row in manifest['files']:
        require(set(row) == {'path', 'bytes', 'sha256'}, 'closed frozen member')
        name = row['path']
        sources[name], raw = add(PL / GPU / name, E / GPU / name, 'supervisor/' + name,
                                  (row['bytes'], row['sha256']))
        if name in CENSUS:
            inventory[name] = names(raw, name)
            require(len(inventory[name]) == CENSUS[name], 'declared per-module inventory')
    require(sources[WRAPPER]['sha256'] == WRAPPER_SHA, 'executed wrapper bytes')
    controller = dict(sources[WRAPPER], path=str(E / WRAPPER))
    directory = L / PURE
    require(directory.resolve(strict=True) == directory and {p.name for p in directory.iterdir()} == RAW,
            'closed four-file MI350 pure result')
    pins, data = {}, {}
    for name in sorted(RAW):
        pins[name], data[name] = add(directory / name, E / PURE / name, 'tests/supervisor/' + name,
                                     COMPLETE if name == 'complete.json' else None)
    value = parse(data['complete.json'])
    require(value['schema'] == 'ferric-p228-projection-ar4-decode-gpu-pure-v1'
        and value['passed'] is True and type(value['tests']) is int and value['tests'] == 52
        and value['source_postchecks_passed'] is True and value['synthetic_policy_tests_only'] is True
        and all(type(value[key]) is int and value[key] == 0 for key in ('errors', 'failures', 'skipped'))
        and all(value[key] is False for key in ('native_execution', 'gpu_execution', 'numerical_acceptance',
            'full_model_acceptance', 'production_authority', 'performance_claim')), 'actual pure52 non-native outcome')
    for field, name in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'),
                        ('transcript', 'tests.log')):
        require(value[field] == pins[name], 'actual raw FilePin join')
    require(parse(data['sources-before.json']) == parse(data['sources-after.json']) == sources
        and value['source_sha256'] == pins['sources-before.json']['sha256']
        and value['manifest_sha256'] == manifest_pin['sha256']
        and value['controller_sha256'] == controller['sha256']
        and value['test_inventory_before'] == value['test_inventory_after'] == inventory,
        'unchanged package, wrapper and complete named inventory')
    transcript(data['tests.log'], inventory)
    return dict(host='mi350 (smci350-rck-g03-b19-03)', evidence_kind='bounded CPU-only pure-test receipt',
        receipt=pins['complete.json'], package=manifest_pin, controller=controller,
        passed=52, errors=0, failures=0, skipped=0, elapsed_seconds=value['elapsed_seconds'],
        inventory=inventory, source_postchecks_observed=True, native_execution=False, gpu_execution=False)


def framework_suite(path, expected_sha):
    require(path == L / 'projection-ar4-framework-pure-root-v228-v1.json'
            and expected_sha == FRAMEWORK_OBSERVATION_SHA,
            'root-pinned local primary observation')
    raw = body(path)
    observation_pin, _ = add(path, path, 'tests/framework/primary-observation.json', (len(raw), expected_sha))
    value = parse(raw)
    require(value['schema'] == 'ferric-p228-projection-ar4-framework-root-test-observation-v1'
        and value['observation_kind'] == OBSERVATION_KIND
        and value['host'] == 'asrock-1w300-g2-2b'
        and value['tool_session'] == 64910 and value['terminal_chunk'] == '6a2772'
        and value['exit_code'] == 0 and type(value['tests']) is int and value['tests'] == 38
        and all(type(value[key]) is int and value[key] == 0 for key in ('errors', 'failures', 'skipped'))
        and all(value[key] is False for key in ('gpu_execution', 'numerical_acceptance',
                                               'production_authority', 'performance_claim')),
        'actual primary-agent CPU38 observation, not manufactured supervisor receipt')
    require(type(value['command']) is str and str(E / FRAMEWORK) in value['command']
        and 'unittest' in value['command'] and 'discover' in value['command'], 'actual declared test invocation')
    sources, inventory = {}, {}
    for name, identity in FRAMEWORK_PINS.items():
        sources[name], body_raw = add(PL / FRAMEWORK / name, E / FRAMEWORK / name,
                                      'framework/' + name, identity)
        if name.startswith('test_'):
            inventory[name] = names(body_raw, name)
    require({name: len(rows) for name, rows in inventory.items()} ==
            {'test_run.py': 20, 'test_launch.py': 18}, 'twenty reference and eighteen launcher cases')
    expected = {name: pin['sha256'] for name, pin in sources.items()}
    require(value['source_sha256_before'] == value['source_sha256_after'] == expected,
            'actually observed before/after source identities')
    tools = value['raw_tool_observations']
    require(set(tools) == {'before', 'run', 'after'}
        and all(item['exit_code'] == 0 for item in tools.values())
        and tools['run']['chunk_id'] == value['terminal_chunk']
        and tools['run']['output'].endswith(value['test_output_excerpt']), 'original primary output joins')
    for name in ('before', 'after'):
        hashes = {}
        for line in tools[name]['output'].splitlines():
            match = re.fullmatch(r'([0-9a-f]{64})  (/[^\n]+)', line)
            require(match is not None and match[2] not in hashes, 'exact unique sha256sum output line')
            hashes[match[2]] = match[1]
        require(hashes == {pin['path']: pin['sha256'] for pin in sources.values()},
                'both raw tool source snapshots join copied source bodies')
    transcript(value['test_output_excerpt'].encode('utf-8'), inventory)
    return dict(host=value['host'], evidence_kind=OBSERVATION_KIND, observation=observation_pin,
        passed=38, errors=0, failures=0, skipped=0, inventory=inventory,
        source_postchecks_observed=True, remote_supervisor_receipt=False,
        native_execution=False, gpu_execution=False, framework_model_loaded=False)


def main():
    require(len(sys.argv) == 3 and not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ,
            'publish.py ACTUAL_FRAMEWORK_OBSERVATION_JSON SHA256')
    require(Path(__file__).resolve() == SELF, 'fixed data-only publisher')
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir()
                and {p.name for p in Q.iterdir()} <= {'README.md'}, 'fresh destination, optional root README only')
        if (Q / 'README.md').exists():
            body(Q / 'README.md')
    else:
        require(Q.parent.resolve(strict=True) == Q.parent, 'canonical qualification parent')
    gpu = gpu_suite()
    framework = framework_suite(Path(sys.argv[1]), sys.argv[2])
    publisher, _ = add(SELF, SELF, 'tools/publish.py')
    for path in list(READS):
        body(path)
    Q.mkdir(exist_ok=True)
    for target, (raw, _) in COPIES.items():
        path = Q / target
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
        require(path.read_bytes() == raw, 'exact published copy')
    for path in list(READS):
        body(path)
    result = dict(schema='ferric-p228-projection-ar4-policy-publication-v1',
        publisher=publisher, suites=dict(native_supervisor_policy=gpu, independent_framework_policy=framework),
        files={name: dict(original=pin, bytes=len(raw), sha256=digest(raw))
               for name, (raw, pin) in sorted(COPIES.items())},
        data_only_publication=True, local_input_postchecks_passed=True,
        tests_rerun=False, tested_modules_imported=False, native_execution=False,
        gpu_execution=False, framework_model_loaded=False, runtime_audit_executed=False,
        numerical_acceptance=False, full_model_acceptance=False, production_authority=False,
        performance_claim=False, sustained_2048_256=False,
        root_readme_not_part_of_authenticated_copies=True,
        limitations=[
            'Both suites are synthetic CPU policy tests, not hardware or framework numerical observations.',
            'Framework38 is a primary-agent SSH output observation, not a remote supervisor receipt.',
            'The publisher does not replay CPU1037 artifacts, runtime audits, model/source dependencies or native admission.',
            'No AR4 token, tensor, throughput or full-model acceptance result is produced by this checkpoint.'])
    raw = (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    with (Q / 'result.json').open('xb') as stream:
        stream.write(raw)
    expected = set(COPIES) | {'result.json'} | ({'README.md'} if (Q / 'README.md').exists() else set())
    require({str(p.relative_to(Q)) for p in Q.rglob('*') if p.is_file()} == expected, 'closed publication roster')
    print(json.dumps(dict(path=str(Q / 'result.json'), bytes=len(raw), sha256=digest(raw),
        copied_files=len(COPIES), supervisor_policy_tests=52, framework_policy_tests=38,
        native_execution=False), sort_keys=True))


if __name__ == '__main__':
    main()
