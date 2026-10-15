"""Offline data-only replay of the published original-byte joins."""
import hashlib
import json
from pathlib import Path
import tarfile

P = Path(__file__).resolve().parent
ledger = json.loads((P/'originals-ledger.json').read_bytes())
manifest = json.loads((P/'manifest.json').read_bytes())
audit = json.loads((P/'publication-audit.json').read_bytes())

def body_pin(body):
    return dict(bytes=len(body),sha256=hashlib.sha256(body).hexdigest())

def content(pin):
    return {k:pin[k] for k in ('bytes','sha256')}

assert body_pin((P/'originals.tar.gz').read_bytes()) == content(manifest['archive'])
assert body_pin((P/'publication-audit.json').read_bytes()) == content(manifest['audit'])
assert body_pin((P/'originals-ledger.json').read_bytes()) == content(manifest['ledger'])
expected_names = sorted([*ledger,'originals-ledger.json'])
with tarfile.open(P/'originals.tar.gz','r:gz') as tar:
    assert sorted(tar.getnames()) == expected_names
    expanded = 0
    for member in tar.getmembers():
        assert member.isfile() and not Path(member.name).is_absolute() and '..' not in Path(member.name).parts
        data = tar.extractfile(member).read()
        assert data == (P/member.name).read_bytes()
        if member.name in ledger:
            assert body_pin(data) == content(ledger[member.name])
        expanded += len(data)
assert expanded == audit['archive_expanded_bytes'] and len(ledger)==audit['verified_originals']
r = json.loads((P/'cpu/complete.json').read_bytes())
assert body_pin((P/'cpu/complete.json').read_bytes()) == content(manifest['receipt'])
assert r['passed'] and r['sources_before']==r['sources_after'] and not r['postcheck_errors']
assert len(r['sources_before']) == audit['verified_source_inputs']
for name,row in r['raw'].items():
    assert body_pin((P/'cpu'/name).read_bytes()) == content(row)
for name in ledger:
    if name.startswith('source/'):
        rel = 'fe2o3/'+str(Path(name).relative_to('source'))
        assert content(ledger[name]) == content(r['sources_before'][rel])
assert len(r['phases'])==16 and all(x['natural_exit'] and x['reaped'] and x['process_group_absent'] and x['exit_code']==0 for x in r['phases'])
assert {k:v['passed'] for k,v in r['tests'].items()} == {'invocation':103,'engineering':25,'capture':8,'selector':10,'binding-wrapper':6,'consumer':9,'output':8,'prior-gate-0':1,'prior-gate-1':1}
assert r['pinned_output_intake_implemented'] and r['same_length_content_recheck_implemented']
first=json.loads((P/'first-failure/failed.json').read_bytes())
assert body_pin((P/'first-failure/failed.json').read_bytes())==content(r['parents']['output-first-failed'])
assert not first['passed'] and first['source_unchanged'] and not first['postcheck_errors']
assert '7 passed; 1 failed' in (P/'first-failure/output-tests.stdout').read_text()
assert r['inert_content_consumer_implemented'] and not r['live_parent_completion_executed']
assert not r['actual_conditional_formal_archive_replayed'] and not r['extraction_only_consumer_qualified']
assert not r['production_transport_integrated'] and not r['engineering_process_schema_compatible'] and not r['worker_compiled'] and not r['gpu_execution'] and not r['production_authority']
print(json.dumps(dict(passed=True,originals=len(ledger),members=len(expected_names),expanded_bytes=expanded,source_inputs=len(r['sources_before']),test_count=sum(v['passed'] for v in r['tests'].values())),sort_keys=True))
