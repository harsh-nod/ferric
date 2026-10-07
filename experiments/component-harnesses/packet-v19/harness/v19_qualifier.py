"""Disabled V19 diagnostic qualification reusing the exact frozen A004 source."""
import datetime
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import time
import tomllib

ENABLED = True
D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
SELF = D / 'inputs/packet-v19-55c-a001/qualify_v19.py'
BASE_HELPER = D / 'inputs/packet-baseline-55c-a004/qualify_controller.py'
BASE_HELPER_SHA = '5d30ac68ac5c6cc0a046764e30accb74117cd077b934d5c83f3b3f617a5663b1'
O = D / 'client-packet-v19-55c-a001'
BINARY = 'ferric-qwen3-ordered64-packet-ticks'
SOURCE_ROSTER_SHA = '3a4dad3fd997dc9092a335bbdbfdaa8591748197e17417d54530a99229257fb2'
ROLES = ('graph', 'parser', 'clippy', 'release', 'retention')
TEST_COUNTS = {'graph': 3, 'parser': 4}
ALLOWANCES = {'graph': 32, 'parser': 192, 'clippy': 192, 'release': 512, 'retention': 64}


def require(value, message):
    if not value:
        raise ValueError(message)


def load_base():
    require(BASE_HELPER.resolve(strict=True) == BASE_HELPER, 'canonical fixed baseline helper')
    before = BASE_HELPER.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_uid == 1046 and before.st_nlink == 1
            and 0 < before.st_size <= 64 * 1024, 'bounded owned helper')
    raw = BASE_HELPER.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == BASE_HELPER_SHA, 'reviewed A004 helper bytes')
    after = BASE_HELPER.lstat()
    require(all(getattr(before, key) == getattr(after, key) for key in
                ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')), 'stable helper')
    value = importlib.util.module_from_spec(importlib.util.spec_from_file_location('baseline_a004', BASE_HELPER))
    exec(compile(raw, str(BASE_HELPER), 'exec'), value.__dict__)
    return value


def commands(base, role):
    common = ['--locked', '--offline', '--manifest-path', str(base.M)]
    features = ['--features', 'c1-ordered64,model-timestamps']
    if role == 'graph':
        command = ['test', *common, *features, '--lib', 'ordered64_kv_copy_packet_ticks_v1',
                   '--', '--test-threads=1']
    elif role == 'parser':
        command = ['test', *common, *features, '--bin', BINARY,
                   'ordered64_packet_ticks_contract::tests', '--', '--test-threads=1']
    elif role == 'clippy':
        command = ['clippy', *common, *features, '--lib', '--bin', BINARY, '--', '-D', 'warnings']
    elif role == 'release':
        command = ['build', '-v', '--release', *common, *features, '--bin', BINARY]
    else:
        require(role == 'retention', 'closed V19 role')
        return []
    return [[str(base.TOOLS / 'bin/cargo'), *command]]


def previous(base, a, role, helper_sha):
    inner_path = O / 'qualification' / role / 'receipt.json'
    inner = a.json_file(inner_path)
    wanted = {'schema': 'FerricV19Packet55cCpuRoleV1', 'role': role,
              'source': str(base.S), 'source_roster_sha256': SOURCE_ROSTER_SHA,
              'helper_sha256': helper_sha, 'base_helper_sha256': BASE_HELPER_SHA,
              'source_verified_after': True, 'returncode': 0,
              'commands': commands(base, role), 'native_executed': False}
    require(all(type(inner.get(key)) is type(value) and inner[key] == value for key, value in wanted.items()),
            'exact prior V19 source/command/helper receipt')
    directory = D / ('results/pages-packet-v19-55c-' + role + '-g36-a001')
    outer = a.json_file(directory / 'result.json')
    wanted = {'status': 0, 'returncode': 0, 'reason': 'completed', 'child_reaped': True,
              'cleanup_ok': True, 'errors': [], 'term_sent': False, 'kill_sent': False,
              'log_limit_exceeded': False, 'profile': base.PROFILE, 'cwd': str(D),
              'argv': ['/bin/bash', str(D / 'owner/cpu-env-36g-emitter.sh'),
                       '/usr/bin/python3', '-I', '-B', str(SELF), role]}
    require(all(type(outer.get(key)) is type(value) and outer[key] == value for key, value in wanted.items()),
            'exact prior clean G36 V19 role')
    require(a.read(directory / 'exit.status')[0] == b'0\n', 'actual prior raw status')
    stdout = a.read(directory / 'stdout', empty=True)[0]
    stderr = a.read(directory / 'stderr', empty=True)[0]
    if role in TEST_COUNTS:
        rows = re.findall(rb'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured;',
                          stdout, re.MULTILINE)
        require(rows == [(str(TEST_COUNTS[role]).encode(), b'0', b'0', b'0')], 'exact V19 graph/parser footers')
    return {'inner': {'path': str(inner_path), 'sha256': a.read(inner_path)[1]},
            'result': {'path': str(directory / 'result.json'), 'sha256': a.read(directory / 'result.json')[1]},
            'stdout_sha256': hashlib.sha256(stdout).hexdigest(),
            'stderr_sha256': hashlib.sha256(stderr).hexdigest()}, inner, outer


def main():
    require(ENABLED, 'disabled pending source review')
    require(len(sys.argv) == 2 and sys.argv[1] in ROLES, 'one exact V19 qualification role')
    role = sys.argv[1]
    base = load_base()
    guard = base.helper(base.Q, base.Q_SHA)
    guard.environment()
    a = base.helper(base.A, base.A_SHA)
    require(Path(__file__).resolve(strict=True) == SELF, 'fixed V19 helper path')
    helper_sha = a.read(SELF)[1]
    allowance = ALLOWANCES[role] * 1024**2
    before = guard.allocation(allowance)
    _, expected, modes = base.source_inputs(a)
    source_sha = hashlib.sha256((json.dumps(expected, sort_keys=True, indent=2) + '\n').encode()).hexdigest()
    require(source_sha == SOURCE_ROSTER_SHA, 'exact unchanged A004 source roster')
    base.source_check(a, expected, modes)
    a.read(base.O / 'source-files.json', SOURCE_ROSTER_SHA)
    require(a.json_file(base.O / 'source-modes.json') == modes, 'unchanged A004 modes')
    baseline = {name: base.previous(a, name, BASE_HELPER_SHA, SOURCE_ROSTER_SHA)[0] for name in base.ROLES}
    retained = a.json_file(base.OUT / 'retention/receipt.json')['artifacts']['source_archive']
    a.read(Path(retained['path']), retained['sha256'])
    manifest = tomllib.loads(a.read(base.M, base.MANIFEST_SHA)[0].decode())
    selected = [row for row in manifest['bin'] if row['name'] == BINARY]
    require(selected == [{'name': BINARY, 'path': 'src/bin/' + BINARY + '.rs',
                         'required-features': ['c1-ordered64', 'model-timestamps']}],
            'existing V19 diagnostic binary with exact features')
    require(os.environ.get('CARGO_HOME') == str(D / 'cargo-home') and all(os.environ.get(key) == '0'
            for key in ('CARGO_INCREMENTAL', 'CARGO_PROFILE_DEV_DEBUG', 'CARGO_PROFILE_TEST_DEBUG',
                        'CARGO_PROFILE_RELEASE_DEBUG')), 'unchanged offline Cargo profile')
    require(base.TARGET.resolve(strict=True) == base.TARGET and base.TARGET.stat().st_uid == 1046,
            'same canonical owned warm target')
    for name, sha in a.TOOLS.items():
        a.read(base.TOOLS / 'bin' / name, sha)
    if role == 'graph':
        require(not os.path.lexists(O), 'create-only V19 evidence directory, no source copy')
        O.mkdir(mode=0o700)
        (O / 'qualification').mkdir(mode=0o700)
    else:
        require(O.resolve(strict=True) == O and O.stat().st_uid == 1046, 'existing owned evidence')
    phases = {name: previous(base, a, name, helper_sha)[0] for name in ROLES[:ROLES.index(role)]}
    output = O / 'qualification' / role
    output.mkdir(mode=0o700)
    environment = dict(os.environ, PATH=str(base.TOOLS / 'bin') + ':/usr/bin:/bin',
        RUSTC=str(base.TOOLS / 'bin/rustc'), RUSTDOC=str(base.TOOLS / 'bin/rustdoc'),
        LD_LIBRARY_PATH=str(base.TOOLS / 'lib') + ':' + str(base.TOOLS / 'lib/rustlib/x86_64-unknown-linux-gnu/lib'),
        RUSTC_BOOTSTRAP='fe2o3_macros,fe2o3_device', CARGO_TARGET_DIR=str(base.TARGET))
    for name in ('FERRIC_TOKEN_ABI_GRAPH_OUTPUT', 'FERRIC_TOKEN_ABI_GRAPH',
                 'FERRIC_TOKEN_ABI_IMAGES', 'FERRIC_V8_TEST_ARTIFACT'):
        environment.pop(name, None)
    artifacts = {}
    started = time.monotonic()
    try:
        for argv in commands(base, role):
            result = subprocess.run(argv, cwd=base.S, env=environment, check=False,
                                    timeout=max(1, 1100 - (time.monotonic() - started)))
            require(result.returncode == 0, 'V19 command failed; preserve all raw output')
        if role in ('release', 'retention'):
            built = base.TARGET / 'release' / BINARY
            raw, sha = a.read(built, maximum=32 * 1024**2)
            require(raw.startswith(b'\x7fELF') and built.stat().st_mode & 0o111, 'actual V19 executable ELF')
            path = built
            if role == 'retention':
                _, release, outer = previous(base, a, 'release', helper_sha)
                require(release['artifacts']['controller'] == {'path': str(built), 'sha256': sha,
                                                              'size_bytes': len(raw)}, 'same actual V19 release ELF')
                start = datetime.datetime.fromisoformat(outer['started']).timestamp()
                finish = datetime.datetime.fromisoformat(outer['finished']).timestamp()
                require(start <= built.stat().st_mtime <= finish, 'ELF produced in dedicated V19 release interval')
                directory = O / 'retained'
                directory.mkdir(mode=0o700)
                path = directory / BINARY
                with path.open('xb') as stream:
                    stream.write(raw)
                path.chmod(0o500)
                a.read(path, sha, maximum=32 * 1024**2)
                a.read(built, sha, maximum=32 * 1024**2)
                artifacts.update(source_archive=retained, phases=phases)
            artifacts['controller'] = {'path': str(path), 'sha256': sha, 'size_bytes': len(raw)}
    finally:
        base.source_check(a, expected, modes)
        base.source_inputs(a)
        a.read(SELF, helper_sha)
        a.read(BASE_HELPER, BASE_HELPER_SHA)
        a.read(base.Q, base.Q_SHA)
        a.read(base.A, base.A_SHA)
        a.read(Path(retained['path']), retained['sha256'])
        for name, sha in a.TOOLS.items():
            a.read(base.TOOLS / 'bin' / name, sha)
        guard.environment()
        after = guard.allocation()
        require(after['stage_allocated_bytes'] - before['stage_allocated_bytes'] <= allowance,
                'fixed V19 incremental envelope')
    receipt = {'schema': 'FerricV19Packet55cCpuRoleV1', 'role': role, 'source': str(base.S),
        'source_files': len(expected), 'source_roster_sha256': SOURCE_ROSTER_SHA,
        'runtime_revision': base.REVISION, 'helper_sha256': helper_sha, 'base_helper_sha256': BASE_HELPER_SHA,
        'profile': base.PROFILE, 'commands': commands(base, role), 'returncode': 0,
        'source_verified_after': True, 'allocation_before': before, 'allocation_after': after,
        'planning_allowance_bytes': allowance, 'artifacts': artifacts, 'baseline_phases': baseline,
        'reused_test_roles': {'timestamps': 32, 'source-policy': 58},
        'native_executed': False, 'latency_sample_admitted': False,
        'scope': 'same A004 source and features; V19 kernel composition, not native publication timing'}
    base.write_json(output / 'receipt.json', receipt)
    print(json.dumps(receipt, sort_keys=True))


if __name__ == '__main__':
    main()
