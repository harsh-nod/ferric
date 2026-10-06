"""One bounded data-only current R2 replay; no child, native or GPU work."""
import ast
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import resource
import signal
import stat
import sys
import time
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
OUT = E / 'guarded-mlp-model-current-r2-replay-v228-v2'
WHOLE_SECONDS, MAX_BODY, MAX_TOTAL = 120, 8 << 20, 32 << 20
SOURCES = {
    'inputs.json': 'ff489d01f1ded2f7d84f0d98f0f7291a28776a328d3d749ebcffc091a82c8fd6',
    'capture.py': 'ab101bd82fad89c889880c77c6c7fdaf2bfcf0ac97322fb6c937ddc600ef38b1',
    'r2.py': 'c9e2d565a5bfa384bf15330653f83422089ad47d245a9bcee159e88caa3dadfd',
    'test_r2.py': 'ca5829ccb77ccf14b25ea5e12dc43090abe4aff1efd318de0b593b6c6437af77',
    'fp32_replay.py': '9a7eb5e4f5c503c83b126c21f6cf880b46dda58881eb22a62f71ca2e4a74cb13',
    'exact_bf16.py': 'b9e53a3afa4fb851a8e7231c020e5ed19fb55deb7ce4d9f0bff1454e472acddf',
    'test_fp32_replay.py': '87a358807c4f76fbd7b0fc7603dc08de7e34c66871b06aa6d8ac89f165798adb',
    'contract/arithmetic.rs': 'e221d035dc7f0c70c1dc362cf754a4928dc76f69eb4e66465b811057d0bacd6a',
    'contract/kernels.rs': '3293184a9b00aa5b33a735a16d9d0665b4b88614e1e61a3f624e11b080eae8cb',
}
TESTS = {
    'test_fp32_replay': ('Fp32ReplayTests', (
        'test_addition_matches_fraction_not_host_float',
        'test_all_lanes_all_steps_and_xor_tree_fraction_reference',
        'test_bf16_boundary_materializes_before_residual',
        'test_encoding_decoding_matches_independent_fraction',
        'test_fraction_nearest_neighbor_midpoints_and_neighbors',
        'test_gradual_underflow_and_signed_zero',
        'test_lane_cancellation_preserves_actual_association',
        'test_midpoint_distance_has_explicit_exact_scale',
        'test_nonfinite_shape_and_overflow_refusals',
        'test_overflow_threshold_both_signs',
        'test_partials_are_not_individually_narrowed',
        'test_product_underflow_and_subnormal_counters')),
    'test_r2': ('R2Tests', (
        'test_all_rows_and_independent_rank_residuals',
        'test_capture_closes_all_parts_and_actual_output_join',
        'test_failed_admission_identity_and_saved_observations_refuse',
        'test_framework_control_is_separate_and_reports_mismatch',
        'test_materialization_zero_and_overflow_boundaries',
        'test_nonfinite_all_operand_and_output_roles_refuse',
        'test_one_mutated_output_remains_an_exact_mismatch',
        'test_role_extent_and_bytes_type_are_closed')),
}


def require(ok, message):
    if not ok: raise ValueError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {k: row[k] for k in ('bytes', 'sha256')}


def parse(raw):
    def pairs(items):
        out = {}
        for k, v in items:
            require(k not in out, 'duplicate JSON key'); out[k] = v
        return out
    def invalid(value): raise ValueError('nonfinite JSON ' + value)
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def encode(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False) + '\n').encode()


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
            value.st_size, value.st_mtime_ns, value.st_ctime_ns)


class Reader:
    def __init__(self, deadline):
        self.deadline, self.inputs = deadline, {}

    def check(self):
        require(time.monotonic() < self.deadline, 'whole data-only deadline')

    def read(self, path, expected=None):
        self.check(); path = Path(path)
        require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical original input')
        with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
            before = os.fstat(stream.fileno())
            require(stat.S_ISREG(before.st_mode) and before.st_nlink >= 1
                    and 0 <= before.st_size <= MAX_BODY, 'bounded ordinary input')
            body = stream.read(MAX_BODY + 1)
            require(stamp(os.fstat(stream.fileno())) == stamp(before), 'open input changed')
        require(stamp(path.stat(follow_symlinks=False)) == stamp(before) and len(body) == before.st_size,
                'final input identity/extent')
        actual = pin(body)
        if expected is not None: require(actual == compact(expected), 'exact original input pin: ' + str(path))
        if str(path) in self.inputs: require(self.inputs[str(path)] == actual, 'input drift')
        self.inputs[str(path)] = actual
        require(len(self.inputs) <= 40 and sum(v['bytes'] for v in self.inputs.values()) <= MAX_TOTAL,
                'bounded selected readset')
        self.check(); return body


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec); sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def function_ast(raw, name):
    nodes = [n for n in ast.parse(raw).body if isinstance(n, ast.FunctionDef) and n.name == name]
    require(len(nodes) == 1, 'one named qualified function')
    return ast.dump(nodes[0], include_attributes=False)


def execute(reader, source, r2, write):
    manifest = parse(reader.read(source / 'inputs.json'))
    require(manifest['schema'] == 'ferric-guarded-mlp-current-r2-input-v1'
            and manifest['future_replay_terminal'] is None
            and manifest['native_execution'] is manifest['gpu_execution'] is False, 'closed actual input manifest')
    files = manifest['files']
    require(set(files) == {'terminal', 'native', 'envelope', 'capture_checked', 'request0', 'observation0',
        'controller', 'numerical', 'candidate_cpu', 'candidate_sources', 'framework_aliases',
        'framework_projection', 'framework_residual', 'framework_final'}, 'fourteen exact original data inputs')
    bodies = {name: reader.read(row['path'], row) for name, row in files.items()}
    terminal, numerical, native = (parse(bodies[k]) for k in ('terminal', 'numerical', 'native'))
    require(terminal['controller'] == files['controller']
            and numerical['diagnostic']['candidate']['terminal'] == files['terminal']
            and numerical['diagnostic']['candidate']['controller'] == files['controller'], 'actual producer/report joins')
    require(function_ast(bodies['controller'], 'capture_admission')
            == function_ast(reader.read(source / 'capture.py'), 'capture_admission'), 'unchanged qualified capture function')
    roles = {'native': 'native/complete.json', 'envelope': 'native/child-stderr.bin',
             'capture_checked': 'capture-observation.json', 'request0': 'native/request-0.json',
             'observation0': 'native/observation-0.bin'}
    for role, name in roles.items():
        require(terminal['raw'][name] == files[role] and numerical['inputs'][files[role]['path']] == files[role],
                'original raw plus independent numerical input join')
    require(r2.Fields.rust_pin(native['files']['child_stderr']) == files['envelope']
            and r2.Fields.rust_pin(native['files']['frames'][0]['request']) == files['request0']
            and r2.Fields.rust_pin(native['files']['frames'][0]['observation']) == files['observation0'],
            'actual native operand file joins')
    allowed = {files[name]['path']: (files[name], bodies[name]) for name in ('envelope', 'request0', 'observation0')}
    def native_read(row):
        require(row['path'] in allowed and row == allowed[row['path']][0], 'closed capture function readset')
        return allowed[row['path']][1]
    selected, custody = r2.admit(terminal, numerical, bodies['native'], native_read, parse(bodies['capture_checked']))
    cpu, sources = parse(bodies['candidate_cpu']), parse(bodies['candidate_sources'])
    require(cpu['passed'] is True and cpu['failure'] is None and cpu['postcheck_errors'] == []
            and cpu['final_sources'] == files['candidate_sources'], 'qualified arithmetic source ancestry')
    contract = manifest['source_contract']
    require(contract['cpu'] == files['candidate_cpu'] and contract['source_map'] == files['candidate_sources'],
            'declared source provenance')
    for key in ('arithmetic', 'kernels'):
        require(pin(reader.read(source / 'contract' / (key + '.rs'))) == compact(sources[contract[key]]),
                'qualified source contract bytes')
    aliases = parse(bodies['framework_aliases'])
    require(aliases['schema'] == 'ferric-guarded-mlp-stage-framework-aliases-v1'
            and aliases['historical_receipts_rewritten'] is aliases['gpu_execution'] is False, 'unchanged reference aliases')
    for key, row in manifest['framework'].items():
        require(aliases['files'][row['original']['path']] == row
                and files['framework_' + key]['path'] == aliases['root'] + '/' + row['relative']
                and compact(files['framework_' + key]) == compact(row['original'])
                and numerical['inputs'][row['original']['path']] == row['original'], 'physical/logical reference pin join')
    control = r2.framework_control(*(bodies['framework_' + key] for key in ('projection', 'residual', 'final')))
    result, outputs = r2.replay(selected)
    result.update(custody=custody, original_inputs=files, source_contract=contract, framework_boundary_control=control)
    for name, body in outputs.items(): write(name, body)
    write('replay.json', encode(result))
    require(control['all_encodings_match'], 'framework residual boundary control mismatch')
    require(result['all_final_encodings_match'], 'observed native R2 encoding mismatch; replay retained')
    return dict(rows_per_rank=4096, matched_words=result['matched_words'], framework_control=control,
                source_contract_only=True, machine_instruction_order_proven=False)


def main():
    require(__debug__ and sys.dont_write_bytecode and sys.argv[1:] in ([], ['--self-test']),
            'python3 -I -B run.py [--self-test]')
    self_test = bool(sys.argv[1:]); out = OUT.with_name(OUT.name + '-tests') if self_test else OUT
    start = time.monotonic(); reader = Reader(start + WHOLE_SECONDS)
    for name in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'): os.environ[name] = ''
    os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 256 << 20), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 4 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        limit = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    def stop(number, frame): raise TimeoutError('bounded data replay interrupted: ' + str(number))
    for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM): signal.signal(number, stop)
    signal.setitimer(signal.ITIMER_REAL, WHOLE_SECONDS)
    require(out.parent.resolve(strict=True) == out.parent, 'canonical fresh output parent')
    out.mkdir(mode=0o700, exist_ok=False)
    source = Path(__file__).resolve(strict=True).parent
    outputs, errors, posterrors, result, count = {}, [], [], None, 0
    def write(name, body):
        reader.check(); require(len(body) <= 4 << 20, 'bounded output body')
        require(sum(v['bytes'] for v in outputs.values()) + len(body) <= 8 << 20, 'bounded output total')
        with (out / name).open('xb') as stream: stream.write(body)
        outputs[name] = pin(body)
    try:
        reader.read(Path(__file__).resolve(strict=True))
        for name, expected in SOURCES.items():
            require(hashlib.sha256(reader.read(source / name)).hexdigest() == expected, 'frozen source input: ' + name)
        for name in ('exact_bf16', 'fp32_replay', 'capture', 'r2'): load(name, source / (name + '.py'))
        suite, loader = unittest.TestSuite(), unittest.TestLoader()
        for name, (cls_name, methods) in TESTS.items():
            module = load(name, source / (name + '.py')); cls = getattr(module, cls_name)
            require(tuple(loader.getTestCaseNames(cls)) == methods, 'exact test roster')
            suite.addTests(loader.loadTestsFromTestCase(cls))
        log = io.StringIO(); observed = unittest.TextTestRunner(stream=log, verbosity=2).run(suite)
        write('tests.stderr', log.getvalue().encode()); count = observed.testsRun
        require(count == 20 and observed.wasSuccessful() and not observed.skipped, 'twenty exact tests passed')
        if not self_test: result = execute(reader, source, sys.modules['r2'], write)
    except BaseException as exc:
        errors.append(type(exc).__name__ + ': ' + str(exc))
    for path, expected in list(reader.inputs.items()):
        try: reader.read(path, expected)
        except BaseException as exc:
            posterrors.append(type(exc).__name__ + ': ' + str(exc))
            if time.monotonic() >= reader.deadline: break
    try:
        reader.check()
        for name, expected in outputs.items():
            require(pin((out / name).read_bytes()) == expected, 'output posthash')
    except BaseException as exc: posterrors.append(type(exc).__name__ + ': ' + str(exc))
    passed = not errors and not posterrors and count == 20 and (self_test or result is not None)
    receipt = dict(schema='ferric-guarded-mlp-current-r2-run-v1', passed=passed, errors=errors,
        postcheck_errors=posterrors, self_test_only=self_test, tests=count,
        test_names=[m + '.' + c + '.' + n for m, (c, names) in TESTS.items() for n in names],
        inputs=reader.inputs, outputs=outputs, result=result, elapsed_seconds=time.monotonic() - start,
        inputs_posthashed=not posterrors, child_processes_spawned=0, data_only=True,
        numerical_acceptance=False, acceptance_threshold=None, full_model_acceptance=False,
        performance_claim=False, production_authority=False, native_execution=False, gpu_execution=False,
        framework_execution=False, full_original_capsule_audit_repeated=False)
    with (out / ('complete.json' if passed else 'failed.json')).open('xb') as stream: stream.write(encode(receipt))
    print(json.dumps(dict(passed=passed, tests=count, result=result, errors=errors, postcheck_errors=posterrors)), flush=True)
    signal.setitimer(signal.ITIMER_REAL, 0)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
