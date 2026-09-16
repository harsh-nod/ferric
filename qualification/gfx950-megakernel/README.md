# gfx950 Megakernel Engineering Measurements

This directory implements the **measurement-contract portion** of
[Ferric #42](https://github.com/harsh-nod/ferric/issues/42). It does not implement
the decode backend or qualify a release. No hardware result, model output,
throughput number, or passing evidence fixture is checked in here.

The checker is deliberately separate from the gfx942 M1 evidence protocol.
It accepts one exact workload cell, authenticates the supplied files, checks
the declared numerical comparisons, and recomputes timing and ablation
arithmetic. File authentication does not independently attest execution,
correctness, provenance truth, or completeness of the submitted sample log.
The collector and comparator remain explicit trusted inputs. In particular,
`passed: true` in a numerical transcript is a checked declaration, not a new
independent numerical evaluation performed by this checker.

Every generated report has `qualification: false` and
`public_faster_claim: false`. `device-smoke`, `single-layer`, and `full-decode`
are distinct scopes; a smoke observation cannot be relabeled as model decode.
Passing this command does not close a milestone or promote a Ferric assurance
status. Production promotion still requires the issue's compiler, model,
numerical, scheduler, lifecycle, and release gates.

## Run

```sh
python3 -I qualification/gfx950-megakernel/test_validate.py
python3 -I qualification/gfx950-megakernel/validate.py EVIDENCE_ROOT measurements.json
```

The second command emits a canonical JSON report on stdout, or exits nonzero
with `REJECTED` on stderr and no report. It never launches a GPU workload,
downloads weights, changes clocks, rewrites input files, or discards failed
samples. Capture the report outside the input directory if desired. Keep the
original collector log, including failures; the checker rejects timing when
any supplied run has a numerical failure or recorded fault.

Tests create visibly `SYNTHETIC` artifacts in a temporary directory, compute
their real file hashes, and remove them afterward. They exercise the contract
and arithmetic only, not source compilation, a device, or an ML model.

## Input Contract

Input JSON is ASCII, sorted-key, two-space-indented JSON with exactly one final
newline. Every object has an exact field roster. Duplicate keys, floating
point values, NaN/infinity, missing or extra fields, and Boolean values where
integers are required are rejected. Timing is integer nanoseconds; ratios in
the output use exact numerator/denominator objects.

Every artifact pin is exactly `{path, sha256, size_bytes}`. `path` is relative
to the evidence root and cannot contain traversal, redundant components,
symlinks, or a hard-linked final file. The checker uses descriptor-relative
opens and compares file identity and metadata before and after each read.
Files and documents are limited to 8 MiB each. Model/weights/source provenance
must therefore use a small authenticated roster or source-closure document,
not embed model weights or an entire source tree. Those rosters must identify
the actual revisions, files, sizes, and digests; this checker pins the roster
bytes but does not walk the model checkpoint or re-prove a source closure.

### Measurement Document

The top-level exact field roster is:

| Field | Meaning |
| --- | --- |
| `format` | `ferric.gfx950-megakernel.measurements.v1` |
| `target` | `gfx950`, exactly one device |
| `scope` | `device-smoke`, `single-layer`, or `full-decode` |
| `authority` | `engineering-observation-only` |
| `qualification`, `public_faster_claim` | Both `false` |
| `case` | Exact model, shape, precision, and timing boundary |
| `provenance` | Pinned model/source/workload/environment roster |
| `variants` | Two through sixteen independently identified variants |
| `runs` | Complete paired run rows, including warmups |
| `ablations` | Zero or more single-optimization comparisons |
| `interactions` | Zero or more matched two-factor experiments |
| `bound` | A modeled bound declaration, or `null` |

`case` has exactly `model`, `batch`, `context`, `precision`, `accumulation`,
`metric`, and `output_tokens`. The first envelope is Qwen/Qwen3-0.6B or
Qwen/Qwen3-8B, batch 1/2/4/8, context 1024/4096/8192, BF16 with FP32
accumulation, `decode-step-latency-ns`, and one output token per request.
Lower scopes describe subcomponents of that model cell, not actual model
execution. Other shapes, precision policies, serving metrics, or longer output
windows need a separately reviewed protocol revision; do not silently coerce
them into this contract.

`provenance` contains exactly these pins:

```text
ferric_source fe2o3_source model tokenizer weights numerical_policy
environment workload tuning_budget
```

The source rosters should bind commits and source-closure digests. The
numerical-policy roster must contain thresholds declared before timing, not
thresholds selected after inspecting a candidate's errors. The tuning-budget
roster must identify equal calibration budgets, held-out inputs, and the
configuration search for all compared engines. The checker enforces identical
budget and workload pins, not truth or sufficiency of their declarations.

`workload` is itself canonical JSON with exactly `case`, `scope`,
`measurement_boundary`, `prompt_trace`, and `seed`. The first two exactly
match the outer document; the boundary is `submit-through-exact-completion`.
`prompt_trace` pins the actual input/arrival/order/output-limit trace and
`seed` is a nonnegative integer. Include reset, embedding, logits/argmax, and
completion/commit costs in the declared timing boundary where the requested
scope includes them; kernel-only time is not whole-token latency.

`environment` is canonical JSON with exactly `target`, `device_uuid`,
`device_count`, `host`, `software`, `hardware`, and `policy`. The latter three
are pins for software identities (ROCm/LLVM/driver/firmware/engine versions),
hardware/topology identities, and clock/power/thermal/CPU/NUMA/affinity/cache
policies. Record the allocated physical GPU UUID, not only a remapped ordinal.
Do not reset other users' GPUs or change shared-machine settings to collect it.

### Variants And Numerical Observations

Each variant contains exactly:

```text
id role scope optimizations executable build_manifest numerical_report
environment_sha256 workload_sha256 tuning_budget_sha256
```

IDs and optimization names are unique lowercase names with digits/hyphens.
Exactly one variant has role `ferric-command-batch` and one `megakernel`.
Other roles are `ablation`, `vllm`, or `sglang`. Match the semantic work, exact
checkpoint, precision policy, KV layout/policy, timing boundary, prompt trace,
and tuning budget. Use the best admitted command-batch implementation, and
enable applicable graph/replay optimizations on external engines. Supplying
an intentionally weak baseline is not prevented by hashing it.

`executable`, `build_manifest`, and `numerical_report` are file pins. Pin the
actual device artifact for a Ferric variant; the build manifest must bind its
compiler arguments, production Rust-to-HSACO route, resource metadata, target
features, and proof/admission identities. For an external engine, pin its
launch executable and include all loaded libraries and container/package
identities in its build manifest. This checker authenticates these bytes; it
does not disassemble HSACO or establish compiler correspondence.

The canonical numerical report has exactly:

```text
format target scope passed comparison tests failed_tests executable_sha256
workload_sha256 model_sha256 weights_sha256 numerical_policy_sha256
reference_implementation
```

`format` is `ferric.gfx950-megakernel.numerical.v1`, `comparison` is
`independent-reference`, `tests` is positive, `passed` must be true, and
`failed_tests` must be empty. All digests and scope must match the compared
variant and provenance. `reference_implementation` pins the independently
implemented comparison source/closure. Its output should retain the actual
per-tensor errors, logit margins, accepted tolerances, and token checks in
the original evidence archive. This adapter does not replace those detailed
reports or decide their numerical acceptance policy.

### Paired Measurements

`runs` has at least 10 ordered warmups followed by at least 30 recorded rows,
and at most 4096 total rows. Each row has exactly `phase`, `ordinal`,
`engine_order`, and `values`. Ordinals start at zero within each phase and
cannot skip or repeat. The engine order is the variant roster rotated by
`ordinal % number_of_variants`, so every engine receives each order position.
Pair the same input for all variants within a row.

`values` has one object per variant ID. Each sample contains exactly:

```text
latency_ns clock_khz temperature_millicelsius passed faults
numerical_report_sha256 executable_sha256
```

No zero/negative latency, identity drift, nonempty fault list, or failed sample
is admitted. The same numerical artifact binds each sample. The checker does
not manufacture a numerical report for an otherwise unverified timing run.
Collectors must verify outputs before timing promotion and retain failures.

The checker follows [the performance policy](../../docs/PERFORMANCE.md):

- Exact rational medians and nearest-rank p90/p99 latency.
- 2048 deterministic paired bootstrap resamples, with speedup defined as
  `median(control latency) / median(candidate latency)`.
- The standard-library `random.Random` seed is `0xF320260915`. Each sampled
  index uses `randrange(count)`, including rejection sampling rather than
  low-bit modulo reduction. Both engines use the same sampled indices.
  Record the Python version in software provenance for reproduction.
- The sorted bootstrap entries at `floor(0.025*N)` and `floor(0.975*N)` form
  the reported 95% interval. Method and seed are versioned output fields.
- Reject latency range/median above 2% and clock or temperature range/median
  above 3%. These checks use raw values, not a supplied summary.
- A 0.95 lower bound passes the engineering nonregression comparison; a
  strictly greater than 1.05 lower bound passes the engineering faster
  threshold. Neither result authorizes a public faster claim.

This is a paired **decode-step engineering protocol**, not the three-fresh-
server-start serving protocol. It has no serving-SLO, suite-wide weighted
geomean, production throughput, or SoTA promotion gate. If external baselines
are supplied, it compares against the fastest *measured* external baseline,
not an asserted globally fastest implementation.

### Ablations And Interactions

Each `ablations` entry has `control`, `candidate`, and
`changed_optimization`. The candidate's optimization-name set must equal the
control's set plus exactly that one optimization. Every endpoint must have its
own artifact/numerical pins and paired samples. The report includes exact
median nanoseconds saved, speedup, and paired confidence bounds.

Suitable experiments include dispatch consolidation, dependency scheduling,
ready-queue balancing, 4-wave versus 8-wave tiling, tile size, software
pipelining, LDS double buffering, prefetch distance, and separate GEMV/MFMA
handlers. A name does not establish that the implementation actually changed
only that feature; retain source diffs and resource/disassembly evidence.

An `interactions` entry has `control`, `a`, `b`, and `combined`. The checker
requires the four optimization sets to form the exact two-factor experiment:
baseline, baseline+A, baseline+B, and baseline+A+B. It reports:

```text
additional_saving = median(A) + median(B) - median(baseline) - median(A+B)
```

A positive value means extra savings beyond the sum of the independent
changes. This is a descriptive median interaction, not a significance test.
Sequential improvements are conditional on the preceding implementation;
do not add their speedups or attribute an interaction to one feature. Empty
interaction experiments remain unmeasured, not zero. Report slower variants
and rejected experiments alongside successful ones in the eventual tutorial.

### Modeled Bound

`bound` is optional. If present it has exactly `minimum_hbm_bytes`,
`operations`, `sustained_bytes_per_second`, `sustained_flops_per_second`,
`critical_path_ns`, `calibration`, and `derivation`. All numbers are integers;
rates must be positive. The last two fields pin the measured calibration and
the written operation/traffic/dependency derivation for this exact workload.

```text
modeled_floor_ns = max(
    minimum_hbm_bytes * 1e9 / sustained_bytes_per_second,
    operations * 1e9 / sustained_flops_per_second,
    critical_path_ns)
```

Derive minimum traffic from the actual active weights, dtype and scale bytes,
KV reads/writes, activations, and explicit cache assumptions; do not charge a
weight matrix once per batch member if it is reused. Derive FLOPs from the
actual projection/attention/MLP graph. Separate precision-dependent compute
ceilings and dependencies in the derivation. Max, not sum, avoids double
counting potentially overlapped compute and memory time.

Measured sustained rates are modeling assumptions, not physical hard upper
bounds. Accordingly, output says `physical_lower_bound_proved: false` and
reports the floor/observed-latency ratio without clipping. A ratio above one
calls the traffic, cache, calibration, or overlap assumptions into question;
it sets `inconsistent_bound_requires_audit: true` and is not super-peak
hardware performance. Actual architectural peak bounds
and a reviewed critical-path model are additional evidence, not supplied by
this checker.
