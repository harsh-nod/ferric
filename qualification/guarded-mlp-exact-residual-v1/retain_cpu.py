"""Verify and retain the recorded CPU attempt as data; never execute its sources."""
import hashlib
import io
import json
from pathlib import Path
import tarfile

SOURCE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/'
              'guarded-mlp-exact-residual-cpu-evidence-v228-v1.tar.gz')
ARCHIVE_SHA = '94f1962ec8d281a4ebf589103e5737a56ea62446444a48a725cd5151bf36c9d9'
RECEIPT_SHA = '0d54a7f2027199ab3ec23f9794e296163caa55560280267768a8e12a8138f994'
ARCHIVE_BYTES = 943117
EXPANDED_BYTES = 5952485


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    assert __debug__ and SOURCE.is_file() and not SOURCE.is_symlink()
    assert SOURCE.stat().st_size == ARCHIVE_BYTES
    data = SOURCE.read_bytes()
    assert pin(data) == dict(bytes=ARCHIVE_BYTES, sha256=ARCHIVE_SHA)
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
        members = archive.getmembers()
        assert len(members) == len({m.name for m in members}) == 113
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
    assert (tests['passed'], tests['failed'], tests['ignored']) == (1065, 0, 7)
    assert len(result['inventory']) == 1072 and len(result['artifacts']) == 5
    for phase in result['phases']:
        assert phase == json.loads(bodies['evidence/' + phase['label'] + '.result.json'])
        assert phase['exit_code'] == 0 and phase['natural_exit'] and phase['reaped'] and phase['process_group_absent']
        assert not phase['forced_cleanup'] and not phase['timed_out'] and phase['exception'] is None
    old = json.loads(bodies['lineage/base_complete.json'])
    assert pin(bodies['lineage/base_complete.json'])['sha256'] == 'f59fd6413856014abe4d6c52597fc6771a6e26272d1d05b8d578c485a04ef8bb'
    before = {name: {k: value[k] for k in ('bytes', 'sha256')} for name, value in old['final_sources'].items()
              if name.startswith('fe2o3/')}
    after = {name: {k: value[k] for k in ('bytes', 'sha256')} for name, value in result['final_sources'].items()
             if name.startswith('fe2o3/')}
    prefix = 'fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_'
    overlays = {prefix + suffix + '.rs': sha for suffix, sha in {
        'profiles_v1': '01b4eb4064e148e3985455af7999b0796e55bcc18011ef99aa62f976db1b735d',
        'v1': '67bd83fe2a1d2f51d0ecaf6025041dd45abc5bcd69dc29b4a84ad806d5de8f0d',
        'v1_tests': 'bd05c6d9a51d1595a354548fddfb9f992b645a0dfb8abcc8f078e1fd745353ce',
        'session_v1': '7ba0a2b29f8e3155ad69c192bf90a5e641ea9a82bd27d5e99893142fd2cb76fd',
        'retained_v1': '0ac67cf65e04a3cff610e86c3bc2fc92ed9544a1dc1113756028e593f5e5d487',
        'retained_v1_tests': '516fa9859e305ca5e0dea3735eb4665b9603a7f0850320af29a99e92570acf04',
    }.items()}
    assert before.keys() == after.keys() and len(before) == 802
    assert {name for name in before if before[name] != after[name]} == set(overlays)
    assert all(after[name]['sha256'] == sha for name, sha in overlays.items())
    old_names = {row['name']: row['outcome'] for row in old['tests']['kfd-tests']['named']}
    new_names = {row['name']: row['outcome'] for row in tests['named']}
    assert len(old_names) == 1067 and len(new_names) == 1072
    assert all(new_names[name] == status for name, status in old_names.items())
    prefix = 'engineering_gfx950::peer::combined_mlp_state_v1::paired::'
    added = {prefix + 'tests::' + name for name in (
        'paired_exact_residual_policy_requires_both_full_identities',
        'paired_exact_residual_policy_refuses_all_live_root_and_peer_aliases',
        'paired_exact_residual_policy_preserves_extents_and_backing_checks')}
    added |= {prefix + 'retained::tests::' + name for name in (
        'retained_exact_residual_binding_keeps_policy_and_roles_immutable',
        'retained_exact_residual_constructor_refuses_invalid_custody')}
    assert new_names.keys() - old_names.keys() == added and all(new_names[name] == 'ok' for name in added)
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
    print(json.dumps(dict(destination=str(destination), members=113, expanded_bytes=EXPANDED_BYTES,
                         archive=pin(data), receipt_sha256=RECEIPT_SHA, verified=True)))


if __name__ == '__main__':
    main()
