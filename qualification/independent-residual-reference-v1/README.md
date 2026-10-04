# Independent Residual Capture Checks

All 18 focused CPU tests passed on `ssh mi350` in 0.453 seconds, with no
failures or skips. Source hashes matched before and after execution.
The exact tested source, transcript, command and result are retained here.

The comparator reuses the unchanged, hash-pinned integer residual oracle.
It checks two rank-local FP32 partials and replicated BF16 residual/output
rows, then connects the post-attention output to the post-MLP residual input.
Each stage requires exact agreement for all 8,192 output words, including
signed zeros. There is no new tolerance.

Tests cover cancellation, ties, early rounding, intermediate/output overflow,
nonfinite values, byte extents, corrupted captures, replica mismatch,
first-stage failure, second-stage input selection and helper substitution.
Rank swapping is arithmetically invariant at TP2; source/capture identity must
be established separately and is not inferred from numerical agreement.

From this directory, run the portable suite with:

```sh
python3 -B -m unittest discover -s source -p test_residual_reference.py -v
```

`run_tests.py` records the actual MI350 invocation body with its fixed path,
UID, affinity and source pins. The source README preserves the author's
pre-execution notes; `result.json` and `tests.log` record the subsequent run.

This validates the capture-comparison component using synthetic test inputs.
It does not check new GPU captures, upstream O/MLP numerics, a full layer,
the complete model, runtime prerequisites, production admission or throughput.
