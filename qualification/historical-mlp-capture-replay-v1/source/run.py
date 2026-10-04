"""CPU-only conditional MLP arithmetic on authenticated historical V227 captures."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import struct
import sys
import types

HISTORICAL_HELPER = (14960, '88f6e66fccb2ea1ef0d6079858a4b6af1d0b609fa50ec3ea02cb21329300021a')
HELPERS = {
    'reference.py': (23782, '2e41d6e5715cc561bf818fdee794e4ced74a77f93acd4961d8b15f427f196c6e'),
    'extract.py': (10473, '76872f3558d6f90c21b9eee3ec0ba4ebdd79168b620338faa6bdbdd8b78bdf4a'),
    'policy.json': (2799, '9497e55e70a42630d50b74ea33b0856a385d051fb60bc7f23124ee5cbd46e44b'),
}
FIXTURE = dict(
    path='/home/harmenon/ferric-asrock-42/evidence/resident-layer-tp2-v218/reference-fixtures-v1/fixture/manifest.json',
    bytes=4230, sha256='bda8f0697ba29928e3f4f0e130fdb1a69d3f9eb094988ffe900287dda145bcbc')
MLP_IMAGE = (32328, 'ead57f9bb74d47de14ab5b8b84f4728df3ce6e62fac4d8cdf2d4cf0be3e0b1e6')
LABELS = ('baseline', 'candidate')
ROLES = ('post_norm', 'gate', 'up', 'down')
KINDS = dict(post_norm='post_attention_layer_norm', gate='gate_projection',
             up='up_projection', down='down_projection')
STAGES = (('first-residual', 'input_words', 4096, 2),
          ('mlp-norm', 'post_norm_words', 4096, 2),
          ('gate', 'gate_words', 6144, 2), ('up', 'up_words', 6144, 2),
          ('activation', 'activation_words', 6144, 2),
          ('down-partial', 'down_partial_bits', 4096, 4))


def require(ok, message):
    if not ok:
        raise ValueError(message)


def ordinary_python():
    require(not sys.flags.optimize and not os.environ.get('PYTHONOPTIMIZE'),
            'ordinary Python required')


def bootstrap(path, expected=HISTORICAL_HELPER):
    """Load the existing tested reader only after authenticating its source."""
    path = Path(path)
    require(path.is_absolute() and not path.is_symlink() and path.resolve(strict=True) == path,
            'canonical historical helper')
    size, sha = expected
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_size == size and size <= 65536,
                'bounded historical helper')
        raw = stream.read(size + 1)
        after = os.fstat(stream.fileno())
    identity = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(identity(before) == identity(after) == identity(path.stat())
            and len(raw) == size and hashlib.sha256(raw).hexdigest() == sha,
            'historical helper exact unchanged bytes')
    module = types.ModuleType('historical_residual_replay_reader')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def load_reference(reader, historical, directory):
    directory = historical.canonical(directory)
    for name, expected in HELPERS.items():
        reader.read(directory / name, expected)
    dependency = historical.module(reader, directory / 'extract.py', HELPERS['extract.py'],
                                   'historical_mlp_extract')
    present, previous = 'extract' in sys.modules, sys.modules.get('extract')
    sys.modules['extract'] = dependency
    try:
        reference = historical.module(reader, directory / 'reference.py', HELPERS['reference.py'],
                                      'historical_mlp_reference')
    finally:
        if present:
            sys.modules['extract'] = previous
        else:
            sys.modules.pop('extract', None)
    require(reference.np.__version__ == '2.2.6', 'frozen NumPy version 2.2.6')
    return reference


def load_fixture(reader, historical, reference, directory, policy_path):
    directory = historical.canonical(directory)
    require(directory == Path(FIXTURE['path']).parent, 'original retained fixture directory')
    manifest = reference.json_bytes(reader.read(directory / 'manifest.json',
        (FIXTURE['bytes'], FIXTURE['sha256']), FIXTURE['path']))
    require((manifest['numerical_policy']['bytes'], manifest['numerical_policy']['sha256'])
            == HELPERS['policy.json'], 'unchanged fixture policy bytes')
    policy = manifest['numerical_policy']
    require(reader.read(Path(policy['path']), (policy['bytes'], policy['sha256']), policy['path'])
            == reader.read(policy_path, HELPERS['policy.json']), 'fixture and helper policy byte equality')
    # The unchanged fixture reader authenticates shapes, rank sharding, finite
    # BF16 weights, original provenance and every payload hash before use.
    checked, norm_weight, ranks = reference.fixture(directory / 'manifest.json', FIXTURE['sha256'])
    require(checked == manifest, 'same exact fixture consumed by frozen arithmetic')
    records = [manifest['post_norm']] + [row[role] for row in manifest['ranks']
                                        for role in ('gate', 'up', 'down')]
    for record in records:
        reader.read(Path(record['path']), (record['bytes'], record['sha256']), record['path'])
    return manifest, norm_weight, ranks


def selected_image(request, historical):
    record = historical.rust_pin(request['mlp_tiles_image'])
    require((record['bytes'], record['sha256']) == MLP_IMAGE, 'selected historical MLP548 image')
    return record


def uploaded_weights(registration, uploads, manifest, rank, historical):
    require(type(rank) is int and rank in (0, 1), 'rank0 or rank1')
    require(type(uploads.get('version')) is int and uploads['version'] == 1
            and type(uploads.get('uploads')) is list, 'actual uploads schema')
    rows = [row for row in registration['layers'] if row.get('rank') == rank and row.get('layer') == 0]
    require(len(rows) == 1 and type(rows[0]['rank']) is int and type(rows[0]['layer']) is int,
            'one actual rank/layer0 registration')
    result, ids = {}, set()
    for role in ROLES:
        matches = [row for row in rows[0]['weights'] if row.get('kind') == KINDS[role]]
        require(len(matches) == 1, 'one registered MLP weight role')
        buffer = matches[0]['buffer']
        expected = manifest['post_norm'] if role == 'post_norm' else manifest['ranks'][rank][role]
        require(set(buffer) == {'rank', 'id', 'elements', 'element_bytes'}
                and all(type(buffer[key]) is int for key in buffer)
                and buffer['rank'] == rank and buffer['id'] >= 0 and buffer['id'] not in ids
                and buffer['element_bytes'] == 2 and buffer['elements'] * 2 == expected['bytes'],
                'actual weight rank, unique source and extent')
        ids.add(buffer['id'])
        key = dict(kind='source', rank=rank, id=buffer['id'])
        matches = [row for row in uploads['uploads'] if row.get('key') == key]
        require(len(matches) == 1 and set(matches[0]) == {'key', 'bytes', 'sha256'},
                'one actual source weight upload')
        value = matches[0]
        require(type(value['key']['rank']) is int and type(value['key']['id']) is int,
                'integer uploaded rank and source')
        pin = historical.rust_pin(dict(path=expected['path'], bytes=value['bytes'], sha256=value['sha256']))
        require(pin['bytes'] == expected['bytes'] and pin['sha256'] == expected['sha256'],
                'actual uploaded weight equals authentic fixture')
        result[role] = value
    return result


def capture_inputs(raw, validator, historical, input_record):
    _, spans = historical.residual_rows(raw, validator, input_record)
    mapped = {(row['rank'], row['stage']): row for row in spans}
    require(len(mapped) == 28, 'unique complete historical capture slices')
    ranks = []
    for rank in range(2):
        record = {}
        for stage, field, count, width in STAGES:
            span = mapped[(rank, stage)]
            require(span['bytes'] == count * width and span['element_bytes'] == width,
                    'MLP capture stage geometry')
            part = raw[span['offset']:span['offset'] + span['bytes']]
            require(len(part) == count * width and historical.digest(part) == span['sha256'],
                    'MLP capture slice bytes')
            record[field] = list(struct.unpack('<' + ('H' if width == 2 else 'I') * count, part))
        ranks.append(record)
    return ranks, [row for row in spans if row['stage'] in {stage[0] for stage in STAGES}]


def independent_stages(reference, weights, norm_weight, raw):
    inputs = reference.words(raw['input_words'], 4096)
    norm = reference.words(raw['post_norm_words'], 4096)
    gate = reference.words(raw['gate_words'], 6144)
    up = reference.words(raw['up_words'], 6144)
    activation = reference.words(raw['activation_words'], 6144)
    down = reference.words(raw['down_partial_bits'], 4096, 32)
    return dict(norm=reference.check_norm(inputs, norm_weight, norm),
                gate=reference.check_bf16_gemv(weights['gate'], norm, gate),
                up=reference.check_bf16_gemv(weights['up'], norm, up),
                swiglu=reference.check_swiglu(gate, up, activation),
                down=reference.check_f32_gemv(weights['down'], activation, down))


def compare_profiles(captures, registrations, uploads, manifest, reference, norm_weight, weights, historical):
    require(set(captures) == set(registrations) == set(uploads) == set(LABELS), 'both historical profiles')
    results = []
    for label in LABELS:
        require(len(captures[label]) == 2, 'both captured ranks')
        ranks = []
        for rank in range(2):
            joined = uploaded_weights(registrations[label], uploads[label], manifest, rank, historical)
            stages = independent_stages(reference, weights[rank], norm_weight, captures[label][rank])
            ranks.append(dict(rank=rank, actual_weight_uploads=joined, stages=stages))
        results.append(dict(profile=label, ranks=ranks))
    return results


def replay(evidence_root, helper_path, validator_path, reference_dir, fixture_dir, mlp_image):
    ordinary_python()
    historical = bootstrap(helper_path)
    reader = historical.Reader(evidence_root)
    reader.read(historical.canonical(helper_path), HISTORICAL_HELPER)
    validator = historical.module(reader, historical.canonical(validator_path), historical.VALIDATOR,
                                  'historical_layer_validator')
    receipt = validator.parse(reader.original(historical.REPLAY))
    require(receipt['schema'] == 'ferric-p227-prefix-layer-replay-v1' and receipt['passed'] is True
            and receipt['original_status'] == 'FAILED_UNCHANGED' and receipt['new_native_attempts'] == 0,
            'successful data replay of unchanged historical attempt')
    request = validator.parse(reader.original(receipt['request']))
    files = {name: reader.original(pin) for name, pin in receipt['retained_native'].items()}
    require(set(files) == validator.BODY | {'summary.json'}, 'exact retained native roster')
    checked = validator.validate(files.pop('summary.json'), files, request)
    require(checked == receipt['checked'] and checked['layer'] == 0 and checked['position'] == 0
            and checked['token'] == 9112, 'unchanged historical layer0 position0')
    image = selected_image(request, historical)
    reader.read(historical.canonical(mlp_image), MLP_IMAGE, image['path'])
    reference_dir = historical.canonical(reference_dir)
    reference = load_reference(reader, historical, reference_dir)
    manifest, norm_weight, weights = load_fixture(reader, historical, reference, fixture_dir,
                                                reference_dir / 'policy.json')
    captures, slices, registrations, uploads = {}, {}, {}, {}
    for label in LABELS:
        raw = files[label + '-capture.bin']
        require(len(raw) == historical.CAPTURE_BYTES and historical.digest(raw) == historical.CAPTURE_SHA,
                'exact authentic historical capture bytes')
        boot = validator.parse(files[label + '-bootstrap.json'])
        captures[label], slices[label] = capture_inputs(raw, validator, historical, boot['input'])
        registrations[label] = validator.parse(files[label + '-registration.json'])
        uploads[label] = validator.parse(files[label + '-uploads.json'])
        require(registrations[label]['model_id'] == request['expected_model_id']
                and registrations[label]['bundle_id'] == request['expected_bundle_id'],
                'actual requested model identity')
    profiles = compare_profiles(captures, registrations, uploads, manifest, reference,
                                norm_weight, weights, historical)
    reader.recheck()
    return dict(schema='ferric-p228-historical-mlp-capture-replay-v1', authority='none', passed=True,
        historical_record=historical.REPLAY, original_status=receipt['original_status'],
        historical_layer_validator_checks_revalidated=True, layer=0, position=0, token=9112,
        historical_v227_image=True, new_v7_image_checked=False, gpu_execution_requested=False,
        model=dict(model_id=request['expected_model_id'], bundle_id=request['expected_bundle_id'],
                   original_source_directory=request['source']),
        fixture=FIXTURE, original_extraction_provenance={key: manifest[key] for key in
            ('source', 'index', 'source_inventory', 'extractor', 'numerical_policy')},
        original_model_shard_rehashed=False, historical_mlp_image=image,
        original_captures={label: receipt['retained_native'][label + '-capture.bin'] for label in LABELS},
        capture_slices=slices, profiles=profiles, conditional_stage_checks_passed=True,
        conditional_on_actual_stage_inputs=True, fixed_policy_sha256=HELPERS['policy.json'][1],
        independent_stage_reference=True, adaptive_tolerance=False,
        input_pins=list(reader.records.values()), source_postchecks_passed=True,
        residual_arithmetic_rechecked=False, full_layer_acceptance=False, full_model_acceptance=False,
        runtime_premises_discharged=False, arithmetic_prerequisites_verified=False,
        production_authority=False, performance_claim=False)


def main():
    ordinary_python()
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('evidence-root', 'historical-helper', 'validator', 'reference-dir',
                 'fixture-dir', 'mlp-image', 'output'):
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    output = Path(args.output)
    require(output.is_absolute() and not os.path.lexists(output)
            and output.parent.resolve(strict=True) == output.parent, 'fresh canonical output path')
    value = replay(args.evidence_root, args.historical_helper, args.validator,
                   args.reference_dir, args.fixture_dir, args.mlp_image)
    raw = (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()
    require(len(raw) <= 2 << 20, 'bounded result')
    with output.open('xb') as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())


if __name__ == '__main__':
    main()
