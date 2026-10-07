# Performance Swarm V14

Updated October 5, 2026 (PDT). Implementation is in progress. No V14 latency
improvement or qualification is claimed. All 33 M1 gates remain open.

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
| Runtime | v14_runtime | Bounded compact program storage, one publication/final wait, all-signal retirement, fault tests | Implementing against fe2o3 `40695ecf5ffd7f3ec2bca8fd138a89cb675b57c6` |
| Ferric | v14_ferric | Explicit backend selection, same 652-dispatch/180-update graph, composition tests, next kernel/prefill candidate | Source audit and integration |
| Verification | v14_measurement | Container-aware GPU attribution, ABBA analysis and machine-readable per-change results | Implementing |
| Integration | root | Current-runtime baseline, serial remote builds, correctness, paired native measurements, custody and cleanup | Remote hosts reachable; reclaiming obsolete owned build artifacts |

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
| V14-00 | Current-main compatible control | Pending | Pending | Not measured | Not measured | Required baseline |
| V14-01 | Compact whole-token submission | Pending | Pending | Not measured | Not measured | Default-off |
| V14-02 | Container-aware attribution and paired results validator | Pending | Not applicable | Not a speedup | Not a speedup | Measurement prerequisite |

## Resource And Source Boundaries

All builds/tests run remotely under the existing four-core G28 guard on mi300x:
8 GiB RSS, nice 19, one build at a time, 20-minute timeout, 28 GiB private stage,
512 MiB reserve and unchanged host-space checks. GPU work runs on mi350 with
fresh resource/identity checks. No local or GitHub-hosted builds.

The existing conflicted fe2o3 checkout is read-only. Core work uses an isolated
worktree at the refreshed main revision; no unrelated changes are reverted.
Generic compiler/KFD work belongs in fe2o3; kernels and inference in Ferric.
Core changes must rebase onto latest main before any push. Owned worktrees and
temporary stages are retired after their patches/results are retained.
