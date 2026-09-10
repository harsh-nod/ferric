//! Bounded rank-tagged framing, shared by the disposable child and safe parent.

use fe2o3_kfd::engineering_wire::{self as base, CommandV1, ResponseV1};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 4;
pub const MODE: &str = "device-peer-serial-v4";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(rename = "request")]
    pub id: u64,
    pub rank: u32,
    pub peer_readable: bool,
    pub command: CommandV1,
}

impl Request {
    pub fn payload_bytes(&self) -> io::Result<usize> {
        if self.id == 0
            || self.rank >= 8
            || self.peer_readable && !matches!(self.command, CommandV1::Allocate { .. })
        {
            return Err(io::Error::other("peer request scope"));
        }
        self.command.payload_bytes()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "peer_op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    Ready {
        protocol: u32,
        mode: String,
        target: String,
        unique_ids: Vec<u64>,
        process_id: u32,
        authority: String,
    },
    Done {
        request: u64,
        rank: u32,
        response: ResponseV1,
    },
}

impl Response {
    pub fn payload_bytes(&self) -> io::Result<usize> {
        match self {
            Self::Done {
                response: ResponseV1::Read { payload_bytes },
                ..
            } if *payload_bytes <= base::MAX_TRANSFER_BYTES_V1 => Ok(*payload_bytes as usize),
            Self::Done {
                response: ResponseV1::Read { .. },
                ..
            } => Err(io::Error::other("peer response transfer bound")),
            _ => Ok(0),
        }
    }
}

pub fn read_request(input: &mut impl Read) -> io::Result<Option<(Request, Vec<u8>)>> {
    let Some(header) = base::read_header_v1::<Request>(input)? else {
        return Ok(None);
    };
    let mut payload = vec![0; header.payload_bytes()?];
    input.read_exact(&mut payload)?;
    Ok(Some((header, payload)))
}

pub fn read_response(input: &mut impl Read) -> io::Result<(Response, Vec<u8>)> {
    let header = base::read_header_v1::<Response>(input)?
        .ok_or_else(|| io::Error::other("peer response pipe closed"))?;
    let mut payload = vec![0; header.payload_bytes()?];
    input.read_exact(&mut payload)?;
    Ok((header, payload))
}

pub fn write_request(output: &mut impl Write, header: &Request, payload: &[u8]) -> io::Result<()> {
    if header.payload_bytes()? != payload.len() {
        return Err(io::Error::other("peer request payload mismatch"));
    }
    base::write_header_v1(output, header)?;
    output.write_all(payload)?;
    output.flush()
}

pub fn write_response(
    output: &mut impl Write,
    header: &Response,
    payload: &[u8],
) -> io::Result<()> {
    if header.payload_bytes()? != payload.len() {
        return Err(io::Error::other("peer response payload mismatch"));
    }
    base::write_header_v1(output, header)?;
    output.write_all(payload)?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequences_reuse_count_payload_and_aggregate_timeout_bounds() {
        let entry = base::SequenceDispatchV1 {
            kernel: 1,
            payload_bytes: 4,
            workgroup: [64, 1, 1],
            grid: [64, 1, 1],
            pointers: vec![],
            timeout_ms: 300_000,
        };
        let request = |entries| Request {
            id: 1,
            rank: 1,
            peer_readable: false,
            command: CommandV1::DispatchSequence {
                dispatches: entries,
            },
        };
        assert_eq!(request(vec![entry.clone(); 2]).payload_bytes().unwrap(), 8);
        assert!(request(vec![entry.clone(); 3]).payload_bytes().is_err());
        assert!(request(vec![]).payload_bytes().is_err());
        assert!(request(vec![entry; 17]).payload_bytes().is_err());
    }

    #[test]
    fn rejects_invalid_scope_before_reading_binary_payload() {
        let mut request = Request {
            id: 1,
            rank: 0,
            peer_readable: false,
            command: CommandV1::Read {
                buffer: 1,
                offset: 0,
                bytes: 4,
            },
        };
        assert_eq!(request.payload_bytes().unwrap(), 0);
        request.peer_readable = true;
        assert!(request.payload_bytes().is_err());
        request.peer_readable = false;
        request.rank = 8;
        assert!(request.payload_bytes().is_err());
        request.rank = 0;
        request.id = 0;
        assert!(request.payload_bytes().is_err());
    }

    #[test]
    fn rejects_truncated_payload_and_oversized_response() {
        let request = Request {
            id: 1,
            rank: 0,
            peer_readable: false,
            command: CommandV1::Write {
                buffer: 1,
                offset: 0,
                payload_bytes: 4,
            },
        };
        let mut bytes = Vec::new();
        write_request(&mut bytes, &request, &[1, 2, 3, 4]).unwrap();
        bytes.pop();
        assert!(read_request(&mut bytes.as_slice()).is_err());
        let response = Response::Done {
            request: 1,
            rank: 0,
            response: ResponseV1::Read {
                payload_bytes: base::MAX_TRANSFER_BYTES_V1 + 1,
            },
        };
        assert!(response.payload_bytes().is_err());
    }
}
