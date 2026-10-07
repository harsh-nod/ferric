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

## Remaining Gates

The corrected coupled runtime/worker build, parent qualification and fresh
same-binary Default/scoped GPU pair must pass before claiming this optimization
runs end to end. Independent numerical acceptance, the full 2,048/256 workload
and the 700 tokens/s target remain open. No issue #42 milestone is closed by
these results.
