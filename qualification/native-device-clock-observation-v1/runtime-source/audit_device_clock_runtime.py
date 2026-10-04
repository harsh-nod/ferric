"""Select a qualified transported ELF; preserve the frozen non-native auditor."""
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
# Root fills only the actual frozen intake package digest, never a future result.
GPU_PACKAGE = ('p228-device-clock-gpu-v1', '2f8387de7fca50496c5c1cb8f42df3bc294d0633d54a7086ef82f0abdcbbf94c')


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
    module = types.ModuleType('device_runtime_custody'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module, path


def load_intake(D, pins):
    require(GPU_PACKAGE[0] == 'p228-device-clock-gpu-v1'
        and type(GPU_PACKAGE[1]) is str and re.fullmatch('[0-9a-f]{64}', GPU_PACKAGE[1]),
        'root must bind actual frozen device intake package before auditing')
    base = D.package(pins, *GPU_PACKAGE)
    manifest, _ = pins.json(base / 'manifest.json', GPU_PACKAGE[1])
    hashes = {row['path']: row['sha256'] for row in manifest['files']}
    previous = sys.modules.get('layer_validation')
    try:
        validation = D.load_module(pins, base / 'layer_validation.py', hashes['layer_validation.py'],
                                   'device_runtime_validation')
        sys.modules['layer_validation'] = validation
        intake = D.load_module(pins, base / 'intake.py', hashes['intake.py'], 'device_runtime_intake')
    finally:
        if previous is None: sys.modules.pop('layer_validation', None)
        else: sys.modules['layer_validation'] = previous
    intake.package_record(pins)
    return intake


def selected(I, args, pins):
    require(len(args) == 6, 'LABEL ROLE CPU_PATH CPU_SHA BINARY_PATH BINARY_SHA')
    label, role, cpu_path, cpu_sha, binary_path, binary_sha = args
    require(role in ('parent', 'worker') and re.fullmatch(
        'device-clock-runtime-' + role + r'-v228-v[1-9][0-9]{0,8}', label), 'closed role and fresh audit label')
    require(all(type(value) is str and re.fullmatch('[0-9a-f]{64}', value)
                for value in (cpu_sha, binary_sha)), 'explicit CPU and binary SHA256')
    cpu = Path(cpu_path); binary = Path(binary_path)
    cpu_label = 'gfx950-clock-parent-cpu-v228-v1' if role == 'parent' else 'gfx950-clock-recorder-cpu-v228-v1'
    expected_name = I.PARENT_NAME if role == 'parent' else I.WORKER_NAME
    require(str(cpu) == cpu_path and cpu.parent.parent == I.E and cpu.name == 'complete.json'
        and re.fullmatch(cpu_label, cpu.parent.name), 'actual CPU receipt namespace')
    require(str(binary) == binary_path and binary.parent == I.E / 'device-clock-runtime-v228-v1'
        and binary.name == expected_name, 'exact transported role executable namespace')
    cpu_pin, _ = pins.read(cpu, cpu_sha, retain=False, maximum=4 << 20)
    _value, artifact, _sources = I.cpu_evidence(pins, cpu_pin, role)
    binary_pin, _ = pins.read(binary, binary_sha, retain=False, maximum=128 << 20)
    require(os.access(binary, os.X_OK), 'actual transported executable mode')
    verified = I.deployed_binary(pins, binary_pin, artifact['binary'])
    return I.E / label, verified


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
    I = load_intake(D, pins)
    out, binary = selected(I, args, pins)
    require(not os.path.lexists(out), 'fresh audit output')
    base = D.package(pins, *AUDITOR)
    audit = D.load_module(pins, base / 'audit.py', AUDIT_SHA, 'device_runtime_auditor')
    audit.host_identity()
    audit.read_pin(pins, binary, 1 << 30)
    topology = D.load_module(pins, audit.TOPOLOGY, audit.TOPOLOGY_SHA, 'device_runtime_topology')
    owned = D.load_module(pins, base / 'frozen_owned.py', audit.OWNED_SHA, 'device_runtime_owned')
    record = audit.execute(out, binary, pins, owned, topology)
    D.progress(dict(complete=record, gpu_execution=False, reviewed=False))


if __name__ == '__main__':
    main(sys.argv[1:])
