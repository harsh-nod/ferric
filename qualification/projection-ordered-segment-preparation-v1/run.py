"""Qualify a caller-pinned paired runtime/Ferric overlay in a fresh bounded source copy."""
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
PACKAGE = E / 'p228-projection-ordered-segment-cpu-v1'
OLD = E / 'projection-ar4-shared-host-cpu-v228-v1'
OLD_SHA = 'deb2aeacded08c336d1fb5a1638cd2accc2b65ec65ffa3e90177bd5e61146d74'
SOURCE_SHA = 'ed98abb445b2dc3209fcd575bfb25e817b86b7bce0dbdb5f299984067149f3b4'
HELPER = E / 'p228-host-policy-cpu-v1/run.py'
HELPER_SHA = '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494'
PRIOR = E / 'p228-projection-residual-runtime-cpu-v1/run.py'
PRIOR_SHA = '6fc90b8cbdda07c52c761767c7c463a3ea0ebecb244c5ace7a0fc3e274ac2d94'
RUNTIME_PRIOR = E / 'gfx950-clock-cpu-v228-v1'
RUNTIME_PRIOR_SHA = '458732e9e0c67501f33584ff1c6c041f93c0b9021884b504c4a7dfb206610ad3'
PARENT = 'adapters/m1-engineering-execution-v1'
WORKER = 'adapters/tp-peer-finite-engineering-worker-v1'
OLD_BIN = 'ferric-qwen3-finite-projection-residual-decode-shared-host-engineering'
NAMES = re.compile('[A-Za-z0-9_:]+')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pairs(items):
    value = {}
    for key, item in items:
        require(key not in value, 'duplicate JSON key')
        value[key] = item
    return value


def document(path):
    require(path.resolve(strict=True) == path and path.is_file() and path.stat().st_size <= 16 << 20,
            'bounded canonical JSON')
    return json.loads(path.read_bytes(), object_pairs_hook=pairs)


def relocated(value, out):
    if isinstance(value, str):
        return value.replace(str(OLD), str(out))
    if isinstance(value, list):
        return [relocated(v, out) for v in value]
    if isinstance(value, dict):
        return {k: relocated(v, out) for k, v in value.items()}
    return value


def plan_shape(plan):
    require(set(plan) == {'schema', 'proposals', 'added_tests', 'parent_binary', 'runtime_full_suite_cpu_reviewed'}
        and plan['schema'] == 'ferric-p228-projection-ordered-segment-cpu-inputs-v1'
        and plan['runtime_full_suite_cpu_reviewed'] is True
        and set(plan['proposals']) == {'fe2o3', 'ferric'}, 'closed root-reviewed CPU plan')
    require(re.fullmatch('ferric-qwen3-finite-projection-residual-decode-[a-z-]+-engineering', plan['parent_binary'])
        and plan['parent_binary'] != OLD_BIN, 'distinct opt-in parent')
    groups = plan['added_tests']
    require(set(groups) == {'runtime', 'worker', 'parent_library', 'parent_binary'}, 'four compiled test inventories')
    for names in groups.values():
        require(type(names) is list and 0 < len(names) <= 180 and names == sorted(set(names))
            and all(NAMES.fullmatch(name) for name in names), 'bounded unique exact added test names')


def overlay_rows(project, proposal):
    rows = proposal['files']
    require(type(rows) is list and 0 < len(rows) <= 64 and len({r['path'] for r in rows}) == len(rows),
        'bounded nonempty unique source overlay')
    for row in rows:
        require(set(row) == {'path', 'source', 'before', 'after'}, 'closed source row')
        relative = Path(row['path'])
        require(not relative.is_absolute() and '..' not in relative.parts and str(relative) == row['path']
            and row['source'] == 'candidate/' + row['path'], 'canonical candidate relative path')
        if project == 'fe2o3':
            require(row['path'].startswith('crates/fe2o3-kfd/src/') and relative.suffix == '.rs', 'runtime Rust scope')
        else:
            require(row['path'].startswith((PARENT + '/', WORKER + '/'))
                and (relative.suffix == '.rs' or row['path'] == PARENT + '/Cargo.toml'), 'Ferric adapter scope')
        for pin in [row['after'], *([] if row['before'] is None else [row['before']])]:
            require(set(pin) == {'bytes', 'sha256'} and type(pin['bytes']) is int and 0 < pin['bytes'] <= 1 << 20
                and re.fullmatch('[0-9a-f]{64}', pin['sha256']), 'bounded source content pin')
    return rows


def extend(actual, previous, added):
    require(not previous.intersection(added) and actual == previous | set(added), 'exact additive compiled inventory')


def ignored(raw):
    return set(re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. ignored', raw, re.M))


def cargo_delta(before, after, binary):
    current = copy.deepcopy(after)
    row = dict(name=binary, path='src/bin/' + binary + '.rs', **{'required-features': ['tp-batch-engineering']})
    require(current['bin'].count(row) == 1 and all(b['name'] != binary for b in before['bin']), 'one additive Cargo target')
    current['bin'].remove(row)
    require(current == before, 'no dependency, profile or other Cargo changes')


def main():
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ,
        'ordinary bytecode-free controller')
    require(len(sys.argv) == 5, 'CONTROLLER_SHA PLAN_PATH PLAN_SHA FRESH_LABEL')
    own_sha, plan_path, plan_sha, label = sys.argv[1:]
    require(all(re.fullmatch('[0-9a-f]{64}', v) for v in (own_sha, plan_sha))
        and re.fullmatch(r'projection-ordered-segment-cpu-v228-v[1-9][0-9]*', label), 'actual pins and fresh bounded label')
    require(hashlib.sha256(HELPER.read_bytes()).hexdigest() == HELPER_SHA, 'retained helper')
    h = types.ModuleType('qualification'); h.__file__ = str(HELPER)
    exec(compile(HELPER.read_bytes(), str(HELPER), 'exec'), h.__dict__)
    x = h.load(h.EXTRACTOR, h.EXTRACTOR_SHA, 'source_helpers')
    n = h.load(h.BOUNDS, h.BOUNDS_SHA, 'bounded')
    q = h.load(PRIOR, PRIOR_SHA, 'prior_controller')
    controller, plan_path = Path(__file__).resolve(strict=True), Path(plan_path)
    require(controller == PACKAGE / 'run.py' and h.sha(controller) == own_sha
        and plan_path.parent == PACKAGE and plan_path.name == 'inputs.json' and h.sha(plan_path) == plan_sha,
        'root-selected controller and actual reviewed plan')
    plan = document(plan_path); plan_shape(plan)
    require(h.sha(OLD / 'complete.json') == OLD_SHA, 'actual CPU883 completion')
    prior = document(OLD / 'complete.json')
    require(prior['passed'] is True and prior['error'] is None and not prior['postcheck_errors']
        and prior['tests_passed'] == 883 and prior['tests_ignored'] == 4 and len(prior['phases']) == 63
        and len(prior['binaries']) == 4 and prior['source_unchanged'] is True, 'actual qualified baseline')
    pinned = [x.pin(p) for p in (controller, plan_path, HELPER, h.EXTRACTOR, h.BOUNDS, PRIOR, OLD / 'complete.json')]
    require(h.sha(RUNTIME_PRIOR / 'complete.json') == RUNTIME_PRIOR_SHA, 'actual unchanged-runtime inventory owner')
    old_runtime = document(RUNTIME_PRIOR / 'complete.json')
    runtime_inventory = old_runtime['raw']['runtime-list-stdout']
    require(old_runtime['passed'] is True and runtime_inventory == x.pin(RUNTIME_PRIOR / 'runtime-list-stdout'),
        'actual qualified compiled runtime inventory')
    pinned.extend((x.pin(RUNTIME_PRIOR / 'complete.json'), runtime_inventory))
    prior_runtime_names = h.inventory((RUNTIME_PRIOR / 'runtime-list-stdout').read_text())
    for name, row in prior['raw'].items():
        require(row['path'] == str(OLD / name) and x.pin(OLD / name) == row, 'all retained CPU883 raw bodies')
        pinned.append(row)
    for row in prior['binaries'].values():
        require(x.pin(Path(row['binary']['path'])) == row['binary'], 'old executable retained')
        pinned.append(row['binary'])
    before = document(OLD / 'sources-after.json')
    require(prior['raw']['sources-after.json']['sha256'] == SOURCE_SHA and len(before) == 7003
        and x.snapshot(OLD / 'sources') == before, 'actual complete qualified paired source')
    overlays = {}
    for project, pin in plan['proposals'].items():
        expected_dir = 'p228-projection-residual-mlp-ordered-' + ('runtime' if project == 'fe2o3' else 'ferric') + '-v1'
        path = E / expected_dir / 'source-manifest.json'
        require(pin == x.pin(path), 'root-pinned source proposal')
        pinned.append(pin)
        overlays[project] = (path.parent, overlay_rows(project, document(path)))
    out = E / label; out.mkdir(mode=0o700)
    source = out / 'sources'; source.mkdir()
    n.F, n.T = source / 'fe2o3', out / 'target'
    n.setup(); require(not any(n.T.iterdir()), 'fresh aggregate target')
    (out / 'tmp').mkdir()
    require(sum(p['bytes'] for p in before.values()) <= 256 << 20, 'bounded paired source copy')
    for name, pin in before.items():
        relative = Path(name)
        require(not relative.is_absolute() and '..' not in relative.parts
            and relative.parts[0] in ('fe2o3', 'ferric') and pin['bytes'] <= 16 << 20, 'closed source member')
        origin, target = OLD / 'sources' / relative, source / relative
        require(x.pin(origin) == dict(path=str(origin), **pin), 'source identity before copy')
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream: stream.write(origin.read_bytes())
        target.chmod(0o700 if origin.stat().st_mode & 0o111 else 0o600)
    require(x.snapshot(source) == before, 'byte-identical copied source')
    n.save(out / 'sources-base.json', before)
    expected = dict(before)
    cargo = source / 'ferric' / PARENT / 'Cargo.toml'
    old_cargo = h.tomllib.loads(cargo.read_text())
    for project, (root, rows) in overlays.items():
        for row in rows:
            key = project + '/' + row['path']
            require((key not in expected) if row['before'] is None else expected.get(key) == row['before'], 'exact preimage')
            body = root / row['source']; pin = x.pin(body)
            require({k: pin[k] for k in ('bytes', 'sha256')} == row['after'], 'exact candidate source')
            pinned.append(pin)
            target = source / key; target.parent.mkdir(parents=True, exist_ok=True)
            with target.open('xb' if row['before'] is None else 'wb') as stream: stream.write(body.read_bytes())
            expected[key] = row['after']
    require(x.snapshot(source) == expected, 'only reviewed source overlays')
    binary = plan['parent_binary']; cargo_delta(old_cargo, h.tomllib.loads(cargo.read_text()), binary)
    n.save(out / 'sources-unformatted.json', expected)
    recipes = {name: relocated(document(OLD / (name + '-command.json')), out) for name in prior['phases']}
    changed_rust = sorted(str(source / project / row['path']) for project, (_, rows) in overlays.items()
        for row in rows if row['path'].endswith('.rs'))
    for name in ('rustfmt', 'rustfmt-check'):
        argv = recipes[name]['argv']; recipes[name]['argv'] = argv[:argv.index('skip_children=true') + 1] + changed_rust
    for suffix in ('list', 'tests'):
        recipe = copy.deepcopy(recipes[OLD_BIN + '-' + suffix])
        recipe['argv'] = [binary if arg == OLD_BIN else arg for arg in recipe['argv']]
        recipes[binary + '-' + suffix] = recipe
    argv = recipes['parent-builds']['argv']; pos = argv.index('--message-format=json')
    argv[pos:pos] = ['--bin', binary]
    worker_recipe = recipes['worker-tests']
    cargo_tool = worker_recipe['argv'][0]
    runtime = [cargo_tool, 'test', '--offline', '--locked', '--jobs', '2', '--manifest-path',
        str(source / 'ferric' / WORKER / 'Cargo.toml'), '-p', 'fe2o3-kfd', '--lib']
    for name, tail in (('runtime-list', ['--', '--list', '--format', 'terse']),
                       ('runtime-tests', ['--', '--test-threads=2'])):
        recipe = copy.deepcopy(worker_recipe); recipe['argv'] = runtime + tail; recipes[name] = recipe
    selected_before = set().union(*(set(v['names']) for name, v in prior['tests'].items() if name.startswith('parent-')))
    old_library = h.inventory((OLD / 'parent-lib-list-stdout').read_text())
    additions = plan['added_tests']
    for i, name in enumerate(additions['parent_library']):
        if any(recipes[key]['argv'][recipes[key]['argv'].index('--lib') + 1] in name
                for key in prior['tests'] if key.startswith('parent-')):
            continue
        recipe = copy.deepcopy(recipes['parent-client']); argv = recipe['argv']
        argv[argv.index('--lib') + 1] = name; argv.append('--exact')
        recipes['parent-added-' + str(i)] = recipe
    require(len(recipes) <= 150, 'bounded command roster')
    q.OLD_TARGETS = (*q.OLD_TARGETS, E / 'projection-residual-runtime-cpu-v228-v1/target',
        E / 'projection-residual-decode-cpu-v228-v1/target', E / 'projection-ar4-cpu-v228-v1/target',
        E / 'projection-ar4-host-observation-cpu-v228-v1/target', OLD / 'target',
        E / 'rpo-compiler-cpu-v228-v2/target', E / 'kir-indexed-formal-join-cpu-v228-v2/target')
    old_targets = q.target_inventory(); n.save(out / 'old-targets-before.json', old_targets)
    config = q.configurations(source, R / 'toolchain/cargo', x); n.save(out / 'configurations.json', config)
    seen_tools = set()
    for recipe in recipes.values():
        for tool, sha in recipe['tools'].items():
            path = Path(recipe['env']['RUSTC']).parent / tool
            if (path, sha) not in seen_tools:
                require(h.sha(path) == sha, 'qualified compiler tools'); pinned.append(x.pin(path))
                seen_tools.add((path, sha))
    require(h.sha(Path(recipes['rustfmt']['argv'][0])) == q.RUSTFMT_SHA, 'qualified formatter')
    pinned.append(x.pin(Path(recipes['rustfmt']['argv'][0])))
    phases, tests, binaries, metadata = {}, {}, {}, {}
    error, post_errors = None, []

    def run(name):
        recipe = recipes[name]; n.N = Path(recipe['env']['RUSTC']).parent.parent; n.PINS = recipe['tools']
        n.F = source / ('fe2o3' if recipe['env']['CARGO_TARGET_DIR'].endswith('/worker') else 'ferric/' + PARENT)
        try: n.run(out, name, recipe['argv'], env=recipe['env'], deadline=recipe['deadline_seconds'])
        finally:
            if (out / (name + '-result.json')).is_file(): phases[name] = document(out / (name + '-result.json'))
        return (out / (name + '-stdout')).read_text()

    def test(name, names, summaries):
        tests[name] = h.results(run(name), names, summaries)

    try:
        run('rustfmt')
        formatted = x.snapshot(source); allowed = {str(Path(p).relative_to(source)) for p in changed_rust}
        require(set(formatted) == set(expected) and all(formatted[k] == v for k, v in expected.items() if k not in allowed), 'formatter scope')
        expected = formatted; n.save(out / 'sources-before.json', expected); run('rustfmt-check')
        for role, directory in (('worker', WORKER), ('parent', PARENT)):
            value = json.loads(run(role + '-metadata'))
            baseline = relocated(document(OLD / (role + '-metadata-stdout')), out)
            current = copy.deepcopy(value)
            if role == 'parent':
                package = next(p for p in current['packages'] if p['manifest_path'] == str(cargo))
                prior_package = next(p for p in baseline['packages'] if p['manifest_path'] == str(cargo))
                target = copy.deepcopy(next(t for t in prior_package['targets'] if t['name'] == OLD_BIN))
                target.update(name=binary, src_path=str(cargo.parent / 'src/bin' / (binary + '.rs')))
                require(package['targets'].count(target) == 1, 'new binary metadata'); package['targets'].remove(target)
            require(current == baseline, 'unchanged full dependency/feature metadata')
            metadata[role] = h.metadata_check(value, source, out / 'target' / role, role == 'worker',
                h.tomllib.loads((source / 'ferric' / directory / 'Cargo.lock').read_text()))
            require(metadata[role] == relocated(prior['metadata'][role], out), 'same resolved dependencies')
        runtime_names = h.inventory(run('runtime-list'))
        # The complete runtime suite is selected, not only the historical 208-test subset.
        extend(runtime_names, prior_runtime_names, additions['runtime'])
        runtime_ignored = {name for name in prior_runtime_names if name.rsplit('::', 1)[-1] in (
            'retained_fixed_image_passes_same_engine_intake',
            'actual_multiwave_image_passes_distinct_same_engine_intake',
            'real_gfx950_kernel_rejects_before_fixed_dispatch_data_preparation')}
        require(len(runtime_ignored) == 3, 'three reviewed CPU-only retained-image fixture gates')
        test('runtime-tests', runtime_names, [(len(runtime_names) - 3, 0, 3)])
        require(ignored((out / 'runtime-tests-stdout').read_text()) == runtime_ignored, 'exact historical runtime ignores')
        worker_names = h.inventory(run('worker-list'))
        extend(worker_names, set(prior['tests']['worker-tests']['names']), additions['worker'])
        old_summaries = [tuple(v) for v in prior['tests']['worker-tests']['summaries']]
        require(old_summaries == [(519, 0, 4), (13, 0, 0)], 'CPU883 worker suite split')
        test('worker-tests', worker_names, [(519 + len(additions['worker']), 0, 4), (13, 0, 0)])
        require(ignored((out / 'worker-tests-stdout').read_text()) == ignored((OLD / 'worker-tests-stdout').read_text()), 'same four historical ignores')
        binaries.update(h.artifacts(run('worker-build'), [h.CHILD_BIN], source / 'ferric' / WORKER / 'Cargo.toml', out / 'target/worker', x.pin))
        parent_names = h.inventory(run('parent-lib-list')); extend(parent_names, old_library, additions['parent_library'])
        selected = set()
        for name, previous in prior['tests'].items():
            if name.startswith('ferric-'):
                require(h.inventory(run(name.removesuffix('-tests') + '-list')) == set(previous['names']), 'same old binary tests')
                test(name, set(previous['names']), [tuple(v) for v in previous['summaries']])
            elif name.startswith('parent-'):
                argv = recipes[name]['argv']; selector = argv[argv.index('--lib') + 1]
                names = {v for v in parent_names if selector in v}
                require(names == set(previous['names']) | {v for v in additions['parent_library'] if selector in v}
                    and not selected.intersection(names), 'retained nonoverlapping parent selections')
                test(name, names, [(len(names), 0, 0)]); selected.update(names)
        for name in recipes:
            if name.startswith('parent-added-'):
                argv = recipes[name]['argv']; exact = argv[argv.index('--lib') + 1]
                require(exact not in selected and exact in parent_names, 'unique added exact parent test')
                test(name, {exact}, [(1, 0, 0)]); selected.add(exact)
        require(selected == selected_before | set(additions['parent_library']), 'every new and prior selected parent test ran')
        names = set(additions['parent_binary']); require(h.inventory(run(binary + '-list')) == names, 'new parent binary tests')
        test(binary + '-tests', names, [(len(names), 0, 0)])
        old_binaries = [name for name in prior['binaries'] if name != h.CHILD_BIN]
        binaries.update(h.artifacts(run('parent-builds'), old_binaries + [binary], cargo, out / 'target/parent', x.pin))
        run('parent-default-check')
        require(set(phases) == set(recipes) and len(binaries) == 5, 'all commands and five binaries completed')
        expected_passed = 883 + len(runtime_names) - 3 + sum(len(additions[k]) for k in ('worker', 'parent_library', 'parent_binary'))
        require(sum(v['passed'] for v in tests.values()) == expected_passed and sum(v['ignored'] for v in tests.values()) == 7,
            'named-test-derived full totals')
    except BaseException as failure:
        error = repr(failure)
    finally:
        checks = (
            ('sources', lambda: require(x.snapshot(source) == expected, 'candidate source/lock drift')),
            ('baseline-source', lambda: require(x.snapshot(OLD / 'sources') == before, 'old source drift')),
            ('old-targets', lambda: require(q.target_inventory() == old_targets, 'old target changed')),
            ('configurations', lambda: require(q.configurations(source, R / 'toolchain/cargo', x) == config, 'config drift')),
            ('inputs', lambda: require(all(x.pin(Path(p['path'])) == p for p in pinned), 'tool/input/old ELF drift')),
            ('dependencies', lambda: require(all(h.sha(Path(p['manifest'])) == p['manifest_sha256']
                for value in metadata.values() for p in value['external']), 'dependency drift')),
            ('binaries', lambda: require(all(x.pin(Path(p['binary']['path'])) == p['binary'] for p in binaries.values()), 'new ELF drift')),
            ('target', lambda: require(n.size(out / 'target') <= 6 << 30, 'aggregate target cap')))
        for name, check in checks:
            try: check()
            except BaseException as failure: post_errors.append(name + ': ' + repr(failure))
        n.save(out / 'sources-after.json', x.snapshot(source)); n.save(out / 'old-targets-after.json', q.target_inventory())
    passed = error is None and not post_errors
    result = dict(schema='ferric-p228-projection-ordered-segment-cpu-result-v1', passed=passed,
        error=error, postcheck_errors=post_errors, prior_completion=x.pin(OLD / 'complete.json'),
        controller=x.pin(controller), plan=x.pin(plan_path), inputs=pinned, phases=phases, tests=tests,
        binaries=binaries, metadata=metadata, declared_test_additions=additions,
        tests_passed=sum(v['passed'] for v in tests.values()), tests_ignored=sum(v['ignored'] for v in tests.values()),
        source_unchanged=not any(v.startswith('sources:') for v in post_errors), empty_initial_target=True,
        complete_runtime_library_suite_selected=True, compiler_requalified=False, gpu_execution=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False,
        raw={p.name: x.pin(p) for p in out.iterdir() if p.is_file()})
    path = out / ('complete.json' if passed else 'failed.json'); n.save(path, result)
    print(json.dumps(dict(passed=passed, receipt=x.pin(path), error=error, postcheck_errors=post_errors)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
