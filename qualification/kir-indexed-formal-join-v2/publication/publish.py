"""Publish the actual V2 CPU and retained-join evidence; no tested imports."""
import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import types

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
F = Path('/home/harsh/ferric-p227-integration')
HERE = L / 'proposals/p228-kir-indexed-v2-publication-v1'
OUT = F / 'qualification/kir-indexed-formal-join-v2'
COMMON = F / 'qualification/fe2o3-partial-move-rpo-v1/publication/export.py'
EXPORT_SHA = 'f421bb6c7dc154db6577aed2d9182460456cc9978c82c93554e12eb2328b1aae'
PURE = L / 'kir-indexed-formal-join-v2-root-test-observation-v228-v1.json'
PURE_SHA = '73666c5473fb49439a873a23c6c8d018aca854554ea75e7d37891bdc54cab074'
BASELINE = F / 'qualification/kir-indexed-baseline-control-v1/baseline/complete.json'
PRIOR = F / 'qualification/kir-indexed-formal-join-attempt-v1'
TOOLS = F / 'qualification/fe2o3-partial-move-rpo-finalizer-v1/complete.json'
TEST_PREFIX = 'production_semantic_kir_v1::wave_formal_evidence_v1::tests::indexed_formal_join_'
FINALIZER = 'finite_join_engineering_hsaco_v1'


def exporter():
    path = HERE / 'export.py'; raw = path.read_bytes()
    if path.resolve(strict=True) != path or hashlib.sha256(raw).hexdigest() != EXPORT_SHA:
        raise ValueError('reviewed V2 data exporter identity')
    module = types.ModuleType('indexed_v2_data_export'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module, module.common(COMMON)


def outcomes(X, raw, names, ignored, filtered):
    X.require(names == sorted(set(names)) and ignored == sorted(set(ignored))
        and set(ignored) <= set(names), 'closed named inventory')
    rows = re.findall(r'^test (\S+)(?: - should panic)? \.\.\. (ok|ignored(?:, [^\r\n]*)?)$', raw, re.M)
    X.require(sorted(name for name, _ in rows) == names
        and sorted(name for name, status in rows if status.startswith('ignored')) == ignored
        and re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', raw)
            == [(str(len(names) - len(ignored)), '0', str(len(ignored)), '0', str(filtered))],
        'all actual named results and full summary')
    return dict(names=names, ignored_names=ignored, passed=len(names) - len(ignored), ignored=len(ignored))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('retained_root', type=Path)
    parser.add_argument('archive', type=Path); parser.add_argument('archive_sha256')
    parser.add_argument('complete_sha256'); parser.add_argument('owner_sha256')
    args = parser.parse_args(); Y, X = exporter()
    X.require(X.digest(args.complete_sha256) == Y.INNER[1] and X.digest(args.owner_sha256) == Y.OUTER[1],
        'actual root-observed V2 terminal digests')
    root, archive = args.retained_root.absolute(), args.archive.absolute()
    X.require(root.resolve(strict=True) == root and root.is_dir(), 'canonical retained tree')
    archive_pin = dict(path=str(archive), **X.extent(X.read(archive)))
    X.require(archive_pin['sha256'] == X.digest(args.archive_sha256), 'actual courier archive')
    manifest_raw = X.read(root / 'export-manifest.json'); manifest = X.parse(manifest_raw)
    X.require(manifest['schema'] == 'ferric-p228-kir-indexed-v2-export-v1'
        and manifest['original_root'] == str(Y.E) and manifest['complete_sha256'] == args.complete_sha256
        and manifest['owner_sha256'] == args.owner_sha256 and manifest['exporter']['sha256'] == EXPORT_SHA
        and all(manifest[key] is False for key in ('snapshot_bodies_exported', 'owner_inventory_bodies_exported',
            'artifact_bodies_exported', 'full_source_tree_exported')), 'actual V2 courier scope')
    records, seen, bodies, local_pins = manifest['files'], set(), {}, {}
    X.require(len(records) == 88, 'eighty-eight retained source/metadata bodies')
    for path in root.rglob('*'):
        X.require(not path.is_symlink(), 'no retained symlink')
        if path.is_file(): seen.add(str(path.relative_to(root)))
        else: X.require(path.is_dir(), 'ordinary retained tree')
        X.require(len(seen) <= 89, 'bounded retained file roster')
    X.require(seen == set(records) | {'export-manifest.json'}, 'closed retained tree')
    for name, pin in records.items():
        X.require(X.pin(pin)['path'] == str(Y.E / X.relative(name)), 'original courier path')
        path = root / name; bodies[name] = X.read(path, pin)
        local_pins[str(path)] = dict(path=str(path), **X.extent(bodies[name]))

    def raw(path): return bodies[str(Path(path).relative_to(Y.E))]
    def doc(path): return X.parse(raw(path))
    def record(path): return records[str(Path(path).relative_to(Y.E))]

    value, owner = doc(Y.CASE / 'complete.json'), doc(Y.OWNER / 'complete.json')
    inner_pin, outer_pin = record(Y.CASE / 'complete.json'), record(Y.OWNER / 'complete.json')
    X.require((inner_pin['bytes'], inner_pin['sha256']) == Y.INNER
        and (outer_pin['bytes'], outer_pin['sha256']) == Y.OUTER, 'actual successful V2 receipts')
    X.success(value, 'ferric-p228-kir-indexed-formal-join-cpu-result-v2')
    X.success(owner, 'ferric-p228-kir-indexed-formal-join-owned-result-v2')
    X.require(owner['completion'] == inner_pin and owner['package'] == value['package']
        and owner['proposal'] == value['proposal'] and owner['actual_capture_join_passed'] is True
        and value['staged_join_qualification_completed'] is True and value['actual_capture_join_passed'] is True
        and value['source_unchanged'] is True and value['prior_compiled_library_inventory_available'] is True,
        'actual staged source/test/retained-join qualification')
    false_flags = ('limits_changed', 'fresh_compiler_built', 'fresh_hsaco_emitted', 'full_compiler_cohort_requalified',
        'gpu_execution', 'numerical_acceptance', 'production_authority', 'performance_claim')
    X.require(all(value[key] is False for key in false_flags)
        and all(owner[key] is False for key in ('fresh_hsaco_emitted', 'gpu_execution', 'production_authority')),
        'no new compiler cohort, image, GPU or acceptance claim')
    owned = owner['owned']; identity = doc(Y.OWNER / 'started.json')['parent']
    X.require(type(owned['exit_code']) is int and owned['exit_code'] == 0 and owned['reason'] is None
        and owned['cleanup_signalled'] is False and owned['owned_groups_absent'] is True
        and owned['owned_processes_reaped'] is True and doc(Y.OWNER / 'owned-result.json') == owned
        and identity['uid'] == 9661 and identity['pid'] == identity['pgid'] == identity['sid']
        and any(row.get('reason') == 'spawned-parent' and row.get('identity') == identity for row in owned['lineage']),
        'natural successful owned process tree')
    package_dir = Path(value['package']['path']).parent
    owner_command = doc(Y.OWNER / 'command.json')
    X.require(owner_command['deadline_seconds'] == owned['deadline_seconds'] == 10800
        and owner_command['gpu_execution'] is False and owner_command['argv'] == ['/usr/bin/python3', '-B',
            str(package_dir / 'run.py'), Y.PACKAGE_SHA, Y.SOURCE_SHA, '--child'], 'exact bounded V2 invocation')
    leaves = {phase + '-' + suffix for phase in Y.PHASES for suffix in X.SUFFIXES}
    X.require(set(value['phases']) == Y.PHASES and set(value['raw']) == leaves | Y.SNAPSHOTS,
        'thirteen executed phases and ten snapshots')
    omitted = manifest['remotely_rehashed_omitted_pins']
    expected_omitted = {str((Y.CASE / name).relative_to(Y.E)): value['raw'][name] for name in Y.SNAPSHOTS}
    X.require(set(value['artifacts']) == {'lower-tests', 'finalizer-tests'}, 'two selected test artifacts')
    for pin in value['artifacts'].values():
        X.require(Path(X.pin(pin)['path']).is_relative_to(Y.CASE / 'target'), 'original V2 artifact namespace')
        expected_omitted[str(Path(pin['path']).relative_to(Y.E))] = pin
    for name in Y.OWNER_OMITTED:
        key = str((Y.OWNER / name).relative_to(Y.E))
        X.require(X.pin(omitted[key])['path'] == str(Y.OWNER / name), 'omitted owner inventory path')
        expected_omitted[key] = omitted[key]
    X.require(omitted == expected_omitted and len(omitted) == 14, 'only ten snapshots/two inventories/two ELFs omitted')
    expected_files = {str((Y.CASE / name).relative_to(Y.E)) for name in leaves | {'complete.json'}}
    expected_files |= {str((Y.OWNER / name).relative_to(Y.E)) for name in X.OWNER_FILES - Y.OWNER_OMITTED}
    for phase in sorted(Y.PHASES):
        for suffix in X.SUFFIXES:
            X.require(value['raw'][phase + '-' + suffix] == record(Y.CASE / (phase + '-' + suffix)), 'actual leaf pin')
        cmd, result = doc(Y.CASE / (phase + '-command.json')), doc(Y.CASE / (phase + '-result.json'))
        started = doc(Y.CASE / (phase + '-started.json'))
        X.require(value['phases'][phase] == result and type(result['exit_code']) is int and result['exit_code'] == 0
            and result['reason'] is None and result['group_absent'] is True and cmd['expected_exit'] == 0
            and cmd['affinity'] == [8, 9] and cmd['nice'] == 10 and cmd['cache_cap_bytes'] == 6 << 30
            and cmd['gpu_execution'] is False and started['pid'] == started['pgid'], 'natural bounded successful leaf')
        X.require(all(cmd['env'][key] == '' for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'))
            and cmd['env']['CARGO_TARGET_DIR'] == str(Y.CASE / 'target')
            and 'FE2O3_KIR_JOIN_WORK_DIAGNOSTIC_V1' not in cmd['env'], 'GPU hidden; diagnostic flag absent')
        for stream in ('stdout', 'stderr'):
            X.require(X.extent(raw(Y.CASE / (phase + '-' + stream)))['sha256'] == result[stream + '_sha256'], 'stream digest')
        cargo = cmd['env']['RUSTC'].rsplit('/', 1)[0] + '/cargo'
        common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(Y.CASE / 'source/fe2o3/Cargo.toml')]
        if phase.startswith('rustfmt'):
            tail = ['--edition', '2024', '--config', 'skip_children=true',
                    *[pin['path'] for _, pin in sorted(value['formatted_sources'].items())]]
            X.require(Path(cmd['argv'][0]).name == 'rustfmt'
                and cmd['argv'][1:] == (['--check', *tail] if phase.endswith('check') else tail), 'exact five-file formatter scope')
        elif phase == 'metadata':
            X.require(cmd['argv'] == [cargo, 'metadata', '--offline', '--locked', '--manifest-path', common[-1], '--format-version', '1'], 'metadata recipe')
        elif phase.endswith('build-tests'):
            selection = ['-p', 'fe2o3-lower-mir-kernel', '--lib'] if phase.startswith('lower') else ['-p', 'fe2o3-hsaco-finalize', '--example', FINALIZER]
            X.require(cmd['argv'] == [cargo, 'test', *common, *selection, '--no-run', '--message-format=json'], 'selected Cargo test build')
        else:
            role = 'lower-tests' if phase.startswith('lower') else 'finalizer-tests'
            tail = (['--exact', Y.SELECTOR, '--ignored', '--show-output', '--test-threads=1'] if phase == 'actual-inert-join'
                else ['--ignored', '--list', '--format', 'terse'] if phase.endswith('ignored-list')
                else ['--list', '--format', 'terse'] if phase.endswith('list')
                else [TEST_PREFIX, '--test-threads=2'] if phase == 'lower-indexed-tests' else ['--test-threads=2'])
            X.require(cmd['argv'] == [value['artifacts'][role]['path'], *tail], 'actual test ELF and exact selector')
        deadline = 60 if phase.startswith('rustfmt') else 120 if phase == 'metadata' or phase.endswith('list') else 900 if phase == 'actual-inert-join' else 1800
        X.require(cmd['deadline_seconds'] == deadline, 'original leaf deadline')
    for role, crate, target in (('lower', 'fe2o3-lower-mir-kernel', 'fe2o3_lower_mir_kernel'),
                                ('finalizer', 'fe2o3-hsaco-finalize', FINALIZER)):
        rows = [X.parse(line) for line in raw(Y.CASE / (role + '-build-tests-stdout')).splitlines() if line.strip()]
        selected = [row for row in rows if row.get('reason') == 'compiler-artifact'
            and row.get('target', {}).get('name') == target and row.get('profile', {}).get('test') is True
            and row.get('manifest_path') == str(Y.CASE / 'source/fe2o3/crates' / crate / 'Cargo.toml')]
        X.require(len(selected) == 1 and selected[0]['executable'] == value['artifacts'][role + '-tests']['path']
            and [row.get('success') for row in rows if row.get('reason') == 'build-finished'] == [True], 'actual selected Cargo artifact')
        names = sorted(re.findall(r'^([^\r\n]+): test$', raw(Y.CASE / (role + '-list-stdout')).decode(), re.M))
        ignored = sorted(re.findall(r'^([^\r\n]+): test$', raw(Y.CASE / (role + '-ignored-list-stdout')).decode(), re.M))
        X.require(value['tests'][role] == outcomes(X, raw(Y.CASE / (role + '-tests-stdout')).decode(), names, ignored, 0), 'named default suite')
    lower, finalizer = value['tests']['lower'], value['tests']['finalizer']
    added = sorted(value['added_tests'])
    X.require(lower['passed'] == 785 and lower['ignored'] == 0 and finalizer['passed'] == 190 and finalizer['ignored'] == 15
        and len(added) == len(set(added)) == 20 and set(added) <= set(lower['names'])
        and value['tests']['indexed_subset_repeat'] == outcomes(X, raw(Y.CASE / 'lower-indexed-tests-stdout').decode(), added, [], 765),
        'full suites and separately repeated twenty-name subset')
    X.require(value['lower_inventory'] == dict(names=lower['names'], ignored_names=[], added_names=added,
        prior_compiled_library_inventory_available=True), 'measured full library roster')
    X.require(Y.SELECTOR in finalizer['ignored_names'], 'actual join remains an explicitly separate ignored test')
    join_text = raw(Y.CASE / 'actual-inert-join-stdout').decode()
    outcomes(X, join_text, [Y.SELECTOR], [], 204)
    X.require(value['actual_join'] == dict(test=Y.SELECTOR, passed=1, ignored=0, filtered_out=204,
        exit_code=0, actual_capture_join_passed=True, fresh_hsaco_emitted=False)
        and 'kir-join-work-v1' not in join_text + raw(Y.CASE / 'actual-inert-join-stderr').decode()
        and 'CanonicalKernelIrWorkLimitV1' not in join_text, 'actual retained join, not prior refusal or diagnostic')
    join_env = doc(Y.CASE / 'actual-inert-join-command.json')['env']
    for suffix, word in (('PATH', value['retained_handoff']['path']), ('BYTES', str(value['retained_handoff']['bytes'])), ('SHA256', value['retained_handoff']['sha256'])):
        X.require(join_env['FE2O3_WAVE_QKV_ATTENTION_OUTPUT_TILE_ENGINEERING_V6_' + suffix] == word, 'exact retained capture input')

    package, proposal = doc(value['package']['path']), doc(value['proposal']['path'])
    X.require(value['package']['path'] == str(Y.E / 'p228-kir-indexed-formal-join-cpu-v2/manifest.json')
        and value['package']['sha256'] == Y.PACKAGE_SHA and value['proposal']['sha256'] == Y.SOURCE_SHA
        and value['proposal']['path'] == str(Y.E / 'p228-kir-indexed-formal-join-v2/source-manifest.json')
        and package['schema'] == 'ferric-p228-kir-indexed-formal-join-cpu-package-v2'
        and {row['path']: row['sha256'] for row in package['files']} == Y.CONTROLLER
        and len(package['files']) == 3, 'exact V2 package and source proposal')
    for pin in (value['package'], value['proposal']):
        X.require(record(pin['path']) == pin, 'receipt/package original identity')
        expected_files.add(str(Path(pin['path']).relative_to(Y.E)))
    for row in package['files']:
        path = package_dir / X.relative(row['path'])
        X.require(X.extent(raw(path)) == {key: row[key] for key in ('bytes', 'sha256')}, 'controller body')
        expected_files.add(str(path.relative_to(Y.E)))
    proposal_dir = Path(value['proposal']['path']).parent
    X.require(proposal['schema'] == 'ferric-p228-kir-indexed-formal-join-source-v2'
        and len(proposal['files']) == 5 and sum(row['before'] is None for row in proposal['files']) == 2
        and set(value['formatted_sources']) == {row['path'] for row in proposal['files']}
        and proposal['added_tests'] == value['added_tests'] and proposal['work_limit'] == 1 << 30
        and proposal['storage_limit'] == 128 << 20 and all(proposal[key] is False
            for key in ('changes_live_join', 'changes_canonical_bytes', 'changes_authority')), 'five reviewed sources and unchanged limits')
    for row in proposal['files']:
        X.require(row['source'] == 'draft/' + row['path'], 'draft source mapping')
        draft = proposal_dir / X.relative(row['source']); pin = value['formatted_sources'][row['path']]
        X.require(X.extent(raw(draft)) == row['after'] and pin['path'] == str(Y.CASE / 'source/fe2o3' / X.relative(row['path']))
            and record(pin['path']) == pin, 'draft and actual formatted source identities')
        expected_files.update((str(draft.relative_to(Y.E)), str(Path(pin['path']).relative_to(Y.E))))
    source_readme = proposal_dir / 'README.md'
    X.require(X.extent(raw(source_readme)) == dict(bytes=3294, sha256='35765ba5161fd6a19b53f9d3357af18c7d3767a2ec9f9459ed1f6dd6bd2622bb'), 'immutable proposal-time README')
    expected_files.add(str(source_readme.relative_to(Y.E)))
    X.require(expected_files == set(records), 'closed copied source/metadata roster')
    baseline_raw = X.read(BASELINE, Y.BASELINE); baseline = X.parse(baseline_raw)
    tools_raw = X.read(TOOLS, value['finalizer_tools']); tools = X.parse(tools_raw)
    old_path = PRIOR / 'proposal/source-manifest.json'; old_raw = X.read(old_path); old = X.parse(old_raw)
    X.require(X.extent(old_raw)['sha256'] == '6501750a4ac345cbd47e543954580775eab5b8df406bc302117906b5c1265560'
        and value['original_source_baseline'] == proposal['baseline_control'] == Y.BASELINE
        and value['original_source_baseline_owner'] == Y.BASELINE_OWNER
        and baseline['preexisting_rpo_failure_observed'] is True and baseline['baseline_library_passed'] is False
        and lower['names'] == sorted(baseline['tests']['lower-tests']['names'] + added)
        and finalizer == tools['tests'], 'measured original baseline and preserved finalizer inventory')
    new_rows = {row['path']: row for row in proposal['files']}
    X.require(all(new_rows[row['path']] == row for row in old['files'])
        and set(new_rows) == {row['path'] for row in old['files']} | {Y.CORRECTED}, 'four indexed bodies unchanged from V1')
    correction = new_rows[Y.CORRECTED]; corrected_raw = raw(proposal_dir / correction['source'])
    new = b'    // RPO visits block 8 before 7; both directly reload the moved Round field.\n    expect_scalar_move_ssa_unavailable(request, 8, 7, ROUND);'
    previous = b'    // Keep the original direct Round reload as an exact earlier SSA control.\n    expect_scalar_move_ssa_unavailable(request, 7, 4, ROUND);'
    X.require(corrected_raw.count(new) == 1 and X.extent(corrected_raw.replace(new, previous)) == correction['before']
        and value['corrected_existing_test'] == proposal['changed_existing_test']['name']
        and value['corrected_existing_test'] in lower['names'], 'only measured first-error expectation/comment changed; later rejection retained')
    checks = dict(original_files=5783, candidate_files=5785, overlay_files=5, additions=2,
        source_transition_checked_remotely=True, source_and_dependency_postchecks_equal=True)
    X.require(manifest['remote_source_checks'] == checks, 'remote full source-map comparison, not local replay')
    for first, second in (('sources-before.json', 'sources-after.json'), ('dependencies-before.json', 'dependencies-after.json'),
                          ('configurations-before.json', 'configurations-after.json')):
        X.require(all(value['raw'][first][key] == value['raw'][second][key] for key in ('bytes', 'sha256')), 'before/after snapshot digest equality')
    pure_raw = X.read(PURE); pure = X.parse(pure_raw)
    names = sorted(node.name for cls in ast.parse(raw(package_dir / 'test_run.py').decode()).body if isinstance(cls, ast.ClassDef)
        for node in cls.body if isinstance(node, ast.FunctionDef) and node.name.startswith('test_'))
    log = pure['terminal_tool_result']['output'].replace('\r\n', '\n')
    X.require(X.extent(pure_raw)['sha256'] == PURE_SHA and pure['schema'] == 'ferric-primary-agent-test-observation-v1'
        and pure['remote_supervisor_receipt'] is False and pure['terminal_tool_result']['exit_code'] == pure['tests_failed'] == 0
        and pure['tests_passed'] == len(names) == 20
        and sorted(re.findall(r'^(test_\w+) \(__main__\.IndexedTests\) \.\.\. ok$', log, re.M)) == names
        and re.search(r'Ran 20 tests in [0-9.]+s\n\nOK\n', log)
        and package['test_census'] == ['IndexedTests.' + name for name in names], 'actual primary20 named observation')
    expected_hashes = {str(package_dir / row['path']): row['sha256'] for row in package['files']}
    expected_hashes.update({value['package']['path']: Y.PACKAGE_SHA, value['proposal']['path']: Y.SOURCE_SHA})
    for key in ('prehash_tool_result', 'posthash_tool_result'):
        sample = pure[key]; pairs = re.findall(r'^([0-9a-f]{64})  (.+)$', sample['output'], re.M)
        X.require(sample['exit_code'] == 0 and len(pairs) == 5 and {path: digest for digest, path in pairs} == expected_hashes,
            'actual before/after source observations')

    copies = {}
    for name, pin in records.items():
        path = Path(pin['path'])
        if path.is_relative_to(Y.CASE / 'source/fe2o3'): destination = 'source/' + str(path.relative_to(Y.CASE / 'source/fe2o3'))
        elif path.is_relative_to(Y.CASE): destination = ('cpu/' if path.name == 'complete.json' else 'raw/') + path.name
        elif path.is_relative_to(Y.OWNER): destination = 'owner/' + path.name
        elif path.is_relative_to(package_dir): destination = 'controller/' + path.name
        elif path.is_relative_to(proposal_dir): destination = 'proposal/' + str(path.relative_to(proposal_dir))
        else: raise ValueError('unselected copy source')
        X.require(destination not in copies, 'unique destination'); copies[destination] = (bodies[name], pin)
    for name, path in (('pure/root-observation.json', PURE), ('publication/export.py', HERE / 'export.py'), ('publication/publish.py', Path(__file__).resolve())):
        body = X.read(path); copies[name] = (body, dict(path=str(path), **X.extent(body)))
    copies['publication/export-manifest.json'] = (manifest_raw, dict(path=str(root / 'export-manifest.json'), **X.extent(manifest_raw)))
    X.require(len(copies) == 92, 'closed V2 publication')
    if os.path.lexists(OUT):
        X.require(OUT.resolve(strict=True) == OUT and OUT.is_dir() and {path.name for path in OUT.iterdir()} <= {'README.md'}, 'fresh publication or root README only')
        if (OUT / 'README.md').exists(): X.read(OUT / 'README.md')
    for path, pin in local_pins.items(): X.read(Path(path), pin)
    for path, body in ((PURE, pure_raw), (BASELINE, baseline_raw), (TOOLS, tools_raw), (old_path, old_raw)): X.read(path, X.extent(body))
    X.read(archive, archive_pin)
    ledger = {name: dict(original=pin, **X.extent(body)) for name, (body, pin) in copies.items()}
    result = dict(schema='ferric-p228-kir-indexed-v2-publication-v1', publication_completed=True,
        staged_cpu_qualification_passed=True, actual_capture_join_passed=True, cpu=inner_pin, owner=outer_pin, archive=archive_pin,
        phases=sorted(Y.PHASES), tests={key: {word: row[word] for word in ('passed', 'ignored')} for key, row in value['tests'].items()},
        indexed_subset_is_repeat=True, actual_join=value['actual_join'], corrected_existing_test=value['corrected_existing_test'],
        original_source_baseline=Y.BASELINE, original_source_baseline_owner=Y.BASELINE_OWNER,
        previous_failed_candidate=value['previous_failed_candidate'], compiler_cpu=value['compiler_cpu'], finalizer_tools=value['finalizer_tools'],
        retained_handoff=value['retained_handoff'], original_source_generation=value['source_generation'],
        formatted_sources=value['formatted_sources'], selected_test_artifacts=value['artifacts'],
        work_limit=proposal['work_limit'], storage_limit=proposal['storage_limit'], source_unchanged=True, postcheck_errors=[],
        controller_policy_tests=20, primary_pure_observation=dict(path=str(PURE), **X.extent(pure_raw)), pure_remote_owner_receipt=False,
        owned_exit_code=0, owned_elapsed_seconds=owned['elapsed_seconds'], copied=ledger,
        remotely_rehashed_omitted_pins=omitted, remote_source_checks=checks,
        snapshot_bodies_rehashed_locally=False, owner_inventory_bodies_rehashed_locally=False, artifact_bodies_rehashed_locally=False,
        transitive_prerequisite_replay=False, historical_proposal_status_bytes_preserved=True,
        limits_changed=False, fresh_compiler_built=False, full_compiler_cohort_requalified=False,
        fresh_hsaco_emitted=False, gpu_execution=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False, production_authority=False)
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (body, _) in copies.items():
        path = OUT / name; path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream: stream.write(body)
        X.read(path, X.extent(body))
    result_raw = (json.dumps(result, indent=2, sort_keys=True) + '\n').encode('ascii')
    with (OUT / 'result.json').open('xb') as stream: stream.write(result_raw)
    print(json.dumps(dict(result=dict(path=str(OUT / 'result.json'), **X.extent(result_raw)), copied_files=len(copies),
        actual_capture_join_passed=True, fresh_hsaco_emitted=False)))


if __name__ == '__main__':
    main()
