#!/usr/bin/env python3
"""Copy original captured operands into a bounded, flat read-only input bundle."""
import hashlib
import io
import json
from pathlib import Path
import tarfile

P = Path(__file__).resolve().parent.parent / 'model-matched-input-mlp-v1'
contract = json.loads((P / 'inputs.json').read_bytes())
local = json.loads((P / 'local-input-locations.json').read_bytes())
assert set(local) == set(contract['files']) == set(contract['locations'])
assert len(local) == 31 and len(set(contract['locations'].values())) == 31
output = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/matched-mlp-inputs-v1.tar.gz')
total = 0
with output.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
    for role, source in sorted(local.items()):
        expected = contract['files'][role]
        path = Path(source)
        before = path.stat()
        raw = path.read_bytes()
        assert path.stat() == before
        assert len(raw) == expected['bytes']
        assert hashlib.sha256(raw).hexdigest() == expected['sha256']
        name = contract['locations'][role]
        assert Path(name).name == name and name not in ('', '.', '..')
        item = tarfile.TarInfo(name)
        item.mode = 0o400
        item.size = len(raw)
        archive.addfile(item, io.BytesIO(raw))
        total += len(raw)
assert total < 8 << 20
print(json.dumps(dict(files=len(local), expanded_bytes=total, archive=str(output),
                      bytes=output.stat().st_size,
                      sha256=hashlib.sha256(output.read_bytes()).hexdigest())))
