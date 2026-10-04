# Linked RoPE AR4 Numerical Comparison

The newly emitted gfx950 prefix image completed four own-output Qwen3
forwards on MI350. Its captured tensors were compared on MI350 against the
retained independent framework reference: two genuine autoregressive runs
with fresh KV state and byte-identical repeated captures. The new native
run, previous native run and reference share the input chain
`9112 -> 67 -> 25 -> 576`; each produces `67, 25, 576, 2701`.
All 152 tensor slices are therefore comparable. Equal tokens are not
numerical acceptance, and none of the new complete slices matches bitwise.

## Measured Result

The result is mixed, not a uniform numerical improvement. Logit relative-L2
error improves at position 1, regresses at positions 2 and 3, and is unchanged
at position 0. Across all 152 slices, relative-L2 error improves for 90,
regresses for 24 and is unchanged for 38. These counts do not weight tensor
size or establish an acceptance criterion.

| Position | Old Logit Relative L2 | New Logit Relative L2 | Old Max Abs | New Max Abs |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0.0030448043 | 0.0030448043 | 0.0625 | 0.0625 |
| 1 | 0.0140315337 | 0.0094134774 | 0.25 | 0.20703125 |
| 2 | 0.0043564111 | 0.0117436782 | 0.125 | 0.1875 |
| 3 | 0.0045131941 | 0.0050785906 | 0.1875 | 0.125 |

Relative L2 is `||native - reference||_2 / ||reference||_2`; max absolute
error is `max(abs(native - reference))`. The metrics use decoded BF16
captures. Each position contains 36 layer-hidden slices and a final-norm
slice of 4,096 elements each, plus a 151,936-element logit slice.

The largest new layer-hidden relative-L2 error is 0.0244600764 at position 3,
layer 18, compared with 0.0249833633 for the previous image at that same
slice. This does not explain the later logit regressions. The data do not
isolate a causal explanation for each difference.

## Evidence And Scope

- [Full before/after table](table.md) and [structured table](table.json).
- [Actual comparison receipt](raw/complete.json) and [publication manifest](result.json).
- [New native GPU observation](../rope-indexed-ar4-native-v1/README.md).
- [Independent framework reference and previous native comparison](../projection-ar4-framework-v1/README.md).
- [Checked image emission](../rope-indexed-checked-emission-v1/README.md).
- [Comparator source](source/comparison.py), [metric implementation](source/diagnostics.py), and [18 policy tests](source/test_comparison.py).

All eighteen comparator tests passed on MI350, with no failures or skips
and unchanged source hashes. Their [primary-agent SSH observation](tests/primary-observation.json)
is explicitly not a remote process-supervisor receipt. The actual comparison
completed with no errors and successful input/source postchecks. It rehashed
eight native captures and eight reference captures; the data-only publisher
rechecks retained inputs and copies recorded metrics without rerunning a
model or recomputing tensor arithmetic. Private BF16 captures are not in Git.

No tolerance has been inferred from these observations. This candidate is
not numerically accepted. This four-step diagnostic does not cover the
single-request BF16 2,048-token prompt / 256-token decode workload and does
not measure throughput. All issue #42 M0-M7 acceptance gates and the
700 tokens/s target remain open. Next is localizing the numerical differences
before long-workload qualification or performance claims.
