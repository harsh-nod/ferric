"""Bounded data-only TF4/AR4 request generation from an actual qualified parent."""
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-model-gpu-v228-v2'
WORKER_ROOT = E / 'guarded-mlp-worker-cpu-v228-v6'
PARENT_NAME = 'ferric-qwen3-finite-guarded-mlp-decode-engineering'
OLD_REQUEST = (8818, '44e9a717750c70ea4c6316df1aae4fcd9abd386759188fb59b443f04a6a6c3ab')
WORKER_CPU = (1559924, '927e6519923ab44aa5f5616886ce9b539ed5b5f77804a48fe5d42f6a37974cd2')
WORKER_ELF = (5727168, 'ac265ab2545a534c228edcb926458f95cca20243ba106b17eb2f60f58f6552ff')
IMAGES = {
    'projection_image': (10864, '25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25'),
    'guarded_image': (28440, 'de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66'),
    'prefix_image': (54344, '29fd58e7b09fee003ed31660e6201c20a7eb5da4b873d0b4d954993f7018f2f8'),
    'tiles_image': (33320, 'b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589'),
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
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 6,
            'python3 -B prepare_model_inputs.py PARENT_RECEIPT SHA PARENT_ELF HISTORICAL_REQUEST GUARDED_IMAGE')
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
    def timeout(_number, _frame): raise RuntimeError('data-only input preparation deadline')
    signal.signal(signal.SIGALRM, timeout); signal.setitimer(signal.ITIMER_REAL, 120)
    require(re.fullmatch('[0-9a-f]{64}', sys.argv[2]), 'observed parent terminal SHA')
    output_names = [mode + suffix for mode in ('tf4', 'ar4') for suffix in ('-request.json', '-input.json')]
    require(not any(os.path.lexists(ROOT / name) for name in (*output_names, 'prepared-inputs.json', 'tf4', 'ar4')),
            'fresh requests, plans and unopened case directories')
    parent_raw, parent_cpu_pin = read(sys.argv[1]); require(parent_cpu_pin['sha256'] == sys.argv[2], 'actual parent receipt SHA')
    parent = parse(parent_raw); cpu(parent, 'ferric-guarded-mlp-parent-cpu-v1', 54)
    require(len(parent['tests']) == 46 and sum(row['passed'] for row in parent['tests'].values()) == 375
            and all(row['failed'] == row['ignored'] == 0 for row in parent['tests'].values())
            and parent['all_selected_parent_tests_executed'] is True
            and parent['full_parent_library_suite_executed'] is False and len(parent['artifacts']) == 5,
            'actual parent selected375-pass/46-scope qualification')
    artifact = parent['artifacts'][PARENT_NAME]
    require(artifact['pin']['path'] == str(Path(sys.argv[3]))
            and artifact['cargo_artifact']['target']['name'] == PARENT_NAME
            and artifact['cargo_artifact']['target']['kind'] == ['bin']
            and artifact['cargo_artifact']['profile']['test'] is False
            and artifact['cargo_artifact']['executable'] == artifact['pin']['path'], 'supplied parent is qualified Cargo ELF')
    elf, parent_pin = read(sys.argv[3], artifact['pin'], limit=128 << 20)
    require(elf[:6] == b'\x7fELF\x02\x01' and elf[18:20] == b'\x3e\x00', 'actual parent ELF header')
    worker_raw, worker_cpu_pin = read(WORKER_ROOT / 'evidence/complete.json'); extent(worker_cpu_pin, WORKER_CPU)
    worker = parse(worker_raw); cpu(worker, 'ferric-guarded-mlp-worker-cpu-v1', 9)
    require(worker['tests']['worker-tests']['passed'] == 586 and worker['tests']['worker-tests']['ignored'] == 4
            and worker['tests']['worker-tests']['failed'] == 0 and worker['full_worker_tests_executed'] is True,
            'exact V6 worker census')
    _, worker_pin = read(worker['artifacts']['worker']['pin']['path'], worker['artifacts']['worker']['pin'], retain=False)
    extent(worker_pin, WORKER_ELF)
    old_raw, old_pin = read(sys.argv[4], limit=16384); extent(old_pin, OLD_REQUEST)
    old = parse(old_raw)
    require(set(old) == {'schema', 'decode', 'projection_residual_image'}
            and old['schema'] == 'FerricFiniteProjectionResidualMlpOrderedRequestV1', 'historical ordered request')
    source = Path(old['decode']['source'])
    require(source.is_dir() and source.resolve(strict=True) == source, 'existing canonical model source')
    for row in [*old['decode']['images'].values(), *old['decode']['prompt'].values(),
                old['decode']['prefix_image'], old['decode']['tiles_image'], old['projection_residual_image']]:
        pin = from_rust(row); read(pin['path'], pin, retain=False)
    for name, row in [('prefix_image', old['decode']['prefix_image']), ('tiles_image', old['decode']['tiles_image']),
                      ('projection_image', old['projection_residual_image'])]:
        extent(from_rust(row), IMAGES[name])
    _, guarded_pin = read(sys.argv[5], retain=False); extent(guarded_pin, IMAGES['guarded_image'])
    generator_pin = read(Path(__file__).resolve(), retain=False)[1]
    prepared = {}; sessions = set()
    for mode, wire_mode in [('tf4', 'teacher_forced'), ('ar4', 'autoregressive')]:
        session = os.urandom(32)
        require(session != bytes(32) and session not in sessions and list(session) != old['decode']['session'],
                'distinct fresh nonzero diagnostic session')
        sessions.add(session)
        decode = copy.deepcopy(old['decode'])
        decode.update(worker=rust(worker_pin), mode=wire_mode, session=list(session), evidence_directory=str(ROOT / mode / 'native'))
        require({key: value for key, value in decode.items() if key not in ('worker', 'mode', 'session', 'evidence_directory')}
                == {key: value for key, value in old['decode'].items() if key not in ('worker', 'mode', 'session', 'evidence_directory')},
                'historical source/prompt/setup inputs unchanged')
        request = dict(schema='FerricFiniteGuardedMlpDecodeRequestV1', decode=decode,
                       projection_image=old['projection_residual_image'], guarded_image=rust(guarded_pin))
        request_raw = bytes_json(request); require(len(request_raw) <= 16384, 'parent request bound')
        request_path = ROOT / (mode + '-request.json')
        request_pin = dict(path=str(request_path), bytes=len(request_raw), sha256=hashlib.sha256(request_raw).hexdigest())
        plan = dict(schema='ferric-guarded-mlp-model-gpu-input-v1', mode=mode, parent_cpu=parent_cpu_pin,
                    worker_cpu=worker_cpu_pin, parent=parent_pin, worker=worker_pin, request=request_pin)
        prepared[mode] = dict(request=request_pin, request_body=request_raw, plan_body=bytes_json(plan), session=list(session))
    for pin in list(INPUTS.values()): read(pin['path'], pin, retain=False)
    generated = {}
    for mode, value in prepared.items():
        generated[mode] = dict(request=write(Path(value['request']['path']), value['request_body']),
            plan=write(ROOT / (mode + '-input.json'), value['plan_body']), session=value['session'])
    for pin in list(INPUTS.values()): read(pin['path'], pin, retain=False)
    receipt = dict(schema='ferric-guarded-mlp-model-input-preparation-v1', passed=True,
        controller=generator_pin, historical_request=old_pin, parent_cpu=parent_cpu_pin, worker_cpu=worker_cpu_pin,
        readset=INPUTS, generated=generated, elapsed_seconds=time.monotonic() - start,
        data_only=True, cpu_tests_executed=False, native_execution=False, gpu_execution=False,
        numerical_acceptance=False, full_model_acceptance=False, performance_claim=False, production_authority=False)
    pin = write(ROOT / 'prepared-inputs.json', bytes_json(receipt))
    signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(receipt=pin, generated=generated), sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())

