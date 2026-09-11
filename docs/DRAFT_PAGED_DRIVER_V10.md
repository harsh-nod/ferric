# Paged Draft06B Driver V10

This is an opt-in engineering host driver, not a complete speculative engine,
protected publication capability, new Verus proof, or performance qualification.
Target constructors, default dispatch paths, and all-or-nothing pool commits are
unchanged. The standalone contiguous draft canary remains a separate path.

## Admission And Storage

`EngineeringTpArtifactV1::open_draft32` binds the real compiler-generated v10
closed14 roster. It has no caller-provided expected-roster parameter. All roots,
including pure RMSNorm, residual, and FP32 head/argmax, have distinct draft names.
Target v5/v8 and large-KV v9 images cannot substitute for this artifact.

`tp_execution::batched::EngineeringTpDraftBatchExecutionV10::new` requires exact
Draft06B geometry, TP1, a fresh independent 32-row pool whose model identity is
the loaded model identity, and a worker containing the complete admitted v10
image before any allocation. Weight intake uses the existing authenticated
layout, per-section integrity checks, and role-specific tensor plan. Session
and unique pool identities are retained for every submitted batch.

Physical storage is the legacy bounded pool (at most 512 pages), with logical
context at most 8192. At the full 512-page limit the 28-layer BF16 KV payload is
939,524,096 bytes. The FP32 head workspace is 19,447,808 bytes. Neither count
includes weights, transposes, runtime allocation overhead or other workspaces.

Only scalar or MFMA projections are admitted. The draft profile always uses
baseline attention, the draft TP1 device residual, pruned output selection, and
FP32 logits/argmax. It exposes no wave, peer, sequence, ordered-batch, large-KV,
numerical-capture, or target configuration escape. No CLI is added in this slice.

## Completed Inputs

`execute_selected` supports ordinary paged prefill/proposal work. Its mutable
ordinary choices are not speculative settlement evidence.

`execute_speculative_draft` consumes the owner's opaque draft ticket. It checks
the role and exact bound prepared batch, executes the real forward path, then
seals only the completed input rows and completion inside the driver. It does
not accept a vector of tokens or a caller-provided completion. Consumed-input
work selects no head output, so no draft argmax provenance is asserted. Proposal
IDs remain untrusted until target greedy verification accepts them.

After full acceptance, the distinct one-row catch-up ticket contains the last
accepted proposal at the missing draft cursor, never the target bonus. The next
round stays blocked until that GPU work completes and its opaque result is
recorded. Partial or zero acceptance retains only the validated consumed prefix.
No already-published token rollback is exposed.

The existing failure contract is unchanged: after an execution error the owner
must call `fail_submitted` and close both drivers. Uncertain completion cannot
produce an opaque success. Dropped tickets remain blocked, not implicitly
committed or reusable. Both metadata states are validated before settlement,
and a stale or substituted result terminalizes/quarantines the pair.

## Validation Boundary

Recording transports execute the actual host forward and settlement methods
with synthetic byte outputs. They test exact draft root/grid/active-row routing,
scalar/MFMA weight selection, rejection before allocation, A=0/partial/K,
page15-to16, failure, and two rounds separated by completion-bound catch-up.
They do not emulate GPU math. Image-bound host admission independently checks
the actual emitted artifact; native arithmetic and complete paired-model runs
remain distinct gates. No `StepPublication` or protected refinement claim is
made by these host tests.
