"""Publish actual CPU38 evidence and its full fixture, never install device code."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/silu-materialized-cpu-v1'
C = L / 'proposals/p228-silu-materialized-cpu-v1'
P = L / 'proposals/p228-silu-materialized-kernel-v1'
DEVICE = 'device/qwen3-tp-wave-rmsnorm-kernels-v15'
RUN_SHA = 'c083d3fc80405d10767d1a0d231965e240bdee9953978bf8142e2ad3935c1f82'
PACKAGE_SHA = 'ad306aeb5ecc446ab8d4f0cf48a0f83e4b28626781743adf82332c140e7dac14'
OVERLAY_SHA = 'ca0bafcdd0297d2d849770791cf04686b33f048402f0fb5cc451183217acb276'
PURE_WRAPPER_SHA = '19ee906451b9d68fbc4048176874ce75c9ba9d07fe2de5b6ea9d1fd45460351a'
TARGETS = {'mlp_tiles_numerics_v2': 4, 'mlp_down_two_row_v1': 10,
           'mlp_claimed_numerics_v1': 8, 'mlp_silu_materialized_v1': 16}
PHASES = ('rustfmt', 'rustfmt-check', 'metadata', 'build-tests') + tuple(
    name + suffix for name in TARGETS for suffix in ('-list', '-ignored-list', ''))
SUFFIXES = ('-command.json', '-started.json', '-result.json', '-stdout', '-stderr')
MAPS = {'sources-unformatted.json', 'sources-before.json', 'sources-after.json',
        'dependencies-before.json', 'dependencies-after.json', 'inputs.json'}
FALSE_FLAGS = ('gpu_execution', 'compiler_hsaco_reproduced', 'full_model_acceptance',
               'numerical_acceptance', 'performance_claim', 'production_authority')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.is_file() and path.resolve(strict=True) == path, 'canonical retained file: ' + str(path))
    h = hashlib.sha256()
    with path.open('rb') as stream:
        while raw := stream.read(1 << 20):
            h.update(raw)
    return dict(path=str(path), bytes=path.stat().st_size, sha256=h.hexdigest())


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ and sys.dont_write_bytecode,
            'ordinary bytecode-free data-only publisher')
    require(len(sys.argv) == 4 and re.fullmatch(r'silu-materialized-cpu-v228-v[1-9][0-9]*', sys.argv[1])
            and all(re.fullmatch('[0-9a-f]{64}', arg) for arg in sys.argv[2:]),
            'publish.py CPU_LABEL ACTUAL_CPU_SHA ACTUAL_PURE_SHA')
    root, remote = L / sys.argv[1], E / sys.argv[1]
    consumed, copies = {}, {}

    def checked(path, expected=None):
        actual = pin(path)
        if isinstance(expected, str):
            require(actual['sha256'] == expected, 'SHA: ' + str(path))
        elif expected is not None:
            require(set(expected) == {'path', 'bytes', 'sha256'}
                    and all(actual[key] == expected[key] for key in ('bytes', 'sha256')),
                    'retained original pin: ' + str(path))
        require(str(path) not in consumed or consumed[str(path)] == actual, 'conflicting retained identity')
        consumed[str(path)] = actual
        return actual

    def document(path, expected=None):
        checked(path, expected)
        return json.loads(path.read_bytes())

    def copy(path, destination, original=None):
        actual = checked(path, original)
        require(destination not in copies and not Path(destination).is_absolute()
                and '..' not in Path(destination).parts, 'exclusive publication member')
        copies[destination] = dict(source=path, original=original or actual, retained=actual)

    cpu = document(root / 'complete.json', sys.argv[2])
    require(cpu['schema'] == 'ferric-p228-silu-materialized-cpu-result-v1'
            and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
            and cpu['source_unchanged'] is True and cpu['cpu_arithmetic_only'] is True
            and cpu['tests_passed'] == 38 and cpu['tests_ignored'] == 0
            and cpu['actual_provider_source_sha256'] == '037e472ea24d7337f88028ed65cb3d6ea6e0c0681538e6121ab78240c44ef675'
            and all(cpu[key] is False for key in FALSE_FLAGS), 'successful actual CPU-only38 cohort')
    require(cpu['fixture'] == str(remote / 'fixture') and set(cpu['phases']) == set(PHASES)
            and set(cpu['tests']) == set(cpu['binaries']) == set(TARGETS)
            and set(cpu['raw']) == MAPS | {name + suffix for name in PHASES for suffix in SUFFIXES},
            'four suites, sixteen phases, eighty-six raw records')
    package = document(C / 'manifest.json', PACKAGE_SHA)
    overlay = document(P / 'source-manifest.json', cpu['overlay'])
    checked(P / 'source-manifest.json', OVERLAY_SHA)
    checked(C / 'run.py', RUN_SHA)
    require(cpu['runner'] == dict(pin(C / 'run.py'), path=str(E / C.name / 'run.py'))
            and cpu['overlay']['path'] == str(E / P.name / 'source-manifest.json'), 'actual runner and proposal')
    for name, expected in package['files'].items():
        require({key: checked(C / name)[key] for key in ('bytes', 'sha256')} == expected, 'frozen CPU package body')
        copy(C / name, 'controller/' + name)
    copy(C / 'manifest.json', 'controller/manifest.json')
    copy(P / 'source-manifest.json', 'proposal/source-manifest.json', cpu['overlay'])
    copy(root / 'complete.json', 'complete.json', dict(pin(root / 'complete.json'), path=str(remote / 'complete.json')))
    for name, record in cpu['raw'].items():
        require(record['path'] == str(remote / name), 'exact original raw path')
        checked(root / name, record)
        if name not in ('dependencies-before.json', 'dependencies-after.json'):
            copy(root / name, 'raw/' + name, record)
    before = document(root / 'sources-before.json')
    require(document(root / 'sources-after.json') == before, 'source before/after equality')
    require(document(root / 'dependencies-before.json') == document(root / 'dependencies-after.json'),
            'dependency before/after equality')
    inputs = document(root / 'inputs.json')
    require(sorted(inputs.values(), key=lambda row: row['path']) == sorted(cpu['input_pins'], key=lambda row: row['path'])
            and inputs[cpu['runner']['path']] == cpu['runner'] and inputs[cpu['overlay']['path']] == cpu['overlay'],
            'actual consumed input/source/tool ledger')
    prior = document(L / 'down2-cpu-v228-v1/complete.json', cpu['prior_cpu'])
    require(cpu['prior_cpu']['sha256'] == '96ef991246b90f9f02b509b3302df0215309bb172f9cae4340c21eba27bda563'
            and cpu['prior_lowering']['sha256'] == '0ba363b9b4106e5293e4e6152d719c795c2d58f05e062b6a67570f194e0eb68e',
            'genuine CPU and checked Down2 lineage')
    prior_body = document(L / 'down2-cpu-v228-v1/sources-before.json', prior['raw']['sources-before.json'])['fixture']
    unformatted = document(root / 'sources-unformatted.json')
    additions = {row['path'].removeprefix(DEVICE + '/'): row['after'] for row in overlay['files']}
    require(len(prior_body) == 31 and len(additions) == 3 and not (set(prior_body) & set(additions))
            and unformatted == {**prior_body, **additions}
            and set(before['fixture']) == set(unformatted)
            and all(before['fixture'][name] == expected for name, expected in prior_body.items()),
            'exact small original fixture with only three formatted additions')
    for row in overlay['files']:
        original = dict(path=str(E / P.name / row['source']), **row['after'])
        require(inputs[original['path']] == original and row['before'] is None, 'additive consumed proposal')
        copy(P / row['source'], 'proposal/' + row['source'], original)
    fixture_sources = {}
    for name, expected in before['fixture'].items():
        require(not Path(name).is_absolute() and '..' not in Path(name).parts, 'relative fixture source')
        original = dict(path=str(remote / 'fixture' / name), **expected)
        copy(root / 'fixture' / name, 'fixture/' + name, original)
        fixture_sources[name] = original
    require(len(fixture_sources) == 34 and len(cpu['formatted_sources']) == 3 and len(cpu['lowering_sources']) == 5,
            'full self-contained tested numerical source roster')
    for name, original in cpu['formatted_sources'].items():
        require(original == fixture_sources[name.removeprefix(DEVICE + '/')], 'formatted tested source join')
    mapping = {'src/lib.rs': 'src/finite_mlp_tiles_silu_materialized_v1.rs',
               **{name: name for name in ('src/wave_numerics_v1.rs', 'src/mlp_numerics_v1.rs',
                                          'src/mlp_tile_numerics_v2.rs', 'src/mlp_silu_materialized_numerics_v1.rs')}}
    require(cpu['lowering_sources'] == {name: fixture_sources[source] for name, source in mapping.items()},
            'five exact CPU-tested lowering source inputs')
    require(fixture_sources['src/finite_mlp_tiles_v2.rs']['sha256']
            == overlay['baseline_fixture']['source_pins']['src/lib.rs']['sha256'], 'retained AST baseline bytes')
    expected_names = {name: set(prior['tests'][name]['names']) for name in list(TARGETS)[:2]}
    expected_names['mlp_claimed_numerics_v1'] = set(re.findall(
        r'#\[test\]\s+fn ([a-z0-9_]+)\(', (root / 'fixture/tests/mlp_claimed_numerics_v1.rs').read_text()))
    expected_names['mlp_silu_materialized_v1'] = set(overlay['test_census']['mlp_silu_materialized_v1'])
    selected = cpu['binaries']
    build_messages = [json.loads(line) for line in (root / 'build-tests-stdout').read_text().splitlines() if line.startswith('{')]
    require([row.get('success') for row in build_messages if row.get('reason') == 'build-finished'] == [True], 'actual Cargo build')
    artifacts = [row for row in build_messages if row.get('reason') == 'compiler-artifact' and row.get('target', {}).get('kind') == ['test']]
    require(len(artifacts) == 4 and {row['target']['name'] for row in artifacts} == set(TARGETS), 'four actual Cargo test artifacts')
    for row in artifacts:
        name, profile = row['target']['name'], row['profile']
        require(selected[name]['cargo'] == row and selected[name]['binary']['path'] == row['executable']
                and row['manifest_path'] == str(remote / 'fixture/Cargo.toml')
                and row['target']['src_path'] == str(remote / 'fixture/tests' / (name + '.rs'))
                and profile['test'] is True and profile['opt_level'] == '2'
                and profile['debug_assertions'] is True and profile['overflow_checks'] is True,
                'exact checked opt2 test artifact')
        relative = Path(row['executable']).relative_to(remote / 'target')
        checked(root / 'target' / relative, selected[name]['binary'])
        names = expected_names[name]
        require(len(names) == TARGETS[name] and set(cpu['tests'][name]['names']) == names, 'expected original/new test names')
        listed = re.findall(r'^([A-Za-z0-9_:]+): test$', (root / (name + '-list-stdout')).read_text(), re.MULTILINE)
        ignored = (root / (name + '-ignored-list-stdout')).read_text()
        results = (root / (name + '-stdout')).read_text()
        outcomes = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. ok$', results, re.MULTILINE)
        require(len(listed) == len(outcomes) == len(names) and set(listed) == set(outcomes) == names
                and ': test' not in ignored and ': benchmark' not in ignored
                and re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', results)
                == [(str(len(names)), '0', '0')]
                and cpu['tests'][name]['passed'] == len(names) and cpu['tests'][name]['ignored'] == 0,
                'actual listed individual outcomes and summary')
    stable = Path(cpu['toolchain']['root']) / 'bin'
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(remote / 'fixture/Cargo.toml')]
    fmt = [cpu['formatter']['path'], '--edition', '2024', '--config', 'skip_children=true',
           *(str(remote / 'fixture' / name) for name in sorted(additions))]
    commands = {'rustfmt': fmt, 'rustfmt-check': [fmt[0], '--check', *fmt[1:]],
                'metadata': [str(stable / 'cargo'), 'metadata', *common[:2], '--manifest-path',
                             str(remote / 'fixture/Cargo.toml'), '--format-version', '1'],
                'build-tests': [str(stable / 'cargo'), 'test', *common,
                                *(part for name in TARGETS for part in ('--test', name)), '--no-run', '--message-format=json']}
    for name in TARGETS:
        for suffix, args in (('-list', ['--list', '--format', 'terse']),
                             ('-ignored-list', ['--ignored', '--list', '--format', 'terse']), ('', ['--test-threads=1'])):
            commands[name + suffix] = [selected[name]['binary']['path'], *args]
    for name in PHASES:
        command = document(root / (name + '-command.json'))
        result, start = document(root / (name + '-result.json')), document(root / (name + '-started.json'))
        require(result == cpu['phases'][name] and result['exit_code'] == 0 and result['reason'] is None
                and result['group_absent'] is True and 0 <= result['cache_bytes'] <= 6 << 30
                and set(start) == {'pid', 'pgid'} and type(start['pid']) is int
                and start['pid'] > 0 and start['pgid'] == start['pid'], 'natural owned CPU phase')
        require(command['argv'] == commands[name] and command['tools'] == cpu['toolchain']['pins']
                and command['affinity'] == [8, 9] and command['nice'] == 10
                and command['expected_exit'] == 0 and command['gpu_execution'] is False
                and command['cache_cap_bytes'] == 6 << 30 and 0 < command['deadline_seconds'] <= 1200,
                'actual reviewed phase command and bounds')
        env = command['env']
        require(env['CARGO_TARGET_DIR'] == str(remote / 'target')
                and env['FE2O3_SILU_BASELINE_ENTRY'] == overlay['baseline_fixture']['directory'] + '/src/lib.rs'
                and all(env[key] == '' for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')),
                'fresh target, exact source-contract baseline and hidden GPUs')
        require(all(result[kind + '_sha256'] == cpu['raw'][name + '-' + kind]['sha256'] for kind in ('stdout', 'stderr')),
                'actual stream digest joins')
    pure_dir = L / 'silu-materialized-cpu-pure-v228-v1'
    pure = document(pure_dir / 'complete.json', sys.argv[3])
    require(pure['schema'] == 'ferric-p228-silu-materialized-cpu-pure-v1' and pure['passed'] is True
            and pure['tests'] == 12 and pure['failures'] == pure['errors'] == pure['skipped'] == 0
            and pure['source_postchecks_passed'] is True and pure['synthetic_only'] is True
            and all(pure[key] is False for key in ('rust_compilation', 'gpu_execution', 'numerical_acceptance',
                                                  'performance_claim', 'production_authority')), 'actual pure12 result')
    pure_before = document(pure_dir / 'sources-before.json', pure['raw']['sources-before.json'])
    require(document(pure_dir / 'sources-after.json', pure['raw']['sources-after.json']) == pure_before,
            'pure source before/after equality')
    require(set(pure_before) == {'manifest.json', 'run.py', 'test_run.py', 'README.md', 'controller'}, 'pure source closure')
    for name in ('manifest.json', 'run.py', 'test_run.py', 'README.md'):
        require(pure_before[name]['path'] == str(E / C.name / name), 'original pure package source')
        checked(C / name, pure_before[name])
    wrapper = L / 'run_silu_cpu_pure_p228_v1.py'
    checked(wrapper, PURE_WRAPPER_SHA)
    checked(wrapper, pure['controller'])
    require(pure_before['controller'] == pure['controller'], 'actual pure controller source')
    logs = (pure_dir / 'tests.log').read_text()
    require(set(pure['raw']) == {'sources-before.json', 'sources-after.json', 'tests.log'}, 'pure raw roster')
    pure_names = package['test_census']['test_run.py']
    require(pure['names'] == sorted('test_run.PolicyTests.' + name for name in pure_names)
            and set(re.findall(r'^(test_[a-z0-9_]+) \([^\n]*\) \.\.\. ok$', logs, re.MULTILINE)) == set(pure_names)
            and re.search(r'Ran 12 tests in [^\n]+\n\nOK\n?$', logs), 'actual pure named outcomes')
    for name, original in pure['raw'].items():
        copy(pure_dir / name, 'pure/' + name, original)
    copy(pure_dir / 'complete.json', 'pure/complete.json')
    copy(wrapper, 'pure/controller.py', pure['controller'])
    copy(Path(__file__).resolve(), 'publish.py')
    copy(Path(__file__).resolve().with_name('export.py'), 'export.py')
    require(not Q.is_symlink() and (not Q.exists() or Q.is_dir() and {p.name for p in Q.iterdir()} <= {'README.md'}),
            'fresh publication, optionally root README only')
    if Q.exists() and (Q / 'README.md').exists():
        checked(Q / 'README.md')
    require(all(pin(Path(path)) == record for path, record in consumed.items()), 'all publication inputs remain exact')
    # No write targets are under device/: the qualified fixture is deliberately separate.
    Q.mkdir(parents=True, exist_ok=True)
    ledger = {}
    for name, row in sorted(copies.items()):
        target = Q / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(row['source'].read_bytes())
        actual = pin(target)
        require(all(actual[key] == row['retained'][key] for key in ('bytes', 'sha256')), 'copied byte identity')
        ledger[name] = dict(original=row['original'], retained=actual)
    require(all(pin(Path(path)) == record for path, record in consumed.items()), 'publication input postchecks')
    result = dict(schema='ferric-p228-silu-materialized-cpu-publication-v1', passed=True,
                  actual_cpu=dict(pin(root / 'complete.json'), path=str(remote / 'complete.json')),
                  actual_pure=pin(pure_dir / 'complete.json'), tests_passed=38, tests_ignored=0,
                  synthetic_policy_tests=12, phases=16, fixture_sources=fixture_sources,
                  formatted_sources=cpu['formatted_sources'], lowering_sources=cpu['lowering_sources'],
                  retained_test_executables={name: row['binary'] for name, row in selected.items()},
                  externally_retained_dependencies={name: cpu['raw'][name] for name in
                                                     ('dependencies-before.json', 'dependencies-after.json')},
                  provider_source_sha256=cpu['actual_provider_source_sha256'], files=ledger,
                  publisher=pin(Path(__file__).resolve()), device_sources_modified=False,
                  production_route_modified=False, cpu_arithmetic_only=True, **{key: False for key in FALSE_FLAGS})
    with (Q / 'result.json').open('x') as stream:
        json.dump(result, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(dict(result=pin(Q / 'result.json'), files=len(ledger), tests=38,
                          synthetic_tests=12, device_sources_modified=False)), flush=True)


if __name__ == '__main__':
    main()
