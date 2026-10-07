# Reusable Guarded MLP Arenas

The opt-in runtime implementation passes **all 21 CPU qualification phases on
MI350**. Its 14 tested, formatted source files are integrated in fe2o3 commit
[`00e49fd8b0`](https://github.com/harsh-nod/fe2o3/commit/00e49fd8b0).
This checkpoint does **not** execute native arena reuse, enable a long Ferric
request, establish model numerical acceptance, or claim a throughput improvement.

## Why Reuse Is Needed

The guarded model starts with 715/711 live allocations on its two ranks. The
existing fresh path adds one arena per rank per layer dispatch, or 36 per rank
per forward. Rank zero reaches the 2,048-allocation limit on the first layer of
forward 38; the next layer cannot allocate. The requested 2,048-token prompt and
256 generated tokens require 2,303 forwards, so that path cannot complete it.

The additive API
`bind_guarded_mlp_pair_exact_own_residual_reusable_unchecked_v1` binds a retained
pair to consuming reuse. Existing fresh APIs and defaults remain unchanged.
There is no fallback allocation on failed reuse and no increase in the limit.

Each of 36 layers has two banks. A successful first use allocates one arena per
rank; later uses of that same bank reuse its completed storage. The expected
steady count is therefore `715 + 36 * 2 = 787` on rank zero and
`711 + 36 * 2 = 783` on rank one. These are source/test predictions until the
opt-in Ferric worker's native census is validated:

| Completed Forwards | Existing Fresh Path | Reusable Path, Expected |
| ---: | --- | --- |
| 0 | 715 / 711 | 715 / 711 |
| 1 | 751 / 747 | 751 / 747 |
| 2 | 787 / 783 | 787 / 783 |
| 3 | 823 / 819 | 787 / 783 |
| 4 | 859 / 855 | 787 / 783 |

## Reuse Contract

The runtime consumes a private, non-clone retirement proof for exactly the next
bank generation. Before mutation it checks both completed batches, all ten
completion signals, queue and mapping identities, and actual hardware read
frontiers past the old packet publications. It does not substitute saved logical
queue credit for observed consumption.

A private writer changes only the kernarg suffix of the mapped arena, never an
ordinary mutable slice covering its atomic signal page. Existing completion
atomics are reset with release stores and read back as pending. The next batch
receives fresh reservations and publication state. Partial failure or unwind
poisons the group instead of allocating a replacement or reusing a partial result.
Backing memory remains owned until the existing queue-first Close path.

The whole selected bank is checked before its retirement proofs transfer to the
next generation. Currentness policy, queue/error checks, existing ownership
boundaries and public allocation limits are unchanged.

## Actual Qualification

The [successful receipt](cpu-attempt-v2/evidence/complete.json) records natural
completion, reaped children, absent process groups, and clean source, dependency,
cache and tool postchecks. Both original lockfiles and all 184 worker source
bodies are unchanged. The worker in this test still selects the old fresh path;
its build verifies compatibility, not native reuse.

| Check | Actual Result |
| --- | --- |
| KFD ordinary library and integration tests | 1,101 passed, 8 unchanged ignored |
| New ordinary tests included above | 13 passed |
| Focused reuse / memory / retained / mixed-bank repeats | 7 / 2 / 18 / 13 passed |
| Selected facade doctests | 9 compile-fail and 1 compiled no-run example passed |
| Unchanged worker tests | 607 passed, 4 unchanged ignored |
| Rustdoc parser regressions | 8 passed |
| Executable / test products | 10 built and pinned |

The [first attempt](cpu-attempt-v1/evidence/failed.json) remains a failure. Its
runtime tests and ten actual doctests passed, but the controller expected one
Rustdoc summary instead of the separate compiled-example and compile-fail
sections. It stopped before the worker tests. The second attempt changes the
parser and output namespace, adds regressions using that original raw report,
and reruns all phases. No Rust source changed between the attempts.

Both attempts retain raw logs, receipts, source maps, exact tested postimages,
lockfiles, dependency/tool metadata and the data-only exporter. Neither archives
compiler executables, model weights or Cargo package bodies. All build and test
execution occurred on MI350 using two CPU cores and private offline Cargo caches.
Only the selected facade doctests were run, not every crate doctest.

## Next Native Gate

The separately implemented opt-in autoregressive worker must first build and
run four forwards through all 36 layers on both GPUs. Its actual five-sample
allocation census must reach the predicted plateau, the four complete observation
payloads must match the existing native control, and Close/device postchecks must
pass. It uses default currentness, so comparison against the older shared-mode
timings would not isolate an arena-reuse speedup.

Removing allocation growth is necessary but not sufficient for sustained
performance. Prior shared-mode host observations were 5.60-6.27 seconds per
forward. The existing one-hour long-request deadline allows only
`3600 / 2303 = 1.563` seconds per forward before setup and Close. Those observations
do not justify assuming reuse alone will meet the deadline. Redundant host checks
remain a separate optimization and measurement task. These are host-inclusive
observations, not GPU timings or a tokens/s benchmark.

All issue #42 milestones, full-model acceptance and the 700 tokens/s target remain
open.
