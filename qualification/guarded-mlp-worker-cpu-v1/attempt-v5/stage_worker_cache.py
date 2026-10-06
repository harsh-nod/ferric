"""Populate only a fresh task-owned Cargo cache from checksum-locked inputs."""
import hashlib
import io
import json
import os
from pathlib import Path
import re
import signal
import stat
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-worker-cpu-v228-v5'
CARGO_HOME = ROOT / 'cargo-home'
LOCK = ROOT / 'ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.lock'
ARCHIVE = E / 'guarded-mlp-worker-cache-v228-v3.tar.gz'
INDEX = 'index.crates.io-1949cf8c6b5b557f'
REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index'
LOCK_PIN = dict(bytes=7470, sha256='df2a4e0b9cf96687328a5b1e41937eb940e62144314d7f903c8c961da947f7aa')
# Canonical sorted registry rows parsed independently from the exact lock above.
LOCKED_ROSTER_SHA = 'f07c07c9f16b5ea1c16541921b31f5785bcf500faf1249fb49dbb23fcdbba7ed'

def require(value, message):
    if not value:
        raise RuntimeError(message)

def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())

def read(path):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and 0 <= before.st_size <= 16 << 20, 'bounded ordinary cache input')
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
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 3
            and all(re.fullmatch(r'[0-9a-f]{64}', value) for value in sys.argv[1:]),
            'python3 -B stage_worker_cache.py ARCHIVE_SHA MANIFEST_SHA')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(ROOT.resolve(strict=True) == ROOT and not os.path.lexists(CARGO_HOME)
            and not os.path.lexists(ROOT / 'cargo-cache-manifest.json')
            and not os.path.lexists(ROOT / 'cargo-cache-stage-complete.json'), 'fresh private Cargo cache')
    lock = read(LOCK)
    require(pin(lock) == LOCK_PIN, 'do not change the worker lockfile')
    archive = read(ARCHIVE)
    require(pin(archive)['sha256'] == sys.argv[1], 'actual cache archive SHA')
    with tarfile.open(fileobj=io.BytesIO(archive), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == 61 and len({m.name for m in members}) == 61
                and all(m.isfile() and not Path(m.name).is_absolute() and '..' not in Path(m.name).parts
                        and Path(m.name).as_posix() == m.name and 0 <= m.size <= 8 << 20 for m in members)
                and sum(m.size for m in members) <= 16 << 20, 'bounded ordinary unique cache members')
        bodies = {m.name: tar.extractfile(m).read() for m in members}
    require(bodies['Cargo.lock'] == lock and pin(bodies['manifest.json'])['sha256'] == sys.argv[2],
            'locked manifest/body join')
    value = json.loads(bodies['manifest.json'])
    require(set(value) == {'schema', 'lock', 'registry', 'index', 'config', 'packages', 'files',
                           'inner_members', 'inner_expanded_bytes', 'cargo_execution'}
            and value['schema'] == 'ferric-guarded-mlp-worker-cache-v1'
            and value['lock'] == LOCK_PIN and value['registry'] == REGISTRY and value['index'] == INDEX
            and value['cargo_execution'] is False and len(value['packages']) == 29
            and len(value['files']) == 59, 'closed locked cache manifest')
    locked_rows = sorted((dict(name=row['name'], version=row['version'], checksum=row['checksum'],
                              source=REGISTRY) for row in value['packages']),
                         key=lambda row: (row['name'], row['version']))
    canonical = (json.dumps(locked_rows, sort_keys=True, separators=(',', ':')) + '\n').encode()
    require(hashlib.sha256(canonical).hexdigest() == LOCKED_ROSTER_SHA,
            'manifest package/version/checksum roster differs from the exact worker lock')
    allowed, package_keys, inventory = set(), [], []
    for row in value['packages']:
        require(set(row) == {'name', 'version', 'checksum', 'archive', 'index', 'members', 'expanded_bytes'}
                and re.fullmatch(r'[a-z0-9_-]+', row['name'])
                and re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', row['version'])
                and re.fullmatch(r'[0-9a-f]{64}', row['checksum']), 'closed registry package identity')
        package = row['name'] + '-' + row['version']
        archive_name = 'registry/cache/' + INDEX + '/' + package + '.crate'
        sparse_name = 'registry/index/' + INDEX + '/.cache/' + index_name(row['name'])
        require(row['archive'] == archive_name and row['index'] == sparse_name
                and not ({archive_name, sparse_name} & allowed), 'unique exact cache allowlist')
        allowed.update((archive_name, sparse_name))
        package_keys.append((row['name'], row['version']))
        require(pin(bodies[archive_name])['sha256'] == row['checksum'], 'locked registry archive checksum')
        choices = [json.loads(chunk) for chunk in bodies[sparse_name].split(b'\0') if chunk.startswith(b'{')]
        chosen = [item for item in choices if item.get('vers') == row['version']]
        require(len(chosen) == 1 and chosen[0]['name'] == row['name'] and chosen[0]['cksum'] == row['checksum'],
                'sparse record joins the locked archive')
        actual = crate_inventory(bodies[archive_name], package)
        require(actual == {key: row[key] for key in actual}, 'bounded inner archive inventory')
        inventory.append(actual)
    config = 'registry/index/' + INDEX + '/config.json'
    allowed.add(config)
    require(value['config'] == config and set(value['files']) == allowed
            and set(bodies) == allowed | {'Cargo.lock', 'manifest.json'}
            and package_keys == sorted(set(package_keys))
            and json.loads(bodies[config]) == dict(dl='https://static.crates.io/crates', api='https://crates.io')
            and all(pin(bodies[name]) == row for name, row in value['files'].items()), 'exact cache member pins/config')
    require(sum(row['members'] for row in inventory) == value['inner_members'] == 2266
            and sum(row['expanded_bytes'] for row in inventory) == value['inner_expanded_bytes'] == 50677903,
            'exact bounded inner expansion')
    os.umask(0o077)
    CARGO_HOME.mkdir(mode=0o700)
    for name in sorted(allowed):
        path = CARGO_HOME / name
        path.parent.mkdir(parents=True, exist_ok=True)
        require(path.parent.resolve(strict=True) == path.parent, 'cache destination alias')
        with path.open('xb') as stream:
            stream.write(bodies[name])
        require(pin(read(path)) == value['files'][name], 'staged cache body verification')
    require(read(LOCK) == lock and read(ARCHIVE) == archive, 'lock/archive changed during cache staging')
    with (ROOT / 'cargo-cache-manifest.json').open('xb') as stream:
        stream.write(bodies['manifest.json'])
    result = dict(schema='ferric-guarded-mlp-worker-cache-stage-v1', passed=True,
        archive=dict(path=str(ARCHIVE), **pin(archive)), manifest=pin(bodies['manifest.json']),
        cargo_home=str(CARGO_HOME), lock=LOCK_PIN, files=value['files'], packages=29,
        cache_files=59, inner_members=2266, inner_expanded_bytes=50677903,
        shared_cache_changed=False, lock_changed=False, project_code_executed=False,
        crate_sources_extracted=False)
    with (ROOT / 'cargo-cache-stage-complete.json').open('x') as stream:
        json.dump(result, stream, sort_keys=True, indent=2)
        stream.write('\n')
    print(json.dumps(result, sort_keys=True))

if __name__ == '__main__':
    def interrupted(signum, _frame):
        raise RuntimeError('cache staging interrupted: ' + str(signum))
    for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(signum, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 120)
    try:
        main()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
