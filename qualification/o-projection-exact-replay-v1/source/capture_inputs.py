"""CPU-only exact QKV dot audit of retained layer-zero BF16 operands."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import resource
import stat
import struct
import sys

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
NATIVE = E / 'prefix-silu-materialized-capture-gpu-v228-v1'
REFERENCE = E / 'layer0-framework-capture-v228-v1'
REFERENCE_ALIAS = E / 'layer0-exact-dot-reference-v228-v1'
MODEL = Path('/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target')
SHARD = 'model-00001-of-00005.safetensors'
SHARD_EXTENT = (3996250744, '31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f')
INDEX_EXTENT = (32878, 'f9fdbcb91c23971c13ec5d5f2573d2349e8f61f2f049371ec699281748fdb1bc')
HEADER_EXTENT = (9328, '979bbeed365485ddaa67a1ed41d0289e15e2f3ba0b3388cb93e42d31f346d1df')
NATIVE_EXTENT = (1024009, '67a235280b48cb5f6a2c51bcca534c182c843857c4c4a98e86718ed368ef12e4')
REFERENCE_EXTENT = (43427, 'cf7512025bb469e06f87c32f788da807b4e6607297b98bb0b33c4135d1e2c78e')
ORACLE_SHA = 'b9e53a3afa4fb851a8e7231c020e5ed19fb55deb7ce4d9f0bff1454e472acddf'
TEST_SHA = '29f9b64975576ce739e77b69b931a300a52c063f7cc2d21d51fe7edd3dbcecd3'
AUDIT_TEST_SHA = 'ea93ffbac80fb21223e83926c3b042f2b72a08d3644b05fd5f8dc3f4d16e3c5d'
NORM_SHA = '49b3afa6ca46528ec64673ca0e1a660e013fffc08717589ad78221c999fc379f'
# data_offsets are relative to the end of the authenticated safetensors header.
TENSORS = (
    ('q', 'query_projection', 4096, 1588609536, 1622163968, 0),
    ('k', 'key_projection', 1024, 1546666240, 1555054848, 2048),
    ('v', 'value_projection', 1024, 1622163968, 1630552576, 2560),
)


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def pin(path, size, sha):
    return dict(path=str(path), bytes=size, sha256=sha)


def wire_sha(value):
    require(type(value) is list and len(value) == 32
            and all(type(x) is int and 0 <= x <= 255 for x in value), 'wire SHA256')
    return bytes(value).hex()


def parse(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
            value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def open_input(path, size):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input path')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size == size, 'ordinary exact-size input')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    if stamp(os.fstat(fd)) != stamp(before):
        os.close(fd)
        raise ValueError('input changed while opening')
    return fd, stamp(before)


def stable(fd, path, before):
    require(stamp(os.fstat(fd)) == before == stamp(Path(path).lstat()), 'unchanged open/path identity')


def read_body(expected, path):
    require(type(expected['bytes']) is int and 0 <= expected['bytes'] <= 16 << 20
            and re.fullmatch('[0-9a-f]{64}', expected['sha256']), 'bounded input pin')
    fd, before = open_input(path, expected['bytes'])
    try:
        with os.fdopen(fd, 'rb', closefd=False) as stream:
            raw = stream.read(expected['bytes'] + 1)
        stable(fd, path, before)
        require(len(raw) == expected['bytes'] and digest(raw) == expected['sha256'], 'input body SHA256')
        return raw
    finally:
        os.close(fd)


class Reader:
    def __init__(self):
        self.consumed = []

    def read(self, expected, path=None):
        path = Path(expected['path'] if path is None else path)
        raw = read_body(expected, path)
        row = dict(original=expected, retained=pin(path, len(raw), digest(raw)))
        if row not in self.consumed:
            self.consumed.append(row)
        return raw

    def document(self, expected, path=None):
        return parse(self.read(expected, path))

    def postcheck(self):
        for row in self.consumed:
            read_body(row['original'], row['retained']['path'])


def selected_capture(reader):
    outer = reader.document(pin(NATIVE / 'complete.json', *NATIVE_EXTENT))
    require(outer['schema'] == 'ferric-p228-silu-materialized-capture-gpu-v1'
            and outer['passed'] is True and outer['failures'] == []
            and outer['native_attempts'] == 1 and outer['retries'] == 0, 'actual current SiLU observation')
    kept = outer['retained_native']
    names = ('summary.json', 'request.json', 'candidate-bootstrap.json',
             'candidate-registration.json', 'candidate-uploads.json', 'candidate-capture.bin')
    for name in names:
        require(kept[name]['path'] == str(NATIVE / 'native' / name), 'native original path')
    summary = reader.document(kept['summary.json'])
    request = reader.document(kept['request.json'])
    bootstrap = reader.document(kept['candidate-bootstrap.json'])
    registration = reader.document(kept['candidate-registration.json'])
    uploads = reader.document(kept['candidate-uploads.json'])
    raw = reader.read(kept['candidate-capture.bin'])
    require(summary['schema'] == 'FerricFiniteProjectionResidualLayerCaptureObservationV1'
            and summary['request'] == request and summary['run']['bootstrap'] == bootstrap
            and summary['completed_layers'] == 1 and summary['native_closed'] is True
            and summary['run']['child_exit_zero'] is True and summary['run']['process_group_absent'] is True,
            'retained native summary/request/bootstrap joins, not lifecycle replay')
    for name in names[1:]:
        rows = [row for row in summary['files'] if row['name'] == name]
        require(len(rows) == 1 and rows[0]['bytes'] == kept[name]['bytes']
                and wire_sha(rows[0]['sha256']) == kept[name]['sha256'], 'summary body identity')
    input_record = bootstrap['layer']['input']
    require(bootstrap['schema'] == 'FerricProjectionResidualLayerBootstrapV1'
            and input_record['token'] == 9112 and input_record['generation'] == 1
            and input_record['cache_metadata'][0] == 0
            and outer['checked']['token'] == 9112 and outer['checked']['position'] == 0,
            'actual layer-zero token/position')
    selected, cursor = {}, 0
    require(len(summary['stages']) == 28, 'retained typed capture roster')
    for row in summary['stages']:
        size = row['bytes']
        require(row['offset'] == cursor and size == row['elements'] * row['element_bytes']
                and cursor + size <= len(raw), 'contiguous typed capture ranges')
        body = raw[cursor:cursor + size]
        require(digest(body) == wire_sha(row['sha256']), 'typed capture slice SHA256')
        key = (row['rank'], row['stage'])
        require(key not in selected, 'unique capture stage')
        selected[key] = body
        cursor += size
    require(cursor == len(raw), 'complete capture extent')
    norm = selected[(0, 'norm')]
    require(len(norm) == 8192 and digest(norm) == NORM_SHA and selected[(1, 'norm')] == norm,
            'same captured BF16 norm on both ranks')
    require(all(len(selected[(rank, 'qkv')]) == 6144 for rank in (0, 1)), 'rank QKV dimensions')
    return outer, request, registration, uploads, norm, selected


def framework(reader, request, registration, norm):
    ref = reader.document(pin(REFERENCE / 'capture.json', *REFERENCE_EXTENT), REFERENCE_ALIAS / 'capture.json')
    require(ref['schema'] == 'ferric-p228-layer0-framework-capture-v1' and ref['status'] == 'PASS'
            and ref['input_token'] == 9112 and ref['position'] == 0
            and ref['genuine_framework_chain'] is True and ref['repeat_passes_byte_equal'] is True
            and ref['candidate_intermediate_inputs'] is False and ref['conditional_replay_performed'] is False,
            'genuine retained framework reference')
    for kind in ('model_id', 'bundle_id'):
        require(ref[kind] == wire_sha(request['layer']['expected_' + kind]) == wire_sha(registration[kind]),
                'native/reference model and bundle identities')
    require(request['layer']['source'] == str(MODEL.parent), 'selected original model path')
    require(len(ref['passes']) == 2, 'two genuine framework passes')
    passes = []
    for ordinal, report in enumerate(ref['passes'], 1):
        require(report['ordinal'] == ordinal and report['fresh_cache'] is True
                and report['input_token'] == 9112 and report['position'] == 0, 'reference pass scope')
        stages = report['stages']
        bodies = {}
        for name, width in (('input-norm', 4096), ('q-projection', 4096),
                            ('k-projection', 1024), ('v-projection', 1024)):
            row = stages[name]
            original = row['pin']
            require(row['dtype'] == 'bfloat16' and row['shape'] == [1, 1, width]
                    and original['bytes'] == 2 * width
                    and original['path'] == str(REFERENCE / f'pass{ordinal}-{name}.bf16'), 'framework typed stage')
            bodies[name] = reader.read(original, REFERENCE_ALIAS / Path(original['path']).name)
        require(bodies['input-norm'] == norm, 'identical native/framework dot inputs')
        for letter in ('q', 'k', 'v'):
            row = stages[letter + '-input']
            require(row['dtype'] == 'bfloat16' and row['shape'] == [1, 1, 4096]
                    and row['pin'] == pin(REFERENCE / f'pass{ordinal}-{letter}-input.bf16', 8192, NORM_SHA),
                    'reference records each Linear input as the same norm; redundant bodies not read')
        passes.append(bodies)
    require(passes[0] == passes[1], 'eight consumed framework payloads repeat exactly')
    return ref, passes[0]


def upload_pins(registration, uploads):
    result = {}
    for letter, kind, rows, _, _, _ in TENSORS:
        for rank in (0, 1):
            layers = [row for row in registration['layers'] if row['rank'] == rank and row['layer'] == 0]
            require(len(layers) == 1, 'one registered layer-zero per rank')
            weights = [row['buffer'] for row in layers[0]['weights'] if row['kind'] == kind]
            require(len(weights) == 1, 'one selected weight registration')
            buffer = weights[0]
            require(buffer['rank'] == rank and buffer['elements'] == rows // 2 * 4096
                    and buffer['element_bytes'] == 2, 'rank weight geometry')
            key = dict(kind='source', rank=rank, id=buffer['id'])
            matches = [row for row in uploads['uploads'] if row['key'] == key]
            require(len(matches) == 1 and matches[0]['bytes'] == rows // 2 * 8192, 'rank weight upload extent')
            result[(letter, rank)] = dict(key=key, bytes=matches[0]['bytes'], sha256=wire_sha(matches[0]['sha256']))
    return result


def hash_shard(stream, before):
    stream.seek(0)
    sha, count = hashlib.sha256(), 0
    while True:
        block = stream.read(2 << 20)
        if not block:
            break
        sha.update(block)
        count += len(block)
    stable(stream.fileno(), MODEL / SHARD, before)
    require((count, sha.hexdigest()) == SHARD_EXTENT, 'complete original shard streaming SHA256')


def tensor_geometry(header, index):
    for letter, _, count, start, end, _ in TENSORS:
        stem = f'model.layers.0.self_attn.{letter}_proj'
        require(header[stem + '.weight'] == dict(dtype='BF16', shape=[count, 4096], data_offsets=[start, end])
                and end - start == count * 8192 and 8 + HEADER_EXTENT[0] + end <= SHARD_EXTENT[0],
                'original weight header/key/range geometry')
        require(index['weight_map'][stem + '.weight'] == SHARD and stem + '.bias' not in index['weight_map'],
                'original index selects this shard and no bias')


def row_location(letter, rank, local_row):
    matches = [row for row in TENSORS if row[0] == letter]
    require(len(matches) == 1 and type(rank) is int and rank in (0, 1), 'selected projection/rank')
    _, _, count, start, _, native_start = matches[0]
    require(type(local_row) is int and 0 <= local_row < count // 2, 'rank-local row extent')
    global_row = rank * (count // 2) + local_row
    return global_row, 8 + HEADER_EXTENT[0] + start + global_row * 8192, native_start + local_row


def audit_rows(stream, header, norm, observed, payloads, uploaded, oracle):
    # Decode each finite encoding once; every product/sum below remains an integer.
    units = tuple(None if word & 0x7f80 == 0x7f80 else oracle.decode_units(word) for word in range(65536))
    norm_words = struct.unpack('<4096H', norm)
    left = [units[word] for word in norm_words]
    require(all(value is not None for value in left), 'finite norm')
    native = {rank: struct.unpack('<3072H', observed[(rank, 'qkv')]) for rank in (0, 1)}
    rows, weight_slices = [], []
    for letter, kind, count, start, end, native_start in TENSORS:
        key = f'model.layers.0.self_attn.{letter}_proj.weight'
        reference = struct.unpack(f'<{count}H', payloads[letter + '-projection'])
        for rank in (0, 1):
            _, rank_start, _ = row_location(letter, rank, 0)
            stream.seek(rank_start)
            sha = hashlib.sha256()
            for local_row in range(count // 2):
                raw = stream.read(8192)
                require(len(raw) == 8192, 'full original weight row')
                sha.update(raw)
                right = [units[word] for word in struct.unpack('<4096H', raw)]
                require(all(value is not None for value in right), 'finite weight row')
                total = sum(a * b for a, b in zip(left, right))
                ideal = oracle.round_dot_units(total)
                global_row, byte_offset, native_index = row_location(letter, rank, local_row)
                a, b = native[rank][native_index], reference[global_row]
                da, db = oracle.distance_units(total, a), oracle.distance_units(total, b)
                rows.append(dict(projection=letter, model_key=key, row=global_row, rank=rank,
                    local_row=local_row, shard_byte_offset=byte_offset,
                    native_bf16=f'{a:04x}', framework_bf16=f'{b:04x}', ideal_bf16=f'{ideal:04x}',
                    exact_sum_units=str(total), native_distance_units=str(da), framework_distance_units=str(db),
                    native_matches_ideal=a == ideal, framework_matches_ideal=b == ideal,
                    native_matches_framework=a == b,
                    closer_to_exact='equal' if da == db else ('native' if da < db else 'framework')))
            expected = uploaded[(letter, rank)]
            require(sha.hexdigest() == expected['sha256'], 'original contiguous model rows equal actual rank upload')
            weight_slices.append(dict(model_key=key, rank=rank, shard_byte_offset=rank_start,
                                      **expected))
    require(len(rows) == 6144, 'all Q/K/V rows, no sampled selection')
    return rows, weight_slices


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('label')
    parser.add_argument('source_sha256')
    args = parser.parse_args()
    require(re.fullmatch(r'layer0-exact-dot-v228-v[1-9][0-9]*', args.label)
            and re.fullmatch('[0-9a-f]{64}', args.source_sha256), 'fresh label and explicit source digest')
    require(not sys.flags.optimize and sys.dont_write_bytecode and 'PYTHONOPTIMIZE' not in os.environ,
            'ordinary Python -B')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
            and all(os.environ.get(name) == '' for name in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES',
                                                         'CUDA_VISIBLE_DEVICES')), 'CPU-only MI350 process bounds')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 300),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        hard = resource.getrlimit(kind)[1]
        resource.setrlimit(kind, (min(cap, hard) if hard != resource.RLIM_INFINITY else cap,) * 2)
    out = E / args.label
    require(not os.path.lexists(out), 'fresh output directory')
    reader = Reader()
    folder = Path(__file__).resolve().parent
    for name, sha in (('audit.py', args.source_sha256), ('exact_bf16.py', ORACLE_SHA),
                      ('test_exact_bf16.py', TEST_SHA), ('test_audit.py', AUDIT_TEST_SHA)):
        reader.read(pin(folder / name, (folder / name).stat().st_size, sha))
    spec = importlib.util.spec_from_file_location('layer0_exact_bf16', folder / 'exact_bf16.py')
    oracle = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(oracle)
    _, request, registration, uploads, norm, observed = selected_capture(reader)
    ref, payloads = framework(reader, request, registration, norm)
    uploaded = upload_pins(registration, uploads)
    for name, expected in ((SHARD, SHARD_EXTENT), ('model.safetensors.index.json', INDEX_EXTENT)):
        require((ref['model_sources'][name]['bytes'], ref['model_sources'][name]['sha256']) == expected,
                'reference original model identities')
    index = reader.document(pin(MODEL / 'model.safetensors.index.json', *INDEX_EXTENT))
    config_pin = ref['model_sources']['config.json']
    config = reader.document(pin(MODEL / 'config.json', config_pin['bytes'], config_pin['sha256']))
    require(config['hidden_size'] == 4096 and config['num_attention_heads'] == 32
            and config['num_key_value_heads'] == 8 and config['head_dim'] == 128
            and config['attention_bias'] is False, 'original model QKV geometry and no projection bias')
    fd, before = open_input(MODEL / SHARD, SHARD_EXTENT[0])
    try:
        with os.fdopen(fd, 'rb', closefd=False) as stream:
            hash_shard(stream, before)
            stream.seek(0)
            require(struct.unpack('<Q', stream.read(8))[0] == HEADER_EXTENT[0], 'safetensors header length')
            header_raw = stream.read(HEADER_EXTENT[0])
            require(digest(header_raw) == HEADER_EXTENT[1], 'original header SHA256')
            header = parse(header_raw)
            tensor_geometry(header, index)
            rows, slices = audit_rows(stream, header, norm, observed, payloads, uploaded, oracle)
            hash_shard(stream, before)
    finally:
        os.close(fd)
    reader.postcheck()
    totals = {name: sum(row[name] for row in rows) for name in
              ('native_matches_ideal', 'framework_matches_ideal', 'native_matches_framework')}
    result = dict(schema='ferric-p228-layer0-exact-dot-audit-v1', completed=True,
        layer=0, position=0, input_token=9112, rows=rows, row_count=6144, terms_per_row=4096,
        counts=totals, disagreement_rows=[dict(projection=row['projection'], row=row['row']) for row in rows
                                       if not row['native_matches_framework']],
        exact_sum_scale_power_of_two=-266, distance_scale_power_of_two=-266,
        arithmetic='exact signed sum of BF16 products followed by one BF16 RNE; not an FP32 reduction model',
        model_shard=pin(MODEL / SHARD, *SHARD_EXTENT), model_shard_complete_hash_passes=2,
        header=dict(bytes=HEADER_EXTENT[0], sha256=HEADER_EXTENT[1], absolute_data_start=8 + HEADER_EXTENT[0]),
        weight_upload_slices=slices, consumed=reader.consumed, source_and_input_postchecks_passed=True,
        framework_repeated_selected_payloads_equal=True, common_norm_sha256=NORM_SHA,
        redundant_framework_linear_input_bodies_read=False,
        recorded_framework_linear_input_pins=[report['stages'][letter + '-input']['pin']
                                             for report in ref['passes'] for letter in ('q', 'k', 'v')],
        gpu_execution=False, model_loaded=False, tests_executed=False, o_projection_compared=False,
        native_or_framework_lifecycle_replayed=False, new_rope_image_intermediates_observed=False,
        accumulator_order_proven=False, numerical_acceptance=False, semantic_bug_proven=False,
        full_model_correctness=False, production_authority=False, performance_claim=False,
        sustained_2048_256=False, acceptance_threshold=None)
    raw = (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')
    require(len(raw) <= 16 << 20, 'bounded JSON result')
    out.mkdir(mode=0o700)
    with (out / 'complete.json').open('xb') as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    expected = pin(out / 'complete.json', len(raw), digest(raw))
    read_body(expected, out / 'complete.json')
    print(json.dumps(dict(complete=expected, rows=6144, counts=totals)), flush=True)


if __name__ == '__main__':
    main()
