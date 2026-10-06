"""Authenticate and retain the actual GPU capsule as data, never replay it."""
import hashlib
import io
import json
from pathlib import Path
import tarfile

SOURCE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/'
              'guarded-mlp-exact-residual-native-gpu-evidence-v228-v1.tar.gz')
ARCHIVE_SHA = 'c79b0b9b680a41925ff131e21a16e1c2e694727da769ccda35e91c7eeaa25131'
RECEIPT_SHA = '448f0fffb63cd5cc5ad72989f37b3c7896118df29a7fc92f2318020b218bbe69'


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    assert __debug__ and SOURCE.is_file() and not SOURCE.is_symlink()
    assert SOURCE.stat().st_size == 575787
    data = SOURCE.read_bytes()
    assert pin(data) == dict(bytes=575787, sha256=ARCHIVE_SHA)
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
        members = archive.getmembers()
        assert len(members) == len({m.name for m in members}) == 59
        assert sum(m.size for m in members) == 13838695 < 64 << 20
        for member in members:
            path = Path(member.name)
            assert member.isfile() and 0 <= member.size < 16 << 20
            assert not path.is_absolute() and '..' not in path.parts and path.as_posix() == member.name
        bodies = {m.name: archive.extractfile(m).read() for m in members}
    manifest = json.loads(bodies['retention-manifest.json'])
    assert set(bodies) == set(manifest['files']) | {'retention-manifest.json'}
    for name, value in manifest['files'].items():
        assert pin(bodies[name]) == {k: value[k] for k in ('bytes', 'sha256')}
    receipt = 'evidence/complete.json'
    assert manifest['receipt'] == manifest['files'][receipt]
    assert pin(bodies[receipt])['sha256'] == RECEIPT_SHA
    result = json.loads(bodies[receipt])
    cpu = json.loads(bodies['lineage/cpu-complete.json'])
    assert pin(bodies['lineage/cpu-complete.json'])['sha256'] == '79baed8dfa3f8302ed65827092c0036ac0cbf16a019661bcf36532b8b4eada8d'
    assert result['schema'] == 'ferric-native-exact-residual-mlp-gpu-result-v1'
    assert result['binary'] == cpu['artifacts']['kfd-lib']['pin'] == manifest['binary']
    assert result['binary']['sha256'] == '7da8e4b727b9006e17c61b7cf1fb243afe249793844685229fc3247ec35267c4'
    assert result['passed'] is True and result['errors'] == result['postcheck_errors'] == []
    assert result['gpu_attempts'] == result['completed_native_phases'] == 1
    assert result['native_spawn_observed'] is True and result['retries'] == 0
    assert result['exact_residual_reuse_tested'] is True
    for flag in ('worker_integrated', 'full_model_acceptance', 'production_authority', 'performance_claim'):
        assert result[flag] is False
    assert len(result['phases']) == 8
    for phase in result['phases']:
        assert phase == json.loads(bodies['evidence/' + phase['label'] + '.result.json'])
        assert phase['exit_code'] == 0 and phase['natural_exit'] and phase['reaped'] and phase['process_group_absent']
        assert not phase['forced_cleanup'] and not phase['timed_out']
        assert phase['exception'] is None and phase['storage_failure'] is None
    selftest = json.loads(bodies['evidence/reference-tests.stdout'])
    assert selftest == result['reference_selftest'] and selftest['passed'] is True
    assert selftest['gpu_execution'] is False and selftest['observation_mutation_refusals'] == 918
    assert selftest['fixed_output_hashes'] == selftest['fixed_up_matrix_hashes'] == 16
    assert selftest['strict_json_refusals'] == selftest['stdout_refusals'] == 2
    verification = json.loads(bodies['evidence/verify.stdout'])
    assert result['verification'] == verification and verification['passed'] is True
    assert verification['schema'] == 'ferric-native-exact-residual-mlp-dyadic-verification-v1'
    assert verification['stdout_sha256'] == pin(bodies['evidence/native.stdout'])['sha256']
    assert verification['request_sha256'] == pin(bodies['request.json'])['sha256']
    for key, expected in dict(computed_elements_checked=557056, final_output_elements_checked=65536,
                              dense_matrix_hashes_checked=48, distinct_dense_matrix_hashes=20,
                              owner_readback_observations_checked=104, terminal_owner_observations_checked=16,
                              inactive_payload_hashes_checked=336, segments_checked=8, bank_rearms_checked=2,
                              residual_alias_observations_checked=16, distinct_residual_output_allocations=8,
                              overwritten_residual_elements_checked=65536).items():
        assert verification[key] == expected
    observation = json.loads(bodies['evidence/observation.json'])
    assert result['observation'] == observation and observation['healthy_close'] is True
    assert observation['schema'] == 'ferric-native-paired-guarded-mlp-exact-residual-observation-v1'
    assert observation['exact_residual_reuse_tested'] is True
    marker = b'FERRIC_NATIVE_PAIRED_EXACT_RESIDUAL_MLP_V1='
    rows = [line.split(marker, 1)[1] for line in bodies['evidence/native.stdout'].splitlines() if marker in line]
    assert len(rows) == 1 and json.loads(rows[0]) == observation
    for name, expected in {
        'r1.hsaco': '25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25',
        'mlp.hsaco': 'b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589',
        'guarded.hsaco': 'de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66',
    }.items():
        assert pin(bodies[name])['sha256'] == expected
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
    print(json.dumps(dict(destination=str(destination), archive=pin(data), members=59,
                         expanded_bytes=13838695, receipt_sha256=RECEIPT_SHA, verified=True, passed=True)))


if __name__ == '__main__':
    main()
