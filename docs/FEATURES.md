# Feature Ledger

This ledger records product breadth. Parity baselines will be pinned to exact
vLLM and SGLang releases when M1 begins qualification.

Statuses:

- `Model`: executable state model exists.
- `Planned`: final semantics and implementation are not yet present.
- `fe2o3`: blocked on a reusable fe2o3 compiler, kernel, or runtime capability.
- `Unsupported`: intentionally rejected by product policy.

| Area | Feature | Status | Required primary property |
| --- | --- | --- | --- |
| Admission | Canonical model/config/tokenizer/weight bundle | Planned | `model_bundle_well_formed` |
| Models | Qwen3 dense target and draft | Planned | `graph_refined` |
| Models | DeepSeek-V4 MoE and hybrid attention | Planned | `graph_refined` |
| Models | Arbitrary remote Python model code | Unsupported | N/A |
| KV | Exclusive generational pages | Model | `kv_refined` |
| KV | Tentative commit and rollback | Model | `rollback_refined` |
| KV | Committed-prefix sharing and copy-on-write | Model | `kv_refined` |
| KV | Quantized KV | Planned | `kv_refined` |
| Scheduling | Continuous batching | Model | `scheduler_refined` |
| Scheduling | Chunked prefill | Planned | `scheduler_refined` |
| Scheduling | Cancellation | Model | `lifetime_safe` |
| Scheduling | Preemption | Planned | `lifetime_safe` |
| Scheduling | Priority and fairness policies | Planned | `resource_bounded` |
| Decoding | Greedy speculative transition | Model | `rollback_refined` |
| Decoding | Exact stochastic speculation | Planned | `distribution_preserved` |
| Decoding | N-gram and model draft providers | Planned | `sampler_refined` |
| Decoding | Tree and multi-token prediction speculation | Planned | `distribution_preserved` |
| Sampling | Temperature, top-k, and top-p | Planned | `sampler_refined` |
| Sampling | Beam and parallel sampling | Planned | `sampler_refined` |
| Output | Structured/grammar-constrained generation | Planned | `sampler_refined` |
| Adapters | LoRA and multi-LoRA batching | Planned | `graph_refined` |
| Kernels | GEMM/GEMV/RMSNorm/RoPE/SwiGLU | fe2o3 | `operator_refined` |
| Kernels | Flash prefill and paged decode attention | fe2o3 | `operator_refined` |
| Kernels | FP8/FP4 and MoE grouped GEMM | fe2o3 | `operator_refined` |
| Runtime | Direct bounded HSA command batches | fe2o3 | `lifetime_safe` |
| Runtime | Runtime JIT or raw kernel plugins | Unsupported | N/A |
| Parallel | Data parallelism | Planned | `multi_device_refined` |
| Parallel | Tensor and pipeline parallelism | Planned | `multi_device_refined` |
| Parallel | Expert parallelism | Planned | `multi_device_refined` |
| Serving | Disaggregated prefill/decode | Planned | `multi_device_refined` |
| Serving | OpenAI-compatible HTTP API | Planned | Outside proof core |
| Serving | Metrics, tracing, and deterministic replay | Planned | `Checked` evidence |
| Multimodal | Image/audio/video encoders and processors | Planned | `graph_refined` |
| Memory | Host/device offload and swap | Planned | `lifetime_safe` |

Feature support is configuration-specific. A feature implemented for one
model, target, numerical policy, or schedule does not silently generalize to
another configuration.
