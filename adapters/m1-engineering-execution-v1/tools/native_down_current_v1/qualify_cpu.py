"""Qualify the small worker refresh adapter under the existing bounded CPU profile."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess

D = Path('/tmp/ferric-v16-emitter-b95a642-r1')
NAMES = ('current_binding.py', 'run_stage.py', 'test_current_binding.py', 'qualify_cpu.py')


def sha(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    inputs = Path(__file__).resolve().parent
    output = args.output
    if not output.is_absolute() or not output.is_relative_to(D) or output.exists():
        raise ValueError('fresh CPU output below the private bounded stage')
    profile = D / 'owner/cpu_profile_48g.py'
    if sha(profile) != '78feddc3bdaf1728c420d734cf2c04c8f714601f07259545344132a3dbae9d80':
        raise ValueError('bounded CPU profile pin')
    spec = importlib.util.spec_from_file_location('profile', profile)
    q = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(q)
    env = q.environment()
    before = q.allocation(1024**2)
    pins = {name: sha(inputs / name) for name in NAMES}
    output.mkdir(mode=0o700)
    for name in NAMES:
        compile((inputs / name).read_bytes(), str(inputs / name), 'exec')
    command = ['/usr/bin/python3', '-I', '-B', str(inputs / 'test_current_binding.py')]
    with (output / 'stdout').open('xb') as out, (output / 'stderr').open('xb') as err:
        result = subprocess.run(command, cwd=inputs, env=env, stdin=subprocess.DEVNULL,
                                stdout=out, stderr=err, timeout=120)
    after = {name: sha(inputs / name) for name in NAMES}
    passed = result.returncode == 0 and re.search(
        rb'\nRan 24 tests in [0-9.]+s\n\nOK\n$', (output / 'stderr').read_bytes())
    record = {'schema': 'FerricNativeDownCurrentCpuV1', 'accepted': bool(passed and pins == after),
              'argv': command, 'returncode': result.returncode, 'expected_tests': 24,
              'source_before': pins, 'source_after': after, 'stage_before': before,
              'stage_after': q.allocation(), 'stdout_sha256': sha(output / 'stdout'),
              'stderr_sha256': sha(output / 'stderr'), 'native_executed': False}
    with (output / 'receipt.json').open('x') as stream:
        json.dump(record, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(record, sort_keys=True), flush=True)
    raise SystemExit(0 if record['accepted'] else 1)


if __name__ == '__main__':
    main()
