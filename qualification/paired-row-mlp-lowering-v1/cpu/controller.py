"""Bounded CPU-only Down2 arithmetic/source tests with the retained provider53."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
F = R / 'fe2o3'
DEVICE = 'device/qwen3-tp-wave-rmsnorm-kernels-v15'
SOURCES = E / 'clock-recorder-source-inputs-v228-v1.json'
SOURCES_SHA = '055b5b6995f199dd81560734ade5f04f2970534cb3aae62dee47c4f9b149a6ff'
BASE_COMMIT = '924703865d9cfcb1dc791af211b7558ff7c58894'
ARCHIVE = dict(path=str(E / 'clock-recorder-ferric-92470386-v228-v1.tar.gz'),
               bytes=20399448, sha256='07942e8a9f9f2b3bf9fc2a57cfa49414d7a37dc23d8a849cdde5c57c5994792a')
HELPERS = (
    (E / 'run_clean_worker_p228_v1.py', '2f3ef5c80e4483ac1c1a1ebb2bbacf18af7e7b263d13d965fa991525d85e6d2a'),
    (R / 'evidence/wave-output-lowering-v216/bounded.py', 'e634e1b3be3b122b2231ad134c770f25d7d71d10ec767d61a16d03e726bdf4f1'),
    (E / 'metadata_p220_v2.py', '5b5efdaf64053b49c3acb78a82a1ffeaa948a4095b689d2f24e0dff8778ee816'),
)
OVERLAY_SHA = '26ac584bc95b4455d5a5d5a3d9503d9aa3de9250a5d22bba42ed8a5a6d811691'
HARNESS = E / 'row-prefix-tiles-provider-v227-v9/numerical-fixture'
HARNESS_FILES = {
    'Cargo.toml': '9db18beef4cdb3f477f0a0de8214bda95541f1b45a84552aa0218c3687a6e188',
    'Cargo.lock': '93282dd447e56780f5c830431520ce3b101d52db6c14c83db4432f1ea122b445',
    'src/fixture_lib.rs': '1e16c6e8c0fb1251ae9fab205acbdabb4c505e0032f00e8880d717593511186f',
}
TARGETS = ('mlp_tiles_numerics_v2', 'mlp_down_two_row_v1')
BASE_TESTS = {
    'every_tile_retains_lane_product_order_and_exact_narrowing_boundaries',
    'tile_arithmetic_matches_v1_inner_macros_for_adversarial_inputs',
    'rejected_tiles_still_execute_every_remaining_row_collective',
    'source_changes_only_outer_row_bounds_and_reuses_old_norm_swiglu',
}
OVERLAY_PATHS = {
    DEVICE + '/src/finite_mlp_tiles_v2.rs',
    DEVICE + '/src/mlp_tile_numerics_v2.rs',
    DEVICE + '/tests/mlp_down_two_row_v1.rs',
}
PROVIDER_SHA = '037e472ea24d7337f88028ed65cb3d6ea6e0c0681538e6121ab78240c44ef675'


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        while block := stream.read(1 << 20):
            digest.update(block)
    return digest.hexdigest()


def load(path, expected, name):
    require(path.is_file() and path.resolve(strict=True) == path and not path.is_symlink()
            and sha(path) == expected, 'retained helper mismatch: ' + str(path))
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(path.read_bytes(), str(path), 'exec'), module.__dict__)
    return module


def inventory(raw, allow_empty=False):
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', raw, re.MULTILINE)
    require((names or allow_empty) and len(names) == len(set(names)) and ': benchmark' not in raw,
            'missing/duplicate/unexpected test inventory')
    return set(names)


def results(raw, names, ignored):
    rows = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|ignored(?:, [^\n]*)?)$', raw, re.MULTILINE)
    require(len(rows) == len({name for name, _ in rows}) and {name for name, _ in rows} == names,
            'actual outcomes must match observed test inventory')
    actual_ignored = {name for name, state in rows if state.startswith('ignored')}
    require(actual_ignored == ignored and ignored <= names, 'ignored test selection changed')
    summaries = [tuple(map(int, row)) for row in re.findall(
        r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw)]
    require(summaries == [(len(names - ignored), 0, len(ignored))], 'actual test summary mismatch')
    return dict(passed=len(names - ignored), ignored=len(ignored), names=sorted(names), summaries=summaries)


def select_tests(raw, fixture, target, pin):
    messages = [json.loads(line) for line in raw.splitlines() if line.startswith('{')]
    require([row.get('success') for row in messages if row.get('reason') == 'build-finished'] == [True],
            'one successful actual Cargo build required')
    selected = {}
    for row in messages:
        if row.get('reason') != 'compiler-artifact' or row.get('target', {}).get('kind') != ['test']:
            continue
        name, profile = row['target']['name'], row['profile']
        require(name in TARGETS and name not in selected and row['manifest_path'] == str(fixture / 'Cargo.toml')
                and profile['test'] is True and profile['opt_level'] == '2'
                and profile['debug_assertions'] is True and profile['overflow_checks'] is True,
                'exact optimized checked arithmetic test artifact')
        binary = Path(row['executable'])
        require(binary.is_relative_to(target) and binary.resolve(strict=True) == binary and os.access(binary, os.X_OK),
                'actual test executable outside fresh target')
        selected[name] = dict(binary=pin(binary), cargo=row)
    require(set(selected) == set(TARGETS), 'both actual selected test binaries required')
    return selected


def provider_snapshot(x):
    root = F / 'crates/fe2o3-device'
    files = [root / 'Cargo.toml', *sorted((root / 'src').rglob('*'))]
    files = [path for path in files if path.is_file()]
    if (root / 'build.rs').exists():
        files.append(root / 'build.rs')
    require(len(files) == 53, 'retained provider roster')
    digest, pins = hashlib.sha256(b'FE2O3/WORKGROUP-SYNC-PROVIDER-SOURCE-CLOSURE/V1\0'), {}
    for path in sorted(files, key=lambda path: str(path.relative_to(root))):
        require(path.resolve(strict=True) == path and not path.is_symlink(), 'provider alias')
        pins[str(path)] = x.pin(path)
        for raw in (str(path.relative_to(root)).encode(), path.read_bytes()):
            digest.update(len(raw).to_bytes(8, 'little'))
            digest.update(raw)
    require(digest.hexdigest() == PROVIDER_SHA, 'retained provider source changed')
    return pins


def configs(fixture, x):
    paths = {directory / '.cargo' / name for directory in (fixture, *fixture.parents)
             for name in ('config', 'config.toml')}
    paths.update(R / 'toolchain/cargo' / name for name in ('config', 'config.toml'))
    return {str(path): x.pin(path) if os.path.lexists(path) else None for path in sorted(paths)}


def proposal(overlay):
    require(overlay['schema'] == 'ferric-p228-mlp-down-two-row-source-manifest-v1'
            and overlay['base_commit'] == BASE_COMMIT and overlay['tests_executed'] is False
            and overlay['gpu_execution'] is False and overlay['compiler_changed'] is False
            and overlay['provider_changed'] is False, 'source-only Down2 proposal')
    changes = overlay['files']
    require(len(changes) == 3 and {row['path'] for row in changes} == OVERLAY_PATHS,
            'exact three-file Down2 roster')
    for row in changes:
        require(row['source'] == 'draft/' + row['path'], 'exact mirrored source path')
        require((row['before'] is None) == (row['path'] == DEVICE + '/tests/mlp_down_two_row_v1.rs'),
                'only the Down2 test file is new')
    names = overlay['tests_authored_not_run']
    require(len(names) == len(set(names)) == 10
            and all(isinstance(name, str) and re.fullmatch(r'[A-Za-z0-9_]+', name) for name in names),
            'ten exact authored Down2 test names')
    return changes, {TARGETS[0]: set(BASE_TESTS), TARGETS[1]: set(names)}


def check_inventory(raw, ignored_raw, expected):
    names = inventory(raw)
    require(names == expected and not inventory(ignored_raw, allow_empty=True),
            'complete exact test inventory with no ignored tests')
    return names


def main():
    require(len(sys.argv) == 3 and not sys.flags.optimize, 'usage: run.py OVERLAY_MANIFEST NEW_LABEL')
    overlay_path, label = Path(sys.argv[1]), sys.argv[2]
    require(overlay_path.is_relative_to(E) and overlay_path.resolve(strict=True) == overlay_path
            and isinstance(OVERLAY_SHA, str) and sha(overlay_path) == OVERLAY_SHA,
            'actual formatted Down2 proposal manifest must be frozen')
    require(re.fullmatch(r'down2-cpu-v228-v[1-9][0-9]*', label), 'owned fresh label')
    require(sha(SOURCES) == SOURCES_SHA, 'retained published source inputs')
    x, bounded, stable = [load(path, digest, 'down2_cpu_' + str(i))
                           for i, (path, digest) in enumerate(HELPERS)]
    controller = x.pin(Path(__file__).resolve())
    inputs, overlay = json.loads(SOURCES.read_text()), json.loads(overlay_path.read_text())
    require(inputs['schema'] == 'ferric-p228-clean-worker-sources-v1'
            and inputs['archives']['ferric']['commit'] == BASE_COMMIT
            and {key: inputs['archives']['ferric'][key] for key in ARCHIVE} == ARCHIVE,
            'exact current Ferric archive baseline')
    changes, expected_tests = proposal(overlay)
    out = E / label
    out.mkdir(mode=0o700)
    source, fixture = out / 'sources', out / 'fixture'
    source.mkdir(mode=0o700)
    fixture.mkdir(mode=0o700)
    bounded.N, bounded.PINS, bounded.F, bounded.T = stable.N, stable.TOOLS, fixture, out / 'target'
    stable.T = bounded.T
    bounded.setup()
    (out / 'tmp').mkdir(mode=0o700)
    require(not any(bounded.T.iterdir()), 'empty initial target required')
    archive = inputs['archives']['ferric']
    require(Path(archive['path']).is_relative_to(E)
            and Path(archive['path']).resolve(strict=True) == Path(archive['path']), 'owned source archive')
    require(x.pin(Path(archive['path'])) == {key: archive[key] for key in ('path', 'bytes', 'sha256')}, 'published archive pin')
    extraction = x.extract(Path(archive['path']), 'ferric', source)
    before = x.snapshot(source)
    bounded.save(out / 'sources-base.json', before)
    for row in changes:
        relative = Path(row['path'])
        require(relative.is_relative_to(DEVICE) and '..' not in relative.parts, 'device-only overlay')
        replacement = overlay_path.parent / 'draft' / relative
        require(replacement.resolve(strict=True) == replacement and not replacement.is_symlink(),
                'canonical regular overlay body')
        actual = x.pin(replacement)
        require({key: actual[key] for key in ('bytes', 'sha256')} == row['after'], 'replacement pin')
        require(before.get('ferric/' + row['path']) == row['before'], 'exact preimage/new-file absence')
    for row in changes:
        path = source / 'ferric' / row['path']
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb' if row['before'] is None else 'wb') as stream:
            stream.write((overlay_path.parent / 'draft' / row['path']).read_bytes())
    installed = x.snapshot(source)
    require(installed == {**before, **{'ferric/' + row['path']: row['after'] for row in changes}}, 'only three Down2 source changes')
    device = source / 'ferric' / DEVICE
    for name in ('src', 'tests'):
        shutil.copytree(device / name, fixture / name)
    for name, digest in HARNESS_FILES.items():
        path = HARNESS / name
        require(sha(path) == digest and path.resolve(strict=True) == path, 'retained CPU harness pin')
        with (fixture / name).open('xb') as stream:
            stream.write(path.read_bytes())
    body = x.snapshot(fixture)
    provider = provider_snapshot(x)
    config = configs(fixture, x)
    workspace = {name: x.pin(F / name) for name in ('Cargo.toml', 'Cargo.lock')}
    bounded.save(out / 'sources-before.json', dict(source=installed, fixture=body, provider=provider, configurations=config, workspace=workspace))
    env = dict(stable.env(), CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_DEV_OPT_LEVEL='2',
               CARGO_PROFILE_TEST_DEBUG='0', CARGO_PROFILE_DEV_DEBUG='0',
               CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
               TMPDIR=str(out / 'tmp'), PYTHONDONTWRITEBYTECODE='1')
    cargo = str(stable.N / 'bin/cargo')
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(fixture / 'Cargo.toml')]
    phases, tests, dependencies, binaries = {}, {}, {}, {}
    def run(name, argv, deadline=1200):
        phases[name] = bounded.run(out, name, argv, env=env, deadline=deadline)
        require(x.snapshot(source) == installed and x.snapshot(fixture) == body, 'candidate source or lock drift')
        return (out / (name + '-stdout')).read_text()
    error = None
    try:
        metadata = json.loads(run('metadata', [cargo, 'metadata', *common[:2], '--manifest-path', str(fixture / 'Cargo.toml'), '--format-version', '1'], 120))
        require(metadata['target_directory'] == str(bounded.T), 'fresh metadata target')
        local = {}
        for package in metadata['packages']:
            path = Path(package['manifest_path']).resolve(strict=True)
            if package['source'] is None:
                require(path == fixture / 'Cargo.toml' or path.is_relative_to(F / 'crates'), 'unexpected local dependency')
                local[package['name']] = str(path)
            else:
                require(package['source'].startswith('registry+') and path.is_relative_to(R / 'toolchain/cargo'), 'unexpected cached dependency')
            dependencies[str(path.parent)] = x.snapshot(path.parent)
        require(local.get('fe2o3-device') == str(F / 'crates/fe2o3-device/Cargo.toml'), 'actual current provider required')
        bounded.save(out / 'dependencies-before.json', dependencies)
        selected = [part for name in TARGETS for part in ('--test', name)]
        raw = run('build-tests', [cargo, 'test', *common, *selected, '--no-run', '--message-format=json'])
        binaries = select_tests(raw, fixture, bounded.T, x.pin)
        for name in TARGETS:
            executable = binaries[name]['binary']['path']
            names = check_inventory(
                run(name + '-list', [executable, '--list', '--format', 'terse'], 120),
                run(name + '-ignored-list', [executable, '--ignored', '--list', '--format', 'terse'], 120),
                expected_tests[name],
            )
            tests[name] = results(run(name, [executable, '--test-threads=1']), names, set())
        require(sum(row['passed'] for row in tests.values()) == 14
                and all(row['ignored'] == 0 for row in tests.values()) and len(phases) == 8,
                'actual full old4 plus new10 cohort')
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    post_error = None
    try:
        require(x.snapshot(source) == installed and x.snapshot(fixture) == body and provider_snapshot(x) == provider
                and configs(fixture, x) == config
                and {name: x.pin(F / name) for name in workspace} == workspace, 'source/provider/configuration drift')
        after_dependencies = {name: x.snapshot(Path(name)) for name in dependencies}
        bounded.save(out / 'dependencies-after.json', after_dependencies)
        require(after_dependencies == dependencies, 'dependency source drift')
        require(sha(SOURCES) == SOURCES_SHA and sha(overlay_path) == OVERLAY_SHA, 'input manifest drift')
        require(x.pin(Path(archive['path'])) == {key: archive[key] for key in ('path', 'bytes', 'sha256')}, 'archive drift')
        for path, digest in HELPERS:
            require(sha(path) == digest, 'helper drift')
        for name, digest in stable.TOOLS.items():
            require(sha(stable.N / 'bin' / name) == digest, 'toolchain drift')
        for row in changes:
            replacement = x.pin(overlay_path.parent / 'draft' / row['path'])
            require({key: replacement[key] for key in ('bytes', 'sha256')} == row['after'], 'overlay input drift')
        for name, digest in HARNESS_FILES.items():
            require(sha(HARNESS / name) == digest, 'retained CPU harness drift')
        require(x.pin(Path(__file__).resolve()) == controller, 'runner source drift')
        for row in binaries.values():
            require(x.pin(Path(row['binary']['path'])) == row['binary'], 'test binary drift')
        bounded.save(out / 'sources-after.json', dict(source=installed, fixture=body, provider=provider, configurations=config, workspace=workspace))
    except BaseException as failure:
        post_error = type(failure).__name__ + ': ' + str(failure)
    passed = error is None and post_error is None
    raw = {path.name: x.pin(path) for path in sorted(out.iterdir()) if path.is_file()}
    bounded.save(out / ('complete.json' if passed else 'failed.json'), dict(
        schema='ferric-p228-down2-cpu-result-v1', passed=passed, error=error, postcheck_error=post_error,
        runner=controller, source_inputs=x.pin(SOURCES), overlay=x.pin(overlay_path),
        extraction=extraction, binaries=binaries, tests=tests, phases=phases, raw=raw,
        tests_passed=sum(row['passed'] for row in tests.values()),
        tests_ignored=sum(row['ignored'] for row in tests.values()),
        actual_provider_source_sha256=PROVIDER_SHA, authored_test_names=sorted(expected_tests[TARGETS[1]]),
        toolchain=dict(root=str(stable.N), pins=stable.TOOLS), source_unchanged=post_error is None,
        cpu_arithmetic_only=True, gpu_execution=False, compiler_hsaco_reproduced=False,
        full_model_acceptance=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False))
    print(json.dumps(dict(passed=passed, error=error, postcheck_error=post_error, output=str(out))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    def terminate(_signal, _frame):
        raise KeyboardInterrupt('owned Down2 CPU runner received termination')
    signal.signal(signal.SIGTERM, terminate)
    sys.exit(main())
