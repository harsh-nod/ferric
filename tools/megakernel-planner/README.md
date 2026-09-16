# Megakernel Planner Engineering Tool

This independent, dependency-free Rust package checks addressless Qwen3
operation-graph declarations for [Ferric issue #42](https://github.com/harsh-nod/ferric/issues/42).
It is host engineering tooling, not a production inference backend.

The package is outside Ferric's verified production Cargo workspace and the
source snapshot measured by `proofs/source-closure.py`. Its unit tests and
compile-fail test are engineering checks, not Verus proof or coverage under
Ferric's authenticated release receipt. Do not add this package as a production
dependency until its required proof and admission boundaries have been closed.

## Scope

- Qwen3-0.6B and Qwen3-8B complete operation DAGs, including model-specific head
  geometry, normalization/position constants, and an explicit numerical policy.
- Batches 1, 2, 4, and 8; context capacities 1K, 4K, and 8K.
- Bounded tensor/task records, predecessor and lifetime declarations, dedicated
  workspace ranges, and hostile-input validation.
- Allocation-free borrowed per-step declarations with private checked fields.

The package does not authenticate weights or tokenizer bytes, implement a tiled
device scheduler, compile or launch kernels, provide KV leases, prove numerical
equivalence, or qualify gfx950 execution. Nonzero identity bytes remain caller
declarations. The graph-order oracle in the tests is not a numerical interpreter.

## Checks

Run from the repository root with the admitted Rust 1.97.1 toolchain. On shared
hosts, keep build output outside the checkout and cap compilation parallelism:

```sh
export CARGO_TARGET_DIR=/path/to/owned-scratch/megakernel-planner-target
export CARGO_BUILD_JOBS=2
cargo fmt --manifest-path tools/megakernel-planner/Cargo.toml --check
cargo test --manifest-path tools/megakernel-planner/Cargo.toml --locked
cargo clippy --manifest-path tools/megakernel-planner/Cargo.toml --locked --all-targets -- -D warnings
```

These explicit commands do not run under root `cargo test --workspace`.
The separate engineering CI job must invoke them directly. Preserve source,
logs and evidence before deleting only build scratch created by your run.

See [the gfx950 design and status](../../docs/GFX950_MEGAKERNEL.md) for the
authenticated integration milestones and the remaining production blockers.
