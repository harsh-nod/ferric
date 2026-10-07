#!/usr/bin/env python3
"""Stage reviewed sources and bind a fresh host/container plan; never executes it."""
import hashlib
import io
import json
import os
from pathlib import Path
import re
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-model-matched-input-mlp-v228-v1'
ENV = E / 'guarded-mlp-matched-framework-env-v228-v2'


def pin(path):
    raw = path.read_bytes()
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


assert len(sys.argv) == 2 and re.fullmatch('[0-9a-f]{64}', sys.argv[1])
assert os.getuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
archive = E / 'matched-mlp-source-v1.tar.gz'
raw = archive.read_bytes()
assert len(raw) <= 2 << 20 and hashlib.sha256(raw).hexdigest() == sys.argv[1]
with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as stream:
    members = stream.getmembers()
    assert len(members) <= 32 and sum(m.size for m in members) < 4 << 20
    assert len({m.name for m in members}) == len(members)
    assert all(m.isfile() and Path(m.name).name == m.name and 0 < m.size < 1 << 20
               for m in members)
    bodies = {m.name: stream.extractfile(m).read() for m in members}
manifest = json.loads(bodies['source-manifest.json'])
assert set(bodies) == set(manifest['files']) | {'source-manifest.json'}
for name, expected in manifest['files'].items():
    assert dict(bytes=len(bodies[name]), sha256=hashlib.sha256(bodies[name]).hexdigest()) == expected
(ROOT / 'source').mkdir(mode=0o700)
for name, body in bodies.items():
    with (ROOT / 'source' / name).open('xb') as output:
        output.write(body)
device = Path('/sys/class/drm/renderD128/device')
unique_id = int((device / 'unique_id').read_text().strip(), 16)
assert unique_id == 16366993098680759275
compact = lambda row: {key: row[key] for key in ('bytes', 'sha256')}
plan = dict(schema='ferric-guarded-mlp-matched-input-launch-v1',
    image='sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba',
    topology=dict(host=os.uname().nodename,
                  boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                  unique_id=unique_id, pci_path=str(device.resolve(strict=True))),
    source_manifest=compact(pin(ROOT / 'source/source-manifest.json')),
    overlay_root=str(ENV / 'packages'), overlay_manifest=pin(ENV / 'overlay-manifest.json'),
    environment=compact(pin(ROOT / 'inputs/environment.json')))
with (ROOT / 'launch-plan.json').open('x') as output:
    json.dump(plan, output, indent=2, sort_keys=True)
    output.write('\n')
print(json.dumps(dict(plan=pin(ROOT / 'launch-plan.json'),
                      source_manifest=plan['source_manifest'],
                      controller=pin(ROOT / 'source/launch.py'),
                      source_files=len(bodies), gpu_executed=False)))
