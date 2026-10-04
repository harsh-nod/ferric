"""Add finalizer CPU evidence without changing the published RPO CPU checkpoint."""
import argparse
import ast
import hashlib
import io
import json
from pathlib import Path
import re
import tarfile
import types

HERE = Path(__file__).resolve().parent
EXPORT_SHA = '9576596a516d55f07d4d921e4df57df18f28452b76e53a8f67991c97e9969b96'
F = Path('/home/harsh/ferric-p227-integration')
L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
PRIOR = F / 'qualification/fe2o3-partial-move-rpo-v1'
PRIOR_SHA = 'b5f03a8741ec1795db7070a3428d0620794847ffc000551ad7dc2307368600fe'
REPLAY_SHA = 'f7d47f15d2e4239186964e100b51e352e7107390f8c3a63b78f30e6ded48a485'
PURE = L / 'partial-move-rpo-finalizer-v2-pure-root-v228-v1.json'
PURE_SHA = '23f8474784c53a6bc63283c7b3d6c37b1113480989a99ea939d2a3fbd45560b2'
OUT = F / 'qualification/fe2o3-partial-move-rpo-finalizer-v1'


def load(path, sha, name):
    raw = path.read_bytes()
    if path.resolve(strict=True) != path or hashlib.sha256(raw).hexdigest() != sha:
        raise ValueError('publication data-helper identity')
    value = types.ModuleType(name)
    value.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), value.__dict__)
    return value


def verify(X, D, P, value, owner, cpu, read, old_read):
    doc = lambda path: X.parse(read(path))
    body = lambda name: read(D.ROW / name, value['raw'][name])
    raw_doc = lambda name: X.parse(body(name))
    old_doc = lambda name: X.parse(old_read(X.CPU / name, cpu['raw'][name]))
    before = raw_doc('before.json')
    X.require(before['files'] == raw_doc('files-after.json')
              and before['sources'] == raw_doc('sources-after.json') == old_doc('sources-before.json')
              and before['configurations'] == raw_doc('configurations-after.json')
              == old_doc('inputs-before.json')['configurations']
              and raw_doc('inputs-after.json') == old_doc('inputs-before.json')['files']
              and raw_doc('dependencies-after.json') == old_doc('dependencies-before.json')
              and raw_doc('metadata-stdout') == old_doc('metadata-stdout'),
              'exact qualified source/dependency/tool/configuration/metadata postchecks')
    X.require(len(before['sources']) == 5783 and value['compiler_artifacts'] == cpu['artifacts'],
              'same qualified compiler source and five products')
    for record in (value['package'], value['compiler_cpu'], value['compiler_owner'], value['prior_finalizer'],
                   *value['patches'].values(), *value['compiler_artifacts'].values(), value['source_snapshot']):
        X.require(before['files'][record['path']]['pin'] == record, 'preserved fixed input/product identity')
    controller = Path(value['package']['path']).with_name('run.py')
    X.require(X.extent(read(controller)) == {k: before['files'][str(controller)]['pin'][k]
                                           for k in ('bytes', 'sha256')}, 'executed controller source')
    old_owner = X.parse(read(Path(value['compiler_owner']['path']), value['compiler_owner']))
    X.require(old_owner['completion'] == value['compiler_cpu'] and old_owner['passed'] is True,
              'compiler owner link')
    owned = owner['owned']
    X.require(owned['exit_code'] == 0 and type(owned['exit_code']) is int and owned['reason'] is None
              and owned['cleanup_signalled'] is False and owned['owned_groups_absent'] is True
              and owned['owned_processes_reaped'] is True and owned['deadline_seconds'] == 10800
              and owner['gpu_execution'] is False and owner['production_authority'] is False,
              'naturally completed/reaped owned finalizer process')
    X.require(doc(D.OWNER / 'owned-result.json') == owned
              and doc(D.OWNER / 'old-targets-before.json') == doc(D.OWNER / 'after.json')['old_targets'],
              'owned result and recorded old-target postcheck')
    started = doc(D.OWNER / 'started.json')
    X.require(started['parent']['uid'] == 9661 and started['parent']['pid'] == started['parent']['pgid']
              == started['parent']['sid'] and any(e.get('reason') == 'spawned-parent'
              and e.get('identity') == started['parent'] for e in owned['lineage']), 'root process identity')
    cmd = doc(D.OWNER / 'command.json')
    X.require(cmd['argv'] == ['/usr/bin/python3', '-B', str(controller), D.PACKAGE_SHA,
                                 X.CPU_SHA, X.OWNER_SHA, '--child']
              and cmd['deadline_seconds'] == 10800 and cmd['gpu_execution'] is False, 'owned command')
    N = X.E.parent.parent / 'toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu'
    source, target = X.CPU / 'source/fe2o3', X.CPU / 'target'
    cargo = str(N / 'bin/cargo')
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(source / 'Cargo.toml')]
    executable = 'finite_join_engineering_hsaco_v1'
    metadata = 'finite_join_request_metadata_v1'
    binary = value['artifacts']['finalizer-tests']['path']
    recipes = {
        'metadata': ([cargo, 'metadata', '--offline', '--locked', '--manifest-path', str(source / 'Cargo.toml'), '--format-version', '1'], 120),
        'finalizer-build': ([cargo, 'build', *common, '-p', 'fe2o3-hsaco-finalize', '--example', executable,
                            '--example', metadata, '--message-format=json'], 1800),
        'finalizer-build-tests': ([cargo, 'test', *common, '-p', 'fe2o3-hsaco-finalize', '--example', executable,
                                  '--no-run', '--message-format=json'], 1800),
        'finalizer-list': ([binary, '--list', '--format', 'terse'], 120),
        'finalizer-ignored-list': ([binary, '--ignored', '--list', '--format', 'terse'], 120),
        'finalizer-tests': ([binary, '--test-threads=2'], 1800)}
    reference_cmd = old_doc('compiler-build-command.json')
    pids = []
    for name, (argv, deadline) in recipes.items():
        command, start, result = [raw_doc(name + '-' + suffix + '.json') for suffix in ('command', 'started', 'result')]
        X.require(command['argv'] == argv and command['deadline_seconds'] == deadline
                  and command['env'] == reference_cmd['env'] and command['tools'] == reference_cmd['tools']
                  and command['cache_cap_bytes'] == 6 << 30 and command['affinity'] == [8, 9]
                  and command['nice'] == 10 and command['expected_exit'] == 0 and command['gpu_execution'] is False,
                  'exact six bounded CPU-only commands')
        X.require(result == value['phases'][name] and result['exit_code'] == 0 and result['reason'] is None
                  and result['group_absent'] is True, 'natural finalizer phase')
        pid = start['pid']
        X.require(type(pid) is int and pid > 0 and start['pgid'] == pid and pid in owned['owned_groups']
                  and any(e.get('event') == 'owned' and e.get('identity', {}).get('pid') == pid
                          and e['identity']['pgid'] == pid and e['identity']['uid'] == 9661
                          for e in owned['lineage']), 'owned phase lineage')
        pids.append(pid)
        for stream in ('stdout', 'stderr'):
            X.require(X.extent(body(name + '-' + stream))['sha256'] == result[stream + '_sha256'], 'phase stream')
    X.require(len(set(pids)) == 6, 'six distinct phase leaders')
    prior = doc(Path(value['prior_finalizer']['path']))
    tests = P.actual_tests(X, body, 'finalizer', value['tests'], prior['tests'], [])
    X.require(tests['passed'] == 190 and tests['ignored'] == 15, '190 passed / 15 preserved ignored names')
    X.require(set(value['artifacts']) == {'finalizer', 'metadata', 'finalizer-tests'}, 'three selected finalizer products')
    for key, name, test, phase in (('finalizer', executable, False, 'finalizer-build'),
                                  ('metadata', metadata, False, 'finalizer-build'),
                                  ('finalizer-tests', executable, True, 'finalizer-build-tests')):
        rows = [X.parse(line) for line in body(phase + '-stdout').splitlines() if line.strip()]
        rows = [r for r in rows if r.get('reason') == 'compiler-artifact' and r.get('target', {}).get('name') == name
                and r.get('profile', {}).get('test') is test
                and r.get('manifest_path') == str(source / 'crates/fe2o3-hsaco-finalize/Cargo.toml')]
        record = X.pin(value['artifacts'][key])
        X.require(len(rows) == 1 and rows[0]['executable'] == record['path']
                  and Path(record['path']).is_relative_to(target / 'debug/examples'), 'actual finalizer Cargo product')
    products = [*value['artifacts'].values(), *value['compiler_artifacts'].values()]
    X.require(len(products) == len({r['path'] for r in products}) == 8, 'eight distinct compiler/finalizer product identities')
    return tests


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('retained_root', type=Path)
    parser.add_argument('cpu_retained_root', type=Path)
    parser.add_argument('archive', type=Path)
    parser.add_argument('archive_sha256')
    args = parser.parse_args()
    D = load(HERE / 'export.py', EXPORT_SHA, 'rpo_finalizer_export_data')
    X = D.common(PRIOR / 'publication/export.py')
    P = load(PRIOR / 'publication/publish.py', REPLAY_SHA, 'rpo_prior_publication_data')
    public_raw = X.read(PRIOR / 'result.json')
    X.require(X.extent(public_raw)['sha256'] == PRIOR_SHA, 'immutable previously published CPU result')
    public = X.parse(public_raw)
    root, old_root = args.retained_root.absolute(), args.cpu_retained_root.absolute()
    X.require(all(p.resolve(strict=True) == p and p.is_dir() for p in (root, old_root)), 'explicit retained roots')
    records_read, old_records = {}, {}
    def reader(base, checked):
        def read(path, expected=None):
            path = Path(path)
            raw = X.read(base / path.relative_to(X.E), expected)
            record = dict(path=str(path), **X.extent(raw))
            X.require(str(path) not in checked or checked[str(path)] == record, 'no changed input alias')
            checked[str(path)] = record
            return raw
        return read
    read, old_read = reader(root, records_read), reader(old_root, old_records)
    value, owner, cpu, records = D.roster(X, read, D.COMPLETE_SHA, D.OWNER_SHA)
    X.require(public['raw_completion'] == value['compiler_cpu'] and public['owner_completion'] == value['compiler_owner']
              and public['artifacts'] == value['compiler_artifacts']
              and public['source_proposal'] == value['patches']['rpo']
              and public['source_snapshots']['sources-before.json'] == value['source_snapshot'], 'published CPU identity join')
    for name in ('complete.json', 'owner-complete.json'):
        record = value['compiler_cpu'] if name == 'complete.json' else value['compiler_owner']
        X.read(PRIOR / name, record)
    archive_raw = X.read(args.archive.absolute())
    X.require(X.extent(archive_raw)['sha256'] == X.digest(args.archive_sha256), 'actual courier archive')
    manifest_raw = X.read(root / 'export-manifest.json')
    manifest = X.parse(manifest_raw)
    X.require(manifest['schema'] == 'ferric-p228-rpo-finalizer-metadata-export-v1'
              and manifest['files'] == records and manifest['original_root'] == str(X.E)
              and manifest['complete_sha256'] == D.COMPLETE_SHA and manifest['owner_sha256'] == D.OWNER_SHA
              and manifest['exporter']['sha256'] == EXPORT_SHA and manifest['data_helper']['sha256'] == D.COMMON_SHA
              and manifest['artifact_bodies_exported'] is False and manifest['full_source_tree_exported'] is False,
              'derived closed courier roster')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as archive:
        members = archive.getmembers()
        X.require(len(members) == len(records) + 1 and {m.name for m in members} == set(records) | {'export-manifest.json'}
                  and all(m.isfile() and str(X.relative(m.name)) == m.name for m in members), 'closed ordinary archive')
        for member in members:
            expected = X.extent(manifest_raw) if member.name == 'export-manifest.json' else records[member.name]
            X.require(member.size == expected['bytes'] and X.extent(archive.extractfile(member).read())
                      == {k: expected[k] for k in ('bytes', 'sha256')}, 'archive/extracted body identity')
    tests = verify(X, D, P, value, owner, cpu, read, old_read)
    pure_raw = X.read(PURE)
    pure = X.parse(pure_raw)
    X.require(X.extent(pure_raw)['sha256'] == PURE_SHA and pure['schema'] == 'ferric-p228-rpo-finalizer-v2-root-test-observation-v1'
              and pure['exit_code'] == 0 and pure['tests'] == 16 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
              and pure['manifest_sha256'] == D.PACKAGE_SHA and pure['raw_tool_result']['exit_code'] == 0, 'actual primary pure observation')
    package = Path(value['package']['path']).parent
    tree = ast.parse(read(package / 'test_run.py'))
    names = sorted(cls.name + '.' + method.name for cls in tree.body if isinstance(cls, ast.ClassDef)
                   for method in cls.body if isinstance(method, ast.FunctionDef) and method.name.startswith('test_'))
    log = pure['raw_tool_result']['output'].replace('\r\n', '\n')
    actual = sorted(cls + '.' + method for method, cls in re.findall(
        r'^(test_\w+) \(__main__[.](\w+)(?:[.]test_\w+)?\) \.\.\. ok$', log, re.M))
    X.require(len(names) == 16 and actual == names and re.search(r'\nRan 16 tests in [0-9.]+s\n\nOK\n', log),
              'all named pure cases, separate primary observation')
    copies = {}
    def copy(name, raw, original):
        X.relative(name)
        X.require(name not in copies, 'unique publication member')
        copies[name] = (raw, original)
    for path, name in ((D.ROW / 'complete.json', 'complete.json'), (D.OWNER / 'complete.json', 'owner-complete.json')):
        raw = read(path)
        copy(name, raw, dict(path=str(path), **X.extent(raw)))
    for phase in sorted(D.PHASES):
        for suffix in X.SUFFIXES:
            name = phase + '-' + suffix
            copy('raw/' + name, read(D.ROW / name, value['raw'][name]), value['raw'][name])
    for name in sorted(X.OWNER_FILES - {'complete.json', 'after.json', 'old-targets-before.json'}):
        path = D.OWNER / name
        raw = read(path)
        copy('owner/' + name, raw, dict(path=str(path), **X.extent(raw)))
    for record in records.values():
        path = Path(record['path'])
        if path.is_relative_to(package):
            copy('controller/' + str(path.relative_to(package)), read(path, record), record)
    copy('pure-root-observation.json', pure_raw, dict(path=str(PURE), **X.extent(pure_raw)))
    for name in ('export.py', 'publish.py'):
        raw = X.read(HERE / name)
        copy('publication/' + name, raw, dict(path=str(HERE / name), **X.extent(raw)))
    ledger = [dict(path=name, original=original, **X.extent(raw)) for name, (raw, original) in sorted(copies.items())]
    summary = dict(schema='FerricPartialMoveRpoFinalizerQualificationV1', passed=True, tests=tests,
        raw_completion=owner['completion'], owner_completion=dict(path=str(D.OWNER / 'complete.json'),
        **X.extent(read(D.OWNER / 'complete.json'))), prior_finalizer=value['prior_finalizer'],
        compiler_publication=dict(path=str(PRIOR / 'result.json'), **X.extent(public_raw)),
        compiler_cpu=value['compiler_cpu'], compiler_owner=value['compiler_owner'],
        qualified_generation=value['qualified_generation'], patches=value['patches'], source_snapshot=value['source_snapshot'],
        package=value['package'], phases=value['phases'], compiler_artifacts=value['compiler_artifacts'],
        finalizer_artifacts=value['artifacts'], product_identities=8, product_bodies_rehashed=False,
        source_files_changed=False, compiler_limits_changed=False, finalizer_cpu_qualified=True,
        owned_outcome={k: owner['owned'][k] for k in ('exit_code', 'reason', 'cleanup_signalled', 'owned_groups_absent',
            'owned_processes_reaped', 'elapsed_seconds', 'deadline_seconds')},
        pure_tests=dict(tests=16, observation_kind=pure['observation_kind'], source_postcheck_replayed=False,
                        observation=dict(path=str(PURE), **X.extent(pure_raw))),
        retained_archive=dict(path=str(args.archive.absolute()), **X.extent(archive_raw)),
        retained_files=records, consumed_prior_cpu_files=list(old_records.values()), copied=ledger,
        data_helpers=[dict(path=str(PRIOR / 'publication' / name), **X.extent(X.read(PRIOR / 'publication' / name)))
                      for name in ('export.py', 'publish.py')],
        limitations=[
            'Three newly built finalizer products join five previously qualified compiler products by recorded identities.',
            'No executable/library body is transported, published or rehashed by this data-only replay.',
            'Source/dependency/tool/configuration snapshots are authenticated data; their transitive bodies are not all rehashed.',
            'All 15 historical actual-capture cases remain ignored; this default CPU suite does not run an inert capture join.',
            'The 16 controller-policy cases are a retained primary SSH-output observation, not a remote supervisor receipt.',
            'The prior CPU publication remains immutable. Source overlays are linked there, not installed by this publisher.',
            'Fresh checked RoPE lowering, actual replay joins, HSACO emission and GPU/model acceptance remain separate gates.'],
        **{k: False for k in (*X.FALSE_FIELDS, 'actual_capture_join')})
    for record in records.values():
        read(Path(record['path']), record)
    for record in list(old_records.values()):
        old_read(Path(record['path']), record)
    X.require(X.read(PRIOR / 'result.json') == public_raw and X.read(PURE) == pure_raw
              and X.read(args.archive.absolute()) == archive_raw, 'publication input postchecks')
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
        path = OUT / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
        X.require(X.extent(X.read(path)) == X.extent(raw), 'published body hash')
    raw = (json.dumps(summary, indent=2, sort_keys=True) + '\n').encode()
    with (OUT / 'result.json').open('xb') as stream:
        stream.write(raw)
    print(json.dumps(dict(output=str(OUT), files=len(copies), result=X.extent(X.read(OUT / 'result.json')))))


if __name__ == '__main__':
    main()
