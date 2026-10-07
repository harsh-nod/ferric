"""Explicit create-only MI350 input restore; never imports or launches GPU code."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import socket
import stat
import subprocess
import tarfile

HOST, UID = 'smci350-rck-g03-b19-03', 9661
BASE = Path('/dev/shm/ferric-v16-splitk-component-finalquartet-a001')
WIDTH = Path('/dev/shm/ferric-prefill-width-inputs-a001')
ROSTER_SHA = '864b0b6b9ac934ba2a6610112bbf18e0fddac0146e286871733ec6dd356ccf36'
STAGE_CAP = 2 * 1024**3


def require(value, message):
    if not value:
        raise ValueError(message)


def decode(raw):
    def unique(items):
        value = {}
        for key, item in items:
            require(key not in value, 'duplicate JSON key')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=unique,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def resources(planned=0):
    values = {}
    for name, path, minimum in (('root_free_bytes', '/', 64 * 1024**3),
                                ('shm_free_bytes', '/dev/shm', 32 * 1024**3 + planned)):
        item = os.statvfs(path)
        values[name] = item.f_bavail * item.f_frsize
        require(values[name] >= minimum, 'unchanged native free-space floor: ' + name)
    mem = dict(line.split(':', 1) for line in Path('/proc/meminfo').read_text().splitlines())
    values['memory_available_bytes'] = int(mem['MemAvailable'].split()[0]) * 1024
    require(values['memory_available_bytes'] >= 128 * 1024**3 + planned, 'unchanged native RAM floor')
    return values


def read_file(path, maximum, expected=None):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical source input')
    descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(descriptor)
        require(stat.S_ISREG(before.st_mode) and before.st_uid == UID and before.st_nlink == 1
                and 0 <= before.st_size <= maximum, 'bounded owned single-link source')
        parts, size = [], 0
        while part := os.read(descriptor, min(1024**2, maximum + 1 - size)):
            parts.append(part)
            size += len(part)
            require(size <= maximum, 'source grew')
        after, current = os.fstat(descriptor), path.lstat()
        fields = ('st_dev', 'st_ino', 'st_mode', 'st_uid', 'st_nlink', 'st_size', 'st_mtime_ns', 'st_ctime_ns')
        require(size == before.st_size and all(getattr(before, field) == getattr(after, field)
                == getattr(current, field) for field in fields), 'stable source input')
        raw = b''.join(parts)
        digest = hashlib.sha256(raw).hexdigest()
        require(expected is None or digest == expected, 'exact source digest')
        return raw, {field: getattr(before, field) for field in fields} | {'sha256': digest}
    finally:
        os.close(descriptor)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--restore', action='store_true', required=True)
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--archive-sha256', required=True)
    args = parser.parse_args()
    os.umask(0o077)
    require(socket.gethostname() == HOST and os.getuid() == UID, 'fixed approved MI350 host/UID')
    require(re.fullmatch('[0-9a-f]{64}', args.archive_sha256), 'pinned delta archive')
    resources()
    archive_raw, archive_identity = read_file(args.archive, 4 * 1024**2, args.archive_sha256)
    members, expanded = {}, 0
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as packed:
        for member in packed:
            require(len(members) < 36 and member.isfile() and member.name not in members
                    and member.mode == 0o600 and 0 <= member.size <= 3 * 1024**2,
                    'exact bounded regular delta members')
            require(member.name == 'delta-roster.json'
                    or re.fullmatch('objects/[0-9a-f]{64}', member.name), 'closed delta member name')
            expanded += member.size
            require(expanded <= 4 * 1024**2, 'delta expansion bound')
            raw = packed.extractfile(member).read(member.size + 1)
            require(len(raw) == member.size, 'complete delta member')
            members[member.name] = raw
    require(hashlib.sha256(members['delta-roster.json']).hexdigest() == ROSTER_SHA, 'reviewed complete restore roster')
    roster = decode(members['delta-roster.json'])
    require(roster['schema'] == 'FerricOrderedComponentDeltaRestoreV1'
            and set(roster['modes']) == {'latency', 'counters', 'ticks'}
            and set(members) == {'delta-roster.json'} | {'objects/' + key for key in roster['objects']},
            'exact three-mode delta object set')
    for digest, value in roster['objects'].items():
        raw = members['objects/' + digest]
        require(len(raw) == value['bytes'] and hashlib.sha256(raw).hexdigest() == digest, 'delta object identity')
    source_before, jobs, total = {}, [], 0
    for mode, value in roster['modes'].items():
        stage = Path(value['stage'])
        require(stage == Path('/dev/shm/ferric-v16-splitk-ordered-' + mode + '-a001')
                and not os.path.lexists(stage) and stage.parent.resolve(strict=True) == stage.parent,
                'fresh exact native stage')
        require(len(value['files']) == 240, 'complete240-member stage')
        stage_bytes = 0
        for name, item in value['files'].items():
            relative = Path(name)
            require(name and not relative.is_absolute() and '..' not in relative.parts
                    and str(relative) == name and type(item['bytes']) is int
                    and 0 <= item['bytes'] <= 64 * 1024**2
                    and type(item['mode']) is int and item['mode'] in (0o600, 0o700), 'closed stage member')
            source = item['source']
            if source['kind'] == 'delta':
                require(source['path'] == 'objects/' + item['sha256'], 'exact delta object reference')
                raw = members[source['path']]
            else:
                require(source['kind'] in ('base', 'width'), 'known retained source kind')
                root = BASE if source['kind'] == 'base' else WIDTH
                path = Path(source['path'])
                require(path.is_relative_to(root), 'retained source inside exact prior stage')
                raw, identity = read_file(path, 64 * 1024**2, item['sha256'])
                require(str(path) not in source_before or source_before[str(path)] == identity,
                        'retained source identity stable across references')
                source_before[str(path)] = identity
            require(len(raw) == item['bytes'] and hashlib.sha256(raw).hexdigest() == item['sha256'],
                    'complete preflight member binding')
            stage_bytes += len(raw)
            jobs.append((stage, relative, item))
        require(stage_bytes + 4 * 1024**2 < STAGE_CAP, 'unchanged two-GiB native stage cap')
        total += stage_bytes
    before_resources = resources(total + 16 * 1024**2)
    for value in roster['modes'].values():
        Path(value['stage']).mkdir(mode=0o700)
    for stage, relative, item in jobs:
        source = item['source']
        raw = (members[source['path']] if source['kind'] == 'delta' else
               read_file(Path(source['path']), 64 * 1024**2, item['sha256'])[0])
        path = stage / relative
        path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
        path.chmod(item['mode'])
        written, _ = read_file(path, 64 * 1024**2, item['sha256'])
        require(len(written) == item['bytes'], 'restored member size')
    for name, identity in source_before.items():
        require(read_file(Path(name), 64 * 1024**2, identity['sha256'])[1] == identity,
                'retained base/width input changed')
    require(read_file(args.archive, 4 * 1024**2, args.archive_sha256)[1] == archive_identity,
            'delta archive changed')
    allocations = {}
    for mode, value in roster['modes'].items():
        result = subprocess.run(['/usr/bin/du', '-sx', '-B1', '--', value['stage']],
                                capture_output=True, check=True, timeout=30)
        fields = result.stdout.decode('ascii').split()
        require(not result.stderr and len(fields) == 2 and fields[1] == value['stage'], 'exact stage allocation')
        allocations[mode] = int(fields[0])
        require(allocations[mode] <= STAGE_CAP, 'unchanged native stage allocation cap')
    receipt = {'schema': 'FerricOrderedComponentDeltaRestoredV1', 'restored': True,
        'roster_sha256': ROSTER_SHA, 'archive_sha256': args.archive_sha256,
        'files': len(jobs), 'bytes': total, 'retained_sources_unchanged': True,
        'stage_allocations': allocations, 'resources_before': before_resources,
        'resources_after': resources(), 'native_launched': False, 'tests_executed': False,
        'plans': {mode: {'path': value['stage'] + '/plan.json', 'sha256': value['plan_sha256']}
                  for mode, value in roster['modes'].items()}}
    with (args.archive.parent / 'restore.json').open('x') as stream:
        json.dump(receipt, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(receipt, sort_keys=True))


if __name__ == '__main__':
    main()
