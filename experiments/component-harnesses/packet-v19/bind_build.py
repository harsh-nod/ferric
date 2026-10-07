#!/usr/bin/env python3
"""Validate dedicated packet inputs; native emission stays explicitly disabled."""
import argparse
import hashlib
import importlib.util
import os
from pathlib import Path


D = Path(__file__).resolve().parent
definition = importlib.util.spec_from_file_location('cached_binding_contract', D / 'harness/launch_contract.py')
c = importlib.util.module_from_spec(definition)
definition.loader.exec_module(c)


def qualify(config, harness):
    c.require(type(config) is dict and set(config) == {'schema', 'runtime_main', 'runtime_source',
        'controller_source', 'worker', 'controllers', 'phases', 'harness_sources', 'external_sources',
        'guard_sources', 'controller_evidence', 'runtime_evidence', 'harness_evidence', 'v19_evidence'}
        and config['schema'] == 'FerricV19Packet55cBindingInputsV1', 'closed dedicated packet binding inputs')
    for name in ('runtime_source', 'controller_source', 'worker'):
        c.binding(config[name])
        c.read(config[name]['path'], config[name]['sha256'])
    c.require(type(config['controllers']) is dict and set(config['controllers']) == set(c.CONTROLLERS),
              'one separate diagnostic binary')
    for item in config['controllers'].values():
        c.binding(item)
        c.read(item['path'], item['sha256'])
    c.require(harness.is_absolute() and harness.resolve(strict=True) == harness, 'canonical harness source root')
    expected_sources = set(c.OWN_SOURCES) | {'measurement/' + name for name in c.MEASUREMENT_SOURCES}
    c.require(type(config['harness_sources']) is dict and set(config['harness_sources']) == expected_sources,
              'exact qualified harness source roster')
    for name, digest in config['harness_sources'].items():
        c.read(harness / name, digest)
    c.require(type(config['external_sources']) is dict
              and set(config['external_sources']) == set(c.EXTERNAL_SOURCES),
              'exact binder and packet replay test source roster')
    for name, digest in config['external_sources'].items():
        c.read(harness.parent / name, digest)
    cpu = {'schema': 'FerricV19Packet55cCpuQualificationV1',
        'runtime_source_sha256': config['runtime_source']['sha256'],
        'controller_source_sha256': config['controller_source']['sha256'],
        'worker_sha256': config['worker']['sha256'],
        'controllers': {key: item['sha256'] for key, item in config['controllers'].items()},
        'phases': config['phases'], 'harness_sources': config['harness_sources'],
        'external_sources': config['external_sources'], 'guard_sources': config['guard_sources'],
        'controller_evidence': config['controller_evidence'], 'runtime_evidence': config['runtime_evidence'],
        'harness_evidence': config['harness_evidence'], 'v19_evidence': config['v19_evidence']}
    build = {key: config[key] for key in ('runtime_main', 'runtime_source', 'controller_source', 'worker', 'controllers')}
    build.update(schema='FerricV19Packet55cBuildBindingV1',
                 cpu_qualification={'path': '/not-yet-created/cpu.json', 'sha256': '0' * 64})
    c.validate_build_inputs(build)
    c.validate_cpu(cpu, build)
    return build, cpu


def emit(config, harness, output):
    c.require(c.PACKET_TICKS_QUALIFIED, 'packet-tick controller/source/CPU qualification is pending')
    build, cpu = qualify(config, harness)
    c.require(output.is_absolute() and output.parent.resolve(strict=True) == output.parent,
              'canonical fresh binding destination')
    os.mkdir(output, 0o700)
    cpu_raw = c.encoded(cpu)
    with (output / 'cpu.json').open('xb') as target:
        target.write(cpu_raw)
    build['cpu_qualification'] = {'path': str(output / 'cpu.json'), 'sha256': hashlib.sha256(cpu_raw).hexdigest()}
    build_raw = c.encoded(build)
    with (output / 'build.json').open('xb') as target:
        target.write(build_raw)
    with (output / 'inputs.json').open('xb') as target:
        target.write(c.encoded(config))
    return {'schema': 'FerricPacketTicksBindingResultV1', 'qualified': True, 'launched': False,
        'build': {'path': str(output / 'build.json'), 'sha256': hashlib.sha256(build_raw).hexdigest()},
        'cpu': build['cpu_qualification'], 'runtime_main': c.RUNTIME_MAIN,
        'phase_profiles': {name: item['profile'] for name, item in config['phases'].items()}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--config-sha256', required=True)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--check-only', action='store_true')
    args = parser.parse_args()
    os.umask(0o077)
    config = c.decode(c.read(args.config, args.config_sha256, 1024**2)[0])
    if args.check_only:
        c.require(args.output is None, 'check-only mode never creates output')
        build, cpu = qualify(config, D / 'harness')
        result = {'schema': 'FerricPacketTicksBindingReviewV1', 'inputs_validated': True,
                  'native_admitted': False, 'launched': False,
                  'executed_tests': c.validate_cpu(cpu, build),
                  'phase_profiles': {name: item['profile'] for name, item in cpu['phases'].items()}}
    else:
        c.require(args.output is not None, 'fresh binding output required')
        result = emit(config, D / 'harness', args.output)
    print(c.encoded(result).decode(), end='')


if __name__ == '__main__':
    main()
