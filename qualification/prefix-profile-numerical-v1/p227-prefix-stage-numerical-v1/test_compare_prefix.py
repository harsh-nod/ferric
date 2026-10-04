import copy
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import struct
import tempfile
import types
import unittest
from unittest.mock import Mock, patch

import compare_prefix as C
import prefix_math as H


def pin(path, raw):
    return dict(path=path, bytes=len(raw), sha256=C.sha(raw))


def cpu_head(M, source, weights, rotary):
    """Explicit source-order test stimulus, not an independent error oracle."""
    np = M.np
    result = []
    for head in range(20):
        x = M.bf16(source[head * 128:(head + 1) * 128])
        w = M.bf16(weights[:128] if head < 16 else weights[128:])
        total = np.float32(0)
        for value in x:
            total = np.float32(total + np.float32(value * value))
        inv = np.float32(np.float32(1) / np.sqrt(np.float32(np.float32(total / np.float32(128)) + np.float32(1e-6))))
        a = [M.scalar_bf16(M.round_bf16_real(float(np.float32(M.scalar_bf16(
            M.round_bf16_real(float(np.float32(value * inv)))) * float(weight))))) for value, weight in zip(x, w)]
        row = [0] * 128
        for lane in range(64):
            ac = np.float32(a[lane] * float(rotary[lane]))
            bs = np.float32(a[lane + 64] * float(rotary[lane + 64]))
            bc = np.float32(a[lane + 64] * float(rotary[lane]))
            ass = np.float32(a[lane] * float(rotary[lane + 64]))
            row[lane] = M.round_bf16_real(float(np.float32(ac - bs)))
            row[lane + 64] = M.round_bf16_real(float(np.float32(bc + ass)))
        result.extend(row)
    return np.asarray(result[:2048], dtype='<u2'), np.asarray(result[2048:], dtype='<u2')


def synthetic_fixture(V, N):
    """Closed in-memory CPU fixture. All identities are explicitly synthetic."""
    store = {}
    def put(name, value):
        raw = value if type(value) is bytes else N.json_bytes(value)
        path = '/synthetic/' + name
        store[path] = raw
        return pin(path, raw)
    zero_inputs = [bytes(size) for size in V.EXTENTS[:6]]
    zero_inputs[4] = struct.pack('<128f', *([1.0] * 64 + [0.0] * 64))
    zero_inputs[5] = struct.pack('<145I', 0, *[(p * 5 + 7) % 144 for p in range(144)])
    names = ['input-pos0.bf16', 'norm-weight.bf16', 'packed-qkv-rank0.bf16', 'head-weights.bf16',
             'rotary-pos0.f32', 'metadata-pos0.u32']
    q, g, p, r = {}, {}, {}, {}
    for name, body, target in zip(names, zero_inputs, [g, q, q, p, r, r]):
        record = put(name, body)
        target[name] = dict(bytes=record['bytes'], sha256=record['sha256'])
    q['packed-qkv-rank1.bf16'] = dict(q['packed-qkv-rank0.bf16'])
    put('packed-qkv-rank1.bf16', zero_inputs[2])
    for name in ('query-norm.bf16', 'key-norm.bf16'):
        q[name] = dict(bytes=256, sha256=C.sha(bytes(256)))
    fixtures = dict(zip(C.INPUTS, [dict(files=q), dict(files=g), dict(files=p), dict(files=r)]))
    ranks = []
    for rank in range(2):
        ranks.append(put('rank%d.json' % rank, dict(schema='fe2o3-qwen-wave-qkv-attention-output-source-request-v5',
            rank=rank, history_kind='genuine', position=0, fixture_directory='/synthetic',
            genuine_fixture_directory='/synthetic', post_fixture_directory='/synthetic',
            rotary_fixture_directory='/synthetic')))
    baseline = dict(schema='fe2o3-qwen-resident-prefix-tp2-request-v1', history_kind='genuine', position=0,
        devices=[dict(rank=r, unique_id=u) for r, u in enumerate(V.DEVICES)], producer=put('v5', b'v5'),
        consumer=put('consumer', b'c'), pair_reference=put('pair.json', {}), timeout_ms=10000, prefix_requests=ranks)
    baseline_pin = put('baseline.json', baseline)
    artifact = dict(object=put('v6', b'v6'), descriptor_sha256='a' * 64, canonical_code_object_digest='b' * 64,
        entry_symbol=V.SYMBOL, descriptor_symbol=V.SYMBOL + '.kd')
    reviews = [put(kind + '.json', dict(schema='fe2o3-qwen-prefix-tiles-comparison-review-v6', authority='none',
        kind=kind, baseline_request_sha256=baseline_pin['sha256'], baseline_image_sha256=baseline['producer']['sha256'],
        tiles_image_sha256=artifact['object']['sha256'], descriptor_sha256='a' * 64, canonical_code_object_digest='b' * 64,
        devices=V.DEVICES, history_kind='genuine', position=0, workgroup=[64, 1, 1], grid=[4096, 1, 1], reviewed=True,
        runtime_premises_discharged=False, production_authority=False, notes='Synthetic test only.')) for kind in V.KINDS]
    requested = dict(schema='fe2o3-qwen-prefix-tiles-comparison-request-v6', baseline_request=baseline_pin,
        tiles=artifact, reviews=reviews, timeout_ms=10000, capture_directory='/synthetic/captures')
    request_pin = put('request.json', requested)
    inspected = dict(schema='fe2o3-qwen-prefix-tiles-comparison-inspection-v6', authority='none',
        request_sha256=request_pin['sha256'], baseline_request=baseline_pin, baseline_image_sha256=baseline['producer']['sha256'],
        tiles=artifact, reviews=reviews, devices=V.DEVICES, history_kind='genuine', position=0, opened_device=False,
        completed_and_closed=False, bitwise_match=False, input_sha256=[[C.sha(x) for x in zero_inputs] + ['c' * 64] for _ in range(2)],
        initial_output_sha256=[['d' * 64] * 7 for _ in range(2)], data_root_bytes=V.EXTENTS,
        v5_grid=[128, 1, 1], v6_grid=[4096, 1, 1], workgroup=[64, 1, 1], stage_order=list(V.STAGES),
        capture_bytes_per_rank=V.CAPTURE_BYTES, capture_format=V.FORMAT, **{key: False for key in V.FALSE_FIELDS})
    stages = [bytes(size) for size in V.EXTENTS[7:]]
    state = lambda i: ([1, 0, 65535, 65535, 0x55555555, 0] + [64] * 16 if i == 0 else
        [1, 0, 0, 31] + [1, 48, 1, 16, 64] * 2 + ([4294967295] * 4 + [3]) * 2 + [1] * 130 + [64] * 130)
    observed = dict(inspected, schema='fe2o3-qwen-prefix-tiles-comparison-observation-v6', opened_device=True,
        completed_and_closed=True, bitwise_match=True, immutable_input_readbacks_match=True, timing_boundary=V.TIMING,
        profiles=[dict(profile=name, states=[state(i), state(i)], host_dispatch_elapsed_ns=[0, 1], closed=True)
                  for i, name in enumerate(('baseline_v5', 'tiles_v6'))],
        captures=[[put('captures/' + name + '-rank%d.bin' % rank, b''.join(stages)) for rank in range(2)]
                  for name in ('baseline-v5', 'tiles-v6')],
        stages=[dict(rank=rank, stage=name, elements=len(stages[i]) // (4 if i == 6 else 2), word_bytes=4 if i == 6 else 2,
                     mismatches=0, baseline_sha256=C.sha(stages[i]), tiles_sha256=C.sha(stages[i]))
                for rank in range(2) for i, name in enumerate(V.STAGES)])
    review = dict(schema='ferric-p227-prefix-stage-prerequisites-v1', authority='none', arithmetic_class=C.CLASS,
        baseline_image_sha256=baseline['producer']['sha256'], tiles_image_sha256=artifact['object']['sha256'],
        source_lineage_review=reviews[0], isa_review=reviews[2], source_sha256=C.SOURCES, policy_sha256=C.POLICY_SHA,
        prerequisites=C.PREREQUISITES, reviewed=True, production_authority=False, notes='Synthetic only, not artifact review.')
    plan = dict(schema='ferric-p227-prefix-stage-inputs-v1', case='genuine-pos0', request=request_pin,
        inspection=put('inspection.json', inspected), observation=put('observation.json', observed),
        numerical_review=put('numerical.json', review))
    return plan, store, fixtures


class Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.V, cls.M, cls.N, cls.loaded_math, cls.fixtures = C.helpers()
        cls.plan, cls.store, cls.synthetic = synthetic_fixture(cls.V, cls.N)

    def read(self, record, maximum=65536):
        self.V.pin(record, maximum)
        raw = self.store[record['path']]
        C.require(len(raw) == record['bytes'] and C.sha(raw) == record['sha256'], 'synthetic pin mismatch')
        return raw

    def math_fixture(self):
        np = self.M.np
        source = np.asarray(([0x3f80, 0x4000] * 64) * 24, dtype='<u2')
        source[64:128] = 0x4040
        weights = np.asarray([0x3f80] * 128 + [0x3f00] * 128, dtype='<u2')
        rotary = np.asarray([0.75] * 64 + [0.5] * 64, dtype='<f4')
        query, key = cpu_head(self.M, source, weights, rotary)
        return [source, weights, rotary, query, key, source[2560:].copy()]

    def test_exact_helpers_metadata_policy_and_source_hashes(self):
        root = Path(C.__file__).parent
        for name, digest in C.HELPERS.items():
            self.assertEqual(C.sha((root / 'helpers' / name).read_bytes()), digest)
        for name, digest in {**C.INPUTS, **C.SOURCES}.items():
            self.assertEqual(C.sha((root / 'inputs' / name).read_bytes()), digest)
        policy = json.loads((root / 'policy.json').read_bytes())
        self.assertEqual(policy['serial_headnorm']['gamma_f32_depth'], 129)
        self.assertEqual(policy['serial_headnorm']['half_subnormal_allowances'], 256)
        self.assertEqual(policy['serial_headnorm']['maximum_first_bf16_candidates'], 32)
        self.assertIs(policy['adaptive_tolerance'], False)
        self.assertEqual(C.sha((root / 'policy.json').read_bytes()), C.POLICY_SHA)

    def test_exact_integer_square_sum_subnormals_signs_and_no_float_reduction(self):
        np = self.M.np
        self.assertEqual(H.exact_square_sum(self.M, np.asarray([1, 0x8001] + [0] * 126, dtype='<u2')), Fraction(2, 1 << 266))
        self.assertEqual(H.exact_square_sum(self.M, np.asarray([0x3f80, 0xbf80] + [0] * 126, dtype='<u2')), 2)
        self.assertEqual(H.exact_square_sum(self.M, np.asarray([0x8000] * 128, dtype='<u2')), 0)

    def test_serial128_source_sum_is_enclosed_for_fixed_extreme_inputs(self):
        np = self.M.np
        for values in ([0] * 128, [1] * 128, [0x3f80] * 128, [0x3f80, 0xbf81] * 64, [0x3a80, 0x4280] * 64):
            a = np.asarray(values, dtype='<u2'); total = np.float32(0)
            for x in self.M.bf16(a): total = np.float32(total + np.float32(x * x))
            lo, hi = H.head_sum_interval(self.M, a)
            self.assertLessEqual(lo, float(total)); self.assertGreaterEqual(hi, float(total))

    def test_serial_envelope_nonfinite_shape_and_overflow_refuse(self):
        np = self.M.np
        for values in ([0] * 127, [0x7f80] + [0] * 127, [0x7f7f] * 128):
            with self.assertRaises(ValueError): H.head_sum_interval(self.M, np.asarray(values, dtype='<u2'))

    def test_serial_inverse_contains_source_sqrt_divide_without_fitting(self):
        np = self.M.np
        a = np.asarray([0x3f80] * 128, dtype='<u2')
        lo, hi = H.head_inverse_interval(self.M, a)
        actual = float(np.float32(np.float32(1) / np.sqrt(np.float32(1 + np.float32(1e-6)))))
        self.assertLessEqual(lo, actual); self.assertGreaterEqual(hi, actual)

    def test_zero_sign_and_two_bf16_boundaries_are_preserved(self):
        self.assertEqual(H.weighted_candidates(self.M, 0x8000, 0xbf80, (1.0, 1.0)), (0,))
        self.assertEqual(H.weighted_candidates(self.M, 0, 0xbf80, (1.0, 1.0)), (0x8000,))
        self.assertEqual(H.weighted_candidates(self.M, 0x3f80, 0x3f80, (1.0, 1.0)), (0x3f80,))
        self.assertNotEqual(self.M.round_bf16_real(1.00390625), self.M.round_bf16_real(math.nextafter(1.00390625, math.inf)))

    def test_candidate_uncertainty_refuses_instead_of_expanding_tolerance(self):
        with self.assertRaises(ValueError): H.weighted_candidates(self.M, 0x3f80, 0x3f80, (0.5, 2.0))

    def test_signed_zero_negative_and_subnormal_learned_weights(self):
        self.assertEqual(H.weighted_candidates(self.M, 0x3f80, 0x8000, (1.0, 1.0)), (0x8000,))
        self.assertEqual(H.weighted_candidates(self.M, 0x3f80, 0xbf80, (1.0, 1.0)), (0xbf80,))
        self.assertEqual(H.weighted_candidates(self.M, 0x3f80, 1, (1.0, 1.0)), (1,))
        self.assertEqual(H.weighted_candidates(self.M, 1, 0x3f80, (0.5, 0.5)), (0,))

    def test_dual_bf16_weighting_rejects_collapsed_rounding_mutant(self):
        midpoint = 1.00390625
        allowed = H.weighted_candidates(self.M, 0x3f80, 0x3f81, (midpoint, midpoint))
        collapsed = self.M.round_bf16_real(float(self.M.np.float32(midpoint * self.M.scalar_bf16(0x3f81))))
        self.assertEqual(allowed, (0x3f81,)); self.assertEqual(collapsed, 0x3f82)
        self.assertNotIn(collapsed, allowed)

    def test_rope_cancellation_distinguishes_separate_multiply_rounding(self):
        cosine = struct.unpack('<f', struct.pack('<I', 0x3f800001))[0]
        sine = struct.unpack('<f', struct.pack('<I', 0x3f810001))[0]
        separate = H.rotate_pair(self.M, 0x3f81, 0x3f80, cosine, sine)[0]
        collapsed = self.M.scalar_bf16(0x3f81) * cosine - sine
        self.assertEqual(separate, 0)
        self.assertEqual(collapsed, 2.0**-30)
        self.assertEqual(self.M.round_bf16_real(collapsed), 0x3080)

    def test_staged_rope_pair_has_exact_products_and_sum_sign(self):
        self.assertEqual(H.rotate_pair(self.M, 0x3f80, 0x4000, 0.75, 0.5), (0xbe80, 0x4000))
        self.assertEqual(H.rotate_pair(self.M, 0x8000, 0, 1.0, 0.0), (0x8000, 0))
        for value in (math.inf, math.nan):
            with self.assertRaises(ValueError): H.rotate_pair(self.M, 0x3f80, 0x4000, value, 0.0)

    def test_twenty_heads_accept_separate_source_stimulus(self):
        result = H.check_head_rope(self.M, *self.math_fixture())
        self.assertEqual((result['heads'], result['query_words'], result['key_words']), (20, 2048, 512))
        self.assertTrue(result['shared_inverse_correlations_overapproximated'])

    def test_wrong_head_weights_split_half_or_rotary_sign_are_sensitive(self):
        original = self.math_fixture()
        for mutate in ('weights', 'split', 'sine'):
            args = [x.copy() for x in original]
            if mutate == 'weights': args[1][:128], args[1][128:] = original[1][128:], original[1][:128]
            elif mutate == 'split': args[3][:128] = args[3][:128].reshape(2, 64).T.reshape(128)
            else: args[2][64:] *= -1
            with self.assertRaises(ValueError): H.check_head_rope(self.M, *args)

    def test_first_last_query_key_and_value_mutants_refuse(self):
        original = self.math_fixture()
        for role, index in ((3, 0), (3, 2047), (4, 0), (4, 511), (5, 0), (5, 511)):
            args = [x.copy() for x in original]; args[role][index] ^= 0x8000
            with self.assertRaises(ValueError): H.check_head_rope(self.M, *args)

    def test_nonfinite_head_inputs_weights_rotary_and_outputs_refuse(self):
        original = self.math_fixture()
        for role in range(6):
            args = [x.copy() for x in original]; args[role][-1] = math.inf if role == 2 else 0x7f80
            with self.assertRaises(ValueError): H.check_head_rope(self.M, *args)

    def test_existing_p218_wave_norm_and_qkv_checks_are_not_reimplemented(self):
        np = self.M.np
        inputs = [bytes(n) for n in self.V.EXTENTS[:6]]
        inputs[5] = struct.pack('<145I', 2048, *[(p * 5 + 7) % 144 for p in range(144)])
        stages = [bytes(n) for n in self.V.EXTENTS[7:]]
        with patch.object(self.M, 'check_norm', return_value={'norm': True}) as norm, \
             patch.object(self.M, 'check_bf16_gemv', return_value={'qkv': True}) as gemv, \
             patch.object(H, 'check_head_rope', return_value={'head': True}) as head:
            result = H.check_rank(self.M, inputs, stages, 2048)
        self.assertEqual(result['physical_slot'], 1136)
        self.assertEqual(gemv.call_args.args[0].shape, (3072, 4096))
        self.assertEqual(gemv.call_args.args[1].tobytes(), stages[0])
        self.assertEqual(head.call_args.args[1].tobytes(), stages[1])
        norm.assert_called_once()

    def test_wrong_metadata_position_mapping_and_boolean_position_refuse(self):
        inputs = [bytes(n) for n in self.V.EXTENTS[:6]]
        stages = [bytes(n) for n in self.V.EXTENTS[7:]]
        for position in (True, -1, 2304, 0):
            with self.assertRaises(ValueError): H.check_rank(self.M, inputs, stages, position)

    def test_actual_metadata_six_cases_have_exact_shapes_and_distinct_rank_weights(self):
        for case in self.V.CASES:
            history, pos = self.V.case_scope(case)
            requests = [dict(schema='fe2o3-qwen-wave-qkv-attention-output-source-request-v5', rank=rank,
                history_kind=history, position=pos, fixture_directory='/qkv', genuine_fixture_directory='/genuine',
                post_fixture_directory='/post', rotary_fixture_directory='/rotary') for rank in range(2)]
            N = types.SimpleNamespace(document=lambda V, read, rec: requests[rec])
            rows = C.expected_inputs(self.V, N, None, {'prefix_requests': [0, 1]}, case, self.fixtures)
            self.assertEqual([p['bytes'] for p in rows[0]], self.V.EXTENTS[:6])
            self.assertEqual(rows[0][0], rows[1][0]); self.assertNotEqual(rows[0][2]['sha256'], rows[1][2]['sha256'])

    def test_review_closes_source_policy_math_prerequisites_and_authority(self):
        requested = self.N.document(self.V, self.read, self.plan['request'])
        baseline = self.N.document(self.V, self.read, requested['baseline_request'])
        review = self.N.document(self.V, self.read, self.plan['numerical_review'])
        C.numerical_review(self.V, review, requested, baseline)
        for key, value in [('reviewed', 1), ('prerequisites', C.PREREQUISITES[:-1]), ('source_sha256', {}),
                           ('policy_sha256', 'f' * 64), ('notes', ''), ('production_authority', True),
                           ('arithmetic_class', 'mfma'), ('tiles_image_sha256', 'f' * 64)]:
            with self.assertRaises(RuntimeError): C.numerical_review(self.V, dict(review, **{key: value}), requested, baseline)

    def test_closed_capture_intake_calls_four_rows_and_retains_false_acceptance(self):
        fake = types.SimpleNamespace(check_rank=Mock(return_value={'checked': True}))
        result = C.compare(self.plan, self.read, (self.V, self.M, self.N, fake, self.synthetic))
        self.assertEqual(fake.check_rank.call_count, 4)
        self.assertEqual([(r['profile'], r['rank']) for r in result['rows']],
                         [('baseline_v5', 0), ('baseline_v5', 1), ('tiles_v6', 0), ('tiles_v6', 1)])
        for flag in C.FALSE: self.assertIs(result[flag], False)
        self.assertIs(result['attention_and_o_checked'], False)

    def test_actual_input_digest_mismatch_refuses_before_any_math(self):
        fixture = copy.deepcopy(self.synthetic)
        fixture['qkv-manifest.json']['files']['norm-weight.bf16']['sha256'] = 'f' * 64
        fake = types.SimpleNamespace(check_rank=Mock())
        with self.assertRaises(RuntimeError): C.compare(self.plan, self.read, (self.V, self.M, self.N, fake, fixture))
        fake.check_rank.assert_not_called()

    def test_capture_or_learned_weight_bytes_cannot_be_replaced(self):
        for path in ('/synthetic/captures/tiles-v6-rank1.bin', '/synthetic/head-weights.bf16'):
            with patch.dict(self.store, {path: b'changed'}):
                with self.assertRaises(RuntimeError): C.compare(self.plan, self.read,
                    (self.V, self.M, self.N, types.SimpleNamespace(check_rank=Mock()), self.synthetic))

    def test_operator_failure_is_not_rescued_by_exact_scheduling_parity(self):
        fake = types.SimpleNamespace(check_rank=Mock(side_effect=ValueError('fixed operator bound exceeded')))
        with self.assertRaises(ValueError): C.compare(self.plan, self.read, (self.V, self.M, self.N, fake, self.synthetic))
        self.assertEqual(fake.check_rank.call_count, 1)

    def test_plan_unknown_fields_and_optimized_python_refuse(self):
        with self.assertRaises(RuntimeError): C.compare(dict(self.plan, tolerance=1), self.read,
            (self.V, self.M, self.N, H, self.synthetic))
        with patch.object(C.sys, 'flags', types.SimpleNamespace(optimize=1)):
            with self.assertRaises(RuntimeError): C.compare(self.plan, self.read, (self.V, self.M, self.N, H, self.synthetic))

    def test_final_fresh_input_recheck_precedes_exclusive_publication(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'plan.json'; path.write_bytes(self.N.json_bytes(self.plan))
            output = Path(directory) / 'result.json'
            reader = Mock(); reader.return_value = path.read_bytes(); reader.recheck.side_effect = RuntimeError('input drift')
            frozen = (self.V, self.M, self.N, H, self.synthetic)
            with patch.object(C, 'helpers', return_value=frozen), patch.object(self.N, 'Reader', return_value=reader), \
                 patch.object(C, 'compare', return_value={'schema': C.SCHEMA}):
                with self.assertRaises(RuntimeError): C.main([str(path), C.sha(path.read_bytes()), str(output)])
            self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()
