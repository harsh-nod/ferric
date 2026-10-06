"""Verify and unpack a worker evidence capsule as data, never as code."""

import hashlib
import json
import os
from pathlib import Path
import sys
import tarfile


def require(value, message):
    if not value:
        raise RuntimeError(message)


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    require(len(sys.argv) == 5, 'ARCHIVE SHA256 BYTES DESTINATION')
    archive, digest, size, destination = sys.argv[1:]
    archive, destination = Path(archive), Path(destination)
    require(archive.resolve(strict=True) == archive and archive.is_file(), 'ordinary archive')
    body = archive.read_bytes()
    require(pin(body) == dict(bytes=int(size), sha256=digest) and len(body) <= 32 << 20,
            'exact bounded archive')
    with tarfile.open(archive, mode='r:gz') as stream:
        members = stream.getmembers()
        names = [member.name for member in members]
        require(1 <= len(names) <= 512 and len(set(names)) == len(names), 'unique bounded members')
        require(all(member.isfile() and not member.issym() and not member.islnk()
                    and Path(member.name).as_posix() == member.name
                    and not Path(member.name).is_absolute()
                    and '..' not in Path(member.name).parts and member.name not in ('', '.')
                    and 0 <= member.size <= 32 << 20 for member in members)
                and sum(member.size for member in members) <= 96 << 20, 'regular bounded bodies')
        bodies = {member.name: stream.extractfile(member).read() for member in members}
    manifest = json.loads(bodies['manifest.json'])
    require(manifest['schema'] == 'ferric-guarded-mlp-stage-capture-worker-cpu-retained-v1', 'capture capsule schema')
    require(manifest['files'] == {name: pin(value) for name, value in bodies.items()
                                 if name != 'manifest.json'}, 'exact manifest/body joins')
    terminal = json.loads(bodies['evidence/' + manifest['terminal_name']])
    require(terminal['schema'] == 'ferric-guarded-mlp-stage-capture-worker-cpu-v1'
            and terminal['capture_source_added'] is True and terminal['capture_native_execution'] is False
            and terminal['postcheck_errors'] == [] and terminal['source_unchanged'] is True
            and terminal['gpu_execution'] is False
            and terminal['whole_model_guarded_execution'] is False
            and terminal['performance_claim'] is False, 'clean CPU-only terminal')
    require(manifest['passed'] == terminal['passed']
            and manifest['failure'] == terminal['failure'], 'terminal result join')
    require(manifest['host_executable_bodies_retained'] is False
            and manifest['capture_source_added'] is True and manifest['capture_native_execution'] is False,
            'bounded source/raw capture qualification only')
    if terminal['passed']:
        require(len(bodies) == 256 and manifest['raw_count'] == 50 and manifest['phases'] == 9
                and manifest['source_map_rows'] == 990
                and (terminal['tests']['worker-tests']['passed'], terminal['tests']['worker-tests']['failed'],
                     terminal['tests']['worker-tests']['ignored']) == (591, 0, 4),
                'exact successful capture worker census')
    require(not os.path.lexists(destination), 'fresh retention directory')
    destination.mkdir(parents=True, mode=0o700)
    require(destination.resolve(strict=True) == destination, 'ordinary destination')
    for name, value in sorted(bodies.items()):
        path = destination / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as output:
            output.write(value)
        require(path.read_bytes() == value, 'retained body differs')
    print(json.dumps(dict(archive=pin(body), members=len(bodies), destination=str(destination),
                          passed=terminal['passed'], failure=terminal['failure']), sort_keys=True))


if __name__ == '__main__':
    main()

