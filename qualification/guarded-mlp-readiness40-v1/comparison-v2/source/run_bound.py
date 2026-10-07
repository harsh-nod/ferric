"""Bounded data-only comparison of two independently retained Readiness40 runs."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import tarfile
import time
import types
import unittest

HERE = Path(__file__).resolve().parent
NATIVE_ARCHIVE = dict(bytes=2210645, sha256='df5c824f18d26b3c1a6cde4e7eb6827f1472a846cbb5b97a0b693001270d3c3c')
NATIVE_TERMINAL = dict(bytes=169839, sha256='65c85efb646a6980be5a036220e5cfc70acd811b656dc5c821c7f67eae3dd131')
CHECKER_ARCHIVE = dict(bytes=27743, sha256='95cebdbd242abc0d5d8256cd7f7fa11ddc45f5aa18d36e4652baae7b549a8660')
CHECKER_TERMINAL = dict(bytes=10645, sha256='f639b7100ee8ec82f5442f0ae9136ea5f367793803ea78b6ca502328e7c69856')
REFERENCE_SOURCE = dict(bytes=4695, sha256='97dcb5a8dd5e6c986cd8fa792670ab184d77dfeaeb2c742d4a837d07723297ca')
REFERENCE_TERMINAL = dict(bytes=4477, sha256='8dd2b1134cdd5cf6c0888eaac08c580e30890c8a0b47203aa0edc55a02e81759')
REFERENCE_ARCHIVE = dict(bytes=3965567, sha256='cd9103d1e73c1af63960f24e9d501f5a6747ff82a505891f38d08657693cf5bc')
REFERENCE_EVIDENCE_PIN = dict(bytes=30149, sha256='98d246d10ac9f755437d94dad9b1a0783b0e307bfae3d7a579ac3c6a086334a4')
MODULE_PINS = {
    "common.py": {
        "bytes": 3172,
        "sha256": "445c602dd0d237beb2ebef62446d1d2b34283367b68529736a890640ae185256"
    },
    "compare.py": {
        "bytes": 5867,
        "sha256": "d12367520a03735cdfd0ca21f957daf15abbad63de73dee9763ca511855797ca"
    },
    "diagnostics.py": {
        "bytes": 5084,
        "sha256": "38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf"
    },
    "native_evidence.py": {
        "bytes": 25540,
        "sha256": "f81ff9ad0a6dd7a1652701286590779fdaa77209455ce31f5929b6efca67dd73"
    },
    "reference.py": {
        "bytes": 11379,
        "sha256": "bdaa410f15b485de29c3f1f97c4de86fc18a7b158bad58ee36bfc4df394b8568"
    },
    "test_compare.py": {
        "bytes": 6698,
        "sha256": "0f8d500634f107fa645edaf3fec2f6b2e402e79397a321bcc92585db0a45159f"
    }
}
BODY_CAP, TOTAL_CAP, WHOLE_SECONDS = 8 << 20, 128 << 20, 180
READSET = {}
DEADLINE = float('inf')


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def parse(raw):
    def pairs(rows):
        value = {}
        for key, row in rows:
            require(key not in value, 'duplicate JSON key')
            value[key] = row
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def same(a, b):
    return encoded(a) == encoded(b)


def tick():
    require(time.monotonic() < DEADLINE, 'whole comparison deadline')


def read(path, expected=None, cap=BODY_CAP):
    tick()
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input path')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and 0 <= before.st_size <= cap,
                'bounded single-link ordinary input')
        raw = stream.read(cap + 1); after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size,
            'input changed during read')
    row = dict(path=str(path), **pin(raw))
    if expected is not None: require(pin(raw) == expected, 'exact observed input pin')
    require(str(path) not in READSET or READSET[str(path)][0] == row, 'readset drift')
    READSET[str(path)] = (row, cap)
    require(len(READSET) <= 32 and sum(v[0]['bytes'] for v in READSET.values()) <= TOTAL_CAP, 'readset bounds')
    return raw


def archive(path, expected, count, maximum):
    require(expected is not None, 'actual archive pin remains pending')
    raw = read(path, expected, maximum)
    bodies, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as stream:
        for row in stream:
            tick(); name = row.name
            require(row.isfile() and name not in bodies and name not in ('', '.')
                    and Path(name).as_posix() == name and not Path(name).is_absolute()
                    and '..' not in Path(name).parts and not row.pax_headers
                    and 0 <= row.size <= BODY_CAP and len(bodies) < count, 'closed ordinary tar member')
            total += row.size; require(total <= maximum, 'expanded archive cap')
            with stream.extractfile(row) as body:
                bodies[name] = body.read(row.size + 1)
            require(len(bodies[name]) == row.size, 'exact archive body extent')
    require(len(bodies) == count, 'exact successful archive member census')
    return bodies


def load(name, filename, expected):
    require(expected is not None, 'reviewed pure module binding remains pending')
    path = HERE / filename
    raw = read(path, expected)
    module = types.ModuleType(name); module.__file__ = str(path)
    sys.modules[name] = module
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def checker(bodies, native):
    raw = bodies['evidence/complete.json']; require(pin(raw) == CHECKER_TERMINAL, 'actual16 checker terminal')
    value = parse(raw)
    require(value['passed'] is True and value['failure'] is None and value['postcheck_errors'] == []
            and value['source_unchanged'] is True and value['sources_before'] == value['sources_after']
            and value['synthetic_data_tests_only'] is True
            and value['tests']['passed'] == 16 and all(value['tests'][k] == 0 for k in ('failed', 'errors', 'skipped')),
            'actual sixteen-test gate, separate from comparison tests')
    require(len(value['phases']) == 1 and set(bodies) == {'evidence/complete.json'}
            | {Path(row['path']).name for row in value['sources_before'].values()}
            | {'evidence/' + name for name in value['raw']}, 'full checker input/raw closure')
    for row in value['sources_before'].values(): require(pin(bodies[Path(row['path']).name]) == {k: row[k] for k in ('bytes', 'sha256')}, 'checker input hash')
    for name, row in value['raw'].items(): require(pin(bodies['evidence/' + name]) == {k: row[k] for k in ('bytes', 'sha256')}, 'checker raw hash')
    require(bodies['validate_readiness.py'] == native['validate_readiness.py']
            and bodies['readiness_announcement.py'] == native['readiness_announcement.py'], 'tested native validation bodies')
    names = re.findall(r'^(test_[A-Za-z0-9_]+) \((test_readiness\.ReadinessTests)(?:\.[A-Za-z0-9_]+)?\) \.\.\. ok$',
                       bodies['evidence/readiness-tests.stderr'].decode(), re.M)
    require(sorted(cls + '.' + name for name, cls in names) == value['tests']['names'], 'actual named16 raw outcomes')
    return dict(terminal=CHECKER_TERMINAL, tests=16, original_bytes_rechecked=True)


def execute(args):
    require(MODULE_PINS is not None and REFERENCE_TERMINAL is not None
            and REFERENCE_ARCHIVE is not None and REFERENCE_EVIDENCE_PIN is not None,
            'actual reference and reviewed source bindings remain pending')
    native = archive(args.native_archive, NATIVE_ARCHIVE, 82, 66 << 20)
    manifest = parse(native.pop('manifest.json'))
    require(manifest['schema'] == 'ferric-guarded-mlp-readiness40-retention-v1'
            and manifest['terminal_name'] == 'complete.json' and set(native) == set(manifest['files'])
            and all(pin(body) == manifest['files'][name] for name, body in native.items())
            and pin(native['readiness/complete.json']) == NATIVE_TERMINAL, 'actual native capsule closure')
    n = load('native_evidence', 'native_evidence.py', MODULE_PINS['native_evidence.py'])
    n.DEADLINE = DEADLINE
    native_check = n.verify(native, 'complete.json', NATIVE_TERMINAL['sha256'])
    require(native_check['original_passed'] is True and native_check['retained_success_revalidated'] is True
            and same(native_check, manifest['observation']), 'qualified actual native admission')
    checked = checker(archive(args.checker_archive, CHECKER_ARCHIVE, 13, 1 << 20), native)
    reference = archive(args.reference_archive, REFERENCE_ARCHIVE, 124, 100 << 20)
    rm = parse(reference.pop('manifest.json')); helper = reference.pop('retention-tool.py')
    require(rm['schema'] == 'ferric-readiness40-reference-retention-v1'
            and rm['terminal'] == 'complete.json' and rm['source_manifest'] == REFERENCE_SOURCE
            and set(rm['files']) == set(reference) | {'retention-tool.py'}
            and pin(helper) == rm['files']['retention-tool.py'] == REFERENCE_EVIDENCE_PIN
            and all(pin(body) == rm['files'][name] for name, body in reference.items())
            and pin(reference['complete.json']) == REFERENCE_TERMINAL, 'actual independent reference capsule closure')
    r = load('reference_evidence', 'reference_evidence.py', REFERENCE_EVIDENCE_PIN)
    reference_check = r.verify(reference, 'complete.json', REFERENCE_TERMINAL['sha256'])
    require(reference_check['passed'] is True and reference_check['repeat_gate_rechecked'] is True
            and same(reference_check, rm['verification']), 'independent successful two-pass reference')
    for filename in ('common.py', 'diagnostics.py', 'reference.py'):
        require(pin(reference['source/' + filename]) == MODULE_PINS[filename], 'frozen comparison function source join')
        load(filename.removesuffix('.py'), filename, MODULE_PINS[filename])
    comparator = load('compare', 'compare.py', MODULE_PINS['compare.py'])
    tests = load('test_compare', 'test_compare.py', MODULE_PINS['test_compare.py'])
    stream = io.StringIO()
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(tests.ComparisonTests)
    expected_tests = sorted(test.id() for test in suite)
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
    save(args.output / 'comparison-tests.stderr', stream.getvalue().encode())
    require(result.wasSuccessful() and result.testsRun == len(expected_tests) == 8
            and not result.skipped and not result.expectedFailures and not result.unexpectedSuccesses,
            'eight focused comparison tests')
    report = parse(reference['output/complete.json']); contract = parse(reference['source/inputs.json'])
    bodies = {role: reference['inputs/' + contract['locations'][role]] for role in contract['files']}
    tokens, _, model = comparator.R.prompt_inputs(contract, bodies)
    require({name: {k: row[k] for k in ('bytes', 'sha256')} for name, row in report['model_sources'].items()} == model,
            'reference actual model hashes equal original authentic model')
    payloads = [{p: reference['output/pass%d-pos%d.bf16' % (ordinal, p)] for p in (0, 15, 16, 39)} for ordinal in (1, 2)]
    diagnostics = comparator.compare(native, report, payloads, contract, tokens)
    return dict(native=native_check, checker=checked, reference=reference_check,
                comparison_tests=expected_tests, diagnostics=diagnostics)


def save(path, raw):
    tick(); require(len(raw) <= BODY_CAP, 'bounded output')
    with path.open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    return pin(raw)


def main():
    global DEADLINE
    require(__debug__ and sys.flags.isolated == 1 and sys.dont_write_bytecode, 'python3 -I -B run.py')
    parser = argparse.ArgumentParser()
    for name in ('native-archive', 'checker-archive', 'reference-archive', 'output'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    require(args.output.is_absolute() and args.output.parent.resolve(strict=True) == args.output.parent
            and not os.path.lexists(args.output), 'fresh output namespace')
    started = time.monotonic(); DEADLINE = started + WHOLE_SECONDS
    os.umask(0o077); os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0); require(priority in (0, 10), 'nice level')
    if priority == 0: os.nice(10)
    for key in ('ROCR_VISIBLE_DEVICES', 'HIP_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'): os.environ[key] = ''
    for kind, cap in ((resource.RLIMIT_AS, 1 << 30), (resource.RLIMIT_CORE, 0), (resource.RLIMIT_FSIZE, BODY_CAP)):
        old = resource.getrlimit(kind); value = min([cap] + [n for n in old if n != resource.RLIM_INFINITY]); resource.setrlimit(kind, (value, value))
    def interrupted(number, _frame): raise RuntimeError('comparison signal ' + str(number))
    original = {s: signal.getsignal(s) for s in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM)}
    for sig in original: signal.signal(sig, interrupted)
    signal.setitimer(signal.ITIMER_REAL, WHOLE_SECONDS - 10)
    args.output.mkdir(mode=0o700)
    error, checks, posterrors = None, None, []
    try:
        read(Path(__file__).resolve())
        checks = execute(args)
    except BaseException as failure:
        error = repr(failure)
    finally:
        for sig in original:
            if sig != signal.SIGALRM: signal.signal(sig, signal.SIG_IGN)
        signal.setitimer(signal.ITIMER_REAL, max(0.001, DEADLINE - time.monotonic()))
    try:
        for row, cap in list(READSET.values()):
            try: read(Path(row['path']), {k: row[k] for k in ('bytes', 'sha256')}, cap)
            except BaseException as failure: posterrors.append(dict(path=row['path'], error=repr(failure)))
        tick()
        passed = error is None and not posterrors
        receipt = dict(schema='ferric-readiness40-comparison-data-v1', passed=passed, error=error,
            postcheck_errors=posterrors, checks=checks, inputs=[r for r, _ in READSET.values()],
            elapsed_seconds=time.monotonic() - started, limits=dict(seconds=180, address_space_bytes=1 << 30,
                input_bytes=TOTAL_CAP, output_file_bytes=BODY_CAP, affinity=[8, 9], nice=10),
            gpu_execution=False, model_execution=False, native_rerun=False, numerical_acceptance=False,
            acceptance_threshold=None, full_model_acceptance=False, full_long_workload=False,
            performance_claim=False, production_authority=False)
        save(args.output / ('complete.json' if passed else 'failed.json'), encoded(receipt))
        print(json.dumps(dict(passed=passed, error=error, output=str(args.output)), sort_keys=True))
        return 0 if passed else 1
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        for sig, handler in original.items(): signal.signal(sig, handler)


if __name__ == '__main__':
    raise SystemExit(main())
