# Performance Qualification

Performance is empirical evidence. Proof restricts which implementations are
legal; it does not prove that an implementation is fast.

Ferric adopts fe2o3 D8-D10 as release policy:

1. Tune only bounded, proved, target-valid, and resource-valid variants.
2. Give fused graphs, finite megakernels, and persistent workers distinct
   identities and refinement evidence.
3. Qualify only exact artifacts that already passed proof, numerical, canary,
   ISA, resource, and target gates.

## Reproducibility Identity

Every result binds:

```text
Ferric and fe2o3 commits
ExecutableId, ScheduleId, and DispatchGraphId
model, tokenizer, config, and weight digests
vLLM and SGLang commit or container digest
ROCm, LLVM, kernel driver, firmware, and GPU identities
topology, clocks, power limit, thermals, CPU, NUMA, and affinity
benchmark harness, workload, trace, flags, environment, and cache policy
```

Baselines receive the same bounded configuration-search budget on calibration
workloads. Qualification uses a held-out suite and compares Ferric with the
faster baseline per workload cell.

## M1 Workload Matrix

| Dimension | Values |
| --- | --- |
| Batch/concurrency | `1, 4, 16, 32` |
| Prefill length | `128, 512, 2048, 8192` |
| Decode KV length | `128, 1024, 4096, 8192` |
| `(ISL, OSL)` | `(128,128)`, `(1024,256)`, `(4096,256)`, `(512,2048)` |
| Arrival | closed-loop, Poisson, burst, overload sweep |
| Prefix sharing | none in M1; `50%` and `90%` after prefix caching lands |
| Speculation | target-only and draft lengths `1, 2, 4, 8` |
| Acceptance | pinned low, mixed, and high-acceptance traces |

Eight-device qualification later adds collective sizes, topology, DP/TP/EP
scaling, MoE skew, expert capacity, communication overlap, and rank balance.

## Required Metrics

Kernel and graph metrics:

- median/p90/p99 latency, achieved FLOP/s, and HBM bandwidth;
- launch count, queue gaps, host synchronization, and allocation activity;
- VGPR/SGPR/AGPR, LDS, occupancy, scratch, spills, stack, and calls; and
- MFMA/VALU utilization, cache/HBM traffic, and wave stall reasons.

Serving metrics:

- TTFT, ITL, TPOT, and end-to-end p50/p90/p99 latency;
- input/output/total tokens per second and requests per second;
- maximum sustainable goodput under predeclared TTFT and ITL SLOs;
- peak HBM, usable KV capacity, fragmentation, CPU time, and energy/token; and
- errors, OOMs, timeouts, cancellations, starvation, and fairness.

Speculation additionally reports acceptance by position, mean accepted length,
draft/verify/sample/commit/rollback time, target invocations, rejected work,
KV rollback cost, and speedup over Ferric target-only execution.

## Statistical Protocol

For every qualification cell:

1. Run at least 10 warmups and 30 recorded samples.
2. For serving, use three fresh server starts with ten steady-state windows.
3. Replay identical prompt order, arrivals, seeds, and output limits.
4. Rotate engine order and retain failed runs and externally recorded faults.
5. Report medians and paired bootstrap 95% confidence intervals.
6. Reject persistent variance above 2% for kernel latency or 5% for serving.
7. Reject thermal/clock drift above 3%, link/ECC errors, or environment drift.

## Release Gates

- Core-kernel weighted geometric mean is at least 95% of the fastest pinned
  applicable vendor baseline; no declared core shape is below 80%.
- No qualified Ferric metric regresses more than 5% without reviewed rebaseline.
- Every primary serving cell has a lower confidence bound of at least 0.95
  relative to the faster tuned vLLM/SGLang baseline at equal p99 SLO.
- A public faster claim requires the primary-suite lower 95% confidence bound
  to exceed 1.05.
- Speculation must beat Ferric target-only throughput by 10% on its eligible
  holdout without more than 5% p99 latency regression.
- Low-acceptance traffic must select an already admitted deterministic plan and
  limit regression to 5%.
- Eight-device DP saturated-throughput efficiency should reach at least 85%.

Before timing, reject unexpected scratch, spills, dynamic stack, calls,
symbols, control flow, occupancy loss, resource-manifest drift, allocations in
the token loop, host waits, queue drains, graph breaks, or dependency/lifetime
violations.

