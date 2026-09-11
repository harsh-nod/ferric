# Performance Sprint V2

Started 2026-09-11. This is an engineering work log, not a benchmark result or
protected M1 qualification receipt. Prior measurements remain frozen in
[the performance ledger](M1_QWEN3_PERFORMANCE.md).

## Team Ownership

| Team | Deliverable | State | Acceptance Gate |
| --- | --- | --- | --- |
| Core runtime | Additive checked concurrent mixed-rank dispatch round in fe2o3 | Inspecting publication, completion and quarantine contracts against public `4fbc0a34` | Independent review, negative/fault tests, genuine TP2/TP8 producer/reader probes before model use |
| Kernel numerics | Scalar/MFMA differential fixtures and diagnosis of the rejected TP1 path | Building bounded real-shape numerical diagnostics in Ferric | Inputs/guards preserved, raw outputs retained, no reference relaxation or unverified fix claims |
| Measurement | Opt-in full-path host timing and strict profile summaries | Defining timing schema and focused tests in Ferric | Timings correctly attributed, failures excluded from performance comparisons, default behavior unchanged |
| Integration | Fresh dependencies, concurrent-round worker/transport, serialized GPU experiments | Confirmed public fe2o3 `4fbc0a34c7938474406d37a45e7972ea8b1ec277`; preparing dependency update | Remote host checks, exact model output checks, matched baseline/candidate runs, archival and cleanup |

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
