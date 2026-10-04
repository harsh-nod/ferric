"""Retain only the actual RPO finalizer records and controller, not products."""
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

COMMON_SHA = '5ba8484263ac02051a06b488f0db6b9565f6cff31d189c8dd35f07d90b08b720'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROW = E / 'rpo-finalizer-tools-v228-v2'
OWNER = E / 'rpo-finalizer-tools-owner-v228-v2'
COMPLETE_SHA = '39eab92ede34991b08c168e827c7111c13533627cbf2efd3d248c6742a92660b'
OWNER_SHA = 'e919fb520672be2a909fd0ec8301bc9923210680f818e411b7db459ffc3059dd'
PACKAGE_SHA = 'd014c66154afefa324bb0f5a49bacfb034229e840efb012acd02c950d5ce0178'
PHASES = {'metadata', 'finalizer-build', 'finalizer-build-tests', 'finalizer-list',
          'finalizer-ignored-list', 'finalizer-tests'}
SNAPSHOTS = {'before.json', 'configurations-after.json', 'dependencies-after.json',
             'files-after.json', 'inputs-after.json', 'sources-after.json'}


def common(path):
    path = Path(path).absolute()
    if path.resolve(strict=True) != path or not path.is_file():
        raise ValueError('canonical common data helper')
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != COMMON_SHA:
        raise ValueError('common data helper hash')
    value = types.ModuleType('rpo_finalizer_publication_common')
    value.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), value.__dict__)
    value.read(path, dict(bytes=len(raw), sha256=COMMON_SHA))
    return value


def roster(X, reader, complete_sha, owner_sha):
    X.require(X.digest(complete_sha) == COMPLETE_SHA and X.digest(owner_sha) == OWNER_SHA,
              'actual finalizer completion identities')
    records = {}
    def add(path, expected=None):
        path = Path(path)
        name = str(X.relative(str(path.relative_to(E))))
        X.require('/target/' not in '/' + name + '/' and not name.endswith(('.so', '.rlib', '.rmeta')),
                  'no target product in metadata courier')
        raw = reader(path, expected)
        record = dict(path=str(path), **X.extent(raw))
        X.require(name not in records or records[name] == record, 'conflicting retained alias')
        records[name] = record
        return raw
    raw, outer_raw = add(ROW / 'complete.json'), add(OWNER / 'complete.json')
    X.require(X.extent(raw)['sha256'] == complete_sha and X.extent(outer_raw)['sha256'] == owner_sha,
              'actual finalizer completed bodies')
    value, owner = X.parse(raw), X.parse(outer_raw)
    X.success(value, 'fe2o3-p228-rpo-finalizer-tools-result-v1')
    X.success(owner, 'fe2o3-p228-rpo-finalizer-tools-owned-result-v1')
    X.require(owner['completion'] == records[str((ROW / 'complete.json').relative_to(E))]
              and all(owner[k] == value[k] for k in ('compiler_cpu', 'compiler_owner', 'patches', 'qualified_generation')),
              'finalizer owner source joins')
    X.require(all(value[k] is False for k in (*X.FALSE_FIELDS, 'actual_capture_join')),
              'no lowering/emission/capture/GPU/numerical authority')
    X.require(set(value['phases']) == PHASES and set(value['raw']) == SNAPSHOTS | {
        phase + '-' + suffix for phase in PHASES for suffix in X.SUFFIXES}, 'six phases / 36 raw records')
    for name, record in value['raw'].items():
        X.require(X.pin(record)['path'] == str(ROW / name), 'finalizer raw namespace')
        add(Path(record['path']), record)
    for name in sorted(X.OWNER_FILES):
        add(OWNER / name)
    X.require(value['compiler_cpu']['sha256'] == X.CPU_SHA
              and value['compiler_owner']['sha256'] == X.OWNER_SHA, 'exact qualified compiler')
    for record in (value['compiler_cpu'], value['compiler_owner'], value['prior_finalizer'], *value['patches'].values()):
        add(Path(X.pin(record)['path']), record)
    cpu = X.parse(reader(Path(value['compiler_cpu']['path']), value['compiler_cpu']))
    X.require(value['compiler_artifacts'] == cpu['artifacts'] and value['source_snapshot'] == cpu['raw']['sources-before.json']
              and value['compiler_required_test_names'] == cpu['required_test_names']
              and value['qualified_generation'] == cpu['qualified_generation']
              and value['patches'] == {'rpo': cpu['patch']}, 'unchanged compiler products/source generation')
    X.require(value['package']['sha256'] == PACKAGE_SHA, 'frozen finalizer package')
    package = X.parse(add(Path(value['package']['path']), X.pin(value['package'])))
    X.require(package['schema'] == 'fe2o3-p228-rpo-finalizer-tools-package-v2'
              and len(package['files']) == 3 and {r['path'] for r in package['files']} == {'run.py', 'test_run.py', 'README.md'},
              'closed finalizer controller')
    for row in package['files']:
        add(Path(value['package']['path']).parent / X.relative(row['path']), row)
    X.require(len(records) <= 64 and sum(r['bytes'] for r in records.values()) <= 64 << 20, 'bounded metadata export')
    return value, owner, cpu, records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('common_helper', type=Path)
    parser.add_argument('complete_sha256')
    parser.add_argument('owner_sha256')
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    X = common(args.common_helper)
    out = args.archive.absolute()
    X.require(out.parent == E and re.fullmatch('rpo-finalizer-publication-evidence-v228-v[1-9][0-9]*[.]tar[.]gz', out.name)
              and not os.path.lexists(out), 'fresh closed archive output')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 30 << 20), (resource.RLIMIT_CORE, 0)):
        hard = resource.getrlimit(kind)[1]
        limit = cap if hard == resource.RLIM_INFINITY else min(cap, hard)
        resource.setrlimit(kind, (limit, limit))
    _, _, _, records = roster(X, X.read, args.complete_sha256, args.owner_sha256)
    manifest = dict(schema='ferric-p228-rpo-finalizer-metadata-export-v1', original_root=str(E), files=records,
        complete_sha256=args.complete_sha256, owner_sha256=args.owner_sha256,
        exporter=dict(path=str(Path(__file__).resolve()), **X.extent(X.read(Path(__file__).resolve()))),
        data_helper=dict(path=str(args.common_helper.absolute()), **X.extent(X.read(args.common_helper.absolute()))),
        artifact_bodies_exported=False, full_source_tree_exported=False)
    with out.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for name, record in sorted(records.items()):
            raw = X.read(Path(record['path']), record)
            member = tarfile.TarInfo(name)
            member.size, member.mode = len(raw), 0o600
            archive.addfile(member, io.BytesIO(raw))
        raw = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode('ascii')
        member = tarfile.TarInfo('export-manifest.json')
        member.size, member.mode = len(raw), 0o600
        archive.addfile(member, io.BytesIO(raw))
    for record in records.values():
        X.read(Path(record['path']), record)
    print(json.dumps(dict(archive=dict(path=str(out), **X.extent(X.read(out))), files=len(records),
                          uncompressed_bytes=sum(r['bytes'] for r in records.values()), artifact_bodies_exported=False)))


if __name__ == '__main__':
    main()
