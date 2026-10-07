# Performance Swarm V3

Started 2026-09-18 UTC at user request. This is a working implementation
tracker, not a benchmark receipt. No new performance result is admitted.

## Baseline And Objective

The retained Qwen3-8B 128-input/128-output HTTP cell uses one MI350X, BF16
decoder weights, an explicit FP32 output head, concurrency one, and prefix
caching/speculation off. Ferric C1 observes 2,827.804 ms mean TTFT,
154.310 ms mean TPOT and 5.707624 output tokens/s; retained vLLM observes
19.243 ms, 4.414 ms and 220.558111 tokens/s. These single-start finite cohorts
are not a current-source or sustained-load qualification. SGLang's numerical
failure remains excluded from rankings.

The objective is measured improvement against correctly matched baselines,
not completion of a feature checklist. The existing
[performance policy](PERFORMANCE.md) controls any competitive claim. Raw
failures, source/artifact identities and earlier observations stay unchanged.

## Team Ownership

| Team | Bounded Deliverable | State | Validation Dependency |
| --- | --- | --- | --- |
| Runtime profiling | Strict live request/batch host timeline joined to runtime snapshots; no inferred GPU duration | Implemented; independent review and remote tests/replay pass | Actual GPU-clock attribution remains unavailable |
| Kernels | Separate opt-in gfx950 single-token draft wave-GEMV candidate and closed host launch contract | Isolated v16 source reviewed; 46d, 47d and f713 host gates pass | Device emission, native and model parity |
| Serving measurement | Opt-in V4 same-eight-GPU allocation and speculation identity contract in continuous-series analyzer | Implemented; independent review and remote tests pass | Real same-timeline serving captures |
| Integration | Review, current compiler audit, execution-host readiness, resource admission and source/result tracking | CPU validation resumed on user-authorized mi300x-2 | Reconstruct exact native benchmark inputs |

No agent may launch competing GPU jobs. Integration serializes admitted native
runs and leaves unrelated users' processes untouched. No extra worktree is
created. Ferric kernels/inference remain private; only generic compiler/KFD
changes belong in fe2o3, rebased before any authorized main push.

## Six Workstreams

1. Attribute runtime cost before choosing the next hot-path change. Existing
   worker counters and controller spans overlap; the new report will not sum
   them as exclusive time, label host waits as GPU time, or assign an entire
   shared batch to one request. Real GPU timing remains a separate dependency.
2. Investigate reusable submissions and fewer host waits. Existing ordered
   submissions are already bounded to 16 dispatches and TP1 in the Ferric fast
   path. Widening that boundary requires core/wire/ring ownership review and is
   not silently part of this change. No runtime checks are removed.
3. Develop the opt-in draft M=1 wave candidate independently of existing C1
   target kernels. Its reduction association differs from ascending-K scalar
   arithmetic, requiring explicit numerical and model validation. It does not
   qualify gfx942 or change the default kernel roster.
4. Compare 8xTP1, 4xTP2, 2xTP4 and 1xTP8 within the same eight physical devices.
   Use one coordinated arrival/drain timeline and latency-SLO goodput; never
   add throughputs from separate cohorts. Draft GPUs count toward the budget.
5. Tune batching and speculation only after exact request-driven validation.
   Target-only and speculative comparisons are distinct. Draft/verification/
   rollback cost and accepted output count must justify speculation; a fixed
   canary acceptance result is not a serving speedup. The V4 analyzer is
   measurement infrastructure, not new batching or speculative execution.
6. Reestablish accepted vLLM/SGLang baselines, including the outstanding
   SGLang output mismatch, and execute the repeated held-out comparison policy.
   No schema adjustment can erase that numerical failure.

## Host And Dependency State

At 17:59:43 UTC, passive mi350 checks find amdgpu and /dev/kfd restored, eight
GPUs at zero busy percent and 297,766,912 bytes allocated on each. This is not
a reservation or launch admission. Post-reboot device bindings must be checked
again immediately before a run.

The old /tmp/ferric-compete-gpu.VabkOGCx, /tmp/ferric-qwen8.TRNKht and
/tmp/ferric-performance-gpu.vbSvPGT1 directories are absent. Their retained
local evidence is not deleted. Canonical model owner
/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0 and runtime owner
/home/harmenon/ferric-mi350-qwen-runtime-r1.tZnRF3nj remain present; their
existence alone does not authenticate current executable/model contents.
The serving team locates hash-matching retained historical controller, worker,
V5/V8/V11 artifacts and frozen comparison inputs. Those binaries can be
restored without rebuilding, but their old owner/lock identities cannot be
reused after reboot. Fresh model authentication must cover both target and
draft trees even for that controller's speculation-off route. Restoration of
historical binaries is not current-source qualification.

mi300x still reports zero available root-filesystem space. The user redirects
builds/tests to mi300x-2 (hostname sharkmi300x-3, UID 1046). Its admission at
18:10:52 UTC observes 636,087,196 KiB root available, 792,480,160 KiB tmpfs
available and 1,481,077,888 KiB MemAvailable. The existing 22-GiB root,
32-GiB tmpfs and 128-GiB available-RAM floors are unchanged. Private CPU-only
validation resumes there; no local or GitHub build is used. Native GPU runs
remain on mi350 with separate fresh admission.

The final upstream observation freezes fe2o3 main at
f7137ab8caa1fc0c9223f5fb21203b843209707c, after the initial 0e7520068 and
47d0650f reviews. The latest merge changes compiler source commitments,
execution-scope custody, helper-call lowering and emission budgeting. Device,
host and macro APIs and Cargo dependencies are unchanged from 47d, but native
emission behavior still requires its own validation. No new runtime/KFD timing
capability is introduced.
The main inference adapter remains at its validated 46d dependency pin. The
isolated v16 candidate now pins f713/7ebf6e6 after a separate successful host
gate. Full-adapter adoption and rebuilt native compiler/runtime/kernel products
remain separate work; retained binaries are not relabelled as current products.

## Evidence Ledger

| Item | Source Review | Executed Validation | Performance Delta |
| --- | --- | --- | --- |
| Live runtime profile | Independent review; two rejection gaps fixed and reviewed | R2 combined suite: 115 tests pass; retained C1 capture replay passes | None |
| Draft wave v16 | Independent source review passes | 46d/cc902, 47d/7ebf and f713/7ebf: 14 tests, formatting and Clippy pass on each | None |
| Eight-GPU V4 comparison | Independent review passes | Same R2 combined suite: 115 tests pass | None |

Every row is updated independently after actual completion. Unit tests, host
replay, kernel emission, native parity, model parity and performance comparison
are separate gates. No M1 gate closes from this tracker.

The first kernel host run compiles successfully but rejects a test fixture
whose whole-signature token comparison includes a syntactically irrelevant
trailing comma. The corrected assertion compares individual parsed arguments.
Baseline R2 passes all five contract and nine arithmetic/reference tests, plus
formatting and Clippy with warnings denied. The failure and original inputs
are retained separately. These tests do not execute GPU kernels.
The independent newer-pin run also passes all 14 tests and the same formatting
and Clippy gates. It uses the exact 47d/7ebf objects, an offline-generated
lockfile and Rust 1.97.1. No source replacement or inherited binding wrapper is
used. Both runs use one CPU, one Cargo job and private temporary/cache paths.
After upstream advances during testing, a third frozen f713/7ebf run passes
the same 14 tests, formatting and Clippy gates under those resource controls.
The final candidate adopts that tested pair, not an untested moving branch.

R1 ran 113 tests in 7.360 seconds. Independent review then identified missing
rejections for I/O duration without an operation and a physical batch span
larger than its enclosing live interval. R2 adds both checks and regressions:
115 tests pass in 7.394 seconds, and the retained C1 replay still passes. These
are CPU tests and replay of old evidence, not a new native measurement.

The replay contains two requests, 270 batches and 166,278 dispatches. Worker
deltas include 38,692,545,397 ns dispatch waiting and 8,771,928,159 ns
operational currentness checking. These overlapping host-wall counters cannot
be added as exclusive time or interpreted as GPU duration. The report records
GPU duration as unavailable. The source audit finds no admitted gfx950 GPU
dispatch-timestamp API; existing clock correlation is not that API.

A separate generic runtime candidate would remove one redundant deep clone
before ordered-batch payload validation by reusing its existing borrowed-slice
validator. No runtime patch is applied yet, no checks would be removed, and
the likely modest benefit remains unmeasured.

The completed Python phase is retained locally under
`.codex-tmp/ferric-performance-swarm-v3/python-retained-r2.tar.gz`, SHA256
`8bec2d591dbe9ee113a41b772b1924e7fcc27b185aa459832a76d91279185f2c`.
Every retained payload passes its hash ledger, and the four implementation/test
files match the reviewed working source. The R2 replay report hash is
`89870b4dd461c5d1b06dc62e5f5e216c9fa0e046242354b874a02e9eb966ed23`.
At 18:27:47 UTC its owned remote stage and redundant archive are removed after
retention verification and a fresh process snapshot check.

Kernel results, exact sources/locks, controls and six host test executables for
the 46d and 47d runs are retained in `kernel-retained-r2.tar.gz`, SHA256
`6093781c246dac9ed2fa1d7f6adafa2ebcc7ea2418cd55f4d9cb868d147646ab`, under
the same local evidence owner. The f713 run and its three executables are in
`kernel-retained-tip-r1.tar.gz`, SHA256
`92f4e4fae998e882b2411b21bea028fad916f23b8b28546f29fda9c42d64437a`.
Both archives pass their payload hash ledgers. Final candidate code and lock
match the f713 tested source; only its explanatory README changes afterward.
The owned Rust build stage, private caches/toolchain and redundant remote
archives are removed after retention and process checks, reclaiming about
4.7 GiB. No new worktree is created, and existing user worktrees are preserved.
