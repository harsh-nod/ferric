"""Five observed Q/K differences on identical captured inputs, not a GEMM emulator."""
import hashlib
import struct

import head as H

WIDTH = 4096
SHARD = 'model-00001-of-00005.safetensors'
TENSORS = {kind: 'model.layers.0.self_attn.%s_proj.weight' % kind for kind in 'qkv'}
ROWS = dict(q=4096, k=1024, v=1024)
# Position, rank, projection, local output row, native word, framework word.
TARGETS = ((0, 0, 'q', 168, 0xb8c2, 0xb8c3),
           (1, 1, 'q', 1902, 0xb976, 0xb977),
           (2, 0, 'q', 1462, 0x3787, 0x3788),
           (3, 0, 'q', 10, 0x35d8, 0x35d7),
           (5, 1, 'k', 97, 0x3663, 0x3664))


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
    return rank * (ROWS[kind] // 2) + local, dict(q=0, k=2048, v=2560)[kind] + local


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
                  'layer-zero original row-major QKV')
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
        body = np[base:base + nc['payload_bytes']]
        base += len(body)
        H.require(H.pin(body) == normalized(dict(bytes=nc['payload_bytes'], sha256=nc['payload_sha256']))
                  and nc['generation'] == position + 1 and nc['layer'] == 0, 'native capture body/scope')
        nr, fr = {}, {}
        for part in nc['parts']:
            key = (part['rank'], part['role'])
            H.require(key not in nr, 'duplicate native capture role')
            value = body[part['offset']:part['offset'] + part['bytes']]
            H.require(H.pin(value) == normalized(part), 'native captured part pin')
            nr[key] = value
        for part in fc['parts']:
            H.require(part['name'] not in fr, 'duplicate framework capture role')
            value = fp[part['offset']:part['offset'] + part['bytes']]
            H.require(H.pin(value) == normalized(part), 'framework captured part pin')
            fr[part['name']] = value
        for rank in (0, 1):
            vector = nr[rank, 'input_normalized']
            H.words(vector, WIDTH)
            H.require(vector == fr['input-norm'], 'same full captured normalized input')
            packed = H.words(nr[rank, 'raw_qkv'], 3072)
            for kind, start in (('q', 0), ('k', 2048), ('v', 2560)):
                H.require(vector == fr[kind + '-input'], 'same actual projection input')
                count = ROWS[kind] // 2
                native = nr[rank, 'raw_qkv'][2 * start:2 * (start + count)]
                reference = fr[kind + '-projection'][2 * rank * count:2 * (rank + 1) * count]
                ref_words = H.words(reference, count)
                row = recorded['%d:%d:%s_projection' % (position, rank, kind)]
                H.require(row['same_captured_inputs'] is True and row['native'] == H.pin(native)
                          and row['reference'] == H.pin(reference), 'actual comparison part/input join')
                differences = 0
                for local, (a, b) in enumerate(zip(packed[start:start + count], ref_words)):
                    if a != b:
                        found.append((position, rank, kind, local, a, b))
                        vectors[position, rank, kind, local] = vector
                        differences += 1
                H.require(differences == row['different_words'], 'complete QKV difference census')
    H.require(base == len(np) and tuple(found) == TARGETS, 'exact five observed QKV word locations')
    return vectors


def packed_upload_pins(uploads):
    H.require(type(uploads) is dict and uploads.get('version') == 1
              and type(uploads.get('uploads')) is list, 'original upload manifest')
    result = {}
    for rank in (0, 1):
        key = dict(kind='pending', rank=rank, layer=0, role='packed_qkv_weight')
        rows = [r for r in uploads['uploads'] if r['key'] == key]
        H.require(len(rows) == 1 and rows[0]['bytes'] == 3072 * WIDTH * 2,
                  'unique current rank-zero-layer packed-QKV upload')
        result[rank] = normalized(rows[0])
    return result


def analyze(vectors, weight_rows):
    keys = {t[:4] for t in TARGETS}
    H.require(type(vectors) is dict and set(vectors) == keys
              and type(weight_rows) is dict and set(weight_rows) == keys, 'closed five-dot input roster')
    rows = []
    for position, rank, kind, local, native, reference in TARGETS:
        key = (position, rank, kind, local)
        vector, weights = H.words(vectors[key], WIDTH), H.words(weight_rows[key], WIDTH)
        exact = H.exact_dot(vector, weights)
        ideal = H.nearest_bf16(exact)
        midpoint = (H.units(native) + H.units(reference)) << 132
        global_row, packed_row = coordinates(rank, kind, local)
        rows.append(dict(position=position, rank=rank, projection=kind, local_row=local,
            checkpoint_tensor=TENSORS[kind], checkpoint_row=global_row, packed_rank_row=packed_row,
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
    return dict(schema='ferric-causal-layer0-five-qkv-exact-dots-v1', rows=rows,
        terms_per_dot=WIDTH, dot_count=5, exact_product_unit='2^-266',
        shared_captured_inputs=True, checkpoint_weight_rows_used=True,
        framework_accumulation_emulation=False, native_accumulation_emulation=False,
        semantic_bug_claimed=False, o_projection_or_residual_diagnosed=False,
        mismatch_explains_position5_argmax=False, tolerance_applied=False,
        numerical_acceptance=False, full_model_acceptance=False, performance_claim=False)
