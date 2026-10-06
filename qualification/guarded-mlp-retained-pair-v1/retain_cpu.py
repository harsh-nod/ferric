"""Verify and retain the two recorded CPU attempts; never execute their sources."""
import hashlib
import io
import json
from pathlib import Path
import sys
import tarfile

WINDOWS = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
ATTEMPTS = {
    'v2': ('guarded-mlp-retained-pair-cpu-evidence-v228-v2.tar.gz', 932306,
           '02d0b98f5814ccde37e6527183d03334978b9af1478788b133d5f01e67fc1811', 109, 5845573,
           'evidence/complete.json', '1df321362ed50ffa24f83163be463d8b0ea04ac8bbc3024d5b212bd1f1a8d98f'),
    'v1': ('guarded-mlp-retained-pair-cpu-failure-v228-v1.tar.gz', 894880,
           'dd1f7219493ea257371c63a164761e4b0c0c9dbf816f027cc6a8dbf58c564a3d', 69, 5533235,
           'evidence/failed.json', '64d5aa6ede01ec3bac7b12ec849259df6b602c51cba7dc38476b3b99bc00562c'),
}


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    assert __debug__ and len(sys.argv) == 2 and sys.argv[1] in ATTEMPTS
    attempt = sys.argv[1]
    name, size, digest, count, expanded, receipt, receipt_digest = ATTEMPTS[attempt]
    source = WINDOWS / name
    assert source.is_file() and not source.is_symlink() and source.stat().st_size == size
    data = source.read_bytes()
    assert pin(data) == dict(bytes=size, sha256=digest)
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
        members = archive.getmembers()
        assert len(members) == count and len({m.name for m in members}) == count
        assert sum(m.size for m in members) == expanded < 64 << 20
        for m in members:
            path = Path(m.name)
            assert m.isfile() and 0 <= m.size < 16 << 20
            assert not path.is_absolute() and '..' not in path.parts and path.as_posix() == m.name
        bodies = {m.name: archive.extractfile(m).read() for m in members}
    manifest = json.loads(bodies['retention-manifest.json'])
    assert set(bodies) == set(manifest['files']) | {'retention-manifest.json'}
    for name, expected in manifest['files'].items():
        assert pin(bodies[name]) == {k: expected[k] for k in ('bytes', 'sha256')}
    assert manifest['receipt'] == manifest['files'][receipt]
    assert pin(bodies[receipt])['sha256'] == receipt_digest
    result = json.loads(bodies[receipt])
    assert result['passed'] is (attempt == 'v2') and result['postcheck_errors'] == []
    assert result['input_sources'] == result['final_sources'] and result['gpu_execution'] is False
    destination = Path(__file__).resolve().parent / ('cpu-attempt-' + attempt)
    assert not destination.exists() and not destination.is_symlink()
    destination.mkdir(mode=0o700)
    for name, body in bodies.items():
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(body)
        assert target.read_bytes() == body
    assert source.read_bytes() == data
    print(json.dumps(dict(destination=str(destination), members=count, expanded_bytes=expanded,
                         archive=pin(data), receipt_sha256=receipt_digest, verified=True)))


if __name__ == '__main__':
    main()
