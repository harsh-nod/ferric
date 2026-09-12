# Paired Accepted-Prefix KV Settlement

This is bounded engineering host support, not a speculative serving engine,
protected publication, verified physical refinement, GPU qualification, or a
performance result. The additive paged Draft06B driver join is described in
`DRAFT_PAGED_DRIVER_V10.md`; native paired-round qualification remains separate.
The additive private autoregressive proposal transaction is described in
`DRAFT_PRIVATE_PROPOSALS_V1.md` and leaves supplied-candidate rounds unchanged.

## Boundary

`tp_paged::speculative::EngineeringTpSpeculativeKvV1` owns two independent,
already-prefilled target/draft pools and binds their exact sequence IDs to one
request generation, completion epoch, plan identity and role selections. Both
committed token prefixes must be identical at attachment. The round anchor is
the next unprocessed input; it is not part of either resident prefix yet.

No mutable pool access, arbitrary committed-cursor truncation, or token
publication API is exposed. Existing target-only `commit_batch` still commits
the complete proposal. Existing pre-submission abort and submitted quarantine
rules are unchanged. Driver lifetime and orderly close remain caller duties.

`reserve_round` validates `SpeculativeKvRoundIndex::validate_for` against the
retained identities and reserves the exact target K+1 and draft K input rows.
The supported finite widths remain K4/K8/K16; K16 needs a 32-row pool. A failed
second reservation aborts the first reservation without publishing either
prefix. Batch IDs consumed by an abandoned reservation are never reused.

The supplied candidate IDs are untrusted proposals, not evidence that draft
argmax produced them. This is sufficient for greedy target verification, but
it is not a claim about draft-generation provenance or quality.

## Completion And Choices

`target_work` marks submission before returning an opaque borrowed ticket.
`EngineeringTpBatchExecutionV2::execute_speculative_target` consumes this ticket,
executes every target output row, and seals choices, exact selected row indices,
input rows, completion and role/request/epoch/plan/batch identity before returning.
Ordinary `EngineeringTpBatchOutputV2` exposes mutable choices and deliberately
has no public conversion to the opaque speculative result.

`record_target` accepts only the sealed result. `settle` takes no caller-supplied
accepted count or choice vector: it applies the existing `verify_greedy_round`
to the sealed choices and the exact retained index proposals. Both target and
draft role completions are required before settlement.

The ordinary batched target constructors still reject Draft06B geometry.
The distinct `EngineeringTpDraftBatchExecutionV10` admits the complete independent
v10 image and exact loaded Draft06B scope before allocation. Its
`execute_speculative_draft` consumes `EngineeringTpSpeculativeDraftWorkV1` and
seals the exact completed inputs inside the driver. This includes the separately
identified catch-up work. `EngineeringTpSpeculativeDraftResultV1` still has no
public constructor or conversion from ordinary output. Standalone contiguous
draft canary outputs do not satisfy this join.

## Retained Prefixes

For a pre-round cursor C, width K and accepted proposal count A:

```text
target tentative end = C + K + 1
draft tentative end  = C + K
target commit end    = C + A + 1
draft commit end     = C + min(A + 1, K)
```

The correction or bonus is emitted logically by the greedy result and retained
as the next anchor, but is not marked resident. The two prefix states are
cloned, trimmed and invariant-checked before either owned state is replaced.
There is no fallible operation after the paired preflight succeeds.

Trimming cannot go below the previously committed prefix. Only newly reserved,
exclusive, uncached suffix pages can be released. Complete cached/shared prefix
pages and other sequences remain framed. A retained partial page can still
contain speculative suffix bytes; those bytes are outside the committed length
and causal attention extent, and must be overwritten by the next reservation
before becoming visible. No byte zeroing or speculative suffix readback is
claimed by this metadata operation.

## Full Acceptance

At A=K the target has consumed the final accepted candidate but the draft has
only produced it. The owner enters `DraftCatchUpRequired` with target cursor
one ahead. `reserve_draft_catch_up` selects exactly that candidate at the draft
cursor, never the bonus. The next round remains blocked until a distinct,
completion-bound draft catch-up result is recorded and both committed token
prefixes agree again. Aborting an unsubmitted catch-up returns to the required
catch-up phase, not to ready.

## Failure Rules

- Unsubmitted reservations can abort without changing committed state or epoch.
- A substituted, stale or duplicate opaque result during submitted work
  terminalizes the pair and quarantines both pools, without a half commit.
- Any driver execution failure after submission requires `fail_submitted` and
  separate closure of resident drivers. No free list becomes usable on failure.
- An incomplete pair cannot settle or start a later round. Dropped tickets do
  not silently release reservations or certify completion.

## Evidence And Remaining Proof Obligations

The implementation follows the existing immutable-preflight/infallible-apply
pattern in `settle_and_publish_speculative_step`. It reuses the existing Verus
index validator, commit-end accessors and greedy verifier. This new engineering
module is ordinary Rust and is not itself a Verus proof or an implementation
refinement of the protected composition.

Host tests cover every A for K4/K8/K16, page boundaries including15->16, exact
page conservation, stale role/epoch/request-generation/batch/row/choice binding,
failed GPU-step simulation, paired failure atomicity, shared cached prefix
framing, and two rounds separated by full-acceptance draft catch-up. Recording
transport tests exercise both real role-driver mints and completion-bound
catch-up without GPU arithmetic.

Remaining obligations include: composing authenticated target/draft lifetimes
with the full serving loop and native draft-proposal generation; refining engineering
page/refcount transitions to the protected isolated KV model; proving that
completed GPU writes match exact row/address bounds; composing the settlement
with `StepPublication` without independently mutable token publication; and
native numerical, lifecycle and multi-round speculative qualification. No
existing authority, source-policy acceptance or qualification tier is upgraded.
