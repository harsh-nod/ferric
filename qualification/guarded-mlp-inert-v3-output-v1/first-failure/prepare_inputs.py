"""Pin both qualified parents and fresh candidate inputs without compilation."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
assert ROOT.name == 'guarded-mlp-inert-v3-output-cpu-v228-v1'

def pin(p):
    before = p.stat()
    assert p.resolve(strict=True) == p and not p.is_symlink() and p.is_file()
    digest = hashlib.sha256()
    with p.open('rb') as f:
        for block in iter(lambda: f.read(1 << 20), b''):
            digest.update(block)
    after = p.stat()
    assert (before.st_dev,before.st_ino,before.st_size,before.st_mtime_ns,before.st_ctime_ns) == (after.st_dev,after.st_ino,after.st_size,after.st_mtime_ns,after.st_ctime_ns)
    return dict(path=str(p), bytes=after.st_size, sha256=digest.hexdigest())

parents = {}
for label, name, sha in (
    ('backend', 'guarded-mlp-finite-predicate-inline-cpu-v228-v3', '41d0425beff3ebef1c98910bc904402452c83cddbb8c72229cb9c05e42787a1a'),
    ('frontend', 'guarded-mlp-semantic-capture-driver-cpu-v228-v1', 'd8d5be4c85fbcac8b963657d1b5054c8fc8e1fdd987b4a9ba1954bc1cff1895f'),
    ('transport', 'guarded-mlp-inert-v3-invocation-cpu-v228-v1', 'd8a386e5f83e565c2dbb106daf74e7e7f6cc5aef8206403c18438dd2fdb83e37'),
    ('selector-recovery', 'guarded-mlp-inert-v3-selector-recovery-cpu-v228-v1', '26ce668bf3c759840c23582781131312f718209ab34405fad0ff4e1835f0cde5'),
    ('consumer', 'guarded-mlp-inert-v3-consumer-cpu-v228-v1', '92e85faaf739ea2f9d45818223a4a8c55d1a8736d38ed0a3dee102dd1c3ca3c2'),
):
    p = ROOT.parent / name / 'evidence/complete.json'
    parents[label] = pin(p)
    assert parents[label]['sha256'] == sha
    receipt = json.loads(p.read_bytes())
    assert receipt['passed'] and receipt['source_unchanged'] and not receipt['postcheck_errors']
    if label == 'backend':
        tools = receipt['tool_pins']
p = ROOT.parent / 'guarded-mlp-inert-v3-selector-cpu-v228-v1/evidence/failed.json'
parents['selector-source-failed'] = pin(p)
assert parents['selector-source-failed']['sha256'] == '927c4ccd1e5d82f1f1820dd0a8c804bc7e20d6be3f887cd40483f79f346d57f5'
paths = [ROOT / name for name in ('run_cpu.py','supervisor.py','prepare_inputs.py')]
paths += sorted(p for p in (ROOT / 'fe2o3').rglob('*') if p.is_file())
files = {str(p.relative_to(ROOT)): {k:v for k,v in pin(p).items() if k in ('bytes','sha256')} for p in paths}
data = (json.dumps(dict(schema='ferric-inert-semantic-output-input-v1', files=files, parents=parents, tool_pins=tools), indent=2, sort_keys=True)+'\n').encode()
with (ROOT / 'input-manifest.json').open('xb') as f:
    f.write(data)
print(json.dumps(dict(files=len(files), sha256=hashlib.sha256(data).hexdigest())))
