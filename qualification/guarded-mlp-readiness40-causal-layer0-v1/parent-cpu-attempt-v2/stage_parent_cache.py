"""Stage a closed locked Cargo cache without executing Cargo or checking out Git."""
import hashlib
import io
import json
import os
from pathlib import Path
import re
import signal
import stat
import struct
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-readiness40-causal-layer0-parent-cpu-v228-v2'
CARGO_HOME = ROOT / 'cargo-home'
LOCK = ROOT / 'ferric/adapters/m1-engineering-execution-v1/Cargo.lock'
ARCHIVE = E / 'guarded-mlp-parent-cache-v228-v2.tar.gz'
INDEX = 'index.crates.io-1949cf8c6b5b557f'
REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index'
LOCK_PIN = dict(bytes=51664, sha256='ec06e964ed72dc9b97d9f5769bd867bf05398781b6e17a9f605137742d6e19ea')
LOCKED_ROSTER_SHA = 'd7627b4708fe0182c8df93dff59647f88a5fd8b7937ef7daf5c6047bc378540c'
BASELINE_CACHE = dict(
    archive=dict(bytes=39383327, sha256='18b1320fa9b4cc5c1e1a6db22b6da546f35edb00ba37ac5668f774e71eed69d2'),
    manifest=dict(bytes=96016, sha256='e8c533b038db432d45a31d461651a50998d0b94f42ecfc678c5823d00ffa8c81'),
    change='refs-heads-main')
GIT = (
    dict(url='https://github.com/harsh-nod/fe2o3.git',
         revision='faaaf15d68eff996b22951758b1a9fa83317d6d2',
         commit='faaaf15d68eff996b22951758b1a9fa83317d6d2',
         database='git/db/fe2o3-3973306df8c37287', object_count=5930, object_bytes=75356603),
    dict(url='https://github.com/harsh-nod/pliron.git',
         revision='cc902cc8c669b5de2b292ae8638d9e8311bc735b',
         commit='cc902cc8c669b5de2b292ae8638d9e8311bc735b',
         database='git/db/pliron-6f963b380d48781e', object_count=230, object_bytes=3034362),
    dict(url='https://github.com/verus-lang/verus.git', revision='b677dd5',
         commit='b677dd5a766f25f56e9aa1e32621aa4e53304b47',
         database='git/db/verus-e4ebf515fa1de14c', object_count=1563, object_bytes=27368046),
)
GIT_CONFIG = b'[core]\n\tbare = true\n\trepositoryformatversion = 0\n\tfilemode = true\n'


def require(value, message):
    if not value:
        raise RuntimeError(message)


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def read(path, limit=128 << 20):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and 0 <= before.st_size <= limit, 'bounded ordinary cache input')
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


def pack_index(pack, index, count, expected_ids=None):
    require(len(pack) >= 32 and pack[:4] == b'PACK'
            and struct.unpack('>II', pack[4:12]) == (2, count)
            and hashlib.sha1(pack[:-20]).digest() == pack[-20:], 'Git pack identity/count/checksum')
    require(index[:8] == b'\xfftOc\x00\x00\x00\x02'
            and len(index) == 8 + 1024 + count * 28 + 40
            and hashlib.sha1(index[:-20]).digest() == index[-20:]
            and index[-40:-20] == pack[-20:], 'Git v2 index identity/checksums/32-bit offsets')
    fanout = struct.unpack('>256I', index[8:1032])
    ids = [index[1032 + i * 20:1052 + i * 20] for i in range(count)]
    require(ids == sorted(set(ids)) and fanout[-1] == count
            and all(fanout[i] == sum(oid[0] <= i for oid in ids) for i in range(256)), 'Git index object roster')
    offsets = struct.unpack('>' + str(count) + 'I', index[1032 + count * 24:1032 + count * 28])
    require(len(set(offsets)) == count and all(12 <= offset < len(pack) - 20 for offset in offsets), 'Git object offset bound')
    names = [oid.hex() for oid in ids]
    if expected_ids is not None:
        require(names == expected_ids, 'packed object roster differs from exact commit tree')
    return hashlib.sha256(('\n'.join(names) + '\n').encode()).hexdigest()


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 3
            and all(re.fullmatch(r'[0-9a-f]{64}', arg) for arg in sys.argv[1:]),
            'python3 -B stage_parent_cache.py ARCHIVE_SHA MANIFEST_SHA')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(ROOT.resolve(strict=True) == ROOT and not os.path.lexists(CARGO_HOME)
            and not os.path.lexists(ROOT / 'cargo-cache-manifest.json')
            and not os.path.lexists(ROOT / 'cargo-cache-stage-complete.json'), 'fresh private parent cache')
    lock = read(LOCK)
    require(pin(lock) == LOCK_PIN, 'do not change the parent lockfile')
    archive = read(ARCHIVE)
    require(pin(archive)['sha256'] == sys.argv[1], 'actual parent cache archive SHA')
    with tarfile.open(fileobj=io.BytesIO(archive), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == 249 and len({m.name for m in members}) == 249
                and all(m.isfile() and not Path(m.name).is_absolute() and '..' not in Path(m.name).parts
                        and Path(m.name).as_posix() == m.name and 0 <= m.size <= 64 << 20 for m in members)
                and sum(m.size for m in members) <= 128 << 20, 'bounded unique ordinary cache members')
        bodies = {m.name: tar.extractfile(m).read() for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'cache member extent')
    require(bodies['Cargo.lock'] == lock and pin(bodies['manifest.json'])['sha256'] == sys.argv[2], 'lock/manifest body join')
    value = json.loads(bodies['manifest.json'])
    require(set(value) == {'schema', 'lock', 'registry', 'index', 'config', 'packages', 'git_commits', 'files',
                           'inner_members', 'inner_expanded_bytes', 'git_objects', 'git_object_bytes', 'cargo_execution', 'baseline_cache'}
            and value['schema'] == 'ferric-guarded-mlp-parent-cache-v1'
            and value['lock'] == LOCK_PIN and value['registry'] == REGISTRY and value['index'] == INDEX
            and value['cargo_execution'] is False and value['baseline_cache'] == BASELINE_CACHE
            and len(value['packages']) == 118
            and len(value['git_commits']) == 3 and len(value['files']) == 247, 'closed parent cache manifest')
    locked_rows = sorted((dict(name=row['name'], version=row['version'], checksum=row['checksum'], source=REGISTRY)
                          for row in value['packages']), key=lambda row: (row['name'], row['version']))
    canonical = (json.dumps(locked_rows, sort_keys=True, separators=(',', ':')) + '\n').encode()
    require(hashlib.sha256(canonical).hexdigest() == LOCKED_ROSTER_SHA, 'registry roster differs from exact lock')
    allowed, archives, indexes, package_keys, inventory = set(), set(), set(), [], []
    for row in value['packages']:
        require(set(row) == {'name', 'version', 'checksum', 'archive', 'index', 'members', 'expanded_bytes'}
                and re.fullmatch(r'[a-z0-9_-]+', row['name'])
                and re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:[+-][a-zA-Z0-9.-]+)?', row['version'])
                and re.fullmatch(r'[0-9a-f]{64}', row['checksum']), 'closed registry package identity')
        package = row['name'] + '-' + row['version']
        archive_name = 'registry/cache/' + INDEX + '/' + package + '.crate'
        sparse_name = 'registry/index/' + INDEX + '/.cache/' + index_name(row['name'])
        require(row['archive'] == archive_name and row['index'] == sparse_name and archive_name not in archives,
                'unique package archive and exact sparse name')
        archives.add(archive_name)
        indexes.add(sparse_name)
        allowed.update((archive_name, sparse_name))
        package_keys.append((row['name'], row['version']))
        require(pin(bodies[archive_name])['sha256'] == row['checksum'], 'locked registry archive checksum')
        choices = [json.loads(chunk) for chunk in bodies[sparse_name].split(b'\0') if chunk.startswith(b'{')]
        chosen = [item for item in choices if item.get('vers') == row['version']]
        require(len(chosen) == 1 and chosen[0]['name'] == row['name'] and chosen[0]['cksum'] == row['checksum'],
                'sparse record joins locked archive')
        actual = crate_inventory(bodies[archive_name], package)
        require(actual == {key: row[key] for key in actual}, 'inner registry archive inventory')
        inventory.append(actual)
    config = 'registry/index/' + INDEX + '/config.json'
    allowed.add(config)
    require(value['config'] == config and len(archives) == 118 and len(indexes) == 110 and len(allowed) == 229
            and package_keys == sorted(set(package_keys))
            and json.loads(bodies[config]) == dict(dl='https://static.crates.io/crates', api='https://crates.io')
            and sum(row['members'] for row in inventory) == value['inner_members'] == 5023
            and sum(row['expanded_bytes'] for row in inventory) == value['inner_expanded_bytes'] == 85992791,
            'closed registry config/expansion')
    for row, expected in zip(value['git_commits'], GIT):
        require(set(row) == set(expected) | {'source', 'tree', 'object_ids_sha256', 'pack', 'index'}
                and all(row[key] == item for key, item in expected.items())
                and row['source'] == 'git+' + row['url'] + '?rev=' + row['revision'] + '#' + row['commit']
                and re.fullmatch(r'[0-9a-f]{40}', row['tree'])
                and re.fullmatch(r'[0-9a-f]{64}', row['object_ids_sha256']), 'exact locked Git identity/closure')
        database, commit = row['database'], row['commit']
        pack, index = bodies[row['pack']], bodies[row['index']]
        basename = database + '/objects/pack/pack-' + pack[-20:].hex()
        require(row['pack'] == basename + '.pack' and row['index'] == basename + '.idx'
                and pack_index(pack, index, row['object_count']) == row['object_ids_sha256'], 'Git pack/index body joins')
        ids = {index[1032 + i * 20:1052 + i * 20].hex() for i in range(row['object_count'])}
        require(commit in ids and row['tree'] in ids, 'locked commit/root tree absent from pack')
        fixed = {database + '/config': GIT_CONFIG,
                 database + '/HEAD': b'ref: refs/heads/main\n',
                 database + '/shallow': (commit + '\n').encode(),
                 database + '/refs/heads/main': (commit + '\n').encode()}
        git_names = set(fixed) | {row['pack'], row['index']}
        require(not (allowed & git_names) and all(bodies[name] == body for name, body in fixed.items()),
                'closed bare Git database metadata; no hooks/remotes/checkouts')
        allowed.update(git_names)
    require(value['git_objects'] == 7723 and value['git_object_bytes'] == 105759011
            and len(allowed) == 247 and set(value['files']) == allowed
            and set(bodies) == allowed | {'Cargo.lock', 'manifest.json'}
            and all(pin(bodies[name]) == expected for name, expected in value['files'].items()), 'closed immutable cache files')
    os.umask(0o077)
    CARGO_HOME.mkdir(mode=0o700)
    for name in sorted(allowed):
        path = CARGO_HOME / name
        path.parent.mkdir(parents=True, exist_ok=True)
        require(path.parent.resolve(strict=True) == path.parent, 'cache destination alias')
        with path.open('xb') as stream:
            stream.write(bodies[name])
        require(pin(read(path)) == value['files'][name], 'staged cache body verification')
    require(read(LOCK) == lock and read(ARCHIVE) == archive, 'lock/archive changed during staging')
    with (ROOT / 'cargo-cache-manifest.json').open('xb') as stream:
        stream.write(bodies['manifest.json'])
    result = dict(schema='ferric-guarded-mlp-parent-cache-stage-v1', passed=True,
                  archive=dict(path=str(ARCHIVE), **pin(archive)), manifest=pin(bodies['manifest.json']),
                  cargo_home=str(CARGO_HOME), lock=LOCK_PIN, files=value['files'], packages=118,
                  cache_files=247, inner_members=5023, inner_expanded_bytes=85992791,
                  git_commits=value['git_commits'], git_objects=7723, git_object_bytes=105759011,
                  baseline_cache=BASELINE_CACHE,
                  shared_cache_changed=False, lock_changed=False, project_code_executed=False,
                  crate_sources_extracted=False, git_sources_checked_out=False)
    with (ROOT / 'cargo-cache-stage-complete.json').open('x') as stream:
        json.dump(result, stream, sort_keys=True, indent=2)
        stream.write('\n')
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    def interrupted(signum, _frame):
        raise RuntimeError('parent cache staging interrupted: ' + str(signum))
    for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(signum, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 180)
    try:
        main()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)


