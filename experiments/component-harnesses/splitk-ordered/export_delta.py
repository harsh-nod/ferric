"""CPU-only bounded delta export from three fully verified prepared bundles."""
import gzip
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import tarfile

D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
INPUTS = D / 'inputs/component-ordered-delta-a001'
OUTPUT = D / 'component-ordered-delta-a001'
GUARDED = D / 'inputs/component-ordered-bundle-a001/prepare_guarded.py'
GUARDED_SHA = 'a98f2fe0937607e7029210dbf17ebcce6ddbb6c181ab37fc456122dd60cd4be4'
QUALIFIER = D / 'inputs/component-ordered-cpu-a001/qualify_source.py'
QUALIFIER_SHA = 'b84d2f83eaffd7adf4f9aa760f88dd44d7fe4b2a58c23ea8a57d7beb3cfd0297'
ROSTER = INPUTS / 'delta-roster.json.gz'
ROSTER_GZIP_SHA = '7c1fe31e327ecc7439580a350ce215b94b05f4ba21454c132d2c2578968b4f82'
ROSTER_SHA = '864b0b6b9ac934ba2a6610112bbf18e0fddac0146e286871733ec6dd356ccf36'
ENVELOPE = 64 * 1024**2


def require(value, message):
    if not value:
        raise ValueError(message)


def load(path, expected):
    require(path.resolve(strict=True) == path and path.stat().st_size < 1024**2
            and hashlib.sha256(path.read_bytes()).hexdigest() == expected, 'exact existing helper')
    spec = importlib.util.spec_from_file_location('delta_' + path.stem, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main():
    q = load(QUALIFIER, QUALIFIER_SHA)
    q.environment()
    guarded = load(GUARDED, GUARDED_SHA)
    before_allocation = q.allocation(ENVELOPE)
    compressed, _ = q.read(ROSTER, ROSTER_GZIP_SHA)
    with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:
        raw = stream.read(1024**2 + 1)
    require(len(raw) == 277902 and hashlib.sha256(raw).hexdigest() == ROSTER_SHA,
            'exact complete restore roster')
    roster = json.loads(raw)
    require(roster['schema'] == 'FerricOrderedComponentDeltaRestoreV1'
            and set(roster['modes']) == {'latency', 'counters', 'ticks'}
            and len(roster['objects']) == 35, 'fixed three-mode delta')
    sources = {}
    for mode, value in roster['modes'].items():
        root = D / ('component-ordered-' + mode + '-prepared-a001/payload')
        expected = value['files']
        require(len(expected) == 240, 'complete240-member prepared bundle')
        actual = set()
        for parent, directories, names in os.walk(root, followlinks=False):
            for name in directories:
                info = (Path(parent) / name).lstat()
                require(stat.S_ISDIR(info.st_mode) and info.st_uid == 1046, 'owned real source directory')
            for name in names:
                relative = str((Path(parent) / name).relative_to(root))
                require(relative in expected, 'unexpected prepared artifact')
                actual.add(relative)
        require(actual == set(expected), 'exact prepared artifact inventory')
        for name, item in expected.items():
            path = root / name
            info = guarded.file_hash(path, item['sha256'])
            require(info['st_size'] == item['bytes'] and stat.S_IMODE(info['st_mode']) == item['mode'],
                    'exact prepared file size/mode')
            sources[str(path)] = info
    require(not OUTPUT.exists() and not OUTPUT.is_symlink(), 'create-only delta export')
    OUTPUT.mkdir(mode=0o700)
    archive_path = OUTPUT / 'ordered-delta.tar.gz'
    with archive_path.open('xb') as target, tarfile.open(fileobj=target, mode='w:gz') as archive:
        def add(name, value):
            item = tarfile.TarInfo(name)
            item.size, item.mode, item.mtime = len(value), 0o600, 0
            archive.addfile(item, io.BytesIO(value))
        add('delta-roster.json', raw)
        total = 0
        for digest, item in sorted(roster['objects'].items()):
            require(item['source'] in sources and sources[item['source']]['sha256'] == digest,
                    'delta object comes from complete verified bundle')
            path = Path(item['source'])
            value = path.read_bytes()
            require(len(value) == item['bytes'] and hashlib.sha256(value).hexdigest() == digest,
                    'delta object identity')
            total += len(value)
            require(total <= 3 * 1024**2, 'delta input byte bound')
            add('objects/' + digest, value)
    require(archive_path.stat().st_size <= 4 * 1024**2, 'compressed delta bound')
    after = {path: guarded.file_hash(Path(path), item['sha256']) for path, item in sources.items()}
    require(sources == after, 'complete prepared source roster changed across export')
    q.read(ROSTER, ROSTER_GZIP_SHA)
    guarded.file_hash(GUARDED, GUARDED_SHA)
    guarded.file_hash(QUALIFIER, QUALIFIER_SHA)
    q.environment()
    after_allocation = q.allocation()
    require(after_allocation['stage_allocated_bytes'] - before_allocation['stage_allocated_bytes'] <= ENVELOPE,
            'delta export stage growth')
    receipt = {'schema': 'FerricOrderedComponentDeltaExportV1', 'roster_sha256': ROSTER_SHA,
        'archive_sha256': hashlib.sha256(archive_path.read_bytes()).hexdigest(),
        'archive_bytes': archive_path.stat().st_size, 'objects': 35, 'object_bytes': total,
        'prepared_source_files': len(sources), 'source_unchanged': True,
        'allocation_before': before_allocation, 'allocation_after': after_allocation,
        'build_executed': False, 'tests_executed': False, 'native_executed': False}
    with (OUTPUT / 'export.json').open('x') as stream:
        json.dump(receipt, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(receipt, sort_keys=True))


if __name__ == '__main__':
    main()
