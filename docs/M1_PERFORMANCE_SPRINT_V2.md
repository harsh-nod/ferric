# Performance Sprint V2

Started 2026-09-11. This is an engineering work log, not a benchmark result or
protected M1 qualification receipt. Prior measurements remain frozen in
[the performance ledger](M1_QWEN3_PERFORMANCE.md).

## Team Ownership

| Team | Deliverable | State | Acceptance Gate |
| --- | --- | --- | --- |
| Core runtime | Additive checked concurrent mixed-rank dispatch round in fe2o3 | Published on main at `79706b43a177a2fd3fa43ec328221fa3e5041af5` after a fresh no-op rebase. 469 KFD tests, 31 doctests, strict scoped Clippy and independent review pass. Genuine TP2/TP8 producer/reader gates pass. | Model performance comparison; engineering execution is not protected qualification |
| Kernel numerics | Scalar/MFMA differential fixtures and diagnosis of the rejected TP1 path | Synthetic and actual-model captures pass integrity checks. The immediate token flip is a BF16 final-logit tie, not an argmax/layout defect. Baseline still passes the fixed reference; MFMA BF16 still fails. A separate opt-in FP32-head candidate is underway. | Fresh three-root emission, native FP32 checks and full fixed-reference model validation; no reference relaxation |
| Measurement | Opt-in full-path host timing and strict profile summaries | Integrated. 256 Rust passes (two existing ignores), strict Clippy/release and 75 Python tests run (one skip). TP2 host control passes exact-reference and native sidecar checks. | Matched variant runs; failures excluded from performance comparisons |
| Integration | Fresh dependencies, concurrent-round worker/transport, serialized GPU experiments | Public `79706b43` combined capture gate passes 266 Rust tests (two existing ignores), strict Clippy/release, standalone peer tests, 26 metadata configurations, source/negative gates and 31 verifier policies. Two new upstream compiler test targets are regenerated into the dependency inventories. | Remaining matched TP8 variants, FP32 candidate gates, archival and cleanup |

## Constraints

- Builds, tests, formatting and Python execution run only on `mi300x` in owned
  private stages, with bounded build concurrency.
- The integration lead alone schedules GPU execution on `mi350`, after checking
  shared-host availability. No overlapping experiments or foreign process kills.
- Core compiler/KFD changes belong in fe2o3 and require fresh fetch/rebase before
  publication to main. Kernel, inference and measurement changes stay in Ferric.
- Previous scalar/host-staged profiles remain available as controls. A concurrent
  profile must have a distinct name; serial dispatch is not reported as overlap.
- Correctness precedes timing acceptance. Setup, host wall time and measured GPU
  time are distinct. No synthetic or rejected output becomes an accepted gain.
- Preserve user worktrees and shared model caches. Archive evidence and remove
  completed owned worktrees and temporary build stages.

## Scope

The first integration targets runtime overhead, real rank concurrency and
numerical diagnosis. Fusion, broader batch/chunk tuning and persistent serving
lifetime changes follow measured attribution; they are not implicitly complete
because the profiling hooks exist. Symmetric memory and MTP remain deferred.

## Concurrent-Round Native Gate

The frozen candidate child (`2032e31b`), built from the now-public core `79706b43`
using a recorded private Cargo path patch, passes both TP2 and TP8 probes. Each
checks 12 real mixed-rank rounds at one, three and sixteen rows: GPU BF16 producers
feed peer readers, and GPU FP32 producers feed ordered peer reductions. All
outputs, guards and tails match; close acknowledgements, zero exits, child reap
and all-eight-card idle snapshots pass. These are correctness checks, not device
overlap timestamps or Qwen benchmarks.

The first TP2 probe stopped before dispatch because its harness incorrectly
required optional pointer metadata omitted by the frozen image. Its failed receipt
is preserved. The revised harness requires the exact absent-field contract and
rejects non-null annotations and ABI mutations; runtime admission was unchanged.

## Numerical Checkpoint

The TP1 diagnostic report is SHA-256
`31f9fa2ff265c2d76627c7e071b912f6eb102572f2914744e8435a00797c77b5`.
It uses the frozen `8c81d3fe` full-v3 image and `189b918d` independent worker;
neither artifact is relabeled as a fresh build. The worker closes/reaps and a
delayed snapshot confirms all eight GPUs idle. The immediate snapshot retained
one percent utilization on card zero with zero allocated memory; both snapshots
are preserved, rather than replacing the first observation.

These are synthetic real-shape correctness diagnostics, not model parity or
performance results. Dense FP32 partial outputs differ at many bit positions
between scalar and MFMA, while sampled MFMA error versus the higher-precision
sum is smaller. This does not establish the model token mismatch's cause.

## Actual Model Capture

The separate bounded capture controller (`b7f33190`) preserves actual scheduler
rows and projection/head operands, never performance-qualified timings. Both
TP1 runs close cleanly and pass payload hashes, bounds, row binding, complete
observed-trace and actual head argmax checks. The independent fixed-reference
check passes baseline and rejects MFMA BF16 with the unchanged seed mismatch.

For selected batch two, layer zero Q input and original weights are byte-identical;
the complete MFMA weight transpose is exact. Four of 24,576 Q outputs differ.
The final normalized inputs differ at 21,314 of 24,576 BF16 elements, but this
capture does not attribute those differences to only the four observed Q values.
Seventy-eight shared selected LM weight rows are byte-identical.

On the seed row, baseline BF16 logits are 24.5 for token `17689` and 24.375 for
`9856`. MFMA's BF16 logits are both 24.375, so the existing lowest-ID argmax
correctly selects `9856`. On MFMA's actual final input, the scalar FP32 reference
is 24.42636108 for `17689` versus 24.36589813 for `9856`; FP64 also favors `17689`.
Both FP32 values round to the observed BF16 tie. All 106 captured top/watch head
outputs per variant match serial FP32 accumulation followed by BF16 narrowing.
The immediate precision-loss mechanism is established; no generic argmax defect
or qualified full-run fix is claimed.

Pair analysis SHA-256:
`e26a5dece9ecf1a42efa8b1e9d3f51164df7b0e4e18c43ea3be84e23fd22fe52`.
Independent diagnostic reports are
`582aef5bc0e67e0b910373c1d7026999327150376b6b900ea1ef162acda91197`
(baseline reference pass) and
`aad0cabe048c95b6fa13b61346701d49eafd80b1db33572ffd92aed39e283fe8`
(MFMA reference failure). Their schemas permanently exclude performance claims.

## Matched TP2 Checkpoint

Same frozen timing controller, base image, workload and policies; serial/round
also use the exact same peer child and peer image. All rows below pass eight
fixed-reference output tokens, complete dispatch/row accounting and idle/close
checks. Each is one short engineering observation, not a serving or stable-tail
result. Host versus peer also changes collective placement and worker graphs.

| Mode | Output Tokens/s | Workload Seconds | Setup Seconds | Reuse TTFT Seconds | Reuse TPOT Seconds |
| --- | ---: | ---: | ---: | ---: | ---: |
| Host staged | 0.871547 | 9.179084 | 106.238880 | 1.697529 | 1.044582 |
| Peer serial | 0.143430 | 55.776348 | 221.343087 | 11.302857 | 10.399142 |
| Peer concurrent round | 0.268161 | 29.832794 | 221.191863 | 6.029658 | 5.583170 |

Concurrent rounds improve workload output rate 86.96% over the serial peer path,
but remain 69.23% below host staged. They do not fix peer upload or teardown
overhead. Full TP2 performance ledger SHA-256:
`085a233fdc654e6cf75beee9c3bc1dc8dd42c6a2d9b7ac2216c2a1692702c420`.
Serial-baseline isolated pair:
`2297d9c4b8067b59c3c809efc38ba424a48ce12def680fb136bf224e9106c1d0`.

The new TP8 host control also passes: 0.380674 output tokens/s, 21.015336 seconds
workload, 135.388216 seconds setup, reuse TTFT/TPOT 3.534553/1.999206 seconds.
Its two non-overlapping collective spans total 12.169842 seconds, about 57.9% of
the workload window. These are host-observed phase durations, not GPU timings.
The TP8 concurrent-round case also passes all fixed-reference and teardown
checks: 0.076712 output tokens/s, 104.286367 seconds workload, 737.535894 seconds
setup, reuse TTFT/TPOT 20.909556/20.617757 seconds. Its comparison report is
`541eaf20f25a96f83521e5bd1be13bd636c9a4aafb0af3546b3ae98876f03abb`.
The two collective spans fall to 10.217258 seconds, but attention and FFN
non-collective spans grow to 58.796540 and 29.843190 seconds. Resident setup
grows from 47.309137 to 644.009930 seconds. These host-observed spans do not
isolate GPU execution or individual system-call overhead. The TP8 serial
control remains pending; do not extrapolate the TP2 gain.

Independent repin review found one stale compiler tree in the promotion
behavioral harness. Its commit/tree pair now matches public `79706b43` and
tree `45f230ccc6fe2eb7400c68f20d68abafdc2f1b82`; the remote repin gate checks
both together. This narrow identity correction does not claim a full rerun
of that behavioral harness.
