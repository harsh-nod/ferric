"""Synthetic adapter/oracle tests; no native ownership or GPU result is minted."""
import copy
import hashlib
from pathlib import Path
import struct
import unittest
from unittest.mock import patch

import comparison as M
import current as K
import compare as C
import diagnostics as D
import boundary as B
import test_current as T


def pin(path, body):
    return dict(path=path, bytes=len(body), sha256=C.sha(body))


def fixture():
    f = T.fixture()
    f['image'] = pin('/synthetic/projection.hsaco', b'new residual image')
    f['worker'] = pin('/synthetic/new-worker', b'new worker')
    s = copy.deepcopy(f['summary'])
    layer = s['request']
    layer.update(evidence_directory='/synthetic/candidate', session=[5] * 32,
                 worker=dict(f['worker'], sha256=list(bytes.fromhex(f['worker']['sha256']))))
    image = dict(f['image'], sha256=list(bytes.fromhex(f['image']['sha256'])))
    s['request'] = dict(schema=M.REQUEST, layer=layer, projection_residual_image=image)
    s['schema'] = M.OBSERVATION
    inner = s['run']['bootstrap']
    inner['begin']['scope'].update(session=layer['session'], child_identity=456)
    s['run'].update(child_pid=456, profile_sha256=[6] * 32,
        bootstrap=dict(schema=M.BOOTSTRAP, layer=inner,
                       projection_residual_image=dict(bytes=image['bytes'], sha256=image['sha256'])))
    bodies = dict(f['bodies'])
    for ordinal in (1, 2):
        for kind in ('request', 'response'):
            name = f'candidate-{kind}-{ordinal}.json'
            value = C.document(bodies[name]); value['profile_sha256'] = [6] * 32
            bodies[name] = T.raw(value)
    f.update(candidate=s, candidate_bodies=bodies)
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
    for name, body in bodies.items(): f['files']['/synthetic/candidate/' + name] = body
    body = T.raw(s)
    f['candidate_pin'] = pin('/synthetic/candidate-summary.json', body)
    f['files'][f['candidate_pin']['path']] = body


def replace_stage(f, rank, name, body):
    row = next(r for r in f['candidate']['stages'] if r['rank'] == rank and r['stage'] == name)
    assert len(body) == row['bytes']
    raw = bytearray(f['candidate_bodies']['candidate-capture.bin'])
    raw[row['offset']:row['offset'] + len(body)] = body
    f['candidate_bodies']['candidate-capture.bin'] = bytes(raw)
    sync(f)


def parts(f):
    return M.stage_bytes(f['candidate'], f['candidate_bodies']['candidate-capture.bin'], 7, D)


def run(f):
    with patch.object(C, 'EMBEDDING_SHA', C.sha(f['hidden'])), patch.object(K, 'CURRENT_HIDDEN_SHA', C.sha(f['hidden'])):
        return M.compare_retained(*f['pins'], f['candidate_pin'], lambda p: f['files'][p['path']], D,
            candidate_image_pin=f['image'], candidate_worker_pin=f['worker'])


class ProjectionResidualComparison(unittest.TestCase):
    def test_unchanged_helpers_and_test_fixture_are_pinned(self):
        for module, expected in ((K, M.CURRENT_SHA), (C, K.COMPARE_SHA), (D, C.DIAGNOSTICS_SHA),
                (B, M.BOUNDARY_SHA), (T, '55260e410cb601e9af38cb6e7c35c6c153c7a0f7d11269a40d581cad1c56b3b4')):
            self.assertEqual(hashlib.sha256(Path(module.__file__).read_bytes()).hexdigest(), expected)
        self.assertEqual(B.ORACLE_SHA256, '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3')

    def test_complete_path_has_four_oracle_and_twenty_four_framework_rows(self):
        result = run(fixture())
        self.assertEqual((result['comparable_rows'], len(result['conditional_residual_comparisons']),
                          result['conditional_residual_words'], result['unchanged_pre_residual_count']), (24, 4, 16384, 14))
        self.assertTrue(result['conditional_residuals_exact'])
        self.assertIsNone(result['earliest_observable_divergence'])
        self.assertEqual(len(result['excluded_fp32_partials']), 4)
        for name in ('receipt_authentication', 'numerical_acceptance', 'full_layer_numerics_accepted',
                     'full_model_correctness', 'gpu_execution', 'performance_measured', 'production_authority',
                     'ordering_is_a_causal_proof', 'upstream_partial_numerics_checked', 'old_hidden_equality_required'):
            self.assertFalse(result[name], name)
        self.assertIsNone(result['acceptance_threshold'])

    def test_new_worker_and_changed_hidden_are_allowed_without_old_hidden_override(self):
        f = fixture()
        replace_stage(f, 0, 'down-partial', struct.pack('<4096I', *([0x3f800000] * 4096)))
        for rank in (0, 1): replace_stage(f, rank, 'final-hidden', b'\x80\x3f' * 4096)
        result = run(f)
        self.assertTrue(result['conditional_residuals_exact'])
        self.assertFalse(result['candidate_hidden_matches_old_tf4'])
        self.assertEqual(result['earliest_observable_divergence']['stage'], 'final-hidden')

    def test_original_source_images_prompt_and_selected_worker_cannot_drift(self):
        for key in ('source', 'images', 'prompt', 'mlp_tiles_image', 'device_ids'):
            f = fixture(); f['candidate']['request']['layer'][key] = 'changed'
            with self.assertRaises(ValueError): M.candidate_request(f['candidate'], f['summary'], f['image'], f['worker'])
        f = fixture(); worker = dict(f['worker'], sha256='a' * 64)
        with self.assertRaises(ValueError): M.candidate_request(f['candidate'], f['summary'], f['image'], worker)

    def test_distinct_image_and_outer_schemas_are_required(self):
        for which in range(4):
            f = fixture()
            if which == 0: f['candidate']['request']['schema'] = 'old'
            elif which == 1: f['candidate']['request']['projection_residual_image'] = f['summary']['request']['images']['residual']
            elif which == 2: f['candidate']['request']['extra'] = 1
            else: f['image']['sha256'] = 'b' * 64
            with self.assertRaises(ValueError): M.candidate_request(f['candidate'], f['summary'], f['image'], f['worker'])

    def test_actual_outer_request_bootstrap_and_close_bytes_are_joined(self):
        for kind in ('request', 'bootstrap', 'close'):
            f = fixture()
            (f['candidate'] if kind == 'request' else f['candidate']['run'])[kind]['extra'] = 1
            with self.assertRaises(ValueError): M.native_files(f['candidate'], lambda p: f['files'][p['path']])

    def test_exact_eleven_files_and_full_capture_pin_are_required(self):
        for which in range(3):
            f = fixture()
            if which == 0: f['candidate']['files'].pop()
            elif which == 1: f['candidate']['files'][1]['name'] = 'other.json'
            else: f['candidate']['files'][9]['sha256'] = [0] * 32
            with self.assertRaises(ValueError): M.native_files(f['candidate'], lambda p: f['files'][p['path']])

    def test_bootstrap_extra_image_own_scope_and_run_close_are_checked(self):
        f = fixture(); self.assertEqual(M.bootstrap_page(f['candidate'], f['candidate_bodies']), 7)
        for which in range(4):
            f = fixture()
            if which == 0: f['candidate']['run']['bootstrap']['projection_residual_image']['bytes'] += 1
            elif which == 1: f['candidate']['run']['child_pid'] += 1
            elif which == 2: f['candidate']['run']['bootstrap']['layer']['input']['token'] = 785
            else:
                value = C.document(f['candidate_bodies']['candidate-request-2.json'])
                value['profile_sha256'] = [4] * 32
                f['candidate_bodies']['candidate-request-2.json'] = T.raw(value)
            with self.assertRaises(ValueError): M.bootstrap_page(f['candidate'], f['candidate_bodies'])

    def test_all_stage_offsets_digests_and_nonfinite_values_are_checked(self):
        f = fixture(); f['candidate']['stages'][0]['offset'] += 1
        with self.assertRaises(ValueError): parts(f)
        f = fixture(); f['candidate']['stages'][0]['sha256'] = [0] * 32
        with self.assertRaises(ValueError): parts(f)
        for name, width, invalid in (('gate', 2, 0x7fc0), ('down-partial', 4, 0x7f800000)):
            f = fixture(); row = next(r for r in f['candidate']['stages'] if r['stage'] == name)
            replace_stage(f, 0, name, struct.pack('<H' if width == 2 else '<I', invalid) + b'\0' * (row['bytes'] - width))
            with self.assertRaises(ValueError): parts(f)

    def test_full_kv_outside_current_slot_and_rank_residual_disagreement_fail(self):
        for name in ('key-cache', 'value-cache', 'first-residual', 'final-hidden'):
            f = fixture(); row = next(r for r in f['candidate']['stages'] if r['stage'] == name)
            replace_stage(f, 0, name, b'\x01\0' + b'\0' * (row['bytes'] - 2))
            with self.assertRaises(ValueError): parts(f)

    def test_different_physical_pages_compare_logical_slot_not_whole_cache(self):
        f = fixture(); left, right = parts(f), parts(f)
        for rank in (0, 1):
            for name in ('key-cache', 'value-cache'):
                for values, page in ((left, 7), (right, 11)):
                    body = bytearray(values[rank][name]); body[page * 16 * 1024:page * 16 * 1024 + 1024] = b'\x80\x3f' * 512
                    values[rank][name] = bytes(body)
        self.assertEqual(len(M.unchanged_prefix(left, right, 7, 11)), 14)
        with self.assertRaises(ValueError): M.unchanged_prefix(left, right, 7, 7)

    def test_any_prefix_or_output_partial_change_is_refused(self):
        f = fixture(); left, right = parts(f), parts(f)
        for name, _, _ in K.STAGES[:7]:
            bad = copy.deepcopy(right)
            start = 7 * 16 * 1024 if name in ('key-cache', 'value-cache') else 0
            body = bytearray(bad[1][name]); body[start] = 1; bad[1][name] = bytes(body)
            with self.assertRaises(ValueError): M.unchanged_prefix(left, bad, 7, 7)

    def test_materialization_tie_occurs_before_residual(self):
        oracle = B.load_oracle()
        projection, output = M.materialized_word(oracle, 0x3f808000, 0, 0xbc00)
        self.assertEqual((projection, output), (0x3f80, 0x3f7e))
        self.assertNotEqual(output, oracle.staged_residual_bits(0x3f808000, 0, 0xbc00)[3])

    def test_rank_partials_are_not_individually_narrowed(self):
        oracle = B.load_oracle()
        projection, output = M.materialized_word(oracle, 0x3f808000, 0x3b800000, 0)
        self.assertEqual((projection, output), (0x3f81, 0x3f81))
        wrong = oracle.narrow_bf16_rne(oracle.add_f32_rne(oracle.narrow_bf16_rne(0x3f808000) << 16,
                                                       oracle.narrow_bf16_rne(0x3b800000) << 16))
        self.assertNotEqual(projection, wrong)

    def test_leading_zero_cancellation_and_tiny_rounding_match_exact_oracle(self):
        oracle = B.load_oracle()
        self.assertEqual(M.materialized_word(oracle, 0x80000000, 0x80000000, 0x8000), (0, 0))
        self.assertEqual(M.materialized_word(oracle, 0x3f800000, 0xbf800000, 0), (0, 0))
        self.assertEqual(M.materialized_word(oracle, 1, 0, 0), (0, 0))
        self.assertEqual(M.materialized_word(oracle, 0x10000, 0, 0), (1, 1))

    def test_projection_overflow_refuses_before_possible_residual_cancellation(self):
        oracle = B.load_oracle()
        for p0, p1, skip in ((0x7f7fffff, 0, 0xff7f), (0x7f800000, 0, 0), (0, 0, 0x7fc0)):
            with self.assertRaises(oracle.FinitePolicyError): M.materialized_word(oracle, p0, p1, skip)

    def test_down_uses_candidate_first_residual_not_framework_or_baseline(self):
        f = fixture(); values = parts(f)
        for value in values:
            value['first-residual'] = b'\x80\x3f' * 4096
            value['final-hidden'] = b'\x80\x3f' * 4096
        rows, inputs = M.residual_comparisons({'embedding': f['hidden'], 'first-residual': f['hidden']}, values)
        self.assertFalse(rows[0]['byte_equal'])
        self.assertTrue(all(row['byte_equal'] for row in rows if row['stage'] == 'down'))
        self.assertEqual([row['residual_source'] for row in inputs],
            ['genuine-framework-embedding'] * 2 + ['candidate-first-residual'] * 2)

    def test_wrong_residual_is_retained_as_mismatch_not_hidden_by_tolerance(self):
        f = fixture()
        for rank in (0, 1): replace_stage(f, rank, 'final-hidden', b'\x80\x3f' * 4096)
        result = run(f)
        self.assertFalse(result['conditional_residuals_exact'])
        self.assertEqual([r['differing_words'] for r in result['conditional_residual_comparisons']], [0, 0, 4096, 4096])
        self.assertFalse(result['numerical_acceptance'])

    def test_reference_mapping_uses_product_and_excludes_fp32_partials(self):
        result = run(fixture())
        self.assertEqual([r['stage'] for r in result['comparisons']], [name for name in K.ORDER for _ in range(2)])
        self.assertNotIn('output-partial', [r['stage'] for r in result['comparisons']])
        self.assertIn('activation-product', [r['stage'] for r in result['comparisons']])

    def test_repeat_framework_and_one_attempt_nonclaims_still_fail_closed(self):
        f = fixture(); f['framework']['candidate_intermediate_inputs'] = True
        body = T.raw(f['framework']); f['files'][f['pins'][0]['path']] = body; f['pins'][0] = pin(f['pins'][0]['path'], body)
        with self.assertRaises(ValueError): run(f)
        for key, value in (('native_attempts', 2), ('bitwise_equal', False), ('numerical_acceptance', True)):
            f = fixture(); f['candidate'][key] = value; sync(f)
            with self.assertRaises(ValueError): run(f)


if __name__ == '__main__':
    unittest.main(verbosity=2)
