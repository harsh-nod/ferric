"""Synthetic SiLU adapter tests; no native ownership or hardware result."""
import copy
import hashlib
from pathlib import Path
import struct
import unittest
from unittest.mock import patch

import silu as A
import comparison as M
import test_comparison as P
import diagnostics as D

C, K, T = M.C, M.K, P.T


def fixture():
    f = P.fixture()
    f['corrected'] = copy.deepcopy(f['candidate'])
    f['corrected_pin'] = dict(f['candidate_pin'])
    f['candidate']['request']['layer'].update(session=[7] * 32, evidence_directory='/synthetic/silu')
    f['mlp'] = P.pin('/synthetic/materialized-mlp.hsaco', b'materialized SiLU MLP')
    f['candidate']['request']['layer']['mlp_tiles_image'] = dict(f['mlp'], sha256=list(bytes.fromhex(f['mlp']['sha256'])))
    b = f['candidate']['run']['bootstrap']['layer']
    b['begin']['scope'].update(session=[7] * 32, child_identity=789)
    b['mlp_image'] = {key: f['candidate']['request']['layer']['mlp_tiles_image'][key] for key in ('bytes', 'sha256')}
    f['candidate']['run'].update(child_pid=789, profile_sha256=[8] * 32)
    for i in (1, 2):
        for kind in ('request', 'response'):
            name = f'candidate-{kind}-{i}.json'
            value = C.document(f['candidate_bodies'][name])
            value['profile_sha256'] = [8] * 32
            f['candidate_bodies'][name] = T.raw(value)
    sync(f)
    return f


def sync(f):
    s, bodies = f['candidate'], f['candidate_bodies']
    bodies['request.json'] = T.raw(s['request'])
    bodies['candidate-bootstrap.json'] = T.raw(s['run']['bootstrap'])
    close = C.document(bodies['candidate-response-2.json'])
    close['capture'] = K.part(bodies['candidate-capture.bin'])
    bodies['candidate-response-2.json'] = T.raw(close)
    s['run']['close'] = close
    for row in s['stages']:
        row['sha256'] = list(bytes.fromhex(C.sha(bodies['candidate-capture.bin'][row['offset']:row['offset'] + row['bytes']])))
    s['files'] = [dict(name=name, **K.part(bodies[name])) for name in K.FILES]
    for name, body in bodies.items():
        f['files']['/synthetic/silu/' + name] = body
    body = T.raw(s)
    f['candidate_pin'] = P.pin('/synthetic/silu-summary.json', body)
    f['files'][f['candidate_pin']['path']] = body


def replace(f, rank, name, body):
    row = next(row for row in f['candidate']['stages'] if row['rank'] == rank and row['stage'] == name)
    assert len(body) == row['bytes']
    raw = bytearray(f['candidate_bodies']['candidate-capture.bin'])
    raw[row['offset']:row['offset'] + len(body)] = body
    f['candidate_bodies']['candidate-capture.bin'] = bytes(raw)
    sync(f)


def run(f):
    with patch.object(C, 'EMBEDDING_SHA', C.sha(f['hidden'])):
        return A.compare_retained(*f['pins'][:3], f['corrected_pin'], f['candidate_pin'],
            lambda pin: f['files'][pin['path']], D, candidate_mlp_image_pin=f['mlp'],
            projection_image_pin=f['image'], candidate_worker_pin=f['worker'])


class SiLUComparison(unittest.TestCase):
    def test_reused_arithmetic_and_residual_oracle_are_exact_pinned_sources(self):
        self.assertEqual(hashlib.sha256(Path(M.__file__).read_bytes()).hexdigest(), A.RESIDUAL_ADAPTER_SHA)
        self.assertEqual(hashlib.sha256(Path(A.S.__file__).read_bytes()).hexdigest(), A.SILU_CONTROL_SHA)
        self.assertEqual(M.B.ORACLE_SHA256, '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3')

    def test_complete_synthetic_path_retains_all_diagnostics_without_acceptance(self):
        result = run(fixture())
        self.assertEqual((result['comparable_rows'], len(result['baseline_comparisons']),
                          result['unchanged_pre_swiglu_count'], result['conditional_residual_words']), (24, 24, 22, 16384))
        self.assertEqual(len(result['conditional_residual_comparisons']), 4)
        self.assertTrue(result['conditional_residuals_exact'])
        self.assertTrue(result['framework_product_control_exact'])
        self.assertEqual(len(result['changed_native_stages']), 6)
        for name in ('numerical_acceptance', 'full_model_correctness', 'gpu_execution', 'receipt_authentication',
                     'old_hidden_equality_required', 'materialization_only_cause_proven', 'native_exp_error_measured'):
            self.assertFalse(result[name])
        self.assertIsNone(result['acceptance_threshold'])

    def test_only_selected_mlp_image_and_scoped_identifiers_may_change(self):
        f = fixture()
        A.request(f['candidate'], f['corrected'], f['mlp'], f['image'], f['worker'])
        for key in ('images', 'prefix_tiles_image', 'prompt', 'source', 'device_ids'):
            bad = copy.deepcopy(f['candidate'])
            bad['request']['layer'][key] = 'changed'
            with self.assertRaises(ValueError):
                A.request(bad, f['corrected'], f['mlp'], f['image'], f['worker'])

    def test_old_image_wrong_residual_worker_or_reused_session_are_refused(self):
        f = fixture()
        for field in ('mlp_tiles_image', 'session', 'evidence_directory'):
            bad = copy.deepcopy(f['candidate'])
            bad['request']['layer'][field] = f['corrected']['request']['layer'][field]
            with self.assertRaises(ValueError):
                A.request(bad, f['corrected'], f['mlp'], f['image'], f['worker'])
        for role in ('image', 'worker'):
            changed = dict(f[role], sha256='a' * 64)
            with self.assertRaises(ValueError):
                A.request(f['candidate'], f['corrected'], f['mlp'],
                          changed if role == 'image' else f['image'], changed if role == 'worker' else f['worker'])

    def test_every_pre_swiglu_stage_is_equal_not_only_prefix(self):
        f = fixture()
        parts = M.stage_bytes(f['candidate'], f['candidate_bodies']['candidate-capture.bin'], 7, D)
        for rank in (0, 1):
            for name, _, width in K.STAGES[:11]:
                changed = copy.deepcopy(parts)
                index = 7 * 16 * 1024 if name in ('key-cache', 'value-cache') else 0
                body = bytearray(changed[rank][name]); body[index:index + width] = bytes([1]) * width
                changed[rank][name] = bytes(body)
                with self.assertRaises(ValueError):
                    A.unchanged_upstream(parts, changed, 7, 7)

    def test_logical_kv_row_can_move_between_authenticated_physical_pages(self):
        f = fixture()
        parts = M.stage_bytes(f['candidate'], f['candidate_bodies']['candidate-capture.bin'], 7, D)
        old, new = copy.deepcopy(parts), copy.deepcopy(parts)
        for rank in (0, 1):
            for name in ('key-cache', 'value-cache'):
                for values, page in ((old, 7), (new, 9)):
                    raw = bytearray(values[rank][name]); start = page * 16 * 1024
                    raw[start:start + 1024] = b'\x80\x3f' * 512
                    values[rank][name] = bytes(raw)
        self.assertEqual(len(A.unchanged_upstream(old, new, 7, 9)), 22)

    def test_changed_activation_mismatches_remain_visible_and_conditional(self):
        f = fixture()
        replace(f, 0, 'activation', b'\x80\x3f' * 6144)
        result = run(f)
        row = result['silu_product_controls']['candidate']['ranks'][0]
        self.assertEqual(row['native_different_materialized'], 6144)
        self.assertEqual(result['earliest_observable_divergence']['stage'], 'activation-product')
        self.assertFalse(result['materialization_only_cause_proven'])

    def test_changed_down_and_final_hidden_use_actual_candidate_residual(self):
        f = fixture()
        replace(f, 0, 'down-partial', struct.pack('<4096I', *([0x3f800000] * 4096)))
        for rank in (0, 1):
            replace(f, rank, 'final-hidden', b'\x80\x3f' * 4096)
        result = run(f)
        self.assertTrue(result['conditional_residuals_exact'])
        self.assertFalse(next(row for row in result['comparisons'] if row['stage'] == 'final-hidden')['byte_equal'])
        partial = next(row for row in result['changed_native_stages'] if row['stage'] == 'down-partial' and row['rank'] == 0)
        self.assertEqual(partial['max_absolute_error'], 1.0)
        self.assertFalse(partial['independent_reference'])
        self.assertFalse(partial['compared_to_full_framework_bf16'])

    def test_incorrect_residual_output_is_reported_not_reclassified_as_success(self):
        f = fixture()
        for rank in (0, 1):
            replace(f, rank, 'final-hidden', b'\x80\x3f' * 4096)
        result = run(f)
        self.assertFalse(result['conditional_residuals_exact'])
        self.assertFalse(result['numerical_acceptance'])

    def test_nonfinite_capture_partial_is_rejected_before_metrics(self):
        for word in (0x7f800000, 0x7fc00000):
            f = fixture()
            replace(f, 0, 'down-partial', struct.pack('<4096I', *([word] * 4096)))
            with self.assertRaises(ValueError):
                run(f)

    def test_fp32_partial_changes_distinguish_signed_zero_and_subnormals(self):
        zeros = b'\0' * 16384
        negative = struct.pack('<4096I', *([0x80000000] * 4096))
        row = A.partial_difference(zeros, negative, 0)
        self.assertFalse(row['byte_equal'])
        self.assertEqual((row['exact_elements'], row['max_absolute_error'], row['relative_l2_error']), (0, 0.0, 0.0))
        tiny = struct.pack('<4096I', *([1] * 4096))
        row = A.partial_difference(zeros, tiny, 0)
        self.assertGreater(row['max_absolute_error'], 0)
        self.assertIsNone(row['relative_l2_error'])

    def test_partial_extent_and_nonfinite_inputs_are_refused(self):
        zeros = b'\0' * 16384
        for body in (zeros[:-4], struct.pack('<4096I', *([0x7f800000] * 4096))):
            with self.assertRaises(ValueError):
                A.partial_difference(zeros, body, 1)


if __name__ == '__main__':
    unittest.main()
