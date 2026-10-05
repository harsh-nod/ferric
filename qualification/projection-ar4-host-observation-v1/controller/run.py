"""Fresh bounded parent/worker qualification of the additive AR4 host observer."""
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
PACKAGE = E / 'p228-projection-ar4-host-observation-cpu-v1'
OLD = E / 'projection-ar4-cpu-v228-v1'
OLD_SHA = '7a3c170ca6ffe6000517588280fcdb99cb58c0cbb22cc3f4be910232f7d51c54'
OLD_CONTROLLER = E / 'p228-projection-ar4-cpu-v1/run.py'
OLD_CONTROLLER_SHA = 'a4f9fe5df6d6203003eaffd11ba0a7dee0586e16a5b0fc419599c8bab16b57b3'
SOURCE_SHA = '22c23c3effbf79c1b38e8c816f43474cdc201aedb971d81bffd75c99dc5ba0a9'
HELPER = E / 'p228-host-policy-cpu-v1/run.py'
HELPER_SHA = '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494'
PRIOR_CONTROLLER = E / 'p228-projection-residual-runtime-cpu-v1/run.py'
PRIOR_CONTROLLER_SHA = '6fc90b8cbdda07c52c761767c7c463a3ea0ebecb244c5ace7a0fc3e274ac2d94'
PROPOSAL = E / 'p228-projection-ar4-host-observation-v1/source-manifest.json'
PROPOSAL_SHA = 'ab5d055a09c320780a0d0da7d235bb0fe023469c9132b37a877fbc7841738074'
PARENT = 'adapters/m1-engineering-execution-v1'
WORKER = 'adapters/tp-peer-finite-engineering-worker-v1'
OLD_BIN = 'ferric-qwen3-finite-projection-residual-decode-engineering'
NEW_BIN = 'ferric-qwen3-finite-projection-residual-decode-host-engineering'
REPORT = 'projection_residual_decode_host_observation_v1::tests::'
FILES = {PARENT + '/' + path for path in (
    'Cargo.toml', 'src/lib.rs', 'src/bin/' + NEW_BIN + '.rs',
    'src/tp_finite_client/prefix_decode.rs', 'src/tp_finite_client/prefix_decode/host_observation.rs',
    'src/tp_finite_client/prefix_decode/projection.rs', 'src/tp_finite_client/prefix_decode/projection_host.rs',
    'src/tp_finite_client/prefix_decode/projection_tests.rs')}
FILES |= {WORKER + '/src/' + name + '.rs' for name in (
    'lib', 'main', 'native_prefix_decode_cli_v1', 'native_prefix_decode_host_v1',
    'native_projection_residual_decode_cli_v1', 'native_projection_residual_decode_host_v1',
    'native_projection_residual_decode_host_v1_tests', 'projection_residual_decode_host_observation_v1',
    'projection_residual_decode_host_observation_v1_tests')}


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
    require(value['schema'] == 'ferric-projection-ar4-cpu-result-v1' and value['passed'] is True
        and value['error'] is None and value['postcheck_errors'] == [] and value['source_unchanged'] is True
        and value['tests_passed'] == 1037 and value['tests_ignored'] == 4
        and len(value['phases']) == 87 and len(value['raw']) == 442 and len(value['binaries']) == 17,
        'actual CPU1037 baseline')
    require(all(type(row['exit_code']) is int and row['exit_code'] == 0 and row['reason'] is None
        and row['group_absent'] is True for row in value['phases'].values()), 'natural baseline leaves')


def proposal_shape(value):
    require(value['schema'] == 'ferric-p228-projection-ar4-host-observation-source-proposal-v1'
        and value['base_cpu'] == dict(path=str(OLD / 'complete.json'), bytes=583779, sha256=OLD_SHA)
        and len(value['files']) == 17 and {row['path'] for row in value['files']} == FILES,
        'exact seventeen-source observation overlay')
    require(sum(row['before'] is None for row in value['files']) == 6, 'six additions, eleven replacements')
    for row in value['files']:
        require(row['source'] == 'draft/' + row['path'] and set(row['after']) == {'bytes', 'sha256'}
                and (row['before'] is None or set(row['before']) == {'bytes', 'sha256'}), 'source/preimage shape')
    require(value['renamed_tests'] == [] and value['old_test_names_removed'] == []
            and value['new_parent_binary'] == NEW_BIN and value['tests_executed'] is False,
            'additive names and distinct unexecuted parent binary')
    groups = value['added_tests']
    require(set(groups) == {'worker', 'parent_library', 'parent_binary'}, 'closed test groups')
    for role, count in (('worker', 13), ('parent_library', 12), ('parent_binary', 1)):
        require(len(groups[role]) == len(set(groups[role])) == count
                and all(re.fullmatch('[A-Za-z0-9_:]+', name) for name in groups[role]), 'unique full test names')
    shared = set(groups['worker']) & set(groups['parent_library'])
    require(len(shared) == 8 and all(name.startswith(REPORT) for name in shared)
        and value['added_worker_test_executions'] == value['added_parent_test_executions'] == 13,
        'eight shared report tests and twenty-six added executions')
    require(all(value[key] is False for key in ('compiler_modified', 'provider_modified', 'kernel_modified',
        'kfd_modified', 'policy_relaxation', 'live_sources_modified', 'gpu_execution')), 'unchanged lower layers')


def overlay_map(before, rows):
    expected = dict(before)
    for row in rows:
        key = 'ferric/' + row['path']
        require((key not in before) if row['before'] is None else before.get(key) == row['before'],
                'exact CPU1037 preimage or absent addition')
        expected[key] = row['after']
    require(len(expected) == len(before) + 6, 'six source additions only')
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
    require([tuple(row) for row in previous] == [(492, 0, 4), (13, 0, 0)] and added == 13,
            'actual CPU1037 library/shared_wire split and thirteen library additions')
    return [(505, 0, 4), (13, 0, 0)]


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
    omitted_tests = {name for name in prior['tests'] if name != 'worker-tests'
                     and not name.startswith(('parent-', 'ferric-'))}
    require(len(omitted_tests) == 28 and sum(prior['tests'][name]['passed'] for name in omitted_tests) == 208,
            'only unchanged historical runtime cohort omitted')
    omitted = omitted_tests | {'runtime-list'}
    recipes = {name: relocate(row, out) for name, row in previous.items() if name not in omitted}
    require(len(recipes) == 58, 'exact prior parent/worker phases')
    for name in ('rustfmt', 'rustfmt-check'):
        argv = recipes[name]['argv']
        recipes[name]['argv'] = argv[:argv.index('skip_children=true') + 1] + [
            str(out / 'sources/ferric' / row['path']) for row in rows if row['path'].endswith('.rs')]
    for suffix in ('list', 'tests'):
        row = copy.deepcopy(recipes[OLD_BIN + '-' + suffix])
        row['argv'] = [NEW_BIN if arg == OLD_BIN else arg for arg in row['argv']]
        recipes[NEW_BIN + '-' + suffix] = row
    report = copy.deepcopy(recipes['parent-projection-residual-decode-wire'])
    argv = report['argv']; argv[argv.index('--lib') + 1] = REPORT
    recipes['parent-projection-host-report'] = report
    argv = recipes['parent-builds']['argv']
    start, end = argv.index('--bin'), argv.index('--message-format=json')
    require(all(argv[i] == '--bin' for i in range(start, end, 2)), 'original selected binary build roster')
    recipes['parent-builds']['argv'] = argv[:start] + ['--bin', OLD_BIN, '--bin', NEW_BIN] + argv[end:]
    require(len(recipes) == 61, 'sixty-one exact bounded phases')
    return recipes, omitted_tests


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ,
            'ordinary bytecode-free controller')
    require(len(sys.argv) == 3, 'CONTROLLER_SHA FRESH_LABEL')
    controller_sha, label = sys.argv[1:]
    require(re.fullmatch('[0-9a-f]{64}', controller_sha)
            and re.fullmatch(r'projection-ar4-host-observation-cpu-v228-v[1-9][0-9]*', label), 'root source/fresh label')
    require(hashlib.sha256(HELPER.read_bytes()).hexdigest() == HELPER_SHA, 'pinned helper')
    h = types.ModuleType('qualification'); h.__file__ = str(HELPER)
    exec(compile(HELPER.read_bytes(), str(HELPER), 'exec'), h.__dict__)
    x = h.load(h.EXTRACTOR, h.EXTRACTOR_SHA, 'source_helpers')
    n = h.load(h.BOUNDS, h.BOUNDS_SHA, 'bounded')
    q = h.load(PRIOR_CONTROLLER, PRIOR_CONTROLLER_SHA, 'prior_controller')
    q.OLD_TARGETS = (*q.OLD_TARGETS, E / 'projection-residual-runtime-cpu-v228-v1/target',
        E / 'projection-residual-decode-cpu-v228-v1/target', OLD / 'target',
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
        row = value['binary']; require(x.pin(Path(row['path'])) == row, 'preserved original seventeen ELFs')
        pinned.append(row)
    require(prior['raw']['sources-after.json']['sha256'] == SOURCE_SHA, 'qualified formatted source identity')
    before = document(OLD / 'sources-after.json')
    require(x.snapshot(OLD / 'sources') == before, 'unchanged complete original paired source')
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
    require(x.snapshot(source) == expected, 'only seventeen reviewed source changes')
    cargo_delta(old_cargo, h.tomllib.loads(cargo.read_text()))
    n.save(out / 'sources-unformatted.json', expected)
    previous = {name: document(OLD / (name + '-command.json')) for name in prior['phases']}
    recipes, omitted = recipes_from(previous, prior, out, rows)
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
        require(len(allowed) == 16 and set(formatted) == set(expected)
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
        report_names = {name for name in additions if name.startswith(REPORT)}
        require(len(report_names) == 8 and not selected.intersection(report_names), 'separate shared report selection')
        test('parent-projection-host-report', report_names, [(8, 0, 0)]); selected.update(report_names)
        require(additions <= selected, 'all twelve new parent library tests executed')
        new_bin_names = set(proposal['added_tests']['parent_binary'])
        require(h.inventory(run(NEW_BIN + '-list')) == new_bin_names, 'new opt-in binary inventory')
        test(NEW_BIN + '-tests', new_bin_names, [(1, 0, 0)])
        binaries.update(h.artifacts(run('parent-builds'), [OLD_BIN, NEW_BIN], source / 'ferric' / PARENT / 'Cargo.toml',
                                    out / 'target/parent', x.pin))
        run('parent-default-check')
        require(set(phases) == set(recipes) and len(phases) == 61 and len(binaries) == 3
                and set(tests) == (set(prior['tests']) - omitted) | {'parent-projection-host-report', NEW_BIN + '-tests'},
                'all scoped recipes and regressions complete')
        require(sum(row['passed'] for row in tests.values()) == 855
                and sum(row['ignored'] for row in tests.values()) == 4, 'actual named scoped855/four old ignores')
    except BaseException as failure:
        error = repr(failure)
    finally:
        for label_, check in [
            ('sources', lambda: require(x.snapshot(source) == expected, 'candidate source/lock drift')),
            ('baseline-source', lambda: require(x.snapshot(OLD / 'sources') == before, 'original CPU1037 source drift')),
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
    result = dict(schema='ferric-p228-projection-ar4-host-observation-cpu-result-v1', passed=passed,
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
