"""Synthetic custody/mapping tests; original integer boundary tests remain separate."""
import copy
import hashlib
import json
import struct
import unittest

import position5 as P


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def fixture():
    native, parts = bytearray(), []
    for rank in (0, 1):
        for role, (boundary, scalar, size) in P.NATIVE.items():
            value = 0x3f000000 if scalar == 'f32' else (0x4000 if role == 'final_hidden' else 0x3f80)
            raw = struct.pack('<4096' + ('I' if scalar == 'f32' else 'H'), *([value] * 4096))
            parts.append(dict(rank=rank, role=role, boundary=boundary, scalar=scalar,
                elements=4096, source_byte_offset=0, offset=len(native), **pin(raw)))
            native.extend(raw)
    for i in range(28):
        raw = b'\0\0'
        parts.append(dict(rank=i % 2, role='other%d' % i, offset=len(native), **pin(raw)))
        native.extend(raw)
    nc = dict(position=5, generation=6, layer=0, parts=parts,
              payload_bytes=len(native), payload_sha256=pin(native)['sha256'])
    framework, fp = bytearray(), []
    for name in P.FRAMEWORK:
        raw = struct.pack('<4096H', *([0x4000 if name == 'layer0-hidden' else 0x3f80] * 4096))
        fp.append(dict(name=name, dtype='bfloat16', shape=[1, 1, 4096],
                       offset=len(framework), **pin(raw)))
        framework.extend(raw)
    for i in range(29):
        raw = b'\0\0'
        fp.append(dict(name='other%d' % i, offset=len(framework), **pin(raw)))
        framework.extend(raw)
    fc = dict(position=5, input_token=271, parts=fp)
    rows = [dict(id='unused%d' % i) for i in range(200)]
    for rank in (0, 1):
        for role, name in (('first_residual', 'first-residual'), ('final_hidden', 'layer0-hidden')):
            n = next(p for p in parts if p['rank'] == rank and p['role'] == role)
            f = next(p for p in fp if p['name'] == name)
            rows.append(dict(id='5:%d:%s' % (rank, role), position=5, input_token=271, rank=rank, role=role,
                native={k: n[k] for k in ('bytes', 'sha256')},
                reference={k: f[k] for k in ('bytes', 'sha256')}))
    return nc, bytes(native), fc, bytes(framework), rows


def envelope(magic, header, payload):
    value = json.dumps(header, separators=(',', ':'), allow_nan=False).encode()
    return magic + struct.pack('<II', len(value), len(payload)) + value + payload


def complete_fixture():
    nc, native, fc, framework, rows = fixture()
    natives, references = [], []
    for position in range(6):
        a, b = copy.deepcopy(nc), copy.deepcopy(fc)
        a.update(position=position, generation=position + 1)
        b['position'] = position
        for part in b['parts']:
            part['offset'] += position * len(framework)
        natives.append(a); references.append(b)
    np, fp = native * 6, framework * 6
    nh = dict(captures=natives, payload_bytes=len(np), payload_sha256=pin(np)['sha256'],
              native_close_confirmed=True)
    fh = dict(captures=references, payload=pin(fp), ordinal=1)
    rh = dict(fh, ordinal=2)
    comparison = dict(checks=dict(both_same_side_parity_gates_rechecked=True,
                                 diagnostic=dict(comparable_rows=rows)))
    return (envelope(b'FCAP061\0', nh, np), envelope(b'FREF061\0', fh, fp),
            envelope(b'FREF061\0', rh, fp), comparison)


class Position5Tests(unittest.TestCase):
    def test_full4096_rows_own_operands_and_separate_framework_control(self):
        captured = P.extract(*complete_fixture())
        report, rows, outputs = P.analyze(captured)
        self.assertTrue(report['all_observed_boundaries_exact'])
        self.assertEqual(report['native_matched_words'], 8192)
        self.assertEqual(report['framework_matched_words'], 4096)
        self.assertEqual(len(rows['native']['rows']), 4096)
        self.assertEqual(len(report['selected_parts']), 10)
        self.assertEqual(report['native_reference_different_rows'], [[], []])
        self.assertEqual(outputs['derived-down.bf16'], b'\x80\x3f' * 4096)
        for key in ('dedicated_down_dot_products_replayed', 'gate_up_or_silu_replayed',
                    'mismatch_explains_position5_argmax', 'numerical_acceptance',
                    'full_model_acceptance', 'semantic_bug_claimed', 'gpu_execution'):
            self.assertFalse(report[key])

    def test_wrong_position_generation_layer_and_boolean_rank_refuse(self):
        original = fixture()
        for key, value in (('position', 4), ('generation', 5), ('layer', 1), ('layer', False)):
            values = copy.deepcopy(original); values[0][key] = value
            with self.assertRaises(ValueError): P.select(*values)
        values = copy.deepcopy(original); values[0]['parts'][0]['rank'] = True
        with self.assertRaises(ValueError): P.select(*values)
        values = copy.deepcopy(original); values[2]['position'] = 4
        with self.assertRaises(ValueError): P.select(*values)
        values = copy.deepcopy(original); values[2]['input_token'] = 272
        with self.assertRaises(ValueError): P.select(*values)

    def test_missing_duplicate_scalar_boundary_extent_and_gap_refuse(self):
        changes = (
            lambda c: c['parts'].pop(),
            lambda c: c['parts'].__setitem__(1, copy.deepcopy(c['parts'][0])),
            lambda c: c['parts'][0].__setitem__('scalar', 'f32'),
            lambda c: c['parts'][0].__setitem__('boundary', 'after_mlp'),
            lambda c: c['parts'][0].__setitem__('elements', True),
            lambda c: c['parts'][0].__setitem__('source_byte_offset', 1),
            lambda c: c['parts'][0].__setitem__('bytes', 8191),
            lambda c: c['parts'][1].__setitem__('offset', 1),
        )
        for change in changes:
            values = copy.deepcopy(fixture()); change(values[0])
            with self.assertRaises(ValueError): P.select(*values)
        for key, value in (('dtype', 'float32'), ('shape', [True, 1, 4096]), ('name', 'other0')):
            values = copy.deepcopy(fixture()); values[2]['parts'][0][key] = value
            with self.assertRaises(ValueError): P.select(*values)

    def test_body_part_and_comparison_hash_drift_refuse(self):
        values = list(fixture())
        values[1] = b'\1' + values[1][1:]
        with self.assertRaises(ValueError): P.select(*values)
        values[0]['payload_sha256'] = pin(values[1])['sha256']
        with self.assertRaises(ValueError): P.select(*values)
        values = list(fixture()); values[3] = b'\1' + values[3][1:]
        with self.assertRaises(ValueError): P.select(*values)
        values = copy.deepcopy(fixture()); values[4][-1]['native']['sha256'] = '0' * 64
        with self.assertRaises(ValueError): P.select(*values)
        values = copy.deepcopy(fixture()); values[4][-1]['reference']['bytes'] -= 2
        with self.assertRaises(ValueError): P.select(*values)

    def test_repeat_complete_capture_pin_and_prior_admission_refuse(self):
        values = list(complete_fixture()); values[2] = values[1]
        with self.assertRaises(ValueError): P.extract(*values)
        values = list(complete_fixture()); raw = bytearray(values[0]); raw[-1] ^= 1; values[0] = bytes(raw)
        with self.assertRaises(ValueError): P.extract(*values)
        values = list(complete_fixture()); values[3]['checks']['both_same_side_parity_gates_rechecked'] = False
        with self.assertRaises(ValueError): P.extract(*values)
        values = list(complete_fixture())
        header, payload = P.Q.envelope(values[0], b'FCAP061\0')
        header['native_close_confirmed'] = False
        values[0] = envelope(b'FCAP061\0', header, payload)
        with self.assertRaises(ValueError): P.extract(*values)

    def test_observed_output_error_is_reported_without_acceptance(self):
        captured = P.extract(*complete_fixture())
        raw = captured['native'][0, 'final_hidden']
        captured['native'][0, 'final_hidden'] = b'\1\x40' + raw[2:]
        report, rows, _ = P.analyze(captured)
        self.assertFalse(report['all_observed_boundaries_exact'])
        self.assertEqual(report['native_mismatch_rows'], [0])
        self.assertEqual(report['native_matched_words'], 8191)
        self.assertTrue(report['framework_boundary_exact'])
        self.assertFalse(report['semantic_bug_claimed'])
        self.assertFalse(report['numerical_acceptance'])
        self.assertEqual(len(rows['native']['rows']), 4096)


if __name__ == '__main__':
    unittest.main()
