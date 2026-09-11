# Draft06B Batch32 Engineering Kernels V10

This is a separate, closed14 Draft06B TP1 source family. It does not change a
target v5/v8/v9 kernel, target default, model identity, pool identity or
speculative completion authority. The crate alone does not execute a model.
Native and independent fixed-reference model qualification remain required.

`compiler_expectation_roster_v10()` supplies the compiler-generated typed
expectation roster. `contract.rs` lists the exact symbols, explicit ABI bytes,
maximum grids and role-local projection geometry. Every workgroup is Wave64.
The complete roster must be emitted/admitted together; a partial image must
not be called v10. The `gfx950` feature is required, not a numerical mode.

## Arithmetic And Geometry

| Role | N | K | Output | Tag |
| --- | ---: | ---: | --- | ---: |
| Query | 2048 | 1024 | BF16 | 1 |
| Key | 1024 | 1024 | BF16 | 2 |
| Value | 1024 | 1024 | BF16 | 3 |
| Gate | 3072 | 1024 | BF16 | 4 |
| Up | 3072 | 1024 | BF16 | 5 |
| Attention output | 1024 | 2048 | FP32 | 1 |
| Down | 1024 | 3072 | FP32 | 2 |
| LM-head | 151936 | 1024 | FP32 | 6 |

Scalar projections retain ascending FP32 multiplication/addition and four-row
weight reuse. MFMA projections retain checked row-major input views, KxN
weight views, 16x16 tiling, zero-filled inactive input rows and masked stores.
Each projection launches `ceil(rows/16) * (N/16)` groups for 1..32 rows. MFMA
and scalar reduction order are distinct numerical profiles. Both require
separate real-model checks; mathematical equivalence is not token equivalence.

Norm is pure-only at width1024/rows1..32 or width128/headrows1..512 with
epsilon1e-6 and empty residual/fused-output slices. Its unchanged generic
arithmetic follows a stronger draft-only entry guard. Embedding and residual
width is1024; SwiGLU width3072. RoPE uses Q16/KV8/head128, and GQA maps each
pair of query heads to one KV head. Paged append/attention retain 16-token
pages, at most512 physical pages, stride at most512 and context at most8192.
Duplicate append slots, nonfinite values, causal page access and exact output
ownership remain checked. Invalid device-fault fixtures are host-only.

The full 28-layer KV payload is939,524,096 bytes. The optional32-row FP32
logits workspace is19,447,808 bytes. These are arithmetic payload bounds, not
measured RSS, and exclude other workspaces and resident transposed weights.
The model's LM-head must be bound to the authenticated tied draft embedding;
the kernel ABI does not authenticate that identity by itself.

## Admission And Future Driver

A future role-checked constructor must authenticate Draft06B, its own model
scope/pool, and an actually loaded image with this complete roster before
allocating buffers. Do not use a target TP2 shard as a draft alias. Initial
scope excludes wave, peer, large-KV, sequences, ordered batches, replicas and
target numerical capture. Target constructors and prior image labels remain
unchanged. No public conversion from ordinary outputs into a draft-completion
result is introduced here. The later driver must consume the opaque
`EngineeringTpSpeculativeDraftWorkV1` and seal completion only after exact
role-local execution, including the distinguished full-accept catch-up work.

## Gates

Host tests cover the exact closed roster/typed ABI, every projection role and
grid, active/tail ownership, real-width dyadic references, FP32 head ties,
nonfinite traps, ratio-two GQA, causal inaccessible pages and duplicate append
slots. Parsed-source comparisons bind all14 emitted bodies and retained
helpers to explicitly listed source changes, including MFMA fragment loops.
They do not constitute native execution, Verus qualification, speculative
decoding or M1 completion.

Root-owned native fixtures should cover rows1/5/17/32 and all projection
shapes, full-vocabulary head/argmax with ordinary fixture IDs, Q16/KV8 norm and
RoPE, and same-sequence page-boundary attention at positions15/16/8191. Check
full active outputs, unchanged read-only inputs and poisoned tail/guard bytes;
use distinct KV-head values to reject a ratio-four mapping. No native launch
belongs in CPU test scripts. Frozen target images and past measurements must
keep their original compiler/source provenance.
