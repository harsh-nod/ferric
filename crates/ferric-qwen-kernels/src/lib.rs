#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

/// Exact Qwen3 RMSNorm and explicitly residual-fused compiler profiles.
pub mod rmsnorm;

/// Exact Qwen3 split-half RoPE and global-pool P16 paged-KV compiler profiles.
pub mod rope_kv;
