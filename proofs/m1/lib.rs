#![forbid(unsafe_code)]

//! M1-specific cross-module composition theorems.

pub mod batching;
pub mod graph;
pub mod isolation;
pub mod kv_physical;
pub mod scheduler;
pub mod speculative_graph;
