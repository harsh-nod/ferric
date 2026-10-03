//! Closed logical contract for one all-or-terminal TP2 dependency collective.

use super::{
    EngineeringTpArgumentV1 as Arg, EngineeringTpBufferAccessV1 as Access, EngineeringTpDispatchV1,
    TpResult,
};
use ferric_engine::tensor_parallel::{
    Qwen3TensorParallelCollectiveKeyV1, Qwen3TensorParallelCollectiveV1,
};
use ferric_spec::Qwen3ModelRole;
use std::collections::BTreeSet;

const BASELINE: &str = "ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2";
const WAVE: &str = "ferric_qwen3_tp_wave_gemv_partial_f32_v3";
const MFMA: &str = "ferric_qwen3_tp_mfma_gemm_partial_f32_v3";
pub(super) const CONSUMER: &str = "ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18";

/// Two partial producers and two unchanged ordered residual consumers.
/// Array indices are canonical logical ranks, never publication order.
#[derive(Clone, Debug, PartialEq)]
pub struct EngineeringTp2CollectiveRequestV1 {
    /// Exact driver collective identity; the transport must echo it unchanged.
    pub key: Qwen3TensorParallelCollectiveKeyV1,
    /// The initial closed profile admits exactly one active row.
    pub rows: u32,
    /// Rank-local partial projections, retained without prior submission.
    pub producers: [EngineeringTpDispatchV1; 2],
    /// Rank-local consumers reading both rank-ordered partials.
    pub consumers: [EngineeringTpDispatchV1; 2],
}

fn buffer(argument: &Arg, elements: usize, bytes: u32, access: Access) -> TpResult<u64> {
    match argument {
        Arg::Buffer {
            id,
            offset: 0,
            elements: actual,
            element_bytes,
            access: actual_access,
        } if *id != 0
            && *actual == elements
            && *element_bytes == bytes
            && *actual_access == access =>
        {
            Ok(*id)
        }
        _ => Err("TP2 dependency buffer shape or access mismatch".into()),
    }
}

impl EngineeringTp2CollectiveRequestV1 {
    /// Checks logical graph/ABI scope; loaded image and physical ownership remain
    /// transport obligations. Recognition is not native profile admission.
    /// # Errors
    /// Rejects another model, shape, root, rank order, alias or explicit ABI.
    pub fn validate(&self) -> TpResult<()> {
        if self.rows != 1 || self.key.model_role != Qwen3ModelRole::Target8B || self.key.layer >= 36
        {
            return Err(
                "TP2 dependency collective requires Target8B, one row and a valid layer".into(),
            );
        }
        let (k, tag) = match self.key.operation {
            Qwen3TensorParallelCollectiveV1::AttentionOutputSum => (2048, 1),
            Qwen3TensorParallelCollectiveV1::FeedForwardDownSum => (6144, 2),
        };
        let root = self.producers[0].kernel;
        if ![BASELINE, WAVE, MFMA].contains(&root) {
            return Err("TP2 dependency producer root is unsupported".into());
        }
        let mut ids = BTreeSet::new();
        let mut partials = [0; 2];
        for (rank, producer) in self.producers.iter().enumerate() {
            if producer.kernel != root
                || producer.workgroup_size != 64
                || producer.grid_workgroups != if root == WAVE { 4096 } else { 256 }
                || producer.arguments.len() != 8
                || producer.arguments[3..]
                    != [
                        Arg::U32(1),
                        Arg::U32(4096),
                        Arg::U32(k),
                        Arg::U32(2),
                        Arg::U32(tag),
                    ]
            {
                return Err("TP2 dependency producer geometry or scalars mismatch".into());
            }
            let input = buffer(&producer.arguments[0], 16 * k as usize, 2, Access::Read)?;
            let weight = buffer(&producer.arguments[1], 4096 * k as usize, 2, Access::Read)?;
            let partial = buffer(&producer.arguments[2], 16 * 4096, 4, Access::Write)?;
            for id in [input, weight, partial] {
                if !ids.insert(id) {
                    return Err("TP2 dependency producer buffer alias".into());
                }
            }
            partials[rank] = partial;
        }
        for consumer in &self.consumers {
            if consumer.kernel != CONSUMER
                || consumer.workgroup_size != 64
                || consumer.grid_workgroups != 64
                || consumer.arguments.len() != 12
                || consumer.arguments[10..] != [Arg::U32(1), Arg::U32(2)]
            {
                return Err("TP2 dependency consumer geometry or scalars mismatch".into());
            }
            for (slot, argument) in consumer.arguments[..8].iter().enumerate() {
                let elements = if slot < 2 { 4096 } else { 0 };
                let expected = partials[if slot < 2 { slot } else { 0 }];
                if buffer(argument, elements, 4, Access::Read)? != expected {
                    return Err("TP2 dependency partial rank order or filler mismatch".into());
                }
            }
            let hidden = buffer(&consumer.arguments[8], 4096, 2, Access::Read)?;
            let scratch = buffer(&consumer.arguments[9], 4096, 2, Access::Write)?;
            if !ids.insert(hidden) || !ids.insert(scratch) {
                return Err("TP2 dependency hidden or scratch alias".into());
            }
        }
        Ok(())
    }
}

/// Successful whole-collective completion, with no per-kernel time claim.
/// Public fields permit recording transports to exercise malformed receipts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineeringTp2CollectiveReceiptV1 {
    /// Echoed driver key.
    pub key: Qwen3TensorParallelCollectiveKeyV1,
    /// Transport generation, starting at one and strictly increasing.
    pub generation: u64,
    /// Exactly two kernels per rank; barrier packets are not kernels.
    pub kernel_dispatches: [u32; 2],
    /// Exactly one Barrier-AND packet per rank.
    pub barrier_packets: [u32; 2],
    /// Exactly three packets per rank.
    pub packet_counts: [u32; 2],
    /// Acquired zero values in rank-major P0/B0/C0/P1/B1/C1 order.
    pub completion_values: [i64; 6],
    /// No rollover is admitted, so both epochs are zero.
    pub queue_epochs: [u64; 2],
    /// Actual per-rank reservation starts, allowing prior serial work.
    pub first_packet_ids: [u64; 2],
    /// Final [write, read] frontiers, each exactly first + three.
    pub frontiers: [[u64; 2]; 2],
}

impl EngineeringTp2CollectiveReceiptV1 {
    /// Validates every field before a driver commits collective state.
    /// Generation/current child/device identity binding remains a transport duty.
    /// # Errors
    /// Rejects wrong identity, partial completion, rollover or frontier overflow.
    pub fn validate_for(&self, request: &EngineeringTp2CollectiveRequestV1) -> TpResult<()> {
        request.validate()?;
        if self.key != request.key
            || self.generation == 0
            || self.kernel_dispatches != [2; 2]
            || self.barrier_packets != [1; 2]
            || self.packet_counts != [3; 2]
            || self.completion_values != [0; 6]
            || self.queue_epochs != [0; 2]
        {
            return Err("TP2 dependency collective completion identity or counts mismatch".into());
        }
        for rank in 0..2 {
            let next = self.first_packet_ids[rank]
                .checked_add(3)
                .ok_or("TP2 dependency packet frontier overflow")?;
            if self.frontiers[rank] != [next; 2] {
                return Err("TP2 dependency collective did not fully drain".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slice(id: u64, elements: usize, element_bytes: u32, access: Access) -> Arg {
        Arg::Buffer {
            id,
            offset: 0,
            elements,
            element_bytes,
            access,
        }
    }

    fn request(operation: Qwen3TensorParallelCollectiveV1) -> EngineeringTp2CollectiveRequestV1 {
        let (k, tag) = match operation {
            Qwen3TensorParallelCollectiveV1::AttentionOutputSum => (2048, 1),
            Qwen3TensorParallelCollectiveV1::FeedForwardDownSum => (6144, 2),
        };
        EngineeringTp2CollectiveRequestV1 {
            key: Qwen3TensorParallelCollectiveKeyV1 {
                group_id: 0,
                model_role: Qwen3ModelRole::Target8B,
                epoch: 0,
                layer: 0,
                operation,
            },
            rows: 1,
            producers: std::array::from_fn(|rank| EngineeringTpDispatchV1 {
                kernel: MFMA,
                grid_workgroups: 256,
                workgroup_size: 64,
                arguments: vec![
                    slice(1 + rank as u64 * 3, 16 * k, 2, Access::Read),
                    slice(2 + rank as u64 * 3, 4096 * k, 2, Access::Read),
                    slice(3 + rank as u64 * 3, 16 * 4096, 4, Access::Write),
                    Arg::U32(1),
                    Arg::U32(4096),
                    Arg::U32(k as u32),
                    Arg::U32(2),
                    Arg::U32(tag),
                ],
            }),
            consumers: std::array::from_fn(|rank| {
                let mut arguments = vec![
                    slice(3, 4096, 4, Access::Read),
                    slice(6, 4096, 4, Access::Read),
                ];
                arguments.extend([slice(3, 0, 4, Access::Read); 6]);
                arguments.extend([
                    slice(7 + rank as u64 * 2, 4096, 2, Access::Read),
                    slice(8 + rank as u64 * 2, 4096, 2, Access::Write),
                    Arg::U32(1),
                    Arg::U32(2),
                ]);
                EngineeringTpDispatchV1 {
                    kernel: CONSUMER,
                    grid_workgroups: 64,
                    workgroup_size: 64,
                    arguments,
                }
            }),
        }
    }

    #[test]
    fn closed_shapes_and_three_logical_roots() {
        for operation in [
            Qwen3TensorParallelCollectiveV1::AttentionOutputSum,
            Qwen3TensorParallelCollectiveV1::FeedForwardDownSum,
        ] {
            for root in [BASELINE, WAVE, MFMA] {
                let mut request = request(operation);
                for producer in &mut request.producers {
                    producer.kernel = root;
                    producer.grid_workgroups = if root == WAVE { 4096 } else { 256 };
                }
                request.validate().unwrap();
            }
        }
    }

    #[test]
    fn rejects_shape_order_alias_and_extent_drift() {
        let original = request(Qwen3TensorParallelCollectiveV1::AttentionOutputSum);
        let mutations: [fn(&mut EngineeringTp2CollectiveRequestV1); 10] = [
            |r| r.rows = 2,
            |r| r.key.layer = 36,
            |r| r.producers[1].kernel = BASELINE,
            |r| r.producers[0].arguments[5] = Arg::U32(6144),
            |r| r.producers[0].arguments[0] = slice(1, 2048, 2, Access::Read),
            |r| r.consumers[0].arguments.swap(0, 1),
            |r| r.consumers[1].arguments[2] = slice(6, 0, 4, Access::Read),
            |r| r.consumers[1].arguments[9] = slice(3, 4096, 2, Access::Write),
            |r| r.consumers[1].grid_workgroups = 1,
            |r| r.consumers[0].kernel = "ferric_qwen3_tp_peer_ordered_residual_bf16_v4",
        ];
        for mutate in mutations {
            let mut changed = original.clone();
            mutate(&mut changed);
            assert!(changed.validate().is_err());
        }
    }

    #[test]
    fn receipt_requires_entire_graph_and_checked_frontiers() {
        let request = request(Qwen3TensorParallelCollectiveV1::AttentionOutputSum);
        let receipt = EngineeringTp2CollectiveReceiptV1 {
            key: request.key,
            generation: 1,
            kernel_dispatches: [2; 2],
            barrier_packets: [1; 2],
            packet_counts: [3; 2],
            completion_values: [0; 6],
            queue_epochs: [0; 2],
            first_packet_ids: [7, 11],
            frontiers: [[10; 2], [14; 2]],
        };
        receipt.validate_for(&request).unwrap();
        let mutations: [fn(&mut EngineeringTp2CollectiveReceiptV1); 8] = [
            |r| r.key.epoch += 1,
            |r| r.generation = 0,
            |r| r.kernel_dispatches[1] = 1,
            |r| r.barrier_packets[0] = 0,
            |r| r.packet_counts[0] = 2,
            |r| r.queue_epochs[0] = 1,
            |r| r.first_packet_ids[1] = u64::MAX,
            |r| r.frontiers[1][1] -= 1,
        ];
        for mutate in mutations {
            let mut changed = receipt.clone();
            mutate(&mut changed);
            assert!(changed.validate_for(&request).is_err());
        }
        for slot in 0..6 {
            let mut changed = receipt.clone();
            changed.completion_values[slot] = 1;
            assert!(changed.validate_for(&request).is_err());
        }
    }
}
