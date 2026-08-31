# Ferric

Ferric is a proof-carrying, high-performance LLM inference engine written in
Rust and built on [fe2o3](https://github.com/harsh-nod/fe2o3).

Ferric has one production path:

```text
authenticated model bundle
  -> verified algorithm and execution plan
  -> Ferric-owned proof-bound model kernels
  -> reusable fe2o3 compiler and runtime
  -> generated Rust runner
  -> reviewed direct HSA command batches
```

There is no PyTorch runtime, vendor-kernel fallback, legacy fe2o3 compiler,
runtime JIT, raw launch mode, or unverified kernel plugin path.

## Status

Ferric has completed its M0 foundation milestone. The repository currently
implements:

- executable sequential semantics for greedy speculative verification;
- a fixed-capacity generational request scheduler with deterministic batching;
- exact completion authority and retirement-before-reuse transitions;
- a fixed-capacity generational paged-KV ownership model;
- atomic KV commit, rollback, sealed-prefix sharing, and copy-on-write transitions;
- strict pinned Qwen3 configuration and tokenizer-metadata admission;
- bounded streaming authentication and semantic admission of the exact shared tokenizer;
- bounded deterministic UTF-8 encode and exact byte decode through that tokenizer authority;
- bounded streaming authentication of the exact pinned Qwen3 safetensors files;
- a fixed-width canonical record for the exact admitted deployment identities;
- all 22 exact sequential target/draft B3 plans and their finite K1-K7 structural profiles;
- byte-reproducible compiled Qwen3/gfx942 runner declarations for all 10,648 typed operations;
- linear logical publication of those retained declarations into engine custody;
- direct pinned-Verus proofs of the executable M0 state machines;
- identity-bound fe2o3 M0 property records with actual-body mutation evidence;
- structural invariant validation and hostile stale-handle tests;
- one production-compiled Qwen3 SwiGLU kernel with an independently inspected
  `gfx942:xnack-` COV6 HSACO and durable Worker V3 load-envelope custody; and
- the roadmap, assurance policy, feature ledger, and performance protocol.

Ferric does **not** currently load a model, dispatch the production HSACO,
execute Qwen, or make a verified-inference or performance claim. Only K6
SwiGLU has an attributed safe device package and source root: one of seven
required packages and one of 12 required roots. Its historical artifact does
not authorize the current write-only source. The production verifier,
generated runner, remaining kernels, model bundle, hardware qualification,
performance qualification, and end-to-end inference path remain open. All 33
M1 roadmap requirements remain open. Unsupported stages fail closed rather
than selecting another implementation.

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
kernels:            Ferric-owned, compiled by fe2o3
compiler/runtime:   reusable fe2o3 APIs
```

See [the roadmap](docs/ROADMAP.md), [assurance policy](docs/ASSURANCE.md),
[proof-development protocol](docs/PROOF_DEVELOPMENT.md),
[architecture](docs/ARCHITECTURE.md), [feature ledger](docs/FEATURES.md), and
[performance protocol](docs/PERFORMANCE.md).

## Development

Ferric pins Rust 1.97.1 to match the admitted Verus release. Correctness-
critical executable bodies are compiled by ordinary Cargo and directly
verified from the same source by `cargo-verus`; proof constructs are erased
from release artifacts.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo test --workspace --locked
```
