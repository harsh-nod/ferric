import copy
import json
import struct
import unittest

import head as H
import qkv as Q


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()


def fixture():
    native, framework, native_body, reference_body, metrics = [], [], bytearray(), bytearray(), []
    for position in range(6):
        nparts, fparts, body = [], [], bytearray()
        vector = bytes(8192)
        for name in ('input-norm', 'q-input', 'k-input', 'v-input'):
            fparts.append(dict(name=name, offset=len(reference_body), **H.pin(vector)))
            reference_body.extend(vector)
        projections = {kind: [0] * Q.ROWS[kind] for kind in 'qkv'}
        for rank in (0, 1):
            packed = [0] * 3072
            for p, r, kind, local, a, b in Q.TARGETS:
                if (p, r) == (position, rank):
                    global_row, packed_row = Q.coordinates(rank, kind, local)
                    packed[packed_row] = a
                    projections[kind][global_row] = b
            for role, raw in [('input_normalized', vector), ('raw_qkv', struct.pack('<3072H', *packed))]:
                nparts.append(dict(rank=rank, role=role, offset=len(body), **H.pin(raw)))
                body.extend(raw)
        for kind in 'qkv':
            raw = struct.pack('<%dH' % Q.ROWS[kind], *projections[kind])
            fparts.append(dict(name=kind + '-projection', offset=len(reference_body), **H.pin(raw)))
            reference_body.extend(raw)
            for rank in (0, 1):
                packed_part = next(x for x in nparts if x['rank'] == rank and x['role'] == 'raw_qkv')
                start = packed_part['offset'] + 2 * Q.coordinates(rank, kind, 0)[1]
                count = Q.ROWS[kind] // 2
                n = bytes(body[start:start + 2 * count])
                f = raw[2 * rank * count:2 * (rank + 1) * count]
                metrics.append(dict(id='%d:%d:%s_projection' % (position, rank, kind),
                    same_captured_inputs=True, native=H.pin(n), reference=H.pin(f),
                    different_words=sum(a != b for a, b in zip(H.words(n, count), H.words(f, count)))))
        native.append(dict(position=position, generation=position + 1, layer=0, parts=nparts,
                           payload_bytes=len(body), payload_sha256=H.pin(bytes(body))['sha256']))
        native_body.extend(body)
        framework.append(dict(position=position, input_token=1, parts=fparts))
    while len(metrics) < 204:
        metrics.append(dict(id='other:%d' % len(metrics)))
    def envelope(magic, header, payload):
        raw = encoded(header)
        return magic + struct.pack('<II', len(raw), len(payload)) + raw + payload
    n = envelope(b'FCAP061\0', dict(captures=native, payload_bytes=len(native_body),
        payload_sha256=H.pin(bytes(native_body))['sha256']), bytes(native_body))
    refs = [envelope(b'FREF061\0', dict(ordinal=i, captures=framework,
        payload=H.pin(bytes(reference_body))), bytes(reference_body)) for i in (1, 2)]
    return n, refs[0], refs[1], dict(checks=dict(diagnostic=dict(comparable_rows=metrics)))


class QkvTests(unittest.TestCase):
    def test_rank_and_projection_row_coordinates(self):
        self.assertEqual(Q.coordinates(0, 'q', 2047), (2047, 2047))
        self.assertEqual(Q.coordinates(1, 'q', 1902), (3950, 1902))
        self.assertEqual(Q.coordinates(1, 'k', 97), (609, 2145))
        self.assertEqual(Q.coordinates(1, 'v', 511), (1023, 3071))
        for args in ((True, 'q', 0), (2, 'q', 0), (0, 'k', 512), (1, 'q', -1), (0, 'o', 0)):
            with self.subTest(args=args), self.assertRaises(ValueError):
                Q.coordinates(*args)

    def test_original_shard_geometry_and_index_refusals(self):
        header, offset = {}, 0
        for kind in 'qkv':
            size = Q.ROWS[kind] * 8192
            header[Q.TENSORS[kind]] = dict(dtype='BF16', shape=[Q.ROWS[kind], 4096],
                data_offsets=[offset, offset + size])
            offset += size
        index = dict(weight_map={name: Q.SHARD for name in header})
        self.assertEqual(Q.tensor_layout(header, index, 100, 108 + offset)['q'], 108)
        for mutation in ('shape', 'dtype', 'offset', 'shard', 'trailer', 'missing'):
            h, i, size = copy.deepcopy(header), copy.deepcopy(index), 108 + offset
            if mutation == 'shape': h[Q.TENSORS['q']]['shape'] = [2048, 8192]
            elif mutation == 'dtype': h[Q.TENSORS['k']]['dtype'] = 'F32'
            elif mutation == 'offset': h[Q.TENSORS['v']]['data_offsets'][0] += 2
            elif mutation == 'shard': i['weight_map'][Q.TENSORS['q']] = 'other'
            elif mutation == 'trailer': size += 2
            else: del h[Q.TENSORS['k']]
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                Q.tensor_layout(h, i, 100, size)

    def test_extraction_closes_five_rows_and_repeated_inputs(self):
        args = fixture()
        vectors = Q.extract(*args)
        self.assertEqual(set(vectors), {row[:4] for row in Q.TARGETS})
        self.assertTrue(all(value == bytes(8192) for value in vectors.values()))
        for mutation in ('input_relation', 'count', 'repeat', 'trailer'):
            n, f, repeat, c = copy.deepcopy(args)
            if mutation == 'input_relation': c['checks']['diagnostic']['comparable_rows'][0]['same_captured_inputs'] = False
            elif mutation == 'count': c['checks']['diagnostic']['comparable_rows'][0]['different_words'] += 1
            elif mutation == 'repeat': repeat = f
            else: n += b'\0'
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                Q.extract(n, f, repeat, c)

    def test_unique_rank_specific_packed_uploads(self):
        rows = [dict(key=dict(kind='pending', rank=r, layer=0, role='packed_qkv_weight'),
                     bytes=25165824, sha256=('%02x' % r) * 32) for r in (0, 1)]
        self.assertEqual(set(Q.packed_upload_pins(dict(version=1, uploads=rows))), {0, 1})
        for value in (rows[:1], rows + [rows[0]], [rows[0], rows[0]]):
            with self.subTest(value=value), self.assertRaises(ValueError):
                Q.packed_upload_pins(dict(version=1, uploads=value))

    def test_exact_five_dots_preserve_midpoints_and_no_acceptance(self):
        vectors = {t[:4]: struct.pack('<4096H', 0x3f80, *([0] * 4095)) for t in Q.TARGETS}
        weights = {t[:4]: struct.pack('<4096H', t[4], *([0] * 4095)) for t in Q.TARGETS}
        result = Q.analyze(vectors, weights)
        self.assertEqual(result['dot_count'], 5)
        for row, target in zip(result['rows'], Q.TARGETS):
            self.assertEqual(row['nearest_real_dot_bf16_word'], target[4])
            self.assertTrue(row['native_matches_real_dot_rne'])
            self.assertFalse(row['framework_matches_real_dot_rne'])
            self.assertEqual(row['native_minus_exact_units_2_pow_minus266'], '0')
            midpoint = (H.units(target[4]) + H.units(target[5])) << 132
            self.assertEqual(int(row['observed_pair_midpoint_units_2_pow_minus266']), midpoint)
        self.assertFalse(result['numerical_acceptance'])
        self.assertFalse(result['semantic_bug_claimed'])
        self.assertFalse(result['o_projection_or_residual_diagnosed'])

    def test_missing_wrong_extent_and_nonfinite_rows_refused(self):
        vectors = {t[:4]: bytes(8192) for t in Q.TARGETS}
        weights = dict(vectors)
        for mutation in ('missing', 'extent', 'nonfinite'):
            changed = dict(weights)
            key = Q.TARGETS[0][:4]
            if mutation == 'missing': del changed[key]
            elif mutation == 'extent': changed[key] = bytes(8190)
            else: changed[key] = struct.pack('<H', 0x7f80) + bytes(8190)
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                Q.analyze(vectors, changed)
