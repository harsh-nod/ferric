"""Bounded CPU-only qualification of native peer signal-completion probes."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import stat
import subprocess
import time
import tomllib

ROOT = Path(__file__).resolve().parent
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/peer-dependency-signal-completion-cpu-v228-v1')
OUT = ROOT / 'evidence'
TARGET = ROOT / 'target'
TMP = ROOT / 'tmp'
TOOLCHAIN = Path('/home/harmenon/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/bin')
TOOLCHAIN_LIB = TOOLCHAIN.parent / 'lib'
SHARED_LIBRARIES = {
    'libLLVM.so.22.1-rust-1.96.0-nightly': 199520544,
    'librustc_driver-7bb70639c3ace5a4.so': 152936640,
    'libLLVM-22-rust-1.96.0-nightly.so': 43,
}
CRATES = {
    'fe2o3-amd-target', 'fe2o3-amdhsa-loader', 'fe2o3-aql', 'fe2o3-drm-uapi',
    'fe2o3-hsaco', 'fe2o3-kfd', 'fe2o3-kfd-uapi', 'fe2o3-runtime-model',
    'fe2o3-target-spec',
}
FORMATTED = {
    'crates/fe2o3-kfd/src/engineering_gfx950_peer_dependency.rs',
    'crates/fe2o3-kfd/src/engineering_gfx950_peer_dependency_tests.rs',
    'crates/fe2o3-kfd/examples/kfd-peer-dependency-sentinel.rs',
    'crates/fe2o3-kfd/examples/kfd-peer-dependency-terminal-join.rs',
    'crates/fe2o3-kfd/src/engineering_gfx950.rs',
    'crates/fe2o3-kfd/src/engineering_gfx950_peer.rs',
    'crates/fe2o3-kfd/src/engineering_gfx950_peer_tests.rs',
    'crates/fe2o3-kfd/src/queue_linux.rs',
    'crates/fe2o3-kfd/src/lib.rs',
    'crates/fe2o3-kfd/examples/kfd-peer-dependency-signal-completion.rs',
}
EXAMPLE = 'kfd-peer-dependency-signal-completion'
TERMINAL_EXAMPLE = 'kfd-peer-dependency-terminal-join'
COMPATIBILITY_EXAMPLE = 'kfd-peer-dependency-sentinel'
PRUNED_LOCK = dict(bytes=8058, sha256='b605fb665bbd9b6ba266c0b6f74f4b9cab50bd1845e136dc6ac6cb6b2dadd252')
REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index'
SPIN = ('spin', '0.12.3', REGISTRY)
LOCK_API = ('lock_api', '0.4.14', REGISTRY)
WHOLE_WALL = 7200
CLEANUP_RESERVE = 50
LEAF_WALL = 1800
AS_LIMIT = 8 << 30
FILE_LIMIT = 4 << 30
CPU_LIMIT = 1200


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
            value.st_uid, value.st_gid, value.st_size, value.st_mtime_ns,
            value.st_ctime_ns)


def pin(path):
    path = Path(path)
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= FILE_LIMIT,
            'not a bounded ordinary file: ' + str(path))
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file changed before read')
        for chunk in iter(lambda: stream.read(1 << 20), b''):
            digest.update(chunk)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file changed during read')
    require(stamp(path.lstat()) == stamp(before), 'file changed after read')
    return dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())


def sources():
    files = [ROOT / name for name in ('Cargo.toml', 'run_cpu.py', 'Cargo.lock',
                                      'Cargo.lock.input', 'Cargo.toml.original',
                                      'rust-toolchain.toml')]
    for name in sorted(CRATES):
        base = ROOT / 'crates' / name
        require(base.is_dir() and not base.is_symlink(), 'missing ordinary crate: ' + name)
        for directory, dirs, names in os.walk(base, followlinks=False,
                                               onerror=lambda error: (_ for _ in ()).throw(error)):
            for child in dirs:
                require(not (Path(directory) / child).is_symlink(), 'symlink in source tree')
            files.extend(Path(directory) / name for name in names)
    return {str(path.relative_to(ROOT)): pin(path) for path in sorted(files)}


def save(name, value):
    path = OUT / name
    with path.open('x') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')
    return pin(path)


def group_exists(pgid):
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False


def reap_group(pgid):
    reaped = []
    while True:
        try:
            pid, status = os.waitpid(-pgid, os.WNOHANG)
        except ChildProcessError:
            break
        if pid == 0:
            break
        reaped.append(dict(pid=pid, wait_status=status))
    return reaped


def stop_group(child, hard_deadline):
    reaped = []
    for sig, allowance in ((signal.SIGTERM, 10), (signal.SIGKILL, 20)):
        child.poll()
        reaped.extend(reap_group(child.pid))
        if not group_exists(child.pid):
            break
        try:
            os.killpg(child.pid, sig)
        except ProcessLookupError:
            break
        deadline = min(hard_deadline, time.monotonic() + allowance)
        while time.monotonic() < deadline:
            child.poll()
            reaped.extend(reap_group(child.pid))
            if not group_exists(child.pid):
                break
            time.sleep(0.02)
    return reaped


def interrupted(signum, _frame):
    raise RuntimeError('interrupted by signal ' + str(signum))


def normalized_libtest(text):
    names = (
        'queue_linux::tests::payload_release_failure_after_event_destroy_is_process_terminal',
        'queue_linux::tests::unpublished_custody_cleanup_failure_is_process_terminal',
    )
    for name in names:
        block = 'test ' + name + ' ... \nrunning 1 test\nok'
        text, count = re.subn('^' + re.escape(block) + r'(?=\n|\Z)',
                              'test ' + name + ' ... ok', text, flags=re.M)
        require(count <= 1, 'duplicate known abort-child parent block: ' + name)
    return text


def test_outcomes(path):
    text = normalized_libtest(path.read_text())
    summaries = [dict(status=status, passed=int(passed), failed=int(failed),
                      ignored=int(ignored), measured=int(measured), filtered_out=int(filtered))
                 for status, passed, failed, ignored, measured, filtered in re.findall(
                     r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; '
                     r'(\d+) measured; (\d+) filtered out;', text, re.M)]
    named = [dict(name=name, outcome=outcome) for name, outcome in re.findall(
        r'^test (.+?) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', text, re.M)]
    require(summaries, 'no actual libtest summaries: ' + str(path))
    for status, field in (('ok', 'passed'), ('FAILED', 'failed'), ('ignored', 'ignored')):
        require(sum(row['outcome'] == status for row in named)
                == sum(row[field] for row in summaries), 'named libtest census differs from summaries')
    return dict(summaries=summaries, named=named,
                passed=sum(row['passed'] for row in summaries),
                failed=sum(row['failed'] for row in summaries),
                ignored=sum(row['ignored'] for row in summaries))


def selected_example(path, example):
    require(example in (EXAMPLE, TERMINAL_EXAMPLE, COMPATIBILITY_EXAMPLE), 'unknown example selection')
    records = [json.loads(line) for line in path.read_text().splitlines() if line]
    finished = [row for row in records if row.get('reason') == 'build-finished']
    require(len(finished) == 1 and finished[0].get('success') is True, 'Cargo build did not finish successfully')
    artifacts = [row for row in records if row.get('reason') == 'compiler-artifact'
                 and row.get('target', {}).get('name') == example
                 and row['target'].get('kind') == ['example']
                 and row.get('profile', {}).get('test') is False
                 and row.get('executable')]
    require(len(artifacts) == 1, 'expected exactly one actual example executable')
    artifact = artifacts[0]
    binary = Path(artifact['executable'])
    require(binary.is_absolute() and binary.is_relative_to(TARGET)
            and binary.resolve() == binary, 'example executable outside selected target')
    require(binary.stat().st_mode & 0o111, 'example artifact is not executable')
    with binary.open('rb') as stream:
        require(stream.read(4) == b'\x7fELF', 'example artifact is not an ELF')
    return dict(pin=pin(binary), cargo_artifact=artifact)


def lock_pruning():
    actual = pin(ROOT / 'Cargo.lock')
    require(all(actual[key] == value for key, value in PRUNED_LOCK.items()),
            'pruned lock differs from the actual retained v1 lock')
    original = tomllib.loads((ROOT / 'Cargo.lock.input').read_text())
    selected = tomllib.loads((ROOT / 'Cargo.lock').read_text())
    require(original.get('version') == selected.get('version'), 'Cargo lock format changed')
    def package_map(document):
        rows = document.get('package')
        require(isinstance(rows, list) and rows, 'empty Cargo package lock')
        result = {}
        for row in rows:
            key = (row['name'], row['version'], row.get('source'))
            require(key not in result, 'duplicate Cargo package identity')
            result[key] = row
        return result
    before, after = package_map(original), package_map(selected)
    require(set(after) <= set(before), 'offline lock generation selected a new package/version')

    def dependencies(row, packages):
        values = row.get('dependencies', [])
        require(isinstance(values, list), 'Cargo dependencies must be a list')
        resolved = []
        for value in values:
            require(isinstance(value, str), 'Cargo dependency must be a string')
            matches = []
            for key in packages:
                name, version, source = key
                forms = {name, name + ' ' + version}
                if source is not None:
                    forms.add(name + ' ' + version + ' (' + source + ')')
                if value in forms:
                    matches.append(key)
            require(len(matches) == 1, 'ambiguous or unavailable locked dependency: ' + value)
            resolved.append(matches[0])
        require(len(set(resolved)) == len(resolved), 'duplicate resolved Cargo dependency')
        return set(resolved)

    removed_edges = set()
    for key, row in after.items():
        require({name: value for name, value in row.items() if name != 'dependencies'}
                == {name: value for name, value in before[key].items() if name != 'dependencies'},
                'offline lock pruning changed retained package identity/checksum/fields')
        old_dependencies = dependencies(before[key], before)
        new_dependencies = dependencies(row, after)
        require(new_dependencies <= old_dependencies, 'offline lock pruning added a dependency edge')
        for removed in old_dependencies - new_dependencies:
            require(key == SPIN and removed == LOCK_API,
                    'offline lock pruning removed an unapproved dependency edge')
            removed_edges.add((key, removed))
    require(removed_edges == {(SPIN, LOCK_API)}, 'expected exact spin optional dependency pruning')
    return dict(original_packages=len(before), selected_packages=len(after),
                removed_packages=len(before) - len(after),
                package_identity_and_checksum_unchanged=True,
                other_resolved_dependencies_unchanged=True,
                removed_dependency_edges=[dict(package=list(SPIN), dependency=list(LOCK_API))],
                actual_pruned_lock=actual)


def spin_feature_evidence(path):
    metadata = json.loads(path.read_text())
    packages = [row for row in metadata['packages']
                if (row['name'], row['version'], row.get('source')) == SPIN]
    require(len(packages) == 1, 'metadata lacks exact retained spin package')
    package = packages[0]
    nodes = [row for row in metadata['resolve']['nodes'] if row['id'] == package['id']]
    require(len(nodes) == 1 and nodes[0]['features'] == ['once'],
            'spin enabled features differ from actual once-only pruning')
    require(package['features'].get('once') == []
            and package['features'].get('lock_api') == ['dep:lock_api_crate'],
            'spin feature-to-optional-dependency contract changed')
    optional = [row for row in package['dependencies']
                if row['name'] == 'lock_api' and row.get('rename') == 'lock_api_crate']
    require(len(optional) == 1 and optional[0]['optional'] is True
            and optional[0].get('source') == REGISTRY, 'spin lock_api dependency is not the expected optional alias')
    workspace = tomllib.loads((ROOT / 'Cargo.toml').read_text())
    selection = workspace['workspace']['dependencies']['spin']
    require(selection.get('default-features') is False and selection.get('features') == ['once'],
            'thin workspace changed spin feature selection')
    return dict(metadata=pin(path), package_id=package['id'], enabled_features=nodes[0]['features'],
                optional_dependency=optional[0], lock_api_feature=package['features']['lock_api'],
                workspace_default_features=False, workspace_features=['once'])


def main():
    started = time.monotonic()
    hard_deadline = started + WHOLE_WALL
    work_deadline = hard_deadline - CLEANUP_RESERVE
    require(__debug__, 'optimized Python is forbidden')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'unexpected CPU qualification host/UID')
    require(ROOT == EXPECTED_ROOT and not any(path.exists() for path in (OUT, TARGET, TMP)),
            'fresh exact qualification namespace required')
    require({path.name for path in (ROOT / 'crates').iterdir()} == CRATES, 'thin workspace crate roster differs')
    require(shutil.disk_usage(ROOT).free >= 4 << 30, 'less than 4 GiB free before build')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected initial nice level')
    if priority == 0:
        os.nice(10)
    require(os.sched_getaffinity(0) == {8, 9}
            and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'CPU affinity/nice mismatch')
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, AS_LIMIT),
                      (resource.RLIMIT_FSIZE, FILE_LIMIT)):
        soft, hard = resource.getrlimit(kind)
        effective = min([cap] + [n for n in (soft, hard) if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (effective, effective))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup failed')
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, interrupted)
    env = dict(HOME='/home/harmenon', PATH=str(TOOLCHAIN) + ':/usr/bin:/bin',
               CARGO_HOME='/home/harmenon/.cargo', CARGO_TARGET_DIR=str(TARGET),
               LD_LIBRARY_PATH=str(TOOLCHAIN_LIB),
               TMPDIR=str(TMP), RUSTC=str(TOOLCHAIN / 'rustc'), RUSTDOC=str(TOOLCHAIN / 'rustdoc'),
               CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', RUST_BACKTRACE='1',
               CARGO_PROFILE_DEV_OPT_LEVEL='2', CARGO_PROFILE_DEV_DEBUG='0',
               CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
               CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_TEST_DEBUG='0',
               CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
               ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
    before = sources()
    require(FORMATTED <= set(before), 'missing selected formatting input')
    require(all(before['Cargo.lock'][key] == before['Cargo.lock.input'][key]
                for key in ('bytes', 'sha256')), 'initial lock differs from retained original')
    input_pin = save('sources-input.json', before)
    tool_pins = {name: pin(TOOLCHAIN / name) for name in ('rustc', 'rustdoc', 'rustfmt', 'cargo')}
    tool_pins['prlimit'] = pin(Path('/usr/bin/prlimit'))
    require(TOOLCHAIN_LIB.is_dir() and not TOOLCHAIN_LIB.is_symlink()
            and TOOLCHAIN_LIB.resolve() == TOOLCHAIN_LIB, 'toolchain library directory is not canonical')
    for name, extent in SHARED_LIBRARIES.items():
        path = TOOLCHAIN_LIB / name
        require(path.resolve() == path, 'toolchain library path is not canonical')
        tool_pins[name] = pin(path)
        require(tool_pins[name]['bytes'] == extent, 'toolchain library extent differs from actual roster')
    results, test_results = [], {}
    formatted = tested = binary = terminal_binary = compatibility_binary = lock_delta = None
    failure = None
    postcheck_errors = []
    signal.setitimer(signal.ITIMER_REAL, max(0.001, work_deadline - time.monotonic()))

    def run(label, argv):
        remaining = work_deadline - time.monotonic()
        require(remaining > 0, 'whole-run work deadline reached')
        wall = min(LEAF_WALL, remaining)
        command = ['/usr/bin/prlimit', '--as=' + str(AS_LIMIT), '--cpu=' + str(CPU_LIMIT),
                   '--fsize=' + str(FILE_LIMIT), '--core=0', '--', *argv]
        command_pin = save(label + '.command.json', dict(argv=command, cwd=str(ROOT), env=env,
                                                        wall_timeout_seconds=wall))
        leaf_started = time.monotonic_ns()
        stdout, stderr = OUT / (label + '.stdout'), OUT / (label + '.stderr')
        timed_out = forced_cleanup = False
        exception = None
        code = None
        reaped = []
        with stdout.open('xb') as so, stderr.open('xb') as se:
            child = subprocess.Popen(command, cwd=ROOT, env=env, stdin=subprocess.DEVNULL,
                                     stdout=so, stderr=se, start_new_session=True)
            pgid = child.pid
            try:
                require(os.getpgid(child.pid) == pgid, 'leaf process group identity differs')
                save(label + '.started.json', dict(pid=child.pid, pgid=pgid, argv=command))
                code = child.wait(timeout=wall)
            except subprocess.TimeoutExpired:
                timed_out = True
            except BaseException as error:
                exception = repr(error)
            finally:
                if timed_out or exception is not None or time.monotonic() >= work_deadline:
                    signal.setitimer(signal.ITIMER_REAL, 0)
                child.poll()
                reaped.extend(reap_group(pgid))
                if child.poll() is None or group_exists(pgid):
                    forced_cleanup = True
                    signal.setitimer(signal.ITIMER_REAL, 0)
                    reaped.extend(stop_group(child, hard_deadline))
                try:
                    code = child.wait(timeout=max(0.001, min(5, hard_deadline - time.monotonic())))
                except subprocess.TimeoutExpired:
                    exception = (exception or '') + ' leader not reaped before cleanup bound'
                reaped.extend(reap_group(pgid))
                group_absent = not group_exists(pgid)
        row = dict(label=label, command=command_pin, argv=command, pid=child.pid, pgid=pgid,
                   exit_code=code, timed_out=timed_out, elapsed_ns=time.monotonic_ns() - leaf_started,
                   natural_exit=code is not None and not timed_out and not forced_cleanup and exception is None,
                   reaped=child.returncode is not None, forced_cleanup=forced_cleanup,
                   process_group_absent=group_absent, adopted_reaped=reaped, exception=exception,
                   stdout=pin(stdout), stderr=pin(stderr))
        results.append(row)
        save(label + '.result.json', row)
        require(code == 0 and row['natural_exit'] and row['reaped'] and group_absent,
                label + ' did not finish with a natural successful reaped exit')

    cargo = str(TOOLCHAIN / 'cargo')
    try:
        run('rustc-version', [str(TOOLCHAIN / 'rustc'), '--version', '--verbose'])
        run('format', [str(TOOLCHAIN / 'rustfmt'), '--edition', '2024',
                       '--config', 'skip_children=true', *sorted(FORMATTED)])
        formatted = sources()
        require(set(formatted) == set(before), 'formatter changed source roster')
        require({name for name in before if before[name] != formatted[name]} <= FORMATTED,
                'formatter changed an unselected source')
        save('sources-formatted.json', formatted)
        run('format-check', [str(TOOLCHAIN / 'rustfmt'), '--edition', '2024', '--check',
                             '--config', 'skip_children=true', *sorted(FORMATTED)])
        run('lock', [cargo, 'metadata', '--offline', '--format-version', '1'])
        tested = sources()
        require(set(tested) == set(formatted)
                and all(tested[name] == value for name, value in formatted.items()
                        if name != 'Cargo.lock'),
                'lock generation changed input other than selected Cargo.lock')
        require(all(tested['Cargo.lock.input'][key] == before['Cargo.lock'][key]
                    for key in ('bytes', 'sha256')), 'original lock input changed')
        lock_delta = lock_pruning()
        lock_delta['feature_evidence'] = spin_feature_evidence(OUT / 'lock.stdout')
        save('sources-tested.json', tested)
        for label, selection in (
            ('aql-tests', ['-p', 'fe2o3-aql']),
            ('kfd-tests', ['-p', 'fe2o3-kfd', '--features', 'engineering-gfx950', '--lib', '--tests']),
            ('probe-tests', ['-p', 'fe2o3-kfd', '--features', 'engineering-gfx950', '--example', COMPATIBILITY_EXAMPLE]),
            ('terminal-probe-tests', ['-p', 'fe2o3-kfd', '--features', 'engineering-gfx950', '--example', TERMINAL_EXAMPLE]),
            ('signal-probe-tests', ['-p', 'fe2o3-kfd', '--features', 'engineering-gfx950', '--example', EXAMPLE]),
        ):
            run(label, [cargo, 'test', '--offline', '--locked', '--jobs', '2',
                        *selection, '--', '--test-threads=1'])
            test_results[label] = test_outcomes(OUT / (label + '.stdout'))
            require(test_results[label]['failed'] == 0 and test_results[label]['passed'] > 0,
                    'libtest reported failure or no passing tests')
        run('probe-build', [cargo, 'build', '--offline', '--locked', '--jobs', '2',
                            '-p', 'fe2o3-kfd', '--features', 'engineering-gfx950',
                            '--example', COMPATIBILITY_EXAMPLE, '--message-format=json'])
        compatibility_binary = selected_example(OUT / 'probe-build.stdout', COMPATIBILITY_EXAMPLE)
        run('terminal-probe-build', [cargo, 'build', '--offline', '--locked', '--jobs', '2',
                                     '-p', 'fe2o3-kfd', '--features', 'engineering-gfx950',
                                     '--example', TERMINAL_EXAMPLE, '--message-format=json'])
        terminal_binary = selected_example(OUT / 'terminal-probe-build.stdout', TERMINAL_EXAMPLE)
        run('signal-probe-build', [cargo, 'build', '--offline', '--locked', '--jobs', '2',
                                   '-p', 'fe2o3-kfd', '--features', 'engineering-gfx950',
                                   '--example', EXAMPLE, '--message-format=json'])
        binary = selected_example(OUT / 'signal-probe-build.stdout', EXAMPLE)
        run('default-check', [cargo, 'check', '--offline', '--locked', '--jobs', '2',
                              '-p', 'fe2o3-kfd', '--no-default-features'])
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)

    final = None
    try:
        final = sources()
        if tested is not None:
            require(final == tested, 'tested source/lock drift')
        elif formatted is not None:
            require(all(final.get(name) == value for name, value in formatted.items())
                    and set(final) == set(formatted), 'formatted source drift')
        else:
            require(set(final) == set(before)
                    and {name for name in before if final[name] != before[name]} <= FORMATTED,
                    'source drift before completed formatting')
        require({name: pin(Path(value['path'])) for name, value in tool_pins.items()} == tool_pins,
                'toolchain or prlimit drift')
        for artifact in (compatibility_binary, terminal_binary, binary):
            if artifact is not None:
                require(pin(Path(artifact['pin']['path'])) == artifact['pin'], 'selected executable drift')
        save('sources-after.json', final)
    except BaseException as error:
        postcheck_errors.append(repr(error))
    if postcheck_errors and failure is None:
        failure = 'postcheck failed'
    if time.monotonic() >= hard_deadline:
        failure = failure or 'whole-run wall bound exceeded'
    result = dict(schema='ferric-peer-dependency-signal-completion-cpu-v1', passed=failure is None,
                  failure=failure, postcheck_errors=postcheck_errors, phases=results,
                  tests=test_results, binary=binary, terminal_binary=terminal_binary,
                  compatibility_binary=compatibility_binary,
                  input_sources=input_pin,
                  raw={path.name: pin(path) for path in sorted(OUT.iterdir()) if path.is_file()},
                  formatted_sources=pin(OUT / 'sources-formatted.json') if (OUT / 'sources-formatted.json').exists() else None,
                  tested_sources=pin(OUT / 'sources-tested.json') if (OUT / 'sources-tested.json').exists() else None,
                  final_sources=pin(OUT / 'sources-after.json') if (OUT / 'sources-after.json').exists() else None,
                  source_unchanged=tested is not None and final == tested,
                  lock_pruning=lock_delta,
                  tool_pins=tool_pins, host=os.uname().nodename,
                  boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                  controller=pin(ROOT / 'run_cpu.py'), elapsed_seconds=time.monotonic() - started,
                  limits=dict(address_space_bytes=AS_LIMIT, cpu_seconds_per_leaf=CPU_LIMIT,
                              file_bytes=FILE_LIMIT, wall_seconds_per_leaf=LEAF_WALL,
                              whole_wall_seconds=WHOLE_WALL, cleanup_reserve_seconds=CLEANUP_RESERVE,
                              affinity=[8, 9], nice=10, cargo_jobs=2, incremental=False),
                  gpu_execution=False, native_peer_ordering_qualified=False,
                  native_probe_invoked=False, performance_claim=False)
    if (ROOT / 'Cargo.lock').is_file():
        result['cargo_lock'] = pin(ROOT / 'Cargo.lock')
    name = 'complete.json' if failure is None else 'failed.json'
    save(name, result)
    print(json.dumps(pin(OUT / name)), flush=True)
    if failure is not None:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
