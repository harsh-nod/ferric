#![forbid(unsafe_code)]

//! Executable sequential semantics used as Ferric's refinement target.
//!
//! This crate is not a serving fallback. Production runners must refine these
//! semantics and do not call them in the inference hot path.

#[allow(unused_imports)]
use vstd::prelude::*;

pub mod completion;
mod configuration;
mod identity;
pub mod scheduling;
mod speculation;

pub use configuration::{EngineLimits, ModelConfig, SpecError, Target};
pub use identity::{Identity, RequestId};
pub use speculation::{verify_greedy_round, GreedyCommit, TokenId};
