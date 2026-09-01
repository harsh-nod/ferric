# M1 Qwen3 SwiGLU Production Admission

Status: fail closed pending a fresh receipt-bearing protected compiler occurrence,
Ferric's protected verifier state, and deployment qualification.

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
`2d275684d7a22f8f913114b51b1d1dd524d1ed9b`. Its raw entry point strictly
decodes V2 into an inert request. Its recovered entry point consumes and
retains one move-only `RecoveredWorkerV3LoadEnvelopeV2`, derives every
compiler-receipt identity from that owner's carriage, checks the exact carried
build and artifact bytes, and returns the owner with any validation failure.
There is no public parallel-identity input.

This remains an authority-free projection adapter, not the accepting host
adapter. It intentionally has no dependency on `fe2o3-host`. The accepting
path must use the exact subject and carriage retained by upstream host
admission, independently compare protected policy and Worker ledger state, and
enforce an external monotonic rollback anchor. Repository commit labels remain
Ferric policy metadata rather than compiler-receipt fields. Ferric does not
implement an accepting protected backend while those independent inputs are
unavailable.

## Upstream Join

The pinned fe2o3 integration head combines the V2 codec, strict restart
recovery, retained carriage, and V2-only host admission with pidfd-owned issuer
lifecycle, the fixed protected listener, a bounded worker pool, and the
policy-bound compiler-execution client. Cargo now transfers one fixed inherited
receipt descriptor to rustc, the backend acquires the subject-bound receipt,
and Cargo reconstructs and admits the durable V2 carriage under currentness.
The upstream host boundary retains the exact compiler subject and carriage,
binds them into lineage evidence, and requires receipt-complete verification
before promotion. Its sealed `WorkerV3ProtectedVerifierAdapterV1` constructs the
decision from independently supplied protected evidence. A backend must cross
an explicit `unsafe` implementation boundary, cannot construct the decision,
and cannot choose the request coordinates that the adapter verifies.

The integration head also provides authenticated service acquisition and exact
verification of the current compiler Worker ledger record. This is the generic
transport and comparison primitive; it is not a deployed Ferric ledger service
or a source of Ferric policy authority.

Those generic mechanisms do not provision Ferric's production authority.
Deployment must still provide the launcher and issuer images, service-owned
root, sealed keys, distinct supervisor UID/GID, and the service process that
receives the fixed listener descriptor. Ferric must also provide independently
protected policy, Worker ledger, and external monotonic rollback state to an
upstream accepting adapter. Any seven compiler-produced Worker V3 owners
accepted by these adapters are structural, inert evidence only; they do not
prove that those protected services or authorities exist.

Ferric now rejects any of those owners unless its canonical link plan uses
exactly COV6, O2, debug stripping, and per-stage verification with the reviewed
default execution limits. Both transcript passes must also retain the 64 MiB
bootstrap ceiling and exact artifact-length replay ceiling. The retired Worker
V2 `ferric-m1-kernel-artifacts` command has been removed. There is no
replacement executable workflow until an authenticated in-process collector
can acquire all seven V3 owners and pass them to the fail-closed artifact
publisher.

All seven kernel families now have exact generated `Marker`/`Arguments`
expectation declarations in Ferric source. Those declarations are expected
compiler output shapes, not receipt-bound current artifacts or authenticated
owners. None of the seven lanes has the complete authenticated proof/executable,
Rust layout, Rust effect, and safety-property evidence needed for an accepting
protected decision.

The remaining production join must:

1. deploy and provision the protected service authority described above;
2. run a fresh protected Cargo/backend occurrence through the deployed service
   so it produces one real V2 envelope for the current artifact;
3. consume each retained owner through fe2o3 host admission, establish the
   missing repository and descriptor-source lineage, and supply the complete
   carriage, semantic handoff, proof receipts, exact artifact bytes, generated
   marker contract, and Ferric request identity to a reviewed protected
   verifier;
4. compare the carried issuer policy and Worker ledger record with independently
   protected configuration and atomically enforce the external monotonic
   rollback position;
5. establish the missing proof/executable, Rust layout, Rust effect, and
   operator-refinement identities before returning fe2o3's verification
   decision; and
6. only then consume the authenticated executable with the generated
   `Marker`/`Arguments`, checked gfx942 device, exact profile geometry, and KFD
   completion custody.

No synthetic verifier, echoed request fields, ambient receipt, V1 fallback, or
HIP-derived authority is admissible.
