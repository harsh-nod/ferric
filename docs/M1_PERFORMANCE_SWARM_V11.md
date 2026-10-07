# Performance Swarm V11

Started 2026-10-01 UTC after the kernel/engine bottleneck review. This campaign
implements isolated improvements and measures them; it does not claim a vendor
win or promote an unmeasured candidate.

## Baseline

The latest matched HTTP comparison remains September 28: Qwen3-8B, MI350 GPU0,
TP1/C1, 128 input and 128 output tokens, BF16 decoder/FP32 logits, context 8192,
greedy output, prefix caching and speculation disabled. Ferric mean TTFT/TPOT
is 888.594/55.775 ms versus vLLM 20.125/4.230 ms. This is one warmed,
fixed-order pair, not repeated-start confidence or sustained throughput.

The October 1 current-wire diagnostic is a separate baseline with ordinary
652-dispatch decode, not the 688-dispatch packed HTTP composition. Its host
publication/completion interval includes GPU and runtime work; it is not shader
time. Historical device ticks have no established conversion to GPU durations.

## Teams

| Lane | Owner | Work | Status | Measured Gain |
| --- | --- | --- | --- | --- |
| Kernels | compiler_receipt_audit / v11_kernel_finish | Packed-down FP32, pack-inclusive parity and timing | CPU qualification, both actual images, 18 native parity cells and 16 timing cells pass; exact 14-file device crate imported; native stage and transports retired | Component controller wall lower 13.0201% for singles / 39.9771% for five-operation groups; no model switch or serving claim |
| Engine | latest_core_audit | Default-off TP1 prefill16 staged ordering, command identity and failure tests | CPU qualification and six native requests pass; raw results retained and independently replayed; guarded retirement refused | TTFT lower 13.4406% / 10.5572% in two cold instrumented pairs; decode/rate mixed, no promotion |
| Profiling | retry_receipt_audit / v11_packet_finish | Vendor/Ferric attribution and current-worker KV-copy ablation | Full capture and four uninstrumented V19 ABBA starts pass; 2,048 exact IDs and independent raw audits; all three native roots retired | V19 TPOT lower 6.9433% / 16.5023%; TTFT worse 11.8241% / 1.2629%; default unchanged |
| Integration | root | Current compiler/runtime, serial remote builds/runs, review, measurements and cleanup | Prefill binding passes; core-only diagnostic patch published to fe2o3 main 4d6c6ae83 with CI skipped; reviewing candidates and retaining failures separately | None yet |

## October 1 Results

The new prefill selector remains default-off and does not change kernel math.
All nine new recording tests pass within the 21-test focused library selection,
including command identity, fallback and completion-failure state handling.
All 11 selector/metadata tests and all 54 source-policy tests also pass. The
binding audit caught a wrong module-name filter in the first selector run;
the corrected `selectors-a003` rerun includes all 11 named tests. The
source archive is `18cc733334ae97a2aef83f48fa97f961089444e49c08a6e24f67806cbd45cd72`;
the release controller is
`af244a35fb43450d3173ad7fe6bb7896686ee083f4ed4939d41d91c1c0a05076`.
The harness's first fixture attempt stopped on a stale analyzer hash before
running its intended tests. That failed receipt is retained, not accepted.
The corrected, separately retained harness passes all 60 fixtures remotely.
The ten-phase source/build/worker/harness binding passes separately, retaining
the actual controller and worker before build-cache reuse.
### Compiler And Kernel Qualification

The compiler producer builds at `962044346` plus the capture patch. Its backend
capture/regression selections pass 9 and 14 tests; CLI capture/regression
selections pass 10 and 27 tests, with overlapping coverage; all 64 extractor
tests pass. The optional capture code is now published separately to
[fe2o3 main, 4d6c6ae83](https://github.com/harsh-nod/fe2o3/commit/4d6c6ae83cdaf77df84d3b98356de870bd4e1ed9),
rebased onto `b8194784a`. That upstream advancement was documentation-only.
Publication applies remote formatting-only postimages and a clean remote
formatting check; it does not relabel the original producer test receipts or
the frozen kernel campaign. The commit skips GitHub CI; the post-push API check
reports zero runs for its SHA. The clean publication worktree was removed
after retaining the commit and patch. No kernel or inference code went to fe2o3.

A subsequent remote refresh finds main at `071105ad40fe865051363fc9f60481da7db82fa9`.
Since publication, `863d0da27` adds the nominal ranked consumer and factors the
access-correspondence predicate; it also fixes a format-sensitive capture test
to accept rustfmt's optional trailing comma without changing the production
guard. `c7b0fb779` packages debugger maintenance source; `071105ad4` updates
documentation. The changed crate files are confined to the compiler; KFD,
SDK, manifests and locks are unchanged over this interval. This is a read-only
source audit, not a build or binary-equivalence claim. The running A/B cohort
keeps its original producer and image identities; the next compiler
qualification must start from refreshed main rather than relabeling this one.
A further refresh finds `7c4d699592fcf1445aa2bb65134417074cecb272`, adding only
one documentation checkpoint since `071105ad4`. No running A/B input changes.

The packed-down candidate passes all 30 CPU tests. Its native parity/timing,
launcher and host-profile fixtures pass 14, 13 and 31 tests respectively.
A separate baseline debug-profile check reached the private stage reserve
and was stopped before Clippy, vendor preparation or emission. That forced-stop
receipt remains failed, including its unverified detached-cleanup status;
a subsequent process check does not rewrite it. Exact retained transport and
unused source cleanup restored admission without changing the cap or reserve.
Two toolchain download archives and eight old native-input archives reclaim
1,441,980,416 allocated bytes; installed toolchains remain untouched. The
unused pristine 962 copy and both old 4fb8 source copies are also retired after
independent full custody. The release-profile baseline check and both strict
Clippy checks pass. A final producer rebuild restores the original build
backend hash, and the actual quartet is retained before emission. Historical
emission needs about 1.014 GB of transient space beyond its small final image;
each new emission requires that space plus a 256 MiB additional margin. Vendor
preparation and both image emissions now pass with clean, unsignaled exits and
completion acknowledgements. Their actual observations bind the same producer
and tool closure. Native finite parity and component timing also pass as
recorded below; no model speedup is claimed.

Actual disassembly explains what this candidate changes. The baseline loop
has three products per body over 64 iterations: six 16-bit loads, an immediate
`vmcnt(0)` after each load, and repeated extent comparisons and execution masks.
The candidate has eight products per body over 24 iterations: eight 32-bit loads
issued before staged waits, with fixed-shape bounds specialized out of that
inner body. Both perform 192 ordered scalar products per lane, check every
product and partial sum for finiteness, and retain six wave-reduction steps and
the final finite-result checks. This halves dynamic load instructions, not
operand bytes. Candidate projection VGPR usage rises 20 -> 23; SGPR usage stays
42, and both have zero fixed LDS/private storage. Neither uses MFMA. The separate
activation pack must be included in timing. These are static code observations,
not measured bottleneck shares, a source-to-binary proof or nonfinite-trap
qualification. Any eventual gain belongs to the complete fixed-shape packed
variant, not packing alone.

### Packed-Down Native Results

On `mi350-2`, all 18 finite parity cells pass with exact full 4,096-element
FP32 outputs from both images, across nine fixtures and two activation epochs.
All 16 timing cells then pass, retaining 512 raw samples and clean unsignaled
worker/supervisor exits. Activation packing runs on every candidate operation;
weight packing, allocation and upload are excluded from the timed interval.

| Operations per group | Baseline mean, us/operation | Candidate mean, us/operation | Lower controller wall | Four paired reductions |
| --- | --- | --- | --- | --- |
| 1 | 714.676484 | 621.625203 | 13.0201% | 12.4527% to 13.4541% |
| 5 | 287.028100 | 172.282631 | 39.9771% | 39.9249% to 40.0655% |

Each group size has AB and BA at both activation epochs. All eight pairs improve.
Root independently recomputed the means from every raw controller start/end
endpoint and checked packing inclusion, sample counts and clean exits. These
are repeated synthetic single-matrix controller-wall measurements, not GPU-only
time, 36-layer model latency, serving throughput, statistical confidence or a
vendor comparison. The prospective model path adds 3.375 GiB of resident packed
weights and a 24 KiB scratch buffer; it remains unintegrated.

The exact 14 qualified files are now in
[`device/qwen3-tp-packed-down-kernels-v1`](../device/qwen3-tp-packed-down-kernels-v1),
with original crate name, SDK pin and bytes preserved. Its historical
proposal-only source comments are not a current results summary; use this
report and the [kernel report](../../ferric-perf-swarm-v6/proposals/perf-v11-kernels-r1/RESULTS.md).
No host selector or default changed. Full custody archive SHA256 is
`88738857b699a336003d39d937934c820f28039a6bbbd37394ec5de7e26014cc`.
The exact native stage and two transport files were retired after independent
custody and unchanged no-use checks, reclaiming 116,736,000 allocated bytes.
The small retirement receipts remain. An initial missing-supervisor CLI
invocation failed before GPU work and remains separate from the accepted run.

### Prefill Scheduling

The complete six-request cohort matches all 768 reference tokens with clean,
unsignaled controller and worker exits. Two correctness requests are excluded
from timing; four fresh processes run serial ABBA. This is a cold, instrumented
TP1/C1 128/128 diagnostic, not warmed HTTP or a matched vendor comparison.
Both policies use the same controller, fe0 worker, eight historical GPU images,
ordinary wait policy and baseline GEMV. Active polling is off in every arm.

Each cell below is baseline -> candidate; BA executed the candidate first.

| Metric | AB | BA |
| --- | --- | --- |
| TTFT, ms | 902.791 -> 781.450 (-13.4406%) | 879.712 -> 786.839 (-10.5572%) |
| Summed prefill batch wall, ms | 902.271 -> 780.949 (-13.4463%) | 879.238 -> 786.332 (-10.5666%) |
| TPOT, ms | 70.782 -> 72.446 (+2.3508%) | 71.808 -> 69.911 (-2.6411%) |
| Finite output tokens/s | 12.939550 -> 12.822904 (-0.9015%) | 12.800891 -> 13.242878 (+3.4528%) |

With only n=2 per policy, ratios of arithmetic means are -12.0176% TTFT,
-12.0251% prefill wall, -0.1631% TPOT and +1.2639% finite output rate.
**Prefill/TTFT improves in both orders, but decode and rate are mixed. The
selector remains default-off; there is no statistical, sustained-serving or
vendor win.**

Independent raw replay confirms eight prefill and 127 decode batches per
request. Prefill ordered groups fall 576 -> 80 (86.1111% fewer); decode remains
1,397, so total groups fall 1,973 -> 1,477. All 87,711 dispatches, 675 metadata
writes and 128 token reads remain unchanged. Recording tests bind the flattened
command/address/argument sequence and transactional failure behavior; kernel
math is unchanged.

Prefill ordered roundtrip wall falls 889.609 -> 772.682 ms in AB and
871.249 -> 777.362 ms in BA. The nested worker publication/completion wall falls
798.358 -> 731.622 ms and 806.565 -> 736.327 ms. These scopes overlap and must
not be added. Worker wall includes GPU work, polling and fences, not just GPU
time; roundtrip minus worker wall includes staging, transport and controller
work, not pure IPC.

### Vendor Profiling

A fresh, separate vLLM run has 30 unprofiled measured requests after ten warmups:
mean TTFT 17.807954 ms, TPOT 4.287830 ms and finite output rate 227.584779 tokens/s.
This is not a new matched Ferric/vendor comparison. The separate instrumented
request records 127 HIP graph launches, each correlated with 434 GPU kernels,
including 144 split-K BF16 projection kernels. Head and sampling work also occurs
outside those graphs, so 434 is not the complete vLLM decode count. Ferric's
ordinary diagnostic complete decode has 652 kernels in 11 ordered groups.
These differently scoped counts are not direct kernel-count or timing ratios.
The profiled vLLM request has about 5.2 times baseline TPOT; frontend profiling
reported an error, and graph-capture traces were absent. Kernel attribution must
use the retained rank-0 events without inferring unavailable decode shapes or
treating profiled durations as uninstrumented latency.

The rank-0 trace contains 58,300 GPU kernel events: 55,118 linked to decode
graph replay and 3,182 outside those graphs, including prefill and head/sampling
work. Its 18,288 split-K calls sum to 349.358 ms and the 128 FP32-output head
calls to 29.816 ms within this instrumented request only. Direct prefill
`aten::mm` correlations identify 36 calls each for QKV, attention output,
combined gate/up and down. Decode split-K events lack External IDs and launch
dimensions, so exact decode shapes or projection assignments remain unproven.
No calibrated Ferric GPU durations or per-kernel Ferric/vendor ratio is claimed.

### Ferric Packet Attribution

The separate eight-image baseline diagnostic passes six library tests (three
new baseline contracts and three preserved V19 contracts), four new selector
tests, all 55 source-policy tests, strict Clippy and its release build. Ordinary
`c1-ordered64` without timestamps additionally passes nine prefill16 and four
ordered64 library tests, plus ten host-selector tests. All 14 native-harness
fixtures pass. The first supplementary library selection ran only the nine
prefill tests because its ordered64 module filter was wrong; that incomplete
receipt is retained separately, and the corrected 13-test receipt supplies the
missing coverage. Overlapping reruns are not distinct tests.

The retained controller is
`201e64a28a52977c86d3a89bf18f296e8b8db2151e117c89fe1482ffee751ab7`;
the frozen formatted source archive is
`2754d259508ed3af3f87d564ef3bd1500b69ab4131453853908b698b1d046c2b`.
The seven remote-format postimages were synced into Ferric only after exact
local preimage checks. No local build or project test was run. Native capture
on `mi350` now preserves all 128 reference tokens, 87,711 packet intervals,
135 batches, 1,973 ordered groups and 11 direct packets. Controller, worker
and supervisor exit cleanly without signals; KFD postflight is empty. Full
archive SHA256 is
`045078f76a1d0365dbc08bf659426a56e5c9c0ba7ab606310e5c446f6c285648`.
All three owned native roots are retired after complete custody and unchanged
no-use checks; their small retirement-control receipts remain.

Within this capture, decode KV append ranks first by accumulated raw intervals
for a tagged operation, followed by down, up and gate projections. Each occurs
4,572 times. Their p50 packet intervals are 31,336, 22,780, 16,068 and 15,920
ticks respectively. This is not a ranking of individual longest calls: head
and argmax are longer per call but occur only once per token. Combined kernel
families also aggregate multiple different operations. Interval sums are not
additive execution shares, and ticks are neither shader-only time nor
calibrated nanoseconds. Instrumented TPOT is not a performance comparison.

Source inspection finds a concrete serial path: baseline paged KV append selects
one grid leader to copy 1,024 TP1 components, while existing V19 distributes them
over 1,024 invocations with host-validated slot views. A current-client/runtime,
same-nine-image V19 baseline/candidate ABBA was selected and completed below.
Existing historical V19 cohorts remain separate; this is requalification of
an existing optimization, not a newly invented kernel or a claimed new gain.

### Current-Worker KV Copy Qualification

The unchanged frozen client now passes 18 ordinary library tests, four
selector/runtime tests, two explicit actual-image tests and four feature-union
rejection tests, plus strict Clippy and the release build. The first library
coordinator expected 16 tests but selected 18 passing tests; a separate named
acceptance binds that original receipt without rerunning it. The first selector
selection passes three CLI tests but misses the runtime test because its module
is `live_tests`, not `tests`. Its incomplete receipt is preserved; an executable
test listing and a separate exact-name continuation supply the missing case and
the remaining coverage. These are harness selection corrections, not product
test failures or overlapping tests counted as new coverage.

The independently retained ordinary controller SHA256 is
`e3efc91348fdddc4206cee95e807366ffc5578d06a657b493785838933080687`
(10,205,104 bytes). Source archive and lock remain unchanged. The actual fe0
worker remains separately identified, not relabeled as current main. A read-only
Git audit confirms identical `crates/fe2o3-kfd` and `crates/fe2o3-runtime` source
trees at fe0 and refreshed main `071105ad4`; other compiler/artifact crates have
changed, so this is not whole-worker binary equivalence.

All 17 remote harness fixtures pass, followed by the actual binding of 22
source/build/worker phases to the retained executables. The complete native ABBA
then passes four fresh starts, each with one excluded warmup and three measured
requests. All 2,048 output IDs and streamed/final bytes match, with identical
five-command input streams, 135 batches and 87,711 dispatches per request.
Controller, worker and supervisor exit naturally without signals; final KFD
is empty. Both policies load the same nine images, use ordinary polling and
disable timestamps, prefix caching and speculation. Only the existing V19
single-row KV-copy selector changes.

| Order | Policy | Mean TTFT, ms | Mean TPOT, ms | Finite output tokens/s |
| --- | --- | ---: | ---: | ---: |
| AB | Baseline | 813.466834 | 63.979433 | 14.319103 |
| AB | Parallel V19 | 909.651647 | 59.537178 | 15.109862 |
| BA | Parallel V19 | 820.866945 | 53.357554 | 16.847425 |
| BA | Baseline | 810.629242 | 63.903010 | 14.339139 |

TPOT falls 6.9433% in AB and 16.5023% in BA; finite-window output rate rises
5.5224% and 17.4926%. TTFT instead worsens 11.8241% and 1.2629%. Candidate
variability is material: the AB request TTFTs are 829.62, 1,081.09 and 818.24 ms;
all remain included, without assigning an unproven cause. Its TPOT varies
55.01 to 62.76 ms, versus 52.95 to 53.57 ms in candidate BA. This is two fresh
starts per policy, each with three measured requests, not statistical confidence
or a sustained serving result. **Decode improves in both orders, but TTFT does
not; the selector remains default-off and this is not a vendor comparison.**

Independent retained-data audits check raw tokens/bytes, nine image identities,
selector-only arguments, exact schedules and process lifetimes. A separate raw
timing audit recomputes every request and excludes only the four planned
warmups. Root independently recomputed all four timing/rate rows directly from
raw request events. The archive and extracted files match identical remote
before/final inventories: 332 files, 72,384,873 payload bytes, archive SHA256
`0cd82eae2e94afd821f048bfe70d770aef279f7cfe8e57bbb1505187fb8ee340`.
The [V19 report](../../ferric-perf-swarm-v6/proposals/perf-v11-v19-staging-r1/RESULTS.md)
and [timing audit](../../ferric-perf-swarm-v6/products/perf-v11-v19-native-a001/timing-a001.json)
retain the complete cohort. Guarded retirement removed the native stage, then
refused the inputs root on process-census churn. That refusal remains retained.
One bounded retry passed the unchanged checks and removed the inputs and upload
roots. The final retained inspection confirms all three roots absent and no
all-UID KFD users; only small retirement-control receipts remain. The final
inspection SHA256 is
`cbf57cbba39163e02c76e930d64ac7d35f6752c3668fe0e8355717a67169132f`.
No safeguard was weakened.

The next kernel priority is full-model qualification of packed-down with its
packing cost and additional memory included. Existing MFMA kernels already
admit one logical row with inactive lanes zero-filled, so a future role-only
screen needs no fake scheduler rows or padding copies. However, historical
all-layer MFMA lost to C1 Wave, small output widths have few workgroups, and
MFMA accumulation order differs. No blanket MFMA switch or speedup is assumed;
see the [source-only feasibility note](../../ferric-perf-swarm-v6/proposals/perf-v11-kernels-r1/NEXT_KERNEL_EXPERIMENTS.md).

### Published Site

The completed prefill checkpoint is live at
[Ferric](https://harsh-nod.github.io/ferric/). Remote validation covers eight
viewport widths; five corrected desktop/mobile/narrow screenshots were reviewed
for complete readable coverage. An initial screenshot capture with a focus
overlay and incomplete scrolling remains a rejected receipt, not a visual pass.
The corrected capture changes only the capture helper, not the site source.

Deploy-only `pages/prebuilt` commit
`f8de8075fb5156ae09559e58834a0bc208206101` succeeds in
[run 36919533626](https://github.com/harsh-nod/ferric/actions/runs/36919533626).
All seven live static files match the remotely validated artifact over ordinary
certificate-verified HTTPS from mi300x. No GitHub-hosted build, protection change
or publishing worktree was needed. The published entry reports the prefill
experiment; it does not yet report the packed-down or completed V19 results.

A second static checkpoint containing those results passes all remote checks
and eight viewport sizes. Root accepts five focused desktop/mobile/narrow
screenshots, including the TTFT regressions and finite-cohort limits. Its
seven-file artifact is 881,118 bytes, SHA256
`fdeb4d42e09e0e08c54d07001dcd9197cfc4f11d04bbc7929a7562d222c7f51a`.
The exact deploy-only candidate is
`628b96386ef58a215bd7f642dc96c7a1546298c7`, but **is not published**:
the first read-only GitHub API request failed on network connectivity before
any remote Git object or ref mutation. No workflow ran. The accepted candidate,
failure receipt and [recovery note](../../ferric-perf-swarm-v6/proposals/perf-v11-site-kv-copy-r1/PUBLICATION_STATUS.md)
remain retained; no local build or network bypass was attempted.

### Custody And Retirement

The [prefill report](../../ferric-perf-swarm-v6/proposals/perf-v11-prefill-r1/RESULTS.md),
[paired comparison](../../ferric-perf-swarm-v6/products/prefill16-campaign-a001/comparison-a001.json)
and [independent audit](../../ferric-perf-swarm-v6/products/prefill16-campaign-a001/audit-a001.json)
retain per-arm values, source/binary identities and unchanged full inventories.
The [full raw archive](../../ferric-perf-swarm-v6/products/prefill16-campaign-a001/full-campaign.tar.gz)
is SHA256 `a2e326984a565469d8cbba2a18504e774dee51a13e7fe70a0c9e6d47ab8f348d`
(102,059,271 bytes). Source custody binds the host build, not new authority for
the unchanged historical GPU images.

Prefill cleanup remains refused, not complete. Two exact-root attempts stopped
before deletion because the unchanged eight-round all-process census did not
converge under transient process churn. Empty all-UID lsof output and a free
`native.lock` did not replace that required check. All ten campaign roots and
the small retirement-control directory remain; both
[first](../../ferric-perf-swarm-v6/products/prefill16-campaign-a001/retirement-a001.stdout)
and [second](../../ferric-perf-swarm-v6/products/prefill16-campaign-a001/retirement-a002.stdout)
refusals are retained. All campaign sessions have ended. No safeguard was
weakened and no further cleanup retry is part of this update.

The separate [vendor report](../../ferric-perf-swarm-v6/proposals/perf-v11-profiling-r1/VENDOR_RESULTS.md)
links the raw trace and audit; its archive SHA256 is
`8f6d0f32bf5a2c5fc10478e901e1554e3c1978458d4159e8ca68b6a6075fa45e`.
That vendor stage was successfully retired after custody; it is distinct from
the still-retained prefill campaign.

## Measurement Rules

- No local builds or project tests. Use the existing bounded remote CPU profile.
- Serialize native runs; record host, GPU, binaries, images, workload and selector.
- Validate correctness before timing. Keep warmups, cold starts and measured
  samples separate. Use both execution orders and retain regressions.
- Change one variable per comparison. Report TTFT, TPOT and finite output rate
  separately; do not substitute kernel/controller microbenchmarks for model gain.
- Preserve compiler/runtime ownership in fe2o3 and kernel/engine ownership in
  Ferric. Keep safeguards and failure semantics intact.
- Retain required raw results before removing owned temporary stages. Do not
  touch unrelated processes, borrowed model files or existing user changes.

## Acceptance Targets

The prefill proposal meets its 80-versus-576 ordered-group target with unchanged
flattened GPU commands. Its measured TTFT result remains limited to the finite
cold, instrumented cohort above; default promotion needs broader evidence.
Packed-down must include activation packing in timing and disclose its extra
3.375 GiB resident weights. Profiling must distinguish shader execution, packet
processing and CPU/GPU gaps without treating overlapping host scopes as GPU time.
