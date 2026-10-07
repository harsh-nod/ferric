# Scoped Warm-Layer Currentness

This is an opt-in engineering optimization under development, not a qualified
GPU route. It moves full topology discovery to entry and exit of a closed
warm-layer call while retaining participant-local queue, signal, generation
and ownership checks inside that call. The intended call includes Prefix,
guarded MLP, hidden readbacks and final validation before publishing completion.
Default paths remain unchanged.

This changes the timing of topology observations. A non-generation fact that
changes and reverts inside the call may escape detection; the proposal
explicitly tests and documents that limitation. It does not claim temporal
equivalence to full discovery at every internal checkpoint.

## First Coupled CPU Attempt

The original run on `mi350` failed after 96.067 seconds, at phase 22 of 26.
All 21 preceding phases completed naturally, including 1,143 runtime passes
with eight existing ignores, nine focused runtime scopes and ten selected
facade doctests. The real worker test build then failed with Rust E0433:
the new caller used `crate::native_forward`, but the existing module is
registered as `crate::native_catalog::forward`.

The failed compiler exited with status 101 and was fully reaped. Original
source, tool, private-cache, dependency and product postchecks passed. Worker
tests did not execute, and this is not a passing coupled qualification. No
canonical runtime/worker source or GPU route was enabled by this attempt.
The corrected module path requires a fresh complete qualification.

`cpu-attempt-v1` retains all 185 original archive members, including 117 raw
bodies, original compiler diagnostics, actual formatted proposal postimages
and the failed terminal. No failed flag or outcome has been rewritten.

| Artifact | SHA-256 |
| --- | --- |
| Coupled failed terminal | `5021149c2139723bc1c2de294ce26e4ee1df14f13fe2f78abcf3ec8ab2b7e44e` |
| Coupled failed archive | `c7b44e5f76218fcabea48407fc56b2b23ba9b4e840453c06f635adf9b7c75bac` |

## Second Coupled CPU Attempt

The fresh V2 run compiled the repaired worker library but failed at the same
test-build phase after 109.393 seconds. Two function-pointer declarations in
the new CLI test used a borrowed slice type whose higher-ranked lifetime did
not match the generic reader function. Rust reported E0308. The test-only
repair uses a concrete owned reader; runtime behavior is unchanged.

The preceding 21 phases again completed naturally, including all 1,143 runtime
passes, the eight existing ignores, focused scopes and ten facade doctests.
The worker test compiler exited naturally with status 101, was fully reaped,
and all postchecks passed. Worker tests did not execute. This remains a failed
qualification, not permission to integrate or launch the scoped GPU route.

`cpu-attempt-v2` retains all 186 original archive members, including the
unmodified failed terminal and compiler diagnostics. The next complete run
must preserve both original proposals and their separate repair lineage.

| Artifact | SHA-256 |
| --- | --- |
| V2 coupled failed terminal | `da226a70faa41176cf98df9ab6f3fd2e9bc5ab3f06c38ee522d142cda5b54ceb` |
| V2 coupled failed archive | `447fa3fdb95351f22977512dd88414c678a58ae83dc10d75fbce9d14b12c1f1d` |

## Passing Coupled CPU Retry

V3 passed all 26 supervised phases on `mi350` in 151.522 seconds. The full
runtime suite passed 1,143 tests with eight existing ignores; the full worker
suite passed 726 with four existing ignores. Nine focused runtime scopes, ten
facade doctests and eight doctest-parser checks also passed. All phases exited
naturally, with their owned processes reaped and no postcheck errors.

The original result records eleven executable products and verifies that the
real worker executable is unchanged across its CLI tests. The retained
`cpu-attempt-v3` capsule has 207 original members, including 137 raw bodies,
both repair histories and all 35 actual formatted source postimages. Both
earlier failed attempts remain separate and unchanged.

This qualifies the coupled CPU build and tests, not a scoped GPU execution.
A fresh same-binary native comparison remains required.

The 22 runtime and 13 worker changes are integrated from those exact tested
postimages. The [integration record](integration-v1/integration.json) and
[read-only postcheck](integration-v1/postcheck.json) verify all 816 nonfixture
runtime bodies, 209 worker bodies and both original Cargo identities. Reduced
qualification workspace fixtures were not copied into the repositories.

| Artifact | SHA-256 |
| --- | --- |
| Passing V3 terminal | `335cf93cc109390f3b7590f7dbe35ed852adca223a042894dcc18418a738ffbd` |
| Passing V3 archive | `8c0160d5017c72e981d78f54cb66a68fcdad63e261d5c261460aa2ffb9e77b0f` |
| Qualified worker executable | `f6a788dbb297b68a2dddc255d7faf49165de9ee8fa4eccc062bfe8b65a6d360a` |

## Parent Qualification

The separate parent V3 run passed all 65 supervised phases on `mi350` in
411.247 seconds. All 492 selected tests across 55 scopes passed, preserving
the 478 prior selected outcomes and adding 14. The library inventory has 961
names; this is not a full parent-library-suite claim. Seven executable
products and all 209 qualified worker source bodies passed their postchecks.

`parent-cpu-attempt-v3` retains all 483 original archive members, including
329 raw bodies and 121 lineage bodies. All seven tested parent changes are
integrated. The [parent integration postcheck](parent-integration-v1/postcheck.json)
verifies the complete 1,263-file Ferric source composition, not just the changed
files. No source, dependency or native authority is inferred from filenames.

| Artifact | SHA-256 |
| --- | --- |
| Parent V3 terminal | `6ca1454eb809ac2a4fa0eae74417ab1316032b00d769b85b30b5e386c0d27171` |
| Parent V3 archive | `9e8cf83ca988e2a9805eba10e576fe8ff63b467eb5a19bb7cba98d77ea95d502` |
| Qualified readiness executable | `1ef5d720029bd4d6dfc9f5ff9278b4fee7627240ee157ce7be91b7f44743f779` |

## Synthetic Data Checker

The first checker attempt recorded 69 successful tests and one fixture error:
its frame-mutation helper looked up an old path after the scoped fixture had
been moved. The original admitted census remains null, not 69 or 70. Its
natural exit-1 result, source map and raw stderr are retained in `checker-cpu-v1`.

The fresh V2 attempt passed all 70 tests on `mi350` in 18.051 seconds, with
one naturally retired GPU-hidden child and clean source/tool postchecks. The
repair reads the frame path from the actual fixture, updates its pin and byte
accounting, and requires the specific chained-completion refusal. All validators,
other 69 tests, test names and resource limits are unchanged. Tests cover
original policy bytes, strict counters and identities, all 40 records, four
complete payloads, matched executable identities and 124-span timing.

`checker-cpu-v2` retains all 25 original archive members. These are synthetic
tests, not a native execution, numerical reference or performance result.

| Artifact | SHA-256 |
| --- | --- |
| Original failed checker terminal | `80688a854ca776ad4df7692e3235e6009725ae319c51dff5777fb01cd04434af` |
| Original failed checker archive | `d33c3e32e80b7e3be5a48974e65189871caefc1cb4a22c4fe503da302ae9ed7e` |
| Passing V2 checker terminal | `61b595c1ebaa70337e7e6306fbf3ab89b4b11bcf6c63a116873ff58f61e18049` |
| Passing V2 checker archive | `25f5d492e5fbe23f976c31c6e913a3d3d361c65ffc64cebf4a23376e86d9602a` |

## Timing Report Qualification

The exact bound reporting code passed all seven synthetic tests on `mi350`.
[Original evidence](matched-timing-report-cpu-v1/README.md) includes integer
nanosecond and decimal-rounding checks, timeline consistency, claim rejection,
and CSV/table/SVG rendering. This is a report-tool qualification only; the
actual same-binary GPU pair and its measurements are separate gates.

## Remaining Gates

A fresh same-binary Default/scoped GPU pair must pass before claiming this
optimization runs end to end. Independent numerical
acceptance, the full 2,048/256 workload
and the 700 tokens/s target remain open. No issue #42 milestone is closed by
these results.
