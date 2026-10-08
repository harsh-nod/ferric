"""One three-record Tail diagnostic case; no matched performance or numerical authority."""
import functools
import hashlib
import json
import math
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
ROOT = E / 'guarded-mlp-readiness40-tail-layer-duration-gpu-v228-v1'
WORKER_ROOT = E / 'guarded-mlp-layer-phases-worker-diagnostic-cpu-v228-v1'
PARENT_ROOT = E / 'guarded-mlp-layer-phases-parent-diagnostic-cpu-v228-v1'
RUNTIME_ROOT = E / 'guarded-mlp-layer-phases-runtime-diagnostic-cpu-v228-v3'
ORDER = ('tail_layer',)
PLAN_SHA = {'tail_layer': None}
CHECKER_CPU = {"path":"/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-tail-layer-duration-checker-cpu-v228-v1/evidence/complete.json","bytes":68280,"sha256":"bff2af1ddf5bfe3247e69f48595c51bf062c9dbc09af1bddd94a9384ba6026aa"}
CHECKER_TESTS = 178
CHECKER_CONTROLLER = {"bytes":11227,"sha256":"b4a0473197e742f41320149145527d39d16d43485e11de637aaf367989a332f6"}
CHECKER_SOURCES = {"readiness_announcement.py":{"bytes":3246,"sha256":"b974ac6b6e936d8239639ac0c595c7db36700e3b1e2cff101224699fd789a9b1"},"test_bank_scoped.py":{"bytes":14853,"sha256":"3ecc622e660cb6e0790a18ff8bb022a84535c91466d02d6fdbb8625ae080a99b"},"test_census.py":{"bytes":19688,"sha256":"aca29934c68523dbc91b30439a2c7a5ae552b64a9faf077ddb99b97c861cb5b0"},"test_duration.py":{"bytes":13609,"sha256":"3f88f035e3f08644dadf393620557b84d7827876fb3e992e498fd6e2950d3fb5"},"test_forward.py":{"bytes":20872,"sha256":"e39afc63a07b5b3b43cffafabb3f5775eeff7794cf97a4edba1fbb630466b735"},"test_layer.py":{"bytes":33750,"sha256":"39ae7e7555a13eb192a0a6eacfee42fd1bdcadca1e849a3afd8ebf255843151e"},"test_matched.py":{"bytes":15400,"sha256":"d5a6b522a1effe29e1ee08430754163cc217cf34c3713ce234926b7c860f9647"},"test_readiness.py":{"bytes":17802,"sha256":"4b644462a71120ee45b3351757ecb0d5867c9453daadb6a10c07bdbcd38e8bb5"},"test_scoped.py":{"bytes":15936,"sha256":"4c015cdfa072943960e0ffa1f25e0e604790e8dbe222875b4bed207be6563fdc"},"test_scoped_pair.py":{"bytes":3629,"sha256":"aa95ff79fd3c72ee3658d66e4117738a06f7984a2a77dcc6712b28a272e57f8f"},"test_shared.py":{"bytes":10737,"sha256":"a122f42f8a47a0213010f1812ef1f95fa7ff582db6a9ffa190356a35874f6b96"},"test_tail.py":{"bytes":19524,"sha256":"2235e1c4b97b396094df484ccaedb25619f70c757ed1656e5eaf6b92f8140c4a"},"test_timing.py":{"bytes":10433,"sha256":"630febf4d840081887692144cc570d70c1d67f9ad3bb5bab1fd6d7d1f61cedb1"},"validate_bank_pair.py":{"bytes":4428,"sha256":"9d3f1470f6e9d7a17f3c91d1deb29a54c6fa7e8c766dc358e9d7d73e15c4e004"},"validate_bank_scoped.py":{"bytes":9534,"sha256":"2432fc14c0ba40fc934abcd7886459c12fc31ca6167ba4cf908a8c7974d26772"},"validate_census.py":{"bytes":10392,"sha256":"090431e59a481d5b681311d5a015ddfd35705cc828c920118d4f0043b862ec9f"},"validate_census_pair.py":{"bytes":4730,"sha256":"d4f50afe412b68c4066e934a80b96b5a5c0f83ef8ea33c59c53380aa1ab819c4"},"validate_duration.py":{"bytes":9343,"sha256":"d7ed85d14191e735490170a606a06cefb331e17b07c36a4a4d34ef38a5cf19bd"},"validate_forward.py":{"bytes":9585,"sha256":"2a238ab236d86be4762738df0b70aff603794a9198e2b214580a56184986a89b"},"validate_layer.py":{"bytes":12027,"sha256":"337021cc59c91745a2d12708fd71c87d319e11eae325dceb5942a4e99e40d6c1"},"validate_matched.py":{"bytes":9362,"sha256":"d31c94a4dd1f254d166be185fe1cca10bc4eed26e500a0cb1fc3c5a74a574437"},"validate_readiness.py":{"bytes":19800,"sha256":"0c319e99142b19909350d9a81404948b494c2e3ea0defbbc165c2e5529be032a"},"validate_scoped.py":{"bytes":9184,"sha256":"c2541e11e534cf1e5a4faade31fe4a734726552171fed9617d6ea938f680b6fd"},"validate_scoped_pair.py":{"bytes":3233,"sha256":"495fc476454e4052467810b55e177f9d2b91d86d3496a6b90876963f7f60ac62"},"validate_shared.py":{"bytes":8015,"sha256":"6557fe5c082b2c92bae15274dd0d19bba3da8c3c36b9fc74757b4c1b74a87eca"},"validate_tail.py":{"bytes":10447,"sha256":"696e55d4bebea97ab82e5b613a76978c5e191087789d299551c304527e598d49"},"validate_tail_pair.py":{"bytes":4883,"sha256":"ab69bcafcc245b8eca05d7e625c42eec7aaec2de4084e719090de1894f4d6995"},"validate_timing.py":{"bytes":10104,"sha256":"27c652f5452676b4cc08a634241aff89b2653e85ee111133c1f1c4ab13efd93f"}}
LAYER_SHA = "337021cc59c91745a2d12708fd71c87d319e11eae325dceb5942a4e99e40d6c1"
FORWARD_SHA = '2a238ab236d86be4762738df0b70aff603794a9198e2b214580a56184986a89b'
CPU_ADMISSION_SHA = "f1105fbef6aad492ef5f32bb7613a4f609134df9395cb1b2416c24e845109d28"
SCOPED_SHA = 'c2541e11e534cf1e5a4faade31fe4a734726552171fed9617d6ea938f680b6fd'
BANK_SHA = '2432fc14c0ba40fc934abcd7886459c12fc31ca6167ba4cf908a8c7974d26772'
DURATION_SHA = 'd7ed85d14191e735490170a606a06cefb331e17b07c36a4a4d34ef38a5cf19bd'
CENSUS_SHA = '090431e59a481d5b681311d5a015ddfd35705cc828c920118d4f0043b862ec9f'
TAIL_SHA = '696e55d4bebea97ab82e5b613a76978c5e191087789d299551c304527e598d49'
TIMING_SHA = '27c652f5452676b4cc08a634241aff89b2653e85ee111133c1f1c4ab13efd93f'
MATCHED_SHA = 'd31c94a4dd1f254d166be185fe1cca10bc4eed26e500a0cb1fc3c5a74a574437'
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
RUNTIME_CPU = {"bytes":3301993,"sha256":"5d1d0d6ac7af9cf94a5883486d41f61812238ac1ae593f26830e3113c1673d13"}
RUNTIME_SOURCES = {"bytes":876134,"sha256":"4a9a3370f55ce81e7ce4e8367f38922a4fd69e274f579349f5f66564c4f3a1e1"}
WORKER = {"bytes":6683640,"sha256":"a907e189118a35842f45c8c5f0bb43cc37c540e862d274386184ab4ce85af784"}
WORKER_CPU = {"bytes":3090092,"sha256":"b5a0c018ed45b739b535901831e6914a68c7c17dee202a4c67077075cea10c7d"}
PARENT = {"bytes":14397976,"sha256":"84fb1d82a7e598e393d7a02642b6f001c4ac85222da621c4d2a50b3be1ad74d2"}
PARENT_CPU = {"bytes":7627326,"sha256":"22cf285a8c3caef27c366424c817e0126d1df80dccc7dea36814f01281744078"}
WORKER_SOURCES = {"bytes":873993,"sha256":"767da06dcaeb617b1f4555b354b4125ea34f0ad315d1ad43aa3096e3d381326b"}
PARENT_SOURCES = {"bytes":873993,"sha256":"30a55c3955c7d43833652c9c7cf4c10ce4b6f1589b7573da815340a9cf7651c2"}
SELECTED_IMAGES = {
    'projection_image': (10864, '25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25'),
    'guarded_image': (28440, 'de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66'),
    'prefix_image': (54344, '29fd58e7b09fee003ed31660e6201c20a7eb5da4b873d0b4d954993f7018f2f8'),
    'tiles_image': (33320, 'b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589'),
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def children_rusage_delta(before, after):
    fields = ('ru_utime', 'ru_stime', 'ru_nvcsw', 'ru_nivcsw', 'ru_minflt', 'ru_majflt')
    for row in (before, after):
        require(type(row) is dict and set(row) == set(fields), 'closed child CPU usage snapshot')
        for name in fields:
            value = row[name]
            require((type(value) is float and math.isfinite(value) and value >= 0)
                    if name in fields[:2] else (type(value) is int and value >= 0),
                    'finite nonnegative exact child CPU usage type: ' + name)
    require(all(after[name] >= before[name] for name in fields), 'monotone child CPU usage')
    return {name: after[name] - before[name] for name in fields}


def validate_children_rusage(usage, complete=True):
    require(type(usage) is dict and set(usage) == {'who', 'before', 'after', 'delta', 'error'}
            and usage['who'] == 'RUSAGE_CHILDREN', 'closed child CPU usage record')
    if usage['error'] is None:
        expected = children_rusage_delta(usage['before'], usage['after'])
        require(type(usage['delta']) is dict and set(usage['delta']) == set(expected)
                and all(type(usage['delta'][name]) is type(value) and usage['delta'][name] == value
                        for name, value in expected.items()), 'exact child CPU usage delta')
    else:
        require(not complete and type(usage['error']) is str and 0 < len(usage['error']) <= 1024
                and usage['delta'] is None, 'explicit failed child CPU usage sample')
        require(usage['before'] is not None or usage['after'] is None,
                'child CPU usage after without before')
        for row in (usage['before'], usage['after']):
            if row is not None:
                children_rusage_delta(row, row)


def children_rusage_snapshot():
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    row = {name: getattr(usage, name) for name in
           ('ru_utime', 'ru_stime', 'ru_nvcsw', 'ru_nivcsw', 'ru_minflt', 'ru_majflt')}
    children_rusage_delta(row, row)
    return row


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
    usage = dict(who='RUSAGE_CHILDREN', before=None, after=None, delta=None, error=None)
    handlers = {n: signal.getsignal(n) for n in SIGNALS}
    try:
        for n in handlers:
            signal.signal(n, interrupted)
        with (directory / 'stdout').open('xb') as stdout, (directory / 'stderr').open('xb') as stderr:
            if seconds == NATIVE_SECONDS:
                try:
                    usage['before'] = children_rusage_snapshot()
                except BaseException as error:
                    usage['error'] = ('before child CPU usage: ' + repr(error))[:1024]
                    raise
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
    if seconds == NATIVE_SECONDS:
        # The subreaper has finished waiting for the owned tree before the second sample.
        if child is not None:
            try:
                usage['after'] = children_rusage_snapshot()
                require(outcome['owned_groups_absent'] is True and outcome['owned_processes_reaped'] is True,
                        'child CPU usage requires complete owned retirement')
                usage['delta'] = children_rusage_delta(usage['before'], usage['after'])
                validate_children_rusage(usage)
            except BaseException as error:
                usage['delta'] = None
                usage['error'] = ('after child CPU usage: ' + repr(error))[:1024]
                reason = reason or usage['error']
        elif usage['error'] is None:
            usage['error'] = 'native parent was not spawned'
            reason = reason or usage['error']
    row = dict(**outcome, reason=reason, command=command, started=started,
        stdout=audit.read(directory / 'stdout', limit=STREAM_CAP, track=False, retain=False)[1],
        stderr=audit.read(directory / 'stderr', limit=STREAM_CAP, track=False, retain=False)[1],
        elapsed_seconds=time.monotonic() - start, gpu_execution_requested=seconds == NATIVE_SECONDS)
    if seconds == NATIVE_SECONDS:
        row['children_rusage'] = usage
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
    directory = Path(__file__).resolve().parent
    path = directory / 'layer_cpu_admission.py'
    require(type(CPU_ADMISSION_SHA) is str and re.fullmatch('[0-9a-f]{64}', CPU_ADMISSION_SHA),
            'layer CPU admission source binding remains pending')
    _, pin = audit.read(path, limit=1 << 20)
    require(pin['sha256'] == CPU_ADMISSION_SHA, 'reviewed layer CPU admission source')
    contract = load(path, CPU_ADMISSION_SHA, 'layer_cpu_admission_bound')
    contract.admit(plan, audit.read)
    return {key: plan[key] for key in
            ('runtime_cpu', 'runtime_sources', 'parent_cpu', 'worker_cpu', 'parent', 'worker', 'parent_sources', 'worker_sources')}


def checker_admission(audit):
    require(type(CHECKER_CPU) is dict and set(CHECKER_CPU) == {'path', 'bytes', 'sha256'},
            'actual readiness data-checker CPU binding remains pending')
    value = audit.parse(audit.read(CHECKER_CPU['path'], CHECKER_CPU)[0])
    require(value['schema'] == 'ferric-guarded-mlp-readiness40-tail-layer-duration-checker-cpu-v1'
            and value['passed'] is True and value['failure'] is None
            and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['sources_before'] == value['sources_after']
            and value['tests']['passed'] == CHECKER_TESTS
            and value['tests']['failed'] == value['tests']['errors'] == value['tests']['skipped'] == 0
            and value['sources_before']['validate_readiness.py']['sha256'] == VALIDATOR_SHA
            and value['sources_before']['readiness_announcement.py']['sha256'] == ANNOUNCEMENT_SHA
            and value['sources_before']['validate_shared.py']['sha256'] == SHARED_SHA,
            'actual independently exercised readiness checker and exact marker')
    compact = lambda pin: {key: pin[key] for key in ('bytes', 'sha256')}
    require(type(CHECKER_CONTROLLER) is dict and type(CHECKER_SOURCES) is dict
            and type(CHECKER_TESTS) is int and CHECKER_TESTS > 146,
            'reviewed layer checker controller, source closure and tests remain pending')
    require(compact(value['controller']) == CHECKER_CONTROLLER
            and value['supervisor']['sha256'] == '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
            and set(value['sources_before']) == {'run_cpu.py', 'supervisor.py', *CHECKER_SOURCES}
            and all(compact(value['sources_before'][name]) == pin for name, pin in CHECKER_SOURCES.items())
            and len(value['tests']['names']) == len(set(value['tests']['names'])) == CHECKER_TESTS
            and len(value['phases']) == 3
            and all(value[k] is False for k in ('gpu_execution', 'native_parent_execution', 'model_execution',
                'numerical_acceptance', 'full_model_acceptance', 'performance_claim', 'production_authority')),
            'exact exercised combined checker source closure and scope')
    require(set(value['leaf_tests']) == {'legacy', 'forward', 'layer'}
            and value['leaf_tests']['legacy']['passed'] == 126
            and value['leaf_tests']['forward']['passed'] == 20
            and value['leaf_tests']['layer']['passed'] == CHECKER_TESTS - 146
            and value['tests']['names'] == sorted(n for leaf in value['leaf_tests'].values()
                for n in leaf['names']), 'all old and new named checker scopes')
    for row, label in zip(value['phases'], ('legacy-tests', 'forward-tests', 'layer-tests')):
        require(row['label'] == label and type(row['exit_code']) is int and row['exit_code'] == 0
                and row['natural_exit'] is True and row['reaped'] is True
                and row['process_group_absent'] is True and row['forced_cleanup'] is False
                and row['timed_out'] is False and row['exception'] is None and row['storage_failure'] is None,
                'three checker leaves retired naturally')
    audit.read(value['input_manifest']['path'], value['input_manifest'], retain=False)
    for pin in value['sources_before'].values(): audit.read(pin['path'], pin, retain=False)
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



def load_validators(directory):
    names = ('validate_readiness', 'validate_shared', 'validate_timing', 'validate_matched', 'validate_scoped', 'validate_bank_scoped', 'validate_census', 'validate_tail', 'validate_duration', 'validate_forward')
    prior = {name: sys.modules.get(name) for name in names}
    try:
        validator = load(directory / 'validate_readiness.py', VALIDATOR_SHA, 'validate_readiness')
        sys.modules['validate_readiness'] = validator
        shared = load(directory / 'validate_shared.py', SHARED_SHA, 'validate_shared')
        sys.modules['validate_shared'] = shared
        timing = load(directory / 'validate_timing.py', TIMING_SHA, 'validate_timing')
        sys.modules['validate_timing'] = timing
        matched = load(directory / 'validate_matched.py', MATCHED_SHA, 'validate_matched')
        sys.modules['validate_matched'] = matched
        scoped = load(directory / 'validate_scoped.py', SCOPED_SHA, 'validate_scoped')
        sys.modules['validate_scoped'] = scoped
        bank = load(directory / 'validate_bank_scoped.py', BANK_SHA, 'validate_bank_scoped')
        sys.modules['validate_bank_scoped'] = bank
        census = load(directory / 'validate_census.py', CENSUS_SHA, 'validate_census')
        sys.modules['validate_census'] = census
        tail = load(directory / 'validate_tail.py', TAIL_SHA, 'validate_tail')
        sys.modules['validate_tail'] = tail
        old_diagnostic = load(directory / 'validate_duration.py', DURATION_SHA, 'validate_duration')
        sys.modules['validate_duration'] = old_diagnostic
        forward = load(directory / 'validate_forward.py', FORWARD_SHA, 'validate_forward')
        sys.modules['validate_forward'] = forward
        diagnostic = load(directory / 'validate_layer.py', LAYER_SHA, 'validate_layer')
        return validator, shared, timing, matched, scoped, bank, census, tail, diagnostic
    finally:
        for name, module in prior.items():
            if module is None: sys.modules.pop(name, None)
            else: sys.modules[name] = module


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
    validator, shared, timing, matched, scoped, bank, census, tail, diagnostic = load_validators(directory)
    guard = load(directory / 'readiness_announcement.py', ANNOUNCEMENT_SHA, 'guarded_model_announcement')
    audit.HARD_DEADLINE = deadline
    plan_raw, plan_pin = audit.read(directory / (case + '-input.json'), limit=65536)
    require(plan_pin['sha256'] == PLAN_SHA[case], 'exact case input')
    plan = audit.parse(plan_raw)
    validator.keys(plan, 'schema case profile runtime_cpu runtime_sources parent_cpu worker_cpu parent worker parent_sources worker_sources request')
    require(plan['schema'] == 'ferric-guarded-mlp-readiness40-tail-layer-duration-gpu-input-v1'
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
    for path in (directory / 'run_layer_model_gpu.py', directory / 'library_audit.py', directory / 'frozen_owned.py',
                 directory / 'validate_readiness.py', directory / 'validate_shared.py', directory / 'validate_timing.py',
                 directory / 'validate_matched.py', directory / 'validate_scoped.py', directory / 'validate_scoped_pair.py',
                 directory / 'validate_bank_scoped.py', directory / 'validate_census.py',
                 directory / 'validate_tail.py', directory / 'validate_duration.py',
                 directory / 'validate_forward.py', directory / 'validate_layer.py', directory / 'layer_cpu_admission.py',
                 directory / 'readiness_announcement.py', Path('/opt/rocm/bin/amd-smi'),
                 Path('/usr/bin/readelf'), Path('/usr/bin/ldd'), Path(sys.executable)):
        audit.resolved_input(path)
    controller_pin = audit.INPUTS[str(directory / 'run_layer_model_gpu.py')]
    out.mkdir(mode=0o700); phases = []; errors = []; checked = None; matched_checked = None; parity = None; lineage_checked = False; attempts = 0; dependencies = {}; initial = None
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
            '--observe-guarded-readiness40-position5-bank-scoped-census-tail-host-timing-v4',
            '--allow-unauthenticated-machine-code'], NATIVE_SECONDS)
        summary_path = out / 'native/complete.json'
        summary, summary_pin = audit.read(summary_path, limit=128 << 10, track=False)
        expected = {'complete.json', 'frames.ndjson', 'child-stderr.bin', 'host-timing.json'} | {
            'capture-%d.bin' % i for i in (0, 5, 16, 39)}
        require({p.name for p in summary_path.parent.iterdir()} == expected, 'exact eight native bodies')
        def read_body(pin):
            return audit.read(pin['path'], pin, limit=2 << 20, track=False)[0]
        matched_checked = diagnostic.validate('tail_layer', raw, summary, request, summary_path.parent, read_body, tokens)
        checked = matched_checked['ordinary']
        def parity_read(pin):
            return audit.read(pin['path'], pin, limit=2 << 20,
                track=not Path(pin['path']).is_relative_to(out))[0]
        parity = diagnostic.compare_same_side(summary, baseline_raw, matched_checked, baseline_checked, parity_read)
        save(out / 'parity.json', parity)
        native = phases[-1]
        start_doc = audit.parse(audit.read(out / 'parent/started.json', track=False)[0])
        guard.validate_lineage(audit.read(out / 'parent/stderr', track=False)[0],
                               native, start_doc, checked['child_pid'])
        lineage_checked = True
        save(out / 'observation.json', checked)
        save(out / 'matched.json', matched_checked)
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
            raw_names |= {'native/complete.json', 'native/frames.ndjson', 'native/child-stderr.bin', 'native/host-timing.json'}
            raw_names |= {'native/capture-%d.bin' % i for i in (0, 5, 16, 39)}
            raw_names |= {'observation.json', 'parity.json', 'matched.json'}
            require(set(raw_files) == raw_names and len(raw_files) == 73,
                    'closed successful diagnostic73 original raw bodies')
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
    result = dict(schema='ferric-guarded-mlp-readiness40-tail-layer-duration-gpu-v1',
        passed=not errors and checked is not None and matched_checked is not None and parity is not None and lineage_checked,
        errors=errors, case=case, profile='readiness40_position5', native_attempts=attempts, retries=0,
        plan=plan_pin, admission=admission, controller=controller_pin, phases=phases, checker_cpu=checker_cpu,
        platform=initial, dependencies=dependencies, readset=audit.INPUTS, raw=raw_files, observation=checked,
        matched_timing=matched_checked, instrumented=True, currentness_duration_diagnostic_requested=True,
        forward_phase_durations=True, disjoint_forward_phases=True, currentness_durations_nested=True,
        layer_phase_durations=True, paired_mlp_durations_nested=True,
        bank_guarded_body_includes_callbacks=True, matched_speed_comparison=False,
        same_side_parity=parity, baseline=BASELINE,
        parent_host_timing_requested=True, gpu_timing=False, host_observer_requested=False,
        paired_terminal_requested=False, cache_kernel_admission_requested=False, operational_currentness_requested=False,
        artifact_contains_full2303_route=True,
        owned_worker_lineage_verified=lineage_checked, prompt_tokens=2048,
        prompt_positions_requested=40, generated_tokens_requested=0, capture_positions=[0, 5, 16, 39],
        allocation_pages=144, default_full_currentness_requested=False,
        shared_full_currentness_requested=False, scoped_warm_currentness_requested=True,
        bank_scoped_rearm_requested=True, scoped_capacity_census_requested=True,
        allocation_preflights_changed=True, census_counters_are_layer_subset=True,
        scoped_tail_requested=True, tail_counters_are_independent=True,
        currentness_temporal_equivalence_claim=False, paired_read_requested=False,
        limits=dict(native_seconds=NATIVE_SECONDS, whole_seconds=CASE_SECONDS, address_space_bytes=32 << 30,
                    stream_bytes=STREAM_CAP, case_bytes=CASE_CAP, case_members=256, affinity=[8, 9]),
        started_monotonic=start, completed_monotonic=time.monotonic(),
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
    require(len(sys.argv) == 2, 'run_layer_model_gpu.py tail_layer')
    return run_case(sys.argv[1])


if __name__ == '__main__':
    raise SystemExit(main())

