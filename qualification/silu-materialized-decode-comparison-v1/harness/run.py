"""Bounded retained TF4 comparison on ASROCK; no model, tool or GPU launch."""
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PACKAGE = E / 'p228-silu-materialized-decode-comparison-v1'
PURE_PATH = PACKAGE / 'pure.py'
PURE_SHA = '4b2b5d01f03dfdb05caea82e58dc126b208e46b922836fb5e25af8b67438f624'
FRAMEWORK = dict(path=str(E / 'framework-rearm-v224-v1/complete.json'), bytes=8288,
    sha256='cac5d79969c2e17a19630a581b5e21c594ea65b452806630ee87bf7855396036')

BASELINE = dict(path=str(E / 'projection-residual-decode-framework-comparison-v228-v1/complete.json'),
    bytes=206798, sha256='2a5b98ae6a3a2ba4a034dbea0662504988d388b12a5447eaa395f82e3a41dd0c')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def bootstrap():
    require(not sys.flags.optimize and sys.dont_write_bytecode
        and all(k not in os.environ for k in ('PYTHONOPTIMIZE', 'PYTHONPATH', 'PYTHONHOME')),
        'ordinary isolated -B Python before helper imports')
    require(PURE_PATH.resolve(strict=True) == PURE_PATH, 'canonical bounded helper')
    with os.fdopen(os.open(PURE_PATH, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        before = os.fstat(stream.fileno()); raw = stream.read(65537); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and stamp(before) == stamp(after) == stamp(PURE_PATH.lstat())
        and len(raw) == before.st_size <= 65536 and hashlib.sha256(raw).hexdigest() == PURE_SHA,
        'exact qualified pure controller source')
    module = types.ModuleType('projection_decode_comparison_harness'); module.__file__ = str(PURE_PATH)
    exec(compile(raw, str(PURE_PATH), 'exec'), module.__dict__)
    return module


def main(args):
    require(len(args) == 5, 'run.py GPU_COMPLETE GPU_SHA FRESH_LABEL PURE_COMPLETE_SHA SELF_SHA')
    P = bootstrap(); P.enforce(300); J = P.bootstrap()
    require(Path(__file__).resolve(strict=True) == PACKAGE / 'run.py', 'selected actual controller path')
    controller = J.actual(Path(__file__).resolve(), args[4])[0]
    require(re.fullmatch(r'silu-materialized-decode-framework-comparison-v228-v[1-9][0-9]{0,8}', args[2]),
        'fresh diagnostic namespace')
    output = E / args[2]
    require(not os.path.lexists(output) and output.parent.resolve(strict=True) == E, 'exclusive diagnostic output')
    gpu_path = Path(args[0])
    require(gpu_path.parent.parent == E and gpu_path.name == 'complete.json'
        and re.fullmatch(r'prefix-silu-materialized-decode-gpu-v228-v[1-9][0-9]{0,8}', gpu_path.parent.name),
        'actual original GPU completion namespace')
    gpu_pin = J.actual(gpu_path, args[1])[0]
    reader = J.Reader({})
    before = P.source_snapshot(J)
    pure_controller = J.actual(PURE_PATH, PURE_SHA)[0]
    before['pure_controller'] = pure_controller; before['controller'] = controller
    for record in before.values(): reader(record)

    pure_pin = J.actual(P.OUT / 'complete.json', args[3])[0]
    pure = reader.doc(pure_pin)
    require(pure['schema'] == 'ferric-p228-silu-materialized-decode-comparison-pure-v1'
        and pure['passed'] is True and type(pure['tests']) is int and pure['tests'] == 17
        and all(type(pure[key]) is int and pure[key] == 0 for key in ('errors', 'failures', 'skipped'))
        and pure['controller'] == pure_controller and pure['source_postchecks_passed'] is True
        and pure['synthetic_only'] is True and pure['gpu_execution'] is False
        and pure['numerical_acceptance'] is False and pure['performance_claim'] is False
        and pure['full_model_acceptance'] is False and pure['production_authority'] is False,
        'actual passing synthetic qualification, not invented counts')
    require(len(pure['names']) == len(set(pure['names'])) == 17
        and {name.rsplit('.', 1)[-1] for name in pure['names']} == P.TESTS, 'actual17 named tests')
    tested = P.source_snapshot(J); tested['controller'] = pure_controller
    require(reader.doc(pure['sources_before']) == reader.doc(pure['sources_after']) == tested,
        'same fully pinned adapter and imported helpers tested before/after')
    reader(pure['transcript'])

    def load(relative, name, aliases=None):
        return J.module(reader, before[relative], name, aliases)
    prefix = 'p228-silu-materialized-decode-gpu-v1/'
    stage = load(prefix + 'stage_core.py', 'retained_tf4_stage')
    smoke = load(prefix + 'smoke_validation.py', 'retained_tf4_smoke', {'stage_core': stage})
    V = load(prefix + 'decode_validation.py', 'retained_projection_tf4',
        {'stage_core': stage, 'smoke_validation': smoke})
    C = load('p227-prefix-decode-gpu-qualification-v2/compare.py', 'retained_decode_comparison')
    D = load('p225-tiles-tf4-framework-comparison/helpers/diagnostics.py', 'retained_tf4_diagnostics')
    A = load('p228-silu-materialized-decode-comparison-v1/comparison.py', 'projection_decode_comparison')
    def read(pin, maximum):
        require(type(pin['bytes']) is int and pin['bytes'] <= maximum, 'caller reader extent bound')
        return reader(pin)

    output.mkdir(mode=0o700)
    before_pin = P.save(J, output, 'sources-before.json', before)
    errors, comparison = [], None
    try:
        comparison = A.compare_retained(gpu_pin, FRAMEWORK, read, C, V, D)
        comparison['recorded_baseline_metrics'] = A.baseline_metrics(read, BASELINE, comparison, C)
    except Exception as error:
        errors.append(type(error).__name__ + ': ' + str(error))
    after = None
    try:
        after = P.source_snapshot(J, False)
        after['pure_controller'] = J.actual(PURE_PATH)[0]
        after['controller'] = J.actual(Path(__file__).resolve())[0]
        require(after == before, 'unchanged full source closure')
        reader.recheck()
    except Exception as error:
        errors.append('postcheck: ' + type(error).__name__ + ': ' + str(error))
    after_pin = P.save(J, output, 'sources-after.json', after)
    passed = not errors and comparison is not None
    payloads = [row['original'] for row in reader.consumed.values()
        if re.fullmatch(re.escape(str(E / 'framework-rearm-v224-v1/reference'))
                        + r'/pass[12]-pos[0-3]\.bf16', row['original']['path'])]
    if passed and (len(payloads) != 8 or any(row['bytes'] != 606976 for row in payloads)):
        passed = False; errors.append('all eight genuine repeat payloads must be rehashed')
    value = dict(schema='ferric-p228-silu-materialized-decode-framework-comparison-observation-v1',
        completed=passed, errors=errors, controller=controller, pure_complete=pure_pin,
        native_outer=gpu_pin, framework_outer=FRAMEWORK, baseline_comparison=BASELINE,
        sources_before=before_pin, sources_after=after_pin,
        source_postchecks_passed=after == before, comparison=comparison,
        framework_payloads_rehashed=len(payloads), consumed=list(reader.consumed.values()),
        reference_arithmetic_unchanged=True, baseline_metrics_recomputed=False,
        baseline_payloads_rehashed=False, source_image_admission_replayed=False,
        current_platform_audit_performed=False, gpu_execution=False, numerical_acceptance=False,
        acceptance_threshold=None, full_model_correctness=False, full_model_acceptance=False,
        performance_claim=False, production_authority=False)
    result_pin = P.save(J, output, 'complete.json' if passed else 'failure.json', value)
    print(json.dumps(dict(completed=passed, result=result_pin,
        tensor_rows=comparison['tensor_rows'] if comparison is not None else None,
        output_tokens=comparison['candidate_output_tokens'] if comparison is not None else None)), flush=True)
    signal.alarm(0)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))

