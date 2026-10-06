"""Authenticate and retain the MI350 CPU capsule as data, without importing it."""
import hashlib
import io
import json
from pathlib import Path
import tarfile

ARCHIVE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/'
               'guarded-mlp-model-interface-cpu-evidence-v228-v1.tar.gz')
ARCHIVE_PIN = dict(bytes=977234, sha256='f5435e67d498508cda293b3d5214f536062a49406621be1f2dcb97f635a4e246')
RECEIPT_SHA = '4a682798a23ac4c8accb0721f332a7b4e7484729bd692209d20f2883b501cee1'


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    assert __debug__ and ARCHIVE.is_file() and not ARCHIVE.is_symlink()
    data = ARCHIVE.read_bytes()
    assert pin(data) == ARCHIVE_PIN
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
        members = archive.getmembers()
        assert len(members) == len({m.name for m in members}) == 138
        assert sum(m.size for m in members) == 6082566
        for member in members:
            p = Path(member.name)
            assert member.isfile() and 0 <= member.size < 16 << 20
            assert not p.is_absolute() and '..' not in p.parts and p.as_posix() == member.name
        bodies = {m.name: archive.extractfile(m).read() for m in members}
    manifest = json.loads(bodies['retention-manifest.json'])
    assert set(bodies) == set(manifest['files']) | {'retention-manifest.json'}
    for name, row in manifest['files'].items():
        assert pin(bodies[name]) == {k: row[k] for k in ('bytes', 'sha256')}
    assert pin(bodies['evidence/complete.json'])['sha256'] == RECEIPT_SHA
    r = json.loads(bodies['evidence/complete.json'])
    assert r['passed'] is True and r['failure'] is None and r['postcheck_errors'] == []
    assert r['input_sources'] == r['final_sources'] and len(r['input_sources']) == 809
    assert r['source_unchanged'] and r['public_engineering_guarded_interface_added']
    assert r['mixed_bank_implemented'] and r['doctests_executed']
    for key in ('gpu_execution', 'native_test_executed', 'native_mixed_bank_qualified',
                'worker_integrated', 'whole_model_bank_integrated', 'production_authority', 'performance_claim'):
        assert r[key] is False
    assert len(r['phases']) == 20 and len(r['artifacts']) == 6
    tests = r['tests']['kfd-tests']
    assert (tests['passed'], tests['failed'], tests['ignored']) == (1088, 0, 8)
    assert len(r['inventory']) == 1096 and len(set(r['inventory'])) == 1096
    docs = r['tests']['interface-doc-tests']
    assert (docs['passed'], docs['failed'], docs['ignored']) == (9, 0, 0)
    assert len(docs['named']) == len(docs['inventory']) == 9
    for phase in r['phases']:
        assert phase == json.loads(bodies['evidence/' + phase['label'] + '.result.json'])
        assert phase['exit_code'] == 0 and phase['natural_exit'] and phase['reaped']
        assert phase['process_group_absent'] and not phase['forced_cleanup'] and not phase['timed_out']
        assert phase['exception'] is None and phase['storage_failure'] is None
        for key in ('stdout', 'stderr', 'command'):
            assert phase[key] == manifest['files']['evidence/' + Path(phase[key]['path']).name]
    assert pin(bodies['lineage/base_complete.json'])['sha256'] == '76c7d14beaecec2f63fff67828ff40cbe5ac439c77f27b0e621da7659ab42ca1'
    old = json.loads(bodies['lineage/base_complete.json'])
    old_names = {row['name']: row['outcome'] for row in old['tests']['kfd-tests']['named']}
    new_names = {row['name']: row['outcome'] for row in tests['named']}
    assert len(old_names) == 1079 and len(new_names) == 1096
    assert all(new_names[n] == status for n, status in old_names.items())
    added = new_names.keys() - old_names.keys()
    assert len(added) == 17 and all(new_names[n] == 'ok' for n in added)
    lib_prefix = 'engineering_gfx950::peer::combined_mlp_state_v1::paired::'
    assert sum(n.startswith(lib_prefix + 'facade::tests::') for n in added) == 2
    assert sum(n.startswith(lib_prefix + 'retained::mixed_bank::tests::') for n in added) == 12
    assert {n for n in added if not n.startswith(lib_prefix)} == {
        'guarded_facade_pair_types_have_no_borrowed_lifetime',
        'guarded_facade_group_signatures_preserve_linear_custody',
        'guarded_facade_mixed_bank_signatures_are_scoped_and_explicit',
    }
    before = {n: {k: p[k] for k in ('bytes', 'sha256')} for n, p in old['final_sources'].items() if n.startswith('fe2o3/')}
    after = {n: {k: p[k] for k in ('bytes', 'sha256')} for n, p in r['final_sources'].items() if n.startswith('fe2o3/')}
    overlay = r['source_lineage']['overlay']
    assert len(before) == 802 and len(after) == 807 and len(overlay) == 11
    assert sum(row['before'] is None for row in overlay.values()) == 5
    assert {n for n in before.keys() | after.keys() if before.get(n) != after.get(n)} == {'fe2o3/' + n for n in overlay}
    for name, row in overlay.items():
        assert before.get('fe2o3/' + name) == row['before']
        assert after['fe2o3/' + name] == row['after'] == pin(bodies['fe2o3/' + name])
    binary = r['artifacts']['kfd-lib']['pin']
    assert {k: binary[k] for k in ('bytes', 'sha256')} == dict(
        bytes=11435960, sha256='c607a1fb102d05cd56fa97e2e781c2e1718d90f24c57e7fb016b12f47138c772')
    destination = Path(__file__).resolve().parent / 'cpu-attempt-v1'
    assert not destination.exists() and not destination.is_symlink()
    destination.mkdir(mode=0o700)
    for name, body in bodies.items():
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(body)
        assert target.read_bytes() == body
    assert ARCHIVE.read_bytes() == data
    print(json.dumps(dict(verified=True, destination=str(destination), members=138,
                         archive=ARCHIVE_PIN, receipt_sha256=RECEIPT_SHA)))


if __name__ == '__main__':
    main()
