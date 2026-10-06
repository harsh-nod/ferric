# Guarded MLP Memory-Bounds DAG Proposal

This directory contains source-only compiler changes. It does not claim a new
compiler qualification, a successful guarded-kernel compile, GPU execution,
numerical correctness, or a performance result. The root agent owns integration
and all execution on the shared host.

## Authenticated Starting Point

The base is the qualified linear-fusion CPU attempt v3: receipt
`7827d969f295a8a0ba709629b2679b71ecde9b93b1f0ad11c5b8be70f7259915`,
source map `0c5a0dce518d1d469d0f7cb34618245aedb74d9279a4aa50473f2f3e20a1de23`.
That attempt completed 44 phases and 31 scopes with 2,972 aggregate passes and
25 ignored results. These are previous results, not results for this proposal.

The subsequent guarded compile failed naturally with a memory-bounds preflight
resource refusal. Its 17,737-byte failed receipt is
`8d4cc1a0cfe0c9104282c3d20349638a616bd98e7e28f92598dcaf0110057ca2`;
its 123,245-byte stderr is
`c60dc2edc62bfe11ba7a8f79ce1465322d2c887aeb3fa5661dae8002a33c5095`.
The rendered graph has 567 blocks, 1,122 raw edges, 2,240 operations, and
552 less-than guard candidates. Independent source/data inspection found every
block reachable, no cycle, no ownership contract, and 552 distinct guard facts.
The diagnostic does not establish runtime work exhaustion or an independent
edge-limit verdict.

The three replacement files are reconstructed from their qualified hashes, not
assumed to match a development checkout. The proposal adds a schedule module and
one nine-test module. It changes 5 source files in total, with 2 additions:
5,806 base compiler files become 5,808, or 5,810 including the two CPU helpers.

## Why the Old Estimate Refuses

Let B be blocks, E edges, O operations, F the guard-fact bound, and
W = ceil(F / 64) the number of bitset words. The old preflight reserves

```text
(B + E) * (W + 1) * (F + 1)
```

for FIFO intersections and revisits. The old runtime starts its FIFO in physical
block order; acyclicity alone does not make that existing execution one-pass.

For the retained graph, F = 552 and W = 9. The intersection term alone is

```text
(567 + 1122) * (9 + 1) * (552 + 1) = 9,340,170
```

which exceeds the unchanged internal work limit of 8,388,608. This is a derived
conservative preflight term, not a measured operation count.

## Selection and Proof

1. Run the old preflight first. If it admits the graph, return its exact bound
   with no schedule and retain the old FIFO execution.
2. Only the exact old internal work-hard-limit refusal is eligible for rescue.
   Other structural, storage, arithmetic, or outer-envelope refusals stay refused.
3. Require zero ownership contracts. Hierarchical ownership can independently
   rerun legacy FIFO under a composed bounds estimate. This conservative
   restriction prevents a one-pass estimate from authorizing that nested FIFO
   execution. Already-admitted contract graphs keep the old route.
4. Admit discovery work and storage before graph traversal or new allocation.
   Build a Kahn order over the whole successor multigraph, including unreachable
   components and duplicate edges. A cycle anywhere, malformed inventory,
   external successor, or allocation failure returns the old refusal.
5. Retain the function pointer, ordered block pointers, exact successor rows,
   and complete topological order. Immediately before one-pass propagation,
   rejoin the function and fresh function-region roster, exact block/edge
   identities, a complete permutation, and strict edge precedence.
6. Any certificate mismatch fails closed. An admitted one-pass bound never
   silently falls back to FIFO. The certificate is dropped after the bounds
   stage, before subsequent stages.

Every reachable predecessor is final when its successor is visited. Induction
over the checked topological order therefore gives the same predecessor
intersection and guard generation as the converged FIFO equations. Entry keeps
the empty set even if an unreachable predecessor points into it. Unreachable
blocks are not processed and retain the initial full/top set, matching legacy
behavior even when an unreachable predecessor enters a reachable join. Their
existing unreachable diagnostics are retained.

Forwarded block arguments continue to use the existing separate transport
analysis. The global bitset propagation remains disabled for those graphs;
neither the transport proof nor an access predicate is changed. All access,
checked-domain, literal-equality, Presburger, structural-verification, finding,
and report logic is inherited unchanged.

The remaining standalone race, semantic-refinement, workgroup-memory, and
barrier entry wrappers that rerun FIFO are test-only. Production uses their
with-analyses/scoped routes. Hierarchical ownership is the only production
nested FIFO consumer found by the source call-graph audit, and the explicit
eligibility restriction covers it.

## Resource Derivation

All charges below use the compiler's existing logical work/storage item model,
not elapsed CPU instructions or a byte-exact allocator measurement. No work,
storage, fact, block, edge, identity, or projection limit increases.

A topological pass calls the existing predecessor intersection once per
reachable non-entry block, then charges one item per processed block. At most E
predecessor edges each charge one edge step plus W bitset-word steps, so

```text
one-pass propagation <= B + E * (W + 1)
                     <= (B + E) * (W + 1).
```

The preflight uses the latter conservative term. All other existing terms stay
in the bound; setting the wave factor to one is valid only with the retained
and runtime-validated certificate.

The added discovery allowance is `2*O + 12*B + 8*E`. The two operation scans
conservatively cover block/terminator inventory access. Block terms cover the
fresh region roster, index construction, successor-row construction, indegree
initialization, zero-indegree scan, bounded queue insertion/removal, order
construction, retained roster copy, and loop/identity checks. Edge terms cover
target lookup and recording, indegree increment/decrement, successor scans, and
the zero-transition checks. Each vertex enters the queue at most once and each
raw edge, including a duplicate, is consumed once by Kahn traversal.

Runtime validation reserves `8*B + 4*E`: row-size census, block/row identity
joins, fresh region roster, position-map initialization, permutation checks,
and all strict edge-order checks. The first B charge precedes the row-size
scan; the remaining `7*B + 4*E` is charged before equality and topology scans.

The additional preflight storage charge is `12*B + 4*E`. A conservative sum
rather than a peak-only subtraction accounts for construction and later
validation even though their temporary lifetimes do not overlap:

| Logical allocation | Block items | Edge items |
| --- | ---: | ---: |
| Discovery index key/value entries | 2B | 0 |
| Successor row records | B | E |
| Indegrees and bounded queue | 2B | 0 |
| Order and retained block roster | 2B | 0 |
| Runtime retained roster, rows, order, and temporary positions | 4B | E |
| Extra conservative allowance | B | 2E |
| Total | 12B | 4E |

The runtime budget independently charges the retained certificate and temporary
position map as `4*B + E` before reserving the map. The preflight includes both
discovery and validation work in the unchanged internal work-cap check and the
outer phase envelope. It includes all additional storage in the unchanged
internal storage-cap check and phase envelope.

Every new vector, queue, hash table, and successor row uses fallible reservation
before resize, push, insertion, or copy. The queue and order cannot grow beyond
B; successor rows are bounded by the authenticated E census. No partial
certificate is published on error. Allocation failure is a refusal, not an
assumption that allocation succeeds. The logical item reservation is not a
claim about exact allocator capacity or bytes.

For the retained graph, the proposed arithmetic is:

```text
one-pass intersection allowance: 16,890
discovery allowance:            20,260
runtime validation allowance:    9,024
additional storage allowance:   11,292
```

These are derived terms only, not a new full phase bound or a measured runtime.
The remaining inherited terms can still refuse admission. A successful
memory-bounds stage also does not imply that later production gates pass.

## Validation Plan

The independent nine-test module covers old-route preservation, differential
DAG/physical-order cases with a set-based fixed-point oracle, duplicate edges,
entry behavior, reachable and unreachable cycles, forwarded arguments,
cross-function and modified certificates, work/storage boundaries, bounds
refusals, and a dense-guard production route. The ownership-contract cases
cover both the old admitted path and refusal of an over-cap rescue.

The root-run qualification must keep all prior CPU recipes and add the focused
nine-test cohort. Expected accounting, conditional on all tests passing, is
45 phases, 32 scopes, 2,990 aggregate passes, and 25 ignored results. The new
Pliron tests are counted once in its full suite and once in the focused cohort.
The separately retained guarded-kernel compile, tool admission, and eventual
GPU correctness/performance evidence remain required.

