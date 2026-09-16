# M1 Team Progress

Updated: 2026-09-16 UTC. This is an implementation checkpoint, not a qualification
receipt. The 33 M1 roadmap gates remain open.

## Current Checkpoint

### Published Compiler And Current Adoption

The current published compiler correction is fe2o3
`1fadb7e0a8ee2a9e8cdbcdf5b8648e20a36161de`. It was rebased onto freshly
fetched main `958a80a3d`, then pushed normally with `[skip ci]`. Post-push and
delayed GitHub API checks find zero workflow runs; the post-push check-run
count is also zero. All builds and tests remain on mi300x. This eight-file
change adds trusted write-only output ABI recognition and exact Thread-call
value resolution with focused regressions. Kernels and inference remain in
Ferric; private gather/slice-read experiments are excluded and remain unfixed.

The clean R5 candidate on parent `4c2983820` passes all 52 selected tests:
17 reference-effect units, 30 GPU-expression tests and five actual extraction
integration selections. Strict Clippy exits 101 with 17 diagnostics at unchanged
upstream source sites; the overall campaign remains 101. No clean-base Clippy
run or successful proof admission is claimed. The positive reaches the real
proof-runtime-unavailable boundary. R5 archive SHA-256 is
`ebe08d207b3eb15aa86c099507724e4377c44a4c302aea27c1187b31c9240a10`.
The later three-path upstream rebase changes only a dev dependency and related
lock/tutorial hashes. A separate mi300x metadata check passes with identical
155-node normal/build dependency graphs and unchanged protected artifacts.
The 52 tests retain their original R5 attribution; they were not rerun on the
rebased commit.

Private Ferric integration `a0cc9929389d02a94a46208064f451592dd94d5b` adopts
that published compiler across exactly 80 active pin files. The first adoption
attempt stops on its revision-only inventory assumption after 32 locked graphs
and 42 fresh source-gate tests pass. Its failure is retained as archive
`0b1ebbcd5b41b56434076a9c0adec81ede14abf29be8f4a89443aa606515b530`.
A separate fresh R2 campaign passes all 47 phases, including four actual CLI
inventory derivations. Besides the compiler revision, the reviewed inventories
add exactly upstream's `native_v12_text_descriptor_replay_v1` test target and
the `dialect-amdgcn` development declaration for `fe2o3-amd-target`. The runtime
inventory and historical property-binder pin remain unchanged. All 1,148 local
source hashes and the full tracked-file roster match the validated remote source.
The 20,544,105-byte successful archive has SHA-256
`dacf7b109448bbc90d4d8bd5ea30f1eac7f1b0723630b674bb6f7776e5579fc8`;
all 1,608 members and 1,607 payload hashes and sizes pass independent local
verification. This is dependency/source-gate coverage, not engine, Verus, native,
numerical or serving validation on the new compiler.

All six matching compiler-tool build/copy/source phases subsequently pass on
mi300x at exact `1fadb7e0`. The CLI, linker proxy, backend and extractor are
fresh Cargo products; the backend build-script record binds the new CLI.
The 5,046-file compiler census and protected historical tools remain unchanged.
CLI SHA-256 is
`081f3092d37176e10f31664a5c8d5288bfeebd2972087b5daf9f65480e1a3b5a`;
backend SHA-256 is
`43741b5938d2c32e307de4655a6ee10d1f9a04ee0508de25d7fd1c839d37606e`.
The 60,995,341-byte archive has SHA-256
`9084ba6be03d07b023703bf023887456d081b807f52e3035f7450f525d606d4b`.
All 85 members and 84 payload hashes and sizes are independently checked after
transfer. This establishes matching engineering tools, not full compiler
qualification, kernel emission or a GPU result.

The matching optimized `ferric-m1-engineering-speculative-smoke` binary also
builds on mi300x from exact `a0cc9929`/`1fad`. All six release phases and source/
control after-checks pass. The normal no-feature opt3/debug0 Cargo product is
fresh, and all 1,148 source files remain unchanged. Its 14,254,656-byte ELF has
SHA-256 `6c46c97b2ad1ab3c880ea69f32b4a295eadcddbc8eb4b666ecf7299395d1edf4`.
The 11,351,881-byte archive has SHA-256
`d3886c416fa71a489632b94cba1970e07c74ae1901b135f3e76ee457fec3859a`;
all 88 members and 87 payload hashes and sizes pass independent local checks.
This establishes the matching host executable, not numerical or serving
validation.

Matching MFMA13 emission subsequently passes all 12 mi300x phases, including
fresh empty-home metadata parity, exact 13-entry/descriptor inspection, actual
MFMA instructions, `gfx942:xnack-`, code-object version 6 and exact output replay.
Source, tool, vendor and control after-checks pass. The reused LLVM worker keeps
its original `21682228` producer attribution. The 112,680-byte HSACO has SHA-256
`08b8b24eac612c3c65739cc9703b7822088863819d96ce4ae92470b858c60568`.
The 2,034,624-byte emission archive has SHA-256
`4bd5880951496586b7134c9ad00f5512f950ca223c5b8354e1ef00baba026666`;
all 169 members and 168 payload hashes and sizes pass local custody checks.
Engineering emission grants remain false. This is not production admission.

A separate guarded M5 native capture and independent reference comparison now
both complete on mi300x GPU 4 at exact `a0cc9929`/`1fad`. Both process and outer
wrapper exits are zero; source, input and control checks pass, both owned
process groups are absent, and GPU allocation returns to its original
298,647,552-byte idle baseline. The capture contains 1,519,360 BF16 bytes for
all five target positions 128 through 132 from one actual S1/K4 round. This
uses the historical 128-active-token filled prefix, not normal real-prompt
serving. The reference evaluates the entire 133-token context without Ferric
KV twice, with byte-identical results. All logits are finite, and all five
argmax tokens match: `[4710, 16, 15, 16, 198]`.

| Position | Maximum Absolute Error | RMSE |
| --- | --- | --- |
| 128 | 0.2421875 | 0.07160754675058952 |
| 129 | 0.203125 | 0.06948075238695375 |
| 130 | 0.125 | 0.03128908742969308 |
| 131 | 0.1875 | 0.07552450047396397 |
| 132 | 0.2001953125 | 0.05692751000637275 |

Maximum BF16 ULP distance is 31,640. No tolerance has been accepted, and token
agreement does not establish logit conformance or close R29/R30. The capture
archive is 5,580,101 bytes, SHA-256
`e6b01d2b8ae509eb74c64a132b776877a90cfabe716d93be707bb067aa25c3be`;
all 50 members and 49 payloads are checked. The reference archive is 2,345,460
bytes, SHA-256
`b9d57066228ac02f24b1a597e693253bf3871b04235ce9e6b79277477adbc2f2`;
all 59 members and 58 payloads are checked. Both have complete local custody,
including exact roster, payload hashes/sizes and retained raw terminal records.
Comparison SHA-256 is
`e4e46d68e521a1391d09cb44f3eac03f12ee2b5d50429225f339d3ad0ac7eec6`.
There is no new TTFT, TPOT, throughput or serving-qualification claim.

Read-only source review identifies a concrete early rounding-boundary candidate:
Ferric RMSNorm keeps normalization and weight multiplication in FP32 until its
final BF16 store, whereas the retained pinned-reference diagnostic narrows
normalization before multiplication. The current RMSNorm source is unchanged
from that diagnostic. This is not yet established as the cause of the actual
five-row differences. No numerical kernel change is included in this checkpoint.

A subsequent upstream fetch observes `86248cd74453f8c6fbf19e08a5b6cbfce3fc42cb`,
five commits after `1fad`. The 63-file change introduces kernel-context
entry generation and frontend authentication, including a new
`fe2o3-macros` dependency on `fe2o3-rustc-front`. No KFD/runtime or toolchain
change is present in this delta, and Ferric's device sources contain no
`KernelContext` or reserved issuer use. The collector nevertheless adds checks
over reachable calls, so source inspection alone does not establish full
compatibility. This newer head is not yet adopted. Running build/emission
checkpoints remain frozen on `a0cc9929`/`1fad`; no result is relabeled as a test
of `86248cd7`.

After separate inventory and no-use checks, exactly 41 superseded single-link
debug rlibs are removed from the owned compiler cache, reclaiming 528,704 KiB.
All 960 protected files, 5,046 compiler source files and 6,909 unselected target
entries remain unchanged. Selected-path checks pass; global process visibility
is explicitly incomplete. Only regenerable cache is retired, with no claim of
a complete byte archive for those discarded files. Existing compiler111 tools,
MFMA13 objects, model inputs and foreign jobs are untouched.

A second separately reviewed cleanup removes exactly 48 unused cache files:
14 superseded release rlibs and 34 older libtest executables, reclaiming
440,084 KiB. All 784 protected files, 5,046 source files and 6,878 unselected
target entries remain unchanged. Fresh compiler-stage allocation is 9,758,752
KiB. Subsequent bounded vendor formation passes: 198 packages and 11,394 entries,
with 60 existing package directories moved and 138 missing packages copied.
All 143 registry rows and original historical inputs remain unchanged; Cargo
normalizes the 55 exact Git packages offline. Formation and later emission stay
within their separately reviewed bounds. The scoped
no-use checks retain their explicit process-visibility limitations. No historical
test pass or complete byte archive is inferred for the discarded cache.
The completed clean local compiler worktree is also removed with ordinary
`git worktree remove`, reclaiming 87,152 KiB after exact HEAD/tree, source-archive
and scoped no-use checks. Published history, private refs and retained evidence
remain intact; unrelated worktrees and dirty user repositories are untouched.

The public Pages checkpoint is now static commit `573a3da5`, deployed by
successful upload-only run `35081715744`. All site QA ran on mi300x: five phases,
exhaustive widths 320 through 1440 and 64 screenshots pass. The retained archive
has SHA-256 `e84fa90c07f6f2a1b07a497a6438babf6f21a5787e1088b9e3916b3011f28c3a`;
all 134 members and 133 payload hashes and sizes are checked. All seven live
assets match the admitted artifact, both with and without cache-busting queries.
The site records compiler adoption and prior M5 host/proof evidence, not the
subsequent tool/release builds or a new performance result. The preceding R15
UI-label failure remains retained. The completed clean Pages source worktree
is removed, reclaiming 3,324 KiB while preserving its branch and evidence.
Five fully retained remote Pages input/build/retention roots are also removed
after exact fresh inventories and scoped no-use checks: 785 entries and
78,248 KiB reclaimed. All five roots are confirmed absent; foreign jobs and
the retained publication checkout are untouched.

### Earlier Checkpoints

All builds, tests and Verus runs remain on mi300x. The fe2o3 main push
`111722028` contains `[skip ci]`; post-push GitHub API checks find zero workflow
runs and zero check runs for that commit. It contains only the six-file core
verifier correction described below, not Ferric kernels or inference code.

Private integration `2bfce38b` now adopts published fe2o3 `111722028` across
the exact 80 active pin-bearing files. All 47 mi300x validation phases pass:
32 locked/offline dependency graphs, 42 freshly built source-gate tests, and
four actual CLI inventory derivations. The three compiler-bearing inventories
change only their revision; the runtime inventory and historical property-binder
pin remain unchanged. All 1,145 final local source hashes and the complete
tracked-file roster match the validated remote snapshot before commit. The
20,487,388-byte archive has SHA-256
`ebbfc150c4811e4f314a81f343977d9eac055e353ffc6c6aa17f2dcec59edd67`;
all 1,604 members pass independent payload, size, mode and numeric ownership
checks. This is dependency/source-gate coverage, not engine, Verus, native or
performance qualification. Main still identified `111722028` when this
checkpoint was adopted.

During the subsequent emission/build work, upstream advances to `349b2cd0`.
The fetched five-commit delta changes nine files: exact unsigned-literal formal
address reasoning and policy-3 semantic replay, with tests and documentation.
It changes no manifest, lockfile, host or KFD API. No direct Ferric caller of
the new replay API is found. Private integration `803916c4` now adopts that
revision after a separate successful 47-phase mi300x campaign: 32 locked graphs,
42 fresh source-gate tests and four inventory derivations. The exact 80-file
transition again preserves the runtime inventory and historical property-binder
pin. All 1,145 local source hashes match the validated remote census; 79 changed
files independently match revision-only substitution, while the remaining
source-policy file also updates its two manifest/lockfile hash rows. The
20,500,070-byte retained archive has SHA-256
`a9c0daa6a5b7203aae6c7f177827bf4c5fa17855e4e2c7c6f3641c07dc3ef3ba`;
all 1,604 regular members and 1,603 payload hashes and sizes pass independent
local checks. No engine, Verus or GPU test is attributed to this adoption.
The `111722028` build/emission/native records below remain explicitly frozen
at that checkpoint and do not validate the newer compiler.
An intermediate read-only upstream check subsequently finds `45d0bf2e`: nine commits
over 349, covering generative tile providers, nominal refusals, descriptor
signature normalization and associated tests. The active diagnostic campaigns
remain frozen on 349; this newer head is not adopted by Ferric. The compiler
fix lane then rebases onto `9af22bb5`, one further optional-GDB-audit test commit.
Its separately attributed validation below does not upgrade Ferric's adopted
compiler pin or relabel any compiler111/349 evidence.

The matching engineering CLI, linker proxy, backend and extractor build on
mi300x from the exact published compiler archive. All six tool-build/copy/source
phases pass; selected executables are fresh Cargo products, and the backend
build-script receipt binds the actual CLI hash. Source before/after censuses
are identical. CLI and backend receipts have SHA-256
`4495ea34e77578894895b6ea76a892294f9c2f6f2fcc9994f72bfbb7fc37e67f` and
`527bd4073fdce65e6fd6d013127e3e1c57c244a0952e97f32ad13add88cc306f`.
These are engineering tools, not full compiler qualification. Fourteen
obsolete 8af test executables are removed only after rechecking their retained
archive identities and lack of live use, reclaiming 475,400 KiB. Their bytes
remain in the earlier R5 archive; shared libraries and foreign jobs are untouched.

Fresh MFMA13 engineering emission on exact `2bfce38b`/`111722028` passes all
13 phases on mi300x, including unchanged source/vendor after-checks and empty
Cargo-home dependency equivalence. The gfx942/COV6 object replays exactly and
inspection finds all 13 expected entry points and 64-byte descriptors, plus
the actual `v_mfma_f32_16x16x16_bf16` instruction in the selected GEMM. The
112,936-byte HSACO has SHA-256
`f4f365f7a79b56d95eb429b0b5bc5a150f09e9a3fb7923363ac43392ac63878b`;
its inspection receipt is
`2275b3eb52a31ad94dc76cfe2bd3cdd94eace306be53ba8e9d25ce00eacfd7c8`.
The unchanged LLVM worker retains its original `21682228` producer attribution.
This is emission/ELF inspection only, not GPU or numerical qualification.
The combined tool/emission archive is 108,806,389 bytes at SHA-256
`e67e7d0fd91bab2285c8b720ef74a39a2d7707e03291bb1a04f1ba311be72003`.
Its exact 230-member roster and all payload hashes pass independent local
verification; all selected remote input identities and both source censuses
remain unchanged. Build targets and vendor/cache payload trees are excluded.

The matching optimized engineering capture and speculative-smoke binaries now
build successfully on mi300x. All six release phases and control after-checks
pass at exact `2bfce38b`/`111722028`, with fresh opt3/debug0/no-feature Cargo
producers and all 1,145 source files unchanged. The smoke binary is 14,187,984
bytes with SHA-256
`4a4b50a3903b88acbc93b10d61f90cf0b09cd53c28fed56895bab679ad71d420`;
the actual producer receipt is
`52a037ce547ec0e822531a2123461f93f485a63285375e6a4454b56961071417`.
The compact 7,426,301-byte release archive has SHA-256
`3cb470990f937b8f1dd2334be6aa948058b1674fb93cb810a356f818b05b1bd1`;
all 90 payloads and 91 members pass independent local verification. Build
success does not establish native catch-up, numerical or serving qualification.

The separate bounded native MFMA13 retry now passes on mi300x GPU 4. It
publishes 32 tokens over 20 speculative rounds and observes one completed
draft catch-up after continuing full K4 acceptance. Process, report validation
and outer control each exit 0; the report recomputes the physical epoch,
generation, token and maintenance counts. The queue is destroyed normally and
GPU 4 returns to its starting 298,647,552-byte allocation with no selected
foreign queues or retention-capture issues. This exercises the previously
failing catch-up path with the rebuilt binary, not merely its host regression.
The 3,989,932-byte native archive has SHA-256
`46c5dc43d7e73cd0a91e00baa7f7b82aee84a9b7bcf2fb76aa2e040455d942f4`;
all 42 payloads and 43 archive members pass independent local checks. The
actual native report has SHA-256
`185100350792c1797ca34a13cf76ce34487b6b7c172442b703659d88a86b31a7`.
Startup allocation/upload completes at 249.178 seconds; prefill through native
teardown takes another 118.417 seconds. Neither is TTFT or TPOT. The prompt
contains active suffix-fill tokens and the 32 outputs exclude the prefill
anchor. This remains an engineering-only observation, without independent
numerical parity, serving qualification, a matched benchmark or a closed gate.
After verified retention, exact file/stat/hash checks, absent owned process
group and diagnostic-free selected-path no-use checks, the completed native
scratch stage is removed: 33 files, three directories, 14,508,032 allocated
bytes. Reusable model, compiler and release inputs remain on mi300x.

The first independent current111 prefill comparison is now retained. Exact
`2bfce38b`/`111722028` captures `prefill-s1-t128.001` on GPU 4; the canonical
PyTorch reference runs twice with byte-identical outputs. Both native processes
exit 0 and return GPU 4 to its 298,647,552-byte baseline. The comparison covers
151,936 finite BF16 logit pairs and one token, with zero token mismatches,
maximum absolute error 0.125, RMSE 0.03982024072957679 and 2,725 bit-identical
pairs. Maximum BF16 ULP distance is 31,373: that pair crosses zero, from Ferric
-0.047607421875 to reference 0.04931640625. No reviewed tolerance or numerical
pass is claimed. Token agreement alone does not close numerical qualification.

The first comparison wrapper exits 1 because it expects the old V1 report
schema; the actual Rust comparator exits 0 and produces a valid V2 report.
The corrected wrapper exits 0 in a separate output directory and its report
is byte-identical to the first report. Both attempts, raw logs, actual capture
and comparator executable bytes, generated inputs, reference sources and
output bundles are preserved in the 6,133,251-byte archive with SHA-256
`e9d233c8d9287ffae71184e7a07c6c064c866bc56c1e80ba643be22313a17f60`.
All 260 regular members, the exact ordered roster, and all 259 payload hashes
and sizes pass independent local verification. Model weights, reference caches
and build targets are excluded. This is not a TTFT/TPOT or serving measurement,
and does not validate compiler `349b2cd0`.
After verified local retention, the four completed prefill capture/reference/
comparison scratch directories are removed, including their private disposable
caches: 113 files, 13 directories and 350,568,448 allocated bytes. All 90
retained payloads in those directories are rechecked, prior process groups are
absent, and both selected-path `fuser` batches return no users and no diagnostics.
Reusable model, reference environment, compiler artifacts and input/output
bundles remain available. The cleanup receipt has SHA-256
`eb80d158c28cc794f4e88134353565b383f62685c685b6dc5d1e39e50b9bf742`.

A separate bounded CPU-only experiment on mi300x finds concrete per-operation
rounding witnesses against the installed Transformers 4.51.0 implementation.
Synthetic RMSNorm has 229/1,024 differing BF16 outputs, SwiGLU 1,033/4,096,
and rotary embedding 172/1,024 when FP32 intermediates are kept through the
final operation instead of applying the canonical intermediate BF16 narrowing.
The explicit SwiGLU intermediate narrowing exactly matches the canonical
activation outputs in this experiment. This does not execute the device
kernels or establish the cause of the full-model error: device transcendental
implementations, reduction order, fused residual inputs and MFMA remain outside
the experiment. No kernel or tolerance is changed. Raw report SHA-256 is
`435ed666e165d378d2f2c6eb921249452d5d3c122194f1fdc720f431b7df72a0`;
the diagnostic source, logs and terminal 0 are retained separately from the
completed prefill archive.

The separate draft catch-up proof candidate `a9fad31b`, on compiler
`111722028`, passes its 21 selected host coordinator tests with zero failures
or ignored tests, plus strict engine all-target Clippy. Verus b677 then verifies
the actual production `commit_draft_catchup_transition` method: one verified,
zero errors, with `--no-cheating`, genuine dependency exports and unchanged
source/dependency after-checks. Its contract covers admission and error
precedence, exact draft cursor/epoch advancement, and success/failure framing.
It does not prove the whole engine, sealed owner mapping, queue completion or
the physical KV join. All nine actual-body negative mutations now fail the
intended postcondition with one error, no VIR error and nonzero solver work.
All nine source restorations and final source/dependency/configuration/artifact
checks pass. The full positive/negative archive is 192,903,662 bytes at SHA-256
`532089c5c9b70861d89a0b09446002d8862b1639cd2d20d67ed99e7c316f1d91`;
all 2,925 regular members and 2,924 payload hashes pass independent local
verification. Combined M5 candidate `568882e2` contains the exact proof file
and import-corrected M5 changes. Its compiler349 host campaign passes and is
integrated privately at `6b5322dc`; only this progress document differs from
the tested source tree. This original focused Verus result and its nine
negative mutations retain their compiler111 attribution.
The fully downloaded and independently verified host and proof archives have
SHA-256 `f125c669bc7113de2e96304ef0a21db31a0bc89fda044588fcf7e5ce81178065`
and `c263b3acd0a6c58a8da0da8184677df58c82d9071eda7844cebe70a37d505f86`.
After confirming the fresh349 proof has no dependency on the old111 host
target, that one reproducible cache is removed. All 1,284 retained source,
control and evidence payloads are rechecked first, including the actual test
executable; all old process groups are absent. A 3,283-entry owned-file census
and 52 diagnostic-free no-use batches pass, with no external hardlinks.
The cleanup reclaims 1,427,656,704 allocated bytes and leaves all non-target
evidence unchanged. Receipt SHA-256:
`7a889c67a7330ca7e440f563377a6433df9d6f33617b9ed180cdeddd55bff6b3`.

The M5 engineering candidate `cf8dd23e` implements one-shot readback of all
five real S1/K4 target logit rows and a separate canonical full-prefix
reference. It includes partial-copy/failed-teardown ownership regressions and
rejects rearming this capture allocation. Its first host campaign on compiler
`349b2cd0` fails engine-test compilation with five errors caused by four missing
test-module imports. Source, formatting, both metadata graphs and final source/
control checks pass, but no Rust/Python tests or Clippy execute. Failed R1 is
fully retained at archive SHA-256
`f60a0a4b651569a2d973c2996981b2d8ce5bc9d7db8b08283519593b9033900b`.
Import-only successor `e16eb929` and R2 controls remain unexecuted. The combined
`568882e2` source instead passes all 21 R3 host phases on mi300x with compiler
`349b2cd0`: 750 engine tests pass with nine explicit ignores, all 22 smoke-bin
tests pass, the three Python suites pass 4, 23 and 15 tests, and strict Clippy
passes for the engine library and exact smoke binary. Every required M5,
partial-read/failed-teardown, catch-up and full-accept regression is explicitly
`ok`, not ignored. Both locked dependency graphs, format, exact test-producer
receipts and final source/control checks pass. This is not all-adapter or
all-target Clippy coverage.

R3 evidence retains the actual engine and smoke test executables, source
archives, controls and raw results in a 57,589,388-byte archive, SHA-256
`a8c9d39b1baceaf6c4ba8c776dc36c15c46986cc6a859de59a759d5607e9a066`.
All 344 members and 343 payload hashes, sizes and modes pass independent local
checks; the embedded manifest and summary match their separate copies.
Private integration `6b5322dc` contains the exact tested implementation.
No M5 native capture, five-position numerical comparison, serving
qualification or new performance result is claimed.

A separate fresh compiler349 Verus campaign now passes on the exact combined
`568882e2` source. All 14 phases pass, including seven genuinely rebuilt and
verified dependency exports and the selected production
`M1SpeculativeGenerationLoopV1::commit_draft_catchup_transition` method:
one verified, zero errors, executable mode, solver rlimit 116,015, with
`--no-cheating`. Complete source/dependency/configuration/Verus-closure and
artifact after-checks pass, as do preservation checks for all 2,924 retained
compiler111 evidence files. The fresh target is isolated under the existing
whole-stage 2 GiB cap; no old proof source or artifact is replaced.
The 169,061,545-byte archive has SHA-256
`6013e24b89fb72ca30b8c654a737a3405c6e5ff59f529bc8a2040b986fedf03c`.
All 1,577 regular members and 1,576 payload hashes pass independent local
verification. The nine negative mutations have not been rerun on349. This
remains a single-method proof, not whole-engine, queue/owner or physical-KV
composition, and does not validate later compiler heads.

A bounded independent source review of the M5 allocation, rearm, binding and
readback changes finds no actionable issue. It confirms capture exclusion from
catch-up/general rearm, two target-logit bindings in segment four, complete
five-row generation/extent checks, and 12 allocations within compiler349's
unchanged fixed limit of 16. This review does not establish native correctness.
The completed clean M5 worktree is removed after source/archive and no-use
checks, reclaiming 44,856 KiB locally; its branch and tested commit remain.
The reusable compiler lane separately reproduces an earlier admission gap:
the real-style guarded gather reference rejects `WriteOnlyDisjointSlice<u16>`
as having no reference ABI relation. The observation harness passes while the
compiler subprocess exits 101; this is not compiler/proof success. A narrow
output-only relation fix builds and passes its three unit tests, but its
ordinary-source positive test fails at a later authenticated store-mapping
boundary. Source/protected-input/control/toolchain after-checks pass. The
failed positive is preserved; remaining read-rejection and gather diagnostics
do not turn that failure into a successful compiler pipeline.

The R4 compiler successor on exact main `9af22bb5`, candidate tree
`6401d5e74a9a072713711825de5caa4b1a519bfe`, adds narrow trusted Thread-call
store-value mapping using the existing dominance resolver. Fresh producers,
three output-ABI unit tests, 30 GPU-expression regressions and the unchanged
ordinary-source positive all pass. The positive now reaches the existing
`functional-refinement proof runtime unavailable` boundary. Its compiler
still stops before proof admission or artifact emission; no solver or
production-admission success is implied.

R4's overall exit remains 101 because scoped strict Clippy reports 17
diagnostics. All reported sites are unchanged relative to the exact `9af` base;
this is source attribution, not a separately executed baseline result. No
warning is suppressed or unrelated code refactored. Final source, producer,
control and protected-input checks pass. The 88,279,315-byte terminal archive
has SHA-256
`717ef333171433e583cc44d4b4e7e1bef80794cd0719ef198fc63940e1552e09`;
all 183 regular members, 182 payload hashes and four actual producer ELF
copies pass independent local verification. Before this run, four exact
obsolete producer names are retired after retention/no-use checks, reclaiming
273,312 KiB without changing the shared-stage cap or protected inputs.

This candidate is not published. The proposed public patch will preserve the
reusable ABI/resolver changes and meaningful passing coverage, but exclude
the privately retained gather/slice-read diagnostic experiments. The latter
failed assertions are not rewritten as passes; the proposed standalone
coordinate-read regression has not run. A fresh-main public diff review and
validation remain required before any `[skip ci]` push. The current shared
compiler stage has only 194,720 KiB free within its cap, below the existing
512 MiB initial reserve; no further build is admitted at this checkpoint.

Separately, two redundant remote archive copies are removed after local
verification, reclaiming 116,240,384 allocated bytes. Their attempted extra
`fuser -s -- PATH` check rejected the argument separator but returned the same
status as no users; the shell incorrectly continued. This is recorded as an
invalid no-use check, not evidence of absence. Only the two owned completed
archive copies were selected; no runtime inputs were removed. The later native
scratch cleanup above rejects all diagnostic output and uses the supported
absolute-path invocation. The original check failure remains recorded under
`.codex-tmp/ferric-compiler111-tools-v1/REMOTE_ARCHIVE_COPY_REMOVAL.md`.

The newer dated native/numerical/proof checkpoint is public at static Pages
commit `3e41846e069abe80bbf49a048cd7e731fe2363cc`, built and validated only
on mi300x from separate site source `4c206bfc`. R14 passes all five QA phases,
the 320--1440 width sweep, eight viewports and 64 screenshots. Its 11,338,223-byte
archive has SHA-256
`56ea79d0d73790347d975378e1c256b034e1fab66879f88ffa5da63d42073bdf`;
all 134 members and 133 payload hashes/sizes pass independent local checks.
Static-only deployment run `35072403884` succeeds; all seven canonical and
seven cache-busted live assets match the exact 633,233-byte artifact. Only
prebuilt assets and workflow hash/size/comment pins change. GitHub runs no
build or test. This dated snapshot precedes the combined M5 host integration
above and does not upgrade numerical, serving or performance qualification.
After verified publication and local evidence retention, the six completed
R13/R14 remote QA/input/retention directories are removed, reclaiming
104,329,216 allocated bytes. Both 445-entry deletion journals match the
reviewed inventories. The first R14 cleanup stops before deletion on an
unknown transient SSH process visibility gap; an unchanged-control retry
passes after that process disappears. Process visibility limitations remain
explicit. The clean completed site worktree is also removed, reclaiming
3,296 KiB locally while preserving its branch and source archives.

The preceding dated dependency checkpoint was published at static Pages commit
`bcc6d1badafd7d6bd1641c3c867cffd6cd7e63d2`, from separate Pages source
`012d7af8`. All five mi300x QA phases pass, including the 320--1440 width sweep,
eight viewports and 64 screenshots. The retained 11,313,074-byte archive has
SHA-256 `4ff3a911384eeca221bcdbf0c09f25dc39f47d1f047286180869c60d7f536caa`;
all 133 payloads and 134 archive members pass independent checks. Static-only
deployment run `35063661900` succeeds without GitHub build or test commands.
All seven canonical and seven cache-busted live assets match the validated
625,414-byte artifact. The publication covers dependency adoption, not the
subsequent compiler-tool/emission work or a new native result. Only the approved
`pages/prebuilt` branch is pushed; private implementation history stays local.
After archive and no-use rechecks, the three completed Pages directories are
removed, reclaiming 50,816 KiB. The terminal receipt is
`a25cad6a58f62612356c554af5c19bc1bdaf8bfa7e0dd52273b8cc2f41724697`;
its 445-path deletion journal matches the approved inventory exactly. Local
screenshots and evidence, shared caches and foreign jobs remain untouched.

Private Ferric `f8eed966` adds an explicit leading `--mfma13` option to the
engineering speculative smoke. The default remains `LegacyScalar12`; one-step
and resident reports bind their strategy and exact 12/13 program count to the
admitted artifact. Argument failures precede artifact or device I/O. The
gfx942-only device boundary and authority/comparison exclusions are unchanged.

On mi300x, smoke R3 checks 110 library, 20 speculative-smoke and 17 target-smoke
tests successfully, with two existing hardware tests ignored. It then records
a genuine source-policy failure: the old test expects a single scalar opener
instead of the explicit strategy match. The test-only successor checks both
admission routes, their common fallible error boundary, and all six ordered
startup completions. All 38 source-policy tests pass. Four selected engine
regressions and three GEMM regressions also pass, including the draft-catchup
reservation test covering K4/K8/K16; adapter `--all-targets -D warnings` passes.
These are separately attributed host cohorts, not new emission, native,
numerical or performance results. R1/R2 parser failures, R3's policy failure,
and R4's pre-engine wrapper-label rejection remain retained rather than being
relabeled as whole-campaign passes.

The fe2o3 verifier correction is published `111722028`, rebased onto freshly
fetched main `d6471109`. It boxes the cold V4 nested error while preserving
Display/direct error-source downcasts, and makes four Cargo fixture paths
mandatory at compile time. Earlier e68-based affected tests pass after using
the fixture's required package-only debuginfo setting. Scoped strict Clippy
passes; its first result checker incorrectly expects executable debug settings
on metadata-only artifacts, and a separate raw-evidence inspection records the
passing lint scope without changing that failed campaign. The exact rebased
candidate passes all 16 current-tip validation phases: 152 tests pass, ten
existing tests are ignored, and strict Clippy passes for the library and four
changed fixture targets. Six fresh test executables and both normal fixtures
are bound to the candidate source; source/control after-checks pass. A final
fetch confirms the parent is still current before the normal main push.
Full-package legacy test lint debt remains outside this six-file correction.
This is verifier regression coverage, not complete compiler qualification.

The final verifier archive is 52,872,086 bytes at SHA-256
`8f62d1a78c64306e7bb3b892ad9f267de1bf7de367e4891ae19baa510e402c34`;
all 166 payloads, 168 members and embedded metadata pass independent local
checks. The incremental Ferric policy/engine/GEMM/lint archive is 25,507,099
bytes at SHA-256
`191e406d7a1c4372395eb981a83bb6ad6377a409479b46ae4f74444dc81353e5`;
all 111 payloads and 113 members pass independent checks. Both retain their
earlier archive dependencies and failed attempts without duplicating full
source trees. Local evidence remains available under
`.codex-tmp/fe2o3-verifier-error-size-v1` and
`.codex-tmp/ferric-smoke-mfma13-v1`. After verifying custody and absence of all
recorded process groups, the two completed owned mi300x build stages are
removed, reclaiming 3,398,644 KiB. The completed compiler worktree is removed
with its branch and published commit preserved. Shared caches and foreign
workloads are untouched. Removing the eight independently verified remote
retention copies reclaims another 293,744 KiB; their local archives remain.

Private integration `71bd407e` adopts freshly fetched fe2o3 `8af54567` across
the 80 active pin-bearing files. Its complete mi300x dependency/source-gate
campaign passes all 47 phases: 32 locked/offline graphs, 42 fresh source-gate
tests, and four actual CLI inventory derivations. The three compiler-bearing
inventories change only the revision; the runtime inventory and historical
property-binder pin remain unchanged. No dependency-edge repair is needed.
The 20,472,348-byte archive has SHA-256
`be2ef2071511d2fe92558e8a853065575f52ae35a7b494db84fc5826f772a57e`;
all 1,603 payload hashes and all 1,145 final local source hashes pass independent
checks. This is not compiler-tool, engine, Verus, emission or GPU qualification.
The separate compiler R1 focused campaign stops with a harness failure after
33 successful phases. Twelve checked test groups pass 661 tests; the backend
suite itself reports another 610 passes, zero failures and zero ignores, but
its merged stderr warning interrupts one named-test line. The exact-name checker
rejects that output, so R1 remains exit 1 and nine later phases are unlaunched.
All 14 freshly produced test executables and prior tools/source are preserved.
The 108,475,406-byte failed archive has SHA-256
`50bca68de861314dcecd7178a6ad8830a956822d9eff6e5e1231e6e77add96e3`;
all 394 payloads and its exact 396-member roster pass independent local checks.
The same-source R2 continuation separates stdout/stderr without suppressing
diagnostics or relaxing named-test checks. Backend, kernel-opt and Pliron pass
610, 26 and 1,258 tests respectively, including the required optimizer-policy
regressions. Together with R1's checked prefix, this is 2,555 checked test
executions across 15 groups, not a completed compiler campaign. Aggregate
reference then fails because the direct runner's loader path omits the freshly
built compiler shared library. R2 remains exit 1; six later phases are unlaunched.
Its 216,091,762-byte archive has SHA-256
`2edc8a42bf66bb818a3553c7dd8994f183f0f10ff4a4f6747b9cdc376dc7bf5a`;
all 494 payload hashes and the exact 496-member roster pass independent checks.
R3 binds the existing normal extractor, exporter and backend producer records,
but both preflight guards reject the existing nightly library directory's
0775 mode. No substantive test phase launches, and R3 remains exit 1. The two
new debug directories are private 0700 paths; the rejected nightly directory
is already the unchanged wrapper's library path. A narrow successor will bind
that existing toolchain trust separately without changing its permissions.
The failed preflight is preserved in 22,078 bytes at SHA-256
`a34220a9118bbc99b97613a2c86c60e68707e287815289ffce4d4d2ce46d9ea3`;
all 39 payloads and the exact 40-member roster pass independent checks. Its
fresh retention-only source checks are not relabeled as original guard passes.
No source or test executable is changed.
Earlier results below retain their original source and compiler identities.

R4 passes both preservation guards and the aggregate-reference test. The V4
test then reaches its scratch debugger build, where `rustc` receives SIGXFSZ:
the direct checker incorrectly applies its 64 MiB log budget as a process-wide
file-size limit inherited by compiler artifact writes. R4 remains exit 1, and
five later phases are unlaunched. Its scratch directory is removed normally.
The next runner correction will cap the two captured streams directly, leaving
the original stage/disk, memory, CPU and wall-time limits unchanged. The checked
compiler prefix now totals 2,556 test executions, not complete qualification.
The failed R4 archive is 356,412,776 bytes at SHA-256
`607b359b0fd36b0fbb04b398cbf63740548cc3a7a4f58a1351f8f00a297ad4a4`;
all 573 payload hashes and the exact 575-member roster pass independent checks,
including the 14 test executables and three normal child-tool binaries. The
R5 stream-capture correction passes six bounded helper tests on mi300x:
separate streams and nonzero exit, exact-cap output, overflow on each stream,
TERM-ignored overflow requiring KILL, and an 80 MiB sparse artifact write with
unchanged inherited file-size limits. The temporary artifact is removed and
all downloaded probe files match remote hashes. Its result SHA-256 is
`e08aa7580fe7ef0c33aa3da20dd08e4ea49132c8e1b009b94bb174638d83ff21`.
The R5 continuation passes aggregate V4, aggregate V5 and control-flow V6,
bringing the checked 8af prefix to 2,559 test executions. Tutorial checks pass;
strict changed-package Clippy then reports the oversized nested V4 verifier
error addressed by the later correction above. R5 remains exit 101, and final
compiler-tool qualification is not established. Its 356,562,561-byte archive
has SHA-256
`1852a74f07a110fa2fbed07ccab60bfe9305b0d9fed4f61838e340f3d0a722af`;
all 714 payload hashes and the exact 716-member roster pass independent checks.

On `71bd407e`, the separate host-kernel campaign passes all 19 phases: 14
target-neutral GEMM tests once, eight TP tests per gfx942/gfx950 target, and
strict aggregate Clippy with `--all-targets -D warnings` for both targets.
All 30 test executions pass with zero failures or ignores, using seven fresh
test executables. Full source and earlier dependency evidence remain unchanged.
The retained 83,210,265-byte archive has SHA-256
`443492f47fbf6f260c7e87a3925764407b573ec077992d05872346f277538591`.
All 1,834 payload hashes, the exact 1,835-member roster, and the separately
retained terminal retainer logs pass independent local checks. Removing only
the redundant host-kernel and compiler-R1/R2 unpacked verification copies
reclaims 1,418,244 KiB; their verified archives and manifests remain retained.
This closes those host regressions, not kernel emission, numerical GPU
qualification, native replay or performance qualification.

Earlier private integration `6c34cbe1` builds on the `5bcf44ed` baseline, which pins
the tested fe2o3 checkpoint `55d9bfe5` across all 80 active pin-bearing files.
Upstream adds enum SSA edge transport
and authenticated-downcast fixes, plus tutorial source-binding corrections;
the compiler lockfile and device/host/KFD trees are unchanged from e3. The
combined MFMA source passes all 32 locked/offline graphs and 42 freshly built
source-gate tests on mi300x. Cargo regenerates exactly the missing normal
`ferric-engine -> fe2o3-hsaco` edge in four standalone locks. The three scoped
and combined dependency inventories change only their compiler revision; the
runtime inventory is byte-identical.

During validation, upstream main advances to `179340e3`; it is freshly fetched
and reviewed, not yet adopted or validated by Ferric. Its nine commits change
47 files but no manifests, lockfile, KFD, host or device APIs. The materialized
compiler owner and ranked-assertion paths change, so actual aggregate emission
needs fresh checks even though no direct Ferric API break is found. Existing
55-pinned results are not relabeled as latest-main evidence. New optimizer and
V12 lowering primitives do not establish production admission or performance.
The next fetch reaches `f0bce59e`, adding one six-file compiler change for
aggregate components at exact source uses. It changes no production manifest,
lockfile, KFD, host or device API. Fallible reaching-definition analysis now
precedes the no-reference lowering branch, so ordinary Ferric kernels also
need fresh emission validation. This newer main is reviewed, not yet adopted.
The subsequent fetch reaches `8af54567`: two further commits add checked
optimizer policy 3 and harden receipt/lifetime boundaries in kernel-opt and
Pliron. This increment changes no manifest, lockfile, KFD, host or device API.
Ferric's dependency adoption is validated above; actual compiler tests, tool
builds and emission remain distinct requirements. No optimizer performance
improvement is inferred from source adoption.

The first source-gate run genuinely reports 41 passes and one failure: its
marker-order fixture anchor now occurs in both rosters. Fixture successor
`358cf5e7` independently mutates each named roster and requires that roster's
exact rejection, preserving the one-occurrence guard and production validator.
The failing executable and raw result remain retained alongside the new pass.
Full precheck evidence contains 54 terminal phases (53 passes, that one preserved
failure), source and four actual executable copies. Its 31,509,791-byte archive
has SHA-256 `e41df61f2fdd34aed60b545d7c0d0f4eab46c278d6c906a758f6098599b4ba99`;
all 1,670 payload hashes and all 1,145 final local source hashes pass independent
checks. This is dependency/source-gate validation, not full engine, Verus,
compiler-tool, kernel-emission or GPU validation.

The combined host R1 campaign on exact source `5bcf44ed` completes 17 substantive
phases and all 20 before/after source checks successfully. Its six test suites
report 1,141 passes, zero failures and 14 ignored: Qwen 81, engine 740,
qualification capture 85, adapter library 110, adapter capture 87, and source
policy 38. The actual batch-return regressions and new MFMA strategy tests pass.
The next phase, the gfx942 aggregate library test build, fails with E0433 because
the new row-coverage test uses `std::vec!` without importing `std` in a `no_std`
crate. Nineteen later phases remain unlaunched; the campaign remains exit 101,
not a full host pass. All source, native executable and control after-checks pass.
Its 105,926,364-byte archive has SHA-256
`60cbb48043e2647f0b392b77a19984627fb48386d812a8d8339ba56a9d321d44`;
all 513 payload hashes, the exact 514-member roster and all six actual test
executable bytes are independently retained and checked locally.

Successor `f9def4c0` adds only a test-module `extern crate std;` plus spacing;
production kernel bodies are unchanged. Its R2 continuation passes all eight
aggregate build/run pairs: 54 tests across gfx942 and gfx950, zero failures or
ignored tests. Together R1/R2 cover 1,195 passes and 14 ignores, attributed to
their separate source commits. Core strict Clippy then finds one redundant
catalog closure; the three later Clippy phases remain unlaunched. R2 stays exit
101. All 528 payload hashes and its eight actual test executables are retained
locally in the 181,816,253-byte archive at SHA-256
`29861cd3a7d507f58e1e10bb141e1eb8a304425904beb8ab060b8196c7149800`.

`2f1dded1` fixes that projection and `538aa053` replaces private-field references
in four crate-visible retired-page accessor contracts with closed spec getters.
R3 builds a distinct engine test executable and passes eight catalog tests and
three batch-return regressions. Its explicit engine-only codegen-units=1 setting
is host regression evidence, not a performance build. Source, prior evidence,
native executable and control after-checks pass. Strict Clippy reports two
ignored-unit-pattern errors in the invocation-only test fixture; `873f4958`
fixes them without lint suppression. R3 remains exit 101 and the later three
Clippy phases are unlaunched. All 122 payloads and the actual executable are
locally checked in the 209,362,975-byte archive at SHA-256
`36c2d719e509f1f2568975a7e23c09512b7d17a43c789205c66c0f038f8501df`.

Strict Clippy R4 on `873f4958` completes all four phases with classified raw
diagnostics. Core engine/spec/Qwen passes. The adapter reports one test-module
ordering lint; `2108e736` moves that module without changing its contents or
production bodies, and remote edition-2024 formatting passes. Its rerun remains
pending. Each architecture reports 103 aggregate-kernel style diagnostics.
Several arithmetic and finite-range spellings are pinned by source contracts;
narrow function-level exceptions are being reviewed instead of silently
rewriting the compiler-facing bodies. R4 remains exit 101. All 100 payloads and
the exact 101-member roster are independently checked in the 218,719,868-byte
archive at SHA-256
`aa94caa28aab43cdf7c8c42e34809bffc115f7b6d80fa635e2408f969c753095`.

Kernel successor `d034c7c2` adds eight function-scoped lint exceptions across
six aggregate-owned files, preserving every device body and source contract.
Remote edition-2024 formatting passes. Host R5 passes adapter strict Clippy;
both aggregate targets advance past the 103 kernel-body diagnostics and report
four host-only logits reference-test lints each. `1c71d9f1` fixes those test
assertions and iteration without suppressions; formatting passes, rerun pending.
R5 builds a fresh gfx942 source-contract executable, but its producer checker
rejects legitimate verbose Cargo build-script lines before executing it. This
is a harness failure, not a compiler failure or a test pass. Overall exit is 1;
all source, native, prior-evidence and control after-checks pass. Its 103 payloads,
104-member roster and unexecuted ELF are independently checked locally in the
233,647,765-byte archive at SHA-256
`e3081d0b28729ae2868381a25a821f3e2db0c993ec6ab7ae560864a52405c7f8`.
A separately named same-source continuation accepts only the exact
source-matched build-script messages. Both aggregate source-contract suites
pass 11 tests each. Standalone GEMM then reports 10 passes and four failures:
its source contract still counts three roots and two matrix roots after the
MFMA addition. The five later suites are unlaunched; exit remains 101, and all
source/native/prior-evidence/control after-checks pass. The original failed
producer receipt and both Clippy failures remain unchanged. This continuation
is host source-contract evidence, not kernel emission or GPU validation.
Its 253,398,168-byte archive is checked independently locally at SHA-256
`ce4159f53108afc5b13728299cd68ec798d18b835995a81ca85a7da5c0a2b06b`,
including all 122 payloads, the exact 123-member roster and both new actual
test executables. Successor `53eaf54e` updates the exact four-root GEMM roster,
three matrix ABIs, finite-divisor/grid counts and MFMA generated-adapter type
coverage. Scalar/A4 recurrence checks and production bodies remain unchanged.
Remote formatting passes; the test rerun remains pending. Separate TP ABI
review finds 14 available source roots versus 13 explicitly selected markers.
Test-only successor `eab370bb` checks all 14 available roots separately from
the exact 13 selected contract markers, including uniqueness and the sole
unselected MFMA root. Production TP exports are unchanged. The later
`71bd407e` host-kernel campaign above passes the TP and GEMM regressions.
An actual compiler emission check remains pending; host roster checks are not
evidence that the emitter selects the intended programs.

The actual current-55 batch consumer has now been attempted in Verus. R2 on
`5bcf44ed` genuinely verifies seven dependency exports but fails engine
translation on four opaque-datatype field references, before consumer SMT work.
After the accessor fix, proof R3 on `538aa053` preserves those seven exports and
reaches the selected consumer's solver obligations. It fails at the final
prefix postcondition and remaining-ticket frame assertion; nonzero solver work
is not a proof pass. Before/after source, dependency, tool-closure and artifact
checks pass. The full 183,762,226-byte failed R3 stage is retained locally at
SHA-256 `fb16cc8bbe4ce52c62272be106c15b805d782f23320a7244956c2b56d836c9c3`,
with all 3,734 members and 3,155 file payload hashes checked. `ea529fd8` adds
17 ghost-only lines connecting entry-state ledgers and explicit per-ticket
frames. Its R4 retry reaches actual SMT but still fails the final and next-prefix
obligations and hits the solver rlimit; the former remaining-ticket assertion
is no longer reported. All seven verified dependency exports and prior raw
evidence remain unchanged. The full 193,456,937-byte failed stage is retained
locally at SHA-256
`e50191a61a5bdf8b09bf25e5a94a2b3988c6f3cf30133377a4aa9383a3de945c`,
with all 3,885 members and 3,295 file payloads independently checked. Explicit
entry-vector links and localized pointwise prefix reasoning in `f29108e4` reach
SMT in R5, but the loop still exceeds the unchanged solver limit. The absence
of the earlier individual diagnostics is not a proof pass. The full
203,221,418-byte failed stage is checked locally at SHA-256
`8f979eca3c6c189ea4e64eb22bc63f1be16edd4e96a30853ff0f26eb7371c500`,
with all 4,041 members and 3,436 file payloads verified.

R6 on proof-only source `869ac60c` narrows predicate expansion but fails Verus
translation because the hide headers follow ghost bindings; no SMT work runs.
All seven verified dependency exports and prior evidence remain unchanged.
Its 212,873,273-byte archive is independently checked locally at SHA-256
`b67b866cffb54d7fa0c8bec42c9bd0ba00a34e71a4097ce6a20b3610abd696db`,
with all 4,190 members and 3,575 file payloads verified. Proof-only successor
`fb020d0e`, integrated as `08a9e3ae`, moves just those two headers to the start
of the function body. R7 reaches SMT and fails the freed-prefix assertion
with nonzero solver work; it is not a proof pass. All after-checks pass, and
the 222,559,843-byte archive is independently checked locally at SHA-256
`1eaa158b6c08056c91824c31d3530a843aeb5d641ed5a3126007357f974d868e`,
including its 4,339 members and 3,714 file payloads. Proof-only `cde4faaa`,
integrated as `a6aebf1c`, adds explicit consumed-lease and current-slot update
connections. Remote formatting passes. R8 succeeds with actual nonzero solver
work for `commit_page_return_batch_ledgers`: verified 2, errors 0, selected
exec success true, rlimit 6,300,370. Fresh root VIR/rmeta, the sole engine
producer and all source/dependency/artifact/control/history after-checks pass.
This is a selected-consumer proof, not verification of its unselected helpers
or the complete caller/pool/queue composition. The full 236,117,213-byte R8
archive is independently checked locally at SHA-256
`a9cb4fbe02832e10ebcba1c94c4391dce7f6ddc970f6579e9bcdc569e7fc27c5`,
including all 4,510 members and 3,875 file payloads. All three temporary proof
worktrees are removed with their branches preserved. No trust
annotations, executable behavior changes, weakened contracts or raised solver
bounds are introduced.

R9 on the same proof-only `cde4faaa` source succeeds for all 14 selected
executable functions: the nine return-family bodies, four retired-lease
accessors and `global_page_index_core`. Every selected exec has successful
nonzero solver work. The seven dependency exports and all prior raw evidence
remain unchanged. This expands R8's selected-consumer coverage, not the outer
pool/queue composition, negative mutations, changed maintenance checker or
native qualification. The full 250,915,254-byte archive is retained locally at
SHA-256 `8596f98ac1c83db0f63499386f1daed2a299fcf7f4881816a118e4434e93e1c1`;
all 4,132 regular-file payload hashes pass independent local checks.

Scoped reproducible-cache cleanup removes 23 obsolete engine/build libraries
and 11 obsolete engineering-adapter libraries, reclaiming 1,041,180 KiB while
preserving protected sources, artifacts and evidence. A separate no-use scan
fails on a transient unreadable process and deletes nothing; the fresh retry
passes after that process is observed absent. A later exact 19-file kernel/spec
cache cleanup reclaims another 78,620 KiB with all 986 protected inputs unchanged.
Its complete inventory and deletion journal match locally. The runtime stage
after host R4 is 9,420,480 KiB, still below its 10 GiB cap; future launches must
restore their required initial reserve rather than weaken the resource guards.
A further exact 56-file obsolete-library cleanup reclaims 265,632 KiB while
preserving 1,081 retained payloads and all 2,917 unselected dependency entries.
Its first inventory attempt stops before deletion because a JavaScript JSON
round-trip rounded nanosecond timestamps. The corrected R6 roster carries exact
remote Python integers without numeric transformation; inventory, separate
delete admission and the 56-row journal all pass. Runtime usage after host R5
is 9,275,436 KiB; the 10 GiB cap and per-phase reserve remain enforced.
After the same-source continuation, runtime usage is 9,387,684 KiB. It remains
below the cap but lacks the full-source launch reserve; a new full-source host
campaign needs scoped cache cleanup before launch.
Removing only the two completed local unpacked verification copies for R7
and the host continuation reclaims 973,488 KiB; their independently checked
archives, manifests, raw results and executable payloads remain archived.
The earlier survey found all eight mi300x GPUs occupied. mi350-2 had one physical
gfx950 GPU at zero utilization but a foreign process still holds three queues
and 17.7 GiB, and its kernel/driver profile is not admitted. A fresh mi300x
survey at 03:08:54 UTC finds GPUs 1-7 idle with no selected-device queues or
VRAM users; the foreign GPU0 workload is untouched. Latest-f0b tool refresh,
current-source emission and GPU numerical validation remain pending.
All 33 M1 gates stay open.

Runtime metadata inventory R7 stops on an FD race in the owned native child and
deletes nothing. After the native and proof jobs terminate, namespace-only R8
passes. Separately admitted cleanup removes exactly 11 obsolete metadata files,
reclaiming 255,032 KiB. All 1,290 protected file payloads and native bytes are
checked before/after, and 2,984 unselected dependency entries remain unchanged.
Measured runtime usage is 9,134,984 KiB, restoring the unchanged full-source
launch reserve. Compiler-cache inventory R1 instead stops before deletion on
one empty metadata file; its strict positive-size assumption requires an exact
file-scoped successor, not a broadened cleanup roster. R2 admits only that
exact empty historical test metadata file and succeeds for 228 obsolete files
(86 rlibs, 142 metadata files), totaling 1,635,144 KiB. The actual inventory
has SHA-256 `a85f2a8adf9ec8a4db704e89fb3118e06726722401614cb136eeefa587f256b8`.
Retention R3 subsequently archives all 228 payloads in 395,853,185 bytes at
SHA-256 `5e964011bf01df98b6da4deff2a91c81e2de068711ca3ccc6dea12f774101d34`.
After independent local archive, payload and metadata checks, separately
reviewed deletion R4 removes exactly those files and reclaims 1,635,144 KiB.
All 168 protected current libraries, 1,720 unselected dependency entries,
source/tool bytes and R2/R3 evidence remain unchanged. Fresh no-use checks pass
under the existing explicit visibility exceptions; complete system process
visibility is not claimed. All 228 journal identities match the original
integer-precise inventory. The deletion receipt has SHA-256
`6d1a6dd3dbc4a156e141bdead84c7f0ad09154c631bf3cf2f738ba528d029e56`;
the measured compiler stage falls from 10,483,440 to 8,848,296 KiB.
The f0b drafts remain unexecuted; the reviewed 8af successor is launched instead.
Removing only R3's redundant local unpacked copy reclaims 1,637,964 KiB while
the verified archive and raw evidence remain retained. Removal of the two earlier
local unpacked proof/native verification copies reclaims 763,552 KiB; checked
archives, manifests and raw evidence remain retained.

Pages source `0f8516e1` prepares the R9 proof and native-maintenance checkpoint.
Actual Pages R9 QA passes structural/negative checks but fails a browser
assertion that still expects the replaced R4 overview. No publication occurs.
Test-only successor `38e3feb8` corrects that assertion and explicitly retains
MFMA opt-in/default and historical-boundary checks; public assets are unchanged
from R9. Its actual mi300x QA retry passes all five phases, including 1,121
widths, eight viewports and 64 screenshots. All 133 payloads in the 11,320,854-byte
archive at SHA-256
`c1a7b01695eb20e501cc9eb5c78793de3a93a481d53b6a6172c45b33be539a75`
pass local checks, and root reviews desktop/mobile overview and checkpoint
images. Static-only public commit `60f65896` deploys successfully in GitHub run
`35054007136`, with no build/test or protection changes. All seven canonical
and seven cache-busted live files match the 619,456-byte retained artifact.
This is the dated proof/maintenance checkpoint, not the later 8af pin result.
Both clean temporary worktrees are removed. The original R9 failure is retained
separately in 932,021 bytes at SHA-256
`eeb91c30c10607ebdacd63ec7e784602c4f783d03ec874b8bb29a336232c3be7`;
all 91 payloads pass independent checks, with QA exit 1 and no artifact or
screenshots. The failed run is not relabeled as successful by the R10 retry.

Pages-only source `3c7a4023` includes the terminal R4 outcomes and is published
as static-only commit `58ebfe67`. All five mi300x QA phases pass, including
1,121 widths, eight viewports and 64 screenshots. Root reviews four desktop and
mobile screenshots; all seven canonical and seven cache-busted live files
match the retained 616,600-byte artifact. GitHub run `35048733488` performs only
artifact admission and deployment, with no build/test or settings changes.
Private implementation ancestry is not pushed. The dated R4 site checkpoint
does not claim later R5/R6 results. The temporary Pages worktree is removed.
Separate reviewed cleanup removes the three completed remote Pages directories,
reclaiming 50,812 KiB after exact inventory, custody and no-use checks. All 445
deletion-journal rows and the full receipts remain retained locally; shared
caches and all runtime/compiler/proof stages remain untouched by that cleanup.

The dated Pages R5-R6 snapshot is deployed at public static-only commit
`f3cb38aa`; all seven live assets match the mi300x-validated artifact. Fresh QA
passes 1,121 widths and eight viewports after a scoped paragraph-wrapping fix.
GitHub performed only artifact admission and deployment, with no build/test or
protection changes. Private source `2e1fa34a` was not pushed; its completed
worktree is removed and its branch/evidence remain. Separate verified cleanup
removes three completed owned remote Pages directories, reclaiming 51,144 KiB;
the earlier failed QA directories remain preserved. No newer native result is
implied by that dated snapshot.

Private integration `07d718a6` brings the six existing MFMA feature commits
onto the current compiler-pinned source: the BF16 K16 device kernel, separate
thirteen-program roster, distinct numerical profiles, strategy propagation
through engine recipes and runner binding, and explicit engineering capture
admission. LegacyScalar12 remains the default and single-row decode retains
its scalar/vectorized path. No fe2o3 files are changed by this Ferric feature.
The 24-file replay preserves current e3 pins, newer runtime hardening and both
page-return proof files. Its temporary worktree is removed; the candidate
branch remains. Its successor's dependency and source-gate checks are described
above; the partial host result and test-only successor are described there too.
Complete architecture checks, current-source emission and GPU execution remain pending.
The source359e host controls remain a separate, not-yet-launched campaign and
must not be treated as validation of this larger feature.

Private integration `9e10cc95` coherently updates the 80 active pin-bearing
files to published fe2o3 `e3c359fb`; formatter-only successor `359e4e12`
preserves that dependency graph. The dependency overlay on `58833759` passes
42 remote phases, all 32 locked/offline graph checks and 38 freshly built
source-gate tests. Package versions and registry checksums are unchanged;
the expected two compiler analysis edges are included in 29 locks. The three
TCB documents are regenerated and independently checked. Full source, actual
source-gate executables and raw results are retained locally at SHA-256
`53d27228aa90f2bd80efeaa79cc23f28746b7a5b5a6b66180011c1412a6ec22f`.
These results validate the dependency overlay, not the newer engine body.

Candidate `8af1e52b`, included in these successors, moves the actual page-return
batch loop into a contracted ledger consumer and adds three host regressions.
The contracts state per-ticket correspondence, unique role/index returns,
remaining-ticket preservation, exact generation advancement and frames for
untouched entries. The existing forward zip truncation is preserved. The combined
source `5bcf44ed` now passes host compilation and all three new regressions within
its 740-pass engine suite. Verus verification of the actual batch consumer remains
pending; outer-caller completeness and native completion custody remain unproved.

The latest runtime executable is source `00ac6a22`, still pinned to fe2o3
`4f6f65ce`. Its optimized build and six successor host phases pass. The full
720-pass/9-ignored engine result belongs to its diagnostic predecessor `ba6f8c9c`;
00ac changes only three lifetime spellings. The source-bound 32-token R5 native
diagnostic stopped before inference on a process-observation race, not the
maintenance transition. No new performance result is available.

R6 reruns that exact frozen binary on idle physical mi300x GPU4 and reaches the
maintenance-publication check. It exits 134 with `step draft catchup custody`,
before submission of the 425 maintenance packets. The parent speculative
selection is deliberately retained by both the KV binder and pending owner,
but the step checker incorrectly expects the outer custody to use Decode.
Successor `6c34cbe1` compares it to the speculative draft selection instead,
leaving all completion-plan, request, epoch, width and ownership guards intact.
Nine new cases use real pending reservations across K4/K8/K16, accept the exact
selection and reject wrong-speculative and Decode outer selections. Independent
review finds no blockers. The test-only assembly helper grants no production
authority; a rebuilt native retry remains pending.

Focused host validation on exact `6c34cbe1`, still compiler55-pinned, passes
all 17 phases on mi300x. A distinct fresh engine test ELF passes 27 executions
covering 26 distinct tests: the nine-case regression runs alone and again in
the seven-test cache cohort, plus 13 readback, four completion and two dispatch
tests. No failures or ignores occur. Changed-line formatting, locked/offline
metadata, all 15 source snapshots, the 1,290 protected-file union, native bytes
and control after-checks pass. This is not full-engine, Clippy, Verus,
latest-compiler, GPU or performance qualification. The 36,213,090-byte archive
has SHA-256 `7632e0f0b2031c1c8ec5bf64b81281baf4d84bf82644ef406ec722719a826044`;
all 174 payload hashes and the actual test executable are checked locally.
Its temporary unpacked copy is removed, reclaiming 131,684 KiB. Measured runtime
usage is now 9,308,300 KiB; the next full-source launch must recheck its reserve.

R6 failure evidence is retained locally in the 3,845,968-byte archive at SHA-256
`c51d8cb5be5cfbe0970b2fee184cd8854bed862f5e2d101cfb152a1b84ef5043`;
all 22 native-stage file hashes and retained review files match. The owned
process group and observed child are absent, GPU4 has no queues or VRAM users,
and fresh device memory is back to its 298,647,552-byte baseline. No reset or
foreign intervention occurs. The report is absent, the failure is not relabeled
as fixed, and the active 128-token prompt fill is not a comparable benchmark.

Compiler `e3c359fb` is now published to fe2o3 main, rebased onto freshly fetched
upstream `2585ce64`. All twelve patches are unchanged by range-diff; the tip's
message contains `[skip ci]` under the user's explicit instruction to skip
GitHub CI for this push and keep all builds on mi300x. The normal, non-force
push completed successfully. At 23:48:06 UTC, GitHub reports the exact main SHA,
zero workflow runs and zero check runs for this commit. No workflow or branch
protection was changed. The published tree is
`1590682689fa240ebd0d10ccf63b488008c794f4`.
The new upstream adds internal scalar-helper-result LLVM struct lowering,
integer wrapping and launch-index fixes, a test-only simulator dependency and
regression module separation. All 19 R12 remote host phases now pass: 1,030 Rust
tests, zero failures, one existing physical gfx1151 ignore, nine dependency-policy
tests and the CI dispatch harness. Current-source artifact freshness and exact
paired V11/V15 BF16 exports pass. Independent review checked the retained raw
results, source identity, actual Cargo producers and byte-checked artifact reuse.
The full 13,891,856-byte source/result archive is retained locally at SHA-256
`e4334c6b7db87c97b494d8c70b0d1d7a8c86c6201b5780f5bbf9e916ac31ba48`.
This is a host delta, not a new release-tool, emission, strict Clippy or GPU result.

The separate e3-source tool campaign passes all four build/copy phases:
release CLI and linker proxy, then debug0 backend and extractor. Actual Cargo
producers are fresh, the backend binds the new CLI digest, source before/after
checks pass and earlier tool copies remain unchanged. Complete binaries, source
and raw phase evidence are retained locally in the 54,958,415-byte archive at
SHA-256 `996fb0367b417e7e6ab99b91e1e74dfce48b07c14210154dd70d54efefc22e0f`.
This does not establish kernel emission or numerical GPU results, and these
tools are not relabeled as the newer `55d9bfe5` source.

Scoped cleanup reclaimed 602,424 KiB from ten retained compiler test executables
and four obsolete tool copies, then 1,737,320 KiB from 349 obsolete runtime
library artifacts. All selected bytes were archived, downloaded and hash-checked
before separate deletion. Runtime stage allocation then was 7,766,132 KiB; current
sources, all 213 dep-info files, proof inputs and the frozen native executable
are unchanged. Failed retention attempts remain documented. The runtime retry
accepted only an independently audited exact lock-timestamp transition, not a
general metadata exemption. The two redundant remote cleanup archives were
removed after successful deletion and full local retention; receipts remain.

The initial CI harness attempt failed before compiler builds on an environment-
dependent argv expectation. Its continuation cleared only `LD_LIBRARY_PATH` for
that harness and preserved the six completed phases. Full prebuild failure
evidence is retained locally at SHA-256
`47740391eff1045047f7fbd243ee6159c312b2e243edd6e8a309f1f24ac63993`;
the earlier source9c2e R11 controls were never launched. The first result retainer
then rejected paired-test JSON interleaved with the named test line. R3 accepted
the exact framing while retaining all test checks and the failed attempt; no
compiler test was rerun for that correction. The published source archive SHA-256 is
`f0e6c827c1743a292b7c76a1b289f16fe0ca41328d3d8bf56bdc911cc8ea3689`.

Previous compiler `12845295` on `48323569` has all twelve patches unchanged
from `86ccc2a5` by range-diff. That exact predecessor passes its
14-phase R10 remote host delta campaign and has not been pushed. Full source,
raw results and identity checks are retained locally at archive SHA-256
`b3cfc86c7c0c6d7c858b4751d8d0a9197a77cb663c67a46cd028f6ce3e9cfaaa`.
Independent archive review found no source, raw-phase, selected-test, artifact
reuse or paired-export mismatch. Compiled executable hashes and producers are
retained; this archive does not contain those executable bytes.
Its source archive SHA-256 is
`80af541343a5474bbde0588f340c1882ac66df690f902b2026eb1cfa1699d7bc`.
The preceding `86ccc2a5` on `bd4d5f42` passes its 23-phase R9 remote host
campaign, including backend, lowerer, Pliron,
tile mapping, frozen MIR v13/v14, macro closure and paired BF16 export coverage.
Source-bound artifact freshness checks pass, and the older e0/5602 tools remain
unchanged. Full source, raw phases and receipts are retained locally with checked
archive SHA-256
`b530f254a2a0f26b25be7ee85ad471b73db8cd4cbf9b8fe147f1a59aa0220528`.
This predecessor is a delta host campaign, not a rerun of all earlier suites or
a new release-tool/emission result. The successor push described above followed
a fresh fetch and explicit rebase check. Actual remote API checks report main
unprotected and no applicable branch rules; no workflow, protection or check-
status changes have been made. The skip marker affects push/PR triggers, not
other event types.
Upstream `48323569` adds immutable-slice helper lowering. R10 passes backend,
AMDGPU and lowerer phases, their fresh artifact checks and paired BF16 exports
for semantic wire versions 11 and 15. The actual
wrapper-PATH assembler is Ubuntu LLVM 18.1.3 at `/usr/bin/llvm-as`, resolved to
`/usr/lib/llvm-18/bin/llvm-as`; its resolved identity and version match before
and after execution. The compiler-module suite has 26 passes and one existing
physical gfx1151 test ignored. Unchanged R9 suites were not repeated; no new
release-tool, emission, strict Clippy or hardware result is claimed. The frozen
native executable still uses `4f6f65ce`; no newer compiler result is attributed
to that runtime. The private source dependency refresh is described above.
The frozen validated compiler is `e0d108b2`, based on `2179a6f4`, whose upstream
changes refresh the local macro-tree pin and add simulator binary32 sqrt support.
Its expanded 42-phase remote host campaign passes, including the
actual paired BF16 exports, macro closure tests, 169 differential simulator rows,
and production-source sqrt simulation for gfx942 and gfx950. Full source and raw
phase evidence are retained locally with checked archive SHA-256
`0061dda1a6e6347b27ad3a2a25e6dea4fb5a023e95c570d6e8e901c9c4280af9`.
The prior 27-phase campaign on `2e9ecd23` / `638e1169` remains separately retained
at `d0ce5dfde70467ffaa3c9fc03655323bd409cdf34627653f7bf6a284d96f39f7`.
No strict compiler Clippy pass is claimed by this campaign.
The completed 24-phase host result belongs to predecessor `06e25566` on
`d2e2ff47`, including artifact freshness, backend, lowerer, Pliron, analysis,
optimizer, MIR-model and paired BF16 fixture coverage. Neither result relabels
the preserved older emitter tools.
Cleanup removed 50 completed predecessor test executables
before the new campaign. After its evidence retention, another 48 completed e0
test executables were removed, preserving all 299 protected non-test artifacts.
The two obsolete owner vendor trees are now fully archived locally and their
contents removed, reducing the stage to 10,347,740 KiB and admitting the release
tool campaign under the unchanged 10,485,760 KiB prelaunch threshold. All four
tool build/copy phases pass: release CLI and linker proxy, dev-debug0 backend and
extractor. Actual Cargo producers are newly built, and the backend build script
binds the exact new CLI hash. The full source archive, four tool binaries, raw
phase logs and source-binding receipts are retained locally at SHA-256
`a251638717b58a101c1d0d710c867068be72e8e7102fdc35fc5ea315c57c97f9`.
After this build the stage was 10,688,940 KiB. Subsequent scoped cleanup removed
24 byte-retained obsolete compiler tool/input files, reducing the measured stage
by 845,388 KiB while preserving all six empty roots and current tools/source.
The frozen `e0d108b2` / Ferric `ce2d3c1f` thirteen-root campaign then passed all
13 preparation, emission, after-check and inspection phases on mi300x. The
113,192-byte engineering HSACO has SHA-256
`02c65b859fef7fac778305918a543d215b723f99bf26831210b5df74fa350856`.
Inspection found exactly 13 entry/descriptor pairs, exact output replay, and an
actual `v_mfma_f32_16x16x16_bf16` instruction in the MFMA GEMM kernel. This is
image emission and read-only inspection only: publication/load/launch grants
remain false, no GPU ran, and no numerical or performance result is claimed.
The full frozen emission input/result archive is retained locally and checked at
SHA-256 `f05db35b45eccd4de06653c97b6a277988447f1f66c8cf6218662787d88fb049`.
The active remote compiler source has transitioned from `12845295` to
`e3c359fb`; the e0 emission
inputs and tools remain separately frozen and are not relabeled as current.
Before R10, 15 completed R9 test executables were fully retained and removed,
reclaiming 528,736 KiB. Their archive SHA-256 is
`ee3112d2d3b8a0e86d3a2257f1d1430b89f7004560ce6eed98e6d18813772562`.
The first cleanup retention attempt failed before archival or deletion on an
overbroad process-warning classification; its logs remain retained. The second
attempt used scoped candidate-inode checks and completed both retention and
separate deletion. No release tools, current sources or dependency libraries
were deleted.
Before R12, ten completed R10 test executables were fully retained, then removed
along with two duplicate source-input archives whose complete bytes were already
retained locally. Exact identity/no-use checks passed and 425,192 KiB was
reclaimed. The ten-ELF archive SHA-256 is
`dd2555b5279b1b8b47f641ad2b20f8ec2b96f8f3d0ff4e8e2f450b9a5a6cbaba`;
the deletion receipt SHA-256 is
`31c891c5f36c5d840ef52f54d9ae64703ec6db2a11abcfa4ba4f6fb52501ce31`.
Current source, dependency libraries, fingerprints, retained evidence and
compiler tools remain intact.

The retirement proof reached Verus on `339c2561` and failed at its pre-update
snapshot equality. Trigger-only successor `bd8e22f7` passes the selected commit
body and loop, then all five current-source preflight/dependency bodies pass.
The six source-bound results total nine verified queries and zero errors under
pinned Verus `b677dd5`. This is scoped metadata verification, not whole-crate,
engine-composition or native-authority proof. All 24 actual-body mutations now
produce checked semantic verification failures. R16 accepted the fifth retained
bounds-precondition diagnostic without rerunning it, then completed the remaining
nineteen cases; the continuation exited zero. The first-five raw archive is
retained locally and independently reviewed. Full six-positive/24-negative
retention also exited zero; root downloaded and checked archive SHA-256
`b3c0760fcc2480913b2fdc72cc2167de68f4049dc06f1482974d38a5d94524e8`.
The ten single-file proof commits are now integrated at Ferric `78c9c9f0`;
that checkpoint's spec file matches the verified `bd8e22f7` bytes exactly. Its
expanded remote host campaign passes all eight phases: format, exact locked metadata, 130 spec
library tests, four spec doctests, engine library check, 721 engine tests with
nine ignored, and strict all-target Clippy for both crates. Source-before/after
checks and the frozen native binary identity also pass. Full evidence, including
the actual test binaries, is retained locally with checked archive SHA-256
`4df5ca07179c43a3947e6130814db351ea17014dc6ccce7ec323a875f7f97cf3`.
Candidate `a7d58129` adds contracts around the actual pool-return helpers and
two host regressions. Its host run passed formatting and metadata, then failed
engine compilation on an unparenthesized enum struct literal in a Verus
comparison. All three source after-checks and the native binary identity passed.
Successor `457ca0aa` changes only parentheses around the three affected contract
comparisons. It passes formatting, locked metadata, engine compilation and both
new regressions. The first run stopped at the unchanged 1 GiB link reserve after
its first test; R3 resumes the same source and exact test binary without repeating
the completed builds. The continuation passes 717 engine library tests, nine
expected ignored cases, and strict all-target engine/spec Clippy. Both test
phases reuse the original source-bound executable with unchanged SHA-256.
All source after-checks and frozen native identity checks pass. These host
results belong to the older helper branch. Its exact engine contracts are now
integrated and validated separately at `8fa40b3a`, as described below.
The combined R2/R3 source archives, raw logs and exact test executable are retained
locally at SHA-256
`8c16063a5347595b4c7de05d823f92ab485fa1bf062bc2f849f1a8d55a920514`.

The actual selected-page successor lemma at `457ca0aa` passes pinned Verus:
one proof-mode query, zero errors, exact source and verifier closure checks.
Its raw output and full source archive are retained locally at SHA-256
`840a54180641bc211e005b3307ab937f9df1538ad6af99ddbdb8852762b9e27d`.
This pure proof lemma was integrated at `54cdb41a`; the entire spec file is
byte-identical to the actual `457ca0aa` proof input. Newer integration engine
work remains intact; the later helper integration changes only `device_cache.rs`.
This establishes only its stated metadata implication; whole-roster/native
composition remains unproved. The first genuine engine helper campaign now
passes pinned `cargo-verus check` on the actual `457ca0aa` source, with dependency
verification and a fresh dedicated target under one-CPU affinity. Its selected
`ferric_engine::device_cache::returned_page_state` exec body has one verified
query, zero errors and the sole nonzero function solver count. Zero-work rows
for other helpers are not counted as proof passes. Source, dependency graph,
Cargo configuration and verifier closure comparisons all match afterward.
Full source, genuine target artifacts, raw logs and controls are retained at
SHA-256 `be4fb393ec0f16c36c828f95bb71f4782b260c7aa0718c5c13cfa0660ebe9ac9`.
The preceding R1 attempt stopped on a snapshot allowlist omission; R2 stopped
on cargo-verus argument ordering before verification. Both failures are retained
separately. No stub or ordinary host artifact substitution was used, and this
is not an entire-engine or caller-composition proof.

R4 now proves the other five actual engine helpers on `457ca0aa`:
`page_return_device_matches`, `page_return_request_matches`,
`page_return_role_matches`, `preflight_page_return_identity` and
`commit_page_return_state`. Each selected exec body reports one verified query,
zero errors and nonzero solver work. Root artifacts are freshly generated for
each selection; all 284 dependency artifacts, source and verifier identities
remain unchanged, and R3 evidence preservation passes. Full R4 evidence is
retained locally at checked archive SHA-256
`0f4f391d77a5d35504ed725f914df19bb86d943b047228231a67f9bc984c1d26`.
All six helper bodies now have actual proof results, but their actual-body
negative mutations and caller/whole-roster/native composition remain outstanding.

Earlier integration `8fa40b3a` applies the exact verified helper engine bytes
without merging the older branch. All eight remote host phases pass: formatting,
locked metadata, engine check, both exact regressions, 723 engine tests with nine
ignored, and strict all-target engine/spec Clippy. All four source before/after
checks and frozen native identity checks pass. The full evidence archive includes
the exact test executable and actual Cargo producers; local SHA-256 is
`450e70c1efc74fb5648c9ab58a0c0a748718cd08ec8e4af6e38e88f946714c2f`.
This does not rebuild or relabel the native executable or change the fe2o3 pin.
Global-index candidate `79f74a3e` extracts the actual production core with bounds
and error contracts plus host regressions. Its actual R5 run fails parsing an
unparenthesized cast comparison before SMT: exit 101, zero verified queries.
The full failure archive is retained locally at SHA-256
`3785d303da9fc5c1cca0bcabe8cab6a968a56c639140c5a0f7b86006a9b1f2a1`.
Parentheses-only successor `74c52a73` passes R6 for the actual
`global_page_index_core` exec body: one verified query, zero errors and nonzero
solver work (rlimit 154038). Source, dependency and verifier checks pass; all
284 dependency artifacts and prior R3/R4/R5 evidence remain unchanged. The full
R6 archive is retained and independently hash-checked locally at
`311aea52da61ba959640db89afb08a1b00da22f8b91559ef6352e92ca52b8317`.
This proves the selected core's generation, slot/index bounds, flattening and
error-precedence contracts, not its native caller or whole-roster composition.
All seven body-only negative mutations now pass the CPU-only R7 campaign.
Generation/slot/page bounds, rejection of a valid page, row stride, page offset
and error precedence each produce an actual selected exec postcondition failure
with nonzero SMT work. Parser, compiler, translation and resource failures are
not accepted. Each mutant is restored to source74 bytes/mode/mtime; intentional
inode/ctime changes are recorded, and failed root exports are quarantined.
All 284 dependency artifacts, unmodified source, verifier/configuration closure
and prior R3-R6 evidence remain unchanged. Full retained R7 evidence is
179,774,359 bytes, locally hash-checked at
`6aed4783947f5556a67f414b7eb563afe366819dc421063022b3e5c7a3d56b28`.
This closes these seven selected negative checks, not an M1 roadmap gate.

Exact proof-source engine bytes are integrated at `2aae5839`. Its R1 host run
stops at rustfmt on two test assertions, before compilation or tests. All five
source after-checks and frozen native identity checks pass. Full failed evidence
is retained at SHA-256
`b556ab277af2f3c134ccf053c56229f9d390fab08aa9e038d6ed0d2e8b42f234`.
Current integration `21818e2a` applies only the formatter's two assertion wraps;
its production prefix remains byte-identical to proof source `74c52a73`.
Its separate R2 host campaign passes all eight phases: formatting, locked
metadata, engine check, both exact regressions, 724 engine tests (nine ignored),
and strict all-target engine/spec Clippy. All six source before/after checks
match, and frozen native/proof identities remain unchanged. The exact newly
built test executable is reused by both regressions and the full suite.
Full evidence is retained and independently hash-checked locally at
`4dc705e003d66605aeb10d823487e90b4735bb8abbcc63409821cf406af70427`.
The earlier 723-test result is not relabeled, and no new native artifact or
proof execution is claimed for this host campaign.

Separate candidate `efa069bd` adds actual production ledger preflight/commit
helpers with exact role/index selection and complete two-ledger frame
postconditions. Independent review found no production regression and caught a
test-discrimination gap, fixed before freezing the candidate. It is Contracted
and not integrated. Its exact-source host run passes formatting, locked metadata,
engine check, both new regressions and 726 engine tests with nine ignores, then
fails strict engine Clippy on `manual_map`. Spec Clippy was not reached. All
seven source before/after checks match, with native and proof inputs preserved.
The complete failure evidence and actual test executable are retained locally at
SHA-256 `5713237da7943871551c8c1894e820cd45f4ae0170194e445b5ad0b36f64bf99`.
Its source archive SHA-256 is
`43077663e18600541784643dea5c0e7155854f1d59ee82be7117a9ff91b08977`.
Successor `47d07cee` changes only the optional state read to an explicitly
bounds-checked slice index, without a lint exception or contract change.
Independent review found equivalent behavior and error ordering; remote
formatting matches. Its fresh host R2 run passed all eight source-before checks,
formatting, locked metadata, compilation and both exact ledger regressions.
The root SSH session ended with transport exit 255 during the full engine suite,
and two reconnect attempts initially failed. Connectivity subsequently returned;
read-only recovery found the original campaign terminal at exit 0 with all owned
groups absent. No rerun was performed. All eight host phases pass, including
726 engine tests with nine ignores and strict all-target engine/spec Clippy.
All sixteen source before/after checks and aggregate equality pass. Full source,
raw phases and the exact newly built test executable are retained locally at
checked archive SHA-256
`4e49bda4a4fd73e09aec4feedafb57a152bbb567f3cf8c2ce477d5d2706276fd`.
The earlier efa test cohort and failure remain separately labeled. Its source
archive SHA-256 is
`6670dd515627fa70c32a6e41aa2d38d51605dace5842a0d15ec3841d631523e3`.
Prepared R8 proof controls were never launched; their R9 successor targets the
corrected source and started only after host completion and full local retention.
R9 has transitioned the dedicated proof source from 74 to 47 with exact six-file
checks; the other 1,139 source files, dependencies and verifier closure match.
All nine selected actual-body proofs now pass: each has one verified exec
query, zero errors and nonzero SMT work. Source, all 284 genuine dependency
artifacts, verifier closure and predecessor evidence remain exact. Full R9
evidence is retained locally and independently hash-checked at
`9edd3b9ecbdb8587a22f2ba26a673b040e624514d9d6c4f56e679baab1aacab8`.
All six actual-body R10 negative checks pass on mi300x: wrong preflight role
and index, wrong commit role and index, missing generation increment and an
extra write each fail the intended selected postconditions with nonzero SMT.
All restorations and final source/dependency/configuration/closure checks pass.
Full evidence is retained locally at SHA-256
`6b7e66b6501ee62d3700835d2585cfcaf3928273e249daf03ae1dd2bec0c8170`.
Independent review found no acceptance blocker. The two device-cache commits
are integrated at `66e32993`; its device-cache bytes exactly match source47
SHA-256 `e9a426580f8e116a5d3bd860046a333ca62c410d7d73df385af01ffa51fc574d`.
At that checkpoint only the tracker differs from the full tested source47 tree. Host and proof
receipts remain attributed to source47, not relabeled as a new full-tree run.
Higher-level composition remains pending.
Before R2, exact retained-artifact cleanup removed the obsolete R1 test
executable and thirteen retired source archives, reclaiming 179,180 KiB on
mi300x. All fourteen have checked local byte retention; current sources,
required archives, dependency libraries and shared caches were preserved.
The wrapper's completion custody and cross-call ticket/ledger continuity remain
outside these helper contracts. That campaign changed no fe2o3 pin or native
executable; the later source dependency update does not relabel its results.
All 33 M1 gates remain open.

Routing-test source `45a211e5` passes remote format, exact locked metadata,
engine library check, the single 425-row enter/restore regression and strict
all-target Clippy. Both frozen source trees and the native `00ac6a22` executable
remain unchanged by the run. All 25 actual mock GPU-guard fixtures also pass.
R6 native controls preserve those tested guard bodies, but no launch occurred:
the fresh device census found all eight mi300x GPUs at 100 percent busy.

| Team | Current Work | Next Evidence Required |
| --- | --- | --- |
| Compiler / Kernels | Published core-only `111722028` has zero GitHub runs/checks. The resolver successor on main `9af22bb5` passes 3 ABI tests, 30 expression regressions and the unchanged real-source positive to the unavailable-proof-runtime boundary. R4 remains exit 101 because strict Clippy reports 17 diagnostics at unchanged upstream sites. Full evidence is retained; no new push or Ferric adoption. | Prepare/review a clean core-only public diff, preserve excluded failed experiments, restore the unchanged build-space reserve, and rebase/revalidate before a CI-skipped push. No proof admission or kernel numerical qualification is established. |
| Runtime / Integration | Exact `2bfce38b`/`111722028` native32 completes 20 speculative rounds, full-accept catch-up and clean teardown. The current111 prefill comparison matches the next token but has max absolute logit error 0.125. Combined M5/catch-up source `568882e2` passes 21 host349 phases and is integrated privately at `6b5322dc`. | Investigate numerical differences, execute actual five-position comparison, broaden continuation/fault coverage and obtain matched TTFT/TPOT/throughput measurements. |
| Verification | Exact `a9fad31b`/111 verifies the production catch-up transition and all nine actual-body mutations fail intended postconditions. Fresh combined `568882e2`/349 separately passes all 14 proof phases and verifies that same selected method with genuine dependency exports. Both campaigns are fully retained; old negatives remain111. | Prove outer queue/caller and physical KV composition separately, preserve exact compiler attribution and obtain native validation of the combined capture path. |
| Integration / Pages | Private integration now includes the host-tested M5/catch-up source on349. Public static `3e41846` reports the dated native/numerical/proof checkpoint; all 14 live asset checks match mi300x QA. Completed prefill scratch and obsolete111 host-cache cleanup reclaim 350,568,448 and 1,427,656,704 bytes after retention. | Keep dated public scopes truthful, deploy only validated prebuilt assets and remove only completed owned stages/worktrees. |

The upstream freshness audit found no service-host, KFD or runtime source changes
from Ferric's old `4f6f65ce` pin through published `e3c359fb`. The coherent
dependency refresh now covers all 80 pin-bearing files: 30 manifests, 30 locks
and 20 policy/source/TCB files. The complete workspace census covers 42 manifests
and 32 locked graphs; the two new compiler analysis edges are included. The frozen native
artifact is not relabeled as using the newer upstream revision.

The user also authorized `mi350-2` for native validation. Read-only inventory
finds one physical MI350X/gfx950 (unique ID `10294934887855545115`), not eight;
the additional render nodes are XCP auxiliary devices. At 23:06:37 UTC it had
zero utilization but foreign UID12634/PID862305 held three KFD queues and about
17.7 GiB VRAM, so it was not admitted as unused. Existing gfx950 admission in
both latest fe2o3 and Ferric's older pin accepts kernel `6.8.0-124-generic`, but
this host reports `5.18.2-mi300-build-140423-ubuntu-22.04+`. A separately reviewed
platform/UAPI admission is needed; changing an accepted string is insufficient.
Exact model/artifact inputs were not found at the known paths. No artifact has
been retargeted, copied or launched there. All builds remain on mi300x.

After R7 retention, the completed clean proof74 worktree was removed, reclaiming
44,496 KiB locally; its branch, source archives and proof evidence remain.
Two exact byte-retained remote artifacts (218's test ELF and the obsolete Verus
seed bundle) were removed after no-use checks, reclaiming 118,600 KiB. The
runtime stage measured 9,399,884 to 9,281,284 KiB, then 9,295,172 KiB after the
new host inputs were uploaded, below the unchanged extraction/link reserve.
Source/proof/native inputs were preserved. The cleanup receipt SHA-256 is
`f3317ce54ad13f910291da5de4ee98d778dde19f36475e3761909899323fd1b6`.
No shared cache or foreign workload was removed.

Earlier R1 Pages payload SHA-256 is
`54165bf91f64fc9ef93b4d52d34c6e5d0ebac4bd8c13d745702cfaf81e6f19d9`.
All 34 source files match the uploaded payload after validation. The admitted
artifact has exactly seven static files totaling 589,025 bytes. Raw QA evidence
archive SHA-256 is
`b1a04f4974e699547f95649d4464bae188ee784c30640949da55a5c3bec97609`.
The completed private Pages stage and input directory were removed after local
retention and no-use checks; the shared browser cache was preserved. Existing
public-main workflows would start GitHub-hosted builds. R1 is superseded and
must not be published. The user approved the exact `pages/prebuilt` allowance;
that branch policy (ID 60080763) is added, preserving `main` (ID 58351133) and
all other environment settings. The verified before/after policy receipt is
`b7a117cf8adfe6f7dfc3aa96234286bf54e7e00fd6779bc7d53b33b586ca6f60`.
Refreshed R4 source payload SHA-256 is
`f74f03efd3a30f6801a977c4bd32228652fbb3e4d267c70bbfb86601024931c4`.
All five remote QA phases pass, including data/negative checks, every width
320 through 1440, and eight rendered viewports. The actual artifact has seven
files totaling 595,623 bytes. Full R4 evidence is retained locally at checked
archive SHA-256
`a3a2c8b221822c6a95dfe6db3234d9f3e7294ab79806e821ec7c25564bbcf296`.
The first retention attempt rejected the harness's exact umask-derived mode
differences; the successor preserves that failure and checks all 34 source
files byte-for-byte with only `0644 -> 0600` allowed. No QA rerun was needed.
Private site source is committed at `4e16c945`. Parentless publication commit
`b0d05f47` contains only seven admitted static files and one deploy-only workflow.
Deployment run `35022808474` succeeds on that exact commit, with deployment
`6468318802` and artifact `10418551245`. All seven live assets match the admitted
hashes at both canonical and cache-busted HTTPS URLs, including the site root.
The refreshed site is live at <https://harsh-nod.github.io/ferric/>. Public main
remains `5d3d93d3`; the environment and both approved branch policies are
unchanged after deployment. Publication receipt SHA-256 is
`42530d1cd8a9f36d9a7863c89a5552700c95c8492521e249b89126ba05d261de`.
No private implementation ancestry, compiler source or build workflow was pushed.
The clean private Pages worktree was removed after commit and no-use review,
reclaiming 3,208 KiB; its source branch remains retained. The deployed snapshot
still marks 218's host campaign pending at its freeze time; the later 724-test
result above is a separate record and does not retroactively alter that snapshot.
The completed remote R4 QA, input, controls and both retention directories are
removed after local archival and a fresh inventory-bound no-use check. Exactly
417 regular files and three symlinks were removed, reclaiming 51,604 KiB.
Generated dependencies are explicitly disposable, not claimed archived. Shared
browser caches and runtime/compiler/proof stages are untouched. The cleanup
receipt is retained locally at SHA-256
`9bb812c1d8d8a3ef0252e90c92cd5711286c0dc658f1b867c368b79d8841cc02`.

### Earlier Stopped Checkpoint

The following records predate the completed host campaigns above.
The earlier scoped shared-host cleanup did not restore workload admission.
The subsequent user-authorized cache cleanup is recorded below; no workload
floor is lowered. Exactly 26
obsolete root-level archives were removed after independent
local byte retention and scoped no-use checks, reclaiming 823,808 KiB on the
root filesystem. Root checked receipt SHA-256
`719924e84f54794601984fee34176d297be4c109de2f038a66360a908398a767`.
Root free space subsequently fell below 16,295,560 KiB. The separately reviewed
combined cleanup is now terminal with exit zero: 37,283 byte-retained duplicate
files and 504 disposable `.rlib`/`.rmeta` files removed, reclaiming 3,098,548 KiB.
Root checked the inventory, all fifteen local archive inputs, actual terminal
result, no-use census, exact transport identity rechecks and final receipt.
All 2,240 noncandidate files remain byte/stat-equal and every directory is
preserved. Compiled library bytes were discarded, not claimed archived; source
and producer records remain retained. Scope-specific identified SSH transport
visibility limitations are recorded, not represented as global process
visibility. Receipt SHA-256:
`12862dc7365442b9e04ad6a15be556507ae3857da10a14079fa07502d6f14952`;
retained archive SHA-256:
`ce51b417fac05d17d6c7d26a7f17d458588b971aa90092cfccdea533ce76e309`.
Root free space after cleanup is 19,279,236 KiB, still 3,789,436 KiB below the
unchanged 23,068,672 KiB floor. No workload resumed. Immediate `/tmp/ferric-*`
ownership inventory found no other significant completed stage; other observed
names without matching ownership records were not recursively inspected or
deleted. The separate completed numerical tmpfs stage remains intact after two
cleanup attempts stopped on incomplete process visibility.

Test-only source `33a31333` derives the actual K4 speculative and maintenance
recipes and exercises `2242 -> 425 -> 2242` routing through the production
source-selection helpers. It checks inert fresh-generation tags, retained
model/KV identities, catch-up choice routing and rejection cases. Root reviewed
the source and diff whitespace check; no formatter, compiler, test or Clippy
has run on this addition. It does not exercise sealed service allocation
witnesses, publication or native queue execution, and it is not a maintenance
defect fix. The frozen native executable and source remain `00ac6a22`.
Its host gate is reviewed and approved only after fresh resource admission:
remote format check, exact 4f6 metadata, library check, the single regression
with an exact source manifest and newly compiled test producer, and strict
all-target Clippy. No gate phase has run. The 29-case synthetic proof-checker
gate is likewise reviewed but unrun; it is not actual Verus or body-mutation
evidence. The approved `339c2561` commit-only Verus diagnostic remains the next
actual proof run after resource recovery. No current-source R13 positive or
negative proof campaign is yet admitted.

## Resource Recovery

The global-index integration campaign removed two fully retained obsolete
runtime files, reclaiming 131,228 KiB (stage 9,406,228 to 9,275,000 KiB).
Receipt SHA-256 is
`dd9462e123e9bf84d35eaa01bf2efb06491a790467bdfcb3425091d77a8aaae6`.
This cleanup used its direct timeout/CPU-affinity invocation, not the later
resource wrapper. Before R2, a separate wrapper-bounded cleanup removed exactly
eight duplicate archives whose full local byte copies were hash-checked:
100,983,416 bytes, 98,632 KiB allocated, stage 9,346,720 to 9,248,088 KiB.
Its downloaded receipt SHA-256 is
`dd2bf89c369237975bad2ca9beb79cd27d4e9e44dab3013e0957fa849c4e8bea`.
Both preserve sources, native/proof identities, required libraries and shared
caches. Same-user inode/lsof inspections retain explicit identified process
visibility limitations; they do not establish system-wide quiescence.

The latest user-authorized home cleanup removed 4,201,396 KiB (about 4.01 GiB).
Of this, 4,064,232 KiB was disposable `target/debug/{deps,build,incremental}`
cache in the already-trashed `fe2o3-runtime-LCD8Sxfn/repo` clone. Its clean Git
status, HEAD, source, logs and other repository files remain unchanged. The other
137,164 KiB was the historical Ferric `acquired-native` extraction, whose 108
paths exactly matched the union of three retained archives; all three complete
tar comparisons and pre/post archive hash checks passed. Scoped lsof checks
reported no users or visibility warnings. Filesystem availability afterward was
86,141,424 KiB (about 82.15 GiB); this shared filesystem figure is not the cleanup
size. At that cleanup, all five registered Ferric worktrees were preserved: integration/proof
are active, Pages and scratch are dirty, and the original checkout has conflicts.
No local build or test ran. Full scope, inventory and terminal log are retained
in `.codex-tmp/cleanup-unused-home-20260915-r2/`.

Before the earlier helper integration host gate, scoped mi300x cleanup removed two
fully retained predecessor test executables and one duplicate archive,
reclaiming exactly 220,296 KiB. The runtime stage decreased from 9,477,696 to
9,257,400 KiB without changing sources, libraries or the frozen native binary.
The older executable's separately retained archive is locally checked at SHA-256
`42dedf2937b178461c4e8dacfa3d54cb1a8aadd09b1fa68f3b5ef9434f29a443`.
The current integration test has since reused one filename with new bytes;
the historical cleanup control must not be reused against that path.

The preceding local-home cleanup removed the redundant `artifacts`
subdirectories from `ferric-layer-c1-runtime-diagnostic-gate-v1/host-r1-failed-retained`
and `host-r2-retained`: 1,454,364 KiB (about 1.39 GiB). Every surviving entry was
compared against its canonical archive before removal, allowing only SSH/local
UID and GID differences. Both local lsof checks found no users or visibility
warnings. Canonical archives remain hash-checked before and after removal:

- `host-r1-failed.tar.gz`: `43305fcb5c5bba788f8584bc0ced43bf9dfa28181b5288152980c4895a978e46`.
- `host-r2-complete.tar.gz`: `b8621f4d156961cae72e60184ccf5de2be719e01bdf861e283db6531dda023a8`.

Cleanup records are in `.codex-tmp/cleanup-ferric-diagnostic-extractions-20260915/`.
All five registered Ferric worktrees were preserved: integration/proof remain
active, Pages is dirty, and the original and detached scratch checkouts contain
uncommitted changes. No local builds/tests or unrelated project cleanup ran.

The preceding local-home cleanup removed three redundant historical extractions,
totaling 3,276,048 KiB (about 3.12 GiB): the layer-C1 host-R3 `artifacts` directory
and the RMSNorm emission `retained-r3` / `retained-r4` directories. All surviving
entries were compared against their canonical archives before removal, and
local lsof checks found no users of the selected paths. The host-R3 comparison
allows only the expected SSH/local UID and GID differences; the RMSNorm
comparisons have no differences. No archive, source worktree, model, current
tool, transport log or unrelated project was removed. Historical extracted-path
references can be restored from these preserved archives:

- `ferric-layer-c1-wave-gate-v1/host-r3-complete.tar.gz`, SHA-256
  `4133520637171a25f12c03a9205420293416a12ac0218ccd37684381548a947f`.
- `ferric-wave-rmsnorm-v15-emission-v1/run-r3/complete.tar.gz`, SHA-256
  `c4f3be27d412250fe44fe28ff12a26e53e4e486230cab45a7d429f2be059c7a9`.
- `ferric-wave-rmsnorm-v15-emission-v1/run-r4/complete.tar.gz`, SHA-256
  `1e21a6ff97f2af54efbdf138abefe08cc12e4a610281b6aea37ffd7785119ec6`.

These paths are relative to `.codex-tmp/`. Full terminal cleanup receipts are in
`cleanup-ferric-host-r3-extraction-20260915/` and
`cleanup-ferric-rmsnorm-extractions-20260915/` under the same directory. No local
build, test, Python or Node invocation was used. Current Ferric
integration, proof and Pages worktrees remain needed and were preserved.

On mi300x, the obsolete `compiler-column-major-r1/vendor` and
`vendor-ce2-private-r1` contents were fully archived and checked locally before
retirement. Their 25,444 files occupied 671,748 KiB; the measured stage reduction
was 664,292 KiB after retaining cleanup receipts. The complete archive remains
SHA-256 `07b85bfc2506c431a01f34dcb0066c869711ce649e61c90b4c8e4942c69a04ef`.
Both empty roots, current source, old copied tools, shared package pool and
the `compiler-0fe/control/vendor-workspace.toml` policy were preserved.

The completed e0 campaign's 48 test executables were removed after full source
and raw producer retention, reclaiming 800,228 KiB from the stage. All 299
protected non-test artifacts, the partial release cache and older copied tools
remain byte-identical. Executable bytes were discarded, not archived. The new
producer archive is retained locally at SHA-256
`efcb213e60879a33352c86a628cc96a22bb046983e35d63c5b6d692d809c92e6`.

The preceding scoped compiler cleanup removed exactly 50 reproducible test ELFs
(1,045,732 KiB allocated), with source and actual producer/log evidence retained
locally. Their executable bytes were discarded, not archived. The raw producer
archive SHA-256 is
`08860666269e68cb22c92a5084d52596f3d5c75b93c994927829476981452a13`.
Runtime cleanup removed two byte-retained host-test executables and their
duplicate remote archive (126,944 KiB), preserving current sources, libraries
and the frozen native executable. The local archive remains SHA-checked at
`4df5ca07179c43a3947e6130814db351ea17014dc6ccce7ec323a875f7f97cf3`.
Both cleanup paths retain same-user inode/lsof checks and explicitly record
incomplete visibility of identity-checked SSH transports and the known sd-pam
process. No shared job was signaled; native GPU admission guards are unchanged.

The failed `a7d58129` host evidence is retained locally at SHA-256
`1bca153171fdece03f2c0acda5517a8cc12be9c2edef2762e54bab8e83f89a36`.
After retention, its superseded 1,145-file source copy was removed (43,660 KiB),
then its duplicate remote archive was removed (21,336 KiB) to restore the runtime
link reserve. The exact `457ca0aa` test executable, current sources, raw logs,
libraries and frozen native executable were preserved.

Completed remote cleanup retained and checked the two obsolete compiler vendor
trees locally before removing their contents (458,756 KiB). Their archive
SHA-256 is `083d3e7fa74fc4c257c2c1480d35aa8dc1e0264928c858395df1fbcdd025adef`.
Two obsolete runtime routing-source copies were also removed after exact source,
archive and no-use checks; current sources, frozen native inputs and prior
failure evidence remain intact. The completed Pages stage and input directory
were removed after QA retention, preserving the shared browser cache.

The subsequent local-home cleanup removed only generated Cargo `deps`, `build`
and `.fingerprint` contents from the inactive, clean `fe2o3-runtime-integration-r43`
checkout, plus clean `fe2o3-derived-bindings-main` and `fe2o3-general-compiler`
worktrees. Both branch tips remain retained. Inode accounting identified
9,840,236 KiB of releasable cache files, excluding 593,404 KiB still linked by
preserved binaries; the two worktrees occupied 132,492 KiB. Local free space
rose from 77,538,464 to 87,516,748 KiB during cleanup, approximately 9.5 GiB.
The difference from the selected sizes includes hardlinks, directory metadata
and concurrent activity. Source remained clean; dirty and active checkouts,
top-level binaries, logs, model files and evidence were preserved. No local
build or test ran. The terminal cleanup log is
`.codex-tmp/local-home-cleanup-20260915.log`, SHA-256
`68d34772d7b3181ec16406bac148a34c949ae7cf1070b6bee95567b500e59ef6`.
This local cleanup is separate from the mi300x cleanup described below.

The resumed turn found root free had fallen again to 197,620 KiB. A further
user-authorized cleanup emptied 87 old Rust incremental-cache directories in
bounded `/tmp` build targets, skipped 26 recently touched trees, and removed
43,908,692 KiB of allocated cache data without touching sources or executables.
Its command exited zero; root free immediately afterward was 42,518,532 KiB.
Subsequent shared-host space changes provided additional headroom. Compiler,
runtime and proof host validation then ran under unchanged guards. The new raw
cleanup log is `.codex-tmp/mi300x-tmp-pr745-cache-cleanup-20260915.log`.
GPU availability, not root space, currently prevents the native retry.

The following paragraphs retain the earlier resource checkpoints.

An earlier root-owned read-only SSH probe recorded 19,239,820 KiB free on `/`
and 142,096,292 KiB on `/run/user/1002`. Root free was still 3,828,852 KiB below
the unchanged 23,068,672 KiB admission floor at that checkpoint, so validation
remained stopped. The user subsequently authorized home-directory cleanup.

On 2026-09-15, two root-owned SSH cleanup commands completed with exit zero:
179 Rust incremental-cache directories in `/home/harsh` and nine in explicitly
selected old `/tmp` build targets were emptied. Their pre-removal allocated
sizes, excluding retained parent directories, were 278,040,300 KiB and
11,123,068 KiB respectively. These are disposable rebuild caches, not archived
artifacts. Each selected tree was user-owned, canonical, on the expected
filesystem, free of symlinks or special files, and untouched for seven days.
The user-scoped `lsof` check reported no open file, mapping or working directory
under the selected roots. It warned about inaccessible Docker/tracefs mounts;
this is not a claim of complete system-wide process visibility. No process was
stopped. Sources, Git worktrees, compiled outputs, model files, evidence and
active Python caches were outside the deletion scope.

Home and root are separate filesystems. Home free space rose from 5,262,160 to
278,893,300 KiB during its cleanup; root free rose from 17,223,136 to 27,977,876
KiB during its cleanup. These live filesystem deltas differ from cache sizes
because of shared-host activity and filesystem accounting. A fresh terminal
SSH check at `2026-09-15T08:09:22Z` records `/` free at 27,896,212 KiB,
`/home` at 278,890,388 KiB, `/run/user/1002` at 142,096,292 KiB, and
MemAvailable at 1,700,723,600 KiB. The observed root-space blocker is cleared,
with 4,827,540 KiB above the unchanged floor. No workload has resumed and no
gate or proof label changes. Local builds remain prohibited.

Controls and raw terminal outputs are retained locally in `.codex-tmp/` as
`mi300x-home-cache-cleanup-20260915.sh`,
`mi300x-home-cache-cleanup-20260915.log` and
`mi300x-tmp-cache-cleanup-20260915.log`.

Root reviewed all five prepared `7f5f15de` source/cache transition controls,
their SHA-256 pins, the exact source archive, and the bundle with already-present
`bf0f4841` prerequisite. The plan captures every partial d3 release-cache path
alongside its 173 reported Cargo artifacts, advances git-verified source mtimes,
and requires newly compiled first-party artifacts before allowing same-campaign
7f reuse. These controls remain unrun, and no release-tool or native result is
implied. Fresh resource/capacity admission, remote syntax checks and canonical
upstream reconciliation remain required before execution.

Recheck all resource and per-stage capacity guards before each launch. The
prepared commit-only Verus diagnostic is the first proof job; runtime guard
fixtures, routing-test validation and the current compiler host campaign follow
under their reviewed scopes. Pages QA and publication remain pending. All 33
M1 gates remain open; the full objective is unchanged.

## Recorded Runtime Evidence

Runtime successor `cef91e5c202f77f660b9e83780d67783fd3e49d0` passes all
six authenticated-admission host phases: normal engine check, eight new-window
tests, one reused-generation snapshot test, 715 engine tests with nine existing
ignores, strict all-target engine Clippy and 37 adapter source policies. The
production change in `1ac71866` snapshots every new cache before committing
page leases, then reserves writes through those retained caches. Its initial
test compile failed because an error enum has no `PartialEq`; cef changes only
that assertion to pattern matching. No production error trait or generation
predicate was relaxed. Root checked the passing raw results, all 62 retained
payload hashes and equal before/after source content for 1,145 files and 205
directories. Evidence archive SHA-256:
`cc97f0c54410b119005ad2dc8ae5152b24e2fce7df0a4ae85c090e633a620bdf`.
The failed 1ac evidence remains separate at
`38593dfa6be478d65fff33d4586c9a99e6cae6b31fa4df47533abdf242261588`.
The optimized cef/4f6 release build also passes in the independent bounded
release target. The normal non-test executable has SHA-256
`6b243c6fdbf1f1d906cb8a3a17201c3f7b03e61768cc02d7dac94bb87b428545`
and is 14,147,584 bytes. Root independently checked all 49 retained payload
hashes, the actual compiler artifact, source equality and producer association
in archive `4d62ad0969b1f5e989e4d57f31d0074238a7ac64506dfb82b7f3bbc265010fdc`.
The prior c5 executable and evidence remain unchanged.

The subsequent cef/4f6 32-token native attempt is terminal with exit 134 at
`maintenance publication`, while attempting the optional 425-packet draft
maintenance transition. No completed token/round JSON report was emitted, so
no completed round count, full-acceptance result or 32-token success is claimed.
The bounded diagnostic retains custody but does not expose the inner submission
phase. Diagnostic-only successor `ba6f8c9ca7557214c808c81bb490601d285136e6`
now preserves copied submission phase, inner transition stage and error class
through maintenance and restore failures without formatting retained owners.
It adds five focused custody/diagnostic tests. The host gate passes compilation,
those five tests and the full engine suite (720 passed, nine existing ignores),
then fails strict Clippy on one elidable named lifetime. Evidence remains at
`ff078bb5b238738e144de52322af3f37313057b90a4090d1e4d5131dc8de58d8`.
Successor `00ac6a22cb65b478b00cc7f1dccdb98c7de6fe67` changes only the three
lifetime spellings. Its six-phase gate passes normal check, all five focused
tests, strict all-target engine Clippy and 37 adapter policies. Root verified all
63 retained payloads and exact source-before/after equality in archive
`20ae08c1a49e0e0a3600cb607739fd697d599f03ca47ad76ddf435e97d217ff5`.
The independent normal opt3 release also passes: executable SHA-256
`03bf2d118e2b5aeecafd75c0e009ca985aa526bef11186bcb652f33f45e6049a`,
14,195,008 bytes, no optional features and no RPATH/RUNPATH. Root checked all
49 retained payloads, actual producer and source equality in release archive
`e297d997f77857a1b9cb07793b7346f9b931b45791991a84c5b53de853ea806a`.
This is not a maintenance defect fix. The one-shot R5 native diagnostic exits
126 after approximately eight seconds. Only artifact-admission startup is
reported and stdout is empty. The guard cannot read the process group of a
KFD PID that disappears before capture; no positive foreign selected-GPU usage
or ownership attribution is established. Only the owned group is terminated.
Root checked all 27 R5 payload hashes and its retained raw failure in archive
`af4f5d9c0c6b44f302f004a72baf49dd4db37c2eefd839dfbb0052b60359d83a`.
Fresh selected GPU memory is 298,647,552 bytes, with no selected usage or queues
and no owned group. It differs from that attempt's 298,721,280-byte prelaunch
observation, so its exact-baseline cleanup check remains false; scratch was not
deleted. The actual maintenance inner condition remains unknown. A narrower
selected-GPU ownership-query order and 25 mock cases are reviewed but untested,
without removing checks on unreadable or conflicting selected-GPU facts.
The earlier R4 maintenance failure used the same legacy twelve-kernel image
and model bundle as c5, not
the MFMA candidate. Root independently checked the actual stderr, executable
producer and all 22 native payload hashes in archive
`916bd86e04abbb382916b7bc841a19d6d11c118a227686e3c3937cb18e111ce9`.
Both the immediate after-state and the separate fresh state show the exact
298,647,552-byte selected GPU baseline, no selected queues or VRAM users, and
no owned process group. No GPU reset or foreign intervention occurred. This
is a retained hardware failure, not a benchmark or M1 qualification receipt.
After archive and fresh no-use verification, the exact completed native scratch
directory was removed, reclaiming 13,920 KiB. Root checked cleanup receipt
`7e821d50f1a64cb1a90547651b553894fe195f51f7cb6bdbb834bf1f1bf32157`.

Runtime candidate `cdc084206dae4332c8f4dcea0f6b59302ac6db1a` now repairs
cache-local retired-page generations at the shared completed-step return.
The existing joined completion, pool-ticket and whole-roster checks run first;
exclusive borrowed metadata guards preflight both roles of every member before
any generation changes. Output allocation, including boxing, precedes commit.
Failure preserves the original completed ownership graph. The stale-generation
check, native lease custody and completion requirements are unchanged.
All fourteen host phases pass: 129 spec tests, 713 engine tests with nine
existing ignores, four spec and 171 engine doctests, strict all-target spec and
engine Clippy, 109 adapter tests with one existing ignore, 15 speculative CLI
tests and 37 source-policy tests. Focused tests reproduce page-nine rollback
and reuse at cursor 142, both roles, stale leases and late-member rejection.
Root independently checked all 118 retained payload hashes in archive
`800b7943ab0f76d59a851036992e009a1c74805806be00ea2485d6d10a517330`.
The cef native attempt above includes this transition, but does not qualify
it or the overall run. Separate proof candidate `dce398da` contains direct
contracts for the actual preflight and commit bodies, bounded loops and exact
state-frame tests. The first gate stopped on source-roster sort order; after
that control correction, four actual Verus attempts stopped before proof queries
on reveal-name syntax, sequence-length types, mutable-reference entry-state
syntax and private-field access in public contracts, respectively. These are
retained frontend failures, not failed or successful theorem queries. Successor
`79c81638416fbb7894e793d41ea6d21d83154050` adds defined public closed
current/future specification views while keeping runtime fields private. The
future view uses the required prophetic mode annotation, not a trust exemption.
That revision first encounters another public-contract/private-helper frontend
error. The visibility-only correction `cf329240` reaches Verus: the actual
preflight passes two solver queries for one executable body. The original
checker incorrectly expected one query; a corrected checker accepts the retained
unchanged raw result after reconciling the pinned Verus loop-query counter.
Its synthetic checker tests pass all nineteen expected outcomes (one accepted,
eighteen rejected); these are not actual-body mutation results.
The actual commit then fails its loop invariant. Ghost-only diagnostic successor
`19ea4e1a` isolates unselected-slot preservation; `5a689e7d` introduces explicit
pointwise preservation but still fails the pre-update snapshot assertion.
The latter raw solver failure is retained in archive
`f08b13f7c2e32b1282d51fe559477039834abd79b8203e3af0453f3a4be9ad93`.
No contract or executable behavior was weakened. The remaining current-source
positive jobs and all 24 actual-body mutations are pending. No proof labels have changed.
The engine's whole-member and native lease composition remains an independent
obligation.

Separate diagnostic fix `d74302d6b82315f9d1c6cc973d15b17b4e5a8382` passes
all six host phases, including three retained-owner tests, three bounded-output
tests, strict engine/CLI Clippy and 37 source policies. Its errors report copied
facts without recursively formatting retained owners; CLI failure output is
bounded to 4,096 bytes. Root checked the raw results and all 62 retained payload
hashes in archive
`d2d33376e91d3a179677f1ff90f89f86c9548eecaa7df9ae6f9e2e4882257d42`.

Adjacent lifecycle review confirms structural new-request admission already
snapshots reused pool generations before leasing. It also finds a separate
authenticated-path ordering defect: that path committed leases before requesting
the Free-only generation snapshot. The host-validated cef correction above
prepares all caches before lease commitment and preserves the Free check.

The earlier 32-token c5/4f6 attempt fails at ordinary speculative target
reservation with `Physical(PageGenerationMismatch)`. Its retained diagnostic
shows target cursor 142, physical slot nine still retired at generation one
after epoch ten, and a newly returned/released pool lease for that index at
generation two. Source review identifies a shared completed-step return defect:
the pool ledger advances its generation, but `take_retired_pages` only removes
the cache's retired lease vector and never advances the request-local physical
slot. The generation check is correct and must remain. A whole-roster,
completion-bound transition of both records is required before another run.

The process aborts after formatting a 66,300,297-byte retained-owner diagnostic.
The outer guard then sees the known native PID disappear during its process
group check and exits 126; this is not evidence of foreign GPU use. Stdout is
empty, so no 32-token, completed-round-count or full-acceptance result is claimed.
The immediate after-state still has 69,029,134,336 bytes of selected GPU memory;
the separately recorded fresh state returns to the exact 298,647,552-byte
baseline with no owned group, selected queues or selected per-process VRAM.
No reset or foreign intervention occurred. Archive SHA-256
`d901a98e4ca873467027f094dd1d177dabf206e632609d1198368cae38e74493`
and all 27 native payload hashes are independently checked. The host-validated
successors above do not relabel that native failure. Its completed owned stage
is removed after retention and fresh no-use checks, reclaiming 78,664 KiB.

The optimized c5/4f6 native attempt now completes five real speculative K4
rounds and publishes eight tokens on an idle gfx942 GPU. The observed accepted
draft-prefix lengths are `[0, 0, 1, 2, 0]`; epochs and dispatch generations
advance from two through six, and the final target/draft cursors both equal
136. This exercises zero- and partial-acceptance continuation through the actual
structural registry, bridge and coordinator. There are no completed draft
catch-ups, so continuing full acceptance and the 425-packet maintenance/restore
path remain unobserved on hardware.

The run uses the retained twelve-kernel 8113/4f6 image, not the MFMA candidate.
Its 128 physical prompt tokens include active EOT suffix fill, not attention-mask
padding, and its published tokens exclude the prefill anchor. The control and
report-consistency check exit zero; queue teardown completes, the owned process
group is absent, and selected GPU memory returns to its exact 298,647,552-byte
baseline with no selected queues. The retained archive has SHA-256
`c46c0023872f404ddda8140f9162fa2d36785618916266ad41d0240234908870`;
all 25 payload hashes are independently checked. This is a source-bound
engineering execution result, not target-only parity, authenticated M1
execution, numerical qualification, production serving or a matched benchmark.

The recorded runtime checkpoint is `c5ad1ba47ffb940bfc0b69f1acb89fd259e60e56`,
still pinned to published fe2o3 `4f6f65ce22222bae9ece5c5f08c66e56e022a9e4`.
Its committed coverage, source-tool closure, all 32 locked/offline dependency
graphs and three dependency TCB comparisons pass. The exact standalone
speculative-smoke debug binary is built, with SHA-256
`4151c135ff693e1d66a1b8004a3d31488dd227ee0c50ce45b90a025f0e488e2a`.
The earlier debug eight-token K4 attempt stopped before inference when its
GPU-use guard detected a PID outside the owned process group. It used the unchanged
twelve-kernel 8113/4f6 image and retains that image's original producer identity.
The c5 checkpoint changes only documentation and body ledgers from the executable
source below; it does not relabel the earlier test or GPU results.

The native control exited 126 after approximately 14 minutes 25 seconds, not
at either deadline. Debug CPU model preparation took 604.307 seconds; KFD
binding completed, but no prefill or speculative-round output was produced.
The detected PID had already exited before attribution could be captured, so
its ownership remains unresolved. The control stopped only the owned process
group. Fresh KFD/DRM observations confirm no owned processes or selected queues
remain and GPU memory returned to the exact 298,647,552-byte baseline. The
retained archive has SHA-256
`66575e9bd00b950ed0a592c931de6e7ae672da2ca27e50017d4f853d76af89ae`.
This is a failed engineering attempt, not a native continuation result or a
performance measurement. The completed native stage is removed after archive
acquisition and fresh no-use checks, reclaiming 42,936 KiB while preserving the
model, artifact and evidence.

The exact c5/4f6 optimized standalone build now passes on mi300x. Its normal,
non-test executable has no optional features, optimization level three and
`fresh=false`; it is 14,133,320 bytes with SHA-256
`d6d6f7af56957d0987d0e2cb01d4d2b75766fcc1f95f43fb19da39c7ff294819`.
Fresh metadata, all 1,145 source files and 205 directories, before/after source
equality, actual artifact identities and the no-RPATH/RUNPATH check pass.
The build used a separate bounded 3 GiB target, with the shared 10 GiB cap
unchanged and automatic Cargo cache cleanup disabled. Its retained archive has
SHA-256 `5c5399489e1d208e90fa5e82f2bc0843f9fe956c72e6d5dc72aa5329b4db6a4a`;
all 51 retained payload hashes are independently checked. This is a build
result, separate from the subsequent native result above; it is not a measured
serving improvement. The native retry includes more complete detected-PID
diagnostics. Initialization and foreign-use checks are not bypassed.

Private runtime integration `dfb4c5140b9c50ad936e27f1850541043135130b`, tree
`b3da45f59cb9f346078c20daa5ce6bff0735eaf9`, passes 707 engine library tests
with nine existing ignores, separate binary cohorts of 11, 2, 84 (two existing
ignores), and 2 tests, plus 171 doctests and strict all-target engine Clippy.
All 32 locked/offline graphs, three dependency TCB comparisons, the retained
source-gate tool's complete source/hash checks and raw body inventory pass.
Before/after source checks cover all 1,145 files and 205 directories. The
retained evidence archive has SHA-256
`f6aedb5d76b2fa5f5661834c548d2dc7a780035de6430f1adb8f782059256d2c`.
The unchanged adapter retains its earlier 9421a6a8 results: 109 library tests
with one existing ignore, 12 speculative CLI tests, 37 source-policy tests
and strict all-target Clippy. These are host results, not repeated GPU
execution or proof of the full-acceptance path.

The earlier combined `32eae300` inventory rejected a new statement-form
`unreachable!` in `settle_structural_draft_catchup_v1`. The dfb correction
replaces it and two reservation assumptions with explicit errors retaining
the actual pending reservation, completion and other owned resources. The
parser is unchanged; the new test checks source/error-tuple shape, not a
fabricated native completion. Both the original rejection and successor
passing evidence are retained.

Ledger-only successor `0a93387cc078619757d3de6488cb137544ed74f0` adds 83
structural runtime and nine all-case R29 comparator bodies as pending-Verus,
removes the obsolete `require_selected_case` row, and preserves all 7,741
surviving full rows verbatim. The ledger now has 7,833 unverified bodies:
7,645 pending-Verus and 188 excluded-presentation. Exact 0a passes fresh
metadata/TCB checks, coverage generation and the generated candidate's
self-check: 177 modules and 8,557 bodies, with all 724 existing verified rows
unchanged. The generated coverage is integrated in this checkpoint; final
committed-source equality and the direct committed-coverage check now pass
at c5, including exact before/after checks of all 1,145 files and 205
directories. No proof or qualification status is upgraded. The 0a coverage
evidence archive has SHA-256
`14280d3c014afd90641e88c7c54c90259dc6f5c0e527757879b2170bd12c6922`.
The final c5 gate and standalone binary evidence archive has SHA-256
`11670d90819a8ed3a731ec733e94434805ad9b929e4b2efe55ed8666b2b775f4`.

The kernel/integration lane now has a separate, host-tested MFMA strategy
candidate. It retains the legacy twelve-program scalar catalog and adds an
explicit thirteen-program attributed catalog. Strategy and executable-catalog
identity are checked through runner, operation plan, dispatch and packet
joins. New tests cover all fifteen complete step intents plus three catch-up
intents, exact kernarg and native weight-buffer layouts, and rejection of
mixed strategies/artifacts. Explicit engineering MFMA input/capture commands
record the selected route; they grant no qualification authority. Remote
formatting passes. The first exact `7e84909f` compile check, with a complete
unpublished fe2o3 `b86d7749` dependency overlay and isolated target/cache,
rejects an opaque expression macro in the MFMA root before compiling the
engine or adapter. Correction `0e3f77c5` exposes the same 108 admitted shape
tuples as plain Boolean expressions, with an independent host-oracle
comparison over literal-boundary partitions. Its retry compiles the aggregate
device crate, then finds a missing strategy argument in engine operation-plan
validation. The two-line forwarding correction `ce2d3c1f` passes the engine
library and actual engineering-R29 capture binary compile checks, all 709
engine library tests with nine existing ignores, and all eleven aggregate
source-contract tests. These checks use the same complete b86 overlay, not
the newer compiler candidate. The first linked-test attempt completed the
eleven source tests but stopped at its original 2 GiB scratch cap before
engine tests. After audited cleanup and an explicitly reviewed 3 GiB total
cap, the unchanged engine cohort passed. Both attempts remain distinct.
The passing engine evidence archive has SHA-256
`20045976f0abc71e65e9a86a132cb685c29b67fce8e494de7c78353c308ac92c`.
The first adapter cohort passed 97 tests and rejected thirteen service tests
before socket creation because the control's temporary path exceeded the
unchanged 100-byte limit. A shorter owned temporary-directory control passes
110 adapter library tests with one existing ignore, 87 engineering capture
tests with two existing ignores, and 38 source-policy tests on unchanged ce2.
Source-gate testing exposed both unsuitable overlay lockfiles in its repository
fixtures and a stale single-roster test anchor. Test-only successor `cbf49f06`
mutates legacy and MFMA marker ordering separately; all 42 source-gate tests
pass against its pristine source and original locks. No production admission
or parser check was weakened. The original failed runs remain retained.
Scoped Clippy then found one introduced redundant method-call closure.
The exact method-reference correction `1fc91418`, still using the complete b86
overlay, passes strict engine-library and selected adapter/capture/source-policy
Clippy, plus all eight focused physical-program catalog tests. Its source and
locks remain equal before/after. No lint suppression or semantic change is
introduced. No Ferric MFMA aggregate image or GPU result exists yet; singleton decode
still selects scalar GEMV. Source-gate commit `8fc15929` admits the exact
new normal `fe2o3-hsaco` declaration; a final dependency-inventory regeneration
is still required.

The compiler candidate's validated upstream base is
`5c0c53c85e4fa748b1d84751cca91f8ea70f8a1a`, four commits beyond bf30. The
delta streams semantic-SSA replay one function at a time, boxes private cold
projection diagnostics, shares the private SSA event-emission grammar and
shares prepared SSA emission across output sinks.
Source review finds no KFD/runtime or device change. Root rebased the local
candidate onto this main as `7074f2b57d97bad12a08c2350de54cdd6baa9e42`;
all ten candidate patches are unchanged by range-diff. On top of that source,
`201136d9` preserves exact live induction comparisons in loop-body predicates,
addressing the unresolved branch that blocked the full MFMA kernel. Its fresh
host build passes, with all 70 first-party artifact records across 63 packages
rebuilt. The first focused run passes eight tests but finds one test expecting
a later rejection where an existing earlier guard correctly rejects the input.
Test-only successor `18f8710d4ada318e3a3462cf45984c8dd44c1722` corrects that
expectation without changing production code. Its complete remote host gate
passes: nine focused tests, 565 backend tests, 140 kernel-analysis tests,
1,103 Pliron tests, two CFG integration tests and the explicit paired BF16
storage export test. The nine focused tests are part of the 565-test cohort,
not additional unique coverage. Reused production artifacts are checked against
the fresh 201 build; the test executable is rebuilt. Root checked the archive
SHA-256 `fd167de7230a0968832cbb552f253fce20c44387797f41f872ceb90d7365d0ac`,
all nine terminal phase statuses and the raw test summaries. The compiler
worker's separate strict Clippy gate exits 101 with 17 library diagnostics;
test-target linting did not complete. Root checked the raw failure archive
`69ba1ac0daf9963f1ef3a7d58bcc0de1b0df4a14df10b4b9f91be6121c8548a3`
and source classification: no candidate-introduced trigger was identified, but
this is not a matched pristine Clippy run or baseline-lint equivalence.
The source-bound emitter rebuild and full
thirteen-kernel MFMA image are still pending; the host result does not establish
that the original aggregate emission now succeeds.
Retained emission tools remain frozen at
`5602c4889142571d7d82041b8a9bdef369799af1`; they are not relabeled as 18f.
No candidate commit has been pushed. Canonical upstream was freshly fetched at
`bf0f4841774de433d5f02a863f684f4675f699ac`, adding projected call destination
evaluation-order handling. Root reviewed the production delta and clean rebase
`d3a52cd2f3939fb415b0980ab3fd0a3298e8b36e`; all twelve patches are unchanged
by range-diff. All seventeen source-bound remote host phases pass: 565 backend,
159 lowerer library plus 58 lowerer integration, 1,123 Pliron plus two CFG,
140 analysis and one explicit paired storage-export test. Nine focused tests
are a subset of the backend cohort. Four artifact-freshness checks pass; reused
first-party bytes are bound to the current d3 builds, not relabeled older output.
Root checked archive
`a353a7939bb87de5c32da3169c555f117b07bbe1cca9c23505e9d5521bb40a8f`
and its actual phase/test summary. The actual d3 tool rebuild was admitted after
verified removal of ten locally retained duplicate archives, reclaiming
481,796 KiB. No source, cache, existing tool or image was deleted.
The CLI release build then stopped with exit 125 when shared root free space
fell below the unchanged 22 GiB reserve. The compiler stage remained below its
12 GiB cap. No CLI copy, backend build or device emission ran; this is a resource
stop, not a compiler semantic failure. There is no d3 strict-Clippy or
native-emission pass. A subsequent canonical fetch finds
`83986180401671aeedf89017d2f52a744c28714b`, adding optional exact source-to-SSA
occurrence capture and shared replay-driver plumbing. No KFD or device files
change in that upstream commit. Root preserves d3 at
`codex/mfma-column-major-b-d3-hostchecked` and rebases all twelve unchanged
patches onto that main as `7f5f15de5aaa63b79cb35033a983f58fff1cb61f`, tree
`2a49a5c6d80621539f9b5a80f02ecb3f5601b918`. Range-diff and diff whitespace
checks pass; no build or test of that source has run. The d3 partial tool
artifacts require reconciliation before a new source transition. Canonical
upstream must still be checked again before any push to main.
Ferric integration still pins 4f6. Prior
pristine 0fe emission produced an independently inspected twelve-entry COV6
image, not a validation of this newer upstream or thirteen-entry candidate.

Compiler candidate host cohorts have passed, including 107 device, 87 MIR,
156 lowering and 545 backend library tests at their recorded candidate sources.
Strict backend Clippy remains failing. A separate pristine baseline attempt
encountered cross-source cached metadata before reaching Clippy, so no baseline
lint equivalence is claimed. Exact first-party fingerprint invalidation was
completed and archived, followed by a fresh b86 rebuild: all 69 first-party
artifact records across 63 packages have `fresh=false` and current-source
manifest identities. The actual paired storage export first rejects a
redundant lane-varying fixture guard. Fixture-only correction `eec1d434`
removes that guard while retaining exact launch geometry and checked tail
writes. Its successor export reaches the real two-phase reduction loop but
rejects unresolved loop-branch uniformity. Both failures occur in the first
row-major case, before testing the column-major case. The unknown value was
traced to direct launch-geometry queries in pre-loop guards, not unsupported
induction generally. Root-reviewed `998c5a99` preserves authenticated direct
geometry queries as bounded, unconstrained analysis arguments; it does not
replace actual geometry with maximum-grid constants or change the runtime ABI.
The production code compiled, but its driver test used a nonexistent unsigned16
type shorthand. Test-only successor 5602 uses the actual canonical KIR type.
All four new geometry tests, the existing geometry-wrapper non-purity negative
and the actual paired export now pass. Row-major retains semantic MIR V11;
column-major uses V15 with its authenticated constructor/load and rejects V14.
Both keep three source/KIR arguments and the exact 48-byte, alignment-eight
kernarg map. Production KIR identity V9 and canonical KIR V10 remain distinct.
The exact candidate release CLI/proxy and CLI-bound backend/extractor now
build successfully. Exact ce2 source preparation, the dependency-only private
5602 overlay, locked/offline metadata and the union vendor byte checks also
pass. The first native row-major emission then rejects the union vendor's
23,310 entries at the unchanged 20,000-entry pinned-tree bound, before Cargo
extraction; no column-major or Ferric aggregate attempt follows that failure.
The installed rustc tree has only 6,561 entries. Lock-derived complete per-mode
vendor views fit separately: 305 packages/19,825 entries for the original
workspace fixtures and 198 packages/11,215 entries for standalone ce2. A
storage-neutral whole-package partition now passes, preserving every package
file and leaving all manifests unchanged. Both original fixtures pass locked,
offline metadata checks in an empty Cargo home. The next actual row-major
extraction reaches Cargo/build-std, then exits 125 at the unchanged 12 GiB stage
cap. Its 802,232 KiB transient growth includes workspace incremental caches;
the extraction boundary clears the outer environment, so the outer
`CARGO_INCREMENTAL=0` does not disable those caches. No semantic compiler error,
HSACO, column-major run or aggregate run is attributed to that stopped attempt.
Neither the reduction loop nor the convergence rejection policy is bypassed.
After preserving raw logs, all 145 generated metadata files, tool-copy equality
and the exact 1,576-file/103-directory roster, only the failed extraction scratch
is discarded, reclaiming 802,212 KiB. Its compiled caches are explicitly not
byte-archived; the retained compact evidence has SHA-256
`e190e665402d5c1e05ab0efdd103aa8d994a1c80a1b8bde4fe5e948971430d5a`.
Further reviewed cleanup removes 648 obsolete first-party debug cache files,
reclaiming 1,943,840 KiB while preserving all 1,445 protected current-source and
executable-closure paths. Compiled cache bytes are explicitly discarded, not
claimed archived. The retained inventory and attribution archive is
`a1c5ad474e34e9d27ef2294d0d28ecb0ed1eb110e8c3d676d102066f2d668411`.
The stage cap remains unchanged.

The unchanged 5602 row-major and column-major fixtures now both emit actual
gfx942 COV6 HSACOs and pass exact replay and read-only ELF/MFMA inspection.
The images are 12,208 and 12,048 bytes respectively; each contains actual
`v_mfma_f32_16x16x16_bf16` instructions. The original row inspection rejected
an incorrect expected mnemonic spelling; the corrected read-only inspection
passes on the same image, with both records retained. Evidence archives are
`519982e9bd41847163bb03a27719e3d503d223fec658292adca24814162ddf3a`
and `fcc43eb604aea87e7943afd20fef8d29b16cd32764b36cff82f9de7c82b3ea62`.
These are compiler fixtures, not the Ferric aggregate or GPU execution.
Two vendor-view transitions stopped before any
package moves on unreadable short-lived transports; no process identity or
foreign-use conclusion is inferred after they disappeared. A separately copied,
byte-checked 198-package input view now passes with 11,215 entries. Every copied
inode is distinct and all 365 original packages remain unchanged. Exact ce2
source checking and locked metadata equality in an empty Cargo home pass.
The actual ce2/5602 aggregate emission then fails `FE2O3-TENSOR-LAYOUT-002`:
MFMA block 205 is control-dependent on unresolved loop-header block 105.
Lowering stops before target IR or artifact emission; no thirteen-root image
exists. The ranked loop body contains an empty-control analysis split, and
source review identifies missing projection of retained induction values into
body predicates as a candidate cause. Exact source-span attribution and the
compiler repair remain in progress; no kernel guard or convergence check is
removed. The failure archive is independently hash-checked at
`6a14af1919ec2970d121270ab6e2b784ac162293670c6da16a393e5658916b2f`.
Completed compiler cleanup removed exactly fifty obsolete test executables,
reclaiming 541,048 KiB after retaining their complete archive locally. Removing
the two redundant remote archives reclaimed another 144,272 KiB; the local
archives, current tools, libraries, source and retained images remain intact.
The completed MFMA check and formatting stages are also removed after exact
inventory and fresh no-use checks, reclaiming 2,968,964 KiB. Source, test evidence
and claimed executables are retained; disposable private build caches were
discarded, not claimed archived. The separate engineering-only MFMA numerical
probe is committed privately at `ee44cb2565a9445ce90876209a92f777e5c130b7`.
It uses strict thirteen-program artifact admission, the existing checked service
lifecycle, generation-bound guarded C readback and the canonical 108-case
catalog. Its signed sparse fixture tests layout, tails and residuals; a second
positive fixture contributes in every K16 phase. Review found and corrected an
earlier signed-pattern cancellation that could hide an omitted 48-phase block;
new regressions cover that block across all 108 references and reject an actual
small-case incomplete output. Single-phase loss can still be hidden by BF16
rounding, so this is not a traversal proof or general numerical qualification.
All five source hashes match remote formatting output. Its first host gate
passes all ten probe tests but finds a stale six-versus-seven dependency-count
assertion; test-only `c9d86317` corrects it. Strict Clippy then rejects a 64 KiB
executable-hashing stack buffer. Successor
`5df6d1e135dd6f52c1a13f578facbd63259341e4` uses the existing 8 KiB streaming
buffer pattern without changing hash or length checks. With the complete
private 5602 dependency overlay, it passes compilation, all ten probe tests,
all 39 source policies, strict selected-probe Clippy and the actual CPU-only
list command. Independent list checking covers all 108 exact shape/stride/grid
contracts and 160 distinct source profiles. Source-before/after and full locked
metadata equality pass. Archive SHA-256 is
`7a97291a46586d68c886489b2aa5e1ffd08450f537abd693a5153f04fdbd3215`;
the final normal probe executable is
`e741e00a19189562cf4320e4b53a31e9ee6989b8da7be129740ee2fa6be428a9`.
Both failed predecessors remain retained. No GPU numerical execution has run,
and dependency/coverage regeneration remains pending. A bounded six-case
campaign covering draft, target partial-M and beta-one shapes with both fixtures
is prepared but cannot launch without the actual thirteen-root artifact.

Pages-only checkpoint `5d3d93d3e7f08645273d274bc35efbc79133e686` is deployed
successfully in workflow 34926507415. Root reviewed the complete six-file delta,
all 34 source hashes, source equality, raw QA/native-report association and
desktop/mobile screenshots. The remote browser gate passes all 1,121 widths
from 320 through 1440 pixels plus eight named viewports. All seven live assets
match the validated 574,624-byte artifact, and historical performance data is
unchanged. Only public Pages ancestry and site changes were pushed. The owned
clean worktree (3,164 KiB) and completed remote frontend stage (75,904 KiB) are
removed after retention and no-use checks. This public checkpoint records the
successful short R2 run, not success for the newer R3 failure described above.
The earlier Pages checkpoint `5fe89b3e` and its 82,380 KiB completed remote stage
retain their distinct deployment/cleanup evidence.
The completed runtime worktree is removed after integration/evidence checks.
Further reviewed cleanup reclaims 1,675,048 KiB of obsolete runtime artifacts,
578,572 KiB of completed review scratch, and 503,972 KiB of completed compiler
trybuild output. Current source, tools, models, caches and active dependencies
are retained; compiled trybuild cache bytes were explicitly discarded, not
claimed archived.

The new GPU result is limited to the zero/partial K4 continuation described
above. There are no new numerical-acceptance, Verus or matched performance
results, and all 33 M1 gates remain open. The next priorities are the separate
authenticated admission-order repair, a current-compiler MFMA aggregate,
sustained hardware continuation with the tested generation repair, and the remaining numerical
cases. The failed 32-token attempt is not bypassed to obtain a shorter success.

## Earlier Integration

Production checkpoint `8113e2314b2e030f93bc27d1281b4caee7e66216`, tree
`b5e3c150a42de4467e8fd9169d16ea4a7bc78ba9`, combines the engineering numerical
pipeline and executed draft-initialization helper, then advances the same
80-file pin roster to published fe2o3 `4f6f65ce22222bae9ece5c5f08c66e56e022a9e4`.
Reverse-byte equality passes before refreshing two derived verifier digests.
The upstream delta adds an inert transition-receipt codec/admission path; it
does not change KFD/runtime, activate new optimizer behavior or provide the
missing theorem checker. Full IR/analysis, 543 backend and 37 aggregate tests,
release tools, actual emission and independent twelve-entry/descriptor ELF
inspection pass. The 103,616-byte 4f6 image has SHA-256
`6f77d6813e6a2c9fd20c8b50eaffe0feb4435f00ac65192e9574a3637b6e5284`;
its manifest is `39cf7d5a915f759cb289e2f48b63acbd725904bb22aa14439bbda575074b3d3f`.
It has exact output replay but no GPU validation. The ccfd GPU runs below retain
their actual source identities.

Test-correction checkpoint `500838ee078dc001f4f26857daf08424b4c908e8`, tree
`ae110bfc0ea86a7a445efe26a9670021d450aaec`, adds only two test corrections:
an inclusive range required by strict Clippy, and a checker listener test race.
The original full checker cohort failed twice; an unchanged isolated retry
passed. Diagnostic instrumentation retained child stderr and naturally exposed
ENOENT from inspecting the socket pathname after the listener deliberately
unlinked it. The test now checks socket/0600 readiness before connecting and
waits for unlink before Begin, retaining the full credentialed exchange and
original deadlines. No production listener behavior changed. Exact 5008 passes
strict engine/checker Clippy, formatting, the focused initialization-custody
test, 81 checker library and 27 integration tests with three existing ignores,
six documentation tests, and ten deterministic unlink-before-Begin repeats.
The earlier failures remain retained rather than relabeled.

Private integration `ef203560abdea979fec74fabbe6d1a47aeb7a521`, tree
`c380d245fb8cb00ff2c21632cbdc825a14461938`, also admits the exact private
engineering R29 input sibling in the source gate and refreshes both body
inventories. Exact 3e74 passes all 32 locked graphs, 38 source-gate tests and
byte equality for all three dependency TCB inventories. Fresh final ef203
metadata and actual source-gate checking pass: 175 modules, 8,466 executable
bodies, 724 verified labels and 7,742 unverified bodies. All 30 new engineering
capture/comparison bodies are proof-pending; only the separately proved actual
initialization helper gains a verified label. No prior labels are changed.
Host/metadata evidence SHA-256 is
`5d1f8144e4b2528046989aa8c00b7908113556e7aad7707fdf44242db19eac32`;
final coverage evidence is
`a6a1b9a78a2568f00d39772a83074598b6acc6592f28c8d9fe52a8529e71d1eb`.

The numerical pipeline is integrated as `6629dfb3`, byte-identical to reviewed
agent source `4ac12507` in all eleven changed files. Its separate engineering
input, capture, reference and comparison commands never enter qualification
acceptance. Exact 4ac/ccfd passes both capture cohorts (83 tests each, two
existing ignores each), 37 adapter policies, 17 comparator tests, three strict
Clippy commands and remote formatting. Python passes 23 existing and eleven
engineering tests. Both capture and comparator release binaries are retained;
the comparator's initial stage-cap stop and successful unchanged retry are
recorded separately. CPU-only real input generation/reopening now passes,
including the exact twenty-file bundle and byte-identical invocation output.

The first real engineering numerical case `prefill-s1-t128.001` completes on
mi300x physical GPU 1: native capture, pinned independent BF16/SDPA reference
and selected comparison all exit zero. The reference loads once and executes
twice with byte-identical outputs. Across all 151,936 finite logits, both
producers select token 198; token mismatch count is zero. Maximum monotonic
BF16 ULP distance is 31,121, so the rows are not identical. Independent
explanatory diagnostics report maximum absolute error 0.0703125, RMSE
0.01524191977257529, relative L2 error 0.01284705380109194 and cosine similarity
0.9999202347605426; top-ten ordering matches. No reviewed tolerance is inferred
from these observations. This is one case, not all seven R29 cases or a gate
closure. Its binary/reference source remains 4ac12507/ccfd, not 8113/4f6.
Evidence archive SHA-256 is
`816f162e5a2d3229239dc305e8658f8c570a269b44a077787d916030ec60335c`.
Both GPU phases return to their exact idle memory baseline. Completed owned
run/control/cache stages are removed after retention, reclaiming 350,916 KiB.

The initialization helper is integrated as `3d24f7e5`, with both changed files
byte-identical to reviewed `abfe390f`. The actual helper plus eight executed
callee bodies each pass a selected Verus run, 1 verified/0 errors. Ten
actual-helper mutations fail genuine postconditions, with no VIR errors.
The pinned 190-file closure matches around all nineteen proof jobs. Success
establishes the real target/draft initialization cursor relation and one-write
transition; failure frames the draft at helper entry. A caller's earlier page
append remains in poisoned custody, not rolled back. Caller routing, leases,
device writes and completion authority remain unproved. Exact 8113 passes 125
spec tests and 698 engine library tests with nine existing ignores, including
the new initialization and failure-custody tests. Adapter 109/one ignored,
37 policies, engineering capture 83/two ignored, numerical seven and owner
16 plus three policy tests pass. The final test-correction and executable-body
inventory checks above now pass at their stated source identities. The old
completed release target
is removed only after exact retention/provenance/no-live-use checks, reclaiming
2,060,340 KiB; independent binaries, current debug artifacts and sources remain.

The structural resident-loop team is implementing genuine repeated speculative
rounds. Audit found that ordinary structural continuation accepted unequal
target/draft cursors after full acceptance. The new path must reject that reuse,
execute the actual 425-packet draft catch-up and restore the original K4 shape
without promoting engineering artifacts to authenticated authority. This work
now has an explicit bounded repeated CLI in its separate worktree, but is not
yet integrated or GPU tested. The reviewed history fix exposes the original
speculative receipt, separate maintenance receipt and both queue transitions
without fabricating or retagging receipts. Its regression checks public API
nameability, not native epoch values. Candidate `dd0ec11e` also corrects a
missing helper import and duplicate inline attribute found by the latest
remote compile check. Exact dd0 now passes both the engine-library check and
the actual speculative-smoke adapter-bin check on mi300x, in 8.04 and 9.51
seconds respectively. Earlier failed checks remain retained. Linked tests
await additional reviewed workspace cleanup; no GPU validation is claimed.
The completed
single-round K4 rejection
smoke below does not cover that full-acceptance path. No new matched serving
TTFT/TPOT/throughput or competitor result is available.

The all-case R29 implementation uses explicit engineering reference V2
formats, either a selected canonical case or atomic seven-case execution, one
model load and two byte-identical runs per case. Its Python gate passes 15
engineering tests, all 23 legacy tests and the reference policy on mi300x; the
five legacy reference implementation/protocol/dependency files are unchanged.
The Rust input generator is updated to recognize V2, with a regression using
the actual protocol bytes. Exact combined candidate `4b7338df` passes all 24
differential tests and both focused protocol-test cohorts. Its benchmark-policy
step hits the unchanged storage cap after nested Cargo drops the outer resource
environment and starts fresh debug-profile artifacts. After reviewed cleanup,
the control-only retry passes benchmark policy on unchanged `4b7338df`. Strict
differential Clippy then rejects one implicit String clone. Follow-up
`940d37e2` changes only `case.kind.to_owned()` to `case.kind.clone()`. Its final
remote gate passes all eleven phases: formatting, all three strict Clippy
commands, all 24 differential tests (411.92 seconds, no ignores or filtering),
both two-test actual-protocol cohorts, 23 legacy and 15 engineering Python
tests, reference policy and benchmark policy. The original storage stop and
lint failure are retained. Private integration `10cc6587d198a315678c8e1d939aa8e5163a32f5`,
tree `047d0fe7a886bb94acbf59b55a6d4ab323cefa54`, includes all seven changed
files byte-identical to exact 940. Final evidence archive SHA-256 is
`a05d9b1a63950a01be2334f087b22c7477ccc1f259d75df8d449cbf09bc064dc`.
The earlier ef203 source-coverage result does not automatically cover these
new executable bodies; fresh coverage generation remains pending. These are CPU fixture checks,
not additional GPU numerical results. Python evidence SHA-256 is
`2912b40195e1d5e2f92da0dc970b55c1fe4bf6eb8f579e79e065ae5c98b0cd70`.

A fresh upstream fetch now finds fe2o3
`0fe50455b2fb65744048789bfc05f720313a12c5`, three commits beyond the validated
4f6 pin. The delta adds inert lineage records, explicit V2 entry-reachable
induction analysis/replay, and an optional immutable canonical analysis scope;
no KFD/runtime files change. Separate Ferric candidate `f9ff2f3c`, tree
`5322132be9403e3f56a8ae43589f8e55b556a8c2`, updates the same 80-file dependency
roster and exactly two derived verifier digests. Independent remote static
checks pass for 42 manifests, 32 lockfiles, 86 compiler/runtime manifest pins
and 1,539 lock package entries, plus exact scope/reverse-byte/hash checks.
The M0 binder's separate e527 proof-contracts identity is explicitly preserved
as required by the M0/M1 manifests; that crate is byte-unchanged through 0fe.
The initial overbroad all-pin assertion failure is retained, not hidden by
rewriting M0 proof identities. Static evidence archive SHA-256 is
`b1772379d463d444e3272ea7eb4e628d1fb07afa4f0d98a79cf533529a606d30`.
Actual locked dependency resolution, compiler/consumer tests and emission
remain pending. Existing artifacts and GPU results keep their older producer
identities. The completed candidate worktree is removed after archive
retention, reclaiming 44,108 KiB; the remote static-check stage is also removed.

Completed historical source cleanup removes only twelve fully reconstructed,
unused snapshots after fresh owner/inode/hash/roster/no-live-use checks,
reclaiming 515,568 KiB. The initial ordering-only cleanup failure and corrected
pass are both retained. Current source/build/cache/tool roots are untouched;
archive SHA-256 is
`9fc9b5c50e16b647f40a6fe543814a3a2b4eb08561d26d0e9cd59757b54ff5d8`.

A subsequent reviewed cleanup removes four completed gate source snapshots,
reclaiming 173,076 KiB after full archive/roster/byte/state/no-live-use checks.
Its retained completion archive SHA-256 is
`e7261750434de5c16e33c8103334558334150ca211f348d8d140019a8dea4991`.
The R29 lane separately removes exactly 1,212 unused default-profile artifacts,
reclaiming 488,812,544 bytes; all 298 protected executables rehash identically
and 131 unclassified files remain untouched. These cleanups restore space
under the unchanged shared-stage cap. No foreign work, active cache, current
tool or independent retained binary is removed.

The completed R29 local worktree is also removed after exact integration and
archive retention, reclaiming 44,128 KiB. The runtime lane removes its two
obsolete source snapshots after archive/byte/no-live-use checks, reclaiming
another 86,884 KiB. Final linked runtime tests still need additional headroom;
the next cleanup is limited to reviewed completed sources and retained
duplicates, not active target or shared cache contents.

The kernel follow-on now has a concrete compiler prerequisite. M1's canonical
weights are physical W[N,K], while the current typed MFMA B loader accepts
row-major logical B[K,N]. A dedicated fe2o3 worktree is implementing a distinct
checked column-major B view, authenticated layout propagation and address
lowering, with explicit new semantic-wire admission. This is reusable compiler
work only; Ferric owns the subsequent GEMM kernel and schedule. Existing scalar
GEMM promises ascending separate FP32 operations, so MFMA requires a distinct
numerical/profile identity rather than relabeling that schedule. Small target
verification batches M5/M9/M17 must be covered; repeated S1 draft decode stays
scalar. No new compiler test, emitted MFMA, numerical result or speedup is
claimed for this in-progress capability.

## Earlier Integration Checkpoints

Tested private integration `d55fd00a44a01dce3f4c8c790b4736e2440a69fa`, tree
`9c7bf3ffa948acdcb65b766a18b1ada757a20db1`, pins published fe2o3
`85255498a021ed2b7a1a511f99f577ee4b11d175`. Pliron remains
`161c385576d45d4e634ba179fa93a545b91124e6`. The 80-file pin migration reverses
byte-for-byte to its parent before two separately checked derived verifier
manifest/lock digests. Historical tool and binary producers are not relabeled.
Intermediate integration `61a9ab324cdd0a6e745aa838d0d0f1ca1ac137f5`, tree
`daaec77b5a07a7c749c9719d387e7dad8f28ee03`, advances that same 80-file roster
to upstream `82d95ca81f3f3a55e1e82b88918a41cca8b20c31`, which indexes checked
control flow and transport pipeline catalogs. The reverse-byte comparison and
two derived digests are checked; remote formatting passes. Exact 61a9/82 passes
all 32 locked graphs, 38 source-gate tests, 31 verifier policies, six source-pin
policies, source coverage and byte equality for the three regenerated TCB
inventories. The full engine host results below remain bound to 852.

Compiler-tested integration `f17efe83d37105c0da78ea2c8bbbe166a0ee4f43`, tree
`a3292fb909cbbaddff4d25a323a9f9421b760258`, pins published fe2o3
`c6b4050dd6c18e1868b24e4580da3eed8a3d19fa`. This is the same 80-file
reverse-byte-equal migration with separately checked derived digests and remote
formatting. Fresh c6 consumer and source-policy validation remain separate
obligations; earlier 82/852 results are not relabeled.

The c6 combined code candidate is `6651f2d1a1739f30c35a4a5c34d2c900be1c5d06`,
tree `0c98c7de8f5b791ed9fd23cd8fa6f6cdedede556`, still pinned to c6b. It
integrates the executed catch-up metadata helper and independent-checker IPC
client, plus two remotely identified formatting corrections. Two focused spec
tests and the engine initialization/settlement test pass on this exact source.
The release engineering target-smoke binary builds successfully; its independent
11,783,960-byte copy has SHA-256
`18ff100fde890a25bbeb0588c0723e741674f737a6a31af5a37e4e2255a29e43`.
These are host results, not GPU correctness or serving measurements.

Current integration `bb2b0123f4f96410269a098d7cba8aa7bb950008`, tree
`b15664e855b5494eb312242dfdd26d266fba432a`, advances the same 80-file roster
to the latest fetched fe2o3 main `ccfd43d6bc58b27e0efe510b0ee1b8463166dfc2`.
Reverse-byte comparison, the two derived verifier digests and remote formatting
pass. The upstream delta is confined to semantic-SSA transparent-borrow
occurrence handling and tests; KFD/runtime sources are unchanged from c6.
Fresh ccfd Pliron tests pass: 1,021 library, 58 integration and 13 doctests,
including all fifteen new focused tests. All 543 backend tests and exact
release CLI/backend builds also pass. Matching bb2 source, worker-closure,
37 aggregate host tests and vendoring pass. Initial overlay/emission attempts
stop at the unchanged 12-GiB owned-stage cap. Those resource stops are retained
separately from compiler failures. After bounded cleanup of completed owned
outputs, same-source overlay and actual emission pass with exact replay and
all twelve expected entry/descriptor pairs. The latest 103,872-byte HSACO has
SHA-256 `46335b09a921b33ed66392e415ce3d2348dd63042242c0387d5a58362ba3c84c`;
its manifest SHA-256 is
`339ea2c2c4528a28d7ca16085a31c57070a304b4a2b5454ef118e4ce0a0d66a2`.
It differs from c6 and does not inherit c6 native results.
Exact bb2/ccfd passes 123 spec tests, 697 engine library tests with nine existing
ignores, binary groups 11/2/75/2 with two existing ignores in the 75-test group,
171 doctests, strict spec/engine Clippy and formatting. The checker normal-only
production build passes with actual `fe2o3_host` artifact features `[]`.
All 71 host phases finish successfully, including checker/adapter/owner tests,
strict Clippy, policies, 32 locked graphs, 38 source-gate tests and byte equality
for all three regenerated TCB inventories. Three independent release binaries
(target, speculative and R29 engineering diagnostics) also build successfully.
Historical c6/82/852 results are not relabeled. The later docs/inventory-only
checkpoint `d09430897232759fb9bbb2a3485f60da69bef890` does not change the bb2
production-code or binary identity.

| Team | Completed Since Prior Checkpoint | Work In Progress |
| --- | --- | --- |
| Compiler / Kernels | Published c6b resolves the checked unsigned-subtraction assertion. Latest bb2/ccfd tools, 37 aggregate host tests and actual 12-root gfx942 emission with exact replay pass. Independent ELF inspection confirms the expected entry and descriptor sets. The latest four-token GPU smoke succeeds. | Full numerical comparison and kernel hardware validation. The emitted artifact and engineering smoke grant no protected publication or qualification authority. |
| Runtime / Integration | The resident loop implements the 425-packet draft-only maintenance step and restores the original speculative shape while retaining queue, KV, checked completion and original target bindings. All 71 bb2/ccfd host phases pass, including 697 engine library tests, service/adapter/owner checks and strict Clippy. Three engineering release binaries build. | Single-round speculative smoke, then repeated native full-accept maintenance/restore, cancellation and faults, physical generation accounting and warmed native allocation checks. Host tests do not establish completed physical catch-up or served-request execution. |
| Verification | Exact 940a/82 verifies the executed catch-up metadata helper and six selected dependency bodies individually with Verus: each 1 verified, 0 errors. Seven actual-body mutations fail genuine postconditions. The integrated helper is byte-identical. Final d094 coverage regeneration and equality pass. | The next executed initialization helper is implemented separately and undergoing its first proof run. Caller routing, page append, lease and completion authentication, broader M1 proofs and hardware qualification remain open. |

The earlier 446ac5db/829 focused joined-Pending tests cover all 31 acceptance lengths across K4/K8/K16,
terminal full acceptance, foreign coordinators, substituted choices, zero
generation and epoch exhaustion. Their observations are diagnostic test fixtures,
not native queue or completed-read witnesses. The KV tests use real component
transitions but do not execute GPU kernels. Maintenance emits no served token and
does not increment the logical speculative round; only its physical epoch advances.

The first 852 emission attempt hit the 12-GiB owned-stage cap and is retained as
a resource stop. After removing completed owned duplicate inputs, the unchanged
retry completed within its controls and rejected the arithmetic assertion above.
The same-source comparison uses byte-identical native MIR but 829 stops first at
a prefill assertion. A scratch-only isolated paged root also changes its first
failure (829 line 454 versus 852 line 373); source-line order is not canonical
semantic-block order. A subsequent full comparison of all 49 isolated paged
assertions finds zero true-to-false changes. 852 newly proves bb33/line 454;
bb97/line 373 is the only remaining unproved assertion and was already unproved
under 829. This is forward proof progress, not an observed regression. The
missing precision was the successful checked `8192 - committed_tokens` result.
Published c6b now retains that unsigned bound only after the existing exact
checked-success authentication. The producer assertion remains an independent
mandatory obligation; the result cannot prove its own producer. Tests reject
eleven missing, changed, non-dominating or escaped authority cases, signed and
out-of-width literals, and preserve the existing evaluator work limit. No
kernel guards or additional evaluator calls were introduced. Actual aggregate
emission now succeeds at exact f17/c6b, resolving this observed rejection.

The emitted code-object V6 targets `gfx942:xnack-`, contains 103,872 bytes and
has SHA-256 `9e04fe0c8c7682146b9e42d21c83d5d77336d08b7d599cbd86a012eb9eef69a1`.
Its engineering manifest SHA-256 is
`ff3dfe2c8b75996b52d69c041197ce12ba8f6fde94d31d6f8d8c53be4c6b691d`.
The compiler-bound inert handoff is 414,259 bytes. Actual output replay is
byte-identical. Independent structured ELF inspection confirms exactly twelve
defined global entry functions and twelve matching 64-byte descriptors; this
set equality does not attest Ferric policy descriptor-table ordering. Authority
is `none`, and publication/load/launch grants remain false.

The bounded one-token engineering smoke on mi300x physical GPU 1 succeeds,
using the exact 6651/c6 binary, f17/c6 artifact and bed11d00/829 snapshot.
The five-token raw prompt `The capital of France is` produces token 12095,
decoded as ` Paris`, with actual hardware completion and terminal exit zero.
Artifact admission, model preparation, KFD binding, memory initialization and
controller execution all complete. Selected-device memory returns to its
298,647,552-byte baseline and busy status returns to zero; the existing foreign
queues remain on GPU 0. All run resource guards pass. No reset is performed.
Initialization/upload completes at 240.793 seconds cumulative; controller
duration is 14.714 seconds and its first-token offset is 13.608 seconds.
These are raw engineering diagnostics, not comparable TTFT/TPOT measurements.
One token is not TPOT-eligible. This establishes prompt-to-token execution for
this exact path, not numerical parity, speculative catch-up, serving readiness,
performance competitiveness or protected qualification. All 33 gates stay open.

The separate latest bb2/ccfd four-token run on mi300x physical GPU 1 finishes
at 2026-09-14T21:30:31Z with exit zero and observed hardware completion. The
same five-token raw prompt produces IDs `[12095,13,576,6722]`, decoded as
` Paris. The capital`. This matches the first four tokens of the retained
independent BF16/SDPA reference frozen before the candidate result was read.
It is a one-prompt token spot check, not full numerical parity or an R29 logit
comparison. The immediate after-state records memory still being released;
a later independent check returns to the exact pre-run 298,647,552-byte VRAM
baseline and zero busy status. No reset is performed. Initialization/upload
finishes at 246.103 seconds cumulative, followed by a 23.100-second controller
duration. These remain diagnostic boundaries, not comparable serving metrics.
The separate bb2/ccfd single-round K4 GPU smoke also passes. Paired prefill
uses 128 active tokens, including 123 end-of-text suffix tokens, not masked
padding. The real speculative graph accepts zero draft tokens and emits target
correction 4710, with checked completion and terminal queue teardown. This
exercises rejection and retirement, not full acceptance, draft maintenance,
repeated rounds, numerical correctness or serving performance. A later sysfs
check confirms return to baseline VRAM without reset. Both run archives and
their distinct immediate/delayed cleanup observations are retained.

The final d094 source inventory passes remote regeneration, committed-byte
equality and validation: 173 modules and 8,435 executable-body identities,
with 7,712 unverified and 723 verified labels. All earlier 722 verified records
are unchanged. The added verified label is the executed metadata-commit helper
covered by the scoped proof described above; the new engine preflight remains
pending. This does not establish a whole-crate or physical catch-up proof.
The verification team's next actual initialization helper is implemented in
a separate candidate, with its selected-body Verus run in progress; it is not
integrated or claimed proved here.

Canonical target/draft admission, full streaming prepack and separate persisted
snapshot reopen verification both pass on the exact bed11d00/829 release CLI.
The new snapshot has all eleven canonical files, including 16,381,470,720 target
and 1,503,264,768 draft weight bytes. Both processes reproduce bundle identity
`6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b`.
The old six-file snapshot is retained as an explicit rejected input, not modified.
The exact d55/852 release generator separately passes `--check` against committed
generated sources. This is source equality, not native image or GPU evidence.

Retained local evidence under `.codex-tmp` includes:
- `ferric-native-ccfd-v1/speculative-k4-evidence-r1/ferric-native-ccfd-speculative-k4-evidence-r1.tar.gz`,
  SHA-256 `f04e1b431f8d3c8656a76555aaa7cd0d8f55cc8b470811536f8d7f9a3d698b92`.
- `ferric-catchup-initialize-proof-gate-v1/proof-abfe390f-evidence-r1.tar.gz`,
  SHA-256 `208ff78e73737cfa01af3cdf6a323ce57419c342b9b8484622488abd2340ee29`.
- `ferric-engineering-r29-evidence-final.tar.gz`, SHA-256
  `176ec9eb77a200bdc3aa33900f2e53b4cc06997f05bab9687c2105ce8b4eab88`.
- `ferric-native-ccfd-v1/target4-evidence-r1/ferric-native-ccfd-target4-evidence-r1.tar.gz`,
  SHA-256 `2a9b13d81098a878ad1f299a7f4f999c368c16aca1a7f906cd9dc283f1e9dffc`.
  The separate `TARGET4_RECEIPT.md` records the delayed settled-GPU observation
  and cleanup of the owned completed run, without rewriting its raw after-state.
- `ferric-current-c6-host-gate-v1/host-bb2-ccfd-evidence-r1.tar.gz`, SHA-256
  `4848bf1306627d8c4b932ac75decf83d9efd20252b2183bb8699e89cf7536fe9`.
- `ferric-current-c6-host-gate-v1/release-bb2-ccfd-evidence-r1.tar.gz`, SHA-256
  `4a18fe27f2449896169dfc2b9e9426a959695ac9253b37ad0cab40e7866fa0b2`.
- `ferric-current-c6-host-gate-v1/coverage-d094-ccfd-evidence-r1.tar.gz`, SHA-256
  `209c1e49d96e91a93010e248a444db0f2b2e92bbba8ee7a801aee899b04d179d`.
- `ferric-divisor-diagnostic-gate-v1/acquisition-ccfd-emission/ccfd-published-evidence.tar.gz`,
  SHA-256 `3eb1c0834b2ab957efa13b063921a25516713c4bb1a0c89cb0cd5fe1fa828107`.
- `ferric-draft-catchup-gate-v1/host-evidence-r10.tar.gz`, SHA-256
  `5bb81795371f3cb9340cfd8dd9208276a5bd5492457b43404e2696d2f1d13036`.
- `ferric-catchup-inventory-v1/catchup-inventory-evidence-r4.tar.gz`, SHA-256
  `5001522110d67c40ac177aae320da36f842092e148dce0f752818eac87190ad7`.
- `ferric-upstream-852-metadata-v1/metadata-852-evidence-r1.tar.gz`, SHA-256
  `1074fbf60b9643f13044c13ae2cd12a00502644371ab69a98efec38fce327f63`.
- `ferric-upstream-82-metadata-v1/metadata-82-evidence-r1.tar.gz`, SHA-256
  `09a16fad2b3d5630287c5882e6c90cf5fdec916461b3d65eb4eb361c44a2c6b5`.
- `ferric-divisor-diagnostic-gate-v1/acquisition-assertion-differential/assertion-differential-evidence.tar.gz`,
  SHA-256 `6916aeae89906f74bef47c7ad422dbfe34fd4185e77d7b96728ff1750c4231a7`.
- `ferric-divisor-diagnostic-gate-v1/acquisition-literal-subtract/literal-subtract-r3-evidence.tar.gz`,
  SHA-256 `5db7a2048a60d5790767f0eeb4f8a5860df4349c9db6da0dff3544ba53e0c031`.
- `ferric-divisor-diagnostic-gate-v1/acquisition-literal-emission/literal-published-emission-evidence.tar.gz`,
  SHA-256 `943179eb018d6d646ea4c5f00609ec5a7b7bc864d9589c7a89e99e2e52dfebea`.
- `ferric-catchup-metadata-proof-gate-v1/proof-evidence-r1.tar.gz`, SHA-256
  `2bd13649ff143a15a989c508e2bd30aa9d2777f9485256edc06b0b8daf5f3ea7`.
- `ferric-checker-ipc-gate-v1/checker-evidence-r8.tar.gz`, SHA-256
  `b0b2b7d65326607298332a6289448633f5d60f6173ec3484bbae95a06f46fd0b`.
- `ferric-current-c6-host-gate-v1/smoke-6651-c6-evidence-r1.tar.gz`, SHA-256
  `864bb8bd823d1a4e44e664383857e58886c007d18ce118d59872b0a735d110eb`.
  This retains the exact source-byte comparison, focused host logs, release
  build logs and independent smoke binary; it is not native-run evidence.
- `ferric-native-target-c6-v1/evidence-r1/native-target-c6-evidence-r1.tar.gz`,
  SHA-256 `0dc8ca2a724bde837750bae565a4224feae03e6f88fde50913ed35c58199999d`.
  This separately retains the reviewed control, exact native binary, actual
  report, startup diagnostics, resources and before/after GPU facts. Its
  authority remains `none`; it is not a qualification receipt.
- `ferric-divisor-diagnostic-gate-v1/acquisition-polarity/polarity-evidence.tar.gz`,
  SHA-256 `6a8ed82ec6eee1d8fda59833745df6847718093b7f9c3eb7e4d4d164a4be40bb`.
- `ferric-m1-bundle-current-v1/evidence-r1.tar.gz`, SHA-256
  `54547e1b14bb419f189d35b98c698d4928a4e9adb37ca232c01cba546fd1d60d`.
  This contains the prepack CLI, controls, logs and non-weight snapshot files;
  the large weights remain only in the owned remote snapshot for native use.
- `ferric-m1-bundle-current-v1/generated-source-evidence-r1.tar.gz`, SHA-256
  `ca256a209e92e4808781cc2f80774132bc30cc653d584d5b804f69a4d639f1ad`.
  This separately retains the d55/852 generator and actual source-check logs.

Earlier compiler, source-macro, stale-README and Clippy failures remain retained
with their original source identities. No checker or admission rule was relaxed.

The Pages-only checkpoint `454b7cd8` is pushed to public main with only public
ancestry. Workflow `34900342080` succeeds and all seven live assets match the
reviewed 550,455-byte artifact. The source/browser archive SHA-256 is
`a4f8f4cbf235b030ac7e7f50d65c28bf770e01b71b23d454011ac114fe2f0d97`;
deployment archive SHA-256 is
`30d9d747d7d9f62cbcb409165a7b47c84297786c834b9e063af3e8ca848aca62`.
The site reports the separate ccfd four-token and c6 one-token observations,
current ccfd host tests and d094 scoped coverage. It does not yet report the
later K4 run or new 4f6 integration; performance data remains unchanged.
The completed Pages stage/worktree are removed. No private ancestry is pushed.

Earlier Pages-only checkpoint `524bfb70` was pushed to public main with only public
ancestry and five site-file changes. Remote browser and scope validation pass;
workflow `34895549244` succeeds for the exact commit and all seven live assets
match the reviewed artifact (539,684 bytes). Deployment archive SHA-256 is
`29cbdf447565b5ba65b519ad37ae911026703c8284c6e3598d525bca2c1f8540`.
The site reports the c6/f17 emission and resolved assertion comparison while
keeping current-pin host validation pending. Performance data is unchanged.
The completed Pages worktree and owned stage are removed; the stage cleanup
reclaims 74,500 KiB. No private implementation ancestry is published.
Completed runtime and proof worktrees have been
removed after exact-source/evidence checks, as has the obsolete owned compiler
stage in `/tmp` (1,073,320 KiB reclaimed). Active build caches, the new model
snapshot, original dirty worktrees and other users' state remain untouched.
The shared host stage additionally reclaims 6,730,804 KiB from exactly 2,542
obsolete owned build outputs after source provenance, hash and no-live-use
checks. Obsolete ae441/829/852 compiler tool copies reclaim another 596,056 KiB
after retained-archive and no-live-use checks; c6 tools and native inputs remain.
The completed native smoke run and control stages are removed after archive
acquisition and no-live-use checks, reclaiming another 14,792 KiB.
All executable validation runs on mi300x. No new TTFT, TPOT, throughput, GPU
correctness or competitiveness result is claimed.

Engineering R29 capture already emits real BF16 logit rows and argmax tokens,
but the existing reference/comparator correctly rejects its nonqualifying
transcript. A separate selected-case engineering comparison path is now under
implementation, retaining numerical and identity checks without producing
qualification acceptance. The selected comparator prototype passes 17 Rust
tests and strict Clippy. The extended engineering reference/input candidate
passes 23 existing and eleven new Python tests plus policy checks; its new
Rust input/capture path is not yet tested or integrated. Its schemas explicitly
remain separate from qualification closure and acceptance policy. The first
planned case is `prefill-s1-t128.001`; existing decode cases require 8,192
teacher-forced rounds. An isolated, version-checked reference environment is
prepared remotely; it has not yet run the numerical comparison on the GPU.

The next native path has an additional explicit boundary: engineering HSACO
emission does not produce an authenticated Worker V3 selector. The protected
compiler profile and supervisor socket are not provisioned on mi300x. Ferric's
concrete independent-checker IPC client is now integrated. Exact agent source
20175f93/82d passes 81 service library tests (including eleven checker tests),
27 integration tests with three existing ignores, six compile-fail doctests,
strict all-target Clippy and a normal-only production check. Actual normal
compiler-artifact receipts show no fe2o3-host test-support features. Two root-owned
wrapping-only formatting failures are corrected in 6651; the full latest-pin
service checks now pass in the bb2/ccfd host suite. Earlier resource-cap stops and the relocated
test-factory policy failure are retained separately, not counted as successful
tests.
It sends every envelope and HSACO byte to the pinned preopened endpoint under
one absolute deadline and rejects mismatched results or ambiguous channel
reuse. The supervisor admission is explicitly unsafe and requires a separately
measured real checker. Neither the client nor its tests supply theorem results.
Actual checker authority, protected compiler/current records, separate signing
authority, durable antirollback state and reviewed deployment remain required.

## Prior Initial 829 Checkpoint

Private integration `b039c17d3287c25d3611e3e2be544003bd9c5a30`, tree
`a6ccddda03b079664bd0a40708a802362d829390`, pins published fe2o3
`82950a3cfc7b0192159fb7afcd1b89fb48d56053`. Pliron remains
`161c385576d45d4e634ba179fa93a545b91124e6`. All 79 active pin files reverse
exactly to their parent before refreshing the two derived verifier manifest
and lockfile SHA-256 arrays. Historical binary producers remain unchanged.

| Team | Completed Since Prior Checkpoint | Work In Progress |
| --- | --- | --- |
| Compiler / Kernels | Actual aggregate extraction at ae441 attributes the divisor rejection to paged GQA coordinates at line 322. Ferric `6da025e` specializes five coordinate expressions with literal divisors; all 35 aggregate host tests pass, including AST equivalence across 14 profiles. Actual extraction then reaches an unproved overflow assertion at line 378. Native and NLL MIR identify the guarded `query_position + 1`. The statement-ordered upper-bound repair is rebased onto newer upstream `5a762019f`, validated and pushed as `82950a3cf`. Exact rebased suites pass: compiler-lineage 38, kernel-analysis 115, focused backend 2, full backend 536. Strict Clippy has no added/removed normalized fingerprint against 5a; it is not an upstream lint-clean result. | Build exact published tools and emit the new aggregate. The host regressions do not prove the actual aggregate now emits. No new image or GPU result. |
| Runtime / Integration | Published fe2o3 `b750` exposes the generation held by a completed-read request; service-host 43, service qualification 45 and three pure KFD tests pass. Local Ferric adds preallocated catch-up inputs/pages, checked one-token draft settlement with zero logical output, exact readback generation checks, retained original bindings and distinct maintenance/restore queue transitions. Registry `06502842` retains Pending/InFlight/Completed phases, including abort and cancellation custody. | Integrate maintenance execution, checked readback and restoration into the real resident loop. Later runtime `12598683` is under integration and is not part of the green full host snapshot below. Same-owner witness registration and intermediate physical epochs still need actual resident tests. |
| Verification | Private `fc194dac` with published b750 passes 34 focused registry tests and 693 full engine library tests, with nine existing ignores. This includes seven new metadata-only registry tests and three diagnostic-reset tests covering K4/K8/K16, exact extents, invalid-token sentinels and rejected substitutions. Earlier selected Verus `kv_physical` remains 4 verified, 0 errors at bd2/e6. | Fresh 829 dependency graphs, source policies and structured TCB equality; latest source coverage with new bodies truthfully unverified; real resident witness/failure tests. No warmed catch-up allocation, physical catch-up proof or gate closure is established by these host tests. |

The registry keeps the logical speculative parent while exposing the actual
425-packet maintenance shape. A sealed pending witness is required before
maintenance publication; aborting either publication retains the obligation.
Ordinary completion cannot settle maintenance. A checked maintenance witness
advances the physical epoch without adding a served token or speculative round;
the next real speculative publication restores the original K4/K8/K16 shape.
The tests exercise metadata keys, not fabricated physical completion witnesses.

The diagnostic reset uses the canonical `u32::MAX` unwritten-token sentinel,
not zero. Exact draft/target extents are 16/20, 32/36 and 64/68 bytes for the
three singleton widths. Missing device writes must not become valid token zero.
Independent source review found no further concrete registry or lower restore
defect, but the real resident association remains a separate integration test.

Evidence acquired locally includes:
- `ferric-catchup-composition-v1/REGISTRY_RESET_RECEIPT.md` and raw logs. The exact
  fc194 source archive SHA-256 is
  `321c3cd04fb2dc990e08d7460de05fbee1c16fbb1036ae5346e35dd85ff3f76a`.
- `ferric-divisor-diagnostic-gate-v1/acquisition-paged-getter/paged-and-getter-evidence.tar.gz`,
  SHA-256 `cf6e686834c3f7ce714d3dfc83c8e49e1c7cc734cc5ddcbe4c1e1e3300258b1e`.
  It retains the host/getter gates, archive mode-control failures and actual
  overflow rejection. The newer MIR and 829 tests are separate evidence.
- Pages-only public commit `a7a38b26` deployed successfully in workflow
  `34883199424`. All seven live assets match reviewed source bytes (530,597 bytes).
  Performance data is unchanged. Its clean worktree and owned 35 MiB remote stage
  were removed after evidence acquisition; no private implementation was pushed.

All builds, tests, proof runs and formatters remain on mi300x. No new TTFT, TPOT,
throughput, GPU correctness result or competitiveness claim follows. The 693-test
suite is bound to b750, not the later 829 pin or later resident integration.

## Prior Ae441 Checkpoint

Private integration `6d10fb900574dcdd11d84364f54b8d90ced4856a`, tree
`3fb0f878ae22b5c72697b9f18ebf18c0aa36d69a`, pins published fe2o3
`ae441734f27ef35c26baf7b3a6281852cd544de9`. Pliron remains
`161c385576d45d4e634ba179fa93a545b91124e6`. The 79-file pin migration reverses
exactly to its parent before the two derived verifier manifest/lock digests
are refreshed. No kernel bytes or historical producer identities change.

| Team | Completed Since Prior Checkpoint | Work In Progress |
| --- | --- | --- |
| Compiler | Bounded typed divisor diagnostics retain root, innermost assignment, source and operand context without changing the nonzero acceptance predicate. Root fetched, rebased onto new main `836afb284`, validated and pushed `ae441734f` to main. Exact rebased tests pass: compiler-lineage 30, kernel-analysis 97, kernel-ir 291, focused backend 16 and full backend 534. Strict Clippy has no fingerprint delta against pristine 836; upstream is not lint-clean. | Build exact published release CLI/backend and emit the matching aggregate. The prior divisor failure still lacks actual root attribution until that run. No new image or GPU result. |
| Runtime / Integration | Local commits `f8c8071`, `385f687` and `68e1f68` add parent-bound K4/K8/K16 catch-up composition, exact workspace/buffer bindings, maintenance output-tag checks and a sealed coordinator transition. | Integrate authenticated reservation, queue/registry phases, readback and resident execution. The new engine changes are remotely formatted but not yet compiled/tested together; full acceptance still fails closed before another ordinary round. |
| Verification | Actual Verus run on `bd2f246` with e6/Pliron161 verifies four items in selected `kv_physical`, zero errors, with exact pinned 190-file closure before/after. Root integrates the strengthened relation as `e6f5151`. | Implement private maintenance receipt/readback and validate the combined runtime checkpoint. This proof does not establish the physical catch-up path or close a roadmap gate. |

Catch-up composition is 424 canonical draft-decode rows followed by the single
canonical target compact row. The full source expansion identity is retained
even though only its last row is projected. Draft argmax writes the exact
four-byte completion `Choices` range consumed by compact; no target KV row runs.
The coordinator transition requires matching parent/request/coordinator,
round, consecutive physical epoch and dispatch generation, active member and
an exact one-token cursor gap. It changes only draft committed position and
last physical epoch, leaving target position, output count, bonus anchor and
logical round count unchanged. Focused positive/rejection tests are added but
await the combined host run.

The preceding e6 migration passes all 32 locked metadata graphs, 38 source-gate
tests and 31 verifier-policy tests. At immutable `e6f5151`, fresh root/verifier/
source-gate metadata and regenerated production/dev/full dependency inventories
match committed bytes. Inventory deltas are only the two new Pliron test targets
in each verifier scope. These results remain bound to e6; the newer ae441 pins
still need their own metadata and consumer validation. New catch-up executable
bodies also need refreshed source coverage with truthful unverified status.

Retained local evidence includes `ferric-catchup-proof-gate-v1/proof-evidence-r2.tar.gz`
(SHA-256 `4d58e31661411697cc82bd09eb0ce055374f71f2cea65b911d64a88b0457dc57`)
and `ferric-divisor-diagnostic-gate-v1/acquisition-ae441-tests/ae441-tests-evidence.tar.gz`
(SHA-256 `cc74d89c88943c2fe9208e3db4925f827e1d7e12f3097e899c1e947ea770e14e`),
under the owned `.codex-tmp` evidence root. No new TTFT, TPOT, throughput,
hardware qualification or performance parity is claimed. Builds, proofs,
tests and formatting run only on mi300x, in bounded owned scratch storage.

## Prior Eaa Checkpoint

Private integration `0b2d53a` advances the 79 active dependency-pin files to
published fe2o3 `eaa057ac34f8fc3cfda3dd2c2cb7df058560de06`, tree
`d356bb5f5f7ffbb1e5d4b46991f246cf3edc176b`. Exact reverse substitution
reproduces `8d29d70` except for two separately validated derived verifier
manifest/lock hashes. Historical images and benchmark receipts retain their
actual producers. The installed ae267 KFD worker's receipt-derived nine-crate
source closure, workspace manifests and toolchain remain byte-identical through
dc8 and eaa; it is not relabeled as a new build. All builds, tests and formatting
run on mi300x, not locally.

A final upstream check finds the newer `e6cfa668f21e5b80d2e70730ed61602c1aaf5804`,
which advances Pliron to `161c385576d45d4e634ba179fa93a545b91124e6` for rewrite
observers and dead internal block-argument elimination. The divisor predicate
and eaa resource fix are unchanged. The KFD crate trees remain unchanged, but
the workspace manifest/lock do not. This newer revision is tracked, not yet
validated here: migration requires refreshed dependency inventories, compiler
builds/tests and aggregate emission. The results below remain bound to eaa.

| Team | Current Work | Acceptance Still Required |
| --- | --- | --- |
| Compiler | Resource fix eaa057ac is rebased on c76f844 and pushed to main. All 1,006 Pliron tests and strict Clippy pass; the 13 focused tests are a subset. Exact published CLI/backend builds and 33 aggregate host tests pass. | Actual aggregate emission now rejects an unproven nonzero control divisor. Attribute the kernel/expression, repair the appropriate layer, emit the aggregate and inspect ABI/resources. No image or GPU result exists for this attempt. |
| Serving | Private 5c74b6f implements S1/K4, K8 and K16 through prefill, registry, queue storage and readback. Private 624d9a2 exposes selection through the sealed owner plan. Private 8d29d70 fixes recipe/preparation workspace identities in the real builder. Legacy canonical bytes and K4 wrappers remain compatible; S8 is not added. | Public-eaa host suite at 9e4d818 passes 659 engine tests (nine ignored), 109 adapter tests (one ignored), 37 adapter policies and 31 verifier policies. The service passes 16 tests and three policies. Authenticated gfx942 execution and serving qualification remain open. |
| Verification | Review found the existing full-accept draft cursor gap. The resident path now rejects another common-anchor round when target/draft cursors differ. K16 storage distinguishes seventeen logical verification choices from thirty-two physical rows. | A real authenticated draft catch-up dispatch, completion-bound cursor advance and repeated full-accept execution. The engineering catch-up path is not a substitute. |

The earlier dc8 rejection occurred at the existing GEMM vector root with 385
projected blocks and 77,791,232 invocations. Its deterministic resource accounting
predates dc8; it was not an OS memory limit or an identified dc8 regression.
Published eaa distinguishes unreachable exact fallback from reachable bounded
Presburger enumeration, preserving mandatory checks and hard limits. Compiler
test archive `de075f4d` retains both the initial fixture failure and corrected
passing run. Exact eaa emission passes the earlier resource blocker, then rejects
"a division or remainder used for deterministic control lacks a statically nonzero
divisor". The diagnostic does not identify the root, so the new failure is not
attributed to GEMM. Archive `4655edd6` and emission log `8529eed1` retain this
failure; no image, replay or native execution is claimed. Diagnostic root and
expression attribution is the next compiler step, before kernel specialization.

The first host attempts retained an upstream feature-environment failure and a
test-only coordinator API error before the successful engine suite. Subsequent
adapter compilation found an existing numerical module missing its batch-feature
gate. Private 6f0621d adds that gate, a source-policy regression check, and an
assertion consuming the resident test's must-use round outcome. These corrections
do not enable engineering features in the protected service. The corrected
deadline policies at `2e073b5` pass, including eighteen bounded-wait mutations and
five terminal-custody mutations. All 32 locked metadata graphs pass. At
`8d6813f`, the isolated warmed planning test passes for K4, K8 and K16 with zero
allocations/reallocations; formatting and strict engine Clippy pass. This checks
host planning only, not physical GPU rounds or end-to-end performance. The unused
clean fe2o3 worktree is removed; the user's dirty original checkout is untouched.

The regenerated source inventory contains 173 modules and 8,252 executable
bodies: 722 existing verified bodies and 7,530 unverified bodies. Its delta is
eighteen added unverified bodies and one removed K4-specific helper; all new
bodies are recorded as `pending-verus`. The 7,512 unchanged rationale records
remain byte-identical. Seven upstream test-target metadata rows are refreshed
in each verifier dependency scope. There is no proof-status upgrade.
Final validation at `dd5ade0` passes source coverage, regenerated coverage and
dependency-inventory equality, fresh locked metadata, and both policy checks.
The retained final source archive has SHA-256
`f2d73e96953334cc7d9cf934288a6c15ea8b1d533d2a25a7ba02e82bfa17c5f0`.

The cursor gap is a draft-prefix correctness issue; this review did not
demonstrate a target-token mismatch. Fully accepted rounds may still terminate
normally when their output policy is complete. Continuing such a request must
fail closed until the missing last accepted draft token has been processed by
a real catch-up step. Host cursor adjustment would not repair its KV contents.

No new TTFT, TPOT or throughput result is claimed by this implementation work.
S8 authenticated cohorts, mixed-active admission, cooperative gfx942 kernels
and the remaining M1 proof/hardware/performance obligations remain open.

## Earlier Checkpoints

At the September 13 checkpoint, upstream fe2o3 main resolved to
`b9ab3553d1e7f87751a24892ba66a591408ee31f`, fetched without changing the
existing dirty fe2o3 worktree. This adds shared bounded semantic verification
and canonical V12 admission; it is a substantive compiler change, not a
revision-only validation. The worker's declared local dependency closure is
byte-identical from ae267 through b9, including KFD and manifests. That does
not relabel its actual producer. A separate compiler migration still needs a
fresh build, verifier/consumer regressions and V15 emission/ABI checks. No
fe2o3 source edits or push were required for the completed comparison.

The frozen comparison and then-active integration were pinned to
`c9c7036cf98034b638886f54a745701e70ac3571`, with Pliron
`9de42fc6ca7b8f3500ccf2346d69ebbb36e889cd`. The two commits since ae267
change private SSA operation-order verification/resource accounting, then add
explicit raw KIR V12 codecs with fail-closed vector/event admission. Inspection
finds no V15 scalar path or runtime ABI change. Runtime/KFD, device SDK, LLVM
worker, all dependency manifests and the upstream lock are unchanged.
Controllers still transitively depend on changed compiler crates.
Private `3ababa7` updates the same 79 active files to `18946f6`
by exact revision substitution; reversing it reproduces all prior bytes.
The separate `319315e` substitution advances those files to `c9c7036` before any
18946 build began. Verifier test manifest/lock hashes are refreshed separately.
The c9 compiler CLI/backend build and 57 identity tests now pass. A collector
expectation for Pliron's empty default feature was corrected separately without
rerunning those tests; both focused V12 tests then pass. The original collector
failure remains retained. V15 emission at Ferric `f393390` passes all 15 phases,
including 23 kernel tests, strict Clippy and 31 verifier source-policy tests.
Archive `1e21a6ff` retains the complete emission. Existing binaries and images
retain their actual producer revisions; this does not relabel the ae267 worker.

The preceding ae267 checkpoint used
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
remain intact. V15's first emission attempt stopped on a missing offline
dependency. The corrected standalone/build-std cache passes exact 163/198
package checks, but ae267 rejects the unchanged kernel with
`UnprovenBarrierConvergence` at bb33/op0. Archive `64c99339` retains that
failure. The first-pass guarded-read correction at `c119660`, remotely formatted
at `39be94d`, now passes all twelve R3 phases: 23 focused host tests, strict
Clippy, unchanged-ae267 emission, capture and independent ABI/resource checks.
Archive `c4f3be27` retains all 33,585 files and matching 40,774 metadata entries.
Image `a9da1d79` measures 96 explicit / 96 hidden offset / 352 total bytes,
Wave64, 70 SGPR / 22 VGPR, with zero spills/private/LDS/AGPR/dynamic stack.
Independent static review finds no blocker: RNE/preserved-denormal/IEEE mode,
convergent reductions and refined sqrt/division match the intended policy.
This is not a complete numerical proof or native result. The correction
changes no compiler, lock, arithmetic, ABI or second-pass volatile read.
Separately, exact analytical BF16 fixtures are integrated at `fcbba18`:
ten CPU tests and an eight-case self-test pass, archive `7b71c9c9`. These
fixtures do not bind an image or launch a GPU. Their completed 204 KiB stage
is removed. The completed format-only and failed R2 emission stages are also
removed (804 and 989,132 KiB), followed by the accepted R3 stage
(1,905,580 KiB).

The separate c9 emission produces image `68607fc1`, observation `66980589`
and handoff `0d9ee564`. Its 2,940-byte kernel body and hardware descriptor are
byte-identical to the ae267 image; differences are confined to nonallocated
compiler metadata. ABI and resource counts remain unchanged. Thirty CPU methods
for the unchanged fixtures and new native probe/wrapper pass on mi300x.
All eight real mi350 GPU0 cases then pass: uniform/signed inputs at rows
1/16/17/32, with 40 exact complete-buffer comparisons and 80 guards. The first
install-only tar-option failure is retained; the corrected install changes no
probe or image bytes. Native result `e6b2919c` and archive `c96ea879` retain the
run. Independent CPU replay `816d5c29` reconstructs all 357 protocol records and
expected payload hashes. These finite normal cases do not cover general FP32
equivalence, invalid/trap behavior, rounding boundaries or model parity.

Private source `9dfedb5` adds a separately admitted opt-in V15 adapter and a thin
same-image baseline/wave-v15 Qwen canary. Only the three pure width4096 target
normalization sites change; Q/K, draft, fused and default routes are unchanged.
Its nine focused tests, 35 source policies, 101 batching regressions, new and
legacy canary tests (67 each), and actual V15 image admission pass. A scratch
budget stop and subsequent strict Clippy failures remain retained. The completed
compiler parent is archived and removed, reclaiming 1,882,992 KiB. Correction
`ff313b4` borrows the V15 constructor argument then copies the same owned binding,
avoiding an oversized temporary profile without heap allocation; rustdoc and
test-only statement corrections end at `87fdcea`. That exact source passes all
20 host-gate phases: formatting, strict all-target Clippy, nine focused tests,
35 source policies, 101 batching regressions, new and legacy canary tests
(67 each), actual V15 image admission and release. Archive `ca6dd019` retains
the run; controller `21168d30` is bound to that source. Independent code review
finds no concrete blocker. The final bound model wrapper passes 23 CPU methods;
the separate prospective ABBA reducer passes 19. All four baseline/V15 Qwen
cases at eight and 128 outputs now pass exact token IDs/UTF8, retirement and
normal unforced close. All eight GPUs are idle before and after each case.
Archive `1e2ff5f1` retains the native run; independent CPU replay `dd2f37f1`
reproduces all four complete checked traces and wrappers. The first launcher's
SSH-stdin mistake is preserved: it launched only the first case; a fixed-array
continuation launched each remaining case once. No numerical retry occurred.

A separate fresh full128 ABBA cohort also passes independent replay `5ee062de`.
At n=2 per mode, baseline/V15 mean TTFT is 2.829254/2.475442 seconds, TPOT
166.854/113.231 ms and mean per-run output rate 5.415718/7.854668 tokens/s.
These descriptive changes are 12.51% lower TTFT, 32.14% lower TPOT and 45.03%
higher rate. Within-mode TPOT range/mean is 27.96%/40.92%, so stability is not
established. The boundary is controller host-prefill-start through final token
commit, excluding setup; it is not HTTP or GPU duration. Correctness timings
and all historical cohorts are excluded. Native archive `4af4c059` and replay
archive `1615f275` retain the comparison. No default or competitive win follows.

The separate V15 live entrypoint is integrated at its tested source `5b3e332`.
All 16 host phases pass: formatting, strict all-target Clippy, 40 live tests,
36 source policies, 35 legacy C1 tests, 29 old wave-live tests and release.
Nine unchanged tests are ignored across those live targets. Archive `2f0be996`
retains the gate; live controller `757d0579` remains a distinct producer from
the canary. The bound HTTP harness passes all 33 CPU tests; its first stale
test-fixture failure is preserved. Both context8192 HTTP correctness runs now
pass: two full128 requests per mode, exact token IDs/UTF8, slot0 generations1/2,
zero cached prefix, normal drain/close and all-eight-idle checks. Native archives
`118335a6`/`2300e7c6` retain the baseline/V15 runs, with 25 files and 27 metadata
entries each. Independent CPU replay `c8fde727` rechecks all four streams,
512 generated IDs, original numerical diagnostics and lifecycle/resource
records; archive `02588b51` retains all 97 replay files. No timing is reduced
from these qualification requests. Matched HTTP measurement remains next.

Completed V15 host scratch is removed after full retention, reclaiming
5,515,132 KiB including formatter stages. The four model run directories and
upload (12,964 KiB), four ABBA directories (6,484 KiB), model/ABBA replay
stages (3,948/7,820 KiB), both HTTP CPU stages (792 KiB) and the HTTP replay
stage (1,344 KiB) are removed. Both completed HTTP run/plan/authorization sets
(816 KiB) and their upload (2,648 KiB unshared allocation) are also removed.
The installed live binary retains its own hardlink and is not counted as freed.
The redundant live worktree is also removed. Models, required installed
controllers, workers and images remain intact.

Pages-only `030b9e1` is deployed; workflow `34749428862` succeeds and all seven
live assets match the tested artifact at 09:26:33 UTC. It adds the model and
descriptive ABBA results while retaining historical checkpoints and unchanged
HTTP timing data. Evidence, browser and every integer width from 320 to 1440 pass.
The first date-validator failure is preserved; the correction explicitly checks
both old/new dates before comparing all other historical fields unchanged.
Both archived result sets remain local. The completed remote parent
(309,656 KiB) and published sparse worktree (1,252 KiB) are removed.

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
| Kernels | V13 `256a6e4` passes host/emission checks at then-current `8efd4fd`. V14 passes eight native finite cases and four Qwen 8/128-output checks. V15 at `f393390` passes c9 emission, 23 kernel tests, strict Clippy, 31 verifier tests and static review. Image `68607fc1` passes eight native cases with 40 complete buffers and 80 guards; independent replay matches all 357 protocol records. Its opt-in adapter/canary `87fdcea` passes all 20 host phases and four exact Qwen 8/128-output cases. Separate n=2 ABBA replay observes 32.14% lower mean TPOT and 45.03% higher mean per-run rate, with substantial variability. Its live route now passes both context8192 HTTP correctness modes and independent replay. | A separate matched HTTP cohort; no stable gain, default promotion or competitive win. V13 native parity/runtime ordering, V14 same-compiler isolation, general FP32 refinement and detailed typed payloads remain open. |
| Runtime Optimization | Diagnostic binary `27c0404e` completed two exact 128-output requests and normal drain. Its original report validator failed on a synchronous-command allowlist omission. A separate corrected CPU replay passes all 17 tests and validates the full raw capture: 166,278 dispatches, 347,568 operational currentness checks and 395,246 completion polls. The original native receipt remains failed; no missing postflight is manufactured. | Currentness consumed 8.772 s of overlapping host time; ordered preparation accounts for 165,240 repeated checks. A bounded preparation optimization needs fault-injection tests and matched measurement. No GPU-duration or speedup inference, no fe2o3 change, and no new performance cohort. |
| Speculation | All eight repeated paged-draft cases pass. Paired K4 has two fresh native passes at frozen `ae355e5` and two separate passes at latest-build `9c2e98e`: each observes real `[4,4]` acceptance, both catch-ups and exact ten-token target prefix. | Broader native rejection coverage and speculative serving integration. No serving or speed qualification. |
| Verification | Standalone v13 integer model `ed2ceb5` passes pinned Verus: 18 verified queries, zero errors and all 16 required proof functions. Both the original typing failure and corrected pass are archived. Integrated proof source SHA `1111950b` is unchanged. | Actual kernel refinement, FP32 behavior, ABI, runtime ordering and native numerics are separate unproven obligations. All 33 M1 gates remain open. |
| Integration and Pages | Pages-only `030b9e1` is deployed with seven matching live assets and the model/ABBA checkpoint. Actual V14 controller `5b7fd71c` and ae267 worker `526cc6bf` retain their producer identities. V15 canary `21168d30` passes its host/model gates at `87fdcea`; separate live source `5b3e332` is integrated after all 16 host phases pass, with binary `757d0579`. Its HTTP harness passes 33 CPU tests, both two-request native qualifications and independent four-request replay. Completed scratch stages and redundant worktrees are removed after retention. No implementation push. | Separate matched HTTP measurement. Upstream b9 compiler migration needs its own validation; c9 comparison artifacts are not relabeled. All 33 M1 gates remain open. |

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
