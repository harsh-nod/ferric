"""Fresh bounded parent/worker qualification of the explicit shared-full AR4 observer."""
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
PACKAGE = E / 'p228-projection-ar4-shared-host-cpu-v1'
OLD = E / 'projection-ar4-host-observation-cpu-v228-v1'
OLD_SHA = '7d8c08eeffab9cbebbf6abc973c0ba93d80616ad81063b35587e74d04df1d59c'
OLD_CONTROLLER = E / 'p228-projection-ar4-host-observation-cpu-v1/run.py'
OLD_CONTROLLER_SHA = '12898f6b9aa1a1af122cf61d0393089fa38cdb898097114dbb67a36ea4fc780a'
SOURCE_SHA = '1d173129c684afe5bcdc009ec939f8e4f0371bad8a4edecdf62b4548cb04f920'
HELPER = E / 'p228-host-policy-cpu-v1/run.py'
HELPER_SHA = '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494'
PRIOR_CONTROLLER = E / 'p228-projection-residual-runtime-cpu-v1/run.py'
PRIOR_CONTROLLER_SHA = '6fc90b8cbdda07c52c761767c7c463a3ea0ebecb244c5ace7a0fc3e274ac2d94'
PROPOSAL = E / 'p228-projection-ar4-shared-host-v1/source-manifest.json'
PROPOSAL_SHA = 'faaece5ab1afea85b7b6f03b02d772847889565a0ae70922180d6f62a1770461'
PARENT = 'adapters/m1-engineering-execution-v1'
WORKER = 'adapters/tp-peer-finite-engineering-worker-v1'
PLAIN_BIN = 'ferric-qwen3-finite-projection-residual-decode-engineering'
OLD_BIN = 'ferric-qwen3-finite-projection-residual-decode-host-engineering'
NEW_BIN = 'ferric-qwen3-finite-projection-residual-decode-shared-host-engineering'
REPORT = 'projection_residual_decode_host_observation_v1::tests::'
FILES = {PARENT + '/' + path for path in (
    'Cargo.toml', 'src/bin/' + NEW_BIN + '.rs', 'src/tp_finite_client/prefix_decode.rs',
    'src/tp_finite_client/prefix_decode/projection_host.rs',
    'src/tp_finite_client/prefix_decode/projection_tests.rs',
    'src/tp_finite_client/prefix_decode/projection_shared_host_tests.rs')}
FILES |= {WORKER + '/src/' + name + '.rs' for name in (
    'main', 'native_prefix_decode_host_v1', 'prefix_decode_host_observation_v1',
    'native_projection_residual_decode_host_v1', 'native_projection_residual_decode_shared_host_v1_tests',
    'projection_residual_decode_host_observation_v1', 'projection_residual_decode_host_observation_v1_tests',
    'projection_residual_decode_shared_host_v1_tests')}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pairs(rows):
    result = {}
    for key, value in rows:
        require(key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def document(path):
    require(path.resolve(strict=True) == path and path.is_file() and path.stat().st_size <= 16 << 20,
            'canonical bounded JSON')
    return json.loads(path.read_bytes(), object_pairs_hook=pairs)


def relocate(value, out):
    if isinstance(value, str):
        return value.replace(str(OLD), str(out))
    if isinstance(value, list):
        return [relocate(item, out) for item in value]
    if isinstance(value, dict):
        return {key: relocate(item, out) for key, item in value.items()}
    return value


def prior_contract(value):
    require(value['schema'] == 'ferric-p228-projection-ar4-host-observation-cpu-result-v1' and value['passed'] is True
        and value['error'] is None and value['postcheck_errors'] == [] and value['source_unchanged'] is True
        and value['tests_passed'] == 855 and value['tests_ignored'] == 4
        and len(value['phases']) == 61 and len(value['raw']) == 312 and len(value['binaries']) == 3
        and value['historical_runtime_tests_not_repeated'] == 208,
        'actual scoped CPU855 baseline')
    require(all(type(row['exit_code']) is int and row['exit_code'] == 0 and row['reason'] is None
        and row['group_absent'] is True for row in value['phases'].values()), 'natural baseline leaves')


def proposal_shape(value):
    require(value['schema'] == 'ferric-p228-projection-ar4-shared-host-source-proposal-v1'
        and value['base_cpu'] == dict(path=str(OLD / 'complete.json'), bytes=488734, sha256=OLD_SHA)
        and value['base_sources_after'] == dict(path=str(OLD / 'sources-after.json'), bytes=1269472, sha256=SOURCE_SHA)
        and len(value['files']) == 14 and {row['path'] for row in value['files']} == FILES,
        'exact fourteen-source shared-full overlay')
    require(sum(row['before'] is None for row in value['files']) == 4, 'four additions, ten replacements')
    for row in value['files']:
        require(row['source'] == 'draft/' + row['path'] and set(row['after']) == {'bytes', 'sha256'}
                and (row['before'] is None or set(row['before']) == {'bytes', 'sha256'}), 'source/preimage shape')
    require(value['renamed_tests'] == [] and value['old_test_names_removed'] == []
            and value['new_parent_binary'] == NEW_BIN and value['tests_executed'] is False,
            'additive names and distinct unexecuted parent binary')
    groups = value['added_tests']
    require(set(groups) == {'worker', 'parent_library', 'parent_binary'}, 'closed test groups')
    for role, count in (('worker', 14), ('parent_library', 13), ('parent_binary', 1)):
        require(len(groups[role]) == len(set(groups[role])) == count
                and all(re.fullmatch('[A-Za-z0-9_:]+', name) for name in groups[role]), 'unique full test names')
    shared = set(groups['worker']) & set(groups['parent_library'])
    require(len(shared) == 8 and all(name.startswith(REPORT) for name in shared)
        and value['added_worker_test_executions'] == value['added_parent_test_executions'] == 14,
        'eight shared report tests and twenty-eight added executions')
    require(value['worker_selector'] == '--engineering-native-projection-residual-decode-shared-host-v1'
        and value['shared_runtime_options'] == [False, False, True]
        and all(type(item) is bool for item in value['shared_runtime_options'])
        and value['original_group_fence_policy_unchanged'] is False
        and value['fresh_full_currentness_preserved'] is True
        and value['configuration_clock'] == 'inclusive-host-wall-nanoseconds-before-observer-enable'
        and value['configuration_time_in_snapshots'] is False, 'explicit fresh shared-full policy and timing scope')
    require(all(value[key] is False for key in ('compiler_modified', 'provider_modified', 'kernel_modified',
        'kfd_modified', 'limits_changed', 'operational_currentness_enabled', 'admission_cache_enabled',
        'live_sources_modified', 'gpu_execution')), 'unchanged lower layers and limits')


def overlay_map(before, rows):
    expected = dict(before)
    for row in rows:
        key = 'ferric/' + row['path']
        require((key not in before) if row['before'] is None else before.get(key) == row['before'],
                'exact CPU855 preimage or absent addition')
        expected[key] = row['after']
    require(len(expected) == len(before) + 4, 'four source additions only')
    return expected


def cargo_delta(before, after):
    value = copy.deepcopy(after)
    new = dict(name=NEW_BIN, path='src/bin/' + NEW_BIN + '.rs', **{'required-features': ['tp-batch-engineering']})
    require(value['bin'].count(new) == 1 and all(row['name'] != NEW_BIN for row in before['bin']),
            'one new opt-in Cargo binary')
    value['bin'].remove(new)
    require(value == before, 'no other Cargo/dependency/profile change')


def extended_inventory(prior, actual, additions):
    additions = set(additions)
    require(not prior.intersection(additions) and actual == prior | additions,
            'full inventory equals authentic prior plus named additions')
    return additions


def worker_summaries(previous, added):
    require([tuple(row) for row in previous] == [(505, 0, 4), (13, 0, 0)] and added == 14,
            'actual CPU855 library/shared_wire split and fourteen library additions')
    return [(519, 0, 4), (13, 0, 0)]


def selected_names(previous, actual, additions, selector):
    expected = previous | {name for name in additions if selector in name}
    require({name for name in actual if selector in name} == expected, 'selected old regressions plus exact additions')
    return expected


def metadata_delta(actual, previous, out, role):
    expected = relocate(previous, out)
    current = copy.deepcopy(actual)
    if role == 'parent':
        manifest = str(out / 'sources/ferric' / PARENT / 'Cargo.toml')
        old_package = next(row for row in expected['packages'] if row['manifest_path'] == manifest)
        package = next(row for row in current['packages'] if row['manifest_path'] == manifest)
        target = copy.deepcopy(next(row for row in old_package['targets'] if row['name'] == OLD_BIN))
        target['name'] = NEW_BIN
        target['src_path'] = str(out / 'sources/ferric' / PARENT / 'src/bin' / (NEW_BIN + '.rs'))
        require(package['targets'].count(target) == 1, 'exact additive Cargo target metadata')
        package['targets'].remove(target)
    require(current == expected, 'unchanged full relocated metadata/dependency graph')


def recipes_from(previous, prior, out, rows):
    require(len(previous) == 61 and set(previous) == set(prior['phases'])
        and prior['historical_runtime_tests_not_repeated'] == 208
        and all(name == 'worker-tests' or name.startswith(('parent-', 'ferric-')) for name in prior['tests']),
        'all sixty-one actual scoped parent/worker phases retained')
    recipes = {name: relocate(row, out) for name, row in previous.items()}
    require('parent-projection-host-report' in recipes and NEW_BIN + '-list' not in recipes
            and NEW_BIN + '-tests' not in recipes, 'old report selector and distinct new binary')
    for name in ('rustfmt', 'rustfmt-check'):
        argv = recipes[name]['argv']
        recipes[name]['argv'] = argv[:argv.index('skip_children=true') + 1] + [
            str(out / 'sources/ferric' / row['path']) for row in rows if row['path'].endswith('.rs')]
    for suffix in ('list', 'tests'):
        row = copy.deepcopy(recipes[OLD_BIN + '-' + suffix])
        row['argv'] = [NEW_BIN if arg == OLD_BIN else arg for arg in row['argv']]
        recipes[NEW_BIN + '-' + suffix] = row
    argv = recipes['parent-builds']['argv']
    start, end = argv.index('--bin'), argv.index('--message-format=json')
    require(argv[start:end] == ['--bin', PLAIN_BIN, '--bin', OLD_BIN], 'original selected binary build roster')
    recipes['parent-builds']['argv'] = argv[:start] + ['--bin', PLAIN_BIN, '--bin', OLD_BIN, '--bin', NEW_BIN] + argv[end:]
    require(len(recipes) == 63, 'sixty-three exact bounded phases')
    return recipes


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ,
            'ordinary bytecode-free controller')
    require(len(sys.argv) == 3, 'CONTROLLER_SHA FRESH_LABEL')
    controller_sha, label = sys.argv[1:]
    require(re.fullmatch('[0-9a-f]{64}', controller_sha)
            and re.fullmatch(r'projection-ar4-shared-host-cpu-v228-v[1-9][0-9]*', label), 'root source/fresh label')
    require(isinstance(PROPOSAL_SHA, str) and re.fullmatch('[0-9a-f]{64}', PROPOSAL_SHA),
            'source proposal binding is pending')
    require(hashlib.sha256(HELPER.read_bytes()).hexdigest() == HELPER_SHA, 'pinned helper')
    h = types.ModuleType('qualification'); h.__file__ = str(HELPER)
    exec(compile(HELPER.read_bytes(), str(HELPER), 'exec'), h.__dict__)
    x = h.load(h.EXTRACTOR, h.EXTRACTOR_SHA, 'source_helpers')
    n = h.load(h.BOUNDS, h.BOUNDS_SHA, 'bounded')
    q = h.load(PRIOR_CONTROLLER, PRIOR_CONTROLLER_SHA, 'prior_controller')
    q.OLD_TARGETS = (*q.OLD_TARGETS, E / 'projection-residual-runtime-cpu-v228-v1/target',
        E / 'projection-residual-decode-cpu-v228-v1/target', E / 'projection-ar4-cpu-v228-v1/target', OLD / 'target',
        E / 'rpo-compiler-cpu-v228-v2/target', E / 'kir-indexed-formal-join-cpu-v228-v2/target')
    controller = Path(__file__).resolve(strict=True)
    require(controller == PACKAGE / 'run.py' and h.sha(controller) == controller_sha, 'selected new controller')
    require(h.sha(OLD / 'complete.json') == OLD_SHA and h.sha(OLD_CONTROLLER) == OLD_CONTROLLER_SHA
            and h.sha(PROPOSAL) == PROPOSAL_SHA, 'actual baseline/controller and reviewed proposal')
    prior = document(OLD / 'complete.json'); prior_contract(prior)
    proposal = document(PROPOSAL); proposal_shape(proposal)
    pinned = [x.pin(path) for path in (controller, HELPER, h.EXTRACTOR, h.BOUNDS, PRIOR_CONTROLLER,
              OLD_CONTROLLER, OLD / 'complete.json', PROPOSAL)]
    for name, row in prior['raw'].items():
        require(row['path'] == str(OLD / name) and x.pin(OLD / name) == row, 'all authentic baseline raw bytes')
        pinned.append(row)
    for value in prior['binaries'].values():
        row = value['binary']; require(x.pin(Path(row['path'])) == row, 'preserved original three production ELFs')
        pinned.append(row)
    require(prior['raw']['sources-after.json']['sha256'] == SOURCE_SHA, 'qualified formatted source identity')
    before = document(OLD / 'sources-after.json')
    require(len(before) == 6999 and x.snapshot(OLD / 'sources') == before, 'unchanged complete original paired source')
    out = E / label; out.mkdir(mode=0o700)
    source = out / 'sources'; source.mkdir()
    n.F, n.T = source / 'fe2o3', out / 'target'
    n.setup(); require(not any(n.T.iterdir()), 'fresh private aggregate target')
    (out / 'tmp').mkdir()
    require(len(before) < 12000 and sum(row['bytes'] for row in before.values()) <= 256 << 20, 'bounded source copy')
    for name, row in before.items():
        rel = Path(name)
        require(not rel.is_absolute() and '..' not in rel.parts and rel.parts[0] in ('ferric', 'fe2o3')
                and row['bytes'] <= 16 << 20, 'bounded paired source member')
        origin, target = OLD / 'sources' / rel, source / rel
        require(x.pin(origin) == dict(path=str(origin), **row), 'source pre-copy identity')
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(origin.read_bytes())
        target.chmod(0o700 if origin.stat().st_mode & 0o111 else 0o600)
    require(x.snapshot(source) == before, 'exact copied qualified source, no archive assumptions')
    n.save(out / 'sources-base.json', before)
    rows = proposal['files']; expected = overlay_map(before, rows)
    cargo = source / 'ferric' / PARENT / 'Cargo.toml'
    old_cargo = h.tomllib.loads(cargo.read_text())
    for row in rows:
        body = PROPOSAL.parent / row['source']; record = x.pin(body)
        require(body.resolve(strict=True) == body and {key: record[key] for key in ('bytes', 'sha256')} == row['after'],
                'exact reviewed source body')
        pinned.append(record)
        target = source / 'ferric' / row['path']; target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb' if row['before'] is None else 'wb') as stream:
            stream.write(body.read_bytes())
    require(x.snapshot(source) == expected, 'only fourteen reviewed source changes')
    cargo_delta(old_cargo, h.tomllib.loads(cargo.read_text()))
    n.save(out / 'sources-unformatted.json', expected)
    previous = {name: document(OLD / (name + '-command.json')) for name in prior['phases']}
    recipes = recipes_from(previous, prior, out, rows)
    require(h.sha(Path(recipes['rustfmt']['argv'][0])) == q.RUSTFMT_SHA, 'qualified formatter')
    pinned.append(x.pin(Path(recipes['rustfmt']['argv'][0])))
    old_targets = q.target_inventory(); n.save(out / 'old-targets-before.json', old_targets)
    config = q.configurations(source, R / 'toolchain/cargo', x); n.save(out / 'configurations.json', config)
    tools_seen = set()
    for recipe in recipes.values():
        for tool, sha in recipe['tools'].items():
            path = Path(recipe['env']['RUSTC']).parent / tool
            require(h.sha(path) == sha, 'unchanged qualified toolchain')
            if path not in tools_seen:
                pinned.append(x.pin(path)); tools_seen.add(path)
    phases, tests, binaries, metadata = {}, {}, {}, {}
    error, post_errors = None, []

    def run(name):
        recipe = recipes[name]
        n.N = Path(recipe['env']['RUSTC']).parent.parent; n.PINS = recipe['tools']
        n.F = source / ('fe2o3' if recipe['env']['CARGO_TARGET_DIR'].endswith('/worker') else 'ferric/' + PARENT)
        try:
            n.run(out, name, recipe['argv'], env=recipe['env'], deadline=recipe['deadline_seconds'])
        finally:
            result = out / (name + '-result.json')
            if result.is_file(): phases[name] = document(result)
        return (out / (name + '-stdout')).read_text()

    def test(name, names, summaries):
        tests[name] = h.results(run(name), names, summaries)

    try:
        run('rustfmt')
        formatted = x.snapshot(source)
        allowed = {'ferric/' + row['path'] for row in rows if row['path'].endswith('.rs')}
        require(len(allowed) == 13 and set(formatted) == set(expected)
                and all(formatted[key] == row for key, row in expected.items() if key not in allowed), 'formatter scope')
        expected = formatted; n.save(out / 'sources-before.json', expected)
        run('rustfmt-check')
        for role, directory in (('worker', WORKER), ('parent', PARENT)):
            manifest = source / 'ferric' / directory / 'Cargo.toml'
            raw_metadata = json.loads(run(role + '-metadata'))
            metadata_delta(raw_metadata, document(OLD / (role + '-metadata-stdout')), out, role)
            metadata[role] = h.metadata_check(raw_metadata, source, out / 'target' / role, role == 'worker',
                                             h.tomllib.loads(manifest.with_name('Cargo.lock').read_text()))
            require(metadata[role] == relocate(prior['metadata'][role], out), 'unchanged dependencies')
        worker_names = h.inventory(run('worker-list'))
        extended_inventory(set(prior['tests']['worker-tests']['names']), worker_names, proposal['added_tests']['worker'])
        test('worker-tests', worker_names,
             worker_summaries(prior['tests']['worker-tests']['summaries'], len(proposal['added_tests']['worker'])))
        ignored = lambda raw: set(re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. ignored', raw, re.M))
        require(ignored((out / 'worker-tests-stdout').read_text()) == ignored((OLD / 'worker-tests-stdout').read_text()),
                'same four historical ignored tests')
        binaries.update(h.artifacts(run('worker-build'), [h.CHILD_BIN], source / 'ferric' / WORKER / 'Cargo.toml',
                                    out / 'target/worker', x.pin))
        parent_names = h.inventory(run('parent-lib-list'))
        additions = extended_inventory(h.inventory((OLD / 'parent-lib-list-stdout').read_text()), parent_names,
                                       proposal['added_tests']['parent_library'])
        selected = set()
        for name, previous_test in prior['tests'].items():
            if name.startswith('ferric-'):
                binary = name.removesuffix('-tests')
                require(h.inventory(run(binary + '-list')) == set(previous_test['names']), 'same old binary inventory')
                test(name, set(previous_test['names']), [tuple(row) for row in previous_test['summaries']])
            elif name.startswith('parent-'):
                argv = recipes[name]['argv']; selector = argv[argv.index('--lib') + 1]
                names = selected_names(set(previous_test['names']), parent_names, additions, selector)
                test(name, names, [(len(names), 0, 0)]); selected.update(names)
        require(additions <= selected, 'all thirteen new parent library tests executed by existing selectors')
        new_bin_names = set(proposal['added_tests']['parent_binary'])
        require(h.inventory(run(NEW_BIN + '-list')) == new_bin_names, 'new opt-in binary inventory')
        test(NEW_BIN + '-tests', new_bin_names, [(1, 0, 0)])
        binaries.update(h.artifacts(run('parent-builds'), [PLAIN_BIN, OLD_BIN, NEW_BIN], source / 'ferric' / PARENT / 'Cargo.toml',
                                    out / 'target/parent', x.pin))
        run('parent-default-check')
        require(set(phases) == set(recipes) and len(phases) == 63 and len(binaries) == 4
                and set(tests) == set(prior['tests']) | {NEW_BIN + '-tests'},
                'all scoped recipes and regressions complete')
        require(sum(row['passed'] for row in tests.values()) == 883
                and sum(row['ignored'] for row in tests.values()) == 4, 'actual named scoped883/four old ignores')
    except BaseException as failure:
        error = repr(failure)
    finally:
        for label_, check in [
            ('sources', lambda: require(x.snapshot(source) == expected, 'candidate source/lock drift')),
            ('baseline-source', lambda: require(x.snapshot(OLD / 'sources') == before, 'original CPU855 source drift')),
            ('old-targets', lambda: require(q.target_inventory() == old_targets, 'old cache modified')),
            ('configurations', lambda: require(q.configurations(source, R / 'toolchain/cargo', x) == config, 'config drift')),
            ('inputs', lambda: require(all(x.pin(Path(row['path'])) == row for row in pinned), 'input/tool/old ELF drift')),
            ('dependencies', lambda: require(all(h.sha(Path(row['manifest'])) == row['manifest_sha256']
                for value in metadata.values() for row in value['external']), 'dependency drift')),
            ('binaries', lambda: require(all(x.pin(Path(row['binary']['path'])) == row['binary']
                for row in binaries.values()), 'new ELF drift')),
            ('target', lambda: require(n.size(out / 'target') <= 6 << 30, 'aggregate target cap')),
        ]:
            try:
                check()
            except BaseException as failure:
                post_errors.append(label_ + ': ' + repr(failure))
        n.save(out / 'sources-after.json', x.snapshot(source))
        n.save(out / 'old-targets-after.json', q.target_inventory())
    passed = error is None and not post_errors
    result = dict(schema='ferric-p228-projection-ar4-shared-host-cpu-result-v1', passed=passed,
        error=error, postcheck_errors=post_errors, prior_completion=x.pin(OLD / 'complete.json'),
        controller=x.pin(controller), proposal=x.pin(PROPOSAL), inputs=pinned,
        metadata=metadata, phases=phases, tests=tests, binaries=binaries,
        declared_test_additions=proposal['added_tests'], historical_runtime_tests_not_repeated=208,
        tests_passed=sum(row['passed'] for row in tests.values()), tests_ignored=sum(row['ignored'] for row in tests.values()),
        source_unchanged=not any(row.startswith('sources:') for row in post_errors), empty_initial_target=True,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False, production_authority=False,
        full_cpu1037_cohort_requalified=False, compiler_requalified=False,
        raw={path.name: x.pin(path) for path in out.iterdir() if path.is_file()})
    path = out / ('complete.json' if passed else 'failed.json'); n.save(path, result)
    print(json.dumps(dict(passed=passed, receipt=x.pin(path), error=error, postcheck_errors=post_errors)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
