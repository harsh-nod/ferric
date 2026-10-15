"""Publish actual qualified originals without embedding compiled/toolchain trees."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import re
import tarfile

P = Path(__file__).resolve().parent
N = P.parent / 'guarded-mlp-inert-v3-consumer-cpu-v228-v1'
assert len(sys.argv)==2 and re.fullmatch('[0-9a-f]{64}',sys.argv[1])
EXPECTED = sys.argv[1]

def pin(p):
    assert p.resolve(strict=True) == p and p.is_file() and not p.is_symlink()
    a = p.stat()
    h = hashlib.sha256()
    with p.open('rb') as f:
        for block in iter(lambda:f.read(1<<20), b''):
            h.update(block)
    b = p.stat()
    assert (a.st_dev,a.st_ino,a.st_size,a.st_mtime_ns,a.st_ctime_ns) == (b.st_dev,b.st_ino,b.st_size,b.st_mtime_ns,b.st_ctime_ns)
    return dict(path=str(p), bytes=b.st_size, sha256=h.hexdigest())

def save(p, value):
    with p.open('x') as f:
        f.write(json.dumps(value, indent=2, sort_keys=True)+'\n')

receipt = N / 'evidence/complete.json'
assert pin(receipt)['sha256'] == EXPECTED
r = json.loads(receipt.read_bytes())
assert r['passed'] and r['source_unchanged'] and not r['postcheck_errors']
assert all(x['exit_code']==0 and x['natural_exit'] and x['reaped'] and x['process_group_absent']
           and not x['forced_cleanup'] and not x['timed_out'] for x in r['phases'])
assert all(not Path('/proc/'+str(pid)).exists() for pid in [r['controller_pid']]+[x['pid'] for x in r['phases']])
inputs = dict(r['sources_before'])
inputs.update({'tool:'+k:v for k,v in r['tool_pins'].items()})
inputs.update({'parent:'+k:v for k,v in r['parents'].items()})
inputs.update({'artifact:'+k:v['pin'] for k,v in r['artifacts'].items()})
for row in inputs.values():
    assert pin(Path(row['path'])) == row
assert r['sources_before'] == r['sources_after']
ledger = {}

def copy(src, dest):
    original = pin(src)
    out = P / dest
    out.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    with src.open('rb') as f, out.open('xb') as g:
        shutil.copyfileobj(f,g,1<<20)
    assert pin(src) == original
    actual = pin(out)
    assert (actual['bytes'],actual['sha256']) == (original['bytes'],original['sha256'])
    ledger[dest] = original

for name in ('run_cpu.py','supervisor.py','prepare_inputs.py','input-manifest.json'):
    copy(N/name, name)
for p in sorted((N/'evidence').iterdir()):
    if p.is_file():
        copy(p,'cpu/'+p.name)
for rel in sorted(set(r['source_delta']) | {
    'fe2o3/crates/rustc-codegen-fe2o3/src/bin/fe2o3-rustc-extract.rs',
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_rustc_driver_v1.rs',
    'fe2o3/crates/rustc-codegen-fe2o3/src/production_pipeline.rs',
    'fe2o3/crates/fe2o3-rustc-invocation/src/model_v2.rs',
    'fe2o3/crates/fe2o3-rustc-invocation/src/model_v3.rs',
    'fe2o3/crates/fe2o3-build-authority/src/compiler_closure.rs',
    'fe2o3/crates/cargo-fe2o3/src/binding_wrapper.rs',
    'fe2o3/crates/cargo-fe2o3/src/compiler_toolchain.rs',
    'fe2o3/crates/fe2o3-compiler-ffi/src/inert_semantic_module_handoff_v3.rs',
    'fe2o3/crates/fe2o3-compiler-lineage/src/capsule.rs',
    'fe2o3/crates/fe2o3-hsaco-finalize/src/first_build_worker_v3.rs',
}):
    copy(N/rel,'source/'+str(Path(rel).relative_to('fe2o3')))
for rel in r['source_delta']:
    old = P.parent/'guarded-mlp-inert-v3-selector-cpu-v228-v1'/rel
    if old.is_file():
        copy(old,'baseline/'+str(Path(rel).relative_to('fe2o3')))
save(P/'originals-ledger.json',ledger)
members = sorted([*ledger, 'originals-ledger.json'])
with (P/'originals.tar.gz').open('xb') as f:
    with tarfile.open(fileobj=f,mode='w:gz') as tar:
        for name in members:
            tar.add(P/name,arcname=name,recursive=False)
with tarfile.open(P/'originals.tar.gz','r:gz') as tar:
    assert sorted(tar.getnames()) == members
    expanded = 0
    for member in tar.getmembers():
        assert member.isfile() and not Path(member.name).is_absolute() and '..' not in Path(member.name).parts
        body = tar.extractfile(member).read()
        assert hashlib.sha256(body).hexdigest() == pin(P/member.name)['sha256']
        expanded += len(body)
save(P/'publication-audit.json',dict(schema='ferric-inert-semantic-consumer-data-audit-v1', passed=True,
    receipt=pin(receipt), verified_inputs=len(inputs), verified_source_inputs=len(r['sources_before']),
    verified_originals=len(ledger), archive_members=len(members), archive_expanded_bytes=expanded,
    archive=pin(P/'originals.tar.gz'), compiler_rebuilt=False, gpu_execution=False,
    engineering_transport_integrated=False, production_authority=False))
save(P/'manifest.json',dict(schema='ferric-inert-semantic-consumer-publication-v1',
    receipt=pin(receipt), archive=pin(P/'originals.tar.gz'), audit=pin(P/'publication-audit.json'),
    exporter=pin(P/'export.py'), ledger=pin(P/'originals-ledger.json'),
    source_delta=r['source_delta'], no_compiled_or_toolchain_tree_embedded=True))
print(json.dumps(dict(archive=pin(P/'originals.tar.gz'),audit=pin(P/'publication-audit.json'),originals=len(ledger),inputs=len(inputs)),sort_keys=True))
