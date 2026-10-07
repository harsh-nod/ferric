#!/usr/bin/env python3
"""Disabled create-only successor of the exact completed R3 component payload."""
import argparse
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import tarfile

ENABLED = True
BASE_PLAN_SHA = 'f4f51dbdbdb36be3991e53e02422bde2cf05fd1fcec85314114c7422a22524e6'
BASE_REVIEW_SHA = '811d31ade0a59a1324c39deb8ce01da0051dd667c3ff17c104d2112920228558'
IMAGE_PINS = {
    'v5': 'bbe68bc260c94dbfdd1b76a0e0ac5c44d0a182996d7b6cd94a5f66cfb5fc4b7c',
    'candidate': '1b16379c91c945883bfc9aecdbde896853573a92546acd228eba6e232d746cae',
}
MAXIMUM = 128 * 1024**2


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def decode(raw):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=unique,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def relative(name):
    path = Path(name)
    require(type(name) is str and name and not path.is_absolute()
            and '..' not in path.parts and str(path) == name and name != '.',
            'canonical relative name')
    return path


def input_bytes(item, maximum=64 * 1024**2, empty=False):
    require(type(item) is dict and set(item) == {'path', 'sha256'}, 'exact input binding')
    path = Path(item['path'])
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input path')
    with path.open('rb') as source:
        before = os.fstat(source.fileno())
        require(stat.S_ISREG(before.st_mode) and (empty or before.st_size > 0)
                and before.st_size <= maximum, 'bounded regular input')
        raw = source.read(maximum + 1)
        after = os.fstat(source.fileno())
    require(len(raw) == before.st_size and all(getattr(before, field) == getattr(after, field)
            == getattr(path.lstat(), field) for field in
            ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')),
            'stable bounded input')
    require(digest(raw) == item['sha256'], 'input digest')
    return raw


def source_files(raw):
    files, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        for count, member in enumerate(archive, 1):
            require(count <= 512 and (member.isdir() or member.isfile()), 'bounded regular source archive')
            name = member.name.removeprefix('./')
            if member.isdir() and name in ('', '.'):
                continue
            relative(name)
            if member.isdir():
                continue
            require(name not in files and 0 < member.size <= 1024**2, 'distinct nonempty source member')
            total += member.size
            require(total <= 8 * 1024**2, 'source expansion bound')
            value = archive.extractfile(member).read(member.size + 1)
            require(len(value) == member.size, 'complete source member')
            files[name] = value
    require(files and all(name.endswith('.py') for name in files), 'Python-only source closure')
    return files


def remap(value, old, new):
    if type(value) is dict:
        return {key: remap(item, old, new) for key, item in value.items()}
    if type(value) is list:
        return [remap(item, old, new) for item in value]
    if type(value) is str and (value == str(old) or value.startswith(str(old) + '/')):
        return str(new) + value[len(str(old)):]
    return value


def load_module(path):
    spec = importlib.util.spec_from_file_location('ordered_preparation_' + path.stem, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def inherited_names(base):
    require(type(base.get('files')) is dict and len(base['files']) <= 512, 'bounded base file roster')
    omitted = set(base['sources']) | {'worker-candidate', 'review.json'}
    return [name for name in base['files'] if name not in omitted
            and not name.startswith('qualification/supervisor-tests/')]


def prepare(config, output):
    require(ENABLED, 'ordered bundle preparation is not qualified/enabled')
    require(type(config) is dict and set(config) == {'schema', 'base_payload', 'source_archive',
            'stage', 'mode', 'worker', 'runtime_evidence', 'supervisor_phase'}
            and config['schema'] == 'FerricOrderedComponentPortableInputsV1', 'closed ordered inputs')
    require(config['mode'] in ('latency', 'counters', 'ticks'), 'one immutable measurement mode')
    base_root = Path(config['base_payload'])
    require(base_root.is_absolute() and base_root.resolve(strict=True) == base_root, 'canonical base payload')
    base_raw = input_bytes({'path': str(base_root / 'plan.json'), 'sha256': BASE_PLAN_SHA})
    base = decode(base_raw)
    require(base['schema'] == 'FerricSplitKComponentLaunchPlanV1'
            and base['review']['sha256'] == BASE_REVIEW_SHA
            and {name: item['sha256'] for name, item in base['images'].items()} == IMAGE_PINS,
            'exact previously prepared compiler/image lineage')
    base_review_raw = input_bytes({'path': str(base_root / 'review.json'), 'sha256': BASE_REVIEW_SHA})
    base_review = decode(base_review_raw)
    sources = source_files(input_bytes(config['source_archive'], 8 * 1024**2))
    require(output.is_absolute() and output.parent.resolve(strict=True) == output.parent,
            'canonical fresh output parent')
    os.mkdir(output, 0o700)
    payload = output / 'payload'
    payload.mkdir(mode=0o700)
    files, total = {}, 0

    def retain(name, raw, mode=0o600):
        nonlocal total
        relative(name)
        require(name not in files and len(files) < 512 and total + len(raw) < MAXIMUM,
                'bounded distinct prepared artifact')
        path = payload / name
        path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        with path.open('xb') as target:
            target.write(raw)
        path.chmod(mode)
        files[name] = {'sha256': digest(raw), 'bytes': len(raw), 'mode': mode}
        total += len(raw)
        return {'path': str(path), 'sha256': digest(raw)}

    for name, raw in sources.items():
        retain(name, raw)
    c = load_module(payload / 'component_contract.py')
    require(set(sources) == set(c.PINS) | set(c.OWN_SOURCES)
            and all(digest(sources[name]) == expected for name, expected in c.PINS.items()),
            'exact successor executable closure and unchanged guard/fixture sources')
    stage = c.stage_name(config['stage'])
    require(stage.name.startswith('ferric-v16-splitk-ordered-' + config['mode'] + '-'),
            'mode-specific output namespace')
    for name in inherited_names(base):
        relative(name)
        item = base['files'][name]
        require(set(item) == {'bytes', 'sha256', 'mode'} and item['mode'] == 0o600,
                'read-only inherited evidence')
        raw = input_bytes({'path': str(base_root / name), 'sha256': item['sha256']}, empty=item['bytes'] == 0)
        require(len(raw) == item['bytes'], 'inherited size identity')
        retain(name, raw)
    retain('history/base-plan.json', base_raw)
    retain('history/base-review.json', base_review_raw)
    runtime = load_module(payload / 'runtime_binding.py')
    require(config['worker']['sha256'] == runtime.WORKER_SHA, 'current runtime worker')
    raw = input_bytes(config['worker'])
    require(len(raw) == runtime.WORKER_BYTES, 'runtime worker size')
    worker = retain('worker-candidate', raw, 0o700)
    require(type(config['runtime_evidence']) is dict
            and set(config['runtime_evidence']) == set(runtime.PINS), 'closed runtime evidence inputs')
    runtime_evidence = {}
    for name, expected in runtime.PINS.items():
        item = config['runtime_evidence'][name]
        require(item['sha256'] == expected, 'selected current runtime evidence')
        runtime_evidence[name] = retain('runtime/' + name, input_bytes(item))
    phase = config['supervisor_phase']
    require(type(phase) is dict and set(phase) == {'status', 'result', 'stdout', 'stderr',
            'source_archive', 'source_directory', 'argv_sha256', 'helper', 'inner'}, 'closed new harness qualification')
    require(phase['source_archive']['sha256'] == config['source_archive']['sha256'],
            'qualified complete successor source archive')
    new_phase = {name: phase[name] for name in ('source_directory', 'argv_sha256')}
    for name in ('status', 'result', 'stdout', 'stderr', 'source_archive', 'helper', 'inner'):
        new_phase[name] = retain('qualification/supervisor-tests/' + name,
            input_bytes(phase[name], maximum=8 * 1024**2, empty=name == 'stdout'))
    phases = {'component-tests': remap(base_review['cpu_phases']['component-tests'], base['stage'], payload),
              'supervisor-tests': new_phase}
    source_hashes = {name: digest(raw) for name, raw in sources.items()}
    images = remap(base['images'], base['stage'], payload)
    review = {'schema': 'FerricOrderedSplitKComponentReviewedInputsV1', 'engineering_only': True,
        'mode': config['mode'], 'worker_sha256': runtime.WORKER_SHA, 'source_hashes': source_hashes,
        'images': remap(base_review['images'], base['stage'], payload),
        'cpu_phases': phases, 'runtime_evidence': runtime_evidence}
    plan = {'schema': 'FerricOrderedSplitKComponentLaunchPlanV1', 'engineering_only': True,
        'stage': str(payload), 'mode': config['mode'], 'device_unique_id': c.DEVICE,
        'sources': source_hashes, 'files': files, 'worker': worker, 'images': images, 'python': base['python']}
    c.verify_files(payload, files)
    c.validate_review(review, plan)
    logical_review = remap(review, payload, stage)
    plan['review'] = retain('review.json', encoded(logical_review))
    logical_plan = remap(plan, payload, stage)
    with (payload / 'plan.json').open('xb') as stream:
        stream.write(encoded(logical_plan))
    with (output / 'inputs.json').open('xb') as stream:
        stream.write(encoded(config))
    receipt = {'schema': 'FerricOrderedComponentPortablePreparationV1', 'prepared': True,
        'native_admitted': False, 'launched': False, 'mode': config['mode'], 'stage': str(stage),
        'base_plan_sha256': BASE_PLAN_SHA, 'base_review_sha256': BASE_REVIEW_SHA,
        'runtime_revision': runtime.REVISION, 'worker_sha256': runtime.WORKER_SHA,
        'images': IMAGE_PINS, 'files': files, 'plan_sha256': digest(encoded(logical_plan)),
        'review_sha256': digest(encoded(logical_review)), 'input_sha256': digest(encoded(config))}
    with (output / 'preparation.json').open('xb') as stream:
        stream.write(encoded(receipt))
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inputs', type=Path, required=True)
    parser.add_argument('--inputs-sha256', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    config = decode(input_bytes({'path': str(args.inputs), 'sha256': args.inputs_sha256}, 1024**2))
    print(encoded(prepare(config, args.output)).decode(), end='')


if __name__ == '__main__':
    main()
