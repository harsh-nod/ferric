# Competitiveness Sprint V1

Status: in progress. No matched vLLM/SGLang result or competitive claim yet.
The prior measurements remain frozen in `M1_PERFORMANCE_SPRINT_V2.md`.

## Active Teams

| Track | Deliverable | State |
| --- | --- | --- |
| Core runtime | Opt-in shared fresh full-topology observation per peer boundary, preserving all per-rank checks | Published `3e3a77284`; 475 library tests, 31 doctests and scoped Clippy pass; native TP2/TP8 producer fixtures pass with the option off and on |
| Kernels | Additive 32-row FP32 LM head/argmax and fast-path integration | Integrated; exact `3e3` emission, 15 native fixtures and both 16/32-budget model canaries pass; wave attention also passes four matched model canaries |
| Serving | Bounded sustained JSONL ingress, wall-clock arrival, token output, cancellation/backpressure | Combined gate passes 289 Rust tests, 16 HTTP tests and strict Clippy; real four-request/nine-token HTTP smoke passes |
| Integration/measurement | Shared streaming benchmark client, baseline identities, GPU scheduling, review and numerical gates | Continuous-window V3 collector and independent paired replay integrated; 109 combined measurement tests pass; matched baseline launch approval still pending |
| Capacity | Explicit larger physical KV pool without changing the logical context/proof boundary | v9 emitted on exact `3e3`; host at `f0cd55f`, 315 ordinary tests plus four emitted-image checks and all nine native fixtures pass; the fixed full-allocation model canary passes, long-context/concurrency qualification remains open |
| Speculation | Draft execution plus target verification and accepted-prefix KV integration into the fast path | Standalone draft passes independent GPU reference; paired KV host settlement and opaque target completion integrated at `228df11`; closed 14-root draft family emitted on `216822284`, native and paged-driver gates pending; no end-to-end speculation yet |
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
two compile-fail boundary doctests, strict Clippy and formatting. Draft success
can currently be minted only inside test fixtures; production cannot complete
a paired round until the real paged Draft06B driver is implemented. This is
engineering support, not a new Verus proof or protected token publication.

The isolated 32-row draft family is integrated at `85a450a` (source `79ba4ea`):
14 distinct roots for Draft06B's 28-layer, hidden-1024, Q16/KV8 geometry,
scalar/MFMA projections and FP32 head, paged append/attention, and remaining
decoder operations. It passes 27 CPU tests and strict Clippy. Fresh `216822284`
compiler emission produces image `3a308c9c`, with exact closed-root and replay
checks; native fixture validation is pending. This does not modify target
kernel geometry. Resident paged draft generation, its completion-bound driver
join, multi-round numerical validation, serving integration and speculative
load measurements remain required.

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
These are synthetic/source-inspection gates only. User launch confirmation,
the independent target 128/128 reference, final Ferric executable/Setup bindings
and native framework API/observer validation remain outstanding. No baseline
server has been started by this preparation.

## Published Checkpoint

The separate Pages-only branch was freshly rebased and published at
`4a3cbd4efe1df08ccdf7e3e59e19036f9f14469d`. Workflow `34651357690` succeeded.
All seven live static assets match the validated files byte for byte. This
checkpoint adds wave-attention, wait-policy, full-allocation KV and native
ordered-submission observations without changing frozen historical performance
data or claiming a baseline win. It predates the model ordered-submission
regression above. Its private build stage and completed worktree were removed
after archive verification. Ferric implementation commits remain local;
fe2o3 main was independently rechecked at `f85bb375e`.
