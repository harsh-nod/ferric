"""Fresh-source candidate-only layer0 parent CPU qualification; no native launch."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
PACKAGE = E / 'p228-layer0-native-capture-cpu-v2'
PRIOR = E / 'gfx950-clock-parent-cpu-v228-v1'
PRIOR_SHA = 'd2a118dd2a3bfac749b18de26883a661a1078a22ebf374853a11b081f43b1484'
PRIOR_CONTROLLER = E / 'p228-gfx950-clock-parent-cpu-v1/run.py'
PRIOR_CONTROLLER_SHA = '8b78543ee92862a53e6b4ed8e41942aec559b7e8b5dd513886e6966ff9d27332'
HELPER = E / 'p228-host-policy-cpu-v1/run.py'
HELPER_SHA = '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494'
SOURCE_MANIFEST = E / 'layer0-native-capture-source-inputs-v228-v2.json'
PARENT_SOURCE_DIR = 'p228-layer0-native-capture-source-v1'
PARENT = 'adapters/m1-engineering-execution-v1/'
WORKER = 'adapters/tp-peer-finite-engineering-worker-v1/'
OLD_DEVICE_BIN = 'ferric-qwen3-finite-prefix-decode-device-engineering'
OLD_CLOCK_BIN = 'ferric-qwen3-finite-prefix-decode-device-clock-engineering'
NEW_BIN = 'ferric-qwen3-finite-prefix-layer-capture-engineering'
CAPTURE_SELECTOR = 'tp_finite_client::prefix_layer::capture::tests::'
EVIDENCE_SELECTOR = 'tp_finite_client::prefix_layer::evidence::capture_tests::'
DATA_SELECTOR = 'prefix_decode_device_clock_observation_v2::tests::'
OLD_DATA_SELECTOR = 'prefix_decode_device_observation_v1::tests::'
DESTINATIONS = {
    PARENT + 'Cargo.toml',
    PARENT + 'src/tp_finite_client/prefix_layer.rs',
    PARENT + 'src/tp_finite_client/prefix_layer/evidence.rs',
    PARENT + 'src/tp_finite_client/prefix_layer/capture.rs',
    PARENT + 'src/tp_finite_client/prefix_layer/capture_tests.rs',
    PARENT + 'src/tp_finite_client/prefix_layer/capture_evidence_tests.rs',
    PARENT + 'src/bin/' + NEW_BIN + '.rs',
}
NEW_DESTINATIONS = {
    PARENT + 'src/tp_finite_client/prefix_layer/capture.rs',
    PARENT + 'src/tp_finite_client/prefix_layer/capture_tests.rs',
    PARENT + 'src/tp_finite_client/prefix_layer/capture_evidence_tests.rs',
    PARENT + 'src/bin/' + NEW_BIN + '.rs',
}
COMMITS = {'ferric': '1a9a2551ee7af44d5482cc37f46cf008158b8d10',
           'fe2o3': '27b53d2b74c1f239988a891a4aed39e089b05663'}
TREES = {'ferric': '6a7ebe6691d2ef7ec96b9da3a4ad1874b1e33a2a',
         'fe2o3': '3c2fa509b7328c4bf7d8e2ef2aab3a0e0c4a422e'}
ARCHIVES = {'ferric': 'layer0-capture-ferric-1a9a2551-v228-v2.tar.gz',
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
    require(value['schema'] == 'ferric-p228-layer0-native-capture-cpu-package-v1'
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
    require(value['schema'] == 'ferric-p228-layer0-native-capture-cpu-overlay-v1', 'overlay schema')
    require(file_pin(value['source_manifest'])['path'] == str(SOURCE_MANIFEST), 'actual source manifest')
    require(isinstance(value['files'], list) and len(value['files']) == 7, 'seven parent-only source members')
    seen = set()
    for row in value['files']:
        require(set(row) == {'path', 'source', 'before', 'after'}, 'overlay row fields')
        require(isinstance(row['path'], str) and row['path'] in DESTINATIONS and row['path'] not in seen,
                'closed unique parent-only destination')
        seen.add(row['path'])
        directory = PARENT_SOURCE_DIR
        require(row['source'] == directory + '/draft/' + row['path'], 'frozen source body path')
        require((row['before'] is None) == (row['path'] in NEW_DESTINATIONS), 'exact new/existing members')
        if row['before'] is not None:
            content_pin(row['before'])
        content_pin(row['after'])
    require(seen == DESTINATIONS, 'complete parent-only source delta')
    additions = value['added_parent_tests']
    require(isinstance(additions, dict) and set(additions) == {'lib', 'bin'}, 'test target fields')
    library = named_tests(additions['lib'])
    require(len(library) == 13 and all(name.startswith((CAPTURE_SELECTOR, EVIDENCE_SELECTOR)) for name in library)
            and sum(name.startswith(CAPTURE_SELECTOR) for name in library) == 9
            and sum(name.startswith(EVIDENCE_SELECTOR) for name in library) == 4,
            'nine capture and four evidence tests under the existing parent selector')
    require(len(named_tests(additions['bin'])) == 1, 'one explicit new bin test')
    return value


def source_shape(value):
    require(set(value) == {'schema', 'archives'} and value['schema'] == 'ferric-p228-clean-worker-sources-v1'
            and set(value['archives']) == {'ferric', 'fe2o3'}, 'paired clean source schema')
    for project, row in value['archives'].items():
        require(set(row) == {'path', 'bytes', 'sha256', 'commit', 'tree'}, 'source archive fields')
        file_pin({key: row[key] for key in ('path', 'bytes', 'sha256')})
        require(row['path'] == str(E / ARCHIVES[project]) and row['commit'] == COMMITS[project]
                and row['tree'] == TREES[project],
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
    rows.append(('parent-device-data', [cargo, 'test', *features, '--lib', OLD_DATA_SELECTOR,
                                       '--', '--test-threads=2'], 1200))
    rows.append(('parent-clock-data', [cargo, 'test', *features, '--lib', DATA_SELECTOR,
                                       '--', '--test-threads=2'], 1200))
    bins = (*h.PARENT_BINS, OLD_DEVICE_BIN, OLD_CLOCK_BIN)
    if extended:
        bins = (*bins, NEW_BIN)
    for name in bins:
        argv = [cargo, 'test', *features, '--bin', name]
        rows.append((name + '-list', [*argv, '--', '--list', '--format', 'terse'], 1200))
        rows.append((name + '-tests', [*argv, '--', '--test-threads=2'], 1200))
    bin_args = [part for name in bins for part in ('--bin', name)]
    rows.append(('parent-builds', [cargo, 'build', '--profile', 'test', *features, *bin_args,
                                    '--message-format=json'], 1200))
    rows.append(('parent-default-check', [cargo, 'check', *common, '--lib', '--message-format=json'], 1200))
    require(len(rows) == len({row[0] for row in rows}) == (46 if extended else 44), 'closed parent phase roster')
    return rows


def old_selectors(h):
    return [(name, selector) for name, selector, _ in h.FILTERS] + [
        ('parent-device-data', OLD_DATA_SELECTOR), ('parent-clock-data', DATA_SELECTOR)]


def authenticate_prior(h, x, stable, pinned):
    complete = PRIOR / 'complete.json'
    require(h.sha(complete) == PRIOR_SHA and complete.stat().st_size == 316284, 'actual parent275 receipt')
    pinned.append(x.pin(complete))
    value = json.loads(complete.read_bytes())
    require(value['schema'] == 'ferric-p228-gfx950-clock-parent-cpu-result-v1' and value['passed'] is True
            and value['error'] is None and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['tests_passed'] == 275 and value['tests_ignored'] == 0
            and value['parent_rebuilt'] is True and value['worker_rebuilt'] is False
            and value['sibling_runtime_rebuilt'] is False, 'qualified parent baseline')
    require(all(value[key] is False for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim',
        'compiler_hsaco_reproduced', 'production_authority', 'timestamp_calibration')), 'historical CPU-only scope')
    require(h.sha(PRIOR_CONTROLLER) == PRIOR_CONTROLLER_SHA, 'exact qualified parent controller')
    require(all(x.pin(path) in value['inputs'] for path in
            (PRIOR_CONTROLLER, HELPER, h.EXTRACTOR, h.BOUNDS, h.STABLE)), 'historical helper identities')
    pinned.append(x.pin(PRIOR_CONTROLLER))
    commands = recipes(h, stable, PRIOR, False)
    require(set(value['phases']) == {name for name, _, _ in commands}, 'historical44 parent phases')
    require(set(value['raw']) == {'sources-base.json', 'sources-before.json', 'sources-after.json'}
            | {name + suffix for name in value['phases'] for suffix in
               ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')}
            and len(value['raw']) == 223, 'historical223 raw census')
    for name, pin in value['raw'].items():
        require(file_pin(pin)['path'] == str(PRIOR / name) and x.pin(Path(pin['path'])) == pin, 'historical raw bytes')
        pinned.append(pin)
    require(json.loads((PRIOR / 'sources-before.json').read_bytes())
            == json.loads((PRIOR / 'sources-after.json').read_bytes()), 'historical immutable compiled source')
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
    require(len(library) == 765, 'historical full compiled library inventory')
    selected, bins = set(), {}
    for name, selector in old_selectors(h):
        names = {test for test in library if selector in test}
        require(names and not selected.intersection(names), 'historical disjoint selections')
        selected.update(names)
        actual = h.results((PRIOR / (name + '-stdout')).read_text(), names, [(len(names), 0, 0)])
        require(json.loads(json.dumps(actual)) == value['tests'][name], 'historical actual parent test outcomes')
    require(len(selected) == 262, 'historical selected library census')
    for binary in (*h.PARENT_BINS, OLD_DEVICE_BIN, OLD_CLOCK_BIN):
        names = h.inventory((PRIOR / (binary + '-list-stdout')).read_text())
        require(len(names) == 1, 'historical bin inventory')
        actual = h.results((PRIOR / (binary + '-tests-stdout')).read_text(), names, [(1, 0, 0)])
        require(json.loads(json.dumps(actual)) == value['tests'][binary], 'historical bin outcomes')
        bins[binary] = names
    require(set(value['tests']) == {name for name, _ in old_selectors(h)} | set(bins),
            'historical named test record census')
    return value, library, bins


def extended_selections(h, actual, prior, additions):
    require(not prior.intersection(additions) and actual == prior | additions, 'exact full parent library extension')
    selections, seen = {}, set()
    for name, selector in old_selectors(h):
        previous = {test for test in prior if selector in test}
        extra = {test for test in additions if selector in test}
        require(previous and (not extra or name == 'parent-client'), 'old parent names retained in exact selectors')
        names = {test for test in actual if selector in test}
        require(names == previous | extra and not seen.intersection(names), 'disjoint parent selection')
        selections[name] = names
        seen.update(names)
    prior_selected = set().union(*({test for test in prior if selector in test} for _, selector in old_selectors(h)))
    require(additions <= seen and len(seen) == len(prior_selected) + len(additions), 'new tests executed exactly once')
    return selections


def metadata_generation(actual, prior, source):
    expected = {name: str(source / Path(path).relative_to(PRIOR / 'sources'))
                for name, path in prior['local'].items()}
    require(actual['local'] == expected and actual['package_count'] == prior['package_count']
            and actual['external'] == prior['external'], 'exact relocated parent dependency generation')


def compatible_source_base(actual, prior, metadata):
    roots = {str(Path(path).parent.relative_to(PRIOR / 'sources')) + '/'
             for path in metadata['local'].values()}
    require(roots and all(root.startswith('ferric/') for root in roots)
            and 'ferric/' + PARENT in roots, 'prior local Ferric dependency roots')
    roots.add('ferric/' + WORKER)
    ignored = {'ferric/' + PARENT + 'README.md', 'ferric/' + WORKER + 'README.md'}
    root_files = {'ferric/Cargo.toml', 'ferric/Cargo.lock',
                  'ferric/rust-toolchain', 'ferric/rust-toolchain.toml'}
    def selected(source):
        return {path: pin for path, pin in source.items()
                if path not in ignored and (path in root_files or path.startswith('ferric/.cargo/')
                    or any(path.startswith(root) for root in roots))}
    expected = selected(prior)
    require(expected and selected(actual) == expected,
            'fresh parent/local dependencies/shared worker source differs from parent275')
    # The paired runtime archive is retained but the parent uses unchanged locked
    # Git dependencies. It is not silently substituted into the parent build.


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
    require(re.fullmatch(r'layer0-native-capture-cpu-v228-v[1-9][0-9]*', label), 'fresh parent CPU label')
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
    compatible_source_base(expected, json.loads((PRIOR / 'sources-after.json').read_bytes()), prior['metadata'])
    n.save(out / 'sources-base.json', expected)
    base_count = len(expected)
    install_overlay(source, expected, overlay['files'], lambda row: (E / row['source']).read_bytes(), x, h.tomllib)
    require(len(expected) == base_count + len(NEW_DESTINATIONS), 'exact declared new source members')
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
        metadata_generation(metadata, prior['metadata'], source)
        names = h.inventory(run('parent-lib-list'))
        selections = extended_selections(h, names, old_library, additions)
        for name, subset in selections.items():
            tests[name] = h.results(run(name), subset, [(len(subset), 0, 0)])
        for binary in (*h.PARENT_BINS, OLD_DEVICE_BIN, OLD_CLOCK_BIN, NEW_BIN):
            names = h.inventory(run(binary + '-list'))
            expected_names = set(overlay['added_parent_tests']['bin']) if binary == NEW_BIN else old_bins[binary]
            require(names == expected_names, 'exact old or declared new bin test names')
            tests[binary] = h.results(run(binary + '-tests'), names, [(len(names), 0, 0)])
        binaries = h.artifacts(run('parent-builds'), [*h.PARENT_BINS, OLD_DEVICE_BIN, OLD_CLOCK_BIN, NEW_BIN], parent, n.T, x.pin)
        run('parent-default-check')
        require(sum(row['passed'] for row in tests.values()) == prior['tests_passed'] + len(additions)
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
    value = dict(schema='ferric-p228-layer0-native-capture-cpu-result-v1', passed=passed,
        error=error, postcheck_errors=post_errors, inputs=pinned,
        package_manifest=x.pin(PACKAGE / 'manifest.json'), overlay=x.pin(PACKAGE / 'overlay.json'),
        source_manifest=x.pin(SOURCE_MANIFEST), source_commits=COMMITS,
        source_trees={key: row['tree'] for key, row in sources['archives'].items()}, extraction=extraction,
        prior_cpu_complete=x.pin(PRIOR / 'complete.json'),
        source_unchanged=not any(row.startswith('sources:') for row in post_errors),
        added_parent_tests=overlay['added_parent_tests'], metadata=metadata, phases=phases, tests=tests, binaries=binaries,
        tests_passed=sum(row['passed'] for row in tests.values()), tests_ignored=sum(row['ignored'] for row in tests.values()),
        empty_initial_target=True, external_cargo_cache_reused=True, parent_rebuilt=passed,
        worker_rebuilt=False, sibling_runtime_rebuilt=False, gpu_execution=False, numerical_acceptance=False, performance_claim=False,
        timestamp_calibration=False, compiler_hsaco_reproduced=False, production_authority=False,
        raw={path.name: x.pin(path) for path in out.iterdir() if path.is_file()})
    result = out / ('complete.json' if passed else 'failed.json')
    n.save(result, value)
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors, receipt=x.pin(result))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
