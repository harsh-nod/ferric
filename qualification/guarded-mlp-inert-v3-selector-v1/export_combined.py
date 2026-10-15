"""Export preserved first-run failure plus non-replaying retained-ELF recovery."""
import hashlib
import json
from pathlib import Path
import shutil
import tarfile

P=Path(__file__).resolve().parent
E=P.parent
N=E/'guarded-mlp-inert-v3-selector-cpu-v228-v1'
R=E/'guarded-mlp-inert-v3-selector-recovery-cpu-v228-v1'
B=E/'guarded-mlp-inert-v3-invocation-cpu-v228-v1'

def pin(p):
    assert p.resolve(strict=True)==p and p.is_file() and not p.is_symlink()
    a=p.stat();h=hashlib.sha256()
    with p.open('rb') as f:
        for block in iter(lambda:f.read(1<<20),b''):
            h.update(block)
    b=p.stat()
    assert (a.st_dev,a.st_ino,a.st_size,a.st_mtime_ns,a.st_ctime_ns)==(b.st_dev,b.st_ino,b.st_size,b.st_mtime_ns,b.st_ctime_ns)
    return dict(path=str(p),bytes=b.st_size,sha256=h.hexdigest())

def save(name,value):
    with (P/name).open('x') as f:
        f.write(json.dumps(value,indent=2,sort_keys=True)+'\n')

failed=pin(N/'evidence/failed.json');complete=pin(R/'evidence/complete.json')
assert failed['sha256']=='927c4ccd1e5d82f1f1820dd0a8c804bc7e20d6be3f887cd40483f79f346d57f5'
assert complete['sha256']=='26ce668bf3c759840c23582781131312f718209ab34405fad0ff4e1835f0cde5'
f=json.loads((N/'evidence/failed.json').read_bytes());r=json.loads((R/'evidence/complete.json').read_bytes())
assert not f['passed'] and f['failure']=="RuntimeError('active cohort')" and not f['postcheck_errors'] and f['source_unchanged']
assert r['passed'] and not r['postcheck_errors'] and r['source_unchanged'] and r['first_failed_receipt']==failed
assert r['input_sources']==r['final_sources'] and f['sources_before']==f['sources_after']
assert not r['rust_build_repeated'] and not r['completed_cohort_repeated']
assert len(f['phases'])==11 and len(r['phases'])==3
for receipt in (f,r):
    assert all(x['natural_exit'] and x['reaped'] and x['process_group_absent'] and x['exit_code']==0 and not x['forced_cleanup'] for x in receipt['phases'])
    assert all(not Path('/proc/'+str(pid)).exists() for pid in [receipt['controller_pid']]+[x['pid'] for x in receipt['phases']])
for row in r['input_sources'].values():
    assert pin(Path(row['path']))==row
for row in r['raw'].values():
    assert pin(Path(row['path']))==row
assert sum(v['passed'] for v in f['tests'].values())==146
assert sum(v['passed'] for v in r['tests'].values())==8
ledger={}

def copy(src,name):
    original=pin(src);out=P/name
    out.parent.mkdir(mode=0o700,parents=True,exist_ok=True)
    with src.open('rb') as a,out.open('xb') as b:
        shutil.copyfileobj(a,b,1<<20)
    assert pin(src)==original
    assert {k:pin(out)[k] for k in ('bytes','sha256')}=={k:original[k] for k in ('bytes','sha256')}
    ledger[name]=original

for root,label in ((N,'first'),(R,'recovery')):
    for name in ('run_cpu.py','supervisor.py','prepare_inputs.py','input-manifest.json'):
        if (root/name).is_file():
            copy(root/name,label+'/'+name)
    for p in sorted((root/'evidence').iterdir()):
        if p.is_file():
            copy(p,'cpu/'+label+'/'+p.name)
context={
    'crates/rustc-codegen-fe2o3/src/lib.rs',
    'crates/rustc-codegen-fe2o3/src/production_pipeline.rs',
    'crates/rustc-codegen-fe2o3/src/production_rustc_driver_v1.rs',
    'crates/cargo-fe2o3/src/main.rs',
    'crates/cargo-fe2o3/src/inert_rustc_invocation_capture.rs',
    'crates/fe2o3-rustc-invocation/src/selected_transport_v3.rs',
}
context|={str(Path(rel).relative_to('fe2o3')) for rel in f['source_delta']}
for rel in sorted(context):
    copy(N/'fe2o3'/rel,'source/'+rel)
for rel in f['source_delta']:
    p=B/rel
    if p.is_file():
        copy(p,'baseline/'+str(Path(rel).relative_to('fe2o3')))
save('originals-ledger.json',ledger)
members=sorted([*ledger,'originals-ledger.json'])
with (P/'originals.tar.gz').open('xb') as out:
    with tarfile.open(fileobj=out,mode='w:gz') as tar:
        for name in members:
            tar.add(P/name,arcname=name,recursive=False)
expanded=0
with tarfile.open(P/'originals.tar.gz','r:gz') as tar:
    assert sorted(tar.getnames())==members
    for member in tar.getmembers():
        assert member.isfile() and not Path(member.name).is_absolute() and '..' not in Path(member.name).parts
        data=tar.extractfile(member).read();expanded+=len(data)
        assert data==(P/member.name).read_bytes()
save('publication-audit.json',dict(schema='ferric-inert-semantic-selector-combined-data-audit-v1',passed=True,
    failed_receipt=failed,recovery_receipt=complete,verified_inputs=len(r['input_sources']),
    verified_originals=len(ledger),archive_members=len(members),archive_expanded_bytes=expanded,
    first_source_inputs=len(f['sources_before']),archive=pin(P/'originals.tar.gz'),
    completed_cohort_repeated=False,rust_build_repeated=False,total_tests=154,
    extraction_only_consumer_qualified=False,worker_compiled=False,gpu_execution=False,production_authority=False))
save('manifest.json',dict(schema='ferric-inert-semantic-selector-combined-publication-v1',
    failed_receipt=failed,recovery_receipt=complete,archive=pin(P/'originals.tar.gz'),
    audit=pin(P/'publication-audit.json'),exporter=pin(P/'export_combined.py'),
    ledger=pin(P/'originals-ledger.json'),source_delta=f['source_delta'],
    no_compiled_or_toolchain_tree_embedded=True))
print(json.dumps(dict(archive=pin(P/'originals.tar.gz'),audit=pin(P/'publication-audit.json'),inputs=len(r['input_sources']),originals=len(ledger),members=len(members),expanded_bytes=expanded),sort_keys=True))
