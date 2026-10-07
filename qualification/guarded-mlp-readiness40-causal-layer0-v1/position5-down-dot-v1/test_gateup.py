import copy
import json
import struct
import unittest

import head as H
import gateup as G


def encoded(value):
    return json.dumps(value, separators=(',', ':'), allow_nan=False).encode()


def fixture(wrong_input=False):
    native, framework, native_body, reference_body, metrics = [], [], bytearray(), bytearray(), []
    for position in range(6):
        nparts, fparts, body = [], [], bytearray()
        vector = bytes(8192)
        for name in ('post-norm', 'gate-input', 'up-input'):
            value = (struct.pack('<H', 0x3f80) + bytes(8190)
                     if wrong_input and position == 5 and name == 'gate-input' else vector)
            fparts.append(dict(name=name, offset=len(reference_body), **H.pin(value)))
            reference_body.extend(value)
        projections = {kind: [0] * G.ROWS[kind] for kind in ('gate', 'up')}
        for rank in (0, 1):
            parts = {kind: [0] * G.PARTITION_ROWS for kind in ('gate', 'up')}
            for p, r, kind, local, a, b in G.TARGETS:
                if (p, r) == (position, rank):
                    global_row, local_row = G.coordinates(rank, kind, local)
                    parts[kind][local_row] = a
                    projections[kind][global_row] = b
            for role, raw in [('post_normalized', vector)] + [
                    (kind, struct.pack('<6144H', *parts[kind])) for kind in ('gate', 'up')]:
                nparts.append(dict(rank=rank, role=role, offset=len(body), **H.pin(raw)))
                body.extend(raw)
            metrics.append(dict(id='%d:%d:post_normalized' % (position, rank),
                native=H.pin(vector), reference=H.pin(vector), different_words=0))
        for kind in ('gate', 'up'):
            raw = struct.pack('<12288H', *projections[kind])
            fparts.append(dict(name=kind, offset=len(reference_body), **H.pin(raw)))
            reference_body.extend(raw)
            for rank in (0, 1):
                part = next(x for x in nparts if x['rank'] == rank and x['role'] == kind)
                n = bytes(body[part['offset']:part['offset'] + part['bytes']])
                f = raw[2 * rank * 6144:2 * (rank + 1) * 6144]
                metrics.append(dict(id='%d:%d:%s' % (position, rank, kind),
                    same_captured_inputs=True, native=H.pin(n), reference=H.pin(f),
                    different_words=sum(a != b for a, b in zip(H.words(n, 6144), H.words(f, 6144)))))
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


def mapping_fixture(mutation=None):
    scope = dict(model_id=[3] * 32, bundle_id=[4] * 32, session=[2] * 32,
                 pool_identity=1, group_id=0, child_identity=200)
    registration = dict(scope, session=[1] * 32, child_identity=100,
        source_program_bytes=3, source_program_sha256=[5] * 32, layers=[])
    uploads = dict(version=1, uploads=[])
    for rank in (0, 1):
        for layer in range(36):
            weights = []
            for ordinal, kind in enumerate(('gate', 'up')):
                buffer = dict(rank=rank, id=190 + rank + 2 * ordinal,
                              elements=6144 * 4096, element_bytes=2)
                weights.append(dict(kind=kind + '_projection', buffer=buffer))
                if layer == 0:
                    uploads['uploads'].append(dict(key=dict(kind='source', rank=rank, id=buffer['id']),
                        bytes=50331648, sha256=('%02x' % (10 + rank * 2 + ordinal)) * 32))
            registration['layers'].append(dict(rank=rank, layer=layer, weights=weights))
    if mutation == 'role': registration['layers'][0]['weights'][0]['kind'] = 'down_projection'
    if mutation == 'duplicate_role': registration['layers'][0]['weights'].append(copy.deepcopy(registration['layers'][0]['weights'][0]))
    if mutation == 'rank': registration['layers'][0]['weights'][0]['buffer']['rank'] = 1
    if mutation == 'alias': registration['layers'][0]['weights'][1]['buffer']['id'] = 190
    if mutation == 'extent': registration['layers'][0]['weights'][0]['buffer']['elements'] -= 1
    if mutation == 'bool': registration['layers'][0]['weights'][0]['buffer']['rank'] = False
    if mutation == 'missing_upload': uploads['uploads'].pop()
    if mutation == 'duplicate_upload': uploads['uploads'].append(copy.deepcopy(uploads['uploads'][0]))
    if mutation == 'upload_role': uploads['uploads'][0]['key']['kind'] = 'pending'
    if mutation == 'upload_extent': uploads['uploads'][0]['bytes'] -= 2
    current = copy.deepcopy(registration)
    current.update(session=scope['session'], child_identity=scope['child_identity'])
    upload_raw = encoded(uploads)
    begin = dict(scope=scope, registration=H.pin(encoded(current)), uploads=H.pin(upload_raw),
                 source_program=dict(bytes=3, sha256=[5] * 32))
    return encoded(registration), dict(bootstrap=dict(sequence=dict(begin=begin))), upload_raw


class GateUpTests(unittest.TestCase):
    def test_rank_and_partition_boundary_coordinates(self):
        self.assertEqual(G.coordinates(0, 'gate', 6143), (6143, 6143))
        self.assertEqual(G.coordinates(1, 'gate', 0), (6144, 0))
        self.assertEqual(G.coordinates(1, 'up', 5310), (11454, 5310))
        for args in ((True, 'gate', 0), (2, 'gate', 0), (0, 'up', 6144),
                     (1, 'gate', -1), (0, 'down', 0), (0, 'gate', True)):
            with self.subTest(args=args), self.assertRaises(ValueError):
                G.coordinates(*args)

    def test_original_shard_transpose_and_extent_refusals(self):
        header, offset = {}, 0
        for kind in ('gate', 'up'):
            size = G.ROWS[kind] * 8192
            header[G.TENSORS[kind]] = dict(dtype='BF16', shape=[12288, 4096],
                data_offsets=[offset, offset + size])
            offset += size
        index = dict(weight_map={name: G.SHARD for name in header})
        self.assertEqual(G.tensor_layout(header, index, 100, 108 + offset)['gate'], 108)
        for mutation in ('transpose', 'dtype', 'offset', 'shard', 'trailer', 'missing'):
            h, i, size = copy.deepcopy(header), copy.deepcopy(index), 108 + offset
            if mutation == 'transpose': h[G.TENSORS['gate']]['shape'] = [4096, 12288]
            elif mutation == 'dtype': h[G.TENSORS['up']]['dtype'] = 'F32'
            elif mutation == 'offset': h[G.TENSORS['up']]['data_offsets'][0] += 2
            elif mutation == 'shard': i['weight_map'][G.TENSORS['gate']] = 'other'
            elif mutation == 'trailer': size += 2
            else: del h[G.TENSORS['up']]
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                G.tensor_layout(h, i, 100, size)

    def test_extraction_closes_seven_rows_and_repetition(self):
        args = fixture()
        vectors = G.extract(*args)
        self.assertEqual(set(vectors), {row[:4] for row in G.TARGETS})
        self.assertTrue(all(value == bytes(8192) for value in vectors.values()))
        for mutation in ('input_relation', 'count', 'repeat', 'trailer'):
            n, f, repeat, c = copy.deepcopy(args)
            row = next(r for r in c['checks']['diagnostic']['comparable_rows'] if r['id'] == '5:0:gate')
            if mutation == 'input_relation': row['same_captured_inputs'] = False
            elif mutation == 'count': row['different_words'] += 1
            elif mutation == 'repeat': repeat = f
            else: n += b'\0'
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                G.extract(n, f, repeat, c)

    def test_wrong_input_and_comparison_pin_refused(self):
        with self.assertRaises(ValueError):
            G.extract(*fixture(wrong_input=True))
        for role in ('post_normalized', 'gate', 'up'):
            for side in ('native', 'reference'):
                n, f, repeat, c = fixture()
                row = next(r for r in c['checks']['diagnostic']['comparable_rows'] if r['id'] == '5:0:' + role)
                row[side]['sha256'] = '0' * 64
                with self.subTest(role=role, side=side), self.assertRaises(ValueError):
                    G.extract(n, f, repeat, c)

    def test_registration_reconstruction_and_four_source_roles(self):
        original, summary, uploads = mapping_fixture()
        rows, proof = G.registration_upload_pins(original, summary, uploads)
        self.assertEqual(set(rows), {(r, k) for r in (0, 1) for k in ('gate', 'up')})
        self.assertEqual(rows[1, 'up']['key'], dict(kind='source', rank=1, id=193))
        self.assertEqual(proof['original'], H.pin(original))
        self.assertEqual(proof['reconstructed_current'], summary['bootstrap']['sequence']['begin']['registration'])
        self.assertFalse(proof['original_body_replaced'])
        for field in ('registration', 'uploads', 'source_program'):
            changed = copy.deepcopy(summary)
            changed['bootstrap']['sequence']['begin'][field]['sha256'] = '0' * 64
            with self.subTest(field=field), self.assertRaises(ValueError):
                G.registration_upload_pins(original, changed, uploads)

    def test_wrong_role_rank_alias_extent_and_upload_refused(self):
        for mutation in ('role', 'duplicate_role', 'rank', 'alias', 'extent', 'bool',
                         'missing_upload', 'duplicate_upload', 'upload_role', 'upload_extent'):
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                G.registration_upload_pins(*mapping_fixture(mutation))

    def test_partition_pin_wrong_role_rank_and_mutated_bytes_refused(self):
        rows, _ = G.registration_upload_pins(*mapping_fixture())
        G.check_partition(rows[0, 'gate'], rows[0, 'gate'])
        for other in (rows[0, 'up'], rows[1, 'gate'],
                      dict(bytes=50331648, sha256='0' * 64), dict(bytes=50331646, sha256=rows[0, 'gate']['sha256'])):
            with self.subTest(other=other), self.assertRaises(ValueError):
                G.check_partition(rows[0, 'gate'], other)

    def test_exact_seven_dots_bounds_midpoints_and_nonclaims(self):
        vectors = {t[:4]: struct.pack('<4096H', 0x3f80, *([0] * 4095)) for t in G.TARGETS}
        weights = {t[:4]: struct.pack('<4096H', t[4], *([0] * 4095)) for t in G.TARGETS}
        result = G.analyze(vectors, weights)
        self.assertEqual(result['dot_count'], 7)
        for row, target in zip(result['rows'], G.TARGETS):
            self.assertEqual(row['nearest_real_dot_bf16_word'], target[4])
            self.assertTrue(row['native_matches_real_dot_rne'])
            self.assertFalse(row['framework_matches_real_dot_rne'])
            self.assertEqual(row['native_minus_exact_units_2_pow_minus266'], '0')
            self.assertEqual(int(row['observed_pair_midpoint_units_2_pow_minus266']),
                             (H.units(target[4]) + H.units(target[5])) << 132)
        for key in ('numerical_acceptance', 'semantic_bug_claimed', 'activation_or_down_projection_diagnosed',
                    'mismatch_explains_position5_argmax', 'native_accumulation_emulation'):
            self.assertFalse(result[key])
        for mutation in ('missing', 'extent', 'nonfinite'):
            changed = dict(weights)
            key = G.TARGETS[0][:4]
            if mutation == 'missing': del changed[key]
            elif mutation == 'extent': changed[key] = bytes(8190)
            else: changed[key] = struct.pack('<H', 0x7f80) + bytes(8190)
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                G.analyze(vectors, changed)
