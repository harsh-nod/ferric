"""Retain exact pinned-ELF repeats as data, without replaying either attempt."""
import hashlib
import io
import json
from pathlib import Path
import tarfile

SOURCE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
ATTEMPTS = {
    3: dict(archive_bytes=409204, archive_sha='8ee2f0cec771f9cab68c6007f8bb9825faa09a45d93d30684bf607c9ac39595e',
            receipt_sha='4c4712c614515f683c980d0eb330ada9accab3e88fc03f50bedf6c97a8c1a57d',
            members=53, expanded=2861367, passed=False,
            elf_sha='dbf5ea87824b4ab2be1e481cf21595f5809fb95bd309552e888a32f8089a8425'),
    4: dict(archive_bytes=569615, archive_sha='f0f71517f5df03257015da1ebeeb9912742261e954d8b7cea2ae4fd36ff4b3b2',
            receipt_sha='eacd2fadb52d33f96361fd8f9d927a5989a8292da4fca876eeafa382e2f268a4',
            members=59, expanded=13792855, passed=True,
            elf_sha='5dfefcd2e2280b6dc83e63a241c3f518ab3bfa38fa705a4cf070d886ac6d47f0'),
}


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    assert __debug__
    prepared = []
    for version, expected in ATTEMPTS.items():
        source = SOURCE / f'guarded-mlp-interleaved-native-gpu-evidence-v228-v{version}.tar.gz'
        assert source.is_file() and not source.is_symlink()
        assert source.stat().st_size == expected['archive_bytes']
        data = source.read_bytes()
        assert pin(data) == dict(bytes=expected['archive_bytes'], sha256=expected['archive_sha'])
        with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
            members = archive.getmembers()
            assert len(members) == len({m.name for m in members}) == expected['members']
            assert sum(m.size for m in members) == expected['expanded'] < 64 << 20
            for member in members:
                path = Path(member.name)
                assert member.isfile() and 0 <= member.size < 16 << 20
                assert not path.is_absolute() and '..' not in path.parts and path.as_posix() == member.name
            bodies = {m.name: archive.extractfile(m).read() for m in members}
        manifest = json.loads(bodies['retention-manifest.json'])
        assert set(bodies) == set(manifest['files']) | {'retention-manifest.json'}
        for name, value in manifest['files'].items():
            assert pin(bodies[name]) == {k: value[k] for k in ('bytes', 'sha256')}
        receipt = 'evidence/' + ('complete.json' if expected['passed'] else 'failed.json')
        assert manifest['receipt'] == manifest['files'][receipt]
        assert pin(bodies[receipt])['sha256'] == expected['receipt_sha']
        result = json.loads(bodies[receipt])
        cpu = json.loads(bodies['lineage/cpu-complete.json'])
        assert result['binary'] == cpu['artifacts']['kfd-lib']['pin']
        assert result['binary']['sha256'] == expected['elf_sha']
        assert result['passed'] is expected['passed'] and result['postcheck_errors'] == []
        assert result['gpu_attempts'] == result['completed_native_phases'] == 1
        assert result['native_spawn_observed'] is True and result['retries'] == 0
        assert result['full_model_acceptance'] is False and result['performance_claim'] is False
        for phase in result['phases']:
            assert phase == json.loads(bodies['evidence/' + phase['label'] + '.result.json'])
            assert phase['natural_exit'] and phase['reaped'] and phase['process_group_absent']
            assert not phase['forced_cleanup'] and not phase['timed_out']
            assert phase['exception'] is None and phase['storage_failure'] is None
            assert phase['exit_code'] == (101 if version == 3 and phase['label'] == 'native' else 0)
        if expected['passed']:
            assert result['errors'] == []
            assert result['verification'] == json.loads(bodies['evidence/verify.stdout'])
            assert result['verification']['passed'] and result['observation']['healthy_close']
            assert result['verification']['computed_elements_checked'] == 557056
            assert result['verification']['final_output_elements_checked'] == 65536
        else:
            assert result['verification'] is result['observation'] is None
            assert b'paired guarded MLP typed allocation role' in bodies['evidence/native.stdout']
            assert b'FERRIC_NATIVE_PAIRED_INTERLEAVED_MLP_V1=' not in bodies['evidence/native.stdout']
        destination = Path(__file__).resolve().parent / f'gpu-attempt-v{version}'
        assert not destination.exists() and not destination.is_symlink()
        prepared.append((source, data, destination, bodies, expected))
    for source, data, destination, bodies, expected in prepared:
        destination.mkdir(mode=0o700)
        for name, body in bodies.items():
            target = destination / name
            target.parent.mkdir(parents=True, exist_ok=True)
            with target.open('xb') as stream:
                stream.write(body)
            assert target.read_bytes() == body
        assert source.read_bytes() == data
        print(json.dumps(dict(destination=str(destination), archive=pin(data),
                             members=expected['members'], expanded_bytes=expected['expanded'],
                             receipt_sha256=expected['receipt_sha'], verified=True,
                             passed=expected['passed'])))


if __name__ == '__main__':
    main()
