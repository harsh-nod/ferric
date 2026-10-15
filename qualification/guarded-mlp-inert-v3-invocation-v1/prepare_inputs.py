"""Pin both qualified parents and fresh candidate inputs without compilation."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
assert ROOT.name == 'guarded-mlp-inert-v3-invocation-cpu-v228-v1'

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
):
    p = ROOT.parent / name / 'evidence/complete.json'
    parents[label] = pin(p)
    assert parents[label]['sha256'] == sha
    receipt = json.loads(p.read_bytes())
    assert receipt['passed'] and receipt['source_unchanged'] and not receipt['postcheck_errors']
    if label == 'backend':
        tools = receipt['tool_pins']
paths = [ROOT / name for name in ('run_cpu.py','supervisor.py','prepare_inputs.py')]
paths += sorted(p for p in (ROOT / 'fe2o3').rglob('*') if p.is_file())
files = {str(p.relative_to(ROOT)): {k:v for k,v in pin(p).items() if k in ('bytes','sha256')} for p in paths}
data = (json.dumps(dict(schema='ferric-selected-invocation-transport-input-v1', files=files, parents=parents, tool_pins=tools), indent=2, sort_keys=True)+'\n').encode()
with (ROOT / 'input-manifest.json').open('xb') as f:
    f.write(data)
print(json.dumps(dict(files=len(files), sha256=hashlib.sha256(data).hexdigest())))
