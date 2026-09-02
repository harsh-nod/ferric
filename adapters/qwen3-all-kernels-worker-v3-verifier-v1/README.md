# Ferric Qwen3 Aggregate Worker V3 Verifier V1

This standalone Ferric package owns the protected-verifier boundary for the
single 12-marker `M1AllKernelsWorkerV3RosterV1`. It has two deliberately
different backends:

- `M1AllKernelsProtectedVerifierV1` is the zero-state default. It performs the
  local artifact preflight and always returns
  `MissingProtectedVerificationReceipt`.
- `M1AllKernelsProductionProtectedVerifierV1` is a move-only binder for a
  separately supervised deployment. Its constructor requires a previously
  admitted one-shot service client, a caller-provisioned trust policy, and an
  inherited FD195 compiler-current auditor.

Neither constructor discovers an endpoint, reads an environment setting,
loads a key, opens `CURRENT`, or manufactures a receipt. The configured binder
is unusable until deployment code supplies all three real inputs.

## Local Custody

Both backends use the same private local revalidation path. It projects one
typed `WorkerV3RosterVerificationRequestV1`, reinspects the exact borrowed
finalized HSACO, independently reconstructs the finalizer derivation, validates
the common multi-root proof inputs, and validates target lineage while retaining
all move-only owners.

The shared association check binds the finalizer identity and exact HSACO
digest/length, the final LLVM module to both finalizer and semantic-handoff
module identities, and the literal `gfx942:xnack-` / COV6 target. It requires
exactly 12 markers, proof roots, target workgroups, metadata kernels, and
descriptor bindings. Marker and proof bindings must form a bijection.

Metadata order is not assumed to equal roster order. Each roster entry finds
one unique reinspected kernel by export name and descriptor symbol, with a
checked 12-element coverage map. Its typed physical kernel and descriptor
binding must equal the reinspection, and its exact launch block must equal the
matched proof-root workgroup. Missing descriptors, lineages, bindings, physical
kernels, `Any`/`AtMost` launches, duplicates, and incomplete coverage fail
closed.

The private projection retains typed descriptor, ELF-binding, physical-kernel,
marker, generated-host, and lineage facts. Descriptive addresses are object
coordinates, not runtime pointers. It has no public constructor or serialization
surface.

## Configured Binder

The configured backend first performs the shared local preflight. It then uses
the inherited FD195 auditor once and consumes the signed current-record audit
through fe2o3's exact subject/carriage binding transition. The resulting
move-only compiler-execution owner supplies every signed current-record,
protected-policy, Worker-ledger, and external rollback coordinate in the
service request. All request-known compiler coordinates are compared back to
the borrowed roster request.

The six source coordinates are not copied without policy checking. Ferric's
public typed source-pin projector repeats the exact LLVM-text, target,
code-object, 12-entry-symbol, and 12-descriptor-symbol policy over the request's
decoded compiler handoff. The service request combines that projection with
the typed request, bound compiler owner, caller policy identity, exact artifact,
and all 12 ordered host lineage/marker/generated-host rows.

The owned client sends that request once. It requires an already connected
unnamed Unix `SOCK_SEQPACKET` peer, pins dedicated non-root credentials, applies
one absolute deadline, rejects ancillary data and ambiguous framing, correlates
the response to the complete request, and authenticates its Ed25519 signature
and distinct verifier/checker measurements under the caller policy.

Before promotion, the binder repeats policy, measurement, full receipt/request,
ordinal, typed lineage, marker, and generated-host association. It maps all 12
signed proof-to-executable, Rust type-layout, Rust effect, and complete Worker V3
safety results. Only then does it consume the local finalizer, bound
compiler-current audit, proof-input, and target-lineage owners into
`WorkerV3ProtectedRosterVerificationEvidenceV1`.

## Receipt And Protocol

The protected receipt is one fixed-width, 3,552-byte little-endian frame with a
3,488-byte signed preimage, domain-separated signing message, and
domain-separated receipt identity. Strict decoding enforces canonical headers,
zero reserved bytes, target/COV, nonzero identities and lengths, an advancing
rollback position, distinct measurements, 12 canonical ordinals, unique
lineage/marker identities, nonzero theorem identities, and every required
safety bit.

The service request is exactly 2,304 bytes and the response is exactly 3,768
bytes. Both are fixed-width binary packets with distinct magic, version,
lengths, target block, reserved-zero policy, and domain-separated identities.
The response repeats the request identity, policy identity, compiler sequence,
current rollback anchor, and receipt identity before carrying the complete
signed receipt.

This remains a coordinate protocol, not evidence transport. The protected
service must already possess, or authentically reacquire over a separately
reviewed bounded channel, the exact Worker V3 V2 envelope, finalized HSACO,
semantic/proof inputs, and protected current-record evidence. It must verify
those payloads and atomically consume each challenge against protected live
current-ledger state shared across service instances and durable across
restarts. Signing caller-supplied hash echoes does not satisfy the backend's
unsafe contract.

## Deployment Prerequisites And Nonclaims

A production deployment still must provide all of the following externally:

1. A reviewed protected-verifier and independent-checker service implementation
   with authentic payload custody, measurements, signing-key custody, freshness,
   and durable rollback-currentness enforcement.
2. A supervisor-created dedicated service endpoint and an admitted client whose
   pinned UID/GID identifies that service.
3. A caller-provisioned non-weak Ed25519 public key and distinct exact verifier
   and checker measurements.
4. A supervisor-installed fe2o3 compiler-current service at inherited FD195,
   including its independently provisioned issuer and external-anchor policies.
5. A real recovered aggregate publication whose typed request carries the exact
   source, artifact, and proof payloads.

This package embeds none of those deployment values. It does not provide a
service process, signing key, real receipt, model bundle, `CURRENT` record,
qualification result, or GPU result. It has no direct KFD, HSA, HIP, engine, or
model-loading API and grants no publication, load, launch, or inference
authority by itself. Test-only signing fixtures exercise framing and
authentication; they are not compiled into production code and are not
deployment evidence.
