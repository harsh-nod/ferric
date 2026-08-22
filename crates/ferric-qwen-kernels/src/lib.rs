#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

/// Exact finite Qwen3 dense GEMM/GEMV compiler profiles.
pub mod gemm;
/// Exact Qwen3 lowest-ID argmax and compact-completion compiler profiles.
pub mod logits;
/// Exact Qwen3 paged-GQA decode and speculative-attention compiler profiles.
pub mod paged_decode;
/// Exact Qwen3 causal-prefill compiler profiles.
pub mod prefill;
/// Exact Qwen3 RMSNorm and explicitly residual-fused compiler profiles.
pub mod rmsnorm;
/// Exact Qwen3 split-half RoPE and global-pool P16 paged-KV compiler profiles.
pub mod rope_kv;
/// Exact Qwen3 SwiGLU compiler profiles.
pub mod swiglu;
