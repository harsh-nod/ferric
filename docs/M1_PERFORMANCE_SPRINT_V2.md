# Performance Sprint V2

Started 2026-09-11. This is an engineering work log, not a benchmark result or
protected M1 qualification receipt. Prior measurements remain frozen in
[the performance ledger](M1_QWEN3_PERFORMANCE.md).

## Team Ownership

| Team | Deliverable | State | Acceptance Gate |
| --- | --- | --- | --- |
| Core runtime | Additive checked concurrent mixed-rank dispatch round in fe2o3 | Published on main at `79706b43a177a2fd3fa43ec328221fa3e5041af5` after a fresh no-op rebase. 469 KFD tests, 31 doctests, strict scoped Clippy and independent review pass. Genuine TP2/TP8 producer/reader gates pass. | Model performance comparison; engineering execution is not protected qualification |
| Kernel numerics | Scalar/MFMA differential fixtures and diagnosis of the rejected TP1 path | Synthetic and actual-model captures pass integrity checks. The immediate token flip is a BF16 final-logit tie, not an argmax/layout defect. Baseline still passes the fixed reference; MFMA BF16 still fails. A separate opt-in FP32-head candidate is underway. | Fresh three-root emission, native FP32 checks and full fixed-reference model validation; no reference relaxation |
| Measurement | Opt-in full-path host timing and strict profile summaries | Integrated. All six TP2/TP8 host, serial and round profiles pass fixed-reference, sidecar, identity and teardown checks. Separate explicit v7 comparison/timing tools pass 66 combined host tests. | FP32 candidate runs; failures excluded from performance comparisons |
| Integration | Fresh dependencies, concurrent-round worker/transport, serialized GPU experiments | Active pins now follow public `5110577a`, with 27 locked metadata configurations and the new shuffle-uniformity test inventoried. The separately rebuilt KFD worker is byte-identical to public797. The earlier public797 combined gate passes 266 Rust tests, strict Clippy/release, source/negative gates and 31 verifier policies. | Latest combined FP32 gate, native/model checks, archival and cleanup |

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
emission is retained as history. Native and model qualification are still pending.

Independent repin review found one stale compiler tree in the promotion
behavioral harness. Its commit/tree pair was corrected at public797 and now
matches public511 tree `7f1329b68e00f3325d3ed2b977a4b6ba7cc20542`; the remote repin gate checks
both together. This narrow identity correction does not claim a full rerun
of that behavioral harness.
