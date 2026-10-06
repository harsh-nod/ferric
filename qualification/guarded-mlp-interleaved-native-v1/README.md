# Delayed two-bank guarded MLP reuse

This checkpoint extends the [borrow-free retained pair](../guarded-mlp-retained-pair-v1/README.md)
with a native two-layer, two-bank, four-forward diagnostic. Both CPU suites pass.
The first native attempt fails an allocation-role check; the diagnostic-only
rebuild passes the synthetic GPU case. The first refusal is **not root-caused**,
and repeatability remains open. This is not a production route or model benchmark.

## What Changed

The [native fixture](cpu-attempt-v2/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_interleaved_native_v1_tests.rs)
allocates four retained pairs, each owning two genuine 2,208-byte combined MLP
states. It retains the same eight owner allocations and private payloads for
the complete schedule. The shared dense Gate, Up and Down matrices are allocated
once per rank. Only the nonzero Up coefficients change between completed runs.

| Forward | Bank | Local Guard Generation | Layer 0 Scale | Layer 1 Scale |
| --- | --- | --- | --- | --- |
| 0 | 0 | 1 | -1 | +1 |
| 1 | 1 | 1 | -2 | +2 |
| 2 | 0 | 2 | -4 | +4 |
| 3 | 1 | 2 | -8 | +8 |

The intended schedule executes paired R1, combined MLP, guard validator, peer
barrier and R2 segments. Both ranks must complete before host writes or the next segment.
At forwards 2 and 3, one rearm call resets both selected layer pairs. The old
proofs must remain valid after work in other layers and the other bank advances
the queues. Fresh arenas are retained until queue-first Close, not recycled.

A test-only [owner observer](cpu-attempt-v1/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_retained_v1.rs)
reads real owner state and current queue frontiers under exclusive group custody.
It exports scalar identities, not reusable buffer tokens or completion proofs.
It does not recheck or update the retained proof, so the later rearm still tests
that proof's first validation after intervening work.

The fixture checks every completed segment's active stages and hashes all inactive private
payloads. Unused pairs must still contain the NaN poison; previously used pairs
must retain their last numerical result, even after state-only rearm. All eight
scale cases have different final values at every element. Dense zero padding
remains positive zero when the nonzero coefficients are negative.

## First CPU Qualification

The actual build and tests ran on `ssh mi350`, with the retained nightly
2026-04-03 compiler and offline locked dependencies. No local build was used.

| Evidence | Result |
| --- | --- |
| [CPU terminal](cpu-attempt-v1/evidence/complete.json) | 1,057 passed, 0 failed, 7 ignored across five targets |
| Inventory | 1,064 tests total; 1,044 in the library |
| New tests | Three numerical/schedule tests and one pre-GPU observer refusal |
| Native diagnostic | Compiled, ignored by ordinary CPU tests |
| Lifecycle | 16 phases, natural zero exits, reaped children, absent process groups |
| Source map | 802 compiler files plus two harness files; unchanged throughout |

All 1,059 previous named outcomes are preserved. There are no timeouts, forced
cleanup or postcheck errors. The four-file source overlay replaces three files
and adds the native fixture; production behavior is unchanged by the diagnostic
observer, which is compiled only for tests.

The [data-only retainer](retain_cpu.py) checks the actual archive and all 110
pinned bodies before retaining 111 members (5,866,572 bytes expanded). The
933,711-byte archive SHA-256 is
`2b5fc103dddfc463414dd5f5d50ffa8e923feb298140e70dbdecdc7197beee3b`.
The receipt is `09746cb353c2b1a6aa8e64a6aaefbb6451d9a538893856bac019105831e351f6`.
The selected library executable is 11,245,904 bytes with SHA-256
`dbf5ea87824b4ab2be1e481cf21595f5809fb95bd309552e888a32f8089a8425`.
Executable metadata is retained, not the host executable body.

## First Native Attempt

The [actual terminal](gpu-attempt-v1/evidence/failed.json) records one requested
native call, no retries, and a natural exit code 101. The
[native stdout](gpu-attempt-v1/evidence/native.stdout) reports
`paired guarded MLP typed allocation role`. There is no successful observation
or independent GPU-output verification. The message does not identify the
rejected role or establish how many packets, if any, were dispatched.

| Actual Evidence | Result |
| --- | --- |
| Independent reference selftest | Passed: 16 final hashes, 16 Up-matrix hashes, 640 mutation refusals |
| Strict parsing checks | Two JSON refusals and two contradictory-stdout refusals passed |
| Native wire bound | 3,661,174 bytes, below the unchanged 4 MiB captured-stream limit |
| Native libtest | 0 passed, 1 failed, 1,043 filtered; 2.64 seconds |
| Owned lifecycle | Seven phases, natural exits, reaped children, absent process groups |
| Cleanup and postchecks | No timeout, forced cleanup, storage failure or postcheck error |
| GPU telemetry | All eight GPUs idle before and after the attempt |

The [independent integer verifier](gpu-attempt-v1/verify_native.py) checks the
intended eight-segment/two-rearm ledger and every exported numerical bit. Its
target counts are 557,056 computed values, including 65,536 final BF16 values,
48 dense-matrix hashes, 104 owner readbacks and 336 inactive-payload hashes.
These are **not outcomes of the failed first attempt**. The
640-mutation selftest uses synthetic observations and passed in 3.50 seconds.

Runtime library admission precedes the first ELF invocation. Child streams and
files remain capped at 4 MiB; compact parent receipts alone permit 8 MiB because
they also retain source/runtime metadata. The whole-run limit is 900 seconds,
native child limit 600 seconds, with 40/38 GiB setup/live disk floors.

The [data-only GPU retainer](retain_gpu.py) preserves all 53 members / 52 pinned
bodies (2,861,223 expanded bytes), including raw failure streams, actual commands,
input hashes, three exact HSACOs and CPU ancestry. It retains no host executable
body and does not replay inputs. Archive: 409,192 bytes,
`4c3f37eef38cc0ed43b5c6154a4b882cfb12da0c396de2217f49ddceaac31d46`.
Failed receipt: `559b515035b9a4902c24375798b7c60b2274d4470f5651c91743f583573823e8`.

## Instrumented Rebuild

The second attempt changes only refusal diagnostics: the typed-role error now
reports scalar buffer identity, actual/expected owner and extent, and kind/mapping
booleans. Allocation, rearm and run errors carry pair/forward/bank/layer context.
No acceptance predicate is relaxed and no address or reusable token is exposed.
The [second CPU terminal](cpu-attempt-v2/evidence/complete.json) again passes
1,057 tests with seven ignored, across 16 clean phases.

The [second native terminal](gpu-attempt-v2/evidence/complete.json) passes.
The [independent verification](gpu-attempt-v2/evidence/verify.stdout) checks
actual exported bytes and the complete event ledger:

| Actual GPU Check | Result |
| --- | --- |
| Schedule | Eight paired segments; two delayed whole-bank rearms |
| Numerical values | 557,056 computed values, including 65,536 final BF16 values, bit-exact |
| Dense matrices | 48 hashes checked; 20 distinct bodies |
| Owner observations | 104 compact readbacks plus 16 full terminal snapshots |
| Inactive payloads | 336 hashes unchanged or equal to their prior expected results |
| Lifecycle | Eight natural zero-exit phases, reaped children, absent process groups |
| Shutdown | Healthy queue-first Close; all eight GPUs idle before and after |

The 112.49-second native phase includes fixture construction, dense writes and
readback/hash checking. It is **not GPU kernel latency or model throughput**.
The whole diagnostic took 118.44 seconds; there was one native call and no
internal retry. This is a new attempt with a newly compiled executable, not a
repetition of the first binary.

A successful diagnostic-only rebuild does not establish why the original binary
refused its allocation role. Do not describe this as a functional fix or erase
the failed attempt. Repeat both pinned host executables in separate fresh
processes and investigate their differing behavior before broader admission.

The [second CPU retainer](retain_cpu_v2.py) preserves 112 members / 111 pins
(5,882,313 expanded bytes); [second GPU retainer](retain_gpu_v2.py) preserves
59 / 58 (13,792,717 expanded bytes). Neither replays the retained project.

| Artifact | SHA-256 |
| --- | --- |
| CPU receipt | `3adc90f6f97584787386b2d150e61a05a6ac5ea50d819111530b07755b98d47f` |
| 11,240,400-byte library ELF | `5dfefcd2e2280b6dc83e63a241c3f518ab3bfa38fa705a4cf070d886ac6d47f0` |
| 936,408-byte CPU archive | `3871dbc64b7f557ab9b02d980fc83fa95a4c3a6f21baa7442486881e4df3abad` |
| GPU receipt | `ab17388f411190bf0c738fdf8a7ed4b598606e3147ce98224573ea7ebb2c25d9` |
| 569,836-byte GPU archive | `a26f86e93d7dee69a909701595ff187e910d100ff10a069dccb5277834a837e7` |

## Scope and Next Gate

The two synthetic layers have independent fixed inputs; this is not a model
layer chain. It does not validate attention, KV-cache updates, arbitrary model
weights, Prefix284 whole-bank reset, or logits. The selected MLP image rounds
SiLU to BF16 before the Up multiply; it is not interchangeable with a fused
single-narrowing model reference.

Integrating the guarded route into Ferric still requires whole-bank validation
of both Prefix284 and combined MLP states before any reset, local generations
separate from the global forward ledger, no duplicate R2 dispatch, and bounded
arena reclamation. Full-model independent correctness, the Qwen3-8B BF16
2,048/256 workload, performance/overlap evidence, 700 tokens/s and issue #42
milestones M0-M7 remain open. This checkpoint makes no throughput claim.
