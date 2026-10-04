"""Fresh paired-source state-bank CPU validation; no GPU or installation."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import types

R = Path('/home/harmenon/ferric-asrock-42')
E = R / 'evidence/finite-resident-integration-v220'
PACKAGE = E / 'p228-state-bank-batch-cpu-v1'
PRIOR = E / 'resident-state-fence-cpu-v228-v2'
PRIOR_SHA = '33fb539a0823dcaa988d9091d7712f40cfb0ed656c87b6e958df98f8e35ccdf9'
BASE = E / 'p228-resident-state-fence-cpu-v2/run.py'
BASE_SHA = 'c16fce5d5e431f893bfccc5f1124ba20cbfca2544e26501e221c3ba452fdbedc'
HELPER = E / 'p228-host-policy-cpu-v1/run.py'
HELPER_SHA = '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494'
RUNTIME_DIR = 'p228-state-bank-batch-runtime-v2'
FERRIC_DIR = 'p228-state-bank-batch-ferric-v2'
RUNTIME_ROOT = 'crates/fe2o3-kfd/src/'
FERRIC_ROOT = 'adapters/tp-peer-finite-engineering-worker-v1/src/state_roster/prefix_tiles_decode_v6/'
RUNTIME_FILES = (
    'lib.rs', 'engineering_gfx950.rs', 'engineering_gfx950_peer.rs',
    'engineering_gfx950_peer_wave_mlp_tiles_state_v2.rs',
    'engineering_gfx950_peer_wave_qkv_attention_output_tiles_state_v6.rs',
    'engineering_gfx950_peer_state_bank_v1.rs',
    'engineering_gfx950_peer_state_bank_v1_tests.rs',
)
FERRIC_FILES = ('reuse.rs', 'tests.rs', 'bank_batch_tests.rs')
BANK_SELECTOR = 'engineering_gfx950::peer::state_bank_v1::tests::'
BANK_TESTS = {BANK_SELECTOR + name for name in (
    'bank_mixed_order_whole_validation_then_reads_and_two_fresh_checks',
    'bank_bounds_one_and144_accept_empty_and145_refuse_before_backend',
    'bank_duplicates_refuse_before_any_load_and_quarantine',
    'bank_first_middle_last_validation_read_and_both_fence_failures_return_no_vector',
    'bank_repeated_calls_take_new_fences_and_later_failure_quarantines',
    'bank_public_bounds_error_poison_and_public_single_fences_remain_guarded',
    'bank_native_routing_accepts_completed_lifetime_but_refuses_missing_owner_storage',
    'bank_native_rejects_unconstructed_lifetimes_without_storage_access',
    'bank_native_prefix_foreign_stale_kind_extent_and_mapping_checks_precede_storage',
    'bank_native_mlp_foreign_stale_kind_extent_and_mapping_checks_precede_storage',
)}
WORKER_SELECTOR = 'state_roster::prefix_tiles_decode_v6::reuse::bank_batch_tests::'
WORKER_TESTS = {WORKER_SELECTOR + name for name in (
    'bank_gather_selects_exact36_layers_and_prefix_then_mlp_rank_order',
    'bank_gather_rejects_wrong_roster_or_bank_before_any_state_access',
    'bank_gather_stops_on_each_failed_entry_without_returning_partial_requests',
    'bank_gather_owner_refusal_keeps_last_rank_from_entering_runtime_batch',
    'bank_decode_retains_every_word_in_both_typed_rank_arrays',
    'bank_decode_refuses_missing_extra_and_every_wrong_typed_slot',
    'bank_ledger_uses_two_snapshots_and_preserves_outer_fences_and144_rearms',
    'bank_ledger_fresh_generation_still_performs_both_scans_without_stores',
    'bank_ledger_each_failed_snapshot_is_terminal_without_publishing_generation',
    'bank_ledger_rejects_missing_or_extra_layers_in_either_scan',
    'bank_ledger_last_state_corruption_never_escapes_either_complete_scan',
)}
MEMORY_FILTERS = (
    ('memory-prefix', 'memory_linux::wave_qkv_attention_output_tiles_v6::tests::', 4),
    ('memory-mlp', 'memory_linux::wave_mlp_tiles_v2::tests::', 4),
    ('memory-rearm', 'memory_linux::state_rearm_tests::', 2),
)


def optimization_guard(optimized, environment):
    if optimized or 'PYTHONOPTIMIZE' in environment:
        raise RuntimeError('optimized Python is forbidden before authenticated helpers load')


def shape_pin(value):
    assert isinstance(value, dict) and set(value) == {'path', 'bytes', 'sha256'}
    assert isinstance(value['path'], str) and Path(value['path']).is_absolute()
    assert type(value['bytes']) is int and value['bytes'] >= 0
    assert isinstance(value['sha256'], str) and re.fullmatch(r'[0-9a-f]{64}', value['sha256'])
    return value


def content_pin(value):
    assert isinstance(value, dict) and set(value) == {'bytes', 'sha256'}
    shape_pin(dict(path='/placeholder', **value))
    return value


def package_inputs(expected_sha):
    assert re.fullmatch(r'[0-9a-f]{64}', expected_sha)
    manifest = PACKAGE / 'manifest.json'
    assert manifest.resolve(strict=True) == manifest and not manifest.is_symlink()
    assert hashlib.sha256(manifest.read_bytes()).hexdigest() == expected_sha
    value = json.loads(manifest.read_bytes())
    assert value['schema'] == 'ferric-p228-state-bank-batch-cpu-package-v1'
    assert len(value['files']) == 4
    assert {row['path'] for row in value['files']} == {'run.py', 'overlay.json', 'test_run.py', 'README.md'}
    assert Path(__file__).resolve(strict=True) == PACKAGE / 'run.py'
    for row in value['files']:
        assert set(row) == {'path', 'bytes', 'sha256'}
        path = PACKAGE / row['path']
        assert path.resolve(strict=True) == path and not path.is_symlink()
        data = path.read_bytes()
        assert len(data) == row['bytes'] and hashlib.sha256(data).hexdigest() == row['sha256']
    return [manifest, *(PACKAGE / row['path'] for row in value['files'])]


def overlay_shape(value):
    assert set(value) == {'schema', 'runtime_preimages', 'ferric_source_manifest', 'files'}
    assert value['schema'] == 'ferric-p228-state-bank-batch-cpu-overlay-v1'
    assert shape_pin(value['runtime_preimages'])['path'] == str(E / RUNTIME_DIR / 'preimages.json')
    assert shape_pin(value['ferric_source_manifest'])['path'] == str(E / FERRIC_DIR / 'source-pins.json')
    destinations = {('fe2o3', RUNTIME_ROOT + name): RUNTIME_DIR + '/source/' + name
                    for name in RUNTIME_FILES}
    destinations.update({('ferric', FERRIC_ROOT + name): FERRIC_DIR + '/draft/' + FERRIC_ROOT + name
                         for name in FERRIC_FILES})
    assert len(value['files']) == 10
    seen = set()
    for row in value['files']:
        assert set(row) == {'project', 'path', 'source', 'before', 'after'}
        key = (row['project'], row['path'])
        assert key in destinations and key not in seen
        seen.add(key)
        assert row['source'] == destinations[key]
        if row['before'] is not None:
            content_pin(row['before'])
        content_pin(row['after'])
    expected_new = {('fe2o3', RUNTIME_ROOT + name) for name in RUNTIME_FILES[-2:]}
    expected_new.add(('ferric', FERRIC_ROOT + 'bank_batch_tests.rs'))
    assert {(r['project'], r['path']) for r in value['files'] if r['before'] is None} == expected_new
    return value


def exact_extension(actual, prior, added):
    assert not prior.intersection(added), 'new tests already present in prior generation'
    assert actual == prior | added, ('compiled test inventory changed', sorted(actual ^ (prior | added)))


def ignored_names(raw):
    return {name for name in re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. ignored(?:, [^\n]*)?$', raw, re.MULTILINE)}


def recipes(cargo, worker, filters):
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(worker)]
    runtime = [cargo, 'test', *common, '-p', 'fe2o3-kfd', '--lib']
    child = [cargo, 'test', *common, '--lib', '--test', 'shared_wire']
    rows = [('metadata', [cargo, 'metadata', '--offline', '--locked', '--manifest-path',
                          str(worker), '--format-version', '1'], 120),
            ('runtime-list', [*runtime, '--', '--list', '--format', 'terse'], 1200)]
    rows.extend((name, [*runtime, selector, '--', '--test-threads=2'], 1200)
                for name, selector, _ in filters)
    rows.extend((
        ('worker-list', [*child, '--', '--list', '--format', 'terse'], 1200),
        ('worker-tests', [*child, '--', '--test-threads=2'], 1200),
        ('worker-build', [cargo, 'build', '--profile', 'test', *common,
                          '--bin', 'ferric-tp-peer-finite-engineering-worker-v1', '--message-format=json'], 1200)))
    return rows


def environment(n, output):
    return dict(n.environment(), CARGO_PROFILE_TEST_OPT_LEVEL='2',
                CARGO_PROFILE_DEV_OPT_LEVEL='2', TMPDIR=str(output / 'tmp'))


def authenticate_prior(h, x, n, base, pinned):
    complete = PRIOR / 'complete.json'
    assert h.sha(complete) == PRIOR_SHA and complete.stat().st_size == 120723
    pinned.append(x.pin(complete))
    prior = json.loads(complete.read_bytes())
    assert prior['schema'] == 'ferric-p228-resident-state-fence-cpu-result-v1'
    assert prior['passed'] is True and prior['error'] is None and prior['postcheck_errors'] == []
    assert prior['source_unchanged'] is True
    assert prior['tests_passed'] == 522 and prior['tests_ignored'] == 4
    for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority'):
        assert prior[key] is False
    assert len(prior['inputs']) == 37 and len(prior['raw']) == 88
    for pin in [*prior['inputs'], *prior['raw'].values()]:
        assert x.pin(Path(shape_pin(pin)['path'])) == pin
        pinned.append(pin)
    old_source = PRIOR / 'sources'
    n.F, n.T = old_source / 'fe2o3', PRIOR / 'target'
    env = environment(n, PRIOR)
    commands = recipes(str(n.N / 'bin/cargo'), old_source / 'ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml', base.FILTERS)
    assert len(commands) == 17 and set(prior['phases']) == {name for name, _, _ in commands}
    assert set(prior['raw']) == {'sources-base.json', 'sources-before.json', 'sources-after.json'} | {
        name + suffix for name, _, _ in commands for suffix in ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')}
    for key, pin in prior['raw'].items():
        assert pin['path'] == str(PRIOR / key)
    for name, argv, deadline in commands:
        command = json.loads((PRIOR / (name + '-command.json')).read_bytes())
        assert command == dict(argv=argv, env=env, tools=n.PINS, deadline_seconds=deadline,
            cache_cap_bytes=6 << 30, affinity=[8, 9], nice=10, gpu_execution=False, expected_exit=0)
        started = json.loads((PRIOR / (name + '-started.json')).read_bytes())
        assert set(started) == {'pid', 'pgid'} and type(started['pid']) is int and started['pid'] > 0
        assert started['pid'] == started['pgid']
        result = json.loads((PRIOR / (name + '-result.json')).read_bytes())
        assert result == prior['phases'][name]
        assert result['exit_code'] == 0 and result['reason'] is None and result['group_absent'] is True
        assert result['cache_bytes'] <= 6 << 30
        assert result['stdout_sha256'] == prior['raw'][name + '-stdout']['sha256']
        assert result['stderr_sha256'] == prior['raw'][name + '-stderr']['sha256']
    runtime = h.inventory((PRIOR / 'runtime-list-stdout').read_text())
    selected = set()
    assert set(prior['tests']) == {name for name, _, _ in base.FILTERS} | {'worker'}
    for name, selector, count in base.FILTERS:
        subset = {test for test in runtime if selector in test}
        assert len(subset) == count and not subset.intersection(selected)
        selected.update(subset)
        found = h.results((PRIOR / (name + '-stdout')).read_text(), subset, [(count, 0, 0)])
        assert json.loads(json.dumps(found)) == prior['tests'][name]
    assert len(selected) == 124 and base.NEW_TESTS <= selected
    worker = h.inventory((PRIOR / 'worker-list-stdout').read_text())
    assert len(worker) == 402
    worker_raw = (PRIOR / 'worker-tests-stdout').read_text()
    found = h.results(worker_raw, worker, [(385, 0, 4), (13, 0, 0)])
    assert json.loads(json.dumps(found)) == prior['tests']['worker']
    artifact = prior['binaries'][h.CHILD_BIN]['binary']
    assert x.pin(Path(artifact['path'])) == artifact
    pinned.append(artifact)
    source = json.loads((PRIOR / 'sources-before.json').read_bytes())
    assert len(source) == 6928 and source == json.loads((PRIOR / 'sources-after.json').read_bytes())
    return prior, source, runtime, worker, ignored_names(worker_raw)


def apply_overlay(source, expected, project, rows, read_source, x):
    for row in rows:
        key = project + '/' + row['path']
        assert expected.get(key) == row['before'], ('overlay preimage', key)
        target = source / key
        if row['before'] is None:
            assert not target.exists() and not target.is_symlink()
        else:
            actual = x.pin(target)
            assert {k: actual[k] for k in ('bytes', 'sha256')} == row['before']
    for row in rows:
        raw = read_source(row)
        assert len(raw) == row['after']['bytes'] and hashlib.sha256(raw).hexdigest() == row['after']['sha256']
        target = source / project / row['path']
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb' if row['before'] is None else 'wb') as stream:
            stream.write(raw)
        expected[project + '/' + row['path']] = row['after']
    assert x.snapshot(source) == expected


def main():
    optimization_guard(sys.flags.optimize, os.environ)
    assert len(sys.argv) == 3
    manifest_sha, label = sys.argv[1:]
    assert re.fullmatch(r'state-bank-batch-cpu-v228-v[1-9][0-9]*', label)
    package_paths = package_inputs(manifest_sha)
    assert HELPER.resolve(strict=True) == HELPER and hashlib.sha256(HELPER.read_bytes()).hexdigest() == HELPER_SHA
    h = types.ModuleType('tested_policy_helpers')
    h.__file__ = str(HELPER)
    exec(compile(HELPER.read_bytes(), str(HELPER), 'exec'), h.__dict__)
    x = h.load(h.EXTRACTOR, h.EXTRACTOR_SHA, 'archive_helpers')
    n = h.load(h.BOUNDS, h.BOUNDS_SHA, 'owned_cpu_leaves')
    base = h.load(BASE, BASE_SHA, 'prior_state_cpu')
    pinned = [x.pin(path) for path in [*package_paths, HELPER, BASE, h.SOURCES, h.EXTRACTOR, h.BOUNDS]]
    assert h.sha(h.SOURCES) == h.SOURCES_SHA
    inputs = json.loads(h.SOURCES.read_bytes())
    assert inputs['archives']['fe2o3']['commit'] == '6964f6129c4c42d343117f61cdbcfe6166392535'
    prior, prior_source, prior_runtime, prior_worker, prior_ignored = authenticate_prior(h, x, n, base, pinned)
    overlay = overlay_shape(json.loads((PACKAGE / 'overlay.json').read_bytes()))
    for key in ('runtime_preimages', 'ferric_source_manifest'):
        pin = overlay[key]
        assert x.pin(Path(pin['path'])) == pin
        pinned.append(pin)
    preimages = json.loads(Path(overlay['runtime_preimages']['path']).read_bytes())
    assert preimages['schema'] == 'ferric-p228-state-bank-batch-source-preimages-v1'
    assert preimages['base_commit'] == '36734d3bc28ed038019d9a16da01c8533cdd83c9'
    runtime_rows = [row for row in overlay['files'] if row['project'] == 'fe2o3']
    assert {row['path']: row['before']['sha256'] for row in runtime_rows if row['before']} == preimages['files']
    assert sorted(row['path'] for row in runtime_rows if row['before'] is None) == sorted(preimages['new_files'])
    ferric_manifest = json.loads(Path(overlay['ferric_source_manifest']['path']).read_bytes())
    assert ferric_manifest['schema'] == 'ferric-p228-state-bank-batch-source-proposal-v1'
    assert ferric_manifest['tests_executed'] is False and ferric_manifest['new_tests_authored'] == 11
    ferric_rows = [row for row in overlay['files'] if row['project'] == 'ferric']
    assert {r['path']: {k: r[k] for k in ('before', 'after')} for r in ferric_rows} == {
        r['path']: {k: r[k] for k in ('before', 'after')} for r in ferric_manifest['files']}
    for row in overlay['files']:
        path = E / row['source']
        assert path.resolve(strict=True) == path
        pin = x.pin(path)
        assert {k: pin[k] for k in ('bytes', 'sha256')} == row['after']
        pinned.append(pin)
    # Reconstruct the original archives and three historical overlays, never
    # requiring the retired old compiled source tree or changing its record.
    old_overlays = []
    for project, directory, sha, count in (
        ('ferric', E / 'p228-host-policy-v1', h.OVERLAY_SHA, 13),
        ('fe2o3', base.OVERLAY, base.OVERLAY_SHA, 3),
    ):
        assert h.sha(directory / 'manifest.json') == sha
        value = json.loads((directory / 'manifest.json').read_bytes())
        assert value['baseline_head'] == inputs['archives'][project]['commit'] and len(value['files']) == count
        old_overlays.append((project, directory / 'draft', value['files']))
    state = json.loads(Path(prior['state_overlay']['path']).read_bytes())
    assert state['schema'] == 'ferric-p228-resident-state-fence-cpu-overlay-v1' and len(state['files']) == 8
    assert state['source_directory'] == 'p228-resident-state-fence-consolidation-v2'
    old_overlays.append(('fe2o3', E / state['source_directory'] / 'source', state['files']))
    OUT = E / label
    OUT.mkdir(mode=0o700)
    source = OUT / 'sources'
    source.mkdir()
    n.F, n.T = source / 'fe2o3', OUT / 'target'
    n.setup()
    assert not any(n.T.iterdir())
    (OUT / 'tmp').mkdir()
    for name, row in inputs['archives'].items():
        pin = x.pin(Path(row['path']))
        assert pin == {k: row[k] for k in ('path', 'bytes', 'sha256')}
        pinned.append(pin)
        x.extract(Path(row['path']), name, source)
    expected = x.snapshot(source)
    assert expected == json.loads((PRIOR / 'sources-base.json').read_bytes())
    n.save(OUT / 'sources-base.json', expected)
    for project, directory, rows in old_overlays:
        apply_overlay(source, expected, project, rows, lambda row, directory=directory: (directory / row['path']).read_bytes(), x)
    assert expected == prior_source and x.snapshot(source) == prior_source
    n.save(OUT / 'sources-prior.json', expected)
    for project in ('fe2o3', 'ferric'):
        rows = [row for row in overlay['files'] if row['project'] == project]
        apply_overlay(source, expected, project, rows, lambda row: (E / row['source']).read_bytes(), x)
    assert len(expected) == 6931
    n.save(OUT / 'sources-before.json', expected)
    worker = source / 'ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml'
    filters = (*base.FILTERS, ('state-bank', BANK_SELECTOR, 10), *MEMORY_FILTERS)
    commands = recipes(str(n.N / 'bin/cargo'), worker, filters)
    assert len(commands) == 21 and len({name for name, _, _ in commands}) == 21
    command_map = {name: (argv, deadline) for name, argv, deadline in commands}
    env = environment(n, OUT)
    phases, tests, metadata, binaries = {}, {}, None, {}
    error, post_errors = None, []

    def run(name):
        argv, deadline = command_map[name]
        phases[name] = n.run(OUT, name, argv, env=env, deadline=deadline)
        return (OUT / (name + '-stdout')).read_text()

    try:
        value = json.loads(run('metadata'))
        metadata = h.metadata_check(value, source, n.T, True, h.tomllib.loads(worker.with_name('Cargo.lock').read_text()))
        package = next(p for p in value['packages'] if p['name'] == 'fe2o3-kfd')
        features = next(p['features'] for p in value['resolve']['nodes'] if p['id'] == package['id'])
        assert 'engineering-gfx950' in features and 'live-validation' not in features
        names = h.inventory(run('runtime-list'))
        exact_extension(names, prior_runtime, BANK_TESTS)
        selected = set()
        for name, selector, count in filters:
            subset = {test for test in names if selector in test}
            assert len(subset) == count and not subset.intersection(selected), (name, len(subset))
            if name == 'state-bank':
                assert subset == BANK_TESTS
            else:
                assert subset == {test for test in prior_runtime if selector in test}
            selected.update(subset)
            tests[name] = h.results(run(name), subset, [(count, 0, 0)])
        assert len(selected) == 144 and base.NEW_TESTS | BANK_TESTS <= selected
        names = h.inventory(run('worker-list'))
        exact_extension(names, prior_worker, WORKER_TESTS)
        assert len(names) == 413
        raw = run('worker-tests')
        tests['worker'] = h.results(raw, names, [(396, 0, 4), (13, 0, 0)])
        assert ignored_names(raw) == prior_ignored and not WORKER_TESTS.intersection(prior_ignored)
        binaries = h.artifacts(run('worker-build'), [h.CHILD_BIN], worker, n.T, x.pin)
        assert sum(t['passed'] for t in tests.values()) == 553
        assert sum(t['ignored'] for t in tests.values()) == 4
        assert set(phases) == set(command_map)
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
    value = dict(schema='ferric-p228-state-bank-batch-cpu-result-v1', passed=passed,
        error=error, postcheck_errors=post_errors, inputs=pinned,
        package_manifest=x.pin(PACKAGE / 'manifest.json'), overlay=x.pin(PACKAGE / 'overlay.json'),
        prior_cpu_complete=x.pin(PRIOR / 'complete.json'),
        source_unchanged=not any(e.startswith('sources:') for e in post_errors),
        metadata=metadata, phases=phases, tests=tests, binaries=binaries,
        tests_passed=sum(t['passed'] for t in tests.values()), tests_ignored=sum(t['ignored'] for t in tests.values()),
        empty_initial_target=True, external_cargo_cache_reused=True,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False, production_authority=False,
        raw={p.name: x.pin(p) for p in OUT.iterdir() if p.is_file()})
    output = OUT / ('complete.json' if passed else 'failed.json')
    n.save(output, value)
    print(json.dumps(dict(passed=passed, error=error, postcheck_errors=post_errors, receipt=x.pin(output))), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
