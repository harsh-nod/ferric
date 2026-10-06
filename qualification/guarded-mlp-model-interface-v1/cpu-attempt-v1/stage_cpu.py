"""Stage and format a fresh diagnostic overlay; preserve the qualified baseline."""
import ctypes
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import stat
import sys
import tarfile
import time
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-deferred-binding-cpu-v228-v1'
ROOT = E / 'guarded-mlp-model-interface-cpu-v228-v1'
ARCHIVE = E / 'guarded-mlp-model-interface-input-v228-v1.tar.gz'
ARCHIVE_PIN = dict(bytes=78307, sha256='da43c60541eed56682af439f578fc415cfd5fc5e1294d9e7839b657dad698750')
CONTROLLER_PIN = dict(bytes=37674, sha256='3595563795816b291f494dd90d063bc62acf6e4f8752793f2395a5fab4b25f8d')
NAMES = tuple('crates/fe2o3-kfd/src/' + n for n in (
    'lib.rs', 'engineering_gfx950.rs', 'engineering_gfx950_peer.rs',
    'engineering_gfx950_peer_combined_mlp_paired_v1.rs',
    'engineering_gfx950_peer_combined_mlp_paired_retained_v1.rs',
    'engineering_gfx950_peer_combined_mlp_paired_interleaved_native_v1_tests.rs',
    'engineering_gfx950_peer_combined_mlp_paired_mixed_bank_v1.rs',
    'engineering_gfx950_peer_combined_mlp_paired_mixed_bank_v1_tests.rs',
    'engineering_gfx950_peer_combined_mlp_paired_facade_v1.rs',
    'engineering_gfx950_peer_combined_mlp_paired_facade_v1_tests.rs',
)) + ('crates/fe2o3-kfd/tests/guarded_mlp_facade_v1.rs',)
PINS = {
    'base_complete': ('evidence/complete.json', 1249931, '76c7d14beaecec2f63fff67828ff40cbe5ac439c77f27b0e621da7659ab42ca1'),
    'base_sources': ('evidence/sources-after.json', 320662, '4b7ef5c6bc235dcd307927aaff30608758005cb323dfa8a6e1a2aa48f8efe013'),
    'base_stdout': ('evidence/kfd-tests.stdout', 122389, 'ae006359e3b77edc5ab769329bdc1f67f87314681c7aa8be1a199846273616b8'),
    'base_controller': ('run_cpu.py', 33240, '5fb5766ccbd6c0001b6bdc9ec957dee5adfe3d13b2db46846c10b01fad1313e2'),
}
DEADLINE = time.monotonic() + 180
CREATED = False


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def read(path):
    before = path.lstat()
    assert path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
    assert 0 <= before.st_size < 16 << 20
    body = path.read_bytes()
    after = path.lstat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    assert len(body) == before.st_size and stamp(before) == stamp(after)
    return body


def main():
    global CREATED
    assert __debug__ and sys.dont_write_bytecode and len(sys.argv) == 1
    assert os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
    assert not os.path.lexists(ROOT) and ROOT.parent.resolve(strict=True) == ROOT.parent
    assert shutil.disk_usage(E).free >= 40 << 30
    data = read(ARCHIVE)
    assert pin(data) == ARCHIVE_PIN
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as tar:
        members = tar.getmembers()
        expected_names = {'run_cpu.py', 'supervisor.py'} | {'source/' + n for n in NAMES}
        assert len(members) == 13 and {m.name for m in members} == expected_names
        assert all(m.isfile() and 0 < m.size < 1 << 20 for m in members)
        bodies = {m.name: tar.extractfile(m).read() for m in members}
    assert pin(bodies['run_cpu.py']) == CONTROLLER_PIN
    assert pin(bodies['supervisor.py']) == dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
    lineage, baseline_bodies = {}, {}
    for key, (name, size, sha) in PINS.items():
        body = read(BASE / name)
        assert pin(body) == dict(bytes=size, sha256=sha)
        lineage[key] = dict(path=str(BASE / name), **pin(body))
        baseline_bodies[BASE / name] = body
    previous = json.loads(baseline_bodies[BASE / 'evidence/complete.json'])
    old_map = json.loads(baseline_bodies[BASE / 'evidence/sources-after.json'])
    assert previous['passed'] and previous['postcheck_errors'] == []
    assert previous['input_sources'] == previous['final_sources'] == old_map and len(old_map) == 804
    source_map = {n: row for n, row in old_map.items() if n.startswith('fe2o3/')}
    assert len(source_map) == 802
    os.umask(0o077)
    ROOT.mkdir(mode=0o700)
    CREATED = True
    for name, row in source_map.items():
        path = BASE / name
        assert row['path'] == str(path) and '..' not in Path(name).parts
        body = read(path)
        assert pin(body) == {k: row[k] for k in ('bytes', 'sha256')}
        destination = ROOT / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open('xb') as stream:
            stream.write(body)
    for name in NAMES:
        destination = ROOT / 'fe2o3' / name
        old = source_map.get('fe2o3/' + name)
        assert destination.exists() == (old is not None)
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(bodies['source/' + name])
    for name in ('run_cpu.py', 'supervisor.py'):
        with (ROOT / name).open('xb') as stream:
            stream.write(bodies[name])
    h = types.ModuleType('stable_r2_format_supervisor')
    h.__file__ = str(ROOT / 'supervisor.py')
    exec(compile(bodies['supervisor.py'], h.__file__, 'exec'), h.__dict__)
    h.ROOT, h.SOURCE = ROOT, ROOT / 'fe2o3'
    h.OUT, h.TMP, h.TARGET = ROOT / 'format', ROOT / 'format-tmp', ROOT / 'format-target'
    h.OUT.mkdir(mode=0o700)
    h.TMP.mkdir(mode=0o700)
    os.sched_setaffinity(0, {8, 9})
    assert os.getpriority(os.PRIO_PROCESS, 0) in (0, 10)
    if os.getpriority(os.PRIO_PROCESS, 0) == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, h.AS_LIMIT), (resource.RLIMIT_FSIZE, h.FILE_LIMIT)):
        soft, hard = resource.getrlimit(kind)
        cap = min([cap] + [n for n in (soft, hard) if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (cap, cap))
    assert ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    phases, failure = [], None
    rustfmt = Path(previous['tool_pins']['rustfmt']['path'])
    assert h.pin(rustfmt) == previous['tool_pins']['rustfmt']
    assert h.pin(Path('/usr/bin/prlimit')) == previous['tool_pins']['prlimit']
    try:
        h.run('rustfmt', [str(rustfmt), '--edition', '2024', '--config', 'skip_children=true',
            *(str(ROOT / 'fe2o3' / name) for name in NAMES)],
            dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C', TMPDIR=str(h.TMP)),
            phases, DEADLINE, None, 60, ROOT)
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, max(0.001, DEADLINE - time.monotonic()))
    h.save('format-outcome.json', dict(phases=phases, failure=failure,
        passed=failure is None, scope='formatter-only, not stage acceptance',
        gpu_execution=False, compiler_executed=False, archive=ARCHIVE_PIN))
    if failure is not None:
        raise RuntimeError(failure)
    assert h.pin(rustfmt) == previous['tool_pins']['rustfmt']
    assert h.pin(Path('/usr/bin/prlimit')) == previous['tool_pins']['prlimit']
    assert all(read(path) == body for path, body in baseline_bodies.items())
    for name, row in source_map.items():
        assert pin(read(BASE / name)) == {k: row[k] for k in ('bytes', 'sha256')}
        if name.removeprefix('fe2o3/') not in NAMES:
            assert pin(read(ROOT / name)) == {k: row[k] for k in ('bytes', 'sha256')}
    lineage['overlay'] = {}
    for name in NAMES:
        old = source_map.get('fe2o3/' + name)
        lineage['overlay'][name] = dict(before=None if old is None else {k: old[k] for k in ('bytes', 'sha256')},
            after=pin(read(ROOT / 'fe2o3' / name)))
    files = {str(p.relative_to(ROOT)): pin(read(p)) for p in h.files_below(ROOT / 'fe2o3')}
    files.update({n: pin(read(ROOT / n)) for n in ('run_cpu.py', 'supervisor.py')})
    assert len(files) == 809
    value = dict(schema='ferric-guarded-mlp-interleaved-native-cpu-input-v1',
        source_generation=previous['source_generation'], files=files,
        tool_pins=previous['tool_pins'], source_lineage=lineage)
    body = (json.dumps(value, sort_keys=True, indent=2) + '\n').encode()
    with (ROOT / 'input-manifest.json').open('xb') as stream:
        stream.write(body)
    assert read(ROOT / 'input-manifest.json') == body and time.monotonic() < DEADLINE
    h.save('complete.json', dict(passed=True, phases=phases, failure=None,
        gpu_execution=False, compiler_executed=False, archive=ARCHIVE_PIN,
        input_manifest=pin(body), baseline_unchanged=True, postchecks_passed=True))
    print(json.dumps(dict(root=str(ROOT), input=pin(body), overlay=lineage['overlay'], format_passed=True)))


if __name__ == '__main__':
    def interrupted(signum, _frame):
        raise RuntimeError('stage interrupted: ' + str(signum))
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, DEADLINE - time.monotonic()))
    try:
        main()
    except BaseException as error:
        signal.setitimer(signal.ITIMER_REAL, 0)
        if CREATED:
            directory = ROOT / 'format'
            directory.mkdir(mode=0o700, exist_ok=True)
            with (directory / 'failed.json').open('x') as stream:
                json.dump(dict(passed=False, failure=repr(error), gpu_execution=False,
                    compiler_executed=False, archive=ARCHIVE_PIN), stream, sort_keys=True)
                stream.write('\n')
        raise
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
