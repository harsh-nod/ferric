# Current Layer-Zero Native Capture

The current Qwen3-8B BF16 TP2 kernels completed a genuine layer-zero capture
on `ssh mi350`. The input is model token `9112` at position zero, with fresh
KV state, the current V7 prefix image and paired-row MLP image. No hidden
values were injected. This is a diagnostic capture, not numerical acceptance
or a throughput benchmark.

## Observed Results

| Check | Result |
| --- | ---: |
| Native attempts / retries | 1 / 0 |
| Captured arrays across two ranks | 28 |
| BF16 arrays / FP32 partial arrays | 24 / 4 |
| Capture payload | 9,670,656 bytes |
| Final hidden state versus current four-forward baseline | Byte-identical on both ranks |
| Untouched KV regions | Checked |
| Before / after selected-device audits | 3 / 3 passed |
| Naturally exited and reaped process leaves | 7 |
| Forced cleanup | None |
| Separate capture-controller tests | 40 passed |

Both selected GPUs had empty process rosters in all six audits; this MI350
run did not use a shared-monitor exception. The parent observed successful
worker `Close` and reap. The retained outer process records independently
record ownership rather than constructing a worker identity after the fact.

The [parent qualification](../layer0-native-capture-parent-v1/README.md)
contains 289 passing Rust tests and its actual build evidence. The unchanged
worker comes from the separate 669-test clock-recorder qualification. Parent
and worker runtime audits were performed before this GPU attempt; these are
separate checks, not one combined test suite.

## What This Establishes

The new diagnostic path captures the same final hidden values as the current
full-forward native path. That makes its intermediates useful for investigating
the known difference from the [independent framework](../layer0-framework-capture-v1/README.md).
It does not establish that those hidden values are numerically correct.

The next comparison uses matching BF16 stage values. Rank-local FP32 output
and down-projection partials are separately typed and must not be compared
directly with complete BF16 framework projections. The first observable
divergence will locate a diagnostic boundary, not prove a causal explanation.
No acceptance tolerance is inferred from the measured discrepancy.

## Evidence

- [GPU completion](capture/complete.json), [publication ledger](result.json),
  and [typed stage ledger](stage-ledger.json).
- [Native summary](capture/native/summary.json),
  [parent process result](capture/parent/result.json), and
  [structural replay](capture/observation.json).
- [Frozen supervisor](source/run.py), [capture validator](source/capture_validation.py),
  [40-test receipt](pure/complete.json), and [actual request](inputs/request.json).

The actual GPU completion SHA-256 is
`632a779159c60bae46aa7952c99519943fd83ca11b83de7d22bd1b9a9cb5742a`.
Raw tensor captures, model data and executables remain outside Git. Publication
rechecks retained capture structure and process evidence; it does not reexecute
the controller or claim to rehash every transitive admission dependency.

Full-model numerical correctness, calibrated GPU timing, overlap and sustained
single-request BF16 2,048/256 decoding remain open. No 700 tokens/s result is
claimed, and no issue #42 M0-M7 milestone is closed by this checkpoint.
