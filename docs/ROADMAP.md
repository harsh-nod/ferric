# Ferric Roadmap

Status: living program contract.

Proof-required work follows [the proof-development protocol](PROOF_DEVELOPMENT.md).
M0 qualification is governed by the
[property contract](M0_PROPERTY_CONTRACT.md).

Ferric's north star is proof-carrying DeepSeek-V4 speculative inference on an
eight-device `gfx942` system. That is a graduation test, not the first
milestone. The roadmap closes one refinement boundary at a time while keeping
every implementation on the final production path.

## Program Rules

1. There is one implementation path. Unsupported behavior fails closed.
2. Model- and inference-specific kernels are owned by Ferric. Reusable GPU
   compiler and runtime capabilities are implemented in fe2o3 and consumed
   through public APIs.
3. A proof applies only through the last boundary it covers. Hashes, machine
   inspection, and hardware tests do not imply semantic refinement.
4. Correctness and performance evidence remain independent and identity-bound.
5. A configuration is served only when all of its required property statuses
   and performance gates are satisfied.
6. Feature parity is measured against pinned vLLM and SGLang releases, not the
   changing head of either project.
7. Tiny generated configurations instantiate production code. They are proof
   and test fixtures, not alternative inference engines.

## M0: Permanent Foundations

Goal: freeze the system theorem, core identities, and reversible KV semantics.

- [x] Create the Rust workspace and assurance policy.
- [x] Implement executable greedy speculative-round semantics.
- [x] Implement exclusive generational paged-KV ownership.
- [x] Implement tentative append, commit, rollback, and release.
- [x] Reject stale page handles and non-transactional exhaustion.
- [x] Add generational request slots and deterministic scheduler transitions.
- [x] Add completion epochs and retirement-before-reuse semantics.
- [x] Add sealed immutable prefix pages and copy-on-write.
- [x] Directly verify the executable M0 state-machine bodies with pinned Verus.
- [x] Bind M0 properties into fe2o3 proof contracts.

Exit gate: every implemented transition has an executable oracle, invariant
tests, and a closed direct Verus result under the published TCB. Every `Proved`
M0 property binds compiler-rooted paths and its assigned sensitive actual-body
mutations.

## M1: Qwen3 Speculative Inference On One gfx942

Goal: Qwen3-8B target inference with a pinned Qwen3-0.6B draft model on one
`gfx942`, using Ferric-owned model kernels over the reusable fe2o3
compiler/runtime path.

Declared first envelope:

```text
precision:          BF16 with declared FP32 accumulation
context:            <= 8K tokens
active sequences:   <= 32
KV:                 paged, exclusive initially
scheduler:           continuous batching
decoding:           greedy speculation
runtime:            direct HSA command batches
fallbacks:          none
```

### Model And Build

- [ ] Define bounded canonical Ferric deployment bundles.
- [ ] Strictly admit Qwen3 config, tokenizer, vocabulary, and safetensors.
- [ ] Check target/draft tokenizer compatibility rather than assuming it.
- [ ] Stream and authenticate prepacked weight sections.
- [ ] Generate a model/target-specific Rust runner and complete plan identity.

### Ferric Kernel Families

- [ ] Parameterized BF16/FP32 GEMM and GEMV.
- [ ] RMSNorm and residual fusion.
- [ ] RoPE plus paged-KV write.
- [ ] FlashAttention prefill.
- [ ] GQA-aware paged decode attention.
- [ ] SwiGLU and projection epilogues.
- [ ] Logit projection, argmax, and compact completion records.
- [ ] Finite proof-bound schedule catalogs for every family.

### Runtime

- [ ] Reviewed long-lived HSA queue authority in fe2o3.
- [ ] Fixed-capacity multi-packet AQL command batches.
- [ ] Typed device and host-pinned allocation leases.
- [ ] One queue publication per generated inference step.
- [ ] Paged KV, continuous batching, cancellation, and quiescent retirement.
- [ ] On-device draft, target verification, accept, commit, and rollback round.

### Proof And Validation

- [ ] Model bundle well-formedness.
- [ ] Operator and generated graph refinement.
- [ ] Paged KV refinement to the contiguous logical cache.
- [ ] Continuous batching refinement to independent requests.
- [ ] Greedy speculation refinement to target-only decoding.
- [ ] Request isolation excluding published timing and side-channel non-claims.
- [ ] Kernel bounds, initialization, race, and convergence properties.
- [ ] ABI, target, artifact, proof, and generated-plan identity closure.
- [ ] Explicit compiler/runtime/hardware TCB report.

### Qualification

- [ ] Differential target-only logits and tokens over declared buckets.
- [ ] Canary, cancellation, exhaustion, rollback, and fault-injection suites.
- [ ] Ferric core kernels meet the D10 baseline gates through the pinned fe2o3 toolchain.
- [ ] Speculation beats Ferric target-only execution on an eligible holdout.
- [ ] Ferric is compared against pinned, tuned vLLM and SGLang baselines.

Exit gate: all required M1 properties are closed at their declared statuses,
hardware tests pass on `mi300x`, and the exact artifact passes the performance
protocol. No machine-refinement claim is made unless the separate validators
in the assurance policy exist and pass.

## M2: Sampling And Single-Device Serving Breadth

- [ ] Counter-based, domain-separated request RNG.
- [ ] Exact finite-distribution temperature, top-k, and top-p semantics.
- [ ] Stochastic speculative distribution-preservation proof.
- [ ] Prefix caching and copy-on-write refinement.
- [ ] Chunked prefill refinement.
- [ ] Quantized KV cache.
- [ ] Structured-output grammar state and constrained sampling.
- [ ] Beam and parallel sampling.
- [ ] Prompt adapters and proof-bound LoRA composition.
- [ ] OpenAI-compatible serving boundary outside the proof core.

## M3: Multi-Device Refinement

- [ ] Atomic multi-device execution epochs.
- [ ] Collective sequence, buffer ownership, and completion contracts.
- [ ] Two-device then eight-device tensor parallelism.
- [ ] Data and pipeline parallel execution.
- [ ] Distributed KV and prefix ownership.
- [ ] Rank-consistent sampling and speculative commit/rollback.
- [ ] Disaggregated prefill/decode state-transfer refinement.

RCCL or another collective implementation remains a contracted TCB component
until replaced or covered by a stronger validator.

## M4: MoE And DeepSeek-V4

- [ ] General top-k routing, capacity, stable permutation, and inverse mapping.
- [ ] Grouped expert GEMM and weighted combine.
- [ ] Expert placement, all-to-all, and expert-parallel refinement.
- [ ] FP8 and FP4 numerical contracts and qualified Ferric-owned kernels.
- [ ] DeepSeek-V4 CSA/HCA attention semantics and kernels.
- [ ] Manifold-constrained hyper-connection semantics and kernels.
- [ ] DeepSeek-V4-Flash target-only inference on eight `gfx942` devices.
- [ ] Distributed execution refinement for the exact admitted configuration.
- [ ] Apply the generic speculation protocol to a compatible pinned draft.
- [ ] Qualify DeepSeek-V4-Pro and longer-context configurations.

Graduation gate: an exact eight-device DeepSeek-V4 speculative configuration
passes the assurance and performance policies and outperforms the faster pinned
vLLM/SGLang baseline under the same correctness policy and p99 SLO.

## Later Feature Parity

The [feature ledger](FEATURES.md) owns parity for multimodal models, adapter
batching, advanced speculation, quantization, offload, expert parallelism,
disaggregation, structured generation, and operational serving features. Each
feature needs a semantic contract, implementation, proof/validation matrix,
negative tests, and performance qualification.
