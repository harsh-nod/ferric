"""Data-only courier for a completed expected-refusal diagnostic, not its ELF."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import tarfile
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
CASE = E / 'kir-join-work-diagnostic-cpu-v228-v1'
OWNER = E / 'kir-join-work-diagnostic-cpu-owner-v228-v1'
COMMON_SHA = '5ba8484263ac02051a06b488f0db6b9565f6cff31d189c8dd35f07d90b08b720'
SOURCE_SHA = '2dc0563b1e09531aed6ced32cc0c9fa839ed7e66215d4072d7a2bd1023806c13'
CONTROLLER = {'run.py': 'b15343d2fb1a244ddd64d87e71f9a683027684d8239ecafcebeac42276c18e73',
    'test_run.py': 'eeae0bd8612a70e0b5f1ef78d20aa72ddfac361e0d21fcfd2b75b744a9955f62',
    'README.md': 'c27eda0a2b476ac3635ae1f9019ad701b02f539108dcf76bd6e69005d46ff6e2'}
PHASES = {'rustfmt', 'rustfmt-check', 'metadata', 'finalizer-build-tests', 'finalizer-list',
    'finalizer-ignored-list', 'finalizer-tests', 'actual-inert-join-diagnostic'}
SNAPSHOTS = {'sources-unformatted.json', 'configurations-before.json', 'sources-before.json',
    'dependencies-before.json', 'original-after.json', 'sources-after.json', 'inputs-after.json',
    'prior-dependencies-after.json', 'configurations-after.json', 'dependencies-after.json'}


def common(path):
    path = path.absolute()
    if path.resolve(strict=True) != path or not path.is_file():
        raise ValueError('canonical data helper')
    raw = path.read_bytes()
    if len(raw) > 1 << 20 or hashlib.sha256(raw).hexdigest() != COMMON_SHA:
        raise ValueError('exact data helper')
    module = types.ModuleType('kir_diagnostic_publication_common')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    module.read(path, dict(bytes=len(raw), sha256=COMMON_SHA))
    return module


def roster(X, reader, complete_sha, owner_sha):
    records, snapshots = {}, {}

    def add(path, expected=None, retained=True):
        path = Path(path)
        name = str(X.relative(str(path.relative_to(E))))
        X.require('/target/' not in '/' + name + '/' and not path.suffix in ('.so', '.rlib', '.rmeta'),
                  'no build product body')
        raw = reader(path, expected)
        pin = dict(path=str(path), **X.extent(raw))
        target = records if retained else snapshots
        X.require(name not in target or target[name] == pin, 'unambiguous original identity')
        target[name] = pin
        return raw

    raw = add(CASE / 'complete.json')
    outer = add(OWNER / 'complete.json')
    X.require(X.extent(raw)['sha256'] == X.digest(complete_sha)
              and X.extent(outer)['sha256'] == X.digest(owner_sha), 'root-supplied actual terminal pins')
    value, owner = X.parse(raw), X.parse(outer)
    X.success(value, 'ferric-p228-kir-join-work-diagnostic-cpu-result-v1')
    X.success(owner, 'ferric-p228-kir-join-work-diagnostic-owned-result-v1')
    X.require(owner['completion'] == records[str((CASE / 'complete.json').relative_to(E))]
              and owner['package'] == value['package'] and owner['proposal'] == value['proposal'],
              'actual owner/diagnostic/source join')
    X.require(value['diagnostic_completed'] is True and value['source_unchanged'] is True
              and all(value[key] is False for key in ('limits_changed', 'actual_capture_join_passed',
                  'fresh_compiler_built', 'fresh_hsaco_emitted', 'gpu_execution', 'numerical_acceptance',
                  'production_authority', 'performance_claim')), 'expected-refusal-only scope')
    raw_names = {phase + '-' + suffix for phase in PHASES for suffix in X.SUFFIXES}
    X.require(set(value['phases']) == PHASES and set(value['raw']) == raw_names | SNAPSHOTS,
              'eight phase leaves plus exact snapshot roster')
    for name, pin in value['raw'].items():
        X.require(X.pin(pin)['path'] == str(CASE / name), 'original raw namespace')
        add(pin['path'], pin, name not in SNAPSHOTS)
    for name in sorted(X.OWNER_FILES):
        add(OWNER / name)
    package = X.parse(add(X.pin(value['package'])['path'], value['package']))
    X.require(package['schema'] == 'ferric-p228-kir-join-work-diagnostic-cpu-package-v1'
              and len(package['files']) == 3 and {row['path'] for row in package['files']} == set(CONTROLLER),
              'closed controller package')
    for row in package['files']:
        X.require(row['sha256'] == CONTROLLER[row['path']], 'reviewed controller source')
        add(Path(value['package']['path']).parent / X.relative(row['path']), row)
    X.require(value['proposal']['sha256'] == SOURCE_SHA, 'reviewed source proposal')
    proposal = X.parse(add(X.pin(value['proposal'])['path'], value['proposal']))
    X.require(proposal['schema'] == 'ferric-p228-kir-join-work-diagnostic-source-v1'
              and len(proposal['files']) == 3
              and set(value['formatted_sources']) == {row['path'] for row in proposal['files']},
              'three diagnostic source bodies')
    for row in proposal['files']:
        X.require(row['source'] == 'draft/' + row['path'], 'exact draft mapping')
        add(Path(value['proposal']['path']).parent / X.relative(row['source']), row['after'])
        pin = X.pin(value['formatted_sources'][row['path']])
        X.require(pin['path'] == str(CASE / 'source/fe2o3' / X.relative(row['path'])), 'formatted source mapping')
        add(pin['path'], pin)
    add(Path(value['proposal']['path']).parent / 'README.md', dict(bytes=7228,
        sha256='433ab829385eaf08cffdfd937b6fa4c721dadbd230cc4cf5b3f9e7aff67ced89'))
    X.require(len(records) <= 64 and sum(pin['bytes'] for pin in records.values()) <= 32 << 20,
              'small evidence courier')
    return value, owner, records, snapshots


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('common_helper', type=Path)
    parser.add_argument('complete_sha256')
    parser.add_argument('owner_sha256')
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    X = common(args.common_helper)
    out = args.archive.absolute()
    X.require(out.parent == E and re.fullmatch('kir-join-work-diagnostic-evidence-v228-v[1-9][0-9]*[.]tar[.]gz', out.name)
              and not os.path.lexists(out), 'fresh bounded archive')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 30 << 20), (resource.RLIMIT_CORE, 0)):
        hard = resource.getrlimit(kind)[1]
        limit = cap if hard == resource.RLIM_INFINITY else min(cap, hard)
        resource.setrlimit(kind, (limit, limit))
    _, _, records, snapshots = roster(X, X.read, args.complete_sha256, args.owner_sha256)
    manifest = dict(schema='ferric-p228-kir-join-work-diagnostic-export-v1', original_root=str(E),
        complete_sha256=args.complete_sha256, owner_sha256=args.owner_sha256, files=records,
        remotely_rehashed_snapshot_pins=snapshots, snapshot_bodies_exported=False,
        artifact_bodies_exported=False, full_source_tree_exported=False,
        exporter=dict(path=str(Path(__file__).resolve()), **X.extent(X.read(Path(__file__).resolve()))))
    with out.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for name, pin in sorted(records.items()):
            raw = X.read(Path(pin['path']), pin)
            member = tarfile.TarInfo(name)
            member.size, member.mode = len(raw), 0o600
            archive.addfile(member, io.BytesIO(raw))
        raw = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode('ascii')
        member = tarfile.TarInfo('export-manifest.json')
        member.size, member.mode = len(raw), 0o600
        archive.addfile(member, io.BytesIO(raw))
    for pin in (*records.values(), *snapshots.values()):
        X.read(Path(pin['path']), pin)
    print(json.dumps(dict(archive=dict(path=str(out), **X.extent(X.read(out))), files=len(records),
        snapshots_rehashed_not_exported=len(snapshots), artifact_bodies_exported=False)))


if __name__ == '__main__':
    main()
