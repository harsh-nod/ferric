# Indexed Inert Formal Join V2 Source

Frozen source-only successor after the actual unchanged-RPO control reproduced the existing assertion failure. No V2 tests, imports, builds, remote execution or live-tree edits have been performed.

## Exact Delta

The four indexed source bodies and all twenty added tests are byte-identical to the frozen V1 proposal `6501750a...`. Their algorithm, metering, caps, live-join behavior and authority remain unchanged. The retained algorithm/accounting description is in [V1 README](../p228-kir-indexed-formal-join-v1/README.md); its original unmeasured status describes that historical proposal, not the later diagnostic.

A fifth overlay changes exactly one existing test expectation and its adjacent comment in `retained_nested_enum_transport_v1_tests.rs`: `(7, Some(4), ROUND)` becomes `(8, Some(7), ROUND)`. The exact function, local and `MaybeMovedValueUsed` checks in the shared helper are unchanged. The second alias-only retained-reference proof-refusal subcase is unchanged.

The original test body SHA `4235c5d9bece86dc631f4dfbc081a3eb233539628f1af4db7b0ca0db7ede16be` matches the actual RPO source snapshot `77fbdd63...`. In that fixture, block 5 moves `ROUND.field1`; both branch 7 statement 4 and branch 8 statement 7 later reload it. The authenticated planner and RPO scheduler visit block 8 before 7. The test's first call only constructs a semantic SSA owner and cannot reach the new indexed formal join.

The actual unchanged-source baseline `a95ee1e3fb32cd50745a0bba5274e5688b72e7ee3f26de25ddbe0cd936e667ee` (244,380 bytes) independently reproduced `(8, Some(7), 5)` versus the old expected `(7, Some(4), 5)` in both the focused test and the full library suite. Its owner `87ae6c49d7ec72a35d6e13339cc501323d10d5e78ce212c77b336ff8a675012c` (19,845 bytes) completed naturally. The six-phase control retained 39 raw files, with no source overlay, formatting or postcheck drift. The focused test recorded one failure; the full unchanged 765-test suite recorded 764 passes, that same single failure and no ignored tests. This establishes that the assertion predates the indexed overlay, not that V2 or the original library is qualified.

## Recorded Outcomes and Limits

The actual diagnostic `71ea4664...` identified the original inert-layout bulk charge: accepted work 40,799,816, pending charge 1,044,025,344, attempted total 1,084,825,160 against the unchanged 1,073,741,824 limit. The indexed V1 attempt `88d0fa37...` then built and ran the lower-library suite: 784 passed and one existing assertion failed; it did not run the separate indexed subset, finalizer stages or actual inert join.

V2 keeps twenty added test names and adds no new test method. The source roster is five overlay bodies, three replacements and two additions, against the same qualified 5,783-file RPO snapshot (5,785 candidate files). No Cargo metadata, compiler checks, work/storage limits, certificates, canonical bytes or admission flags change.

Root must run all 785 candidate library names without suppressing the corrected test, and only then run the explicit twenty-test subset repeat, unchanged finalizer cohort and retained actual join. Passing the source tests alone is not compiler qualification, checked emission, HSACO, GPU or numerical acceptance.
