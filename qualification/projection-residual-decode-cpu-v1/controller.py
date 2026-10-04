"""Fresh-source TF4 residual-route CPU qualification, using retained bounded tools."""
import json
import os
from pathlib import Path
import re
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
OLD = E / 'projection-residual-runtime-cpu-v228-v1'
OLD_SHA = 'bf1a12f78981d9ff9b8157e1dec6dca300752b238680e16380b98e9d1260bafb'
HELPER = E / 'p228-host-policy-cpu-v1/run.py'
HELPER_SHA = '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494'
PRIOR_CONTROLLER = E / 'p228-projection-residual-runtime-cpu-v1/run.py'
PRIOR_CONTROLLER_SHA = '6fc90b8cbdda07c52c761767c7c463a3ea0ebecb244c5ace7a0fc3e274ac2d94'
PARENT = 'adapters/m1-engineering-execution-v1'
WORKER = 'adapters/tp-peer-finite-engineering-worker-v1'
NEW_BIN = 'ferric-qwen3-finite-projection-residual-decode-engineering'
OLD_BIN = 'ferric-qwen3-finite-projection-residual-layer-capture-engineering'
WIRE = 'finite_projection_residual_decode_wire_v1::tests::'
OLD_WIRE = 'finite_projection_residual_layer_wire_v1::tests::'


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
    require(path.resolve(strict=True) == path and path.stat().st_size <= 16 << 20,
            'canonical bounded JSON')
    return json.loads(path.read_bytes(), object_pairs_hook=pairs)


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ,
            'ordinary bytecode-free controller')
    require(len(sys.argv) == 5, 'CONTROLLER_SHA INPUTS_PATH INPUTS_SHA FRESH_LABEL')
    controller_sha, input_name, input_sha, label = sys.argv[1:]
    require(re.fullmatch('projection-residual-decode-cpu-v228-v[1-9][0-9]*', label), 'fresh label')
    import hashlib
    require(hashlib.sha256(HELPER.read_bytes()).hexdigest() == HELPER_SHA, 'pinned helper')
    h = types.ModuleType('qualification'); h.__file__ = str(HELPER)
    exec(compile(HELPER.read_bytes(), str(HELPER), 'exec'), h.__dict__)
    x = h.load(h.EXTRACTOR, h.EXTRACTOR_SHA, 'source_helpers')
    n = h.load(h.BOUNDS, h.BOUNDS_SHA, 'bounded')
    q = h.load(PRIOR_CONTROLLER, PRIOR_CONTROLLER_SHA, 'prior_controller')
    q.OLD_TARGETS = (*q.OLD_TARGETS, OLD / 'target')
    controller = Path(__file__).resolve(strict=True)
    require(controller == E / 'p228-projection-residual-decode-cpu-v1/run.py'
            and h.sha(controller) == controller_sha, 'selected controller')
    inputs_path = Path(input_name)
    require(inputs_path.parent == E
            and re.fullmatch('projection-residual-decode-cpu-inputs-v228-v[1-9][0-9]*\\.json', inputs_path.name)
            and h.sha(inputs_path) == input_sha, 'selected inputs')
    inputs = document(inputs_path)
    require(inputs['schema'] == 'ferric-projection-residual-decode-cpu-inputs-v1'
            and set(inputs['archives']) == {'ferric', 'fe2o3'}
            and set(inputs['proposals']) == {'worker', 'parent'}, 'input schema')
    require(h.sha(OLD / 'complete.json') == OLD_SHA, 'qualified prior completion')
    prior = document(OLD / 'complete.json')
    require(prior['passed'] is True and prior['error'] is None and not prior['postcheck_errors']
            and prior['tests_passed'] == 988 and prior['tests_ignored'] == 4, 'prior successful suite')
    pinned = [x.pin(p) for p in [controller, inputs_path, HELPER, h.EXTRACTOR,
              h.BOUNDS, PRIOR_CONTROLLER, OLD / 'complete.json']]
    for name, row in prior['raw'].items():
        require(Path(row['path']) == OLD / name and x.pin(OLD / name) == row, 'prior raw bytes')
        pinned.append(row)
    out = E / label
    out.mkdir(mode=0o700)
    source = out / 'sources'; source.mkdir()
    n.F, n.T = source / 'fe2o3', out / 'target'
    n.setup()
    require(not any(n.T.iterdir()), 'fresh private aggregate target')
    (out / 'tmp').mkdir()
    for project, row in inputs['archives'].items():
        path = Path(row['path'])
        require(path.parent == E and x.pin(path) == row, 'selected source archive')
        pinned.append(row)
        x.extract(path, project, source)
    before = x.snapshot(source)
    expected_base = document(OLD / 'sources-after.json')
    require(set(before) == set(expected_base), 'unchanged base file census')
    changed = {k: dict(before=expected_base[k], after=v) for k, v in before.items() if v != expected_base[k]}
    require(changed == inputs['documentation_changes']
            and all(k.endswith('.md') for k in changed), 'only declared documentation base changes')
    n.save(out / 'sources-base.json', before)
    rows, proposals, destinations = [], {}, set()
    for role, manifest_pin in inputs['proposals'].items():
        path = Path(manifest_pin['path'])
        require(path.parent.parent == E and x.pin(path) == manifest_pin, 'selected proposal')
        pinned.append(manifest_pin)
        proposal = document(path); proposals[role] = proposal
        require(proposal['base_commit'] == 'da9f613224b81232776ebb37ad8a1b903dc0488a', 'proposal base')
        for row in proposal['files']:
            rel = Path(row['path'])
            require(not rel.is_absolute() and '..' not in rel.parts and str(rel) == row['path']
                    and row['path'].startswith((WORKER if role == 'worker' else PARENT) + '/')
                    and row['path'] not in destinations
                    and (rel.suffix == '.rs' or row['path'] == PARENT + '/Cargo.toml'),
                    'disjoint bounded source destination')
            require(row['source'] == 'draft/' + row['path'], 'proposal source path')
            body = path.parent / row['source']
            require(body.resolve(strict=True) == body and {k: x.pin(body)[k] for k in ('bytes', 'sha256')} == row['after']
                    and before.get('ferric/' + row['path']) == row['before'], 'exact source preimage and body')
            pinned.append(x.pin(body)); destinations.add(row['path']); rows.append((row, body))
    expected = dict(before)
    for row, body in rows:
        path = source / 'ferric' / row['path']; path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb' if row['before'] is None else 'wb') as stream:
            stream.write(body.read_bytes())
        expected['ferric/' + row['path']] = row['after']
    require(x.snapshot(source) == expected, 'exact source-only overlay')
    n.save(out / 'sources-unformatted.json', expected)
    recipes = {}
    for name in prior['phases']:
        recipe = document(OLD / (name + '-command.json'))
        recipe['argv'] = [v.replace(str(OLD), str(out)) for v in recipe['argv']]
        recipe['env'] = {k: v.replace(str(OLD), str(out)) for k, v in recipe['env'].items()}
        recipes[name] = recipe
    fmt_files = [str(source / 'ferric' / row['path']) for row, _ in rows if row['path'].endswith('.rs')]
    for name in ['rustfmt', 'rustfmt-check']:
        argv = recipes[name]['argv']
        recipes[name]['argv'] = argv[:argv.index('skip_children=true') + 1] + fmt_files
    require(h.sha(Path(recipes['rustfmt']['argv'][0])) == q.RUSTFMT_SHA, 'qualified formatter')
    pinned.append(x.pin(Path(recipes['rustfmt']['argv'][0])))
    for suffix in ['list', 'tests']:
        recipe = json.loads(json.dumps(recipes[OLD_BIN + '-' + suffix]))
        recipe['argv'] = [v.replace(OLD_BIN, NEW_BIN) for v in recipe['argv']]
        recipes[NEW_BIN + '-' + suffix] = recipe
    recipe = json.loads(json.dumps(recipes['parent-projection-residual-wire']))
    recipe['argv'] = [WIRE if v == OLD_WIRE else v for v in recipe['argv']]
    recipes['parent-projection-residual-decode-wire'] = recipe
    build = recipes['parent-builds']['argv']
    build[build.index('--message-format=json'):build.index('--message-format=json')] = ['--bin', NEW_BIN]
    old_targets = q.target_inventory(); n.save(out / 'old-targets-before.json', old_targets)
    config = q.configurations(source, R / 'toolchain/cargo', x); n.save(out / 'configurations.json', config)
    tools_seen = set()
    for recipe in recipes.values():
        for tool, sha in recipe['tools'].items():
            path = Path(recipe['env']['RUSTC']).parent / tool
            require(h.sha(path) == sha, 'qualified toolchain')
            if path not in tools_seen:
                pinned.append(x.pin(path)); tools_seen.add(path)
    phases, tests, binaries, metadata = {}, {}, {}, {}
    error, post_errors = None, []

    def run(name):
        recipe = recipes[name]
        n.N = Path(recipe['env']['RUSTC']).parent.parent; n.PINS = recipe['tools']
        n.F = source / ('fe2o3' if recipe['env']['CARGO_TARGET_DIR'].endswith('/worker') else 'ferric/' + PARENT)
        phases[name] = n.run(out, name, recipe['argv'], env=recipe['env'], deadline=recipe['deadline_seconds'])
        return (out / (name + '-stdout')).read_text()

    def test(name, names, summaries):
        tests[name] = h.results(run(name), names, summaries)

    try:
        run('rustfmt')
        formatted = x.snapshot(source)
        allowed = {'ferric/' + row['path'] for row, _ in rows if row['path'].endswith('.rs')}
        require(set(formatted) == set(expected) and all(formatted[k] == v for k, v in expected.items() if k not in allowed),
                'formatter changed only selected Rust files')
        expected = formatted; n.save(out / 'sources-before.json', expected)
        run('rustfmt-check')
        for role, directory in [('worker', WORKER), ('parent', PARENT)]:
            manifest = source / 'ferric' / directory / 'Cargo.toml'
            metadata[role] = h.metadata_check(json.loads(run(role + '-metadata')), source, out / 'target' / role,
                role == 'worker', h.tomllib.loads(manifest.with_name('Cargo.lock').read_text()))
        runtime_names = h.inventory(run('runtime-list'))
        require(runtime_names == h.inventory((OLD / 'runtime-list-stdout').read_text()), 'unchanged runtime inventory')
        for name, previous in prior['tests']['runtime'].items():
            test(name, set(previous['names']), [tuple(v) for v in previous['summaries']])
        worker_names = h.inventory(run('worker-list'))
        old_names = set(prior['tests']['worker']['names'])
        added_worker = worker_names - old_names
        declared = proposals['worker']['authored_tests']
        require(old_names <= worker_names and declared['executed'] is False
                and len(added_worker) == declared['count'] == len(set(declared['names']))
                and {v.rsplit('::', 1)[-1] for v in added_worker} == set(declared['names']), 'exact worker additions')
        summaries = [tuple(v) for v in prior['tests']['worker']['summaries']]
        summaries[0] = (summaries[0][0] + len(added_worker), 0, summaries[0][2])
        test('worker-tests', worker_names, summaries)
        ignored = lambda raw: set(re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. ignored', raw, re.M))
        require(ignored((out / 'worker-tests-stdout').read_text()) == ignored((OLD / 'worker-tests-stdout').read_text()),
                'unchanged ignored test identities')
        binaries.update(h.artifacts(run('worker-build'), [h.CHILD_BIN], source / 'ferric' / WORKER / 'Cargo.toml', out / 'target/worker', x.pin))
        parent_names = h.inventory(run('parent-lib-list'))
        direct = set(proposals['parent']['test_census']['library'])
        wire_names = {v for v in added_worker if v.startswith(WIRE)}
        require(wire_names and parent_names == h.inventory((OLD / 'parent-lib-list-stdout').read_text()) | direct | wire_names,
                'exact parent additions')
        parent_bins = [name for name in prior['binaries'] if name != h.CHILD_BIN]
        for name, previous in prior['tests']['parent'].items():
            if name in parent_bins:
                require(h.inventory(run(name + '-list')) == set(previous['names']), 'unchanged binary test census')
                test(name + '-tests', set(previous['names']), [tuple(v) for v in previous['summaries']])
            else:
                selector = recipes[name]['argv'][recipes[name]['argv'].index('--lib') + 1]
                names = {v for v in parent_names if selector in v}
                require(set(previous['names']) <= names and names - set(previous['names']) <= direct, 'parent regression retained')
                test(name, names, [(len(names), 0, 0)])
        test('parent-projection-residual-decode-wire', wire_names, [(len(wire_names), 0, 0)])
        new_bin_names = set(proposals['parent']['test_census']['binary'])
        require(h.inventory(run(NEW_BIN + '-list')) == new_bin_names, 'new binary census')
        test(NEW_BIN + '-tests', new_bin_names, [(len(new_bin_names), 0, 0)])
        require(direct | wire_names <= {v for t in tests.values() for v in t['names']}, 'every new parent test executed')
        binaries.update(h.artifacts(run('parent-builds'), [*parent_bins, NEW_BIN], source / 'ferric' / PARENT / 'Cargo.toml', out / 'target/parent', x.pin))
        run('parent-default-check')
        require(set(phases) == set(recipes), 'every declared phase completed')
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
    result = dict(schema='ferric-projection-residual-decode-cpu-result-v1', passed=passed,
        error=error, postcheck_errors=post_errors, prior_completion=x.pin(OLD / 'complete.json'),
        controller=x.pin(controller), inputs=pinned, metadata=metadata, phases=phases, tests=tests,
        binaries=binaries, tests_passed=sum(v['passed'] for v in tests.values()),
        tests_ignored=sum(v['ignored'] for v in tests.values()), source_unchanged=not any(v.startswith('sources:') for v in post_errors),
        empty_initial_target=True, gpu_execution=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False,
        raw={p.name: x.pin(p) for p in out.iterdir() if p.is_file()})
    path = out / ('complete.json' if passed else 'failed.json'); n.save(path, result)
    print(json.dumps(dict(passed=passed, receipt=x.pin(path), error=error, postcheck_errors=post_errors)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
