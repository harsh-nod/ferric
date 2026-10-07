#!/usr/bin/env python3
"""Create a closed selected-native matched HTTP plan; no process or GPU launch."""
import argparse
import importlib.util
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent
definition = importlib.util.spec_from_file_location('native_http_preparation_contract', ROOT / 'matched128_contract.py')
c = importlib.util.module_from_spec(definition)
sys.modules[definition.name] = c
definition.loader.exec_module(c)


def binding(path):
    c.require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical retained input required')
    return {'path': str(path), 'sha256': c.digest(path)}


def assemble(native_plan, native_report, arm, workload, tokenizer, target, reference, cpu, campaign):
    sources = {name: c.digest(ROOT / name) for name in c.DRIVER_FILES}
    c.require(campaign in ('V17', 'V19', 'Width55c', 'GateUpDa6b', 'DownDa6b'), 'closed standalone native campaign')
    selection = {'schema': 'Ferric' + campaign + 'HttpSelectionV1', 'arm': arm,
                 'plan': binding(native_plan), 'report': binding(native_report)}
    ferric = c.selection_adapter(sources).admit(selection)
    value = {'schema': c.PLAN_SCHEMA, 'settings': dict(c.SETTINGS),
        'workload': binding(workload), 'tokenizer_receipt': binding(tokenizer),
        'target_manifest': binding(target), 'reference': binding(reference),
        'client': binding(ROOT / 'competitive_benchmark.py'),
        'serving': binding(ROOT / 'frozen/serve_ferric.py'),
        'ferric': ferric, 'driver_sources': sources, 'render_device': '/dev/dri/renderD128',
        'cpu_qualification': binding(cpu) if cpu is not None else None}
    c.validate_plan(value)
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('native-plan', 'native-report', 'workload', 'tokenizer-receipt',
                 'target-manifest', 'reference', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--arm', choices=('A', 'B'), required=True)
    parser.add_argument('--native-campaign', choices=('V17', 'V19', 'Width55c', 'GateUpDa6b', 'DownDa6b'), required=True)
    parser.add_argument('--cpu-qualification', type=Path)
    args = parser.parse_args()
    c.require(args.output.is_absolute() and not args.output.exists()
        and args.output.parent.resolve(strict=True) == args.output.parent, 'fresh canonical plan output')
    value = assemble(args.native_plan, args.native_report, args.arm, args.workload,
        args.tokenizer_receipt, args.target_manifest, args.reference, args.cpu_qualification, args.native_campaign)
    c.write(args.output, value)
    print(c.digest(args.output))


if __name__ == '__main__':
    main()
