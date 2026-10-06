"""Verify and retain the recorded CPU attempt as data; never execute its sources."""
import hashlib
import io
import json
from pathlib import Path
import tarfile

SOURCE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/'
              'guarded-mlp-deferred-binding-cpu-evidence-v228-v1.tar.gz')
ARCHIVE_SHA = '5528a34505c1d7d2bb41a92e4cc1bad390132a6cebe33668678e7c365ddfabfc'
RECEIPT_SHA = '76c7d14beaecec2f63fff67828ff40cbe5ac439c77f27b0e621da7659ab42ca1'
ARCHIVE_BYTES = 944653
EXPANDED_BYTES = 5993521


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    assert __debug__ and SOURCE.is_file() and not SOURCE.is_symlink()
    assert SOURCE.stat().st_size == ARCHIVE_BYTES
    data = SOURCE.read_bytes()
    assert pin(data) == dict(bytes=ARCHIVE_BYTES, sha256=ARCHIVE_SHA)
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
        members = archive.getmembers()
        assert len(members) == len({m.name for m in members}) == 111
        assert sum(m.size for m in members) == EXPANDED_BYTES < 64 << 20
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
    assert result['passed'] is True and result['failure'] is None and result['postcheck_errors'] == []
    assert result['input_sources'] == result['final_sources'] and result['gpu_execution'] is False
    assert len(result['input_sources']) == 804 and len(result['phases']) == 16
    tests = result['tests']['kfd-tests']
    assert (tests['passed'], tests['failed'], tests['ignored']) == (1071, 0, 8)
    assert len(result['inventory']) == 1079 and len(result['artifacts']) == 5
    assert result['private_deferred_binding_added'] is True
    for phase in result['phases']:
        assert phase == json.loads(bodies['evidence/' + phase['label'] + '.result.json'])
        assert phase['exit_code'] == 0 and phase['natural_exit'] and phase['reaped'] and phase['process_group_absent']
        assert not phase['forced_cleanup'] and not phase['timed_out'] and phase['exception'] is None
    old = json.loads(bodies['lineage/base_complete.json'])
    assert pin(bodies['lineage/base_complete.json'])['sha256'] == '79baed8dfa3f8302ed65827092c0036ac0cbf16a019661bcf36532b8b4eada8d'
    before = {name: {k: value[k] for k in ('bytes', 'sha256')} for name, value in old['final_sources'].items()
              if name.startswith('fe2o3/')}
    after = {name: {k: value[k] for k in ('bytes', 'sha256')} for name, value in result['final_sources'].items()
             if name.startswith('fe2o3/')}
    source = 'fe2o3/crates/fe2o3-kfd/src/'
    qualified = {
        'engineering_gfx950_peer_combined_mlp_paired_v1.rs': 'afb10f6c546f84de0304259986f89754ac765fe42147dae0a3e083c0b5081353',
        'engineering_gfx950_peer_combined_mlp_paired_retained_v1.rs': 'ad028425d67a51daaf8239c0eff2ac9ee7885af94621d5f54c4bebf23daffcb3',
        'engineering_gfx950_peer_combined_mlp_paired_retained_v1_tests.rs': 'afec0abbf4531e1383ce8de60cf9eeefb365ca7cac04e7a2e55ec1a57b4f9e48',
        'engineering_gfx950_peer_combined_mlp_paired_interleaved_native_v1_tests.rs': '1115de7f4cbe01110bfb0f23a307e4d30790f484c4556ca9109a02b4af2ca3bd',
    }
    assert before.keys() == after.keys() and len(before) == 802
    assert {name for name in before if before[name] != after[name]} == {source + name for name in qualified}
    assert all(after[source + name]['sha256'] == sha for name, sha in qualified.items())
    old_names = {row['name']: row['outcome'] for row in old['tests']['kfd-tests']['named']}
    new_names = {row['name']: row['outcome'] for row in tests['named']}
    assert len(old_names) == 1075 and len(new_names) == 1079
    assert all(new_names[name] == status for name, status in old_names.items())
    prefix = 'engineering_gfx950::peer::combined_mlp_state_v1::paired::retained::tests::'
    added = {prefix + name for name in (
        'retained_unbound_identity_requires_initial_private_owners',
        'retained_unbound_storage_refuses_invalid_custody',
        'retained_unbound_bind_consumes_and_quarantines_invalid_custody',
        'retained_partial_allocation_custody_poisoned_on_drop_and_unwind')}
    native = 'engineering_gfx950::peer::combined_mlp_state_v1::tests::native::paired::interleaved::native_paired_guarded_mlp_exact_residual_v1'
    assert new_names.keys() - old_names.keys() == added
    assert all(new_names[name] == 'ok' for name in added) and new_names[native] == 'ignored'
    assert result['native_test_name'] == native and result['native_test_executed'] is False
    destination = Path(__file__).resolve().parent / 'cpu-attempt-v1'
    assert not destination.exists() and not destination.is_symlink()
    destination.mkdir(mode=0o700)
    for name, body in bodies.items():
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(body)
        assert target.read_bytes() == body
    assert SOURCE.read_bytes() == data
    print(json.dumps(dict(destination=str(destination), members=111, expanded_bytes=EXPANDED_BYTES,
                         archive=pin(data), receipt_sha256=RECEIPT_SHA, verified=True)))


if __name__ == '__main__':
    main()
