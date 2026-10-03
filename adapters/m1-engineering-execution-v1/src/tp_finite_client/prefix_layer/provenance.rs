//! Closed source snapshot comparison. Only authenticated owner scopes may differ.
use super::{Result, hash, require};
use crate::finite_composition_wire::Registration;
use crate::finite_setup_wire_v1::UploadManifest;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    positions: u64,
    page_table: u64,
    cos: u64,
    sin: u64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Access {
    Read,
    Write,
    ReadWrite,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Argument {
    Buffer {
        source_id: u64,
        offset: usize,
        elements: usize,
        element_bytes: u32,
        access: Access,
    },
    U32 {
        value: u32,
    },
    F32Bits {
        value: u32,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dispatch {
    symbol: String,
    grid_workgroups: u32,
    workgroup_size: u32,
    arguments: Vec<Argument>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Model {
    Target8b,
    Draft06b,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Operation {
    AttentionOutputSum,
    FeedForwardDownSum,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Step {
    Rank {
        rank: u32,
        dispatch: Dispatch,
    },
    Collective {
        rows: usize,
        group_id: u64,
        epoch: u64,
        layer: usize,
        model: Model,
        operation: Operation,
        producers: Vec<Dispatch>,
        consumers: Vec<Dispatch>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Program {
    schema: String,
    profile: String,
    group_id: u64,
    token_buffer: u64,
    result_buffer: u64,
    metadata: [Metadata; 2],
    steps: Vec<Step>,
}
impl Program {
    fn checked(raw: &[u8], registration: &Registration) -> Result<Self> {
        require(raw.len() <= 4 << 20, "source snapshot byte bound")?;
        let program: Self = serde_json::from_slice(raw).map_err(|e| e.to_string())?;
        require(
            program.schema == "ferric-finite-source-grammar-v1"
                && program.profile == registration.profile
                && program.group_id == registration.group_id
                && program.steps.len() == 1013,
            "layer source schema/group/closed step census",
        )?;
        for step in &program.steps {
            match step {
                Step::Rank { rank, .. } => require(*rank < 2, "source rank")?,
                Step::Collective {
                    group_id,
                    model,
                    layer,
                    rows,
                    producers,
                    consumers,
                    ..
                } => require(
                    *group_id == registration.group_id
                        && *model == Model::Target8b
                        && *layer < 36
                        && *rows == 1
                        && producers.len() == 2
                        && consumers.len() == 2,
                    "source collective scope",
                )?,
            }
        }
        Ok(program)
    }
}

pub(super) struct Source {
    pub registration: Registration,
    pub raw_program: Vec<u8>,
    pub manifest: UploadManifest,
    program: Program,
}
impl Source {
    pub fn new(
        registration: Registration,
        raw_program: Vec<u8>,
        manifest: UploadManifest,
        pid: u32,
    ) -> Result<Self> {
        registration.validate().map_err(|e| e.to_string())?;
        require(
            registration.child_identity == pid
                && pid != 0
                && registration.source_program_bytes as usize == raw_program.len()
                && registration.source_program_sha256 == hash(&raw_program),
            "layer source own byte/scope joins",
        )?;
        let program = Program::checked(&raw_program, &registration)?;
        Ok(Self {
            registration,
            raw_program,
            manifest,
            program,
        })
    }
    pub fn same_input_source(&self, other: &Self) -> Result<()> {
        let a = &self.registration;
        let b = &other.registration;
        // Each raw record remains pinned to its own source/child. No normalized
        // hash is promoted to a source registration or executable identity.
        require(
            a.profile == b.profile
                && a.bundle_id == b.bundle_id
                && a.model_id == b.model_id
                && a.layers == b.layers
                && a.globals == b.globals
                && a.auxiliary == b.auxiliary
                && a.scratch == b.scratch
                && a.pending_buffers == b.pending_buffers
                && a.state_slots == b.state_slots
                && self.manifest == other.manifest,
            "layer logical source/upload mismatch",
        )?;
        let a = &self.program;
        let b = &other.program;
        require(
            a.schema == b.schema
                && a.profile == b.profile
                && a.token_buffer == b.token_buffer
                && a.result_buffer == b.result_buffer
                && a.metadata == b.metadata
                && a.steps.len() == b.steps.len(),
            "layer source metadata/order mismatch",
        )?;
        for (a, b) in a.steps.iter().zip(&b.steps) {
            match (a, b) {
                (
                    Step::Rank {
                        rank: ar,
                        dispatch: ad,
                    },
                    Step::Rank {
                        rank: br,
                        dispatch: bd,
                    },
                ) => require(ar == br && ad == bd, "layer rank command differs")?,
                (
                    Step::Collective {
                        rows: ar,
                        epoch: ae,
                        layer: al,
                        model: am,
                        operation: ao,
                        producers: ap,
                        consumers: ac,
                        ..
                    },
                    Step::Collective {
                        rows: br,
                        epoch: be,
                        layer: bl,
                        model: bm,
                        operation: bo,
                        producers: bp,
                        consumers: bc,
                        ..
                    },
                ) => require(
                    ar == br
                        && ae == be
                        && al == bl
                        && am == bm
                        && ao == bo
                        && ap == bp
                        && ac == bc,
                    "layer collective command/epoch differs",
                )?,
                _ => return Err("layer source step kind/order differs".into()),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "provenance_tests.rs"]
mod tests;
