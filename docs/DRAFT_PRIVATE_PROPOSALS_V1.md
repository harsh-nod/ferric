# Private Autoregressive Draft Proposals

This additive engineering host path generates K4/K8/K16 proposals through the
real paged Draft06B driver. It is not a complete speculative serving loop,
protected publication, a new Verus proof, numerical qualification or a timing
result. Kernels, fe2o3, runtime defaults and existing supplied-candidate rounds
are unchanged. The standalone two-output paged canary is also unchanged.

## Transaction

The existing paired owner retains the request generation, epoch, plan, role
selections, equal committed role prefixes and deferred anchor. Its new
`reserve_proposals` takes no candidate vector. It derives the finite width from
the attached selection, checks counters and context, and reserves target K+1
rows before draft K rows. Draft reservation failure aborts the still-unsubmitted
target reservation. Aborted aggregate and per-forward IDs are never reused.

Unresolved tokens are private reservation placeholders. Only `proposal_work`
can expose the next one-row view: first the anchor, then the previous sealed
draft choice. Each view has its own monotone driver batch ID, distinct from the
aggregate draft reservation ID. The reserved pages remain owned throughout,
and the draft pool stays submitted between completed rows. Neither role's
committed tokens advance during generation. Ordinary `commit_batch` is never
used to make a tentative row visible.

`execute_speculative_proposal` executes the actual draft forward with output
row zero selected. It seals the exact input, one valid FP32-head choice and
completion to the request, epoch, role, aggregate reservation, ordinal and
one-row batch identity. Ordinary mutable driver outputs cannot be converted to
this result. `record_proposal` validates the result before updating both private
reservation states and the next input. All K individual completions are retained;
the owner does not manufacture a whole-batch GPU completion.

Only after K sealed results can `target_work` expose the resolved anchor plus K
candidates. The existing real target driver seals all K+1 verification choices.
Settlement rechecks the retained proposal evidence, uses the existing greedy
verifier, preflights both trimmed prefix states, then replaces both atomically.
Draft choice provenance does not replace target acceptance.

## Acceptance And Failure

For pre-round cursor C and accepted count A, the unchanged retained cursors are
target C+A+1 and draft C+min(A+1,K). Partial/rejected suffix bytes are outside the
committed causal extent and must be overwritten before future visibility. Cached
or shared prefix pages and unrelated sequences are not changed.

Full acceptance still requires a separate one-row catch-up consuming the last
draft candidate, not the target bonus. Its batch ID follows every proposal-row
ID. No next round starts until catch-up completes and both role prefixes agree.

Dropped work blocks progress. A malformed, stale, foreign or replayed result
terminalizes both submitted pools without a half commit. A driver execution or
queue-preparation error requires `fail_submitted` and separate closure of both
drivers, as in the existing paired path. Submitted reservations cannot abort.

K16 means sixteen complete one-row draft forwards, not one sixteen-row forward.
Long-lived operation uses the existing explicitly enabled queue-rollover
transport. The driver already prepares a complete forward's packet budget
before execution and rejects exhaustion when rollover is absent. This slice
does not enable rollover by default or add an unchecked queue reset.

## Validation Boundary

Focused metadata tests cover every accepted count at K4/K8/K16, cursor/page
boundaries, target/draft OOM before a first ticket, ID exhaustion and abandonment,
blocked/dropped tickets, exact choice/completion/ordinal bindings, paired failure,
cached-prefix framing and full-acceptance catch-up. Recording-transport tests run
the actual draft and target host methods, feed each sealed choice into the next
forward, and cover zero/partial/full acceptance, two rounds, failure after a
completed row, target failure and the existing rollover boundary.

These tests use synthetic compact choices and do not emulate GPU arithmetic.
Independent paged draft references, native K-row generation, paired-model greedy
parity, long-lived lifecycle checks and a serving-loop/publication join remain
separate work. No performance comparison is implied.
