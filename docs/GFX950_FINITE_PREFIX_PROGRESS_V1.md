# Finite Qwen3 Prefix Progress

This is an engineering checkpoint for [issue #42](https://github.com/harsh-nod/ferric/issues/42),
recorded on 2026-10-03. It is not a production admission, a sustained decode
benchmark, or a claim that the 700 tokens/s target has been reached. All issue #42 M0-M7
milestones remain open. The checkpoint is being published incrementally on an
engineering branch; it does not change the production execution path.

## Observed Results

| Check | Observed result | Boundary |
| --- | --- | --- |
| Corrected finite parent/worker CPU suite on `mi350-2` | 605 passed, 4 ignored; 34 owned commands | Includes 24 tests for the opt-in host-observation extension; no GPU execution |
| Historical KFD host-observation suite on `mi350-2` | 1,776 selected tests passed | Overlapping selections, including 15 new tests; separate source generation |
| Fresh Git-source worker build on `mi350-2` | 387 passed, 4 ignored; binary built from an empty target | Published Ferric/fe2o3 source pair; cached registry/toolchain, no parent or GPU run |
| Fresh-source V2 host-policy parent/worker cohort on `mi350-2` | 633 passed, 4 ignored; 12 binaries built and default-library check passed | Exact 13-file overlay; CPU checks alone do not qualify GPU execution |
| V2 host-policy baseline on `mi350` | 152 retained native tensor rows bitwise equal; six device audits and clean Close/reap | Newly built CPU633 binaries; no V2 autoregressive or sustained benchmark |
| V2 admission-cache arm on `mi350` | 152 native rows bitwise equal; repeated admissions fall to zero; six audits and clean Close/reap | No reliable overall latency improvement from the A/B/A diagnostic |
| V2 baseline repeat on `mi350` | 152 native rows bitwise equal; six audits and clean Close/reap; baseline operation counts restored | Baseline drift exceeds the initial aggregate cache delta; no confidence interval |
| V2 shared-currentness arm on `mi350` | 152 native rows bitwise equal; six audits and clean Close/reap; forward host total 61.458 seconds | Shorter than both baselines in this diagnostic, not a qualified speedup or isolated benchmark |
| Consolidated group-fence candidate on `mi350-2` | 475 selected CPU tests passed, 4 ignored; new worker built | 77 runtime plus 398 worker tests; parent remains separately qualified CPU633 |
| Consolidated group-fence worker on `mi350` | 152 native rows bitwise equal; six audits and clean Close/reap; forward host total 30.394 seconds | 18,720 duplicate full checks removed versus old shared worker; unisolated diagnostic, not sustained decode |
| Four teacher-forced forwards on `mi350` | 152 retained native tensor rows bitwise equal to the prior native route | All 36 layers, TP2, finite engineering images; not an independent full-model numerical bound |
| Four autoregressive forwards on `mi350` | 152 retained native tensor rows bitwise equal to the prior native route | Own-token history; no 2,048/256 workload or sustained throughput measurement |
| Six prefix-stage numerical cases | All 24 rank/profile rows satisfy the declared conditional norm/QKV bounds | Independent conditional arithmetic check of retained captures, not a new GPU run |
| Idempotent partial-move compiler candidate on `mi350-2` | 2,675 CPU tests passed, 25 ignored; fresh compiler built | Six new regressions executed; actual prefix-tile replay remains ignored |
| Matching finalizer tools on `mi350-2` | 190 default example tests passed, 15 ignored; three tools rebuilt | Same compiler source generation; actual-capture inert join remains ignored, no new HSACO |
| Checked reciprocal probe with new compiler on `mi350-2` | Previous storage gate cleared; canonical semantic capture emitted | Ranked projection rejects 1,027 blocks against the unchanged 1,024 limit; no HSACO or GPU run |
| Reduced-CFG reciprocal V3 on `mi350-2` | Exact census measures 1,013 blocks, 14 fewer; CFG-size gate cleared | Checked lowering now rejects an unsupported induction-latch form; no HSACO or GPU run |
| Canonical-counter reciprocal V4 on `mi350-2` | CPU arithmetic passes; fresh checked probe fails cumulative partial-move storage limit | No semantic capture or HSACO; prior V3 compiler successes do not transfer |
| Independent per-profile comparator on `mi350-2` | 20 CPU tests and eight historical rank rows passed | No paired bitwise prerequisite; unchanged bounds; not a new GPU run or new-image acceptance |
| Ordinary induction temporary compiler candidate on `mi350-2` | 2,683 CPU tests passed, 25 ignored; fresh backend and extractor built | Eight new regressions executed; actual prefix replay remains pending |
| Ordinary induction matching finalizer tools on `mi350-2` | 190 default tests passed, 15 ignored; three tools rebuilt | Exact new compiler generation; actual-capture inert join remains a separate gate |
| V3 checked lowering with ordinary-induction compiler on `mi350-2` | Natural zero exit in 1,771.339 seconds; KIR V11, gfx950 LLVM and inert handoff emitted | Interim leaf snapshot; actual replay and whole-probe postchecks pending, no new HSACO or GPU result |

Both four-forward runs completed their Close protocol, reaped their owned
processes, and passed their six surrounding device-state audits. The CPU suite
also completed with no forced cleanup. The V1 host-instrumented (CPU605)
generation has independently completed its teacher-forced run, described below. Its
autoregressive run has not yet been repeated with instrumentation.

The six numerical cases cover genuine positions 0 and 4, plus patterned
positions 15, 16, 2,047 and 2,048. They check first normalization, QKV projection,
head normalization, split-half RoPE and exact current-value append. The checks
are conditional on captured preceding-stage values and the declared floating
point premises: at most one FP32 ULP for the square-root sequence and correctly
rounded FP32 division. A universal proof of these instruction sequences is
still open. The candidate cap and numerical policy were not loosened to obtain
these results. The older attention/output-projection check remains separate.

A subsequent exact binary32 CPU diagnostic found six standalone reciprocal
counterexamples and 21 chained counterexamples over permitted synthetic raw
seeds. For example, with denominator `1 - 2^-24` and reciprocal seed `1`, the
correction can round a midpoint back to `1`, although the correctly rounded
reciprocal is the next FP32 value. These are not observations of the GPU's
actual seed selection or model failures. They show that the assumed division
rounding does not follow from the tested instruction model and seed envelope;
the prerequisite remains unresolved. No threshold was widened. The diagnostic
receipt is `9af99ee3b0b4247a0a7b504f8364fc6ad1dd6f8c825b37f05aedc330140722d8`.

The first fresh checked compiler probe for a separate exact-reciprocal candidate
passed metadata but failed semantic-SSA partial-move validation before producing
an HSACO. Its cumulative storage budget first rejected charge 2,097,153 against
the existing 2,097,152-word limit. This does not mean that one additional word
would suffice for the whole function. The limit was not raised. Source and
old-target postchecks passed; the failed owned process tree exited without
forced cleanup and was reaped. The [failure record](assets/finite-prefix-v228/reciprocal-probe-failure.json)
keeps this distinct from successful CPU arithmetic checks and from the existing
GPU images. A smaller candidate control-flow graph needs its own arithmetic and
checked compiler validation before any GPU claim.

The revised candidate now uses masked subtraction and boolean nearest-even
rounding within the same 24-step restoring algorithm. Bounded integer
operations use wrapping methods to avoid unnecessary checked-MIR branches;
only creation of the all-ones subtraction mask intentionally wraps. The
positive-normal domain, power-of-two path, two prefix wrappers, shared V3
arithmetic, kernel body and independent numerical references remain unchanged
relative to the first candidate.
No compiler storage cap or numerical tolerance was increased.

Its fresh [CPU validation](assets/finite-prefix-v228/reciprocal-cpu-branchless.json)
passed 11 ordinary arithmetic tests, six source contracts, the explicitly
selected exhaustive test over 8,388,608 significands in one binade, and all
1,007 independent Fraction nearest-neighbor vectors. The exhaustive test is
ignored by the ordinary invocation but was separately executed successfully.
All nine owned phases exited naturally, source/dependency postchecks passed,
and all 50 raw records plus both test binaries were rehashed locally. The
additional source contract pins macro-rule structure; it is not an independent
numerical reference.

The revised candidate has now undergone a fresh checked compiler probe on
`mi350-2`, but [lowering still fails](assets/finite-prefix-v228/reciprocal-probe-branchless-failure.json)
at the same first rejected cumulative partial-move storage charge:
2,097,153 against 2,097,152. The branchless rewrite did **not** unblock the
retained compiler. No HSACO was emitted and no GPU execution was attempted.
The probe driver passed 23 pure tests; its owned outer supervisor passed 14.
Before compilation, both joined the exact passing candidate CPU receipt,
seven Rust source bodies and 53 device-provider sources. Metadata passed,
Cargo then exited 101, and all source/tool/configuration/dependency/old-target
postchecks passed. The owned process tree exited naturally and was reaped.

The [next compiler candidate](../qualification/fe2o3-partial-move-idempotent-v1/README.md)
removes a redundant clear/reinsert when a local is already marked wholly moved.
Its [fresh CPU validation](../qualification/fe2o3-partial-move-idempotent-v1/cpu-result.json)
passed 1,487 Pliron tests and 1,188 compiler tests, with 25 ignored. All six new
exact-limit, cumulative-charge, moved-read and unchanged-join regressions ran;
the full default suites also exercised existing CFG, loop and replay coverage.
Actual prefix-tile replay is among the ignored tests and remains unqualified.
The fresh backend and extractor built, all ten commands exited naturally, and
source/dependency/tool/old-target checks passed. The compiler patch, regression
tests and precise source boundary are published separately from runtime code.

The first CPU controller attempt rejected libtest's passing `should panic`
annotation. Its failure was retained; the parser fix passed nine policy tests,
then both suites and the compiler build were rerun under fresh paths. The
completed run's 59 raw records, full copied compiler source and five selected
artifacts were retained and rehashed locally. This is not evidence that the
complete reciprocal function fits. The [matching finalizer build](../qualification/fe2o3-partial-move-idempotent-v1/finalizer-result.json)
subsequently passed 190 default tests with 15 ignored, rebuilt three tools,
and preserved all five qualified compiler products. Its six commands exited
naturally; source, dependency and prior-target checks passed. The 36 raw
records and three new tools were retained and rehashed locally. The actual
V6 capture join remains ignored.
Compiler limits, numerical tolerances and production admission remain unchanged.

The [fresh checked probe](../qualification/fe2o3-partial-move-idempotent-v1/checked-probe-result.json)
with those exact compiler/finalizer products cleared the partial-move storage
gate. It emitted a canonical V41 semantic capture, then rejected the unchanged
ranked CFG block limit. A matching-decoder [read-only census](../qualification/fe2o3-partial-move-idempotent-v1/semantic-census.json)
measured one kernel-root function containing 1,027 stored blocks, three above
the per-function limit of 1,024. All source/tool/dependency/prior-target
postchecks passed and the owned processes exited naturally and were reaped.
This is not ranked or target KIR acceptance, replay, HSACO emission or GPU
validation. The next source change targets avoidable reciprocal branches;
the numerical policy and compiler caps remain unchanged.

The [reduced-CFG V3 candidate](../qualification/exact-prefix-reciprocal-v3/README.md)
now removes the reciprocal domain and power-of-two branches while preserving
the 24-step recurrence and rejection sentinel. Its actual `mi350-2` CPU run
passed 12 ordinary tests, six source contracts, the explicit exhaustive
8,388,608-significand test and all 1,007 Fraction vectors. The unapplied source
patch and exact source pins are published with the result. All nine commands
exited naturally and source/dependency checks passed.

Its [fresh checked probe](../qualification/exact-prefix-reciprocal-v3/checked-probe-result.json)
has now cleared the CFG-size gate. The new exact-decoder census measures
1,013 stored blocks, 14 fewer than V2 and 11 below the unchanged cap. Lowering
then rejects an induction latch that does not copy field zero of a checked
result. The owned tree exits naturally and is reaped; all source/tool/dependency
and prior-target checks pass. There is still no new ranked/target KIR, handoff,
HSACO or GPU result. This block-count reduction is not a GPU speedup.

The [V4 source candidate](../qualification/exact-prefix-reciprocal-v4/README.md)
now changes only the bounded reciprocal counter to `step += 1_u32`, with its
matching macro contract. The actual V3 capture identifies both rejected
counter latches as plain temporary copies. V4 passed the same 12 ordinary
tests, six contracts, explicit exhaustive test and 1,007 Fraction vectors on
`mi350-2`. The source patch and exact CPU result are published separately from
runtime adoption. Its [fresh checked probe](../qualification/exact-prefix-reciprocal-v4/checked-probe-result.json)
has now failed semantic-SSA partial-move validation before emitting a capture:
the first rejected cumulative charge is 2,097,153 against the unchanged
2,097,152-word budget. The new checked counter changes the MIR; V3's successful
storage and CFG checks cannot be carried forward. All postchecks passed and
the owned tree exited naturally and was reaped. The latch fix remains
unqualified, with no new HSACO or GPU result.

The new [per-profile comparator](../qualification/prefix-profile-numerical-v1/README.md)
checks each implementation against the independent norm/QKV/head/RoPE,
attention and output references without first requiring V5/V6 bitwise parity.
All 20 CPU tests passed on `mi350-2`, including nonzero historical KV across a
page boundary and rejection of a finite historical-value corruption. Four
independent replays of retained position-15/16 profiles passed, covering eight
rank rows. All 16,384 attention BF16 words match the rounded reference; the
largest O error/bound ratio is below 0.011039. These are conditional checks of
older captures, not new GPU execution or validation of the reciprocal candidate.
The future native-child adapter still must authenticate ownership, captures,
untouched KV and actual-image arithmetic/ISA prerequisites.

The [next compiler candidate](../qualification/fe2o3-ordinary-induction-alias-v1/README.md)
recognizes only the adjacent ordinary-Add temporary seen in the V3 capture.
It retains exact type/definition custody, no-address-escape, positive-step,
unsigned no-overflow and replay checks, without granting assertion-elision
authority or changing caps. A fresh `mi350-2` run passed 1,487 Pliron and 1,196
compiler tests, with 25 unchanged ignored selections. All eight new regressions
ran; the extractor/backend built and all postchecks passed. The complete
5,780-file prior source generation was joined before applying the two-file
change. The patch, source pins and actual result are published; this CPU result
does not yet establish full checked lowering or a new GPU image.

The [matching finalizer tools](../qualification/fe2o3-ordinary-induction-alias-v1/finalizer-result.json)
have now built against that same generation. Their 190 default tests and 14
controller policy tests passed; 15 finalizer tests remain ignored, including
the actual-capture inert join. All six commands exited naturally, all five
compiler products stayed unchanged, and source/dependency/tool/configuration
and four old-target postchecks passed. The complete archive and selected
products were retained and rehashed locally.

Fresh V3 checked lowering has now [passed as a separate leaf](../qualification/fe2o3-ordinary-induction-alias-v1/checked-lowering-checkpoint.json)
in 1,771.339 seconds, naturally exiting zero before the unchanged 1,800-second
deadline. The actual outputs include semantic MIR, neutral/target KIR V11,
gfx950 LLVM and a 4,022,416-byte inert handoff. This clears the previous latch
refusal. This publication is an interim checkpoint: capture replay is running,
whole-probe source/dependency/prior-target postchecks remain pending, and no
HSACO, GPU validation or performance result follows from the leaf alone.

## Source Publication

The [source index](assets/finite-prefix-v227/source-index.json) binds the 289
imported files to the build-host source snapshot. All 355 Ferric paths in the
CPU cohort's source map were checked against the published tree. Cargo path
dependencies and literal source/fixture includes were audited separately;
the 167-file finite test census alone was not a complete source closure.
That index describes the original CPU605 generation. The separately tested
[V2 overlay](assets/finite-prefix-v228/host-policy-source.json) records later
changes without rewriting the original source evidence.

This includes the finite parent/worker, shared wire imports, consumed tokenizer
fixtures, and required device source/test dependencies. Only the four shared
wire files were added under the legacy v4 worker. Its Cargo manifest, lockfile,
main program and unrelated experiment additions were left unchanged.

The matching sibling runtime is now published at
[fe2o3 `6964f6129`](https://github.com/harsh-nod/fe2o3/commit/6964f6129c4c42d343117f61cdbcfe6166392535)
in both fe2o3 repositories. The bounded runtime export retained 765 files,
checked all 479 mapped runtime inputs, and imported exactly 84 changed paths
against its pinned base. The parent uses its own locked fe2o3 Git dependency;
the two dependency generations are not interchangeable.

A fresh Git-archive build paired Ferric `9eca2576` with that fe2o3 commit on
`mi350-2`. With an empty target directory, all 387 selected worker tests passed
and four were ignored; the worker binary also built. Cargo metadata verified
the exact ten local packages and 29 registry packages, and all 6,921 extracted
source files remained unchanged. The registry cache and toolchain were reused.
The [fresh-build record](assets/finite-prefix-v228/clean-worker-build.json)
and [worker instructions](../adapters/tp-peer-finite-engineering-worker-v1/README.md)
record the exact pair. This does not reproduce the parent, compiler/HSACO
pipeline, or GPU run from clean source, and is not a workspace-wide test pass
or a newly qualified legacy-worker build. Full workspace formatting, Clippy
and regression gates remain required before merging to production.

## Measured Host Overhead

The instrumented teacher-forced run completed on `mi350`: all 152 native
tensor rows matched the prior native baseline bit-for-bit, all three pre-run
and three post-run device audits passed, and Close/EOF/owned process reap
completed without forced cleanup. Its receipt is
`5825e7859db0066b13f3f844d6a103dec5d0d6ec428973426a2d6d4529bc6304`.
The selected parent and worker are the corrected CPU605 binaries, not the
earlier uninstrumented binaries. All 57 pinned case outputs were independently
rehashed after retention on the local machine.

The [machine-readable host summary](assets/finite-prefix-v227/host-observation.json)
retains nanosecond values and counter names. These are **inclusive, nested host
scopes**, not calibrated GPU durations or additive contributions. Currentness
checks revalidate device, topology, allocation and queue state. Do not add
rank times, subtract them from wall time, infer overlap, or turn four
teacher-forced forwards into a target-workload tokens/s result.

| Position | Forward Wall (ms) | Full Currentness R0 (ms) | Full Currentness R1 (ms) | Admission R0 (ms) | Admission R1 (ms) | Full Checks R0 / R1 |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| 0 | 18,240.289 | 8,512.988 | 8,477.279 | 14.444 | 10.846 | 5,068 / 5,048 |
| 1 | 18,252.250 | 8,523.919 | 8,486.701 | 14.496 | 10.812 | 5,068 / 5,048 |
| 2 | 20,186.874 | 9,484.764 | 9,451.487 | 14.461 | 10.819 | 5,644 / 5,624 |
| 3 | 20,203.428 | 9,499.867 | 9,463.653 | 14.412 | 10.813 | 5,644 / 5,624 |

The setup snapshot interval was 167.644 seconds and Close took 26.703 seconds.
The parent diagnostic took 413.571 seconds including its setup, forwards,
readbacks and teardown; the surrounding controller took 430.597 seconds.
These boundaries differ and must not be mixed into a performance comparison.
This is one diagnostic run with all performance opt-ins off, not an ablation
or a statistically qualified latency measurement.

The observations make repeated currentness checks the first optimization
candidate to investigate. The controlled comparisons separately
enable immutable kernel-admission caching and sharing a fresh full-currentness
observation within a group fence. Mutable allocation, pointer, ABI, geometry
and queue checks must remain. Round publication already shares a fresh
observation in the baseline, so that existing behavior cannot be claimed as a
new optimization. No speedup is claimed until the separate variants pass their
own numerical, ownership and measurement checks.

The explicit V2 policy implementation has now passed a fresh-source CPU
cohort: 398 worker/shared-wire tests, 224 disjoint parent-library tests and
11 parent CLI tests, with four ignored. All 12 binaries built, the default
parent library check passed, and all 6,928 post-overlay source files stayed unchanged.
All 213 retained raw records and 12 binary bodies were rehashed after transfer.
The [policy guide](GFX950_HOST_POLICY_V2.md) documents the checks each arm
retains and the [CPU record](assets/finite-prefix-v228/host-policy-cpu.json)
identifies this new generation. Its separate V2 baseline has now completed on
`mi350`, with all 152 native tensor rows bitwise equal, six passing device audits
and clean Close/reap. The [GPU record](assets/finite-prefix-v228/host-policy-baseline.json)
retains the new identities and host counters. All 57 final output files were
independently rehashed locally. Forward host durations were 18.266, 18.304,
20.174 and 20.171 seconds. These are not GPU durations or qualified throughput.
The separate admission-cache arm also passed native parity and lifecycle checks.
Per-forward repeated admissions fell from 148 / 145 to zero, while initial
admission, dispatch, full-currentness and I/O counts remained unchanged.
Its four forward host durations sum to 76.933 seconds versus baseline's
76.916 seconds: no overall improvement is established. The
[cache record](assets/finite-prefix-v228/host-policy-cache.json) and
[comparison table](GFX950_HOST_POLICY_V2.md#admission-cache-observation) retain
this observation without a speedup claim. The
[repeated baseline](assets/finite-prefix-v228/host-policy-baseline-repeat.json)
also passed parity, six audits and clean Close/reap; all 57 final outputs were
rehashed locally. Its forward host durations sum to 77.471 seconds, so the
cache total lies between the two baseline totals. The observed baseline drift
exceeds the initial aggregate cache delta. These three runs establish no
reliable end-to-end cache improvement or confidence interval.

The separate [shared-currentness arm](assets/finite-prefix-v228/host-policy-shared.json)
has now passed the same parity, six audits and shutdown checks; its 57 final
outputs were rehashed locally. The four forward host durations total 61.458
seconds versus 76.916 and 77.471 seconds for the baselines. New shared-group
checks replace repeated per-rank full checks; admissions, dispatches and I/O
counts remain unchanged. The [plot and counter table](GFX950_HOST_POLICY_V2.md#shared-currentness-observation)
show the observed effect and limits. Another compiler job was observed during
C preflight on the shared host. This is not an isolated or statistically
qualified performance result, GPU timing, kernel overlap or sustained decode.
The [consolidation candidate](GFX950_HOST_POLICY_V2.md#consolidation-candidate)
has since passed 475 selected CPU tests with four ignored, and its new worker
built from fresh sources. It removes duplicate rank currentness checks after
a fresh group fence while retaining queue and poison validation. Its runtime
source is published at
[fe2o3 `725ecc6a5`](https://github.com/harsh-nod/fe2o3/commit/725ecc6a500ff49e7dfaefb38b027f6bcc223ebf)
in both repositories. Its own GPU comparison and the V2 autoregressive diagnostic
remain pending; no earlier GPU result is transferred to the new binary.

## Changes And Retained Failures

The finite route uses an explicit bounded parent/worker protocol, typed
generation and bank checks, and ownership-bound shutdown. Host observation is
opt-in and keeps operational optimizations off. Its nested counters measure
host activity, not GPU elapsed time, kernel overlap, or tokens/s.

A parent compile failure exposed a `u32` image-size versus `u64` request-size
comparison. The correction widens the image size with `u64::from`, and adds
regressions rejecting oversized requests with identical low 32 bits. The
corrected 605-test run is a new result; the failed run is retained.

Portable-deployment tests also exposed aliasing in a cached test fixture.
The test-only correction deep-copies the fixture and adds a mutation-isolation
regression: all 23 tests passed. The production reader and exporter were
unchanged. The earlier 22-test run remains a failure, not a passing result.

## Evidence Identities

These are SHA-256 identities of retained receipts, not links to publicly
downloadable artifacts. Large logs, captures, model weights, compiler caches
and runtime binaries are deliberately excluded from Git. A receipt digest
alone is not a self-contained reproducer or proof.

| Retained receipt | SHA-256 |
| --- | --- |
| Corrected CPU605 completion | `f9b4da95d00cb62a3eadb33d86ee942d0bdf0ecb5c559650161846f4dcd081e8` |
| Fresh-source V2 CPU633 completion | `1fc4d17534161e6e7f96e6d0eba0a2455227ba2166623823f6a74f845b93deae` |
| V2 baseline GPU completion | `8d46f69e481ea906733765dffee163a0a6ba33bf7816316741e867880a82ea5f` |
| V2 admission-cache GPU completion | `0952f130801dcf3c958f5dff8b1228a401f436640ee7591465947edcc3881cac` |
| V2 repeated-baseline GPU completion | `4f5cbd384e136d5c5a05cf0a9d87733b249ee85fec5cd421d3164df87af4110d` |
| V2 shared-currentness GPU completion | `6aad2f46f234a98ac9f792a6108e98e6bbb6269ad81a6913aa1524f8972cd097` |
| Consolidated group-fence CPU475 completion | `0231c40bf9a5ff285e041f29b2c958e26972e73f6a4485072e153a0bd178171b` |
| Consolidated group-fence GPU completion | `ebecff7a6e459cbb6fa8b4c750bc9c524adb417405e2827608175ba0a2c66a5f` |
| Exact-reciprocal checked probe failure | `3d3fd530e171a6e2977d995c78d5665d9e5a728e173eb21b445891148ba61e0d` |
| Branchless reciprocal CPU completion | `d16dd989342d6092445d6fc3d046184ddf160527b1681d713e9b1291f6b106a4` |
| Corrected CPU605 owner | `dde383021440a721bd2c41bff614ea1156a3b36c6fa48cd2ad27c95337af7cb2` |
| Historical KFD completion | `e6cdd55c6a2dda4dafdd25a4c8d5e3670312c912cdbc1f20258a6e93db48e603` |
| Teacher-forced four-forward completion | `56f72c0c8c76e7c2c3f00ca767d5a7b53c66ba4b2c17589059565c2c2532fb8b` |
| Autoregressive four-forward completion | `0553862c7e65b74020028421b3bb668ff6eaf0080ba4a1bc405587707a403d77` |
| Numerical case: genuine position 0 | `a1990e1f8153aa8df23a8db175e293bff7d17b0f3c7fc181b223158df9a41586` |
| Numerical case: genuine position 4 | `81d9b3b17d7763b5c9d40db023c6ba1cf0eb61c61d429d1f1214fbef55475adb` |
| Numerical case: patterned position 15 | `fa7795a3ff7968c212600e2ba30dca7c39b4ccf29ac2b075d42a18377b66fa9e` |
| Numerical case: patterned position 16 | `39ae8374a10ea306e0f1426c87e9c3184f04de250e9f82695973158ad941e1e4` |
| Numerical case: patterned position 2,047 | `7cb4e5ee27e423b96f2eae3fbb2518e14da4f546066cdb08bf5d251f2df32fca` |
| Numerical case: patterned position 2,048 | `676d7e878105972365b15c9aba5f5361825a7761e04f761178487271fae491de` |
| Portable test-only correction, 23 tests | `0481e077defafed91c2272984b3d5d1da5d97cb9da9431ba307760000698de8a` |

## Next Gates

1. Extend the freshly built parent/worker source pair to reproducible
   compiler/HSACO builds. Use the tested idempotent compiler and newly rebuilt
   matching finalizer tools to retry checked reciprocal lowering;
   the full pipeline is not yet reproduced.
2. Follow the consolidated worker's completed teacher-forced GPU comparison
   with its autoregressive check and repeated shared-currentness timing on
   `mi350`, preserving the exact numerical checks and retained-device review.
3. Close independent numerical obligations and validate the longer resident
   request, multiple requests, KV lifetime and cleanup boundaries.
4. Qualify the full target workload: single-request Qwen3-8B, BF16,
   target-only decoding, 2,048 prompt tokens and 256 generated tokens. Report
   post-first-token throughput as `255 / (last_delivery - first_delivery)`.
5. Run equal-work baselines and paired optimization ablations. Host-inclusive
   diagnostics are not GPU overlap graphs or a qualified vLLM comparison.

The selected TP2 experiments do not satisfy the issue's original single-GPU
matrix. No M0-M7 milestone is closed by this checkpoint. Assurance properties
without closed, identity-bound proof remain `Contracted` or `Unsupported`, as
required by [the proof policy](PROOF_DEVELOPMENT.md).
