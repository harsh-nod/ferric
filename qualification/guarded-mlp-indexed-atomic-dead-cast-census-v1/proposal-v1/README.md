# Indexed Atomic Dead-Cast Census Proposal

Source-only proposal against the actually qualified membership producer. No
compiler, tests, candidate import, or remote command was run by the author.
This does not claim successful guarded lowering, HSACO emission, alias
admission, or GPU execution.

## Measured Baseline

The retained membership CPU receipt is `d13a558c...` (38 phases, 25 scopes,
2,876 passing executions and 25 ignored tests). Its actual source map is
`ff47dcc2...`, input `a349d29a...`, and controller `bb91990a...`.
The retained subsequent guarded compilation failed naturally with receipt
`914ba560...` and stderr `7dbc29c0...` at
`indexed_atomic_dead_cast_v1.rs:99:23`: work before 3,145,728, charge 1, sum
3,145,729, unchanged ceiling 3,145,728. The authenticated context is semantic
function 1, hash `72f186b0...`, kernel root
`ferric_qwen3_mlp_state_guard_v1`. This is the old temporary-unused scan's
statement charge, not a membership lookup or an inferred alias verdict.

The base is reconstructed without copying a full tree: the three retained
membership postimages override the unchanged qualified diagnostic tree at
`guarded-mlp-ranked-graph-work-diagnostic-input-v228-v2/fe2o3`. Every changed
preimage is joined to the actual membership source map. The manifest records
complete byte/hash pins; abbreviated hashes above are explanatory only.

## Scope and Equivalence

Four source paths change: `indexed_atomic_v1.rs` creates one local holder in
`reject_indexed_atomic_escapes_v1`; `indexed_atomic_dead_cast_v1.rs` implements
the census; the existing dead-cast test helper constructs a fresh holder and
includes one new test file, `indexed_atomic_dead_cast_census_v1_tests.rs`.
The resulting source closure adds one file, from 5,800 to 5,801 source bodies.

The holder owns an immutable function reference and is never stored in the
allocation inventory. Its query cannot be supplied another function. Existing
cast-kind, source-membership, destination/type, raw-pointer, and exact physical
layout checks run before any census query. No pointer, allocation, atomic,
source-origin, or alias authority is added.

Each valid local has one of three states: never mentioned; mentioned only in
one statement site; or mentioned in multiple sites or any terminator.
Repeated occurrences in the same statement remain one site. This preserves
the old scan's exclusion of the *entire* defining statement, not just its
destination or one occurrence. A terminator cannot equal a definition site.
Every declared block is scanned, including unreachable blocks. StorageLive
and StorageDead remain ignored; every other old statement/rvalue/terminator
mention position is retained, including projection Index locals. The
Temporary role check remains before charges, preserving implicit Return-local
behavior. Zero mentions still qualifies with an absent definition. Unrelated
out-of-range local IDs are ignored rather than introducing a new semantic
refusal; valid Index locals on those places are still counted.

## Resource Policy

This is a resource-admission change, not diagnostics-only. The work ceiling,
block/edge limits, semantic predicates, and independent analysis/storage
ceilings remain unchanged. The old repeated scan is replaced by one complete
census plus constant-time queries. A negative query that previously stopped
early may now do more work; no unconditional admission/performance claim is
made.

New compiler scratch is explicit: a single Vec with one mention state per
declared local, capped before allocation by the existing 262,144 capability
state-entry ceiling. The requested payload is at most
`262144 * size_of::<IndexedAtomicDeadCastMentionV1>()` bytes, in addition to
the existing inventory. This is not an unchanged-memory-footprint claim or a
new global shared storage accounting rule. Allocation uses try_reserve_exact;
allocator failure is a refusal. No cloned function, per-local search cache,
unbounded map, or second census buffer is retained.

Charges precede initialization, each block, statement, terminator, operand,
and place/projection visit. For N locals, B blocks, S statements, T
terminators, O visited operands, and P equal to the sum of (1 + projection
count) over visited places, construction charges N+B+S+T+O+P. Each valid query
adds one, including cache hits. The existing per-charge Cell commit semantics
are unchanged. Construction uses a private temporary Vec and publishes it
only after all traversal and charges succeed; a failed build leaves no cache.
Earlier successful charges are not rolled back. A completed census stays
valid after a later query-budget refusal.

## Authored Tests and Integration

Nine new names use filter
`production_ranked_projection_v1::tests::indexed_atomic_dead_cast_census_`;
the exact sorted roster is in the manifest. A test-only reference is the
qualified full scanner copied byte-for-byte except its function name.
Thirty-five statement variants/positions and 24 terminator variants/positions
give 1,652 differential queries across all fixture locals and present/absent
definition sites. Additional checks cover repeated same-site mentions,
multiple sites, terminators, ignored storage, roles, invalid IDs, unreachable
blocks, exact work boundaries, every insufficient construction allowance,
retry after failure, scratch bounds, immutable-function isolation, unchanged
real cast/escape negatives, and 1,656 distinct candidates sharing one scan.
OS allocator failure is not simulated; fallible reservation is source-reviewed.

Root applies `integration.patch` only to a fresh source generation based on
the actual membership map, verifies all four postimages, and runs the full
prior CPU qualification plus the new cohort. The previous 38 recipes remain;
one focused repeat would give 39 phases and 26 scopes. Nine added tests in the
full compiler suite plus nine repeated executions would conditionally give
2,894 passing executions with the same 25 ignored tests. Those are planned
counts, not results. Actual guarded lowering must run separately with the
new qualified producer and audited tools; its outcome remains unknown.
