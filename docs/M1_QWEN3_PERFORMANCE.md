# Qwen3 Performance Work

This is an engineering optimization campaign, not M1 qualification or a
serving benchmark. Frozen baseline: Ferric `713e22c`, public fe2o3
`3546d54d2c4a913f5d079701aed557d0a378bba8`, Qwen3-8B BF16 on mi350.
Builds, tests, and formatting run only on mi300x. Root coordinates hardware
runs so independent teams cannot contaminate one another's measurements.

## Progress

| Change | Owner | Implementation | GPU Measurement |
| --- | --- | --- | --- |
| Fresh unchanged baseline | Integration | Frozen existing controller/image/worker | Two TP8 cache-on runs complete; strict comparison pending |
| Runtime admission cache and checked operational validation | Runtime | Opt-in implementation; host tests and independent review pass | Live GPU probe in progress |
| Checked command sequences and safe queue rollover | Runtime | Implemented; host tests and independent review pass | Live GPU probe in progress |
| Skip intermediate-prefill output heads | Integration | Implemented with independent opt-in flag; remote tests and Clippy pass | Pending |
| Cooperative decode GEMV and BF16 MFMA GEMM | Kernels | V3 sources and host tests complete; MFMA convergence repro isolated; wave-only emission in progress | Pending |
| Cooperative paged attention | Kernels | V3 source and host tests complete; emission in progress | Pending |
| GPU-resident TP1 residuals | Collectives | Implemented and host tested; integration pending | Pending |
| Reusable host collective scratch | Collectives | Implemented and host tested; still host-staged TP1/2/8 | Pending |
| True device-resident TP2/8 collective | Collectives/runtime | Owned gfx950 peer-group capability in progress; no device-collective claim yet | Pending |
| Larger row envelope and TP allocation tuning | Integration/kernels | Follow kernel bounds and queue lifecycle work | Pending |
| Bound comparison and performance ledger | Runtime | Strict per-variant comparison and repeated-run aggregation in progress | Pending |

## Measurement Contract

Each result retains the exact controller, worker, image, model, workload,
options, device roster, exit status, raw JSONL, and correctness comparison.
Individual toggles isolate pruning, runtime policy, projection, attention,
and reduction changes. Cumulative combinations are reported separately.
No percentage improvement is recorded before matching output/correctness
checks and an actual timed run. Failed, noisy, and regressing candidates stay
visible instead of being relabeled as wins.

TTFT starts at actual admission and includes in-runtime queueing, but excludes
model setup. TPOT uses adjacent committed output times and records the number
of intervals. Workload output throughput is generated tokens divided by time
from first admission to the final terminal event, including cancellations;
it is not steady-state HTTP throughput. Setup and whole-process time remain
separate. Per-request latencies are compared by request identity, not pooled
across different prompt/cache/output shapes.

Initial ablations retain the frozen four-request workload with eight generated
tokens; these are short engineering observations. Repetitions, warmup policy,
cache on/off, and TP1/2/8 are explicit. Broader workloads and eight TP1 replicas
are separate experiments, not substitutions for the fixed TP8 comparison.

Matrix/wave reductions change floating-point evaluation order. V3 numerical
profiles therefore need their own tests and reference comparisons, not an
assertion that prior scalar or Verus receipts cover them. Compiler/KFD changes
stay in fe2o3; all model kernels and inference changes stay in Ferric.
