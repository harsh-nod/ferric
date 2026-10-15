"""Data-only audit of pinned replay inputs and exported evidence."""
import hashlib
import io
import json
import os
from pathlib import Path
import stat
import tarfile

ROOT = Path(__file__).resolve().parent
RUN = ROOT.parent / 'guarded-mlp-fixed-formal-replay-cpu-v228-v1'


def pin(path):
    first = path.lstat()
    assert path.resolve(strict=True) == path and stat.S_ISREG(first.st_mode)
    assert first.st_size <= 512 << 20
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b''):
            h.update(chunk)
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    assert stamp(first) == stamp(path.lstat())
    return dict(bytes=first.st_size, sha256=h.hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


assert os.getuid() == os.geteuid() == 9661
assert os.uname().nodename == 'smci350-rck-g03-b19-03'
assert os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
assert not (ROOT / 'publication-audit.json').exists()
manifest = json.loads((ROOT / 'manifest.json').read_bytes())
receipt_path = RUN / 'evidence/complete.json'
assert pin(receipt_path) == manifest['replay_receipt']
assert manifest['replay_receipt']['sha256'] == '12e961aaf9a5bf4d2a1a4750517568ec7284a974b82a852674c678559ca2f507'
replay = json.loads(receipt_path.read_bytes())
assert replay['passed'] and replay['source_unchanged'] and not replay['postcheck_errors']
assert replay['inputs_before'] == replay['inputs_after'] and len(replay['inputs_after']) == 5846
for path, row in replay['inputs_after'].items():
    assert pin(Path(path)) == compact(row)
assert pin(RUN / 'input-manifest.json') == compact(replay['input_manifest'])
archive_path = ROOT / 'originals.tar.gz'
assert pin(archive_path) == manifest['archive']
with tarfile.open(archive_path, mode='r:gz') as archive:
    rows = archive.getmembers()
    assert len(rows) == manifest['members'] == 48
    assert len({row.name for row in rows}) == len(rows)
    assert sum(row.size for row in rows) == manifest['expanded_bytes'] == 37564092
    assert all(row.isfile() and not row.name.startswith('/') and '..' not in Path(row.name).parts
               and row.size <= 16 << 20 for row in rows)
    ledger = json.loads(archive.extractfile('originals.json').read())['originals']
    assert len(ledger) == manifest['originals'] == 47
    assert {row.name for row in rows} == set(ledger) | {'originals.json'}
    for name, row in ledger.items():
        raw = archive.extractfile(name).read()
        assert dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest()) == compact(row)
        assert pin(Path(row['path'])) == compact(row)
    for name in ledger:
        if name.startswith('source/'):
            assert (ROOT / name).read_bytes() == archive.extractfile(name).read()
assert (ROOT / 'replay/complete.json').read_bytes() == receipt_path.read_bytes()
for name in ('run.py', 'prepare_inputs.py'):
    assert (ROOT / name).read_bytes() == (RUN / name).read_bytes()
for name in ('fixed-formal-replay.stdout', 'fixed-formal-replay.stderr'):
    assert (ROOT / 'replay' / name).read_bytes() == (RUN / 'evidence' / name).read_bytes()
for name in ('semantic-mir-v1.bin', 'semantic-source-map-v1.json'):
    actual = ROOT.parent / 'guarded-mlp-early-stop-compile-v228-v18/fixed-run/fe2o3-engineering-diagnostics-v1' / name
    assert (ROOT / 'capture' / name).read_bytes() == actual.read_bytes()
assert not Path('/proc/' + str(replay['execution_scope']['pid'])).exists()
for phase in replay['phases']:
    assert phase['natural_exit'] and phase['reaped'] and phase['process_group_absent']
    for key in ('pid', 'pgid'):
        assert not Path('/proc/' + str(phase[key])).exists()
result = dict(schema='ferric-fixed-formal-replay-data-audit-v1', passed=True,
              replay_receipt=manifest['replay_receipt'], archive=manifest['archive'],
              inputs_verified=5846, archive_members_verified=48, originals_verified=47,
              project_code_executed=False, gpu_execution=False, production_authority=False,
              audit_source=pin(Path(__file__).resolve()),
              boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
              pid=os.getpid(), affinity=sorted(os.sched_getaffinity(0)))
raw = (json.dumps(result, indent=2, sort_keys=True) + '\n').encode()
with (ROOT / 'publication-audit.json').open('xb') as stream:
    stream.write(raw)
assert (ROOT / 'publication-audit.json').read_bytes() == raw
print(json.dumps(dict(result=pin(ROOT / 'publication-audit.json'), passed=True,
                      inputs_verified=5846, archive_members_verified=48), sort_keys=True))
