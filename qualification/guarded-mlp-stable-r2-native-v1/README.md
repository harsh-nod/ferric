# Stable Guarded R2 Native Diagnostic

Status: **CPU qualification and native GPU component execution pass on MI350.**
This is a private serial component diagnostic, not a paired scheduler, model
correctness result, production capability, or performance claim.

The diagnostic connects the [checked gfx950 image](../guarded-mlp-handoff-capture-v1/README.md)
to the [private combined-state owner](../guarded-mlp-combined-state-owner-v1/README.md).
It does not relax generic SharedAtomic admission. All issue #42 milestones
and the 700 tokens/s Qwen3-8B target remain open.

## What Changed

Four runtime source postimages are retained under
[the actual CPU attempt](cpu-attempt-v1/retention-manifest.json). Three existing
files change and one ignored native-test module is added. The rest of the
qualified runtime source map is unchanged.

The native diagnostic uses one 2208-byte allocation per rank containing 552
genuine atomic words. A test-only quiescent seed helper updates those existing
objects without constructing an ordinary byte view or recreating atomics.
It is not an owner submit, completion, rearm, or quiescence proof.

Both guards remain stable during each synchronous R2 dispatch. That premise
is explicit because the [actual image uses a wave-wide guard branch](../guarded-mlp-atomic-load-alias-lowering-v1/IMAGE-REVIEW.md).
The test cannot establish mixed-lane safety under concurrent guard updates.

## Actual Native Matrix

| Coverage | Count |
| --- | ---: |
| Generations, including a nonzero high word | 2 |
| Cases per generation | 29 |
| R2 dispatches, both ranks | 116 |
| Valid R2 outputs | 4 |
| Rejected R2 outputs | 112 |
| Actual validator dispatches | 76 |
| Total GPU dispatches | 192 |

Cases cover valid state, stale low/high generation words, pending and failed
verdicts, nonzero reserved words, and nine prefix corruption boundaries per
rank. This is not a native sweep of all 548 prefix words.

Each valid output contains 4096 BF16 values. The corpus includes cancellation,
signed zero, subnormals, overflow-adjacent finite values and rounding ties.
Both outputs are reset before each serial R2 call; the target must match all
expected bytes and canaries while the other output remains untouched. Inputs
and all combined-state words are also checked after dispatches.

The in-test reference independently performs staged FP32 and BF16 rounding.
A separate integer-only verifier reconstructs every input bit and compares
all four valid outputs. Neither reference is a full-model numerical check.

## Actual GPU Evidence

The [native receipt](gpu-attempt-v1/evidence/complete.json) records one requested
and observed native process, with no retries. Eight phases exit naturally
with status zero; all children are reaped and their process groups are absent.
The request, checked HSACO, CPU-built host ELF, source bodies and runtime
libraries are bound to the run, with 859 final input pins and no postcheck
errors. Read-only telemetry reports all eight GPUs idle before and after;
the native process reports a healthy queue-first close.

The [actual native output](gpu-attempt-v1/evidence/native.stdout) and
[independent verification](gpu-attempt-v1/evidence/verify.stdout) establish:

| Check | Actual result |
| --- | --- |
| Valid outputs | 4/4, all 16,384 BF16 elements bit-exact |
| Rejected outputs | 112/112 preserve the full guarded sentinel buffer |
| Rank isolation | Both outputs reset per call; non-target output unchanged |
| Inputs and atomic state | Native full-buffer/atomic checks pass |
| Cases and dispatches | Exact ordered 116-record roster; 192 dispatches |

The [integer reference](gpu-attempt-v1/verify_native.py) represents FP32 values
as signed integer multiples of `2^-149`. It rounds each FP32 addition and
each BF16 narrowing separately with round-to-nearest, ties-to-even. It uses
no host floating-point arithmetic and does not call the Rust/device reference.
All emitted input bytes are independently reconstructed before comparing the
outputs. Native stdout and request hashes explicitly join the verifier result
to the supervised attempt.

Before GPU execution, [reference self-tests](gpu-attempt-v1/evidence/reference-tests.stdout)
pass 32 fixed arithmetic anchors, 13 nonfinite/overflow refusals, two signed-zero
checks, one positive synthetic observation, 14 observation mutation refusals
and two strict-JSON refusals. Synthetic fixtures are not GPU evidence.

For rejected cases, the native test checks every output byte and emits its
digest, not the raw output payload. The external verifier checks the expected
sentinel digest and closed record; it does not independently reread GPU memory.
Guard, input-canary and rank-isolation checks likewise rely on the pinned native
test's observations. No concurrent guard mutation or paired publication occurs.

The supervised native process takes 20.214936 seconds and the whole controller
takes 21.548849 seconds. These are diagnostic host intervals, **not GPU kernel
latency, model throughput, or evidence of progress toward a particular tokens/s
rate**.

The [GPU capsule](gpu-attempt-v1/retention-manifest.json) contains 57 members,
56 pinned bodies and 4,047,491 expanded bytes, including the exact harness,
request, HSACO, raw records and CPU receipt. It exports no host ELF body.
The archive is 458,547 bytes, SHA-256
`bfa35fb254ef5044ec533aa0fcb92f6eba771305fd3455511bbbddb669254ea0`.
The native receipt is 949,008 bytes, SHA-256
`a01bfed1b94ab8cfcb81c112b3c3bc5d41f2a317db239baee64acbf4222cb6a9`.

## Actual CPU Evidence

The [receipt](cpu-attempt-v1/evidence/complete.json) records nine naturally
successful phases in 59.358891 seconds. Every child was reaped with its process
group absent, with no timeout, forced cleanup or postcheck error.

| Scope | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Full KFD suite, five targets | 1012 | 0 | 4 |
| Focused combined owner | 11 | 0 | 1 |
| Focused atomic memory | 7 | 0 | 0 |

All 1011 prior named outcomes are preserved. Four ordinary passing tests and
one ignored native test are added. The compiled inventory contains 1016 names;
the library contains 996. GPU execution stayed disabled in this qualification.
All 791 source/harness rows and dependency snapshots remain unchanged, and
all five selected Cargo ELF products are pinned in the receipt.

The 76-member capsule retains 75 pinned bodies and 5,417,218 expanded bytes.
It includes the actual formatted Rust postimages, all raw CPU records, source
lineage, and formatting records; binary bodies are deliberately not exported.
The archive is 877,883 bytes, SHA-256
`5e7d398c7f65038415d5a0662afb41fab5fdd340fa80c61b362dc403bd77a21e`.
The actual CPU receipt is 1,143,535 bytes, SHA-256
`5d84e176512f0b9b59c85c2361ab95b4286c6142ced5cf4cd7995d5a8a6749b3`.

## Remaining Integration

The [distinct private paired coordinator](../guarded-mlp-paired-v1/README.md)
now compiles and passes CPU qualification on MI350. It prepares R1, MLP,
validator, the both-validator barrier, and R2 on each rank, with separate
tests for ten signals, queue frontiers, failure handling and owner lifecycle.
Its actual paired GPU execution is still unqualified. Ferric must also consume
that combined completion without calling its existing final residual step a
second time. No such paired execution is claimed by this serial diagnostic.
