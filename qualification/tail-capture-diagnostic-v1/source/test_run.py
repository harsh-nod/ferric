"""Synthetic adapter/policy tests; not a numerical qualification of real captures."""
import copy
import hashlib
import math
import os
import struct
import types
import unittest
from unittest.mock import patch

import numpy as np

import run as R


def digest(sha):
    return list(bytes.fromhex(sha))


def uploads():
    return dict(version=1, uploads=[
        dict(key=dict(kind='source', rank=0, id=978), bytes=R.NORM[0], sha256=digest(R.NORM[1])),
        dict(key=dict(kind='source', rank=0, id=979), bytes=R.HEAD[0], sha256=digest(R.HEAD[1]))],
        tail=dict(source=dict(kind='source', rank=0, id=979), source_sha256=digest(R.HEAD[1]),
                  bytes=R.HEAD[0], sha256=digest(R.TRANSPOSE_SHA)))


def program():
    def buffer(source, access, elements, width=2):
        return dict(kind='buffer', source_id=source, access=access,
                    elements=elements, element_bytes=width, offset=0)
    scalar = lambda value, kind='u32': dict(kind=kind, value=value)
    steps = []
    for symbol, grid, args in [
        ('qwen3_rmsnorm_v1', 1, [buffer(1490, 'read', 4096), buffer(73, 'read', 0),
            buffer(978, 'read', 4096), buffer(73, 'write', 0), buffer(75, 'write', 4096),
            scalar(1), scalar(4096), scalar(897988541, 'f32_bits'), scalar(0)]),
        ('ferric_qwen3_tp_mfma_gemm_bf16_v3', 9496, [buffer(75, 'read', 65536),
            buffer(1488, 'read', 622329856), buffer(91, 'write', 2430976),
            *map(scalar, (1, 151936, 4096, 2, 6))]),
        ('ferric_qwen3_tp_batch_argmax_bf16_v2', 1,
            [buffer(91, 'read', 2430976), buffer(92, 'write', 16, 4), scalar(1)]),
    ]:
        steps.append(dict(kind='rank', rank=0, dispatch=dict(symbol=symbol, grid_workgroups=grid,
                                                            workgroup_size=64, arguments=args)))
    return dict(schema='ferric-finite-source-grammar-v1', steps=steps)


def fake_index():
    shapes = {**R.TENSOR_SHAPES, **{f'other{n}': (2,) for n in range(1, 4)}}
    model = types.SimpleNamespace(tensor_shapes=lambda _: shapes, QWEN8B=None, math=math)
    index = dict(metadata=dict(total_size=sum(2 * math.prod(shape) for shape in shapes.values())),
        weight_map={**R.TENSOR_SHARDS,
                    **{f'other{n}': f'model-{n:05}-of-00005.safetensors' for n in range(1, 4)}})
    return model, index


class FakeDiagnostics:
    """Small test double for the separately pinned and already tested capture reader."""
    def validate_case(self, record, raw, position):
        R.require(type(raw) is bytes and len(raw) == 606976, 'capture extent')
        words = np.frombuffer(raw, dtype='<u2')
        R.require(np.all(words & 0x7f80 != 0x7f80), 'capture finite')
        R.require(record['position'] == position and 0 <= position < 4, 'capture position')
        return {name: raw[offset:offset + count] for name, (offset, count) in R.CAPTURE_LAYOUT.items()}

    def compare_tensor(self, expected, actual):
        R.require(len(expected) == len(actual), 'comparison geometry')
        left, right = np.frombuffer(expected, dtype='<u2'), np.frombuffer(actual, dtype='<u2')
        return dict(elements=len(left), exact_words=int(np.count_nonzero(left == right)))


def synthetic_case():
    raw = bytearray(606976)
    hidden = np.full(4096, 0x3f80, dtype='<u2')
    captured_norm = np.full(4096, 0x4000, dtype='<u2')
    logits = np.zeros(151936, dtype='<u2')
    logits[3] = 0x4040
    for name, value in [('layer35-hidden', hidden), ('final-norm', captured_norm), ('logits', logits)]:
        offset, count = R.CAPTURE_LAYOUT[name]
        raw[offset:offset + count] = value.tobytes()
    return dict(generation=1, position=0, input_token=9112, output_token=3), bytes(raw)


def fake_reference(logits_index=4):
    calls = []
    def norm(helpers, actual, weight):
        calls.append(('norm', actual.copy(), weight))
        return np.full(4096, 0x4080, dtype='<u2')
    def project(mlp, weight, actual):
        calls.append(('head', actual.copy(), weight))
        return np.arange(151936, dtype='<f8')
    def narrow(mlp, values):
        calls.append(('narrow', values.shape))
        result = np.zeros(151936, dtype='<u2')
        result[logits_index] = 0x4080
        return result
    def choose(words, mlp):
        values = (words.astype('<u4') << 16).view('<f4')
        R.require(np.isfinite(values).all(), 'choice finite')
        return int(np.argmax(values))
    model = types.SimpleNamespace(np=np, dense_norm=norm, choose=choose)
    helpers = (types.SimpleNamespace(dense_project=project, narrow_projection=narrow),
               object(), None, None)
    return model, helpers, calls


class LayoutTests(unittest.TestCase):
    def test_nxk_and_transpose_hash_exact_order(self):
        weights = np.array([[1, 2, 3], [4, 5, 6]], dtype='<u2')
        original = R.digest_layout(np, weights, block=1)
        transposed = R.digest_layout(np, weights, transpose=True, block=1)
        self.assertEqual(original, dict(bytes=12, sha256=hashlib.sha256(struct.pack('<6H', 1, 2, 3, 4, 5, 6)).hexdigest()))
        self.assertEqual(transposed, dict(bytes=12, sha256=hashlib.sha256(struct.pack('<6H', 1, 4, 2, 5, 3, 6)).hexdigest()))
        self.assertNotEqual(original, transposed)

    def test_transpose_hash_chunk_boundary_invariant(self):
        words = np.arange(35, dtype='<u2').reshape(5, 7)
        expected = R.digest_layout(np, words, transpose=True, block=1)
        for block in (2, 3, 16, 128):
            self.assertEqual(R.digest_layout(np, words, transpose=True, block=block), expected)

    def test_noncontiguous_view_preserves_logical_order(self):
        words = np.arange(30, dtype='<u2').reshape(5, 6)[:, ::2]
        actual = R.digest_layout(np, words)
        self.assertEqual(actual['sha256'], hashlib.sha256(words.copy(order='C').tobytes()).hexdigest())

    def test_rejects_invalid_layout_dtype_extent_and_block(self):
        for words, transpose, block in [(np.zeros(2, dtype='<f4'), False, 16),
                (np.zeros(0, dtype='<u2'), False, 16), (np.zeros(2, dtype='<u2'), True, 16),
                (np.zeros((1, 1, 1), dtype='<u2'), False, 16),
                (np.zeros(2, dtype='<u2'), False, 0), (np.zeros(2, dtype='<u2'), False, True),
                (np.zeros(2, dtype='<u2'), False, 129)]:
            with self.subTest(shape=words.shape, transpose=transpose, block=block), self.assertRaises(ValueError):
                R.digest_layout(np, words, transpose, block)


class ProvenanceTests(unittest.TestCase):
    def test_uploads_keep_original_and_transposed_distinct(self):
        found, tail = R.tail_uploads(uploads())
        self.assertEqual(found[R.HEAD_NAME]['key']['id'], 979)
        self.assertNotEqual(found[R.HEAD_NAME]['sha256'], tail['sha256'])

    def test_uploads_reject_original_as_transposed_digest(self):
        value = uploads()
        value['tail']['sha256'] = value['tail']['source_sha256']
        with self.assertRaisesRegex(ValueError, 'transposed'):
            R.tail_uploads(value)

    def test_uploads_reject_duplicate_wrong_source_and_extent(self):
        for mutation in ('duplicate', 'source', 'extent'):
            value = uploads()
            if mutation == 'duplicate':
                value['uploads'].append(copy.deepcopy(value['uploads'][0]))
            elif mutation == 'source':
                value['tail']['source']['id'] = 1488
            else:
                value['uploads'][0]['bytes'] -= 2
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                R.tail_uploads(value)

    def test_byte_digest_rejects_bool_and_bad_extent(self):
        for value in ([False] * 32, [0] * 31, [256] * 32, '00' * 32):
            with self.subTest(value=value), self.assertRaises(ValueError):
                R.byte_digest(value)

    def test_new_bootstrap_join_requires_exact_size_and_content(self):
        pin = dict(path='/retained/old.json', bytes=R.UPLOADS[0], sha256=R.UPLOADS[1])
        row = dict(bytes=R.UPLOADS[0], sha256=digest(R.UPLOADS[1]))
        R.join_bootstrap(row, pin)
        for changed in (dict(row, bytes=row['bytes'] - 1), dict(row, sha256=[0] * 32)):
            with self.assertRaises(ValueError):
                R.join_bootstrap(changed, pin)

    def test_exact_actual_tail_dispatch_and_head_source(self):
        result = R.tail_program(program())
        self.assertEqual(result['original_head_source_id'], 979)
        self.assertEqual(result['uploaded_head_source_id'], 1488)
        self.assertEqual(result['head_scalars'], [1, 151936, 4096, 2, 6])

    def test_tail_dispatch_rejects_wrong_source_epsilon_and_world_size(self):
        for step, argument, key, replacement in [(0, 7, 'value', 0), (1, 1, 'source_id', 979),
                                                 (1, 6, 'value', 1)]:
            value = program()
            value['steps'][step]['dispatch']['arguments'][argument][key] = replacement
            with self.subTest(step=step, argument=argument), self.assertRaises(ValueError):
                R.tail_program(value)

    def test_original_index_selects_shards_four_and_five(self):
        model, index = fake_index()
        self.assertEqual(R.tail_index(model, index)[R.HEAD_NAME], (151936, 4096))

    def test_original_index_rejects_wrong_shard_shape_and_byte_total(self):
        for mutation in ('shard', 'shape', 'total'):
            model, index = fake_index()
            if mutation == 'shard':
                index['weight_map'][R.NORM_NAME] = 'model-00001-of-00005.safetensors'
            elif mutation == 'shape':
                model.tensor_shapes(None)[R.HEAD_NAME] = (4096, 151936)
            else:
                index['metadata']['total_size'] += 2
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                R.tail_index(model, index)


class ConditionalTests(unittest.TestCase):
    def test_reference_receives_separate_captured_immediate_inputs(self):
        model, helpers, calls = fake_reference()
        record, raw = synthetic_case()
        norm_weight, head_weight = object(), object()
        row, expected = R.conditional_tail(model, helpers, FakeDiagnostics(), record, raw, norm_weight, head_weight)
        self.assertTrue(np.all(calls[0][1] == 0x3f80))
        self.assertTrue(np.all(calls[1][1] == 0x4000))
        self.assertIs(calls[0][2], norm_weight)
        self.assertIs(calls[1][2], head_weight)
        self.assertEqual(calls[2], ('narrow', (151936,)))
        self.assertEqual(len(expected['final-norm']), 8192)
        self.assertEqual(len(expected['logits']), 303872)
        self.assertFalse(row['numerical_acceptance'])

    def test_different_reference_token_is_reported_not_accepted_or_rejected(self):
        model, helpers, _ = fake_reference(logits_index=4)
        record, raw = synthetic_case()
        row, _ = R.conditional_tail(model, helpers, FakeDiagnostics(), record, raw, None, None)
        self.assertEqual(row['captured_output_token'], 3)
        self.assertEqual(row['reference_head_output_token'], 4)
        self.assertFalse(row['reference_head_token_equal'])
        self.assertIsNone(row['acceptance_threshold'])
        self.assertEqual(row['tensors']['final-norm']['exact_words'], 0)

    def test_captured_token_disagreement_fails(self):
        model, helpers, _ = fake_reference()
        record, raw = synthetic_case()
        record['output_token'] = 4
        with self.assertRaisesRegex(ValueError, 'captured lowest-index'):
            R.conditional_tail(model, helpers, FakeDiagnostics(), record, raw, None, None)

    def test_capture_extent_nonfinite_and_offset_corruption_fail(self):
        model, helpers, _ = fake_reference()
        record, raw = synthetic_case()
        bad = bytearray(raw)
        bad[286720:286722] = struct.pack('<H', 0x7f80)
        for payload in (raw[:-2], bytes(bad)):
            with self.assertRaises(ValueError):
                R.conditional_tail(model, helpers, FakeDiagnostics(), record, payload, None, None)
        diagnostics = FakeDiagnostics()
        original = diagnostics.validate_case
        def shifted(*args):
            rows = original(*args)
            rows['layer35-hidden'] = rows['final-norm']
            return rows
        diagnostics.validate_case = shifted
        with self.assertRaisesRegex(ValueError, 'offsets'):
            R.conditional_tail(model, helpers, diagnostics, record, raw, None, None)

    def test_reference_failure_is_not_hidden(self):
        model, helpers, _ = fake_reference()
        record, raw = synthetic_case()
        def fail(*args):
            raise ValueError('original reference failed')
        helpers[0].dense_project = fail
        with self.assertRaisesRegex(ValueError, 'original reference failed'):
            R.conditional_tail(model, helpers, FakeDiagnostics(), record, raw, None, None)

    def test_wrong_reference_geometry_fails(self):
        model, helpers, _ = fake_reference()
        record, raw = synthetic_case()
        helpers[0].narrow_projection = lambda *_: np.zeros(151935, dtype='<u2')
        with self.assertRaisesRegex(ValueError, 'output geometry'):
            R.conditional_tail(model, helpers, FakeDiagnostics(), record, raw, None, None)


class PolicyTests(unittest.TestCase):
    def test_success_is_diagnostic_only(self):
        result = R.nonclaims()
        self.assertTrue(result['passed'])
        self.assertEqual(result['authority'], 'none')
        self.assertEqual(result['status'], 'CONDITIONAL_TAIL_DIAGNOSTICS_ONLY')
        self.assertIsNone(result['acceptance_threshold'])
        self.assertTrue(all(result[key] is False for key in R.FALSE))

    def test_nonempty_pythonoptimize_rejected_even_zero(self):
        with patch.dict(os.environ, {'PYTHONOPTIMIZE': '0'}):
            with self.assertRaisesRegex(ValueError, 'ordinary Python'):
                R.ordinary_python()

    def test_optimized_interpreter_rejected_without_environment(self):
        with patch.dict(os.environ, {}, clear=True), patch.object(R.sys, 'flags', types.SimpleNamespace(optimize=1)):
            with self.assertRaisesRegex(ValueError, 'ordinary Python'):
                R.ordinary_python()


if __name__ == '__main__':
    unittest.main()
