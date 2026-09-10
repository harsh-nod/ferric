# Qwen3 Performance Work

This is an engineering optimization campaign, not M1 qualification or a
serving benchmark. Frozen baseline: Ferric `713e22c`, public fe2o3
`3546d54d2c4a913f5d079701aed557d0a378bba8`, Qwen3-8B BF16 on mi350.
Builds, tests, and formatting run only on mi300x. Root coordinates hardware
runs so independent teams cannot contaminate one another's measurements.

## Progress

| Change | Owner | Implementation | GPU Measurement |
| --- | --- | --- | --- |
| Fresh unchanged baseline | Integration | Frozen existing controller/image/worker | Two TP8 cache-on runs strictly validated |
| Runtime admission cache and checked operational validation | Runtime | Published to fe2o3; integrated opt-in controls pass host tests/Clippy | Four-policy native probe passes; two operational-only Qwen runs strictly validated |
| Checked command sequences and safe queue rollover | Runtime | Published to fe2o3; integrated 10/5-command segments and exact queue receipts | Native retained-buffer rollover and dependent sequences pass; cumulative Qwen R1 passes but is slower than operational-only |
| Skip intermediate-prefill output heads | Integration | Implemented with independent opt-in flag; remote tests and Clippy pass | Two strictly validated TP8 runs; substantial timing variation, no consistent win |
| Cooperative decode GEMV and BF16 MFMA GEMM | Kernels | Wave and full MFMA images emitted; stronger numerical differential probe added | 26 wave and 36 full fixtures pass; wave Qwen seed differs from reference, so no wave speedup is accepted; MFMA model comparison pending |
| Cooperative paged attention | Kernels | Closed wave image emitted; driver mode passes host tests and strict Clippy | Included in 26-fixture native wave probe; model comparison pending |
| GPU-resident TP1 residuals | Collectives | Implemented; driver and explicit v3 admission integrated | Synthetic native fixtures pass; TP1 model comparison pending |
| Reusable host collective scratch | Collectives | Integrated and host tested; still host-staged TP1/2/8 | Isolated model ablation pending |
| True device-resident TP2/8 collective | Collectives/runtime | Peer owner published; separate serial peer child/transport integrated and host-tested; v4 image emitted | TP2 8-case and TP8 128-case ownership probes, 13 arithmetic fixtures, and GPU-producer TP2/8 6/24 observations pass; Qwen comparison pending |
| Larger row envelope and TP allocation tuning | Integration/kernels | Independent full 32-row and peer32 images emitted on public fe2o3 `3e74a932`; explicit host envelopes, admission, routing and failure tests pass; 16-row defaults retained | Native 32-row validation pending; no replica-throughput claim yet |
| Bound comparison and performance ledger | Runtime | Implemented; 36 host tests and real archived-run checks pass | Baseline, pruning, and two operational runs recorded |

## Initial Observations

Fixed TP8 cache-on workload, fresh workers, no warmup, eight generated tokens.
Every row below passed exact output-token/byte and identity-bound comparison.
Ranges show individual repetitions, not confidence intervals or stable tails.

| Variant | Repetitions | Reuse TTFT (s) | Reuse TPOT (s) | Workload Output (tokens/s) |
| --- | ---: | ---: | ---: | ---: |
| Frozen baseline | 2 | 44.2761-46.8909 | 40.6842-46.3793 | 0.033547-0.034407 |
| Output-head pruning only | 2 | 26.9561-40.5045 | 26.2307-39.6742 | See per-run ledger; large variation |
| Operational currentness only | 2 | 2.4190-2.4330 | 1.1400-1.1569 | 0.471980-0.504046 |
| Current controller/runtime, all controls off | 1 | 46.5623 | 38.1315 | 0.028966 |
| Operational + admission cache + sequences | 1 | 4.1125 | 2.5865 | 0.296581 |
| Admission cache only | 1 | 46.2735 | 52.9616 | 0.032950 |
| Sequences only | 1 | 64.6440 | 64.9780 | 0.025022 |

The final four rows use the same controller `bc4a283b...` and worker `70572ff2...`.
The cumulative/control single-run ratio is 10.2391x output rate. The cumulative
profile is slower than the separately measured operational-only profile;
there is no established incremental cache/sequence benefit. Admission-only
has mixed throughput/TPOT results, and sequence-only has 13.61% lower workload
output rate than the matched control. Host scratch reuse and wave attention
are queued separately. These single observations do not establish repeatable
gains or regressions.

The wave-projection candidate completed but failed strict model comparison:
`seed-prefix` generated `[9856, 374]` (" Germany is") instead of `[17689, 374]`
(" Spain is"). The other three request summaries matched. This failed
candidate is archived, excluded from accepted timing ledgers, and remains
unqualified while stronger arithmetic differential tests are performed.

The operational-only runs show roughly 14-15x baseline workload output rate,
not a serving-throughput claim. They retain admission caching, sequences,
rollover, pruning and kernel substitutions disabled. Controller SHA-256:
`d06b54cc59f1daf787111f873433cb6ecdc3c8205582dad78738a887d654bffe`;
worker `70572ff23ae4453ea1d7947061692c6e9e410c6980c7679fde4b467d636d6912`;
image remains the frozen `af5019d3...` v2 image. The exact ledger and raw receipts
are archived in the local `ferric-perf-measurement-peer-evidence-v4` and
`ferric-performance-evidence-v1` directories. Model setup (133.3140 s) and whole
run (163.6495 s) are reported separately from its 15.8716 s workload window.

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
