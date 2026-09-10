# Qwen3 TP Batch Kernels V2

Additive, Contracted engineering compilation unit for Qwen3-8B BF16 on
TP1/TP2/TP8. The old thirteen-root single-sequence unit and protected
twelve-root unit remain unchanged. Exactly one of `gfx942` and `gfx950` is
required; the default remains `gfx942`. Device builds reject mismatched target
features. All fe2o3 dependencies are pinned to public `3546d54d`.

## Batch And Page Contract

- One dispatch processes 1 through 16 independent token rows. The host may
  maintain up to 32 requests and put multiple contiguous prompt tokens from
  a request into one batch. Request admission and scheduling are host-owned.
- Hidden width is 4096, intermediate width is 12288, vocabulary is 151936,
  and head dimension is 128. Local query heads are 32/16/4 and local KV heads
  are 8/4/1 for TP1/2/8; every group of four local query heads maps to one
  local KV head.
- KV storage is `[physical_page, 16, local_kv_heads, 128]`. Physical page count
  and logical page-table stride are independently configurable from 1 to 512.
  Each table row is indexed at `row * max_pages_per_sequence + token / 16`.
  Physical IDs are zero-based. Entries outside a query's initialized causal
  prefix may be `u32::MAX`; attention must not read them.
- Per-row absolute positions are less than 8192. Attention additionally takes
  `max_context_tokens`, a positive uniform loop bound no larger than the
  logical table capacity. The host supplies the maximum selected position plus
  one. Each row independently masks `token > positions[row]` before reading
  any page-table, key, or value element.
- Append validates every selected slot and rejects duplicate physical token
  slots before writing. A single grid leader then copies only selected rows,
  not the full pool. This intentionally uses the established GridExclusive
  ownership contract instead of claiming arbitrary scatter is disjoint.
- The host must establish page ownership, immutable sealed-prefix reuse,
  copy-on-write, initialized prefixes, and completion-before-reuse. A page ID
  and the append distinctness check do not establish these temporal properties.

New kernels accept active-prefix buffers through their bounded 16-row storage
extent and preserve inactive output tails. Weights and physical KV buffers
must have their exact full shapes. The imported RMSNorm kernel instead
requires exact active slice lengths; the host supplies shortened views into
its retained maximum-size allocations.

## Exact ABI

Every slice is an eight-byte pointer followed by a `u64` element count. BF16
elements are `u16`; page tables, positions, tokens and choices are `u32`;
partial outputs and trigonometric tables are `f32`. Scalars are `u32`.
The following explicit-byte counts exclude compiler padding and the hidden
COV6 tail. Hosts must use inspected argument offsets and the full inspected
kernarg segment size, not assume concatenation accounts for hidden arguments.

All launch workgroups are `[64,1,1]`; the table gives one-dimensional workgroup
counts, not thread counts. `I = 12288 / world`, `Q = 32 / world`.

| Root Suffix After `ferric_qwen3_tp_batch_` | Ordered Arguments | Explicit Bytes | Workgroups |
| --- | --- | ---: | ---: |
| `embedding_bf16_v2` | tokens, weight, output, rows | 52 | rows * 64 |
| `gemm_bf16_f32_bf16_v2` | a, weights, output, rows, n, k, world_size, projection | 68 | n / 16 |
| `gemm_partial_bf16_f32_v2` | a, weights, output, rows, n, k, world_size, projection | 68 | n / 16 |
| `swiglu_bf16_f32_v2` | gate, up, output, rows, world_size | 56 | rows * I / 64 |
| `rope_v2` | query, key, cos, sin, positions, rotated_query, rotated_key, rows, world_size | 120 | rows |
| `paged_kv_append_v2` | key, value, positions, page_table, key_cache, value_cache, rows, world_size, max_pages_per_sequence, physical_pages | 112 | 1 |
| `paged_gqa_bf16_f32_v2` | query, key_cache, value_cache, positions, page_table, output, rows, world_size, max_pages_per_sequence, physical_pages, max_context_tokens | 116 | rows * Q |
| `argmax_bf16_v2` | logits, choices, rows | 36 | rows |

The ninth root is unchanged `qwen3_rmsnorm_v1`, imported from the aggregate.
`compiler_expectation_roster_v2()` returns all nine current compiler-generated
markers, sorted by kernel binding ID. The host must check every logical,
AMDHSA entry, descriptor and export identity against that exact roster.

Column projection tags are Q=1, K=2, V=3, Gate=4, Up=5, LM=6. LM projection and
argmax execute only on rank zero under the host schedule. Partial tags are O=1
and Down=2. Weights are row-major `[n,k]`; activations are row-major `[rows,k]`.
Both GEMM roots use a real 16-by-16 output tile: four token-row accumulators per
lane share each weight load. Each accumulator preserves ascending FP32
multiply/add order. There is no serialized loop of one-token GPU dispatches.
O/down store FP32 partials without residual or BF16 narrowing; the host sums
rank order, adds each row's residual once and narrows once. This order differs
from unpartitioned FP32 accumulation and is not claimed bitwise equivalent for
arbitrary inputs. SwiGLU, RoPE, online softmax and argmax retain the previous
engineering path's numerical ordering and tie rule.

## Evidence Boundary

The loop-bearing device bodies are explicit because the compiler's V1
control-flow sidecar does not admit opaque macros. Host tests execute matching
numerical macros; source-linked token comparisons check their correspondence
to those explicit bodies. Source-linked tests also check ABI, launch roster,
and causal/write-validation structure; positive and negative fixtures check
row isolation, noncontiguous pages, page boundaries, stale NaN/future data,
duplicate slots, finite arithmetic, and partial-output precision.
These checks are not Verus numerical proofs, source-to-machine refinement,
protected admission, transport completion, serving qualification, or a speedup
measurement. Actual emission and hardware results must be recorded separately
against the final source and exact compiler/runtime identities.

The source closure includes this entire crate plus the unchanged imports
`qwen3-all-kernels-v1/src/{rmsnorm,target}.rs` and
`qwen3-all-kernels-v1/build/target_contract.rs`. The old protected build policy
does not admit this unit. Builds, formatting and host tests run only on the
owned `mi300x` stage; hardware execution is coordinated separately on `mi350`.
