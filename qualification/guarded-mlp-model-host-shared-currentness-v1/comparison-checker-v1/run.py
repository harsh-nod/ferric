"""Bounded comparison of two immutable host reports; no child or GPU execution."""
import copy
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

BASELINE_TERMINAL = dict(bytes=614572, sha256='91a20d628644eb784d651e4d4d87571cb55cdb367971092fcea40d259ee38ec2')
CANDIDATE_TERMINAL = None
SOURCES = {
    'analyze.py': dict(bytes=11568, sha256='f3e903f52cebafa8fa1842aaefe6777c3ea1d1dda66a703e60d7028d71238491'),
    'compare.py': dict(bytes=10735, sha256='e5fea8b0ad3e76ffbfa25360c5c3a5df92dec72692e510dcd6ae0e1dff0fc46a'),
    'test_analysis.py': dict(bytes=9521, sha256='2f97754e9379af486893f578327a5c3f0c0b9a58e9f175ea080a69e1678a3d4e'),
    'test_compare.py': dict(bytes=6395, sha256='512fb462b9b302f1e7cd03907791cedab00a50faa4d10f60cd87639b1bc8a873'),
}
TESTS = {
    'test_analysis': ('AnalysisTests', (
        'test_checked_raw_cross_joins', 'test_close_and_no_authority_upgrade', 'test_closed_phase_schedule_and_census',
        'test_counter_decrease_and_wrong_delta', 'test_exact_disjoint_wall_partition_and_total',
        'test_forward_and_whole_sum_overflow_refuse', 'test_inclusive_counters_telescope_without_wall_addition',
        'test_markdown_exact_integer_seconds_and_scope', 'test_policy_identity_and_fresh_baseline',
        'test_strict_u64_not_bool_float_or_overflow')),
    'test_compare': ('CompareTests', (
        'test_exact_ratios_large_values_zero_and_decimal_ties',
        'test_payload_or_genuine_history_mismatch_precedes_accounting',
        'test_policy_mapping_preserves_original_reports_and_partition',
        'test_slower_candidate_is_reported_without_performance_claim',
        'test_structured_svg_has_exact_eight_disjoint_stacks',
        'test_wrong_mode_mixed_policy_and_checked_policy_refuse')),
}
LABELS = ('parent-readelf', 'parent-ldd', 'worker-readelf', 'worker-ldd',
          'before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2')
REMOTE = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/'
WHOLE_SECONDS, MAX_BODY = 120, 8 << 20


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
        require(time.monotonic() < self.deadline, 'whole data-comparison deadline')

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
        require(len(self.inputs) <= 40 and sum(row['bytes'] for row in self.inputs.values()) <= 64 << 20,
                'bounded selected readset')
        self.check()
        return body


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def wire_pin(row):
    values = row['sha256']
    require(type(values) is list and len(values) == 32
            and all(type(v) is int and 0 <= v <= 255 for v in values), 'native digest octets')
    return dict(path=row['path'], **compact(dict(bytes=row['bytes'], sha256=bytes(values).hex())))


def case(reader, root, terminal_pin, shared, analysis):
    require(type(terminal_pin) is dict and set(terminal_pin) == {'bytes', 'sha256'}, 'actual case terminal remains unbound')
    require(root.is_absolute() and root.resolve(strict=True) == root and root.is_dir(), 'canonical case root')
    mode = 'shared-currentness' if shared else 'observation'
    remote = REMOTE + 'guarded-mlp-model-host-' + mode + '-gpu-v228-v1'
    terminal = parse(reader.read(root / 'ar4/complete.json', terminal_pin))
    require(terminal['schema'] == 'ferric-guarded-mlp-model-host-' + mode + '-gpu-v1'
            and terminal['mode'] == 'ar4' and terminal['passed'] is True and terminal['errors'] == []
            and type(terminal['native_attempts']) is int and terminal['native_attempts'] == 1
            and type(terminal['retries']) is int and terminal['retries'] == 0
            and terminal['host_observation_requested'] is terminal['host_observation_verified'] is True,
            'successful exact host diagnostic')
    for key in ('numerical_acceptance', 'full_model_acceptance', 'independent_full_model_reference',
                'full_long_workload', 'performance_claim', 'production_authority'):
        require(terminal[key] is False, 'original nonclaims')
    if shared:
        require(terminal['shared_full_currentness_requested'] is terminal['shared_full_currentness_verified'] is True
                and terminal['timing_comparison_performed'] is terminal['speedup_claim'] is False,
                'separate verified shared-full mode without timing claims')
    require(terminal['limits']['native_seconds'] == 4000 and terminal['limits']['whole_seconds'] == 4300
            and [p['label'] for p in terminal['phases']] == list(LABELS) and len(terminal['raw']) == 79,
            'same bounded phase definitions')
    for phase in terminal['phases']:
        require(type(phase['exit_code']) is int and phase['exit_code'] == 0 and phase['reason'] is None
                and phase['cleanup_signalled'] is False and phase['owned_groups_absent'] is True
                and phase['owned_processes_reaped'] is True, 'recorded natural owned lifecycle')
    require(terminal['controller']['path'] == remote + '/run_model_gpu.py', 'case controller path')
    reader.read(root / 'run_model_gpu.py', terminal['controller'])
    def selected(name, row):
        require(row['path'] == remote + '/' + name, 'original selected body path')
        return reader.read(root / name, row)
    def raw(name): return selected('ar4/' + name, terminal['raw'][name])
    plan = parse(selected('ar4-input.json', terminal['plan']))
    request = parse(selected('ar4-request.json', plan['request']))
    require(plan['schema'] == 'ferric-guarded-mlp-model-host-' + mode + '-gpu-input-v1'
            and plan['mode'] == 'ar4' and request['schema'] == 'FerricFiniteGuardedMlpDecodeRequestV1'
            and request['decode']['mode'] == 'autoregressive', 'same guarded request category')
    for key in ('worker', 'worker_cpu', 'parent', 'parent_cpu'):
        require(plan[key] == terminal['admission'][key], 'plan/CPU/ELF receipt join')
    native = parse(raw('native/complete.json'))
    checked = parse(raw('host-observation.json'))
    report_body = raw('native/child-stderr.bin'); report = parse(report_body)
    require(analysis.same(native['request'], request) and native['child_exit_zero'] is native['native_closed'] is True
            and native['process_group_absent'] is True and native['completed_forwards'] == 4
            and native['native_attempts'] == 1 and native['retries'] == 0
            and native['close']['event']['status'] == 'closed' and native['close']['event']['completed_forwards'] == 4,
            'actual guarded native request and healthy Close')
    require(analysis.same(checked, terminal['host_observation'])
            and checked['source'] == terminal['raw']['native/child-stderr.bin']
            and wire_pin(native['files']['child_stderr']) == checked['source']
            and compact(checked['source']) == pin(report_body), 'original report/checked/native body joins')
    require(analysis.same(report['bootstrap'], native['bootstrap'])
            and analysis.same(report['worker_sha256'], native['request']['decode']['worker']['sha256'])
            and analysis.same(report['profile_sha256'], native['profile_sha256'])
            and analysis.same(report['child_pid'], native['child_pid'])
            and analysis.same(report['completions'], [{k: v for k, v in frame['response']['event'].items() if k != 'status'}
                                                     for frame in native['files']['frames']]), 'same-run host identity/completion joins')
    require(analysis.same(native['input_tokens'], terminal['observation']['input_tokens'])
            and native['input_tokens'][1:] == native['observed_output_tokens'][:-1]
            and len(native['observed_output_tokens']) == len(native['files']['frames']) == 4,
            'actual autoregressive history')
    payloads = []
    for index, frame in enumerate(native['files']['frames']):
        name = 'native/observation-%d.bin' % index
        require(wire_pin(frame['observation']) == terminal['raw'][name], 'native observation pin join')
        payloads.append(raw(name))
    return dict(report=report, checked=checked, payloads=payloads, history=native['input_tokens'], request=request)


def main():
    self_test = len(sys.argv) == 3 and sys.argv[1] == '--self-test'
    require(self_test or len(sys.argv) == 4, 'usage: run.py BASELINE_ROOT CANDIDATE_ROOT OUTPUT_ROOT | run.py --self-test OUTPUT_ROOT')
    start = time.monotonic(); reader = Reader(start + WHOLE_SECONDS)
    for kind, cap in ((resource.RLIMIT_AS, 256 << 20), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 8 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (cap, cap))
    os.umask(0o077)
    def stop(signum, frame): raise TimeoutError('bounded comparison interrupted: %d' % signum)
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM): signal.signal(signum, stop)
    signal.setitimer(signal.ITIMER_REAL, WHOLE_SECONDS)
    source = Path(__file__).resolve(strict=True).parent; out = Path(sys.argv[-1])
    require(out.is_absolute() and out.parent.resolve(strict=True) == out.parent, 'canonical output parent')
    out.mkdir(mode=0o700, exist_ok=False)
    outputs, errors, result, count = {}, [], None, 0
    def write(name, body):
        reader.check(); require(len(body) <= 1 << 20, 'bounded data output')
        (out / name).write_bytes(body); outputs[name] = pin(body)
    try:
        reader.read(Path(__file__).resolve(strict=True))
        for name, expected in SOURCES.items(): reader.read(source / name, expected)
        analysis = load('analyze', source / 'analyze.py')
        comparison = load('compare', source / 'compare.py')
        loader, suite = unittest.TestLoader(), unittest.TestSuite()
        for name, (class_name, names) in TESTS.items():
            module = load(name, source / (name + '.py')); cls = getattr(module, class_name)
            require(tuple(loader.getTestCaseNames(cls)) == names, 'exact source test census')
            suite.addTests(loader.loadTestsFromTestCase(cls))
        log = io.StringIO(); observed = unittest.TextTestRunner(stream=log, verbosity=2).run(suite)
        write('tests.stderr', log.getvalue().encode()); count = observed.testsRun
        require(count == 16 and observed.wasSuccessful() and not observed.skipped, 'sixteen synthetic tests passed')
        if not self_test:
            baseline = case(reader, Path(sys.argv[1]), BASELINE_TERMINAL, False, analysis)
            candidate = case(reader, Path(sys.argv[2]), CANDIDATE_TERMINAL, True, analysis)
            requests = []
            for value in (baseline, candidate):
                request = copy.deepcopy(value['request'])
                for key in ('worker', 'session', 'evidence_directory'): del request['decode'][key]
                requests.append(request)
            require(analysis.same(*requests), 'identical model/image/prompt/deadline request apart from worker/session/output')
            result = comparison.compare(baseline, candidate)
            result['original_terminals'] = dict(conservative=BASELINE_TERMINAL, shared=CANDIDATE_TERMINAL)
            write('comparison.json', encode(result))
            write('comparison.md', comparison.markdown(result).encode())
            write('host-wall.svg', comparison.svg(result))
            for key, title in (('baseline', 'Conservative'), ('candidate', 'Shared-Full')):
                table = analysis.markdown(result[key]).replace('# Guarded Host Observation', '# ' + title + ' Host Observation', 1)
                write(key + '-table.md', table.encode())
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
        for name, expected in outputs.items(): require(pin((out / name).read_bytes()) == expected, 'output posthash')
    except BaseException as exc: postchecks.append(type(exc).__name__ + ': ' + str(exc))
    passed = not errors and not postchecks and count == 16 and (self_test or result is not None)
    receipt = dict(schema='ferric-guarded-mlp-shared-host-comparison-run-v1', passed=passed,
        errors=errors, postcheck_errors=postchecks, self_test_only=self_test, tests=count,
        test_names=[module + '.' + cls + '.' + name for module, (cls, names) in TESTS.items() for name in names],
        inputs=reader.inputs, outputs=outputs, elapsed_seconds=time.monotonic() - start,
        original_terminals=None if self_test else dict(conservative=BASELINE_TERMINAL, shared=CANDIDATE_TERMINAL),
        data_analysis=True, child_processes_spawned=0, source_input_posthashes_clean=not postchecks,
        full_original_capsule_audits_repeated=False, original_gpu_execution_repeated=False,
        controlled_repeated_benchmark=False, gpu_execution=False, gpu_time=False, gpu_overlap=False,
        throughput=False, speedup_claim=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False, production_authority=False)
    (out / ('complete.json' if passed else 'failed.json')).write_bytes(encode(receipt))
    print(json.dumps(dict(passed=passed, tests=count, outputs=outputs, errors=errors, postcheck_errors=postchecks)), flush=True)
    signal.setitimer(signal.ITIMER_REAL, 0)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
