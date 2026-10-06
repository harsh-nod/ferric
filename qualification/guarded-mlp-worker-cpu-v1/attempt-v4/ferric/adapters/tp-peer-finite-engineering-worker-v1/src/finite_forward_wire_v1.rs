//! Closed engineering observations. Parent custody is not production admission.

use crate::finite_composition_wire::MAX_HEADER;
use crate::finite_setup_wire_v1::{Part, Scope};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 1;
pub const LAYERS: usize = 36;
pub const ROW_BYTES: usize = 8192;
pub const LOGIT_BYTES: usize = 151_936 * 2;
pub const OBSERVATION_BYTES: usize = LAYERS * ROW_BYTES + ROW_BYTES + LOGIT_BYTES;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    TeacherForced,
    Autoregressive,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub protocol: u32,
    pub device_ids: [u64; 2],
    pub scope: Scope,
    pub mode: InputMode,
    pub timeout_ms: u32,
}

impl Bootstrap {
    pub fn validate(
        &self,
        devices: [u64; 2],
        mode: InputMode,
        timeout_ms: u32,
        actual_pid: u32,
    ) -> io::Result<()> {
        check(
            self.protocol == PROTOCOL
                && devices_valid(self.device_ids)
                && self.device_ids == devices
                && self.mode == mode
                && self.timeout_ms == timeout_ms
                && (1..=10_000).contains(&timeout_ms)
                && self.scope.child_identity == actual_pid
                && actual_pid != 0
                && self.scope.bundle_id != [0; 32]
                && self.scope.model_id != [0; 32]
                && self.scope.session != [0; 32]
                && self.scope.pool_identity != 0,
            "finite bootstrap identity or invocation mismatch",
        )
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
        )?;
        match &self.command {
            Command::Forward {
                generation,
                token,
                cache_metadata,
                rotary_bits,
            } => {
                check(
                    self.id <= 2
                        && *generation == self.id
                        && *token < 151_936
                        && cache_metadata.len() == 145
                        && rotary_bits.len() == 128,
                    "finite forward exact extents or order",
                )?;
                check(
                    cache_metadata[0] == self.id as u32 - 1
                        && rotary_bits
                            .iter()
                            .all(|word| f32::from_bits(*word).is_finite()),
                    "finite forward position or rotary",
                )?;
                let mut seen = [false; 144];
                for &page in &cache_metadata[1..] {
                    let entry = seen
                        .get_mut(page as usize)
                        .ok_or_else(|| io::Error::other("finite forward page bounds"))?;
                    check(!*entry, "finite forward duplicate page")?;
                    *entry = true;
                }
                Ok(())
            }
            Command::Close => check(self.id == 3, "finite close must follow two forwards"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerObservation {
    pub prefix_states: [[u32; 22]; 2],
    pub mlp_states: [[u32; 11]; 2],
    /// Host-observed paired queue durations, not device cycles or a benchmark.
    pub paired_ns: [[u64; 2]; 4],
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Payload {
    pub layer_hidden: Vec<Part>,
    pub final_normalized: Part,
    pub logits: Part,
    pub total: Part,
}

impl Payload {
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        check(
            bytes.len() == OBSERVATION_BYTES,
            "finite observation extent",
        )?;
        let split = LAYERS * ROW_BYTES;
        Ok(Self {
            layer_hidden: bytes[..split].chunks_exact(ROW_BYTES).map(part).collect(),
            final_normalized: part(&bytes[split..split + ROW_BYTES]),
            logits: part(&bytes[split + ROW_BYTES..]),
            total: part(bytes),
        })
    }

    fn valid_shape(&self) -> bool {
        self.layer_hidden.len() == LAYERS
            && self
                .layer_hidden
                .iter()
                .all(|p| p.bytes as usize == ROW_BYTES)
            && self.final_normalized.bytes as usize == ROW_BYTES
            && self.logits.bytes as usize == LOGIT_BYTES
            && self.total.bytes as usize == OBSERVATION_BYTES
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    pub generation: u64,
    pub position: u32,
    pub input_token: u32,
    pub output_token: u32,
    pub embedding_ns: [u64; 2],
    pub layers: Vec<LayerObservation>,
    pub tail_ns: [u64; 3],
    pub payload: Payload,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    Completed(Completion),
    Closed { completed_forwards: u32 },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub protocol: u32,
    pub id: u64,
    pub device_ids: [u64; 2],
    pub session: [u8; 32],
    pub registration: [u8; 32],
    pub event: Event,
    pub native_closed: bool,
    pub gpu_execution: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
}

impl Response {
    fn payload_bytes(&self) -> io::Result<usize> {
        envelope(
            self.protocol,
            self.id,
            self.device_ids,
            self.session,
            self.registration,
        )?;
        check(
            self.gpu_execution
                && !self.numerical_acceptance
                && !self.performance_claim
                && !self.production_authority,
            "finite response claim boundary",
        )?;
        match &self.event {
            Event::Completed(c) => {
                check(
                    !self.native_closed
                        && self.id <= 2
                        && c.generation == self.id
                        && c.position == self.id as u32 - 1
                        && c.input_token < 151_936
                        && c.output_token < 151_936
                        && c.layers.len() == LAYERS
                        && c.payload.valid_shape(),
                    "finite completion shape",
                )?;
                Ok(OBSERVATION_BYTES)
            }
            Event::Closed { completed_forwards } => {
                check(
                    self.native_closed && self.id == 3 && *completed_forwards == 2,
                    "finite close acknowledgement",
                )?;
                Ok(0)
            }
        }
    }
}

pub fn part(bytes: &[u8]) -> Part {
    Part {
        bytes: bytes.len() as u32,
        sha256: Sha256::digest(bytes).into(),
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
) -> io::Result<()> {
    check(
        protocol == PROTOCOL
            && (1..=3).contains(&id)
            && devices_valid(ids)
            && session != [0; 32]
            && registration != [0; 32],
        "finite forward envelope",
    )
}
fn check(value: bool, message: &str) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(io::Error::other(message))
    }
}

fn read_header<T: serde::de::DeserializeOwned>(reader: &mut impl Read) -> io::Result<Option<T>> {
    let mut prefix = [0; 4];
    if reader.read(&mut prefix[..1])? == 0 {
        return Ok(None);
    }
    reader.read_exact(&mut prefix[1..])?;
    let length = u32::from_le_bytes(prefix) as usize;
    check(length > 0 && length <= MAX_HEADER, "finite header bound")?;
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(io::Error::other)
}

struct BoundedHeader(Vec<u8>);
impl Write for BoundedHeader {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        check(
            self.0
                .len()
                .checked_add(bytes.len())
                .is_some_and(|n| n <= MAX_HEADER),
            "encoded finite header bound",
        )?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn write_header(writer: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    let mut bytes = BoundedHeader(Vec::new());
    serde_json::to_writer(&mut bytes, value).map_err(io::Error::other)?;
    writer.write_all(&(bytes.0.len() as u32).to_le_bytes())?;
    writer.write_all(&bytes.0)
}

pub fn read_bootstrap(reader: &mut impl Read) -> io::Result<Option<Bootstrap>> {
    read_header(reader)
}
pub fn write_bootstrap(writer: &mut impl Write, bootstrap: &Bootstrap) -> io::Result<()> {
    bootstrap.validate(
        bootstrap.device_ids,
        bootstrap.mode,
        bootstrap.timeout_ms,
        bootstrap.scope.child_identity,
    )?;
    write_header(writer, bootstrap)?;
    writer.flush()
}
pub fn read_request(reader: &mut impl Read) -> io::Result<Option<Request>> {
    let request: Option<Request> = read_header(reader)?;
    if let Some(ref request) = request {
        request.validate()?;
    }
    Ok(request)
}
pub fn write_request(writer: &mut impl Write, request: &Request) -> io::Result<()> {
    request.validate()?;
    write_header(writer, request)?;
    writer.flush()
}
pub fn write_response(
    writer: &mut impl Write,
    response: &Response,
    bytes: &[u8],
) -> io::Result<()> {
    check(
        response.payload_bytes()? == bytes.len(),
        "finite response payload length",
    )?;
    if let Event::Completed(c) = &response.event {
        check(
            c.payload == Payload::from_bytes(bytes)?,
            "finite response payload digest",
        )?;
    }
    write_header(writer, response)?;
    writer.write_all(bytes)?;
    writer.flush()
}
pub fn read_response(reader: &mut impl Read) -> io::Result<Option<(Response, Vec<u8>)>> {
    let Some(response): Option<Response> = read_header(reader)? else {
        return Ok(None);
    };
    let mut bytes = vec![0; response.payload_bytes()?];
    reader.read_exact(&mut bytes)?;
    if let Event::Completed(c) = &response.event {
        check(
            c.payload == Payload::from_bytes(&bytes)?,
            "finite response payload digest",
        )?;
    }
    Ok(Some((response, bytes)))
}

#[cfg(test)]
#[path = "finite_forward_wire_v1_tests.rs"]
mod tests;
