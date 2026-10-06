"""One explicit guarded AR4 stage capture, after healthy Close; unchanged owned cleanup."""
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
ROOT = E / 'guarded-mlp-model-stage-capture-gpu-v228-v1'
PLAN_SHA = {
    'ar4': None,
}
TF4_REVALIDATION_SHA = 'd0551f310a57f63dfe98af8c887b5996752e3b62128a3087c4a03849082e6a3f'
ANNOUNCEMENT_SHA = '96f84389071ef2c1a9e354705ebb7144e9ff97e30c242bd9b494f03d2aea6b80'
PARSER_CPU = dict(path=str(E / 'guarded-mlp-model-announcement-cpu-v228-v1/evidence/complete.json'),
    bytes=8872, sha256='170ba440cdf33cd88fc55c2ddff709624fd819d3d461d54daad239a719efdf5a')
VALIDATOR_SHA = '367d1904f15741138db708c1f10e5a777905b62d9440a961e952a6cc9125b055'
OWNED_SHA = 'ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'
AUDIT_SHA = 'b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'
IDS = [16366993098680759275, 10838076764495710945]
NATIVE_SECONDS, CASE_SECONDS, AUDIT_SECONDS = 4000, 4300, 30
STREAM_CAP, CASE_CAP = 8 << 20, 64 << 20
SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)
ENV = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
           OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1',
           HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
WORKER = dict(bytes=5785096, sha256='c747212d88ef54532194792bdcc2d2dbe35610abdd54d39ce2affe6ec57653e7')
WORKER_CPU = dict(bytes=1607204, sha256='64855967eb15f5b5b5fadb98cbc30e8e382968099ec439090562cacc174eec47')
PARENT = None
PARENT_CPU = None
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
            'actual capture CPU/ELF bindings remain pending')
    worker = doc(plan['worker_cpu']); parent = doc(plan['parent_cpu'])
    require({k: plan['worker_cpu'][k] for k in ('bytes', 'sha256')} == WORKER_CPU
            and {k: plan['worker'][k] for k in ('bytes', 'sha256')} == WORKER
            and {k: plan['parent_cpu'][k] for k in ('bytes', 'sha256')} == PARENT_CPU
            and {k: plan['parent'][k] for k in ('bytes', 'sha256')} == PARENT,
            'actual capture CPU/ELF pins')
    for value, schema in [(worker, 'ferric-guarded-mlp-stage-capture-worker-cpu-v1'),
                          (parent, 'ferric-guarded-mlp-stage-capture-parent-cpu-v1')]:
        require(value['schema'] == schema and value['passed'] is True and value['failure'] is None
                and value['postcheck_errors'] == [] and value['source_unchanged'] is True
                and value['input_sources'] == value['final_sources']
                and value['gpu_execution'] is False and value['capture_source_added'] is True
                and value['capture_native_execution'] is False, 'qualified capture CPU terminal')
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
            and len(worker['phases']) == 9
            and worker['full_worker_tests_executed'] is True
            and worker['tests']['worker-tests']['passed'] == 591
            and worker['tests']['worker-tests']['ignored'] == 4, 'capture worker qualification census')
    require(len(parent['phases']) == 54 and len(parent['tests']) == 46 and len(parent['artifacts']) == 5
            and sum(row['passed'] for row in parent['tests'].values()) == 380
            and all(row['failed'] == row['ignored'] == 0 for row in parent['tests'].values())
            and parent['all_selected_parent_tests_executed'] is True
            and parent['full_parent_library_suite_executed'] is False, 'selected capture parent qualification census')
    require(worker['artifacts']['worker']['pin'] == plan['worker'], 'qualified worker Cargo ELF')
    artifact = parent['artifacts']['ferric-qwen3-finite-guarded-mlp-decode-engineering']
    require(artifact['pin'] == plan['parent'], 'qualified parent Cargo ELF')
    for key in ('parent', 'worker'):
        raw, _ = audit.read(plan[key]['path'], plan[key], limit=128 << 20)
        require(raw[:6] == b'\x7fELF\x02\x01' and raw[18:20] == b'\x3e\x00', 'qualified x86_64 ELF')
    return dict(parent_cpu=plan['parent_cpu'], worker_cpu=plan['worker_cpu'], parent=plan['parent'], worker=plan['worker'])


def capture_admission(summary_raw, read_body, validator):
    """Additional capture checks after the ordinary model validator accepts this summary."""
    summary = validator.parse(summary_raw)
    pin = validator.rust_pin(summary['files']['child_stderr'])
    raw = read_body(pin)
    require(0 < len(raw) <= 1100000 and len(raw) == pin['bytes']
            and hashlib.sha256(raw).hexdigest() == pin['sha256'], 'nonempty exact bounded capture stderr')
    envelope = validator.parse(raw)
    validator.keys(envelope, 'schema profile_sha256 registration_sha256 session device_ids '
                   'completed_forwards native_closed sampling capture numerical_acceptance '
                   'performance_claim production_authority')
    require(envelope['schema'] == 'FerricFiniteGuardedMlpLayerZeroCaptureV1'
            and validator.octets(envelope['profile_sha256']) == validator.octets(summary['profile_sha256'])
            and validator.octets(envelope['registration_sha256']) == validator.octets(summary['registration_sha256'])
            and validator.octets(envelope['session']) == validator.octets(summary['request']['decode']['session'])
            and envelope['device_ids'] == IDS and all(type(v) is int for v in envelope['device_ids'])
            and validator.uint(envelope['completed_forwards']) == 4 and envelope['native_closed'] is True
            and envelope['sampling'] == 'prefix-boundaries-and-post-paired-retained'
            and all(envelope[k] is False for k in ('numerical_acceptance', 'performance_claim', 'production_authority')),
            'closed capture identity, completed workload and nonclaims')
    capture = envelope['capture']
    validator.keys(capture, 'schema generation position layer parts payload_bytes payload_sha256 payload '
                   'full_cache_capture native_close_confirmed numerical_acceptance performance_claim production_authority')
    require(capture['schema'] == 'FerricFiniteLayerZeroCaptureV1'
            and [validator.uint(capture[k]) for k in ('generation', 'position', 'layer')] == [1, 0, 0]
            and type(capture['parts']) is list and len(capture['parts']) == 34
            and validator.uint(capture['payload_bytes']) == 256136
            and capture['native_close_confirmed'] is True
            and all(capture[k] is False for k in ('full_cache_capture', 'numerical_acceptance',
                                                'performance_claim', 'production_authority')),
            'exact capture layout and lifetime scope')
    payload = validator.octets(capture['payload'], 256136)
    require(hashlib.sha256(payload).digest() == validator.octets(capture['payload_sha256']), 'whole capture digest')
    first = summary['files']['frames'][0]
    request = validator.parse(read_body(validator.rust_pin(first['request'])))
    observation = read_body(validator.rust_pin(first['observation']))
    command = request['command']
    metadata = struct.pack('<145I', *command['cache_metadata'])
    rotary = struct.pack('<128I', *command['rotary_bits'])
    require(request['id'] == command['generation'] == 1 and command['cache_metadata'][0] == 0
            and len(observation) == validator.PAYLOAD_BYTES, 'already validated first-forward capture join')
    stages = (
        ('before_prefix', (('input', 'bf16', 8192), ('cache_metadata', 'u32', 580), ('rotary', 'f32', 512))),
        ('after_prefix', (('input_normalized', 'bf16', 8192), ('raw_qkv', 'bf16', 6144),
                          ('query', 'bf16', 4096), ('current_key', 'bf16', 1024), ('current_value', 'bf16', 1024),
                          ('attention', 'bf16', 4096), ('output_partial', 'f32', 16384))),
        ('after_first_residual', (('first_residual', 'bf16', 8192),)),
        ('after_mlp', (('post_normalized', 'bf16', 8192), ('gate', 'bf16', 12288), ('up', 'bf16', 12288),
                      ('activation', 'bf16', 12288), ('down_partial', 'f32', 16384))),
        ('after_final_residual', (('final_hidden', 'bf16', 8192),)),
    )
    offset = ordinal = 0
    for boundary, specs in stages:
        for rank in (0, 1):
            for role, scalar, size in specs:
                part = capture['parts'][ordinal]
                validator.keys(part, 'boundary rank role scalar elements offset bytes source_byte_offset sha256')
                width = 2 if scalar == 'bf16' else 4
                source_offset = command['cache_metadata'][1] * 16384 if role in ('current_key', 'current_value') else 0
                require((part['boundary'], validator.uint(part['rank']), part['role'], part['scalar'],
                         validator.uint(part['elements']), validator.uint(part['offset']), validator.uint(part['bytes']),
                         validator.uint(part['source_byte_offset']))
                        == (boundary, rank, role, scalar, size // width, offset, size, source_offset),
                        'closed ordered capture part')
                data = payload[offset:offset + size]
                require(hashlib.sha256(data).digest() == validator.octets(part['sha256']), 'capture part digest')
                if scalar in ('bf16', 'f32'):
                    mask = 0x7f80 if width == 2 else 0x7f800000
                    require(all(int.from_bytes(data[i:i + width], 'little') & mask != mask
                                for i in range(0, size, width)), 'finite captured scalar')
                if role == 'cache_metadata':
                    require(data == metadata, 'capture actual first metadata')
                elif role == 'rotary':
                    require(data == rotary, 'capture actual first rotary')
                elif role == 'final_hidden':
                    require(data == observation[:8192], 'capture actual first layer output')
                offset += size
                ordinal += 1
    require(ordinal == 34 and offset == len(payload), 'complete capture coverage')
    return dict(schema='ferric-guarded-mlp-model-stage-capture-checked-v1', source=pin,
                generation=1, position=0, layer=0, parts=34, payload_bytes=len(payload),
                payload_sha256=hashlib.sha256(payload).hexdigest(), native_close_confirmed=True,
                same_run_request_and_layer_output_joined=True,
                sampling='prefix-boundaries-and-post-paired-retained',
                independent_framework_comparison=False, numerical_acceptance=False,
                performance_claim=False, production_authority=False)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python required')
    require(len(sys.argv) == 2 and sys.argv[1] in PLAN_SHA, 'one explicit capture ar4 selector; no implicit model rerun')
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
    require(plan['schema'] == 'ferric-guarded-mlp-model-stage-capture-gpu-input-v1' and plan['mode'] == mode, 'case generation')
    out = ROOT / mode; require(not os.path.lexists(out), 'fresh one-shot case root')
    resources(owned, True)
    request_raw, _ = audit.read(plan['request']['path'], plan['request'], limit=16384)
    request = validator.parse(request_raw); config = request['decode']
    require(request['schema'] == 'FerricFiniteGuardedMlpDecodeRequestV1'
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
    out.mkdir(mode=0o700); phases = []; errors = []; checked = None; capture_checked = None; attempts = 0; dependencies = {}; initial = None
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
            '--capture-guarded-layer-zero', '--allow-unauthenticated-machine-code'], NATIVE_SECONDS)
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
        capture_checked = capture_admission(summary, read_body, validator)
        save(out / 'observation.json', checked)
        save(out / 'capture-observation.json', capture_checked)
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
    result = dict(schema='ferric-guarded-mlp-model-stage-capture-gpu-v1',
        passed=not errors and checked is not None and capture_checked is not None,
        errors=errors, mode=mode, native_attempts=attempts, retries=0, plan=plan_pin, admission=admission,
        controller=controller_pin, phases=phases, parser_cpu=PARSER_CPU, tf4_data_revalidation=revalidation_pin,
        platform=initial, dependencies=dependencies, readset=audit.INPUTS, raw=raw_files, observation=checked, capture_observation=capture_checked,
        capture_requested=True, capture_verified=capture_checked is not None,
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
