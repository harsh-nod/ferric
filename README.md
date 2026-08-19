# Ferric

Ferric is a proof-carrying, high-performance LLM inference engine written in
Rust and built on [fe2o3](https://github.com/harsh-nod/fe2o3).

Ferric has one production path:

```text
authenticated model bundle
  -> verified algorithm and execution plan
  -> proof-bound fe2o3 kernels
  -> generated Rust runner
  -> reviewed direct HSA command batches
```

There is no PyTorch runtime, vendor-kernel fallback, legacy fe2o3 compiler,
runtime JIT, raw launch mode, or unverified kernel plugin path.

## Status

Ferric is at its foundation milestone. The repository currently implements:

- executable sequential semantics for greedy speculative verification;
- a fixed-capacity, generational paged-KV ownership model;
- atomic KV commit and rollback transitions;
- structural invariant validation and hostile stale-handle tests; and
- the roadmap, assurance policy, feature ledger, and performance protocol.

Ferric does **not** currently load a model, compile a GPU kernel, dispatch HSA,
or make a verified-inference or performance claim. Unsupported stages fail
closed rather than selecting another implementation.

## First Product Milestone

The first product milestone is Qwen3-8B target inference with Qwen3-0.6B as a
speculative draft model on one `gfx942` device:

```text
precision:          BF16 with declared FP32 accumulation
context:            <= 8K tokens
active sequences:   <= 32
KV cache:           paged
scheduling:         continuous batching
decoding:           greedy, then exact finite-distribution sampling
runtime:            direct HSA command batches
kernels:            fe2o3 only
```

See [the roadmap](docs/ROADMAP.md), [assurance policy](docs/ASSURANCE.md),
[proof-development protocol](docs/PROOF_DEVELOPMENT.md),
[architecture](docs/ARCHITECTURE.md), [feature ledger](docs/FEATURES.md), and
[performance protocol](docs/PERFORMANCE.md).

## Development

Ferric currently supports Rust 1.75 or newer.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
