"""Retain actual RoPE CPU arithmetic evidence without compiling or installing code."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
Q = Path('/home/harsh/ferric-p227-integration/qualification/rope-materialized-cpu-v1')
C = L / 'proposals/p228-rope-materialized-cpu-v1'
P = L / 'proposals/p228-rope-materialized-source-v1'
RUN_SHA = 'cb3d019f07d79899ea9885fe5a9424265ec8bcb74074631535080a58bdceac22'
OVERLAY_SHA = '728edfbfaa684279fc603c03356682ebda0e3f3e72a604d53670d5427ac94837'
PRIOR = 'reciprocal-cpu-v228-v3'
PRIOR_SHA = '6b02d358eca33d31d6839e9c28de38276b4f968cfaec93b7ddb97963a77956d3'
BASE = 'row-reciprocal-checked-probe-v228-v7'
BASE_SHA = '6ba25826b71e30f5106e1efc9e786c021c56f5bab397add20b365373685081a9'
PROVIDER_SHA = '037e472ea24d7337f88028ed65cb3d6ea6e0c0681538e6121ab78240c44ef675'
TARGETS = {'prefix_reciprocal_v1': 13, 'rope_materialized_v1': 20}
EXHAUSTIVE = 'exhaustive_all_significands_in_one_binade'
PHASES = ('rustfmt', 'rustfmt-check', 'metadata', 'build-tests') + tuple(
    name + suffix for name in TARGETS for suffix in ('-list', '-ignored-list', '')) + ('exhaustive',)
SUFFIXES = ('-command.json', '-started.json', '-result.json', '-stdout', '-stderr')
MAPS = {'sources-unformatted.json', 'sources-before.json', 'sources-after.json',
        'dependencies-before.json', 'dependencies-after.json', 'inputs.json'}
BASE_RUST = {'src/lib.rs', 'src/wave_numerics_v1.rs', 'src/head_rope_numerics_v3.rs',
             'src/prefix_reciprocal_numerics_v1.rs', 'src/attention_online.rs',
             'src/output_projection_numerics_v5.rs', 'src/prefix_tiles_numerics_v6.rs'}
CHANGES = {'src/lib.rs', 'src/prefix_rope_materialized_numerics_v1.rs',
           'tests/rope_materialized_v1.rs', 'tests/fixtures/v7_lib.rs'}
FORMATTED = CHANGES - {'tests/fixtures/v7_lib.rs'}
HARNESS = {'Cargo.toml', 'Cargo.lock', 'src/fixture_lib.rs', 'tests/prefix_reciprocal_v1.rs',
           'tests/fixtures/prefix_reciprocal_fraction_v1.tsv'}
FALSE_FLAGS = ('kernel_entry_host_compiled', 'gpu_execution', 'compiler_hsaco_reproduced',
               'full_model_acceptance', 'numerical_acceptance', 'performance_claim', 'production_authority')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.is_absolute() and path.is_file() and path.resolve(strict=True) == path,
            'canonical retained file: ' + str(path))
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        while block := stream.read(1 << 20):
            digest.update(block)
    return dict(path=str(path), bytes=path.stat().st_size, sha256=digest.hexdigest())


def outcomes(text, names, ignored):
    rows = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|ignored)(?:, [^\n]*)?$', text, re.M)
    require(len(rows) == len(names) and {name for name, _ in rows} == names
            and {name for name, status in rows if status == 'ignored'} == ignored,
            'actual individual test outcomes')
    require(re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
            == [(str(len(names) - len(ignored)), '0', str(len(ignored)))], 'actual test summary')


def inventory(text):
    rows = re.findall(r'^([A-Za-z0-9_:]+): test$', text, re.M)
    require(len(rows) == len(set(rows)) and ': benchmark' not in text, 'ordinary unique test inventory')
    return set(rows)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ and sys.dont_write_bytecode,
            'ordinary bytecode-free publisher')
    require(len(sys.argv) == 3 and re.fullmatch('[0-9a-f]{64}', sys.argv[2]),
            'publish.py ACTUAL_CPU_COMPLETE_PATH ACTUAL_CPU_SHA')
    supplied = Path(sys.argv[1])
    require(supplied.name == 'complete.json' and supplied.parent.parent in (L, E)
            and re.fullmatch(r'rope-materialized-cpu-v228-v[1-9][0-9]*', supplied.parent.name),
            'closed actual completion namespace')
    root, remote = L / supplied.parent.name, E / supplied.parent.name
    consumed, copies = {}, {}

    def checked(path, expected=None):
        actual = pin(path)
        if isinstance(expected, str):
            require(actual['sha256'] == expected, 'SHA: ' + str(path))
        elif expected is not None:
            require(set(expected) == {'path', 'bytes', 'sha256'}
                    and all(actual[key] == expected[key] for key in ('bytes', 'sha256')),
                    'retained original bytes: ' + str(path))
        require(str(path) not in consumed or consumed[str(path)] == actual, 'unchanged consumed identity')
        consumed[str(path)] = actual
        return actual

    def document(path, expected=None):
        record = checked(path, expected)
        require(record['bytes'] <= 16 << 20, 'bounded JSON')
        return json.loads(path.read_bytes())

    def copy(path, destination, original=None):
        actual = checked(path, original)
        require(destination not in copies and not Path(destination).is_absolute()
                and '..' not in Path(destination).parts and actual['bytes'] <= 4 << 20,
                'exclusive small publication member')
        copies[destination] = dict(source=path, original=original or actual, retained=actual)

    cpu = document(root / 'complete.json', sys.argv[2])
    require(cpu['schema'] == 'ferric-p228-rope-materialized-cpu-result-v1'
            and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
            and cpu['source_unchanged'] is True and cpu['cpu_arithmetic_only'] is True
            and cpu['tests_passed'] == 33 and cpu['tests_ignored'] == 1
            and cpu['actual_provider_source_sha256'] == PROVIDER_SHA
            and all(cpu[key] is False for key in FALSE_FLAGS), 'actual CPU-only arithmetic result')
    require(cpu['fixture'] == str(remote / 'fixture') and set(cpu['phases']) == set(PHASES)
            and set(cpu['tests']) == set(TARGETS) | {'exhaustive'} and set(cpu['binaries']) == set(TARGETS)
            and set(cpu['raw']) == MAPS | {name + suffix for name in PHASES for suffix in SUFFIXES},
            'eleven phases, sixty-one raw records, two binaries')
    overlay = document(P / 'source-manifest.json', OVERLAY_SHA)
    runner = checked(C / 'run.py', RUN_SHA)
    require(cpu['runner'] == dict(runner, path=str(E / C.name / 'run.py'))
            and cpu['overlay'] == dict(pin(P / 'source-manifest.json'), path=str(E / P.name / 'source-manifest.json')),
            'actual executed controller and source proposal')
    for name in ('run.py', 'README.md'):
        copy(C / name, 'controller/' + name)
    copy(P / 'source-manifest.json', 'proposal/source-manifest.json', cpu['overlay'])
    copy(root / 'complete.json', 'complete.json', dict(pin(root / 'complete.json'), path=str(remote / 'complete.json')))
    for name, record in cpu['raw'].items():
        require(record['path'] == str(remote / name), 'original raw path')
        checked(root / name, record)
        if name not in ('dependencies-before.json', 'dependencies-after.json'):
            copy(root / name, 'raw/' + name, record)
    before = document(root / 'sources-before.json')
    require(document(root / 'sources-after.json') == before, 'source before/after equality')
    require(document(root / 'dependencies-before.json') == document(root / 'dependencies-after.json'),
            'dependency snapshot equality')
    inputs = document(root / 'inputs.json')
    require(all(path == record['path'] for path, record in inputs.items())
            and sorted(inputs.values(), key=lambda row: row['path']) == sorted(cpu['input_pins'], key=lambda row: row['path'])
            and inputs[cpu['runner']['path']] == cpu['runner'] and inputs[cpu['overlay']['path']] == cpu['overlay'],
            'recorded consumed-input identities')
    prior = document(L / PRIOR / 'complete.json', PRIOR_SHA)
    require(cpu['prior_cpu'] == dict(pin(L / PRIOR / 'complete.json'), path=str(E / PRIOR / 'complete.json')),
            'actual reciprocal CPU lineage')
    base = document(L / BASE / 'complete.json', BASE_SHA)
    require(cpu['prior_lowering'] == dict(pin(L / BASE / 'complete.json'), path=str(E / BASE / 'complete.json'))
            and base['candidate_cpu_receipt'] == cpu['prior_cpu'], 'actual checked V7 lineage')
    prior_fixture = document(L / PRIOR / 'sources-before.json', prior['raw']['sources-before.json'])['fixture']
    baseline = overlay['baseline_fixture']['files']
    require(overlay['schema'] == 'ferric-p228-rope-materialized-source-proposal-v1'
            and set(baseline) == BASE_RUST | {'Cargo.toml', 'Cargo.lock'}
            and len(overlay['files']) == 4 and {row['path'] for row in overlay['files']} == CHANGES,
            'original V7 plus four explicit source entries')
    expected = {name: baseline[name] for name in BASE_RUST}
    expected.update({name: prior_fixture[name] for name in HARNESS})
    for row in overlay['files']:
        require(row['source'] == 'draft/' + row['path'] and row['before'] == baseline.get(row['path']),
                'original preimage or additive absence')
        original = dict(path=str(E / P.name / row['source']), **row['after'])
        require(inputs[original['path']] == original, 'consumed authored proposal source')
        copy(P / row['source'], 'proposal/' + row['source'], original)
        expected[row['path']] = row['after']
    require(len(expected) == 15 and document(root / 'sources-unformatted.json') == expected
            and set(before['fixture']) == set(expected)
            and all(before['fixture'][name] == item for name, item in expected.items() if name not in FORMATTED)
            and before['fixture']['tests/fixtures/v7_lib.rs'] == baseline['src/lib.rs'],
            'only three candidate bodies formatted within exact fifteen-file fixture')
    fixture_sources = {}
    for name, item in before['fixture'].items():
        original = dict(path=str(remote / 'fixture' / name), **item)
        copy(root / 'fixture' / name, 'fixture/' + name, original)
        fixture_sources[name] = original
    require(cpu['formatted_sources'] == {name: fixture_sources[name] for name in CHANGES}
            and cpu['lowering_sources'] == {name: fixture_sources[name] for name in BASE_RUST | {'src/prefix_rope_materialized_numerics_v1.rs'}},
            'four formatted and eight exact lowering source pins')
    require(set(cpu['lowering_fixture_pins']) == {'Cargo.toml', 'Cargo.lock'}, 'original lowering Cargo pair')
    for name, original in cpu['lowering_fixture_pins'].items():
        require(original == dict(path=str(E / BASE / 'fixture' / name), **baseline[name]), 'checked Cargo source identity')
        copy(L / BASE / 'fixture' / name, 'lowering-fixture/' + name, original)
    expected_names = {'prefix_reciprocal_v1': set(prior['tests']['arithmetic']['names']),
                      'rope_materialized_v1': set(overlay['test_census']['rope_materialized_v1'])}
    messages = [json.loads(line) for line in (root / 'build-tests-stdout').read_text().splitlines() if line.startswith('{')]
    require([row.get('success') for row in messages if row.get('reason') == 'build-finished'] == [True], 'successful Cargo build')
    artifacts = [row for row in messages if row.get('reason') == 'compiler-artifact' and row.get('target', {}).get('kind') == ['test']]
    require(len(artifacts) == 2 and {row['target']['name'] for row in artifacts} == set(TARGETS), 'two actual test artifacts')
    local_elfs, absent_elfs = {}, {}
    for artifact in artifacts:
        name, profile = artifact['target']['name'], artifact['profile']
        selected = cpu['binaries'][name]
        require(selected['cargo'] == artifact and selected['binary']['path'] == artifact['executable']
                and artifact['manifest_path'] == str(remote / 'fixture/Cargo.toml')
                and artifact['target']['src_path'] == str(remote / 'fixture/tests' / (name + '.rs'))
                and profile['test'] is True and profile['opt_level'] == '2'
                and profile['debug_assertions'] is True and profile['overflow_checks'] is True,
                'actual checked opt2 test artifact')
        relative = Path(artifact['executable']).relative_to(remote / 'target')
        path = root / 'target' / relative
        if path.exists() or path.is_symlink():
            local_elfs[name] = checked(path, selected['binary'])
        else:
            absent_elfs[name] = selected['binary']
        names = expected_names[name]
        ignored = {EXHAUSTIVE} if name == 'prefix_reciprocal_v1' else set()
        require(len(names) == TARGETS[name] and len(cpu['tests'][name]['names']) == len(names)
                and set(cpu['tests'][name]['names']) == names
                and inventory((root / (name + '-list-stdout')).read_text()) == names
                and inventory((root / (name + '-ignored-list-stdout')).read_text()) == ignored,
                'exact original and new compiled inventories')
        outcomes((root / (name + '-stdout')).read_text(), names, ignored)
        require(cpu['tests'][name]['passed'] == len(names) - len(ignored)
                and cpu['tests'][name]['ignored'] == len(ignored), 'receipt ordinary outcomes')
    outcomes((root / 'exhaustive-stdout').read_text(), {EXHAUSTIVE}, set())
    require(cpu['tests']['exhaustive']['names'] == [EXHAUSTIVE]
            and cpu['tests']['exhaustive']['passed'] == 1 and cpu['tests']['exhaustive']['ignored'] == 0,
            'separate explicit exhaustive execution')
    stable = Path(cpu['toolchain']['root']) / 'bin'
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(remote / 'fixture/Cargo.toml')]
    fmt = [cpu['formatter']['path'], '--edition', '2024', '--config', 'skip_children=true',
           *(str(remote / 'fixture' / name) for name in sorted(FORMATTED))]
    commands = {'rustfmt': fmt, 'rustfmt-check': [fmt[0], '--check', *fmt[1:]],
                'metadata': [str(stable / 'cargo'), 'metadata', *common[:2], '--manifest-path',
                             str(remote / 'fixture/Cargo.toml'), '--format-version', '1'],
                'build-tests': [str(stable / 'cargo'), 'test', *common,
                                *(part for name in TARGETS for part in ('--test', name)), '--no-run', '--message-format=json'],
                'exhaustive': [cpu['binaries']['prefix_reciprocal_v1']['binary']['path'],
                               '--ignored', '--exact', EXHAUSTIVE, '--test-threads=1']}
    for name in TARGETS:
        for suffix, args in (('-list', ['--list', '--format', 'terse']),
                             ('-ignored-list', ['--ignored', '--list', '--format', 'terse']), ('', ['--test-threads=1'])):
            commands[name + suffix] = [cpu['binaries'][name]['binary']['path'], *args]
    for name in PHASES:
        command = document(root / (name + '-command.json'))
        result, started = document(root / (name + '-result.json')), document(root / (name + '-started.json'))
        require(result == cpu['phases'][name] and result['exit_code'] == 0 and result['reason'] is None
                and result['group_absent'] is True and 0 <= result['cache_bytes'] <= 6 << 30
                and set(started) == {'pid', 'pgid'} and type(started['pid']) is int
                and started['pid'] > 0 and started['pgid'] == started['pid'], 'natural owned phase')
        require(command['argv'] == commands[name] and command['tools'] == cpu['toolchain']['pins']
                and command['affinity'] == [8, 9] and command['nice'] == 10
                and command['expected_exit'] == 0 and command['gpu_execution'] is False
                and command['cache_cap_bytes'] == 6 << 30 and 0 < command['deadline_seconds'] <= 1200,
                'actual command and unchanged bounded CPU envelope')
        require(command['env']['CARGO_TARGET_DIR'] == str(remote / 'target')
                and all(command['env'][key] == '' for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'))
                and all(result[key + '_sha256'] == cpu['raw'][name + '-' + key]['sha256'] for key in ('stdout', 'stderr')),
                'fresh target, hidden GPUs and exact stream joins')
    copy(Path(__file__).resolve(), 'publish.py')
    require(Q.parent.resolve(strict=True) == Q.parent and not Q.is_symlink()
            and (not Q.exists() or Q.is_dir() and {path.name for path in Q.iterdir()} <= {'README.md'}),
            'fresh publication except root README')
    if Q.exists() and (Q / 'README.md').exists():
        checked(Q / 'README.md')
    require(all(pin(Path(path)) == record for path, record in consumed.items()), 'all read inputs unchanged before copy')
    Q.mkdir(exist_ok=True)
    ledger = {}
    for name, row in sorted(copies.items()):
        target = Q / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(row['source'].read_bytes())
        actual = pin(target)
        require(all(actual[key] == row['retained'][key] for key in ('bytes', 'sha256')), 'published byte identity')
        ledger[name] = dict(original=row['original'], retained=actual)
    require(all(pin(Path(path)) == record for path, record in consumed.items()), 'publication input postchecks')
    result = dict(schema='ferric-p228-rope-materialized-cpu-publication-v1', passed=True,
        actual_cpu=dict(pin(root / 'complete.json'), path=str(remote / 'complete.json')),
        tests_passed=33, tests_ignored=1, explicit_exhaustive_passed=1, phases=11,
        fixture_sources=fixture_sources, formatted_sources=cpu['formatted_sources'],
        lowering_sources=cpu['lowering_sources'], lowering_fixture_pins=cpu['lowering_fixture_pins'],
        remote_test_executables={name: row['binary'] for name, row in cpu['binaries'].items()},
        locally_rehashed_test_executables=local_elfs, test_executable_bodies_not_locally_rehashed=absent_elfs,
        externally_retained_dependencies={name: cpu['raw'][name] for name in ('dependencies-before.json', 'dependencies-after.json')},
        locally_rehashed_inputs=list(consumed.values()), all_transitive_input_bodies_rehashed=False,
        provider_source_sha256=PROVIDER_SHA, files=ledger, publisher=pin(Path(__file__).resolve()),
        device_sources_modified=False, production_route_modified=False, cpu_arithmetic_only=True,
        **{key: False for key in FALSE_FLAGS})
    with (Q / 'result.json').open('x') as stream:
        json.dump(result, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(dict(result=pin(Q / 'result.json'), files=len(ledger), tests=33,
                          explicit_exhaustive_passed=1, device_sources_modified=False)), flush=True)


if __name__ == '__main__':
    main()
