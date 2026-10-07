"""Qualify capture adapters on the CPU build host without GPU access."""
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess

D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
I = D / 'inputs/native-wait-capture-a001'
O = D / 'native-wait-capture-a001'
NAMES = ('run_wait.py', 'capture_binding.py', 'wait_observation.py',
         'test_capture_binding.py', 'test_wait_observation.py', 'qualify_capture.py')


def sha(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def main():
    profile = D / 'owner/cpu_profile_45g.py'
    if sha(profile) != 'c314a83ab409f65201c0a3ae15f110ca9ca6996d5f6866c1079b95df20fb5f27':
        raise ValueError('bounded CPU profile pin')
    spec = importlib.util.spec_from_file_location('profile', profile)
    q = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(q)
    env = q.environment()
    before = q.allocation(16 * 1024**2)
    pins = {name: sha(I / name) for name in NAMES}
    O.mkdir(mode=0o700)
    command = ['/usr/bin/python3', '-I', '-B', str(I / 'test_capture_binding.py')]
    with (O / 'stdout').open('xb') as out, (O / 'stderr').open('xb') as err:
        result = subprocess.run(command, cwd=I, env=env, stdin=subprocess.DEVNULL,
                                stdout=out, stderr=err, timeout=120)
    after = {name: sha(I / name) for name in NAMES}
    passed = result.returncode == 0 and re.search(rb'\nRan 28 tests in [0-9.]+s\n\nOK\n$', (O / 'stderr').read_bytes())
    record = {'schema': 'FerricNativeWaitCaptureCpuV1', 'accepted': bool(passed and pins == after),
              'argv': command, 'returncode': result.returncode, 'expected_tests': 28,
              'source_before': pins, 'source_after': after, 'stage_before': before,
              'stage_after': q.allocation(), 'stdout_sha256': sha(O / 'stdout'),
              'stderr_sha256': sha(O / 'stderr'), 'native_executed': False}
    with (O / 'receipt.json').open('x') as output:
        json.dump(record, output, indent=2, sort_keys=True)
        output.write('\n')
    print(json.dumps(record, sort_keys=True), flush=True)
    raise SystemExit(0 if record['accepted'] else 1)


if __name__ == '__main__':
    main()
