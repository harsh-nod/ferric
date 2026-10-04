"""Fresh bounded CPU qualification for the projection-boundary candidate and v18."""
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import sys
import types

try:
    import tomllib
except ModuleNotFoundError as error:
    if error.name != 'tomllib':
        raise
    import tomli as tomllib

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
PROPOSAL = E / 'p228-projection-residual-kernel-v1'
MANIFEST_SHA = '08973f7fd8851ad0c3ea628006454f91f2af87e9db156778b6e3de2ea5de865f'
OLD = 'qwen3-tp-peer-tp2-kernels-v18'
NEW = 'qwen3-tp-projection-residual-kernels-v1'
OLD_PACKAGE = 'ferric-qwen3-tp-peer-tp2-kernels-device-v18'
NEW_PACKAGE = 'ferric-qwen3-tp-projection-residual-kernels-device-v1'
HELPERS = (
    (E / 'run_clean_worker_p228_v1.py', '2f3ef5c80e4483ac1c1a1ebb2bbacf18af7e7b263d13d965fa991525d85e6d2a'),
    (R / 'evidence/wave-output-lowering-v216/bounded.py', 'e634e1b3be3b122b2231ad134c770f25d7d71d10ec767d61a16d03e726bdf4f1'),
    (E / 'metadata_p220_v2.py', '5b5efdaf64053b49c3acb78a82a1ffeaa948a4095b689d2f24e0dff8778ee816'),
)
OLD_FILES = {
    'Cargo.toml': '981fb820206bab880607f58f0c54f0edbb2a0c89f42cbadd66f18c3ea308b33a',
    'Cargo.lock': 'e69451fab45d0855bb7a6dc08bc1dc6474d0271bed9d1e53d94dfc30292c8470',
    'README.md': 'affc97a8407492faea0d7d578d26b37ae7096346e80e4ebf15d1a1089f6da3cd',
    'build.rs': 'cc114acdcb7648b9668bd1f3dcfd6d2d5ed249b2cff3852f8288e39632d94813',
    'rust-toolchain.toml': 'ba50d1103ca205adaabf39f6321d74653fdf38123b31e2c2003c45417dd58907',
    'src/lib.rs': '6800ea442ac0a656ab3ae01f43f772d0809d307cd74aebea17db11195475843b',
    'src/copy.rs': '3d4de38cfa0544af7442fb91b7c52593fe7a0056d56f4bea21f163a887ec21dd',
    'src/collective.rs': 'dac509f21bdd5f43c5f6334f4d36a9ddb6c3fda2de710c9d4f8936df233541f0',
    'tests/contract.rs': 'abd1d0bb007c9faae6b57e6e1d9c143fdf11e11e635d3bf1b5d305c4bcbff97d',
}
SIBLINGS = {
    'qwen3-all-kernels-v1/src/target.rs': 'ab331af33d638f84a8214c229fc9e35cf3c461669689687a7808115c06f56403',
    'qwen3-all-kernels-v1/build/target_contract.rs': '590e235c06df8041b8717caffaea9d1f56e40738d03d6947f4a3324cb171865a',
}
OLD_TARGETS = tuple(E / name / 'target' for name in (
    'ordinary-induction-compiler-cpu-v228-v1',
    'gfx950-clock-recorder-cpu-v228-v1',
    'gfx950-clock-parent-cpu-v228-v1',
    'layer0-native-capture-cpu-v228-v2',
))


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        while raw := stream.read(1 << 20):
            h.update(raw)
    return h.hexdigest()


def pin(path):
    require(path.is_file() and not path.is_symlink() and path.resolve(strict=True) == path,
            'canonical ordinary file: ' + str(path))
    return dict(path=str(path), bytes=path.stat().st_size, sha256=digest(path))


def load(path, expected, name):
    require(pin(path)['sha256'] == expected, 'retained helper pin')
    value = types.ModuleType(name)
    value.__file__ = str(path)
    exec(compile(path.read_bytes(), str(path), 'exec'), value.__dict__)
    return value


def inventory(text, empty=False):
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', text, re.MULTILINE)
    require((names or empty) and len(names) == len(set(names)) and ': benchmark' not in text,
            'exact ordinary test inventory')
    return set(names)


def outcomes(text, names):
    rows = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|ignored(?:, [^\n]*)?)$', text, re.MULTILINE)
    require(len(rows) == len(names) and {name for name, _ in rows} == names
            and all(state == 'ok' for _, state in rows), 'each observed test passed exactly once')
    summaries = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
    require(summaries == [(str(len(names)), '0', '0')], 'one exact successful test summary')
    return dict(passed=len(names), ignored=0, names=sorted(names))


def selected(raw, crate, target):
    messages = [json.loads(line) for line in raw.splitlines() if line.startswith('{')]
    require([r['success'] for r in messages if r.get('reason') == 'build-finished'] == [True],
            'actual Cargo build-finished success')
    result = {}
    for row in messages:
        if row.get('reason') != 'compiler-artifact' or not row.get('profile', {}).get('test'):
            continue
        if row.get('manifest_path') != str(crate / 'Cargo.toml'):
            continue
        kind = row['target']['kind']
        require(kind in (['lib'], ['test']), 'only library and contract tests')
        role = 'lib' if kind == ['lib'] else row['target']['name']
        profile = row['profile']
        require(role in ('lib', 'contract') and role not in result
                and profile['opt_level'] == '0' and profile['debug_assertions'] is True
                and profile['overflow_checks'] is True, 'ordinary checked test artifact profile')
        binary = Path(row['executable'])
        require(binary.is_relative_to(target) and os.access(binary, os.X_OK), 'fresh executable')
        result[role] = dict(binary=pin(binary), artifact=row)
    require(set(result) == {'lib', 'contract'}, 'both real crate test executables')
    return result


def target_inventory():
    result = {}
    for root in OLD_TARGETS:
        require(root.is_dir() and root.resolve(strict=True) == root, 'retained target missing or aliased')
        rows = {}
        for directory, dirs, files in os.walk(root, followlinks=False):
            for name in sorted(dirs + files):
                path = Path(directory) / name
                st = path.lstat()
                rows[str(path.relative_to(root))] = [st.st_mode, st.st_ino, st.st_size,
                    st.st_mtime_ns, st.st_ctime_ns, os.readlink(path) if path.is_symlink() else None]
        result[str(root)] = rows
    return result


def configurations(root):
    paths = {parent / '.cargo' / name for parent in (root, *root.parents)
             for name in ('config', 'config.toml')}
    paths |= {R / 'toolchain/cargo' / name for name in ('config', 'config.toml')}
    return {str(path): pin(path) if os.path.lexists(path) else None for path in sorted(paths)}


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and sys.dont_write_bytecode and len(sys.argv) == 3,
            'usage: python3 -B run.py BASE_DEVICE_DIRECTORY projection-residual-cpu-v228-vN')
    base, label = Path(sys.argv[1]), sys.argv[2]
    require(base.is_relative_to(R) and base.resolve(strict=True) == base
            and re.fullmatch(r'projection-residual-cpu-v228-v[1-9][0-9]*', label), 'root-owned input and fresh label')
    manifest_pin = pin(PROPOSAL / 'source-manifest.json')
    require(manifest_pin['sha256'] == MANIFEST_SHA, 'frozen source manifest')
    value = json.loads((PROPOSAL / 'source-manifest.json').read_bytes())
    require(value['schema'] == 'ferric-p228-projection-residual-kernel-proposal-v1'
            and value['formatted'] is value['tests_executed'] is value['gpu_executed'] is False,
            'unexecuted source candidate')
    expected = {'lib': set(value['test_census']['arithmetic']),
                'contract': set(value['test_census']['source_contract'])}
    require(len(expected['lib']) == 12 and len(expected['contract']) == 5, 'seventeen authored tests')
    inputs = {str(manifest_pin['path']): manifest_pin, str(Path(__file__).resolve()): pin(Path(__file__).resolve())}
    if tomllib.__name__ == 'tomli':
        require(tomllib.__version__ == '2.2.1', 'retained Python3.10 TOML parser version')
    parser_root = Path(tomllib.__file__).resolve(strict=True).parent
    for path in sorted(parser_root.rglob('*.py')):
        inputs[str(path)] = pin(path)
    paths = {row['path'] for row in value['files']}
    require(paths == {'device/' + NEW + '/' + name for name in
                     ('Cargo.toml', 'build.rs', 'src/lib.rs', 'src/collective.rs', 'tests/contract.rs')}
            and len(value['files']) == 5, 'five new candidate inputs')
    for row in value['files']:
        require(row['before'] is None and row['source'] == 'draft/' + row['path'], 'candidate-only new sources')
        actual = pin(PROPOSAL / row['source'])
        require({key: actual[key] for key in ('bytes', 'sha256')} == row['after'], 'candidate source pin')
        inputs[actual['path']] = actual
    require({str(p.relative_to(base / OLD)) for p in (base / OLD).rglob('*') if p.is_file()} == set(OLD_FILES),
            'all nine original v18 files, no omitted source')
    for rel, expected_sha in {**{OLD + '/' + k: v for k, v in OLD_FILES.items()}, **SIBLINGS}.items():
        actual = pin(base / rel)
        require(actual['sha256'] == expected_sha, 'old source/sibling pin: ' + rel)
        inputs[actual['path']] = actual
    x, b, stable = [load(path, expected_sha, 'projection_cpu_' + str(i))
                    for i, (path, expected_sha) in enumerate(HELPERS)]
    for path, _ in HELPERS:
        inputs[str(path)] = pin(path)
    old_targets = target_inventory()
    out = E / label
    require(not os.path.lexists(out), 'fresh exclusive output')
    out.mkdir(mode=0o700)
    sources = out / 'sources'
    sources.mkdir(mode=0o700)
    b.N, b.PINS, b.F, b.T, b.D = stable.N, stable.TOOLS, sources, out / 'target', out
    stable.T = b.T
    b.setup()
    require(not any(b.T.iterdir()), 'fresh empty target')
    for rel in [*(OLD + '/' + name for name in OLD_FILES), *SIBLINGS]:
        dest = sources / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes((base / rel).read_bytes())
    candidate = sources / NEW
    for row in value['files']:
        dest = sources / Path(row['path']).relative_to('device')
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes((PROPOSAL / row['source']).read_bytes())
    original_lock = (sources / OLD / 'Cargo.lock').read_bytes()
    before_name, after_name = ('name = "' + OLD_PACKAGE + '"').encode(), ('name = "' + NEW_PACKAGE + '"').encode()
    require(original_lock.count(before_name) == 1 and after_name not in original_lock, 'one lock root rename')
    new_lock = original_lock.replace(before_name, after_name)
    old_doc, new_doc = tomllib.loads(original_lock.decode()), tomllib.loads(new_lock.decode())
    renamed = [p for p in new_doc['package'] if p['name'] == NEW_PACKAGE]
    require(len(renamed) == 1 and renamed[0].get('source') is None, 'new root lock identity')
    renamed[0]['name'] = OLD_PACKAGE
    require(new_doc == old_doc, 'lock dependency closure byte-preserved apart from root name')
    (candidate / 'Cargo.lock').write_bytes(new_lock)
    b.save(out / 'sources-unformatted.json', x.snapshot(sources))
    b.save(out / 'old-targets-before.json', old_targets)
    configs = configurations(candidate)
    fmt = pin(stable.N / 'bin/rustfmt')
    inputs[fmt['path']] = fmt
    for name, expected_sha in stable.TOOLS.items():
        actual = pin(stable.N / 'bin' / name)
        require(actual['sha256'] == expected_sha, 'pinned stable tool')
        inputs[actual['path']] = actual
    env = dict(stable.env(), CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0',
               TMPDIR=str(out / 'tmp'), PYTHONDONTWRITEBYTECODE='1')
    phases, tests, binaries, dependencies = {}, {}, {}, {}
    formatted = None
    error, post_errors = None, []
    def run(name, argv, deadline=900):
        phases[name] = b.run(out, name, argv, env=env, deadline=deadline)
        if formatted is not None:
            require(x.snapshot(sources) == formatted, 'immutable formatted sources/locks drift')
        return (out / (name + '-stdout')).read_text()
    try:
        args = [fmt['path'], '--edition', '2024', '--config', 'skip_children=true',
                *(str(candidate / name) for name in ('build.rs', 'src/lib.rs', 'src/collective.rs', 'tests/contract.rs'))]
        run('rustfmt', args, 60)
        formatted = x.snapshot(sources)
        for rel in [*(OLD + '/' + name for name in OLD_FILES), *SIBLINGS]:
            require(pin(sources / rel)['sha256'] == digest(base / rel), 'formatter changed control or sibling')
        require((candidate / 'Cargo.lock').read_bytes() == new_lock
                and (candidate / 'Cargo.toml').read_bytes() == (PROPOSAL / 'draft/device' / NEW / 'Cargo.toml').read_bytes(),
                'formatting may only change candidate Rust')
        b.save(out / 'sources-before.json', formatted)
        run('rustfmt-check', [args[0], '--check', *args[1:]], 60)
        package_sets = {}
        for role, crate in (('candidate', candidate), ('control', sources / OLD)):
            cargo = str(stable.N / 'bin/cargo')
            metadata = json.loads(run(role + '-metadata', [cargo, 'metadata', '--offline', '--locked',
                '--manifest-path', str(crate / 'Cargo.toml'), '--format-version', '1'], 120))
            require(metadata['target_directory'] == str(b.T), 'only fresh target selected')
            packages = []
            for package in metadata['packages']:
                path = Path(package['manifest_path'])
                require(path.resolve(strict=True) == path, 'canonical package source')
                if package['source'] is None:
                    require(path == crate / 'Cargo.toml', 'no local dependency substitution')
                    continue
                require(path.is_relative_to(R / 'toolchain/cargo') and (
                    package['source'].startswith('registry+') or
                    package['source'] in {
                        'git+https://github.com/harsh-nod/fe2o3.git?rev=faaaf15d68eff996b22951758b1a9fa83317d6d2#faaaf15d68eff996b22951758b1a9fa83317d6d2',
                        'git+https://github.com/harsh-nod/pliron.git?rev=cc902cc8c669b5de2b292ae8638d9e8311bc735b#cc902cc8c669b5de2b292ae8638d9e8311bc735b'}), 'original cached dependency only')
                packages.append((package['name'], package['version'], package['source'], str(path)))
                snapshot = x.snapshot(path.parent)
                require(str(path.parent) not in dependencies or dependencies[str(path.parent)] == snapshot,
                        'shared dependency source unchanged')
                dependencies[str(path.parent)] = snapshot
            package_sets[role] = sorted(packages)
        require(package_sets['candidate'] == package_sets['control'], 'actual candidate/control dependency equality')
        b.save(out / 'dependencies-before.json', dependencies)
        for role, crate in (('candidate', candidate), ('control', sources / OLD)):
            raw = run(role + '-build-tests', [str(stable.N / 'bin/cargo'), 'test', '--offline', '--locked',
                '--jobs', '2', '--target-dir', str(b.T), '--manifest-path', str(crate / 'Cargo.toml'),
                '--lib', '--test', 'contract', '--no-run', '--message-format=json'], 1800)
            binaries[role] = selected(raw, crate, b.T)
            tests[role] = {}
            for kind, artifact in binaries[role].items():
                stem = role + '-' + kind
                binary = artifact['binary']['path']
                names = inventory(run(stem + '-list', [binary, '--list', '--format', 'terse'], 120))
                require(not inventory(run(stem + '-ignored', [binary, '--ignored', '--list', '--format', 'terse'], 120), True),
                        'no ignored arithmetic/contract tests')
                if role == 'candidate':
                    require(names == expected[kind], 'full authored candidate census')
                tests[role][kind] = outcomes(run(stem + '-tests', [binary, '--test-threads=1'], 300), names)
        require(sum(row['passed'] for row in tests['candidate'].values()) == 17,
                'all seventeen actual candidate tests pass')
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    def check(name, function):
        try:
            function()
        except BaseException as failure:
            post_errors.append(name + ': ' + type(failure).__name__ + ': ' + str(failure))
    check('inputs', lambda: require(all(pin(Path(p)) == v for p, v in inputs.items()), 'input/tool/controller drift'))
    check('old-roster', lambda: require({str(p.relative_to(base / OLD)) for p in (base / OLD).rglob('*') if p.is_file()} == set(OLD_FILES), 'original roster drift'))
    check('configurations', lambda: require(configurations(candidate) == configs, 'configuration drift'))
    after = {}
    def target_postcheck():
        after['targets'] = target_inventory()
        require(after['targets'] == old_targets, 'prior target mutation')
    def source_postcheck():
        after['sources'] = x.snapshot(sources)
        require(formatted is not None and after['sources'] == formatted, 'source drift')
    check('old-targets', target_postcheck)
    check('formatted-sources', source_postcheck)
    after_deps = {}
    for path, before in dependencies.items():
        def recheck(path=path, before=before):
            after_deps[path] = x.snapshot(Path(path))
            require(after_deps[path] == before, 'dependency source drift')
        check('dependency:' + path, recheck)
    for rows in binaries.values():
        for row in rows.values():
            check('binary:' + row['binary']['path'], lambda row=row: require(pin(Path(row['binary']['path'])) == row['binary'], 'test ELF drift'))
    b.save(out / 'sources-after.json', after.get('sources'))
    b.save(out / 'dependencies-after.json', after_deps)
    b.save(out / 'inputs.json', inputs)
    b.save(out / 'configurations.json', configs)
    b.save(out / 'old-targets-after.json', after.get('targets'))
    passed = error is None and not post_errors
    b.save(out / ('complete.json' if passed else 'failed.json'), dict(
        schema='ferric-p228-projection-residual-cpu-result-v1', passed=passed, error=error,
        postcheck_errors=post_errors, controller=inputs[str(Path(__file__).resolve())],
        source_manifest=manifest_pin, phases=phases, tests=tests, binaries=binaries,
        formatted_source_root=str(sources), test_count=sum(v['passed'] for rows in tests.values() for v in rows.values()),
        source_unchanged=not post_errors, gpu_execution=False, compiler_hsaco_reproduced=False,
        numerical_acceptance=False, production_authority=False, performance_claim=False,
        raw={p.name: pin(p) for p in out.iterdir() if p.is_file()}))
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors, output=str(out))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt('terminated')))
    sys.exit(main())
