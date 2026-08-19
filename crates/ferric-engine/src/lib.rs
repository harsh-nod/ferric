#![forbid(unsafe_code)]

//! Safe state machines used by the generated Ferric runtime.

mod kv;

pub use kv::{KvError, KvPool, PageId};
