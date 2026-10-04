import copy
import hashlib
import json
import math
from pathlib import Path
import struct
import tempfile
import types
import unittest
from unittest.mock import patch, Mock

import attention_reference as A
import output_reference as O
import validation as V
import compare_numerical as S


def pin(path, raw):
    return dict(path=path, bytes=len(raw), sha256=S.digest(raw))


def state(index):
    return ([1, 0, 65535, 65535, 0x55555555, 0] + [64] * 16 if index == 0 else
        [1, 0, 0, 31] + [1, 48, 1, 16, 64] * 2 + ([4294967295] * 4 + [3]) * 2
        + [1] * 130 + [64] * 130)


def synthetic_fixture():
    """In-memory zero operands, not authentic weights, images or GPU evidence."""
    store = {}
    def put(name, value):
        raw = value if type(value) is bytes else S.json_bytes(value)
        path = '/synthetic/' + name
        store[path] = raw
        return pin(path, raw)
    baseline = dict(schema='fe2o3-qwen-resident-prefix-tp2-request-v1', history_kind='genuine',
        position=0, devices=[dict(rank=r, unique_id=u) for r, u in enumerate(V.DEVICES)],
        producer=put('v5.hsaco', b'v5'), consumer=put('consumer.hsaco', b'c'),
        pair_reference=put('pair.json', {}), timeout_ms=10000,
        prefix_requests=[put('rank0.json', {}), put('rank1.json', {})])
    baseline_pin = put('baseline.json', baseline)
    artifact = dict(object=put('v6.hsaco', b'v6'), descriptor_sha256='a' * 64,
        canonical_code_object_digest='b' * 64, entry_symbol=V.SYMBOL, descriptor_symbol=V.SYMBOL + '.kd')
    reviews = []
    for kind in V.KINDS:
        reviews.append(put(kind + '.json', dict(schema='fe2o3-qwen-prefix-tiles-comparison-review-v6',
            authority='none', kind=kind, baseline_request_sha256=baseline_pin['sha256'],
            baseline_image_sha256=baseline['producer']['sha256'], tiles_image_sha256=artifact['object']['sha256'],
            descriptor_sha256='a' * 64, canonical_code_object_digest='b' * 64,
            devices=V.DEVICES, history_kind='genuine', position=0, workgroup=[64, 1, 1], grid=[4096, 1, 1],
            reviewed=True, runtime_premises_discharged=False, production_authority=False,
            notes='Synthetic test fixture only.')))
    requested = dict(schema='fe2o3-qwen-prefix-tiles-comparison-request-v6', baseline_request=baseline_pin,
        tiles=artifact, reviews=reviews, timeout_ms=10000, capture_directory='/synthetic/captures')
    request_pin = put('request.json', requested)
    weight = bytes(16777216)
    weights = [put('o-rank0.bf16', weight), put('o-rank1.bf16', weight)]
    input_hashes = [['c' * 64] * 7 for _ in range(2)]
    for row in input_hashes:
        row[5] = S.digest(struct.pack('<145I', 0, *[(p * 5 + 7) % 144 for p in range(144)]))
        row[6] = weights[0]['sha256']
    inspected = dict(schema='fe2o3-qwen-prefix-tiles-comparison-inspection-v6', authority='none',
        request_sha256=request_pin['sha256'], baseline_request=baseline_pin,
        baseline_image_sha256=baseline['producer']['sha256'], tiles=artifact, reviews=reviews,
        devices=V.DEVICES, history_kind='genuine', position=0, opened_device=False,
        completed_and_closed=False, bitwise_match=False, input_sha256=input_hashes,
        initial_output_sha256=[['d' * 64] * 7 for _ in range(2)], data_root_bytes=V.EXTENTS,
        v5_grid=[128, 1, 1], v6_grid=[4096, 1, 1], workgroup=[64, 1, 1], stage_order=list(V.STAGES),
        capture_bytes_per_rank=V.CAPTURE_BYTES, capture_format=V.FORMAT,
        **{key: False for key in V.FALSE_FIELDS})
    stages = [bytes(size) for size in V.EXTENTS[7:]]
    raw = b''.join(stages)
    capture_pins = [[put('captures/' + profile + '-rank' + str(rank) + '.bin', raw)
                     for rank in range(2)] for profile in ('baseline-v5', 'tiles-v6')]
    observed = dict(inspected, schema='fe2o3-qwen-prefix-tiles-comparison-observation-v6',
        opened_device=True, completed_and_closed=True, bitwise_match=True,
        profiles=[dict(profile=name, states=[state(i), state(i)], host_dispatch_elapsed_ns=[0, 1], closed=True)
                  for i, name in enumerate(('baseline_v5', 'tiles_v6'))], captures=capture_pins,
        immutable_input_readbacks_match=True, timing_boundary=V.TIMING,
        stages=[dict(rank=r, stage=name, elements=len(stages[i]) // (4 if i == 6 else 2),
            word_bytes=4 if i == 6 else 2, mismatches=0, baseline_sha256=S.digest(stages[i]),
            tiles_sha256=S.digest(stages[i])) for r in range(2) for i, name in enumerate(V.STAGES)])
    review = dict(schema='ferric-p227-prefix-numerical-prerequisites-v1', authority='none',
        arithmetic_class=S.CLASS, baseline_image_sha256=baseline['producer']['sha256'],
        tiles_image_sha256=artifact['object']['sha256'], source_lineage_review=reviews[0], isa_review=reviews[2],
        attention_policy_sha256=S.ATTENTION_POLICY, output_reference_sha256=S.HELPERS['output_reference.py'],
        prerequisites=S.PREREQUISITES, reviewed=True, production_authority=False,
        notes='Synthetic arithmetic review, no GPU observation.')
    plan = dict(schema='ferric-p227-prefix-numerical-inputs-v1', case='genuine-pos0', request=request_pin,
        inspection=put('inspection.json', inspected), observation=put('observation.json', observed),
        numerical_review=put('numerical-review.json', review), output_weights=weights)
    return plan, store


def memory_reader(store):
    def read(record, maximum=65536):
        V.pin(record, maximum)
        raw = store[record['path']]
        S.require(len(raw) == record['bytes'] and S.digest(raw) == record['sha256'], 'synthetic pin mismatch')
        return raw
    return read


class Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.plan, cls.store = synthetic_fixture()

    def test_frozen_helpers_and_policy_are_exact(self):
        for name, expected in S.HELPERS.items():
            self.assertEqual(hashlib.sha256(Path(S.__file__).with_name(name).read_bytes()).hexdigest(), expected)
        raw = Path(S.__file__).with_name('attention-policy.json').read_bytes()
        self.assertEqual(S.digest(raw), S.ATTENTION_POLICY)
        self.assertEqual(V.parse(raw), A.POLICY)
        self.assertEqual(O.SHARD_SHA, list(S.WEIGHTS))
        checked_v, checked_a, checked_o = S.helpers()
        self.assertEqual(checked_v.CAPTURE_BYTES, V.CAPTURE_BYTES)
        self.assertEqual(checked_a.POLICY, A.POLICY)
        self.assertEqual(checked_o.SHARD_SHA, O.SHARD_SHA)

    def test_attention_position_zero_preserves_signed_zero(self):
        expected = [-0.0] * 2048
        self.assertEqual(A.compare([0x8000] * 2048, expected, [0.0] * 16, 0)['exact'], 2048)
        with self.assertRaises(ValueError): A.compare([0] * 2048, expected, [0.0] * 16, 0)

    def test_attention_one_bf16_step_and_one_beyond(self):
        self.assertEqual(A.compare([0x3f81] * 2048, [1.0] * 2048, [0.0] * 16, 1)['max_bf16_steps'], 1)
        with self.assertRaises(ValueError): A.compare([0x3f82] * 2048, [1.0] * 2048, [0.0] * 16, 1)

    def test_attention_cancellation_exact_bound_and_one_under(self):
        A.compare([0x4000] * 2048, [0.0] * 2048, [40000.0] * 16, 1)
        with self.assertRaises(ValueError):
            A.compare([0x4000] * 2048, [0.0] * 2048, [math.nextafter(40000.0, -math.inf)] * 16, 1)

    def test_independent_gqa_reference_distinguishes_wrong_head_mapping(self):
        values = [word for word in (0x3e80, 0x3f00, 0x3f40, 0x3f80) for _ in range(128)]
        expected, maxima = A.dense_reference([0] * 2048, [0] * 512, values, 0)
        right = [values[(head // 4) * 128 + d] for head in range(16) for d in range(128)]
        A.compare(right, expected, maxima, 0)
        with self.assertRaises(ValueError): A.compare(list(reversed(right)), expected, maxima, 0)

    def test_logical_cache_preserves_order_across_pages_and_long_boundary(self):
        for position in (0, 15, 16, 2047, 2048):
            raw = bytearray(b'\xc1\x7f' * (2304 * 512))
            for token in range(position + 1):
                slot = ((token // 16 * 5 + 7) % 144) * 16 + token % 16
                raw[slot * 1024:(slot + 1) * 1024] = struct.pack('<H', token) * 512
            logical = S.logical_cache(V, bytes(raw), position)
            self.assertEqual(len(logical), (position + 1) * 1024)
            self.assertEqual([struct.unpack_from('<H', logical, t * 1024)[0]
                              for t in range(position + 1)], list(range(position + 1)))

    def test_future_poison_is_unread_but_current_poison_refuses(self):
        raw = bytearray(b'\xc1\x7f' * (2304 * 512))
        raw[112 * 1024:113 * 1024] = bytes(1024)
        self.assertEqual(S.logical_cache(V, bytes(raw), 0), bytes(1024))
        with self.assertRaises(RuntimeError): S.logical_cache(V, bytes(raw), 1)

    def test_cache_extent_position_and_capture_extent_are_closed(self):
        for position in (-1, 2304, True):
            with self.assertRaises(RuntimeError): S.logical_cache(V, bytes(2359296), position)
        with self.assertRaises(RuntimeError): S.logical_cache(V, bytes(2359295), 0)
        with self.assertRaises(RuntimeError): S.split_capture(V, bytes(V.CAPTURE_BYTES - 1))

    def test_partial_bound_exact_upper_and_one_under(self):
        raw = struct.pack('<f', 1.0) * 4096
        result = S.check_partial(raw, [0.0] * 4096, [math.nextafter(1.0, math.inf)] * 4096)
        self.assertEqual(result['max_bound_ratio'], 1.0)
        with self.assertRaises(RuntimeError): S.check_partial(raw, [0.0] * 4096, [1.0] * 4096)
        S.check_partial(bytes(16384), [0.0] * 4096, [0.0] * 4096)

    def test_partial_nonfinite_negative_bound_and_extent_refuse(self):
        for reference, bound in ((math.nan, 1.0), (0.0, math.inf), (0.0, -1.0)):
            with self.assertRaises(RuntimeError): S.check_partial(bytes(16384), [reference] * 4096, [bound] * 4096)
        with self.assertRaises(RuntimeError): S.check_partial(struct.pack('<f', math.inf) * 4096, [0.0] * 4096, [1.0] * 4096)
        with self.assertRaises(RuntimeError): S.check_partial(bytes(16383), [0.0] * 4096, [1.0] * 4096)

    def test_numerical_review_rejects_other_accumulation_classes(self):
        requested = V.parse(self.store[self.plan['request']['path']])
        baseline = V.parse(self.store[requested['baseline_request']['path']])
        review = V.parse(self.store[self.plan['numerical_review']['path']])
        S.review(V, review, requested, baseline)
        for name in ('mfma', 'split-k', 'split-attention', 'reassociated', ''):
            with self.assertRaises(RuntimeError): S.review(V, dict(review, arithmetic_class=name), requested, baseline)

    def test_numerical_review_binds_images_policies_and_prerequisites(self):
        requested = V.parse(self.store[self.plan['request']['path']])
        baseline = V.parse(self.store[requested['baseline_request']['path']])
        review = V.parse(self.store[self.plan['numerical_review']['path']])
        for key, value in [('baseline_image_sha256', 'f' * 64), ('tiles_image_sha256', 'f' * 64),
                ('attention_policy_sha256', 'f' * 64), ('prerequisites', S.PREREQUISITES[:-1]),
                ('reviewed', 1), ('production_authority', True), ('notes', '')]:
            with self.assertRaises(RuntimeError): S.review(V, dict(review, **{key: value}), requested, baseline)

    def checked_fixture(self, plan=None, store=None, reference=None):
        plan, store = plan or self.plan, store or self.store
        reference = reference or Mock(return_value=([0.0] * 4096, [1.0] * 4096, None, None))
        output = types.SimpleNamespace(np=O.np, reference=reference)
        # Only this synthetic test substitutes zero-shard identities; CLI has no such option.
        with patch.object(S, 'WEIGHTS', tuple(p['sha256'] for p in self.plan['output_weights'])):
            result = S.compare(plan, memory_reader(store), (V, A, output))
        return result, reference

    def test_full_capture_replay_and_four_conditioned_rows_no_model_claim(self):
        result, reference = self.checked_fixture()
        self.assertTrue(result['scheduling_bitwise_parity'])
        self.assertTrue(result['conditional_operator_checks_passed'])
        self.assertEqual([(r['profile'], r['rank']) for r in result['rows']],
                         [('baseline_v5', 0), ('baseline_v5', 1), ('tiles_v6', 0), ('tiles_v6', 1)])
        self.assertEqual(reference.call_count, 4)
        for row, call in zip(result['rows'], reference.call_args_list):
            self.assertEqual(call.args[0].shape, (4096, 2048))
            self.assertEqual(S.digest(call.args[1].tobytes()), row['conditioning']['attention_sha256'])
        for key in S.FALSE_FIELDS: self.assertIs(result[key], False)

    def test_exact_parity_does_not_rescue_independently_wrong_attention(self):
        plan, store = copy.deepcopy(self.plan), dict(self.store)
        observed = V.parse(store[plan['observation']['path']])
        offset = sum(V.EXTENTS[7:12]); wrong = b'\x80\x3f' * 2048
        for pair in observed['captures']:
            for record in pair:
                old = store[record['path']]
                new = old[:offset] + wrong + old[offset + len(wrong):]
                store[record['path']] = new; record.update(pin(record['path'], new))
        for row in observed['stages']:
            if row['stage'] == 'attention':
                row['baseline_sha256'] = row['tiles_sha256'] = S.digest(wrong)
        raw = S.json_bytes(observed); store[plan['observation']['path']] = raw
        plan['observation'] = pin(plan['observation']['path'], raw)
        reference = Mock()
        with self.assertRaises(ValueError): self.checked_fixture(plan, store, reference)
        reference.assert_not_called()

    def test_attention_pass_does_not_rescue_bad_conditional_projection(self):
        reference = Mock(return_value=([2.0] * 4096, [1.0] * 4096, None, None))
        with self.assertRaises(RuntimeError): self.checked_fixture(reference=reference)
        self.assertEqual(reference.call_count, 1)

    def test_weight_input_identity_and_raw_capture_hash_refuse(self):
        plan = copy.deepcopy(self.plan); plan['output_weights'][0]['sha256'] = 'f' * 64
        with self.assertRaises(RuntimeError): self.checked_fixture(plan=plan)
        store = dict(self.store)
        capture = '/synthetic/captures/tiles-v6-rank1.bin'
        store[capture] = store[capture][:-1]
        with self.assertRaises(RuntimeError): self.checked_fixture(store=store)

    def test_reader_rejects_symlink_wrong_hash_and_changed_recheck(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'data'; path.write_bytes(b'abc')
            read = S.Reader(V); record = pin(str(path), b'abc')
            self.assertEqual(read(record), b'abc')
            with self.assertRaises(RuntimeError): read(dict(record, sha256='f' * 64))
            link = Path(directory) / 'link'; link.symlink_to(path)
            with self.assertRaises(RuntimeError): read(pin(str(link), b'abc'))
            path.write_bytes(b'abd')
            with self.assertRaises(RuntimeError): read.recheck()

    def test_failed_cli_comparison_never_publishes_result(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'plan.json'; raw = S.json_bytes({}); path.write_bytes(raw)
            result = Path(directory) / 'result.json'
            with patch.object(S, 'helpers', return_value=(V, A, O)), \
                    patch.object(S, 'compare', side_effect=RuntimeError('numerical failure')):
                with self.assertRaises(RuntimeError): S.main([str(path), S.digest(raw), str(result)])
            self.assertFalse(result.exists())


if __name__ == '__main__':
    unittest.main()
