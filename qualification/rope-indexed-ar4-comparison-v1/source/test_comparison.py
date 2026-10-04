"""Synthetic policy/metric tests only; no native or installed-framework evidence."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest

import comparison as C
import diagnostics as D
import run as R


def cases(outputs=(67, 25, 576, 2701), seed=9112):
    rows, token = [], seed
    for pos, output in enumerate(outputs):
        raw = bytearray(D.PAYLOAD_BYTES)
        start = (D.LAYERS + 1) * D.HIDDEN * 2
        struct.pack_into('<H', raw, start + output * 2, 0x3f80)
        rows.append((dict(generation=pos + 1, position=pos, input_token=token, output_token=output), bytes(raw)))
        token = output
    return rows


def pin(path, raw):
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def requests():
    old = dict(schema='FerricFiniteProjectionResidualDecodeRequestV1', projection_residual_image={'fixed': 1},
        decode=dict(mode='autoregressive', prefix_image={'bytes': 10, 'sha256': [1] * 32}, session=[1] * 32,
            evidence_directory='/old/native', expected_model_id=list(bytes.fromhex(C.MODEL_ID)),
            expected_bundle_id=list(bytes.fromhex(C.BUNDLE_ID)), source={'fixed': 2},
            device_ids=[16366993098680759275, 10838076764495710945]))
    new = copy.deepcopy(old)
    new['decode'].update(prefix_image={'bytes': C.IMAGE[0], 'sha256': list(bytes.fromhex(C.IMAGE[1]))},
        session=[2] * 32, evidence_directory='/new/native')
    return old, new


class ComparisonTests(unittest.TestCase):
    def test_all_same_histories_have_152_before_after_rows(self):
        ref = cases()
        result = C.metric_rows(D, ref, ref, ref)
        self.assertEqual((result['candidate_tensor_rows'], result['comparable_delta_rows']), (152, 152))
        self.assertTrue(result['all_four_output_tokens_equal'])
        self.assertTrue(all(row['candidate_minus_baseline']['relative_l2'] == 0 for row in result['tensors']))
        self.assertFalse(result['numerical_acceptance'])

    def test_diverged_history_never_recovers_when_current_token_matches(self):
        result = C.metric_rows(D, cases(), cases(), cases((7, 25, 576, 2701)))
        self.assertEqual([row['candidate_same_history'] for row in result['positions']], [True, False, False, False])
        self.assertEqual(result['candidate_tensor_rows'], 38)
        self.assertTrue(all(row['candidate'] is None and row['candidate_minus_baseline'] is None
            and row['needs_conditional_reference'] for row in result['tensors'][38:]))
        self.assertEqual(result['positions'][2]['framework_input_token'], result['positions'][2]['candidate_input_token'])

    def test_different_last_output_does_not_invalidate_same_current_input_history(self):
        result = C.metric_rows(D, cases(), cases(), cases((67, 25, 576, 7)))
        self.assertEqual(result['candidate_tensor_rows'], 152)
        self.assertFalse(result['all_four_output_tokens_equal'])
        self.assertGreater(result['tensors'][-1]['candidate']['max_abs_error'], 0)

    def test_wrong_seed_refuses(self):
        with self.assertRaises(ValueError):
            D.compare_four_forwards('autoregressive', [9112], cases(), cases(seed=2190))

    def test_teacher_forced_inputs_cannot_fake_ar_history(self):
        rows = cases()
        rows[1][0]['input_token'] = 2190
        with self.assertRaises(ValueError):
            D.compare_four_forwards('autoregressive', [9112], cases(), rows)

    def test_boolean_record_token_refuses(self):
        rows = cases()
        rows[0][0]['input_token'] = True
        with self.assertRaises(ValueError):
            D.validate_case(*rows[0], 0)

    def test_nonfinite_bf16_refuses(self):
        with self.assertRaises(ValueError):
            D.compare_tensor(b'\x80\x7f', b'\x00\x00')

    def test_lowest_index_argmax_is_checked_against_payload(self):
        record, raw = cases()[0]
        changed = bytearray(raw)
        struct.pack_into('<H', changed, (D.LAYERS + 1) * D.HIDDEN * 2, 0x3f80)
        with self.assertRaises(ValueError):
            D.validate_case(record, bytes(changed), 0)

    def test_signed_zero_is_numerically_equal_but_not_bitwise_equal(self):
        value = D.compare_tensor(b'\x00\x00', b'\x00\x80')
        self.assertEqual((value['exact_words'], value['max_abs_error'], value['relative_l2']), (0, 0.0, 0.0))

    def test_nonzero_error_against_zero_norm_is_not_invented_relative_l2(self):
        value = D.compare_tensor(b'\x00\x00', b'\x80\x3f')
        self.assertIsNone(value['relative_l2'])
        self.assertTrue(value['zero_reference_norm'])

    def test_request_allows_only_actual_prefix_and_fresh_run_identity(self):
        old, new = requests()
        C.request_delta(old, new)
        self.assertEqual(new['decode']['device_ids'][0], 16366993098680759275)

    def test_request_wrong_image_or_residual_refuses(self):
        for mutate in (lambda v: v['decode']['prefix_image'].update(bytes=54343),
                       lambda v: v.update(projection_residual_image={'fixed': 2})):
            old, new = requests(); mutate(new)
            with self.assertRaises(ValueError):
                C.request_delta(old, new)

    def test_request_workload_or_tf_mode_change_refuses(self):
        for key, value in (('source', {'fixed': 3}), ('mode', 'teacher_forced')):
            old, new = requests(); new['decode'][key] = value
            with self.assertRaises(ValueError):
                C.request_delta(old, new)

    def test_duplicate_and_nonfinite_json_refuse(self):
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}'):
            for parser in (C.parse, R.parse):
                with self.assertRaises(ValueError):
                    parser(raw)

    def test_transport_preserves_original_pin_and_exact_uint64(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'retained.json'
            raw = b'{"id":16366993098680759275}'
            path.write_bytes(raw)
            retained = pin(path, raw); original = dict(retained, path='/original/input.json')
            reader = R.Reader({original['path']: retained})
            self.assertEqual(C.parse(reader.read(original))['id'], 16366993098680759275)
            self.assertEqual(reader.consumed[original['path']], dict(original=original, retained=retained))
            reader.recheck()

    def test_transport_extent_substitution_and_postread_drift_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'retained'; path.write_bytes(b'abc')
            retained = pin(path, b'abc'); original = dict(retained, path='/original/file')
            reader = R.Reader({original['path']: retained})
            with self.assertRaises(ValueError):
                reader.read(dict(original, bytes=2))
            reader.read(original); path.write_bytes(b'abd')
            with self.assertRaises(ValueError):
                reader.recheck()

    def test_private_data_helper_alias_restoration_on_success_and_error(self):
        name = '_rope_comparison_test_alias'
        missing = object(); initial = sys.modules.get(name, missing)
        try:
            for old in (missing, None, object()):
                if old is missing:
                    sys.modules.pop(name, None)
                else:
                    sys.modules[name] = old
                R.module('synthetic', '/synthetic.py', b'value = 1', {name: object()})
                self.assertIs(sys.modules.get(name, missing), old)
                with self.assertRaises(ValueError):
                    R.module('synthetic', '/synthetic.py', b'raise ValueError("expected")', {name: object()})
                self.assertIs(sys.modules.get(name, missing), old)
        finally:
            if initial is missing:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = initial

    def test_reference_repeat_body_drift_is_not_accepted(self):
        raw_cases = cases(); bodies = {}
        value = dict(schema='ferric-p228-projection-ar4-framework-reference-v1', status='PASS',
            model_id=C.MODEL_ID, bundle_id=C.BUNDLE_ID, seed=9112, native_complete=C.BASELINE,
            genuine_framework_chain=True, candidate_intermediate_inputs=False, repeat_passes_byte_equal=True,
            framework_forward_count=8, captured_tensors_per_forward=38, conditional_passes=[],
            conditional_replay_performed=False, numerical_acceptance=False, genuine_passes=[])
        owner = dict(schema='ferric-p228-projection-ar4-framework-launch-complete-v1', passed=True,
            failures=[], reference=C.REFERENCE, native_attempts=1, retries=0, numerical_acceptance=False)
        for ordinal in (1, 2):
            entry = dict(ordinal=ordinal, kind='genuine_ar', fresh_cache=True, cases=[])
            for pos, (record, raw) in enumerate(raw_cases):
                path = Path(C.REFERENCE['path']).parent / f'genuine-ar-pass{ordinal}-pos{pos}.bf16'
                parts = D.split_payload(raw)
                entry['cases'].append(dict(record=record, payload=pin(path, raw), cache_sha256=[],
                    tensors={name: dict(bytes=len(body), sha256=C.digest(body)) for name, body in parts.items()}))
                bodies[str(path)] = raw
            value['genuine_passes'].append(entry)
        bodies[C.REFERENCE_OWNER['path']] = json.dumps(owner).encode()
        bodies[C.REFERENCE['path']] = json.dumps(value).encode()
        reader = lambda record: bodies[record['path']]
        C.framework(reader, D)
        last = value['genuine_passes'][1]['cases'][3]
        changed = bytearray(bodies[last['payload']['path']]); changed[1] = 0x3f
        bodies[last['payload']['path']] = bytes(changed)
        with self.assertRaises(ValueError):
            C.framework(reader, D)


if __name__ == '__main__':
    unittest.main(verbosity=2)
