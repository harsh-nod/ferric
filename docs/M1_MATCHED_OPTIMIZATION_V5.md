# Matched Optimization V5

Started September 18, 2026 at user request: establish a fresh matched vLLM
comparison, identify bottlenecks, and optimize Ferric. No win is assumed.

## Latest Matched Pair

The V22 packed-versus-vLLM V2 HTTP pair completes on mi350 with correctness,
sampled ownership, input-preservation and clean shutdown checks accepted.
Paired replay accepts the complete raw cohorts. The monitor has 22 passing
remote CPU tests. Both engines use the measurement contract below, one start
each, vLLM first, with the same 500 ms ownership sampling policy.

| Engine | Mean TTFT (ms) | Mean TPOT (ms) | Measured-span output tokens/s |
| --- | ---: | ---: | ---: |
| Ferric V22 packed | 2329.922 | 83.598 | 9.885898 |
| vLLM 0.28.0 | 18.636 | 4.252 | 228.904359 |

Ferric remains 19.660x higher in TPOT and 125.021x higher in TTFT. Thirty
measured requests succeed per engine. This is not a win, stock BF16-head
comparison, stable-tail estimate, TP8 result or speculative-decoding result.
The separate old V17 pair below is retained as history, not combined with the
new baseline or used to attribute a cross-campaign gain to one optimization.
Summary SHA256:
`0283449b779015bffe0ba72954069ffcca7b809ae6aadf91aa24d6b3fb9d1c07`.
Plan SHA256:
`71855e76d38c3d3ff11d21b7ff33d7c67303a5fc28dff07c8deebb0ee14cf7fd`.

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
twelve CPU tests and strict Clippy after remote formatting. These initial CPU
results preceded the V20 GPU results below; neither changes the default route.

V20 emission preparation exposed a genuine upstream trusted-manifest defect:
Cargo discovers ordered_program_api and ordered_region_api test targets that
the pinned fixture omitted. The unmodified vendor tree is retained. A narrow
compiler-only fixture/closure repair and actual-target-roster regression are
prepared on freshly fetched fe2o3 main ca72d44b. The rebuilt backend/extractor
and all four exact regression tests pass remotely. Strict Clippy fails on two
unchanged dependency diagnostics, before reaching the backend/new-test target;
fresh structured baseline/patch runs reproduce the same two errors. This is
not a strict lint pass or backend lint qualification. An explicit limited
engineering receipt permits isolated experimental emission only. The core fix
was rebased onto upstream 0b0e6c0d and pushed to main as `7dfbe5cc` with CI
skipped; the four repaired files and compiler source closure match the tested
snapshot. The clean temporary core worktree was removed after bundle retention.
V20 now emits. Its first ISA review rejected all fused FP32 instructions;
independent review locates exactly two per root in integer launch-geometry
division, before tensor loads, not in the dot product. The corrected R2 gate
binds the exact artifact, setup/loop instruction bytes, separate mul/add pairs
and finite checks. All eight checker tests and the actual R2 inspection pass;
the original R1 failure remains retained. V21's first emission rejects a NaN
fallback constant. A four-sentinel infinity correction retains fail-closed
finite checks and proceeds through the fresh validation attempts below.
The first corrected-source test attempt stops at the unchanged stage reserve
with status 125; its abnormal-cleanup uncertainty is retained. After retaining
compiler source, results and emission evidence locally, the integration lead
removes only the unused nightly compiler debug cache under the build lock,
reclaiming 1,964,836 KiB. Copied compiler tools pass their hash checks before and
after cleanup. All thirteen corrected-source tests and strict candidate Clippy
then pass under unchanged limits. The new emission completes extraction to a
94,407-byte intermediate handoff, but fails before producing an HSACO because
the LLVM worker cannot open its gfx950 device-library directory. That failure
is retained. Read-only diagnosis identifies a removed private `/tmp` dependency
path in the historical worker, not a missing shared ROCm installation. All nine
installed candidate library files match the expected hashes (489,572 bytes
total). The exact files are restored into a newly owned private directory;
an accounting wrapper includes that external tree under the unchanged stage
cap. No shared library, worker executable or compiler/protected-runtime check
is changed. Fresh emission succeeds with HSACO SHA256
`45c91170c4e83178f3f3f849daffb0b0e1e302ec2c3d4f1178df1a48a9ccc069`.
Actual ABI/resource inspection and independent manual ISA review pass their
engineering checks: both roots are Wave64 with zero LDS/private/spills, and
weighted recurrence remains separate multiply/add. Conversion, exponential
and division lowering account for the fused instructions. This is not native
or formal qualification. Manual report SHA256:
`8612ee0aa3d977f6cc8e81d46b322d95630f5a833192b1de4eedbd2a2a68abbf`.
The split-attention native harness passes nineteen CPU tests
after a test-only exception-type correction; its initial failure is retained.
V21 GPU numerical and performance validation remain pending. Fresh stage
`/tmp/ferric-opt-v5-attention.YWkhYzAB` passes staging, but GPU admission stops
with status125 before spawning a worker: GPU busy and foreign KFD PID2762504.
No process is signaled. This failed preflight is retained, not a kernel failure.
The planned analytic campaign checks fifteen correctness cells before six
whole-chain timing cells. Nonzero varied-score validation and exact full-model
greedy output remain separate required gates. Once the foreign process exits,
a fresh retry in `/tmp/ferric-opt-v5-attention.DL2FXZIM` passes all twenty-one
cells with clean teardown and no remaining KFD process. At logical group sizes
1/4/8, V14 versus the complete V21 chain has worker host-wall microseconds per
attention of 358.047/157.703, 265.601/88.087 and 244.529/82.124. Controller
host-wall at N8 is 274.333 versus113.760 us. Both denominators are256 logical
operations; V21 uses512 measured packets versus V14's256. These zero-score
analytic fixtures and cache-hot timings do not establish general softmax
correctness, Qwen parity, model latency or a vendor gain. Report SHA256:
`a1ea1feeed3139ffe35632f30ca854d99e06af596b83dcd84eba94f778cfbfbf`;
retained archive SHA256:
`41e74ddb1592a18f50594f96d5cca814e56011751492cffde6d2e8ec9182417e`.
Both the completed stage and failed preflight stage are removed after verified
local retention, no-live-KFD checks and exclusive stage locks, reclaiming
7,208 KiB and2,904 KiB. Shared model and baseline inputs are preserved.

The separate varied-score oracle passes all20 CPU tests and its launcher passes
all14 CPU tests. A first native attempt completes12 of21 cells, then fails closed
while the thirteenth worker exits: `/proc/2901660/exe` disappears before its KFD
sysfs entry. That worker's complete numerical checks pass, but the supervisor
interrupts its controller and the campaign remains failed125. All owned children
are reaped and postflight finds no KFD process. A bounded coherent-observation
retry was prepared without ignoring persistent unknown or foreign entries.
The separate R2 supervisor passes32 CPU tests, then all21 native varied-score
cells and63 packets complete with normal exits, clean teardown and no remaining
KFD process. The bounded retry journal is empty: this successful native attempt
did not naturally exercise the exit race; fake CPU tests cover that mechanism.
The same independent Decimal oracle and predeclared numerical limits are used.
Three cells have five total differing BF16 elements between V14 and V21, all
within those limits. OCML accuracy bounds remain stated assumptions, not proved
facts. No timing, bitwise-equivalence or full-model claim follows. Report SHA256:
`ff4329f040dbfc4e62599f20ef6cfe3fe2e211633df383c4e553807e4692d469`;
retained archive SHA256:
`f0ec4b4ea6bbe11554ec7fe9ad2327a4951a95570a38021de07951e605c437fa`.
After local archive verification, both varied-score GPU stages are removed,
reclaiming138,024 KiB and86,492 KiB. The failed R1 remains retained as failed.

The distinct V25 full-model integration passes two independent source reviews.
Both arms admit the same six images and allocate the same133,120-byte workspace.
Only single-row contexts128..256 select partial/merge; all other batches retain
V14. Its independent batch-aware counter expects87,711 packets per fixed
128/128 request in the candidate versus83,139 in the control. Its native harness
passes18 CPU tests in50.793s. Parser12 and preparation pass. The first format
check fails because rustfmt needs a second pass for one return expression;
the failed receipt and first-pass inventory/archive remain unchanged. A narrow
successor admits only that exact whitespace change, passes three helper tests
and two consecutive formatting checks, and reuses the actual parser/prepare
receipts. Device contract5 and host8 tests pass. Actual adapter compilation then
fails101 with252 duplicate `fe2o3_device` diagnostic items: the5a and c4c SDKs
cannot coexist in one Rust link unit, even when only names cross their APIs.
The failed focused log and clean guard remain retained. A narrow successor moves
new-kernel metadata generation into an isolated build-script executable. Its
first preparation rejects Cargo's changed dependency ordering; the raw before
and after metadata are retained. An exact-edge ordering retry passes12 helper
tests, actual metadata replay and two formatting checks. Focused adapter
compilation stops at the unchanged stage reserve before tests: status125,
return-15, known process group reaped; the guard leaves detached cleanup
unqualified. Subsequent read-only checks find all known PIDs and their group
absent, without relabeling that failed receipt. No successful compile is claimed.
An unchanged-source, fresh-log retry after bounded cleanup also stops at the
stage reserve: status125/return-9, peak observed RSS1,115,402,240 bytes. The c4c
kernel library and adapter build-script executable finish; compilation reaches
`ferric-engine` but no adapter tests run. The guard again leaves detached cleanup
unqualified; subsequent read-only checks find its known PIDs and recorded process
group absent. Both failed receipts are preserved. After retaining and removing
the unused nightly installation, focusedretry2 passes all11 Rust tests and six
receipt tests with clean guard0; ABI passes2 with1 ignored actual-image case.
The twelve remaining CPU phases, including explicit actual-image admission,
regressions, strict Clippy and release build, are running separately. The twelve
original native-retention helper tests pass, but no V25 controller build,
actual-image admission or full-model result exists.

## V20 Isolated Native Results

All twenty V5/V20 GPU cells pass full input/output/guard parity against the
independent dyadic oracle, warmup/final validation, exact dispatch counters and
clean teardown. Each cell uses a fresh worker, both images, sixteen warmup
packets and 256 measured packets in ordered groups of sixteen. No KFD process
remains after supervision. Mixed-fixture worker host-wall microseconds per
packet are:

| Root / Shape | V5 | V20 |
| --- | ---: | ---: |
| BF16, N=1024 K=4096 | 48.103 | 48.414 |
| BF16, N=4096 K=4096 | 70.972 | 67.481 |
| BF16, N=12288 K=4096 | 114.551 | 111.027 |
| FP32 partial, N=4096 K=4096 | 79.907 | 66.894 |
| FP32 partial, N=4096 K=12288 | 167.912 | 114.082 |

The separate last-element fixture shows the same broad pattern. The larger
partial GEMV result motivates an opt-in full-model comparison, not a model or
vendor gain claim. These are cache-hot host-wall observations with different
historical/new image producers, not GPU durations or source-only attribution.
Actual ISA does not achieve four-pair overlapping loads: BF16 waits after each
load, while partial GEMV allows an input/weight pair before waiting. Register
counts are 22 VGPR/38 SGPR and 24/48, with zero scratch, LDS or spills.
Accepted report SHA256:
`d8d2202d0595a49fd025d6783f6d7da5f7b25187cd38c29db12882a412f5dba9`.
Retained archive SHA256:
`66d79e205cd5a0a600f77713d1b0110e7a42db24d84d4247a94854d7abd7ff7d`.
The first archive attempt reports a directory-mtime warning because its output
was created inside the archived root; the accepted retry names inputs explicitly.
The completed GPU stage was removed after verified local retention, reclaiming
7,404 KiB; the shared model and historical baseline remain untouched.
The separate V23 full-model integration has passed independent source review
after one test-only type correction. Both arms retain six identical images,
baseline packet grouping and the FP32 head; only seven C1 layer projection roots
change. CPU and actual-image admission tests, native output parity and timings
are still required. Its first CPU preparation fails in the evidence parser:
Cargo lock dependency edges omit a commit fragment present on package sources.
The failure is retained. A separate R2 parser preserves exact package pins,
rejects ambiguous/wrong-source edges, and passes all nine remote parser tests.
R2 remains a parser-only result; a separate R3 source recipe will retain the
later five-file V22 test corrections without rewriting original evidence.

The separate V24 direct-index Q GEMV passes nine CPU tests and strict candidate
Clippy, but emission fails before HSACO generation with
`UnprovenBarrierConvergence` at the reduction. The admitted index bounds are
mathematically valid; lane-dependent trap and safe-index paths precede the
collective, and the compiler cannot prove uniform convergence. No retained IR
identifies the exact predecessor. This experiment stops with no gate relaxation,
GPU launch or load-overlap claim; its clean failed emission is retained.

## Completion Packing Candidate

Opt-in V22 keeps all 616 packets of an eligible single-row, single-output
forward in the same order and groups them into 39 completions instead of 76.
Pending hidden/scratch buffers and collective state commit only after a
successful completion; failures poison the execution path. Existing multirow
and headless routes, defaults, kernel images and runtime are unchanged. The
source includes flattened command/buffer equivalence and failure tests for all
39 boundaries. All six focused tests pass remotely, and the separate native A/B
harness passes twelve CPU tests. Independent source review finds no concrete
blocker. Remote CLI tests pass 49/4 ignored, legacy V17 47/4 ignored and
diagnostic 50/4 ignored. Batched execution passes 121 tests (including the six
focused tests), execution passes34, and peer regressions pass. Source-policy
passes39 and selected strict Clippy passes. Broader test-target Clippy fails on
eleven test-only lints: five missing semicolons, five excessive-precision
literals and one floating-point array comparison. Five test files now use
semicolons and exact f32 bit identities without changing production code or
numerical values. The fresh R2 recipe preserves the failed R1 receipt and
explicitly checks this limited delta. Its eleven helper tests, six focused,
121 batching,34 execution and four residual tests pass; broad strict test-target
Clippy, release build and retention also pass. Actual controller SHA256:
`faf1aaa8c5874af18e2d25b6710e664f0ac77802acd4405cbb488bc7b5cc7bc5`.
The locally verified CPU archive is
`2a433bff97e60c69a16cc4190f8242f30098c55976e41fb93ad4cb52deb615fd`.

The first same-source native A/B now passes all1,024 output IDs and decoded
bytes across eight requests,1,080 batches and665,112 dispatches. Both arms use
the same actual controller, historical f68 runtime, five images, model and
commands; only the packet-packing selector differs. One warmup and three measured
requests run per arm, baseline first. Clean outer supervision and postflight
find no remaining owned or foreign KFD process.

| Native Arm | Mean TTFT (ms) | Mean TPOT (ms) | Measured-span Output Tokens/s |
| --- | ---: | ---: | ---: |
| Baseline | 2600.412 | 111.871 | 7.614984 |
| Packed16 V22 | 2524.218 | 98.857 | 8.488262 |

This cohort observes11.63% lower TPOT and11.47% higher output rate, not an HTTP
or vendor win. The three candidate TPOT samples span89.237..109.135ms; repeated
starts and a fresh matched HTTP pair remain necessary. Report SHA256:
`634c1634aac5a1a01fd7e436fe155a8d3300e14e5dca76bb0920dd0af54aa697`.
Complete native archive SHA256:
`a16078f0541f06857cad27a6e549ed83759535ba71db1b967b61cced223e7287`.

The separate candidate HTTP successor passes all31 remote CPU tests in13.641s,
with clean owned-process teardown and all thirty input hashes unchanged before
and after execution. It requires successful retained V22 or V23 native output
parity before admitting a new matched HTTP pair; synthetic test fixtures do not
satisfy that prerequisite. Both engines must run under the same new plan, so the
historical vLLM cohort cannot be relabeled as its comparator. Input-manifest
SHA256: `3e9e7c5ce78ca8a7cce61002521077529567684e12c9bc3e9520121a3fd4bec7`.
Actual native replay and preparation now produce fresh matched plan SHA256
`e21c932e2ca6514e39dda9e14456d85c3fe22ef26159854867365ecd13b0512d`.
The first vLLM attempt stops before container creation: its GPU preflight sees
an unrelated TP2 worker on the shared host. A fresh second attempt subsequently
passes its complete30-request cohort, diagnostics and clean teardown, recording
mean19.684309ms TTFT and4.350754ms TPOT. Ferric R1 completes both diagnostics,
all40 timed token checks and clean owned teardown, but an unrelated TP2 worker
is present on GPUs0/1 at final postflight. Its final timing admission is false;
it is not paired with vLLM or promoted by its earlier successful token checks.
Both failed attempts remain failed and no unrelated process is stopped.
Ferric R2 launches only after a fresh all-eight-GPU idle observation, but a
foreign TP2 worker subsequently overlaps it on GPUs0/1. Root sends SIGINT only
to the exact PID/start/argv-checked owned supervisor through pidfd. The controller
closes normally; its owner terminates the SSE client with SIGTERM. The complete
interruption/forced-client-cleanup/postflight exception chain remains rejected.
All four known owned PIDs are absent afterward; no foreign process is signaled.
No further GPU launch is attempted pending an uninterrupted idle window.
The frozen harness checks idle pre/postflight, not continuous exclusivity;
even a later accepted finite cohort must retain that limitation.
All four attempts and257 source/evidence files are retained locally with verified
archive SHA256 `63e6865642ca0caa73d9deb9893cb4a52d26286b5f10b0527b8d53d0fe452af8`.
Closed result-file permissions were tightened from664 to644 before retention;
contents were not rewritten. There is no new accepted pair or recomputed win.
The remotely validated V22 implementation is checkpointed privately at
`60153b823ca53af0a5c2d5b6939e5122b22dda09`; it is not pushed.

A separate ordered64 core-runtime proposal is based on latest observed fe2o3
main7dfbe5cc. Its new wire operation preserves the old sixteen-packet API and
all per-packet ordering, completion, ownership and failure checks. Remote tests
pass11 focused,24 ordered and505 full-library tests, with one existing actual
image test ignored. Strict library/test Clippy and the release worker build
also pass. Worker SHA256:
`9649a9ad9218bbe3ba7fe88fb6b37856a271ae3b28061af77495a48ba1377868`.
CPU evidence archive SHA256:
`6919d9cbbf6581a9e989473338b8d27c8c83d247d55c2b677920d555a3719de9`.
Its isolated native correctness run now passes251 dependent packets in groups
1/17/40/64/64, idle rollover, then1/64. All251 outputs and594 full-buffer guard
checks pass, the worker exits normally, and postflight finds no KFD process.
Report SHA256:
`77e225baf226ef220234834207f6be8568b941ee35410da092bc7f014d5d0b33`.
Complete native archive SHA256:
`b3e4a995eddf47d841b3e4f10ab694e62d2d631c0880c8d297329f926ee46a7d`.
The native harness separately passes18 CPU tests. Actual Ferric serialized-header
checks pass source preparation/formatting and export all616 recording commands.
The next test fails compilation on digest hexadecimal formatting and a missing
`open_batch32` boolean argument. A reviewed two-file test-only repair passes
seven helper tests, preparation, formatting, actual packing and core decoding.
All 616 commands fit ten groups per case; both the actual-width case and a
synthetic maximal-integer-width clone decode all 20 headers. Maximum header
sizes are 28,811 and 43,983 bytes, below the unchanged 65,536-byte cap. Synthetic
allocation capacities do not establish native validity. Strict bridge Clippy
then fails cleanly on two redundant method closures in the Ferric test module;
the core Clippy command is not reached. This failure remains retained.
A separate R3 changes only those two closures to method references. Its five
helper tests, preparation, formatting, actual packing, core decoding and both
strict Clippy commands pass under unchanged limits. Its wire, packing and core
report bytes exactly match R2. This validates the header bridge, not a new
model route or a runtime performance gain.

The independently validated seven-file core ordered64 overlay is published to
fe2o3 main as `d10f49bfedc26848285d20ec1399c193b3340f47`, after fetching and
rebasing onto unchanged latest main7dfbe5cc. Every file matches the complete
remote formatted inventory used by its 505-test library suite, strict Clippy
and successful native chain. The commit has `[skip ci]`; no matching GitHub
workflow run is observed. No Ferric kernel, model route, header fixture or
dependency change is included. Its temporary clean worktree is removed.
This core change is not used by any accepted model benchmark.

## Fused Kernel CPU Checkpoints

V29 combines the C1 gate and up projections with SwiGLU, preserving each
projection's BF16 narrowing before the activation. It shares activation loads
but does not reduce weight bytes. Nine Rust tests, five independent fixture/
oracle tests and strict all-target Clippy pass on mi300x-2. A test-only loop
lint fails in R1; the exact one-file R2 repair and all original failure receipts
are retained. The dense dyadic fixtures establish their bounded arithmetic
cases, not a general OCML exponential error guarantee or full-model parity.

V30 combines Q/K RMSNorm and RoPE with one wave per Q head. It preserves the
normalization, weight multiplication and rotation BF16 boundaries. Five
contract tests, eight host-model tests and strict all-target Clippy pass on
mi300x-2. Parallel norm reduction changes summation association explicitly;
the host butterfly model does not establish the actual emitted reduction tree
or exact equivalence to serial normalization. Both candidates would replace
three C1 packets with one per layer, but neither is emitted, natively validated,
integrated into the active adapter or measured. No speedup is claimed.

The retained fourteen-phase CPU continuation, including the R3 header bridge,
V29 R2 and V30 R1 source/receipts, has archive SHA256
`90b2a2993e76a34da9d0c8bbfb0eb4fbb959accb07b8706af2346b43689dab18`.
All fourteen actual guards report status0, normal completion and clean owned
cleanup. Builds and tests remain remote; no resource limit is relaxed.

The latest upstream check still finds fe2o3 main at d10f49bf. Its device SDK,
macros and host/HSACO interfaces are unchanged from the c4c snapshot used for
these CPU checks. A revision-only successor imports a pinned local Git bundle
and passes V26 preparation: the regenerated lock differs only by the revision,
and metadata resolves the exact d10 SDK. Its test build then reaches the private
stage reserve at 10,193,813,504 bytes and stops with status125/return-15. This is
not a test pass or an ordinary test failure. The guard records TERM, no KILL,
and unqualified detached cleanup. A later read-only check finds all four known
PIDs and the recorded group absent; that observation does not relabel the
failure. No subsequent d10 phase is admitted. The 10 GiB cap and 512 MiB reserve
remain unchanged; earlier c4c passes and binaries retain their identities.
New GPU emission still requires restoring the complete pinned nightly toolchain
and additional stage headroom. The existing
fixed compiler implementation and ordered64 worker match the corresponding
published source, but their original build provenance remains explicit.

## Bottleneck Priorities

Retained decode host spans put54.360ms in attention-collective flushes and
27.446ms in feed-forward flushes out of85.226ms per batch. These scopes include
GPU execution and waits, not isolated kernel durations. A distinct steady trace
records84.529ms inside batches and only0.005657ms between them; HTTP delivery or
between-batch scheduling is not supported as the primary explanation.

Source and retained ISA review find that the V5 Wave GEMV accesses are already
coalesced, with adjacent lanes reading adjacent BF16 elements. The stronger
instruction-level hypothesis is serialized checked loads: each operand load is
followed immediately by `s_waitcnt vmcnt(0)`, including all eight loads in V20's
four-pair group. The intended prefetch overlap is therefore absent in that
emission. The V5 partial root also executes 192 steps at K4096, with 128 masked
inactive steps. Neither contribution is independently timed. Direct-KFD GPU
dispatch timestamps and memory/issue-stall counters remain the discriminating
measurement; overlapping host counters or requested sleeps cannot manufacture
GPU duration or achieved HBM bandwidth.

Cache-hot isolated GEMV and attention results motivate V23/V25 integration but
cannot be added into an end-to-end prediction. Actual V20 ISA does not achieve
the requested four-pair prefetch overlap. V24's direct-indexed alternative is
still rejected for convergence, and the exact rejected IR was not retained.
A default-off, bounded diagnostic proposal against main7dfbe5cc is independently
reviewed; it changes error context only and still needs compilation and actual
failure capture before any compiler proof change is justified. V26's paired
down-projection outputs share one activation load across two independent weight
loads. Its standalone crate now passes five contract tests, six host tests,
formatting and strict all-target Clippy on mi300x-2 with clean bounded guards.
Each output keeps the original V5 arithmetic/check order. Emission, actual ISA
inspection and native validation remain pending; no performance gain is claimed.

After verified local evidence retention, the unused stable candidate debug
cache is also removed under the build lock, reclaiming 1,239,796 KiB. Release
cache, compiler tools and all failed receipts were preserved at that step.
After a separately verified archive and copied-binary identity check, the unused
nightly compiler release cache is also removed under the build lock, reclaiming
849,368 KiB. The stable release cache, retained compiler tools, source and all
failed receipts remain. Product archive SHA256:
`58f110eac88023642ab9a9f42276b0e9aaa8abeedbcceee18ac21a94c3b29a1c`.
A redundant583,818,934-byte toolchain transport archive is subsequently removed
under the same build lock after its exact local copy is verified at SHA256
`9e87a85cd995a51417872ab6a27820a1ecd4fa1e5854972767eb164198cef446`.
Installed toolchains and all build/source/failure records remain unchanged.
No guard is relaxed.
Five obsolete test executables and23 obsolete normal executable products
(46 hardlink paths for the latter) are subsequently removed under the build
lock after exact owner, size, hash and link-count checks plus verified local
retention. They reclaim78,839,312 and222,749,520 bytes respectively. Their local
archives are `d3abe4f8eb21c4e412e44159f14da53f112e1a72f40dbb7a156f281528e4856d`
and `49dbf5e1a110045e92c14d682f987ca683e0c6ede7b372102032909e233330fa`.
Redundant remote archive copies are removed only after local verification.
Current V22 controller, ordered64 worker, libraries, tools and all failure
records remain intact; later Cargo tests may rebuild some removed executables.
The inactive nightly installation is then removed separately, reclaiming
1,784,488KiB. All5,603 installed regular-file name/hash pairs match the existing
locally retained transport archive exactly in both directions; the installed
path/type/mode inventory is retained at SHA256
`d06e0bd12f147104daafef2ceea296b68206992b655dc4315b3d4cf2e524da0d`.
Locked deletion rechecks every installed hash and that exact inventory. V25 uses
stage A's separate stable toolchain, which remains untouched. The nightly
installation must be restored and checked before future compiler emission.
A redundant fresh archive attempt times out124 after300s on the shared CPU;
its incomplete local output is discarded, not used as retention evidence.
Stage C is8,188,472KiB immediately after this cleanup; limits remain unchanged.
The unused clean BF16 logical-sampler refinement worktree is also removed,
reclaiming about45MiB locally; branch706718a7 and its history remain available.
Uncommitted changes in the separate dispatch-review worktree are preserved.

## V25 Validation Throughput and V27 Prefill Candidate

The unchanged V25 SDK-isolated retry passes six receipt tests and eleven Rust
tests with a clean guard; ABI passes two tests with its actual-image case
explicitly ignored. Shapes (two tests) and bridge (one test) also pass. Each
of those short phases takes about 315 seconds because nested historical
validators repeat 1,066 source inventory walks. A separate successor shares
hashes and inventories within each boundary, then checks their file identities
and roster; it retains fresh initial and EXIT-final validation boundaries.
The old driver stops before actual-ABI admission when the parent takes its
build lock. There is no actual-ABI child or result directory, and no fabricated
failure receipt. Original passes, source and failures remain unchanged.

The efficient successor passes its 13 helper tests, then the CLI (50 tests),
legacy V17/V22/diagnostic CLI suites (48/50/51), source-policy (40), library
(398, nine ignored and one explicit memory-heavy exclusion) and normal strict
Clippy. Each CLI suite has five ignored cases. Actual-image validation passes
two tests but the third selector runs zero and is correctly rejected; the
correct namespace is `wave_target_v17_runner::live_tests`. Test-target Clippy
then fails on one redundant closure and six missing semicolons, all in two
test modules. Both failures are retained. A successor changes only those two
test modules and passes six repair-helper tests, eleven artifact tests (four
ignored), eleven split-attention tests, all three actual-image ABI tests,
strict test-target Clippy and the release build. Six retention-policy tests
also pass. Unchanged production checks are explicitly reused from R3, not
claimed as fresh R4 full-suite coverage; the heavy-fixture exclusion remains.
The actual controller SHA256 is
`593d807726fc0531284d824db8072d2bb4e1733ee7fae5fba73304c3d5202d90`;
its build receipt is
`5fffef6426bb1721883b2147183ba62220916587b995846a87f26d2c3412a20c`.
Staging on mi350 passes without copying models. A separately admitted native
baseline/split A/B uses plan
`c78bcec2c52a0f51cc8d02f11d7fa2199e56a48478e872ef035c2bf9f6c0cf53`.
The baseline checks all 512 output IDs, but an unrelated TP2 worker arrives on
GPUs0/1 during the candidate arm. The supervisor rejects the pair with status125
and returncode-9; its immediate cleanup receipt is unqualified. A subsequent
read-only check finds all five known owned PIDs absent, with no additional
signals sent and the unrelated job untouched. This later observation does not
convert the rejected run into a pass. No V25 paired parity or timing gain is
claimed. The complete failed-stage archive is retained locally at SHA256
`df7fcb14104a8b3ad058b5ee704b8621ce9105b52187ad97c86396f904768af3`.

A fresh retry on the same physical GPU subsequently passes under plan
`a31552f23efdceb00e95f64a3806c69dfc168d4449ead8e1bf1f49bd28420cea`.
Both arms use the same controller, historical worker, six images and workload,
with one warmup and three measured requests each. Independent retained-raw
review checks all 1,024 output IDs and every decoded byte against the reference,
recomputes latency/rate, and verifies all batch/dispatch counts and input hashes.

| Native Mode | Mean TTFT (ms) | Mean TPOT (ms) | Output tokens/s |
| --- | ---: | ---: | ---: |
| Baseline | 2303.258 | 86.710 | 9.612678 |
| Split8 V25 | 2331.938 | 77.867 | 10.473148 |

The short cohort observes 10.20% lower TPOT and 8.95% higher output rate, with
1.25% worse TTFT. This fixed-order native comparison is not HTTP, a vLLM result,
sustained throughput, statistical confidence or formal qualification. Do not
combine its latency with the separate matched HTTP table. Report SHA256 is
`cc9fc0502b25aedb127eed7e4fa6b1a60a550c5b778e1e66c7ce65cea67e526f`;
the complete retained archive is
`3b967c53395636eacb1e5f47d722815f9f0a87af29a06f6e8ce86a9170b687e9`.
The outer supervisor exits normally with clean cleanup, no signals and empty
post-run KFD ownership. Both arm cleanup records preserve TERM/KILL flags: the
frozen WNOWAIT helper signals each reserved group after observing exit0 and only
its zombie leader, then reaps it. Raw census records confirm that path. Do not
describe the arms as signal-free. The interrupted first attempt stays rejected.

The exact 28-path R4 kernel/adapter overlay is imported privately into Ferric.
The entire tested source inventory matches except the six independently changed
tracker/site files. A tested trailing blank line in adapter Cargo.toml is kept
for byte identity. The new executable requires explicit artifact and mode
selection; existing defaults and V22 hybrid rejection remain unchanged. This
import preserves the actual adapter5a/build-only c4c SDK split, not an unfinished
d10 migration, and is not pushed as private implementation.

An existing-image per-shape Wave-versus-MFMA diagnostic passes all 22 bounded
CPU tests and two independent source reviews, then completes natively with
ten correctness workers followed by thirty N1/4/16 timing workers. It uses one
V5 image, the same signed-dyadic mathematical fixtures in NK/KN layouts, and
full-buffer guard checks. All cells pass, all workers exit normally and GPU
idle postflight passes with no remaining KFD clients. The supervisor reports
status0, clean cleanup and no termination/kill. Native report SHA256:
`c905dfea6b58cfaf424015dbd2c894189280e0f45583f2d3fdd967eb5364cfb5`.

MFMA is slower in every one of the fifteen matched shape/group comparisons.
For ordered groups of16, controller wall time per complete projection is:

| Shape | Wave (us) | MFMA (us) |
| --- | ---: | ---: |
| Q4096 | 79.697 | 187.715 |
| KV1024 | 62.796 | 163.034 |
| Up12288 | 125.262 | 194.009 |
| Output4096 | 93.784 | 187.746 |
| Down12288 | 178.912 | 481.695 |

These are cache-hot synthetic host measurements over256 packets per cell,
not GPU timestamps, model TPOT or a competitive comparison. Fixed root order
alternates by shape; one worker per cell does not establish stable tails.
The older all-MFMA model slowdown remains consistent with this diagnostic;
no selective MFMA route switch is justified by the current evidence.

V27 is a separate Ferric kernel for aligned, exclusive 16-token page copies.
Its 16,384 lanes copy key and value raw u16 words without a collective or
serial row loop. It supports both natural order and the final prompt's
last-logical-row-first permutation. Five contract tests, six host tests and
strict all-target Clippy pass on mi300x-2 under the unchanged resource guard.
Runtime routing and defaults are not changed. The nightly installation must
be restored byte-for-byte before engineering emission; actual ABI/ISA checks,
native V5 parity and timing, and full-model output checks remain pending.
The separate native harness passes all 26 CPU-double tests with a clean guard;
this does not substitute for actual image emission or GPU execution.

The motivating retained prefill spans total 2,282.386812 ms across eight
16-row chunks: attention groups account for 1,889.415254 ms, feedforward for
386.139320 ms and the final head for 2.256607 ms. These are nested host spans,
not GPU timings or measured isolated KV-copy costs. Source review shows that
prefill already uses MFMA projections and suppresses intermediate output heads;
V27 targets a different serial copy path. No V27 performance claim follows.

The new V2 matched HTTP plan is
`71855e76d38c3d3ff11d21b7ff33d7c67303a5fc28dff07c8deebb0ee14cf7fd`.
Its shared ownership monitor passes 22 remote tests. The vLLM arm finishes
cleanly with 192 census samples, a maximum gap of 500.081 ms and no observed
foreign owner. Ferric then completes under the same plan and paired replay
accepts both complete raw cohorts, with results recorded at the top of this
document. The older unmonitored vLLM arm is not substituted.

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

The V19 no-gain result and explicit coverage limits are published at static-only
Pages commit `b8d08cbc`. Remote QA passes every width 320..1440, eight named
viewports and 64 screenshots; deployment `35418497983` succeeds without a GitHub
build. Live HTML SHA256 matches the validated artifact:
`d31be47346f205cbb7f0c218755057d4522724d34270ddea80d68c38d462e138`.
The temporary publishing worktree is removed. The frozen matched V5 result and
historical data files remain unchanged. Raw evidence, private implementation
and this tracker are not published.

The subsequent V22 site update publishes the accepted small native A/B and
ordered64 correctness result at static-only commit
`d27c6ec77eb9c57045bc076f31023f0e65c87b79`. Original negative-fixture and
screenshot-postprocessor failures remain retained. A separate nine-test capture
replay validates the64 existing captures; nine publication-parser tests and
actual static preparation pass on mi300x-2. Deployment35425803909 succeeds with
no GitHub build. All seven live file hashes match the remotely validated
artifact, including HTML SHA256
`67a2a2ab724736dccd4c962cc07ed083556ec34018dd85c67fc923048acc55bf`.
Only the public index and workflow artifact hash/size metadata change; no
private implementation, tracker or raw evidence is pushed. The historical
HTTP comparison remains unchanged and no fresh HTTP win is claimed.

The latest matched HTTP loss is subsequently published at static-only commit
`57db861224c10bd9da899430f1f68a06a55152a3`. Remote site QA passes the responsive
checks and 64 captures. A supplemental capture selector initially times out;
the preserved clean failure is followed by a corrected seven-image desktop/
mobile capture and 15 passing publication tests. Root reviews all seven new
images before preparation. Deployment35430034786 succeeds, and all seven live
file hashes match; HTML SHA256 is
`e7d7fee8cab2822ff6cf4cd9844eb541b28f3c4c3ddc8d800a2fcb12c0378524`.
No private source, raw evidence or GitHub build is published or triggered.

All builds/CPU tests stay on mi300x-2 and GPU runs on mi350, serialized by the
integration lead. A fresh GPU stage is `/tmp/ferric-opt-v5.Q8EAzf7k`; CPU work
uses a fresh `opt-v5` namespace in the retained, locked V4 stage, preserving old
receipts and resource limits. No local/GitHub build, shared-job interference,
private implementation push or cleanup of unrelated data is permitted.
