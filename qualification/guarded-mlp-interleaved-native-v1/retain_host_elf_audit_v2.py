"""Retain static disassembly evidence without executing host or GPU code."""
import hashlib
import io
import json
from pathlib import Path
import tarfile

SOURCE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/'
              'guarded-mlp-host-elf-audit-evidence-v228-v2.tar.gz')
ARCHIVE_SHA = '7c7d8ffb9fe3b1c1a2646e94321d384414923f1188e30e13626142913bc943df'
RECEIPT_SHA = '0457febcc2629402b4c373c69cd71cdd360f2c1c487a2bcd9c0fe5ceb1dd4e38'


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    assert __debug__ and SOURCE.is_file() and not SOURCE.is_symlink()
    assert SOURCE.stat().st_size == 514647
    data = SOURCE.read_bytes()
    assert pin(data) == dict(bytes=514647, sha256=ARCHIVE_SHA)
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
        members = archive.getmembers()
        assert len(members) == len({m.name for m in members}) == 45
        assert sum(m.size for m in members) == 4347527
        for member in members:
            path = Path(member.name)
            assert member.isfile() and 0 <= member.size < 4 << 20
            assert not path.is_absolute() and '..' not in path.parts and path.as_posix() == member.name
        bodies = {m.name: archive.extractfile(m).read() for m in members}
    manifest = json.loads(bodies['retention-manifest.json'])
    assert set(bodies) == set(manifest['files']) | {'retention-manifest.json'}
    for name, expected in manifest['files'].items():
        assert pin(bodies[name]) == {k: expected[k] for k in ('bytes', 'sha256')}
    assert manifest['receipt'] == manifest['files']['evidence/complete.json']
    assert pin(bodies['evidence/complete.json'])['sha256'] == RECEIPT_SHA
    result = json.loads(bodies['evidence/complete.json'])
    assert result['passed'] and result['errors'] == result['postcheck_errors'] == []
    assert result['project_execution'] is result['gpu_execution'] is False
    assert len(result['phases']) == 8 and len(result['executables']) == 2
    for phase in result['phases']:
        assert phase == json.loads(bodies['evidence/' + phase['label'] + '.result.json'])
        assert phase['natural_exit'] and phase['exit_code'] == 0 and phase['reaped']
        assert phase['process_group_absent'] and not phase['forced_cleanup'] and not phase['timed_out']
        assert phase['exception'] is None and phase['storage_failure'] is None
    for row in result['executables']:
        assert row['missing_or_inlined'] == []
        assert row['full_symbol_coverage'] is True and len(row['symbols']) == 3
    destination = Path(__file__).resolve().parent / 'host-elf-audit-v2'
    assert not destination.exists() and not destination.is_symlink()
    destination.mkdir(mode=0o700)
    for name, body in bodies.items():
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(body)
        assert target.read_bytes() == body
    assert SOURCE.read_bytes() == data
    print(json.dumps(dict(destination=str(destination), members=45, expanded_bytes=4347527,
                         archive=pin(data), receipt_sha256=RECEIPT_SHA, verified=True)))


if __name__ == '__main__':
    main()
