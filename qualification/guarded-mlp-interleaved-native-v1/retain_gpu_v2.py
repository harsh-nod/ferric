"""Retain the successful instrumented attempt as data, without replaying inputs."""
import hashlib
import io
import json
from pathlib import Path
import tarfile

SOURCE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/'
              'guarded-mlp-interleaved-native-gpu-evidence-v228-v2.tar.gz')
ARCHIVE_SHA = 'a26f86e93d7dee69a909701595ff187e910d100ff10a069dccb5277834a837e7'
RECEIPT_SHA = 'ab17388f411190bf0c738fdf8a7ed4b598606e3147ce98224573ea7ebb2c25d9'


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    assert __debug__ and SOURCE.is_file() and not SOURCE.is_symlink()
    assert SOURCE.stat().st_size == 569836
    data = SOURCE.read_bytes()
    assert pin(data) == dict(bytes=569836, sha256=ARCHIVE_SHA)
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
        members = archive.getmembers()
        assert len(members) == len({m.name for m in members}) == 59
        assert sum(m.size for m in members) == 13792717 < 64 << 20
        for member in members:
            path = Path(member.name)
            assert member.isfile() and 0 <= member.size < 16 << 20
            assert not path.is_absolute() and '..' not in path.parts and path.as_posix() == member.name
        bodies = {m.name: archive.extractfile(m).read() for m in members}
    manifest = json.loads(bodies['retention-manifest.json'])
    assert set(bodies) == set(manifest['files']) | {'retention-manifest.json'}
    for name, expected in manifest['files'].items():
        assert pin(bodies[name]) == {k: expected[k] for k in ('bytes', 'sha256')}
    receipt = 'evidence/complete.json'
    assert manifest['receipt'] == manifest['files'][receipt]
    assert pin(bodies[receipt])['sha256'] == RECEIPT_SHA
    result = json.loads(bodies[receipt])
    assert result['passed'] is True and result['errors'] == result['postcheck_errors'] == []
    assert result['gpu_attempts'] == 1 and result['retries'] == 0
    assert result['verification']['passed'] is True and result['observation']['healthy_close'] is True
    assert result['verification']['computed_elements_checked'] == 557056
    assert result['verification']['final_output_elements_checked'] == 65536
    assert result['full_model_acceptance'] is False and result['performance_claim'] is False
    destination = Path(__file__).resolve().parent / 'gpu-attempt-v2'
    assert not destination.exists() and not destination.is_symlink()
    destination.mkdir(mode=0o700)
    for name, body in bodies.items():
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(body)
        assert target.read_bytes() == body
    assert SOURCE.read_bytes() == data
    print(json.dumps(dict(destination=str(destination), members=59, expanded_bytes=13792717,
                         archive=pin(data), receipt_sha256=RECEIPT_SHA, verified=True, passed=True)))


if __name__ == '__main__':
    main()
