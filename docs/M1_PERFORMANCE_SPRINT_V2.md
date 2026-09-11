# Performance Sprint V2

Started 2026-09-11. This is an engineering work log, not a benchmark result or
protected M1 qualification receipt. Prior measurements remain frozen in
[the performance ledger](M1_QWEN3_PERFORMANCE.md).

## Team Ownership

| Team | Deliverable | State | Acceptance Gate |
| --- | --- | --- | --- |
| Core runtime | Additive checked concurrent mixed-rank dispatch round in fe2o3 | Published on main at `79706b43a177a2fd3fa43ec328221fa3e5041af5` after a fresh no-op rebase. 469 KFD tests, 31 doctests, strict scoped Clippy and independent review pass. Native TP2/TP8 and all six matched model profiles pass. | This sprint's comparison is complete; peer setup/teardown remain follow-ups, and engineering execution is not protected qualification |
| Kernel numerics | Scalar/MFMA differential fixtures and diagnosis of the rejected TP1 path | The immediate token flip is a BF16 final-logit tie, not an argmax/layout defect. The separate FP32 head has latest-compiler emission, nine passing native fixtures and three passing fixed-reference TP1 model profiles. FP32+MFMA resolves the observed seed mismatch; the historical BF16 MFMA rejection remains unchanged. | This short-workload gate is complete; broader numerical coverage remains necessary |
| Measurement | Opt-in full-path host timing and strict profile summaries | All six TP2/TP8 transport profiles and all three TP1 head profiles pass fixed-reference, original-sidecar, identity and teardown checks. Only the two approved v7 pairs are compared; historical ledgers remain separate. | Completed n=1 matrices; repetitions and larger workloads remain follow-ups |
| Integration | Fresh dependencies, concurrent-round worker/transport, serialized GPU experiments | Active pins follow public `5110577a`. The latest combined gate passes 273 adapter Rust tests, nine kernel tests, strict Clippy/release, 27 metadata configurations, exact inventories/source/negative gates and 31 verifier policies. Both image-bound loader tests and the matched FP32 model runs pass. The rebuilt KFD worker is byte-identical to public797. | Final Pages checkpoint, archival and cleanup; protected M1 qualification remains open |

## Constraints

- Builds, tests, formatting and Python execution run only on `mi300x` in owned
  private stages, with bounded build concurrency.
- The integration lead alone schedules GPU execution on `mi350`, after checking
  shared-host availability. No overlapping experiments or foreign process kills.
- Core compiler/KFD changes belong in fe2o3 and require fresh fetch/rebase before
  publication to main. Kernel, inference and measurement changes stay in Ferric.
- Previous scalar/host-staged profiles remain available as controls. A concurrent
  profile must have a distinct name; serial dispatch is not reported as overlap.
- Correctness precedes timing acceptance. Setup, host wall time and measured GPU
  time are distinct. No synthetic or rejected output becomes an accepted gain.
- Preserve user worktrees and shared model caches. Archive evidence and remove
  completed owned worktrees and temporary build stages.

## Scope

The first integration targets runtime overhead, real rank concurrency and
numerical diagnosis. Fusion, broader batch/chunk tuning and persistent serving
lifetime changes follow measured attribution; they are not implicitly complete
because the profiling hooks exist. Symmetric memory and MTP remain deferred.

## Concurrent-Round Native Gate

The frozen candidate child (`2032e31b`), built from the now-public core `79706b43`
using a recorded private Cargo path patch, passes both TP2 and TP8 probes. Each
checks 12 real mixed-rank rounds at one, three and sixteen rows: GPU BF16 producers
feed peer readers, and GPU FP32 producers feed ordered peer reductions. All
outputs, guards and tails match; close acknowledgements, zero exits, child reap
and all-eight-card idle snapshots pass. These are correctness checks, not device
overlap timestamps or Qwen benchmarks.

The first TP2 probe stopped before dispatch because its harness incorrectly
required optional pointer metadata omitted by the frozen image. Its failed receipt
is preserved. The revised harness requires the exact absent-field contract and
rejects non-null annotations and ABI mutations; runtime admission was unchanged.

## Numerical Checkpoint

The TP1 diagnostic report is SHA-256
`31f9fa2ff265c2d76627c7e071b912f6eb102572f2914744e8435a00797c77b5`.
It uses the frozen `8c81d3fe` full-v3 image and `189b918d` independent worker;
neither artifact is relabeled as a fresh build. The worker closes/reaps and a
delayed snapshot confirms all eight GPUs idle. The immediate snapshot retained
one percent utilization on card zero with zero allocated memory; both snapshots
are preserved, rather than replacing the first observation.

These are synthetic real-shape correctness diagnostics, not model parity or
performance results. Dense FP32 partial outputs differ at many bit positions
between scalar and MFMA, while sampled MFMA error versus the higher-precision
sum is smaller. This does not establish the model token mismatch's cause.

## Actual Model Capture

The separate bounded capture controller (`b7f33190`) preserves actual scheduler
rows and projection/head operands, never performance-qualified timings. Both
TP1 runs close cleanly and pass payload hashes, bounds, row binding, complete
observed-trace and actual head argmax checks. The independent fixed-reference
check passes baseline and rejects MFMA BF16 with the unchanged seed mismatch.

For selected batch two, layer zero Q input and original weights are byte-identical;
the complete MFMA weight transpose is exact. Four of 24,576 Q outputs differ.
The final normalized inputs differ at 21,314 of 24,576 BF16 elements, but this
capture does not attribute those differences to only the four observed Q values.
Seventy-eight shared selected LM weight rows are byte-identical.

On the seed row, baseline BF16 logits are 24.5 for token `17689` and 24.375 for
`9856`. MFMA's BF16 logits are both 24.375, so the existing lowest-ID argmax
correctly selects `9856`. On MFMA's actual final input, the scalar FP32 reference
is 24.42636108 for `17689` versus 24.36589813 for `9856`; FP64 also favors `17689`.
Both FP32 values round to the observed BF16 tie. All 106 captured top/watch head
outputs per variant match serial FP32 accumulation followed by BF16 narrowing.
The immediate precision-loss mechanism is established; no generic argmax defect
or qualified full-run fix is claimed.

Pair analysis SHA-256:
`e26a5dece9ecf1a42efa8b1e9d3f51164df7b0e4e18c43ea3be84e23fd22fe52`.
Independent diagnostic reports are
`582aef5bc0e67e0b910373c1d7026999327150376b6b900ea1ef162acda91197`
(baseline reference pass) and
`aad0cabe048c95b6fa13b61346701d49eafd80b1db33572ffd92aed39e283fe8`
(MFMA reference failure). Their schemas permanently exclude performance claims.

## Matched TP2 Checkpoint

Same frozen timing controller, base image, workload and policies; serial/round
also use the exact same peer child and peer image. All rows below pass eight
fixed-reference output tokens, complete dispatch/row accounting and idle/close
checks. Each is one short engineering observation, not a serving or stable-tail
result. Host versus peer also changes collective placement and worker graphs.

| Mode | Output Tokens/s | Workload Seconds | Setup Seconds | Reuse TTFT Seconds | Reuse TPOT Seconds |
| --- | ---: | ---: | ---: | ---: | ---: |
| Host staged | 0.871547 | 9.179084 | 106.238880 | 1.697529 | 1.044582 |
| Peer serial | 0.143430 | 55.776348 | 221.343087 | 11.302857 | 10.399142 |
| Peer concurrent round | 0.268161 | 29.832794 | 221.191863 | 6.029658 | 5.583170 |

Concurrent rounds improve workload output rate 86.96% over the serial peer path,
but remain 69.23% below host staged. They do not fix peer upload or teardown
overhead. Full TP2 performance ledger SHA-256:
`085a233fdc654e6cf75beee9c3bc1dc8dd42c6a2d9b7ac2216c2a1692702c420`.
Serial-baseline isolated pair:
`2297d9c4b8067b59c3c809efc38ba424a48ce12def680fb136bf224e9106c1d0`.

## Completed TP8 Matrix

The same acceptance requirements and one-observation limits apply. All eight
output tokens match; five batches process 34 physical rows. The serial/round
comparison keeps the exact peer binary and images unchanged.

| Mode | Output Tokens/s | Workload Seconds | Setup Seconds | Whole Seconds | Reuse TTFT Seconds | Reuse TPOT Seconds |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Host staged | 0.380674 | 21.015336 | 135.388216 | 171.034465 | 3.534553 | 1.999206 |
| Peer serial | 0.011286 | 708.819742 | 736.568782 | 1565.344075 | 142.227667 | 139.406734 |
| Peer concurrent round | 0.076712 | 104.286367 | 737.535894 | 961.521453 | 20.909556 | 20.617757 |

Concurrent rounds improve output rate 579.69% (6.797x) over serial peers and
reduce request TTFT/TPOT about 85.3%, but remain 79.85% below host-staged output
rate. This is not an overall peer win or a stable serving result.

Selected non-overlapping host spans, host/serial/round respectively: attention
5.440/413.946/58.797 seconds; FFN 3.178/210.609/29.843; both collectives
12.170/78.029/10.217; metadata 0.0158/4.436/4.438. Resident setup is
47.309/643.611/644.010 and close is 14.631/119.955/119.699 seconds. Rounds reduce
serial dispatch spans but do not improve metadata, resident setup or close.
These observations do not isolate GPU durations or individual system calls.

Full TP8 performance ledger:
`d2e86b755c7b76915888f26ec4a7049e71981cb11b7e0ef43352130539575b64`.
Full timing summary:
`76ac7c137b67e05aeb07118947ee42fe2611b5a92b46d03ddc761529d6a38366`.
Isolated serial-to-round performance ledger:
`d4a52661591b145eb24dc4573c90b368eb813c147612a0981cf95b20252294cf`.

## Latest Source

Public fe2o3 advanced to `5110577a6d8c45390dfb353386cde748efd5d76c` during
the matrix. Its only delta is shuffle-uniformity analysis and one new regression
target; runtime, device SDK and lockfile sources are unchanged. Active Ferric
pins and the compiler commit/tree pair follow this revision; frozen matrix
artifacts retain their actual identities. A separately rebuilt latest KFD worker
is byte-identical to `aaa0216a`. The v7 head image was separately re-emitted
with the latest compiler and exact replay, HSACO `d6086650`; its earlier public797
emission is retained as history. Native arithmetic/custody checks and the three
fixed-reference TP1 model cases below pass; broader numerical qualification is
not established.

Independent repin review found one stale compiler tree in the promotion
behavioral harness. Its commit/tree pair was corrected at public797 and now
matches public511 tree `7f1329b68e00f3325d3ed2b977a4b6ba7cc20542`; the remote repin gate checks
both together. This narrow identity correction does not claim a full rerun
of that behavioral harness.

## FP32 Head Native Gate

The separate v7 image adds scalar FP32-output head, MFMA FP32-output head and
FP32 argmax roots. The opt-in TP1 host path retains the original BF16 allocation,
adds 9,723,904 bytes only for FP32, and leaves non-head arithmetic unchanged.
Its explicit BF16 control loads the same extra image without allocating that
workspace or changing the old head/argmax dispatches. Default execution is
unchanged; neither path relaxes the fixed token reference.

All nine native cases pass at one, three and sixteen rows using the latest511
worker and image. An independent integer-scaled reference verifies every active
output and inactive tail; protocol review verifies immutable inputs, exact
dispatch payloads, guards, 24 freed allocations, clean close/exit and all-eight-GPU
idle snapshots. Native result SHA-256:
`fb2c23697b2fe144534df214225f44d14359f3f4fe6810ae766f11b07721b69b`.
This is not full-model parity or a performance result.

The first latest511 combined adapter gate stopped on two exact source-policy
inventories missing the newly declared optional v7 dependency. `00fa58d` adds
only that name to both lists; exact equality and production/default isolation
remain intact. All 273 adapter Rust tests, strict Clippy and release now pass;
the full repin, nine kernel tests, six peer tests plus its doctest, and 80 Python
comparison tests (one explicit skip) also pass. Additional suites pass eleven
launcher, four capture-replay and four capture-pair tests. The repin verifies
27 locked metadata configurations, exact dependency/module inventories,
38 source-gate tests, source/negative gates, 31 verifier policies and the
protected release policy. These are host/source checks, not a new Verus or M1
qualification. The original failed receipt is retained.

The image-bound gate first rejected a staging-only layout mistake: the copied
content directory lacked the required immediate `fe2o3-engineering-v1` parent.
Correcting the filesystem layout without changing source or image bytes makes
both CLI image-bound tests pass, including duplicate roots, invalid second-load
metadata and partial-load poison/child reap. Four native-harness host tests and
its CPU-only self-test also pass. The failed receipt is retained separately.

The three matched model cases use release controller
`3d039c49bf4a4610f3b704d52c17cc46398f0c0f010c060af66c369b8ea48192`,
built from `00fa58d` on latest511, the same latest worker and separately admitted
v7 image. BF16 control, FP32 baseline and FP32 MFMA all pass their full
fixed-reference runs and original timing/custody checks for this workload.

Completed core, numerical, v7 kernel and measurement worktrees/build stages have
been removed after checksum-verified archival. The root CPU integration stage,
including its 6 GB target, is also removed; its archive is SHA-256
`434492c9cf1df9dd8fa7dbd30d5fed4261337d3b3bb5e27f65c4f3234270b0cd`.
Nine performance-case directories and both actual-model captures were verified
against local hashes before remote deletion. A 46.3 MB GPU rerun/native-receipt
bundle and the active Ferric integration checkout remain intentionally retained.
Pages has its own scoped publication/cleanup receipts. Shared models, caches and
user worktrees are untouched; no inference worker is left running.

## Matched FP32 Head Results

The approved prelaunch plan is SHA-256
`4cfe035142fdd4751106cef6781d2238a1316c25f9e22625bd3975eb1daeb199`.
All three cases keep TP1, the same controller/worker/base/v7 images, four fixed
requests, eight outputs, prefix caching, sixteen-row/chunk budgets and operational
validation. Each executes rows `[16,6,7,4,1]`, five batches and 2,720 dispatches.
Complete token IDs and decoded bytes match the unchanged reference; close/reap
and before/after all-eight-card idle checks pass. Independent review reproduces
the three strict comparison reports exactly and checks the logical traces and
intended workspace/profile changes separately.

| Profile | Output Tokens/s | Workload Seconds | Setup Seconds | Whole Seconds | Reuse TTFT Seconds | Reuse TPOT Seconds |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| BF16 head control / baseline projections | 0.833118 | 9.602485 | 101.510401 | 112.979126 | 1.884560 | 1.191099 |
| FP32 head / baseline projections | 0.810583 | 9.869445 | 103.747054 | 115.498046 | 2.119207 | 1.375666 |
| FP32 head / MFMA projections | 2.059318 | 3.884780 | 122.058354 | 128.706665 | 0.626952 | 0.487188 |

Each row is one short observation, not steady-state serving or a stable tail.
The precision-only pair observes 2.70% lower workload output rate, not a gain.
Under the same FP32 head, MFMA improves workload rate 154.05% (2.5405x).
It also increases setup and whole-process time: projection preparation takes
18.936 seconds, so this fresh-process run is slower overall. Selected host
attention/FFN spans fall from 4.408/4.132 to 1.691/0.759 seconds; these are not
isolated GPU kernel durations.

The previously failing seed now produces `[17689,374]` (` Spain is`) under
FP32+MFMA, and all four request traces match. This establishes a working opt-in
path for this fixed workload, not a generic numerical fix, new default, HTTP
serving readiness or a vLLM/SGLang comparison. No rejected historical BF16
profile is retroactively repaired or admitted into a performance ledger.

Three-case/two-pair timing report:
`38ba909a53c3b6d4ae491b22ffbe56927ff50b484dd445fc048eb8184f55b97a`.
Precision-only pair:
`4ab3737b072c6aa52a84c313614ab22d38cc4fc62c5a4cff4ddcb9f2033581a6`.
MFMA-under-FP32 pair:
`6622bd900a2ab97fbdb6314b3840498bb1c7064ddef85cd02a65e7302f62d4f7`.
Independent review:
`f602265a2be148c59ac80f37bc7bf427ea66109c29f9e3da1a2a7f38f7828a8a`.

## Remaining Work

These implementation and comparison gates are complete, not all performance
work or M1. Priorities from the observed costs are persistent serving lifetime,
peer resident-upload/teardown overhead, and further phase-level profiling before
fusion or batching changes. Repeat and broaden the accepted profiles before
changing defaults. All 33 protected M1 gates remain open; symmetric memory and
MTP are still deferred.
