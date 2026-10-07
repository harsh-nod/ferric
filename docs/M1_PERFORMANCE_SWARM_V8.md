# Performance Swarm V8

Started 2026-09-24 UTC after the user requested implementation of the
projection, runtime, prefill and matched-measurement plan with parallel agents.
This is an engineering progress record, not M1 completion or a vLLM win.

Latest checkpoint (2026-09-28 UTC): the default-off packed gate/up model
controller completes its full CPU qualification and independent raw audit,
then passes actual-bound harness fixtures, cold full-model parity and native
ABBA. Packed TPOT improves 8.6..10.3%, but TTFT worsens 8.8..11.6%; keep it
opt-in. Its fresh matched HTTP pair also passes correctness and cleanup, but
Ferric is not competitive: packed mean TPOT is 55.775 ms versus vLLM 4.230 ms,
a 13.184x latency ratio. The separately measured 652-dispatch Ferric client
passes full-model native parity and a September 26 HTTP pair.
Native ABBA does not establish a repeatable token-program speedup.
Latest-runtime adoption remains a separate qualification gate.

## Baseline And Target

The historical three-pair matched Qwen3-8B series uses TP1/C1, 128 input and
128 output tokens, BF16 decoder with FP32 head, context 8192, greedy output,
prefix caching and speculation off. Ferric TPOT is 63.833..79.535 ms versus
vLLM 4.228..4.394 ms. See [V6 evidence](M1_PERFORMANCE_SWARM_V6.md).
The first competitive target is repeated sub-4-ms TPOT on that contract,
without sacrificing correctness or hiding a TTFT regression. It is not a
prediction or a claim about TP8, loaded serving, quantization or speculation.

## Team Progress

| Team | Current Work | Verification / Next Gate |
| --- | --- | --- |
| Ferric kernels | Added `device/qwen3-tp-packed-gate-up-kernels-v8`, default off, with independent strict accumulators and checked views | All eight scoped host phases and independent audit pass, including 16 Rust tests; emission and native checks remain open |
| fe2o3 runtime | Published exact qualified runtime changes to main as `807f0bef`, atop current `2af2a8d7` | CPU and synthetic native gates pass; CI skipped as requested; historical worker qualification labels unchanged |
| Ferric client | Integrated default-off 652-dispatch client, policy and positive-ABI checks | R14 CPU collection and exact 21-file integration pass. Both release controllers now pass cold parity and native ABBA; the token path has no repeatable timing gain |
| Verification | Full-model native and common HTTP replay | All 2048 native ABBA token IDs/decoded bytes match; the fresh HTTP pair passes numerical, timing, input-identity and clean-shutdown gates |
| Integration | Completed the September 28 packed single-pair HTTP comparison on mi350 | Ferric/vLLM TPOT ratio 13.184x; no win, default promotion, TP8 or sustained-serving claim |
| Packed model continuation | Integrated the default-off standalone packed gate/up path across exactly 20 files, preserving 1320 unrelated paths | Full mixed R3/R4 matrix and independent audit pass: 36 targets, 5813 passed executions, both 53-test policy suites, 39 image checks, actual ABI and retained controller. Final 26 harness fixtures, cold parity and all 2048 native ABBA token-ID/byte checks pass. Native TPOT improves in both orders but TTFT regresses; the matched HTTP pair remains substantially slower than vLLM |
| Next kernels | Standalone packed-down integration and a fixed-shape gate/up safe-load candidate are independently source-reviewed | Down413 revision-transition harness passes 23 fresh remote fixtures. Rust compilation is held for space; neither model proposal is integrated or native-qualified. FP32 output and actual ISA still need qualification |
| Pages | Published the remotely validated September 26 snapshot as `391b2a54` | Mobile wrapping and bounded capture repairs pass full remote QA; all 159 PNGs are retained, 22 selected views reviewed, deploy-only workflow succeeds and all seven live files match. The snapshot does not claim September 28 results |

Source-only proposals, CPU qualification, emitted artifacts, native numerical
results and performance results remain separate statuses. No default is
changed by authoring a proposal. Existing worktree changes are preserved.

### Packed CPU Continuation

Fast qualification R5 completes all 13 guarded phases on mi300x-2. The 44
focused tests, 53 policy tests and seven pairs of production check/strict
Clippy commands pass, with clean, unsignaled owners. Only four matching
test-only feature guards differ from R4; the other 1328 inventoried paths
remain identical. Complete custody is
`products/packed-cpu-fast-r5-complete-custody-a001.tar.gz`, SHA256
`412bdfa44a53a8dba096d109700266827014e60085a33bcc0ea0b581beebffdf`.
This is not the full matrix, a final release controller or native acceptance.

Full matrix R3 passes preparation, early policy, token, default, TP, model
and ordered families. The union family compiles and executes its eight
selected targets, then exits 1 at the additional 512-MiB after-phase planning
floor. Its guard reports no resource violation or signal, and reaps the
child cleanly. The 24-GiB cap and 512-MiB guard reserve are unchanged. The
failed owner is not promoted: an append-only continuation must replay the
retained same-source union ELFs under a new clean owner before completing
the remaining tests, image checks, ABI and controller build. The prefix and
failed-union raw record is
`products/packed-cpu-matrix-r3-prefix-union-failure-custody-a001.tar.gz`,
SHA256 `13ceeb1d1f79fcbdee661e5e41a7258056e479a9b08a3de7aad64fd1f165c7c2`.

An exact 25-file archived-cache retirement attempt stops at the whole-process
no-use census because privileged read-only inspection is unavailable. No
files are deleted, and neither the cleanup checks nor the hard resource
limits are relaxed. The smaller warm-build planning allowance is separate
from those limits and is not a measured peak-memory or peak-disk guarantee.

### Completed Packed CPU Qualification

On September 28, root resumes at the first unstarted R4 phase after confirming
that union replay and token-union completed cleanly before the session change.
No completed build or suite is repeated. Policy, images, ABI production and
consumption, the release controller and final collection all pass. Independent
raw review accepts 5813 passed executions across 36 target cells, 341 disclosed
ignores and seven instances of the existing filtered library test. Both
53-method policy runs and all 39 explicit image checks pass. The fresh ABI
graph preserves 652 dispatches at positions 143 and 144; the packed 688-dispatch
model path still uses ordinary transport, not that token program.

The final record retains 15 clean successful owners and the unchanged failed
R3 union owner. Its explicit continuation binds the original union compilation
and eight retained ELFs to freshly replayed R4 test outcomes. All 36 test ELFs
and the new controller are independently rehashed. Complete custody is
`products/packed-cpu-matrix-r4-complete-custody-a001.tar.gz`, 173050509 bytes,
SHA256 `8095e9776d29bbfa33cfbb0ad3d74a1ba0f043d7dcc1a1b6df007faa5c6d1c3f`.
The retained packed controller is 10586960 bytes, SHA256
`96a306e8b401abf46fa3720d2d5e65118f26a744cff8885f36003b085dc1fb5d`.
The final actual-bound native harness passes all 26 fixtures with no skips.
Its source manifest is
`2e0483d6225d7df6aa0b9cb0c10601d9c182bab9730371c0d13d343cf872d5dd`.
Both cold full-model arms then pass: all 256 generated token IDs and streamed
UTF-8 bytes match the reference. The same controller, worker, model and images
serve both arms; only the packed selector differs. Both arms allocate the same
7247757312 bytes of packed weights and 8192 bytes of scratch. Each arm's 135
batches match the expected schedule, with ordinary 652-dispatch versus packed
688-dispatch decode. No token-program transport is used by this experiment.

Independent raw audit accepts the 72-entry authenticated packed-weight census
at setup, close and report, plus unchanged input identities and clean, reaped,
unsignaled owners with empty final KFD use. The census is not an independent
rehash of raw weight payloads, and the expected packet-group count is derived
from source/tests, not an observed hardware counter. Cold custody is
`products/packed-model-native-r3-cold-custody-a001.tar.gz`, 4006249 bytes,
SHA256 `5120f10d12ab4683da5a2f1acd7584b8479039b5afd3c949e61178f2d43afa86`.
Cold parity is not a speedup result. The separate four-start ABBA timing
result follows below; no default promotion or vendor comparison is implied.

The resumed mi350 check initially finds eight visible PCI accelerators but no
loaded `amdgpu` module or `/dev/kfd`. One bounded explicit load of the already
installed driver succeeds. No package, persistent configuration or reboot is
changed. All eight GPUs subsequently report zero utilization and 297766912
bytes VRAM use each. Native launch still requires its fresh full admission.

### Packed Native Timing

The September 28 four-start ABBA screen passes independent raw replay. Each
fresh start has one excluded warmup and three measured requests. All 2048
generated token IDs and UTF-8 bytes match, including warmups. All 68 input
files remain unchanged, with identical packed setup allocations in both arms.
Each arm completes 540 batches: 350844 baseline versus 369132 packed dispatches.
All controller and supervisor processes exit cleanly without signals, and the
final KFD census is empty.

| Arm | Mean Native TTFT (ms) | Mean Native TPOT (ms) | Finite Ingress Tokens/s |
| --- | ---: | ---: | ---: |
| Ordinary A-AB | 884.924 | 63.034 | 14.396 |
| Packed B-AB | 987.272 | 57.592 | 15.416 |
| Packed B-BA | 931.702 | 53.142 | 16.662 |
| Ordinary A-BA | 856.124 | 59.245 | 15.272 |

Packed TPOT falls 8.634% in AB and 10.300% in BA, and finite output rate rises
7.087% and 9.096%. TTFT worsens 11.566% and 8.828%, respectively. This is a
mixed native result, not a default promotion, confidence/tail result or vendor
win. Setup and warmups are excluded; the finite cohort rate is not sustained
serving throughput. Do not compare these controller-ingress timings directly
with HTTP measurements or pool them with the prior token-program ABBA.

Complete custody is `products/packed-model-native-r3-abba-custody-a001.tar.gz`,
4435683 bytes, SHA256
`b3a546bebd9e92512666307f16a1f280d617b483b734815d4e1f4ede132976ea`.
The report SHA256 is
`b74bf34564f9fdd66b43dbece80562dffbe9d9d3f9d9472eb9833173b4f4c4d6`.
The actual-bound HTTP successor passes all 49 CPU fixtures and independent
audit; it preserves the matched workload and requires replay of this complete
native evidence before launch. No Rust rebuild or model copy is needed.

### Packed Matched HTTP Result

On September 28 UTC, the same packed B-AB controller, worker and ten-image
composition complete a fresh Ferric then vLLM 0.28.0 HTTP pair on mi350 GPU0.
The workload remains Qwen3-8B, TP1/C1, 128 input/128 output tokens, context
8192, greedy, BF16 decoder with explicit FP32 output head, prefix caching and
speculation off. Each engine has ten excluded warmups, thirty measured
requests and two untimed output diagnostics. No Rust rebuild, image pull or
model copy is performed.

| Metric | Packed Ferric | vLLM 0.28.0 | Ferric / vLLM |
| --- | ---: | ---: | ---: |
| Mean HTTP TTFT (ms) | 888.594 | 20.125 | 44.154x |
| Mean HTTP TPOT (ms) | 55.775 | 4.230 | 13.184x |
| Finite output rate (tokens/s) | 16.054 | 229.386 | 0.06999x |

The unchanged remote pair replayer and independent raw audits pass. All 42
Ferric outputs match the full token-ID and decoded-byte reference. All forty
vendor warmup/measured streams match text and usage; vendor token IDs are
checked in the two diagnostics only. All sixty measured requests succeed.
Both engines retain identical before/after inputs, clean process/container
shutdown and idle GPU endpoints. KFD journals match only the owned worker or
container at every sample; sampling does not establish absence between samples.

TTFT is client-send to first nonempty text; TPOT is first-to-last text divided
by 127, not true token ITL. Finite rate includes cohort gaps and drain. This
single fixed-order pair is not repeated-start confidence, sustained serving,
TP8, speculative decoding or a stock BF16-head ranking. Its difference from
the September 26 token-program pair is not an isolated packed-feature effect.
Keep the packed path default off; its native TPOT improvement does not amount
to a vendor win or resolve the native TTFT regression. HTTP measured TTFT
thirds are 871.105, 866.504 and 928.172 ms, not monotonic stabilization.

Full post-cleanup HTTP-stage custody is
`products/packed-http-gpu-r2-pair-custody-a001.tar.gz`, 656791 bytes, SHA256
`9442e1069e4282e5869c72fbffd7662126aacec449734a246cc7298a4371bf62`.
A fresh remote inventory exactly matches its 194 files and twelve directories,
9059184 logical bytes. The runner normally removes its ephemeral vendor cache
after container teardown; those already-discarded cache bytes are not retained.
The separate 14221-byte accepted replay output has SHA256
`89add2c7cce8deb6357e286f01f6de3d107bc68574b6be0781ac10256f8fe1ec`.

After retention and independent review, the completed cold, HTTP and ABBA
GPU stages are removed in that order. An initial cold cleanup refuses a
changing process census before deletion; one explicit quiet retry retains
the same all-UID checks and succeeds. The three no-use scans cover 3438
current processes and removal reclaims 44093440 allocated file bytes. All
408 files match their retained archives, and deletion uses ordinary UID9661,
not privileged deletion. The borrowed model and other jobs are untouched.
Small external cleanup-control directories remain as diagnostic evidence;
no new worktree is created by this continuation.

### Next Packed-Down Gate

The standalone FP32 down-projection proposal advances to a narrowly scoped
413 SDK qualification recipe. Source review finds that a manifest-only update
would conflict with two tests asserting the old 390 revision. Before any
execution, exactly four revision literals are updated: two dependency pins
and those two expectations. All ten other source files, kernel arithmetic,
activation layout and the V5 FP32 oracle remain byte-identical. The historical
lock is only a seed for later actual offline resolution, not a fabricated 413
lock or a reused 390 qualification.

All 23 harness fixtures pass on mi300x-2 in 0.528 seconds with no skips or
failures, under a clean, reaped, unsignaled G24 owner. Full source and fixture
custody is `products/packed-down-cpu-413-r2-fixtures-custody-a001.tar.gz`,
119452 bytes, SHA256
`1a49f8f59e23691dadaaf4eee04bcab18c0b4386c5d5abd29c648c34bb98cdfe`.
This checks the qualification recipe only: no Rust compilation, actual new SDK
resolution, producer rebuild, emission, native run or model integration occurs.

The next Rust build is held. Historical down390 workload compilation alone
grew the stage by 855519232 bytes; its prepare-to-retain span grew by
1144438784 bytes. These are net observations, not scratch-peak guarantees.
They exceed the roughly 216 MiB currently available above reserve. In
particular, the copied 768-MiB compile planning floor is below the historical
816-MiB net growth and does not justify a new launch. A separate budget audit
supersedes that wording without mutating the sealed recipe. No cap increase,
reserve reduction or cleanup-check bypass is used to advance the build.

### Pages Publication

The frozen September 26 snapshot completes all four bounded R6 phases on
mi300x-2. Full responsive QA covers every width from 320 through 1440 pixels;
96 main and 63 supplemental screenshots are retained. Root directly reviews
eight selected views and the Pages reviewer fourteen, including complete
mobile continuation frames. The unmodified artifact receipt and separate
root visual-acceptance record preserve this provenance.

The deploy-only `pages/prebuilt` commit is
`391b2a54201d8cc14458ca4779ae6ede8d127326`. Workflow `36455572213` succeeds,
and an independent HTTPS fetch on mi350 verifies all seven live files against
the 869442-byte static payload. No GitHub-hosted project build/test, local
project build or new worktree is used. GitHub API publication preserves the
exact Git objects and uses a non-forced fast-forward; the API's one-terminal-LF
message representation difference is independently checked before publication.
TLS verification and environment protections remain enabled. The public
snapshot remains dated September 26 and does not include this new HTTP pair.

## Fresh Matched Measurement

On September 26 UTC, one fresh Ferric then vLLM 0.28.0 HTTP pair completes on
physical GPU0 of mi350. Each engine runs 10 excluded warmups, 30 measured
requests and two untimed exact-output diagnostics. The common contract is
Qwen3-8B, TP1/C1, 128 input/128 output tokens, context 8192, greedy output,
BF16 decoder and explicit FP32 output head, prefix caching and speculation off.
Ferric uses the qualified `807f0bef` worker/client, prefill16, V19 KV copy,
split8 attention and the opt-in token program; this is not a latest-SDK build.

| Metric | Ferric | vLLM 0.28.0 | Ferric / vLLM |
| --- | ---: | ---: | ---: |
| Mean HTTP TTFT (ms) | 890.522 | 19.567 | 45.510x |
| Mean HTTP TPOT (ms) | 51.443 | 4.377 | 11.752x |
| Finite output rate (tokens/s) | 17.240 | 222.219 | 0.07758x |

All 60 measured requests succeed. Ferric checks full token IDs and decoded
bytes for all 42 requests; vLLM checks token IDs in the two diagnostics and
decoded text/usage in the 40 warmup/measured streams. Both engines retain
unchanged pre/post model/image identities, clean shutdown and idle GPU/KFD
postflight. The unchanged pair replayer accepts the complete results.

TTFT is client-send to first nonempty text; TPOT is first-to-last text divided
by 127, not true per-token interarrival latency. Rate is 3840 tokens divided
by each finite measured cohort including gaps and drain, not sustained loaded
throughput. This is one fixed-order pair, not a repeated-start confidence or
tail result, stock BF16-head ranking, TP8 result or speculative comparison.
Do not pool it with the historical three-pair series or native ingress timing.

Complete HTTP custody is `products/token-http-gpu-r1-pair-custody-a001.tar.gz`,
628238 bytes, SHA256
`0fed9d4682fcdf1a2e9e43715179a713c92ab033f54ec3ebf5dd63be5e5de137`.
The 13699-byte pair summary has SHA256
`8ea42b053b11716d805b7865fed98279163da959b8d43903d0b75d394d5a37f3`.
Independent raw replay passes in
`proposals/perf-v8-token-http-first-pair-final-independent-audit-r1.md`,
SHA256 `312d75a6519ffd6f4e32a5a95cc55c8faf6562b85e5e0d1ac48b8cbde28be09a`.

After complete custody and replay, the HTTP, native ABBA and cold stages are
removed using fresh whole-process no-use checks and ordinary-owner deletion.
This reclaims 63090688 allocated file bytes; the three temporary manifests are
also removed. Cold cleanup attempts a001/a002 remain failed in the record;
the separately selected a003 passes unchanged checks after the two other
stages retire. Read-only privileged process inspection is used, never
privileged deletion. Borrowed model files and all local evidence remain intact.

## Native Model Measurement

The model-harness R6 CPU attempt fails on a fake-controller parameter-name
collision. R7 repairs only that helper parameter and pins; all 24 fixtures
then pass on mi300x-2 under the existing 24-GiB/four-core guard. The separate
HTTP harness passes all 49 CPU fixtures. No Rust source or binary is rebuilt.
The mixed R12/R13/R14 client CPU provenance remains explicit.

Both cold paths pass all 256 output-token and decoded-byte checks. The native
ABBA run uses four fresh controller starts, each with one excluded warmup and
three measured requests. All 2048 generated tokens, including warmups, match.
Every controller and supervisor exits cleanly without signals or KFD leftovers.

| Arm | Mean TTFT (ms) | Mean TPOT (ms) | Finite Ingress Tokens/s |
| --- | ---: | ---: | ---: |
| A-AB: ordinary ordered64 | 871.966 | 60.702 | 14.916 |
| B-AB: token program | 890.295 | 55.296 | 16.175 |
| B-BA: token program | 859.552 | 55.297 | 16.238 |
| A-BA: ordinary ordered64 | 821.900 | 54.161 | 16.621 |

Candidate/base TPOT is 0.91093 in AB but 1.02098 in BA: 8.9% lower in one
order and 2.1% higher in reverse. TTFT is 2.1% and 4.6% higher respectively.
The baseline drift prevents a repeatable speedup claim. These are controller
ingress measurements, not HTTP, GPU-only duration or a vendor comparison.
The fixed prompt ends at context 255; this does not qualify the 256-to-257
fallback transition, loaded serving or TP8.

Native ABBA custody is `products/token-model-native-r7-abba-custody-a001.tar.gz`,
7038551 bytes, SHA256
`421788f3be9ebc42dcd590dd1e3c208b605415eedb640afa88d442aa0a98967c`.
The accepted report has SHA256
`34ef4b4e3eff4e55c023c2b656b90f102b17964bac78a4d1d2e6b183ec4d4ab7`.

## Currentness And Resources

Fresh GitHub `main` observation selected
`b7d5f2bf7bf9e66037df1ff1d7ca2738ffdf98dd`, retained as
`refs/ferric/github-main-20260924-b7d-currentness-v8` in the existing bare audit
repository. The runtime candidate targets this exact revision. Earlier 091 CPU
results do not qualify b7d; historical 413 compiler/SDK and 5e2 worker artifacts
retain their original identities while the coordinated refresh proceeds.

A later main observation is `1b2e5dd364e63c5d107115f379e9c21a6a84236e`,
three commits after b7d. All nine packages in the actual runtime normal
dependency closure, plus root manifests and the toolchain pin, have identical
Git objects. This supports only runtime-input equivalence, not relabeling the
qualified worker. Compiler code changes, and the fused host closure includes
one changed kernel-IR file. Those results remain at their actual revisions.
The comparison is retained as `products/fe2o3-currentness-1b2-v8-r1.json`;
any core push still requires rebasing on the then-current main.

Main advances again to `2af2a8d7dc75d8edac3da50325c91ea1911f8324`. The
same nine runtime crate trees and root manifest/toolchain inputs are still
identical to b7d. The independently reviewed comparison is retained in
`products/fe2o3-currentness-2af-v8-r1.json`. On top of that exact current main,
the qualified runtime is published as
`807f0bef70da75c81e56de6eb4fd6e9c1f78e5e2`: exactly seven modified and nine
added Rust files under `crates/fe2o3-kfd`, with every other tree entry unchanged.
The normal non-force push succeeds, and GitHub reports zero Actions runs for
the `[skip ci]` commit. No local build, dirty-worktree change or new worktree
is used; the temporary private Git index is removed afterward. This is exact
runtime-source integration, not a rebuilt whole compiler or a relabeled ELF.

At 23:08 UTC, main is `6a58e31a470bdb068c3b7859b0287cdb4a58623a`,
a descendant of `807f0bef`. All nine runtime crate trees and all 896 mapped
SDK/workspace-input files remain identical. The 71 changed paths are outside
that closure. The comparison is retained as
`proposals/perf-v8-runtime-currentness-6a58-r1.json`; it establishes runtime
source equivalence only. Client qualification remains pinned to `807f0bef`,
and neither the historical worker nor compiler artifacts are relabeled.

After the session restart, main is
`1e368326c3d21cf3354b4fa839e45177cfa29282` at 04:05 UTC on September 25.
The intervening commit changes seven documentation files only; all 896 runtime
closure files still match `807f0bef`. The fixed-object comparison is retained
as `proposals/perf-v8-runtime-currentness-1e36-r1.md`.

CPU builds/tests remain remote on mi300x-2 under the approved four-core,
8-GiB-RSS, nice-19, 1200-second, 20-GiB private-stage owner with the unchanged
reserve, host-resource and cleanup checks. GPU execution remains on mi350.
No local project builds/tests or GitHub CI are authorized by this campaign.
The user separately approves a 24-GiB private-stage cap for the Ferric
token-client feature matrix. Its fresh profile passes 30 remote fixtures;
all non-cap limits remain unchanged. Earlier campaigns and receipts keep
their original 20-GiB cap.

Fourteen obsolete local source-copy directories are removed after all 47,646
files and directory rosters match retained archives and active agents confirm
no use. This reclaims 1092571136 allocated bytes. No worktree, evidence archive
or remote stage is removed. The local process census finds no readable
references but is not a whole-host absence certificate; that limitation is
disclosed in `products/local-source-reclamation-r2-execution-a002.json`.
The shared CPU stage's stricter cleanup requirements are unchanged. A fresh
September 28 read-only visibility probe identifies the current obstruction:
ordinary inspection cannot read root PID1's executable link, and the existing
noninteractive privileged inspection requires a password. This is not evidence
of a stale SSH session. The G24 owner exits 77 cleanly, with no signals or
deletion. Custody SHA256 is
`30a00d8c2abeaa31ba2fda97fc79115e4623ca2efc270098f3728645fb0e8578`.
The unchanged stage has about 217 MiB of growth above its reserve. No cache
reclaim, larger stage or new emission budget is admitted by this diagnostic.

## Packed Pair Retry

At 20:18 UTC, MI350 reports all eight GPUs idle with no SMI process entries.
Fresh staging independently checks idle/KFD state and reopens the qualified
input evidence. It creates `/tmp/ferric-opt-v5-pair.bdfd8bcb` with ordinary
private copies, not shared inodes or symlinks. The failed predecessor stage
`/tmp/ferric-opt-v5-pair.ZAKjrVaQ` and its raw results remain untouched.

The unchanged native limit is 2 GiB per active stage; the host free-space and
memory floors remain 64 GiB and 128 GiB. Staging completes in 6.22 seconds with
1611706008 copied payload bytes and 1615724544 allocated bytes before its final
record. This temporary duplicate is retained only for execution and subsequent
archive-backed cleanup under fresh no-use checks.

The actual retry plan SHA256 is
`b78ce7b1c3ebc4523460a2f206e869faf7343defc2d12ce4704b6c3763c28f09`;
the unchanged qualified runtime source manifest is
`1ac8423d4b44f8fd50638e402e9e58a878e46cea7880de92b7d0275a1ef0ab03`.
The copied worker is the actual historical 1796208-byte ELF, SHA256
`dd6bd3b4a910478e85d2f3530be25819153fad1f08bf33bb14dd534514a601a2`.
Only explicit retry-stage locations in the plan and ISA paths are rebound;
ISA and input-document hashes are recomputed, not historical CPU receipts.

The retry ran from 20:25:41 to 20:26:26 UTC. Exhaustive packing and the first
baseline parity cell passed; foreign KFD activity interrupted the next cell.
The supervisor returned 125, sent TERM only to its owned work, and reaped its
child. Its overall `cleanup_ok` is false because foreign KFD work remained.
Zero timing cells completed. The subsequent staging attempt refused admission
before creating another directory because KFD was occupied again.
After the user approved a 25-minute idle window, attempt A003 also refused
before creating a directory: Scout's parent and eight load-generator children
still owned KFD. A reservation start time or scheduler pause is requested;
the Scout supervisor is not signaled.
At 21:14 UTC, a fresh observation instead finds eight root-owned PyTorch
matrix-multiply jobs (PIDs 1281310..1281317, parent 1281294), whose command lines
specify 23 hours. They are not the previously approved `/tmp/worker.py` children.
Approval to stop these exact jobs is requested; no signal is sent.
At 21:36 UTC the jobs have already exited: KFD is empty and all eight GPUs
report zero utilization. The user's subsequent exact-job stop approval is
not needed. A fresh retry stages at `/tmp/ferric-opt-v5-pair.563a4c13` and
starts the unchanged 33-cell diagnostic under fresh admission.

The complete 5171200-byte small-result archive has SHA256
`4491dd9ab2e31cca7b977bac4834118ede03ae2bd395c24c2ddc7fbdb582943a`.
It joins the unchanged historical object archive. The new failed stage is now
retired: all 1441 retained file hashes match; a whole-process inspection covers
3519 current processes with no stage references; ordinary-UID deletion removes
1616506880 allocated file bytes. The first cleanup attempt refused process
roster churn, and its failure is retained. The successor checks newly appeared
process identities while tolerating confirmed exits; it does not skip process
visibility. Read-only privileged inspection was used, never privileged
deletion. Temporary cleanup helpers are also removed. The older failed stage
and model files remain untouched. An uninterrupted idle window is requested.

The fresh 33-cell run completes at 21:58:48 UTC with a clean, unsignaled
supervisor exit. All 65536 packing patterns, 16 finite-parity cells and 16 ABBA
timing cells pass. Independent review checks all 512 measured groups and the
retained input/output records. Controller time per gate/up pair is:

| Pairs Per Group | Input Epoch | Existing (us) | Candidate (us) | Speedup |
| --- | --- | --- | --- | --- |
| 1 | 0 | 449.406 | 269.547 | 1.667x |
| 1 | 1 | 457.662 | 273.077 | 1.676x |
| 5 | 0 | 263.116 | 150.925 | 1.743x |
| 5 | 1 | 263.212 | 152.711 | 1.724x |

Candidate timing includes activation packing and both projections; both arms
exclude weight upload. This is a warm, synthetic single-weight-pair controller
benchmark, not GPU-only time, the full model working set, current-source
qualification, Qwen TTFT/TPOT or a vendor comparison. Complete small-result
custody is 12390400 bytes, SHA256
`86d880d8284e4adaad40bd304204c20633cc8f20aac1fe4600b97a222db7ac7f`;
it joins the unchanged archived objects. Independent audit SHA256 is
`234cc956625fc1bf751bf129cf6338759fdc53fd5f85352ef02f27dc69ce57d6`.
The completed private stage is removed after archive-backed, whole-process
no-use checks, reclaiming 1623932928 allocated file bytes. The two wait stages,
token smoke stage and temporary retirement manifests are also removed. Two
initial wait-stage cleanup attempts refuse process-roster churn before any
deletion. After a fresh read-only census stabilizes, separately chosen attempts
pass the unchanged checks. Total allocated file bytes reclaimed are 1634713600;
all evidence archives, model files and older retained stages remain intact.

## Runtime CPU Results

The first recipe compiles the library but refuses its test listing: fixed b7d
adds two upstream ignored artifact-input tests to the prior native ignore.
That failure is retained. The R3 successor explicitly checks all three exact
identities and reuses the unchanged, pinned formatted source without another
source copy or mutating formatter. A V4 receipt discloses that reuse.

All twelve R3 phases finish under clean, unsignaled approved CPU guards:
27 recipe fixtures, library and worker-CLI compilation, 13 focused tests,
655 full-library passes with three disclosed ignores, four CLI tests, 53
doctests, strict Clippy, the ROCr ABI oracle, worker build and final retention.
The focused cohort is a subset of the full library, not additional coverage.
The 68 predecessor keepers remain byte-for-byte and identity unchanged.

The actual 1851392-byte worker has SHA256
`ee51921d32a3778eb9f218882573cd9a7e58df367d3b2fd658fc38159412b1ca`.
Its compact receipt is 105902 bytes, below the unchanged 128-KiB envelope.
The 47994880-byte complete CPU custody archive has SHA256
`baa0820fbb75979a6fc45c6c91dc46fd4e51e3b9d586828a0256aaa825526181`.
Independent raw review passes over all source/tool bindings, command joins,
test identities and clean guards. The audit SHA256 is
`2449c1c6209daba210ac424b65b4cb5d03925873cbef4e797035ce5d986ddac2`.
These are CPU results, not GPU
correctness, whole-compiler/SDK qualification, or a speedup measurement.

## Kernel Host Qualification

The R2 recipe passes six fixtures but stops during offline metadata resolution:
Cargo's existing Git database lacks the exact b7d commit. No kernel Rust test
has run in that attempt. Its complete failure custody remains retained.

R3 imports only the pinned incremental Git bundle into the existing owned Cargo
database, verifies both prerequisite commits and preserves existing refs. It
uses a fresh source/review directory and the unchanged 20-GiB owner. The
524-MiB preparation allowance and 384-MiB actual post-prepare compile floor
are checked without reducing the reserve. All eight phases pass: seven recipe
fixtures, preparation, compilation, 11 host tests, five AST/target-contract
tests, the inert `gfx950` feature check, strict Clippy and final retention.
The fused device feature itself is not compiled by these scoped host checks.
All ten Ferric source files now match the remotely formatted source; no local
formatter runs. Complete custody is 10178560 bytes, SHA256
`6aff759e7f03777759dee855ea9c252092c506cdf62dfd6fc169bd0c3c7f7d3d`.
Independent raw audit passes, including all 1222 SDK closure hashes against
fixed b7d, the 14-package Cargo closure, all 20 command/log joins and preservation
of all 104 preexisting Git cache refs. The audit SHA256 is
`ebac76fab899d426a7d74d97ce354d4e0990415547d4532719c2c93bf6f1f45b`.
This is not GPU emission or native kernel qualification.

## Wait Smoke Readiness

All 24 harness fixtures pass on mi300x-2 under a clean unsignaled approved guard,
including actual V4 worker receipt admission and rejection/cleanup cases.
Protocol children in these CPU fixtures are fakes; the actual Rust worker is
only read or copied. The first wrapper invocation used an invalid `--cwd`
argument and stopped before a guard/test launch; A002 is the actual passing
attempt, not a relabeled A001.

Independent audit accepts the 542720-byte complete fixture custody archive,
SHA256 `bce282c22c11abd483d7df8f9aaf6d5743d79d472927022605314a5cf3af925f`.
The real 15026-byte harness receipt is
`c9ec2a76e2b2180b664da7eeaceed18811f22979acb00bacc4c1393bb52af51c`.
Data-only staging creates `/tmp/ferric-opt-v8-wait.9671001d` on mi350 with
2437120 allocated bytes. The native run passes its default-policy arm:
210 dependent packets, 528 guard checks, exact outputs and clean unsignaled
exit. Before the active arm, the zero-utilization check reports `GPU is busy`.
The supervisor exits 1 cleanly, with no owned or foreign KFD users recorded
afterward. Residual utilization from the preceding arm is a hypothesis, not
an established cause. No active worker is launched and the pair is not passed.
Full failed custody is 3123200 bytes, SHA256
`acef3fb94810e0667bf06b4e3b27cecdbb1d4224c72e337c2a9c467b599a9901`.

R3 adds only a bounded five-second between-arm settle/recheck, still requiring
zero utilization and unchanged full KFD/resource admission before proceeding.
Foreign users and other errors reject immediately. All 28 CPU fixtures pass
with independent review. Its fresh 2441216-byte GPU data stage is
`/tmp/ferric-opt-v8-wait.c531111c`. After the kernel-pair run completes, both
native arms pass: 210 packets and 528 guard checks per arm, exact outputs,
rollover and clean unsignaled exits. Independent raw audit passes, SHA256
`f8f4bc6d8752ea230628d0f8944ced7c0af824c32cd21290ff43d22984fccc95`.
Full custody is 3829760 bytes, SHA256
`f2dc3ab23e3cd0ac4c4b54ae487fa122d0e300a61da2109ac47782752c6fedd7`.
These correctness checks do not measure a wait-policy performance benefit.

## Token Program Candidate

A separate fe2o3 source candidate retains one bounded dispatch template and
accepts explicit positional scalar/pointer updates. It validates the complete
request before publishing fresh groups of at most 64 packets under one absolute
deadline. It does not reuse queue packets or completion signals. Lifecycle and
rejection tests pass in the scoped CPU qualification below. Ferric full-token planning,
failure handling and output commit integration are implemented in a separate,
frozen source proposal, not yet applied or qualified. The fixed TP1/C1 V19+split8 forward has 652 packets
and 180 positional updates, within the 1024/256 bounds. Metadata writes and
choice readback remain outside the program. This targets
repeated protocol/template transfer, not all host overhead.
The 652-packet template is specific to split8's context range 128..256;
multirow prefill and other C1 contexts need the ordinary path. Explicit template
release and ordinary/token transitions are implemented and untested. Thirteen
authored client tests cover driver transitions, aggregate failures, IPC
identity/frontier checks and policy selection; one actual-image test is
explicitly ignored pending retained input. The new feature requires an
unpublished fe2o3 overlay, not plain upstream b7d, and is not runnable yet.

The token CPU staging A001 rejects a mistyped checksum before creating its
directory or changing source. The append-only corrected stager succeeds as
A002 with the original input archive. Actual recipe execution then reports
29 passing fixtures and one stale assertion: it expects the old 448-MiB
preparation allowance instead of the reviewed same-path 160-MiB allowance.
The clean guard exits 1 without signals. No prepare, compilation or Rust test
starts. Complete failed custody is 3020800 bytes, SHA256
`ec0e4d296d849a55d963983e9c4354b566c895f40a97b8e6ace38a0f9b0285a5`.
The fresh R2 recipe corrects the stale fixture without changing core source or
resource limits. All twelve phases pass, with independent raw review:
30 recipe fixtures, 13 focused tests (a subset), 677 library passes and the
same three explicit ignores, six CLI tests, 54 doctests, strict Clippy, ROCr
ABI checking and worker retention. The actual 2023280-byte worker SHA256 is
`5edabd3c998e15cb0b69567cf27b83d1c01c1911fdd4e6635980d575e297c560`.
Complete custody is 20838400 bytes, SHA256
`890b9b305192f242c47f0112c19b1d4bd7bf17813176856d3e9d4c1c7d64fe70`.
All 68 older retained binaries keep their exact bytes and file identities.
The shared SDK source directory now holds the explicitly recorded token
successor; the old wait recipe must not be resumed against that directory.
Copied wait workers and their historical receipts remain unchanged.

The Ferric client also has separate source-policy and actual-ABI test overlays.
The latter captures the real recording driver's full graph at positions 143
and 144, then checks packing and template reuse against retained image metadata.
It is authored but unrun, and uses synthetic weight placeholders rather than
model arithmetic. Selective qualification-only resolution of the new SDK
alias is approved; no historical dependency alias or default route is changed.

The token-native harness passes all 30 remote CPU fixtures under a clean,
unsignaled 20-GiB guard. Its complete 604160-byte custody archive SHA256 is
`4cdb6864e8bb8256e336434f91166f0c8c59d1f12f5d0daa95b3a13b51d1a01f`.
Independent CPU audit passes. The native run then passes all three serial
cases: default-entry rejection, 195 exact dependent packets with changed input
buffers and rollover, and the expected invalid 65th-pointer rejection. The
supervisor exits cleanly without signals. Complete native custody is 3338240
bytes, SHA256
`57c560b04adf4a4f2c72f4827de0abe6d5151e1982c99a36f5e70d81d24fc775`.
Its independent audit passes, SHA256
`b4e3f0b8a8403e38e59ce1f01232dcb4020a38ea173e629172eba7782dec4907`.
Fatal rejection prevents trustworthy
after-error readback, so this is not native proof of zero GPU publication.
It is also not the Ferric full-token graph or model inference.

The separate client preparation passes under the approved 24-GiB profile.
Initial compilation then fails on three test-only `LowerHex` uses of the
sha2 0.11 digest type. No client Rust tests run. The clean, unsignaled failure
is retained in a 61552640-byte archive, SHA256
`26f45a0397398c2d846bd2a6689739719b23080bb72490a41d4b983d0e335814`.
A one-file successor uses explicit lowercase byte formatting and adds golden
digest tests. The runtime, manifest, lock and other source bytes are unchanged.
All four initial R3 phases pass with clean, unsignaled guards: preparation,
compilation, 463 library passes and 98 binary passes. The library's 12 ignores
and one existing exclusion, plus the binary's 10 ignores, remain disclosed.
The new hash regression and all regular authored token methods pass. Complete
custody is 84152320 bytes, SHA256
`b59a5339b1318d948d97a1070c3d3998115f736ce9d3faf7952094e042e8c958`.
Independent actual audit passes, SHA256
`a7f95439279def56e2054d29c8108e10d225ffa2fe6f8d476a618aa9e534a93a`.

These initial binaries use the explicit path SDK. The public-SDK successor
now pins only the new alias to `807f0bef`. Offline Cargo resolution and an
independent audit pass: all 343 lock rows match the exact nine-package source
transition, all older Git refs and dependency aliases remain unchanged, and
only Cargo.toml and Cargo.lock change in the 1321-file source inventory.
Preparation custody is 4843520 bytes, SHA256
`74aeeb96d28b5b2f1f8ed2c4f2751f9784538d7572c1444d0d77de321fdb07af`.
Git package identity requires fresh compilation; path-built binaries cannot
be relabeled. The full seven-family matrix, strict Clippy, actual-image
and positive 652/180 ABI gates, controller retention and native model tests
remain required.

The first public-Git matrix attempt passes its three recipe fixtures, then
stops during the token family's Clippy run when all three existing resource
census retries race with removal of a rustc temporary metadata path. The guard
reports status 125, TERM, child reaped and `cleanup_ok=false`; this family is
not accepted, despite its completed 463-pass library log. Failure custody is
96419840 bytes, SHA256
`336fc25f747d8e1697a4834625d55b591d02571ad2ae1e1d30c2e4867b44c2ca`.
A separate read-only observation finds all seven recorded processes absent
and no owned build processes. It does not rewrite the failed cleanup receipt.
The bounded full-census retry correction passes all 38 remote profile tests
under a clean, unsignaled guard. Up to eight full-scan attempts share one 15-second
deadline and one million visited entries; partial counts are discarded and
all other errors remain fatal. Profile custody is 112640 bytes, SHA256
`5caf07cd61105f0c32dd1543c22b0fdd8ce45e6a6d855c7e3b5aaae957d2033d`.
Fresh matrix qualification remains pending; the 24 GiB stage cap and all other
resource limits remain unchanged.

The R3 retry completes its owner lifecycle cleanly, then fails strict Clippy
on five new-client lint errors: one wildcard import, three narrowing casts
and a function-pointer type annotation. This is not another guard failure.
Failure custody is 96491520 bytes, SHA256
`e35b150169c2f8a43521715f55ecee864a89983cea197d9b3b5ce8f725301ddc`.
A three-file, behavior-preserving correction passes source review, including
explicit test-local imports. R4 preparation and its three recipe fixtures
pass remotely. The lint-first token family then stops on `large_enum_variant`:
the new optional token state increases `Worker` to 352 bytes versus the peer
variant's 64 bytes. Its owner exits cleanly without signals. A narrow optional
state allocation repair is being prepared; coverage and resource limits remain
unchanged. No matrix family or controller is qualified by this failed attempt.

R5 boxes the optional token state without changing the pending-request or
transport protocol. Its preparation and recipe checks pass after the session
restart; strict Clippy no longer reports the oversized worker enum. The full
test-target lint check instead exposes a redundant binding in the new layout
test and fourteen test-only issues in prior recording/prefill helpers. No
Rust test suite runs in this attempt. The owner exits cleanly and unsignaled;
complete compressed failure/source custody is 8678569 bytes, SHA256
`8f5ac63079f91671977adfe7d6a72353bd4f17bfdbd8dafbb8a152f0a222a610`.
Narrow test-code repairs are in progress, with no global lint relaxation.

R6 applies the six reviewed test-file repairs. Preparation and recipe fixtures
pass, but another target reports the boxed `Worker` at 272 bytes against the
64-byte peer variant, still above the enum lint threshold. No Rust test suite
runs. Compressed failure/source custody is 8745205 bytes, SHA256
`5faee5ea91522169f65d42b59721a8e5fb2e14b07c7d60d6acef875637152df2`.
The next repair boxes the independent-rank enum variant at its one cold
construction site, with no per-token allocation or request-protocol change.
Clippy will use `--keep-going` to report independent target errors together;
the family still fails on any error and all resource limits remain in force.

R7's rank-wrapper allocation repair clears the enum issue; three remaining
test-only semicolons stop strict Clippy. R8 repairs those and passes preparation,
recipe fixtures and the complete token family: production checks, strict
all-target Clippy, release compilation and full tests for six selected targets.
The default build then fails because `cfg!` does not suppress type-checking of
the token-only `Split8V21` enum variant. No later family or ABI gate runs.
Complete partial-result/source custody is 36043033 bytes, SHA256
`2d2f0266edacc0b48c7545fec8035158dfbf5fdba49fd568261c73627199bd3e`.
The owner exits cleanly and unsignaled. R9 replaces this selector with actual
compile-time gates, preserving the enabled predicate and disabled false result.
Preparation and all three recipe fixtures pass, followed by full token and
default-family passes. TP production checks pass, but strict Clippy finds two
`unused_self` diagnostics in ordered helpers when ordered batching is disabled.
No later family or ABI gate runs. The owner is clean, reaped and unsignaled;
complete partial-result/source custody is 40101917 bytes, SHA256
`fe1a1cbbf26bccd423e0f7267179eed8f58fcc342c905d42a03e6b9b0a9c050c`.
A narrow R10 repair is being prepared, with all-family compilation/lint
preflight before expensive tests to expose configuration issues together.
All full tests and resource limits remain required. Successful prior families
remain historical, not relabeled as qualification of a later source.

R10 repairs the two receiver lints and passes preparation plus six recipe
fixtures. Its diagnostic-first run collects all 14 check/Clippy commands:
all seven production checks and token/default/TP/ordered Clippy pass. Model,
union and token-union Clippy report three Boolean-to-integer test expressions
in `model_timestamps/tests.rs`; union and token-union also report one narrowing
cast in `batched/tests/c1_ordered64.rs`. The zero-error aggregate fails
cleanly, with no full Rust suite, image gate or controller build launched.
All ten owners are reaped and unsignaled. Full source/diagnostic custody is
9146610 bytes, SHA256
`acb27ba0d68a6c8c3d49f236faf27d847503af4675a9c661b3e3464e7a34b204`.
R11 will use exact typed `From<bool>` and checked integer conversions, with no
lint allowance or production behavior change, then rerun the same 23 phases.
This adds only those two existing test files to the 21-path integration scope.

R11 applies those four conversions and passes all 14 diagnostic commands and
six recipe fixtures, with independent raw audit. A separate source review
finds that the policy assertion expects an unbraced closure while actual
formatted `performance.rs` uses closure braces. Root briefly acquires the
existing build lock after the last diagnostic owner exits; the next aggregate
admission is denied before any child or receipt, then the lock is released.
No signals or source changes occur. All nine executed owners finish cleanly;
no aggregate, full Rust suite or policy test is executed in R11. The source
finding is not described as an executed test failure. Complete partial custody
is 9128961 bytes, SHA256
`0d67a6d403f11205e4a2625d7960e5a011b00e4920325af0757d484a1a98385d`.
R12 repairs only the expected closure braces and schedules the existing 52-test
policy suite after token test compilation, before the longer full suites. The
normal later policy gate also remains required; scope stays at 21 files.
Independent source and final-packet audits pass, followed by actual preparation
and six remote recipe fixtures. The full 23-phase matrix is running on
`mi300x-2`, under the approved 24 GiB cap and unchanged other limits. Actual
preparation custody is 2314240 bytes, SHA256
`4845ad42b5dd329507074a56d8ab6e3e4a566b6badc4b6fe1d9b4a32056fd528`.
All 14 fresh check/Clippy commands and their zero-error aggregate pass, followed
by seven full feature families and 32 test executables: 5168 passing executions,
278 documented ignores and seven instances of the existing library exclusion.
These are execution counts, not unique-test counts. Both 52-test policy gates,
ten actual-image checks and ABI graph production pass. ABI consumption fails
at `tp_worker_token_program_tests.rs:693`: 2569 pointer fixups versus 2279
expected. The owner exits cleanly, reaped and unsignaled; no controller build,
final collection, client integration or full-model qualification occurs.
All 21 actual phase guards are retained with full source and result custody:
159755835 bytes, SHA256
`c20f53144a687ebef14d3df9bc38e116d5c0b06ae161f69f2a19b76af383da33`.

Independent source/graph analysis derives 2569 total buffer arguments at both
positions: 2279 nonempty plus 290 zero-length RMSNorm slices. The latter are
two arguments in each of 73 wave RMSNorm and 72 Qwen RMSNorm dispatches. The
packer intentionally retains a pointer fixup for every buffer argument. The
failed assertion counted only nonempty buffers. A strengthened test-only
correction is being prepared; later encoding and fake-IPC assertions remain
unexecuted and must pass, not be inferred from this diagnosis. Only five token
binary test targets include this test file. The focused continuation will
retain unaffected R12 evidence with explicit source/dependency attribution,
fresh diagnostics and affected-target/gate execution; it will not relabel
old ELFs as newly tested source or relax the 24 GiB cap and other limits.
The read-only, build-lock-held dependency capture verifies all 32 original
ELFs against their retained copies and actual Cargo compiler-artifact records.
Exactly five token binary dependency files include the changed test module;
the remaining 27 exclude it. The retained capture is 197180 bytes, SHA256
`7f2a82ab79fe12401dd4422dd7492b86274048fe850c578adaa95701dffb5d9f`.
The five affected ELFs total 76263120 bytes. The proposed R13 continuation
retains an explicit inherited-versus-fresh evidence distinction and runs the
actual ABI checks immediately after the affected binary build, before the
remaining suites. It is not qualified merely by this source review.

R13 completes 17 phases successfully: preparation, nine recipe fixtures, all
14 fresh check/Clippy commands and their aggregate, five affected binary test
targets, actual ABI production/consumption, both 52-test policy gates and ten
actual-image checks. The full 1321-file source inventory changes only the
declared test file; actual formatting produces SHA256
`057d459793d834971bce7724ead0e383fae36a91d1980506015c8e90e4d92d0d`.
The ABI report at positions 143 and 144 confirms all 2569 pointer fixups,
72 pointer slots and 108 scalar slots, then passes encoding and fake-worker
IPC. This is still CPU-only, not native model parity. Compiler-generated
dependencies are checked around every command; the 18-input capture and the
five-fresh/27-inherited boundary remain explicit.

Both ordinary release controllers compile successfully in 47.63 seconds,
with the same source and `c1-token-program` features. The phase then fails
during retention because the generic evidence binder requires a single link,
while Cargo's top-level executable and `release/deps` executable share an
inode. The owner returns 1, cleanly reaped and unsignaled; the compilation
command itself returns 0. A locked read-only census accounts for exactly two
owned canonical aliases per controller. No test or production source changes
are needed. A narrow successor will validate these exact already-built bytes
and create independent single-link copies before final collection. The failed
R13 owner remains failed, and no final CPU qualification is claimed yet.
Partial custody is 36589911 bytes, SHA256
`e2175387fd37d12dc3ac76582c9998f95dfcda20703b1b1eebccb036bd55f156`;
alias capture is 10020 bytes, SHA256
`b386a71eaabfef4b3306781d8c04a94035167bf61ed8da8b567d64740ffe2a6e`.

R14 completes all three narrow phases: six retention regression fixtures,
independent controller copies and final collection. It runs no Cargo build,
changes no Rust source and executes neither controller. All three owners pass
cleanly, reaped and unsignaled. Final custody is 14129621 bytes, SHA256
`6cd9ddc4863658850bbfeef2407efa05c6d0c9fbc7c28f60935fbf8cccbbd2ef`.
Independent audit verifies the complete V4 receipt, including 27 inherited
R12 test targets and five refreshed R13 binaries: 5168 passing executions,
278 documented ignores and seven instances of the existing library exclusion.
These are execution counts, not unique tests. Both ordinary controllers match
the actual successful R13 Cargo outputs; explicit provenance records their
R13 build and R14 retention, without promoting the failed R13 owner to a pass.
The final V4 receipt is 60059 bytes, SHA256
`c5a377f797da4f4449534d94b08ae72982526fd50ff4c0a950387a92c9b89c41`;
independent audit note SHA256 is
`7c8e14c28017bb01e5e4fc4b6ebd796884caa1775f7d40eaa0a75cc5223051b2`.

The append-only R9 admission joins that final qualification to the unchanged
R8 source patch. Root applies exactly 21 files after fresh preimage/mode checks;
all resulting hashes match tested source, and 1311 unrelated worktree paths
remain unchanged. Existing modes are preserved and the five additions use
0644. The post-apply receipt is
`products/ferric-token-client-integration-r9-postapply-a001.json`, SHA256
`6a3791dd35c0cbae38adcba255774390966e3a5e3f95d47320d77f035e94cd35`.
The feature remains opt-in. No local build/test, Git commit/push, native model
run, default promotion or new performance claim accompanies this integration.

Source review of the pending model harness also finds that per-arm acceptance
is recorded before metric validation. The outer campaign still rejects a
metric failure, but the retained arm can contradict that failure. An append-only
repair and regression fixture are required before native use. CPU harness
fixtures remain unrun pending the separately requested stage-scope approval.
The R2 source-only repair and its 22-fixture roster now pass independent review.
It sets acceptance only after successful metrics; the new fixture checks
unaccepted retained failures for both arms. Controller and qualification pins
remain unset, so this draft cannot admit native execution.

At the earlier R5 checkpoint, the source-only model harness incorporates the actual V4
mixed-generation evidence and explicit R13-build/R14-retention continuation.
Independent source review passes, with 24 authored fixture methods and
unchanged workload/worker/execution behavior. Its actual execution authority
remains unset. The separate 24-GiB-stage approval for those CPU fixtures is
still pending; neither the fixtures nor native model runs had executed then.
The later R7 and matched measurement results above supersede that readiness
status without relabeling the failed earlier attempts.

## Remaining Work

The following dated observations preserve the earlier continuation history;
the completed September 28 CPU, native and HTTP results above supersede their
then-pending work. The current next actions are listed after that history.

The September 25 04:48 UTC GitHub main observation is `d6b5887f`, one
compiler-test/evidence commit after `1e368326`. All 896 mapped runtime and
workspace inputs still match qualified `807f0bef` by mode, Git blob and SHA-256,
including all 762 files in the nine complete runtime crate trees. This is
source equivalence only: the client pin, historical compiler/worker labels,
and outstanding qualification gates are unchanged. Exact currentness data is
retained in `perf-v8-runtime-currentness-d6b5-r1.json`.

The next explicit GitHub observation is
`89c8c8992e8ec51f4db5d9fb6a0725e3a5a9f4e6`. All nine runtime crate trees and
their 762 paths still match `807f0bef`, but only 894 of the broader 896 mapped
inputs do: `Cargo.lock` and `crates/fe2o3-artifacts/Cargo.toml` now include a
kernel-descriptor dependency. The additive artifact-schema API is outside the
client's nine-crate runtime dependency closure; existing artifact loader code
is unchanged. R12 continues qualifying its exact `807f0bef` pin. A newer Git
pin requires fresh lock/package-identity validation and client gates; adopting
new compiler artifacts requires separate image, ABI and native qualification.
See `perf-v8-runtime-currentness-89c8-r1.json`; do not extend the historical
all-896-equivalent claim to this revision.

At September 25 06:20:26 UTC, GitHub main is
`fd1b32e8d36f72a2d478424471287d322a05506e`. Eight runtime crate trees remain
identical to `807f0bef`; KFD changes three files and adds two, leaving 759 of
the old 762 paths identical and 764 current paths. The broader mapped set has
891 of 896 old inputs identical. New conditional-dispatch APIs affect the
gfx942 direct-dispatch path; existing gfx950 engineering worker, token-wire,
ordered-queue and wait source files are unchanged. Runtime manifests, features
and dependency edges are unchanged, but this is not whole-runtime or binary
equivalence. The currentness record is
`proposals/perf-v8-runtime-currentness-fd1b-r1.json`. Actual adoption requires
a new SDK snapshot and client/worker qualification; R13 remains exactly
`807f0bef`, not a latest-revision build.

The September 26 observation is
`1f4d83c4d8edf70f32adaf45ffd4892df796aafc`. Its complete runtime closure and
896 mapped inputs are identical to the prior `fd1b32e8` observation. The
eight-of-nine-tree equivalence and KFD three-modified/two-added distinction
from `807f0bef` still apply. The comparison is retained in
`proposals/perf-v8-runtime-currentness-1f4d-r1.md`. The packed controller will
compile with the token feature to select the measured 807 SDK alias, but its
explicit packed selector uses ordinary transport, not token execution.

The packed integration's independent source/postimage audit passes. All 12
preimages matched without a rebase; eight files were added, 1320 other paths
were unchanged, and HEAD/default selection did not change. Remote fast R2
preparation then stops cleanly at check-only rustfmt, before live source writes
or Cargo. The reported formatting-only changes in the packed recording tests
and source-policy test are applied. R3 preparation and production compilation
then pass, but strict Clippy finds one redundant empty match arm in a test.
Removing only that arm preserves the wildcard behavior. Fresh R4 preparation,
production compile and strict all-target Clippy now pass on mi300x-2 under the
unchanged four-core/24-GiB guard. Its prepared 1329-file inventory has SHA256
`6a914960a0043e6b9aaaa2317526a5bb7158d1666d09155a73ec5dd4b5aa074e`.
R4 then passes all 44 focused tests, all 53 policy tests and default-feature
diagnostics. The TP-family check passes, but strict Clippy reports `dead_code`
for two failure-injection variants whose consuming tests are feature-disabled.
The collector stops cleanly; no later R4 phase is launched. Complete partial
custody is `products/packed-cpu-fast-r4-partial-custody-a001.tar.gz`, 13049748
bytes, SHA256 `1980e63818d6811910c0707063dba3ff0af255264c81f42ae241281833f29cf4`.
Four matching feature guards on those test variants and their two injection
hooks are independently reviewed and applied; production source is unchanged.
Fresh R5 preparation, token-feature diagnostics and focused compilation pass;
remaining tests were then running under the same bounds. The completed R5
and full-matrix outcomes are recorded above.

The separately staged native-harness draft passes all 26 fake-controller
fixtures with clean temporary-file cleanup. Its custody is
`products/packed-model-native-r1-preflight-custody-a001.tar.gz`, 133954 bytes,
SHA256 `7522d19a9c1a84940512d4810de6c15be0056cf30e2ff9ba6ffcd35cb9892d82`.
At that draft checkpoint, actual controller pins were unset and a final bound
fixture rerun was mandatory. Those checks, the completed 36-target matrix and
the subsequent native model measurements are now recorded above. The draft
fixture result alone never established native or performance acceptance.

1. Close the packed path's measured 13.184x TPOT and 44.154x TTFT gap against
   vLLM. Native parity, ABBA and its matched HTTP pair are complete at the
   actual retained revisions; neither packed nor token paths earn promotion.
   Latest-runtime adoption needs separate SDK/client/worker qualification.
2. Qualify the standalone packed-down FP32 workload against the retained 413
   producer, then check actual emission/ISA, pack-inclusive native parity and
   full-model performance. Preserve the measured gate/up TTFT tradeoff.
   Measure default/active wait policy separately;
   the completed correctness smoke is not a performance result.
3. Qualify the fused projection only if its emitted/native evidence supports it;
   retain existing unsuccessful prefetch experiments rather than repeating them
   without a new hypothesis.
4. Keep prefill and decode selection separate. Larger prefill GEMM/fusion and
   reusable decode submissions need implementation and measured selection;
   neither is completed by the current kernel or wait-policy patch.
5. Run uninstrumented exact-output model pairs for surviving changes, followed
   by a fresh matched vLLM series. Loaded batching, prefix reuse, TP tuning,
   speculation and quantization remain separately matched workload campaigns.

Evidence and source drafts are under
`/home/harsh/.codex-tmp/ferric-perf-swarm-v6/{products,proposals}`. The remotely
validated September 26 Pages snapshot is published as recorded above; today's
new packed HTTP result is currently in these progress records, not that frozen
public snapshot.
