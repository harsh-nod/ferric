# Unchanged RPO Baseline Control

The original RPO source reproduces the existing nested-enum assertion that
stopped the [indexed-join attempt](../kir-indexed-formal-join-attempt-v1/README.md).
This establishes that the failure exists without the indexed overlay. It
does not turn the failing library into a passing qualification.

| Run | Passed | Failed | Ignored | Natural Exit |
| --- | ---: | ---: | ---: | ---: |
| Exact original test | 0 | 1 | 0 | 101 |
| Full original library | 764 | 1 | 0 | 101 |

Both [focused](raw/lower-focused-test-stdout) and
[full-suite](raw/lower-tests-stdout) logs report observed location
`(8, Some(7), 5)` versus the existing expectation `(7, Some(4), 5)` in
`retained_nested_enum_referent_scalar_move_invalidates_saved_references`.
The full inventory is exactly the prior candidate's 785 names minus its
twenty additions. No test was ignored or suppressed.

The control copied all 5,783 original source files without an overlay or
formatter. It rebuilt the library in a fresh target and executed six bounded
phases. Source, dependency, configuration and protected-product postchecks
passed. The observer exited naturally with all owned processes reaped.
Its `passed=true` records completion of the observation, not library success.

Source review identifies two invalid reads and an RPO traversal that visits
block 8 before block 7. The measured control supports correcting the old
first-error location expectation. It does not independently prove the
scheduler's soundness. The later alias-only rejection subcase has not yet
run because the first assertion panics; the candidate rerun must restore
that coverage without deleting or weakening the second check.

The [publication ledger](result.json) links actual receipts, all thirty phase
bodies and the original controller. Nine snapshots, two ownership inventories
and the test ELF were rehashed remotely and are hash-linked, not copied into
Git. The separate [16-test controller observation](pure/root-observation.json)
retains a primary-agent summary and source hashes, not a full test transcript
or a remote supervisor receipt.

No actual inert join, HSACO emission or GPU execution occurred in this control.
All issue #42 milestones and full-model numerical/performance acceptance remain
open. Next is the five-file candidate rerun with the single existing test
expectation corrected and the original finalizer and handoff gates retained.
