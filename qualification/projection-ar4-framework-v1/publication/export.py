"""Retain the actual AR4 framework owner/reference closure, excluding private cache."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import tarfile
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
OWNER = E / 'projection-ar4-framework-launch-v228-v1'
REFERENCE = E / 'projection-ar4-framework-reference-v228-v1'
INPUTS = E / 'projection-ar4-framework-inputs-v228-v1'
PACKAGE = E / 'p228-projection-ar4-framework-v1'
OWNER_SHA = '94403d351120c0eb756f5660e333682ca0f47c4c6c1ac0f3cf6e9c10db896771'
REFERENCE_SHA = '00952244362ad51d241d179f741ae5ae61ff8acfcb3fd160b9dc64ce3b5f699e'
NATIVE_SHA = '15938580d218f855883a589c819d532bbf941a02c4677cb29928f7bf7106d1cb'
COMMON_SHA = '5ba8484263ac02051a06b488f0db6b9565f6cff31d189c8dd35f07d90b08b720'
SOURCE_SHAS = {
    'run.py': 'cba39ec416fe1d1ac5d586a44a9a73099598ff3600c51bd8df47f365983c3d33',
    'test_run.py': 'b04e78325e4fc6bfe40e96a6e3b1ca4eca3c488c1c955c31f395f24977857675',
    'launch.py': '2a0d60facae6f27855d6caa40a584e3257f5580c299527e0d367b824e0031304',
    'test_launch.py': 'c83843cb9924b10490b9068963865ea38d817851f5c2a0784e123b1be904fe12',
    'README.md': 'a246371c6c54002fa39b5665c0bafdc70d658a96402621cdeb6d6303d387969f',
}
LABELS = [side + '-' + str(i) for side in ('before', 'after') for i in range(3)] + ['immediate']
LEAVES = {'cpu-tests', 'pre-list', 'inspect', 'execute'} | {
    label + '-' + suffix for label in LABELS for suffix in ('live-path', 'live-sha', 'process')}
OWNER_FILES = {'ready.json', 'approval.json', 'capture-plan.json', 'inputs.json', 'python-identity.json',
    'immediate-idle.json', 'root-execution-review.json', 'complete.json', 'reference-plan.json',
    'capture-projection.json'} | {name + '/' + suffix for name in LEAVES
    for suffix in ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')}


def common(path):
    path = Path(path).absolute()
    if path.resolve(strict=True) != path or not path.is_file():
        raise ValueError('canonical retained data helper')
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != COMMON_SHA:
        raise ValueError('exact retained data helper')
    value = types.ModuleType('ar4_framework_courier_data')
    value.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), value.__dict__)
    value.read(path, dict(bytes=len(raw), sha256=COMMON_SHA))
    return value


def roster(X, reader):
    records = {}
    def add(path, expected=None):
        path = Path(path)
        name = str(X.relative(str(path.relative_to(E))))
        X.require('private-cache' not in path.parts and 'target' not in path.parts
                  and path.suffix not in ('.so', '.rlib', '.safetensors'), 'no cache/model/executable courier')
        raw = reader(path, expected)
        record = dict(path=str(path), **X.extent(raw))
        X.require(name not in records or records[name] == record, 'stable courier alias')
        records[name] = record
        X.require(len(records) <= 256 and sum(p['bytes'] for p in records.values()) <= 32 << 20,
                  'bounded exact evidence closure')
        return raw
    outer_raw = add(OWNER / 'complete.json')
    X.require(X.extent(outer_raw) == dict(bytes=17692, sha256=OWNER_SHA), 'actual terminal owner')
    outer = X.parse(outer_raw)
    X.require(outer['schema'] == 'ferric-p228-projection-ar4-framework-launch-complete-v1'
              and outer['passed'] is True and outer['failures'] == [] and outer['native_attempts'] == 1
              and outer['retries'] == 0 and outer['gpu_execution'] is True
              and all(outer[k] is False for k in ('candidate_gpu_execution', 'numerical_acceptance',
                  'performance_claim', 'production_authority')), 'one successful scoped framework attempt')
    for name in sorted(OWNER_FILES):
        add(OWNER / name)
    X.require(outer['reference'] == dict(path=str(REFERENCE / 'reference.json'), bytes=201239,
                                      sha256=REFERENCE_SHA), 'actual reference identity')
    report = X.parse(add(REFERENCE / 'reference.json', outer['reference']))
    X.require(report['schema'] == 'ferric-p228-projection-ar4-framework-reference-v1'
              and report['status'] == 'PASS' and report['framework_forward_count'] == 8
              and report['conditional_passes'] == [] and report['conditional_replay_performed'] is False
              and report['native_complete']['sha256'] == NATIVE_SHA, 'genuine actual eight forwards')
    add(REFERENCE / 'input-plan.json', outer['plan'])
    X.require(len(report['genuine_passes']) == 2, 'two fresh framework passes')
    for ordinal, run in enumerate(report['genuine_passes'], 1):
        X.require(run['ordinal'] == ordinal and run['fresh_cache'] is True and len(run['cases']) == 4,
                  'closed genuine pass')
        for position, case in enumerate(run['cases']):
            expected_path = REFERENCE / f'genuine-ar-pass{ordinal}-pos{position}.bf16'
            X.require(case['payload']['path'] == str(expected_path) and case['payload']['bytes'] == 606976,
                      'exact retained payload extent/path')
            add(expected_path, X.pin(case['payload']))
    X.require(len(report['retained_implementation_sources']) == 4, 'four installed-source copies')
    for record in report['retained_implementation_sources'].values():
        X.require(Path(record['path']).parent == REFERENCE, 'retained implementation namespace')
        add(Path(record['path']), X.pin(record))
    inputs = X.parse(add(INPUTS / 'inputs.json', outer['input_pins'][0]))
    X.require(inputs == X.parse(reader(OWNER / 'inputs.json')), 'actual launch inputs')
    for record in (inputs['execution_review'], inputs['platform_monitor'], inputs['owned_helper'],
                   inputs['reference_helper'], *inputs['reference_support'].values()):
        add(Path(X.pin(record)['path']), record)
    for name, digest in SOURCE_SHAS.items():
        X.require(X.extent(add(PACKAGE / name))['sha256'] == digest, 'frozen framework source')
    for record in inputs['native']['validators'].values():
        add(Path(X.pin(record)['path']), record)
    X.require(len(report['native_consumed']) == len(inputs['native']['transport']) == 16, 'sixteen native input bodies')
    for pair in report['native_consumed']:
        original, retained = X.pin(pair['original']), X.pin(pair['retained'])
        X.require(inputs['native']['transport'][original['path']] == retained
                  and all(original[k] == retained[k] for k in ('bytes', 'sha256')), 'native transport body identity')
        add(Path(retained['path']), retained)
    return outer, report, inputs, records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('common_helper', type=Path)
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    X = common(args.common_helper)
    out = args.archive.absolute()
    X.require(out.parent == E and out.name == 'projection-ar4-framework-evidence-v228-v1.tar.gz'
              and not os.path.lexists(out), 'fresh exact evidence archive')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 120),
                      (resource.RLIMIT_FSIZE, 32 << 20), (resource.RLIMIT_CORE, 0)):
        hard = resource.getrlimit(kind)[1]
        limit = cap if hard == resource.RLIM_INFINITY else min(cap, hard)
        resource.setrlimit(kind, (limit, limit))
    _, _, _, records = roster(X, X.read)
    manifest = dict(schema='ferric-p228-projection-ar4-framework-export-v1', original_root=str(E),
        owner_sha256=OWNER_SHA, reference_sha256=REFERENCE_SHA, files=records,
        exporter=dict(path=str(Path(__file__).resolve()), **X.extent(X.read(Path(__file__).resolve()))),
        data_helper=dict(path=str(args.common_helper.absolute()), **X.extent(X.read(args.common_helper.absolute()))),
        private_cache_exported=False, model_or_executable_bodies_exported=False)
    with out.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for name, record in sorted(records.items()):
            raw = X.read(Path(record['path']), record)
            member = tarfile.TarInfo(name)
            member.size, member.mode = len(raw), 0o600
            archive.addfile(member, io.BytesIO(raw))
        raw = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode('ascii')
        member = tarfile.TarInfo('export-manifest.json')
        member.size, member.mode = len(raw), 0o600
        archive.addfile(member, io.BytesIO(raw))
    for record in records.values():
        X.read(Path(record['path']), record)
    print(json.dumps(dict(archive=dict(path=str(out), **X.extent(X.read(out))), files=len(records),
                          uncompressed_bytes=sum(r['bytes'] for r in records.values()))))


if __name__ == '__main__':
    main()
