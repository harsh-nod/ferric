"""Create one private two-request diagnostic stage from pinned CPU evidence."""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import socket
import stat
import tarfile

BASE = Path('/dev/shm/ferric-native-gate-up-a004')
CPU_ARCHIVE = 'b95c39aebf61b9bdeb4cf5cc7c904a0599012f63f6270723268276b26313dfdc'
CPU_RECEIPT = '95524bc8882908e8b601bce7c34c5ebc97b70e8c5c2c5b143a7d1ee63dc3af46'
SOURCES = ('decode_capture.py','run_decode.py','test_decode_capture.py','prepare_capture.py','qualify_capture.py')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest-sha256',required=True)
    parser.add_argument('--stage',type=Path,required=True)
    args = parser.parse_args()
    incoming = Path(__file__).resolve().parent
    raw = (incoming / 'run_decode.py').read_bytes()
    manifest_raw = (incoming / 'incoming.json').read_bytes()
    if hashlib.sha256(manifest_raw).hexdigest() != args.manifest_sha256:
        raise ValueError('incoming manifest pin')
    manifest = json.loads(manifest_raw)
    for name,row in manifest['files'].items():
        path = incoming / name
        if str(Path(name)) != name or Path(name).is_absolute() or '..' in Path(name).parts:
            raise ValueError('canonical incoming path')
        if (path.resolve(strict=True) != path or not stat.S_ISREG(path.lstat().st_mode)
                or hashlib.sha256(path.read_bytes()).hexdigest() != row['sha256']):
            raise ValueError('incoming file mismatch')
    driver = importlib.util.module_from_spec(importlib.util.spec_from_file_location('capture_driver',incoming / 'run_decode.py'))
    if hashlib.sha256(raw).hexdigest() != manifest['files']['run_decode.py']['sha256']:
        raise ValueError('exact executed driver buffer')
    exec(compile(raw,str(incoming / 'run_decode.py'),'exec'),driver.__dict__)
    c = driver.module(BASE / 'launch_contract.py',driver.CONTRACT_SHA)
    c.require(socket.gethostname() == c.HOST and os.getuid() == c.UID,'authorized staging host')
    c.require(set(manifest['files']) == set(SOURCES) | {'cpu-evidence.tar.gz','cpu-receipt.json',
        'harness/receipt.json','harness/result.json','harness/stdout','harness/stderr'},'closed incoming roster')
    c.require(manifest['files']['cpu-evidence.tar.gz']['sha256'] == CPU_ARCHIVE
        and manifest['files']['cpu-receipt.json']['sha256'] == CPU_RECEIPT,'new CPU artifact pins')
    c.stage_name(args.stage)
    os.umask(0o077)
    lock = os.open(BASE / 'native.lock',os.O_RDWR|os.O_CLOEXEC|os.O_NOFOLLOW)
    try:
        fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
        parent = c.decode(c.read(BASE / 'plan.json',driver.PARENT_SHA)[0])
        c.verify_files(BASE,parent['files'])
        guard = c.module(BASE / 'run_stage.py','36d8eef5890c13c3a0ed6d1a99d327796ca9277bb3265cab0f5b21d3fe66fbed')
        guard.tmpfs_parent()
        loaded,*_ = c.validate_plan(parent,BASE,'counter-A')
        loaded[6].resource_snapshot(BASE)
        test = c.decode(c.read(incoming / 'harness/receipt.json')[0])
        outer = c.decode(c.read(incoming / 'harness/result.json')[0])
        c.require(test['accepted'] is True and test['returncode'] == 0
            and test['source_before'] == test['source_after'] == {n:manifest['files'][n]['sha256'] for n in SOURCES}
            and outer['status'] == outer['returncode'] == 0 and outer['cleanup_ok'] and outer['child_reaped']
            and not outer['term_sent'] and not outer['kill_sent'] and not outer['errors'],'fresh exact harness CPU evidence')
        args.stage.mkdir(mode=0o700)
        roster = {}
        def write(name,raw,mode=0o600):
            path = args.stage / c.relative(name)
            path.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
            with path.open('xb') as stream:
                stream.write(raw)
            path.chmod(mode)
            roster[name] = {'sha256':hashlib.sha256(raw).hexdigest(),'bytes':len(raw),'mode':mode}
        retained = c.decode(c.read(incoming / 'cpu-receipt.json',CPU_RECEIPT)[0])
        seen = set()
        with tarfile.open(incoming / 'cpu-evidence.tar.gz') as archive:
            for member in archive:
                c.require(member.isfile() and member.name not in seen and member.name in retained['members'], 'closed CPU archive')
                item = retained['members'][member.name]
                raw = archive.extractfile(member).read(32*1024**2)
                c.require(len(raw) == member.size == item['bytes'] and hashlib.sha256(raw).hexdigest() == item['sha256'], 'CPU archive member')
                write('cpu/'+member.name,raw,0o700 if member.name == 'binaries/controller-decode' else 0o600)
                seen.add(member.name)
        c.require(seen == set(retained['members']),'complete CPU archive')
        for name in SOURCES:
            write(name,c.read(incoming / name,manifest['files'][name]['sha256'])[0])
        write('cpu-retention.json',c.read(incoming / 'cpu-receipt.json',CPU_RECEIPT)[0])
        for name in ('receipt.json','result.json','stdout','stderr'):
            write('harness-cpu/'+name,c.read(incoming / 'harness' / name,empty=True)[0])
        write('worker-candidate',c.read(BASE / 'worker-candidate',c.WORKER_SHA)[0],0o700)
        scanner = 'measurement/gpu_activity.py'
        write(scanner,c.read(BASE / scanner,parent['sources'][scanner])[0])
        def bind(name):
            return {'path':str(args.stage / name),'sha256':roster[name]['sha256']}
        harness = {'schema':'FerricDecodeCaptureCpuR1','accepted':True,'sources':test['source_after'],
            'outer':bind('harness-cpu/result.json'),'inner':bind('harness-cpu/receipt.json'),
            'argv':outer['argv'],'logs':{n:bind('harness-cpu/'+n) for n in ('stdout','stderr')}}
        write('harness-cpu.json',c.encoded(harness))
        plan = {'schema':'FerricNativeDecodeCapturePlanR1','stage':str(args.stage),'arms':['A','B'],
            'requests_per_arm':1,'performance_qualified':False,'latency_sample_admitted':False,
            'parent':{'path':str(BASE / 'plan.json'),'sha256':driver.PARENT_SHA},
            'cpu_retention':bind('cpu-retention.json'),'harness_cpu':bind('harness-cpu.json'),
            'files':dict(roster),'sources':{n:roster[n]['sha256'] for n in (*SOURCES,scanner)}}
        guard.save_new(args.stage / 'plan.json',plan)
        driver.validate(c,args.stage,plan,'A')
        driver.validate(c,args.stage,plan,'B')
        loaded[6].resource_snapshot(args.stage)
        c.verify_files(BASE,parent['files'])
        guard.save_new(args.stage / 'prepared.json',{'schema':'FerricDecodeCapturePreparedR1','accepted':True,
            'plan_sha256':c.read(args.stage / 'plan.json')[1],'incoming_sha256':args.manifest_sha256,
            'native_executed':False,'performance_qualified':False})
        print(c.read(args.stage / 'plan.json')[1],flush=True)
    finally:
        os.close(lock)


if __name__ == '__main__':
    main()
