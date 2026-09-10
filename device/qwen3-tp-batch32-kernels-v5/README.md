# Qwen3 TP Batch32 Kernels V5

Independent opt-in 32-row profile for gfx950. V2/V3 remain frozen at 16 rows.
This crate does not change production defaults or grant launch authority.
The source pins current public fe2o3 `3e74a9324a5acd7107e96a4a9b5319d3dd5ecde8`.

The full `gfx950,mfma` feature profile has exactly 15 roots. The explicit
`--no-default-features --features gfx950` profile has 13, omitting only the
two MFMA roots. `compiler_expectation_roster_v5()` exposes that closed roster.
All model-specific roots use the `ferric_qwen3_tp_batch32_` prefix and `_v5`
suffix. The unchanged `qwen3_rmsnorm_v1` root already supports the required
flattened hidden/head rows. All argument byte layouts match their V2/V3 roles.

## Geometry

Rows 1..32, world sizes 1/2/8, hidden width4096, intermediate width12288,
vocabulary151936, head dimension128. Physical pages remain512 with16 tokens
per page and8192 maximum context tokens. Page-table/trig/position and activation
capacities grow to32 rows, not the page size or model dimensions.

Every workgroup has64 lanes. Grid workgroup counts:

| Root Role | Workgroups |
| --- | --- |
| Scalar/MFMA projection | `ceil(rows / 16) * (n / 16)` |
| Wave projection | `rows * n` |
| Paged attention | `rows * (32 / world)` |
| Embedding / TP1 residual | `rows * 64` |
| SwiGLU | `rows * (12288 / world) / 64` |
| RoPE / argmax | `rows` |
| Paged append | `1` |

Scalar/MFMA projections use real two-dimensional row tiles in one launch.
Tile16 geometry is preserved: tile rows0/1 own activation rows0..15/16..31.
There are no hidden serial16 host microbatches. MFMA loads zero for inactive
rows through checked matrix views; no padded activation reads are required.
Scalar and wave weights remain NxK; MFMA weights remain separately resident KxN.

## Admission

The host must explicitly select this source/profile and its independently
admitted artifact, allocate32-row workspaces/metadata/trig tables, and enable
32-row scheduler/chunk admission. Frozen V4 peer kernels still cap rows at16
and must be rejected with this profile until a separate peer32 image exists.
Host-staged TP8 and device-local TP1 residual are valid integration choices.

Output ownership, page bounds, per-row causality, distinct append slots,
input finiteness and final narrowing checks remain in source. Scalar order
matches V2 per row. Wave and MFMA reassociation still require independent
numerical and end-to-end model qualification. Emission, native GPU tests,
and performance are separate evidence stages; none is implied by this README.
