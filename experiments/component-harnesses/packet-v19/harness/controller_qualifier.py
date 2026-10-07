"""Disabled current55c baseline packet controller qualification and retention."""
import datetime
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tarfile
import time
import tomllib

ENABLED = True
D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
I = D / 'inputs/packet-baseline-55c-a004'
O = D / 'client-packet-baseline-55c-a004'
S = O / 'source'
OUT = O / 'qualification'
SELF = I / 'qualify_controller.py'
ADAPTER = 'adapters/m1-engineering-execution-v1/'
M = S / ADAPTER / 'Cargo.toml'
BINARY = 'ferric-qwen3-ordered64-baseline-packet-ticks'
TARGET = D / 'target-fence-client-a001'
TOOLS = D / 'client-toolchain'
REVISION = '55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9'
PROFILE = 'FerricCpuFourCore36GiBEmitterV1'
Q = D / 'inputs/component-ordered-cpu-a001/qualify_source.py'
Q_SHA = 'b84d2f83eaffd7adf4f9aa760f88dd44d7fe4b2a58c23ea8a57d7beb3cfd0297'
A = D / 'inputs/native32-qualification-g36-a001/qualify_client_g36_a005_enabled.py'
A_SHA = '5f29d7dd42f913334e9467465a674ac94ec05aaa6f4e6c187d3927421de0c354'
BASE = D / 'client-native32-a003/repair-policy-a001/ferric-native32-55c1a9b6-policy-a001.tar.gz'
BASE_SHA = '6089abea6ebc3363557c8e5fb8b4e2e488fa49b4f30f99029dfcc16b7c5bddd7'
MANIFEST_SHA = '5953b9a99f886e154be6ccaa31c5d4fa59ab398b65d7327db4496baf5444f74b'
LOCK_SHA = '137f98661f7c5a154ba1d83b6dcab7da51a026dbacb2c24a9bf8a5d0e8a37b93'
RUSTFMT_SHA = '30de9e1efcd8f8fe7750e00d0c45ff8f4c480608ef1be5baf9ab6f1b4556e8f8'
OVERLAYS = {
    ADAPTER + 'src/bin/tp_worker.rs': (
        '6a8bce8aa8ccf5fea0b146c004b8feede2dffd5eff1f791cca52adc0af5f6084',
        '49b0eaf5af06a62fd533db713108c4a6b7c8f232831622023224cc4f82f53a04'),
    ADAPTER + 'src/model_timestamps.rs': (
        '9fcd96de320e7078cc9c8ffa8082bc33d2b842927c4ee0dc6cf89c1f8280d944',
        'f9a58b388b1646c6cdfa3ed73fbf9eced9a27d1141d133b4f0cb97e9b5534e83'),
    ADAPTER + 'src/model_timestamps/tests.rs': (
        '389f2abacd923ac94de5685b0c7abf31f039dca3ba0dfcf4f59b21f132811d84',
        'cf0d091b71a991cd4a7372c24793aaa3f18450e19abf7f6781df43ac960751be'),
}
ALLOWANCES = {'stage': 128, 'format': 16, 'timestamps': 512, 'graph': 32,
              'parser': 192, 'source-policy': 640, 'clippy': 192,
              'release': 512, 'retention': 64}
TEST_COUNTS = {'timestamps': 32, 'graph': 3, 'parser': 4, 'source-policy': 58}
ROLES = tuple(ALLOWANCES)


def require(value, message):
    if not value:
        raise ValueError(message)


def helper(path, digest):
    require(path.resolve(strict=True) == path, 'canonical helper')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_uid == 1046
            and before.st_nlink == 1 and 0 < before.st_size <= 64 * 1024,
            'bounded owned helper')
    raw = path.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == digest, 'exact reviewed helper hash')
    module = importlib.util.module_from_spec(importlib.util.spec_from_file_location(path.stem, path))
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def write_json(path, value):
    raw = (json.dumps(value, sort_keys=True, indent=2) + '\n').encode()
    with path.open('xb') as stream:
        stream.write(raw)
    return hashlib.sha256(raw).hexdigest()


def source_inputs(a):
    raw, _ = a.read(BASE, BASE_SHA)
    expected = a.json_file(a.CHECKPOINT / 'source-files.json', a.SOURCE_PINS['source-files.json'])
    modes = a.json_file(a.CHECKPOINT / 'source-modes.json', a.SOURCE_PINS['source-modes.json'])
    files, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        for ordinal, member in enumerate(archive, 1):
            relative = Path(member.name)
            require(ordinal <= 10000 and not relative.is_absolute()
                    and '..' not in relative.parts, 'bounded safe source member')
            if member.isdir():
                continue
            name = str(relative)
            require(member.isfile() and name in expected and name not in files,
                    'exact distinct regular source member')
            require(0 <= member.size <= 32 * 1024**2, 'bounded source member')
            total += member.size
            require(total <= 96 * 1024**2, 'bounded complete source expansion')
            value = archive.extractfile(member).read(member.size + 1)
            require(len(value) == member.size
                    and hashlib.sha256(value).hexdigest() == expected[name]
                    and stat.S_IMODE(member.mode) == modes[name], 'pinned complete base source')
            files[name] = value
    require(len(files) == 1433 and set(files) == set(expected) == set(modes), 'full source closure')
    for name, (old, new) in OVERLAYS.items():
        require(expected[name] == old, 'exact three-file preimage')
        files[name] = a.read(I / 'after' / name, new)[0]
        expected[name] = new
    return files, expected, modes


def source_check(a, expected, modes):
    require(S.resolve(strict=True) == S and S.lstat().st_uid == 1046, 'private canonical source')
    observed = {}
    for path in sorted(S.rglob('*')):
        info = path.lstat()
        require(not path.is_symlink() and info.st_uid == 1046, 'owned real source entry')
        if stat.S_ISDIR(info.st_mode):
            continue
        name = str(path.relative_to(S))
        require(name in expected and stat.S_ISREG(info.st_mode) and info.st_nlink == 1
                and stat.S_IMODE(info.st_mode) == modes[name], 'exact source member/mode')
        observed[name] = a.read(path, expected[name], empty=True)[1]
    require(observed == expected, 'complete exact source roster')
    a.read(M, MANIFEST_SHA)
    a.read(S / ADAPTER / 'Cargo.lock', LOCK_SHA)
    manifest = tomllib.loads(a.read(M)[0].decode())
    wire = manifest['dependencies']['fe2o3-kfd-current-wire']
    require(wire['package'] == 'fe2o3-kfd' and wire['rev'] == REVISION, 'current55c wire dependency')
    binary = [row for row in manifest['bin'] if row['name'] == BINARY]
    require(len(binary) == 1 and binary[0]['path'] == 'src/bin/' + BINARY + '.rs'
            and binary[0]['required-features'] == ['c1-ordered64', 'model-timestamps'],
            'existing baseline diagnostic binary')
    return observed


def commands(role):
    common = ['--locked', '--offline', '--manifest-path', str(M)]
    features = ['--features', 'c1-ordered64,model-timestamps']
    tail = ['--', '--test-threads=1']
    if role == 'format':
        return [[str(TOOLS / 'bin/rustfmt'), '--edition', '2024', '--config',
                 'skip_children=true', '--check', str(S / name)] for name in OVERLAYS]
    if role in ('timestamps', 'graph'):
        selector = 'model_timestamps::tests' if role == 'timestamps' else 'ordered64_baseline_packet_ticks_v1'
        command = ['test', *common, *features, '--lib', selector, *tail]
    elif role == 'parser':
        command = ['test', *common, *features, '--bin', BINARY,
                   'ordered64_baseline_packet_ticks_contract::tests', *tail]
    elif role == 'source-policy':
        command = ['test', *common, '--no-default-features', '--test', 'source_policy', *tail]
    elif role == 'clippy':
        command = ['clippy', *common, *features, '--lib', '--bin', BINARY, '--', '-D', 'warnings']
    elif role == 'release':
        command = ['build', '-v', '--release', *common, *features, '--bin', BINARY]
    else:
        require(role in ('stage', 'retention'), 'closed role')
        return []
    return [[str(TOOLS / 'bin/cargo'), *command]]


def outer_directory(role):
    return D / ('results/pages-packet-baseline-55c-' + role + '-g36-a004')


def previous(a, role, helper_sha, source_sha):
    inner_path = OUT / role / 'receipt.json'
    inner = a.json_file(inner_path)
    require(inner['schema'] == 'FerricBaselinePacket55cCpuRoleV1' and inner['role'] == role
            and inner['base_archive_sha256'] == BASE_SHA and inner['runtime_revision'] == REVISION
            and inner['helper_sha256'] == helper_sha and inner['source_roster_sha256'] == source_sha
            and inner['source_verified_after'] is True and inner['returncode'] == 0
            and inner['commands'] == commands(role), 'exact prior inner role')
    directory = outer_directory(role)
    outer = a.json_file(directory / 'result.json')
    wanted = {'status': 0, 'returncode': 0, 'reason': 'completed', 'child_reaped': True,
              'cleanup_ok': True, 'errors': [], 'term_sent': False, 'kill_sent': False,
              'log_limit_exceeded': False, 'profile': PROFILE, 'cwd': str(D),
              'argv': ['/bin/bash', str(D / 'owner/cpu-env-36g-emitter.sh'),
                       '/usr/bin/python3', '-I', '-B', str(SELF), role]}
    require(all(type(outer.get(key)) is type(value) and outer[key] == value
                for key, value in wanted.items()), 'exact clean prior G36 result')
    require(a.read(directory / 'exit.status')[0] == b'0\n', 'raw prior exit status')
    stdout = a.read(directory / 'stdout', empty=True)[0]
    stderr = a.read(directory / 'stderr', empty=True)[0]
    if role in TEST_COUNTS:
        footers = re.findall(rb'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured;',
                             stdout, flags=re.MULTILINE)
        require(footers == [(str(TEST_COUNTS[role]).encode(), b'0', b'0', b'0')],
                'exact executed passing test count and no skipped tests')
    return {'inner': {'path': str(inner_path), 'sha256': a.read(inner_path)[1]},
            'result': {'path': str(directory / 'result.json'), 'sha256': a.read(directory / 'result.json')[1]},
            'stdout_sha256': hashlib.sha256(stdout).hexdigest(),
            'stderr_sha256': hashlib.sha256(stderr).hexdigest()}, inner, outer


def retain(a, expected, modes, helper_sha, source_sha):
    phases = {role: previous(a, role, helper_sha, source_sha)[0] for role in ROLES[:-1]}
    _, release, outer = previous(a, 'release', helper_sha, source_sha)
    built = TARGET / 'release' / BINARY
    raw, sha = a.read(built, release['artifacts']['controller']['sha256'], maximum=32 * 1024**2)
    require(raw.startswith(b'\x7fELF') and built.stat().st_mode & 0o111, 'actual executable ELF')
    start = datetime.datetime.fromisoformat(outer['started']).timestamp()
    finish = datetime.datetime.fromisoformat(outer['finished']).timestamp()
    require(start <= built.stat().st_mtime <= finish, 'ELF produced within dedicated release phase')
    directory = O / 'retained'
    require(not os.path.lexists(directory), 'create-only final retention')
    directory.mkdir(mode=0o700)
    binary = directory / BINARY
    with binary.open('xb') as stream:
        stream.write(raw)
    binary.chmod(0o500)
    archive_path = directory / 'ferric-packet-baseline-55c1a9b6-a004.tar.gz'
    with tarfile.open(archive_path, mode='x:gz') as archive:
        for name in sorted(expected):
            value = a.read(S / name, expected[name], empty=True)[0]
            item = tarfile.TarInfo(name)
            item.size, item.mode, item.mtime = len(value), modes[name], 0
            archive.addfile(item, io.BytesIO(value))
    observed, archive_modes = {}, {}
    with tarfile.open(archive_path, 'r:gz') as archive:
        for member in archive:
            require(member.isfile() and member.name in expected and member.name not in observed,
                    'exact retained regular archive member')
            value = archive.extractfile(member).read(member.size + 1)
            require(len(value) == member.size, 'retained source member size')
            observed[member.name] = hashlib.sha256(value).hexdigest()
            archive_modes[member.name] = stat.S_IMODE(member.mode)
    require((observed, archive_modes) == (expected, modes), 'complete retained source content/modes')
    a.read(built, sha, maximum=32 * 1024**2)
    a.read(binary, sha, maximum=32 * 1024**2)
    return {'controller': {'path': str(binary), 'sha256': sha, 'size_bytes': len(raw)},
            'source_archive': {'path': str(archive_path), 'sha256': a.read(archive_path)[1],
                               'size_bytes': archive_path.stat().st_size},
            'phases': phases}


def main():
    require(ENABLED, 'disabled pending exact source/qualification helper review')
    require(len(sys.argv) == 2 and sys.argv[1] in ROLES, 'one closed role required')
    role = sys.argv[1]
    q = helper(Q, Q_SHA)
    q.environment()
    a = helper(A, A_SHA)
    require(Path(__file__).resolve(strict=True) == SELF, 'fixed qualifier path')
    helper_sha = a.read(SELF)[1]
    for path, digest in ((Q, Q_SHA), (A, A_SHA), (TOOLS / 'bin/rustfmt', RUSTFMT_SHA)):
        a.read(path, digest)
    for name, digest in a.TOOLS.items():
        a.read(TOOLS / 'bin' / name, digest)
    require(os.environ.get('CARGO_HOME') == str(D / 'cargo-home'), 'fixed offline Cargo home')
    for key in ('CARGO_INCREMENTAL', 'CARGO_PROFILE_DEV_DEBUG', 'CARGO_PROFILE_TEST_DEBUG',
                'CARGO_PROFILE_RELEASE_DEBUG'):
        require(os.environ.get(key) == '0', 'bounded Cargo profile')
    require(TARGET.resolve(strict=True) == TARGET and TARGET.stat().st_uid == 1046,
            'canonical owned warm target')
    allowance = ALLOWANCES[role] * 1024**2
    before = q.allocation(allowance)
    files, expected, modes = source_inputs(a)
    source_raw = (json.dumps(expected, sort_keys=True, indent=2) + '\n').encode()
    source_sha = hashlib.sha256(source_raw).hexdigest()
    if role == 'stage':
        require(not os.path.lexists(O), 'create-only successor source')
        O.mkdir(mode=0o700)
        S.mkdir(mode=0o700)
        for name, value in files.items():
            path = S / name
            path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            with path.open('xb') as stream:
                stream.write(value)
            path.chmod(modes[name])
        write_json(O / 'source-files.json', expected)
        write_json(O / 'source-modes.json', modes)
        OUT.mkdir(mode=0o700)
    else:
        require(O.resolve(strict=True) == O and O.stat().st_uid == 1046
                and OUT.resolve(strict=True) == OUT and OUT.stat().st_uid == 1046,
                'existing owned successor outputs')
        a.read(O / 'source-files.json', source_sha)
        require(a.json_file(O / 'source-modes.json') == modes, 'exact source modes roster')
        for prior in ROLES[:ROLES.index(role)]:
            previous(a, prior, helper_sha, source_sha)
    source_check(a, expected, modes)
    output = OUT / role
    output.mkdir(mode=0o700)
    environment = dict(os.environ, PATH=str(TOOLS / 'bin') + ':/usr/bin:/bin',
        RUSTC=str(TOOLS / 'bin/rustc'), RUSTDOC=str(TOOLS / 'bin/rustdoc'),
        LD_LIBRARY_PATH=str(TOOLS / 'lib') + ':' + str(TOOLS / 'lib/rustlib/x86_64-unknown-linux-gnu/lib'),
        RUSTC_BOOTSTRAP='fe2o3_macros,fe2o3_device', CARGO_TARGET_DIR=str(TARGET))
    for name in ('FERRIC_TOKEN_ABI_GRAPH_OUTPUT', 'FERRIC_TOKEN_ABI_GRAPH',
                 'FERRIC_TOKEN_ABI_IMAGES', 'FERRIC_V8_TEST_ARTIFACT'):
        environment.pop(name, None)
    artifacts = {}
    started = time.monotonic()
    try:
        for argv in commands(role):
            result = subprocess.run(argv, cwd=S, env=environment, check=False,
                                    timeout=max(1, 1100 - (time.monotonic() - started)))
            require(result.returncode == 0, 'qualification command failed; preserve raw evidence')
        if role == 'release':
            path = TARGET / 'release' / BINARY
            raw, digest = a.read(path, maximum=32 * 1024**2)
            require(raw.startswith(b'\x7fELF'), 'actual dedicated ELF')
            artifacts['controller'] = {'path': str(path), 'sha256': digest, 'size_bytes': len(raw)}
        elif role == 'retention':
            artifacts = retain(a, expected, modes, helper_sha, source_sha)
    finally:
        source_check(a, expected, modes)
        source_inputs(a)
        a.read(SELF, helper_sha)
        a.read(Q, Q_SHA)
        a.read(A, A_SHA)
        a.read(TOOLS / 'bin/rustfmt', RUSTFMT_SHA)
        for name, digest in a.TOOLS.items():
            a.read(TOOLS / 'bin' / name, digest)
        q.environment()
        after = q.allocation()
        require(after['stage_allocated_bytes'] - before['stage_allocated_bytes'] <= allowance,
                'observed role growth exceeded fixed allowance')
    receipt = {'schema': 'FerricBaselinePacket55cCpuRoleV1', 'role': role,
        'source': str(S), 'source_files': len(expected), 'source_roster_sha256': source_sha,
        'base_archive_sha256': BASE_SHA, 'overlays': OVERLAYS, 'runtime_revision': REVISION,
        'helper_sha256': helper_sha, 'profile': PROFILE, 'commands': commands(role),
        'returncode': 0, 'source_verified_after': True, 'allocation_before': before,
        'allocation_after': after, 'planning_allowance_bytes': allowance, 'artifacts': artifacts,
        'native_executed': False, 'latency_sample_admitted': False,
        'scope': 'CPU qualification only; prior test roles bind actual raw passing counts'}
    write_json(output / 'receipt.json', receipt)
    print(json.dumps(receipt, sort_keys=True))


if __name__ == '__main__':
    main()
