"""Bounded CPU qualification of the guarded MLP candidate; no device execution."""
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
import sys
import time
import tomllib

ROOT = Path(__file__).resolve().parent
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-segment-cpu-v228-v1')
OUT = ROOT / 'evidence'
TARGET = ROOT / 'target'
TMP = ROOT / 'tmp'
CANDIDATE = ROOT / 'candidate'
DEPENDENCIES = ROOT / 'fe2o3'
COMMIT = '097b4f796a283f554339cc0c0ef5c2c8c3858d2a'
GIT = 'https://github.com/harsh-nod/fe2o3.git'
PACKAGE = 'ferric-qwen3-tp-guarded-mlp-segment-kernels-device-v1'
LIBRARY = 'ferric_qwen3_tp_guarded_mlp_segment_kernels_device_v1'
FORMATTED = {'candidate/src/' + name for name in
             ('lib.rs', 'arithmetic.rs', 'guard.rs', 'kernels.rs', 'guard_tests.rs')}
TOOLCHAIN = Path('/home/harmenon/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/bin')
TOOLCHAIN_LIB = TOOLCHAIN.parent / 'lib'
SHARED_LIBRARIES = {
    'libLLVM.so.22.1-rust-1.96.0-nightly': 199520544,
    'librustc_driver-7bb70639c3ace5a4.so': 152936640,
    'libLLVM-22-rust-1.96.0-nightly.so': 43,
}
WHOLE_WALL = 7200
CLEANUP_RESERVE = 50
LEAF_WALL = 1800
AS_LIMIT = 12 << 30
FILE_LIMIT = 1 << 30
CPU_LIMIT = 1200
CACHE_LIMIT = 6 << 30
STREAM_LIMIT = 64 << 20
START_FREE = 40 << 30
LIVE_FREE = 38 << 30

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
        if child.returncode is not None:
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
            if child.returncode is not None:
                reaped.extend(reap_group(child.pid))
            if not group_exists(child.pid):
                break
            time.sleep(0.02)
    return reaped


def interrupted(signum, _frame):
    raise RuntimeError('interrupted by signal ' + str(signum))


def files_below(root, packed=True):
    require(root.is_dir() and not root.is_symlink(), 'missing ordinary source directory')
    result = []
    for directory, dirs, names in os.walk(root, followlinks=False,
                                          onerror=lambda error: (_ for _ in ()).throw(error)):
        require((not packed or '.git' not in dirs) and 'target' not in dirs,
                'source pack contains Git/cache directory')
        for name in dirs:
            require(not (Path(directory) / name).is_symlink(), 'source directory alias')
        result.extend(Path(directory) / name for name in names)
    return result


def sources():
    files = [ROOT / 'run_cpu.py', ROOT / 'Cargo.lock.input']
    files.extend(files_below(DEPENDENCIES))
    files.extend(files_below(CANDIDATE))
    require(len(files) <= 20000, 'source roster bound exceeded')
    return {str(path.relative_to(ROOT)): pin(path) for path in sorted(files)}


def extent(pin_value):
    return {key: pin_value[key] for key in ('bytes', 'sha256')}


def configuration_pins():
    paths = {parent / '.cargo' / name for parent in (ROOT, *ROOT.parents)
             for name in ('config', 'config.toml')}
    paths |= {CANDIDATE / '.cargo' / name for name in ('config', 'config.toml')}
    paths |= {Path('/home/harmenon/.cargo') / name for name in ('config', 'config.toml')}
    result = {str(path): pin(path) if os.path.lexists(path) else None for path in sorted(paths)}
    require(not any(result.values()), 'inherited Cargo configuration is forbidden')
    return result


def input_contract(expected_sha):
    manifest_path = ROOT / 'input-manifest.json'
    manifest_pin = pin(manifest_path)
    require(manifest_pin['sha256'] == expected_sha, 'root-supplied input manifest hash differs')
    value = json.loads(manifest_path.read_bytes())
    require(set(value) == {'schema', 'dependency_commit', 'files', 'expected_tests', 'tool_pins'}
            and value['schema'] == 'ferric-guarded-mlp-segment-cpu-input-v1'
            and value['dependency_commit'] == COMMIT, 'input manifest generation/fields')
    expected = value['expected_tests']
    require(isinstance(expected, list) and len(expected) == 27
            and expected == sorted(set(expected))
            and sum(name.startswith('arithmetic::tests::') for name in expected) == 12
            and sum(name.startswith('guard_tests::') for name in expected) == 15
            and all(re.fullmatch(r'[A-Za-z0-9_:]+', name) for name in expected),
            'exact authored 12 arithmetic plus 15 guard tests required')
    before = sources()
    require(value['files'] == {name: extent(row) for name, row in before.items()},
            'source pack differs from exact root-pinned roster')
    require({str(p.relative_to(CANDIDATE)) for p in files_below(CANDIDATE)}
            == {'Cargo.toml', 'Cargo.toml.input', 'Cargo.lock',
                *(str(Path(name).relative_to('candidate')) for name in FORMATTED)},
            'candidate must contain only six authored bodies and two retained Cargo inputs')
    require(extent(before['candidate/Cargo.lock']) == extent(before['Cargo.lock.input'])
            == extent(before['fe2o3/Cargo.lock']), 'seed lock is not the original RT lock')
    canonical = tomllib.loads((CANDIDATE / 'Cargo.toml.input').read_text())
    local = tomllib.loads((CANDIDATE / 'Cargo.toml').read_text())
    require(canonical['package']['name'] == PACKAGE
            and canonical['lib']['name'] == LIBRARY
            and canonical['lib']['path'] == 'src/lib.rs', 'candidate package/target identity')
    expected_local = json.loads(json.dumps(canonical))
    for table, name in ((expected_local['dependencies'], 'fe2o3-device'),
                        (expected_local['target']['cfg(not(target_arch = "amdgpu"))']['dependencies'],
                         'fe2o3-host')):
        dependency = table[name]
        require(dependency == {'git': GIT, 'rev': COMMIT, 'version': '=0.1.0'},
                'canonical candidate dependency declaration changed')
        del dependency['git']
        del dependency['rev']
        dependency['path'] = '../fe2o3/crates/' + name
    require(local == expected_local, 'only the two declared git/rev to local-path changes are allowed')
    return value, manifest_pin, before


def inventory(text):
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', text, re.M)
    require(len(names) == len(set(names)) and ': benchmark' not in text, 'invalid libtest inventory')
    return names


def test_outcomes(path, expected):
    text = path.read_text()
    named = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', text, re.M)
    summaries = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
                           r'(\d+) ignored; (\d+) measured; (\d+) filtered out;', text, re.M)
    require(len(named) == len(expected) and {name for name, _ in named} == set(expected)
            and all(outcome == 'ok' for _, outcome in named)
            and summaries == [('ok', str(len(expected)), '0', '0', '0', '0')],
            'all exact candidate tests must pass once with no ignored or filtered cases')
    return dict(passed=len(expected), failed=0, ignored=0, names=sorted(expected))


def artifact(path, testing):
    records = [json.loads(line) for line in path.read_text().splitlines() if line.startswith('{')]
    require([row.get('success') for row in records if row.get('reason') == 'build-finished'] == [True],
            'missing unique Cargo build-finished success')
    rows = [row for row in records if row.get('reason') == 'compiler-artifact'
            and row.get('manifest_path') == str(CANDIDATE / 'Cargo.toml')
            and row.get('target', {}).get('name') == LIBRARY
            and row['target'].get('kind') == ['lib']
            and row.get('profile', {}).get('test') is testing]
    require(len(rows) == 1, 'exact candidate library artifact required')
    row = rows[0]
    require(row['profile'].get('opt_level') == '0'
            and row['profile'].get('debug_assertions') is True
            and row['profile'].get('overflow_checks') is True, 'checked debug build profile required')
    paths = [Path(row['executable'])] if testing else [
        Path(name) for name in row['filenames'] if name.endswith('.rlib')]
    require(len(paths) == 1, 'one selected test ELF or host rlib required')
    selected = paths[0]
    require(selected.is_relative_to(TARGET) and selected.resolve(strict=True) == selected,
            'artifact outside fresh target')
    if testing:
        require(os.access(selected, os.X_OK), 'selected test is not executable')
        with selected.open('rb') as stream:
            require(stream.read(4) == b'\x7fELF', 'selected test is not ELF')
    return dict(pin=pin(selected), cargo_artifact=row)


def lock_contract(metadata):
    original = tomllib.loads((ROOT / 'Cargo.lock.input').read_text())
    current = tomllib.loads((CANDIDATE / 'Cargo.lock').read_text())
    require(original.get('version') == current.get('version'), 'lock format changed')
    def index(document):
        result = {}
        for row in document['package']:
            key = (row['name'], row['version'], row.get('source'))
            require(key not in result, 'duplicate lock package identity')
            result[key] = row
        return result
    old, new = index(original), index(current)
    candidate_key = (PACKAGE, '0.1.0', None)
    require(set(new) - set(old) == {candidate_key}, 'only the candidate root may be added to the lock')
    def dependencies(row, packages):
        keys = []
        for entry in row.get('dependencies', []):
            matches = []
            for key in packages:
                name, version, source = key
                forms = {name, name + ' ' + version}
                if source:
                    forms.add(name + ' ' + version + ' (' + source + ')')
                if entry in forms:
                    matches.append(key)
            require(len(matches) == 1, 'ambiguous resolved lock dependency: ' + entry)
            keys.append(matches[0])
        require(len(keys) == len(set(keys)), 'duplicate resolved lock dependency')
        return set(keys)
    package_rows = {(p['name'], p['version'], p.get('source')): p for p in metadata['packages']}
    nodes = {node['id']: node for node in metadata['resolve']['nodes']}
    removed_edges = []
    for key, row in new.items():
        if key == candidate_key:
            require(set(row) == {'name', 'version', 'dependencies'}
                    and dependencies(row, new)
                    == {('fe2o3-device', '0.1.0', None), ('fe2o3-host', '0.1.0', None)},
                    'candidate lock root changed')
            continue
        previous = old[key]
        require({k: v for k, v in row.items() if k != 'dependencies'}
                == {k: v for k, v in previous.items() if k != 'dependencies'},
                'locked version/source/checksum/package fields changed')
        earlier, later = dependencies(previous, old), dependencies(row, new)
        require(later <= earlier, 'new dependency edge in retained lock package')
        for dependency in sorted(earlier - later, key=repr):
            package = package_rows.get(key)
            require(package is not None and package['id'] in nodes,
                    'removed dependency requires actual resolved feature evidence')
            node = nodes[package['id']]
            declarations = [d for d in package['dependencies']
                            if d['name'] == dependency[0] and d.get('source') == dependency[2]]
            aliases = {(d.get('rename') or d['name']).replace('-', '_') for d in declarations}
            require(declarations and not any(d['name'].replace('-', '_') in aliases for d in node['deps']),
                    'removed dependency has a live resolved edge')
            if all(d.get('kind') == 'dev' for d in declarations):
                require(package['id'] not in metadata['workspace_members'],
                        'cannot prune workspace-member dev dependency')
                removed_edges.append(dict(package=list(key), dependency=list(dependency),
                                          reason='dev-not-workspace', declarations=declarations,
                                          enabled_features=node['features']))
                continue
            declarations = [d for d in declarations if d.get('kind') != 'dev']
            require(len(declarations) == 1, 'removed edge is not one declared optional dependency')
            declaration = declarations[0]
            require(declaration.get('optional') is True,
                    'required normal/build dependency cannot be pruned')
            alias = declaration.get('rename') or declaration['name']
            active = [item for feature in node['features']
                      for item in package['features'].get(feature, [])]
            require(alias not in node['features']
                    and 'dep:' + alias not in active
                    and not any(item.startswith(alias + '/') for item in active),
                    'enabled feature activates removed optional dependency')
            removed_edges.append(dict(package=list(key), dependency=list(dependency),
                                      reason='optional-feature-disabled', alias=alias,
                                      enabled_features=node['features']))
    return dict(original_packages=len(old), selected_packages=len(new),
                new_candidate=list(candidate_key), removed_packages=[list(k) for k in sorted(set(old) - set(new), key=repr)],
                removed_inactive_edges=removed_edges)


def metadata_contract(path):
    value = json.loads(path.read_bytes())
    require(value['target_directory'] == str(TARGET)
            and len(value['workspace_members']) == 1, 'candidate-only workspace/fresh target required')
    local, external = {}, {}
    candidate = []
    for row in value['packages']:
        source = Path(row['manifest_path'])
        require(source.resolve(strict=True) == source, 'aliased Cargo dependency source')
        if row.get('source') is None:
            if source == CANDIDATE / 'Cargo.toml':
                candidate.append(row)
            else:
                require(source.is_relative_to(DEPENDENCIES / 'crates'), 'undeclared local dependency')
                local[row['name']] = str(source)
        else:
            require(source.is_relative_to(Path('/home/harmenon/.cargo'))
                    and (row['source'].startswith('registry+') or row['source'].startswith('git+')),
                    'external dependency outside existing offline Cargo cache')
            external[str(source.parent)] = {str(p.relative_to(source.parent)): pin(p)
                                           for p in files_below(source.parent, packed=False)}
    require(len(candidate) == 1 and candidate[0]['id'] == value['workspace_members'][0]
            and {'fe2o3-device', 'fe2o3-host', 'fe2o3-macros'} <= set(local),
            'exact new local typed-atomic dependency closure required')
    targets = candidate[0]['targets']
    require(len(targets) == 1 and targets[0]['name'] == LIBRARY and targets[0]['kind'] == ['lib'],
            'unexpected candidate target')
    return value, external, local


def scratch_bytes():
    total = 0
    count = 0
    for root in (TARGET, TMP):
        if not root.exists():
            continue
        for directory, _, names in os.walk(root, followlinks=False,
                                           onerror=lambda error: (_ for _ in ()).throw(error)):
            for name in names:
                try:
                    total += (Path(directory) / name).lstat().st_size
                except FileNotFoundError:
                    # Cargo removes/renames temporary files during this bounded inventory.
                    continue
                count += 1
                require(count <= 200000, 'scratch inventory bound')
    return total


def main():
    started = time.monotonic()
    hard_deadline = started + WHOLE_WALL
    work_deadline = hard_deadline - CLEANUP_RESERVE
    require(__debug__ and len(sys.argv) == 2 and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]),
            'usage: python3 -B run_cpu.py ACTUAL_INPUT_MANIFEST_SHA256')
    require(sys.dont_write_bytecode and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'unexpected CPU host/UID')
    require(ROOT == EXPECTED_ROOT and not any(path.exists() for path in (OUT, TARGET, TMP)),
            'fresh exact namespace required')
    require(shutil.disk_usage(ROOT).free >= START_FREE, 'initial 40 GiB storage floor')
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
    inputs, input_manifest, before = input_contract(sys.argv[1])
    configurations = configuration_pins()
    tool_pins = {name: pin(TOOLCHAIN / name) for name in ('rustc', 'rustdoc', 'rustfmt', 'cargo')}
    tool_pins['prlimit'] = pin(Path('/usr/bin/prlimit'))
    require(TOOLCHAIN_LIB.is_dir() and not TOOLCHAIN_LIB.is_symlink()
            and TOOLCHAIN_LIB.resolve() == TOOLCHAIN_LIB, 'noncanonical toolchain library directory')
    for name, count in SHARED_LIBRARIES.items():
        path = TOOLCHAIN_LIB / name
        require(path.resolve() == path, 'noncanonical toolchain library')
        tool_pins[name] = pin(path)
        require(tool_pins[name]['bytes'] == count, 'unexpected compiler library extent')
    require(tool_pins == inputs['tool_pins'], 'actual toolchain differs from root-pinned tool roster')
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, interrupted)
    env = dict(HOME='/home/harmenon', PATH=str(TOOLCHAIN) + ':/usr/bin:/bin',
               CARGO_HOME='/home/harmenon/.cargo', CARGO_TARGET_DIR=str(TARGET),
               LD_LIBRARY_PATH=str(TOOLCHAIN_LIB), TMPDIR=str(TMP),
               RUSTC=str(TOOLCHAIN / 'rustc'), RUSTDOC=str(TOOLCHAIN / 'rustdoc'),
               CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', RUST_BACKTRACE='1',
               CARGO_PROFILE_DEV_OPT_LEVEL='0', CARGO_PROFILE_DEV_DEBUG='0',
               CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
               CARGO_PROFILE_TEST_OPT_LEVEL='0', CARGO_PROFILE_TEST_DEBUG='0',
               CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
               ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
    input_pin = save('sources-input.json', before)
    phases, artifacts, external_sources, local_dependencies = [], {}, {}, {}
    formatted = tested = final = tests = lock_delta = None
    failure = None
    postcheck_errors = []
    signal.setitimer(signal.ITIMER_REAL, max(0.001, work_deadline - time.monotonic()))

    def run(label, argv, seconds=LEAF_WALL):
        require(shutil.disk_usage(ROOT).free >= LIVE_FREE and scratch_bytes() <= CACHE_LIMIT,
                'pre-leaf storage bound')
        remaining = work_deadline - time.monotonic()
        require(remaining > 0, 'whole-run work deadline reached')
        wall = min(seconds, remaining)
        command = ['/usr/bin/prlimit', '--as=' + str(AS_LIMIT), '--cpu=' + str(CPU_LIMIT),
                   '--fsize=' + str(FILE_LIMIT), '--core=0', '--', *argv]
        command_pin = save(label + '.command.json', dict(argv=command, cwd=str(ROOT), env=env,
                                                        wall_timeout_seconds=wall))
        leaf_started = time.monotonic_ns()
        stdout, stderr = OUT / (label + '.stdout'), OUT / (label + '.stderr')
        timed_out = forced_cleanup = False
        exception = storage_failure = None
        code = None
        reaped = []
        deferred = []
        def defer(signum, _frame):
            deferred.append(signum)
        with stdout.open('xb') as so, stderr.open('xb') as se:
            handled = (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM)
            previous_handlers = {sig: signal.getsignal(sig) for sig in handled}
            for sig in handled:
                signal.signal(sig, defer)
            child = None
            try:
                require(not deferred, 'termination requested before spawn')
                child = subprocess.Popen(command, cwd=ROOT, env=env, stdin=subprocess.DEVNULL,
                                         stdout=so, stderr=se, start_new_session=True)
                pgid = child.pid
                require(os.getpgid(child.pid) == pgid, 'leaf process group identity differs')
                save(label + '.started.json', dict(pid=child.pid, pgid=pgid, argv=command))
                leaf_deadline = time.monotonic() + wall
                next_storage = 0.0
                while child.poll() is None:
                    now = time.monotonic()
                    if deferred:
                        exception = 'deferred signals: ' + repr(deferred)
                        break
                    if now >= leaf_deadline:
                        timed_out = True
                        break
                    if max(os.fstat(so.fileno()).st_size, os.fstat(se.fileno()).st_size) > STREAM_LIMIT:
                        storage_failure = 'stream cap'
                        break
                    if now >= next_storage:
                        if shutil.disk_usage(ROOT).free < LIVE_FREE or scratch_bytes() > CACHE_LIMIT:
                            storage_failure = 'storage floor/cache cap'
                            break
                        next_storage = now + 2
                    time.sleep(0.1)
                code = child.poll()
            except BaseException as error:
                exception = repr(error)
            finally:
                signal.setitimer(signal.ITIMER_REAL, 0)
                try:
                    if child is not None:
                        child.poll()
                        if child.returncode is not None:
                            reaped.extend(reap_group(pgid))
                        if child.poll() is None or group_exists(pgid):
                            forced_cleanup = True
                            reaped.extend(stop_group(child, hard_deadline))
                        try:
                            code = child.wait(timeout=max(0.001, min(5, hard_deadline - time.monotonic())))
                        except subprocess.TimeoutExpired:
                            exception = (exception or '') + ' leader not reaped within cleanup bound'
                        if child.returncode is not None:
                            reaped.extend(reap_group(pgid))
                        group_absent = not group_exists(pgid)
                finally:
                    for sig, previous in previous_handlers.items():
                        signal.signal(sig, previous)
        require(child is not None, 'spawn failed before owned child registration: ' + str(exception))
        if deferred:
            exception = (exception or '') + ' deferred signals: ' + repr(deferred)
        if time.monotonic() >= work_deadline:
            timed_out = True
        row = dict(label=label, command=command_pin, argv=command, pid=child.pid, pgid=pgid,
                   exit_code=code, timed_out=timed_out, elapsed_ns=time.monotonic_ns() - leaf_started,
                   natural_exit=code is not None and not timed_out and not forced_cleanup
                       and exception is None and storage_failure is None,
                   reaped=child.returncode is not None, forced_cleanup=forced_cleanup,
                   process_group_absent=group_absent, adopted_reaped=reaped, exception=exception,
                   observed_signals=deferred,
                   storage_failure=storage_failure, stdout=pin(stdout), stderr=pin(stderr))
        phases.append(row)
        save(label + '.result.json', row)
        require(code == 0 and row['natural_exit'] and row['reaped'] and group_absent,
                label + ' did not finish naturally/reaped/successfully')
        require(max(row['stdout']['bytes'], row['stderr']['bytes']) <= STREAM_LIMIT
                and shutil.disk_usage(ROOT).free >= LIVE_FREE and scratch_bytes() <= CACHE_LIMIT,
                'post-leaf storage bound')
        if tested is not None:
            require(sources() == tested, 'qualified source/lock drift during phase')
        signal.setitimer(signal.ITIMER_REAL, max(0.001, work_deadline - time.monotonic()))

    cargo = str(TOOLCHAIN / 'cargo')
    selection = ['--manifest-path', str(CANDIDATE / 'Cargo.toml'),
                 '--no-default-features', '--features', 'gfx950']
    try:
        run('rustc-version', [str(TOOLCHAIN / 'rustc'), '--version', '--verbose'], 60)
        run('format', [str(TOOLCHAIN / 'rustfmt'), '--edition', '2024',
                       '--config', 'skip_children=true', *sorted(FORMATTED)], 60)
        formatted = sources()
        require(set(formatted) == set(before)
                and {name for name in before if before[name] != formatted[name]} <= FORMATTED,
                'formatter changed an unselected source')
        save('sources-formatted.json', formatted)
        run('format-check', [str(TOOLCHAIN / 'rustfmt'), '--edition', '2024', '--check',
                             '--config', 'skip_children=true', *sorted(FORMATTED)], 60)
        run('metadata', [cargo, 'metadata', '--offline', '--format-version', '1', *selection], 120)
        metadata_sources = sources()
        require(set(metadata_sources) == set(formatted)
                and all(metadata_sources[name] == row for name, row in formatted.items()
                        if name != 'candidate/Cargo.lock'), 'metadata changed an input other than selected lock')
        metadata, external_sources, local_dependencies = metadata_contract(OUT / 'metadata.stdout')
        lock_delta = lock_contract(metadata)
        save('dependencies-before.json', external_sources)
        tested = metadata_sources
        save('sources-tested.json', tested)
        run('host-check', [cargo, 'check', '--offline', '--locked', '--jobs', '2', '--lib', *selection])
        run('build-tests', [cargo, 'test', '--offline', '--locked', '--jobs', '2',
                            '--lib', '--no-run', '--message-format=json', *selection])
        artifacts['library-tests'] = artifact(OUT / 'build-tests.stdout', True)
        executable = artifacts['library-tests']['pin']['path']
        run('lib-list', [executable, '--list', '--format', 'terse'], 120)
        require(sorted(inventory((OUT / 'lib-list.stdout').read_text())) == inputs['expected_tests'],
                'compiled inventory differs from frozen 27-name census')
        run('lib-ignored', [executable, '--ignored', '--list', '--format', 'terse'], 120)
        require(not inventory((OUT / 'lib-ignored.stdout').read_text()), 'unexpected ignored candidate test')
        run('lib-tests', [executable, '--test-threads=1'], 300)
        tests = test_outcomes(OUT / 'lib-tests.stdout', inputs['expected_tests'])
        run('host-build', [cargo, 'build', '--offline', '--locked', '--jobs', '2',
                           '--lib', '--message-format=json', *selection])
        artifacts['host-library'] = artifact(OUT / 'host-build.stdout', False)
        require(len(phases) == 10, 'ten complete CPU phases required')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)

    def check(label, function):
        try:
            function()
        except BaseException as error:
            postcheck_errors.append(label + ': ' + repr(error))
    def source_postcheck():
        nonlocal final
        final = sources()
        if tested is not None:
            require(final == tested, 'tested sources/lock drift')
        elif formatted is not None:
            require(set(final) == set(formatted)
                    and all(final[name] == row for name, row in formatted.items()
                            if name != 'candidate/Cargo.lock'),
                    'source drift before qualified metadata')
        else:
            require(set(final) == set(before)
                    and {name for name in before if final[name] != before[name]} <= FORMATTED,
                    'source drift before formatting completion')
        save('sources-after.json', final)
    check('sources', source_postcheck)
    check('input manifest', lambda: require(pin(ROOT / 'input-manifest.json') == input_manifest,
                                           'input manifest drift'))
    check('tools', lambda: require({name: pin(Path(row['path'])) for name, row in tool_pins.items()}
                                   == tool_pins, 'tool drift'))
    check('configuration', lambda: require(configuration_pins() == configurations, 'Cargo config drift'))
    final_external = {}
    for directory, original in external_sources.items():
        def dependency_postcheck(directory=directory, original=original):
            path = Path(directory)
            actual = {str(p.relative_to(path)): pin(p) for p in files_below(path, packed=False)}
            final_external[directory] = actual
            require(actual == original, 'offline dependency source drift')
        check('dependency:' + directory, dependency_postcheck)
    if external_sources:
        save('dependencies-after.json', final_external)
    for name, row in artifacts.items():
        check(name, lambda row=row: require(pin(Path(row['pin']['path'])) == row['pin'],
                                           'selected artifact drift'))
    if postcheck_errors:
        failure = failure or 'postcheck failed'
    if time.monotonic() >= hard_deadline:
        failure = failure or 'whole-run wall bound exceeded'
    result = dict(schema='ferric-guarded-mlp-segment-cpu-v1', passed=failure is None,
                  failure=failure, postcheck_errors=postcheck_errors, phases=phases, tests=tests,
                  artifacts=artifacts, dependency_commit=COMMIT, local_dependencies=local_dependencies,
                  input_manifest=input_manifest, input_sources=input_pin,
                  raw={p.name: pin(p) for p in sorted(OUT.iterdir()) if p.is_file()},
                  formatted_sources=pin(OUT / 'sources-formatted.json') if (OUT / 'sources-formatted.json').exists() else None,
                  tested_sources=pin(OUT / 'sources-tested.json') if (OUT / 'sources-tested.json').exists() else None,
                  final_sources=pin(OUT / 'sources-after.json') if (OUT / 'sources-after.json').exists() else None,
                  source_unchanged=tested is not None and final == tested,
                  lock_transition=lock_delta, tool_pins=tool_pins, configurations=configurations,
                  host=os.uname().nodename, boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                  controller=pin(ROOT / 'run_cpu.py'), elapsed_seconds=time.monotonic() - started,
                  limits=dict(address_space_bytes=AS_LIMIT, cpu_seconds_per_leaf=CPU_LIMIT,
                              file_bytes=FILE_LIMIT, cache_bytes=CACHE_LIMIT, stream_bytes=STREAM_LIMIT,
                              initial_free_bytes=START_FREE, live_free_bytes=LIVE_FREE,
                              wall_seconds_per_leaf=LEAF_WALL, whole_wall_seconds=WHOLE_WALL,
                              cleanup_reserve_seconds=CLEANUP_RESERVE, affinity=[8, 9],
                              nice=10, cargo_jobs=2, incremental=False),
                  gpu_execution=False, compiler_hsaco_reproduced=False, numerical_acceptance=False,
                  full_model_acceptance=False, performance_claim=False, production_authority=False,
                  qualified_rpo_dependency_identity_claim=False)
    name = 'complete.json' if failure is None else 'failed.json'
    save(name, result)
    print(json.dumps(pin(OUT / name)), flush=True)
    if failure is not None:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
