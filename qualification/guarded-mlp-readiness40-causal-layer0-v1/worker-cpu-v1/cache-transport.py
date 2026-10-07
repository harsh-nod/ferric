"""Pack/stage the exact two-lock registry union; never execute Cargo or extract crates."""
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
import tomllib

P = Path(__file__).resolve().parent
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-readiness40-causal-layer0-worker-cpu-v228-v1'
BASE = Path('/home/harsh/ferric-p227-integration/qualification/guarded-mlp-model-host-shared-currentness-v1/worker-cpu-v1')
CACHE = Path('/home/harsh/.cargo')
ARCHIVE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/guarded-mlp-peer-read-pair-worker-cache-v228-v3.tar.gz')
REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index'
INDEX = 'index.crates.io-1949cf8c6b5b557f'
LOCKS = {
    'runtime': ('fe2o3/Cargo.lock', 8058, 'b605fb665bbd9b6ba266c0b6f74f4b9cab50bd1845e136dc6ac6cb6b2dadd252'),
    'worker': ('ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.lock', 7470,
               'df2a4e0b9cf96687328a5b1e41937eb940e62144314d7f903c8c961da947f7aa'),
}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def parse(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(path, cap=64 << 20):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input')
    fields = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and 0 <= before.st_size <= cap,
                'bounded single-link ordinary input')
        raw = stream.read(cap + 1)
        after = os.fstat(stream.fileno())
    require(fields(before) == fields(after) == fields(path.lstat()) and len(raw) == before.st_size,
            'input changed during read')
    return raw


def relative(name):
    return type(name) is str and Path(name).as_posix() == name and not Path(name).is_absolute() \
        and '..' not in Path(name).parts and name not in ('', '.')


def sparse(name):
    if len(name) < 3:
        return str(len(name)) + '/' + name
    if len(name) == 3:
        return '3/' + name[0] + '/' + name
    return name[:2] + '/' + name[2:4] + '/' + name


def locked_union(bodies):
    result, counts = {}, {}
    for role, (_, size, digest) in LOCKS.items():
        require(pin(bodies[role]) == dict(bytes=size, sha256=digest), 'immutable lock pin')
        packages = [row for row in tomllib.loads(bodies[role].decode())['package'] if 'source' in row]
        counts[role] = len(packages)
        for row in packages:
            require(row['source'] == REGISTRY and re.fullmatch('[0-9a-f]{64}', row['checksum']),
                    'only exact checksummed crates.io dependencies')
            key = row['name'], row['version']
            value = dict(name=key[0], version=key[1], checksum=row['checksum'], source=REGISTRY)
            require(key not in result or result[key] == value, 'lock union checksum conflict')
            result[key] = value
    require(counts == dict(runtime=32, worker=29) and len(result) == 39, 'closed two-lock union')
    return [result[key] for key in sorted(result)]


def crate_inventory(raw, package):
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(0 < len(members) <= 2048 and len({m.name for m in members}) == len(members), 'crate member census')
        for member in members:
            path = Path(member.name)
            require(not path.is_absolute() and '..' not in path.parts and path.parts[0] == package
                    and member.name.rstrip('/') == path.as_posix() and (member.isfile() or member.isdir())
                    and 0 <= member.size <= 4 << 20, 'crate path/type/extent; no links')
            if member.isfile():
                require(len(tar.extractfile(member).read(member.size + 1)) == member.size, 'crate member extent')
        total = sum(m.size for m in members)
        require(total <= 48 << 20 and package + '/Cargo.toml' in {m.name for m in members if m.isfile()},
                'bounded crate with manifest')
    return dict(members=len(members), expanded_bytes=total)


def validate(manifest, bodies, locks):
    packages = locked_union(locks)
    require(manifest['schema'] == 'ferric-guarded-mlp-reusable-arena-cache-v1'
            and manifest['locked_packages'] == packages and manifest['cargo_execution'] is False
            and manifest['locks'] == {role: pin(body) for role, body in locks.items()}
            and manifest['shared_cache_changed'] is False and manifest['lock_changed'] is False
            and manifest['registry'] == REGISTRY and manifest['index'] == INDEX,
            'closed locked cache provenance')
    expected, inventories = set(), {}
    for row in packages:
        package = row['name'] + '-' + row['version']
        archive = 'registry/cache/' + INDEX + '/' + package + '.crate'
        index = 'registry/index/' + INDEX + '/.cache/' + sparse(row['name'])
        expected.update((archive, index))
        require(pin(bodies[archive])['sha256'] == row['checksum'], 'locked archive checksum')
        choices = [parse(chunk) for chunk in bodies[index].split(b'\0') if chunk.startswith(b'{')]
        chosen = [value for value in choices if value.get('vers') == row['version']]
        require(len(chosen) == 1 and chosen[0]['name'] == row['name']
                and chosen[0]['cksum'] == row['checksum'], 'locked sparse version/checksum')
        inventories[package] = crate_inventory(bodies[archive], package)
    config = 'registry/index/' + INDEX + '/config.json'
    expected.add(config)
    require(parse(bodies[config]) == dict(dl='https://static.crates.io/crates', api='https://crates.io'),
            'standard sparse registry configuration')
    require(set(bodies) == set(manifest['files']) == expected
            and all(pin(bodies[name]) == row for name, row in manifest['files'].items())
            and inventories == manifest['crate_inventory'], 'exact immutable cache closure')
    require(sum(row['expanded_bytes'] for row in inventories.values()) <= 128 << 20,
            'bounded eventual Cargo extraction')


def pack():
    require(not os.path.lexists(ARCHIVE) and not os.path.lexists(P / 'cache-manifest.json'), 'fresh package outputs')
    locks = {role: read(BASE / row[0]) for role, row in LOCKS.items()}
    packages = locked_union(locks)
    bodies, inputs, inventories = {}, {}, {}
    for row in packages:
        name = row['name'] + '-' + row['version']
        archive = 'registry/cache/' + INDEX + '/' + name + '.crate'
        index = 'registry/index/' + INDEX + '/.cache/' + sparse(row['name'])
        for relative_name in (archive, index):
            if relative_name not in bodies:
                bodies[relative_name] = read(CACHE / relative_name)
                inputs[relative_name] = pin(bodies[relative_name])
        inventories[name] = crate_inventory(bodies[archive], name)
    config = 'registry/index/' + INDEX + '/config.json'
    bodies[config] = read(CACHE / config); inputs[config] = pin(bodies[config])
    manifest = dict(schema='ferric-guarded-mlp-reusable-arena-cache-v1', registry=REGISTRY, index=INDEX,
        locks={role: pin(body) for role, body in locks.items()}, locked_packages=packages,
        files=dict(sorted(inputs.items())), crate_inventory=inventories, cargo_execution=False,
        shared_cache_changed=False, lock_changed=False)
    validate(manifest, bodies, locks)
    raw_manifest = encoded(manifest)
    all_bodies = dict(bodies, **{'manifest.json': raw_manifest},
                     **{role + '.lock': body for role, body in locks.items()})
    require(len(all_bodies) <= 128 and sum(map(len, all_bodies.values())) <= 64 << 20, 'bounded outer archive')
    require(all(pin(read(CACHE / name)) == row for name, row in inputs.items())
            and all(read(BASE / LOCKS[role][0]) == raw for role, raw in locks.items()), 'source posthash')
    with ARCHIVE.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(all_bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(raw), 0o600, 0
                tar.addfile(info, io.BytesIO(raw))
    with (P / 'cache-manifest.json').open('xb') as stream:
        stream.write(raw_manifest)
    print(json.dumps(dict(archive=dict(path=str(ARCHIVE), **pin(read(ARCHIVE))),
        manifest=pin(raw_manifest), members=len(all_bodies), cache_files=len(bodies), packages=len(packages),
        expanded_bytes=sum(map(len, all_bodies.values())), cargo_execution=False), sort_keys=True))


def stage(path, archive_sha, manifest_sha):
    require(os.getuid() == os.geteuid() == 9661 and ROOT.resolve(strict=True) == ROOT, 'fresh task-owned remote root')
    home = ROOT / 'cargo-home'
    require(not os.path.lexists(home) and not os.path.lexists(ROOT / 'cargo-cache-manifest.json')
            and not os.path.lexists(ROOT / 'cargo-cache-stage-complete.json'), 'exclusive cache staging')
    raw = read(path)
    require(pin(raw)['sha256'] == archive_sha, 'actual cache archive SHA')
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        members = tar.getmembers()
        require(0 < len(members) <= 128 and len({m.name for m in members}) == len(members)
                and sum(m.size for m in members) <= 64 << 20, 'outer cache bounds')
        require(all(m.isfile() and relative(m.name) and not m.pax_headers and 0 <= m.size <= 8 << 20
                    for m in members), 'regular closed USTAR members only')
        bodies = {m.name: tar.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'outer member bytes')
    raw_manifest = bodies.pop('manifest.json')
    require(pin(raw_manifest)['sha256'] == manifest_sha, 'actual cache manifest SHA')
    manifest = parse(raw_manifest)
    locks = {role: bodies.pop(role + '.lock') for role in LOCKS}
    require(all(read(ROOT / LOCKS[role][0]) == body for role, body in locks.items()), 'both staged locks byte equal')
    validate(manifest, bodies, locks)
    home.mkdir(mode=0o700)
    for name, body in sorted(bodies.items()):
        target = home / name
        target.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(body)
        require(pin(read(target)) == manifest['files'][name], 'staged immutable cache pin')
    with (ROOT / 'cargo-cache-manifest.json').open('xb') as stream:
        stream.write(raw_manifest)
    result = dict(schema='ferric-guarded-mlp-reusable-arena-cache-stage-v1', passed=True,
        archive=pin(raw), manifest=pin(raw_manifest), locks=manifest['locks'], cargo_home=str(home),
        files=manifest['files'], packages=39, cache_files=len(bodies), shared_cache_changed=False,
        lock_changed=False, project_code_executed=False, crate_sources_extracted=False,
        controller=pin(read(Path(__file__).resolve())))
    with (ROOT / 'cargo-cache-stage-complete.json').open('xb') as stream:
        stream.write(encoded(result))
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) in (2, 5),
            'python3 -B cache.py pack | stage ARCHIVE ARCHIVE_SHA MANIFEST_SHA')
    os.umask(0o077)
    def interrupted(number, _frame):
        raise RuntimeError('cache helper signal ' + str(number))
    for number in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(number, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 180)
    try:
        if sys.argv[1:] == ['pack']:
            pack()
        else:
            require(len(sys.argv) == 5 and sys.argv[1] == 'stage'
                    and all(re.fullmatch('[0-9a-f]{64}', value) for value in sys.argv[3:]), 'closed cache CLI')
            stage(Path(sys.argv[2]), sys.argv[3], sys.argv[4])
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)

