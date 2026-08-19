#![forbid(unsafe_code)]

//! Executable sequential semantics used as Ferric's refinement target.
//!
//! This crate is not a serving fallback. Production runners must refine these
//! semantics and do not call them in the inference hot path.

mod identity;
mod model;
mod speculation;

pub use identity::{Identity, RequestId};
pub use model::{EngineLimits, ModelConfig, SpecError, Target};
pub use speculation::{verify_greedy_round, GreedyCommit, TokenId};
