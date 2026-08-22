# Ferric Qwen kernels

This crate owns the model-specific compiler catalogs, typed kernel graphs, host
bindings, and linear Worker custody wrappers for Ferric's admitted Qwen
envelope. It consumes public, reusable fe2o3 compiler/runtime APIs; fe2o3 does
not own the Qwen geometry or operator declarations in this crate.

The first module, `src/rope_kv.rs`, declares 44 profiles: target and draft roles,
eleven exact Ferric buckets, and separate `RoPE` and paged-KV-write operations.
Target query tensors are `[S,T,32,128]`, draft query tensors are
`[S,T,16,128]`, and both key/value tensors are `[S,T,8,128]`.

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

The module constructs separate typed Handoff V2 functions for `RoPE` and KV
write, serializes canonical LLVM, forms a linear Worker V2 request, and defines
strict post-worker structural inspection. It uses typed Handoff V2 directly
because the reusable Pliron AMDGPU route does not support this scalar BF16
graph. This is an explicit compiler-boundary limitation, not a Pliron result.

The catalog and structural checks are not evidence of operator or numerical
refinement, physical-KV refinement, source or artifact authentication,
machine-code refinement, allocation, load, launch, completion, hardware
behavior, or performance. Checked-in tests do not execute Worker V2 and do not
establish HSACO existence. The `kernel-schedule-catalog` obligation and exact
Ferric generated-plan/runner identity join remain open. This crate does not
close `m1.r08`, an assurance property, or a roadmap row.

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
mode requires both optional residual pointer/length pairs to be exactly zero,
the pointer parameters carry neither `nonnull` nor dereferenceable attributes,
and its control flow cannot reach residual loads or fused-output stores.
Residual-fused mode is hidden-width only, requires exact nonzero disjoint spans,
and rejects width 128.

The `RMSNorm` module uses direct typed Handoff V2 because the reusable Pliron
AMDGPU route cannot represent this scalar BF16 graph. It serializes canonical
LLVM, forms linear Worker V2 custody, and defines strict post-worker ELF,
AMDHSA ABI, descriptor, resource, and loader inspection. Source tests inspect
the serialized LLVM structure but do not execute Worker V2, create or
disassemble an HSACO, or establish numerical, operator, source-to-LLVM,
LLVM-to-machine, hardware, completion, or performance refinement. The
`ferric-rmsnorm` and `kernel-schedule-catalog` paths and `m1.r07` remain Open.

The workspace dependency revision is pinned to accepted reusable fe2o3 generic
compiler/runtime commit `a6c779f6f8052839c3a07901f9bfafa681f7b09a`.
That source closure supplies generic infrastructure only; it is not Ferric
kernel authority, kernel qualification evidence, or evidence for an M1 row.
