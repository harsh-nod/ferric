//! Explicit whole-program validation scope, distinct from the serial interpreter.

use super::prepared_forward_wire::ExecutionReceipt;
use fe2o3_kfd::engineering_wire::{self as base, ResponseV1};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 1;
pub const MODE: &str = "device-peer-tp2-prepared-native-program-v1";
pub const LAUNCH_FLAG: &str = "--tp2-prepared-native-program";
pub const CURRENTNESS: &str = "full-program-boundaries-operational-inner";
pub const NATIVE_SOURCE: &str = "6b61f0093bfe79521fe277078f215ea6281cf585c1e16250ac37f77950315a99";
pub const NATIVE_PROGRAM_DEADLINE_MS: u32 = 30_000;

/// Scalar fields from the actual native receipt; all 72 receipts are in `ExecutionReceipt`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeProgramCompletion {
    pub program_id: u64,
    pub group_id: u64,
    pub epoch: u64,
    pub unique_ids: [u64; 2],
    pub program_api_calls: u32,
    pub logical_steps: u32,
    pub rank_dispatches: u32,
    pub kernel_counts: [u32; 2],
    pub barrier_counts: [u32; 2],
    pub packet_counts: [u32; 2],
    pub first_frontiers: [u64; 2],
    pub final_frontiers: [[u64; 2]; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "prepared_program_op",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Response {
    Ready {
        protocol: u32,
        mode: String,
        profile: String,
        unique_ids: [u64; 2],
        process_id: u32,
        authority: String,
        currentness: String,
        control_allocation_flags: u32,
        native_source_sha256: String,
        native_program_deadline_ms: u32,
    },
    Setup {
        id: u64,
        response: ResponseV1,
    },
    Registered {
        id: u64,
        plan_sha256: [u8; 32],
        catalog_sha256: [u8; 32],
        steps: u32,
        kernel_counts: [u32; 2],
    },
    Executed {
        receipt: ExecutionReceipt,
        native_program: NativeProgramCompletion,
    },
    Closed {
        id: u64,
    },
    Fatal {
        id: u64,
        message: String,
    },
}

pub fn write_response(output: &mut impl Write, response: &Response) -> io::Result<()> {
    base::write_header_v1(output, response)?;
    output.flush()
}

pub fn read_response(input: &mut impl Read) -> io::Result<Option<Response>> {
    base::read_header_v1(input)
}
