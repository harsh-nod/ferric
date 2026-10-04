"""Bounded CPU arithmetic/source qualification of the additive V7 RoPE macro."""
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import sys
import time
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
BASE = E / 'row-reciprocal-checked-probe-v228-v7'
BASE_SHA = '6ba25826b71e30f5106e1efc9e786c021c56f5bab397add20b365373685081a9'
BASE_MAPS = {
    'before.json': 'ae528b8655b9a80027aa09780c4be9d04191357f50cac2b7b27e344e5198b36e',
    'after.json': '3c0ea0fd42729d37fb8bb2b713fb481ba68a8607a7943f36a37731f3e86d2c19',
}
PRIOR = E / 'reciprocal-cpu-v228-v3'
PRIOR_SHA = '6b02d358eca33d31d6839e9c28de38276b4f968cfaec93b7ddb97963a77956d3'
OLD_RUNNER = E / 'p228-down2-cpu-v1/run.py'
OLD_RUNNER_SHA = 'a48976422c3eba35308d3f210022bcd2b736dcd9928f7b4f40d9f417293f9f24'
FORMATTER = R / 'toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/bin/rustfmt'
FORMATTER_SHA = 'a9137d0c198ceb6c72193d517d3c9007b3ec7a90d3d10ec6889773eca48261b4'
PROVIDER_SHA = '037e472ea24d7337f88028ed65cb3d6ea6e0c0681538e6121ab78240c44ef675'
TARGETS = ('prefix_reciprocal_v1', 'rope_materialized_v1')
EXHAUSTIVE = 'exhaustive_all_significands_in_one_binade'
BASE_RUST = {
    'src/lib.rs', 'src/wave_numerics_v1.rs', 'src/head_rope_numerics_v3.rs',
    'src/prefix_reciprocal_numerics_v1.rs', 'src/attention_online.rs',
    'src/output_projection_numerics_v5.rs', 'src/prefix_tiles_numerics_v6.rs',
}
CHANGES = {'src/lib.rs', 'src/prefix_rope_materialized_numerics_v1.rs',
           'tests/rope_materialized_v1.rs', 'tests/fixtures/v7_lib.rs'}
FORMAT = CHANGES - {'tests/fixtures/v7_lib.rs'}
OLD_TEST_INPUTS = {'tests/prefix_reciprocal_v1.rs',
                   'tests/fixtures/prefix_reciprocal_fraction_v1.tsv'}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    path = Path(path)
    require(path.is_absolute() and path.is_file() and not path.is_symlink()
            and path.resolve(strict=True) == path, 'canonical regular input: ' + str(path))
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        while block := stream.read(1 << 20):
            digest.update(block)
    return dict(path=str(path), bytes=path.stat().st_size, sha256=digest.hexdigest())


def load(path, expected, name):
    require(pin(path)['sha256'] == expected, 'helper hash: ' + str(path))
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(path.read_bytes(), str(path), 'exec'), module.__dict__)
    return module


def proposal(value, base):
    require(value['schema'] == 'ferric-p228-rope-materialized-source-proposal-v1'
            and value['status'] == 'authored-not-executed'
            and value['authored_tests'] == 20
            and all(value[key] is False for key in ('tests_executed', 'old_sources_modified',
                    'default_route_modified', 'provider_modified', 'compiler_modified',
                    'gpu_execution', 'numerical_acceptance', 'production_authority')),
            'source-only candidate proposal')
    require(value['baseline_fixture']['directory'] == str(BASE / 'fixture')
            and value['baseline_fixture']['files'] == base
            and value['baseline_fixture']['complete'] == pin(BASE / 'complete.json'),
            'source proposal binds actual V7 fixture')
    rows = value['files']
    require(len(rows) == 4 and {row['path'] for row in rows} == CHANGES,
            'closed four-file candidate roster')
    for row in rows:
        require(row['source'] == 'draft/' + row['path']
                and row['before'] == base.get(row['path']), 'exact V7 preimage/new-file absence')
    names = value['test_census'][TARGETS[1]]
    require(set(value['test_census']) == {TARGETS[1]}
            and len(names) == len(set(names)) == 20
            and all(re.fullmatch(r'[a-z][a-z0-9_]*', name) for name in names),
            'closed named candidate test census')
    return rows, set(names)


def select_tests(raw, fixture, target):
    messages = [json.loads(line) for line in raw.splitlines() if line.startswith('{')]
    require([v.get('success') for v in messages if v.get('reason') == 'build-finished'] == [True],
            'one successful Cargo build')
    selected = {}
    for row in messages:
        if row.get('reason') != 'compiler-artifact' or row.get('target', {}).get('kind') != ['test']:
            continue
        name, profile = row['target']['name'], row['profile']
        require(name in TARGETS and name not in selected
                and row['manifest_path'] == str(fixture / 'Cargo.toml')
                and row['target']['src_path'] == str(fixture / 'tests' / (name + '.rs'))
                and profile['test'] is True and profile['opt_level'] == '2'
                and profile['debug_assertions'] is True and profile['overflow_checks'] is True,
                'exact opt2 checked CPU test artifact')
        binary = Path(row['executable'])
        require(binary.is_relative_to(target) and os.access(binary, os.X_OK), 'fresh target executable')
        selected[name] = dict(binary=pin(binary), cargo=row)
    require(set(selected) == set(TARGETS), 'both real test binaries required')
    return selected


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ,
            'ordinary Python required before loading assert-based owned helpers')
    require(sys.dont_write_bytecode, 'invoke python3 -B')
    started = time.monotonic()
    require(len(sys.argv) == 4, 'usage: run.py SOURCE_MANIFEST SOURCE_SHA FRESH_LABEL')
    overlay_path, overlay_sha, label = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    require(overlay_path == E / 'p228-rope-materialized-source-v1/source-manifest.json'
            and re.fullmatch('[0-9a-f]{64}', overlay_sha)
            and re.fullmatch(r'rope-materialized-cpu-v228-v[1-9][0-9]*', label), 'closed source and output namespace')
    inputs = {}

    def keep(path, expected=None):
        actual = pin(path)
        require(expected is None or actual == expected or
                isinstance(expected, str) and actual['sha256'] == expected, 'input pin: ' + str(path))
        require(str(path) not in inputs or inputs[str(path)] == actual, 'conflicting input identity')
        inputs[str(path)] = actual
        return actual

    def doc(path, expected=None):
        keep(path, expected)
        return json.loads(path.read_bytes())

    controller = keep(Path(__file__).resolve())
    old = load(OLD_RUNNER, OLD_RUNNER_SHA, 'rope_cpu_retained_data')
    keep(OLD_RUNNER, OLD_RUNNER_SHA)
    for path, digest in old.HELPERS:
        keep(path, digest)
    x, bounded, stable = [load(path, digest, 'rope_cpu_helper_' + str(i))
                          for i, (path, digest) in enumerate(old.HELPERS)]
    prior = doc(PRIOR / 'complete.json', PRIOR_SHA)
    require(prior['schema'] == 'ferric-p228-reciprocal-cpu-result-v3'
            and prior['passed'] is True and prior['error'] is None
            and prior['postcheck_error'] is None and prior['source_unchanged'] is True,
            'actual reciprocal CPU prerequisite')
    for name, value in prior['raw'].items():
        require(Path(value['path']) == PRIOR / name, 'original raw path')
        keep(PRIOR / name, value)
    for name, result in prior['phases'].items():
        require(doc(PRIOR / (name + '-result.json')) == result and result['exit_code'] == 0
                and result['reason'] is None and result['group_absent'] is True,
                'original natural completed phase')
        for stream in ('stdout', 'stderr'):
            require(inputs[str(PRIOR / (name + '-' + stream))]['sha256']
                    == result[stream + '_sha256'], 'original phase stream join')
    old_names = set(prior['tests']['arithmetic']['names'])
    require(len(old_names) == 13 and prior['tests']['arithmetic']['passed'] == 12
            and prior['tests']['arithmetic']['ignored'] == 1
            and prior['tests']['exhaustive']['passed'] == 1, 'actual reciprocal census')
    require(old.inventory((PRIOR / 'arithmetic-list-stdout').read_text()) == old_names
            and old.inventory((PRIOR / 'arithmetic-ignored-list-stdout').read_text()) == {EXHAUSTIVE},
            'original named reciprocal inventory')
    for key, names, ignored in (('arithmetic', old_names, {EXHAUSTIVE}),
                                ('exhaustive', {EXHAUSTIVE}, set())):
        actual = old.results((PRIOR / (key + '-stdout')).read_text(), names, ignored)
        require(json.loads(json.dumps(actual)) == prior['tests'][key], 'original actual outcomes')
    prior_sources = doc(PRIOR / 'sources-before.json')
    require(doc(PRIOR / 'sources-after.json') == prior_sources, 'old CPU source equality')
    lower = doc(BASE / 'complete.json', BASE_SHA)
    require(lower['schema'] == 'ferric-p228-reciprocal-checked-probe-result-v7'
            and lower['probe_completed'] is True and lower['error'] is None
            and lower['postcheck_error'] is None and lower['fresh_checked_lowering'] is True
            and lower['fresh_checked_replay'] is True and lower['fresh_hsaco_emitted'] is True
            and lower['candidate_cpu_receipt'] == inputs[str(PRIOR / 'complete.json')], 'actual V7 source lineage')
    maps = {name: doc(BASE / name, digest) for name, digest in BASE_MAPS.items()}
    require(maps['before.json']['fixture'] == maps['after.json']['fixture'], 'V7 fixture changed during lowering')
    baseline = {}
    for name, item in maps['before.json']['fixture'].items():
        path = Path(name)
        require(path.is_relative_to(BASE / 'fixture'), 'V7 fixture path')
        baseline[str(path.relative_to(BASE / 'fixture'))] = {
            key: keep(path, item['pin'])[key] for key in ('bytes', 'sha256')}
    require(set(baseline) == BASE_RUST | {'Cargo.toml', 'Cargo.lock'}
            and x.snapshot(BASE / 'fixture') == baseline, 'exact nine-file V7 fixture')
    lowering_fixture_pins = {name: inputs[str(BASE / 'fixture' / name)] for name in ('Cargo.toml', 'Cargo.lock')}
    fixture_inputs = {name: inputs[str(BASE / 'fixture' / name)] for name in BASE_RUST}
    for name, digest in old.HARNESS_FILES.items():
        fixture_inputs[name] = keep(old.HARNESS / name, digest)
    for name in OLD_TEST_INPUTS:
        fixture_inputs[name] = keep(PRIOR / 'fixture' / name,
                                  dict(path=str(PRIOR / 'fixture' / name), **prior_sources['fixture'][name]))
    for name in ('src/head_rope_numerics_v3.rs', 'src/prefix_reciprocal_numerics_v1.rs'):
        require(baseline[name] == prior_sources['fixture'][name], 'old test macro source compatibility')
    value = doc(overlay_path, overlay_sha)
    changes, candidate_names = proposal(value, baseline)
    for row in changes:
        keep(overlay_path.parent / row['source'], dict(path=str(overlay_path.parent / row['source']), **row['after']))
    require(next(row['after'] for row in changes if row['path'] == 'tests/fixtures/v7_lib.rs')
            == baseline['src/lib.rs'], 'test source fixture is exact original V7 entry')
    formatter = keep(FORMATTER, FORMATTER_SHA)
    for name, digest in stable.TOOLS.items():
        keep(stable.N / 'bin' / name, digest)
    out = E / label
    require(not os.path.lexists(out), 'fresh output required')
    out.mkdir(mode=0o700)
    fixture = out / 'fixture'
    fixture.mkdir(mode=0o700)
    (out / 'tmp').mkdir(mode=0o700)
    bounded.N, bounded.PINS, bounded.F, bounded.T = stable.N, stable.TOOLS, fixture, out / 'target'
    stable.T = bounded.T
    bounded.setup()
    require(not any(bounded.T.iterdir()), 'fresh empty target')
    for name, record in fixture_inputs.items():
        target = fixture / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(Path(record['path']).read_bytes())
    for row in changes:
        path = fixture / row['path']
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('wb' if row['before'] is not None else 'xb') as stream:
            stream.write((overlay_path.parent / row['source']).read_bytes())
    expected = {name: {key: row[key] for key in ('bytes', 'sha256')} for name, row in fixture_inputs.items()}
    expected.update({row['path']: row['after'] for row in changes})
    require(x.snapshot(fixture) == expected, 'exact fixture copy/overlay')
    bounded.save(out / 'sources-unformatted.json', expected)
    provider, config = old.provider_snapshot(x), old.configs(fixture, x)
    original_config = prior_sources['configurations']
    require(all(value == original_config[path] if path in original_config else value is None
                for path, value in config.items()), 'original shared/fresh configurations')
    workspace = {name: keep(old.F / name) for name in ('Cargo.toml', 'Cargo.lock')}
    require(provider == prior_sources['provider'] and workspace == prior_sources['workspace'],
            'unchanged provider53 and workspace')
    for path, record in provider.items():
        keep(Path(path), record)
    env = dict(stable.env(), CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_DEV_OPT_LEVEL='2',
               CARGO_PROFILE_TEST_DEBUG='0', CARGO_PROFILE_DEV_DEBUG='0',
               CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
               CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
               TMPDIR=str(out / 'tmp'), PYTHONDONTWRITEBYTECODE='1')
    phases, tests, dependencies, binaries = {}, {}, {}, {}
    body, formatted, lowering_sources, error = None, {}, {}, None

    def run(name, argv, deadline):
        remaining = 1650 - (time.monotonic() - started)
        require(remaining > 0, 'cumulative CPU budget exhausted')
        try:
            bounded.run(out, name, argv, env=env, deadline=min(deadline, remaining))
        finally:
            if (out / (name + '-result.json')).is_file():
                phases[name] = json.loads((out / (name + '-result.json')).read_bytes())
        if body is not None:
            require(x.snapshot(fixture) == body, 'candidate source or lock changed')
        return (out / (name + '-stdout')).read_text()

    try:
        argv = [formatter['path'], '--edition', '2024', '--config', 'skip_children=true',
                *(str(fixture / name) for name in sorted(FORMAT))]
        run('rustfmt', argv, 60)
        body = x.snapshot(fixture)
        require(set(body) == set(expected)
                and all(body[name] == expected[name] for name in body if name not in FORMAT),
                'formatter changed original source or frozen test fixture')
        run('rustfmt-check', [argv[0], '--check', *argv[1:]], 60)
        bounded.save(out / 'sources-before.json', dict(fixture=body, provider=provider,
                                                      configurations=config, workspace=workspace))
        formatted = {name: pin(fixture / name) for name in sorted(CHANGES)}
        lowering_sources = {name: pin(fixture / name) for name in sorted(BASE_RUST | {'src/prefix_rope_materialized_numerics_v1.rs'})}
        cargo = str(stable.N / 'bin/cargo')
        common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(fixture / 'Cargo.toml')]
        metadata = json.loads(run('metadata', [cargo, 'metadata', *common[:2], '--manifest-path',
                                              str(fixture / 'Cargo.toml'), '--format-version', '1'], 120))
        require(metadata['target_directory'] == str(bounded.T), 'fresh Cargo target')
        historical_metadata = json.loads((PRIOR / 'metadata-stdout').read_bytes())
        def package_identities(value, root):
            rows = []
            for package in value['packages']:
                path = package['manifest_path']
                if path == str(root / 'Cargo.toml'):
                    path = '<numerical-fixture>/Cargo.toml'
                rows.append((package['name'], package['version'], package['source'], path))
            require(len(rows) == len(set(rows)), 'duplicate Cargo package identity')
            return sorted(rows, key=repr)
        require(package_identities(metadata, fixture)
                == package_identities(historical_metadata, PRIOR / 'fixture'),
                'original locked numerical dependency identities')
        local = {}
        for package in metadata['packages']:
            path = Path(package['manifest_path']).resolve(strict=True)
            if package['source'] is None:
                require(path == fixture / 'Cargo.toml' or path.is_relative_to(old.F / 'crates'), 'unexpected local dependency')
                local[package['name']] = str(path)
            else:
                require(package['source'].startswith('registry+') and path.is_relative_to(R / 'toolchain/cargo'),
                        'unexpected cached dependency')
            dependencies[str(path.parent)] = x.snapshot(path.parent)
        require(local.get('fe2o3-device') == str(old.F / 'crates/fe2o3-device/Cargo.toml'), 'original provider')
        bounded.save(out / 'dependencies-before.json', dependencies)
        selected = [part for name in TARGETS for part in ('--test', name)]
        binaries = select_tests(run('build-tests', [cargo, 'test', *common, *selected,
                                                   '--no-run', '--message-format=json'], 1200), fixture, bounded.T)
        for name, names, ignored in ((TARGETS[0], old_names, {EXHAUSTIVE}),
                                     (TARGETS[1], candidate_names, set())):
            executable = binaries[name]['binary']['path']
            require(old.inventory(run(name + '-list', [executable, '--list', '--format', 'terse'], 120)) == names
                    and old.inventory(run(name + '-ignored-list', [executable, '--ignored', '--list', '--format', 'terse'], 120),
                                      allow_empty=True) == ignored, 'exact compiled named inventory')
            tests[name] = old.results(run(name, [executable, '--test-threads=1'], 1200), names, ignored)
        tests['exhaustive'] = old.results(run('exhaustive', [binaries[TARGETS[0]]['binary']['path'],
            '--ignored', '--exact', EXHAUSTIVE, '--test-threads=1'], 1200), {EXHAUSTIVE}, set())
        require(len(phases) == 11 and sum(row['passed'] for row in tests.values()) == 13 + len(candidate_names)
                and sum(row['ignored'] for row in tests.values()) == 1, 'full actual reciprocal/RoPE cohort')
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    post_errors, after, after_dependencies = [], {}, {}
    for name, snapshot in (
            ('fixture', lambda: x.snapshot(fixture)), ('provider', lambda: old.provider_snapshot(x)),
            ('configurations', lambda: old.configs(fixture, x)),
            ('workspace', lambda: {name: pin(old.F / name) for name in workspace})):
        try:
            after[name] = snapshot()
        except BaseException as failure:
            post_errors.append(name + ': ' + type(failure).__name__ + ': ' + str(failure))
    bounded.save(out / 'sources-after.json', after)
    for name in dependencies:
        try:
            after_dependencies[name] = x.snapshot(Path(name))
        except BaseException as failure:
            post_errors.append(name + ': ' + type(failure).__name__ + ': ' + str(failure))
    bounded.save(out / 'dependencies-after.json', after_dependencies)
    for check in (
            lambda: require(body is not None and after == dict(fixture=body, provider=provider,
                configurations=config, workspace=workspace), 'source/provider/config drift'),
            lambda: require(after_dependencies == dependencies, 'dependency drift'),
            lambda: require(x.snapshot(BASE / 'fixture') == baseline, 'original V7 fixture drift'),
            lambda: require(all(pin(Path(path)) == expected for path, expected in inputs.items()), 'consumed input drift'),
            lambda: require(all(pin(Path(row['binary']['path'])) == row['binary'] for row in binaries.values()), 'test ELF drift')):
        try:
            check()
        except BaseException as failure:
            post_errors.append(type(failure).__name__ + ': ' + str(failure))
    passed = error is None and not post_errors
    bounded.save(out / 'inputs.json', inputs)
    raw = {path.name: pin(path) for path in sorted(out.iterdir()) if path.is_file()}
    bounded.save(out / ('complete.json' if passed else 'failed.json'), dict(
        schema='ferric-p228-rope-materialized-cpu-result-v1', passed=passed, error=error,
        postcheck_errors=post_errors, runner=controller, prior_cpu=inputs[str(PRIOR / 'complete.json')],
        prior_lowering=inputs[str(BASE / 'complete.json')], overlay=inputs[str(overlay_path)],
        fixture=str(fixture), formatted_sources=formatted, lowering_sources=lowering_sources,
        lowering_fixture_pins=lowering_fixture_pins,
        tests=tests, phases=phases, binaries=binaries, raw=raw, input_pins=list(inputs.values()),
        tests_passed=sum(row['passed'] for row in tests.values()), tests_ignored=sum(row['ignored'] for row in tests.values()),
        actual_provider_source_sha256=PROVIDER_SHA, source_unchanged=not post_errors,
        toolchain=dict(root=str(stable.N), pins=stable.TOOLS), formatter=formatter,
        cpu_arithmetic_only=True, kernel_entry_host_compiled=False, gpu_execution=False,
        compiler_hsaco_reproduced=False, full_model_acceptance=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False))
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors, output=str(out))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    def terminate(_signal, _frame):
        raise KeyboardInterrupt('owned RoPE CPU runner received termination')
    signal.signal(signal.SIGTERM, terminate)
    sys.exit(main())
