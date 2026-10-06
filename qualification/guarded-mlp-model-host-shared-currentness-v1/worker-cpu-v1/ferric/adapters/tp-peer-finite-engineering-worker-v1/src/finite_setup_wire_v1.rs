//! Separate bounded setup protocol. No forward execution or Ready response.

use crate::finite_composition_wire::{self as source, PendingKind};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 1;
pub const MAX_COMMANDS: u64 = 16_384;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Key {
    Source {
        rank: u32,
        id: u64,
    },
    Pending {
        rank: u32,
        layer: Option<u32>,
        role: PendingKind,
    },
}

impl Key {
    pub fn rank(self) -> u32 {
        match self {
            Self::Source { rank, .. } | Self::Pending { rank, .. } => rank,
        }
    }
    fn validate(self) -> io::Result<()> {
        let valid = match self {
            Self::Source { rank, id } => rank < 2 && id != 0,
            Self::Pending { rank, layer, role } => {
                rank < 2
                    && match role {
                        PendingKind::PackedQkvWeight | PendingKind::PackedHeadNormWeight => {
                            layer.is_some_and(|v| v < 36)
                        }
                        PendingKind::QkvOutput
                        | PendingKind::Rotary
                        | PendingKind::CacheMetadata => layer.is_none(),
                    }
            }
        };
        check(valid, "setup semantic key")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub bundle_id: [u8; 32],
    pub model_id: [u8; 32],
    pub session: [u8; 32],
    pub pool_identity: u64,
    pub group_id: u64,
    pub child_identity: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    pub bytes: u32,
    pub sha256: [u8; 32],
}

impl Part {
    fn size(self, maximum: usize) -> io::Result<usize> {
        check(
            self.bytes > 0 && self.bytes as usize <= maximum && self.sha256 != [0; 32],
            "setup payload part bound",
        )?;
        Ok(self.bytes as usize)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Begin {
    pub scope: Scope,
    pub registration: Part,
    pub source_program: Part,
    pub uploads: Part,
    pub prefix_image: Part,
    pub mlp_image: Part,
    pub residual_image: Part,
    pub tail_image: Option<Part>,
}

impl Begin {
    pub fn parts(&self) -> [Part; 6] {
        [
            self.registration,
            self.source_program,
            self.uploads,
            self.prefix_image,
            self.mlp_image,
            self.residual_image,
        ]
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Upload {
    pub key: Key,
    pub bytes: u64,
    pub sha256: [u8; 32],
}

/// Distinct transpose extension; cannot reuse an original Source/Pending token.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TailHeadTranspose {
    pub source: Key,
    pub source_sha256: [u8; 32],
    pub bytes: u64,
    pub sha256: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadManifest {
    pub version: u32,
    pub uploads: Vec<Upload>,
    pub tail: Option<TailHeadTranspose>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Begin(Begin),
    Allocate { key: Key },
    Write { key: Key, offset: u64, part: Part },
    AllocateTailHead,
    WriteTailHead { offset: u64, part: Part },
    LoadArtifacts,
    BindAndSeal,
    AbortClose,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol: u32,
    pub id: u64,
    pub device_ids: [u64; 2],
    pub session: [u8; 32],
    pub command: Command,
}

impl Request {
    pub fn payload_bytes(&self) -> io::Result<usize> {
        check(
            self.protocol == PROTOCOL
                && (1..=MAX_COMMANDS).contains(&self.id)
                && self.device_ids[0] != 0
                && self.device_ids[1] != 0
                && self.device_ids[0] != self.device_ids[1]
                && self.session != [0; 32],
            "setup envelope",
        )?;
        match &self.command {
            Command::Begin(begin) => {
                check(
                    self.id == 1 && begin.scope.session == self.session,
                    "setup first command",
                )?;
                let base = begin.parts().into_iter().enumerate().try_fold(
                    0_usize,
                    |total, (i, part)| {
                        let size = part.size(if i < 3 {
                            source::MAX_REGISTRATION
                        } else {
                            source::MAX_IMAGE
                        })?;
                        total
                            .checked_add(size)
                            .ok_or_else(|| io::Error::other("setup payload sum overflow"))
                    },
                )?;
                base.checked_add(
                    begin
                        .tail_image
                        .map(|part| part.size(source::MAX_IMAGE))
                        .transpose()?
                        .unwrap_or(0),
                )
                .ok_or_else(|| io::Error::other("setup tail payload sum overflow"))
            }
            Command::Allocate { key } => {
                key.validate()?;
                Ok(0)
            }
            Command::Write { key, offset, part } => {
                key.validate()?;
                check(
                    offset.checked_add(u64::from(part.bytes)).is_some(),
                    "setup chunk offset overflow",
                )?;
                part.size(source::MAX_TRANSFER)
            }
            Command::WriteTailHead { offset, part } => {
                check(
                    offset.checked_add(u64::from(part.bytes)).is_some(),
                    "setup tail offset overflow",
                )?;
                part.size(source::MAX_TRANSFER)
            }
            Command::AllocateTailHead
            | Command::LoadArtifacts
            | Command::BindAndSeal
            | Command::AbortClose => Ok(0),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    OwnerCreated,
    Allocated,
    Uploaded,
    TailHeadAllocated,
    TailHeadUploaded,
    ArtifactsLoaded,
    LayersSealed,
    LayersAndTailSealed,
    Closed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub protocol: u32,
    pub id: u64,
    pub status: Status,
    pub catalog_id: Option<u64>,
    pub native_opened: bool,
    pub gpu_execution: bool,
    pub forward_started: bool,
    pub production_authority: bool,
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
    check(
        length > 0 && length <= source::MAX_HEADER,
        "setup header bound",
    )?;
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
                .is_some_and(|n| n <= source::MAX_HEADER),
            "encoded setup header bound",
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

pub fn read_request(reader: &mut impl Read) -> io::Result<Option<(Request, Vec<u8>)>> {
    let Some(request): Option<Request> = read_header(reader)? else {
        return Ok(None);
    };
    let mut payload = vec![0; request.payload_bytes()?];
    reader.read_exact(&mut payload)?;
    Ok(Some((request, payload)))
}

pub fn write_request(writer: &mut impl Write, request: &Request, payload: &[u8]) -> io::Result<()> {
    check(
        request.payload_bytes()? == payload.len(),
        "setup exact payload length",
    )?;
    write_header(writer, request)?;
    writer.write_all(payload)?;
    writer.flush()
}

fn response_valid(response: &Response) -> bool {
    response.protocol == PROTOCOL
        && (1..=MAX_COMMANDS).contains(&response.id)
        && response.native_opened
        && !response.gpu_execution
        && !response.forward_started
        && !response.production_authority
        && match response.status {
            Status::OwnerCreated => response.id == 1 && response.catalog_id.is_none(),
            Status::Allocated => response.catalog_id.is_some_and(|id| id != 0),
            _ => response.catalog_id.is_none(),
        }
        && (response.id != 1 || response.status == Status::OwnerCreated)
}

pub fn write_response(writer: &mut impl Write, response: &Response) -> io::Result<()> {
    check(response_valid(response), "setup response scope")?;
    write_header(writer, response)?;
    writer.flush()
}

pub fn read_response(reader: &mut impl Read) -> io::Result<Option<Response>> {
    let response = read_header(reader)?;
    if let Some(ref value) = response {
        check(response_valid(value), "setup response scope")?;
    }
    Ok(response)
}

#[cfg(test)]
#[path = "finite_setup_wire_v1_tests.rs"]
mod tests;
