# Indexed Atomic Statement Lookup

This source-only proposal replaces one measured linear statement-site lookup
with bounded binary search over its existing immutable ordered inventory. It
does not change kernel code, atomic or alias predicates, source authentication,
allocation layout, graph/block/edge/fact limits or any other lookup. The author
has not imported or run candidate code, tests, a compiler or remote commands.
Root owns integration and qualification.

## Actual Baseline

The direct qualified producer is the dead-cast census CPU generation:
`F/qualification/guarded-mlp-indexed-atomic-dead-cast-census-v1/attempt-v1`,
where `F=/home/harsh/ferric-p227-integration`. Its actual complete receipt is
`3a5ae5b18b497282a05895b8beca033f316a261c5632f4fbecf02832cd552e3f`;
the source map is `94d7a9ffede6584dbd1403273bbb6312fb99c24a767c8b6156c10c37c3fbc1ae`.
The manifest binds the actual input and controller as well.

The subsequent guarded compilation failed naturally with no HSACO. The retained
failure is 12,321 bytes / SHA-256
`34699054a75a7300bd308d99e22635c25237c3d85ecb8f15a1ca9de00c5831fd`;
stderr is 8,554 bytes / SHA-256
`ba24844fd07b9b9a7bc3d5934ff29b297a85ed965a3871a21b917fda5da0c044`.
Its authenticated graph-work diagnostic identifies `indexed_atomic_v1.rs:101:14`,
`validate_statement`, with before=3145488, amount=552, sum=3146040 and
limit=3145728, for function 1 / `ferric_qwen3_mlp_state_guard_v1`.
The charge failed before the old linear scan. This is baseline attribution,
not an outcome of the proposed optimization and not an alias verdict.

## Invariant and Cost

The only production `indexed_uses.push` is in
`authenticated_atomic_allocations_v1`, inside the existing nested ascending
block/statement `enumerate` loops. Each statement is charged before inspection;
there is at most one push per site. No later production mutation changes this
vector. Empty/default and cloned inventories preserve the order. No sort,
auxiliary index, cache, bitset or extra compiler scratch is introduced.

`statement_use` compares lexicographic `(block, statement)` keys. Each iteration
reduces the remaining interval to at most half its previous length. It charges
`floor(log2(n))+1` key comparisons before any search for nonempty `n`, and still
calls the existing charge function with zero for empty inventories. The original
charge wrapper, checked overflow, work ceiling and failure Cell noncommit are
unchanged. The implementation carries the matching reference on equality and
continues left, preserving the original linear `find` first-match behavior even
for duplicate test keys. There is no additional uncharged key comparison after
the loop. Safe midpoint arithmetic also handles extreme queried keys.

At the measured inventory length 552, the new bound is 10 comparisons instead of
552 row inspections. This is a static work-accounting bound, not measured speed
or a prediction of successful lowering. Logical compiler resource admission can
change and is explicitly reported as such. Existing independent ceilings are
unchanged. Other linear queries (`usage`, `authorizes_bounds`, `authorizes_root`,
`permits_address` and `atomic_only`) are deliberately untouched because this
proposal addresses only the measured caller.

`validate_statement` still accepts an absent key exactly as before and applies
the unchanged exact effect/address/kind/ordering/scope comparison to a matching
use. Source-origin, pointer escape, bounds, marker, ambiguous-origin and alias
checks remain independent and unchanged. The vector remains private; arbitrary
external unsorted inventories are not a supported construction path.

## Source and Tests

The three-path patch is relative to the qualified census source map. It changes
`indexed_atomic_v1.rs`, appends one include to `indexed_atomic_v1_tests.rs`, and
adds `indexed_atomic_use_lookup_v1_tests.rs`. The local diagnostic source tree
plus the manifest's six authenticated membership/census overrides supplies the
preimages without another whole-tree copy. All other bodies are unchanged.

Nine authored tests compare returned pointer identity against the original
linear `find`, including 6,400 exhaustive small-set queries, empty/singleton/odd
and power-of-two boundaries, length 552, the existing custody scale, duplicates
and extreme keys. Charge tests cover exact-limit success, over-limit refusal,
overflow, empty zero-charge behavior and unchanged work on failure. Independent
inventories/clones are checked. A synthetic 552-use inventory serves 8,192 queries
with an asserted 81,920-unit charge; this is not GPU execution or a benchmark.
Real authenticated fixtures verify sorted construction, exact statement binding,
absent-key behavior, and rejection of changed address, access kind, ordering or
scope, missing/ambiguous guards and pointer escapes.

The source manifest contains the exact nine names and pins. They are unexecuted
by the author. The proposed fresh qualification retains all 39 previous recipes
and adds one nine-test repeat: conditional 40 phases, 27 scopes, 2,912 passing test
executions and 25 ignored statuses, counting full-suite/repeat executions rather
than distinct tests. The source addition gives 5,802 source bodies and 5,804 total
source/controller/helper rows. Actual inventory, source and outcome joins remain
execution gates; no new compiler, guarded-kernel, GPU, numerical, model or
performance qualification is claimed here.
