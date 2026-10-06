"""One bounded serial component test; never admit a paired or production runtime."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import sys
import time
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-stable-r2-native-gpu-v228-v1'
CPU = E / 'guarded-mlp-stable-r2-native-cpu-v228-v1'
OUT = ROOT / 'evidence'
CPU_SHA = '5d84e176512f0b9b59c85c2361ab95b4286c6142ced5cf4cd7995d5a8a6749b3'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
LIBRARY_AUDIT_SHA = 'b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'
IMAGE = E / 'guarded-mlp-handoff-capture-v228-v1/fe2o3-engineering-v1/d98a12d35a2f0ecea506a9384213c22dbf692c03347efb91a009fc24e0982e1b/observation.hsaco'
IMAGE_SHA = 'de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66'
TEST = 'engineering_gfx950::peer::combined_mlp_state_v1::tests::native::native_stable_guarded_r2_v1'
IDS = [16366993098680759275, 10838076764495710945]
TOOL_LIB = '/home/harmenon/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/lib'
WHOLE = 900


def require(value, message):
    if not value:
        raise RuntimeError(message)


def load_exact(name, path, digest):
    require(path.resolve(strict=True) == path and not path.is_symlink()
            and path.is_file() and path.stat().st_size < 128 << 10, 'bounded helper')
    before = path.stat()
    body = path.read_bytes()
    after = path.stat()
    require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns)
            == (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)
            and hashlib.sha256(body).hexdigest() == digest, 'exact helper pin')
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(body, str(path), 'exec'), module.__dict__)
    return module


def topology():
    rows = []
    for node, uid in zip((2, 3), IDS):
        path = Path(f'/sys/class/kfd/kfd/topology/nodes/{node}/properties')
        body = path.read_text()
        require(len(body) < 16384, 'bounded topology')
        values = dict(line.split() for line in body.splitlines())
        require(int(values['unique_id']) == uid and int(values['gfx_target_version']) == 90500,
                'selected gfx950 topology identity')
        rows.append(dict(node=node, unique_id=uid, properties=body))
    return rows


def main():
    started, deadline = time.monotonic(), time.monotonic() + WHOLE
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2, 'run_gpu.py INPUT_SHA')
    require(Path(__file__).resolve() == ROOT / 'run_gpu.py' and not os.path.lexists(OUT), 'fresh exact output')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'host/UID')
    require(shutil.disk_usage(ROOT).free >= 40 << 30, '40 GiB setup floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'nice')
    if priority == 0:
        os.nice(10)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper')
    h = load_exact('stable_r2_supervisor', ROOT / 'supervisor.py', SUPERVISOR_SHA)
    audit = load_exact('stable_r2_library_audit', ROOT / 'library_audit.py', LIBRARY_AUDIT_SHA)
    h.ROOT, h.OUT, h.TARGET, h.TMP = ROOT, OUT, ROOT / 'target', ROOT / 'tmp'
    h.AS_LIMIT, h.CPU_LIMIT, h.FILE_LIMIT, h.STREAM_LIMIT = 64 << 30, 300, 4 << 20, 4 << 20
    h.CACHE_LIMIT = 64 << 20
    audit.HARD_DEADLINE = deadline
    OUT.mkdir(mode=0o700)
    h.TMP.mkdir(mode=0o700)
    for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, deadline - time.monotonic() - 50)
    phases, errors, post_errors = [], [], []
    binary = cpu_pin = request_pin = runtime = before = after = topology_before = None
    verification = selftest = observation = None
    native_attempt_requested = False
    boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    env = dict(HOME='/home/harmenon', PATH='/opt/rocm/bin:/usr/bin:/bin', LC_ALL='C', LANG='C',
               LD_LIBRARY_PATH=str(CPU / 'target/debug/deps') + ':' + TOOL_LIB,
               TMPDIR=str(h.TMP), RUST_BACKTRACE='1', OMP_NUM_THREADS='1',
               PYTHONDONTWRITEBYTECODE='1', PYTHONNOUSERSITE='1', MALLOC_ARENA_MAX='2')

    def read(path, expected=None):
        return audit.read(Path(path), expected)

    def leaf(label, argv, seconds=30, native=False):
        child_env = dict(env)
        if native:
            child_env.update(FE2O3_ALLOW_UNAUTHENTICATED_MACHINE_CODE_V1='1',
                             FE2O3_NATIVE_GUARDED_R2_REQUEST_V1=str(ROOT / 'request.json'),
                             FE2O3_NATIVE_GUARDED_R2_REQUEST_SHA256=request_pin['sha256'])
        h.run(label, argv, child_env, phases, deadline, None, seconds, ROOT)
        return (OUT / (label + '.stdout')).read_bytes()

    try:
        input_bytes, input_pin = read(ROOT / 'input-manifest.json')
        require(input_pin['sha256'] == sys.argv[1], 'frozen GPU inputs')
        inputs = audit.parse(input_bytes)
        require(set(inputs) == {'schema', 'files'} and inputs['schema'] == 'ferric-native-stable-r2-gpu-input-v1'
                and set(inputs['files']) == {'run_gpu.py', 'supervisor.py', 'library_audit.py',
                                            'verify_native.py', 'request.json'}, 'closed GPU input roster')
        for name, expected in inputs['files'].items():
            require({k: read(ROOT / name)[1][k] for k in ('bytes', 'sha256')} == expected, 'GPU input changed')
        cpu_bytes, cpu_pin = read(CPU / 'evidence/complete.json')
        require(cpu_pin['sha256'] == CPU_SHA, 'actual CPU terminal')
        cpu = audit.parse(cpu_bytes)
        require(cpu['passed'] is True and cpu['failure'] is None and cpu['postcheck_errors'] == []
                and cpu['source_unchanged'] is True and cpu['input_sources'] == cpu['final_sources']
                and cpu['gpu_execution'] is False and cpu['native_test_executed'] is False
                and cpu['native_test_name'] == TEST and TEST in cpu['ignored'], 'CPU diagnostic qualification')
        for pin in cpu['final_sources'].values():
            read(pin['path'], pin)
        for pin in cpu['raw'].values():
            read(pin['path'], pin)
        read(CPU / 'input-manifest.json', cpu['input_manifest'])
        selected = cpu['artifacts']['kfd-lib']
        binary = selected['pin']
        require(binary['bytes'] == 10587072 and binary['sha256']
                == '3508ea13687fd5872ef1ce37e769b312d8e996f1bb8301d1c19a7e48bda6fb43'
                and Path(binary['path']).is_relative_to(CPU / 'target/debug/deps'), 'selected CPU test ELF')
        elf, _ = read(binary['path'], binary)
        require(elf[:6] == b'\x7fELF\x02\x01' and elf[18:20] == b'\x3e\x00', 'native x86_64 ELF')
        cargo = [audit.parse(line) for line in (CPU / 'evidence/kfd-tests-build.stdout').read_bytes().splitlines()]
        require([row for row in cargo if row.get('reason') == 'compiler-artifact'
                 and row.get('executable') == binary['path']] == [selected['cargo_artifact']], 'Cargo ELF join')
        request_bytes, request_pin = read(ROOT / 'request.json')
        require(audit.parse(request_bytes) == dict(schema='ferric-native-stable-guarded-r2-request-v1',
                    device_unique_ids=IDS, image=str(IMAGE), image_sha256=IMAGE_SHA, timeout_ms=5000),
                'exact fixed native request')
        read(IMAGE, dict(path=str(IMAGE), bytes=28440, sha256=IMAGE_SHA))
        for tool in ('/usr/bin/python3', '/usr/bin/prlimit', '/usr/bin/readelf', '/usr/bin/ldd', '/opt/rocm/bin/amd-smi'):
            audit.resolved_input(tool)
        selftest = audit.parse(leaf('reference-tests', ['/usr/bin/python3', '-I', '-S', '-B',
                                                     str(ROOT / 'verify_native.py'), '--self-test'], 60))
        require(selftest['passed'] is True and selftest['gpu_execution'] is False
                and selftest['fixed_arithmetic_anchors'] == 32 and selftest['arithmetic_refusals'] == 13
                and selftest['observation_mutation_refusals'] == 14, 'independent reference selftest')
        inventory = leaf('native-list', [binary['path'], '--ignored', '--exact', TEST,
                                         '--list', '--format=terse']).decode().splitlines()
        require(inventory == [TEST + ': test'], 'one exact ignored native test')
        dynamic = leaf('readelf', ['/usr/bin/readelf', '-d', '-l', binary['path']])
        libraries = leaf('ldd', ['/usr/bin/ldd', binary['path']])
        runtime = audit.runtime_libraries(dynamic, libraries)
        h.save('runtime-libraries.json', runtime)
        topology_before = topology()
        h.save('topology-before.json', topology_before)
        h.save('inputs-before.json', audit.INPUTS)
        before = audit.idle(leaf('before', ['/opt/rocm/bin/amd-smi', 'process', '--json'], 60))
        require(deadline - time.monotonic() > 800,
                '600s native + 60s verification + 60s postflight + 50s cleanup + 30s final checks')
        native_attempt_requested = True
        raw = leaf('native', [binary['path'], '--ignored', '--exact', TEST,
                              '--show-output', '--test-threads=1'], 600, native=True)
        read(OUT / 'native.stdout', phases[-1]['stdout'])
        verification = audit.parse(leaf('verify', ['/usr/bin/python3', '-I', '-S', '-B',
            str(ROOT / 'verify_native.py'), str(OUT / 'native.stdout'), str(ROOT / 'request.json')], 60))
        require(verification['schema'] == 'ferric-native-stable-r2-integer-verification-v1'
                and verification['passed'] is True and verification['valid_output_elements_checked'] == 16384
                and verification['stdout_sha256'] == hashlib.sha256(raw).hexdigest()
                and verification['request_sha256'] == request_pin['sha256'],
                'actual independent full-bit reference')
        lines = [line for line in raw.splitlines() if line.startswith(b'FERRIC_NATIVE_GUARDED_R2_V1=')]
        require(len(lines) == 1, 'one native observation')
        observation = audit.parse(lines[0].split(b'=', 1)[1])
        h.save('observation.json', observation)
    except BaseException as error:
        errors.append(repr(error))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    # Postflight is read-only telemetry, never another dispatch or GPU reset.
    try:
        after = audit.idle(leaf('after', ['/opt/rocm/bin/amd-smi', 'process', '--json'], 60))
    except BaseException as error:
        errors.append('postflight: ' + repr(error))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - time.monotonic() - 5))
    try:
        for path, canonical in audit.ALIASES.items():
            require(str(Path(path).resolve(strict=True)) == canonical, 'tool/library alias drift')
        for pin in list(audit.INPUTS.values()):
            read(pin['path'], pin)
        require(topology_before is None or topology() == topology_before, 'topology drift')
        require(Path('/proc/sys/kernel/random/boot_id').read_text().strip() == boot, 'boot drift')
        require(time.monotonic() < deadline and all(p['reaped'] and p['process_group_absent'] for p in phases),
                'bounded terminal owned leaves')
        h.save('inputs-after.json', audit.INPUTS)
    except BaseException as error:
        post_errors.append(repr(error))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    passed = not errors and not post_errors and observation is not None and before is not None and after is not None
    value = dict(schema='ferric-native-stable-r2-gpu-result-v1', passed=passed,
                 errors=errors, postcheck_errors=post_errors, cpu_complete=cpu_pin, binary=binary,
                 request=request_pin, inputs=audit.INPUTS, input_aliases=audit.ALIASES,
                 phases=phases, runtime_libraries=runtime, reference_selftest=selftest,
                 verification=verification, observation=observation, idle_before=before, idle_after=after,
                 topology=topology_before, boot=boot, elapsed_seconds=time.monotonic() - started,
                 gpu_attempts=int(native_attempt_requested), native_attempt_requested=native_attempt_requested,
                 gpu_attempts_scope='requested native calls, not proof of successful spawn or GPU dispatch',
                 completed_native_phases=sum(p['label'] == 'native' for p in phases),
                 native_spawn_observed=(OUT / 'native.started.json').is_file(), retries=0,
                 stable_serial_component_qualified=passed, paired_coordinator_tested=False,
                 owner_lifecycle_tested=False, full_model_acceptance=False, production_authority=False,
                 performance_claim=False, limits=dict(whole_seconds=900, native_seconds=600,
                    dispatch_timeout_ms=5000, cpu_seconds=300, address_space_bytes=64 << 30,
                    stream_bytes=4 << 20, initial_free_bytes=40 << 30, live_free_bytes=38 << 30,
                    affinity=[8, 9], nice=10, cleanup_reserve_seconds=50),
                 raw={p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()})
    result = h.save('complete.json' if passed else 'failed.json', value)
    print(json.dumps(dict(passed=passed, errors=errors, postcheck_errors=post_errors,
                         receipt=result, gpu_attempts=value['gpu_attempts']), sort_keys=True))
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
