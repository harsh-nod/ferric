"""Publish a retained failed CPU attempt without importing any tested controller."""
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
HERE = L / 'proposals/p228-kir-indexed-formal-join-failure-publication-v1'
OUT = F / 'qualification/kir-indexed-formal-join-attempt-v1'
COMMON = F / 'qualification/fe2o3-partial-move-rpo-v1/publication/export.py'
EXPORT_SHA = '0e052526a1536a3308fe1156a7076576e07436a89f335ee0dc9bfdc1ff1f4978'
PURE = L / 'kir-indexed-formal-join-root-test-observation-v228-v1.json'
PURE_SHA = '1fdaa67f3c0a394db45d1a8ac0a02e123e624a89bdd75cf05556088ff92ce8de'


def exporter():
    path = HERE / 'export.py'
    raw = path.read_bytes()
    if path.resolve(strict=True) != path or hashlib.sha256(raw).hexdigest() != EXPORT_SHA:
        raise ValueError('reviewed data exporter identity')
    module = types.ModuleType('indexed_failure_data_export')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module, module.common(COMMON)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('retained_root', type=Path)
    parser.add_argument('archive', type=Path); parser.add_argument('archive_sha256')
    args = parser.parse_args()
    Y, X = exporter()
    root, archive = args.retained_root.absolute(), args.archive.absolute()
    X.require(root.resolve(strict=True) == root and root.is_dir(), 'canonical retained tree')
    archive_pin = dict(path=str(archive), **X.extent(X.read(archive)))
    X.require(archive_pin['sha256'] == X.digest(args.archive_sha256), 'root-authenticated actual archive')
    manifest_raw = X.read(root / 'export-manifest.json')
    manifest = X.parse(manifest_raw)
    X.require(manifest['schema'] == 'ferric-p228-kir-indexed-formal-join-failure-export-v1'
        and manifest['original_root'] == str(Y.E)
        and manifest['inner_sha256'] == Y.INNER[1] and manifest['owner_sha256'] == Y.OUTER[1]
        and manifest['exporter']['sha256'] == EXPORT_SHA
        and all(manifest[key] is False for key in ('snapshot_bodies_exported', 'owner_inventory_bodies_exported',
            'artifact_bodies_exported', 'full_source_tree_exported')), 'exact failed-attempt courier scope')
    records, seen, bodies, local_pins = manifest['files'], set(), {}, {}
    X.require(len(records) == 56, '56 retained metadata/source bodies')
    for path in root.rglob('*'):
        X.require(not path.is_symlink(), 'no retained symlink')
        if path.is_file(): seen.add(str(path.relative_to(root)))
        else: X.require(path.is_dir(), 'ordinary retained tree')
        X.require(len(seen) <= 57, 'bounded file roster')
    X.require(seen == set(records) | {'export-manifest.json'}, 'closed retained body roster')
    for name, pin in records.items():
        X.require(X.pin(pin)['path'] == str(Y.E / X.relative(name)), 'original courier path')
        path = root / name
        bodies[name] = X.read(path, pin)
        local_pins[str(path)] = dict(path=str(path), **X.extent(bodies[name]))

    def raw(path):
        return bodies[str(Path(path).relative_to(Y.E))]

    def doc(path):
        return X.parse(raw(path))

    value, owner = doc(Y.CASE / 'failed.json'), doc(Y.OWNER / 'failed.json')
    inner_pin = records[str((Y.CASE / 'failed.json').relative_to(Y.E))]
    outer_pin = records[str((Y.OWNER / 'failed.json').relative_to(Y.E))]
    X.require((inner_pin['bytes'], inner_pin['sha256']) == Y.INNER
        and (outer_pin['bytes'], outer_pin['sha256']) == Y.OUTER, 'actual immutable failed terminals')
    Y.terminal(X, value, owner)
    outcome = owner['owned']
    identity = doc(Y.OWNER / 'started.json')['parent']
    X.require(doc(Y.OWNER / 'owned-result.json') == outcome and identity['uid'] == 9661
        and identity['pid'] == identity['pgid'] == identity['sid']
        and any(row.get('reason') == 'spawned-parent' and row.get('identity') == identity
                for row in outcome['lineage']), 'owned terminal and original parent identity')
    owner_command = doc(Y.OWNER / 'command.json')
    X.require(owner_command['deadline_seconds'] == 10800 and owner_command['gpu_execution'] is False
        and owner_command['argv'] == ['/usr/bin/python3', '-B', str(Path(value['package']['path']).parent / 'run.py'),
                                     Y.PACKAGE_SHA, Y.SOURCE_SHA, '--child'], 'bounded exact controller invocation')
    leaves = {phase + '-' + suffix for phase in Y.PHASES for suffix in X.SUFFIXES}
    X.require(set(value['raw']) == leaves | Y.SNAPSHOTS, 'seven attempted phases and ten snapshot pins')
    omitted = manifest['remotely_rehashed_omitted_pins']
    expected_omitted = {str((Y.CASE / name).relative_to(Y.E)): value['raw'][name] for name in Y.SNAPSHOTS}
    X.require(set(value['artifacts']) == {'lower-tests'}, 'no finalizer test or emission artifact')
    binary = X.pin(value['artifacts']['lower-tests'])
    X.require(Path(binary['path']).is_relative_to(Y.CASE / 'target/debug/deps'), 'actual lower test artifact namespace')
    expected_omitted[str(Path(binary['path']).relative_to(Y.E))] = binary
    for name in Y.OWNER_OMITTED:
        key = str((Y.OWNER / name).relative_to(Y.E))
        X.require(X.pin(omitted[key])['path'] == str(Y.OWNER / name), 'omitted owner inventory namespace')
        expected_omitted[key] = omitted[key]
    X.require(omitted == expected_omitted and len(omitted) == 13, 'only ten snapshots/two inventories/one ELF omitted')
    expected_files = {str((Y.CASE / name).relative_to(Y.E)) for name in leaves | {'failed.json'}}
    expected_files |= {str((Y.OWNER / name).relative_to(Y.E))
        for name in (X.OWNER_FILES - {'complete.json'} - Y.OWNER_OMITTED) | {'failed.json'}}
    for phase in Y.PHASES:
        for suffix in X.SUFFIXES:
            name = phase + '-' + suffix
            X.require(value['raw'][name] == records[str((Y.CASE / name).relative_to(Y.E))], 'exact raw leaf FilePin')
        command, result = doc(Y.CASE / (phase + '-command.json')), doc(Y.CASE / (phase + '-result.json'))
        started = doc(Y.CASE / (phase + '-started.json'))
        X.require(value['phases'][phase] == result and result['exit_code'] == (101 if phase == 'lower-tests' else 0)
            and result['reason'] is None and result['group_absent'] is True
            and command['expected_exit'] == 0 and command['affinity'] == [8, 9] and command['nice'] == 10
            and command['gpu_execution'] is False and started['pid'] == started['pgid'],
            'unexpected natural test failure; unchanged bounded success expectation')
        X.require(all(command['env'][key] == '' for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'))
            and 'FE2O3_KIR_JOIN_WORK_DIAGNOSTIC_V1' not in command['env'], 'GPU hidden; diagnostic instrumentation absent')
        for stream in ('stdout', 'stderr'):
            X.require(X.extent(raw(Y.CASE / (phase + '-' + stream)))['sha256'] == result[stream + '_sha256'], 'stream hash')
        if phase in ('lower-list', 'lower-ignored-list', 'lower-tests'):
            tail = {'lower-list': ['--list', '--format', 'terse'],
                    'lower-ignored-list': ['--ignored', '--list', '--format', 'terse'],
                    'lower-tests': ['--test-threads=2']}[phase]
            X.require(command['argv'] == [binary['path'], *tail], 'same selected ELF and full-suite selection')
    build = [X.parse(line) for line in raw(Y.CASE / 'lower-build-tests-stdout').splitlines() if line.strip()]
    selected = [row for row in build if row.get('reason') == 'compiler-artifact'
        and row.get('target', {}).get('name') == 'fe2o3_lower_mir_kernel' and row.get('profile', {}).get('test') is True
        and row.get('manifest_path') == str(Y.CASE / 'source/fe2o3/crates/fe2o3-lower-mir-kernel/Cargo.toml')]
    X.require(len(selected) == 1 and selected[0]['executable'] == binary['path']
        and [row.get('success') for row in build if row.get('reason') == 'build-finished'] == [True],
        'actual successfully built Cargo test artifact, body not transported')
    tests = Y.observed_tests(X, value, *(raw(Y.CASE / (phase + '-stdout')).decode()
        for phase in ('lower-list', 'lower-ignored-list', 'lower-tests')))
    X.require(tests == manifest['observed_tests'], 'courier and local named transcript replay agree')
    package, proposal = doc(value['package']['path']), doc(value['proposal']['path'])
    X.require(value['package']['path'] == str(Y.E / 'p228-kir-indexed-formal-join-cpu-v1/manifest.json')
        and value['proposal']['path'] == str(Y.E / 'p228-kir-indexed-formal-join-v1/source-manifest.json')
        and value['package']['sha256'] == Y.PACKAGE_SHA and value['proposal']['sha256'] == Y.SOURCE_SHA,
        'exact source/controller generations')
    for pin in (value['package'], value['proposal']):
        key = str(Path(pin['path']).relative_to(Y.E))
        X.require(records[key] == pin, 'receipt/source package identity')
        expected_files.add(key)
    X.require(package['schema'] == 'ferric-p228-kir-indexed-formal-join-cpu-package-v1'
        and len(package['files']) == 3 and {row['path']: row['sha256'] for row in package['files']} == Y.CONTROLLER,
        'exact three-file controller package')
    for row in package['files']:
        path = Path(value['package']['path']).parent / X.relative(row['path'])
        X.require(X.extent(raw(path)) == {key: row[key] for key in ('bytes', 'sha256')}, 'package source body')
        expected_files.add(str(path.relative_to(Y.E)))
    X.require(proposal['schema'] == 'ferric-p228-kir-indexed-formal-join-source-v1'
        and len(proposal['files']) == 4 and sum(row['before'] is None for row in proposal['files']) == 2
        and set(value['formatted_sources']) == {row['path'] for row in proposal['files']}
        and value['added_tests'] == proposal['added_tests'], 'four overlays, two additions, twenty authored tests')
    for row in proposal['files']:
        X.require(row['source'] == 'draft/' + row['path'], 'declared source mapping')
        draft = Path(value['proposal']['path']).parent / X.relative(row['source'])
        X.require(X.extent(raw(draft)) == row['after'], 'proposal draft body')
        pin = value['formatted_sources'][row['path']]
        X.require(pin['path'] == str(Y.CASE / 'source/fe2o3' / X.relative(row['path']))
            and pin == records[str(Path(pin['path']).relative_to(Y.E))], 'actual compiled formatted source body')
        expected_files.update((str(draft.relative_to(Y.E)), str(Path(pin['path']).relative_to(Y.E))))
    source_readme = Path(value['proposal']['path']).parent / 'README.md'
    X.require(X.extent(raw(source_readme)) == dict(bytes=6033,
        sha256='54ec0e8f146735085f3cac58510446212633291634f73c528898559eb9156d90'), 'reviewed proposal README')
    expected_files.add(str(source_readme.relative_to(Y.E)))
    X.require(expected_files == set(records), 'no unrelated metadata admitted to publication')
    source_checks = dict(original_files=5783, candidate_files=5785, overlay_files=4, additions=2,
        source_transition_checked_remotely=True, source_and_dependency_postchecks_equal=True)
    X.require(manifest['remote_source_checks'] == source_checks, 'exporter full snapshot comparison, not local replay')
    for before, after in (('sources-before.json', 'sources-after.json'),
                          ('configurations-before.json', 'configurations-after.json'),
                          ('dependencies-before.json', 'dependencies-after.json')):
        X.require(all(value['raw'][before][key] == value['raw'][after][key] for key in ('bytes', 'sha256')),
            'retained before/after snapshot content identities')
    X.require(all(value['raw']['original-after.json'][key] == value['source_generation'][key]
                  for key in ('bytes', 'sha256')), 'unchanged original source identity')

    pure_raw = X.read(PURE, dict(sha256=PURE_SHA, bytes=PURE.stat().st_size))
    pure = X.parse(pure_raw)
    test_source = raw(Path(value['package']['path']).parent / 'test_run.py').decode()
    names = sorted(node.name for cls in ast.parse(test_source).body if isinstance(cls, ast.ClassDef)
        for node in cls.body if isinstance(node, ast.FunctionDef) and node.name.startswith('test_'))
    output = pure['terminal_tool_output']
    X.require(pure['schema'] == 'ferric-p228-kir-indexed-formal-join-root-test-observation-v1'
        and pure['remote_supervisor_receipt'] is False and pure['tests'] == len(names) == 16
        and pure['exit_code'] == output['exit_code'] == pure['errors'] == pure['failures'] == pure['skipped'] == 0
        and sorted(re.findall(r'^(test_\w+) \(__main__\.IndexedTests\) \.\.\. ok$', output['output'], re.M)) == names,
        'sixteen real primary-tool observations, not a remote owner receipt')
    expected_hashes = {str(Path(value['package']['path']).parent / row['path']): row['sha256'] for row in package['files']}
    expected_hashes.update({value['package']['path']: Y.PACKAGE_SHA, value['proposal']['path']: Y.SOURCE_SHA})
    for key in ('source_hash_before', 'source_hash_after'):
        sample = pure[key]
        rows = re.findall(r'^([0-9a-f]{64})  (.+)$', sample['output'], re.M)
        X.require(sample['exit_code'] == 0 and len(rows) == 5
            and {path: digest for digest, path in rows} == expected_hashes, 'actual before/after source observations')

    copies = {}
    for name, pin in records.items():
        path = Path(pin['path'])
        if path.is_relative_to(Y.CASE / 'source/fe2o3'):
            destination = 'source/' + str(path.relative_to(Y.CASE / 'source/fe2o3'))
        elif path.is_relative_to(Y.CASE):
            destination = ('attempt/' if path.name == 'failed.json' else 'raw/') + path.name
        elif path.is_relative_to(Y.OWNER): destination = 'owner/' + path.name
        elif path.is_relative_to(Path(value['package']['path']).parent): destination = 'controller/' + path.name
        elif path.is_relative_to(Path(value['proposal']['path']).parent):
            destination = 'proposal/' + str(path.relative_to(Path(value['proposal']['path']).parent))
        else: raise ValueError('unselected copy source')
        X.require(destination not in copies, 'unique destination')
        copies[destination] = (bodies[name], pin)
    for name, path in (('pure/root-observation.json', PURE), ('publication/export.py', HERE / 'export.py'),
                       ('publication/publish.py', Path(__file__).resolve())):
        body = X.read(path)
        copies[name] = (body, dict(path=str(path), **X.extent(body)))
    copies['publication/export-manifest.json'] = (manifest_raw, dict(path=str(root / 'export-manifest.json'), **X.extent(manifest_raw)))
    X.require(len(copies) == 60, 'closed small publication')
    if os.path.lexists(OUT):
        X.require(OUT.resolve(strict=True) == OUT and OUT.is_dir()
            and {path.name for path in OUT.iterdir()} <= {'README.md'}, 'fresh publication or root README only')
        if (OUT / 'README.md').exists(): X.read(OUT / 'README.md')
    for path, pin in local_pins.items(): X.read(Path(path), pin)
    X.read(archive, archive_pin)
    X.read(PURE, dict(bytes=len(pure_raw), sha256=PURE_SHA))
    ledger = {name: dict(original=pin, **X.extent(body)) for name, (body, pin) in copies.items()}
    summary_tests = {key: item for key, item in tests.items() if key != 'names'}
    result = dict(schema='ferric-p228-kir-indexed-formal-join-failure-publication-v1',
        publication_completed=True, qualification_passed=False, attempt=inner_pin, owner=outer_pin,
        archive=archive_pin, attempted_phases=list(Y.PHASES), unattempted_phases=list(Y.UNATTEMPTED),
        library_tests=summary_tests, controller_policy_tests=16,
        primary_pure_observation=dict(path=str(PURE), **X.extent(pure_raw)),
        selected_test_artifact=binary, formatted_sources=value['formatted_sources'],
        compiler_cpu=value['compiler_cpu'], finalizer_tools=value['finalizer_tools'],
        original_source_generation=value['source_generation'], retained_handoff=value['retained_handoff'],
        source_unchanged=value['source_unchanged'], postcheck_errors=value['postcheck_errors'],
        owned_exit_code=outcome['exit_code'], owned_elapsed_seconds=outcome['elapsed_seconds'],
        copied=ledger, remotely_rehashed_omitted_pins=omitted, remote_source_checks=source_checks,
        snapshot_bodies_rehashed_locally=False, owner_inventory_bodies_rehashed_locally=False,
        executable_bodies_rehashed_locally=False, transitive_inputs_replayed=False,
        baseline_cause_established=False, actual_capture_join_attempted=False, actual_capture_join_passed=False,
        compiler_qualification=False, fresh_hsaco_emitted=False, gpu_execution=False,
        numerical_acceptance=False, performance_claim=False)
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (body, _) in copies.items():
        path = OUT / X.relative(name); path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream: stream.write(body)
    with (OUT / 'result.json').open('x') as stream:
        json.dump(result, stream, indent=2, sort_keys=True); stream.write('\n')
    for name, (body, _) in copies.items(): X.read(OUT / name, X.extent(body))
    print(json.dumps(dict(output=str(OUT), copied_files=len(copies), result=X.extent(X.read(OUT / 'result.json')))))


if __name__ == '__main__':
    main()
