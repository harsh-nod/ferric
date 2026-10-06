"""Transfer a closed, original-byte framework reference into a fresh alias root."""
import hashlib
import io
import json
import os
from pathlib import Path
import sys
import tarfile

P = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
Q = Path('/home/harsh/ferric-p227-integration/qualification/layer0-framework-capture-v1')
MAP = P / 'proposals/guarded-mlp-model-interface-v228-v1/model-stage-capture-numerical-v2/framework-aliases.json'
MAP_SHA = 'f69fff0120cd8b25cf8350b246631a88e504de387b685c1b7c92f181d1d0bd3f'
DEST = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-model-stage-framework-v228-v1')


def require(value, message):
    if not value:
        raise RuntimeError(message)


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def verify(bodies):
    raw = bodies['framework-aliases.json']
    require(pin(raw) == dict(bytes=35122, sha256=MAP_SHA), 'frozen alias map')
    manifest = json.loads(raw)
    rows = manifest['files']
    require(len(rows) == 73 and len(bodies) == 74, 'closed reference census')
    names = set()
    for original, row in rows.items():
        name = row['relative']
        path = Path(name)
        require(not path.is_absolute() and '..' not in path.parts
                and path.as_posix() == name and name not in names, 'unique relative member')
        names.add(name)
        require(row['original']['path'] == original
                and pin(bodies[name]) == {k: row['original'][k] for k in ('bytes', 'sha256')},
                'original reference bytes')
    require(set(bodies) == names | {'framework-aliases.json'}
            and sum(len(bodies[name]) for name in names) == 651037, 'exact reference closure')


def pack(archive):
    raw = MAP.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == MAP_SHA, 'map identity before parsing')
    bodies = {'framework-aliases.json': raw}
    for row in json.loads(raw)['files'].values():
        relative = row['relative']
        path = (P / 'layer0-framework-capture-v228-v1' / Path(relative).name
                if relative.endswith('.bf16') else Q / relative)
        require(path.resolve(strict=True) == path and path.is_file(), 'ordinary retained source')
        bodies[relative] = path.read_bytes()
    verify(bodies)
    with archive.open('xb') as output:
        with tarfile.open(fileobj=output, mode='w:gz', format=tarfile.USTAR_FORMAT) as stream:
            for name, body in sorted(bodies.items()):
                member = tarfile.TarInfo(name)
                member.size, member.mode, member.mtime = len(body), 0o600, 0
                stream.addfile(member, io.BytesIO(body))
    print(json.dumps(dict(archive=str(archive), **pin(archive.read_bytes()), members=74), sort_keys=True))


def stage(archive, size, digest):
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'selected unprivileged host')
    require(archive.resolve(strict=True) == archive and archive.is_file()
            and archive.stat().st_size == int(size) <= 2 << 20, 'bounded ordinary archive')
    require(pin(archive.read_bytes()) == dict(bytes=int(size), sha256=digest), 'actual transport pin')
    with tarfile.open(archive, 'r:gz') as stream:
        members = stream.getmembers()
        require(len(members) == 74 and len({m.name for m in members}) == 74
                and all(m.isfile() and 0 <= m.size <= 1 << 20 for m in members)
                and sum(m.size for m in members) <= 2 << 20, 'bounded regular member closure')
        bodies = {m.name: stream.extractfile(m).read() for m in members}
    verify(bodies)
    require(DEST.parent.resolve(strict=True) == DEST.parent and not os.path.lexists(DEST), 'fresh alias root')
    os.umask(0o077)
    DEST.mkdir(mode=0o700)
    for name, body in sorted(bodies.items()):
        path = DEST / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as output:
            output.write(body)
        require(path.read_bytes() == body, 'staged reference drift')
    verify({name: (DEST / name).read_bytes() for name in bodies})
    print(json.dumps(dict(destination=str(DEST), members=74, reference_bytes=651037,
                         original_paths_rewritten=False), sort_keys=True))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode, 'python3 -B only')
    if len(sys.argv) == 3 and sys.argv[1] == 'pack':
        pack(Path(sys.argv[2]))
    elif len(sys.argv) == 5 and sys.argv[1] == 'stage':
        stage(Path(sys.argv[2]), sys.argv[3], sys.argv[4])
    else:
        raise SystemExit('pack ARCHIVE | stage ARCHIVE BYTES SHA256')
