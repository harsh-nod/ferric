# Ordered TP1 Residual Tail

Source-only experiment based on Ferric
`0d1a90d9b0940552a6d87b699353f5be6b71bb80`. No host gate, native numerical
qualification, or latency result is claimed for this change.

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
