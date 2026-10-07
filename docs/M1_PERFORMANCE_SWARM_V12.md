# Performance Swarm V12

Updated October 3, 2026 (PDT). This checkpoint integrates the previously measured
packed-down kernel into a separate model experiment. **Remote focused tests,
corrected policy, strict Clippy, release, 37 harness fixtures and final binding
validation pass. One independently audited native AB pair shows 7.17% lower
TPOT, but foreign GPU activity interrupts the reverse order.** This is an
incomplete comparison, not a repeatable gain, default change, serving
qualification or vendor win. All 33 M1 gates remain open.

## Teams

| Lane | Owner | Status |
| --- | --- | --- |
| Kernel integration | integration lane | 23 ordinary packed-down tests and explicit actual-image test pass remotely; fault-injection coverage extended |
| Inference entry | integration lane | Separate CLI and runner compiled; CLI, actual-image and feature-union rejection checks pass |
| Verification | v12_policy_resume / v12_native_resume | CPU qualification passes; a2 partial AB raw timing, all 1,024 output IDs/bytes, identities and dispatch counts independently pass; failed outer receipts retained |
| Engine follow-on | v12_policy_resume | Full-token native submission design and source bindings complete; no implementation, test or measured gain |
| Integration | root / v12_space_resume | Both native attempts interrupted by foreign KFD; complete evidence retained and all four owned native/input roots retired |

## Source Changes

The new executable is `ferric-qwen3-packed-down-r1-live`, requiring the existing
`c1-ordered64` feature. Its explicit `--packed-down-mode` is `baseline` or
`packed-down-u32-r1`; artifact, producer-roster and image identity arguments are
mandatory. Existing executable defaults remain unchanged.

Both arms load the same nine images and prepare the same additional 36 packed
down-weight matrices: 3,623,878,656 bytes (3.375 GiB), plus 24,576 bytes of
activation scratch. Weight bytes are authenticated before packing and upload.
The candidate packs the current activation after each decode SwiGLU and then
uses the packed projection, retaining FP32 output and the existing residual
path. There is no per-token host packing or model-weight upload.

The scheduler phase is bound to the immutable prepared batch before packet
preflight. One-row final prefill is not decode and must remain on the original
path. Pre-submission rollback abandons the binding; submitted failures stop
reuse. Ordinary constructors have no packed workspace and retain a no-op hook.
This is a closed TP1/C1 experiment: mixed-request and multirow-decode batches
are rejected before submission, not advertised as continuous-batching support.

The profile excludes V19 KV copy, packed gate/up, partial GEMV, prefill ordering,
prefill32, active polling, token programs and instrumentation. This isolates one
change instead of combining earlier gains. Setup, profile and closed events
report the requested/actual mode, image identities, producer roster, all 36
authenticated source hashes, memory costs and experiment scope.

The exact 14 qualified device files and client `Cargo.lock` remain unchanged.
No compiler or KFD code was changed. Historical kernel producer `962044346` and
worker `fe0b352` remain explicitly identified; neither is relabeled as a new
mainline build.

Independent review corrected two recording-test expectations: the exact SwiGLU
export name and the deliberately narrower candidate output view. Added tests now
cover all 11 submission/wait boundaries of the 688-packet decode, including its
48-packet tail; mixed-request/multirow-decode rejection and retry; and later
setup/close failures. These focused tests pass remotely. Earlier source-only
reviews are not substituted for the actual remote test receipts.

## Remote Qualification

The original `a001` campaign on mi300x passed formatting and 97 focused test
executions across ten groups: ordinary library/CLI/shared runtime, gate/up
artifact regression, two explicitly executed actual-image tests, and timestamp
and token-program feature combinations. Repeated feature cases are counted as
executions, not unique tests. The original 25 native-harness fixtures also pass.

Source policy then stopped naturally: 52 passed and three failed. Two closed
entrypoint allowlists omitted the new separate binary; one exact-source assertion
still expected dispatch-count validation before the new row-binding hook.
Only `tests/source_policy.rs` changed to correct these assertions and add a
dedicated packed-down isolation test. The correction preserves default isolation,
bind/count/submit ordering and rollback checks. The original failure is retained;
it is not relabeled as a successful run.

A separate `a002` policy supplement uses a 121-file immutable include closure,
the original pinned dependency, unchanged production bytes, and all 56 policy
tests. All five supplement phases pass, including all 56 executed tests.
The original release build also passes. Its 10,300,544-byte retained controller
has SHA-256 `b774ce96949e29926ba39a15c941deeb8996c02ce2f79f637e776a8d2c638087`;
independent ELF hashing and post-build source custody pass.

Strict Clippy initially fails solely on the missing `# Panics` documentation for
`bind_dispatch_rows`. A separate immutable source adds precisely 103 documentation
bytes, with no executable-code change. Its first Clippy attempt stops on the
existing stage reserve (status 125, TERM, `cleanup_ok: false`). That failure is
retained unchanged. After file-only reserve restoration, the same-source cached
retry passes strict `-D warnings` and separate verification, without signals or
relaxing resource limits. The release and focused tests remain explicitly bound
to the original production source, not relabeled as the documentation successor.

All 37 successor harness fixtures pass under the bounded CPU owner. BuildV4
assembly retains the original 32 phase triples, both original failures, corrected
policy, the resource stop and clean Clippy retry. Its build binding is
`726feec20020b32039ef5b663a7e4b991c0d9e0fc1ceda5d268db921bf22e3b4`, and the
actual harness proof is `9d77b75814f228b7997fb2fcb7efbb4a08da150be5d35d5610fb482762f24fdb`.
Full binding validation also passes with clean unsignaled completion; native
ABBA remains incomplete. The independently retained complete binding is under
`products/perf-v12-qualified-20261003/native-binding-a004`.

## Measurement Gate

The prior [V11 results](M1_PERFORMANCE_SWARM_V11.md) establish the component result:
packed-down component controller time fell 13.02% for single operations and
39.98% for groups of five, with activation packing included. Those synthetic
results do not establish model TTFT, TPOT or throughput improvement.

CPU qualification and final binding verification are complete under the existing
bounded mi300x build profile, without weakening space, memory, CPU or timeout
limits. The native qualification contract requires four fresh serial ABBA starts on MI350:
one excluded warmup and three measured 128-input/128-output requests per start.
All 2,048 output IDs and streamed/final bytes must match the exact reference.

The accepted a2 AB arms match the expected source schedule below. Across four
requests per arm, raw totals are 350,844 baseline and 369,132 candidate
dispatches, with 540 physical batches in each arm.

| Metric | Baseline | Packed Down |
| --- | ---: | ---: |
| Decode packets/token | 652 | 688 |
| Packets/request | 87,711 | 92,283 |
| Physical batches/request | 135 | 135 |
| Ordered64 groups/decode | 11 | 11 |

Count every activation-pack dispatch in candidate timing. Report TTFT, TPOT and
finite output rate per execution order, retain every slow sample, and report
setup time and memory separately. Do not pool these results with V19, prefill
diagnostics or historical matched HTTP runs. Ferric has not demonstrated a
performance win over vLLM.

## Partial Native AB Result

Attempt `down12d004a2` completes baseline-AB and candidate-AB. Each fresh start
has one excluded warmup and three measured 128-input/128-output requests.
Independent raw replay verifies all 1,024 IDs and every streamed/final byte,
per-token timestamps, serial requests, distinct process sessions, exact GPU
identity, all 66 argv elements except the mode value, and natural unsignaled
controller/worker teardown for both accepted arms.

| Native Ingress Metric | Baseline AB | Packed-Down AB | Observed Change |
| --- | ---: | ---: | ---: |
| Mean TTFT | 863.879396 ms | 872.062754 ms | 0.95% worse |
| Mean TPOT | 71.284978 ms | 66.174387 ms | 7.17% lower |
| Finite-window output rate | 12.906712 tokens/s | 13.798386 tokens/s | 6.91% higher |
| Separate setup | 124.472792 s | 124.731572 s | Excluded from request metrics |

Finite-window rate counts 384 measured output tokens over first measured arrival
to last completion, not a sustained-throughput test. Candidate-BA is interrupted
during setup by another Ferric TP2 experiment using the same physical GPU.
There is no accepted reverse-order result, completed ABBA, confidence estimate,
HTTP comparison or current matched vLLM run. Packed-down remains default-off.
Do not pool a1's baseline or historical Ferric measurements into this pair.

The audit is retained at
`products/perf-v12-native-a2-20261003/INDEPENDENT_PARTIAL_AB_AUDIT.md`, with exact
values in `independent-partial-ab-audit.json` (SHA-256
`2f9b34217fa0a8ba4d302ef5dcce9be7884a03c6357f84d9861a587a73732737`).
The final post-quiescence archive is
`f4c90f9fe1b98df349d798c874759daca4c23944a20c5226f7cb9f26eb995be8`,
covering 466 files and 89,483,274 bytes. Full archive verification and independent
custody review pass.

## Current Limits

Normal SSH to `mi300x` and `mi350` is restored. Native attempt `down12d004a1` accepted
the first baseline start with all 512 output IDs/bytes exact and clean controller
teardown. During candidate setup, unrelated KFD PIDs appeared. The outer guard
stopped with status 125, TERM then KILL, return code -9 and `cleanup_ok: false`.
The report remains unaccepted; there is no candidate result or paired speedup.
Subsequent inspection found the recorded owned processes absent and KFD empty;
this does not relabel the original uncertain-cleanup receipt. The fresh identical
`down12d004a2` attempt has plan SHA-256
`16ac450c30580735310f51a08bc9b4047a6d81b158d3e43f8761ebc217ef4941`.
Its accepted AB results are recorded above. Its outer supervisor also stops
with status 125, TERM/KILL, return code -9 and `cleanup_ok: false`; the overall
report remains unaccepted. A later process inspection observes an orphaned
candidate-BA controller and worker. Both are already absent when exact-identity
cleanup arrives, so no additional signals are sent. Later absence and successful
directory retirement do not repair the original failed cleanup receipt.

The first a2 archival snapshot was taken before process quiescence was
established and is retained separately as premature. The final a2r2 capture is
made only after the exact owned process identities and stage-prefix census are
empty. Both captures are preserved; only the final one binds retirement.
After complete local archive verification, unchanged guarded retirement removes
both attempts' native and input roots (four directories), with status 0 and no
refusal. Complete retirement controls/results are retained locally. No owned
benchmark process remains; the interfering Ferric job is left untouched, and
no further GPU launch is attempted in this checkpoint.

The local scratch evidence directory was
absent after interruption; original CPU inputs, raw receipts and harness inputs
were recovered from mi300x into `products/perf-v12-recovered-20261003`. A surviving
MI350 baseline input set matches its pinned image and reference hashes.
The current [retry and assembly handoff](../../ferric-perf-swarm-v6/proposals/perf-v12-packed-down-cpu-r4/README.md)
and [native stage source](../../ferric-perf-swarm-v6/proposals/perf-v12-packed-down-native-r4/down-native/prepare_down_stage.py)
retain the exact interfaces. Historical scratch links that could not be recovered
are not treated as available evidence.

The six obsolete expanded build-input directories remain untouched after their
earlier all-process census refusal. Independent file-only cleanup subsequently
retired five site and four compiler transport archives after verified local
retention, exact identity checks and repeated strict no-use checks. A separate
data-only reserve-restoration operation retired seven additional retained
transport archives after the Clippy resource stop. Total reclaimed allocation
is 322,359,296 bytes across 16 files. No expanded source or toolchain directory
was deleted. Reserve restoration is not a build/test qualification receipt and
does not change the stopped attempt's unqualified cleanup status. The existing
four-core, 8 GiB RSS, 20-minute, 28 GiB stage and 512 MiB reserve limits remain
unchanged. No new worktree was created.

Upstream fe2o3 main was refreshed to `0955563481ce092c813b097fa37f3ce5c5d44ca6`.
Its KFD changes affect the unbacked empty-slice sentinel in conditional dispatch;
the selected Ferric route uses explicit engineering ordered batches instead.
The runtime subtree is unchanged. This source audit is not binary equivalence or
a newly built worker; frozen producer/worker identities remain explicit, and the
next producer qualification must use refreshed main.

## Engine Follow-On

The independently bound source audit finds that Ferric already registers and
executes token programs, but the fe2o3 backend still prepares every dispatch and
executes 652-command decode as eleven synchronous groups of at most 64. Native
staging also clears a full 64 KiB kernarg slot per dispatch: 40.75 MiB per token.
These are mechanisms to investigate, not measured explanations of the full
vendor gap. Worker wall time overlaps GPU execution and cannot be labeled host
overhead in its entirety.

The source-only [full-token submission handoff](../../ferric-perf-swarm-v6/proposals/perf-v12-full-token-engine-handoff-r1/HANDOFF.md)
specifies a separate default-off generic fe2o3 backend with owned signal/kernarg
storage, all-dispatch preflight, one publication/final wait, complete signal
retirement, unchanged deadlines and fail-closed lifetime handling. The proposed
counter target is eleven to one publications/waits per decode, with unchanged
dispatches. No implementation or speedup is claimed. Kernel/model admission and
benchmarks stay in Ferric; generic runtime changes belong in fe2o3. Refresh main
again before implementation and keep the existing conflicted checkout untouched.

The separately validated V11 site candidate remains unpublished
because its first read-only GitHub API request failed before any remote mutation.
