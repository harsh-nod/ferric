#!/usr/bin/env python3
"""Read-only exact experimental artifact/CPU binding; never launches a GPU."""
import argparse
import hashlib
import importlib.util
import os
from pathlib import Path

D = Path(__file__).resolve().parent
definition = importlib.util.spec_from_file_location('splitk_binding_contract', D / 'harness/launch_contract.py')
c = importlib.util.module_from_spec(definition)
definition.loader.exec_module(c)


def qualify(config, harness):
    c.require(type(config) is dict and set(config) == {'schema', 'runtime_main', 'runtime_source',
        'controller_source', 'worker', 'controller', 'experimental_retention', 'splitk', 'runtime_evidence',
        'phases', 'harness_sources'} and config['schema'] == 'FerricSplitKExperimentalBindingInputsV1',
        'closed split-K experimental binding inputs')
    for name in ('runtime_source', 'controller_source', 'worker', 'controller', 'experimental_retention'):
        c.binding(config[name])
        raw, _ = c.read(config[name]['path'], config[name]['sha256'])
        if name == 'controller':
            c.require(len(raw) == 10455688 and raw.startswith(b'\x7fELF'), 'exact original release ELF')
    c.validate_splitk(config['splitk'])
    image, roster = config['splitk']['image'], config['splitk']['roster']
    for filename, field in (('observation.json', 'manifest'), ('observation.hsaco', 'hsaco')):
        c.read(Path(image['path']) / filename, image[field], 64 * 1024**2)
    c.require(c.bound({key: roster[key] for key in ('path', 'sha256')}) == roster['value'],
              'compiler roster exact bytes/value')
    c.require(harness.is_absolute() and harness.resolve(strict=True) == harness, 'canonical harness root')
    expected = set(c.OWN_SOURCES) | {'measurement/' + name for name in c.MEASUREMENT_SOURCES}
    c.require(type(config['harness_sources']) is dict and set(config['harness_sources']) == expected,
              'complete tested production and fixture source roster')
    for name, digest in config['harness_sources'].items():
        c.read(harness / name, digest, 1024**2)
    cpu = {'schema': 'FerricSplitKExperimentalCpuV1', **{key: config[key] for key in
        ('runtime_evidence', 'experimental_retention', 'phases', 'harness_sources')}}
    build = {key: config[key] for key in ('runtime_main', 'runtime_source', 'controller_source', 'worker',
                                        'controller', 'experimental_retention', 'splitk')}
    build.update(schema='FerricSplitKExperimentalBuildV1',
                 cpu_qualification={'path': '/not-created/cpu.json', 'sha256': '0' * 64})
    c.validate_build(build, admission=False)
    scope = c.validate_cpu(cpu, build)
    return build, cpu, scope


def emit(config, harness, output):
    build, cpu, scope = qualify(config, harness)
    c.require(output.is_absolute() and output.parent.resolve(strict=True) == output.parent,
              'canonical fresh binding output')
    os.mkdir(output, 0o700)
    raw = c.encoded(cpu)
    with (output / 'cpu.json').open('xb') as stream:
        stream.write(raw)
    build['cpu_qualification'] = {'path': str(output / 'cpu.json'), 'sha256': hashlib.sha256(raw).hexdigest()}
    build_raw = c.encoded(build)
    with (output / 'build.json').open('xb') as stream:
        stream.write(build_raw)
    with (output / 'inputs.json').open('xb') as stream:
        stream.write(c.encoded(config))
    return {'schema': 'FerricSplitKExperimentalBindingResultV1', 'accepted': True, 'native_executed': False,
        'scope': scope, 'build': {'path': str(output / 'build.json'), 'sha256': hashlib.sha256(build_raw).hexdigest()},
        'cpu': build['cpu_qualification']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--config-sha256', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    config = c.decode(c.read(args.config, args.config_sha256, 1024**2)[0])
    print(c.encoded(emit(config, D / 'harness', args.output)).decode(), end='')


if __name__ == '__main__':
    main()
