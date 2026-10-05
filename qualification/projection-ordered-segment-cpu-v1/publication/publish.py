"""Root-executed local integration/publication of an actual successful CPU cohort."""
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
PL = L / 'proposals'
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOTS = {'ferric': Path('/home/harsh/ferric-p227-integration'), 'fe2o3': Path('/home/harsh/fe2o3-p228-runtime')}
OUT = ROOTS['ferric'] / 'qualification/projection-ordered-segment-cpu-v1'
PACKAGE = 'p228-projection-ordered-segment-cpu-v2'
CONTROLLER_SHA = 'c5fd9ef33af9f6bdae2adac608f69a7d88a969e1385210f8c65c371a86077d6f'
TEST_SHA = '467c50323665a333c43ef7108ef962342cdf8f3154213ca1a1c3d02bc110dd7a'
README_SHA = 'b6cd46d58e7b3c66d9d963609286d78567f8c8c637b7b26875ee1b3c9d3afd40'
EXPORT_SHA = 'e896894416338d969af6162fabf82be56fab704b8248105b5405988c5809b9fb'
PRIMARY = L / 'projection-ordered-segment-preparation-primary-v228-v2.json'
PRIMARY_SHA = '7371c32eeed9508ad075a7ea84b301bc25f3694b984afb9c3e6dce80f110cadb'
OLD_SHA = 'deb2aeacded08c336d1fb5a1638cd2accc2b65ec65ffa3e90177bd5e61146d74'
SOURCE_SHA = 'ed98abb445b2dc3209fcd575bfb25e817b86b7bce0dbdb5f299984067149f3b4'
MAPS = {'sources-base.json', 'sources-unformatted.json', 'sources-before.json',
        'sources-after.json', 'old-targets-before.json', 'old-targets-after.json', 'configurations.json'}
TAILS = ('-command.json', '-started.json', '-result.json', '-stdout', '-stderr')
WORKER = 'ferric-tp-peer-finite-engineering-worker-v1'
OLD_BINARIES = {'ferric-qwen3-finite-projection-residual-decode' + suffix
                for suffix in ('-engineering', '-host-engineering', '-shared-host-engineering')} | {WORKER}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pairs(items):
    value = {}
    for key, item in items:
        require(key not in value, 'duplicate JSON key')
        value[key] = item
    return value


def stamp(value):
    return tuple(getattr(value, key) for key in
        ('st_dev', 'st_ino', 'st_mode', 'st_uid', 'st_gid', 'st_nlink', 'st_size', 'st_mtime_ns', 'st_ctime_ns'))


def read(path):
    require(path.resolve(strict=True) == path, 'canonical local ordinary path')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= 32 << 20, 'bounded ordinary metadata/source body')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'opened body identity')
        body = stream.read((32 << 20) + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'stable opened body')
    require(stamp(path.lstat()) == stamp(before) and len(body) == before.st_size, 'stable named body')
    return body, dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def git(root, *args):
    value = subprocess.run(['git', *args], cwd=root, check=True, capture_output=True, text=True, timeout=30)
    return value.stdout


def main():
    require(len(sys.argv) == 3 and sys.dont_write_bytecode and not sys.flags.optimize,
            'python -B publish.py RETAINED_COMPLETE_PATH ACTUAL_SHA256')
    complete = Path(sys.argv[1]); case = complete.parent
    require(complete.is_relative_to(W) and complete.name == 'complete.json'
            and re.fullmatch(r'projection-ordered-segment-cpu-v228-v(?:[2-9]|[1-9][0-9]+)', case.name)
            and re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'actual retained original-path CPU completion')
    remote = E / case.name
    consumed, copies = {}, {}

    def checked(path, expected=None):
        body, pin = read(path)
        require(expected is None or pin == {k: expected[k] for k in ('bytes', 'sha256')}, 'actual local bytes: ' + str(path))
        require(path not in consumed or consumed[path] == pin, 'repeated local identity')
        consumed[path] = pin
        return body

    def document(path, expected=None):
        return json.loads(checked(path, expected), object_pairs_hook=pairs)

    def copy(path, destination, original=None, expected=None):
        body = checked(path, expected)
        require(destination not in copies and not Path(destination).is_absolute()
                and '..' not in Path(destination).parts, 'unique closed publication member')
        copies[destination] = (path, body, original or str(path), consumed[path])

    value = document(complete)
    require(consumed[complete]['sha256'] == sys.argv[2], 'caller-authenticated actual completion')
    require(value['schema'] == 'ferric-p228-projection-ordered-segment-cpu-result-v1'
            and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
            and value['source_unchanged'] is True and value['empty_initial_target'] is True
            and value['complete_runtime_library_suite_selected'] is True
            and all(value[key] is False for key in ('compiler_requalified', 'gpu_execution',
                'numerical_acceptance', 'performance_claim', 'production_authority')), 'actual CPU-only success')
    copy(complete, 'complete.json', str(remote / 'complete.json'))
    plan_path = PL / PACKAGE / 'inputs.json'
    require(value['plan']['path'] == str(E / PACKAGE / 'inputs.json')
            and value['controller']['path'] == str(E / PACKAGE / 'run.py')
            and value['controller']['sha256'] == CONTROLLER_SHA, 'actual closed controller/plan identities')
    plan = document(plan_path, value['plan'])
    require(plan['schema'] == 'ferric-p228-projection-ordered-segment-cpu-inputs-v1'
            and plan['runtime_full_suite_cpu_reviewed'] is True
            and set(plan['proposals']) == {'fe2o3', 'ferric'}
            and plan['added_tests'] == value['declared_test_additions'], 'actual paired source plan')
    require(0 < len(value['phases']) <= 150
            and set(value['raw']) == MAPS | {name + tail for name in value['phases'] for tail in TAILS},
            'exact actual raw/phase roster')
    for name, pin in value['raw'].items():
        require(Path(name).name == name and pin['path'] == str(remote / name), 'original raw identity')
        if name in MAPS:
            checked(case / name, pin)
        else:
            copy(case / name, 'raw/' + name, pin['path'], pin)
    for name, phase in value['phases'].items():
        require(type(phase['exit_code']) is int and phase['exit_code'] == 0 and phase['reason'] is None
                and phase['group_absent'] is True and document(case / (name + '-result.json')) == phase
                and all(phase[s + '_sha256'] == value['raw'][name + '-' + s]['sha256'] for s in ('stdout', 'stderr')),
                'all recorded natural/reaped successful phase/stream joins')
    base = document(case / 'sources-base.json')
    prior_root = W / 'projection-ar4-shared-host-cpu-v228-v1'
    prior = document(prior_root / 'complete.json', value['prior_completion'])
    old_sources = document(prior_root / 'sources-after.json', prior['raw']['sources-after.json'])
    require(value['prior_completion']['path'] == str(E / prior_root.name / 'complete.json')
            and value['prior_completion']['sha256'] == OLD_SHA and prior['passed'] is True
            and prior['raw']['sources-after.json']['sha256'] == SOURCE_SHA and base == old_sources,
            'authenticated CPU883 predecessor and exact original source map')
    unformatted = document(case / 'sources-unformatted.json')
    tested = document(case / 'sources-after.json')
    require(len(base) == 7003 and tested == document(case / 'sources-before.json')
            and document(case / 'old-targets-before.json') == document(case / 'old-targets-after.json')
            and str(E / 'projection-ordered-segment-cpu-v228-v1/target') in document(case / 'old-targets-before.json'),
            'recorded source and old-target postchecks')
    expected, replacements, overlays = dict(base), [], {}
    for project, pin in plan['proposals'].items():
        folder = ('p228-projection-residual-mlp-ordered-runtime-v1' if project == 'fe2o3'
            else 'p228-projection-residual-mlp-ordered-ferric-v2')
        proposal_path = PL / folder / 'source-manifest.json'
        require(pin['path'] == str(E / folder / 'source-manifest.json'), 'original source proposal identity')
        proposal = document(proposal_path, pin)
        rows = proposal['files']
        require(0 < len(rows) <= 64 and len({row['path'] for row in rows}) == len(rows), 'bounded distinct overlay')
        for name in ('source-manifest.json', 'README.md', 'changes.patch'):
            copy(PL / folder / name, 'proposals/' + project + '/' + name, str(E / folder / name))
        for row in rows:
            relative = Path(row['path']); key = project + '/' + row['path']
            require(not relative.is_absolute() and '..' not in relative.parts and str(relative) == row['path']
                    and row['source'] == 'candidate/' + row['path'], 'closed overlay path')
            require((project == 'fe2o3' and row['path'].startswith('crates/fe2o3-kfd/src/') and relative.suffix == '.rs')
                    or (project == 'ferric' and row['path'].startswith(('adapters/m1-engineering-execution-v1/',
                        'adapters/tp-peer-finite-engineering-worker-v1/'))
                        and (relative.suffix == '.rs' or row['path'] == 'adapters/m1-engineering-execution-v1/Cargo.toml')),
                    'runtime/Ferric source ownership boundary')
            require((key not in expected) if row['before'] is None else expected.get(key) == row['before'], 'manifest preimage in actual base')
            checked(PL / folder / row['source'], row['after'])
            target = ROOTS[project] / relative
            require(target.resolve(strict=False) == target, 'canonical live source path')
            if row['before'] is None:
                require(not target.exists(), 'new live path remains absent'); before = None
            else:
                before = checked(target, row['before'])
                checked(PL / folder / 'baseline' / relative, row['before'])
            expected[key] = row['after']; overlays[key] = row
            path = case / 'sources' / key
            after = checked(path, tested[key])
            before_text = None if before is None else before.decode('utf-8')
            after_text = after.decode('utf-8')
            require(after_text.endswith('\n') and (before_text is None or before_text.endswith('\n')),
                    'exact apply_patch text newline contract')
            replacements.append((project, target, before_text, after_text, tested[key]))
            copy(path, 'source-overlay/' + key, str(remote / 'sources' / key), tested[key])
    require(len(overlays) == 60 and unformatted == expected and set(tested) == set(expected)
            and all(tested[k] == v for k, v in expected.items() if k not in overlays or not k.endswith('.rs')),
            'exact sixty-row source/formatter scope')
    require(set(plan['added_tests']) == {'runtime', 'worker', 'parent_library', 'parent_binary'}, 'four named addition scopes')
    for name, test in value['tests'].items():
        raw = checked(case / (name + '-stdout')).decode()
        rows = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|ignored)(?:,.*)?$', raw, re.M)
        require(len(rows) == len({r[0] for r in rows}) and {r[0] for r in rows} == set(test['names'])
                and sum(r[1] == 'ok' for r in rows) == test['passed']
                and sum(r[1] == 'ignored' for r in rows) == test['ignored'], 'actual named outcomes')
        summaries = [list(map(int, row)) for row in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw)]
        require(summaries == test['summaries'] and all(row[1] == 0 for row in summaries), 'actual zero-failure summaries')
    runtime_names = re.findall(r'^([A-Za-z0-9_:]+): test$', checked(case / 'runtime-list-stdout').decode(), re.M)
    require(len(runtime_names) == len(set(runtime_names))
            and set(runtime_names) == set(value['tests']['runtime-tests']['names']), 'full compiled runtime/list identity')
    derived = 883 + len(runtime_names) - 3 + sum(len(plan['added_tests'][k]) for k in ('worker', 'parent_library', 'parent_binary'))
    require(value['tests_passed'] == sum(t['passed'] for t in value['tests'].values()) == derived
            and value['tests_ignored'] == sum(t['ignored'] for t in value['tests'].values()) == 7, 'named-derived actual counts')
    require(plan['parent_binary'] not in OLD_BINARIES
            and set(value['binaries']) == OLD_BINARIES | {plan['parent_binary']}, 'five actual ELF artifact roles')
    for name, row in value['binaries'].items():
        role = 'worker' if name == WORKER else 'parent'
        binary_path = str(remote / 'target' / role / 'debug' / name)
        require(row['binary']['path'] == row['artifact']['executable'] == binary_path
                and row['artifact']['target']['name'] == name and row['artifact']['profile']['test'] is False,
                'closed actual executable metadata, not local body rehash')
        raw = checked(case / ('worker-build-stdout' if role == 'worker' else 'parent-builds-stdout')).decode()
        require([json.loads(line, object_pairs_hook=pairs) for line in raw.splitlines() if line.startswith('{')].count(row['artifact']) == 1,
                'actual Cargo artifact stream')
    primary = document(PRIMARY)
    require(consumed[PRIMARY]['sha256'] == PRIMARY_SHA
            and primary['schema'] == 'ferric-p228-projection-ordered-segment-preparation-primary-v1'
            and primary['exit_code'] == 0 and primary['tests_passed'] == 17
            and primary['tests_failed'] == primary['tests_ignored'] == 0
            and primary['controller_sha256'] == CONTROLLER_SHA and primary['test_sha256'] == TEST_SHA
            and primary['readme_sha256'] == README_SHA and primary['rust_qualification'] is False,
            'separate authentic primary synthetic-test observation, not remote owner')
    for name, sha in (('run.py', CONTROLLER_SHA), ('test_run.py', TEST_SHA), ('README.md', README_SHA)):
        path = PL / PACKAGE / name; checked(path)
        require(consumed[path]['sha256'] == sha, 'qualified controller source')
        copy(path, 'controller/' + name, str(E / PACKAGE / name))
    copy(plan_path, 'controller/inputs.json', value['plan']['path'], value['plan'])
    copy(PRIMARY, 'controller/primary-test-observation.json')
    exporter = PL / 'p228-projection-ordered-segment-publication-v2/export.py'; checked(exporter)
    require(consumed[exporter]['sha256'] == EXPORT_SHA, 'frozen data courier source')
    copy(exporter, 'publication/export.py')
    copy(Path(__file__).resolve(strict=True), 'publication/publish.py')
    copy(Path(__file__).resolve(strict=True).parent / 'README.md', 'publication/README.md')
    copy(Path(__file__).resolve(strict=True).parent / 'COUNTER-SCOPE.md', 'publication/COUNTER-SCOPE.md')
    require(not OUT.exists(), 'fresh publication destination')
    for root in ROOTS.values():
        require(not git(root, 'status', '--porcelain=v1', '--untracked-files=all'), 'clean worktree before integration')
    require(all(read(path)[1] == pin for path, pin in consumed.items()), 'all local input postchecks before writes')
    patch = ['*** Begin Patch']
    for _, target, before, after, _ in replacements:
        if before is None:
            patch.append('*** Add File: ' + str(target))
        else:
            patch.extend(['*** Update File: ' + str(target), '@@'])
            patch.extend('-' + line for line in before.splitlines())
        patch.extend('+' + line for line in after.splitlines())
    patch.append('*** End Patch')
    subprocess.run(['apply_patch'], input='\n'.join(patch) + '\n', text=True, check=True, timeout=60)
    for _, target, _, _, pin in replacements:
        require(read(target)[1] == pin, 'exact tested source integrated')
    for project, root in ROOTS.items():
        changed = set(git(root, 'diff', '--name-only').splitlines()) | set(git(root, 'ls-files', '--others', '--exclude-standard').splitlines())
        require(changed == {str(target.relative_to(root)) for p, target, _, _, _ in replacements if p == project},
                'only reviewed live source paths changed')
    OUT.mkdir(mode=0o755)
    ledger = []
    for name, (source, body, original, pin) in sorted(copies.items()):
        target = OUT / name; target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(body)
        target.chmod(0o644)
        require(read(target)[1] == pin and read(source)[1] == pin, 'publication copy/source postchecks')
        ledger.append(dict(path=name, original=original, retained=str(source), **pin))
    result = dict(schema='ferric-p228-projection-ordered-segment-cpu-publication-v1',
        actual_complete=dict(path=str(remote / 'complete.json'), **consumed[complete]),
        phases=len(value['phases']), tests_passed=derived, tests_ignored=7,
        tests={k: dict(passed=v['passed'], ignored=v['ignored']) for k, v in value['tests'].items()},
        source_overlay_count=len(replacements), source_maps={name: value['raw'][name] for name in sorted(MAPS)},
        omitted_large_raw_maps={name: dict(original=value['raw'][name],
            retained=dict(path=str(case / name), **consumed[case / name])) for name in sorted(MAPS)},
        omitted_large_raw_bodies=len(MAPS), copied_phase_raw_bodies=5 * len(value['phases']),
        binaries=value['binaries'], copies=ledger,
        integrated_sources=[dict(project=p, path=str(target.relative_to(ROOTS[p])), **pin) for p, target, _, _, pin in replacements],
        primary_test_observation=dict(path=str(PRIMARY), **consumed[PRIMARY]),
        primary_is_remote_owner=False, executable_bodies_locally_rehashed=False, exported_executable_bodies=0,
        all_raw_bodies_locally_rehashed=True, complete_source_tree_locally_rehashed=False,
        compiler_requalified=False, gpu_execution=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)
    body = (json.dumps(result, indent=2, sort_keys=True) + '\n').encode()
    with (OUT / 'result.json').open('xb') as stream:
        stream.write(body)
    (OUT / 'result.json').chmod(0o644)
    print(json.dumps(dict(output=str(OUT), copies=len(ledger), integrated_sources=len(replacements),
        result=read(OUT / 'result.json')[1])), flush=True)


if __name__ == '__main__':
    main()
