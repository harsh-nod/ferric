"""One explicit reusable AR4 run; mandatory plateau/equality and unchanged owned cleanup."""
import functools
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import stat
import struct
import subprocess
import sys
import time
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-reusable-ar4-gpu-v228-v1'
PLAN_SHA = {'ar4': '3b1db3249727bcd363ce585efeace612f46c703cf5caf264a5e3384eed09a00e'}
CENSUS_CPU = dict(bytes=1513, sha256='a1f048f2a32ccb44f5c2d7f6263faa3620a4eaf130a2c55222c8e2c32461f5a8')
CENSUS_CONTROLLER = dict(bytes=6463, sha256='6374616ca699a066d6814b45e2ce233f289cc09204cf7bc5af1ea7c366f3ae37')
CENSUS_TEST = dict(bytes=4375, sha256='d4f19ae2f4bcf9f68464a7b8951b82368183cec6db998a0ed55449d2349e7f64')
TF4_REVALIDATION_SHA = 'd0551f310a57f63dfe98af8c887b5996752e3b62128a3087c4a03849082e6a3f'
ANNOUNCEMENT_SHA = '96f84389071ef2c1a9e354705ebb7144e9ff97e30c242bd9b494f03d2aea6b80'
PARSER_CPU = dict(path=str(E / 'guarded-mlp-model-announcement-cpu-v228-v1/evidence/complete.json'),
    bytes=8872, sha256='170ba440cdf33cd88fc55c2ddff709624fd819d3d461d54daad239a719efdf5a')
VALIDATOR_SHA = '9d0f860d0174ef5574c3023253b4e220ff16bfba845ad13c7e0ce210cdc66ed7'
OWNED_SHA = 'ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'
AUDIT_SHA = 'b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'
IDS = [16366993098680759275, 10838076764495710945]
NATIVE_SECONDS, CASE_SECONDS, AUDIT_SECONDS = 4000, 4300, 30
STREAM_CAP, CASE_CAP = 8 << 20, 64 << 20
SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)
ENV = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
           OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1',
           HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
WORKER = dict(bytes=5912952, sha256='a5c5c8b323e2d88ec27df1065709171fb9bd46114aff99b083b63d3644743fb8')
WORKER_CPU = dict(bytes=1631185, sha256='24feb83a0bde60dc9c6b8db28f2ce8252379b40d8e0d40483690f58038881b2a')
PARENT = dict(bytes=13859224, sha256='3d02886f0d36ebcd73d34797e0046eeb850eff0c49a1cff0e7ac16dd46c6fa91')
PARENT_CPU = dict(bytes=3796001, sha256='f24fab8b524afc87726d6aec918abce01dc39643f56c7ac7f323af4c4ce953f2')
SELECTED_IMAGES = {
    'projection_image': (10864, '25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25'),
    'guarded_image': (28440, 'de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66'),
    'prefix_image': (54344, '29fd58e7b09fee003ed31660e6201c20a7eb5da4b873d0b4d954993f7018f2f8'),
    'tiles_image': (33320, 'b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589'),
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def load(path, sha, name):
    require(type(sha) is str and re.fullmatch('[0-9a-f]{64}', sha), 'unbound helper SHA')
    path = Path(path)
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= 1 << 20, 'ordinary helper')
    with path.open('rb') as stream:
        raw = stream.read((1 << 20) + 1); after = os.fstat(stream.fileno())
    fields = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(fields(before) == fields(after) == fields(path.lstat()) and len(raw) == before.st_size
            and hashlib.sha256(raw).hexdigest() == sha, 'frozen helper bytes')
    module = types.ModuleType(name); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def save(path, value):
    raw = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    require(len(raw) <= STREAM_CAP, 'bounded JSON output')
    with path.open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def inventory(directory):
    count = size = 0
    for parent, dirs, files in os.walk(directory, followlinks=False, onerror=lambda e: (_ for _ in ()).throw(e)):
        for name in dirs + files:
            row = (Path(parent) / name).lstat(); count += 1
            require(row.st_uid == os.getuid() and (stat.S_ISDIR(row.st_mode) or stat.S_ISREG(row.st_mode)),
                    'owned ordinary output tree')
            if stat.S_ISREG(row.st_mode):
                require(row.st_nlink == 1 and row.st_size <= CASE_CAP, 'bounded ordinary output')
                size += row.st_size
    require(count <= 256 and size <= CASE_CAP, '64 MiB/256 member case cap')


def resources(owned, initial=False):
    owned.resources(initial)


def child_limits(seconds):
    os.sched_setaffinity(0, {8, 9}); os.nice(10)
    for kind, value in ((resource.RLIMIT_AS, (32 if seconds == NATIVE_SECONDS else 12) << 30),
                        (resource.RLIMIT_FSIZE, CASE_CAP), (resource.RLIMIT_CPU, seconds), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        ceiling = min(value, hard) if hard != resource.RLIM_INFINITY else value
        if soft != resource.RLIM_INFINITY:
            ceiling = min(ceiling, soft)
        resource.setrlimit(kind, (ceiling, ceiling))


def interrupted(number, _frame):
    raise RuntimeError('supervisor signal ' + str(number))


def bounded(directory, argv, owned, audit, case_root, seconds, deadline):
    require(seconds in (NATIVE_SECONDS, AUDIT_SECONDS), 'closed leaf deadline')
    command = save(directory / 'command.json', dict(argv=argv, env=ENV, cwd=str(E),
        deadline_seconds=seconds, affinity=[8, 9], nice=10,
        address_space_bytes=(32 if seconds == NATIVE_SECONDS else 12) << 30,
        file_cap_bytes=CASE_CAP, stream_cap_bytes=STREAM_CAP,
        gpu_execution_requested=seconds == NATIVE_SECONDS))
    owned.subreaper()
    tracker, child, outcome, reason = owned.OwnedProcesses(), None, None, None
    started = None; start = time.monotonic()
    handlers = {n: signal.getsignal(n) for n in SIGNALS}
    try:
        for n in handlers:
            signal.signal(n, interrupted)
        with (directory / 'stdout').open('xb') as stdout, (directory / 'stderr').open('xb') as stderr:
            child = subprocess.Popen(argv, cwd=E, env=ENV, stdin=subprocess.DEVNULL,
                stdout=stdout, stderr=stderr, start_new_session=True,
                preexec_fn=functools.partial(child_limits, seconds))
            tracker.attach(child)
            started = save(directory / 'started.json', dict(parent=tracker.root,
                supervisor_pid=os.getpid(), command_sha256=command['sha256']))
            while True:
                tracker.discover()
                require(time.monotonic() - start < seconds and time.monotonic() + 65 < deadline,
                        'owned leaf deadline or cleanup reserve')
                require(max(os.fstat(stdout.fileno()).st_size, os.fstat(stderr.fileno()).st_size)
                        <= STREAM_CAP, 'owned leaf stream cap')
                resources(owned); inventory(case_root)
                if os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT):
                    break
                time.sleep(.2)
    except BaseException as error:
        reason = type(error).__name__ + ': ' + str(error)
    finally:
        try:
            if child is not None:
                for n in handlers:
                    signal.signal(n, signal.SIG_IGN)
                try:
                    require(tracker.root is not None and owned.token(tracker.root) in tracker.records,
                            'reserved parent pidfd initialization')
                    outcome = tracker.cleanup()
                except BaseException as error:
                    reason = reason or ('primary cleanup failed: ' + repr(error))
                    outcome = owned.emergency_reap(child, tracker.events)
                if outcome['cleanup_signalled']:
                    reason = reason or 'owned tree needed forced cleanup'
        finally:
            try:
                tracker.close_fds()
            finally:
                for n, handler in handlers.items():
                    signal.signal(n, handler)
    if outcome is None:
        require(child is None, 'spawned child without owned cleanup result')
        outcome = dict(exit_code=None, cleanup_signalled=False, owned_groups=[], owned_groups_absent=True,
                       owned_processes_reaped=True, lineage=[])
    row = dict(**outcome, reason=reason, command=command, started=started,
        stdout=audit.read(directory / 'stdout', limit=STREAM_CAP, track=False, retain=False)[1],
        stderr=audit.read(directory / 'stderr', limit=STREAM_CAP, track=False, retain=False)[1],
        elapsed_seconds=time.monotonic() - start, gpu_execution_requested=seconds == NATIVE_SECONDS)
    save(directory / 'result.json', row)
    return row


def successful(row):
    require(type(row['exit_code']) is int and row['exit_code'] == 0 and row['reason'] is None
            and row['cleanup_signalled'] is False and row['owned_groups_absent'] is True
            and row['owned_processes_reaped'] is True, 'natural successful owned tree required')


def parse_topology_node(node, raw, gpu_raw):
    require(type(raw) is str and 0 < len(raw) <= 16384
            and type(gpu_raw) is str and 0 < len(gpu_raw) <= 64, 'topology input bounds')
    pairs = [line.split() for line in raw.splitlines()]
    require(all(len(row) == 2 for row in pairs) and len({row[0] for row in pairs}) == len(pairs),
            'topology property syntax')
    properties = dict(pairs)
    for value in (gpu_raw.strip(), properties.get('unique_id', ''), properties.get('gfx_target_version', '')):
        require(re.fullmatch('[0-9]+', value), 'topology numeric identity syntax')
    return dict(node=node, gpu_id=int(gpu_raw.strip()), properties=properties)


def topology():
    rows = []
    for node in (2, 3):
        directory = Path('/sys/class/kfd/kfd/topology/nodes') / str(node)
        with (directory / 'properties').open() as stream:
            raw = stream.read(16385)
        with (directory / 'gpu_id').open() as stream:
            gpu_raw = stream.read(65)
        rows.append(parse_topology_node(node, raw, gpu_raw))
    return dict(host=os.uname().nodename, boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip(), devices=rows)


def validate_topology(snapshot):
    require(snapshot['host'] == 'smci350-rck-g03-b19-03'
            and re.fullmatch('[0-9a-f-]{36}', snapshot['boot']), 'selected host/boot shape')
    require(len(snapshot['devices']) == 2, 'selected topology extent')
    for row, node, unique_id in zip(snapshot['devices'], (2, 3), IDS):
        require(row['node'] == node and type(row['gpu_id']) is int and row['gpu_id'] > 0
                and int(row['properties']['unique_id']) == unique_id
                and int(row['properties']['gfx_target_version']) == 90500,
                'exact selected gfx950 unique_id; gpu_id is a separate KFD handle')


def cpu_admission(plan, audit):
    def doc(pin):
        return audit.parse(audit.read(pin['path'], pin)[0])
    require(all(type(pin) is dict and set(pin) == {'bytes', 'sha256'}
                for pin in (WORKER, WORKER_CPU, PARENT, PARENT_CPU)),
            'actual reusable AR4 CPU/ELF bindings remain pending')
    worker = doc(plan['worker_cpu']); parent = doc(plan['parent_cpu'])
    require({k: plan['worker_cpu'][k] for k in ('bytes', 'sha256')} == WORKER_CPU
            and {k: plan['worker'][k] for k in ('bytes', 'sha256')} == WORKER
            and {k: plan['parent_cpu'][k] for k in ('bytes', 'sha256')} == PARENT_CPU
            and {k: plan['parent'][k] for k in ('bytes', 'sha256')} == PARENT,
            'actual reusable AR4 CPU/ELF pins')
    for value, schema in [(worker, 'ferric-guarded-mlp-reusable-ar4-worker-cpu-v1'),
                          (parent, 'ferric-guarded-mlp-reusable-ar4-parent-cpu-v1')]:
        require(value['schema'] == schema and value['passed'] is True and value['failure'] is None
                and value['postcheck_errors'] == [] and value['source_unchanged'] is True
                and value['input_sources'] == value['final_sources']
                and value['gpu_execution'] is False and value['reusable_arena_native_execution'] is False
                and value['default_arena_policy_changed'] is False and value['full_long_workload'] is False
                and value['inherited_shared_full_currentness_source_added'] is True
                and value['inherited_host_observation_source_added'] is True
                and value['inherited_capture_source_added'] is True
                and value['performance_policy_changed'] is False and value['default_policy_changed'] is False, 'qualified reusable AR4 CPU terminal')
        for key in ('controller', 'input_manifest'):
            audit.read(value[key]['path'], value[key], retain=False)
        source_pin = value['raw']['sources-after.json']
        require(doc(source_pin) == value['final_sources'], 'CPU final source map/body')
        for phase in value['phases']:
            require(phase['natural_exit'] is True and phase['exit_code'] == 0
                    and phase['reaped'] is True and phase['process_group_absent'] is True
                    and phase['forced_cleanup'] is False and phase['timed_out'] is False
                    and phase['exception'] is None and phase['storage_failure'] is None,
                    'CPU leaf lifecycle')
    require(worker['source_generation'] == '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'
            and worker['reusable_arena_worker_route_added'] is True
            and worker['runtime_source_qualified_by_reusable_cpu'] is True
            and len(worker['phases']) == 9
            and worker['full_worker_tests_executed'] is True
            and len(worker['inventory']) == 619
            and worker['tests']['worker-tests']['failed'] == 0
            and worker['tests']['worker-tests']['passed'] == 615
            and worker['tests']['worker-tests']['ignored'] == 4, 'reusable AR4 worker qualification census')
    require(parent['reusable_arena_parent_route_added'] is True and len(parent['phases']) == 55 and len(parent['tests']) == 47 and len(parent['artifacts']) == 5
            and len(parent['inventory']) == 875
            and sum(row['passed'] for row in parent['tests'].values()) == 396
            and all(row['failed'] == row['ignored'] == 0 for row in parent['tests'].values())
            and parent['all_selected_parent_tests_executed'] is True
            and parent['full_parent_library_suite_executed'] is False, 'selected reusable AR4 parent qualification census')
    require(worker['artifacts']['worker']['pin'] == plan['worker'], 'qualified worker Cargo ELF')
    artifact = parent['artifacts']['ferric-qwen3-finite-guarded-mlp-decode-engineering']
    require(artifact['pin'] == plan['parent'], 'qualified parent Cargo ELF')
    for key in ('parent', 'worker'):
        raw, _ = audit.read(plan[key]['path'], plan[key], limit=128 << 20)
        require(raw[:6] == b'\x7fELF\x02\x01' and raw[18:20] == b'\x3e\x00', 'qualified x86_64 ELF')
    return dict(parent_cpu=plan['parent_cpu'], worker_cpu=plan['worker_cpu'], parent=plan['parent'], worker=plan['worker'])


def census_admission(audit):
    require(type(CENSUS_CPU) is dict and set(CENSUS_CPU) == {'bytes', 'sha256'},
            'actual eight-test census checker remains pending')
    path = E / 'guarded-mlp-reusable-ar4-census-cpu-v228-v1/complete.json'
    pin = dict(path=str(path), **CENSUS_CPU)
    value = audit.parse(audit.read(path, pin)[0])
    expected = {
        str(E / 'guarded-mlp-reusable-ar4-validator-v228-v1.py'):
            dict(bytes=15574, sha256=VALIDATOR_SHA),
        str(E / 'guarded-mlp-reusable-ar4-census-tests-v228-v1.py'): CENSUS_TEST,
    }
    require(value['schema'] == 'ferric-guarded-mlp-reusable-ar4-census-cpu-v1'
            and value['passed'] is True and value['failure'] is None
            and value['postcheck_errors'] == [] and value['controller'] == CENSUS_CONTROLLER
            and value['sources'] == expected and type(value['tests_run']) is int
            and value['tests_run'] == 8 and type(value['failures']) is int
            and type(value['errors']) is int and value['failures'] == value['errors'] == 0
            and value['synthetic_evidence_only'] is True and value['subprocesses_started'] == 0
            and value['gpu_execution'] is False and value['numerical_acceptance'] is False
            and value['performance_claim'] is False, 'actual exact eight-test synthetic census gate')
    for name, body_pin in expected.items():
        audit.read(name, dict(path=name, **body_pin), retain=False)
    for name in ('stdout', 'stderr'):
        body_pin = value[name]
        require(body_pin['path'] == str(path.parent / name), 'closed census checker raw path')
        audit.read(body_pin['path'], body_pin, retain=False)
    return pin


def ordinary_ar4(audit, validator):
    pin = dict(path=str(E / 'guarded-mlp-model-gpu-v228-v3/ar4/complete.json'), bytes=93497,
               sha256='edf05cf2dd19a9934dab9762b328ea3e6160cf01c15606a3df3299ce967e4991')
    prior = audit.parse(audit.read(pin['path'], pin)[0])
    require(prior['passed'] is True and prior['errors'] == [] and prior['mode'] == 'ar4'
            and prior['native_attempts'] == 1 and prior['retries'] == 0,
            'exact earlier ordinary AR4 result')
    values = []
    for index in range(4):
        body_pin = prior['raw']['native/observation-%d.bin' % index]
        require(body_pin['path'] == str(E / ('guarded-mlp-model-gpu-v228-v3/ar4/native/observation-%d.bin' % index))
                and body_pin['bytes'] == validator.PAYLOAD_BYTES, 'ordinary AR4 exact payload path/extent')
        values.append((body_pin, audit.read(body_pin['path'], body_pin)[0]))
    return pin, prior['observation']['input_tokens'], values


def compare_payloads(summary_raw, read_body, validator, prior):
    summary = validator.parse(summary_raw)
    prior_pin, prior_inputs, values = prior
    rows = []
    for index, (old_pin, old) in enumerate(values):
        current = validator.rust_pin(summary['files']['frames'][index]['observation'])
        raw = read_body(current)
        require(len(raw) == current['bytes'] == len(old) == validator.PAYLOAD_BYTES
                and hashlib.sha256(raw).hexdigest() == current['sha256'], 'current observation body')
        rows.append(dict(position=index, current=current, ordinary=old_pin, byte_equal=raw == old,
                         same_history=summary['input_tokens'][:index + 1] == prior_inputs[:index + 1]))
    return dict(schema='ferric-guarded-mlp-reusable-ar4-comparison-v1', ordinary_terminal=prior_pin,
        frames=rows, all_payloads_equal=all(row['byte_equal'] for row in rows),
        all_histories_equal=all(row['same_history'] for row in rows),
        observed_payload_difference=any(not row['byte_equal'] for row in rows),
        causal_effect_established=False,
        comparison_completed=True, independent_accuracy_reference=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python required')
    require(len(sys.argv) == 2 and sys.argv[1] in PLAN_SHA, 'one explicit reusable AR4 ar4 selector; no implicit model rerun')
    mode = sys.argv[1]; directory = Path(__file__).resolve().parent
    require(directory == ROOT and os.getuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged deployment host/root')
    require(type(PLAN_SHA[mode]) is str and re.fullmatch('[0-9a-f]{64}', PLAN_SHA[mode]), 'actual case input remains unbound')
    audit = load(directory / 'library_audit.py', AUDIT_SHA, 'guarded_model_audit')
    owned = load(directory / 'frozen_owned.py', OWNED_SHA, 'guarded_model_owned')
    validator = load(directory / 'validate_observation.py', VALIDATOR_SHA, 'guarded_model_validation')
    guard = load(directory / 'guarded_announcement.py', ANNOUNCEMENT_SHA, 'guarded_model_announcement')
    start = time.monotonic(); deadline = start + CASE_SECONDS; audit.HARD_DEADLINE = deadline
    plan_raw, plan_pin = audit.read(directory / (mode + '-input.json'), limit=65536)
    require(plan_pin['sha256'] == PLAN_SHA[mode], 'exact case input')
    plan = audit.parse(plan_raw)
    validator.keys(plan, 'schema mode parent_cpu worker_cpu parent worker request')
    require(plan['schema'] == 'ferric-guarded-mlp-reusable-ar4-gpu-input-v1' and plan['mode'] == mode, 'case generation')
    out = ROOT / mode; require(not os.path.lexists(out), 'fresh one-shot case root')
    resources(owned, True)
    request_raw, _ = audit.read(plan['request']['path'], plan['request'], limit=16384)
    request = validator.parse(request_raw); config = request['decode']
    require(request['schema'] == 'FerricFiniteGuardedMlpReusableAr4RequestV1'
            and config['mode'] == ('teacher_forced' if mode == 'tf4' else 'autoregressive')
            and config['device_ids'] == IDS and config['child_deadline_ms'] == 3600000
            and config['dispatch_timeout_ms'] == 10000 and config['evidence_directory'] == str(out / 'native')
            and validator.rust_pin(config['worker']) == plan['worker'], 'closed guarded request')
    for name, extent in SELECTED_IMAGES.items():
        value = request[name] if name in request else config[name]
        pin = validator.rust_pin(value)
        require((pin['bytes'], pin['sha256']) == extent, 'selected qualified image')
        audit.read(pin['path'], pin, retain=False)
    for value in [*config['images'].values(), *config['prompt'].values()]:
        pin = validator.rust_pin(value); audit.read(pin['path'], pin, retain=False)
    require(config['source'] == '/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model'
            and validator.octets(config['expected_model_id']).hex() == 'f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a'
            and validator.octets(config['expected_bundle_id']).hex() == '6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b',
            'authenticated source bundle identity')
    admission = cpu_admission(plan, audit)
    census_cpu = census_admission(audit)
    prior_payloads = ordinary_ar4(audit, validator)
    parser_cpu = audit.parse(audit.read(PARSER_CPU['path'], PARSER_CPU)[0])
    require(parser_cpu['passed'] is True and parser_cpu['failure'] is None
            and parser_cpu['postcheck_errors'] == [] and parser_cpu['source_unchanged'] is True
            and parser_cpu['sources_before'] == parser_cpu['sources_after']
            and parser_cpu['tests']['passed'] == 6
            and parser_cpu['tests']['failed'] == parser_cpu['tests']['errors'] == parser_cpu['tests']['skipped'] == 0
            and parser_cpu['sources_before']['guarded_announcement.py']['sha256'] == ANNOUNCEMENT_SHA,
            'actual six-test guarded parser qualification')
    require(type(TF4_REVALIDATION_SHA) is str and re.fullmatch('[0-9a-f]{64}', TF4_REVALIDATION_SHA),
            'actual immutable TF4 revalidation remains pending')
    revalidation_raw, revalidation_pin = audit.read(E / 'guarded-mlp-model-tf4-revalidation-v228-v3/complete.json')
    require(revalidation_pin['sha256'] == TF4_REVALIDATION_SHA, 'actual TF4 revalidation pin')
    revalidation = audit.parse(revalidation_raw)
    require(revalidation['schema'] == 'ferric-guarded-mlp-model-tf4-data-revalidation-v1'
            and revalidation['passed'] is True and revalidation['error'] is None
            and revalidation['actual_original_observation_revalidated'] is True
            and revalidation['input_posthashes_complete'] is True
            and revalidation['original_controller_passed'] is False
            and revalidation['original_failure_preserved'] is True
            and revalidation['native_rerun'] is False and revalidation['gpu_execution'] is False
            and revalidation['parser_cpu'] == PARSER_CPU
            and revalidation['original_terminal']['sha256'] == '8ced1ef7f167348fbcb68ce56cca8fc8e2238273ebe18fd737127563779871ce',
            'distinct data-only TF4 revalidation; original failure preserved')
    for path in (directory / 'run_model_gpu.py', directory / 'library_audit.py', directory / 'frozen_owned.py',
                 directory / 'validate_observation.py', directory / 'guarded_announcement.py', Path('/opt/rocm/bin/amd-smi'),
                 Path('/usr/bin/readelf'), Path('/usr/bin/ldd'), Path(sys.executable)):
        audit.resolved_input(path)
    controller_pin = audit.INPUTS[str(directory / 'run_model_gpu.py')]
    out.mkdir(mode=0o700); phases = []; errors = []; checked = None; payload_comparison = None; attempts = 0; dependencies = {}; initial = None
    def leaf(name, argv, seconds, post=False):
        if not post:
            require(time.monotonic() + seconds + 3 * AUDIT_SECONDS + 65 < deadline, 'case post-audit/cleanup reserve')
        else:
            require(time.monotonic() + seconds + 65 < deadline, 'post-audit cleanup reserve')
        child_dir = out / name; child_dir.mkdir(mode=0o700)
        row = bounded(child_dir, argv, owned, audit, out, seconds, deadline); phases.append(dict(label=name, **row))
        successful(row)
        return audit.read(child_dir / 'stdout', limit=STREAM_CAP, track=False)[0]
    def idle(name, post=False):
        snapshot = topology()
        save(out / (name + '-topology.json'), snapshot)
        validate_topology(snapshot)
        require(initial is not None and snapshot == initial, 'platform/boot/topology drift')
        raw = leaf(name, ['/opt/rocm/bin/amd-smi', 'process', '--json'], AUDIT_SECONDS, post)
        audit.idle(raw)
    handlers = {n: signal.getsignal(n) for n in SIGNALS}
    for number in handlers:
        signal.signal(number, interrupted)
    try:
        initial = topology()
        save(out / 'initial-topology.json', initial)
        validate_topology(initial)
        for key in ('parent', 'worker'):
            dynamic = leaf(key + '-readelf', ['/usr/bin/readelf', '-l', '-d', plan[key]['path']], AUDIT_SECONDS)
            linked = leaf(key + '-ldd', ['/usr/bin/ldd', plan[key]['path']], AUDIT_SECONDS)
            dependencies[key] = audit.runtime_libraries(dynamic, linked)
        for index in range(3):
            idle('before-' + str(index))
            if index < 2: time.sleep(.2)
        attempts = 1
        raw = leaf('parent', [plan['parent']['path'], '--request', plan['request']['path'],
            '--observe-guarded-reusable-ar4', '--allow-unauthenticated-machine-code'], NATIVE_SECONDS)
        summary_path = out / 'native/complete.json'
        summary, summary_pin = audit.read(summary_path, limit=65536, track=False)
        require(validator.parse(raw) == validator.parse(summary), 'parent stdout/retained summary join')
        expected = {'complete.json', 'child-stderr.bin'} | {f'{stem}-{i}.{suffix}' for i in range(4)
            for stem, suffix in [('request', 'json'), ('control', 'bin'), ('observation', 'bin')]}
        require({p.name for p in summary_path.parent.iterdir()} == expected, 'exact fourteen-file native closure')
        def read_body(pin):
            return audit.read(pin['path'], pin, limit=2 << 20, track=False)[0]
        checked = validator.validate(summary, request, summary_path.parent, read_body)
        native = phases[-1]
        start_doc = audit.parse(audit.read(out / 'parent/started.json', track=False)[0])
        guard.validate_lineage(audit.read(out / 'parent/stderr', track=False)[0],
                               native, start_doc, checked['child_pid'])
        payload_comparison = compare_payloads(summary, read_body, validator, prior_payloads)
        require(payload_comparison['all_payloads_equal'] is True
                and payload_comparison['all_histories_equal'] is True,
                'all four prior ordinary AR4 payloads and histories must match')
        require(checked['actual_arena_plateau_verified'] is True,
                'actual five-sample arena plateau and healthy Close required')
        save(out / 'observation.json', checked)
        save(out / 'ordinary-comparison.json', payload_comparison)
    except BaseException as error:
        errors.append(type(error).__name__ + ': ' + str(error))
    finally:
        for number in handlers:
            signal.signal(number, signal.SIG_IGN)
        for index in range(3):
            try: idle('after-' + str(index), True)
            except BaseException as error: errors.append('postflight: ' + repr(error))
            if index < 2: time.sleep(.2)
        try:
            require(time.monotonic() < deadline, 'whole case deadline')
            require(not any(r['ppid'] == os.getpid() for r in owned.processes().values()), 'all owned children absent')
            for path, pin in list(audit.INPUTS.items()):
                audit.read(path, pin, retain=False)
            for alias, canonical in audit.ALIASES.items():
                require(str(Path(alias).resolve(strict=True)) == canonical, 'input/library alias drift')
            resources(owned); inventory(out)
        except BaseException as error:
            errors.append('postcheck: ' + repr(error))
    raw_files = {str(p.relative_to(out)): audit.read(p, retain=False, track=False)[1]
                 for p in sorted(out.rglob('*')) if p.is_file()}
    try:
        for phase in phases:
            label = phase['label']
            for key, filename in [('command', 'command.json'), ('started', 'started.json'),
                                  ('stdout', 'stdout'), ('stderr', 'stderr')]:
                if phase[key] is not None:
                    require(raw_files.get(label + '/' + filename) == phase[key], 'raw phase pin drift')
            result_pin = raw_files[label + '/result.json']
            result_raw, actual_pin = audit.read(result_pin['path'], track=False)
            require(actual_pin == result_pin and audit.parse(result_raw) ==
                    {key: value for key, value in phase.items() if key != 'label'}, 'phase result/body drift')
        if checked is not None and not errors:
            expected = ['parent-readelf', 'parent-ldd', 'worker-readelf', 'worker-ldd']
            expected += ['before-' + str(i) for i in range(3)] + ['parent']
            expected += ['after-' + str(i) for i in range(3)]
            require([row['label'] for row in phases] == expected, 'exact eleven successful phases')
            expected_raw = {label + '/' + filename for label in expected
                            for filename in ('command.json', 'started.json', 'stdout', 'stderr', 'result.json')}
            expected_raw |= {'initial-topology.json'} | {when + '-' + str(i) + '-topology.json'
                for when in ('before', 'after') for i in range(3)}
            expected_raw |= {'native/complete.json', 'native/child-stderr.bin',
                             'observation.json', 'ordinary-comparison.json'}
            expected_raw |= {'native/%s-%d.%s' % (stem, i, suffix) for i in range(4)
                for stem, suffix in (('request', 'json'), ('control', 'bin'), ('observation', 'bin'))}
            require(len(expected_raw) == 78 and set(raw_files) == expected_raw,
                    'closed seventy-eight-body successful raw evidence')
    except BaseException as error:
        errors.append('raw reconciliation: ' + repr(error))
    spawn_pin = raw_files.get('parent/started.json')
    spawned = False
    if spawn_pin is not None:
        try:
            spawn_raw, actual_spawn_pin = audit.read(spawn_pin['path'], track=False)
            require(actual_spawn_pin == spawn_pin, 'native started body changed')
            spawn_doc = audit.parse(spawn_raw)
            identity = spawn_doc['parent']
            require(type(identity['pid']) is int and identity['pid'] > 0
                    and identity['pid'] == identity['pgid'] == identity['sid']
                    and identity['uid'] == 9661 and identity['ppid'] == spawn_doc['supervisor_pid']
                    and spawn_doc['command_sha256'] == raw_files['parent/command.json']['sha256'],
                    'authenticated native started identity')
            spawned = True
        except BaseException as error:
            errors.append('native started evidence: ' + repr(error))
    result = dict(schema='ferric-guarded-mlp-reusable-ar4-gpu-v1',
        passed=not errors and checked is not None and payload_comparison is not None,
        errors=errors, mode=mode, native_attempts=attempts, retries=0, plan=plan_pin, admission=admission,
        controller=controller_pin, phases=phases, parser_cpu=PARSER_CPU, tf4_data_revalidation=revalidation_pin,
        platform=initial, dependencies=dependencies, readset=audit.INPUTS, raw=raw_files, observation=checked, census_cpu=census_cpu,
        reusable_arena_requested=True, actual_arena_plateau_verified=checked is not None,
        actual_allocation_counts=checked['actual_arena_census']['allocation_counts'] if checked is not None else None,
        host_observation_requested=False, shared_full_currentness_requested=False,
        default_full_currentness_requested=True, performance_policy_changed=False,
        timing_comparison_performed=False, speedup_claim=False,
        ordinary_ar4_comparison=payload_comparison, baseline_payload_equality_required=True,
        limits=dict(native_seconds=NATIVE_SECONDS, whole_seconds=CASE_SECONDS, address_space_bytes=32 << 30,
                    stream_bytes=STREAM_CAP, case_bytes=CASE_CAP, case_members=256, affinity=[8, 9]),
        elapsed_seconds=time.monotonic() - start,
        native_spawn_observed=spawned, native_started=spawn_pin, gpu_execution_requested=attempts == 1,
        gpu_execution=checked is not None, gpu_execution_confirmed=checked is not None,
        partial_gpu_execution_possible=spawned and checked is None,
        full_model_acceptance=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False, full_long_workload=False, independent_full_model_reference=False,
        model_source_authenticated_by_qualified_parent=checked is not None, outer_model_shards_rehashed=False)
    save(out / ('complete.json' if result['passed'] else 'failed.json'), result)
    for number, handler in handlers.items():
        signal.signal(number, handler)
    print(json.dumps({k: result[k] for k in ('passed', 'errors', 'mode', 'native_attempts')}, sort_keys=True))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
