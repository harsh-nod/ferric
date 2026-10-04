"""Publish actual CPU source/test evidence; never import or execute tested code."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/projection-residual-cpu-v1'
P = L / 'proposals/p228-projection-residual-kernel-v1'
C = L / 'proposals/p228-projection-residual-cpu-v1'
OLD = 'qwen3-tp-peer-tp2-kernels-v18'
NEW = 'qwen3-tp-projection-residual-kernels-v1'
MANIFEST_SHA = '08973f7fd8851ad0c3ea628006454f91f2af87e9db156778b6e3de2ea5de865f'
RUNNER_SHA = '6ad7d210014606a278a30be4aff1665719300d67f9790e24a0cc95fa29f2af29'
SUFFIXES = ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')
MAPS = {'sources-unformatted.json', 'old-targets-before.json', 'sources-before.json',
        'dependencies-before.json', 'sources-after.json', 'dependencies-after.json',
        'inputs.json', 'configurations.json', 'old-targets-after.json'}
EXTERNAL_ONLY = {'old-targets-before.json', 'old-targets-after.json',
                 'dependencies-before.json', 'dependencies-after.json'}
COPY_SOURCE = {'Cargo.toml', 'Cargo.lock', 'build.rs', 'src/lib.rs', 'src/collective.rs', 'tests/contract.rs'}


def require(ok, reason):
    if not ok:
        raise RuntimeError(reason)


def pin(path):
    require(path.is_file() and not path.is_symlink() and path.resolve(strict=True) == path,
            'canonical retained file: ' + str(path))
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        while body := stream.read(1 << 20):
            digest.update(body)
    return dict(path=str(path), bytes=path.stat().st_size, sha256=digest.hexdigest())


def checked(path, expected=None, sha=None):
    actual = pin(path)
    if expected is not None:
        require(set(expected) == {'path', 'bytes', 'sha256'}
                and all(actual[k] == expected[k] for k in ('bytes', 'sha256')),
                'retained bytes: ' + str(path))
    if sha is not None:
        require(actual['sha256'] == sha, 'pinned SHA: ' + str(path))
    return actual


def document(path, expected=None, sha=None):
    checked(path, expected, sha)
    return json.loads(path.read_bytes())


def names(raw, empty=False):
    found = re.findall(r'^([A-Za-z0-9_:]+): test$', raw, re.MULTILINE)
    require((found or empty) and len(found) == len(set(found)) and ': benchmark' not in raw,
            'actual test inventory')
    return set(found)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ and sys.dont_write_bytecode,
            'unoptimized data-only publisher with bytecode disabled')
    require(len(sys.argv) == 3 and re.fullmatch(r'projection-residual-cpu-v228-v[1-9][0-9]*', sys.argv[1])
            and re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'CPU_LABEL ACTUAL_COMPLETE_SHA')
    cpu_dir, remote = L / sys.argv[1], E / sys.argv[1]
    cpu_pin = checked(cpu_dir / 'complete.json', sha=sys.argv[2])
    cpu = document(cpu_dir / 'complete.json', cpu_pin)
    require(cpu['schema'] == 'ferric-p228-projection-residual-cpu-result-v1'
            and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
            and cpu['source_unchanged'] is True, 'actual successful CPU completion')
    require(all(cpu[name] is False for name in ('gpu_execution', 'compiler_hsaco_reproduced',
            'numerical_acceptance', 'production_authority', 'performance_claim')), 'CPU-only boundaries')
    require(cpu['formatted_source_root'] == str(remote / 'sources'), 'actual fresh formatted source')
    manifest = document(P / 'source-manifest.json', cpu['source_manifest'], MANIFEST_SHA)
    checked(C / 'run.py', cpu['controller'], RUNNER_SHA)
    require(cpu['source_manifest']['path'] == str(E / P.name / 'source-manifest.json')
            and cpu['controller']['path'] == str(E / C.name / 'run.py'), 'exact original controller/source paths')
    expected = {'lib': set(manifest['test_census']['arithmetic']),
                'contract': set(manifest['test_census']['source_contract'])}
    require(len(expected['lib']) == 12 and len(expected['contract']) == 5, 'authored candidate names')
    phases = {'rustfmt', 'rustfmt-check', 'candidate-metadata', 'control-metadata',
              'candidate-build-tests', 'control-build-tests'}
    phases |= {role + '-' + kind + suffix for role in ('candidate', 'control')
               for kind in ('lib', 'contract') for suffix in ('-list', '-ignored', '-tests')}
    require(len(phases) == 18 and set(cpu['phases']) == phases, 'all eighteen actual phases')
    require(set(cpu['raw']) == MAPS | {name + suffix for name in phases for suffix in SUFFIXES},
            'exact ninety-nine raw leaf/map records')
    retained = {}
    copies = [(cpu_dir / 'complete.json', 'complete.json'),
              (C / 'run.py', 'controller/run.py'), (P / 'source-manifest.json', 'proposal/source-manifest.json')]
    for name, original in cpu['raw'].items():
        require(original['path'] == str(remote / name), 'original direct CPU record')
        retained[name] = checked(cpu_dir / name, original)
        if name not in EXTERNAL_ONLY:
            copies.append((cpu_dir / name, 'raw/' + name))
    inputs = document(cpu_dir / 'inputs.json', retained['inputs.json'])
    for original in (cpu['controller'], cpu['source_manifest']):
        require(inputs.get(original['path']) == original, 'actual source/controller input pin')
    before = document(cpu_dir / 'sources-before.json', retained['sources-before.json'])
    after = document(cpu_dir / 'sources-after.json', retained['sources-after.json'])
    unformatted = document(cpu_dir / 'sources-unformatted.json', retained['sources-unformatted.json'])
    require(before == after and len(before) == 17, 'six candidate/nine old/two sibling source closure')
    for stem in ('old-targets', 'dependencies'):
        require(document(cpu_dir / (stem + '-before.json'), retained[stem + '-before.json'])
                == document(cpu_dir / (stem + '-after.json'), retained[stem + '-after.json']),
                'immutable retained ' + stem)
    require({name.removeprefix(NEW + '/') for name in before if name.startswith(NEW + '/')} == COPY_SOURCE,
            'six actual formatted candidate files')
    require(set(unformatted) == set(before), 'formatting cannot add/remove sources')
    for name, row in before.items():
        require(set(row) == {'bytes', 'sha256'}, 'source snapshot shape')
        if not name.startswith(NEW + '/'):
            require(row == unformatted[name], 'old control/shared target source unchanged')
            suffix = '/' + name
            originals = [p for p in inputs.values() if p['path'].endswith(suffix)]
            require(len(originals) == 1 and all(originals[0][k] == row[k] for k in row), 'actual original source input')
            checked(F / 'device' / name, originals[0])
    for row in manifest['files']:
        require(row['before'] is None and row['source'] == 'draft/' + row['path'], 'only candidate additions')
        source = P / row['source']
        actual = checked(source)
        require({k: actual[k] for k in ('bytes', 'sha256')} == row['after'], 'original proposal body')
        require(inputs.get(str(E / P.name / row['source'])) == dict(actual, path=str(E / P.name / row['source'])),
                'actual original candidate consumption')
        require(unformatted[row['path'].removeprefix('device/')] == row['after'], 'exact candidate preformat body')
    candidate_sources = {}
    for name in sorted(COPY_SOURCE):
        source = cpu_dir / 'sources' / NEW / name
        original = dict(before[NEW + '/' + name], path=str(remote / 'sources' / NEW / name))
        actual = checked(source, original)
        checked(F / 'device' / NEW / name, original)
        candidate_sources[name] = dict(original=original, retained=actual)
        copies.append((source, 'source/' + name))
    require({str(p.relative_to(F / 'device' / NEW)) for p in (F / 'device' / NEW).rglob('*') if p.is_file()} == COPY_SOURCE,
            'root-integrated crate contains exactly the tested six files')
    old_lock = (F / 'device' / OLD / 'Cargo.lock').read_bytes()
    old_name = b'name = "ferric-qwen3-tp-peer-tp2-kernels-device-v18"'
    new_name = b'name = "ferric-qwen3-tp-projection-residual-kernels-device-v1"'
    require(old_lock.count(old_name) == 1 and new_name not in old_lock
            and (F / 'device' / NEW / 'Cargo.lock').read_bytes() == old_lock.replace(old_name, new_name),
            'only lock root name changes; dependency bytes preserved')
    stable = Path('/home/harmenon/ferric-asrock-42/toolchain/rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin')
    target = remote / 'target'
    for phase in sorted(phases):
        command = document(cpu_dir / (phase + '-command.json'), retained[phase + '-command.json'])
        started = document(cpu_dir / (phase + '-started.json'), retained[phase + '-started.json'])
        result = document(cpu_dir / (phase + '-result.json'), retained[phase + '-result.json'])
        require(result == cpu['phases'][phase] and result['exit_code'] == 0 and result['reason'] is None
                and result['group_absent'] is True and 0 <= result['cache_bytes'] <= 6 << 30,
                'natural successful bounded owned phase: ' + phase)
        require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int
                and started['pid'] > 0 and started['pid'] == started['pgid'], 'actual owned PID/PGID')
        require(all(result[key + '_sha256'] == retained[phase + '-' + key]['sha256']
                    for key in ('stdout', 'stderr')), 'actual stream/result joins')
        require(command['affinity'] == [8, 9] and command['nice'] == 10 and command['expected_exit'] == 0
                and command['gpu_execution'] is False and command['cache_cap_bytes'] == 6 << 30,
                'unchanged CPU phase policy')
        env = command['env']
        require(all(env[key] == '' for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'))
                and env['CARGO_TARGET_DIR'] == str(target) and env['CARGO_BUILD_JOBS'] == '2'
                and env['RUSTC'] == str(stable / 'rustc') and env['RUSTDOC'] == str(stable / 'rustdoc'),
                'fresh target/pinned stable tool/GPU-hidden environment')
        if phase.startswith('rustfmt'):
            args = [str(stable / 'rustfmt')]
            if phase == 'rustfmt-check':
                args.append('--check')
            args += ['--edition', '2024', '--config', 'skip_children=true']
            args += [str(remote / 'sources' / NEW / name) for name in
                     ('build.rs', 'src/lib.rs', 'src/collective.rs', 'tests/contract.rs')]
            deadline = 60
        else:
            role = phase.split('-', 1)[0]
            crate = remote / 'sources' / (NEW if role == 'candidate' else OLD)
            if phase.endswith('-metadata'):
                args = [str(stable / 'cargo'), 'metadata', '--offline', '--locked', '--manifest-path', str(crate / 'Cargo.toml'), '--format-version', '1']
                deadline = 120
            elif phase.endswith('-build-tests'):
                args = [str(stable / 'cargo'), 'test', '--offline', '--locked', '--jobs', '2', '--target-dir', str(target), '--manifest-path', str(crate / 'Cargo.toml'), '--lib', '--test', 'contract', '--no-run', '--message-format=json']
                deadline = 1800
            else:
                kind, operation = phase.split('-')[1:]
                args = [cpu['binaries'][role][kind]['binary']['path']]
                args += {'list': ['--list', '--format', 'terse'], 'ignored': ['--ignored', '--list', '--format', 'terse'], 'tests': ['--test-threads=1']}[operation]
                deadline = 300 if operation == 'tests' else 120
        require(command['argv'] == args and command['deadline_seconds'] == deadline, 'exact retained CPU recipe')
    total = 0
    require(set(cpu['tests']) == set(cpu['binaries']) == {'candidate', 'control'}, 'both actual test crates')
    for role in ('candidate', 'control'):
        require(set(cpu['tests'][role]) == set(cpu['binaries'][role]) == {'lib', 'contract'}, 'two actual test kinds')
        build = [json.loads(line) for line in (cpu_dir / (role + '-build-tests-stdout')).read_text().splitlines() if line.startswith('{')]
        require([r['success'] for r in build if r.get('reason') == 'build-finished'] == [True], 'real successful Cargo build')
        for kind, row in cpu['tests'][role].items():
            stem = role + '-' + kind
            actual_names = names((cpu_dir / (stem + '-list-stdout')).read_text())
            require(not names((cpu_dir / (stem + '-ignored-stdout')).read_text(), True), 'no ignored tests')
            require(row == dict(passed=len(actual_names), ignored=0, names=sorted(actual_names)), 'actual inventory/count join')
            if role == 'candidate':
                require(actual_names == expected[kind], 'full seventeen candidate tests')
            stream = (cpu_dir / (stem + '-tests-stdout')).read_text()
            found = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. ok$', stream, re.MULTILINE)
            require(len(found) == len(actual_names) and set(found) == actual_names
                    and re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', stream)
                    == [(str(len(actual_names)), '0', '0')], 'every actual test passed once')
            artifact = cpu['binaries'][role][kind]
            require(artifact['artifact'] in build and artifact['artifact']['executable'] == artifact['binary']['path']
                    and Path(artifact['binary']['path']).is_relative_to(target), 'test execution uses selected Cargo ELF')
            profile = artifact['artifact']['profile']
            require(profile['test'] is True and profile['opt_level'] == '0' and profile['debug_assertions'] is True
                    and profile['overflow_checks'] is True, 'checked ordinary test profile')
            total += row['passed']
    require(cpu['test_count'] == total, 'total comes from all actual named outcomes')
    copies.append((Path(__file__).resolve(), 'publish.py'))
    source_pins = {str(path): checked(path) for path, _ in copies}
    require(len({dest for _, dest in copies}) == len(copies), 'exclusive publication destinations')
    readme = None
    if os.path.lexists(Q):
        require(Q.is_dir() and not Q.is_symlink() and Q.resolve(strict=True) == Q
                and {p.name for p in Q.iterdir()} == {'README.md'}, 'only root-authored README may preexist')
        readme = pin(Q / 'README.md')
    for path, _ in copies:
        require(pin(path) == source_pins[str(path)], 'recheck all copied bytes before publication')
    Q.mkdir(parents=True, exist_ok=True)
    ledger = {}
    for path, name in copies:
        dest = Q / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        with dest.open('xb') as stream:
            stream.write(path.read_bytes())
        copied = checked(dest, source_pins[str(path)])
        ledger[name] = {k: copied[k] for k in ('bytes', 'sha256')}
    if readme is not None:
        require(pin(Q / 'README.md') == readme, 'root README unchanged')
    result = dict(schema='ferric-p228-projection-residual-cpu-publication-v1', passed=True,
        original_cpu_complete=dict(cpu_pin, path=str(remote / 'complete.json')),
        retained_cpu_complete=cpu_pin, controller=cpu['controller'], source_manifest=cpu['source_manifest'],
        candidate_tests=sum(r['passed'] for r in cpu['tests']['candidate'].values()),
        control_tests=sum(r['passed'] for r in cpu['tests']['control'].values()), tests=cpu['tests'],
        actual_phase_count=len(phases), candidate_sources=candidate_sources,
        omitted_large_maps={name: dict(original=cpu['raw'][name], retained=retained[name]) for name in sorted(EXTERNAL_ONLY)},
        declared_test_artifacts=cpu['binaries'], file_ledger=ledger, root_readme=readme,
        compiler_hsaco_reproduced=False, gpu_execution=False, numerical_acceptance=False,
        production_authority=False, performance_claim=False,
        interpretation='Actual bounded CPU arithmetic/source regression qualification only; no GPU or model acceptance.')
    with (Q / 'result.json').open('x') as stream:
        json.dump(result, stream, sort_keys=True, indent=2)
        stream.write('\n')
    print(json.dumps(dict(output=str(Q), result=pin(Q / 'result.json'), files=len(ledger), tests=total)))


if __name__ == '__main__':
    main()
