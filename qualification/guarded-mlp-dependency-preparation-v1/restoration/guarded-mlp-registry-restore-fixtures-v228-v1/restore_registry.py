"""Restore only the exact missing source packages from the failed locked fetch."""
import ctypes
import fcntl
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import resource
import shutil
import signal
import stat
import sys
import tarfile
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
FETCH = E / 'guarded-mlp-fetch-v228-v1/evidence'
OUT = E / 'guarded-mlp-registry-restore-v228-v1'
REGISTRY = Path('/home/harmenon/.cargo/registry')
SECTIONS = ('src', 'cache', 'index')
MAX_FILES, MAX_BYTES = 200000, 256 << 20
EXPECTED_PACKAGES, EXPECTED_FILES, EXPECTED_BYTES = 133, 9095, 229085242


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def stamp(s):
    return (s.st_dev, s.st_ino, s.st_mode, s.st_uid, s.st_nlink,
            s.st_size, s.st_mtime_ns, s.st_ctime_ns)


def pin(path, retain=False):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'noncanonical body')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_uid == 9661
            and before.st_size <= MAX_BYTES, 'body type/owner/extent')
    digest, pieces = hashlib.sha256(), []
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC), 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'body changed before read')
        for body in iter(lambda: stream.read(1 << 20), b''):
            digest.update(body)
            if retain:
                pieces.append(body)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'body changed during read')
    require(stamp(path.lstat()) == stamp(before), 'body changed after read')
    return dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest()), b''.join(pieces)


def document(path, expected):
    value, body = pin(path, True)
    require(value == expected or value['sha256'] == expected, 'document pin differs')
    def pairs(items):
        result = {}
        for key, item in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = item
        return result
    return json.loads(body, object_pairs_hook=pairs)


def relative(name):
    p = PurePosixPath(name)
    require(name and p.parts and not p.is_absolute() and str(p) == name and '..' not in p.parts
            and '\\' not in name and '\x00' not in name, 'unsafe relative member')
    return p.parts


def snapshot():
    result, count = {}, 0
    for section in SECTIONS:
        root, rows = REGISTRY / section, {}
        require(root.is_dir() and root.resolve() == root, 'registry section absent or aliased')
        for directory, dirs, names in os.walk(root, followlinks=False,
                onerror=lambda error: (_ for _ in ()).throw(error)):
            require(all(not (Path(directory) / n).is_symlink() for n in dirs), 'directory alias')
            for name in names:
                path = Path(directory) / name
                count += 1
                require(count <= MAX_FILES, 'registry file-count bound')
                rows[str(path.relative_to(root))] = pin(path)[0]
        result[section] = rows
    return result


def missing_packages(before, after):
    require(set(before) == set(after) == set(SECTIONS), 'registry snapshot sections')
    for section in SECTIONS:
        for name, value in before[section].items():
            relative(name)
            require(value['path'] == str(REGISTRY / section / name), 'before namespace')
            if name in after[section] and section != 'index':
                require(after[section][name] == value, 'existing source/archive changed')
        require(section == 'src' or section == 'index'
                or set(before[section]) <= set(after[section]), 'archive removed')
    missing = sorted(set(before['src']) - set(after['src']))
    groups = {}
    for name in missing:
        parts = relative(name)
        require(len(parts) >= 3, 'source package path')
        package, member = '/'.join(parts[:2]), '/'.join(parts[2:])
        groups.setdefault(package, {})[member] = before['src'][name]
    for package, rows in groups.items():
        original = {n[len(package) + 1:]: v for n, v in before['src'].items()
                    if n.startswith(package + '/')}
        require(rows == original and not any(n.startswith(package + '/') for n in after['src']),
                'package is not wholly missing')
    return groups


def stage_package(archive, archive_pin, package, expected, destination, marker):
    require(pin(archive)[0] == archive_pin, 'archive changed before staging')
    destination.mkdir(mode=0o700)
    seen, directories = set(), set()
    with tarfile.open(archive, mode='r|gz') as source:
        for count, entry in enumerate(source, 1):
            require(count <= MAX_FILES and not entry.pax_headers and not entry.issparse(),
                    'unsupported/oversized tar headers')
            parts = relative(entry.name.rstrip('/') if entry.isdir() else entry.name)
            require(parts[0] == package and (entry.isdir() or entry.isfile()), 'tar root/type')
            name = '/'.join(parts[1:])
            if entry.isdir():
                require(entry.name not in directories, 'duplicate tar directory')
                directories.add(entry.name)
                continue
            require(name in expected and name != '.cargo-ok' and name not in seen,
                    'unexpected or duplicate tar body')
            wanted = expected[name]
            require(entry.size == wanted['bytes'] and entry.size <= MAX_BYTES, 'tar body extent')
            stream = source.extractfile(entry)
            require(stream is not None, 'missing tar body')
            body = stream.read(entry.size + 1)
            require(len(body) == entry.size and hashlib.sha256(body).hexdigest() == wanted['sha256'],
                    'tar body hash differs from original source')
            path = destination / name
            path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            with path.open('xb') as output:
                output.write(body)
            os.chmod(path, 0o700 if entry.mode & 0o111 else 0o600)
            seen.add(name)
    require(seen == set(expected) - {'.cargo-ok'}, 'archive/source member census differs')
    wanted = expected['.cargo-ok']
    require(len(marker) == wanted['bytes'] and hashlib.sha256(marker).hexdigest() == wanted['sha256'],
            'marker donor differs from original marker')
    with (destination / '.cargo-ok').open('xb') as output:
        output.write(marker)
    require(pin(archive)[0] == archive_pin, 'archive changed after staging')
    verify_stage(destination, expected)


def verify_stage(destination, expected):
    actual = {}
    for directory, dirs, names in os.walk(destination, followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(all(not (Path(directory) / n).is_symlink() for n in dirs), 'staged directory alias')
        for name in names:
            path = Path(directory) / name
            require(len(actual) < MAX_FILES, 'staged roster bound')
            value = pin(path)[0]
            actual[str(path.relative_to(destination))] = (value['bytes'], value['sha256'])
    require(actual == {n: (v['bytes'], v['sha256']) for n, v in expected.items()}, 'staged body census drift')


def rename_noreplace(source, destination):
    require(source.parent.resolve() == source.parent and destination.parent.resolve() == destination.parent,
            'rename parent alias')
    libc = ctypes.CDLL(None, use_errno=True)
    rename = libc.renameat2
    rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
    rename.restype = ctypes.c_int
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
    a, b = os.open(source.parent, flags), os.open(destination.parent, flags)
    try:
        if rename(a, os.fsencode(source.name), b, os.fsencode(destination.name), 1) != 0:
            raise OSError(ctypes.get_errno(), 'renameat2 RENAME_NOREPLACE refused', str(destination))
    finally:
        os.close(a)
        os.close(b)


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch('[0-9a-f]{64}', sys.argv[1]), 'supply actual failed-fetch SHA')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'wrong restoration host/UID')
    require(not os.path.lexists(OUT) and E.resolve() == E and REGISTRY.resolve() == REGISTRY,
            'fresh canonical output/registry required')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    require(os.getpriority(os.PRIO_PROCESS, 0) in (0, 10), 'unexpected nice')
    if os.getpriority(os.PRIO_PROCESS, 0) == 0:
        os.nice(10)
    for kind, limit in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 300),
                        (resource.RLIMIT_FSIZE, MAX_BYTES), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        cap = min([limit] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (cap, cap))
    require(shutil.disk_usage(E).free >= 40 << 30, 'initial40GiB floor')
    OUT.mkdir(mode=0o700)
    stage = OUT / 'stage'
    stage.mkdir(mode=0o700)
    started, locks, installed, archives = time.monotonic(), [], [], {}
    acknowledged_renames = []
    failure, postchecks, before, after, groups, donor = None, [], None, None, {}, None
    def interrupted(signum, _frame):
        raise RuntimeError('restoration interrupted: ' + str(signum))
    for sig in (signal.SIGALRM, signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        signal.signal(sig, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 540)
    try:
        for name in ('.package-cache', '.package-cache-mutate'):
            path = REGISTRY.parent / name
            require(path.resolve(strict=True) == path, 'cache lock alias')
            fd = os.open(path, os.O_RDWR | os.O_NOFOLLOW | os.O_CLOEXEC)
            locks.append(fd)
            require(stat.S_ISREG(os.fstat(fd).st_mode) and os.fstat(fd).st_uid == 9661,
                    'cache lock type/owner')
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        failed = document(FETCH / 'failed.json', sys.argv[1])
        require(failed['schema'] == 'ferric-guarded-mlp-nightly-fetch-v1' and not failed['passed']
                and failed['failure'] is None and len(failed['postcheck_errors']) == 1
                and failed['postcheck_errors'][0].startswith('registry source/archive custody and index delta:')
                and len(failed['phases']) == 1, 'not the closed successful-fetch/custody refusal')
        leaf = failed['phases'][0]
        require(leaf['exit_code'] == 0 and leaf['natural_exit'] and leaf['reaped']
                and leaf['process_group_absent'] and not leaf['forced_cleanup']
                and not leaf['timed_out'] and leaf['exception'] is None, 'fetch leaf not naturally complete')
        before = document(FETCH / 'registry-before.json', failed['raw']['registry-before.json'])
        after = document(FETCH / 'registry-after.json', failed['raw']['registry-after.json'])
        require(snapshot() == after, 'current registry differs from actual post-fetch snapshot')
        groups = missing_packages(before, after)
        require(len(groups) == EXPECTED_PACKAGES and sum(map(len, groups.values())) == EXPECTED_FILES
                and sum(p['bytes'] for rows in groups.values() for p in rows.values()) == EXPECTED_BYTES,
                'exact133/9095/229085242 repair census')
        marker_pin = next(iter(groups.values()))['.cargo-ok']
        donors = [v for n, v in after['src'].items() if n.endswith('/.cargo-ok')
                  and before['src'].get(n) == v and all(v[k] == marker_pin[k] for k in ('bytes', 'sha256'))]
        require(donors, 'no unchanged actual marker donor')
        donor = sorted(donors, key=lambda p: p['path'])[0]
        actual, marker = pin(Path(donor['path']), True)
        require(actual == donor, 'marker donor drift')
        for package, rows in sorted(groups.items()):
            registry, name = package.split('/')
            destination = REGISTRY / 'src' / package
            require(not os.path.lexists(destination) and destination.parent.stat().st_dev == stage.stat().st_dev,
                    'destination exists or cross-filesystem staging')
            key = package + '.crate'
            require(before['cache'].get(key) == after['cache'].get(key) and key in before['cache'],
                    'archive is not unchanged since before fetch')
            archives[key] = before['cache'][key]
            target = stage / registry / name
            target.parent.mkdir(mode=0o700, exist_ok=True)
            stage_package(REGISTRY / 'cache' / key, archives[key], name, rows, target, marker)
        require(snapshot() == after, 'registry changed during full staging')
        for package in sorted(groups):
            require(shutil.disk_usage(E).free >= 38 << 30, 'live38GiB floor')
            verify_stage(stage / package, groups[package])
            rename_noreplace(stage / package, REGISTRY / 'src' / package)
            acknowledged_renames.append(package)
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        if after is not None:
            try:
                signal.setitimer(signal.ITIMER_REAL, max(0.001, 600 - (time.monotonic() - started)))
                current = snapshot()
                # Recover the rename/append interruption window from actual complete package bodies.
                for package, rows in sorted(groups.items()):
                    present = {n[len(package) + 1:]: v for n, v in current['src'].items()
                               if n.startswith(package + '/')}
                    if present:
                        require(present == rows, 'partially restored or changed package: ' + package)
                        installed.append(package)
                expected = {s: dict(after[s]) for s in SECTIONS}
                for package in installed:
                    expected['src'].update({package + '/' + n: v for n, v in groups[package].items()})
                require(current == expected, 'post-restoration current/unaffected body census drift')
                for key, expected_pin in archives.items():
                    require(pin(REGISTRY / 'cache' / key)[0] == expected_pin, 'archive posthash drift')
                if donor is not None:
                    require(pin(Path(donor['path']))[0] == donor, 'donor posthash drift')
            except BaseException as error:
                postchecks.append(repr(error))
            finally:
                signal.setitimer(signal.ITIMER_REAL, 0)
        for fd in reversed(locks):
            os.close(fd)
    passed = failure is None and not postchecks and len(installed) == EXPECTED_PACKAGES
    result = dict(schema='ferric-guarded-mlp-registry-restoration-v1', passed=passed,
                  failure=failure, postcheck_errors=postchecks, failed_fetch_sha256=sys.argv[1],
                  controller=pin(Path(__file__))[0], installed_packages=installed,
                  acknowledged_renames=acknowledged_renames,
                  restored_files=sum(len(groups[p]) for p in installed), archives=archives,
                  marker_donor=donor, elapsed_seconds=time.monotonic() - started,
                  original_failure_preserved=True, overwrites=False, cargo_executed=False,
                  source_byte_restoration_only=True, original_modes_not_recorded=True)
    with (OUT / ('complete.json' if passed else 'failed.json')).open('x') as stream:
        json.dump(result, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(pin(OUT / ('complete.json' if passed else 'failed.json'))[0]), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
