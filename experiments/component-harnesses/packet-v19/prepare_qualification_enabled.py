"""Disabled, create-only current55c V19 packet harness qualification under G36."""

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import tarfile


D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
ENABLED = True
ARCHIVE = D / 'inputs/packet-v19-harness-a003/source.tar.gz'
ARCHIVE_SHA = None
STAGE_ROOT = D / 'packet-v19-55c-harness-a003'
ROOT = STAGE_ROOT / 'source'
GUARD = D / 'owner/cpu_guard_36g_emitter.py'
ENVIRONMENT = D / 'owner/cpu-env-36g-emitter.sh'
PINS = {GUARD: 'fba93769c349a1bca0ea72ff2dbc0ed9b077be712dc8f370775d050df96af796',
        ENVIRONMENT: 'a4f373e86b56bfc692b56d726280fc1f7efbfa521db96e936dd92787d7b2c7b8'}
SOURCES = (
    'harness/launch_contract.py', 'harness/prepare_stage.py', 'harness/run_stage.py',
    'harness/test_native_launch.py', 'harness/test_cpu_binding.py',
    'harness/controller_binding.py', 'harness/runtime_binding.py',
    'harness/controller_qualifier.py', 'harness/v19_qualifier.py', 'harness/fixtures/source-files.json',
    'harness/fixtures/source-modes.json', 'harness/measurement/native_token_cell.py',
    'harness/measurement/abba_ledger.py', 'harness/measurement/gpu_activity.py',
    'harness/measurement/native_campaign_replay.py', 'harness/measurement/native_lifecycle.py',
    'harness/measurement/packet_ticks.py', 'harness/measurement/test_gpu_activity.py',
    'harness/measurement/test_native_lifecycle.py', 'bind_build.py',
    'test_packet_ticks.py', 'test_packet_v19_cell.py', 'prepare_qualification.py',
)
ACTIONS = {'launch-tests': ('harness', 'test_*.py', 31),
           'measurement-tests': ('harness/measurement', 'test_*.py', 130),
           'packet-replay-tests': ('.', 'test_packet*.py', 24)}
STAGE_ENVELOPE = 32 * 1024 * 1024
TEST_ENVELOPE = 64 * 1024 * 1024
STAGE_CAP = 38654705664
STAGE_RESERVE = 536870912



def require(value, message):
    if not value:
        raise ValueError(message)


def read(path, expected=None):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input')
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK)
    try:
        before = os.fstat(descriptor)
        require(stat.S_ISREG(before.st_mode) and before.st_uid == os.getuid()
                and 0 < before.st_size <= 1024 * 1024, 'bounded owned regular input')
        raw = bytearray()
        while part := os.read(descriptor, 1024 * 1024 + 1 - len(raw)):
            raw.extend(part)
            require(len(raw) <= 1024 * 1024, 'input grew')
        after = os.fstat(descriptor)
        current = path.lstat()
        fields = ('st_dev', 'st_ino', 'st_mode', 'st_uid', 'st_size', 'st_mtime_ns', 'st_ctime_ns')
        require(len(raw) == before.st_size and all(getattr(before, field) == getattr(after, field)
                == getattr(current, field) for field in fields), 'input changed')
        digest = hashlib.sha256(raw).hexdigest()
        require(expected is None or digest == expected, 'input hash differs')
        return bytes(raw), digest
    finally:
        os.close(descriptor)


def environment():
    os.umask(0o077)
    require(os.uname().nodename == 'sharkmi300x-3' and os.getuid() == 1046
            and os.environ.get('FERRIC_CPU_PROFILE') == 'FerricCpuFourCore36GiBEmitterV1',
            'fixed G36 CPU host/profile')
    require(os.sched_getaffinity(0) == {0, 1, 2, 3}
            and os.getpriority(os.PRIO_PROCESS, 0) == 19
            and os.environ.get('CARGO_BUILD_JOBS') == '4'
            and os.environ.get('RUST_TEST_THREADS') == '1'
            and all(os.environ.get(key) == '-1' for key in
                ('CUDA_VISIBLE_DEVICES', 'HIP_VISIBLE_DEVICES', 'HSA_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES')),
            'unchanged CPU placement and GPU masks')
    require(STAGE_ROOT.parent.resolve(strict=True) == STAGE_ROOT.parent
            and STAGE_ROOT.parent.stat().st_uid == os.getuid(),
            'canonical owned parent')
    for path, digest in PINS.items():
        read(path, digest)


def payload():
    require(type(ARCHIVE_SHA) is str and len(ARCHIVE_SHA) == 64, 'reviewed archive pin pending')
    raw, _ = read(ARCHIVE, ARCHIVE_SHA)
    files, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        for member in archive:
            name = member.name
            require(len(files) < len(SOURCES) and member.isfile() and name not in files
                    and str(Path(name)) == name and not Path(name).is_absolute()
                    and '..' not in Path(name).parts and 0 < member.size <= 512 * 1024,
                    'bounded canonical distinct regular archive member')
            total += member.size
            require(total <= 2 * 1024 * 1024, 'bounded expanded source')
            files[name] = archive.extractfile(member).read(member.size + 1)
            require(len(files[name]) == member.size, 'exact source member size')
    require(set(files) == set(SOURCES), 'exact 23-file source archive roster')
    roster = {name: hashlib.sha256(value).hexdigest() for name, value in files.items()}
    return files, roster, total


def check_sources(files):
    require(ROOT.resolve(strict=True) == ROOT and ROOT.stat().st_uid == os.getuid(),
            'canonical owned fixture stage')
    expected_dirs = {str(parent) for name in files for parent in Path(name).parents if str(parent) != '.'}
    observed_files, observed_dirs, pending = set(), set(), [ROOT]
    while pending:
        directory = pending.pop()
        with os.scandir(directory) as entries:
            for entry in entries:
                relative = str(Path(entry.path).relative_to(ROOT))
                value = entry.stat(follow_symlinks=False)
                require(value.st_uid == os.getuid(), 'foreign source entry')
                if stat.S_ISDIR(value.st_mode):
                    require(relative in expected_dirs and relative not in observed_dirs,
                            'unexpected source directory')
                    observed_dirs.add(relative)
                    pending.append(Path(entry.path))
                else:
                    require(stat.S_ISREG(value.st_mode) and value.st_nlink == 1
                            and relative in files and relative not in observed_files,
                            'unexpected, linked, or special source file')
                    observed_files.add(relative)
    require(observed_files == set(files) and observed_dirs == expected_dirs,
            'exact complete recursive source file set')
    observed = {}
    for name, data in files.items():
        path = ROOT / name
        require(path.lstat().st_nlink == 1, 'private source file required')
        _, observed[name] = read(path, hashlib.sha256(data).hexdigest())
    require(sum(path.stat().st_size for path in (ROOT / name for name in files)) < 2 * 1024 * 1024,
            'bounded complete source bytes')
    return observed


def allocation(allowance=0):
    completed = subprocess.run(['/usr/bin/du', '-s', '-B1', '--', str(D)],
                               check=False, capture_output=True, timeout=60)
    require(completed.returncode == 0 and not completed.stderr, 'bounded stage allocation read failed')
    fields = completed.stdout.decode('ascii').strip().split()
    require(len(fields) == 2 and fields[0].isdigit() and fields[1] == str(D),
            'exact stage allocation output')
    allocated = int(fields[0])
    require(allocated <= STAGE_CAP - STAGE_RESERVE - allowance,
            'unchanged G36 reserve plus fresh fixed role planning envelope required')
    return {'stage_allocated_bytes': allocated, 'stage_cap_bytes': STAGE_CAP,
            'stage_reserve_bytes': STAGE_RESERVE,
            'remaining_reserved_bytes': STAGE_CAP - STAGE_RESERVE - allocated,
            'planned_increment_bytes': allowance}


def stage():
    files, roster, total = payload()
    require(not STAGE_ROOT.exists() and not STAGE_ROOT.is_symlink(), 'create-only owned stage')
    before_allocation = allocation(STAGE_ENVELOPE)
    require(total + ARCHIVE.stat().st_size + 8 * 1024 * 1024 < STAGE_ENVELOPE,
            '32MiB incremental source and log allowance')
    STAGE_ROOT.mkdir(mode=0o700)
    ROOT.mkdir(mode=0o700)
    for name, data in files.items():
        path = ROOT / name
        path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        with path.open('xb') as output:
            output.write(data)
        path.chmod(0o600)
        read(path, hashlib.sha256(data).hexdigest())
    observed_sources = check_sources(files)
    after_allocation = allocation()
    require(after_allocation['stage_allocated_bytes'] - before_allocation['stage_allocated_bytes'] <= STAGE_ENVELOPE,
            'observed source-stage growth exceeded planning allowance')
    environment()
    receipt = {'schema': 'FerricV19PacketHarnessStageV1',
        'archive': {'path': str(ARCHIVE), 'sha256': ARCHIVE_SHA},
        'root': str(ROOT), 'files': len(files),
        'source_files': len(roster), 'expanded_bytes': total,
        'source_after': observed_sources, 'allocation_before': before_allocation,
        'allocation_after': after_allocation,
        'native_qualified': False, 'native_executed': False, 'tests_executed': False}
    with (STAGE_ROOT / 'staging.json').open('x', encoding='utf-8') as output:
        json.dump(receipt, output, indent=2, sort_keys=True)
        output.write('\n')
    print(json.dumps(receipt, sort_keys=True))


def tests(action):
    files, _, _ = payload()
    before_allocation = allocation(TEST_ENVELOPE)
    before = check_sources(files)
    relative, pattern, authored_count = ACTIONS[action]
    argv = ['/usr/bin/python3', '-I', '-B', '-m', 'unittest', 'discover',
            '-s', str(ROOT / relative), '-p', pattern, '-v']
    completed = None
    try:
        completed = subprocess.run(argv, check=False, timeout=1100)
    finally:
        after = check_sources(files)
        after_allocation = allocation()
        require(before == after, 'qualification mutated source')
        require(after_allocation['stage_allocated_bytes'] - before_allocation['stage_allocated_bytes'] <= TEST_ENVELOPE,
                'observed test-phase growth exceeded planning allowance')
        environment()
        receipt = {'schema': 'FerricV19PacketHarnessCustodyV1',
                   'action': action, 'argv': argv, 'source_before': before, 'source_after': after,
                   'source_root': str(ROOT), 'archive_sha256': ARCHIVE_SHA,
                   'helper_sha256': read(Path(__file__).resolve(strict=True))[1],
                   'allocation_before': before_allocation, 'allocation_after': after_allocation,
                   'authored_test_count': authored_count,
                   'returncode': None if completed is None else completed.returncode,
                   'native_executed': False, 'native_qualified': False}
        path = STAGE_ROOT / (action + '-custody.json')
        with path.open('x', encoding='utf-8') as output:
            json.dump(receipt, output, indent=2, sort_keys=True)
            output.write('\n')
    return completed.returncode


def main():
    global ARCHIVE_SHA
    require(ENABLED, 'disabled pending baseline harness source transport and review')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('stage', *ACTIONS))
    parser.add_argument('--archive-sha256', required=True)
    args = parser.parse_args()
    require(re.fullmatch('[0-9a-f]{64}', args.archive_sha256), 'exact authored archive hash')
    ARCHIVE_SHA = args.archive_sha256
    environment()
    if args.action == 'stage':
        stage()
        return 0
    return tests(args.action)


if __name__ == '__main__':
    raise SystemExit(main())
