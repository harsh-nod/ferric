# Memory-Bounds DAG Test Plan

These nine Pliron unit tests are authored source only. No project import,
compiler, test, SSH, or GPU invocation was performed by the test author.
The parent agent owns build, complete-suite qualification, and actual lowering.

New source:
`crates/fe2o3-pliron/src/production_analysis/pliron_ranked_bounds/dag_schedule_v1_tests.rs`.
The parent includes it as the test-only child module `dag_schedule_v1_tests`.
Its exact common prefix is
`production_analysis::pliron_ranked_bounds::dag_schedule_v1_tests::`.

## Closed Cohort

1. `dag_schedule_preserves_previously_admitted_path`: actual parsed/captured
   small input retains the exact old preflight bound and no certificate,
   including an ownership-contract census; direct scheduled and old runtime
   reports agree.
2. `dag_schedule_differential_dags_and_physical_orders`: all eligible reachable
   four-node DAGs with at most two outgoing edges, across all six non-entry
   physical orders, give 78 authored cases. Production bitset intersections
   in certificate order are compared with an independent descending
   `BTreeSet` fixed-point solver. Actual old/scheduled reports also agree.
3. `dag_schedule_duplicate_edges_and_entry`: true/false parallel edges retain
   both predecessor occurrences, so a fact present only on the true edge is
   not invented at the join. Entry remains empty. A separate unreachable
   predecessor into a reachable access retains the old unreachable finding
   without destroying the existing access proof.
4. `dag_schedule_cycles_and_unreachable_fallback`: reachable cycle, entry
   backedge, and disconnected self-cycle refuse a certificate; previously
   admitted inputs retain their old bound. A dense cyclic graph still returns
   the original work-hard-limit refusal rather than receiving the DAG bound.
5. `dag_schedule_forwarded_arguments_preserve_transport`: six existing real
   textual fixtures exercise direct, diamond, parallel, split, forwarded-view,
   and asymmetric-view transport; complete old and scheduled reports match.
6. `dag_schedule_certificate_mismatch_fails_closed`: eight independent
   certificate mutations cover function identity, block roster, successor
   order, duplicated/missing/out-of-range/permuted schedule entries, and
   malformed successor index. Runtime returns structural refusal, not FIFO
   fallback. Cross-function use, wrong census rows, and a foreign inventory
   are also refused.
7. `dag_schedule_work_storage_boundaries`: actual certificate validation is
   measured inside the test, then exercised at exact and one-short work and
   storage budgets. Checked size-product overflow, impossible vector capacity,
   and `usize::MAX` work are refused; overflow does not alter the existing
   counter.
8. `dag_schedule_preserves_bounds_refusals`: wrong guard edge, same-target
   true/false edges, bypassed guard, and different accessed index all preserve
   the actual `UnprovedBound` findings in old and scheduled execution.
9. `dag_schedule_dense_guard_production_route`: a constructed real Pliron
   fixture has the observed structural census of 567 blocks, 2240 operations,
   1122 raw successors and 552 distinct guards. Forward and reverse physical
   orders both require the actual preflight rescue and the production
   `require_pliron_ranked_bounds_with_schedule_v1` entry. Exact/one-short
   admission limits are tested on this path. A flipped first guard remains
   an actual `UnprovedBound` rejection after successful DAG admission.

The dense fixture models 548 atomic reads and four atomic writes, but it is
not a byte-for-byte replay of the candidate, its two views, memory-order
attributes, kernel source predicates, or payload values. Padding constants
and a short equality-branch prefix deliberately reproduce the measured
structural census. No actual post-optimization success is inferred from it.

The ownership-contract exclusion is tested by requiring an otherwise eligible
dense census with `ownership_contracts=1` to retain the original work-hard-limit
refusal. This preserves the old nested ownership analysis path; propagating a
certificate into that separate analysis is outside this proposal.

## Scope and Remaining Gates

Small unit cases call the private certificate collector directly to exercise
algorithm behavior; they do not claim those already-admitted inputs select the
new production route. The dense cases use the exact public(crate) preflight
and required runtime entry points. All nine methods and existing methods must
run in the complete Pliron suite, followed by the existing compiler/extraction
qualification and actual guarded compilation.

Allocation is fallible in production. These fixtures cover logical storage
denial and partial-certificate refusal, not an induced system allocator OOM.
No new allocator hook or production fault-injection API is added. Existing
work, storage, edge and fact ceilings are asserted unchanged; the new schedule
does not grant refinement, artifact, launch, or device authority.
