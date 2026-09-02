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
it neither panics nor invents a zero identity, and the call still rejects. The
projection has no public constructor, serializer, JSON preflight, environment, file, or CLI input.
The projection is neither protected evidence nor a verifier decision and
cannot leave the rejection path.

Every verifier call then returns the unconditional `Err(MissingProtectedVerificationReceipt)`.
The adapter does not construct fe2o3 verification evidence or enable fe2o3's
synthetic test support. It does not accept hashes as a substitute for protected
proof, finalizer, compiler-execution, source/target custody, layout, effect, or
executable verification.

This scaffold grants no verification, load, launch, or inference authority. It
has no direct KFD, HSA, HIP, engine, or model import and invokes none of those
surfaces. Its `fe2o3-host` dependency has a broader resolved runtime closure;
that transitive closure does not grant this adapter runtime authority. A future
implementation must replace the unconditional error only when a reviewed
protected backend can satisfy every obligation of fe2o3's unsafe aggregate
Worker V3 verifier trait.
