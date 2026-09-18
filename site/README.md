# Ferric project site

This directory deploys a dependency-free static GitHub Pages site. Project
status is kept in [`data/project.js`](data/project.js); curated, strictly
checked performance observations live in
[`data/performance.js`](data/performance.js). The harness pins Playwright for
real browser checks. Run all checks on the designated remote build host, not
locally, and remove the private stage after archiving evidence.

## September 18 Performance Swarm V4

The current overview records private Ferric `1fc45a52`, published core runtime
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
