"""CPU522 Four capture observation with unchanged ownership/audit bounds."""
import functools
import os
from pathlib import Path
import resource
import shutil
import signal
import stat
import subprocess
import sys
import time

import intake as I

D, V, E, R = I.D, I.V, I.E, I.R
NATIVE_SECONDS, AUDIT_SECONDS, CASE_SECONDS = 4000, 30, 4300
STREAM_CAP, CASE_CAP = 8 << 20, 64 << 20


def resources(initial=False):
    V.require(shutil.disk_usage(E).free >= (40 if initial else 38) << 30, 'unchanged disk floor')


def inventory(directory, reserve=0):
    size, count = 0, 0
    def unreadable(error):
        raise error
    for root, dirs, files in os.walk(directory, followlinks=False, onerror=unreadable):
        for name in dirs + files:
            row = (Path(root) / name).lstat(); count += 1
            V.require(row.st_uid == os.getuid() and (stat.S_ISDIR(row.st_mode) or stat.S_ISREG(row.st_mode)),
                      'owned regular layer evidence tree')
            if stat.S_ISREG(row.st_mode):
                V.require(row.st_nlink == 1 and row.st_size <= CASE_CAP, 'regular bounded evidence file')
                size += row.st_size
    V.require(count + 1 <= 256 and size + reserve <= CASE_CAP, 'whole64MiB layer case evidence cap')


def child_limits(seconds):
    os.sched_setaffinity(0, {8, 9}); os.nice(10)
    for kind, value in ((resource.RLIMIT_AS, (32 if seconds == NATIVE_SECONDS else 12) << 30),
                        (resource.RLIMIT_FSIZE, CASE_CAP), (resource.RLIMIT_CPU, seconds), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        ceiling = min(value, hard) if hard != resource.RLIM_INFINITY else value
        if soft != resource.RLIM_INFINITY: ceiling = min(ceiling, soft)
        resource.setrlimit(kind, (ceiling, ceiling))


def bounded(directory, argv, env, owned, case_root, gpu, seconds):
    V.require((gpu is True and seconds == NATIVE_SECONDS) or (gpu is False and seconds == AUDIT_SECONDS),
              'closed native or process-audit leaf')
    command = I.save(directory / 'command.json', dict(argv=argv, env=env, cwd=str(R),
        deadline_seconds=seconds, affinity=[8, 9], nice=10,
        address_space_bytes=(32 if gpu else 12) << 30, file_cap_bytes=CASE_CAP,
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
            started = I.save(directory / 'started.json', dict(parent=tracker.root,
                supervisor_pid=os.getpid(), command_sha256=command['sha256']))
            while True:
                tracker.discover()
                V.require(time.monotonic() - start < seconds, 'owned layer leaf deadline')
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
    return value, I.save(directory / 'result.json', value)


def retained(pins, directory, complete=False):
    if not directory.exists():
        V.require(not directory.is_symlink() and not complete, 'native evidence missing or aliased')
        return {}, {}
    V.require(directory.resolve(strict=True) == directory and directory.is_dir(), 'canonical native evidence')
    names = {path.name for path in directory.iterdir()}
    allowed = I.BODY | {'complete.json'}
    V.require(names <= allowed and (not complete or names == allowed), 'closed native body and summary roster')
    records, bodies = {}, {}
    for name in sorted(names):
        record, raw = pins.read(directory / name, retain=complete,
                                maximum=(65536 if name == 'complete.json' else 2 << 20))
        records[name] = record
        if complete and name != 'complete.json': bodies[name] = raw
    return records, bodies


def retained_host(pins, out, complete=False):
    path = out / 'native-host-policy-v2.json'
    if not os.path.lexists(path):
        V.require(not complete, 'successful diagnostic requires host sidecar')
        return None
    # Retain bounded failure bytes even if too large or malformed for successful decode.
    record, _ = pins.read(path, retain=complete, maximum=(65536 if complete else STREAM_CAP))
    return record


def checked_observation(c, value):
    V.keys(value, 'schema status structural_observation_complete mode policy request prefix_image '
        'structural owned_record_checks host_observation captured_tensor_rows inputs '
        'recorded_close_and_owner_reap_checked gpu_launched native_baseline_comparison_performed '
        'independent_framework_comparison_performed numerical_acceptance independent_tensor_acceptance '
        'independent_tensor_threshold full_model_acceptance current_source_binary_image_authority_verified '
        'current_platform_idle_audits_verified top_level_observer_reaping_verified sustained_2048_256 '
        'performance_claim production_authority')
    V.require(value['schema'] == 'ferric-p228-prefix-decode-independent-observation-v1'
        and value['status'] == 'FINITE_FOUR_FORWARD_OBSERVED'
        and value['structural_observation_complete'] is True
        and value['recorded_close_and_owner_reap_checked'] is True
        and type(value['captured_tensor_rows']) is int and value['captured_tensor_rows'] == 152,
        'complete independent Four structural observation, never legacy parity')
    V.require(value['mode'] == c['request']['mode'] and value['policy'] == c['policy']
        and value['request'] == c['plan']['request']
        and value['prefix_image'] == c['selected_runtime']['image'],
        'observed request, mode, policy, and selected V7 image')
    V.require(type(value['owned_record_checks']) is dict and bool(value['owned_record_checks']),
        'validated recorded native owner and Close checks')
    V.require(value['independent_tensor_threshold'] is None
        and all(value[key] is False for key in ('gpu_launched',
            'native_baseline_comparison_performed', 'independent_framework_comparison_performed',
            'numerical_acceptance', 'independent_tensor_acceptance', 'full_model_acceptance',
            'current_source_binary_image_authority_verified', 'current_platform_idle_audits_verified',
            'top_level_observer_reaping_verified', 'sustained_2048_256',
            'performance_claim', 'production_authority')),
        'structural helper grants no numerical, provenance, production, or performance authority')
    return value['owned_record_checks']


def execute(c):
    pins, owned, O, helper = c['pins'], c['owned'], c['O'], c['topology']
    out = c['out']; V.require(not os.path.lexists(out), 'fresh one-shot layer directory')
    resources(True); I.guard(c); O.quiescent(owned); out.mkdir(mode=0o700)
    leaves, before, after, errors = {}, [], [], []
    attempts, checked, identities = 0, None, None
    started = time.monotonic(); native_records = {}; host_sidecar = None
    handlers = {n: signal.getsignal(n) for n in (signal.SIGINT, signal.SIGTERM)}
    def guard(reserve=0):
        V.require(time.monotonic() - started + reserve <= CASE_SECONDS, 'bounded layer case with post-audit reserve')
        resources(); inventory(out); I.guard(c)
        V.require(time.monotonic() - started + reserve <= CASE_SECONDS, 'bounded evidence recheck')
    def leaf(name, argv, gpu, seconds, post=False):
        if post: resources(); inventory(out)
        else: guard(seconds + 3 * AUDIT_SECONDS)
        O.quiescent(owned); directory = out / name; directory.mkdir(mode=0o700)
        try:
            value, record = bounded(directory, argv, c['environment'], owned, out, gpu, seconds)
            leaves[name] = dict(result=record)
            for key in ('command', 'started', 'stdout', 'stderr'):
                if value[key] is not None: I.read(pins, value[key], STREAM_CAP, False)
            I.read(pins, record); I.owned_success(value)
            if not gpu: V.require(value['stderr']['bytes'] == 0, 'unexpected process-audit stderr')
            return value, I.read(pins, value['stdout'], STREAM_CAP)
        finally:
            rows = {}
            for name_on_disk in ('command.json', 'started.json', 'stdout', 'stderr', 'result.json'):
                path = directory / name_on_disk
                if path.exists(): rows[name_on_disk] = pins.pin(path)
            leaves.setdefault(name, {})['retained_files'] = rows
            O.quiescent(owned)
    def audit(name, rows, post=False):
        O.quiescent(owned)
        sample = helper.topology_sample(); O.check_platform(c['platform'], sample)
        record = I.save(out / (name + '-topology.json'), sample)
        _, raw = leaf(name, O.SMI_ARGV, False, AUDIT_SECONDS, post)
        O.process_audit(raw, c['platform'])
        rows.append(dict(topology=record, process_result=leaves[name]['result']))
        return sample
    try:
        for n in handlers: signal.signal(n, D.stop_signal)
        samples = []
        for n in range(3):
            samples.append(audit('before-' + str(n), before))
            if n < 2: time.sleep(.2)
        helper.require_idle(samples)
        guard(NATIVE_SECONDS + 3 * AUDIT_SECONDS); attempts = 1
        value, raw = leaf('parent', [c['runtime']['parent']['path'], '--request', c['plan']['request']['path'],
            '--allow-unauthenticated-machine-code', '--observe-host-policy'], True, NATIVE_SECONDS)
        native_records, _ = retained(pins, out / 'native', True)
        host_sidecar = retained_host(pins, out, True)
        checked = I.compare_native(c, native_records, host_sidecar, leaves['parent']['result'], value)
        I.save(out / 'observation.json', checked)
        identities = checked_observation(c, checked)
    except BaseException as error:
        errors.append(type(error).__name__ + ': ' + str(error))
    finally:
        for n in handlers: signal.signal(n, signal.SIG_IGN)
        try:
            samples = []
            for n in range(3):
                try: samples.append(audit('after-' + str(n), after, True))
                except BaseException as error: errors.append('post-audit: ' + repr(error))
                if n < 2: time.sleep(.2)
            try: helper.require_idle(samples); guard(); O.quiescent(owned)
            except BaseException as error: errors.append('postcheck: ' + repr(error))
        finally:
            for n, handler in handlers.items(): signal.signal(n, handler)
    try:
        inventory(out); native_records, _ = retained(pins, out / 'native')
    except BaseException as error: errors.append('native evidence retention: ' + repr(error))
    try:
        host_sidecar = retained_host(pins, out)
    except BaseException as error: errors.append('host evidence retention: ' + repr(error))
    passed = not errors and attempts == 1 and checked is not None and identities is not None and len(before) == len(after) == 3
    base_deployment = c['base_deployment']
    value = dict(schema='ferric-p228-resident-state-decode-observation-v1', passed=passed, plan=c['plan_pin'],
        controller=pins.pin(Path(__file__).resolve()), supervisor_manifest=c['supervisor_manifest'],
        deployment=c['plan']['deployment'], image_deployment=c['plan']['image_deployment'],
        base_deployment=c['deployment']['base_deployment'], prior_deployment=c['plan']['prior_deployment'],
        prior_worker_cpu_complete=c['prior_deployment']['worker_cpu'],
        prior_worker_cpu_review=c['prior_deployment']['worker_cpu_review'],
        parent_cpu_complete=base_deployment['cpu']['complete'], parent_cpu_review=base_deployment['cpu_review'],
        worker_cpu_complete=c['deployment']['worker_cpu'], worker_cpu_review=c['deployment']['worker_cpu_review'],
        parent=c['runtime']['parent'], worker=c['runtime']['worker'],
        selected_runtime=c['selected_runtime'], historical_runtime=c['historical_runtime'],
        standalone_prepared=c['plan']['standalone_prepared'], standalone_cases=c['standalone_receipts'],
        numericals=c['plan']['numericals'],
        request=c['plan']['request'], mode=c['request']['mode'], policy=c['policy'], leaves=leaves,
        before_audits=before, after_audits=after, retained_native=native_records, host_sidecar=host_sidecar, checked=checked,
        observation=(pins.pin(out / 'observation.json') if (out / 'observation.json').exists() else None),
        owned_children=identities, failures=errors, native_attempts=attempts, retries=0,
        elapsed_seconds=time.monotonic() - started, input_pins=dict(pins.records),
        standalone_input_pins=dict(c['standalone']['pins'].records),
        gpu_execution_requested=attempts == 1, full_model_correctness=False,
        independent_numerical_acceptance=False, numerical_acceptance=False, independent_tensor_acceptance=False,
        full_model_acceptance=False, native_baseline_comparison_performed=False,
        independent_framework_comparison_performed=False, sustained_2048_256=False, gpu_time=False,
        inclusive_nested_host_scopes=True, performance_claim=False, production_authority=False)
    inventory(out, len((__import__('json').dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()))
    record = I.save(out / ('complete.json' if passed else 'failure.json'), value)
    D.progress(dict(passed=passed, receipt=record))
    return 0 if passed else 1


def main(args):
    V.require(len(args) == 2 and not sys.flags.optimize, 'PLAN_PATH PLAN_SHA')
    V.require(os.getuid() == os.geteuid() == 9661 and os.sched_getaffinity(0) == {8, 9}
        and os.getpriority(os.PRIO_PROCESS, 0) == 0, 'fixed numerical UID/affinity/controller nice')
    pins = D.Pins(); record, _ = pins.read(Path(args[0]), args[1], maximum=1 << 20)
    raise SystemExit(execute(I.context(record)))


if __name__ == '__main__':
    main(sys.argv[1:])
