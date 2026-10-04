# Captured SiLU Materialization

On 2026-10-04, the independent diagnostic passed all **18 synthetic tests** on
ASROCK (`mi350-2`). Its subsequent retained-data run matched **12,288/12,288**
genuine framework products exactly. These are CPU checks of previously captured
GPU tensors, not a new GPU kernel run or full-model correctness result.

## Boundary Under Investigation

The selected checked Down2 image computes FP32 SiLU, multiplies by widened BF16
up, then narrows the product. The eager framework capture has an additional
BF16 storage boundary at the SiLU output. The control uses captured values:

```text
product = round_bf16(round_fp32(widen(captured_bf16_silu) * widen(captured_bf16_up)))
```

Exact integer multiplication implements FP32 nearest/ties-even rounding, followed
by the existing independent BF16 narrowing oracle. No exponential is evaluated.
Tests include an independent rational nearest-neighbor oracle, both precision
boundaries, signed zero, subnormals, overflow and nonfinite rejection.

## Conditional Native Comparison

Each rank has 6,144 elements. Predictions use the framework's captured BF16 SiLU
only where native and framework gate bits match, multiplied by the actual native
up. Different gate inputs are excluded from that prediction.

| Measurement | Rank 0 | Rank 1 |
| --- | ---: | ---: |
| Same gate and up | 5,702 | 5,723 |
| Same gate, different up | 209 | 235 |
| Different gate, same up | 225 | 181 |
| Different gate and up | 8 | 5 |
| Same-gate predictions matching native product | 4,317 / 5,911 | 4,416 / 5,958 |
| Same-gate predictions differing from native product | 1,594 | 1,542 |
| Native/framework differences with both inputs identical | 1,542 / 5,702 | 1,473 / 5,723 |

The last row isolates a difference inside the activation/product path from
gate/up input differences. It does **not** distinguish GPU exponential error,
FP32 evaluation order, and missing SiLU materialization: the native FP32 SiLU
intermediate was not captured. A separately compiled candidate must test the
additional boundary before attributing improvements to it.

## Reproduction And Limits

[Source and retained-data invocation](source/README.md), [tests](source/test_diagnostic.py),
[actual test log](pure/tests.log), and the [result ledger](result.json) are retained.
The real-data runner rechecked 102 consumed file identities, including genuine
framework tensors and the selected checked source, LLVM and image. It reused the
prior ownership checkpoint rather than repeating its GPU process audits.

The [publisher](publish.py) checks retained result/source/test identities and
extracts these measured counts; it does not rehash GPU tensor bodies or rerun
the arithmetic. The larger original result retains all partition and mismatch
indices outside Git. No tolerance, numerical admission or performance claim was
introduced. Full-model validation and the BF16 2,048/256, 700 tokens/s target
remain open.
