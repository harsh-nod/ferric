"""Data-only publication audit: hashes and archive contents, never project code."""
import hashlib
import io
import json
import os
from pathlib import Path
import stat
import tarfile

ROOT = Path(__file__).resolve().parent
CPU = ROOT.parent / 'guarded-mlp-semantic-call-census-cpu-v228-v2'

def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())

def compact(row):
    return {k: row[k] for k in ('bytes', 'sha256')}

def read(path):
    first = path.lstat()
    assert path.resolve(strict=True) == path and stat.S_ISREG(first.st_mode)
    assert first.st_size <= 32 << 20
    raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    assert stamp(first) == stamp(path.lstat()) and len(raw) == first.st_size
    return raw

assert os.uname().nodename == 'smci350-rck-g03-b19-03'
assert os.getuid() == os.geteuid() == 9661
assert os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
assert not (ROOT / 'publication-audit.json').exists()
receipt_raw = read(CPU / 'evidence/complete.json')
assert pin(receipt_raw)['sha256'] == '60898b32e9d93cdae3ec5359177055213898dc27c0e14e5bf68b5d727376a5a7'
receipt = json.loads(receipt_raw)
assert receipt['passed'] and not receipt['postcheck_errors'] and receipt['source_unchanged']
for row in receipt['final_sources'].values():
    assert pin(read(Path(row['path']))) == compact(row)
assert len(receipt['final_sources']) == 5844
manifest = json.loads(read(ROOT / 'manifest.json'))
archive_raw = read(ROOT / 'originals.tar.gz')
assert pin(archive_raw) == manifest['archive']
with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as archive:
    rows = archive.getmembers()
    assert len(rows) == manifest['members'] == 148
    assert len(set(row.name for row in rows)) == len(rows)
    assert sum(row.size for row in rows) == manifest['expanded_bytes'] == 94142186
    assert all(row.isfile() and not row.name.startswith('/')
               and '..' not in Path(row.name).parts and row.size <= 16 << 20 for row in rows)
    ledger = json.loads(archive.extractfile('originals.json').read())['originals']
    assert len(ledger) == manifest['originals'] == 147
    assert {row.name for row in rows} == set(ledger) | {'originals.json'}
    for name, row in ledger.items():
        raw = archive.extractfile(name).read()
        assert pin(raw) == compact(row) == pin(read(Path(row['path'])))
    assert archive.extractfile('source-after/fe2o3/crates/fe2o3-mir-model/src/bin/fe2o3-semantic-capture-inspect.rs').read() == read(ROOT / 'source/crates/fe2o3-mir-model/src/bin/fe2o3-semantic-capture-inspect.rs')
assert read(ROOT / 'cpu/complete.json') == receipt_raw
assert pin(read(ROOT / 'reader.patch')) == manifest['reader_patch']
assert pin(read(ROOT / 'source-join/result.json')) == manifest['source_join']
for name in ('fixed', 'early', 'call-fixed', 'call-early'):
    filename = 'evidence/inspect-' + name + '.stdout'
    assert read(CPU / filename) == read(CPU.parent / 'guarded-mlp-semantic-call-inspect-cpu-v228-v1' / filename)
for version in ('v16', 'v17'):
    for arm in ('fixed', 'early'):
        assert read(ROOT / ('census/' + version + '-' + arm + '.json')) == read(CPU / ('evidence/census-' + version + '-' + arm + '.stdout'))
result = dict(schema='ferric-semantic-census-data-audit-v1', passed=True,
              project_code_executed=False, gpu_execution=False, production_authority=False,
              source_inputs_verified=5844, archive_members_verified=148, originals_verified=147,
              archive=pin(archive_raw), receipt=pin(receipt_raw),
              reader_source=pin(read(ROOT / 'source/crates/fe2o3-mir-model/src/bin/fe2o3-semantic-capture-inspect.rs')),
              boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
              pid=os.getpid(), affinity=sorted(os.sched_getaffinity(0)),
              audit_source=pin(read(Path(__file__).resolve())))
raw = (json.dumps(result, indent=2, sort_keys=True) + '\n').encode()
with (ROOT / 'publication-audit.json').open('xb') as stream:
    stream.write(raw)
assert read(ROOT / 'publication-audit.json') == raw
print(json.dumps(dict(result=pin(raw), passed=True, source_inputs_verified=5844,
                      archive_members_verified=148), sort_keys=True))
