"""Strict serial ABBA ownership wrapper; no retries, timing analysis or GPU overlap."""
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import sys
import time
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-peer-read-pair-gpu-v228-v2'
ORDER = ('control-0', 'paired-0', 'paired-1', 'control-1')
RUNNER_PIN = None
CHECKER_CPU = None
WHOLE_SECONDS = 4 * 4300 + 120
SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(raw, path):
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def parse(raw):
    def pairs(rows):
        result = {}
        for name, value in rows:
            require(name not in result, 'duplicate JSON field')
            result[name] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def qualified_source(template, current, bindings):
    def tree(raw):
        value = ast.parse(raw)
        found = set()
        for node in value.body:
            if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
                name = node.targets[0].id
                if name in bindings:
                    found.add(name)
                    node.value = ast.Constant(value=None)
        require(found == set(bindings), 'closed qualified literal bindings')
        return ast.dump(value, include_attributes=False)
    require(tree(template) == tree(current), 'only explicit bindings may differ from tested controller')


def input_contract(plans, requests):
    require(tuple(plans) == tuple(requests) == ORDER, 'closed ABBA plans and requests')
    shared, sessions = None, set()
    for name in ORDER:
        plan, request = plans[name], requests[name]
        require(set(plan) == {'schema', 'mode', 'case', 'paired_read', 'parent_cpu', 'worker_cpu',
                             'parent', 'worker', 'request'}
                and plan['schema'] == 'ferric-guarded-mlp-peer-read-pair-gpu-input-v2'
                and plan['mode'] == 'ar4' and plan['case'] == name
                and plan['paired_read'] is name.startswith('paired-'), 'explicit case policy')
        decode = request['decode']
        require(request['schema'] == 'FerricFiniteGuardedMlpDecodeRequestV1'
                and decode['mode'] == 'autoregressive'
                and decode['evidence_directory'] == str(ROOT / name / 'native'), 'fresh AR4 case destination')
        session = decode['session']
        require(type(session) is list and len(session) == 32
                and all(type(v) is int and 0 <= v <= 255 for v in session)
                and any(session) and tuple(session) not in sessions, 'four distinct nonzero sessions')
        sessions.add(tuple(session))
        normalized = dict(request, decode={key: value for key, value in decode.items()
                                          if key not in ('session', 'evidence_directory')})
        identity = dict(request=normalized, **{key: plan[key] for key in
                        ('parent_cpu', 'worker_cpu', 'parent', 'worker')})
        encoded = json.dumps(identity, sort_keys=True, separators=(',', ':'), allow_nan=False)
        require(shared is None or encoded == shared, 'same qualified ELFs, CPU receipts, model and images')
        shared = encoded


def drive_cases(invoke, admit, clock, record):
    """Only an admitted completed case permits the next call; failed prefixes remain owned."""
    rows = []
    for name in ORDER:
        begin = clock()
        require(type(begin) is int and 0 <= begin < 1 << 64
                and (not rows or begin >= rows[-1]['finished_monotonic_ns']), 'monotonic case start')
        status = invoke(name, begin)
        end = clock()
        require(type(end) is int and begin < end < 1 << 64, 'monotonic case finish')
        require(type(status) is int and status == 0, 'case failed; no next case or retry')
        terminal = admit(name)
        row = dict(name=name, terminal=terminal, started_monotonic_ns=begin, finished_monotonic_ns=end)
        record(row)
        rows.append(row)
    return rows


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B run_serial.py')
    require(Path(__file__).resolve().parent == ROOT and ROOT.resolve(strict=True) == ROOT
            and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'exact deployment host/root')
    require(type(RUNNER_PIN) is dict and set(RUNNER_PIN) == {'bytes', 'sha256'}, 'bound reviewed case runner')
    require(type(CHECKER_CPU) is dict and set(CHECKER_CPU) == {'path', 'bytes', 'sha256'}, 'actual checker CPU pin pending')
    require(not any(os.path.lexists(ROOT / name) for name in
                (*ORDER, 'serial-start.json', 'serial-complete.json', 'serial-failed.json', 'comparison-input.json')),
            'fresh serial run and unopened cases')
    os.umask(0o077)
    start = time.monotonic(); deadline = start + WHOLE_SECONDS
    inputs, rows, completed, children = {}, [], {}, set()
    active, failure, comparison, pending_comparison = None, None, None, None
    def interrupted(number, _frame):
        raise RuntimeError('serial signal ' + str(number))
    handlers = {number: signal.getsignal(number) for number in SIGNALS}
    for number in SIGNALS:
        signal.signal(number, interrupted)
    def guard():
        remaining = deadline - time.monotonic()
        require(remaining > 0, 'serial whole deadline')
        signal.setitimer(signal.ITIMER_REAL, remaining)
    def read(path, expected=None, track=True):
        guard(); path = Path(path)
        require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical serial input')
        stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
        with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
            before = os.fstat(stream.fileno())
            require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= 8 << 20, 'bounded ordinary input')
            raw = stream.read((8 << 20) + 1); after = os.fstat(stream.fileno())
        require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size, 'input changed')
        actual = pin(raw, path)
        require(expected is None or actual == expected, 'actual input pin')
        require(not track or str(path) not in inputs or inputs[str(path)] == actual, 'conflicting input pin')
        if track: inputs[str(path)] = actual
        return raw, actual
    def save(name, value):
        guard(); raw = (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()
        require(len(raw) <= 8 << 20, 'bounded serial output')
        path = ROOT / name
        with path.open('xb') as stream:
            stream.write(raw); stream.flush(); os.fsync(stream.fileno())
        return pin(raw, path)
    try:
        guard()
        controller = read(Path(__file__).resolve())[1]
        body, runner_pin = read(ROOT / 'run_model_gpu.py')
        require({key: runner_pin[key] for key in ('bytes', 'sha256')} == RUNNER_PIN, 'reviewed runner bytes')
        checker = parse(read(CHECKER_CPU['path'], CHECKER_CPU)[0])
        require(checker['schema'] == 'ferric-peer-read-pair-checker-cpu-v2'
                and checker['passed'] is True and checker['failure'] is None
                and checker['postcheck_errors'] == [] and checker['source_unchanged'] is True
                and checker['sources_before'] == checker['sources_after']
                and checker['tests']['passed'] == 26
                and all(checker['tests'][key] == 0 for key in ('failed', 'errors', 'skipped'))
                and checker['gpu_execution'] is False and len(checker['phases']) == 1,
                'actual26-test pure checker gate')
        phase = checker['phases'][0]
        require(phase['exit_code'] == 0 and phase['natural_exit'] is True and phase['reaped'] is True
                and phase['process_group_absent'] is True and phase['forced_cleanup'] is False
                and phase['timed_out'] is False and phase['exception'] is None,
                'natural clean checker leaf')
        tested = {}
        require(set(checker['sources_before']) == {'run_cpu.py', 'supervisor.py', 'run_model_gpu.py',
                'run_serial.py', 'test_host.py', 'test_topology.py', 'test_serial.py', 'validate_observation.py'},
                'exact eight checked source bodies')
        for name, source_pin in checker['sources_before'].items():
            tested[name] = read(source_pin['path'], source_pin)[0]
        qualified_source(tested['run_model_gpu.py'], body,
                         {'PLAN_SHA', 'WORKER', 'WORKER_CPU', 'PARENT', 'PARENT_CPU'})
        qualified_source(tested['run_serial.py'], read(Path(__file__).resolve())[0], {'RUNNER_PIN', 'CHECKER_CPU'})
        runner = types.ModuleType('paired_read_case_runner'); runner.__file__ = str(ROOT / 'run_model_gpu.py')
        exec(compile(body, runner.__file__, 'exec'), runner.__dict__)
        require(runner.ROOT == ROOT and runner.ORDER == ORDER and runner.CASE_SECONDS == 4300,
                'unchanged bounded case contract')
        plans, requests = {}, {}
        for name in ORDER:
            raw, row = read(ROOT / (name + '-input.json'))
            require(type(runner.PLAN_SHA[name]) is str and row['sha256'] == runner.PLAN_SHA[name], 'bound case plan')
            plans[name] = parse(raw)
            requests[name] = parse(read(plans[name]['request']['path'], plans[name]['request'])[0])
        input_contract(plans, requests)
        save('serial-start.json', dict(controller=controller, runner=runner_pin, order=list(ORDER),
            started_monotonic_ns=time.monotonic_ns(), whole_seconds=WHOLE_SECONDS))
        def invoke(name, begin):
            nonlocal active
            guard(); require(time.monotonic() + runner.CASE_SECONDS + 60 < deadline, 'next full case reserve')
            active = name
            save(name + '-serial-start.json', dict(name=name, started_monotonic_ns=begin))
            try:
                return runner.run_case(name)
            finally:
                for number in SIGNALS:
                    signal.signal(number, interrupted)
                guard()
        def admit(name):
            guard()
            raw, terminal_pin = read(ROOT / name / 'complete.json')
            value = parse(raw)
            require(value['schema'] == 'ferric-guarded-mlp-peer-read-pair-gpu-v2'
                    and value['passed'] is True and value['errors'] == [] and value['case'] == name
                    and value['paired_read_requested'] is name.startswith('paired-')
                    and value['native_attempts'] == 1 and value['retries'] == 0
                    and value['host_observation_verified'] is True and value['shared_full_currentness_verified'] is True
                    and value['instrumentation_comparison']['all_payloads_equal'] is True
                    and value['instrumentation_comparison']['all_histories_equal'] is True
                    and value['controller'] == runner_pin and len(value['phases']) == 11,
                    'actual clean qualified case and payload equality')
            for phase in value['phases']:
                runner.successful(phase)
            for key in ('parent_cpu', 'worker_cpu', 'parent', 'worker'):
                require(value['admission'][key] == plans[name][key], 'same actual qualified product joins')
            pid = value['observation']['child_pid']
            require(type(pid) is int and pid > 0 and pid not in children, 'distinct native worker process')
            children.add(pid)
            summary_pin = value['raw']['native/complete.json']
            summary = parse(read(summary_pin['path'], summary_pin)[0])
            require(summary['request'] == requests[name] and summary['child_pid'] == pid, 'actual fresh session/request')
            if completed:
                first = next(iter(completed.values()))
                require(value['platform'] == first['platform']
                        and value['observation']['input_tokens'] == first['observation']['input_tokens'],
                        'same platform and actual token histories')
                for index in range(4):
                    key = 'native/observation-%d.bin' % index
                    require(all(value['raw'][key][field] == first['raw'][key][field] for field in ('bytes', 'sha256')),
                            'all four complete same-ELF payloads')
            completed[name] = value
            return terminal_pin
        def record(row):
            rows.append(row)
            save(row['name'] + '-serial-complete.json', row)
        chronology = drive_cases(invoke, admit, time.monotonic_ns, record)
        require(chronology == rows and len(children) == 4, 'four actual independent serial cases')
        for path, expected in list(inputs.items()): read(path, expected, track=False)
        pending_comparison = dict(schema='ferric-peer-read-supervised-abba-v1', controller=controller, runs=chronology)
    except BaseException as error:
        failure = type(error).__name__ + ': ' + str(error)
    finally:
        for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP): signal.signal(number, signal.SIG_IGN)
        post = []
        try:
            for path, expected in list(inputs.items()):
                guard()
                try: read(path, expected, track=False)
                except Exception as error: post.append(str(error))
            if post: failure = failure or 'serial input postcheck failed'
            if failure is None:
                comparison = save('comparison-input.json', pending_comparison)
            save('serial-failed.json' if failure else 'serial-complete.json',
                 dict(schema='ferric-peer-read-supervised-abba-terminal-v1', passed=failure is None,
                      failure=failure, postcheck_errors=post, completed_runs=rows, active_case=active,
                      comparison_input=comparison, readset=inputs, retries=0,
                      elapsed_seconds=time.monotonic() - start, whole_seconds=WHOLE_SECONDS,
                      strict_serial=True, gpu_parallel_execution=False, timing_analysis_performed=False,
                      performance_claim=False, numerical_acceptance=False, production_authority=False))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            for number, handler in handlers.items(): signal.signal(number, handler)
    print(json.dumps(dict(passed=failure is None, failure=failure, completed=len(rows)), sort_keys=True))
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
