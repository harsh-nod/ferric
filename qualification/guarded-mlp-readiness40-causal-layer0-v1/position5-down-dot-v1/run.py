"""Bounded CPU-only selected Down exact-error diagnostic on pinned original captures."""
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

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-position5-down-dot-v228-v1')
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
    'registration-original.json': dict(bytes=122075, sha256='3ecdd5afb99848539310b8eb6201f09645cc27df1275d5bd2b6db0d40550f20e'),
    'uploads.json': dict(bytes=178103, sha256='d5e66dbf7e3abfec463424addb6735f9a3a5b2d50653d56689ee79d01404da47'),
}
INPUTS.update({
    'r2-terminal.json': dict(bytes=21551, sha256='bf16c755ede242da389e99a2a907e2907e8cf442c2e82fd62a41c0a7e9d2528b'),
    'r2-replay.json': dict(bytes=1008054, sha256='eee5940bdf309285cc68e03dd2e8889cf074024965c0466836c13d0e67409bdd'),
    'r2-derived-down.bf16': dict(bytes=8192, sha256='968bc3d6a1ac14ba80885fd9ee8cacb3426a17dde254135d565ee92b324cea10'),
    'r2-derived-ordered-sum.f32': dict(bytes=16384, sha256='0797e1c13dff8e02d8788e52f8fa75d913fbab6dbd2dbb094f2581bfdcb35992'),
})
SOURCE_NAMES = {
    'head.py', 'qkv.py', 'exact_bf16.py', 'fp32_replay.py', 'capture.py', 'r2.py',
    'position5.py', 'gateup.py', 'test_fp32_replay.py', 'test_r2.py', 'test_position5.py',
    'test_gateup.py', 'arithmetic.rs', 'kernels.rs', 'down.py', 'test_down.py', 'run.py', 'README.md'
}
TEST_CLASSES = [('test_fp32_replay', 'Fp32ReplayTests'), ('test_r2', 'R2Tests'),
                ('test_position5', 'Position5Tests'), ('test_gateup', 'GateUpTests'), ('test_down', 'DownTests')]
TEST_NAMES = [
    "test_down.DownTests.test_exact_6144_term_dot_against_independent_fraction",
    "test_down.DownTests.test_exact_error_terms_reconcile_without_floating_threshold",
    "test_down.DownTests.test_full_shard_layout_transpose_gap_dtype_and_index_refusal",
    "test_down.DownTests.test_original_registration_and_current_upload_roles",
    "test_down.DownTests.test_partition_byte_hash_and_complete_extent",
    "test_down.DownTests.test_product_halves_and_exact_seven_original_coordinates",
    "test_down.DownTests.test_product_role_scalar_extent_identity_and_hash_refusals",
    "test_down.DownTests.test_reuse_complete_original_boundary_rows_without_dot_reexecution",
    "test_down.DownTests.test_reuse_failure_pin_row_order_operand_and_output_drift_refuse",
    "test_down.DownTests.test_row_major_column_halves_not_contiguous_tensor_halves",
    "test_down.DownTests.test_selected_row_bounds_sparse_delta_and_nonclaims",
    "test_down.DownTests.test_signed_zero_and_own_input_counterfactual_is_not_model_feedback",
    "test_down.DownTests.test_strict_finite_operands_sum_join_and_ideal_overflow",
    "test_down.DownTests.test_tp_sum_rounding_and_bf16_projection_rounding_are_distinct",
    "test_fp32_replay.Fp32ReplayTests.test_addition_matches_fraction_not_host_float",
    "test_fp32_replay.Fp32ReplayTests.test_all_lanes_all_steps_and_xor_tree_fraction_reference",
    "test_fp32_replay.Fp32ReplayTests.test_bf16_boundary_materializes_before_residual",
    "test_fp32_replay.Fp32ReplayTests.test_encoding_decoding_matches_independent_fraction",
    "test_fp32_replay.Fp32ReplayTests.test_fraction_nearest_neighbor_midpoints_and_neighbors",
    "test_fp32_replay.Fp32ReplayTests.test_gradual_underflow_and_signed_zero",
    "test_fp32_replay.Fp32ReplayTests.test_lane_cancellation_preserves_actual_association",
    "test_fp32_replay.Fp32ReplayTests.test_midpoint_distance_has_explicit_exact_scale",
    "test_fp32_replay.Fp32ReplayTests.test_nonfinite_shape_and_overflow_refusals",
    "test_fp32_replay.Fp32ReplayTests.test_overflow_threshold_both_signs",
    "test_fp32_replay.Fp32ReplayTests.test_partials_are_not_individually_narrowed",
    "test_fp32_replay.Fp32ReplayTests.test_product_underflow_and_subnormal_counters",
    "test_gateup.GateUpTests.test_exact_seven_dots_bounds_midpoints_and_nonclaims",
    "test_gateup.GateUpTests.test_extraction_closes_seven_rows_and_repetition",
    "test_gateup.GateUpTests.test_original_shard_transpose_and_extent_refusals",
    "test_gateup.GateUpTests.test_partition_pin_wrong_role_rank_and_mutated_bytes_refused",
    "test_gateup.GateUpTests.test_rank_and_partition_boundary_coordinates",
    "test_gateup.GateUpTests.test_registration_reconstruction_and_four_source_roles",
    "test_gateup.GateUpTests.test_wrong_input_and_comparison_pin_refused",
    "test_gateup.GateUpTests.test_wrong_role_rank_alias_extent_and_upload_refused",
    "test_position5.Position5Tests.test_body_part_and_comparison_hash_drift_refuse",
    "test_position5.Position5Tests.test_full4096_rows_own_operands_and_separate_framework_control",
    "test_position5.Position5Tests.test_missing_duplicate_scalar_boundary_extent_and_gap_refuse",
    "test_position5.Position5Tests.test_observed_output_error_is_reported_without_acceptance",
    "test_position5.Position5Tests.test_repeat_complete_capture_pin_and_prior_admission_refuse",
    "test_position5.Position5Tests.test_wrong_position_generation_layer_and_boolean_rank_refuse",
    "test_r2.R2Tests.test_all_rows_and_independent_rank_residuals",
    "test_r2.R2Tests.test_capture_closes_all_parts_and_actual_output_join",
    "test_r2.R2Tests.test_failed_admission_identity_and_saved_observations_refuse",
    "test_r2.R2Tests.test_framework_control_is_separate_and_reports_mismatch",
    "test_r2.R2Tests.test_materialization_zero_and_overflow_boundaries",
    "test_r2.R2Tests.test_nonfinite_all_operand_and_output_roles_refuse",
    "test_r2.R2Tests.test_one_mutated_output_remains_an_exact_mismatch",
    "test_r2.R2Tests.test_role_extent_and_bytes_type_are_closed"
]
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
    postnorm = dict(bytes=8192, sha256='62339bad15d35910fe2fe0e8bf4206871c1c656ae5fb5b92291b62df47084a67')
    require(all(pin(vector) == postnorm for vector in vectors.values()), 'actual same position-five postnorm pin')
    partitions, registration = Q.registration_upload_pins(
        bodies['registration-original.json'], summary, bodies['uploads.json'])
    require(Q.normalized(summary['request']['tiles_image']) == dict(bytes=33320,
        sha256='b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589'),
        'recorded current selected tiles image, not historical ISA identity')
    return vectors, partitions, registration


def scan_shard(H, D, index, expected_uploads):
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
        start = D.tensor_layout(H.parse(raw_header), index, length, before.st_size)
        rows, row_meta = {}, []
        for row in D.ROWS:
            for rank in (0, 1):
                offset = D.row_offset(start, row, rank)
                body = os.pread(stream.fileno(), D.HALF * 2, offset)
                H.words(body, D.HALF)
                rows[row, rank] = body
                row_meta.append(dict(position=5, layer=0, rank=rank, checkpoint_row=row,
                    checkpoint_first_column=rank * D.HALF, uploaded_partition_row=row,
                    file_offset=offset, retained_offset=(len(row_meta) * D.HALF * 2), **pin(body)))
        partitions = []
        for rank in (0, 1):
            digest, total = hashlib.sha256(), 0
            for row in range(D.WIDTH):
                guard()
                body = os.pread(stream.fileno(), D.HALF * 2, D.row_offset(start, row, rank))
                require(len(body) == D.HALF * 2, 'complete original strided rank-column half')
                digest.update(body)
                total += len(body)
            actual = dict(bytes=total, sha256=digest.hexdigest())
            D.check_partition(actual, expected_uploads[rank])
            partitions.append(dict(rank=rank, checkpoint_tensor=D.TENSOR,
                first_checkpoint_column=rank * D.HALF, rows=D.WIDTH, columns=D.HALF,
                row_stride_bytes=2 * D.HALF * 2, upload=expected_uploads[rank], **actual))
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
        tensor_file_offset=start, rows=row_meta, uploaded_down_partitions=partitions,
        current_native_upload_manifest=INPUTS['uploads.json'], stable_stat=list(stamp(before)))


def main():
    global DEADLINE
    started = time.monotonic()
    DEADLINE = started + WHOLE_SECONDS
    require(__debug__ and sys.dont_write_bytecode, 'python3 -B bounded diagnostic')
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
    result, failure, errors, tests, model, source_pins, registration = None, None, [], None, None, {}, None
    try:
        manifest_path = ROOT / 'source-manifest.json'
        raw = read(manifest_path, dict(bytes=manifest_path.stat().st_size, sha256=sys.argv[1]), 65536)
        manifest = json.loads(raw)
        require(manifest['schema'] == 'ferric-position5-selected-down-exact-source-v1'
                and set(manifest['files']) == SOURCE_NAMES and manifest['test_names'] == TEST_NAMES,
                'closed eighteen-source/48-test manifest')
        source = {name: read(ROOT / name, row, 65536) for name, row in manifest['files'].items()}
        source_pins = {str(ROOT / name): row for name, row in manifest['files'].items()}
        loaded = {}
        for name in ('head', 'qkv', 'exact_bf16', 'fp32_replay', 'capture', 'r2',
                     'position5', 'gateup', 'down', 'test_fp32_replay', 'test_r2',
                     'test_position5', 'test_gateup', 'test_down'):
            loaded[name] = load(name, source[name + '.py'], ROOT / (name + '.py'))
        H, G, D = loaded['head'], loaded['gateup'], loaded['down']
        suite, observed_names = unittest.TestSuite(), []
        for module, name in TEST_CLASSES:
            part = unittest.defaultTestLoader.loadTestsFromTestCase(getattr(loaded[module], name))
            observed_names.extend(test.id() for test in part)
            suite.addTests(part)
        require(sorted(observed_names) == TEST_NAMES, 'exact 48-name inherited and new roster')
        text = io.StringIO()
        observed = unittest.TextTestRunner(stream=text, verbosity=2).run(suite)
        raw_test = text.getvalue().encode()
        require(len(raw_test) <= 65536, 'bounded synthetic stderr')
        test_pin = save(out / 'tests.stderr', raw_test)
        tests = dict(names=TEST_NAMES, passed=observed.testsRun-len(observed.failures)-len(observed.errors),
            failures=len(observed.failures), errors=len(observed.errors), skipped=len(observed.skipped), stderr=test_pin)
        require(observed.testsRun == 48 and observed.wasSuccessful() and not observed.skipped, '48 synthetic tests')
        require({p.name for p in (ROOT / 'inputs').iterdir()} == set(INPUTS), 'closed thirteen-body inputs')
        bodies = {name: read(ROOT / 'inputs' / name, expected) for name, expected in INPUTS.items()}
        admit(H, G, bodies)
        captured = D.extract(bodies)
        reused = D.reuse_r2(captured, bodies['r2-terminal.json'], bodies['r2-replay.json'],
                           bodies['r2-derived-down.bf16'], bodies['r2-derived-ordered-sum.f32'])
        uploads, registration = D.registration_upload_pins(
            bodies['registration-original.json'], H.parse(bodies['native-summary.json']), bodies['uploads.json'])
        index = H.parse(read(MODEL / INDEX, MODEL_PINS[INDEX], 65536))
        require(index['metadata']['total_size'] == 16381470720, 'original full model index')
        rows, model = scan_shard(H, D, index, uploads)
        result = D.analyze(captured, reused, rows)
        post_rows, post_model = scan_shard(H, D, index, uploads)
        require(rows == post_rows and model == post_model, 'full shard/partitions/extracted rows posthash')
        weight_raw = b''.join(rows[row, rank] for row in D.ROWS for rank in (0, 1))
        result['retained_weight_halves'] = save(out / 'selected-weight-halves.bf16', weight_raw)

    except BaseException as error:
        failure = repr(error)
    for path, row in list(READSET.items()):
        guard()
        try:
            read(Path(path), compact(row))
        except Exception as error:
            errors.append(dict(path=path, error=repr(error)))
    guard()
    terminal = dict(schema='ferric-position5-selected-down-exact-cpu-v1', passed=failure is None and not errors,
        error=failure, postcheck_errors=errors, tests=tests, source_pins=source_pins, inputs=READSET,
        model_shard=model, registration_join=registration, diagnostic=result, elapsed_seconds=time.monotonic()-started,
        limits=dict(wall_seconds=WHOLE_SECONDS, cpu_seconds=CPU_SECONDS, address_space_bytes=MEMORY_BYTES,
                    affinity=sorted(os.sched_getaffinity(0)), nice=os.getpriority(os.PRIO_PROCESS, 0)),
        reused_prior_native_reference_comparison_admission=True, full_capsule_revalidated=False,
        current_native_down_partition_hashes_rechecked=model is not None, original_full_shard_rehashed=model is not None,
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
