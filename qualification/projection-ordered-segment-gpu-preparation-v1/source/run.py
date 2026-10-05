"""One explicitly selected default/shared AR4 arm; unchanged owned bounds and audits."""
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
import decode_validation as CV
import host_validation as HV
import ordered_validation as OV

D, V, E, R = I.D, I.V, I.E, I.R
NATIVE_SECONDS, AUDIT_SECONDS, CASE_SECONDS = 4000, 30, 4300
STREAM_CAP, CASE_CAP = 8 << 20, 64 << 20


def route(name):
    V.require(type(name) is str and name in ('default', 'shared', 'ordered'), 'explicit closed observer route')
    if name == 'ordered':
        return '--observe-projection-residual-mlp-ordered', 'native-projection-residual-mlp-ordered-observation.json', HV.validate_ordered
    if name == 'default':
        return '--observe-projection-host', 'native-projection-host-observation.json', HV.validate
    return '--observe-projection-shared-host', 'native-projection-shared-host-observation.json', HV.validate_shared


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
    allowed = CV.BODY | {'complete.json'}
    V.require(names <= allowed and (not complete or names == allowed), 'closed native body and summary roster')
    records, bodies = {}, {}
    for name in sorted(names):
        record, raw = pins.read(directory / name, retain=complete,
                                maximum=(65536 if name == 'complete.json' else V.LIMIT))
        records[name] = record
        if complete and name != 'complete.json': bodies[name] = raw
    return records, bodies


def lineage(pins, native, checked):
    start = I.doc(pins, native['started']); parent = start['parent']
    V.require(parent['pid'] == parent['pgid'] == parent['sid'] and parent['uid'] == 9661
        and parent['ppid'] == start['supervisor_pid'], 'actual owned model parent')
    rows = [r['identity'] for r in native['lineage'] if r.get('event') == 'owned']
    pids = checked['closed_child_pids']
    V.require(type(pids) is list and len(pids) == 1, 'one capture worker')
    V.require(parent in rows and parent['pid'] not in pids
        and 1 <= len(rows) <= 2 and len({(r['pid'], r['starttime']) for r in rows}) == len(rows)
        and {r['pid'] for r in rows} <= {parent['pid'], *pids}, 'only model parent and one known worker')
    result = []
    for pid in pids:
        found = [r for r in rows if r['pid'] == pid]
        V.require(len(found) <= 1, 'no worker PID incarnation ambiguity')
        if found:
            r = found[0]
            V.require(r['ppid'] == parent['pid'] and r['pid'] == r['pgid'] and r['sid'] == parent['sid']
                and r['uid'] == parent['uid'] and r['starttime'] >= parent['starttime'],
                'actual worker private-group/inherited-session lineage')
        result.append(dict(pid=pid, identity=found[0] if found else None, outer_pidfd_observed=bool(found)))
    return dict(parent=parent, workers=result, parent_asserted_close_and_reap=True,
        outer_groups_absent=True, synthesized_child_identity=False)


def execute(c):
    flag, sidecar_name, host_validator = route(c['route'])
    decoder = OV if c['route'] == 'ordered' else CV
    pins, owned, O, helper = c['pins'], c['owned'], c['O'], c['topology']
    out = c['out']; V.require(not os.path.lexists(out), 'fresh one-shot layer directory')
    resources(True); I.guard(c); O.quiescent(owned); out.mkdir(mode=0o700)
    leaves, before, after, errors = {}, [], [], []
    attempts, checked, identities = 0, None, None
    host_record, host_checked = None, None
    started = time.monotonic(); native_records = {}
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
            '--allow-unauthenticated-machine-code', flag], True, NATIVE_SECONDS)
        native_records, bodies = retained(pins, out / 'native', True)
        summary = I.read(pins, native_records['complete.json'], 65536)
        checked = decoder.validate(summary, bodies, c['request'])
        decoder.child_marker(I.read(pins, value['stderr'], STREAM_CAP), checked['closed_child_pids'][0])
        identities = lineage(pins, value, checked)
        host_record, host_raw = pins.read(out / sidecar_name,
                                          retain=True, maximum=HV.MAX_BYTES)
        host_checked = host_validator(raw, host_raw, host_record, summary, V.parse(summary), c['runtime']['parent'])
        I.save(out / 'observation.json', checked)
        I.save(out / 'host-observation.json', host_checked)
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
        sidecar = out / sidecar_name
        if sidecar.exists() or sidecar.is_symlink():
            host_record, _ = pins.read(sidecar, maximum=HV.MAX_BYTES)
    except BaseException as error: errors.append('native evidence retention: ' + repr(error))
    passed = (not errors and attempts == 1 and checked is not None and identities is not None
              and host_checked is not None and len(before) == len(after) == 3)
    value = dict(schema=('ferric-p228-projection-ordered-segment-gpu-v1' if c['route'] == 'ordered' else
        'ferric-p228-projection-ar4-shared-host-gpu-v1'), route=c['route'], passed=passed, plan=c['plan_pin'],
        controller=pins.pin(Path(__file__).resolve()), supervisor_manifest=c['supervisor_manifest'],
        baseline=c['plan']['baseline'], projection_image=c['plan']['projection_image'],
        layer_comparison=c['plan']['layer_comparison'], mlp_image=c['plan']['mlp_image'], mlp_cpu=c['plan']['mlp_cpu'],
        mlp_lowering_complete=c['plan']['mlp_lowering_complete'], mlp_lowering_owner=c['plan']['mlp_lowering_owner'],
        prefix_image=c['plan']['prefix_image'], prefix_cpu=c['plan']['prefix_cpu'],
        prefix_emission_complete=c['plan']['prefix_emission_complete'], prefix_emission_owner=c['plan']['prefix_emission_owner'],
        prefix_provenance=c['prefix_provenance'],
        lowering_complete=c['plan']['lowering_complete'], inspection_complete=c['plan']['inspection_complete'],
        parent_cpu_complete=c['plan']['parent_cpu'], worker_cpu_complete=c['plan']['worker_cpu'],
        parent=c['runtime']['parent'], worker=c['runtime']['worker'], selected_runtime=c['runtime'],
        parent_runtime_review=c['plan']['parent_runtime_review'],
        worker_runtime_review=c['plan']['worker_runtime_review'], decode_review=c['plan']['decode_review'],
        request=c['plan']['request'], leaves=leaves, before_audits=before, after_audits=after,
        retained_native=native_records, checked=checked,
        host_sidecar=host_record, host_checked=host_checked,
        host_observation=(pins.pin(out / 'host-observation.json') if (out / 'host-observation.json').exists() else None),
        observation=(pins.pin(out / 'observation.json') if (out / 'observation.json').exists() else None),
        owned_children=identities, failures=errors, native_attempts=attempts, retries=0,
        elapsed_seconds=time.monotonic() - started, input_pins=dict(pins.records),
        standalone_input_pins=dict(c['standalone']['pins'].records),
        gpu_execution_requested=attempts == 1, old_native_equality_required=False,
        own_output_trajectory_checked=passed, teacher_forced_token_parity_required=False,
        native_baseline_comparison_performed=False, conditional_residual_checks_performed=False,
        captured_tensor_rows=(152 if passed else None), captured_payloads=(4 if passed else None),
        paired_comparison_performed=False,
        full_model_correctness=False, independent_numerical_acceptance=False,
        numerical_acceptance=False, independent_tensor_acceptance=False, full_model_acceptance=False,
        independent_framework_comparison_performed=False, full_forward=passed, sustained_2048_256=False,
        gpu_time=False, calibrated_nanoseconds=False, cross_device_clock_alignment=False,
        full_currentness_policy_unchanged=c['route'] == 'default', fresh_full_currentness_preserved=True,
        intermediate_host_fence_removed=c['route'] == 'ordered',
        per_kernel_publish_wait_poll_comparable=False if c['route'] == 'ordered' else None,
        configuration_time_in_snapshots=False, inclusive_nested_host_scopes=True,
        overlap_claim=False, performance_claim=False, production_authority=False)
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
