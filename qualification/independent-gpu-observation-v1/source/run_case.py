"""One independent-profile case, with separate owned inspection/native/audit trees."""
import functools
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import stat
import subprocess
import sys
import time

import prepare as P
import child_evidence as C
import observe as O

D, V, E, R = P.D, P.V, P.E, P.R
STREAM_CAP = 8 << 20
CASE_CAP = 32 << 20
LEAF_SECONDS = 180
AUDIT_SECONDS = 30
CASE_SECONDS = 2400
SMI_ARGV = ['/opt/rocm/bin/amd-smi', 'process', '--gpu', '0000:05:00.0', '0000:15:00.0', '--json']
SMI_LAUNCHER = Path('/opt/rocm-7.2.1/libexec/amdsmi_cli/amdsmi_cli.py')
SMI_SHA = 'c6185991e96dc45b3ae930eace23869f070fce2afab5e061a336c0a7e2e9fa4a'
SCHEMA = 'ferric-p228-independent-gpu-observation-v1'
RECEIPT_FIELDS = set(('schema passed prepared case_index case request controller leaves '
    'before_audits after_audits checked failures native_attempts retained_captures profile_children '
    'retained_profile_files elapsed_seconds input_pins retries separately_owned_native_trees '
    'extra_owned_coordinator_tree production_authority performance_claim independent_numerical_acceptance '
    'full_model_correctness').split())


def arguments(index):
    V.require(type(index) is str and re.fullmatch(r'[0-5]', index), 'explicit one case index0..5')
    return int(index)


def resources(initial=False):
    V.require(shutil.disk_usage(E).free >= (40 if initial else 38) << 30, 'unchanged disk floor')


def inventory(directory, reserve=0):
    size, count = 0, 0
    for root, dirs, files in os.walk(directory, followlinks=False):
        for name in dirs + files:
            row = (Path(root) / name).lstat(); count += 1
            V.require(row.st_uid == os.getuid() and (stat.S_ISDIR(row.st_mode) or stat.S_ISREG(row.st_mode)),
                      'owned regular evidence tree')
            if stat.S_ISREG(row.st_mode):
                V.require(row.st_nlink == 1 and row.st_size <= STREAM_CAP, 'file/stream bound')
                size += row.st_size
    V.require(count + 1 <= 256 and size + reserve <= CASE_CAP, 'fixed32MiB case evidence cap')


def quiescent(owned):
    V.require(not any(row['ppid'] == os.getpid() for row in owned.processes().values()),
              'no child remains between independent ownership scopes')


def leaf_success(value):
    V.require(type(value['exit_code']) is int and value['exit_code'] == 0 and value['reason'] is None
        and value['owned_groups_absent'] is True and value['owned_processes_reaped'] is True
        and value['cleanup_signalled'] is False, 'owned leaf failed; no retry')


def child_limits(seconds):
    os.sched_setaffinity(0, {8, 9}); os.nice(10)
    for kind, value in ((resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, STREAM_CAP),
                        (resource.RLIMIT_CPU, seconds), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        ceiling = min(value, hard) if hard != resource.RLIM_INFINITY else value
        if soft != resource.RLIM_INFINITY: ceiling = min(ceiling, soft)
        resource.setrlimit(kind, (ceiling, ceiling))


def bounded(directory, argv, env, owned, case_root, gpu, seconds=LEAF_SECONDS):
    V.require(seconds in (LEAF_SECONDS, AUDIT_SECONDS) and type(gpu) is bool,
              'closed native/audit leaf bound')
    command = P.save(directory / 'command.json', dict(argv=argv, env=env, cwd=str(R),
        deadline_seconds=seconds, affinity=[8, 9], nice=10, address_space_bytes=12 << 30,
        stream_cap_bytes=STREAM_CAP, gpu_execution_requested=gpu))
    owned.subreaper()
    tracker, child, outcome, reason = owned.OwnedProcesses(), None, None, None
    started = None; start = time.monotonic()
    handlers = {n: signal.getsignal(n) for n in (signal.SIGINT, signal.SIGTERM)}
    try:
        for n in handlers: signal.signal(n, D.stop_signal)
        with (directory / 'stdout').open('xb') as stdout, (directory / 'stderr').open('xb') as stderr:
            child = subprocess.Popen(argv, cwd=R, env=env, stdin=subprocess.DEVNULL,
                stdout=stdout, stderr=stderr, start_new_session=True,
                preexec_fn=functools.partial(child_limits, seconds))
            tracker.attach(child)
            started = P.save(directory / 'started.json', dict(parent=tracker.root,
                supervisor_pid=os.getpid(), command_sha256=command['sha256']))
            while True:
                tracker.discover()
                V.require(time.monotonic() - start < seconds, 'owned leaf deadline')
                V.require(max(os.fstat(stdout.fileno()).st_size, os.fstat(stderr.fileno()).st_size)
                          <= STREAM_CAP, 'owned leaf stream cap')
                resources(); inventory(case_root)
                if os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT): break
                time.sleep(.2)
    except BaseException as error:
        reason = type(error).__name__ + ': ' + str(error)
    finally:
        try:
            if child is not None:
                for n in handlers: signal.signal(n, signal.SIG_IGN)
                try:
                    V.require(tracker.root is not None and owned.token(tracker.root) in tracker.records,
                              'reserved parent pidfd initialization')
                    outcome = tracker.cleanup()
                except BaseException as error:
                    reason = reason or ('primary cleanup failed: ' + repr(error))
                    outcome = owned.emergency_reap(child, tracker.events)
                if outcome['cleanup_signalled']: reason = reason or 'owned tree needed forced cleanup'
        finally:
            try: tracker.close_fds()
            finally:
                for n, handler in handlers.items(): signal.signal(n, handler)
    if outcome is None:
        V.require(child is None, 'spawned child without owned cleanup result')
        outcome = dict(exit_code=None, cleanup_signalled=False, owned_groups=[], owned_groups_absent=True,
                       owned_processes_reaped=True, lineage=[])
    value = dict(**outcome, reason=reason, command=command, started=started,
        stdout=D.read_file(directory / 'stdout', maximum=STREAM_CAP)[0],
        stderr=D.read_file(directory / 'stderr', maximum=STREAM_CAP)[0],
        elapsed_seconds=time.monotonic() - start, gpu_execution_requested=gpu)
    return value, P.save(directory / 'result.json', value)


def topology_identity(sample):
    V.require(type(sample) is dict and type(sample.get('devices')) is list
              and len(sample['devices']) == 2, 'paired topology sample')
    rows = []
    for row, uid in zip(sample['devices'], V.DEVICES):
        V.require(type(row['unique_id']) is int and row['unique_id'] == uid, 'ordered selected UID')
        rows.append({key: value for key, value in row.items()
                     if key not in ('gpu_busy', 'memory_busy', 'vram_used')})
    return rows


def check_platform(platform, sample):
    P.platform_review(platform)
    V.require(os.uname().nodename == platform['host']
        and Path('/proc/sys/kernel/random/boot_id').read_text().strip() == platform['boot_id']
        and topology_identity(sample) == platform['topology_identity'], 'current reviewed paired platform')


def process_audit(raw, platform):
    rows = V.empty_processes(raw)
    P.platform_review(platform)
    V.require([Path(row['device_path']).name for row in platform['topology_identity']]
              == SMI_ARGV[3:5], 'SMI selection joins actual reviewed physical BDFs')
    return rows


def capture_bytes(pins, requested, observed):
    V.require(type(observed.get('captures')) is list and len(observed['captures']) == 2,
              'both completed native capture pairs')
    result = {}
    directory = Path(requested['capture_directory'])
    for profile, name in enumerate(('baseline-v5', 'tiles-v6')):
        rows = observed['captures'][profile]
        V.require(type(rows) is list and len(rows) == 2, 'two retained ranks')
        for rank, record in enumerate(rows):
            path = directory / (name + '-rank' + str(rank) + '.bin')
            V.require(record['path'] == str(path) and record['bytes'] == V.CAPTURE_BYTES,
                      'closed capture filename/extent')
            result[str(path)] = P.read(pins, record, True, V.CAPTURE_BYTES)
    V.require(directory.is_dir() and not directory.is_symlink()
        and {str(path) for path in directory.iterdir()} == set(result), 'exact four capture files')
    return result


def prior_cases(c, base, index):
    if index == 0:
        V.require(not os.path.lexists(base), 'case0 begins exclusive matrix; no retry')
        return
    V.require(base.resolve(strict=True) == base and base.is_dir()
        and {p.name for p in base.iterdir()} == set(V.CASES[:index]), 'only passed preceding case directories')
    for previous in range(index):
        replay_case(c, c['pins'].pin(base / V.CASES[previous] / 'complete.json'))


def replay_case(c, receipt_pin):
    """Read-only closed-case replay for prior cases and the later CPU consumer."""
    value = P.document(c['pins'], receipt_pin, 8 << 20)
    V.keys(value, RECEIPT_FIELDS | set(c['provenance']))
    index = V.uint(value['case_index'], 5)
    base = E / c['inputs']['matrix_label']
    directory = base / V.CASES[index]
    V.require(receipt_pin['path'] == str(directory / 'complete.json')
        and directory.resolve(strict=True) == directory and directory.is_dir(),
        'exact selected independent case receipt')
    V.require(value['schema'] == SCHEMA and value['passed'] is True
        and value['prepared'] == c['prepared_pin'] and value['case'] == V.CASES[index]
        and value['request'] == c['prepared']['cases'][index]['request']
        and value['controller'] == c['pins'].pin(Path(__file__).resolve())
        and type(value['native_attempts']) is int and value['native_attempts'] == 1
        and type(value['retries']) is int and value['retries'] == 0 and value['failures'] == []
        and value['separately_owned_native_trees'] is True
        and value['extra_owned_coordinator_tree'] is False,
        'prior completed one-shot independent capture observation')
    for key in c['provenance']:
        V.require(value[key] == c['prepared'][key], 'exact new compiler/image/native provenance')
    for key in ('production_authority', 'performance_claim', 'independent_numerical_acceptance',
                'full_model_correctness'):
        V.require(value[key] is False, 'closed capture, never numerical or production authority')
    for name, record in value['input_pins'].items():
        V.require(name == record['path'], 'original input ledger key')
        P.read(c['pins'], record, maximum=1 << 30)
    V.keys(value['leaves'], {'inspection', 'native', *('before-' + str(i) for i in range(3)),
                            *('after-' + str(i) for i in range(3))})
    for key, prefix in (('before_audits', 'before'), ('after_audits', 'after')):
        V.require(type(value[key]) is list and len(value[key]) == 3, 'three actual audits on each side')
        samples = []
        for number, audit in enumerate(value[key]):
            V.keys(audit, 'topology process_result')
            name = prefix + '-' + str(number)
            V.require(audit['topology']['path'] == str(directory / (name + '-topology.json'))
                and audit['process_result']['path'] == str(directory / name / 'result.json'),
                'distinct ordered before/after audit records')
            sample = P.document(c['pins'], audit['topology'])
            check_platform(c['platform'], sample); samples.append(sample)
            V.require(audit['process_result'] == value['leaves'][name]['result'],
                      'audit result is the retained owned leaf')
            audited = replay_recorded_leaf(c, value, directory, name, SMI_ARGV, False, AUDIT_SECONDS)
            process_audit(P.read(c['pins'], audited['stdout'], True, STREAM_CAP), c['platform'])
        c['topology'].require_idle(samples)
    requested, baseline = c['requests'][index]
    results = {}
    for name, mode, gpu in (('inspection', O.INSPECT, False), ('native', O.EXECUTE, True)):
        results[name] = replay_recorded_leaf(c, value, directory, name,
            [c['prepared']['binary']['path'], mode, value['request']['path'], value['request']['sha256']],
            gpu, LEAF_SECONDS)
    inspect_result, native_result = results['inspection'], results['native']
    inspected = P.document(c['pins'], inspect_result['stdout'], 64 << 10)
    observed = P.document(c['pins'], native_result['stdout'], 64 << 10)
    children = C.validate(P, c['pins'], directory, value['request'],
        c['prepared']['binary'], observed, native_result)
    V.require(children == value['profile_children'] and C.records(P, c['pins'], directory, True)
        == value['retained_profile_files'], 'exact child controls, logs and actual owned lineage')
    checked = O.observation(observed, inspected, requested, value['request'], baseline,
        V.CASES[index], capture_bytes(c['pins'], requested, observed))
    V.require(checked == value['checked'], 'independent structural checks replayed from all capture bytes')
    expected_captures = {Path(record['path']).name: record for pair in observed['captures'] for record in pair}
    V.require(value['retained_captures'] == expected_captures, 'four exact closed retained captures')
    P.guard(c)
    return dict(receipt=value, receipt_pin=receipt_pin, request=requested, baseline=baseline,
        case_directory=str(directory), request_pin=value['request'], binary=c['prepared']['binary'],
        inspection_result=value['leaves']['inspection']['result'], native_result=value['leaves']['native']['result'])


def replay_recorded_leaf(c, receipt, directory, name, argv, gpu, seconds):
    row = receipt['leaves'][name]
    V.keys(row, 'result retained_files')
    names = {'command.json', 'started.json', 'stdout', 'stderr', 'result.json'}
    V.keys(row['retained_files'], names)
    V.require(row['result'] == row['retained_files']['result.json'], 'same retained owned leaf result')
    result = P.document(c['pins'], row['result'])
    for filename, record in row['retained_files'].items():
        V.require(record['path'] == str(directory / name / filename), 'exact retained leaf member path')
        if filename != 'result.json':
            V.require(result[filename.removesuffix('.json')] == record, 'owned result/file identity')
        P.read(c['pins'], record, maximum=STREAM_CAP)
    replay_leaf(c, result, argv, gpu, seconds)
    return result


def replay_leaf(c, result, argv, gpu, seconds):
    leaf_success(result)
    command = P.document(c['pins'], result['command'])
    V.require(command == dict(argv=argv, env=c['environment'], cwd=str(R), deadline_seconds=seconds,
        affinity=[8, 9], nice=10, address_space_bytes=12 << 30, stream_cap_bytes=STREAM_CAP,
        gpu_execution_requested=gpu), 'original exact bounded leaf command')
    started = P.document(c['pins'], result['started'])
    V.require(started['command_sha256'] == result['command']['sha256']
        and result['gpu_execution_requested'] is gpu and result['stderr']['bytes'] == 0,
        'original leaf start/mode/empty stderr')
    P.read(c['pins'], result['stderr'])
    P.read(c['pins'], result['stdout'], maximum=STREAM_CAP)


def execute(c, index):
    pins, owned, helper = c['pins'], c['owned'], c['topology']
    base = E / c['inputs']['matrix_label']; prior_cases(c, base, index)
    if index == 0: base.mkdir(mode=0o700)
    out = base / V.CASES[index]; out.mkdir(mode=0o700)
    requested, baseline = c['requests'][index]
    request_pin = c['prepared']['cases'][index]['request']
    V.require(requested['capture_directory'] == str(out / 'captures'), 'fixed per-case capture path')
    controller = pins.pin(Path(__file__).resolve())
    binary, env = c['prepared']['binary'], c['environment']
    leaves, before, after, errors, attempts, checked = {}, [], [], [], 0, None
    inspected = None; children = None; start = time.monotonic()
    handlers = {n: signal.getsignal(n) for n in (signal.SIGINT, signal.SIGTERM)}

    def recheck(reserve=0):
        V.require(time.monotonic() - start + reserve <= CASE_SECONDS, 'case deadline and reserved post-audit')
        resources(); inventory(out); P.guard(c)
        V.require(time.monotonic() - start + reserve <= CASE_SECONDS, 'bounded custody checks')

    def leaf(name, argv, gpu, seconds, is_audit=False):
        if is_audit:
            # Post-audits still run after a source/input guard failure.
            resources(); inventory(out)
        else:
            recheck(seconds + 3 * AUDIT_SECONDS)
        quiescent(owned)
        directory = out / name; directory.mkdir(mode=0o700)
        try:
            value, record = bounded(directory, argv, env, owned, out, gpu, seconds)
            leaves[name] = dict(result=record)
            for key in ('command', 'started', 'stdout', 'stderr'):
                if value[key] is not None: P.read(pins, value[key], maximum=STREAM_CAP)
            P.read(pins, record); leaf_success(value)
            V.require(value['stderr']['bytes'] == 0, 'unexpected leaf stderr')
            return value, P.read(pins, value['stdout'], True, STREAM_CAP)
        finally:
            retained = {}
            for name_on_disk in ('command.json', 'started.json', 'stdout', 'stderr', 'result.json'):
                path = directory / name_on_disk
                if path.exists(): retained[name_on_disk] = pins.pin(path)
            leaves.setdefault(name, {})['retained_files'] = retained
            quiescent(owned)

    def audit(name, destination):
        quiescent(owned)
        V.require(Path(SMI_ARGV[0]).resolve(strict=True) == SMI_LAUNCHER, 'actual canonical audit launcher')
        pins.pin(SMI_LAUNCHER, SMI_SHA)
        sample = helper.topology_sample(); check_platform(c['platform'], sample)
        sample_pin = P.save(out / (name + '-topology.json'), sample)
        _, raw = leaf(name, SMI_ARGV, False, AUDIT_SECONDS, is_audit=True)
        process_audit(raw, c['platform'])
        destination.append(dict(topology=sample_pin, process_result=leaves[name]['result']))
        quiescent(owned)
        return sample

    try:
        for n in handlers: signal.signal(n, D.stop_signal)
        resources(True); recheck(); quiescent(owned)
        _, raw = leaf('inspection', [binary['path'], O.INSPECT, request_pin['path'], request_pin['sha256']],
                      False, LEAF_SECONDS)
        inspected = O.inspection(V.parse(raw), requested, request_pin, baseline, V.CASES[index])
        samples = []
        for i in range(3):
            samples.append(audit('before-' + str(i), before))
            if i < 2: time.sleep(.2)
        helper.require_idle(samples)
        recheck(LEAF_SECONDS + 3 * AUDIT_SECONDS)
        attempts = 1
        native_result, raw = leaf('native', [binary['path'], O.EXECUTE,
                     request_pin['path'], request_pin['sha256']], True, LEAF_SECONDS)
        observed = V.parse(raw)
        children = C.validate(P, pins, out, request_pin, binary, observed, native_result)
        checked = O.observation(observed, inspected, requested, request_pin, baseline, V.CASES[index],
                                capture_bytes(pins, requested, observed))
    except BaseException as error:
        errors.append(type(error).__name__ + ': ' + str(error))
    finally:
        for n in handlers: signal.signal(n, signal.SIG_IGN)
        try:
            samples = []
            for i in range(3):
                try:
                    samples.append(audit('after-' + str(i), after))
                except BaseException as error:
                    errors.append('post-audit: ' + type(error).__name__ + ': ' + str(error))
                if i < 2: time.sleep(.2)
            try:
                helper.require_idle(samples); recheck(); quiescent(owned)
            except BaseException as error:
                errors.append('postcheck: ' + type(error).__name__ + ': ' + str(error))
        finally:
            for n, handler in handlers.items(): signal.signal(n, handler)
    # Retain any captures written after a real Close, including a mismatch failure.
    retained_captures = {}
    directory = out / 'captures'
    try:
        inventory(out)
        if directory.is_dir() and not directory.is_symlink():
            for path in sorted(directory.iterdir()):
                if path.is_file() and not path.is_symlink(): retained_captures[path.name] = pins.pin(path)
    except BaseException as error:
        errors.append('capture retention: ' + type(error).__name__ + ': ' + str(error))
    retained_profile_files = {}
    try:
        retained_profile_files = C.records(P, pins, out, False)
    except BaseException as error:
        errors.append('profile child retention: ' + type(error).__name__ + ': ' + str(error))
    passed = (not errors and checked is not None and children is not None
        and attempts == 1 and len(before) == len(after) == 3)
    receipt = dict(schema=SCHEMA, passed=passed,
        prepared=c['prepared_pin'], case_index=index, case=V.CASES[index], request=request_pin,
        controller=controller, leaves=leaves, before_audits=before, after_audits=after,
        checked=checked, failures=errors, native_attempts=attempts, retained_captures=retained_captures,
        profile_children=children, retained_profile_files=retained_profile_files,
        **c['provenance'],
        elapsed_seconds=time.monotonic() - start, input_pins=dict(pins.records), retries=0,
        separately_owned_native_trees=True, extra_owned_coordinator_tree=False,
        production_authority=False, performance_claim=False, independent_numerical_acceptance=False,
        full_model_correctness=False)
    raw = (__import__('json').dumps(receipt, sort_keys=True, indent=2) + '\n').encode()
    inventory(out, len(raw))
    record = P.save(out / ('complete.json' if passed else 'failure.json'), receipt)
    D.progress(dict(passed=passed, receipt=record))
    return 0 if passed else 1


def main():
    V.require(not sys.flags.optimize and len(sys.argv) == 4, 'PREPARED_PATH SHA INDEX')
    path, digest, index = sys.argv[1:]; index = arguments(index)
    V.require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == P.HOST
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 0,
        'fixed paired host, UID, affinity and controller nice')
    pins = D.Pins(); prepared_pin, _ = pins.read(Path(path), digest, maximum=8 << 20)
    raise SystemExit(execute(P.load(prepared_pin), index))


if __name__ == '__main__':
    main()
