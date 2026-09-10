# Qwen3 Peer Arithmetic V4

This additive gfx950 image contains exactly two roots and does not change the
V2 or V3 images. Kernels use ordinary, checked fe2o3 slices. They do not create
peer mappings or grant cross-device access. The host must own all allocations
in one process, prove destination-device access, preserve their lifetimes, and
complete every producer before dispatching either kernel.

| Symbol | Explicit Arguments | Bytes |
| --- | --- | --- |
| `ferric_qwen3_tp_peer_ordered_residual_bf16_v4` | `p0..p7: &[f32]`, `residual: &[u16]`, write-only BF16 output, `rows: u32`, `world: u32` | 168 |
| `ferric_qwen3_tp_peer_copy_bf16_v4` | BF16 source, write-only BF16 output, `rows: u32` | 36 |

Both accept rows 1 through 16 and exactly `rows*4096` active elements, with
64-lane workgroups and `rows*64` workgroups (maximum 1024). The ordered reduction
accepts TP2 or TP8. TP2 requires empty `p2..p7` slices backed by valid live
allocations; those expressions are never read. TP8 requires all eight partials
at exact active extent. Output and residual always have exact active extents.

Reduction uses FP32 `0+p0+p1+...` in rank order, then adds the BF16 residual once
and rounds to BF16 once. Every active input, intermediate sum, residual, final
FP32 value and narrowed value is checked for finiteness. Failure traps and must
poison the enclosing inference step. The copy preserves all 16-bit patterns,
including signed zero and nonfinite payloads; it is not a numeric conversion.

`compiler_expectation_roster_v4()` returns the exact, sorted two-root host
expectation. This image has separate admission from the projection/attention
image. Host scalar tests check the actual reduction macro, disabled-rank
non-evaluation, cancellation order, final rounding and finite traps. A synthetic
single-device arithmetic probe cannot prove peer reachability or multi-device
ordering; that requires the single-process transport integration probe.

All builds, formatting, tests and emission must run in an owned private mi300x
stage with bounded build concurrency. GPU runs belong to the integration lead.
No performance or production-qualification claim follows from source alone.
