# Closed TP2 Peer Arithmetic V18

This separate gfx950 image has exactly two roots. It does not replace or
narrow the generic V4 crate. Host-owned peer mappings, producer completion,
allocation lifetimes and failure quarantine remain prerequisites.

| Root | Contract | Explicit Bytes |
| --- | --- | ---: |
| `ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18` | rows1/world2, active4096, Wave64, exactly64 workgroups | 168 |
| `ferric_qwen3_tp_peer_copy_bf16_v4` | unchanged V4 bit copy, rows1..16, rows*64 workgroups | 36 |

The consumer preserves ten slice pointer/length pairs and the rows/world u32
arguments. p0/p1/residual/output must have length4096; p2..p7 must be empty,
backed by valid live allocations. Inactive partials are never loaded. The
launch must contain exactly4096 threads. Other rows or worlds trap.

Arithmetic is `0.0f32 + p0 + p1`, then BF16 residual widened once, then exactly
one BF16 narrowing. Active values and every sum are checked for finiteness,
including the final narrowed result. Rank order and intermediate FP32
rounding are unchanged from the generic V4 world2 path. The copy function and
its launch attributes retain the exact V4 source body, including nonfinite
payload-preserving bit copies.

The closed consumer exists because the unchanged generic V4 launch/effect
envelope exceeded the original FAA compiler's production logical-storage
bound. No proof policy or compiler budget changes are allowed. A smaller
source contract does not itself prove successful emission or GPU correctness.

CPU tests exercise the actual scalar arithmetic macro with an independent
staged-f64/bit-rounding reference, two4096-element nonuniform generations,
single-rounding distinctions, finite edge cases, trap cases, and the closed
source/roster/ABI contract. Actual ELF ABI and ordinary admission are required
after checked emission; the source signature is not an actual metadata claim.
Native collective correctness and model execution require separate root-owned
gates. No latency, overlap, production-readiness or performance claim follows.
