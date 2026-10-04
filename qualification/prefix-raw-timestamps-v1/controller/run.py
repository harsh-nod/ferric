"""Fresh clean-source timestamp CPU regression; no GPU or live-tree writes."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
PACKAGE = E / 'p228-prefix-raw-timestamps-cpu-v1'
PRIOR = E / 'state-bank-batch-cpu-v228-v1'
PRIOR_SHA = '14dab6e776cdc2184457b2865cf7d48b91a71008d49bea040deca68bba75838f'
STATE = E / 'p228-state-bank-batch-cpu-v1/run.py'
STATE_SHA = 'd5c2237645915cd965aeb46fadf70130486abfc894fffafc9fc3513f277925da'
BASE = E / 'p228-resident-state-fence-cpu-v2/run.py'
BASE_SHA = 'c16fce5d5e431f893bfccc5f1124ba20cbfca2544e26501e221c3ba452fdbedc'
HELPER = E / 'p228-host-policy-cpu-v1/run.py'
HELPER_SHA = '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494'
SOURCE_MANIFEST = E / 'prefix-raw-timestamps-source-inputs-v228-v1.json'
COMMITS = {
    'ferric': '44e308d72815405f9127473368671e9696c5ebaa',
    'fe2o3': '3d217aabc3c48e9c767fa28a05bd596987c29f9d',
}
TREES = {
    'ferric': 'fb509bfe4adb1afb98a8abb788f2ac74a7b3dc29',
    'fe2o3': 'e549d1ef8c8c2abec85d52838eebfe0f5a2ff46d',
}
ARCHIVES = {
    'ferric': 'prefix-raw-timestamps-ferric-44e308d7-v228-v1.tar.gz',
    'fe2o3': 'prefix-raw-timestamps-fe2o3-3d217aab-v228-v1.tar.gz',
}
RUNTIME_DIR = 'p228-prefix-raw-timestamps-runtime-v2'
RUNTIME_ROOT = 'crates/fe2o3-kfd/src/'
RUNTIME_FILES = (
    'engineering_gfx950_peer_wave_qkv_attention_output_tiles_v6.rs',
    'engineering_gfx950_peer_wave_qkv_attention_output_tiles_v6_tests.rs',
    'engineering_gfx950_peer_wave_qkv_attention_output_tiles_timestamp_tests.rs',
)
PREFIX_RESIDENT = 'engineering_gfx950::peer::wave_qkv_attention_output_tiles_v6::tests::'
PREFIX_TIMESTAMPS = 'engineering_gfx950::peer::wave_qkv_attention_output_tiles_v6::timestamp_tests::'
NEW_TESTS = {
    PREFIX_RESIDENT + 'resident_v6_raw_entry_invalid_deadline_poison_blocks_both_modes_and_rearm',
    PREFIX_RESIDENT + 'resident_v6_raw_entry_routes_to_native_validation_before_activation',
    *(PREFIX_TIMESTAMPS + name for name in (
        'timestamp_prefix_join_preserves_real_state_host_and_raw_observations',
        'timestamp_prefix_join_rejects_missing_or_extra_observations',
        'timestamp_prefix_join_rejects_wrong_or_duplicate_rank_order',
        'timestamp_prefix_join_rejects_host_timer_substitution',
        'timestamp_prefix_join_rejects_foreign_group_or_duplicate_device',
        'timestamp_prefix_join_checks_every_terminal_word_on_both_ranks',
        'timestamp_prefix_join_failure_enters_existing_group_poison_path',
    )),
}
EXTRA_FILTERS = (
    ('prefix-timestamps', PREFIX_TIMESTAMPS, 7),
    ('raw-timestamps', 'engineering_gfx950::raw_timestamps::tests::', 8),
    ('memory-raw-timestamps', 'memory_linux::raw_timestamps_tests::', 6),
    ('queue-raw-timestamps', 'queue::submit::tests::raw_timestamp_control_', 2),
)


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def optimization_guard(optimized, environment):
    require(not optimized and 'PYTHONOPTIMIZE' not in environment,
            'optimized Python is forbidden before authenticated helpers load')


def content_pin(value):
    require(isinstance(value, dict) and set(value) == {'bytes', 'sha256'}, 'content pin fields')
    require(type(value['bytes']) is int and value['bytes'] >= 0, 'content pin bytes')
    require(isinstance(value['sha256'], str) and re.fullmatch('[0-9a-f]{64}', value['sha256']),
            'content pin digest')
    return value


def shape_pin(value):
    require(isinstance(value, dict) and set(value) == {'path', 'bytes', 'sha256'}, 'file pin fields')
    require(isinstance(value['path'], str) and Path(value['path']).is_absolute(), 'absolute pin path')
    content_pin({key: value[key] for key in ('bytes', 'sha256')})
    return value


def package_inputs(expected_sha):
    require(isinstance(expected_sha, str) and re.fullmatch('[0-9a-f]{64}', expected_sha), 'manifest digest')
    manifest = PACKAGE / 'manifest.json'
    require(manifest.resolve(strict=True) == manifest and not manifest.is_symlink(), 'canonical manifest')
    require(hashlib.sha256(manifest.read_bytes()).hexdigest() == expected_sha, 'frozen manifest')
    value = json.loads(manifest.read_bytes())
    require(value['schema'] == 'ferric-p228-prefix-raw-timestamps-cpu-package-v1', 'package schema')
    names = {'run.py', 'overlay.json', 'test_run.py', 'README.md'}
    require(len(value['files']) == 4 and {row['path'] for row in value['files']} == names, 'package census')
    require(Path(__file__).resolve(strict=True) == PACKAGE / 'run.py', 'executed controller')
    for row in value['files']:
        require(set(row) == {'path', 'bytes', 'sha256'}, 'package file fields')
        content_pin({key: row[key] for key in ('bytes', 'sha256')})
        path = PACKAGE / row['path']
        require(path.resolve(strict=True) == path and not path.is_symlink(), 'canonical package source')
        raw = path.read_bytes()
        require(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256'],
                'frozen package source')
    return [manifest, *(PACKAGE / row['path'] for row in value['files'])]


def overlay_shape(value):
    require(isinstance(value, dict) and set(value) == {
        'schema', 'source_manifest', 'runtime_preimages', 'files', 'added_runtime_tests'}, 'overlay fields')
    require(value['schema'] == 'ferric-p228-prefix-raw-timestamps-cpu-overlay-v1', 'overlay schema')
    require(shape_pin(value['source_manifest'])['path'] == str(SOURCE_MANIFEST), 'fresh source manifest')
    require(shape_pin(value['runtime_preimages'])['path'] == str(E / RUNTIME_DIR / 'preimages.json'),
            'runtime preimage record')
    require(isinstance(value['files'], list) and len(value['files']) == 3, 'exact three runtime files')
    seen = set()
    for row in value['files']:
        require(set(row) == {'path', 'source', 'before', 'after'}, 'overlay row fields')
        require(row['path'] in {RUNTIME_ROOT + name for name in RUNTIME_FILES}
                and row['path'] not in seen, 'closed runtime destination')
        seen.add(row['path'])
        require(row['source'] == RUNTIME_DIR + '/source/' + Path(row['path']).name,
                'runtime source path')
        if row['path'] == RUNTIME_ROOT + RUNTIME_FILES[-1]:
            require(row['before'] is None, 'timestamp module must be new')
        else:
            content_pin(row['before'])
        content_pin(row['after'])
    added = value['added_runtime_tests']
    require(isinstance(added, list) and all(isinstance(name, str) for name in added)
            and len(added) == len(set(added)) == 9, 'nine distinct authored test names')
    require(all(re.fullmatch('[A-Za-z0-9_:]+', name) for name in added), 'test name syntax')
    require(sum(name.startswith(PREFIX_RESIDENT) for name in added) == 2
            and sum(name.startswith(PREFIX_TIMESTAMPS) for name in added) == 7, 'new typed test routing')
    require(set(added) == NEW_TESTS, 'exact authored timestamp and public-entry test names')
    return value


def source_shape(value):
    require(set(value) == {'schema', 'archives'}
            and value['schema'] == 'ferric-p228-clean-worker-sources-v1', 'clean archive schema')
    require(set(value['archives']) == {'ferric', 'fe2o3'}, 'paired source archives')
    for project, row in value['archives'].items():
        require(set(row) == {'path', 'bytes', 'sha256', 'commit', 'tree'}, 'source archive fields')
        shape_pin({key: row[key] for key in ('path', 'bytes', 'sha256')})
        require(row['path'] == str(E / ARCHIVES[project]), 'fresh archive path')
        require(row['commit'] == COMMITS[project] and row['tree'] == TREES[project],
                'actual clean source generation')
    return value


def exact_extension(actual, prior, added):
    require(not prior.intersection(added), 'added tests already existed in baseline')
    require(actual == prior | added, 'compiled inventory must preserve every old name and exact additions')


def filters_for(base, state):
    return tuple((name, selector, count + (2 if selector == PREFIX_RESIDENT else 0))
                 for name, selector, count in base.FILTERS) + (
        ('state-bank', state.BANK_SELECTOR, 10), *state.MEMORY_FILTERS, *EXTRA_FILTERS)


def historical_inventory(h, x, n, base, state, pinned):
    complete = PRIOR / 'complete.json'
    require(h.sha(complete) == PRIOR_SHA and complete.stat().st_size == 162810, 'actual CPU553 completion')
    pinned.append(x.pin(complete))
    prior = json.loads(complete.read_bytes())
    require(prior['schema'] == 'ferric-p228-state-bank-batch-cpu-result-v1'
            and prior['passed'] is True and prior['error'] is None
            and prior['postcheck_errors'] == [] and prior['source_unchanged'] is True, 'CPU553 success')
    require(prior['tests_passed'] == 553 and prior['tests_ignored'] == 4, 'actual historical census')
    require(all(prior[key] is False for key in (
        'gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority')), 'CPU-only baseline')
    require(len(prior['raw']) == 109, 'historical raw census')
    for name, pin in prior['raw'].items():
        require(shape_pin(pin)['path'] == str(PRIOR / name) and x.pin(Path(pin['path'])) == pin,
                'historical raw bytes')
        pinned.append(pin)
    old_filters = (*base.FILTERS, ('state-bank', state.BANK_SELECTOR, 10), *state.MEMORY_FILTERS)
    n.F, n.T = PRIOR / 'sources/fe2o3', PRIOR / 'target'
    commands = state.recipes(str(n.N / 'bin/cargo'),
        PRIOR / 'sources/ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml', old_filters)
    require(len(commands) == 21 and set(prior['phases']) == {name for name, _, _ in commands},
            'historical phase roster')
    require(set(prior['raw']) == {'sources-base.json', 'sources-prior.json',
        'sources-before.json', 'sources-after.json'} | {name + suffix for name, _, _ in commands
        for suffix in ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')}, 'historical raw roster')
    for name, argv, deadline in commands:
        command = json.loads((PRIOR / (name + '-command.json')).read_bytes())
        require(command == dict(argv=argv, env=state.environment(n, PRIOR), tools=n.PINS,
            deadline_seconds=deadline, cache_cap_bytes=6 << 30, affinity=[8, 9], nice=10,
            gpu_execution=False, expected_exit=0), 'historical command/environment')
        started = json.loads((PRIOR / (name + '-started.json')).read_bytes())
        require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int
                and started['pid'] > 0 and started['pid'] == started['pgid'], 'historical process group')
        result = json.loads((PRIOR / (name + '-result.json')).read_bytes())
        require(result == prior['phases'][name] and result['exit_code'] == 0
                and result['reason'] is None and result['group_absent'] is True
                and result['cache_bytes'] <= 6 << 30, 'historical natural/reaped leaf')
        require(result['stdout_sha256'] == prior['raw'][name + '-stdout']['sha256']
                and result['stderr_sha256'] == prior['raw'][name + '-stderr']['sha256'], 'historical streams')
    runtime = h.inventory((PRIOR / 'runtime-list-stdout').read_text())
    selected = set()
    require(set(prior['tests']) == {name for name, _, _ in old_filters} | {'worker'}, 'historical test groups')
    for name, selector, count in old_filters:
        subset = {test for test in runtime if selector in test}
        require(len(subset) == count and not subset.intersection(selected), 'historical named selection')
        selected.update(subset)
        actual = h.results((PRIOR / (name + '-stdout')).read_text(), subset, [(count, 0, 0)])
        require(json.loads(json.dumps(actual)) == prior['tests'][name], 'historical runtime outcomes')
    require(len(selected) == 144 and state.BANK_TESTS <= selected, 'historical bank/currentness coverage')
    worker = h.inventory((PRIOR / 'worker-list-stdout').read_text())
    require(len(worker) == 413 and state.WORKER_TESTS <= worker, 'historical worker inventory')
    raw = (PRIOR / 'worker-tests-stdout').read_text()
    actual = h.results(raw, worker, [(396, 0, 4), (13, 0, 0)])
    require(json.loads(json.dumps(actual)) == prior['tests']['worker'], 'historical worker outcomes')
    return runtime, worker, state.ignored_names(raw)


def install_overlay(source, expected, rows, read_source, x):
    for row in rows:
        key = 'fe2o3/' + row['path']
        require(expected.get(key) == row['before'], 'runtime overlay preimage: ' + key)
        target = source / key
        if row['before'] is None:
            require(not target.exists() and not target.is_symlink(), 'new module already exists')
        else:
            actual = x.pin(target)
            require({key: actual[key] for key in ('bytes', 'sha256')} == row['before'], 'actual runtime preimage')
    # Validate every existing body before performing any overlay write.
    for row in rows:
        raw = read_source(row)
        require(len(raw) == row['after']['bytes']
                and hashlib.sha256(raw).hexdigest() == row['after']['sha256'], 'overlay replacement bytes')
        target = source / 'fe2o3' / row['path']
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb' if row['before'] is None else 'wb') as stream:
            stream.write(raw)
        expected['fe2o3/' + row['path']] = row['after']
    require(x.snapshot(source) == expected, 'unlisted overlay mutation')


def main():
    optimization_guard(sys.flags.optimize, os.environ)
    require(len(sys.argv) == 3, 'MANIFEST_SHA FRESH_LABEL')
    manifest_sha, label = sys.argv[1:]
    require(re.fullmatch(r'prefix-raw-timestamps-cpu-v228-v[1-9][0-9]*', label), 'fresh output label')
    package_paths = package_inputs(manifest_sha)
    require(HELPER.resolve(strict=True) == HELPER
            and hashlib.sha256(HELPER.read_bytes()).hexdigest() == HELPER_SHA, 'tested helper')
    h = types.ModuleType('tested_policy_helpers')
    h.__file__ = str(HELPER)
    exec(compile(HELPER.read_bytes(), str(HELPER), 'exec'), h.__dict__)
    x = h.load(h.EXTRACTOR, h.EXTRACTOR_SHA, 'archive_helpers')
    n = h.load(h.BOUNDS, h.BOUNDS_SHA, 'owned_cpu_leaves')
    base = h.load(BASE, BASE_SHA, 'prior_state_cpu')
    state = h.load(STATE, STATE_SHA, 'tested_bank_cpu')
    pinned = [x.pin(path) for path in [*package_paths, HELPER, BASE, STATE, h.EXTRACTOR, h.BOUNDS]]
    prior_runtime, prior_worker, prior_ignored = historical_inventory(h, x, n, base, state, pinned)
    overlay = overlay_shape(json.loads((PACKAGE / 'overlay.json').read_bytes()))
    for key in ('source_manifest', 'runtime_preimages'):
        pin = overlay[key]
        require(x.pin(Path(pin['path'])) == pin, 'actual source input')
        pinned.append(pin)
    inputs = source_shape(json.loads(SOURCE_MANIFEST.read_bytes()))
    preimages = json.loads(Path(overlay['runtime_preimages']['path']).read_bytes())
    require(preimages['schema'] == 'ferric-p228-prefix-raw-timestamps-source-preimages-v1'
            and preimages['base_commit'] == COMMITS['fe2o3'], 'timestamp proposal baseline')
    require(preimages['files'] == {row['path']: row['before']['sha256'] for row in overlay['files']
                                  if row['before'] is not None}, 'proposal existing preimages')
    require(sorted(preimages['new_files']) == [RUNTIME_ROOT + RUNTIME_FILES[-1]], 'proposal new module')
    for row in overlay['files']:
        path = E / row['source']
        require(path.resolve(strict=True) == path, 'canonical runtime proposal source')
        pin = x.pin(path)
        require({key: pin[key] for key in ('bytes', 'sha256')} == row['after'], 'actual overlay source')
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
    for project, row in inputs['archives'].items():
        archive = Path(row['path'])
        require(archive.resolve(strict=True) == archive, 'canonical clean source archive')
        pin = x.pin(archive)
        require(pin == {key: row[key] for key in ('path', 'bytes', 'sha256')}, 'actual clean archive bytes')
        pinned.append(pin)
        extraction[project] = x.extract(archive, project, source)
    expected = x.snapshot(source)
    n.save(out / 'sources-base.json', expected)
    base_count = len(expected)
    install_overlay(source, expected, overlay['files'], lambda row: (E / row['source']).read_bytes(), x)
    require(len(expected) == base_count + 1, 'one new runtime module')
    n.save(out / 'sources-before.json', expected)
    worker = source / 'ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml'
    filters = filters_for(base, state)
    commands = state.recipes(str(n.N / 'bin/cargo'), worker, filters)
    require(len(commands) == len({name for name, _, _ in commands}) == 25, 'bounded phase roster')
    command_map = {name: (argv, deadline) for name, argv, deadline in commands}
    env = state.environment(n, out)
    phases, tests, metadata, binaries = {}, {}, None, {}
    error, post_errors = None, []

    def run(name):
        argv, deadline = command_map[name]
        phases[name] = n.run(out, name, argv, env=env, deadline=deadline)
        return (out / (name + '-stdout')).read_text()

    try:
        value = json.loads(run('metadata'))
        metadata = h.metadata_check(value, source, n.T, True, h.tomllib.loads(worker.with_name('Cargo.lock').read_text()))
        package = next(p for p in value['packages'] if p['name'] == 'fe2o3-kfd')
        features = next(p['features'] for p in value['resolve']['nodes'] if p['id'] == package['id'])
        require('engineering-gfx950' in features and 'live-validation' not in features, 'CPU runtime features')
        names = h.inventory(run('runtime-list'))
        added = set(overlay['added_runtime_tests'])
        exact_extension(names, prior_runtime, added)
        selected = set()
        for name, selector, count in filters:
            subset = {test for test in names if selector in test}
            wanted = {test for test in prior_runtime | added if selector in test}
            require(subset == wanted and len(subset) == count
                    and not subset.intersection(selected), 'actual runtime inventory: ' + name)
            selected.update(subset)
            tests[name] = h.results(run(name), subset, [(count, 0, 0)])
        require(len(selected) == 169 and added | state.BANK_TESTS | base.NEW_TESTS <= selected,
                'typed timestamp/currentness/poisoning/mapped-memory coverage')
        names = h.inventory(run('worker-list'))
        require(names == prior_worker and len(names) == 413, 'unchanged complete worker inventory')
        raw = run('worker-tests')
        tests['worker'] = h.results(raw, names, [(396, 0, 4), (13, 0, 0)])
        require(state.ignored_names(raw) == prior_ignored, 'unchanged worker ignored cases')
        binaries = h.artifacts(run('worker-build'), [h.CHILD_BIN], worker, n.T, x.pin)
        require(sum(t['passed'] for t in tests.values()) == 578
                and sum(t['ignored'] for t in tests.values()) == 4, 'actual combined ordinary census')
        require(set(phases) == set(command_map), 'every planned leaf completed')
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
    value = dict(schema='ferric-p228-prefix-raw-timestamps-cpu-result-v1', passed=passed,
        error=error, postcheck_errors=post_errors, inputs=pinned,
        package_manifest=x.pin(PACKAGE / 'manifest.json'), overlay=x.pin(PACKAGE / 'overlay.json'),
        source_manifest=x.pin(SOURCE_MANIFEST), source_commits=COMMITS, source_trees=TREES,
        extraction=extraction, prior_cpu_complete=x.pin(PRIOR / 'complete.json'),
        source_unchanged=not any(e.startswith('sources:') for e in post_errors),
        metadata=metadata, phases=phases, tests=tests, binaries=binaries,
        tests_passed=sum(t['passed'] for t in tests.values()), tests_ignored=sum(t['ignored'] for t in tests.values()),
        empty_initial_target=True, external_cargo_cache_reused=True,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False,
        timestamp_calibration=False, parent_rebuilt=False, production_authority=False,
        raw={p.name: x.pin(p) for p in out.iterdir() if p.is_file()})
    result = out / ('complete.json' if passed else 'failed.json')
    n.save(result, value)
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors, receipt=x.pin(result))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
