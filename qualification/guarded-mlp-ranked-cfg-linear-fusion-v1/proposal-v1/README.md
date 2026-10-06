# Ranked CFG Linear Fusion V1

Source-only proposal against the actually qualified CFG-compaction generation.
No build, Rust test, project import, GPU execution, or remote command was run by
the author. The new implementation, its nine tests, and the guarded workload
remain unqualified at freeze. Root owns assembly, independent review, complete
CPU qualification, loader inspection, and the next guarded lowering attempt.

## Evidence And Scope

The direct CPU baseline is receipt `439d4b5b`, source map `c00481ef`, input
`eee83b9d`, and controller `f5dc942c`. The actual guarded failure is receipt
`1bf26b28` and stderr `9a309b8f`, retained in
`qualification/guarded-mlp-ranked-cfg-compaction-lowering-v1/attempt-v1`.
Full compact pins are in `source-manifest.json`.

That stderr contains one closed ranked function with 1,675 blocks and 2,230 raw
successor occurrences. PRESERVE-002's count 1,025 is the first identity-limit
refusal at the independent 1,024-block ceiling, not the total graph size. There
was no downstream edge verdict or HSACO. These are baseline observations, not
results from this proposal.

A data-only inventory of that rendered graph finds 1,108 eligible original
arcs: 556 empty-source and 552 atomic-source unconditional branches. Fusion
alone, excluding entry as both source and target, predicts 567 blocks and
1,122 raw edges. This is arithmetic over authenticated text, not execution of
the Rust pass or proof of eventual compiler admission. It supersedes the need
for a separate branch-threading stage proposed in the earlier investigation.

## Production Change

The four-row overlay contains two replacements and two additions:

- `production_ranked_projection_v1.rs`: private module/test inclusion and one
  hook immediately after CFG construction, before reference writes, access
  correspondence, wave compilation, or generated-effect validation consume
  ranked coordinates.
- `production/ranked.rs` in `fe2o3-pliron`: a consuming `into_parts` accessor
  moves block payloads without cloning nested data. Constructors, validation,
  visibility of fields, and safety checks are unchanged.
- `production_ranked_projection_v1/cfg_linear_fusion_v1.rs`: the bounded pass.
- `production_ranked_projection_v1/cfg_linear_fusion_v1_tests.rs`: nine new
  compiler tests. No existing test is redirected to a legacy bypass.

The production hook attempts fusion only above the existing exported identity
block ceiling, currently 1,024. Smaller graphs retain their original owners
and layout without borrowing an additional budget. Small edge-heavy graphs
are deliberately outside this narrow change. Direct component tests exercise
the helper on small graphs; the full-route test uses genuine materialized
semantic owners on both sides of the production threshold.

Eligibility is a plain unconditional branch from a non-entry block to a
different non-entry block having exactly one raw predecessor occurrence.
Parallel edges count separately. A fixed predecessor census defines disjoint
chains; no repeated whole-graph search, reachability deletion, predicate
simplification, or separate branch-threading pass is introduced. All declared
blocks participate, including unreachable ones. Entry stays block zero and
its operations and terminator kind remain intact, with any target remapped.

The replacement preserves operation order along every chain, all predicate
operands and branch choices, both atomic orderings for CAS, scopes, accesses,
barriers, and terminal outcomes. Only eliminated unconditional branch
terminators disappear. SSA local and function-argument IDs are unchanged.
Access-source records retain their order, semantic site, and provenance;
generated-effect records retain semantic block/ordinal, origin, and recipe
identity. Their ranked block/operation coordinates and wave-sync coordinates
receive the exact chain-offset mapping. Duplicate analysis-split successors
are never deduplicated.

Index-argument headers, argument-bearing terminators, encountered valid
block-argument operands, and unfamiliar operation recipes cause whole-function
fallback. A pure eligible unconditional cycle likewise retains the original
function, including unreachable cycles. Supported value operands, every raw
terminator target, and every supplied ranked source/effect/wave coordinate are
checked before fallback. Unsupported nested recipes are left unchanged for
their existing validator; fallback is not new admission authority. Conditional
cycles may be fused without deleting their backedge or its predicate.

## Resources And Failure

No identity, ranked-block, raw-edge, operation, fact, graph-work, canonical-work,
or storage ceiling is increased. In particular, identity remains 1,024, ranked
blocks/edges remain 2,048, and projection graph work remains 3,145,728. The pass
borrows the real canonical assertion session's existing resource ledger; a
synthetic assertion object cannot supply a substitute production budget.

Input planning does not prematurely apply the downstream 2,048-edge output
limit: the measured input already has 2,230 edges. The closed terminator enum
has at most two raw successors per block, so the existing 2,048-block input
bound also bounds this scan. Existing output validators still enforce their
unchanged limits and semantic obligations.

All size/count arithmetic is checked. One per-block plan stores predecessor,
chain, and coordinate information. Vector reservations are fallible. The
lexical extra logical-storage charge is

```
V * size_of::<Plan>()
+ V * size_of::<ProductionRankedBlockV1>()
+ O * size_of::<ProductionRankedOperationV1>()
```

where `V` and `O` are input block and operation counts. The charge conservatively
covers coexisting old operation buffers and new moved-operation buffers;
nested operation payloads are moved, not cloned. The pass adds compiler
scratch, not a new memory ceiling, and this formula is not an RSS claim.
Caller-owned inputs and metadata vectors keep their existing ownership.

Work is charged before each block, raw edge, operation, operand, source record,
chain step, operation move, target remap, or coordinate update. Planning and
assembly are linear in those bounded inventories. Charges are incremental;
failure preserves the accepted ledger prefix. The pass owns all four output
collections until success, so partial construction/remapping cannot escape.
Scratch is dropped or explicitly transferred to the returned graph before
restoring the caller's storage floor; the ledger retains its peak.

Whole-function fallback can still incur charged analysis and scratch. Thus
resource admission may change even when layout is retained. Successful fusion
also changes structural resource acceptance, while all semantic predicates and
all hard ceilings remain fixed. This is not diagnostic-only and is not an
alias, provenance, bounds, atomic, source-safety, or callback bypass.

## Authored Verification

The manifest lists exactly nine names under
`production_ranked_projection_v1::tests::cfg_linear_fusion_`:

1. Conditional traces and trap-before-access behavior.
2. Access/source/wave/generated identity and CAS-field remapping.
3. Raw duplicate edges and preserved entry.
4. Cycles, argument references, and unsupported-form fallback.
5. Malformed target, operand, and metadata coordinates.
6. Every prefix below exact work cost, exact storage boundary, overflow, and
   restored storage floors without partial output.
7. Chains and 1,024/1,025/2,048/2,049/zero block boundaries.
8. A synthetic 552-guard family: all-success plus every exact failing predicate,
   548 Acquire reads, three Relaxed writes, and one Release write. Its constructed
   fixture has authored input counts 1,659 blocks/2,209 edges and expected output
   555/1,105; it is explicitly not the actual guarded graph replay.
9. Full production projection from genuine semantic owners at 1,024 and 1,025
   pre-fusion ranked blocks, using the real canonical budget path.

The untransformed graph is the differential trace control, not a production
mode or optional-cap path. Full CPU suites and extraction controls, exact
artifact lineage, loader inspection, and actual guarded lowering are still
required. Predicted block/edge counts alone do not establish later identity,
memory safety, race, alias, target-code, or GPU correctness.
