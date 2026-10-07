#!/usr/bin/env python3
"""Create a source-bound qualification from an exact retained G30 or G32 phase."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    sys.modules[name] = value
    spec.loader.exec_module(value)
    return value


c = module('http_cpu_binding_contract', ROOT / 'matched128_contract.py')
cpu = module('http_cpu_binding_rules', ROOT / 'http_cpu.py')


def binding(path):
    c.require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical retained CPU input')
    return {'path': str(path), 'sha256': c.digest(path)}


def emit(phase_root, guard, environment, output):
    result = c.read(phase_root / 'result.json')
    argv = result['argv']
    expected = ['/bin/bash', str(environment), '/usr/bin/python3', '-I', '-B', '-m',
                'unittest', 'discover', '-s', str(ROOT), '-p', 'test_*.py', '-v']
    c.require(argv == expected, 'only the actual full-suite command on these source paths can qualify')
    encoded = json.dumps(argv, sort_keys=True, separators=(',', ':'), allow_nan=False).encode() + b'\n'
    value = {'schema': 'FerricNativeHttpCpuQualificationV1',
        'driver_sources': {name: c.digest(ROOT / name) for name in c.DRIVER_FILES},
        'test_sources': {name: c.digest(ROOT / name) for name in cpu.TEST_FILES},
        'guard': binding(guard), 'environment': binding(environment),
        'phase': {'status': binding(phase_root / 'exit.status'), 'result': binding(phase_root / 'result.json'),
                  'stderr': binding(phase_root / 'stderr'), 'argv_sha256': hashlib.sha256(encoded).hexdigest()},
        'source_root': str(ROOT), 'servers_launched': False}
    def raw(item, limit):
        c.require(c.digest(item['path'], limit) == item['sha256'], 'raw CPU receipt changed')
        return Path(item['path']).read_bytes()
    cpu.validate(value, value['driver_sources'], bound=c.binding_value, raw=raw, digest=c.digest, root=ROOT)
    c.require(output.is_absolute() and output.parent.resolve(strict=True) == output.parent
              and not output.exists(), 'fresh qualification output')
    c.write(output, value)
    return {'schema': 'FerricNativeHttpCpuBindingV1', 'qualified': True,
            'binding': binding(output), 'servers_launched': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('phase-root', 'guard', 'environment', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(emit(args.phase_root, args.guard, args.environment, args.output), sort_keys=True))


if __name__ == '__main__':
    main()
