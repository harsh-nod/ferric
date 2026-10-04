# Partial-Move RPO Scheduler Candidate V2

Untested source proposal. No formatting, imports, compilation, tests, checked
lowering or remote execution were performed by the author. The actual RoPE V1
and V2 failures remain failures; this proposal has not solved either one.

## Test Fixture Correction

The preserved V1 CPU attempt reached Rust test compilation and failed with
E0308: `SsaEdgeInputV1::new` takes `SsaEdgeRoleV1`, whereas
`semantic_edge_role_v1` returns `u16`. No Rust test ran and this is not
a production scheduler or RoPE-lowering outcome. The actual failed completion
is retained locally as `L/failed.json`, SHA
`597712260e281fbaa904513ae64636bb906c9d50678f19651aa467cf001d123b`.

V2 changes only that fixture call to
`SsaEdgeRoleV1::new(semantic_edge_role_v1(edge.role()))`, matching the
qualified `projected_call_destination_v1_tests.rs` construction at line 933.
All production bodies, FIFO oracle, seventeen test names, transfer/join rules,
storage/work accounting and caps are byte-identical to V1. This successor
has not been formatted, compiled or tested.

## Exact Base

The base is the retained, qualified ordinary-induction compiler generation at
`L/ordinary-induction-compiler-cpu-v228-v1/source/fe2o3`, where
`L=/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216`.
Its completion SHA is
`c45aa83b01bf76fdfd9962b06884611658dab607b001ba67fecc66db9fd5ba97`.
The matching before/after source maps have SHA
`5aad4c8ba6520c323310424ce343de2ae5088e7f8c1c0b1fd67853b5fd3bd9f9`.

Do not apply to `/home/harsh/fe2o3-p228-runtime` as though it were this generation.
Its partial-move file differs: the retained source contains qualified indirect
pointer-carrier checks and the whole-local idempotent-mark fix missing there.
Both fixes, their existing tests, and the ordinary-induction change are retained.
The manifest records the exact two preimages and all four proposed Rust bodies.

## Change

Only `semantic_ssa.rs` and `semantic_ssa/partial_moves.rs` change production code.
The existing certified `SsaConstructionPlanV1::reverse_postorder()` supplies the
priority order. A dense inverse rank map, the existing queued bitmap and a
minimum-rank cursor replace the FIFO queue. Every first incoming edge still
schedules its target, even with an empty state. Changed incoming states still
reschedule targets, including lower-priority-number backedges. Duplicate pending
targets are collapsed, and each pop clears its queued bit before transfer.

All statement/terminator validation, call-result edge handling, reachability,
state cloning, merge rules and mark rules remain byte-identical. No local/path
is pruned. No new availability or liveness analysis is introduced. The earlier
availability-demand proposal is deferred; RPO is the smaller first experiment.

For a DAG, RPO is topological, so a join can collect all predecessor states before
running its transfer. FIFO can run a short-path join and its successors before
a longer path arrives, charging repeated local reinitialization/tombstones.
Cycles still need monotone fixed-point iteration. This argument predicts a
possible reduction in repeated work; no claim is made about the actual failed
RoPE graph, its dominant charges, or an observed runtime improvement.

## Bounds And Replay

The existing 2,097,152-word and 67,108,864-work-unit hard caps are unchanged.
For a function containing a projected local move, auxiliary storage adds exactly
one word per block for the inverse rank map and six fixed words for its Vec
header (3), borrowed RPO slice (2) and cursor (1), with checked arithmetic. The old
reservation is retained without credit for the removed FIFO. The unchanged
caller enforces auxiliary and combined planner limits before queue allocation.

Initialization charges one work unit per block for the inverse map/bitmap, then
one per reachable RPO entry. Each scheduling attempt and each scanned RPO slot
charges through the existing checked work budget before modifying scheduler
state. Scanning empty slots is charged too. There is no library heap/tree with
unaccounted internal comparisons. A pop can scan up to the reachable-block
count, so pathological cyclic scans can still hit the unchanged work cap.

`state_entries` retains cumulative insertion semantics, including no refunds
after StorageLive or assignment. It is not reinterpreted as peak memory.
Transfer/join charges are unchanged. Resource summaries and identities can
legitimately differ because work and storage really differ; the same canonical
replay must recompute them. No old certificate is reusable for a new compiler
generation. Multiple-error inputs may report a different first violation due
to order; single-error differential fixtures require exact error equality.
Admission at a tight resource boundary may change in either direction, since
the new scheduler has both a real overhead and potential savings.

## Authored Tests

`partial_move_rpo_v1_tests.rs` declares 17 tests. The separate test-only
`partial_move_fifo_oracle_v1.rs` is the exact retained 121-line validator loop,
with only its function name changed. It uses unchanged production transfer/join
helpers. It is not compiled into production and provides no bypass.

Tests cover deterministic priority, duplicate pending targets, backward
rescheduling, malformed priority maps, initialization/push/skipped-slot work,
work overflow, the additional storage reservation, zero-projected-move behavior,
an asymmetric diamond with repeated tombstones, exact storage/work boundaries,
single-error join rejection, loops/irreducible CFGs, parallel/unreachable edges,
whole-tombstone widening and StorageLive, union/missing/unsupported projections,
indirect pointer-carrier writes, and normal-versus-unwind call initialization.
The final test checks two real canonical function-plan reconstructions, not
`ProductionSemanticSsaOwnerV1::verify_replay()`.

Most differential fixtures intentionally isolate the partial-move validator:
the real planner computes the exact CFG's reachability/RPO with no promoted
variables. They do not claim complete typed semantic-owner admission. The
separate public-planner test uses the actual semantic adapter. The synthetic
diamond asserts savings only if executed successfully; no result is invented.

Keep the existing six idempotent/no-refund tests and all indirect-destination,
call-address and indexed-path tests unchanged. Qualification must also run the
existing real owner-replay and occurrence-capture suites, especially
`streaming_replay_checks_exact_retained_fields_not_only_stored_identities` and
the occurrence tests mutating projected_moves/state_entries/work_units. Those
are the actual replay/tamper coverage, not the reconstruction test above.

## Root Qualification

Apply `changes.patch` only to a fresh copy of the exact qualified generation,
after verifying preimages. Format the four copied Rust bodies there, retain the
formatted outputs, run focused tests plus the complete compiler/pliron/finalizer
cohorts and produce new compiler/backend/finalizer identities. Do not edit the
retained qualified source, RT, F, or any frozen proposal. Reuse the existing
bounded qualification controller and unchanged source/dependency/provider closure.

Only then attempt the unchanged nine-stage checked RoPE route with the new
compiler generation. If it still fails, retain the result and measure bounded
per-block visits/mark/merge contributions before selecting a further algorithm.
No guard waiver, image admission, GPU run, numerical acceptance or performance
claim follows from this source proposal.
