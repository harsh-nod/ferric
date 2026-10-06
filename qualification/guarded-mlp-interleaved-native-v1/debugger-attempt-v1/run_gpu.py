"""One debugger-assisted predicate capture; never bare-ELF qualification."""
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
ROOT = E / 'guarded-mlp-interleaved-debugger-gpu-v228-v1'
CPU = E / 'guarded-mlp-interleaved-native-cpu-v228-v1'
OUT = ROOT / 'evidence'
CPU_SHA = '09746cb353c2b1a6aa8e64a6aaefbb6451d9a538893856bac019105831e351f6'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
LIBRARY_AUDIT_SHA = 'b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'
OWNED_CHILDREN_SHA = '41e30b38b6e7205e51b3ba5a45113e9b305b8c706e42ea505ce094d8ede849f7'
IMAGES = [(ROOT / 'r1.hsaco', 10864, '25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25'),
          (ROOT / 'mlp.hsaco', 33320, 'b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589'),
          (ROOT / 'guarded.hsaco', 28440, 'de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66')]
TEST = 'engineering_gfx950::peer::combined_mlp_state_v1::tests::native::paired::interleaved::native_paired_guarded_mlp_interleaved_v1'
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
    signal.signal(signal.SIGCHLD, signal.SIG_DFL)
    h = load_exact('stable_r2_supervisor', ROOT / 'supervisor.py', SUPERVISOR_SHA)
    audit = load_exact('stable_r2_library_audit', ROOT / 'library_audit.py', LIBRARY_AUDIT_SHA)
    owned = load_exact('debugger_owned_children', ROOT / 'owned_children.py', OWNED_CHILDREN_SHA)
    owned.preconditions(empty=True)
    h.ROOT, h.OUT, h.TARGET, h.TMP = ROOT, OUT, ROOT / 'target', ROOT / 'tmp'
    h.AS_LIMIT, h.CPU_LIMIT, h.FILE_LIMIT, h.STREAM_LIMIT = 64 << 30, 300, 4 << 20, 4 << 20
    h.CACHE_LIMIT = 64 << 20
    h.CLEANUP_RESERVE = 80
    audit.HARD_DEADLINE = deadline
    OUT.mkdir(mode=0o700)
    h.TMP.mkdir(mode=0o700)
    for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, deadline - time.monotonic() - h.CLEANUP_RESERVE)
    phases, errors, post_errors = [], [], []
    retirements = []
    binary = cpu_pin = request_pin = runtime = before = after = topology_before = None
    verification = selftest = observation = cleanup_selftest = debugger_inventory = None
    native_attempt_requested = False
    boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    env = dict(HOME='/home/harmenon', PATH='/opt/rocm/bin:/usr/bin:/bin', LC_ALL='C', LANG='C',
               LD_LIBRARY_PATH=str(CPU / 'target/debug/deps') + ':' + TOOL_LIB,
               TMPDIR=str(h.TMP), RUST_BACKTRACE='1', OMP_NUM_THREADS='1',
               PYTHONDONTWRITEBYTECODE='1', PYTHONNOUSERSITE='1', MALLOC_ARENA_MAX='2')

    def read(path, expected=None):
        return audit.read(Path(path), expected)

    def save_compact(name, value, limit):
        # Keep raw native output capped at 4 MiB. The final parent receipt also
        # embeds the authenticated source/runtime metadata, with its own bound.
        body = (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()
        require(len(body) <= limit, 'bounded compact parent receipt')
        path = OUT / name
        with path.open('xb') as stream:
            stream.write(body)
        return audit.read(path, limit=limit, track=False)[1]

    def leaf(label, argv, seconds=30, native=False, debugger=False, expected_exit=0):
        child_env = dict(env)
        if debugger:
            child_env['FERRIC_DEBUGGER_MODE_V1'] = 'native' if native else 'inventory'
        if native:
            child_env.update(FE2O3_ALLOW_UNAUTHENTICATED_MACHINE_CODE_V1='1',
                             FE2O3_NATIVE_PAIRED_MLP_REQUEST_V1=str(ROOT / 'request.json'),
                             FE2O3_NATIVE_PAIRED_MLP_REQUEST_SHA256=request_pin['sha256'])
        owned.preconditions(empty=True)
        try:
            try:
                h.run(label, argv, child_env, phases, deadline, None, seconds, ROOT)
            except RuntimeError as error:
                require(expected_exit == 101 and label == 'native' and phases
                        and phases[-1]['label'] == label and phases[-1]['exit_code'] == 101
                        and str(error) == label + ' did not finish naturally/reaped/successfully',
                        'unexpected leaf refusal: ' + repr(error))
        finally:
            # ROCgdb gives its inferior a different PG. Only this serial
            # subreaper's authenticated adopted children may be retired here.
            retirement = owned.retire(min(deadline - 5, time.monotonic() + 20))
            retirements.append(dict(label=label, **retirement))
            h.save(label + '.owned-children.json', retirements[-1])
            require(retirement['complete'] and not retirement['forced']
                    and not retirement['deferred_signals'], 'natural complete child retirement')
        phase = phases[-1]
        require(phase['exit_code'] == expected_exit and phase['natural_exit']
                and phase['reaped'] and phase['process_group_absent'], 'natural expected leaf exit')
        require(max(phase['stdout']['bytes'], phase['stderr']['bytes']) <= h.STREAM_LIMIT
                and shutil.disk_usage(ROOT).free >= h.LIVE_FREE and h.scratch_bytes() <= h.CACHE_LIMIT,
                'post-leaf storage bounds including expected nonzero exit')
        require(time.monotonic() < deadline - h.CLEANUP_RESERVE, 'whole work deadline after cleanup')
        signal.setitimer(signal.ITIMER_REAL, deadline - h.CLEANUP_RESERVE - time.monotonic())
        return (OUT / (label + '.stdout')).read_bytes()

    def debugger_command(args):
        return ['/opt/rocm/bin/rocgdb', '-nx', '-nh', '--batch', '--return-child-result',
                '-iex', 'set auto-load off', '-iex', 'set debuginfod enabled off',
                '-x', str(ROOT / 'capture.gdb'), '--args', binary['path'], *args]

    def marker(raw, prefix):
        lines = [line for line in raw.splitlines() if line.startswith(prefix)]
        require(len(lines) == 1, 'one debugger marker: ' + prefix.decode())
        return audit.parse(lines[0].split(b'=', 1)[1])

    def debug_lifecycle(raw, mode, exit_code, hits):
        start = marker(raw, b'FERRIC_DEBUGGER_START_V1=')
        end = marker(raw, b'FERRIC_DEBUGGER_EXIT_V1=')
        phase = phases[-1]
        require(start['mode'] == mode and start['elf_sha256'] == binary['sha256']
                and start['debugger_pid'] == phase['pid']
                and start['inferior_pid'] > 1 and start['inferior_pid'] != phase['pid']
                and start['inferior_start_ticks'] > 0
                and end == dict(mode=mode, exit_codes=[exit_code], hits=hits)
                and phase['exit_code'] == exit_code and phase['natural_exit']
                and phase['reaped'] and phase['process_group_absent']
                and not phase['forced_cleanup'] and not phase['timed_out']
                and phase['exception'] is None and phase['storage_failure'] is None,
                'natural exact debugger/inferior lifetime')
        return dict(start=start, end=end)

    try:
        input_bytes, input_pin = read(ROOT / 'input-manifest.json')
        require(input_pin['sha256'] == sys.argv[1], 'frozen GPU inputs')
        inputs = audit.parse(input_bytes)
        require(set(inputs) == {'schema', 'files'} and inputs['schema'] == 'ferric-interleaved-mlp-debugger-input-v1'
                and set(inputs['files']) == {'run_gpu.py', 'supervisor.py', 'library_audit.py',
                                            'owned_children.py', 'capture.gdb', 'verify_native.py',
                                            'request.json', 'r1.hsaco', 'mlp.hsaco', 'guarded.hsaco'}, 'closed GPU input roster')
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
        require(binary['bytes'] == 11245904 and binary['sha256']
                == 'dbf5ea87824b4ab2be1e481cf21595f5809fb95bd309552e888a32f8089a8425'
                and Path(binary['path']).is_relative_to(CPU / 'target/debug/deps'), 'selected CPU test ELF')
        elf, _ = read(binary['path'], binary)
        require(elf[:6] == b'\x7fELF\x02\x01' and elf[18:20] == b'\x3e\x00', 'native x86_64 ELF')
        cargo = [audit.parse(line) for line in (CPU / 'evidence/kfd-tests-build.stdout').read_bytes().splitlines()]
        require([row for row in cargo if row.get('reason') == 'compiler-artifact'
                 and row.get('executable') == binary['path']] == [selected['cargo_artifact']], 'Cargo ELF join')
        request_bytes, request_pin = read(ROOT / 'request.json')
        require(audit.parse(request_bytes) == dict(schema='ferric-native-paired-guarded-mlp-request-v1',
                    device_unique_ids=IDS, images=[str(p) for p, _, _ in IMAGES],
                    image_sha256=[sha for _, _, sha in IMAGES], timeout_ms=5000),
                'exact fixed native request')
        for path, size, sha in IMAGES:
            read(path, dict(path=str(path), bytes=size, sha256=sha))
        for tool in ('/usr/bin/python3', '/usr/bin/prlimit', '/usr/bin/readelf', '/usr/bin/ldd',
                     '/opt/rocm/bin/rocgdb', '/opt/rocm/bin/amd-smi'):
            audit.resolved_input(tool)
        cleanup_selftest = audit.parse(leaf('owned-cleanup-tests', ['/usr/bin/python3', '-I', '-S', '-B',
                                                                 str(ROOT / 'owned_children.py')], 60))
        require(cleanup_selftest == dict(passed=True, gpu_execution=False, project_execution=False,
                    tests=['empty', 'escaped_group', 'nested_adoption', 'zombie', 'incomplete_proc_snapshot',
                           'nonchild_refused_without_signal', 'deadline_exhaustion',
                           'deferred_signal_and_timer']), 'actual owned cleanup CPU selftests')
        selftest = audit.parse(leaf('reference-tests', ['/usr/bin/python3', '-I', '-S', '-B',
                                                     str(ROOT / 'verify_native.py'), '--self-test'], 60))
        require(selftest['passed'] is True and selftest['gpu_execution'] is False
                and selftest['fixed_output_hashes'] == 16 and selftest['fixed_up_matrix_hashes'] == 16
                and selftest['strict_json_refusals'] == 2
                and selftest['stdout_refusals'] == 2
                and selftest['observation_mutation_refusals'] == 640
                and selftest['native_wire_upper_bytes'] < h.STREAM_LIMIT, 'independent reference selftest')
        dynamic = leaf('readelf', ['/usr/bin/readelf', '-d', '-l', binary['path']])
        libraries = leaf('ldd', ['/usr/bin/ldd', binary['path']])
        runtime = audit.runtime_libraries(dynamic, libraries)
        h.save('runtime-libraries.json', runtime)
        inventory = leaf('native-list', [binary['path'], '--ignored', '--exact', TEST,
                                         '--list', '--format=terse']).decode().splitlines()
        require(inventory == [TEST + ': test'], 'one exact ignored native test')
        raw_inventory = leaf('debugger-list', debugger_command(
            ['--ignored', '--exact', TEST, '--list', '--format=terse']), 60, debugger=True)
        debugger_inventory = debug_lifecycle(raw_inventory, 'inventory', 0, 0)
        require([line for line in raw_inventory.decode().splitlines() if line.endswith(': test')]
                == [TEST + ': test'], 'debugger exact inventory without test execution')
        topology_before = topology()
        h.save('topology-before.json', topology_before)
        h.save('inputs-before.json', audit.INPUTS)
        before = audit.idle(leaf('before', ['/opt/rocm/bin/amd-smi', 'process', '--json'], 60))
        require(deadline - time.monotonic() > 780,
                '600s native + 60s postflight + 80s cleanup + 30s final checks plus margin')
        native_attempt_requested = True
        raw = leaf('native', debugger_command(['--ignored', '--exact', TEST,
            '--show-output', '--test-threads=1']), 600, native=True, debugger=True, expected_exit=101)
        read(OUT / 'native.stdout', phases[-1]['stdout'])
        verification = debug_lifecycle(raw, 'native', 101, 1)
        observation = marker(raw, b'FERRIC_ROLE_PREDICATE_V1=')
        require(observation['schema'] == 'ferric-original-mlp-role-predicate-observation-v1'
                and observation['elf_sha256'] == binary['sha256'] and observation['hit'] == 1
                and observation['region_pc_offset'] == 0x136, 'exact refusal observation')
        u64 = {'hit', 'region_pc_offset', 'saved_rank', 'saved_byte_index', 'expected_owner',
               'expected_bytes', 'validation_tag_u64', 'group_id', 'eflags_u64'}
        u32 = {'combined_u32', 'eax_u32', 'ecx_u32'}
        u8 = {'record_kind_u8', 'mapping_u8'}
        words = {'source_token', 'copied_token', 'record_token'}
        flags = {'source_address_matches_role', 'record_pointer_matches_return'}
        require(set(observation) == u64 | u32 | u8 | words | flags
                | {'schema', 'elf_sha256', 'prepare_return_offset', 'role', 'root_index'},
                'closed predicate field roster')
        for names, bits in ((u64, 64), (u32, 32), (u8, 8)):
            require(all(type(observation[name]) is int and 0 <= observation[name] < 1 << bits
                        for name in names), 'exact-width machine scalar')
        require(all(type(observation[name]) is bool for name in flags)
                and all(type(observation[name]) is list and len(observation[name]) == 4
                        and all(type(word) is int and 0 <= word < 1 << 64 for word in observation[name])
                        for name in words), 'token and flag scalar types')
        require(type(observation['prepare_return_offset']) is int, 'signed caller offset')
        roles = {0x10a: 'root', 0x189: 'combined', 0x3ae: 'partial0', 0x636: 'partial1',
                 0x6da: 'residual0', 0x760: 'residual1', 0x7cd: 'output0', 0x84f: 'output1'}
        role = roles.get(observation['prepare_return_offset'], 'unknown')
        require(observation['role'] == role and observation['root_index']
                == (observation['saved_byte_index'] // 32 if role == 'root' else None), 'caller-role derivation')
        predicates = dict(owner=observation['source_token'][2] == observation['expected_owner'],
                          extent=observation['source_token'][3] == observation['expected_bytes'],
                          kind=observation['record_kind_u8'] == 2 * (observation['combined_u32'] & 0xff),
                          mapping=observation['mapping_u8'] == 1)
        # Discrepancies are evidence, not grounds to discard the capture.
        verification.update(predicates=predicates,
            first_false_predicate=next((key for key, value in predicates.items() if not value), None),
            tokens_equal=observation['source_token'] == observation['copied_token'] == observation['record_token'],
            root_loop_valid=(observation['saved_rank'] < 2
                and observation['saved_rank'] == observation['expected_owner']
                and observation['saved_byte_index'] < 0x140 and observation['saved_byte_index'] % 32 == 0)
                if role == 'root' else None)
        require(b'paired guarded MLP typed allocation role' in raw
                and b'test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1043 filtered out;' in raw
                and b'FERRIC_NATIVE_PAIRED_INTERLEAVED_MLP_V1=' not in raw,
                'original libtest failure, not numerical acceptance')
        save_compact('observation.json', observation, 4 << 20)
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
        owned.preconditions(empty=True)
        require(len(retirements) == len(phases) and all(r['complete'] and not r['forced']
                and not r['deferred_signals'] for r in retirements), 'all descendants naturally retired')
        h.save('inputs-after.json', audit.INPUTS)
    except BaseException as error:
        post_errors.append(repr(error))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    passed = not errors and not post_errors and observation is not None and before is not None and after is not None
    value = dict(schema='ferric-interleaved-mlp-debugger-result-v1', passed=passed,
                 errors=errors, postcheck_errors=post_errors, cpu_complete=cpu_pin, binary=binary,
                 request=request_pin, inputs=audit.INPUTS, input_aliases=audit.ALIASES,
                 phases=phases, runtime_libraries=runtime, reference_selftest=selftest,
                 owned_cleanup_selftest=cleanup_selftest, owned_retirements=retirements,
                 debugger_inventory=debugger_inventory, debugger_assisted=True,
                 verification=verification, observation=observation, idle_before=before, idle_after=after,
                 topology=topology_before, boot=boot, elapsed_seconds=time.monotonic() - started,
                 gpu_attempts=int(native_attempt_requested), native_attempt_requested=native_attempt_requested,
                 gpu_attempts_scope='requested native calls, not proof of successful spawn or GPU dispatch',
                 completed_native_phases=sum(p['label'] == 'native' for p in phases),
                 native_spawn_observed=(OUT / 'native.started.json').is_file(), retries=0,
                 refusal_capture_completed=passed, synthetic_paired_component_qualified=False,
                 paired_coordinator_tested=False, owner_lifecycle_tested=False,
                 rearm_tested=False, interleaving_tested=False,
                 worker_integrated=False,
                 full_model_acceptance=False, production_authority=False,
                 performance_claim=False, limits=dict(whole_seconds=900, native_seconds=600,
                    dispatch_timeout_ms=5000, cpu_seconds=300, address_space_bytes=64 << 30,
                    stream_bytes=4 << 20, parent_receipt_bytes=8 << 20,
                    initial_free_bytes=40 << 30, live_free_bytes=38 << 30,
                    affinity=[8, 9], nice=10, cleanup_reserve_seconds=80,
                    adopted_child_cleanup_seconds=20),
                 raw={p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()})
    result = save_compact('complete.json' if passed else 'failed.json', value, 8 << 20)
    print(json.dumps(dict(passed=passed, errors=errors, postcheck_errors=post_errors,
                         receipt=result, gpu_attempts=value['gpu_attempts']), sort_keys=True))
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
