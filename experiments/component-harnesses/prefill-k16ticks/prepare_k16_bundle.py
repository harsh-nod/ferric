#!/usr/bin/env python3
"""Prepare a fresh K16 component bundle with no inherited test or image credit."""
import argparse
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import tarfile

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


def prepare(config, output):
    require(type(config) is dict and set(config) == {'schema', 'source_archive', 'stage', 'mode',
            'worker', 'python', 'runtime_evidence', 'cpu_phase', 'image_evidence', 'images'}
            and config['schema'] == 'FerricK16TicksComponentPortableInputsV1', 'closed K16 inputs')
    require(config['mode'] == 'ticks', 'only instrumented raw-tick component timing')
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
            'exact successor executable closure and unchanged ownership guards')
    stage = c.stage_name(config['stage'])
    require(stage.name.startswith('ferric-prefill-k16ticks-component-ticks-'), 'latency namespace')
    runtime = load_module(payload / 'runtime_binding.py')
    require(config['worker']['sha256'] == runtime.WORKER_SHA, 'current runtime worker')
    raw = input_bytes(config['worker'])
    require(len(raw) == runtime.WORKER_BYTES, 'runtime worker size')
    worker = retain('worker-candidate', raw, 0o700)
    provenance = load_module(payload / 'k16_provenance.py')
    evidences = {}
    for field, directory, pins in (('runtime_evidence', 'runtime', runtime.PINS),
                                    ('image_evidence', 'provenance', provenance.PINS)):
        require(type(config[field]) is dict and set(config[field]) == set(pins), 'closed exact evidence inputs')
        evidence = {}
        for name, expected in pins.items():
            item = config[field][name]
            require(item['sha256'] == expected, 'exact existing evidence binding')
            evidence[name] = retain(directory + '/' + name, input_bytes(item))
        evidences[field] = evidence
    require(type(config['images']) is dict and set(config['images']) == set(c.IMAGE_PINS), 'exact two image arms')
    images = {}
    for arm, expected in c.IMAGE_PINS.items():
        item = config['images'][arm]
        require(item['sha256'] == expected, 'exact BF16 control or K16 image')
        images[arm] = retain('images/' + arm + '.hsaco', input_bytes(item))
    require(images['control']['sha256'] == images['paired']['sha256'], 'one unique code object')
    phase = config['cpu_phase']
    require(type(phase) is dict and set(phase) ==
            {'status', 'result', 'stdout', 'stderr', 'source_archive', 'helper', 'inner'},
            'fresh single full-closure harness qualification')
    require(phase['source_archive'] == config['source_archive'], 'actual complete tested source archive')
    new_phase = {name: retain('qualification/harness/' + name,
        input_bytes(item, maximum=8 * 1024**2, empty=name == 'stdout')) for name, item in phase.items()}
    c.binding(config['python'])
    require(config['python']['path'].startswith('/usr/bin/'), 'native system Python binding')
    source_hashes = {name: digest(raw) for name, raw in sources.items()}
    review = {'schema': 'FerricK16TicksComponentReviewedInputsV1', 'engineering_only': True,
        'mode': 'ticks', 'worker_sha256': runtime.WORKER_SHA, 'source_hashes': source_hashes,
        'cpu_phase': new_phase, 'control_compiler_matches_candidate': True, **evidences}
    plan = {'schema': 'FerricK16TicksComponentLaunchPlanV1', 'engineering_only': True,
        'stage': str(payload), 'mode': 'ticks', 'device_unique_id': c.DEVICE,
        'sources': source_hashes, 'files': files, 'worker': worker, 'images': images, 'python': config['python']}
    c.verify_files(payload, files)
    c.validate_review(review, plan)
    logical_review = remap(review, payload, stage)
    plan['review'] = retain('review.json', encoded(logical_review))
    logical_plan = remap(plan, payload, stage)
    with (payload / 'plan.json').open('xb') as stream:
        stream.write(encoded(logical_plan))
    with (output / 'inputs.json').open('xb') as stream:
        stream.write(encoded(config))
    receipt = {'schema': 'FerricK16TicksComponentPortablePreparationV1', 'prepared': True,
        'native_admitted': False, 'launched': False, 'mode': 'ticks', 'stage': str(stage),
        'runtime_revision': runtime.REVISION, 'worker_sha256': runtime.WORKER_SHA,
        'images': c.IMAGE_PINS, 'files': files, 'plan_sha256': digest(encoded(logical_plan)),
        'review_sha256': digest(encoded(logical_review)), 'input_sha256': digest(encoded(config)),
        'control_compiler_matches_candidate': True}
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
