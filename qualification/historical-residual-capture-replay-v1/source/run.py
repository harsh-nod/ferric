"""Data-only replay of historical V227 residual captures; never launches code."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
REPLAY = dict(path=str(E / 'prefix-layer-replay-v227-v1/complete.json'), bytes=465821,
              sha256='d9958b9a6d37fc707993aa4dd4ae05ca062820df4692bf9ab37d8df7821f8d8a')
VALIDATOR = (16186, '757c077979b7ee02d4c0e7d222e75292cd9d417927f84ae9a8134344f8e217b9')
REFERENCE_FILES = {
    'residual_reference.py': (4338, '72f22c7134d609034a033e5ac70fb8c67032a95b9ad96eeafb399092b3655ad8'),
    'helpers/residual_oracle.py': (4347, '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3'),
    'test_residual_reference.py': (9284, '4ff70d01d01aef66711b4b129a7f394dc7eb10cece9c3df1a812be1a42b03d0b'),
    'README.md': (6861, '7ece2a87b1c2b3f9fdb60b28985c6d4c07e3dc3d64cf989d9ab79fe3c89732ea'),
}
MODEL = (3996250744, '31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f')
TENSOR_BYTES = 1244659712
TENSOR_SHA = '458b4af1d22ed8d7d12235ed5249a4606c7606e35e775e7ea2244ba623b0312e'
CAPTURE_BYTES = 9670656
CAPTURE_SHA = 'c46324f59a03f3db83e891475261cf527d19b7c1b5a981ce21b7b763e182f52f'
LABELS = ('baseline', 'candidate')
STAGES = (('norm', 8192, 2), ('qkv', 6144, 2), ('query', 4096, 2),
          ('key-cache', 2359296, 2), ('value-cache', 2359296, 2), ('attention', 4096, 2),
          ('output-partial', 16384, 4), ('first-residual', 8192, 2), ('mlp-norm', 8192, 2),
          ('gate', 12288, 2), ('up', 12288, 2), ('activation', 12288, 2),
          ('down-partial', 16384, 4), ('final-hidden', 8192, 2))


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(path):
    path = Path(path)
    require(path.is_absolute() and not path.is_symlink() and path.resolve(strict=True) == path,
            'canonical nonsymlink path')
    return path


def stamp(path):
    st = path.stat()
    return [st.st_dev, st.st_ino, st.st_size, st.st_mtime_ns, st.st_ctime_ns]


def rust_pin(value):
    require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}, 'closed Rust FilePin')
    require(type(value['path']) is str and type(value['bytes']) is int and value['bytes'] >= 0,
            'Rust pin path and extent')
    words = value['sha256']
    require(type(words) is list and len(words) == 32
            and all(type(word) is int and 0 <= word <= 255 for word in words), 'Rust digest bytes')
    return dict(value, sha256=bytes(words).hex())


class Reader:
    def __init__(self, evidence_root):
        self.root = canonical(evidence_root)
        self.records = {}

    def read(self, path, expected, original=None, limit=64 << 20):
        path = canonical(path)
        size, sha = expected
        require(type(size) is int and 0 <= size <= limit and type(sha) is str
                and len(sha) == 64 and path.is_file(), 'bounded file pin')
        before = stamp(path)
        with path.open('rb') as stream:
            raw = stream.read(size + 1)
        require(stamp(path) == before and len(raw) == size and digest(raw) == sha, 'exact pinned bytes')
        record = dict(path=str(path), bytes=size, sha256=sha, original_path=original)
        previous = self.records.setdefault(str(path), record)
        require(previous == record, 'conflicting file identity')
        return raw

    def original(self, pin):
        require(type(pin) is dict and set(pin) == {'path', 'bytes', 'sha256'}, 'closed original FilePin')
        original = Path(pin['path'])
        require(original.is_absolute() and '..' not in original.parts, 'original absolute path')
        relative = original.relative_to(E)
        return self.read(self.root / relative, (pin['bytes'], pin['sha256']), str(original))

    def recheck(self):
        for record in list(self.records.values()):
            self.read(record['path'], (record['bytes'], record['sha256']), record['original_path'])


def module(reader, path, expected, name):
    raw = reader.read(path, expected)
    value = types.ModuleType(name)
    value.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), value.__dict__)
    return value


def embedding_upload(registration, uploads):
    require(type(uploads.get('version')) is int and uploads['version'] == 1
            and type(uploads.get('uploads')) is list, 'actual uploads schema')
    globals_ = [entry for entry in registration['globals'] if entry.get('kind') == 'token_embedding']
    require(len(globals_) == 1, 'one registered token embedding')
    buffer = globals_[0]['buffer']
    require(set(buffer) == {'rank', 'id', 'elements', 'element_bytes'}
            and all(type(buffer[key]) is int for key in buffer)
            and buffer['id'] >= 0 and buffer['rank'] == 0
            and buffer['elements'] == 151936 * 4096 and buffer['element_bytes'] == 2,
            'registered embedding geometry')
    key = dict(kind='source', rank=0, id=buffer['id'])
    selected = [entry for entry in uploads['uploads'] if entry.get('key') == key]
    require(len(selected) == 1 and set(selected[0]) == {'key', 'bytes', 'sha256'}, 'one actual embedding upload')
    record = selected[0]
    pin = rust_pin(dict(path='recorded-upload', bytes=record['bytes'], sha256=record['sha256']))
    require(pin['bytes'] == TENSOR_BYTES and pin['sha256'] == TENSOR_SHA, 'original uploaded embedding tensor')
    return record


def tensor_span(header, header_bytes, total_bytes):
    require(type(header) is dict and type(header_bytes) is int and 0 < header_bytes <= 1 << 20,
            'bounded safetensors header')
    tensor = header.get('model.embed_tokens.weight')
    require(type(tensor) is dict and set(tensor) == {'dtype', 'shape', 'data_offsets'}, 'embedding tensor record')
    offsets = tensor['data_offsets']
    require(tensor['dtype'] == 'BF16' and tensor['shape'] == [151936, 4096]
            and type(offsets) is list and len(offsets) == 2
            and all(type(value) is int for value in offsets), 'original BF16 embedding geometry')
    first, end = offsets
    require(0 <= first < end and end - first == TENSOR_BYTES
            and header_bytes + 8 + end <= total_bytes, 'bounded embedding payload span')
    return header_bytes + 8 + first, end - first


def stream_span(stream, tensor_start, tensor_bytes, row_start, row_bytes, chunk_bytes=8 << 20):
    """Hash the complete shard and tensor in one pass, retaining just one row."""
    require(0 <= tensor_start <= row_start and row_start + row_bytes <= tensor_start + tensor_bytes,
            'row inside tensor')
    require(type(chunk_bytes) is int and chunk_bytes > 0, 'positive stream chunk')
    whole, tensor, row = hashlib.sha256(), hashlib.sha256(), bytearray()
    count = 0
    stream.seek(0)
    while True:
        chunk = stream.read(chunk_bytes)
        if not chunk:
            break
        whole.update(chunk)
        for start, length, sink in ((tensor_start, tensor_bytes, tensor.update),
                                    (row_start, row_bytes, row.extend)):
            left, right = max(start, count), min(start + length, count + len(chunk))
            if left < right:
                sink(chunk[left - count:right - count])
        count += len(chunk)
    require(count >= tensor_start + tensor_bytes and len(row) == row_bytes, 'complete tensor and row')
    return count, whole.hexdigest(), tensor.hexdigest(), bytes(row)


def read_embedding(path, token, parse):
    path = canonical(path)
    require(path.is_file() and type(token) is int and 0 <= token < 151936, 'original token row')
    before = stamp(path)
    require(before[2] == MODEL[0], 'original model shard extent')
    with path.open('rb') as stream:
        header_size = struct.unpack('<Q', stream.read(8))[0]
        require(0 < header_size <= 1 << 20, 'bounded model header')
        header = parse(stream.read(header_size))
        first, size = tensor_span(header, header_size, MODEL[0])
        count, shard_sha, tensor_sha, raw = stream_span(stream, first, size, first + token * 8192, 8192)
    require(stamp(path) == before and (count, shard_sha) == MODEL and tensor_sha == TENSOR_SHA,
            'original model shard and actual uploaded tensor hashes')
    return raw, dict(path=str(path), bytes=count, sha256=shard_sha, stamp=before,
                     tensor='model.embed_tokens.weight', tensor_offset=first, tensor_bytes=size,
                     tensor_sha256=tensor_sha, token=token, row_offset=first + token * 8192,
                     row_bytes=len(raw), row_sha256=digest(raw))


def residual_rows(raw, validator, input_record):
    require(tuple(validator.STAGES) == STAGES and validator.CAPTURE_BYTES == CAPTURE_BYTES,
            'exact retained 28-array capture contract')
    rows = validator.capture(raw, input_record)
    require(len(rows) == 28, 'all28 capture arrays')
    indexed, pins, offset = {}, [], 0
    for index, part in enumerate(rows):
        name, size, width = STAGES[index % 14]
        require(len(part) == size and raw[offset:offset + size] == part, 'exact ordered capture offset')
        rank = index // 14
        indexed[(rank, name)] = part
        pins.append(dict(rank=rank, stage=name, offset=offset, bytes=size, element_bytes=width,
                         sha256=digest(part)))
        offset += size
    require(offset == len(raw) == CAPTURE_BYTES, 'complete capture partition')
    pairs = {name: tuple(indexed[(rank, name)] for rank in range(2)) for name in
             ('output-partial', 'first-residual', 'down-partial', 'final-hidden')}
    return pairs, pins


def replay(evidence_root, validator_path, reference_dir, model_shard, prompt_tokens):
    reader = Reader(evidence_root)
    validator = module(reader, canonical(validator_path), VALIDATOR, 'historical_layer_validator')
    receipt = validator.parse(reader.original(REPLAY))
    require(receipt['schema'] == 'ferric-p227-prefix-layer-replay-v1' and receipt['passed'] is True
            and receipt['original_status'] == 'FAILED_UNCHANGED' and receipt['new_native_attempts'] == 0,
            'exact successful data replay of unchanged historical attempt')
    request = validator.parse(reader.original(receipt['request']))
    files = {name: reader.original(pin) for name, pin in receipt['retained_native'].items()}
    require(set(files) == validator.BODY | {'summary.json'}, 'exact retained native evidence roster')
    summary = files.pop('summary.json')
    checked = validator.validate(summary, files, request)
    require(checked == receipt['checked'] and checked['token'] == 9112 and checked['position'] == 0,
            'unchanged recorded layer replay checks')
    prompt_pin = rust_pin(request['prompt']['tokens'])
    prompt = reader.read(canonical(prompt_tokens), (prompt_pin['bytes'], prompt_pin['sha256']),
                         prompt_pin['path'])
    require(len(prompt) == 8192 and struct.unpack_from('<I', prompt)[0] == checked['token'],
            'authentic first prompt token')
    reference_dir = canonical(reference_dir)
    for name, expected in REFERENCE_FILES.items():
        reader.read(reference_dir / name, expected)
    comparator = module(reader, reference_dir / 'residual_reference.py',
                        REFERENCE_FILES['residual_reference.py'], 'tested_residual_comparator')
    uploads = []
    for label in LABELS:
        registration = validator.parse(files[label + '-registration.json'])
        require(registration['model_id'] == request['expected_model_id']
                and registration['bundle_id'] == request['expected_bundle_id'], 'actual requested model identity')
        uploads.append(embedding_upload(registration, validator.parse(files[label + '-uploads.json'])))
    require(uploads[0] == uploads[1], 'same original embedding upload in both profiles')
    hidden, model = read_embedding(model_shard, checked['token'], validator.parse)
    results = []
    for label in LABELS:
        raw = files[label + '-capture.bin']
        require(len(raw) == CAPTURE_BYTES and digest(raw) == CAPTURE_SHA, 'exact original historical capture')
        bootstrap = validator.parse(files[label + '-bootstrap.json'])
        stages, spans = residual_rows(raw, validator, bootstrap['input'])
        result = comparator.compare_stages((hidden, hidden), stages['output-partial'],
            stages['first-residual'], stages['down-partial'], stages['final-hidden'])
        results.append(dict(profile=label, capture=receipt['retained_native'][label + '-capture.bin'],
                            capture_slices=spans, conditional_comparison=result))
    reader.recheck()
    require(stamp(canonical(model_shard)) == model['stamp'], 'unchanged model after comparisons')
    return dict(schema='ferric-p228-historical-two-residual-replay-v1', authority='none',
        passed=True, historical_record=REPLAY, historical_layer_validator_checks_revalidated=True,
        layer=0, position=0, token=9112, historical_v227_image=True,
        new_v7_image_checked=False, gpu_execution_requested=False,
        model=dict(original_directory=request['source'], model_id=request['expected_model_id'],
                   bundle_id=request['expected_bundle_id'], original_embedding_upload=uploads[0], source=model),
        historical_images=dict(baseline=request['images']['prefix'], candidate=request['prefix_tiles_image'],
                               residual=request['images']['residual'], mlp=request['mlp_tiles_image']),
        profiles=results, input_pins=list(reader.records.values()), source_postchecks_passed=True,
        full_layer_acceptance=False, full_model_acceptance=False, mlp_numerics_checked=False,
        runtime_premises_discharged=False, arithmetic_prerequisites_verified=False,
        production_authority=False, performance_claim=False)


def main():
    require(not sys.flags.optimize and not os.environ.get('PYTHONOPTIMIZE'), 'ordinary Python required')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--evidence-root', required=True)
    parser.add_argument('--validator', required=True)
    parser.add_argument('--reference-dir', required=True)
    parser.add_argument('--model-shard', required=True)
    parser.add_argument('--prompt-tokens', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    output = Path(args.output)
    require(output.is_absolute() and not output.exists() and not output.is_symlink(), 'fresh output path')
    canonical(output.parent)
    result = replay(args.evidence_root, args.validator, args.reference_dir, args.model_shard,
                    args.prompt_tokens)
    with output.open('x', encoding='utf-8') as stream:
        json.dump(result, stream, indent=2, sort_keys=True)
        stream.write('\n')


if __name__ == '__main__':
    main()
