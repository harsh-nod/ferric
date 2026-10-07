# Qwen3 TP Batch32 Kernels V5

Independent opt-in 32-row profile for gfx950. V2/V3 remain frozen at 16 rows.
This crate does not change production defaults or grant launch authority.
The source pins observed upstream fe2o3
`1a5999f6e1c5f2363bc2d525af65e84c46502ce6`, migrated from the prior manifest's
`5a503c04f5ae107a3b3e951ec970b36c5d6a9a79`. This pin migration is unqualified:
older SDK, image and native results do not qualify this revision. Preserve the
prior lockfile as baseline evidence before regenerating it with Cargo on the
approved remote CPU host. Both feature profiles require fresh SDK and emission
qualification; K2 remains default-off and its performance gain is unmeasured.

The full `gfx950,mfma` feature profile has exactly 15 roots. The explicit
`--no-default-features --features gfx950` profile has 13, omitting only the
two MFMA roots. `compiler_expectation_roster_v5()` exposes that closed roster.
All model-specific roots use the `ferric_qwen3_tp_batch32_` prefix and `_v5`
suffix. The unchanged `qwen3_rmsnorm_v1` root already supports the required
flattened hidden/head rows. All argument byte layouts match their V2/V3 roles.

## Geometry

Rows 1..32, world sizes 1/2/8, hidden width 4096, intermediate width 12288,
vocabulary 151936, head dimension 128. Physical pages remain 512 with 16 tokens
per page and 8192 maximum context tokens. Page-table/trig/position and activation
capacities grow to 32 rows, not the page size or model dimensions.

Every workgroup has 64 lanes. Grid workgroup counts:

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
Tile16 geometry is preserved: tile rows 0/1 own activation rows 0..15/16..31.
There are no hidden serial16 host microbatches. MFMA loads zero for inactive
rows through checked matrix views; no padded activation reads are required.
Scalar and wave weights remain NxK; MFMA weights remain separately resident KxN.

## Admission

The host must explicitly select this source/profile and its independently
admitted artifact, allocate 32-row workspaces/metadata/trig tables, and enable
32-row scheduler/chunk admission. Frozen V4 peer kernels still cap rows at 16
and must be rejected with this profile. The separate V6 peer32 source has an
independent two-root roster and requires its own artifact admission and native
qualification. Host-staged TP8 and device-local TP1 residual remain valid
integration choices.

Output ownership, page bounds, per-row causality, distinct append slots,
input finiteness and final narrowing checks remain in source. Scalar order
matches V2 per row. Wave and MFMA reassociation still require independent
numerical and end-to-end model qualification. Emission, native GPU tests,
and performance are separate evidence stages; none is implied by this README.

The four tiled projection kernels keep the bounded `((rows + 15) / 16)`
arithmetic explicit for ranked/MIR admission, with a focused Clippy allowance.
MFMA tile coordinates derive from authenticated WG64 invocation indices. The
row/column guards precede lossless u8/u16 narrowing, exposing bounded offset
arithmetic without changing the accepted coordinates or weakening checks.

## Experimental K2 Prefill

The non-default `prefill-mfma-k2` feature selects `src/projection_k2.rs` under
the same `projection` module and implies `mfma`. Ordinary builds retain the
original `src/projection.rs`; no adapter enables the candidate automatically.
The candidate buffers two checked K16 fragment pairs and consumes them in the
original single-accumulator order. Shapes, ABI, root names, row tails and final
finite/narrowing checks are unchanged. The two Wave roots are byte-identical.

On the remote CPU build host, compare the existing default profile against
`--no-default-features --features gfx950,prefill-mfma-k2`. Emission must use that
same candidate feature selection and its newly generated compiler roster;
an existing baseline handoff or image cannot be relabeled as the candidate.
Any future adapter qualification must explicitly enable the dependency feature
and bind the candidate image. The existing synthetic probe accepts the unchanged
ABI, but its two projection shapes do not qualify all admitted K widths.

`tests/mfma_k2_model.rs` is independently compilable with `rustc --test` and
screens the feature boundary, source schedule, and host tail/order models.
`tests/mfma_k2.rs` additionally checks the AST and actual checked SDK constructors.
Neither is an MFMA emulator. Both feature profiles still require remote Cargo
checks, native full-buffer parity, ISA scheduling/register review and a matched
model comparison before performance or integration acceptance.
