"""Bounded CPU-only exact QKV diagnostic on pinned, previously admitted captures."""
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

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-causal-qkv-exact-v228-v1')
MODEL = Path('/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target')
SHARD = 'model-00001-of-00005.safetensors'
INDEX = 'model.safetensors.index.json'
MODEL_PINS = {
    SHARD: dict(bytes=3996250744, sha256='31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f'),
    INDEX: dict(bytes=32878, sha256='f9fdbcb91c23971c13ec5d5f2573d2349e8f61f2f049371ec699281748fdb1bc'),
}
INPUTS = {
    'comparison.json': dict(bytes=285894, sha256='3f94fe36125e3cbbf92e33caca6049b94fd33963956ef1477bf304dcec6bf38f'),
    'native-summary.json': dict(bytes=21934, sha256='5a414a72e6d0e4b76031d57de909f954c7cd0853a39888b5f3a167fe6e94d8f4'),
    'native-terminal.json': dict(bytes=278903, sha256='00aa0447a23f57c6d9bb2ed39cd4e5b2b9da692f15bceb33107ac3822f95516f'),
    'native-sidecar.bin': dict(bytes=1666665, sha256='6577294939bca22bb62bab672e053b45d5c19798fdfc7f5cd10b8f4b6f7e93a2'),
    'reference-inner.json': dict(bytes=285346, sha256='5ebd12d258fc9c76190b4ec4c7270d9732d15e635c7944d966bd50ffc81fa027'),
    'reference-sidecar1.bin': dict(bytes=1855037, sha256='d3b7af6a85243168c440f2332905ca493248f9631fc894732dc5cc860c6bb273'),
    'reference-sidecar2.bin': dict(bytes=1855037, sha256='14a4139ae3653edb35d04c28009c26147069b9e42dd5cfe41823921c0dd87d9c'),
    'uploads.json': dict(bytes=178103, sha256='d5e66dbf7e3abfec463424addb6735f9a3a5b2d50653d56689ee79d01404da47'),
}
ORACLE_NAMES = ['test_decimal_is_exact_not_float', 'test_dot_cancellation_and_exact_products',
    'test_exact_decode_matches_fraction', 'test_four_dots_and_delta_decomposition',
    'test_fraction_nearest_independent_local_oracle', 'test_header_rejects_shape_dtype_extent_and_duplicate',
    'test_overflow_boundary', 'test_payload_extent_and_nonfinite', 'test_roundtrip_finite_samples',
    'test_shard_layout_two_row_offsets', 'test_subnormal_normal_and_signed_underflow', 'test_ties_even_and_signs']
QKV_NAMES = ['test_exact_five_dots_preserve_midpoints_and_no_acceptance',
    'test_extraction_closes_five_rows_and_repeated_inputs', 'test_missing_wrong_extent_and_nonfinite_rows_refused',
    'test_original_shard_geometry_and_index_refusals', 'test_rank_and_projection_row_coordinates',
    'test_unique_rank_specific_packed_uploads']
WHOLE_SECONDS, CPU_SECONDS, MEMORY_BYTES = 240, 180, 512 << 20
DEADLINE = float('inf')
READSET = {}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def guard():
    remaining = DEADLINE - time.monotonic()
    if remaining <= 0:
        raise TimeoutError('whole 240-second diagnostic deadline')
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


def admit(H, Q, bodies):
    comparison = H.parse(bodies['comparison.json'])
    native = H.parse(bodies['native-terminal.json'])
    summary = H.parse(bodies['native-summary.json'])
    reference = H.parse(bodies['reference-inner.json'])
    require(comparison['passed'] is True and comparison['error'] is None
            and comparison['postcheck_errors'] == [] and comparison['numerical_acceptance'] is False,
            'actual completed comparison, not numerical acceptance')
    checks = comparison['checks']
    require(checks['full_original_receipts_authenticated'] is True
            and checks['full_prompt_model_bundle_joined'] is True
            and checks['both_same_side_parity_gates_rechecked'] is True
            and Q.normalized(checks['native']['original_terminal']) == INPUTS['native-terminal.json']
            and checks['reference']['causal_repeat_gate_rechecked'] is True
            and checks['reference']['same_side_reference_parity_rechecked'] is True, 'prior full capsule admission')
    require(native['passed'] is True and native['errors'] == [] and native['native_attempts'] == 1
            and native['causal_layer_zero_verified'] is True
            and summary['native_closed'] is True and summary['completed_forwards'] == 40
            and summary['generated_tokens'] == [], 'actual native40 healthy Close')
    for role, name in [('native/complete.json', 'native-summary.json'),
                       ('native/child-stderr.bin', 'native-sidecar.bin')]:
        require(Q.normalized(native['raw'][role]) == INPUTS[name], 'native raw body join')
    require(reference['passed'] is True and reference['error'] is None
            and reference['postcheck_errors'] == [] and reference['repeat_gate_passed'] is True
            and reference['causal_repeat_gate_passed'] is True
            and reference['native_intermediates_used'] is False, 'actual repeated genuine reference')
    sequence = summary['bootstrap']['sequence']
    require(summary['request']['base']['source'] == str(MODEL.parent)
            and bytes(sequence['scope']['model_id']).hex() == reference['model_id']
            and bytes(sequence['scope']['bundle_id']).hex() == reference['bundle_id'], 'same original model/bundle')
    require(Q.normalized(sequence['begin']['uploads']) == INPUTS['uploads.json']
            and bytes(summary['upload_manifest_sha256']).hex() == INPUTS['uploads.json']['sha256'],
            'historical exact manifest is the current native Begin upload manifest')
    for name, expected in MODEL_PINS.items():
        require(Q.normalized(reference['model_sources'][name]) == expected, 'authentic model pin ' + name)
    for ordinal, row in enumerate(reference['causal_layer_zero'], 1):
        require(row['file'] == 'pass%d-causal-layer0.bin' % ordinal
                and Q.normalized(row['pin']) == INPUTS['reference-sidecar%d.bin' % ordinal],
                'original repeated reference sidecar join')
    require(len(reference['causal_layer_zero']) == 2, 'two original sidecars')
    vectors = Q.extract(bodies['native-sidecar.bin'], bodies['reference-sidecar1.bin'],
                        bodies['reference-sidecar2.bin'], comparison)
    return vectors, Q.packed_upload_pins(H.parse(bodies['uploads.json']))


def scan_shard(H, Q, index, expected_uploads):
    guard()
    path = MODEL / SHARD
    require(path.resolve(strict=True) == path, 'canonical original shard')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink >= 1
                and before.st_size == MODEL_PINS[SHARD]['bytes'], 'pinned full shard extent')
        raw_length = stream.read(8)
        require(len(raw_length) == 8, 'header length')
        length = struct.unpack('<Q', raw_length)[0]
        require(2 <= length <= 1 << 20, 'bounded safetensors header')
        raw_header = stream.read(length)
        require(len(raw_header) == length, 'complete safetensors header')
        starts = Q.tensor_layout(H.parse(raw_header), index, length, before.st_size)
        rows, row_meta = {}, []
        for target in Q.TARGETS:
            position, rank, kind, local = target[:4]
            global_row, packed_row = Q.coordinates(rank, kind, local)
            offset = starts[kind] + global_row * 8192
            body = os.pread(stream.fileno(), 8192, offset)
            H.words(body, 4096)
            rows[target[:4]] = body
            row_meta.append(dict(position=position, rank=rank, kind=kind, local_row=local,
                checkpoint_row=global_row, packed_rank_row=packed_row, file_offset=offset, **pin(body)))
        packed = {}
        for rank in (0, 1):
            digest, total = hashlib.sha256(), 0
            for kind in 'qkv':
                count = Q.ROWS[kind] // 2 * 8192
                start = starts[kind] + rank * count
                for offset in range(0, count, 1 << 20):
                    guard()
                    size = min(1 << 20, count - offset)
                    body = os.pread(stream.fileno(), size, start + offset)
                    require(len(body) == size, 'complete original rank-half bytes')
                    digest.update(body)
                    total += len(body)
            packed[rank] = dict(bytes=total, sha256=digest.hexdigest())
            require(packed[rank] == expected_uploads[rank], 'current native packed QKV equals authentic checkpoint halves')
        stream.seek(0)
        digest, count = hashlib.sha256(), 0
        while True:
            guard()
            body = stream.read(2 << 20)
            if not body:
                break
            count += len(body)
            require(count <= before.st_size, 'shard growth')
            digest.update(body)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and count == before.st_size,
            'stable entire shard and extracted original rows')
    actual = dict(bytes=count, sha256=digest.hexdigest())
    require(actual == MODEL_PINS[SHARD], 'entire original shard SHA')
    guard()
    return rows, dict(path=str(path), **actual, header=pin(raw_header),
        tensor_file_offsets=starts, rows=row_meta, packed_rank_qkv=packed,
        current_native_upload_manifest=INPUTS['uploads.json'], stable_stat=list(stamp(before)))


def main():
    global DEADLINE
    started = time.monotonic()
    DEADLINE = started + WHOLE_SECONDS
    require(len(sys.argv) == 2 and len(sys.argv[1]) == 64, 'usage: run.py SOURCE_MANIFEST_SHA256')
    require(Path(__file__).resolve().parent == ROOT and ROOT.resolve(strict=True) == ROOT,
            'fresh owned source root')
    require(socket.gethostname() == 'smci350-rck-g03-b19-03' and os.getuid() == 9661, 'MI350 owner host/uid')
    for key in ('ROCR_VISIBLE_DEVICES', 'HIP_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES', 'GPU_DEVICE_ORDINAL'):
        os.environ[key] = ''
    os.sched_setaffinity(0, {8, 9})
    current = os.getpriority(os.PRIO_PROCESS, 0)
    if current < 10:
        os.nice(10 - current)
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
        raw = read(manifest_path, dict(bytes=manifest_path.stat().st_size, sha256=sys.argv[1]), 65536)
        manifest = json.loads(raw)
        require(manifest['schema'] == 'ferric-causal-five-qkv-exact-source-v1'
                and set(manifest['files']) == {'head.py', 'test_head.py', 'qkv.py', 'test_qkv.py', 'run.py', 'README.md'}
                and manifest['oracle_tests'] == ORACLE_NAMES and manifest['qkv_tests'] == QKV_NAMES,
                'closed source and exact eighteen-test manifest')
        source = {name: read(ROOT / name, row, 65536) for name, row in manifest['files'].items()}
        source_pins = {str(ROOT / name): row for name, row in manifest['files'].items()}
        H = load('head', source['head.py'], ROOT / 'head.py')
        Q = load('qkv', source['qkv.py'], ROOT / 'qkv.py')
        HT = load('test_head', source['test_head.py'], ROOT / 'test_head.py')
        QT = load('test_qkv', source['test_qkv.py'], ROOT / 'test_qkv.py')
        suite = unittest.TestSuite()
        for cls, names in [(HT.HeadTests, ORACLE_NAMES), (QT.QkvTests, QKV_NAMES)]:
            part = unittest.defaultTestLoader.loadTestsFromTestCase(cls)
            require(sorted(t._testMethodName for t in part) == names, 'exact class test roster')
            suite.addTests(part)
        text = io.StringIO()
        observed = unittest.TextTestRunner(stream=text, verbosity=2).run(suite)
        raw_test = text.getvalue().encode()
        require(len(raw_test) <= 65536, 'bounded synthetic stderr')
        test_pin = save(out / 'tests.stderr', raw_test)
        tests = dict(names=ORACLE_NAMES + QKV_NAMES, passed=observed.testsRun-len(observed.failures)-len(observed.errors),
            failures=len(observed.failures), errors=len(observed.errors), skipped=len(observed.skipped), stderr=test_pin)
        require(observed.testsRun == 18 and observed.wasSuccessful() and not observed.skipped, 'eighteen synthetic tests')
        require({p.name for p in (ROOT / 'inputs').iterdir()} == set(INPUTS), 'closed eight-body inputs')
        bodies = {name: read(ROOT / 'inputs' / name, expected) for name, expected in INPUTS.items()}
        vectors, uploads = admit(H, Q, bodies)
        index = H.parse(read(MODEL / INDEX, MODEL_PINS[INDEX], 65536))
        require(index['metadata']['total_size'] == 16381470720, 'original full model index')
        rows, model = scan_shard(H, Q, index, uploads)
        result = Q.analyze(vectors, rows)
        post_rows, post_model = scan_shard(H, Q, index, uploads)
        require(rows == post_rows and model == post_model, 'full shard and extracted row posthash')
    except BaseException as error:
        failure = repr(error)
    for path, row in list(READSET.items()):
        guard()
        try:
            read(Path(path), compact(row))
        except Exception as error:
            errors.append(dict(path=path, error=repr(error)))
    guard()
    terminal = dict(schema='ferric-causal-five-qkv-exact-cpu-v1', passed=failure is None and not errors,
        error=failure, postcheck_errors=errors, tests=tests, source_pins=source_pins, inputs=READSET,
        model_shard=model, diagnostic=result, elapsed_seconds=time.monotonic()-started,
        limits=dict(wall_seconds=WHOLE_SECONDS, cpu_seconds=CPU_SECONDS, address_space_bytes=MEMORY_BYTES,
                    affinity=sorted(os.sched_getaffinity(0)), nice=os.getpriority(os.PRIO_PROCESS, 0)),
        reused_prior_native_reference_comparison_admission=True, full_capsule_revalidated=False,
        current_native_packed_qkv_hashes_rechecked=model is not None, original_full_shard_rehashed=model is not None,
        gpu_execution=False, model_execution=False, native_rerun=False, fixed_accumulation_emulation=False,
        semantic_bug_claimed=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False, production_authority=False)
    save(out / ('complete.json' if terminal['passed'] else 'failed.json'), encoded(terminal))
    guard()
    print(json.dumps(dict(passed=terminal['passed'], error=failure, postcheck_errors=errors)))
    signal.setitimer(signal.ITIMER_REAL, 0)
    return 0 if terminal['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
