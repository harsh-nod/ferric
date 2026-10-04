# Indexed Inert Formal Join V2

The indexed candidate passed all thirteen staged CPU phases on ASROCK,
including the actual retained RoPE/RPO handoff that previously exceeded the
canonical KIR work budget. The work limit remains `1 << 30`; the storage
limit remains 128 MiB. No HSACO or GPU result follows from this checkpoint.

| Check | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Full lower-library suite | 785 | 0 | 0 |
| Indexed subset, repeated from that suite | 20 | 0 | 0 |
| Default finalizer suite | 190 | 0 | 15 |
| Explicit actual retained-handoff join | 1 | 0 | 0 |
| Controller-policy tests | 20 | 0 | 0 |

The twenty indexed tests are part of the 785-test suite, not twenty additional
unique tests. The finalizer's fifteen historical actual-capture tests remain
ignored in its default run; the relevant retained-handoff test was then run
explicitly and passed. All thirteen leaves exited naturally, all owned
processes were reaped, and source/input postchecks passed.

## What Changed

The inert formal join builds borrowed block and value tables once, sorts
them by `(id, original ordinal)` using a controlled heapsort, then uses
binary searches for each lookup. Sort, lookup and allocation costs remain
metered. The original ordinal preserves first-definition behavior for duplicate
value IDs; duplicate block IDs are rejected. Live-join behavior, canonical
bytes and admission authority are unchanged.

This removes repeated whole-function scans. The earlier
[diagnostic](../kir-join-work-diagnostic-v1/README.md) measured a pending
1,044,025,344-unit charge after 40,799,816 accepted units, exceeding the
1,073,741,824-unit limit. The new candidate passes the same actual handoff
under that limit. This is not a measured GPU speedup, nor a measured exact
work-reduction ratio; the passing run does not report a new accounting total.

The [unchanged-source control](../kir-indexed-baseline-control-v1/README.md)
reproduced the existing RPO diagnostic-location assertion before the indexed
overlay. V2 corrects only that test's first-error location from block 7,
statement 4 to block 8, statement 7. Both branches contain invalid reads;
RPO visits block 8 first. The subsequent alias-only rejection subcase remains
unchanged and now executes successfully as part of the passing full suite.

## Evidence And Next Gate

The [publication ledger](result.json) binds the actual CPU and owner receipts,
the five proposed and five formatted Rust bodies, the controller, and all
sixty-five phase evidence files. Ten source/dependency snapshots, two ownership
inventories and two test executables are retained by hash rather than copied
into Git. The separate controller-test observation is a primary-agent command
record, not an invented remote supervisor receipt. Archived proposal documents
retain their original pre-test status; this page reports the subsequent run.

Next is checked HSACO emission using the qualified indexed finalizer and the
previously checked compiler handoff, followed by fresh GPU numerical evaluation.
The full compiler cohort was not rebuilt here. Production admission, full-model
numerical acceptance, sustained BF16 2,048/256 decode and 700 tokens/s remain
open, as do all issue #42 milestones.
