"""Fixed G36 Python-only split-K harness qualification; no Rust or native work."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess

D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
ROOT = D / 'splitk-model-harness-a003'
OUTPUT = D / 'splitk-model-harness-results-a003'
GUARD = D / 'owner/cpu_guard_36g_emitter.py'
ENVIRONMENT = D / 'owner/cpu-env-36g-emitter.sh'
PINS = {GUARD: 'fba93769c349a1bca0ea72ff2dbc0ed9b077be712dc8f370775d050df96af796',
        ENVIRONMENT: 'a4f373e86b56bfc692b56d726280fc1f7efbfa521db96e936dd92787d7b2c7b8'}
SOURCES = (
    "bind_build.py",
    "harness/experimental_evidence.py",
    "harness/fixtures/frozen_run_v17_native.py",
    "harness/fixtures/frozen_run_v25_native.py",
    "harness/fixtures/retained_cpu.json",
    "harness/launch_contract.py",
    "harness/measurement/abba_ledger.py",
    "harness/measurement/gpu_activity.py",
    "harness/measurement/native_campaign_replay.py",
    "harness/measurement/native_lifecycle.py",
    "harness/measurement/native_token_cell.py",
    "harness/measurement/splitk_selection.py",
    "harness/measurement/test_abba_ledger.py",
    "harness/measurement/test_gpu_activity.py",
    "harness/measurement/test_native_campaign_replay.py",
    "harness/measurement/test_native_lifecycle.py",
    "harness/measurement/test_native_token_cell.py",
    "harness/prepare_stage.py",
    "harness/run_stage.py",
    "harness/test_cpu_binding.py",
    "harness/test_native_launch.py",
    "harness/test_splitk_selection.py"
)
ACTIONS = {'launch-tests': ('harness', 60), 'measurement-tests': ('harness/measurement', 214)}
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
    require(ROOT.parent.resolve(strict=True) == ROOT.parent and ROOT.parent.stat().st_uid == os.getuid(),
            'canonical owned parent')
    for path, digest in PINS.items():
        read(path, digest)



def payload():
    raw, _ = read(ROOT / 'source-roster.json')
    record = json.loads(raw)
    require(set(record) == {'schema', 'source_files'} and record['schema'] == 'FerricSplitKAuthoredSourceV1'
            and type(record['source_files']) is dict and set(record['source_files']) == set(SOURCES),
            'exact source manifest')
    files = {'source-roster.json': raw}
    for name, digest in record['source_files'].items():
        require(type(digest) is str and re.fullmatch('[0-9a-f]{64}', digest), 'source SHA256')
        files[name] = read(ROOT / name, digest)[0]
    return files

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



def tests(role):
    before_allocation = allocation(TEST_ENVELOPE)
    files = payload()
    before = check_sources(files)
    relative, count = ACTIONS[role]
    command = ['/usr/bin/python3', '-I', '-B', '-m', 'unittest', 'discover',
               '-s', str(ROOT / relative), '-p', 'test_*.py', '-v']
    OUTPUT.mkdir(mode=0o700, exist_ok=True)
    require(OUTPUT.resolve(strict=True) == OUTPUT and OUTPUT.stat().st_uid == os.getuid(), 'owned result root')
    output = OUTPUT / role
    output.mkdir(mode=0o700)
    completed, error = None, None
    try:
        with (output / 'test-stdout').open('xb') as stdout, (output / 'test-stderr').open('xb') as stderr:
            completed = subprocess.run(command, stdout=stdout, stderr=stderr, timeout=1100, check=False)
        require(completed.returncode == 0, 'fixture command failed')
    except BaseException as failure:
        error = type(failure).__name__ + ': ' + str(failure)
    finally:
        after = check_sources(files)
        after_allocation = allocation()
        require(before == after, 'qualification mutated source')
        require(after_allocation['stage_allocated_bytes'] - before_allocation['stage_allocated_bytes'] <= TEST_ENVELOPE,
                'observed test-phase growth exceeded planning allowance')
        environment()
        output_text = (output / 'test-stdout').read_bytes()
        error_text = (output / 'test-stderr').read_bytes()
        require(len(output_text) + len(error_text) <= 1024 * 1024, 'bounded fixture output')
        # Empty test stdout is expected; stderr carries the full unittest footer.
        passed = completed is not None and completed.returncode == 0 and error is None
        passed = passed and not output_text and re.search(
            rb'\nRan ' + str(count).encode() + rb' tests? in [0-9.]+s\n\nOK\n\Z', error_text) is not None
        receipt = {'schema': 'FerricSplitKHarnessCpuV1', 'role': role,
            'profile': 'FerricCpuFourCore36GiBEmitterV1', 'command': command,
            'returncode': None if completed is None else completed.returncode,
            'helper_sha256': read(Path(__file__).resolve())[1],
            'source_before': {name.removeprefix('harness/'): digest for name, digest in before.items() if name.startswith('harness/')},
            'source_after': {name.removeprefix('harness/'): digest for name, digest in after.items() if name.startswith('harness/')},
            'full_source_before': before, 'full_source_after': after,
            'allocation_before': before_allocation, 'allocation_after': after_allocation,
            'planning_allowance_bytes': TEST_ENVELOPE, 'tests': count,
            'test_stdout': output_text.decode(), 'test_stderr': error_text.decode(),
            'native_executed': False, 'accepted': passed, 'error': error}
        with (output / 'receipt.json').open('x') as stream:
            json.dump(receipt, stream, indent=2, sort_keys=True)
            stream.write('\n')
        print(json.dumps(receipt, sort_keys=True), flush=True)
    return 0 if passed else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('role', choices=ACTIONS)
    args = parser.parse_args()
    environment()
    return tests(args.role)


if __name__ == '__main__':
    raise SystemExit(main())
