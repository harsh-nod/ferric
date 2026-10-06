"""Bounded data-only accounting; authenticated original inputs are never modified."""
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import time
import unittest

TERMINAL = dict(bytes=614572, sha256='91a20d628644eb784d651e4d4d87571cb55cdb367971092fcea40d259ee38ec2')
CONTROLLER = dict(bytes=38453, sha256='4c49f8053fdb5b676338b45b257d37280ec068a3f98cd37efb1aa0cc50c4f156')
SOURCES = {
    'analyze.py': dict(bytes=11568, sha256='f3e903f52cebafa8fa1842aaefe6777c3ea1d1dda66a703e60d7028d71238491'),
    'test_analysis.py': dict(bytes=9521, sha256='2f97754e9379af486893f578327a5c3f0c0b9a58e9f175ea080a69e1678a3d4e'),
}
TESTS = (
    'test_checked_raw_cross_joins', 'test_close_and_no_authority_upgrade', 'test_closed_phase_schedule_and_census',
    'test_counter_decrease_and_wrong_delta', 'test_exact_disjoint_wall_partition_and_total',
    'test_forward_and_whole_sum_overflow_refuse', 'test_inclusive_counters_telescope_without_wall_addition',
    'test_markdown_exact_integer_seconds_and_scope', 'test_policy_identity_and_fresh_baseline',
    'test_strict_u64_not_bool_float_or_overflow',
)
LABELS = ('parent-readelf', 'parent-ldd', 'worker-readelf', 'worker-ldd',
          'before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2')
REMOTE = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-model-host-observation-gpu-v228-v1'
WHOLE_SECONDS = 120
MAX_BODY = 8 << 20


def require(ok, reason):
    if not ok: raise ValueError(reason)


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def compact(row):
    require(type(row) is dict and type(row['bytes']) is int and 0 <= row['bytes'] <= MAX_BODY
            and type(row['sha256']) is str and re.fullmatch('[0-9a-f]{64}', row['sha256']), 'strict body pin')
    return dict(bytes=row['bytes'], sha256=row['sha256'])


def parse(body):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    def invalid(value): raise ValueError('nonfinite JSON ' + value)
    return json.loads(body, object_pairs_hook=pairs, parse_constant=invalid)


def encode(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False) + '\n').encode()


def stamp(s):
    return s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns


class Reader:
    def __init__(self, deadline):
        self.deadline, self.inputs = deadline, {}

    def check(self):
        require(time.monotonic() < self.deadline, 'whole data-analysis deadline')

    def read(self, path, expected=None):
        self.check()
        path = Path(path)
        require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical original input path')
        with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
            before = os.fstat(stream.fileno())
            require(stat.S_ISREG(before.st_mode) and before.st_nlink >= 1 and before.st_size <= MAX_BODY,
                    'bounded regular input')
            body = stream.read(MAX_BODY + 1)
            after = os.fstat(stream.fileno())
        require(len(body) == before.st_size and stamp(before) == stamp(after) == stamp(path.stat(follow_symlinks=False)),
                'stable input identity')
        actual = pin(body)
        if expected is not None: require(actual == compact(expected), 'exact input hash: ' + str(path))
        if str(path) in self.inputs: require(self.inputs[str(path)] == actual, 'input changed across reads')
        self.inputs[str(path)] = actual
        require(len(self.inputs) <= 16 and sum(row['bytes'] for row in self.inputs.values()) <= 32 << 20,
                'bounded selected readset')
        self.check()
        return body


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def original_inputs(reader, root, analysis):
    require(root.is_absolute() and root.resolve(strict=True) == root and root.is_dir(), 'canonical input root')
    terminal = parse(reader.read(root / 'ar4/complete.json', TERMINAL))
    require(terminal['schema'] == 'ferric-guarded-mlp-model-host-observation-gpu-v1'
            and terminal['mode'] == 'ar4' and terminal['passed'] is True and terminal['errors'] == []
            and type(terminal['native_attempts']) is int and terminal['native_attempts'] == 1
            and type(terminal['retries']) is int and terminal['retries'] == 0
            and terminal['host_observation_requested'] is terminal['host_observation_verified'] is True,
            'actual successful host-only diagnostic provenance')
    for key in ('numerical_acceptance', 'full_model_acceptance', 'independent_full_model_reference',
                'full_long_workload', 'performance_claim', 'production_authority'):
        require(terminal[key] is False, 'original nonclaims')
    require([p['label'] for p in terminal['phases']] == list(LABELS) and len(terminal['raw']) == 79,
            'actual phase/raw census')
    for phase in terminal['phases']:
        require(type(phase['exit_code']) is int and phase['exit_code'] == 0 and phase['reason'] is None
                and phase['cleanup_signalled'] is False and phase['owned_groups_absent'] is True
                and phase['owned_processes_reaped'] is True, 'recorded natural owned lifecycle')
    require(compact(terminal['controller']) == CONTROLLER
            and terminal['controller']['path'] == REMOTE + '/run_model_gpu.py', 'qualified source controller')
    reader.read(root / 'run_model_gpu.py', CONTROLLER)
    def raw(name):
        row = terminal['raw'][name]
        require(row['path'] == REMOTE + '/ar4/' + name, 'original raw path')
        return reader.read(root / 'ar4' / name, row)
    checked = parse(raw('host-observation.json'))
    report_body = raw('native/child-stderr.bin')
    report = parse(report_body)
    native = parse(raw('native/complete.json'))
    require(analysis.same(checked, terminal['host_observation'])
            and checked['source'] == terminal['raw']['native/child-stderr.bin']
            and compact(checked['source']) == pin(report_body), 'checked/original report hash joins')
    rust = native['files']['child_stderr']
    require(type(rust['sha256']) is list and len(rust['sha256']) == 32
            and all(type(v) is int and 0 <= v <= 255 for v in rust['sha256']), 'native digest octets')
    require(dict(path=rust['path'], bytes=rust['bytes'], sha256=bytes(rust['sha256']).hex()) == checked['source'],
            'native report body join')
    require(analysis.same(report['bootstrap'], native['bootstrap'])
            and analysis.same(report['worker_sha256'], native['request']['decode']['worker']['sha256'])
            and analysis.same(report['profile_sha256'], native['profile_sha256'])
            and analysis.same(report['child_pid'], native['child_pid'])
            and analysis.same(report['completions'], [{k: v for k, v in frame['response']['event'].items() if k != 'status'}
                                                     for frame in native['files']['frames']]),
            'same-run native identities and four wire completions')
    return report, checked


def main():
    require(len(sys.argv) == 3, 'usage: run.py INPUT_ROOT OUTPUT_ROOT | run.py --self-test OUTPUT_ROOT')
    start = time.monotonic()
    reader = Reader(start + WHOLE_SECONDS)
    resource.setrlimit(resource.RLIMIT_AS, (256 << 20, 256 << 20))
    resource.setrlimit(resource.RLIMIT_CPU, (120, 120))
    resource.setrlimit(resource.RLIMIT_FSIZE, (8 << 20, 8 << 20))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    os.umask(0o077)
    def stop(signum, frame): raise TimeoutError('bounded data analysis interrupted: %d' % signum)
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM): signal.signal(signum, stop)
    signal.setitimer(signal.ITIMER_REAL, WHOLE_SECONDS)
    source = Path(__file__).resolve(strict=True).parent
    out = Path(sys.argv[2])
    require(out.is_absolute() and out.parent.resolve(strict=True) == out.parent, 'canonical output parent')
    out.mkdir(mode=0o700, exist_ok=False)
    outputs, errors, result, count = {}, [], None, 0
    self_test = sys.argv[1] == '--self-test'
    try:
        reader.read(Path(__file__).resolve(strict=True))
        for name, expected in SOURCES.items(): reader.read(source / name, expected)
        analysis = load('analyze', source / 'analyze.py')
        tests = load('test_analysis', source / 'test_analysis.py')
        loader = unittest.TestLoader()
        require(tuple(loader.getTestCaseNames(tests.AnalysisTests)) == TESTS, 'exact ten-test census')
        log = io.StringIO()
        observed = unittest.TextTestRunner(stream=log, verbosity=2).run(loader.loadTestsFromTestCase(tests.AnalysisTests))
        test_body = log.getvalue().encode()
        require(len(test_body) <= 65536, 'bounded test log')
        (out / 'tests.stderr').write_bytes(test_body)
        outputs['tests.stderr'] = pin(test_body)
        count = observed.testsRun
        require(count == 10 and observed.wasSuccessful() and not observed.skipped, 'ten synthetic tests passed')
        reader.check()
        if not self_test:
            result = analysis.analyze(*original_inputs(reader, Path(sys.argv[1]), analysis))
            result['original_terminal'] = TERMINAL
            result['selected_raw_rehashed'] = ['host-observation.json', 'native/child-stderr.bin', 'native/complete.json']
            for name, body in (('analysis.json', encode(result)), ('summary.md', analysis.markdown(result).encode())):
                reader.check()
                require(len(body) <= 1 << 20, 'bounded analysis output')
                (out / name).write_bytes(body)
                outputs[name] = pin(body)
    except BaseException as exc:
        errors.append(type(exc).__name__ + ': ' + str(exc))
    postchecks = []
    for path, expected in list(reader.inputs.items()):
        try: reader.read(Path(path), expected)
        except BaseException as exc:
            postchecks.append(type(exc).__name__ + ': ' + str(exc))
            if time.monotonic() >= reader.deadline: break
    try:
        reader.check()
        for name, expected in outputs.items():
            require(pin((out / name).read_bytes()) == expected, 'output posthash')
    except BaseException as exc: postchecks.append(type(exc).__name__ + ': ' + str(exc))
    passed = not errors and not postchecks and count == 10 and (self_test or result is not None)
    receipt = dict(schema='ferric-guarded-mlp-model-host-analysis-run-v1', passed=passed,
        errors=errors, postcheck_errors=postchecks, self_test_only=self_test, tests=count,
        test_names=list(TESTS), inputs=reader.inputs, outputs=outputs, elapsed_seconds=time.monotonic() - start,
        original_terminal=None if self_test else TERMINAL, original_gpu_execution_repeated=False,
        data_analysis=True, child_processes_spawned=0, selected_input_posthashes_clean=not postchecks,
        full_original_capsule_audit_repeated=False, gpu_execution=False, gpu_time=False, gpu_overlap=False,
        throughput=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False, production_authority=False)
    (out / ('complete.json' if passed else 'failed.json')).write_bytes(encode(receipt))
    print(json.dumps(dict(passed=passed, tests=count, outputs=outputs, errors=errors, postcheck_errors=postchecks)), flush=True)
    signal.setitimer(signal.ITIMER_REAL, 0)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
