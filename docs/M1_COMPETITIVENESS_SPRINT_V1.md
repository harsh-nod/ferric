# Competitiveness Sprint V1

Status: in progress. The first matched Ferric/vLLM cell passes, with Ferric
substantially slower. SGLang measurement and competitive qualification remain open.
The prior measurements remain frozen in `M1_PERFORMANCE_SPRINT_V2.md`.

## Active Teams

| Track | Deliverable | State |
| --- | --- | --- |
| Core runtime | Opt-in shared fresh full-topology observation per peer boundary, preserving all per-rank checks | Published `3e3a77284`; 475 library tests, 31 doctests and scoped Clippy pass; native TP2/TP8 producer fixtures pass with the option off and on |
| Kernels | FP32 heads and opt-in parallel selection | Existing v8 head route passes; new one-root Wave64 v11 argmax passes separate exact-216 and latest-8efd emission/replay and fourteen-case finite-active native gates. All six corrected opt-in model comparisons pass independent replay; full128 output rate +4.67% at n=1 per mode, short8 ABBA +10.16% at n=2. Native host-wall diagnostics only; default unchanged. |
| Serving | Bounded sustained JSONL ingress, wall-clock arrival, token output, cancellation/backpressure | Combined gate passes 289 Rust tests, 16 HTTP tests and strict Clippy; real four-request/nine-token HTTP smoke passes |
| Integration/measurement | Shared streaming client, baseline identities, GPU scheduling and numerical gates | Matched Ferric/vLLM 128/128 cohorts pass; Ferric remains substantially slower. SGLang r7 starts and finishes but fails exact output in 10 of 30 measured responses and its final diagnostic. No admitted SGLang metrics. |
| Capacity | Explicit larger physical KV pool without changing the logical context/proof boundary | v9 emitted on exact `3e3`; host at `f0cd55f`, 315 ordinary tests plus four emitted-image checks and all nine native fixtures pass; the fixed full-allocation model canary passes, long-context/concurrency qualification remains open |
| Speculation | Private autoregressive proposals and paired target verification | All eight repeated paged-draft canaries pass. Two-round K4 source `ae355e5` passes 594 host test invocations/eight doctests and two fresh native runs with `[4,4]` acceptance, both catch-ups and exact output. Speculative serving remains separate. |
| Dispatch batching | Distinct bounded ordered submission in core runtime | Currentness fix published at `216822284`; native counters/lifecycle and model/reference gates pass; fresh same-worker ABBA shows 22.19% mean rate gain over serial in this short canary, not a performance default |

## Frozen Comparison Contract

Start with Qwen/Qwen3-8B BF16, raw completion prompts, greedy decoding, fixed
output lengths, no chat-template transformation, target-only execution, and
prefix caching disabled. Use the identical canonical checkpoint/tokenizer.
The first frozen 128/128 cell explicitly uses BF16 weights/decoder and FP32
output-head computation in all engines. The baseline switches were selected
before generating reference outputs: vLLM `head_dtype=float32` and SGLang
`--enable-fp32-lm-head`. Their operand-conversion implementations differ and
must be recorded. A stock BF16-head cell is separate; this FP32-head cell cannot
establish a win over default baseline configurations.
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
chunk timestamps are not ITL. The current client bounds every HTTP I/O wait by
one absolute monotonic deadline and joins cancelled workers after closing their
owned sockets. Its 74 client/transport/adapter/series tests pass, including
dripped headers, bodies and chunk framing. This is not a real-time OS or remote
server cleanup guarantee; older soft-deadline receipts keep their original
meaning. Full open-loop SLO qualification remains incomplete.

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

During the sprint, upstream advanced to `c94e2101a` with a source-order
uniformity-analysis construction change. Active Ferric dependencies were
repinned at `2b58dde`, with only the two audited verifier Cargo raw hashes
refreshed at `b0c4086`. The five reviewed verifier Rust sources are unchanged.
The c94 aggregate passes all 22 gate steps, including 320 ordinary adapter
tests, strict Clippy, 38 source-gate tests, 31 protected-verifier policies,
unchanged generated inventories and negative/release policy checks. The newer
generic ordered-batch runtime is now published at `f85bb375e`; active dependencies
are repinned at `d8c8990` with two separately audited Cargo hashes at `663f780`.
The exact combined `2ce566d` f85 gate passes all 22 steps: 333 ordinary adapter
tests, strict Clippy/formatting, 38 source-gate tests, 31 protected policies,
unchanged generated inventories and negative/release policy checks. No further
hash, AST or inventory refresh was needed. This is not a new Verus proof.
No completed `3e3` measurement is relabelled as a newer source revision.

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

The exact-`3e3` v9 image `592034c8` passes all nine native fixtures: append at
rows 1/16/17/32 and physical capacities 513/8192/16384, attention at those
capacities plus one row with 8,192 logical tokens. High physical pages, reverse
page placement, inaccessible NaN data, inactive tails, full outputs, immutable
inputs and surrounding guards are checked. Worker close and all-eight-GPU idle
checks pass. Result SHA-256:
`4b9791bf859236a031c56f2de197068ebfd3ec86c5591ebedba348a9c9ef7ceb`.
The independent model canary comparator validates actual pool conservation
before translating only unused capacity in memory for the unchanged frozen
semantic checker. Its four test methods and existing suites pass 50 tests.
Model traces do not expose physical addresses; native evidence is separate.

The full-allocation model canary also passes: the same controller `489e1751`,
worker `933d73d2`, v5/v8 images and fixed workload run first with 8 legacy pages
and then with 16,384 v9 pages (38,654,705,664 bytes of target K/V payload).
Both match all eight reference outputs and bytes, including cancellation and
prefix reuse; both execute five batches, 34 physical rows and 3,077 dispatches.
Maximum observed batch occupancy is 16 rows. Both close without forced cleanup
and leave all eight GPUs idle. Comparison hashes are
`f7392fd154fcd77ae186387d4da3018e7315826d29be9da9ed2ec71cc2d5dd4d`
and `bf372d42f51dffdf8144cee96b9a3f68c5105ee60a96406272c541cead250fb4`.
This small model test establishes allocation/profile integration, not high-page
model access, long-context correctness, 32-request concurrency or a speed gain.

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
build at `4491b6b70`. Raw failed-format and successful test receipts are retained.
Neither wait experiment is published or changes general-purpose wait defaults.

The capped follow-up also passes all four model/reference/cleanup gates:

| Worker wait | Run | Output tokens/s | Reuse TTFT ms | Reuse TPOT ms |
| --- | --- | --- | --- | --- |
| Fixed control | 1 | 4.109710 | 373.165 | 338.128 |
| Adaptive 50-us cap | 1 | 4.957867 | 293.963 | 262.582 |
| Adaptive 50-us cap | 2 | 4.652421 | 300.650 | 272.097 |
| Fixed control | 2 | 4.158887 | 370.264 | 337.031 |

Mean rate improves 16.23%, reuse TTFT falls 20.02%, and TPOT falls 20.81% within
this pair. These controls are slower than those in the uncapped matrix;
cross-matrix ranking is therefore not a valid attribution. Candidate worker
`2320594d` is not adopted from two observations per mode, and no serving or
confidence-interval qualification is claimed.

## Ordered Submission

The generic KFD worker now offers a separate `DispatchOrderedBatch` operation
for 1..16 ordered packets, with distinct retained argument/signal slots, one
publication and wait phase, all-signal completion validation, bounded aggregate
deadline and terminal failure handling. Serial `DispatchSequence` is unchanged.
Core main `f85bb375e7d6f4b8193e697293d29a8888439f0b` was published after a fresh
fetch/rebase and non-forced push. No inference/kernel implementation was pushed
to fe2o3 and no Ferric implementation was published.

Each native mode (full and operational currentness) passes 184 packets across
26 serial/ordered dependency chains at counts 1/2/16 and rows 1/3, including
repeated storage reuse and a real queue rollover with live buffers. Every full
output/input/guard check, worker close/reap and all-eight-GPU idle check passes.
Worker `761027c5` was built/tested at `f64c86e0c` on upstream `c94e2101a`; the
published commit adds only native-result documentation to those code bytes.
Root review SHA-256:
`5754053d21a8de9e5f79c1ce2cfbdd70859c1c0db9179bff0232587046c2cd42`.
This is runtime dependency/lifecycle evidence, not model speed qualification.
In this published implementation, the ordered entry/exit boundaries still
perform full currentness checks even when operational mode is selected.
Ferric's separate opt-in adapter is integrated at `2ce566d`, using the existing
36 pairs of 10-kernel attention and 5-kernel FFN groups, with explicit host-I/O
barriers. The legacy seven-field performance profile is unchanged; the new
option has a separate Setup field and is not enabled by default.

The initial serial/ordered/ordered/serial model comparison uses the identical
controller `108feeb8` (source `826f62a`, byte-identical compiled closure to root
`2ce566d`), worker `761027c5`, frozen v5/v8 images, device TP1 reduction, output
head pruning and operational mode. All four pass eight reference outputs and
bytes, cancellation, prefix reuse, five batches, 34 physical rows and 3,077
dispatches, with unforced close and all-eight-GPU idle checks.

| Submission | Run | Output tokens/s | Reuse TTFT ms | Reuse TPOT ms |
| --- | --- | --- | --- | --- |
| Serial | 1 | 4.962428 | 295.123 | 264.895 |
| Ordered | 1 | 3.017013 | 500.564 | 468.802 |
| Ordered | 2 | 3.002672 | 506.883 | 473.272 |
| Serial | 2 | 4.356362 | 366.004 | 329.259 |

Mean output rate regresses 35.40%, reuse TTFT increases 52.38% and TPOT
increases 58.56%. Serial controls also vary; these are two short observations
per mode, not steady-state serving or confidence intervals. Ordered submission
is therefore not adopted as a performance default. The first-pair host phase
audit observes approximately 1.027 seconds of additional attention/FFN IPC
round-trip time despite lower send time. Source inspection identifies 720 full
topology scans at ordered boundaries across this workload as a plausible cause,
not an exclusive syscall or GPU-time attribution.

The core change built at `c110ac55c` makes ordered dispatch boundaries
honor the already opt-in operational-currentness policy. Default-mode dispatch,
first-use allocation, rollover and teardown retain full checks. Its exact-source
gate passes 483 engineering library tests, 20 integration tests, 420 default
library tests, 31 doctests, strict Clippy, formatting and worker release build
(one preexisting library ignore per configuration). Native counter probes at
chain lengths 1/1/16 observe operational full-check deltas 4/2/2 for the control
and 2/0/0 for the candidate. Both retain the identical full-mode 13/11/26
deltas. Each candidate lifecycle mode also passes 184 packets, 26 chains,
storage reuse, actual rollover, full output/input/guard checks, clean teardown
and all-eight-GPU idle checks.

The exact candidate worker `b91ddef7` and old worker `761027c5` then passed a
control/candidate/candidate/control Qwen canary, keeping the controller, images
and ordered profile fixed:

| Worker | Run | Output tokens/s | Reuse TTFT ms | Reuse TPOT ms |
| --- | --- | --- | --- | --- |
| Old ordered | 1 | 3.011428 | 503.553 | 468.889 |
| Currentness fix | 1 | 5.500006 | 261.776 | 228.193 |
| Currentness fix | 2 | 5.495006 | 263.160 | 228.401 |
| Old ordered | 2 | 2.912701 | 510.112 | 473.657 |

Mean rate improves 85.60%, reuse TTFT falls 48.21%, and TPOT falls 51.56%
against the old, regressed ordered path only. This is not an 85.60% gain over
Ferric serial, steady-state throughput, or a framework comparison. Every run
passes all eight reference outputs, five batches, 34 rows, 3,077 packets,
unforced cleanup and idle checks.

A fresh serial/ordered/ordered/serial comparison then uses the same new worker
`b91ddef7` in all four cases, leaving controller, images and every other profile
setting fixed. All four pass the same reference, packet and cleanup gates:

| Submission | Run | Output tokens/s | Reuse TTFT ms | Reuse TPOT ms |
| --- | --- | --- | --- | --- |
| Serial | 1 | 4.110346 | 372.971 | 337.001 |
| Ordered | 1 | 5.486351 | 264.471 | 227.896 |
| Ordered | 2 | 5.588073 | 258.208 | 226.036 |
| Serial | 2 | 4.952575 | 295.475 | 265.160 |

Mean rate is 4.531460 versus 5.537212 tokens/s, a 22.19% gain; reuse TTFT falls
21.81% and TPOT falls 24.62%. The serial controls vary substantially. Two
observations per mode do not provide confidence intervals, steady-state load,
a framework comparison or sufficient evidence for default adoption. Raw runs
remain `ordered-final-{serial,ordered}-r{1,2}` in the root evidence archive.

After these gates, a fresh fetch/rebase and non-forced push published
`21682228486f7186cc3c37ddf165fffc438d8b6a` to fe2o3 main. Its only delta from
the exact tested `c110ac55c` source is native-result documentation. Active
Ferric dependencies are repinned at `3c554d9`, with only two separately audited
verifier Cargo raw hashes refreshed. The paired source-tree identity is fixed
at `346d588`, and formatting at `656edb2`. The exact `656edb2` aggregate passes
all 28 steps: 516 adapter test invocations, 18 existing ignores, five doctests,
38 source-gate tests, 31 verifier-source-policy tests, 29 metadata configurations,
five unchanged inventories, strict Clippy/formatting and negative/release gates.
Aggregate SHA-256:
`c0819d39d6a9a882bac2f6da3b6050fac8231a60ab915fd0032a31915da46f4b`.
Earlier failed identity/format/summary-recorder attempts are retained. This is
not a new Verus proof; older receipts keep their actual source identities.

Separate fresh serial and ordered HTTP smoke runs also pass all nine reference
outputs, client text/usage, process cleanup and idle checks. Their corrected
receipt parser requires the actual stopped/drained terminal record. The first
parser attempt remains a failed receipt; it was not overwritten or relabelled.

## Draft Intake

`EngineeringQwenModelV1::open_with_draft` retains the exact authenticated
1,503,264,768-byte draft payload and exposes a borrowed draft/config/layout
view without cloning weights. Default `open` still authenticates draft input
into a sink and allocates no draft payload. The scoped intake gate passes 305
ordinary tests plus strict Clippy and formatting; full-model intake and
speculative numerical execution are not established by the bounded fixtures.
The standalone fixed-envelope draft CLI is integrated at `24dddba` and its
host gate passes 291 tests plus retained-image admission, Clippy and release
build. It consumes five prompt inputs and produces two outputs through six
baseline forwards. The current independent PyTorch producer passes 16 CPU
tests and its bounded GPU run repeats all six results exactly, producing IDs
`[12095, 13]` and text `" Paris."`. All 151,936 logits are finite at every step;
the reference records runner/source/checkpoint identities and token tie policy.
Neither host fixture values nor target-model outputs are draft references.

Ferric's standalone draft GPU canary now passes that independently pinned
reference: six forwards, 2,544 packets, matching step choices, final IDs/bytes,
unforced close and identity-bound idle checks across all eight GPUs. It uses
controller `e84b104e` built from `346d588`, worker `b91ddef7`, and the retained
13-root baseline image `7c0b1934` with its original compiler provenance. The
later `656edb2` build does not relabel this binary. Raw Ferric receipt SHA-256:
`bf9da61e5c2a4fbec0a3779aac1f40a3de8f13833b245ba0e9bb27860e56e770`.
This is a tiny BF16-head, token-at-a-time draft correctness canary, not paged
draft execution, speculative serving or a performance measurement.

Paired accepted-prefix KV support is integrated at `228df11`: it requires an
opaque target-driver result, derives acceptance from every K+1 target choice,
preflights both prefix states before applying either, and requires one draft
catch-up input after full acceptance. Its host gate passes 376 ordinary tests,
two compile-fail boundary doctests, strict Clippy and formatting. At that
checkpoint only test fixtures could mint draft success. The separate paged
driver below now seals real completed draft inputs. This remains engineering
support, not a new Verus proof or protected token publication.

The isolated 32-row draft family is integrated at `85a450a` (source `79ba4ea`):
14 distinct roots for Draft06B's 28-layer, hidden-1024, Q16/KV8 geometry,
scalar/MFMA projections and FP32 head, paged append/attention, and remaining
decoder operations. It passes 27 CPU tests and strict Clippy. Fresh `216822284`
compiler emission produces image `3a308c9c`, with exact closed-root and replay
checks. All 36 native fixtures now pass, spanning all 14 roots: scalar/MFMA
projection shapes, FP32 head/argmax at rows 1/17/32, Q16/KV8 mapping, append at
page boundary 15/16 and physical page 511, attention through logical position
8,191, full outputs, immutable inputs, inactive tails and surrounding guards.
The worker closes unforced and all eight GPUs are idle before/after. Native
result SHA-256 is
`573256efddc1686a861c50dc9eedc23894a4b6f44571a57a5f5d9307c3a5c7d0`.
These synthetic fixtures do not modify or qualify target kernel geometry.
The completion-bound paged driver is integrated at `fa93f8f` from `58ed5c5`.
It admits the exact draft model and closed v10 image before allocation, checks
tied embedding/head identity, and seals actual consumed-input completion,
including the distinct full-acceptance catch-up row. It does not certify
caller-supplied proposals as draft argmax outputs. Exact-source host gates pass
526 adapter test invocations, 19 explicit ignores, six doctests, one separately
run emitted-image admission test, strict Clippy/formatting and release builds.
Aggregate SHA-256:
`12590c09eb3d359b99b1fded99ccb700a766b20c2f7d924777822cd1d3cf6536`.
Recording transports are synthetic. Native paged model checks, autoregressive
proposal generation, multi-round numerical validation, serving integration and
speculative load measurements remain required.

## Continuous Measurement

The V3 collector keeps one constant or seeded-Poisson arrival schedule across
warmup, adjacent fixed measurement windows and a loaded guard interval, followed
by bounded drain. It does not drain between windows. Completion-window usage
and arrival-cohort failures are recorded separately; failures are never dropped
to improve reported rate. The integrated collector gate passes 90 tests.

The separate paired V3 analyzer replays raw observations against pinned plans
and source identities, checks actual wall-clock containment, retains failed
windows, and invalidates comparisons on client/resource/cleanup faults. Its
combined 109-test gate includes 19 new paired-series methods. Descriptive
whole-start bootstrap intervals require at least three fresh pairs and preserve
adjacent-window correlation. Structural gate checks are not performance
qualification: token ITL, stationarity, equal baseline tuning and held-out
primary-suite comparisons are still outstanding. No matched vLLM/SGLang run
has occurred.

The separate 128/128 baseline driver is frozen with 79 CPU tests and 12 syntax
checks passing. It binds the exact cached images, tokenizer/workload, resource
limits, owned cleanup, startup dtype observations and unchanged timed client.
These are synthetic/source-inspection gates only. The user approved the two
sequential baseline launches on September 11. The independent target reference
now passes two complete 128-input/128-output generations: BF16 decoder weights
and eager attention, with separately copied FP32 head operands/output. Every
full-vocabulary logit is finite, and repeated output IDs, UTF-8 and complete
logit-vector digests match exactly. The offline reference container was removed
unforced and all eight GPUs were idle before/after. Reference SHA-256:
`cd7f512537e44d677244c606cba919e57747e8c26caf3c0df4a0aca7acc0eb8b`;
raw producer SHA-256:
`fa725662e8ec9e7ab4f30c981c048167a5b63a4fda972d4fe570527b80fcf314`.

The reviewed initial plan `15faf5c6` binds that reference, controller source
`656edb2` and worker source `c110ac55c` (unchanged code closure on public
`216822284`). Its first vLLM launch failed before task/model startup because
Docker's local log driver rejects compression with a one-file rotation limit.
The exact owned container was removed, no numerical/timing output was produced,
and all eight GPUs stayed idle. The failed receipt is retained; a separately
versioned launch-only correction disables compression without changing log
limits. Its 80-test gate passes. The corrected vLLM attempt reached a healthy
endpoint, but the startup observer targeted the unused legacy runner instead
of this image's default V2 runner. No diagnostic or timed request ran. Its
container was removed; the immediate idle check failed on activity counters,
and a later all-eight-GPU sample was idle. Both failed attempts remain retained.
The next revision will bind the actual runner, use a bounded graceful shutdown,
and retain bounded post-close idle samples without weakening identity checks.
Ferric's unchanged 128/128 cell is running separately while that baseline-only
correction is prepared. Native framework API/observer validation and measured
comparisons remain outstanding.

## First Matched 128/128 Results

On September 12, the unchanged Ferric v2 cell and corrected vLLM v3 cell both
completed ten warmups and thirty measured requests. Both before/after diagnostics
match all 128 independent output IDs and decoded UTF-8 bytes exactly. The timed
client replay matches output bytes and token usage for every request. Both owned
process/container teardowns pass, followed by all-eight-GPU idle checks.

| Engine | Mean TTFT | Mean TPOT | Output tokens/s |
| --- | ---: | ---: | ---: |
| Ferric, controller `656edb2` | 3705.967 ms | 506.969 ms | 1.879805 |
| vLLM 0.28.0 | 19.243 ms | 4.414 ms | 220.558111 |

These are descriptive single-start, concurrency-one, closed-loop finite cohorts,
not the repeated primary suite or a confidence-qualified framework comparison.
The interval includes inter-window gaps and final drain, excludes ten warmups,
and uses the unchanged client aggregator. The shared checkpoint is Qwen3-8B
BF16 with an explicitly selected FP32 output head, TP1 on physical MI350X GPU0,
8192-token context, 128 input/output tokens, greedy fixed-length output, and
speculation/prefix caching disabled. This is not a comparison of stock BF16-head
defaults. Ferric is substantially slower on this cell; no competitiveness or win
claim follows from the earlier short-canary improvements.

Ferric p50/p99 TTFT are 3684.923/3963.110 ms and TPOT are 505.073/516.929 ms.
vLLM p50/p99 TTFT are 19.486/20.167 ms and TPOT are 4.414/4.420 ms. The cohort
durations are 2042.764846 and 17.410378 seconds respectively. Pair replay SHA-256:
`f6fb4811915afce5d004320f3d6f1d455b7a3a9093b22858ac9c7fba925a254f`.
Ferric receipt SHA-256:
`6192619afddae282d66813c9c9f50aee98c0cac3bde14f0507c8e59759781a2e`.
Source, raw requests, diagnostics and lifecycle evidence remain retained under
the local `ferric-matched-result-summary-v1` and `ferric-matched-final-plan-v2/v3`
evidence directories. No profiled run is substituted into these numbers.

The first SGLang v3 attempt failed before model execution: AITER tried to copy
unreadable image-bundled cache files into its owned user cache. Its exact owned
container was removed and all GPUs returned idle. It produced no numerical or
performance result. A separately versioned startup correction is under review;
the failed receipt remains retained. Revision v4 passed cache import but failed
on the observer's Python-3.11-only hashing API. Revision v5 replaces only that
hashing helper with bounded SHA-256 reads and passes 101 CPU tests. Its actual
model load and observer completed, but graph capture failed opening a FlyDSL
lock in the read-only image cache. Both owned containers were removed cleanly,
with all eight GPUs idle afterward. Neither failure produced a numerical or
timing result. The next correction must retain the selected backend/graphs and
use an owned writable cache. Speculative serving remains open.

## Wave Plus Ordered And Private Proposals

Exact controller source `f65c4603f936b3cf5d009991bc19a9645ecdb604` adds the
explicit wave-attention/ordered-batch combination without changing defaults.
The same controller binary and frozen worker run a baseline/wave/wave/baseline
sequence on the four-request/eight-output canary. All four runs match every
reference token and decoded byte, cancellation and prefix reuse, 34 physical
rows, five batches and 3,077 packets. Owned workers exit and all eight GPUs are
idle before/after each run.

| Attention With Ordered Batches | Run 1 Rate | Run 2 Rate | Mean Rate |
| --- | ---: | ---: | ---: |
| Baseline | 5.495401 | 5.507684 | 5.501543 |
| Wave | 6.178240 | 6.174699 | 6.176470 |

Rates are output tokens/s over the fixed workload window. The mean-rate
increase is 12.27%. Mean arriving-short TTFT/TPOT change from
284.777/280.808 ms to 253.348/247.320 ms; reuse-prefix changes from
262.911/228.906 ms to 229.464/194.947 ms. These are two fresh runs per mode,
not confidence intervals, serving results, per-kernel GPU timings or a new
default. Both modes retain host-timing instrumentation, context 64, eight KV
pages, 16-row chunks, prefix caching, and no queue rollover. Do not substitute
these values into the separate unprofiled HTTP 128/128 comparison.

Evidence is retained under `ferric-ordered-wave-evidence-v1`, with run tags
`ordered-wave-f65-{control,candidate}-r{1,2}`. Controller SHA-256 is
`d1c006b89f918541820a3335ba0f931a7394d1f7fa1a4eca2de5e4653e824237`.
The exact wrapper is `7512e47a4d664ce401e644c935238eba9c4a21306d3c5322f9940dcfbbc131cc`;
the independently checked composed comparator is
`1caf29f9f957263ae563f9fd66207d9ba7a361650d67248fe75a6d332b68590a`.
Batch-five head spans are roughly 23 ms; the aggregated host trace cannot
attribute its longest dispatch to a particular kernel or isolate GPU duration.
Source inspection identifies the lane-zero serial FP32 vocabulary argmax as
an additive parallel-reduction candidate. No argmax speedup is measured yet.

Private autoregressive proposals are integrated in `82703c6`: target K+1 is
reserved before draft K, each next draft input comes from the previous sealed
device choice, and individual completion evidence is retained through target
verification and accepted-prefix settlement. Full acceptance still requires
last-candidate catch-up. No user-supplied candidate vector or synthesized
aggregate completion substitutes for actual draft execution. Host fixtures
cover K4/K8/K16, rejected/replayed/foreign choices, exhaustion, failures and
multi-round catch-up; native paired-model execution and speculative serving
remain open.

The exact integrated `82703c6a788e99e23ef8d4204ea5c976413ae6e1` gate passes
568 adapter test invocations, 22 existing optional image skips, seven doctests,
31 Python tests, formatting, strict all-target Clippy and release on mi300x.
All 1,040 source files remain unchanged; all fe2o3 packages resolve to public
`216822284`, freshly rechecked September 12. Aggregate SHA-256:
`81a847daa89ab1b7829b3229a197e6ace567ea84585f54be2d9dc2248a9d0863`.
Archive SHA-256:
`61971250985491f8d902df090f949748e571f20ea313cd43dd76aa66fa9cf970`.
The completed draft worktree was removed after archival; branch and evidence
are retained. This is not a new Verus proof or native speculative result.

## Published Checkpoint

The separate Pages-only branch was freshly rebased and published at
`4a3cbd4efe1df08ccdf7e3e59e19036f9f14469d`. Workflow `34651357690` succeeded.
All seven live static assets match the validated files byte for byte. This
checkpoint adds wave-attention, wait-policy, full-allocation KV and native
ordered-submission observations without changing frozen historical performance
data or claiming a baseline win. It predates the model ordered-submission
regression above. Its private build stage and completed worktree were removed
after archive verification. Ferric implementation commits remain local;
fe2o3 main was independently rechecked at `f85bb375e` at that earlier checkpoint.

The subsequent Pages-only update is published at
`4200318970ddd0edebf24761cccdb4bb18c22794`. Workflow `34659538722` succeeded;
all seven live files match the validated artifact exactly. It adds ordered
regression/recovery and fresh serial controls, standalone/native draft results,
the consumed-input host driver, exact aggregate and V3 measurement checks, and
the independent 128-output reference. Historical performance data are unchanged.
The final browser gate passes eight named viewports, every width from 320 to
1440 pixels, and 101 scope-mutation checks. Live receipt SHA-256:
`d56134f1a90773ed39f0dfa7ff7bff2a74946b6f25be8555cd7a435e80ff9d35`.
After archiving, the owned Pages worktree and remote stage (1,390,612 KiB) were
removed. No implementation files were pushed. Public fe2o3 main was freshly
rechecked at `216822284` during this update.

The next Pages-only checkpoint is live at
`8561285d68c066cfae53e5105b981644bdaba6d0`; workflow `34687558585`
succeeded. All seven assets (401,240 bytes) match the validated files. It adds
the matched Ferric/vLLM cell, exact head-operand policy disclosure, independent
draft reference and first native draft case, plus private-proposal host status.
It predates the completed eight-case matrix, wave ABBA and SGLang r7 below.
Live receipt: `e5d84d7a3bcf8447668a93deaf43b1c8341a20872a55af978178f2876a4f08b5`.
The completed Pages worktree and 1,245,156 KiB remote stage were removed after
archival. No Ferric implementation was pushed.

## September 12 Follow-On

All eight paged-draft native cases pass: baseline and MFMA projection, full and
tokenwise prefill, two repetitions each. Every case emits the fixed two-token
` Paris.` continuation with exact IDs and bytes, expected dispatch counts,
cursor retirement, normal worker close and all-eight-device idle checks.
Tokenwise prefill diagnostics are not generated output. Controller `ac2682a`,
image v10 and worker `c110ac55c` retain their frozen identities; these are not
speculative serving or timing measurements. Eight-case archive SHA-256:
`7ce5999a57f97285a6519a4376453992271567518ddafeac7fd648229ded6c3b`.

The additive v11 argmax preserves finite FP32 maxima and lowest-ID ties without
changing any head arithmetic or default route. Source `67b7290` passes eleven
numeric/property and five ABI/source tests; exact-216 emission/replay and an
independent ELF review pass. The new image is
`122ee2a146b1f0735a315764c5eb1e2ff9aa83c26f5800b42ce6d69b0602236b`.
All fourteen native fixtures pass, covering full-vocabulary finite inputs,
every lane at three scan boundaries, 1/2/16/17/31/32 rows, signed zeros,
subnormals, random finite bit patterns, inactive tails and all guards. No
deliberately trapping GPU fixture was launched. Wrapper receipt:
`4c4fda5650912f8b26198ee91f1d27c27ca50e18898ee8cc9a295d2d45c7e3ac`.
Native archive: `9ee2d38456137241e2693e6fd9e991f4cf2bc34a6fca8a06f75617ca2fb9abd6`.
This is not a model speedup; the adapter still uses the frozen v8 argmax.

The separately gated paired K4 canary is frozen at
`ae355e52e7674f027ca63f561dc6ec2b31ce5b3c`. Its exact-216 gate passes
594 all-target test invocations, eight doctests, strict Clippy, formatting and
locked metadata. The 25 existing image-gated skips include three repeated
worker-module tests in the new binary. All 1,053 committed files match the
before/after source manifests. Aggregate:
`f67227b267089f52ec7618dc14bc2bcc2b0051f6f21645c13acbca2ac8a827e1`.
The new closed CLI performs two genuine K4 rounds over independent target and
draft workers/pools; it does not accept caller-supplied draft choices or
acceptance counts. A separately decoded ten-prefix adapter retains the complete
unchanged 128-token target oracle. Its external checker passes all 25 synthetic
acceptance pairs and negative trace tests. Two fresh native runs now pass:
each independently observes `[4,4]` acceptance, both genuine catch-up steps,
the exact ten-token target prefix, 6,136 target and 8,610 draft packets, normal
worker closes and all-eight-device idle checks. Catch-up was not forced by
replacing choices. Native rejection/rollback branches still need additional
workloads; these two runs are not speculative serving or speed qualification.
First native wrapper: `70d6ff75e7f8e69d4dfe7b026ed6d854d02e0813744e114c22aaa83f06d17865`.
Second native wrapper: `bb662c2dfe6be7a0661e66bf9d2f65f201136165362d001105f7d83ed0373926`.
Both-run archive, including the frozen binary, wrapper/checker, prefix producer
and sixteen passing CPU Python tests:
`8d9997bb81a9827fbe408b256125d9f1fd72c6c676e68f865a59f9976102c927`.

SGLang r7 fixes the startup-only private AITER module lookup while preserving
original packaged-module precedence, model arithmetic and serving flags. It
starts and finishes the ten warmups and thirty measured requests. It remains
rejected: every response has 128/128/256 usage, but an additive
`reasoning_tokens: 0` field fails the frozen whole-object usage check; separately,
ten measured texts and four warmup texts differ from the exact reference.
The native before diagnostic matches, but the identical after request diverges
at output index four. Fixing usage parsing cannot admit this numerical failure.
Normal exit, container/cache removal and all-eight-device idle checks pass.
Receipt: `69bba54516149f01ca0bf52e415f7577b5b9d5e8addcd7ea8afe510196118a6a`.
CPU-only numerical audit, without latency calculations:
`e228350363d13b0e3639c3c21f5fa93c1da1fc060d1ec426c0c3f8ae924df412`.
No SGLang timing enters the comparison, and earlier failed revisions remain
preserved. In particular r6 required forced teardown and is not a clean sample.

Upstream fe2o3 advanced to `8efd4fd416d1ffae7a718144e4d299fe3c8f7590`,
tree `93a51b4af0287ccb51dec07fe431dae71fc55d0f`. The 29-file delta is
compiler, macro, analysis and associated test/policy work; KFD runtime, device
SDK and Cargo lockfiles are unchanged. Active repin `ddd3aaf` passes locked
metadata, explicit source scope, three identity-only dependency inventories and
the audited manifest/lock review-hash refresh. Its audit is
`0b1e21160bef449b45c3cc79bc61a1764f3558bb9167db3d1ddcadb560313c97`.
Combined source `9c2e98e` adds the paired canary; its latest full adapter test and
release gate passes, as do the source/verifier, inventory, negative and release
policy checks. Its separate v11 kernel gate passes eleven numeric/property and
five contract tests, strict Clippy, formatting, locked metadata, nine Python tests
and the fourteen-case scalar self-test. All 1,055 source files remain unchanged.
The exact latest-8efd one-root emission/replay and independent ELF review pass:
36 explicit/296 total argument bytes, Wave64, no scratch, LDS or spills.
The LLVM worker is explicitly reused from 216; it is not relabeled as rebuilt.
The new image is
`de9db78c0ef7ad5d84fc903d41ee9db59026113d79e23f12d3909761363b9390`.
All fourteen native cases also pass with this latest image, normal worker close
and identity-bound all-eight-device idle checks. Raw native report:
`ba0b459c265bf96c6d2dce6b3ecb20af89046a860347b5ceb268878f56accc5b`.
The archived prelaunch manifest binds actual source/image/tool identities;
the pass-only wrapper JSON is intentionally identical to the earlier pass.
Latest native archive, including the additive wrapper and nineteen passing CPU
wrapper/checker tests:
`dccf98beae535e1e400b7ba6765a70cfa2e5fa7aa1b8bcb0b127a8f46a66fd4f`.
There is still no routed model gain or default change. Frozen 216 and older
artifacts retain their own identities.

Six completed source directories and four completed evidence
directories were archived, hash-checked and removed from the owned mi300x stage,
reclaiming 246,892 and 341,524 KiB respectively without deleting its active cache.

The latest `9c2e98e` paired controller is separately frozen as
`d0bef75dec8e9287aaa21acba14e526f6770634e8d2e0e3ff244df58798759e6`.
Two additional fresh native runs both pass with actual `[4,4]` acceptance, two
catch-ups, exact ten-token IDs/UTF8, 6,136 target and 8,610 draft packets, normal
worker closes and all-eight-device idle checks. Frozen images and the c110
worker retain their actual identities. Latest-run wrappers are
`a8a379b2ae25a4efe4d10fbaf7f14286113042dd2e6b1a884b6b8303aaf96ea8`
and `28c731e8ab742b577680ace747d5a0374c2c1f2d512fb6e9813ae2d94832144a`.
Both-run archive, including the exact binary, checker, wrapper and references:
`e924a2c7aee87e063acd601d3a0cd40c068d82369a7ab7e053f4a13eada01f4a`.
These are separate latest-build observations, not relabeled 216 runs, broader
native rejection coverage, speculative HTTP serving or timing qualification.

The latest policy build first stopped at its reserved pre-cap threshold while
compiling verifier tests; no result is claimed for that attempt. After archiving
the exact source, logs and three release binaries, package-only cleanup removed
2,650,132 KiB of completed adapter outputs without deleting dependencies or
source/verifier state. The retry passes within the unchanged 9 GiB stage cap.
The unchanged negative-policy script used separately bounded temporary scratch.

Pages-only commit `75d8c0351d0223e60fc02360d9a1ddc56bd1db70` was reviewed,
rebased onto freshly fetched public main and published without implementation
files. Deployment `34691255551` succeeds; all seven live files match the
410,270-byte reviewed artifact. The frozen performance ledger is unchanged.
Its new native checkpoint preserves actual 216 provenance and distinguishes
active development at 8efd. Both evidence replays, 142 new mutation cases, all
existing site checks, eight named viewports and widths 320 through 1440 pass.
Live receipt: `bfe0b9ded38ac399b22fc7d185428256359c932e688ae6fad878f5ae619ca221`.
The completed Pages stage and clean worktree were removed after archival,
reclaiming 1,288,084,362 remote bytes.

## Bounded Decoder Attribution

The four frozen f65 host-timing files show that the wave candidate's disjoint
decoder-plus-local-reduction scopes account for 92.44-92.46% of batch wall time;
the entire output head accounts for 7.32-7.36%. Removing that entire head at zero
cost, with all other latencies unchanged, would allow at most about 1.079x on
this five-batch workload. This is an arithmetic bound, not an argmax prediction.
The final single-row decode's whole-head share is 12.11-12.32%, also not all argmax.

Ordered decoder groups drain inside the scopes named `collective_attention`
and `collective_feed_forward`; these are not network-communication costs.
Nested IPC/flush/response-wait spans overlap and must not be added to outer
stage totals. Setup is excluded. The pinned jq replay and note identify the
four input hashes; note SHA-256:
`a842f4bed38a476af6bb9b4a5e411c08747c24fc55b4064e8d5896d694c2fe5b`.
This tiny instrumented workload is not matched HTTP TTFT/TPOT or a GPU profile.
The opt-in argmax path needs model measurements, while larger improvements
need finer attribution of decoder projections and dispatch overhead.

## Opt-In Argmax Model Route

Source `f662d541e1ab9a47ef8dada410fffd964851d297` is integrated locally at
`a8c87bc3c67ee36f89f1004462f745a48f947aa8`. Only two progress documents differ
between the tested source and that integration. The new target-only constructor
admits the separate v11 image before allocating unchanged v5/v8 buffers. Both
comparison modes load identical images, and only the explicit argmax selector
changes. Serial v8 remains the default; the existing serving and paired CLIs
are unchanged. See [the canary contract](TP1_ARGMAX_CANARY_V11.md).

The final 29-step host gate passes 633 adapter test invocations, eight doctests,
strict Clippy, formatting, release build, explicit latest-image admission and
compiled workload goldens, 38 source-gate tests, 31 protected verifier policies,
and five byte-identical inventories. Two new ignored fixtures were explicitly
executed; 28 image-gated invocations remain skipped. All 1,059 source files and
the binary remain unchanged over the final gate. Earlier attempts caught a
test-only Clippy lint and two engineering dependency allowlists; their failed
logs are retained. No protected identity was regenerated and no Verus proof
was added. Final aggregate:
`3cdbddb6742b75c3f3d0ff96376609c2d05bd7589ef1d574956f57cf5cddff57`.
Source archive:
`1eb904b3a3e1f2f57f3809c7136376d1e8b489aaf15061706bd0241190a7365f`.
Controller binary:
`053a342ae270e2fdb6f7e2572af865cfc90e3bc4a4c2875e6a659d782df8423a`.

An independent fixed-reference checker and root lifecycle wrapper pass 36
combined Python tests on mi300x, including 560 rejected scalar mutations.
Both Rust and Python agree on the mode-independent 8/128 workload hashes.
Checker and wrapper pins are respectively
`eb7a5f7d243675fac25c318d640e50c6a5aad1d39b7bc3eedefc5f5316964a76`
and `186e2de9c64a278d2e16458ca4e24b0f8eea2b0ed20fec73e460634df7ab2daf`.
The predeclared native plan is one full128 serial/wave pair, followed only on
success by an eight-output serial/wave/wave/serial sequence. All use the same
binary, images, full128 prompt, context256/pages16/chunk16 and runtime policy.
Every actual autoregressive token, decoded byte, packet counter, pool retirement
and worker close must pass unchanged checks. Measurements exclude setup and
are instrumented native host diagnostics, not HTTP or GPU durations. Native
completion and performance results remain pending at this checkpoint.

The previous latest `9c2e98e` combined gate is archived separately; its aggregate
is `02e7cc66091de4ed891af66896a47db103a5f92bda2be96fba9a33a03172452b`
and archive is `bb71cab213b1d90777173ba8867b1b5d656b5157195c743bfb65b4550255ff4c`.
Completed kernel stages were fully archived with matching all-file/symlink
ledgers and removed after the latest image was copied to the active host stage,
reclaiming 1,310,208 KiB. Their final archive is
`0a525b4281ffc3271be9ac816368a1711e5137c11948fc9d79aef426c97caa74`.

Read-only inspection of the identical c110/8efd KFD source tree identifies
N+9 operational currentness checks per steady-state ordered frame containing
N dispatches. This is a source-derived repetition count, not measured cost.
An existing `--runtime-profile` diagnostic can separate cumulative counter
deltas without changing core code. Preparation, publication, wait and command
timers overlap; wait includes host checks and sleep and is not GPU duration.
The source-line audit is pinned as
`a573988b7a4e2764706a182d60ea8ab9e77cddce25433247631959d5cbbecc77`.

### Retirement Correction

The first `f662d54` serial128 run completed 83,139 packets and 135 batches but
was rejected at the final `pool.is_empty()` assertion. That documented API
means never admitted or reserved, not released ownership; retirement correctly
does not rewind monotone IDs. The worker closed normally and all eight GPUs
were idle, but no Observation record was emitted. There is no admitted full128
parity or timing from this attempt. Rejected wrapper:
`cf9543fec83c68a5d88cbd14b808896b87c5a8f47d31e55138e6e25b2cf488ee`.
Rejected native archive:
`b2dbbd7b5006b45d991130f7c1687657b84994716961c3c4bd4a2d58cca62005`.

Correction source `d9a2705e6b2f3a8d8dd173b9700a63446b611016`, integrated
locally at `17dcaa4`, preserves the initial freshness guard and pool API. Its
production retirement helper requires successful retirement, invariants,
16 free pages, zero sequences/retained/cached/quarantined pages, and exact
stale-sequence rejection. Regressions cover actual 135/255-input metadata
commit flows, nonfresh monotone IDs, zero prefix reuse, and pending/submitted/
quarantined rejection. The reference, trace schema, kernels and clocks are
unchanged. All 29 gate steps pass: 637 adapter invocations, eight doctests,
explicit image/reference fixtures, strict Clippy/release and final 38/31 source
policies with five unchanged inventories. Aggregate:
`497de7d70a785c39d99f9b919e7906542896928e27559fad31bb0f9f7e7df861`.
Controller: `d298c5ea6b0dfd64c66722b12ffba1f5e8498bf215b15b9ca6d0d88da56f890f`.
Full gate archive:
`2e61230d7c5bba7908ff9298da0dfbf2fbb3d0a409fe03016b3fde5d60971ee5`.

A replacement six-run plan was frozen before corrected output at
`5781935439ab23f4e234135ab14fa4ea425e71d210919980afe6de46a6f10c79`.
It uses new d9a2705 tags/binary, the unchanged root wrapper/checker and exact
same workloads and arithmetic. The original attempt stays excluded. Corrected
native results are pending; a successful host gate is not model qualification.

### Runtime Counter Observation

A separate frozen f65/c110 ordered-wave run adds only the existing runtime
profiling switch. It passes all fixed tokens/bytes, 3,077 packets, 34 physical
rows, five batches, normal worker closure and all-eight idle checks. The
independent replay matches its report, preserving actual profiling=true and
never extracting ordinary performance metrics. Native diagnostic:
`d4ebc18f20fb1697237503a7f94b38ccadac4853b127157bf5e106466cdb91c8`.
Native archive: `dbf7b82756a1e572b13b6827cec764b0ab1ecec38ff0af2b060be014f0c86eb1`.
Independent replay: `c90e86c9da0da08550fb8ef108685033986398e173b062ad0a09fd56c4b500bb`.

Worker deltas are 1,358.312 ms command wall time, 139.592 ms operational
currentness over 7,507 checks, 7.309 ms full currentness, 60.078 ms preparation,
14.345 ms publication and 1,217.956 ms waiting. These overlap and are not an
additive breakdown or GPU duration. Operational currentness is 10.277% of
command span; all preparation is only 4.423%, so preparation-check coalescing
is not the highest-impact demonstrated target. Cached kernel admission adds
zero workload admissions/time.

There are 737 completed wait loops and 12,624 polls: 11,887 unsuccessful polls
request 594.350 ms of nominal sleep. Actual elapsed sleep is unmeasured, can
be extended by scheduling, and overlaps GPU work. It is not established waste
and cannot be subtracted to derive kernel duration. Finer wait observation and
kernel analysis are the next diagnostic priorities; core checks are unchanged.
The detailed source/counter note is
`d6a787adf81101a35f6806c1a74041de65925dd637e7cf9350ba852be4de0984`.

### Corrected Six-Run Result

All six corrected d9a2705 runs pass. The full128 serial and wave-v11 runs
each match all 128 generated IDs and UTF-8 bytes, 83,139 packets, 135 batches,
cursor255, retirement and normal worker closure. The following short8
serial/wave/wave/serial runs each match all eight generated IDs/bytes,
9,219 packets, 15 batches and cursor135. Every run has fresh process/resource
receipts, unchanged binary/input pins and identity-bound all-eight idle checks
before and after. The rejected f662 attempt remains permanently excluded.

The only experimental selector is target FP32 argmax. Both modes use the same
v5 MFMA projections, baseline attention, v8 FP32 head, v11 admitted image,
TP1 worker, 128-token prompt, context256, 16 physical pages, 32-row workspace
and 16-row prefill chunks. Prefix caching, speculation and ordered submission
are off. The host clock starts before prefill and ends at generated-token
commit; setup, final detokenization, retirement and closure are excluded.

| Diagnostic Cohort | Mode | TTFT (ms) | TPOT (ms) | Workload (s) | Output Tokens/s |
| --- | --- | ---: | ---: | ---: | ---: |
| Full128, n=1 | Serial | 4182.023 | 553.420 | 74.466305 | 1.718898 |
| Full128, n=1 | Wave v11 | 3966.503 | 528.962 | 71.144681 | 1.799151 |
| Short8 ABBA, n=2 mean | Serial | 4337.997 | 489.293 | 7.763047 | 1.032721 |
| Short8 ABBA, n=2 mean | Wave v11 | 4034.981 | 428.179 | 7.032234 | 1.137693 |

Descriptively, the full128 pair reduces TTFT 5.15% and TPOT 4.42%, with output
rate +4.67%. Short8 ratios of arithmetic means give TTFT -6.99%, TPOT -12.49%
and mean output rate +10.16%. The two individual short TPOT reductions are
16.79% and 7.72%; n=2 does not establish a stable improvement or confidence
interval. Mean per-run output rate is not tokens divided by mean elapsed time,
and ratio of means is not mean paired speed ratio.

The full128 output-head host span is 3.004575642 -> 0.384304466 s; its nested
argmax span is 2.763174148 -> 0.147159205 s. The latter's 18.78x ratio describes
host scope, not an isolated GPU kernel speedup. Broad attention spans remain
57.598240118/57.189654603 s. These spans and IPC timings overlap; do not add
nested values or claim the total improvement is fully explained by argmax.
The earlier +12.27% wave-plus-ordered result is a separate workload/profile,
not a multiplier for these observations. Default and HTTP routes are unchanged.

The frozen reducer passed 31 CPU tests with its manifest helper and checker.
Root independently compared all 60 manifest file hashes to original mi350
files before authorizing replay; the unchanged reducer then admitted all six.
Root also independently checked per-run values and arithmetic means.

- Authenticated manifest: `d7efd64eb1aad2f55e5639b97fdae52b9bd60e2a75c56d5c4a346c12fd3448f1`.
- Summary: `064e1ced84a0ac4e275225bcc9243a161462c19f907f9ca2080048daafc7eaab`.
- Reducer: `d112d5113657867417bc8358ddbbf19a2c1a918a658e146c0522b1860b7863d7`.
- Raw six-run archive: `d203ad9443cfae86ea10672330c9aa09fa1ae8388915a1305bf86ae5f7e4e4be`, matching a second independent remote archive stream.

These are instrumented native controller-wall results, not HTTP or GPU-clock
measurements, competitive ranking, default promotion, new Verus proof or M1
completion. The matched Ferric/vLLM cell and rejected SGLang result above are
unchanged; Ferric remains substantially slower.

### Released Stages And Next Kernel

After verified source/log/executable custody, the completed correction
worktree and mi300x stages `ferric-large-kv-host.SfMXxwBa` (7,237,660 KiB)
and `ferric-paired-canary-cpu.oBWfSYP9` (22,016 KiB) are removed. The SfMX
closure archive excludes redundant Cargo target bulk; its SHA-256 is
`f6a31f6749b7504e890f08c41c72e76bac8dddd35cb8aeed02b3192d623b80ca`.
The complete root Python-gate archive is
`affff2bbd7a0cf1ed1eab5635a75a98d11df6406acccb7bbc37c9c8a7904b1e2`.
Both match independently repeated remote archive streams. Audits found no
process references or foreign-owned entries; nondumpable SSH/PAM services and
foreign processes were not fully inspectable. Cleanup relies on the released
private ownership/leases, not a claim of privileged process visibility.
Shared models, caches and unrelated worktrees are untouched.

A separate additive v12 prototype at `ef619051248f469a13f02f8054cbf50ef0e9f9e2`
specializes TP1 one-row down projection and groups pairs of K16 loads while
preserving the existing single-accumulator order and weight layout. Nine
focused host tests, format, Clippy and locked metadata pass on mi300x.
Its host archive is `81a6ed8ea2d096bc4417676c95d5348851bdb2cedb962d6aad7d6411717ce11a`.
It is not integrated into a route or default. The subsequent emission and ISA
results are recorded below; native bitwise comparison remains open.
Active core remains public `8efd4fd` / tree `93a51b4a`, freshly checked after
the native runs. No core code was changed or pushed in this continuation.

### V12 Emission And Cleanup

The initial emission rejects the original v12 source with
`FE2O3-TENSOR-LAYOUT-002`: dimension predicates precede MFMA in the compiler
control-flow analysis. Source `73617b7cee6dd0fa65e40008f91b8fb7a51955f5`
moves those predicates after accumulator extraction and before stores, matching
the supported v5 ordering. No guard is removed; required launch admission and
the earlier shape/slice bounds remain. Ten focused host tests and emission/
exact replay pass with latest fe2o3 `8efd4fd`; the reused LLVM worker retains
its actual `2168222` provenance. The rejected first attempt remains archived.

HSACO `2748bc178ac01c32a9a95d929ed67ce5ba358e0525ca2d8490d07c112db20c85`
has 32 VGPRs, 34 SGPRs and four AGPRs, with no scratch, LDS or spills. ISA
retains paired K16 loads and the original single-accumulator MFMA order, but
waits for all loads before the pair of MFMAs. It does not establish software-
pipelined load/compute overlap, occupancy improvement or a performance gain.
The full final archive is
`73f454b6f7dd3437f157b6f86fe8bd42d5af61373fa30308d4ba94c45e50a2ca`;
emission receipt is
`e07f6f3bfc6b05606a20bdbcbd8865994b920b0a384b9809ca84eb57d171f8b9`.
Both completed remote roots and the clean local worktree are removed after
verified custody, reclaiming about 1.29 GiB remotely and 41.2 MiB locally.
The branch and evidence remain; no native v12 parity or route integration is
claimed.

### Seven-Group Attention Diagnostic

Source `809af24513f71cde7439bd8f7cfbff4c11351ade`, tree
`29f9a335f2317b570c67f1f8958aebb339f00a3e`, adds seven nested host spans
without changing kernels, commands, arithmetic or runtime flags. All 32 remote
host-gate steps pass: 638 adapter test invocations, eight doctests, explicit
image/reference fixtures, strict Clippy/release, 38 source-gate tests, 31
protected verifier policies and five unchanged inventories. Thirty adapter
invocations are ignored in the general suites, including the two separately
executed fixtures. All 1,059 source files retain exact before/after custody.
Aggregate receipt:
`d371ec67b801b5be3a210ae8b84c88c279d5e705b75cfaa9d9e9fd36f25b1651`.

One fresh full128 native run uses the same baseline-attention, MFMA, FP32-v8,
wave-v11-argmax profile and unchanged oracle. All 128 IDs and decoded bytes
match; 83,139 packets, 135 batches, retirement, normal worker close and exact
all-eight idle checks pass. The independently reviewed reporter admits 945
group records, with 4,860 observations per group:

| Host Operation Group | Seconds | Percent Of Attention Parent |
| --- | ---: | ---: |
| GQA math | 41.068390375 | 72.5853% |
| Input normalization | 4.165605679 | 7.3624% |
| Q/K/V projections | 4.119716878 | 7.2813% |
| KV append | 3.637546242 | 6.4291% |
| Q/K normalization | 1.530490487 | 2.7050% |
| Output projection | 1.275791124 | 2.2549% |
| RoPE | 0.778113253 | 1.3753% |

The parent attention span is 56.579512157 seconds. Sibling groups are disjoint,
but their parent and IPC spans overlap. These are controller wall times that
include dispatch and waits, not GPU durations or a cross-run speedup. The next
candidate explicitly composes the existing wave GQA kernel with v11 argmax;
its summation order differs, so correctness and matched performance remain
separate gates. Old selectors, defaults and references remain unchanged.

- Controller SHA-256: `950b42c13f5f2fa548cce4a0c0c029a6b2bb87a1addb9f8cc8119a3467517a82`.
- Raw case archive: `aa951138f204ff7ccf04b5c1c1de35eac21376f6cfafebcb9314e6bd3bee70cc`, matching an independent second remote stream.
- Wrapper receipt: `9b5c000eb0e2462fd40c25b2dd8f7d1d0fcca8d7726a70b391b055476ee3fa52`.
- Reviewed reporter: `43282d1f74fea4730beb26912e4fa90b34b548bf205a0a74e10e15053c55d715`; 19 CPU tests pass.
- Independent diagnostic: `0ebd4dd53827536f8dbca6f4ed3c5be964dfd6a5e1f36d29eb86bfe488f6b633`.

The redundant instrumentation worktree is removed after archive/bundle checks.
Its private mi300x stage is explicitly leased for the next source-bound gate;
any reuse of its Cargo target will be recorded as a warm cache, not a clean
rebuild. No compiler/runtime code change or new Verus proof is implied.

### Published Native Checkpoint

Pages-only `de9980fc299a738e8835d203e0cc8799d219ab31` is published after
fresh fetch/rebase, independent data review, 225 negative schema cases and
desktop/mobile validation. Deployment `34695107216` succeeds and all seven
live assets match the tested artifact. The site records the six-run argmax
results, rejected canary and runtime diagnostic without changing the frozen
matched HTTP table. It does not yet publish the later seven-group diagnostic.
Live-asset receipt:
`998e5370245424bd6ed2d15f691b59392591303102e35a400088b5a5e8fb0527`.
Completed Pages stage/worktree are removed after verified archival. Ferric
implementation is not pushed; only the separately reviewed site is public.

### Finite Native Attention And Combined Route

Additive fixture source `c26c1f4dc267b53f840b2fb01cafd19234596ed2` is
integrated as `8da127b`. All eight native baseline/wave cases pass at active
rows 1/17/31/32: eight complete 262,144-byte output slices, 40 immutable input
slices and 96 guard sides. Dense nonzero QK, token-varying V, permuted pages,
causal masks and inactive tails are checked against controlled finite integer
expectations. Baseline/wave bytes match exactly in each pair. This is bounded
engineering evidence, not arbitrary-softmax qualification or a new Verus proof.
The frozen v5 image retains its actual fe2o3 `3e74a93` provenance.

Normal worker closure and all-eight idle checks pass. Independent CPU replay
authenticates the native report and all 48 buffer records. Native archive
`84022e9f347212bb371a619add6bd735eb35376d5f8327a2b1ecb0be54a9630a`
matches a second remote stream; replay receipt is
`ff826ed5b092c613cd9547a4709e2fcd9672205bb28c9fb7b82ec2d410c65ea8`.
The released fixture worktree and CPU stage are removed after verified custody.

The distinct attention/argmax canary is integrated as `dcbbd5b` from tested
source `ed112e0b8298d1d2070b93dea638e7572ec65ba9`, tree
`e3fd5c5757298312394522bbd80c89eb05e35f73`. All 42 required host commands
pass: 19 under the original harness and 23 under the reviewed short-TMP
continuation. There were 43 actual ed112 attempts, including the retained
all-targets failure caused by Unix-socket paths exceeding the harness limit.
No source changed between those attempts. The earlier strict-Clippy failure
and its two test-only semicolon corrections remain separately recorded.

The complete gate includes 685 passing adapter test invocations, 34 ignored,
eight doctests, both old/new release builds, strict Clippy, explicit v11 image
admission and both workload goldens, 38 source-gate and 31 protected verifier
tests, and five byte-identical inventories. All builds/tests ran on mi300x
using an explicitly warm Cargo cache. No local build or new core change ran.
Gate note: `139e1696797b43049dfa42c8a140c56375714242f1aa62caaf2c5487f306ea9d`.
Composite archive: `f3860c912f5c44b611a2209c14e7eb733c092c9ea7481ddad20237ae7d785dfd`.
New executable: `d0ec403ea4713f4d795ee55221da5282a5668b7ebff283ba56c531f236cc0468`.

Native model qualification is separate: baseline8, wave8, baseline128, wave128,
stopping at any failure against the unchanged reference. These are correctness
diagnostics, not comparison samples. Only after all pass may the frozen full128
baseline/wave/wave/baseline cohort run with the same binary, images and runtime
flags. The root launch wrapper and new raw-protocol checker passed 24 CPU test
methods; the prospective reducer and all checker/wrapper tests passed 38.
All four model correctness cases now pass, in the declared order. Each matches
all generated IDs/UTF-8, packet/batch/cursor counts, retirement, normal worker
closure and all-eight idle checks before and after. The two short runs have
9,219 packets/15 batches/cursor135; the two full runs have 83,139 packets/135
batches/cursor255. Root directly authenticated all forty raw-file hashes
against original mi350 files. The four-case archive matches a second remote
stream: `d00d832c6a4e0e8732b7902c9e6470414324c615589ccf23ab9eb6ae2b4ba45f`.
These are correctness diagnostics, excluded from comparison metrics. The
separate full128 ABBA cohort and independent replay now pass, as recorded below.
Existing six-run argmax and matched HTTP observations remain unchanged.

After verified archive/binary custody, the completed mi300x J8idsFOi stage and
clean ed112 worktree are removed, retaining the source branch and local evidence.
Measured pre-removal allocation was 3,620,618,240 bytes remotely and 43,171,840
bytes locally. Cleanup receipt is
`672799aad29d56aa21fd7a56e175af8fd928e69c04eb314d323225d0527355ad`.
Private ownership and released leases were checked; process visibility excluded
other UIDs and explicitly recorded the restricted sd-pam/cleanup SSH ancestry.
No privileged global process-absence claim or shared-cache deletion is implied.

### Full128 Attention ABBA

The frozen baseline-a1, wave-b1, wave-b2, baseline-a2 acquisition order completes
without reruns. Each case matches all 128 reference IDs and decoded UTF-8,
83,139 packets, 135 batches and cursor255, followed by successful retirement,
normal worker close and exact all-eight idle checks. Tested source remains
`ed112e0b8298d1d2070b93dea638e7572ec65ba9` and executable remains
`d0ec403ea4713f4d795ee55221da5282a5668b7ebff283ba56c531f236cc0468`.
No integrated revision is substituted for that actual build identity.

Only attention changes. Both modes use TP1 on physical GPU0, MFMA projections,
FP32-v8 output head, wave-v11 argmax, device TP1 residuals, pruning, batch cap32,
context256, pages16 and chunk16. Prompt/output lengths are 128/128 with actual
autoregressive token feedback. Runtime caching, operational mode and rollover
are on; prefix caching, speculation, ordered submission, sequences, peers,
large-KV and capture are off. Instrumented controller wall time starts before
prefill and ends at final token commit. It excludes model setup, detokenization,
retirement and close; it is neither HTTP latency nor GPU duration.

| Acquisition | TTFT ms | TPOT ms | Workload Seconds | Output Tokens/s |
| --- | ---: | ---: | ---: | ---: |
| Baseline a1 | 3971.460221 | 521.835731 | 70.244598116 | 1.822204176 |
| Wave b1 | 3095.787923 | 224.460207 | 31.602234295 | 4.050346529 |
| Wave b2 | 3519.556492 | 292.663886 | 40.687870082 | 3.145900725 |
| Baseline a2 | 3974.757081 | 521.080117 | 70.151931942 | 1.824611190 |
| Baseline arithmetic mean | 3973.108651 | 521.457924 | 70.198265029 | 1.823407683 |
| Wave arithmetic mean | 3307.672208 | 258.562047 | 36.145052189 | 3.598123627 |

Ratios of arithmetic means describe 16.7485% lower TTFT, 50.4155% lower TPOT,
48.5100% lower workload duration and 97.3296% higher mean per-run output rate.
Rate is the arithmetic mean of individual 128/duration values, not 128 divided
by mean duration. There are two runs per mode: wave TPOT ranges
224.460207-292.663886 ms (26.38% of its mean) and wave rate ranges
3.145900725-4.050346529 tokens/s (25.14% of its mean). Paired TPOT reductions
are 56.9864% and 43.8351%; the two latency ratios are 2.324847407 and
1.780472899. Paired ratios are distinct from ratios of cohort means.

This admits a descriptive native comparison, not a confidence interval,
stable-gain estimate, default promotion, serving qualification, competitive
win or M1 completion. The four prior correctness diagnostics and all older
argmax/ordered/HTTP cohorts remain excluded. Gains are not additive. Nested
attention and IPC spans overlap and are not GPU durations or an additive
causal explanation of the workload difference.

Root directly authenticated all forty raw-file hashes and sizes against the
original mi350 files: 6,900,569 bytes total. The raw archive matches a second
independent remote stream. The unchanged reducer passes independent mi300x
replay, while a wrong external manifest hash exits1 with empty stdout and the
exact byte-pin error. All twelve source/helper/reference files, forty raw files,
manifest, size ledger, harness and Python binary retain before/after custody.
The prior 38-method reducer/checker/wrapper gate also passes. No local build,
test, formatter or new Verus proof ran.

- Raw archive: `173e6df4b6620f2d1cd3c2a5c806ca6a0d9a53a72b0c80c70f58d979367511d6`.
- Manifest: `c088748c7d9b478e2650bddd3c1ee9abfabd77e772390bf16259825feb139e20`.
- Size ledger: `4ca6c145e756f5e3e2bafa5e6f9b967c9c9ab21ce1b011a9d28215c6567b64ae`.
- Frozen reducer: `ade46c5deb31c093579950550b265f9d35dd7c03d92a30e482d2a1b75e1d66a3`.
- Descriptive summary: `0435191640d3d8669910ce4c6de5906636ee525804c1440fe0ae45bc2fe957f3`.
- Strict replay receipt: `caa0c160ce94b6242a566326e11b30444fd332ff203d4aa525efd24a64efbfa9`.
- Complete replay archive: `b0599fc0743a48df2481213f9b6c14667a759fb28f458468e895ae3d8506baf2`, also matching a second remote stream.

The completed isolated CPU replay stage is removed after verified custody,
reclaiming 7,516,160 allocated bytes. Cleanup receipt is
`931b0e67e662d7c8d032f86697d54342a2d05f6dd260dc3e1bbcded3a8d4fbce`;
restricted process visibility is recorded without a global absence claim.

### Ordered Wave/V11 Driver

Source `ad0663303a642a5d5c5b7e300b4a6f3fab5bced3`, tree
`7f8cc3ce530e5f163aa2f929e12e796676521ddc`, is integrated as `a68f04c`
after all 44 remote host-gate commands pass. The new explicit terminal selector
validates the fresh TP1/cap32, MFMA, pruning, device-residual, wave-attention,
FP32-v8 and admitted-v11 profile plus ordered transport capability before
publishing either selector field. It reuses the existing ten-command attention
and five-command FFN groups at each layer's barriers. Embedding, residuals and
all selected output-head operations remain synchronous. No kernel arithmetic,
worker protocol, reference, default, prior selector or CLI changes occur in
this driver-only revision.

Six focused recording tests cover active-row/output selections, flattened
commands and byte arguments, packet reservations, atomic admission rejection,
legacy exclusion, failure poisoning/quarantine and host timing. The full gate
passes 691 adapter test invocations with 34 ignored, eight doctests, strict
Clippy, 26 adapter source-policy tests, both existing release binaries, explicit image
and workload goldens, 38 source-gate tests and 31 protected verifier policies.
All five inventories match byte-for-byte. The adapter source-policy count for
this driver revision is 26; the later canary adds one more test.

All 1,070 source files retain before/after custody across the 44 commands. The
target was fresh at entry while offline toolchain/dependency caches were reused;
peak post-command stage size is 2,704,984 KiB under the 6 GiB cap. The initial
raw-source formatting check's exit1 remains separate from the passing gate.
Host note: `af96e909805386b722cb0e7422c68a3e917709b9c7a4734db243065287334d72`.
Complete archive: `e638f869557d2508c190dcb3f5d328971ed217249d225562115edfcbe22d6f44`,
matching an independent second remote stream. Integrated implementation bytes
match tested ad066 across adapters, device, crates, proofs and Cargo.lock.
The redundant clean driver worktree is removed, reclaiming 43,245,568 allocated
bytes. Its private target remains explicitly leased for the next warm gate.

The separate canary candidate is raw `144a885`, formatted `7cb6522`, with new
closed schemas and `--submission synchronous|ordered`. It fixes wave attention,
v11 argmax and the unchanged 8/128-token workload, and rejects profile/runtime
mismatches before sidecar creation or model/worker effects. Both source reviews
pass; remote formatting changes only two new test sections. Its full host and
native gates remain separate and pending. Ordered attention/FFN spans describe
command preparation; waits move to the collective flush spans. A new checker
must validate that exact roster without rewriting older raw evidence.

### Published Attention Checkpoint

Pages-only `0fa8c199e9a65d44ce8cd0a35e7e19a0e7b92373` is published after
fresh fetch/rebase and a seven-site-file path audit. All ten remote checks pass,
including 260 negative checkpoint mutations, authenticated raw-file arithmetic,
desktop/mobile screenshots and every width 320-1440. Workflow `34701617457`
deploys that exact source; all seven live assets match the tested 434,648-byte
artifact. Live verification receipt:
`7057da81f741af264f44d59c95a63f16bbd48bafc21bafdfb511df091f7ae3e4`.

The page publishes the seven-group diagnostic, finite fixtures, four model
checks and separate full128 ABBA means/ranges, preserving all prior HTTP and
argmax observations. It makes no stable, competitive, serving or M1 claim.
The failed r4 validator binding collision and six-identifier correction remain
archived; no raw result or metric changed. Final gate archive:
`23e53ff3c6f5ecef828c4ae0cebb896e9c56d7e3f2c6d80b7b04f23c1529eedb`.
The completed owned stage and worktree are removed after full closure custody,
reclaiming 325,460 KiB remotely and 2,748 KiB locally. Cleanup receipt:
`333d1ad953fd30a562fdea08abc480ddf086251a318898d2e6aada94276f5c19`.
Only Pages commits are pushed; Ferric implementation remains local.
