# Qwen3 Key RMSNorm Fixture

This is a completion-ordered consumer for the real Qwen3-0.6B layer-0 key
projection. It is an engineering baseline, not a single-launch megakernel,
tensor-publication protocol, benchmark, or production runtime qualification.

## Fixed Contract

`ferric_gfx950_qwen3_knorm_v1` accepts four slices, in this order:

| Argument | Role | Elements |
| --- | --- | --- |
| `inputs: &[f32]` | Completed projection output, immutable | 1024 |
| `weights: &[u16]` | BF16 `model.layers.0.self_attn.k_norm.weight`, immutable | 128 |
| `quantized` | Disjoint BF16 key checkpoint output | 1024 |
| `output` | Disjoint BF16 normalized key output | 1024 |

The explicit ABI is four pointer/length pairs: 64 bytes, alignment 8, pointer
offsets 0/16/32/48 and length offsets 8/24/40/56. The harness must keep both
outputs disjoint from each other and the read inputs. It must observe producer
completion and the required runtime visibility transition before this dispatch.

Launch exactly 512 work-items in workgroups of 128: four workgroups, two full
wave64 subgroups per workgroup. One wave handles one head of 128 elements.
Lane `l` owns columns `l` and `l+64`, using the existing
`RowStriped2D<Index1D,64,2>` write capability. Total-read views preserve full-wave
convergence before reduction. No raw pointers or handwritten IR are used.

## Numerical Sequence

The reference operation order is pinned to Transformers commit
`0720e206c6ba28887e4d60ef60a6a089f6c1cc76`,
[`Qwen3RMSNorm`](https://github.com/huggingface/transformers/blob/0720e206c6ba28887e4d60ef60a6a089f6c1cc76/src/transformers/models/qwen3/modeling_qwen3.py#L62-L76).
The checkpoint is `Qwen/Qwen3-0.6B` revision
`c1899de289a04d12100db370d81485cdf75e47ca`.

1. Round each completed FP32 projection value to BF16 with ties to even. Emit
   these bits in `quantized` for independent stage validation.
2. Widen those BF16 values to FP32, square two per lane, add the pair, then
   reduce across wave64. Multiply by exact `1/128` and add FP32 `1e-6`.
3. Compute square root followed by FP32 division of one by that result.
4. Multiply each key by this scale and round to BF16 **before** multiplying
   by the BF16 norm weight. Round the weighted result to BF16 again.

Step 3 is an explicit alternative to eager PyTorch's `rsqrt`, not a bitwise
equivalence claim. Reduction error alone does not bound square root/division.
The independent reference must freeze a finite-input domain and a bound for
the actual emitted operations before GPU evaluation. Retain zero, BF16 tie,
epsilon-dominant, mixed, and cast-order-sensitive cases. Do not evaluate the
whole expression in FP64 and apply a single final BF16 cast.

The device and generated host interfaces are pinned to fe2o3 commit
`3dfa5b3fdac1832bd7d8902e32f591d81300d1e3`. Host contract tests do not execute the
kernel. Compilation alone implies neither GPU correctness nor model speedup.

## Current Verification

Managed host check and strict Clippy pass. Three host tests cover exact ownership
coverage, the four-slice ABI, and BF16 ties-to-even/cast-order behavior. Actual
source extraction through semantic MIR, ranked PLIRON, Kernel IR and gfx950 LLVM
also passes with four checked stores. The emitted LLVM retains two separate
squares, a pairwise add, six XOR reduction stages (1/2/4/8/16/32), `llvm.sqrt.f32`,
and an unrelaxed `fdiv`. It disables fast math and contraction and preserves IEEE
FP32 denormals.

The completion-ordered native chain has now passed five cases on `mi350-2`,
including all 5120 final BF16 words exactly matching the staged reference.
Both stages use two wave64 waves per workgroup. The norm stage uses four
workgroups, 28 SGPRs/19 VGPRs, and no spills, LDS, or private memory. See the
[bound native evidence](../../qualification/gfx950-qwen3-knorm-v1/README.md#native-gpu-evidence)
for source/object hashes, projection error bounds, lifecycle checks, and limits.
