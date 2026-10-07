# Performance Swarm V10

Updated: 2026-10-01 UTC. Per-change experiments, not a serving qualification.

## Current Continuation

Ordinary SSH access is restored. Builds and tests remain remote on `mi300x`;
there are no local builds or substituted performance results. `mi350` is also
reachable, with eight idle MI350X GPUs and retained model filenames/sizes
observed. That observation is not fresh launch admission or model-byte validation.

R4 a006 now has a clean finite-only timing result: five-pair groups show lower
candidate controller-wall latency in all four comparisons, while one-pair
groups disagree by order. This is not an engine win or promotion; the exact
paired table and custody appear below. Earlier B4/a002 refused before launching a
child because the GPU was busy: zero cells, status 125. Its complete failure
evidence is now retained and its stage is verified removed; the original
`cleanup_ok=false` receipt is not rewritten. Fresh same-payload a003 passes
CPU/native admission, then accepts 13 parity cells before the process census
fails during cell 013 teardown. No timing cells run. The outer supervisor sends
TERM, reaps its child, and reports clean owned cleanup; all 14 worker cleanup
records are clean. This is an interrupted campaign, not an accepted result.
The exact census subprocess failure was not retained by the old generic error;
an exit race is a hypothesis, not a confirmed cause. Complete a003 archive:
`fixed-safe-r3-r4-native-results-a003.tar.gz`, SHA256
`adc05e160e7282d59bbd5b633df47f77028ef77ffdf99ea58152ea19d279c8f9`.

A004 stops before child launch with zero cells because the staged worker is
mode 0600 instead of the archive's 0700. This is a staging error, not another
census failure; its original `cleanup_ok=false` receipt remains unchanged.
Full failure retention and guarded stage/upload retirement pass. Fresh a005
then passes all 20 finite parity cells, 140 buffer/guard checks and 80 parity
dispatches, with 20 distinct clean, unsignaled, reaped workers and clean outer
status 0. Independent custody confirms zero timing cells. Its stage/upload
are retired after retention. A005 remains parity-only; its receipt is not
relabelled as the separate a006 timing result.
[A004 custody](/home/harsh/.codex-tmp/ferric-perf-swarm-v6/proposals/perf-v10-a004-retrieval-r1/README.md)
and [a005 independent audit](/home/harsh/.codex-tmp/ferric-perf-swarm-v6/proposals/perf-v10-a005-retrieval-r1/a005-result-audit.json)
retain the separate outcomes; a005 archive SHA256 is
`3e05ae5c2da526ae15d87f17db0a5751ebef1ac69e03a62af0d7ed77ec7773c3`.

Latest observed fe2o3 main is `fe0b3520811536a82cf14af8973ba992454bfd1b`.
Its standalone release worker build, 106 selected library tests and six CLI
tests pass remotely with clean unsignaled receipts. Historical R3/R4 binaries
remain unchanged. The earlier 4fb8 capture cohorts are now retrieved: 9 backend
capture, 14 backend regression, 10 CLI capture and 27 CLI regression executions
pass, representing 50 unique tests because the CLI cohorts overlap. Fresh fe0
producer and remaining extractor/transitive qualification are not yet complete.

Private current-wire client metadata and lock resolution now pass offline.
Independent lock inspection confirms unchanged registry versions/checksums,
retained legacy 5a503 host dependencies and the exact nine-crate fe0 KFD closure.
The first focused client check fails on 252 duplicate device-SDK diagnostic
items and one batch32 roster type mismatch. The separate compiler-derived V5
roster bridge now resolves that boundary: the combined library, packed entry,
ordered-host diagnostic and source-policy check passes remotely. Kernel code
and strict artifact admission remain unchanged. Remote formatting and the
resolved lock are integrated after checking all local source preimages; the
lock SHA256 is `dfaa06375ba8f810aebf6a44467702fbe97c36a897e8e75cac88edde61b286a3`.
Executed follow-up passes seven active-poll selector tests in each of three
feature configurations, 54 source-policy tests, one roster test and one explicit
canonical V5 artifact-admission test. The standalone bridge's
`tests::generated_roster_retains_full_and_wave_profile_names` test also passes
with zero failures. The actual ordered64 host release build and strict Clippy
pass; the ordinary executable, not a test ELF, has SHA256
`a2ea6d961895644011b382017b1c42538ab2e568e531ba8e613a675679fec757`.
These scoped CPU results do not establish native compatibility or performance.

The separately named G28 CPU profile passes 23 fixtures. Its 28 GiB stage cap
retains four cores, 8 GiB aggregate RSS, nice 19, a 1,200-second limit and a
512 MiB stage reserve. This is a distinct CPU profile, not a change to the
existing native resource guards. All builds and tests remain remote.

The census diagnostic change passes 31 tests on each of Python 3.12 and 3.10;
its pin-only successor launcher passes 11 on Python 3.10. It retains bounded
subprocess error detail without changing census admission or retry policy.
A003's stage and upload are now verified removed after full evidence retention.
The a005 parity result and separate a006 finite timing result are independently
audited; their native acceptance is not inferred from these CPU passes.

Default-off active-poll selector r2 is integrated and its scoped Rust tests pass
as recorded above. The wait-policy evidence checker passes all 19 Python tests under both Python 3.12
and 3.10. The new same-binary diagnostic harness passes all 48 CPU fixtures;
the actual nine-phase raw build binding also passes. Full-source custody checks
all 1,372 files against the frozen baseline, combined delta, formatter outputs
and resolved lock, with no missing, changed or extra paths. This does not grant
new GPU-artifact source authority. Both retained native ELF loader checks pass
on mi350. All six full-model active-poll campaign runs now pass: two separate
correctness requests excluded from timing, followed by four measured cold
128-input/128-output requests in A/B/B/A order. All 768 output tokens match
exactly. Both policies use the same current-wire controller and fe0 worker
binaries. The accepted comparison records modest latency reductions in both
orders, alongside roughly tenfold worker CPU time; the paired table below
keeps this tradeoff explicit. The selector remains default-off, with no
promotion or statistical/sustained-serving qualification. Complete six-stage
raw custody is independently verified, and all nine authorized campaign roots
are retired with explicit absence receipts. The September 28 matched
vLLM loss remains current; this is not a new vendor comparison.

[Full-source audit](/home/harsh/.codex-tmp/ferric-perf-swarm-v6/proposals/perf-v10-latest-core-followup-r1/FULL_SOURCE_AUDIT.json),
[nine-phase CPU binding](/home/harsh/.codex-tmp/ferric-perf-swarm-v6/products/activepoll-binding-a001.tar.gz)
(SHA256 `12a6e0568125b9ef16a47b900af5fe812dc49c596a918387be0f46fd0c650e6e`),
and [harness CPU custody](/home/harsh/.codex-tmp/ferric-perf-swarm-v6/products/activepoll-harness-cpu-a001.tar.gz)
(SHA256 `05d6bbefd2c2aff92871edcbc4404a2fe03c8dc9a4a937b6bfb197fb2e9cd217`)
bind the prerequisites. The bridge unit build/test receipts are retained under
`E/products/perf-v10-fe0-cpu-a001/pages-clientfe0-v5-bridge-unit-{build,test}-a001`.
Both site snapshots pass remote JavaScript/data checks and eight-viewport
Playwright validation using private dependencies. R2 includes the final measured
results and passes focused desktop/mobile/narrow visual review. The
[live site](https://harsh-nod.github.io/ferric/) is updated through deploy-only
`pages/prebuilt` commit `d71d482e6f96e2560170a68ab222cb23ee4cf43d`, deployment run
`36801884010`. Verified HTTPS on mi300x confirms all seven served static files
match the validated artifact; the local certificate-chain failure remains
retained and TLS was not weakened. No GitHub-hosted build, protection change or
publishing worktree was needed. Site validation does not qualify performance;
all 33 M1 gates remain open.

### Active-Poll Paired Diagnostic

A is the default sleep policy; B is active polling. Each measured row is one
cold full-model 128/128 request in its own process. Two earlier correctness
requests, one per policy, are excluded. There are only two measured samples per
policy, not a statistically qualified or sustained-serving result. TTFT/TPOT
and finite controller-ingress output rate retain their request-level scope;
worker CPU seconds are a separate cost, not GPU time.

| Order | Policy | TTFT (ms) | TPOT (ms) | Finite Output Tokens/s | Worker CPU (s) |
| --- | --- | ---: | ---: | ---: | ---: |
| AB | Default A | 859.967 | 70.378 | 13.064 | 0.95 |
| AB | Active B | 835.992 | 69.782 | 13.198 | 9.43 |
| BA | Active B | 785.728 | 62.188 | 14.740 | 8.53 |
| BA | Default A | 814.453 | 63.099 | 14.499 | 0.82 |

Active polling reduces TPOT by 0.8475% in AB and 1.4440% in BA, and TTFT by
2.7879% and 3.5269%. Finite output rate rises 1.0283% and 1.6633%. Worker CPU
time rises 9.9263x and 10.4024x, respectively. Those small observed latency
changes do not justify silently adopting the much higher CPU cost. Keep the
default unchanged; no engine promotion, GPU-time claim or vendor ranking is
established by this diagnostic.

[The actual paired comparison](/home/harsh/.codex-tmp/ferric-perf-swarm-v6/products/activepoll-campaign-a001/comparison-a001.json)
has SHA256 `788ac3b171aad83387ecb3a4dc21f93fb6b3f98403c380883f400abc992aabee`.
It binds all six plan/report/supervisor identities, explicitly excludes
correctness timings and keeps `performance_qualified=false`.
[Full campaign custody and retirement](/home/harsh/.codex-tmp/ferric-perf-swarm-v6/products/activepoll-campaign-a001/README.txt)
retains all 803 files in a 100,744,872-byte archive, SHA256
`825da54ebdf4b5bfc065f4b6b3fc1f0b9fce0a7c12a7ae7f2b73f0ff08ac459d`.
The six stages, shared input root and two uploaded archives were removed only
after complete before/after custody, fresh all-UID no-use checks and ordinary
owner deletion; each exact path has a nonquiet absence receipt. The external
model and private CPU stages were untouched. The retirement control directory
was subsequently retired under separate authorization after all 19 files matched
local custody and fresh ownership/no-use checks passed; its actual absence
receipt is retained with the campaign.

## First Experiment: CPU Affinity

The September 29 host diagnostics sampled the controller and worker on CPUs
remote from GPU 0's NUMA node. That observation is not evidence of causality.
This experiment changes only controller/worker CPU affinity and retains the
same qualified uninstrumented packed controller, worker, images and workload.
Neither pending kernel candidate is selected.

| Setting | CPU Affinity | Other Changes |
| --- | --- | --- |
| A: baseline | Inherited full mask, CPUs 0-255 | None |
| B: candidate | GPU 0 local mask, CPUs 0-63 and 128-191 | None |

GPU topology must match fresh sysfs observations before launch. No explicit
memory binding, frequency, clock, precision, kernel, batching or runtime change
is permitted. CPU affinity may indirectly affect memory placement; this is not
an isolated GPU-compute experiment.

The first screen uses four fresh starts in A/B/B/A order, each with one excluded
warmup and three measured requests. Every request uses the same Qwen3-8B TP1/C1
128-input/128-output workload, BF16 decoder/FP32 head, 8,192-token context,
greedy output, prefix caching and speculation off. Every output must match the
retained token-ID and decoded-byte reference. Raw samples, CPU placement,
TTFT, TPOT and finite controller-ingress output rate are retained per start.
This is native timing, not HTTP, sustained throughput or a new vLLM comparison.

Status: all 11 remote CPU fixture tests pass under the unchanged four-core,
8 GiB RSS and 24 GiB stage profile. Independent source review found no blocking
issues. The first bounded native campaign was interrupted during B-BA when a
new KFD process could not be inspected. Its supervisor returned 125 with
`cleanup_ok=false`; the incomplete campaign is not accepted. A subsequent
privileged process census found its controller, worker, runner and process
group absent, with no KFD users. No process was signaled by that census.
A fresh R2 campaign completed with the same inputs and full A/B/B/A order.
All 2,048 token-ID/decoded-byte checks pass, with four excluded warmups and
12 measured requests. All four arms and the supervisor exited cleanly without
signals or errors. Independent raw-output, metric and placement audit passes.

### Completed R2 Result

| Order | Setting | TTFT (ms) | TPOT (ms) | Finite Output Tokens/s |
| --- | --- | ---: | ---: | ---: |
| AB | Unrestricted A | 810.353 | 46.425 | 19.084 |
| AB | GPU-local B | 819.571 | 48.866 | 18.217 |
| BA | GPU-local B | 966.009 | 54.793 | 16.150 |
| BA | Unrestricted A | 810.343 | 46.412 | 19.089 |

Candidate TPOT worsens 5.258% in AB and 18.059% in BA. TTFT worsens 1.138%
and 19.210%; finite output rate falls 4.542% and 15.392%. Candidate samples are
also more variable: TPOT sample standard deviation is 5.550 ms versus 0.490 ms
for A across six measured requests each. This small native screen does not
establish a general affinity effect or a new HTTP/vendor ranking. It provides
no reason to adopt this candidate; defaults and measured production composition
remain unchanged.

Full R2 custody `perf-v10-affinity-native-custody-a002.tar.gz` has SHA256
`50da33996075a86b57f445ea9bfb724449c10381b1209f1a9d898a0052336f69`.

R1's completed first pair retains exact output parity but is descriptive only:
baseline/candidate TTFT 819.807/918.763 ms, TPOT 48.718/49.578 ms, and finite
controller-ingress rate 18.265/17.738 output tokens/s. Candidate TPOT worsened
1.766% and TTFT 12.071%; the reverse pair is incomplete and excluded. Preserve
this failed campaign rather than joining its samples to the fresh retry.
Its full archive `perf-v10-affinity-native-custody-a001.tar.gz` has SHA256
`451c19d8d39f1b890fe91d719420d994869121c58024bf25480db17c4c120cda`.

The test archive `perf-v10-affinity-cpu-custody-a001.tar.gz` is retained in
`ferric-perf-swarm-v6/products` with SHA256
`8281089640e4ced4d709a6a0e01990cf67b7e9347f7b42148e93fc6e3dd2d6e9`.
The measured driver is bound to SHA256
`394f3da23720c9733fea7ccc45658c2c97680846909b62459bf06e0b07a48a45`.

## Teams

| Team | Work | Status |
| --- | --- | --- |
| Measurement | Single-variable affinity harness and result analysis | Complete; candidate slower in both orders; not adopted |
| Independent review | Workload, placement, process ownership and metric checks | R1 rejection and full R2 correctness/metrics independently audited |
| Compiler preparation | Pinned `1a5999f6` comparison; latest `fe0b352` qualification | Latest worker build and 106 library plus six CLI tests pass; 4fb8 capture has 50 unique passing tests; fresh fe0 producer pending |
| Kernel preparation | Separate fixed-safe gate/up and paired-load K2 prefill candidates | R4 a006 passes 20 parity and 16 timing cells; grouped finite latency improves 8.65-10.59%, one-pair orders disagree; not promoted |
| Integration | Remote execution, SDK boundary and evidence retention | V5 bridge, source policy, artifact admission, release host build and strict Clippy pass; six full-model requests pass; full custody verified and nine owned campaign roots retired |
| Wait-policy qualification | Default-off r2 selector and terminal/runtime evidence checker | Six native requests pass with 768 exact tokens; four measured ABBA requests show small latency reductions at about 10x worker CPU cost; default unchanged |

The prior mi300x-2 stage still lacks build headroom and privileged all-UID
cleanup inspection. The originally approved mi300x build host was reachable and
had roughly 686 GB free at the earlier bootstrap observation. Ten initial bootstrap fixtures and 14 fresh
allowlisted-environment fixtures pass there under the unchanged bounded profile.
Complete shared installed nightly verification failed (18 missing
files and two mismatched configuration files). That check did not execute a
compiler or change the shared installation. The complete retained toolchain was then
privately staged, and all 5,603 installed-file hashes pass. Its Rust compiler
reports `1.96.0-nightly (55e86c996 2026-04-02)`; this is toolchain execution,
not a fe2o3 build. The first bootstrap invocation rejected a missing lock before launching
any child; after exclusive lock creation the fixtures passed. The source archive
and inventory hashes match their retained inputs. Bootstrap raw custody is
`perf-v10-latest-bootstrap-custody-a001.tar.gz`, SHA256
`6051c3e52614849b4895480f1aaf2aad063b7dd6e08fc484eec840c5a52ea3e1`.
Ten dependency-transport fixtures pass. All 540 selected input files are
checked against the current lock, and fresh offline locked metadata resolves
426 packages with the lock unchanged. The unmodified `1a5999f6` compiler backend
and extraction binary build passes in 75 seconds, cleanly reaped and unsignaled,
with peak sampled aggregate RSS 2,953,994,240 bytes. Upstream warnings remain in
the raw log; this is not a warning-free or native-qualified producer. The
separate CLI build passed from 18:59:59 to 19:01:03 UTC, status/returncode 0,
cleanly reaped with no signals/errors and peak sampled aggregate RSS
2,217,259,008 bytes. The clean compiler baseline backend, extractor, CLI and
linker-proxy artifacts and their actual hashes are retained. No latest-runtime
adoption is claimed.
The affinity experiment keeps
compiler/runtime identities frozen so that this change remains isolated.
This qualification remains pinned to
`1a5999f6e1c5f2363bc2d525af65e84c46502ce6`. The earlier main observation
`f2dd128e35d5345cf4b7f6fb3f71480ee483c961` has a source/runtime audit. The next
observed main was `4fb8ae500e38ad22475861aae7c3b3ef238068f6`; its separate core
worker builds and passes 106 library plus six CLI tests. Its fresh capture
producer also builds; the additional compiler regression qualification is
partial, as recorded above. Full-model/client qualification and compiler
adoption remain open. Migration stays separate,
without changing pins midway through this comparison. Source/dependency work
does not change the historical-binary affinity experiment. The new compiler
has no native performance result here; the isolated r3 kernel result
below retains its actual pinned producer and historical runtime.

Raw CPU custody `perf-v10-latest-cpu-build-raw-custody-a004.tar.gz` has SHA256
`a616b2f4f21331ce405a2de24cf0a6c22962176f0f9b85298b7106155c493d49`;
binary custody `perf-v10-latest-compiler-binaries-custody-a001.tar.gz` has SHA256
`1413057b4e8388a5a96ee3a9092cb1a67d29b019ca85a941c81ac296b752abd3`.
## Pinned Compiler and Kernel Checks

The reviewed `capture-1a599` patch is applied in the private build stage. Its
producer rebuild and 99 unique compiler regression tests pass. The SDK retains
the real `1a5999f6` Git identity, with no path substitution. Initial remote results
in a005 custody:

| Scope | Result |
| --- | --- |
| Fixed-safe gate/up | 21 tests in each default and feature configuration; feature all-target strict Clippy passes |
| Batch32 prefill | 22 tests in each default and `prefill-mfma-k2` configuration |
| Packed R2 baseline | 34 SDK-backed host tests pass |
| Fixed-safe native harness | 13 CPU fixtures pass; no GPU launch |
| LLVM worker | Build passes in 16 seconds; all six CPU CTests pass |

The first batch32 attempt failed because a shared `target.rs` input was absent.
After staging the unchanged shared inputs, the retry passes; the failed attempt
is retained. The host's ROCm 7.2.4 provider hashes differ from the reviewed
closure. The worker uses only the nine exact privately restored gfx950 files;
gfx950 is enabled and gfx942 is disabled. Its 27,990-file, 995,832,147-byte input
inventory is unchanged after build/tests, SHA256
`8f9484da8ae4e92b0920c700d9cf45ed497b23998badea329d8c973683f039b5`.
Generated-command coverage and actual library aliases are checked. This is a
build-input record, not authenticated runtime-closure qualification.

Raw CPU custody `perf-v10-current-cpu-raw-custody-a005.tar.gz` has SHA256
`358eeac3c4e89a369dab126c75bde08596a40998f5a2ed141d1034e31e81569a`.
Worker configure custody `llvm-worker-1a599-configure-custody-a001.tar.gz` has
SHA256 `f7c356bddf4be4eecd12d978055c9de12a551b5773beedcaf0a0a8158493b742`.

Vendor attempt a001 failed offline because the pinned rust-library closure
lacked `dlmalloc`. That failure remains in a005. Subsequent root-reported checks
pass eight std-transport and six absent-only merge fixtures. The exact std lock
selects 40 package versions and 80 transport files; archive verification and
merge pass, creating 30 missing files and reusing 50 byte-identical files.
Offline locked vendor retry a002 then succeeds in about three seconds. These
later results are separate from a005.

## Emission and Native Preparation

The pinned packed R2 baseline now emits successfully with and without diagnostic
capture. Both produce byte-identical HSACO
`fdffa040ad94723460a0891cfb5be22be25cd90c440008067f81bde37fed453e`
and manifest `e215ce56af578694828b124effda28545a34408663058779cec3d55fdbd017d1`.
Retained archive `packed-r2-1a599-emissions-a001.tar.gz` has SHA256
`c50f68ecfb7e275d11ceaeab625c5c38f92293c3f2ea2178f3f63cdf475d5b71`.
This is emission/capture parity, not native numerical or performance qualification.

Fixed-safe candidate emission a001 failed an obsolete build-route assertion.
Its raw archive `fixed-safe-emission-failed-a001.tar.gz` is retained with SHA256
`5b2303796ead80e01624a6213b3825617894440eef32f7d3b7f1967d4f2c0edd`.
After the route correction, feature tests a003 and default tests a002 pass
27 tests each; strict Clippy a002 also passes. Feature a002 accidentally ran
the earlier source before staging and is explicitly excluded as post-fix evidence.
Emission a002 then failed a distinct LLVM convergence check: `bb18`, operation 0,
reached subgroup reduction under varying control. The compiler check was not
weakened. Failure archive SHA256 is
`39c7bb054548cf74f3f2f614902b424459e5a190525c1f17ceb0cafd59185d12`.

The fixed-shape SDK-view r3 correction passes 30 tests in each feature mode and
strict Clippy. Emission a003 passes cleanly on the CPU host, status 0, from
20:20:51 to 20:26:59 UTC. Its HSACO SHA256 is
`4f6ebd3d2294f20ece54a40b20a040129363886fb05b6b7e512bdd9bce447f89`;
manifest SHA256 is
`4192c310614955cba89c0b6316cfcd3bfa4e780dc34773611f6c575dd26056f0`.
Finite-only static review reports body size 1,852 -> 1,760 bytes and SGPR 35 -> 30,
with VGPR 18 unchanged. Both weight guards and the load/wait pattern remain;
loop-guard elimination was not achieved. These observations are not a speedup.
Raw custody a007 SHA256 is
`3e4ea24d45657df3b481f10aa78bbe4e0f4b34d040f340ea6a76ebd3d6fe3a38`.

Remote G24 checks pass 19 mi350-2 profile fixtures and nine thin-launcher fixtures,
with clean, reaped, unsignaled teardown. The actual retained monitor census then
revealed that the root-running monitor's executable file is owned by UID1001.
The profile now pins that exact observed executable identity/hash separately
from root process credentials. Its expanded 21-test revision and nine-test
launcher repin both pass at a002 under G24, cleanly reaped and unsignaled.
The real mi350-2 profile preflight passes with only the bound monitor present;
actual-data admission also passes on the CPU host.

The first GPU-host launcher attempt stops before any GPU worker because system
Python 3.10 lacks `hashlib.file_digest`; no launch outputs are created. A private
prebuilt CPython 3.12.13 is downloaded through `uv` on mi300x, not built locally
or on the GPU host. All 13 harness, 21 profile and nine launcher CPU fixtures,
plus actual-data admission, pass using it. After transfer, native attempt a001
returns status 125 before creating the supervisor child or a GPU worker: this
private Python build lacks `os.pidfd_open`. The required process-ownership check
is not bypassed. Cleanup reports true, no TERM/KILL was sent, and postflight
finds only the bound zero-work monitor, with no owned or foreign KFD users.
The complete native-attempt archive SHA256 is
`471e8808e03db16eb9af019e8cef2a8d5f20401a11ae256a5182730c2420b3f1`.

A separately versioned Python 3.10 support copy replaces only `file_digest`
with bounded-memory streaming SHA256 and propagates dependent source pins; the
original frozen support remains unchanged. All 18 compatibility tests pass on
mi300x using prebuilt Python 3.10.20 and system Python 3.12. With Python 3.10,
the fixed harness passes 13 fixtures, the profile passes 21 at verbose a002,
and launcher r2 passes nine, including rejection of the old manifest. The new
launcher SHA256 is
`ba4b7e376e4af03e609eb74036f2bda81456fa2ebfa6a221e3cc175c6e00a669`.
CPU custody SHA256 is
`52433bfb6753c054b6763af8e3e88cd0ea133935e5e530d5c8cbdd1902819837`.

Input bundle a002 is assembled with plan SHA256
`f36f9ed53c90b41d3d6b7e814e5fe349c6551391264f279f67e8c1de1d672742`.
Both kernel images, pinned compiler and historical `dd6bd3b4` runtime remain
unchanged. Actual-data admission and the native retry now complete successfully.

### Completed R3 Finite Diagnostic

Native a002 runs from 23:27:48.122908 to 23:39:48.454260 UTC, about 12 minutes,
and exits status 0 with clean reaped teardown, no signals/errors and only the
bound zero-work monitor remaining. All 36 cells pass: ten finite cases across
two activation epochs, followed by 16 timing cells in A/B/B/A order. All seven
full-buffer/guard records agree, including final timing-cell checks. Each timing
cell excludes two warmup groups and retains all 32 measured groups, 512 samples
total. Prepacking and uploads are excluded.

The metric is instrumented controller wall per cache-hot gate/up pair, in
microseconds. Positive change means lower candidate latency; negative means worse.

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

R3 demonstrates no useful gain and is not promoted. The final 206.882 us baseline
is retained, not excluded or pooled away. Independent data-only audit matches
all 36 result files to report cells, recomputes all sample means and eight
comparisons exactly, and matches raw protocol snapshots and worker elapsed
samples to every timing record. Counters and packet/warmup counts agree.
Results archive SHA256 is
`ceed68468e5b30e30dc64fa89b9cdf79cc3cc421b9703fe5141a8a8bf352a5c0`.

Every 32-group timing cell records 321 operational-currentness checks. Their
overlapping host scope costs about 384-387 us per pair with one pair per group,
versus 77-79 us per pair with five. No transfers or kernel admissions occur in
the measured interval. This prioritizes currentness amortization and command
batching in the inference-engine/runtime path; it does not establish a GPU
bottleneck or permit subtracting overlapping counters to derive GPU time.
The final faster baseline also records lower worker aggregate wall (120.011
versus about 129-130 us/pair), wait scope (81.097 versus about 90 us/pair) and
145 completion polls rather than 159-160. That correlation is a polling-variability
clue, not proof of causality or a reason to discard the observation.

Separate r4 finite-check-hoist CPU validation now passes 36 tests in each feature
mode and strict Clippy, plus 16 native-harness and 11 launcher fixtures. Its
emission a001 failed before launch because the output parent was absent; that
failure is retained. Emission a002 succeeds from 23:37:11.701955 to
23:43:19.934447 UTC, clean status 0, with peak sampled aggregate RSS
2,996,101,120 bytes. Actual ISA and ELF metadata are captured. Independent static
review accepts the finite-only diagnostic. The first r3-versus-r4 native attempt
a001 runs from 2026-09-30 00:03:50.016577 to 00:04:54.579247 UTC. Six parity cells
are accepted; the seventh (`tail-one`) is interrupted and not accepted. No
timing cell runs. The supervisor returns 125 after `complete privileged
read-only census failed`, sends owned TERM, reaps its child and reports clean
cleanup with no KILL. Its immediate postflight records no owned or foreign KFD
users, only the bound monitor. Three subsequent censuses succeed but observe
transient foreign `/usr/bin/python3.12` PID 97300; root later observes that PID
absent. The failed census's underlying cause is not established by the retained
stderr, so the later process is not assigned as its cause. Full failed archive
SHA256 is `5ba67da3f137a93925d1d0996736ebe42e7469ed6c049f7302ab406e71fe2206`.
B4 retains the same payload in fresh stage
`/tmp/ferric-opt-v10-fixed-safe.0c21e874`, with plan SHA256
`c616bc6c660fed34c4bfc3ff23252585c00fc85a835b992c0c590c6032b4e38b`.
CPU admission passes. Its retrieved native receipt reports status 125 before
child launch because the GPU is busy, with zero cells. Historical cleanup is
false; subsequent guarded retirement succeeds. Fresh a003 accepts 13 parity
cells before another census failure, with no timing cells. Later a004's staging
failure and a005's clean 20-cell finite parity pass are retained separately in
Current Continuation. Fresh a006 then passes the unchanged 20-cell parity
cohort and all 16 timing cells, with 36 clean unsignaled/reaped workers. R4 is
not promoted.

### R4 A006 Finite Timing

The October 1 run starts at 01:05:51.932535 UTC and finishes at
01:13:44.928869 UTC, elapsed 472.996334 seconds. Outer status/returncode are
zero, cleanup/reap pass, errors are empty and no signals are sent. Independent
audit verifies every cell, all 512 measured groups, 3072 measured dispatches,
protocol completions, counter deltas and eight separate AB/BA comparisons.
Each timing cell has two excluded warm-up groups and 32 measured groups.
A is fixed-view R3; B is finite-hoist R4. Means are instrumented controller
microseconds per cache-hot gate/up pair, excluding packing and uploads.

| Pairs/Group | Epoch | Order | A Mean (us) | B Mean (us) | B Latency Improvement | A/B Speedup |
| ---: | ---: | --- | ---: | ---: | ---: | ---: |
| 1 | 0 | AB | 623.674 | 621.233 | +0.3913% | 1.003928x |
| 1 | 0 | BA | 622.850 | 623.118 | -0.0431% | 0.999569x |
| 1 | 1 | AB | 628.672 | 623.174 | +0.8744% | 1.008821x |
| 1 | 1 | BA | 622.172 | 631.971 | -1.5750% | 0.984494x |
| 5 | 0 | AB | 217.709 | 194.662 | +10.5860% | 1.118393x |
| 5 | 0 | BA | 216.574 | 197.850 | +8.6454% | 1.094636x |
| 5 | 1 | AB | 216.288 | 197.172 | +8.8381% | 1.096950x |
| 5 | 1 | BA | 218.178 | 197.406 | +9.5206% | 1.105224x |

One-pair results change direction with order and show no consistent gain.
Five-pair groups show an 8.6454-10.5860% reduction across all four comparisons,
limited to this finite dense, cache-hot diagnostic. Do not pool away the
one-pair disagreement or turn the grouped observation into an engine/model
win. Controller and worker scopes overlap; this is not GPU time, TTFT/TPOT,
serving throughput, rejection/trap qualification or a vendor comparison.
Raw performance-gain acceptance, full qualification and model promotion remain
false. Historical producer, runtime and image identities are unchanged.

[The independent a006 audit](/home/harsh/.codex-tmp/ferric-perf-swarm-v6/proposals/perf-v10-a006-retrieval-r1/a006-result-audit.json)
retains all 512 raw samples and recomputed statistics, SHA256
`887b3e40983ea6dd6e09086799d14d696f433bbd73e4afab77184af7d07483a9`.
Complete raw archive `fixed-safe-r3-r4-native-results-a006.tar.gz` has SHA256
`982af337b40565ec6de79ff2e14e509d8f77826b42f9b0956a0fbd823cdf7505`;
all 352 files are retained and all 164 input files remain unchanged. The exact
stage and upload are verified removed at 01:17:40.718375624 UTC after repeated
inventory/hash, privileged all-UID no-use and existing native.lock checks.
[Custody and retirement details](/home/harsh/.codex-tmp/ferric-perf-swarm-v6/proposals/perf-v10-a006-retrieval-r1/README.md)
preserve the separate logical and allocated storage measurements.

After these isolated R4 and active-poll comparisons, the next narrow evidence
step is full-model R4 qualification or qualification of the existing packed-down
FP32 path, including all packing, setup and transfer costs. Preserve exact
outputs and the full ownership/resource checks; neither next step has a claimed
end-to-end win. Token programs are already integrated and were
measured with the historical `807f0bef` runtime: ordinary/token TPOT is
60.702/55.296 ms in AB and 54.161/55.297 ms in BA, with 2,048 exact output tokens
and no repeatable gain. The retained ABBA archive SHA256 is
`421788f3be9ebc42dcd590dd1e3c208b605415eedb640afa88d442aa0a98967c`.
The earlier 4fb8 source review indicates the same minimum ten currentness checks per
backend group for token and matched ordered64 paths, plus five outer token
checks. This is source-count reasoning, not measured token counters; token
framing alone does not reduce those per-group checks. The private stable Rust 1.97.1 toolchain
passes its full retained inventory after restoring 11 inner directory modes
from 0700 to their recorded 0755; its reported version is 1.97.1. Initial client
metadata fails on missing `onig`. A separate `client-cargo-home` is now installed
with 293 exact registry package versions, 260 sparse-index entries and a verified
copy of the private bare Git databases, leaving the compiler cache unchanged.
October 1 offline metadata and private lock regeneration succeed. The first
focused client check exposes the mixed-SDK batch32 roster boundary described
above. Its bridge fix and scoped CPU qualification pass, and the separate
six-request active-poll full-model campaign now passes. This does not relabel
historical measurements or grant broader performance qualification.

Existing full-model diagnostics already use ten 64-packet groups plus one
12-packet tail for every 652-packet decode: 127 decode batches, 82,804 dispatches
and 1,397 groups. There is no missing decode-packing implementation to add.
The retained host-timing diagnostic records 8.207 s decode batch wall, 7.426 s
worker publication/completion wall and 0.0217 s controller packing. A separate
whole-request counter run records 10.025 s command scope, 9.258 s wait scope,
0.573 s operational-currentness scope and 92,969 polls, without a prefill/decode
split. These overlapping scopes cannot be summed or treated as GPU time;
roundtrip-minus-worker is not pure IPC. They motivate measuring the existing
core active-poll policy against the unchanged sleep policy, not predicting a
gain or weakening checks. The completed finite native comparison above records
the small latency changes and substantial CPU cost; the Ferric selector stays
default-off and unpromoted. Decode grouping is already
full except its tail; prefill
group utilization remains a distinct analysis. Evidence/source review is
`E/proposals/perf-v10-runtime-isolation-review-r1/review.json`; packet grouping
is audited in `E/proposals/packet-ticks-native-r4-analysis-r1.md`.

The earlier mi350 observation found unrelated vLLM services, which were untouched;
the October 1 read-only check instead finds all eight GPUs idle. The
authorized mi350-2 has one MI350X and is reserved here for a separately labeled
finite diagnostic, not TP8, vendor comparison or historical same-host timing.
This finite correctness/timing result is not rejection/trap qualification, GPU
time, TTFT/TPOT, serving throughput, model promotion or a vendor comparison. No
compiler or model speedup is claimed; the narrow grouped R4 latency observation
above is not general kernel qualification. Measure each isolated change before promotion.
The retained `RUNTIME_F2DD_AUDIT.md` reviews the separate runtime migration;
latest compiler and runtime adoption remain open and are not implied by this
pinned candidate campaign.

R1 retirement a001 removed 26,185,728 allocated file bytes. V9 retirement a003
removed 43,307,008 bytes after two refused checks. R2 retirement a002 removed
27,934,720 bytes after its first attempt refused process churn before deletion.
All three target directories are verified absent; total allocated file storage
reclaimed is 97,427,456 bytes, about 92.9 MiB. Earlier failed checks remain in
the record. Eight retirement-binding fixtures pass for each V10 stage; the initial
R1 fixture's overly narrow error-message assertion and its correction are kept
in the raw record. No process or borrowed model was removed.

After retaining both fixed-safe attempts, the integration lead removes only
completed mi350-2 stages `/tmp/ferric-opt-v10-fixed-safe.bce279a1` and
`/tmp/ferric-opt-v10-fixed-safe.19e4c57b`. A subsequent nonquiet SSH `stat` verifies
both absent. This retirement reclaims 273,275,815 logical file bytes; it is not
an allocated-disk-byte measurement. No other stage or shared work is touched.

The first r3-versus-r4 stage `/tmp/ferric-opt-v10-fixed-safe.74ab3e90`
is also verified removed after retaining its complete failed archive and a
privileged `lsof` check finding no open users. No reclaimed-byte amount is
asserted for that retirement. Retry stage `0c21e874` is now verified removed
after complete evidence retention and repeated no-use checks, reclaiming
46,604,288 allocated stage bytes. A003 retirement is tracked separately.

The [per-change measurement rule](M1_PERFORMANCE_SWARM_V9.md#per-change-measurement-rule)
applies. R3 and R4 are measured and not promoted; paired-load K2 prefill remains
a separate pending native measurement.
