#!/usr/bin/env python3
"""Install the authenticated flat operand archive without touching old evidence."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-model-matched-input-mlp-v228-v1'
archive = E / 'matched-mlp-inputs-v1.tar.gz'
assert os.getuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
assert shutil.disk_usage(E).free >= 40 << 30
assert hashlib.sha256(archive.read_bytes()).hexdigest() == 'be9e611a05698b36b7aa227bd8036243c9465c294aae2ce9ad708edfa6d1752c'
ROOT.mkdir(mode=0o700)
(ROOT / 'inputs').mkdir(mode=0o700)
files = {}
with tarfile.open(archive, 'r:gz') as source:
    members = source.getmembers()
    assert len(members) == 31 and sum(m.size for m in members) == 2293477
    for member in members:
        assert member.isfile() and Path(member.name).name == member.name
        assert member.name not in files and 0 < member.size < 8 << 20
        raw = source.extractfile(member).read(member.size + 1)
        assert len(raw) == member.size
        path = ROOT / 'inputs' / member.name
        with path.open('xb') as target:
            target.write(raw)
        path.chmod(0o400)
        files[member.name] = dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
with (ROOT / 'input-staging.json').open('x') as target:
    json.dump(dict(archive=str(archive), files=files), target, indent=2, sort_keys=True)
print(json.dumps(dict(root=str(ROOT), files=len(files), gpu_executed=False)))
