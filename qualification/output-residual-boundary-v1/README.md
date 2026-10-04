# Projection Rounding Boundary

This CPU-only replay on `ssh mi350-2` (ASROCK) tests a specific explanation for
the [current layer-zero discrepancy](../layer0-native-capture-v1/comparison/README.md).
It uses the captured native FP32 O-projection partials and genuine framework
embedding, projection and residual values. No model was reloaded and no GPU
kernel was launched. This is conditional arithmetic evidence, not a new
end-to-end numerical result.

## Formulas

The existing v18 residual kernel adds both FP32 rank partials, adds the BF16
residual widened to FP32, then rounds the result to BF16. The framework
materializes its projection in BF16 before adding the residual.

Let `R32` and `R16` mean round-to-nearest, ties-to-even to FP32 and BF16.
BF16 operands are widened exactly before FP32 addition. All intermediates must
remain finite; the existing integer oracle also preserves signed-zero behavior.

```text
s0 = R32(+0 + rank0_partial)
s  = R32(s0 + rank1_partial)
A  = R16(R32(s + embedding))       # Existing native boundary
B  = R16(R32(R16(s) + embedding))  # Materialize projection first
C  = R16(R32(framework_O + embedding))
```

The partials are not individually narrowed. `B` changes only the rounding
boundary after their ordered sum. `C` is a control that does not use native
projection partials.

## Actual Results

Every comparison covers all 4,096 BF16 values, including signed zeros.

| Compared Values | Exact Words | Different Words |
| --- | ---: | ---: |
| A versus native rank 0 | 4096 | 0 |
| A versus native rank 1 | 4096 | 0 |
| A versus framework residual | 2886 | 1210 |
| B versus native rank 0 | 2888 | 1208 |
| B versus native rank 1 | 2888 | 1208 |
| B versus framework residual | 4094 | 2 |
| C versus framework residual | 4096 | 0 |
| R16(s) versus framework projection | 4093 | 3 |

Adding the BF16 projection boundary reduces residual mismatches from 1,210
to two for these captured inputs. It does not make the TP projection identical
to the framework's full GEMM. The three remaining projection differences are
adjacent BF16 values; two survive the residual addition:

| Index | R16(s) / Framework Projection | B / Framework Residual |
| --- | --- | --- |
| 1024 | `bc9d / bc9e` | `bd2d / bd2e` |
| 1356 | `b492 / b491` | Equal |
| 3444 | `3b88 / 3b87` | `bb86 / bb87` |

These are BF16 hexadecimal bit patterns. The replay supports testing a
versioned materializing kernel, but it does not prove upstream GEMM correctness
or establish a universal causal explanation. In particular, native embedding
equality and hardware arithmetic premises are not independently discharged by
this conditional replay. The next GPU experiment must execute the changed
kernel and compare the resulting genuine chain, not substitute these replayed
values into a claimed model result.

## Validation And Evidence

All 12 arithmetic/policy tests passed on ASROCK with GPU visibility disabled.
They cover ties, ordered additions, signed zero, subnormals, nonfinite input,
overflow at the new materialization boundary, complete extents and oracle
authentication. The replay rehashed 260 prior comparison inputs and validated
the native stage slices and both genuine framework passes before calculation.
The final read ledger contains 271 original/retained pairs, with postchecks.

- [Actual eight-comparison replay](complete.json) and [publication ledger](result.json).
- [Arithmetic diagnostic](source/boundary.py), [integer oracle](source/residual_oracle.py),
  [tests](source/test_boundary.py), and [12-test receipt](pure/complete.json).
- [Retained-data runner](runner/run.py) and [actual input plan](inputs/plan.json).

Completion SHA-256:
`39c12232b3504eccbca898383b70244093fe930b9c37656cbec1515cd30eaf11`.
Tensor binaries and model weights remain outside Git. No tolerance was fitted,
no new image was admitted, and no full-layer/model correctness or performance
claim is made. All issue #42 M0-M7 milestones and the 700 tokens/s BF16
single-request 2,048/256 target remain open.
