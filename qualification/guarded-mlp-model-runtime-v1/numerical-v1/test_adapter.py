"""Small synthetic adapter regressions, not candidate numerical qualification."""
import copy
import unittest

C = A = D = None


class AdapterTests(unittest.TestCase):
    def terminal(self, tag):
        return dict(schema='ferric-guarded-mlp-model-gpu-v1', mode=tag,
            passed=tag == 'ar4', errors=[] if tag == 'ar4' else ['RuntimeError: one actual worker announcement'],
            native_attempts=1, retries=0, gpu_execution=True, gpu_execution_confirmed=True,
            native_spawn_observed=True, **{name: False for name in A.FALSE})

    def test_original_failed_tf4_cannot_be_promoted(self):
        value = self.terminal('tf4')
        A.terminal_contract(C, value, 'tf4')
        for changes in ({'passed': True}, {'errors': []}, {'errors': ['other refusal']}):
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                A.terminal_contract(C, dict(value, **changes), 'tf4')

    def test_ar4_requires_actual_clean_success(self):
        value = self.terminal('ar4')
        A.terminal_contract(C, value, 'ar4')
        for changes in ({'passed': False}, {'errors': ['failure']}, {'mode': 'tf4'}, {'retries': 1}):
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                A.terminal_contract(C, dict(value, **changes), 'ar4')

    def test_acceptance_and_unobserved_gpu_flags_refuse(self):
        value = self.terminal('ar4')
        changes = [{key: True} for key in A.FALSE]
        changes += [{key: False} for key in ('gpu_execution', 'gpu_execution_confirmed', 'native_spawn_observed')]
        for change in changes:
            with self.subTest(change=change), self.assertRaises(ValueError):
                A.terminal_contract(C, dict(value, **change), 'ar4')

    def test_json_tuple_u64_join_preserves_numbers(self):
        number = 16366993098680759275
        self.assertTrue(A.same_json(C, {'frontier': [(number, 1)]}, {'frontier': [[number, 1]]}))
        self.assertFalse(A.same_json(C, {'frontier': [(number, 1)]}, {'frontier': [[number + 1, 1]]}))

    def test_history_divergence_never_resumes_tensor_comparison(self):
        raw = bytes(D.PAYLOAD_BYTES)
        left = [dict(generation=i + 1, position=i, input_token=v, output_token=0)
                for i, v in enumerate((9112, 0, 0, 0))]
        right = copy.deepcopy(left)
        right[1]['input_token'] = 1
        rows = C.compare_rows(left, [raw] * 4, right, [raw] * 4, D)
        self.assertEqual([r['same_input_history'] for r in rows], [True, False, False, False])
        self.assertEqual(len(rows[0]['tensors']), 38)
        self.assertTrue(all(r['byte_equal'] and r['max_abs_error'] == 0 for r in rows[0]['tensors']))
        self.assertEqual([r['tensors'] for r in rows[1:]], [None, None, None])
