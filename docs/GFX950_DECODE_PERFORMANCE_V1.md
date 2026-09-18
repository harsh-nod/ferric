# gfx950 Decode Performance And Overlap

This is the experiment protocol for the next part of
[issue #42](https://github.com/harsh-nod/ferric/issues/42), not a 700 tokens/s
result. The implementation starts from Ferric M1 commit
`362ab57d8a0b81af138c7668188de07b9f4c6233`, with its existing gfx950 TP1
engineering model, artifact and worker owners. The finite scheduler results
on the earlier megakernel branch are separate experiments, not model timings.

Actual short-run results and host-timeline graphs are retained in the
[Asrock decode observations](GFX950_DECODE_OBSERVATIONS_V1.md). They are
Qwen3-0.6B engineering diagnostics, not the Qwen3-8B target result.
The separate [Qwen3-8B two-token smoke](GFX950_TARGET8B_SMOKE_V1.md) checks the
actual target model on Asrock, but is not a megakernel or performance result.
The [32-token target-model ablations](GFX950_TARGET8B_ABLATIONS_V2.md) retain
accepted numerical observations, rejected candidates and actual host-time plots.
The separate [native two-GPU dependency milestone](GFX950_NATIVE_TP2_DEPENDENCY_V1.md)
retains two rejected attempts and one accepted primitive canary. It is not a
tensor-parallel model result, dependency-latency benchmark, or overlap measurement.

## Execution And Numerical Scope

Start with the complete Qwen3-0.6B v10 paged path, then Qwen3-8B, followed by
North Mini Code. The first model profile retains the existing v10 BF16 body,
FP32 head, lowest-ID argmax, exact model/checkpoint intake and independently
pinned reference. It does not substitute a new model loader or reinterpret a
gfx942 production admission as gfx950. Scalar and MFMA projections are distinct
numerical profiles and each must pass its declared reference checks.

The v10 output/down projections accumulate in FP32, add the residual and then
round once to BF16. This is not the earlier foundation planner's separate
projection-rounding policy. Successful v10 execution does not establish parity
with a different rounding policy, a protected runner, or a megakernel.

The initial fixed prompt is `The capital of France is` (five authenticated
tokens). Short generation is a bring-up case, not evidence for the 1K/4K/8K
product matrix. Freeze an independently produced complete reference before
Ferric outputs are inspected. No guessed next token, candidate-generated
reference, retry of a failed numerical run, or omitted failure may become a
passing observation.

## What 700 Tokens/s Means

The user-selected target is **single-request, BF16, target-only Qwen3-8B
decode**, not aggregate throughput or Qwen3-0.6B. The current experiment uses
one GPU (TP1); single-request decoding does not itself require single-GPU
execution. Any tensor-parallel experiment must declare its GPU count and remain
a separate comparison. BF16 is the
required optimization contract, not merely a baseline to replace with a
lower-precision result. FP8, FP4, integer or weight-only quantization and
lower-precision KV-cache substitutions are out of scope. Preserve the existing
numerical policy, including its higher-precision accumulation where specified;
BF16 does not require rounding every intermediate operation to BF16.

Target-only excludes draft-model and speculative decoding. Each output token
must come from the target model's ordinary autoregressive decode; accepted
speculative tokens cannot count toward the target. The separate Draft06B
bring-up observations remain useful diagnostics, not a substitute workload.

At 700 tokens/s the mean post-first-token interval is
`1_000_000_000 / 700 = 1_428_571.43 ns`. This is a target line, not a measured
bar or promised outcome. Every comparison must retain the pinned checkpoint,
BF16 numerical contract and batch-one execution, and name live KV lengths and
the token-counting boundary. Do not relax these constraints to meet the target.

Report setup, prefill/first-token latency, post-first decode, complete resident
request time and teardown separately. End-to-end measurements must include all
work inside their declared request boundary. Device-only elapsed time excludes
host scheduling; host dispatch-to-completion time is not a GPU timestamp.
For multiple windows, divide total generated work by total measured duration,
not an average of per-window reciprocal latencies.

## BF16 Weight-Streaming Bound

For Qwen3-8B revision `b968826d9c46dd6066d109eabc6255188de91218`, the pinned
safetensors index records `16,381,470,720` tensor bytes. Its separate embedding
matrix is not read in full on each decode step. Counting one embedding row and
all dense decoder/output-head weights gives:

```text
active_weight_bytes = 16,381,470,720 - 151,936 * 4,096 * 2 + 4,096 * 2
                    = 15,136,819,200 bytes/token
ideal_streaming_tokens_per_second = bandwidth_bytes_per_second / active_weight_bytes
```

| Bandwidth assumption | Ideal streaming ceiling |
| --- | ---: |
| Public MI350X peak, 8.000 TB/s | 528.51 tokens/s |
| Installed Asrock MI350X advertised bandwidth, 6.810 TB/s | 449.90 tokens/s |

The public peak comes from AMD's
[MI350 architecture specifications](https://rocm.docs.amd.com/en/latest/reference/gpu-arch/mi350.html).
The installed-host value is `amd-smi static` telemetry, not measured sustained
bandwidth. Both rows assume no persistent weight-cache credit, perfect weight
streaming, and no KV, activation, scheduling or compute costs. They are an
analytical model, not measured throughput or a cache-independent physical limit.
Under these assumptions, uncompressed BF16 target-only decoding cannot reach
700 tokens/s. Reducing launch overhead alone cannot remove the weight traffic.
At 700 tokens/s, this model would require `15,136,819,200 * 700 =
10,595,773,440,000 bytes/s` (10.596 TB/s) for weights alone. The work therefore
aims to maximize measured BF16 target-only performance and explain the remaining
gap to a validated bound, not substitute quantization or speculative decoding.
Report a missed target honestly; change the bound only with measured evidence
for changed traffic or reuse assumptions.

A separate [CPU-only lossless feasibility study](assets/asrock-target8b-ablations-v2/bf16-lossless-feasibility.md)
estimates entropy and an exponent escape-code traffic model from deterministic
checkpoint samples. It implements no codec and measures no GPU gain. Those
estimates are neither achieved compression ratios nor universal bounds; they
do not change the admitted dense BF16 representation used by these experiments.

## Independent Reference

`tools/draft_decode_reference_v1.py` is a separate, CPU-only oracle for repeated
v10 bring-up. It does not change the earlier GPU-pinned reference. Its pinned
container, package versions and Qwen source hashes are checked before loading;
all model files are checked before and after execution. The original frozen
helper supplies checkpoint and tokenizer validation, not the old GPU policy.

The oracle uses a BF16 Transformers Qwen body with SDPA and an explicit FP32
linear head. Two fresh-cache executions must have identical complete FP32
logits and exact lowest-ID greedy choices at every scheduled forward. Raw
logits are retained; a separately emitted CPU model-admission identity is
required before adapting them to the runner's reference schema. This checks
token parity with an independent implementation, not elementwise equivalence
of every intermediate tensor or equivalence of CPU and GPU rounding.

Container execution needs no GPU devices or network. Limit CPU threads and
memory, provide a writable bounded `/tmp`, and set `HOME`, `USER` and `LOGNAME`
for the non-root container UID. Synchronous model loading and bounded BLAS
threads avoid exhausting the container's PID limit on a many-core host.

## Reproducing A Diagnostic

Use the existing [V10 artifact, source-bundle and worker prerequisites](DRAFT_PAGED_CANARY_V10.md).
The new profile does not accept a target-model image in place of the exact draft
image. Build its host executable with the adapter's `tp-batch-engineering`
feature; the existing CI toolchain and device-crate bootstrap policy still apply.
All paths and digests below are operator-supplied values fixed before GPU output
is observed, not values copied from a candidate's output record.

Inside the exact CPU-only reference environment described above, generate the
raw reference before running the candidate:

```sh
python3 -B tools/draft_decode_reference_v1.py \
  --producer-sha256 "$PRODUCER_SHA256" \
  --legacy-helper tools/draft_reference.py produce \
  --source "$PAIRED_SOURCE/draft" --output "$RAW_REFERENCE" \
  --image-id "$PINNED_IMAGE_ID" --prefill full --new-tokens 4
```

The CPU-only `draft_reference_identity` example takes the canonical paired
source directory as its sole argument. Retain its JSON output and digest, then
join that independent admission identity with the raw reference:

```sh
python3 -B tools/draft_decode_reference_v1.py \
  --producer-sha256 "$PRODUCER_SHA256" \
  --legacy-helper tools/draft_reference.py adapt \
  --raw-dir "$RAW_REFERENCE" --raw-sha256 "$RAW_SHA256" \
  --identity "$IDENTITY_JSON" --identity-sha256 "$IDENTITY_SHA256" \
  --output "$REFERENCE_JSON"
```

After reserving the admitted device and retaining idle/environment, binary,
source, artifact and reference checks, run one bounded observation:

```sh
"$PROFILE_BIN" --source "$PAIRED_SOURCE" --artifact "$DRAFT_ARTIFACT" \
  --worker "$WORKER" --worker-sha256 "$WORKER_SHA256" \
  --device-unique-id "$PRIVATE_DEVICE_ID" \
  --projection mfma --prefill full --new-tokens 4 --warmups 0 --samples 1 \
  --reference "$REFERENCE_JSON" --reference-sha256 "$REFERENCE_SHA256" \
  --runtime-cache-admission --runtime-operational \
  --allow-unauthenticated-machine-code
```

Retain complete stdout/stderr, exit status, post-run identity/idle checks and
every failure without retrying a failed observation. The explicit engineering
consent flag does not grant protected runtime admission. For the baseline omit
both runtime options and choose `--projection baseline`; the intermediate
variants change only the options named in the observations table. Use the
[report manifest and validation commands](../tools/qwen-decode-report/README.md)
to produce the sanitized JSON, timing plot and matched-workload comparison.
Do not publish private device selectors or process identifiers.

## Frozen Comparison

For each numerical profile, retain at least ten warmups and thirty recorded
samples before claiming a benchmark result, following [PERFORMANCE.md](PERFORMANCE.md).
Shorter diagnostic captures remain visibly non-qualified. Keep all raw token
choices, consumed positions, dispatch counts, failures and cleanup outcomes.
Bind every capture to exact controller, worker, compiler, source, artifact,
model, reference, workload and environment identities. Freeze those identities
and the execution order before observing a candidate.

Compare equal workloads on the same GPU against the completion-ordered Ferric
path and applicable tuned ROCm serving baselines. Measure dispatch consolidation,
ready-task scheduling, tile/worker tuning, prefetch, software pipelining and LDS
multi-buffering as separately named variants. Preserve losing variants. Include
leave-one-out ablations when changes interact; percentage gains are not additive.
Represent runtime changes explicitly in the experiment policy and validate
them independently. Precision changes are excluded by the BF16 target-only
contract, even if they would improve the reported rate.

The draft profile keeps the existing no-rollover packet budget. Ten warmups and
thirty samples at four full-prefill output tokens use `40 * 4 * 480 = 76,800`
packets and fit; eight tokens would exceed the 131,072-packet budget and are
rejected before spawning a worker. This short diagnostic does not qualify a
long-context or long-generation workload.

## Timeline Evidence

A Cohere-style overlap plot requires actual GPU task intervals, not a drawing
of the planned DAG. Each record needs dispatch/epoch, task and operation IDs,
worker identity, phase, begin/end ticks and the exact instrumented artifact.
Retain phase boundaries for dependency waiting, loads, compute, stores and
publication only when they are actually observed. A broad task interval alone
does not prove simultaneous MFMA execution or HBM traffic.

Clocks have explicit domains, widths, rates and calibration evidence. Do not
overlay unsynchronized per-CU/XCC counters, truncate 64-bit counters, assume
clock32 cannot wrap, or treat the KFD host timestamp frequency as GPU frequency.
Counter observations are not memory-completion fences. Required synchronization
and optimizer ordering must be established separately for each recorded phase.
Reject dropped or malformed records and mismatched artifact/capture identities.

Instrumented traces explain scheduling; uninstrumented runs measure throughput.
Measure tracing overhead with paired otherwise-identical runs. Keep host
timelines separate and label them as host observations. A plot must say when
only host timestamps exist rather than displaying inferred GPU overlap.

The separate [checked gfx950 `realtime64` lowering](https://github.com/harsh-nod/fe2o3/blob/7deace1d8666250c1a14cde4b8a7d8f494052309/docs/semantic-realtime-v32.md)
provides an ordinary Rust source path for full-width diagnostic observations.
It is a compiler prerequisite, not a completed trace collector, calibrated
clock, memory-completion fence, or evidence that any model tasks overlapped.
The retained model observations here use the pristine earlier compiler and do
not use that timer extension.

## Remaining Gates

Refine the streaming estimate with measured sustained bandwidth, compute
ceilings, required KV/activation/descriptor bytes and the actual dependency
graph. State cache and reuse assumptions. Do not extrapolate a model rate from
one optimized GEMM.

Complete model numerics, a progress-safe tensor scheduler, valid GPU trace
capture, equal-work baseline measurements and production qualification remain
separate gates. None is supplied by this protocol or by a plotting tool.
