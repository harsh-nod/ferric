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

This binder is not the future fe2o3 adapter. It accepts a caller-supplied,
untrusted identity projection and does not decode an envelope, validate a
carriage, consult protected policy, or enforce rollback currentness.

## Upstream Join

At the time of this audit, fe2o3's active Ferric integration branch has the
required V2 codec, strict restart recovery, retained carriage, and V2-only host
admission. Fresh Cargo completion intentionally fails because live
`CompilerExecutionClientV1::acquire` wiring does not yet supply the carriage.
Newer fe2o3 main separately adds protected issuer `clone3`/pidfd ownership,
readiness, cancellation, and exactly-once reaping. Ferric must pin the eventual
merged head and revalidate the complete API; neither intermediate branch alone
is the production dependency.

After that service path is wired, Ferric must add a narrow adapter that:

1. consumes one `RecoveredWorkerV3LoadEnvelopeV2` and admits the exact generated
   SwiGLU marker, never a caller-selected kernel identity;
2. projects every carriage identity above from the same strictly decoded owner;
3. supplies the complete carriage, semantic handoff, proof receipts, exact
   artifact bytes, generated marker contract, and Ferric request identity to a
   reviewed protected verifier;
4. compares the carried issuer policy with protected configuration and
   atomically enforces the external monotonic rollback position;
5. establishes the missing proof/executable, Rust layout, Rust effect, and
   operator-refinement identities before returning fe2o3's verification
   decision; and
6. only then consumes the authenticated executable with the generated
   `Marker`/`Arguments`, checked gfx942 device, exact profile geometry, and KFD
   completion custody.

No synthetic verifier, echoed request fields, ambient receipt, V1 fallback, or
HIP-derived authority is admissible.
