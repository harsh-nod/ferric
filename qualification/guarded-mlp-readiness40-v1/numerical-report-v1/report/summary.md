# Readiness40 Numerical Diagnostics

Forty authentic prompt-fed positions, zero generated tokens, independent own KV caches.
This is not the full 2,048-input/256-output workload and does not establish numerical acceptance.

**39/40 argmax matches; 1/152 selected tensors bit exact.**

| Mismatch position | Input token | Native argmax | Reference argmax | Tensor capture |
| --- | --- | --- | --- | --- |
| 5 | 271 | 9112 | 2 | NOT captured |

Position 5 has only authenticated original argmax records. Its logits, tensor errors, and margin were not captured.

| Selected position | Native / reference argmax | Logits relative L2 | Logits exact words | Logits max absolute error |
| --- | --- | --- | --- | --- |
| 0 | 67 / 67 | 0.00304480431964523 | 90814 / 151936 | 0.0625 |
| 15 | 374 / 374 | 0.006475731989600666 | 45789 / 151936 | 0.25 |
| 16 | 389 / 389 | 0.00688652912696301 | 40401 / 151936 | 0.125 |
| 39 | 11 / 11 | 0.006560731719287106 | 39947 / 151936 | 0.1875 |

![Hidden-state relative L2 error across 36 layers](hidden-relative-l2.svg)

The plot displays the recorded relative-L2 values multiplied by 100, with a linear axis. The CSV preserves the original metric values, without rounding or a new threshold.
BF16-step distance is an encoding-order diagnostic, not a numerical acceptance rule.

[All 152 tensor rows](tensor-metrics.csv) | [All 40 argmax records](argmax-diagnostics.csv)

No performance, overlap, full-model acceptance, or production claim. No model execution or metric recomputation is performed by this renderer.

Input: 94179 bytes, SHA-256 `50dcd3b01a95164186e517005719de46bc321396089762cf40e9013be14c0494`.
