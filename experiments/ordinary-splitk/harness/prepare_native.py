#!/usr/bin/env python3
"""Restore exact split-K delta, rebind portable paths and prepare; never launch."""
import hashlib
import io
import json
import os
from pathlib import Path
import socket
import stat
import subprocess
import tarfile
import time

TRANSPORT = Path('/dev/shm/ferric-splitk-model-inputs-a001')
PAYLOAD = TRANSPORT / 'payload'
STAGE = Path('/dev/shm/ferric-native-splitk-down-a001')
BASE = Path('/dev/shm/ferric-prefill-width-transport-a001/base')
WIDTH = Path('/dev/shm/ferric-native-prefill-width-a003')
ARCHIVE_SHA = '602ba82f2b57b7de396eb2e35b6068d4b8752d2b143617efab69b3e8efaa1f28'
RECEIPT_SHA = '18cf00c5b85fb4f9bcb9e796218351d441d623c883bd3ee9e0d94ce8a8c5a06c'
MANIFEST_SHA = '25cb311e3ccc51ed62f11479653ec6f90d9088ef1b215c79e484940164afcb02'
CONFIG_SHA = '20888465c7e7443bfc9713f7eb94b489edc4ad5fb85c4020043e71e35bf3efea'
WORKER_SHA = 'af246e5b872b639b90906336493629eb2bbfa90ff57ffc4ffd859f8b963d9b30'
CORE_SHA = 'a9772e55f9a140d0749a621c07ea2c9cb19c36b2416656278dd8d2a2095321d3'


def require(value, message):
    if not value:
        raise ValueError(message)


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()


def read(path, digest, limit, empty=False):
    require(path.resolve(strict=True) == path, 'canonical input')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    with os.fdopen(fd, 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_uid == 9661 and before.st_nlink == 1
                and (empty or before.st_size > 0) and before.st_size <= limit, 'bounded owned input')
        raw = stream.read(limit + 1)
        after = os.fstat(stream.fileno())
    current = path.lstat()
    fields = ('st_dev', 'st_ino', 'st_uid', 'st_mode', 'st_nlink', 'st_size', 'st_mtime_ns', 'st_ctime_ns')
    require(len(raw) == before.st_size and all(getattr(before, k) == getattr(after, k)
            == getattr(current, k) for k in fields) and hashlib.sha256(raw).hexdigest() == digest,
            'stable exact input: ' + str(path))
    return raw


def resources():
    values = {}
    for name, path, floor in (('root_free_bytes', '/', 64 * 1024**3),
                              ('shm_free_bytes', '/dev/shm', 32 * 1024**3)):
        info = os.statvfs(path)
        values[name] = info.f_bavail * info.f_frsize
        require(values[name] >= floor, name + ' floor')
    fields = dict(line.split(':', 1) for line in Path('/proc/meminfo').read_text().splitlines())
    values['memory_available_bytes'] = int(fields['MemAvailable'].split()[0]) * 1024
    require(values['memory_available_bytes'] >= 128 * 1024**3, 'RAM floor')
    return values


def save(path, raw):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'wb') as stream:
        stream.write(raw)


def main():
    require(socket.gethostname() == 'smci350-rck-g03-b19-03' and os.getuid() == 9661,
            'fixed native host/UID')
    os.umask(0o077)
    started = time.monotonic()
    initial = resources()
    require(TRANSPORT.resolve(strict=True) == TRANSPORT and TRANSPORT.stat().st_uid == 9661
            and stat.S_IMODE(TRANSPORT.stat().st_mode) == 0o700, 'private transport root')
    require(not os.path.lexists(PAYLOAD) and not os.path.lexists(STAGE), 'fresh payload/native stage')
    receipt = json.loads(read(TRANSPORT / 'export-receipt.json', RECEIPT_SHA, 1024**2))
    manifest = json.loads(read(TRANSPORT / 'export-inputs-a001.json', MANIFEST_SHA, 1024**2))
    require(receipt['accepted'] is True and receipt['native_executed'] is False
            and receipt['manifest_sha256'] == MANIFEST_SHA
            and receipt['archive']['sha256'] == ARCHIVE_SHA
            and manifest['schema'] == 'FerricSplitKPortableExportV1'
            and len(manifest['files']) == 113, 'qualified exact export')
    files = receipt['files']
    require(len(files) == 89 and sum(x['bytes'] for x in files.values()) == 20458657,
            'exact bounded portable roster')
    archive_raw = read(TRANSPORT / 'splitk-model-delta-a001.tar.gz', ARCHIVE_SHA, 12021725)
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as archive:
        members = archive.getmembers()
        require(len(members) == 89 and len({x.name for x in members}) == 89
                and {x.name for x in members} == set(files), 'exact unique archive members')
        for member in members:
            name = Path(member.name)
            require(member.isfile() and not name.is_absolute() and '..' not in name.parts
                    and str(name) == member.name and member.mode == 0o600
                    and member.size == files[member.name]['bytes'], 'closed regular archive')
        PAYLOAD.mkdir(mode=0o700)
        for member in members:
            require(time.monotonic() - started < 120, 'bounded preparation deadline')
            raw = archive.extractfile(member).read(member.size + 1)
            require(len(raw) == member.size and hashlib.sha256(raw).hexdigest()
                    == files[member.name]['sha256'], 'exact archive member bytes')
            path = PAYLOAD / member.name
            path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            require(path.parent.resolve(strict=True).is_relative_to(PAYLOAD), 'owned member parent')
            save(path, raw)

    def check_payload():
        actual = {str(path.relative_to(PAYLOAD)) for path in PAYLOAD.rglob('*') if not path.is_dir()}
        require(actual == set(files), 'complete unchanged payload file roster')
        for name, item in files.items():
            read(PAYLOAD / name, item['sha256'], item['bytes'], empty=True)
        read(WIDTH / 'worker-candidate', WORKER_SHA, 64 * 1024**2)
        read(WIDTH / 'evidence/runtime_source.archive', CORE_SHA, 64 * 1024**2)

    check_payload()
    config = json.loads(read(PAYLOAD / 'binding/inputs.json', CONFIG_SHA, 1024**2))
    replacements = {path: str(PAYLOAD / item['relative']) for path, item in manifest['files'].items()}
    require(set(manifest['borrowed'].values()) == {WORKER_SHA, CORE_SHA}, 'closed borrowed artifacts')
    for path, digest in manifest['borrowed'].items():
        replacements[path] = str(WIDTH / ('worker-candidate' if digest == WORKER_SHA
                                          else 'evidence/runtime_source.archive'))
    old_image = config['splitk']['image']['path']
    replacements[old_image] = str(PAYLOAD / 'image')

    def remap(value):
        if isinstance(value, dict):
            return {key: replacements[item] if key == 'path' else remap(item)
                    for key, item in value.items()}
        if isinstance(value, list):
            return [remap(item) for item in value]
        return value

    native_config = remap(config)
    config_raw = encoded(native_config)
    config_path = TRANSPORT / 'native-config.json'
    save(config_path, config_raw)

    def run(name, argv):
        resources()
        remaining = 120 - (time.monotonic() - started)
        require(remaining > 0, 'bounded preparation deadline')
        with (TRANSPORT / (name + '.stdout')).open('xb') as stdout, \
                (TRANSPORT / (name + '.stderr')).open('xb') as stderr:
            result = subprocess.run(argv, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr,
                                    timeout=remaining, check=False)
        save(TRANSPORT / (name + '.status'), (str(result.returncode) + '\n').encode())
        out, err = TRANSPORT / (name + '.stdout'), TRANSPORT / (name + '.stderr')
        require(result.returncode == 0 and out.stat().st_size <= 65536 and err.stat().st_size == 0,
                name + ' refused; raw retained')
        return json.loads(out.read_bytes())

    binding = run('bind', ['/usr/bin/python3', '-I', '-B', str(PAYLOAD / 'bind_build.py'),
        '--config', str(config_path), '--config-sha256', hashlib.sha256(config_raw).hexdigest(),
        '--output', str(TRANSPORT / 'native-binding')])
    require(binding['accepted'] is True and binding['native_executed'] is False, 'binding scope')
    read(BASE / 'counter-plan.json', 'd91216856d50fdb22ea92ad67766af44b92120af3c5afec500617991af35516d', 1024**2)
    argv = ['/usr/bin/python3', '-I', '-B', str(PAYLOAD / 'harness/prepare_stage.py'),
        '--stage', str(STAGE), '--base-root', str(BASE), '--v19-root', str(WIDTH),
        '--kv-image', str(WIDTH / 'native-inputs/v19/fe2o3-engineering-v1/0f1dc95268aca76a3aeaf69eda200875d317b5d4dbf26af7f9dcf96a4a9f581a'),
        '--measurement-root', str(PAYLOAD / 'harness/measurement'),
        '--build-manifest', binding['build']['path'], '--build-manifest-sha256', binding['build']['sha256']]
    prepared = run('prepare', argv)
    check_payload()
    read(TRANSPORT / 'splitk-model-delta-a001.tar.gz', ARCHIVE_SHA, 12021725)
    final = {'schema': 'FerricSplitKNativePreparationV1', 'native_executed': False,
        'files_verified_before_after': 89, 'archive_sha256': ARCHIVE_SHA,
        'export_receipt_sha256': RECEIPT_SHA, 'binding': binding, 'prepared': prepared,
        'argv': argv, 'initial_resources': initial, 'final_resources': resources()}
    save(TRANSPORT / 'preparation-receipt.json', encoded(final))
    print(encoded(final).decode(), end='')


if __name__ == '__main__':
    main()
