"""Data-only authentic full-prompt Readiness40 causal layer-zero request preparation; no native execution."""
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
ROOT = E / 'guarded-mlp-readiness40-causal-layer0-gpu-v228-v3'
WORKER_ROOT = E / 'guarded-mlp-readiness40-causal-layer0-worker-cpu-v228-v1'
PARENT_NAME = 'ferric-qwen3-guarded-mlp-readiness-engineering'
OLD_REQUEST = (9338, '526f0c30630edb83114cfa763ffb98b9242154371250797b9ed55bf8747bed1d')
WORKER_CPU = (1731960, '5c4ffff4b06c7ad1d73c6985c6bcb0193b596ab6618bbda44010347a9b26aa1a')
WORKER_ELF = (6164328, '44be19f70ff77cbb95651667af4038be4c861502774ae47600aa2da08396494e')
PARENT_CPU = (3942470, 'f2c161753e64f9f2b8b4049ae75cac13332b7eb14217eaad247c8c2e9e13d4ea')
PARENT_ELF = (13763616, 'a8ada7572717e931bd9149bcab6d043780d3e8513fb5cd3282d8f3c36092b8aa')
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
    require(all(type(p) is tuple and len(p) == 2 for p in (WORKER_CPU, WORKER_ELF, PARENT_CPU, PARENT_ELF)),
            'actual readiness CPU/ELF pins remain pending')
    require(re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'actual parent SHA')
    require(not any(os.path.lexists(ROOT / n) for n in ('readiness-request.json', 'readiness-input.json',
            'prepared-inputs.json', 'readiness')), 'fresh one-shot preparation and native namespace')
    parent_raw, parent_pin = read(sys.argv[1]); extent(parent_pin, PARENT_CPU)
    require(parent_pin['sha256'] == sys.argv[2], 'supplied actual parent terminal')
    parent = parse(parent_raw); cpu(parent, 'ferric-guarded-mlp-readiness40-causal-layer0-parent-cpu-v2', 60)
    require(len(parent['tests']) == 51 and len(parent['inventory']) == 918 and len(parent['final_sources']) == 1244
            and sum(v['passed'] for v in parent['tests'].values()) == 444
            and all(v['failed'] == v['ignored'] == 0 for v in parent['tests'].values())
            and parent['all_selected_parent_tests_executed'] is True
            and parent['full_parent_library_suite_executed'] is False and len(parent['artifacts']) == 6
            and parent['causal_layer0_parent_route_added'] is True
            and parent['causal_layer0_native_execution'] is False
            and parent['parent_causal_file_reads_retained'] is True
            and parent['previous_parent_failure_preserved'] is True
            and parent['causal_capture_positions'] == list(range(6)) and parent['causal_layer'] == 0
            and parent['inherited_warm_paired_terminal_parent_route_preserved'] is True
            and parent['position5_diagnostic_parent_route_added'] is True
            and parent['position5_native_execution'] is False
            and parent['inherited_readiness_parent_route_preserved'] is True and parent['readiness_native_execution'] is False
            and parent['full_long_workload'] is False, 'actual selected readiness parent qualification')
    artifact = parent['artifacts'][PARENT_NAME]
    require(artifact['pin']['path'] == str(Path(sys.argv[3]))
            and artifact['cargo_artifact']['target']['name'] == PARENT_NAME
            and artifact['cargo_artifact']['target']['kind'] == ['bin']
            and artifact['cargo_artifact']['profile']['test'] is False
            and artifact['cargo_artifact']['executable'] == artifact['pin']['path'], 'new readiness Cargo product')
    elf, parent_elf = read(sys.argv[3], artifact['pin'], limit=128 << 20); extent(parent_elf, PARENT_ELF)
    require(elf[:6] == b'\x7fELF\x02\x01' and elf[18:20] == b'\x3e\x00', 'actual parent ELF')
    worker_raw, worker_pin = read(WORKER_ROOT / 'evidence/complete.json'); extent(worker_pin, WORKER_CPU)
    worker = parse(worker_raw); cpu(worker, 'ferric-guarded-mlp-readiness40-causal-layer0-worker-cpu-v1', 9)
    require(len(worker['inventory']) == 684 and len(worker['final_sources']) == 1014
            and worker['tests']['worker-tests']['passed'] == 680
            and worker['tests']['worker-tests']['failed'] == 0 and worker['tests']['worker-tests']['ignored'] == 4
            and worker['full_worker_tests_executed'] is True and len(worker['artifacts']) == 5
            and worker['causal_layer0_source_added'] is True
            and worker['causal_layer0_native_execution'] is False
            and worker['causal_capture_positions'] == list(range(6)) and worker['causal_layer'] == 0
            and worker['inherited_warm_paired_terminal_route_preserved'] is True
            and worker['runtime_source_qualified_by_terminal_cpu'] is True
            and worker['position5_diagnostic_source_added'] is True
            and worker['position5_native_execution'] is False
            and worker['inherited_readiness40_profile_preserved'] is True
            and worker['capture_count_changed'] is False
            and worker['readiness_cli_source_added'] is True and worker['readiness_owner_source_added'] is True
            and worker['pure_long_sequence_source_added'] is True and worker['readiness40_native_execution'] is False
            and worker['full2303_native_enabled'] is False and worker['existing_ar4_limits_changed'] is False
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin'],
            'actual full worker qualification including real executable readiness test')
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
    session = os.urandom(32)
    require(session != bytes(32) and list(session) != d['session'], 'new readiness session')
    base = {k: copy.deepcopy(v) for k, v in d.items() if k not in ('mode', 'prefix_image', 'tiles_image')}
    base.update(schema='FerricFiniteLongRequestV1', worker=rust(worker_elf), session=list(session),
                evidence_directory=str(ROOT / 'readiness/native'))
    require(set(base) == {'schema', 'source', 'worker', 'images', 'expected_bundle_id', 'expected_model_id',
        'device_ids', 'session', 'prompt', 'evidence_directory', 'dispatch_timeout_ms', 'child_deadline_ms'},
        'no four-forward selector transferred into closed readiness config')
    require(base['device_ids'] == [16366993098680759275, 10838076764495710945]
            and base['dispatch_timeout_ms'] == 10000 and base['child_deadline_ms'] == 3600000,
            'unchanged device/deadline bounds')
    request = dict(schema='FerricGuardedMlpReadiness40CausalLayerZeroRequestV1', base=base, **images)
    generator = read(Path(__file__).resolve(), retain=False)[1]
    for pin in list(INPUTS.values()): read(pin['path'], pin, retain=False)
    req_pin = write(ROOT / 'readiness-request.json', bytes_json(request))
    plan = dict(schema='ferric-guarded-mlp-readiness40-causal-layer0-gpu-input-v1', case='readiness', profile='readiness40_position5',
                parent_cpu=parent_pin, worker_cpu=worker_pin, parent=parent_elf, worker=worker_elf, request=req_pin)
    plan_pin = write(ROOT / 'readiness-input.json', bytes_json(plan))
    for pin in list(INPUTS.values()): read(pin['path'], pin, retain=False)
    receipt = dict(schema='ferric-guarded-mlp-readiness40-causal-layer0-input-preparation-v1', passed=True,
        controller=generator, historical_request=old_pin, parent_cpu=parent_pin, worker_cpu=worker_pin,
        readset=INPUTS, request=req_pin, plan=plan_pin, prompt_tokens=2048, prompt_positions_requested=40,
        generated_tokens_requested=0, capture_positions=[0, 5, 16, 39], session=list(session),
        causal_capture_positions=list(range(6)), causal_layer=0,
        elapsed_seconds=time.monotonic() - start, data_only=True, cpu_tests_executed=False,
        native_execution=False, gpu_execution=False, full_long_workload=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
    result = write(ROOT / 'prepared-inputs.json', bytes_json(receipt))
    signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(receipt=result, request=req_pin, plan=plan_pin), sort_keys=True)); return 0


if __name__ == '__main__':
    raise SystemExit(main())
