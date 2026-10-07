"""Synthetic exact arithmetic and original-evidence custody tests, never model runs."""
import copy
from fractions import Fraction
import json
import struct
import unittest

import down as D
import exact_bf16 as E
import fp32_replay as F
import gateup as G
import head as H
import position5 as P
import r2 as R
import test_gateup as TG
import test_position5 as TP


def encoded(value):
    return json.dumps(value, separators=(',', ':'), allow_nan=False).encode()


def fraction(word):
    sign, exponent, mantissa = (-1 if word & 0x8000 else 1), (word >> 7) & 255, word & 127
    if exponent == 0:
        return sign * Fraction(mantissa, 1 << 133)
    power = exponent - 134
    return sign * Fraction(128 + mantissa) * (Fraction(2 ** power) if power >= 0 else Fraction(1, 2 ** -power))


def products():
    words = {(side, rank): [0] * D.HALF for side in ('native', 'framework') for rank in (0, 1)}
    for rank, local, native, reference in D.PRODUCT_DIFFERENCES:
        words['native', rank][local] = native
        words['framework', rank][local] = reference
    return {key: struct.pack('<6144H', *value) for key, value in words.items()}


def product_fixture():
    values, body, parts = products(), bytearray(), []
    for rank in (0, 1):
        raw = values['native', rank]
        parts.append(dict(rank=rank, role='activation', boundary='after_mlp', scalar='bf16',
            elements=D.HALF, source_byte_offset=0, offset=len(body), **H.pin(raw)))
        body.extend(raw)
    full = values['framework', 0] + values['framework', 1]
    return (dict(position=5, generation=6, layer=0, parts=parts), bytes(body),
        dict(position=5, input_token=271, parts=[dict(name='product', dtype='bfloat16',
             shape=[1, 1, 12288], offset=0, **H.pin(full))]), full)


def mapping_fixture(mutation=None):
    original, summary, uploads_raw = TG.mapping_fixture()
    original, uploads = H.parse(original), H.parse(uploads_raw)
    for layer in original['layers']:
        rank = layer['rank']
        layer['weights'].append(dict(kind='down_projection',
            buffer=dict(rank=rank, id=188+rank, elements=D.WIDTH*D.HALF, element_bytes=2)))
        if layer['layer'] == 0:
            uploads['uploads'].append(dict(key=dict(kind='source', rank=rank, id=188+rank),
                                          bytes=50331648, sha256=('%02x' % (20+rank)) * 32))
    down = original['layers'][0]['weights'][-1]
    if mutation == 'missing': original['layers'][0]['weights'].pop()
    elif mutation == 'duplicate': original['layers'][0]['weights'].append(copy.deepcopy(down))
    elif mutation == 'rank': down['buffer']['rank'] = True
    elif mutation == 'shape': down['buffer']['elements'] -= 1
    elif mutation == 'alias': down['buffer']['id'] = 190
    elif mutation == 'role': down['kind'] = 'gate_projection'
    elif mutation == 'upload': uploads['uploads'][-2]['key']['kind'] = 'pending'
    elif mutation == 'extent': uploads['uploads'][-2]['bytes'] -= 2
    elif mutation == 'duplicate_upload': uploads['uploads'].append(copy.deepcopy(uploads['uploads'][-2]))
    begin = summary['bootstrap']['sequence']['begin']
    current = copy.deepcopy(original)
    current.update(session=begin['scope']['session'], child_identity=begin['scope']['child_identity'])
    begin['registration'] = H.pin(encoded(current))
    upload_raw = encoded(uploads)
    begin['uploads'] = H.pin(upload_raw)
    return encoded(original), summary, upload_raw


def capture_fixture():
    nc, np, fc, fp, rows = TP.fixture()
    products_nc, products_np, products_fc, products_fp = product_fixture()
    native, framework = bytearray(), bytearray()
    for part in nc['parts']:
        value = np[part['offset']:part['offset'] + part['bytes']]
        if part['role'] in ('other0', 'other1'):
            replacement = copy.deepcopy(products_nc['parts'][int(part['role'][-1])])
            value = products_np[replacement['offset']:replacement['offset'] + replacement['bytes']]
            part.clear(); part.update(replacement)
        part['offset'] = len(native); native.extend(value)
    for part in fc['parts']:
        value = fp[part['offset']:part['offset'] + part['bytes']]
        if part['name'] == 'other0':
            part.clear(); part.update(copy.deepcopy(products_fc['parts'][0])); value = products_fp
        part['offset'] = len(framework); framework.extend(value)
    nc.update(payload_bytes=len(native), payload_sha256=H.pin(bytes(native))['sha256'])
    natives, references = [], []
    for position in range(6):
        a, b = copy.deepcopy(nc), copy.deepcopy(fc)
        a.update(position=position, generation=position+1); b['position'] = position
        for part in b['parts']: part['offset'] += position * len(framework)
        natives.append(a); references.append(b)
    np, fp = bytes(native)*6, bytes(framework)*6
    nh = dict(captures=natives, payload_bytes=len(np), payload_sha256=H.pin(np)['sha256'], native_close_confirmed=True)
    fh = dict(captures=references, payload=H.pin(fp), ordinal=1)
    comparison = dict(checks=dict(both_same_side_parity_gates_rechecked=True, diagnostic=dict(comparable_rows=rows)))
    return {'comparison.json': encoded(comparison), 'native-sidecar.bin': TP.envelope(b'FCAP061\0',nh,np),
            'reference-sidecar1.bin': TP.envelope(b'FREF061\0',fh,fp),
            'reference-sidecar2.bin': TP.envelope(b'FREF061\0',dict(fh,ordinal=2),fp)}


def reused_fixture():
    nc, np, fc, fp, rows = TP.fixture()
    native, reference, pins = P.select(nc, np, fc, fp, rows)
    ref_down, ref_final = [0x3f80] * D.WIDTH, [0x4000] * D.WIDTH
    for i in D.DIFFERING_ROWS:
        ref_down[i], ref_final[i] = 0x3f00, 0x3fc0
    reference.update({'down-projection': struct.pack('<4096H', *ref_down),
                      'mlp-output': struct.pack('<4096H', *ref_down),
                      'layer0-hidden': struct.pack('<4096H', *ref_final)})
    captured = dict(native=native, reference=reference, selected_parts=pins,
                    original_sidecars=dict(native=H.pin(b'n'), reference=H.pin(b'f'), repeat=H.pin(b'r')),
                    products=products(), product_pins=[])
    nr, outputs = R.replay(native)
    replay = dict(schema='ferric-causal-position5-r2-original-replay-rows-v1', position=5, layer=0,
                  native=nr, framework=R.framework_control(reference['down-projection'],
                       reference['first-residual'], reference['layer0-hidden']))
    replay_raw = encoded(replay)
    diagnostic = dict(order=D.R2_ORDER, all_observed_boundaries_exact=True,
        native_matched_words=8192, framework_matched_words=4096,
        original_sidecars=captured['original_sidecars'], native_reference_different_rows=[list(D.DIFFERING_ROWS)] * 2,
        retained_outputs={'replay.json': H.pin(replay_raw), **{k: H.pin(v) for k, v in outputs.items()}})
    terminal = dict(schema='ferric-causal-position5-r2-replay-cpu-v1', passed=True, error=None,
        postcheck_errors=[], tests=dict(passed=26, failures=0, errors=0, skipped=0),
        dedicated_down_dot_products_replayed=False, numerical_acceptance=False, diagnostic=diagnostic)
    return captured, encoded(terminal), replay_raw, outputs['derived-down.bf16'], outputs['derived-ordered-sum.f32']


class DownTests(unittest.TestCase):
    def test_row_major_column_halves_not_contiguous_tensor_halves(self):
        self.assertEqual(D.row_offset(108, 0, 0), 108)
        self.assertEqual(D.row_offset(108, 0, 1), 108 + 12288)
        self.assertEqual(D.row_offset(108, 1, 0), 108 + 24576)
        self.assertEqual(D.row_offset(108, 4095, 1) + 12288, 108 + 100663296)
        for args in ((108, True, 0), (108, 4096, 0), (108, 0, True), (108, 0, 2), (-1, 0, 0)):
            with self.subTest(args=args), self.assertRaises(ValueError): D.row_offset(*args)

    def test_full_shard_layout_transpose_gap_dtype_and_index_refusal(self):
        header, offset = {}, 0
        for name, shape in [(G.TENSORS['gate'], [12288,4096]), (G.TENSORS['up'], [12288,4096]),
                            (D.TENSOR, [4096,12288])]:
            size = 2 * shape[0] * shape[1]
            header[name] = dict(dtype='BF16', shape=shape, data_offsets=[offset, offset+size]); offset += size
        index = dict(weight_map={name: G.SHARD for name in header})
        self.assertEqual(D.tensor_layout(header, index, 100, 108+offset), 108+201326592)
        for mutation in ('transpose', 'bool', 'dtype', 'gap', 'index', 'trailer'):
            h, i, size = copy.deepcopy(header), copy.deepcopy(index), 108+offset
            if mutation == 'transpose': h[D.TENSOR]['shape'] = [12288,4096]
            elif mutation == 'bool': h[D.TENSOR]['shape'] = [True,50331648]
            elif mutation == 'dtype': h[D.TENSOR]['dtype'] = 'F32'
            elif mutation == 'gap': h[D.TENSOR]['data_offsets'][0] += 2
            elif mutation == 'index': i['weight_map'][D.TENSOR] = 'other'
            else: size += 2
            with self.subTest(mutation=mutation), self.assertRaises(ValueError): D.tensor_layout(h, i, 100, size)

    def test_original_registration_and_current_upload_roles(self):
        partitions, join = D.registration_upload_pins(*mapping_fixture())
        self.assertEqual([partitions[r]['key']['id'] for r in (0,1)], [188,189])
        self.assertEqual(join['replaced_fields'], ['session','child_identity'])
        for mutation in ('missing','duplicate','rank','shape','alias','role','upload','extent','duplicate_upload'):
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                D.registration_upload_pins(*mapping_fixture(mutation))

    def test_partition_byte_hash_and_complete_extent(self):
        expected = H.pin(b'abcd')
        D.check_partition(H.pin(b'abcd'), expected)
        for raw in (b'abce', b'abc', b'abcdabcd'):
            with self.assertRaises(ValueError): D.check_partition(H.pin(raw), expected)
        blocks = [bytes([i]) * 12288 for i in range(4)]
        tensor = b''.join(blocks)
        for rank in (0,1):
            selected = b''.join(tensor[D.row_offset(0,row,rank):D.row_offset(0,row,rank)+12288] for row in (0,1))
            expected = H.pin(blocks[rank]+blocks[2+rank])
            D.check_partition(H.pin(selected), expected)
            with self.assertRaises(ValueError): D.check_partition(H.pin(tensor[rank*24576:(rank+1)*24576]), expected)

    def test_product_halves_and_exact_seven_original_coordinates(self):
        selected, pins = D.select_products(*product_fixture())
        self.assertEqual(selected, products())
        self.assertEqual(D.product_differences(selected), D.PRODUCT_DIFFERENCES)
        self.assertEqual(len(pins), 5)
        captured = D.extract(capture_fixture())
        self.assertEqual(captured['products'], selected)
        self.assertEqual(captured['native'][0,'first_residual'], captured['reference']['first-residual'])

    def test_product_role_scalar_extent_identity_and_hash_refusals(self):
        for field, value in [('rank', True), ('elements',6143), ('scalar','f32'), ('boundary','other'),
                             ('source_byte_offset',1), ('bytes',12286), ('offset',1)]:
            args = copy.deepcopy(product_fixture()); args[0]['parts'][0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError): D.select_products(*args)
        args = copy.deepcopy(product_fixture()); args[0]['parts'].append(copy.deepcopy(args[0]['parts'][0]))
        with self.assertRaises(ValueError): D.select_products(*args)
        args = copy.deepcopy(product_fixture()); args[2]['parts'][0]['shape'] = [True,1,12288]
        with self.assertRaises(ValueError): D.select_products(*args)
        args = list(product_fixture()); args[1] = b'\1\0' + args[1][2:]
        with self.assertRaises(ValueError): D.select_products(*args)

    def test_exact_6144_term_dot_against_independent_fraction(self):
        left = ([0x3f80,0xbf80,1,0x8001,0x3b81,0xbb7f] * 1024)
        right = ([0x3f81,0x3f80,1,0x3f80,0xbc00,0x3f00] * 1024)
        expected = sum((fraction(a)*fraction(b) for a,b in zip(left,right)), Fraction())
        self.assertEqual(Fraction(E.dot_units(left,right), 1 << 266), expected)
        self.assertEqual(E.dot_units(left,right), sum(E.dot_units(left[i:i+64],right[i:i+64]) for i in range(0,6144,64)))

    def test_reuse_complete_original_boundary_rows_without_dot_reexecution(self):
        args = reused_fixture()
        reused = D.reuse_r2(*args)
        self.assertEqual(reused['down'], [0x3f80] * 4096)
        self.assertEqual(reused['final'][0], reused['framework_final'][0])
        self.assertEqual([i for i in range(4096) if reused['final'][i] != reused['framework_final'][i]], list(D.DIFFERING_ROWS))

    def test_reuse_failure_pin_row_order_operand_and_output_drift_refuse(self):
        original = reused_fixture()
        for mutation in ('failed','order','rows','pin','operand','result','sidecar'):
            args = copy.deepcopy(list(original)); t, r = H.parse(args[1]), H.parse(args[2])
            if mutation == 'failed': t['passed'] = False
            elif mutation == 'order': t['diagnostic']['order'] = 'different'
            elif mutation == 'rows': t['diagnostic']['native_reference_different_rows'][0].pop()
            elif mutation == 'pin': t['diagnostic']['retained_outputs']['derived-down.bf16']['sha256'] = '0'*64
            elif mutation == 'sidecar': t['diagnostic']['original_sidecars']['native'] = H.pin(b'changed')
            else:
                r['native']['rows'][0]['down_partial_f32' if mutation == 'operand' else 'replay_final_bf16'][0] = '00000000'
                args[2] = encoded(r); t['diagnostic']['retained_outputs']['replay.json'] = H.pin(args[2])
            args[1] = encoded(t)
            with self.subTest(mutation=mutation), self.assertRaises(ValueError): D.reuse_r2(*args)

    def test_exact_error_terms_reconcile_without_floating_threshold(self):
        unit = 1 << 266
        result = D.decompose([unit//2-1,unit//2+3], [unit//2,unit//2-7],
            [0x3f000000,0x3f000000], 0x3f800000,0x3f80,0x3f00,0x3f80,0x4000,0x3fc0)
        t = {k:int(v) for k,v in result['terms'].items()}
        self.assertEqual(t['upstream_own_product_delta'],9)
        self.assertEqual(t['native_rank0_accumulation_error'],1)
        self.assertEqual(t['native_rank1_accumulation_error'],-3)
        self.assertEqual(t['observed_projection_difference'],unit//2)
        self.assertEqual(t['observed_final_hidden_difference'],unit//2)
        self.assertTrue(result['exact_decomposition_reconciled'])

    def test_tp_sum_rounding_and_bf16_projection_rounding_are_distinct(self):
        p = [0x3f808001,0x33000000]
        total = F.add_f32(F.add_f32(0,p[0]),p[1]); projection = F.narrow_f32(total)
        result = D.decompose([F.f32_units(p[0]), F.f32_units(p[1])], [1 << 266,0],
            p,total,projection,0x3f80,0,projection,0x3f80)
        t = {k:int(v) for k,v in result['terms'].items()}
        self.assertNotEqual(t['native_tp_sum_rounding'],0)
        self.assertNotEqual(t['native_projection_bf16_rounding'],0)
        self.assertEqual(t['native_rank0_accumulation_error']+t['native_rank1_accumulation_error'],0)

    def test_strict_finite_operands_sum_join_and_ideal_overflow(self):
        args = [[1 << 265,1 << 265],[1 << 265,1 << 265],[0x3f000000]*2,
                0x3f800000,0x3f80,0x3f80,0x3f80,0x4000,0x4000]
        for index,value in [(2,[0x7f800000,0]),(3,0),(4,0),(5,0x7fc0),(6,0x7f80),
                            (7,0x7f80),(0,[True,0]),(0,[1 << 600,0])]:
            bad=copy.deepcopy(args); bad[index]=value
            with self.subTest(index=index), self.assertRaises(ValueError): D.decompose(*bad)
        with self.assertRaises(ValueError): E.dot_units([0x7fc0],[0x3f80])

    def test_signed_zero_and_own_input_counterfactual_is_not_model_feedback(self):
        result = D.decompose([0,0],[0,0],[0x80000000,0x80000000],0,0,0x8000,0,0,0)
        self.assertEqual(result['terms']['observed_projection_difference'],'0')
        self.assertTrue(result['counterfactual_not_model_execution'])
        self.assertEqual(result['own_input_ideal_projection_bf16'],['0000','0000'])

    def test_selected_row_bounds_sparse_delta_and_nonclaims(self):
        captured, *raw = reused_fixture(); reused = D.reuse_r2(captured,*raw)
        weights = {(row,rank): struct.pack('<6144H', *([0x3f80]*6144)) for row in D.ROWS for rank in (0,1)}
        report = D.analyze(captured,reused,weights)
        self.assertEqual(len(report['rows']),15)
        self.assertEqual(report['exact_rank_dot_count'],60)
        self.assertEqual(report['rows'][0]['row'],0)
        self.assertTrue(report['rows'][0]['fixed_matching_control'])
        for key in ('native_fp32_accumulation_tree_emulated','framework_accumulation_emulated',
                    'mismatch_explains_position5_argmax','numerical_acceptance','full_model_acceptance','policy_changed'):
            self.assertFalse(report[key])
        del weights[0,0]
        with self.assertRaises(ValueError): D.analyze(captured,reused,weights)


if __name__ == '__main__':
    unittest.main()
