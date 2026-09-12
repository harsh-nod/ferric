# Sharded FP32 argmax v13: source-only prototype

This standalone crate proposes a separate two-stage fixed-vocabulary argmax. It
does not replace v11/v12, join a driver, change a selector/default, or establish
native correctness or a performance gain. No build, formatting, host test, typed
emission, or GPU result is claimed by this source checkpoint.

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
  ownership/bounds/uniformity proof is accepted still requires typed emission;
  the source shape is not a proof result.
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
rows. Hidden arguments, alignment, total kernarg size, descriptors, spill counts,
and emitted ABI admission remain unmeasured and unqualified.

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
There are 13 numerical fixture methods and 6 source/contract methods. Ownership
and key encodings are exhaustive; numerical fixtures cover all shard/lane scan
boundaries, cross-shard ties, signed zero, finite extremes/subnormals, FP32 values
that collapse in BF16, deterministic finite bit patterns, every lane's persistent
nonfinite flag, every shard's sentinel/malformed scratch, and active-capacity
tails with input/guard bit preservation. They are not general numerical proofs.

Only after the host gate is admitted should a separately reviewed exact-compiler
emission bind the two-root ABI, descriptor/resource/progress results, and source
identity. A later native fixture would compare the two-stage output bitwise to
the unchanged v11 and an independent scalar oracle, with guarded private scratch
and explicit stage completion. Driver integration and matched timing would be
separate decisions; the extra dispatch/scratch traffic may outweigh any benefit.
