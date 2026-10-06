"""Static inspection of pinned host ELFs, with the qualified owned supervisor."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import sys
import time
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-host-elf-audit-v228-v2'
OUT = ROOT / 'evidence'
HELPERS = E / 'guarded-mlp-interleaved-native-gpu-v228-v3'
ELFS = {
    1: (11245904, 'dbf5ea87824b4ab2be1e481cf21595f5809fb95bd309552e888a32f8089a8425'),
    2: (11240400, '5dfefcd2e2280b6dc83e63a241c3f518ab3bfa38fa705a4cf070d886ac6d47f0'),
}
TARGETS = ('retained::RetainedPair>::allocate', 'interleaved::inputs',
           'Gfx950EngineeringPeerGroupV1>::validate_token')


def require(value, message):
    if not value:
        raise RuntimeError(message)


def load_exact(name, path, digest):
    require(path.resolve(strict=True) == path and not path.is_symlink()
            and path.is_file() and path.stat().st_size < 128 << 10, 'bounded helper')
    before = path.stat()
    body = path.read_bytes()
    after = path.stat()
    require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns)
            == (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)
            and hashlib.sha256(body).hexdigest() == digest, 'exact helper pin')
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(body, str(path), 'exec'), module.__dict__)
    return module


def main():
    started, deadline = time.monotonic(), time.monotonic() + 300
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'isolated static audit')
    require(not os.path.lexists(ROOT) and ROOT.parent.resolve(strict=True) == ROOT.parent, 'fresh output')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(shutil.disk_usage(E).free >= 40 << 30, '40 GiB setup floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'nice')
    if priority == 0:
        os.nice(10)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper')
    h = load_exact('static_audit_supervisor', HELPERS / 'supervisor.py',
                   '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
    audit = load_exact('static_audit_inputs', HELPERS / 'library_audit.py',
                       'b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d')
    h.ROOT, h.OUT, h.TARGET, h.TMP = ROOT, OUT, ROOT / 'target', ROOT / 'tmp'
    h.AS_LIMIT, h.CPU_LIMIT, h.FILE_LIMIT, h.STREAM_LIMIT = 2 << 30, 20, 4 << 20, 4 << 20
    h.CACHE_LIMIT = 64 << 20
    audit.HARD_DEADLINE = deadline
    ROOT.mkdir(mode=0o700)
    OUT.mkdir(mode=0o700)
    h.TMP.mkdir(mode=0o700)
    for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, deadline - time.monotonic() - 50)
    boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    rows, phases, errors, post_errors = [], [], [], []
    env = dict(HOME='/home/harmenon', PATH='/usr/bin:/bin', LC_ALL='C', LANG='C',
               TMPDIR=str(h.TMP), OMP_NUM_THREADS='1', MALLOC_ARENA_MAX='2')

    def run(label, argv):
        h.run(label, argv, env, phases, deadline, None, 30, ROOT)
        return (OUT / (label + '.stdout')).read_text()

    try:
        audit.read(Path(__file__).resolve())
        audit.read(HELPERS / 'supervisor.py', dict(
            path=str(HELPERS / 'supervisor.py'), bytes=41485,
            sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'))
        audit.read(HELPERS / 'library_audit.py', dict(
            path=str(HELPERS / 'library_audit.py'), bytes=25872,
            sha256='b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'))
        for tool in ('/usr/bin/prlimit', '/usr/bin/nm', '/usr/bin/objdump'):
            audit.resolved_input(tool)
        for version, (size, sha) in ELFS.items():
            elf = E / f'guarded-mlp-interleaved-native-cpu-v228-v{version}/target/debug/deps/fe2o3_kfd-11448a9880d552d9'
            _, before = audit.read(elf, dict(path=str(elf), bytes=size, sha256=sha))
            symbols = run(f'v{version}-nm', ['/usr/bin/nm', '-S', '--defined-only', '--demangle', str(elf)])
            selected = []
            for line in symbols.splitlines():
                fields = line.split(maxsplit=3)
                if len(fields) != 4:
                    continue
                address, extent, kind, name = fields
                if 'fe2o3_kfd::engineering_gfx950::peer::' in name and name.endswith(TARGETS):
                    require(kind in ('t', 'T') and 0 < int(extent, 16) <= 128 << 10,
                            'bounded function extent')
                    selected.append(dict(address=int(address, 16), bytes=int(extent, 16), name=name))
            require(len(selected) <= 8, 'bounded selected symbols')
            matched = [target for target in TARGETS if any(s['name'].endswith(target) for s in selected)]
            missing = [target for target in TARGETS if target not in matched]
            row = dict(version=version, elf=before, symbols=selected, requested=list(TARGETS),
                       matched=matched, missing_or_inlined=missing, full_symbol_coverage=not missing)
            rows.append(row)
            for index, symbol in enumerate(selected):
                run(f'v{version}-symbol-{index}', [
                    '/usr/bin/objdump', '-d', '-C',
                    '--start-address=' + hex(symbol['address']),
                    '--stop-address=' + hex(symbol['address'] + symbol['bytes']), str(elf)])
            audit.read(elf, before)
    except BaseException as error:
        errors.append(repr(error))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - time.monotonic() - 5))
    try:
        for path, canonical in audit.ALIASES.items():
            require(str(Path(path).resolve(strict=True)) == canonical, 'static tool alias drift')
        for value in list(audit.INPUTS.values()):
            audit.read(Path(value['path']), value)
        require(Path('/proc/sys/kernel/random/boot_id').read_text().strip() == boot, 'boot drift')
        require(time.monotonic() < deadline and all(p['reaped'] and p['process_group_absent'] for p in phases),
                'bounded terminal owned tools')
    except BaseException as error:
        post_errors.append(repr(error))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    passed = not errors and not post_errors and len(rows) == 2
    result = dict(schema='ferric-guarded-mlp-static-host-elf-audit-v1', passed=passed,
                  errors=errors, postcheck_errors=post_errors, boot=boot,
                  gpu_execution=False, project_execution=False, executables=rows,
                  inputs=audit.INPUTS, input_aliases=audit.ALIASES, phases=phases,
                  elapsed_seconds=time.monotonic() - started,
                  raw={p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()})
    name = 'complete.json' if passed else 'failed.json'
    h.save(name, result)
    print(json.dumps(dict(passed=passed, errors=errors, postcheck_errors=post_errors,
                         receipt=h.pin(OUT / name), executables=rows), sort_keys=True))
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
