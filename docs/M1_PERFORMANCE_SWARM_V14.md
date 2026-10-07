# Performance Swarm V14

Updated October 6, 2026 (PDT). The latest complete Width55c matched HTTP comparison
measures Ferric 22.7233x slower in median TTFT and 12.2045x slower in median TPOT
than vLLM. Ferric has not beaten vLLM. Current-runtime prefill32 improves median
native TTFT by 48.1458% versus prefill16; its fresh HTTP result is 431.032717 ms
TTFT and 53.118820 ms TPOT. Earlier V17 a004 improves median native TTFT by 10.9687%
within the native backend; TPOT is effectively unchanged. V16 boundary fences
improve median TPOT by 1.0693% but miss the preregistered 5% gate. V14 a005
regresses. No default promotion, vendor win or serving qualification is claimed.
All 33 M1 gates remain open.

## Latest Results

### Width55c Matched HTTP Comparison

Both engines complete the same plan sequentially on the same MI350: Qwen3-8B,
TP1/C1, 128 input and 128 output tokens, context 8192, greedy, BF16 decoder and
explicit FP32 head computation, prefix caching and speculation off. Each arm
has 30 measured requests after ten excluded warmups and two untimed diagnostics.

| Engine | Median TTFT (ms) | Median TPOT (ms) | Finite Output Rate (tok/s) |
| --- | ---: | ---: | ---: |
| Ferric native prefill32, current55c | 431.032717 | 53.118820 | 17.901874 |
| vLLM 0.28.0 | 18.968767 | 4.352392 | 223.767613 |

Ferric/vLLM median ratios are **22.7233x TTFT** and **12.2045x TPOT**.
Both complete archives pass independent raw timing, parity, ownership and clean
unsignaled teardown review. All 42 Ferric requests match 5,376 exact output IDs;
vLLM's exact IDs are limited to its two untimed diagnostics. Both engines' timed
and warmup streams match exact UTF-8 and usage. All 116 Ferric and 31 vLLM GPU
samples pass; two rejected startup attempts remain retained with exact container
birth and proven non-init departure evidence under the frozen startup policy.

This is one sequential cohort per engine, not cross-engine ABBA. It confirms the
prefill benefit at HTTP level, but does not isolate the change from historical
R6: compiler/runtime and prefill selection differ. Decode is unchanged and the
HTTP measurements show no meaningful TPOT gain. Clock/power/throttle equivalence
and continuous isolation are not established. SSE-derived TPOT is not per-token
ITL, and finite closed-loop output rate is not sustained loaded throughput.
The [independent pair review](performance/width55c-http-a001-independent-review.json)
binds both full archives and all arithmetic. No default promotion or vendor win.

### Native Prefill Width32

Attempt `a003` completes both mechanism cells and three uninstrumented ABBA
blocks on current `55c1a9b6`, with 24 timed requests per arm after two excluded
warmups per cell. Complete replay and independent archive audit verify all
14 clean, reaped, unsignaled closures and 9,472 exact output token IDs/bytes.

| Arm | Median TTFT (ms) | Median TPOT (ms) | Finite Output Rate (tok/s) |
| --- | ---: | ---: | ---: |
| A: prefill16, native613 | 772.737844 | 52.912262 | 17.553749 |
| B: prefill32, native649 | 400.696764 | 47.786564 | 19.456638 |

Median TTFT is **48.1458% lower**; all six adjacent pairs improve by
40.9855..48.8456%. Observed median TPOT is 9.6872% lower, but decode remains the
same 652-command program and TPOT p95 rises 0.8392%, from 53.651083 to 54.101318 ms.
Do not attribute the aggregate TPOT change to a new decode kernel. Finite-window
output rate rises 10.8404%. All fixed engineering gates pass; the promotion scope
is prefill width within the same native backend, not an ordered64 comparison,
production/default promotion or a vendor win.

The [independent review](performance/native-width-a003-independent-review.json)
checks all 2,773 retained files, 200 plan inputs, 37 source pins, raw timing and
counter arithmetic. The complete 75,869,812-byte archive SHA256 is
`524395c7360154a05e16fb286d78fc3c751b262a2b5a0390753857a665aba33d`;
report SHA256 is `3b7168cf632793b9ec97fc06b85472d652e46c58d32879ae95cdfbcde147a3dd`.
Failed `a001`/`a002` attempts remain separately retained and are not pooled.
The successor matched HTTP comparison above reuses R6's vendor image, precision
policy, request client and workload settings with a new shared plan.

The integration checkout now contains the exact measured native-prefill32 source:
15 existing files replaced and nine added from the final qualified archive
`6089abea6ebc3363557c8e5fb8b4e2e488fa49b4f30f99029dfcc16b7c5bddd7`.
All 210 adapter non-tool files and 29 local dependency trees match; newer
benchmark lifecycle tools and unrelated dirty files are preserved. The prior
15 files remain in a separate preimage archive. This is byte-level integration
of already qualified code, not a fresh build, test pass or new speedup.
See the [overlay record](performance/native32-measured-source-overlay-a001.json).

### Current-Runtime Kernel Attribution

The V19 packet diagnostic completes one exact 128-input/128-output request on
`55c1a9b6`, using the same nine-image composition as the measured model, but
ordinary ordered64 submission rather than R6's native whole-program schedule.
Independent raw review checks all 394 retained files, 216 plan inputs, 48 source
pins, 128 output IDs/bytes, 87,711 packet records and 43 recomputed summaries.
Both child and outer teardown are clean and unsignaled.

| Decode Operation | Calls Per Token | Median Raw Packet Interval (ticks) |
| --- | ---: | ---: |
| Down projection | 36 | 22,664 |
| Up projection | 36 | 15,912 |
| Gate projection | 36 | 15,772 |
| Attention output projection | 36 | 10,888 |
| Input RMSNorm | 36 | 6,868 |
| Post-attention RMSNorm | 36 | 6,824 |
| Output head | 1 | 87,264 |
| Argmax | 1 | 102,428 |

These are instrumented packet intervals with unspecified clock frequency, not
calibrated nanoseconds, shader-only durations or additive wall-time shares.
The repeated projection intervals support prioritizing full-model split-K and
gate/up work; they do not predict its end-to-end speedup. Prefill down projection
has a median 58,728 ticks across 288 calls in this prefill16 diagnostic.

Source review confirms two-byte BF16 decoder weights and activations, not blanket
FP32 expansion. Original and KxN layouts duplicate residency; each dispatch reads
one selected layout. The logical unique projection-weight volume per decode is
15,136,194,560 bytes including the output head, not a measured HBM-traffic count.
The current C1 gate/up/down path uses scalar Wave BF16 loads and dependent FP32
accumulation; split-K instead uses existing BF16 MFMA and an FP32 merge. Their
combined diagnostic interval-sum proxy is 38.069%, versus 1.993% for argmax.
These are nonadditive raw-tick proxies, not Amdahl bounds or TPOT predictions.
No occupancy bottleneck is established by this source inspection.

The original standalone summary command fails because its replay helper still
expects an obsolete plan-schema name. The failure remains archived. A separately
qualified, one-literal offline correction passes 15 fixtures and replays the
retained capture without mutating the original stage or rerunning the GPU
workload. All 43 summaries match the independent raw audit. The accepted replay
report SHA256 is `3620454cd201de285d7255401760dc3a8dd3b0157205c9f430251fa4a9342c90`;
its custody record is `33d7b8d92743df4dd554a35cb34ce92525ebf2e1ba5f6ef547c29a9db43072e4`.
Neither report admits a latency or end-to-end performance claim.
The [independent raw review](performance/packet-v19-55c-a001-independent-review.json)
binds the complete 98,543,608-byte archive at SHA256
`f1a8a30a7ebc91f6a2a83d5dc894173d91a24923c936b14f26886fcbb6706f0e`.

### Ordered Down-Projection Component

The current `55c1a9b6` runtime completes the uninstrumented latency and separate
counter cohorts with exact fixture parity and clean, unsignaled teardown. Each
arm uses one ordered submission, including both split-K partial and merge.
Three ABBA blocks provide six adjacent control/candidate pairs per comparison.

| Compared Arms | Control Host Median (ms) | Split-K Host Median (ms) | Reduction |
| --- | ---: | ---: | ---: |
| Wave / split-K | 0.362390 | 0.273720 | 24.468% |
| Unsplit MFMA / split-K | 0.666886 | 0.275690 | 58.660% |

All six pairs favor split-K in each comparison. These are synchronous host-call
intervals on one reused C1 down-projection fixture (N4096/K12288), not GPU-only
time, model-streaming bandwidth, TTFT or TPOT. Worker-response medians are
0.258170/0.156400 ms for Wave/split-K and 0.559546/0.157040 ms for MFMA/split-K;
they include publication, completion and exit fences. Counter replay confirms
one frame with one/one/two dispatches, no timed reads/writes or new admissions,
and no post-warmup full currentness checks. Existing images retain their older
compiler provenance; the current runtime does not imply current re-emission.

The timestamp cohort's child completes, but its outer postflight fails the
unchanged root-disk floor. That cohort is quarantined, with no admitted tick
aggregate or silent retry. The [independent raw review](performance/ordered-component-a001-independent-review.json)
binds all 127 archived files, complete source manifests, raw parity and paired
arithmetic. Full-model integration and latency measurement remain required;
there is no default promotion or new vLLM win.

The separate full-model candidate reuses authenticated down-projection weights,
adds 128 KiB of private scratch in both arms, and changes only decode down
projection from one Wave kernel to split-K partial plus merge. Its selected
CPU scope passes 91 tests, actual-image ABI admission and release compilation
on current `55c1a9b6`. Strict Clippy fails on documentation formatting and an
allocation in error teardown, leaving subsequent binary lint coverage incomplete.
The exact ELF `a8ab66586d6fde3ff15642796f0614378f9f3a8c8b1dbb4b8bf02c1ed7157a30`
is therefore experimental only, with the failure and seven deferred matrix roles
explicitly retained. No numerical, isolation, cleanup or performance gate is
relaxed. A separate
source-only lint repair is not silently substituted into this artifact.
The final measurement harness passes 274 remote tests and actual artifact/CPU
binding with unchanged source. Earlier malformed-evidence and fixture failures
remain separate.

Native attempt `a001` completes both correctness/graph cells, with 128 exact
output token IDs per arm and the expected 87,711 baseline versus 92,283 split-K
dispatches. The first baseline latency cell also completes, but the first timed
candidate cell stops before setup or requests when shared root-disk free space
crosses the unchanged 64 GiB floor. Its outer receipt is rejected with failed
resource postflight; its inner cleanup reaps the signaled controller and proves
owned descendants absent. The inner receipt records both TERM and KILL, whereas
the outer receipt records TERM only. These distinct receipts remain unchanged.
There is no complete A/B block, latency report or admitted model speedup.

The interrupted archive is 46,392,174 bytes at SHA256
`f13ffb25a23057762a70345938545105a3eae9a56e25d8eba5984f89bdd38f8b`.
Its retention receipt binds all 789 regular files and preserves the failed cell.
The [independent review](performance/splitk-model-interrupted-a001-independent-review.json)
also checks all 168 plan inputs, eight completed requests, 1,024 exact token IDs
and streamed UTF-8. This custody does not admit interrupted A/B timing.
Only exact old, unreferenced pip/npm cache files were subsequently removed:
1,609 files totaling 715,366,400 allocated bytes. Foreign work, model data,
source and active build caches remain untouched. The recovered headroom does
not establish a stable window for another complete campaign.

Successor `a002` completes both correctness cells and two full ABBA blocks:
50 total requests, 6,400 exact output IDs and 32 timed requests. Independent
raw replay gives provisional median TPOT **53.133042 -> 49.222673 ms (7.3596%
lower)** and TTFT **810.968878 -> 812.045854 ms (0.1328% higher)**. Three of
four adjacent TPOT pairs improve. This is the ordinary ordered64/prefill16
backend, not the native-prefill32 HTTP comparison above.

The third block stops before setup when foreign KFD PID 573196 appears. The
failed outer postflight and signaled inner teardown remain rejected. No complete
campaign gate or model-speedup qualification is claimed. The entire 2,120-file
stage passes [independent raw review](performance/splitk-model-interrupted-a002-independent-review.json)
and is retained in the 60,387,092-byte archive
`9351d1ca436a8218e14da499aea0b655d5a7b5387e5713c1b7e7eb5508c65fe1`.
Fresh unchanged campaign `a003` also stops during the third block. Eleven cells
complete with clean unsignaled teardown, 56 requests and 7,168 exact output IDs.
Its first two complete ABBA blocks give provisional median TPOT
**53.026398 -> 49.372703 ms (6.8903% lower)** and TTFT
**811.501693 -> 810.162473 ms (0.1650% lower)**. All four adjacent TPOT pairs
improve, but the second block has substantial latency variation. The extra
unpaired `block3-A1` cell is excluded from these estimates.

`block3-B1` stops before Setup or requests. A complete root-visible scan finds
previously unbound GPU owner PID 773727. The startup monitor's attempt to classify
that newcomer rejects its executable inode; this does not show that an already
admitted Ferric worker changed identity. The capture does not retain the
newcomer's executable or command line. The failed outer postflight and signaled
inner teardown remain rejected. All 2,258 regular files are retained in archive
`27c05ff20d0353097bc3d93da20261edd2f5d35053d8da9620aacf169c63a188`
and pass [independent raw review](performance/splitk-model-interrupted-a003-independent-review.json).
No complete campaign gate, promotion or HTTP speedup is claimed, and interrupted
attempts are not pooled.

### M32 Gate/Up Prefill Prototype

The new default-off Ferric crate computes two M16 gate/up tiles per Wave64
workgroup for exactly 32 rows, N12288, K4096 and TP1. It halves the grid from
1,536 to 768 workgroups, retaining ascending K16 FP32 accumulation and final
BF16 rounding. Two adjacent typed output views cover the existing allocation;
there is no extra weight layout, scratch allocation, output copy or merge.
Separately owned B fragments respect the SDK's consuming API. The inspection
below resolves B-load sharing and static register counts; the native component
comparison below establishes no useful timing gain.

Remote qualification against SDK `55c1a9b6` passes all 24 host/source tests in
both default and opt-in profiles, plus strict all-target Clippy in both profiles.
The original cold-test phase remains rejected because dependency growth exceeded
its declared 512 MiB increment, despite passing its tests. Distinct warm phases
pass without changing the frozen source or relabeling that failure.
The source-roster receipt is
`80e4548467b717161e97b9213018be9770ce35f2a9bf3a474405a21b5d9e9ede`.

These phases use the explicitly declared G40 build profile: a 40 GiB private
stage, 512 MiB reserve, four cores, 8 GiB RSS, nice 19 and 20-minute per-phase
timeout, with existing host-space and memory floors unchanged. Fresh vendor
qualification and standard one-root engineering emission pass with clean,
unsignaled outer teardown. The emitted image is 10,968 bytes at SHA256
`bb164d23d55dec43e55b8d8cce936929a3cc749a29486bdcfff8bfe34ac7d5dc`.
Inspection confirms the expected 344-byte COV6 kernarg segment, alignment 8,
88-byte padded explicit area, Wave64, zero private/LDS bytes and zero reported
spills. Metadata reports 50 SGPRs, 44 VGPRs and 8 AGPRs; these are not an occupancy
measurement. Diagnostic capture is explicitly `omitted-ineligible`, not a
retained KIR/LLVM trace or formal proof.

Crucially, actual ISA still issues two copies of each of the four B loads per
iteration. The two MFMA operations consume separate packed B tuples, not one
shared register tuple. Logical issued B-load volume remains 192 MiB, equal to
the existing V5 control for this shape; this is not measured HBM traffic.
The prototype therefore tests dual accumulators and fewer workgroups, not an
established weight-bandwidth reduction. Native parity passes, but component
timing does not justify client integration. Explicit B-operand reuse would
require a separate fe2o3 SDK/compiler capability while retaining fragment
ownership and wave-provenance checks. No such core change is implemented yet.
The compiler executable retains its historical 79a plus
published fixes identity; using the current SDK does not relabel that binary as
a complete current55c rebuild. No serving default or performance claim changes.

A subsequent [scoped compiler-source audit](performance/compiler-55c-scoped-source-equivalence.json)
verifies all 10,294 retained emitter files and the 10,347-file current55c source
roster against immutable Git. Active compiler sources match the published
`510ec3b` fixes; the only later changes are 13 KFD files behind the default-off
engineering feature, which the recorded compiler build did not enable. No
stale compiler functionality is identified for that configuration. This does
not establish identical complete source trees, bit-identical binaries or a
fresh55c compiler build; historical compiler identity remains explicit.

The component fixture, adjacent-slice encoder and driver pass 15 remote CPU
tests under G40 with unchanged five-file source and clean outer teardown.
The exact dyadic fixture avoids tolerance-dependent parity; both gate/up tags
are correctness warmups, followed by three gate-only ABBA timing blocks.
Output resets and complete readbacks remain outside the dispatch interval.
Inner receipt `922a6e24ad155d6cbae46ef16f8b43fb4a9c3d23da1c9b36beb1079c8ef50ed0`
and outer receipt `d40148cdf6663a69db3c664949e3331e13c99701ebd98a0663813dbe11078cd1`
prove synthetic worker/session protocol tests, not actual image loading,
native ABI admission or model performance. The complete native harness then
passes 71 remote CPU tests with unchanged 28-file source, actual-artifact binding
and portable export. The native run completes all 16 cells with clean, unsignaled
teardown. Independent archive inspection verifies 136 regular files, all 91 plan
inputs, 137 command/response pairs and all 16 guarded output hashes against a
separately reconstructed exact dyadic reference. Inputs and guards are unchanged.

Across three ABBA blocks, V5 versus M32 host medians are 0.469880 versus
0.472145 ms, a **0.4820% regression**. Ordered-worker elapsed medians are
0.360295 versus 0.3607755 ms, a **0.1334% regression**. Only one of six adjacent
pairs improves in either metric; both order medians regress. No outlier is
discarded. These hot-buffer durations include dispatch/queue overhead and do
not establish shader time, model TTFT/TPOT or a bandwidth improvement. V5 and
M32 have distinct retained compiler histories, so the comparison cannot isolate
M-tiling alone. The prototype remains default-off with no serving integration.

The complete archive is 58,477,195 bytes at SHA256
`3fe1c6e3c19dbc4d4d534a6365f1610d8bddb9b1e8e14d5a8d0d5dbcb0e12cde`.
See the [independent native review](performance/prefill-m32-native-a001-independent-review.json).

### Split-K4 Gate/Up Decode Prototype

The next default-off Ferric crate, `qwen3-tp-c1-splitk-gate-up-kernels-v1`,
implements C1/TP1 gate and up projections for N12288/K4096. Four K1024 partitions
use 3,072 Wave64 MFMA workgroups and a separate 192-workgroup merge. The merge
adds FP32 partials in increasing partition order, then rounds once to BF16.
One 192 KiB scratch can be reused sequentially, and the current MFMA-prefill
client already retains authenticated KN weights. No extra weight allocation,
serving selector, default change or fe2o3 core modification is included.

This is not simply an occupancy fix: the existing Wave projection already has
12,288 workgroups. The proposed difference is sixteen-column MFMA computation
and partitioning, with only one useful MFMA output row and one extra dispatch
per projection. Reduction association changes, so native numerical checks and
real-model token parity remain required. Source review and remote host
qualification pass: 16 default tests, 17 opt-in tests and strict all-target
Clippy in both profiles. The [independent CPU review](performance/c1-splitk4-gate-up-host-a001-independent-review.json)
verifies all five clean phases and unchanged source against SDK `55c1a9b6`.
Emission and independent static ELF/ISA review now pass. The two-root image is
13,136 bytes at SHA256
`d28610d291eeec0589afbf269e26d21b7111c96f08106e1f661d6a66f024bf03`.
The [independent static review](performance/c1-splitk4-gate-up-isa-a001-independent-review.json)
finds partial/merge kernarg sizes of 328/288 bytes; register counts are respectively
36/28/4 and 25/9/0 SGPR/VGPR/AGPR, with no spills, private memory or LDS.
The complete handoff, KIR and pre-worker LLVM capture remains diagnostic-only.
The new guarded component harness passes 80 remote CPU tests, including
three-root ABI encoding, ordered partial-to-merge submission, poisoned scratch,
untouched output tails and failure handling. Actual-artifact binding also passes
[independent review](performance/c1-splitk4-component-cpu-a002-independent-review.json)
of all 98 portable files. The native latency run now completes with clean,
unsignaled teardown. Independent raw review verifies all 149 retained files,
16 guarded outputs, all partials and unchanged inputs against a separately
reconstructed fixture. Three ABBA blocks provide six timed samples per arm.
Host-call median falls **0.3616855 -> 0.2764455 ms (23.5674% lower)**, with five
of six adjacent pairs faster. Worker-response median falls
**0.258195 -> 0.1570805 ms (39.1621% lower)**, with all six pairs faster.
Candidate timing includes partial plus merge in one ordered frame.

These are hot-buffer component intervals, not shader time or model TPOT. One
Wave sample is near the candidate cluster; timing/wait aliasing is possible,
not established. Control and candidate compiler identities differ. No Qwen
token parity, model gain or serving integration is claimed. See the
[independent raw review](performance/c1-splitk4-component-native-a001-independent-review.json).
Historical packed gate/up and down-projection results do not qualify this image.

True B sharing for the M32 prefill experiment is a separate core capability,
not another source-only tiling change. The source investigation proposes a
private linear accumulator-pair API that preserves both typed chains through
loops and lowers two existing MFMAs with identical B values. It requires SDK,
provider authentication and semantic transport changes; it is not implemented.
Even then, optimized IR and ISA must demonstrate shared B loads/registers.

### Paired K16 Prefill Qualification

The full 15-root V5 `prefill-mfma-k2` profile is not emission-qualified on the
current SDK. An exact array-to-scalar source repair passes 26 default and 26
enabled tests plus strict all-target Clippy in both configurations. It clears
the original private-array witness refusal, but the next emission stops at
the FP32 partial root's race-analysis peak-storage bound. The failure retains
251 ranked blocks and clean, unsignaled teardown; host RSS and stage limits
were not exceeded. It does not establish an actual race or a GPU-storage error.
The [independent review](performance/prefill-k2-current-host-a002-independent-review.json)
retains both the CPU pass and emission refusal. No verifier bound was relaxed,
no root omitted from that attempted V5 image, and no device speedup is claimed.

A separately declared, default-off `qwen3-tp-prefill-k2-kernels-v1` experiment
now contains two BF16 roots: an unchanged-load control and a paired-load
candidate with the same guards, shapes and arithmetic order. Both are compiled
in one image so the comparison holds compiler provenance constant.
This is a new two-root contract, not a passing full-K2 artifact or a change to
the existing V5 profiles. Remote qualification passes seven default and eight
enabled tests plus strict all-target Clippy in both configurations. Standard
emission and independent ABI/ISA inspection pass with current55c SDK and the
explicitly retained `79a` compiler plus published private-access/witness fixes;
this is not a newly rebuilt current-main compiler.

The 23,272-byte image contains both roots. Each has a 328-byte kernarg, Wave64,
and zero LDS, private memory and spills. Control/paired register counts are
50/52 SGPR, 48/68 VGPR and four AGPR. Actual ISA groups eight scalar ushort
loads and one MFMA per K16 step versus sixteen loads and two dependent MFMAs
per K32 step. Total loads and arithmetic do not decrease; no cross-iteration
pipeline or B sharing is established. Higher register use is a measured-code
tradeoff, not evidence of a speedup.

A same-image component successor passes all 76 fresh remote harness tests.
Both arms use three full tensor views, identical 1,536-workgroup geometry,
shared allocations, output poisoning/guards, and three ABBA blocks. The first
harness attempt's stale unmatched-compiler negative fixture failed and remains
retained; the corrected successor passes without relaxing production checks.
Native timing now passes all 16 calls, full dyadic output parity and guards,
with clean unsignaled teardown. Across three ABBA blocks, host median changes
from 0.4697255 to 0.468805 ms (0.1960% lower), with only two of six adjacent
pairs faster. Worker-response median changes from 0.361335 to 0.3603405 ms
(0.2752% lower), with four of six pairs faster. This is effectively flat;
the variant stays disabled and no model TTFT/TPOT gain is credited.
The [independent native review](performance/prefill-k16-component-native-a001-independent-review.json)
retains the complete source, fixture, protocol and lifecycle checks.

A separate fixed timestamp-mode successor passes all 77 remote CPU tests.
All 16 native profiled calls now pass, including exact outputs, guards, raw
timestamp identities and clean teardown. Timed control/paired median intervals
are 24,796/21,756 raw ticks, a 12.2600% decrease; all six adjacent pairs improve
by 11.15-12.88%. This is a packet-level improvement in a hot-buffer diagnostic,
not calibrated nanoseconds, shader-only time, B-register reuse or model
performance. Profiled host and worker wall medians remain essentially flat.
Do not pool this cohort with the uninstrumented latency run.
The [timestamp audit](performance/prefill-k16ticks-component-native-a001-independent-review.json)
retains complete raw replay. This warrants a separately measured native-program
composition, not default promotion or a TTFT claim.

### Paired Loads Within Split-K

The separate paired-load image passes all 16 guarded component cells and clean
unsignaled teardown on current55c. Independent audit verifies 590 retained files,
280 inputs, 80 qualifying fixtures and all 32 partial/merge dispatch packets.
Across three ABBA blocks, host-call median changes from 0.3757105 to 0.3753855 ms
(0.0865% lower), with only three of six pairs faster. Worker-response median
changes from 0.259035 to 0.257875 ms (0.4478% lower), with four of six pairs faster.
This does not establish a useful repeatable gain. The paired variant stays off;
no follow-up instrumented cohort or full-model composition is justified by this
result. These are reused-buffer component intervals, not model TTFT/TPOT or
shader-only durations. The [independent review](performance/paired-ordered-component-a001-independent-review.json)
binds complete source, CPU, numerical, lifecycle and timing evidence. Full-model
unpaired split-K remains a separate pending experiment.

### Historical V18 HTTP R6 Matched Comparison

Both engines complete the same R6 plan sequentially on the same MI350: Qwen3-8B,
TP1/C1, 128 input and 128 output tokens, context 8192, greedy, BF16 decoder and
explicit FP32 head computation, prefix caching and speculation off. Each arm
has 30 measured requests after ten excluded warmups and two untimed diagnostics.

| Engine | Median TTFT (ms) | Median TPOT (ms) | Finite Output Rate (tok/s) |
| --- | ---: | ---: | ---: |
| Ferric standalone V17 B, prefill16 | 783.766966 | 53.399713 | 16.932225 |
| vLLM 0.28.0 | 19.322447 | 4.397164 | 221.379677 |

Ferric/vLLM median ratios are **40.5625x TTFT** and **12.1441x TPOT**.
Both clean, unsignaled receipt chains and all retained raw hashes pass
independent review. Ferric's 42 internal requests / 5,376 token IDs match the
reference. vLLM's 40 timed/warmup streams match exact UTF-8 and usage; token-ID
parity is checked only in its two untimed diagnostics. All 123 Ferric and 30
vLLM sampled GPU observations are accepted, with retained bounded reconciliation
attempts. The exact plan, client, workload and settings match; there is no
historical R3/R5 pooling or native32/kernel/fence composition in these numbers.

This is one sequential cohort per engine, not cross-engine ABBA or formal
qualification. SSE-derived TPOT is not per-token ITL; finite closed-loop output
rate is not sustained loaded throughput. Clock, power, temperature and throttle
equivalence are not established, and sampled observations do not prove
continuous GPU isolation. The [independent pair report](performance/http-v18-r6-matched-a001-independent-review.json)
binds both complete archives and raw arithmetic. No vendor win or default
promotion is claimed.

#### Earlier Attempts And Monitor Repair

R3 completes the standalone V17 B HTTP arm with ten excluded warmups, thirty
measured requests and two untimed diagnostics. Independent raw replay confirms
all 40 client streams, exact IDs/bytes for all 42 internal requests (5,376 output
tokens), receipt/identity hashes and clean unsignaled Closed/exit-zero teardown.

| Ferric V17 B HTTP | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| TTFT | 787.483946 | 776.553537 | 847.102016 |
| TPOT | 53.312156 | 53.207102 | 55.396056 |

Finite measured-window output rate is 16.934201 tokens/s. This is Qwen3-8B,
TP1/C1, 128/128 tokens, context 8192, BF16 decoder and explicit FP32 head,
greedy, prefix caching and speculation off. SSE-derived TPOT is not per-token
ITL or steady-state throughput. All 122 paired GPU samples pass, including
85 positive active samples; interval observations do not prove continuous
isolation. Clock, power, temperature and throttle equivalence remain unmeasured.

The [independent raw review](performance/http-v18-r3-ferric-a001-independent-review.json)
binds the complete 1,045,661-byte archive, SHA256
`b060ae521fd6fce3809c9460d850f578ad1c81fa1d906790732fd04a4e340a27`.
All 526 archive entries are retained. This is not an A/B speedup estimate or a
new vendor win. Two vLLM attempts stop before timing when container birth changes
the startup owner roster within a sample. Both failed closures remain retained;
the exact containers and caches are removed. Further unchanged retries stop.
R4 passes 218 fixtures and native preparation, then stops before timing on a
departing non-init container process. R5 adds proven zero-GPU non-init departure
handling without changing the scanner or active policy. All 235 remote fixtures
and actual CPU binding pass. Its fresh vLLM arm completes cleanly: independent
raw replay gives median TTFT 20.180312 ms, TPOT 4.397091 ms and finite output rate
221.121387 tokens/s. Timed vLLM checks cover exact UTF-8/usage, with token-ID
parity only in the untimed before/after diagnostics. All 31 GPU samples pass,
including nine positive active descriptor samples. R5 Ferric is interrupted by
shared root-disk growth below the fixed floor; no timing from that attempt is
admitted. Pinned unused test/metadata cache cleanup reclaims 10.787 GB, but
concurrent writes leave the native root-space floor unmet. A fresh same-plan
Ferric arm is required after stable resource admission. After exact cache
cleanup and input restoration, R5 Ferric passes admission but stops in warmup9
before any timed requests. FD6 closes during the descriptor scan; the exact
owned process identity and paired positive GPU endpoints remain unchanged.
R5 refuses this as process disappearance. The failed raw run is retained and
not relabeled. R6 adds only a bounded complete native-owner FD rescan with
unchanged budgets, full identity checks and fresh positive descriptor evidence;
both disabled and exact enabled sources pass 262 remote fixtures. The enabled
source's actual CPU binding and MI350 preparation pass. Fresh R6 Ferric now
completes all 30 timed requests, ten excluded warmups and two diagnostics with
clean unsignaled teardown. Independent raw replay gives median TTFT
783.766966 ms, TPOT 53.399713 ms and finite output rate 16.932225 tokens/s;
all 42 internal requests / 5,376 token IDs match. All 529 retained file hashes
and 123 accepted GPU samples are checked. The bounded FD retry is exercised
once and retains both the failed attempt and complete stable rescan. This is
monitor correctness, not a performance improvement. Fresh vLLM also completes
with the identical R6 plan, producing the reviewed pair above. R3 Ferric and
R5 vLLM remain separate historical observations.

### V17 Native Prefill: Complete TTFT Improvement

The fourth attempt completes both counter cells and three ABBA blocks, with
24 measured requests per arm after two excluded warmups per latency cell.
The complete replay and independent streamed archive review pass: 9,472 exact
tokens across 74 requests, all 14 clean receipt chains and 86 raw/receipt hashes.
Only prefill submission grouping changes. Both arms retain the optimized V27
KV-copy kernel, ordinary fences, full planning and native 652-command decode.

| Native Prefill Mode | Median TTFT (ms) | Median TPOT (ms) | Finite Output Rate (tok/s) |
| --- | ---: | ---: | ---: |
| A: ordinary prefill submission | 868.156454 | 53.210963 | 16.929360 |
| B: native 613-command prefill | 772.931099 | 53.192764 | 17.518594 |

Median TTFT improves **10.9687%**, with all six adjacent pairs positive in
both execution orders. TTFT p95 improves 1099.927384 -> 785.066423 ms.
TPOT median improves only **0.0342%**; its p95 improves
55.389392 -> 53.884683 ms. All fixed engineering gates pass. This is not a
statistical proof, an ordered64 comparison, HTTP latency or a vendor win.
The scope is prefill scheduling within the native backend only; default
selection remains unchanged. Failed a001/a002/a003 attempts are not pooled.
The [complete report](performance/prefill-v17-native-a004-report.json) and
[independent review](performance/prefill-v17-native-a004-independent-review.json)
bind the 75,559,089-byte archive, SHA256
`ba3c28709dd76a55a4b700a314f39c152fac9c8a15b37d2196825f015a5ea05d`.
All 2,805 archive entries and embedded plan/report hashes are checked. Native
admission is replayed on the host, not rerun by the independent local arithmetic
review. Clock, power, temperature and throttle equivalence are not measured.

V18 preparation binds this exact complete cohort and selects standalone B for
matched HTTP measurement. Its first supervised attempt fails before launching
the engine on tuple-keyed GPU-evidence JSON serialization. No HTTP timing is
admitted. A narrow successor passes 87 fixtures and actual CPU binding, but its
first run then rejects the owned worker during startup: that phase is incorrectly
mapped to the scanner's empty preflight policy. It stops before timing, with a
reaped outer group and empty postflight but failed inner clean closure. A separate
startup-phase repair passes 200 remote tests and actual CPU binding. Fresh R3
native preparation and the supervised Ferric HTTP run pass, as recorded above.
No failed attempt is treated as a comparison. Both R6 HTTP arms now complete
with independent matched review as recorded above.

### V20 Host Diagnostic: Completion Wait Dominates

One separately instrumented 128/128 request passes exact token/UTF-8 parity,
all 135 batches and 87,711 dispatches, clean unsignaled teardown and full raw
replay. Ordinary prefill16, system fences, historical images and the qualified
`dcf345452` worker are unchanged. Independent review recomputes all 19 general
and seven token-counter deltas from retained endpoint responses.

| Component | Host Wall ms/Decode Execution |
| --- | ---: |
| Runtime dispatch preparation | 0.174519 |
| Kernarg staging | 0.014469 |
| Packet publication | 0.016712 |
| Completion wait, including polling/validation | 49.443990 |
| Signed unassigned Execute round-trip residual | 0.391279 |
| Complete Execute round trip | 50.040968 |

Completion wait is **98.8070% of the measured Execute round trip**, not 98.8%
of GPU time or of end-to-end TPOT. Warm caller planning is 0.356612 ms/call.
The broader registered-program lifetime averages 57.752972 ms/execution, leaving
7.712004 ms/execution outside Execute. That lifetime extends to release and can
include harness endpoint checks and drain delay after the final token; it is not
an isolated between-token overhead budget. Its total even exceeds the retained
request's ingress interval. Caller graph/read/write costs remain unattributed.
General command/currentness counters overlap other scopes and are not added to
the table.

This directs the next work toward kernel/queue execution, rather than claiming
the cached planner alone can close the gap. Caller optimization requires tighter
scope attribution before treating the broad lifetime residual as critical path.
It is a one-request diagnostic, not a latency comparison or a vLLM speedup.
The [exact report](performance/host-v20-native-a001-report.json) has SHA256
`575cf82e1313a3b33232189731b07ec98286dcf44b5e8504f11f96492bcd2ee5`.

### V16 Boundary Fences: Complete, Below Promotion Gate

All 14 V16 cells and the guarded raw replay pass. Three ABBA blocks provide
24 measured 128/128 requests per arm after two excluded warmups per cell.
All 9,472 output token IDs and streamed bytes match, including counter cells
and warmups. Both arms use the same `dcf345452` worker, controller and images;
only ordinary system fences versus system-boundary fences differ. Prefill
and kernel mathematics are unchanged.

| Native Fence Mode | Median TTFT (ms) | Median TPOT (ms) | Finite Output Rate (tok/s) |
| --- | ---: | ---: | ---: |
| A: system every dispatch | 877.497994 | 53.734052 | 16.510235 |
| B: system boundaries | 871.949030 | 53.159463 | 16.715455 |

Median TPOT improves **1.0693%** and TTFT **0.6324%**. Five of six adjacent
pairs and both order medians improve; all regression limits pass. The fixed
5% minimum TPOT gate fails, so the evaluator returns `inconclusive` and the
candidate remains default-off. This estimates fence placement within the native
backend only, not promotion over ordered64 or a fresh HTTP/vendor comparison.
Both counter cells retain 127 publications/waits, 82,804 dispatches/retirements
and 27,815,032 initialized bytes. Observed controller/worker CPU per output
falls 4.296875 -> 3.802083 ms (11.5152%); this includes prefill and warmups,
excludes setup/teardown and counter cells, and is not GPU time.

The [machine observation](performance/fence-v16-native-attempt-a001.json)
binds the [complete report](performance/fence-v16-native-a001-report.json)
and independent streamed raw arithmetic/token review. Full local archive custody
is verified: 70,072,638 bytes, SHA256
`e4dc8fcac13b00316b77ebf488d46dad328856fe5c393cae92f00e65302cdaaa`.
All 14 outer receipts are clean, reaped and unsignaled. Earlier failures and
the frozen V14 comparison below retain their original attribution.

### V14 Whole Token: Retained Regression

All 14 `a005` cells complete with status zero, clean teardown and independent
raw replay. Counter A/B precede three ABBA blocks, each latency cell containing
two excluded warmups and four measured requests: 24 measured requests per arm.
All 9,472 output token IDs and streamed bytes match, including counters and
warmups. The SSH disconnect occurs after cohort completion at 21:20 UTC, not
during a failed cell. Frozen worker/controllers remain attributed to `eded9474b`;
this is not a V15 counted-encoding or latest-compiler kernel measurement.

| Frozen V14 Backend | Median TTFT (ms) | Median TPOT (ms) | Finite Output Rate (tok/s) |
| --- | ---: | ---: | ---: |
| A: ordered64 groups | 820.787174 | 50.134290 | 17.398367 |
| B: native whole-token | 872.554286 | 53.245459 | 16.878285 |

Candidate median TPOT is **6.2057% worse**, median TTFT **6.3070% worse**, and
finite output rate **2.9893% lower**. Only two of six adjacent comparisons improve;
all four predeclared promotion checks fail. The evaluator returns `inconclusive`,
not promotable. TPOT p95 is 55.332631/55.144087 ms and TTFT p95 is
1070.407940/1113.855917 ms for A/B. These are native ingress measurements,
not HTTP latency, sustained loaded throughput or a fresh vLLM comparison.
The candidate remains default-off and no slower clean observation is discarded.

The instrumented a005 counters confirm 1,397 -> 127 publications/waits while
preserving 82,804 dispatches/retirement signals over 127 decode executions.
Initialized bytes are 5,454,457,976 -> 27,815,032; host staging is
62,357,256 -> 2,992,013 ns. These mechanism reductions did not yield a model
latency gain. Total observed controller plus worker CPU falls from 5.592448 to
4.266493 ms per output token (23.71% lower), while model latency regresses.
This CPU scope includes prefill and warmups, excludes setup/teardown, and is
neither decode-only nor GPU time. Host work reduction is not critical-path gain.
The [complete replay report](performance/whole-token-v14-native-a005-report.json)
and [machine observation](performance/whole-token-v14-native-attempt-a005.json)
retain exact metrics and hashes. Full local archive custody is now verified:
72,947,665 bytes, SHA256
`1c82dd9f151d402a277b6d40e7cbb52177b01eade8b9d74e8c7d838362f93114`;
all 2,771 archive entries and embedded plan/report hashes were checked. The
remote stage still requires a fresh ownership/non-use check before retirement.

### Retained Earlier Attempts And Implementation

The `a004` counter A/B pair passes native correctness: each validates all 128
output tokens. Across 127 decode executions, publications and final waits fall
from 1,397 to 127, while dispatches and retirement signals both remain 82,804.
Initialized kernarg bytes fall from 5,454,457,976 to 27,815,032. Instrumented
host staging falls from 91,417,611 to 1,734,822 ns across those executions.
These establish the mechanism, not a TTFT/TPOT gain.

The first uninstrumented cell stops during startup after process-monitoring
refusals. Its separately-sessioned controller survives wrapper teardown, then
exits before a fresh manual identity check; no manual signals are sent. The
failed cleanup receipt stays failed. All raw evidence is retained in the
[fourth attempt observation](performance/whole-token-v14-native-attempt-a004.json).
The teardown successor is now integrated from the exact remotely qualified
18-file `a005` harness. Its dedicated stop exception cannot be swallowed by
Python selectors; controller and worker share the outer-owned wrapper group,
which remains unreaped until group cleanup finishes. The finite-roster scanner
records late births explicitly and requires paired sysfs/fuser endpoints;
it does not claim atomic host coverage or continuous isolation. Qualification
passes 29 teardown fixtures, 101 scanner fixtures, 213 combined measurement tests
and 28 launcher tests, plus 12 portable-binding fixtures and the actual retained
bundle check. Three teardown fixtures use real CPU-only stand-in processes.
See the [integrated harness record](performance/whole-token-v14-harness-a005.json).
The preceding `a003` preflight refusal and failed `a004` cell remain retained;
no failed cell is overwritten or pooled, and this repair is not a speedup.

The generic V15 counted-encoding API is now on fe2o3 main at `cea62c79d`,
rebased onto fresh `13c49a56`, with CI skipped. Final remote qualification
passes 711 library and seven worker tests, strict Clippy and the release build.
The clean temporary core worktree is removed. Ferric's counted-encoding client
has passed published-pin qualification and is integrated in the shared checkout:
126/127/126 controller tests, 56 source-policy tests, strict Clippy, two actual-image
CPU gates and three release controllers. A same-core old-encode native controller
is separately built and qualified for the next comparison. Neither is substituted
into the frozen V14 control: that campaign still uses the original `eded9474b`
worker/controllers, with ordered64 A and whole-token B. V15 will compare old
encoding versus counted validation with the native backend in both arms. See the
[publication record](performance/counted-encoding-v15-publication.json).

## Baseline

The completed [V13 comparison](M1_PERFORMANCE_SWARM_V13.md) measured Qwen3-8B
on one physical MI350, TP1/C1, 128 input and 128 output tokens, context 8192,
greedy generation, BF16 decoder with an explicit FP32 head in both engines,
prefix caching and speculation off. Each cohort had 10 excluded warmups,
30 measured requests and two untimed diagnostics.

| Engine / Profile | Mean TTFT (ms) | Mean TPOT (ms) | Finite Output Rate (tok/s) |
| --- | ---: | ---: | ---: |
| Ferric packed-down R1 | 827.434 | 59.780 | 15.201 |
| vLLM 0.28.0 | 19.671 | 4.245 | 228.823 |

These are historical reference values, not the V14 control. The frozen V13
worker predates current fe2o3 main. Empty vendor KFD samples did not establish
positive container attribution or continuous isolation. Packed-down's native
ABBA gain was not repeatable; it remains default-off.

## Teams

| Lane | Owner | Work | Status |
| --- | --- | --- | --- |
| Runtime | v14_runtime | Current compiler and checked race-storage accounting; opt-in 512-slot token programs | Accounting corrections are published with 1,892 tests. New 512-slot family is on main `55c1a9b6`: 47 focused, 734 library, 9 worker and 463 default-mode tests plus strict production Clippy pass remotely. Three engineering and one default tests are ignored. Release worker af246e5b completes the full width cohort and packet diagnostic. Legacy 256-slot limits unchanged |
| Ferric | native32_cpu_recovery | Native prefill32 HTTP successor, paired split-K and precision audit | Width a003 and fresh Width55c HTTP pair pass independent review. Paired split-K passes80 fixtures and full native audit but improves host median only0.0865%,3/6pairs. No promotion. Reviewing weight traffic and existing BF16 projection paths; no inferred model TPOT gain |
| Verification | v14_measurement | Split-K full-model harness, raw replay and paired metrics | Width and packet-diagnostic archives independently pass raw review. Ordered split-K component gains remain component-only. Full-model split-K selected CPU scope passes91 tests and release; strict Clippy fails and seven matrix roles remain deferred. Exact ELF retained for explicitly experimental native qualification; full-model harness in progress |
| Integration | root | Serial resources, correctness, paired native measurements, archive custody and live Pages | Fresh Width55c HTTP comparison complete and independently audited:22.7233x TTFT and12.2045x TPOT slower than vLLM. Paired-load component does not justify promotion; full-model unpaired split-K next. Pages still publishes the earlier V17 checkpoint, not the new width result. No vendor win or default promotion |

## Experiment Order

1. Rebuild and qualify the existing compatible token-program control on current
   fe2o3 main. A source revision alone does not establish a current baseline.
2. Qualify the whole-token backend using the identical 652-dispatch graph and
   kernels. Keep dynamic validation, WaitForPrior ordering, deadlines, poison
   retention and completion-frontier checks. Defaults remain unchanged.
3. Measure the backend alone. Keep instrumented mechanism/cost runs separate
   from uninstrumented latency runs. Do not label runtime wait as GPU-only time.
4. Measure kernel and prefill changes individually, then explicitly qualify
   their composition. Do not combine incompatible packed-projection profiles.
5. Repeat the matched vLLM comparison after a repeatable Ferric improvement.

## Fixed Measurement Gates

Correctness and mechanism qualification precede performance measurement.
Both backends must execute 652 dispatches and validate 652 retirement signals
per decode token. The candidate targets 11 -> 1 publications, doorbells and
final-wait episodes, without weakening validation or reusing outstanding slots.
Compact staging and publication reduction are one initial experiment; later
ablations are necessary to attribute their individual effects.

Use three ABBA blocks, each cell with two excluded warmups and four measured
128/128 requests: 24 measured requests per arm. Retain every clean cell, including
regressions. Interrupted or invalid cells remain separate failed receipts.

- All output token IDs, graph, artifacts, precision and completion counts match.
- At least 5% lower aggregate median TPOT, faster in five of six adjacent pairs,
  and positive improvement in both AB and BA orders.
- TTFT median/p95, TPOT p95 and finite output rate do not regress by over 5%.
- No timeout, poison, fallback, resource leak or foreign GPU interference.
- Record worker CPU, memory, setup and stage bytes separately from latency.

## Change Ledger

| ID | Change | CPU Validation | Native Correctness | TTFT Delta | TPOT Delta | Decision |
| --- | --- | --- | --- | --- | --- | --- |
| V14-00 | Frozen eded9474b compatible control | 695 initial library tests passed; release worker built | a005 all reference IDs/bytes match | Median 820.787174 ms | Median 50.134290 ms | Measured control, not latest-source relabeling |
| V14-01 | Compact whole-token submission | Runtime, controller, source-policy, feature-union, strict Clippy and release checks passed on the published pin | a005 all reference IDs/bytes match; 11 -> 1 publications/waits per decode | Median 872.554286 ms; +6.3070% worse | Median 53.245459 ms; +6.2057% worse | All promotion checks fail; default-off |
| V14-02 | Container-aware attribution, owned teardown and paired results validator | 213 measurement/replay + 28 launcher tests; 12 binding fixtures and actual retained-bundle check passed | Successor not itself native qualification | Not a speedup | Not a speedup | Exact qualified a005 harness integrated |
| V14-03 | Uninstrumented prefill grouping selector | Isolated proposal; not integrated or tested | Pending | Not measured | Not measured | Run separately after V14-01 |
| V15-01 | Counted encoding instead of discarded owned payload | Approximately 149 us/call saved in release CPU replay; published-pin controller, source-policy, actual-image, strict Clippy and release checks pass | Pending | Not measured | Not measured | Generic API published; Ferric client integrated, no native promotion |
| V16-01 | Eight-way split-K C1 down projection | Qualified SDK84 tests and emission pass. Full-model selected scope passes91 tests and release; strict Clippy fails, seven matrix roles deferred | Ordered component parity passes; full-model native parity pending | Not measured | Not measured | Exact full-model ELF retained as experimental only; no model performance promotion |
| V16-COMPONENT-ORDINARY | Frozen three-arm down-projection fixture, ordinary dispatch | Original component/launcher and exact203-file preparation closure pass | All30 fixture samples and clean teardown independently verified | Not model TTFT | Not model TPOT | Wave/MFMA about7 ms host-route versus split-K about14 ms for two calls; no GPU ranking or kernel promotion. Preserved historical cohort |
| V16-COMPONENT-ORDERED | Same down fixture, one ordered frame per arm | 66 unique remote fixtures and three actual bundle preparations pass | Latency/counters pass; timestamp cohort quarantined on outer disk-floor failure | Not model TTFT | Not model TPOT | Host-call medians improve24.468% versus Wave and58.660% versus MFMA; six faster pairs each. No shader-only or inference gain claim |
| V16-PAIRED-LOADS | Paired loads within split-K partial, identical merge | 80 remote fixtures, exact source and actual preparation pass | All16 cells,32packets, exact guarded parity and full590-file audit pass | Not model TTFT | Not model TPOT | Host-call median0.0865% lower,3/6pairs; worker median0.4478% lower,4/6pairs. No useful repeatable gain; keep disabled |
| V16-FENCE | System-boundary fences within native backend | Published-pin runtime/controller, actual-image and harness checks pass | All 14 cells and 9,472 reference tokens/bytes pass | Median 877.497994 -> 871.949030 ms; 0.6324% lower | Median 53.734052 -> 53.159463 ms; 1.0693% lower | Below fixed 5% gate; default-off; not ordered64/vendor promotion |
| V17-PREFILL | 613-command native prefill program, ordinary fences | Eight planner and 666 controller tests; actual613/652/rejection, Clippy, release and actual harness binding pass | a004 all14 cells, raw replay and independent archive review pass. Failed attempts retained separately | 868.156454 -> 772.931099 ms median (-10.9687%) | 53.210963 -> 53.192764 ms median (-0.0342%) | All engineering gates pass within-native only; no default promotion or composition |
| V19-CACHED | Registered raw-ABI preparation template, ordinary prefill/fences | 701 final-source controller tests; actual ABI/replay/652/rejection, Clippy and release; 227 measurement, 37 launcher, four binder and actual-bundle checks pass | Counter A/B, first two ABBA blocks and third-block A1/B1/B2 pass; only final A2 pending | Not measured | Not measured | Separate within-native experiment; no default promotion |
| V20-HOST | Per-program host timing diagnostic using existing runtime counters | 761 controller, 59 source-policy, 112 union tests; default check, strict Clippy, real652/rejection and release; 174 measurement, 28 launcher, four binder and actual-bundle gates pass | One native128/128 request, exact parity, clean closure and raw replay pass | Diagnostic only | 49.443990 ms completion wait inside 50.040968 ms Execute round trip | Kernel/queue interval dominates; no GPU-only or benchmark TPOT claim |
| PREFILL32 | Explicit native649-command/396-slot prefill, unchanged legacy decode | Current55c1a9b6 worker; exact A005 source passes all22 CPU/release roles and explicit actual-image gates. R4 harness323 unique tests and real28-role binding pass | a003 all14 clean cells and9,472 token IDs/bytes pass complete replay and independent archive audit | 772.737844 -> 400.696764 ms median (-48.1458%) | 52.912262 -> 47.786564 ms median (-9.6872%); unchanged decode, p95+0.8392% | Fixed within-native width gates pass; default-off, no vendor win or isolated decode-gain claim |
| PREFILL32-HTTP | Same qualified width B32 through unchanged matched HTTP workload | Exact enabled successor passes280 tests on mi300x-2; original and portable CPU bindings retained | Both arms independently audited; exact parity and clean teardown | Ferric431.032717 ms versus vLLM18.968767 ms (22.7233x slower) | Ferric53.118820 ms versus vLLM4.352392 ms (12.2045x slower) | Fresh same-plan sequential comparison, not cross-engine ABBA. No meaningful decode gain or vendor win |

The frozen V14 latency arms use the same retained worker binary with distinct
explicit backend flags. Kernel images remain byte-identical historical artifacts;
this does not claim re-emission with the latest compiler. Initial Ferric CPU builds
used an explicit current-wire dependency overlay. The final manifest and lockfile
for that frozen campaign pin published core `eded9474b`; remote qualification
passed on that pin. Current shared Ferric source now pins `cea62c79d` for V15.
The two revisions and their independently retained binaries are not relabeled.
Default binaries and production readiness are not claimed.

The [ordinary component audit](performance/splitk-component-ordinary-a001-independent-review.json)
binds the complete 248-file archive `8a981ad0...`. Paired host-route medians are
Wave 6.996234 ms versus split-K 14.010309 ms, and unsplit MFMA 7.054594 ms versus
split-K 14.028869 ms. The original probe uses default currentness checks and
two synchronous calls for split-K; these measurements cannot rank GPU execution
or establish a kernel regression. All modes have exact output/partial/guard
checks, with six excluded warmups and 24 timed fixture samples. No 5 ms sleep
attribution, calibrated device timing or model speedup is inferred.

The earlier [machine-readable checkpoint](performance/whole-token-v14-checkpoint.json)
and [hash-linked change ledger](performance/whole-token-v14.jsonl) preserve their
historical pending/null metrics. The separately dated
[a005 observation](performance/whole-token-v14-native-attempt-a005.json) and exact
report above contain the completed regression; earlier entries are not rewritten.
The reusable
[campaign tool](../adapters/m1-engineering-execution-v1/tools/whole_token_benchmark_v1/README.md)
predeclares counter A/B followed by three ABBA blocks with shared immutable input
paths. Failed, incomplete or non-repeatable results cannot promote a default.

The earlier native-host observation at `2026-10-05T18:28:48Z` reported
64,619,393,024 root bytes available against the unchanged 68,719,476,736-byte
floor. All identified Ferric temporary roots total only 347,734,016 bytes, so
deleting them would not cover the shortfall. Borrowed models and unrelated
campaigns remained untouched. No V14 GPU run or vendor comparison was launched
at that checkpoint.

### Native Handoff Follow-Up

The upstream audit now observes main `bee71f6b4aaa507a5bcefcf23d5bcd562ee145cd`.
Only compiler-client files changed after `eded9474b`; all nine current-wire
dependency trees and the root manifest/lock are identical Git objects. The
qualified worker remains explicitly pinned to `eded9474b`, not relabeled as
built from the newer revision. See the
[source audit](performance/whole-token-v14-upstream-audit.json).

The portable build-binding helper passed 12 remote fixtures and an actual
retained-bundle integration check under the unchanged G28 guard. It runs the
existing CPU/build validators and does not invent or rewrite qualification
receipts. The native transport SHA-256 is
`b77ee63a614f1dbd2f98255353fdb5e490a843f52c7ab829ae2020d948900314`.

An expanded MI350 inventory found an unused September 7 Cargo target in the
user's home. After an empty all-process open-reference scan, Cargo locks and
old-entry identity checks, cleanup removed only inventoried incremental files
and Rust static archives. It preserved executables, shared libraries, kernel
artifacts, models and benchmark evidence. Available root space rose from
59,247,173,632 to 71,326,732,288 bytes; the 64 GiB floor was unchanged.
All eight GPUs were idle before staging.

The original MI350 profile is now prepared as
`/dev/shm/ferric-v14-native-40695ec-a001`, plan SHA-256
`b098b9cde0816f54aeac54c02a1b0ccfcef507b0c08911e50e11f1a86abefd9a`.
Counter-A supervision refused before controller launch: each of the three
preflight descriptor scans encountered a departed PID, and postflight scans
also refused. Its status-125 receipt and complete stage are retained in
`native-a001-failed.tar.gz`, SHA-256
`e7c5163b5868c382ad1f751f12e225f60ba6058a0ba95d32e4f1e6229cd60579`.
The [attempt observation](performance/whole-token-v14-native-attempt-a001.json)
preserves the failure with null metrics. A separate scanner successor is being
tested; the frozen failed campaign is not modified or overwritten.
Correctness, mechanism, ABBA latency and vendor results remain pending; no
speedup is claimed. MI350-2 was also inspected
but not used: its root remains below the floor and its system GPU monitor owns
KFD. No monitor service or shared job was stopped.

The departed-process scanner successor passed 138 measurement/replay and 18
launcher tests. A fresh `a002` counter-A attempt passed admission and reached
the worker handshake, but monitoring later refused live-process descriptor
disappearance and identity changes. The supervisor terminated its owned
processes before any completed request; postflight also fell below the unchanged
root-space floor. The failed cleanup receipt remains failed even though a later
check found no owned process or KFD user. The complete attempt is retained as
`native-a002-failed.tar.gz`, SHA-256
`ce41a0d58bfb6192ad2b381cd25d43f73375d5a5771c0764dbfbe924af3e9d88`.
See the [second attempt](performance/whole-token-v14-native-attempt-a002.json).

A separate read-only diagnostic probe identified an SSH process changing
cgroups during login and ordinary descriptor closure in `du` and a terminating
health-metrics process. These probes do not admit a native run. A bounded full
per-process rescan successor is being developed without ignoring GPU users or
lowering resource limits. A second unused old Cargo cache yielded 4,082,814,976
allocated bytes; models, executables, sources and evidence were preserved.

The isolated V15 counted-encoding candidate now passes 711 core library and
seven worker tests. The three controller suites pass 126, 127 and 126 tests;
their explicit real-image and CPU replay checks are tracked separately. An
optional broad library run was interrupted in an unchanged slow test and is
not counted as passing. The candidate avoids materializing discarded per-token
encoding while retaining the same validation and bounds. The two actual-image
CPU gates now pass, including late-invalid dispatch rejection before
registration. The release CPU replay measures 356.533 -> 207.360 us/call at
position 143 and 359.045 -> 209.987 us/call at position 144: 41.84% and 41.52%
reductions in this operation, about 149 us/call saved. All 12 adjacent AB/BA
pairs improve. Its 24 measured groups of 64 calls run in one process; they are
not 24 independent process starts. See the
[CPU observation](performance/counted-encoding-v15-cpu-a001.json).
Allocation counts, native TTFT/TPOT and model parity are not established by
this replay. The generic API is published; the Ferric candidate is now integrated
after published-pin qualification. Native performance promotion remains pending,
and the opt-in native backend is not enabled as a production default.

An additional exact old-cache cleanup removed 1,464 unused static archives
(10,917,408,768 allocated bytes) from three inactive targets. Eight Cargo locks,
old-entry identities and all-process open references were rechecked; the active
sibling build and two hardlinked archives were preserved. Root space rose from
68,247,191,552 to 79,154,130,944 bytes. Inventory SHA-256 is
`74c0bac734aac6d311275f01a06f66b376346bc59dc9b8a6784e9cefaaa10d99`.

The [second upstream audit](performance/whole-token-v14-upstream-audit-a002.json)
observes main `60dc6edb`; the current-wire dependency trees and root manifests
remain unchanged. Frozen binaries retain their original revision identities.

The final worker, three controllers, source archives and all CPU receipts are
retained under `products/perf-v14-20261005` in the integration evidence store.
Ferric source changes remain in the existing dirty integration checkout, not
pushed to main. The bounded mi300x build cache remains needed for the pending
native handoff and subsequent isolated prefill experiment; no build or GPU job
is left running at this checkpoint.

Core publication followed a fresh fetch and rebase onto main, with `[skip ci]`;
the GitHub API reported no workflow runs for that commit. Its source archive is
retained with SHA-256
`5087f055a8368123bd62a947e27b86a0e28c3ad961e4905335193b609af84eb9`.
The clean temporary core worktree was removed after publication and retention.

Failed and interrupted attempts are retained separately, including an initial
stale Cargo fingerprint, a strict-query decoding test caught and corrected, and
an intentionally stopped broad debug test run. They are not accepted validation.
Only clean, completed focused reruns count above.

## Resource And Source Boundaries

All builds/tests run remotely in one serialized four-core lane: the retained
client stage uses G28 on mi300x, and current compiler/kernel qualification uses
the separately identified G32 stage on mi300x-2. Both retain 8 GiB RSS, nice 19,
20-minute phases, a 512 MiB reserve and unchanged host-space checks. Historical
receipts retain their original caps. GPU work runs on mi350 only after fresh
resource/identity checks. No local or GitHub-hosted builds.

The existing conflicted fe2o3 checkout is read-only. Core work uses an isolated
worktree at the refreshed main revision; no unrelated changes are reverted.
Generic compiler/KFD work belongs in fe2o3; kernels and inference in Ferric.
Core changes rebase onto latest main before any push. Owned worktrees and
temporary stages are retired after their patches/results are retained.
