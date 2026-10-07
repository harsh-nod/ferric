# Ferric project site

This directory deploys a dependency-free static GitHub Pages site. Project
status is kept in [`data/project.js`](data/project.js); curated, strictly
checked performance observations live in
[`data/performance.js`](data/performance.js). The harness pins Playwright for
real browser checks. Run all checks on the designated remote build host, not
locally, and remove the private stage after archiving evidence.

## September 29 Fixed Safe Screen

This additive source checkpoint is not yet remotely validated or published.
It preserves all prior HTML/JavaScript benchmark records and their original
producer and qualification boundaries, including September 26 HTTP data.
The project update date becomes September 29; earlier checkpoint labels remain.

On one MI350X on mi350-2, fixed-safe R3 completes 36 cells: ten finite cases in
each of two activation epochs, followed by 16 A/B/B/A timing cells. All seven
full-buffer/guard records pass, with clean unsignaled teardown. Each timing
cell has a fresh worker, two excluded warmup groups and 32 measured groups;
the 512 inner samples are not 512 independent worker repetitions. Prepacking
and uploads are excluded. Both arms retain compiler/SDK `1a5999f6` plus the
same capture patch and the historical `5e2`/`dd6` runtime.

| Pairs/Group | Epoch | Order | Baseline (us/Pair) | R3 (us/Pair) | Improvement |
| ---: | ---: | --- | ---: | ---: | ---: |
| 1 | 0 | AB | 624.215 | 623.537 | +0.1086% |
| 1 | 0 | BA | 624.504 | 624.165 | +0.0543% |
| 1 | 1 | AB | 626.242 | 625.710 | +0.0851% |
| 1 | 1 | BA | 623.300 | 625.341 | -0.3275% |
| 5 | 0 | AB | 217.030 | 217.230 | -0.0920% |
| 5 | 0 | BA | 218.166 | 218.418 | -0.1153% |
| 5 | 1 | AB | 218.988 | 219.872 | -0.4037% |
| 5 | 1 | BA | 206.882 | 217.991 | -5.3698% |

These are instrumented controller-wall microseconds per cache-hot gate/up pair,
not GPU time, model TTFT/TPOT or serving throughput. Positive improvement means
lower candidate latency. R3 has no useful measured gain and is not promoted;
the final faster baseline is retained, not excluded or pooled away. Independent
data-only review matches all 36 retained results, 512 samples, raw protocol
counter snapshots and eight comparison calculations. Result archive SHA256 is
`ceed68468e5b30e30dc64fa89b9cdf79cc3cc421b9703fe5141a8a8bf352a5c0`.

Each 32-group cell records 321 operational-currentness checks. Their overlapping
host scope records about 384-387 us/pair for one pair per group and 77-79 for five,
with zero measured transfers or kernel admissions. Token programs are already
integrated and measured on historical `807f0`: ordinary/token TPOT is
60.702/55.296 ms in AB and 54.161/55.297 ms in BA, with no repeatable gain.
Latest source review indicates the same minimum ten currentness checks per
backend group for token and matched ordered64 paths, plus five outer token
checks; these are source counts, not measured token counters. Latest-client
qualification and measured group utilization/batching analysis are the next
priorities, not a proven GPU bottleneck or guaranteed currentness reduction.
Counter scopes must not be added or subtracted to infer GPU duration.

Separate R4 passes 36 CPU tests per feature mode, strict Clippy, 16 native-harness
and 11 launcher fixtures. Emission a002 succeeds with captured ISA/ELF metadata;
static review accepts the finite-only diagnostic, with no completed native
R3-versus-R4 result recorded here. Separately built
core worker `4fb8ae50` passes 106 library and six CLI tests. Latest client source
and wire migration remain CPU-unqualified, its lock is not regenerated, and
full-model qualification remains pending. No new HTTP/vendor result, default
promotion, rejection/trap qualification or M1 gate closure follows.

Source validators bind every new paragraph, all eight rows and five evidence
identities, including the negative final comparison. Browser checks verify the
expanded table/identities and add desktop/mobile evidence screenshots. The new
checks have not run locally; remote structural and browser qualification is
required before the unchanged seven-file artifact can be considered for the
existing deploy-only `pages/prebuilt` publication path.

## September 24: Packed Screen And Runtime Publication

This source-only checkpoint replaces only the superseded packed summary and
adds bounded runtime/client status. The site remains unvalidated and
unpublished until fresh remote static/browser checks, screenshot review and
the separately admitted seven-file Pages artifact. Historical failed attempts
and all earlier model/vendor measurements remain unchanged evidence.

The fresh packed campaign passes all 33 cells: one exhaustive 65,536-pattern
packing cell, 16 finite-parity cells and 16 timing cells. Each of four separate
cohorts uses ABBA worker order, two excluded warmup groups and 32 measured groups
per worker. Each arm has two independent workers per cohort; inner groups are
not independent worker repetitions. The baseline uses two dispatches per pair;
the candidate includes activation packing and two consumers, three dispatches.
Host weight preparation and uploads are outside the timed interval.

| Pairs Per Group | Queue Epoch | Baseline Wall Per Pair (us) | Packed Wall Per Pair (us) | Candidate/Baseline |
| --- | --- | ---: | ---: | ---: |
| 1 | 0 | 449.406 | 269.547 | 0.599785 |
| 1 | 1 | 457.662 | 273.077 | 0.596679 |
| 5 | 0 | 263.116 | 150.925 | 0.573607 |
| 5 | 1 | 263.212 | 152.711 | 0.580181 |

Ratios use sums of raw measured controller-wall samples within each cohort,
not averaged cell ratios or pooled cohorts. The observed 40.0% to 42.6%
reduction is a 1.67 to 1.74x baseline/candidate ratio for this cache-hot
12288x4096 synthetic gate/up pair only. It is not GPU time, TTFT, TPOT,
throughput, a model gain, statistical significance or a vendor comparison.
The packed worker is still historical 5e2/dd6 with compiler/SDK 413, not the
new runtime. Numerical-trap/model gates and all 33 M1 gates remain open.

The separate wait smoke passes both arms, 210 dependent packets and 528
guarded readbacks per arm; the active policy completes six groups without
fallback. Token native uses its own later worker and passes 195 dependent
packets across reuse/release/rollover, with exact default and late-pointer
fatal rejections. Negative exits are unforced/reaped but not clean_exit true.
The late failure does not directly prove zero GPU publication. Neither smoke
is a timing comparison or full Ferric-client/model execution.

The initial client R3 CPU suite passes 561 tests (463 library, 98 binary),
including the lowercase fixed-width SHA256 golden method. All 22 ignores and
one pre-existing library exclusion remain disclosed. Positive actual-image/ABI
checks and the full public-Git feature matrix remained open at that earlier R3
checkpoint.
The offline SDK migration passes independent audit, changing only the new
alias and its nine package identities while preserving all Rust source and
historical dependency pins. Preparation custody SHA256 is
74aeeb96d28b5b2f1f8ed2c4f2751f9784538d7572c1444d0d77de321fdb07af.
The historical public-Git R8 token feature family passes checking, strict Clippy and
release tests across all six selected targets. Its default feature family
fails compilation: a `cfg!` condition still type-checks the unavailable
`Split8V21` variant. R9 uses true `#[cfg]` guards: its token and default feature
families pass checking, strict Clippy and release tests. TP production checking
passes, but strict Clippy fails on `unused_self` in two ordered helpers when
`c1-ordered64` is off. No remaining family ran in that historical R9 attempt.
R9 partial custody is retained as
`ferric-token-client-matrix-r9-partial-custody-a001.tar.gz`, 40,101,917 bytes,
SHA256 `fe1a1cbbf26bccd423e0f7267179eed8f58fcc342c905d42a03e6b9b0a9c050c`.
That is failed-attempt/partial-pass custody, not a completed feature matrix.

Historical R10's diagnostic-first preflight completes all seven production checks, all
passing. Strict Clippy passes token, default, TP and ordered configurations.
The timestamp-enabled model configuration fails three test-only
`bool_to_int_with_if` locations; union and token-union also fail
`cast_possible_truncation` at `src/tp_execution/batched/tests/c1_ordered64.rs:257`.
There are four distinct test-only locations across two files, not only three.
The zero-error gate fails cleanly, and no expensive full-test phases run in
R10. Its partial custody is
`ferric-token-client-matrix-r10-partial-custody-a001.tar.gz`, 9,146,610 bytes,
SHA256 `acb27ba0d68a6c8c3d49f236faf27d847503af4675a9c661b3e3464e7a34b204`.
R11 repairs those four test-only lint locations. All seven production-check
and strict-Clippy pairs pass: 14 commands. The attempt intentionally stops
before the diagnostic aggregate gate and full suites after the source-policy
exact-text assertion is found to omit the actual closure braces. These passing
diagnostic commands do not constitute full feature-matrix qualification.
Actual R11 partial custody is retained as
`ferric-token-client-matrix-r11-partial-custody-a001.tar.gz`, 9,128,961 bytes,
SHA256 `0d67a6d403f11205e4a2625d7960e5a011b00e4920325af0757d484a1a98385d`.
R12 changes only the two braces in that policy assertion and adds an early
gate for the existing 52 source-policy tests, not 52 new tests. All seven
production-check/strict-Clippy pairs pass again, 14 commands, and the diagnostic
aggregate gate passes. The early policy gate also passes all 52 existing tests.
All seven R12 feature families and 32 test ELFs pass, totaling 5,168 passing
test executions, not 5,168 unique tests. The family counts retain 278
hardware/authored ignores and seven feature-family instances of the existing
library exclusion. Both 52-test source-policy gates pass, as do all 10
actual-image checks and the ABI producer. Those separately counted gates do
not turn the family execution count into a unique-test total.

The R12 actual ABI consumer fails after its slot checks: it expects 2,279
pointer fixups but observes 2,569. Independent analysis resolves the stale
test-count assertion: 2,569 total equals 2,279 nonempty plus 290 required
zero-extent RMSNorm fixups, two for each of 73 wave RMSNorm and 72 Qwen RMSNorm
dispatches. Zero-extent descriptors still require their pointer fixups. The
R13 repair is limited to the token-worker test file, not production code;
R12's failed gate is not retroactively relabeled as passed. Its controller
build, final qualification collection and integration were not reached.
All 21 attempted R12 phase guards report clean, reaped, unsignaled process
cleanup; cleanup is not a pass for the failed ABI-consumer phase.

Full partial-attempt custody is retained as
`ferric-token-client-matrix-r12-partial-custody-a001.tar.gz`, 159,755,835 bytes,
SHA256 `c20f53144a687ebef14d3df9bc38e116d5c0b06ae161f69f2a19b76af383da33`.
This is failure/partial-pass custody, not a completed 23-phase qualification.
Historical failed attempts, including test-only LowerHex R2, stay failed.

The focused R13 continuation passes 17 phases: actual preparation, nine recipe
fixtures, all 14 production-check/strict-Clippy commands and their aggregate,
five fresh token-binary suites, both 52-test policy gates, 10 actual-image
checks and the actual ABI producer/consumer. The whole 1,321-file source
inventory is 169,637 bytes, SHA256
`66fa120349f824279ca6dde3c6679a2018369a72a2d467b11ac7660e715e8d96`.
The sole formatted test file is 33,057 bytes, SHA256
`057d459793d834971bce7724ead0e383fae36a91d1980506015c8e90e4d92d0d`;
the other 1,320 source files are unchanged. These two identities are distinct.
Exactly five token-binary test targets are rebuilt, while 27 unaffected R12
target cells retain explicit inherited source, ELF and outcome identities.
Actual compiler dependencies establish the five-target closure, and all 18
generated compiler inputs outside the source inventory are guarded against
content, path or mode drift. This is not 32 freshly rebuilt targets, a global
source-equivalence claim or a production-code repair.

The R13 release-controller Cargo command succeeds in 47.63 seconds. Its
retention phase then fails because the generic file binder rejects Cargo's
two hardlinked output aliases; that failed guard remains failed. R14 performs
no Rust change or rebuild. Its six recipe fixtures, exact two-alias independent
controller copy and final collection all pass, with three clean phase guards.
Both independently retained controllers remain unexecuted. The final V4
receipt explicitly joins R12 inherited targets, R13 fresh tests/builds and
R14 controller retention. Its 5,168 execution passes, 278 ignores and seven
instances of the existing library exclusion are not a unique-test count.

Final custody `ferric-token-client-matrix-r14-final-custody-a001.tar.gz` is
14,129,621 bytes, SHA256
`6cd9ddc4863658850bbfeef2407efa05c6d0c9fbc7c28f60935fbf8cccbbd2ef`.
The 60,059-byte V4 receipt has SHA256
`c5a377f797da4f4449534d94b08ae72982526fd50ff4c0a950387a92c9b89c41`.
Independent final CPU audit is retained in
`perf-v8-r14-final-v4-independent-audit-r1.md`, SHA256
`7c8e14c28017bb01e5e4fc4b6ebd796884caa1775f7d40eaa0a75cc5223051b2`.

Root then integrates exactly 21 qualified postimages into the local Ferric
checkout with the 652-packet client opt-in still default-off. Postcheck verifies
every owned byte/mode, preserves 1,311 unrelated paths and leaves HEAD unchanged.
Its 5,179-byte receipt is
`ferric-token-client-integration-r9-postapply-a001.json`, SHA256
`6a3791dd35c0cbae38adcba255774390966e3a5e3f95d47320d77f035e94cd35`.
This is exact-807 CPU qualification and local source integration, not native
execution, full-model parity, serving qualification, a default promotion or a
new model/vendor score. R5's 24-method native-harness fixture qualification
still awaits explicit CPU approval; its actual-input candidates grant no
authority and its execution pins and owner remain unset. No local project
execution or site QA/publication accompanies this source update.

Core commit 807f0bef70da75c81e56de6eb4fd6e9c1f78e5e2 is actually published on
fe2o3 main atop 2af with [skip ci], by a non-force push. Its 16 source files are
the exact retained formatted runtime bytes. Runtime dependency inputs match
qualified b7d; the historical CPU/native receipts are not relabeled as builds
of the new commit, and no whole-compiler qualification follows. A retained
22:25:10 UTC observation reports zero Actions runs, not a promise about future
activity. Ferric SDK pinning/qualification is separate from core publication.

The previous retained main observation is
`d6b5887f55a4b7592b6905f4415ec88b544a0191` at
`2026-09-25T04:48:38.528937+00:00`. All 896 mapped files and 762 complete runtime
closure paths matched published 807 at that historical checkpoint. Its retained
record is
`perf-v8-runtime-currentness-d6b5-r1.json`, SHA256
`f25c1389de7ab60cc9ef56c21329df2d716a02183744b70ea493a3f86c37d19b`.

The historical retained main observation is
`89c8c8992e8ec51f4db5d9fb6a0725e3a5a9f4e6` at
`2026-09-25T05:32:19.541314+00:00`. All nine runtime trees and their 762 closure
paths were equal to published 807 by mode, Git blob and recomputed SHA256.
Only 894 of the broader 896 mapped files were equal: `Cargo.lock` and
`crates/fe2o3-artifacts/Cargo.toml` differ. The new artifact/descriptor API is
outside the nine-crate runtime closure; existing loader and runtime dependency
edges are unchanged. The retained review record is
`perf-v8-runtime-currentness-89c8-r1.json`, SHA256
`c6c31d236ba8fd5d234b6d66a0e67ee4422f445c1b41378be0391f0962cb28b5`.
That historical source equivalence was not a rebuild or latest-compiler
qualification and does not extend to newer main.

Latest observed main is `fd1b32e8d36f72a2d478424471287d322a05506e` at
`2026-09-25T06:20:26.539571+00:00`. Eight of nine runtime trees remain identical
to 807, but KFD changes. Of the old 762 runtime paths, 759 are identical and
three modified; two added paths make 764 current paths, with no deletion.
The broader comparison has 891 of 896 paths equal: the prior lock/artifacts
manifest differences plus three KFD files. Runtime source equivalence no longer
holds. The new conditional gfx942 direct-dispatch path leaves the existing
gfx950 worker, token-program, ordered-batch and wait source files unchanged;
this does not imply newly linked binary or timing equivalence. Runtime manifests,
features and dependency edges are unchanged. The retained source-only record is
`perf-v8-runtime-currentness-fd1b-r1.json`, 8,665 bytes, SHA256
`3a2c767bc3fce525e6ffbb5884d6e3550e6f5307f32f4eac106716cb4c8970d5`.
The integrated client remains CPU-qualified against exact 807, not fd1b.
Adoption needs a separately pinned SDK snapshot, fresh lock/package identities,
remote rebuilds, client/worker gates and bounded native correctness. Latest
compiler or new image qualification remains separate. Historical binaries and
evidence retain their original identities.

Public evidence disclosure binds packed report 7e3e71e0..., wait report 592d5b1b...,
token report 77b77a30... and initial client archive b59a5339..., each with its full
SHA256. Complete protocols, binaries, source inventories and failed attempts
remain outside the public artifact. No model/vendor score changes: the last
matched HTTP series still has Ferric 15.100 to 18.157 times slower than vLLM.

Exact source assertions and negative mutations bind the mixed-generation
seven-family/32-target CPU receipt, 5,168 execution passes rather than unique
tests, retained ignore/exclusion counts, five fresh versus 27 inherited targets,
dependency/generated-input checks, and the test-only repair. They distinguish
2,279 nonempty from 2,569 total fixups and the required 290 empty RMSNorm fixups.
They preserve the failed R13 retention guard, successful no-rebuild R14 copy/
collection, 21 locally integrated default-off postimages and pending R5 fixture
authorization, native/model outcomes and vendor comparison. They also scope
89c8 historically, bind fd1b's changed runtime closure and deny latest-runtime,
latest-compiler or historical-binary relabeling. Earlier failed receipts remain
historical and separate; the four existing public evidence identities are
unchanged, and no complete R12 qualification is claimed.
Render assertions retain existing views and add
desktop/mobile runtime-token-v8-progress and client-token-v8-progress captures:
88 main images plus the unchanged 63 supplementary images, 151 total. Full
claims and caveats must fit their focused captures on both viewports; no
screenshot result is claimed before remote execution. Existing CSS, JavaScript
rendering, project/performance data and all unrelated draft changes are intact.
A fresh source-bound QA successor must replace the prior 84-main/147-total
bindings; never mutate or rerun that frozen historical recipe against new bytes.

## September 23-24: Native Screens And Attribution

The new progress block keeps the prefill improvement and decode regression
together. It does not alter any J1/J2 or HTTP/vendor data. In four fresh native
Qwen3-8B TP1/C1 starts on mi350, all 2,048 output IDs and decoded bytes match.
Each start excludes one warmup and measures three 128/128-token requests;
context8192, BF16 with explicit FP32 head, prefix caching and speculation off.
Both arms use chunk32, the same controller, actual dd6 worker built at 5e2,
nine images, V19 decode, split8 and ordered64. Only baseline V5 prefill append
versus two V27 page copies changes. This is not chunk16 versus chunk32.

| Native start | Mean TTFT (ms) | Mean TPOT (ms) | Output tokens/s |
| --- | ---: | ---: | ---: |
| Baseline AB | 2041.379 | 65.646 | 12.332303 |
| Two-page copy AB | 565.331 | 70.619 | 13.424622 |
| Two-page copy BA | 597.046 | 66.836 | 14.086379 |
| Baseline BA | 1954.125 | 61.811 | 13.054065 |

Candidate TTFT is 72.306% lower in AB and 69.447% lower in BA; TPOT is
7.575% higher in AB and 8.130% higher in BA. Finite-window ingress output
rate is 8.857% and 7.908% higher, respectively. Orders are separate, with no
pooling, stable-gain, HTTP, sustained-throughput or vendor-win claim.
Controller exits and all_workers_exited metadata are accepted. Reserved inner
group cleanup retains TERM/KILL flags; only the outer supervisor is unsignaled.
Do not transfer the isolated parity worker's unsignaled-exit claim to these arms.

Report `products/ferric-opt-v5-v25.2IGl9SCl/prefill32-results/report.json`:
`5ef8c2f2e5a313e16c69401a72f633b9539afcee285d407fe94b65843693bfdc`.
Its `launch-supervisor.json`:
`3443cba3e4030e19900e50d4e0875c14eb1a53a6cd3ade10c33503787a56853f`.

The separate isolated native prerequisite passes 20 two-half/rotation byte
and guard parity cells with clean unsignaled worker/outer exits. It is not
full-model or timing evidence. Its report SHA256 is
`d8a5df743cd9d30ca714d02e8aa7a24774b36067bda80afbbe3c633deb825508`;
supervisor SHA256 is
`3a381a2e184f9f9a1295984821da2f7882b235969732ee453bcf7d3c3397c277`.
Raw native/build evidence stays outside the public seven-file artifact.

The separate opt-in packet-tick diagnostic now has accepted CPU qualification:
all eleven gates and 2,585 passing Rust executions, with 137 existing ignored
executions and one documented library exclusion in each of three feature
suites. Its mixed-generation CPU receipt is
`92606ee41b9bbfda016136ce99d769ad0098004fdd0597952b329c63d33b3a96`.
The R4 measurement harness passes all 31 fixtures, with receipt
`f7e5a0c7953d5bca656abee1dcfeae8256b215db6fa8bf4ef54321a7830c177b`.
Earlier failed preparation, build and fixture records stay historical; they
are not overwritten or relabeled as these accepted successors.

The native diagnostic matches all 128 output IDs and decoded bytes and records
87,711 packets in 135 batches. Independent raw-interval and custody reviews
pass; the complete archive SHA256 is
`aae4f2542809942a30f3fd4087dade8f521a908f7690ad11af8611a4c558c9a7`.
These are uncalibrated packet intervals, not nanoseconds, shader-only time,
additive time shares or a speedup. No new vendor comparison is established.

The first packed gate/up native attempt is an independently audited retained
failure. Its accepted prefix checks all 65,536 packing patterns and 16
finite-parity cells. A foreign load generator restarts during the first timing
cell's warmup validation, before any measured interval. The unchanged guard
fails closed while inspecting a root-owned KFD process, sends TERM only to its
owned runner, reaps it and retains status 125 with `cleanup_ok: false`.
The load generator's supervisor is not signaled. Zero timing intervals are
retained; the prefix is not full-campaign acceptance, numerical-trap
qualification, model inference or performance evidence.

Failed native archive SHA256:
`966737386790c6c996687c2248900c5d9767d8b5e14d3f87f38aa4a13712e47c`.
Its report SHA256 is
`0417f06c0979d632c67115ab82db1f639194b2d4bcb10711b24f74443239995b`;
outer supervisor SHA256 is
`c183366d923aa6a6a741cd422ea1015774084b26e6ce85328294183dbe215f1d`.
The source-only failure audit is
`perf-v7-packed-gate-up-native-failure-ZAKjrVaQ-audit-r1.md` in the private
V6 evidence store's proposals directory. The existing object store is retained
separately. No raw protocols or private source enter the public artifact.

The campaign used compiler/SDK
`413ba987b8878f53e13127fdb53f9e1495ab93b3` and the actual 5e2/dd6 worker.
`perf-v7-latest-main-091-impact-r1.md` observes upstream
`091856733e7053f9f50b7b205f85d4654d467e65` on September 24 UTC, not a new
build or qualification. Compiler/SDK surfaces and fe2o3-kfd changed; the old
nine-runtime-tree equality does not extend to this snapshot. Coordinated
compiler, SDK and native-worker refresh and fresh qualification are required.
The historical binaries are not latest-qualified. Core/runtime ownership
stays in fe2o3; workload kernels and inference stay in Ferric.
No vLLM win is claimed. Defaults and all 33 open M1 gates remain unchanged.

The four changed site files are index.html, validate.mjs, render-validate.mjs
and this README. New exact source assertions and mutation cases bind the
regressions, accepted attribution, interrupted campaign, zero timing,
currentness limits and four existing prefill evidence identities. Exact claim
checks reject altered packing/parity/packet counts, calibrated-tick claims,
clean-stop claims, historical-binary relabeling, premature qualification and
vendor/default/gate promotion. Browser assertions retain all previous views
and add desktop/mobile `packed-gate-up-v7-progress` captures, each containing
the full failure and currentness limits. Main coverage becomes 84 images;
the existing 63 supplemental views stay unchanged, for 147 total.

Root can reuse the mechanics of the existing private
`perf-v7-september23-site-qa-r1` recipe on the designated remote build host:
serial guarded preparation, package/source checks, eight named viewports,
the 320..1440 width sweep, captures and seven-file artifact preparation.
That frozen recipe binds older source and 82-main/145-total images; it must
not run against this update or have its historical receipts overwritten.
A fresh reviewed source freeze and successor bindings must include the two
new capture names, 84 main and 147 total images, with unchanged resource and
publication guards. No new QA infrastructure is required.

This current four-file update is source-only, unvalidated and unpublished.
All new checks and capture assertions are authored but unexecuted. Fresh
remote QA, full actual screenshot review and the separately admitted static
publication still gate deployment. No local build, test, import or browser
execution, remote execution, or publication is claimed here.

## September 22: Two Ordered64 KV-Copy Native Screens

The newest progress paragraph and `#kv-copy-v7-title` retain both independent
accepted native campaigns, J1 and J2, in separate tables. No sample is pooled,
replaced or dropped. J2 has an independent retained-artifact audit. This
four-file proposal is source-only and has not been built, tested, rendered or
deployed; fresh approved remote QA and screenshot review still gate publication.

Qwen3-8B on mi350, TP1/C1, sequential 128-input/128-output requests, context8192,
BF16 checkpoint with explicit FP32 head, prefix caching and speculation off.
Each campaign has four fresh starts in baseline-AB/candidate-AB/candidate-BA/
baseline-BA order. Each start excludes one warmup and measures three requests:
eight starts, 24 measured requests and eight excluded warmups across the two
campaigns. Both retain the same controller, actual dd6 worker built from 5e2
and nine images. Only baseline versus `parallel-c1-v19` KV append differs;
ordered64, parallel-prefill16-v27, split8-v21 and baseline partial GEMV are fixed.

### Campaign J1: first screen

| Native start | Mean TTFT (ms) | Mean TPOT (ms) | Output tokens/s |
| --- | ---: | ---: | ---: |
| Baseline AB | 807.243 | 64.455 | 14.232567 |
| KV copy AB | 809.845 | 52.820 | 17.024785 |
| KV copy BA | 827.309 | 52.755 | 17.003771 |
| Baseline BA | 807.647 | 63.878 | 14.348660 |

### Campaign J2: independent repeat

| Native start | Mean TTFT (ms) | Mean TPOT (ms) | Output tokens/s |
| --- | ---: | ---: | ---: |
| Baseline AB | 944.284 | 77.743 | 11.831300 |
| KV copy AB | 858.399 | 59.668 | 15.171775 |
| KV copy BA | 852.170 | 59.939 | 15.121163 |
| Baseline BA | 892.749 | 71.798 | 12.785176 |

J1 candidate TPOT is 18.051% lower in AB and 17.413% lower in BA; finite-window
ingress output rate is 19.619% and 18.504% higher. TTFT is 0.322% and 2.435%
higher, respectively. The first campaign's BA TTFT regression is retained.

J2 candidate TPOT is 23.250% lower in AB and 16.516% lower in BA; finite-window
ingress output rate is 28.234% and 18.271% higher. TTFT is 9.095% and 4.545%
lower, respectively. Absolute timings drift between campaigns: baseline TPOT
is 63.878 to 64.455 ms in J1 versus 71.798 to 77.743 ms in J2; candidate TPOT
is 52.755 to 52.820 ms in J1 versus 59.668 to 59.939 ms in J2.

These are separately retained orders and rounded descriptive measurements,
not pooled results, confidence bounds, a stable gain, sustained throughput,
GPU time or HTTP latency. No new vendor comparison, vLLM/SGLang win, TP8 or
speculative-decoding result, serving qualification or default change follows.
All 33 M1 gates remain open. The prior HTTP/vLLM tables and evidence are unchanged.

All 32 requests, including eight excluded warmups, match all 4,096 output IDs
and decoded bytes; each retains 135 batches and 87,711 packets. All eight
controller starts exit 0; the worker closure metadata separately confirms
`all_workers_exited`. Reserved inner process groups are absent after cleanup,
with TERM/KILL flags retained as sent. Both outer supervisors exit 0 without
TERM/KILL. Controller exit status is not a per-worker exit-code receipt, and
outer signal-free completion is not signal-free inner teardown.

J1 evidence remains under `products/ferric-opt-v5-v25.QyQQf25M`; J2 is under
`products/ferric-opt-v5-v25.pzJnxacO`. Both use `kv-copy-results/report.json`
and `launch-supervisor.json`. Their exact report/supervisor identities are:

- J1 KV-copy native report SHA-256: `ce74369df5b5ed52721f2ad043d1e0828060494bd7bfebf6da7fa5de9359bb68`.
- J1 KV-copy supervisor SHA-256: `9ed41c54a927adc53426342f3b3563703138a4507c152dc33772903f675caeee`.
- J2 KV-copy native report SHA-256: `55c1455bea17de55ca52aaaaa1ceb5b8bbe8f2d230f3c79ad423c4a0c22d6634`.
- J2 KV-copy supervisor SHA-256: `7a02e34c290c21cd932debb61bd37baf3e3c93bad8e34e69ab14a58e38b05479`.

The retained J2 archive `products/v7-kv-native-r2.tar.gz` has SHA-256
`f7d16934c7de2b346c1a14a1df2ac8664176e51276559510085c8eebd5bfb6c3`.
Raw model/native/build evidence stays outside the public artifact.

Focused source assertions and mutations pin both tables, all eight starts,
24 displayed metrics, per-order percentage changes, all four evidence hashes,
coverage totals, timing drift, exact cleanup claims and historical separation.
The existing HTTP/diagnostic progress paragraph remains
`#september22-http-progress` with its previous checks. Browser assertions check
exact values/claims, evidence disclosure, cell clipping and horizontal scrolling
across the existing viewport matrix. Five existing KV screenshot names are
retained; both complete tables must fit in each table capture, including both
right-scrolled mobile tables, and all four evidence identities must fit in each
evidence capture. These are authored checks, not screenshot or pass claims.

## September 22: Complete HTTP Series and Separate Diagnostic

The separate sections `#matched-ordered64-http-title` and
`#v7-host-diagnostic-title` precede the retained historical campaigns. The
source is [M1 team progress](../docs/M1_TEAM_PROGRESS.md); raw evidence stays
outside the public static artifact.

| Accepted pair | Ferric mean TPOT (ms) | vLLM mean TPOT (ms) | Ferric / vLLM |
| --- | ---: | ---: | ---: |
| 1 | 63.833 | 4.228 | 15.100 |
| 2 | 79.535 | 4.380 | 18.157 |
| 3 | 67.801 | 4.394 | 15.432 |

All six starts and three alternating paired replays pass. Ferric remains
slower in every pair. The 16.229x mean paired ratio is neither pooled latency
nor a confidence bound. This is Qwen3-8B, mi350, TP1/C1, 128/128, context8192,
BF16 checkpoint with explicit FP32 head, prefix caching and speculation off.
The completed series does not repair or relabel the earlier rejected R3 series.
Series SHA256: `579c4abeda3f56ca2a77ec1e1f63d37c0961cec92381f90da6a368d79ab3a3ec`.

The separate V7 single-cold native diagnostic matches all 128 output IDs and
decoded bytes, with 135 batches, 87,711 dispatches and clean exit. Instrumented
TTFT is 818.53 ms and TPOT is 64.64 ms. Mean decode batch wall is 64.626 ms:
worker publication/completion is 58.473 ms (90.48%), controller packing
0.171 ms (0.265%). Worker duration excludes preparation/staging and includes
GPU work, polling and fences; it is not GPU time. Phase scopes overlap, and
all 127 decode batches, including seven slower final batches, remain included.
Report SHA256: `3ddeaf566cffd14d4503a5078de7f87d6a99c0da4227ffd9cf25606f622998fe`.

No speedup, vendor win, stable-tail or sustained-throughput qualification is
claimed. Defaults and all 33 open M1 gates are unchanged. The earlier readiness
dataset retains its own date; this new checkpoint is explicitly September 22.

This four-file update is source-only pending fresh remote QA and publication.
Source assertions and mutation cases bind all table values, interval boundaries,
evidence identities and historical separation. Browser assertions cover exact
rows, scope, evidence disclosure and clipping/scrolling across existing viewports.
The screenshot harness adds seven focused views: comparison and diagnostic on
desktop/mobile, mobile table scrolled right, and evidence on desktop/mobile.
No local test, build, render, deployment or new M1 gate pass is claimed.

## Earlier Ordered64 Native Screen: HTTP Unchanged at That Checkpoint

The completed native AB/BA screen is separate from the matched HTTP scoreboard
and the earlier Partial GEMV R4 no-gain result. The workload is Qwen3-8B on
mi350, TP1/C1, sequential 128-input/128-output requests. Each of four starts
excludes one warmup and measures three requests. AB runs baseline then candidate;
BA reverses that order. Do not pool the orders or use this screen as a vendor win.

| Native start | Mean TTFT (ms) | Mean TPOT (ms) | Output tokens/s |
| --- | ---: | ---: | ---: |
| Baseline AB | 1024.554916 | 94.799826 | 9.797235 |
| Candidate AB | 858.870490 | 70.714876 | 13.007960 |
| Candidate BA | 865.604204 | 70.180564 | 13.089270 |
| Baseline BA | 876.261260 | 80.273610 | 11.561235 |

Candidate/baseline TPOT ratios are 0.745939 in AB and 0.874267 in BA, or
25.4061% and 12.5733% lower. TTFT ratios are 0.838286 and 0.987838;
output-rate ratios are 1.327717 and 1.132169. These are finite-window native
ingress rates, not shader timing, confidence intervals, a stable gain or
sustained throughput. A separate matched HTTP test is justified, but no new
HTTP result, vLLM win, SGLang comparison, TP8 gain or speculative-decoding gain
is established. Defaults and the existing roughly 16-times-vLLM-TPOT matched
HTTP scoreboard remain unchanged. All 33 M1 gates remain open.

Both arms use the same controller, dd6 worker built from 5e2 and eight images.
Only packed16 versus packed64 ordered submission changes. The existing
parallel-prefill16-v27 and split8-v21 paths remain enabled, and partial GEMV is
off in both arms. The baseline profile retains its historical
`prefill16-decode-partial-gemv-v28-live-v1` name despite GEMV being off; the
candidate is `prefill16-decode-ordered64-v29-live-v1`.

The actual CPU qualification is mixed provenance: 17 original R3 one-core
gates, all selected at a001, plus the successful new four-core controller
build at a002. It is not 18 fresh R3 or four-core gates. The controller's
opt-in runtime dependency is pinned to 13e; the worker remains built from
5e2. No failed earlier build receipt is relabeled as a pass.

All 16 requests match all 2,048 reference output IDs and decoded bytes. The
outer supervisor exits 0 without TERM or KILL. All four arms record
`cleanup_ok: true`, owned group absent and exit 0, **with both TERM and KILL
flags retained as sent**. Do not describe arm teardown as signal-free.
Output parity is not independent source-to-binary authentication, protected
proof or serving qualification. The report retains `performance_qualified`,
`http`, `vendor_comparison` and `sustained` as false.

The actual retained evidence is under
`products/ordered64-four-core-native-mi350-r1/ferric-opt-v5-v25.v1fakSWO`:

- `v25-native-results/report.json` SHA-256:
  `cbef8172a332d0d3086889f8ac5cab4b1f913114a4c37535bcf23b062c812015`.
- `launch-supervisor.json` SHA-256:
  `e2e234a94971f996b4f4bb1167ed80664ee1362c45a7d091a816b6d3ce2d7617`.

The new progress and performance sections are
`#ordered64-native-progress` and `#native-ordered64-four-core-results`.
Source checks bind the four ordered rows, all 12 metrics, six ratios, both
evidence identities, workload, mixed provenance, teardown and nonpromotion
claims. Focused mutations reject changed metrics, ratios, hashes, missing or
reordered arms and overstated scope. Render checks require exact values,
text fitting, horizontal scrolling when needed and visible evidence
disclosure. The old `#ordered64-progress` remains explicitly historical and
synthetic-only, with its original 251-packet/594-check boundary.

This update is source-only until a fresh remote source/render/build QA run
accepts these exact files. No new screenshot, rendered-site hash, deployment
receipt or public result is claimed here. The existing QA and publication
records below remain historical; a source-bound successor must retain their
existing coverage and add focused ordered64 captures before publication.
Raw native/build evidence stays outside the public static artifact.

## Partial GEMV R4: No Demonstrated Gain

The source now records the completed native AB/BA screen separately from the
unchanged matched HTTP scoreboard and older native campaigns. All four starts
pass all 2,048 output IDs and decoded bytes. Each start excludes one warmup and
measures three requests; AB runs baseline then candidate, and BA reverses that
order. The same eight images are admitted in both modes. Only one-row
attention-output/down partial GEMV changes; grouping and reduction order remain.

| Native start | Mean TTFT (ms) | Mean TPOT (ms) | Finite-window output tokens/s |
| --- | ---: | ---: | ---: |
| Baseline AB | 877.878138 | 79.613330 | 11.647774 |
| Candidate AB | 945.068094 | 82.110499 | 11.253843 |
| Candidate BA | 1040.976727 | 91.870497 | 10.071339 |
| Baseline BA | 885.450433 | 91.893385 | 10.194000 |

Candidate/baseline TPOT ratios are 1.031366 and 0.999751, while TTFT ratios are
1.076537 and 1.175647. No gain is demonstrated: no HTTP promotion or default
change follows. Orders are not pooled or cherry-picked. These native ingress
measurements are neither vendor results nor sustained throughput. The matched
HTTP pairs still show roughly 16 times vLLM's TPOT; no SGLang, TP8 or speculative
gain is established. All 33 M1 gates remain open.

Report SHA256: `2d5c5846318a93fd888cf35e27cd87f9d53914976bb8371bc1a4e3f92a4d05a4`.
Clean supervisor SHA256: `ff36b8b0e62cd722cfc3c8b2acb091bee8283952d7e859bf082af8af92bf2ae0`.
The retained supervisor completed without signals and no KFD owners remained.
Output parity does not confer source authentication, proof or serving authority.

This four-file source update has not been built, tested, rendered or deployed.
Existing static checks now bind every row, scope statement and digest, with
mutation checks; existing browser checks cover exact cells, disclosure hashes
and table clipping/scrolling. The old V22 HTTP section boundary stops before the
new native section, preserving the historical table's independent assertions.
Use a fresh successor of `site-http-r3-qa-r1` with a newly hashed 34-file source
snapshot and fresh source/review/capture/artifact paths. Preserve its guards,
offline tools, 64 main screenshots and existing 49 supplemental captures; add
focused R4 table, mobile left/right, outcome and evidence-disclosure captures.
Only actual remote QA, retained PNG inspection and the separate publication
helper can establish a published update. No future QA or artifact hash is stated.

## HTTP R3: Two Pairs, Rejected Series

The HTTP scoreboard records accepted individual Pair1 and Pair2 ahead of the
unchanged historical V22 table. The original three-pair campaign is rejected
and incomplete, not an accepted multi-start comparison. No aggregate or
replacement run is presented. This source and the native R6 update still need
fresh remote QA and publication; neither has been locally built or tested.

| Pair and engine | Mean TTFT (ms) | Mean TPOT (ms) | Finite measured-span output tokens/s |
| --- | ---: | ---: | ---: |
| Pair1: Ferric composed | 810.928 | 69.587 | 13.265065 |
| Pair1: vLLM 0.28.0 | 20.148 | 4.367 | 222.524242 |
| Pair2: Ferric composed | 811.404 | 69.186 | 13.335104 |
| Pair2: vLLM 0.28.0 | 18.835 | 4.350 | 223.905213 |

Pair1 order is Ferric then vLLM; Pair2 reverses the order. Each start has ten
excluded warmups, thirty measured 128/128 requests and two untimed diagnostics.
All measured requests pass in the accepted pairs. Ferric TPOT is 15.93x and
15.91x vLLM, and TTFT 40.25x and 43.08x, respectively. These are per-pair ratios,
not pooled observations, confidence intervals, stable-tail estimates or a win.

Pair3 Ferric failed sampled KFD ownership after a foreign process appeared.
Its timing was not admitted and an owned command required forced cleanup;
cleanup completed. Pair3 vLLM was never started. The original manifest still declares all three pairs.
The actual aggregator returned `rejected-series`, `accepted:false`,
`competitiveness_accepted:false` and `framework_win_claim:false`, with the
third summary missing and aggregate spread null. No splice, replacement,
completed-series, equal-p99-SLO, sustained or release qualification is claimed.

TTFT remains client-send to first nonempty text; TPOT is the first-to-last
text span/127, not true token ITL. Rate is 3,840 output tokens divided by the
complete measured span, including gaps/drain and excluding warmups/diagnostics.
For each accepted start Ferric compares all 42 full token-ID/decoded-byte
outputs; vLLM exact IDs are checked only in two untimed diagnostics and its 40
timed requests match text/usage. 500 ms ownership sampling is not continuous
isolation proof. The hardware/profile is Qwen3-8B, mi350 GPU 0, TP1/C1, 8192
context, BF16 decoder/FP32 head, greedy fixed length, cache/speculation off.
This is not a stock BF16-head, TP8, SGLang or speculative-decoding comparison.

Ferric enables packed16-v22, split8-v21 and parallel-prefill16-v27 in the
Composition R6 controller, using the rebuilt 5e2/dd6 worker and seven images.
HTTP instrumentation is off. Model/image receipts are replayed, not independently
rehashed or source-to-binary authenticated. Historical/native differences do
not isolate individual optimization gains. The public page publishes only
curated facts and identities, not private source, raw records or host paths.

| Identity | SHA-256 |
| --- | --- |
| Pair1 replay | `d3fa11cce07e0181e155498b72191a47e88a3240abf8cd840dbd5c075c5e788e` |
| Pair2 replay | `e74214f1923eac3bbda26ae13acdaae0d0a2d1011ec3aa3cfce52338d5da4c97` |
| Rejected series | `962bce221f76fee67bb40a71bd9e0302095eba742f60885ef0b6cd1fc2e03159` |
| Pair3 rejection | `c8a69e6f21cc9fe08d161617ef4a100630bc32674270d6634bb49bf60e4bbe11` |
| Original manifest | `1753e5cb0fd40b7f4b6dcedef54d8dfa506915de26e317515e93c3354a5d17ee` |
| Shared plan | `981c0991d88692b36d53df47866b0292c16f843d281ffbbe74fdfc7c701b5e04` |

Source/render checks pin all 12 displayed values, both orders, all 6 identities,
the rejection/cleanup semantics and qualification boundaries. Mutations are
section-scoped, including the preserved V22 table, so earlier duplicate text
cannot divert a negative test. Remote exhaustive browser QA must inspect
desktop/mobile tables, horizontal scroll, incomplete-series text, timing
semantics and expanded identities. Use fresh QA/artifact/publication names and
actual source hashes; retain prior receipts unchanged. The seven public files
and static-only `pages/prebuilt` workflow policy remain unchanged.

## Composition R6 Native Milestone

This source update adds the accepted four-arm native prefill/decode composition
after the new HTTP R3 pairs and unchanged historical V22 table. Its native
measurements are separate from HTTP. No site QA, artifact, publication,
sustained throughput or vendor-win result is claimed by the update.

| Native arm | Mean TTFT (ms) | Mean TPOT (ms) | Measured-ingress output tokens/s |
| --- | ---: | ---: | ---: |
| Baseline | 2430.855653 | 93.406125 | 8.954908643 |
| Decode only | 2365.162141 | 76.228505 | 10.625352362 |
| Prefill only | 1021.103724 | 93.120622 | 9.962451944 |
| Prefill + decode | 855.944209 | 85.379419 | 10.940545624 |

Relative to the same-run baseline, combined TTFT is 64.7884% lower, TPOT is
8.5933% lower and the finite-window rate is 22.1737% higher. Combined TPOT is
also **12.0046% higher than decode-only in this cohort**. Repeated runs are
needed to determine whether that difference persists; no stable composition
penalty, causal explanation or multiplied historical gain is inferred. The fixed order is baseline, decode-only,
prefill-only, combined; one excluded warmup and three measured requests per
arm. All 16 requests match 2,048 output IDs and decoded bytes. Each arm uses
the same controller, fresh rebuilt 5e2/dd6 worker and seven fixed images,
with instrumentation off. The historical f68 results remain separate.

Report SHA256: `dbf5e34ea6466d84211c85b778fe668d046c5de931491c32b1f4a8118498bcd6`.
Archive SHA256: `eb1f97196a736c2b9512a12c966a8ddb25d5aa3f09153ead95489dcf70ef564c`.
These are public identity strings only; no private source, raw traces, model
files, native archives or local paths are published by this site change.

All arms retain 135 batches/request. Baseline/prefill-only use 83,139 packets;
decode-only/combined use 87,711. Group counts are expected source schedules,
not observed counters. Inner owned cleanup used TERM/KILL, while the outer
supervisor passed cleanly without signals. Native ingress TTFT/TPOT and the
384-token measured-window rates exclude warmups and include gaps; they are not
HTTP or shader timings. V21's unverified OCML premise, name-string bridge
limitations, unchanged defaults and open M1 gates remain explicit.

Model R8 is recorded as separate instrumentation parity: four cold requests,
512 matched output IDs/decoded bytes, 166,278 raw records and 20,478 groups.
The same rebuilt worker serves ordinary and diagnostic controls. Raw ticks
are not nanoseconds, additive shader durations or a performance/proof result.

Fresh remote structural and exhaustive browser QA remain mandatory. The new
checks pin all 12 metrics, four hashes, arm ordering, counter scope, the cohort-only
decode comparison and qualification limits. Section-scoped mutation tests include
earlier duplicate text, avoiding the historical global-replacement failure.
Review desktop/mobile screenshots of the new table, cohort-comparison/scope paragraphs
and expanded evidence, while preserving the HTTP-first captures. Use a fresh
source/QA/artifact campaign and the existing approved static-only pages branch;
do not reuse an older QA receipt, change workflow protections, add dependencies,
build locally or trigger GitHub-hosted build/test.

## Historical V28 Native Screening

This source-only update adds the accepted V28 R4 full-model native A/B separately
from the unchanged historical V22 / vLLM HTTP pair. HTTP R3 now comes first. No new site QA,
artifact, publication or matched vendor result is claimed by this patch.

| Native arm | Mean TTFT (ms) | Mean TPOT (ms) | Measured-ingress output tokens/s |
| --- | ---: | ---: | ---: |
| Baseline | 2312.563649 | 85.149690 | 9.750891 |
| Parallel prefill copy | 808.974061 | 85.154418 | 11.011655 |

TTFT is 65.0183% lower; TPOT is effectively unchanged. All eight requests match
all 1,024 output IDs and decoded bytes. This is one excluded warmup plus three
measured requests per arm, fixed baseline then candidate order, same V28 binary,
historical f68 worker and six images. Both arms retain 135 batches and 83,139
packets per request. V27 substitutes 288 prefill append roots; V22 packing and
V25 split attention are absent. The independent decode-composition gain cannot
be multiplied into these values. The accepted Composition R6 comparison above
measures the combined candidate separately.

The report SHA256 is
`fd58ba7aa82f7bc7a18868e9837bf0de2929910ef519d74b8302a0b2d68ead17`;
the retained native archive is
`039561bd6c792e3e57c7685a65c27e787905aa7f8579850254852f7c17e6fee5`.
Owned inner teardown used TERM/KILL; the outer supervisor exited cleanly without
signals. The earlier staging-path failure remains excluded, not rehabilitated.
Controller-ingress timings are not HTTP or GPU durations. The finite rate uses
384 measured output tokens over the complete ingress window, including gaps and
excluding warmup. No stable gain, confidence interval, tail, latest-runtime,
TP8, speculative, serving or protected qualification follows.
The 5a/d10 SDK name-string bridge is engineering custody, not independent
source-to-binary authentication.

The unchanged historical V22 HTTP result remains 2329.922/83.598 ms TTFT/TPOT for Ferric
and 18.636/4.252 ms for vLLM 0.28.0, with 9.885898/228.904359 output tokens/s.
Its 19.66-fold TPOT and 125.02-fold TTFT gap is not reduced by inference from the
native result. No SGLang comparison has been added.

Exact-value source mutations and rendered table/claim/geometry assertions cover
the new section. Run the existing complete remote structural and exhaustive
Chromium QA, and review new desktop/mobile screenshots of the V28 table,
limitations and expanded evidence disclosure. Preserve the HTTP-first captures.
Use a fresh source/artifact/publication campaign; prior tested snapshots and
receipts are immutable. The seven-file public roster and static-only
`pages/prebuilt` workflow policy remain unchanged. No local or GitHub-hosted
build/test is permitted, and no workflow/environment protection is modified.

## Earlier V22 Matched HTTP Pair

The new source records the accepted V2 HTTP pair and paired replay, separately
from the preserved historical V17 HTTP and V22 native observations:

| Engine | Mean TTFT (ms) | Mean TPOT (ms) | Output tokens/s |
| --- | ---: | ---: | ---: |
| Ferric V22 packed | 2329.922 | 83.598 | 9.885898 |
| vLLM 0.28.0 | 18.636 | 4.252 | 228.904359 |

Ferric remains 19.66 times higher in TPOT and 125.02 times higher in TTFT.
This is not a win. The exact summary SHA256 is
`0283449b779015bffe0ba72954069ffcca7b809ae6aadf91aa24d6b3fb9d1c07`.
The same SSE client, Qwen3-8B BF16 decoder / FP32 head, TP1/C1, context 8192,
128/128 tokens, cache/speculation off, ten warmups, thirty measured requests
and two untimed diagnostics apply. Both engines pass correctness, sampled
ownership and owned cleanup. The common 500 ms census records no observed
contamination; it is not continuous isolation proof. Earlier rejected attempts
remain excluded, not rehabilitated by this later pair.

TTFT is client-send to first nonempty text; TPOT is first-to-last text / 127,
not true token ITL. Rate is 3,840 tokens / full measured span including gaps/drain,
excluding warmup/diagnostics, not sustained goodput. Cohort order is vLLM then
Ferric, one start each; no confidence interval, stable-tail, stock BF16-head,
TP8, speculative-decoding or SGLang claim. Ferric IDs match all 42 requests;
vLLM IDs match two untimed diagnostics and all 40 timed text/usage checks pass.
Retained model/image receipts are replayed, not independently rehashed here.
The controller uses packed16-v22 with historical f68/5ed3840a and five images;
newer source changes do not relabel those products. Cross-campaign differences
against the old 114.619/4.232 ms pair do not isolate packet-packing causality.

The new source and claim/render assertions await fresh remote QA/build using
only cached dependencies/browser and the unchanged OCML-accounting guard.
Expected scope remains eight viewports, widths 320..1440, 64 compact screenshots
and seven public files. No new site build, artifact or publication is claimed
by this source update; old published artifacts and failures remain unchanged.

## September 18 Matched Optimization V5 Publication

The additive V5 content in `index.html` records completion of the first
sequential Ferric V17 combined / vLLM HTTP pair on mi350. Both retained raw
receipts and independent paired replay pass; the replay checker passes eight
CPU tests on mi300x-2. The initial finite-cohort table is:

| Engine | Mean TTFT (ms) | Mean TPOT (ms) | Output tokens/s |
| --- | ---: | ---: | ---: |
| Ferric V17 combined | 2555.823 | 114.619 | 7.479295 |
| vLLM 0.28.0 | 19.909 | 4.232 | 229.358430 |

Ferric has 27.08 times the mean TPOT and 128.37 times the mean TTFT in this cell.
This is a loss, not a performance win. Paired summary SHA256:
`11c761a2e4408fc29a822a29f87ed10995c5799e407fc3233068f197836974a9`.

The common cell is Qwen3-8B, TP1/C1, 128 input and 128 output tokens, context
8192, BF16 decoder with the FP32 output-head profile, speculation and prefix
caching off, the same SSE client, ten excluded warmups, thirty measured requests
and two untimed output diagnostics per engine. The baseline retains private
Ferric `1fc45a52` and runtime `5ed3840a`; subsequent compiler adoption does not
relabel those products or the unchanged kernel images. This is an initial
finite cohort, not sustained serving, TP8, SGLang or a performance-win claim.
Cohorts run vLLM then Ferric with one server start each. TTFT uses client send
to first nonempty text; TPOT divides first-to-last text time by 127, not true
token ITL. Rate is 3,840 tokens over the full measured window including gaps
and drain. Ferric token IDs match all 42 requests; vLLM token IDs match two
untimed diagnostics and its 40 timed requests match decoded text and usage.
Cleanup and GPU idle postflight pass for both. Retained model/image hash
receipts are compared, not a fresh independent model-file rehash. No confidence
interval, stable tail or stock BF16-head comparison is claimed.

The separate instrumented runtime-counter diagnostic R2 is accepted: one
warmup and one diagnostic request, all 256 output IDs matching the unchanged
reference, 270 batches, 166,278 dispatches, completed execution and successful
owned cleanup. Controller and worker inherit CPUs 0-255 and nice0; the two
leader observations are not a scheduling trace or NUMA-placement proof.
The retained report SHA-256 is
`01869cfa796365849ef78d950dfef9e22f217996266abcdb8ea08306575c2642`.

Overlapping worker counters record 23.000106146 seconds in dispatch waits,
1.844776817 seconds in operational currentness and 0.061417113 seconds in full
currentness, against 24.896958489 seconds of worker command time. These scopes
must not be added. Wait includes GPU work, polling and scheduling; it is not
GPU execution time or wholly removable delay. The interval includes warmup
and snapshot overhead, not measured-only TPOT or an HTTP/competitive result.
Operation-level isolation and the V19 native comparison are next; these
counters do not identify currentness alone as the main performance bottleneck.

V19 KV-copy is freshly emitted with fe2o3 `5a503c04`; its device CPU tests pass
on mi300x-2. The emitted ABI reports Wave64, six VGPRs, zero scratch and zero
LDS. Additional validation and native control/candidate correctness remain
pending; no GPU gain or serving qualification is claimed. The runtime
diagnostic's fresh CPU rerun passes all 14 harness tests with owned cleanup
confirmed. Its original 13-pass/one-error run is retained; the fake-controller
fixture collision is corrected and negative assertions reject premature EOF.
Neither V19 nor the instrumented diagnostic is part of the completed HTTP pair.
Historical `data/project.js`, `data/performance.js`, producer attribution,
defaults and all 33 open gates remain unchanged.

This snapshot is published through static-only `pages/prebuilt` commit
`72b767485d3d50e56892471d7e0017538293232a`, deployment `35411581137`.
Remote structural/negative checks, widths 320..1440, eight named viewports and
64 screenshots pass. The seven-file artifact SHA256 is
`370eb3238d4114279118a713bc7991ac0c1f3bd52d6565c1c428fc714d28aca4`.
The fetched live index matches the validated artifact. The workflow only
authenticates and deploys static files; no GitHub build is triggered. Private
implementation ancestry, team tracker and raw evidence stay out of Pages.
This published snapshot predates subsequent V19 native results, tracked in
`docs/M1_MATCHED_OPTIMIZATION_V5.md`; it is not relabelled as that later run.

## Historical September 18 Performance Swarm V4

The retained V4 overview records private Ferric `1fc45a52`, published core runtime
`5ed3840a`, final-pin CPU validation, and a four-arm native TP1 comparison.
All 16 requests match the independent 128-output reference. Independent raw
replay and six mutation/positive tests pass on mi300x-2.

The combined V14/V15 arm records 84.55 ms TPOT and 9.83 tokens/s versus baseline
135.85 ms and 6.43 tokens/s. These are fixed-order controller-ingress diagnostics
with one excluded warmup and three measured requests per arm, not HTTP,
sustained throughput, stable gains or a vendor comparison. Runtime and packing
changes are common to every arm and have no isolated gain measurement.
Native archive SHA256:
`c1cd3b9448e05076cc935801f82ad99b5b48c3d664ad758628860e50b9f953d6`.

The overview's exact text and selected rendered claims are checked, with added
negative mutations for counts, timings, arm order and scope. Historical
`performance.js`, kernel producer attribution, defaults and all 33 open gates
remain unchanged. The new measurement flag applies to this diagnostic, not
serving or protected qualification. Remote structural/negative checks, every
width 320..1440, eight named viewports and 64 screenshots pass. The validated
seven-file artifact (789,721 bytes) is live at
[harsh-nod.github.io/ferric](https://harsh-nod.github.io/ferric/) through
static-only commit `329a86b4bab2b7fc4c8585437f395c295c8cbe44` on
`pages/prebuilt`. Deploy-only workflow `35394680140` succeeds, and all live
files are byte-equal to the artifact. No local or GitHub build/test ran. The
temporary publishing worktree is removed after retaining its Git bundle and
deployment receipt. The older source-bound records below are historical.

## Historical September 16 Combined M5 Host and Proof Draft

This source-only successor starts from Pages source `4c206bfc` and follows
private tracker `e4e55afb`, retained R5 compiler results and completed compiler
publication/adoption receipts. It has not run QA or been
published. Public static-only commit `3e41846e` remains the prior live Pages
checkpoint; no private implementation history is pushed.

Four additive records precede the retained native/numerical checkpoints:

- `compilerAdoption1fad`: private Ferric `a0cc9929` adopts published `1fadb7e0`
  across exactly 80 active pin files. The separate R2 passes all 47 phases,
  with 32 locked graphs, 42 fresh source-gate tests and four actual inventories.
  The first attempt's revision-only assumption fails; its retained archive is
  `0b1ebbcd5b41b56434076a9c0adec81ede14abf29be8f4a89443aa606515b530`.
  The reviewed transition also adds the upstream
  `native_v12_text_descriptor_replay_v1` test target and `dialect-amdgcn`'s
  development dependency on `fe2o3-amd-target`. Runtime inventory and the
  property-binder pin stay unchanged. All 1,148 local source hashes match.
  R2 archive SHA-256 is
  `dacf7b109448bbc90d4d8bd5ea30f1eac7f1b0723630b674bb6f7776e5579fc8`.
  This is source/dependency admission, not engine, proof or GPU validation.
- `combinedM5Host349`: exact combined source `568882e2`, integrated privately
  at `6b5322dc`, passes all 21 host phases on compiler `349b2cd0`: 750 engine
  tests with nine explicit ignores, 22 smoke-bin tests and Python suites of
  4, 23 and 15 tests. Strict Clippy passes only the engine library and exact
  smoke binary. Archive SHA-256 is
  `a8c9d39b1baceaf6c4ba8c776dc36c15c46986cc6a859de59a759d5607e9a066`.
  No real M5 capture or five-position comparison is claimed.
- `catchupProof349`: a fresh 14-phase campaign on the same source and compiler
  rebuilds seven verified dependency exports and verifies the actual production
  catch-up method: one verified, zero errors, with `--no-cheating` and solver
  rlimit 116,015. Archive SHA-256 is
  `6013e24b89fb72ca30b8c654a737a3405c6e5ff59f529bc8a2040b986fedf03c`.
  The nine negative mutations remain on earlier `a9fad31b` / compiler 111 and
  have not rerun on compiler 349. Neither campaign proves whole-engine, queue/owner or
  physical-KV composition.
- `compilerBoundaryR5`: the eight-file core-only WriteOnly output ABI and
  trusted Thread-value correction passes 52 selected tests on tree `a29459fd`
  over `4c298382`. Strict Clippy exits 101 at 17 unchanged parent sites; the
  whole campaign remains 101, not a compiler/proof pass. The positive extraction
  reaches the unavailable functional-refinement runtime, not proof admission.
  R5 archive SHA-256 is
  `ebe08d207b3eb15aa86c099507724e4377c44a4c302aea27c1187b31c9240a10`.
  The unchanged implementation is published as `1fadb7e0` after rebase onto
  `958a80a3`. Separate remote checks preserve 155 normal/build dependency nodes
  and source/protected bytes; the 52 tests are not relabeled as a rerun on the
  rebased tree. `[skip ci]` yields zero GitHub workflow/check runs. Ferric's
  separate dependency-only adoption is recorded above, not inferred from R5.

Earlier source-bound records remain unchanged under explicit historical
headings. In particular, the retained compiler-111 native 32-token observation and
prefill errors are not new M5 or compiler-349 GPU evidence. No reviewed numerical
tolerance, new performance result, serving qualification or gate closure is
claimed; all 33 M1 gates remain open. Performance data, styles, assets, package
pins, index date and the seven-file artifact roster are unchanged.

Exact-value, per-field negative and rendered assertions are updated as source
only; they have not run. After root review, prepare a fresh successor of the
retained R14 bounded mi300x QA/retention controls in a fresh R15 namespace,
keeping the exclusive Pages lock, all resource limits, five phases, eight named viewports, exhaustive widths
320..1440, 64 screenshots and seven-file static artifact. No remote launch or
deployment is approved by this draft. Wait for CPU 0 release from the compiler
tool build before any approved QA launch. No local or GitHub build/test is allowed.
The existing static-only `pages/prebuilt` deployment protections are unchanged.

## Historical September 16 Native, Numerical and Proof Checkpoint

This Pages-only successor starts from source `012d7af8` and follows private
tracker `f39c23d2`, plus the integration lead's terminal negative-proof report.
It has not run QA or been published. Static-only public commit `bcc6d1ba`
remains the live prior checkpoint; no private implementation history is pushed.

Four additive records precede the historical checkpoints:

- `native111`: exact `2bfce38b`/`111722028` completes 32 output tokens over 20
  speculative rounds, including one completed full-K4-acceptance draft catch-up
  and a subsequent round. Normal teardown restores the starting allocation.
  The suffix-filled prompt and excluded prefill anchor remain explicit.
- `prefill111`: a separate canonical prefill comparison matches one token but
  has maximum absolute logit error 0.125 and RMSE 0.03982024072957679. Its maximum
  BF16 ULP distance of 31,373 crosses zero. No reviewed tolerance or numerical
  pass is claimed. The failed V1-schema wrapper and successful V2 successor
  are both retained; the comparator reports are byte-identical.
- `catchupProof111`: separate candidate `a9fad31b` passes 21 selected coordinator
  tests, strict engine all-target Clippy and a focused production-method Verus
  proof, one verified and zero errors. All nine implementation mutations fail
  the selected postconditions, with source restored. This does not prove the
  whole engine, sealed owner mapping, queue completion or physical KV join,
  and is not integrated or validated on compiler 349 at this checkpoint.
- `compiler349`: adoption `803916c4` passes 47 dependency/source-gate phases,
  including 32 locked graphs, 42 source-gate tests and four inventories. No
  engine, Verus or GPU result is attributed to that adoption. Observed upstream
  `45d0bf2e` remains unadopted and unvalidated here.

Exact-value checks, per-field negative mutations and rendered assertions guard
these source identities, metrics and scope limits. Existing performance data,
styles, assets, package pins and the seven-file static artifact are unchanged.
All 33 M1 gates remain open. No new TTFT, TPOT, throughput, competitive ranking
or serving qualification is claimed.

Reuse the retained R12 five-phase QA and retention controls in a fresh R13
namespace on mi300x only, under their existing resource limits and exclusive
Pages lock. Require structural/negative checks, exhaustive widths 320..1440,
eight selected viewports, 64 screenshots, unchanged source/runtime/browser
identities and the exact seven-file artifact. Retain and independently verify
the archive before publication review. Only the approved `pages/prebuilt`
branch may receive validated static bytes and its deploy-only workflow.
No local or GitHub build/test and no remote launch before root review.

## Historical September 16 Published Compiler Adoption

This private Pages-only successor starts from prepared source `e04e2247` and
records exact Ferric adoption commit `2bfce38bd624a3f65b719960ae2ebb5233f772bc`.
It has not run QA or been published; static-only `60f65896` remains live.
The complete 20,487,388-byte adoption archive is independently retained at
SHA-256 `ebbfc150c4811e4f314a81f343977d9eac055e353ffc6c6aa17f2dcec59edd67`;
all 1,145 integrated source file hashes and the exact tracked roster match.

The current adoption record covers published fe2o3 `111722028`: all 47 remote
dependency/source-gate phases pass, including 32 locked/offline graphs across
35 metadata checks, 42 fresh source-gate tests and four actual CLI inventory
derivations. Three compiler-bearing inventories change only their revision;
the runtime inventory is unchanged. This is dependency/source adoption, not
full compiler-tool, engine-suite, Verus, emission or GPU qualification.

Earlier MFMA13 smoke R3/R4 results remain at compiler `8af54567`; the earlier
publication record's pending-adoption status is explicitly historical. No
result is relabeled as a new-engine pass on 111. LegacyScalar12 stays default,
MFMA13 stays opt-in, no new performance result is claimed and all 33 gates stay
open. Use a fresh R12 successor of the same remote QA/static-only publication
flow; preserve the unlaunched R11 controls and earlier evidence.

## Historical September 16 MFMA13 Host Checkpoint

This private Pages-only successor starts from source `38e3feb8` and follows
tracker `4e7e9ef9`. It has not run QA or been published. Static-only public
`60f65896` remains the live prior checkpoint; no implementation ancestry is
published.

Private Ferric `f8eed966` adds explicit leading `--mfma13` selection to the
gfx942 engineering speculative smoke. LegacyScalar12 remains the default;
one-step and resident reports bind their strategy and exact 12/13 count to the
admitted artifact. The strict selected opener has no fallback.

Smoke R3 passes 147 tests with two existing hardware ignores, then retains a
source-policy failure. The test-only follow-up passes 38 policies, four engine
regressions, three GEMM regressions and strict all-target adapter Clippy. These
are separate source-bound cohorts, not one green campaign. Earlier parser,
policy and wrapper failures remain retained.

Ferric remains pinned to `8af54567`. Separately, public fe2o3 `111722028`, rebased
onto `d6471109`, passes 152 tests with ten existing ignores and scoped strict
Clippy for the corrected verifier and four fixture targets. Post-push checks
find zero GitHub workflow/check runs; all builds and tests ran on mi300x.
This core-only correction is not full compiler qualification or a Ferric repin.

No new GPU result, performance measurement, serving qualification or M1 gate
closure is claimed. Current emission and the rebuilt GPU retry remain pending;
all 33 gates stay open. Earlier R9 proof and R6 native records retain their
original source attribution. Performance data, styles, assets, package pins
and the static-only publication policy are unchanged.

Reuse the prior R10 remote QA/retention flow in a fresh R11 namespace. Require
exhaustive browser checks and the closed seven-file prebuilt artifact before
root review and publication to the user-approved `pages/prebuilt` branch.
No local or GitHub build/test, and no publication before successful remote QA.

## Historical September 16 Proof and Native Checkpoint

This private Pages-only draft follows tracker `06260001`, based on prior Pages
source `3c7a4023`, and the separately reported compiler fetch `8af54567`.
It has not run Pages QA or been published. Static-only `58ebfe67` remains the
published R4 checkpoint; no private implementation ancestry is published.

R9 on exact proof source `cde4faaa` passes all 14 selected executable bodies:
nine page-return helpers, four retired-lease accessors and the global-index
core. Each has successful nonzero solver work. Seven genuine dependency
exports and prior evidence remain unchanged. This is not outer pool/queue
composition, negative-mutation coverage or proof of the maintenance checker.

Native R6 on frozen `00ac6a22/4f6` exits 134 at `step draft catchup custody`,
before submission of the 425 maintenance packets. No successful maintenance
or complete 32-token continuation is inferred. The selection fix `6c34cbe1`
preserves completion and ownership guards. Its compiler55-pinned host campaign
passes 17 phases and 26 distinct tests across 27 executions, including nine
K4/K8/K16 selection cases, with no failures or ignores. This is focused host
evidence, not a full-engine, strict-Clippy, Verus or native pass.

The fixed native executable still needs rebuilding and a GPU retry. Latest
compiler adoption remains pending: the fetched `8af54567`, and earlier `f0b`,
do not replace the validated `55d9bfe5` pin in these results. MFMA stays opt-in,
with `LegacyScalar12` the default. No new performance or serving qualification
is claimed, and all 33 M1 gates remain open.

The additive `integration.latestProgress` record has exact value, claim and
negative checks. Older R4 and prior records retain their original identities
and dated pending states under an explicit historical heading. Styling,
assets, performance values, package pins and deployment policy are unchanged.

Next: use a fresh owned mi300x CPU-only stage and the existing pinned browser
cache, run `npm ci` and `FERRIC_EXHAUSTIVE_WIDTHS=1 npm test`, then
`npm run stage -- /private/owned/pages-artifact`. Retain the source, complete raw
QA logs, viewport screenshots and exact seven-file artifact before review.
No build, test, browser run or publication is authorized on this local machine
or GitHub. Only the reviewed prebuilt static files may later enter the isolated
`pages/prebuilt` publication branch through the existing deploy-only workflow;
private implementation ancestry, docs, test harness and raw evidence stay private.

## Historical September 15 Sustained-Run Follow-up

The following records retain their earlier source identities and then-pending
states. They do not supersede the September 16 checkpoint above.

Private integration `9e10cc956e031adc828646272c033aea5b473901`, tree
`1c3efda960ee4ed947d8a3131582dd6fc5525ea0`, updates 80 dependency, lock, policy
and inventory files to published compiler `e3c359fb`. The dependency-only
overlay on `58833759` passes all 42 mi300x phases, including 32 locked/offline
graphs, a fresh source-gate build and 38 source-gate tests. Package versions
are unchanged; the M0 property binder keeps its separate `e527` pin.
The complete 20,176,971-byte archive is retained and independently audited:
`53d27228aa90f2bd80efeaa79cc23f28746b7a5b5a6b66180011c1412a6ec22f`.
The reviewed combined inventory has SHA-256
`9c901826689badc35c7c501f7342602753e85dd3d88b6fb78e4864fdc59fd68d`.

Those receipts validate the `588` dependency overlay, not the newer Rust caller
or a full `9e` engine run. Integration also includes `8af1e52b`'s contracted
batch ledger consumer loop; its host, Verus and native validation have not run.
Earlier source47 host/proof receipts and the frozen `00ac/4f6` native executable
retain their original identities. No performance result or M1 gate is upgraded.

The current runtime executable is exact `00ac6a22/4f6`, not a newer documentation
commit. It passes six scoped host phases and an optimized build; its five
diagnostic tests and 37 adapter policies are distinct from the predecessor
ba6 full-engine result of 720 passes and nine existing ignores. The earlier
c5 source coverage is retained as historical; successor inventory remains
pending and no verified label is upgraded.

The 32-token attempts remain separate negatives: c5 R3 fails page-generation
bookkeeping, cef R4 aborts at maintenance publication, and diagnostic 00ac R5
stops before inference on a disappearing KFD PID's process-group observation.
R5's PID ownership is unknown, not evidence of positive foreign GPU use. Its
archive `af4f5d9c0c6b44f302f004a72baf49dd4db37c2eefd839dfbb0052b60359d83a`
and all 27 native payloads are retained. No complete JSON report means no
inferred rounds, tokens, full acceptance or maintenance success. Fresh versus
immediate GPU facts remain separate, with cleanup readiness false. The narrowed
selected-GPU ownership guard passes 25 actual remote mock fixtures, not GPU or
native validation. Its archive SHA-256 is
`d248b2bf6ff43146b917d29741bfa47e86545ffb4329ac494ff12a7af061e90f`.
Native R6 has not run.

Test-only routing candidate `da3b3a1d` adds actual 425-row maintenance and saved
2,242-row speculative routing coverage, using inert tags without native
allocation witnesses. Actual remote formatting, metadata, library check and
the exact one routing test pass; strict Clippy fails on two test-only lints.
Successor `45a211e5` passes all five remote host phases: formatting, exact 4f6
metadata, engine library check, the exact one routing test and strict all-target
Clippy. Before/after source receipts match and the frozen native binary is
unchanged. The independently hash-checked archive SHA-256 is
`142083a5b330e88f7f66a6a3f15a29355a8b6a5e97c6922d19addf18cd5c0f4d`.
Neither candidate changes production behavior or relabels the frozen
`00ac/4f6` native binary; no new 32-token native result is claimed.

Earlier helper integration `8fa40b3a` passes eight remote host phases, both exact
regressions, 723 engine tests with nine ignored, and strict engine/spec Clippy.
The source and frozen native binary checks pass; no native artifact is relabeled.

Published compiler `e3c359fb` is rebased onto observed upstream `2585ce64`,
with all twelve patches unchanged. R12 completes all 19 mi300x phases: 1,030
Rust tests passed, zero failed and one existing gfx1151 ignore, plus nine Python
dependency tests and the CI dispatch harness. The initial inherited-environment
CI harness failure after six passing phases is retained at SHA-256
`47740391eff1045047f7fbd243ee6159c312b2e243edd6e8a309f1f24ac63993`.
Only its `LD_LIBRARY_PATH` environment was corrected for continuation. The R2
retainer rejected paired `--nocapture` framing; the strict R3 retainer passes
without rerunning tests. The complete 13,891,856-byte R12 archive has SHA-256
`e4334c6b7db87c97b494d8c70b0d1d7a8c86c6201b5780f5bbf9e916ac31ba48`;
it is retained locally and independently SHA-256 checked. Independent archive
review confirms the source tar commit and current-source Cargo rebuild/reuse
closure. A normal non-force main push completed. At `2026-09-15T23:48:06Z`,
GitHub main equals `e3c359fb1bf39ec21c4239ac37ce59b7a3a51db9`, with zero Actions
runs and zero check runs for that exact commit. No workflow or protection
settings were changed. No current release tools, emission, strict compiler
Clippy or native GPU execution is claimed. Ferric's separately validated
dependency refresh is described above.
Its tip contains `[skip ci]` under explicit user
approval, with all builds remaining on mi300x. The prepared 9c2e R11 controls
were never launched. No earlier host, tool or emission result is relabeled.
Previous private compiler `12845295` is rebased onto observed upstream `48323569`,
with all twelve local patches unchanged. All 14 R10 delta host phases pass,
including actual source-bound artifact freshness and paired BF16 export checks.
The actual wrapper-PATH LLVM 18.1.3 assembler identity matches before and after.
Full source and raw results are retained locally and independently SHA-256
checked at `b3cfc86c7c0c6d7c858b4751d8d0a9197a77cb663c67a46cd028f6ce3e9cfaaa`.
This does not claim new 128 release tools, emission, strict compiler Clippy,
GPU execution, a compiler push or a Ferric dependency repin.
Earlier tested private compiler `86ccc2a5` on observed base `bd4d5f42` passes all 23 R9
delta host phases, including fresh artifact checks and paired BF16 export.
Unchanged R9 suites were not rerun in R10. New 86 source-bound tools
and emission are not claimed. Frozen `e0d108b2` separately passes 42 host phases,
four tool phases and the thirteen-root `ce2` emission/inspection campaign.
The 113,192-byte image has 13 entry/descriptor pairs, exact replay and inspected
BF16 MFMA instructions. Its retained archive SHA-256 is
`f05db35b45eccd4de06653c97b6a277988447f1f66c8cf6218662787d88fb049`.
No GPU ran and publication/load/launch grants remain false. These tools and
emission results are not relabeled as 86, 128 or e3c. The seventeen-phase host result
remains attached to predecessor `d3a52cd2` on `bf0f4841`.

The historical root-free observation of 19,279,236 KiB was below the unchanged
23,068,672 KiB floor; it is not a current admission reading. Reviewed archive
cleanup reclaimed 823,808 KiB without restoring admission. The later
independently reviewed combined cleanup removed 37,787 listed files and
reclaimed 3,098,548 KiB, with receipt SHA-256
`12862dc7365442b9e04ad6a15be556507ae3857da10a14079fa07502d6f14952`.
Additional cleanup restored resources and remote host/proof work resumed.
The latest recorded mi300x GPU census reports all eight GPUs at 100% busy, not idle
admission.
Separate read-only `mi350-2` inventory finds one physical MI350X/gfx950 GPU,
not eight; additional render nodes are auxiliary devices. Foreign KFD queues
prevent idle admission despite zero utilization at that observation. Its kernel
`5.18.2-mi300-build-140423-ubuntu-22.04+` is not admitted by the current gfx950
policy; a platform/UAPI review is needed, not just an accepted-string change.
Exact model/artifact inputs were not found at the known paths. No artifact was
retargeted or launched there.
Root free space fluctuates on the shared host, so every new launch still
requires fresh resource admission; no floor is lowered and no native retry
success is claimed.

The 5602 compiler fixtures emit two inspected MFMA images, separate from the
later e0/ce2 aggregate and not a GPU numerical result. The d3 predecessor passed seventeen
host phases, but its emitter rebuild stopped at the unchanged root-disk floor.
The earlier 5a689 commit-proof failure and 339c resource stop remain retained;
the actual R12 commit proof at 339c subsequently fails. Trigger-only successor
`bd8e22f7` passes the selected commit proof with pinned Verus `b677dd5`, two
solver queries and zero errors. Its commit-proof archive SHA-256 is
`2e4c580164958d83fbe92ebdb5ba6fe11f60c7ae3705a8da0fae74ab36f84e7d`.
All six scoped proof bodies pass: metadata preflight, commit,
table_contains_index, same_role, RequestId slot and generation, with checked
query counts 2/2/2/1/1/1, nine total. Actual-runtime mutation campaign R16 exits
0 with all 24 negative receipts accepted; the complete positive/negative raw
archive is retained and independently SHA-256 checked:
`b3c0760fcc2480913b2fdc72cc2167de68f4049dc06f1482974d38a5d94524e8`.
This is not whole-crate or hardware proof; the full retirement transition and
native completion/pool composition remain unproved.
The later selected-page lemma and six actual engine helper bodies at `457ca0aa`
pass pinned Verus with one query per body and zero errors. Their contracts are
integrated at `8fa40b3a`; helper negative mutations and caller/whole-roster/native
composition remain pending. Global-index candidate `79f74a3e` fails R5 parsing
with exit101 and zero verified queries. Its failed archive is retained.
Parentheses-only successor `74c52a73` passes R6 for the actual
`global_page_index_core` exec body: one query, zero errors, rlimit154038.
Source/dependency/verifier checks and prior-evidence preservation pass. Its exact
code is integrated at `2aae5839`; its R1 host attempt exits 1 at formatting only,
for line wraps in two new `assert_eq!` statements. All five source after-checks
pass, but no completed host validation is claimed. Test-formatting-only successor
`21818e2a` preserves production bytes equal to proof source `74c52a73`; its host
R2 passes all eight phases, including both exact regressions, 724 engine tests
with nine ignored and strict all-target engine/spec Clippy. All six source
before/after checks match; native/proof identities stay unchanged. Its full
evidence archive is retained and independently SHA-256 checked at
`4dc705e003d66605aeb10d823487e90b4735bb8abbcc63409821cf406af70427`.
The earlier `8fa40b3a` 723-test cohort is not relabeled.
The full R6 archive is retained and independently SHA-256 checked:
`311aea52da61ba959640db89afb08a1b00da22f8b91559ef6352e92ca52b8317`.
R7 completes all seven actual index-body negatives with selected exec
postcondition failures and nonzero SMT work. Parser, compiler, translation and
resource failures are not accepted. The full raw archive is retained and
independently SHA-256 checked at
`6aed4783947f5556a67f414b7eb563afe366819dc421063022b3e5c7a3d56b28`.

Separate Contracted ledger candidate `efa069bd` passes formatting, metadata,
engine check, both regressions and 726 engine tests with nine ignored, then
fails strict engine Clippy on `manual_map`; spec Clippy was not reached.
The full failure evidence and test executable are retained at checked SHA-256
`5713237da7943871551c8c1894e820cd45f4ae0170194e445b5ad0b36f64bf99`.
Successor `47d07cee` preserves the contracts and changes only the optional state
read to an explicitly bounds-checked slice index. Equivalent behavior and error
ordering are source-reviewed, and remote formatting matches. Its original R2
host run is recovered without a rerun and exits 0. All eight host phases pass,
including both targeted tests, 726 engine tests with nine ignored and strict
engine/spec Clippy. All 16 source checks pass; source-after equality matches and
all owned process groups are absent. The earlier SSH transport exit 255 is
separate from the actual host run's exit 0. Full host-only evidence is retained
and independently SHA-256 checked at
`4e49bda4a4fd73e09aec4feedafb57a152bbb567f3cf8c2ce477d5d2706276fd`
(77,205,056 bytes). R9 completes all nine selected actual helper proofs: each
reports one verified exec query, zero errors and nonzero SMT work. All 284
dependency artifacts, source, verifier closure and predecessor evidence remain
exact. Full R9 proof evidence is retained and independently SHA-256 checked at
`9edd3b9ecbdb8587a22f2ba26a673b040e624514d9d6c4f56e679baab1aacab8`.
R10 completes all six actual ledger-body negative checks and exact restorations.
Final source/dependency/configuration/closure and predecessor preservation
checks pass. Its 197,607,173-byte full archive is downloaded and SHA-256 checked at
`6b7e66b6501ee62d3700835d2585cfcaf3928273e249daf03ae1dd2bec0c8170`.
Independent retained-evidence review is complete with no blockers. The verified
ledger helpers are integrated at `66e3299300e17a644126458a29e9c0b1a5766ba4`.
Its `device_cache.rs` is byte-identical to host/proof source `47d07cee`, and the
only source-tree difference is the tracker. All host and proof receipts remain
attributed to source47; no integrated-source rerun is claimed.
Wrapper completion custody and
cross-call ticket/ledger continuity remain outside these helper contracts.
All 33 M1 gates remain open and all performance JSON is byte-unchanged.

This private source worktree starts at retained Pages source `4e16c945`.
No private implementation ancestry or source may enter the publication repo.
After fresh mi300x QA and artifact review, only the seven admitted static files
and the deploy-only workflow may enter the isolated `pages/prebuilt` branch.
GitHub may upload/deploy those prebuilt bytes, but must not build or test them.
The approved branch allowance and existing main/environment protections remain
unchanged. No local build, new browser result or deployment is claimed here.
This source snapshot is frozen for root review. No Pages QA or publication has
run for it; the host-only archive does not substitute for a proof receipt.

## Earlier September 15 Resident And Host Checkpoint

This public-only checkpoint adds the exact c5/4f6 native guard exit 126, empty
output and unresolved detected-PID attribution. It records return to the exact
GPU baseline and evidence-checked owned cleanup without turning the attempt
into repeated-round, catch-up, numerical or performance success.

The separate optimized c5/4f6 build passes with an independently retained
14,133,320-byte release executable. GPU access was disabled during that build.
Its separate native GPU4 attempt exits 0 with five K4 rounds and eight tokens,
acceptance counts [0, 0, 1, 2, 0], hardware completion and native queue teardown.
GPU memory returns to the exact baseline. No full-acceptance maintenance/restore
was observed. The five raw prompt tokens are actively EOT-filled to 128, not
attention-mask padded; prefill anchor 12 is excluded from the output. This is
not ordinary raw-prompt serving, numerical qualification or a performance gain.
The exact native archive SHA-256 is
`c46c0023872f404ddda8140f9162fa2d36785618916266ad41d0240234908870`.
The successful release attempt does not relabel the earlier debug failure.

The current c5 coverage is separate from dfb's 707 engine tests and the unchanged
942 adapter's 109 library, 12 CLI and 37 policy tests. No verified label is
upgraded. MFMA full host results retain ce2 plus the unpublished b86 overlay;
scoped strict Clippy and eight focused catalog tests retain successor 1fc.
Unpublished compiler 5602 has actual paired storage exports and release tools,
but no HSACO/GPU result. The older 0fe, 4f6 and ccfd images keep their identities.

All 33 M1 gates remain open. The seven-case tooling and single-case numerical
record are unchanged, as is all historical performance JSON. The implementation
remains private; only `site/` changes descend from public Pages commit 5fe89b3e.

## September 14 Seven-Case And Host Progress Checkpoint

Seven-case R29 tooling is integrated at `10cc6587`, byte-identical to the
validated `940d37e2` implementation. It validates the complete seven-kind,
52-row suite, ordered lane identities and actual c8192 execution bindings.
All 24 comparison tests, 15 engineering and 23 legacy reference tests,
capture-protocol checks, policy checks and scoped strict Clippy pass. The
gate archive SHA-256 is
`a05d9b1a63950a01be2334f087b22c7477ccc1f259d75df8d449cbf09bc064dc`.
Actual GPU comparison coverage remains **1 of 7**, at the retained ccfd
prefill source below. The other six cases have not run; host fixtures are
not substituted for hardware evidence or numerical tolerance acceptance.

The September 14 checkpoint had exact `0536` engine 706/9 and `dd0` adapter
12 tests, with the `942` adapter retry stopped by the workspace cap. These
historical records remain separate from the later dfb/942 host passes and c5
native observation above; original import and resource failures are retained.

The private Ferric MFMA matrix-kernel/catalog candidate `8b74` passes 81 host
library tests and strict library/test Clippy. Device compilation, GPU execution
and performance remain untested. Latest checked compiler `0fe50455` passes
42 lineage, 84 MIR, 1,029 Pliron and 543 backend tests, associated integration/doc
suites and 37 exact `f9` aggregate host tests. Pristine release tools now emit
and exactly replay its 103,616-byte gfx942 image, with 12 kernel entries and
12 descriptors. The HSACO SHA-256 is
`e17bf955d3de70c44d721cb798785f539915cb003c7b046efd2cce787e2df7c3`;
evidence archive SHA-256 is
`539970f202a048cc5cd3664b6fadde528da13fc7e99249bdac7bae82ffe3e45f`.
This differs from 4f6 and has **no GPU validation**. The unchanged baseline
dependency Clippy failure and original pre-emission resource stop are retained
separately. Authority is none; publication/load/launch grants remain false,
and installed workers keep their actual older producer identities.

All 33 M1 gates remain open. No new TTFT, TPOT, throughput, accepted matched
SGLang result or reviewed numerical tolerance is added. `performance.js` is
byte-unchanged. Implementation sources remain private; this public checkpoint
contains only site data, documentation, rendering and claim checks.

### Retained Selected Numerical And Emission Checkpoint

Exact `4ac1250/ccfd` capture, independent canonical Qwen3-8B BF16/SDPA reference,
and comparison each exit 0 for `prefill-s1-t128.001`. This is one generated
128-token input, not the earlier English or speculative padded prompt. All
151,936 logits are finite; both select token 198 with identical top-10 token
ordering. The rows are **not byte-identical**: maximum BF16 ULP error 31,121,
maximum absolute error 0.0703125, RMSE 0.01524191977257529, cosine similarity
0.9999202347605426, 20,968 exact BF16 matches and 619 opposite nonzero signs.
The reference repeats twice byte-identically within one model load, not two
fresh launches. This is one of seven planned R29 cases, with no reviewed
tolerance acceptance, full R29 comparison, qualification or benchmark result.
The retained archive SHA-256 is
`816f162e5a2d3229239dc305e8658f8c570a269b44a077787d916030ec60335c`.

Separately, emitted fe2o3 `4f6f65ce` and private Ferric `8113e231`
pass the changed kernel-ir/analysis full suites, 543 backend tests and all
37 aggregate host tests. Pristine release tools emit/replay the canonical
gfx942:xnack- V6 image. Structural ELF inspection confirms 12 kernel entries
and 12 matching descriptors, not policy descriptor-table order. The image is
103,616 bytes, SHA-256
`6f77d6813e6a2c9fd20c8b50eaffe0feb4435f00ac65192e9574a3637b6e5284`.
It differs from ccfd despite unchanged kernel bodies. The later c5 observation
above exercises this retained image on a GPU, without numerical qualification
or changing its 8113 emission producer. Engineering authority is none; all
publication/load/launch grants remain false. Installed workers retain their
actual older producer identities. The emission evidence SHA-256 is
`6208de3c1329e000c5393739b600d6e04b49b4b20dff4fb8dfc5443a8e65d3cb`.

All 33 M1 gates remain open. No TTFT, TPOT or throughput values are added;
`performance.js` is unchanged. Latest combined host validation is separate from
the historical bb2/ccfd host and d094 coverage records retained below.

### Retained Native And Compiler Checkpoint

Exact `bb2/ccfd` target-only execution on mi300x completes the same five-token
raw prompt with four IDs `[12095,13,576,6722]`, text ` Paris. The capital`,
hardware completion and exit 0. These match the first four IDs from the
independently frozen historical BF16/SDPA reference. This one-prompt token-prefix
spot check is not an R29 logit comparison, general numerical validation,
speculative catch-up, R33 qualification or serving performance. Its retained
archive SHA-256 is `2a9b13d81098a878ad1f299a7f4f999c368c16aca1a7f906cd9dc283f1e9dffc`.

The immediate archived after-state still reports 56,838,090,752 VRAM bytes,
zero selected KFD-process memory and busy 0. A later independent direct sysfs
check observes return to the exact 298,647,552-byte baseline and busy 0. This
delayed observation is separate; the original archive is not rewritten. No
device reset or foreign-process termination was performed.

The separate `6651f2d/c6b4050` target-only native smoke on mi300x passes with
the earlier `f17/c6` image: prompt `The capital of France is` (five token IDs)
produces one token, ID 12095, text ` Paris`. Hardware completion is observed,
the process exits 0 and selected-device memory returns to baseline. Its archive
SHA-256 is `0dc8ca2a724bde837750bae565a4224feae03e6f88fde50913ed35c58199999d`.
Authority is none; benchmark comparability, worker-v3 authentication,
compiler-origin authentication and current-publication selection remain false.
This is not general numerical parity, speculative catch-up, serving,
performance or M1 qualification. Raw controller offsets are excluded from
TTFT/TPOT and the performance data.

Separately, published fe2o3 `ccfd43d6` passes 1,021 Pliron library tests,
58 integration tests, 13 doctests and 543 backend tests. Exact private Ferric
`bb2b012` passes 37 aggregate host tests. Pristine release compiler tools emit
the canonical `gfx942:xnack-` code-object V6 engineering HSACO with exact replay.
Independent structured ELF inspection confirms 12 kernel entries and 12 matching
descriptors. The image is 103,872 bytes, SHA-256
`46335b09a921b33ed66392e415ce3d2348dd63042242c0387d5a58362ba3c84c`.
These are structural symbol-set checks, not descriptor-table ordering, numerical
execution or native qualification. Engineering authority is none; publication,
load and launch grants remain false.
This newer image differs from c6 and has its own separate four-token native
observation. The c6 result is not relabeled as latest-compiler hardware evidence.

The isolated same-source `829/852` comparison resolves the earlier uncertainty:
49 assertions, one false-to-true proof gain, zero true-to-false losses. The paged
`committed_tokens + query_token` obligation at `bb97`, line 373 was already
unproved, not a regression introduced by `852`. The new compiler bounds unsigned
literal subtraction only below the exact authenticated checked-success edge;
all producer assertions and limits remain mandatory. The complete aggregate now
passes. Earlier resource stops and rejected extractions remain separately retained.

Exact `bb2/ccfd` passes all 71 host and metadata phases. Spec passes 123 tests;
engine passes 697 library tests (nine ignored), binary suites 11/2/75/2 (two
ignored), and 171 doctests. Checker passes 81 library, 27 integration (three
ignored), and six doc tests with normal host features `[]`. Adapter passes
109 tests (one ignored), 37 policies and seven host numerical tests; owner
passes 16 tests and three policies. All five scoped strict Clippy checks pass.
All 32 locked graphs, 38 source-gate tests, 31 verifier policies, six source-pin
policies and three dependency inventory comparisons pass. The target-smoke,
speculative-smoke and R29-capture release binaries are built and source-bound,
not GPU-qualified by those builds.

Final `d094/ccfd` source coverage also matches the committed manifest exactly:
173 modules, 8,435 bodies, 723 verified labels and 7,712 unverified identities.
All 722 prior verified records are unchanged. The added helper label is backed
by its scoped Verus run with six executed callees and seven rejected executable
mutations, not whole-crate or physical catch-up verification. The engine
preflight and full physical catch-up chain remain unverified. Host tests and
binaries remain at bb2; d094 adds documentation and reviewed inventories only.

No new TTFT/TPOT, throughput, general numerical parity or serving qualification
is claimed. All 33 M1 gates remain open, and `performance.js` remains byte-identical.

### Retained Earlier Host Cohorts

Authenticated resident K4/K8/K16 catch-up is integrated. It consumes the missing
last accepted draft candidate, advances draft KV by one, emits zero served
tokens, and restores the original speculative queue shape. Twenty focused
host tests passed on the earlier `829` cohort. The exact `d55/852` engine gate
passes 697 library tests (nine ignored), binary suites of 11, 2, 75 and 2 tests
(two ignored in the 75-test suite), 171 doctests, and strict engine Clippy.
The current adapter passes 109 tests (one ignored) and 37 policies; the owner
passes 16 tests and three policies. Strict adapter and owner Clippy also passes.
The exact `d55/852` release generator also passes `--check`: generated runner
source matches the current renderer, not a produced native runner or image.
No warmed allocation-free catch-up or native serving qualification is claimed.

Full canonical target and draft prepack and reopen verification both pass on
the exact `bed11/829` CLI. These producer results are not relabeled as `852`;
they are model-data checks, not kernel execution, model parity or latency data.
Actual source coverage matches 173 modules and 8,433 executable identities:
181 new pending-Verus identities, 7,711 unverified identities in total, and
722 unchanged existing verified labels. This does not establish new physical
proofs. The separate `852` gate passes 32 locked graphs, 38 source-gate tests,
31 verifier policies, six source-pin policies and three exact dependency
inventory comparisons. All 33 M1 gates remain open; performance data is
unchanged.

### Earlier Diagnostic And Getter Checkpoints

The following cohorts retain their original source identities and historical
pending states. They are not relabeled by the current checkpoint above.

The follow-up records published fe2o3 diagnostic `ae441`, rebased onto `836afb284`,
and its separate backend/lineage/analysis/IR host suites. Actual aggregate
extraction attributed the earlier divisor failure to paged GQA coordinates.
Private Ferric `6da025e` uses literal coordinates and passes 35 host tests,
including actual AST equivalence for 14 profiles. Actual gfx942 extraction then
passes the divisor check but rejects an overflow assertion at `bb81`, source
fingerprint `09ab2715abb3`, line 378. Optimized MIR attributes the guarded
`query_position + 1`; this is not a verified compiler fix or a produced image.

Published fe2o3 `b750` exposes the generation retained by the owned completed-read
request. Its 43 service-host, 45 service-qualification and three pure KFD tests
pass; private Ferric `7c3f9729` pins it, with combined engine validation pending.
Pinned Verus verifies only the selected `kv_physical` module at `bd2/e6`:
4 verified, 0 errors, exact 190-file distribution closure before and after.
That source-settlement result is not whole-crate or physical catch-up proof.
Catch-up composition, bindings, coordinator, typed KV, maintenance readback and
preclock storage are implemented locally; resident dispatch is still under
integration. Continuing full acceptance remains fail-closed. No warmed catch-up
allocation claim, image, GPU run or performance observation follows.

The earlier checkpoint below remains source-bound, not revalidated by these
newer results:

The additive `residentCheckpoint` distinguishes private authenticated singleton
K4/K8/K16 selection from native qualification. It records separate host snapshots,
the real owner-builder identity repair, and the public fe2o3 resource-accounting
fix. The 13 focused compiler regressions are a subset of the 1,006 passing Pliron
tests, not additional tests. The old dc8 resource-limit failure is retained
separately. At eaa, CLI/backend builds and 33 gfx942 aggregate host tests pass,
but actual aggregate emission rejects a deterministic-control divisor without
static nonzero evidence. Kernel attribution is unknown. Exit 1 produces no
image, replay or GPU result.
Newer upstream `e6cfa668` was observed afterward; migration and revalidation are
pending. All compiler build, test and emission results stay bound to `eaa057ac`.

All 33 M1 gates remain open. There is no new native
K8/K16 result, GPU measurement or TTFT/TPOT/throughput claim. Every historical
project object, including the prior V15 checkpoint, and all of `performance.js`
remain unchanged. `validate-resident-checkpoint.mjs` rejects scope, source, count,
catch-up and qualification mutations. The earlier live-C1 evidence validator
checks this new object before removing it for the original historical comparison.

## September 12 Matched Cell

The new `matched128` section is pending root review and Pages-only publication.
It records the first admitted Ferric and vLLM Qwen3-8B cell: one MI350X GPU, TP1,
BF16 weights/decoder with an explicitly selected FP32 output head, context 8192,
128 input/output tokens, concurrency one, ten warmups and thirty measured
requests per engine. Speculation and prefix caching are off. Ferric is
substantially slower. The table is descriptive, single-start and closed-loop;
it does not qualify sustained serving, stock defaults, a repeated primary suite,
confidence intervals, token ITL or a competitive win. SGLang startup failures
remain without a metric, never zero throughput. The output-head operand policy
is explicit: Ferric's MFMA head and vLLM use BF16 operands with FP32
accumulation/output, while the independent reference converts head operands to
FP32. These implementation differences were declared before engine outputs.

`latestReadiness` and `pagedDraftReference` separately record the independent
BF16/SDPA-body FP32-head draft reference, one of eight planned native paged
canary cases, the private proposal candidate's passing host gate, and the exact
82703c6 combined host gate (568 invocations, 22 image skips, seven doctests and
31 Python tests). They do not claim a complete native matrix, new Verus proof
or speculative serving. All prior scoped objects and `performance.js` remain
unchanged; their no-baseline/no-paged-model flags describe their earlier cutoff.

`validate-matched128.mjs` rejects metric, precision, identity and qualification
mutations. Its optional evidence route binds every displayed metric to the
root-reviewed pair replay and the independent draft captions to raw, adapted and
clean-lifecycle receipts. It is not a replacement model or performance qualifier.

```sh
node validate-matched128.mjs /private/matched-evidence
```

## Earlier Checkpoints

The recovery checkpoint was published at Pages-only commit `4200318`. The prior
`competitivenessSprint` snapshot in `data/project.js` is preserved verbatim and records private integrated JSONL
and loopback HTTP source, scoped host/protocol tests, v8 native fixtures and
separate exact-reference model canaries at budgets 16/32 (actual maxima 16/17,
n=1 each, effectively flat rates). The real HTTP smoke has four sequential
requests/nine outputs with prefix caching off, exact token bytes and usage,
clean teardown and all-eight-GPU idle checks. No endpoint remains active;
the smoke is not concurrent or sustained-load qualification and provides no
HTTP performance claim. Shared peer-currentness results remain native-only;
the frozen-511 two-per-mode admission-cache canary remains separate. Native
fixtures alone are not model evidence. Its then-unimplemented larger-KV flag
is historical, not the current implementation status. Sequential baseline launches
were approved; no matched result was available at that earlier cutoff.
No serving-comparison or competitive-win claim follows from the canary.

The preserved historical `competitivenessFollowup` records the exact-reference wave/v8
canary (two observations per attention mode, four requests/eight outputs per
run), both the performance-rejected 1-ms-backoff experiment and the separate
unpublished 50-us-cap follow-up, host/native v9 physical-capacity extension,
published ordered-batch native checks, and optional authenticated draft intake.
Neither v9 nor ordered native fixtures establish model or 32-request serving
qualification. Separately, the actual 36-GiB v9 allocation and legacy control
both pass the tiny eight-output model reference, n=1 each, at most 16 observed
rows; these remain frozen 3e3 observations, not long-context, 32-request or
speed claims. Draft intake is not speculative execution. The logical context
limit remains 8,192 tokens; 36 GiB refers only to the maximum TP1 KV payload, excluding model
weights and workspaces. The old BF16 wave failures and all `performance.js`
observations remain unchanged.

`validate-competitiveness-followup.mjs` enforces the closed new snapshot,
rejects scope/promotion mutations, rehashes twelve separate canary reports and
raw traces, and recomputes each paired mean independently. It also checks the
nine v9 fixtures, two bounded model-allocation reports and two ordered-native receipts, their root reviews, and scoped
host test receipts. All validation remains remote-only:

```sh
node validate-competitiveness-followup.mjs /private/compete-evidence
```

`validate-competitiveness.mjs` validates the closed snapshot and rejects 30
scope/promotion mutations. Its optional evidence route, run only on mi300x,
rehashes the root-reviewed native/host receipts and all four exact-reference
cache reports, then recomputes the caption's per-mode mean changes. It also
rehashes both v8 model reports/raw traces and checks actual row maxima and
rates, plus HTTP receipt/controller events/clients/teardown without deriving
HTTP performance metrics:

```sh
node validate-competitiveness.mjs /private/compete-evidence /private/serving-evidence
```

Include the separately hashed source-gate `check.log` and
`metadata-summary.json` under `compete-evidence/source-gate/`. The 38 source
tests, 31 verifier-policy tests, 28 metadata configurations and coverage counts
describe compiler-rooted dependency/source checks, not new Verus or GPU proof.

This route does not replace the original native or token checkers. Historical
`performance.js` values, rejected cases and frozen source pins are unchanged.
The 6f6 host receipt is explicitly historical. Neither the earlier f85 ordered
checkpoint nor current public 216822284 relabels frozen 3e3 emissions/wave models
or 511 head/cache measurements. The polling branches remain unpublished.

The additive `competitivenessRecovery` preserves both older data objects and
all of `performance.js`. It records three distinct ordered cohorts, each with
two observations per mode on the same four-request/eight-output logical-tick
canary: the original -35.40% serial-relative regression; +85.60% recovery against
the old regressed ordered worker; and +22.19% against a fresh same-new-worker
serial control. Serial controls vary substantially. None is a steady-state
HTTP result, confidence interval, default adoption or framework win.

Standalone Draft06B matches an independent offline PyTorch reference for six
choices/two final outputs. Its controller was built at 346d588, not the later
exact 656edb2 CPU aggregate. The frozen baseline image retains compiler/SDK 3546
provenance. The separate closed14 v10 image emits on compiler 216822284 and
passes 36 native full-buffer fixtures; it does not establish paged draft model
execution, speculative serving or draft performance. The later role-checked
consumed-input driver at 58ed5c5 seals completed externally supplied, untrusted
proposal inputs and last-proposal catch-up. Its separate host gate passes 526
invocations, 19 ignores, six doctests and one exact-v10 image admission check.
Recording outputs are synthetic; automatic proposals, paged native model
matching and complete speculative serving remain unqualified. The 656 aggregate has
516 test invocations, not a deduplicated count of unique tests; its 18 explicit
ignores and prior failed attempts are retained. No new Verus claim follows.

Continuous V3 tooling has no drain between adjacent windows, retains failures,
and separates completion-window usage from arrival cohorts. The 90-test
collector and 109-test paired-series gates are CPU-only. Descriptive bootstrap
intervals require at least three fresh start pairs; this is not evidence of
stationarity, stable tails or equal framework tuning. The approved sequential
baseline launches had no matched performance result at that earlier cutoff.
The independent target reference passed two repeated 128-input/128-output
runs with a BF16 eager decoder and explicit FP32-head operands/output. It is
not the stock-BF16-head cell, and that reference-only checkpoint was not itself
a Ferric matching result or a baseline serving run.
Its raw, adapted reference and clean-container/idle wrapper receipts are separately pinned.

`validate-competitiveness-recovery.mjs` checks the closed new snapshot, negative
scope mutations, all twelve independently pinned ordered reports/raw traces,
per-mode means, standalone reference/trace custody, native36 output/guard
receipts, consumed-input driver host gate, independent target128 reference,
exact 656 aggregate and the two V3 CPU logs:

```sh
node validate-competitiveness-recovery.mjs /private/recovery-evidence
```

When implementation or qualification state changes:

1. Update `updated`, `readiness`, `validation`, `latestObservation`, and
   `recentProgress` in `data/project.js`.
2. Keep diagnostic, hardware-observed, and qualified claims distinct.
   Update performance values only from current, externally pinned correctness
   reports and ledgers. Keep n=1 ablations separate from repeated profiles,
   never pool request identities, exclude rejected numerical runs, and retain
   the exact controller/worker/artifact/ledger identities. The historical
   ablation queue contains five accepted n=1 profiles; both wave model profiles
   are rejected.
   Keep the later matched MFMA pair separate, including its startup regression.
   Initial serial-peer TP2/TP8 observations had no matched host controls; later
   source-matched controls use deliberately distinct worker executables and
   show large regressions on the peer path. The CPU
   transpose-helper benchmark is not a model latency or throughput result.
3. Install the pinned browser harness and run both the structural and rendered
   checks:

   ```sh
   cd site
   npm ci
   npx playwright install chromium
   npm test
   FERRIC_EXHAUSTIVE_WIDTHS=1 npm test
   npm run stage -- /tmp/ferric-pages-artifact
   ```

Publication requires the exhaustive Chromium sweep and structural checks on
mi300x before the deploy-only workflow receives the admitted static bytes.
GitHub-hosted Node, browser, build and test steps are not permitted by the
static-only publication policy. `validate.mjs` rejects schema
drift, stale current-dependency claims, unknown status states, malformed source
references, duplicate transitions, missing render targets, and missing local
assets. `render-validate.mjs` checks populated visible output without horizontal
overflow at the desktop, breakpoint-edge, and mobile widths; its exhaustive mode
checks every width from 320px through 1440px. `stage-artifact.mjs` creates and
validates the seven-file static deployment roster so `node_modules`, dependency
metadata, documentation, and test-only files cannot enter the Pages artifact.
`validate-performance.mjs` checks the closed performance schema and seventy-two
negative mutations, including incorrect repetition counts, throughput windows,
runtime profiles, ledger identities, and cancelled-request TPOT. A Pages-only
publication must not include Ferric implementation, model files, or raw logs.
`validate-performance-evidence.mjs` additionally compares every new model metric,
request latency, binary/report pin and helper aggregate against externally
SHA-pinned local evidence inputs. Run it remotely with the measurement archive
and transpose archive paths, the previous public performance file and replica
archive path; these inputs are not part of the Pages artifact. Replica cohorts
use a distinct 64-output workload and common RAW release clock, not summed
instance rates or the earlier eight-output window. Publish the per-instance
row policy, actual loaded weight bytes and every request's separate admission
TTFT, release-to-first-token and seven-gap TPOT.

The legacy MFMA data key `ledgerCanonicalId` retains its historical name and
value, but its source field `ledger_sha256` hashes the ledger generator script,
not a canonical report. Render it as `ledger generator source SHA-256`.
The separate ledger-file SHA-256 and actual canonical replica-expectation
digests keep their existing meanings and labels.

Keep the later wide16/32 row-policy pair, source-matched peer controls and
full-model transpose observation separate. Wide row and chunk budgets change
together; request00 TTFT regresses. Peer worker executables differ deliberately
on the same core source. The transpose production change is setup-only, so
observed decode-rate differences are not attributed to that change.

The two-repetition MFMA and TP1 residual tables reuse their published R1
observations plus one new run per profile. Preserve those R1 tables and pins;
do not count them twice. Means and ranges remain per profile and request, with
n=2 nearest-rank p50/p95 equal to the minimum/maximum, not stable tails. The
single MFMA-plus-pruning observation is a distinct cumulative profile and does
not show an additive pruning gain.

Current-controller compatibility is separate from the earlier paired timings.
The rebuilt worker is byte-identical to the earlier binary, and emitted images
retain their original provenance. The cached TP8 capacity-32 smoke observes
only 17 rows. Current TP1 MFMA-only and cumulative MFMA/pruning/residual fail
the exact seed reference and have no accepted timings. Baseline projection
plus pruning/residual passes, with n=1 metrics from its pinned ledger; two
flags change, so no isolated gain follows. Preserve the completed per-case
receipts and the separate outer SSH255 anomaly. The failure is observed
without pruning or device residual, but its numerical cause is not proven.
## Argmax Route Checkpoint

The additive `routeCheckpoint` and `routeReadiness` fields record the private
`d9a2705` correction's exact host gate, the permanently rejected `f662d54`
native attempt, and frozen `f65` overlapping host-counter diagnostics.
The additive `argmaxNative` record binds the corrected six-run native diagnostic
to the unchanged summary, manifest, reducer, checker and controller identities.
Full128 has one run per mode; short8 ABBA has two. Displayed gains use ratios of
arithmetic means, with individual short-pair variability retained. These are
instrumented native controller-wall measurements, not HTTP or GPU durations,
confidence-qualified estimates, a competitive ranking or a new default.
Existing `nativeFollowup`, `matched128`, and historical `performance.js`
records retain their original source and measurement scope.

`node validate-argmax-checkpoint.mjs EVIDENCE_DIR` additionally checks the
five pinned raw receipts/traces. Evidence is retained outside the published
site. Its mutation checks also run in the normal static validator.

`node validate-argmax-native.mjs EVIDENCE_DIR` checks the pinned summary,
manifest, plan and reducer plus all 60 raw files in the six manifest-named
directories. It independently recomputes displayed means and paired reductions,
checks successful reference/cleanup receipts and preserves overlapping host
scope exclusions. The earlier `f662d54` rejection contributes no result or timing.

## Attention Checkpoint

The additive `attentionCheckpoint` and `attentionReadiness` records bind one
`809af24` native host-attribution diagnostic and the separate eight-case
`c26c1f4` finite TP1 fixture pass. The seven sibling groups are disjoint, but
the parent and IPC spans overlap. None is an isolated GPU duration or a
measured optimization gain. The fixtures compare exact complete BF16 bytes,
immutable inputs, masks and tails against controlled finite expectations;
they do not qualify arbitrary softmax inputs, model performance or new Verus
proofs. Their old v5 image retains its original fe2o3 `3e74` provenance.

Run `node validate-attention-checkpoint.mjs EVIDENCE_DIR` on the approved remote
CPU stage to authenticate the reporter, raw timing/trace/receipt and native
fixture probe/report/replay/receipt/prelaunch files. The validator recomputes
each displayed host-span total from the pinned raw timing records and checks
all eight complete native buffer/guard rosters. Evidence stays outside the
site artifact. Mutation checks are part of the normal static validator.

The new combined attention/argmax route's exact ed112 host gate passes 42
required commands across 43 actual attempts, with 685 passed test invocations
and 34 ignored. The nested-TMP failed attempt and earlier Clippy failure remain
separately retained; the unchanged-source gate used a warm Cargo target.
The validator authenticates `composition-HOST-GATE.md` and its archived failure
identities. The same source's four native model qualifications now pass exact
IDs and UTF-8: baseline/wave attention, each at 8 and 128 outputs, with fixed
wave-v11 argmax. These are correctness diagnostics, not comparison samples.
The validator authenticates the qualification note, both equivalent forty-file
hash rosters, all raw files, original references, fixed profile and normal
retirement/close/idle receipts. No qualification timing becomes a metric.
The separate full128 ABBA cohort is now admitted as descriptive native host
diagnostics only, with two runs per attention mode. Its forty raw files are
bound by the frozen manifest and size ledger, then connected to the unchanged
reducer replay. The validator independently recomputes per-run timings from
the original observation offsets, arithmetic means, min/max ranges and paired
ratios. Output rate is the arithmetic mean of per-run rates, not a pooled rate.
Wave's substantial observed variation prevents a stable-gain claim. No HTTP,
GPU-duration, confidence, competitive, default-promotion or M1 claim follows;
qualification timings and older cohorts are excluded. Existing
`matched128`, `routeCheckpoint`, `routeReadiness`, `argmaxNative` and
`performance.js` data are immutable historical observations in this update.
