"""Select two actual observer CPU products; reuse the frozen non-native audit."""
import hashlib
import os
from pathlib import Path
import re
import resource
import shutil
import stat
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
AUDITOR = ('p227-prefix-runtime-audit-v2', '9b80913aa0dac2da2a52bfe6e053a4db09a5e4c5647731fd269f39719e8bb062')
AUDIT_SHA = 'def16c2f69c082fa273596bef0caf7a2af195d07d2ac6f96d944706d5f44a514'
HELPER_SHA = '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820'
GPU_PACKAGE = ('p228-projection-ar4-host-observation-gpu-v1',
    '8ce9861812a27d115e68edb7f2fb0a3f6fc5aaf3ee045bd70313356aa9d333b1')
VALIDATOR_SHA = '757c077979b7ee02d4c0e7d222e75292cd9d417927f84ae9a8134344f8e217b9'
OBSERVER_CPU_SHA = 'ac69404cec3d51d733daeb81dda3f51947c4c0a8065c7644946b8e4238ea4a72'
CPU_LABEL = 'projection-ar4-host-observation-cpu-v228-v1'
CPU = (488734, '7d8c08eeffab9cbebbf6abc973c0ba93d80616ad81063b35587e74d04df1d59c')
PRIOR_CPU = dict(path=str(E / 'projection-ar4-cpu-v228-v1/complete.json'), bytes=583779,
    sha256='7a3c170ca6ffe6000517588280fcdb99cb58c0cbb22cc3f4be910232f7d51c54')
CONTROLLER_SHA = '12898f6b9aa1a1af122cf61d0393089fa38cdb898097114dbb67a36ea4fc780a'
PROPOSAL_SHA = 'ab5d055a09c320780a0d0da7d235bb0fe023469c9132b37a877fbc7841738074'
BASE_SHA = '22c23c3effbf79c1b38e8c816f43474cdc201aedb971d81bffd75c99dc5ba0a9'
BINARIES = {
    'parent': (13938936, '1fad2d8aafe9ce4e799322d42d1015756ead228bf2f237c302ee260fecb656fd'),
    'worker': (5159512, '14135b08c276ba9d38fcbae635fe14c3db8bd11b934ccf5ac8d51e7c2876bd75'),
}
NAMES = dict(parent='ferric-qwen3-finite-projection-residual-decode-host-engineering',
             worker='ferric-tp-peer-finite-engineering-worker-v1')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def bootstrap():
    path = E / AUDITOR[0] / 'run_row_facts_v2.py'
    require(path.resolve(strict=True) == path and not path.is_symlink(), 'canonical frozen custody helper')
    with path.open('rb') as stream:
        before = os.fstat(stream.fileno()); raw = stream.read((1 << 20) + 1); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and 0 < len(raw) == before.st_size <= 1 << 20
        and stamp(before) == stamp(after) == stamp(path.lstat())
        and hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'exact frozen custody helper bytes')
    module = types.ModuleType('projection_ar4_host_runtime_custody'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module, path


def read_pin(pins, record, maximum=16 << 20):
    require(type(record) is dict and set(record) == {'path', 'bytes', 'sha256'}
        and type(record['path']) is str and type(record['bytes']) is int
        and 0 < record['bytes'] <= maximum and type(record['sha256']) is str
        and re.fullmatch('[0-9a-f]{64}', record['sha256']), 'closed nonempty FilePin')
    actual, raw = pins.read(Path(record['path']), record['sha256'], retain=True, maximum=maximum)
    require(actual == record, 'exact recorded extent and digest')
    return raw


def contract_module(D, pins):
    base = D.package(pins, *GPU_PACKAGE)
    missing = object(); previous = sys.modules.get('layer_validation', missing)
    try:
        sys.modules['layer_validation'] = D.load_module(pins, base / 'layer_validation.py',
            VALIDATOR_SHA, 'observer_runtime_layer_validation')
        return D.load_module(pins, base / 'observer_cpu.py', OBSERVER_CPU_SHA, 'observer_runtime_cpu')
    finally:
        if previous is missing:
            sys.modules.pop('layer_validation', None)
        else:
            sys.modules['layer_validation'] = previous


def sources(D, pins, value, root):
    require(value['controller']['path'] == str(E / 'p228-projection-ar4-host-observation-cpu-v1/run.py')
        and value['controller']['sha256'] == CONTROLLER_SHA
        and value['proposal']['path'] == str(E / 'p228-projection-ar4-host-observation-v1/source-manifest.json')
        and value['proposal']['sha256'] == PROPOSAL_SHA, 'actual controller and proposal namespaces')
    read_pin(pins, value['controller'], 128 << 10)
    proposal = D.parse(read_pin(pins, value['proposal']))
    require(proposal['schema'] == 'ferric-p228-projection-ar4-host-observation-source-proposal-v1'
        and proposal['base_cpu'] == value['prior_completion'] == PRIOR_CPU
        and len(proposal['files']) == 17, 'exact observer overlay and CPU1037 predecessor')
    maps = {}
    for name in ('base', 'unformatted', 'before', 'after'):
        pin = value['raw']['sources-' + name + '.json']
        require(pin['path'] == str(root / ('sources-' + name + '.json')), 'original CPU source-map identity')
        maps[name] = D.parse(read_pin(pins, pin))
    require(value['raw']['sources-base.json']['sha256'] == BASE_SHA and maps['base'], 'actual CPU1037 preimage')
    expected = dict(maps['base']); changed, seen = set(), set()
    for row in proposal['files']:
        key = 'ferric/' + row['path']
        require(key not in seen and ((key not in expected) if row['before'] is None
            else maps['base'].get(key) == row['before']), 'unique exact overlay preimage')
        seen.add(key); expected[key] = row['after']
        if key.endswith('.rs'): changed.add(key)
    require(len(expected) == len(maps['base']) + 6 and len(changed) == 16
        and maps['unformatted'] == expected and set(maps['before']) == set(expected)
        and all(maps['before'][k] == v for k, v in expected.items() if k not in changed)
        and maps['before'] == maps['after'], 'only reviewed Rust formatting and no qualified source drift')


def cpu_artifact(D, pins, cpu_pin, role):
    root = E / CPU_LABEL
    require(role in NAMES and cpu_pin == dict(path=str(root / 'complete.json'), bytes=CPU[0], sha256=CPU[1]),
        'exact actual scoped855 receipt and selected role')
    value = D.parse(read_pin(pins, cpu_pin, 4 << 20))
    O = contract_module(D, pins)
    selected = O.contract(value, root)
    require(value['prior_completion'] == PRIOR_CPU, 'actual CPU1037 is the unchanged predecessor')
    sources(D, pins, value, root)
    for phase, names in (('worker-build', [O.WORKER]), ('parent-builds', [O.PLAIN, O.PARENT])):
        stream = value['raw'][phase + '-stdout']
        require(stream['path'] == str(root / (phase + '-stdout')), 'original successful build stream')
        raw = read_pin(pins, stream, 32 << 20)
        rows = [D.parse(line) for line in raw.splitlines() if line.strip()]
        artifacts = [r for r in rows if r.get('reason') == 'compiler-artifact' and r.get('executable')]
        require(hashlib.sha256(raw).hexdigest() == value['phases'][phase]['stdout_sha256']
            and [r['success'] for r in rows if r.get('reason') == 'build-finished'] == [True]
            and len(artifacts) == len(names) and {r['target']['name'] for r in artifacts} == set(names)
            and all(artifacts.count(value['binaries'][name]['artifact']) == 1 for name in names),
            'exact successful Cargo build products, not inferred executable names')
    for selected_role, row in selected.items():
        require((row['binary']['bytes'], row['binary']['sha256']) == BINARIES[selected_role]
            and row['artifact']['target']['name'] == NAMES[selected_role], 'actual two observed ELF identities')
    return selected[role]['binary']


def selected(D, args, pins):
    require(len(args) == 6, 'LABEL ROLE CPU_PATH CPU_SHA BINARY_PATH BINARY_SHA')
    label, role, cpu_path, cpu_sha, binary_path, binary_sha = args
    require(role in NAMES and re.fullmatch(
        'projection-ar4-host-observation-runtime-' + role + r'-v228-v[1-9][0-9]{0,8}', label),
        'closed role and fresh audit label')
    require(all(type(v) is str and re.fullmatch('[0-9a-f]{64}', v) for v in (cpu_sha, binary_sha)),
            'explicit actual CPU and binary SHA256')
    cpu, binary = Path(cpu_path), Path(binary_path)
    root = E / CPU_LABEL
    require(str(cpu) == cpu_path and cpu == root / 'complete.json', 'actual CPU receipt namespace')
    require(str(binary) == binary_path and binary == root / 'target' / role / 'debug' / NAMES[role],
            'exact new observer executable namespace')
    cpu_pin, _ = pins.read(cpu, cpu_sha, retain=False, maximum=4 << 20)
    original = cpu_artifact(D, pins, cpu_pin, role)
    binary_pin, raw = pins.read(binary, binary_sha, retain=True, maximum=128 << 20)
    require(binary_pin == original and os.access(binary, os.X_OK), 'actual selected executable identity/mode')
    require(len(raw) >= 20 and raw[:6] == b'\x7fELF\x02\x01' and raw[18:20] == b'\x3e\x00',
            'actual ELF64 little-endian x86_64 executable')
    return E / label, binary_pin


def main(args):
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
        and 'PYTHONPATH' not in os.environ and 'PYTHONHOME' not in os.environ
        and os.getuid() == os.geteuid() == 9661 and os.sched_getaffinity(0) == {8, 9}
        and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'same bounded runtime-audit identity')
    require(shutil.disk_usage(E.parents[1]).free >= 40 << 30, 'existing initial disk floor')
    os.umask(0o077)
    for kind, limit in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                        (resource.RLIMIT_FSIZE, 64 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (limit, limit))
    D, helper = bootstrap(); pins = D.Pins()
    pins.pin(helper, HELPER_SHA); pins.pin(Path(__file__).resolve())
    out, binary = selected(D, args, pins)
    require(not os.path.lexists(out), 'fresh audit output')
    base = D.package(pins, *AUDITOR)
    audit = D.load_module(pins, base / 'audit.py', AUDIT_SHA, 'projection_ar4_host_runtime_auditor')
    audit.host_identity()
    audit.read_pin(pins, binary, 1 << 30)
    topology = D.load_module(pins, audit.TOPOLOGY, audit.TOPOLOGY_SHA, 'projection_ar4_host_runtime_topology')
    owned = D.load_module(pins, base / 'frozen_owned.py', audit.OWNED_SHA, 'projection_ar4_host_runtime_owned')
    record = audit.execute(out, binary, pins, owned, topology)
    D.progress(dict(complete=record, gpu_execution=False, reviewed=False))


if __name__ == '__main__':
    main(sys.argv[1:])
