# Ferric Qwen kernels

This crate owns the model-specific compiler catalogs, typed kernel graphs, host
bindings, and linear Worker custody wrappers for Ferric's admitted Qwen
envelope. It consumes public, reusable fe2o3 compiler/runtime APIs; fe2o3 does
not own the Qwen geometry or operator declarations in this crate.

The integrated structural compiler lanes are:

- `gemm.rs`: 176 profiles for eight dense operations over both roles and all
  eleven buckets. A, B, and C use BF16 storage with declared ascending FP32
  accumulation and BF16 round-to-nearest-even output. Draft attention output
  projection consumes the exact flattened query width 2048, not hidden width
  1024.
- `rope_kv.rs`: 44 profiles for separate `RoPE` and paged-KV-write operations.
  Target query tensors are `[S,T,32,128]`, draft query tensors are
  `[S,T,16,128]`, and both key/value tensors are `[S,T,8,128]`.
- `prefill.rs`: eight initial causal-prefill attention profiles. Query/output
  width is 4096 for target and 2048 for draft.
- `paged_decode.rs`: fourteen decode/speculative attention profiles over an
  exact per-sequence committed-token vector. Query/output width is 4096 for
  target and 2048 for draft.
- `swiglu.rs`: 22 target/draft profiles over the eleven admitted buckets.
  Gate, up, and output tensors use BF16 storage with role-exact intermediate
  widths, fixed-order FP32 activation arithmetic, and BF16 round-to-nearest-even
  output.
- `logits.rs`: 22 lowest-ID argmax profiles over BF16 logits with vocabulary
  151,936, plus target-only compact completion profiles. Direct modes publish
  the final active row; speculative modes publish the maximal accepted draft
  prefix plus a correction or bonus in the canonical 120-byte record.

`RoPE` uses split-half D128 pairing and reads absolute U32 position IDs into fixed
FP32 cosine and sine tables of extent `[8192,64]`. The typed graph does not
compute trigonometric functions. The deployment table's values, theta relation,
provenance, authentication, and join to Ferric's generated `PositionIds` remain
open obligations.

Paged KV write uses one 512-entry U32 page table per active sequence, P16
translation, eight KV heads, and one fixed global physical cache pool with
layout `[16384,16,8,128]`. Layer selection remains outside the kernel. Machine
control flow checks logical position, selected context, the per-sequence
page-table index, the global physical page, and exact byte spans before address
use. A sequence selects a translation table; it does not prefix cache storage.
Generation, exclusive-owner, role, profile, and page-table identities remain
host labels and are not authenticated by the machine ABI.

The modules construct pinned direct-LLVM or typed Handoff V2 payloads and bind
them only to compiler-produced, move-only Worker V3 evidence before strict
post-worker structural inspection. The nested V2 label describes the module
codec, not an executable V2 authority route. The attention lanes retain an
exact unresolved OCML exponential provider contract. These are explicit
compiler boundaries, not evidence that the Worker ran or that an executable
artifact exists until the matching V3 owner is supplied.

Every accepted V3 owner must retain the exact canonical link options
`code-object-version=6`, `opt-level=2`, `strip-debug=true`, and
`verify-each=true`, a 64 MiB bootstrap output ceiling, and an exact replay
ceiling equal to the retained artifact length. Artifact-set publication also
requires the exact reviewed Worker measurement and default execution limits
for all seven lanes. The retired Worker V2 artifact CLI has no V3 replacement:
an authenticated in-process collector that acquires all seven owners and
invokes publication is still unavailable.

The catalogs and structural checks are not evidence of operator or numerical
refinement, physical-KV refinement, source or artifact authentication,
machine-code refinement, allocation, load, launch, completion, hardware
behavior, or performance. Checked-in tests do not execute Worker V3 and do not
establish HSACO existence. The `ferric-gemm`, `ferric-prefill`,
`ferric-paged-decode`, `ferric-rope-kv`, `ferric-swiglu`, `ferric-logits`, and
`kernel-schedule-catalog`
obligations, plus the exact Ferric generated-plan/runner identity join, remain
Open. This crate does not close an assurance property or roadmap row.

The `src/rmsnorm.rs` module declares 132 profiles: target and draft roles,
eleven exact Ferric buckets, five pure graph operations, and one separate
hidden-width residual-fused operation. Hidden normalization widths are 4096
for the target and 1024 for the draft. Query and key normalization are
per-head width 128, with exact target/draft query-head and shared KV-head row
counts. The generic machine ABI carries behavior, rows, width, epsilon, and
element counts. It requires positive rows and one supported mode/width pair,
but it does not constrain rows to the finite catalog or carry an operation or
profile tag. Consequently it cannot distinguish Input, `PostAttention`, and
Final `RMSNorm` when their pure hidden geometry is identical. Exact operation,
role, bucket, and row selection remain checked host catalog state and are not
yet joined to a generated Ferric plan or runner.

`RMSNorm` inputs, weights, residuals, fused residual outputs, and normalized
outputs are BF16. The typed graph declares FP32 square accumulation,
normalization, weighting, and residual addition before BF16 conversion. Pure
mode requires the two optional residual lengths to be exactly zero while their
pointer arguments remain nonzero aligned sentinels. The pointers carry
`nonnull` but no dereferenceable attributes, and pure-mode control flow cannot
reach residual loads or fused-output stores.
Residual-fused mode is hidden-width only, requires exact nonzero disjoint spans,
and rejects width 128.

The `RMSNorm` module uses a direct typed Handoff V2 payload because the reusable Pliron
AMDGPU route cannot represent this scalar BF16 graph. It serializes canonical
LLVM, requires matching linear Worker V3 custody, and defines strict post-worker ELF,
AMDHSA ABI, descriptor, resource, and loader inspection. Source tests inspect
the serialized LLVM structure but do not execute Worker V3, create or
disassemble an HSACO, or establish numerical, operator, source-to-LLVM,
LLVM-to-machine, hardware, completion, or performance refinement. The
`ferric-rmsnorm` and `kernel-schedule-catalog` paths and `m1.r07` remain Open.

The workspace dependency revision is pinned to accepted reusable fe2o3 generic
compiler/runtime commit `52815c9ed52a3075e26322cf506144cb22da12d2`.
The historical M1 upstream base remains
`a6c779f6f8052839c3a07901f9bfafa681f7b09a`; neither source closure is Ferric
kernel authority, kernel qualification evidence, or evidence for an M1 row.
