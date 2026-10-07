#!/usr/bin/env python3
"""Fresh component-only outer supervisor; no implicit native launch or promotion."""
import argparse
import fcntl
import importlib.util
import os
from pathlib import Path
import resource
import signal
import socket
import stat

D = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('component_contract', D / 'component_contract.py')
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)


def clean_completion(result, admission, stage, plan, plan_path, plan_sha, output):
    try:
        result['postflight'] = admission.check('postflight')
        c.require(result['postflight'].get('accepted') is True, 'positive idle postflight required')
        c.read(plan_path, plan_sha)
        c.verify_files(stage, plan['files'])
        if result['status'] == 0:
            c.require(result['cleanup_ok'] and result['child_reaped']
                      and result.get('owned_group_absent') is True
                      and not result['term_sent'] and not result['kill_sent'] and not result['errors'],
                      'clean unsignaled owned-group completion required')
            complete = c.decode(c.read(output / 'completion.json', maximum=1024**2)[0])
            c.require(complete.get('schema') == 'FerricM32ComponentCompletionV1'
                      and complete.get('mode') == plan['mode']
                      and complete.get('accepted') is True and complete.get('input_files_stable') is True,
                      'accepted component completion required')
            inner = c.decode(c.read(output / 'component-results/result.json',
                                   complete['component_result_sha256'], 32 * 1024**2)[0])
            c.read(output / 'process-endpoints.json', complete['endpoints_sha256'], 32 * 1024**2)
            c.require(inner.get('schema') == 'FerricSupervisedM32ComponentV1'
                      and inner.get('mode') == plan['mode']
                      and inner.get('accepted') is True and inner.get('clean_teardown') is True
                      and inner.get('source_sha256') == plan['sources'], 'exact component inner receipt')
            result['completion_sha256'] = c.read(output / 'completion.json')[1]
    except BaseException as error:
        result['errors'].append('postflight: ' + type(error).__name__ + ': ' + str(error))
        result.update(status=125, cleanup_ok=False)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--plan-sha256', required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--supervise', action='store_true')
    mode.add_argument('--execute', action='store_true')
    args = parser.parse_args()
    c.require(c.NATIVE_ENABLED, 'M32 component is not qualified/enabled')
    os.umask(0o077)
    c.require(socket.gethostname() == c.HOST and os.getuid() == c.UID, 'approved component host and UID')
    stage = c.stage_name(D)
    c.require(args.plan == stage / 'plan.json', 'one immutable component plan location')
    plan = c.decode(c.read(args.plan, args.plan_sha256, 32 * 1024**2)[0])
    supervisor, profile, lifecycle, evidence = c.validate_plan(plan, stage)
    admission_module = c.module(stage / 'component_admission.py', plan['sources']['component_admission.py'])
    output = stage / 'outputs/component'
    if args.execute:
        admission = admission_module.Admission(stage, plan, supervisor, profile, output)
        entry = c.module(stage / 'component_entry.py', plan['sources']['component_entry.py'])
        return entry.execute(stage, plan, admission_module, admission, c.load_component(stage),
                             output, lifecycle, evidence)
    lock = os.open(stage / 'native.lock', os.O_CREAT | os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
    try:
        item = os.fstat(lock)
        c.require(stat.S_ISREG(item.st_mode) and item.st_uid == c.UID and item.st_nlink == 1
                  and stat.S_IMODE(item.st_mode) == 0o600, 'owned native launch lock')
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        admission_module.stage_identity(stage)
        admission_module.fresh_output(stage, output)
        admission = admission_module.Admission(stage, plan, supervisor, profile, output)
        owner = lifecycle.process(os.getpid())
        os.environ.update(PATH='/usr/bin:/bin', LC_ALL='C', PYTHONDONTWRITEBYTECODE='1', PYTHONNOUSERSITE='1',
            FERRIC_COMPONENT_SUPERVISOR_PID=str(owner['pid']),
            FERRIC_COMPONENT_SUPERVISOR_START_TICKS=str(owner['start']))
        for key in ('LD_PRELOAD', 'LD_LIBRARY_PATH', 'PYTHONPATH'):
            os.environ.pop(key, None)
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        resource.setrlimit(resource.RLIMIT_AS, (128 * 1024**3, 128 * 1024**3))
        stopped = []
        for number in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            signal.signal(number, lambda value, _frame: stopped.append(value) if not stopped else None)
        argv = [plan['python']['path'], '-I', '-B', str(stage / 'supervise_component.py'), '--execute',
                '--plan', str(args.plan), '--plan-sha256', args.plan_sha256]
        result = lifecycle.supervise_group(supervisor, argv, output,
            lambda initial: admission.check('preflight' if initial else 'monitor', initial=initial),
            lambda: stopped[0] if stopped else None, duration=1500, grace=20, kill_wait=5, interval=3)
        result['schema'] = 'FerricM32ComponentOuterReceiptV1'
        result['mode'] = plan['mode']
        result['plan_sha256'] = args.plan_sha256
        clean_completion(result, admission, stage, plan, args.plan, args.plan_sha256, output)
        supervisor.save(output / 'launch-supervisor.json', result)
        with (output / 'launch.status').open('x') as stream:
            stream.write(str(result['status']) + '\n')
        return result['status']
    finally:
        os.close(lock)


if __name__ == '__main__':
    raise SystemExit(main())
