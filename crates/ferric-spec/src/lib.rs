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

pub use configuration::{
    DeploymentBundle, EngineLimits, ModelArtifact, ModelConfig, NumericalPolicy, Qwen3ModelRole,
    SpecError, Target, TokenizerConfig, WeightManifest, M1_MAX_ACTIVE_SEQUENCES,
    M1_MAX_CONTEXT_TOKENS, M1_MAX_DRAFT_TOKENS, M1_MAX_KV_PAGE_TOKENS, M1_MAX_WEIGHT_BYTES,
    M1_MAX_WEIGHT_SECTIONS, QWEN3_END_OF_TEXT_TOKEN, QWEN3_IM_END_TOKEN, QWEN3_IM_START_TOKEN,
    QWEN3_VOCABULARY_SIZE,
};
pub use identity::{Identity, RequestId};
pub use speculation::{verify_greedy_round, GreedyCommit, TokenId};
