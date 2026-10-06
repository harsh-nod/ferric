//! Closed long engineering stream. Original setup custody is not long execution proof.

use crate::finite_composition_wire::MAX_HEADER;
use crate::finite_forward_wire_v1::{LayerObservation, OBSERVATION_BYTES, Payload, part};
use crate::finite_setup_wire_v1::{Part, Scope};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 1;
pub const PROMPT_TOKENS: usize = 2048;
pub const OUTPUT_TOKENS: usize = 256;
pub const FORWARDS: u32 = 2303;
pub const CONTROL_BYTES: usize = 11_848;
pub const CAPTURE_POSITIONS: [u32; 4] = [0, 2047, 2048, 2302];
pub const STREAM_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    Prompt2048Output256V1,
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
}

impl Bootstrap {
    pub fn validate(&self, devices: [u64; 2], timeout_ms: u32, actual_pid: u32) -> io::Result<()> {
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
                && self.prompt_tokens.iter().all(|token| *token < 151_936),
            "long bootstrap identity, prompt or invocation",
        )
    }

    /// A custody digest, not model or executable admission.
    pub fn sha256(&self) -> io::Result<[u8; 32]> {
        self.validate(self.device_ids, self.timeout_ms, self.scope.child_identity)?;
        let mut hash = Sha256::new();
        hash.update(b"ferric-finite-long-profile-v1\0");
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
                    self.id <= u64::from(FORWARDS)
                        && *generation == self.id
                        && *token < 151_936
                        && cache_metadata.len() == 145
                        && rotary_bits.len() == 128,
                    "long forward order or extent",
                )?;
                check(
                    cache_metadata[0] == self.id as u32 - 1
                        && rotary_bits
                            .iter()
                            .all(|bits| f32::from_bits(*bits).is_finite()),
                    "long forward position or rotary",
                )?;
                let mut seen = [false; 144];
                for &page in &cache_metadata[1..] {
                    let entry = seen
                        .get_mut(page as usize)
                        .ok_or_else(|| io::Error::other("long page bounds"))?;
                    check(!*entry, "long page alias")?;
                    *entry = true;
                }
                Ok(())
            }
            Command::Close => check(self.id == u64::from(FORWARDS) + 1, "long premature close"),
        }
    }
}

/// Fixed LE encoding: embedding[2], then36(prefix rank0/1, MLP rank0/1,
/// paired durations phase0..3/rank0..1), then tail[3]. No decimal array expansion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Control {
    pub embedding_ns: [u64; 2],
    pub layers: [LayerObservation; 36],
    pub tail_ns: [u64; 3],
}

impl Control {
    pub fn validate(&self) -> io::Result<()> {
        for layer in &self.layers {
            for rank in 0..2 {
                terminal(&layer.prefix_states[rank], 16)?;
                terminal(&layer.mlp_states[rank], 5)?;
            }
        }
        Ok(())
    }

    pub fn encode(&self) -> [u8; CONTROL_BYTES] {
        let mut bytes = [0; CONTROL_BYTES];
        let mut offset = 0;
        let mut put = |data: &[u8]| {
            bytes[offset..offset + data.len()].copy_from_slice(data);
            offset += data.len();
        };
        for value in self.embedding_ns {
            put(&value.to_le_bytes());
        }
        for layer in &self.layers {
            for words in &layer.prefix_states {
                for word in words {
                    put(&word.to_le_bytes());
                }
            }
            for words in &layer.mlp_states {
                for word in words {
                    put(&word.to_le_bytes());
                }
            }
            for times in &layer.paired_ns {
                for time in times {
                    put(&time.to_le_bytes());
                }
            }
        }
        for value in self.tail_ns {
            put(&value.to_le_bytes());
        }
        bytes
    }

    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        check(bytes.len() == CONTROL_BYTES, "long control extent")?;
        let mut cursor = io::Cursor::new(bytes);
        fn word<const N: usize>(cursor: &mut io::Cursor<&[u8]>) -> io::Result<[u8; N]> {
            let mut value = [0; N];
            cursor.read_exact(&mut value)?;
            Ok(value)
        }
        let mut result = Self {
            embedding_ns: [0; 2],
            layers: core::array::from_fn(|_| LayerObservation {
                prefix_states: [[0; 22]; 2],
                mlp_states: [[0; 11]; 2],
                paired_ns: [[0; 2]; 4],
            }),
            tail_ns: [0; 3],
        };
        for value in &mut result.embedding_ns {
            *value = u64::from_le_bytes(word(&mut cursor)?);
        }
        for layer in &mut result.layers {
            for words in &mut layer.prefix_states {
                for value in words {
                    *value = u32::from_le_bytes(word(&mut cursor)?);
                }
            }
            for words in &mut layer.mlp_states {
                for value in words {
                    *value = u32::from_le_bytes(word(&mut cursor)?);
                }
            }
            for times in &mut layer.paired_ns {
                for value in times {
                    *value = u64::from_le_bytes(word(&mut cursor)?);
                }
            }
        }
        for value in &mut result.tail_ns {
            *value = u64::from_le_bytes(word(&mut cursor)?);
        }
        result.validate()?;
        Ok(result)
    }
}

fn terminal(words: &[u32], tasks: usize) -> io::Result<()> {
    let mask = (1u32 << tasks) - 1;
    check(
        words.len() == tasks + 6
            && words[..4] == [1, 0, mask, mask]
            && words[5] == 0
            && words[6..].iter().all(|word| *word == 64)
            && (0..tasks).all(|task| matches!((words[4] >> (task * 2)) & 3, 1 | 2))
            && (tasks == 16 || words[4] >> (tasks * 2) == 0),
        "long nonterminal state",
    )
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    pub generation: u64,
    pub position: u32,
    pub input_token: u32,
    pub output_token: u32,
    pub control: Part,
    /// Digest of all606976 checked bytes even when the raw capture is omitted.
    pub observation: Part,
    pub capture: Option<Payload>,
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
            "long response claim boundary",
        )?;
        match &self.event {
            Event::Completed(c) => {
                check(
                    !self.native_closed
                        && self.id <= u64::from(FORWARDS)
                        && c.generation == self.id
                        && c.position == self.id as u32 - 1
                        && c.input_token < 151_936
                        && c.output_token < 151_936
                        && c.control.bytes as usize == CONTROL_BYTES
                        && c.observation.bytes as usize == OBSERVATION_BYTES
                        && c.capture.is_some() == capture_position(c.position),
                    "long completion shape",
                )?;
                if let Some(p) = &c.capture {
                    check(
                        p.total == c.observation
                            && p.layer_hidden.len() == 36
                            && p.layer_hidden.iter().all(|part| part.bytes == 8192)
                            && p.final_normalized.bytes == 8192
                            && p.logits.bytes == 303_872,
                        "long selected capture shape",
                    )?;
                }
                Ok(CONTROL_BYTES
                    + if c.capture.is_some() {
                        OBSERVATION_BYTES
                    } else {
                        0
                    })
            }
            Event::Closed {
                completed_forwards, ..
            } => {
                check(
                    self.native_closed
                        && self.id == u64::from(FORWARDS) + 1
                        && *completed_forwards == FORWARDS,
                    "long close acknowledgement",
                )?;
                Ok(0)
            }
        }
    }
}

pub fn capture_position(position: u32) -> bool {
    CAPTURE_POSITIONS.contains(&position)
}

pub struct Chain([u8; 32]);
impl Chain {
    pub fn new(registration: [u8; 32], profile_sha256: [u8; 32]) -> Self {
        let mut hash = Sha256::new();
        hash.update(b"ferric-finite-long-transcript-v1\0");
        hash.update(registration);
        hash.update(profile_sha256);
        Self(hash.finalize().into())
    }
    /// Call only after validating this completion and body; does not trust c.chain.
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
        self.0 = hash.finalize().into();
        self.0
    }
    pub fn digest(&self) -> [u8; 32] {
        self.0
    }
}

/// Per-direction forward/bootstrap budget. Existing separately bounded setup
/// upload frames are not misreported as belonging to this observation stream.
pub struct FrameBudget {
    used: usize,
}
impl Default for FrameBudget {
    fn default() -> Self {
        Self::new()
    }
}
impl FrameBudget {
    pub const fn new() -> Self {
        Self { used: 0 }
    }
    pub const fn used(&self) -> usize {
        self.used
    }
    pub(crate) fn charge(&mut self, amount: usize) -> io::Result<()> {
        self.used = self
            .used
            .checked_add(amount)
            .ok_or_else(|| io::Error::other("long stream accounting overflow"))?;
        check(self.used <= STREAM_BYTES, "long cumulative stream bound")
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
            && (1..=u64::from(FORWARDS) + 1).contains(&id)
            && devices_valid(ids)
            && session != [0; 32]
            && registration != [0; 32]
            && profile != [0; 32],
        "long envelope",
    )
}
fn check(value: bool, message: &str) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(io::Error::other(message))
    }
}
struct BoundedHeader(Vec<u8>);
impl Write for BoundedHeader {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        check(
            self.0
                .len()
                .checked_add(bytes.len())
                .is_some_and(|n| n <= MAX_HEADER),
            "long encoded header bound",
        )?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(crate) fn header_bytes(value: &impl Serialize) -> io::Result<Vec<u8>> {
    let mut bytes = BoundedHeader(Vec::new());
    serde_json::to_writer(&mut bytes, value).map_err(io::Error::other)?;
    Ok(bytes.0)
}
pub(crate) fn write_header(
    writer: &mut impl Write,
    budget: &mut FrameBudget,
    value: &impl Serialize,
) -> io::Result<()> {
    let bytes = header_bytes(value)?;
    budget.charge(4 + bytes.len())?;
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(&bytes)
}
pub(crate) fn read_header<T: serde::de::DeserializeOwned>(
    reader: &mut impl Read,
    budget: &mut FrameBudget,
) -> io::Result<Option<T>> {
    let mut prefix = [0; 4];
    if reader.read(&mut prefix[..1])? == 0 {
        return Ok(None);
    }
    budget.charge(4)?;
    reader.read_exact(&mut prefix[1..])?;
    let length = u32::from_le_bytes(prefix) as usize;
    check(
        length > 0 && length <= MAX_HEADER,
        "long received header bound",
    )?;
    budget.charge(length)?;
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(io::Error::other)
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
) -> io::Result<()> {
    let body_bytes = value.body_bytes()?;
    match &value.event {
        Event::Completed(c) => {
            let control = control.ok_or_else(|| io::Error::other("long control missing"))?;
            control.validate()?;
            let bytes = control.encode();
            check(part(&bytes) == c.control, "long control digest")?;
            check(
                capture.len() + CONTROL_BYTES == body_bytes,
                "long selected payload length",
            )?;
            if let Some(p) = &c.capture {
                check(
                    *p == Payload::from_bytes(capture)?,
                    "long selected payload digest",
                )?;
            }
            budget.charge(body_bytes)?;
            write_header(writer, budget, value)?;
            writer.write_all(&bytes)?;
            writer.write_all(capture)?;
        }
        Event::Closed { .. } => {
            check(
                control.is_none() && capture.is_empty(),
                "long close has payload",
            )?;
            write_header(writer, budget, value)?;
        }
    }
    writer.flush()
}
pub fn read_response(
    reader: &mut impl Read,
    budget: &mut FrameBudget,
) -> io::Result<Option<(Response, Option<Control>, Vec<u8>)>> {
    let Some(value): Option<Response> = read_header(reader, budget)? else {
        return Ok(None);
    };
    let body_bytes = value.body_bytes()?;
    budget.charge(body_bytes)?;
    match &value.event {
        Event::Completed(c) => {
            let mut bytes = [0; CONTROL_BYTES];
            reader.read_exact(&mut bytes)?;
            check(part(&bytes) == c.control, "long received control digest")?;
            let control = Control::decode(&bytes)?;
            let mut capture = vec![0; body_bytes - CONTROL_BYTES];
            reader.read_exact(&mut capture)?;
            if let Some(p) = &c.capture {
                check(
                    *p == Payload::from_bytes(&capture)?,
                    "long received payload digest",
                )?;
            }
            Ok(Some((value, Some(control), capture)))
        }
        Event::Closed { .. } => Ok(Some((value, None, Vec::new()))),
    }
}

#[cfg(test)]
#[path = "finite_long_wire_v1_tests.rs"]
mod tests;
