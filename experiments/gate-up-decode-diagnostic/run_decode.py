"""Two independently supervised cold diagnostic requests; never a latency campaign."""
import argparse
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import resource
import re
import signal
import socket
import stat
import time

BASE = Path('/dev/shm/ferric-native-gate-up-a004')
PARENT_SHA = '67bf416039c4062d76e3044137fa77dc2a5d5b1eeed9978c601272479cf5fe3f'
CONTRACT_SHA = '97f83d65302fd0d8fd276c57dc1454e4daf743259f733b0907c16b44eb542815'
ELF_SHA = '634da1c53d46753ffed133be814938a6ebe5075deed72d597fe5ad142de0ed56'
ARCHIVE_SHA = 'aacc2bcb90fc279c354d2a9924f71768f1ed141b60c38c504c14fd21d3fd680d'
ROLES = ('prepare','format','metadata','test','clippy','release')


def require(ok, message):
    if not ok:
        raise ValueError(message)


def module(path, pin):
    raw = path.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == pin, 'pinned helper')
    value = importlib.util.module_from_spec(importlib.util.spec_from_file_location(path.stem,path))
    exec(compile(raw,str(path),'exec'),value.__dict__)
    return value


def derive(parent, stage, arm, diagnostic_setup):
    row = copy.deepcopy(parent['cells'][0 if arm == 'A' else 1])
    require(row['cell_id'] == 'counter-' + arm and row['spec']['mode'] == 'counters', 'original counter arm')
    spec = row['spec']
    spec['controller'] = {'path':str(stage / 'cpu/binaries/controller-decode'),'sha256':ELF_SHA}
    spec['worker']['path'] = str(stage / 'worker-candidate')
    spec['argv'][0] = spec['controller']['path']
    for argv in (spec['argv'],row['common_args']):
        require(argv.count('--worker') == 1, 'one worker selector')
        argv[argv.index('--worker')+1] = spec['worker']['path']
    spec['setup_expected']['decode_diagnostic'] = diagnostic_setup
    row['output'] = str(stage / 'cells' / row['cell_id'])
    return row


def cpu(c, stage, plan):
    root = stage / 'cpu'
    retained = c.bound(plan['cpu_retention'])
    require(retained['accepted'] is True, 'accepted CPU retention')
    for name,item in retained['members'].items():
        c.read(root / c.relative(name),item['sha256'],32*1024**2,empty=True)
    previous = None
    for role in ROLES:
        receipt = c.decode(c.read(root / 'qualification' / role / 'receipt.json')[0])
        outer = c.decode(c.read(root / 'outer' / role / 'result.json')[0])
        require(receipt['accepted'] is True and receipt['returncode'] == 0
            and receipt['archive_sha256'] == ARCHIVE_SHA and receipt['original_source_and_elfs_unchanged'] is True
            and receipt['stage_after_bytes']-receipt['stage_before_bytes'] <= receipt['planning_increment_bytes'], 'CPU role closure')
        require(previous is None or receipt['source_before'] == previous['source_after'], 'CPU source chain')
        require(outer['status'] == outer['returncode'] == 0 and outer['cleanup_ok'] is True
            and outer['child_reaped'] is True and outer['errors'] == []
            and outer['term_sent'] is False and outer['kill_sent'] is False
            and outer['profile'] == 'FerricCpuFourCore42GiBEmitterV1', 'CPU outer closure')
        previous = receipt
    require(previous['binary']['sha256'] == ELF_SHA, 'independent new binary qualification')
    for name,digest in previous['source_after'].items():
        c.read(root / 'qualification/source' / c.relative(name),digest,1024**2,empty=True)
    c.read(root / 'binaries/controller-decode',ELF_SHA,32*1024**2)
    # The original adapter library is provenance, not qualification for this new ELF.
    original = c.decode(c.read(root / 'original/model-source-a009.json',
        'c53c170a8b853e14a7156fd308b97434422152031fed3b8c0d0ca74bc15b1119')[0])
    require(len(original['files']) == 1441, 'original library source identity')


def validate(c, stage, plan, arm):
    require(plan['schema'] == 'FerricNativeDecodeCapturePlanR1' and plan['stage'] == str(stage)
        and plan['arms'] == ['A','B'] and plan['requests_per_arm'] == 1
        and plan['performance_qualified'] is False and plan['latency_sample_admitted'] is False,
        'two-request diagnostic-only plan')
    require(plan['parent'] == {'path':str(BASE / 'plan.json'),'sha256':PARENT_SHA}, 'immutable parent provenance')
    c.verify_files(stage,plan['files'])
    require(all(plan['files'][name]['sha256'] == pin for name,pin in plan['sources'].items()), 'source bindings')
    harness = c.bound(plan['harness_cpu'])
    require(harness['schema'] == 'FerricDecodeCaptureCpuR1' and harness['accepted'] is True
        and harness['sources'] == {name:plan['sources'][name] for name in
            ('decode_capture.py','run_decode.py','test_decode_capture.py','prepare_capture.py','qualify_capture.py')}, 'fresh exact harness CPU qualification')
    inner = c.bound(harness['inner'])
    require(inner['schema'] == 'FerricDecodeCaptureCpuTestR1' and inner['accepted'] is True and inner['returncode'] == 0
        and inner['tests'] == 15 and inner['skipped'] == 0
        and inner['source_before'] == inner['source_after'] == harness['sources'], 'stable tested harness bytes')
    outer = c.bound(harness['outer'])
    require(outer['status'] == outer['returncode'] == 0 and outer['cleanup_ok'] is True and outer['child_reaped'] is True
        and outer['errors'] == [] and outer['term_sent'] is False and outer['kill_sent'] is False
        and outer['profile'] == 'FerricCpuFourCore42GiBEmitterV1'
        and outer['argv'] == harness['argv'], 'fresh clean harness CPU outer')
    test_path = Path(inner['argv'][-1])
    require(inner['argv'] == ['/usr/bin/python3','-I','-B',str(test_path)]
        and test_path.name == 'test_decode_capture.py'
        and test_path.parent.parent == Path('/tmp/ferric-v16-emitter-b95a642-r1/inputs')
        and outer['argv'] == ['/bin/bash','/tmp/ferric-v16-emitter-b95a642-r1/owner/cpu-env-42g-emitter.sh',
            '/usr/bin/python3','-I','-B',str(test_path.with_name('qualify_capture.py'))], 'exact qualifier and test invocation')
    for name,binding in harness['logs'].items():
        raw,pin = c.read(binding['path'],binding['sha256'],1024**2,empty=True)
        require(inner['log_sha256'][name] == pin,'inner/outer test log binding')
        if name == 'stderr':
            require(re.search(rb'\nRan 15 tests in [0-9.]+s\n\nOK\n$',raw), 'actual test count and no skips')
    parent = c.bound(plan['parent'])
    loaded,counter,evidence,cell,_,_ = c.validate_plan(parent,BASE,'counter-' + arm)
    cpu(c,stage,plan)
    check = c.module(stage / 'decode_capture.py',plan['sources']['decode_capture.py'])
    selected = derive(parent,stage,arm,check.SETUP)
    cell.shape(selected['spec'])
    _,_,legacy,_,_,runner,_,_ = loaded
    options = c.parse_common(selected['common_args'],runner)
    identities = legacy.inputs({**parent['inputs'],'controller':selected['spec']['controller'],
        'images':parent['images']},options,runner)
    return parent,loaded,counter,evidence,cell,check,selected,options,identities


def capture(spec, output, runner, legacy, evidence, cell, check, admission):
    output.mkdir(mode=0o700)
    result = {'schema':'FerricDecodeCaptureCellR1','accepted':False,'arm':spec['arm'],
        'spec_sha256':cell.ledger.digest(spec),'performance_qualified':False,'latency_sample_admitted':False,
        'started_ns':time.monotonic_ns()}
    controller = None
    try:
        admission('preflight',None)
        deadline = time.monotonic()+spec['timeouts']['cell_seconds']
        with cell.lifecycle.deferred_stop():
            controller = cell.lifecycle.controller_class(runner)(spec['argv'],output,deadline)
        controller.deadline = min(deadline,time.monotonic()+spec['timeouts']['setup_seconds'])
        def before(setup):
            check.exact(setup.get('decode_diagnostic'),check.SETUP)
            admission('active',setup)
        def before_request(_index):
            controller.deadline = min(deadline,time.monotonic()+spec['timeouts']['request_seconds'])
        def after(setup):
            admission('active',setup)
            controller.deadline = min(deadline,time.monotonic()+15)
        observed = cell.consume(controller,spec,runner=runner,legacy=legacy,evidence=evidence,
            deadline=deadline,before_requests=before,before_request=before_request,after_requests=after)
        cleanup = controller.close()
        controller = None
        result['cleanup'] = cleanup
        check.clean_exit(cleanup)
        replay = cell.consume(cell.Replay(output,runner,1),spec,runner=runner,legacy=legacy,evidence=evidence,deadline=0)
        require(replay == observed,'independent raw token replay')
        stderr = cell.retained_bytes(output / 'stderr.raw',check.MAX_STDERR)
        breakdown = check.replay(stderr,spec,observed['setup'],cell.counter_replay)
        admission('postflight',None)
        result.update(observed,diagnostic=breakdown,accepted=True,raw_replay_passed=True,
            raw_sha256={name:hashlib.sha256(cell.retained_bytes(output / name,runner.MAX_STREAM)).hexdigest()
                for name in ('stdin.raw','stdout.raw','stderr.raw')})
    except BaseException as error:
        result['error'] = type(error).__name__ + ': ' + str(error)
        raise
    finally:
        with cell.lifecycle.deferred_stop(deliver=False):
            if controller is not None:
                result['failed_cleanup'] = controller.close()
            result['finished_ns'] = time.monotonic_ns()
            runner.save(output / 'result.json',result)
    return result


def execute(c, stage, plan, context, guard):
    require(os.environ.get('FERRIC_V14_SUPERVISOR_PID') == str(os.getppid()),'live owning supervisor')
    parent,loaded,_,evidence,cell,check,selected,options,identities = context
    _,_,legacy,support,profile,runner,supervisor,_ = loaded
    for number in (signal.SIGTERM,signal.SIGINT,signal.SIGHUP):
        signal.signal(number,cell.lifecycle.interrupted)
    output = Path(selected['output'])
    admission = guard.Admission(stage,plan,supervisor,profile,output)
    guard.save_new(output / 'wrapper.json',admission.activity.identity(Path('/proc'),os.getpid()))
    model = runner.model_identities(options)
    guard.save_new(output / 'model-before.json',model)
    placement = support.inherited_placement()
    guard.save_new(output / 'launch-placement.json',placement)
    admitted = evidence.admitted_executables(identities)
    endpoints,placements = [],[]
    def checked(phase, setup):
        observation = admission.check(phase,setup)
        require(observation['accepted'] is True,'GPU admission accepted')
        if phase == 'active':
            worker = evidence.process(setup['worker_pids'][0])['process']
            controller = evidence.process(worker['parent_pid'])['process']
            observed = {role:{key:row[key] for key in evidence.STABLE_PROCESS}
                for role,row in (('worker',worker),('controller',controller))}
            require(controller['parent_pid'] == os.getpid()
                and all(row['process_group'] == row['session'] == os.getpid() for row in observed.values()), 'owned process group')
            require(all(row['nice'] == placement['nice'] == 0
                and support.affinity_list(row['cpus_allowed_list']) == placement['affinity']
                for row in observed.values()),'unchanged default placement')
            if placements:
                legacy.stable_placement(placements[0],observed)
            placements.append(observed)
            endpoints.append(evidence.endpoint(observed,admitted,after_request=bool(endpoints)))
        return observation
    process_record = {'placements':placements,'endpoints':endpoints,'accepted':False}
    try:
        with cell.lifecycle.handling_stop():
            result = capture(selected['spec'],output / 'cell-results',runner,legacy,evidence,cell,check,checked)
        require(len(endpoints) == 2 and evidence.admitted_executables(identities) == admitted,'stable paired process endpoints')
        process_record.update(accepted=True,cpu_cost=evidence.cpu_cost(endpoints[0],endpoints[1],os.sysconf('SC_CLK_TCK')))
    finally:
        guard.save_new(output / 'process-endpoints.json',process_record)
    require(runner.model_identities(options) == model,'stable model')
    c.verify_files(BASE,parent['files'])
    c.verify_files(stage,plan['files'])
    guard.save_new(output / 'completion.json',{'accepted':result['accepted'],'cell_id':selected['cell_id'],
        'cell_result_sha256':c.read(output / 'cell-results/result.json')[1],
        'model_stable':True,'input_files_stable':True,'latency_sample_admitted':False})
    return 0


def preceding(c, stage, arm, context=None):
    if arm == 'A':
        return
    output = stage / 'cells/counter-A'
    outer = c.decode(c.read(output / 'launch-supervisor.json')[0])
    require(outer['status'] == 0 and outer['cleanup_ok'] is True and outer['child_reaped'] is True
        and outer['term_sent'] is False and outer['kill_sent'] is False and outer['errors'] == []
        and outer['postflight']['accepted'] is True, 'A failure prevents B')
    completion = c.decode(c.read(output / 'completion.json',outer['completion_sha256'])[0])
    result = c.decode(c.read(output / 'cell-results/result.json',completion['cell_result_sha256'])[0])
    require(completion['accepted'] is True and result['accepted'] is True
        and result['raw_replay_passed'] is True, 'accepted complete A required before B')
    require(context is not None,'independent prior spec binding')
    parent,_,_,_,cell,check,*_ = context
    expected = derive(parent,stage,'A',check.SETUP)['spec']
    require(result['arm'] == 'A' and result['spec_sha256'] == cell.ledger.digest(expected)
        and completion['cell_id'] == 'counter-A','exact prior A spec')
    require(set(result['raw_sha256']) == {'stdin.raw','stdout.raw','stderr.raw'},'closed prior raw roster')
    for name,pin in result['raw_sha256'].items():
        c.read(output / 'cell-results' / name,pin,8*1024**2,empty=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan-sha256',required=True)
    parser.add_argument('--arm',choices=('A','B'),required=True)
    parser.add_argument('--execute',action='store_true')
    args = parser.parse_args()
    os.umask(0o077)
    c = module(BASE / 'launch_contract.py',CONTRACT_SHA)
    require(socket.gethostname() == c.HOST and os.getuid() == c.UID,'authorized MI350 host')
    stage = c.stage_name(Path(__file__).resolve().parent)
    plan = c.decode(c.read(stage / 'plan.json',args.plan_sha256)[0])
    parent = c.bound(plan['parent'])
    require(plan['parent']['sha256'] == PARENT_SHA,'pinned parent plan')
    c.verify_files(BASE,parent['files'])
    guard = c.module(BASE / 'run_stage.py','36d8eef5890c13c3a0ed6d1a99d327796ca9277bb3265cab0f5b21d3fe66fbed')
    locks = []
    try:
        if not args.execute:
            for root in (BASE,stage):
                fd = os.open(root / 'native.lock',os.O_CREAT|os.O_RDWR|os.O_CLOEXEC|os.O_NOFOLLOW,0o600)
                locks.append(fd)
                st = os.fstat(fd)
                require(stat.S_ISREG(st.st_mode) and st.st_uid == c.UID and st.st_nlink == 1
                    and stat.S_IMODE(st.st_mode) == 0o600,'owned launch lock')
                fcntl.flock(fd,fcntl.LOCK_EX|fcntl.LOCK_NB)
        context = validate(c,stage,plan,args.arm)
        preceding(c,stage,args.arm,context)
        if args.execute:
            return execute(c,stage,plan,context,guard)
        _,loaded,_,_,cell,_,selected,_,_ = context
        _,_,_,_,profile,runner,supervisor,_ = loaded
        output = Path(selected['output'])
        guard.fresh_output(stage,output)
        admission = guard.Admission(stage,plan,supervisor,profile,output)
        os.environ.update(PATH='/usr/bin:/bin',LC_ALL='C',PYTHONDONTWRITEBYTECODE='1',PYTHONNOUSERSITE='1',
            FERRIC_V14_SUPERVISOR_PID=str(os.getpid()))
        for key in ('LD_PRELOAD','LD_LIBRARY_PATH','PYTHONPATH'):
            os.environ.pop(key,None)
        resource.setrlimit(resource.RLIMIT_CORE,(0,0))
        resource.setrlimit(resource.RLIMIT_AS,(128*1024**3,128*1024**3))
        stopped = []
        for number in (signal.SIGTERM,signal.SIGINT,signal.SIGHUP):
            signal.signal(number,lambda value,_frame: stopped.append(value) if not stopped else None)
        argv = ['/usr/bin/python3','-I','-B',str(stage / 'run_decode.py'),'--execute',
            '--plan-sha256',args.plan_sha256,'--arm',args.arm]
        result = cell.lifecycle.supervise_group(supervisor,argv,output,
            lambda initial: admission.check('preflight' if initial else 'monitor',initial=initial),
            lambda:stopped[0] if stopped else None,duration=1500,grace=20,kill_wait=5,interval=3)
        try:
            result['postflight'] = admission.check('postflight')
            c.read(stage / 'plan.json',args.plan_sha256)
            c.verify_files(BASE,parent['files'])
            c.verify_files(stage,plan['files'])
            if result['status'] == 0:
                require(result['cleanup_ok'] and result['child_reaped'] and not result['term_sent']
                    and not result['kill_sent'] and not result['errors'],'clean unsignaled outer')
                complete = c.decode(c.read(output / 'completion.json')[0])
                require(complete['accepted'] is True,'accepted capture completion')
                result['completion_sha256'] = c.read(output / 'completion.json')[1]
        except BaseException as error:
            result['errors'].append('postflight: '+type(error).__name__+': '+str(error))
            result.update(status=125,cleanup_ok=False)
        runner.save(output / 'launch-supervisor.json',result)
        with (output / 'launch.status').open('x') as stream:
            stream.write(str(result['status'])+'\n')
        return result['status']
    finally:
        for fd in reversed(locks):
            os.close(fd)


if __name__ == '__main__':
    raise SystemExit(main())
