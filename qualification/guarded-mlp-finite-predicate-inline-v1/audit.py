"""Data-only source and publication audit, with no project/GPU execution."""
import hashlib
import io
import json
import os
from pathlib import Path
import stat
import tarfile

ROOT = Path(__file__).resolve().parent
CPU = ROOT.parent / 'guarded-mlp-finite-predicate-inline-cpu-v228-v3'


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def read(path):
    first = path.lstat()
    assert path.resolve(strict=True) == path and stat.S_ISREG(first.st_mode)
    assert first.st_size <= 32 << 20
    raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    assert stamp(first) == stamp(path.lstat()) and len(raw) == first.st_size
    return raw


assert os.getuid() == os.geteuid() == 9661
assert os.uname().nodename == 'smci350-rck-g03-b19-03'
assert os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
assert not (ROOT / 'publication-audit.json').exists()
manifest = json.loads(read(ROOT / 'manifest.json'))
cpu_raw = read(CPU / 'evidence/complete.json')
assert pin(cpu_raw) == manifest['cpu']['receipt']
cpu = json.loads(cpu_raw)
assert cpu['passed'] and cpu['source_unchanged'] and cpu['postcheck_errors'] == []
assert len(cpu['final_sources']) == 5812
for row in cpu['final_sources'].values():
    assert pin(read(Path(row['path']))) == compact(row)
for artifact in cpu['artifacts'].values():
    expected = artifact['pin']
    path = Path(expected['path'])
    first = path.lstat()
    assert path.resolve(strict=True) == path and stat.S_ISREG(first.st_mode)
    assert first.st_size == expected['bytes'] <= 512 << 20
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b''):
            h.update(chunk)
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    assert stamp(first) == stamp(path.lstat()) and h.hexdigest() == expected['sha256']
archive_raw = read(ROOT / 'originals.tar.gz')
assert pin(archive_raw) == manifest['archive']
with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as archive:
    rows = archive.getmembers()
    assert len(rows) == manifest['members'] <= 256
    assert len({row.name for row in rows}) == len(rows)
    assert sum(row.size for row in rows) == manifest['expanded_bytes'] <= 256 << 20
    assert all(row.isfile() and not row.name.startswith('/')
               and '..' not in Path(row.name).parts and row.size <= 16 << 20 for row in rows)
    ledger = json.loads(archive.extractfile('originals.json').read())['originals']
    assert len(ledger) == manifest['originals']
    assert {row.name for row in rows} == set(ledger) | {'originals.json'}
    for name, row in ledger.items():
        assert pin(archive.extractfile(name).read()) == compact(row) == pin(read(Path(row['path'])))
    for name in cpu['source_delta']:
        body = archive.extractfile('source-after/' + name).read()
        assert body == read(ROOT / 'source' / name.removeprefix('fe2o3/'))
    assert archive.extractfile('correction.patch').read() == read(ROOT / 'correction.patch')
assert read(ROOT / 'cpu/complete.json') == cpu_raw
pair_root = CPU.parent / 'guarded-mlp-early-stop-compile-v228-v18'
for arm in ('fixed', 'early'):
    for name in ('result.json', 'compile.stderr'):
        assert read(ROOT / 'pair' / arm / name) == read(pair_root / (arm + '-run/evidence') / name)
    result = json.loads(read(pair_root / (arm + '-run/evidence/result.json')))
    assert result['postcheck_errors'] == []
    assert not Path('/proc/' + str(result['execution_scope']['pid'])).exists()
    for phase in result['phases']:
        assert phase['natural_exit'] and phase['reaped'] and phase['process_group_absent']
        for key in ('pid', 'pgid'):
            assert not Path('/proc/' + str(phase[key])).exists()
result = dict(schema='ferric-finite-predicate-inline-data-audit-v1', passed=True,
              source_inputs_verified=5812, archive_members_verified=manifest['members'],
              originals_verified=manifest['originals'], archive=pin(archive_raw),
              cpu=pin(cpu_raw), project_code_executed=False, gpu_execution=False,
              production_authority=False, boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
              pid=os.getpid(), affinity=sorted(os.sched_getaffinity(0)),
              audit_source=pin(read(Path(__file__).resolve())))
raw = (json.dumps(result, indent=2, sort_keys=True) + '\n').encode()
with (ROOT / 'publication-audit.json').open('xb') as stream:
    stream.write(raw)
assert read(ROOT / 'publication-audit.json') == raw
print(json.dumps(dict(result=pin(raw), source_inputs_verified=5812,
                      archive_members_verified=manifest['members']), sort_keys=True))
