# Qwen3-8B BF16 Decode: Measured Optimization Steps

These are actual **single-request, target-only Qwen3-8B BF16** observations on
Asrock's AMD Instinct MI350X (`gfx950`), not Qwen3-0.6B results. Every plotted
capture checks all 32 generated token IDs and decoded bytes against an independent,
SHA-pinned reference. **700 tokens/s is not achieved.** These are multi-dispatch
engineering executions, not a full-model megakernel or a SOTA comparison.

Each configuration has **one unwarmed request and 31 post-first-token intervals**.
CPU isolation and recording-overhead corrections are absent. Ratios below describe
these observations; they are not controlled causal contributions, repeated-run
speedup estimates, stable tail measurements or production qualifications.

## Fixed Workload

| Item | Checked value |
| --- | --- |
| Model | `Qwen/Qwen3-8B` |
| Revision | `b968826d9c46dd6066d109eabc6255188de91218` |
| Precision / target / concurrency | BF16 / target-only / one request, TP1 |
| Prompt | `The capital of France is` |
| Prompt tokens | `[785, 6722, 315, 9625, 374]` |
| Generated tokens / processed KV positions | 32 / 36 |
| Context capacity | 64 |
| Independent reference | Two matching BF16 SDPA greedy passes, generated earlier on MI300X |
| Reference SHA-256 | `1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094` |

Five token-at-a-time prompt forwards produce the first output, and 31 further
forwards produce the remaining outputs. Decode throughput is therefore:

```text
post_first_tokens_per_second = 31 / sum(the 31 inter-token intervals)
mean_TPOT = sum(the 31 inter-token intervals) / 31
```

It is not 32 divided by that duration, and it excludes first-token latency and
setup. The reference output is ` Paris. The capital of Italy is Rome. The capital
of Spain is Madrid. The capital of Germany is Berlin. The capital of the
Netherlands is Amsterdam. The`. Exact choices and bytes for this prompt are
evidence of bounded parity, not an error bound on every logit or model-wide quality.

## Legacy Runtime Ablation

These variants use the same target v1 controller, runtime worker, 14-root native
image, model, reference, prompt, precision and workload. They execute 19,584
dispatches each. The only configuration changes in this series are the named
runtime admission/currentness options; prefix reuse, quantization and speculation
are not enabled.

| Configuration | TTFT (s) | Mean TPOT (s) | Post-first tokens/s | Observed rate / first | Setup (s) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Baseline | 15.931709 | 3.191286 | 0.313353 | 1.000x | 197.600449 |
| Admission cache | 13.390205 | 2.686603 | 0.372217 | 1.188x | 197.974242 |
| Cache + operational | 4.461510 | 0.951921 | 1.050508 | 3.352x | 166.020448 |

| Added option, relative to the preceding row | Observed rate ratio | Observed mean TPOT change |
| --- | ---: | ---: |
| Admission cache | 1.188x | -15.81% |
| Operational currentness, retaining the cache | 2.822x | -64.57% |

These adjacent contrasts are descriptive, not isolated causal estimates. Their
percentages must not be added together or presented as statistically validated
optimization contributions.

![Actual target-model runtime decode rates](assets/asrock-target8b-ablations-v2/legacy-rates.svg)

![Every recorded target-model runtime decode interval](assets/asrock-target8b-ablations-v2/legacy-intervals.svg)

The interval plot starts at output token 2; token 1 belongs to TTFT. Its points
are actual host observations, not sampled or inferred GPU events. Source values
are retained in the [interval CSV](assets/asrock-target8b-ablations-v2/legacy-intervals.csv),
[baseline report](assets/asrock-target8b-ablations-v2/legacy-baseline-report.json),
[admission-cache report](assets/asrock-target8b-ablations-v2/legacy-cache-report.json),
and [operational-currentness report](assets/asrock-target8b-ablations-v2/legacy-operational-report.json).
The [observed contrasts](assets/asrock-target8b-ablations-v2/observed-contrasts.json)
calculate adjacent rate ratios and TPOT changes without treating them as additive
optimization contributions. Large initialization costs remain visible in the
table; the rate bars are not process-start-to-finish throughput.

### Cache Immutable Admission Work

`--runtime-cache-admission` retains kernel inspection/binding information already
validated when the worker loads its immutable owned image. Without this option,
the runtime repeats object validation and symbol binding during dispatch.
Caching avoids that repeated host work for an unchanged loaded kernel.

It **does not cache the KV prefix or skip each dispatch's dynamic checks**.
Buffer ownership, aliasing, argument values, launch geometry and queue state are
checked again. The source comment and dispatch path are in the pinned
[gfx950 engineering runtime](https://github.com/harsh-nod/fe2o3/blob/fe406b0c3275c33b92f81734a4e6e3852b891073/crates/fe2o3-kfd/src/engineering_gfx950.rs).
That distinction matters when removing host overhead: reusable immutable
validation and changing dispatch operands have different lifetimes.

### Use The Retained-Queue Currentness Contract

`--runtime-operational` selects the existing engineering operational fence for
non-lifecycle operations. Full topology/aperture checks remain required at
lifecycle boundaries. The shorter fence still checks process incarnation,
reset state, KFD/render descriptors, UAPI version, XNACK mode, and DRM identity or
VRAM-loss state. A failure poisons the retained token; it cannot silently recover.
This is **not a switch that disables all safety checks**. See the pinned
[device currentness implementation](https://github.com/harsh-nod/fe2o3/blob/fe406b0c3275c33b92f81734a4e6e3852b891073/crates/fe2o3-kfd/src/device_gfx950.rs).

No isolated attribution to any individual fence operation is possible from the
host token intervals. The experiment changes a named runtime mode and preserves
its existing numerical and lifecycle contracts.

## Paged Runtime: Separate Experiments

The paged batch controller is a different execution path. Its single-request
experiments must not be inserted into the legacy-runtime chart as though only one
option changed. Its fixed workload is one row, prefill chunk one, four 16-token
physical pages, context 64, disabled prefix caching and disabled output-head
pruning. Admission caching and operational currentness are enabled together.

The first completed capture checks all 32 target choices and decoded bytes. Its
19,584 dispatches use scalar projections and host-staged reusable reduction
buffers. It does not establish a speedup over the different legacy controller.

| Configuration | TTFT (s) | Mean TPOT (s) | Post-first tokens/s | Observed rate / first | Setup (s) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Baseline + host reuse | 5.953338 | 1.272268 | 0.785998 | 1.000x | 168.567387 |

![Actual paged target-model decode intervals](assets/asrock-target8b-ablations-v2/batch-intervals.svg)

The [paged baseline report](assets/asrock-target8b-ablations-v2/batch-baseline-host-report.json)
and [interval CSV](assets/asrock-target8b-ablations-v2/batch-intervals.csv) retain
the actual data. There is no comparison bar for this single configuration.
The techniques below describe source paths; rejected or unmeasured configurations
and pending native candidates are not plotted as successful results.

### Rejected Numerical Candidate

The subsequent `wave-host` capture changed only the projection mode while
retaining the same v3 image, controller, worker, host-reuse collective and other
options. It completed and closed its worker, but **failed the immutable 32-token
reference**:

| Check | Observed result |
| --- | --- |
| Matching initial output tokens | 12 |
| First differing output | Index 12, the 13th token |
| Expected token | `17689` (` Spain`) |
| Actual token | `9856` (` Germany`) |
| Numerical acceptance | Rejected |

No rate, interval plot or speedup from this rejected capture is published.
Successful worker shutdown and post-run idle/input checks do not override a
numerical mismatch. The original capture is retained with SHA-256
`fea7a82c7a83bbc06593d1e909e12b5c395bdeb15a2c882947dc1af18f7d78d1`;
the validator rejection log has SHA-256
`5136ce0f6f1a16d9d59a7d2daa33213491c538d3666cabae808863163e5e332e`.

Dependent wave/device-residual and wave/IPC-sequence captures were halted pending
diagnosis. The reference, token window and acceptance rule were not weakened.
Wave reductions change summation order, which is a reason to validate them, not
an established diagnosis of this particular mismatch.

### Cooperate Across Wave64 For GEMV

The [projection kernels](../device/qwen3-tp-perf-kernels-v3/src/projection.rs)
assign one Wave64 to a row/output-column dot product. Lane `l` accumulates
elements `l, l + 64, l + 128, ...` in FP32, and the subgroup combines the 64
partials. Adjacent lanes load adjacent BF16 weights from the unchanged row-major
layout. Only the designated output lane stores after the collective and finite
checks.

This exposes parallelism along K for batch-one projections and coalesces adjacent
weight accesses. It is a WaveGEMV path, not a claim that these runs use an MFMA
matrix tile. Floating-point reduction order differs from scalar accumulation,
so independent token/byte validation is mandatory even when shapes and dtype match.

### Keep The TP1 Residual On The Device

The [TP1 residual kernel](../device/qwen3-tp-perf-kernels-v3/src/collective.rs)
retains an output/down projection's FP32 partial, adds the BF16 residual converted
to FP32, and rounds the result to BF16 once. It preserves the existing arithmetic
order, including its initial FP32 zero addition and finite-value checks.

Device-side residual handling removes the corresponding host-staged transfer and
reduction boundary. It adds two GPU dispatches per layer: 616 instead of 544 per
forward, or 22,176 instead of 19,584 over the request. More dispatches can still
mean less host work, but only actual end-to-end measurements establish the net
effect. No cross-device collective is claimed by the TP1 kernel.

### Group Serial IPC Submissions

The existing `--dispatch-sequences` path groups bounded dependent dispatches in a
single worker command. This amortizes serial controller/worker communication;
the underlying kernels still execute with their required ordering and completion
checks. Packet counts do not decrease merely because command frames are grouped.

This is **not the ordered-AQL-batch path, GPU execution overlap, or a persistent
full-model megakernel**. Any reduction in IPC command count is a source-level
schedule property, not a measured hardware overlap duration. This experiment does
not enable queue rollover or relax the separately guarded ordered-batch mode.

### Retain Rejected Loop-Tuning Attempts

A further projection candidate tries to stop the FP32 partial's reduction loop
after its admitted K dimension rather than continuing empty iterations up to the
largest supported width. It is **not included in the measured charts** until a
native artifact and actual GPU reference check both succeed.

Three source variants failed native lowering and produced no runnable candidate
artifact. A divided dynamic loop bound failed the checked arithmetic proof for
`step * 64`. Adding an explicit maximum guard resolved that issue, but the dynamic
ranking header failed `FE2O3-PROGRESS-002`. Restoring a static header with a uniform
early break then failed the unique-header-exit requirement. All failures and
source snapshots are retained; no compiler gate, arithmetic policy or numerical
acceptance rule was relaxed. A source proposal or passing CPU test is not GPU
performance evidence.

A fourth formulation uses six explicit static loops for the admitted K values
512, 1536, 2048, 4096, 6144 and 12288. Their trip counts are respectively 8, 24,
32, 64, 96 and 192. Native compilation and ordinary artifact admission passed,
but this alone is not a candidate GPU result. In TP1, the attention-output partial
needs 64 useful iterations while the down projection still needs 192; the
per-lane FP32 operation order and final subgroup reduction remain unchanged.

## Theoretical Context, Not A Measured Bar

The [BF16 performance protocol](GFX950_DECODE_PERFORMANCE_V1.md#bf16-weight-streaming-bound)
derives an optimistic weight-streaming model from the pinned checkpoint:

```text
active_weight_bytes = 16,381,470,720 - 151,936 * 4,096 * 2 + 4,096 * 2
                    = 15,136,819,200 bytes/token
streaming_tokens_per_second = bandwidth_bytes_per_second / active_weight_bytes
```

The subtraction removes the full input embedding matrix from per-token traffic;
the final addition counts the one embedding row actually used. Dense decoder and
output-head weights remain in the count.

| Explicit bandwidth assumption | Ideal streaming tokens/s |
| --- | ---: |
| Installed-host advertised bandwidth: 6.810 TB/s | 449.90 |
| Public MI350X peak: 8.000 TB/s | 528.51 |

Neither row measures sustained bandwidth. Both assume no persistent weight-cache
credit, perfect streaming, and zero KV, activation, scheduling or compute cost.
They are conditional analytical bounds, not cache-independent physical limits.
At 700 tokens/s the same traffic model requires **10.596 TB/s for weights alone**.
Reducing dispatch overhead cannot, by itself, remove those bytes. The target
remains BF16 and target-only; quantization and speculative output are not substitutes.

The observed host rates are far below these analytical reference values. Dividing
a rate by a streaming bound is not a measurement of HBM utilization: the host
interval also includes checking, IPC, scheduling and kernel work. Separate GPU
profiling and measured sustained bandwidth are needed to explain that gap.

### Lossless Sampling Is Not A Runtime Result

A separate [CPU-only lossless feasibility study](assets/asrock-target8b-ablations-v2/bf16-lossless-feasibility.md)
sampled 17.2 MB across all 399 tensors without changing the admitted weights,
GPU kernels or BF16 numerical policy. Its sampled fixed exponent escape model
estimated 10.6858 GB of weight traffic per token. Dividing the two declared
bandwidth scenarios by that estimate gives 637.3 and 748.7 tokens/s **before any
codec, metadata, indexing, activation, KV, synchronization or compute cost**.

Those are not achieved compression ratios or decode rates. No packed format or
fused decoder is implemented by this study. Sample entropy is not a universal
bound on lossless representations, and unsampled exponent tails can increase
escapes. Any future codec must reconstruct every original BF16 word exactly;
substituting lower-precision values would violate this workload. The note records
the sampling policy, resource bounds, estimator limitations and required next
tests. Its analytical scenarios are deliberately absent from the measured charts.

## Reproduction And Limits

The [target-profile validator](../tools/target_decode_profile_v1.py) checks the
original exit status, exact ordered receipts, predeclared controller/worker/native
identities, frozen reference, token and byte parity, KV/dispatch counts, finite
timings and successful worker closure. The separate
[paged-target validator](../tools/target_batch_decode_v1.py) additionally checks
the exact request schedule and emitted page accounting. System idle/topology and
input-digest checks remain separately retained evidence, not deductions from a
close record.

[The plotting tool](../tools/target_decode_plots_v1.py) consumes only those
validated reports. It keeps runtime families separate, requires matching
controller/artifact/reference identities within a comparison, recomputes rates
from all intervals and publishes an explicit field whitelist. Regenerate into a
new directory, for example:

```sh
python3 -B tools/target_decode_plots_v1.py \
  --legacy baseline="$BASELINE_VALIDATED_REPORT" \
  --legacy cache="$CACHE_VALIDATED_REPORT" \
  --legacy operational="$OPERATIONAL_VALIDATED_REPORT" \
  --batch baseline-host="$BATCH_BASELINE_VALIDATED_REPORT" \
  --output "$NEW_PLOT_DIRECTORY"
```

Published [asset hashes](assets/asrock-target8b-ablations-v2/SHA256SUMS) bind the
normalized reports, charts, tables and CSV samples. Private device selectors,
process identifiers, machine paths and raw worker logs are excluded. No GPU
timestamps were collected: these graphs must not be relabeled as a kernel overlap
timeline. Matched repeated runs, numerical coverage beyond this prompt, isolated
baselines against tuned serving engines, calibrated GPU traces and a full-model
megakernel remain separate work.
