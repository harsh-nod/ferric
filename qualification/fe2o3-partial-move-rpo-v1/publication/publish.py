"""Publish retained RPO CPU data and the four formatted source overlays only."""
import argparse
import ast
import difflib
import io
import json
import os
from pathlib import Path
import re
import tarfile
import types

HERE = Path(__file__).resolve().parent
EXPORT_SHA = '5ba8484263ac02051a06b488f0db6b9565f6cff31d189c8dd35f07d90b08b720'
L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
OUT = Path('/home/harsh/ferric-p227-integration/qualification/fe2o3-partial-move-rpo-v1')
PURE = L / 'partial-move-rpo-cpu-v2-pure-root-v228-v1.json'
PURE_SHA = '97b89195eb20279d4b08d9a136be5660b53e319ec222550c0af7e3febfc7f178'


def helper():
    import hashlib
    raw = (HERE / 'export.py').read_bytes()
    if hashlib.sha256(raw).hexdigest() != EXPORT_SHA:
        raise ValueError('publication data helper identity')
    module = types.ModuleType('rpo_publication_data')
    module.__file__ = str(HERE / 'export.py')
    exec(compile(raw, module.__file__, 'exec'), module.__dict__)
    return module


def source_map(X, value, root):
    result = {}
    for name, row in value.items():
        X.require(set(row) == {'pin', 'stamp'} and X.pin(row['pin'])['path'] == name,
                  'source snapshot row')
        relative = X.relative(str(Path(name).relative_to(root)))
        X.require(type(row['stamp']) is list and len(row['stamp']) == 5
                  and all(type(n) is int for n in row['stamp'])
                  and row['stamp'][2] == row['pin']['bytes'], 'source stamp')
        result[str(relative)] = {k: row['pin'][k] for k in ('bytes', 'sha256')}
    X.require(len(result) == len(value), 'source roster uniqueness')
    return result


def actual_tests(X, body, short, suite, prior, additions):
    listed = re.findall(r'^([^\r\n]+): test$', body(short + '-list-stdout').decode(), re.M)
    ignored = re.findall(r'^([^\r\n]+): test$', body(short + '-ignored-list-stdout').decode(), re.M)
    X.require(listed and len(set(listed)) == len(listed) and len(set(ignored)) == len(ignored),
              'unique actual test inventories')
    text = body(short + '-tests-stdout').decode()
    rows = re.findall(r'^test (\S+)(?: - should panic)? \.\.\. (ok|ignored(?:, [^\n]*)?)$', text, re.M)
    passed = sorted(name for name, status in rows if status == 'ok')
    skipped = sorted(name for name, status in rows if status.startswith('ignored'))
    X.require(sorted(passed + skipped) == sorted(listed) and skipped == sorted(ignored), 'all named outcomes')
    actual = dict(passed=len(passed), ignored=len(skipped), names=sorted(listed), ignored_names=skipped)
    X.require(actual == suite and re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
              == [(str(len(passed)), '0', str(len(skipped)))], 'receipt and actual transcript agree')
    X.require(set(listed) == set(prior['names']) | set(additions)
              and not set(prior['names']) & set(additions)
              and skipped == prior['ignored_names'] and not set(additions) & set(skipped),
              'all predecessor tests/ignores retained, only declared additions')
    return dict(passed=len(passed), ignored=len(skipped), ignored_names=skipped, added_passed=sorted(additions))


def recipes(X, cpu, proposal):
    R = X.E.parent.parent
    N = R / 'toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu'
    source, target = X.CPU / 'source/fe2o3', X.CPU / 'target'
    cargo = str(N / 'bin/cargo')
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(source / 'Cargo.toml')]
    fmt = [str(N / 'bin/rustfmt'), '--edition', '2024', '--config', 'skip_children=true',
           *[str(source / row['path']) for row in proposal['files']]]
    result = {'rustfmt': (fmt, 60), 'rustfmt-check': ([fmt[0], '--check', *fmt[1:]], 60),
              'metadata': ([cargo, 'metadata', '--offline', '--locked', '--manifest-path',
                            str(source / 'Cargo.toml'), '--format-version', '1'], 120),
              'compiler-build': ([cargo, 'build', *common, '-p', 'rustc-codegen-fe2o3', '--lib',
                                  '--bin', 'fe2o3-rustc-extract', '--message-format=json'], 1800)}
    for short, package in (('pliron', 'fe2o3-pliron'), ('compiler', 'rustc-codegen-fe2o3')):
        binary = cpu['artifacts'][short + '-tests']['path']
        X.require(Path(binary).is_relative_to(target / 'debug/deps'), 'fresh test artifact path')
        result[short + '-build-tests'] = ([cargo, 'test', *common, '-p', package, '--lib',
                                           '--no-run', '--message-format=json'], 1800)
        result[short + '-list'] = ([binary, '--list', '--format', 'terse'], 120)
        result[short + '-ignored-list'] = ([binary, '--ignored', '--list', '--format', 'terse'], 120)
        result[short + '-tests'] = ([binary, '--test-threads=2'], 1800)
    return result


def verify(X, cpu, owner, read):
    doc = lambda path: X.parse(read(path))
    body = lambda name: read(X.CPU / name, cpu['raw'][name])
    raw_doc = lambda name: X.parse(body(name))
    proposal = doc(Path(cpu['patch']['path']))
    prior = doc(Path(cpu['qualified_generation']['completion']['path']))
    owned = owner['owned']
    X.require(type(owned['exit_code']) is int and owned['exit_code'] == 0 and owned['reason'] is None
              and owned['cleanup_signalled'] is False and owned['owned_groups_absent'] is True
              and owned['owned_processes_reaped'] is True and owned['deadline_seconds'] == 10800,
              'natural owned terminal CPU process')
    X.require(owner['gpu_execution'] is False and owner['production_authority'] is False,
              'owner nonclaims')
    X.require(doc(X.OWNER / 'owned-result.json') == owned, 'owner outcome body')
    X.require(doc(X.OWNER / 'old-targets-before.json') == doc(X.OWNER / 'after.json')['old_targets'],
              'recorded protected-target stat inventories unchanged')
    start = doc(X.OWNER / 'started.json')
    root_pid = start['parent']['pid']
    X.require(start['parent']['uid'] == 9661 and start['parent']['pgid'] == root_pid
              and start['parent']['sid'] == root_pid
              and any(e.get('reason') == 'spawned-parent' and e.get('identity') == start['parent']
                      for e in owned['lineage']), 'owner launch identity')
    command = doc(X.OWNER / 'command.json')
    X.require(command['argv'] == ['/usr/bin/python3', '-B', str(Path(cpu['package']['path']).with_name('run.py')),
                                 X.PACKAGE_SHA, '--child']
              and command['deadline_seconds'] == 10800 and command['gpu_execution'] is False,
              'owned controller command')
    pids = []
    tool_maps = []
    environments = []
    for name, (argv, deadline) in recipes(X, cpu, proposal).items():
        phase = cpu['phases'][name]
        X.require(raw_doc(name + '-result.json') == phase and phase['exit_code'] == 0
                  and phase['reason'] is None and phase['group_absent'] is True, 'natural phase: ' + name)
        cmd = raw_doc(name + '-command.json')
        X.require(cmd['argv'] == argv and cmd['deadline_seconds'] == deadline
                  and cmd['expected_exit'] == 0 and cmd['gpu_execution'] is False
                  and cmd['affinity'] == [8, 9] and cmd['nice'] == 10
                  and cmd['cache_cap_bytes'] == 6 << 30, 'exact bounded recipe: ' + name)
        env = cmd['env']
        X.require(all(env[k] == '' for k in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'))
                  and env['CARGO_TARGET_DIR'] == str(X.CPU / 'target') and env['CARGO_INCREMENTAL'] == '0'
                  and env['CARGO_BUILD_JOBS'] == '2' and env['TMPDIR'] == str(X.CPU / 'target/tmp'),
                  'CPU-only private target environment')
        environments.append(env)
        tool_maps.append(cmd['tools'])
        started = raw_doc(name + '-started.json')
        pid = started['pid']
        X.require(type(pid) is int and pid > 0 and started['pgid'] == pid
                  and pid in owned['owned_groups']
                  and any(e.get('event') == 'owned' and e.get('identity', {}).get('pid') == pid
                          and e['identity']['uid'] == 9661 and e['identity']['pgid'] == pid
                          for e in owned['lineage']), 'phase ownership lineage')
        pids.append(pid)
        for stream in ('stdout', 'stderr'):
            X.require(X.extent(body(name + '-' + stream))['sha256'] == phase[stream + '_sha256'],
                      'phase stream hash')
    X.require(len(set(pids)) == 12 and all(e == environments[0] for e in environments), 'same bounded environment')
    for name in ('original', 'dependencies'):
        X.require(raw_doc(name + '-before.json') == raw_doc(name + '-after.json'), name + ' postcheck')
    inputs = raw_doc('inputs-before.json')
    X.require(inputs['files'] == raw_doc('inputs-after.json')
              and inputs['configurations'] == raw_doc('configurations-after.json'), 'input/config postchecks')
    N = X.E.parent.parent / 'toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu'
    X.require(all(t == tool_maps[0] for t in tool_maps)
              and set(tool_maps[0]) == {'cargo', 'rustc', 'rustdoc'}, 'recorded tool roster')
    for tool, sha in tool_maps[0].items():
        X.require(inputs['files'][str(N / 'bin' / tool)]['pin']['sha256'] == sha, 'tool snapshot identity')
    for record in (cpu['package'], cpu['patch'], *cpu['qualified_generation'].values(), *inputs['patch']):
        X.require(inputs['files'][record['path']]['pin'] == record, 'immutable input pin join')
        read(Path(record['path']), record)
    controller_path = Path(cpu['package']['path']).with_name('run.py')
    X.require(X.extent(read(controller_path)) == {k: inputs['files'][str(controller_path)]['pin'][k]
                                                for k in ('bytes', 'sha256')}, 'executed controller snapshot')
    metadata = raw_doc('metadata-stdout')
    X.require(metadata['target_directory'] == str(X.CPU / 'target'), 'metadata private target')
    for package in ('fe2o3-pliron', 'rustc-codegen-fe2o3'):
        X.require([r['manifest_path'] for r in metadata['packages'] if r['name'] == package]
                  == [str(X.CPU / 'source/fe2o3/crates' / package / 'Cargo.toml')], 'changed package metadata')
    X.require(raw_doc('sources-before.json') == raw_doc('sources-after.json'), 'source postcheck')
    old_root = Path(cpu['qualified_generation']['sources']['path']).parent / 'source/fe2o3'
    old = source_map(X, doc(Path(cpu['qualified_generation']['sources']['path'])), old_root)
    unformatted = source_map(X, raw_doc('sources-unformatted.json'), X.CPU / 'source/fe2o3')
    formatted = source_map(X, raw_doc('sources-before.json'), X.CPU / 'source/fe2o3')
    expected = dict(old)
    source_rows, patch = [], []
    for row in proposal['files']:
        name = row['path']
        X.require(old.get(name) == row['before'], 'source preimage')
        expected[name] = row['after']
        record = cpu['formatted_sources'][name]
        after = {k: record[k] for k in ('bytes', 'sha256')}
        X.require(formatted[name] == after, 'compiled formatted overlay')
        after_raw = read(Path(record['path']), record)
        before_raw = b'' if row['before'] is None else read(
            Path(cpu['patch']['path']).parent / 'preimage' / name, row['before'])
        patch.extend(difflib.unified_diff(before_raw.decode().splitlines(keepends=True),
                    after_raw.decode().splitlines(keepends=True),
                    fromfile='/dev/null' if row['before'] is None else 'a/' + name, tofile='b/' + name))
        source_rows.append(dict(path=name, before=row['before'], proposed=row['after'], after=after,
                                formatted_source=record))
    X.require(len(old) == 5781 and len(formatted) == 5783 and unformatted == expected, 'exact source transition')
    selected = set(cpu['formatted_sources'])
    X.require(set(formatted) == set(unformatted)
              and all(formatted[k] == v for k, v in unformatted.items() if k not in selected),
              'formatter changed no unselected source')
    joined = raw_doc('qualified-generation.json')
    X.require(joined == dict(cpu['qualified_generation'], source_members=5781,
                             relative_source_identity_matched=True), 'qualified full-source join')
    X.require(cpu['required_test_names']['compiler'] == []
              and sorted(cpu['required_test_names']['pliron']) == sorted(proposal['test_names'])
              and len(set(proposal['test_names'])) == 17, 'declared new test inventory')
    tests = {short: actual_tests(X, body, short, cpu['tests'][short], prior['tests'][short],
                                cpu['required_test_names'][short]) for short in ('pliron', 'compiler')}
    X.require([(tests[s]['passed'], tests[s]['ignored']) for s in ('pliron', 'compiler')]
              == [(1504, 1), (1196, 24)], 'actual 2700/25 census')
    target = X.CPU / 'target'
    X.require(set(cpu['artifacts']) == {'pliron-tests', 'compiler-tests', 'fe2o3-rustc-extract',
                                      'librustc_codegen_fe2o3.rlib', 'librustc_codegen_fe2o3.so'}, 'five build artifacts')
    for key, package, symbol, test, phase in (
        ('pliron-tests', 'fe2o3-pliron', 'fe2o3_pliron', True, 'pliron-build-tests'),
        ('compiler-tests', 'rustc-codegen-fe2o3', 'rustc_codegen_fe2o3', True, 'compiler-build-tests'),
        ('fe2o3-rustc-extract', 'rustc-codegen-fe2o3', 'fe2o3-rustc-extract', False, 'compiler-build'),
        ('librustc_codegen_fe2o3.rlib', 'rustc-codegen-fe2o3', 'rustc_codegen_fe2o3', False, 'compiler-build'),
        ('librustc_codegen_fe2o3.so', 'rustc-codegen-fe2o3', 'rustc_codegen_fe2o3', False, 'compiler-build')):
        rows = [X.parse(line) for line in body(phase + '-stdout').splitlines() if line.strip()]
        rows = [r for r in rows if r.get('reason') == 'compiler-artifact'
                and r.get('target', {}).get('name') == symbol and r.get('profile', {}).get('test') is test
                and r.get('manifest_path') == str(X.CPU / 'source/fe2o3/crates' / package / 'Cargo.toml')]
        record = X.pin(cpu['artifacts'][key])
        X.require(len(rows) == 1 and Path(record['path']).is_relative_to(target)
                  and record['path'] in ([rows[0]['executable']] if rows[0]['executable'] else rows[0]['filenames']),
                  'actual Cargo artifact selection')
    return tests, source_rows, ''.join(patch).encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('retained_root', type=Path)
    parser.add_argument('archive', type=Path)
    parser.add_argument('archive_sha256')
    args = parser.parse_args()
    X = helper()
    root = args.retained_root.absolute()
    X.require(root.resolve(strict=True) == root and root.is_dir(), 'explicit canonical retained root')
    archive_raw = X.read(args.archive.absolute())
    X.require(X.extent(archive_raw)['sha256'] == X.digest(args.archive_sha256), 'actual archive identity')
    manifest_raw = X.read(root / 'export-manifest.json')
    manifest = X.parse(manifest_raw)
    X.require(manifest['schema'] == 'ferric-p228-rpo-cpu-metadata-export-v1' and manifest['original_root'] == str(X.E)
              and manifest['exporter']['sha256'] == EXPORT_SHA, 'authenticated export generation')
    read = lambda path, expected=None: X.read(root / Path(path).relative_to(X.E), expected)
    cpu, owner, records = X.roster(read, X.CPU_SHA, X.OWNER_SHA)
    X.require(manifest['files'] == records and manifest['cpu_sha256'] == X.CPU_SHA
              and manifest['owner_sha256'] == X.OWNER_SHA and manifest['artifact_bodies_exported'] is False
              and manifest['full_source_tree_exported'] is False, 'export exact derived roster')
    names = set(records) | {'export-manifest.json'}
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as archive:
        members = archive.getmembers()
        X.require(len(members) == len(names) and {m.name for m in members} == names
                  and all(m.isfile() and str(X.relative(m.name)) == m.name for m in members), 'closed ordinary tar')
        for member in members:
            expected = X.extent(manifest_raw) if member.name == 'export-manifest.json' else records[member.name]
            X.require(member.size == expected['bytes'] and X.extent(archive.extractfile(member).read())
                      == {k: expected[k] for k in ('bytes', 'sha256')}, 'archive body matches retained body')
    tests, sources, patch = verify(X, cpu, owner, read)
    pure_raw = X.read(PURE)
    pure = X.parse(pure_raw)
    X.require(X.extent(pure_raw)['sha256'] == PURE_SHA and pure['schema'] == 'ferric-p228-rpo-cpu-v2-root-test-observation-v1'
              and pure['exit_code'] == 0 and pure['tests'] == 16 and pure['failures'] == pure['errors'] == pure['skipped'] == 0
              and pure['manifest_sha256'] == X.PACKAGE_SHA and pure['raw_tool_result']['exit_code'] == 0,
              'actual primary-agent pure observation, not an owned receipt')
    package = Path(cpu['package']['path']).parent
    tree = ast.parse(read(package / 'test_run.py'))
    names = sorted(node.name + '.' + method.name for node in tree.body if isinstance(node, ast.ClassDef)
                   for method in node.body if isinstance(method, ast.FunctionDef) and method.name.startswith('test_'))
    log = pure['raw_tool_result']['output'].replace('\r\n', '\n')
    actual = sorted(cls + '.' + method for method, cls in re.findall(
        r'^(test_\w+) \(__main__[.](\w+)(?:[.]test_\w+)?\) \.\.\. ok$', log, re.M))
    X.require(len(names) == 16 and actual == names and re.search(r'\nRan 16 tests in [0-9.]+s\n\nOK\n', log),
              'named primary pure test transcript')
    copies = {}
    def copy(name, raw, original):
        X.relative(name)
        X.require(name not in copies, 'unique public destination')
        copies[name] = (raw, original)
    copy('complete.json', read(X.CPU / 'complete.json'), owner['completion'])
    copy('owner-complete.json', read(X.OWNER / 'complete.json'), dict(path=str(X.OWNER / 'complete.json'),
                                                                 **X.extent(read(X.OWNER / 'complete.json'))))
    for phase in sorted(X.PHASES):
        for suffix in X.SUFFIXES:
            name = phase + '-' + suffix
            copy('raw/' + name, read(X.CPU / name, cpu['raw'][name]), cpu['raw'][name])
    for name in sorted(X.OWNER_FILES - {'complete.json', 'after.json', 'old-targets-before.json'}):
        raw = read(X.OWNER / name)
        copy('owner/' + name, raw, dict(path=str(X.OWNER / name), **X.extent(raw)))
    for name, record in records.items():
        original = Path(record['path'])
        if original.is_relative_to(package):
            copy('controller/' + str(original.relative_to(package)), read(original, record), record)
        if original.is_relative_to(Path(cpu['patch']['path']).parent):
            copy('proposal/' + str(original.relative_to(Path(cpu['patch']['path']).parent)), read(original, record), record)
    for row in sources:
        copy('source/' + row['path'], read(Path(row['formatted_source']['path']), row['formatted_source']), row['formatted_source'])
    copy('pure-root-observation.json', pure_raw, dict(path=str(PURE), **X.extent(pure_raw)))
    for name in ('export.py', 'publish.py'):
        raw = X.read(HERE / name)
        copy('publication/' + name, raw, dict(path=str(HERE / name), **X.extent(raw)))
    copy('compiler.patch', patch, None)
    copy('source-pins.json', (json.dumps(dict(schema='FerricPartialMoveRpoSourcePinsV1', files=sources,
        qualified_generation=cpu['qualified_generation'], source_members_before=5781, source_members_after=5783,
        patch=X.extent(patch), compiler_limits_changed=False, runtime_adoption=False), indent=2, sort_keys=True) + '\n').encode(), None)
    ledger = [dict(path=name, original=original, **X.extent(raw)) for name, (raw, original) in sorted(copies.items())]
    summary = dict(schema='FerricPartialMoveRpoCpuQualificationV1', passed=True, tests=tests,
        tests_passed=2700, tests_ignored=25, raw_completion=owner['completion'],
        owner_completion=dict(path=str(X.OWNER / 'complete.json'), **X.extent(read(X.OWNER / 'complete.json'))),
        runner_package=cpu['package'], source_proposal=cpu['patch'], qualified_generation=cpu['qualified_generation'],
        phases=cpu['phases'], artifacts=cpu['artifacts'], artifact_bodies_rehashed=False,
        source_snapshots={k: cpu['raw'][k] for k in X.SNAPSHOTS}, source_files=sources,
        owned_outcome={k: owner['owned'][k] for k in ('exit_code', 'reason', 'cleanup_signalled',
            'owned_groups_absent', 'owned_processes_reaped', 'elapsed_seconds', 'deadline_seconds')},
        pure_tests=dict(tests=16, observation_kind=pure['observation_kind'], source_postcheck_replayed=False,
                        observation=dict(path=str(PURE), **X.extent(pure_raw))),
        retained_archive=dict(path=str(args.archive.absolute()), **X.extent(archive_raw)), retained_files=records,
        copied=ledger, fresh_compiler_built=True, compiler_limits_changed=False, runtime_adoption=False,
        finalizer_qualification_included=False, actual_capture_replay_executed=False,
        limitations=[
            'Data-only replay of recorded source, commands, named tests, ownership and postchecks; no test rerun.',
            'Only four formatted compiler files are published, not a complete independent compiler source/toolchain.',
            'Five executable/library identities are joined to Cargo records; their bodies are not transported or rehashed here.',
            'All retained snapshots are rehashed as data. Untransported dependency/tool/full-source bodies are not rehashed.',
            'Old-target postchecks are recorded stat inventories, not proof of absence of transient writes.',
            'The 16 pure cases are a primary SSH-output observation, not an owned remote supervisor receipt.',
            'Preserved ignored suites include actual native/capture cases; default CPU success does not execute those cases.',
            'Finalizer qualification, checked RoPE lowering, new HSACO and independent model acceptance remain separate gates.'],
        **{k: False for k in X.FALSE_FIELDS})
    # Every source and all input evidence is rehashed before the first publication write.
    for record in records.values():
        read(Path(record['path']), record)
    X.require(X.read(PURE) == pure_raw and X.read(args.archive.absolute()) == archive_raw, 'publication input postcheck')
    for name, (raw, original) in copies.items():
        if name.startswith('publication/'):
            X.require(X.read(Path(original['path'])) == raw, 'publisher source postcheck')
    if OUT.exists():
        X.require(OUT.resolve(strict=True) == OUT and OUT.is_dir()
                  and all(p.name == 'README.md' and p.is_file() and not p.is_symlink() for p in OUT.iterdir()),
                  'only root README may preexist')
    else:
        OUT.mkdir()
    for name, (raw, _) in sorted(copies.items()):
        target = OUT / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(raw)
        X.require(X.extent(X.read(target)) == X.extent(raw), 'published body')
    raw = (json.dumps(summary, indent=2, sort_keys=True) + '\n').encode()
    with (OUT / 'result.json').open('xb') as stream:
        stream.write(raw)
    print(json.dumps(dict(output=str(OUT), files=len(copies), result=X.extent(X.read(OUT / 'result.json')))))


if __name__ == '__main__':
    main()
