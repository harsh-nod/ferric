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
another request identity; the production backend has no protected policy key
or trust root with which to authenticate it.

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

After constructing that projection, the backend calls the pinned
`fe2o3-hsaco-finalize` finalized-HSACO verifier exactly once on the exact bytes
borrowed from the typed request. Descriptor-table, binding, parse, or canonical
whole-HSACO digest failure maps to the coarse
`ExactFinalizedHsacoVerificationFailed` error. The returned owned
`FinalizedDescriptorInspection` is same-process descriptive integrity, not an
independent authority; the adapter retains it but neither owns the artifact
bytes nor loads the code object.

The backend then performs an authority-free common-custody preflight in exact
order. It independently revalidates the finalizer derivation from the exact
borrowed replay, validates the common multi-root compiler proof inputs, and
then validates the common multi-root target lineage by borrowing those proof
inputs. Each failure maps to a distinct fail-closed error. The three inferred
move-only common owners and the owned descriptive reinspection are retained
together through the private rejection helper; they are not exposed or
serialized.

A lexically scoped, zero-argument association closure then borrows all three
common owners, the owned descriptive reinspection, and the typed request
projection. It checks the finalizer identity and finalized-HSACO digest and
length, cross-binds the final LLVM module to both the finalizer compiler module
and semantic handoff module, and requires the literal
`gfx942:xnack-` / COV6 target. The reinspection target and COV must equal the
typed request, its complete descriptor table must be fully equal to the typed
request table, it must contain exactly 12 metadata kernels and descriptor
bindings, and it must include a descriptive load layout. The closure requires
exactly 12 markers, proof roots, and target workgroups; establishes
marker/proof binding bijection in both directions; and matches each entry to
its proof root by binding identity rather than ordinal.

The metadata-kernel and marker orders are distinct domains. For every canonical
entry, the closure finds one unique reinspected metadata index using both the
export name and descriptor symbol. It marks that index through a checked
12-element coverage table, rejects duplicates, and requires a complete
permutation. It never uses the canonical ordinal to index either reinspection
array. The reinspected physical kernel and descriptor binding must be fully
equal to the typed request values at the canonical ordinal, and both the
reinspected and typed binding kernel indices must equal the found metadata
index.

Every matched row must contain lineage, descriptor, ELF-binding, and
physical-kernel facts with consistent logical name, export name, kernel
binding, physical export, and descriptor symbol. The target workgroup is read
at the matched proof-root index and must name that proof root's Kernel IR kernel
and exact workgroup. Descriptor launch constraints are matched exhaustively:
only `BlockSizeV1::Exact` equal to the proof-root workgroup is accepted;
`Any`, `AtMost`, or a missing descriptor is rejected. The closure ends before
all four owners are moved unchanged into the private rejection helper.

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

## Protected receipt V1

The `protected_receipt` module defines the authority-free wire boundary needed
by that future implementation. The receipt is one fixed-width, 3,552-byte
little-endian frame with a 3,488-byte signed preimage, a domain-separated
Ed25519 signing message, and a domain-separated whole-receipt identity. The
strict decoder requires the exact magic, version, header size, entry size,
total size, zero reserved fields, literal `gfx942:xnack-` target, COV6, and 12
canonical entry ordinals. It rejects truncation, trailing bytes, all-zero
required identities, zero content lengths, a zero compiler sequence, an
unadvanced rollback anchor, duplicate lineage or marker identities, aliased
verifier/checker measurements, and any entry missing one of the eight required
Worker V3 safety properties.

The common signed claims bind the exact host challenge, aggregate roster and
lineage, independently replayed finalizer identity, semantic capsule,
formal-memory and proof-binding receipts, and finalized HSACO digest and
length. The source pin covers the neutral compiler module, nested compiler
handoff, and symbol manifest by both SHA-256 and byte length. Compiler claims
cover the subject, carriage, issuer policy, issuer journal, occurrence,
receipt, publication, acknowledgment, Worker-ledger record, rollback sequence,
prior and current anchors, signed current-record verification and attestation,
protected policy and Worker-ledger decisions, and external rollback decision.
The receipt also carries distinct protected-verifier and independent-checker
measurements plus the complete verification transcript.

Each of the 12 ordered signed results binds its ordinal, host lineage, marker
binding, generated host contract, proof-to-executable theorem, Rust type-layout
theorem, Rust effect theorem, and complete safety-property set. The roster
identity covers the canonical logical and export names. A future backend join
must compare the three per-entry host coordinates again at every ordinal.

`M1AllKernelsProtectedVerifierTrustPolicyV1` has no default and embeds no key or
measurement. A caller must independently provision an exact non-weak Ed25519
public key and distinct verifier/checker measurements. Its domain-separated
identity also covers the schema version, roster cardinality, target, COV, and
required safety mask. Authentication strictly verifies the signature only
after canonical decoding and exact policy/measurement comparison. The
authenticated receipt still explicitly grants no verifier, load, launch, or
inference authority.

This codec intentionally does not yet expose a typed-request binding
transition. The pinned fe2o3 API creates
`WorkerV3RosterVerificationRequestV1` only from a real recovered aggregate
publication, and this repository does not carry such a fixture. Shipping an
untested comparison map or a forgeable host constructor would weaken the
boundary. Typed binding must therefore land with the real service/backend and
aggregate-publication integration fixture. That later comparison must cover
every host-known common identity, all six source-pin coordinates, all
compiler-execution input and rollback coordinates, target/COV, finalized
artifact, and all 12 ordered entry coordinates before any evidence promotion.

The production backend does not read, embed, or instantiate this policy or
receipt. `M1AllKernelsProtectedVerifierV1::new()` remains zero-state, never
constructs protected evidence, and terminates at
`MissingProtectedVerificationReceipt`. A later reviewed service/backend join
must supply genuine external policy and receipt custody before that terminal
state can change.
