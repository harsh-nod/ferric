# Ferric Qwen3 Aggregate Worker V3 Verifier V1

This standalone Ferric package marks the protected-verifier boundary for the
single 12-marker `M1AllKernelsWorkerV3RosterV1`. The current production backend
is intentionally fail-closed because Ferric does not yet have an independently
authenticated protected-verification receipt covering every aggregate roster
entry and its exact executable.

Before rejecting, the backend constructs one private reject-only projection
directly from the borrowed, typed `WorkerV3RosterVerificationRequestV1`. It
copies the challenge, roster, host-lineage, finalizer, complete compiler
carriage/currentness, capsule, formal-memory, proof-binding, finalized-HSACO,
target, and code-object identities. The carried compiler-policy digest is only
another request identity; the adapter has no protected policy key or trust root
with which to authenticate it.

The projection contains exactly 12 ordered entry rows in the roster's canonical
descriptor-table order:

1. `qwen3_swiglu_bf16_f32_v1`
2. `qwen3_gqa_prefill_causal_bf16_f32_v1`
3. `ferric_qwen3_lowest_id_argmax_bf16_v1`
4. `qwen3_paged_kv_write_v1`
5. `qwen3_paged_gqa_decode_bf16_f32_v1`
6. `ferric_qwen3_speculative_token_assembly_v1`
7. `ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1`
8. `ferric_qwen3_gemm_reference_bf16_f32_bf16_v1`
9. `ferric_qwen3_token_embedding_bf16_copy_v1`
10. `ferric_qwen3_compact_completion_v1`
11. `qwen3_rope_v1`
12. `qwen3_rmsnorm_v1`

Every row retains its ordinal, logical and export names, marker-binding
identity, generated-host-contract identity, and host-lineage identity. It also
copies typed descriptor, ELF-binding, and physical-kernel facts from that same
request ordinal. Descriptor facts cover the kernel ID, names, source and IR
evidence identities, and physical ABI counts. Binding facts cover the metadata
index, descriptive descriptor/entry addresses and file offsets, entry size, and
raw descriptor resource fields, including the kernarg-preload field. Physical
facts cover the name, symbol, kernarg ABI, segment sizes, registers, spills, workgroup limits, execution-mode
declarations, and explicit/hidden argument counts. The descriptive addresses
are code-object coordinates, not runtime pointers or load authority.

Each descriptor, binding, physical-kernel, and lineage subprojection remains
`Option`. If any typed accessor lacks a row fact, the adapter retains `None`;
it neither panics nor invents a zero identity. `None` is faithfully projected,
but the association preflight rejects it. The projection has no public
constructor, serializer, or JSON input.
It has no environment, file, or CLI input. The projection is neither protected
evidence nor a verifier decision and cannot leave the rejection path.

After constructing that projection, the backend performs an authority-free
common-custody preflight in exact order. It independently revalidates the
finalizer derivation from the exact borrowed replay, validates the common
multi-root compiler proof inputs, and then validates the common multi-root
target lineage by borrowing those proof inputs. Each failure maps to a distinct
fail-closed error. The three inferred move-only owners are retained together
through the private rejection helper; they are not exposed or serialized.

A lexically scoped, zero-argument association closure then borrows all three
owners and the typed request projection. It checks the finalizer identity and
finalized-HSACO digest and length, cross-binds the final LLVM module to both the
finalizer compiler module and semantic handoff module, and requires the literal
`gfx942:xnack-` / COV6 target. It requires exactly 12 markers, proof roots, and
target workgroups; establishes marker/proof binding bijection in both
directions; and matches each entry to its proof root by binding identity rather
than ordinal. Every matched row must contain lineage, descriptor, ELF-binding,
and physical-kernel facts with consistent logical name, export name, kernel
binding, physical export, and descriptor symbol. The target workgroup is read
at the matched proof-root index and must name that proof root's Kernel IR kernel
and exact workgroup. Descriptor launch constraints are matched exhaustively:
only `BlockSizeV1::Exact` equal to the proof-root workgroup is accepted;
`Any`, `AtMost`, or a missing descriptor is rejected. The closure ends before
all three owners are moved unchanged into the private rejection helper.

Passing the preflight still returns the unconditional
`Err(MissingProtectedVerificationReceipt)`. The common owners do not establish
the per-entry proof-to-executable, Rust layout, or Rust effect joins, nor do
they authenticate compiler policy, Worker-ledger currentness, or rollback
currentness. The adapter performs none of those checks and does not construct
fe2o3 verification evidence or enable fe2o3's synthetic test support. It does
not accept hashes as a substitute for protected proof, finalizer,
compiler-execution, source/target custody, layout, effect, or executable
verification.

This scaffold grants no verification, load, launch, or inference authority. It
has no direct KFD, HSA, HIP, engine, or model import and invokes none of those
surfaces. Its `fe2o3-host` dependency has a broader resolved runtime closure;
that transitive closure does not grant this adapter runtime authority. A future
implementation must replace the unconditional error only when a reviewed
protected backend can satisfy every obligation of fe2o3's unsafe aggregate
Worker V3 verifier trait.
