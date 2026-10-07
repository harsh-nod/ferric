"""Fixed 32 MiB create-only portable delta; borrow core/worker, no native work."""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile

D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
I = D / 'inputs/splitk-model-export-a001'
Q = D / 'inputs/splitk-model-harness-a003/qualify.py'
Q_SHA = 'c3e5b5350dc1abaa7a7e76d292878356ad40735facabbfae72a631686a3f68c6'
MANIFEST = I / 'export-inputs-a001.json'
MANIFEST_SHA = '25cb311e3ccc51ed62f11479653ec6f90d9088ef1b215c79e484940164afcb02'
OUT = D / 'splitk-model-portable-a001'


def main():
    assert Q.resolve(strict=True) == Q and hashlib.sha256(Q.read_bytes()).hexdigest() == Q_SHA
    spec = importlib.util.spec_from_file_location('fixed_export_qualification', Q)
    q = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(q)
    q.environment()
    before = q.allocation(32 * 1024**2)
    files = q.payload()
    source_before = q.check_sources(files)
    definition = importlib.util.spec_from_file_location('fixed_export_binder', q.ROOT / 'bind_build.py')
    binder = importlib.util.module_from_spec(definition)
    definition.loader.exec_module(binder)
    manifest = json.loads(q.read(MANIFEST, MANIFEST_SHA)[0])
    q.require(manifest['schema'] == 'FerricSplitKPortableExportV1' and len(manifest['files']) == 113,
              'exact export input roster')
    payload, checked, total = {}, {}, 0
    for path, item in manifest['files'].items():
        raw, digest = binder.c.read(path, item['sha256'], 24 * 1024**2, empty=True)
        name = item['relative']
        q.require(str(Path(name)) == name and not Path(name).is_absolute() and '..' not in Path(name).parts,
                  'canonical export member')
        q.require(name not in payload or payload[name] == raw, 'conflicting shared evidence destination')
        payload[name] = raw
        checked[path] = digest
    for path, digest in manifest['borrowed'].items():
        binder.c.read(path, digest)
    total = sum(map(len, payload.values()))
    q.require(total <= 24 * 1024**2 and len(payload) <= 113, 'bounded export payload')
    OUT.mkdir(mode=0o700)
    archive = OUT / 'splitk-model-delta-a001.tar.gz'
    with tarfile.open(archive, 'x:gz', compresslevel=1) as target:
        for name, raw in sorted(payload.items()):
            info = tarfile.TarInfo(name)
            info.size, info.mode, info.mtime = len(raw), 0o600, 0
            target.addfile(info, io.BytesIO(raw))
    q.require(archive.stat().st_size <= 24 * 1024**2, 'bounded compressed delta')
    after_inputs = {path: binder.c.read(path, digest, 24 * 1024**2, empty=True)[1]
                    for path, digest in checked.items()}
    q.require(after_inputs == checked and source_before == q.check_sources(files), 'export inputs changed')
    q.read(MANIFEST, MANIFEST_SHA)
    after = q.allocation()
    q.require(after['stage_allocated_bytes'] - before['stage_allocated_bytes'] <= 32 * 1024**2,
              'fixed export growth allowance exceeded')
    q.environment()
    receipt = {'schema': 'FerricSplitKPortableExportResultV1', 'accepted': True,
        'archive': {'path': str(archive), 'sha256': hashlib.sha256(archive.read_bytes()).hexdigest(),
                    'bytes': archive.stat().st_size},
        'manifest_sha256': MANIFEST_SHA, 'inputs': checked, 'borrowed': manifest['borrowed'],
        'files': {name: {'sha256': hashlib.sha256(raw).hexdigest(), 'bytes': len(raw)}
                  for name, raw in payload.items()},
        'source_before': source_before, 'source_after': q.check_sources(files),
        'expanded_bytes': total, 'allocation_before': before, 'allocation_after': after,
        'native_executed': False}
    with (OUT / 'receipt.json').open('x') as stream:
        json.dump(receipt, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(receipt, sort_keys=True), flush=True)


if __name__ == '__main__':
    main()
