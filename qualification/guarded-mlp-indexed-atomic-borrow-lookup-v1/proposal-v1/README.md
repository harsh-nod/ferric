# Indexed Atomic Borrow Lookup

Source-only proposal against the actual qualified use-lookup generation. No
compiler, project import or test was run by the author. Root owns integration,
qualification, loader audit and any guarded retry. All compiler resource limits
and semantic authorization predicates remain unchanged; resource admission and
work cost may change. No performance or GPU result is claimed.

## Measured Boundary

The retained guarded failure is
`F/qualification/guarded-mlp-indexed-atomic-use-lookup-lowering-v1/attempt-v1`,
where `F=/home/harsh/ferric-p227-integration`. Its receipt is 13,412 bytes,
SHA-256 `5a9b97db2af7e9e6f152aa84f632112abb587a27f3ffb6dca14920b0657a53df`;
the 8,512-byte stderr is
`f47c6cd5b7bbd4d9238212cd6b570dfd6e7b2898d9518e5af435f110f4db1160`.
The actual diagnostic identifies `permits_address` at the qualified
`indexed_atomic_v1.rs:84:14`, before 3,145,230 plus charge 1,656 exceeds the
unchanged 3,145,728 work ceiling. The function is kernel root
`ferric_qwen3_mlp_state_guard_v1`, semantic function 1, hash
`72f186b0c49aafc62f29a7b77a5bbca4fc458c507019297c8bea173b272fe70c`.
The measured 1,656 is a charged amount, not an observed borrow-row count.
The natural failed compile emitted no HSACO and establishes no alias verdict.

The direct CPU baseline is complete `e6d2f3d5`, source map `4070da1f`,
input `4ae19d4b` and controller `2051a7e5`. Full pins, three exact overlay
preimages/postimages, seven inherited source overrides and the patch pin are
in `source-manifest.json`. The proposal adds one test file: 5,802 compiler/source
bodies become 5,803; the controller/helper-inclusive map becomes 5,805.

## Algorithm And Custody

Construction appends `indexed_borrows` in atomic use-site order, but each row's
`borrow.block` is its authenticated provenance definition block. A dominating
definition may be in a different block; binary searching the original append
order would be unsound. The sole constructor now explicitly orders this private
borrow vector by definition block before escape validation. Only
`permits_address` consumes this vector. Its existential predicate has no
first-match output, so reorderings and duplicate definition blocks are safe.
All row fields are moved intact; `indexed_uses` and every provenance/guard
predicate remain unchanged.

A small in-place heapsort charges before each sift visit, key comparison and
swap. Each heap-build root also charges one unit before entering its sift;
each extraction charges one unit before its swap. `root < end / 2` proves
`2 * root + 1` cannot overflow. The sort allocates nothing and never clones a
place. A charged failure may leave the constructor-local vector partly ordered,
but `?` returns before escape validation or publication of the authenticated
inventory. The existing per-charge Cell noncommit remains unchanged.

For N >= 2, H=floor(log2(N)), there are floor(N/2)+N-1 sift invocations,
each with at most H visits. Each visit charges at most four units; each invocation
has one external charge. Thus sorting charges at most
`(floor(N/2)+N-1)*(1+4*H)`. This is a conservative bound on this explicit code,
not an assumed constant for a library sort. Empty/singleton vectors need no sort.

The query charges two explicit binary-boundary searches, each at most
`floor(log2(N))+1` comparisons for nonempty input and zero for empty input.
The upper boundary compares directly with the block key, never `block+1`, so
`usize::MAX` is supported. After finding the block range, it charges exactly
`range_length*(query_projection_length+1)` before the original complete
`SemanticPlaceV1` equality check over that range. Local ID, projection kinds,
projection payloads/result types, projection sequence and final place type are
not abbreviated or hashed. All rows in the range are included in the charge,
even if the existential equality check returns early.

Both query charges use a temporary counter. The Cell commits only after both
succeed, so a failed search or bucket charge leaves it unchanged, just like the
old single-charge query. Empty queries still charge zero and therefore still
reject an already-invalid counter. Sorting adds bounded work and can reject a
generation earlier; a highly populated block still incurs its entire exact
place-comparison charge. The unchanged storage bound remains 262,144 entries;
blocks 2,048, edges 2,048 and graph work 3,145,728 are not enlarged. No new
compiler scratch allocation is added; inherited census scratch still exists.

## Tests And Scope

Nine authored, unexecuted methods use the manifest's exact common filter.
Coverage includes all 364 length-zero-through-five ternary block inventories
and 5,460 differential queries against the old whole-vector existential scan;
24 distinct valid places covering every projection kind and equality field;
empty/singleton/odd/power boundaries, shuffled/duplicate/extreme block keys;
exact two-row sort charges and every failed budget prefix of a small sort;
both query charge stages, overflow, empty zero-charge and Cell noncommit;
independent inventories; and actual authenticated fixture provenance/refusals.
The 552-row/8,192-query test is a synthetic bounded-work example, not an observed
row census or measured speedup. Helpers with fabricated inventories do not mint
source authority. Root must run the full prior suites, these nine tests and all
unchanged atomic, unsafe, matrix and attention controls before a guarded retry.

The survey found other linear queries, deliberately left unchanged:
`usage` scans use blocks plus exact address/access/atomic semantics and checks
agreement among origins; its use-block order could support a separately
reviewed range query. `authorizes_root` instead filters origin guard access
blocks, and `authorizes_bounds` filters full guard equality, neither matching
the new borrow ordering. `atomic_only` searches allocation origins. There is
no shared new index or generic helper that silently changes these contracts.
