"""Fresh AR4 source qualification using the actual CPU1022 recipes and bounds."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
OLD = E / 'projection-residual-decode-cpu-v228-v1'
OLD_SHA = '1f4365064da1a0884385035d1f2d280e6bba66afc2b5bb4265bf7d60b1418c83'
OLD_CONTROLLER = E / 'p228-projection-residual-decode-cpu-v1/run.py'
OLD_CONTROLLER_SHA = '6dac9b6fef7159d6906eab6582963f4931d7337b311ad5f0d30d1535455fe400'
HELPER = E / 'p228-host-policy-cpu-v1/run.py'
HELPER_SHA = '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494'
PRIOR_CONTROLLER = E / 'p228-projection-residual-runtime-cpu-v1/run.py'
PRIOR_CONTROLLER_SHA = '6fc90b8cbdda07c52c761767c7c463a3ea0ebecb244c5ace7a0fc3e274ac2d94'
PROPOSAL = E / 'p228-projection-ar4-runtime-v1/source-manifest.json'
PROPOSAL_SHA = '58ae10d372b552190a6c811e1a0e53591171a6d8682cafd3bca55279d2eb3d7f'
PARENT = 'adapters/m1-engineering-execution-v1'
WORKER = 'adapters/tp-peer-finite-engineering-worker-v1'
WIRE_FILE = WORKER + '/src/finite_projection_residual_decode_wire_v1_tests.rs'
FILES = {
    PARENT + '/src/tp_finite_client/prefix_decode/projection.rs',
    PARENT + '/src/tp_finite_client/prefix_decode/projection_tests.rs',
    WORKER + '/src/finite_projection_residual_decode_wire_v1.rs', WIRE_FILE,
    WORKER + '/src/native_projection_residual_decode_cli_v1.rs',
    WORKER + '/src/native_projection_residual_decode_cli_v1_tests.rs',
    WORKER + '/src/native_prefix_tiles_decode_v6.rs',
    WORKER + '/src/native_prefix_tiles_decode_v6/projection_tests.rs',
    WORKER + '/src/native_prefix_decode_projection_tests.rs',
}
TEST_FILES = {
    PARENT + '/src/tp_finite_client/prefix_decode/projection_tests.rs': 3,
    WIRE_FILE: 2,
    WORKER + '/src/native_projection_residual_decode_cli_v1_tests.rs': 2,
    WORKER + '/src/native_prefix_tiles_decode_v6/projection_tests.rs': 3,
    WORKER + '/src/native_prefix_decode_projection_tests.rs': 3,
}


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


def input_shape(value):
    require(set(value) == {'schema', 'archives', 'proposal', 'documentation_changes'}
        and value['schema'] == 'ferric-projection-ar4-cpu-inputs-v1'
        and set(value['archives']) == {'ferric', 'fe2o3'}
        and value['proposal']['path'] == str(PROPOSAL)
        and value['proposal']['sha256'] == PROPOSAL_SHA, 'closed selected input/proposal')


def source_base(before, expected, changes):
    require(set(before) == set(expected), 'unchanged CPU1022 base file census')
    actual = {key: dict(before=expected[key], after=row) for key, row in before.items() if row != expected[key]}
    require(actual == changes and all(key.endswith('.md') for key in actual),
            'only explicit noncompiled documentation changes')


def proposal_shape(value):
    require(value['schema'] == 'ferric-p228-projection-ar4-runtime-source-proposal-v1'
        and value['base_commit'] == '324d6fe0631a44f8a4d5c1b5c5d9fac794612929'
        and len(value['files']) == 9 and {row['path'] for row in value['files']} == FILES,
        'exact reviewed nine-source AR4 overlay')
    for row in value['files']:
        require(row['source'] == 'draft/' + row['path'] and row['before'] is not None
            and set(row['before']) == set(row['after']) == {'bytes', 'sha256'}, 'replacement source/preimage')
    require(set(value['added_tests']) == set(TEST_FILES) and value['authored_added_tests'] == 13
        and value['added_worker_test_executions'] == 10 and value['added_parent_test_executions'] == 5
        and value['tests_executed'] is False and len(value['renamed_tests']) == 4,
        'thirteen authored methods, fifteen executions, four renames')
    for name, count in TEST_FILES.items():
        names = value['added_tests'][name]
        require(len(names) == len(set(names)) == count
            and all(re.fullmatch(r'[A-Za-z0-9_:]+', n) for n in names), 'exact named additions per source')
    for row in value['renamed_tests']:
        require(row['path'] in TEST_FILES and row['shared_in_parent'] is (row['path'] == WIRE_FILE)
            and row['from'] != row['to'], 'explicit local/shared rename')
    for role in ('parent', 'worker'):
        added, renamed = test_delta(value, role)
        require(len(added) == (5 if role == 'parent' else 10)
            and len(renamed) == (2 if role == 'parent' else 3)
            and len(set(renamed.values())) == len(renamed)
            and not added.intersection(renamed) and not added.intersection(renamed.values()),
            'disjoint additive/renamed test identities')


def test_delta(proposal, role):
    require(role in ('parent', 'worker'), 'known test role')
    selected = lambda path: path.startswith((PARENT if role == 'parent' else WORKER) + '/') or (
        role == 'parent' and path == WIRE_FILE)
    added = {name for path, names in proposal['added_tests'].items() if selected(path) for name in names}
    renamed = {row['from']: row['to'] for row in proposal['renamed_tests'] if selected(row['path'])}
    return added, renamed


def extended_inventory(prior, actual, proposal, role):
    added, renamed = test_delta(proposal, role)
    require(set(renamed) <= prior and not set(renamed.values()).intersection(prior)
        and not added.intersection(prior), 'renames originate in actual baseline and additions are new')
    expected = {renamed.get(name, name) for name in prior} | added
    require(actual == expected, 'no unexplained test deletion/addition/rename')
    declared = proposal['test_census'][role]
    require(len(declared) == len(set(declared)), 'unique affected-module census')
    prefixes = {name.rsplit('::', 1)[0] + '::' for name in declared}
    require({name for name in actual if any(name.startswith(prefix) for prefix in prefixes)} == set(declared),
            'exact affected module inventory')
    return added, renamed


def relocate(value, old, new):
    if isinstance(value, str):
        return value.replace(str(old), str(new))
    if isinstance(value, list):
        return [relocate(item, old, new) for item in value]
    if isinstance(value, dict):
        return {key: relocate(item, old, new) for key, item in value.items()}
    return value


def selected_names(previous, actual, added, renamed, selector):
    expected = {renamed.get(name, name) for name in previous} | {name for name in added if selector in name}
    require({name for name in actual if selector in name} == expected, 'same selected regressions plus declared additions')
    return expected


def prior_contract(value):
    require(value['schema'] == 'ferric-projection-residual-decode-cpu-result-v1'
        and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
        and value['source_unchanged'] is True and value['tests_passed'] == 1022 and value['tests_ignored'] == 4
        and len(value['phases']) == 87 and len(value['raw']) == 442 and len(value['binaries']) == 17,
        'actual successful CPU1022 baseline')
    require(all(type(row['exit_code']) is int and row['exit_code'] == 0 and row['reason'] is None
        and row['group_absent'] is True for row in value['phases'].values()), 'natural baseline phases')


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ,
            'ordinary bytecode-free controller')
    require(len(sys.argv) == 5, 'CONTROLLER_SHA INPUTS_PATH INPUTS_SHA FRESH_LABEL')
    controller_sha, input_name, input_sha, label = sys.argv[1:]
    require(re.fullmatch('projection-ar4-cpu-v228-v[1-9][0-9]*', label), 'fresh label')
    require(hashlib.sha256(HELPER.read_bytes()).hexdigest() == HELPER_SHA, 'pinned helper')
    h = types.ModuleType('qualification'); h.__file__ = str(HELPER)
    exec(compile(HELPER.read_bytes(), str(HELPER), 'exec'), h.__dict__)
    x = h.load(h.EXTRACTOR, h.EXTRACTOR_SHA, 'source_helpers')
    n = h.load(h.BOUNDS, h.BOUNDS_SHA, 'bounded')
    q = h.load(PRIOR_CONTROLLER, PRIOR_CONTROLLER_SHA, 'prior_controller')
    q.OLD_TARGETS = (*q.OLD_TARGETS, E / 'projection-residual-runtime-cpu-v228-v1/target', OLD / 'target')
    controller = Path(__file__).resolve(strict=True)
    require(controller == E / 'p228-projection-ar4-cpu-v1/run.py' and h.sha(controller) == controller_sha,
            'selected controller')
    inputs_path = Path(input_name)
    require(inputs_path.parent == E and re.fullmatch(r'projection-ar4-cpu-inputs-v228-v[1-9][0-9]*\.json', inputs_path.name)
        and h.sha(inputs_path) == input_sha, 'selected inputs')
    inputs = document(inputs_path); input_shape(inputs)
    require(h.sha(OLD / 'complete.json') == OLD_SHA and h.sha(OLD_CONTROLLER) == OLD_CONTROLLER_SHA,
            'actual baseline completion/controller')
    prior = document(OLD / 'complete.json'); prior_contract(prior)
    pinned = [x.pin(path) for path in (controller, inputs_path, HELPER, h.EXTRACTOR, h.BOUNDS,
              PRIOR_CONTROLLER, OLD_CONTROLLER, OLD / 'complete.json')]
    for name, row in prior['raw'].items():
        require(row['path'] == str(OLD / name) and x.pin(OLD / name) == row, 'all actual baseline raw bytes')
        pinned.append(row)
    out = E / label; out.mkdir(mode=0o700)
    source = out / 'sources'; source.mkdir()
    n.F, n.T = source / 'fe2o3', out / 'target'
    n.setup(); require(not any(n.T.iterdir()), 'fresh private aggregate target')
    (out / 'tmp').mkdir()
    for project, row in inputs['archives'].items():
        path = Path(row['path'])
        require(path.parent == E and x.pin(path) == row, 'actual selected source archive')
        pinned.append(row); x.extract(path, project, source)
    before = x.snapshot(source)
    source_base(before, document(OLD / 'sources-after.json'), inputs['documentation_changes'])
    n.save(out / 'sources-base.json', before)
    require(x.pin(PROPOSAL) == inputs['proposal'], 'actual selected proposal body')
    pinned.append(inputs['proposal'])
    proposal = document(PROPOSAL); proposal_shape(proposal)
    rows, expected = [], dict(before)
    for row in proposal['files']:
        body = PROPOSAL.parent / row['source']
        record = x.pin(body)
        require(body.resolve(strict=True) == body and {key: record[key] for key in ('bytes', 'sha256')} == row['after']
            and before['ferric/' + row['path']] == row['before'], 'exact source body/preimage')
        pinned.append(record); rows.append((row, body))
    for row, body in rows:
        with (source / 'ferric' / row['path']).open('wb') as stream:
            stream.write(body.read_bytes())
        expected['ferric/' + row['path']] = row['after']
    require(x.snapshot(source) == expected, 'only nine reviewed source replacements')
    n.save(out / 'sources-unformatted.json', expected)
    recipes = {name: relocate(document(OLD / (name + '-command.json')), OLD, out) for name in prior['phases']}
    for name in ('rustfmt', 'rustfmt-check'):
        argv = recipes[name]['argv']
        recipes[name]['argv'] = argv[:argv.index('skip_children=true') + 1] + [str(source / 'ferric' / row['path']) for row, _ in rows]
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
        allowed = {'ferric/' + row['path'] for row, _ in rows}
        require(set(formatted) == set(expected) and all(formatted[key] == item for key, item in expected.items() if key not in allowed),
                'formatter changed only selected Rust files')
        expected = formatted; n.save(out / 'sources-before.json', expected)
        run('rustfmt-check')
        for role, directory in (('worker', WORKER), ('parent', PARENT)):
            manifest = source / 'ferric' / directory / 'Cargo.toml'
            metadata[role] = h.metadata_check(json.loads(run(role + '-metadata')), source, out / 'target' / role,
                role == 'worker', h.tomllib.loads(manifest.with_name('Cargo.lock').read_text()))
            require(metadata[role] == relocate(prior['metadata'][role], OLD, out), 'unchanged exact local and locked external dependencies')
        runtime_names = h.inventory(run('runtime-list'))
        require(runtime_names == h.inventory((OLD / 'runtime-list-stdout').read_text()), 'unchanged full runtime inventory')
        for name, previous in prior['tests'].items():
            if name != 'worker-tests' and not name.startswith(('parent-', 'ferric-')):
                test(name, set(previous['names']), [tuple(v) for v in previous['summaries']])
        worker_names = h.inventory(run('worker-list'))
        added_worker, renamed_worker = extended_inventory(set(prior['tests']['worker-tests']['names']), worker_names, proposal, 'worker')
        summaries = [tuple(v) for v in prior['tests']['worker-tests']['summaries']]
        summaries[0] = (summaries[0][0] + len(added_worker), 0, summaries[0][2])
        test('worker-tests', worker_names, summaries)
        ignored = lambda raw: set(re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. ignored', raw, re.M))
        require(ignored((out / 'worker-tests-stdout').read_text()) == ignored((OLD / 'worker-tests-stdout').read_text()),
                'unchanged four ignored identities; no new ignored test')
        binaries.update(h.artifacts(run('worker-build'), [h.CHILD_BIN], source / 'ferric' / WORKER / 'Cargo.toml', out / 'target/worker', x.pin))
        parent_names = h.inventory(run('parent-lib-list'))
        added_parent, renamed_parent = extended_inventory(h.inventory((OLD / 'parent-lib-list-stdout').read_text()), parent_names, proposal, 'parent')
        parent_bins = [name for name in prior['binaries'] if name != h.CHILD_BIN]
        for name, previous in prior['tests'].items():
            if name.startswith('ferric-'):
                binary = name.removesuffix('-tests')
                require(binary in parent_bins and h.inventory(run(binary + '-list')) == set(previous['names']),
                        'unchanged binary test inventory')
                test(name, set(previous['names']), [tuple(v) for v in previous['summaries']])
            elif name.startswith('parent-'):
                argv = recipes[name]['argv']; selector = argv[argv.index('--lib') + 1]
                names = selected_names(set(previous['names']), parent_names, added_parent, renamed_parent, selector)
                test(name, names, [(len(names), 0, 0)])
        require(added_parent <= {name for row in tests.values() for name in row['names']}, 'all five new parent executions completed')
        binaries.update(h.artifacts(run('parent-builds'), parent_bins, source / 'ferric' / PARENT / 'Cargo.toml', out / 'target/parent', x.pin))
        run('parent-default-check')
        require(set(phases) == set(recipes) and set(tests) == set(prior['tests']) and len(binaries) == 17,
                'all original phases, test selections and binaries retained')
        require(sum(row['passed'] for row in tests.values()) == 1037 and sum(row['ignored'] for row in tests.values()) == 4,
                'actual named 1022 plus fifteen passing executions')
    except BaseException as failure:
        error = repr(failure)
    finally:
        for label_, check in [
            ('sources', lambda: require(x.snapshot(source) == expected, 'source/lock drift')),
            ('old-targets', lambda: require(q.target_inventory() == old_targets, 'old cache modified')),
            ('configurations', lambda: require(q.configurations(source, R / 'toolchain/cargo', x) == config, 'config drift')),
            ('inputs', lambda: require(all(x.pin(Path(v['path'])) == v for v in pinned), 'input drift')),
            ('dependencies', lambda: require(all(h.sha(Path(v['manifest'])) == v['manifest_sha256'] for m in metadata.values() for v in m['external']), 'dependency drift')),
            ('binaries', lambda: require(all(x.pin(Path(v['binary']['path'])) == v['binary'] for v in binaries.values()), 'ELF drift')),
            ('target', lambda: require(n.size(out / 'target') <= 6 << 30, 'aggregate target cap')),
        ]:
            try:
                check()
            except BaseException as failure:
                post_errors.append(label_ + ': ' + repr(failure))
        n.save(out / 'sources-after.json', x.snapshot(source))
        n.save(out / 'old-targets-after.json', q.target_inventory())
    passed = error is None and not post_errors
    result = dict(schema='ferric-projection-ar4-cpu-result-v1', passed=passed,
        error=error, postcheck_errors=post_errors, prior_completion=x.pin(OLD / 'complete.json'),
        controller=x.pin(controller), proposal=inputs['proposal'], inputs=pinned,
        metadata=metadata, phases=phases, tests=tests, binaries=binaries,
        declared_test_additions=proposal['added_tests'], declared_test_renames=proposal['renamed_tests'],
        tests_passed=sum(v['passed'] for v in tests.values()), tests_ignored=sum(v['ignored'] for v in tests.values()),
        source_unchanged=not any(v.startswith('sources:') for v in post_errors), empty_initial_target=True,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False, production_authority=False,
        raw={path.name: x.pin(path) for path in out.iterdir() if path.is_file()})
    path = out / ('complete.json' if passed else 'failed.json'); n.save(path, result)
    print(json.dumps(dict(passed=passed, receipt=x.pin(path), error=error, postcheck_errors=post_errors)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
