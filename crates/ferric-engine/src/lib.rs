#![forbid(unsafe_code)]

//! Safe state machines used by the generated Ferric runtime.

#[allow(unused_imports)]
use vstd::prelude::*;

mod cache;
mod epoch;
mod runner;
mod scheduler;
mod system;

pub use cache::{KvError, PageId};
pub use epoch::ExactCompletion;
pub use runner::{LogicalRunnerDeclaration, LogicalRunnerError};
pub use scheduler::{DispatchBatch, SchedulerError};
pub use system::{CompletionFailure, Engine, EngineError};
