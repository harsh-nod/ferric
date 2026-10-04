"""Joint fresh-source worker/parent qualification; no native or GPU execution."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
PACKAGE = E / 'p228-projection-residual-runtime-cpu-v1'
HELPER = E / 'p228-host-policy-cpu-v1/run.py'
HELPER_SHA = '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494'
WORKER_HELPER = E / 'p228-gfx950-clock-recorder-cpu-v1/run.py'
WORKER_HELPER_SHA = 'c496ad5041c4c235953d0057708d1799a6050d9e596d421146c1528cb831adeb'
PARENT_HELPER = E / 'p228-layer0-native-capture-cpu-v2/run.py'
PARENT_HELPER_SHA = 'fc55d1156a6b2b9eeab77760f59bc2f56e6c48201dcba0485b1d2e72b45a80ef'
PRIOR_WORKER = E / 'gfx950-clock-recorder-cpu-v228-v1'
PRIOR_PARENT = E / 'layer0-native-capture-cpu-v228-v2'
PRIOR_PINS = {
    'worker': dict(path=str(PRIOR_WORKER / 'complete.json'), bytes=213924,
        sha256='41ddcf8a9cb48b970d4f9a4187de6f6de1fd0506ba098d524e82d17335030f1d'),
    'parent': dict(path=str(PRIOR_PARENT / 'complete.json'), bytes=326552,
        sha256='78c12f822e95d50a1239f56411c5b8da9411a3fbda7510fbbf38d5644e1dffbb'),
}
PROPOSALS = {'worker': ('p228-projection-residual-worker-v1',
    'b3278f3bc86dcee8ef03b8d23d4456280d5b4b7502294549faf3ae4aa7f71049'),
    'parent': ('p228-projection-residual-parent-v1',
    'a415fef2075cfe00c7402b18cf87413a212f43bbdc1672c6e788bd77ebe9772d')}
PARENT = 'adapters/m1-engineering-execution-v1/'
WORKER = 'adapters/tp-peer-finite-engineering-worker-v1/'
NEW_BIN = 'ferric-qwen3-finite-projection-residual-layer-capture-engineering'
WIRE = 'finite_projection_residual_layer_wire_v1::tests::'
DIRECT = 'tp_finite_client::prefix_layer::projection_capture::tests::'
RUSTFMT_SHA = '30de9e1efcd8f8fe7750e00d0c45ff8f4c480608ef1be5baf9ab6f1b4556e8f8'
SOURCE_MANIFEST = E / 'projection-residual-runtime-source-inputs-v228-v1.json'
OLD_TARGETS = (PRIOR_WORKER / 'target', PRIOR_PARENT / 'target',
    E / 'gfx950-clock-parent-cpu-v228-v1/target', E / 'projection-residual-cpu-v228-v1/target')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pairs(rows):
    value = {}
    for key, item in rows:
        require(key not in value, 'duplicate JSON key')
        value[key] = item
    return value


def document(path):
    require(path.resolve(strict=True) == path and path.is_file()
            and path.stat().st_size <= 16 << 20, 'bounded canonical JSON')
    return json.loads(path.read_bytes(), object_pairs_hook=pairs)


def pin_shape(value):
    require(isinstance(value, dict) and set(value) == {'path', 'bytes', 'sha256'}, 'FilePin fields')
    require(type(value['bytes']) is int and value['bytes'] >= 0
            and isinstance(value['sha256'], str) and re.fullmatch('[0-9a-f]{64}', value['sha256']), 'FilePin content')
    path = Path(value['path'])
    require(path.is_absolute() and '..' not in path.parts and str(path) == value['path'], 'FilePin path')
    return value


def controller_input(sha):
    path = PACKAGE / 'run.py'
    require(re.fullmatch('[0-9a-f]{64}', sha) and Path(__file__).resolve(strict=True) == path
            and hashlib.sha256(path.read_bytes()).hexdigest() == sha, 'exact root-selected controller')
    return path


def pruned_source(value):
    require(isinstance(value, dict) and value, 'qualified source inventory')
    return {key: pin for key, pin in value.items() if not key.startswith('ferric/qualification/')}


def source_shape(value):
    require(set(value) == {'schema', 'qualified_parent', 'archives'}
            and value['schema'] == 'ferric-p228-projection-residual-runtime-sources-v1'
            and value['qualified_parent'] == PRIOR_PINS['parent']
            and set(value['archives']) == {'ferric', 'fe2o3'}, 'qualified paired source identity')
    for project, record in value['archives'].items():
        pin_shape(record)
        expected = ('projection-runtime-ferric-qualified-v228-v1.tar.gz' if project == 'ferric'
                    else 'clock-recorder-fe2o3-27b53d2b-v228-v1.tar.gz')
        require(Path(record['path']) == E / expected,
                'closed root-supplied source archive path')
    return value


def proposal_shape(role, value):
    require(value['base_commit'] == 'ee1aac9221a2fda8bdb88a58fc86b1700d682cb7', 'proposal source generation')
    rows = value['files']
    require(len(rows) == (15 if role == 'worker' else 7), 'exact role source census')
    require(sum(r['before'] is None for r in rows) == (6 if role == 'worker' else 3), 'exact new source census')
    seen = set()
    for row in rows:
        require(set(row) == {'path', 'source', 'before', 'after'}, 'proposal row fields')
        relative = Path(row['path'])
        require(not relative.is_absolute() and '..' not in relative.parts
                and relative.as_posix() == row['path'] and row['path'] not in seen
                and row['path'].startswith(WORKER if role == 'worker' else PARENT), 'bounded unique role destination')
        require(row['source'] == 'draft/' + row['path']
                and (relative.suffix == '.rs' or row['path'] == PARENT + 'Cargo.toml'), 'closed source kind')
        for pin in [row['after'], *([] if row['before'] is None else [row['before']])]:
            require(set(pin) == {'bytes', 'sha256'} and type(pin['bytes']) is int and pin['bytes'] > 0
                    and re.fullmatch('[0-9a-f]{64}', pin['sha256']), 'source content pin')
        seen.add(row['path'])
    return rows


def worker_additions(value):
    names = value['authored_tests']['names']
    require(value['authored_tests']['count'] == len(names) == 16
            and len(set(names)) == len(names) and value['authored_tests']['executed'] is False,
            'sixteen declared unexecuted worker tests')
    return set(names)


def extend_worker(actual, prior, short_names):
    require(prior <= actual, 'old worker regression removed')
    additions = actual - prior
    require(len(additions) == len(short_names) and {v.rsplit('::', 1)[-1] for v in additions} == short_names,
            'actual full inventory extends exactly declared worker tests')
    require(all(v.startswith(('finite_projection_residual_layer_wire_v1::tests::',
        'native_projection_residual_layer_cli_v1::tests::', 'native_prefix_layer_cli_v1::tests::',
        'native_catalog::forward::prefix_tiles_layer_v6::tests::',
        'resident_layer::prefix_tiles_v6::projection_residual::tests::')) for v in additions),
        'new worker test module routing')
    return additions


def parent_selections(actual, prior, direct, wire, old_selectors):
    additions = direct | wire
    require(not prior.intersection(additions) and actual == prior | additions, 'exact parent full library extension')
    selections, seen = {}, set()
    for name, selector in [*old_selectors, ('parent-projection-residual-wire', WIRE)]:
        names = {v for v in actual if selector in v}
        require(names and not seen.intersection(names), 'nonempty disjoint parent selections')
        selections[name] = names
        seen.update(names)
    require(additions <= seen, 'every new parent case is selected')
    return selections


def target_inventory():
    result = {}
    for root in OLD_TARGETS:
        require(root.resolve(strict=True) == root and root.is_dir(), 'original target retained')
        rows = {}
        for directory, dirs, files in os.walk(root, followlinks=False):
            for name in sorted(dirs + files):
                path = Path(directory) / name
                stat = path.lstat()
                rows[str(path.relative_to(root))] = [stat.st_mode, stat.st_ino, stat.st_size,
                    stat.st_mtime_ns, stat.st_ctime_ns, os.readlink(path) if path.is_symlink() else None]
        result[str(root)] = rows
    return result


def configurations(source, cargo_home, x):
    roots = [source / 'ferric' / PARENT, source / 'ferric' / WORKER, source / 'fe2o3']
    paths = {parent / '.cargo' / name for root in roots for parent in (root, *root.parents)
             for name in ('config', 'config.toml')}
    paths.update(cargo_home / name for name in ('config', 'config.toml'))
    return {str(path): (x.pin(path) if os.path.lexists(path) else None) for path in sorted(paths)}


def prior_receipt(role, commands, environment, tools, h, x, pinned):
    record = PRIOR_PINS[role]
    require(x.pin(Path(record['path'])) == record, 'actual prior completion pin')
    pinned.append(record)
    value = document(Path(record['path']))
    require(value['passed'] is True and value['error'] is None and not value['postcheck_errors']
            and value['source_unchanged'] is True and value['gpu_execution'] is False
            and value['tests_passed'] == (669 if role == 'worker' else 289)
            and value['tests_ignored'] == (4 if role == 'worker' else 0), 'qualified role baseline')
    expected_schema = ('ferric-p228-gfx950-clock-recorder-cpu-result-v1' if role == 'worker'
                       else 'ferric-p228-layer0-native-capture-cpu-result-v1')
    require(value['schema'] == expected_schema and set(value['phases']) == {name for name, _, _ in commands},
            'historical schema and exact phase roster')
    out = Path(record['path']).parent
    expected_raw = {'sources-base.json', 'sources-before.json', 'sources-after.json'} | {
        name + suffix for name, _, _ in commands for suffix in
        ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')}
    require(set(value['raw']) == expected_raw, 'complete historical raw roster')
    for name, pin in value['raw'].items():
        require(Path(pin['path']) == out / name and x.pin(Path(pin['path'])) == pin, 'historical raw bytes')
        pinned.append(pin)
    for name, argv, deadline in commands:
        require(document(out / (name + '-command.json')) == dict(argv=argv, env=environment, tools=tools,
            deadline_seconds=deadline, cache_cap_bytes=6 << 30, affinity=[8, 9], nice=10,
            gpu_execution=False, expected_exit=0), 'historical exact recipe/environment')
        started = document(out / (name + '-started.json'))
        require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int
                and started['pid'] > 0 and started['pid'] == started['pgid'], 'historical owned PID')
        result = document(out / (name + '-result.json'))
        require(result == value['phases'][name] and result['exit_code'] == 0 and result['reason'] is None
                and result['group_absent'] is True and result['cache_bytes'] <= 6 << 30, 'historical natural/reaped phase')
        require(all(result[s + '_sha256'] == value['raw'][name + '-' + s]['sha256'] for s in ('stdout', 'stderr')),
                'historical streams')
    require(document(out / 'sources-before.json') == document(out / 'sources-after.json'), 'historical source unchanged')
    for name, test in value['tests'].items():
        stem = 'worker-tests' if name == 'worker' else name + '-tests' if name in value['binaries'] else name
        actual = h.results((out / (stem + '-stdout')).read_text(), set(test['names']),
                           [tuple(v) for v in test['summaries']])
        require(json.loads(json.dumps(actual)) == test, 'all historical actual test outcomes')
    require(sum(v['passed'] for v in value['tests'].values()) == value['tests_passed'], 'historical aggregate replay')
    return value


def parent_recipes(h, p, stable, out):
    rows = p.recipes(h, stable, out, True)
    cargo = str(stable.N / 'bin/cargo')
    manifest = out / 'sources/ferric' / PARENT / 'Cargo.toml'
    flags = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(manifest), '--features', 'tp-batch-engineering']
    rows.insert(2, ('parent-projection-residual-wire', [cargo, 'test', *flags, '--lib', WIRE, '--', '--test-threads=2'], 1200))
    rows.extend([(NEW_BIN + '-list', [cargo, 'test', *flags, '--bin', NEW_BIN, '--', '--list', '--format', 'terse'], 1200),
                 (NEW_BIN + '-tests', [cargo, 'test', *flags, '--bin', NEW_BIN, '--', '--test-threads=2'], 1200)])
    rows = [(name, [*argv[:-1], '--bin', NEW_BIN, argv[-1]] if name == 'parent-builds' else argv, deadline)
            for name, argv, deadline in rows]
    require(len(rows) == len({name for name, _, _ in rows}) == 49, 'new parent recipe census')
    return rows


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ and sys.dont_write_bytecode,
            'ordinary bytecode-free controller')
    require(len(sys.argv) == 4, 'CONTROLLER_SHA SOURCE_MANIFEST_SHA FRESH_LABEL')
    controller_sha, source_sha, label = sys.argv[1:]
    require(re.fullmatch('[0-9a-f]{64}', source_sha)
            and re.fullmatch('projection-residual-runtime-cpu-v228-v[1-9][0-9]*', label), 'closed input namespace')
    controller = controller_input(controller_sha)
    require(HELPER.resolve(strict=True) == HELPER and hashlib.sha256(HELPER.read_bytes()).hexdigest() == HELPER_SHA,
            'pinned qualification helper')
    h = types.ModuleType('joint_qualification_helpers'); h.__file__ = str(HELPER)
    exec(compile(HELPER.read_bytes(), str(HELPER), 'exec'), h.__dict__)
    x = h.load(h.EXTRACTOR, h.EXTRACTOR_SHA, 'joint_extract')
    n = h.load(h.BOUNDS, h.BOUNDS_SHA, 'joint_bounds')
    stable = h.load(h.STABLE, h.STABLE_SHA, 'joint_stable')
    w = h.load(WORKER_HELPER, WORKER_HELPER_SHA, 'qualified_clock_worker')
    p = h.load(PARENT_HELPER, PARENT_HELPER_SHA, 'qualified_capture_parent')
    previous = h.load(w.PREVIOUS, w.PREVIOUS_SHA, 'timestamp_helpers')
    base = h.load(previous.BASE, previous.BASE_SHA, 'state_helpers')
    state = h.load(previous.STATE, previous.STATE_SHA, 'bank_helpers')
    clock = h.load(w.CLOCK, w.CLOCK_SHA, 'clock_helpers')
    pinned = [x.pin(v) for v in [controller, HELPER, h.EXTRACTOR, h.BOUNDS, h.STABLE, WORKER_HELPER,
        PARENT_HELPER, w.PREVIOUS, previous.BASE, previous.STATE, w.CLOCK]]
    nightly, nightly_pins = n.N, dict(n.PINS)
    filters = (*previous.filters_for(base, state), *clock.EXTRA_FILTERS)
    n.F, n.T = PRIOR_WORKER / 'sources/fe2o3', PRIOR_WORKER / 'target'
    old_worker_commands = state.recipes(str(n.N / 'bin/cargo'), PRIOR_WORKER / 'sources/ferric' / WORKER / 'Cargo.toml', filters)
    prior_worker = prior_receipt('worker', old_worker_commands, state.environment(n, PRIOR_WORKER), n.PINS, h, x, pinned)
    prior_parent = prior_receipt('parent', p.recipes(h, stable, PRIOR_PARENT, True),
        p.environment(stable, PRIOR_PARENT), stable.TOOLS, h, x, pinned)
    prior_runtime_names = h.inventory((PRIOR_WORKER / 'runtime-list-stdout').read_text())
    prior_worker_names = h.inventory((PRIOR_WORKER / 'worker-list-stdout').read_text())
    prior_parent_names = h.inventory((PRIOR_PARENT / 'parent-lib-list-stdout').read_text())
    require(len(prior_runtime_names) == 900 and len(prior_worker_names) == 465
            and len(prior_parent_names) == 778, 'actual complete baseline inventories')
    require(prior_worker_names == set(prior_worker['tests']['worker']['names']), 'worker baseline listing/outcomes')
    old_bins = {name: h.inventory((PRIOR_PARENT / (name + '-list-stdout')).read_text()) for name in prior_parent['binaries']}
    for name, names in old_bins.items():
        require(names == set(prior_parent['tests'][name]['names']), 'parent bin inventory/outcomes')
    proposals, rows = {}, []
    for role, (directory, digest) in PROPOSALS.items():
        path = E / directory / 'source-manifest.json'
        require(h.sha(path) == digest, 'reviewed exact source proposal')
        pinned.append(x.pin(path)); value = document(path)
        proposals[role] = value
        for row in proposal_shape(role, value):
            body = E / directory / row['source']; actual = x.pin(body)
            require(body.resolve(strict=True) == body and {k: actual[k] for k in ('bytes', 'sha256')} == row['after'], 'proposal body')
            pinned.append(actual); rows.append(dict(row, body=body))
    short_names = worker_additions(proposals['worker'])
    direct_names = set(proposals['parent']['test_census']['library'])
    new_bin_names = set(proposals['parent']['test_census']['binary'])
    require(len(direct_names) == 9 and all(v.startswith(DIRECT) for v in direct_names)
            and len(new_bin_names) == 1, 'declared parent additions')
    require(h.sha(SOURCE_MANIFEST) == source_sha, 'root-selected exact source archives')
    sources = source_shape(document(SOURCE_MANIFEST)); pinned.append(x.pin(SOURCE_MANIFEST))
    out = E / label; out.mkdir(mode=0o700)
    source = out / 'sources'; source.mkdir()
    n.F, n.T, n.N, n.PINS = source / 'fe2o3', out / 'target', nightly, nightly_pins
    n.setup(); require(not any(n.T.iterdir()), 'fresh aggregate target')
    (out / 'tmp').mkdir()
    extraction = {}
    for project, record in sources['archives'].items():
        path = Path(record['path']); require(x.pin(path) == record and path.resolve(strict=True) == path, 'source archive bytes')
        pinned.append(record); extraction[project] = x.extract(path, project, source)
    expected = x.snapshot(source)
    require(expected == pruned_source(document(PRIOR_PARENT / 'sources-after.json')), 'exact qualified source pair minus qualification documents')
    w.compatible_source_base(expected, document(PRIOR_WORKER / 'sources-after.json'))
    n.save(out / 'sources-base.json', expected)
    for row in rows:
        key = 'ferric/' + row['path']; require(expected.get(key) == row['before'], 'exact source preimage')
    prior_bin = p.NEW_BIN
    p.NEW_BIN = NEW_BIN
    try:
        p.install_overlay(source, expected, rows, lambda r: r['body'].read_bytes(), x, h.tomllib)
    finally:
        p.NEW_BIN = prior_bin
    n.save(out / 'sources-unformatted.json', expected)
    old_targets = target_inventory(); n.save(out / 'old-targets-before.json', old_targets)
    config = configurations(source, R / 'toolchain/cargo', x); n.save(out / 'configurations.json', config)
    fmt = stable.N / 'bin/rustfmt'
    require(h.sha(fmt) == RUSTFMT_SHA, 'actual retained stable formatter')
    pinned.append(x.pin(fmt))
    worker_target, parent_target = n.T / 'worker', n.T / 'parent'
    n.T = worker_target
    worker_env = state.environment(n, out)
    n.T = out / 'target'
    parent_env = dict(p.environment(stable, out), CARGO_TARGET_DIR=str(parent_target))
    for tools, home in [(nightly_pins, nightly), (stable.TOOLS, stable.N)]:
        for tool, digest in tools.items():
            require(h.sha(home / 'bin' / tool) == digest, 'actual selected toolchain')
            pinned.append(x.pin(home / 'bin' / tool))
    worker_manifest = source / 'ferric' / WORKER / 'Cargo.toml'
    parent_manifest = source / 'ferric' / PARENT / 'Cargo.toml'
    worker_commands = [('worker-metadata' if name == 'metadata' else name, argv, deadline)
        for name, argv, deadline in state.recipes(str(nightly / 'bin/cargo'), worker_manifest, filters)]
    parent_commands = parent_recipes(h, p, stable, out)
    recipes = {name: (argv, deadline, 'worker') for name, argv, deadline in worker_commands}
    recipes.update({name: (argv, deadline, 'parent') for name, argv, deadline in parent_commands})
    rust_files = [str(source / 'ferric' / r['path']) for r in rows if r['path'].endswith('.rs')]
    fmt_args = [str(fmt), '--edition', '2024', '--config', 'skip_children=true', *rust_files]
    recipes['rustfmt'] = (fmt_args, 60, 'parent')
    recipes['rustfmt-check'] = ([str(fmt), '--check', *fmt_args[1:]], 60, 'parent')
    require(len(recipes) == 84, 'closed joint phase census')
    phases, tests, metadata, binaries = {}, {'runtime': {}, 'parent': {}}, {}, {}
    error, post_errors, additions = None, [], set()

    def run(name):
        argv, deadline, role = recipes[name]
        n.F, n.N, n.PINS = ((source / 'fe2o3', nightly, nightly_pins) if role == 'worker'
                            else (source / 'ferric' / PARENT, stable.N, stable.TOOLS))
        phases[name] = n.run(out, name, argv, env=worker_env if role == 'worker' else parent_env, deadline=deadline)
        return (out / (name + '-stdout')).read_text()

    try:
        run('rustfmt')
        formatted = x.snapshot(source)
        allowed = {'ferric/' + r['path'] for r in rows if r['path'].endswith('.rs')}
        require(set(formatted) == set(expected) and all(formatted[k] == v for k, v in expected.items() if k not in allowed),
                'formatter changed only selected source files')
        expected = formatted; n.save(out / 'sources-before.json', expected)
        run('rustfmt-check')
        value = json.loads(run('worker-metadata'))
        metadata['worker'] = h.metadata_check(value, source, worker_target, True, h.tomllib.loads(worker_manifest.with_name('Cargo.lock').read_text()))
        runtime = next(v for v in value['packages'] if v['name'] == 'fe2o3-kfd')
        features = next(v['features'] for v in value['resolve']['nodes'] if v['id'] == runtime['id'])
        require('engineering-gfx950' in features and 'live-validation' not in features, 'CPU-only runtime feature closure')
        names = h.inventory(run('runtime-list')); require(names == prior_runtime_names, 'unchanged full runtime inventory')
        selected = set()
        for name, selector, count in filters:
            subset = {v for v in names if selector in v}
            require(subset == set(prior_worker['tests'][name]['names']) and len(subset) == count
                    and not selected.intersection(subset), 'exact disjoint runtime regression')
            selected.update(subset); tests['runtime'][name] = h.results(run(name), subset, [(count, 0, 0)])
        names = h.inventory(run('worker-list')); additions = extend_worker(names, prior_worker_names, short_names)
        raw = run('worker-tests')
        summaries = [tuple(v) for v in prior_worker['tests']['worker']['summaries']]
        summaries[0] = (summaries[0][0] + len(additions), 0, summaries[0][2])
        tests['worker'] = h.results(raw, names, summaries)
        require(state.ignored_names(raw) == state.ignored_names((PRIOR_WORKER / 'worker-tests-stdout').read_text())
                and not additions.intersection(state.ignored_names(raw)), 'old ignores only; every addition passed')
        binaries.update(h.artifacts(run('worker-build'), [h.CHILD_BIN], worker_manifest, worker_target, x.pin))
        metadata['parent'] = h.metadata_check(json.loads(run('parent-metadata')), source, parent_target, False,
            h.tomllib.loads(parent_manifest.with_name('Cargo.lock').read_text()))
        p.PRIOR = PRIOR_PARENT
        p.metadata_generation(metadata['parent'], prior_parent['metadata'], source)
        names = h.inventory(run('parent-lib-list'))
        wire_names = {v for v in additions if v.startswith(WIRE)}
        require(len(wire_names) == 4, 'four shared-wire additions')
        selections = parent_selections(names, prior_parent_names, direct_names, wire_names, p.old_selectors(h))
        for name, subset in selections.items():
            tests['parent'][name] = h.results(run(name), subset, [(len(subset), 0, 0)])
        for binary in [*old_bins, NEW_BIN]:
            expected_names = new_bin_names if binary == NEW_BIN else old_bins[binary]
            names = h.inventory(run(binary + '-list')); require(names == expected_names, 'exact parent bin inventory')
            tests['parent'][binary] = h.results(run(binary + '-tests'), names, [(len(names), 0, 0)])
        binaries.update(h.artifacts(run('parent-builds'), [*old_bins, NEW_BIN], parent_manifest, parent_target, x.pin))
        run('parent-default-check')
        require(set(phases) == set(recipes), 'every declared phase completed')
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        for label_, check in (
            ('sources', lambda: require(x.snapshot(source) == expected, 'source/lock drift')),
            ('old-targets', lambda: require(target_inventory() == old_targets, 'old cache modified')),
            ('configurations', lambda: require(configurations(source, R / 'toolchain/cargo', x) == config, 'config drift')),
            ('inputs', lambda: require(all(x.pin(Path(v['path'])) == v for v in pinned), 'immutable source/tool/helper input drift')),
            ('dependencies', lambda: require(all(h.sha(Path(v['manifest'])) == v['manifest_sha256']
                for m in metadata.values() for v in m['external']), 'external dependency manifest drift')),
            ('binaries', lambda: require(all(x.pin(Path(v['binary']['path'])) == v['binary'] for v in binaries.values()), 'built ELF drift')),
            ('target', lambda: require(n.size(out / 'target') <= 6 << 30, 'aggregate private target cap')),
        ):
            try: check()
            except BaseException as failure: post_errors.append(label_ + ': ' + repr(failure))
        n.save(out / 'sources-after.json', x.snapshot(source))
        n.save(out / 'old-targets-after.json', target_inventory())
    passed = error is None and not post_errors
    all_tests = [*tests['runtime'].values(), *tests['parent'].values(), *([tests['worker']] if 'worker' in tests else [])]
    value = dict(schema='ferric-p228-projection-residual-runtime-cpu-result-v1', passed=passed, error=error,
        postcheck_errors=post_errors, inputs=pinned, controller=x.pin(controller),
        source_manifest=x.pin(SOURCE_MANIFEST), proposal_manifests={role: x.pin(E / directory / 'source-manifest.json')
            for role, (directory, _) in PROPOSALS.items()}, extraction=extraction,
        prior_worker_cpu=PRIOR_PINS['worker'], prior_parent_cpu=PRIOR_PINS['parent'],
        source_unchanged=not any(v.startswith('sources:') for v in post_errors),
        added_worker_tests=sorted(additions), added_parent_tests=dict(lib=sorted(direct_names | {v for v in additions if v.startswith(WIRE)}), bin=sorted(new_bin_names)),
        metadata=metadata, phases=phases, tests=tests, binaries=binaries,
        tests_passed=sum(v['passed'] for v in all_tests), tests_ignored=sum(v['ignored'] for v in all_tests),
        empty_initial_target=True, external_cargo_cache_reused=True, parent_rebuilt=passed, worker_rebuilt=passed,
        compiler_hsaco_reproduced=False, gpu_execution=False, numerical_acceptance=False,
        performance_claim=False, timestamp_calibration=False, production_authority=False,
        raw={path.name: x.pin(path) for path in out.iterdir() if path.is_file()})
    result = out / ('complete.json' if passed else 'failed.json'); n.save(result, value)
    print(json.dumps(dict(passed=passed, receipt=x.pin(result), error=error, postcheck_errors=post_errors)), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
