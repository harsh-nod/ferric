# Performance Sprint V2

Started 2026-09-11. This is an engineering work log, not a benchmark result or
protected M1 qualification receipt. Prior measurements remain frozen in
[the performance ledger](M1_QWEN3_PERFORMANCE.md).

## Team Ownership

| Team | Deliverable | State | Acceptance Gate |
| --- | --- | --- | --- |
| Core runtime | Additive checked concurrent mixed-rank dispatch round in fe2o3 | Published on main at `79706b43a177a2fd3fa43ec328221fa3e5041af5` after a fresh no-op rebase. 469 KFD tests, 31 doctests, strict scoped Clippy and independent review pass. Genuine TP2/TP8 producer/reader gates pass. | Model performance comparison; engineering execution is not protected qualification |
| Kernel numerics | Scalar/MFMA differential fixtures and diagnosis of the rejected TP1 path | 23 host tools tests pass. Native TP1 suite passes all 15 diagnostic cases and 30 dispatches, including exact full-output layout checks. Offline analysis finds accumulation differences, not an established layout/narrowing defect. | Actual model operands/logit margins; no reference relaxation or unverified fix claim |
| Measurement | Opt-in full-path host timing and strict profile summaries | Integrated. 256 Rust passes (two existing ignores), strict Clippy/release and 75 Python tests run (one skip). TP2 host control passes exact-reference and native sidecar checks. | Matched variant runs; failures excluded from performance comparisons |
| Integration | Fresh dependencies, concurrent-round worker/transport, serialized GPU experiments | Active dependencies refreshed to public `79706b43`; combined final-pin host gates pending. The preceding `4fbc0a34` pure-repin baseline has 239 Rust passes (two existing ignores), strict Clippy/release success and 70 Python tests run (one skip). | Exact model output checks, matched baseline/candidate runs, archival and cleanup |

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
