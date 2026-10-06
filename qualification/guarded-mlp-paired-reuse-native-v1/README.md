# Paired MLP GPU Reuse

Status: **two consecutive generations pass on MI350 using the same private
Session, atomic owners and payload allocations.** Both generations execute the
actual two-rank R1 -> tiled MLP -> validator -> paired barrier -> R2 coordinator.
This is synthetic component correctness, not full-model decoding or a speedup.

All issue #42 milestones and the single-request Qwen3-8B BF16, 2048-token prompt,
256-token decode, 700 tokens/s target remain open.

## What Runs

The [native reuse test](cpu-attempt-v1/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_reuse_native_v1_tests.rs)
uses the [CPU-qualified Session](../guarded-mlp-paired-reuse-v1/README.md):

```text
allocate owners and payloads once; load the three exact checked images
run generation 1 -> compare and capture actual VRAM results
rearm_next() -> revalidate both peers before either reset -> Ready at generation 2
double only Up weights; NaN-poison all writable intermediates and final outputs
run generation 2 -> compare and capture actual VRAM results
check final owner identity/generation/snapshots -> queue-first Close
```

The same Session remains alive through both runs and retains actual completion
snapshots privately. The test does not seed terminal state, bypass Session rearm,
or replace the paired coordinator with serial host dispatches. Both generation-2
guards must be `[2, 0, 1, 0]`; the MLP prefix is reset to its established initial
epoch 1. Guard generation and the MLP prefix epoch are distinct fields.

Owner identity is explicitly compared before and after execution. Payload
tokens remain unchanged in the retained Inputs and both readback passes. These
reuse claims are grounded in the pinned native source and its successful call,
not inferred from numerical equality or an independent allocation trace.

Each generation publishes five packets per rank: four kernels and one barrier.
Across both generations that is 16 kernel dispatches, four barriers and 20
completion signals. These are source-contract counts, not per-packet GPU tracing.
Signal/kernarg arenas are fresh for each generation and retained until Close;
this does **not** qualify signal reset, early reclamation or constant-space
arena reuse for sustained decoding.

The exact images are unchanged from the [one-generation diagnostic](../guarded-mlp-paired-native-v1/README.md):

| Role | Bytes | Retained Image |
| --- | ---: | --- |
| R1 projection/residual | 10864 | [r1.hsaco](gpu-attempt-v1/r1.hsaco) |
| Tiled MLP, BF16-materialized SiLU | 33320 | [mlp.hsaco](gpu-attempt-v1/mlp.hsaco) |
| Validator and guarded R2 | 28440 | [guarded.hsaco](gpu-attempt-v1/guarded.hsaco) |

Full hashes and device identities are in the [request](gpu-attempt-v1/request.json).
Each R1/MLP/R2 launch has 64 workgroups with one Wave64 per workgroup;
each validator has one Wave64 workgroup.

## Numerical Evidence

The fixture retains width 4096, rank-local intermediate width 6144, and dense
physical matrices with sparse nonzero rows. Generation 1 is the prior fixture.
Generation 2 doubles only Up weights. R1, RMSNorm and Gate remain unchanged;
Up, activation and Down values double. The BF16-materialized SiLU boundary is
preserved. Bounded dyadic values make all relevant sums exactly representable.

Every final BF16 word changes between generations. Exact cancellation in
generation 2 produces 820 positive zeros on rank 0 and 819 on rank 1. These zeros
must replace NaN poison and match positive-zero bits, not merely a tolerance.

The [independent verifier](gpu-attempt-v1/verify_native.py) reconstructs values
with integer dyadics and rebuilds dense matrix hashes row by row. It does not
import the Rust fixture, model code or native test. The native observer also
compares every weight byte read back from VRAM in bounded 4 MiB chunks. Raw
weight payloads are not exported; their emitted digests are independently checked
against the reconstructed full dense representations.

| Check | Actual Result |
| --- | --- |
| Computed intermediate/final values | 139,264, all bit-exact |
| Final BF16 values | 16,384, all bit-exact |
| Weight checks | 12 full matrix digests, eight distinct representations |
| Terminal owner observations | Four, across two consecutive generations |
| Final words changed from generation 1 | All 8,192 |
| Generation-2 positive-zero words | Rank 0: 820; rank 1: 819 |
| Verifier self-tests | Four final hashes, two Up-matrix hashes, 99 observation refusals, two JSON and two stdout refusals |

Generation-2 final hashes:

- Rank 0: `403903da99ade382e932312fb9fcdaa5d5fe3e41bf05768f614667fd77b160bf`.
- Rank 1: `e7164a702f121bbaa1b9e940960177f2b7670bf279f641e6e827be6bdfba3e93`.

The [actual native stdout](gpu-attempt-v1/evidence/native.stdout) is SHA-256 joined
to the [independent result](gpu-attempt-v1/evidence/verify.stdout). Terminal
prefixes/guards and all emitted payload bytes are retained in the
[observation](gpu-attempt-v1/evidence/observation.json).

## Lifecycle And Timing

The [GPU receipt](gpu-attempt-v1/evidence/complete.json) records one requested,
spawned and completed native attempt, no retry, and eight natural-zero phases.
All children were reaped and process groups absent, with no timeout, forced
cleanup or postcheck error. All eight GPUs were idle in before/after telemetry;
the test reports healthy queue-first Close.

Actual queue observations, identical on both ranks:

| Generation | Write Cursor | Observed Read Cursor | Coordinator Host Interval |
| --- | ---: | ---: | ---: |
| 1 | 5 | 3 | 170.552316 ms |
| 2 | 10 | 7 | 170.498655 ms |

All completion signals were acquired as zero despite lagging hardware read
cursors. The implementation does not manufacture queue capacity. These
intervals include preparation, validation and polling; **they are not GPU
kernel latencies or model throughput.** The native process takes 26.920868
seconds including upload/readback; the controller takes 28.564208 seconds.
Neither establishes an overlap or 700 tokens/s claim.

## CPU Qualification

The [CPU receipt](cpu-attempt-v1/evidence/complete.json) records 13 natural-zero
phases in 63.009175 seconds on `ssh mi350`, with unchanged sources, dependencies
and tool identities and no postcheck error.

| Scope | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Full KFD suite, five targets | 1041 | 0 | 6 |
| Combined owner and fixture CPU tests | 16 | 0 | 3 |
| Existing paired coordinator | 14 | 0 | 0 |
| Paired native fixtures | 5 | 0 | 2 |
| Session custody | 10 | 0 | 0 |
| New reuse fixture | 2 | 0 | 1 |
| Atomic memory | 7 | 0 | 0 |

All 1044 prior outcomes are preserved. Two passing fixture tests and the ignored
native reuse test are added. The library inventory is 1027 tests; the GPU run
selects exactly one and filters 1026. The 799-entry source map contains 797
source files plus two harness files. Only the existing native-test parent and
one new child change; the qualified Session/runtime is unchanged. The parent
retains the original one-generation exercise and output schema.

The [CPU capsule](cpu-attempt-v1/retention-manifest.json) has 94 members and 93
pinned bodies, including 69 raw files. Its 901,796-byte archive SHA-256 is
`79a4bce8d23ace210af6b7b6091de77a14e43f23e19109192335fff7079093a4`.
The [GPU capsule](gpu-attempt-v1/retention-manifest.json) has 59 members and 58
pinned bodies, including 45 raw files and all three checked images. Its
435,711-byte archive SHA-256 is
`b8b49ea78302229233caf41d6914659ad6bd8ed4d40683efb68171d37958ba47`.
The 1,434,513-byte GPU receipt SHA-256 is
`f3aacc7f93374ec5549dcc3af59598cfe6ccc3e66522095e65be4f1ae7b5c46b`.
All 890 final authenticated input pins join the attempt. Host executable bodies
are not exported.

## Remaining Work

This does not test dense checkpoint weights, full-model numerical error, GPU
fault injection, allocation canaries, arena recycling or sustained capacity.
The private Session needs a multi-layer integration API preserving per-layer
custody while scheduling prefix/tail work. Ferric must use a distinct guarded
completion path without its legacy duplicate R2 callback or fake 2192-byte
state tokens. Then qualify model state banks and the complete BF16 Qwen
workload before equivalent vLLM comparisons, overlap plots and ablations.

The next [CPU-qualified retained-pair checkpoint](../guarded-mlp-retained-pair-v1/README.md)
adds borrow-free owner custody and sealed completed-batch revalidation after
intervening queue work. Its new executable still needs native interleaving
validation; this page's GPU receipt applies only to the Session run above.
