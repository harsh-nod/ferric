"""One owned, bounded Readiness40 Position5 SharedFull policy attempt; no full2303 or numerical acceptance."""
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
ROOT = E / 'guarded-mlp-readiness40-shared-full-gpu-v228-v1'
ORDER = ('readiness',)
PLAN_SHA = {'readiness': 'c3a0914e49f73eed73f9f4910a0198b4dd69fe3ec65a135cec7d843f95e59421'}
CHECKER_CPU = dict(path=str(E / 'guarded-mlp-readiness40-shared-full-checker-cpu-v228-v1/evidence/complete.json'), bytes=13382, sha256='7a9029ae0e864809fa98062f4f15a13c954ce67d8b1d20e8c2cb36baa888850c')
CHECKER_TESTS = 29
SHARED_SHA = '6557fe5c082b2c92bae15274dd0d19bba3da8c3c36b9fc74757b4c1b74a87eca'
ANNOUNCEMENT_SHA = 'b974ac6b6e936d8239639ac0c595c7db36700e3b1e2cff101224699fd789a9b1'
VALIDATOR_SHA = '0c319e99142b19909350d9a81404948b494c2e3ea0defbbc165c2e5529be032a'
BASELINE_ROOT = E / 'guarded-mlp-readiness40-position5-gpu-v228-v1'
BASELINE = dict(path=str(BASELINE_ROOT / 'readiness/complete.json'), bytes=171456,
    sha256='5b9617aba0b588c33923c5cc458b013d0929828be5635ea75f3fd702a12c2cd9')
OWNED_SHA = 'ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'
AUDIT_SHA = 'b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'
IDS = [16366993098680759275, 10838076764495710945]
NATIVE_SECONDS, CASE_SECONDS, AUDIT_SECONDS = 4000, 4300, 30
STREAM_CAP, CASE_CAP = 8 << 20, 64 << 20
SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)
ENV = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
           OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1',
           HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
WORKER = dict(bytes=6255224, sha256='9dbebbb014562f1bb704e8d504e46b8b719745abff5e0c7674af18412541bda8')
WORKER_CPU = dict(bytes=1746409, sha256='4a016b7e09b0cc6f9b4bd32c98a5713f24709bd6509564a589c29b0478f7e337')
PARENT = dict(bytes=13852264, sha256='682d5455b9c5966fa6a3ad02f0e755117b7d7e69241ead372ee22fbd703de804')
PARENT_CPU = dict(bytes=3983617, sha256='7e4ad4324ea3b0302e1f7cf738b699e0e0721025963e659f0ba7f5bc35d70a8d')
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
            'actual readiness CPU/ELF bindings remain pending')
    worker = doc(plan['worker_cpu']); parent = doc(plan['parent_cpu'])
    for name, expected in [('worker_cpu', WORKER_CPU), ('worker', WORKER),
                           ('parent_cpu', PARENT_CPU), ('parent', PARENT)]:
        require({k: plan[name][k] for k in ('bytes', 'sha256')} == expected, 'actual readiness CPU/ELF pin')
    for value, schema, phases, sources in [
            (worker, 'ferric-guarded-mlp-readiness40-shared-full-worker-cpu-v1', 9, 1022),
            (parent, 'ferric-guarded-mlp-readiness40-shared-full-parent-cpu-v1', 64, 1260)]:
        require(value['schema'] == schema and value['passed'] is True and value['failure'] is None
                and value['postcheck_errors'] == [] and value['source_unchanged'] is True
                and value['input_sources'] == value['final_sources'] and value['gpu_execution'] is False
                and len(value['phases']) == phases and len(value['final_sources']) == sources,
                'actual qualified readiness CPU terminal')
        for key in ('controller', 'input_manifest'):
            audit.read(value[key]['path'], value[key], retain=False)
        require(doc(value['raw']['sources-after.json']) == value['final_sources'], 'CPU final source body')
        for phase in value['phases']:
            require(type(phase['exit_code']) is int and phase['exit_code'] == 0
                    and phase['natural_exit'] is True and phase['reaped'] is True
                    and phase['process_group_absent'] is True and phase['forced_cleanup'] is False
                    and phase['timed_out'] is False and phase['exception'] is None
                    and phase['storage_failure'] is None, 'CPU leaf lifecycle')
    require(len(worker['inventory']) == 708 and worker['full_worker_tests_executed'] is True
            and worker['tests']['worker-tests']['passed'] == 704
            and worker['tests']['worker-tests']['failed'] == 0
            and worker['tests']['worker-tests']['ignored'] == 4
            and set(worker['artifacts']) == {'worker-lib', 'worker-bin-test', 'worker-readiness-test',
                                            'worker-wire-test', 'worker'}
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and all(worker[k] is True for k in ('readiness_cli_source_added',
                'readiness_owner_source_added', 'pure_long_sequence_source_added',
                'position5_diagnostic_source_added', 'inherited_readiness40_profile_preserved'))
            and all(worker[k] is False for k in ('readiness40_native_execution',
                'existing_ar4_limits_changed',
                'position5_native_execution', 'capture_count_changed', 'full2303_native_execution',
                'full2303_launch_feasibility'))
            and worker['full2303_native_enabled'] is True and worker['full2303_source_added'] is True
            and worker['full2303_source_abort_ms'] == 3600000,
            'full readiness worker census and genuine executable CLI qualification')
    require(len(parent['tests']) == 54 and len(parent['inventory']) == 946
            and sum(row['passed'] for row in parent['tests'].values()) == 475
            and all(row['failed'] == row['ignored'] == 0 for row in parent['tests'].values())
            and len(parent['artifacts']) == 7 and parent['all_selected_parent_tests_executed'] is True
            and parent['full_parent_library_suite_executed'] is False
            and parent['position5_diagnostic_parent_route_added'] is True
            and parent['position5_native_execution'] is False
            and parent['inherited_readiness_parent_route_preserved'] is True
            and parent['readiness_native_execution'] is False and parent['full_long_workload'] is False,
            'selected readiness parent census')
    require(worker['readiness40_shared_full_source_added'] is True
            and worker['readiness40_shared_full_native_execution'] is False
            and worker['selected_readiness_currentness_policy_changed'] is True
            and worker['inherited_full2303_route_preserved'] is True
            and parent['readiness40_shared_full_parent_route_added'] is True
            and parent['readiness40_shared_full_native_execution'] is False
            and parent['selected_readiness_currentness_policy_changed'] is True
            and parent['qualified_worker_sources_preserved'] is True
            and parent['ordinary_wire_schema_changed'] is False
            and parent['ordinary_observation_schema_changed'] is False,
            'qualified explicit shared-only readiness route')
    prefix = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
    compact = lambda value: {k: {n: v[n] for n in ('bytes', 'sha256')}
        for k, v in value['final_sources'].items() if k.startswith(prefix)}
    require(len(compact(worker)) == 205 and compact(worker) == compact(parent),
            'all205 parent-compiled worker bodies equal actual standalone worker')
    require(worker['artifacts']['worker']['pin'] == plan['worker'], 'qualified worker Cargo product')
    artifact = parent['artifacts']['ferric-qwen3-guarded-mlp-readiness-engineering']
    require(artifact['pin'] == plan['parent']
            and artifact['cargo_artifact']['target']['name'] == 'ferric-qwen3-guarded-mlp-readiness-engineering'
            and artifact['cargo_artifact']['target']['kind'] == ['bin']
            and artifact['cargo_artifact']['profile']['test'] is False, 'new qualified readiness parent product')
    for key in ('parent', 'worker'):
        raw, _ = audit.read(plan[key]['path'], plan[key], limit=128 << 20)
        require(raw[:6] == b'\x7fELF\x02\x01' and raw[18:20] == b'\x3e\x00', 'qualified x86_64 ELF')
    return dict(parent_cpu=plan['parent_cpu'], worker_cpu=plan['worker_cpu'], parent=plan['parent'], worker=plan['worker'])


def checker_admission(audit):
    require(type(CHECKER_CPU) is dict and set(CHECKER_CPU) == {'path', 'bytes', 'sha256'},
            'actual readiness data-checker CPU binding remains pending')
    value = audit.parse(audit.read(CHECKER_CPU['path'], CHECKER_CPU)[0])
    require(value['schema'] == 'ferric-guarded-mlp-readiness40-shared-full-checker-cpu-v1'
            and value['passed'] is True and value['failure'] is None
            and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['sources_before'] == value['sources_after']
            and value['tests']['passed'] == CHECKER_TESTS
            and value['tests']['failed'] == value['tests']['errors'] == value['tests']['skipped'] == 0
            and value['sources_before']['validate_readiness.py']['sha256'] == VALIDATOR_SHA
            and value['sources_before']['readiness_announcement.py']['sha256'] == ANNOUNCEMENT_SHA
            and value['sources_before']['validate_shared.py']['sha256'] == SHARED_SHA,
            'actual independently exercised readiness checker and exact marker')
    return CHECKER_CPU


def baseline_admission(audit, validator, guard, tokens):
    raw, _ = audit.read(BASELINE['path'], BASELINE)
    old = audit.parse(raw)
    require(old['schema'] == 'ferric-guarded-mlp-readiness40-position5-gpu-v1'
            and old['passed'] is True and old['errors'] == [] and old['native_attempts'] == 1
            and old['retries'] == 0 and old['owned_worker_lineage_verified'] is True
            and old['default_full_currentness_requested'] is True
            and old['shared_full_currentness_requested'] is False and old['paired_read_requested'] is False
            and all(old[k] is False for k in ('numerical_acceptance', 'performance_claim', 'production_authority',
                'full_model_acceptance', 'full_long_workload')), 'actual successful original position5 baseline')
    labels = ['parent-readelf', 'parent-ldd', 'worker-readelf', 'worker-ldd']
    labels += ['before-' + str(i) for i in range(3)] + ['parent'] + ['after-' + str(i) for i in range(3)]
    names = {label + '/' + name for label in labels
             for name in ('command.json', 'started.json', 'stdout', 'stderr', 'result.json')}
    names |= {'initial-topology.json', 'observation.json'}
    names |= {side + '-' + str(i) + '-topology.json' for side in ('before', 'after') for i in range(3)}
    names |= {'native/complete.json', 'native/frames.ndjson', 'native/child-stderr.bin'}
    names |= {'native/capture-%d.bin' % i for i in (0, 5, 16, 39)}
    require(set(old['raw']) == names and len(names) == 70
            and [row['label'] for row in old['phases']] == labels, 'closed original baseline raw/phases')
    bodies = {}
    for name, pin in old['raw'].items():
        require(pin['path'] == str(BASELINE_ROOT / 'readiness' / name), 'exact original baseline path')
        bodies[name] = audit.read(pin['path'], pin, limit=8 << 20)[0]
    for row in old['phases']:
        successful(row)
        label = row['label']
        require(audit.parse(bodies[label + '/result.json']) == {k: v for k, v in row.items() if k != 'label'},
                'baseline original owned result join')
        for key, suffix in [('command', 'command.json'), ('started', 'started.json'), ('stdout', 'stdout'), ('stderr', 'stderr')]:
            require(row[key] == old['raw'][label + '/' + suffix], 'baseline raw phase pins')
        if label.startswith(('before-', 'after-')):
            audit.idle(bodies[label + '/stdout'])
            snapshot = audit.parse(bodies[label + '-topology.json'])
            validate_topology(snapshot)
            require(snapshot == old['platform'], 'baseline immutable idle topology')
    require(audit.parse(bodies['initial-topology.json']) == old['platform'], 'baseline initial topology')
    plan = audit.parse(audit.read(old['plan']['path'], old['plan'])[0])
    require(old['plan']['path'] == str(BASELINE_ROOT / 'readiness-input.json')
            and plan['request']['path'] == str(BASELINE_ROOT / 'readiness-request.json'), 'baseline input paths')
    request = audit.parse(audit.read(plan['request']['path'], plan['request'])[0])
    require(validator.same(validator.parse(bodies['parent/stdout']), validator.parse(bodies['native/complete.json'])),
            'baseline parent stdout and original summary')
    def read_body(pin):
        return audit.read(pin['path'], pin, limit=2 << 20)[0]
    checked = validator.validate(bodies['native/complete.json'], request,
                                 BASELINE_ROOT / 'readiness/native', read_body, tokens)
    require(validator.same(checked, old['observation'])
            and validator.same(checked, audit.parse(bodies['observation.json'])), 'baseline independently repeated admission')
    native = old['phases'][7]
    guard.validate_lineage(bodies['parent/stderr'], native, audit.parse(bodies['parent/started.json']), checked['child_pid'])
    return bodies['native/complete.json'], checked


def case_deadline(deadline):
    remaining = deadline - time.monotonic()
    require(remaining > 0, 'whole case deadline')
    signal.signal(signal.SIGALRM, interrupted)
    signal.setitimer(signal.ITIMER_REAL, remaining)


def run_case(case):
    start = time.monotonic()
    handlers = {number: signal.getsignal(number) for number in SIGNALS}
    timer, interval = signal.getitimer(signal.ITIMER_REAL)
    try:
        for number in SIGNALS:
            signal.signal(number, interrupted)
        case_deadline(start + CASE_SECONDS)
        return _run_case(case, start, start + CASE_SECONDS)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        for number, handler in handlers.items():
            signal.signal(number, handler)
        if timer:
            signal.setitimer(signal.ITIMER_REAL, max(0.001, timer - (time.monotonic() - start)), interval)


def _run_case(case, start, deadline):
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python required')
    require(type(case) is str and case in ORDER, 'one explicit readiness case; no implicit rerun')
    directory = Path(__file__).resolve().parent
    require(directory == ROOT and os.getuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged deployment host/root')
    require(type(PLAN_SHA[case]) is str and re.fullmatch('[0-9a-f]{64}', PLAN_SHA[case]), 'actual case input remains unbound')
    audit = load(directory / 'library_audit.py', AUDIT_SHA, 'guarded_model_audit')
    owned = load(directory / 'frozen_owned.py', OWNED_SHA, 'guarded_model_owned')
    validator = load(directory / 'validate_readiness.py', VALIDATOR_SHA, 'guarded_model_validation')
    prior_module = sys.modules.get('validate_readiness')
    sys.modules['validate_readiness'] = validator
    try:
        shared = load(directory / 'validate_shared.py', SHARED_SHA, 'readiness_shared_full_validation')
    finally:
        if prior_module is None: sys.modules.pop('validate_readiness', None)
        else: sys.modules['validate_readiness'] = prior_module
    guard = load(directory / 'readiness_announcement.py', ANNOUNCEMENT_SHA, 'guarded_model_announcement')
    audit.HARD_DEADLINE = deadline
    plan_raw, plan_pin = audit.read(directory / (case + '-input.json'), limit=65536)
    require(plan_pin['sha256'] == PLAN_SHA[case], 'exact case input')
    plan = audit.parse(plan_raw)
    validator.keys(plan, 'schema case profile parent_cpu worker_cpu parent worker request')
    require(plan['schema'] == 'ferric-guarded-mlp-readiness40-shared-full-gpu-input-v1'
            and plan['case'] == case and plan['profile'] == 'readiness40_position5', 'closed readiness profile')
    out = ROOT / case; require(not os.path.lexists(out), 'fresh one-shot case root')
    resources(owned, True)
    request_raw, _ = audit.read(plan['request']['path'], plan['request'], limit=16384)
    request = validator.parse(request_raw); config = request['base']
    require(request['schema'] == 'FerricGuardedMlpReadiness40Position5RequestV1'
            and config['schema'] == 'FerricFiniteLongRequestV1'
            and config['device_ids'] == IDS and config['child_deadline_ms'] == 3600000
            and config['dispatch_timeout_ms'] == 10000 and config['evidence_directory'] == str(out / 'native')
            and validator.rust_pin(config['worker']) == plan['worker'], 'closed guarded request')
    for name, extent in SELECTED_IMAGES.items():
        value = request[name]
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
    checker_cpu = checker_admission(audit)
    prompt_pins = {
        'manifest': (21318, '30d047aafbd7de2b94647154680b0b9d9fb75aae2fd90b081afe0de595a88600'),
        'text': (11224, 'a43ef3619cb96c9c23d020e07630493ced7a588088033a53f1d612448375751a'),
        'tokens': (8192, '2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02')}
    prompt_bodies = {}
    require(set(config['prompt']) == set(prompt_pins), 'complete original prompt body roster')
    for name, expected in prompt_pins.items():
        pin = validator.rust_pin(config['prompt'][name])
        require((pin['bytes'], pin['sha256']) == expected, 'authentic2048 prompt pin')
        prompt_bodies[name] = audit.read(pin['path'], pin)[0]
    tokens = list(struct.unpack('<2048I', prompt_bodies['tokens']))
    require(tokens == validator.parse(prompt_bodies['manifest'])['input_token_ids']
            and all(type(v) is int and 0 <= v < 151936 for v in tokens), 'all2048 authentic prompt tokens')
    baseline_raw, baseline_checked = baseline_admission(audit, validator, guard, tokens)
    for path in (directory / 'run_model_gpu.py', directory / 'library_audit.py', directory / 'frozen_owned.py',
                 directory / 'validate_readiness.py', directory / 'validate_shared.py', directory / 'readiness_announcement.py', Path('/opt/rocm/bin/amd-smi'),
                 Path('/usr/bin/readelf'), Path('/usr/bin/ldd'), Path(sys.executable)):
        audit.resolved_input(path)
    controller_pin = audit.INPUTS[str(directory / 'run_model_gpu.py')]
    out.mkdir(mode=0o700); phases = []; errors = []; checked = None; shared_checked = None; parity = None; lineage_checked = False; attempts = 0; dependencies = {}; initial = None
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
            '--observe-guarded-readiness40-position5-shared-full',
            '--allow-unauthenticated-machine-code'], NATIVE_SECONDS)
        summary_path = out / 'native/complete.json'
        summary, summary_pin = audit.read(summary_path, limit=128 << 10, track=False)
        expected = {'complete.json', 'frames.ndjson', 'child-stderr.bin'} | {
            'capture-%d.bin' % i for i in (0, 5, 16, 39)}
        require({p.name for p in summary_path.parent.iterdir()} == expected, 'exact seven native bodies')
        def read_body(pin):
            return audit.read(pin['path'], pin, limit=2 << 20, track=False)[0]
        checked, shared_checked = shared.validate(raw, summary, request, summary_path.parent, read_body, tokens)
        def parity_read(pin):
            return audit.read(pin['path'], pin, limit=2 << 20,
                track=not Path(pin['path']).is_relative_to(out))[0]
        parity = shared.compare_same_side(summary, baseline_raw, checked, baseline_checked, parity_read)
        save(out / 'parity.json', parity)
        native = phases[-1]
        start_doc = audit.parse(audit.read(out / 'parent/started.json', track=False)[0])
        guard.validate_lineage(audit.read(out / 'parent/stderr', track=False)[0],
                               native, start_doc, checked['child_pid'])
        lineage_checked = True
        save(out / 'observation.json', checked)
    except BaseException as error:
        errors.append(type(error).__name__ + ': ' + str(error))
    finally:
        for number in handlers:
            if number != signal.SIGALRM:
                signal.signal(number, signal.SIG_IGN)
        case_deadline(deadline)
        for index in range(3):
            try:
                case_deadline(deadline)
                idle('after-' + str(index), True)
            except BaseException as error: errors.append('postflight: ' + repr(error))
            if index < 2: time.sleep(.2)
        try:
            require(time.monotonic() < deadline, 'whole case deadline')
            require(not any(r['ppid'] == os.getpid() for r in owned.processes().values()), 'all owned children absent')
            for path, pin in list(audit.INPUTS.items()):
                case_deadline(deadline)
                audit.read(path, pin, retain=False)
            for alias, canonical in audit.ALIASES.items():
                require(str(Path(alias).resolve(strict=True)) == canonical, 'input/library alias drift')
            resources(owned); inventory(out)
        except BaseException as error:
            errors.append('postcheck: ' + repr(error))
    raw_files = {}
    for path in sorted(out.rglob('*')):
        case_deadline(deadline)
        if path.is_file():
            raw_files[str(path.relative_to(out))] = audit.read(path, retain=False, track=False)[1]
    try:
        for phase in phases:
            case_deadline(deadline)
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
            raw_names = {label + '/' + name for label in expected
                         for name in ('command.json', 'started.json', 'stdout', 'stderr', 'result.json')}
            raw_names |= {'initial-topology.json'} | {side + '-' + str(i) + '-topology.json'
                         for side in ('before', 'after') for i in range(3)}
            raw_names |= {'native/complete.json', 'native/frames.ndjson', 'native/child-stderr.bin'}
            raw_names |= {'native/capture-%d.bin' % i for i in (0, 5, 16, 39)}
            raw_names |= {'observation.json', 'parity.json'}
            require(set(raw_files) == raw_names and len(raw_files) == 71, 'closed successful71 raw bodies')
    except BaseException as error:
        errors.append('raw reconciliation: ' + repr(error))
    spawn_pin = raw_files.get('parent/started.json')
    spawned = False
    if spawn_pin is not None:
        try:
            case_deadline(deadline)
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
    result = dict(schema='ferric-guarded-mlp-readiness40-shared-full-gpu-v1',
        passed=not errors and checked is not None and shared_checked is not None and parity is not None and lineage_checked,
        errors=errors, case=case, profile='readiness40_position5', native_attempts=attempts, retries=0,
        plan=plan_pin, admission=admission, controller=controller_pin, phases=phases, checker_cpu=checker_cpu,
        platform=initial, dependencies=dependencies, readset=audit.INPUTS, raw=raw_files, observation=checked,
        shared_policy=shared_checked, same_side_parity=parity, baseline=BASELINE,
        parent_host_timing_requested=False, gpu_timing=False, host_observer_requested=False,
        paired_terminal_requested=False, cache_kernel_admission_requested=False, operational_currentness_requested=False,
        artifact_contains_full2303_route=True,
        owned_worker_lineage_verified=lineage_checked, prompt_tokens=2048,
        prompt_positions_requested=40, generated_tokens_requested=0, capture_positions=[0, 5, 16, 39],
        allocation_pages=144, default_full_currentness_requested=False,
        shared_full_currentness_requested=True, paired_read_requested=False,
        limits=dict(native_seconds=NATIVE_SECONDS, whole_seconds=CASE_SECONDS, address_space_bytes=32 << 30,
                    stream_bytes=STREAM_CAP, case_bytes=CASE_CAP, case_members=256, affinity=[8, 9]),
        elapsed_seconds=time.monotonic() - start,
        native_spawn_observed=spawned, native_started=spawn_pin, gpu_execution_requested=attempts == 1,
        gpu_execution=checked is not None, gpu_execution_confirmed=checked is not None,
        partial_gpu_execution_possible=spawned and checked is None,
        full_model_acceptance=False, numerical_acceptance=False, performance_claim=False,
        production_authority=False, full_long_workload=False, full2303_native_enabled=False,
        independent_full_model_reference=False, ar4_parity_transferred=False,
        model_source_authenticated_by_qualified_parent=checked is not None, outer_model_shards_rehashed=False)
    case_deadline(deadline)
    save(out / ('complete.json' if result['passed'] else 'failed.json'), result)
    for number, handler in handlers.items():
        signal.signal(number, handler)
    print(json.dumps({k: result[k] for k in ('passed', 'errors', 'case', 'native_attempts')}, sort_keys=True))
    return 0 if result['passed'] else 1


def main():
    require(len(sys.argv) == 2, 'run_model_gpu.py EXPLICIT_CASE')
    return run_case(sys.argv[1])


if __name__ == '__main__':
    raise SystemExit(main())
