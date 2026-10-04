# Projection-Residual Comparison Helper

All 20 synthetic tests passed on `mi350-2` on 2026-10-04, with no failures,
errors or skips. The eight source bodies and test controller were unchanged
after execution. This qualifies the comparison helper, not the candidate GPU
kernel or full-model numerical correctness.

## Checks

The helper consumes separately authenticated baseline and candidate captures.
It checks all 28 stage partitions, finite values, eleven evidence files,
rank consistency, and untouched KV regions. Fourteen upstream arrays must
match the baseline; KV comparisons use each run's own logical-slot mapping.
The candidate's final hidden state is not required to equal the old result.

Both output-projection and down-projection residuals are compared against the
independent integer FP32/BF16 oracle, using this order:

```text
s0 = round_f32(+0 + rank0_partial)
s1 = round_f32(s0 + rank1_partial)
p  = round_bf16(s1)
y  = round_bf16(round_f32(widen(p) + widen(residual)))
```

All rounding is ties-to-even, with finite checks at each boundary. Neither
rank partial is narrowed separately. Output projection uses the genuine
embedding as its residual; down projection uses the candidate's captured
first residual. Four comparisons cover 16,384 BF16 words. Disagreements are
reported, not hidden behind a new tolerance.

The unchanged framework mapping separately reports 24 BF16 comparisons.
Conditional agreement does not establish GEMM equivalence or independent
full-layer acceptance. Runtime, image, process and profile authentication
remain the caller's responsibility.

## Evidence

- [Actual test completion](evidence/complete.json) and [test transcript](evidence/tests.log).
- [Comparison source](source/comparison.py), [tests](source/test_comparison.py), and [bounded runner](run_pure.py).
- Unchanged dependencies: [current-stage helpers](../layer0-native-capture-v1/comparison/source/current.py) and [integer residual oracle](../output-residual-boundary-v1/source/residual_oracle.py).
- [Before](evidence/sources-before.json) and [after](evidence/sources-after.json) inventories pin every imported source body.

The runner records the original remote paths and exact source hashes. It uses
two CPU cores, nice level 10, hidden GPUs, a 2 GiB address-space limit and a
120-second CPU limit. Tests completed in 17.302 seconds. The source layout
and invocation are preserved in the runner; this directory is an evidence
snapshot, not a self-contained installed package.

The completion SHA-256 is
`00f8b30490a7566d449ffe4b0dde0fe6a53fc035b31162c5445ef9ea1b1ed3f3`.
Genuine candidate GPU capture and numerical comparison remain pending.
