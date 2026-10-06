"""Verify and retain the recorded CPU attempt as data; never execute its sources."""
import hashlib
import io
import json
from pathlib import Path
import tarfile

SOURCE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/'
              'guarded-mlp-exact-residual-native-cpu-evidence-v228-v1.tar.gz')
ARCHIVE_SHA = '521bab2749182cdc905617d7de271ce5f9577f14376f6bba98e3bc1c44b71613'
RECEIPT_SHA = '79baed8dfa3f8302ed65827092c0036ac0cbf16a019661bcf36532b8b4eada8d'
ARCHIVE_BYTES = 933207
EXPANDED_BYTES = 5907307


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    assert __debug__ and SOURCE.is_file() and not SOURCE.is_symlink()
    assert SOURCE.stat().st_size == ARCHIVE_BYTES
    data = SOURCE.read_bytes()
    assert pin(data) == dict(bytes=ARCHIVE_BYTES, sha256=ARCHIVE_SHA)
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
        members = archive.getmembers()
        assert len(members) == len({m.name for m in members}) == 108
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
    assert (tests['passed'], tests['failed'], tests['ignored']) == (1067, 0, 8)
    assert len(result['inventory']) == 1075 and len(result['artifacts']) == 5
    for phase in result['phases']:
        assert phase == json.loads(bodies['evidence/' + phase['label'] + '.result.json'])
        assert phase['exit_code'] == 0 and phase['natural_exit'] and phase['reaped'] and phase['process_group_absent']
        assert not phase['forced_cleanup'] and not phase['timed_out'] and phase['exception'] is None
    old = json.loads(bodies['lineage/base_complete.json'])
    assert pin(bodies['lineage/base_complete.json'])['sha256'] == '0d54a7f2027199ab3ec23f9794e296163caa55560280267768a8e12a8138f994'
    before = {name: {k: value[k] for k in ('bytes', 'sha256')} for name, value in old['final_sources'].items()
              if name.startswith('fe2o3/')}
    after = {name: {k: value[k] for k in ('bytes', 'sha256')} for name, value in result['final_sources'].items()
             if name.startswith('fe2o3/')}
    fixture = 'fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_interleaved_native_v1_tests.rs'
    assert before.keys() == after.keys() and len(before) == 802
    assert {name for name in before if before[name] != after[name]} == {fixture}
    assert after[fixture]['sha256'] == '893511eb030e1a546cbe03952059df8bfa299930a9e91dda6ac154866129e848'
    old_names = {row['name']: row['outcome'] for row in old['tests']['kfd-tests']['named']}
    new_names = {row['name']: row['outcome'] for row in tests['named']}
    assert len(old_names) == 1072 and len(new_names) == 1075
    assert all(new_names[name] == status for name, status in old_names.items())
    prefix = 'engineering_gfx950::peer::combined_mlp_state_v1::tests::native::paired::interleaved::'
    added = {prefix + name for name in (
        'interleaved_exact_residual_inputs_preserve_anchors_and_change_every_output',
        'interleaved_exact_residual_payload_roles_are_pair_private')}
    native = prefix + 'native_paired_guarded_mlp_exact_residual_v1'
    assert new_names.keys() - old_names.keys() == added | {native}
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
    print(json.dumps(dict(destination=str(destination), members=108, expanded_bytes=EXPANDED_BYTES,
                         archive=pin(data), receipt_sha256=RECEIPT_SHA, verified=True)))


if __name__ == '__main__':
    main()
