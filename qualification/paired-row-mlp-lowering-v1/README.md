# Paired-Row MLP: Checked gfx950 Lowering

The paired-row BF16 down-projection candidate passed fourteen CPU tests and
the complete nine-stage checked Rust-to-HSACO pipeline on `mi350-2`. It has
not been selected in the live worker or executed on GPU. This checkpoint
qualifies compilation and host-side equivalence tests, not a speedup or
independent full-model numerical acceptance.

## Optimization

The original down-projection processes 64 output rows per tile. For each row,
each lane reads 96 activations and weights, accumulates a partial sum, and
participates in the ordered Wave64 reduction.

The candidate processes 32 row pairs. Each pair shares an acquired activation
read between two independent FP32 accumulators. Each row retains its original
96-step multiply/add order, reduction and output position. If the first row
fails, the cached second partial is invalidated without skipping its collective.
The other MLP stages, provider, 512 scheduler rounds, launch geometry, storage
layout and arithmetic policy remain unchanged.

| Per-Lane Work For One Down Tile | Original | Candidate |
| --- | ---: | ---: |
| Activation-read calls | 6,144 | 3,072 |
| Weight-read calls | 6,144 | 6,144 |
| Row reductions | 64 | 64 |
| Accumulation steps per row | 96 | 96 |

These are source-level work counts, not measured memory transactions or timing
contributions. Cache behavior, compiler transformations and scheduling still
determine the actual performance effect. This remains a scalar FP32
accumulation kernel, not an MFMA implementation.

## Actual Qualification

Four existing numerical tests and ten new paired-row tests passed, with no
ignored tests. Coverage includes all lanes and tiles, ordered reductions and
writes, cancellation, signed zero, subnormals, missing inputs, nonfinite
products, overflow and sticky rejection of the speculative second row.

The later compiler run passed fixture metadata, checked lowering, actual formal
replay, the inert artifact join, HSACO emission, retained-artifact extraction,
descriptor metadata, ELF notes and disassembly. All nine commands exited
naturally. The owner reaped its processes, performed no forced cleanup, and
passed its source, tool, dependency and prior-cache postchecks.

| Emitted Resource | Result |
| --- | ---: |
| HSACO bytes | 33,112 |
| VGPRs / SGPRs | 106 / 106 |
| VGPR / SGPR spills | 0 / 0 |
| Private segment bytes | 0 |
| Shared-memory bytes | 512 |
| Required workgroup | 64 x 1 x 1 |
| Wave size | 64 |
| Explicit arguments / executable kernarg bytes | 88 / 344 |

The emitted image SHA-256 is
`65a76f917c12476f4814174ee39d515642a643a0352c6609ee514955a6a97449`.
The original selected MLP used 104 VGPRs and 106 SGPRs, also without spills.
The candidate therefore adds two reported VGPRs; no occupancy or speedup
claim follows from these resource counts.

The lowering completion SHA-256 is
`0ba363b9b4106e5293e4e6152d719c795c2d58f05e062b6a67570f194e0eb68e`.
Its outer completion SHA-256 is
`eeb7d115928426b9a03ca584210a0d79f8da2bc0f525768aefdba93d890df266`.
The separate lowering-controller suite passed seventeen synthetic tests.

## Scope And Next Gates

The published source is an isolated candidate, not a change to live kernel
selection. Binary artifacts and the full immutable-input snapshots are retained
outside Git. Local evidence publication does not rerun the compiler or rehash
every transitive dependency. The compiler receipt still lists eight unresolved
runtime requirements; it grants no launch or production authority.

Retained evidence: [publication ledger](result.json),
[lowering completion](lowering/complete.json), [owner completion](owner/complete.json),
[CPU tests](cpu/complete.json), [controller tests](pure/complete.json),
[LLVM](lowering/extracted/module.ll),
[ELF metadata](row-down2-checked-probe-v228-v1/elf-notes-stdout) and
[disassembly](row-down2-checked-probe-v228-v1/disassembly-stdout).
The [paired-row macro](source/device/qwen3-tp-wave-rmsnorm-kernels-v15/src/mlp_tile_numerics_v2.rs)
and [regression tests](source/device/qwen3-tp-wave-rmsnorm-kernels-v15/tests/mlp_down_two_row_v1.rs)
are the exact formatted candidate sources.

Next, qualify the exact image through the GPU admission and lifecycle checks,
compare complete state and output buffers against the unchanged workload, and
then run a timing ablation. The [native clock baseline](../native-device-clock-observation-v1/README.md)
is available, but its raw ticks are not calibrated nanoseconds. Independent
model-reference acceptance and the sustained single-request Qwen3-8B BF16
target-only 2,048/256, 700 tokens/s target remain open.
