"""Conditional original-weight tail diagnostics from retained TF4 bytes; no launch."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import struct
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
READER_SHA = '259f6f233be23da36eac213bfb5e0c905afa43461461fbb0305efcc45c08d930'
P222_MANIFEST_SHA = '7c19a46b59c67094447152d3e30c2c9bbe1deedfbef4a20c2a23ed4574b3efe3'
P222_SHA = 'a058b58c17446e9fa6bfd1e8e5ae2231fb0727401a609a0d6c02b05c6326c37f'
TF4_SHA = 'db417b2f7d0728577a711aa212564d0f3df783ec33a133fadaf35356ea9c159b'
TF4 = 'prefix-independent-decode-tf4-shared-full-currentness-gpu-v228-v1'
UPLOADS = (178103, 'd5e66dbf7e3abfec463424addb6735f9a3a5b2d50653d56689ee79d01404da47')
PROGRAM = (871211, 'eb8607aae21d2774b188c3477e0b03a0b924dcd75638b18624707e543b743392')
TAIL = (112872, '11f53cfe2d18668f191af9f998f48627211aa09a36d5483e516946032ad54d8e')
NORM = (8192, '4f4f6cc0467f0cf8516b154f3ec0748ed03d82ae24d435dfe678075e9a8e2070')
HEAD = (1244659712, '6e46ee56769d64b9a338f350100bdfca5f26a91c3fb5bf479ffcb36c733f7939')
TRANSPOSE_SHA = 'bbeb36637eac1dcf46c74be1241037752551edf73beed03887938b640f1651bb'
P222_FILES = {'model_reference.py', 'test_model_reference.py', 'policy.json', 'README.txt',
    'helpers/layer_reference_v2.py', 'helpers/extract.py', 'helpers/reference.py',
    'helpers/policy.json', 'helpers/attention_reference.py', 'helpers/residual_oracle.py'}
NORM_NAME, HEAD_NAME = 'model.norm.weight', 'lm_head.weight'
TENSOR_SHAPES = {NORM_NAME: (4096,), HEAD_NAME: (151936, 4096)}
TENSOR_SHARDS = {NORM_NAME: 'model-00004-of-00005.safetensors',
                 HEAD_NAME: 'model-00005-of-00005.safetensors'}
CAPTURE_LAYOUT = {'layer35-hidden': (286720, 8192), 'final-norm': (294912, 8192),
                  'logits': (303104, 303872)}
FALSE = ('gpu_execution_requested', 'numerical_acceptance', 'independent_numerical_acceptance',
    'independent_tensor_acceptance', 'full_model_acceptance', 'full_model_correctness',
    'runtime_premises_discharged', 'arithmetic_prerequisites_verified', 'performance_claim',
    'production_authority', 'sustained_2048_256', 'whole_tail_reference_chain_performed')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def ordinary_python():
    require(not sys.flags.optimize and not os.environ.get('PYTHONOPTIMIZE'),
            'ordinary Python without PYTHONOPTIMIZE required')


def authenticated_reader(path):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path and not path.is_symlink(),
            'canonical frozen diagnostic reader')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb') as stream:
        before = os.fstat(stream.fileno())
        raw = stream.read((1 << 20) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda value: (value.st_dev, value.st_ino, value.st_size,
                           value.st_mtime_ns, value.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and 0 < len(raw) == before.st_size <= 1 << 20
        and stamp(before) == stamp(after) == stamp(path.lstat())
        and hashlib.sha256(raw).hexdigest() == READER_SHA, 'unchanged frozen diagnostic reader')
    module = types.ModuleType('_tail_retained_reader')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def reference_modules(D, pins, directory):
    directory = Path(directory)
    require(directory.is_absolute() and directory.resolve(strict=True) == directory,
            'canonical P222 directory')
    manifest, manifest_pin = pins.json(directory / 'manifest.json', P222_MANIFEST_SHA)
    rows = manifest['files']
    require(type(rows) is list and len(rows) == len(P222_FILES)
        and {row['path'] for row in rows} == P222_FILES, 'exact unchanged P222 source closure')
    for row in rows:
        require(pins.pin(directory / row['path'], row['sha256'])['bytes'] == row['bytes'],
                'P222 source extent')
    model = D.load_module(pins, directory / 'model_reference.py', P222_SHA, '_tail_p222')
    aliases = ('extract', 'reference', 'attention_reference')
    previous = {name: sys.modules.pop(name, None) for name in aliases}
    try:
        helpers = model.bootstrap(directory / 'helpers')
    finally:
        for name, value in previous.items():
            sys.modules.pop(name, None)
            if value is not None:
                sys.modules[name] = value
    return model, helpers, manifest_pin


def byte_digest(value):
    require(type(value) is list and len(value) == 32
        and all(type(item) is int and 0 <= item <= 255 for item in value), 'closed byte digest')
    return bytes(value).hex()


def join_bootstrap(row, pin):
    require(type(row) is dict and set(row) == {'bytes', 'sha256'}
        and type(row['bytes']) is int and row['bytes'] == pin['bytes']
        and byte_digest(row['sha256']) == pin['sha256'], 'actual new-bootstrap content identity')


def tail_uploads(uploads):
    require(type(uploads) is dict and set(uploads) == {'version', 'uploads', 'tail'}
        and type(uploads['version']) is int and uploads['version'] == 1
        and type(uploads['uploads']) is list, 'tail upload manifest schema')
    found = {}
    for name, source_id, expected in ((NORM_NAME, 978, NORM), (HEAD_NAME, 979, HEAD)):
        key = dict(kind='source', rank=0, id=source_id)
        rows = [row for row in uploads['uploads'] if row.get('key') == key]
        require(len(rows) == 1 and set(rows[0]) == {'key', 'bytes', 'sha256'},
                'one original tail source upload')
        row = rows[0]
        require(type(row['bytes']) is int and row['bytes'] == expected[0]
            and byte_digest(row['sha256']) == expected[1], 'original tail payload identity')
        found[name] = row
    tail = uploads['tail']
    require(type(tail) is dict and set(tail) == {'source', 'source_sha256', 'bytes', 'sha256'}
        and tail['source'] == found[HEAD_NAME]['key']
        and tail['source_sha256'] == found[HEAD_NAME]['sha256']
        and type(tail['bytes']) is int and tail['bytes'] == HEAD[0]
        and byte_digest(tail['sha256']) == TRANSPOSE_SHA, 'separate original-NxK/transposed-KxN join')
    return found, tail


def tail_program(program):
    def buffer(source, access, count):
        return dict(kind='buffer', source_id=source, access=access, elements=count,
                    element_bytes=2, offset=0)
    scalar = lambda value, kind='u32': dict(kind=kind, value=value)
    expected = [
        ('qwen3_rmsnorm_v1', 1, [buffer(1490, 'read', 4096), buffer(73, 'read', 0),
            buffer(978, 'read', 4096), buffer(73, 'write', 0), buffer(75, 'write', 4096),
            scalar(1), scalar(4096), scalar(897988541, 'f32_bits'), scalar(0)]),
        ('ferric_qwen3_tp_mfma_gemm_bf16_v3', 9496, [buffer(75, 'read', 65536),
            buffer(1488, 'read', 622329856), buffer(91, 'write', 2430976),
            *map(scalar, (1, 151936, 4096, 2, 6))]),
        ('ferric_qwen3_tp_batch_argmax_bf16_v2', 1, [buffer(91, 'read', 2430976),
            dict(kind='buffer', source_id=92, access='write', elements=16,
                 element_bytes=4, offset=0), scalar(1)]),
    ]
    require(program['schema'] == 'ferric-finite-source-grammar-v1'
        and type(program['steps']) is list and len(program['steps']) >= 3, 'tail source grammar')
    for row, (symbol, grid, arguments) in zip(program['steps'][-3:], expected):
        require(row == dict(kind='rank', rank=0, dispatch=dict(symbol=symbol,
            grid_workgroups=grid, workgroup_size=64, arguments=arguments)), 'exact final three tail dispatches')
    return dict(norm_source_id=978, original_head_source_id=979, uploaded_head_source_id=1488,
                head_input_layout='NxK original; KxN uploaded', head_scalars=[1, 151936, 4096, 2, 6])


def tail_index(model, index):
    require(type(index) is dict and set(index) == {'metadata', 'weight_map'}
        and type(index['weight_map']) is dict, 'original model index schema')
    expected = model.tensor_shapes(model.QWEN8B)
    require(set(index['weight_map']) == set(expected)
        and index['metadata'] == {'total_size': sum(2 * model.math.prod(shape) for shape in expected.values())}
        and set(index['weight_map'].values()) ==
            {f'model-{number:05}-of-00005.safetensors' for number in range(1, 6)},
        'complete authentic model index geometry')
    for name, shard in TENSOR_SHARDS.items():
        require(index['weight_map'][name] == shard and expected[name] == TENSOR_SHAPES[name],
                'norm/head original shard and shape')
    return expected


def digest_layout(np, words, transpose=False, block=16):
    require(words.dtype == np.dtype('<u2') and words.ndim in (1, 2)
        and all(dimension > 0 for dimension in words.shape)
        and type(block) is int and 1 <= block <= 128
        and type(transpose) is bool and (not transpose or words.ndim == 2), 'bounded BF16 layout hash')
    view = words.T if transpose else words
    digest, count = hashlib.sha256(), 0
    for first in range(0, view.shape[0], block):
        raw = np.ascontiguousarray(view[first:first + block]).tobytes(order='C')
        digest.update(raw)
        count += len(raw)
    return dict(bytes=count, sha256=digest.hexdigest())


class TailWeights:
    """Pinned original shards 4/5; map only the two required read-only tensors."""
    def __init__(self, model, directory):
        self.model, self.files, self.arrays, self.payloads = model, [], {}, {}
        directory = Path(directory)
        require(directory.is_absolute() and directory.resolve(strict=True) == directory,
                'canonical original model directory ending model/target')
        self.pins = {}
        for name in ('model.safetensors.index.json', *TENSOR_SHARDS.values()):
            size, sha = model.ORIGINAL_PINS[name]
            pin = dict(path=str(directory / name), bytes=size, sha256=sha)
            model.original_pin(pin, name)
            self.pins[name] = pin
        index = model.json_bytes(model.small_pin(self.pins['model.safetensors.index.json'], 1 << 20))
        expected = tail_index(model, index)
        try:
            for tensor, shard in TENSOR_SHARDS.items():
                source = model.PinnedFile(self.pins[shard], 5 << 30)
                self.files.append(source)
                header_bytes = struct.unpack('<Q', source.read(0, 8))[0]
                require(2 <= header_bytes <= 8 << 20 and 8 + header_bytes < self.pins[shard]['bytes'],
                        'bounded original safetensors header')
                offset = 8 + header_bytes
                header = model.decode_header(source.read(8, header_bytes), self.pins[shard]['bytes'] - offset)
                require(set(header) == {key for key, value in index['weight_map'].items() if value == shard}
                    and all(shape == expected[key] for key, (shape, _, _) in header.items()),
                    'complete original shard header/index membership and geometry')
                shape, first, end = header[tensor]
                self.arrays[tensor] = model.np.memmap(source.stream, mode='r', dtype='<u2',
                    offset=offset + first, shape=shape, order='C')
                self.payloads[tensor] = dict(source=self.pins[shard], offset=offset + first,
                    bytes=end - first, shape=list(shape), layout='N' if len(shape) == 1 else 'NxK')
        except BaseException:
            self.close()
            raise

    def authenticate_uploads(self, uploads):
        found, tail = tail_uploads(uploads)
        result = {}
        for name, words in self.arrays.items():
            actual = digest_layout(self.model.np, words)
            require(actual == dict(bytes=found[name]['bytes'], sha256=byte_digest(found[name]['sha256'])),
                    'original tensor bytes equal the actual source upload')
            result[name] = dict(**self.payloads[name], sha256=actual['sha256'], upload=found[name])
        actual = digest_layout(self.model.np, self.arrays[HEAD_NAME], transpose=True)
        require(actual == dict(bytes=tail['bytes'], sha256=byte_digest(tail['sha256'])),
                'bounded original-NxK transpose equals actual KxN upload')
        return dict(original_tensors=result, transposed_head=dict(**actual,
            shape=[4096, 151936], layout='KxN', upload=tail, source_id=1488))

    def recheck(self):
        for source in self.files:
            source.recheck()
        self.model.small_pin(self.pins['model.safetensors.index.json'], 1 << 20)

    def close(self):
        for array in self.arrays.values():
            array._mmap.close()
        self.arrays.clear()
        for source in self.files:
            source.close()
        self.files.clear()


def conditional_tail(model, helpers, diagnostics, record, raw, norm_weight, head_weight):
    rows = diagnostics.validate_case(record, raw, record['position'])
    for name, (offset, count) in CAPTURE_LAYOUT.items():
        require(rows[name] == raw[offset:offset + count] and len(rows[name]) == count,
                'checked tail capture offsets')
    np = model.np
    words = lambda value: np.frombuffer(value, dtype='<u2')
    old, mlp, _, _ = helpers
    norm = model.dense_norm(helpers, words(rows['layer35-hidden']), norm_weight)
    # Head receives the observed normalized vector, not our predicted norm.
    logits64 = old.dense_project(mlp, head_weight, words(rows['final-norm']))
    logits = old.narrow_projection(mlp, logits64)
    require(norm.dtype == logits.dtype == np.dtype('<u2') and norm.shape == (4096,)
        and logits.shape == (151936,), 'unchanged P222 tail output geometry')
    expected = {'final-norm': norm.tobytes(), 'logits': logits.tobytes()}
    metrics = {name: diagnostics.compare_tensor(predicted, rows[name])
               for name, predicted in expected.items()}
    captured_choice = model.choose(words(rows['logits']), mlp)
    require(captured_choice == record['output_token'], 'captured lowest-index argmax')
    choice = model.choose(logits, mlp)
    return dict(record=record, comparison_scope='conditional on each captured immediate input',
        norm_input='captured layer35-hidden', head_input='captured final-norm',
        tensors=metrics, captured_output_token=captured_choice, reference_head_output_token=choice,
        reference_head_token_equal=(choice == captured_choice), numerical_acceptance=False,
        acceptance_threshold=None), expected


def nonclaims():
    return dict(authority='none', status='CONDITIONAL_TAIL_DIAGNOSTICS_ONLY',
        passed=True, acceptance_threshold=None, **{key: False for key in FALSE})


def source_snapshot(pins):
    for name in ('run.py', 'test_run.py', 'README.md'):
        pins.pin(Path(__file__).resolve().parent / name)
    return {path: pin for path, pin in pins.records.items()
            if Path(path).suffix in ('.py', '.md', '.txt') or Path(path).name in ('manifest.json', 'policy.json')}


def write_bytes(path, raw):
    with path.open('xb') as stream:
        require(stream.write(raw) == len(raw), 'complete diagnostic payload write')
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def execute(args):
    ordinary_python()
    output = Path(args.output)
    require(output.is_absolute() and output.parent.resolve(strict=True) == output.parent
        and not os.path.lexists(output), 'fresh canonical tail diagnostic directory')
    reader = authenticated_reader(args.diagnostic_reader)
    D = reader.bootstrap()
    pins = D.Pins()
    pins.pin(args.diagnostic_reader, READER_SHA)
    I, M, C, H, observer_manifest = reader.loaded(D, pins)
    model, helpers, reference_manifest = reference_modules(D, pins, args.p222_directory)
    receipt_pin, raw = pins.read(Path(args.observation), TF4_SHA, retain=True, maximum=8 << 20)
    receipt = D.parse(raw)
    before = source_snapshot(pins)
    output.mkdir(mode=0o700)
    before_pin = I.save(output / 'sources-before.json', before)
    plan, observed, files, checked = reader.replay_observation(
        I, M, C, H, pins, receipt_pin, receipt, observer_manifest)
    require(receipt['mode'] == 'teacher_forced', 'fixed authenticated TF4 observation')
    documents, retained = {}, {}
    for name, path, expected in (('uploads', args.uploads, UPLOADS),
                                  ('source_program', args.program, PROGRAM)):
        pin, raw = pins.read(Path(path), expected[1], retain=True, maximum=1 << 20)
        require(pin['bytes'] == expected[0], 'exact tail source document extent')
        join_bootstrap(observed['bootstrap']['begin'][name], pin)
        retained[name], documents[name] = pin, D.parse(raw)
    program = tail_program(documents['source_program'])
    tail_pins = [pin for pin in receipt['input_pins'].values()
                 if pin['bytes'] == TAIL[0] and pin['sha256'] == TAIL[1]]
    require(len(tail_pins) == 1, 'one actual authenticated tail image pin')
    tail_pin = pins.pin(tail_pins[0]['path'], TAIL[1])
    require(tail_pin == tail_pins[0], 'exact retained tail image')
    join_bootstrap(observed['bootstrap']['begin']['tail_image'], tail_pin)
    weights = TailWeights(model, args.model_directory)
    try:
        weight_join = weights.authenticate_uploads(documents['uploads'])
        rows = []
        records = C.records(observed)
        require(len(records) == 4, 'all four authenticated TF4 positions')
        for position, record in enumerate(records):
            require(record['position'] == position, 'ordered retained TF4 records')
            row, expected = conditional_tail(model, helpers, H['diagnostics'], record,
                files[f'observation-{position}.bin'], weights.arrays[NORM_NAME], weights.arrays[HEAD_NAME])
            row['capture'] = receipt['retained_native'][f'observation-{position}.bin']
            row['reference_outputs'] = {name: write_bytes(output / f'{position}-{name}.bf16', data)
                                        for name, data in expected.items()}
            for pin in row['reference_outputs'].values():
                require(pins.pin(pin['path'], pin['sha256']) == pin, 'retained predicted output pin')
            rows.append(row)
        weights.recheck()
        pins.recheck()
        after = {path: pins.pin(path, pin['sha256']) for path, pin in before.items()}
        require(before == after, 'reference/reader/controller closure unchanged')
        after_pin = I.save(output / 'sources-after.json', after)
        result = dict(schema='ferric-p228-tail-capture-diagnostic-v1', **nonclaims(),
            observation=receipt_pin, observer_package=observer_manifest,
            reference_package=reference_manifest, request=plan['request'], plan=receipt['plan'],
            selected_runtime=receipt['selected_runtime'], tail_image=tail_pin,
            tail_documents=retained, tail_dispatch=program, original_weights=weights.pins,
            original_weight_upload_join=weight_join, capture_layout=CAPTURE_LAYOUT,
            conditional_diagnostics=rows, compared_stage_rows=8,
            numerical_policy='P222 independent FP64 norm and FP64 dot -> FP32 -> BF16; no tolerance',
            limitations=['not a serial-FP32 norm replay', 'not an MFMA accumulation replay',
                'not a reference-fed complete-tail chain', 'does not accept preceding 36 layers'],
            numpy_version=model.np.__version__, structural=checked['structural'],
            owned_record_checks=checked['owned_record_checks'], retained_native_and_close_replayed=True,
            recorded_six_audit_leaf_bytes_rehashed=True, source_postchecks_passed=True,
            original_weight_postchecks_passed=True, sources_before=before_pin, sources_after=after_pin,
            input_pins=dict(pins.records))
        return I.save(output / 'complete.json', result)
    finally:
        weights.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--diagnostic-reader', default=str(E / 'p228-independent-decode-diagnostic-v1/run.py'))
    parser.add_argument('--observation', default=str(E / TF4 / 'complete.json'))
    parser.add_argument('--uploads', default=str(E / 'prefix-layer-gpu-v227-v1/native/baseline-uploads.json'))
    parser.add_argument('--program', default=str(E / 'prefix-layer-gpu-v227-v1/native/baseline-program.json'))
    parser.add_argument('--p222-directory', required=True)
    parser.add_argument('--model-directory', required=True)
    parser.add_argument('--output', required=True)
    print(json.dumps(execute(parser.parse_args()), sort_keys=True))


if __name__ == '__main__':
    main()
