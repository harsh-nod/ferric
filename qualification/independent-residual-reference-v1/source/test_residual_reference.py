"""Authored capture tests; root owns actual execution and receipts."""
import hashlib
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

import residual_reference as R


def row(word, width):
    return struct.pack('<I' if width == 4 else '<H', word) * R.ELEMENTS


class ResidualCaptureTests(unittest.TestCase):
    def setUp(self):
        self.partials = (row(0x3f000000, 4), row(0x3e800000, 4))
        self.hidden = (row(0x3f80, 2),) * 2
        self.first = (row(0x3fe0, 2),) * 2
        self.mlp = (row(0xbf800000, 4), row(0x3f000000, 4))
        self.final = (row(0x3fa0, 2),) * 2

    def test_helper_is_byte_identical_to_retained_pin(self):
        path = Path(__file__).resolve().parent / 'helpers/residual_oracle.py'
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), R.ORACLE_SHA256)
        self.assertEqual(R.load_oracle().ordered_residual_bits(0x4b800000, 0xcb800000, 0x3f80), 0x3f80)

    def test_changed_helper_refuses_before_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            path, marker = Path(directory) / 'oracle.py', Path(directory) / 'executed'
            path.write_text(f'open({str(marker)!r}, "w").write("bad")\n')
            with self.assertRaisesRegex(ValueError, 'pinned p222'):
                R.load_oracle(path)
            self.assertFalse(marker.exists())

    def test_symlink_and_relative_helper_paths_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'linked.py'
            path.symlink_to(Path(__file__).resolve().parent / 'helpers/residual_oracle.py')
            with self.assertRaisesRegex(ValueError, 'canonical'):
                R.load_oracle(path)
        with self.assertRaisesRegex(ValueError, 'canonical'):
            R.load_oracle('helpers/residual_oracle.py')

    def test_both_stage_labels_match_exact_outputs(self):
        for stage in R.STAGES:
            result = R.compare(stage, self.partials, self.hidden, self.first)
            self.assertTrue(result['conditional_tp_residual_checks_passed'])
            self.assertEqual(result['exact_bf16_words'], 8192)

    def test_nonuniform_4096_elements(self):
        patterns = [(0x3f000000, 0x3e800000, 0x3f80, 0x3fe0),
                    (0x4b800000, 0xcb800000, 0x3f80, 0x3f80),
                    (0x3f804000, 0xbf800000, 0, 0x3b00), (0x80000001, 0, 0, 0x8000)]
        rows = [struct.pack('<' + ('I' if c < 2 else 'H') * R.ELEMENTS,
                            *(patterns[i % 4][c] for i in range(R.ELEMENTS))) for c in range(4)]
        R.compare('post_attention', rows[:2], (rows[2],) * 2, (rows[3],) * 2)

    def test_rank_swap_invariance_does_not_hide_changed_conditioning(self):
        first = R.compare('post_attention', self.partials, self.hidden, self.first)
        swapped = R.compare('post_attention', self.partials[::-1], self.hidden, self.first)
        self.assertEqual(first['expected_sha256'], swapped['expected_sha256'])
        self.assertEqual(first['conditioning']['partial_sha256'][::-1],
                         swapped['conditioning']['partial_sha256'])

    def test_cancellation_preserves_late_residual(self):
        partials = (row(0x4b800000, 4), row(0xcb800000, 4))
        R.compare('post_attention', partials, self.hidden, self.hidden)
        with self.assertRaises(ValueError):
            R.compare('post_attention', partials, self.hidden, (row(0, 2),) * 2)

    def test_early_narrowing_and_tie_errors_refuse(self):
        for p0, p1, skip, correct, wrong in (
            (0x3f804000, 0xbf800000, 0, 0x3b00, 0),
            (0x3b800000, 0x3b800000, 0x3f80, 0x3f81, 0x3f80),
            (0x3f808000, 0, 0, 0x3f80, 0x3f81),
            (0x3f818000, 0, 0, 0x3f82, 0x3f81), (0x00018000, 0, 0, 2, 1)):
            args = ((row(p0, 4), row(p1, 4)), (row(skip, 2),) * 2)
            R.compare('post_mlp', *args, (row(correct, 2),) * 2)
            with self.assertRaises(ValueError):
                R.compare('post_mlp', *args, (row(wrong, 2),) * 2)

    def test_second_skip_is_first_residual_not_original_hidden(self):
        result = R.compare_stages(self.hidden, self.partials, self.first, self.mlp, self.final)
        self.assertEqual([stage['stage'] for stage in result['stages']], list(R.STAGES))
        self.assertEqual(result['stages'][0]['conditioning']['output_sha256'],
                         result['stages'][1]['conditioning']['residual_sha256'])
        with self.assertRaises(ValueError):
            R.compare_stages(self.hidden, self.partials, self.first, self.mlp, (row(0x3f00, 2),) * 2)

    def test_first_failure_prevents_second_comparison(self):
        with patch.object(R, 'compare', wraps=R.compare) as compared:
            with self.assertRaises(ValueError):
                R.compare_stages(self.hidden, self.partials, (row(0, 2),) * 2, self.mlp, self.final)
            self.assertEqual(compared.call_count, 1)

    def test_last_element_corruption_refuses_whole_result(self):
        changed = self.final[1][:-2] + struct.pack('<H', 0x3fa1)
        with self.assertRaisesRegex(ValueError, 'post_mlp rank 1 element 4095'):
            R.compare_stages(self.hidden, self.partials, self.first, self.mlp, (self.final[0], changed))

    def test_nonfinite_inputs_and_outputs_refuse(self):
        for bad in (0x7f800000, 0xff800000, 0x7fc00000, 0x7f800001):
            for rank in range(2):
                partials = list(self.partials)
                partials[rank] = partials[rank][:-4] + struct.pack('<I', bad)
                with self.assertRaises(ValueError):
                    R.compare('post_attention', partials, self.hidden, self.first)
        for bad in (0x7f80, 0xff80, 0x7fc0, 0x7f81):
            with self.assertRaises(ValueError):
                R.compare('post_attention', self.partials, (row(bad, 2),) * 2, self.first)
            with self.assertRaises(ValueError):
                R.compare('post_attention', self.partials, self.hidden, (row(bad, 2),) * 2)

    def test_intermediate_final_and_bf16_overflows_refuse(self):
        for p0, p1, skip in ((0x7f7fffff, 0x7f7fffff, 0),
                             (0x7f7fffff, 0, 0x7f7f), (0x7f7fffff, 0, 0)):
            with self.assertRaises(ValueError):
                R.compare('post_mlp', (row(p0, 4), row(p1, 4)),
                          (row(skip, 2),) * 2, (row(0, 2),) * 2)

    def test_exact_extents_types_and_rank_pairs(self):
        args = [self.partials, self.hidden, self.first]
        for role in range(3):
            for rank in range(2):
                raw = args[role][rank]
                for bad in (raw[:-1], raw + b'\0', bytearray(raw)):
                    changed = args.copy()
                    changed[role] = list(args[role])
                    changed[role][rank] = bad
                    with self.assertRaises(ValueError):
                        R.compare('post_attention', *changed)
            for pair in (args[role][:1], args[role] + args[role][:1], b''):
                changed = args.copy()
                changed[role] = pair
                with self.assertRaises(ValueError):
                    R.compare('post_attention', *changed)

    def test_closed_stage_and_equal_replicated_residuals(self):
        for stage in ('', 'full_layer', None, True):
            with self.assertRaises(ValueError):
                R.compare(stage, self.partials, self.hidden, self.first)
        with self.assertRaisesRegex(ValueError, 'replicated residual'):
            R.compare('post_attention', self.partials, (self.hidden[0], row(0, 2)), self.first)

    def test_signed_zero_remains_exact(self):
        positive, negative = (row(0, 2),) * 2, (row(0x8000, 2),) * 2
        partials = (row(0x80000000, 4),) * 2
        R.compare('post_attention', partials, negative, positive)
        with self.assertRaises(ValueError):
            R.compare('post_attention', partials, negative, negative)
        R.compare('post_mlp', (row(0x80000001, 4), row(0, 4)), positive, negative)

    def test_wrong_partial_residual_and_one_rank_output_refuse(self):
        for args in (((self.partials[0],) * 2, self.hidden, self.first),
                     (self.partials, (row(0, 2),) * 2, self.first),
                     (self.partials, self.hidden, (self.first[0], row(0, 2)))):
            with self.assertRaises(ValueError):
                R.compare('post_attention', *args)

    def test_hashes_bind_actual_bytes_and_all_nonclaims_stay_false(self):
        before = self.partials + self.hidden + self.first + self.mlp + self.final
        result = R.compare_stages(self.hidden, self.partials, self.first, self.mlp, self.final)
        self.assertEqual(before, self.partials + self.hidden + self.first + self.mlp + self.final)
        for report in [result] + result['stages']:
            self.assertEqual(report['authority'], 'none')
            self.assertEqual(report['oracle_sha256'], R.ORACLE_SHA256)
            for flag in R.FALSE_FIELDS:
                self.assertIs(report[flag], False)
        self.assertEqual(result['stages'][0]['conditioning']['partial_sha256'],
                         [hashlib.sha256(raw).hexdigest() for raw in self.partials])


if __name__ == '__main__':
    unittest.main(verbosity=2)
