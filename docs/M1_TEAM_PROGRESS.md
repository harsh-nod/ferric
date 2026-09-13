# M1 Team Progress

Updated: 2026-09-13 UTC. This is an implementation checkpoint, not a qualification
receipt. The 33 M1 roadmap gates remain open.

Current continuation: public fe2o3 main resolves to
`ae26717922b1fb7ad62fdd5ad70814d83eb01177`, with Pliron
`9de42fc6ca7b8f3500ccf2346d69ebbb36e889cd`. The two upstream commits since
61e change auxiliary-queue teardown and a runtime benchmark's error reporting,
not manifests, the upstream lock, compiler code or the worker protocol.
Private integration `94eb4e1` updates 79 files by exact revision substitution
only; reversing that substitution reproduces every prior file byte-for-byte.
Its locked metadata and focused host checks now pass. The actual ae267 KFD
worker passes 484 library tests, 31 doctests and strict Clippy, with one
unchanged hardware-fixture test ignored. Its binary SHA is `526cc6bf`.
Current ae267 CLI/backend tools also build successfully on mi300x. The first
compiler attempt's missing nightly library is retained; the separately scoped
second attempt changes only its loader environment and passes all seven phases.
Existing emitted images retain their actual producer revisions. No fe2o3
source changes or implementation pushes were made in this continuation.

The combined V14 host snapshot `f3b596f` passed 1,025 tests across 23 targets
(56 ignored), the focused V14 checks, image admission and source policies,
then failed strict Clippy on three test-only warnings. Commit `af1b384` fixes
those warnings without production changes. The ae267 continuation at `7c92836`
passes 29 steps including focused tests, strict Clippy, release builds, 74 HTTP
tests and 38 source-policy tests. Its stale verifier manifest/lock hash arrays
are corrected only in tests at `4f21514`; all 31 verifier tests and final source,
dependency and seven-binary checks pass. Remote formatting passes at `0793070`.
The original failures remain retained; the full 1,025-test suite was not rerun
against ae267. Production controller source remains `7c92836`, binary `5b7fd71c`.

All four real Qwen V14 correctness cases now pass on mi350 GPU0: resident wave
and query-hoist V14 at both 8 and 128 outputs, using that controller and ae267
worker. Exact token IDs/UTF8, normal shutdown, retirement/drain and all-eight
idle checks pass. An independent mi300x CPU replay matches all four complete
wrapper objects. The first launch's content-directory layout failure remains
preserved; R2 changes only the artifact path and run-directory tags. These
finite context256 cases do not add timing metrics, HTTP qualification, a
same-compiler ablation, default promotion or an M1 gate closure.

Native result `14a0b50e` and archive `8d4fe904` retain the four cases; independent
replay result `a38f1d6c` and archive `26727569` retain the recomputation. The
completed host stage and three compiler targets were removed (7,377,032 KiB),
as were eight mi350 run/wrapper/upload directories (14,680 KiB) and the CPU
replay stage (3,964 KiB). Models, active compiler inputs and installed binaries
remain intact. V15 emission stopped before compilation because its private
offline cache lacks `smallvec 1.16.0`; exact dependency and build-std cache
preparation is the next step, with no kernel/compiler/lock changes.

## Current Performance Swarm

Combined integration `9c2e98e` includes the paired K4 canary and standalone
parallel argmax, with active dependencies pinned to public fe2o3
`8efd4fd416d1ffae7a718144e4d299fe3c8f7590`. The latest adapter tests,
release builds, source/verifier policies, inventory comparisons and negative/
release policy gates pass, with the final aggregate archived. Opt-in target
argmax route and retirement correction `17dcaa4` are integrated locally from
tested source `d9a2705`; all host and source-policy gates pass. The first
`f662d54` native control was rejected for a canary freshness-predicate error,
with clean teardown and no admitted parity or timing. All six corrected
fixed-oracle runs now pass independent replay. The full128 diagnostic pair
observes TPOT 553.420 -> 528.962 ms and output rate +4.67%; the short8 ABBA
observes +10.16% mean output rate, with two runs per mode. These are native
host-wall descriptions, not HTTP performance or default-promotion evidence.
Frozen GPU binaries retain their actual source identities. Ferric implementation
remains local; separately reviewed Pages content is published independently.
Instrumentation source `809af245` is integrated and passes all 32 host-gate
steps. Its unchanged full128 model diagnostic passes: the GQA math group
accounts for 72.59% of the attention parent host span. This is attribution,
not a measured speedup from instrumentation.
The separate attention/v11 composition is integrated as `dcbbd5b` from tested
source `ed112e0`; all 42 required host-gate steps pass across two reviewed
harness revisions. Eight native finite attention fixtures also pass exact
full-buffer and guard comparisons. All four fixed-reference baseline/wave
8/128-output model checks pass, with clean teardown and all-eight idle checks.
The separate full128 ABBA comparison and independent byte-authenticated replay
pass. With wave-v11 argmax fixed, baseline-to-wave attention observes mean
TTFT 3973.109 -> 3307.672 ms, TPOT 521.458 -> 258.562 ms and mean per-run
output rate 1.823408 -> 3.598124 tokens/s. These are native host-clock
descriptions with two runs per mode, not new HTTP results. Wave TPOT ranges
224.460-292.664 ms; no stable or competitive gain is claimed.

The separate synchronous/ordered composition now passes four exact-reference
8/128-output correctness cases and all four predeclared full128 ABBA samples.
Independent byte-authenticated replay admits a descriptive n=2 comparison:
mean TTFT 3126.512 -> 2848.070 ms, TPOT 230.559 -> 194.927 ms, and mean per-run
output rate 3.949884 -> 4.643287 tokens/s (+17.56%). Ordered TPOT ranges
187.289-202.566 ms. This changes only submission with wave attention and v11
argmax fixed; context256 host timing does not replace context8192 HTTP timing.
All four correctness timings and prior cohorts remain excluded.

The new live entrypoint now also passes two fresh sequential full128 HTTP
requests in each submission mode at context8192. Both modes reproduce exact
reference token IDs and UTF8, reuse slot0 with generations1/2 and no cached
prefix, and close normally with all eight GPUs idle afterward. Independent raw
review agrees. These four correctness requests provide no timing samples.
Both separate matched HTTP cohorts now pass all 42 requests and independent
raw replay. Thirty measured requests per mode observe mean TTFT 3482.380 /
2862.079 ms, TPOT 276.255 / 192.127 ms and throughput 3.318800 / 4.694997
tokens/s for synchronous / ordered submission. Ten warmups and two diagnostics
per mode are excluded. These are sequential, single-start cohorts, not a
stable-gain or isolated per-kernel claim.

| Team | Current Progress | Next Gate |
| --- | --- | --- |
| Measurement | The new same-binary HTTP pair passes independent replay: MFMA layers TTFT 2873.632 ms / TPOT 199.719 ms / 4.532745 tokens/s; C1-wave layers 2827.804 ms / 154.310 ms / 5.707624 tokens/s. Each has 30 measured requests, ten warmups and two diagnostics. C1 observes 25.92% higher rate and 22.74% lower TPOT. Retained vLLM: 19.243 / 4.414 ms and 220.558111 tokens/s, about 38.64x the C1 rate. SGLang r7 retains its exact-output rejection. | Repeated primary workloads and bottleneck attribution remain pending. These are single-start finite cohorts, not stable gains or sustained-load qualification. No admitted SGLang metrics or competitive win. |
| Kernels | V13 `256a6e4` passes host/emission checks at then-current `8efd4fd`, with expected ABI and zero spills/private/LDS/AGPR. V14 passes its eight native finite parity cases (48 complete buffers, 96 guards) and independent protocol replay. Four real Qwen 8/128-output resident/V14 checks now also pass, with exact independent CPU replay. Width4096 Wave64 RMSNorm V15 is integrated at `9c8c1aa`; 20 focused CPU tests, strict Clippy and offline lock checks pass with 61e/9de. Its ae267 emission attempt stopped before compilation on a missing offline dependency. | V13 needs native parity/runtime ordering. V14 needs a same-compiler control before isolated performance attribution. V15 needs exact standalone plus build-std cache closure, emission, finite native comparisons and model qualification; changed FP32 reduction association is explicit. Detailed typed payloads are not retained by standard emission. No default promotion or new speedup is admitted. |
| Runtime Optimization | Diagnostic binary `27c0404e` completed two exact 128-output requests and normal drain. Its original report validator failed on a synchronous-command allowlist omission. A separate corrected CPU replay passes all 17 tests and validates the full raw capture: 166,278 dispatches, 347,568 operational currentness checks and 395,246 completion polls. The original native receipt remains failed; no missing postflight is manufactured. | Currentness consumed 8.772 s of overlapping host time; ordered preparation accounts for 165,240 repeated checks. A bounded preparation optimization needs fault-injection tests and matched measurement. No GPU-duration or speedup inference, no fe2o3 change, and no new performance cohort. |
| Speculation | All eight repeated paged-draft cases pass. Paired K4 has two fresh native passes at frozen `ae355e5` and two separate passes at latest-build `9c2e98e`: each observes real `[4,4]` acceptance, both catch-ups and exact ten-token target prefix. | Broader native rejection coverage and speculative serving integration. No serving or speed qualification. |
| Verification | Standalone v13 integer model `ed2ceb5` passes pinned Verus: 18 verified queries, zero errors and all 16 required proof functions. Both the original typing failure and corrected pass are archived. Integrated proof source SHA `1111950b` is unchanged. | Actual kernel refinement, FP32 behavior, ABI, runtime ordering and native numerics are separate unproven obligations. All 33 M1 gates remain open. |
| Integration and Pages | Pages-only `555d2077` is deployed; the next update covers V15 CPU progress, current compiler/KFD, and four real Qwen V14 correctness cases. Current integration through `0793070` has closed its separately scoped host, verifier and formatting follow-ups. Actual production controller `5b7fd71c` and ae267 worker `526cc6bf` pass all four native cases. Completed host targets, run directories and replay stages are removed after evidence retention. No implementation push. | Remotely validate and publish the next Pages update. Continue V15 emission and fixtures independently. Historical images and installed binaries keep their actual producer revisions; no new performance result follows from correctness checks. |

Latest custody checkpoint: C1 replay archive `e2f30d79` and all 115 files are
accepted; its completed mi300x stage is removed (7,408 KiB). V13 host archive
`88fb1446` and all 276 files are accepted; its completed stage is removed
(912,288 KiB), as is the redundant integrated source worktree (42,520 KiB).
All required source, raw test output and executable artifacts remain retained.
These updates supersede the pending-build/replay notes in the history below.
The C1 live complete host archive `7aec042f` and all 2,354 files are also accepted.
All 93 own-source artifacts were fresh after a closed 22-package clean; external
dependencies were explicitly warm. The redundant integrated live worktree is
removed (42,480 KiB). The complete owned build parent is also removed after
10,838 content checks, exact metadata and all 80 original/new groups were
confirmed absent, reclaiming 4,199,364 KiB; independent SSH absence passed.
The eight completed mi350 C1 case directories and six authenticated archive/ledger
files are also removed after local custody and a fresh normal-close/process check
(10,788 KiB); shared models and installed images/controllers remain untouched.

Static inspection of the actual resident v5 image `98b5fdb1` confirms two query
loads/waits inside the attention token loop. The attention descriptor reports
97 SGPR / 36 VGPR and zero spills; these are not occupancy or elapsed-time data.
Complete four-command CPU archive `224b645f` is accepted and its small stage is
removed (780 KiB). Separate query-hoist source `1843d8f` preserves the existing
token loop, arithmetic and reduction order; only two invariant query reads move.
Remote formatting archive `39ac6317` and all 92 files are accepted, including
the original exclusive-transfer flag failure and its one-token correction.
Only remote formatter output was applied locally; the completed stage is removed
(832 KiB). Its thirteen host-model/contract tests now pass; complete host archive
`9d025c46` and all 276 files are accepted, including five fresh own compiler
records and six immediately retained outputs. Its completed owned host stage is
removed after fresh content/process checks (913,836 KiB); independent SSH absence
passed. The clean integrated source worktree is removed nonforced (42,720 KiB),
with branch, source bundle and all failed/successful evidence retained.
Partial GEMV also retains a masked fixed-K tail, but that is a separate future
candidate. No GPU performance attribution or gain follows from disassembly.

The C1 HTTP synthetic gate archive `51240a2f` and all 29 files are accepted;
all 29 test methods pass with no retry. Its completed CPU stage is removed
(376 KiB). The new `8ae69215` controller and reviewed HTTP harness are installed
in separate owned mi350 paths. Both separate context8192 correctness arms now
pass two exact full128 HTTP requests, slot0 generations1/2, zero cached prefix,
normal drain and all-eight idle checks. MFMA archive `b71f038b` and C1-wave
archive `1af0b1ee` match their independent streams and complete file ledgers.
Both exact matched plans pass independent review and replay their own qualification
receipts. The MFMA arm completes all 42 requests and closes normally; archive
`8e1a8fd6` is accepted. C1 R1 fails at the pre-worker socket probe with zero
requests, preserved in archive `8b61ba88`. The unchanged C1 workload then passes
in a separate R2 directory, archive `260d126f`. Both independent raw replays pass,
stdout hashes `783f0dcc` / `0814bd8f`, with forty HTTP/final associations and
twenty raw files checked per arm. Each arm excludes ten warmups and two diagnostics
from its thirty measured requests. The zero-request failure remains explicitly
bound and excluded, with no dropped timing sample. See the
[same-binary HTTP result](M1_COMPETITIVENESS_SPRINT_V1.md#layer-c1-matched-http-comparison).
The three completed matched case directories, two matched plan directories and
three run-specific authorization files are removed after fresh local/remote
custody and process-absence checks (8,320 KiB); independent absence passes.
All local archives, failures and replays remain retained. Shared models,
installed controllers/images and the two qualification cases are untouched.

V13 emission R2 archive `0601efc2` and all 107 files are accepted. Image
`72104603` has producer/finalizer SGPR counts 35/30 and VGPR counts 18/13;
both measure 52 explicit, hidden start 56 and 312 total kernarg bytes. The
original source-mode extraction failure remains archived, with no compiler
invocation in that attempt. The immutable restored toolchain is retained for
V14 reuse. V14's separate ten synthetic control tests pass; archive `f18848ad`
is accepted and its completed CPU stage is removed (3,596 KiB).

V14 emission R2 archive `cb58d401` and all 126 files are accepted. All nine
phases pass, including eleven revised synthetic control tests in the same stage.
Image `8f21681f` has 116 explicit, hidden start 120 and 376 total kernarg bytes,
95 SGPR / 37 VGPR and zero spills/private/LDS/AGPR/dynamic stack. Separate
static def/use review confirms query loads and conversions stay outside the
recurring token loop. This is not a same-compiler comparison with resident v5,
native numerical parity or a measured gain. The typed handoff is digest-only;
full typed/progress replay remains unavailable. The completed stage is removed
after fresh exact custody and nine-group absence checks (24,552 KiB), with
independent SSH absence. The restored compiler is retained for a control.

The additive C1 runtime diagnostic is formatted at source `6116495`, with
profiling isolated from the timed entrypoint. All 2,268 format archive files
and 1,103 formatted source hashes pass; archive `c85fc647` is accepted. Its
completed remote format stage is removed (134,000 KiB). The source-fresh full
host gate stops at step 28: the source inventory correctly rejects the standalone
`sharded_argmax_v13.rs` as unreachable inside the Cargo proof package. The first
27 steps pass (936 tests / 51 ignored / 22 rows, strict Clippy, eight doctests,
release parity, 74 HTTP regressions and 38 source-gate tests). The last twelve
steps do not run, so there is no full host admission. Correction `c4fb35d` moves
only the Rust proof to `proofs/standalone`, preserving SHA `1111950b`, and explains
the placement in its existing note. No source-gate exception or proof authority
is added. This original failure remains in full archive `43305fcb`; it is not
relabeled as a pass.

Corrected source `c4fb35d` passes the separately recorded R2: two scoped clean
commands and all 40 unchanged checks. The 936/51/22 all-target results, strict
Clippy, eight doctests, 74 HTTP regressions, 38 source-gate tests, all five
inventory comparisons and 31 protected policies pass. External compiled
dependencies are explicitly warm; all 95 first-run own artifact records are
freshly built. Binary `27c0404e` matches its early, release and final copies.
Success archive `b8621f4d` has matching independent streams and all 1,244 files
pass root hash verification. All 42 new and 28 old owned groups close normally.
Integration through `0e879d6` preserves tested Rust and dependency bytes;
only existing documentation and the separate Python fixture differ from the
tested source. Actual runtime counter capture remains pending. A duplicate
local failed-build target is removed after full failure-archive custody,
reclaiming 2,300,296 KiB; its original bytes remain in that archive.
The completed remote diagnostic parent is then removed after fresh exact
10,587-file and 12,216-entry metadata checks, all 70 owned-group absences and
binary/tool parity (4,305,344 KiB). Independent SSH absence passes. The clean
integrated diagnostic worktree is removed nonforced (42,824 KiB); both source
bundles, branch and all executable/failure evidence remain retained.

The V14 finite two-image fixture `fec0311` is integrated as `4dcc92a`. The
mi300x CPU gate passes 15 new methods, ten unchanged fixture methods and the
exact eight-case self-test. Complete archive `12d83421` has two matching
streams; all 39 files pass independent root checks. All three groups close
without signals. This validates the fixture and report checks, not native
parity: its worker/stage/image bindings remain unset. The resident-v5 and V14
images use different compilers, so even a future numerical pass would not be
a same-compiler performance ablation. V13's completed emission R2 stage is
removed after fresh exact checks (24,404 KiB); the restored toolchain stays.
The completed parity CPU stage and clean integrated source worktree are also
removed after fresh source/archive/group checks (300 and 42,836 KiB), with
independent absence checks. The native launch copy is retained separately.

The separately bound V14 native run now passes all eight TP1/context32 cases,
with exact integer BF16 output, complete input/inactive-tail equality, 48 buffer
hashes and 96 guards. Resident and candidate loaded ABIs agree at 17 arguments,
116 explicit and 376 total kernarg bytes. The worker/probe group close normally;
all eight GPUs are idle before and after. Archive `bfb4ea39` and all22 files
pass independent root custody. A separate mi300x replay calls the unchanged
validator and reconstructs all421 protocol rows and guarded payload hashes;
archive `7403deac` and all35 files pass root custody, with unchanged41 metadata
entries. This is finite parity, not Qwen parity, TP2/8, context8192, a general
numerical proof, same-compiler ablation or performance/default admission.
The completed replay stage is removed (388 KiB). Native runout/upload cleanup
first stops before deletion at an apparent-size predicate that included directory
st_sizes; separately reviewed R2 corrects only that expected byte count and
passes. Only the runout and upload are removed (252 KiB); the installed candidate
and all shared inputs/models remain unchanged. Independent SSH absence passes,
and both original failed cleanup and all successful evidence stay local.

Public Pages workflow `34726232156` succeeds on exact source `8e6643f`; all
seven deployed assets match the accepted artifact, with historical serving
metrics unchanged. No Ferric implementation or fe2o3 changes were pushed.
The completed Pages stage and clean worktree are removed after publication and
fresh custody checks, reclaiming 316,387,328 bytes in total. The retained LLVM
worker's actual build remains `216822`; its complete worker source subtree
`613ef51b10cdb00c192b8c6292c06f051f519a6a` is identical at latest fe2o3 `8efd4fd`.
The later Pages-only checkpoint `351cb254` passes remote QA: all 13 checks,
eight named viewports, integer widths 320 through 1440, and 58 screenshots.
Its original dependency-admission failure remains in archive `00844f87`;
corrected QA archive `dbf44311` and all 881 files pass root custody checks.
Workflow `34731342554` succeeds for the exact source; all seven canonical
HTTPS assets match at 2026-09-13T01:54:13Z. Historical `performance.js` remains
`05ad1f50`. Fresh custody/process checks and independent absence precede release
of the new Pages stage (308,792 KiB) and clean worktree (1,120 KiB).
Root fetched fe2o3 again after this gate; public main remains `8efd4fd`.

The matched serving cell uses TP1 on one MI350X, BF16 decoder weights, explicitly
selected FP32 output heads, context 8192, concurrency one, ten excluded warmups,
greedy fixed-length output, and speculation/prefix caching off. It is a
single-start finite cohort, not sustained-load or stock-default qualification.
Ferric is substantially slower than vLLM. The wave canary is a separate
host-timed four-request/eight-output workload, with two runs per mode; it cannot
replace the matched HTTP numbers. See the
[competitiveness sprint](M1_COMPETITIVENESS_SPRINT_V1.md) for exact evidence.

The earlier native argmax comparison keeps TP1, the same v5 MFMA projections,
baseline attention, FP32 v8 head, 128-token prompt, context256, prefix caching
off and ordered submission off. Its full128 pair is n=1 per mode; short8 uses
the predeclared serial/wave/wave/serial order, n=2 per mode. Do not combine it
with the earlier wave-plus-ordered gain or replace the matched serving table.
The new attention ABBA cohort also uses TP1/context256 and a 128-token prompt
with 128 generated tokens, but fixes wave-v11 argmax and changes only attention.
Prefix caching, speculation and ordered submission remain off. Timing starts
before prefill and ends at final token commit, excluding setup, detokenization,
retirement and close. Mean output rate averages each run's 128/duration value;
it is not 128 divided by mean duration. The four prior correctness runs are
excluded. No older gain is multiplied into this result.
Completed correction and prior route worktrees are removed. Hash-verified
closure archives precede removal of 7,259,676 KiB of released mi300x stages;
active team stages, current GPU artifacts, shared models and caches remain.
The completed Pages stage, v12 stages and redundant instrumentation worktree
are also removed after verified archival. The completed combined-route stage
is now removed too, reclaiming 3,620,618,240 allocated bytes remotely and
43,171,840 bytes for its local worktree. A separate ordered-driver candidate
has a separate canary worktree; no deleted stage is silently reused. The
driver-only worktree is removed after integration, reclaiming 43,245,568
allocated bytes. Its explicitly leased mi300x target is retained for the next
source-bound gate and will be reported as warm, not a fresh rebuild.
The completed ABBA replay stage is removed (7,516,160 allocated bytes), as are
the published attention Pages stage (325,460 KiB) and worktree (2,748 KiB).
All required source, failures, receipts and deployment evidence remain archived.
The submission canary is integrated as `0a9d5df`, with implementation bytes
matching tested `7cb6522`. Its completed candidate worktree is removed
(42,260 KiB), and the checker CPU stage is removed (208 KiB). The remote
target was explicitly warm for the completed live-entrypoint gate. Its entire
released stage is now removed (5,000,296 KiB), as is the redundant live worktree
(42,308 KiB). The completed submission replay stage is removed (7,296 KiB), with
its raw inputs, admitted summary and cleanup receipt retained. The published
submission Pages stage (338,332 KiB) and worktree (2,788 KiB) are removed too.
Active kernel stages and source candidates remain tracked; unrelated worktrees,
shared models, images and caches are untouched.
The superseded clean append worktree `2716371` is also removed (42,284 KiB);
its commit, branch, source archives and rejected-emission evidence are retained.
The latest HTTP Pages stage and clean worktree are also removed after verified
publication and archival, reclaiming 351,616 KiB remotely and 2,816 KiB locally.
Both validation attempts, final screenshots and exact public asset bytes remain.
The stopped visible-attention worktree is removed (42,344 KiB). The completed
residual host stage and worktree are removed after verified archival, reclaiming
3,296,632 KiB remotely and 42,364 KiB locally. Their source branches, binaries,
raw test output and failure evidence remain retained.
The guarded append worktree is also removed (42,292 KiB), preserving its failed
emission and source branch. The latest native-correctness Pages stage and clean
worktree are removed after successful deployment and live-byte checks, reclaiming
312,136 KiB remotely and 2,832 KiB locally. The separate residual-result Pages
stage and worktree are now removed too: 303,872 KiB remotely and 1,088 KiB
locally. Workflow `34720093591` deployed exact Pages-only `45b608e`; complete
gate archive `a92c1ec8`, closure archive `1650da4a` and cleanup receipt `23548311`
remain retained. Active C1 work remains separate.

The C1 formatting pass produced `f27f85c`. Its first host compile failed before
tests because a test referenced the private `batched::GEMM` constant; `8b7cb03`
substitutes the exact existing kernel-name literal without a production change.
The full failure archive remains, and its owned stage was removed (1,959,704 KiB).
The next cold all-target command passes all 856 tests, but the enclosing gate
stops on an incorrect 845 expectation. Eight unchanged paired-contract tests
and three unchanged worker-diagnostic tests were omitted from the forecast.
The separately reviewed count-only continuation authenticated the original
source, artifacts and successful raw output, then passed 15 more declared
steps before strict Clippy rejected two test-only missing semicolons. Source
`c2a235e` adds only those semicolons. All failed attempts remain archived;
the count-stop/continuation stage was removed after verified custody,
reclaiming 2,894,220 KiB. No failed attempt is represented as a full gate pass.

The new `c2a235e` stage used an empty target and the unchanged 40-step gate,
with the audited 856/45/20 expectation. All 40 commands and the outer gate pass,
including strict Clippy, eight doctests, 74 CPU HTTP fixtures, 38 source-gate
tests, 31 protected source policies and five unchanged inventory comparisons.
Actual non-test controller `ae5fad8d` (9,018,536 bytes) matches the explicit
release artifact. Complete host archive `41335206` is independently verified.
Separate checker and reducer gates pass 28 and 18 methods, respectively;
archives `ffbd1a4a` and `2fc04d5f` retain their raw evidence. The completed
checker stage is removed (220 KiB); the reducer stage (292 KiB) remains for
later actual replay. The idle host stage is briefly retained for the conditional
live-entrypoint gate, with no reuse authorized yet. A fresh fe2o3 fetch still
resolves to `8efd4fd`.

Root installed only the new pinned controller/wrapper under the shared mi350
lease. Both MFMA and C1-Wave pass the 8-token and 128-token cases with exact
IDs/UTF8, normal close and all-eight-idle pre/postflight. The short cases have
9,219 packets, 15 batches and cursor135; the full cases have 83,139 packets,
135 batches and cursor255. Complete four-case archive `d58812cf` matches two
remote streams and all 52 raw-file hashes and sizes. These correctness timings
are excluded from the separately declared full128 ABBA cohort, now running.
Root integrated the opt-in source through `b7b1eda`; all relevant code bytes
match tested `c2a235e`. The redundant clean source worktree was removed,
reclaiming 42,436 KiB. Defaults, head and multirow projection remain unchanged.

Independent work produced two-stage sharded FP32 argmax source `5cd315f` and
remote-format-only follow-up `1ca7223`. It does not change v11, the unrelated
v12 down-projection candidate, artifact inventories or the C1 route. Source
review found no algorithm defect. Added parsed store-operand binding mutations
and scratch-reuse cases bring the forecast to 15 numerical plus 8 contract
tests; `256a6e4` applies accepted remote-only formatting to those tests. The
first host build is still pending. Separate integer proof `7faa90c` stopped
before verification on E0283; raw JSON reports zero verified queries. Correction
`ed2ceb5` only types two branch literals as `int`; its separate Verus rerun passes
18 queries, zero errors and all 16 required functions. Raw JSON `f1ec34e3` and
complete archive `ce0feb56` retain the standalone integer-model result, not actual
kernel refinement or FP32/runtime admission. The redundant proof worktree is
removed after integration (42,404 KiB). Conditional live entrypoint `e1ac86f`,
formatted as `87f38de`, adds explicit ordered-only layer selection without changing
the old live/parser path; its host gate is still pending. Typed emission, native
kernel correctness and performance remain unproven; the extra dispatch may
offset potential parallelism. No new kernel or M1 qualification is claimed.

A read-only timing audit confirms that the current ordered KFD worker exposes
host elapsed time, not per-dispatch GPU duration. Ferric currently discards the
returned ordered-batch elapsed value. Retaining it in a separate diagnostic is
a possible follow-on without a core runtime change, but would still not provide
GPU timing. No such instrumentation change or new measurement is claimed here.

## September 11 Performance Swarm

Active integration uses public fe2o3 `5110577a6d8c45390dfb353386cde748efd5d76c`,
including the concurrent-rank runtime published at public797. Frozen benchmark binaries
retain their actual build identities. Ferric implementation remains local.

| Team | Current Progress | Next Gate |
| --- | --- | --- |
| Core runtime | Concurrent mixed-rank dispatch is published after independent review, remote tests and genuine TP2/TP8 producer-reader probes. Completed owned core worktree/build stage removed after archival. | Further overhead work must preserve lifecycle/currentness and failure-quarantine checks. |
| Measurement | All six TP2/TP8 host/serial/round profiles pass the fixed reference and full sidecar/identity/close checks. Rounds improve output rate 86.96% at TP2 and 579.69% at TP8 versus serial peers, but remain slower than host. | Completed matrix publication; all current observations are n=1. |
| Kernel numerics | Actual TP1 captures identify the immediate MFMA token flip as a BF16 final-logit tie. The unchanged reference still rejects MFMA BF16. Separate FP32 head/argmax kernels and driver are integrated locally; latest511 emission/replay and host tests pass. | Native FP32/guard checks and matched full-reference model runs; no candidate fix or gain claimed yet. |
| Integration and Pages | Public797 combined host integration passes 266 Rust tests, strict Clippy/release, source gates and 31 verifier policies. Active511 resolves 27 locked configurations; regenerated inventories and compiler commit/tree pair are independently reviewed. Latest KFD worker tests/build pass and its bytes equal public797. | Latest combined FP32 gate, reviewed Pages publication and owned-stage cleanup. |

The [sprint record](M1_PERFORMANCE_SPRINT_V2.md) contains exact evidence hashes,
scope and numerical findings. TP8 rounds currently observe 0.076712 output
tokens/s versus host-staged 0.380674. Collective spans improve modestly, but
other host-observed dispatch and resident-setup spans dominate. These spans
are not GPU timestamps and do not isolate individual system-call costs.
The previous checkpoint below remains historical, including its cleanup and
publication claims; new sprint stages are currently active.

## September 10 Performance Checkpoint

The integrated source tracks the observed public fe2o3 `7528e734` cutoff; frozen benchmark
inputs retain their original source and binary identities. Compiler/KFD
changes are published to fe2o3 main after review/rebase. Ferric implementation
remains local; the Pages performance checkpoint is published separately.

| Team | Current Progress | Next Gate |
| --- | --- | --- |
| Runtime and measurement | Operational mode has two exact-output TP8 runs at roughly 14-15x frozen workload output rate. Isolated/cumulative ablations, repeated MFMA and TP1 pairs, source-matched peers, allocation cohorts and the 16/32-row policy pair are archived. The integrated four-request/replica checkers pass 59 host tests. All five final current-controller cases have strict accepted/rejected dispositions. | Broader workloads and repetitions; no serving-throughput or stable-tail claim yet. |
| Kernels | Full16 TP8 MFMA has two passing repetitions per variant: mean workload rate +28.63%, with startup regression. Adding pruning passes but is slower than MFMA alone. Both wave selectors and current TP1 MFMA-only/cumulative profiles fail the exact seed reference. Full32 native 30 fixtures, peer32 native 19 fixtures and actual 32-row model execution pass. Tiled transpose passes 240 full-array comparisons and saves 15.33% setup in one matched model pair. | Numerical diagnosis of rejected wave/TP1 MFMA profiles; no reference relaxation. |
| Collectives | Public `902fef6e` cached peer sequences pass genuine producer-to-reader TP2/8 probes including rows 17/31/32. Both peer models and source-matched host controls pass; peer workload rates are 82.06%/97.66% lower at TP2/TP8. TP1 residuals pass two runs per variant with +13.85% mean workload rate. Current scalar-projection pruning + residual passes, observing +12.06% workload rate in one cumulative pair. | Profile repeated all-rank fences/serial execution before further peer changes; overlap remains unimplemented. |
| Integration | The integrated `7528e734` source passes 168 library, 43 CLI/worker, 6 control and 22 source-policy tests (two explicit ignores), strict Clippy and release build, plus 70 Python tests on mi300x. Separate repin gates pass 26 metadata configurations, 38 source-gate and 31 verifier-policy tests. Final current-controller TP1 control, scalar pruning/residual and TP8 wide MFMA/pruning cases pass. Pages checkpoint `f4664dc` is live and byte-verified; completed team stages/worktrees are removed after archival. | Broader numerical/performance qualification and protected M1 gates; retain the active local integration and replay evidence. |

The reviewed public delta from `902fef6e` through `7528e734` is compiler/analysis-side, with no
KFD, worker, SDK or lockfile changes. A freshly built `1b262ac3` independent
worker is byte-identical to the frozen `902fef6e` worker; its relevant source
closure is unchanged at the `7528e734` cutoff. Active-pin integration is
separate from frozen runtime comparisons. Old emitted images
retain their actual compiler identities; they are never relabeled as rebuilds.

The allocation cohort observes 1.442/5.734/6.588 output tokens/s for
1xTP8/4xTP2/8xTP1 respectively, at one sample each and a 16-row per-instance
budget. All 64 outputs and 96 physical token rows match. Replication increases
aggregate row capacity and duplicates model storage; this is not an isolated
kernel comparison. The separate same-image 16/32-row policy pair observes a
12.56% workload-rate increase, but request-00 TTFT worsens. The tiled transpose
model pair saves 26.55 s of setup (15.33%); decode variation is not attributed
to the setup-only helper. The [published Pages checkpoint](https://harsh-nod.github.io/ferric/#performance)
retains the `196ae50` measurement data: the initial trio, repeated and cumulative measurements,
all five final current-controller outcomes, and all failed/regressing paths.
Its deployment `34533071403` succeeded. Caption-only follow-up `f4664dc` fixes
the legacy ledger-generator hash label without changing any measurements or
hashes; deployment `34533902613` also succeeded and all seven live assets were
byte-verified. Canonical replica-expectation labels retain their actual meaning.
The site passes 72 negative schema cases, eight named browser viewports and
every width from 320 through 1440 pixels.

The current TP8 wide cumulative case observes 0.444870 output tokens/s and
reuse TTFT/TPOT 2.565240/1.604293 seconds, without a matched gain claim. Its
32-row capacity produced actual rows `[17,6,6,4,1]`; it is not the separate
actual-32-row cohort. Current TP1 MFMA-only and cumulative profiles fail the
same Germany-versus-Spain seed reference and contribute no accepted timings.

The completed root mi300x build stage was removed after source, logs, scripts
and exact controller archival, reclaiming 7,314,760 KiB. No local builds were
run. The completed GPU stage was fully archived, then reduced from 150,492 to
11,612 KiB by removing obsolete owned experiment files. Current binaries,
images, wrappers, all five final case receipts, models and fixed workload
remain. The unpublished integration source is retained. No Ferric GPU jobs
remain and the final all-eight-device snapshot is idle. The measurement and
Pages teams removed all their completed private stages and worktrees after
checksum-verified archival; unrelated user worktrees were untouched.

These are short engineering workloads, not HTTP serving qualification or a
matched vLLM/SGLang comparison. See [the live performance ledger summary](M1_QWEN3_PERFORMANCE.md).

## TP8 Batching Integration

September 10 follow-on work is integrated and GPU-checked on the engineering path. The existing
single-sequence results below remain frozen and are not measurements of these
new features. Public fe2o3 main was rechecked at `3546d54` before this work.

| Team | Owned Slice | Status |
| --- | --- | --- |
| Kernels | Additive multi-row projection, RoPE, paged KV append, causal paged GQA, and output kernels | 32 host/source tests per target pass; independent indexing/numerics review passes. Both nine-root target emissions and eleven genuine gfx950 GPU probes pass. The old thirteen-root image remains unchanged. |
| KV and radix | Bounded physical page pool, immutable complete-page prefix retention, exact radix lookup, cancellation, eviction, and failure quarantine | Integrated. 18 release tests, strict Clippy, eight actual-body mutations, and four external privacy negatives pass. Independently reviewed; completed stage/worktree removed after archival. |
| Scheduling | Continuous request admission, decode/prefill fairness, bounded prefill chunks, generational request IDs, and completion-gated publication | Integrated. 17 release tests, strict Clippy, and eight actual-body mutations pass. Completion preflight is shared with commit validation. |
| Integration | One resident weight set, true multi-row GPU dispatch, atomic all-rank completion, workload CLI, and combined numerical checks | 139 library tests (one prior real-image ignore), 17 CLI/worker tests, 22 source policies, all-targets strict Clippy, and release build pass. TP1 cached and TP8 cached/uncached mixed Qwen workloads each match all eight expected output tokens/bytes. TP8 checks physical prefix reuse, chunked prefill, continuous admission, paged causal attention, and cancellation; independent paired receipt review passes. |

The initial envelope is Qwen3-8B BF16, TP1/2/8, up to 16 token rows per GPU
batch, 16 tokens per physical page, and up to 32 resident requests. The
engineering profile remains explicitly opted in and does not create protected
M1 execution authority or an HTTP serving endpoint.

The TP8 cache-on/off pair uses 34/50 physical token rows and 5/6 GPU batches:
retaining the sixteen-token prefix avoids sixteen rows and one full forward.
This is a work-reduction result, not a timing speedup: whole-run times are
466.414/461.166 seconds in these single unrepeated logical-tick workloads.
All seventeen workers across the three model runs were confirmed absent and
all eight GPUs idle afterward. The three implementation team worktrees and
their completed remote build stages were removed after evidence archival;
the small integration source workspace and runnable model bundles are retained.

The new modules and numerical kernels are explicitly Contracted engineering
code. Host invariants, negative tests, and independent review are not a new
Verus proof of the scheduler, paged allocator, GPU math, or end-to-end execution.
See [the batching runbook](M1_QWEN3_TP_BATCHING.md) for the actual workload
interface, limits, and remaining validation boundaries.

## MI350 And Eight-Rank Work

The initial September 9 MI350 foundation checkpoint below did not yet run
eight-GPU Qwen. The follow-on execution results appear later in this section;
neither checkpoint is M1 qualification. Compiler/runtime changes track
[fe2o3 #274](https://github.com/harsh-nod/fe2o3/issues/274).

| Team | Current Slice | Status |
| --- | --- | --- |
| Core runtime | Explicit gfx950 topology discovery, COV6 loader profile, and a separate checked device-observation token with no execution authority | 420 KFD unit tests, 20 integration tests, and 27 doctests pass, along with scoped strict Clippy. All eight MI350X devices pass repeated no-queue bind/currentness/drop checks without descriptor leaks. All twelve kernels in the genuine gfx950 object pass loader/materialization and both old gfx942 dispatch-path rejection tests. Independent review passes. |
| Compiler integration | Target-bound engineering artifact production and additive AMDGPU compiler-handoff extraction; measured library function targets checked after import selection and before optimization | 518 backend, 17 CLI, 7 finalizer, 16 extractor, and 2 export-simulator tests pass. Four native-worker suites pass, including selected incompatible helper rejection with optional verification disabled. Both target emissions and exact replay pass. Published on fe2o3 main through `a8b016e14`. |
| Ferric kernels | Explicit mutually exclusive gfx942/gfx950 aggregate features over the same twelve kernel bodies; both target helper files bound into the build source closure | 33 aggregate tests per target and 21 historical RMSNorm wrapper tests pass. Updated build/release policy tests pass. Both twelve-kernel emissions pass with the same final native worker; gfx942 bytes are unchanged by the worker fix. |
| Tensor parallelism | Qwen3 1/2/8-rank head/weight partitioning, actual BF16 shard-row copying, and ordered collective readiness epochs | Direct Verus passes 36 obligations with zero errors. All 9 release tests and 8 actual-body mutation checks pass; strict Clippy and independent review pass. Integrated in `4afe852`. |

The combined Ferric host checkpoint at `4afe852` passes 645 engine tests (9
separately gated tests ignored), 38 source-gate tests, formatting, strict
Clippy, and source admission. Its inventory has 172 modules and 8,228 executable
bodies: exactly one new module and 21 directly verified bodies, with no added
unverified bodies. Protected runtime feature resolution remains gfx942-only.

The preceding Ferric pin checkpoint used public fe2o3
`a8b016e14ca8c77c9e7abe4591086f7cab11ce61`, integrated locally in `c092df2`.
All fifteen locked graphs resolve, with no registry-version drift; independently
reviewed dependency records and the rebuilt source-pin adapter binding match.
The final-pin remote run passes 645 engine tests (9 ignored), 38 source-gate
tests, source-gate strict Clippy, formatting, property binding, and full source
admission. Connectivity has recovered. The interrupted engine Clippy log was
recovered with exit status zero; the remaining adapter/source-policy checks,
four isolated allocation tests, and both fresh engineering host release
builds now pass. No local build/test fallback was used.

The new gfx950 engineering object is 103,616 bytes, SHA-256
`2679e59626eee9939412aaf7af6a542c8aeccbe4dd13fb7e6ea3bdcf4f3b8222`,
content ID `431f1e294d5018e0f057d490495921a1983bac0c25e4e900c3f72a05350b3f84`.
It contains twelve Wave64 kernels with COV6 and exact replay; all authority
grants are false. Component provenance remains explicit: Ferric kernel source
`296bb98`, device dependencies `0ea54ed9`, compiler tool images `24e842d70`,
and final native-worker source `a9ea16247`. The matching gfx942 object has
content ID `fa6ccd9130001647932502d13092ca44f7ec78c9580984c96ce28f34710ab670`
and is byte-identical to the pre-worker-fix regression object. Neither object
is relabeled as a final-pin Ferric rebuild or a GPU execution observation.

Compiler-wide Clippy and aggregate-kernel Clippy are not reported as clean:
the existing backend has 344 source-attributed diagnostics, and the aggregate
retains 104 baseline kernel-body diagnostics. The changed compiler CLI/finalizer,
runtime, engine, source gate, and kernel build helpers pass their scoped strict
checks; no broad lint suppression or unrelated kernel rewrite was added.

The follow-on engineering execution batch uses published fe2o3 main
`3546d54d2c4a913f5d079701aed557d0a378bba8`. Its release gfx950 worker passes
433 library tests, a genuine single-device RMSNorm dispatch with exact BF16
output and unchanged guards, and eight concurrent isolated memory/queue
lifecycles with distinct device IDs, live executable hashes, and confirmed
child teardown. The eight-worker lifecycle fixture does not dispatch kernels
and is not a Qwen result.

The new thirteen-root TP crate includes six rank-local kernels and seven
unchanged imported helpers. Its frozen `8cdde149` source passes 31 host/source
tests per target, genuine gfx950/gfx942 emission and exact replay, and six real
gfx950 numerical probes with unchanged inputs/guards and clean teardown.
Eight probe-harness tests pass. The integrated driver, bounded child transport,
authenticated intake, and release controller pass 95 library/client tests,
21 source policies, and scoped strict Clippy. Combined source admission passes
with 173 modules and 8,235 executable bodies, alongside 648 engine tests
(9 ignored) and strict Clippy. The final annotated sequence cursor has an
exact same-source replay: eight Verus obligations, zero errors, three tests,
and an unchanged 754-file frozen closure. Its earlier actual-body mutation
checks remain separately scoped; none of this proves whole-driver numerics.

Real Qwen3-8B two-token smokes now pass on one, two, and eight MI350X GPUs.
All generate `[12095, 13]` (` Paris.`), matching both frozen reference prefixes,
with exact rank dispatch counts, KV position six, confirmed worker teardown,
and idle GPU observations afterward. The eight-rank smoke measures
160.644607963 seconds TTFT and one 24.711778123-second decode interval, with
186.586377561 seconds of setup excluded. These smoke intervals are not
repeated TPOT measurements or serving qualification.

The extended single unwarmed TP8 run also passes: all 32 generated IDs and
decoded bytes match both reference passes and argmax arrays. TTFT is
163.647853495 seconds; the mean of 31 post-first intervals is
40.25950984674194 seconds, with 193.519502061 seconds of setup excluded.
All eight workers close/reap, exact dispatch counts and KV position 36 match,
and GPU observations are idle afterward. This is a single-prompt engineering
observation, not repeated statistics or a controlled framework comparison.
See [the engineering runbook](M1_QWEN3_TP_ENGINEERING.md) for identities,
all four results, reproduction, and performance limitations.

Measurements use release binaries, host-staged FP32 ordered collectives,
contiguous rank-local KV, and token-at-a-time prompt processing. The initial
path scales poorly. Substantial system-call pressure was observed; repeated
full runtime currentness scans are the leading bottleneck hypothesis.
No gfx942 artifact or device authority is relabeled as
gfx950. Builds/tests stay on `mi300x`; execution uses `mi350`. Symmetric memory
and MTP remain deferred.

The core and kernel worktrees and roughly 36 GiB of completed build/proof
stages have been removed after evidence archival. This includes the final
core stage `mi300x:/tmp/fe2-mi350-engineering.B01UxK` and integration build
stage `mi300x:/tmp/ferric-mi350-integration.2WTV7c`. The small local integration
worktree remains the source workspace for the unpublished implementation.
The needed `mi350:/tmp/ferric-qwen8.TRNKht` model/runtime bundle is retained
for reproduction, with no workers left running. No other user's processes,
directories, or networking settings were modified.

## Historical M1 Checkpoint

| Team | Implemented And Checked | Current Work |
| --- | --- | --- |
| Integration and runtime | This checkpoint used published fe2o3 `0ea54ed921cbef5fb2171f4ed7a25b9c1c5e2b71`; all 15 lockfiles, dependency records, and the rebuilt source-pin adapter binding passed independent review without unrelated registry-version drift. Resident storage preparation, explicit production-owner close and recovery, and settlement allocation tests are integrated. All 23 reference unit tests pass. Verified full-block SHA updates and opt-in startup phase diagnostics are integrated. Combined host checks at `a6a3f2e` and the 32-token Qwen comparison pass. | Broaden the numerical workloads and profile remaining token latency. Authenticated serving still requires the protected authority bundle. |
| Kernels | Native Qwen `[N,K]` weight layout correction; K3 paged-KV writes copy active rows instead of scanning every physical page. Generic dynamic GridExclusive projection and singleton-invocation race handling are reviewed and published in fe2o3 main. The matching `0ea54ed9` 12-kernel aggregate emits successfully with exact replay and passes independent review. | Measure K3 with the existing independent reference; investigate matrix-instruction support for the remaining dense projection work. No controlled K3 speedup has been measured. |
| Verification | The complete developer qualifier passed on frozen `bb50d0e`: 1,586 selected Verus queries, zero errors, 694 admitted bodies, actual-body negative mutations, and all listed formatting, strict Clippy, debug/release and all-feature test gates. The receipt's 197 files pass hash validation. This checkpoint excludes the later SHA optimization, diagnostics, and `0ea54ed9` repin. The separate planner repair at `eb219f0` passes the positive plan and dependency rejection cases against `0ea54ed9`. Independent review accepts the new 32-token engineering observation. | The protected M1 qualification gates remain outside the developer receipt, the planner's synthetic policy tests, and this single-prompt hardware check. |
| Documentation | Public Pages site describes architecture, implemented functionality, limitations, and milestone status. | Publish the latest tested integration checkpoint without claiming a new benchmark or closed M1 gate. |

## Historical MI300X Qwen Observations

The historical Qwen3-8B target-only run completed 32 generated tokens from the
five-token prompt `The capital of France is`. Every generated ID matched both
frozen Hugging Face reference passes and both reference argmax arrays. The
decoded output continues through Paris, Rome, Madrid, Berlin, and Amsterdam.
Observation SHA-256 is
`d894caf042156abf21436c98fa3de7d40af124ba7374baa0b879bf7df582af44`.

| Measured Boundary | Seconds |
| --- | ---: |
| Host setup through KFD binding | 82.938931 |
| Memory initialization, allocation, and upload phase | 163.126609 |
| Cumulative setup | 246.065540 |
| Controller entry to first generated token | 13.442289 |
| Average post-first interval, 31 intervals | 2.769105 |
| Controller through completed teardown | 100.370762 |
| Whole process | 346.46 |

Startup diagnostics use `std::Instant`; controller events use
`CLOCK_MONOTONIC_RAW`. Arithmetic combining those clocks is approximate.
This is one shared-host engineering observation, not numerical or serving
qualification, a controlled K3 speedup, or a vLLM/SGLang comparison. The output's
`r33_tpot_eligible=true` expresses event-count/timing eligibility only; all
authority fields remain `none` or false and `benchmark_comparable=false`.
GPU 3 subsequently returned to its exact pre-run VRAM baseline. The immediate
post-exit counter was transiently higher; its mechanism was not established.

The earlier target-only Qwen3-8B smoke produced:

```text
Prompt: The capital of France is
Output:  Paris. The capital of Italy is Rome
```

That eight-token engineering observation recorded approximately 13.65 seconds
to the first token and 2.77 seconds per subsequent token. It is not serving
qualification or a result comparable to vLLM or SGLang. No K3 speedup has yet
been measured.

An independent offline Hugging Face Qwen3-8B run using the same canonical model
produced exactly the same eight token IDs on two passes. A separate 32-token
reference also repeated identically and preserves that eight-token prefix.
That historical Ferric run matches the reference across logical KV positions 16
and 32. These are not physical page boundaries (the KV page size is 256).
These short reference checks do not establish full-model numerical qualification.

The artifact used for this Qwen observation, compiled with `0ea54ed9`, has content ID
`d33933adf7f5dfe0a9aa4aba0cc4cb3909b5aa35f9bfb65db7af9af4a5c5bb40`
and HSACO SHA-256
`33d754aaa10292fa37e974eb004b6c52067a141dcd5b58ed080023c6bf315c2d`.
Its 12-kernel roster and compiler replay pass, with authority explicitly
`none`. Ferric device source is identical between emitted `9392755` and
integrated `eb219f0`; the matching release host is built from `a6a3f2e`.
The previous `42882993` artifact, content ID `528fa128`, is retained as
historical evidence and is not relabeled as the current artifact.

All eight shared GPUs were occupied before the September 8 launch attempt.
On September 9, GPU 3 was idle with almost all memory free, allowing the new
32-token attempt. No other user's process was stopped or modified.

An earlier K3 attempt exceeded a 240-second whole-process limit without a
result. Historical successful release runs took approximately 24-27 minutes
for the full process, but their source checkpoints (`6239bdb`, `f151965`) pin
fe2o3 `1ddcd36b`, before the parallel device-initialization change in `d211c9a`.
That older path copied serially and hashed GPU-mapped memory. Those wall times
are not a baseline for the current `42882993`/`0ea54ed9` initialization path.
The historical executable hashes were not archived, so those figures cannot
support a controlled comparison with the new phase-separated measurement.

A later non-dispatch probe of the `42882993` host and final artifact took
98.23 seconds through artifact admission, CPU preparation, runner binding, KFD
admission/topology, and intentionally failing device selection. It stopped
before `initialize_memory`, so it measured no HBM allocation/upload, dispatch,
tokens, TTFT, or TPOT. The two independent model-authentication passes run in
parallel. Setup is excluded from the controller timing above. The earlier
timeout is not a TTFT measurement or a demonstrated kernel failure.

A separate CPU-only probe of public `0ea54ed9` KFD descriptor constructors
measured 58.133 seconds of hashing for 97.313 GiB of input, plus 10.370 seconds
reading weight files. Repeated weight-descriptor calls model the second hash
payload, not the actual private allocation-time validation. This uses KFD's
registry `sha2 0.11.0`, not Ferric's verified SHA implementation. The helper
opened no KFD/render device and measured no HBM initialization or token timing.
It is a one-run engineering observation, not a startup improvement claim.

Both engineering smoke tools now accept exact opt-in
`FERRIC_M1_ENGINEERING_STARTUP_PHASE_DIAGNOSTICS_V1=1`. Six fixed cumulative
completion phases go to stderr only; stdout observations, admission, and
controller timing remain unchanged. The output explicitly grants no authority
and is not benchmark-comparable. The diagnostics passed 10 speculative and 17
target unit tests (one real-GPU test ignored), 20 source-policy tests, formatting,
strict release Clippy, and independent review before integration.

The verified SHA update preserves the existing streaming specification and
passes whole-crate Verus verification (296 queries, zero errors), 143 release
unit tests, strict Clippy, and independent source review. Five alternating CPU
benchmark matrices per build measured 1.115-1.119x throughput for 1 MiB update
chunks, with a 0.49-1.53% time regression for 63/65-byte chunks. This is not a
measured total-startup improvement. All seven regenerated source/dependency
records remain exact, with 171 modules and 8,207 executable bodies.

The combined `a6a3f2e` host check also passes 10 speculative, 17 target, and 20
source-policy release tests (one real-GPU test ignored), strict release Clippy,
formatting, and both release builds. Source-gate tests pass 36/36, and all seven
fresh records match. The later `eb219f0` changes only the planner and its tests;
its actual-history policy test accepts the 354-slot plan and rejects missing,
extra, moved, and incorrectly pinned promotion dependencies.

The reference package now pins Accelerate 1.14.0 and psutil 7.2.2, the actual
dependencies needed by the pinned Transformers loading path. Its exact-version
and virtual-environment provenance checks include both dependencies. The
existing 27 external lock records were preserved.

The new optimized prepack reproduces canonical bundle ID
`6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b`.

## Remaining Dependencies

- The complete developer receipt binds only `bb50d0e`/tree
  `7f3f23579b0adfac8ad7357fbf1246ab4b05dc95`, not the current tree. Receipt
  SHA-256 is `c09f212e82eace9326a6d0a0e47ff7a898a8901e5e531fa01ea52213b064b54b`;
  its unchanged source closure contains 711 files. The earlier `cb9b5ca` run
  remains an incomplete historical run that hit its 600-second cold-build
  bound. Neither receipt establishes GPU machine-code correctness or serving
  qualification.
- Authenticated serving qualification still requires the protected current
  record, verifier, model, and artifact authority bundle.
- Kernel performance work still includes matrix instructions for dense
  projections, attention, normalization, and logits reductions. Historical
  MI300X target-only source analysis found 545 dispatch packets per target
  decode token; this is not the new TP schedule or a measured attribution of
  GPU time.
- Physical radix prefix reuse belongs to M2. Symmetric memory and MTP remain
  deferred.

All builds and tests run on `mi300x`. Completed temporary build directories and
integrated worktrees are removed; retained evidence and branches identify the
work without keeping duplicate build trees. Compiler/runtime changes belong
in fe2o3; kernels and inference remain in Ferric.
