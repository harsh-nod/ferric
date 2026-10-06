"""Retain the actual failed GPU attempt as data, without replaying its inputs."""
import hashlib
import io
import json
from pathlib import Path
import tarfile

SOURCE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/'
              'guarded-mlp-interleaved-native-gpu-evidence-v228-v1.tar.gz')
ARCHIVE_SHA = '4c3f37eef38cc0ed43b5c6154a4b882cfb12da0c396de2217f49ddceaac31d46'
RECEIPT_SHA = '559b515035b9a4902c24375798b7c60b2274d4470f5651c91743f583573823e8'


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    assert __debug__ and SOURCE.is_file() and not SOURCE.is_symlink()
    assert SOURCE.stat().st_size == 409192
    data = SOURCE.read_bytes()
    assert pin(data) == dict(bytes=409192, sha256=ARCHIVE_SHA)
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
        members = archive.getmembers()
        assert len(members) == len({m.name for m in members}) == 53
        assert sum(m.size for m in members) == 2861223 < 64 << 20
        for member in members:
            path = Path(member.name)
            assert member.isfile() and 0 <= member.size < 16 << 20
            assert not path.is_absolute() and '..' not in path.parts and path.as_posix() == member.name
        bodies = {m.name: archive.extractfile(m).read() for m in members}
    manifest = json.loads(bodies['retention-manifest.json'])
    assert set(bodies) == set(manifest['files']) | {'retention-manifest.json'}
    for name, expected in manifest['files'].items():
        assert pin(bodies[name]) == {k: expected[k] for k in ('bytes', 'sha256')}
    receipt = 'evidence/failed.json'
    assert manifest['receipt'] == manifest['files'][receipt]
    assert pin(bodies[receipt])['sha256'] == RECEIPT_SHA
    result = json.loads(bodies[receipt])
    assert result['passed'] is False and result['postcheck_errors'] == []
    assert result['gpu_attempts'] == 1 and result['retries'] == 0
    assert result['verification'] is None and result['observation'] is None
    assert result['full_model_acceptance'] is False and result['performance_claim'] is False
    destination = Path(__file__).resolve().parent / 'gpu-attempt-v1'
    assert not destination.exists() and not destination.is_symlink()
    destination.mkdir(mode=0o700)
    for name, body in bodies.items():
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(body)
        assert target.read_bytes() == body
    assert SOURCE.read_bytes() == data
    print(json.dumps(dict(destination=str(destination), members=53, expanded_bytes=2861223,
                         archive=pin(data), receipt_sha256=RECEIPT_SHA, verified=True, passed=False)))


if __name__ == '__main__':
    main()
