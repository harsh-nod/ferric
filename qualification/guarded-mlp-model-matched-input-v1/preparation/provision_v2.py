#!/usr/bin/env python3
"""Provision only a task-private package overlay in the cached ROCm image."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-matched-framework-env-v228-v2')
IMAGE = 'sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba'
NAME = 'ferric-matched-mlp-env-20261006-v2'
PACKAGES = ['transformers==4.51.0', 'tokenizers==0.21.4',
            'huggingface-hub==0.36.0',
            'safetensors==0.5.3', 'psutil==7.0.0']


def save(name, value):
    with (ROOT / name).open('x') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')


def run(argv, timeout=30):
    return subprocess.run(argv, check=True, capture_output=True, text=True,
                          timeout=timeout)


def main():
    assert os.getuid() == 9661
    assert os.uname().nodename == 'smci350-rck-g03-b19-03'
    assert shutil.disk_usage(ROOT.parent).free >= 40 << 30
    assert run(['docker', 'image', 'inspect', '--format', '{{.Id}}', IMAGE]).stdout.strip() == IMAGE
    assert not run(['docker', 'ps', '-aq', '--filter', f'name=^/{NAME}$']).stdout.strip()
    ROOT.mkdir(mode=0o700)
    started = time.monotonic()
    argv = ['docker', 'create', '--pull=never', '--name', NAME,
            '--label', f'ferric.owner={ROOT}', '--read-only', '--user', '9661:9661',
            '--cap-drop=ALL', '--security-opt=no-new-privileges',
            '--cpuset-cpus=8,9', '--cpus=2', '--memory=4g', '--memory-swap=4g',
            '--pids-limit=128', '--tmpfs=/tmp:rw,nosuid,nodev,size=1g',
            '--env=HOME=/tmp', '--env=PIP_DISABLE_PIP_VERSION_CHECK=1',
            '--env=PYTHONDONTWRITEBYTECODE=1', '--log-driver=none',
            '--mount', f'type=bind,src={ROOT},dst=/work',
            '--entrypoint=/usr/bin/timeout', IMAGE,
            '--signal=TERM', '--kill-after=10s', '300s',
            'python3', '-I', '-B', '-m', 'pip', 'install',
            '--no-cache-dir', '--no-deps', '--only-binary=:all:',
            '--index-url=https://pypi.org/simple', '--target=/work/packages',
            '--report=/work/pip-report.json', *PACKAGES]
    save('command.json', argv)
    save('source.json', {'sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                         'path': str(Path(__file__).resolve())})
    cid = None
    failure = None
    try:
        cid = run(argv).stdout.strip()
        assert len(cid) == 64 and all(c in '0123456789abcdef' for c in cid)
        save('container.json', {'id': cid, 'name': NAME, 'image': IMAGE})
        with (ROOT / 'install.stdout').open('xb') as out, (ROOT / 'install.stderr').open('xb') as err:
            result = subprocess.run(['docker', 'start', '-a', cid], stdout=out,
                                    stderr=err, timeout=330)
        assert result.returncode == 0, f'install exit {result.returncode}'
        assert shutil.disk_usage(ROOT).free >= 40 << 30
        files = {}
        for path in sorted((ROOT / 'packages').rglob('*')):
            if path.is_file():
                assert not path.is_symlink()
                files[str(path.relative_to(ROOT))] = {
                    'bytes': path.stat().st_size,
                    'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
        assert files and sum(row['bytes'] for row in files.values()) <= 512 << 20
        save('package-files.json', files)
    except BaseException as exc:
        failure = f'{type(exc).__name__}: {exc}'
    finally:
        if cid:
            state = json.loads(run(['docker', 'inspect', cid]).stdout)[0]
            if state['State']['Running']:
                run(['docker', 'stop', '--time=10', cid], timeout=30)
                state = json.loads(run(['docker', 'inspect', cid]).stdout)[0]
            assert not state['State']['Running']
            save('container-final.json', state)
            run(['docker', 'rm', cid])
            assert not run(['docker', 'ps', '-aq', '--filter', f'id={cid}']).stdout.strip()
        save('complete.json', {'status': 'FAIL' if failure else 'PASS',
                               'error': failure, 'image': IMAGE,
                               'gpu_executed': False,
                               'elapsed_seconds': time.monotonic() - started,
                               'container_removed': cid is not None})
    if failure:
        raise RuntimeError(failure)
    print('PASS: private CPU-only package overlay; no GPU execution')


if __name__ == '__main__':
    main()
