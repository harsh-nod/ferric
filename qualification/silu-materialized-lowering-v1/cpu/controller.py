"""CPU-only SiLU candidate qualification over the actual small Down2 fixture."""
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
DEVICE = 'device/qwen3-tp-wave-rmsnorm-kernels-v15'
BASE = E / 'down2-cpu-v228-v1'
PRIOR_SHA = '96ef991246b90f9f02b509b3302df0215309bb172f9cae4340c21eba27bda563'
LOWERING = E / 'row-down2-checked-probe-v228-v1'
LOWERING_SHA = '0ba363b9b4106e5293e4e6152d719c795c2d58f05e062b6a67570f194e0eb68e'
OVERLAY_SHA = 'ca0bafcdd0297d2d849770791cf04686b33f048402f0fb5cc451183217acb276'
OLD_RUNNER = E / 'p228-down2-cpu-v1/run.py'
OLD_RUNNER_SHA = 'a48976422c3eba35308d3f210022bcd2b736dcd9928f7b4f40d9f417293f9f24'
FORMATTER = R / 'toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/bin/rustfmt'
FORMATTER_SHA = 'a9137d0c198ceb6c72193d517d3c9007b3ec7a90d3d10ec6889773eca48261b4'
PROVIDER_SHA = '037e472ea24d7337f88028ed65cb3d6ea6e0c0681538e6121ab78240c44ef675'
TARGETS = ('mlp_tiles_numerics_v2', 'mlp_down_two_row_v1',
           'mlp_claimed_numerics_v1', 'mlp_silu_materialized_v1')
CLAIMED_TESTS = {
    'both_projection_handlers_preserve_all_rows_and_only_lane_zero_writes',
    'rejected_projection_and_down_still_execute_every_collective',
    'down_keeps_fp32_low_bits_and_meets_independent_fp64_bound',
    'specialized_down_matches_every_active_queued_iteration_and_overflow_status',
    'norm_preserves_two_bf16_rounds_and_all_lane_coverage',
    'norm_rejects_collective_invalid_math_and_local_write_failure',
    'swiglu_full_coverage_stable_expression_and_zero_signs',
    'swiglu_invalid_inputs_exp_and_writes_finish_all_components',
}
NEW_PATHS = {
    'src/finite_mlp_tiles_silu_materialized_v1.rs',
    'src/mlp_silu_materialized_numerics_v1.rs',
    'tests/mlp_silu_materialized_v1.rs',
}
LOWERING_MAP = {
    'src/lib.rs': 'src/finite_mlp_tiles_silu_materialized_v1.rs',
    'src/wave_numerics_v1.rs': 'src/wave_numerics_v1.rs',
    'src/mlp_numerics_v1.rs': 'src/mlp_numerics_v1.rs',
    'src/mlp_tile_numerics_v2.rs': 'src/mlp_tile_numerics_v2.rs',
    'src/mlp_silu_materialized_numerics_v1.rs': 'src/mlp_silu_materialized_numerics_v1.rs',
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    path = Path(path)
    require(path.is_absolute() and path.is_file() and not path.is_symlink()
            and path.resolve(strict=True) == path, 'canonical regular input: ' + str(path))
    h = hashlib.sha256()
    with path.open('rb') as stream:
        while block := stream.read(1 << 20):
            h.update(block)
    return dict(path=str(path), bytes=path.stat().st_size, sha256=h.hexdigest())


def load(path, digest, name):
    require(pin(path)['sha256'] == digest, 'helper hash: ' + str(path))
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(path.read_bytes(), str(path), 'exec'), module.__dict__)
    return module


def plain_python():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ,
            'ordinary Python is required by the unchanged bounded helper')
    require(sys.dont_write_bytecode, 'use python3 -B')


def source_proposal(value):
    require(value['schema'] == 'ferric-p228-silu-materialized-kernel-proposal-v1'
            and value['status'] == 'authored-not-executed', 'source-only candidate proposal')
    changes = value['files']
    require(len(changes) == 3 and {row['path'] for row in changes}
            == {DEVICE + '/' + name for name in NEW_PATHS}, 'exact three additive files')
    for row in changes:
        require(row['before'] is None and row['source'] == 'draft/' + row['path'],
                'additive mirrored source only')
    census = value['test_census']
    require(set(census) == {TARGETS[-1]} and len(census[TARGETS[-1]]) == 16
            and len(set(census[TARGETS[-1]])) == 16
            and all(re.fullmatch(r'[a-z][a-z0-9_]*', name) for name in census[TARGETS[-1]]),
            'exact sixteen candidate test names')
    return changes, set(census[TARGETS[-1]])


def prior_contract(value):
    require(value['schema'] == 'ferric-p228-down2-cpu-result-v1' and value['passed'] is True
            and value['error'] is None and value['postcheck_error'] is None
            and value['source_unchanged'] is True
            and value['actual_provider_source_sha256'] == PROVIDER_SHA
            and value['tests_passed'] == 14 and value['tests_ignored'] == 0,
            'actual completed Down2 CPU prerequisite')
    require(set(value['tests']) == set(TARGETS[:2]) and len(value['phases']) == 8,
            'original two-suite CPU cohort')
    for name, count in zip(TARGETS[:2], (4, 10)):
        row = value['tests'][name]
        require(row['passed'] == count and row['ignored'] == 0
                and len(row['names']) == len(set(row['names'])) == count,
                'complete original named test roster')


def format_transition(before, after):
    require(set(before) == set(after) and NEW_PATHS <= set(after), 'format source roster changed')
    require(all(before[name] == after[name] for name in before if name not in NEW_PATHS),
            'formatter changed an existing Down2 file')


def select_tests(raw, fixture, target):
    messages = [json.loads(line) for line in raw.splitlines() if line.startswith('{')]
    require([row.get('success') for row in messages if row.get('reason') == 'build-finished'] == [True],
            'one successful Cargo build required')
    selected = {}
    for row in messages:
        if row.get('reason') != 'compiler-artifact' or row.get('target', {}).get('kind') != ['test']:
            continue
        name, profile = row['target']['name'], row['profile']
        require(name in TARGETS and name not in selected and row['manifest_path'] == str(fixture / 'Cargo.toml')
                and row['target']['src_path'] == str(fixture / 'tests' / (name + '.rs'))
                and profile['test'] is True and profile['opt_level'] == '2'
                and profile['debug_assertions'] is True and profile['overflow_checks'] is True,
                'exact opt2 checked CPU test artifact')
        binary = Path(row['executable'])
        require(binary.is_relative_to(target) and os.access(binary, os.X_OK), 'fresh target executable')
        selected[name] = dict(binary=pin(binary), cargo=row)
    require(set(selected) == set(TARGETS), 'all four actual test binaries required')
    return selected


def main():
    plain_python()
    started = time.monotonic()
    require(len(sys.argv) == 3, 'usage: run.py CANDIDATE_SOURCE_MANIFEST NEW_LABEL')
    overlay_path, label = Path(sys.argv[1]), sys.argv[2]
    require(overlay_path == E / 'p228-silu-materialized-kernel-v1/source-manifest.json'
            and re.fullmatch(r'silu-materialized-cpu-v228-v[1-9][0-9]*', label), 'closed inputs and fresh label')
    inputs = {}

    def keep(path, expected=None):
        actual = pin(path)
        if isinstance(expected, str):
            require(actual['sha256'] == expected, 'input digest: ' + str(path))
        elif expected is not None:
            require(actual == expected, 'input pin: ' + str(path))
        require(str(path) not in inputs or inputs[str(path)] == actual, 'conflicting input pins')
        inputs[str(path)] = actual
        return actual

    def doc(path, expected=None):
        keep(path, expected)
        return json.loads(path.read_bytes())

    controller = keep(Path(__file__).resolve())
    keep(OLD_RUNNER, OLD_RUNNER_SHA)
    old = load(OLD_RUNNER, OLD_RUNNER_SHA, 'silu_cpu_retained_down2')
    for path, digest in old.HELPERS:
        keep(path, digest)
    x, bounded, stable = [load(path, digest, 'silu_cpu_helper_' + str(i))
                           for i, (path, digest) in enumerate(old.HELPERS)]
    prior = doc(BASE / 'complete.json', PRIOR_SHA)
    prior_contract(prior)
    for name, value in prior['raw'].items():
        require(Path(value['path']) == BASE / name, 'original raw path')
        keep(BASE / name, value)
    for name, result in prior['phases'].items():
        require(doc(BASE / (name + '-result.json')) == result and result['exit_code'] == 0
                and result['reason'] is None and result['group_absent'] is True,
                'original natural completed phase')
        for kind in ('stdout', 'stderr'):
            require(inputs[str(BASE / (name + '-' + kind))]['sha256'] == result[kind + '_sha256'],
                    'original result stream join')
    for name in TARGETS[:2]:
        names = old.check_inventory((BASE / (name + '-list-stdout')).read_text(),
                                    (BASE / (name + '-ignored-list-stdout')).read_text(),
                                    set(prior['tests'][name]['names']))
        observed = old.results((BASE / (name + '-stdout')).read_text(), names, set())
        require(json.loads(json.dumps(observed)) == prior['tests'][name], 'original actual named outcomes')
    original_snapshot = doc(BASE / 'sources-before.json')
    require(doc(BASE / 'sources-after.json') == original_snapshot, 'original source snapshots differ')
    base_body = original_snapshot['fixture']
    require(len(base_body) == 31 and x.snapshot(BASE / 'fixture') == base_body, 'exact retained small fixture')
    for name, expected in base_body.items():
        keep(BASE / 'fixture' / name, dict(path=str(BASE / 'fixture' / name), **expected))
    lowering = doc(LOWERING / 'complete.json', LOWERING_SHA)
    require(lowering['schema'] == 'ferric-p228-down2-lowering-result-v1'
            and lowering['passed'] is True and lowering['error'] is None
            and not lowering['postcheck_errors'] and lowering['fresh_checked_lowering'] is True
            and lowering['candidate_cpu'] == inputs[str(BASE / 'complete.json')], 'actual Down2 source lineage')
    for name in ('src/lib.rs', 'src/wave_numerics_v1.rs', 'src/mlp_numerics_v1.rs', 'src/mlp_tile_numerics_v2.rs'):
        baseline_name = 'src/finite_mlp_tiles_v2.rs' if name == 'src/lib.rs' else name
        keep(LOWERING / 'fixture' / name, dict(path=str(LOWERING / 'fixture' / name), **base_body[baseline_name]))
    value = doc(overlay_path, OVERLAY_SHA)
    changes, new_names = source_proposal(value)
    expected_tests = {name: set(prior['tests'][name]['names']) for name in TARGETS[:2]}
    expected_tests.update({TARGETS[2]: CLAIMED_TESTS, TARGETS[3]: new_names})
    for row in changes:
        keep(overlay_path.parent / row['source'], dict(path=str(overlay_path.parent / row['source']), **row['after']))
        require(row['path'][len(DEVICE) + 1:] not in base_body, 'new source already exists')
    formatter = keep(FORMATTER, FORMATTER_SHA)
    for name, digest in stable.TOOLS.items():
        keep(stable.N / 'bin' / name, digest)
    out = E / label
    require(not os.path.lexists(out), 'fresh output directory required')
    out.mkdir(mode=0o700)
    fixture = out / 'fixture'
    fixture.mkdir(mode=0o700)
    (out / 'tmp').mkdir(mode=0o700)
    bounded.N, bounded.PINS, bounded.F, bounded.T = stable.N, stable.TOOLS, fixture, out / 'target'
    stable.T = bounded.T
    bounded.setup()
    require(not any(bounded.T.iterdir()), 'fresh empty target')
    for name in base_body:
        target = fixture / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write((BASE / 'fixture' / name).read_bytes())
    for row in changes:
        with (fixture / row['path'][len(DEVICE) + 1:]).open('xb') as stream:
            stream.write((overlay_path.parent / row['source']).read_bytes())
    unformatted = x.snapshot(fixture)
    require(unformatted == {**base_body, **{row['path'][len(DEVICE) + 1:]: row['after'] for row in changes}},
            'only three additive candidate source copies')
    bounded.save(out / 'sources-unformatted.json', unformatted)
    provider, config = old.provider_snapshot(x), old.configs(fixture, x)
    original_config = original_snapshot['configurations']
    require(all(value == original_config[path] if path in original_config else value is None
                for path, value in config.items()), 'original shared configurations or fresh-path config changed')
    workspace = {name: keep(old.F / name) for name in ('Cargo.toml', 'Cargo.lock')}
    require(provider == original_snapshot['provider'] and workspace == original_snapshot['workspace'],
            'original provider53 and workspace remain exact')
    for path, value in provider.items():
        keep(Path(path), value)
    env = dict(stable.env(), CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_DEV_OPT_LEVEL='2',
               CARGO_PROFILE_TEST_DEBUG='0', CARGO_PROFILE_DEV_DEBUG='0',
               CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
               CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
               TMPDIR=str(out / 'tmp'), PYTHONDONTWRITEBYTECODE='1',
               FE2O3_SILU_BASELINE_ENTRY=str(LOWERING / 'fixture/src/lib.rs'))
    phases, tests, dependencies, binaries = {}, {}, {}, {}
    body, formatted, lowering_sources, error = None, {}, {}, None

    def run(name, argv, deadline):
        remaining = 1650 - (time.monotonic() - started)
        require(remaining > 0, 'cumulative CPU phase budget exhausted')
        try:
            bounded.run(out, name, argv, env=env, deadline=min(deadline, remaining))
        finally:
            result_path = out / (name + '-result.json')
            if result_path.is_file():
                phases[name] = json.loads(result_path.read_bytes())
        if body is not None:
            require(x.snapshot(fixture) == body, 'candidate source or lock changed')
        return (out / (name + '-stdout')).read_text()

    try:
        argv = [formatter['path'], '--edition', '2024', '--config', 'skip_children=true',
                *(str(fixture / name) for name in sorted(NEW_PATHS))]
        run('rustfmt', argv, 60)
        body = x.snapshot(fixture)
        format_transition(unformatted, body)
        run('rustfmt-check', [argv[0], '--check', *argv[1:]], 60)
        bounded.save(out / 'sources-before.json', dict(fixture=body, provider=provider,
                                                      configurations=config, workspace=workspace))
        formatted = {DEVICE + '/' + name: pin(fixture / name) for name in sorted(NEW_PATHS)}
        lowering_sources = {name: pin(fixture / source) for name, source in LOWERING_MAP.items()}
        cargo = str(stable.N / 'bin/cargo')
        common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(fixture / 'Cargo.toml')]
        metadata = json.loads(run('metadata', [cargo, 'metadata', *common[:2], '--manifest-path',
                                              str(fixture / 'Cargo.toml'), '--format-version', '1'], 120))
        require(metadata['target_directory'] == str(bounded.T), 'metadata selected the fresh target')
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
        require(local.get('fe2o3-device') == str(old.F / 'crates/fe2o3-device/Cargo.toml'), 'original provider path')
        bounded.save(out / 'dependencies-before.json', dependencies)
        selected = [part for name in TARGETS for part in ('--test', name)]
        binaries = select_tests(run('build-tests', [cargo, 'test', *common, *selected,
                                                   '--no-run', '--message-format=json'], 1200), fixture, bounded.T)
        for name in TARGETS:
            executable = binaries[name]['binary']['path']
            names = old.check_inventory(run(name + '-list', [executable, '--list', '--format', 'terse'], 120),
                                        run(name + '-ignored-list', [executable, '--ignored', '--list', '--format', 'terse'], 120),
                                        expected_tests[name])
            tests[name] = old.results(run(name, [executable, '--test-threads=1'], 1200), names, set())
        require(len(phases) == 16 and sum(row['passed'] for row in tests.values()) == 38
                and all(row['ignored'] == 0 for row in tests.values()), 'complete actual 14+8+16 test cohort')
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    post_errors = []
    after = {}
    for name, snapshot in (
            ('fixture', lambda: x.snapshot(fixture)),
            ('provider', lambda: old.provider_snapshot(x)),
            ('configurations', lambda: old.configs(fixture, x)),
            ('workspace', lambda: {name: pin(old.F / name) for name in workspace})):
        try:
            after[name] = snapshot()
        except BaseException as failure:
            post_errors.append(name + ': ' + type(failure).__name__ + ': ' + str(failure))
    bounded.save(out / 'sources-after.json', after)
    after_dependencies = {}
    for name in dependencies:
        try:
            after_dependencies[name] = x.snapshot(Path(name))
        except BaseException as failure:
            post_errors.append('dependency ' + name + ': ' + type(failure).__name__ + ': ' + str(failure))
    bounded.save(out / 'dependencies-after.json', after_dependencies)
    checks = (
        lambda: require(body is not None and after == dict(fixture=body, provider=provider,
                                                          configurations=config, workspace=workspace), 'source/provider/config drift'),
        lambda: require(after_dependencies == dependencies, 'dependency drift'),
        lambda: require(x.snapshot(BASE / 'fixture') == base_body, 'original fixture drift'),
        lambda: require(all(pin(Path(path)) == expected for path, expected in inputs.items()), 'consumed input drift'),
        lambda: require(all(pin(Path(row['binary']['path'])) == row['binary'] for row in binaries.values()), 'test ELF drift'),
    )
    for check in checks:
        try:
            check()
        except BaseException as failure:
            post_errors.append(type(failure).__name__ + ': ' + str(failure))
    passed = error is None and not post_errors
    bounded.save(out / 'inputs.json', inputs)
    raw = {path.name: pin(path) for path in sorted(out.iterdir()) if path.is_file()}
    bounded.save(out / ('complete.json' if passed else 'failed.json'), dict(
        schema='ferric-p228-silu-materialized-cpu-result-v1', passed=passed, error=error,
        postcheck_errors=post_errors, runner=controller, prior_cpu=inputs[str(BASE / 'complete.json')],
        prior_lowering=inputs[str(LOWERING / 'complete.json')], overlay=inputs[str(overlay_path)],
        fixture=str(fixture), formatted_sources=formatted, lowering_sources=lowering_sources,
        tests=tests, phases=phases, binaries=binaries, raw=raw, input_pins=list(inputs.values()),
        tests_passed=sum(row['passed'] for row in tests.values()), tests_ignored=sum(row['ignored'] for row in tests.values()),
        actual_provider_source_sha256=PROVIDER_SHA, source_unchanged=not post_errors,
        toolchain=dict(root=str(stable.N), pins=stable.TOOLS), formatter=formatter,
        cpu_arithmetic_only=True, gpu_execution=False, compiler_hsaco_reproduced=False,
        full_model_acceptance=False, numerical_acceptance=False, performance_claim=False, production_authority=False))
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors, output=str(out))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    def terminate(_signal, _frame):
        raise KeyboardInterrupt('owned SiLU CPU runner received termination')
    signal.signal(signal.SIGTERM, terminate)
    sys.exit(main())
