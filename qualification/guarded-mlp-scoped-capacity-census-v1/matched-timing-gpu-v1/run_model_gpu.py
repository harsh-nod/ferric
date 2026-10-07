"""One BankV2 or CensusV3 timed case; census requires the same qualified control ELF pair."""
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
ROOT = E / 'guarded-mlp-readiness40-bank-scoped-census-matched-timing-gpu-v228-v1'
WORKER_ROOT = E / 'guarded-mlp-scoped-capacity-census-cpu-v228-v1'
PARENT_ROOT = E / 'guarded-mlp-readiness40-bank-scoped-census-parent-cpu-v228-v1'
ORDER = ('bank', 'census')
PLAN_SHA = {'bank': 'fbb7bd0a9d51febfdadfb47a387dca2b2430e84dfe483a5028603e6cab555f30', 'census': '973c0aa3a47a0ae3b834199d75c19b6f0a4ffb3efba6e98008c6b5659f1dee81'}
CHECKER_CPU = {"path":"/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-bank-scoped-census-checker-cpu-v228-v1/evidence/complete.json","bytes":29548,"sha256":"50d1268c73ea34d2ebfe98b995cf56900cd51255dd44234017bbb67f94cac729"}
CHECKER_TESTS = 98
CHECKER_CONTROLLER = dict(bytes=20457, sha256='d7d2952b5e6dc5d30342b3f71b7ec8230d743a24fbe5c84976bbd14191dce736')
CHECKER_SOURCES = {"validate_readiness.py":{"bytes":19800,"sha256":"0c319e99142b19909350d9a81404948b494c2e3ea0defbbc165c2e5529be032a"},"readiness_announcement.py":{"bytes":3246,"sha256":"b974ac6b6e936d8239639ac0c595c7db36700e3b1e2cff101224699fd789a9b1"},"test_readiness.py":{"bytes":17802,"sha256":"4b644462a71120ee45b3351757ecb0d5867c9453daadb6a10c07bdbcd38e8bb5"},"validate_shared.py":{"bytes":8015,"sha256":"6557fe5c082b2c92bae15274dd0d19bba3da8c3c36b9fc74757b4c1b74a87eca"},"test_shared.py":{"bytes":10737,"sha256":"a122f42f8a47a0213010f1812ef1f95fa7ff582db6a9ffa190356a35874f6b96"},"validate_timing.py":{"bytes":10104,"sha256":"27c652f5452676b4cc08a634241aff89b2653e85ee111133c1f1c4ab13efd93f"},"test_timing.py":{"bytes":10433,"sha256":"630febf4d840081887692144cc570d70c1d67f9ad3bb5bab1fd6d7d1f61cedb1"},"validate_matched.py":{"bytes":9362,"sha256":"d31c94a4dd1f254d166be185fe1cca10bc4eed26e500a0cb1fc3c5a74a574437"},"test_matched.py":{"bytes":15400,"sha256":"d5a6b522a1effe29e1ee08430754163cc217cf34c3713ce234926b7c860f9647"},"validate_scoped.py":{"bytes":9184,"sha256":"c2541e11e534cf1e5a4faade31fe4a734726552171fed9617d6ea938f680b6fd"},"test_scoped.py":{"bytes":15936,"sha256":"4c015cdfa072943960e0ffa1f25e0e604790e8dbe222875b4bed207be6563fdc"},"validate_scoped_pair.py":{"bytes":3233,"sha256":"495fc476454e4052467810b55e177f9d2b91d86d3496a6b90876963f7f60ac62"},"test_scoped_pair.py":{"bytes":3629,"sha256":"aa95ff79fd3c72ee3658d66e4117738a06f7984a2a77dcc6712b28a272e57f8f"},"validate_bank_scoped.py":{"bytes":9534,"sha256":"2432fc14c0ba40fc934abcd7886459c12fc31ca6167ba4cf908a8c7974d26772"},"validate_bank_pair.py":{"bytes":4428,"sha256":"9d3f1470f6e9d7a17f3c91d1deb29a54c6fa7e8c766dc358e9d7d73e15c4e004"},"test_bank_scoped.py":{"bytes":14853,"sha256":"3ecc622e660cb6e0790a18ff8bb022a84535c91466d02d6fdbb8625ae080a99b"},"validate_census.py":{"bytes":10392,"sha256":"090431e59a481d5b681311d5a015ddfd35705cc828c920118d4f0043b862ec9f"},"validate_census_pair.py":{"bytes":4730,"sha256":"d4f50afe412b68c4066e934a80b96b5a5c0f83ef8ea33c59c53380aa1ab819c4"},"test_census.py":{"bytes":19688,"sha256":"aca29934c68523dbc91b30439a2c7a5ae552b64a9faf077ddb99b97c861cb5b0"}}
SCOPED_SHA = 'c2541e11e534cf1e5a4faade31fe4a734726552171fed9617d6ea938f680b6fd'
BANK_SHA = '2432fc14c0ba40fc934abcd7886459c12fc31ca6167ba4cf908a8c7974d26772'
PAIR_SHA = 'd4f50afe412b68c4066e934a80b96b5a5c0f83ef8ea33c59c53380aa1ab819c4'
CENSUS_SHA = '090431e59a481d5b681311d5a015ddfd35705cc828c920118d4f0043b862ec9f'
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
WORKER = {'bytes': 6476504, 'sha256': '00beb857fd00914f1f305f9646c2087e20901b81f332d774b8211240497e6f95'}
WORKER_CPU = {'bytes': 2442525, 'sha256': '4d68df6862da654f9044cb08ad5ecedf008fb96f1fa72345d5382d6c96b89e41'}
PARENT = {"bytes": 14117968, "sha256": "f0d91a0413591b2c6f6b9c0420652ec418ca5377fc3d848b8d23ebd4b03c1021"}
PARENT_CPU = {"bytes": 4095966, "sha256": "e3915ff3e15e7d52fb57675bda0cc0752ec349c1a8317b0ef826ddcdaed70d44"}
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
    require(plan['worker_cpu']['path'] == str(WORKER_ROOT / 'evidence/complete.json')
            and plan['parent_cpu']['path'] == str(PARENT_ROOT / 'evidence/complete.json'),
            'actual successful census coupled and parent qualification roots')
    worker = doc(plan['worker_cpu']); parent = doc(plan['parent_cpu'])
    require(worker['source_generation'] == 'scoped-capacity-census-coupled-v1'
            and worker['controller']['path'] == str(WORKER_ROOT / 'run_cpu.py')
            and parent['controller']['path'] == str(PARENT_ROOT / 'run_cpu.py'),
            'explicit census coupled generation and parent controller identity')
    for name, expected in [('worker_cpu', WORKER_CPU), ('worker', WORKER),
                           ('parent_cpu', PARENT_CPU), ('parent', PARENT)]:
        require({k: plan[name][k] for k in ('bytes', 'sha256')} == expected, 'actual readiness CPU/ELF pin')
    for value, schema, phases, sources in [
            (worker, 'ferric-guarded-mlp-scoped-capacity-census-cpu-v1', 26, 1049),
            (parent, 'ferric-guarded-mlp-readiness40-bank-scoped-census-parent-cpu-v1', 67, 1282)]:
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
    require(set(worker['inventories']) == {'runtime', 'worker'}
            and len(worker['inventories']['runtime']) == 1173
            and len(worker['inventories']['worker']) == 783
            and worker['full_runtime_tests_executed'] is True
            and worker['full_worker_tests_executed'] is True
            and worker['tests']['kfd-tests']['passed'] == 1165
            and worker['tests']['kfd-tests']['failed'] == 0
            and worker['tests']['kfd-tests']['ignored'] == 8
            and worker['tests']['worker-tests']['passed'] == 779
            and worker['tests']['worker-tests']['failed'] == 0
            and worker['tests']['worker-tests']['ignored'] == 4
            and worker['selected_facade_doctests_executed'] is True
            and worker['all_crate_doctests_executed'] is False
            and worker['tests']['interface-doc-tests']['passed'] == 10
            and worker['tests']['interface-doc-tests']['failed'] == 0
            and worker['doc_parser_tests']['passed'] == 8
            and worker['doc_parser_tests']['failed'] == 0
            and worker['doc_parser_tests']['project_execution'] is False
            and worker['doc_parser_tests']['gpu_execution'] is False,
            'actual coupled runtime/worker/full named suite and selected docs')
    require(set(worker['artifacts']) == {'kfd-lib', 'engineering-worker-test', 'guarded-facade-test',
                'debug-trap-test', 'telemetry-env-test', 'telemetry-test',
                'worker-lib', 'worker-bin-test', 'worker-readiness-test', 'worker-wire-test', 'worker'}
            and len({r['pin']['path'] for r in worker['artifacts'].values()}) == 11
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and all(worker[k] is True for k in ('scoped_currentness_runtime_source_added',
                'scoped_currentness_worker_source_added', 'explicit_scoped_readiness_selector_source_added',
                'inherited_terminal_pair_runtime_preserved', 'legacy_terminal_dispatch_preserved',
                'fresh_terminal_opt_in_refused', 'inherited_peer_read_pair_runtime_preserved',
                'scoped_bank_rearm_runtime_source_added', 'readiness40_bank_scoped_warm_source_added',
                'inherited_scoped_currentness_runtime_preserved', 'runtime_source_changed',
                'full_model_long_request_enabled', 'full2303_scoped_warm_source_added',
                'inherited_bank_scoped_currentness_preserved', 'scoped_capacity_census_runtime_source_added',
                'readiness40_bank_scoped_census_source_added', 'allocation_preflights_changed',
                'allocation_preflights_changed_only_for_explicit_warm_position5'))
            and all(worker[k] is False for k in ('scoped_currentness_native_execution',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'arena_policy_changed', 'default_fresh_arena_policy_changed', 'ordinary_read_api_changed',
                'public_runtime_limits_changed', 'lockfiles_changed', 'shared_cache_changed',
                'readiness40_bank_scoped_warm_native_execution', 'readiness40_bank_scoped_census_native_execution',
                'full2303_scoped_warm_native_execution', 'full2303_launch_admitted', 'gpu_qualified',
                'numerical_acceptance', 'performance_claim', 'production_authority')),
            'actual explicit scoped source, preserved defaults and genuine executable tests')
    require(len(parent['tests']) == 57 and len(parent['inventory']) == 1001
            and sum(row['passed'] for row in parent['tests'].values()) == 535
            and all(row['failed'] == row['ignored'] == 0 for row in parent['tests'].values())
            and len(parent['artifacts']) == 7 and parent['all_selected_parent_tests_executed'] is True
            and parent['full_parent_library_suite_executed'] is False
            and parent['position5_diagnostic_parent_route_added'] is True
            and parent['position5_native_execution'] is False
            and parent['inherited_readiness_parent_route_preserved'] is True
            and parent['readiness_native_execution'] is False and parent['full_long_workload'] is False,
            'selected readiness parent census')
    require(parent['readiness40_bank_scoped_warm_parent_source_added'] is True
            and parent['readiness40_bank_scoped_warm_native_execution'] is False
            and parent['allocation_preflights_changed'] is True
            and parent['allocation_preflights_changed_only_for_explicit_warm_position5'] is True
            and parent['readiness40_bank_scoped_census_parent_source_added'] is True
            and parent['readiness40_bank_scoped_census_native_execution'] is False
            and parent['full2303_scoped_warm_parent_source_added'] is True
            and parent['full2303_scoped_warm_native_execution'] is False
            and parent['full2303_launch_admitted'] is False
            and parent['readiness40_scoped_warm_parent_source_added'] is True
            and parent['readiness40_scoped_warm_native_execution'] is False
            and parent['scoped_warm_host_timing_parent_source_added'] is True
            and parent['currentness_temporal_equivalence_claim'] is False
            and parent['qualified_worker_sources_preserved'] is True
            and parent['ordinary_wire_schema_changed'] is False
            and parent['ordinary_observation_schema_changed'] is False,
            'qualified separately selected scoped ordinary and timed parent routes')
    require(parent['shared_full_host_timing_parent_source_added'] is True
            and parent['shared_full_host_timing_native_execution'] is False
            and parent['parent_host_timing_source_added'] is True
            and parent['parent_host_timing_native_execution'] is False
            and parent['parent_host_timing_rows'] == 40 and parent['parent_host_timing_disjoint_spans'] == 124
            and parent['default_policy_changed'] is False and parent['global_currentness_policy_changed'] is False,
            'both timed routes qualified with unchanged clock partition and default policy')
    prefix = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
    compact = lambda value: {k: {n: v[n] for n in ('bytes', 'sha256')}
        for k, v in value['final_sources'].items() if k.startswith(prefix)}
    require(len(compact(worker)) == 220 and compact(worker) == compact(parent),
            'all220 parent-compiled worker bodies equal actual standalone worker')
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
    require(value['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-checker-cpu-v1'
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
    require(type(CHECKER_CONTROLLER) is dict, 'reviewed scoped checker controller source remains pending')
    require(compact(value['controller']) == CHECKER_CONTROLLER
            and value['supervisor']['sha256'] == '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
            and set(value['sources_before']) == {'run_cpu.py', 'supervisor.py', *CHECKER_SOURCES}
            and all(compact(value['sources_before'][name]) == pin for name, pin in CHECKER_SOURCES.items())
            and len(value['tests']['names']) == len(set(value['tests']['names'])) == CHECKER_TESTS
            and len(value['phases']) == 1
            and all(value[k] is False for k in ('gpu_execution', 'native_parent_execution', 'model_execution',
                'numerical_acceptance', 'full_model_acceptance', 'performance_claim', 'production_authority')),
            'exact exercised combined checker source closure and scope')
    row = value['phases'][0]
    require(row['exit_code'] == 0 and row['natural_exit'] is True and row['reaped'] is True
            and row['process_group_absent'] is True and row['forced_cleanup'] is False
            and row['timed_out'] is False and row['exception'] is None and row['storage_failure'] is None,
            'actual combined checker clean natural leaf')
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
    names = ('validate_readiness', 'validate_shared', 'validate_timing', 'validate_matched', 'validate_scoped', 'validate_bank_scoped', 'validate_census')
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
        pair = load(directory / 'validate_census_pair.py', PAIR_SHA, 'validate_census_pair')
        return validator, shared, timing, matched, scoped, bank, census, pair
    finally:
        for name, module in prior.items():
            if module is None: sys.modules.pop(name, None)
            else: sys.modules[name] = module


def bank_admission(audit, validator, bank, guard, tokens, admission,
                      controller, checker, sha, start, baseline_raw, baseline_checked):
    require(type(sha) is str and re.fullmatch('[0-9a-f]{64}', sha), 'observed BankV2 control terminal SHA')
    root = ROOT / 'bank'
    raw, pin = audit.read(root / 'complete.json', limit=8 << 20)
    require(pin['sha256'] == sha and not os.path.lexists(root / 'failed.json'),
            'exact original successful BankV2 control terminal; no failed-case promotion')
    old = audit.parse(raw)
    require(old['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-matched-timing-gpu-v1'
            and old['passed'] is True and old['errors'] == [] and old['case'] == 'bank'
            and old['profile'] == 'readiness40_position5' and old['native_attempts'] == 1 and old['retries'] == 0
            and old['admission'] == admission and old['controller'] == controller and old['checker_cpu'] == checker
            and old['prior_bank'] is None and old['matched_pair'] is None
            and old['owned_worker_lineage_verified'] is True and old['parent_host_timing_requested'] is True
            and old['default_full_currentness_requested'] is False and old['shared_full_currentness_requested'] is False
            and old['scoped_warm_currentness_requested'] is True
            and old['bank_scoped_rearm_requested'] is True and old['allocation_preflights_changed'] is False
            and old['scoped_capacity_census_requested'] is False and old['census_counters_are_layer_subset'] is False and old['currentness_temporal_equivalence_claim'] is False
            and old['baseline'] == BASELINE and old['native_spawn_observed'] is True
            and old['gpu_execution_confirmed'] is True
            and all(old[k] is False for k in ('gpu_timing', 'host_observer_requested', 'paired_terminal_requested',
                'paired_read_requested', 'cache_kernel_admission_requested', 'operational_currentness_requested',
                'full_model_acceptance', 'numerical_acceptance', 'performance_claim', 'production_authority',
                'full_long_workload', 'full2303_native_enabled', 'ar4_parity_transferred')),
            'same qualified controller/products and explicit BankV2 control timing scope')
    require(type(old['started_monotonic']) in (int, float) and type(old['completed_monotonic']) in (int, float)
            and 0 < old['started_monotonic'] < old['completed_monotonic'] < start,
            'same-boot BankV2 control terminal chronologically precedes CensusV3 invocation')
    labels = ['parent-readelf', 'parent-ldd', 'worker-readelf', 'worker-ldd']
    labels += ['before-' + str(i) for i in range(3)] + ['parent'] + ['after-' + str(i) for i in range(3)]
    names = {label + '/' + name for label in labels
             for name in ('command.json', 'started.json', 'stdout', 'stderr', 'result.json')}
    names |= {'initial-topology.json', 'observation.json', 'parity.json', 'matched.json'}
    names |= {side + '-' + str(i) + '-topology.json' for side in ('before', 'after') for i in range(3)}
    names |= {'native/complete.json', 'native/frames.ndjson', 'native/child-stderr.bin', 'native/host-timing.json'}
    names |= {'native/capture-%d.bin' % i for i in (0, 5, 16, 39)}
    require(set(old['raw']) == names and len(names) == 73
            and [row['label'] for row in old['phases']] == labels, 'closed complete BankV2 control raw and phase rosters')
    require({str(p.relative_to(root)) for p in root.rglob('*') if p.is_file()} == names | {'complete.json'},
            'no unrecorded BankV2 control files')
    inventory(root)
    bodies = {}
    for name, recorded in old['raw'].items():
        require(recorded['path'] == str(root / name), 'exact prior BankV2 control body path')
        bodies[name] = audit.read(recorded['path'], recorded, limit=8 << 20)[0]
    for row in old['phases']:
        successful(row)
        label = row['label']
        require(audit.parse(bodies[label + '/result.json']) == {k: v for k, v in row.items() if k != 'label'},
                'BankV2 control original owned result join')
        for key, suffix in [('command', 'command.json'), ('started', 'started.json'), ('stdout', 'stdout'), ('stderr', 'stderr')]:
            require(row[key] == old['raw'][label + '/' + suffix], 'BankV2 control original phase body pins')
        if label.startswith(('before-', 'after-')):
            audit.idle(bodies[label + '/stdout'])
            snapshot = audit.parse(bodies[label + '-topology.json'])
            validate_topology(snapshot)
            require(snapshot == old['platform'], 'BankV2 control six idle samples and immutable topology')
    require(audit.parse(bodies['initial-topology.json']) == old['platform'], 'BankV2 control initial topology')
    require(old['plan']['path'] == str(ROOT / 'bank-input.json')
            and old['plan']['sha256'] == PLAN_SHA['bank'], 'pinned BankV2 control input')
    plan = audit.parse(audit.read(old['plan']['path'], old['plan'])[0])
    validator.keys(plan, 'schema case profile parent_cpu worker_cpu parent worker request')
    require(plan['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-matched-timing-gpu-input-v1'
            and plan['case'] == 'bank' and plan['profile'] == 'readiness40_position5'
            and plan['request']['path'] == str(ROOT / 'bank-request.json')
            and cpu_admission(plan, audit) == admission, 'same actual qualified BankV2 control CPU/product pair')
    request = validator.parse(audit.read(plan['request']['path'], plan['request'])[0])
    require(request['base']['evidence_directory'] == str(root / 'native'), 'BankV2 control native output root')
    command = audit.parse(bodies['parent/command.json'])
    require(command['argv'] == [plan['parent']['path'], '--request', plan['request']['path'],
            '--observe-guarded-readiness40-position5-bank-scoped-warm-host-timing-v2', '--allow-unauthenticated-machine-code']
            and command['env'] == ENV and command['cwd'] == str(E)
            and command['deadline_seconds'] == NATIVE_SECONDS and command['gpu_execution_requested'] is True,
            'actual BankV2 control selector and bounded command')
    def read_body(value):
        return audit.read(value['path'], value, limit=2 << 20)[0]
    checked = bank.validate('bank_timed', bodies['parent/stdout'], bodies['native/complete.json'],
                               request, root / 'native', read_body, tokens)
    require(validator.same(checked, old['matched_timing'])
            and validator.same(checked, audit.parse(bodies['matched.json']))
            and validator.same(checked['ordinary'], old['observation'])
            and validator.same(checked['ordinary'], audit.parse(bodies['observation.json'])),
            'independently revalidated original BankV2 control policy/timeline/summary')
    parity = bank.compare_same_side(bodies['native/complete.json'], baseline_raw,
                                       checked, baseline_checked, read_body)
    require(validator.same(parity, old['same_side_parity'])
            and validator.same(parity, audit.parse(bodies['parity.json'])), 'BankV2 control historical full parity recheck')
    native = old['phases'][7]
    started = audit.parse(bodies['parent/started.json'])
    guard.validate_lineage(bodies['parent/stderr'], native, started, checked['ordinary']['child_pid'])
    require(type(old['readset']) is dict and 0 < len(old['readset']) <= 1024, 'bounded BankV2 control input readset')
    for path, recorded in old['readset'].items():
        require(recorded['path'] == path, 'BankV2 control readset path key')
        audit.read(path, recorded, limit=128 << 20, retain=False)
    return dict(pin=pin, terminal=old, summary=bodies['native/complete.json'],
                checked=checked, parent_identity=started['parent'])


def case_deadline(deadline):
    remaining = deadline - time.monotonic()
    require(remaining > 0, 'whole case deadline')
    signal.signal(signal.SIGALRM, interrupted)
    signal.setitimer(signal.ITIMER_REAL, remaining)


def run_case(case, bank_sha=None):
    start = time.monotonic()
    handlers = {number: signal.getsignal(number) for number in SIGNALS}
    timer, interval = signal.getitimer(signal.ITIMER_REAL)
    try:
        for number in SIGNALS:
            signal.signal(number, interrupted)
        case_deadline(start + CASE_SECONDS)
        return _run_case(case, start, start + CASE_SECONDS, bank_sha)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        for number, handler in handlers.items():
            signal.signal(number, handler)
        if timer:
            signal.setitimer(signal.ITIMER_REAL, max(0.001, timer - (time.monotonic() - start)), interval)


def _run_case(case, start, deadline, bank_sha):
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary Python required')
    require(type(case) is str and case in ORDER, 'one explicit readiness case; no implicit rerun')
    require((case == 'bank' and bank_sha is None) or
            (case == 'census' and type(bank_sha) is str and re.fullmatch('[0-9a-f]{64}', bank_sha)),
            'Census candidate requires the observed successful BankV2 control terminal SHA')
    directory = Path(__file__).resolve().parent
    require(directory == ROOT and os.getuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged deployment host/root')
    require(type(PLAN_SHA[case]) is str and re.fullmatch('[0-9a-f]{64}', PLAN_SHA[case]), 'actual case input remains unbound')
    audit = load(directory / 'library_audit.py', AUDIT_SHA, 'guarded_model_audit')
    owned = load(directory / 'frozen_owned.py', OWNED_SHA, 'guarded_model_owned')
    validator, shared, timing, matched, scoped, bank, census, pair_validator = load_validators(directory)
    guard = load(directory / 'readiness_announcement.py', ANNOUNCEMENT_SHA, 'guarded_model_announcement')
    audit.HARD_DEADLINE = deadline
    plan_raw, plan_pin = audit.read(directory / (case + '-input.json'), limit=65536)
    require(plan_pin['sha256'] == PLAN_SHA[case], 'exact case input')
    plan = audit.parse(plan_raw)
    validator.keys(plan, 'schema case profile parent_cpu worker_cpu parent worker request')
    require(plan['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-matched-timing-gpu-input-v1'
            and plan['case'] == case and plan['profile'] == 'readiness40_position5', 'closed readiness profile')
    out = ROOT / case; require(not os.path.lexists(out), 'fresh one-shot case root')
    require(case != 'bank' or not os.path.lexists(ROOT / 'census'), 'BankV2 control must precede a fresh CensusV3 case')
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
                 directory / 'validate_readiness.py', directory / 'validate_shared.py', directory / 'validate_timing.py',
                 directory / 'validate_matched.py', directory / 'validate_scoped.py', directory / 'validate_scoped_pair.py',
                 directory / 'validate_bank_scoped.py', directory / 'validate_census.py', directory / 'validate_census_pair.py',
                 directory / 'readiness_announcement.py', Path('/opt/rocm/bin/amd-smi'),
                 Path('/usr/bin/readelf'), Path('/usr/bin/ldd'), Path(sys.executable)):
        audit.resolved_input(path)
    controller_pin = audit.INPUTS[str(directory / 'run_model_gpu.py')]
    control = None
    if case == 'census':
        control = bank_admission(audit, validator, bank, guard, tokens, admission,
                                    controller_pin, checker_cpu, bank_sha, start, baseline_raw, baseline_checked)
    out.mkdir(mode=0o700); phases = []; errors = []; checked = None; matched_checked = None; pair = None; parity = None; lineage_checked = False; attempts = 0; dependencies = {}; initial = None
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
        require(control is None or control['terminal']['platform'] == initial, 'same host/boot/topology for matched pair')
        for key in ('parent', 'worker'):
            dynamic = leaf(key + '-readelf', ['/usr/bin/readelf', '-l', '-d', plan[key]['path']], AUDIT_SECONDS)
            linked = leaf(key + '-ldd', ['/usr/bin/ldd', plan[key]['path']], AUDIT_SECONDS)
            dependencies[key] = audit.runtime_libraries(dynamic, linked)
        for index in range(3):
            idle('before-' + str(index))
            if index < 2: time.sleep(.2)
        attempts = 1
        raw = leaf('parent', [plan['parent']['path'], '--request', plan['request']['path'],
            ('--observe-guarded-readiness40-position5-bank-scoped-warm-host-timing-v2' if case == 'bank' else
             '--observe-guarded-readiness40-position5-bank-scoped-census-host-timing-v3'),
            '--allow-unauthenticated-machine-code'], NATIVE_SECONDS)
        summary_path = out / 'native/complete.json'
        summary, summary_pin = audit.read(summary_path, limit=128 << 10, track=False)
        expected = {'complete.json', 'frames.ndjson', 'child-stderr.bin', 'host-timing.json'} | {
            'capture-%d.bin' % i for i in (0, 5, 16, 39)}
        require({p.name for p in summary_path.parent.iterdir()} == expected, 'exact eight native bodies')
        def read_body(pin):
            return audit.read(pin['path'], pin, limit=2 << 20, track=False)[0]
        matched_checked = (bank.validate('bank_timed', raw, summary, request, summary_path.parent, read_body, tokens)
            if case == 'bank' else census.validate('census_timed', raw, summary, request, summary_path.parent, read_body, tokens))
        checked = matched_checked['ordinary']
        def parity_read(pin):
            return audit.read(pin['path'], pin, limit=2 << 20,
                track=not Path(pin['path']).is_relative_to(out))[0]
        parity = (bank.compare_same_side(summary, baseline_raw, matched_checked, baseline_checked, parity_read)
            if case == 'bank' else census.compare_same_side(summary, baseline_raw, matched_checked, baseline_checked, parity_read))
        save(out / 'parity.json', parity)
        native = phases[-1]
        start_doc = audit.parse(audit.read(out / 'parent/started.json', track=False)[0])
        guard.validate_lineage(audit.read(out / 'parent/stderr', track=False)[0],
                               native, start_doc, checked['child_pid'])
        lineage_checked = True
        save(out / 'observation.json', checked)
        save(out / 'matched.json', matched_checked)
        if control is not None:
            require(start_doc['parent']['pid'] != control['parent_identity']['pid'],
                    'distinct actual native parent process identities')
            pair = pair_validator.compare_pair(control['summary'], summary, control['checked'], matched_checked,
                    dict(bank=control['terminal']['admission'], census=admission), parity_read)
            save(out / 'pair.json', pair)
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
            if case == 'census': raw_names.add('pair.json')
            require(set(raw_files) == raw_names and len(raw_files) == (73 if case == 'bank' else 74),
                    'closed successful bank73 or census74 raw bodies')
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
    result = dict(schema='ferric-guarded-mlp-readiness40-bank-scoped-census-matched-timing-gpu-v1',
        passed=not errors and checked is not None and matched_checked is not None and parity is not None and lineage_checked
            and (case == 'bank' or pair is not None),
        errors=errors, case=case, profile='readiness40_position5', native_attempts=attempts, retries=0,
        plan=plan_pin, admission=admission, controller=controller_pin, phases=phases, checker_cpu=checker_cpu,
        platform=initial, dependencies=dependencies, readset=audit.INPUTS, raw=raw_files, observation=checked,
        matched_timing=matched_checked, matched_pair=pair,
        prior_bank=control['pin'] if control is not None else None,
        same_side_parity=parity, baseline=BASELINE,
        parent_host_timing_requested=True, gpu_timing=False, host_observer_requested=False,
        paired_terminal_requested=False, cache_kernel_admission_requested=False, operational_currentness_requested=False,
        artifact_contains_full2303_route=True,
        owned_worker_lineage_verified=lineage_checked, prompt_tokens=2048,
        prompt_positions_requested=40, generated_tokens_requested=0, capture_positions=[0, 5, 16, 39],
        allocation_pages=144, default_full_currentness_requested=False,
        shared_full_currentness_requested=False, scoped_warm_currentness_requested=True,
        bank_scoped_rearm_requested=True, scoped_capacity_census_requested=case == 'census',
        allocation_preflights_changed=case == 'census', census_counters_are_layer_subset=case == 'census',
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
    require(len(sys.argv) in (2, 3), 'run_model_gpu.py bank | census OBSERVED_BANK_TERMINAL_SHA')
    return run_case(sys.argv[1], sys.argv[2] if len(sys.argv) == 3 else None)


if __name__ == '__main__':
    raise SystemExit(main())
