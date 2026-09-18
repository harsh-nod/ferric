# Target-Only BF16 Decode Checker

`target_batch_decode_v1.py` validates one real, single-request Qwen3-8B capture
from `ferric-qwen3-tp-batch-engineering`. It is a separate observation policy;
it does not change existing TP8, prefix-cache, cancellation, replica or ordered
batch qualification checkers. Its tests use synthetic records and are not
numerical evidence.

## Fixed Work

- Qwen3-8B pinned revision `b968826d9c46dd6066d109eabc6255188de91218`.
- BF16 target-only, one GPU, one request, no quantization or speculation.
- Prompt `The capital of France is`, exactly five prompt tokens.
- Two-token smoke or full 32-token reference continuation.
- `v3-wave` image, `baseline` or `wave` projection, baseline attention.
- One scheduled row and one-token prefill chunk, context 64, four 16-token pages.
- Prefix cache and output-head pruning disabled; TTL 1024.
- No ordered batches, queue rollover or runtime counters.
- Dispatch sequences are an explicit fourth candidate, only for wave projection
  with `device-tp1-v3`; the other candidates leave them disabled.
- Existing baseline, reused host staging, or `device-tp1-v3` residual path.

The 32-token case processes 36 KV tokens, requiring three physical pages.
It emits 36 completed batches and executes 19,584 host-residual dispatches or
22,176 device-residual dispatches. Four physical pages leave one unused page.
`max_batches` must be exactly 36, or six for the two-token smoke.

The current controller deliberately rejects ordered batches for `v3-wave`.
Do not remove that guard to make an experiment fit this checker.

The separate `--dispatch-sequences` option is already supported. It sends the
ten attention-stage or five FFN-stage dispatches in one IPC message while the
worker still executes each kernel synchronously. Each device residual remains a
separate completed dispatch before the hidden-state buffer swap. For this exact
profile, controller dispatch messages are analytically reduced from 616 to 148
per forward: 72 sequences, 72 residuals and four embedding/head singletons.
The 616 GPU packets and their arithmetic/order are unchanged. This count is a
source-derived expectation, not observed IPC telemetry or GPU overlap evidence.
The independent plan and actual Setup must both declare `dispatch_sequences` as
true to validate that candidate; the checker never normalizes it away.

## Independent Plan

Create and freeze an expectation **before capture**, independently of the output
being checked. The exact fields are:

```json
{
  "schema": "FerricTargetBatchDecodeExpectationV1",
  "controller_sha256": "<actual controller SHA-256>",
  "worker_sha256": "<actual worker SHA-256>",
  "artifact_hsaco_id": "<admitted HSACO SHA-256>",
  "artifact_manifest_id": "<admitted manifest SHA-256>",
  "artifact_handoff_id": "<admitted handoff identity>",
  "device_unique_id": 1,
  "request_name": "target-single-request",
  "new_tokens": 32,
  "kernel_profile": "v3-wave",
  "collective": "device-tp1-v3",
  "max_batches": 36,
  "cache_ttl_ticks": 1024,
  "host_timing_enabled": false,
  "performance_profile": {
    "runtime_cache_admission": true,
    "runtime_operational": true,
    "dispatch_sequences": false,
    "queue_rollover": false,
    "projection": "wave",
    "attention": "baseline",
    "runtime_profiling": false
  }
}
```

The example identity values are placeholders, not authorization or usable
artifacts. The selected physical device identifier and raw captures stay private.
Kernel profile and trace-enabled declarations are externally pinned plan fields;
the batch Setup record does not itself repeat those two declarations. Preserve
the exact invocation and artifact-admission evidence alongside the plan.

The workload must contain exactly:

```json
{
  "schema": "FerricQwen3TpWorkloadV2",
  "requests": [{
    "name": "target-single-request",
    "prompt": "The capital of France is",
    "new_tokens": 32,
    "arrival_tick": 0
  }]
}
```

## Validate

```bash
python3 -B tools/target_batch_decode_v1.py \
  --capture results.jsonl --status status \
  --workload workload.json --reference reference.json \
  --expect expectation.json --output report.json
python3 -B -m unittest discover -s tools -p test_target_batch_decode_v1.py -v
```

The controller status file must contain `0` followed by a newline. The frozen
reference must match SHA-256
`1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094`.
That independent reference was produced earlier on MI300X, with two matching
32-token passes. The two-token smoke validates only its first two choices.

The checker rejects extra/missing records, changed schema keys, unknown options,
wrong device or executable identities, different token choices or bytes,
incorrect KV/page/dispatch schedules, broken timing arithmetic and unsuccessful
worker closure. It publishes only whitelisted results, digests and host timings;
raw device selectors, PIDs, session IDs and private paths are not copied.

## Measurement Limits

Each capture is one unwarmed request, even if other independent invocations exist.
Report post-first rate as `(output_count - 1) / sum(decode_intervals)`. Do not
include the first token in that numerator, pool setup into decode, report
aggregate throughput as single-request speed, or label 31 token intervals as
31 independent requests. Compare variants with the same work and instrumentation,
including repeated baseline controls before attributing speedups.

Host completion receipts include runtime, IPC and between-batch JSON logging.
They are not GPU durations or overlap evidence. Setup and whole-controller time
remain visible. Trace overhead requires a separate matched trace-off experiment.

Completed batch page counts precede request retirement. A Request retirement
record and successful worker-close record are checked, but the controller does
not emit final pool counters. The checker therefore does **not** claim an
independently observed final free-page count or system-wide GPU idleness.
Record pre/post idle and owned-process checks separately. No benchmark, serving,
protected admission, full-model megakernel or 700 tokens/s claim follows merely
from successful validation.
