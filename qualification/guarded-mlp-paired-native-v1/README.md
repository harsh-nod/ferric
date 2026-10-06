# Paired Guarded MLP GPU Execution

Status: **the actual two-rank paired coordinator passes a nonzero synthetic
numerical diagnostic on MI350.** This is one generation of the private
R1 -> tiled MLP -> validator -> paired barrier -> R2 path, not a full-model
decode result, production capability or performance benchmark.

All issue #42 milestones and the single-request Qwen3-8B BF16
2048-prompt/256-generated-token, 700 tokens/s target remain open.

## Actual Execution

The [native test source](cpu-attempt-v1/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_native_v1_tests.rs)
calls the [qualified private coordinator](../guarded-mlp-paired-v1/README.md)
once. It does not emulate paired execution with serial host dispatch calls.
Each rank receives the same five-packet ordering:

```text
R1 -> tiled MLP -> validator -> barrier(validator0, validator1) -> R2
```

There are eight kernel dispatches, two barriers and ten completion signals in
the pinned construction. These counts are source-contract annotations joined
to the completed native call, not a separate per-packet hardware trace.
Both queues are published before host completion polling; this is not yet
a measured overlap timeline.

The test starts with fresh combined atomic owners at generation 1. The
coordinator performs submission and completion; no test-only state seeding,
separate host validator launch or unsafe completion shortcut is used.
All intermediates and final outputs start with NaN poison. Both completed
owners must have current Valid guards and all 548 terminal prefix words.

The three exact checked gfx950 images are retained:

| Image | Bytes | SHA-256 |
| --- | ---: | --- |
| [R1](gpu-attempt-v1/r1.hsaco) | 10864 | `25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25` |
| [Tiled MLP](gpu-attempt-v1/mlp.hsaco) | 33320 | `b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589` |
| [Validator and R2](gpu-attempt-v1/guarded.hsaco) | 28440 | `de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66` |

The selected MLP image explicitly materializes BF16 SiLU before multiplying
Up. Other images sharing its symbol are not interchangeable. Each R1, MLP
and R2 dispatch uses 64 workgroups with one Wave64 per workgroup; each
validator uses one Wave64 workgroup.

## Nonzero Numerical Fixture

The fixture has width 4096 and rank-local intermediate width 6144. It uses
full-size dense physical weight allocations with sparse, nonzero rows:

- R1 combines nonzero FP32 partials with BF16 residuals to produce a
  rank-dependent pattern of exactly +1 or -1.
- Norm weights are one. RMSNorm with epsilon 1e-6 narrows back to BF16 +1/-1.
- Every Gate row has one nonzero weight producing 8 or 16. Every Up row has
  one nonzero weight in a distinct column, producing +/-1/8 or +/-1/4.
- BF16-materialized SiLU rounds to 8 or 16. Activations are exactly +/-1,
  +/-2 or +/-4.
- Every Down row has three nonzero weights on distinct columns and lanes.
  Its FP32 result is an exactly representable multiple of 1/8. Both rank
  Down vectors and both final outputs are nonzero at every element.

The small dyadic values make all dot-product sums exact regardless of the
wave reduction order. This deliberately isolates publication, dependency and
buffer-role correctness from tolerance selection. It does not replace testing
dense checkpoint weights or full-model numerical error.

The [independent verifier](gpu-attempt-v1/verify_native.py) uses integer dyadics,
not the Rust fixture's floating-point helper. It reconstructs every emitted
input, intermediate and final output byte. Dense matrix hashes are rebuilt
one row at a time without loading the Rust implementation or a model library.
The native test also reads and compares every byte of all six weight matrices
after execution, in bounded 4 MiB chunks. Raw weight payloads are not exported;
their GPU readback is supplied by the pinned native observer, while the
verifier independently reconstructs and checks the six emitted digests.

The nonlinear reference has explicit rounding boundaries: normalized
magnitudes near 1 round to BF16 one; for Gate 8/16, SiLU lies strictly within
the Gate value's BF16 rounding cell before the Up multiplication. Actual
GPU intermediates must still match those values. This is not a general
accuracy guarantee for the GPU sqrt/exp implementations.

## Results

The [GPU receipt](gpu-attempt-v1/evidence/complete.json) records exactly one
requested and spawned native attempt, no retry, and eight naturally successful
phases. All children are reaped and their process groups are absent, with no
timeout, forced cleanup or postcheck error. Read-only telemetry reports all
eight GPUs idle before and after; the native test reports healthy queue-first
close.

| Check | Actual Result |
| --- | --- |
| Computed intermediates and final outputs | 69,632 elements, all bit-exact |
| Final BF16 outputs | 8,192 elements, all bit-exact |
| Dense weight representations | Six full matrix hashes reconstructed |
| Atomic owner states | Both terminal prefixes and guards validated |
| Independent reference self-tests | Two fixed output hashes; 41 observation, two strict-JSON and two stdout refusals |
| Reuse, rearm, allocation canaries | Not tested |

The [actual native output](gpu-attempt-v1/evidence/native.stdout) is joined by
its SHA-256 to the [independent result](gpu-attempt-v1/evidence/verify.stdout).
The output payload hashes are:

- Rank 0: `6e6ecb4b75e6e5f09a212bbd4d4779fc2975c026f4398c2241230843cf4a33bd`.
- Rank 1: `9fde0e89ff2a46d8b366a4adc5dbe4bb1728106112b2b8fa563e4af6d944d3bd`.

Actual final queue frontiers are `(write=5, read=3)` on both ranks despite
all ten completion signals being zero. The coordinator correctly retires
logical work without changing the observed read cursor or inventing capacity.
The lagging cursor is also why this result does not authorize resetting or
reusing the signal arenas.

The native process takes 13.510894 seconds, including allocation, upload and
full readback. The controller takes 14.743259 seconds. The coordinator's
host interval is 169.890275 ms and includes preparation, validation and polling.
**None of these is GPU kernel latency, sustained model throughput or evidence
of a speedup.** No 700 tokens/s or overlap claim follows from this run.

## Evidence

The [CPU receipt](cpu-attempt-v1/evidence/complete.json) records 11 clean phases
in 61.370393 seconds:

| Scope | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Full KFD suite, five targets | 1029 | 0 | 5 |
| Combined owner, including native fixture CPU tests | 14 | 0 | 2 |
| Existing paired coordinator | 14 | 0 | 0 |
| New native fixture CPU tests | 3 | 0 | 1 |
| Atomic memory | 7 | 0 | 0 |

All 1030 prior outcomes are preserved; three passing tests and one ignored
native test are added. The source/harness map has 796 entries. Only the
existing native-test module registration changes, and one child module is
added. No production runtime source changes in this checkpoint.

The CPU [capsule](cpu-attempt-v1/retention-manifest.json) has 84 members and 83
pinned bodies; its 895,207-byte archive has SHA-256
`0455e359c0a71a81fc73351b25f7cf8870cdd2f6a636a48f2e143adf06382280`.
The GPU [capsule](gpu-attempt-v1/retention-manifest.json) has 59 members and 58
pinned bodies, including 45 raw files and all three checked images. Its
413,580-byte archive has SHA-256
`ae4867914969ba05064ec29de79957310e40bb06aea7c803ce2b02dee333eb42`.
The actual GPU receipt is 944,021 bytes with SHA-256
`43b07e7fab6470214ca52fac565bb2fc765b4e5352b5d485a1936553ed7b8987`.
All 877 final authenticated input pins join the attempt. Host executable
bodies are not exported.

## Remaining Work

The follow-on [retained Session checkpoint](../guarded-mlp-paired-reuse-v1/README.md)
now compiles and passes CPU tests. Consecutive-generation GPU reuse remains a
separate pending gate.

Retain explicit paired quiescence authority across calls, test exact-next
generation rearm and owner/payload reuse, and keep fresh signal arenas until
their reuse protocol is separately qualified. Integrate Ferric's distinct
completion path without its old duplicate R2 callback. Then qualify state
banks and the complete BF16 Qwen workload before collecting equal-work vLLM
comparisons, overlap plots and performance ablations.
