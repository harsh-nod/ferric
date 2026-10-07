"""Bounded MI350 CPU-only exact-head diagnostic using already observed evidence."""
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import signal
import socket
import stat
import struct
import sys
import time
import types
import unittest

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-position5-head-exact-v228-v1')
MODEL = Path('/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target')
SHARD = 'model-00005-of-00005.safetensors'
INDEX = 'model.safetensors.index.json'
MODEL_PINS = {
    SHARD: dict(bytes=1244659840, sha256='20c2d6366ab85c90786ccdd829cd2b9e7d30ef3b2ebbb998280e7e4014b542ff'),
    INDEX: dict(bytes=32878, sha256='f9fdbcb91c23971c13ec5d5f2573d2349e8f61f2f049371ec699281748fdb1bc'),
}
INPUTS = {
    'native-terminal.json': dict(bytes=171456, sha256='5b9617aba0b588c33923c5cc458b013d0929828be5635ea75f3fd702a12c2cd9'),
    'native-summary.json': dict(bytes=21394, sha256='1ac8dda60bc2234fa640b5cb1c851868bebb1e44898ae4a76ea8cf45138f6078'),
    'native-capture-5.bin': dict(bytes=849800, sha256='f0fbe98f9b1c01fafd0f70fef346408d108cf2616e73690b7b3b697a6ebead3b'),
    'native-request.json': dict(bytes=9173, sha256='d0ad15ae0ccb90109fe92da6fab8281745da479cf77e68af5a10abeb0988eee4'),
    'reference-terminal.json': dict(bytes=222675, sha256='d256b149229d4ba94a1e4f9a3aa2d6f9354c56a721bfe1dc0cb69bbb7c49674f'),
    'reference-pass1-pos5.bf16': dict(bytes=606976, sha256='64dcde3a548c3bc93d0086a212465828be460a902034e43809f68ba7b6c5df8e'),
    'reference-pass2-pos5.bf16': dict(bytes=606976, sha256='64dcde3a548c3bc93d0086a212465828be460a902034e43809f68ba7b6c5df8e'),
    'comparison-terminal.json': dict(bytes=32247, sha256='e6dbc4712f6a04e11be40b70b09f46bf388cdbd26150e6e2fec1c6e2cac12578'),
}
TEST_NAMES = ['test_decimal_is_exact_not_float', 'test_dot_cancellation_and_exact_products', 'test_exact_decode_matches_fraction',
    'test_four_dots_and_delta_decomposition', 'test_fraction_nearest_independent_local_oracle',
    'test_header_rejects_shape_dtype_extent_and_duplicate', 'test_overflow_boundary',
    'test_payload_extent_and_nonfinite', 'test_roundtrip_finite_samples',
    'test_shard_layout_two_row_offsets', 'test_subnormal_normal_and_signed_underflow',
    'test_ties_even_and_signs']
WHOLE_SECONDS, CPU_SECONDS, MEMORY_BYTES = 180, 120, 512 << 20
DEADLINE = float('inf')
READSET = {}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def guard():
    remaining = DEADLINE - time.monotonic()
    if remaining <= 0:
        raise TimeoutError('whole 180-second diagnostic deadline')
    signal.setitimer(signal.ITIMER_REAL, remaining)


def interrupted(number, _frame):
    raise TimeoutError('diagnostic signal %d' % number)


def stamp(s):
    return (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_uid, s.st_gid,
            s.st_size, s.st_mtime_ns, s.st_ctime_ns)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return dict(bytes=row['bytes'], sha256=row['sha256'])


def read(path, expected, cap=2 << 20):
    guard()
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink >= 1
                and 0 <= before.st_size <= cap, 'bounded regular input')
        raw = stream.read(cap + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size,
            'input identity changed')
    value = pin(raw)
    require(value == expected, 'input pin ' + str(path))
    row = dict(path=str(path), **value)
    require(str(path) not in READSET or READSET[str(path)] == row, 'input drift')
    READSET[str(path)] = row
    guard()
    return raw


def save(path, raw):
    guard()
    require(type(raw) is bytes and len(raw) <= 2 << 20, 'bounded output')
    with path.open('xb') as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    guard()
    return pin(raw)


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def load(name, raw, filename):
    module = types.ModuleType(name)
    module.__file__ = str(filename)
    sys.modules[name] = module
    exec(compile(raw, str(filename), 'exec'), module.__dict__)
    return module


def normalized(row):
    digest = row['sha256']
    if type(digest) is list:
        require(len(digest) == 32 and all(type(n) is int and 0 <= n <= 255 for n in digest), 'digest octets')
        digest = bytes(digest).hex()
    require(type(row['bytes']) is int and row['bytes'] >= 0 and type(digest) is str
            and len(digest) == 64 and all(c in '0123456789abcdef' for c in digest), 'data pin')
    return dict(bytes=row['bytes'], sha256=digest)


def admit(H, bodies):
    native = H.parse(bodies['native-terminal.json'])
    summary = H.parse(bodies['native-summary.json'])
    request = H.parse(bodies['native-request.json'])
    reference = H.parse(bodies['reference-terminal.json'])
    comparison = H.parse(bodies['comparison-terminal.json'])
    require(native['passed'] is True and native['errors'] == [] and native['native_attempts'] == 1
            and native['observation']['all40_transcript_checked'] is True, 'actual native admission')
    require(summary['native_closed'] is True and summary['completed_forwards'] == 40
            and summary['generated_tokens'] == [] and summary['request'] == request, 'native Close/request')
    require(reference['passed'] is True and reference['error'] is None and reference['postcheck_errors'] == []
            and reference['repeat_gate_passed'] is True and reference['selected_positions'] == [0, 5, 16, 39]
            and reference['native_intermediates_used'] is False, 'genuine repeated reference')
    require(comparison['passed'] is True and comparison['error'] is None and comparison['postcheck_errors'] == []
            and comparison['numerical_acceptance'] is False, 'actual authenticated comparison')
    checks = comparison['checks']
    require(checks['both_native_structural_admissions_rechecked'] is True
            and checks['full_original_receipts_authenticated'] is True
            and checks['full_prompt_and_model_bundle_joined'] is True
            and normalized(checks['diagnostic_native']['original_terminal']) == INPUTS['native-terminal.json'],
            'prior comparison native/source admission')
    diagnostic = checks['diagnostic']
    require(diagnostic['all40_native_pins_unchanged'] is True and diagnostic['all40_reference_pins_unchanged'] is True
            and diagnostic['common_payloads_equal'] == [0, 16, 39] and diagnostic['position'] == 5,
            'unchanged original histories and common captures')
    require(request['base']['source'] == str(MODEL.parent)
            and bytes(summary['bootstrap']['sequence']['scope']['model_id']).hex() == reference['model_id']
            and bytes(summary['bootstrap']['sequence']['scope']['bundle_id']).hex() == reference['bundle_id'],
            'same checkpoint source/model/bundle')
    for name, expected in MODEL_PINS.items():
        require(normalized(reference['model_sources'][name]) == expected, 'original model pin ' + name)
    require(normalized(native['raw']['native/complete.json']) == INPUTS['native-summary.json']
            and normalized(native['raw']['native/capture-5.bin']) == INPUTS['native-capture-5.bin'], 'actual raw joins')
    selected = [item for item in native['observation']['captures'] if item['position'] == 5]
    require(len(selected) == 1 and normalized(selected[0]['capture']) == INPUTS['native-capture-5.bin'], 'native selected capture')
    capture = bodies['native-capture-5.bin']
    require(len(capture) == H.CONTROL_BYTES + H.PAYLOAD_BYTES, 'capture control/payload boundary')
    native_payload = capture[H.CONTROL_BYTES:]
    require(H.pin(native_payload[H.LOGIT_OFFSET:]) == normalized(selected[0]['logits']), 'native logits slice')
    reference_payload = bodies['reference-pass1-pos5.bf16']
    require(reference_payload == bodies['reference-pass2-pos5.bf16'], 'actual reference payload repeat')
    for value in reference['passes']:
        row = value['cases'][5]
        require(row['position'] == 5 and row['input_token'] == 271 and row['generation'] == 6
                and row['payload'] == H.pin(reference_payload)
                and row['tensors']['final-norm'] == H.pin(reference_payload[H.FINAL_OFFSET:H.LOGIT_OFFSET]),
                'reference selected payload/finalnorm')
    return native_payload, reference_payload


def scan_shard(H):
    guard()
    path = MODEL / SHARD
    require(path.resolve(strict=True) == path, 'canonical model shard')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink >= 1
                and before.st_size == MODEL_PINS[SHARD]['bytes'], 'pinned full shard extent')
        raw_length = stream.read(8)
        require(len(raw_length) == 8, 'safetensors header length')
        length = struct.unpack('<Q', raw_length)[0]
        require(2 <= length <= 1 << 20, 'bounded safetensors header')
        raw_header = stream.read(length)
        require(len(raw_header) == length and raw_header.startswith(b'{'), 'complete JSON safetensors header')
        start, locations = H.tensor_layout(H.parse(raw_header), length, before.st_size)
        rows = {token: os.pread(stream.fileno(), count, offset) for token, (offset, count) in locations.items()}
        require(all(len(body) == 8192 for body in rows.values()), 'two complete original head rows')
        stream.seek(0)
        whole, tensor, offset = hashlib.sha256(), hashlib.sha256(), 0
        while True:
            guard()
            block = stream.read(2 << 20)
            if not block:
                break
            require(offset + len(block) <= before.st_size, 'shard growth')
            whole.update(block)
            tensor.update(block[max(0, start - offset):])
            offset += len(block)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and offset == before.st_size,
            'stable whole shard and extracted rows')
    actual = dict(bytes=offset, sha256=whole.hexdigest())
    require(actual == MODEL_PINS[SHARD], 'entire original shard SHA')
    guard()
    return rows, dict(path=str(path), **actual, header=pin(raw_header),
        tensor=dict(bytes=offset-start, sha256=tensor.hexdigest()), tensor_file_offset=start,
        rows={str(token): dict(file_offset=locations[token][0], **pin(body)) for token, body in rows.items()},
        stable_stat=list(stamp(before)))


def main():
    global DEADLINE
    started = time.monotonic()
    DEADLINE = started + WHOLE_SECONDS
    require(len(sys.argv) == 2 and len(sys.argv[1]) == 64, 'usage: run.py SOURCE_MANIFEST_SHA256')
    require(Path(__file__).resolve().parent == ROOT and ROOT.resolve(strict=True) == ROOT,
            'fresh owned source root')
    require(socket.gethostname() == 'smci350-rck-g03-b19-03' and os.getuid() == 9661, 'MI350 owner host/uid')
    for key in ('ROCR_VISIBLE_DEVICES', 'HIP_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
        os.environ[key] = ''
    os.environ['GPU_DEVICE_ORDINAL'] = ''
    os.sched_setaffinity(0, {8, 9})
    current = os.getpriority(os.PRIO_PROCESS, 0)
    if current < 10:
        os.nice(10-current)
    resource.setrlimit(resource.RLIMIT_AS, (MEMORY_BYTES, MEMORY_BYTES))
    resource.setrlimit(resource.RLIMIT_CPU, (CPU_SECONDS, CPU_SECONDS))
    resource.setrlimit(resource.RLIMIT_FSIZE, (2 << 20, 2 << 20))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    for number in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, interrupted)
    guard()
    out = ROOT / 'output'
    require(not out.exists(), 'fresh output; never retry/overwrite')
    out.mkdir(mode=0o700)
    result, failure, errors, tests, model, source_pins = None, None, [], None, None, {}
    try:
        manifest_path = ROOT / 'source-manifest.json'
        manifest_size = manifest_path.stat().st_size
        raw = read(manifest_path, dict(bytes=manifest_size, sha256=sys.argv[1]), 65536)
        manifest = json.loads(raw)
        require(manifest['schema'] == 'ferric-position5-exact-head-source-v1'
                and set(manifest['files']) == {'head.py', 'test_head.py', 'run.py', 'README.md'}
                and manifest['test_names'] == TEST_NAMES, 'closed source/test manifest')
        source = {name: read(ROOT / name, row, 65536) for name, row in manifest['files'].items()}
        source_pins = {str(ROOT / name): row for name, row in manifest['files'].items()}
        H = load('head', source['head.py'], ROOT / 'head.py')
        T = load('test_head', source['test_head.py'], ROOT / 'test_head.py')
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(T.HeadTests)
        require(sorted(test._testMethodName for test in suite) == TEST_NAMES, 'twelve exact synthetic tests')
        text = io.StringIO()
        observed = unittest.TextTestRunner(stream=text, verbosity=2).run(suite)
        raw_test = text.getvalue().encode()
        require(len(raw_test) <= 65536, 'bounded synthetic test stderr')
        test_pin = save(out / 'tests.stderr', raw_test)
        tests = dict(names=TEST_NAMES, passed=observed.testsRun-len(observed.failures)-len(observed.errors),
                     failures=len(observed.failures), errors=len(observed.errors), skipped=len(observed.skipped), stderr=test_pin)
        require(observed.testsRun == 12 and observed.wasSuccessful() and not observed.skipped, 'synthetic arithmetic gate')
        require({p.name for p in (ROOT / 'inputs').iterdir()} == set(INPUTS), 'closed eight-body input directory')
        bodies = {name: read(ROOT / 'inputs' / name, expected) for name, expected in INPUTS.items()}
        native, reference = admit(H, bodies)
        index = H.parse(read(MODEL / INDEX, MODEL_PINS[INDEX], 65536))
        require(index['weight_map']['lm_head.weight'] == SHARD
                and index['metadata']['total_size'] == 16381470720, 'original model index placement')
        rows, model = scan_shard(H)
        result = H.analyze(native, reference, rows)
        post_rows, post_model = scan_shard(H)
        require(rows == post_rows and model == post_model, 'full shard and row posthash')
    except BaseException as error:
        failure = repr(error)
    for path, row in list(READSET.items()):
        guard()
        try:
            read(Path(path), compact(row))
        except Exception as error:
            errors.append(dict(path=path, error=repr(error)))
    guard()
    terminal = dict(schema='ferric-position5-exact-head-cpu-v1', passed=failure is None and not errors,
        error=failure, postcheck_errors=errors, tests=tests, source_pins=source_pins,
        inputs=READSET, model_shard=model, diagnostic=result, elapsed_seconds=time.monotonic()-started,
        limits=dict(wall_seconds=WHOLE_SECONDS, cpu_seconds=CPU_SECONDS, address_space_bytes=MEMORY_BYTES,
                    affinity=sorted(os.sched_getaffinity(0)), nice=os.getpriority(os.PRIO_PROCESS, 0)),
        reused_prior_native_and_reference_admission=True, full_capsule_revalidated=False,
        native_weight_upload_revalidated=False, original_full_head_shard_rehashed=model is not None,
        gpu_execution=False, model_execution=False, native_rerun=False, mfma_emulation=False,
        numerical_acceptance=False, full_model_acceptance=False, performance_claim=False, production_authority=False)
    guard()
    save(out / ('complete.json' if terminal['passed'] else 'failed.json'), encoded(terminal))
    guard()
    print(json.dumps(dict(passed=terminal['passed'], error=failure, postcheck_errors=errors)))
    signal.setitimer(signal.ITIMER_REAL, 0)
    return 0 if terminal['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
