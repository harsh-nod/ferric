# Opt-In Target Argmax Canary

`ferric-qwen3-argmax-canary` is a separate, non-authoritative target-only
comparison. It does not change the existing batch or paired canary CLIs,
enable speculative serving, or change default serial argmax execution.

Both `serial` and `wave-v11` load the same v5 target, v8 FP32 head, and
separate one-root v11 image before allocating device buffers. The new
`new_wide32_with_argmax_v11` constructor validates and retains the exact
loaded v11 binding while the worker is still fresh. Later opt-in selection
checks that retained private binding; it does not weaken the worker's
pre-allocation image guard. The v8 projection, BF16 operands, FP32 logits,
choice buffer, selected-row order, and packet counts remain unchanged.

## Fixed Workload

The CLI requires `--source`, `--target-artifact`, `--target-head-artifact`,
`--argmax-artifact`, `--argmax-mode serial|wave-v11`,
`--max-new-tokens 8|128`, `--worker`, `--worker-sha256`,
`--device-unique-id`, `--reference`, `--prefix-reference`, and
`--host-timing-output`. Explicit unauthenticated-machine-code consent,
runtime admission caching, operational checks, and queue rollover are
mandatory. Every other option, duplicate, or missing flag is rejected.

The fixed ordinary target TP1 profile uses MFMA projection, baseline
attention, FP32-v8 output, 32-row workspace capacity, 16-token prefill chunks,
context 256, and 16 physical pages. Prefix caching, wave attention, ordered
batches, sequences, peers, large-KV routing, and numerical capture are not
supported. This isolates the head selector rather than claiming comparison
with the fastest available complete serving profile.

The original 128-token prompt and 128-output target oracle are byte-pinned
to `cd7f512537e44d677244c606cba919e57747e8c26caf3c0df4a0aca7acc0eb8b`.
The unchanged seven-field prefix reference is separately pinned to
`e5588cefc924a8be5b77345876b6893d53436213de03108a7fc4702cc86356a0`.
Eight-output decoding uses its independently decoded eighth prefix; it
never truncates UTF8 by a token count. Later inputs are actual GPU choices,
not reference tokens. All generated IDs and decoded bytes must match.

Seven prefill batches omit the output head. The eighth computes the head
for selected row 15 only, followed by one-row autoregressive decode batches.
Expected totals are 9,219 packets and 15 batches for eight outputs, or
83,139 packets and 135 batches for 128 outputs. Resident input counts before
retirement are 135 and 255; the last generated token is not consumed.
The completed sequence retires without publishing a cached prefix.
Retirement requires zero live sequences, retained/cached/quarantined pages,
all 16 pages free, a stale sequence handle, and valid pool invariants. It
does not reset monotone sequence or batch identities: `is_empty()` remains
a never-used admission guard, not a post-retirement ownership predicate.

## Evidence Boundaries

Four successful JSONL records describe setup, all eight prefill steps,
actual decode steps and outputs, and normal worker closure. Any submitted
failure quarantines its batch; all configuration or observation failures
still attempt worker closure. Sidecar creation is exclusive, with mode 0600.

Per-token elapsed offsets start immediately before prefill and stop after
each generated token's batch commit. They exclude model/worker setup and
are not HTTP latency. The sidecar retains the existing `output_head` total
and adds nested normalization, projection, and argmax host scopes. These
include host submission and waiting, overlap their parent totals, and are
not GPU kernel durations. Disabled instrumentation leaves dispatches
unchanged and does not sample clocks.

Host tests check closed admission, selection ordering, row/grid bounds,
unchanged command traces, failure poisoning, and timing scope behavior.
Recording transports are not kernel emulators. Native kernel fixtures,
full-model parity, and matched timing trials require separate source- and
image-pinned receipts; this document makes no native success or speedup claim.

The first native 128-output serial attempt at source `f662d54` completed
83,139 packets and 135 batches but failed a controller assertion that
mistakenly reused the fresh-pool guard after retirement. The worker closed
normally and all eight GPUs were idle afterward. That attempt is retained
as rejected, with no published observation or admitted parity result. The
correction changes only the post-retirement check; metadata regressions
exercise actual reserve/begin/commit flows for 135 and 255 resident inputs,
and the canary's retirement helper rejects pending or quarantined work.
