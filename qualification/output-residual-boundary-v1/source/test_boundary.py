"""Synthetic bit-pattern tests only; no native buffers or framework execution."""
import copy
from pathlib import Path
import struct
import tempfile
import unittest

import boundary as B


def vector(width, first=0, index=0):
    values = [0] * B.ELEMENTS
    values[index] = first
    return struct.pack('<4096' + ('I' if width == 4 else 'H'), *values)


def fixture():
    return dict(partials=[vector(4), vector(4)], embedding=vector(2),
        native_residuals=[vector(2), vector(2)], framework_projection=vector(2), framework_residual=vector(2))


def rows(result):
    return {row['name']: row for row in result['comparisons']}


class BoundaryTests(unittest.TestCase):
    def test_all_zero_fixture_calls_real_pinned_oracle_without_authority(self):
        result = B.compare(**fixture())
        self.assertEqual(len(result['comparisons']), 8)
        self.assertTrue(all(row['byte_equal'] and row['exact_words'] == 4096 for row in result['comparisons']))
        self.assertTrue(result['conditional_replay_performed'])
        self.assertIsNone(result['acceptance_threshold'])
        for key in ('input_provenance_verified', 'native_embedding_equality_verified', 'upstream_partial_numerics_checked',
                    'projection_gemm_equivalence_proven', 'causal_explanation_proven', 'genuine_framework_chain_rerun',
                    'arithmetic_prerequisites_verified', 'runtime_premises_discharged', 'numerical_acceptance',
                    'full_layer_numerics_accepted', 'full_model_acceptance', 'gpu_execution',
                    'performance_claim', 'production_authority'):
            self.assertIs(result[key], False)

    def test_intermediate_bf16_tie_changes_residual_without_changing_partial_sum(self):
        value = fixture()
        value.update(partials=[vector(4, 0x3f808000), vector(4)], embedding=vector(2, 0xbf80),
            native_residuals=[vector(2, 0x3b80), vector(2, 0x3b80)], framework_projection=vector(2, 0x3f80))
        result = rows(B.compare(**value))
        self.assertTrue(result['native-formula-vs-native-rank0']['byte_equal'])
        self.assertTrue(result['native-formula-vs-native-rank1']['byte_equal'])
        self.assertEqual(result['native-formula-vs-framework-residual']['differing_words'], 1)
        self.assertTrue(result['materialized-formula-vs-framework-residual']['byte_equal'])
        self.assertTrue(result['framework-add-vs-framework-residual']['byte_equal'])
        self.assertTrue(result['rounded-rank-sum-vs-framework-projection']['byte_equal'])

    def test_framework_projection_difference_is_reported_separately(self):
        value = fixture()
        value.update(partials=[vector(4, 0x3f800000), vector(4)], native_residuals=[vector(2, 0x3f80)] * 2,
            framework_projection=vector(2, 0x3f81), framework_residual=vector(2, 0x3f81))
        result = rows(B.compare(**value))
        self.assertTrue(result['framework-add-vs-framework-residual']['byte_equal'])
        self.assertEqual(result['rounded-rank-sum-vs-framework-projection']['differing_words'], 1)
        self.assertEqual(result['materialized-formula-vs-framework-residual']['differing_words'], 1)

    def test_actual_formula_mismatch_is_not_silently_accepted(self):
        value = fixture(); value['native_residuals'][1] = vector(2, 0x3f80)
        result = rows(B.compare(**value))
        self.assertTrue(result['native-formula-vs-native-rank0']['byte_equal'])
        self.assertEqual(result['native-formula-vs-native-rank1']['first_differences'],
                         [dict(index=0, expected_bits=0, actual_bits=0x3f80)])

    def test_framework_residual_control_can_fail_independently(self):
        value = fixture(); value['framework_residual'] = vector(2, 0x3f80, 4095)
        result = rows(B.compare(**value))
        self.assertEqual(result['framework-add-vs-framework-residual']['differing_words'], 1)
        self.assertEqual(result['framework-add-vs-framework-residual']['first_differences'][0]['index'], 4095)

    def test_initial_positive_zero_and_framework_signed_zero_stay_distinct(self):
        value = fixture()
        value.update(partials=[vector(4, 0x80000000)] * 2, embedding=vector(2, 0x8000),
                     framework_projection=vector(2, 0x8000), framework_residual=vector(2, 0x8000))
        result = rows(B.compare(**value))
        self.assertTrue(result['native-formula-vs-native-rank0']['byte_equal'])
        self.assertEqual(result['native-formula-vs-framework-residual']['differing_words'], 1)
        self.assertTrue(result['framework-add-vs-framework-residual']['byte_equal'])

    def test_final_word_is_included_and_input_buffers_are_unchanged(self):
        value = fixture()
        value.update(partials=[vector(4, 0x3f800000, 4095), vector(4)],
                     native_residuals=[vector(2, 0x3f80, 4095)] * 2,
                     framework_projection=vector(2, 0x3f80, 4095), framework_residual=vector(2, 0x3f80, 4095))
        before = copy.deepcopy(value)
        result = B.compare(**value)
        self.assertTrue(all(row['byte_equal'] for row in result['comparisons']))
        self.assertEqual(value, before)
        self.assertEqual(result['conditioning']['output_partials'][0]['bytes'], 16384)

    def test_all_input_extents_and_pair_cardinalities_are_closed(self):
        for key in ('embedding', 'framework_projection', 'framework_residual'):
            for raw in (b'', vector(2) + b'\0\0', bytearray(vector(2))):
                value = fixture(); value[key] = raw
                with self.assertRaises(ValueError): B.compare(**value)
        for key, width in (('partials', 4), ('native_residuals', 2)):
            for pair in ([], [vector(width)], [vector(width)] * 3, [vector(width), b'']):
                value = fixture(); value[key] = pair
                with self.assertRaises(ValueError): B.compare(**value)

    def test_nonfinite_inputs_refuse_at_first_middle_and_last_elements(self):
        for index in (0, 2048, 4095):
            for key, width, bad in (('partials', 4, 0x7f800000), ('native_residuals', 2, 0x7fc1),
                                    ('embedding', 2, 0xff80), ('framework_projection', 2, 0x7fc0),
                                    ('framework_residual', 2, 0x7f80)):
                value = fixture()
                if key in ('partials', 'native_residuals'): value[key][1] = vector(width, bad, index)
                else: value[key] = vector(width, bad, index)
                with self.assertRaises(ValueError): B.compare(**value)

    def test_rank_sum_overflow_refuses_instead_of_returning_partial_diagnostics(self):
        value = fixture(); value['partials'] = [vector(4, 0x7f7fffff, 4095)] * 2
        with self.assertRaisesRegex(ValueError, 'element 4095:'): B.compare(**value)

    def test_new_materialization_nonfinite_boundary_is_not_waived(self):
        value = fixture()
        value.update(partials=[vector(4, 0x7f7fffff), vector(4)], embedding=vector(2, 0xff7f))
        with self.assertRaisesRegex(ValueError, 'element 0: bf16-result'): B.compare(**value)

    def test_oracle_hash_and_symlink_refusals(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'oracle.py'; path.write_bytes(b'raise RuntimeError("must not execute")\n')
            with self.assertRaisesRegex(ValueError, 'unchanged retained oracle'): B.load_oracle(path)
            link = Path(temp) / 'link.py'; link.symlink_to(path)
            with self.assertRaisesRegex(ValueError, 'canonical pinned oracle'): B.load_oracle(link)


if __name__ == '__main__':
    unittest.main()
