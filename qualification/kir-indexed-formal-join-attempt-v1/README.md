# Indexed Formal Join Attempt

The indexed inert-layout join compiled on ASROCK. All twenty new regression
tests passed inside the full lower-library suite, but the complete suite did
not pass: 784 tests passed and one existing nested-enum test failed, with zero
ignored tests. Qualification stopped there. The separately repeated indexed
subset, finalizer build/tests and actual retained handoff join were not run.

## Implementation

The four-file candidate replaces repeated block and value scans in the inert
formal join with borrowed, deterministically sorted indices. Binary lookup
preserves the first definition's original traversal order for duplicate value
IDs. Duplicate blocks still reject; evidence records remain in their original
order. The live producer-side join and canonical encodings are unchanged.

Construction, sorting, lookups and temporary storage are explicitly charged.
The work/storage limits remain `1 << 30` / 128 MiB; the previous 128 work
multiplier is not simply discounted. The tests cover legacy differential
behavior, sparse IDs, duplicate definitions, malformed evidence, exact and
one-under resource limits, storage restoration and immutable identities.
No reduction in actual full-handoff work or GPU time has yet been measured.

## Observed Failure

The [full test log](raw/lower-tests-stdout) reports a location assertion in
`retained_nested_enum_referent_scalar_move_invalidates_saved_references`:

| Diagnostic Location | Block | Statement | Local |
| --- | ---: | ---: | ---: |
| Observed | 8 | 7 | 5 |
| Expected by the existing test | 7 | 4 | 5 |

Source review finds two invalid reads after the same scalar-field move.
The RPO scheduler visits block 8 first. The assertion occurs during semantic
SSA validation, before the indexed formal join. This suggests a stale
first-error expectation, but unchanged-source control has not yet established
the cause. The panic also prevents the test's subsequent saved-reference
subcase from executing; that coverage must be restored, not skipped.

The test leaf exited naturally with code 101 and the owner with code 1.
All owned processes were reaped and source/input postchecks passed.
The [failed attempt](attempt/failed.json), [failed owner](owner/failed.json),
actually formatted Rust sources, proposal, controller and raw phase bodies
are retained with the [publication ledger](result.json). Sixteen controller
tests passed separately, recorded as a [primary-agent SSH observation](pure/root-observation.json),
not a remote supervisor receipt. Large snapshots, inventories and the test
ELF are remotely rehashed and hash-linked rather than copied into Git.

Next: run the focused test and full library suite on unchanged RPO source,
resolve the discrepancy without suppressing tests, then repeat the full
candidate qualification and actual handoff join. This attempt qualifies no
new compiler, image or GPU execution. Numerical acceptance, all issue #42
milestones and the sustained single-request BF16 2,048/256 target remain open.
