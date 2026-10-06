# Guarded CFG Compaction Proposal

Source-only proposal, not a qualified compiler or successful guarded lowering.
No author imports, compiler runs, tests, SSH, or canonical edits were performed.
The root owns application, full CPU qualification, tool inspection, and lowering.

## Measured Baseline

The retained expansion-diagnostic lowering failure identifies
`ferric_qwen3_mlp_state_guard_v1`, semantic root/body 1, function identity
`72f186b0c49aafc62f29a7b77a5bbca4fc458c507019297c8bea173b272fe70c`.
It reports 2778 projected blocks against 2048, with 1121 semantic blocks.
The complete qualified CPU generation and actual failure/stderr byte pins are
recorded in `source-manifest.json`. They describe the baseline, not this proposal.

These counts do not identify the number of reachable guarded accesses, their
comparison counts, or the resulting raw edge inventory. No post-compaction block
count, successful alias check, device artifact, GPU result, or speedup is claimed.

## Exact Change

`production_ranked_projection_v1.rs` compacts only generated `Guarded(Trap)`
accesses with an empty live-induction set. It reserves one shared empty terminal
trap for eligible reachable sites and accumulates each successful access in its
continuation block instead of emitting an access-to-continuation branch.

The existing builder wrapper always selects compaction. Its implementation has
a legacy-layout argument only under `cfg(test)` for differential checks; no
production argument, environment switch, alternate cap, or admission bypass is
introduced. `ContinueWithoutAccess`, live-induction accesses, semantic traps,
and functions with no eligible sites keep their existing layout. Unreachable
guarded items neither reserve a trap nor create an access source.

All access predicates, atomic ordering/scope validation, exact access operation
construction, source provenance, generated-effect ordinal/recipe validation,
wave synchronization, and independent alias/callable/source checks remain.
Source/effect/wave records follow the actual block and operation positions after
fusion. The shared trap accepts no block arguments and has no outgoing edge.

The compact count is computed before block allocation and before the unchanged
2048-block gate. There is no oversized intermediate ranked CFG. Only scalar
layout state is added; no new scratch vector or analysis cache is introduced.
The existing reachable-block and live-induction inventories are reused.

## Independent Resource Gates

For `g > 0` eligible sites, irrespective of their individual comparison counts,
this layout has `2*g - 1` fewer blocks and `g` fewer raw successor edges than the
legacy layout. For zero eligible sites both inventories are identical. Sharing
the trap alone saves blocks, not incoming false edges. The bounds facts attached
to successful predicate edges remain required.

The block and edge limits remain 2048, graph work remains 3145728, and all fact,
operation, analysis-work, storage, and other ceilings are unchanged. Resource
admission may change because a smaller equivalent representation can fit these
limits. Passing the block gate does not imply passing the edge or later proof
gates. The focused edge fixture deliberately produces 2047, 2048, and 2049 raw
edges while all three compact graphs have fewer than 2048 blocks; it does not
assert that the last graph passes downstream ranked-bounds preflight.

## Source Roster and Tests

The three-row manifest is relative to the qualified diagnostic source map:

- Replace `crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1.rs`.
- Replace its `production_ranked_projection_v1/projection_04_tests.rs` child.
- Add its `production_ranked_projection_v1/cfg_compaction_v1_tests.rs` child.

`integration.patch` contains all three rows in sorted path order, with `/dev/null`
as the added file preimage. The full two test postimages accompany it; the parent
postimage is represented by the authenticated patch rather than another full tree.
There is exactly one added source file and nine new focused methods, whose exact
fully qualified names/filter are in the manifest. Every prior method is retained.

The new tests use the real builder with compact and test-only legacy layouts.
They cover 120 check/effect trace combinations (including load, store,
compare-exchange and non-atomic accesses), eight mixed failure-policy traces,
16 multi-block traces, source-site identities, following memory effects, barriers,
wave coordinates, generated ordinals/recipes, unreachable sites, zero eligible
sites, live induction, seven malformed atomic/predicate inputs and an invalid
generated ordinal. They also check the exact 2048/2049 projected-block boundary
and independent raw edge counts. These are authored assertions, not executed
results or a captured inventory of the actual guarded kernel.

Three inherited layout tests change only expected block counts, final branches,
and access/barrier co-location. Their existing actual PLIRON bounds/race checks,
semantic operation ordering, names, and refusal checks are preserved. No prior
test is redirected to the legacy layout to hide the production change.

The required execution gates are both compiler builds, the complete existing CPU
suite plus the exact new cohort, all prior extraction/negative controls, tool
inspection, and a fresh unchanged guarded compile. Any further edge/fact/alias
refusal must remain a refusal, not motivate a silent budget increase.
