# Ordered TP1 Residual Tail

Experiment based on Ferric `0d1a90d9b0940552a6d87b699353f5be6b71bb80`.
Source `2396a82` passes the complete source-fresh 31-step CPU gate and is
integrated as `212ec85` with identical tested code. The adapter suite records
780 passed and 41 ignored across 19 targets; eight doctests, strict Clippy,
74 HTTP regressions, policy checks and release builds also pass. Gate archive:
`1bb70bbd294d740237c22fc13d256bc239b2dca1898e01f957b966e07bc1c70f`.
Four fixed-reference native correctness cases and a separate old/new/new/old
comparison now pass independent checks. The comparison shows slower decode,
not a performance improvement; details follow below. No new HTTP result exists.

## Boundary

The existing opt-in ordered target `DeviceTp1V3` profile appends the unchanged
residual command to its pending attention or feed-forward group. It submits and
waits once, then performs the existing collective arrival/advance and
hidden/scratch handle swap. A failed aggregate ACK cannot advance either state.
Synchronous, host/peer, draft, output-head and embedding paths are unchanged.
There is no cross-layer queue, planned state, new kernel, allocation, runtime
policy, CLI, default, or fe2o3 change.

The producer count must be exactly 10 for attention or 5 for feed-forward.
The combined count must fit the existing 16-packet bound and completed-dispatch
counter before any publication. Existing residual shape/alias checks and
per-command row binding remain before group submission. The existing ordered
flush clears its pending list on submitted failure and increments completed
packet counts only after a successful aggregate ACK. Unpublished validation
failures are terminal when reached through the batch executor; close does not
submit the retained host-side commands.

## Commands And Timing

Each of 36 layers changes groups `[10, 5]` plus two singleton residuals into
`[11, 6]`, with identical flattened kernels, arguments, extents, grids and order.
There are still 612 layer packets, 613 packets without output, and 616 with the
three-packet head. The fixed128 workload still has 135 forwards and 83,139
packets. Queue preparation/rollover and pool completion/retirement are unchanged.

The collective host trace changes to 36 ordered groups carrying 396 attention
packets and 36 groups carrying 216 feed-forward packets per forward. Each phase
has 36 aggregate waits, with no residual singleton dispatch/wait: collective
request/response pairs decrease from 144 to 72. `dispatch_each` now measures
residual preparation in each collective scope; `flush_ordered_batches` includes
that residual's completion. Old source/binary-bound raw checkers and evidence
must remain frozen; a new candidate trace checker must declare this roster.

Retained context256 native ordered b1/b2 host traces put the old residual
singleton spans at 11.685/14.790 ms of 187.284/202.558 ms per single-row decode
forward (6.24/7.30%). These are host-wall scopes including actual residual work,
not GPU durations or predicted savings. They do not establish that this is the
leading bottleneck or predict unprofiled context8192 HTTP performance.

## Validation Plan

- Existing recording equivalence covers rows 1/16/17/32 with empty, last, sparse
  and all output selections, unchanged allocation/I/O/packet preparation, exact
  11/6 groups, synchronous head barriers and completed pool retirement.
- Five new Rust tests cover malformed phase counts (including 16/17), residual
  extents/aliases, near-overflow counters, ordered submit/aggregate-wait failure
  in both collective phases, and unchanged synchronous residual success/failure.
- Existing submitted-pool failure coverage now includes residual submit failure
  and checks the actual collective key and hidden/scratch handles on failed
  first-group completion, with poison/quarantine and no completion/retirement.
- Expected adapter all-target delta is five passing tests, no new target or
  ignored tests. Against the live433 775-pass/41-ignored/19-target baseline this
  is 780/41/19; freeze exact source/counts before the remote gate.
- Root reviews source before an mi300x-only source-fresh format/host gate. Native
  qualification then needs unchanged fixed8/fixed128 IDs, UTF-8, full packet and
  lifecycle checks before a separate matched measurement. No native work is
  authorized by this note.

## Native Result

Both synchronous and ordered profiles pass exact 8/128-output token IDs and
UTF8, packet/cursor counts, retirement and normal close on physical GPU 0 of
mi350. All eight GPUs were idle before and after each case; cleanup was unforced.
The four-case archive is
`b53c4d08c3044aed9d72ed0340941b21d2b64dcc30bcb303e42929b77c707756`.
All qualification timings are excluded from the separate comparison.

The predeclared full128 old/new/new/old cohort keeps TP1, context256, the fixed
prompt, v5/v8/v11 images, worker, Wave attention and ordered submission constant.
Its arithmetic means across two runs per exact binary are:

| Controller-wall metric | Old `7cb6522` | New `2396a82` |
| --- | ---: | ---: |
| TTFT, ms | 2859.059 | 2748.736 |
| TPOT, ms | 194.521 | 218.333 |
| Workload, seconds | 27.563246 | 30.476985 |
| Mean per-run output tokens/s | 4.648212 | 4.200225 |

New TPOT is 12.24% higher and output rate 9.64% lower. Both new decode samples
are slower than both old samples. Fewer host request/response pairs did not
yield a measured overall gain here; this change is not promoted on performance.
The exact-binary comparison does not prove identical build environments, stable
performance, HTTP latency or GPU durations. Changed flush boundaries are not
ratioed, and overlapping host scopes are not added together.

Complete raw archive:
`b9c658552b29bfe11910449f61f22f1a886cc50ce072fb23df2129d208c34f01`.
Independently replayed summary:
`cb0a76b9b50dae39d28e6f03c0f2d758692c6bdd0123d9fde749b4003978496d`.
The unchanged reducer also rejects the wrong manifest pin; original source,
all 40 core file pins, lifecycle evidence and excluded diagnostics are retained.
