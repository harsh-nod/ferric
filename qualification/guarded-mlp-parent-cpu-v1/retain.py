"""Verify a parent CPU evidence capsule and retain only its pinned data."""
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
    require(archive.stat().st_size == int(size) <= 32 << 20, 'bounded archive')
    body = archive.read_bytes()
    require(pin(body) == dict(bytes=int(size), sha256=digest), 'exact archive')
    with tarfile.open(archive, mode='r:gz') as stream:
        members = stream.getmembers()
        names = [member.name for member in members]
        require(1 <= len(names) <= 768 and len(set(names)) == len(names), 'unique bounded members')
        require(all(member.isfile() and not member.issym() and not member.islnk()
                    and Path(member.name).as_posix() == member.name
                    and not Path(member.name).is_absolute()
                    and '..' not in Path(member.name).parts and member.name not in ('', '.')
                    and 0 <= member.size <= 32 << 20 for member in members)
                and sum(member.size for member in members) <= 128 << 20, 'regular bounded bodies')
        bodies = {member.name: stream.extractfile(member).read() for member in members}
    manifest = json.loads(bodies['manifest.json'])
    require(manifest['files'] == {name: pin(value) for name, value in bodies.items()
                                 if name != 'manifest.json'}, 'exact manifest/body joins')
    terminal_name = 'complete.json' if manifest['passed'] else 'failed.json'
    terminal_body = bodies['evidence/' + terminal_name]
    require(pin(terminal_body) == manifest['terminal'], 'terminal pin')
    terminal = json.loads(terminal_body)
    require(terminal['schema'] == 'ferric-guarded-mlp-parent-cpu-v1'
            and terminal['postcheck_errors'] == [] and terminal['source_unchanged'] is True,
            'clean parent CPU postchecks')
    for field in ('gpu_execution', 'full_model_acceptance', 'numerical_acceptance',
                  'performance_claim', 'production_authority'):
        require(terminal[field] is False and manifest[field] is False, 'CPU-only claim boundary')
    require(manifest['passed'] == terminal['passed']
            and manifest['failure'] == terminal['failure'], 'terminal result join')
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
