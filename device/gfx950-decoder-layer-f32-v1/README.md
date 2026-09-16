# Fixed-Shape gfx950 Decoder Layer

An engineering numerical/compiler/runtime baseline for Ferric issue #42.
This is a complete tiny decoder-layer computation in ordinary attributed
fe2o3 Rust, not a full model, a production-qualified backend, or a performance
claim. Its independent per-workitem ownership does not implement the planned
cooperative worker scheduler.

The kernel computes input RMSNorm, Q/K/V projections, per-head Q/K RMSNorm,
RoPE, causal grouped-query attention over two prefix tokens plus the current
token, output projection and residual, post-attention RMSNorm, gate/up
projections, SwiGLU, and down projection with residual. All stage values are
observable through separate disjoint checkpoints.

## Shape And Launch

- Hidden width 4, query heads 2, KV heads 1, head dimension 2, intermediate width 4.
- Exactly 256 independent requests. One complete request per workitem.
- Required workgroup dimensions 128x1x1: two wave64 waves.
- AQL grid dimensions 256x1x1: two workgroups, not 256 workgroups.
- Source `max_grid=[2,1,1]` bounds workgroup count.
- No atomics, barriers, inter-request communication, shared scratch, or MFMA.
- Distinct per-request output blocks; read-only input/weight slices.

## Explicit ABI

Symbol: `ferric_gfx950_decoder_layer_f32_v1`.

Three pointer-plus-64-bit-length slice records, in this exact order. Confirm
these offsets against emitted metadata before using the engineering client;
this table does not substitute for a compiler artifact.

| Argument | Pointer offset | Length offset | F32 elements | Bytes |
|---|---:|---:|---:|---:|
| inputs | 0 | 8 | 3072 | 12288 |
| weights | 16 | 24 | 110 | 440 |
| output | 32 | 40 | 10240 | 40960 |

Explicit kernargs occupy 48 bytes. Compiler-selected implicit argument records,
alignment, LDS and private resource sizes must be obtained from the actual
emitted metadata and matched by the launch client.

Each 12-float request input record contains hidden[4], prefix keys[2,2], then
prefix values[2,2]. Prefix keys are already RoPE-transformed. The current token
is at absolute position 2; cosine/sine are supplied in the weight record.

All projection matrices are row-major `[output_channel,input_channel]`, with
no biases. The single weight record is shared read-only across requests.

| Weight data | F32 offsets |
|---|---|
| Input RMS scale[4] | 0..4 |
| Wq[4,4] | 4..20 |
| Wk[2,4] | 20..28 |
| Wv[2,4] | 28..36 |
| Q-head RMS scale[2] | 36..38 |
| K-head RMS scale[2] | 38..40 |
| Wo[4,4] | 40..56 |
| Post-attention RMS scale[4] | 56..60 |
| Gate[4,4] | 60..76 |
| Up[4,4] | 76..92 |
| Down[4,4] | 92..108 |
| RoPE cosine, sine | 108,109 |

Each 40-float output record stores these checkpoints:

| Checkpoint | F32 offsets |
|---|---|
| Input RMSNorm | 0..4 |
| Rotated Q | 4..8 |
| Rotated current K | 8..10 |
| Current V | 10..12 |
| GQA attention | 12..16 |
| Attention projection plus residual | 16..20 |
| Post-attention RMSNorm | 20..24 |
| Gate projection | 24..28 |
| Up projection | 28..32 |
| SwiGLU activation | 32..36 |
| Final down projection plus residual | 36..40 |

## Numerical Contract

All storage and intermediate arithmetic is F32. RMSNorm epsilon is 1e-6.
Each query head has its own two-element norm; both share the same Q norm
weights. RoPE maps `(a,b)` to `(a*cos-b*sin,b*cos+a*sin)`. Both query heads
use the same KV head. Attention uses scale 1/sqrt(2) and a max-subtracted softmax
over exactly three initialized tokens. SwiGLU uses a sign-stable sigmoid.

The implementation spells out left-associated scalar reductions. The admitted
compiler may contract multiplication/addition and chooses its supported device
transcendental implementation. Independent tests must report tolerance and
errors at all 40 checkpoints, not assume bitwise equality. This is deliberately
a distinct F32 diagnostic policy, not BF16 Qwen numerical equivalence.

## Compilation And Evidence

Use the pinned fe2o3 `nightly-2026-04-03` toolchain and the ordinary
`fe2o3-rustc-extract` production-source route with target CPU `gfx950` and
wave64. Keep any local dependency path overrides in external build
configuration and record their exact source identity. Do not hand-author IR,
assembly, or HIP as a substitute for a source-lowering failure.

The package is an isolated device engineering workspace, outside Ferric's
verified production Cargo workspace. Rust source presence and host shape tests
do not establish successful device lowering, GPU numerics, proof, or release
qualification. Parent issue evidence must retain compiler/source/artifact
hashes, metadata, complete launch geometry, all numerical cases, and cleanup.

The first successful Rust-to-gfx950 build is recorded in
[the engineering build evidence](../../qualification/gfx950-decoder-layer-f32-v1/BUILD.md).
It includes the exact source, extractor executable and backend library hashes,
upstream-decoded LLVM, measured ROCm 7.2.0 providers, and final HSACO identity.
Numerical dispatch results belong to the independent qualification harness;
the build record does not substitute for those results or production authority.
