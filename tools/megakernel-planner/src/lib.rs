#![forbid(unsafe_code)]
//! Engineering-only addressless Qwen3 decode planning.
//!
//! This isolated host tool is outside Ferric's verified production workspace
//! and authenticated Verus release source closure. Tests are not proof.
//!
//! These records describe a model graph; they do not authenticate model bytes,
//! prove numerics, admit GPU resources, or authorize loading or launching code.
//! In particular, caller-supplied identity bytes are declarations, not fe2o3
//! task-schema or executable admission receipts. Production custody belongs to
//! Ferric's authenticated bundle/runner bridge and fe2o3 issue #135.
//!
//! The first plan is an operation DAG, not a tiled GPU scheduler. Every
//! intermediate has a distinct workspace range retained until dispatch
//! quiescence. This intentionally trades memory for an uncomplicated lifetime
//! argument before a dependency-aware reuse/tile plan is qualified.

mod planner;

pub use planner::*;
