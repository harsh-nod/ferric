# Aggregate Worker V3 Promotion Prerequisite V1

This adapter collects one move-only, source-bound promotion prerequisite for
the aggregate seven-family, 12-program Worker V3 artifact. Collection requires
all of the following as explicit caller inputs:

- one retained durable-root descriptor, exact V2 build attempt, and current
  publication lease;
- one canonical protected-verifier receipt and independently provisioned
  verifier key/measurement policy;
- the exact caller-retained canonical verifier service request, including the
  full protected intent and ordered 12-entry roster;
- one independently provisioned compiler issuer policy;
- one nonzero caller-provided compiler-current challenge; and
- the separately transported canonical V3 current verification and signed
  attestation.

The collector strictly correlates the envelope source pin, exact current HSACO
bytes, complete compiler receipt carriage, signed protected-verifier compiler
claims, the complete caller-retained verifier request, and every V3
current-record identity. Missing, noncanonical, mismatched, or locally
non-current material fails closed. The protected request's result-only proof,
layout, and effect digests are accepted under the authenticated protected
verifier TCB after the complete request matches the signed receipt; this
collector does not independently rederive those results.

A direct Verus refinement makes the final executable acceptance gate the
conjunction of every protected-request, source, artifact, compiler, current,
token, and exact ordered 12-entry observation. It also proves that the
accepted outcome grants no publication, verifier, load, or launch authority.
The model begins after the operating-system, filesystem, decoding, hashing,
and cryptographic operations; those bodies remain explicitly unverified. The
sole `vstd` dependency is proof-only and has no callable production path after
proof erasure.

Recovery uses fe2o3's retained-directory V2 API. The locked durable-root
descriptor is preserved in the recovered publication lease, so promotion
collection never re-resolves an ambient output path. After every correlation,
the collector acquires and validates the exact lease's currentness token and
revalidates the retained files while keeping the cooperative publication lock
held in the returned owner.

The supported transition for an external promotion service consumes the live
owner, immediately revalidates the retained locked `CURRENT` state, and returns
an opaque sealed handoff. It does not reveal the recovered publication,
currentness token, or authenticated owners and still grants no authority. A
change between collection and this transition returns a typed durable-link
error.

The returned live object is not cloneable or serializable. Its descriptive
evidence exposes typed identity getters and no serialization API; it explicitly
reports `is_current() == false`. The recovered publication, currentness token,
and authenticated evidence owners remain private. No code in this boundary
writes `CURRENT` or grants verifier, publication, GPU load, or GPU launch
authority.

## External Blocker

Production promotion still requires a separately deployed service that owns
authenticated protected-verifier key custody, protected compiler-current
service policy and Worker-ledger validation, durable anti-rollback/currentness
policy, and authorization to publish `CURRENT`. The challenge checked here is
caller-provided: atomic one-time challenge consumption and atomic current-ledger
consumption are responsibilities of that external promotion service. This
collector alone does not prevent replay of otherwise valid signed material.
Those deployment materials are not present in this repository and are not
fabricated by this prerequisite.

## Behavioral Qualification Boundary

Collector qualification constructs a synthetic, test-only 12-entry HSACO with
the exact Ferric entry/descriptor roster through fe2o3's adversarial Worker V3
fixture builder. It exercises live retained-directory recovery, current-token
locking, full request/compiler/entry correlation, mutation rejection, and a
real second publication that makes the first attempt stale. The synthetic
artifact is not compiler-produced evidence and cannot close any artifact,
promotion, load, launch, or hardware gate.

The preserved cf6 V77 compiler-produced HSACO is evaluated separately. Pairing
it with the fixture builder's generic launch contract is expected to fail
strict finalizer admission; that rejection is negative compatibility evidence,
not a replacement production publication.
