# Typed Prefix Timestamp Adapter

Engineering checkpoint, 2026-10-04. The additive fe2o3 prefix timestamp adapter
compiled on `ssh mi350-2` (`asrock-1w300-g2-2b`). The selected runtime and Ferric
worker suite passed **578 tests**, with four unchanged ignored tests. A separate
controller suite passed **16 tests**, with no failures, errors or skips.

This is a CPU-qualified runtime capability, not a new GPU timing result.
Ferric's existing decode selectors remain unchanged. Wiring a separate native
timing selector, capturing real device records and calibrating timestamps are
still required.

## Implementation

The new engineering-only Group method is
`dispatch_wave_qkv_attention_output_tiles_round_with_raw_timestamps_unchecked_v1`.
It returns the existing typed prefix round together with two rank-ordered raw
completion-signal observations, matching the existing typed MLP API.

The ordinary prefix entry still selects the ordinary dispatch path. The new
entry selects the existing timestamp-enabled dispatch path; it does not alter
kernel arithmetic, HSACO bytes, launch geometry, state layouts or model roots.
It accepts only a group opened with fresh profiling-enabled queues. It never
enables profiling on a live ordinary queue or substitutes host wall time.

The unchanged resident coordinator validates both ranks before publication,
retains alias and ownership checks, consumes both activations before dispatch,
acquires terminal state and checks all 284 state words per rank. All outer
currentness checks remain. Raw observations stay private until those checks
succeed. The final join verifies exact rank order, paired count, original host
intervals, common group identity and distinct device identities. Failure uses
the existing group-poisoning path; observations must remain private until
healthy group Close.

## Actual Validation

| Check | Actual result |
| --- | ---: |
| Selected runtime tests | 169 passed |
| Ferric worker library and shared-wire tests | 409 passed, four unchanged ignored |
| Total Rust tests | 578 passed |
| Controller policy tests | 16 passed |
| Owned command phases | 25 natural successful exits, groups absent |
| Reconstructed compiled source inventory | 7,626 files, before/after exact |
| Rehashed raw files / consumed input references | 128 / 127 |
| Cargo packages / local runtime crates | 39 / nine |

The total preserves all 553 previously selected passes, adds nine new tests,
and newly selects 16 existing raw-signal and queue tests. Those 25 are not all
new tests. The existing seven typed MLP timestamp tests remain included.

The nine new tests cover valid paired joins, missing/extra observations,
wrong ranks, substituted host intervals, mixed group/device identities,
corruption of every terminal word on both ranks, real poisoning after a failed
join, and actual public-entry refusal paths. The native refusal fixtures stop
before GPU I/O. Synthetic timestamps are not hardware captures.

The build used fresh archives from Ferric `44e308d7` and fe2o3 `3d217aabc`, with
only the three recorded runtime files overlaid. The target began empty. Builds
were offline and locked, with two jobs on CPUs 8/9 at nice 10, no visible GPU,
unchanged deadlines and storage limits, and an existing dependency cache.
Test optimization level was 2; debug assertions and overflow checks remained
enabled. The runtime test build reports an existing unused import in
`engineering_gfx950_resident_layer_tp2_v1.rs`; it is unrelated to this patch.

## Evidence

- [Summary and worker identity](result.json)
- [Actual Rust receipt](cpu/complete.json) and [worker test log](cpu/worker-tests-stdout)
- [Prefix timestamp tests](cpu/prefix-timestamps-stdout) and [prefix resident tests](cpu/prefix-resident-stdout)
- [Actual controller receipt](pure/complete.json) and [policy test log](pure/tests.log)
- [Frozen controller](controller/manifest.json), [CPU runner](controller/run.py), and [pure runner](run_pure.py)
- [Runtime source](implementation/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_wave_qkv_attention_output_tiles_v6.rs)
- [Timestamp test source](implementation/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_wave_qkv_attention_output_tiles_timestamp_tests.rs)
- [Exact source inputs](implementation/source-inputs.json) and [preimages](implementation/preimages.json)

The Rust receipt SHA256 is
`2d53e512b2ab882b97333c1d1889780c8fed0ab2763b411dc167b8f1735ee5da`.
The selected worker is 4,787,936 bytes, SHA256
`a5eaf7d8ca865360f3a7ae5ee5bd72e79978a173ba94fc4a2cb016649e18a0ae`.
Worker and archive bodies are retained separately, not committed to Git.
The [publication verifier](publish.py) reconstructs source hashes directly
from the retained archives and overlay, avoiding a duplicate extracted tree.
This publication is not a self-contained model or toolchain bundle.

## Remaining Work

A separate Ferric diagnostic must route every dispatch on a profiling-enabled
group, not just prefix and MLP. That includes both residual pairs and the five
single embedding/copy/tail operations: 293 records per forward, or 1,172 for
the four-forward case. Legacy host-observation schemas must keep refusing raw
queues. Complete output invariance, actual native completion and healthy Close
are required before publishing a successful capture.

Raw start/end ticks are not calibrated nanoseconds. Cross-GPU overlap graphs
need a reviewed common-clock mapping and uncertainty bounds. This checkpoint
provides no GPU timing, new numerical acceptance, speedup or throughput claim.
The [previous same-image GPU comparison](../state-bank-batch-observation-v1/tf4/README.md)
remains the latest executed decode observation.

All issue #42 M0-M7 milestones remain open. Independent full-model numerical
validation, sustained BF16 target-only Qwen3-8B 2,048/256 decoding, production
admission and 700 tokens/s remain unmet. `main` and the tutorial site are unchanged.
