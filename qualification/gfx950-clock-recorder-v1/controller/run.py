"""Fresh clock recorder worker qualification; no native or GPU execution."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
PACKAGE = E / 'p228-gfx950-clock-recorder-cpu-v1'
PRIOR = E / 'gfx950-clock-cpu-v228-v1'
PRIOR_SHA = '458732e9e0c67501f33584ff1c6c041f93c0b9021884b504c4a7dfb206610ad3'
CLOCK = E / 'p228-gfx950-clock-cpu-v1/run.py'
CLOCK_SHA = '6a20d062f15396c24e24e9aa9aaa72b27b5cfc5e50d8433cbf2ac002c89f7ee6'
PREVIOUS = E / 'p228-prefix-raw-timestamps-cpu-v1/run.py'
PREVIOUS_SHA = '91dd8ffc848259644aa8903f9cb83f8eda924cbf2a63c603adcf093a6b24094f'
HELPER = E / 'p228-host-policy-cpu-v1/run.py'
HELPER_SHA = '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494'
SOURCE_MANIFEST = E / 'clock-recorder-source-inputs-v228-v1.json'
SOURCE_DIR = 'p228-gfx950-clock-recorder-source-v2'
WORKER_ROOT = 'adapters/tp-peer-finite-engineering-worker-v1/src/'
COMMITS = {'ferric': '924703865d9cfcb1dc791af211b7558ff7c58894',
           'fe2o3': '27b53d2b74c1f239988a891a4aed39e089b05663'}
TREES = {'ferric': '66b302c5ee7a71993685df884229514d2196b785',
         'fe2o3': '3c2fa509b7328c4bf7d8e2ef2aab3a0e0c4a422e'}
ARCHIVES = {'ferric': 'clock-recorder-ferric-92470386-v228-v1.tar.gz',
            'fe2o3': 'clock-recorder-fe2o3-27b53d2b-v228-v1.tar.gz'}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def optimization_guard(optimized, environment):
    require(not optimized and 'PYTHONOPTIMIZE' not in environment,
            'optimized Python refused before authenticated helpers load')


def content_pin(value):
    require(isinstance(value, dict) and set(value) == {'bytes', 'sha256'}, 'content pin fields')
    require(type(value['bytes']) is int and value['bytes'] >= 0, 'content pin bytes')
    require(isinstance(value['sha256'], str) and re.fullmatch('[0-9a-f]{64}', value['sha256']), 'content pin digest')
    return value


def file_pin(value):
    require(isinstance(value, dict) and set(value) == {'path', 'bytes', 'sha256'}, 'file pin fields')
    require(isinstance(value['path'], str) and Path(value['path']).is_absolute()
            and '..' not in Path(value['path']).parts and str(Path(value['path'])) == value['path'], 'absolute canonical path')
    content_pin({key: value[key] for key in ('bytes', 'sha256')})
    return value


def package_inputs(sha):
    require(isinstance(sha, str) and re.fullmatch('[0-9a-f]{64}', sha), 'actual package SHA')
    path = PACKAGE / 'manifest.json'
    require(path.resolve(strict=True) == path and not path.is_symlink(), 'canonical package manifest')
    require(hashlib.sha256(path.read_bytes()).hexdigest() == sha, 'frozen package manifest')
    value = json.loads(path.read_bytes())
    require(value['schema'] == 'ferric-p228-gfx950-clock-recorder-cpu-package-v1'
            and len(value['files']) == 4 and {row['path'] for row in value['files']}
            == {'run.py', 'overlay.json', 'test_run.py', 'README.md'}, 'closed four-file package')
    require(Path(__file__).resolve(strict=True) == PACKAGE / 'run.py', 'executed controller identity')
    for row in value['files']:
        require(set(row) == {'path', 'bytes', 'sha256'}, 'package row fields')
        content_pin({key: row[key] for key in ('bytes', 'sha256')})
        member = PACKAGE / row['path']
        require(member.resolve(strict=True) == member and not member.is_symlink(), 'canonical package member')
        raw = member.read_bytes()
        require(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256'], 'package member bytes')
    return [path, *(PACKAGE / row['path'] for row in value['files'])]


def overlay_shape(value):
    require(isinstance(value, dict) and set(value) == {'schema', 'source_manifest', 'files', 'added_worker_tests'},
            'overlay fields')
    require(value['schema'] == 'ferric-p228-gfx950-clock-recorder-cpu-overlay-v1', 'overlay schema')
    require(file_pin(value['source_manifest'])['path'] == str(SOURCE_MANIFEST), 'actual fresh source manifest')
    require(isinstance(value['files'], list) and 1 <= len(value['files']) <= 32, 'bounded nonempty source delta')
    seen = set()
    for row in value['files']:
        require(set(row) == {'path', 'source', 'before', 'after'}, 'overlay row fields')
        require(isinstance(row['path'], str), 'string source destination')
        path = Path(row['path'])
        require(not path.is_absolute() and '..' not in path.parts
                and path.as_posix() == row['path'] and row['path'].startswith(WORKER_ROOT)
                and path.suffix == '.rs' and row['path'] not in seen, 'worker Rust-only unique destination')
        seen.add(row['path'])
        require(row['source'] == SOURCE_DIR + '/draft/' + row['path'], 'frozen source body path')
        if row['before'] is not None:
            content_pin(row['before'])
        content_pin(row['after'])
    names = value['added_worker_tests']
    require(isinstance(names, list) and names and all(isinstance(name, str)
            and re.fullmatch('[A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)+', name) for name in names), 'actual named worker additions')
    require(names == sorted(set(names)), 'sorted unique frozen worker additions')
    require(any(name.startswith('prefix_decode_device_clock_observation_v2::') for name in names)
            and any('clock' in name and name.startswith('native_prefix_') for name in names),
            'clock schema and native recorder coverage required')
    return value


def source_shape(value):
    require(set(value) == {'schema', 'archives'} and value['schema'] == 'ferric-p228-clean-worker-sources-v1'
            and set(value['archives']) == {'ferric', 'fe2o3'}, 'paired clean source schema')
    for project, row in value['archives'].items():
        require(set(row) == {'path', 'bytes', 'sha256', 'commit', 'tree'}, 'source archive fields')
        file_pin({key: row[key] for key in ('path', 'bytes', 'sha256')})
        require(row['path'] == str(E / ARCHIVES[project])
                and row['commit'] == COMMITS[project] and row['tree'] == TREES[project], 'exact pushed source generation')
    return value


def worker_extension(actual, prior, additions):
    require(not prior.intersection(additions), 'new tests already existed')
    require(actual == prior | additions, 'exact worker regression inventory plus declared additions')
    return [(427 + len(additions), 0, 4), (13, 0, 0)]


def authenticate_prior(h, x, n, previous, base, state, clock, pinned):
    complete = PRIOR / 'complete.json'
    require(h.sha(complete) == PRIOR_SHA and complete.stat().st_size == 204451, 'actual CPU648 receipt')
    pinned.append(x.pin(complete))
    value = json.loads(complete.read_bytes())
    require(value['schema'] == 'ferric-p228-gfx950-clock-cpu-result-v1' and value['passed'] is True
            and value['error'] is None and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['tests_passed'] == 648 and value['tests_ignored'] == 4, 'qualified regression baseline')
    require(all(value[key] is False for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim',
        'timestamp_calibration', 'parent_rebuilt', 'production_authority')), 'historical CPU-only scope')
    filters = (*previous.filters_for(base, state), *clock.EXTRA_FILTERS)
    n.F, n.T = PRIOR / 'sources/fe2o3', PRIOR / 'target'
    commands = state.recipes(str(n.N / 'bin/cargo'),
        PRIOR / 'sources/ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml', filters)
    require(len(commands) == 33 and set(value['phases']) == {name for name, _, _ in commands}, 'historical phase roster')
    expected_raw = {'sources-base.json', 'sources-before.json', 'sources-after.json'} | {
        name + suffix for name, _, _ in commands for suffix in (
            '-command.json', '-started.json', '-stdout', '-stderr', '-result.json')}
    require(set(value['raw']) == expected_raw and len(expected_raw) == 168, 'historical raw roster')
    for name, pin in value['raw'].items():
        require(file_pin(pin)['path'] == str(PRIOR / name) and x.pin(Path(pin['path'])) == pin, 'historical raw bytes')
        pinned.append(pin)
    for name, argv, deadline in commands:
        command = json.loads((PRIOR / (name + '-command.json')).read_bytes())
        require(command == dict(argv=argv, env=state.environment(n, PRIOR), tools=n.PINS,
            deadline_seconds=deadline, cache_cap_bytes=6 << 30, affinity=[8, 9], nice=10,
            gpu_execution=False, expected_exit=0), 'historical exact command/environment')
        started = json.loads((PRIOR / (name + '-started.json')).read_bytes())
        require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int
                and started['pid'] > 0 and started['pid'] == started['pgid'], 'historical process group')
        result = json.loads((PRIOR / (name + '-result.json')).read_bytes())
        require(result == value['phases'][name] and result['exit_code'] == 0 and result['reason'] is None
                and result['group_absent'] is True and result['cache_bytes'] <= 6 << 30, 'historical natural owned exit')
        require(result['stdout_sha256'] == value['raw'][name + '-stdout']['sha256']
                and result['stderr_sha256'] == value['raw'][name + '-stderr']['sha256'], 'historical streams')
    runtime = h.inventory((PRIOR / 'runtime-list-stdout').read_text())
    require(len(runtime) == 900, 'historical complete runtime inventory')
    selected = set()
    require(set(value['tests']) == {name for name, _, _ in filters} | {'worker'}, 'historical test selections')
    for name, selector, count in filters:
        names = {test for test in runtime if selector in test}
        require(len(names) == count and not selected.intersection(names), 'historical disjoint runtime selections')
        selected.update(names)
        found = h.results((PRIOR / (name + '-stdout')).read_text(), names, [(count, 0, 0)])
        require(json.loads(json.dumps(found)) == value['tests'][name], 'historical actual runtime outcomes')
    require(len(selected) == 208 and previous.NEW_TESTS <= selected
            and set(value['added_runtime_tests']) <= selected, 'all existing timestamp/runtime/clock cases')
    worker = h.inventory((PRIOR / 'worker-list-stdout').read_text())
    require(len(worker) == 444, 'historical complete worker inventory')
    raw = (PRIOR / 'worker-tests-stdout').read_text()
    found = h.results(raw, worker, [(427, 0, 4), (13, 0, 0)])
    require(json.loads(json.dumps(found)) == value['tests']['worker'], 'historical worker outcomes')
    return runtime, worker, state.ignored_names(raw), filters


def compatible_source_base(actual, prior):
    # Only these two READMEs changed outside the qualified runtime/worker code.
    ignored = {'fe2o3/crates/fe2o3-kfd/README.md',
               'ferric/adapters/tp-peer-finite-engineering-worker-v1/README.md'}
    def compiled_inputs(source):
        return {path: pin for path, pin in source.items() if path not in ignored
                and (path.startswith('fe2o3/')
                     or path.startswith('ferric/adapters/tp-peer-finite-engineering-worker-v1/'))}
    expected = compiled_inputs(prior)
    require(expected and compiled_inputs(actual) == expected,
            'fresh runtime/worker inputs differ from qualified CPU648')


def install_overlay(source, expected, rows, read_source, x):
    for row in rows:
        key = 'ferric/' + row['path']
        require(expected.get(key) == row['before'], 'worker source preimage: ' + key)
        target = source / key
        if row['before'] is None:
            require(not target.exists() and not target.is_symlink(), 'new source already exists')
        else:
            actual = x.pin(target)
            require({key: actual[key] for key in ('bytes', 'sha256')} == row['before'], 'actual existing source preimage')
    for row in rows:
        raw = read_source(row)
        require(len(raw) == row['after']['bytes'] and hashlib.sha256(raw).hexdigest() == row['after']['sha256'],
                'frozen replacement body')
        target = source / 'ferric' / row['path']
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb' if row['before'] is None else 'wb') as stream:
            stream.write(raw)
        expected['ferric/' + row['path']] = row['after']
    require(x.snapshot(source) == expected, 'unlisted source change')


def main():
    optimization_guard(sys.flags.optimize, os.environ)
    require(len(sys.argv) == 3, 'MANIFEST_SHA FRESH_LABEL')
    manifest_sha, label = sys.argv[1:]
    require(re.fullmatch(r'gfx950-clock-recorder-cpu-v228-v[1-9][0-9]*', label), 'fresh CPU label')
    paths = package_inputs(manifest_sha)
    require(HELPER.resolve(strict=True) == HELPER and hashlib.sha256(HELPER.read_bytes()).hexdigest() == HELPER_SHA,
            'pinned metadata/inventory helper')
    h = types.ModuleType('tested_policy_helpers')
    h.__file__ = str(HELPER)
    exec(compile(HELPER.read_bytes(), str(HELPER), 'exec'), h.__dict__)
    x = h.load(h.EXTRACTOR, h.EXTRACTOR_SHA, 'archive_helpers')
    n = h.load(h.BOUNDS, h.BOUNDS_SHA, 'owned_cpu_leaves')
    previous = h.load(PREVIOUS, PREVIOUS_SHA, 'qualified_timestamp_cpu')
    base = h.load(previous.BASE, previous.BASE_SHA, 'qualified_state_cpu')
    state = h.load(previous.STATE, previous.STATE_SHA, 'qualified_bank_cpu')
    clock = h.load(CLOCK, CLOCK_SHA, 'qualified_clock_cpu')
    pinned = [x.pin(path) for path in [*paths, HELPER, PREVIOUS, previous.BASE, previous.STATE, CLOCK, h.EXTRACTOR, h.BOUNDS]]
    prior_runtime, prior_worker, prior_ignored, filters = authenticate_prior(h, x, n, previous, base, state, clock, pinned)
    overlay = overlay_shape(json.loads((PACKAGE / 'overlay.json').read_bytes()))
    require(x.pin(SOURCE_MANIFEST) == overlay['source_manifest'], 'actual frozen source manifest')
    pinned.append(overlay['source_manifest'])
    sources = source_shape(json.loads(SOURCE_MANIFEST.read_bytes()))
    for row in overlay['files']:
        path = E / row['source']
        require(path.resolve(strict=True) == path, 'canonical overlay source')
        pin = x.pin(path)
        require({key: pin[key] for key in ('bytes', 'sha256')} == row['after'], 'actual overlay body')
        pinned.append(pin)
    out = E / label
    out.mkdir(mode=0o700)
    source = out / 'sources'
    source.mkdir()
    n.F, n.T = source / 'fe2o3', out / 'target'
    n.setup()
    require(not any(n.T.iterdir()), 'fresh empty target')
    (out / 'tmp').mkdir()
    extraction = {}
    for project, row in sources['archives'].items():
        path = Path(row['path'])
        require(path.resolve(strict=True) == path, 'canonical archive')
        pin = x.pin(path)
        require(pin == {key: row[key] for key in ('path', 'bytes', 'sha256')}, 'actual clean archive bytes')
        pinned.append(pin)
        extraction[project] = x.extract(path, project, source)
    expected = x.snapshot(source)
    compatible_source_base(expected, json.loads((PRIOR / 'sources-after.json').read_bytes()))
    n.save(out / 'sources-base.json', expected)
    base_count = len(expected)
    install_overlay(source, expected, overlay['files'], lambda row: (E / row['source']).read_bytes(), x)
    require(len(expected) == base_count + sum(row['before'] is None for row in overlay['files']), 'exact source extension')
    n.save(out / 'sources-before.json', expected)
    worker = source / 'ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml'
    commands = state.recipes(str(n.N / 'bin/cargo'), worker, filters)
    require(len(commands) == len({name for name, _, _ in commands}) == 33, 'unchanged 33-phase pipeline')
    recipes = {name: (argv, deadline) for name, argv, deadline in commands}
    env = state.environment(n, out)
    phases, tests, metadata, binaries = {}, {}, None, {}
    error, post_errors = None, []
    additions = set(overlay['added_worker_tests'])

    def run(name):
        argv, deadline = recipes[name]
        phases[name] = n.run(out, name, argv, env=env, deadline=deadline)
        return (out / (name + '-stdout')).read_text()

    try:
        value = json.loads(run('metadata'))
        metadata = h.metadata_check(value, source, n.T, True, h.tomllib.loads(worker.with_name('Cargo.lock').read_text()))
        runtime = next(package for package in value['packages'] if package['name'] == 'fe2o3-kfd')
        features = next(row['features'] for row in value['resolve']['nodes'] if row['id'] == runtime['id'])
        require('engineering-gfx950' in features and 'live-validation' not in features, 'CPU runtime features')
        names = h.inventory(run('runtime-list'))
        require(names == prior_runtime, 'complete runtime inventory unchanged')
        selected = set()
        for name, selector, count in filters:
            subset = {test for test in names if selector in test}
            require(len(subset) == count and not selected.intersection(subset), 'actual runtime selector census')
            selected.update(subset)
            tests[name] = h.results(run(name), subset, [(count, 0, 0)])
        require(len(selected) == 208, 'all existing runtime tests')
        names = h.inventory(run('worker-list'))
        summaries = worker_extension(names, prior_worker, additions)
        raw = run('worker-tests')
        tests['worker'] = h.results(raw, names, summaries)
        require(state.ignored_names(raw) == prior_ignored and not additions.intersection(prior_ignored),
                'new tests pass and historical ignores stay unchanged')
        binaries = h.artifacts(run('worker-build'), [h.CHILD_BIN], worker, n.T, x.pin)
        require(sum(row['passed'] for row in tests.values()) == 648 + len(additions)
                and sum(row['ignored'] for row in tests.values()) == 4, 'actual inventory-derived totals')
        require(set(phases) == set(recipes), 'every bounded phase completed')
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        try:
            after = x.snapshot(source)
            n.save(out / 'sources-after.json', after)
            require(after == expected, 'source or lockfile changed')
        except BaseException as failure:
            post_errors.append('sources: ' + repr(failure))
        try:
            for pin in pinned:
                require(x.pin(Path(pin['path'])) == pin, 'immutable input changed')
            for tool, sha in n.PINS.items():
                require(h.sha(n.N / 'bin' / tool) == sha, 'tool changed')
        except BaseException as failure:
            post_errors.append('inputs: ' + repr(failure))
        try:
            if metadata is not None:
                for row in metadata['external']:
                    require(h.sha(Path(row['manifest'])) == row['manifest_sha256'], 'dependency manifest changed')
            for row in binaries.values():
                require(x.pin(Path(row['binary']['path'])) == row['binary'], 'built binary changed')
        except BaseException as failure:
            post_errors.append('dependencies or binary: ' + repr(failure))
        try:
            require(n.size(n.T) <= 6 << 30, 'final target cap')
        except BaseException as failure:
            post_errors.append('target: ' + repr(failure))
    passed = error is None and not post_errors
    value = dict(schema='ferric-p228-gfx950-clock-recorder-cpu-result-v1', passed=passed,
        error=error, postcheck_errors=post_errors, inputs=pinned,
        package_manifest=x.pin(PACKAGE / 'manifest.json'), overlay=x.pin(PACKAGE / 'overlay.json'),
        source_manifest=x.pin(SOURCE_MANIFEST), source_commits=COMMITS, source_trees=TREES,
        extraction=extraction, prior_cpu_complete=x.pin(PRIOR / 'complete.json'),
        source_unchanged=not any(row.startswith('sources:') for row in post_errors),
        added_worker_tests=sorted(additions), metadata=metadata, phases=phases, tests=tests, binaries=binaries,
        tests_passed=sum(row['passed'] for row in tests.values()), tests_ignored=sum(row['ignored'] for row in tests.values()),
        empty_initial_target=True, external_cargo_cache_reused=True,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False,
        timestamp_calibration=False, parent_rebuilt=False, production_authority=False,
        raw={path.name: x.pin(path) for path in out.iterdir() if path.is_file()})
    result = out / ('complete.json' if passed else 'failed.json')
    n.save(result, value)
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors, receipt=x.pin(result))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
