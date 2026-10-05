"""Select actual shared-host CPU products; the frozen ELF auditor is unchanged."""
import contextlib
import hashlib
import os
from pathlib import Path
import re
import resource
import shutil
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PREVIOUS = E / 'p228-projection-ar4-host-observation-deployment-v1/audit_projection_ar4_host_observation_runtime.py'
PREVIOUS_SHA = '32e45386b1513457101b670f702df48b3af554d617692468b59a7d05c687c26a'
HELPER = E / 'p227-prefix-runtime-audit-v2/run_row_facts_v2.py'
HELPER_SHA = '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820'
PACKAGE = 'p228-projection-ar4-shared-host-gpu-v1'
PACKAGE_SCHEMA = 'ferric-p228-projection-ar4-shared-host-gpu-package-v1'
BASELINE = dict(path=str(E / 'projection-ar4-host-observation-cpu-v228-v1/complete.json'),
    bytes=488734, sha256='7d8c08eeffab9cbebbf6abc973c0ba93d80616ad81063b35587e74d04df1d59c')
VALIDATOR_SHA = '757c077979b7ee02d4c0e7d222e75292cd9d417927f84ae9a8134344f8e217b9'
BASE_CPU_SHA = 'ac69404cec3d51d733daeb81dda3f51947c4c0a8065c7644946b8e4238ea4a72'
ROLES = {
    'default-parent': ('default', 'parent', 'ferric-qwen3-finite-projection-residual-decode-host-engineering'),
    'shared-parent': ('shared', 'parent', 'ferric-qwen3-finite-projection-residual-decode-shared-host-engineering'),
    'worker': ('worker', 'worker', 'ferric-tp-peer-finite-engineering-worker-v1'),
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def bootstrap():
    # Authenticate the same small data reader before loading the frozen selector.
    import stat
    require(HELPER.resolve(strict=True) == HELPER and not HELPER.is_symlink(), 'canonical data helper')
    with HELPER.open('rb') as stream:
        before = os.fstat(stream.fileno()); raw = stream.read((1 << 20) + 1); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and 0 < len(raw) == before.st_size <= 1 << 20
        and stamp(before) == stamp(after) == stamp(HELPER.lstat())
        and hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'exact stable frozen data helper')
    D = types.ModuleType('shared_runtime_custody'); D.__file__ = str(HELPER)
    exec(compile(raw, str(HELPER), 'exec'), D.__dict__)
    pins = D.Pins(); pins.pin(HELPER, HELPER_SHA)
    previous = D.load_module(pins, PREVIOUS, PREVIOUS_SHA, 'shared_runtime_previous_selector')
    return D, pins, previous


@contextlib.contextmanager
def contracts(D, pins, manifest_sha):
    require(type(manifest_sha) is str and re.fullmatch('[0-9a-f]{64}', manifest_sha),
            'explicit root-reviewed frozen package SHA')
    base = D.package(pins, PACKAGE, manifest_sha)
    manifest, _ = pins.json(base / 'manifest.json', manifest_sha)
    require(manifest['schema'] == PACKAGE_SCHEMA, 'shared-host package, not predecessor')
    rows = {r['path']: r for r in manifest['files']}
    require(len(rows) == len(manifest['files'])
        and all(re.fullmatch(r'[A-Za-z0-9_.-]+', n) for n in rows), 'flat unique package roster')
    require(rows['layer_validation.py']['sha256'] == VALIDATOR_SHA
        and rows['observer_cpu.py']['sha256'] == BASE_CPU_SHA, 'unchanged baseline admission helpers')
    missing = object(); saved = {}
    try:
        for name in ('layer_validation', 'observer_cpu', 'shared_cpu'):
            saved[name] = sys.modules.get(name, missing)
            sys.modules[name] = D.load_module(pins, base / (name + '.py'),
                rows[name + '.py']['sha256'], 'shared_runtime_' + name)
        yield sys.modules['observer_cpu'], sys.modules['shared_cpu']
    finally:
        for name, previous in reversed(tuple(saved.items())):
            if previous is missing:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = previous


def cpu_artifact(D, pins, previous, cpu_pin, role, manifest_sha):
    require(role in ROLES, 'closed selected role')
    root = Path(cpu_pin['path']).parent
    require(root.parent == E and Path(cpu_pin['path']).name == 'complete.json'
        and re.fullmatch(r'projection-ar4-shared-host-cpu-v228-v[1-9][0-9]{0,8}', root.name),
        'actual new CPU namespace')
    read = lambda p, record, maximum=16 << 20: previous.read_pin(p, record, maximum)
    doc = lambda p, record, maximum=16 << 20: D.parse(read(p, record, maximum))
    value = doc(pins, cpu_pin, 4 << 20)
    with contracts(D, pins, manifest_sha) as (B, S):
        selected = S.contract(value, root)
        require(value['prior_completion'] == BASELINE, 'exact actual855 predecessor')
        prior = doc(pins, BASELINE, 4 << 20)
        B.contract(prior, Path(BASELINE['path']).parent)
        I = types.SimpleNamespace(E=E, D=D, hashlib=hashlib, read=read, doc=doc)
        proposal = S.sources(I, pins, value, root)
        require(proposal['base_sources_after'] == prior['raw']['sources-after.json'],
                'independently authenticated855 source-map join')
        S.test_delta(value, prior, proposal)
        key, _, name = ROLES[role]
        require(selected[key]['artifact']['target']['name'] == name, 'actual selected Cargo role')
        return selected[key]['binary']


def selected(D, args, pins, previous):
    require(len(args) == 7, 'PACKAGE_SHA LABEL ROLE CPU_PATH CPU_SHA BINARY_PATH BINARY_SHA')
    package_sha, label, role, cpu_path, cpu_sha, binary_path, binary_sha = args
    require(role in ROLES and re.fullmatch(
        'projection-ar4-shared-host-runtime-' + role + r'-v228-v[1-9][0-9]{0,8}', label),
        'closed role-specific fresh audit label')
    require(all(type(s) is str and re.fullmatch('[0-9a-f]{64}', s)
        for s in (package_sha, cpu_sha, binary_sha)), 'explicit actual input digests')
    cpu, binary = Path(cpu_path), Path(binary_path); root = cpu.parent
    require(str(cpu) == cpu_path and root.parent == E and cpu.name == 'complete.json'
        and re.fullmatch(r'projection-ar4-shared-host-cpu-v228-v[1-9][0-9]{0,8}', root.name),
        'exact new CPU receipt namespace')
    _, directory, name = ROLES[role]
    require(str(binary) == binary_path and binary == root / 'target' / directory / 'debug' / name,
            'original role-specific product path, no old or copied ELF')
    cpu_pin, _ = pins.read(cpu, cpu_sha, retain=False, maximum=4 << 20)
    original = cpu_artifact(D, pins, previous, cpu_pin, role, package_sha)
    binary_pin, raw = pins.read(binary, binary_sha, retain=True, maximum=128 << 20)
    require(binary_pin == original and os.access(binary, os.X_OK), 'actual selected executable identity and mode')
    require(len(raw) >= 20 and raw[:6] == b'\x7fELF\x02\x01' and raw[18:20] == b'\x3e\x00',
            'genuine ELF64 little-endian x86_64 header')
    return E / label, binary_pin


def main(args):
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
        and 'PYTHONPATH' not in os.environ and 'PYTHONHOME' not in os.environ
        and os.getuid() == os.geteuid() == 9661 and os.sched_getaffinity(0) == {8, 9}
        and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'unchanged bounded audit identity')
    require(shutil.disk_usage(E.parents[1]).free >= 40 << 30, 'unchanged initial disk floor')
    os.umask(0o077)
    for kind, limit in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                       (resource.RLIMIT_FSIZE, 64 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (limit, limit))
    D, pins, previous = bootstrap(); pins.pin(Path(__file__).resolve())
    out, binary = selected(D, args, pins, previous)
    require(not os.path.lexists(out), 'fresh audit output')
    base = D.package(pins, *previous.AUDITOR)
    audit = D.load_module(pins, base / 'audit.py', previous.AUDIT_SHA, 'shared_runtime_auditor')
    audit.host_identity(); audit.read_pin(pins, binary, 1 << 30)
    topology = D.load_module(pins, audit.TOPOLOGY, audit.TOPOLOGY_SHA, 'shared_runtime_topology')
    owned = D.load_module(pins, base / 'frozen_owned.py', audit.OWNED_SHA, 'shared_runtime_owned')
    record = audit.execute(out, binary, pins, owned, topology)
    D.progress(dict(complete=record, gpu_execution=False, reviewed=False))


if __name__ == '__main__':
    main(sys.argv[1:])
