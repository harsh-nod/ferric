# Projection-Residual Native Capture

The new checked fe2o3 gfx950 residual kernel ran on `mi350` on 2026-10-04,
through the separately qualified Rust parent and worker. Both residual stages
matched the independent conditional arithmetic oracle on both ranks:
**16,384 of 16,384 BF16 words were exact**. This is a layer-zero result, not
full-model numerical acceptance or a decode performance measurement.

## Arithmetic Change

The candidate materializes the combined projection in BF16 before adding the
BF16 residual, preserving the framework's intermediate storage boundary:

```text
s0 = round_fp32(+0 + partial_rank0)
s1 = round_fp32(s0 + partial_rank1)
p  = round_bf16(s1)
y  = round_bf16(round_fp32(widen(p) + widen(residual)))
```

The old formula added the residual before narrowing the projection. The checked
[kernel and CPU tests](../projection-residual-cpu-v1/README.md),
[generic lowering and ISA inspection](../projection-residual-lowering-v1/README.md),
and [988-test runtime qualification](../projection-residual-runtime-v1/README.md)
preceded this genuine hardware run. Existing runtime routes remain unchanged.

## Actual Checks

- One native attempt, no retries or forced cleanup; the parent and worker closed
  and were reaped. All six selected-device audits passed.
- All 28 typed arrays were retained; fourteen pre-residual arrays matched the
  old capture. Both complete KV caches passed active-slot and untouched-byte checks.
- Four independent residual comparisons each matched all 4,096 words. The Down
  comparison uses the candidate first residual as its skip, not the old or
  framework first residual.
- The independent framework comparison produced 24 stage/rank rows. The
  [supervisor](../projection-residual-capture-supervisor-v1/README.md) and
  [numerical helper](../projection-residual-comparison-v1/README.md) separately
  passed 34 and 20 synthetic tests; the new runtime-audit selector passed nine.

## Framework Comparison

Both ranks have the following values. The old measurements come from the
[prior genuine capture comparison](../layer0-native-capture-v1/comparison/README.md),
using the same framework reference, token 9112 and position zero.

| Stage | Old Exact Words | New Exact Words | Old Max Absolute Error | New Max Absolute Error | Old Relative L2 | New Relative L2 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| First residual | 2,886 / 4,096 | 4,094 / 4,096 | 0.0078125 | 0.000244140625 | 0.0030462006 | 0.0000543881 |
| Final hidden | 1,834 / 4,096 | 2,728 / 4,096 | 0.0625 | 0.0078125 | 0.0035785690 | 0.0017142513 |

The earliest observable framework difference remains one rank-0 QKV word.
Gate, up and activation-product differences also remain. These are cumulative
stage comparisons, not proof that any single upstream operation caused a later
difference. No tolerance was fitted to these results or used to accept them.

## Evidence And Limits

The [publication ledger](result.json) and [complete measured-row table](table.json)
bind the original GPU completion
`4f25030567c470062fd50862876bc35e93d080f1778c0be4ca36f2b39af4199a`
and independent CPU comparison
`affe711d0dcac8e95609396c74e0f546ca09d9d6a4abf798eab5bfc72a7e7551`.
The [comparison runner](compare_retained.py) is retained with its original bytes.
Raw capture buffers, binaries and model weights remain outside Git. The original
controllers require the retained evidence layout documented in their qualification
pages; these records are not a standalone installed test package.

The residual oracle is conditioned on the actual captured FP32 partials. Its
exact agreement does not establish GEMM accuracy, complete-layer equivalence,
36-layer numerical stability or the requested 2,048-prompt/256-decode workload.
Full-model evaluation, calibrated timing, overlap and 700 tokens/s remain open.
