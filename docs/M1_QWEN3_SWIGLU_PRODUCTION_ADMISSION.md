# M1 Qwen3 SwiGLU Production Admission

Status: fail closed pending a receipt-bearing protected compiler occurrence and
reviewed verifier.

## Current Candidate

Ferric commit `57f6cfdf` and fe2o3 compiler commit `21e4c106` produced the
14,192-byte `gfx942:xnack-` HSACO with SHA-256
`57ecb86b40db136237e65a5fae04c955f2c92fe3347c085ec5c806984fc6afa7`.
The protected-build record establishes compilation, finalization, structural
inspection, and inert V1 load-envelope publication. The HIP numerical result
is separate qualification evidence.

This candidate cannot enter production. Its `F3LDENV1` replay and 356-byte
load-readiness receipt are not the 2,058-byte
`CompilerExecutionReceiptCarriageV1` required by fe2o3's `F3LDENV2` envelope.
The two receipts have different roles and are not interchangeable.

## Ferric Boundary

`ferric_engine::prepare_m1_swiglu_protected_verifier_request_v1` binds:

- the exact artifact, attributed device source, compiler/provider commits,
  compiler handoff, finalization, publication intent, nested replay, and claim;
- the exact kernel symbol, descriptor symbol, target, code-object version,
  workgroup, element ownership, kernarg sizes, and three-buffer access ABI;
- all 22 target-8B and draft-0.6B Qwen profile records, including shape,
  launch geometry, and the declared stable FP32-SiLU/BF16-RNE policy; and
- V2 envelope, carriage, issuer-policy, compiler-subject, request,
  signed-receipt, publication, Worker-ACK, ledger-record, sequence, and next
  rollback-anchor identities.

The result is an authority-free pending request. It has no conversion to a
fe2o3 authenticated executable, load capability, or KFD invocation. The
checked-in V1 candidate is rejected explicitly by
`require_current_m1_swiglu_receipt_bearing_envelope_v2`.

The standalone
`adapters/qwen3-swiglu-worker-v3-envelope-v2` package now pins fe2o3
`5362f3cba0fccf1c75c6b34d94240b29f17d7b9b`. Its raw entry point strictly
decodes V2 into an inert request. Its recovered entry point consumes and
retains one move-only `RecoveredWorkerV3LoadEnvelopeV2`, derives every
compiler-receipt identity from that owner's carriage, checks the exact carried
build and artifact bytes, and returns the owner with any validation failure.
There is no public parallel-identity input.

This is still a verifier precursor, not the accepting host adapter. The V2
owner does not expose the later host-admitted descriptor lineage or
authenticate repository commit labels, so those labels remain policy metadata.
The adapter does not consult protected policy, enforce rollback currentness,
authenticate compiler supervision, or grant verifier, load, or launch
authority.

## Upstream Join

The pinned fe2o3 integration head combines the V2 codec, strict restart
recovery, retained carriage, and V2-only host admission with pidfd-owned issuer
lifecycle, the fixed protected listener, a bounded worker pool, and the
policy-bound rustc FD 202/195/196 client session. Its fixed production
supervisor connector is available: the child-session transfer creates the
connection internally and permits neither an alternate path nor a caller-owned
control descriptor. Fresh Cargo completion still fails closed because the
active `cargo-fe2o3` binding wrapper does not construct that child session or
invoke the connector. The rustc/backend path does not yet consume FD 195,
acquire the exact-subject carriage, or return it on FD 196. Deployment also
remains outstanding: it must provision the launcher and issuer images,
service-owned root, sealed key, distinct supervisor UID/GID, and service process
receiving the fixed listener descriptor.

The remaining production join must:

1. deploy and provision the protected service authority described above;
2. wire Cargo and the rustc/backend through the exact client session so fresh
   completion produces one real V2 envelope;
3. consume the retained owner through fe2o3 host admission, establish the
   missing repository and descriptor-source lineage, and supply the complete
   carriage, semantic handoff, proof receipts, exact artifact bytes, generated
   marker contract, and Ferric request identity to a reviewed protected
   verifier;
4. compare the carried issuer policy with protected configuration and
   atomically enforce the external monotonic rollback position;
5. establish the missing proof/executable, Rust layout, Rust effect, and
   operator-refinement identities before returning fe2o3's verification
   decision; and
6. only then consume the authenticated executable with the generated
   `Marker`/`Arguments`, checked gfx942 device, exact profile geometry, and KFD
   completion custody.

No synthetic verifier, echoed request fields, ambient receipt, V1 fallback, or
HIP-derived authority is admissible.
