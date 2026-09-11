import copy
from pathlib import Path
import tempfile
import unittest

import compare_tp_numerical_captures as compare
import replay_tp_numerical_capture as replay


class PairTests(unittest.TestCase):
    def test_pinned_replay_and_replacement_rejection(self):
        helper=compare.load_replay(Path(replay.__file__))
        self.assertEqual(helper.narrow(1.0),0x3f80)
        with tempfile.TemporaryDirectory() as root:
            path=Path(root)/"replaced.py"
            path.write_text("raise RuntimeError('must not execute')")
            with self.assertRaises(ValueError):
                compare.load_replay(path)

    def test_pair_identity_binds_every_named_policy_and_rows(self):
        left={"identity":dict.fromkeys(compare.IDENTICAL_FIELDS,"same"),"batch_ordinal":2,
              "scheduler_batch_id":2,"pool_batch_id":2,"execution_rows":[{"slot":0,"generation":1}],
              "projection":dict.fromkeys(("role","layer","rows","n","k","world","tag"),1)}
        left["identity"].update(projection="baseline",session_id="a")
        right=copy.deepcopy(left)
        right["identity"].update(projection="mfma",session_id="b")
        compare.pair_identity(left,right)
        for key in compare.IDENTICAL_FIELDS:
            wrong=copy.deepcopy(right);wrong["identity"][key]="different"
            with self.assertRaises(ValueError):
                compare.pair_identity(left,wrong)
        right["execution_rows"][0]["generation"]=2
        with self.assertRaises(ValueError):
            compare.pair_identity(left,right)

    def test_exact_bf16_and_fp32_difference_counts(self):
        left=b"\x00\x00\x80\x3f"
        right=b"\x00\x80\x00\x40"
        result=compare.difference(replay,left,right,2,2)
        self.assertEqual(result["different_bits"],2)
        self.assertEqual(result["maximum_absolute_difference"],1)
        self.assertEqual(result["first_changed_elements"][0]["column"],0)
        with self.assertRaises(ValueError):
            compare.difference(replay,left,right[:-1],2,2)

    def test_full_logits_ties_watch_and_corruption(self):
        logits=[0]*replay.VOCABULARY
        for token in compare.WATCH:
            logits[token]=0x3f80
        top=[{"token":token,"bf16_bits":0x3f80,"value":1.0} for token in compare.WATCH]
        top.extend({"token":token,"bf16_bits":0,"value":0.0} for token in range(14))
        row={"slot":0,"generation":1}
        summary={"row_identity":row,"top16":top,"watch":top[:2],"gpu_choice":9856,
                 "top1_minus_top2":0.0,"top_two_tied":True}
        actual,_=compare.check_head_summary(replay,logits,summary,row)
        self.assertEqual(actual,top)
        wrong=copy.deepcopy(summary);wrong["gpu_choice"]=17689
        with self.assertRaises(ValueError):
            compare.check_head_summary(replay,logits,wrong,row)
        logits[42]=0x7f80
        with self.assertRaises(ValueError):
            compare.check_head_summary(replay,logits,summary,row)


if __name__ == "__main__":
    unittest.main()
