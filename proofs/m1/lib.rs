#![forbid(unsafe_code)]

//! M1-specific cross-crate and cross-module composition theorems.

pub mod authenticated_prefill_bootstrap;
pub mod batching;
pub mod graph;
pub mod isolation;
pub mod kernel_contracts;
pub mod kv_physical;
pub mod model_bundle;
pub mod physical_new_window;
pub mod r33_daemon_lifecycle;
pub mod scheduler;
pub mod speculative_graph;
