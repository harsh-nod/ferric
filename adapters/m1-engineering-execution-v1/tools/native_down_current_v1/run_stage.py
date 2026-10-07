"""Run fresh native down ABBA cells through unchanged admission and measurement."""
import argparse
import fcntl
import importlib.util
import os
from pathlib import Path
import resource
import signal
import socket
import stat


def local_module(path):
    spec = importlib.util.spec_from_file_location('current_' + path.stem, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def main():
    stage = Path(__file__).resolve().parent
    binding = local_module(stage / 'current_binding.py')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--plan-sha256', required=True)
    parser.add_argument('--cell-id', choices=[row[0] for row in binding.CELL_ORDER])
    mode = parser.add_mutually_exclusive_group(required=True)
    for name in ('supervise', 'execute', 'summarize', 'validate-only'):
        mode.add_argument('--' + name, action='store_true')
    args = parser.parse_args()
    os.umask(0o077)
    # Read the contract as inert bytes before importing any historical implementation.
    import hashlib
    contract_path = stage / 'launch_contract.py'
    binding.require(hashlib.sha256(contract_path.read_bytes()).hexdigest() == binding.CONTRACT_SHA,
                    'unchanged historical contract')
    c = local_module(contract_path)
    c.require(socket.gethostname() == c.HOST and os.getuid() == c.UID, 'authorized native host/UID')
    c.stage_name(stage)
    c.require(args.plan == stage / 'plan.json', 'one owned current plan location')
    c.require((args.cell_id is None) == (args.summarize or args.validate_only), 'closed cell mode')
    plan = c.decode(c.read(args.plan, args.plan_sha256)[0])
    old = Path(plan['worker_refresh']['parent_plan']['path']).parent
    guard = None
    locks = []
    try:
        if not args.execute:
            for root in (old, stage):
                c.stage_name(root)
                fd = os.open(root / 'native.lock', os.O_CREAT | os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
                locks.append(fd)
                item = os.fstat(fd)
                c.require(stat.S_ISREG(item.st_mode) and item.st_uid == c.UID and item.st_nlink == 1
                          and stat.S_IMODE(item.st_mode) == 0o600, 'owned historical/current launch lock')
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        original, loaded, counter, evidence, cell, options, identities = binding.validate(
            c, stage, plan, args.cell_id or 'counter-A')
        guard = c.module(stage / 'historical/run_stage.py', binding.GUARD_SHA)
        _, _, legacy, _, profile, runner, supervisor, _ = loaded
        if args.validate_only:
            guard.resources(stage, supervisor, guard.stage_identity(stage))
            print(c.encoded({'inputs_validated': True, 'native_executed': False}).decode(), end='')
            return 0
        selected = None if args.summarize else c.selected_cell(plan, args.cell_id)
        if args.execute:
            return guard.execute(stage, plan, selected, loaded, counter, evidence, cell, options, identities)
        replay = c.module(stage / 'measurement/native_campaign_replay.py',
                          plan['sources']['measurement/native_campaign_replay.py'])
        root_binding = {'path': str(args.plan), 'sha256': args.plan_sha256}
        before = c.read(args.plan, args.plan_sha256)[0]

        def stable_inputs():
            c.require(c.read(args.plan, args.plan_sha256)[0] == before, 'current plan changed')
            c.bound(plan['worker_refresh']['parent_plan'])
            c.verify_files(old, original['files'])
            c.verify_files(stage, plan['files'])

        if args.summarize:
            rows = guard.retained_cells(plan, plan['cells'], replay, root_binding, runner, legacy, evidence)
            report = cell.evaluate_campaign(plan['comparison'], rows[2:], rows[:2])
            report['cpu_cost_by_cell'] = {row['cell_id']: row['completion']['cpu_cost'] for row in rows}
            report['worker_refresh'] = plan['worker_refresh']
            stable_inputs()
            guard.save_new(stage / 'campaign-report.json', report)
            return 0
        prior = guard.retained_cells(plan, guard.preceding_cells(plan, args.cell_id), replay,
                                     root_binding, runner, legacy, evidence)
        mechanism = replay.validate_counter_pair(prior[:2]) if len(prior) >= 2 else None
        output = Path(selected['output'])
        guard.fresh_output(stage, output)
        guard.save_new(output / 'prior-cells.json', [{'cell_id': row['cell_id'],
            'completion_sha256': row['outer']['completion_sha256']} for row in prior])
        if mechanism is not None:
            guard.save_new(output / 'mechanism-pair.json', mechanism)
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
        argv = ['/usr/bin/python3', '-I', '-B', str(stage / 'run_stage.py'), '--execute',
                '--plan', str(args.plan), '--plan-sha256', args.plan_sha256, '--cell-id', args.cell_id]
        result = cell.lifecycle.supervise_group(supervisor, argv, output,
            lambda initial: admission.check('preflight' if initial else 'monitor', initial=initial),
            lambda: stopped[0] if stopped else None, duration=1500, grace=20, kill_wait=5, interval=3)
        try:
            result['postflight'] = admission.check('postflight')
            stable_inputs()
            if result['status'] == 0:
                c.require(result['cleanup_ok'] and result['child_reaped'] and not result['term_sent']
                          and not result['kill_sent'] and not result['errors'], 'clean unsignaled outer completion')
                complete = c.decode(c.read(output / 'completion.json', maximum=1024**2)[0])
                c.require(complete.get('accepted') is True, 'complete accepted cell receipt required')
                result['completion_sha256'] = c.read(output / 'completion.json')[1]
        except BaseException as error:
            result['errors'].append('postflight: ' + type(error).__name__ + ': ' + str(error))
            result.update(status=125, cleanup_ok=False)
        runner.save(output / 'launch-supervisor.json', result)
        with (output / 'launch.status').open('x') as stream:
            stream.write(str(result['status']) + '\n')
        return result['status']
    finally:
        for fd in reversed(locks):
            os.close(fd)


if __name__ == '__main__':
    raise SystemExit(main())
