# Competitiveness Sprint V1

Status: in progress. No matched vLLM/SGLang result or competitive claim yet.
The prior measurements remain frozen in `M1_PERFORMANCE_SPRINT_V2.md`.

## Active Teams

| Track | Deliverable | State |
| --- | --- | --- |
| Core runtime | Opt-in shared fresh full-topology observation per peer boundary, preserving all per-rank checks | Published `3e3a77284`; 475 library tests, 31 doctests and scoped Clippy pass; native TP2/TP8 producer fixtures pass with the option off and on |
| Kernels | Additive 32-row FP32 LM head/argmax and fast-path integration | Integrated; exact `3e3` emission, 15 native fixtures and both 16/32-budget model canaries pass; wave attention also passes four matched model canaries |
| Serving | Bounded sustained JSONL ingress, wall-clock arrival, token output, cancellation/backpressure | Combined gate passes 289 Rust tests, 16 HTTP tests and strict Clippy; real four-request/nine-token HTTP smoke passes |
| Integration/measurement | Shared streaming benchmark client, baseline identities, GPU scheduling, review and numerical gates | Bounded open-loop client and paired-series tooling integrated; 105 combined measurement tests pass; baseline launch approval still pending |
| Capacity | Explicit larger physical KV pool without changing the logical context/proof boundary | v9 kernels emitted on exact `3e3`; host integrated at `f0cd55f`, 315 ordinary tests plus four emitted-image checks pass; native/model qualification in progress |
| Speculation | Draft execution plus target verification and accepted-prefix KV integration into the fast path | Authenticated optional draft retention integrated at `8d6418f`; speculative execution and transactional KV settlement are not complete |
| Dispatch batching | Distinct bounded ordered submission in core runtime | Implemented in an unpublished fe2o3 branch; host tests pass, root review/native qualification pending |

## Frozen Comparison Contract

Start with Qwen/Qwen3-8B BF16, raw completion prompts, greedy decoding, fixed
output lengths, no chat-template transformation, target-only execution, and
prefix caching disabled. Use the identical canonical checkpoint/tokenizer.
First compare one GPU; separately compare the best allocation of eight GPUs,
including replicas rather than requiring TP8 for every engine. Do not compare
the old tick-driven four-request canary rate to a different serving workload.

The shared `competitive_benchmark.py` client records bounded closed-loop
windows, exact workload/client/identity hashes, per-request failures, final
server token usage and raw SSE arrival times. TTFT is client send to the first
nonempty text chunk; TPOT uses first/last text arrival and the server token
count. Chunk intervals are NOT per-token ITL, particularly under speculation.
An HTTP baseline and an in-process Ferric timer are different measurement
boundaries and cannot silently share a ranking. The shared Ferric HTTP adapter
is implemented. The bounded constant/Poisson open-loop client includes client
queue delay in TTFT and records overload and failures. The series analyzer
checks source/workload/identity receipts and computes paired hierarchical
bootstrap intervals. Client budget/environment faults invalidate comparisons.
These remain finite arrival cohorts including drain, not steady-state load;
chunk timestamps are not ITL and HTTP deadlines are not hard bounds. Full
open-loop SLO qualification remains incomplete.

Every run is explicitly non-qualifying. Passing a short canary or collecting
30 windows alone does not satisfy `PERFORMANCE.md`: equal baseline tuning,
held-out workloads, three fresh starts, numerical checks, controlled hardware,
paired confidence intervals and the entire declared primary suite still apply.

## Resource And Publication Rules

Build/test/format only on mi300x, private stages, jobs2, no local builds.
Root serializes all mi350 GPU use after identity-bound idle checks. Never kill
foreign jobs or delete shared models/caches/images. Archive evidence before
removing owned stages/worktrees. Ferric implementation remains local; only
separately reviewed Pages content may be published. fe2o3 updates must rebase
onto freshly fetched main before their authorized non-forced push.

At intake, both hosts had approximately 85GiB free. Cached baseline images:

- vLLM 0.28.0+rocm723: `sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba`.
- SGLang 0.5.15.post1.dev20260715+g495ae9aaa6: `sha256:cb8089ca16bd9182698b1bb5a915e6982bf9eeff63d2f6d027a9c31d8d6279d3`.

Fresh fe2o3 origin/main at intake was `310ce7b8c`; its delta from the previous
Ferric pin `6f6a67bb2` was test-only cleanup. Root repinned to `310` at `7c6edeb`,
then to the reviewed published runtime `3e3a77284` at `1ba3e01`. The final combined
controller passes 206 library, 55 batch CLI, 6 replica-control and 22 source-policy
tests, with three preexisting ignores, plus strict Clippy and release build.
The separate source/dependency gate at `121609f` passes 38 source-gate tests,
31 verifier policies, 28 metadata configurations, five unchanged generated
inventories and negative/release policies. Two stale verifier raw-file checksums
were independently audited and refreshed at `4182cad`; acceptance logic did not
change. Coverage remains 173 modules and 8,235 executable bodies. This is not a
new Verus proof or model qualification. Preliminary host gates and frozen `511`
GPU ablations retain their actual provenance.

## Native And Serving Gates

The additive v8 image passes scalar head, MFMA head and FP32 argmax fixtures at
1, 16, 17, 31 and 32 rows. Full-vocabulary outputs, immutable inputs, inactive
tails and surrounding guards are checked. Image `5f19b3ba` and worker `933d73d2`
retain exact `3e3` provenance. The root-reviewed result hash is
`5e27c9f5175321c570954b601d7d1cd0d5d14fd330ab347a4f9376ec1cf0a85a`.

Shared-currentness producer tests pass on TP2 and TP8, separately off and on,
using the same frozen base/peer images. These direct-core fixtures establish
GPU producer/peer-consumer ordering and checked teardown, not worker-wire,
concurrent-round, model or speed qualification. All native runs finish with
identity-bound idle checks across all eight GPUs.

The new HTTP path is loopback-only raw-prompt greedy streaming, with bounded
admission, cancellation, backpressure, deadlines and owned worker cleanup.
The common client passes fake-child ordinary and split-UTF8 streams. The real
smoke passes three complete reference cases and repeats the first, nine outputs
total, sequentially with prefix caching off. Every client-visible text/usage and
backend prompt/output token/byte check matches the frozen reference. Worker,
controller, process group and threads exit cleanly; all eight GPUs are idle.
Receipt SHA-256 is
`3bfc4e938f32ea21ad4800e2645072072e1660502e3eda8168aeffdecd293bb8`.
Observed client TTFT is 453-1,261 ms and TPOT 265-300 ms for these four short
requests. These ranges are not distributions or sustained-load estimates.
No persistent server is left running. This is not the static cancellation
workload, a concurrent load test or a physical queue-rollover qualification.

The default physical KV pool remains capped at 512 pages. Conservative reservations
allow at most 32, 6 and 1 active requests at ISL/OSL 128/128, 1024/256 and
4096/256 respectively. The [large-pool plan](M1_LARGE_KV_POOL_PLAN_V1.md) describes
the coordinated host/kernel expansion. Its explicit TP1 `large-kv-v9` profile
is now implemented with up to 16,384 physical pages and unchanged 8,192-token
per-request context. Maximum target K/V payload is 36 GiB, excluding weights
and workspace. It requires a separately admitted v9 image before allocation;
the legacy profile still rejects more than 512 pages. CPU capacity tests are
not a long-context model or 32-concurrent-request performance qualification.

## Current Wide-Head Canary

Both cases use controller `cd90bba6`, worker `933d73d2`, frozen v5 image
`98b5fdb1` and new v8 head `5f19b3ba`; only the scheduling budget and prefill
chunk change. Both pass the entire reference and cleanup gate. Both still use
five batches, 34 physical rows and 2,720 dispatches. There is one observation
per case, not a repeated comparison; the wider policy does not demonstrate a
gain on this tiny workload or full 32-row utilization.

| Budget / Chunk | Maximum Rows Observed | Output Tokens/s | Reuse TTFT ms | Reuse TPOT ms |
| --- | --- | --- | --- | --- |
| 16 / 16 | 16 | 2.678148 | 446.273 | 295.175 |
| 32 / 32 | 17 | 2.665643 | 446.964 | 295.290 |

The first attempt was rejected before execution because the staged v8 artifact
lacked its required `fe2o3-engineering-v1` parent directory. The failed receipt
is retained; correcting the directory layout changed no image or runtime code.

## Runtime Diagnostics

A separate 16-row instrumented run passes the same full reference and cleanup
checks. Its two snapshots are validated by `runtime_diagnostic_report.py`;
49 combined diagnostic/candidate/semantic tests pass. Raw records remain
unchanged, and diagnostic runs never enter the ordinary performance ledger.

The host-staged workload deltas include 2,720 dispatches and 3,481 commands. Host reads
transfer 40,386,696 bytes in 370 calls and record 1.482 seconds; dispatch waits
record 1.469 seconds. Full-currentness and immutable admission deltas are zero,
while operational checks record 0.112 seconds. Counter durations overlap,
include instrumentation overhead, and are not GPU timestamps or an exclusive
time breakdown. The earlier snapshot command is included in command deltas.
These observations motivate testing device-side TP1 reductions to eliminate
hidden/partial host copies before pursuing further runtime changes.

A separate device-TP1-plus-pruning diagnostic passes the same reference and
cleanup gate: 3,077 dispatches, 3,107 commands, four reads totaling 32 bytes and
25 writes totaling 18,224 bytes. Read time falls to 81,576 ns; dispatch waits
still record 1.456 seconds. Operational currentness records 0.111 seconds.
These overlapping host-wall counters motivate bounded ordered submission;
they do not identify exclusive GPU kernel time.

## First Target-Path Ablation

The existing immutable kernel-admission cache was tested with the now-working
TP1 MFMA plus FP32-head profile. The earlier cache experiments used TP8 and
did not establish this combination's effect. These runs keep the identical
controller, images, worker, four-request/eight-output canary, scheduling and
physical GPU. Only `--runtime-cache-admission` changes. Run order was control1,
cache1, cache2, control2. All four pass unchanged full token/byte references,
worker exit and identity-bound all-eight-GPU idle checks.

| Mode | Run | Output tokens/s | Workload s | Reuse TTFT ms | Reuse TPOT ms | Whole process s |
| --- | --- | --- | --- | --- | --- | --- |
| Control |1|2.005281|3.989466|639.762|493.493|131.580225|
| Cache |1|2.481748|3.223534|485.988|316.198|135.195576|
| Cache |2|2.635293|3.035715|462.558|303.700|128.176270|
| Control |2|2.001220|3.997561|643.552|492.986|128.812970|

Across the two observations per mode, mean output rate improves 27.72%, reuse
TTFT falls 26.09% and reuse TPOT falls 37.16%. Mean whole-process time is 1.14%
slower because setup varies. These are short engineering observations, not
steady-state serving, confidence intervals, a new default or a vLLM/SGLang win.
The frozen controller is `3d039c49` (Ferric `00fa58d` / public `511`), worker
`aaa0216a`, base image `8c81d3fe` and separate head image `d6086650`. They are not
relabeled as the active `3e3` source. Raw receipts are retained in the root-owned
`ferric-compete-evidence-v1/tp1-mfma-{control,cache}-r{1,2}` evidence directories.

## Device Reduction And Head Pruning

The existing device-side TP1 reduction and output-head pruning options now pass
in the current v8/MFMA/admission-cache combination. These are explicit profile
ablations, not newly implemented kernels or promoted defaults. All six cases
use controller `cd90bba6`, worker `933d73d2`, v5 image `98b5fdb1`, v8 image
`5f19b3ba`, budget/chunk16, and the same four-request/eight-output reference.
Only the collective and pruning settings differ. Run order is host1, device1,
device+prune1, device+prune2, device2, host2. All full token/byte, worker-close
and all-eight-GPU idle checks pass.

| Profile | Run | Output tokens/s | Workload s | Reuse TTFT ms | Reuse TPOT ms | Whole process s |
| --- | --- | --- | --- | --- | --- | --- |
| Host staged | 1 | 2.678148 | 2.987139 | 446.273 | 295.175 | 124.625608 |
| Host staged | 2 | 2.515140 | 3.180737 | 463.546 | 303.082 | 134.223709 |
| Device TP1 | 1 | 4.933816 | 1.621463 | 293.151 | 260.300 | 124.437870 |
| Device TP1 | 2 | 4.070982 | 1.965128 | 372.862 | 336.891 | 133.330778 |
| Device TP1 + pruning | 1 | 5.027332 | 1.591301 | 292.167 | 258.793 | 123.658619 |
| Device TP1 + pruning | 2 | 4.179222 | 1.914232 | 366.985 | 333.343 | 132.901353 |

Mean rates are 2.596644, 4.502399 and 4.603277 tokens/s respectively. Device
reduction improves mean rate 73.39%; pruning adds 2.24%, for 77.28% combined.
Combined mean reuse TTFT falls 27.55%, while mean reuse TPOT falls only 1.02%.
The second device observations are substantially slower than the first ones;
these two-observation means do not establish stable variance, confidence
intervals, serving latency, or a framework comparison. The retained idle
snapshots do not establish thermal/clock/CPU-load equivalence. Dispatch counts
are 2,720, 3,080 and 3,077 respectively: eliminating host copies adds device
work, and pruning removes three unnecessary head dispatches on this workload.

An additive wave-attention/v8 candidate checker preserves the old comparator
bytes and validates the actual wave policy through the frozen full-reference
checker. Its five test methods plus the candidate/semantic suite pass all 46
remote tests. Host opt-in is integrated at `529a35c`; model measurements below
are separate from those checker tests.

## Wave Attention Ablation

Four canaries use controller `a0dbe7d9` (wave host `3a89091` / exact `3e3`),
worker `933d73d2`, unchanged v5/v8 images, budget/chunk16, device-TP1 reduction,
head pruning and admission caching. Only attention changes. Run order is
control1, wave1, wave2, control2. Every token, byte, schedule, worker-close and
all-eight-GPU idle check passes; no forced process cleanup was needed.

| Attention | Run | Output tokens/s | Reuse TTFT ms | Reuse TPOT ms |
| --- | --- | --- | --- | --- |
| Baseline | 1 | 5.026699 | 291.670 | 258.695 |
| Wave | 1 | 5.498143 | 263.157 | 230.293 |
| Wave | 2 | 5.400687 | 268.684 | 234.377 |
| Baseline | 2 | 5.000058 | 293.259 | 260.520 |

Two-observation mean output rate improves 8.70%; reuse TTFT falls 9.08% and
TPOT falls 10.51%. This is the fixed four-request/eight-output engineering
canary, not a steady-state HTTP test, confidence interval, new default or
framework win. The wave/v8 combination currently excludes the v9 large pool.

## Rejected Polling Experiment

An unpublished core experiment replaced the serial worker's fixed 50-us sleep
with the existing adaptive wait, whose backoff can reach 1 ms. All four model
runs pass correctness and teardown, but the candidate regresses.

| Worker wait | Run | Output tokens/s | Reuse TTFT ms | Reuse TPOT ms |
| --- | --- | --- | --- | --- |
| Fixed control | 1 | 4.928390 | 297.049 | 266.605 |
| Adaptive 1-ms cap | 1 | 3.885786 | 405.628 | 347.523 |
| Adaptive 1-ms cap | 2 | 4.176812 | 358.044 | 294.504 |
| Fixed control | 2 | 4.945817 | 296.332 | 264.923 |

Mean output rate falls 18.35%, reuse TTFT rises 28.70%, and TPOT rises 20.79%.
Candidate worker `5372f002` is built from unpublished core `0052fe181`; it is
not published or adopted. A separate follow-up capped at 50 us passes 477
library tests, 31 doctests, strict engineering/default Clippy and a release
build at `4491b6b70`; its GPU measurement is still pending. Raw failed-format
and successful test receipts are retained. Neither experiment changes the
public `3e3` pin or the general-purpose wait defaults.

## Draft Intake

`EngineeringQwenModelV1::open_with_draft` retains the exact authenticated
1,503,264,768-byte draft payload and exposes a borrowed draft/config/layout
view without cloning weights. Default `open` still authenticates draft input
into a sink and allocates no draft payload. The scoped intake gate passes 305
ordinary tests plus strict Clippy and formatting; full-model intake and
speculative numerical execution are not established by the bounded fixtures.
Resident draft execution, K+1 target verification, selective accepted-prefix
KV commit/rollback, correction/bonus handling and load tests remain required.

## Published Checkpoint

The separate Pages-only branch was freshly rebased and published at
`0a8df6e3595c1ed3ae7fa15dd0edcd9e00986e76`. Workflow `34644821323` succeeded.
All seven live static assets match the validated files byte for byte. The site
records the v8 model and HTTP smoke checkpoint without altering historical
performance results or claiming a baseline win. Its private build stage and
completed worktree were removed after the evidence archive was verified.
Ferric implementation commits remain local; fe2o3 main was independently
rechecked and remains `3e3a77284a61654134211f8145dd0ddeebb2ff91`.
