"""Inspect the retained gfx950 image on MI350; never load or launch it."""

import hashlib
import io
import json
import os
from pathlib import Path
import signal
import stat
import subprocess
import sys
import tarfile
import time


BASE = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
LOWERING = BASE / 'guarded-mlp-atomic-load-alias-lowering-v228-v1'
OUT = BASE / 'guarded-mlp-atomic-load-alias-image-inspection-v228-v1'
ARCHIVE = BASE / 'guarded-mlp-atomic-load-alias-image-inspection-v228-v1.tar.gz'
RECEIPT_SHA = '1053d69036d94db89349b17584819a14f27a824691678730bb4204d283ea93da'
IMAGE_SHA = 'de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66'
IMAGE = LOWERING / 'fe2o3-engineering-v1' / 'd98a12d35a2f0ecea506a9384213c22dbf692c03347efb91a009fc24e0982e1b' / 'observation.hsaco'
READOBJ = Path('/opt/rocm-7.2.1/lib/llvm/bin/llvm-readobj')
OBJDUMP = Path('/opt/rocm-7.2.1/lib/llvm/bin/llvm-objdump')
PRLIMIT = Path('/usr/bin/prlimit')


def pin(body):
    return {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()}


def read(path, maximum=64 << 20):
    before = path.lstat()
    assert stat.S_ISREG(before.st_mode) and path.resolve(strict=True) == path
    assert 0 <= before.st_size <= maximum
    body = path.read_bytes()
    after = path.lstat()
    fields = ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')
    assert tuple(getattr(before, k) for k in fields) == tuple(getattr(after, k) for k in fields)
    assert len(body) == before.st_size
    return body


def write(name, body):
    with (OUT / name).open('xb') as stream:
        stream.write(body)
        stream.flush()
        os.fsync(stream.fileno())


def write_json(name, value):
    write(name, (json.dumps(value, sort_keys=True, indent=2) + '\n').encode())


def group_absent(pgid):
    try:
        os.killpg(pgid, 0)
    except ProcessLookupError:
        return True
    return False


def interrupted(number, _frame):
    raise RuntimeError('inspection interrupted by signal ' + str(number))


def inspect(label, tool, args):
    command = [str(PRLIMIT), '--as=2147483648', '--cpu=30', '--fsize=8388608',
               '--core=0', '--', str(tool), *args, str(IMAGE)]
    environment = {'PATH': '/usr/bin:/bin', 'LANG': 'C', 'LC_ALL': 'C', 'HOME': str(OUT),
                   'ROCR_VISIBLE_DEVICES': '', 'HIP_VISIBLE_DEVICES': '', 'CUDA_VISIBLE_DEVICES': ''}
    write_json(label + '.command.json', {'argv': command, 'cwd': str(OUT), 'env': environment})
    started = time.monotonic_ns()
    process = None
    timed_out = False
    error = None
    cleanup = False
    exit_code = None
    try:
        with (OUT / (label + '.stdout')).open('xb') as stdout, (OUT / (label + '.stderr')).open('xb') as stderr:
            process = subprocess.Popen(command, cwd=OUT, env=environment, stdin=subprocess.DEVNULL,
                                       stdout=stdout, stderr=stderr, start_new_session=True)
            write_json(label + '.started.json', {'pid': process.pid, 'pgid': process.pid})
            try:
                exit_code = process.wait(timeout=40)
            except subprocess.TimeoutExpired:
                timed_out = True
    except BaseException as exc:
        error = type(exc).__name__ + ': ' + str(exc)
    finally:
        # Do not let a repeated catchable signal interrupt child retirement.
        old_handlers = {number: signal.signal(number, signal.SIG_IGN)
                        for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)}
        try:
            if process is not None:
                if process.returncode is None or not group_absent(process.pid):
                    cleanup = True
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                exit_code = process.wait(timeout=10)
        finally:
            for number, handler in old_handlers.items():
                signal.signal(number, handler)
    absent = process is None or group_absent(process.pid)
    result = {'label': label, 'pid': None if process is None else process.pid,
              'pgid': None if process is None else process.pid, 'exit_code': exit_code,
              'natural_exit': exit_code == 0 and not timed_out and not cleanup and error is None,
              'reaped': process is not None and process.returncode is not None,
              'process_group_absent': absent, 'timed_out': timed_out,
              'forced_cleanup': cleanup, 'exception': error,
              'elapsed_ns': time.monotonic_ns() - started}
    write_json(label + '.result.json', result)
    assert absent, 'inspection process group remains live'
    return result


def main():
    assert __debug__ and sys.dont_write_bytecode and len(sys.argv) == 1
    assert os.getuid() == os.geteuid() == 9661
    assert os.uname().nodename == 'smci350-rck-g03-b19-03'
    assert not os.path.lexists(OUT) and not os.path.lexists(ARCHIVE)
    receipt_bytes = read(LOWERING / 'complete.json', 1 << 20)
    assert pin(receipt_bytes)['sha256'] == RECEIPT_SHA
    receipt = json.loads(receipt_bytes)
    assert receipt['passed'] is True and receipt['postcheck_errors'] == []
    assert receipt['gpu_execution'] is False
    assert receipt['artifact']['image']['path'] == str(IMAGE)
    image_bytes = read(IMAGE)
    assert pin(image_bytes) == {'bytes': 28440, 'sha256': IMAGE_SHA}
    assert pin(image_bytes) == {k: receipt['artifact']['image'][k] for k in ('bytes', 'sha256')}
    before = {str(p): pin(read(p)) for p in (READOBJ, OBJDUMP, PRLIMIT, IMAGE, LOWERING / 'complete.json')}
    controller = read(Path(__file__))
    OUT.mkdir(mode=0o700)
    write('controller.py', controller)
    write_json('inputs-before.json', before)
    started = time.monotonic_ns()
    phases = []
    failure = None
    postchecks = []
    for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
        signal.signal(number, interrupted)
    signal.alarm(180)
    try:
        for label, tool, args in (
            ('metadata', READOBJ, ['--file-headers', '--notes', '--symbols']),
            ('disassembly', OBJDUMP, ['--disassemble', '--mcpu=gfx950']),
        ):
            result = inspect(label, tool, args)
            phases.append(result)
            assert result['natural_exit'] and result['reaped'] and result['process_group_absent'], label
    except BaseException as exc:
        failure = type(exc).__name__ + ': ' + str(exc)
    finally:
        signal.alarm(0)
        for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
            signal.signal(number, signal.SIG_IGN)
        try:
            after = {str(p): pin(read(Path(p))) for p in before}
            write_json('inputs-after.json', after)
            if before != after:
                postchecks.append('inspection inputs changed')
            if read(Path(__file__)) != controller:
                postchecks.append('inspection controller changed')
        except Exception as exc:
            postchecks.append(type(exc).__name__ + ': ' + str(exc))
    for phase in phases:
        if phase['pid'] is not None and not group_absent(phase['pid']):
            postchecks.append('live process group: ' + str(phase['pid']))
    raw = {p.name: pin(read(p, 8 << 20)) for p in OUT.iterdir() if p.is_file()}
    result = {'schema': 'ferric-guarded-mlp-atomic-load-alias-image-inspection-v1',
              'passed': failure is None and not postchecks, 'failure': failure,
              'postcheck_errors': postchecks, 'elapsed_ns': time.monotonic_ns() - started,
              'image': pin(image_bytes), 'lowering_receipt': pin(receipt_bytes),
              'phases': phases, 'raw': raw, 'gpu_execution': False,
              'numerical_acceptance': False, 'full_model_acceptance': False,
              'performance_claim': False, 'production_authority': False,
              'load_authority': False, 'launch_authority': False}
    terminal = 'complete.json' if result['passed'] else 'failed.json'
    write_json(terminal, result)
    bodies = {p.name: read(p, 8 << 20) for p in OUT.iterdir() if p.is_file()}
    assert sum(map(len, bodies.values())) < 32 << 20
    manifest = {'schema': 'ferric-guarded-mlp-image-inspection-retention-v1',
                'files': {name: pin(body) for name, body in bodies.items()}}
    write_json('retention-manifest.json', manifest)
    bodies['retention-manifest.json'] = read(OUT / 'retention-manifest.json')
    with ARCHIVE.open('xb') as raw_archive:
        with tarfile.open(fileobj=raw_archive, mode='w:gz') as archive:
            for name, body in sorted(bodies.items()):
                item = tarfile.TarInfo(name)
                item.size = len(body)
                item.mode = 0o600
                archive.addfile(item, io.BytesIO(body))
        raw_archive.flush()
        os.fsync(raw_archive.fileno())
    assert all(read(OUT / name, 8 << 20) == body for name, body in bodies.items())
    print(json.dumps({'terminal': terminal, 'receipt': pin(bodies[terminal]),
                      'passed': result['passed'], 'failure': failure,
                      'archive': pin(read(ARCHIVE)), 'members': len(bodies)}))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
