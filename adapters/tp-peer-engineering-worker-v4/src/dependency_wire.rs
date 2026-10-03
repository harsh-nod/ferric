//! Separate default-off TP2 envelope. Legacy rank framing is not reinterpreted.

use super::wire as legacy;
use fe2o3_kfd::engineering_wire::{self as base, CommandV1, SequenceDispatchV1};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 1;
pub const MODE: &str = "device-peer-tp2-collective-dependency-v1";
pub const PRODUCER_ROOT: &str = "ferric_qwen3_tp_mfma_gemm_partial_f32_v3";
pub const CONSUMER_ROOT: &str = "ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18";
pub const TIMEOUT_MS: u32 = 2000;
pub const CONTROL_ALLOCATION_FLAGS: u32 = 0x8600_0002;

pub fn is_collective_root(root: &str) -> bool {
    matches!(
        root,
        "ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2"
            | "ferric_qwen3_tp_wave_gemv_partial_f32_v3"
            | "ferric_qwen3_tp_mfma_gemm_partial_f32_v3"
            | "ferric_qwen3_tp_peer_ordered_residual_bf16_v4"
            | "ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18"
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRole {
    Target8b,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    AttentionOutputSum,
    FeedForwardDownSum,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub request_id: u64,
    pub generation: u64,
    pub group_id: u64,
    pub model_role: ModelRole,
    pub epoch: u64,
    pub layer: u32,
    pub operation: Operation,
}

impl Identity {
    pub fn validate(&self) -> io::Result<()> {
        if self.request_id == 0 || self.generation == 0 || self.layer >= 36 {
            return Err(io::Error::other("TP2 dependency identity scope"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Collective {
    pub identity: Identity,
    pub producers: [SequenceDispatchV1; 2],
    pub consumers: [SequenceDispatchV1; 2],
}

impl Collective {
    pub fn payload_bytes(&self) -> io::Result<usize> {
        self.identity.validate()?;
        let dispatches: Vec<_> = self
            .producers
            .iter()
            .chain(&self.consumers)
            .cloned()
            .collect();
        for (index, dispatch) in dispatches.iter().enumerate() {
            if dispatch.kernel == 0
                || dispatch.payload_bytes == 0
                || dispatch.timeout_ms != TIMEOUT_MS
                || dispatch.workgroup != [64, 1, 1]
                || dispatch.grid
                    != if index < 2 {
                        [16384, 1, 1]
                    } else {
                        [4096, 1, 1]
                    }
            {
                return Err(io::Error::other(
                    "TP2 dependency kernel identity, payload, geometry or timeout",
                ));
            }
        }
        CommandV1::DispatchSequence { dispatches }.payload_bytes()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RankQueue {
    pub unique_id: u64,
    pub queue_epoch: u64,
    pub first_packet: u64,
    pub next_packet: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub identity: Identity,
    pub queues: [RankQueue; 2],
    pub kernel_counts: [u32; 2],
    pub barrier_counts: [u32; 2],
    pub packet_counts: [u32; 2],
    pub completion_values: [[i64; 3]; 2],
    pub final_frontiers: [[u64; 2]; 2],
}

impl Receipt {
    pub fn validate(&self) -> io::Result<()> {
        self.identity.validate()?;
        if self.kernel_counts != [2; 2]
            || self.barrier_counts != [1; 2]
            || self.packet_counts != [3; 2]
            || self.completion_values != [[0; 3]; 2]
            || self.queues[0].unique_id == self.queues[1].unique_id
        {
            return Err(io::Error::other("TP2 dependency incomplete receipt"));
        }
        for (rank, queue) in self.queues.iter().enumerate() {
            if queue.unique_id == 0
                || queue.queue_epoch != 0
                || queue.first_packet.checked_add(3) != Some(queue.next_packet)
                || self.final_frontiers[rank] != [queue.next_packet; 2]
            {
                return Err(io::Error::other("TP2 dependency queue did not drain"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "dependency_op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Rank { request: legacy::Request },
    Collective { collective: Collective },
}

impl Request {
    pub fn payload_bytes(&self) -> io::Result<usize> {
        match self {
            Self::Collective { collective } => collective.payload_bytes(),
            Self::Rank { request } => {
                if request.rank >= 2
                    || !request.round_ranks.is_empty()
                    || !matches!(
                        request.command,
                        CommandV1::LoadKernel { .. }
                            | CommandV1::Allocate { .. }
                            | CommandV1::Write { .. }
                            | CommandV1::Read { .. }
                            | CommandV1::Dispatch { .. }
                            | CommandV1::Close
                            | CommandV1::ConfigurePerformance {
                                operational_currentness: false,
                                profile: false,
                                ..
                            }
                    )
                {
                    return Err(io::Error::other("TP2 dependency ordinary command scope"));
                }
                request.payload_bytes()
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "dependency_op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    Ready {
        protocol: u32,
        mode: String,
        target: String,
        unique_ids: [u64; 2],
        process_id: u32,
        authority: String,
        producer_root: String,
        consumer_root: String,
        currentness: String,
        control_allocation_flags: u32,
        collective_timeout_ms: u32,
    },
    Rank {
        response: legacy::Response,
    },
    CollectiveCompleted {
        receipt: Receipt,
    },
    Fatal {
        request_id: u64,
        message: String,
    },
}

impl Response {
    pub fn payload_bytes(&self) -> io::Result<usize> {
        match self {
            Self::Rank { response } => {
                if !matches!(response, legacy::Response::Done { request, rank, .. } if *request>0 && *rank<2)
                {
                    return Err(io::Error::other("TP2 dependency ordinary receipt scope"));
                }
                response.payload_bytes()
            }
            Self::CollectiveCompleted { receipt } => {
                receipt.validate()?;
                Ok(0)
            }
            Self::Ready { .. } | Self::Fatal { .. } => Ok(0),
        }
    }
}

pub fn read_request(input: &mut impl Read) -> io::Result<Option<(Request, Vec<u8>)>> {
    let Some(header) = base::read_header_v1::<Request>(input)? else {
        return Ok(None);
    };
    let mut bytes = vec![0; header.payload_bytes()?];
    input.read_exact(&mut bytes)?;
    Ok(Some((header, bytes)))
}
pub fn read_response(input: &mut impl Read) -> io::Result<(Response, Vec<u8>)> {
    let header = base::read_header_v1::<Response>(input)?
        .ok_or_else(|| io::Error::other("TP2 dependency response pipe closed"))?;
    let mut bytes = vec![0; header.payload_bytes()?];
    input.read_exact(&mut bytes)?;
    Ok((header, bytes))
}
pub fn write_request(output: &mut impl Write, header: &Request, bytes: &[u8]) -> io::Result<()> {
    if header.payload_bytes()? != bytes.len() {
        return Err(io::Error::other("TP2 dependency request payload mismatch"));
    }
    base::write_header_v1(output, header)?;
    output.write_all(bytes)?;
    output.flush()
}
pub fn write_response(output: &mut impl Write, header: &Response, bytes: &[u8]) -> io::Result<()> {
    if header.payload_bytes()? != bytes.len() {
        return Err(io::Error::other("TP2 dependency response payload mismatch"));
    }
    base::write_header_v1(output, header)?;
    output.write_all(bytes)?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity() -> Identity {
        Identity {
            request_id: 1,
            generation: 1,
            group_id: 0,
            model_role: ModelRole::Target8b,
            epoch: 0,
            layer: 0,
            operation: Operation::AttentionOutputSum,
        }
    }
    fn collective() -> Collective {
        let dispatch = |grid| SequenceDispatchV1 {
            kernel: 1,
            payload_bytes: 4,
            workgroup: [64, 1, 1],
            grid: [grid, 1, 1],
            pointers: vec![],
            timeout_ms: 2000,
        };
        Collective {
            identity: identity(),
            producers: [dispatch(16384), dispatch(16384)],
            consumers: [dispatch(4096), dispatch(4096)],
        }
    }
    #[test]
    fn explicit_envelope_roundtrips_and_legacy_does_not_parse() {
        let header = Request::Collective {
            collective: collective(),
        };
        let mut bytes = vec![];
        write_request(&mut bytes, &header, &[0; 16]).unwrap();
        let (decoded, payload) = read_request(&mut &bytes[..]).unwrap().unwrap();
        assert_eq!(decoded.payload_bytes().unwrap(), 16);
        assert_eq!(payload, [0; 16]);
        assert!(legacy::read_request(&mut &bytes[..]).is_err());
        let legacy = legacy::Request {
            id: 1,
            rank: 0,
            peer_readable: false,
            round_ranks: vec![],
            command: CommandV1::Close,
        };
        let mut bytes = vec![];
        legacy::write_request(&mut bytes, &legacy, &[]).unwrap();
        assert!(read_request(&mut &bytes[..]).is_err());
    }
    #[test]
    fn fixed_shapes_timeout_identity_and_payload_bounds() {
        let original = collective();
        assert_eq!(original.payload_bytes().unwrap(), 16);
        let changes: [fn(&mut Collective); 8] = [
            |c| c.identity.generation = 0,
            |c| c.identity.request_id = 0,
            |c| c.identity.layer = 36,
            |c| c.producers[0].timeout_ms = 1999,
            |c| c.consumers[1].grid[0] = 64,
            |c| c.producers[1].kernel = 0,
            |c| c.consumers[0].payload_bytes = 0,
            |c| c.producers[1].payload_bytes = u32::MAX,
        ];
        for (index, change) in changes.into_iter().enumerate() {
            let mut c = original.clone();
            change(&mut c);
            assert!(c.payload_bytes().is_err(), "mutation {index}");
        }
        let mut value = serde_json::to_value(original).unwrap();
        value["identity"]["model_role"] = "draft".into();
        assert!(serde_json::from_value::<Collective>(value).is_err());
    }
    #[test]
    fn every_collective_kernel_id_is_nonzero_and_layer35_is_admitted() {
        let mut valid = collective();
        valid.identity.layer = 35;
        assert_eq!(valid.payload_bytes().unwrap(), 16);
        for slot in 0..4 {
            let mut invalid = valid.clone();
            if slot < 2 {
                invalid.producers[slot].kernel = 0;
            } else {
                invalid.consumers[slot - 2].kernel = 0;
            }
            assert!(invalid.payload_bytes().is_err(), "kernel slot {slot}");
            let header = Request::Collective {
                collective: invalid,
            };
            assert!(write_request(&mut Vec::new(), &header, &[0; 16]).is_err());
        }
        valid.identity.layer = 36;
        assert!(valid.payload_bytes().is_err());
    }
    #[test]
    fn rank_wrappers_preserve_bytes_and_reject_other_modes() {
        let mut rank = legacy::Request {
            id: 1,
            rank: 0,
            peer_readable: false,
            round_ranks: vec![],
            command: CommandV1::Close,
        };
        assert_eq!(
            Request::Rank {
                request: rank.clone()
            }
            .payload_bytes()
            .unwrap(),
            0
        );
        rank.rank = 2;
        assert!(
            Request::Rank {
                request: rank.clone()
            }
            .payload_bytes()
            .is_err()
        );
        rank.rank = 0;
        rank.command = CommandV1::ConfigurePerformance {
            cache_kernel_admission: true,
            operational_currentness: true,
            profile: false,
        };
        assert!(Request::Rank { request: rank }.payload_bytes().is_err());
    }
    #[test]
    fn success_is_six_zero_signals_and_both_drained_queues() {
        let original = Receipt {
            identity: identity(),
            queues: [
                RankQueue {
                    unique_id: 1,
                    queue_epoch: 0,
                    first_packet: 7,
                    next_packet: 10,
                },
                RankQueue {
                    unique_id: 2,
                    queue_epoch: 0,
                    first_packet: 11,
                    next_packet: 14,
                },
            ],
            kernel_counts: [2; 2],
            barrier_counts: [1; 2],
            packet_counts: [3; 2],
            completion_values: [[0; 3]; 2],
            final_frontiers: [[10; 2], [14; 2]],
        };
        original.validate().unwrap();
        for rank in 0..2 {
            for slot in 0..3 {
                let mut changed = original.clone();
                changed.completion_values[rank][slot] = 1;
                assert!(changed.validate().is_err());
            }
        }
        let changes: [fn(&mut Receipt); 6] = [
            |r| r.queues[0].queue_epoch = 1,
            |r| r.queues[1].unique_id = 1,
            |r| r.queues[0].first_packet = u64::MAX,
            |r| r.final_frontiers[1][1] -= 1,
            |r| r.kernel_counts[0] = 1,
            |r| r.barrier_counts[1] = 0,
        ];
        for change in changes {
            let mut r = original.clone();
            change(&mut r);
            assert!(r.validate().is_err());
        }
    }
}
