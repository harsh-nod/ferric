"""Export exact V19 diagnostic inputs, reusing hash-bound native width objects."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import tarfile

ENABLED = True
D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
QUALIFIER = D / 'inputs/packet-v19-harness-a002/prepare_qualification.py'
QUALIFIER_SHA = '6d42abce25b1cd22f434a615d68847ad9ff561eec67223d3f044ce077223726c'
ARCHIVE_SHA = '276641563537c905da2e9e0654a86658ccfb33be0c7bd9d079d9ab89cd0de6dc'
BOUND = D / 'packet-v19-binding-review-a002/bound'
BUILD_SHA = '313706298e6c622bff51f4630e6715e735e4b8b7b8b7038b36a527f276448a30'
WIDTH = D / 'native32-width-transport-a001'
WIDTH_ROSTER_SHA = '6f1a54afbfa7d8d3c3ff755584454cd5be968cdd5776847777917e87598f826b'
WIDTH_NATIVE = Path('/dev/shm/ferric-prefill-width-inputs-a001')
OUT = D / 'packet-v19-native-transport-a002'
NATIVE = Path('/dev/shm/ferric-packet-v19-inputs-a001')
ALLOWANCE = 128 * 1024**2


def require(value, message):
    if not value:
        raise ValueError(message)


def main():
    require(ENABLED, 'disabled until source review')
    os.umask(0o077)
    require(QUALIFIER.resolve(strict=True) == QUALIFIER, 'fixed canonical qualifier')
    info = QUALIFIER.lstat()
    require(stat.S_ISREG(info.st_mode) and info.st_uid == 1046 and info.st_nlink == 1
            and 0 < info.st_size <= 64 * 1024, 'owned qualifier')
    raw = QUALIFIER.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == QUALIFIER_SHA, 'qualified source helper')
    q = importlib.util.module_from_spec(importlib.util.spec_from_file_location('export_custody', QUALIFIER))
    exec(compile(raw, str(QUALIFIER), 'exec'), q.__dict__)
    q.ARCHIVE_SHA = ARCHIVE_SHA
    q.environment()
    source_files, sources, _ = q.payload()
    require(q.check_sources(source_files) == sources, 'qualified source before export')
    before = q.allocation(ALLOWANCE)
    contract = q.ROOT / 'harness/launch_contract.py'
    c = importlib.util.module_from_spec(importlib.util.spec_from_file_location('export_contract', contract))
    exec(compile(source_files['harness/launch_contract.py'], str(contract), 'exec'), c.__dict__)
    inventory = {}

    def read(path, expected=None, maximum=64 * 1024**2, empty=False):
        path = Path(path)
        require(path.is_relative_to(D), 'private source input')
        data, digest = c.read(path, expected, maximum, empty=empty)
        item = path.lstat()
        require(item.st_uid == 1046 and item.st_nlink == 1, 'owned single-link export input')
        record = (digest, item.st_dev, item.st_ino, item.st_size, item.st_mtime_ns, item.st_ctime_ns)
        require(str(path) not in inventory or inventory[str(path)] == record, 'unchanged duplicate input')
        inventory[str(path)] = record
        return data, digest

    original = c.decode(read(BOUND / 'build.json', BUILD_SHA)[0])
    cpu_raw = read(original['cpu_qualification']['path'], original['cpu_qualification']['sha256'])[0]
    original_cpu = c.decode(cpu_raw)
    c.validate_build(original)
    counts = c.validate_cpu(original_cpu, original,
        read_bound=lambda value: c.decode(read(value['path'], value['sha256'])[0]), read_raw=read)
    width = c.decode(read(WIDTH / 'transport-roster.json', WIDTH_ROSTER_SHA, 1024**2)[0])
    require(width['native_root'] == str(WIDTH_NATIVE), 'fixed native reuse root')
    require(not os.path.lexists(OUT), 'create-only export')
    OUT.mkdir(mode=0o700)
    payload = OUT / 'payload'
    payload.mkdir(mode=0o700)
    files, reused, local_paths = {}, {}, {}

    def create(name, data, executable=False):
        c.relative(name)
        digest = hashlib.sha256(data).hexdigest()
        mode = 0o700 if executable else 0o600
        record = {'sha256': digest, 'bytes': len(data), 'mode': mode}
        if name in files:
            require(files[name] == record, 'exact duplicate object mode/content')
            return {'path': str(NATIVE / name), 'sha256': digest}
        require(len(files) < 512 and sum(item['bytes'] for item in files.values()) + len(data)
                <= 96 * 1024**2, 'bounded complete input roster')
        if name.startswith('objects/') and width['files'].get(name) == record:
            require(read(WIDTH / 'payload' / name, digest, empty=not data)[0] == data,
                    'reused byte-identical frozen object')
            reused[name] = str(WIDTH_NATIVE / name)
            local_paths[name] = WIDTH / 'payload' / name
        else:
            path = payload / name
            path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode)
            with os.fdopen(descriptor, 'wb') as target:
                target.write(data)
            local_paths[name] = path
        files[name] = record
        return {'path': str(NATIVE / name), 'sha256': digest}

    executables = {original['worker']['sha256'], original['controllers']['diagnostic']['sha256'],
                   original_cpu['v19_evidence']['baseline_controller']['sha256']}

    def portable(value):
        if type(value) is dict:
            if set(value) == {'path', 'sha256'}:
                c.binding(value)
                data, digest = read(value['path'], value['sha256'], empty=True)
                return create('objects/' + digest, data, digest in executables)
            return {key: portable(item) for key, item in value.items()}
        if type(value) is list:
            return [portable(item) for item in value]
        return value

    cpu = portable(original_cpu)
    build = {key: portable(value) for key, value in original.items() if key != 'cpu_qualification'}
    for name, data in source_files.items():
        create(name, data)
    create('evidence/original-build.json', c.encoded(original))
    create('evidence/original-cpu.json', cpu_raw)
    build['cpu_qualification'] = create('cpu.json', c.encoded(cpu))
    build_binding = create('build.json', c.encoded(build))

    def portable_read(path, expected=None, maximum=64 * 1024**2, empty=False):
        name = str(Path(path).relative_to(NATIVE))
        require(name in local_paths, 'closed portable input')
        return c.read(local_paths[name], expected, maximum, empty=empty)

    original_module = c.module
    qualifier_bindings = {cpu['controller_evidence']['qualifier']['path']:
        cpu['controller_evidence']['qualifier']['sha256'],
        cpu['v19_evidence']['qualifier']['path']: cpu['v19_evidence']['qualifier']['sha256']}

    def portable_module(path, expected):
        if Path(path).is_relative_to(NATIVE):
            require(qualifier_bindings.get(str(path)) == expected, 'only two pinned portable qualifier modules')
            name = str(Path(path).relative_to(NATIVE))
            portable_read(path, expected, 1024**2)
            return original_module(local_paths[name], expected)
        return original_module(path, expected)

    c.validate_build(build)
    c.module = portable_module
    try:
        require(c.validate_cpu(cpu, build,
            read_bound=lambda value: c.decode(portable_read(value['path'], value['sha256'])[0]),
            read_raw=portable_read) == counts, 'portable evidence replay unchanged')
    finally:
        c.module = original_module
    for name, item in files.items():
        portable_read(NATIVE / name, item['sha256'], empty=item['bytes'] == 0)
    for path, record in tuple(inventory.items()):
        read(path, record[0], empty=record[3] == 0)
    require(q.check_sources(source_files) == sources, 'qualified source after export')
    roster = {'schema': 'FerricV19Packet55cPortableInputsV1', 'native_root': str(NATIVE),
        'files': files, 'reuse': reused, 'build': build_binding,
        'original_build_sha256': BUILD_SHA, 'source_archive_sha256': ARCHIVE_SHA,
        'width_roster_sha256': WIDTH_ROSTER_SHA, 'native_executed': False}
    roster_raw = c.encoded(roster)
    with (OUT / 'transport-roster.json').open('xb') as target:
        target.write(roster_raw)
    archive = OUT / 'native-delta.tar.gz'
    with tarfile.open(archive, mode='x:gz', compresslevel=1) as target:
        for name in sorted(set(files) - set(reused)):
            target.add(payload / name, arcname=name, recursive=False)
    require(archive.stat().st_size <= 48 * 1024**2, 'bounded archive')
    q.environment()
    after = q.allocation()
    require(after['stage_allocated_bytes'] - before['stage_allocated_bytes'] <= ALLOWANCE,
            'fixed export allocation envelope')
    receipt = {'schema': 'FerricV19Packet55cPortableExportV1', 'native_executed': False,
        'files': len(files), 'reused_files': len(reused),
        'logical_bytes': sum(item['bytes'] for item in files.values()),
        'delta_bytes': sum(item['bytes'] for name, item in files.items() if name not in reused),
        'archive': {'path': str(archive), 'sha256': hashlib.sha256(archive.read_bytes()).hexdigest(),
                    'bytes': archive.stat().st_size},
        'roster_sha256': hashlib.sha256(roster_raw).hexdigest(), 'build': build_binding,
        'source_before': sources, 'source_after': sources, 'executed_tests': counts,
        'allocation_before': before, 'allocation_after': after, 'allowance_bytes': ALLOWANCE}
    with (OUT / 'export-receipt.json').open('xb') as target:
        target.write(c.encoded(receipt))
    print(c.encoded(receipt).decode(), end='')


if __name__ == '__main__':
    main()
