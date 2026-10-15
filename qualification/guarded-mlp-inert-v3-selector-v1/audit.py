"""Data-only replay of original archive joins and failed-run recovery accounting."""
import hashlib
import json
from pathlib import Path
import tarfile

P=Path(__file__).resolve().parent
ledger=json.loads((P/'originals-ledger.json').read_bytes())
manifest=json.loads((P/'manifest.json').read_bytes())
audit=json.loads((P/'publication-audit.json').read_bytes())

def pin(body):
    return dict(bytes=len(body),sha256=hashlib.sha256(body).hexdigest())

def content(row):
    return {key:row[key] for key in ('bytes','sha256')}

assert pin((P/'originals.tar.gz').read_bytes())==content(manifest['archive'])
assert pin((P/'publication-audit.json').read_bytes())==content(manifest['audit'])
assert pin((P/'originals-ledger.json').read_bytes())==content(manifest['ledger'])
expected=sorted([*ledger,'originals-ledger.json']);expanded=0
with tarfile.open(P/'originals.tar.gz','r:gz') as tar:
    assert sorted(tar.getnames())==expected
    for m in tar.getmembers():
        assert m.isfile() and not Path(m.name).is_absolute() and '..' not in Path(m.name).parts
        data=tar.extractfile(m).read();expanded+=len(data)
        assert data==(P/m.name).read_bytes()
        if m.name in ledger:
            assert pin(data)==content(ledger[m.name])
assert expanded==audit['archive_expanded_bytes']
f=json.loads((P/'cpu/first/failed.json').read_bytes());r=json.loads((P/'cpu/recovery/complete.json').read_bytes())
assert pin((P/'cpu/first/failed.json').read_bytes())==content(manifest['failed_receipt'])
assert pin((P/'cpu/recovery/complete.json').read_bytes())==content(manifest['recovery_receipt'])
assert not f['passed'] and f['failure']=="RuntimeError('active cohort')" and f['source_unchanged'] and not f['postcheck_errors']
assert r['passed'] and r['source_unchanged'] and not r['postcheck_errors']
assert r['first_failed_receipt']==manifest['failed_receipt'] and r['first_tests']==f['tests']
assert not r['rust_build_repeated'] and not r['completed_cohort_repeated']
assert r['input_sources']==r['final_sources'] and len(r['input_sources'])==audit['verified_inputs']==5889
assert len(f['sources_before'])==audit['first_source_inputs']==5813
for receipt,label in ((f,'first'),(r,'recovery')):
    for name,row in receipt['raw'].items():
        assert pin((P/'cpu'/label/name).read_bytes())==content(row)
    assert all(x['natural_exit'] and x['reaped'] and x['process_group_absent'] and x['exit_code']==0 for x in receipt['phases'])
assert len(f['phases'])==11 and len(r['phases'])==3
assert {k:v['passed'] for k,v in f['tests'].items()}==dict(invocation=103,engineering=25,capture=8,selector=10)
assert {k:v['passed'] for k,v in r['tests'].items()}=={'lifecycle':6,'prior-gate-0':1,'prior-gate-1':1}
for name,row in ledger.items():
    if name.startswith('source/'):
        source='fe2o3/'+str(Path(name).relative_to('source'))
        assert content(row)==content(f['sources_before'][source])
assert not r['extraction_only_consumer_qualified'] and not r['engineering_transport_integrated'] and not r['worker_compiled'] and not r['gpu_execution'] and not r['production_authority']
print(json.dumps(dict(passed=True,originals=len(ledger),members=len(expected),expanded_bytes=expanded,first_test_count=146,recovery_test_count=8,completed_cohort_repeated=False),sort_keys=True))
