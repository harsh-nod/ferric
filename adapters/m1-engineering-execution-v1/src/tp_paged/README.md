# Engineering TP Paged Pool

This module is a **Contracted**, non-authoritative host implementation. It is
outside the protected Engine/cache dependency closure and does not fabricate
Engine page custody, completion proof, or M1 authority. No closed Verus proof
is claimed for these executable bodies. Tests and invariant checks do not
substitute for the same-source refinement proof required by protected bundles.

## State and Boundary

One unique process-local pool identity and exact nonzero model/session scope
bind the metadata to one resident driver allocation set. A driver must accept
only a never-used pool for fresh GPU allocations. The session must cover the
exact resident model, kernels, target, ranks, layout, and numerical convention.
Equality of caller-provided scope bytes is not external authentication.

The fixed limits are 16 tokens per page, at most 8192 context tokens, 32 live
sequences, 512 physical pages, and 16 token rows per batch. Physical page IDs
index the same layout across all ranks/layers. The row table stride is
ceil(context_tokens / 16), not an implicit 512 when a smaller context is used.

Each sequence retains a committed token prefix and logical-to-physical table.
Each physical page's reference count equals sequence references plus at most
one immutable cache pin. Partial pages have exactly one reference and no cache
pin. The radix tree uses complete 16-token edges and parent identity: equal
suffix tokens after different ancestors cannot collide. Complete cached pages
survive request retirement in this resident session, not process restart or
disk persistence. Prefix lookup leaves at least the final prompt token to
execute; KV alone cannot supply the next-token logits.

Reservation stages all selected sequences in one bounded proposed state. Any
invalid row, OOM, stale/foreign identity, or overflow leaves committed metadata
unchanged. Only one batch may be pending; dropping its returned metadata does
not recycle pages. The caller marks submission before device effects. Abort is
permitted only before submission. Successful driver completion of every layer
and every rank mints a crate-private completion token, after which commit moves
the whole proposed state into committed state. This token is a trusted
engineering host contract, not an authenticated hardware receipt.

No other mutating operation, including admission, retirement, expiry, eviction,
or cancellation, may run while a batch is pending. Submitted uncertainty
quarantines the whole pool, including unused slots; no cancellation, expiry,
abort, or reset can return them to a free list. The caller must close all
resident workers and create a new driver/pool before resuming.

Retirement may publish only complete immutable pages. Duplicate computed
prefixes retain the existing canonical cached physical page and release the
duplicate request pages. Monotone caller-clock TTL expiry removes cache pins
without invalidating live sequence references. LRU leaf eviction reclaims only
cache-only pages. Both operations are bounded and transactional. The caller
chooses expiry/eviction policy; reserve itself never silently evicts on OOM.

## Assurance Work

The last checked boundary is host metadata and scheduler-to-row ownership.
Required external contracts are truthful all-rank/layer completion, correct
resident allocation binding, causal paged attention, exact token/model/session
identity, deterministic compatible numerical KV semantics, and fail-closed
driver teardown. This module does not prove these contracts, machine code,
GPU memory isolation, transport liveness, performance, or protected serving.

Future same-source proofs should establish reference-sum conservation,
exclusive partial-page writes, exact radix-prefix correspondence, no physical
reuse during submission, fail-atomic reservation/commit/abort, and permanent
quarantine. Any proof must cover these actual transaction bodies rather than a
separate model asserted to correspond. Until then, deployments requiring these
properties as Proved must reject this engineering module.

Focused tests cover sharing and canonicalization, partial-page isolation,
cross-ancestor/model/session/pool misses, all-rank completion ordering, stale
batch/sequence identities, cancellation, TTL pin retention, LRU/OOM atomicity,
capacity/overflow, maximum 8192-token/512-page geometry, and deterministic adversarial
transition sequences. Actual-body negative mutation checks are supplementary
test evidence only, not Verus authority. Integration owns protected-source
inventory/export changes and the cross-module completion contract.
