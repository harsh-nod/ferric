"""Bounded data-only courier for one completed linked-RoPE AR4 native case."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import stat
import tarfile
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
CASE = E / 'prefix-rope-indexed-ar4-gpu-v228-v1'
INPUTS = E / 'prefix-rope-indexed-ar4-inputs-v228-v1'
PLAN_SHA = '4e9b2e9ce3f1d43c8e242d75508f42def544b675a54ba09e80d25ae67ccd285c'
COMMON_SHA = '5ba8484263ac02051a06b488f0db6b9565f6cff31d189c8dd35f07d90b08b720'
LEAVES = {'parent', *[side + '-' + str(i) for side in ('before', 'after') for i in range(3)]}
SUFFIXES = {'command.json', 'started.json', 'result.json', 'stdout', 'stderr'}
NATIVE = {'complete.json', 'child-stderr.bin', *[f'{kind}-{i}.{suffix}'
    for kind, suffix in (('observation', 'bin'), ('control', 'bin'), ('request', 'json')) for i in range(4)]}


def common(path):
    path = Path(path).absolute()
    if path.resolve(strict=True) != path or not path.is_file():
        raise ValueError('canonical retained data helper')
    before = path.stat(); raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns, s.st_mode, s.st_nlink)
    if stamp(before) != stamp(path.stat()) or hashlib.sha256(raw).hexdigest() != COMMON_SHA:
        raise ValueError('exact stable retained data helper')
    module = types.ModuleType('rope_ar4_courier_data'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    module.read(path, dict(bytes=len(raw), sha256=COMMON_SHA))
    return module


def roster(X, reader, complete_sha):
    X.digest(complete_sha)
    records = {}
    def add(path, expected=None):
        path = Path(path); name = str(X.relative(str(path.relative_to(E))))
        raw = reader(path, expected)
        record = dict(path=str(path), **X.extent(raw))
        X.require(name not in records or records[name] == record, 'unchanged courier alias')
        records[name] = record
        X.require(len(records) <= 62 and sum(p['bytes'] for p in records.values()) <= 32 << 20,
                  'bounded case evidence only')
        return raw
    raw = add(CASE / 'complete.json')
    X.require(X.extent(raw)['sha256'] == complete_sha, 'root-supplied actual completion')
    value = X.parse(raw)
    X.require(value['schema'] == 'ferric-p228-rope-indexed-ar4-gpu-v1' and value['passed'] is True
        and value['failures'] == [] and value['native_attempts'] == 1 and value['retries'] == 0
        and value['captured_payloads'] == 4 and value['captured_tensor_rows'] == 152
        and value['numerical_acceptance'] is False and value['performance_claim'] is False
        and value['production_authority'] is False, 'actual structural observation, not acceptance')
    X.require(set(value['leaves']) == LEAVES and set(value['retained_native']) == NATIVE,
              'closed seven-leaf/fourteen-native roster')
    X.require(value['observation']['path'] == str(CASE / 'observation.json'), 'observation namespace')
    add(CASE / 'observation.json', value['observation'])
    for name, leaf in value['leaves'].items():
        X.require(set(leaf['retained_files']) == SUFFIXES
            and leaf['retained_files']['result.json'] == leaf['result'], 'five records per owned leaf')
        for suffix, pin in leaf['retained_files'].items():
            X.require(pin['path'] == str(CASE / name / suffix), 'owned leaf namespace')
            add(Path(pin['path']), X.pin(pin))
    for name, pin in value['retained_native'].items():
        X.require(pin['path'] == str(CASE / 'native' / name), 'native namespace')
        if name.startswith('observation-'):
            X.require(pin['bytes'] == 606976, 'exact four payload extents')
        add(Path(pin['path']), X.pin(pin))
    for side in ('before', 'after'):
        X.require(len(value[side + '_audits']) == 3, 'three audits per side')
        for i, audit in enumerate(value[side + '_audits']):
            pin = X.pin(audit['topology'])
            X.require(pin['path'] == str(CASE / f'{side}-{i}-topology.json')
                and audit['process_result'] == value['leaves'][f'{side}-{i}']['result'], 'audit identity')
            add(Path(pin['path']), pin)
    case_names = {str(Path(name).relative_to(CASE.name)) for name in records}
    actual = set()
    for root, dirs, files in os.walk(CASE, followlinks=False,
                                   onerror=lambda error: (_ for _ in ()).throw(error)):
        for name in dirs:
            directory = Path(root) / name
            X.require(stat.S_ISDIR(directory.lstat().st_mode), 'no symlink directory')
        for name in files:
            actual.add(str((Path(root) / name).relative_to(CASE)))
    X.require(actual == case_names and len(actual) == 57, 'exact completed case; no private caches')
    plan_pin = X.pin(value['plan'])
    X.require(plan_pin['path'] == str(INPUTS / 'plan.json') and plan_pin['sha256'] == PLAN_SHA,
              'actual reviewed input plan')
    plan = X.parse(add(Path(plan_pin['path']), plan_pin))
    X.require(plan['output_label'] == CASE.name and plan['schema'] == 'ferric-p228-rope-indexed-ar4-inputs-v1',
              'same input namespace')
    for key in ('request', 'decode_review', 'parent_runtime_review', 'worker_runtime_review'):
        pin = X.pin(plan[key])
        X.require(value[key] == pin, 'plan/observation input identity')
        if key in ('request', 'decode_review'):
            X.require(Path(pin['path']).parent == INPUTS, 'new input namespace')
        else:
            X.require(Path(pin['path']).parent == E / 'prefix-projection-ar4-decode-inputs-v228-v1',
                      'retained CPU1037 runtime review namespace')
        add(Path(pin['path']), pin)
    X.require(len(records) == 62, '57 case and five input bodies')
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('common_helper', type=Path)
    parser.add_argument('actual_complete_sha256')
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    X = common(args.common_helper); out = args.archive.absolute()
    X.require(out.parent == E and out.name == 'rope-indexed-ar4-native-evidence-v228-v1.tar.gz'
        and not os.path.lexists(out), 'fresh exact native evidence archive')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 32 << 20), (resource.RLIMIT_CORE, 0)):
        hard = resource.getrlimit(kind)[1]
        limit = cap if hard == resource.RLIM_INFINITY else min(cap, hard)
        resource.setrlimit(kind, (limit, limit))
    records = roster(X, X.read, args.actual_complete_sha256)
    manifest = dict(schema='ferric-p228-rope-indexed-ar4-native-export-v1', original_root=str(E),
        completion_sha256=args.actual_complete_sha256, files=records,
        exporter=dict(path=str(Path(__file__).resolve()), **X.extent(X.read(Path(__file__).resolve()))),
        data_helper=dict(path=str(args.common_helper.absolute()), **X.extent(X.read(args.common_helper.absolute()))),
        retained_native_payloads=True, model_or_executable_bodies_exported=False,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False)
    with out.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for name, pin in sorted(records.items()):
            raw = X.read(Path(pin['path']), pin)
            member = tarfile.TarInfo(name); member.size, member.mode = len(raw), 0o600
            archive.addfile(member, io.BytesIO(raw))
        raw = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode('ascii')
        member = tarfile.TarInfo('export-manifest.json'); member.size, member.mode = len(raw), 0o600
        archive.addfile(member, io.BytesIO(raw))
    for pin in records.values(): X.read(Path(pin['path']), pin)
    print(json.dumps(dict(archive=dict(path=str(out), **X.extent(X.read(out))), files=len(records),
        uncompressed_bytes=sum(p['bytes'] for p in records.values()))))


if __name__ == '__main__':
    main()
