# Qwen3 Tensor-Parallel Engineering Kernels

This standalone compilation unit adds six rank-local kernels and imports seven
unchanged roots from the protected aggregate. It does not change or inherit the
protected aggregate's twelve-root admission. Artifact intake must bind this
crate's exact source closure, compiler marker roster, target, and measured code.

The first path is one sequence and one token per launch. Model role 1 is Qwen3-8B
(hidden 4096, Q 32, KV 8, head dimension 128, intermediate 12288); role 2 is
Qwen3-0.6B (hidden 1024, Q 16, KV 8, dimension 128, intermediate 3072).
World size is exactly 1, 2, or 8. Heads and intermediate dimensions divide world
size; hidden states remain replicated. The contiguous local cache layout is
`[capacity, local_kv_heads, 128]`, with capacity 1 through 8192. Append overwrites
one position before attention reads exactly `count = position + 1` initialized
rows. Resetting a sequence does not authorize reading stale trailing rows.

## ABI

All launches use workgroup `[64, 1, 1]`. Slice records are `(ptr:u64,len:u64)`;
lengths count elements, not bytes. Scalars are ordered `u32` unless noted.
The explicit byte count excludes alignment and hidden AMDHSA arguments, which
must be checked against the emitted COV6 metadata.

| New root suffix | Ordered arguments after slice records | Explicit bytes | Grid |
| --- | --- | --- | --- |
| `tp_gemv_bf16_f32_bf16_v1` | A BF16, W BF16, output BF16; n,k,role,world,projection | 68 | n/64 |
| `tp_gemv_partial_bf16_f32_v1` | A BF16, W BF16, output FP32; n,k,role,world,projection | 68 | n/64 |
| `tp_swiglu_bf16_f32_v1` | gate BF16, up BF16, output BF16; role,world | 56 | local intermediate/64 |
| `tp_rope_v1` | Q BF16, K BF16, cos FP32, sin FP32, Q-out BF16, K-out BF16; position,role,world | 108 | 1 |
| `tp_kv_append_v1` | K BF16, V BF16, K-cache BF16, V-cache BF16; position,capacity,role,world | 80 | 1 |
| `tp_gqa_decode_bf16_f32_v1` | Q BF16, K-cache BF16, V-cache BF16, output BF16; count,capacity,role,world | 80 | local Q heads |

All six names have prefix `ferric_qwen3_`. Column projection tags are Q=1,
K=2, V=3, gate=4, up=5. Partial tags are O=1, down=2. Weights are compact
row-major `[n,k]`. RoPE takes exactly 64 cosine/sine elements for the supplied
position. Attention uses local Q-to-KV ratios 4 (8B) or 2 (0.6B), online softmax,
and no explicit scratch buffer. The exact fourteen-symbol roster is in
`src/contract.rs`; imported roots retain their existing ABI. The imported GEMM
module emits its MFMA root unconditionally, so image admission includes that
root even though the single-sequence controller does not select it for dispatch.

## Numerical And Proof Scope

O/down emit FP32 partial sums without residual or BF16 narrowing. The host must
sum ranks in deterministic order, add the replicated residual once, then narrow
once. Partitioning changes floating-point accumulation order: bitwise equality
to an unpartitioned dot product is not claimed. A cancellation regression makes
this difference explicit. Source-link tests compare Rust-tokenized kernel
arithmetic regions against the macro bodies exercised by host tests. Explicit
loop bodies preserve the compiler's source control-flow checks. Tests cover
exact binary projections, all model/rank GQA
mappings, RoPE head partitions, stable SwiGLU, and nonfinite rejection.

These numerical properties are Contracted, not Verus-proved. Host exponential
tests use host libm rather than the emitted OCML implementation; GPU numerical
validation remains separate. Tests and source hashes grant no dispatch or M1
qualification authority. Formal host layout/scheduling contracts live in the
engineering TP driver; this crate does not claim those theorems prove floating
point arithmetic or cross-GPU transport.

Default target remains `gfx942`; `--no-default-features --features gfx950`
selects `gfx950`. Missing, overlapping, and mismatched target selections fail
closed through the shared target contract. Both use Wave64, xnack-, and COV6.
