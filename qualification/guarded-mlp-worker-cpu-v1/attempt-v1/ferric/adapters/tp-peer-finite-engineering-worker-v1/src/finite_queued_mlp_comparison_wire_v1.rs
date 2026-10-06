//! Bounded inert framing for one explicit finite-versus-queued MLP comparison.
//! Bootstrap and Begin are trusted-parent custody, not model admission.

use crate::finite_forward_wire_v1::{OBSERVATION_BYTES, Payload, part};
pub use crate::finite_long_wire_v1::{CONTROL_BYTES, Control, FrameBudget};
use crate::finite_long_wire_v1::{read_header, write_header};
use crate::finite_setup_wire_v1::{Part, Scope};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 1;
pub const TOKEN: u32 = 9112;
pub const COMPARISON_JSON_BYTES: usize = 16 * 1024;
pub const NORM_IMAGE_BYTES: usize = 11_432;
pub const NORM_IMAGE_SHA256: [u8; 32] = [
    0xc3, 0x38, 0x82, 0xdb, 0x1a, 0xfc, 0xd8, 0xeb, 0x26, 0xec, 0x02, 0xbc, 0x43, 0xea, 0x21, 0x44,
    0xaf, 0x32, 0x2d, 0x29, 0xbf, 0x4e, 0x0e, 0x92, 0x1e, 0x03, 0xea, 0x74, 0xad, 0xc5, 0x5a, 0xf6,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    FiniteThenQueuedLayerZeroV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub protocol: u32,
    pub profile: Profile,
    pub device_ids: [u64; 2],
    pub scope: Scope,
    pub timeout_ms: u32,
    pub token: u32,
    pub norm_image: Part,
}

fn check(value: bool, message: &str) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(io::Error::other(message))
    }
}
fn devices(ids: [u64; 2]) -> bool {
    ids[0] != 0 && ids[1] != 0 && ids[0] != ids[1]
}

impl Bootstrap {
    pub fn validate(&self, ids: [u64; 2], timeout_ms: u32, actual_pid: u32) -> io::Result<()> {
        check(
            self.protocol == PROTOCOL
                && devices(self.device_ids)
                && self.device_ids == ids
                && (1..=10_000).contains(&timeout_ms)
                && self.timeout_ms == timeout_ms
                && self.token == TOKEN
                && self.scope.bundle_id != [0; 32]
                && self.scope.model_id != [0; 32]
                && self.scope.session != [0; 32]
                && self.scope.pool_identity != 0
                && actual_pid != 0
                && self.scope.child_identity == actual_pid
                && self.norm_image.bytes as usize == NORM_IMAGE_BYTES
                && self.norm_image.sha256 == NORM_IMAGE_SHA256,
            "queued comparison bootstrap identity/profile/image",
        )
    }
    pub fn sha256(&self) -> io::Result<[u8; 32]> {
        self.validate(self.device_ids, self.timeout_ms, self.scope.child_identity)?;
        let mut hash = Sha256::new();
        hash.update(b"ferric-finite-queued-mlp-comparison-profile-v1\0");
        hash.update(serde_json::to_vec(self).map_err(io::Error::other)?);
        Ok(hash.finalize().into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Forward {
        generation: u64,
        token: u32,
        cache_metadata: Vec<u32>,
        rotary_bits: Vec<u32>,
    },
    Close,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol: u32,
    pub id: u64,
    pub device_ids: [u64; 2],
    pub session: [u8; 32],
    pub registration: [u8; 32],
    pub profile_sha256: [u8; 32],
    pub command: Command,
}
fn envelope(
    protocol: u32,
    id: u64,
    ids: [u64; 2],
    session: [u8; 32],
    registration: [u8; 32],
    profile: [u8; 32],
) -> io::Result<()> {
    check(
        protocol == PROTOCOL
            && (1..=2).contains(&id)
            && devices(ids)
            && session != [0; 32]
            && registration != [0; 32]
            && profile != [0; 32],
        "queued comparison envelope",
    )
}
impl Request {
    pub fn validate(&self) -> io::Result<()> {
        envelope(
            self.protocol,
            self.id,
            self.device_ids,
            self.session,
            self.registration,
            self.profile_sha256,
        )?;
        match &self.command {
            Command::Close => check(self.id == 2, "queued comparison Close id"),
            Command::Forward {
                generation,
                token,
                cache_metadata,
                rotary_bits,
            } => {
                check(
                    self.id == 1
                        && *generation == 1
                        && *token == TOKEN
                        && cache_metadata.len() == 145
                        && cache_metadata[0] == 0
                        && rotary_bits.len() == 128
                        && rotary_bits
                            .iter()
                            .all(|bits| f32::from_bits(*bits).is_finite()),
                    "queued comparison first-forward shape",
                )?;
                let mut seen = [false; 144];
                for &page in &cache_metadata[1..] {
                    let value = seen
                        .get_mut(page as usize)
                        .ok_or_else(|| io::Error::other("queued comparison page bounds"))?;
                    check(!*value, "queued comparison page alias")?;
                    *value = true;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Norm,
    Gate,
    Up,
    Activation,
    Down,
}
impl Stage {
    pub const ALL: [Self; 5] = [
        Self::Norm,
        Self::Gate,
        Self::Up,
        Self::Activation,
        Self::Down,
    ];
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputEquality {
    pub stage: Stage,
    pub rank: u32,
    pub bytes: u32,
    pub words: u32,
    pub sha256: [u8; 32],
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonReport {
    pub schema: String,
    pub generation: u64,
    pub position: u32,
    pub layer: u32,
    pub finite_states: [[u32; 11]; 2],
    pub finite_queue_host_ns: [u64; 2],
    pub queued_stage_host_ns: [[u64; 2]; 5],
    pub equality: [[OutputEquality; 2]; 5],
    pub queued_semantic_state: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
}
impl ComparisonReport {
    pub fn validate(&self, control: &Control) -> io::Result<()> {
        control.validate()?;
        check(
            self.schema == "FerricFiniteQueuedMlpComparisonV1"
                && self.generation == 1
                && self.position == 0
                && self.layer == 0
                && self.finite_states == control.layers[0].mlp_states
                && self.finite_queue_host_ns == control.layers[0].paired_ns[2]
                && !self.queued_semantic_state
                && !self.numerical_acceptance
                && !self.performance_claim
                && !self.production_authority,
            "queued comparison report scope/finite provenance",
        )?;
        for (index, stage) in Stage::ALL.into_iter().enumerate() {
            for rank in 0..2 {
                let row = &self.equality[index][rank];
                check(
                    row.stage == stage
                        && row.rank == rank as u32
                        && row.bytes == [8192, 12288, 12288, 12288, 16384][index]
                        && row.words == [4096, 6144, 6144, 6144, 4096][index]
                        && row.sha256 != [0; 32],
                    "queued comparison output roster",
                )?;
            }
        }
        Ok(())
    }
}
pub fn validate_comparison(bytes: &[u8], control: &Control) -> io::Result<ComparisonReport> {
    check(
        !bytes.is_empty() && bytes.len() <= COMPARISON_JSON_BYTES,
        "queued comparison JSON bound",
    )?;
    let report: ComparisonReport = serde_json::from_slice(bytes).map_err(io::Error::other)?;
    report.validate(control)?;
    Ok(report)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    pub generation: u64,
    pub position: u32,
    pub input_token: u32,
    pub output_token: u32,
    pub control: Part,
    pub observation: Part,
    pub capture: Payload,
    pub comparison: Part,
    pub chain: [u8; 32],
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    Completed(Completion),
    Closed {
        completed_forwards: u32,
        transcript_sha256: [u8; 32],
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub protocol: u32,
    pub id: u64,
    pub device_ids: [u64; 2],
    pub session: [u8; 32],
    pub registration: [u8; 32],
    pub profile_sha256: [u8; 32],
    pub event: Event,
    pub native_closed: bool,
    pub gpu_execution: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
}
impl Response {
    fn body_bytes(&self) -> io::Result<usize> {
        envelope(
            self.protocol,
            self.id,
            self.device_ids,
            self.session,
            self.registration,
            self.profile_sha256,
        )?;
        check(
            self.gpu_execution
                && !self.numerical_acceptance
                && !self.performance_claim
                && !self.production_authority,
            "queued comparison authority flags",
        )?;
        match &self.event {
            Event::Closed {
                completed_forwards,
                transcript_sha256,
            } => {
                check(
                    self.id == 2
                        && self.native_closed
                        && *completed_forwards == 1
                        && *transcript_sha256 != [0; 32],
                    "queued comparison healthy Close",
                )?;
                Ok(0)
            }
            Event::Completed(c) => {
                check(
                    self.id == 1
                        && !self.native_closed
                        && c.generation == 1
                        && c.position == 0
                        && c.input_token == TOKEN
                        && c.output_token < 151_936
                        && c.control.bytes as usize == CONTROL_BYTES
                        && c.observation.bytes as usize == OBSERVATION_BYTES
                        && c.control.sha256 != [0; 32]
                        && c.observation.sha256 != [0; 32]
                        && c.capture.total == c.observation
                        && c.comparison.bytes > 0
                        && c.comparison.bytes as usize <= COMPARISON_JSON_BYTES
                        && c.comparison.sha256 != [0; 32]
                        && c.chain != [0; 32],
                    "queued comparison completion shape",
                )?;
                Ok(CONTROL_BYTES + OBSERVATION_BYTES + c.comparison.bytes as usize)
            }
        }
    }
}

pub struct Chain([u8; 32]);
impl Chain {
    pub fn new(registration: [u8; 32], profile: [u8; 32]) -> Self {
        let mut hash = Sha256::new();
        hash.update(b"ferric-finite-queued-mlp-comparison-transcript-v1\0");
        hash.update(registration);
        hash.update(profile);
        Self(hash.finalize().into())
    }
    pub fn advance(&mut self, c: &Completion) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(self.0);
        hash.update(c.generation.to_le_bytes());
        hash.update(c.position.to_le_bytes());
        hash.update(c.input_token.to_le_bytes());
        hash.update(c.output_token.to_le_bytes());
        for p in [&c.control, &c.observation, &c.comparison] {
            hash.update(p.bytes.to_le_bytes());
            hash.update(p.sha256);
        }
        self.0 = hash.finalize().into();
        self.0
    }
    pub fn digest(&self) -> [u8; 32] {
        self.0
    }
}

pub fn write_bootstrap(
    writer: &mut impl Write,
    budget: &mut FrameBudget,
    value: &Bootstrap,
    norm: &[u8],
) -> io::Result<()> {
    value.validate(
        value.device_ids,
        value.timeout_ms,
        value.scope.child_identity,
    )?;
    check(
        norm.len() == NORM_IMAGE_BYTES && part(norm) == value.norm_image,
        "queued norm object hash",
    )?;
    budget.charge(norm.len())?;
    write_header(writer, budget, value)?;
    writer.write_all(norm)?;
    writer.flush()
}
pub fn read_bootstrap(
    reader: &mut impl Read,
    budget: &mut FrameBudget,
) -> io::Result<Option<(Bootstrap, Vec<u8>)>> {
    let Some(value) = read_header::<Bootstrap>(reader, budget)? else {
        return Ok(None);
    };
    value.validate(
        value.device_ids,
        value.timeout_ms,
        value.scope.child_identity,
    )?;
    budget.charge(NORM_IMAGE_BYTES)?;
    let mut norm = vec![0; NORM_IMAGE_BYTES];
    reader.read_exact(&mut norm)?;
    check(part(&norm) == value.norm_image, "queued norm object hash")?;
    Ok(Some((value, norm)))
}
pub fn write_request(
    writer: &mut impl Write,
    budget: &mut FrameBudget,
    value: &Request,
) -> io::Result<()> {
    value.validate()?;
    write_header(writer, budget, value)?;
    writer.flush()
}
pub fn read_request(
    reader: &mut impl Read,
    budget: &mut FrameBudget,
) -> io::Result<Option<Request>> {
    let value: Option<Request> = read_header(reader, budget)?;
    if let Some(value) = &value {
        value.validate()?;
    }
    Ok(value)
}
fn main_payload(c: &Completion, bytes: &[u8]) -> io::Result<()> {
    check(
        bytes.len() == OBSERVATION_BYTES
            && part(bytes) == c.observation
            && Payload::from_bytes(bytes)? == c.capture,
        "queued main payload shape/hash",
    )?;
    check(
        bytes
            .chunks_exact(2)
            .all(|word| u16::from_le_bytes(word.try_into().unwrap()) & 0x7f80 != 0x7f80),
        "queued main payload nonfinite",
    )?;
    let logits = &bytes[37 * 8192..];
    let mut winner = 0;
    let mut best = f32::NEG_INFINITY;
    for (index, word) in logits.chunks_exact(2).enumerate() {
        let value = f32::from_bits(u32::from(u16::from_le_bytes(word.try_into().unwrap())) << 16);
        if value > best {
            best = value;
            winner = index as u32;
        }
    }
    check(winner == c.output_token, "queued main payload argmax")
}
pub fn write_response(
    writer: &mut impl Write,
    budget: &mut FrameBudget,
    value: &Response,
    control: Option<&Control>,
    main: &[u8],
    report: &[u8],
) -> io::Result<()> {
    let size = value.body_bytes()?;
    match &value.event {
        Event::Closed { .. } => check(
            control.is_none() && main.is_empty() && report.is_empty(),
            "queued Close body",
        )?,
        Event::Completed(c) => {
            let control = control.ok_or_else(|| io::Error::other("queued missing control"))?;
            control.validate()?;
            check(
                part(&control.encode()) == c.control && part(report) == c.comparison,
                "queued comparison body hashes",
            )?;
            main_payload(c, main)?;
            validate_comparison(report, control)?;
        }
    }
    budget.charge(size)?;
    write_header(writer, budget, value)?;
    if let Some(control) = control {
        writer.write_all(&control.encode())?;
    }
    writer.write_all(main)?;
    writer.write_all(report)?;
    writer.flush()
}
pub fn read_response(
    reader: &mut impl Read,
    budget: &mut FrameBudget,
) -> io::Result<Option<(Response, Option<Control>, Vec<u8>, Vec<u8>)>> {
    let Some(value) = read_header::<Response>(reader, budget)? else {
        return Ok(None);
    };
    let size = value.body_bytes()?;
    budget.charge(size)?;
    if let Event::Completed(c) = &value.event {
        let mut encoded = [0; CONTROL_BYTES];
        reader.read_exact(&mut encoded)?;
        let control = Control::decode(&encoded)?;
        check(part(&encoded) == c.control, "queued control hash")?;
        let mut main = vec![0; OBSERVATION_BYTES];
        reader.read_exact(&mut main)?;
        main_payload(c, &main)?;
        let mut report = vec![0; c.comparison.bytes as usize];
        reader.read_exact(&mut report)?;
        check(part(&report) == c.comparison, "queued comparison hash")?;
        validate_comparison(&report, &control)?;
        Ok(Some((value, Some(control), main, report)))
    } else {
        Ok(Some((value, None, Vec::new(), Vec::new())))
    }
}

#[cfg(test)]
#[path = "finite_queued_mlp_comparison_wire_v1_tests.rs"]
pub(crate) mod tests;
