"""Bounded twelve-test synthetic qualification; no GPU or subprocess launch."""
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import signal
import stat
import sys
import types
import unittest

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PACKAGE = E / 'p228-projection-residual-decode-comparison-v1'
OUT = E / 'projection-residual-decode-comparison-pure-v228-v1'
J_PATH = E / 'p228-layer0-current-comparison-v1/run.py'
J_SHA = '9c3db491d4f8c20890b045474bd1ab5bd8565d42916a8eb7772c0eb07c44717e'
SOURCES = {
    'p228-projection-residual-decode-comparison-v1/comparison.py': '84398daa04db188d626defa0a769d35eb88b90eaf0e1aa6858da8c93ca734ce1',
    'p228-projection-residual-decode-comparison-v1/test_comparison.py': '71eb3881ed2c736360d028aed77b792cb405c377bf3b3ad3524b708b8bd5f4c8',
    'p228-projection-residual-decode-comparison-v1/README.md': '4935fdbc2f95d90064423f6b1d4e86c20b9558a41e545e30531cab829c88641f',
    'p227-prefix-decode-gpu-qualification-v2/compare.py': '8154580de7f5ad40fd4897ce264fc3d92c9a109808ea7ea5dab6d0486f01e622',
    'p225-tiles-tf4-framework-comparison/helpers/diagnostics.py': '38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf',
    'p228-projection-residual-decode-gpu-v1/stage_core.py': '7e3d64e80e66bacbbba05ac3236189249949bb95ea3f717150822cf45740eec2',
    'p228-projection-residual-decode-gpu-v1/smoke_validation.py': '9bf460451e722452da9c49168c730615d32e9b73b200c579f003e9e45238ea26',
    'p228-projection-residual-decode-gpu-v1/decode_validation.py': 'b4e4cee33a9c82dd58189ca359e5a64d7e1dd1780e974e215c85c40e0df479e0',
    'p228-projection-residual-decode-gpu-v1/test_decode_validation.py': '37a24160ea948a8d19e9f5abac34d5407c239be7311c3e72efbe9074a026ff82',
    'p228-layer0-current-comparison-v1/run.py': J_SHA,
}
TESTS = {
    'test_all152_direct_rows_and_four_token_records',
    'test_last_hidden_norm_and_logits_keep_exact_differences',
    'test_output_token_mismatch_is_diagnostic_not_rejection',
    'test_changed_tf_input_is_not_same_workload',
    'test_nonfinite_tensor_refuses',
    'test_layer_error_can_shrink_without_causal_claim',
    'test_signed_zero_is_nonexact_with_zero_absolute_error',
    'test_truncated_payload_and_wrong_own_argmax_refuse',
    'test_candidate_nested_schema_real_validator_and_seven_leaves',
    'test_failed_or_authority_bearing_gpu_receipt_refuses',
    'test_capture_digest_structural_result_and_request_cannot_drift',
    'test_failed_owner_wrong_selector_and_missing_audit_refuse',
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def enforce(seconds):
    require(not sys.flags.optimize and sys.dont_write_bytecode
        and all(k not in os.environ for k in ('PYTHONOPTIMIZE', 'PYTHONPATH', 'PYTHONHOME')),
        'ordinary isolated -B interpreter')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
        'ASROCK CPU8/9 nice10 identity')
    require(all(os.environ.get(k) == '' for k in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES',
        'CUDA_VISIBLE_DEVICES')), 'hidden GPUs')
    for kind, value in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, seconds),
                        (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        bound = min(n for n in (value, soft, hard) if n != resource.RLIM_INFINITY)
        resource.setrlimit(kind, (bound, bound))
    def deadline(_signum, _frame):
        raise TimeoutError('bounded CPU-only wall deadline')
    signal.signal(signal.SIGALRM, deadline); signal.alarm(seconds)


def bootstrap():
    require(J_PATH.resolve(strict=True) == J_PATH, 'canonical Reader source')
    with os.fdopen(os.open(J_PATH, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        before = os.fstat(stream.fileno()); raw = stream.read(65537); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and stamp(before) == stamp(after) == stamp(J_PATH.lstat())
        and len(raw) == before.st_size == 23607 and hashlib.sha256(raw).hexdigest() == J_SHA,
        'exact qualified Reader bytes')
    module = types.ModuleType('retained_layer0_reader'); module.__file__ = str(J_PATH)
    exec(compile(raw, str(J_PATH), 'exec'), module.__dict__)
    return module


def source_snapshot(J, expected=True):
    return {name: J.actual(E / name, digest if expected else None)[0] for name, digest in SOURCES.items()}


def save(J, output, name, value):
    raw = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    require(len(raw) <= 16 << 20, 'bounded retained JSON')
    with (output / name).open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    return J.actual(output / name)[0]


def main(args):
    enforce(180)
    require(len(args) == 1, 'pure.py SELF_SHA256')
    J = bootstrap(); controller = J.actual(Path(__file__).resolve(), args[0])[0]
    before = source_snapshot(J); before['controller'] = controller
    require(not os.path.lexists(OUT) and OUT.parent.resolve(strict=True) == E, 'fresh pure output')
    OUT.mkdir(mode=0o700); before_pin = save(J, OUT, 'sources-before.json', before)
    reader = J.Reader({})
    adapter = J.module(reader, before['p228-projection-residual-decode-comparison-v1/comparison.py'], 'comparison')
    test_pin = before['p228-projection-residual-decode-comparison-v1/test_comparison.py']
    tests = J.module(reader, test_pin, 'projection_decode_comparison_tests', {'comparison': adapter})
    expected_dependencies = {name: (str(E / path), digest) for name, (path, digest) in tests.DEPENDENCIES.items()}
    require(all(path in {p['path'] for p in before.values()} and
        J.actual(Path(path), digest)[0] in before.values() for path, digest in expected_dependencies.values()),
        'all imported test helper sources pre-pinned')
    suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
    names = [case.id() for group in suite for case in group]
    require(len(names) == len(set(names)) == 12 and {name.rsplit('.', 1)[-1] for name in names} == TESTS,
        'exact12 authored test inventory')
    stream = io.StringIO(); result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
    text = stream.getvalue().encode(); require(len(text) <= 1 << 20, 'bounded test transcript')
    with (OUT / 'tests.log').open('xb') as output:
        output.write(text); output.flush(); os.fsync(output.fileno())
    after = source_snapshot(J, False); after['controller'] = J.actual(Path(__file__).resolve())[0]
    after_pin = save(J, OUT, 'sources-after.json', after)
    reader.recheck(); unchanged = after == before
    passed = unchanged and result.wasSuccessful() and result.testsRun == 12 and not result.skipped
    value = dict(schema='ferric-p228-projection-residual-decode-comparison-pure-v1', passed=passed,
        tests=result.testsRun, names=names, failures=len(result.failures), errors=len(result.errors),
        skipped=len(result.skipped), controller=controller, sources_before=before_pin, sources_after=after_pin,
        transcript=J.actual(OUT / 'tests.log')[0], source_postchecks_passed=unchanged,
        synthetic_only=True, gpu_execution=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False, production_authority=False)
    pin = save(J, OUT, 'complete.json' if passed else 'failed.json', value)
    print(stream.getvalue(), end='', flush=True); print(json.dumps(dict(passed=passed, receipt=pin)), flush=True)
    signal.alarm(0)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
