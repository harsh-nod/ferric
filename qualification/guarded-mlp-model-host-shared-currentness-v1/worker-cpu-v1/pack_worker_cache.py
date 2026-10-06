"""Package only the exact locked registry inputs; execute no Cargo commands."""
import hashlib
import io
import json
import os
from pathlib import Path
import signal
import stat
import sys
import tarfile
import tomllib

P = Path(__file__).resolve().parent
LOCK = Path('/home/harsh/ferric-p227-integration/adapters/tp-peer-finite-engineering-worker-v1/Cargo.lock')
CACHE = Path('/home/harsh/.cargo')
INDEX = 'index.crates.io-1949cf8c6b5b557f'
REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index'
LOCK_PIN = dict(bytes=7470, sha256='df2a4e0b9cf96687328a5b1e41937eb940e62144314d7f903c8c961da947f7aa')
ARCHIVE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/guarded-mlp-worker-cache-v228-v3.tar.gz')
MANIFEST = P / 'worker-cache-manifest-v228-v3.json'

def require(value, message):
    if not value:
        raise RuntimeError(message)

def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())

def read(path):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and 0 <= before.st_size <= 8 << 20, 'bounded ordinary cache input')
    body = path.read_bytes()
    after = path.lstat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(len(body) == before.st_size and stamp(before) == stamp(after), 'cache input changed')
    return body

def index_name(name):
    if len(name) == 1:
        return '1/' + name
    if len(name) == 2:
        return '2/' + name
    if len(name) == 3:
        return '3/' + name[0] + '/' + name
    return name[:2] + '/' + name[2:4] + '/' + name

def crate_inventory(body, package):
    with tarfile.open(fileobj=io.BytesIO(body), mode='r:gz') as tar:
        members = tar.getmembers()
        require(0 < len(members) <= 1024 and len({m.name for m in members}) == len(members), 'crate member count')
        for member in members:
            path = Path(member.name)
            require(not path.is_absolute() and '..' not in path.parts
                    and path.parts[0] == package and member.name.rstrip('/') == path.as_posix()
                    and (member.isfile() or member.isdir()) and 0 <= member.size <= 2 << 20,
                    'crate path/type/extent; links and devices are forbidden')
            if member.isfile():
                require(len(tar.extractfile(member).read()) == member.size, 'crate body extent')
        total = sum(m.size for m in members)
        require(total <= 32 << 20, 'per-crate expansion cap')
        require(package + '/Cargo.toml' in {m.name for m in members if m.isfile()}, 'crate manifest absent')
        return dict(members=len(members), expanded_bytes=total)

def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B pack_worker_cache.py')
    require(not os.path.lexists(ARCHIVE) and not os.path.lexists(MANIFEST), 'fresh cache package outputs')
    lock = read(LOCK)
    require(pin(lock) == LOCK_PIN, 'unchanged qualified worker lock')
    locked = sorted((row for row in tomllib.loads(lock.decode())['package'] if 'source' in row),
                    key=lambda row: (row['name'], row['version']))
    require(len(locked) == 29 and all(row['source'] == REGISTRY for row in locked), 'exact locked registry closure')
    bodies, observed, packages = {}, {}, []
    for row in locked:
        package = row['name'] + '-' + row['version']
        archive_name = 'registry/cache/' + INDEX + '/' + package + '.crate'
        sparse_name = 'registry/index/' + INDEX + '/.cache/' + index_name(row['name'])
        for name in (archive_name, sparse_name):
            require(name not in bodies, 'duplicate registry input')
            body = read(CACHE / name)
            bodies[name] = body
            observed[name] = pin(body)
        require(pin(bodies[archive_name])['sha256'] == row['checksum'], 'locked archive checksum')
        choices = [json.loads(chunk) for chunk in bodies[sparse_name].split(b'\0') if chunk.startswith(b'{')]
        chosen = [value for value in choices if value.get('vers') == row['version']]
        require(len(chosen) == 1 and chosen[0]['name'] == row['name']
                and chosen[0]['cksum'] == row['checksum'], 'sparse index locked-version checksum')
        packages.append(dict(name=row['name'], version=row['version'], checksum=row['checksum'],
            archive=archive_name, index=sparse_name, **crate_inventory(bodies[archive_name], package)))
    config_name = 'registry/index/' + INDEX + '/config.json'
    bodies[config_name] = read(CACHE / config_name)
    observed[config_name] = pin(bodies[config_name])
    require(json.loads(bodies[config_name]) == dict(dl='https://static.crates.io/crates', api='https://crates.io'),
            'standard crates.io sparse configuration')
    require(len(bodies) == 59 and sum(p['members'] for p in packages) == 2266
            and sum(p['expanded_bytes'] for p in packages) == 50677903, 'closed inner archive census')
    value = dict(schema='ferric-guarded-mlp-worker-cache-v1', lock=LOCK_PIN, registry=REGISTRY,
        index=INDEX, config=config_name, packages=packages,
        files={name: pin(body) for name, body in sorted(bodies.items())},
        inner_members=2266, inner_expanded_bytes=50677903, cargo_execution=False)
    manifest = (json.dumps(value, indent=2, sort_keys=True) + '\n').encode()
    bodies.update({'Cargo.lock': lock, 'manifest.json': manifest})
    require(len(bodies) == 61 and sum(map(len, bodies.values())) <= 16 << 20, 'outer cache archive bound')
    require(read(LOCK) == lock and all(pin(read(CACHE / name)) == row for name, row in observed.items()),
            'locked inputs changed during packing')
    os.umask(0o077)
    with ARCHIVE.open('xb') as output:
        with tarfile.open(fileobj=output, mode='w:gz', format=tarfile.PAX_FORMAT) as tar:
            for name, body in sorted(bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(body), 0o600, 0
                info.uid = info.gid = 0
                info.uname = info.gname = ''
                tar.addfile(info, io.BytesIO(body))
    with MANIFEST.open('xb') as stream:
        stream.write(manifest)
    require(read(MANIFEST) == manifest, 'manifest write verification')
    archive = ARCHIVE.read_bytes()
    require(len(archive) <= 16 << 20, 'compressed archive bound')
    print(json.dumps(dict(archive=dict(path=str(ARCHIVE), **pin(archive)), manifest=pin(manifest),
        members=len(bodies), expanded_bytes=sum(map(len, bodies.values())),
        packages=29, cache_files=59, cargo_execution=False), sort_keys=True))

if __name__ == '__main__':
    def interrupted(signum, _frame):
        raise RuntimeError('cache packaging interrupted: ' + str(signum))
    for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(signum, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 120)
    try:
        main()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
