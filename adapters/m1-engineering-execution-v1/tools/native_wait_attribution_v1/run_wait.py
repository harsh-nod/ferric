"""One supervised baseline request with a freshly built observation-only worker."""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import resource
import signal
import socket
import stat

BASE = Path('/dev/shm/ferric-native-gate-up-a004')
OLD = Path('/dev/shm/ferric-native-gate-up-decode-a001')
CONTRACT_SHA = '97f83d65302fd0d8fd276c57dc1454e4daf743259f733b0907c16b44eb542815'
DRIVER_SHA = '614df5e782036ecd86206081b48ee89c7634818d1ae25a984f7d0ab0a0421430'
GUARD_SHA = '36d8eef5890c13c3a0ed6d1a99d327796ca9277bb3265cab0f5b21d3fe66fbed'
WORKER_SHA = '2464e21d04f32063008c5142f2dc8cb1a9ccf5981a09cce0882991f6a031fa4a'
RUNTIME = '179629a7310e31dde354afa6465f304f09ec87fb'
SOURCES = ('run_wait.py', 'capture_binding.py', 'wait_observation.py',
           'test_capture_binding.py', 'test_wait_observation.py', 'qualify_capture.py')


def require(ok, why):
    if not ok:
        raise ValueError(why)


def module(path, pin):
    raw = path.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == pin, 'pinned loader input')
    value = importlib.util.module_from_spec(importlib.util.spec_from_file_location(path.stem, path))
    exec(compile(raw, str(path), 'exec'), value.__dict__)
    return value


def clean_cpu(value):
    require(value['status'] == value['returncode'] == 0 and value['cleanup_ok'] is True
            and value['child_reaped'] is True and value['term_sent'] is False
            and value['kill_sent'] is False and value['errors'] == []
            and value['profile'] == 'FerricCpuFourCore45GiBEmitterV1', 'clean bounded CPU closure')


def validate(c, driver, stage, plan):
    require(plan['schema'] == 'FerricNativeWaitCapturePlanV1' and plan['stage'] == str(stage)
            and plan['performance_qualified'] is False and plan['latency_sample_admitted'] is False
            and plan['requests'] == 1, 'one diagnostic-only request')
    c.verify_files(stage, plan['files'])
    require(plan['sources'] == {name: plan['files'][name]['sha256']
            for name in (*SOURCES, 'measurement/gpu_activity.py')}, 'closed source bindings')
    require(plan['original_plan']['path'] == str(OLD / 'plan.json'), 'qualified controller plan path')
    original = c.bound(plan['original_plan'])
    context = driver.validate(c, OLD, original, 'A')
    parent, loaded, counter, evidence, cell, check, selected, _, _ = context
    binding = c.module(stage / 'capture_binding.py', plan['sources']['capture_binding.py'])
    observation = c.module(stage / 'wait_observation.py', plan['sources']['wait_observation.py'])
    worker = c.decode(c.read(stage / 'worker-cpu/receipt.json')[0])
    require(worker['schema'] == 'FerricNativeWaitWorkerBuildV1' and worker['returncode'] == 0
            and worker['source_unchanged'] is True and worker['native_executed'] is False
            and worker['source_binding']['git_revision'] == RUNTIME
            and worker['binary']['sha256'] == WORKER_SHA, 'exact new diagnostic worker build')
    require(worker['argv'][-6:] == ['fe2o3-kfd', '--no-default-features', '--features',
            'engineering-native-wait-diagnostics', '--bin', 'fe2o3-gfx950-engineering-worker'],
            'diagnostic feature is opt-in')
    clean_cpu(c.decode(c.read(stage / 'worker-cpu/result.json')[0]))
    c.read(stage / 'worker-candidate', WORKER_SHA, 32 * 1024**2)
    for name in ('stdout', 'stderr'):
        c.read(stage / 'worker-cpu' / name, worker[name + '_sha256'], 1024**2, empty=True)
    harness = c.decode(c.read(stage / 'harness-cpu/receipt.json')[0])
    require(harness['accepted'] is True and harness['returncode'] == 0
            and harness['source_before'] == harness['source_after'] ==
                {name: plan['sources'][name] for name in SOURCES}
            and harness['expected_tests'] == 28, 'exact current capture CPU qualification')
    clean_cpu(c.decode(c.read(stage / 'harness-cpu/result.json')[0]))
    for name in ('stdout', 'stderr'):
        c.read(stage / 'harness-cpu' / name, harness[name + '_sha256'], 1024**2, empty=True)
    selected = binding.derive(selected, stage, WORKER_SHA)
    cell.shape(selected['spec'])
    _, _, legacy, _, _, runner, _, _ = loaded
    options = c.parse_common(selected['common_args'], runner)
    identities = legacy.inputs({**parent['inputs'], 'controller': selected['spec']['controller'],
                                'images': parent['images']}, options, runner)
    return (parent, loaded, counter, evidence, cell, binding.Replay(check, observation),
            selected, options, identities)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan-sha256', required=True)
    parser.add_argument('--execute', action='store_true')
    parser.add_argument('--validate-only', action='store_true')
    args = parser.parse_args()
    require(not (args.execute and args.validate_only), 'one execution mode')
    os.umask(0o077)
    c = module(BASE / 'launch_contract.py', CONTRACT_SHA)
    require(socket.gethostname() == c.HOST and os.getuid() == c.UID, 'authorized native host')
    stage = c.stage_name(Path(__file__).resolve().parent)
    plan = c.decode(c.read(stage / 'plan.json', args.plan_sha256)[0])
    driver = c.module(OLD / 'run_decode.py', DRIVER_SHA)
    guard = c.module(BASE / 'run_stage.py', GUARD_SHA)
    locks = []
    try:
        if not args.execute:
            for root in (BASE, OLD, stage):
                fd = os.open(root / 'native.lock', os.O_CREAT | os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
                locks.append(fd)
                st = os.fstat(fd)
                require(stat.S_ISREG(st.st_mode) and st.st_uid == c.UID and st.st_nlink == 1
                        and stat.S_IMODE(st.st_mode) == 0o600, 'owned launch lock')
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        context = validate(c, driver, stage, plan)
        if args.validate_only:
            context[1][6].resource_snapshot(stage)
            print(json.dumps({'inputs_validated': True, 'native_executed': False}), flush=True)
            return 0
        if args.execute:
            return driver.execute(c, stage, plan, context, guard)
        _, loaded, _, _, cell, _, selected, _, _ = context
        _, _, _, _, profile, runner, supervisor, _ = loaded
        output = Path(selected['output'])
        guard.fresh_output(stage, output)
        admission = guard.Admission(stage, plan, supervisor, profile, output)
        os.environ.update(PATH='/usr/bin:/bin', LC_ALL='C', PYTHONDONTWRITEBYTECODE='1',
                          PYTHONNOUSERSITE='1', FERRIC_V14_SUPERVISOR_PID=str(os.getpid()))
        for key in ('LD_PRELOAD', 'LD_LIBRARY_PATH', 'PYTHONPATH'):
            os.environ.pop(key, None)
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        resource.setrlimit(resource.RLIMIT_AS, (128 * 1024**3, 128 * 1024**3))
        stopped = []
        for number in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            signal.signal(number, lambda value, _frame: stopped.append(value) if not stopped else None)
        argv = ['/usr/bin/python3', '-I', '-B', str(stage / 'run_wait.py'), '--execute',
                '--plan-sha256', args.plan_sha256]
        result = cell.lifecycle.supervise_group(supervisor, argv, output,
            lambda initial: admission.check('preflight' if initial else 'monitor', initial=initial),
            lambda: stopped[0] if stopped else None, duration=1500, grace=20, kill_wait=5, interval=3)
        try:
            result['postflight'] = admission.check('postflight')
            c.read(stage / 'plan.json', args.plan_sha256)
            c.verify_files(BASE, context[0]['files'])
            c.verify_files(stage, plan['files'])
            if result['status'] == 0:
                require(result['cleanup_ok'] and result['child_reaped'] and not result['term_sent']
                        and not result['kill_sent'] and not result['errors'], 'clean unsignaled outer')
                complete = c.decode(c.read(output / 'completion.json')[0])
                require(complete['accepted'] is True, 'accepted wait-capture completion')
                result['completion_sha256'] = c.read(output / 'completion.json')[1]
        except BaseException as error:
            result['errors'].append('postflight: ' + type(error).__name__ + ': ' + str(error))
            result.update(status=125, cleanup_ok=False)
        runner.save(output / 'launch-supervisor.json', result)
        with (output / 'launch.status').open('x') as target:
            target.write(str(result['status']) + '\n')
        return result['status']
    finally:
        for fd in reversed(locks):
            os.close(fd)


if __name__ == '__main__':
    raise SystemExit(main())
