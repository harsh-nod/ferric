# Position-Five R2 Boundary Results

The bounded CPU diagnostic passed on `mi350` in 1.050 seconds with all
26 tests passing, no skips and no postcheck errors. It used the original
position-5, layer-0 captures, not a new GPU/model run or substituted operands.

| Check | Exact words | Checked words |
| --- | ---: | ---: |
| Native rank 0 final hidden | 4096 | 4096 |
| Native rank 1 final hidden | 4096 | 4096 |
| Separate framework residual control | 4096 | 4096 |

Native replay order is FP32(+0 + rank 0), FP32(+ rank 1), BF16 round-to-nearest
even, FP32 addition of the captured BF16 residual, then BF16 narrowing. Both
native ranks' saved final arrays match the replay exactly. Framework control
uses only its own captured BF16 projection and residual. Their first residuals
are equal; their final hidden arrays differ at 14 rows per native rank.

The archive preserves 32 originals: source, the eight authentic inputs,
test output, the complete 4,096-row report, four derived arrays, original
terminal, collector and manifest. `README.md` is the unchanged source-time
description; this results file was added after retention.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Original terminal | 21551 | `bf16c755ede242da389e99a2a907e2907e8cf442c2e82fd62a41c0a7e9d2528b` |
| Original archive | 3888108 | `093ee5fbd23403bb94ebfa26dd57c0c1c43479055bebefe9a41c64e3dced931c` |
| All-row replay | 1008054 | `eee5940bdf309285cc68e03dd2e8889cf074024965c0466836c13d0e67409bdd` |

This checks the captured residual boundary. It is not an exact Down-dot
reference, proof of current machine-instruction order, explanation of the
position-five argmax, independent full-model acceptance or a performance result.
No checkpoint shard was read. The exact 256-generated-ID and decoded-byte
acceptance gate remains unchanged and unmet by this diagnostic.
