//! Test-only handoff from the real recording driver to actual ABI packing.
use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "FerricTokenProgramRecordingAbiFixtureV1";
pub const MAX_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema: String,
    pub buffers: Vec<(u64, usize)>,
    pub graphs: Vec<Graph>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    pub position: u32,
    pub commands: Vec<Dispatch>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dispatch {
    pub kernel: String,
    pub grid_workgroups: u32,
    pub workgroup_size: u16,
    pub arguments: Vec<Argument>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Argument {
    Buffer {
        id: u64,
        offset: usize,
        elements: usize,
        element_bytes: u32,
        access: Access,
    },
    U32 {
        value: u32,
    },
    F32 {
        bits: u32,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    Read,
    Write,
    ReadWrite,
}
