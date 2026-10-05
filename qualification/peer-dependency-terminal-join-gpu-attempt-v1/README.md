# Terminal Peer Join: Native Failure

This checkpoint supports [issue #42](https://github.com/harsh-nod/ferric/issues/42).
The explicit final join did **not** satisfy the strict queue-retirement gate.
This is retained failure evidence, not GPU qualification or a performance result.
All M0-M7, independent numerical acceptance, sustained 2,048/256 decoding and
the 700 tokens/s target remain open.

## Actual MI350 Attempt

The controller used the exact ELF from the
[1,029-pass CPU checkpoint](../peer-dependency-terminal-join-cpu-v1/README.md),
runtime commit `203cfcbef166932b04683cd5c88f3414fe3b8a25`.
All 32 controller tests passed on MI350: twelve new admission fixtures,
twelve library-parser fixtures and eight test-log-parser fixtures.

One native process ran over `ssh mi350` on `smci350-rck-g03-b19-03`.
It used two gfx950 GPUs, two device iterations, eight kernel packets and eight
barriers, with sixteen distinct completions and no reset or retry.

| Observation | Actual result |
| --- | --- |
| First sampled all-zero time | 84,211,522 ns from native operation start |
| Signals in that sample | All sixteen zero |
| Actual `(write, read)` counters | `(8, 3)` on both queues |
| Packet headers in that sample | All sixteen INVALID |
| Last sample time | 59,999,695,823 ns |
| Last signal/counter/header state | Same as first all-zero sample |
| Retained changed states / dropped changes | 5 / 0 |
| Diagnostic read error | None |
| Native outcome | Aggregate deadline; natural exit code 1 |
| Native process wall interval | 60.641720581 s |
| Pre/post process audits | All eight GPUs idle |

These are sequential host observations, not simultaneous device timestamps.
The sampled state did not change after the fifth recorded transition; the
trace does not prove continuous hardware state between samples. No output or
guard readback occurred, no healthy Close was reported, and no throughput was
measured. Signals and INVALID headers alone did not bypass the strict gate.
All five subprocesses were reaped with absent process groups; none timed out
externally or required forced cleanup. Final input checks passed.

## Next Contract Review

ROCr's pinned
[interceptor source](https://github.com/ROCm/rocm-systems/blob/97f5574fe2fdc7bef44fb01545347912ee9f1779/projects/rocr-runtime/runtime/hsa-runtime/core/runtime/intercept_queue.cpp#L78)
notes that read-index advancement has no specified latest point. Existing
fe2o3 ordinary/ordered dispatch already separates signal-completed work from
the actual read cursor used for ring capacity. The sentinel's equality gate
is stronger. The next change must reconcile those contracts and peer-arena
lifetime explicitly, not write hardware cursors or infer reusable storage
from this failed run. The cause of the particular read value 3 is not established.

## Evidence

[`result.json`](result.json) hashes all 35 retained files. The original
[`failed.json`](retained/evidence/failed.json) has SHA256
`dc9bc553d5f88ee70008521f06c563e6c088adf82f316885691986b99bc96f4d`.
The full unmodified trace is in [`native.stderr`](retained/evidence/native.stderr).
The controller checks the caller-pinned CPU receipt, all eleven CPU phases,
59 raw files, both selected ELFs, sources, tools and runtime libraries.

The retained controller is pinned to this host and a fresh workspace; it is
not a portable launcher. Request and result use V2 schemas. This failed
attempt does not alter the previous V1 failure or either strict runtime entry.
