#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

/// Exact Qwen3 split-half RoPE and global-pool P16 paged-KV compiler profiles.
pub mod rope_kv;
