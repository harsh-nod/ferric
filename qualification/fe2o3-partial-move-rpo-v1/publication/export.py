"""Data-only courier for the successful RPO CPU V2 records; no target products."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import stat
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
CPU = E / 'rpo-compiler-cpu-v228-v2'
OWNER = E / 'rpo-compiler-cpu-owner-v228-v2'
CPU_SHA = '56fc51fc326980e00156d550d0a7052f44bb481ded7b9948c06217653fb246c1'
OWNER_SHA = 'afca99d8799f910ce607873c320b8f244be66f9ffdb3df4dce4545eb1552ea66'
PACKAGE_SHA = '9fe67c55e38e4713c692abb4e145cd58bcea81ecf162879de57568b81d729603'
PATCH_SHA = '939b76eb28df8d7e2b53ae0b4f5034257185f12f077d20e581d9880b006a252c'
PHASES = {'rustfmt', 'rustfmt-check', 'metadata', 'compiler-build'} | {
    role + '-' + suffix for role in ('pliron', 'compiler')
    for suffix in ('build-tests', 'list', 'ignored-list', 'tests')}
SUFFIXES = ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')
SNAPSHOTS = {'configurations-after.json', 'dependencies-after.json', 'dependencies-before.json',
             'inputs-after.json', 'inputs-before.json', 'original-after.json', 'original-before.json',
             'qualified-generation.json', 'sources-after.json', 'sources-before.json',
             'sources-unformatted.json'}
OWNER_FILES = {'complete.json', 'command.json', 'started.json', 'stdout', 'stderr',
               'owned-result.json', 'after.json', 'old-targets-before.json'}
FALSE_FIELDS = ('checked_lowering', 'fresh_hsaco_emitted', 'gpu_execution',
                'production_authority', 'numerical_acceptance', 'performance_claim',
                'full_model_acceptance')


def require(ok, message):
    if not ok:
        raise ValueError(message)


def extent(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def digest(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value), 'SHA256')
    return value


def relative(value):
    path = Path(value)
    require(type(value) is str and not path.is_absolute() and str(path) == value
            and value not in ('', '.') and '..' not in path.parts, 'canonical relative path')
    return path


def pin(record):
    require(type(record) is dict and set(record) == {'path', 'bytes', 'sha256'}, 'FilePin shape')
    require(type(record['bytes']) is int and record['bytes'] >= 0, 'FilePin extent')
    digest(record['sha256'])
    path = Path(record['path'])
    require(path.is_absolute() and str(path) == record['path'] and '..' not in path.parts,
            'canonical original path')
    return record


def parse(raw):
    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(path, expected=None, cap=32 << 20):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical local path')
    before = path.stat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= cap,
            'bounded ordinary unaliased file: ' + str(path))
    raw = path.read_bytes()
    after = path.stat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(after) and len(raw) == before.st_size, 'read changed')
    if expected is not None:
        require(extent(raw) == {k: expected[k] for k in ('bytes', 'sha256')}, 'body pin: ' + str(path))
    return raw


def success(value, schema):
    require(value['schema'] == schema and value['passed'] is True and value['error'] is None
            and value['postcheck_errors'] == [], 'successful terminal record')


def roster(reader, cpu_sha, owner_sha):
    """Derive an exact original-path roster from authenticated completed records."""
    require(digest(cpu_sha) == CPU_SHA and digest(owner_sha) == OWNER_SHA, 'actual CPU/owner identity')
    records = {}
    def add(path, expected=None):
        path = Path(path)
        name = str(path.relative_to(E))
        relative(name)
        require('/target/' not in '/' + name + '/' and not name.endswith(('.so', '.rlib', '.rmeta')),
                'no build products in courier')
        raw = reader(path, expected)
        record = dict(path=str(path), **extent(raw))
        require(name not in records or records[name] == record, 'conflicting courier alias')
        records[name] = record
        return raw
    cpu_raw = add(CPU / 'complete.json')
    owner_raw = add(OWNER / 'complete.json')
    require(extent(cpu_raw)['sha256'] == cpu_sha and extent(owner_raw)['sha256'] == owner_sha,
            'actual completed bodies')
    cpu, owner = parse(cpu_raw), parse(owner_raw)
    success(cpu, 'fe2o3-p228-rpo-compiler-cpu-result-v1')
    success(owner, 'fe2o3-p228-rpo-compiler-owned-result-v1')
    require(owner['completion'] == records[str((CPU / 'complete.json').relative_to(E))], 'owner CPU join')
    require(owner['patch'] == cpu['patch'] and owner['qualified_generation'] == cpu['qualified_generation'],
            'owner source generation')
    require(all(cpu[k] is False for k in FALSE_FIELDS) and cpu['fresh_compiler_built'] is True
            and cpu['source_unchanged'] is True, 'CPU claim scope')
    require(set(cpu['phases']) == PHASES, 'twelve CPU phases')
    expected_raw = SNAPSHOTS | {p + '-' + s for p in PHASES for s in SUFFIXES}
    require(set(cpu['raw']) == expected_raw and len(expected_raw) == 71, 'closed CPU raw roster')
    for name, record in cpu['raw'].items():
        pin(record)
        require(record['path'] == str(CPU / name), 'CPU raw namespace')
        add(Path(record['path']), record)
    for name in sorted(OWNER_FILES):
        add(OWNER / name)
    require(cpu['package']['sha256'] == PACKAGE_SHA and cpu['patch']['sha256'] == PATCH_SHA,
            'frozen package/proposal')
    package = parse(add(Path(cpu['package']['path']), pin(cpu['package'])))
    require(package['schema'] == 'fe2o3-p228-rpo-compiler-cpu-package-v2'
            and len(package['files']) == 3
            and {r['path'] for r in package['files']} == {'run.py', 'test_run.py', 'README.md'},
            'closed controller package')
    for row in package['files']:
        add(Path(cpu['package']['path']).parent / relative(row['path']), row)
    proposal = parse(add(Path(cpu['patch']['path']), pin(cpu['patch'])))
    require(proposal['schema'] == 'ferric-p228-partial-move-rpo-source-proposal-v2'
            and len(proposal['files']) == 4, 'four-source proposal')
    for row in proposal['files']:
        add(Path(cpu['patch']['path']).parent / relative(row['source']), row['after'])
    for row in proposal['retained_files']:
        add(Path(cpu['patch']['path']).parent / relative(row['path']), row)
    require(set(cpu['formatted_sources']) == {r['path'] for r in proposal['files']}, 'four formatted sources')
    for name, record in cpu['formatted_sources'].items():
        require(pin(record)['path'] == str(CPU / 'source/fe2o3' / relative(name)), 'formatted source path')
        add(Path(record['path']), record)
    for record in cpu['qualified_generation'].values():
        add(Path(pin(record)['path']), record)
    require(len(records) <= 128 and sum(r['bytes'] for r in records.values()) <= 96 << 20,
            'metadata courier budget')
    return cpu, owner, records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('cpu_sha256')
    parser.add_argument('owner_sha256')
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    out = args.archive.absolute()
    require(out.parent == E and re.fullmatch('rpo-cpu-publication-evidence-v228-v[1-9][0-9]*[.]tar[.]gz', out.name)
            and not os.path.lexists(out), 'fresh closed archive path')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 30 << 20), (resource.RLIMIT_CORE, 0)):
        hard = resource.getrlimit(kind)[1]
        limit = cap if hard == resource.RLIM_INFINITY else min(cap, hard)
        resource.setrlimit(kind, (limit, limit))
    _, _, records = roster(read, args.cpu_sha256, args.owner_sha256)
    manifest = dict(schema='ferric-p228-rpo-cpu-metadata-export-v1', original_root=str(E),
                    cpu_sha256=args.cpu_sha256, owner_sha256=args.owner_sha256, files=records,
                    exporter=dict(path=str(Path(__file__).resolve()), **extent(read(Path(__file__).resolve()))),
                    artifact_bodies_exported=False, full_source_tree_exported=False)
    with out.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for name, record in sorted(records.items()):
            raw = read(Path(record['path']), record)
            member = tarfile.TarInfo(name)
            member.size, member.mode = len(raw), 0o600
            archive.addfile(member, io.BytesIO(raw))
        raw = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode('ascii')
        member = tarfile.TarInfo('export-manifest.json')
        member.size, member.mode = len(raw), 0o600
        archive.addfile(member, io.BytesIO(raw))
    for record in records.values():
        read(Path(record['path']), record)
    print(json.dumps(dict(archive=dict(path=str(out), **extent(read(out))), files=len(records),
                          uncompressed_bytes=sum(r['bytes'] for r in records.values()),
                          artifact_bodies_exported=False, full_source_tree_exported=False)))


if __name__ == '__main__':
    main()
