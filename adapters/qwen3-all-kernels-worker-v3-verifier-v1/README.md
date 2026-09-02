# Ferric Qwen3 Aggregate Worker V3 Verifier V1

This standalone Ferric package owns the protected-verifier boundary for the
single 12-marker `M1AllKernelsWorkerV3RosterV1`. It has two deliberately
different backends:

- `M1AllKernelsProtectedVerifierV1` is the zero-state default. It performs the
  local artifact preflight and always returns
  `MissingProtectedVerificationReceipt`.
- `M1AllKernelsProductionProtectedVerifierV1` is a move-only binder for a
  separately supervised deployment. Its unsafe constructor requires a
  previously admitted one-shot V2 service client, a move-only unpredictably
  generated Begin challenge already reserved in durable deployment replay
  state, a caller-provisioned trust policy, and an inherited FD195
  compiler-current auditor. The caller must also
  uphold the external service, evidence-custody, freshness, and durable
  rollback-currentness obligations that those inert inputs cannot prove, and
  guarantee correct theorem and safety results for every concrete invocation
  covered by each marker contract.

Neither constructor discovers an endpoint, reads an environment setting,
loads a key, opens `CURRENT`, or manufactures a receipt. The configured binder
is unusable until deployment code supplies all four real inputs.

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

After its unsafe deployment contract is upheld, the configured backend first
performs the shared local preflight and the exact aggregate source-policy
projection. It constructs and retains the receipt source-pin owner before it
takes the client or Begin challenge, sends Begin, reserves service state, or
consumes the FD195 current-record auditor. An invalid source therefore leaves
all four one-shot resources untouched. It constructs the generic V2 Begin request
from the reserved caller challenge, exact roster and policy identities, the
complete verifier-runtime-closure measurement, and all 12 ordered coordinates.
It creates separate sealed, unlinked memfd snapshots of the exact originally
admitted Worker V3 V2 envelope and current finalized HSACO, and transfers those
two descriptors in canonical order.

Only after the service accepts Begin and returns a correlated, globally unique
service challenge does the binder consume the inherited FD195 auditor. It uses
that exact challenge, copies the two complete canonical current-record arrays
from the resulting lifetime-bound audit view, and submits them on the still-open
V2 session. The service must independently authenticate those bytes; the host's
audit is not verifier authority.

The six source coordinates are not copied without policy checking. Ferric's
public typed source-pin projector repeats the exact LLVM-text, target,
code-object, 12-entry-symbol, and 12-descriptor-symbol policy over the request's
decoded compiler handoff before any external effect. Both the pre-bind and
post-bind service requests borrow the same retained source-pin owner and
reassociate it with the still-borrowed handoff. The service request combines that projection with
the typed request, bound compiler owner, caller policy identity, exact artifact,
and all 12 ordered host lineage/marker/generated-host rows.

The owned client requires an already connected unnamed Unix `SOCK_SEQPACKET`
peer and pins dedicated non-root credentials before transferring the descriptor
to fe2o3's authority-free V2 transport. Ferric computes one absolute monotonic
deadline during peer admission and passes that exact `Instant` to
`WorkerV3VerificationClientV2::admit_until`; it never restarts the caller's
relative timeout. That same deadline governs every generic phase and Ferric's
final application-response authentication. The transport rejects ancillary data and ambiguous framing, and correlates
every phase to the exact Begin request, service challenge, and reservation. The
Ferric terminal then requires an application response of exactly 3,768 bytes
and authenticates its Ed25519 signature and distinct verifier/checker
measurements under the caller policy.

The V1 Ferric receipt is retained intentionally. Its signature directly binds
the exact current-record verification and challenge-bound attestation
identities. Because the service challenge is globally unique and durably
burned, a receipt from another session cannot match the current audit. The
generic terminal separately binds the current request, challenge, and opaque
reservation. The V1 signature does **not** directly cover the generic Begin
request identity or reservation identity, and this package does not claim that
it does. A future requirement for direct signature coverage of those transport
coordinates requires a new receipt schema.

Before consuming the current audit into compiler execution, the binder requires
the correlated terminal and authenticates the complete receipt. It then performs
fe2o3's exact subject/carriage binding transition and requires the resulting
service request to byte-match the pre-bind request. Before promotion, the binder
repeats policy, measurement, full receipt/request,
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

The legacy V1 application packet remains a coordinate protocol, but generic V2
now transports immutable envelope and HSACO snapshots plus the complete
current-record arrays. The protected service must independently decode and
verify those payloads and must already possess, or authentically reacquire over
a separately reviewed bounded channel, the remaining semantic/proof inputs. It
must atomically consume each challenge against protected live
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
3. An unpredictable nonzero Begin challenge durably reserved and globally
   replay-excluded before constructing its move-only Ferric token.
4. A caller-provisioned non-weak Ed25519 public key and distinct exact verifier
   and checker measurements.
5. A supervisor-installed fe2o3 compiler-current service at inherited FD195,
   including its independently provisioned issuer and external-anchor policies.
6. A real recovered aggregate publication whose typed request carries the exact
   source, artifact, and proof payloads.

This package embeds none of those deployment values. It does not provide a
service process, signing key, real receipt, model bundle, `CURRENT` record,
qualification result, or GPU result. It has no direct KFD, HSA, HIP, engine, or
model-loading API and grants no publication, load, launch, or inference
authority by itself. Test-only signing fixtures exercise framing and
authentication; they are not compiled into production code and are not
deployment evidence.
