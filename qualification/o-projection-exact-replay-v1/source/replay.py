"""CPU-only retained O-projection replay; no model load or GPU execution."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import resource
import signal
import struct
import sys

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
SOURCE_PINS = {
    'capture_inputs.py': {'bytes': 21145, 'sha256': '67b3982c7e1f3f8f64429ee4ee98b45ef266c15da1752b1c2f209a327e20ec40'},
    'exact_bf16.py': {'bytes': 2554, 'sha256': 'b9e53a3afa4fb851a8e7231c020e5ed19fb55deb7ce4d9f0bff1454e472acddf'},
    'fp32_replay.py': {'bytes': 4657, 'sha256': '9a7eb5e4f5c503c83b126c21f6cf880b46dda58881eb22a62f71ca2e4a74cb13'},
    'test_fp32_replay.py': {'bytes': 9013, 'sha256': '87a358807c4f76fbd7b0fc7603dc08de7e34c66871b06aa6d8ac89f165798adb'},
    'test_replay.py': {'bytes': 6718, 'sha256': '4600504366c145957e7d3b906b7d1918f7b40541264664a8f154c711e7342453'},
}
WEIGHT_KEY = 'model.layers.0.self_attn.o_proj.weight'
ROWS, RANK_WIDTH = 4096, 2048
KERNEL_ROOT = E / 'row-reciprocal-checked-probe-v228-v7'
KERNEL_PINS = {
    'complete.json': (25107, '6ba25826b71e30f5106e1efc9e786c021c56f5bab397add20b365373685081a9'),
    'fixture/src/output_projection_numerics_v5.rs': (2033, 'ef397ea5bfe59d6284172e0b2dda6c0f37eef69b48c1392274941d5b521cffd5'),
    'fixture/src/prefix_tiles_numerics_v6.rs': (6092, '3f729b83bc50d4e252a73911ac69bb4e4e1829e1cb05ca3f0bcffa36f09475ad'),
    'extracted/module.ll': (442931, 'df75a6fd370774e05568031fffdbfeca026059542c9cddb6db79b4b881b1e94a'),
    'disassembly-stdout': (438594, 'ff67d340bb61e2b7b22f202cbfd23ca56ab7dfe9a152b9bf581241a81dbac7b5'),
    'disassembly-command.json': (2225, '3bbfee0a717d630de1cd5dbe895b8ec619e6ef5a8ed49f39b427b0fa805a2666'),
    'disassembly-result.json': (305, '5ba3f85a8bca749846d1d60b8dddb19dfedca5da3e111ac82aad7a095514fafb'),
    'emitted/artifact.hsaco': (53560, '4885204c8d510122588549107f42d2bc6f180f48fbc4eddd3bb1260e8d6629c5'),
}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def kernel_evidence(reader, request, capture, directory):
    pins = {name: capture.pin(KERNEL_ROOT / name, *extent) for name, extent in KERNEL_PINS.items()}
    bodies = {name: reader.read(row, directory / name) for name, row in pins.items()}
    receipt = capture.parse(bodies['complete.json'])
    require(receipt['probe_completed'] is True and receipt['error'] is None
            and receipt['postcheck_error'] is None, 'completed historical lowering')
    for name in ('extracted/module.ll', 'emitted/artifact.hsaco'):
        require(receipt['artifacts'][name] == pins[name], 'historical lowering artifact join')
    image = request['layer']['prefix_tiles_image']
    require((image['bytes'], capture.wire_sha(image['sha256'])) == KERNEL_PINS['emitted/artifact.hsaco'],
            'captured native prefix is the reviewed historical image')
    matches = [row for row in receipt['commands'] if row['name'] == 'disassembly']
    require(len(matches) == 1 and all(matches[0][kind] == pins[name] for kind, name in
            (('command', 'disassembly-command.json'), ('result', 'disassembly-result.json'),
             ('stdout', 'disassembly-stdout'))), 'retained actual disassembly records')
    command = capture.parse(bodies['disassembly-command.json'])
    result = capture.parse(bodies['disassembly-result.json'])
    require(command['argv'][1:] == ['--disassemble', '--mcpu=gfx950', pins['emitted/artifact.hsaco']['path']]
            and result['exit_code'] == 0 and result['group_absent'] is True and result['reason'] is None
            and result['stdout_sha256'] == pins['disassembly-stdout']['sha256'], 'natural image disassembly')
    return dict(inputs=pins, image_equals_captured_prefix=True,
                llvm_multiply_add_lines=[3234, 3236], llvm_xor_reduce_lines=[3260, 3297],
                llvm_strict_fp_attributes_line=6320,
                isa_product_range=['0x3cfc', '0x4390'], isa_accumulation_range=['0x4398', '0x46b4'],
                isa_reduce_range=['0x46c4', '0x475c'], source_and_codegen_review=True,
                exhaustive_machine_semantics_proof=False)


def geometry(header, index, capture):
    require(index['weight_map'].get(WEIGHT_KEY) == capture.SHARD
            and 'model.layers.0.self_attn.o_proj.bias' not in index['weight_map'], 'O shard identity and no bias')
    row = header[WEIGHT_KEY]
    require(set(row) == {'dtype', 'shape', 'data_offsets'} and row['dtype'] == 'BF16'
            and row['shape'] == [ROWS, 2 * RANK_WIDTH], 'O full matrix geometry')
    start, end = row['data_offsets']
    require(type(start) is int and type(end) is int and 0 <= start < end
            and end - start == ROWS * 2 * RANK_WIDTH * 2
            and 8 + capture.HEADER_EXTENT[0] + end <= capture.SHARD_EXTENT[0], 'O bounded source byte range')
    return 8 + capture.HEADER_EXTENT[0] + start


def uploaded_o(registration, uploads, capture):
    result = []
    for rank in (0, 1):
        layers = [row for row in registration['layers'] if row['rank'] == rank and row['layer'] == 0]
        require(len(layers) == 1, 'one registered layer-zero per rank')
        weights = [row['buffer'] for row in layers[0]['weights'] if row['kind'] == 'output_projection']
        require(len(weights) == 1, 'one registered O projection')
        buffer = weights[0]
        require(buffer['rank'] == rank and buffer['elements'] == ROWS * RANK_WIDTH
                and buffer['element_bytes'] == 2, 'rank-column-sharded O geometry')
        key = dict(kind='source', rank=rank, id=buffer['id'])
        matches = [row for row in uploads['uploads'] if row['key'] == key]
        require(len(matches) == 1 and matches[0]['bytes'] == ROWS * RANK_WIDTH * 2, 'O upload extent')
        result.append(dict(key=key, bytes=matches[0]['bytes'], sha256=capture.wire_sha(matches[0]['sha256'])))
    return result


def framework_o(reader, request, registration, selected, directory, capture):
    ref = reader.document(capture.pin(capture.REFERENCE / 'capture.json', *capture.REFERENCE_EXTENT),
                          directory / 'capture.json')
    require(ref['schema'] == 'ferric-p228-layer0-framework-capture-v1' and ref['status'] == 'PASS'
            and ref['input_token'] == 9112 and ref['position'] == 0
            and ref['genuine_framework_chain'] is True and ref['repeat_passes_byte_equal'] is True
            and ref['candidate_intermediate_inputs'] is False and ref['conditional_replay_performed'] is False,
            'genuine retained framework capture, not conditional model execution')
    for kind in ('model_id', 'bundle_id'):
        require(ref[kind] == capture.wire_sha(request['layer']['expected_' + kind])
                == capture.wire_sha(registration[kind]), 'native/reference model identity')
    require(request['layer']['source'] == str(capture.MODEL.parent) and len(ref['passes']) == 2,
            'source model and two genuine passes')
    passes = []
    for ordinal, report in enumerate(ref['passes'], 1):
        require(report['ordinal'] == ordinal and report['fresh_cache'] is True
                and report['input_token'] == 9112 and report['position'] == 0, 'reference pass scope')
        bodies = {}
        for name in ('attention-output', 'o-projection', 'embedding', 'first-residual'):
            row = report['stages'][name]
            original = row['pin']
            require(row['dtype'] == 'bfloat16' and row['shape'] == [1, 1, ROWS]
                    and original['bytes'] == 8192
                    and original['path'] == str(capture.REFERENCE / f'pass{ordinal}-{name}.bf16'),
                    'typed complete reference stage')
            bodies[name] = reader.read(original, directory / Path(original['path']).name)
        passes.append(bodies)
    require(passes[0] == passes[1], 'all eight selected framework payloads repeat exactly')
    require(all(len(selected[(rank, 'attention')]) == 4096 for rank in (0, 1))
            and selected[(0, 'attention')] + selected[(1, 'attention')] == passes[0]['attention-output'],
            'identical immediate O operands on both implementations')
    require(all(len(selected[(rank, 'output-partial')]) == 16384
                and len(selected[(rank, 'first-residual')]) == 8192 for rank in (0, 1)), 'native O and residual extents')
    return ref, passes[0]


def audit_rows(stream, absolute_start, selected, reference, uploads, exact, replay):
    units = tuple(None if word & 0x7f80 == 0x7f80 else exact.decode_units(word) for word in range(65536))
    attention = [struct.unpack('<2048H', selected[(rank, 'attention')]) for rank in (0, 1)]
    left_units = [[units[word] for word in words] for words in attention]
    require(all(value is not None for rank in left_units for value in rank), 'finite attention operands')
    partials = [struct.unpack('<4096I', selected[(rank, 'output-partial')]) for rank in (0, 1)]
    residuals = [struct.unpack('<4096H', selected[(rank, 'first-residual')]) for rank in (0, 1)]
    embedding = struct.unpack('<4096H', reference['embedding'])
    framework = struct.unpack('<4096H', reference['o-projection'])
    framework_residual = struct.unpack('<4096H', reference['first-residual'])
    hashes = [hashlib.sha256(), hashlib.sha256()]
    rows = []
    stream.seek(absolute_start)
    for index in range(ROWS):
        raw = stream.read(8192)
        require(len(raw) == 8192, 'full source O row')
        words = struct.unpack('<4096H', raw)
        reports = []
        for rank in (0, 1):
            hashes[rank].update(raw[rank * 4096:(rank + 1) * 4096])
            reports.append(replay.replay_rank(attention[rank], words[rank * 2048:(rank + 1) * 2048],
                                               left_units[rank], units))
        total = sum(row['exact_units'] for row in reports)
        ideal = exact.round_dot_units(total)
        observed = replay.combined(partials[0][index], partials[1][index], embedding[index])
        predicted = replay.combined(reports[0]['word'], reports[1]['word'], embedding[index])
        control = replay.narrow_f32(replay.add_f32(replay.bf16_f32(framework[index]),
                                                  replay.bf16_f32(embedding[index])))
        require(control == framework_residual[index], 'framework projection/residual boundary control')
        rank_rows = []
        for rank, model in enumerate(reports):
            actual = partials[rank][index]
            rank_rows.append(dict(rank=rank, native_f32=f'{actual:08x}', replay_f32=f"{model['word']:08x}",
                native_matches_fixed_order=actual == model['word'], exact_sum_units=str(model['exact_units']),
                absolute_sum_units=str(model['absolute_units']),
                native_distance_units=str(abs(replay.f32_units(actual) - model['exact_units'])),
                replay_distance_units=str(abs(replay.f32_units(model['word']) - model['exact_units'])),
                rounded_products=model['rounded_products'], subnormal_products=model['subnormal_products'],
                subnormal_sums=model['subnormal_sums']))
        native_projection, framework_projection = observed['projection'], framework[index]
        da, db = exact.distance_units(total, native_projection), exact.distance_units(total, framework_projection)
        rows.append(dict(row=index, shard_byte_offset=absolute_start + index * 8192, ranks=rank_rows,
            exact_sum_units=str(total), ideal_bf16=f'{ideal:04x}',
            native_projection_bf16=f'{native_projection:04x}', framework_projection_bf16=f'{framework_projection:04x}',
            replay_projection_bf16=f"{predicted['projection']:04x}", native_tp_sum_f32=f"{observed['sum_word']:08x}",
            replay_tp_sum_f32=f"{predicted['sum_word']:08x}",
            native_matches_ideal=native_projection == ideal, framework_matches_ideal=framework_projection == ideal,
            native_matches_framework=native_projection == framework_projection,
            native_distance_units=str(da), framework_distance_units=str(db),
            closer_to_exact='equal' if da == db else ('native' if da < db else 'framework'),
            differing_projection_midpoint=None if native_projection == framework_projection else
                replay.midpoint(total, native_projection, framework_projection),
            embedding_bf16=f'{embedding[index]:04x}',
            native_residual_bf16=[f'{residuals[rank][index]:04x}' for rank in (0, 1)],
            conditional_residual_bf16=f"{observed['residual']:04x}",
            replay_residual_bf16=f"{predicted['residual']:04x}",
            framework_residual_bf16=f'{framework_residual[index]:04x}',
            native_residual_matches_conditional_boundary=[residuals[rank][index] == observed['residual'] for rank in (0, 1)],
            native_residual_matches_framework=[residuals[rank][index] == framework_residual[index] for rank in (0, 1)]))
    slices = []
    for rank in (0, 1):
        require(hashes[rank].hexdigest() == uploads[rank]['sha256'], 'all deinterleaved O weight columns equal actual upload')
        slices.append(dict(rank=rank, model_key=WEIGHT_KEY, source_column_start=rank * RANK_WIDTH,
                           source_row_stride_bytes=8192, **uploads[rank]))
    return rows, slices


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('label')
    parser.add_argument('source_sha256')
    parser.add_argument('--reference-directory', default=str(E / 'layer0-framework-capture-v228-v1'))
    parser.add_argument('--kernel-directory', default=str(KERNEL_ROOT))
    args = parser.parse_args()
    require(re.fullmatch(r'o-projection-exact-replay-v228-v[1-9][0-9]*', args.label)
            and re.fullmatch('[0-9a-f]{64}', args.source_sha256), 'fresh versioned label and actual source hash')
    require(SOURCE_PINS is not None and not sys.flags.optimize and sys.dont_write_bytecode
            and 'PYTHONOPTIMIZE' not in os.environ, 'bound ordinary Python -B')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
            and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
            and all(os.environ.get(name) == '' for name in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')),
            'CPU-only bounded MI350 environment')
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_CPU, 600),
                      (resource.RLIMIT_FSIZE, 16 << 20), (resource.RLIMIT_CORE, 0)):
        hard = resource.getrlimit(kind)[1]
        resource.setrlimit(kind, (min(cap, hard) if hard != resource.RLIM_INFINITY else cap,) * 2)
    signal.alarm(900)
    out = E / args.label
    require(not os.path.lexists(out), 'fresh output directory')
    folder = Path(__file__).resolve().parent
    for name, expected in SOURCE_PINS.items():
        raw = (folder / name).read_bytes()
        require(len(raw) == expected['bytes'] and hashlib.sha256(raw).hexdigest() == expected['sha256'], 'source pin before loading')
    capture = load('o_capture_inputs', folder / 'capture_inputs.py')
    reader = capture.Reader()
    reader.read(capture.pin(folder / 'replay.py', (folder / 'replay.py').stat().st_size, args.source_sha256))
    for name, expected in SOURCE_PINS.items():
        reader.read(dict(path=str(folder / name), **expected))
    exact = load('exact_bf16', folder / 'exact_bf16.py')
    arithmetic = load('fp32_replay', folder / 'fp32_replay.py')
    _, request, registration, uploads, _, selected = capture.selected_capture(reader)
    codegen = kernel_evidence(reader, request, capture, Path(args.kernel_directory))
    ref, reference = framework_o(reader, request, registration, selected, Path(args.reference_directory), capture)
    uploaded = uploaded_o(registration, uploads, capture)
    for name, expected in ((capture.SHARD, capture.SHARD_EXTENT), ('model.safetensors.index.json', capture.INDEX_EXTENT)):
        require((ref['model_sources'][name]['bytes'], ref['model_sources'][name]['sha256']) == expected,
                'reference original model identity')
    index = reader.document(capture.pin(capture.MODEL / 'model.safetensors.index.json', *capture.INDEX_EXTENT))
    config_pin = ref['model_sources']['config.json']
    config = reader.document(capture.pin(capture.MODEL / 'config.json', config_pin['bytes'], config_pin['sha256']))
    require(config['hidden_size'] == ROWS and config['num_attention_heads'] == 32
            and config['num_key_value_heads'] == 8 and config['head_dim'] == 128
            and config['attention_bias'] is False, 'original model geometry')
    fd, before = capture.open_input(capture.MODEL / capture.SHARD, capture.SHARD_EXTENT[0])
    try:
        with os.fdopen(fd, 'rb', closefd=False) as stream:
            capture.hash_shard(stream, before)
            stream.seek(0)
            require(struct.unpack('<Q', stream.read(8))[0] == capture.HEADER_EXTENT[0], 'original header length')
            header_raw = stream.read(capture.HEADER_EXTENT[0])
            require(capture.digest(header_raw) == capture.HEADER_EXTENT[1], 'original header hash')
            start = geometry(capture.parse(header_raw), index, capture)
            rows, slices = audit_rows(stream, start, selected, reference, uploaded, exact, arithmetic)
            capture.hash_shard(stream, before)
    finally:
        os.close(fd)
    reader.postcheck()
    counts = dict(rows=len(rows), rank_partials=2 * len(rows),
        native_partials_match_fixed_order=sum(rank['native_matches_fixed_order'] for row in rows for rank in row['ranks']),
        native_projection_matches_ideal=sum(row['native_matches_ideal'] for row in rows),
        framework_projection_matches_ideal=sum(row['framework_matches_ideal'] for row in rows),
        native_projection_matches_framework=sum(row['native_matches_framework'] for row in rows),
        native_residual_matches_conditional_boundary=sum(sum(row['native_residual_matches_conditional_boundary']) for row in rows),
        native_residual_matches_framework=sum(sum(row['native_residual_matches_framework']) for row in rows),
        rounded_products=sum(rank['rounded_products'] for row in rows for rank in row['ranks']),
        subnormal_products=sum(rank['subnormal_products'] for row in rows for rank in row['ranks']),
        subnormal_sums=sum(rank['subnormal_sums'] for row in rows for rank in row['ranks']))
    require(counts['rows'] == 4096 and counts['rank_partials'] == 8192, 'complete all-row diagnostic')
    result = dict(schema='ferric-p228-o-projection-exact-replay-v1', completed=True,
        layer=0, position=0, input_token=9112, counts=counts, rows=rows,
        disagreement_rows=[row['row'] for row in rows if not row['native_matches_framework']],
        residual_disagreement_rows=[row['row'] for row in rows if not all(row['native_residual_matches_framework'])],
        exact_sum_scale_power_of_two=-266, distance_scale_power_of_two=-266,
        fixed_order=dict(rank_terms=2048, lanes=64, terms_per_lane=32, xor_masks=[1, 2, 4, 8, 16, 32],
                         multiplication='RNE FP32', addition='RNE FP32, gradual underflow'),
        kernel_evidence=codegen,
        projection_provenance='derived from two directly captured native FP32 partials',
        residual_operand_provenance='conditional on genuine framework embedding; native input was not captured',
        native_materialized_projection_directly_captured=False, native_residual_input_directly_captured=False,
        model_shard=capture.pin(capture.MODEL / capture.SHARD, *capture.SHARD_EXTENT),
        model_shard_complete_hash_passes=2, weight_upload_slices=slices, consumed=reader.consumed,
        source_and_input_postchecks_passed=True, immediate_attention_inputs_byte_equal=True,
        framework_repeated_selected_payloads_equal=True, observed_historical_layer0_only=True,
        current_guarded_internal_stages_observed=False, framework_accumulator_order_proven=False,
        gpu_execution=False, model_loaded=False, tests_executed=False, numerical_acceptance=False,
        semantic_bug_proven=False, full_model_correctness=False, production_authority=False,
        performance_claim=False, sustained_2048_256=False, acceptance_threshold=None)
    raw = (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode('ascii')
    require(len(raw) <= 16 << 20, 'bounded all-row JSON result')
    out.mkdir(mode=0o700)
    with (out / 'complete.json').open('xb') as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    expected = capture.pin(out / 'complete.json', len(raw), capture.digest(raw))
    capture.read_body(expected, out / 'complete.json')
    print(json.dumps(dict(complete=expected, counts=counts)), flush=True)


if __name__ == '__main__':
    main()
