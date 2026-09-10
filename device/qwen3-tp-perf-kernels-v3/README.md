# Qwen3 TP Performance Kernels V3

This opt-in, gfx950-only engineering crate keeps the nine v2 entry points and
adds up to six independently selectable kernels. It does not change the v2 source,
protected admission, model precision, or any Verus qualification claim.

The default `gfx950,mfma` feature set has an exact 15-root roster. Building with
`--no-default-features --features gfx950` has a distinct, exact 13-root roster
that omits only the two MFMA roots. A host must select the matching closed roster
and reject forced MFMA when the artifact does not contain those roots; missing
roots must never be silently accepted as a full profile.

| Variant | Weight layout | Workgroups | Explicit ABI bytes |
| --- | --- | --- | --- |
| `ferric_qwen3_tp_wave_gemv_bf16_v3` | `[n,k]` | `rows*n` | 68 |
| `ferric_qwen3_tp_wave_gemv_partial_f32_v3` | `[n,k]` | `rows*n` | 68 |
| `ferric_qwen3_tp_mfma_gemm_bf16_v3` | `[k,n]` | `n/16` | 68 |
| `ferric_qwen3_tp_mfma_gemm_partial_f32_v3` | `[k,n]` | `n/16` | 68 |
| `ferric_qwen3_tp_wave_paged_gqa_bf16_v3` | Unchanged paged KV | `rows*(32/world)` | 116 |
| `ferric_qwen3_tp_batch_residual_bf16_v3` | Not applicable | `rows*64` | 52 |

Every workgroup is exactly `[64,1,1]`. Projection arguments and shape tags match
v2, except that the MFMA weight allocation has an explicitly different layout.
The host must transpose the authenticated weights once, retain that resident
allocation, and select the matching kernel. It must not relabel the original
allocation or include transpose setup in steady-state inference timings.
Rows remain bounded to 1 through 16; TP1, TP2, and TP8 geometries are unchanged.

The wave variant partitions the reduction dimension among 64 lanes, then uses
a fixed shuffle reduction. It is intended for small decode batches. The MFMA
variant computes a full 16x16 tile with BF16 matrix instructions and FP32
accumulators; its checked activation view zero-fills inactive rows without
reading uninitialized padding. Both return FP32 TP partials where required and
BF16 column projections with a single final narrowing.
The FP32 partial wave kernel uses a fixed 192-step envelope with bounds-masked
loads for shorter reduction dimensions. Total checked read views preserve
collective convergence without exposing raw-pointer reads.

Paged attention computes two QK products per lane and shares the reduced score,
replacing 64 repeated serial 128-dimensional dot products. Its existing causal
position checks, physical-page bounds, online softmax and disjoint two-component
output ownership remain in place. Position and page metadata are broadcast as
exact bits before guarded accesses, making their control explicitly uniform.
All physical lanes participate in reductions. Numeric predicates accumulate
without lane exits until the final collective and reject before output stores.

FP32 association differs from v2 for both projection variants and attention.
These variants require numerical probes and frozen-token inference checks;
source-level work reduction is not a measured latency or throughput speedup.
Nonfinite computed results trap, and the driver must retain fail-closed worker
failure handling. The residual kernel is a local FP32-partial plus BF16-residual
operation, not a TP collective or permission to access peer allocations.

## Verification

Host tests, formatting, Clippy and emission run only on the private mi300x build
stage. Format this crate with `rustfmt --config skip_children=true` on its own
Rust files, preserving imported baseline modules byte-for-byte. Baseline unit
tests belong to the original v2 crate; v3 integration tests check the complete
15- or 13-entry generated roster and the six source ABIs.

`tools/probe.py --self-test` constructs 36 bounded fixtures without launching a
worker. An explicit `--run` requires the exact worker and artifact hashes, one
device identity, an output directory, and the existing hash-bound probe helper.
It checks 13 roots, input immutability, dense and sparse projections, rows 1/3/16,
causal page crossing, QK contributions from both dimension halves, active output
tails and residual precision. RMSNorm and embedding remain unexecuted by this
synthetic probe and need end-to-end inference coverage.
`--profile wave` selects 26 fixtures across 11 roots for the 13-root artifact.
`tools/test_probe.py` checks both fixture profiles without executing a worker.

The source has host-test coverage, but this README is not a GPU qualification.
Artifact emission, numerical execution and end-to-end frozen-token comparisons
must be recorded against exact source and compiler identities before enabling
any new variant. In particular, current MFMA emission is under investigation
for a generic checked-layout control-convergence admission failure.

The probe's single-dispatch wall times are diagnostic receipts, not an inference
benchmark. TTFT, TPOT and throughput need repeated, matched end-to-end workloads
with setup, cache state, active row count, variant and hardware identity recorded.
