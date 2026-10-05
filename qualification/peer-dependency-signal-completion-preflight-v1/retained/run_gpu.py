"""One caller-pinned native signal-completion witness; no retry or performance claim."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import subprocess
import sys
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'peer-dependency-signal-completion-gpu-v228-v1'
CPU_ROOT = E / 'peer-dependency-signal-completion-cpu-v228-v1'
OUT = ROOT / 'evidence'
BOOT = '2dbbbc5a-d8b7-46ac-bd1f-236e5847e24a'
HOST = 'smci350-rck-g03-b19-03'
IMAGE = Path('/home/harmenon/ferric-asrock-42/evidence/p223-runtime-v1/residual.hsaco')
IMAGE_SHA = '8436c1861dcc14f0346aeecc136e4187c459ac1c0b31778265c88b3d8c63396d'
CPU_RUNNER_SHA = 'd66b641752986b09952dc73b28ee7984796359e39d1373046dfa2229ffa68406'
IDS = [16366993098680759275, 10838076764495710945]
PHASES = ['rustc-version', 'format', 'format-check', 'lock', 'aql-tests',
          'kfd-tests', 'probe-tests', 'terminal-probe-tests', 'signal-probe-tests',
          'probe-build', 'terminal-probe-build', 'signal-probe-build', 'default-check']
SMI = '/opt/rocm/bin/amd-smi'
EXAMPLE = 'kfd-peer-dependency-signal-completion'
TERMINAL_EXAMPLE = 'kfd-peer-dependency-terminal-join'
COMPATIBILITY_EXAMPLE = 'kfd-peer-dependency-sentinel'
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
LOG_LIMIT = 4 << 20
READ_LIMIT = 512 << 20
AS_LIMIT = 64 << 30
WHOLE_WALL = 900
SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)
INPUTS = {}
ALIASES = {}
LAUNCHES = []
HARD_DEADLINE = float('inf')


def require(value, message):
    if not value:
        raise RuntimeError(message)


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_uid,
            value.st_gid, value.st_nlink, value.st_size,
            value.st_mtime_ns, value.st_ctime_ns)


def read(path, expected=None, retain=True, limit=READ_LIMIT, track=True):
    path = Path(path)
    require(path.is_absolute() and path.resolve() == path, 'noncanonical input: ' + str(path))
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= limit,
            'not a bounded regular file: ' + str(path))
    digest, parts = hashlib.sha256(), []
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    with os.fdopen(fd, 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed at open')
        while True:
            if track or expected is not None:
                require(time.monotonic() < HARD_DEADLINE, 'whole-run deadline during input read')
            chunk = stream.read(1 << 20)
            if not chunk:
                break
            digest.update(chunk)
            if retain:
                parts.append(chunk)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed during read')
    require(stamp(path.lstat()) == stamp(before), 'input changed after read')
    pin = dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())
    if expected is not None:
        require(pin == expected, 'FilePin mismatch: ' + str(path))
    if track:
        require(str(path) not in INPUTS or INPUTS[str(path)] == pin, 'conflicting input pin')
        INPUTS[str(path)] = pin
    return (b''.join(parts) if retain else None), pin


def no_duplicates(pairs):
    value = {}
    for key, item in pairs:
        require(key not in value, 'duplicate JSON key')
        value[key] = item
    return value


def parse(raw):
    return json.loads(raw, object_pairs_hook=no_duplicates,
                      parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))


def document(pin):
    return parse(read(pin['path'], pin)[0])


def resolved_input(path):
    path = Path(path)
    require(path.is_absolute(), 'tool/library path must be absolute')
    canonical = path.resolve(strict=True)
    require(str(path) not in ALIASES or ALIASES[str(path)] == str(canonical), 'tool/library alias drift')
    ALIASES[str(path)] = str(canonical)
    return read(canonical, retain=False)[1]


def save(name, value):
    path = OUT / name
    raw = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('utf-8')
    require(len(raw) <= LOG_LIMIT, 'metadata exceeds four MiB')
    with path.open('xb') as stream:
        stream.write(raw)
    return read(path, retain=False, track=False)[1]


def natural(row):
    require(row['exit_code'] == 0 and row['natural_exit'] is True
            and row['reaped'] is True and row['process_group_absent'] is True
            and row['forced_cleanup'] is False and row['timed_out'] is False
            and row['exception'] is None, 'non-natural or unsuccessful owned leaf')


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


def test_counts(raw):
    text = normalized_libtest(raw.decode('utf-8'))
    summaries = [dict(status=s, passed=int(p), failed=int(f), ignored=int(i),
                      measured=int(m), filtered_out=int(x)) for s, p, f, i, m, x in re.findall(
        r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; '
        r'(\d+) measured; (\d+) filtered out;', text, re.M)]
    named = [dict(name=n, outcome=o) for n, o in re.findall(
        r'^test (.+?) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', text, re.M)]
    require(summaries and all(r['status'] == 'ok' and r['failed'] == 0 for r in summaries),
            'failed or absent actual libtest summaries')
    totals = {k: sum(r[k] for r in summaries) for k in ('passed', 'failed', 'ignored')}
    for outcome, field in [('ok', 'passed'), ('FAILED', 'failed'), ('ignored', 'ignored')]:
        require(sum(r['outcome'] == outcome for r in named) == totals[field], 'libtest census mismatch')
    require(totals['passed'] > 0, 'no passing tests')
    return dict(summaries=summaries, named=named, **totals)


def admit_cpu(path, sha):
    require(CPU_RUNNER_SHA is not None and path == CPU_ROOT / 'evidence/complete.json'
            and re.fullmatch('[0-9a-f]{64}', sha),
            'exact CPU completion and caller SHA required')
    raw, cpu_pin = read(path)
    require(cpu_pin['sha256'] == sha, 'CPU completion SHA mismatch')
    cpu = parse(raw)
    require(cpu['schema'] == 'ferric-peer-dependency-signal-completion-cpu-v1'
            and cpu['passed'] is True and cpu['failure'] is None
            and cpu['postcheck_errors'] == [] and cpu['source_unchanged'] is True
            and cpu['host'] == HOST and cpu['boot'] == BOOT, 'CPU completion contract')
    require(cpu['controller']['path'] == str(CPU_ROOT / 'run_cpu.py')
            and cpu['controller']['sha256'] == CPU_RUNNER_SHA, 'CPU controller generation')
    require([r['label'] for r in cpu['phases']] == PHASES, 'CPU thirteen-phase roster')
    expected_raw = {name + '.' + suffix for name in PHASES
                    for suffix in ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')}
    expected_raw |= {'sources-' + name + '.json' for name in ('input', 'formatted', 'tested', 'after')}
    require(set(cpu['raw']) == expected_raw, 'CPU raw roster')
    for name, pin in cpu['raw'].items():
        require(pin['path'] == str(CPU_ROOT / 'evidence' / name), 'CPU raw path')
        read(pin['path'], pin, retain=False)
    for row in cpu['phases']:
        natural(row)
        label = row['label']
        require(document(cpu['raw'][label + '.result.json']) == row, 'CPU phase result join')
        require(row['command'] == cpu['raw'][label + '.command.json']
                and row['stdout'] == cpu['raw'][label + '.stdout']
                and row['stderr'] == cpu['raw'][label + '.stderr'], 'CPU phase raw joins')
        command = document(row['command'])
        started = document(cpu['raw'][label + '.started.json'])
        require(command['argv'] == row['argv'] == started['argv']
                and command['cwd'] == str(CPU_ROOT)
                and row['pid'] == row['pgid'] == started['pid'] == started['pgid'], 'CPU owned command join')
    require(set(cpu['tests']) == {'aql-tests', 'kfd-tests', 'probe-tests', 'terminal-probe-tests',
                                  'signal-probe-tests'}, 'CPU test scopes')
    for name, recorded in cpu['tests'].items():
        require(test_counts(read(cpu['raw'][name + '.stdout']['path'])[0]) == recorded, 'CPU test log replay')
    tested, after = document(cpu['tested_sources']), document(cpu['final_sources'])
    require(cpu['tested_sources'] == cpu['raw']['sources-tested.json']
            and cpu['final_sources'] == cpu['raw']['sources-after.json']
            and tested == after and FORMATTED <= set(tested), 'CPU source postcheck snapshots')
    source_input, formatted = document(cpu['input_sources']), document(cpu['formatted_sources'])
    require(cpu['input_sources'] == cpu['raw']['sources-input.json']
            and cpu['formatted_sources'] == cpu['raw']['sources-formatted.json']
            and set(source_input) == set(formatted) == set(tested)
            and {name for name in source_input if source_input[name] != formatted[name]} <= FORMATTED
            and all(tested[name] == formatted[name] for name in tested if name != 'Cargo.lock'),
            'CPU ten-file formatter and lock transition')
    for relative, pin in tested.items():
        require(pin['path'] == str(CPU_ROOT / relative)
                and '..' not in Path(relative).parts and not Path(relative).is_absolute(), 'CPU source path')
        read(pin['path'], pin, retain=False)
    for pin in [cpu['controller'], cpu['cargo_lock'], *cpu['tool_pins'].values()]:
        read(pin['path'], pin, retain=False)
    require(cpu['cargo_lock'] == tested['Cargo.lock']
            and cpu['controller'] == tested['run_cpu.py'], 'CPU selected input pins')
    selected = {}
    for key, label, example in (('compatibility_binary', 'probe-build', COMPATIBILITY_EXAMPLE),
                                ('terminal_binary', 'terminal-probe-build', TERMINAL_EXAMPLE),
                                ('binary', 'signal-probe-build', EXAMPLE)):
        binary = cpu[key]['pin']
        records = [parse(line) for line in read(cpu['raw'][label + '.stdout']['path'])[0].splitlines() if line]
        finished = [r for r in records if r.get('reason') == 'build-finished']
        require(len(finished) == 1 and finished[0].get('success') is True, 'Cargo build completion')
        artifacts = [r for r in records if r.get('reason') == 'compiler-artifact'
                     and r.get('target', {}).get('name') == example and r['target'].get('kind') == ['example']
                     and r.get('profile', {}).get('test') is False and r.get('executable')]
        require(artifacts == [cpu[key]['cargo_artifact']]
                and artifacts[0]['executable'] == binary['path'], 'actual Cargo example selection')
        executable = Path(binary['path'])
        require(executable.is_relative_to(CPU_ROOT / 'target') and executable.stat().st_mode & 0o111,
                'CPU executable path or mode')
        raw, _ = read(executable, binary)
        require(raw[:6] == b'\x7fELF\x02\x01' and raw[18:20] == b'\x3e\x00', 'x86_64 little-endian ELF')
        selected[key] = binary
    require(len({pin['path'] for pin in selected.values()}) == 3,
            'signal, terminal and compatibility executables alias')
    return cpu_pin, selected['binary'], selected['terminal_binary'], selected['compatibility_binary']


def group_exists(pgid):
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False


def reap_adopted(pgid, leader):
    leader.poll()
    if leader.returncode is None:
        return []
    rows = []
    while True:
        try:
            pid, status = os.waitpid(-pgid, os.WNOHANG)
        except ChildProcessError:
            return rows
        if not pid:
            return rows
        rows.append(dict(pid=pid, wait_status=status))


def owned(label, argv, wall, cpu_seconds, leaves):
    require(time.monotonic() + wall + 40 < HARD_DEADLINE, 'insufficient owned-leaf cleanup budget')
    env = dict(HOME='/home/harmenon', PATH='/opt/rocm/bin:/usr/bin:/bin', LC_ALL='C', LANG='C',
               PYTHONDONTWRITEBYTECODE='1', PYTHONNOUSERSITE='1', OMP_NUM_THREADS='1')
    command = ['/usr/bin/prlimit', '--as=' + str(AS_LIMIT), '--cpu=' + str(cpu_seconds),
               '--fsize=' + str(LOG_LIMIT), '--core=0', '--', *argv]
    command_pin = save(label + '.command.json', dict(argv=command, cwd=str(ROOT), env=env, wall_seconds=wall))
    stdout, stderr = OUT / (label + '.stdout'), OUT / (label + '.stderr')
    start, child, exception = time.monotonic_ns(), None, None
    timed_out = forced = False
    reaped = []
    with stdout.open('xb') as so, stderr.open('xb') as se:
        try:
            child = subprocess.Popen(command, cwd=ROOT, env=env, stdin=subprocess.DEVNULL,
                                     stdout=so, stderr=se, start_new_session=True)
            LAUNCHES.append(dict(label=label, pid=child.pid, pgid=child.pid))
            require(os.getpgid(child.pid) == child.pid, 'owned group identity')
            save(label + '.started.json', dict(pid=child.pid, pgid=child.pid, argv=command))
            child.wait(timeout=wall)
        except subprocess.TimeoutExpired:
            timed_out = True
        except BaseException as error:
            exception = repr(error)
        finally:
            old_handlers = {sig: signal.getsignal(sig) for sig in SIGNALS}
            deferred = []
            signal.setitimer(signal.ITIMER_REAL, 0)
            for sig in SIGNALS:
                signal.signal(sig, lambda number, _frame: deferred.append(number))
            if child is not None:
                child.poll()
                reaped.extend(reap_adopted(child.pid, child))
                if child.returncode is None or group_exists(child.pid):
                    forced = True
                    for sig, allowance in ((signal.SIGTERM, 10), (signal.SIGKILL, 20)):
                        try:
                            os.killpg(child.pid, sig)
                        except ProcessLookupError:
                            pass
                        deadline = min(HARD_DEADLINE - 5, time.monotonic() + allowance)
                        while time.monotonic() < deadline:
                            child.poll()
                            reaped.extend(reap_adopted(child.pid, child))
                            if child.returncode is not None and not group_exists(child.pid):
                                break
                            time.sleep(0.02)
                        if child.returncode is not None and not group_exists(child.pid):
                            break
                try:
                    child.wait(timeout=max(0.001, min(5, HARD_DEADLINE - time.monotonic())))
                except subprocess.TimeoutExpired:
                    exception = (exception or '') + ' leader not reaped'
                reaped.extend(reap_adopted(child.pid, child))
            for sig, handler in old_handlers.items():
                signal.signal(sig, handler)
            if deferred:
                exception = (exception or '') + ' signals during cleanup: ' + repr(deferred)
    row = dict(label=label, command=command_pin, pid=child.pid if child else None,
               pgid=child.pid if child else None, exit_code=child.returncode if child else None,
               timed_out=timed_out, forced_cleanup=forced, exception=exception,
               natural_exit=child is not None and child.returncode is not None
               and not timed_out and not forced and exception is None,
               reaped=child is not None and child.returncode is not None,
               process_group_absent=child is not None and not group_exists(child.pid),
               adopted_reaped=reaped, elapsed_ns=time.monotonic_ns() - start,
               stdout=read(stdout, retain=False, track=False, limit=LOG_LIMIT)[1],
               stderr=read(stderr, retain=False, track=False, limit=LOG_LIMIT)[1])
    leaves.append(row)
    save(label + '.result.json', row)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, HARD_DEADLINE - time.monotonic() - 45))
    natural(row)
    return read(stdout, track=False, limit=LOG_LIMIT)[0]


def idle(raw):
    rows = parse(raw)
    require(isinstance(rows, list) and len(rows) == 8, 'expected exactly eight GPU audit rows')
    require(all(set(row) == {'gpu', 'process_list'} and type(row['gpu']) is int for row in rows)
            and {row['gpu'] for row in rows} == set(range(8))
            and all(row['process_list'] == [{'process_info': 'No running processes detected'}]
                    for row in rows), 'GPU process roster is not exact-empty')
    return rows


def runtime_libraries(dynamic_raw, ldd_raw):
    dynamic, output = dynamic_raw.decode('utf-8'), ldd_raw.decode('utf-8')
    needed = re.findall(r'\(NEEDED\).*Shared library: \[([^\]\n]+)\]', dynamic)
    interpreters = re.findall(r'\[Requesting program interpreter: ([^\]\n]+)\]', dynamic)
    require(needed and len(set(needed)) == len(needed) and len(interpreters) == 1,
            'readelf missing/duplicate dynamic dependencies or interpreter')
    require(output.strip() and 'not found' not in output and 'not a dynamic executable' not in output,
            'ldd reports empty or unresolved runtime')
    libraries, direct, vdso = {}, [], []
    for line in output.splitlines():
        if not line.strip():
            continue
        named = re.fullmatch(r'\s*(\S+) => (/\S+) \(0x[0-9a-fA-F]+\)\s*', line)
        absolute = re.fullmatch(r'\s*(/\S+) \(0x[0-9a-fA-F]+\)\s*', line)
        virtual = re.fullmatch(r'\s*linux-vdso\.so\.1 \(0x[0-9a-fA-F]+\)\s*', line)
        if named:
            name, path = named.groups()
            require(name not in libraries, 'duplicate ldd SONAME')
            libraries[name] = dict(reported_path=path, pin=resolved_input(path))
        elif absolute:
            direct.append(absolute.group(1))
        elif virtual:
            vdso.append('linux-vdso.so.1')
        else:
            raise RuntimeError('unrecognized ldd line: ' + line)
    interpreter = interpreters[0]
    require(len(direct) == 1 and len(vdso) == 1,
            'ldd must resolve exactly one interpreter/vDSO')
    interp_pin = resolved_input(interpreter)
    direct_pin = resolved_input(direct[0])
    require(interp_pin == direct_pin, 'ldd/readelf interpreter mismatch')
    loader_name = Path(interpreter).name
    if loader_name in needed:
        require(Path(direct[0]).name == loader_name, 'needed interpreter basename mismatch')
        if loader_name in libraries:
            require(libraries[loader_name]['pin'] == direct_pin, 'named interpreter pin mismatch')
        else:
            libraries[loader_name] = dict(reported_path=direct[0], pin=direct_pin)
    require(set(needed) <= set(libraries), 'ldd must resolve every DT_NEEDED')
    return dict(needed=needed, libraries=libraries, interpreter=dict(reported_path=interpreter, pin=interp_pin),
                virtual_objects=vdso, all_needed_resolved=True)


def pattern(rank, iteration):
    data = bytearray(b'\xa5' * (8192 + 128))
    for index in range(4096):
        word = 0x3c00 | ((index * 13 + rank * 257 + iteration * 521) % 1024)
        data[64 + index * 2:66 + index * 2] = word.to_bytes(2, 'little')
    return hashlib.sha256(data).hexdigest()


def observation(raw, request_pin):
    value = parse(raw)
    expected = dict(schema='ferric-peer-dependency-native-observation-v228-v3',
                    request_sha256=request_pin['sha256'], device_unique_ids=IDS,
                    artifact_path=str(IMAGE), artifact_bytes=14624, artifact_sha256=IMAGE_SHA,
                    kernel_symbol='ferric_qwen3_tp_peer_copy_bf16_v4', timeout_ms=60000,
                    witness_requested=True, witness_observed=True, completion_count=16,
                    completion_contract='all signals complete; actual read retained for ring capacity',
                    elapsed_scope='native aggregate host interval; not GPU duration or throughput',
                    device_iterations=2, kernel_packets=8, barrier_packets=8,
                    intermediate_reused_between_iterations=True,
                    completion_slots_reset_between_iterations=False, repeated_host_calls_tested=False,
                    all_data_and_guards_verified=True, input_files_unchanged=True, healthy_close=True,
                    performance_claim=False, production_authority=False)
    buffers = []
    for rank in range(2):
        for role, owner, iteration in [('seed0', rank, 0), ('seed1', rank, 1),
                                       ('intermediate', rank, 1), ('output0', 1-rank, 0),
                                       ('output1', 1-rank, 1)]:
            buffers.append(dict(owner_rank=rank, role=role, bytes=8320, sha256=pattern(owner, iteration)))
    expected['buffers'] = buffers
    require(set(value) == set(expected) | {'elapsed_ns', 'observed_queue_frontiers'},
            'native observation closed fields')
    frontiers = value['observed_queue_frontiers']
    require(isinstance(frontiers, list) and len(frontiers) == 2
            and all(isinstance(row, list) and len(row) == 2
                    and all(type(counter) is int for counter in row)
                    and row[0] == 8 and 0 <= row[1] <= 8 for row in frontiers),
            'native observed queue frontiers contract')
    require(json.dumps({key: value[key] for key in expected}, sort_keys=True)
            == json.dumps(expected, sort_keys=True),
            'native observation or guarded-buffer hash mismatch')
    require(type(value['elapsed_ns']) is int and 0 < value['elapsed_ns'] <= 60_000_000_000,
            'native inclusive elapsed bound')
    return value


def interrupted(number, _frame):
    raise RuntimeError('interrupted by signal ' + str(number))


def main():
    global HARD_DEADLINE
    started = time.monotonic()
    HARD_DEADLINE = started + WHOLE_WALL
    require(__debug__ and len(sys.argv) == 3, 'usage: run_gpu.py CPU_COMPLETE_PATH EXPECTED_SHA256')
    require(Path(__file__).resolve() == ROOT / 'run_gpu.py' and not OUT.exists(), 'fresh exact output namespace')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == HOST
            and Path('/proc/sys/kernel/random/boot_id').read_text().strip() == BOOT, 'selected host/boot/UID')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected initial priority')
    if priority == 0:
        os.nice(10)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper failed')
    OUT.mkdir(mode=0o700)
    for sig in SIGNALS:
        signal.signal(sig, interrupted)
    signal.setitimer(signal.ITIMER_REAL, WHOLE_WALL - 45)
    leaves, errors = [], []
    cpu_pin = binary = terminal_binary = compatibility_binary = None
    native = before = after = request_pin = runtime = None
    try:
        read(Path(__file__), retain=False)
        for executable in (SMI, '/usr/bin/prlimit', '/usr/bin/readelf', '/usr/bin/ldd', sys.executable):
            resolved_input(executable)
        cpu_pin, binary, terminal_binary, compatibility_binary = admit_cpu(Path(sys.argv[1]), sys.argv[2])
        raw, request_pin = read(ROOT / 'request.json', limit=4096)
        request = parse(raw)
        require(request == dict(schema='ferric-peer-dependency-native-request-v228-v3',
                                devices=[{'unique_id': value} for value in IDS],
                                artifact=dict(path=str(IMAGE), sha256=IMAGE_SHA),
                                timeout_ms=60000, witness=True)
                and type(request['timeout_ms']) is int and request['witness'] is True
                and all(type(row['unique_id']) is int for row in request['devices']),
                'closed actual native request')
        read(IMAGE, dict(path=str(IMAGE), bytes=14624, sha256=IMAGE_SHA), retain=False)
        dynamic = owned('readelf', ['/usr/bin/readelf', '-d', '-l', binary['path']], 30, 30, leaves)
        ldd = owned('ldd', ['/usr/bin/ldd', binary['path']], 30, 30, leaves)
        runtime = runtime_libraries(dynamic, ldd)
        save('runtime-libraries.json', runtime)
        save('inputs-before.json', INPUTS)
        before = idle(owned('before', [SMI, 'process', '--json'], 60, 60, leaves))
        require(time.monotonic() + 600 + 35 + 60 + 35 + 30 < HARD_DEADLINE,
                'native requires both cleanup budgets, post-audit and final input postchecks')
        native = observation(owned('native', [binary['path'], '--allow-unauthenticated-machine-code',
                                            '--request', str(ROOT / 'request.json')], 600, 300, leaves), request_pin)
        save('observation.json', native)
    except BaseException as error:
        errors.append(repr(error))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        try:
            after = idle(owned('after', [SMI, 'process', '--json'], 60, 60, leaves))
        except BaseException as error:
            errors.append('post-audit: ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    postcheck_errors = []
    for path, canonical in ALIASES.items():
        try:
            require(str(Path(path).resolve(strict=True)) == canonical, 'tool/library alias drift: ' + path)
        except BaseException as error:
            postcheck_errors.append(repr(error))
    for pin in list(INPUTS.values()):
        try:
            read(pin['path'], pin, retain=False, track=False)
        except BaseException as error:
            postcheck_errors.append(repr(error))
    if os.uname().nodename != HOST or Path('/proc/sys/kernel/random/boot_id').read_text().strip() != BOOT:
        postcheck_errors.append('host/boot drift')
    if time.monotonic() >= HARD_DEADLINE:
        postcheck_errors.append('whole-run wall bound exceeded')
    passed = not errors and not postcheck_errors and native is not None and before is not None and after is not None
    value = dict(schema='ferric-peer-dependency-signal-completion-gpu-result-v1', passed=passed,
                 errors=errors, postcheck_errors=postcheck_errors, cpu_complete=cpu_pin,
                 binary=binary, terminal_binary=terminal_binary, compatibility_binary=compatibility_binary,
                 request=request_pin, inputs=INPUTS, leaves=leaves, launches=LAUNCHES,
                 input_aliases=ALIASES, runtime_libraries=runtime,
                 observation=native, idle_before=before, idle_after=after,
                 host=HOST, boot=BOOT, elapsed_seconds=time.monotonic() - started,
                 limits=dict(whole_wall_seconds=WHOLE_WALL, native_wall_seconds=600,
                             native_cpu_seconds=300, address_space_bytes=AS_LIMIT,
                             post_native_reserve_seconds=160,
                             log_bytes=LOG_LIMIT, affinity=[8, 9], nice=10),
                 gpu_attempts=sum(row['label'] == 'native' for row in LAUNCHES), retries=0,
                 native_peer_dependency_observed=passed, repeated_host_calls_tested=False,
                 model_execution=False, performance_claim=False, production_authority=False,
                 executable_library_audit_replayed=runtime is not None,
                 raw={path.name: read(path, retain=False, track=False)[1]
                      for path in sorted(OUT.iterdir()) if path.is_file()})
    result = save('complete.json' if passed else 'failed.json', value)
    print(json.dumps(result), flush=True)
    if not passed:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
