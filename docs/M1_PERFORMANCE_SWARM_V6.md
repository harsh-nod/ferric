# Performance Swarm V6

Started 2026-09-20 UTC. User-authorized implementation of the plan to close
Ferric's matched vLLM performance gap. This is an engineering campaign, not
M1 qualification or a claim that Ferric is faster.

## Baseline And Acceptance

The accepted V5 pair below is the historical reference. Two fresh R3 pairs
are recorded separately below; the immutable three-pair series is incomplete
after the third Ferric start failed its foreign-KFD check.
Qwen3-8B, TP1/C1, 128 input/128 output tokens, context 8192, BF16
decoder with FP32 output head, greedy fixed-length output, no speculation or
prefix caching:

| Engine | Mean TTFT (ms) | Mean TPOT (ms) | Measured-span output tokens/s |
| --- | ---: | ---: | ---: |
| Ferric V22 packed | 2329.922 | 83.598 | 9.885898 |
| vLLM 0.28.0 | 18.636 | 4.252 | 228.904359 |

TTFT ends at first nonempty SSE text; TPOT is the first-to-last text span
divided by 127, not individual token interarrival. Rate includes gaps/drain.
One start per engine, fixed vLLM-first order, ten warmups and thirty measured
requests do not establish stable tails, confidence intervals or a general
framework comparison. The FP32-head profile is not stock-default vLLM.
See [the retained V5 contract](M1_MATCHED_OPTIMIZATION_V5.md).

The first optimization target is at least 10% lower TTFT and TPOT than a
fresh matched baseline, without correctness regression. Existing measurements
would imply approximately 16.8 ms TTFT and 3.83 ms TPOT; these are targets,
not predictions. Production BF16-head and wider workload comparisons remain
separate. Public claims continue to require [performance policy](PERFORMANCE.md),
including held-out workloads, tuned baselines and confidence bounds.

### Ordered64 Repeated Starts

On September 22, the wholly fresh ordered64 campaign completes all three
predeclared pairs in AB/BA/AB order. Every start passes exact-output diagnostics,
timing replay and clean owned teardown. Each engine has ten warmups, thirty
measured requests and two untimed diagnostics per start; settings and timing
definitions above are unchanged. These runs do not reuse the historical R3
samples or replace its rejected third pair.

| Pair | Engine | Mean TTFT (ms) | Mean TPOT (ms) | Measured-span tokens/s |
| --- | --- | ---: | ---: | ---: |
| 1 | Ferric ordered64 | 811.949979 | 63.833496 | 14.350312 |
| 1 | vLLM 0.28.0 | 19.665213 | 4.227510 | 229.693751 |
| 2 | Ferric ordered64 | 992.805919 | 79.534571 | 11.537057 |
| 2 | vLLM 0.28.0 | 19.089385 | 4.380342 | 222.283045 |
| 3 | Ferric ordered64 | 844.273265 | 67.801331 | 13.536618 |
| 3 | vLLM 0.28.0 | 19.916154 | 4.393673 | 221.301476 |

Ferric/vLLM TPOT ratios are 15.099549x, 18.157160x and 15.431582x. Their mean
is 16.229431x and median 15.431582x; these describe paired-start means, not
pooled request tails or confidence bounds. The series is accepted as
`repeated-pair-screening`, not competitive or performance-qualified. Aggregate
SHA256 is `579c4abeda3f56ca2a77ec1e1f63d37c0961cec92381f90da6a368d79ab3a3ec`;
pair summaries are `ab1586e889ef933682a233af7bc9d2488da426ec89f34c1f9b826bdfdde16f96`,
`1eabed4025f6fef149a7fcbac9f7cac7d95ae1a509c7317a4ce9a308610d1b08`, and
`e39439549ee998315f133c6a549ce51a41dabd8aacb89d345e5ef21f48f453fe`.

Joining controller events to measured client IDs places the gap inside Ferric:
pair1 client/controller TPOT is 63.833496/63.834527 ms and pair2 is
79.534571/79.536857 ms. This does not separate GPU work from host preparation,
dispatch, polling or output backpressure. The next bounded diagnostic reuses
host spans and the existing worker duration; worker duration is explicitly not
GPU time. No matched profile or default is broadened to enable diagnostics.

### Prior Composed Pairs

HTTP R3 pair 1 passes both engines' gates and paired replay, in Ferric-first
order. Each engine has ten warmups, thirty measured requests and two untimed
diagnostics, with all requests successful. The matched profile and timing
definitions above are unchanged.

| Engine | Mean TTFT (ms) | Mean TPOT (ms) | Measured-span output tokens/s |
| --- | ---: | ---: | ---: |
| Ferric composed | 810.927514 | 69.587059 | 13.265064623 |
| vLLM 0.28.0 | 20.148123 | 4.367310 | 222.524242491 |

Ferric/vLLM latency ratios are 40.2483x TTFT and 15.9336x TPOT. This is one
accepted pair, not an accepted repeated-start series or a competitive win.
The summary and its remote/local SHA256 match at
`d3fa11cce07e0181e155498b72191a47e88a3240abf8cd840dbd5c075c5e788e`;
paired replay exits zero with empty stderr.

Pair 2 also passes both engines' gates and paired replay, in vLLM-first order,
with the same complete request scope and unchanged profile:

| Engine | Mean TTFT (ms) | Mean TPOT (ms) | Measured-span output tokens/s |
| --- | ---: | ---: | ---: |
| Ferric composed | 811.403565 | 69.185620 | 13.335104493 |
| vLLM 0.28.0 | 18.835039 | 4.349581 | 223.905212947 |

Its retained summary SHA256 is
`e74214f1923eac3bbda26ae13acdaae0d0a2d1011ec3aa3cfce52338d5da4c97`.
Pair 3's Ferric start fails on foreign KFD activity: timing is not admitted.
Cleanup completes, but the shared client requires forced cleanup. The third
vendor start does not run. The foreign process subsequently exits without
signals from the integration lead. This immutable AB/BA/AB series is
incomplete and cannot support a series win. The immutable aggregator exits
one and records `rejected-series`: the first two pairs pass replay, while
the third lacks a pair summary. Its retained output SHA256 is
`962bce221f76fee67bb40a71bd9e0302095eba742f60885ef0b6cd1fc2e03159`;
the failed Ferric receipt is
`c8a69e6f21cc9fe08d161617ef4a100630bc32674270d6634bb49bf60e4bbe11`.
The two successful pairs and the failed third start all
remain evidence, with no replacement, splicing or historical V22 samples.
Repeating unchanged software is deferred in favor of codegen and kernel work.

## Ownership And Progress

| Owner | Bounded Deliverable | State | Next Gate |
| --- | --- | --- | --- |
| Compiler/runtime worker | Current compiler capture and packed-u32 GEMV CPU qualification | Actual ProducerV6 retained; Q-only CPU/emission/ISA and all 17 native finite cells pass | Preserve actual 13e provenance; numerical-trap classification still unsupported |
| Kernel worker | Packed-load decode and prefill candidates | R4 has no timing gain; Q-only packed images emitted; separate 390-pinned down-FP32 candidate passes all nine CPU gates, 30 Rust tests and strict Clippy | Distinct emission/native gates; do not promote from CPU results |
| Measurement/verification worker | Matched measurement and independent producer review | All three fresh ordered64 HTTP pairs pass; Ferric remains 15.1..18.2x slower on TPOT; host diagnostic source independently reviewed | Remote validation of the distinct bounded host diagnostic; 17 new tests authored, unrun |
| Integration lead | Resource admission, integration and published scoreboard | Ordered64 opt-in integrated; static-only site f6bcf9ae deployed; completed GPU stages and retirement owner removed after complete local custody | Validate host diagnostic and separate down-FP32 emission/native path; no default promotion |

States advance independently: source complete, CPU tested, emitted/built,
native correct, measured, accepted. A pure-test pass cannot advance a native
gate. No candidate gain is recorded until the corresponding measurement
exists. Separate V22 and V25 gains must not be multiplied or treated as a
composed result. Existing defaults remain unchanged during screening.

### Packed Q Finite Screen

The fresh MI350 Q-only packed-u32 campaign completes at 02:09:25 UTC on
September 22. All 17 fresh workers pass with clean unsignaled teardown: one
worker tests all 65,536 activation-pack bit patterns and sixteen workers cover
eight finite cases in alternating baseline/candidate order. All guards, tails,
immutable inputs, GPU scratch continuity and independent oracle checks pass.
There are 6,592 dispatch packets across the complete campaign.

| Case | Baseline us/projection | Pack + candidate us/projection | Lower controller wall |
| --- | ---: | ---: | ---: |
| Mixed | 81.422 | 70.115 | 13.887% |
| Cancellation | 79.506 | 69.277 | 12.866% |
| Signed zero | 82.737 | 69.331 | 16.203% |
| Subnormal | 79.270 | 69.357 | 12.505% |
| Underflow | 80.870 | 69.594 | 13.943% |
| Last element | 81.610 | 69.501 | 14.837% |
| One-element tail | 80.779 | 69.928 | 13.432% |
| Maximum tail | 82.442 | 69.863 | 15.258% |

Each cell measures 256 logical projections after 16 warmups. Baseline uses
256 packets in 16 ordered groups; candidate uses 512 packets in 32 groups and
includes every activation pack. These are summed controller-group walls for a
synthetic cache-hot N=K4096 Q projection, not GPU timestamps, model TPOT, stable
confidence bounds or a vendor comparison. One-time 32 MiB weight packing/upload
is excluded. Worker walls, polling and other nested counters overlap.

Report SHA256 is
`cea4120ed4931ebba10cc73832c651ee8693166903d4cd83c772eb24ba74c3d3`;
outer supervisor SHA256 is
`a5191ac533301fb7f947a7445ad3236f46a8fad8ef69319441fa373f18c6d56d`.
The actual producer/SDK remains13e and worker remains5e2/dd6. Full numerical
trap classification is unsupported, so `numeric_trap_parity_passed`,
`performance_qualified` and `model_inference` remain false. No engine/default
promotion or extrapolated end-to-end gain follows from this screen.

Complete local byte custody covers 184 packed-native, 110 ordered64-native and
530 matched-HTTP files. All three completed GPU stages and their separate
retirement owner are removed after fresh unused-path checks, reclaiming
59,736,064 allocated bytes without signaling any process or touching the shared
model. CPU targets still needed for subsequent source/emission work remain
under the unchanged owner-stage cap.

Current execution at 21:12 UTC: actual ProducerV6 is retained, ordered64's
eight recipe fixtures and fifteen native-harness fixtures pass, and its actual
source/lock preparation passes. Compilation has started in a new private target.
Two fully archived unused vendor trees reclaim 509,243,392 allocated bytes
without relaxing the resource limits. A fresh MI350 native owner
`/tmp/ferric-opt-v5-v25.v1fakSWO` is being staged with immutable inputs only.
At 21:27:30 UTC all five default/plain/ordered/model/union feature-profile
checks pass at a001: seven of eighteen CPU gates are complete. Strict ordered
Clippy then fails cleanly at 21:30:23 on the newly added fourth execution-state
boolean. The retained failure is not waived: a private packet-width enum fix
and fresh R2 campaign are being prepared, reusing only the owned compilation
cache, not R1 gate results. No ordered64 controller, GPU correctness result or timing gain
exists yet. The source-reviewed HTTP successor has 47 unrun fixtures and is
staged for CPU testing; actual native improvement remains a prerequisite to
the integration lead launching a fresh matched vendor comparison.

Fresh R2's 15 native-harness fixtures pass at 21:41:33. The eight CPU recipe
fixtures and actual preparation pass at 22:08:16 and 22:08:56; both strict
ordered and combined-feature Clippy pass by 22:12:10. The prepared source
inventory is `7604ad3eeb2b89c5c75754bec38d1d766e00bc6d9adb082c20c3789365178cb4`.
The actual resolved lock and dependency delta are unchanged. All five fresh
feature-profile checks pass by 22:19:50, completing nine of eighteen R2 CPU
gates. Test-binary compilation is running; executable suites and controller
build remain required. The independently reviewed HTTP R2 is a pin-only
successor with 47 unrun fixtures, not new benchmark evidence.

At 22:22:29 UTC, six unused historical prepared-source trees are retired after
complete local archive and metadata verification, reclaiming 302,137,344
allocated bytes. Current sources, recipes, receipts and selected products remain
unchanged. Historical replay of the removed trees requires restoration from
the retained archive. Resource limits are unchanged.

Ordered test compilation a001 is interrupted at 22:36:20 by an uncaught
`ProcessLookupError` in the supervisor, not a compiler diagnostic. Status125,
TERM and `cleanup_ok=false` remain retained without reinterpretation; the
remote/local result SHA256 is
`52a9b830cc7206c8773540b6461029abaddb5d4caf75d55a7166ee8fc76751c4`.
A fresh owned group/session/source/target process census returns no remaining
processes. Fresh a002 then resumes with cache reuse only and the unchanged
guard. Source review identifies the per-PID `/proc` read's missing ESRCH catch
as the likely race, not a traceback-proven cause. No test pass is inferred.

Compile-ordered/a002 passes cleanly at 22:43:21. At 22:47:27 the ordered
suite passes 433 library tests (11 ignored, one existing resource exclusion),
65 CLI tests (7 ignored), 76 peer tests (5 ignored), plus the focused engine4
and wire6 cohorts. All four actual-image metadata fixtures pass at 22:48:04.
These complete twelve R2 gates. Source-policy then fails cleanly at 22:48:43:
45 pass, but the exact optional-dependency roster omits the new
`fe2o3-kfd-ordered64` alias. The dedicated ordered64 policy test passes.
The failure receipt is retained with matching remote/local SHA256
`c9912b091762d2fee8b3b46ebdf061e15d18e5775ea9998f75ae83ad0c1ffdf6`.
A test-only one-line roster correction and fresh R3 handoff are being prepared;
no production-code change, assertion waiver, controller or native gain is
claimed. Remaining R2 feature tests serve additional defect discovery and
cache reuse only; they will not become inherited R3 passes.
R3 subsequently passes eight recipe fixtures at 22:59:27, fifteen native-harness
fixtures at 23:02:52 and preparation at 23:06:40. Its actual prepared census is
`39f4dcb215fc37069bfa5f24e6c2b659062b825add0eb6d68e68c83b02696c75`;
the resolved lock remains `a773a4dc...`, preserving 325 existing packages and
adding exactly nine optional runtime packages plus one adapter edge. All 1,301
R2 source files and eighteen selected ELF/rlib artifacts (96,569,138 bytes) are
locally retained and hash-verified before cache reuse; complete R2 review and
outer-result copies are retained too. No R2 pass is inherited by R3.
All seventeen R3 pre-build gates pass by 23:30:55, including all 46 policy
tests and the actual-image fixtures. The final build/a001 is interrupted at
23:41:25 by supervisor `ProcessLookupError`, not a compiler diagnostic. Its
status125, TERM and cleanup_ok=false remain retained in receipt
`e70c1ac17c1ae4724b6dced44c82fa5b07c8627b544faa6106f20ac0cbec9cb2`.
A fresh UID1046 census finds no matching live group/session/source/target
processes; the failed receipt is not retrospectively rewritten. The precise
exception site remains unproven without a traceback.

The newly user-approved four-core/four-job, 8-GiB-RSS profile is separate from
the frozen R3 helpers. It retains nice19, the shared build lock, 12-GiB stage,
512-MiB reserve, 1200-second timeout and all other cleanup/resource limits.
Its 22 fixtures pass under the original remote guard at 23:43:51. A build-only
continuation is being reviewed to preserve all seventeen actual original R3
checks while explicitly binding the new controller build profile. No old pass
is relabeled, no source change is implied, and no native gain exists yet.

The first actual new-profile invocation passes all seventeen packed-CPU recipe
fixtures at 23:47:44. Its outer receipt records CPUs0..3, jobs4, nice19, the exact
8-GiB RSS bound and clean unsignaled completion; remote/local SHA256 is
`7d6a310ddef21b7f0cf3fe5864d76b24c11b4f1cfebb04273816ef2192b7c912`.
The separate 22-fixture profile receipt also matches remote/local SHA256
`45b61703e9d4905b327be28bd873ef71df587469adb54dffe17fcb4a89a8d936`.
No packed Rust compilation, emission or native result is inferred from these
fixture passes. Packed compilation remains serialized behind the ordered64
controller build and reviewed target-cache retirement to preserve headroom.
The separately rebound emission recipe then passes all 21 fixtures at
23:50:21 under the same new profile, including the post-compiler environment
override and explicit CPUV3 boundary. Its retained outer receipt matches
remote/local SHA256
`d2d5fec37f7860e47e39a51183948e685608ca0242aff0ed2d484fa669356bae`.
The actual compiler producer remains unchanged. The emission recipe claims
four-core affinity, not an observed four-job count inside the producer's
isolated extraction subprocess. No kernel image has been emitted by this
successor yet.

The ordered64 four-core controller continuation runs its six focused fixtures
successfully, then build/a001 stops at 23:58:26 when the unchanged stage reserve
is reached. Peak observed RSS is 2,399,469,568 bytes. Status125, TERM and
cleanup_ok=false remain in the complete retained result
`349915dd5e89e94bda2ba151ae573868d7d1f2b09c31134d6a1bfc54cb9c6d92`,
whose remote/local hashes match. A fresh owned-process census is empty; no
compiler error, controller pass or native gain is inferred. Two separately
reviewed, fully locally retained historical compiler-archive duplicates are
subsequently removed, reclaiming 33,882,112 allocated bytes without changing
the build guard, cap or reserve. Further exact obsolete-source cleanup is
being reviewed before another build attempt.
That cleanup completes at 00:14:38 on September 22 UTC: the fully retained R2
source and three freshly archived historical source/tool trees are removed,
along with the two duplicate archives. Total reclaimed allocation is
347,885,568 bytes; every cleanup completes cleanly and current inputs/cache
are preserved. The three-root archive verifies all 13,920 files and metadata.

Four-core build/a002 then passes at 00:15:29 with clean unsignaled completion,
six continuation fixtures and all seventeen original R3 records/source/lock
reopened unchanged. Its warm continuation takes 24.507 seconds, not a matched
cold-build speed measurement; peak observed RSS is 902,082,560 bytes.
Actual controller SHA256 is
`142cc29781fda56d5e73f48588d9e37c43e50de2ff7c69c3f09581c6e180b3a8`
(10,289,080 bytes); retained outer receipt SHA256 is
`65bc50bd798a4ebeaa1d561439b7669ccdc38597359bda06680c0e3dc091d601`.
All remote/local hashes match. The fresh fifteen-fixture mixed-profile native
harness passes at 00:18:10 with outer receipt
`094162f643dbd65536631d16cf769280b3054561279bc624b1da7aaa2ea209a1`.
The BuildV2 collector passes cleanly at 00:19:19; its exact stdout is
`dd623168690c64aad555e770c9eb4362b7d3ededc09eb2a38d6176783e61b748`
and its clean outer is
`e5e647354d1b148fa8122a601d02e193a9cec6289887559aa41ad5edcc5d69a7`.
The collector preserves seventeen original one-core gates and the separate
new four-core build. All three remote/local hashes agree.

The reviewed integration patch `90f805e3...` is applied to exactly17 Ferric
paths. All proposed hashes match the actual R3 source census/lock; the complete
before/after file inventory verifies that unlisted paths are untouched.
The two new test files are present, lock mode0600 is preserved and
`git diff --check` passes. No local builds, tests or whole-worktree remote
qualification are claimed. The feature stays opt-in. Actual plan `c70fdf18...`
is launched through the unchanged native supervisor in fresh stage
`/tmp/ferric-opt-v5-v25.v1fakSWO` after idle admission. Native parity and timing
are pending, not inferred from reduced source-derived packet-group counts.

The first target-retirement export stops cleanly at 00:26:27 with
`BrokenPipeError`: the root-owned local receiver used an invalid dd option.
No cache file is deleted, and its failed outer/intent are retained. A reviewed
R2 changes only output namespaces and uses a corrected exclusive receiver;
the cap/reserve and full archive verification remain mandatory.
R2 export passes at 00:31:36. Its 58,598,750-byte archive is verified locally
against every member/hash/metadata entry: 2,214 files and 444 directories.
Retirement passes at 00:34:41 and reclaims 4,525,428,736 allocated bytes while
keeping all 21 selected file paths and both controller copies. Its actual
BuildV2 recollection passes again after deletion; current source, recipes and
receipts remain intact. The retained retirement JSON is `3e75c086...` and its
clean outer is `2bd2d938...`, with remote/local hash agreement. The three exact
unused archive FIFOs are subsequently removed after owner/type/no-use checks.
Packed-u32's fresh four-core preparation then passes at 00:35:43; Rust
compilation passes at 00:39:20 in its separate private target. Cargo reports
2m51s; the guarded phase takes 175.546 seconds with peak observed RSS
3,247,116,288 bytes, below the approved 8-GiB limit. The 13/3/7/3 Rust test
cohorts and strict all-target Clippy pass next; all nine gates complete with
clean retention at 00:41:42. No cold-build comparison or emitted-image pass
is inferred.

The packed-u32 four-core native successor passes all36 remote CPU fixtures at
00:29:13, including the six new exact resource/consumer cases. Its clean outer
receipt is `ed604bde...`, verified against the complete local copy. These are
synthetic transport/admission tests only; Rust compilation, capture, ISA and
real numerical/timing evidence remain outstanding. Numerical-trap acceptance
is still unsupported and unconditionally fail-closed.

### Ordered64 Native Screen

The actual four-start campaign completes at 00:37:05 on September 22, with
all 16 requests, 2,048 output IDs and decoded bytes matching. Outer status0,
clean reaping/cleanup, no outer signals/errors and no remaining KFD owners;
per-arm owned TERM/KILL cleanup fields remain visible and are not relabelled.
All 110 retained stage files have matching remote/local hashes.

| Arm | TTFT ms | TPOT ms | Finite-window output tokens/s |
| --- | ---: | ---: | ---: |
| baseline-AB | 1024.554916 | 94.799826 | 9.797235 |
| candidate-AB | 858.870490 | 70.714876 | 13.007960 |
| candidate-BA | 865.604204 | 70.180564 | 13.089270 |
| baseline-BA | 876.261260 | 80.273610 | 11.561235 |

Separate AB/BA TPOT reductions are25.406%/12.573%, TTFT reductions
16.171%/1.216%, and finite-window output-rate increases32.772%/13.217%.
Baseline drift is visible; these small native samples are not pooled, stable
gain, sustained serving throughput, shader time or a vendor comparison.
Both arms use the same controller142cc297, actual dd6/5e2 worker and eight
images, with prefill16/split8 on and partial GEMV off. Only packed16/packed64
scheduling differs. Defaults, prior HTTP losses and33 open M1 gates stand.
This warrants a new matched HTTP series, not a vLLM-win claim.

Actual report SHA256:
`cbef8172a332d0d3086889f8ac5cab4b1f913114a4c37535bcf23b062c812015`.
Actual outer SHA256:
`e2e234a94971f996b4f4bb1167ed80664ee1362c45a7d091a816b6d3ce2d7617`.
The new HTTP helper is independently source-reviewed and its fresh47 fixtures
are running remotely; no HTTP launch/result follows merely from those tests.

At 18:12 UTC, R2 focused routing finishes with four passed and two failed tests,
status 1, clean reaping and no signals. It expects 649 packets for context 1 but
observes 613; the 16+1 prefill tail expects 1,265 total but observes 1,229.
Production split attention selects only one physical row at context 128..=256;
both observations are correct for these short fixtures. R3 changes only this
test module: explicit fallback counts, split-boundary post-prompt coverage at
contexts 127/128/129/256/257, both V21 roots, and unchanged 72 V20 substitutions
with full normalized command and group parity. The native 128/128 schedule
remains 87,711 packets and 135 batches; 5,783 groups and 9,144 substitutions
remain source expectations, not newly observed GPU counters. R2's seven clean
gates and failure remain separate; R3 does not inherit their pass status.
By 18:27 UTC R3 passes its first six gates, including all 13 recipe fixtures,
source preparation, strict Clippy and default/plain/model all-binary checks.
Its separate 24-test native harness passes in 83.763 seconds with clean outer
cleanup and no signals. Test compilation is running; no focused-test pass,
new controller or GPU measurement is claimed yet. A direct GitHub fetch at
18:24 reconfirms published fe2o3 main as 13e; the dirty local worktree is not
used as the upstream authority or changed by this check.

R3 subsequently passes 457 library tests (11 inherited explicit ignores, one
existing high-memory exclusion), 56 ordinary CLI tests, 63 model CLI tests,
both one-test composed/partial actual-image gates, and the focused 6/2/1
cohorts. At 18:38:33 UTC source policy fails cleanly with 44 passed and one
failed test. Its new whole-file `--gemv` check matches V25's existing negative
parser test, which explicitly rejects that option. The R4 correction changes
only `tests/source_policy.rs`: require `#[cfg(test)]` and apply the unchanged
prohibition to preceding production code. The negative fixture and all parser
behavior remain unchanged. All 22 gates remain required; source-policy compile
and execution move immediately after library compilation. R3's failed outer
receipt is `ece5974686a8d60d42e2fc2e3631cdf7bc199132c69e16e7004a3b0bbebd5319`;
its complete inner review and three selected test ELFs are retained locally.
No native launch follows the failed gate.

Fresh R4 starts at 18:52 UTC. Its 13 recipe fixtures pass; the independent
native harness passes all 24 fixtures in 84.457 seconds, ending at 18:54:30
with clean outer teardown and no signals. The 22-gate CPU campaign is running
under unchanged limits. The HTTP successor pins this actual R4 collector and
BuildV4, preserves the matched transport/settings and replays all four native
arms; its 44 written fixtures remain unrun. No new native or HTTP result is
claimed. An 18:54 mi350 sysfs census observes eight idle GPUs; this is not a
future ownership/resource admission.

By 19:08:50 UTC R4 passes all 21 pre-build gates at a001: strict Clippy,
default/plain/model checks, 45 source-policy tests, focused 6/2/1 cohorts,
457 library tests, 56 ordinary and 63 model CLI tests, and six explicit
actual-image tests. Existing ignores/exclusion are unchanged. The release
controller build is running. Fresh native owner `ferric-opt-v5-v25.cpys8E7W`
contains verified static helpers, all eight images and unchanged dd6 worker,
but has no launched workload or accepted native result.

R4 then completes all 22 CPU gates, with the release build clean at 19:24:17
UTC. The BuildV4 collector completes cleanly at 19:24:52, worker admission at
19:25:44 and plan preparation at 19:27:06. The separately reviewed HTTP
successor passes all 44 tests, clean at 19:28:29. These are actual CPU and
metadata milestones, not new native numerical or timing observations.
Fresh native AB/BA completes cleanly at 19:41:36 UTC on mi350 at
`/tmp/ferric-opt-v5-v25.cpys8E7W`. All four starts pass, with all 2,048 output
IDs and decoded bytes matching, no signals, and no remaining KFD owners.

| Start | TTFT ms | TPOT ms | Finite-window output tokens/s |
| --- | ---: | ---: | ---: |
| Baseline AB | 877.878138 | 79.613330 | 11.647774 |
| Candidate AB | 945.068094 | 82.110499 | 11.253843 |
| Candidate BA | 1040.976727 | 91.870497 | 10.071339 |
| Baseline BA | 885.450433 | 91.893385 | 10.194000 |

Candidate/baseline TPOT ratios are 1.031366 in AB and 0.999751 in BA;
TTFT ratios are 1.076537 and 1.175647. This is no demonstrated gain, so the
candidate is not promoted to HTTP or made the default. Each start has one
excluded warmup and three measured requests. These native ingress measurements
are neither HTTP/vendor results nor sustained throughput; orders are not pooled
or cherry-picked. Report SHA256 is
`2d5c5846318a93fd888cf35e27cd87f9d53914976bb8371bc1a4e3f92a4d05a4`;
clean supervisor SHA256 is
`ff36b8b0e62cd722cfc3c8b2acb091bee8283952d7e859bf082af8af92bf2ae0`.
The complete stage is retained under `products/partial-gemv-native-r4`.
The two
historical accepted pairs still show roughly a 16x TPOT gap, and their third
declared pair remains rejected. Existing ignores/exclusion, compiler warning
limitations and open M1 qualification gates are unchanged.

After full local custody verification, exact owned-cache retirement reclaims
4,430,888,960 allocated bytes, retains 27 selected products, and reconstructs
the historical and R4 producers. No resource limit or reserve changes. Fresh
13e qualification then passes 26 recipe fixtures, source preparation, scoped
strict CLI Clippy and release CLI compilation (clean at 19:59:41 UTC).
At 20:01:20, the backend-warning gate fails despite Cargo status 0: it observes
the expected 19 backend warnings plus an extractor `prepare` warning for nine
arguments. That signature already exists upstream; the policy did not admit
extractor warnings. Independent source review confirms the unchanged upstream
signature and exact formatted diagnostic span. The separate continuation passes
15 fixtures at 20:17:52 and fresh closed-warning lint at 20:18:07; the original
failure remains retained. Four test compilations and seven execution cohorts
subsequently pass: focused counts 10/7/2, regression counts 27/214/24, and
source policy five with one existing ignore. Backend regression retains 99
existing ignores. The final backend build stops at 20:33:33 on the unchanged
stage reserve. Its actual failed outer receipt and logs are retained locally;
a fresh process census finds no remaining build processes. Exact locally
archived diagnostic copies are removed without changing limits: one 225161216
allocated-byte R1 copy, 373014528 bytes of R2 diagnostics, and seven further R1
copies totaling 136212480 allocated bytes. The R2 operation completes under
the existing CPU guard; the narrow R1 operations are metadata-only under the
existing build lock. Backend-build/a002 passes at 20:47:12 and retain/a001 at
20:47:33, selecting the 13 prior continuation passes without relabeling either
failure. Actual ProducerV6 SHA256 is
`40c552c254353fa12adc5f50c17ae75e46ed078766295f79399ea3e0ac362a13`.
No emission or native qualification is implied. The completed compiler's unused
cache retirement completes at 20:56:07 after complete local archive verification,
reclaiming 3,644,727,296 allocated bytes and preserving six selected binary/DSO
paths. Actual V6 reconstructs afterward. Archive SHA256 is
`8f63c595af0c1c5ae607f6aa9ac436885e2cea61d3f738cc81609c62cebdd06d`;
retirement receipt is `131e42b903d7e8b3a979fa2411d5fd9d79b1eedd59ea46c5a6c8ef37a231f8c5`.
The continuation manifest is
`0cf897781e273371dc2a3f2dc42ecae3e4237b495143531b1dd722533107d275`.
The exact offline13e SDK import subsequently passes at 20:04:21; packed-u32's
14 recipe fixtures pass at 20:12:25 and actual lock/source preparation at
20:13:43. Its 26 Rust tests, strict lint and emission remain pending.
Opt-in ordered64 passes 15 remote native-harness fixtures at 20:54:10 and
eight CPU recipe fixtures at 20:56:32; actual source/lock preparation passes at
20:57:08 with exactly nine new runtime packages and one edge, preserving 325
existing packages. Actual lock SHA256 is
`a773a4dce5fe76acfaf13a5463943f940e3cdd467d504b6494e54a647b8c7a68`.
Its controller is not yet built or measured. Both native arms keep partial GEMV
off. Paired-K16 prefill and a separate bounded completion-polling candidate
remain source work, not measured gains.

A separate exact-13e packed-u32 source expansion now covers TP1 C1 Q/K/V and
gate/up BF16 outputs, with 19/4/7/3 authored fixtures and independent source
review. It retains contribution order, typed checked loads and the original
activation pack. QKV and gate/up require distinct normalization epochs;
packed weights add 240 MiB per layer. CPU/emission/native/inference integration
remain unrun. It does not alter the frozen Q-only qualification/emission lane
or the R4 controller.

At 19:19 UTC a direct GitHub fetch observes main `9d4387d` after two CI-only
commits. Independent metadata review confirms the complete crates tree,
Cargo manifests/lock, toolchain and eight capture-overlay bases are unchanged
from `13e`. No fe2o3 Git-HEAD-dependent build hook is found. The queued `13e` producer,
SDK bundle and source labels remain truthful and unchanged; this scoped
current-code equivalence is not a `9d` build, binary-identity claim or new pass.

The corrected model R3 recipe passes its first 17 phases: 21 recipe tests,
preparation, default/plain/all-binaries checks, 443 library tests, both CLI
cohorts, 15 peer tests and the actual V8 metadata gate. The existing 10 ignored
library tests and one known high-memory exclusion are unchanged. The subsequent
direct-policy wrapper fails before rustc because Cargo emitted both build and
release variants of its dependencies. A separately reviewed continuation passes
10 recipe tests, direct policy compilation and the 43-test source-policy suite
with the exact release profile. Its strict Clippy gate then finds four
documentation warnings and two unchecked narrowing conversions in the diagnostic
summary. No GPU launch follows that failed gate. The next complete source
validation runs Clippy early and uses fresh receipts rather than relabeling
old passes.
The two-file fix is now integrated: checked index conversions preserve the
infallible private-capture API, and a new fixture covers all 21 operations over
256 batches. Fresh Model R4 passes its 22 recipe tests, preparation and default
check. Its first plain check reaches the stage reserve and remains a status-125
failure; its observed owned PIDs subsequently no longer exist. A fresh a002
retry passes after archived-source cleanup, as does the all-binaries check.
Early Clippy then finds one remaining diagnostic queue-bound lint. The exact
one-line correction (`len > MAX_GROUP_PACKETS`) preserves the allowed 16+1
deferred-tag case; independent source review confirms no behavior change.
Raw source R6 is frozen for a fresh Model R5 campaign, not reuse of prior
passes. Model R5 reaches early Clippy after its selected preparation and
default/plain/all-binaries checks. Attempt `clippy/a001` fails with `status=1`
and `returncode=1` at 2026-09-21 00:52:26 UTC, with clean, signal-free owned
teardown. It finds the unused shared `enable_model_timestamps` entry point and
`large_enum_variant` caused by inline optional diagnostic state. The applied
two-file fix boxes only that optional state and adds a method-local
`allow(dead_code)` for controllers that share the module without opting in.
Frozen raw source R7 contains exactly those two changed files. Its guarded
development Clippy preflight passes, but is not reused as a required successor
gate. Fresh Model R6 passes its first six phases, all at `a001`: 22 recipe
tests, preparation, default check, plain check, all-binaries check and strict
Clippy. The strict lint gate completes cleanly at 2026-09-21 01:21:19 UTC without
signals. Library compilation then passes, followed by 27 collector tests, one
driver test and 444 library tests, all at `a001`. The library run retains 10
inherited ignored tests and the one explicit high-memory exclusion; its clean
guard completes at 01:30:31 UTC. All 23 Model R6 phases subsequently pass at
`a001`, including 52 ordinary CLI tests, 58 diagnostic CLI tests, four FIFO
tests, 15 peer tests, both actual-image metadata gates, 43 source-policy tests
and both release builds. Each CLI cohort retains five ignored tests, and the
peer cohort retains one. The actual prepared inventory
has SHA256 `8d1382a448cc4d22f7f06e9780b30d91e0b3eb3f981d2bec58d7724cf475a077`.
Native R8 separately passes all 26 remote CPU harness tests in 46.238 seconds,
with clean teardown at 01:14:13 UTC. Earlier R6/R7 harness results remain
historical. The R8 build collector and plan gates pass cleanly at 01:45:40 and
01:45:46 UTC. The actual ordinary controller is SHA256
`0d652d9672cd3737a470dbc8f325fb399e6c9e52c09ddb79763a92097ba04a0d`
(10,191,928 bytes); the diagnostic controller is
`87045480337b8d11722bdc27387e3303fbfd74b4a7d6301f2fbca29302f08bb4`
(10,334,960 bytes). The supervised four-cold-arm model campaign passes on
mi350 after fresh all-eight-idle/no-KFD admission. All 512 output IDs and
decoded bytes match. Both diagnostic arms retain 83,139 validated raw records
and 10,239 original groups each: 166,278 records and 20,478 groups total.
Every arm preserves 83,139 dispatches/135 batches. The outer supervisor exits
cleanly at 01:58:52 UTC without signals or remaining owned/foreign KFD PIDs.
All four retained inner cleanup receipts have TERM/KILL flags, exit zero and
an absent owned group; the inspected ordinary-baseline pre-cleanup census
contains only its controller zombie. This is not a signal-free inner-cleanup
claim. The accepted report SHA256 is
`122ed545e2ecf6b9547bdb47ebb29a2865270386a229cb8fd9dd065684f0e398`;
the clean outer receipt is
`e712225ff8397dbfac7e50e17d78484c391d6da5a9c012522737f311d9a71199`.
These are full-model correctness and raw-dispatch-interval diagnostics, not
qualified kernel durations, an uninstrumented performance result or a vendor
comparison. Cold starts and recorder overhead remain explicit limitations.
The earlier Model R3 failure is retained as
`products/model-cpu-failure-r3.tar.gz`, SHA256
`923178d7d8c85c5888083cefe04af31fd077c47ed3a4275fb09c5704356bf5f6`.
All 24 native R4 harness tests pass separately. Composition CPU/native R5 is
frozen and independently reviewed against actual Model R6 preparation plus
the original ten-file overlay. Its 14 recipe tests and 22 native-harness CPU
tests pass. Preparation `a001` then fails cleanly with status/returncode one
at `format-check-1`: rustfmt requests another wrapping pass for one error
return in `prefill_kv_copy_v28_live_contract.rs`. No compiler or native gate
passes as a consequence. The complete failed formatting attempt is retained
locally as SHA256
`217511b01712b3959025d714f71803d6ce4341952c4a1e33bcbdbff4ce9620d0`.
A recipe-only bounded fixed-point formatting successor, composition R6,
passes all 33 phases at `a001`, with the raw source unchanged. It includes
15 recipe tests, default/plain/all-binaries checks, strict Clippy, 420 library
tests (10 inherited ignored and one explicit high-memory exclusion), 44
source-policy tests, and focused V28/V25/V22/V19/V17 cohorts of 10/17/6/10/4
passes (one ignored in V19). The six live/model/V25/V22/V19/V17 CLI cohorts
pass 54/61/54/53/53/51 tests, retaining six ignored tests each. The composed
runner case, actual dual-image admission and each V5/V21/V27 image gate pass
one test. Its separate native harness passes 22 CPU tests. Final release
build completes cleanly at 02:17:05 UTC; the retained controller is SHA256
`1ffd68bc905ed6a1cc1bfb7dfad88f64b519ed1aede5aa2c079768982960afe6`
(10,224,264 bytes). Build collection, worker admission and plan preparation
pass with clean, signal-free guards at 02:17:21/27/33 UTC. Their full outer
directories and controller are retained locally. Build receipt SHA256 is
`cd8a341e74d847171ace986c3ae9cd5f3789dda36c7a731f0d80c57f245a3d52`;
worker admission is
`41a231b52bd5f1acc1e04eda3ebf000fedf221fb5436d9900ad7caf7e079ea22`;
plan is `c3851fc78a63109ab9140cd2d687294739dac44b6620d35d5eb304054b245516`.
The four-arm native campaign passes through the existing supervisor in the
separate `99je2Ebp` stage: all 16 requests match all 2,048 reference output
IDs and decoded bytes. The outer exits zero with reaped child, clean cleanup,
no errors/signals and no remaining owned/foreign KFD clients. The actual
report SHA256 is `dbf5e34ea6466d84211c85b778fe668d046c5de931491c32b1f4a8118498bcd6`,
outer receipt is `c58f0af2debca9d41626cd0e05bc79c0aa014bb2910ea2f687f06ebff0ffae8d`,
and complete locally retained archive is
`eb1f97196a736c2b9512a12c966a8ddb25d5aa3f09153ead95489dcf70ef564c`.

| Composition R6 Native Arm | Mean TTFT (ms) | Mean TPOT (ms) | Measured-Window Output Tokens/s |
| --- | ---: | ---: | ---: |
| Baseline | 2430.855653 | 93.406125 | 8.954908643 |
| Decode only | 2365.162141 | 76.228505 | 10.625352362 |
| Prefill only | 1021.103724 | 93.120622 | 9.962451944 |
| Prefill + decode | 855.944209 | 85.379419 | 10.940545624 |

Each arm has one excluded warmup and three measured requests, in the fixed
table order. Native TTFT is arrival to first completion; TPOT is first-to-last
completion span/127, and rate is 384 tokens over the full measured ingress
window. These are not HTTP timings or qualified GPU durations. Combined TTFT
is 64.7884% lower than this baseline, TPOT 8.5933% lower and rate 22.1737%
higher, but its TPOT is 12.0046% worse than decode-only. Runtime audit finds
decode-only request TPOTs of 75.803/76.963/75.919 ms versus combined
75.904/100.344/79.890 ms. The slowdown persists inside the slower request's
batches, with no route mismatch or proven external cause. All three samples
remain included; this small fixed-order cohort does not establish a stable
gain, cause, confidence interval or vendor win. Inner owned TERM/KILL cleanup
flags remain retained and separate from the clean outer supervisor.

Its older unlaunched R4 binding remains superseded. The model capture uses
the separately validated 5e2 worker, not the historical model worker. Compiler
capture R3 at `91d989a5d` fails cleanly in `cli-focused`: nine tests pass and
one formatting-sensitive source-string assertion fails. Its full failed
archive is retained and matches locally/remotely at SHA256
`3c5a2ab0ceeaa8d34beb2a8ac2e614668dd8a0051d3fd92eb2ab1bf39e45c303`.
The reviewed correction changes only that test to inspect AST call order.
Capture R4 is an unlaunched, unstaged 91d checkpoint, not a pass. Capture R5
is based on `c60cd746e63b34b9072a493744d73b87ed1defc3` and passes actual
staging, all 13 recipe tests, preparation and CLI build. Its
`compile-backend/a001` then fails while reusing a stale R3 dependency artifact
reported `fresh: true`; the required API exists in current source. The prior
CLI pass does not establish a valid current dependency closure. Following the
verified R2 cache retirement below, fresh CLI build, CLI compilation and
backend compilation pass at `a002`; extractor/policy compilation and the
subsequent tests pass at `a001`. The focused CLI/backend/extractor tests pass
10/7/2 cases. Selected regressions pass 27 CLI, 195 backend (91 inherited
ignored), 24 extractor and five source-policy tests (one inherited ignored).
Strict `cli-clippy/a001` then fails cleanly at 04:27:05 UTC on unchanged
upstream `fe2o3-kernel-ir`: `too_many_arguments` in `chain_v1.rs:250` and
`nonminimal_bool` in `local_frame_effects_v1.rs:686`. No signals or cleanup
errors occur. A separate package-only scoped continuation passes eight recipe
tests and strict CLI Clippy, then fails strict backend Clippy at 04:45:08 UTC.
All 20 diagnostics occur in five files byte-identical to c60 and outside the
eight capture-patched paths. This is a source comparison, not a separately
executed baseline lint pass. No lint is suppressed or relabeled as passing.
Final backend build and producer retention have not run; no complete successor
producer or emission pass is claimed. The full failed outer stdout is retained
at SHA256 `187de9ebc8dbf5a65c1d1cfc2ff3c235f1a111e5c9ce0eb608385b90823b508e`.
The separate copied inner stdout is empty and is not accepted as complete
evidence. A source-reviewed, frozen `capture-known-warnings-r1` continuation
now has four phases and 12 written, unrun fixtures. Its manifest is
`7138f45b861b54d19690c7a204de178945488868a647f120e222bb927dd93b7a`.
It preserves both strict failures and requires exactly the 20 reviewed
warning identities and unchanged source hashes in fresh structured output;
new, missing, duplicate or changed diagnostics fail. It neither suppresses
lint categories nor edits upstream source. Any future V4 producer explicitly
states `known-upstream-warnings`, `strict_backend_lint_pass: false` and
`dependencies_linted: false`. No such producer exists yet. Its emission
consumer remains a documented, unimplemented follow-up.

Latest observed main, refreshed in the separate bare audit repository at
05:04 UTC without touching the user's dirty worktree, is
`ecdbf612265e81395531184063c2bb33b3f6062b`. The retained content audit finds
only three JavaScript debugger-script/test changes from c60; the `crates`
tree, `Cargo.toml` and `Cargo.lock` Git objects are identical. The producer
remains identified as c60, and the new JavaScript tests have not run. All
nine normal KFD crate trees remain identical to 5e2, so the actual dd6 worker
keeps its 5e2 producer label. This is a source-content audit, not a new build
or independent attestation; a core main push still requires fresh fetch/rebase.
Existing evidence is not relabeled. Fifteen unused remote archive
copies were removed after local checksum verification, reclaiming 280,162,304
allocated bytes while preserving all current sources, tools and receipts.
The superseded capture source R2 was removed after comparison with its retained
remote archive; its local download subsequently completed and matched SHA256
`6a9bec31e5465d60903603d144701e551fcf372151c57db02941f973cbbdddf6`.
Two further duplicate archives were removed only after complete local hash
verification. A separate executable cleanup refused before export because its
roster overlapped current compilation records; none of those files was removed.
A narrowed eight-executable cleanup subsequently passes after full local
archive/member verification, reclaiming 93,749,248 allocated bytes. All 25
referenced hard-link pairs remain. The two fully archived old model source
trees were then removed with complete census and tar comparison checks,
reclaiming 100,442,112 allocated bytes; their recipes, reviews and failed
receipts remain unchanged. These are owned-file cleanups, not broader cache
deletions or changes to shared-host limits.
Two further bounded retirements have completed after full archive/member
verification and local retention acknowledgement: obsolete compiler source
roots `source` and `source-c4c` reclaim 208,416,768 allocated bytes; the exact
12 files and three roots `tools`, `tools-c4c`, `tools-vendorfix` reclaim
700,166,144 allocated bytes. Current compiler tools, LLVM worker, toolchain,
sources and receipts remain outside that deletion scope. The retired-byte
counts are not a reservation of future build headroom.
A separate six-tree compiler-history retirement now completes successfully.
Its archive and complete local member/metadata acknowledgement remain retained.
Attempts `retire/a001` and `retire/a002` failed cleanly at the `fuser` no-use
check, before intent or deletion; those failures are unchanged. A bounded
read-only observer subsequently records `one_complete_scan_clear` across all
41,091 paths, without authorizing cleanup itself. Fresh `retire/a003` passes
the unchanged retirement checks and exits cleanly at 01:33:17 UTC, reclaiming
638,459,904 allocated bytes. Local receipts and the archive remain retained.
Archive SHA256 is
`9735942e10d49fb37eb77228e7bf3e4dfc517c721dfbb6b1de2c95408f7e636e`,
export SHA256 is
`fbcc3905eaf4ac89ae0485b73ff511a2852f2be378803b96630be6dba7fda4c8`,
and local-retention acknowledgement SHA256 is
`2a2b98bfd096ef679325553e57964c522aef48ced6c9008cf99a1cba14464e50`.
The completed Model R8 `634McnZ4` GPU stage and its duplicate remote archive
are also removed after verified local retention of the complete archive,
SHA256 `13447480bbc0804f2004724de7a1d429676daf099b810c59c0ac2295fc4b89a1`
(8,569,256 bytes). The new composition stage is separate. No new signals are
sent by this cleanup; the historical frozen inner zombie cleanup remains
recorded as above.
A subsequent selected `target-stable` retirement rechecks and retains all
selected producer evidence before deletion. Its export/custody archive,
local acknowledgement and clean outer are retained under
`products/target-stable-retirement-r1` and the corresponding outer directory.
It keeps 24 files and removes 14,778 files/2,995 directories, reclaiming
6,264,061,952 allocated regular-file bytes. Source, toolchain and reviews stay
untouched. The signal-free guard completes at 02:35:47 UTC and observes stage
usage of 6,026,477,568 bytes; this is a point-in-time value, not reserved
headroom or relaxed limits.

The separate c60 compiler-cache recovery preserves its failed R1 export:
the protected model collector was called with the nightly rather than its
recorded stable `PATH`, and the guard exits cleanly with status one before
archive creation or deletion. Recorded model source/input hashes remain
unchanged. R2 invokes that same protected collector through the hash-bound
stable environment before exporting; export passes cleanly at 03:49:20 UTC.
Its 139,364,727-byte custody archive is retained locally at SHA256
`bb59f26fefb7cabd22cefaf3391e32544e9b37e8a138a3fe692ef0867d13efd4`,
with complete archived-byte/member-metadata verification and acknowledgement
`3315a6b29ce74c89664db7cbdc29a74bc1020feea13264028ac6c44ba7824dee`.
The archive retains source, tools, reviews and selected failure diagnostics;
it does not claim to archive every disposable cache byte. Fresh retirement
then removes 3,514 files and 674 directories from the owned `target` cache,
reclaiming 3,131,686,912 allocated regular-file bytes. It preserves the cache
root, `target-stable`, selected producers, source, toolchain and reviews.
The guard completes cleanly without signals at 03:56:15 UTC and observes
stage usage of 7,300,530,176 bytes. Retirement receipt SHA256 is
`e3528b91de23245c2f6d6db1389fea98121ad26d0606e87d68f8878cbf0e3171`.

The separate V28 Pages source passes all four fresh remote phases, including
the 320-through-1440-pixel layout sweep, 64 QA captures and 27 supplemental
captures. Root reviewed all 27 supplemental images; the measurement agent
directly reviewed all 64 QA images. No blocking layout defect was found.
Artifact receipt SHA256 is
`0b182c2b129a54f125f3b9db8ac5f3e722e45bd11cbfe378d7236874e8e2bf92`;
the supplemental capture receipt is
`78e7836830265a9e5e8d5c21bfc4db57cbdbbb03478bd8eac744a2fd382ff22c`.
All 22 publication tests and static preparation pass remotely. Static-only
commit `aebd59fa2ca83e1c6d772e5deaed8705fdbb8f97` deploys successfully through
run `35547903324`, with no GitHub build. The live index matches the tested
SHA256 `ed378bad4404862989e3e8a5ad577eecd0933c41afe701a100e7fe38edd00311`.
The latest matched HTTP result remains a loss to vLLM.
The newer HTTP R3/Composition R6/Model R8 site source and assertions are
updated and independently reviewed. Its exact 34-file source archive is
`111ce8f63f926d26dfb95f4bf80a3a37d6d754a5e0280ca9534d876fd6262d0f`.
Fresh remote preparation passes cleanly at 03:51:37 UTC, followed by a
separate `validate.mjs` structural/claim check at 03:52:59 UTC. Both outer
receipts are retained locally. Full remote QA subsequently passes, including
all widths 320 through 1440, followed by 64 standard and 49 supplemental
captures. Root directly reviews all 113 images without finding a publication
blocker. All 23 publication tests and static preparation pass remotely.
Static-only commit `e8cd0f634de8eeb6b11399f1eb7b4db85c009309` deploys through
successful deploy-only run `35562392538`; no GitHub build runs. The live index
matches SHA256 `d383434a320a6e9faca075b1a4ee3771fa010aff91289a724b0172c30ae547f3`.
Artifact SHA256 is `f43d28a1d2f2f7b62aa7254b832157e706411e52e445e9136905ae78c14b4aea`.
Only the seven static files and existing deployment workflow are published;
Ferric implementation remains private. The new scoreboard explicitly retains
both accepted losses to vLLM and the rejected third start.

The private HTTP R2 successor is frozen and independently source-reviewed.
All 39 CPU tests pass in 0.039 seconds with clean, signal-free teardown at
2026-09-21 01:04:46 UTC. This is tooling validation, not an HTTP benchmark.
It requires the actual dd6/5e2 worker and
the complete 38-file native roster, including unchanged worker-admission
replay; historical f68 remains only in prior prerequisite evidence.
R2 retains `NATIVE_BINDINGS=None` as historical source. R3 changes only its
actual accepted R6 helper bindings, fresh CPU wrapper path, README and
manifest. Manifest SHA256 is
`49781f13b9727c00feeda651167e738532b0017747eb21e91cec30a6ec25894f`;
policy SHA256 is
`91d2d71c46563beedbd2c1f90afb240cfd8f96e5bdd2ccc7769791a39bafd02c`.
Its 39 CPU tests pass with clean, signal-free teardown at 02:39:11 UTC.
The seven images, shared server/client,
comparison profile, limits and alternating AB/BA policy are unchanged. A
read-only mi350 audit finds the exact vLLM0.28.0 image and pinned inputs
available without reserving GPUs. The actual HTTP plan and preparation are
retained; plan SHA256 is
`981c0991d88692b36d53df47866b0292c16f843d281ffbbe74fdfc7c701b5e04`.
Under `/tmp/ferric-opt-v6-http.l1xwwb9Z`, six exact genuine per-engine
authorizations and the AB/BA/AB roster are now retained locally in
`products/http-r3-launch-r1`. Roster SHA256 is
`5f90db3624430bac988c865fb5744e489ece059b0d7f27c93b2cb184d8023642`;
complete series manifest is
`1753e5cb0fd40b7f4b6dcedef54d8dfa506915de26e317515e93c3354a5d17ee`.
Pair 1 now accepts both engine cohorts and paired replay, with the provisional
metrics above. Ferric's retained 42 final requests match all 5,376 reference
token IDs and all output bytes. Its controller/client exit zero without
signals, with joined threads and a clean worker close; the sampled KFD census
accepts 1,059 observations, with no claim about absence between samples.
Pair 2 also passes. Pair 3's Ferric start is rejected on foreign KFD activity,
with timing not admitted and completed cleanup that includes forced shared-client
cleanup; its vendor start is not launched. The foreign process later exits
without integration-lead signals. The original series remains incomplete and
immutable; its explicit rejected-series output is retained, not repaired
by replacing the failed start. Retention R2 passes all seven remote CPU tests
in 0.034 seconds, with clean, signal-free guard completion at 03:31:25 UTC.
The full outer receipt is retained locally. Its unchanged helper/manifest
hashes are `44a740e0...` / `5fd7fd91...`. The actual GPU-host retention then
exits zero, retaining 390 input files / 17,630,313 bytes. Archive SHA256 is
`d6a4af0eb6fb7c2fa21bf9a3005cec051cb7813eb5fc605aaee49e1b0f109e4e`,
and manifest SHA256 is
`6c358f87d86e0e1a9ab1554a4499dbec99b73ac5dafaa0e0c93108fa291a0690`.
Both copied hashes match the remote checksum file; the complete local audit
verifies all 391 members / 17,827,720 bytes against the manifest with no extra
or missing members and no extraction. Independent metadata review also
confirms the checksum and member-name roster. The launch record truthfully
transcribes the tool result: stdout/stderr were not captured separately, and
the operator record is not an independent attestation. Failed pair 3 and the
absent third vendor remain recorded. No timing is readmitted. A later bounded
HTTP-only cleanup attempt exits one before intent or deletion because the
same-UID systemd process 634685 has an unreadable `/proc/634685/cwd`.
Its result records `accepted=false`, zero removed files/directories and no
automatic retry; the entire external attempt, including stdout/stderr, is
retained at `products/ferric-http-r3-cleanup-r1`. No workload signals are
sent. The HTTP and native stages and model remain untouched; no no-use check
is relaxed to complete cleanup.
The original user-confirmation evidence
is extracted and hash-checked at
`products/composition-r6-inputs/user-confirmation.json`, SHA256
`6dcbd931b3ff6aee9db483b6485ceb3543a5bd4c6ff0bafaaa763e88b158d2a2`.
It binds the exact root-issued per-engine authorizations, not an authorization
bypass or substitute for the actual launch receipts. Every failed or
contaminated attempt must remain in the declared series.

## Implemented Changes

The partial-only V20 route is integrated in the private Ferric tree: ten
modified adapter files, four new adapter modules and ten unchanged historical
device files. It replaces only attention-output/down partial GEMV for one-row
work, including one-token prefill tails. Both comparison modes admit the same
eight images and preserve allocation, grouping and reduction order. No
end-to-end gain is claimed. Its independent source-reviewed CPU recipe has
22 fresh phases and 12 written fixtures; manifest SHA256 is
`6c45cb1606e530ae77ecc9a3282165ece63a0bcfbae5b54fe44022cd4face85e`.
All phases remain unrun. The independently reviewed native AB/BA collector
and harness are source-frozen at manifest
`660f71ca7b55efd75c2e6d0b54fc2f0988d0ee8a47941fb636d18756bb45de59`.
Its 23 fixtures are written, unrun. The collector requires the actual 22-phase
producer, controlled new lock, exact image gates and fresh harness receipt;
no actual build receipt, plan or native result exists for this candidate.
The local transfer archive is ready at SHA256
`ec699346436aa677aaa94b8048c1a8a313e4c4dae6aa7720e2930da8a1289192`
(140,248 bytes), containing the frozen recipe, exact overlay and canonical
historical V20 image. Root reviewed all four formatting-only base differences
against the accepted prepared inventory. This is not remote staging, resource
admission or an executed test. Existing remote cache growth is unmeasured;
the roughly 1.37 GB last-observed headroom is not guaranteed sufficient for
the complete release build.

Packed-u32 GEMV has 16 written Rust tests plus ten for its activation-packing
overlay, all unrun. Its R2 recipe passes 11 fixture tests, then preparation
fails because its dependency audit incorrectly requires a third Pliron
package absent from the actual lock. R3 preserves that actual lock and exact
c60 source, corrects the two-package audit and adds three fixtures; its nine
fresh phases are unrun. Manifest SHA256 is
`4bf0d8110d6e0466ee4ce641ce830c8a06dcecdfd3d03d9fe5d1f41475f7bc27`.
The separate pair-only guarded-load CPU recipe has 14 written, unrun tests.
It still binds obsolete capture R3 and
cannot consume the new R5 capture without a reviewed successor. These are
source proposals, not emitted kernels, measured gains or current admission.

After the earlier no-match check, a new exact `ferric42-tp2-closed-token`
run appeared on mi350. Under the user's explicit authorization, root verified
the supervisor and its three descendant identities using UID, start time and
namespace, then sent SIGTERM only to supervisor PID 1820595 via pidfd. All
four original processes exited without SIGKILL or signals to unrelated jobs.
This is distinct from the earlier Pair 3 foreign process's unaided exit.
Subsequent mi300x-2 and mi350 SSH attempts time out after successful DNS
resolution; no local build or alternate-host
build is substituted. This access issue delays new CPU validation, not the
already completed site deployment or existing performance evidence.

### Restored Access And Fresh Validation

The user's 2026-09-21 access update is confirmed on both hosts. At 16:38 UTC,
mi350 has eight idle GPUs, an empty KFD process list and no fuser owners.
Debugfs is permission-denied and not claimed inspected. All 28 checked native
prerequisite inputs retain their hashes; six large model weight files have
matching sizes but are not freshly rehashed in this snapshot. Future native
admission must still perform its full checks. No new GPU workload is launched.

Partial-GEMV CPU R1 passes its first three selected phases at a001: 12 recipe
tests, preparation and default Rust check. The prepared source inventory is
`3b835581838794f32cb20d0db45d10e534704e96f591592354103fddfc0f30c1`;
its controlled adapter lock is
`043d8a53597a77eb3197fd8518f6b3518ab58187a4f8dd99cab91e44287601f7`.
Default check exits cleanly at 16:44:15 UTC. All 23 native-harness fixtures
pass in 83.937 seconds, with clean, signal-free outer completion at 16:46:49.
The plain controller check a001 stops at 16:55:58 UTC with status 125 after
reaching the stage reserve at 12,340,641,792 bytes. The supervisor sends TERM
only to its own process group and reaps the child; its receipt explicitly
leaves detached cleanup unverified. A fresh UID 1046 process census at
17:00:55 UTC finds only systemd and the inspection SSH processes, with no
remaining build processes. This later observation does not rewrite the failed
receipt. The full failed outer and completed-to-date inner review are retained
locally. No complete controller producer or native candidate result is
claimed. A local review-directory snapshot taken
while default-check ran is explicitly named `snapshot-20260921-1643`; its
in-progress phase files are not complete evidence.

The exact obsolete Capture R3 retirement completes cleanly at 16:49:45 UTC.
Local acknowledgement `364792b1526050b8701fa921d567a098253ecd883dbdedf89c2ee1a84bf45fc6`
verifies every archive member before remote custody/no-use checks and deletion.
Only its three source/tools/review trees and duplicate archive are removed:
248,416,216 content bytes and 286,621,696 census-allocated bytes. Retirement
receipt SHA256 is `32293ac73dea5b32ce481d9f3698739e27ade5d845071ea027293bf7688eae05`.
Full local archive, acknowledgement, intent, no-use checks and outer remain
retained. Current sources, targets, original outer receipts and helper recipes
remain untouched. The 12 GiB stage cap and 512 MiB reserve do not change.

Subsequent post-TERM cache growth leaves the stage above its admission reserve.
The existing lock-based duplicate-archive cleanup pattern removes only the
two fresh-hash-verified remote copies of c60 recovery R2 and target-stable
retirement R1 custody archives. Both full local archives, acknowledgements,
exports and all other remote recovery files remain. This cleanup-only helper
exits zero and reclaims 207,138,816 allocated bytes; it does not override any
build admission limit.

The independently reviewed c60 target retirement then streams a 207,831,037-byte
archive directly to local custody, SHA256
`4ad3f610b9358e409d7113abdd38a0faef72a3d102e2a238d9c1d6d24447cffe`.
Export completes cleanly at 17:13:03 UTC. Full local member verification yields
ACK `ca765333674b14ac1357d1e04d38dcff5fad4d66f08aa54c0433bfa628fd2d3e`.
Guarded retirement completes without signals at 17:15:09 UTC, removing 4,691
unused cache files and 878 empty directories. Actual allocation falls from
3,428,962,304 to 469,778,432 bytes: 2,959,183,872 bytes reclaimed. All six selected
binary paths remain, and historical/protected products pass fresh record
rechecks. Receipt SHA256 is
`b795872ab6ad2ee69de454e460ba61fc0a5afb7bb00e6c49b01b36cd2d559791`.
Only the compiler cache is retired; current Ferric target, toolchains, sources,
reviews and workers remain. The partial-GEMV sequence now retries plain-check
at a002 and stops on the first failure in subsequent gates. The retry passes
cleanly at 17:17:22 UTC and the all-binaries check passes at 17:17:57 UTC.
Clippy then fails cleanly at 17:20:22 UTC on two
`clippy::fn_params_excessive_bools` diagnostics in the new partial-GEMV
configuration methods. Failed outer SHA256 is
`72523445c2f34de1f16ed758708e0b6ad387233d2d60e10273d41af25743ed98`;
complete outer stdout is
`ab957f6fb63805c9454240499feab6caaa882b29f08493b58c3d99bf12f5f73d`.
The full R1 inner review through this failure is retained separately.

Root's reviewed fix changes four existing overlay paths: it adds an explicit
Baseline/Prefetch4 mode, reexports it, converts the runner and test helper's
fourth boolean, and checks both conversions within the existing six routing
tests. It preserves the source-policy assignment marker and ordering, with no
new lint suppression or runtime protocol/arithmetic change. A fresh R2 CPU
and native-collector successor is being frozen; R1 passes are not relabeled.

Fresh upstream main is `13e00263000362532d72a5dde52749349a9af122`.
Its nine normal runtime trees remain identical to the actual worker's 5e2
trees, but its compiler changes 337 files. The eight-file capture rebase is
source-reviewed at manifest
`7b0c571f15bd8753b11d1001426dd2d1a39cb5f5a3738584215bc504cd9ec8f7`.
It preserves the three new upstream module declarations and corrected AST
test. Source review is not compilation or emission. A fresh 18-phase campaign
is implemented and source-reviewed but unrun; it requires a fresh cache, with no
c60 successes reused. Historical warning identities must be remapped and any
unmapped diagnostic rejected. The existing c60 backend inner stdout is also
confirmed empty on the remote host; the complete outer remains authoritative.
The local 13e source archive is now frozen at
`66265da1bb7d89ee7035573f4a587ceb921539ddce2953975bc8bf562c181060`
(17,168,624 bytes). Its 6,514 files differ from exact upstream only at the six
reviewed modified paths and two additions. Root verifies the complete file
manifest and artifact manifest; Cargo files and toolchain remain unchanged.
This is a source input, not a built compiler or an executed test gate.
The new compiler recipe manifest is
`d5056aca967c419fd4aeb2339fec66e5b75e75c8213b7b8749b29d58e0ee10c3`,
with 26 written fixtures and a source-reviewed exact 19-warning expectation.
The reduction from the historical 20 warnings is explained by upstream's
replacement of a production function with an already-allowed multi-entry
version and a test-only compatibility wrapper. Actual diagnostics remain
unobserved; package-scoped/known-warning lint is not strict backend or
dependency lint. The packed-u32 13e source successor is separately reviewed
at manifest `1777a6ac1020b08888fe834d3e534a2f4b613f1bdcecc115d10fedd3f2c79a38`.
Its three-path delta changes only SDK pins and the existing combined-roster
assertion to expect its two actual exports. Its 26 tests and controlled lock
resolution remain unrun.

The existing V25 controller accepts the explicit composition:

```text
--split-attention-mode split8-v21 --c1-packet-mode packed16-v22
```

Omitting the packet flag preserves historical V25 behavior and event metadata.
Explicit selection records both policies. Packed state now distinguishes the
baseline ten-packet and split eleven-packet attention producer segments. The
split forward retains all 652 packets, grouped as forty groups of sixteen plus
twelve. Persistent scratch remains owned until teardown; partial/merge queue
order is preserved across group boundaries. Headless and multirow batches
remain unpacked. Eight new tests cover command/address equivalence, scratch
reuse, fallback, all 41 submit/wait failure frontiers, rollover, selection,
schedule mismatch, readback failure and metadata. Focused V25/V22, three CLI
suites, 40 source-policy tests and 404 broader library tests pass remotely.
The library run retains nine ignored tests and excludes one known high-memory
R33 test. Strict lint and release build also pass. Existing
adapter/worker/image pins remain unchanged for this isolated
comparison; this is not completed latest-runtime adoption.

The core proposal adds `DispatchOrderedBatch64Profiled`, returning raw GPU
start/end ticks with packet/kernel/device/queue identities. It reuses ordered64
execution and its checks. Only a successful capture restores the profiling
property; every error poisons the disposable worker. Raw ticks are not yet
qualified shader durations or nanoseconds. Review corrected malformed patch
headers and a missing unsafe-source inventory update; inherited d10 ordered64
inventory drift is separately documented. The ten-path patch is retained at
`/home/harsh/.codex-tmp/ferric-perf-swarm-v6/runtime/complete.patch`, SHA256
`539488e7182cef528a76d61b3f4a87cf5d66e83aeb968a212926b21f1efd0b4b`.
Patch-application checks pass against exact d10 source. The 13 focused tests,
518 library tests (one ignored), doctests and strict Clippy pass remotely.
Five unsafe-source policy tests pass (one maintenance test ignored), as does the
C ABI oracle against checksum-pinned ROCr 6.4.3 headers. The release worker is
retained outside the disposable cache with SHA256
`dd6bd3b4a910478e85d2f3530be25819153fad1f08bf33bb14dd534514a601a2`.
The remotely formatted patch is retained separately with SHA256
`770fa58d5354624f6ee51ce5ce219c1eaa52608c35e155b7f6eb4b4a3e67324f`;
the original proposal remains unchanged. After fresh main verification, the
validated ten-path core change was pushed as
`3d473f9ffc850a3a762efee8cd4f210d363f3354` with `[skip ci]`; no GitHub run was
created. The existing dirty core worktree and conflicted index were untouched.

The independently reviewed timestamp harness passes all 12 remote CPU fake
tests. Its native campaign passed 210 dependent operations, completion-slot
reuse, idle rollover and 528 guarded-allocation comparisons with signal-free
owned teardown. It accepts only raw
timestamp identities and positive intervals, without nanosecond conversion or
any shader-time claim. The newly built worker, not the historical model worker,
is bound into that separate plan. The retained archive is
`products/timestamps-r1.tar.gz` under the V6 evidence owner, SHA256
`c2feef4d22b7d513477994b7e8c438796a00ba65911252aa8f5b037c07b33ea2`.

## Native Screening

The same-binary four-arm campaign passed all 2,048 complete output token IDs
and decoded bytes. Each arm used one excluded warmup and three measured
requests, in fixed baseline, packed-only, split-only, composed order:

| Arm | Mean TTFT (ms) | Mean TPOT (ms) | Measured-ingress output tokens/s |
| --- | ---: | ---: | ---: |
| Baseline | 2299.826 | 84.737 | 9.799532 |
| Packed only | 2325.464 | 83.253 | 9.923006 |
| Split only | 2302.797 | 73.957 | 10.944131 |
| Composed | 2311.806 | 70.242 | 11.395125 |

Composition reduced TPOT by 17.1069% relative to this run's baseline, with TTFT
0.5209% worse. Relative to split-only, TPOT was 5.0239% lower. These are native
ingress measurements, not HTTP, GPU durations, sustained throughput, confidence
bounds or a new vendor comparison. Do not multiply these gains with older A/B
results. Legacy arm cleanup used signals for its owned process groups and
confirmed them absent; the outer supervisor exited cleanly without signaling.
Archive `products/composition-r3.tar.gz`, SHA256
`bda385eba79356b7bb311eb83c44e0e064c7ec4a598bb6ab0dadff2a696aee43`.

V27's emitted parallel full-page copy passes 28 isolated raw-bit correctness
cells, including both row orders and protected neighbors. Twelve diagnostic
timing cells complete with signal-free owned cleanup. The group16 worker wall
span per full copy was about 5.025 ms for V5 and 28.9 microseconds for V27;
this includes runtime overhead and is not shader time or a model TTFT gain.
Archive `products/v27-native-r1.tar.gz`, SHA256
`32f5972eb1274bc0d3825e52e7c0cf65a09249f71f3aa36c44ce68c71223a32a`.
All three completed GPU stages were removed after verified local retention.

V28's staging-only retry passes all 1,024 complete output IDs and decoded bytes.
Both arms use the same controller and historical worker, one excluded warmup
and three measured requests, in fixed baseline then parallel-copy order:

| Arm | Mean TTFT (ms) | Mean TPOT (ms) | Measured-ingress output tokens/s |
| --- | ---: | ---: | ---: |
| Baseline | 2312.564 | 85.150 | 9.750891 |
| Parallel prefill copy | 808.974 | 85.154 | 11.011655 |

TTFT falls 65.0183%; TPOT is effectively unchanged. This is native screening,
not HTTP, sustained throughput or a new vendor pair. It must not be multiplied
by the independent decode-composition result. Both arms keep 135 batches and
83,139 packets per request. Legacy inner arm teardown used owned signals;
the outer supervisor exited cleanly without signals. Report SHA256 is
`fd58ba7aa82f7bc7a18868e9837bf0de2929910ef519d74b8302a0b2d68ead17`;
full archive `products/ferric-v28-native-r4.tar.gz` has SHA256
`039561bd6c792e3e57c7685a65c27e787905aa7f8579850254852f7c17e6fee5`.

[Repeated-start tooling](../adapters/m1-engineering-execution-v1/tools/compare_paired_starts.md)
requires matching frozen profiles, chronological alternating engine order and
every declared attempted pair. It recomputes metrics and checks retained
outputs/receipts. It reports descriptive spread, not confidence intervals or
qualification. Complete launch validation remains with the frozen per-pair
replayer; caller roster completeness and fresh starts need external records.

## Focused Validation

An independent CPU-only scratch directory on mi300x-2 used the unchanged owner
guard plus a tighter 64 MiB cap/8 MiB reserve. It did not use or expand the
compiler cache. All three phases returned zero with clean, signal-free owned
process teardown:

- Rust formatting of the ten changed adapter files, without building them.
- Twenty comparison-tool unit tests, all passing.
- The same twenty tests plus a real retained-V5-pair replay, all 21 passing.

The real replay reproduces the old measured values and is correctly classified
as limited single-pair screening. It is not another benchmark run. Replay SHA256:
`b8802719f73e9ace56bb9831bd3a27ad04e52345d69d697f2b68775884b9f974`.
The local checksum-verified validation archive is
`/home/harsh/.codex-tmp/ferric-perf-swarm-v6/validation-evidence.tar.gz`, SHA256
`62e201edaf432fe2621067384cc2009fd09112a2bf0254d65fc018f2c8984cd8`.
The original raw V5 archive remains retained separately. The new remote scratch
was removed after checksum-verified retention and clean owned-process checks;
older protected stages and unrelated worktrees are untouched. No new worktree
was created.

The first new source-policy run retained 39 passes and one failure: its lexical
assertion still expected the old `pending.len() != 8` spelling. The corrected
test checks both the unpacked eight-producer prefix and the packed schedule,
layer and collective frontier before publication. The original file, inventory
and failure remain retained. Only that test file changed in the remote source
revision; all nine adapter phases passed under distinct `-r2` receipts.

The existing V27 parallel full-page prefill-copy candidate now passes five
contract tests, six host tests and strict Clippy against d10. Its successor
recipe and simulated-worker harness pass 10 and 26 tests respectively. Emission
and native parity now pass. A narrow, independently reviewed V28 integration
patch is applied privately; full-model parity and screening now pass as above.
Its first CPU recipe tests exposed shared fixture aliases, while preparation
passed the dependency audit before a broad formatting command encountered an
unrelated historical missing module. Both failures are retained; the successor
separates fixture copies and formats only touched Rust files. All 12 successor
recipe tests, preparation and the default feature-disabled check pass. The
remote formatting and controlled lock delta have been integrated locally.
The first focused compile then failed before tests with 254 duplicate compiler
diagnostic items: the adapter build script linked V21's c4c and V27's d10
device SDKs in one Rust unit. The integrated correction gives V27's typed roster
its own build-script process and exposes only generated string constants to
the adapter, leaving kernel source and image pins unchanged. The failed R2
run has clean teardown and is retained as `products/v28-cpu-failure-r2.tar.gz`,
SHA256 `78a683922f9ad068eb77cd5787e825fe41191a6eb4e6722621d74373354eb040`.
The R3 successor passes all 26 selected phases: 20 recipe tests, dependency
preparation, the default check, all five focused and five CLI cohorts, 41
source-policy tests, 416 library tests (10 ignored and one known high-memory
exclusion), strict Clippy, both actual image gates and the release build.
The first policy compilation timed out with forced cleanup marked unverified;
its original failure remains distinct from the accepted clean second attempt.
Its prepared lock SHA256 is
`37e5644e07d7b128827cd7e327f920a24a08a141b620ed0d418d8a79ddabcd3a`.
The 19-test native harness and build collector pass. Controller SHA256 is
`3ea106b88e1e77c48ff9d32aa988ecd0554d9b6d5bc1d7eb62b5e4caeff0914c`;
build receipt SHA256 is
`03a9b32e4d9cff83b20968334f028f6600125d5d436bbca4476f35e40be3f02f`.
The first native launch rejected the flat V27 image path before setup with
`ContentDirectoryIdentity`. The adapter correctly requires the namespace and
content-ID directory. A staging-only retry preserves the executable, image
bytes and helpers; no adapter check is weakened. The failed launch has clean
outer teardown; its owned arm cleanup used signals. The successful retry is
recorded separately above. Full CPU evidence, including the failed first policy
compile, is retained in `products/v28-cpu-evidence-r3.tar.gz`, SHA256
`da63e574e80aff16b9d3c2f5c3756016fccd0a35e9d71df8facbd83f0fe6b4da`.
The
separate V20 loop-bound refinement is also a source-only proposal; it preserves
all active loads and arithmetic while skipping fully inactive groups.

The reviewed model timestamp sidecar is integrated privately behind a dedicated
optional feature and diagnostic executable. Its KFD dependency now pins latest
main `5e2f668d0e91815f6c7aafe18dbdf283ab0394aa` without changing ordinary
serving defaults. Original command boundaries
and groups of at most 16 remain intact; semantic operation tags accompany each
packet. Its first planned capture is one cold complete 128/128 request per
fresh worker, with no warmup/reset, not a hot-serving measurement. All ten fresh
latest-worker CPU phases pass, including 518 library tests and strict Clippy.
A new same-worker native smoke passes 210 outputs and 528 guards with no cleanup
signals; raw ticks remain diagnostic, not qualified shader durations. Its full
archive `products/ferric-timestamps-5e2-r1.tar.gz` has SHA256
`92c09beda62cf10814fc5315d3a24fc98df1a475fc0a16c3351396833ee1f601`.
The model controller's first compile found a test-only u32/usize mismatch; that
failed attempt is retained. A checked conversion fixes the fixture. R2 passes
its first 13 phases, including 443 library tests (10 ignored, one known
high-memory exclusion), the collector/driver tests and both CLI cohorts.
The source-policy compile then exposed a real cross-binary feature mismatch:
the shared worker selected latest KFD wire types while the peer transport
retained its historical types. That failed compile exited cleanly and is
retained in `products/model-cpu-failure-r2.tar.gz`, SHA256
`eab5f6c705efaa5a0dc82c39fedfedea21b6857af08a124d7b711b09de9bf835`.
An independently reviewed four-file correction moves unchanged packing helpers
into one source included under each transport's own wire namespace. It does not
change the peer protocol, dependencies or wire serialization. That corrected
diagnostic-only R4 snapshot excludes the separately integrated composition;
its subsequent validation history is recorded above. Successors check all
model-feature binaries and strict Clippy early. The latest Model R6 passes
all 23 gates and retains both controllers; the R8 native harness passes 26
remote CPU tests. The supervised model campaign accepts all four cold arms,
512 exact output IDs/bytes and both complete raw-record captures. Raw ticks
remain diagnostic, not nanoseconds or qualified shader-only durations.

The paired-start aggregator now admits only the matching V3 summary/receipt
schema pair for the composed candidate. All 24 remote tests pass, including
mixed-version rejection, whole-series rejection after selector/image drift,
and retention of failed attempts. This is tooling validation, not a new pair.
Completed V28 failed/successful and latest-worker smoke GPU stages were removed
after checksum-verified local retention. The obsolete duplicate 278,254,354-byte
executable archive was also removed under the CPU owner lock; the active cache
and evidence rosters were left intact.

The three-file Pages source update includes the composition, runtime and V27
results while keeping the matched HTTP loss first. Remote checks and screenshots
passed, including 18 additional visually reviewed captures. Static-only commit
`5a662b7661e6325acab471a523994a815c29b0ae` deployed successfully through run
`35539388416`, with no GitHub build or test. The live index SHA256 matches the
tested bytes: `29a3813418e92d789c54318effc18b96f274ebc4023571a7b7403f2618fee2d9`.
The full retained Pages evidence archive has SHA256
`0253cba8b5a06ea88607fa815a50c892b2a37db57a9f19230b96506f5ae733db`.
An initial artifact-staging basename error is retained separately from the
successful successor; it was not relabeled as a pass.

## Admission Snapshot

- Builds/tests stay on mi300x-2 (`sharkmi300x-3`, UID 1046); no local builds.
- GPU/model execution stays on mi350 (`smci350-rck-g03-b19-03`, UID 9661).
- Remote fe2o3 main is `c60cd746e63b34b9072a493744d73b87ed1defc3` at this
  check. Historical worker/images keep their original identities; current
  main does not retroactively describe old binaries.
- Completed executable products were checksum-retained before deleting the
  disposable Cargo cache. The 5,603-file nightly archive was restored and
  rehashed under a corrected 1,809,220,877-byte total. A failed first restore
  and its receipt remain documented; the verified partial was then retired.
  Latest compiler-owner allocation before V28 staging was about 6.22 GB.
  These are snapshots, not reservations.
- The user approved a private compiler-stage increase to 12 GiB. A new
  guard copy changes only that cap; the 512 MiB reserve, host free-space
  floors, one CPU, 4 GiB RSS and 1,200-second phase limits are unchanged.
  Original guards and historical receipts remain untouched.
- A later mi350 census briefly found another Ferric TP worker after the first
  idle check. It was left untouched and subsequently exited. A historical
  census at 21:45 UTC found a new unrelated TP2 worker on GPU0 and GPU1.
  It was left untouched. Later fresh global-idle admission passed for V28 and
  the latest-worker smoke; neither observation reserves future capacity.
  Recheck before each launch.
- The user later authorized stopping only the specific
  `ferric42-tp2-closed-token` job on mi350. The integration lead found no active
  matching process or container; all eight GPUs were idle and no KFD holders
  were found. No signals were sent. This fresh snapshot does not reserve GPUs
  or authorize unrelated job cleanup.
- The first composition admission found a newly appeared foreign GPU worker
  and refused before spawning a benchmark child. Its failed receipt and full
  stage are retained in `composition-admission-r2.tar.gz`; no process was
  signaled. The owned remote stage was removed after retention. A fresh retry
  stage was admitted and all four arms passed. The accepted native screening
  values above do not replace the retained matched HTTP loss.
- No new worktree is needed. Unrelated dirty trees remain untouched.

## Critical Path

1. Preserve the fully retained incomplete HTTP R3 series and rejected
   aggregate; the seven retention tests pass and both remote stages remain. Keep the
   failed third start and unlaunched third vendor distinct; do not rerun
   unchanged software merely to replace the failure.
2. Analyze the complete Model R8 operation-attributed raw intervals. Qualify
   timestamps separately before claiming kernel durations;
   instrumented timings never replace uninstrumented serving data.
3. Investigate the higher combined native TPOT in this cohort using all
   retained samples and unchanged routes; persistence and cause are not established.
4. Use measured attribution to prioritize prefill attention/tiling and the
   already prepared fusion candidates. Do not accumulate more unvalidated
   kernels while emission is blocked.
5. After meaningful optimization, run a wholly fresh repeated HTTP campaign
   with new owner, plan, six authorizations and AB/BA/AB roster. Preserve failed
   attempts and distinguish exploratory screening from qualification.
6. Expand to concurrency/context and latency-constrained goodput, then compare
   DP8 and TP8 with equal GPU budgets. Speculation is a separate later track;
   symmetric memory remains deferred.

Core compiler/runtime changes belong in fe2o3; kernels, inference and
benchmark tooling belong in Ferric. Core publication requires a fresh rebase
onto main, successful relevant validation and skipped GitHub CI. Ferric
implementation stays private. Site updates report accepted evidence only.
