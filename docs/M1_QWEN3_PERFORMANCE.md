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
| Cooperative decode GEMV and BF16 MFMA GEMM | Kernels | Wave and full MFMA images emitted; stronger numerical differential probe added | 26 wave and 36 full fixtures pass; wave Qwen seed differs from reference; first matched MFMA model pair passes with a 27.91% workload-rate gain and a startup regression |
| Cooperative paged attention | Kernels | Closed wave image emitted; driver mode passes host tests and strict Clippy | Native fixtures pass, but both original and operational-matched model runs have the same seed mismatch as wave projection; rejected for timing claims |
| GPU-resident TP1 residuals | Collectives | Implemented; driver and explicit v3 admission integrated | First exact-reference matched model pair passes; workload rate +14.19%, reuse TTFT -6.76%, reuse TPOT -0.97%; single sample only |
| Reusable host collective scratch | Collectives | Integrated and host tested; still host-staged TP1/2/8 | First isolated model ablation passes; mixed per-request results, no repeatable gain established |
| True device-resident TP2/8 collective | Collectives/runtime | Public `902fef6e` peer runtime and child, explicit operational/cache/sequence controls, separate v4/v6 images | Native GPU-producer and cached-sequence probes pass at TP2/8, including rows 17/31/32; model and source-matched host controls pass, but peer workload rate is 82.06%/97.66% lower at TP2/TP8 |
| Larger row envelope and TP allocation tuning | Integration/kernels | Full 32-row and peer32 images emitted on public fe2o3 `3e74a932`; explicit admission/routing and real coordinator schedule tests pass; default remains 16 | Native suites and actual 32-row Qwen cohort pass; same-image row-policy pair gains 12.56% workload rate with mixed request latency |
| Bound comparison and performance ledger | Runtime | Implemented; 59 comparison tests and historical-run revalidation pass; wide capacity and observed rows are distinct | Baseline, pruning, operational, isolated ablations, matched MFMA/TP1 pairs, slow peers and rejected wave cases retained |
| Same-host replica cohorts | Kernels/measurement/integration | Identity-bound shared RAW-clock control, exact workload partition, strict cohort verifier and process cleanup integrated; 11 launcher tests and strict Clippy pass | All three allocations pass 64 exact outputs; observed rates 1.442/5.734/6.588 tokens/s for 1xTP8/4xTP2/8xTP1, with row-capacity and model-memory costs retained |
| Bit-exact tiled MFMA weight transpose | Kernels/integration | Integrated setup-only raw-byte transpose; 240 full-array comparisons pass across actual TP1/2/8 shard shapes | Host helper suites improve 2.67-3.37x; first matched model pair saves 26.55 s setup (15.33%), with no causal decode-speed claim |

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
| Host scratch reuse only | 1 | 46.7256 | 40.0161 | 0.034571 |

The final five rows use the same controller `bc4a283b...` and worker `70572ff2...`.
The cumulative/control single-run ratio is 10.2391x output rate. The cumulative
profile is slower than the separately measured operational-only profile;
there is no established incremental cache/sequence benefit. Admission-only
has mixed throughput/TPOT results, and sequence-only has 13.61% lower workload
output rate than the matched control. Host scratch reuse improves global rate
in one sample but slightly worsens reuse-request TTFT/TPOT. These single
observations do not establish repeatable gains or regressions.

The wave-projection candidate completed but failed strict model comparison:
`seed-prefix` generated `[9856, 374]` (" Germany is") instead of `[17689, 374]`
(" Spain is"). The other three request summaries matched. This failed
candidate is archived and excluded from accepted timing ledgers. The isolated
wave-attention run has the same seed mismatch. A baseline-arithmetic control
using that same wave image passes strict comparison. Stronger mixed-sign and
cancellation projection fixtures pass their respective serial/wave arithmetic
orders, but those tests do not establish model-token parity. Both matched
operational-mode reruns also fail with the same seed output. No failed wave
timing is included in accepted performance ledgers.

### Matched MFMA Pair

These two single-run observations share controller `bc4a283b...`, worker
`70572ff2...`, full-v3 image `8c81d3fe...`, operational validation enabled,
baseline attention, and all other optimization flags disabled. Only projection
changes. All eight expected tokens/bytes and timing/teardown records pass.

| Projection | Reuse TTFT (s) | Reuse TPOT (s) | Workload Output (tokens/s) | Setup (s) | Whole Run (s) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Baseline | 2.630847 | 1.261079 | 0.449565 | 119.146961 | 151.628723 |
| MFMA | 1.819253 | 0.854889 | 0.575054 | 173.301530 | 209.070221 |

The candidate lowers these TTFT/TPOT observations by 30.85%/32.21% and raises
global workload rate 27.91%, but setup and whole-run time regress. It allocates
15,136,194,560 extra resident transposed-weight bytes. The timing change is
consistent with added setup work; no separate setup-cost attribution is claimed.
Pair ledger SHA256:
`8660be59f4d452e3944131069c5a7824c0c72f6c3e87bfcd83f4a7d475ad3884`.
This is not a stable serving result or a long-generation numerical qualification.

### Peer Model Gate

The first TP2 serial device-peer run passes strict reference, ownership,
dispatch-count and teardown checks. Reuse TTFT is 11.040452221 seconds and TPOT
is 10.162302622 seconds; setup is 220.861978648 seconds and whole run is
285.630395408 seconds. Comparison SHA256:
`7452fafe952ead228830782e83c291a8f4d1b345b90cc50ab690114f6713e9ee`.

The TP8 serial peer run also passes all strict checks, but reuse TTFT is
141.017906811 seconds and TPOT is 138.200419385 seconds. Workload rate is
0.011380701458 tokens/s over 702.944368538 seconds; setup is 734.921241644
seconds and whole run is 1557.722091593 seconds. The profile is operational-only
with baseline arithmetic, not cached admission or command sequences. This is
a correctness result, not a performance win. Comparison SHA256:
`e1babe489df564a1240679fd6062e148a60eac27039af3b6f47822d7d722e2bd`.

Later host controls use the same `d03089ba...` controller, public-902 runtime
source, baseline image, workload and operational profile. The intentional
worker programs differ: independent `189b918d...` versus peer `6891fb58...`.
Each comparison has one run per implementation, not new peer repetitions.

| World | Host Output (tokens/s) | Peer Output (tokens/s) | Peer Window / Host Window |
| --- | ---: | ---: | ---: |
| TP2 | 0.816745 | 0.146494 | 5.5753x |
| TP8 | 0.485641 | 0.011381 | 42.6724x |

The host controls strictly pass. Peer rates are 82.06%/97.66% lower at TP2/TP8.
Source inspection identifies repeated all-rank full-currentness fences and
serial child execution; this is a mechanism hypothesis, not a captured CPU
profile. No checks were weakened to make this path faster. Paired ledger hashes:
`d59696cb6616994677e42db34aab16cd33396a99a90a29f29965392b135f5a85` (TP2),
`886a3d29d7555cda2de47ab3ea305f0567bab235a7d1384ee2700b0259ab1ccc` (TP8).

### Matched TP1 Residual Pair

Both runs share controller `bc4a283b...`, worker `70572ff2...`, wave-v3 image
`306a27d8...`, baseline arithmetic and operational validation. Only the collective
changes; all eight expected tokens and bytes pass.

| Collective | Reuse TTFT (s) | Reuse TPOT (s) | Workload Output (tokens/s) | Setup (s) | Whole Run (s) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Host staged | 1.878017 | 1.178605 | 0.834914 | 103.902004 | 115.313311 |
| Device TP1 residual | 1.751118 | 1.167159 | 0.953393 | 102.418670 | 112.654805 |

This single pair observes +14.19% workload rate, -6.76% reuse TTFT and -0.97%
reuse TPOT. The last difference is especially small and is not established as
repeatable. Full per-request metrics and raw identities are retained in ledger
SHA256 `07cae3b84184cf8fa806a3ce452a2a6959b7ad3f16bf27587abf06e0e4d1d385`.

### Setup Transpose

The integrated tiled transpose preserves every BF16 bit, authenticated shard
range and upload extent. Its CPU-only 80-shape, three-repeat suite passes all
240 full-array comparisons. Sums of per-case medians improve 3.37x/3.36x/2.67x
for TP1/TP2/TP8 respectively. These are helper-only results, not whole-model
setup or inference measurements. See [the benchmark contract](../adapters/m1-engineering-execution-v1/tools/transpose_benchmark.md).

A later model pair changes only production transpose code (`bef12d57...` to
`c5d8cda0...` controller), with the same public-902 worker, full-v3 image,
operational checks, MFMA projection and fixed four-request workload. Both pass.
Setup falls from 173.194680 to 146.643121 seconds, saving 26.551560
seconds (15.33%). Whole run falls from 212.744133 to 183.431643 seconds.
Decode timings also vary, but a setup-only change does not justify attributing
that variation to a faster decoding algorithm. This is a single matched pair.
Ledger SHA256: `389c7b8ee88fb443e2f3396cb0978e92509142e58c3087321de43bc097d0b191`.

### Replica Allocation

This is a separate cache-off workload: eight named five-token prompts, eight
outputs each, 64 outputs and 96 processed rows in total. Each layout uses all
eight physical GPUs, the same `bef12d57...` controller, `189b918d...` worker and
baseline image. The strict common-RAW-clock checker verifies all outputs,
preselected identity/epoch, actual loaded payloads, disjoint device assignments,
owned-process cleanup and global idle observations. Cohorts do not overlap.

| Layout | Output (tokens/s) | Request-00 TTFT (s) | Request-00 TPOT (s) | Aggregate Row Budget | Loaded Host Model Bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1xTP8 | 1.441585 | 6.365368 | 4.625090 | 16 | 16,381,470,720 |
| 4xTP2 | 5.733503 | 2.142079 | 1.212193 | 64 | 65,525,882,880 |
| 8xTP1 | 6.587693 | 1.877833 | 1.119589 | 128 | 131,051,765,760 |

Rate is 64 divided by common release to last output, not a sum of replica
rates. Rows are budgeted at 16 per instance. Replication duplicates weights
and increases aggregate row capacity; these 3.9772x/4.5698x rate observations
are allocation-policy results, not isolated kernel gains. There is one sample
per layout. All eight request identities retain separate latencies and seven
decode intervals; the table selects request-00 rather than pooling them.
Payload bytes exclude KV, activation/scratch, allocator rounding and process
RSS. Ledger SHA256: `47d98300668c31db0dd8f91846a4d6a7b361e353c419d7d17a911c7625ec7b32`.

### Actual 32-Row Policy

The separate same-image TP8 cohort pair uses full-v5 image `98b5fdb1...`,
the same replica controller/worker and baseline arithmetic. Only row/chunk
budgets change. The completed row schedules are `[16,16,16,8,8,8,8,8,5,3]`
and `[32,14,8,8,8,8,8,8,2]`: an actual 32-row model batch, not just capacity.
Both match all 64 outputs and process 96 rows.

| Row/Chunk Budget | Output (tokens/s) | Request-00 TTFT (s) | Request-00 TPOT (s) |
| --- | ---: | ---: | ---: |
| 16 | 1.322906 | 6.511720 | 5.084949 |
| 32 | 1.489117 | 9.640646 | 4.414652 |

The larger budget raises the single-sample rate 12.56%, but request-00 TTFT
worsens. Other request identities remain separate in the full table. This
policy pair is not pooled with the earlier baseline-image allocation trio.
Ledger SHA256: `0bd49a1c66cb297e9eb20fd502ca5c99faefbaf722bd963de3acb96b5cc6fb6e`.

### Runtime Provenance

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
