"""Finish only unsubmitted cohorts against the retained selector test ELF."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import sys
import time
import types

ROOT = Path(__file__).resolve().parent
E = ROOT.parent
FIRST = E / 'guarded-mlp-inert-v3-selector-cpu-v228-v1'
Q = E / 'guarded-mlp-finite-predicate-inline-cpu-v228-v3'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence','target','tmp'))
FAILED_SHA = '927c4ccd1e5d82f1f1820dd0a8c804bc7e20d6be3f887cd40483f79f346d57f5'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
GATES = (
 'production_pipeline::conditional_formal_tests::conditional_formal_pipeline_keeps_v2_and_protected_handoffs_closed',
 'production_pipeline::tests::extraction_custody_cannot_enter_protected_publication',
)

def require(value,message):
    if not value:
        raise RuntimeError(message)

def main():
    start=time.monotonic();deadline=start+500
    require(ROOT.name=='guarded-mlp-inert-v3-selector-recovery-cpu-v228-v1'
            and ROOT.resolve(strict=True)==ROOT and __debug__ and sys.dont_write_bytecode,'exact root')
    require(not any(os.path.lexists(p) for p in (OUT,TARGET,TMP)), 'fresh outputs')
    require(os.getuid()==os.geteuid()==9661 and os.uname().nodename=='smci350-rck-g03-b19-03','host/uid')
    require(shutil.disk_usage(ROOT).free>=40<<30,'disk floor')
    os.umask(0o077);os.sched_setaffinity(0,{8,9})
    require(os.getpriority(os.PRIO_PROCESS,0)==10,'nice10')
    cg=Path('/proc/self/cgroup').read_text().strip().split(':',2)[2]
    scope=Path('/sys/fs/cgroup')/cg.lstrip('/')
    require(scope.name=='ferric-inert-v3-selector-recovery-cpu-v228-v1.service'
        and (scope/'memory.max').read_text().strip()==str(12<<30)
        and (scope/'memory.swap.max').read_text().strip()=='0','bounded service')
    for kind,cap in ((resource.RLIMIT_CORE,0),(resource.RLIMIT_AS,12<<30),(resource.RLIMIT_FSIZE,1<<30)):
        _,hard=resource.getrlimit(kind);limit=cap if hard==resource.RLIM_INFINITY else min(cap,hard)
        resource.setrlimit(kind,(limit,limit))
    require(ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)==0,'subreaper')
    body=(ROOT/'supervisor.py').read_bytes()
    require(hashlib.sha256(body).hexdigest()==SUPERVISOR_SHA,'supervisor pin')
    h=types.ModuleType('owned_supervisor');h.__file__=str(ROOT/'supervisor.py')
    exec(compile(body,h.__file__,'exec'),h.__dict__)
    h.ROOT,h.SOURCE,h.OUT,h.TARGET,h.TMP=ROOT,FIRST/'fe2o3',OUT,TARGET,TMP
    for sig in (signal.SIGTERM,signal.SIGINT,signal.SIGHUP,signal.SIGALRM):
        signal.signal(sig,h.interrupted)
    OUT.mkdir(mode=0o700);TMP.mkdir(mode=0o700)
    inputs={};phases=[];tests={};errors=[];before=after=None;failure=None
    first=qualified=None
    try:
        failed_pin=h.pin(FIRST/'evidence/failed.json')
        require(failed_pin['sha256']==FAILED_SHA,'exact original failure')
        first=json.loads((FIRST/'evidence/failed.json').read_bytes())
        require(first['failure']=="RuntimeError('active cohort')" and first['source_unchanged']
            and not first['postcheck_errors'] and not first['passed'],'closed original harness failure')
        require(first['boot_id']==Path('/proc/sys/kernel/random/boot_id').read_text().strip(),'same boot')
        require(all(not Path('/proc/'+str(pid)).exists() for pid in [first['controller_pid']]+[x['pid'] for x in first['phases']]),'original PIDs absent')
        require(len(first['phases'])==11 and first['phases'][-1]['label']=='selector-tests'
            and all(x['natural_exit'] and x['reaped'] and x['process_group_absent'] and x['exit_code']==0 for x in first['phases']),'original eleven clean leaves')
        inputs.update({'source:'+k:v for k,v in first['sources_before'].items()})
        inputs.update({'tool:'+k:v for k,v in first['tool_pins'].items()})
        inputs.update({'parent:'+k:v for k,v in first['parents'].items()})
        inputs.update({'raw:'+k:v for k,v in first['raw'].items()})
        inputs.update({'artifact:'+k:v['pin'] for k,v in first['artifacts'].items()})
        inputs['original-failed']=failed_pin
        qualified=json.loads((Q/'evidence/complete.json').read_bytes())
        require(h.pin(Q/'evidence/complete.json')==first['parents']['backend'],'qualified compiler parent')
        inputs['prior-compiler-test']=qualified['artifacts']['test']['pin']
        for name in ('run_cpu.py','supervisor.py'):
            inputs['recovery:'+name]=h.pin(ROOT/name)
        def sources():
            return {key:h.pin(Path(row['path'])) for key,row in sorted(inputs.items())}
        h.sources=sources
        before=sources();require(before==inputs,'all original/current input pins')
        h.save('sources-before.json',before)
        for label,cohort in first['tests'].items():
            require(label in ('invocation','engineering','capture','selector'),'only completed original cohorts')
            stream={'invocation':'invocation-tests','engineering':'engineering-tests','capture':'capture-tests','selector':'selector-tests'}[label]
            total=len(first['inventories']['invocation' if label=='invocation' else 'driver'])
            require(h.outcomes(FIRST/'evidence'/(stream+'.stdout'),cohort['names'],total)==cohort,'original exact named outcomes')
        require({k:v['passed'] for k,v in first['tests'].items()}==dict(invocation=103,engineering=25,capture=8,selector=10),'original cohort census')
        inventory=h.inventory(FIRST/'evidence/driver-list.stdout')
        require(inventory==first['inventories']['driver'],'actual retained driver inventory')
        ignored=h.inventory(FIRST/'evidence/driver-ignored.stdout')
        require(ignored==first['ignored']['driver'],'actual retained ignored inventory')
        lifecycle=[name for name in inventory if name.startswith('binding_wrapper::lifecycle_tests::')]
        require(len(lifecycle)==6 and not set(lifecycle)&set(ignored),'six actual lifecycle tests')
        env=json.loads((FIRST/'evidence/selector-tests.command.json').read_bytes())['env']
        require(all(env[k]=='' for k in ('ROCR_VISIBLE_DEVICES','HIP_VISIBLE_DEVICES','CUDA_VISIBLE_DEVICES')),'GPU visibility closed')
        env=dict(env,TMPDIR=str(TMP),CARGO_TARGET_DIR=str(TARGET))
        def leaf(label,argv,cwd):
            h.run(label,argv,env,phases,deadline,before,120,cwd)
        binary=first['artifacts']['driver']['pin']['path']
        leaf('binding-wrapper-lifecycle-tests',[binary,'binding_wrapper::lifecycle_tests::','--test-threads=1'],FIRST/'fe2o3/crates/cargo-fe2o3')
        tests['lifecycle']=h.outcomes(OUT/'binding-wrapper-lifecycle-tests.stdout',lifecycle,len(inventory))
        for index,name in enumerate(GATES):
            label='prior-gate-'+str(index)
            leaf(label,[qualified['artifacts']['test']['pin']['path'],name,'--exact','--test-threads=1'],Q/'fe2o3/crates/rustc-codegen-fe2o3')
            tests[label]=h.outcomes(OUT/(label+'.stdout'),[name],len(qualified['inventory']))
    except BaseException as error:
        failure=repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL,0)
    try:
        after=h.sources();h.save('sources-after.json',after)
        require(before==after,'source/tool/artifact/raw input drift')
    except BaseException as error:
        errors.append(repr(error))
    failure=failure or ('postcheck failed' if errors else None)
    raw={p.name:h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()}
    result=dict(schema='ferric-inert-semantic-selector-retained-binary-recovery-v1',passed=failure is None,
        failure=failure,postcheck_errors=errors,source_unchanged=before==after,
        first_failed_receipt=h.pin(FIRST/'evidence/failed.json'),first_tests=first['tests'] if first else {},
        tests=tests,phases=phases,raw=raw,input_sources=before,final_sources=after,
        elapsed_seconds=time.monotonic()-start,boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
        controller_pid=os.getpid(),controller_starttime_ticks=int(Path('/proc/self/stat').read_text().rsplit(') ',1)[1].split()[19]),
        cgroup=cg,affinity=sorted(os.sched_getaffinity(0)),rust_build_repeated=False,
        completed_cohort_repeated=False,selector_preflight_integrated=True,
        extraction_only_consumer_qualified=False,engineering_transport_integrated=False,
        worker_compiled=False,gpu_execution=False,production_authority=False)
    h.save('complete.json' if failure is None else 'failed.json',result)
    print(json.dumps(dict(passed=failure is None,failure=failure,errors=errors,phases=len(phases),tests={k:v['passed'] for k,v in tests.items()}),sort_keys=True))
    return 0 if failure is None else 1

if __name__=='__main__':
    raise SystemExit(main())
