# Current Native Versus Framework Intermediates

The retained current-image GPU capture was compared on `ssh mi350-2`
(ASROCK) with the independent Qwen3-8B BF16 framework capture. Both runs use
the genuine model token `9112`, position zero and fresh KV state. The new
framework result reproduces the original framework reference exactly; the
new native capture reproduces the current native full-forward output exactly.
This is a numerical diagnostic, not a correctness or performance qualification.

## Measured Stages

Each exact count is the number of identical BF16 words divided by the number
of words in that rank's compared stage. Errors are measured against the
independent framework, not against the previous native implementation.

| Stage | Rank 0 Exact | Rank 1 Exact | Max Abs, Rank 0 | Max Abs, Rank 1 | Relative L2, Rank 0 | Relative L2, Rank 1 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Input norm | 4096/4096 | 4096/4096 | 0 | 0 | 0 | 0 |
| QKV projection | 3071/3072 | 3072/3072 | 4.76837e-7 | 0 | 1.95960e-7 | 0 |
| Query norm / rotary | 2047/2048 | 2048/2048 | 1.52588e-5 | 0 | 2.17379e-7 | 0 |
| Current key | 512/512 | 512/512 | 0 | 0 | 0 | 0 |
| Current value | 512/512 | 512/512 | 0 | 0 | 0 | 0 |
| Attention output | 2048/2048 | 2048/2048 | 0 | 0 | 0 | 0 |
| First residual | 2886/4096 | 2886/4096 | 0.0078125 | 0.0078125 | 0.00304620 | 0.00304620 |
| MLP input norm | 2932/4096 | 2932/4096 | 0.00390625 | 0.00390625 | 0.00287141 | 0.00287141 |
| Gate projection | 3279/6144 | 3337/6144 | 0.0078125 | 0.0078125 | 0.00262132 | 0.00256162 |
| Up projection | 3184/6144 | 3262/6144 | 0.00390625 | 0.00390625 | 0.00280865 | 0.00267409 |
| Activation product | 1844/6144 | 1909/6144 | 0.0078125 | 0.00390625 | 0.00363738 | 0.00253529 |
| Layer-zero output | 1834/4096 | 1834/4096 | 0.0625 | 0.0625 | 0.00357857 | 0.00357857 |

The first observable difference is one BF16 step in one rank-0 QKV value.
However, both attention outputs are byte-identical to the framework. For
this position-zero capture, the larger first-residual difference therefore
cannot be attributed to different attention output values. The next useful
isolation boundary is output projection, cross-rank partial summation and
residual addition, including where BF16 rounding occurs.

This does not yet identify the arithmetic cause. Later rows include differences
propagated from earlier stages and are not isolated operator error measurements.
FP32 rank-local output/down partials were checked for finiteness but deliberately
excluded from comparisons to full BF16 framework projections. No tolerance
was chosen after observing these errors.

## Validation

The CPU-only comparison checked all seven retained native process leaves,
their recorded arguments and ownership, six historical idle-device audits,
the capture structure and both native final-hidden joins. It also checked the
framework receipt and its 21 directly linked process results. These are
retained-run checks, not new GPU audits or a second native execution.

The comparison adapter passed 15 tests on `mi350-2`; the unchanged intermediate
comparator separately passed 18 tests. All source postchecks passed. These
synthetic tests validate the diagnostic machinery, not the model's numerics.

- [Actual comparison](complete.json) and [publication ledger](result.json).
- [Comparator](source/current.py) and [retained-run adapter](runner/run.py).
- [15-test adapter receipt](pure-adapter/complete.json) and
  [18-test comparator receipt](pure-comparator/complete.json).
- [Native capture](../README.md) and
  [independent framework capture](../../layer0-framework-capture-v1/README.md).

The comparison completion SHA-256 is
`758c2479a3f5a5f78e6ff088adb70a31ca5fb19c86e5a25cf718c3e1e46f4dab`.
Raw captures and model weights remain outside Git. Full-model correctness,
calibrated timing, GPU overlap and the single-request BF16 2,048/256 target of
700 tokens/s remain unqualified. All issue #42 M0-M7 milestones remain open.
