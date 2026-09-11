# Performance Sprint V2

Started 2026-09-11. This is an engineering work log, not a benchmark result or
protected M1 qualification receipt. Prior measurements remain frozen in
[the performance ledger](M1_QWEN3_PERFORMANCE.md).

## Team Ownership

| Team | Deliverable | State | Acceptance Gate |
| --- | --- | --- | --- |
| Core runtime | Additive checked concurrent mixed-rank dispatch round in fe2o3 | Implemented; initial scoped host tests, doctests and Clippy pass. Independent review found no blocker; added pending-identity negatives are in the incremental gate. | Genuine TP2/TP8 producer/reader probes before model use; fresh rebase before main publication |
| Kernel numerics | Scalar/MFMA differential fixtures and diagnosis of the rejected TP1 path | 23 host tools tests pass. Native TP1 suite passes all 15 diagnostic cases and 30 dispatches, including exact full-output layout checks. Offline analysis finds accumulation differences, not an established layout/narrowing defect. | Actual model operands/logit margins; no reference relaxation or unverified fix claim |
| Measurement | Opt-in full-path host timing and strict profile summaries | Timing module, CLI sidecar, driver/IPC hooks and strict summary checks implemented; combined remote host gate in progress. | Timings correctly attributed, failures excluded from performance comparisons, default behavior unchanged |
| Integration | Fresh dependencies, concurrent-round worker/transport, serialized GPU experiments | Active dependencies refreshed to public `4fbc0a34c7938474406d37a45e7972ea8b1ec277`; pure-repin baseline passes 239 Rust tests, strict Clippy, release build and 70 Python tests. Concurrent transport integrated for combined testing. | Exact model output checks, matched baseline/candidate runs, archival and cleanup |

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
