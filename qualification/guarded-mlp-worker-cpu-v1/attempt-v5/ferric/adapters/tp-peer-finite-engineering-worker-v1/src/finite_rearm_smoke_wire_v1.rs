//! Four-forward rearm smoke only. Not a long-workload completion or benchmark.

use crate::finite_forward_wire_v1::{OBSERVATION_BYTES, Payload, part};
pub use crate::finite_long_wire_v1::{CONTROL_BYTES, Control, FrameBudget};
use crate::finite_long_wire_v1::{header_bytes, read_header, write_header};
use crate::finite_setup_wire_v1::{Part, Scope};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 1;
pub const FORWARDS: u32 = 4;
pub const PROMPT_TOKENS: usize = 4;
pub const STAGE_JSON_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    RearmFourForwardV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub protocol: u32,
    pub profile: Profile,
    pub device_ids: [u64; 2],
    pub scope: Scope,
    pub timeout_ms: u32,
    pub prompt_tokens: Vec<u32>,
    pub capture_layer0: bool,
}
impl Bootstrap {
    pub fn validate(
        &self,
        devices: [u64; 2],
        timeout_ms: u32,
        actual_pid: u32,
        capture_layer0: bool,
    ) -> io::Result<()> {
        check(
            self.protocol == PROTOCOL
                && devices_valid(self.device_ids)
                && self.device_ids == devices
                && self.timeout_ms == timeout_ms
                && (1..=10_000).contains(&timeout_ms)
                && self.scope.child_identity == actual_pid
                && actual_pid != 0
                && self.scope.bundle_id != [0; 32]
                && self.scope.model_id != [0; 32]
                && self.scope.session != [0; 32]
                && self.scope.pool_identity != 0
                && self.prompt_tokens.len() == PROMPT_TOKENS
                && self.prompt_tokens.iter().all(|token| *token < 151_936)
                && self.capture_layer0 == capture_layer0,
            "rearm smoke bootstrap identity, prompt or options",
        )
    }
    pub fn sha256(&self) -> io::Result<[u8; 32]> {
        self.validate(
            self.device_ids,
            self.timeout_ms,
            self.scope.child_identity,
            self.capture_layer0,
        )?;
        let mut hash = Sha256::new();
        hash.update(b"ferric-rearm-four-forward-profile-v1\0");
        hash.update(header_bytes(self)?);
        Ok(hash.finalize().into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
            Command::Forward {
                generation,
                token,
                cache_metadata,
                rotary_bits,
            } => {
                check(
                    self.id <= 4
                        && *generation == self.id
                        && *token < 151_936
                        && cache_metadata.len() == 145
                        && rotary_bits.len() == 128,
                    "smoke forward extent/order",
                )?;
                check(
                    cache_metadata[0] == self.id as u32 - 1
                        && rotary_bits
                            .iter()
                            .all(|word| f32::from_bits(*word).is_finite()),
                    "smoke position/rotary",
                )?;
                let mut seen = [false; 144];
                for &page in &cache_metadata[1..] {
                    let entry = seen
                        .get_mut(page as usize)
                        .ok_or_else(|| io::Error::other("smoke page bounds"))?;
                    check(!*entry, "smoke page alias")?;
                    *entry = true;
                }
                Ok(())
            }
            Command::Close => check(self.id == 5, "smoke Close follows exactly four forwards"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    pub generation: u64,
    pub position: u32,
    pub input_token: u32,
    pub output_token: u32,
    pub control: Part,
    pub observation: Part,
    pub capture: Payload,
    pub stage_capture: Option<Part>,
    pub chain: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    Completed(Completion),
    Closed {
        completed_forwards: u32,
        transcript_sha256: [u8; 32],
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
            "smoke claim boundary",
        )?;
        match &self.event {
            Event::Completed(c) => {
                check(
                    !self.native_closed
                        && self.id <= 4
                        && c.generation == self.id
                        && c.position == self.id as u32 - 1
                        && c.input_token < 151_936
                        && c.output_token < 151_936
                        && c.control.bytes as usize == CONTROL_BYTES
                        && c.observation.bytes as usize == OBSERVATION_BYTES
                        && c.capture.total == c.observation
                        && c.capture.layer_hidden.len() == 36
                        && c.capture.layer_hidden.iter().all(|part| part.bytes == 8192)
                        && c.capture.final_normalized.bytes == 8192
                        && c.capture.logits.bytes == 303_872,
                    "smoke completion shape",
                )?;
                let stage_bytes = if let Some(stage) = &c.stage_capture {
                    check(
                        c.position == 0
                            && stage.bytes > 0
                            && stage.bytes as usize <= STAGE_JSON_BYTES,
                        "smoke stage annex position or extent",
                    )?;
                    stage.bytes as usize
                } else {
                    0
                };
                Ok(CONTROL_BYTES + OBSERVATION_BYTES + stage_bytes)
            }
            Event::Closed {
                completed_forwards, ..
            } => {
                check(
                    self.native_closed && self.id == 5 && *completed_forwards == 4,
                    "smoke close acknowledgement",
                )?;
                Ok(0)
            }
        }
    }
}

pub struct Chain([u8; 32]);
impl Chain {
    pub fn new(registration: [u8; 32], profile: [u8; 32]) -> Self {
        let mut hash = Sha256::new();
        hash.update(b"ferric-rearm-four-forward-transcript-v1\0");
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
        hash.update(c.control.bytes.to_le_bytes());
        hash.update(c.control.sha256);
        hash.update(c.observation.bytes.to_le_bytes());
        hash.update(c.observation.sha256);
        match &c.stage_capture {
            Some(part) => {
                hash.update([1]);
                hash.update(part.bytes.to_le_bytes());
                hash.update(part.sha256);
            }
            None => hash.update([0]),
        }
        self.0 = hash.finalize().into();
        self.0
    }
    pub fn digest(&self) -> [u8; 32] {
        self.0
    }
}

fn devices_valid(ids: [u64; 2]) -> bool {
    ids[0] != 0 && ids[1] != 0 && ids[0] != ids[1]
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
            && (1..=5).contains(&id)
            && devices_valid(ids)
            && session != [0; 32]
            && registration != [0; 32]
            && profile != [0; 32],
        "smoke envelope",
    )
}
fn check(value: bool, message: &str) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(io::Error::other(message))
    }
}

pub fn read_bootstrap(
    reader: &mut impl Read,
    budget: &mut FrameBudget,
) -> io::Result<Option<Bootstrap>> {
    read_header(reader, budget)
}
pub fn write_bootstrap(
    writer: &mut impl Write,
    budget: &mut FrameBudget,
    value: &Bootstrap,
) -> io::Result<()> {
    value.validate(
        value.device_ids,
        value.timeout_ms,
        value.scope.child_identity,
        value.capture_layer0,
    )?;
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
pub fn write_request(
    writer: &mut impl Write,
    budget: &mut FrameBudget,
    value: &Request,
) -> io::Result<()> {
    value.validate()?;
    write_header(writer, budget, value)?;
    writer.flush()
}
pub fn write_response(
    writer: &mut impl Write,
    budget: &mut FrameBudget,
    value: &Response,
    control: Option<&Control>,
    capture: &[u8],
    stage: &[u8],
) -> io::Result<()> {
    let bytes = value.body_bytes()?;
    match &value.event {
        Event::Completed(c) => {
            let control = control.ok_or_else(|| io::Error::other("smoke control missing"))?;
            control.validate()?;
            let encoded = control.encode();
            check(
                part(&encoded) == c.control
                    && c.capture == Payload::from_bytes(capture)?
                    && CONTROL_BYTES + capture.len() + stage.len() == bytes,
                "smoke body hash or extent",
            )?;
            validate_annex(c.stage_capture.as_ref(), stage, capture)?;
            budget.charge(bytes)?;
            write_header(writer, budget, value)?;
            writer.write_all(&encoded)?;
            writer.write_all(capture)?;
            writer.write_all(stage)?;
        }
        Event::Closed { .. } => {
            check(
                control.is_none() && capture.is_empty() && stage.is_empty(),
                "smoke Close payload",
            )?;
            write_header(writer, budget, value)?;
        }
    }
    writer.flush()
}
pub fn read_response(
    reader: &mut impl Read,
    budget: &mut FrameBudget,
) -> io::Result<Option<(Response, Option<Control>, Vec<u8>, Vec<u8>)>> {
    let Some(value): Option<Response> = read_header(reader, budget)? else {
        return Ok(None);
    };
    let bytes = value.body_bytes()?;
    budget.charge(bytes)?;
    match &value.event {
        Event::Completed(c) => {
            let mut encoded = [0; CONTROL_BYTES];
            reader.read_exact(&mut encoded)?;
            check(part(&encoded) == c.control, "smoke received control hash")?;
            let control = Control::decode(&encoded)?;
            let mut capture = vec![0; OBSERVATION_BYTES];
            reader.read_exact(&mut capture)?;
            check(
                c.capture == Payload::from_bytes(&capture)?,
                "smoke received capture hash",
            )?;
            let mut stage = vec![0; bytes - CONTROL_BYTES - OBSERVATION_BYTES];
            reader.read_exact(&mut stage)?;
            validate_annex(c.stage_capture.as_ref(), &stage, &capture)?;
            Ok(Some((value, Some(control), capture, stage)))
        }
        Event::Closed { .. } => Ok(Some((value, None, vec![], vec![]))),
    }
}
fn validate_annex(expected: Option<&Part>, bytes: &[u8], observation: &[u8]) -> io::Result<()> {
    match expected {
        Some(expected) => {
            check(part(bytes) == *expected, "smoke stage annex hash")?;
            stage_capture::validate_observation(bytes, observation)
        }
        None => check(bytes.is_empty(), "undeclared smoke stage annex"),
    }
}

/// Bounded structural/numerical-shape validation only, never execution admission.
pub fn validate_stage_capture(bytes: &[u8]) -> io::Result<()> {
    stage_capture::validate(bytes)
}

#[path = "finite_rearm_smoke_stage_v1.rs"]
mod stage_capture;
#[cfg(test)]
#[path = "finite_rearm_smoke_wire_v1_tests.rs"]
mod tests;
