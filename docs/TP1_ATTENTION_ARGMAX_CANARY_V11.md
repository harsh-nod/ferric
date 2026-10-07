# Opt-In Attention Plus Argmax Canary

`ferric-qwen3-attention-argmax-canary` compares baseline and wave paged GQA
with v11 argmax fixed. It is a separate engineering experiment, not a default,
serving endpoint, or numerical/performance qualification.

## Closed Profile

The CLI requires `--attention baseline|wave` and every option required by
`ferric-qwen3-argmax-canary`, including explicit `--argmax-mode wave-v11`.
Serial argmax, missing or duplicate options, and additional policies are rejected.
The original argmax CLI still accepts serial or v11 argmax with baseline attention
only, and still rejects `--attention`.

Both arms use the same v5 target, v8 FP32 head, separately admitted v11 image,
MFMA projection, TP1, 32-row workspace, device TP1 residuals, output-head pruning,
context 256, 16 physical pages and 16-token prefill chunks. Runtime admission
caching, operational checks and rollover are required. Ordered batches, legacy
sequences, peers, large KV, numerical capture and prefix reuse remain excluded.

Every image is loaded before allocation through the existing constructor. The
baseline arm uses the unchanged `configure_fp32_argmax_v11` selector. The wave
arm configures wave attention before the v8 head, then uses the distinct
`configure_wave_attention_fp32_argmax_v11` selector. This selector requires MFMA
and pruning in addition to the existing binding, ownership and lifecycle checks.
Neither selector permits later attention or execution-policy changes.

The shared runtime retains the old execution, actual-choice feedback, reference
validation, retirement and worker-close behavior. New records use exactly
`FerricAttentionArgmaxCanary{Setup,Prefill,Observation,Closed}V1`; their fields
are unchanged apart from these schema labels and the truthful setup `attention`
value. `argmax_mode` is always `wave-v11`. The old record schemas and stderr
prefix remain unchanged for the original entry point.

## Validation Contract

The unchanged `FerricArgmaxCanaryWorkloadV1` hash, original target reference
`cd7f512537e44d677244c606cba919e57747e8c26caf3c0df4a0aca7acc0eb8b`, and prefix
reference `e5588cefc924a8be5b77345876b6893d53436213de03108a7fc4702cc86356a0`
remain required. The 128-token prompt uses eight prefill batches, with the head
omitted in the first seven. Decode consumes actual generated choices. Eight
outputs require 9,219 packets/15 batches; 128 outputs require 83,139 packets/135
batches. Every generated token ID and decoded byte must match; a mismatch cannot
be accepted by replacing the oracle.

Host recording tests compare rows 1/16/17/32 and empty, last-only, sparse and all
selected outputs. Only the 36 GQA root names per batch may differ; arguments,
grids, order, packet counts and allocations must match. Separate tests retain
wrong-binding and unsupported-profile rejection, immutable configuration,
submitted-failure poisoning, quarantine, and host-span command equivalence.
Recording transports are not numerical kernel emulators.

Native finite-input fixtures, unchanged-reference eight-output smoke, and full
128-output comparisons require separately reviewed launches and receipts. The
wave QK tree reduction changes FP32 addition order, so composition must pass the
unchanged oracle before any timing comparison. No deliberate trapping fixtures
belong on the shared GPU host. Matched timing must retain the same binary, images,
runtime flags and instrumentation in both arms. Host spans include submission
and waiting, overlap their parent scopes, and are not GPU kernel durations.
