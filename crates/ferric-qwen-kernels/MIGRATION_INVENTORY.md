# Qwen kernel ownership migration inventory

This is a developer inventory, not source authentication, review evidence, or
an M1 status record. The historical fe2o3 branches remain unchanged. Ferric
must re-own and re-qualify every migrated source and identity.

## GEMM/GEMV

Historical reference commit:
`dba12e9cc3bbf7810f05dd4e82cc2e2d355b63f8`, tree
`a6b983137deb931c1ef9da11a0826d62aeaae8f7`.

Move into `src/gemm.rs`:

- the two-role, eleven-bucket, eight-operation finite catalog;
- BF16 A/B/C buffer contracts with FP32 accumulation and BF16
  round-to-nearest-even conversion, exact alpha/beta choices, and launch
  geometry;
- the general-GEMM Pliron/compiler lane and its reference/vectorized schedules;
- linear prepared, request, Worker-evidence, inspected, and checked-launch
  custody types;
- strict post-worker ABI, resource, compilation-binding, descriptor, and
  loader inspection;
- unit tests for the finite catalog, buffer bounds/aliasing, schedule choice,
  source-label rejection, and structural compiler output.

The migrated catalog must use Ferric graph-width correction commit `15bafaf`
as its semantic reference. Attention output-projection input width is flattened
`query_heads * head_dim`: target 4096 and draft 2048. In particular, the draft
O-projection weight/input contract is `[hidden=1024, query_width=2048]`, not a
square 1024-by-1024 hidden projection. Add a hostile hidden-width substitution
test when moving the module; the historical fe2o3 catalog predates this Ferric
graph correction and must not be copied blindly.

Move and re-domain the six existing compile-fail source/baseline pairs for
prepared, request, and inspected custody. Rename the generic filenames with a
`gemm_` prefix. Add real trybuild-generated non-Clone/private-field baselines
for Worker evidence; the historical branch did not contain those two cases.

Do not move fe2o3 workspace membership, lockfile, CI-local, or dependency-policy
edits. Replace every historical fe2o3-owned Qwen GEMM identity domain with a
Ferric domain and use only the shared crate's pinned public generic
compiler/runtime dependencies.

## RMSNorm

Historical reference commit:
`e1748f45d99a5c2da688eff4a0392827385ade84`, tree
`62d11e929a2eb9e6cf71007c8a674fb7b9d493c8`, directly based on the GEMM
reference commit.

Migrated into `src/rmsnorm.rs` from the historical reference:

- the 132-profile catalog: two roles, eleven buckets, five pure graph
  operations, and a separate hidden-width residual-fused operation;
- hidden widths 4096/1024 and per-head query/key width 128 with exact target and
  draft head counts and row multiplication;
- the BF16 buffer contract and declared FP32 sum/normalization contract;
- pure-mode zero optional residual/output spans and fused-mode exact nonzero
  disjoint spans, including fused-width-128 rejection;
- the direct typed Handoff V2 graph, explicit Pliron scalar-BF16 blocker, exact
  ABI/geometry, Worker custody, post-worker inspection, and checked binding;
- unit tests for all profiles, modes, maximum rows, source labels, bounds,
  aliases, hostile geometry, and structural compiler output.

The six historical RMSNorm compile-fail source/baseline pairs are re-domained
for Ferric. Non-Clone/private-field Worker-evidence cases are included and
their baselines must come only from a real trybuild run.

Do not copy historical lockfile or fe2o3 package/README/module ownership edits.
Replace every historical fe2o3-owned Qwen RMSNorm identity domain with a Ferric
domain. Preserve the explicit nonclaims: declarations and structural checks
are not numerical, operator, LLVM-to-machine, artifact, hardware, or
performance evidence, and Worker V2 was not executed by the source-only
qualification.

## Integration

Both modules belong in `ferric-qwen-kernels` beside `rope_kv.rs`. Their host
profile identities must later join the Ferric generated plan and protected
runner. `ferric-gemm`, `ferric-rmsnorm`, and `kernel-schedule-catalog` remain
Open path obligations until fresh Ferric-owned qualification closes the exact
declared boundaries.
