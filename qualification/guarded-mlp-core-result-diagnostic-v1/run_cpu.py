"""Bounded non-admitting core Result diagnostic; no GPU or provider qualification."""
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

ROOT = Path(__file__).resolve().parent
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-core-result-diagnostic-cpu-v228-v1')
SOURCE = ROOT / 'fe2o3'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
TOOLCHAIN = Path('/home/harmenon/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/bin')
TOOLCHAIN_LIB = TOOLCHAIN.parent / 'lib'
SHARED_LIBRARIES = {
    'libLLVM.so.22.1-rust-1.96.0-nightly': 199520544,
    'librustc_driver-7bb70639c3ace5a4.so': 152936640,
    'libLLVM-22-rust-1.96.0-nightly.so': 43,
}
WHOLE_WALL, CLEANUP_RESERVE, LEAF_WALL = 7200, 50, 1800
AS_LIMIT, FILE_LIMIT, CPU_LIMIT = 12 << 30, 1 << 30, 1200
CACHE_LIMIT, STREAM_LIMIT = 6 << 30, 64 << 20
START_FREE, LIVE_FREE = 40 << 30, 38 << 30
ROLES = {
    'device-lib': ('fe2o3-device', 'fe2o3_device', 'lib'),
    'device-api-ui': ('fe2o3-device', 'device_api_ui', 'test'),
    'device-ffi-ui': ('fe2o3-device', 'device_ffi_ui', 'test'),
    'compiler-lib': ('rustc-codegen-fe2o3', 'rustc_codegen_fe2o3', 'dylib'),
    'atomic-extraction': ('rustc-codegen-fe2o3', 'production_extraction_driver_v1', 'test'),
    'matrix-extraction': ('rustc-codegen-fe2o3', 'production_general_matrix_driver_v1', 'test'),
}
BASE_SOURCE_GENERATION = 'a057bf3b0e9175f481a31181b19d587d342bf8f3'
MATRIX = 'dynamic_matrix_kernel_reaches_gfx942_llvm'
DIAGNOSTIC_PREFIX = 'collector::core_result_diagnostic_v1::tests::'
DIAGNOSTIC_TESTS = tuple(DIAGNOSTIC_PREFIX + name for name in (
    'core_result_diagnostic_complete_has_explicit_non_authority_trailer',
    'core_result_diagnostic_partial_is_not_complete',
    'core_result_diagnostic_large_write_is_bounded',
    'core_result_diagnostic_exact_payload_cap_leaves_trailer_space',
    'core_result_diagnostic_utf8_cap_uses_bytes_and_keeps_valid_text',
    'core_result_diagnostic_render_failure_preserves_partial_evidence',
))
DIAGNOSTIC_SOURCE_FILES = {
    'fe2o3/crates/rustc-codegen-fe2o3/src/collector.rs',
    'fe2o3/crates/rustc-codegen-fe2o3/src/collector/core_result_diagnostic_v1.rs',
    'fe2o3/crates/rustc-codegen-fe2o3/src/collector/core_result_diagnostic_v1_tests.rs',
}

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
        require(not packed or ('.git' not in dirs and 'target' not in dirs),
                'source pack contains Git/cache directory')
        for name in dirs:
            require(not (Path(directory) / name).is_symlink(), 'source directory alias')
        result.extend(Path(directory) / name for name in names)
    return result



def sources():
    files = [ROOT / 'run_cpu.py', *files_below(SOURCE)]
    require(len(files) <= 20000, 'source file-count bound')
    return {str(p.relative_to(ROOT)): pin(p) for p in sorted(files)}


def configurations():
    paths = {parent / '.cargo' / name for parent in (SOURCE, ROOT, *ROOT.parents)
             for name in ('config', 'config.toml')}
    paths |= {Path('/home/harmenon/.cargo') / name for name in ('config', 'config.toml')}
    values = {str(p): pin(p) if os.path.lexists(p) else None for p in sorted(paths)}
    require(not any(values.values()), 'inherited Cargo configuration refused')
    return values


def input_contract(digest):
    actual = pin(ROOT / 'input-manifest.json')
    require(actual['sha256'] == digest, 'literal input manifest mismatch')
    value = json.loads((ROOT / 'input-manifest.json').read_bytes())
    require(set(value) == {'schema', 'base_source_generation', 'files', 'tool_pins', 'diagnostic_source_files'}
            and value['schema'] == 'ferric-guarded-mlp-core-result-diagnostic-input-v1'
            and value['base_source_generation'] == BASE_SOURCE_GENERATION, 'diagnostic base generation/fields')
    before = sources()
    require(value['files'] == {name: {k: row[k] for k in ('bytes', 'sha256')}
                               for name, row in before.items()}, 'root-pinned source roster mismatch')
    require(type(value['diagnostic_source_files']) is dict
            and set(value['diagnostic_source_files']) == DIAGNOSTIC_SOURCE_FILES
            and value['diagnostic_source_files'] == {name: value['files'][name]
                                                    for name in DIAGNOSTIC_SOURCE_FILES},
            'exact three diagnostic source bodies must join the pinned source roster')
    require('fe2o3/Cargo.lock' in before and 'fe2o3/Cargo.toml' in before, 'full workspace inputs required')
    return value, actual, before


def metadata_contract():
    value = json.loads((OUT / 'metadata.stdout').read_bytes())
    require(value['workspace_root'] == str(SOURCE) and value['target_directory'] == str(TARGET),
            'metadata workspace/target mismatch')
    packages = {row['id']: row for row in value['packages']}
    require(len(packages) == len(value['packages']), 'duplicate metadata package')
    require(all(Path(packages[k]['manifest_path']).is_relative_to(SOURCE)
                for k in value['workspace_members']), 'workspace member outside pinned source')
    local, external = {}, {}
    for row in packages.values():
        path = Path(row['manifest_path'])
        require(path.resolve(strict=True) == path, 'dependency path alias')
        if row['source'] is None:
            require(path.is_relative_to(SOURCE), 'unlisted local dependency')
            local[row['name']] = str(path)
        else:
            require(path.is_relative_to(Path('/home/harmenon/.cargo'))
                    and row['source'].startswith(('registry+', 'git+')), 'external dependency source')
            external[str(path.parent)] = {str(p.relative_to(path.parent)): pin(p)
                                         for p in files_below(path.parent, packed=False)}
    for package, name, kind in ROLES.values():
        rows = [p for p in packages.values() if p['manifest_path'] == str(SOURCE / 'crates' / package / 'Cargo.toml')]
        require(len(rows) == 1 and any(t['name'] == name and kind in t['kind']
                                      for t in rows[0]['targets']), 'selected Cargo target absent')
    return external, local


def build_records(path):
    rows = [json.loads(line) for line in path.read_text().splitlines() if line.startswith('{')]
    require([row.get('success') for row in rows if row.get('reason') == 'build-finished'] == [True],
            'one successful Cargo build-finished record required')
    return rows


def select_artifact(records, package, target, kind, testing, suffix=None):
    rows = [row for row in records if row.get('reason') == 'compiler-artifact'
            and row.get('manifest_path') == str(SOURCE / 'crates' / package / 'Cargo.toml')
            and row.get('target', {}).get('name') == target and kind in row['target'].get('kind', [])
            and row.get('profile', {}).get('test') is testing]
    require(len(rows) == 1, 'unique selected artifact required: ' + target)
    row = rows[0]
    if suffix:
        paths = [Path(p) for p in row['filenames'] if p.endswith(suffix)]
    else:
        paths = [Path(row['executable'])] if row.get('executable') else []
    require(len(paths) == 1, 'selected artifact extent/role')
    selected = paths[0]
    require(selected.is_relative_to(TARGET) and selected.resolve(strict=True) == selected,
            'selected artifact outside fresh target')
    with selected.open('rb') as stream:
        require(stream.read(4) == b'\x7fELF', 'selected product is not ELF')
    return dict(pin=pin(selected), cargo_artifact=row)


def inventory(path):
    text = path.read_text()
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', text, re.M)
    require(len(names) == len(set(names)) and ': benchmark' not in text, 'ambiguous libtest inventory')
    return sorted(names)


def outcomes(path, selected, total):
    text = path.read_text()
    named = re.findall(r'^test ([A-Za-z0-9_:]+)(?: - should panic)? \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', text, re.M)
    summaries = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
                           r'(\d+) ignored; (\d+) measured; (\d+) filtered out;', text, re.M)
    require(selected and len(named) == len(selected)
            and sorted(name for name, _ in named) == sorted(selected)
            and all(status == 'ok' for _, status in named)
            and summaries == [('ok', str(len(selected)), '0', '0', '0', str(total - len(selected)))],
            'selected named tests must all pass exactly once, with no hidden ignored cases')
    return dict(names=sorted(selected), passed=len(selected), failed=0, ignored=0,
                filtered_out=total-len(selected))


def scratch_bytes():
    total = 0
    count = 0
    for root in (TARGET, TMP):
        if not root.exists():
            continue
        def walk_error(error):
            path = Path(error.filename) if isinstance(error.filename, str) else None
            if (isinstance(error, FileNotFoundError) and path is not None
                    and path.is_absolute() and '..' not in path.parts
                    and path != root and path.is_relative_to(root)):
                # Cargo may remove an owned cache descendant between directory scans.
                return
            raise error
        for directory, _, names in os.walk(root, followlinks=False, onerror=walk_error):
            for name in names:
                try:
                    total += (Path(directory) / name).lstat().st_size
                except FileNotFoundError:
                    # Cargo removes/renames temporary files during this bounded inventory.
                    continue
                count += 1
                require(count <= 200000, 'scratch inventory bound')
    return total


def run(label, argv, env, phases, hard_deadline, tested, seconds=LEAF_WALL, cwd=ROOT):
    work_deadline = hard_deadline - CLEANUP_RESERVE
    require(shutil.disk_usage(ROOT).free >= LIVE_FREE and scratch_bytes() <= CACHE_LIMIT,
            'pre-leaf storage bound')
    remaining = work_deadline - time.monotonic()
    require(remaining > 0, 'whole-run work deadline reached')
    wall = min(seconds, remaining)
    command = ['/usr/bin/prlimit', '--as=' + str(AS_LIMIT), '--cpu=' + str(CPU_LIMIT),
               '--fsize=' + str(FILE_LIMIT), '--core=0', '--', *argv]
    command_pin = save(label + '.command.json', dict(argv=command, cwd=str(cwd), env=env,
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
            child = subprocess.Popen(command, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
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


def main():
    started = time.monotonic()
    hard_deadline = started + WHOLE_WALL
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]), 'python3 -B run_cpu.py ACTUAL_INPUT_SHA')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve() == ROOT
            and not any(os.path.lexists(p) for p in (OUT, TARGET, TMP)), 'fresh exact namespace required')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'unexpected host/UID')
    require(shutil.disk_usage(ROOT).free >= START_FREE, 'initial40GiB floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected nice')
    if priority == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, AS_LIMIT), (resource.RLIMIT_FSIZE, FILE_LIMIT)):
        soft, hard = resource.getrlimit(kind)
        effective = min([cap] + [n for n in (soft, hard) if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (effective, effective))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup failed')
    inputs, input_pin, before = input_contract(sys.argv[1])
    config_before = configurations()
    tool_pins = {name: pin(TOOLCHAIN / name) for name in ('rustc', 'rustdoc', 'rustfmt', 'cargo')}
    tool_pins['prlimit'] = pin(Path('/usr/bin/prlimit'))
    require(TOOLCHAIN_LIB.resolve(strict=True) == TOOLCHAIN_LIB, 'toolchain library alias')
    for name, count in SHARED_LIBRARIES.items():
        path = TOOLCHAIN_LIB / name
        require(path.resolve(strict=True) == path, 'compiler shared-library alias')
        tool_pins[name] = pin(path)
        require(tool_pins[name]['bytes'] == count, 'compiler shared-library extent')
    require(tool_pins == inputs['tool_pins'], 'root-pinned tools differ')
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    env = dict(HOME='/home/harmenon', PATH=str(TOOLCHAIN) + ':/usr/bin:/bin',
               CARGO_HOME='/home/harmenon/.cargo', CARGO_TARGET_DIR=str(TARGET),
               LD_LIBRARY_PATH=str(TARGET / 'debug/deps') + ':' + str(TOOLCHAIN_LIB), TMPDIR=str(TMP),
               RUSTC=str(TOOLCHAIN / 'rustc'), RUSTDOC=str(TOOLCHAIN / 'rustdoc'),
               CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', RUST_BACKTRACE='1',
               CARGO_NET_OFFLINE='true', CARGO_CACHE_AUTO_CLEAN_FREQUENCY='never',
               MALLOC_ARENA_MAX='2',
               CARGO_PROFILE_DEV_OPT_LEVEL='0', CARGO_PROFILE_DEV_DEBUG='0',
               CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
               CARGO_PROFILE_TEST_OPT_LEVEL='0', CARGO_PROFILE_TEST_DEBUG='0',
               CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
               ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
    # No ambient or synthetic crate binding; the extraction fixtures establish their own contracts.
    source_pin = save('sources-before.json', before)
    phases, artifacts, external, local, inventories, ignored, tests = [], {}, {}, {}, {}, {}, {}
    compiler_products_before_test_build = {}
    failure, postchecks, final = None, [], None
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, hard_deadline-CLEANUP_RESERVE-time.monotonic()))
    def leaf(label, argv, seconds=LEAF_WALL, package=None):
        cwd = SOURCE / 'crates' / package if package else ROOT
        selected_env = dict(env)
        if package:
            selected_env['CARGO_MANIFEST_DIR'] = str(cwd)
        if label == 'matrix-extraction-0':
            selected_env['FE2O3_DIAG_CORE_RESULT_MIR_V1'] = '1'
        run(label, argv, selected_env, phases, hard_deadline, before, seconds, cwd)
    cargo = str(TOOLCHAIN / 'cargo')
    common = ['--offline', '--locked', '--manifest-path', str(SOURCE / 'Cargo.toml')]
    try:
        leaf('rustc-version', [str(TOOLCHAIN / 'rustc'), '--version', '--verbose'], 60)
        leaf('metadata', [cargo, 'metadata', *common, '--format-version', '1'], 120)
        external, local = metadata_contract()
        save('dependencies-before.json', external)
        leaf('device-build', [cargo, 'test', *common, '--jobs', '2', '-p', 'fe2o3-device',
                             '--lib', '--test', 'device_api_ui', '--test', 'device_ffi_ui',
                             '--no-run', '--message-format=json'])
        records = build_records(OUT / 'device-build.stdout')
        for role in ('device-lib', 'device-api-ui', 'device-ffi-ui'):
            artifacts[role] = select_artifact(records, *ROLES[role], True)
        leaf('compiler-products', [cargo, 'build', *common, '--jobs', '2', '-p', 'rustc-codegen-fe2o3',
                                   '--lib', '--bin', 'fe2o3-rustc-extract', '--message-format=json'])
        records = build_records(OUT / 'compiler-products.stdout')
        compiler_products_before_test_build['backend'] = select_artifact(records, 'rustc-codegen-fe2o3', 'rustc_codegen_fe2o3',
                                                'dylib', False, '.so')
        compiler_products_before_test_build['extractor'] = select_artifact(records, 'rustc-codegen-fe2o3', 'fe2o3-rustc-extract',
                                                  'bin', False)
        leaf('compiler-tests-build', [cargo, 'test', *common, '--jobs', '2', '-p', 'rustc-codegen-fe2o3',
                                      '--lib', '--test', 'production_extraction_driver_v1',
                                      '--test', 'production_general_matrix_driver_v1',
                                      '--no-run', '--message-format=json'])
        records = build_records(OUT / 'compiler-tests-build.stdout')
        # Cargo test feature unification can rebuild these same production paths.
        artifacts['backend'] = select_artifact(records, 'rustc-codegen-fe2o3', 'rustc_codegen_fe2o3',
                                                'dylib', False, '.so')
        artifacts['extractor'] = select_artifact(records, 'rustc-codegen-fe2o3', 'fe2o3-rustc-extract',
                                                  'bin', False)
        for role in ('backend', 'extractor'):
            require(artifacts[role]['pin']['path'] == compiler_products_before_test_build[role]['pin']['path'],
                    'final Cargo product path differs from its prior build observation')
        for role in ('compiler-lib', 'atomic-extraction', 'matrix-extraction'):
            artifacts[role] = select_artifact(records, *ROLES[role], True)
        for role in ROLES:
            executable = artifacts[role]['pin']['path']
            leaf(role + '-list', [executable, '--list', '--format=terse'], 120, ROLES[role][0])
            leaf(role + '-ignored', [executable, '--ignored', '--list', '--format=terse'], 120, ROLES[role][0])
            inventories[role] = inventory(OUT / (role + '-list.stdout'))
            ignored[role] = inventory(OUT / (role + '-ignored.stdout'))
            require(inventories[role] and set(ignored[role]) <= set(inventories[role]), 'compiled inventory closure')
        for role, expected in (('device-api-ui', ['device_api_enforces_witness_boundaries']),
                               ('device-ffi-ui', ['device_ffi_contracts_enforce_the_review_boundary'])):
            require(inventories[role] == expected, 'exact existing UI target test')
        selected = [name for name in inventories['compiler-lib'] if name.startswith(DIAGNOSTIC_PREFIX)]
        require(selected == sorted(DIAGNOSTIC_TESTS)
                and not set(selected) & set(ignored['compiler-lib']),
                'exact six diagnostic rendering tests required, with no ignored cases')
        leaf('core-result-diagnostic-tests',
             [artifacts['compiler-lib']['pin']['path'], DIAGNOSTIC_PREFIX, '--test-threads=1'],
             package='rustc-codegen-fe2o3')
        tests['core-result-diagnostic-tests'] = outcomes(
            OUT / 'core-result-diagnostic-tests.stdout', selected, len(inventories['compiler-lib']))
        role, label = 'matrix-extraction', 'matrix-extraction-0'
        require(MATRIX in ignored[role], 'exact historical ignored matrix control absent')
        leaf(label, [artifacts[role]['pin']['path'], MATRIX, '--ignored', '--exact', '--test-threads=1'],
             package='rustc-codegen-fe2o3')
        tests[label] = outcomes(OUT / (label + '.stdout'), [MATRIX], len(inventories[role]))
        require(len(phases) == 19 and len(tests) == 2, 'closed19-phase/two-test-scope diagnostic completion')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    def check(label, action):
        try:
            remaining = hard_deadline - 5 - time.monotonic()
            require(remaining > 0, 'postcheck whole deadline')
            signal.setitimer(signal.ITIMER_REAL, remaining)
            action()
        except BaseException as error:
            postchecks.append(label + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    def source_check():
        nonlocal final
        final = sources()
        save('sources-after.json', final)
        require(final == before, 'workspace sources/lock changed')
    check('sources', source_check)
    check('input manifest', lambda: require(pin(ROOT/'input-manifest.json') == input_pin, 'manifest drift'))
    check('tools', lambda: require({n:pin(Path(p['path'])) for n,p in tool_pins.items()} == tool_pins, 'tool drift'))
    check('configuration', lambda: require(configurations() == config_before, 'Cargo configuration drift'))
    external_after = {}
    for directory, expected in external.items():
        def dependency_check(directory=directory, expected=expected):
            path = Path(directory)
            actual = {str(p.relative_to(path)): pin(p) for p in files_below(path, packed=False)}
            external_after[directory] = actual
            require(actual == expected, 'dependency source changed')
        check('dependency:' + directory, dependency_check)
    if external:
        save('dependencies-after.json', external_after)
    for role, row in artifacts.items():
        check(role, lambda row=row: require(pin(Path(row['pin']['path'])) == row['pin'], 'artifact drift'))
    failure = failure or ('postcheck failed' if postchecks else None)
    if time.monotonic() >= hard_deadline:
        failure = failure or 'whole deadline exceeded'
    result = dict(schema='ferric-guarded-mlp-core-result-diagnostic-cpu-v1', passed=failure is None,
                  diagnostic_only=True, diagnostic_admission_authority=False,
                  control_test=MATRIX, qualification_passed=False,
                  failure=failure, postcheck_errors=postchecks, phases=phases, tests=tests,
                  tests_passed=sum(t['passed'] for t in tests.values()), tests_ignored=0,
                  test_inventories=inventories, ignored_inventories=ignored, artifacts=artifacts,
                  compiler_products_before_test_build=compiler_products_before_test_build,
                  historical_product_pins_postchecked=False,
                  final_compiler_product_phase='compiler-tests-build'
                      if {'backend', 'extractor'} <= set(artifacts) else None,
                  base_source_generation=inputs['base_source_generation'],
                  diagnostic_source_files=inputs['diagnostic_source_files'], input_manifest=input_pin,
                  input_sources=source_pin, final_sources=pin(OUT/'sources-after.json')
                      if (OUT/'sources-after.json').exists() else None,
                  source_unchanged=final==before, tool_pins=tool_pins, local_dependencies=local,
                  configurations=config_before, controller=pin(ROOT/'run_cpu.py'),
                  raw={p.name:pin(p) for p in sorted(OUT.iterdir()) if p.is_file()},
                  elapsed_seconds=time.monotonic()-started,
                  limits=dict(whole_seconds=WHOLE_WALL, leaf_seconds=LEAF_WALL,
                              cleanup_reserve_seconds=CLEANUP_RESERVE, address_space_bytes=AS_LIMIT,
                              cpu_seconds_per_leaf=CPU_LIMIT, file_bytes=FILE_LIMIT, cache_bytes=CACHE_LIMIT,
                              stream_bytes=STREAM_LIMIT, initial_free_bytes=START_FREE,
                              live_free_bytes=LIVE_FREE, affinity=[8,9], nice=10, cargo_jobs=2),
                  offline=True, cache_auto_clean='never', gpu_execution=False,
                  guarded_candidate_hsaco_emitted=False, full_model_acceptance=False,
                  numerical_acceptance=False, performance_claim=False)
    save('complete.json' if failure is None else 'failed.json', result)
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
