# Bounded Indexed-Atomic Borrow Lookup

This compiler experiment addresses the [measured address-permission scan](../guarded-mlp-indexed-atomic-use-lookup-lowering-v1/README.md).
It changes compiler analysis, not GPU arithmetic or launch geometry.

Borrow records are appended in atomic-use order, which does not establish
their definition-block order. The proposal explicitly orders only that private
inventory with an in-place heapsort, charging each sift visit, comparison and
swap before doing the work. No new heap allocation or work-limit increase is
introduced. An unsuccessful construction is discarded before publication.

Address queries then use two binary searches to find the requested block's
range and apply the original exact place-equality check within it. Both work
charges are staged before committing the query counter, preserving the old
failed-query behavior. Use records, provenance checks and other queries remain
unchanged; resource admission may change because charged analysis work changes.

For N records and B records in the selected block, the nonempty query bound is
`2*(floor(log2(N))+1) + B*(projection_length+1)`. The explicit construction
bound and its derivation are in the [proposal](proposal-v1/README.md).
The previous 1,656-unit diagnostic is a charged amount, not a measured record
count. These analytical bounds are not benchmark speedups.

Nine authored tests cover shuffled and duplicate blocks, 5,460 differential
queries over 364 small inventories, exact place components, boundary keys,
sort/query work limits and overflow, failed-query counter preservation,
independent inventories and actual authenticated fixture refusals. A synthetic
552-row repeated-query case does not claim that the real kernel has that
borrow inventory shape.

Manifest: `1a089d8bcee0b539e42b7870706da89c23ef60ed494f24e1c4a6ee2fa529d49b`.
Patch: `3bffd4e8fa956ae4051457cfd5053e1b4c3e8bba0f0cf465fe236267139b3dd2`.

This is source-only progress. Fresh full compiler qualification, binary-loader
inspection and guarded lowering remain separate gates. No new CPU, HSACO,
GPU, model numerical or performance outcome is claimed. All issue #42
milestones and the 700 tokens/s target remain open.
