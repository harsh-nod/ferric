"""One fresh, offline CPU-only qualification and pure-loader opportunity run."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import time
import tomllib
import types

ROOT = Path(__file__).resolve().parent
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/owned-kernel-admission-bench-v228-v1')
OUT, TARGET, TMP, BENCH, RUNTIME, CACHE = (ROOT / n for n in ('evidence', 'target', 'tmp', 'bench', 'runtime', 'cargo-home'))
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
PROPOSAL = (14692, '600f4cc39c0bdfcc969ad2a3cdc52da4fdea91413ab001a7e0b8a833dc8a4d6c')
BASE = (2448697, '3c4fe3a6c4942b20ec61fd7ea5c3409526e2167c43d24a62fb8660c34b0e713e')
SOURCE_MAP = (424416, '5de4929b2832eda55a53f99945fc4b64a9fbda28b4d4f8bef07f2e8d5e000d6a')
LOCAL = {'fe2o3-amdhsa-loader', 'fe2o3-hsaco', 'fe2o3-amd-target', 'fe2o3-target-spec'}
REGISTRY_NAMES = {'block-buffer', 'cfg-if', 'cpufeatures', 'crypto-common', 'digest', 'hybrid-array',
    'itoa', 'libc', 'memchr', 'proc-macro2', 'quote', 'serde', 'serde_core', 'serde_derive',
    'serde_json', 'sha2', 'syn', 'typenum', 'unicode-ident', 'zmij'}
PACKAGE = 'ferric-owned-kernel-admission-bench-v1'
RUST = ('bench/src/lib.rs', 'bench/src/main.rs', 'bench/src/tests.rs')
SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)
WHOLE, BUILD, CLEANUP = 900, 600, 50
PHASES = ('rustfmt', 'rustfmt-check', 'rustc-version', 'generate-lock', 'metadata',
          'tests-build', 'tests-list', 'tests', 'release-build', 'benchmark')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def compact(row):
    return {k: row[k] for k in ('bytes', 'sha256')}


def parse(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def load_supervisor():
    path = ROOT / 'supervisor.py'
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_nlink == 1 and before.st_size == 41485, 'ordinary exact supervisor')
    raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(path.lstat()) and hashlib.sha256(raw).hexdigest() == SUPERVISOR_SHA,
            'qualified supervisor body')
    module = types.ModuleType('owned_loader_benchmark_supervisor'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def interrupted(number, _frame):
    raise RuntimeError('controller signal ' + str(number))


def dependency_tree(h, root, checksum):
    require(root.resolve(strict=True) == root, 'registry root alias')
    files = h.files_below(root, packed=False)
    require(len(files) <= 2048 and all(p.resolve(strict=True) == p for p in files), 'bounded ordinary extracted crate')
    rows = {str(p.relative_to(root)): h.pin(p) for p in files}
    require(all(row['bytes'] <= 4 << 20 for row in rows.values())
            and sum(row['bytes'] for row in rows.values()) <= 48 << 20, 'extracted crate extent')
    checksums = parse((root / '.cargo-checksum.json').read_bytes())
    require(checksums['package'] == checksum and all(name in rows and rows[name]['sha256'] == digest
            for name, digest in checksums['files'].items()), 'actual extracted package checksum/body joins')
    return rows


def main():
    started = time.monotonic(); deadline = started + WHOLE
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch('[0-9a-f]{64}', sys.argv[1]), 'python3 -B run_cpu.py OBSERVED_INPUT_SHA')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT
            and all(not os.path.lexists(p) for p in (OUT, TARGET, TMP, BENCH / 'Cargo.lock')),
            'fresh exact namespace/target/tmp/lock/evidence')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged CPU host')
    os.umask(0o077); os.sched_setaffinity(0, {8, 9})
    require(os.getpriority(os.PRIO_PROCESS, 0) in (0, 10), 'unexpected nice level')
    if os.getpriority(os.PRIO_PROCESS, 0) == 0: os.nice(10)
    for kind, cap in ((resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, 1 << 30), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        limit = min([cap] + [x for x in (soft, hard) if x != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup')
    h = load_supervisor()
    h.ROOT, h.OUT, h.TARGET, h.TMP, h.SOURCE = ROOT, OUT, TARGET, TMP, BENCH
    h.AS_LIMIT, h.FILE_LIMIT, h.CPU_LIMIT = 12 << 30, 1 << 30, BUILD
    h.CACHE_LIMIT, h.STREAM_LIMIT, h.CLEANUP_RESERVE = 2 << 30, 4 << 20, CLEANUP
    old_scratch = h.scratch_bytes
    def scratch():
        total = old_scratch(); count = 0
        for directory, dirs, names in os.walk(CACHE, followlinks=False):
            require(all(not (Path(directory) / name).is_symlink() for name in dirs), 'cache directory alias')
            for name in names:
                count += 1
                try: total += (Path(directory) / name).lstat().st_size
                except FileNotFoundError: pass
        require(count <= 15000, 'bounded generated cache roster')
        return total
    h.scratch_bytes = scratch
    require(h.shutil.disk_usage(ROOT).free >= h.START_FREE, 'initial40GiB floor')
    def pinned(path, expected):
        row = h.pin(path)
        require((row['bytes'], row['sha256']) == tuple(expected), 'fixed input ' + str(path))
        return row
    pinned(ROOT / 'benchmark-source-manifest.json', PROPOSAL)
    pinned(ROOT / 'baseline-complete.json', BASE); pinned(ROOT / 'baseline-sources.json', SOURCE_MAP)
    proposal = parse((ROOT / 'benchmark-source-manifest.json').read_bytes())
    baseline = parse((ROOT / 'baseline-complete.json').read_bytes())
    source_map = parse((ROOT / 'baseline-sources.json').read_bytes())
    require(baseline['passed'] is True and baseline['failure'] is None and baseline['postcheck_errors'] == [],
            'actual qualified dependency/cache/tool ancestor')
    inputs = parse((ROOT / 'input-manifest.json').read_bytes())
    require(h.pin(ROOT / 'input-manifest.json')['sha256'] == sys.argv[1]
            and set(inputs) == {'schema', 'files'}
            and inputs['schema'] == 'ferric-owned-kernel-admission-stage-v1', 'observed closed staging manifest')
    expected = {}
    original_proposal_root = Path(proposal['sources'][0]['path']).parents[1]
    for row in proposal['sources']:
        relative = Path(row['path']).relative_to(original_proposal_root).as_posix()
        name = 'proposal-README.md' if relative == 'README.md' else relative
        require(not Path(name).is_absolute() and '..' not in Path(name).parts, 'proposal relative source')
        expected[name] = compact(row)
    runtime = {name.removeprefix('fe2o3/'): compact(row) for name, row in source_map.items() if name.startswith('fe2o3/')}
    require(len(runtime) == 827, 'qualified827 runtime map')
    expected.update({'runtime/' + name: row for name, row in runtime.items()})
    for row in proposal['runtime_dependency_sources']:
        name = Path(row['path']).relative_to(proposal['runtime_source_root']).as_posix()
        name = {'Cargo.toml': 'Cargo.toml.original', 'Cargo.lock': 'Cargo.lock.input'}.get(name, name)
        require(runtime[name] == compact(row), 'pure dependency source/qualified original join')
    cache = baseline['cache_provenance']['files']
    require(len(cache) == 73, 'exact73 immutable private cache inputs')
    expected.update({'cargo-home/' + name: compact(row) for name, row in cache.items()})
    for row in proposal['image_inputs']:
        expected['images/' + row['copy_name']] = compact(row)
    fixed = {'run_cpu.py', 'supervisor.py', 'benchmark-source-manifest.json', 'baseline-complete.json', 'baseline-sources.json'}
    for name in fixed:
        expected[name] = compact(h.pin(ROOT / name))
    require(inputs['files'] == expected, 'closed staged source/cache/image roster')
    def ordinary_files(path):
        files = h.files_below(path)
        require(all(p.resolve(strict=True) == p and p.lstat().st_nlink == 1 for p in files), 'ordinary single-link tree')
        return files
    initial = {str(p.relative_to(ROOT)): compact(h.pin(p)) for p in ordinary_files(ROOT)}
    require(initial == dict(expected, **{'input-manifest.json': compact(h.pin(ROOT / 'input-manifest.json'))}), 'exact complete fresh input tree')
    require(sum(row['bytes'] for row in initial.values()) <= 96 << 20, 'bounded input tree')
    original_workspace = tomllib.loads((RUNTIME / 'Cargo.toml.original').read_text())['workspace']
    fixture_workspace = tomllib.loads((RUNTIME / 'Cargo.toml').read_text())['workspace']
    require(original_workspace['package'] == fixture_workspace['package']
            and all(original_workspace['dependencies'][name] == fixture_workspace['dependencies'][name]
                    for name in LOCAL | {'sha2'}), 'qualified reduced workspace preserves every inherited pure field')
    immutable = set(initial) - set(RUST) - {name for name in initial if name.startswith('cargo-home/')}
    def sources():
        paths = [ROOT / name for name in sorted(immutable | set(RUST))]
        if (BENCH / 'Cargo.lock').exists(): paths.append(BENCH / 'Cargo.lock')
        rows = {str(p.relative_to(ROOT)): h.pin(p) for p in paths}
        require({str(p.relative_to(BENCH)) for p in ordinary_files(BENCH)}
                == {'Cargo.toml', 'src/lib.rs', 'src/main.rs', 'src/tests.rs'} | ({'Cargo.lock'} if (BENCH / 'Cargo.lock').exists() else set()),
                'closed benchmark source tree')
        require({str(p.relative_to(RUNTIME)) for p in ordinary_files(RUNTIME)} == set(runtime), 'closed runtime source tree')
        return rows
    h.sources = sources
    before = sources(); configurations = h.configurations()
    tools = {name: h.pin(Path(row['path'])) for name, row in baseline['tool_pins'].items()}
    require(tools == baseline['tool_pins'], 'same qualified compiler/linker/prlimit bodies')
    toolbin = Path(tools['cargo']['path']).parent
    environment = dict(PATH=str(toolbin) + ':/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
        CARGO_HOME=str(CACHE), CARGO_TARGET_DIR=str(TARGET), CARGO_NET_OFFLINE='true', CARGO_BUILD_JOBS='2',
        RUSTC=tools['rustc']['path'], RUSTDOC=tools['rustdoc']['path'], TMPDIR=str(TMP),
        HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='',
        OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1')
    cargo = tools['cargo']['path']; manifest = str(BENCH / 'Cargo.toml')
    def command(verb, *args):
        return [cargo, verb, '--manifest-path', manifest, '--offline', '--locked', *args]
    OUT.mkdir(mode=0o700); TARGET.mkdir(mode=0o700); TMP.mkdir(mode=0o700)
    phases = []; errors = []; failure = None; tests = None; artifact = None; test_artifact = None; report = None; tested = None; after = None
    dependencies = {}; dependency_checksums = {}; dependencies_after = {}
    handlers = {n: signal.getsignal(n) for n in SIGNALS}
    for n in handlers: signal.signal(n, interrupted)
    def arm():
        left = deadline - time.monotonic(); require(left > 0, 'whole bound exhausted')
        signal.setitimer(signal.ITIMER_REAL, left)
    def leaf(label, argv, frozen, seconds=BUILD):
        h.run(label, argv, environment, phases, deadline, frozen, seconds=seconds, cwd=BENCH)
    def select(label, testing, kind):
        records = h.build_records(OUT / (label + '.stdout'))
        selected = [r for r in records if r.get('reason') == 'compiler-artifact'
            and r.get('manifest_path') == manifest and kind in r.get('target', {}).get('kind', [])
            and r.get('profile', {}).get('test') is testing and r.get('executable')]
        require(len(selected) == 1, 'one exact own Cargo product')
        row = selected[0]; path = Path(row['executable'])
        require(path.is_relative_to(TARGET / 'release') and path.resolve(strict=True) == path
                and row['profile']['opt_level'] == '2' and row['profile']['debug_assertions'] is True,
                'release opt2/assertions selected product')
        with path.open('rb') as stream: require(stream.read(4) == b'\x7fELF', 'selected ELF')
        return dict(pin=h.pin(path), cargo_artifact=row)
    try:
        arm(); h.save('sources-before.json', before)
        fmt = [tools['rustfmt']['path'], '--edition', '2024', *[str(ROOT / name) for name in RUST]]
        leaf('rustfmt', fmt, None, 60)
        formatted = sources()
        require(set(formatted) == set(before) and all(formatted[n] == before[n] for n in immutable), 'only three benchmark Rust files may format')
        h.save('sources-formatted.json', formatted)
        leaf('rustfmt-check', [*fmt[:1], '--check', *fmt[1:]], formatted, 60)
        leaf('rustc-version', [tools['rustc']['path'], '-Vv'], formatted, 30)
        leaf('generate-lock', [cargo, 'generate-lockfile', '--manifest-path', manifest, '--offline'], None, 60)
        tested = sources()
        require(set(tested) == set(formatted) | {'bench/Cargo.lock'}
                and all(tested[n] == formatted[n] for n in formatted), 'only fresh benchmark lock generated')
        h.save('sources-tested.json', tested)
        lock = tomllib.loads((BENCH / 'Cargo.lock').read_text())
        archive_pins = {Path(n).name.removesuffix('.crate'): row['sha256'] for n, row in cache.items() if n.endswith('.crate')}
        for row in lock['package']:
            if 'source' in row:
                require(row['name'] in REGISTRY_NAMES and row['source'] == 'registry+https://github.com/rust-lang/crates.io-index'
                        and archive_pins.get(row['name'] + '-' + row['version']) == row['checksum'], 'allowlisted exact cached lock package')
            else: require(row['name'] in LOCAL | {PACKAGE}, 'only pure local dependency lock packages')
        leaf('metadata', command('metadata', '--format-version', '1'), tested, 60)
        metadata = parse((OUT / 'metadata.stdout').read_bytes())
        packages = metadata['packages']; ids = {p['id'] for p in packages}
        require(len(ids) == len(packages) and len(packages) <= 25
                and {n['id'] for n in metadata['resolve']['nodes']} == ids, 'closed resolved dependency graph')
        own = [p for p in packages if p['name'] == PACKAGE]
        require(len(own) == 1 and metadata['workspace_members'] == [own[0]['id']]
                and metadata['resolve']['root'] == own[0]['id'], 'standalone workspace root')
        for p in packages:
            path = Path(p['manifest_path'])
            if p['source'] is None:
                expected_path = BENCH / 'Cargo.toml' if p['name'] == PACKAGE else RUNTIME / 'crates' / p['name'] / 'Cargo.toml'
                require(p['name'] in LOCAL | {PACKAGE} and path == expected_path, 'no KFD/native local dependency')
            else:
                require(p['name'] in REGISTRY_NAMES and path.is_relative_to(CACHE / 'registry/src')
                        and p['source'] == 'registry+https://github.com/rust-lang/crates.io-index', 'private allowlisted registry dependency')
                checksum = archive_pins[p['name'] + '-' + p['version']]
                dependencies[str(path.parent)] = dependency_tree(h, path.parent, checksum)
                dependency_checksums[str(path.parent)] = checksum
        require({p['name'] for p in packages if p['source'] is None} == LOCAL | {PACKAGE}, 'all four safe local crates')
        h.save('dependencies-before.json', dependencies)
        leaf('tests-build', command('test', '--release', '--lib', '--no-run', '--message-format=json'), tested)
        test_artifact = select('tests-build', True, 'lib')
        test_elf = test_artifact['pin']['path']
        leaf('tests-list', [test_elf, '--list'], tested, 30)
        names = proposal['new_test_names']
        require(h.inventory(OUT / 'tests-list.stdout') == sorted(names) and len(names) == len(set(names)) == 10, 'exact ten test names')
        leaf('tests', [test_elf, '--test-threads=1'], tested, 60)
        tests = h.outcomes(OUT / 'tests.stdout', names, 10)
        require(h.pin(Path(test_elf)) == test_artifact['pin'], 'tested ELF unchanged')
        leaf('release-build', command('build', '--release', '--bin', PACKAGE, '--message-format=json'), tested)
        artifact = select('release-build', False, 'bin')
        h.AS_LIMIT, h.CPU_LIMIT, h.STREAM_LIMIT = 256 << 20, 45, 64 << 10
        leaf('benchmark', [artifact['pin']['path'], str(ROOT / 'images')], tested, 60)
        require((OUT / 'benchmark.stderr').read_bytes() == b'', 'no benchmark diagnostics on success')
        report = parse((OUT / 'benchmark.stdout').read_bytes())
        require(report['schema'] == 'ferric-owned-kernel-admission-opportunity-v1' and report['passed'] is True
                and report['layers_per_sample'] == 36 and report['preparations_per_sample'] == 360
                and len(report['warmup_pairs']) == 2 and len(report['sample_pairs']) == 12
                and report['pre_and_post_equivalence_checked'] is True and report['original_images_rehashed_after_samples'] is True,
                'closed successful pure opportunity report')
        require(all(report[k] is False for k in ('external_resource_limits_verified_by_program', 'dynamic_dispatch_validation_measured',
                'native_or_model_execution', 'end_to_end_gain', 'full2303_feasibility', 'numerical_acceptance')), 'no transferred authority')
        require(h.pin(Path(artifact['pin']['path'])) == artifact['pin'], 'benchmark ELF unchanged')
        require(tuple(row['label'] for row in phases) == PHASES, 'exact ten owned phases')
    except BaseException as error:
        failure = type(error).__name__ + ': ' + str(error)
    finally:
        for n in handlers: signal.signal(n, interrupted if n == signal.SIGALRM else signal.SIG_IGN)
        arm()
        try:
            after = sources(); h.save('sources-after.json', after)
            require(set(after) <= set(before) | {'bench/Cargo.lock'}
                    and all(after[n] == before[n] for n in immutable)
                    and (tested is None or after == tested), 'source/lock postcheck')
            for name, row in cache.items(): require(compact(h.pin(CACHE / name)) == compact(row), 'immutable private cache input drift')
            require(not any(os.path.lexists(CACHE / n) for n in ('config', 'config.toml')), 'private Cargo configuration appeared')
            require(h.configurations() == configurations and all(h.pin(Path(row['path'])) == row for row in tools.values()), 'tool/configuration drift')
            for selected in (test_artifact, artifact):
                if selected is not None: require(h.pin(Path(selected['pin']['path'])) == selected['pin'], 'selected artifact posthash')
            for path, rows in dependencies.items():
                dependencies_after[path] = dependency_tree(h, Path(path), dependency_checksums[path])
                require(dependencies_after[path] == rows, 'compiled registry dependency drift')
            h.save('dependencies-after.json', dependencies_after)
            require({p.name for p in ROOT.iterdir()} == {'run_cpu.py', 'supervisor.py', 'benchmark-source-manifest.json',
                'baseline-complete.json', 'baseline-sources.json', 'proposal-README.md', 'input-manifest.json',
                'bench', 'runtime', 'images', 'cargo-home', 'evidence', 'target', 'tmp'}, 'unexpected root writes')
            require({p.name for p in ordinary_files(ROOT / 'images')} == {r['copy_name'] for r in proposal['image_inputs']}, 'image directory closure')
            require(all(row['reaped'] and row['process_group_absent'] for row in phases), 'all started groups retired')
            require(tuple(row['label'] for row in phases) == PHASES[:len(phases)], 'exact original phase prefix')
            require(time.monotonic() < deadline, 'whole controller bound')
        except BaseException as error:
            if time.monotonic() >= deadline: raise
            errors.append(type(error).__name__ + ': ' + str(error))
    arm()
    raw = {p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()}
    try:
        allowed_raw = {'sources-before.json', 'sources-formatted.json', 'sources-tested.json', 'sources-after.json',
                       'dependencies-before.json', 'dependencies-after.json'}
        allowed_raw |= {label + suffix for label in PHASES for suffix in
                        ('.command.json', '.started.json', '.result.json', '.stdout', '.stderr')}
        require(set(raw) <= allowed_raw, 'closed original evidence prefix')
        for row in phases:
            for key in ('command', 'stdout', 'stderr'):
                require(raw[Path(row[key]['path']).name] == row[key], 'original phase raw join')
            require(parse((OUT / (row['label'] + '.result.json')).read_bytes()) == row, 'original phase receipt join')
            require(parse((OUT / (row['label'] + '.started.json')).read_bytes()) == dict(pid=row['pid'], pgid=row['pgid'], argv=row['argv']), 'original registration join')
    except BaseException as error:
        if time.monotonic() >= deadline: raise
        errors.append('raw reconciliation: ' + repr(error))
    failure = failure or ('postcheck failed' if errors else None)
    value = dict(schema='ferric-owned-kernel-admission-cpu-v1', passed=failure is None and report is not None,
        failure=failure, postcheck_errors=errors, input_manifest=h.pin(ROOT / 'input-manifest.json'),
        controller=before['run_cpu.py'], supervisor=before['supervisor.py'], proposal=before['benchmark-source-manifest.json'],
        sources_before=before, sources_tested=tested, sources_after=after, phases=phases, raw=raw,
        tests=tests, test_artifact=test_artifact, benchmark_artifact=artifact, tool_pins=tools,
        dependencies_before=dependencies, dependencies_after=dependencies_after,
        qualified_reduced_runtime_workspace_preserved=True,
        pure_inherited_workspace_fields_equal_to_original=True,
        environment=environment, benchmark_report=report, elapsed_seconds=time.monotonic() - started,
        limits=dict(whole_seconds=WHOLE, build_leaf_seconds=BUILD, cleanup_seconds=CLEANUP,
            build_address_space_bytes=12 << 30, scratch_bytes=2 << 30, build_stream_bytes=4 << 20,
            benchmark_wall_seconds=60, benchmark_cpu_seconds=45, benchmark_address_space_bytes=256 << 20,
            benchmark_each_stream_bytes=64 << 10, benchmark_internal_stdout_bytes=128 << 10,
            affinity=[8, 9], nice=10, initial_free_bytes=h.START_FREE, live_free_bytes=h.LIVE_FREE),
        gpu_execution=False, native_execution=False, model_execution=False, runtime_source_changed=False,
        end_to_end_gain=False, numerical_acceptance=False, full2303_feasibility=False, production_authority=False)
    h.save('complete.json' if value['passed'] else 'failed.json', value)
    print(json.dumps({k: value[k] for k in ('passed', 'failure', 'postcheck_errors', 'tests')}, sort_keys=True))
    signal.setitimer(signal.ITIMER_REAL, 0)
    for n, old in handlers.items(): signal.signal(n, old)
    return 0 if value['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
