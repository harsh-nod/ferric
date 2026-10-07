"""Retain original selected Down diagnostic bodies; never evaluate arithmetic or model code."""
import ast
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import struct
import sys
import tarfile
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-position5-down-dot-v228-v2'
INPUT_ROOT = ROOT / 'inputs'
MODEL = Path('/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target')
DEST = Path('/home/harsh/ferric-p227-integration/qualification/guarded-mlp-readiness40-causal-layer0-v1/position5-down-dot-v2')
ARCHIVE = E / 'guarded-mlp-position5-down-dot-evidence-v228-v2.tar.gz'
SOURCE_MANIFEST = dict(bytes=24838, sha256='8fed6635d3fcdbafb4972183123f2f34677c01ba7376680e92ac74b1a59eba54')
SOURCES = {'head.py', 'qkv.py', 'exact_bf16.py', 'fp32_replay.py', 'capture.py',
    'r2.py', 'position5.py', 'gateup.py', 'down.py', 'test_fp32_replay.py', 'test_r2.py',
    'test_position5.py', 'test_gateup.py', 'test_down.py', 'arithmetic.rs', 'kernels.rs', 'run.py', 'README.md'}
OUTPUT_ORDER = ('tests.stderr', 'selected-weight-halves.bf16')
ROWS = (0, 219, 955, 1352, 1408, 1953, 2040, 2070, 2208, 2210, 2328, 2706, 2850, 3704, 4017)
PRODUCT_DIFFERENCES = ((0, 553, 0xbb2b, 0xbb29), (0, 2575, 0x3864, 0x3863),
    (0, 2603, 0x39ca, 0x39c9), (0, 2840, 0x3746, 0x3747),
    (1, 1719, 0xb5d1, 0xb5d2), (1, 2063, 0x39dc, 0x39dd), (1, 5310, 0x35c9, 0x35ca))
MAX_FILE, MAX_TOTAL = 2 << 20, 16 << 20
DEADLINE = float('inf')


def require(ok, message):
    if not ok:
        raise ValueError(message)


def guard():
    remaining = DEADLINE - time.monotonic()
    require(remaining > 0, '120-second retention deadline')
    signal.setitimer(signal.ITIMER_REAL, remaining)


def parse(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    require(type(row) is dict and type(row['bytes']) is int and 0 <= row['bytes'] <= 2 << 30
            and type(row['sha256']) is str and re.fullmatch('[0-9a-f]{64}', row['sha256']), 'data/metadata pin')
    return {key: row[key] for key in ('bytes', 'sha256')}


def ordinary(name):
    return type(name) is str and Path(name).as_posix() == name and not Path(name).is_absolute() \
        and '..' not in Path(name).parts and name not in ('', '.')


def read(path, cap=MAX_FILE):
    guard()
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical ordinary body')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_uid, s.st_gid,
                       s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink >= 1
                and 0 <= before.st_size <= cap, 'bounded regular body')
        raw = stream.read(cap + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(raw) == before.st_size,
            'body changed while reading')
    guard()
    return raw


def declarations(raw):
    # Interpret authenticated data literals, never import or execute run.py.
    def literal(node):
        if isinstance(node, ast.Dict):
            keys = [literal(k) for k in node.keys]
            require(len(set(keys)) == len(keys), 'duplicate literal key')
            return dict(zip(keys, (literal(v) for v in node.values)))
        if isinstance(node, ast.Call):
            require(isinstance(node.func, ast.Name) and node.func.id == 'dict' and not node.args
                    and all(k.arg is not None for k in node.keywords), 'closed dict constructor only')
            require(len({k.arg for k in node.keywords}) == len(node.keywords), 'duplicate dict keyword')
            return {k.arg: literal(k.value) for k in node.keywords}
        return ast.literal_eval(node)
    wanted, values = {'INPUTS', 'TEST_NAMES'}, {}
    for node in ast.parse(raw).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            name = node.targets[0].id
            if name in wanted:
                require(name not in values, 'duplicate source declaration')
                values[name] = literal(node.value)
    updates = [node.value for node in ast.parse(raw).body if isinstance(node, ast.Expr)
               and isinstance(node.value, ast.Call) and isinstance(node.value.func, ast.Attribute)
               and isinstance(node.value.func.value, ast.Name) and node.value.func.value.id == 'INPUTS'
               and node.value.func.attr == 'update']
    require(len(updates) == 1 and len(updates[0].args) == 1 and not updates[0].keywords,
            'single closed original INPUTS update')
    extra = literal(updates[0].args[0])
    require(not set(extra) & set(values['INPUTS']), 'no replaced input declarations')
    values['INPUTS'].update(extra)
    require(set(values) == wanted and len(values['INPUTS']) == 13 and len(values['TEST_NAMES']) == 48
            and len(set(values['TEST_NAMES'])) == 48, 'closed authenticated declaration roster')
    return values


def uint(value, maximum):
    require(type(value) is int and 0 <= value <= maximum, 'strict bounded integer')
    return value


def envelope(raw, magic):
    require(raw[:8] == magic and len(raw) >= 16, 'original sidecar framing')
    header_bytes, payload_bytes = struct.unpack_from('<II', raw, 8)
    require(2 <= header_bytes <= 128 << 10 and 16 + header_bytes + payload_bytes == len(raw),
            'exact original sidecar extent')
    return parse(raw[16:16 + header_bytes]), raw[16 + header_bytes:]


def selected_parts(bodies, diagnostic):
    nh, np = envelope(bodies['inputs/native-sidecar.bin'], b'FCAP061\0')
    fh, fp = envelope(bodies['inputs/reference-sidecar1.bin'], b'FREF061\0')
    rh, rp = envelope(bodies['inputs/reference-sidecar2.bin'], b'FREF061\0')
    require(fp == rp and fh['captures'] == rh['captures'] and fh['ordinal'] == 1 and rh['ordinal'] == 2,
            'original repeated framework payloads')
    nc, fc = nh['captures'][5], fh['captures'][5]
    require(nc['position'] == fc['position'] == 5 and nc['generation'] == 6 and nc['layer'] == 0
            and fc['input_token'] == 271, 'original position-five capture scope')
    offset = sum(uint(c['payload_bytes'], len(np)) for c in nh['captures'][:5])
    np = np[offset:offset + uint(nc['payload_bytes'], len(np))]
    parts, selected = [], {}
    for side, records, payload in (('native', nc['parts'], np), ('framework', fc['parts'], fp)):
        for part in records:
            role = part['role'] if side == 'native' else part['name']
            if role not in (('first_residual', 'down_partial', 'final_hidden') if side == 'native'
                            else ('down-projection', 'mlp-output', 'first-residual', 'layer0-hidden')):
                continue
            start, size = uint(part['offset'], len(payload)), uint(part['bytes'], len(payload))
            require(size > 0 and start + size <= len(payload), 'original selected slice bound')
            raw = payload[start:start + size]
            row = dict(side=side, role=role, offset=start, **pin(raw))
            if side == 'native':
                row['rank'] = uint(part['rank'], 1)
                key = side, row['rank'], role
            else:
                key = side, role
            require(key not in selected, 'unique selected original')
            parts.append(row); selected[key] = raw
    require(parts == diagnostic['selected_parts'] and len(parts) == 10,
            'all ten reported selected pins match original sidecars')
    require(diagnostic['original_sidecars'] == dict(native=pin(bodies['inputs/native-sidecar.bin']),
            reference=pin(bodies['inputs/reference-sidecar1.bin']),
            repeat=pin(bodies['inputs/reference-sidecar2.bin'])), 'reported whole-sidecar pins')
    return selected


def products(bodies):
    nh, np = envelope(bodies['inputs/native-sidecar.bin'], b'FCAP061\0')
    fh, fp = envelope(bodies['inputs/reference-sidecar1.bin'], b'FREF061\0')
    nc, fc = nh['captures'][5], fh['captures'][5]
    base = sum(c['payload_bytes'] for c in nh['captures'][:5])
    np = np[base:base + nc['payload_bytes']]
    values, pins = {}, []
    for rank in (0, 1):
        parts = [p for p in nc['parts'] if p['role'] == 'activation' and p['rank'] == rank]
        require(len(parts) == 1, 'one original product per rank')
        part = parts[0]
        raw = np[part['offset']:part['offset'] + part['bytes']]
        require(len(raw) == 12288 and pin(raw)['sha256'] == bytes(part['sha256']).hex(),
                'native product original pin')
        values['native', rank] = raw
        pins.append(dict(side='native', rank=rank, role='activation', **pin(raw)))
    parts = [p for p in fc['parts'] if p['name'] == 'product']
    require(len(parts) == 1, 'one framework product')
    part = parts[0]
    raw = fp[part['offset']:part['offset'] + part['bytes']]
    require(len(raw) == 24576 and pin(raw)['sha256'] == part['sha256'], 'framework product original pin')
    pins.append(dict(side='framework', role='product', **pin(raw)))
    for rank in (0, 1):
        half = raw[rank * 12288:(rank + 1) * 12288]
        values['framework', rank] = half
        pins.append(dict(side='framework', rank=rank, role='product-column-half', **pin(half)))
    return values, pins


def decimal(value):
    require(type(value) is str and len(value) <= 256 and re.fullmatch(r'0|-?[1-9][0-9]*', value),
            'bounded exact integer string')
    return int(value)


def verify_diagnostic(bodies, result, source):
    diagnostic = result['diagnostic']
    require(diagnostic['schema'] == 'ferric-position5-selected-down-exact-error-v1'
            and diagnostic['position'] == 5 and diagnostic['input_token'] == 271 and diagnostic['layer'] == 0
            and diagnostic['selected_differing_rows'] == list(ROWS[1:]) and diagnostic['fixed_control_row'] == 0
            and diagnostic['exact_rank_dot_count'] == 60 and diagnostic['terms_per_rank_dot'] == 6144
            and diagnostic['full_dot_terms'] == 12288 and diagnostic['product_difference_count'] == 7,
            'fixed selected-row diagnostic scope')
    require(diagnostic['exact_unit'] == '2^-266' and diagnostic['acceptance_threshold'] is None
            and all(diagnostic[k] is True for k in ('exact_integers_encoded_as_decimal_strings',
                'framework_rank_dots_are_exact_diagnostic_partitions_not_observed_accumulators',
                'native_projection_is_original_r2_derived_not_direct_capture', 'selected_rows_only'))
            and all(diagnostic[k] is False for k in ('native_fp32_accumulation_tree_emulated',
                'framework_accumulation_emulated', 'current_machine_instruction_order_independently_proven',
                'input_substitution_in_model', 'r2_full_vector_reexecution', 'all_layer_down_acceptance',
                'gate_up_or_silu_replayed', 'mismatch_explains_position5_argmax', 'independent_model_reference',
                'numerical_acceptance', 'full_model_acceptance', 'policy_changed', 'semantic_bug_claimed',
                'performance_claim', 'gpu_execution', 'model_execution')), 'unchanged diagnostic nonclaims')
    original = parse(bodies['inputs/r2-terminal.json'])
    require(original['passed'] is True and original['diagnostic']['all_observed_boundaries_exact'] is True,
            'original already-qualified R2 success')
    selected = selected_parts(bodies, original['diagnostic'])
    aliases = dict(terminal='r2-terminal.json', replay='r2-replay.json',
                   derived_down='r2-derived-down.bf16', derived_sum='r2-derived-ordered-sum.f32')
    require(diagnostic['original_r2_reused'] ==
            {k: pin(bodies['inputs/' + n]) for k, n in aliases.items()}, 'original R2 reuse pins')
    vectors, product_pins = products(bodies)
    require(diagnostic['products'] == product_pins, 'reported own-input product pins')
    words = {k: struct.unpack('<6144H', v) for k, v in vectors.items()}
    differences = [(rank, i, a, b) for rank in (0, 1)
                   for i, (a, b) in enumerate(zip(words['native', rank], words['framework', rank])) if a != b]
    require(differences == list(PRODUCT_DIFFERENCES), 'seven unchanged original product coordinates')
    replay = parse(bodies['inputs/r2-replay.json'])['native']['rows']
    require(len(replay) == 4096 and [r['row'] for r in replay] == list(range(4096)), 'original complete R2 ledger')
    framework_down = struct.unpack('<4096H', selected['framework', 'down-projection'])
    framework_final = struct.unpack('<4096H', selected['framework', 'layer0-hidden'])
    rows = diagnostic['rows']
    require(type(rows) is list and len(rows) == 15 and [r['row'] for r in rows] == list(ROWS),
            'all fourteen differences plus matching row-zero control')
    retained = diagnostic.get('retained_weight_halves')
    weights = bodies.get('output/selected-weight-halves.bf16')
    if retained is not None:
        require(weights is not None and len(weights) == 368640 and retained == pin(weights), 'original retained weight halves')
    elif weights is not None:
        require(len(weights) == 368640, 'stable complete failed-save weight body')
    for index, row in enumerate(rows):
        prior = replay[row['row']]
        require(row['fixed_matching_control'] is (row['row'] == 0)
                and row['unit'] == '2^-266'
                and row['native_partials_f32'] == prior['down_partial_f32']
                and row['original_sum_f32'] == prior['ordered_sum_f32']
                and row['native_projection_bf16'] == prior['derived_down_bf16']
                and row['common_residual_bf16'] == prior['first_residual_bf16'][0]
                and row['native_final_bf16'] == prior['observed_final_bf16'][0]
                and row['framework_projection_bf16'] == '%04x' % framework_down[row['row']]
                and row['framework_final_bf16'] == '%04x' % framework_final[row['row']],
                'selected original operand/output encoding joins without replay')
        require(row['counterfactual_not_model_execution'] is True
                and row['exact_decomposition_reconciled'] is True, 'reported counterfactual/error scope')
        native = [decimal(x) for x in row['exact_native_rank_dots']]
        framework = [decimal(x) for x in row['exact_framework_rank_dots']]
        require(len(native) == len(framework) == 2 and sum(native) == decimal(row['exact_native_full_dot'])
                and sum(framework) == decimal(row['exact_framework_full_dot']), 'reported integer rank/full totals')
        terms = {k: decimal(v) for k, v in row['terms'].items()}
        require(set(terms) == {'upstream_own_product_delta', 'native_rank0_accumulation_error',
                'native_rank1_accumulation_error', 'native_tp_sum_rounding', 'native_projection_bf16_rounding',
                'framework_projection_error', 'observed_projection_difference', 'native_residual_boundary_error',
                'framework_residual_boundary_error', 'observed_final_hidden_difference'}, 'closed reported error terms')
        rhs = (terms['upstream_own_product_delta'] + terms['native_rank0_accumulation_error']
               + terms['native_rank1_accumulation_error'] + terms['native_tp_sum_rounding']
               + terms['native_projection_bf16_rounding'] - terms['framework_projection_error'])
        require(terms['upstream_own_product_delta'] == sum(native) - sum(framework)
                and terms['observed_projection_difference'] == rhs
                and terms['observed_final_hidden_difference'] ==
                    rhs + terms['native_residual_boundary_error'] - terms['framework_residual_boundary_error'],
                'reported integer error totals reconcile, not independently recomputed dot terms')
        sparse = row['sparse_upstream_delta']
        require(len(sparse) == 7, 'all seven reported sparse terms')
        for term, (rank, local, a, b) in zip(sparse, PRODUCT_DIFFERENCES):
            require(term['rank'] == rank and term['local_column'] == local
                    and term['global_column'] == rank * 6144 + local
                    and term['native_product_bf16'] == '%04x' % a
                    and term['framework_product_bf16'] == '%04x' % b, 'original sparse input coordinates')
            decimal(term['delta_units_2_pow_minus266'])
            if weights is not None:
                offset = (index * 2 + rank) * 12288
                require(term['weight_bf16'] == '%04x' % struct.unpack_from('<H', weights, offset + 2 * local)[0],
                        'sparse coefficient retained original byte')
        require(all(sum(decimal(t['delta_units_2_pow_minus266']) for t in sparse if t['rank'] == rank)
                    == native[rank] - framework[rank] for rank in (0, 1)), 'reported sparse/rank totals')
        require(len(row['weight_halves']) == 2, 'two selected weight pins')
        if weights is not None:
            require(row['weight_halves'] == [pin(weights[(index * 2 + rank) * 12288:
                    (index * 2 + rank + 1) * 12288]) for rank in (0, 1)], 'every selected original half pin')
    model = result['model_shard']
    expected = {k: source['original_model_shard'][k] for k in ('path', 'bytes', 'sha256')}
    require(all(model[k] == v for k, v in expected.items())
            and model['current_native_upload_manifest'] == pin(bodies['inputs/uploads.json'])
            and len(model['stable_stat']) == 9 and model['stable_stat'][6] == expected['bytes']
            and model['stable_stat'][3] >= 1 and stat.S_ISREG(model['stable_stat'][2]),
            'observed full-shard metadata only, no retained shard rehash')
    start = uint(model['tensor_file_offset'], expected['bytes'])
    require(start >= 8 and start + 4096 * 24576 <= expected['bytes'], 'observed tensor extent')
    require(len(model['rows']) == 30, 'thirty extracted original half metadata rows')
    for i, part in enumerate(model['rows']):
        row, rank = ROWS[i // 2], i % 2
        require(part == dict(position=5, layer=0, rank=rank, checkpoint_row=row,
                checkpoint_first_column=rank * 6144, uploaded_partition_row=row,
                file_offset=start + row * 24576 + rank * 12288, retained_offset=i * 12288,
                **rows[i // 2]['weight_halves'][rank]), 'original retained half location and identity')
    uploads = parse(bodies['inputs/uploads.json'])['uploads']
    require(len(model['uploaded_down_partitions']) == 2, 'two original full partitions')
    for rank, part in enumerate(model['uploaded_down_partitions']):
        key = dict(kind='source', rank=rank, id=188 + rank)
        matches = [p for p in uploads if p['key'] == key]
        require(len(matches) == 1 and set(matches[0]) == {'key', 'bytes', 'sha256'}, 'one original upload')
        octets = matches[0]['sha256']
        require(type(octets) is list and len(octets) == 32
                and all(type(x) is int and 0 <= x <= 255 for x in octets), 'original upload digest octets')
        upload = dict(key=key, bytes=matches[0]['bytes'], sha256=bytes(octets).hex())
        require(part['rank'] == rank and part['upload'] == upload
                and compact(part) == compact(upload) and part['bytes'] == 50331648
                and part['checkpoint_tensor'] == 'model.layers.0.mlp.down_proj.weight'
                and part['first_checkpoint_column'] == rank * 6144 and part['rows'] == 4096
                and part['columns'] == 6144 and part['row_stride_bytes'] == 24576,
                'original upload commitments joined to reported checkpoint partition hashes')
    return retained is not None


def verify(bodies, terminal_name, terminal_sha):
    guard()
    require(terminal_name in ('complete.json', 'failed.json') and re.fullmatch('[0-9a-f]{64}', terminal_sha),
            'observed terminal name/SHA')
    require(pin(bodies['source-manifest.json']) == SOURCE_MANIFEST, 'frozen original source manifest')
    source = parse(bodies['source-manifest.json'])
    require(source['schema'] == 'ferric-position5-selected-down-exact-source-v1'
            and set(source['files']) == SOURCES, 'eighteen original source bodies')
    require(all(pin(bodies[name]) == expected for name, expected in source['files'].items()), 'source byte pins')
    constants = declarations(bodies['run.py'])
    inputs = constants['INPUTS']
    require(source['test_names'] == constants['TEST_NAMES']
            and {n: compact(p) for n, p in source['input_originals'].items()} == inputs, 'held test/input rosters')
    require(all(pin(bodies['inputs/' + name]) == expected for name, expected in inputs.items()),
            'thirteen original immutable data pins')
    fixed = SOURCES | {'source-manifest.json'} | {'inputs/' + name for name in inputs}
    outputs = {n.removeprefix('output/') for n in bodies if n.startswith('output/')}
    prefix = outputs - {terminal_name}
    require(prefix == set(OUTPUT_ORDER[:len(prefix)]) and terminal_name in outputs
            and set(bodies) == fixed | {'output/' + n for n in outputs}
            and 33 <= len(bodies) <= 35 and sum(map(len, bodies.values())) <= MAX_TOTAL,
            'closed source/input set and actual stable output prefix')
    raw = bodies['output/' + terminal_name]
    require(pin(raw)['sha256'] == terminal_sha, 'observed original terminal hash')
    result = parse(raw)
    require(result['schema'] == 'ferric-position5-selected-down-exact-cpu-v1' and type(result['passed']) is bool
            and result['passed'] is (terminal_name == 'complete.json'), 'original status/name preserved')
    require(all(result[k] is False for k in ('full_capsule_revalidated', 'gpu_execution', 'model_execution',
            'native_rerun', 'fixed_accumulation_emulation', 'semantic_bug_claimed', 'numerical_acceptance',
            'full_model_acceptance', 'performance_claim', 'production_authority'))
            and result['reused_prior_native_reference_comparison_admission'] is True
            and result['current_native_down_partition_hashes_rechecked'] is (result['model_shard'] is not None)
            and result['original_full_shard_rehashed'] is (result['model_shard'] is not None), 'original diagnostic scope')
    require(result['limits'] == dict(wall_seconds=240, cpu_seconds=180, address_space_bytes=512 << 20,
                                   affinity=[8, 9], nice=10), 'unchanged bounded CPU resource metadata')
    require(type(result['elapsed_seconds']) in (int, float) and 0 <= result['elapsed_seconds'] < 240,
            'actual bounded elapsed interval')
    expected = {str(ROOT / n): pin(bodies[n]) for n in fixed}
    expected[str(MODEL / 'model.safetensors.index.json')] = compact(source['original_model_index'])
    require(set(result['inputs']) <= set(expected), 'no unexpected input/readset authority')
    for path, row in result['inputs'].items():
        require(row == dict(path=path, **expected[path]), 'retained input/posthash metadata join')
    require(all(path in {str(ROOT / n) for n in SOURCES} and row == expected[path]
                for path, row in result['source_pins'].items()), 'reported source pins')
    tests = result['tests']
    if tests is not None:
        require(type(tests) is dict and tests['names'] == constants['TEST_NAMES']
                and tests['stderr'] == pin(bodies['output/tests.stderr']), 'original named-test stream pin')
        require(sum(uint(tests[k], 48) for k in ('passed', 'failures', 'errors')) == 48
                and uint(tests['skipped'], 48) <= tests['passed'], 'original observed test census')
    diagnostic_checked = False
    if result['diagnostic'] is not None:
        diagnostic_checked = verify_diagnostic(bodies, result, source)
    if result['passed']:
        require(result['error'] is None and result['postcheck_errors'] == []
                and set(result['inputs']) == set(expected)
                and result['source_pins'] == {str(ROOT / n): v for n, v in source['files'].items()},
                'clean complete source/input postchecks')
        require(prefix == set(OUTPUT_ORDER) and tests is not None
                and tests['passed'] == 48 and tests['failures'] == tests['errors'] == tests['skipped'] == 0
                and diagnostic_checked and result['model_shard'] is not None and result['registration_join'] is not None,
                'completed fixture/custody/diagnostic gates, not model acceptance')
        lines, rows = bodies['output/tests.stderr'].decode().splitlines(), []
        pattern = r'(test_\w+) \(((?:test_fp32_replay\.Fp32ReplayTests|test_r2\.R2Tests|test_position5\.Position5Tests|test_gateup\.GateUpTests|test_down\.DownTests))(?:\.(test_\w+))?\) \.\.\. ok'
        for line in lines:
            match = re.fullmatch(pattern, line)
            if match:
                require(match[3] in (None, match[1]), 'unittest repeated method mismatch')
                rows.append(match[2] + '.' + match[1])
        require(sorted(rows) == constants['TEST_NAMES'] and len(rows) == len(set(rows)) == 48
                and sum(bool(re.fullmatch(r'Ran 48 tests in [0-9.]+s', line)) for line in lines) == 1
                and lines.count('OK') == 1, 'exact original unittest outcomes')
    else:
        require(result['error'] is not None or result['postcheck_errors'], 'honest failed terminal')
    guard()
    return dict(original_passed=result['passed'], terminal=pin(raw), original_members=len(bodies),
                source_members=19, input_members=13, output_members=len(outputs),
                original_receipt_and_body_joins_verified=True, complete_output_joins_verified=diagnostic_checked,
                original_test_census=tests, arithmetic_reexecuted=False,
                reported_integer_totals_checked=diagnostic_checked, independent_dot_terms_recomputed=False,
                model_bodies_retained=False, checkpoint_bodies_rehashed=False, native_rerun=False,
                full_capsule_revalidated=False, numerical_acceptance=False,
                full_model_acceptance=False, performance_claim=False)

def tree(root):
    names = set()
    for parent, dirs, files in os.walk(root, followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(all(not (Path(parent) / n).is_symlink() for n in dirs), 'no directory aliases')
        names.update(str((Path(parent) / n).relative_to(root)) for n in files)
    return names


def export(terminal_name, terminal_sha):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'actual export host')
    require(ROOT.resolve(strict=True) == ROOT and INPUT_ROOT.resolve(strict=True) == INPUT_ROOT
            and not os.path.lexists(ARCHIVE), 'fresh evidence archive and original roots')
    names = tree(ROOT)
    require(33 <= len(names) <= 35 and all(ordinary(n) for n in names), 'bounded original source/output root')
    bodies = {n: read(ROOT / n) for n in names}
    constants = declarations(bodies['run.py'])
    require(tree(INPUT_ROOT) == set(constants['INPUTS']), 'exact thirteen original inputs')
    checks = verify(bodies, terminal_name, terminal_sha)
    bodies['retention-tool.py'] = read(Path(__file__).resolve())
    manifest = dict(schema='ferric-position5-selected-down-exact-evidence-v1', terminal_name=terminal_name,
        terminal_sha256=terminal_sha, checks=checks, files={n: pin(b) for n, b in sorted(bodies.items())})
    bodies['manifest.json'] = encoded(manifest)
    require(35 <= len(bodies) <= 37 and sum(map(len, bodies.values())) <= MAX_TOTAL,
            'bounded original members plus helper and manifest')
    require(tree(ROOT) == names and all(read(ROOT / n) == bodies[n] for n in names),
            'all original source/output posthashes')
    require(tree(INPUT_ROOT) == set(constants['INPUTS'])
            and all(read(INPUT_ROOT / n) == bodies['inputs/' + n] for n in constants['INPUTS']),
            'all original read-only input alias posthashes')
    require(read(Path(__file__).resolve()) == bodies['retention-tool.py'], 'retainer source posthash')
    guard()
    with ARCHIVE.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive:
            for name, raw in sorted(bodies.items()):
                guard()
                row = tarfile.TarInfo(name); row.size = len(raw); row.mode = 0o600
                archive.addfile(row, io.BytesIO(raw))
        stream.flush()
        os.fsync(stream.fileno())
    print(json.dumps(dict(archive=dict(path=str(ARCHIVE), **pin(read(ARCHIVE, MAX_TOTAL))), checks=checks)))


def retain(path, archive_sha, terminal_sha):
    require(re.fullmatch('[0-9a-f]{64}', archive_sha), 'observed archive SHA')
    raw = read(Path(path), MAX_TOTAL)
    require(pin(raw)['sha256'] == archive_sha, 'exact exported archive')
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        members = archive.getmembers()
        require(35 <= len(members) == len({m.name for m in members}) <= 37
                and all(m.isfile() and ordinary(m.name) and not m.pax_headers and 0 <= m.size <= MAX_FILE for m in members)
                and sum(m.size for m in members) <= MAX_TOTAL, 'closed bounded ordinary archive')
        bodies = {m.name: archive.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'complete member reads')
    manifest = parse(bodies['manifest.json'])
    require(manifest['schema'] == 'ferric-position5-selected-down-exact-evidence-v1'
            and manifest['terminal_sha256'] == terminal_sha
            and set(manifest['files']) == set(bodies) - {'manifest.json'}
            and all(pin(bodies[n]) == value for n, value in manifest['files'].items()), 'all retained member pins')
    require(bodies['retention-tool.py'] == read(Path(__file__).resolve()), 'exact export/retention helper')
    original = {n: body for n, body in bodies.items() if n not in ('manifest.json', 'retention-tool.py')}
    checks = verify(original, manifest['terminal_name'], terminal_sha)
    require(checks == manifest['checks'], 'same checked original claims')
    require(not os.path.lexists(DEST), 'fresh canonical retention destination')
    DEST.mkdir(parents=True, mode=0o700)
    for name, body in sorted(bodies.items()):
        guard()
        dest = DEST / name; dest.parent.mkdir(parents=True, exist_ok=True)
        with dest.open('xb') as stream:
            stream.write(body)
    require(tree(DEST) == set(bodies) and all(read(DEST / n) == body for n, body in bodies.items()),
            'verbatim copy posthashes')
    require(read(Path(path), MAX_TOTAL) == raw, 'archive transport posthash')
    print(json.dumps(dict(destination=str(DEST), members=len(bodies), checks=checks)))


def main():
    global DEADLINE
    require(__debug__ and sys.dont_write_bytecode, 'unoptimized python -B required')
    DEADLINE = time.monotonic() + 120
    def stop(number, _frame):
        raise TimeoutError('retention signal %d' % number)
    for number in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, stop)
    resource.setrlimit(resource.RLIMIT_AS, (256 << 20, 256 << 20))
    resource.setrlimit(resource.RLIMIT_CPU, (120, 120))
    resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_TOTAL, MAX_TOTAL))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    guard()
    if len(sys.argv) == 4 and sys.argv[1] == 'export':
        export(sys.argv[2], sys.argv[3])
    elif len(sys.argv) == 5 and sys.argv[1] == 'retain':
        retain(sys.argv[2], sys.argv[3], sys.argv[4])
    else:
        raise ValueError('evidence.py export TERMINAL_NAME SHA | retain ARCHIVE ARCHIVE_SHA TERMINAL_SHA')
    guard()
    signal.setitimer(signal.ITIMER_REAL, 0)


if __name__ == '__main__':
    main()
