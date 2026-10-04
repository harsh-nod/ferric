"""Publish the actual negative baseline control without importing tested controllers."""
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
HERE = L / 'proposals/p228-kir-indexed-baseline-publication-v1'
OUT = F / 'qualification/kir-indexed-baseline-control-v1'
COMMON = F / 'qualification/fe2o3-partial-move-rpo-v1/publication/export.py'
EXPORT_SHA = '0c170c1ca9f1d39a2fe4cfa8bbbbed0728890af79ceaec46f7d10f545b0b3558'
PURE = L / 'kir-indexed-baseline-root-test-observation-v228-v1.json'
PURE_SHA = '68998fe059f1a20c228a704e82f61eae92b06034334f57ccd09094a360f3b47f'
PRIOR = F / 'qualification/kir-indexed-formal-join-attempt-v1'
SOURCE_SHA = '6501750a4ac345cbd47e543954580775eab5b8df406bc302117906b5c1265560'


def exporter():
    path = HERE / 'export.py'
    raw = path.read_bytes()
    if path.resolve(strict=True) != path or hashlib.sha256(raw).hexdigest() != EXPORT_SHA:
        raise ValueError('reviewed data exporter identity')
    module = types.ModuleType('indexed_baseline_data_export')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module, module.common(COMMON)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('retained_root', type=Path)
    parser.add_argument('archive', type=Path); parser.add_argument('archive_sha256')
    parser.add_argument('inner_sha256'); parser.add_argument('owner_sha256')
    args = parser.parse_args(); Y, X = exporter()
    X.require(X.digest(args.inner_sha256) == Y.INNER[1] and X.digest(args.owner_sha256) == Y.OUTER[1],
        'actual root-observed baseline terminal digests')
    root, archive = args.retained_root.absolute(), args.archive.absolute()
    X.require(root.resolve(strict=True) == root and root.is_dir(), 'canonical retained tree')
    archive_pin = dict(path=str(archive), **X.extent(X.read(archive)))
    X.require(archive_pin['sha256'] == X.digest(args.archive_sha256), 'root-authenticated actual archive')
    manifest_raw = X.read(root / 'export-manifest.json'); manifest = X.parse(manifest_raw)
    X.require(manifest['schema'] == 'ferric-p228-kir-indexed-baseline-export-v1'
        and manifest['original_root'] == str(Y.E) and manifest['inner_sha256'] == args.inner_sha256
        and manifest['owner_sha256'] == args.owner_sha256 and manifest['exporter']['sha256'] == EXPORT_SHA
        and all(manifest[key] is False for key in ('snapshot_bodies_exported', 'owner_inventory_bodies_exported',
            'artifact_bodies_exported', 'full_source_tree_exported')), 'exact baseline courier identity and scope')
    records, seen, bodies, local_pins = manifest['files'], set(), {}, {}
    X.require(len(records) == 41, 'forty-one retained baseline metadata bodies')
    for path in root.rglob('*'):
        X.require(not path.is_symlink(), 'no retained symlink')
        if path.is_file(): seen.add(str(path.relative_to(root)))
        else: X.require(path.is_dir(), 'ordinary retained tree')
        X.require(len(seen) <= 42, 'bounded retained roster')
    X.require(seen == set(records) | {'export-manifest.json'}, 'closed retained body roster')
    for name, pin in records.items():
        X.require(X.pin(pin)['path'] == str(Y.E / X.relative(name)), 'original courier identity')
        path = root / name; bodies[name] = X.read(path, pin)
        local_pins[str(path)] = dict(path=str(path), **X.extent(bodies[name]))

    def raw(path):
        return bodies[str(Path(path).relative_to(Y.E))]

    def doc(path):
        return X.parse(raw(path))

    value, owner = doc(Y.CASE / 'complete.json'), doc(Y.OWNER / 'complete.json')
    inner_pin = records[str((Y.CASE / 'complete.json').relative_to(Y.E))]
    outer_pin = records[str((Y.OWNER / 'complete.json').relative_to(Y.E))]
    X.require((inner_pin['bytes'], inner_pin['sha256']) == Y.INNER
        and (outer_pin['bytes'], outer_pin['sha256']) == Y.OUTER, 'exact actual completed observer receipts')
    Y.terminal(X, value, owner, inner_pin)
    outcome = owner['owned']; identity = doc(Y.OWNER / 'started.json')['parent']
    X.require(doc(Y.OWNER / 'owned-result.json') == outcome and identity['uid'] == 9661
        and identity['pid'] == identity['pgid'] == identity['sid']
        and any(row.get('reason') == 'spawned-parent' and row.get('identity') == identity
                for row in outcome['lineage']), 'owned result and actual spawned parent identity')
    command = doc(Y.OWNER / 'command.json')
    X.require(command['deadline_seconds'] == outcome['deadline_seconds'] == 10800
        and command['gpu_execution'] is False and command['argv'] == ['/usr/bin/python3', '-B',
            str(Y.PACKAGE / 'run.py'), Y.PACKAGE_SHA, '--child'], 'original bounded baseline observer command')
    leaves = {phase + '-' + suffix for phase in Y.PHASES for suffix in X.SUFFIXES}
    X.require(set(value['raw']) == leaves | Y.SNAPSHOTS, 'six phases and nine snapshot pins')
    omitted = manifest['remotely_rehashed_omitted_pins']
    expected_omitted = {str((Y.CASE / name).relative_to(Y.E)): value['raw'][name] for name in Y.SNAPSHOTS}
    X.require(set(value['artifacts']) == {'lower-tests'}, 'only selected original lower-library ELF')
    binary = X.pin(value['artifacts']['lower-tests'])
    X.require(Path(binary['path']).is_relative_to(Y.CASE / 'target/debug/deps'), 'actual baseline test artifact namespace')
    expected_omitted[str(Path(binary['path']).relative_to(Y.E))] = binary
    for name in Y.OWNER_OMITTED:
        key = str((Y.OWNER / name).relative_to(Y.E))
        X.require(X.pin(omitted[key])['path'] == str(Y.OWNER / name), 'omitted owner inventory namespace')
        expected_omitted[key] = omitted[key]
    X.require(omitted == expected_omitted and len(omitted) == 12,
        'only nine snapshots, two owner inventories and one ELF omitted')
    expected_files = {str((Y.CASE / name).relative_to(Y.E)) for name in leaves | {'complete.json'}}
    expected_files |= {str((Y.OWNER / name).relative_to(Y.E)) for name in X.OWNER_FILES - Y.OWNER_OMITTED}
    for phase in Y.PHASES:
        for suffix in X.SUFFIXES:
            name = phase + '-' + suffix
            X.require(value['raw'][name] == records[str((Y.CASE / name).relative_to(Y.E))], 'exact leaf FilePin')
        cmd, result = doc(Y.CASE / (phase + '-command.json')), doc(Y.CASE / (phase + '-result.json'))
        started = doc(Y.CASE / (phase + '-started.json'))
        X.require(value['phases'][phase] == result and type(result['exit_code']) is int
            and result['exit_code'] == (101 if phase in ('lower-focused-test', 'lower-tests') else 0)
            and result['reason'] is None and result['group_absent'] is True
            and cmd['expected_exit'] == 0 and cmd['affinity'] == [8, 9] and cmd['nice'] == 10
            and cmd['gpu_execution'] is False and started['pid'] == started['pgid']
            and cmd['cache_cap_bytes'] == 6 << 30, 'bounded naturally completed leaves; test failures remain failures')
        X.require(all(cmd['env'][key] == '' for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'))
            and cmd['env']['CARGO_TARGET_DIR'] == str(Y.CASE / 'target')
            and 'FE2O3_KIR_JOIN_WORK_DIAGNOSTIC_V1' not in cmd['env'], 'GPU hidden and diagnostic instrumentation absent')
        for stream in ('stdout', 'stderr'):
            X.require(X.extent(raw(Y.CASE / (phase + '-' + stream)))['sha256'] == result[stream + '_sha256'], 'leaf stream hash')
        tail = {'lower-list': ['--list', '--format', 'terse'],
                'lower-ignored-list': ['--ignored', '--list', '--format', 'terse'],
                'lower-focused-test': ['--exact', Y.FAILURE, '--show-output', '--test-threads=1'],
                'lower-tests': ['--test-threads=2']}.get(phase)
        if tail is not None:
            X.require(cmd['argv'] == [binary['path'], *tail], 'exact original ELF and test selection')
        else:
            cargo = cmd['env']['RUSTC'].rsplit('/', 1)[0] + '/cargo'
            manifest_path = str(Y.CASE / 'source/fe2o3/Cargo.toml')
            argv = ([cargo, 'metadata', '--offline', '--locked', '--manifest-path', manifest_path, '--format-version', '1']
                if phase == 'metadata' else [cargo, 'test', '--offline', '--locked', '--jobs', '2',
                    '--manifest-path', manifest_path, '-p', 'fe2o3-lower-mir-kernel', '--lib', '--no-run', '--message-format=json'])
            X.require(cmd['argv'] == argv, 'exact metadata or library build recipe')
        X.require(cmd['deadline_seconds'] == (120 if phase in ('metadata', 'lower-list', 'lower-ignored-list') else 1800),
            'original leaf deadline')
    build = [X.parse(line) for line in raw(Y.CASE / 'lower-build-tests-stdout').splitlines() if line.strip()]
    selected = [row for row in build if row.get('reason') == 'compiler-artifact'
        and row.get('target', {}).get('name') == 'fe2o3_lower_mir_kernel' and row.get('profile', {}).get('test') is True
        and row.get('manifest_path') == str(Y.CASE / 'source/fe2o3/crates/fe2o3-lower-mir-kernel/Cargo.toml')]
    X.require(len(selected) == 1 and selected[0]['executable'] == binary['path']
        and [row.get('success') for row in build if row.get('reason') == 'build-finished'] == [True],
        'actual successfully built test artifact, body not copied')
    tests = Y.observed_tests(X, value, *(raw(Y.CASE / (name + '-stdout')).decode()
        for name in ('lower-list', 'lower-ignored-list', 'lower-focused-test', 'lower-tests')))
    X.require(tests == manifest['observed_tests'], 'remote/local actual transcript replays agree')
    prior_raw = X.read(PRIOR / 'attempt/failed.json', Y.FAILURE_PIN); prior = X.parse(prior_raw)
    proposal_path = PRIOR / 'proposal/source-manifest.json'
    proposal_raw = X.read(proposal_path); proposal = X.parse(proposal_raw)
    X.require(X.extent(proposal_raw)['sha256'] == SOURCE_SHA
        and prior['lower_inventory']['names'] == value['candidate_observation']['names']
        and prior['lower_inventory']['added_names'] == sorted(proposal['added_tests'])
        and len(proposal['added_tests']) == len(set(proposal['added_tests'])) == 20
        and tests['full_names'] == sorted(set(prior['lower_inventory']['names']) - set(proposal['added_tests'])),
        'published candidate785 minus exact twenty indexed additions equals baseline765')
    package = doc(value['package']['path'])
    X.require(value['package']['path'] == str(Y.PACKAGE / 'manifest.json')
        and value['package']['sha256'] == Y.PACKAGE_SHA
        and records[str((Y.PACKAGE / 'manifest.json').relative_to(Y.E))] == value['package']
        and package['schema'] == 'ferric-p228-kir-indexed-baseline-cpu-package-v1'
        and len(package['files']) == 3 and {row['path']: row['sha256'] for row in package['files']} == Y.CONTROLLER,
        'exact baseline observer source package')
    expected_files.add(str((Y.PACKAGE / 'manifest.json').relative_to(Y.E)))
    for row in package['files']:
        path = Y.PACKAGE / X.relative(row['path'])
        X.require(X.extent(raw(path)) == {key: row[key] for key in ('bytes', 'sha256')}, 'controller source body')
        expected_files.add(str(path.relative_to(Y.E)))
    X.require(expected_files == set(records), 'no unrelated file admitted to publication')
    source_checks = dict(original_files=5783, baseline_files=5783, overlay_files=0, formatted_files=0,
        source_transition_checked_remotely=True, source_and_dependency_postchecks_equal=True)
    X.require(manifest['remote_source_checks'] == source_checks, 'remote full-map comparison, not local body replay')
    for before, after in (('sources-before.json', 'sources-after.json'),
                          ('configurations-before.json', 'configurations-after.json'),
                          ('dependencies-before.json', 'dependencies-after.json')):
        X.require(all(value['raw'][before][key] == value['raw'][after][key] for key in ('bytes', 'sha256')),
            'retained before/after snapshot content identities')
    X.require(all(value['raw']['original-after.json'][key] == Y.SOURCE_PIN[key] for key in ('bytes', 'sha256')),
        'unchanged original source generation')

    pure_raw = X.read(PURE); pure = X.parse(pure_raw)
    names = sorted(cls.name + '.' + node.name for cls in ast.parse(raw(Y.PACKAGE / 'test_run.py').decode()).body
        if isinstance(cls, ast.ClassDef) for node in cls.body
        if isinstance(node, ast.FunctionDef) and node.name.startswith('test_'))
    X.require(X.extent(pure_raw)['sha256'] == PURE_SHA
        and pure['schema'] == 'ferric-primary-agent-test-observation-v1'
        and pure['remote_supervisor_receipt'] is False and pure['exit_code'] == 0
        and pure['tests_passed'] == len(names) == 16 and pure['tests_failed'] == 0
        and pure['pre_and_post_hashes_equal'] is True
        and pure['package_files'] == dict(Y.CONTROLLER, **{'manifest.json': Y.PACKAGE_SHA})
        and package['test_census'] == names
        and re.fullmatch(r'Ran 16 tests in [0-9.]+s\n\nOK\n', pure['terminal_summary'])
        and pure['actual_baseline_run'] is pure['candidate_qualified'] is pure['gpu_execution'] is False,
        'primary16 summary/source observations, not full transcript or remote owner receipt')

    copies = {}
    for name, pin in records.items():
        path = Path(pin['path'])
        if path.is_relative_to(Y.CASE): destination = ('baseline/' if path.name == 'complete.json' else 'raw/') + path.name
        elif path.is_relative_to(Y.OWNER): destination = 'owner/' + path.name
        elif path.is_relative_to(Y.PACKAGE): destination = 'controller/' + path.name
        else: raise ValueError('unselected copy source')
        X.require(destination not in copies, 'unique copy destination')
        copies[destination] = (bodies[name], pin)
    for name, path in (('pure/root-observation.json', PURE), ('publication/export.py', HERE / 'export.py'),
                       ('publication/publish.py', Path(__file__).resolve())):
        body = X.read(path); copies[name] = (body, dict(path=str(path), **X.extent(body)))
    copies['publication/export-manifest.json'] = (manifest_raw, dict(path=str(root / 'export-manifest.json'), **X.extent(manifest_raw)))
    X.require(len(copies) == 45, 'closed small baseline publication')
    if os.path.lexists(OUT):
        X.require(OUT.resolve(strict=True) == OUT and OUT.is_dir()
            and {path.name for path in OUT.iterdir()} <= {'README.md'}, 'fresh publication or root README only')
        if (OUT / 'README.md').exists(): X.read(OUT / 'README.md')
    for path, pin in local_pins.items(): X.read(Path(path), pin)
    for path, body in ((PURE, pure_raw), (PRIOR / 'attempt/failed.json', prior_raw), (proposal_path, proposal_raw)):
        X.read(path, X.extent(body))
    X.read(archive, archive_pin)
    ledger = {name: dict(original=pin, **X.extent(body)) for name, (body, pin) in copies.items()}
    result = dict(schema='ferric-p228-kir-indexed-baseline-publication-v1', publication_completed=True,
        observer_completed=True, library_qualified=False, candidate_qualified=False,
        baseline=inner_pin, owner=outer_pin, archive=archive_pin, phases=list(Y.PHASES),
        tests={key: item for key, item in tests.items() if key != 'full_names'},
        primary_pure_observation=dict(path=str(PURE), **X.extent(pure_raw)), controller_policy_tests=16,
        pure_full_transcript_retained=False, pure_remote_owner_receipt=False,
        candidate_failure=Y.FAILURE_PIN, candidate_owner=Y.FAILURE_OWNER_PIN,
        published_candidate_receipt=dict(path=str(PRIOR / 'attempt/failed.json'), **X.extent(prior_raw)),
        indexed_source_proposal=dict(path=str(proposal_path), **X.extent(proposal_raw)),
        compiler_cpu=Y.CPU_PIN, original_source_generation=Y.SOURCE_PIN, selected_test_artifact=binary,
        source_unchanged=True, source_overlay_applied=False, source_formatted=False,
        owned_exit_code=0, owned_elapsed_seconds=outcome['elapsed_seconds'], postcheck_errors=[],
        copied=ledger, remotely_rehashed_omitted_pins=omitted, remote_source_checks=source_checks,
        snapshot_bodies_rehashed_locally=False, owner_inventory_bodies_rehashed_locally=False,
        artifact_body_rehashed_locally=False, transitive_prerequisite_replay=False,
        exact_failure_reproduced_without_indexed_overlay=True, precise_cause_proven=False,
        error_order_soundness_proven=False, second_alias_only_subcase_completed=False,
        actual_inert_join_attempted=False, fresh_compiler_built=False, fresh_hsaco_emitted=False,
        gpu_execution=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False, production_authority=False)
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (body, _) in copies.items():
        path = OUT / name; path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream: stream.write(body)
        X.read(path, X.extent(body))
    result_raw = (json.dumps(result, indent=2, sort_keys=True) + '\n').encode('ascii')
    with (OUT / 'result.json').open('xb') as stream: stream.write(result_raw)
    print(json.dumps(dict(result=dict(path=str(OUT / 'result.json'), **X.extent(result_raw)),
        copied_files=len(copies), observer_completed=True, library_qualified=False)))


if __name__ == '__main__':
    main()
