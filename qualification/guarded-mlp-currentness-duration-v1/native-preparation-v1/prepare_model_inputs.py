"""Prepare one instrumented Tail V4 diagnostic request from observed matching CPU products; no native execution."""
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import struct
import sys
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-readiness40-tail-currentness-duration-gpu-v228-v1'
ORDER = ('tail_duration',)
WORKER_ROOT = E / 'guarded-mlp-currentness-duration-diagnostic-cpu-v228-v2'
PARENT_NAME = 'ferric-qwen3-guarded-mlp-readiness-engineering'
OLD_REQUEST = (9338, '526f0c30630edb83114cfa763ffb98b9242154371250797b9ed55bf8747bed1d')
PARENT_ROOT = E / 'guarded-mlp-currentness-duration-diagnostic-parent-cpu-v228-v2'
WORKER_SOURCES = (450252, '74bcd0c27c264faf5991e8b13f8dcd2b9f52d42ad1d9b3e9a30350ee2c1baa0c')
PARENT_SOURCES = (537481, '2737318f2f1e13ac89eabf9e1ad603fc11c11f6a86adfbd2e0115a9bcb6a0a1b')
DURATION_WORKER_SOURCE = dict(bytes=25694, sha256='c6224930fef73622d36b2634f5ef88846db8c1b55e9a7e3fdd550247e5024e52')
DURATION_RUNTIME_SOURCE = dict(bytes=14821, sha256='85cd4a641b0e7429f3eeb8aebe310e56a64487b467ca5e81431002c4a7ffc8aa')
WORKER_CPU = (2569774, '5a27b9983ecd0d523226cbf2db4070108cd6deaf75d2ee6fe528274d2f8a5ac2')
WORKER_ELF = (6598384, '10c7432d29f8ca780ce9338c3217bd93c75b04cd3463d7cd0921c0904382606d')
PARENT_CPU = (4165800, 'f253584fcc0fde7087614c2d9469f37ff950aeb48825fc64d1be1614d6596602')
PARENT_ELF = (14281472, 'c1c90ba29f3e77b8111dee1a04d9d35f91cbff6abb3e914a9b75f599378bc7e4')
IMAGES = {
    'projection_image': (10864, '25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25'),
    'guarded_image': (28440, 'de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66'),
    'prefix_image': (54344, '29fd58e7b09fee003ed31660e6201c20a7eb5da4b873d0b4d954993f7018f2f8'),
    'tiles_image': (33320, 'b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589'),
}
PROMPT = {
    'manifest': (21318, '30d047aafbd7de2b94647154680b0b9d9fb75aae2fd90b081afe0de595a88600'),
    'text': (11224, 'a43ef3619cb96c9c23d020e07630493ced7a588088033a53f1d612448375751a'),
    'tokens': (8192, '2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02'),
}
INPUTS = {}
DEADLINE = float('inf')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)

def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))

def read(path, expected=None, limit=16 << 20, retain=True):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical data input')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= limit, 'bounded ordinary data input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    digest = hashlib.sha256(); parts = []
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    with os.fdopen(fd, 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed at open')
        while True:
            require(time.monotonic() < DEADLINE, 'data-only whole deadline')
            chunk = stream.read(1 << 20)
            if not chunk: break
            digest.update(chunk)
            if retain: parts.append(chunk)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed while read')
    require(stamp(path.lstat()) == stamp(before), 'input changed after read')
    pin = dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())
    if expected is not None:
        require(pin == expected, 'exact data input pin')
    require(str(path) not in INPUTS or INPUTS[str(path)] == pin, 'conflicting data pin')
    INPUTS[str(path)] = pin
    return b''.join(parts), pin

def extent(pin, expected):
    require((pin['bytes'], pin['sha256']) == expected, 'qualified extent/hash')

def rust(pin):
    return dict(path=pin['path'], bytes=pin['bytes'], sha256=list(bytes.fromhex(pin['sha256'])))

def from_rust(value):
    require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}
            and type(value['bytes']) is int and value['bytes'] > 0
            and type(value['sha256']) is list and len(value['sha256']) == 32
            and all(type(v) is int and 0 <= v <= 255 for v in value['sha256']), 'Rust FilePin')
    return dict(path=value['path'], bytes=value['bytes'], sha256=bytes(value['sha256']).hex())

def cpu(value, schema, phases):
    require(value['schema'] == schema and value['passed'] is True and value['failure'] is None
            and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['input_sources'] == value['final_sources'] and value['gpu_execution'] is False
            and len(value['phases']) == phases, 'actual successful qualified CPU receipt')
    for row in value['phases']:
        require(type(row['exit_code']) is int and row['exit_code'] == 0 and row['natural_exit'] is True
                and row['reaped'] is True and row['process_group_absent'] is True
                and row['forced_cleanup'] is False and row['timed_out'] is False
                and row['exception'] is None and row['storage_failure'] is None, 'CPU natural clean lifecycle')
    for name in ('controller', 'input_manifest'):
        read(value[name]['path'], value[name], retain=False)
    pin = value['raw']['sources-after.json']
    require(parse(read(pin['path'], pin)[0]) == value['final_sources'], 'actual CPU source body join')

def bytes_json(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()

def write(path, raw):
    require(len(raw) <= 65536, 'bounded generated data body')
    with path.open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    pin = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    read(path, pin)
    return pin


def diagnostic_cpu_contract(worker, parent, worker_cpu, worker_sources):
    """Fixed diagnostic-mode extension to the inherited CPU/product admission."""
    feature = 'engineering-currentness-duration-diagnostics'
    parent_features = ['guarded-mlp-readiness-engineering', 'guarded-mlp-full2303-engineering', feature]
    compact = lambda row: {key: row[key] for key in ('bytes', 'sha256')}
    for value, generation in ((worker, 'currentness-duration-diagnostic-coupled-v2'),
                              (parent, 'currentness-duration-diagnostic-parent-v2')):
        require(value['qualification_mode'] == 'diagnostic'
                and value['source_generation'] == generation
                and value['currentness_duration_diagnostic_build'] is True
                and value['currentness_duration_native_execution'] is False,
                'only actual diagnostic V2 qualification admits instrumentation')
        require(compact(value['readset']['worker-proposal.json']) == DURATION_WORKER_SOURCE
                and compact(value['readset']['runtime-proposal.json']) == DURATION_RUNTIME_SOURCE,
                'exact reviewed repaired worker and runtime source lineage')
    require(worker['selected_runtime_features'] == sorted(['default', 'engineering-gfx950', feature])
            and worker['selected_worker_features'] == [feature]
            and worker['default_check_no_default_features'] is True
            and worker['default_check_diagnostic_feature_requested'] is True
            and worker['currentness_duration_runtime_source_added'] is True
            and worker['currentness_duration_worker_source_added'] is True
            and worker['runtime_source_changed'] is True
            and worker['artifacts']['worker']['cargo_artifact']['features'] == [feature],
            'feature-enabled tested worker and runtime, never default-mode substitution')
    require(parent['currentness_duration_parent_source_added'] is True
            and parent['selected_parent_features'] == parent_features
            and parent['artifacts']['ferric-qwen3-guarded-mlp-readiness-engineering']['cargo_artifact']['features']
                == sorted(parent_features + ['guarded-mlp-model-engineering', 'tp-batch-engineering', 'tp-engineering'])
            and parent['worker_qualification'] == compact(worker_cpu)
            and parent['worker_source_manifest'] == compact(worker_sources),
            'diagnostic parent compiled against the exact qualified worker sources')


def main():
    global DEADLINE
    start = time.monotonic(); DEADLINE = start + 120
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 5,
            'python3 -B prepare_model_inputs.py PARENT_RECEIPT SHA PARENT_ELF HISTORICAL_REQUEST')
    require(Path(__file__).resolve().parent == ROOT and ROOT.resolve(strict=True) == ROOT
            and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'exact unprivileged deployment root')
    os.umask(0o077); os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0); require(priority in (0, 10), 'unexpected nice')
    if priority == 0: os.nice(10)
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_FSIZE, 1 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        ceiling = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (ceiling, ceiling))
    def timeout(_number, _frame): raise RuntimeError('data-only preparation deadline')
    for number in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, timeout)
    signal.setitimer(signal.ITIMER_REAL, 120)
    require(all(type(p) is tuple and len(p) == 2 for p in (WORKER_CPU, WORKER_ELF, PARENT_CPU, PARENT_ELF, WORKER_SOURCES, PARENT_SOURCES)),
            'actual readiness CPU/ELF pins remain pending')
    require(re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'actual parent SHA')
    require(not any(os.path.lexists(ROOT / n) for n in ('tail_duration-request.json', 'tail_duration-input.json', 'prepared-inputs.json', *ORDER)), 'fresh one-shot preparation and native namespace')
    parent_raw, parent_pin = read(sys.argv[1]); extent(parent_pin, PARENT_CPU)
    require(parent_pin['sha256'] == sys.argv[2]
            and parent_pin['path'] == str(PARENT_ROOT / 'evidence/complete.json'),
            'supplied actual census parent terminal')
    parent = parse(parent_raw); cpu(parent, 'ferric-guarded-mlp-currentness-duration-parent-cpu-v1', 69)
    require(len(parent['tests']) == 59 and len(parent['inventory']) == 1058 and len(parent['final_sources']) == 1302
            and sum(v['passed'] for v in parent['tests'].values()) == 595
            and all(v['failed'] == v['ignored'] == 0 for v in parent['tests'].values())
            and parent['all_selected_parent_tests_executed'] is True
            and parent['full_parent_library_suite_executed'] is False and len(parent['artifacts']) == 7
            and parent['position5_diagnostic_parent_route_added'] is True
            and parent['position5_native_execution'] is False
            and parent['inherited_readiness_parent_route_preserved'] is True and parent['readiness_native_execution'] is False
            and parent['full_long_workload'] is False, 'actual selected readiness parent qualification')
    require(parent['readiness40_bank_scoped_warm_parent_source_added'] is True
            and parent['readiness40_bank_scoped_warm_native_execution'] is False
            and parent['allocation_preflights_changed'] is True
            and parent['allocation_preflights_changed_only_for_explicit_warm_position5'] is False
            and parent['allocation_preflights_changed_only_for_explicit_warm_position5_or_full'] is True
            and parent['full2303_bank_scoped_census_parent_source_added'] is True
            and parent['full2303_bank_scoped_census_policy_prepublication_checked'] is True
            and parent['full2303_bank_scoped_census_native_execution'] is False
            and parent['full2303_deadline_changed'] is False
            and parent['readiness40_bank_scoped_census_tail_parent_source_added'] is True
            and parent['readiness40_bank_scoped_census_tail_policy_prepublication_checked'] is True
            and parent['readiness40_bank_scoped_census_tail_native_execution'] is False
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
    artifact = parent['artifacts'][PARENT_NAME]
    require(artifact['pin']['path'] == str(Path(sys.argv[3]))
            and artifact['cargo_artifact']['target']['name'] == PARENT_NAME
            and artifact['cargo_artifact']['target']['kind'] == ['bin']
            and artifact['cargo_artifact']['profile']['test'] is False
            and artifact['cargo_artifact']['executable'] == artifact['pin']['path'], 'new readiness Cargo product')
    elf, parent_elf = read(sys.argv[3], artifact['pin'], limit=128 << 20); extent(parent_elf, PARENT_ELF)
    require(elf[:6] == b'\x7fELF\x02\x01' and elf[18:20] == b'\x3e\x00', 'actual parent ELF')
    worker_raw, worker_pin = read(WORKER_ROOT / 'evidence/complete.json'); extent(worker_pin, WORKER_CPU)
    worker = parse(worker_raw); cpu(worker, 'ferric-guarded-mlp-currentness-duration-cpu-v1', 27)
    require(len(worker['final_sources']) == 1069
            and worker['source_generation'] == 'currentness-duration-diagnostic-coupled-v2'
            and worker['runtime_source_changed'] is True
            and parent['source_generation'] == 'currentness-duration-diagnostic-parent-v2'
            and parent['shared_test_fixture_repaired'] is True
            and worker['controller']['path'] == str(WORKER_ROOT / 'run_cpu.py')
            and parent['controller']['path'] == str(PARENT_ROOT / 'run_cpu.py'),
            'census coupled source extent and parent controller identity')
    require(set(worker['inventories']) == {'runtime', 'worker'}
            and len(worker['inventories']['runtime']) == 1200
            and len(worker['inventories']['worker']) == 857
            and worker['full_runtime_tests_executed'] is True
            and worker['full_worker_tests_executed'] is True
            and worker['tests']['kfd-tests']['passed'] == 1192
            and worker['tests']['kfd-tests']['failed'] == 0
            and worker['tests']['kfd-tests']['ignored'] == 8
            and worker['tests']['worker-tests']['passed'] == 853
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
                'inherited_scoped_currentness_runtime_preserved', 'shared_test_fixture_repaired',
                'full_model_long_request_enabled', 'full2303_scoped_warm_source_added',
                'inherited_bank_scoped_currentness_preserved', 'scoped_capacity_census_runtime_source_added',
                'readiness40_bank_scoped_census_source_added', 'allocation_preflights_changed',
                'inherited_readiness40_bank_scoped_census_preserved',
                'full2303_bank_scoped_census_source_added', 'scoped_tail_runtime_source_added',
                'readiness40_bank_scoped_census_tail_source_added'))
            and all(worker[k] is False for k in ('scoped_currentness_native_execution',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'arena_policy_changed', 'default_fresh_arena_policy_changed', 'ordinary_read_api_changed',
                'public_runtime_limits_changed', 'lockfiles_changed', 'shared_cache_changed',
                'readiness40_bank_scoped_warm_native_execution', 'readiness40_bank_scoped_census_native_execution',
                'full2303_scoped_warm_native_execution', 'full2303_bank_scoped_census_native_execution',
                'readiness40_bank_scoped_census_tail_native_execution', 'full2303_deadline_changed',
                'full2303_launch_admitted', 'gpu_qualified',
                'numerical_acceptance', 'performance_claim', 'production_authority')),
            'actual explicit scoped source, preserved defaults and genuine executable tests')
    source_pins = {}
    for name, value, root, expected in [('worker', worker, WORKER_ROOT, WORKER_SOURCES),
                                       ('parent', parent, PARENT_ROOT, PARENT_SOURCES)]:
        source_raw, source_pin = read(root / 'evidence/sources-after.json')
        extent(source_pin, expected)
        require(source_pin == value['raw']['sources-after.json']
                and parse(source_raw) == value['final_sources'], 'qualified source-map original body')
        source_pins[name] = source_pin
    diagnostic_cpu_contract(worker, parent, worker_pin, source_pins['worker'])
    prefix = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
    compact = lambda value: {k: {n: v[n] for n in ('bytes', 'sha256')}
        for k, v in value['final_sources'].items() if k.startswith(prefix)}
    require(len(compact(worker)) == 236 and compact(worker) == compact(parent),
            'all236 parent-compiled worker bodies equal actual standalone worker')
    _, worker_elf = read(worker['artifacts']['worker']['pin']['path'], worker['artifacts']['worker']['pin'], retain=False)
    extent(worker_elf, WORKER_ELF)
    old_raw, old_pin = read(sys.argv[4], limit=16384); extent(old_pin, OLD_REQUEST)
    old = parse(old_raw)
    require(set(old) == {'schema', 'decode', 'projection_image', 'guarded_image'}
            and old['schema'] == 'FerricFiniteGuardedMlpDecodeRequestV1', 'authentic historical input container only')
    d = old['decode']; source = Path(d['source'])
    require(source.is_dir() and source.resolve(strict=True) == source, 'canonical authentic model source')
    prompt = {}
    for name, value in d['prompt'].items():
        pin = from_rust(value); extent(pin, PROMPT[name]); prompt[name] = read(pin['path'], pin)[0]
    require(set(prompt) == set(PROMPT), 'three original prompt bodies')
    tokens = list(struct.unpack('<2048I', prompt['tokens']))
    require(tokens == parse(prompt['manifest'])['input_token_ids']
            and all(type(v) is int and 0 <= v < 151936 for v in tokens), 'all2048 authentic token encodings')
    for value in d['images'].values():
        pin = from_rust(value); read(pin['path'], pin, retain=False)
    images = {name: copy.deepcopy(old[name] if name in old else d[name]) for name in IMAGES}
    for name, value in images.items():
        pin = from_rust(value); extent(pin, IMAGES[name]); read(pin['path'], pin, retain=False)
    requests = {}; plans = {}; sessions = []
    generator = read(Path(__file__).resolve(), retain=False)[1]
    for pin in list(INPUTS.values()): read(pin['path'], pin, retain=False)
    for case in ORDER:
        session = os.urandom(32)
        require(session != bytes(32) and list(session) != d['session']
                and list(session) not in sessions, 'fresh diagnostic readiness session')
        sessions.append(list(session))
        base = {k: copy.deepcopy(v) for k, v in d.items() if k not in ('mode', 'prefix_image', 'tiles_image')}
        base.update(schema='FerricFiniteLongRequestV1', worker=rust(worker_elf), session=list(session),
                    evidence_directory=str(ROOT / case / 'native'))
        require(set(base) == {'schema', 'source', 'worker', 'images', 'expected_bundle_id', 'expected_model_id',
            'device_ids', 'session', 'prompt', 'evidence_directory', 'dispatch_timeout_ms', 'child_deadline_ms'},
            'no four-forward selector transferred into closed readiness config')
        require(base['device_ids'] == [16366993098680759275, 10838076764495710945]
                and base['dispatch_timeout_ms'] == 10000 and base['child_deadline_ms'] == 3600000,
                'unchanged device/deadline bounds')
        request = dict(schema='FerricGuardedMlpReadiness40Position5RequestV1', base=base, **images)
        requests[case] = write(ROOT / (case + '-request.json'), bytes_json(request))
        plan = dict(schema='ferric-guarded-mlp-readiness40-tail-currentness-duration-gpu-input-v1',
                    case=case, profile='readiness40_position5', parent_cpu=parent_pin, worker_cpu=worker_pin,
                    parent=parent_elf, worker=worker_elf, parent_sources=source_pins['parent'],
                    worker_sources=source_pins['worker'], request=requests[case])
        plans[case] = write(ROOT / (case + '-input.json'), bytes_json(plan))
    for pin in list(INPUTS.values()): read(pin['path'], pin, retain=False)
    receipt = dict(schema='ferric-guarded-mlp-readiness40-tail-currentness-duration-input-preparation-v1', passed=True,
        controller=generator, historical_request=old_pin, parent_cpu=parent_pin, worker_cpu=worker_pin,
        readset=INPUTS, requests=requests, plans=plans, prompt_tokens=2048, prompt_positions_requested=40,
        generated_tokens_requested=0, capture_positions=[0, 5, 16, 39], sessions=sessions,
        modes=list(ORDER), instrumented=True, matched_speed_comparison=False, fresh_case_namespaces=True,
        elapsed_seconds=time.monotonic() - start, data_only=True, cpu_tests_executed=False,
        parent_host_timing_requested=True, gpu_timing=False,
        currentness_policies=dict(tail_duration='bank_scoped_census_tail_duration'),
        allocation_preflights_changed=True, allocation_preflights_changed_only_for_candidate=False,
        scoped_tail_requested=True, tail_counters_are_independent=True, bank_guarded_body_includes_callbacks=True,
        census_counters_are_layer_subset=True,
        temporal_equivalent_to_full=False, shared_full_currentness_requested=False,
        cache_kernel_admission_requested=False, operational_currentness_requested=False,
        paired_read_requested=False, paired_terminal_requested=False,
        ordinary_wire_schema_changed=False, ordinary_observation_schema_changed=False,
        native_execution=False, gpu_execution=False, full_long_workload=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
    result = write(ROOT / 'prepared-inputs.json', bytes_json(receipt))
    signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(receipt=result, requests=requests, plans=plans), sort_keys=True)); return 0


if __name__ == '__main__':
    raise SystemExit(main())
