# Readiness40 Parent Host Timing

This diagnostic partitions parent wall time for the existing Position5 request.
The standalone data validator and instrumented Rust parent passed their MI350
CPU qualifications. No native timing result is claimed yet.

## Measurement Scope

The explicit parent selector is
`--observe-guarded-readiness40-position5-host-timing`. The ordinary request,
worker selector and wire remain unchanged: 40 authentic prompt positions,
zero generated tokens, and full captures at positions 0, 5, 16 and 39. Kernel
images, currentness, readbacks, deadlines and numerical policy are unchanged.

A fixed 124-event recorder produces contiguous, nonoverlapping intervals:
source preparation; spawn through setup seal; three intervals per forward
(prepare/write, flush through frame read, validate/retain/commit); Close and
retirement; and postchecks through ordinary evidence publication. Exactly
40 ordered rows and every interval must reconcile with the total. Nested
coordinator/rank/kernel timers are not added to this timeline.

These are parent wall intervals, not GPU kernel durations or tokens/s.
Flush-to-frame-read includes pipe wait and framing. Request parsing before
the recorder and the timing wrapper's own validation, sidecar publication
and stdout emission are outside the timeline and are explicitly disclosed.

The ordinary `complete.json` is unchanged. A separate `host-timing.json`
binds it only after healthy Close, natural child retirement and the ordinary
retained-publication checks. The sidecar is capped at 64 KiB and fits inside
the unchanged 32 MiB aggregate limit with the original supervisor allowance.
Failed or incomplete execution cannot publish a successful timing result.

## Data Validator Result

The actual MI350 CPU run passed all 29 tests: 17 unchanged readiness checks
and 12 timing/parity checks, with zero failures, errors or skips. The single
owned phase exited naturally and was reaped, all sources/tools were unchanged,
and GPUs were hidden. Elapsed qualification time was 1.529 seconds, not a
kernel measurement. Fixtures are synthetic and do not constitute model output.

Coverage includes exact unsigned 64-bit arithmetic, all 124 intervals, row
ordering, gaps/overlap, malformed fields, retained-file identity, healthy Close,
unchanged retention limits, and same-side completion/payload comparison.

`checker-cpu-v1` retains all 15 original source/evidence bodies plus the executed
collector and original manifest; local retention adds a separate record.

| Artifact | SHA-256 |
| --- | --- |
| Checker terminal | `f655204d168408bd47ea09968d56fa3d8545f9647ce1085ba61ae52fe88f76b2` |
| Executed checker | `4b08c9b1ca9d3298e8d9dcac4331c4049aa0b203c4ebc50b98241080305d5a6b` |
| Timing validator | `27c652f5452676b4cc08a634241aff89b2653e85ee111133c1f1c4ab13efd93f` |
| Retained archive | `9a70a4797af857993d59091dba10c6452cea580e84b2d674cb15d1b66aaee732` |

## Rust Parent Result

The actual MI350 parent qualification passed 466 selected tests in 53 scopes
across 63 clean phases, with zero failures or ignored tests. It preserved all
455 predecessor outcomes and added ten timing-library tests and one explicit
CLI-selector test. The library inventory contains 938 names; this selected
run is not a claim that every parent-library test ran. All seven binaries built.

Qualification took 406.948 seconds with GPUs hidden, offline Cargo, CPUs 8/9,
two build jobs, opt-level 2, debug assertions and overflow checks enabled.
Source, dependency, private cache, tool and product postchecks passed.

`parent-cpu-v1` retains all 447 original capsule members. The four integrated
Rust files are the actual formatted, tested bodies; the complete 1,253-file
Ferric source map is recorded in `parent-source-integration.json`. All 203
worker files and seven previously qualified Full2303 parent files are unchanged.

| Artifact | SHA-256 |
| --- | --- |
| Parent terminal | `d7239641f355b945bf6baac1ba7e8e6a887cedd5c4857d30ac36c5b896e54d3f` |
| Tested source map | `1896bde4efb77b0da0635af6371c5877ce2c6f93b5bfbdafea1fd6e629b44e3f` |
| Readiness parent ELF | `2e21c44c6857ee1696ddac53eaebc66c3f920f075266d729a142c1f9567621eb` |
| Retained archive | `0ffdc7452a16db111737998925c53fb41c1ad7bc8241a6a08dfbc3cc67d8c9c8` |

## Remaining Gates

The owned GPU attempt must preserve all 40 semantic
completion records and all four original Position5 payloads. Same-side parity
is not independent model accuracy. A timing result can guide optimization;
it does not by itself establish Full2303 completion feasibility or throughput.
The requested BF16 target-only 2,048/256 acceptance, 700 tokens/s and M0-M7
remain open.
