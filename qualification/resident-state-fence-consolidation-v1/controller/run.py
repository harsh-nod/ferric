"""Fresh-source resident-state regression; unchanged bounded helper, no GPU."""
import hashlib
import json
import os
import re
from pathlib import Path
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
PACKAGE = E / 'p228-resident-state-fence-cpu-v2'
OVERLAY = E / 'p228-group-fence-consolidation-v1'
OVERLAY_SHA = '9a8f1d1f4da5cc846891f4721f7b37783777e4c2a368c5bf1fc340cface77834'
FILTERS = (
    ('context', 'engineering_gfx950::tests::', 14),
    ('peer', 'engineering_gfx950::peer::tests::', 15),
    ('device-group', 'device::gfx950::group_currentness::tests::', 7),
    ('publication', 'engineering_gfx950::peer::round::tests::', 16),
    ('observation', 'engineering_gfx950::peer::host_observation::tests::', 13),
    ('policy', 'engineering_gfx950::peer::performance::tests::', 5),
    ('ordered', 'engineering_gfx950::ordered_batch::tests::', 7),
    ('prefix-state', 'engineering_gfx950::peer::wave_qkv_attention_output_tiles_state_v6::tests::', 10),
    ('prefix-resident', 'engineering_gfx950::peer::wave_qkv_attention_output_tiles_v6::tests::', 10),
    ('mlp-state', 'engineering_gfx950::peer::wave_mlp_tiles_state_v2::tests::', 10),
    ('mlp-resident', 'engineering_gfx950::peer::wave_mlp_tiles_v2::tests::', 10),
    ('mlp-timestamps', 'engineering_gfx950::peer::wave_mlp_tiles_v2::timestamp_tests::', 7),
)

NEW_TESTS = {
    "engineering_gfx950::peer::wave_qkv_attention_output_tiles_state_v6::tests::v6_private_resident_read_checks_active_phase_and_owner_before_storage",
    "engineering_gfx950::peer::wave_qkv_attention_output_tiles_state_v6::tests::v6_private_resident_read_keeps_token_kind_extent_and_mapping_refusals",
    "engineering_gfx950::peer::wave_qkv_attention_output_tiles_v6::tests::resident_v6_native_adapter_routes_to_private_guard_not_public_observer",
    "engineering_gfx950::peer::wave_mlp_tiles_state_v2::tests::v2_private_resident_read_checks_active_phase_and_owner_before_storage",
    "engineering_gfx950::peer::wave_mlp_tiles_state_v2::tests::v2_private_resident_read_keeps_token_kind_extent_and_mapping_refusals",
    "engineering_gfx950::peer::wave_mlp_tiles_v2::tests::resident_v2_native_adapter_routes_to_private_guard_not_public_observer",
}


def package_inputs(expected_sha):
    assert re.fullmatch(r'[0-9a-f]{64}', expected_sha)
    manifest = PACKAGE / 'manifest.json'
    assert manifest.resolve(strict=True) == manifest and not manifest.is_symlink()
    assert hashlib.sha256(manifest.read_bytes()).hexdigest() == expected_sha
    value = json.loads(manifest.read_bytes())
    assert value['schema'] == 'ferric-p228-resident-state-fence-cpu-package-v1'
    assert {row['path'] for row in value['files']} == {'run.py', 'overlay.json', 'README.md'}
    assert len(value['files']) == 3
    assert Path(__file__).resolve(strict=True) == PACKAGE / 'run.py'
    for row in value['files']:
        path = PACKAGE / row['path']
        assert path.resolve(strict=True) == path and not path.is_symlink()
        data = path.read_bytes()
        assert len(data) == row['bytes'] and hashlib.sha256(data).hexdigest() == row['sha256']
    return [manifest, *(PACKAGE / row['path'] for row in value['files'])]


def main():
    assert len(sys.argv) == 3 and not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
    manifest_sha, label = sys.argv[1:]
    assert re.fullmatch(r'resident-state-fence-cpu-v228-v[1-9][0-9]*', label)
    package_paths = package_inputs(manifest_sha)
    OUT = E / label
    helper = E / 'p228-host-policy-cpu-v1/run.py'
    raw = helper.read_bytes()
    assert helper.resolve(strict=True) == helper
    assert hashlib.sha256(raw).hexdigest() == '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494'
    h = types.ModuleType('tested_policy_helpers')
    h.__file__ = str(helper)
    exec(compile(raw, str(helper), 'exec'), h.__dict__)
    x = h.load(h.EXTRACTOR, h.EXTRACTOR_SHA, 'archive_helpers')
    n = h.load(h.BOUNDS, h.BOUNDS_SHA, 'owned_cpu_leaves')
    assert h.sha(h.SOURCES) == h.SOURCES_SHA
    inputs = json.loads(h.SOURCES.read_bytes())
    assert inputs['archives']['fe2o3']['commit'] == '6964f6129c4c42d343117f61cdbcfe6166392535'
    state = json.loads((PACKAGE / 'overlay.json').read_bytes())
    assert state['schema'] == 'ferric-p228-resident-state-fence-cpu-overlay-v1'
    assert state['baseline_head'] == '725ecc6a500ff49e7dfaefb38b027f6bcc223ebf'
    assert state['source_directory'] == 'p228-resident-state-fence-consolidation-v2'
    state_directory = E / state['source_directory']
    preimage_pin = x.pin(state_directory / 'preimages.json')
    assert {key: preimage_pin[key] for key in ('bytes', 'sha256')} == state['preimages']
    preimages = json.loads((state_directory / 'preimages.json').read_bytes())
    assert preimages['schema'] == 'ferric-p228-resident-state-fence-source-preimages-v1'
    assert preimages['base_commit'] == state['baseline_head']
    assert len(state['files']) == 8 and len(preimages['files']) == 8
    assert {row['path']: row['before']['sha256'] for row in state['files']} == preimages['files']
    overlays = []
    pinned = [x.pin(path) for path in package_paths]
    pinned.extend([x.pin(helper), x.pin(h.SOURCES), x.pin(h.EXTRACTOR), x.pin(h.BOUNDS), preimage_pin])
    for project, directory, sha, count in (
        ('ferric', E / 'p228-host-policy-v1', h.OVERLAY_SHA, 13),
        ('fe2o3', OVERLAY, OVERLAY_SHA, 3),
    ):
        assert h.sha(directory / 'manifest.json') == sha
        pinned.append(x.pin(directory / 'manifest.json'))
        value = json.loads((directory / 'manifest.json').read_bytes())
        assert value['baseline_head'] == inputs['archives'][project]['commit']
        assert len(value['files']) == count
        overlays.append((project, directory / 'draft', value))
    overlays.append(('fe2o3', state_directory / 'source', state))
    for project, directory, value in overlays:
        assert len({row['path'] for row in value['files']}) == len(value['files'])
        for row in value['files']:
            relative = Path(row['path'])
            assert not relative.is_absolute() and '..' not in relative.parts
            assert relative.parts[0] == ('adapters' if project == 'ferric' else 'crates')
            path = directory / relative
            assert path.resolve(strict=True) == path
            pin = x.pin(path)
            assert {key: pin[key] for key in ('bytes', 'sha256')} == row['after']
            pinned.append(pin)
    OUT.mkdir(mode=0o700)
    source = OUT / 'sources'
    source.mkdir()
    n.F, n.T = source / 'fe2o3', OUT / 'target'
    n.setup()
    assert not any(n.T.iterdir())
    (OUT / 'tmp').mkdir()
    for name, row in inputs['archives'].items():
        assert x.pin(Path(row['path'])) == {key: row[key] for key in ('path', 'bytes', 'sha256')}
        pinned.append(x.pin(Path(row['path'])))
        x.extract(Path(row['path']), name, source)
    before = x.snapshot(source)
    expected = dict(before)
    n.save(OUT / 'sources-base.json', before)
    for project, directory, value in overlays:
        # Each overlay is joined to the actual preceding source generation,
        # not to the historical Git label attached to another overlay.
        for row in value['files']:
            key = project + '/' + row['path']
            assert expected.get(key) == row['before'], ('overlay preimage', key)
            target = source / key
            if row['before'] is None:
                assert not target.exists() and not target.is_symlink()
            else:
                actual = x.pin(target)
                assert {k: actual[k] for k in ('bytes', 'sha256')} == row['before']
        for row in value['files']:
            target = source / project / row['path']
            target.parent.mkdir(parents=True, exist_ok=True)
            with target.open('xb' if row['before'] is None else 'wb') as stream:
                stream.write((directory / row['path']).read_bytes())
            expected[project + '/' + row['path']] = row['after']
        assert x.snapshot(source) == expected
    assert x.snapshot(source) == expected
    n.save(OUT / 'sources-before.json', expected)
    worker = source / 'ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml'
    cargo = str(n.N / 'bin/cargo')
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(worker)]
    env = dict(n.environment(), CARGO_PROFILE_TEST_OPT_LEVEL='2',
               CARGO_PROFILE_DEV_OPT_LEVEL='2', TMPDIR=str(OUT / 'tmp'))
    phases, tests, metadata, binaries = {}, {}, None, {}
    error, post_errors = None, []

    def run(name, argv, deadline=1200):
        phases[name] = n.run(OUT, name, argv, env=env, deadline=deadline)
        return (OUT / (name + '-stdout')).read_text()

    try:
        raw = run('metadata', [cargo, 'metadata', '--offline', '--locked', '--manifest-path',
                              str(worker), '--format-version', '1'], 120)
        value = json.loads(raw)
        metadata = h.metadata_check(value, source, n.T, True,
                                    h.tomllib.loads(worker.with_name('Cargo.lock').read_text()))
        package = next(p for p in value['packages'] if p['name'] == 'fe2o3-kfd')
        features = next(p['features'] for p in value['resolve']['nodes'] if p['id'] == package['id'])
        assert 'engineering-gfx950' in features and 'live-validation' not in features
        # The standalone worker already enables the dependency's engineering
        # feature. Cargo rejects CLI feature selection outside that workspace.
        args = [cargo, 'test', *common, '-p', 'fe2o3-kfd', '--lib']
        names = h.inventory(run('runtime-list', [*args, '--', '--list', '--format', 'terse']))
        selected = set()
        for name, selector, count in FILTERS:
            subset = {test for test in names if selector in test}
            assert len(subset) == count and not subset.intersection(selected), (name, len(subset))
            selected.update(subset)
            tests[name] = h.results(run(name, [*args, selector, '--', '--test-threads=2']), subset, [(count, 0, 0)])
        assert len(selected) == 124 and NEW_TESTS <= selected
        args = [cargo, 'test', *common, '--lib', '--test', 'shared_wire']
        names = h.inventory(run('worker-list', [*args, '--', '--list', '--format', 'terse']))
        assert len(names) == 402
        tests['worker'] = h.results(run('worker-tests', [*args, '--', '--test-threads=2']),
                                    names, [(385, 0, 4), (13, 0, 0)])
        raw = run('worker-build', [cargo, 'build', '--profile', 'test', *common,
                                  '--bin', h.CHILD_BIN, '--message-format=json'])
        binaries = h.artifacts(raw, [h.CHILD_BIN], worker, n.T, x.pin)
        assert sum(t['passed'] for t in tests.values()) == 522
        assert sum(t['ignored'] for t in tests.values()) == 4
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        try:
            after = x.snapshot(source)
            n.save(OUT / 'sources-after.json', after)
            assert after == expected, 'source or lockfile changed'
        except BaseException as failure:
            post_errors.append('sources: ' + repr(failure))
        try:
            for pin in pinned:
                assert x.pin(Path(pin['path'])) == pin
            for tool, sha in n.PINS.items():
                assert h.sha(n.N / 'bin' / tool) == sha
        except BaseException as failure:
            post_errors.append('inputs: ' + repr(failure))
        try:
            if metadata is not None:
                for row in metadata['external']:
                    assert h.sha(Path(row['manifest'])) == row['manifest_sha256']
            for row in binaries.values():
                assert x.pin(Path(row['binary']['path'])) == row['binary']
        except BaseException as failure:
            post_errors.append('dependencies or binary: ' + repr(failure))
        try:
            assert n.size(n.T) <= 6 << 30, 'final target cap'
        except BaseException as failure:
            post_errors.append('target: ' + repr(failure))
    passed = error is None and not post_errors
    value = dict(schema='ferric-p228-resident-state-fence-cpu-result-v1', passed=passed,
                 error=error, postcheck_errors=post_errors, inputs=pinned,
                 package_manifest=x.pin(PACKAGE / 'manifest.json'),
                 state_overlay=x.pin(PACKAGE / 'overlay.json'),
                 source_unchanged=not any(e.startswith('sources:') for e in post_errors),
                 metadata=metadata, phases=phases, tests=tests, binaries=binaries,
                 tests_passed=sum(t['passed'] for t in tests.values()),
                 tests_ignored=sum(t['ignored'] for t in tests.values()),
                 empty_initial_target=True, external_cargo_cache_reused=True,
                 gpu_execution=False, numerical_acceptance=False, performance_claim=False,
                 production_authority=False,
                 raw={p.name: x.pin(p) for p in OUT.iterdir() if p.is_file()})
    output = OUT / ('complete.json' if passed else 'failed.json')
    n.save(output, value)
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors,
                         receipt=x.pin(output))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
