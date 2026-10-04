"""Fresh-source parent device-observation CPU qualification; no native launch."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
PACKAGE = E / 'p228-device-parent-cpu-v2'
PRIOR = E / 'host-policy-cpu-v228-v1'
PRIOR_SHA = '1fc4d17534161e6e7f96e6d0eba0a2455227ba2166623823f6a74f845b93deae'
HELPER = E / 'p228-host-policy-cpu-v1/run.py'
HELPER_SHA = '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494'
SOURCE_MANIFEST = E / 'device-parent-source-inputs-v228-v1.json'
SOURCE_DIR = 'p228-device-parent-source-v2'
PARENT = 'adapters/m1-engineering-execution-v1/'
NEW_BIN = 'ferric-qwen3-finite-prefix-decode-device-engineering'
DEVICE_SELECTOR = 'tp_finite_client::prefix_decode::device_v1::tests::'
DATA_SELECTOR = 'prefix_decode_device_observation_v1::tests::'
DESTINATIONS = {
    PARENT + 'Cargo.toml', PARENT + 'src/lib.rs',
    PARENT + 'src/tp_finite_client/prefix_decode.rs',
    PARENT + 'src/tp_finite_client/prefix_decode/host_policy_v2.rs',
    PARENT + 'src/tp_finite_client/prefix_decode/device_v1.rs',
    PARENT + 'src/tp_finite_client/prefix_decode/device_v1_tests.rs',
    PARENT + 'src/bin/' + NEW_BIN + '.rs',
}
NEW_DESTINATIONS = {
    PARENT + 'src/tp_finite_client/prefix_decode/device_v1.rs',
    PARENT + 'src/tp_finite_client/prefix_decode/device_v1_tests.rs',
    PARENT + 'src/bin/' + NEW_BIN + '.rs',
}
COMMITS = {'ferric': 'dc04a484a2424baa453f73fc832b2fa890875b44',
           'fe2o3': '9a321f3f98e597a75e8ebeafdda169ec10e12e9e'}
ARCHIVES = {'ferric': 'device-parent-ferric-dc04a484-v228-v1.tar.gz',
            'fe2o3': 'device-routing-fe2o3-9a321f3f-v228-v1.tar.gz'}


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
    require(value['schema'] == 'ferric-p228-device-parent-cpu-package-v1'
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


def named_tests(names):
    require(isinstance(names, list) and names and all(isinstance(name, str)
            and re.fullmatch('[A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)+', name) for name in names), 'named Rust tests')
    require(names == sorted(set(names)), 'sorted unique frozen test names')
    return set(names)


def overlay_shape(value):
    require(isinstance(value, dict) and set(value) == {'schema', 'source_manifest', 'files', 'added_parent_tests'},
            'overlay fields')
    require(value['schema'] == 'ferric-p228-device-parent-cpu-overlay-v1', 'overlay schema')
    require(file_pin(value['source_manifest'])['path'] == str(SOURCE_MANIFEST), 'actual source manifest')
    require(isinstance(value['files'], list) and len(value['files']) == 7, 'seven source members')
    seen = set()
    for row in value['files']:
        require(set(row) == {'path', 'source', 'before', 'after'}, 'overlay row fields')
        require(isinstance(row['path'], str) and row['path'] in DESTINATIONS and row['path'] not in seen,
                'closed unique parent destination')
        seen.add(row['path'])
        require(row['source'] == SOURCE_DIR + '/draft/' + row['path'], 'frozen source body path')
        require((row['before'] is None) == (row['path'] in NEW_DESTINATIONS), 'exact new/existing members')
        if row['before'] is not None:
            content_pin(row['before'])
        content_pin(row['after'])
    require(seen == DESTINATIONS, 'complete parent source delta')
    additions = value['added_parent_tests']
    require(isinstance(additions, dict) and set(additions) == {'lib', 'bin'}, 'test target fields')
    library = named_tests(additions['lib'])
    require(all(name.startswith((DEVICE_SELECTOR, DATA_SELECTOR)) for name in library)
            and any(name.startswith(DEVICE_SELECTOR) for name in library)
            and any(name.startswith(DATA_SELECTOR) for name in library), 'device module and shared schema coverage')
    require(len(named_tests(additions['bin'])) == 1, 'one explicit new bin test')
    return value


def source_shape(value):
    require(set(value) == {'schema', 'archives'} and value['schema'] == 'ferric-p228-clean-worker-sources-v1'
            and set(value['archives']) == {'ferric', 'fe2o3'}, 'paired clean source schema')
    for project, row in value['archives'].items():
        require(set(row) == {'path', 'bytes', 'sha256', 'commit', 'tree'}, 'source archive fields')
        file_pin({key: row[key] for key in ('path', 'bytes', 'sha256')})
        require(row['path'] == str(E / ARCHIVES[project]) and row['commit'] == COMMITS[project]
                and isinstance(row['tree'], str) and re.fullmatch('[0-9a-f]{40}', row['tree']),
                'pushed source generation and frozen tree')
    return value


def manifest_delta(before, after):
    expected_bin = dict(name=NEW_BIN, path='src/bin/' + NEW_BIN + '.rs',
                        **{'required-features': ['tp-batch-engineering']})
    require(after.get('bin', []).count(expected_bin) == 1 and expected_bin not in before.get('bin', []),
            'exact new opt-in bin')
    restored = dict(after, bin=[row for row in after['bin'] if row != expected_bin])
    require(restored == before, 'Cargo manifest changed outside new bin registration')


def environment(stable, out):
    stable.A, stable.T = out / 'sources/ferric' / PARENT, out / 'target'
    value = dict(stable.env(), CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_DEV_OPT_LEVEL='2',
                 CARGO_PROFILE_TEST_DEBUG='0', CARGO_PROFILE_DEV_DEBUG='0', TMPDIR=str(out / 'tmp'))
    require(value['RUSTC_BOOTSTRAP'] == 'fe2o3_device,fe2o3_macros'
            and value['RUSTUP_TOOLCHAIN'] == '1.97.1-x86_64-unknown-linux-gnu', 'qualified stable environment')
    return value


def recipes(h, stable, out, extended):
    cargo = str(stable.N / 'bin/cargo')
    parent = out / 'sources/ferric' / PARENT / 'Cargo.toml'
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(parent)]
    features = [*common, '--features', 'tp-batch-engineering']
    rows = [('parent-metadata', [cargo, 'metadata', '--offline', '--locked', '--manifest-path', str(parent),
             '--features', 'tp-batch-engineering', '--format-version', '1'], 120),
            ('parent-lib-list', [cargo, 'test', *features, '--lib', '--', '--list', '--format', 'terse'], 1200)]
    for name, selector, _ in h.FILTERS:
        rows.append((name, [cargo, 'test', *features, '--lib', selector, '--', '--test-threads=2'], 1200))
    if extended:
        rows.append(('parent-device-data', [cargo, 'test', *features, '--lib', DATA_SELECTOR,
                                           '--', '--test-threads=2'], 1200))
    bins = (*h.PARENT_BINS, NEW_BIN) if extended else h.PARENT_BINS
    for name in bins:
        argv = [cargo, 'test', *features, '--bin', name]
        rows.append((name + '-list', [*argv, '--', '--list', '--format', 'terse'], 1200))
        rows.append((name + '-tests', [*argv, '--', '--test-threads=2'], 1200))
    bin_args = [part for name in bins for part in ('--bin', name)]
    rows.append(('parent-builds', [cargo, 'build', '--profile', 'test', *features, *bin_args,
                                    '--message-format=json'], 1200))
    rows.append(('parent-default-check', [cargo, 'check', *common, '--lib', '--message-format=json'], 1200))
    require(len(rows) == len({row[0] for row in rows}) == (41 if extended else 38), 'closed parent phase roster')
    return rows


def authenticate_prior(h, x, stable, pinned):
    complete = PRIOR / 'complete.json'
    require(h.sha(complete) == PRIOR_SHA and complete.stat().st_size == 290447, 'actual CPU633 receipt')
    pinned.append(x.pin(complete))
    value = json.loads(complete.read_bytes())
    require(value['schema'] == 'ferric-p228-host-policy-cpu-result-v1' and value['passed'] is True
            and value['source_unchanged'] is True and value['tests_passed'] == 633
            and value['tests_ignored'] == 4, 'qualified historical result')
    require(all(value[key] is False for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim',
        'compiler_hsaco_reproduced', 'production_authority')), 'historical CPU-only scope')
    require(value['runner'] == x.pin(HELPER) and value['stable_environment'] == x.pin(h.STABLE)
            and value['helper'] == x.pin(h.BOUNDS) and value['extractor'] == x.pin(h.EXTRACTOR), 'historical helper identities')
    require(value['toolchains']['parent'] == dict(root=str(stable.N), pins=stable.TOOLS), 'historical parent toolchain')
    commands = recipes(h, stable, PRIOR, False)
    require(set(value['phases']) == {name for name, _, _ in commands}
            | {'worker-metadata', 'worker-list', 'worker-tests', 'worker-build'}, 'historical phase census')
    require(set(value['raw']) == {'sources-base.json', 'sources-before.json', 'sources-after.json'}
            | {name + suffix for name in value['phases'] for suffix in
               ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')}, 'historical raw census')
    for name, pin in value['raw'].items():
        require(file_pin(pin)['path'] == str(PRIOR / name) and x.pin(Path(pin['path'])) == pin, 'historical raw bytes')
        pinned.append(pin)
    for name, argv, deadline in commands:
        command = json.loads((PRIOR / (name + '-command.json')).read_bytes())
        require(command == dict(argv=argv, env=environment(stable, PRIOR), tools=stable.TOOLS,
            deadline_seconds=deadline, cache_cap_bytes=6 << 30, affinity=[8, 9], nice=10,
            gpu_execution=False, expected_exit=0), 'historical exact parent command/environment')
        started = json.loads((PRIOR / (name + '-started.json')).read_bytes())
        require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int and started['pid'] > 0
                and started['pid'] == started['pgid'], 'historical process group')
        result = json.loads((PRIOR / (name + '-result.json')).read_bytes())
        require(result == value['phases'][name] and result['exit_code'] == 0 and result['reason'] is None
                and result['group_absent'] is True and result['cache_bytes'] <= 6 << 30, 'historical natural owned exit')
        require(result['stdout_sha256'] == value['raw'][name + '-stdout']['sha256']
                and result['stderr_sha256'] == value['raw'][name + '-stderr']['sha256'], 'historical streams')
    library = h.inventory((PRIOR / 'parent-lib-list-stdout').read_text())
    selected, bins = set(), {}
    for name, selector, count in h.FILTERS:
        names = {test for test in library if selector in test}
        require(len(names) == count and not selected.intersection(names), 'historical disjoint selections')
        selected.update(names)
        actual = h.results((PRIOR / (name + '-stdout')).read_text(), names, [(count, 0, 0)])
        require(json.loads(json.dumps(actual)) == value['tests'][name], 'historical parent test outcomes')
    require(len(selected) == 224, 'historical selected library census')
    for binary in h.PARENT_BINS:
        names = h.inventory((PRIOR / (binary + '-list-stdout')).read_text())
        require(len(names) == 1, 'historical bin inventory')
        actual = h.results((PRIOR / (binary + '-tests-stdout')).read_text(), names, [(1, 0, 0)])
        require(json.loads(json.dumps(actual)) == value['tests'][binary], 'historical bin outcomes')
        bins[binary] = names
    require(set(value['tests']) == {name for name, _, _ in h.FILTERS} | set(h.PARENT_BINS) | {'worker'},
            'historical named test record census')
    return value, library, bins


def extended_selections(h, actual, prior, additions):
    require(not prior.intersection(additions) and actual == prior | additions, 'exact full parent library extension')
    selections, seen = {}, set()
    for name, selector, count in h.FILTERS:
        previous = {test for test in prior if selector in test}
        extra = {test for test in additions if selector in test}
        require(len(previous) == count, 'old parent names retained')
        require(not extra or name == 'parent-client', 'unexpected new names in old selector')
        names = {test for test in actual if selector in test}
        require(names == previous | extra and not seen.intersection(names), 'disjoint parent selection')
        selections[name] = names
        seen.update(names)
    names = {test for test in actual if test.startswith(DATA_SELECTOR)}
    require(names == {test for test in additions if test.startswith(DATA_SELECTOR)}
            and names and not seen.intersection(names), 'new shared schema selection')
    selections['parent-device-data'] = names
    seen.update(names)
    require(additions <= seen and len(seen) == 224 + len(additions), 'new tests executed exactly once')
    return selections


def metadata_generation(actual, prior, source):
    expected = {name: str(source / Path(path).relative_to(PRIOR / 'sources'))
                for name, path in prior['local'].items()}
    require(actual['local'] == expected and actual['package_count'] == prior['package_count']
            and actual['external'] == prior['external'], 'exact relocated parent dependency generation')


def install_overlay(source, expected, rows, read_source, x, toml):
    for row in rows:
        key = 'ferric/' + row['path']
        require(expected.get(key) == row['before'], 'parent source preimage: ' + key)
        target = source / key
        if row['before'] is None:
            require(not target.exists() and not target.is_symlink(), 'new source already exists')
        else:
            actual = x.pin(target)
            require({field: actual[field] for field in ('bytes', 'sha256')} == row['before'], 'actual source preimage')
        raw = read_source(row)
        require(len(raw) == row['after']['bytes'] and hashlib.sha256(raw).hexdigest() == row['after']['sha256'],
                'frozen replacement body')
        if row['path'] == PARENT + 'Cargo.toml':
            manifest_delta(toml.loads(target.read_text()), toml.loads(raw.decode()))
    for row in rows:
        target = source / 'ferric' / row['path']
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb' if row['before'] is None else 'wb') as stream:
            stream.write(read_source(row))
        expected['ferric/' + row['path']] = row['after']
    require(x.snapshot(source) == expected, 'unlisted source change')


def main():
    optimization_guard(sys.flags.optimize, os.environ)
    require(len(sys.argv) == 3, 'MANIFEST_SHA FRESH_LABEL')
    manifest_sha, label = sys.argv[1:]
    require(re.fullmatch(r'device-parent-cpu-v228-v[1-9][0-9]*', label), 'fresh parent CPU label')
    paths = package_inputs(manifest_sha)
    require(HELPER.resolve(strict=True) == HELPER and hashlib.sha256(HELPER.read_bytes()).hexdigest() == HELPER_SHA,
            'pinned historical controller APIs')
    h = types.ModuleType('qualified_parent_cpu_helpers')
    h.__file__ = str(HELPER)
    exec(compile(HELPER.read_bytes(), str(HELPER), 'exec'), h.__dict__)
    x = h.load(h.EXTRACTOR, h.EXTRACTOR_SHA, 'archive_helpers')
    n = h.load(h.BOUNDS, h.BOUNDS_SHA, 'owned_cpu_leaves')
    stable = h.load(h.STABLE, h.STABLE_SHA, 'qualified_stable_environment')
    pinned = [x.pin(path) for path in [*paths, HELPER, h.EXTRACTOR, h.BOUNDS, h.STABLE]]
    prior, old_library, old_bins = authenticate_prior(h, x, stable, pinned)
    overlay = overlay_shape(json.loads((PACKAGE / 'overlay.json').read_bytes()))
    require(x.pin(SOURCE_MANIFEST) == overlay['source_manifest'], 'actual frozen source manifest')
    pinned.append(overlay['source_manifest'])
    sources = source_shape(json.loads(SOURCE_MANIFEST.read_bytes()))
    for row in overlay['files']:
        path = E / row['source']
        require(path.resolve(strict=True) == path, 'canonical source body')
        pin = x.pin(path)
        require({key: pin[key] for key in ('bytes', 'sha256')} == row['after'], 'actual overlay body')
        pinned.append(pin)
    out = E / label
    out.mkdir(mode=0o700)
    source = out / 'sources'
    source.mkdir()
    n.F, n.T, n.N, n.PINS = source / 'ferric' / PARENT, out / 'target', stable.N, stable.TOOLS
    n.setup()
    require(not any(n.T.iterdir()), 'fresh empty target')
    (out / 'tmp').mkdir()
    extraction = {}
    for project, row in sources['archives'].items():
        path = Path(row['path'])
        require(path.resolve(strict=True) == path, 'canonical source archive')
        pin = x.pin(path)
        require(pin == {key: row[key] for key in ('path', 'bytes', 'sha256')}, 'actual clean archive bytes')
        pinned.append(pin)
        extraction[project] = x.extract(path, project, source)
    expected = x.snapshot(source)
    n.save(out / 'sources-base.json', expected)
    base_count = len(expected)
    install_overlay(source, expected, overlay['files'], lambda row: (E / row['source']).read_bytes(), x, h.tomllib)
    require(len(expected) == base_count + 3, 'exact three new parent source members')
    n.save(out / 'sources-before.json', expected)
    parent = source / 'ferric' / PARENT / 'Cargo.toml'
    commands = recipes(h, stable, out, True)
    by_name = {name: (argv, deadline) for name, argv, deadline in commands}
    env = environment(stable, out)
    phases, tests, metadata, binaries = {}, {}, None, {}
    error, post_errors = None, []
    additions = set(overlay['added_parent_tests']['lib'])

    def run(name):
        argv, deadline = by_name[name]
        phases[name] = n.run(out, name, argv, env=env, deadline=deadline)
        return (out / (name + '-stdout')).read_text()

    try:
        metadata = h.metadata_check(json.loads(run('parent-metadata')), source, n.T, False,
                                    h.tomllib.loads(parent.with_name('Cargo.lock').read_text()))
        metadata_generation(metadata, prior['metadata']['parent'], source)
        names = h.inventory(run('parent-lib-list'))
        selections = extended_selections(h, names, old_library, additions)
        for name, subset in selections.items():
            tests[name] = h.results(run(name), subset, [(len(subset), 0, 0)])
        for binary in (*h.PARENT_BINS, NEW_BIN):
            names = h.inventory(run(binary + '-list'))
            expected_names = set(overlay['added_parent_tests']['bin']) if binary == NEW_BIN else old_bins[binary]
            require(names == expected_names, 'exact old or declared new bin test names')
            tests[binary] = h.results(run(binary + '-tests'), names, [(len(names), 0, 0)])
        binaries = h.artifacts(run('parent-builds'), [*h.PARENT_BINS, NEW_BIN], parent, n.T, x.pin)
        run('parent-default-check')
        require(sum(row['passed'] for row in tests.values()) == 235 + len(additions)
                + len(overlay['added_parent_tests']['bin']) and all(row['ignored'] == 0 for row in tests.values()),
                'actual inventory-derived parent-only totals')
        require(set(phases) == set(by_name), 'every parent phase completed')
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
            for tool, sha in stable.TOOLS.items():
                require(h.sha(stable.N / 'bin' / tool) == sha, 'stable tool changed')
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
    value = dict(schema='ferric-p228-device-parent-cpu-result-v1', passed=passed,
        error=error, postcheck_errors=post_errors, inputs=pinned,
        package_manifest=x.pin(PACKAGE / 'manifest.json'), overlay=x.pin(PACKAGE / 'overlay.json'),
        source_manifest=x.pin(SOURCE_MANIFEST), source_commits=COMMITS,
        source_trees={key: row['tree'] for key, row in sources['archives'].items()}, extraction=extraction,
        prior_cpu_complete=x.pin(PRIOR / 'complete.json'),
        source_unchanged=not any(row.startswith('sources:') for row in post_errors),
        added_parent_tests=overlay['added_parent_tests'], metadata=metadata, phases=phases, tests=tests, binaries=binaries,
        tests_passed=sum(row['passed'] for row in tests.values()), tests_ignored=sum(row['ignored'] for row in tests.values()),
        empty_initial_target=True, external_cargo_cache_reused=True, parent_rebuilt=passed,
        worker_rebuilt=False, gpu_execution=False, numerical_acceptance=False, performance_claim=False,
        timestamp_calibration=False, compiler_hsaco_reproduced=False, production_authority=False,
        raw={path.name: x.pin(path) for path in out.iterdir() if path.is_file()})
    result = out / ('complete.json' if passed else 'failed.json')
    n.save(result, value)
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors, receipt=x.pin(result))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
