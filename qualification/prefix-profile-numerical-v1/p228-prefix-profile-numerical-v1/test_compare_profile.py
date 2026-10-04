"""CPU tests whose capture fixtures are synthetic, never GPU receipts."""
import copy
from contextlib import contextmanager
from pathlib import Path
import struct
import types
import unittest
from unittest.mock import patch

import compare_profile as C


def synthetic_fixture(frozen, case='genuine-pos0'):
    S, V, M, N, H, _, A, O = frozen
    if case not in ('genuine-pos0', 'patterned-pos16'):
        raise ValueError('closed synthetic test cases')
    history, position = V.case_scope(case)
    store = {}

    def put(name, value):
        raw = value if type(value) is bytes else N.json_bytes(value)
        path = '/synthetic/' + name
        store[path] = raw
        return dict(path=path, bytes=len(raw), sha256=C.sha(raw))

    def exact_bf16(value):
        bits = struct.unpack('<I', struct.pack('<f', value))[0]
        if bits & 0xffff:
            raise ValueError('synthetic stimulus must be exactly BF16 representable')
        return struct.pack('<H', bits >> 16)

    def scale(rank, head, dim):
        magnitude = (0.5, 1.0, 2.0)[dim % 3]
        return -magnitude if (rank + head + dim) % 2 else magnitude

    q, g, p, r = {}, {}, {}, {}
    common = [bytes(size) for size in V.EXTENTS[:6]]
    common[4] = struct.pack('<128f', *([1.0] * 64 + [0.0] * 64))
    common[5] = struct.pack('<145I', position, *[(page * 5 + 7) % 144 for page in range(144)])
    for name, raw, manifest in (
        (('input-pos%d.bf16' % position) if history == 'genuine' else 'input.bf16',
         common[0], g if history == 'genuine' else q),
        ('norm-weight.bf16', common[1], q),
        ('head-weights.bf16', common[3], p),
        (('rotary-pos%d.f32' % position) if history == 'genuine' else ('rotary-%d.f32' % position),
         common[4], r if history == 'genuine' else p),
        (('metadata-pos%d.u32' % position) if history == 'genuine' else ('metadata-%d.u32' % position),
         common[5], r if history == 'genuine' else p),
    ):
        record = put(name, raw)
        manifest[name] = {key: record[key] for key in ('bytes', 'sha256')}
    for name in ('query-norm.bf16', 'key-norm.bf16'):
        q[name] = dict(bytes=256, sha256=C.sha(bytes(256)))
    requests, ranks, weight_hashes = [], [], []
    for rank in range(2):
        stages = [bytes(size) for size in V.EXTENTS[7:]]
        for stage in (3, 4):
            raw = bytearray(b'\xc1\x7f' * (V.EXTENTS[7 + stage] // 2))
            for token in range(position + 1):
                slot = ((token // 16 * 5 + 7) % 144) * 16 + token % 16
                if token == position:
                    current = bytes(1024)
                else:
                    current = b''.join(exact_bf16(
                        ((token * 7 + head * 3 + dim + rank * 5) % 17 - 8) / 16.0
                        if stage == 3 else
                        (17.0 / 16.0 if token < 8 else 51.0 / 16.0) * scale(rank, head, dim)
                    ) for head in range(4) for dim in range(128))
                raw[slot * 1024:(slot + 1) * 1024] = current
            stages[stage] = bytes(raw)
        if position == 16:
            # Q=0 makes all 17 softmax weights equal. Eight values of 17/16
            # plus eight of 51/16 and current zero sum to 34: the mean is 2.
            # Signs/powers of two keep every input and expected result exact.
            stages[5] = b''.join(exact_bf16(2.0 * scale(rank, head // 4, dim))
                                 for head in range(16) for dim in range(128))
            first_partial = 2.0 * scale(rank, 0, 0) if rank == 1 else 0.0
            stages[6] = struct.pack('<f', first_partial) + bytes(V.EXTENTS[13] - 4)
        # Distinct rank weights, but a zero input keeps this stimulus simple.
        qkv = common[2] if rank == 0 else b'\x80\x3f' + common[2][2:]
        name = 'packed-qkv-rank%d.bf16' % rank
        record = put(name, qkv)
        q[name] = {key: record[key] for key in ('bytes', 'sha256')}
        requests.append(put('rank%d.json' % rank, dict(
            schema='fe2o3-qwen-wave-qkv-attention-output-source-request-v5',
            rank=rank, history_kind=history, position=position,
            fixture_directory='/synthetic', genuine_fixture_directory='/synthetic',
            post_fixture_directory='/synthetic', rotary_fixture_directory='/synthetic')))
        weights = bytes(V.EXTENTS[6]) if rank == 0 else b'\x80\x3f' + bytes(V.EXTENTS[6] - 2)
        weight_pin = put('output-weight-rank%d.bf16' % rank, weights)
        weight_hashes.append(weight_pin['sha256'])
        ranks.append(dict(rank=rank, capture=put('rank%d.bin' % rank, b''.join(stages)),
                          input_sha256=[C.sha(raw) for raw in
                                        [common[0], common[1], qkv, *common[3:]]] + [C.sha(weights)],
                          output_weights=weight_pin))
    baseline = put('baseline.json', dict(schema='fe2o3-qwen-resident-prefix-tp2-request-v1',
                                        history_kind=history, position=position, prefix_requests=requests))
    plan = dict(schema=C.PLAN_SCHEMA, profile='baseline_v5', case=case,
                baseline_request=baseline, ranks=ranks)
    fixtures = dict(zip(S.INPUTS, [dict(files=q), dict(files=g), dict(files=p), dict(files=r)]))
    # Only the test dependency bundle accepts synthetic weight identities.
    # helpers() always loads the original two authentic shard digests.
    synthetic_N = types.SimpleNamespace(**{**vars(N), 'WEIGHTS': tuple(weight_hashes)})
    return plan, store, (S, V, M, synthetic_N, H, fixtures, A, O)


class ProfileTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.original = C.helpers()
        cls.template, cls.buffers, cls.synthetic = synthetic_fixture(cls.original)

    def setUp(self):
        self.plan = copy.deepcopy(self.template)
        self.store = dict(self.buffers)
        self.frozen = (*self.synthetic[:5], copy.deepcopy(self.synthetic[5]), *self.synthetic[6:])
        self.S, self.V, self.M, self.N, self.H, self.fixtures, self.A, self.O = self.frozen

    def read(self, record, maximum=64 << 10):
        return self.store[record['path']]

    def compare(self):
        return C.compare_profile(self.plan, self.read, self.frozen)

    def replace(self, record, raw):
        self.store[record['path']] = raw
        record.update(bytes=len(raw), sha256=C.sha(raw))

    def change_stage(self, stage, raw, rank=0):
        record = self.plan['ranks'][rank]['capture']
        stages = self.N.split_capture(self.V, self.store[record['path']])
        stages[stage] = raw
        self.replace(record, b''.join(stages))

    @contextmanager
    def routing_only(self):
        """Mocks prove plumbing only. They cannot qualify numerical evidence."""
        np = self.O.np
        with patch.object(self.H, 'check_rank', return_value={'synthetic_routing_only': True}) as prefix, \
             patch.object(self.A, 'dense_reference', return_value=([0.0] * 2048, [0.0] * 16)) as dense, \
             patch.object(self.O, 'reference', return_value=(np.zeros(4096), np.zeros(4096), None, 0.0)) as output:
            yield prefix, dense, output

    def test_loaded_references_and_policies_remain_original(self):
        S, V, M, N, H, _, A, O = self.original
        self.assertEqual(M.np.__version__, '2.2.6')
        self.assertEqual(O.np.__version__, '2.2.6')
        self.assertEqual(A.POLICY['bf16_steps'], 1)
        self.assertEqual(A.POLICY['cancellation_coefficient'], 5e-5)
        self.assertIs(A.POLICY['adaptive_tolerance'], False)
        self.assertEqual((H.HEAD_DEPTH, H.HEAD_HALF_SUBNORMALS, H.MAX_FIRST_CANDIDATES), (129, 256, 32))
        self.assertEqual(tuple(N.WEIGHTS), tuple(O.SHARD_SHA))
        self.assertEqual(V.CAPTURE_BYTES, sum(V.EXTENTS[7:]))

    def test_real_references_accept_synthetic_zero_profile_without_authority(self):
        result = self.compare()
        self.assertIs(result['conditional_operator_checks_passed'], True)
        self.assertEqual([(row['profile'], row['rank']) for row in result['rows']],
                         [('baseline_v5', 0), ('baseline_v5', 1)])
        for row in result['rows']:
            self.assertEqual(row['attention']['exact'], 2048)
            self.assertEqual(row['output_partial']['max_abs_error_upper'], 0.0)
        for name in C.FALSE_FIELDS:
            self.assertIs(result[name], False)

    def test_real_references_accept_nonzero_history_across_a_page_boundary(self):
        plan, store, frozen = synthetic_fixture(self.original, 'patterned-pos16')
        plan['profile'] = 'tiles_v6'
        result = C.compare_profile(plan, lambda record, maximum: store[record['path']], frozen)
        N, V = frozen[3], frozen[1]
        for rank, row in enumerate(result['rows']):
            stages = N.split_capture(V, store[plan['ranks'][rank]['capture']['path']])
            keys, values = [N.logical_cache(V, raw, 16) for raw in stages[3:5]]
            self.assertEqual(len(keys), 17 * 1024)
            self.assertEqual(len(values), 17 * 1024)
            self.assertTrue(any(N.words(keys[:16 * 1024])))
            self.assertTrue(all(word & 0x7fff for word in N.words(values[:16 * 1024])))
            self.assertEqual(keys[-1024:], bytes(1024))
            self.assertEqual(values[-1024:], bytes(1024))
            self.assertEqual(values[15 * 1024:16 * 1024], stages[4][127 * 1024:128 * 1024])
            self.assertEqual(values[16 * 1024:], stages[4][192 * 1024:193 * 1024])
            self.assertEqual(row['prefix']['physical_slot'], 192)
            self.assertEqual(row['attention']['exact'], 2048)
            self.assertEqual(row['output_partial']['max_abs_error_upper'], 0.0)
            self.assertEqual(row['conditioning']['causal_value_sha256'], C.sha(values))
        for name in C.FALSE_FIELDS:
            self.assertIs(result[name], False)

    def test_causal_prior_value_corruption_fails_the_real_attention_reference(self):
        plan, store, frozen = synthetic_fixture(self.original, 'patterned-pos16')
        record = plan['ranks'][0]['capture']
        N, V = frozen[3], frozen[1]
        stages = N.split_capture(V, store[record['path']])
        raw = bytearray(stages[4])
        # Token15 is on physical page7, immediately before the logical page
        # transition. Keep the pin honest and all words finite; only the
        # unchanged attention result is now inconsistent with its causal V.
        offset = 127 * 1024
        word = struct.unpack_from('<H', raw, offset)[0]
        struct.pack_into('<H', raw, offset, word ^ 0x8000)
        stages[4] = bytes(raw)
        capture = b''.join(stages)
        store[record['path']] = capture
        record.update(bytes=len(capture), sha256=C.sha(capture))
        with self.assertRaisesRegex(ValueError, 'attention tolerance'):
            C.compare_profile(plan, lambda pin, maximum: store[pin['path']], frozen)

    def test_profiles_do_not_require_a_paired_bitwise_comparison(self):
        with self.routing_only(), \
             patch.object(self.S, 'compare', side_effect=AssertionError('old paired wrapper')), \
             patch.object(self.N, 'compare', side_effect=AssertionError('old paired wrapper')), \
             patch.object(self.V, 'observation', side_effect=AssertionError('old parity gate')):
            baseline = self.compare()
            self.plan['profile'] = 'tiles_v6'
            # Negative zero is numerically equal, but its FP32 bits differ.
            self.change_stage(6, struct.pack('<I', 0x80000000) + bytes(self.V.EXTENTS[13] - 4))
            candidate = self.compare()
        self.assertNotEqual(baseline['rows'][0]['stage_sha256'][6], candidate['rows'][0]['stage_sha256'][6])
        for result in (baseline, candidate):
            self.assertIs(result['paired_comparison_performed'], False)
            self.assertIs(result['untouched_kv_bytes_checked'], False)

    def test_closed_plan_profile_case_and_rank_fields(self):
        mutations = [dict(self.plan, tolerance=1), dict(self.plan, profile='tiles-v6'),
                     dict(self.plan, schema='old'), dict(self.plan, case='genuine-pos1'),
                     dict(self.plan, ranks=self.plan['ranks'][:1])]
        for rank_value in (True, 1):
            plan = copy.deepcopy(self.plan)
            plan['ranks'][0]['rank'] = rank_value
            mutations.append(plan)
        plan = copy.deepcopy(self.plan)
        plan['ranks'][0]['extra'] = True
        mutations.append(plan)
        for plan in mutations:
            with self.subTest(plan=plan), self.assertRaises(RuntimeError):
                C.compare_profile(plan, self.read, self.frozen)

    def test_seven_input_hashes_have_closed_shape_and_spelling(self):
        for value in ([0] * 7, ['f' * 64] * 6, ['F' * 64] * 7, 'not a list'):
            self.plan['ranks'][0]['input_sha256'] = value
            with self.assertRaises(RuntimeError):
                self.compare()

    def test_authentic_rank_input_and_o_shard_substitution_refuse(self):
        for role in ('qkv', 'output', 'extent'):
            self.plan = copy.deepcopy(self.template)
            if role == 'qkv':
                self.plan['ranks'][0]['input_sha256'][2] = self.plan['ranks'][1]['input_sha256'][2]
            elif role == 'output':
                self.plan['ranks'][0]['output_weights'] = self.plan['ranks'][1]['output_weights']
            else:
                self.plan['ranks'][0]['output_weights']['bytes'] -= 2
            with self.subTest(role=role), self.assertRaises(RuntimeError):
                self.compare()

    def test_capture_alias_extent_and_changed_bytes_refuse(self):
        for mutation in ('alias', 'extent', 'changed'):
            self.plan = copy.deepcopy(self.template)
            self.store = dict(self.buffers)
            if mutation == 'alias':
                self.plan['ranks'][1]['capture'] = dict(self.plan['ranks'][0]['capture'])
            elif mutation == 'extent':
                self.plan['ranks'][0]['capture']['bytes'] -= 2
            else:
                record = self.plan['ranks'][0]['capture']
                self.store[record['path']] = b'changed'
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                self.compare()

    def test_reader_cannot_skip_the_comparators_byte_hash_checks(self):
        self.store['/synthetic/norm-weight.bf16'] = bytes(8190) + b'\x80\x3f'
        with self.assertRaisesRegex(RuntimeError, 'FilePin'):
            self.compare()

    def test_learned_qk_half_identity_is_checked_before_math(self):
        self.fixtures['qkv-manifest.json']['files']['query-norm.bf16']['sha256'] = 'f' * 64
        with patch.object(self.H, 'check_rank') as checked:
            with self.assertRaisesRegex(RuntimeError, 'learned head-weight'):
                self.compare()
            checked.assert_not_called()

    def test_original_source_request_rank_and_position_are_bound(self):
        for key, value in (('rank', True), ('rank', 1), ('position', 4)):
            self.plan = copy.deepcopy(self.template)
            self.store = dict(self.buffers)
            baseline = self.V.parse(self.store[self.plan['baseline_request']['path']])
            request = baseline['prefix_requests'][0]
            body = self.V.parse(self.store[request['path']])
            body[key] = value
            self.replace(request, self.N.json_bytes(body))
            self.replace(self.plan['baseline_request'], self.N.json_bytes(baseline))
            with self.subTest(key=key, value=value), self.assertRaises(RuntimeError):
                self.compare()

    def test_nonfinite_computed_stages_and_causal_kv_refuse(self):
        for stage in range(7):
            self.plan = copy.deepcopy(self.template)
            self.store = dict(self.buffers)
            raw = bytearray(self.N.split_capture(self.V, self.store['/synthetic/rank0.bin'])[stage])
            offset = 7 * 16 * 1024 if stage in (3, 4) else 0
            bad = struct.pack('<I', 0x7f800000) if stage == 6 else struct.pack('<H', 0x7f80)
            raw[offset:offset + len(bad)] = bad
            self.change_stage(stage, bytes(raw))
            with self.subTest(stage=stage), self.routing_only(), self.assertRaises(RuntimeError):
                self.compare()

    def test_each_reference_receives_actual_preceding_stage_bytes(self):
        with self.routing_only() as (prefix, dense, output):
            result = self.compare()
        self.assertEqual(prefix.call_count, 2)
        self.assertEqual(dense.call_count, 2)
        self.assertEqual(output.call_count, 2)
        for rank in range(2):
            stages = self.N.split_capture(self.V, self.store['/synthetic/rank%d.bin' % rank])
            self.assertIs(prefix.call_args_list[rank].args[0], self.M)
            self.assertEqual(prefix.call_args_list[rank].args[2], stages)
            self.assertEqual(dense.call_args_list[rank].args,
                             (self.N.words(stages[2]), [0] * 512, [0] * 512, 0))
            weights, attention = output.call_args_list[rank].args
            self.assertEqual(weights.shape, (4096, 2048))
            self.assertEqual(weights.tobytes(), self.store['/synthetic/output-weight-rank%d.bf16' % rank])
            self.assertEqual(attention.tobytes(), stages[5])
            self.assertEqual(result['rows'][rank]['stage_sha256'], [C.sha(raw) for raw in stages])

    def test_operator_failures_propagate_without_parity_rescue(self):
        for module, name in ((self.H, 'check_rank'), (self.A, 'compare'),
                             (self.O, 'reference'), (self.N, 'check_partial')):
            with self.routing_only(), patch.object(module, name, side_effect=ValueError('fixed bound exceeded')):
                with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'fixed bound'):
                    self.compare()

    def test_source_position_rejects_bool_and_float_json_numbers(self):
        for case, value in (('genuine-pos0', False), ('genuine-pos0', 0.0),
                            ('patterned-pos16', 16.0)):
            plan, store, frozen = synthetic_fixture(self.original, case)
            baseline_pin = plan['baseline_request']
            baseline = self.V.parse(store[baseline_pin['path']])
            request_pin = baseline['prefix_requests'][0]
            request = self.V.parse(store[request_pin['path']])
            request['position'] = value
            raw = self.N.json_bytes(request)
            store[request_pin['path']] = raw
            request_pin.update(bytes=len(raw), sha256=C.sha(raw))
            raw = self.N.json_bytes(baseline)
            store[baseline_pin['path']] = raw
            baseline_pin.update(bytes=len(raw), sha256=C.sha(raw))
            with self.subTest(case=case, value=value), \
                 self.assertRaisesRegex(RuntimeError, 'integer source request position'):
                C.compare_profile(plan, lambda pin, maximum: store[pin['path']], frozen)

    def test_output_partial_mismatch_is_not_hidden_by_the_reference(self):
        self.change_stage(6, struct.pack('<f', 1.0) + bytes(self.V.EXTENTS[13] - 4))
        with self.routing_only(), self.assertRaisesRegex(RuntimeError, 'conditional O error bound'):
            self.compare()

    def test_conflicting_pin_for_an_already_read_path_refuses(self):
        self.plan['ranks'][0]['output_weights']['path'] = '/synthetic/norm-weight.bf16'
        with self.assertRaisesRegex(RuntimeError, 'conflicting original input identity'):
            self.compare()

    def test_optimized_python_refuses_before_reading_inputs(self):
        with patch.object(C.sys, 'flags', types.SimpleNamespace(optimize=1)):
            with self.assertRaisesRegex(RuntimeError, 'assertions'):
                self.compare()

    def test_reference_digest_mismatch_refuses_before_execution(self):
        with self.assertRaisesRegex(RuntimeError, 'pinned reference module'):
            C._module(Path(C.__file__).resolve(), '0' * 64, 'must_not_execute')


if __name__ == '__main__':
    unittest.main()
