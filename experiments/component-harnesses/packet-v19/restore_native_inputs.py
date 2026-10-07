"""Restore a closed V19 diagnostic input roster; never import or launch native code."""
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
import time

HOST, UID = 'smci350-rck-g03-b19-03', 9661
ROOT = Path('/dev/shm/ferric-packet-v19-inputs-a001')
WIDTH = Path('/dev/shm/ferric-prefill-width-inputs-a001')
SOURCE_SHA = 'c78b27ec8d7f1f53d3e1d22631bfdb8037327be734ce59cb328dac3f40ccd4f6'
WIDTH_SHA = '6f1a54afbfa7d8d3c3ff755584454cd5be968cdd5776847777917e87598f826b'
BUILD_SHA = '17aad5341fd650a62d10a673be517627751b2eaedfc6f170d744cda3e118194a'
CAP = 128 * 1024**2


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
    result = {}
    for name, path, floor in (('root_free_bytes', '/', 64 * 1024**3),
                              ('shm_free_bytes', '/dev/shm', 32 * 1024**3 + planned)):
        info = os.statvfs(path)
        result[name] = info.f_bavail * info.f_frsize
        require(result[name] >= floor, 'unchanged native floor: ' + name)
    memory = dict(line.split(':', 1) for line in Path('/proc/meminfo').read_text().splitlines())
    result['memory_available_bytes'] = int(memory['MemAvailable'].split()[0]) * 1024
    require(result['memory_available_bytes'] >= 128 * 1024**3 + planned, 'native RAM floor')
    return result


def read(path, maximum, expected):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input')
    descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(descriptor)
        require(stat.S_ISREG(before.st_mode) and before.st_uid == UID and before.st_nlink == 1
                and 0 <= before.st_size <= maximum, 'bounded owned single-link input')
        with os.fdopen(os.dup(descriptor), 'rb') as stream:
            raw = stream.read(maximum + 1)
        after, current = os.fstat(descriptor), path.lstat()
        fields = ('st_dev', 'st_ino', 'st_size', 'st_mode', 'st_uid', 'st_nlink', 'st_mtime_ns', 'st_ctime_ns')
        require(len(raw) == before.st_size and all(getattr(before, field) == getattr(after, field)
                == getattr(current, field) for field in fields), 'input unchanged during read')
        require(hashlib.sha256(raw).hexdigest() == expected, 'exact input SHA256')
        return raw, {field: getattr(before, field) for field in fields}
    finally:
        os.close(descriptor)


def relative(name):
    path = Path(name)
    require(type(name) is str and path.parts and not path.is_absolute() and '..' not in path.parts
            and str(path) == name, 'canonical relative member')
    return path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--restore', action='store_true', required=True)
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--archive-sha256', required=True)
    parser.add_argument('--roster', type=Path, required=True)
    parser.add_argument('--roster-sha256', required=True)
    args = parser.parse_args()
    os.umask(0o077)
    deadline = time.monotonic() + 120
    require(socket.gethostname() == HOST and os.getuid() == UID, 'fixed MI350 host and UID')
    require(all(re.fullmatch('[0-9a-f]{64}', digest) for digest in
                (args.archive_sha256, args.roster_sha256)), 'explicit fixed transport pins')
    require(not os.path.lexists(ROOT) and ROOT.parent.resolve(strict=True) == ROOT.parent,
            'create-only exact restore root')
    resources(CAP)
    roster_raw, roster_identity = read(args.roster, 1024**2, args.roster_sha256)
    roster = decode(roster_raw)
    require(set(roster) == {'schema', 'native_root', 'files', 'reuse', 'build',
        'original_build_sha256', 'source_archive_sha256', 'width_roster_sha256', 'native_executed'}
        and roster['schema'] == 'FerricV19Packet55cPortableInputsV1'
        and roster['native_root'] == str(ROOT) and roster['original_build_sha256'] == BUILD_SHA
        and roster['source_archive_sha256'] == SOURCE_SHA and roster['width_roster_sha256'] == WIDTH_SHA
        and roster['native_executed'] is False, 'exact qualified export provenance')
    require(type(roster['files']) is dict and 23 <= len(roster['files']) <= 512
            and type(roster['reuse']) is dict and set(roster['reuse']) <= set(roster['files']),
            'closed bounded complete roster')
    total = 0
    for name, item in roster['files'].items():
        relative(name)
        require(set(item) == {'bytes', 'sha256', 'mode'} and type(item['bytes']) is int
                and 0 <= item['bytes'] <= 64 * 1024**2 and re.fullmatch('[0-9a-f]{64}', item['sha256'])
                and type(item['mode']) is int and item['mode'] in (0o600, 0o700), 'closed member metadata')
        total += item['bytes']
    require(total <= 96 * 1024**2, 'expanded complete input bound')
    require(roster['build'] == {'path': str(ROOT / 'build.json'),
                               'sha256': roster['files']['build.json']['sha256']}, 'exact portable build binding')
    archive_raw, archive_identity = read(args.archive, 48 * 1024**2, args.archive_sha256)
    members, expanded = {}, 0
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as archive:
        for member in archive:
            require(time.monotonic() < deadline, 'bounded restore deadline')
            require(member.name in roster['files'] and member.name not in roster['reuse']
                    and member.name not in members and member.isfile(), 'exact regular delta member')
            item = roster['files'][member.name]
            require(member.size == item['bytes'] and member.mode == item['mode'], 'delta size/mode')
            expanded += member.size
            require(expanded <= 96 * 1024**2, 'bounded delta expansion')
            data = archive.extractfile(member).read(member.size + 1)
            require(len(data) == member.size and hashlib.sha256(data).hexdigest() == item['sha256'],
                    'delta content identity')
            members[member.name] = data
    require(set(members) == set(roster['files']) - set(roster['reuse']), 'complete delta set')
    identities = {}
    for name, source in roster['reuse'].items():
        require(re.fullmatch('objects/[0-9a-f]{64}', name) and source == str(WIDTH / name)
                and name == 'objects/' + roster['files'][name]['sha256'], 'exact fixed reusable object')
        data, identity = read(Path(source), 64 * 1024**2, roster['files'][name]['sha256'])
        require(len(data) == roster['files'][name]['bytes'], 'reused input size')
        identities[name] = identity
    before = resources(CAP)
    ROOT.mkdir(mode=0o700)
    for name, item in roster['files'].items():
        require(time.monotonic() < deadline, 'bounded restore deadline')
        data = (read(Path(roster['reuse'][name]), 64 * 1024**2, item['sha256'])[0]
                if name in roster['reuse'] else members[name])
        path = ROOT / relative(name)
        path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, item['mode'])
        with os.fdopen(descriptor, 'wb') as target:
            target.write(data)
        require(len(read(path, 64 * 1024**2, item['sha256'])[0]) == item['bytes'], 'restored file size')
    for name, identity in identities.items():
        require(read(Path(roster['reuse'][name]), 64 * 1024**2,
                     roster['files'][name]['sha256'])[1] == identity, 'reused input unchanged')
    require(read(args.roster, 1024**2, args.roster_sha256)[1] == roster_identity
            and read(args.archive, 48 * 1024**2, args.archive_sha256)[1] == archive_identity,
            'transport inputs unchanged')
    allocation = subprocess.run(['/usr/bin/du', '-sx', '-B1', '--', str(ROOT)],
                                 check=True, capture_output=True, timeout=30)
    fields = allocation.stdout.decode('ascii').split()
    require(not allocation.stderr and len(fields) == 2 and fields[1] == str(ROOT)
            and int(fields[0]) <= CAP and time.monotonic() < deadline, 'restore cap/deadline')
    receipt = {'schema': 'FerricV19Packet55cPortableRestoredV1', 'native_launched': False,
        'tests_executed': False, 'files': len(roster['files']), 'reused_files': len(identities),
        'logical_bytes': total, 'stage_allocated_bytes': int(fields[0]),
        'archive_sha256': args.archive_sha256, 'roster_sha256': args.roster_sha256,
        'resources_before': before, 'resources_after': resources(), 'build': roster['build'],
        'source_inputs_unchanged': True}
    with (ROOT / 'restore-receipt.json').open('x') as target:
        json.dump(receipt, target, indent=2, sort_keys=True)
        target.write('\n')
    print(json.dumps(receipt, sort_keys=True))


if __name__ == '__main__':
    main()
