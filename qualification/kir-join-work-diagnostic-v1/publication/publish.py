"""Publish retained diagnostic data without importing the tested controller."""
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
HERE = L / 'proposals/p228-kir-join-work-diagnostic-publication-v1'
OUT = F / 'qualification/kir-join-work-diagnostic-v1'
COMMON = F / 'qualification/fe2o3-partial-move-rpo-v1/publication/export.py'
EXPORT_SHA = 'd495f817318d7701a0951a7dff94055c88a8bf574a2fcf3d3e16e7f0e6082169'
PURE = L / 'kir-join-work-diagnostic-root-test-observation-v228-v1.json'
PURE_SHA = '6a0b7f5b185b036c9fb546b796addf1e57243da8dc5054ef7a516d2b32d8b5d0'
SELECTOR = 'linux::wave_qkv_attention_output_tiles_v6::tests::wave_emission_actual_retained_v6_passes_full_inert_join'
PACKAGE_SHA = 'ff66f002141884538d54cc338ea34dd478c56f6ea97e5f120ae91b37680d8319'


def exporter():
    path = HERE / 'export.py'
    raw = path.read_bytes()
    if path.resolve(strict=True) != path or hashlib.sha256(raw).hexdigest() != EXPORT_SHA:
        raise ValueError('reviewed data courier source')
    module = types.ModuleType('kir_diagnostic_data_export')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module, module.common(COMMON)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('retained_root', type=Path)
    parser.add_argument('archive', type=Path)
    parser.add_argument('archive_sha256')
    parser.add_argument('complete_sha256')
    parser.add_argument('owner_sha256')
    args = parser.parse_args()
    Y, X = exporter()
    root = args.retained_root.absolute()
    X.require(root.resolve(strict=True) == root and root.is_dir(), 'canonical retained root')
    archive = args.archive.absolute()
    archive_pin = dict(path=str(archive), **X.extent(X.read(archive)))
    X.require(archive_pin['sha256'] == X.digest(args.archive_sha256), 'actual courier archive')
    manifest_raw = X.read(root / 'export-manifest.json')
    manifest = X.parse(manifest_raw)
    X.require(manifest['schema'] == 'ferric-p228-kir-join-work-diagnostic-export-v1'
              and manifest['original_root'] == str(Y.E)
              and manifest['complete_sha256'] == args.complete_sha256
              and manifest['owner_sha256'] == args.owner_sha256
              and manifest['exporter']['sha256'] == EXPORT_SHA
              and all(manifest[key] is False for key in ('snapshot_bodies_exported',
                  'artifact_bodies_exported', 'full_source_tree_exported')), 'exact data courier scope')
    records = manifest['files']
    X.require(len(records) == 61, 'closed small courier roster')
    seen, bodies, local_pins = set(), {}, {}
    for path in root.rglob('*'):
        X.require(not path.is_symlink(), 'no retained symlink')
        if path.is_file():
            seen.add(str(path.relative_to(root)))
        else:
            X.require(path.is_dir(), 'ordinary retained tree')
        X.require(len(seen) <= 62, 'bounded retained file tree')
    X.require(seen == set(records) | {'export-manifest.json'}, 'no missing or extra retained file')
    for name, pin in records.items():
        X.require(X.pin(pin)['path'] == str(Y.E / X.relative(name)), 'original courier path')
        path = root / name
        bodies[name] = X.read(path, pin)
        local_pins[str(path)] = dict(path=str(path), **X.extent(bodies[name]))

    def raw(path):
        return bodies[str(Path(path).relative_to(Y.E))]

    def doc(path):
        return X.parse(raw(path))

    value, owner = doc(Y.CASE / 'complete.json'), doc(Y.OWNER / 'complete.json')
    complete_pin = records[str((Y.CASE / 'complete.json').relative_to(Y.E))]
    owner_pin = records[str((Y.OWNER / 'complete.json').relative_to(Y.E))]
    X.require(complete_pin['sha256'] == X.digest(args.complete_sha256)
              and owner_pin['sha256'] == X.digest(args.owner_sha256), 'actual terminal body pins')
    X.success(value, 'ferric-p228-kir-join-work-diagnostic-cpu-result-v1')
    X.success(owner, 'ferric-p228-kir-join-work-diagnostic-owned-result-v1')
    X.require(owner['completion'] == complete_pin and owner['package'] == value['package']
              and owner['proposal'] == value['proposal'], 'owner diagnostic generation')
    X.require(value['diagnostic_completed'] is True and value['source_unchanged'] is True
              and all(value[key] is False for key in ('limits_changed', 'actual_capture_join_passed',
                  'fresh_compiler_built', 'fresh_hsaco_emitted', 'gpu_execution', 'numerical_acceptance',
                  'production_authority', 'performance_claim')), 'no join/image/accuracy authority')
    outcome = owner['owned']
    X.require(outcome == doc(Y.OWNER / 'owned-result.json') and outcome['exit_code'] == 0
              and outcome['reason'] is None and outcome['cleanup_signalled'] is False
              and outcome['owned_groups_absent'] is True and outcome['owned_processes_reaped'] is True,
              'natural completed diagnostic owner')
    identity = doc(Y.OWNER / 'started.json')['parent']
    X.require(identity['pid'] == identity['pgid'] == identity['sid'] and identity['uid'] == 9661
              and any(row.get('identity') == identity and row.get('reason') == 'spawned-parent'
                      for row in outcome['lineage']), 'actual root process identity')
    leaves = {phase + '-' + suffix for phase in Y.PHASES for suffix in X.SUFFIXES}
    X.require(set(value['phases']) == Y.PHASES and set(value['raw']) == leaves | Y.SNAPSHOTS,
              'eight phases and ten linked snapshots')
    snapshots = {str((Y.CASE / name).relative_to(Y.E)): value['raw'][name] for name in Y.SNAPSHOTS}
    X.require(manifest['remotely_rehashed_snapshot_pins'] == snapshots, 'exact omitted snapshot pins')
    binary = X.pin(value['artifacts']['finalizer-tests'])
    X.require(set(value['artifacts']) == {'finalizer-tests'}
              and Path(binary['path']).is_relative_to(Y.CASE / 'target/debug/examples'), 'selected test ELF identity only')
    for phase in sorted(Y.PHASES):
        for suffix in X.SUFFIXES:
            name = phase + '-' + suffix
            X.require(value['raw'][name] == records[str((Y.CASE / name).relative_to(Y.E))], 'raw body identity')
        command, result = doc(Y.CASE / (phase + '-command.json')), doc(Y.CASE / (phase + '-result.json'))
        started = doc(Y.CASE / (phase + '-started.json'))
        expected_exit = 101 if phase == 'actual-inert-join-diagnostic' else 0
        X.require(value['phases'][phase] == result and result['exit_code'] == expected_exit
                  and result['reason'] is None and result['group_absent'] is True
                  and started['pid'] == started['pgid'] and command['expected_exit'] == expected_exit
                  and command['affinity'] == [8, 9] and command['nice'] == 10
                  and command['gpu_execution'] is False, 'natural bounded leaf outcome')
        X.require(all(command['env'][key] == '' for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')),
                  'hidden native devices')
        for stream in ('stdout', 'stderr'):
            X.require(X.extent(raw(Y.CASE / (phase + '-' + stream)))['sha256'] == result[stream + '_sha256'], 'leaf stream hash')
        if phase in ('finalizer-list', 'finalizer-ignored-list', 'finalizer-tests', 'actual-inert-join-diagnostic'):
            X.require(command['argv'][0] == binary['path'], 'same selected actual test ELF')
    build = [X.parse(line) for line in raw(Y.CASE / 'finalizer-build-tests-stdout').splitlines() if line.strip()]
    selected = [row for row in build if row.get('reason') == 'compiler-artifact'
        and row.get('target', {}).get('name') == 'finite_join_engineering_hsaco_v1'
        and row.get('profile', {}).get('test') is True
        and row.get('manifest_path') == str(Y.CASE / 'source/fe2o3/crates/fe2o3-hsaco-finalize/Cargo.toml')]
    X.require(len(selected) == 1 and selected[0]['executable'] == binary['path'], 'actual Cargo artifact, no inferred ELF')
    names = sorted(re.findall(r'^([^\r\n]+): test$', raw(Y.CASE / 'finalizer-list-stdout').decode(), re.M))
    ignored = sorted(re.findall(r'^([^\r\n]+): test$', raw(Y.CASE / 'finalizer-ignored-list-stdout').decode(), re.M))
    default_log = raw(Y.CASE / 'finalizer-tests-stdout').decode()
    default_rows = re.findall(r'^test (\S+)(?: - should panic)? \.\.\. (ok|ignored(?:, [^\n]*)?)$', default_log, re.M)
    actual_tests = dict(passed=sum(status == 'ok' for _, status in default_rows), ignored=len(ignored), names=names, ignored_names=ignored)
    X.require(len(names) == len(set(names)) == 205 and len(ignored) == 15
              and sorted(name for name, _ in default_rows) == names
              and sorted(name for name, status in default_rows if status.startswith('ignored')) == ignored
              and actual_tests == value['tests'] and actual_tests['passed'] == 190
              and re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', default_log) == [('190', '0', '15')],
              '190 named default passes and15 historical ignores')
    command = doc(Y.CASE / 'actual-inert-join-diagnostic-command.json')
    X.require(command['argv'][1:] == ['--exact', SELECTOR, '--ignored', '--show-output', '--test-threads=1']
              and command['env']['FE2O3_KIR_JOIN_WORK_DIAGNOSTIC_V1'] == '1', 'exact opted-in ignored test')
    handoff = value['retained_handoff']
    for key, field in (('PATH', 'path'), ('BYTES', 'bytes'), ('SHA256', 'sha256')):
        X.require(command['env']['FE2O3_WAVE_QKV_ATTENTION_OUTPUT_TILE_ENGINEERING_V6_' + key] == str(handoff[field]), 'retained handoff binding')
    log = (raw(Y.CASE / 'actual-inert-join-diagnostic-stdout') + b'\n' + raw(Y.CASE / 'actual-inert-join-diagnostic-stderr')).decode()
    observed = value['expected_refusal']
    X.require(observed['expected_refusal_observed'] is True and observed['test'] == SELECTOR
              and observed['exit_code'] == 101 and observed['actual_capture_join_passed'] is False
              and observed['total_required_work_known'] is False and observed['failure_phase_inferred'] is False
              and re.findall(r'^test (\S+) \.\.\. (\S+)$', log, re.M) == [(SELECTOR, 'FAILED')]
              and re.findall(r'CanonicalKernelIrWorkLimitV1 \{ actual: (\d+), limit: (\d+) \}', log)
                  == [(str(observed['error']['actual']), str(observed['error']['limit']))], 'actual unchanged work refusal')
    trace_lines = []
    for row in observed['markers']:
        fields = ('rows', 'blocks', 'nodes', 'charge') if row['stage'] == 'join-shape' else ('work', 'remaining', 'storage', 'peak', 'charge')
        trace_lines.append('kir-join-work-v1 stage=' + row['stage'] + ''.join(' ' + key + '=' + str(row[key]) for key in fields))
    X.require([line for line in log.splitlines() if 'kir-join-work-v1' in line] == trace_lines, 'recorded checkpoint rows equal actual log')
    shape, last = observed['markers'][-2:]
    X.require(shape['stage'] == 'join-shape' and last['stage'] == 'layout-join-before'
              and shape['charge'] == (shape['rows'] + shape['blocks'] + 1) * shape['nodes'] * 128 == last['charge']
              and last['work'] + last['remaining'] == observed['error']['limit'] == 1 << 30
              and last['work'] + last['charge'] == observed['error']['actual'] > observed['error']['limit'], 'measured rejecting bulk charge')
    package, proposal = doc(value['package']['path']), doc(value['proposal']['path'])
    for pin in (value['package'], value['proposal']):
        X.require(pin == records[str(Path(pin['path']).relative_to(Y.E))], 'complete package FilePin join')
    X.require(value['package']['sha256'] == PACKAGE_SHA and value['proposal']['sha256'] == Y.SOURCE_SHA
              and len(package['files']) == 3 and {row['path']: row['sha256'] for row in package['files']} == Y.CONTROLLER,
              'frozen source packages')
    for row in package['files']:
        X.require(X.extent(raw(Path(value['package']['path']).parent / row['path'])) == {key: row[key] for key in ('bytes', 'sha256')}, 'controller body')
    X.require(proposal['handoff'] == handoff and len(proposal['files']) == 3
              and set(value['formatted_sources']) == {row['path'] for row in proposal['files']}, 'same three-source diagnostic')
    for row in proposal['files']:
        X.require(X.extent(raw(Path(value['proposal']['path']).parent / row['source'])) == row['after'], 'draft source body')
        pin = value['formatted_sources'][row['path']]
        X.require(pin == records[str(Path(pin['path']).relative_to(Y.E))]
                  and pin['path'] == str(Y.CASE / 'source/fe2o3' / X.relative(row['path'])), 'actual formatted source body')

    pure_raw = X.read(PURE)
    X.require(X.extent(pure_raw)['sha256'] == PURE_SHA, 'actual primary16 observation')
    pure = X.parse(pure_raw)
    test_source = raw(Path(value['package']['path']).parent / 'test_run.py').decode()
    tests = sorted(node.name for cls in ast.parse(test_source).body if isinstance(cls, ast.ClassDef)
        for node in cls.body if isinstance(node, ast.FunctionDef) and node.name.startswith('test_'))
    terminal = pure['terminal_tool_output']
    X.require(pure['remote_supervisor_receipt'] is False and pure['exit_code'] == terminal['exit_code'] == 0
              and pure['tests'] == len(tests) == 16 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
              and sorted(re.findall(r'^(test_\w+) \(__main__\.DiagnosticTests\) \.\.\. ok$', terminal['output'], re.M)) == tests,
              'actual named16 primary observation, not remote owner')
    expected_hashes = {str(Path(value['package']['path']).parent / row['path']): row['sha256'] for row in package['files']}
    expected_hashes[value['package']['path']] = value['package']['sha256']
    for key in ('source_hash_before', 'source_hash_after'):
        sample = pure[key]
        rows = re.findall(r'^([0-9a-f]{64})  (.+)$', sample['output'], re.M)
        X.require(sample['exit_code'] == 0 and len(rows) == 4 and {path: sha for sha, path in rows} == expected_hashes, 'observed source hashes')

    copies, linked_owner_inventories = {}, {}
    for name, pin in records.items():
        path = Path(pin['path'])
        if path in (Y.OWNER / 'after.json', Y.OWNER / 'old-targets-before.json'):
            linked_owner_inventories[path.name] = dict(original=pin, retained=local_pins[str(root / name)])
            continue
        if path.is_relative_to(Y.CASE / 'source/fe2o3'):
            destination = 'source/' + str(path.relative_to(Y.CASE / 'source/fe2o3'))
        elif path.is_relative_to(Y.CASE):
            destination = ('diagnostic/' if path.name == 'complete.json' else 'raw/') + path.name
        elif path.is_relative_to(Y.OWNER):
            destination = 'owner/' + path.name
        elif path.is_relative_to(Path(value['package']['path']).parent):
            destination = 'controller/' + path.name
        elif path.is_relative_to(Path(value['proposal']['path']).parent):
            destination = 'proposal/' + str(path.relative_to(Path(value['proposal']['path']).parent))
        else:
            raise ValueError('unselected courier file')
        X.require(destination not in copies, 'unique published destination')
        copies[destination] = (bodies[name], pin)
    for destination, path in (('pure/root-observation.json', PURE), ('publication/export.py', HERE / 'export.py'),
                              ('publication/publish.py', Path(__file__).resolve())):
        body = X.read(path)
        copies[destination] = (body, dict(path=str(path), **X.extent(body)))
    copies['publication/export-manifest.json'] = (manifest_raw, dict(path=str(root / 'export-manifest.json'), **X.extent(manifest_raw)))
    table = ['# Actual Work Checkpoints', '', '| Stage | Accepted Work | Pending Charge | Live Storage | Peak Storage |',
             '| --- | ---: | ---: | ---: | ---: |']
    for row in observed['markers']:
        table.append('| ' + ' | '.join(str(row.get(key, '')) for key in ('stage', 'work', 'charge', 'storage', 'peak')) + ' |')
    table += ['', 'The rejected charge was not performed. Attempted cumulative work is not total required work.',
              'This diagnostic did not accept the inert join or emit an image.', '']
    table_raw = '\n'.join(table).encode('ascii')
    if os.path.lexists(OUT):
        X.require(OUT.resolve(strict=True) == OUT and OUT.is_dir()
                  and {path.name for path in OUT.iterdir()} <= {'README.md'}, 'fresh publication or root README only')
        if (OUT / 'README.md').exists(): X.read(OUT / 'README.md')
    for path, pin in local_pins.items(): X.read(Path(path), pin)
    X.read(archive, archive_pin)
    X.read(PURE, dict(bytes=len(pure_raw), sha256=PURE_SHA))
    ledger = {name: dict(original=pin, bytes=len(body), sha256=X.extent(body)['sha256']) for name, (body, pin) in copies.items()}
    result = dict(schema='ferric-p228-kir-join-work-diagnostic-publication-v1', diagnostic_completed=True,
        diagnostic=complete_pin, owner=owner_pin, archive=archive_pin, default_tests=value['tests'],
        expected_refusal=observed, primary_pure_observation=dict(path=str(PURE), bytes=len(pure_raw), sha256=PURE_SHA),
        selected_test_artifact=binary, formatted_sources=value['formatted_sources'],
        compiler_cpu=value['compiler_cpu'], finalizer_tools=value['finalizer_tools'],
        preserved_compiler_products=value['preserved_compiler_products'], preserved_finalizer_products=value['preserved_finalizer_products'],
        copied=ledger, checkpoint_table=dict(path='checkpoints.md', **X.extent(table_raw)),
        linked_owner_inventories=linked_owner_inventories,
        owner_inventory_bodies_rehashed_locally=True, owner_inventory_bodies_copied=False,
        remotely_rehashed_snapshot_pins=snapshots, snapshot_bodies_rehashed_locally=False,
        executable_bodies_rehashed_locally=False, transitive_inputs_replayed=False,
        actual_capture_join_passed=False, fresh_hsaco_emitted=False, compiler_qualification=False,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False)
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (body, _) in copies.items():
        path = OUT / X.relative(name)
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream: stream.write(body)
    with (OUT / 'checkpoints.md').open('xb') as stream: stream.write(table_raw)
    with (OUT / 'result.json').open('x') as stream:
        json.dump(result, stream, indent=2, sort_keys=True); stream.write('\n')
    for name, (body, _) in copies.items(): X.read(OUT / name, X.extent(body))
    X.read(OUT / 'checkpoints.md', X.extent(table_raw))
    print(json.dumps(dict(output=str(OUT), copied_files=len(copies), result=X.extent(X.read(OUT / 'result.json')))))


if __name__ == '__main__':
    main()
