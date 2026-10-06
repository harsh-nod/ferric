"""Format only the pinned draft sources; this is not compiler qualification."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import signal
import stat
import sys
import time
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-atomic-load-alias-format-v228-v1'
SUPERVISOR = E / 'guarded-mlp-combined-state-owner-cpu-v228-v1/supervisor.py'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
TOOLCHAIN = Path('/home/harmenon/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu')
PINS = {
    'formal_memory_obligations.rs': (60323, '75d7efd321153299194d698661eacb4abe5ad54ac11707e6be31337b7ae59ff7'),
    'formal_atomic_load_alias_v1.rs': (16475, 'ae50f13590a1256234509f206dd7a88fbd248ffb3f347f3d4e2bfe3c18337e25'),
}


def pin(path):
    before = path.lstat()
    assert path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
    assert before.st_size < 16 << 20
    body = path.read_bytes()
    after = path.lstat()
    assert (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) == (
        after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)
    return {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()}


assert __debug__ and sys.dont_write_bytecode and len(sys.argv) == 1
assert os.getuid() == os.geteuid() == 9661
assert os.uname().nodename == 'smci350-rck-g03-b19-03'
assert ROOT.resolve(strict=True) == ROOT
assert Path(__file__).resolve(strict=True) == ROOT / 'format_remote.py'
assert {p.name for p in ROOT.iterdir()} == set(PINS) | {'format_remote.py'}
assert pin(SUPERVISOR)['sha256'] == SUPERVISOR_SHA
assert pin(TOOLCHAIN / 'bin/rustfmt') == {
    'bytes': 5281712, 'sha256': 'a9137d0c198ceb6c72193d517d3c9007b3ec7a90d3d10ec6889773eca48261b4'}
for parent in (ROOT, *ROOT.parents):
    for name in ('rustfmt.toml', '.rustfmt.toml'):
        assert not os.path.lexists(parent / name)
before = {name: pin(ROOT / name) for name in PINS}
assert before == {name: dict(bytes=size, sha256=sha) for name, (size, sha) in PINS.items()}
controller = pin(ROOT / 'format_remote.py')
body = SUPERVISOR.read_bytes()
assert hashlib.sha256(body).hexdigest() == SUPERVISOR_SHA
h = types.ModuleType('pinned_formatter_supervisor')
h.__file__ = str(SUPERVISOR)
exec(compile(body, str(SUPERVISOR), 'exec'), h.__dict__)
h.ROOT = ROOT
h.OUT = ROOT / 'evidence'
h.TARGET = ROOT / 'target'
h.TMP = ROOT / 'tmp'
h.AS_LIMIT = 2 << 30
h.FILE_LIMIT = 16 << 20
h.STREAM_LIMIT = 4 << 20
h.CACHE_LIMIT = 64 << 20
h.CPU_LIMIT = 120
h.CLEANUP_RESERVE = 50
for directory in (h.OUT, h.TARGET, h.TMP):
    directory.mkdir()
assert {8, 9} <= os.sched_getaffinity(0)
os.sched_setaffinity(0, {8, 9})
os.nice(10)
assert ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0
env = {
    'PATH': str(TOOLCHAIN / 'bin') + ':/usr/bin:/bin',
    'HOME': '/home/harmenon', 'LC_ALL': 'C', 'LANG': 'C',
    'TMPDIR': str(h.TMP), 'RUST_BACKTRACE': '0',
    'HIP_VISIBLE_DEVICES': '', 'ROCR_VISIBLE_DEVICES': '', 'CUDA_VISIBLE_DEVICES': '',
}
phases = []
failure = None
started = time.monotonic()
deadline = started + 300
try:
    for name in PINS:
        h.run('format-' + name, [str(TOOLCHAIN / 'bin/rustfmt'), '--edition', '2024',
              '--config', 'skip_children=true', str(ROOT / name)], env, phases,
              deadline, None, seconds=60, cwd=ROOT)
    h.run('format-check', [str(TOOLCHAIN / 'bin/rustfmt'), '--check', '--edition', '2024',
          '--config', 'skip_children=true', *[str(ROOT / name) for name in PINS]],
          env, phases, deadline, None, seconds=60, cwd=ROOT)
except BaseException as error:
    failure = repr(error)
finally:
    signal.setitimer(signal.ITIMER_REAL, 0)
after = {name: pin(ROOT / name) for name in PINS}
assert pin(ROOT / 'format_remote.py') == controller
assert pin(SUPERVISOR)['sha256'] == SUPERVISOR_SHA
result = dict(schema='ferric-source-format-only-v1', passed=failure is None,
              failure=failure, before=before, after=after, controller=controller,
              phases=phases, seconds=time.monotonic() - started,
              compiler_qualification=False, gpu_execution=False)
path = h.OUT / ('complete.json' if failure is None else 'failed.json')
with path.open('x') as stream:
    stream.write(json.dumps(result, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(receipt=pin(path), after=after, passed=failure is None, failure=failure)))
sys.exit(0 if failure is None else 1)
