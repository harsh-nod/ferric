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

Ferric has completed its M0 foundation milestone. The repository currently
implements:

- executable sequential semantics for greedy speculative verification;
- a fixed-capacity generational request scheduler with deterministic batching;
- exact completion authority and retirement-before-reuse transitions;
- a fixed-capacity generational paged-KV ownership model;
- atomic KV commit, rollback, sealed-prefix sharing, and copy-on-write transitions;
- strict pinned Qwen3 configuration and tokenizer-metadata admission;
- bounded streaming authentication and semantic admission of the exact shared tokenizer;
- bounded deterministic ASCII-domain encode and exact byte decode through that tokenizer authority;
- bounded streaming authentication of the exact pinned Qwen3 safetensors files;
- a fixed-width canonical record for the exact admitted deployment identities;
- all 22 exact sequential target/draft B3 plans and their finite K1-K7 structural profiles;
- byte-reproducible compiled Qwen3/gfx942 runner declarations for all 10,648 typed operations;
- direct pinned-Verus proofs of the executable M0 state machines;
- identity-bound fe2o3 M0 property records with actual-body mutation evidence;
- structural invariant validation and hostile stale-handle tests; and
- the roadmap, assurance policy, feature ledger, and performance protocol.

Ferric does **not** currently support non-ASCII tokenization or prove full
behavioral equivalence to an external regex or Unicode implementation,
transform or pack weights,
sign deployment records, allocate device memory, load a model onto a device,
compile a GPU kernel, instantiate a command template, dispatch HSA, prove graph
refinement, or make a verified-inference or performance claim. The canonical
records bind pinned values but do not authenticate external compiler, runtime,
proof, executable, or machine identities. The model-admission, planning,
catalog, identity, and declaration bodies remain explicitly pending Verus.
Unsupported stages fail closed rather than selecting another implementation.

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

Ferric pins Rust 1.97.1 to match the admitted Verus release. Correctness-
critical executable bodies are compiled by ordinary Cargo and directly
verified from the same source by `cargo-verus`; proof constructs are erased
from release artifacts.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo test --workspace --locked
```
