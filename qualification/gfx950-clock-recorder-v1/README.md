# gfx950 Clock Recorder V2

This is a CPU-qualified engineering worker route, not a GPU timing result or
production admission. It advances [issue #42](https://github.com/harsh-nod/ferric/issues/42);
all M0-M7 milestones and the single-request BF16 2,048/256, 700 tokens/s target
remain open.

## Implementation

The exact worker selector is
`--engineering-native-prefix-decode-device-clock-v2`. It uses the existing raw
timestamp queues and retains the complete V1 dispatch report inside
`FerricPrefixDecodeDeviceClockObservationV2.raw`. The additional sixteen samples
come from the checked native runtime clock API, before and after each of four
forwards on both ranks. Samples retain GPU/CPU/system counters and the system
counter frequency, without converting raw GPU ticks to nanoseconds.

Each sample binds its generation, forward position, pre/post boundary, rank,
group incarnation, unique device ID, KFD GPU ID, queue epoch and monotonic host
sampling bracket. Packet boundaries must be 0/293/586/879/1,172 as appropriate.
The wrapper validates each pair before appending it. It preserves all raw
Control/completion joins and the 2 MiB sidecar bound.

Sampling, recording, wire, capture and control failures terminalize both
recorders and the native owner's catalog, state bank and forward sequence.
This includes post-sample failures after a forward has committed. A report is
published only after consuming the existing native Close result and successfully
writing the Closed response. The old selectors do not silently select V2.

## Actual Remote Qualification

Tests ran on `ssh mi350-2` (`asrock-1w300-g2-2b`), CPU affinity 8/9, nice 10,
with GPU visibility empty. Fresh archived sources used Ferric
`924703865d9cfcb1dc791af211b7558ff7c58894` and runtime
`27b53d2b74c1f239988a891a4aed39e089b05663`, followed by the exact twelve-file
worker overlay. Cargo ran offline/locked with two jobs, an empty target and
checked optimized test profiles.

| Check | Actual result |
| --- | --- |
| Existing selected runtime tests | 208 passed |
| Worker and shared-wire tests | 461 passed, 4 existing ignores |
| Newly added worker tests | 21 passed, included above |
| Total selected Rust tests | 669 passed, 4 ignored |
| Separate controller-policy tests | 15 passed |
| Bounded commands | 33 naturally completed |
| Input, source, tool and artifact postchecks | Passed |

The full runtime inventory remains 900 tests. The full worker inventory is the
previous 444 names plus exactly 21 additions. New cases cover the V2 schema,
recorder pair/order/Close boundaries, terminal failure callbacks and exact CLI
separation. Native-success clock sampling is not simulated into a GPU claim.

The actual worker is 5,018,856 bytes, SHA-256
`d2af909e7fef88af5b20f4ae8f2a5779a7aee93a187169e97a939bec7d3a0fed`.
The remote CPU receipt is SHA-256
`41ddcf8a9cb48b970d4f9a4187de6f6de1fd0506ba098d524e82d17335030f1d`.
See [the retained evidence ledger](result.json), [CPU receipt](cpu/complete.json)
and [controller](controller/run.py). The local publication rechecks retained
data and source identities; it does not rerun the remote compiler or controller.

## Remaining Gates

The separately versioned parent route must pass its own build and validation.
The actual binaries then need runtime audits and a fresh MI350 capture proving
all sixteen samples, 1,172 rows, unchanged tensors and normal Close/reap.
Clock-domain validation, calibrated durations with uncertainty, cross-device
alignment, overlap, independent full-model acceptance and sustained throughput
are not established here. No kernel image or arithmetic changed.

The first formatting harness rejected the parent Cargo manifest before copying;
the second copied drafts but did not launch rustfmt because its working directory
was absent. The corrected V3 formatter passed both rustfmt and its check on the
remote host. Only that completed formatter's worker bodies entered this build.
