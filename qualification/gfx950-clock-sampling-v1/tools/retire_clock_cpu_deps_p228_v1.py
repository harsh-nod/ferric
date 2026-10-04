"""Retire only completed, explicitly selected CPU intermediate dependencies; preserve sources and executables."""
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import stat
import subprocess
import sys

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROSTER = {
    'device-parent-cpu-v228-v2': ('3b51d61268bee492dd6ded7623742153513c46cd3d246ebc60a6bc9a2555077b', 257, 0, 41, 12),
    'device-routing-cpu-v228-v2': ('407990eeac9332f4807ff71fcc7c884a0f14a92dffd44f97fd19ea7fa0b7bbf7', 609, 4, 25, 1),
    'prefix-raw-timestamps-cpu-v228-v1': ('2d53e512b2ab882b97333c1d1889780c8fed0ab2763b411dc167b8f1735ee5da', 578, 4, 25, 1),
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(path):
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical file')
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1 << 20), b''):
            digest.update(block)
    return dict(path=str(path), bytes=path.stat().st_size, sha256=digest.hexdigest())


def main():
    require(len(sys.argv) == 2 and sys.argv[1] in ROSTER and not sys.flags.optimize and sys.dont_write_bytecode,
            'ordinary standalone retirement')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'asrock-1w300-g2-2b'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'task-owned build-host identity')
    for kind, limit in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                        (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (limit, limit))
    label = sys.argv[1]
    C = E / label
    TARGET = C / 'target/debug/deps'
    OUTPUT = E / (label + '-deps-retired-for-clock-v1.json')
    digest, passed, ignored, phases, binary_count = ROSTER[label]
    complete = pin(C / 'complete.json')
    require(complete['sha256'] == digest,
            'actual completed CPU receipt')
    value = json.loads((C / 'complete.json').read_bytes())
    require(value['passed'] is True and value['tests_passed'] == passed
            and value['tests_ignored'] == ignored and len(value['phases']) == phases,
            'completed historical CPU run')
    for row in value['phases'].values():
        require(row['exit_code'] == 0 and row['reason'] is None and row['group_absent'] is True,
                'natural completed owned leaves')
    preserved = {complete['path']: complete}

    def collect(item):
        if isinstance(item, dict):
            if set(item) == {'path', 'bytes', 'sha256'}:
                require(not Path(item['path']).is_relative_to(TARGET), 'no evidence body in dependency cache')
                preserved[item['path']] = item
            for child in item.values():
                collect(child)
        elif isinstance(item, list):
            for child in item:
                collect(child)

    collect(value)
    for row in preserved.values():
        require(pin(Path(row['path'])) == row, 'original evidence identity')
    binaries = [row['binary'] for row in value['binaries'].values()]
    require(len(binaries) == binary_count, 'all original parent and worker binaries')
    dynamic = {}
    for row in binaries:
        result = subprocess.run(['/usr/bin/readelf', '-d', row['path']], check=True,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=10,
            env={'PATH': '/usr/bin:/bin', 'LANG': 'C', 'LC_ALL': 'C'})
        output = result.stdout.decode('ascii')
        require(not result.stderr and 'RPATH' not in output and 'RUNPATH' not in output,
                'executables do not require dependency directory search paths')
        needed = [line for line in output.splitlines() if '(NEEDED)' in line]
        require(needed and all(line.rsplit('[', 1)[-1].rstrip(']') in
                {'libgcc_s.so.1', 'libm.so.6', 'libc.so.6', 'ld-linux-x86-64.so.2'}
                for line in needed), 'system-only dynamic dependencies')
        dynamic[row['path']] = output
    require(TARGET.resolve(strict=True) == TARGET and TARGET.is_dir() and not os.path.lexists(OUTPUT),
            'exact existing cache and fresh receipt')
    entries = [TARGET, *TARGET.rglob('*')]
    for path in entries:
        mode = path.lstat()
        require(mode.st_uid == 9661 and not stat.S_ISLNK(mode.st_mode)
                and (stat.S_ISREG(mode.st_mode) or stat.S_ISDIR(mode.st_mode)), 'owned ordinary cache only')
    for proc in Path('/proc').iterdir():
        if not proc.name.isdigit() or int(proc.name) == os.getpid():
            continue
        try:
            if proc.stat().st_uid != 9661:
                continue
            command = (proc / 'cmdline').read_bytes()
            try:
                cwd = (proc / 'cwd').resolve(strict=True)
                environment = (proc / 'environ').read_bytes()
                mappings = (proc / 'maps').read_bytes()
                executable = os.readlink(proc / 'exe')
                descriptors = []
                for descriptor in (proc / 'fd').iterdir():
                    try:
                        descriptors.append(os.readlink(descriptor))
                    except FileNotFoundError:
                        continue
            except PermissionError:
                require(command.rstrip(b'\x00') in (b'(sd-pam)', b'sshd: harmenon@notty'),
                        'uninspectable process is not a known session broker')
                require(str(C).encode() not in command, 'broker does not select retired build')
                continue
        except (FileNotFoundError, ProcessLookupError):
            continue
        require(str(C).encode() not in command and not cwd.is_relative_to(C)
                and str(C).encode() not in environment and str(C).encode() not in mappings
                and str(C) not in executable and all(str(C) not in name for name in descriptors),
                'historical build has no active command, environment, mapping or open-file user')
    identity = dict(path=str(TARGET), inode=TARGET.stat().st_ino,
        files=sum(path.is_file() for path in entries), allocated_bytes=sum(path.lstat().st_blocks * 512 for path in entries))
    before = shutil.disk_usage(E).free
    require(before >= 38 << 30 and TARGET.stat().st_ino == identity['inode'], 'free-space floor and cache identity')
    shutil.rmtree(TARGET)
    for row in preserved.values():
        require(pin(Path(row['path'])) == row, 'all original evidence and binaries preserved')
    result = dict(schema='ferric-p228-clock-deps-retirement-v1', retired=identity,
        complete=complete, preserved_references=len(preserved), preserved_binaries=binaries,
        dynamic_sections=dynamic, free_before=before, free_after=shutil.disk_usage(E).free,
        source_removed=False, gpu_execution=False)
    with OUTPUT.open('x', encoding='ascii') as stream:
        json.dump(result, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(dict(receipt=pin(OUTPUT), retired=identity, free_after=result['free_after'])), flush=True)


if __name__ == '__main__':
    main()
