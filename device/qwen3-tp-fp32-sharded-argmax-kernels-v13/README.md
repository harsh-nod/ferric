# Sharded FP32 argmax v13: emission-checked prototype

This standalone crate proposes a separate two-stage fixed-vocabulary argmax. It
does not replace v11/v12, join a driver, change a selector/default, or establish
native correctness or a performance gain. Source `256a6e4` passes remote-only
formatting, 15 numerical tests, eight source/contract tests, strict Clippy and
zero doctests on mi300x. All ten host-gate commands pass, with fresh own artifacts
and unchanged source/dependency receipts. Complete archive:
`88fb14461cd48795c4fab23f2db9ce9bfbc3862aaaf47350236212d7e2e18c09`.
The separate eight-phase emission gate now passes using compiler `8efd4fd` and
the retained LLVM worker built at `216822`, whose worker subtree is unchanged
at that compiler revision. Actual ABI/resource inspection passes for both roots.
Native correctness and performance remain unqualified. The separate
[integer model](../../proofs/m1/sharded_argmax_v13.md) is not a refinement proof
of these kernel bodies.

Complete emission archive:
`0601efc2d683bf6b02100f3d36d887ba5bbce4cbc22d92753d9135fc6d4ab9b6`.
Image SHA256:
`72104603f91ac5037a7b106307fc7023e20860f905e912d43700a119381665db`.
The first attempt failed during source extraction before invoking the compiler;
its original evidence is retained separately. The corrected attempt changed
only the extraction/review harness, not kernel or compiler source.

| Root | SGPR | VGPR | Code Bytes |
| --- | ---: | ---: | ---: |
| Producer | 35 | 18 | 2252 |
| Finalizer | 30 | 13 | 2112 |

Both descriptors report zero register spills, private segment, LDS and AGPR,
with no dynamic stack. These are static values, not occupancy or timing results.
The compiler reports successful typed extraction and retains the handoff digest,
but this emission path does not persist the separate typed handoff payload.
Detailed typed-node review and native numerical/ordering checks remain separate.

## Closed source contract

- Exactly two new roots, in producer/finalizer order, listed by `ROOTS_V13`.
- FP32 logits, vocabulary 151936, 1 through 32 active rows, Wave64 only.
- Producer: `rows * 64` workgroups. Group `g` owns row `g / 64` and shard
  `g % 64`; lane `l` reads token `64 * (64 * step + shard) + l`.
- Each lane initializes from step 0, then iterates `step < 38` with the uniform
  `64 * step + shard < 2374` load guard. Shards 0 through 5 read 38 values per
  lane; shards 6 through 63 read 37. Every active logit has one reader.
- The literal induction header follows the exact pinned 8efd canonical progress
  shape in `crates/fe2o3-kernel-analysis/src/pliron_progress.rs`, function
  `canonical_positive_induction_loop`: the header compares its induction value
  directly against a cycle-external bound. Whether the complete new
  ownership/bounds/uniformity checks now pass compiler emission. This does not
  make the separate integer model a refinement proof of the emitted image.
- Two separate FP32 scratch arrays each have exactly 2048 elements, representing
  `[32, 64]`. The declared workspace is 16384 bytes total, excluding logits and
  choices. This is request-private global memory, not claimed LDS/private-segment
  resource usage from an emitted image.
- Producer lane 0 stores one maximum/key pair per group using the existing
  `RowStriped2D<Index1D, 64, 1>` owner with a flattened `[rows * 64, 1]` view.
- Finalizer: one workgroup per active row, each lane reads one shard pair. Its
  only output is one `u32` token ID per row. Inactive scratch/choice tails are not
  written; inactive scratch rows are not read.

Both roots have three slice carriers and a `u32 rows` argument: a calculated
52 explicit argument bytes on the existing 64-bit slice ABI. Producer arguments
are read-only FP32 logits, disjoint FP32 maxima, disjoint FP32 keys, and rows;
finalizer arguments are read-only FP32 maxima/keys, disjoint `u32` choices, and
rows. Actual inspection measures 52 explicit bytes, hidden arguments beginning
at byte 56, 312 total kernarg bytes and alignment 8 in both roots. Both require
Wave64 and a 64-by-1-by-1 workgroup. Native launch admission remains separate.

## Numerical and lifecycle policy

All comparisons and reductions remain FP32. A token key is the exactly
representable integer `151936 - token`, so maximum-key tie breaking chooses the
lowest token ID, including ties between signed zeros. No BF16 narrowing is used.

The producer retains a nonfinite flag even if later finite values win its lane.
All lanes participate in all three producer reductions. Any nonfinite value in
a shard gives that shard key zero, including a nonwinning shard. The finalizer
checks every shard for a finite maximum and a finite, integral key in 1 through
151936, then uniformly rejects any invalid lane before max/key selection or a
row output. The final token decode retains the v11 checked range and zero guards.

A future caller must bind separate nonaliasing scratch buffers to the same
request and row count, complete the producer before submitting the finalizer,
and discard the entire request's outputs on either stage's failure. Output is
not transactionally rolled back across rows. Scratch keys do not authenticate
producer completion or detect stale but valid pairs; this crate adds no runtime
admission/lifecycle protocol. The host numerical fixtures do not emulate device
capabilities, GPU traps, cross-workgroup scheduling, or memory ordering.

## Proposed focused gates

After source review, use a bounded remote CPU stage with the pinned fe2o3
`8efd4fd416d1ffae7a718144e4d299fe3c8f7590` dependency graph, package-fresh artifacts,
and unchanged-source receipts. Proposed commands from this directory:

```text
cargo fmt --all -- --check
cargo test --locked --offline --release --lib
cargo test --locked --offline --release --test contract
cargo test --locked --offline --release --doc
cargo clippy --locked --offline --release --all-targets -- -D warnings
cargo check --locked --offline --no-default-features
```

The last command is an expected feature-rejection negative, not a passing build.
There are 15 numerical fixture methods and 8 source/contract methods. Ownership
and key encodings are exhaustive; numerical fixtures cover all shard/lane scan
boundaries, cross-shard ties, signed zero, finite extremes/subnormals, FP32 values
that collapse in BF16, deterministic finite bit patterns, every lane's persistent
nonfinite flag, every shard's sentinel/malformed scratch, and active-capacity
tails with input/guard bit preservation. Reuse fixtures cover 32/1/17 active rows
with changed winners and minimum-carrier valid/invalid/valid input transitions.
Parsed-root checks bind the sentinel and complete write operands/witnesses;
synthetic source mutations demonstrate rejection of changed keys or stores.
These host/source fixtures pass at the checkpoint above; they are not general
numerical proofs. The proposed feature-rejection negative above was not part of
the accepted ten-step host gate and is not claimed executed.

Only after the host gate is admitted should a separately reviewed exact-compiler
emission bind the two-root ABI, descriptor/resource/progress results, and source
identity. A later native fixture would compare the two-stage output bitwise to
the unchanged v11 and an independent scalar oracle, with guarded private scratch
and explicit stage completion. Driver integration and matched timing would be
separate decisions; the extra dispatch/scratch traffic may outweigh any benefit.
