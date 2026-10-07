# Performance Swarm V7

Started 2026-09-22 UTC. This campaign implements the user-approved attribution-first
plan. It is not an M1 completion, formal proof, or vLLM competitiveness claim.

## Objective And Baseline

First identify the cost of a complete Qwen3-8B decode step, then optimize its
measured critical path. The retained three-pair TP1/C1, 128/128, BF16-with-FP32-head
baseline has Ferric TPOT 63.833..79.535 ms versus vLLM 4.228..4.394 ms. Prefix caching
and speculation are off. V7 now has two small native KV-copy campaigns with
lower TPOT in all four paired comparisons. Absolute timings drift between
campaigns and the first campaign has slightly worse TTFT; neither is a vendor
comparison or a stable-gain claim. Defaults remain unchanged.
See [V6 evidence](M1_PERFORMANCE_SWARM_V6.md).

## Team Progress

| Track | Implemented | Verified | Next |
| --- | --- | --- | --- |
| Runtime/dispatch | Host/counter and nine-image packet diagnostics integrated; fixed-091 ordinary worker built separately | Historical 2585 diagnostic executions/native replay pass; new worker's ten CPU phases and independent final raw audit pass | Qualify the new worker's native smoke and coordinated compiler/SDK before promotion |
| Measurement | V19 composition, prefill32 AB/BA and separate parity harnesses implemented | Mixed-generation CPU proof audited; R4 harness passes 31 fixtures; current native capture and independent raw/custody audits pass | Preserve raw evidence; require uninstrumented full-model AB/BA before claiming a candidate gain |
| Kernels/prefill | Capture overlay/full-N workload on pinned 413; isolated V19-preserving setup/upload successor | Historical compiler/workload/ISA gates pass; packing and 16 parity cells pass before a foreign-job stop; all eight setup tests pass with independent audit | Complete native timing and full integration qualification before model timing |
| Integration | Exact native plan and ISA bindings assembled; evidence transferred without a duplicate CPU-stage copy | Approved 20 GiB profile, 80 native-harness fixtures, 14 bundle fixtures, inventory replay and frozen copied-byte admission pass | Obtain uninterrupted native window; audit failed run and latest upstream; remote site QA/publication still pending |

### Packed Gate/Up Qualification In Progress

The user requested continued work toward a matched vLLM win after the accepted
packet attribution. Three parallel lanes now own latest producer/workload,
V19-preserving model integration, and guarded CPU execution. Root owns the pair
campaign driver, independent review and final measurement integration.

The selected GitHub main revision is `413ba987b8878f53e13127fdb53f9e1495ab93b3`. The capture
successor preserves its ordered-origin additions in all three overlapping
extractor/backend/driver files. Only the existing eight capture paths differ
from upstream; default-hunk added/deleted overlay lines match the prior bcd
overlay, and source diff checks pass. The fresh 413 build results are below.
Full-N packed arithmetic remains unchanged; SDK and producer identities are
refreshed in an isolated workload successor.

The isolated model proposal preserves V19/V27/split8 and the same admitted
images/storage in both arms. Its source expectation is 87711 baseline versus
92283 candidate packets over the same 135 batches and 1973 ordered groups.
The candidate adds one activation pack per layer per decode. No default or
accepted frozen controller changes, and no future image identities are invented.
The campaign component authors a complete worker lifetime for one all-bit pack
cell, 16 paired finite-parity cells and 16 same-cell ABBA timing cells; its fourteen
CPU fixtures pass remotely with exact source/roster and clean guard review.
Real image admission and the outer native launcher
must be qualified before it can execute.

Fresh CPU census is 13207314432 allocated bytes, already above the original
12 GiB cap minus reserve. Cleanup still cannot inspect the owned session daemon;
no exception or deletion is applied. Historical 431 preparation/build/capture
tests alone grew by 3409674240 bytes, before remaining producer qualification.
The user explicitly approves a temporary 20 GiB cap for latest compiler and
packed gate/up qualification. The isolated successor retains four cores/jobs,
8 GiB observed RSS, nice 19, one build lock, 1200 seconds per phase, the 512 MiB
reserve, external-library accounting and every host/cleanup check. Historical
12/14 GiB guards and results remain immutable. The new profile passes all 30
remote fixtures, including exact-cap boundaries and cross-generation rejection.
Independent raw/source/guard review passes; bootstrap archive SHA256 is
`98e5f28d866cdcf9eccc8a73ad39968692ed0c2f264d665c8e04fbcada3b22c3`.
The separate 14-fixture campaign record is
`E6/products/packed-gate-up-campaign-cpu-v7-r1.tar`, SHA256
`28963a6d48c7133670106f50bb2a443c2d3e008e9211a74388d11a6682c38085`.
Its exact guard SHA256 is
`6ac4a407c1476cc75359e77dc3985b4be412cc19d6d6d3791306eb12ad761aa1`.
Both are CPU component qualification only, not compiler, image or model proof.

The 1071714-byte exact-413 offline SDK bundle is retained at
`E6/products/fe2o3-sdk-413-packed-gate-up-r1.bundle`, SHA256
`295035f3b3441813ca4823744c9d001a0bf4d7ce721de9072110f0d9abafeed8`.
It advertises `refs/ferric/sdk-413-packed-gate-up-20260923-r1` and requires both
13e00263000362532d72a5dde52749349a9af122 and its ancestor
63d53606a67af457794dbc1362c4599024437bb7. Both prerequisites must pass actual
remote bundle verification before guarded import. Source audit expects the
same 163-package SDK lock cohort, including 53 core packages and two Pliron
packages; actual offline Cargo lock/metadata qualification remains pending.
Producer recipe source review passes independently. Its frozen manifest is
`6d9d0f65fb5609a1b67a335052779631d7c3ac653d2965721c98ba89866b7524`.
All 12 recipe fixtures pass remotely. Preparation verifies all 6762 source
files, applies formatting only within the eight permitted capture paths,
passes two final format checks and reopens the exact locked/offline compiler
closure with three Pliron packages. Final formatted source inventory is
`bed542de3b226ecdf7d6548dc5b1a69486b55f1eee84f40d8805c327fb767ecd`.
Strict scoped CLI Clippy passes with `--no-deps -D warnings`; peak observed
RSS is 1687207936 bytes and final stage usage is 13674020864 bytes. All three
guards are clean and unsignaled. All 19 compiler CPU stages now pass. The actual
CLI is `5f26667ef82f47f562e06249e0c02c17911658572d2a8d5238c7b6f464d16274`,
backend is `f96e0497e3752279661bae3ab858b6f341451be4ebc0b7099f863a36efb0d367`,
and extractor is `d8a51a2a6bbcd08857f63f945f132d4e65559bd46260f1e00b384ab9278f76e6`.
Focused CLI/backend/extractor executions pass 10/7/2 cases. Regression runs
pass 27 CLI, 223 backend driver, 95 additional current-source backend and
26 extractor cases. The driver retains its 117 existing ignores; source policy
passes five cases with its one explicit inventory-refresh maintenance ignore.
The final backend build observes 18070495232 allocated stage bytes, within the
20 GiB cap and unchanged reserve. The retained raw CPU record has SHA256
`b2360bcbb9eb7fc7addb9026b7ddabbafd42cc36d7dc8b051441ce82568b8686`.

The separate current-warning collector passes all 16 unique unskipped fixtures
and admits the producer after reopening all 19 stages. Its 336952-byte producer
record is `a1af515816ce66a6e4ef6a5f0070f41567d0f3afe9c92724f6e01fa26ed7dc2f`.
The policy binds all 32 exact raw diagnostics (31 backend and one extractor)
and their current source, not the old warning allowlist. Strict CLI lint is
accepted; strict backend/dependency lint, emission, native and proof authority
remain false. Independent final raw-evidence review passes: all 21 guards,
390 raw bound files, exact test listings/outcomes and current warning streams
match. The complete raw archive is
`E6/products/capture-cpu-producer-413-raw-v7-r1-complete.tar`, SHA256
`938da1d418aff30c2b0493effb7e3d2429880e90b489037afbd69ec2d07e1100`.

All ten workload CPU stages pass, including 34 Rust tests, six component tests,
20 recipe fixtures and strict release all-target Clippy for both source arms.
No test is ignored, filtered or skipped. Actual offline resolution preserves
all 163 package versions/checksums/edges/features while replacing exactly 53
core source IDs. Independent review also checks all ten clean guards, 773 raw
bindings, four actual test ELF copies, all 20 formatted source/lock files and
36 fresh metadata graphs; the V5 oracle remains byte-identical. The actual
qualification is `2f4f4c421beeb7be40272712a866dabed072a5850eda2cffce3668f54a44db86`.
Complete custody is `E6/products/packed-gate-up-cpu-413-v7-r1.tar`, SHA256
`c18a8da550ecd51649a3fb782a9a2d9691d7f40f58da9a94666bd4387b2236d1`.
The emission recipe passes all 25 fixtures and independent raw/source review.
All five actual emission stages also pass. Independent artifact review reopens
1206 bound files across the compiler/workload/emission archives, verifies all
five clean guards and both images with their five sidecars. The baseline image
is `07335b3e9866e9bb58e6b97d8f0489928b2ad5b6043a086ef578f5ac037edcca`
(10912 bytes); candidate is
`2db621ccb8f533228283e659d365dd93e9dbd2960b49346f54420c12b9887928`
(13904 bytes). Complete emission custody is
`E6/products/packed-gate-up-emission-413-v7-r1.tar`, SHA256
`57a36353ebeeac1597ffaf80322a4b09e0dd25ac8e4ef85a684fece0129e027d`.
All four stable-toolchain host-roster stages pass, including 12 fresh fixtures.
Actual API order is the baseline projection alone, and candidate projection then
activation pack. All eight read-only ISA/descriptor captures pass; independent
manual review finds no static blocker. Actual packed dword loads, low/high
multiply/add order, six-step reduction, finite checks and denormal-preserving
final descriptor modes agree. Geometry-lowering FMA is explicitly separate from
payload arithmetic. The raw ISA archive is
`E6/products/packed-gate-up-isa-413-v7-r1.tar`, SHA256
`b1f93885249a13a1d7bd114c990e0788381a8b3d8e5dec6350b04acf02eaf43b`.
The native R2 harness passes all 80 fresh methods (22 contract, 27 retained
evidence, 17 pair and 14 campaign), with exact source/guard/raw-log audit.
Qualification SHA256 is
`d09a9db64a29507f1ec1cf9394b976eb05f1d8f429633197a627a82f0cbeaece`;
complete archive SHA256 is
`6e00f31648ec6b2a5f0e8dfac2dfe3e6adeb41da0512a0052aebc60d11c3a70c`.
The bundle's 14 fixtures and actual-data inventory replay pass. Exact inventory
contains 2,441 logical paths and 1,378 unique objects totaling 1,607,497,581 bytes.
Archive SHA256 is `623a1cd584a8db5d635932e746bdba6c45bd205cf22b11f7b0d2e594be3dcf9b`.
Duplicating these on the build host would exceed approved CPU-stage headroom by
325,824,512 bytes. No cap or cleanup check is relaxed. Root changes only transfer
ordering: stream the pinned bytes, then run the frozen read-only native admission
on mi350 before any worker starts. The separate 2 GiB native cap remains unchanged.
Actual stage `/tmp/ferric-opt-v5-pair.ZAKjrVaQ` has a twice-reviewed plan,
SHA256 `d82f8c3838fb5efd6c4b21814de4e27315a22d2bfbf6bc0a0cde7c5a1bac75a4`.
The complete 1,609,728,000-byte object archive is retained locally with SHA256
`d064d99f33ec3a58db5442217da780318416dec8c3c0d5a0baf457c9cd091afb`.
Interrupted transfers remain failed records; the eventual frozen read-only
admission reopens all copied bytes and passes independently before GPU launch.
Its stdout SHA256 is
`b19afa9f6d1511e94787e61b231532af75257c0eb93713fff7260795a4fdc3ae`.

The first native attempt runs from 01:33 to 01:40 UTC on 2026-09-24. Packing
checks all 65,536 input patterns, and all 16 finite-parity cells pass with clean
individual worker exits. Cell 17, the first timing cell, is interrupted during
warmup buffer validation before any measured interval. The load generator
restarts on the host: a fresh privileged read-only inspection identifies foreign
KFD PID 2474999 as `/opt/scout/worker.py`, with eight new `/tmp/worker.py` children.
That supervisor is not signaled. The unchanged native guard fails closed on
`PermissionError` inspecting that root-owned process, sends TERM to its owned
runner, reaps it, and retains status 125 with cleanup_ok false. Cell 17's owned
worker also exits under TERM and is reaped. No timing ratio or complete campaign
acceptance is inferred from the successful prefix.

Complete native logs, actual inputs and raw cell protocols are retained in
`E6/products/packed-gate-up-native-ZAKjrVaQ-r1.tar` (8,427,520 bytes), SHA256
`966737386790c6c996687c2248900c5d9767d8b5e14d3f87f38aa4a13712e47c`.
The 142-file archive omits only the separately retained object store. A 25-minute
uninterrupted window has been requested through the load generator's scheduler.
Numerical-trap, complete native, model and performance qualification remain open.

Upstream advanced during qualification. The fixed currentness snapshot is
`c859548ab9487e824b644ed2b9aa92cab86ae46c`, three commits and 33 changed paths
after 413. The actual worker's nine dependency crate trees and all eight capture
overlay paths remain unchanged. The broader runtime adds a partial-workgroup
refusal; this protocol uses complete groups, and that function is outside the
worker closure. New inert device metadata changes global trusted-provider
identities, so a future refresh must rebuild compiler and SDK together. This
run stays an explicitly pinned 413 engineering comparison, not c859 qualification.

A later source-only audit observes main
`091856733e7053f9f50b7b205f85d4654d467e65`, checked twice on 2026-09-24 UTC.
There are now 60 commits and 590 changed paths after 413, including 22 changed
SDK crate trees and seven manifests. The eight capture-overlay base paths still
match, but transitive compiler/IR/ABI code and the worker's KFD dependency have
changed. In particular, ordinary KFD load factoring and process/fork gating have
changed; c859's nine-tree runtime equivalence cannot be reused. The next
latest-main qualification must refresh compiler, SDK and worker together, with
fresh resolution, emission and native validation. No old pass is relabeled.
The source audit is `E6/proposals/perf-v7-latest-main-091-impact-r1.md`, SHA256
`937085f04b4c5b25b6eac0c348e99b38f6012479b861d76387699f4d35e6f945`.

Further isolated model review finds no concrete routing or packet-count defect,
and all 12 integration preimages still match the active Ferric tree. Its
recording fixtures bypass production packed-weight setup, so the authenticated
72-matrix upload and partial-failure cleanup still need targeted coverage.
Fresh emitted images must also pass the actual ignored artifact-opener test.
The model controller must be built with `c1-ordered64` only; timestamp-union
builds intentionally reject this profile. No new native performance result exists.

The targeted CPU lane now passes all 12 recipe checks on mi300x-2. Its frozen
1324-file source passes inventory/preimage checks, scoped formatting and locked
offline metadata resolution. The ordered library compiles in 50.60 seconds;
the clean unsignaled 20 GiB guard observes peak RSS 1653100544 bytes and final
allocated stage usage 19855949824 bytes. The retained predecessor binaries pass
the pre-cache check. Preparation custody is
`E6/products/packed-integration-preparation-custody-a001-r1.tar` (3072000 bytes),
SHA256 `d7849c2c78828911ed9c41bf2c0d4a428819deeeaab6b5891d3b68b09dbdc2e0`.
All eight exact setup-method executions now pass. Each invocation reports one
pass, zero failures/ignores and 493 deliberately filtered methods from the
494-name actual library listing. The actual 19150744-byte independently retained
ELF is `267bd21a87eb977de675fb9b91aac9a0ba40e8a87f5f7ae494809026d5ae6086`.
All 67 historical keeper records (857434302 bytes) match exactly before and
after cache reuse, including their file identities. Complete raw custody is
`E6/products/packed-integration-setup-custody-a001-r1.tar` (23050240 bytes), SHA256
`1fc1a5f47e818ec9714132ed745ceab4a6bce11bc2d30e0c1dfd27dd414b8f06`.
Independent final audit passes over all 159 regular files, 16 clean serial
guards, exact launches, actual test roster/outcomes, retained ELF, source
bindings and unchanged keeper identities. Audit note:
`E6/proposals/perf-v7-packed-setup-targeted-actual-audit-r1.md`, SHA256
`1de9aed1d3e2cd3c0ce260b750d8c49bd5bcd7e50a0e641ba2884235b486ba70`.
The recipe-tests launch is reconstructed from its retained actual guard because
no standalone local request file was saved. This is eight-setup-methods-only
evidence: the historical integration bridge does not qualify latest main, the
full feature matrix, a native controller or any performance gain.

The source-only coordinated-refresh plan maps the reusable compiler, SDK, worker
and native recipes to fixed main `091856733e7053f9f50b7b205f85d4654d467e65`.
Real dependency-edge changes rule out revision-only substitution, and typed
host/device boundaries and three KFD aliases need a coordinated migration.
No packed ABI or wire incompatibility is identified by source review alone.
The plan is `E6/proposals/perf-v7-refresh-091-action-plan-r1.md`, SHA256
`db7eec44ecda89ee7c9982cc163b078bd3da528336eb7cf9e280011a7ff02411`;
it does not itself imply execution or qualification.

The separate plain-091 ordinary KFD worker now passes all ten remote CPU phases
under the unchanged four-core 20 GiB owner. Twenty recipe fixtures pass. The
actual retained library lists exactly 571 tests: 570 pass in the unfiltered
suite and the one established hardware-only test remains ignored. Thirteen
timestamp-focused reruns and all 31 doctests pass. Strict library/test Clippy,
the original ROCr 6.4.3 ABI oracle, release worker build and final retention pass.
The new ordinary worker is 1798544 bytes, SHA256
`42f9bc93d54b4f59ad5800539e16d7bb763c677e19ecd0925a72406a1e9c178c`.
Its 105183-byte V2 CPU receipt is
`de418f3d8d43451b01759e7c6a637cab13b48b93ef4da0cc9d33849c1d6d008c`.
Complete custody is `E6/products/core-worker-091-cpu-custody-a001-r1.tar`
(33310720 bytes), SHA256
`d8f16bb0bda4bfc73407cd870b1e0a0fc26b95a55ab26bed47c02071da977eda`.
All 68 predecessor keeper records (876585046 bytes) are identical before and
after cache reuse, including file identities. Peak sampled RSS is 1139507200
bytes; final allocated stage is 20190535680 bytes. All guards complete cleanly
without signals. Independent final raw audit passes over all 174 regular files,
14 clean serial guards, exact launches and outcomes, both retained ELFs, all 44
V2 file bindings, source/tool bindings and unchanged keeper identities. Note:
`E6/proposals/core-worker-091-actual-cpu-audit-r1.md`, SHA256
`1631d7fa7c4d6ad1e4ce23583dc13b93a915124540a2e30fcbbdbbcfd71ae63f`.
The 31 doctests are compile-fail cases. Cargo retains its existing duplicate-target
manifest warning despite successful strict Clippy; inherited SIGABRT self-test
headers receive no extra pass credit. The new worker
is not admitted by historical V1/native consumers: its native smoke, coupled
compiler/SDK qualification, model integration and performance remain open.

The post-worker read-only MI350 availability check completes at
2026-09-24 02:56:24 UTC: all eight GPUs report 100% utilization. This SMI-only
capture does not identify the current processes' owners or command lines. No
GPU run or signal follows it. Raw stdout is
`E6/products/mi350-post-worker-091-availability-r1.stdout`, SHA256
`4f333e64dc924e7122b6cf28e5c8c2b850a50a794e0bd3d91ee58188904ca5e2`.
The obsolete 19589120-byte local source-transfer envelope is removed after
exact bytewise reconstruction from complete actual custody. Its retirement
record is `E6/products/core-worker-091-transfer-retirement-r1.json`, SHA256
`158b62772d6d8b601c839930206efdf4b76d7e46a333ce600834b24acbddfda0`.
No source, retained binary, raw run record or remote-stage file is deleted.

### Current Prefill Checkpoint

R1 ordered-library testing reports 458 passed, one failed, 11 ignored and one
filtered. The rollover fixture did not fill the real 131072-packet ring; the
minimal correction changes only that test, not production scheduling. Failed
R1 evidence remains at `E6/products/prefill32-failed-ordered-v7-r1`.
Fresh R2 preparation, feature checks and strict Clippy pass. The ordered library
passes 459 tests with zero failures, 11 ignored and one documented exclusion;
four CLI suites add 351 passes. All 14 focused tests, the 72-test plain-feature
suite, six actual-image inspections and 22 measurement fixtures also pass.
The combined-feature library passes 467 tests and four CLI suites pass 316,
for another 783 passing executions, 43 ignored and one documented exclusion.
The policy suite reports 49 passes and one stale batch-only preflight assertion.
A reviewed test-only fix now requires the actual selection-aware preflight
before submission. Its separate amendment preserves the original R2 source and
failed result. Controller build a001 stops at the stage reserve with status 125;
its guard and incomplete output are retained, not selected as a pass. Fresh
a002 passes after cleanup with the same limits: 38 seconds, peak RSS
1919512576 bytes, clean unsignaled teardown and controller SHA256
`06a6605c0879e9b30cd6a1c5cac2d79362f50710d9b9f6d120db71d2ca55928b` (10332408 bytes).
The separate policy amendment passes all 50 original names, zero ignored or
filtered, under the same guard. Its source inventory is
`c6da8192bba2ece27dd839434c0391ed802b644e3bf76e4bf31bfbfb0e447775`, differing
only on the policy test from the original 1312-file controller source inventory
`4dc0fd38206f83ef6d31403c499dc42a309bc969e96399ce1ceb868a26d97d91`.
Formatted policy SHA remains the reviewed input
`f3c49dd5eaf09fd75d0e74e4979e11e193cdf40d7a7c5af365d89b5fafe0901f`.
There are 1735 passing Rust executions across feature variants and explicit
focused/image reruns, not 1735 unique tests; 94 existing ignored executions
remain, and each library run keeps its sole documented resident-roster exclusion.
Final amended collection passes with clean unsignaled teardown. The accepted
receipt `E6/products/prefill32-policy-review-v7-r1/build.json` hashes to
`f51a854e0429abd9e0c40e6878e8bc48fcd4199c6dc91579a56fd6be974620b2` and contains
all eleven actual passing gates, original controller/source bindings, the
one-file policy amendment and its retained original failed attempt. Collection
guard SHA256 is `12879a0f556309ed0b5ca6d1d5a958f0d17a96b7b820e5115934853682cb0620`.
`native_qualified` and `performance_gain_claimed` remain false.
Independent artifact review matches all eleven embedded guards, nineteen Rust
test/ELF records, command/log bindings and the 22-fixture stream to retained
evidence. A comparison before packet-tick integration against all 1312 amended
source paths found only the two progress documents changed. The later diagnostic
source is a separate snapshot and does not relabel the qualified controller.
Its source archive `E6/products/prefill32-source-v7-r2.tar.gz` hashes to
`12a2235b94457be6c40b484251899bd625d4317ade30c5e27dab9babd16f0c16`, and input inventory
`E6/products/prefill32-source-input-v7-r2.json` hashes to
`facd30b15c6457c6914cc5283d728286ec83a9962e24652d8e1bb3d7ebccccd4`.

The separate two-half V27 parity harness passes all 22 actual CPU fixtures with
clean guarded completion. Receipt `E6/products/prefill32-parity-cpu-r1/harness-cpu.json`
hashes to `3bec4ce2b50539a65729a6015eeac04376c82ee2321fae16c77e34c8229916dc`.
Native launch at `/tmp/ferric-opt-v5-prefill32.PY2giCjE` returns 125 during
frozen KFD admission, before Popen or any parity worker. Reading
`/proc/3065063/exe` raises PermissionError; the process is root-owned
`/opt/venv/bin/python3 /tmp/worker.py 4`, and rocm-smi reports all eight GPUs at
100% utilization. No parity cells ran and unrelated work is left untouched.
Admission-failure custody is complete at
`E6/products/prefill32-parity-admission-v7-r1.tar.gz`, SHA256
`9f7362f4bc844e42f9fe930d42b0bbe0306c7bc6fd01791bed3cfe44cb7e7512`.
The owned remote stage/archive were removed. On September 23 the jobs initially
exited without intervention; a fresh parity run in `jCKSdzmQ` then stopped when
the same eight `/tmp/worker.py` jobs relaunched. Its 16 completed cells are not
an accepted campaign. Under the user's explicit authorization, root verified
the exact eight argv/parent/UID identities and sent TERM through held pidfds.
All eight exited; no KILL or broad process-name signaling was used.

The fresh unchanged retry in `/tmp/ferric-opt-v5-prefill32.1mdgmy7b` passes all
20 cells with clean unsignaled worker and outer exits, empty postflight KFD,
identical before/after inputs and matching six-allocation readbacks. Report
SHA256 is `d8a5df743cd9d30ca714d02e8aa7a24774b36067bda80afbbe3c633deb825508`
(255562 bytes); supervisor SHA256 is
`3a381a2e184f9f9a1295984821da2f7882b235969732ee453bcf7d3c3397c277`.
Independent review checks all 99 report-bound files and all 13 CPU-qualified
helpers. This is isolated byte/guard parity, not full-model or timing evidence.
The completed full-model AB/BA run is in `/tmp/ferric-opt-v5-v25.2IGl9SCl`, plan
SHA256 `1c423c76c84231b00e1d6c5ce8c8f884c52cb1932c550321d959d832fafbe760`.
Both arms use chunk32, fixed V19 decode, split8 and ordered64/packed64; only
V5 prefill append versus two V27 page copies changes. Each of four fresh starts
excludes one warmup and measures three 128/128-token TP1/C1 requests. All 2048
output IDs and decoded bytes match. Context8192, BF16 with FP32 head, prefix
caching and speculation off.

| Start | Mean TTFT (ms) | Mean TPOT (ms) | Finite-window output tokens/s |
| --- | ---: | ---: | ---: |
| Baseline AB | 2041.379490 | 65.645799 | 12.332303 |
| Candidate AB | 565.331427 | 70.618747 | 13.424622 |
| Candidate BA | 597.045942 | 66.836311 | 14.086379 |
| Baseline BA | 1954.125291 | 61.811245 | 13.054065 |

TTFT falls 72.306% in AB and 69.447% in BA, but TPOT worsens 7.575% and 8.130%.
Finite-window output rate rises 8.857% and 7.908%. Keep the orders separate and
the option disabled by default: this is not HTTP, sustained throughput, a stable
gain or a vendor win. Independent review replays all tokens and 2096 batches,
checks all artifact bindings, and recomputes every mean and ratio. Over 99.67%
of excess decode interval time is inside host batch start-to-completion, not
between batches; this does not establish GPU time or a cause.
Report SHA256 is `5ef8c2f2e5a313e16c69401a72f633b9539afcee285d407fe94b65843693bfdc`;
outer supervisor SHA256 is
`3443cba3e4030e19900e50d4e0875c14eb1a53a6cd3ade10c33503787a56853f`.
The outer exits unsignaled with status zero and empty postflight KFD. Inner
reserved-group cleanup retains TERM/KILL flags for exited controller zombies;
those records must not be relabeled as unsignaled worker exits. Full custody is
`E6/products/prefill32-native-abba-v7-r1.tar.gz`, SHA256
`d9c5c06c56468045ddcc8b66122adbb5f0f0cf0a7f3aafe54ece57bbbed2a6f6`.

## Boundaries

CPU builds/tests run only on mi300x-2, serialized under the approved four-core,
8 GiB RSS, nice 19, 512 MiB reserve, 1200-second profile. The user approved a
temporary 14 GiB private-stage successor for the now-completed diagnostic, then
a separate temporary 20 GiB successor for latest compiler and packed gate/up
qualification. Historical 12/14 GiB receipts remain unchanged. GPU work
runs on mi350 with fresh idle/resource checks and owned teardown. No local
build/test/project import, GitHub build or new worktree is used.

Core/compiler/runtime belongs in fe2o3; kernel/inference/measurement code belongs
in Ferric. At the completed diagnostic checkpoint, observed main was
0fa063fd17921d24f955479bb4736d0a8beb1cf2. Its 96 changed
paths after bcd add debugger/runtime observations and simulator lifecycle
behavior. All nine worker crates are unchanged from built-at 5e2; manifests,
root lock, toolchain, host SDK and eight capture-overlay paths are unchanged
from bcd. This is source-only audit, not new build/emission qualification.
The previous bcd checkpoint's 75 changed paths after f4 add device ordered-program diagnostics
and change the backend trusted-source closure. All 75 upstream objects/modes
survive the source-only overlay rebase; all eight overlay paths are unchanged.
The complete nine-crate runtime closure remains byte-identical to built-at 5e2;
Cargo manifests, lock, toolchain and host SDK are unchanged from f4. The KFD
tree remains c2382af8dc7055f26beb4f0f33bf53a4c383bd94.
The preceding dc48 changed the compiler prefix helper, tests, docs
and scripts, not the nine worker/runtime packages. The earlier 431 runtime trees were unchanged
from the measured 13e source, but the compiler backend changed.
The host diagnostic deliberately retains actual 13e controller and 5e2 worker
identities. The compiler track cannot relabel its actual 431 build as dc48.

## Diagnostic Contract

One cold 128/128 request uses 135 model batches under a 256-batch process budget.
The completed current capture confirms the source-derived 1973 ordered groups
carrying 87700 packets plus 11 direct prefill packets, and all 128 generated
tokens. This qualifies diagnostic replay, not performance.

The diagnostic records controller argument packing, transport round trips and
already returned worker duration. Worker time includes publication, GPU work,
polling and currentness fences, and excludes preparation/staging. It is not GPU
time. Nested spans are not added together. The analyzer separately partitions
serial intervals and reports setup/inter-batch/teardown exclusions.

The actual sidecar footer is checked and bound to the observed controller PID
and exact setup/close events. Live-stdin workload_sha256 is null; workload and
prompt identity remain bound through the admitted input and raw submitted
command, not an invented sidecar hash.

The next nine-image packet-tick proposal is refreshed against current F at
`E6/proposals/perf-v7-ordered64-packet-ticks-v19-r2`, manifest SHA256
`b3b8a43db4ad6e3d4e7de3e6c3a1033cb1ea12f44c4d162ff2782c5bcba144d0`.
Only its policy-test base changes from R1, preserving the selection-aware fix.
The eighteen-path change is now integrated, retaining ordinary default gates.
R1 remote preparation and all five feature checks pass; strict union Clippy
finds an import after statements. The semantics-neutral import move is in fresh
R2 input inventory `357eb90237b162f362ff536c0468c4aa21f68d064103a611ca827cb23a3dfd86`;
R2 preparation, feature checks, strict Clippy, ordered suites (818 passes) and
model suites (628 passes) pass. Union-library testing reports 476 passes, one
failure, 11 ignored and one documented exclusion. Its recording transport
expands every ordered packet into an individual Submit event, so the fixture's
expectation of only 11 Submit events was wrong. A reviewed test-only R3 correction
asserts 87711 total submissions minus 87700 ordered packets equals 11 direct
packets, preserving exact batch/group rosters and production behavior. R3 input
inventory is `1d0d58c2fa8608457ff5d16d102aa4cdab1735c362e91818051552a8b422f96f`;
archive SHA256 is `7f7e129e73858edb6d999fb9e8c6b6d64a506c40e8f36ae59201c52441819a38`.
R3 preparation and union suites pass: 477 library and 523 CLI executions,
1000 total passes, 59 ignored and one documented library exclusion. The corrected
test and both other new packet fixtures pass. The union guard exits cleanly and
unsignaled, peak RSS 2689585152 bytes, in 239 seconds. All seven focused reruns
also pass using the retained union ELFs. At that checkpoint, the selected gates are exactly
prepare, tests-union and focused, all a001. Release build a001 is admitted at
its unchanged 256 MiB growth floor but reaches the stage reserve after about
two minutes: status 125, returncode -15, TERM true, KILL false, child reaped,
cleanup_ok false. Its detached-cleanup uncertainty is preserved. All seven
recorded build PIDs and the recorded session are absent in the later owned
postflight; this does not rewrite the failed guard. No build pass or controller
receipt was selected at that checkpoint. Complete partial custody is
`E6/products/packet-ticks-cpu-r3-partial-through-build-a001.tar`, SHA256
`3f6b0128461e1a7be6d3d9f47ac8403f359b14b1c1710be741906ab28c386a7a`.
The remaining R3 gates were pending before the approved continuation below. R1 lint and R2 union
failures remain retained; the latter archive hashes to
`e62598affae6cf7cbddade62a0c173c3eecc6d40a3f1d8dbe4302ef4e4ecfbf1`.
The separate measurement harness R1 has 23 passes and one fixture error: its
positive temporary root inherited the CPU TMPDIR instead of required `/tmp`.
The successor fixes only fixture placement and exact provenance paths; no native
placement restriction is loosened. Fresh harness R3 passes all 24 unique,
unskipped fixtures and clean unsignaled test/collection guards. Independent
review checks its exact source/dependency, command, log and resource bindings.
Receipt SHA256 is `344667b8fc84b0b798d48e7167009cfd6599f6b4172a5dde84f49da289d489b9`.
Combined R3 and original failed R1 custody is
`E6/products/packet-ticks-harness-cpu-r3-and-failed-r1.tar`, SHA256
`1bad778e03914b09c048d11abf109dbea4a67bf49914bfb17b7ad26e50c30285`.
This qualifies only the measurement harness, not the diagnostic controller or
native attribution. Raw tags distinguish gate/up/down, attention partial/merge
and V19/V27 copies. Profiling adds currentness checks and timestamp readback, so
its TPOT is not matched performance and raw intervals are not additive GPU-time
shares. Native exact-output attribution must precede kernel prioritization.

### Approved 14 GiB Continuation

The user's temporary stage-cap increase is applied through an isolated profile;
all other limits and cleanup checks remain unchanged. Thirty resource-profile
fixtures and eight continuation-helper fixtures pass remotely. The original
prepare/union/focused a001 records remain 12 GiB; the other eight phases pass
under the new 14 GiB generation, with build a002 and all other attempts a001.
Five feature checks, both strict Clippy configurations, all 16 full-suite ELFs,
seven focused tests, seven image checks and 51 policy tests pass. The full
suites have 2520 passes, 137 ignored executions and three instances of the same
library exclusion; all selected Rust executions total 2585 passes.

The release build completes cleanly in 53.704 seconds. Controller is 10421592
bytes, SHA256 `53998fdba1c80aef0f3967b0edf835efe14884886fe27e238e207805d7dc412b`.
Collection a001 fails because the deliberately aborting child test writes into
the parent's harness line. Its failed source and raw evidence remain unchanged.
The separately named collector/parser successor requires the exact nested
block, panic stderr and resident-session source SHA, and accounts only the
three full-library transcripts. All 23 parser fixtures pass, including all 31
actual transcript replays. No raw output, Rust source, skip or exclusion changes.
The separate mixed-contract suite passes all 27 fixtures.

Actual V2 receipt is 1272254 bytes, SHA256
`92606ee41b9bbfda016136ce99d769ad0098004fdd0597952b329c63d33b3a96`.
Independent review checks all eleven actual guards, 78 commands and raw log
bindings, every executed ELF/roster, both preserved failures and the three
source-bound parser adjustments. Full raw custody:
`E6/products/packet-ticks-cpu-14g-through-collect-a001-v7-r1.tar`, SHA256
`16c0f4a46fd5b61c63f38dd17783b4c96a46c435ef682e4bb4fb4b54277fe49b`.
Successor and final receipt custody:
`E6/products/packet-ticks-cpu-14g-collector-a002-v7-r1.tar`, SHA256
`6dff690fc60aecae0531d13a37c9604aeddaaee66249a7d5e739ff1fde68c35f`.

The independently qualified R4 native harness passes all 31 fixtures. Its
11542-byte receipt SHA256 is
`f7e5a0c7953d5bca656abee1dcfeae8256b215db6fa8bf4ef54321a7830c177b`;
complete custody is `E6/products/packet-ticks-harness-cpu-v7-r4.tar`, SHA256
`6c0f2d18ed3be64a6a1cc5e70dd2a34b5e7fb203d7f05a6d67eab120519a5c99`.
R4 uses separately pinned actual 12/14 GiB auditors and the shared mixed-proof
contract. Its CPU-receipt-only reader has an explicit 2 MiB bound to fit the
complete method rosters; generic JSON/log, 12 MiB sidecar and GPU resource limits
do not change.

Fresh mi350 stage `/tmp/ferric-opt-v5-v25.z8FOGCwV` binds the actual controller
and both receipts, with plan SHA256
`9afc057b1240a83a9d2b6d296126fd1b95883408943da5c648e48836caafa248`.
Under the user's prior exact-worker authorization, eight newly reverified
`/tmp/worker.py` jobs (426537 through 426544) exit after pidfd-bound TERM. No KILL
or supervisor signal is sent. All eight GPUs then report zero utilization and
KFD is empty. The guarded capture then completes with exact 128-token and
decoded-byte replay. It contains 87711 packets in 135 batches, with 1397 decode
and 576 prefill ordered groups plus 11 direct prefill packets. Controller and
supervisor both exit zero, unsignaled and reaped; postflight KFD is empty.

Report SHA256 is
`3dad3bca93b3f6e448d5bd413f3ed718408b19dcdde24cd42cbc6585b0207567`;
supervisor SHA256 is
`5578a9019f42c2b028b1c02d3cc0cfa610f032eee9c15eeb1329670670963ead`.
The 4937188-byte raw sidecar has SHA256
`e9886a9a8370262577b47adfffc72d2c3619317a7a3458f1ce1dcfd0aa8f30d9`.
Complete archive `E6/products/packet-ticks-native-v7-r4-a001.tar.gz` has SHA256
`aae4f2542809942a30f3fd4087dade8f521a908f7690ad11af8611a4c558c9a7`,
matching a second independent remote stream. Independent interval/custody
reviews pass. Unknown-frequency packet intervals are not shader-only
durations, calibrated wall time, additive execution shares or a vendor result.

The raw audit reconstructs all 87711 records, all original boundaries, the full
27-symbol catalog and all 43 stored summaries without discrepancy. It verifies
all nine image manifests and HSACO bytes. Independent custody review also
checks the actual controller/worker, both CPU receipts, all helper sources,
the two-command/272-record transcript, terminal order and exact decoded bytes.
Current attribution note:
`E6/proposals/packet-ticks-native-r4-analysis-r1.md`, SHA256
`73f9f05b12f78cc47d25b4642117e5c7df1138bb391d11bc7398ad9882dcfd73`.

Selected decode observations, in raw packet ticks only:

| Role | p50 | p95 | Observations |
| --- | ---: | ---: | ---: |
| V19 KV copy | 740 | 880 | 4572 |
| V21 attention partial (kernel 24) | 8128 | 9724 | 4572 |
| V21 attention merge (kernel 23) | 936 | 1088 | 4572 |
| Gate projection | 14260 | 14500 | 4572 |
| Up projection | 14240 | 14500 | 4572 |
| Down projection | 21008 | 21304 | 4572 |
| FP32 head projection | 85964 | 86360 | 127 |
| FP32 argmax | 102452 | 104416 | 127 |

Gate/up/down remain stable across decode thirds and layers; attention-partial
median rises 6960 -> 8116 -> 9284 as context grows. Layer and group position are
confounded, and these strata are not causal boundary penalties. No common
large interval floor is visible. Head/argmax occur once per token, not per layer.
Do not turn interval sums or instrumented inter-group gaps into execution shares
or removable overhead.

The next candidate is qualification of the existing full-N packed gate/up pair,
including one activation pack and both consumers. It must preserve current V19
integration and account for the source-expected 36 extra packets per decode and
6.75 GiB extra packed weights. Neither BF16 weight bytes nor FP32 arithmetic
halve. Current producer/source, native packing/parity and equal-group pair
qualification remain pending, followed by uninstrumented token-exact full-model
AB/BA and only then a fresh matched vLLM comparison. Prior V20 partial-prefetch
model tests did not demonstrate a gain; a large down-projection interval alone
does not justify repeating that candidate. No production default changes.

After independent archive/member/source review, the completed native stage is
retired under ordinary UID9661. The exact 100-file/35-directory inventory matches
the retained archive and fresh process-use checks pass. Five uninspectable
owned processes receive actual read-only privileged exe/cwd/FD inspection,
without an identity-only exception or privileged deletion. Reclaimed allocated
file bytes: 24662016. Cleanup manifest SHA256:
`dc92d1c255d3085d459be010803e98691f579d15b0e40b3e8cee29bd5348e171`.
Result: `E6/products/packet-ticks-native-retirement-v7-r1.json`.
The model, reusable input stage and local evidence remain intact. CPU cleanup's
separate uninspectable-daemon blocker is unchanged. No new worktree or local
build/test is used, and no diagnostic subprocess is left running.

## Current Resource Checkpoint

September 23 cleanup removes four verified unused Cargo checkouts and completes
an exact 930-file non-ELF Rust-intermediate plan, reclaiming 2305048576 allocated
bytes. The first attempt stopped on a census/unlink race after 168 files; it
remains failed. The reviewed finisher removes only the remaining 762 entries
and passes cleanly, preserving 8822 other files. The full census, journal,
failed guards and successful finish are retained. All 17 independently bound
prefill controller/test/worker/inventory/toml keepers match before and after.
The stage cap and reserve are unchanged; no active worktree was deleted.

A later exclusive-lock cleanup removes 1728 exact files (398794752 allocated
bytes): archive-backed R1 source, 403 check-only metadata files and eight target
test-ELF duplicates. All 56 independently retained keepers remain byte-exact.
It is a bounded cleanup-only operation, not a build/test guard receipt. Its
plan/result remain at `E6/products/packet-ticks-cache-retirement-v7-r1`.

After complete local custody, the three completed native parity/model stages
are removed from mi350: 371 exact files, 37347328 allocated bytes. The initial
cleanup stopped before unlinking because the user's systemd process was
non-dumpable; a read-only privileged process-reference check resolves that
inspection, while all deletion remains under the ordinary stage owner.
The model and retained next-run inputs remain untouched. Result is
`E6/products/prefill32-completed-stage-retirement-v7-r1-retry.json`.
Local worktree inspection finds an older scratch checkout with modified
`crates/ferric-engine/src/scheduler.rs`; it is preserved, not treated as disposable.

A read-only CPU retirement plan identifies 1334 exact archived R2 files and
duplicate test ELFs (334376960 allocated bytes), disjoint from 55 retained
keepers and 538 current libraries. Complete R2 custody is
`E6/products/packet-ticks-r2-complete-history-before-retirement.tar`, SHA256
`b89dbf1ffa04590de1357a6f6d0a4339c57065396e6cc401fc9c5d1ae3b0474f`.
Cleanup stops before any unlink because an own-UID non-dumpable systemd user
manager cannot be inspected; sudo requires a password on mi300x-2. The exact
daemon metadata does not establish descriptor state, so no no-use exception is
made. The later failed build changes the target census; the old plan cannot be
applied without fresh review. The user has now approved a temporary 14 GiB
private-stage cap for the remaining diagnostic validation. Isolated profile
and phase successors are independently reviewed and remotely qualified.
Only the cap and explicit profile identity change; all CPU/RSS/timeout,
host floors, 512 MiB reserve, ownership and cleanup rules remain unchanged.
The three actual 12 GiB passes and failed build/a001 are retained unchanged;
the new build attempt must be a002. The new receipt must validate each phase
with its actual generation's auditor rather than relabeling old records.

## Retained Attribution

The older accepted Model R6/R8 six-image ordered16 capture has been independently
re-audited: all 512 output IDs/bytes, both 83139-record captures and all 42
per-arm summaries reconstruct exactly. Its V14 attention/V5 KV paths are not
current split8/V19, and it cannot substitute for current ordered64 attribution.
Old decode down-projection medians are about 21k raw ticks and gate/up about
14.3k, each occurring 36 times/token and stable across layers/context strata.
Head/argmax have larger individual intervals but occur once/token in singleton
groups. Neither raw intervals nor intergroup gaps establish shader durations,
additive GPU-time shares or removable launch cost. The isolated note is
`E6/proposals/model-timestamps-analysis-r8.md`, SHA256
`05c7850c24eb2dab70201b49145b1c6d227bd8834641955ec77eb0e7072bd727`.
The follow-up decision table preserves V19 when composing packed gate/up and
records that the prior V20 prefetch model experiment showed no demonstrated gain.
A separate same-agent fence review finds normative AQL support but unresolved
direct-KFD mapping/publication and kernel-effect contracts. No fence or runtime
change is authorized or implemented from that source-only review.

## Earlier Resource Checkpoints

The five-feature compile check completed in 171 seconds, with peak observed RSS
2660327424 bytes and clean unsignaled teardown. Its cold target grew the owner
to 12171374592 bytes. A completed, guarded retirement then removed 5010 disposable
intermediate files from two completed kernel targets, reclaiming 1886048256
allocated bytes. Eight actual test ELFs, shared libraries, source trees and all
test evidence were retained. The full before/after census and deletion journal
are retained under `E6/products/perf-v7-cache-retirement-r1`. Limits are unchanged.

The final release build passed in 185 seconds, with peak observed RSS
2884141056 bytes and final private-stage size 12060131328 bytes. Actual controller
SHA is `0b4903d80accc284d61add857f724baa7aafe77f686e1215b931467068a626fa`.
CPU receipt SHA is
`1e0fef2331a9883eaa03e13ccbda42a44aebb59e365a3bcd83dfa13239c7059b`.
The receipt and raw command/test evidence are retained in
`E6/products/host-diagnostic-cpu-review-v7-r1`.

Rust execution counts span feature variants, not unique tests: ordered64
79/75/437, combined-feature 73/465/69, and plain compatibility 65. There were
57 pre-existing ignored executions and two executions of the documented
resident-roster test exclusion. The source-policy suite passed all 47 tests.
Its first recipe attempt stopped before compilation because Cargo supplied both
host-build and optimized copies of `toml`; the retained second attempt explicitly
selects the optimized artifact. No production source was changed for that retry.

Additional completed retirements preserve actual executables, shared libraries,
source custody and test evidence: 649527296 allocated bytes of old prepared
sources, 3717242880 bytes of diagnostic-target intermediates, and approximately
726 MB net from current CLI release intermediates. The current compiler test
phase ends at 11109789696 private-stage bytes. Limits are unchanged; no default
or matched benchmark contract is widened.

Before the KV-composition release tests, another guarded retirement removes
exactly 339 completed check-only `.rmeta` files, reclaiming 263454720 allocated
bytes while retaining all executables and evidence. The completed 431 compiler
target is then fully archived and member-verified, copied locally with matching
hashes, and retired after fresh no-use/custody checks. Its 2160 file entries and
632 directories account for 674418688 regular-file allocated bytes. All four
independent compiler tools and capture-screen receipts remain unchanged. The
complete archive has SHA `229d067974d497d26bab0d9f1c7963b5397b0c6cd11cb6fe438e9bf184aa9763`;
future target/test reuse requires restoring it. The stage ends at 10831994880
bytes before release testing, with no cap or reserve changes.

After verified local custody, another guarded retirement removes 20 historical
ELF files (19 unique inodes), reclaiming 685387776 allocated bytes. Original 13e
and current prerequisite keepers remain; no other tree is removed. The receipt
is `E6/products/historical-elf-retirement-v7-r1/retirement.json`.

A further exact-path retirement removes 19 obsolete target ELF copies across
17 inodes, reclaiming 246489088 allocated bytes. All 11885 other target entries
and current review/tool keepers remain unchanged. The receipt is
`E6/products/target-elf-retirement-v7-r1/retirement.json`.
The consumed R1 source and two remote archive copies are also retired after
all 1312 formatted-source member hashes match the retained local archive
`E6/products/prefill32-formatted-source-v7-r1.tar.gz` (SHA256
`0b5cbd9a358d4c9cb9052203abd7f2940379e0c772eab85a7b727777b475aec9`).
R1 failure receipts and independent ELF copies remain retained.
Four additional obsolete R1 target ELF copies are then removed under the same
guard after fresh exact-path SHA/stat/no-use checks, reclaiming 63197184
allocated bytes. Their independent remote/local review copies and the fifth
unarchived R1 diagnostic remain. This bounded four-path removal has a retained
guard and root observation record, not another whole-target census.

The prefill32 release retry needs further headroom. Fourteen completed site
snapshots lose only their installed `node_modules` trees (251453440 allocated
bytes), under a bounded shrink-only command because the initial stage reserve
prevented allocating work. Three cold source archives with hash-matched local
custody are then removed under the unchanged guard (48504832 allocated bytes).
Finally eleven R2 target test-executable duplicates are removed under that
guard after fresh SHA/stat/no-use checks (171642880 allocated bytes). All
eleven independent remote/local review copies remain unchanged. Sources,
locks, evidence, the release controller path and models are preserved. This is
exact-path cleanup, not a claim of another full-target census.

## Current Compiler And Site

The capture overlay on actual upstream `431fa35b6795e5170ae74858da8b2416002a8717`
passes remote preparation, CLI build, backend build and seven capture-focused
tests. Actual CLI SHA is
`1812ccd87e6447805d0548a63830eaf37d4eba18bcaba46bc42ca67df06a0ce3`;
backend SHA is
`92fc267c404c6fa2c91e8abd8bf4c7989f6c246539bb1a73477ef1ec4728b1a1`.
Evidence is retained in `E6/products/capture-review-431-v7-r1`. This is a build
screen, not full producer regression/lint qualification, a new emitted image or
native performance evidence. No fe2o3 change is pushed by this campaign.
The identical eight-file capture patch now also applies over fresh dc48 while
preserving all 18 upstream changed paths. The new source-only assembled tree is
`f63a6ac55ebd2e5edadc28d38447466ed70fdafe`; it has not been built or qualified.
The newest bcd successor is `E6/proposals/perf-v7-kernel-track-bcd-r1`, assembled
tree `abd96567dc9aba5d2f7f9cd59ebc28ef1786c13a`, archive SHA256
`40ff8bc8ad1ad6f293a4afa6cdad54a9c366a53cfcd29c098a3a2b9bde450121`.
The corresponding `perf-v7-packed-gate-up-workload-bcd-r1` refreshes both SDK
pins and contract assertions; it retains the earlier stale-assertion fix and
rejects historical f4 without changing arithmetic. Its 34 Rust cases, six component cases, producer qualification,
emission and native checks are pending. Neither successor relabels the actual
431 compiler screen or built-at-5e2/dd6 worker.

The remotely validated static site is published at
<https://harsh-nod.github.io/ferric/> by commit
`8d936afd64def9b4370e10993e077dacde4fe95b`, deployment run `35783426921`.
All seven served files match the remote-tested artifact. Remote QA covers eight
named widths and every width from 320 through 1440 pixels; all 139 captured
views were visually reviewed by the team. All 28 publication fixtures and
guarded static preparation pass. The published update includes both separate
KV-copy campaigns, the unchanged three-pair vendor result and first host diagnostic.
Only static files and their deployment bindings are published; no GitHub build
or private implementation publication occurs.

## Next Candidate: Composed KV Copy

The accepted V19 single-token parallel KV-copy image is not selected in the
current ordered64 + split8 + V27 parallel-prefill composition. A narrowly scoped
constructor/atomic selector and dedicated live profile are integrated to
compose it without new kernel emission. Both arms must admit the same nine
images and use the same worker; only KV-copy selection differs. Defaults and
the existing forward path remain unchanged. Expected counts stay at 652 packets
per decode, 87711 per request, 135 batches and 1973 ordered groups.

Older retained packet-tick evidence has median decode KV-append intervals of
31436..33232 raw ticks versus roughly 14300 for each gate/up projection. These
are uncalibrated packet-processing intervals from an older group/attention
composition, not current GPU time or a predicted speedup. They support testing
this existing kernel before new packed projections. Independent source review,
remote preparation, five feature checks and strict Clippy pass. The check phase
took 173 seconds with peak observed RSS 2411704320 bytes; Clippy took 16 seconds
with peak 1082257408 bytes. All 16 benchmark-harness fixtures pass remotely in
64.470 seconds, with no skips. Ordered suites pass 445 library, 86 new CLI,
83 legacy CLI and 89 diagnostic tests; combined-feature suites pass 466/77/75/81.
The plain compatibility suite and six exact actual-image inspections also pass.
The library suites preserve their single documented resident-roster exclusion;
each feature group retains 35 pre-existing ignored executions.

Policy attempt a001 reports 46 passes and three failures in stale wrapper
allowlists/validator-shape assertions. The corrected policy file changes no
production code or test count. It is independently reviewed and will be tested
in a separately inventoried, one-file-amended snapshot; the original source and
failed attempt remain immutable. Controller build a001 is interrupted by the
unchanged stage reserve at 19:27:45 UTC, with status 125, not accepted. A fresh
process census finds no surviving compiler process. Historical V19 acceptance
does not qualify this controller or establish a gain.

The completed shrink-only cleanup verifies all 6529 archived source files and
four retained tool hashes before retiring the obsolete remote 431 source, tools
and archive. It removes 6534 files and 850 directories, preserves the original
13e producer/tools and all capture evidence, and restores the full build reserve.
Stage size falls from 12398116864 to 11996418048 bytes including the durable
journal. Local full archive and tool custody remain available for restoration.

The supplemental policy phase then passes all 49 tests without ignores or
filters. Its final formatted policy SHA is
`772800eb89c38d4bc2c73dba26af23151146e77323f5a362b3920b2a3b9ebae7`.
Controller build a002 completes at 19:49:54 UTC in 58.3 seconds, peak observed RSS
2344837120 bytes, with clean unsignaled teardown and final stage size
12248289280 bytes. Controller SHA is
`76a26e70b2cb5456b7a962113134172e38e09db9ba96380451a71dc25439ad81`.
The accepted amended CPU receipt SHA is
`a5f4263ca5cedbc6160bb6515621a6d1ce4f0b19927fb909c6c421660cb515e4`.
It keeps the original controller source inventory and explicitly binds the
policy-only correction and original failed history. Complete original and
amended reviews and all guards are retained locally. Native qualification and
performance claims remain false in that CPU receipt.

Fresh MI350 staging copies only the same nine images, worker, new controller,
receipt and pinned harness; no checkpoint or historical result is copied. The
AB/BA campaign is launched at `/tmp/ferric-opt-v5-v25.QyQQf25M`, plan SHA
`d12ac9093ad4f3e89bda49f1f352b55b95a5830dbdbb38d78dfb0faaa3cbff2a`.
Four starts each have one excluded warmup and three measured requests. The fresh
eight-GPU idle check passes, but only TP1/C1 is exercised. No native result or
speedup was accepted at launch.

### First KV Screen

The completed campaign is accepted with all 2048 output IDs and decoded bytes
matching the fixed reference. Each arm executes 540 batches and 350844
dispatches. The outer supervisor exits zero without signals, reaps its child,
and finds no owned or foreign KFD process. Inner controllers each exit zero;
the frozen harness deliberately sends TERM/KILL to their reserved groups during
post-exit cleanup. Those flags remain visible, not described as unsignaled exit.

| Order | Arm | TTFT (ms) | TPOT (ms) | Finite-Window Output Tokens/s |
| --- | --- | ---: | ---: | ---: |
| AB | Baseline | 807.242508 | 64.454714 | 14.232567 |
| AB | Parallel KV | 809.844509 | 52.819936 | 17.024785 |
| BA | Parallel KV | 827.309448 | 52.755369 | 17.003771 |
| BA | Baseline | 807.646956 | 63.878421 | 14.348660 |

TPOT is 18.051% lower in AB and 17.413% lower in BA. Output rate increases
19.619% and 18.504%. TTFT is 0.322% and 2.435% worse. Each mean has only three
measured requests after one excluded warmup. Orders are not pooled; rate is a
finite ingress window, not sustained throughput. The proposed conservative
screen's 2% TTFT bound is exceeded in BA, so defaults are not promoted. The
same-controller AB/BA repeat below uses the unchanged contract.

Report SHA is `ce74369df5b5ed52721f2ad043d1e0828060494bd7bfebf6da7fa5de9359bb68`.
The complete archive is `E6/products/v7-kv-native-r1.tar.gz`, SHA
`286fe2ef94c218675bc003d0d118b4a44ed9f21f43d8bfb713f9968fd964b8f4`.
The full remote tree matches that archive; after local verification and fresh
no-use checks, the completed remote stage and duplicate archive are removed.
This screen does not establish HTTP, TP8, speculation, SGLang or vLLM performance.

### Independent KV Repeat

The fresh repeat finishes at 20:19:26 UTC with all 2048 output IDs and decoded
bytes matching the reference. The same controller, worker, nine images and
four-start contract are retained. The outer supervisor exits zero without
signals and finds no surviving owned or foreign KFD process. All inner
controllers exit zero, with the same explicit post-exit reserved-group cleanup
flags as the first campaign. Independent raw-artifact audit finds no mismatch:
all stream/process/cleanup and image hashes, all token/byte outputs, every
request's raw-clock timing, exact schedule and unchanged input snapshots match.

| Order | Arm | TTFT (ms) | TPOT (ms) | Finite-Window Output Tokens/s |
| --- | --- | ---: | ---: | ---: |
| AB | Baseline | 944.284289 | 77.743166 | 11.831300 |
| AB | Parallel KV | 858.398593 | 59.667637 | 15.171775 |
| BA | Parallel KV | 852.169842 | 59.939340 | 15.121163 |
| BA | Baseline | 892.748743 | 71.797674 | 12.785176 |

TPOT is 23.250% lower in AB and 16.516% lower in BA. Finite-window output
rate rises 28.234% and 18.271%; TTFT falls 9.095% and 4.545%. These are separate
three-request means, not pooled with the first campaign. Absolute baseline and
candidate timings both shift between campaigns. The first campaign's TTFT
regression and every warmup/measured request remain visible. Across both
campaigns there are eight starts, 24 measured requests, eight excluded warmups
and 4096 checked output tokens; this coverage is not a confidence interval or
default-promotion decision.

Report SHA is `55c1455bea17de55ca52aaaaa1ceb5b8bbe8f2d230f3c79ad423c4a0c22d6634`;
outer supervisor SHA is
`7a02e34c290c21cd932debb61bd37baf3e3c93bad8e34e69ab14a58e38b05479`.
The complete locally retained archive is `E6/products/v7-kv-native-r2.tar.gz`,
SHA `f7d16934c7de2b346c1a14a1df2ac8664176e51276559510085c8eebd5bfb6c3`.
After complete archive comparison and fresh no-use checks, the completed
remote stage and its duplicate archive are removed. The shared checkpoint and
inputs needed for subsequent experiments remain untouched.

### Parallel Follow-On Work

The full-N packed gate/up pair component passes all 17 remote CPU fixtures,
with no skips, at 20:09:41 UTC. The measured unit explicitly includes one
activation pack plus both projections, versus two baseline projections, with
fixed ABI scalar order and full-capacity output guards. This is a tested
benchmark component, not a GPU launcher, new emitted image or model gain.

The integrated prefill implementation reuses two existing 16-row V27 dispatches
for a genuine prepared 32-row batch, with shared terminal-row permutation,
bounded half-view binding and selection-aware packet preflight. Its current R2
qualification and pending native gates are recorded above; no gain is accepted.

The capture overlay is also assembled on current bcb4 as source tree
`5edcb0e4766393e99e3e7722fbd3de655293ef0e`, archive SHA
`976ab637229a932e87ac9d46943c18342645aa77477768faf560c66e35e7176e`.
The eight-file overlay is byte-identical to its prior rebase; all 16 upstream
changed paths are preserved. No bcb4 build or producer qualification is claimed.

Completed diagnostic source retirement preserves all 2607 files in a locally
verified archive before removing only the two obsolete remote source trees.
It reclaims 101699584 allocated bytes and leaves current KV sources, actual
executables, policy dependencies and all evidence untouched. The completed
pair-fixture phase ends at 12166406144 private-stage bytes; further substantial
compilation needs more owned-cache cleanup under the unchanged limits.

## Accepted Host Diagnostic

The 2026-09-22 MI350 run completed with exact 128-token output parity, independent
raw-stream replay, 135 batches, 1973 ordered groups and 87711 dispatches. Both
controller and outer supervisor exited without signals; no owned or foreign KFD
process remained. Instrumented request TTFT was 818.53 ms and TPOT was 64.64 ms.
These are one cold diagnostic request, not a matched vendor measurement.

| Mean Per Decode Batch (127 Batches) | Wall Time |
| --- | ---: |
| Complete enclosing batch | 64.626 ms |
| Worker publication through completion | 58.473 ms (90.48%) |
| Ordered roundtrip minus worker interval | 5.472 ms |
| Controller ordered argument packing | 0.171 ms (0.265%) |
| Other transport round trips | 0.232 ms |
| Remaining enclosing batch wall | 0.278 ms |

The worker interval includes GPU work, polling and runtime fences; it is not GPU
time. This evidence deprioritizes controller packing as the first optimization.
The separately selected runtime-counter mode passes all seven remote CPU gates.
Its native run uses a distinct controller, receipt, profile and harness; it is
not admitted by the completed host-only harness.

All seven late decode batches are retained: batches 9..128 average 63.682 ms,
whereas 129..135 average 80.802 ms. The latter show both controller-only and worker
increases. No queue rollover is recorded and packet geometry is unchanged.
Context 249 also changes the split-attention partition span, but that alone cannot
explain simultaneous controller-only increases. Endpoint CPU placement samples
show migration but cannot locate it or establish causality. Repeat measurements
and runtime counters are needed before assigning this change to a kernel.

Native evidence is retained at `E6/products/ferric-opt-v5-v25.AxZFdl4m`.
The complete archive SHA is
`6baf53e3867bad414343a8beb54402fa092ebfba84c5e1dff4b155a9e61eee92`;
sidecar SHA is
`c67f26a54667dabf91adc7e87742dd84df1d4df748df62d62b73c12542fb17b8`.
No speedup or vLLM competitiveness is claimed.

## Runtime-Counter Qualification

The successor passes five feature checks, strict selected-target lint, 394 CLI
test executions (88/82 ordered, 80/74 combined, 70 plain), all 48 source-policy
tests and all 23 measurement fixtures. The CLI suites retain 35 existing ignored
executions. Unchanged library suites are linked to their prior qualification,
not reported as freshly rerun. The first test attempt failed its disk-headroom
preflight before compilation; the second passed without relaxing limits.

Actual controller SHA is
`04ce59ed762ace7c5befdfda55292130caf3812d7eaae060dd8eed9370cb00ff`;
CPU receipt SHA is
`91a13791a5aa2028d8c21838eb7b24b9f2dbffd6bda42359d0a0129db83fa3f9`.
The complete review and executed test identities are retained at
`E6/products/runtime-counters-cpu-review-v7-r1`. Release build elapsed time was
41.5 seconds, peak observed RSS 1877200896 bytes, with clean unsignaled teardown.

Two cumulative snapshots cover the entire request, not separate prefill/decode
windows. Publication, wait and currentness durations overlap and cannot be added
into an exclusive CPU/GPU partition. Command and currentness snapshot boundaries
differ; currentness need not be less than command time. Waiting is not GPU time.
The native counter run is accepted, as detailed below.

The source audit at actual worker 5e2 and fresh main 431 finds identical AQL/KFD
trees. Every ordered packet requests barrier ordering and system acquire/release
scopes; only host publication/doorbell work is group-amortized. Boundary-only GPU
scopes are a hypothesis requiring correctness review, not a measured bottleneck
or implemented optimization. The audit and focused test matrix are retained at
`E6/proposals/perf-v7-ordered-scope-audit-r1.md`.

## Accepted Runtime-Counter Diagnostic

The MI350 run completed at 17:55:14 UTC with exact 128-token output, 135 batches,
1973 ordered groups and 87711 dispatches. Inner and outer exits were zero without
signals; owned and foreign KFD process lists were empty. The two retained snapshots
reconcile all 19 counters. Instrumented TTFT was 911.75 ms and TPOT 86.38 ms. This
different profile and separate cold run are not a comparison with the host-only
diagnostic or vLLM, and do not establish a regression or gain.

| Whole-Request Runtime Counter | Delta |
| --- | ---: |
| Commands | 2788; 10.024729185 s |
| Completion waiting | 9.258467152 s; 92969 polls |
| Operational currentness | 0.572865233 s; 21381 checks |
| Full currentness | 0.003667206 s; 2 checks |
| Dispatch preparation | 0.079515903 s |
| Publication | 0.053916000 s |
| Kernel admissions | 0 |
| Reads | 128; 512 bytes; 0.008485115 s |
| Writes | 675; 654840 bytes; 0.041678735 s |

The dominant measured scope is completion waiting, not currentness, admission or
publication. Waiting still includes both pending GPU work and host polling or
scheduling delays. The actual engineering worker sleeps 50 microseconds after
each pending poll; it does not use the protected runtime's adaptive policy.
Across 1984 completion operations, 90985 pending polls request 4.54925 seconds of
nominal sleep. That sleep can overlap GPU work and is not reclaimable overhead.
A bounded, opt-in active-poll experiment can test detection delay without changing
packet scopes, ordering or correctness checks. No default change is justified yet.

Complete evidence is retained at `E6/products/ferric-opt-v5-v25.b812jgG0`.
Archive SHA:
`3e55c812beacad828261823c5290c01c5661c9c15f002b76f808b85d335f3aaa`.
Native report SHA:
`e1a984d12a5f240d60981cf6fc2e10568f5cd5aaead42884497655d838cf85ce`.
Independent source/receipt review confirms the replay, counter arithmetic and
clean teardown. No new matched vendor run or performance gain is claimed.
