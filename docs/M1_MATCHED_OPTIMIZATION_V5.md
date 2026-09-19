# Matched Optimization V5

Started September 18, 2026 at user request: establish a fresh matched vLLM
comparison, identify bottlenecks, and optimize Ferric. No win is assumed.

## Measurement Contract

The first cell retains the previous independent reference and common SSE client:
Qwen3-8B BF16 decoder with explicit FP32 output head, TP1 on physical mi350 GPU0,
context8192, concurrency one, 128 input/128 output tokens, greedy fixed-length
generation, prefix caching and speculation off. Ten warmups precede thirty
recorded requests. Exact output IDs and decoded bytes gate measurement.
This is an explicitly configured FP32-head comparison, not a stock-default claim.
Startup and diagnostics are excluded; both engines use the same HTTP boundary.

The retained vLLM 0.28.0 image is still cached. Image ID:
`c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba`.
Digest: `vllm/vllm-openai-rocm@sha256:e0a3b2bd3fe7ec563916c3a5d949898d133458c18d6b2f460c906885cfb32032`.
Both fresh HTTP cohorts passed their output, input-preservation and clean
shutdown checks. An independent remote replay accepted the pair, after all
eight replay-validator tests passed:

| Engine | Mean TTFT (ms) | Mean TPOT (ms) | Measured-span output tokens/s |
| --- | ---: | ---: | ---: |
| Ferric V17 combined | 2555.823 | 114.619 | 7.479 |
| vLLM 0.28.0 | 19.909 | 4.232 | 229.358 |

Ferric's mean TPOT is27.084x vLLM's: this is a measured gap, not a win.
Both engines completed30 measured requests,10 warmups and2 untimed diagnostics.
Ferric token IDs and decoded output matched on all42; vLLM timed output/usage
matched, with exact IDs checked in its two untimed diagnostics. TTFT is
client-send to first nonempty text; TPOT uses first-to-last text divided by127,
not true per-token interarrival. Output rate is3840tokens divided by the entire
measured cohort span, including inter-request gaps. Cohort order was vLLM then
Ferric, with one process start each. Matched plan SHA256:
`b25fa9824afeac3b5531aa4748b708abfd8edf87e839a7e862d68f1d41f18175`.
Replay summary SHA256:
`11c761a2e4408fc29a822a29f87ed10995c5799e407fc3233068f197836974a9`.

Historical vLLM TPOT4.414ms and V4
Ferric controller TPOT84.546ms belong to different campaigns and are not a new
matched comparison. Initial finite cohorts are descriptive; a faster claim still
requires repeated paired starts, held-out workloads and the existing
[performance policy](PERFORMANCE.md).

## Team Progress

| Team | Work | State |
| --- | --- | --- |
| Serving measurement | Distinct V17 HTTP profile, unchanged SSE client/reference, fresh ownership and pre-reap cleanup | 17 harness and8 replay tests passed; completed pair independently accepted; both engines cleaned up |
| Runtime | Latest compiler adoption and separate bounded host diagnostic | 81-file migration, 48 source-gate tests, core/adapter/CLI tests and Clippy passed; latest controller/worker built; native counter diagnostic accepted |
| Kernels | Inspect actual attention, KV append and Q/K normalization geometry | V18 decode hypothesis rejected; V19 emitted at frozen 5a503c04, CPU checks and 68 isolated GPU cases pass; full-model parity passes but initial A/B shows no gain |
| Integration | Remote resource admission, matched launches, evidence and candidate acceptance | Host/runtime diagnostics, matched HTTP pair and V19 native A/B accepted; operation diagnostic passes 19 CPU tests and all 12 native cells |

R2 source-bound batch replay finds 39,046.9ms inside 405 measured batches and
only 8.1ms between them. Decode batch latency averages84.529ms. This points to
batch execution, not inter-batch JSON scheduling; it does not separate GPU work
from runtime waits. A separate `--host-timing` run exposes overlapping host
scopes, not GPU durations, and never replaces uninstrumented timings.

The bounded diagnostic passed one warmup plus one measured request, all256
output IDs/decoded bytes, 270 batches and166,278 dispatches. Its measured
controller TTFT was2282.764ms and TPOT85.249ms. These instrumented figures are
not HTTP results or a vendor comparison. The completed sidecar is SHA256
`e5d9206333b83087dd9c95809ea3ee25b9c34e4e656b5dbc861df3e9128eaf1a`.
The supervised child exited/reaped cleanly, with no remaining KFD processes.
The first attempt failed before controller launch because relocated artifact
directories were flat; that failure is retained, and the retry used unchanged
images under their expected content-addressed directory layout.

Measured-request decode batches144..270 average85.226ms in the batch scope:
attention ordered groups54.360ms (63.8%), feed-forward ordered groups27.446ms
(32.2%), output head2.235ms (2.6%). Controller attention/feed-forward command
construction totals0.138ms/token. These are distinct top-level wall scopes;
nested IPC/flush counters must not be added again. A separate worker counter
diagnostic now records currentness, preparation, publication and wait costs.
Its 14-test CPU harness first reported13 passes and one fake-only
EOF failure: a Python helper argument collided with the emitted request name.
The fake is fixed and failure assertions strengthened; all14 tests pass in the
fresh remote retry, with clean owned-process teardown. Source review finds about720
operational-currentness fences across72
ordered groups per decode token, not a measured runtime cost or a reason to
remove checks. No GPU timestamp attribution is available in this run.

The latest-runtime diagnostic subsequently passed one warmup and one diagnostic
request on mi350, checking all 256 output IDs and decoded bytes, 270 batches and
166,278 dispatches. The counter delta spans both requests, including warmup:
24.897s worker command time, 23.000s dispatch waits, 1.845s operational-currentness
checks, 0.061s full-currentness checks, 0.061s preparation and 0.189s publication.
These scopes overlap and are not additive or GPU-only durations. Wait includes
currentness, polling and sleeps. Its 247,600 polls include 227,122 unsuccessful
polls requesting 11.356s of sleeps, which overlap GPU execution. The approximate
1.43% duplicate-fence opportunity is an average-cost estimate, not a measured gain;
that source-only runtime patch remains unactivated.

The successful diagnostic report SHA256 is
`01869cfa796365849ef78d950dfef9e22f217996266abcdb8ea08306575c2642`;
retained archive SHA256 is
`61442076425ab9f15cb185e7e5e1b79995bfb7f94023cf5104efc96a2ac5f692`.
Supervisor and worker cleanup passed, with no remaining KFD processes. An earlier
attempt rejected foreign KFD work before child creation; its evidence is retained
and the foreign process was not signaled.

V19 replaces only C1 raw-bit KV-slot copy with 16 Wave64 workgroups. The existing
multirow path, page ownership, copy-on-write and currentness checks remain in
place. Fresh compiler emission at 5a503c04 produced one 8,568-byte image, six VGPRs,
zero scratch/LDS/spills, verified O2 and exact compiler-output replay. HSACO SHA256:
`951619f87213020aaf0eebf794a37ef523400fac1f23ce9fd90e1b78d4b3e1da`.
Compiler handoff SHA256:
`d5e8dc290096f53617cae4b13cde59d04a2b562112da4cea2a132e5022a51765`.
The first emission failed due to an omitted relative build-support directory;
the successful retry included that unchanged source and retained both attempts.
Device tests and strict Clippy pass. Adapter library validation passes 376 tests
with seven ignored and one explicit exclusion: the existing
`resident_roster_pins_all_twenty_rows_before_start` test exceeds the unchanged
4 GiB RSS cap, including when isolated. Both stopped attempts are retained;
there is no full-suite pass claim. V19 CLI tests pass 48 with four ignored,
legacy V17 passes 46 with four ignored, runtime diagnostic passes 49 with four
ignored, and source policy passes 39. Actual-image admission passes. The
isolated-copy harness passes 18 CPU tests and the A/B harness passes eleven.

The isolated GPU run validates 68 cases, including every u16 encoding, page
boundaries, entire cache contents and guards. Its clean accepted report SHA256:
`29785b99cfd4ff4b6398ee51a3559070e62e3b35409a9ec009491e4fe9710030`.
The initial postflight-busy rejection is retained; the retry uses bounded idle
settling without relaxing foreign-work, resource or identity checks.

The same-source full-model A/B passes all eight requests (1,024 output IDs and
decoded bytes), with one warmup and three measured requests per arm:

| Native arm | Mean TTFT (ms) | Mean TPOT (ms) | Measured-span output tokens/s |
| --- | ---: | ---: | ---: |
| Existing C1 copy | 2300.203 | 84.754 | 9.798 |
| Parallel C1 V19 | 2458.523 | 90.230 | 9.197 |

This short fixed-order native comparison does not show a gain. The candidate
has substantial request variation (81.875, 101.385 and 87.431 ms TPOT); do not
attribute the full difference to the kernel or compare these native numbers
directly with the HTTP baseline. Defaults remain unchanged. Both arms use the
same controller, runtime and six images; only the copy selector differs.
The accepted report SHA256 is
`93817dcd9bb5d66ced2f03d639a89e152626097dc979783a82c6d76ea95600b3`.
Supervisor cleanup passes with no remaining owned or foreign KFD process.

## Operation Isolation

The separate unchanged-runtime operation diagnostic passes all twelve native
cells, with full input/output/guard validation before and after measurement and
clean owned-process teardown. Each cell runs 16 warmup packets and 256 measured
packets. Group sizes 1, 4 and 16 preserve `WaitForPrior` ordering; these are not
independent simultaneous kernels. All nineteen counters are captured exactly
twice around the measured interval, which contains no reads, writes or kernel
admissions. Nineteen remote CPU harness tests and two independent source reviews
preceded the native run.

| Operation | Worker wall us/packet, N=1 | N=4 | N=16 | Controller wall us/packet, N=16 |
| --- | ---: | ---: | ---: | ---: |
| Residual, 4096 elements | 158.413 | 39.528 | 16.341 | 29.278 |
| Paged attention, context 192 | 357.194 | 266.162 | 239.570 | 259.370 |
| Q normalization, width 128 | 156.723 | 64.659 | 47.626 | 63.813 |
| Down projection, K=12288 | 257.709 | 189.186 | 167.854 | 180.910 |

Larger groups amortize boundary cost, while attention and down projection remain
much slower than the small residual control. This supports investigating both
kernel efficiency and group-boundary overhead, not identifying polling or
currentness as the sole bottleneck. These are synthetic cache-hot repeated
operations, not GPU timestamps or model inference; never multiply their times
by layer counts into a TPOT claim. Timers and runtime counters overlap.
The real attention scalar at position 191 is 192, not the configured capacity
8192. The fixture uses that actual scalar and retains a 512-page physical pool.

Accepted operation report SHA256:
`421e0964c3dce1eb4c4aa59f764811106590387be05eea4c70acdca5df794907`.
Retained archive SHA256:
`6ea5c8e53c736a78958c31b87ac3dd7f35ff7a615367efd482fca10db6da0aa9`.
The next GEMV prefetch proposal passes eight source/host tests and strict
Clippy remotely at the frozen 5a compiler pin. At the newer c4c5cdd0 pin,
all eight tests also pass, but strict Clippy is terminated when the CPU stage
reaches its unchanged 10 GiB minus 512 MiB reserve. That is not a Clippy pass;
the stopped receipt remains retained, and the earlier 5a pass does not cover
the c4c build. Its initial formatting-sensitive
test and subsequent test-only lint failure are retained and corrected without
changing the kernel arithmetic. It remains outside the active adapter until
latest-compiler emission, ISA inspection and native validation. An attention
parallelization proposal is also being evaluated; neither has a gain claim.

## Request-Ordinal Audit

The same thirty HTTP requests retain both controller and SSE timings.
Controller mean TPOT is 114.622051 ms versus client 114.619229 ms; the largest
absolute per-request difference is 0.004851 ms. Mean client TTFT exceeds the
controller boundary by 2.219368 ms. Thus the cross-campaign difference from
roughly 85 ms native TPOT is not evidence of 30 ms of post-completion SSE delay.
Backpressure or host contention could still slow the controller itself.

Within that one HTTP run, controller mean TPOT is 94.153257 ms for requests
2..9, 112.332222 ms for 10..11, 114.622051 ms for measured requests 12..41,
and 118.348164 ms for the final diagnostic. This is observed drift, not proof
of an ordinal-dependent bug. Prior short native comparisons measured only
requests 2..4. Their placement/instrumentation and process starts also differ.

Read-only source review finds no growing default-path request history,
allocation roster or retired-queue list that explains the drift. Optional
host timing does retain records, but was disabled in these runs. Synchronous
JSONL output remains a possible feedback mechanism. Old HTTP evidence does
not retain individual batch events, so it cannot separate active batch time
from inter-batch gaps. The separate native control now passes thirteen CPU
tests and all fourteen native requests: one pre-diagnostic, ten warmups and
three measured requests (12..14), using the exact frozen HTTP controller,
worker and five images. All 1,792 output IDs and decoded bytes, 1,890 batches
and 1,163,946 dispatches match. Measured mean TTFT is 2313.017469 ms and TPOT
84.534652 ms; finite native ingress rate is 9.808638 output tokens/s.
Ordinals 1..3 have TPOT 104.659, 104.459 and 97.790 ms; ordinals 4..14 stay
near 84..85 ms. Request age alone therefore does not reproduce the HTTP drift
through request 14. This is a different-time diagnostic, not a matched
HTTP/native A/B, a vendor comparison or an optimization gain.

The native report SHA256 is
`a5e23fae8d41fad8f248c6033d5d93af0ef715d2fc9aaebafaac4266571988b1`;
retained archive SHA256 is
`7121a2a60f470ee91aa56b8ecc3047fcd95e4d655770fdc56d71cc8507dba0ab`.
Outer supervision completes without termination signals, and postflight finds
no KFD processes. The inner frozen helper records TERM/KILL calls to its
reserved, already-exited leader group after verifying exit zero and no live
descendants; do not describe every cleanup layer as signal-free.
CPU/RSS observations and raw batch events are retained. Offline replay passes
twelve CPU tests and all actual output/batch checks. Measured mean decode
batch time is 84.528996538 ms/token, with only 0.005656556 ms/token between
batches. Measured controller and worker RSS remain unchanged. These host-wall
spans do not separate GPU duration from waits or currentness checks.
Replay SHA256:
`be5b5b6e10dc3ed1a90f20a315add10f2e9ab34c77807834cabdf1b5170a03c7`.
The historical f562 controller/f68 worker is not relabelled as latest runtime.

V20 passes eight CPU tests, strict Clippy and sixteen native-harness CPU tests
at the c4c dependency pin. Its
first Clippy attempt reached stage A's unchanged storage reserve; the retry
passes in existing stage C without changing any resource limit. V21 passes
twelve CPU tests and strict Clippy after remote formatting. Neither has GPU
performance evidence or an active default route.

V20 emission preparation exposed a genuine upstream trusted-manifest defect:
Cargo discovers ordered_program_api and ordered_region_api test targets that
the pinned fixture omitted. The unmodified vendor tree is retained. A narrow
compiler-only fixture/closure repair and actual-target-roster regression are
prepared on freshly fetched fe2o3 main ca72d44b; remote compiler validation
has built the new backend and extractor successfully. Four focused regression
tests and strict Clippy remain pending. Kernel and runtime sources are
unchanged between c4c and ca72. The split-attention native harness has passed
two independent source reviews; its nineteen CPU tests and GPU validation
remain pending. These preparation steps do not establish a performance gain.

## Deferred Hypotheses

An initial source-only hypothesis confused the configured context8192 with the
actual attention argument. Call-site review corrects it: the driver passes the
batch maximum position plus one. V18's per-row active bound therefore removes no
inactive iterations for single-row decode; at most it trims15 in chunk16 prefill.
Its adapter/controller integration is stopped before emission or GPU work, and
the isolated draft is retained separately as deferred work. No gain is claimed.
Serial KV append and width128 Q/K RMSNorm remain profiling candidates, not
established bottlenecks. Existing V14/V17 identities and defaults are preserved.

The V19 campaign is frozen at `5a503c04f5ae107a3b3e951ec970b36c5d6a9a79`.
A subsequent upstream check finds main at
`c4c5cdd0f69f3844386440a5addb4d4c3dce0e4b` (18 non-merge commits later).
KFD/runtime/AQL trees are unchanged; compiler, debugger and proof paths changed.
The next source/emission snapshot will adopt the newer compiler separately.
Its native-worker build identity must also be refreshed because the changed
CMake file participates in the identity preimage, even though worker code is
unchanged. Historical binaries and images retain their original provenance.
During preparation, upstream advances to `70b3fe00`, a four-file JavaScript
navigation-capture update with no compiler/runtime source changes. The active
producer build remains explicitly frozen at c4c, not relabelled as the new tip.

The matched-result site is published at static-only Pages commit `72b76748`.
Remote QA passes every width 320..1440, eight named viewports and 64 screenshots;
deployment `35411581137` succeeds without a GitHub build. The published snapshot
predates V19 native completion. Raw evidence, private implementation and this
tracker are not published.

All builds/CPU tests stay on mi300x-2 and GPU runs on mi350, serialized by the
integration lead. A fresh GPU stage is `/tmp/ferric-opt-v5.Q8EAzf7k`; CPU work
uses a fresh `opt-v5` namespace in the retained, locked V4 stage, preserving old
receipts and resource limits. No local/GitHub build, shared-job interference,
private implementation push or cleanup of unrelated data is permitted.
