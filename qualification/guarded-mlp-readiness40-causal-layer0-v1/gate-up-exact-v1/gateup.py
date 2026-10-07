"""Seven observed position-five Gate/Up differences, not a GEMM emulator."""
import hashlib
import json
import struct

import head as H

WIDTH = 4096
SHARD = 'model-00001-of-00005.safetensors'
TENSORS = {kind: 'model.layers.0.mlp.%s_proj.weight' % kind for kind in ('gate', 'up')}
ROWS = dict(gate=12288, up=12288)
PARTITION_ROWS = 6144
# Position, rank, projection, local output row, native word, framework word.
TARGETS = ((5, 0, 'gate', 553, 0xbd86, 0xbd85),
           (5, 0, 'up', 2575, 0xbadd, 0xbadc),
           (5, 0, 'up', 2603, 0x3cb2, 0x3cb1),
           (5, 0, 'up', 2840, 0xba49, 0xba4a),
           (5, 1, 'gate', 2063, 0x3c50, 0x3c51),
           (5, 1, 'up', 1719, 0xb846, 0xb847),
           (5, 1, 'up', 5310, 0xb8cb, 0xb8cc))


def normalized(row):
    H.require(type(row) is dict and type(row.get('bytes')) is int and row['bytes'] >= 0,
              'data pin extent')
    digest = row.get('sha256')
    if type(digest) is list:
        H.require(len(digest) == 32 and all(type(x) is int and 0 <= x <= 255 for x in digest),
                  'digest octets')
        digest = bytes(digest).hex()
    H.require(type(digest) is str and len(digest) == 64
              and all(c in '0123456789abcdef' for c in digest), 'SHA256')
    return dict(bytes=row['bytes'], sha256=digest)


def coordinates(rank, kind, local):
    H.require(type(rank) is int and rank in (0, 1) and kind in ROWS
              and type(local) is int and 0 <= local < ROWS[kind] // 2, 'rank/projection/local row')
    return rank * PARTITION_ROWS + local, local


def tensor_layout(header, index, header_bytes, file_bytes):
    H.require(type(header) is dict and type(index) is dict
              and type(index.get('weight_map')) is dict, 'original safetensors/index maps')
    H.require(type(header_bytes) is int and 2 <= header_bytes <= 1 << 20
              and type(file_bytes) is int and file_bytes > 8 + header_bytes, 'bounded shard header')
    spans = []
    for name, row in header.items():
        if name == '__metadata__':
            H.require(type(row) is dict and all(type(k) is str and type(v) is str
                      for k, v in row.items()), 'safetensors metadata')
            continue
        H.require(type(row) is dict and set(row) == {'dtype', 'shape', 'data_offsets'}
                  and row['dtype'] == 'BF16' and type(row['shape']) is list
                  and 1 <= len(row['shape']) <= 4
                  and all(type(n) is int and n > 0 for n in row['shape']), 'BF16 tensor geometry')
        extent = 2
        for n in row['shape']:
            extent *= n
        offsets = row['data_offsets']
        H.require(type(offsets) is list and len(offsets) == 2
                  and all(type(n) is int for n in offsets)
                  and 0 <= offsets[0] < offsets[1] <= file_bytes - 8 - header_bytes
                  and offsets[1] - offsets[0] == extent
                  and index['weight_map'].get(name) == SHARD, 'indexed original tensor extent')
        spans.append((offsets[0], offsets[1], name))
    spans.sort()
    H.require(spans and spans[0][0] == 0 and spans[-1][1] == file_bytes - 8 - header_bytes
              and all(a[1] == b[0] for a, b in zip(spans, spans[1:])),
              'complete nonoverlapping no-hole/no-trailer original shard')
    H.require({name for name, shard in index['weight_map'].items() if shard == SHARD}
              == {s[2] for s in spans}, 'entire indexed shard roster')
    result = {}
    for kind, name in TENSORS.items():
        H.require(name in header and header[name]['shape'] == [ROWS[kind], WIDTH],
                  'layer-zero original row-major Gate/Up')
        result[kind] = 8 + header_bytes + header[name]['data_offsets'][0]
    return result


def envelope(raw, magic):
    H.require(type(raw) is bytes and 16 <= len(raw) <= 2 << 20 and raw[:8] == magic,
              'bounded captured sidecar envelope')
    header_bytes, payload_bytes = struct.unpack_from('<II', raw, 8)
    H.require(2 <= header_bytes <= 128 << 10 and 16 + header_bytes + payload_bytes == len(raw),
              'complete sidecar extent')
    header = H.parse(raw[16:16 + header_bytes])
    payload = raw[16 + header_bytes:]
    expected = (dict(bytes=header['payload_bytes'], sha256=header['payload_sha256'])
                if magic == b'FCAP061\0' else header['payload'])
    H.require(H.pin(payload) == normalized(expected), 'entire sidecar payload hash')
    H.require(len(header['captures']) == 6
              and [c['position'] for c in header['captures']] == list(range(6)), 'six ordered positions')
    return header, payload


def extract(native_raw, reference_raw, repeat_raw, comparison):
    nh, np = envelope(native_raw, b'FCAP061\0')
    fh, fp = envelope(reference_raw, b'FREF061\0')
    rh, rp = envelope(repeat_raw, b'FREF061\0')
    H.require(fh['ordinal'] == 1 and rh['ordinal'] == 2 and fp == rp
              and fh['captures'] == rh['captures'], 'genuine second-pass stage repetition')
    rows = comparison['checks']['diagnostic']['comparable_rows']
    H.require(len(rows) == 204 and len({r['id'] for r in rows}) == 204, 'authenticated metric roster')
    recorded = {r['id']: r for r in rows}
    vectors, found, base = {}, [], 0
    for nc, fc in zip(nh['captures'], fh['captures']):
        position = nc['position']
        H.require(type(nc['payload_bytes']) is int and 0 <= nc['payload_bytes'] <= len(np) - base,
                  'bounded native capture extent')
        body = np[base:base + nc['payload_bytes']]
        base += len(body)
        H.require(H.pin(body) == normalized(dict(bytes=nc['payload_bytes'], sha256=nc['payload_sha256']))
                  and nc['generation'] == position + 1 and nc['layer'] == 0, 'native capture body/scope')
        nr, fr = {}, {}
        for part in nc['parts']:
            key = (part['rank'], part['role'])
            H.require(key not in nr and type(part['offset']) is int and type(part['bytes']) is int
                      and 0 <= part['offset'] <= len(body)
                      and 0 <= part['bytes'] <= len(body) - part['offset'], 'unique bounded native part')
            value = body[part['offset']:part['offset'] + part['bytes']]
            H.require(H.pin(value) == normalized(part), 'native captured part pin')
            nr[key] = value
        for part in fc['parts']:
            H.require(part['name'] not in fr and type(part['offset']) is int and type(part['bytes']) is int
                      and 0 <= part['offset'] <= len(fp)
                      and 0 <= part['bytes'] <= len(fp) - part['offset'], 'unique bounded framework part')
            value = fp[part['offset']:part['offset'] + part['bytes']]
            H.require(H.pin(value) == normalized(part), 'framework captured part pin')
            fr[part['name']] = value
        if position != 5:
            continue
        for rank in (0, 1):
            vector = nr[rank, 'post_normalized']
            H.words(vector, WIDTH)
            H.require(vector == fr['post-norm'] == fr['gate-input'] == fr['up-input'],
                      'same full captured postnorm and actual Gate/Up inputs')
            post = recorded['5:%d:post_normalized' % rank]
            H.require(post['different_words'] == 0 and post['native'] == H.pin(vector)
                      and post['reference'] == H.pin(vector), 'actual postnorm comparison pin')
            for kind in ('gate', 'up'):
                native = nr[rank, kind]
                H.words(fr[kind], ROWS[kind])
                reference = fr[kind][2 * rank * PARTITION_ROWS:2 * (rank + 1) * PARTITION_ROWS]
                native_words, ref_words = H.words(native, PARTITION_ROWS), H.words(reference, PARTITION_ROWS)
                row = recorded['5:%d:%s' % (rank, kind)]
                H.require(row['same_captured_inputs'] is True and row['native'] == H.pin(native)
                          and row['reference'] == H.pin(reference), 'actual comparison part/input join')
                differences = 0
                for local, (a, b) in enumerate(zip(native_words, ref_words)):
                    if a != b:
                        found.append((position, rank, kind, local, a, b))
                        vectors[position, rank, kind, local] = vector
                        differences += 1
                H.require(differences == row['different_words'], 'complete position-five Gate/Up difference census')
    H.require(base == len(np) and tuple(found) == TARGETS, 'exact seven observed Gate/Up word locations')
    return vectors


def registration_upload_pins(original_raw, summary, uploads_raw):
    registration = H.parse(original_raw)
    begin = summary['bootstrap']['sequence']['begin']
    scope = begin['scope']
    H.require(type(registration) is dict and type(scope) is dict, 'original registration/current scope')
    # These are the only changed identity fields; the resulting complete bytes must match Begin.
    for key in ('session', 'child_identity'):
        H.require(key in registration and key in scope, 'current identity field')
        registration[key] = scope[key]
    current = json.dumps(registration, separators=(',', ':'), allow_nan=False).encode()
    H.require(H.pin(current) == normalized(begin['registration']), 'complete reconstructed current registration commitment')
    for key in ('model_id', 'bundle_id', 'session', 'pool_identity', 'group_id', 'child_identity'):
        H.require(registration[key] == scope[key], 'registration scope identity')
    H.require(normalized(dict(bytes=registration['source_program_bytes'],
              sha256=registration['source_program_sha256'])) == normalized(begin['source_program']),
              'original source program commitment')
    H.require(H.pin(uploads_raw) == normalized(begin['uploads']), 'current complete Begin upload commitment')
    uploads = H.parse(uploads_raw)
    H.require(type(uploads) is dict and type(uploads.get('version')) is int and uploads['version'] == 1
              and type(uploads.get('uploads')) is list, 'original upload manifest')
    layers = registration['layers']
    H.require(type(layers) is list and len(layers) == 72
              and all(type(row['rank']) is int and type(row['layer']) is int for row in layers)
              and {(row['rank'], row['layer']) for row in layers}
                  == {(r, l) for r in (0, 1) for l in range(36)}, 'full rank/layer registration roster')
    result, ids = {}, set()
    for rank in (0, 1):
        layer = next(row for row in layers if (row['rank'], row['layer']) == (rank, 0))
        for kind in ('gate', 'up'):
            candidates = [row for row in layer['weights'] if row['kind'] == kind + '_projection']
            H.require(len(candidates) == 1, 'one registered Gate/Up weight role')
            buffer = candidates[0]['buffer']
            H.require(set(buffer) == {'rank', 'id', 'elements', 'element_bytes'}
                      and all(type(buffer[k]) is int for k in buffer)
                      and buffer['rank'] == rank and 0 <= buffer['id'] < 1 << 64
                      and (rank, buffer['id']) not in ids
                      and buffer['elements'] == PARTITION_ROWS * WIDTH and buffer['element_bytes'] == 2,
                      'unique rank-local contiguous BF16 partition')
            ids.add((rank, buffer['id']))
            key = dict(kind='source', rank=rank, id=buffer['id'])
            matches = [row for row in uploads['uploads'] if row['key'] == key]
            H.require(len(matches) == 1 and set(matches[0]) == {'key', 'bytes', 'sha256'}
                      and type(matches[0]['key']['rank']) is int
                      and type(matches[0]['key']['id']) is int, 'one exact actual source upload')
            value = normalized(matches[0])
            H.require(value['bytes'] == PARTITION_ROWS * WIDTH * 2, '6144-row uploaded partition extent')
            result[rank, kind] = dict(key=key, **value)
    return result, dict(original=H.pin(original_raw), reconstructed_current=H.pin(current),
                        replaced_fields=['session', 'child_identity'], original_body_replaced=False)


def check_partition(actual, expected):
    H.require(normalized(actual) == normalized(expected), 'current native Gate/Up partition equals original checkpoint')


def analyze(vectors, weight_rows):
    keys = {t[:4] for t in TARGETS}
    H.require(type(vectors) is dict and set(vectors) == keys
              and type(weight_rows) is dict and set(weight_rows) == keys, 'closed seven-dot input roster')
    rows = []
    for position, rank, kind, local, native, reference in TARGETS:
        key = (position, rank, kind, local)
        vector, weights = H.words(vectors[key], WIDTH), H.words(weight_rows[key], WIDTH)
        exact = H.exact_dot(vector, weights)
        ideal = H.nearest_bf16(exact)
        midpoint = (H.units(native) + H.units(reference)) << 132
        global_row, partition_row = coordinates(rank, kind, local)
        rows.append(dict(position=position, rank=rank, projection=kind, local_row=local,
            checkpoint_tensor=TENSORS[kind], checkpoint_row=global_row, uploaded_partition_row=partition_row,
            input=H.pin(vectors[key]), weight_row=H.pin(weight_rows[key]),
            native_word=native, framework_word=reference, nearest_real_dot_bf16_word=ideal,
            nearest_real_dot_is_finite=(ideal & 0x7f80) != 0x7f80,
            native_matches_real_dot_rne=native == ideal, framework_matches_real_dot_rne=reference == ideal,
            exact_dot_units_2_pow_minus266=str(exact), exact_dot_decimal=H.decimal_dyadic(exact),
            observed_pair_midpoint_units_2_pow_minus266=str(midpoint),
            exact_minus_observed_pair_midpoint_units_2_pow_minus266=str(exact - midpoint),
            exact_is_observed_pair_midpoint=exact == midpoint,
            native_minus_exact_units_2_pow_minus266=str((H.units(native) << 133) - exact),
            framework_minus_exact_units_2_pow_minus266=str((H.units(reference) << 133) - exact)))
    return dict(schema='ferric-causal-layer0-seven-gate-up-exact-dots-v1', rows=rows,
        terms_per_dot=WIDTH, dot_count=7, exact_product_unit='2^-266',
        shared_captured_inputs=True, checkpoint_weight_rows_used=True,
        framework_accumulation_emulation=False, native_accumulation_emulation=False,
        semantic_bug_claimed=False, activation_or_down_projection_diagnosed=False,
        mismatch_explains_position5_argmax=False, tolerance_applied=False,
        numerical_acceptance=False, full_model_acceptance=False, performance_claim=False)
