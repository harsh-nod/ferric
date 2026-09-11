//! Bounded rank-tagged framing, shared by the disposable child and safe parent.

use fe2o3_kfd::engineering_wire::{self as base, CommandV1, ResponseV1};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 4;
pub const MODE: &str = "device-peer-serial-v4";
pub const ROUND_MODE: &str = "device-peer-concurrent-round-v1";

#[allow(clippy::trivially_copy_pass_by_ref)] // Serde predicates require a borrowed field.
fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(rename = "request")]
    pub id: u64,
    pub rank: u32,
    pub peer_readable: bool,
    pub command: CommandV1,
    /// Empty preserves the serial protocol. Otherwise entries name distinct ranks
    /// for a single, all-or-terminal mixed-rank dispatch round.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub round_ranks: Vec<u32>,
}

impl Request {
    pub fn payload_bytes(&self) -> io::Result<usize> {
        if self.id == 0
            || self.rank >= 8
            || self.peer_readable && !matches!(self.command, CommandV1::Allocate { .. })
        {
            return Err(io::Error::other("peer request scope"));
        }
        if !self.round_ranks.is_empty() {
            let CommandV1::DispatchSequence { dispatches } = &self.command else {
                return Err(io::Error::other("peer round requires dispatch entries"));
            };
            if self.peer_readable
                || self.round_ranks.len() > 8
                || self.round_ranks.len() != dispatches.len()
                || self.round_ranks.first() != Some(&self.rank)
                || self.round_ranks.iter().any(|&rank| rank >= 8)
                || self
                    .round_ranks
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != self.round_ranks.len()
            {
                return Err(io::Error::other("peer round rank roster"));
            }
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
        #[serde(default, skip_serializing_if = "is_false")]
        shared_full_currentness: bool,
    },
    Done {
        request: u64,
        rank: u32,
        response: ResponseV1,
    },
    RoundDone {
        request: u64,
        ranks: Vec<u32>,
        response: ResponseV1,
    },
}

impl Response {
    pub fn payload_bytes(&self) -> io::Result<usize> {
        match self {
            Self::RoundDone {
                request,
                ranks,
                response,
            } => {
                if *request == 0
                    || ranks.is_empty()
                    || ranks.len() > 8
                    || ranks.iter().any(|&rank| rank >= 8)
                    || ranks
                        .iter()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        != ranks.len()
                    || !matches!(response,
                        ResponseV1::DispatchSequenceCompleted { elapsed_ns } if elapsed_ns.len() == ranks.len())
                        && !matches!(response, ResponseV1::Error { fatal: true, .. })
                {
                    return Err(io::Error::other("peer round completion scope"));
                }
                Ok(0)
            }
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
    fn shared_fence_handshake_is_explicit_and_legacy_bytes_stay_unchanged() {
        let ready = |shared_full_currentness| Response::Ready {
            protocol: PROTOCOL,
            mode: MODE.into(),
            target: "gfx950:xnack-".into(),
            unique_ids: vec![1, 2],
            process_id: 42,
            authority: "none".into(),
            shared_full_currentness,
        };
        let legacy = serde_json::to_value(ready(false)).unwrap();
        assert!(legacy.get("shared_full_currentness").is_none());
        assert_eq!(
            serde_json::to_value(ready(true)).unwrap()["shared_full_currentness"],
            true
        );
        for shared in [false, true] {
            let mut bytes = Vec::new();
            write_response(&mut bytes, &ready(shared), &[]).unwrap();
            let (decoded, _) = read_response(&mut &bytes[..]).unwrap();
            assert!(
                matches!(decoded, Response::Ready { shared_full_currentness, .. } if shared_full_currentness == shared)
            );
        }
    }

    #[test]
    fn concurrent_round_checks_roster_shape_before_payload() {
        let dispatch = base::SequenceDispatchV1 {
            kernel: 1,
            payload_bytes: 4,
            workgroup: [64, 1, 1],
            grid: [64, 1, 1],
            pointers: vec![],
            timeout_ms: 1000,
        };
        let mut request = Request {
            id: 1,
            rank: 1,
            peer_readable: false,
            round_ranks: vec![1, 0],
            command: CommandV1::DispatchSequence {
                dispatches: vec![dispatch; 2],
            },
        };
        assert_eq!(request.payload_bytes().unwrap(), 8);
        for ranks in [vec![1, 1], vec![1], vec![0, 1], vec![1, 8], vec![1; 9]] {
            request.round_ranks = ranks;
            assert!(request.payload_bytes().is_err());
        }
        request.round_ranks = vec![1, 0];
        request.command = CommandV1::Close;
        assert!(request.payload_bytes().is_err());
    }

    #[test]
    fn concurrent_round_completion_is_all_or_terminal() {
        let response = |ranks, elapsed_ns| Response::RoundDone {
            request: 1,
            ranks,
            response: ResponseV1::DispatchSequenceCompleted { elapsed_ns },
        };
        assert_eq!(response(vec![1, 0], vec![5, 6]).payload_bytes().unwrap(), 0);
        for invalid in [
            response(vec![1, 0], vec![5]),
            response(vec![1, 1], vec![5, 6]),
            response(vec![], vec![]),
            response(vec![8], vec![5]),
        ] {
            assert!(invalid.payload_bytes().is_err());
        }
        let failed = Response::RoundDone {
            request: 1,
            ranks: vec![0, 1],
            response: ResponseV1::Error {
                message: "uncertain".into(),
                fatal: true,
            },
        };
        assert_eq!(failed.payload_bytes().unwrap(), 0);
    }

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
            round_ranks: vec![],
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
            round_ranks: vec![],
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
            round_ranks: vec![],
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
