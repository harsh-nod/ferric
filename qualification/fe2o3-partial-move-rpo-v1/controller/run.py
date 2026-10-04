"""Fresh RPO scheduler qualification over the actual ordinary compiler source."""
import argparse
import json
import os
from pathlib import Path
import re
import resource
import signal
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
QUALIFIED = E / 'ordinary-induction-compiler-cpu-v228-v1'
QUALIFIED_COPY = QUALIFIED / 'source/fe2o3'
F = QUALIFIED_COPY
OUT = E / 'rpo-compiler-cpu-v228-v2'
OWNER = E / 'rpo-compiler-cpu-owner-v228-v2'
COPY = OUT / 'source/fe2o3'
PATCH = E / 'p228-partial-move-rpo-v2'
PATCH_SHA = '939b76eb28df8d7e2b53ae0b4f5034257185f12f077d20e581d9880b006a252c'
QUALIFIED_COMPLETE = dict(path=str(QUALIFIED / 'complete.json'), bytes=369402,
    sha256='c45aa83b01bf76fdfd9962b06884611658dab607b001ba67fecc66db9fd5ba97')
QUALIFIED_SOURCES = dict(path=str(QUALIFIED / 'sources-before.json'), bytes=3779269,
    sha256='5aad4c8ba6520c323310424ce343de2ae5088e7f8c1c0b1fd67853b5fd3bd9f9')
QUALIFIED_OWNER = dict(path=str(E / 'ordinary-induction-compiler-cpu-owner-v228-v1/complete.json'),
    bytes=57318, sha256='85de66610631612ed2473f9d5fbb80ce89ea7ffc6064c1784834a6751d623609')
QUALIFIED_CONTROLLER = E / 'p228-ordinary-induction-compiler-cpu-v1/run.py'
QUALIFIED_CONTROLLER_SHA = '7d14e9ecdf50f4e81c8f65469b630cdcbc118d67bae9ce879e4455a351885e18'
FORMATTER = R / 'toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/bin/rustfmt'
FORMATTER_SHA = 'a9137d0c198ceb6c72193d517d3c9007b3ec7a90d3d10ec6889773eca48261b4'
REL = Path('crates/fe2o3-pliron/src/production')
FILES = {str(REL / 'semantic_ssa.rs'), *{str(REL / 'semantic_ssa' / name) for name in (
    'partial_moves.rs', 'partial_move_rpo_v1_tests.rs', 'partial_move_fifo_oracle_v1.rs')}}
PREFIX = 'production::semantic_ssa::partial_moves::rpo_tests::'
EXTRA_OLD_TARGETS = (QUALIFIED / 'target', E / 'idempotent-compiler-cpu-v228-v2/target',
    E / 'row-reciprocal-checked-probe-v228-v6/target', E / 'row-reciprocal-checked-probe-v228-v7/target')
BASE_PHASES = {'metadata', 'compiler-build'} | {
    short + '-' + suffix for short in ('pliron', 'compiler')
    for suffix in ('build-tests', 'list', 'ignored-list', 'tests')
}
PHASES = BASE_PHASES | {'rustfmt', 'rustfmt-check'}
HELPERS = {
    'probe': ('p228-reciprocal-checked-probe-v3/run.py', 'bb735600db46442e5f5c87c073524bde945041f85a9fc236f5ba0516f6f3d27a'),
    'outer': ('p228-reciprocal-outer-v2/run.py', 'a024e17e2a6b4456aa04c1ab895aead002bd12aa47045533e4e26b2b6b590121'),
    'driver': ('p227-prefix-tiles-source-pipeline-v5/run_row_facts_v2.py', '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820'),
    'owned': ('p227-prefix-tiles-cohort-v9/frozen_owned.py', 'ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'),
    'inventory': ('p227-prefix-tiles-cohort-v9/native_bounded.py', '48d61daaf67b145f3cf2928c1abadcbcf1295831ee8a1ccf3fd6a442f74cfac0'),
    'bounded': ('bounded.py', 'e634e1b3be3b122b2231ad134c770f25d7d71d10ec767d61a16d03e726bdf4f1'),
}


def require(value, message):
    if not value:
        raise RuntimeError(message)


def load_helpers():
    import hashlib
    result = {}
    for key, (name, digest) in HELPERS.items():
        path = E / name
        require(path.resolve(strict=True) == path and path.is_file(), 'canonical helper')
        raw = path.read_bytes()
        require(len(raw) < 1 << 20 and hashlib.sha256(raw).hexdigest() == digest, 'retained helper identity')
        value = types.ModuleType('rpo_compiler_' + key)
        value.__file__ = str(path)
        exec(compile(raw, str(path), 'exec'), value.__dict__)
        result[key] = value
    return result


def metadata_paths(metadata, source):
    require(metadata['target_directory'] == str(OUT / 'target'), 'fresh Cargo target')
    packages = metadata['packages']
    require(0 < len(packages) <= 1024, 'bounded metadata')
    roots = set()
    for row in packages:
        path = Path(row['manifest_path'])
        require(path.name == 'Cargo.toml' and path.is_relative_to(R), 'owned dependency root')
        require(row['source'] is not None or path.is_relative_to(source), 'local package escaped fresh copy')
        roots.add(path.parent)
    for name in ('fe2o3-pliron', 'rustc-codegen-fe2o3'):
        require([row['manifest_path'] for row in packages if row['name'] == name]
                == [str(source / 'crates' / name / 'Cargo.toml')], 'exact changed compiler package')
    return roots


def inventory(raw):
    names = re.findall(r'^([^\r\n]+): test$', raw, re.M)
    require(len(names) == len(set(names)) and names, 'nonempty unique actual test inventory')
    return sorted(names)


def test_results(raw, names, ignored):
    rows = re.findall(r'^test (\S+)(?: - should panic)? \.\.\. (ok|ignored(?:, [^\n]*)?)$', raw, re.M)
    passed = sorted(name for name, status in rows if status == 'ok')
    skipped = sorted(name for name, status in rows if status.startswith('ignored'))
    require(sorted(passed + skipped) == names and skipped == ignored, 'actual named results match inventory')
    require(re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw)
            == [(str(len(passed)), '0', str(len(ignored)))], 'exact test summary')
    return dict(passed=len(passed), ignored=len(skipped), names=names, ignored_names=ignored)


def built_artifacts(raw, package, target, tests, root):
    rows = [json.loads(line) for line in raw.splitlines() if line.strip()]
    candidates = [row for row in rows if row.get('reason') == 'compiler-artifact'
                  and row.get('target', {}).get('name') == target and row.get('profile', {}).get('test') is tests
                  and row.get('manifest_path') == str(COPY / 'crates' / package / 'Cargo.toml')]
    require(len(candidates) == 1, 'one actual Cargo artifact: ' + target)
    row = candidates[0]
    paths = [row['executable']] if row['executable'] else row['filenames']
    require(paths and all(Path(name).is_relative_to(root) for name in paths), 'fresh build artifacts')
    return paths


def relative_sources(snapshot, root):
    require(type(snapshot) is dict and snapshot, 'nonempty source snapshot')
    result = {}
    for name, row in snapshot.items():
        require(type(name) is str and type(row) is dict and set(row) == {'pin', 'stamp'},
                'closed source snapshot row')
        path = Path(name)
        require(path.is_absolute() and str(path) == name and '..' not in path.parts
                and path.is_relative_to(root) and path != root, 'exact source snapshot root')
        pin = row['pin']
        require(type(pin) is dict and set(pin) == {'path', 'bytes', 'sha256'}
                and pin['path'] == name and type(pin['bytes']) is int and pin['bytes'] >= 0
                and type(pin['sha256']) is str and re.fullmatch('[0-9a-f]{64}', pin['sha256']),
                'closed source snapshot pin')
        require(type(row['stamp']) is list and len(row['stamp']) == 5
                and all(type(word) is int for word in row['stamp'])
                and row['stamp'][2] == pin['bytes'], 'source stamp/extent binding')
        relative = str(path.relative_to(root))
        require(relative not in result, 'unique relative source member')
        result[relative] = {key: pin[key] for key in ('bytes', 'sha256')}
    return result



def overlay_members(value):
    require(value.get('schema') == 'ferric-p228-partial-move-rpo-source-proposal-v2'
            and value.get('status') == 'untested-candidate'
            and value.get('tests_authored') == 17, 'exact RPO proposal')
    for flag in ('tests_executed', 'formatted', 'compiler_build_executed',
                 'checked_probe_executed', 'gpu_execution', 'production_authority', 'limits_changed'):
        require(value.get(flag) is False, 'proposal confers no execution or authority')
    require(len(value['files']) == 4 and {row['path'] for row in value['files']} == FILES,
            'exact four RPO source destinations')
    pins = []
    for row in value['files']:
        require(set(row) == {'path', 'source', 'before', 'after'}
                and row['source'] == 'draft/' + row['path']
                and set(row['after']) == {'bytes', 'sha256'}, 'closed source overlay row')
        expected_new = row['path'].endswith(('partial_move_rpo_v1_tests.rs', 'partial_move_fifo_oracle_v1.rs'))
        require((row['before'] is None) == expected_new
                and (expected_new or set(row['before']) == {'bytes', 'sha256'}), 'two preimages and two new bodies')
        pins.append(dict(row['after'], path=str(PATCH / row['source'])))
    for row in value['retained_files']:
        relative = Path(row['path'])
        require(not relative.is_absolute() and '..' not in relative.parts and str(relative) == row['path'],
                'retained proposal member path')
        pins.append(dict(row, path=str(PATCH / relative)))
    require(len(pins) == len({row['path'] for row in pins}), 'unique proposal members')
    for key, pin in (('qualified_generation', QUALIFIED_COMPLETE), ('qualified_source_snapshot', QUALIFIED_SOURCES)):
        require({k: value[key][k] for k in ('bytes', 'sha256')}
                == {k: pin[k] for k in ('bytes', 'sha256')}, 'exact qualified predecessor identity')
    names = value['test_names']
    require(len(names) == len(set(names)) == 17
            and all(type(name) is str and name.startswith(PREFIX)
                    and re.fullmatch('[A-Za-z0-9_:]+', name) for name in names), 'seventeen exact RPO names')
    return pins


def qualified_generation(value, snapshot):
    require(value.get('schema') == 'fe2o3-p228-ordinary-induction-compiler-cpu-result-v1'
            and value.get('passed') is True and value.get('error') is None
            and value.get('postcheck_errors') == [] and value.get('fresh_compiler_built') is True
            and value.get('gpu_execution') is False and value.get('production_authority') is False,
            'actual passing ordinary compiler generation')
    require(value['raw']['sources-before.json'] == QUALIFIED_SOURCES
            and value['raw']['sources-after.json'] == dict(QUALIFIED_SOURCES,
                path=str(QUALIFIED / 'sources-after.json')), 'qualified source before/after identity')
    require(set(value['phases']) == BASE_PHASES and all(
            type(row.get('exit_code')) is int and row['exit_code'] == 0
            and row.get('group_absent') is True and row.get('reason') is None
            for row in value['phases'].values()), 'ten natural qualified phases')
    require(set(value['tests']) == {'pliron', 'compiler'}, 'both complete qualified suites')
    for short, passed, ignored in (('pliron', 1487, 1), ('compiler', 1196, 24)):
        row = value['tests'][short]
        require(row['passed'] == passed and row['ignored'] == ignored
                and row['names'] == sorted(set(row['names'])) and len(row['names']) == passed + ignored
                and row['ignored_names'] == sorted(set(row['ignored_names']))
                and len(row['ignored_names']) == ignored and set(row['ignored_names']) <= set(row['names']),
                'actual full-suite named inventory')
    sources = relative_sources(snapshot, QUALIFIED_COPY)
    require(len(sources) == 5781, 'complete qualified source roster')
    return sources


def required_tests(short, names, ignored, qualified, proposal):
    require(short in ('pliron', 'compiler'), 'closed compiler test suite')
    previous = qualified['tests'][short]
    extra = set(proposal['test_names']) if short == 'pliron' else set()
    require(not extra.intersection(previous['names']), 'new tests are not old results')
    require(names == sorted(set(previous['names']) | extra)
            and ignored == previous['ignored_names'], 'every old test and ignore preserved')
    require({name for name in names if name.startswith(PREFIX)} == extra
            and not extra.intersection(ignored), 'all RPO tests compiled and nonignored')
    return sorted(extra)


def source_transition(before, after, proposal, formatted=False):
    expected = dict(before)
    for row in proposal['files']:
        require(before.get(row['path']) == row['before'], 'exact source preimage or new-file absence')
        expected[row['path']] = row['after']
    require(set(after) == set(expected), 'complete source roster after overlay')
    require(all(after[name] == pin for name, pin in expected.items()
                if not formatted or name not in FILES), 'only declared source changes')
    return after


def relocate(value):
    if isinstance(value, str):
        return value.replace(str(QUALIFIED), str(OUT))
    if isinstance(value, list):
        return [relocate(item) for item in value]
    if isinstance(value, dict):
        return {key: relocate(item) for key, item in value.items()}
    return value


def replay_prior(p, qualified):
    pins = []
    for name, pin in qualified['raw'].items():
        require(pin['path'] == str(QUALIFIED / name), 'original prior raw path')
        p.read(pin['path'], pin)
        pins.append(pin)
    for name in BASE_PHASES:
        result = p.parse(p.read(QUALIFIED / (name + '-result.json'), retain=True)[2])
        require(result == qualified['phases'][name], 'prior actual phase result')
        for stream in ('stdout', 'stderr'):
            require(qualified['raw'][name + '-' + stream]['sha256'] == result[stream + '_sha256'],
                    'prior actual phase stream')
    for short in ('pliron', 'compiler'):
        read = lambda suffix: p.read(QUALIFIED / (short + '-' + suffix + '-stdout'), retain=True)[2].decode()
        names = inventory(read('list'))
        ignored = sorted(re.findall(r'^([^\r\n]+): test$', read('ignored-list'), re.M))
        require(test_results(read('tests'), names, ignored) == qualified['tests'][short],
                'prior full named outcome replay')
    for pin in qualified['artifacts'].values():
        p.read(pin['path'], pin)
        pins.append(pin)
    return pins


def child(modules, package_pin):
    p, n = modules['probe'], modules['bounded']
    require(not os.path.lexists(OUT), 'fresh CPU row')
    OUT.mkdir(mode=0o700)
    n.F, n.T, n.D = COPY, OUT / 'target', OUT
    n.setup()
    input_pin = dict(path=str(E / 'reciprocal-checked-probe-inputs-v228-v3.json'), bytes=1274,
                     sha256='acda76c9d288695bddcf5c3744997c1732231651519ec1c1620cd86651824c92')
    inputs = p.parse(p.read(input_pin['path'], input_pin, retain=True)[2])
    require(set(inputs) == {'schema', 'source_maps', 'cargo_configurations'}
            and inputs['schema'] == 'ferric-p228-reciprocal-checked-probe-inputs-v3', 'closed source inputs')
    p.configurations(inputs['cargo_configurations'])
    merged, source_records = p.source_maps(inputs['source_maps'])
    p.provider(merged)
    expected = {str(R / name): digest for name, digest in merged.items()}
    qualified = p.parse(p.read(QUALIFIED_COMPLETE['path'], QUALIFIED_COMPLETE, retain=True)[2])
    qualified_snapshot = p.parse(p.read(QUALIFIED_SOURCES['path'], QUALIFIED_SOURCES, retain=True)[2])
    qualified_sources = qualified_generation(qualified, qualified_snapshot)
    old_owner = p.parse(p.read(QUALIFIED_OWNER['path'], QUALIFIED_OWNER, retain=True)[2])
    require(old_owner['passed'] is True and old_owner['error'] is None
            and old_owner['postcheck_errors'] == [] and old_owner['completion'] == QUALIFIED_COMPLETE,
            'actual qualified owner completion')
    modules['outer'].check_outcome(old_owner['owned'])
    prior_pins = replay_prior(p, qualified)
    source_files = p.tree(F, exclusions=('.git', 'target'))
    original = p.snapshot(sorted(set(source_files) | set(expected)), expected, byte_cap=4 << 30)
    require(relative_sources({name: original[name] for name in source_files}, F) == qualified_sources,
            'exact actual ordinary source bodies before copy')
    p.save(OUT / 'original-before.json', original)
    manifest_pin, _, raw = p.read(PATCH / 'source-manifest.json', retain=True)
    require(manifest_pin['sha256'] == PATCH_SHA, 'frozen RPO source manifest')
    proposal = p.parse(raw)
    patch_pins = overlay_members(proposal)
    for pin in patch_pins:
        p.read(pin['path'], pin)
    config_names = set(p.config_paths())
    p.F = F
    config_names.update(p.config_paths())
    p.F = COPY
    config_names.update(p.config_paths())
    p.config_paths = lambda: sorted(config_names)
    configurations = {name: None for name in config_names}
    before_config = p.configurations(configurations)
    for name in source_files:
        source = Path(name)
        target = COPY / source.relative_to(F)
        target.parent.mkdir(parents=True, exist_ok=True)
        raw = p.read(source, original[name]['pin'], retain=True)[2]
        with target.open('xb') as stream:
            stream.write(raw)
        target.chmod(0o700 if source.stat().st_mode & 0o111 else 0o600)
    require(relative_sources(p.snapshot(p.tree(COPY)), COPY) == qualified_sources, 'exact qualified source copy')
    for row in proposal['files']:
        source, target = PATCH / row['source'], COPY / row['path']
        require(qualified_sources.get(row['path']) == row['before'], 'exact overlay preimage')
        pin = dict(row['after'], path=str(source))
        raw = p.read(source, pin, retain=True)[2]
        with target.open('xb' if row['before'] is None else 'wb') as stream:
            stream.write(raw)
    unformatted = p.snapshot(p.tree(COPY))
    source_transition(qualified_sources, relative_sources(unformatted, COPY), proposal)
    p.save(OUT / 'sources-unformatted.json', unformatted)
    p.save(OUT / 'qualified-generation.json', dict(completion=QUALIFIED_COMPLETE, sources=QUALIFIED_SOURCES,
        owner=QUALIFIED_OWNER, source_members=len(qualified_sources), relative_source_identity_matched=True))
    require(p.read(FORMATTER)[0]['sha256'] == FORMATTER_SHA
            and p.read(QUALIFIED_CONTROLLER)[0]['sha256'] == QUALIFIED_CONTROLLER_SHA, 'formatter and predecessor controller')
    immutable = [manifest_pin['path'], package_pin['path'], str(Path(__file__).resolve()), input_pin['path'],
                 str(FORMATTER), str(QUALIFIED_CONTROLLER), QUALIFIED_COMPLETE['path'],
                 QUALIFIED_SOURCES['path'], QUALIFIED_OWNER['path']]
    immutable += [str(E / name) for name, _ in HELPERS.values()] + [pin['path'] for pin in patch_pins + prior_pins]
    immutable += [record['pin']['path'] for record in source_records]
    immutable += [str(n.N / 'bin' / name) for name in n.PINS]
    rust_source_root = n.N / 'lib/rustlib/src/rust/library'
    rust_source_paths = p.tree(rust_source_root, exclusions=('.git', 'target'))
    immutable += rust_source_paths
    immutable += [str(path) for directory in (n.N / 'lib', n.N / 'lib/rustlib/x86_64-unknown-linux-gnu/lib')
                  for path in directory.glob('*.so*')]
    immutable_before = p.snapshot(immutable, byte_cap=4 << 30)
    p.save(OUT / 'inputs-before.json', dict(files=immutable_before, configurations=before_config,
        source_maps=source_records, patch=patch_pins,
        qualified_generation=dict(completion=QUALIFIED_COMPLETE, sources=QUALIFIED_SOURCES, owner=QUALIFIED_OWNER)))
    env = n.environment()
    (n.T / 'tmp').mkdir(mode=0o700)
    env['TMPDIR'] = str(n.T / 'tmp')
    cargo = str(n.N / 'bin/cargo')
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(COPY / 'Cargo.toml')]
    phases, results, artifacts, dependencies = {}, {}, {}, None
    before, formatted_sources = unformatted, {}
    error, post_errors = None, []

    def run(name, argv, deadline=1800):
        try:
            n.run(OUT, name, argv, env=env, deadline=deadline)
        finally:
            path = OUT / (name + '-result.json')
            if path.is_file():
                phases[name] = p.parse(p.read(path, retain=True)[2])
        return p.read(OUT / (name + '-stdout'), cap=64 << 20, retain=True)[2].decode()

    try:
        fmt = [str(FORMATTER), '--edition', '2024', '--config', 'skip_children=true',
               *[str(COPY / row['path']) for row in proposal['files']]]
        run('rustfmt', fmt, 60)
        before = p.snapshot(p.tree(COPY))
        source_transition(qualified_sources, relative_sources(before, COPY), proposal, formatted=True)
        p.save(OUT / 'sources-before.json', before)
        formatted_sources = {name: p.read(COPY / name)[0] for name in sorted(FILES)}
        run('rustfmt-check', [fmt[0], '--check', *fmt[1:]], 60)
        metadata = p.parse(run('metadata', [cargo, 'metadata', '--offline', '--locked',
                              '--manifest-path', str(COPY / 'Cargo.toml'), '--format-version', '1'], 120))
        require(metadata == relocate(p.parse(p.read(QUALIFIED / 'metadata-stdout', retain=True)[2])),
                'exact relocated qualified dependency metadata')
        roots = metadata_paths(metadata, COPY)
        dependency_paths = sorted({name for root in roots for name in p.tree(root, exclusions=('.git', 'target'))})
        dependencies = p.snapshot(dependency_paths, byte_cap=4 << 30)
        p.save(OUT / 'dependencies-before.json', dependencies)
        for short, package, target in (('pliron', 'fe2o3-pliron', 'fe2o3_pliron'),
                                        ('compiler', 'rustc-codegen-fe2o3', 'rustc_codegen_fe2o3')):
            built = run(short + '-build-tests', [cargo, 'test', *common, '-p', package,
                        '--lib', '--no-run', '--message-format=json'])
            paths = built_artifacts(built, package, target, True, n.T)
            require(len(paths) == 1, 'one test executable')
            binary = paths[0]
            artifacts[short + '-tests'] = p.read(binary)[0]
            names = inventory(run(short + '-list', [binary, '--list', '--format', 'terse'], 120))
            ignored_raw = run(short + '-ignored-list', [binary, '--ignored', '--list', '--format', 'terse'], 120)
            ignored = sorted(re.findall(r'^([^\r\n]+): test$', ignored_raw, re.M))
            require(set(ignored) <= set(names), 'ignored names belong to inventory')
            required_tests(short, names, ignored, qualified, proposal)
            results[short] = test_results(run(short + '-tests', [binary, '--test-threads=2']), names, ignored)
        built = run('compiler-build', [cargo, 'build', *common, '-p', 'rustc-codegen-fe2o3',
                    '--lib', '--bin', 'fe2o3-rustc-extract', '--message-format=json'])
        for target in ('rustc_codegen_fe2o3', 'fe2o3-rustc-extract'):
            paths = built_artifacts(built, 'rustc-codegen-fe2o3', target, False, n.T)
            required_path = n.T / 'debug' / ('librustc_codegen_fe2o3.so'
                                           if target == 'rustc_codegen_fe2o3' else target)
            require(str(required_path) in paths, 'actual fresh backend dylib and extractor required')
            for name in paths:
                artifacts[Path(name).name] = p.read(name)[0]
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        checks = {
            'original': lambda: p.snapshot(sorted(set(p.tree(F, exclusions=('.git', 'target'))) | set(expected)), expected, byte_cap=4 << 30),
            'sources': lambda: p.snapshot(p.tree(COPY)),
            'inputs': lambda: p.snapshot(immutable, byte_cap=4 << 30),
            'configurations': lambda: p.configurations(configurations),
        }
        expected_checks = dict(original=original, sources=before, inputs=immutable_before, configurations=before_config)
        if dependencies is not None:
            checks['dependencies'] = lambda: p.snapshot(sorted({name for root in roots for name in p.tree(root, exclusions=('.git', 'target'))}), byte_cap=4 << 30)
            expected_checks['dependencies'] = dependencies
        for name, operation in checks.items():
            try:
                observed = operation()
                p.save(OUT / (name + '-after.json'), observed)
                require(observed == expected_checks[name], name + ' changed')
            except BaseException as failure:
                post_errors.append(name + ': ' + repr(failure))
        try:
            require(p.tree(rust_source_root, exclusions=('.git', 'target')) == rust_source_paths, 'rust-src roster changed')
            p.read(manifest_pin['path'], manifest_pin)
            for pin in patch_pins + prior_pins:
                p.read(pin['path'], pin)
            for pin in (QUALIFIED_COMPLETE, QUALIFIED_SOURCES, QUALIFIED_OWNER, *artifacts.values()):
                p.read(pin['path'], pin)
            require(p.source_maps(inputs['source_maps']) == (merged, source_records), 'original source-map bytes or stamps changed')
        except BaseException as failure:
            post_errors.append('immutable pins: ' + repr(failure))
    passed = error is None and not post_errors and set(phases) == PHASES
    result = dict(schema='fe2o3-p228-rpo-compiler-cpu-result-v1', passed=passed,
                  error=error, postcheck_errors=post_errors, phases=phases, tests=results, artifacts=artifacts,
                  patch=manifest_pin, formatted_sources=formatted_sources,
                  qualified_generation=dict(completion=QUALIFIED_COMPLETE, sources=QUALIFIED_SOURCES, owner=QUALIFIED_OWNER),
                  required_test_names=dict(pliron=sorted(proposal['test_names']), compiler=[]),
                  package=package_pin, source_unchanged=not any(row.startswith('sources:') for row in post_errors),
                  fresh_compiler_built=passed, checked_lowering=False, fresh_hsaco_emitted=False,
                  gpu_execution=False, production_authority=False, numerical_acceptance=False,
                  performance_claim=False, full_model_acceptance=False,
                  raw={path.name: p.read(path)[0] for path in OUT.iterdir() if path.is_file()})
    p.save(OUT / ('complete.json' if passed else 'failed.json'), result)
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors, output=str(OUT))), flush=True)
    return 0 if passed else 1


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and sys.dont_write_bytecode, 'ordinary unoptimized bytecode-free Python')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('manifest_sha256')
    parser.add_argument('--child', action='store_true')
    args = parser.parse_args()
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'fixed CPU envelope')
    modules = load_helpers()
    p, outer, driver = modules['probe'], modules['outer'], modules['driver']
    pins = driver.Pins()
    package = Path(__file__).resolve().parent
    require(package == E / 'p228-partial-move-rpo-cpu-v2', 'exact isolated controller package')
    manifest, package_pin = pins.json(package / 'manifest.json', args.manifest_sha256)
    require(len(manifest['files']) == 3 and {row['path'] for row in manifest['files']}
            == {'run.py', 'test_run.py', 'README.md'}, 'closed CPU package')
    for member in manifest['files']:
        outer.pin_exact(pins, dict(member, path=str(package / member['path'])))
    for name, sha in HELPERS.values():
        pins.pin(E / name, sha)
    proposal, patch_pin = pins.json(PATCH / 'source-manifest.json', PATCH_SHA)
    for pin in overlay_members(proposal):
        outer.pin_exact(pins, pin, PATCH)
    for pin in (QUALIFIED_COMPLETE, QUALIFIED_SOURCES, QUALIFIED_OWNER):
        outer.pin_exact(pins, pin, E)
    pins.pin(QUALIFIED_CONTROLLER, QUALIFIED_CONTROLLER_SHA)
    pins.pin(FORMATTER, FORMATTER_SHA)
    if args.child:
        signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt('owned termination')))
        return child(modules, package_pin)
    for kind, cap in ((resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, 1 << 30), (resource.RLIMIT_CORE, 0)):
        old = resource.getrlimit(kind)
        value = min([cap] + [n for n in old if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))
    require(not os.path.lexists(OWNER) and not os.path.lexists(OUT), 'fresh owned CPU outputs')
    os.umask(0o077)
    OWNER.mkdir(mode=0o700)
    protected_targets = tuple(dict.fromkeys((*outer.OLD_TARGETS, *EXTRA_OLD_TARGETS)))
    old_targets = outer.inventory(modules['inventory'], protected_targets)
    p.save(OWNER / 'old-targets-before.json', old_targets)
    argv = ['/usr/bin/python3', '-B', str(Path(__file__).resolve()), args.manifest_sha256, '--child']
    env = modules['bounded'].environment()
    p.save(OWNER / 'command.json', dict(argv=argv, env=env, deadline_seconds=10800, gpu_execution=False))
    outcome, error, completion = None, None, None
    try:
        outcome = driver.run_coordinator(OWNER, argv, F, env, modules['owned'], p.save, deadline=10800)
        p.save(OWNER / 'owned-result.json', outcome)
        outer.check_outcome(outcome)
        value, completion = pins.json(OUT / 'complete.json')
        require(value['passed'] is True and value['package'] == package_pin
                and value['schema'] == 'fe2o3-p228-rpo-compiler-cpu-result-v1'
                and value['patch'] == patch_pin
                and value['qualified_generation'] == dict(completion=QUALIFIED_COMPLETE, sources=QUALIFIED_SOURCES, owner=QUALIFIED_OWNER)
                and value['gpu_execution'] is False and value['production_authority'] is False, 'CPU completion join')
        for record in [*value['raw'].values(), *value['artifacts'].values(), *value['formatted_sources'].values()]:
            outer.pin_exact(pins, record, OUT)
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        after, post_errors = outer.postchecks([
            ('pins', None, pins.recheck),
            ('old_targets', old_targets, lambda: outer.inventory(modules['inventory'], protected_targets)),
        ])
        p.save(OWNER / 'after.json', after)
    passed = error is None and not post_errors and completion is not None
    p.save(OWNER / ('complete.json' if passed else 'failed.json'), dict(
        schema='fe2o3-p228-rpo-compiler-owned-result-v1', passed=passed,
        error=error, postcheck_errors=post_errors, owned=outcome, completion=completion,
        patch=patch_pin, qualified_generation=dict(completion=QUALIFIED_COMPLETE, sources=QUALIFIED_SOURCES, owner=QUALIFIED_OWNER),
        gpu_execution=False, production_authority=False))
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors, output=str(OWNER))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
