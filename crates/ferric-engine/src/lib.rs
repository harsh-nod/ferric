#![forbid(unsafe_code)]

//! Safe state machines used by the generated Ferric runtime.

mod cache;
pub mod epoch;
pub mod scheduler;

pub use cache::{KvError, KvPool, PageId};
