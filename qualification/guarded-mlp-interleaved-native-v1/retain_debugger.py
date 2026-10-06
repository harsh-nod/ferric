"""Validate and retain debugger observations as data; never replay project code."""
import hashlib
import io
import json
from pathlib import Path
import tarfile

SOURCE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
ATTEMPTS = {
    1: dict(archive_bytes=420886, archive_sha='aaffb61ea8c696b972a665c9903105cbe87ece555d43f748350f18e4eb7540e3',
            receipt_sha='589f184c68643298d1d34f30c1f0505502f9fe68fac42d64a093e7572579492a',
            members=74, expanded=2922375, passed=False, native_exit=1),
    2: dict(archive_bytes=422345, archive_sha='092ece748e5f74476216d8b86da55b9b145fcae1096560bfb47a00f4250b85fc',
            receipt_sha='3ea6a047a4f91407d904763af59761f52ca3122c7c8f96e12653ac82a94938cb',
            members=75, expanded=2924460, passed=True, native_exit=101),
    3: dict(archive_bytes=425770, archive_sha='864312231e2feba322033706dcc935747960dd142d32380a5c598dcff6448fb5',
            receipt_sha='7c8827c8f906bb4b5b6e9e7bd92b4cf1ec9e06f23e848e0247269de8b107b049',
            members=76, expanded=2937043, passed=True, native_exit=101),
    4: dict(archive_bytes=425821, archive_sha='caed846e35a98348a2a8f5cce4970393684db26f1551a650d6dcf6b00863ae0f',
            receipt_sha='b02b083fdb9ecd889074704b8b3791e209c616e62d39054b1429511dde732716',
            members=76, expanded=2937173, passed=True, native_exit=101),
}


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    assert __debug__
    for version, expected in ATTEMPTS.items():
        source = SOURCE / f'guarded-mlp-interleaved-debugger-evidence-v228-v{version}.tar.gz'
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
        assert manifest['schema'] == 'ferric-interleaved-mlp-debugger-retention-v1'
        assert set(bodies) == set(manifest['files']) | {'retention-manifest.json'}
        for name, value in manifest['files'].items():
            assert pin(bodies[name]) == {k: value[k] for k in ('bytes', 'sha256')}
        receipt = 'evidence/' + ('complete.json' if expected['passed'] else 'failed.json')
        assert manifest['receipt'] == manifest['files'][receipt]
        assert pin(bodies[receipt])['sha256'] == expected['receipt_sha']
        result = json.loads(bodies[receipt])
        cpu = json.loads(bodies['lineage/cpu-complete.json'])
        assert result['schema'] == 'ferric-interleaved-mlp-debugger-result-v1'
        assert result['binary'] == cpu['artifacts']['kfd-lib']['pin']
        assert result['binary']['sha256'] == 'dbf5ea87824b4ab2be1e481cf21595f5809fb95bd309552e888a32f8089a8425'
        assert result['passed'] is expected['passed'] and result['postcheck_errors'] == []
        assert result['gpu_attempts'] == result['completed_native_phases'] == 1
        assert result['native_spawn_observed'] is True and result['retries'] == 0
        assert result['debugger_assisted'] is True
        assert result['refusal_capture_completed'] is expected['passed']
        for key in ('full_model_acceptance', 'performance_claim', 'production_authority',
                    'worker_integrated', 'synthetic_paired_component_qualified',
                    'paired_coordinator_tested', 'owner_lifecycle_tested', 'rearm_tested', 'interleaving_tested'):
            assert result[key] is False
        assert len(result['phases']) == len(result['owned_retirements']) == 9
        for phase, retirement in zip(result['phases'], result['owned_retirements']):
            label = phase['label']
            assert phase == json.loads(bodies['evidence/' + label + '.result.json'])
            assert retirement == json.loads(bodies['evidence/' + label + '.owned-children.json'])
            assert retirement['label'] == label and retirement['complete'] and retirement['no_children']
            assert not retirement['forced'] and not retirement['deferred_signals'] and retirement['error'] is None
            assert phase['natural_exit'] and phase['reaped'] and phase['process_group_absent']
            assert not phase['forced_cleanup'] and not phase['timed_out']
            assert phase['exception'] is None and phase['storage_failure'] is None
            assert phase['exit_code'] == (expected['native_exit'] if label == 'native' else 0)
            for key in ('stdout', 'stderr', 'command'):
                name = 'evidence/' + Path(phase[key]['path']).name
                assert phase[key] == manifest['files'][name]
        cleanup = json.loads(bodies['evidence/owned-cleanup-tests.stdout'])
        assert cleanup == result['owned_cleanup_selftest']
        assert cleanup['passed'] and len(set(cleanup['tests'])) == 8
        assert cleanup['gpu_execution'] is cleanup['project_execution'] is False
        assert len(result['idle_before']) == len(result['idle_after']) == 8
        if not expected['passed']:
            assert result['verification'] is result['observation'] is None
            assert b'process_t::read_global_memory failed: Cannot access memory at global#0' in bodies['evidence/native.stderr']
            assert b'FERRIC_ROLE_PREDICATE_V1=' not in bodies['evidence/native.stdout']
        else:
            assert result['errors'] == []
            assert result['observation'] == json.loads(bodies['evidence/observation.json'])
            observations = [json.loads(line.split(b'=', 1)[1]) for line in bodies['evidence/native.stdout'].splitlines()
                            if line.startswith(b'FERRIC_ROLE_PREDICATE_V1=')]
            assert observations == [result['observation']]
            assert result['verification']['end'] == dict(mode='native', exit_codes=[101], hits=1)
            o, v = result['observation'], result['verification']
            assert o['source_token'] == o['copied_token'] == o['record_token'] == [1, 1, 0, 8192]
            assert o['role'] == 'root' and o['root_index'] == o['saved_rank'] == o['expected_owner'] == 1
            assert o['saved_byte_index'] == 32 and o['expected_bytes'] == 8192
            assert o['combined_u32'] == o['record_kind_u8'] == 0 and o['mapping_u8'] == 1
            assert o['source_address_matches_role'] and o['record_pointer_matches_return']
            assert o['validation_tag_u64'] == 1 << 63
            assert v['first_false_predicate'] == 'owner' and v['tokens_equal'] and v['root_loop_valid']
            assert v['predicates'] == dict(owner=False, extent=True, kind=True, mapping=True)
            assert v['start']['host_only_debugger'] and 'amd-dbgapi' not in v['start']['target_stack']
            if version >= 3:
                chain = result['token_chain']
                assert chain == json.loads(bodies['evidence/token-chain.json'])
                prefixes = (b'FERRIC_COMMON_SELECTION_V1=', b'FERRIC_PAYLOAD_TRANSFER_V1=', b'FERRIC_PREPARE_INPUT_V1=')
                lines = bodies['evidence/native.stdout'].splitlines()
                assert [json.loads(next(line for line in lines if line.startswith(prefix)).split(b'=', 1)[1])
                        for prefix in prefixes] == chain
                selected, transfer, prepare = chain
                assert selected['common_rows'] == [
                    [[1, 1, 0, 8192], [1, 2, 0, 50331648], [1, 3, 0, 50331648], [1, 4, 0, 50331648]],
                    [[1, 5, 1, 8192], [1, 6, 1, 50331648], [1, 7, 1, 50331648], [1, 8, 1, 50331648]]]
                assert selected['selected_token'] == transfer['payload_token'] == transfer['inputs_token']
                assert transfer['inputs_token'] == prepare['inputs_token'] == o['source_token']
                assert o['inputs_pointer_matches_transfer'] and transfer['common_unchanged']
                assert selected['actual_delta'] == 0 and selected['root_index'] == 1
                assert selected['rank'] == (0 if version == 3 else 1)
                assert selected['expected_delta'] == (0 if version == 3 else 128)
                checks = v['token_chain']
                assert checks == dict(selector_site_matches=version == 4,
                    source_rows_have_expected_owners_and_extents=True, source_ids_distinct=True,
                    source_unchanged=True, selected_rank0_instead_of_rank1=version == 4,
                    selected_token_preserved=True, transfer_and_prepare_pointer_joins=True)
        destination = Path(__file__).resolve().parent / f'debugger-attempt-v{version}'
        assert not destination.is_symlink()
        if destination.exists():
            assert {p.relative_to(destination).as_posix() for p in destination.rglob('*') if p.is_file()} == set(bodies)
            assert all((destination / name).read_bytes() == body for name, body in bodies.items())
        else:
            destination.mkdir(mode=0o700)
            for name, body in bodies.items():
                target = destination / name
                target.parent.mkdir(parents=True, exist_ok=True)
                with target.open('xb') as stream:
                    stream.write(body)
                assert target.read_bytes() == body
        assert source.read_bytes() == data
        print(json.dumps(dict(destination=str(destination), archive=pin(data), members=expected['members'],
                             expanded_bytes=expected['expanded'], verified=True, passed=expected['passed'])))


if __name__ == '__main__':
    main()
