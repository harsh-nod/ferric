//! Host-owned descriptions of the opt-in, whole-token TP2 interpreter.
//! Recording these values neither publishes packets nor completes GPU work.

use super::{
    EngineeringTp2CollectiveReceiptV1, EngineeringTp2CollectiveRequestV1, EngineeringTpDispatchV1,
    TpResult,
};

/// One source-ordered step of the immutable single-row target program.
#[derive(Clone, Debug, PartialEq)]
pub enum EngineeringTp2PreparedStepV1 {
    /// One rank-local kernel, including its typed allocation identifiers.
    Rank {
        /// Logical rank, zero or one.
        rank: u32,
        /// Original admitted dispatch with no process-local addresses.
        dispatch: EngineeringTpDispatchV1,
    },
    /// One complete producer/barrier/consumer transaction on both ranks.
    Collective(EngineeringTp2CollectiveRequestV1),
}

/// Fixed rank-local destinations for the four per-token metadata uploads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineeringTp2PreparedMetadataV1 {
    /// Position buffer, capacity sixteen u32 elements.
    pub positions: u64,
    /// Four page identifiers per row, capacity sixteen rows.
    pub page_table: u64,
    /// FP32 rotary cosines, capacity sixteen by64.
    pub cos: u64,
    /// FP32 rotary sines, capacity sixteen by64.
    pub sin: u64,
}

/// The exact baseline TP2 graph, recorded without changing live model state.
#[derive(Clone, Debug, PartialEq)]
pub struct EngineeringTp2PreparedProgramV1 {
    /// Existing collective group identity; it is not a new native capability.
    pub group_id: u64,
    /// Rank-zero input token allocation.
    pub token_buffer: u64,
    /// Rank-zero output choice allocation.
    pub result_buffer: u64,
    /// Role-bound metadata allocations in rank order.
    pub metadata: [EngineeringTp2PreparedMetadataV1; 2],
    /// Exactly1013 steps, expanding to616/613 kernels and72 barriers per rank.
    pub steps: Vec<EngineeringTp2PreparedStepV1>,
}

/// Dynamic token data; no caller-selected write addresses or offsets are accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineeringTp2PreparedInputV1 {
    /// Hash returned by registration of the complete immutable program.
    pub plan_sha256: [u8; 32],
    /// One-based whole-token generation.
    pub generation: u64,
    /// Zero-based source collective epoch.
    pub epoch: u64,
    /// Input token, below151936.
    pub token: u32,
    /// Position, below64, equal to the epoch in this single-request profile.
    pub position: u32,
    /// Active physical pages followed by `u32::MAX` entries.
    pub page_table: [u32; 4],
    /// Exactly256 cosine bytes followed by256 sine bytes, both FP32.
    pub cos_sin: Vec<u8>,
}

/// Whole-token completion; transport identity validation precedes its creation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineeringTp2PreparedReceiptV1 {
    /// Echo of the registered immutable program identity.
    pub plan_sha256: [u8; 32],
    /// Echo of the one-based execution generation.
    pub generation: u64,
    /// Echo of the source collective epoch.
    pub epoch: u64,
    /// Echo of the input position.
    pub position: u32,
    /// Echo of the input token, distinct from the decoded choice.
    pub input_token: u32,
    /// Actual rank-zero GPU argmax readback.
    pub output_token: u32,
    /// Completed kernel counts, excluding barriers.
    pub kernel_counts: [u32; 2],
    /// Completed Barrier-AND counts.
    pub barrier_counts: [u32; 2],
    /// Completed packet accounting, not a fabricated native frontier.
    pub packet_counts: [u32; 2],
    /// Exactly72 actual native collective receipts in source order.
    pub collectives: Vec<EngineeringTp2CollectiveReceiptV1>,
}

impl EngineeringTp2PreparedReceiptV1 {
    /// Checks every driver-visible field before the all-or-terminal host commit.
    /// Physical device/worker identity and native request IDs are transport duties.
    /// # Errors
    /// Rejects stale identities, incomplete work or incorrect queue progression.
    pub fn validate_for(
        &self,
        program: &EngineeringTp2PreparedProgramV1,
        input: &EngineeringTp2PreparedInputV1,
    ) -> TpResult<()> {
        if input.epoch.checked_add(1) != Some(input.generation)
            || u64::from(input.position) != input.epoch
            || input.position >= 64
            || input.token >= 151_936
            || input.cos_sin.len() != 512
            || self.plan_sha256 != input.plan_sha256
            || self.generation != input.generation
            || self.epoch != input.epoch
            || self.position != input.position
            || self.input_token != input.token
            || self.output_token >= 151_936
            || self.kernel_counts != [616, 613]
            || self.barrier_counts != [72; 2]
            || self.packet_counts != [688, 685]
            || self.collectives.len() != 72
            || program.steps.len() != 1013
        {
            return Err("prepared TP2 completion identity or cardinality mismatch".into());
        }
        let mut frontier = [
            input
                .epoch
                .checked_mul(688)
                .ok_or("prepared rank-zero cursor overflow")?,
            input
                .epoch
                .checked_mul(685)
                .ok_or("prepared rank-one cursor overflow")?,
        ];
        let mut collective = 0usize;
        for step in &program.steps {
            match step {
                EngineeringTp2PreparedStepV1::Rank { rank, .. } => {
                    let cursor = frontier
                        .get_mut(*rank as usize)
                        .ok_or("prepared rank outside TP2")?;
                    *cursor = cursor
                        .checked_add(1)
                        .ok_or("prepared ordinary cursor overflow")?;
                }
                EngineeringTp2PreparedStepV1::Collective(template) => {
                    let mut request = template.clone();
                    request.key.epoch = input.epoch;
                    let actual = self
                        .collectives
                        .get(collective)
                        .ok_or("prepared missing collective")?;
                    actual.validate_for(&request)?;
                    let generation = input
                        .epoch
                        .checked_mul(72)
                        .and_then(|value| value.checked_add(collective as u64 + 1))
                        .ok_or("prepared collective generation overflow")?;
                    if actual.generation != generation || actual.first_packet_ids != frontier {
                        return Err("prepared actual collective queue progression mismatch".into());
                    }
                    for cursor in &mut frontier {
                        *cursor = cursor
                            .checked_add(3)
                            .ok_or("prepared collective cursor overflow")?;
                    }
                    collective += 1;
                }
            }
        }
        let completed_epoch = input
            .epoch
            .checked_add(1)
            .ok_or("prepared epoch overflow")?;
        let expected = [
            completed_epoch.checked_mul(688),
            completed_epoch.checked_mul(685),
        ];
        if collective != 72 || expected != frontier.map(Some) {
            return Err("prepared incomplete collective grammar".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tp_execution::{
        EngineeringTpArgumentV1 as Arg, EngineeringTpBufferAccessV1 as Access,
    };
    use ferric_engine::tensor_parallel::{
        Qwen3TensorParallelCollectiveKeyV1, Qwen3TensorParallelCollectiveV1 as Op,
    };
    use ferric_spec::Qwen3ModelRole;

    fn slice(id: u64, elements: usize, width: u32, access: Access) -> Arg {
        Arg::Buffer {
            id,
            offset: 0,
            elements,
            element_bytes: width,
            access,
        }
    }

    fn collective(layer: u32, operation: Op) -> EngineeringTp2CollectiveRequestV1 {
        let (k, tag) = if operation == Op::AttentionOutputSum {
            (2048, 1)
        } else {
            (6144, 2)
        };
        EngineeringTp2CollectiveRequestV1 {
            key: Qwen3TensorParallelCollectiveKeyV1 {
                group_id: 7,
                model_role: Qwen3ModelRole::Target8B,
                epoch: 0,
                layer,
                operation,
            },
            rows: 1,
            producers: std::array::from_fn(|rank| EngineeringTpDispatchV1 {
                kernel: "ferric_qwen3_tp_mfma_gemm_partial_f32_v3",
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
                    kernel: "ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18",
                    grid_workgroups: 64,
                    workgroup_size: 64,
                    arguments,
                }
            }),
        }
    }

    // These ordinary placeholders test receipt accounting only, never native admission.
    fn ordinary(rank: u32) -> EngineeringTp2PreparedStepV1 {
        EngineeringTp2PreparedStepV1::Rank {
            rank,
            dispatch: EngineeringTpDispatchV1 {
                kernel: "cpu-receipt-fixture-only",
                grid_workgroups: 1,
                workgroup_size: 64,
                arguments: Vec::new(),
            },
        }
    }

    fn fixture(
        epoch: u64,
    ) -> (
        EngineeringTp2PreparedProgramV1,
        EngineeringTp2PreparedInputV1,
        EngineeringTp2PreparedReceiptV1,
    ) {
        let mut steps = vec![ordinary(0), ordinary(1)];
        for layer in 0..36 {
            for _ in 0..9 {
                steps.extend([ordinary(0), ordinary(1)]);
            }
            steps.push(EngineeringTp2PreparedStepV1::Collective(collective(
                layer,
                Op::AttentionOutputSum,
            )));
            for _ in 0..4 {
                steps.extend([ordinary(0), ordinary(1)]);
            }
            steps.push(EngineeringTp2PreparedStepV1::Collective(collective(
                layer,
                Op::FeedForwardDownSum,
            )));
        }
        steps.extend([ordinary(0), ordinary(0), ordinary(0)]);
        let program = EngineeringTp2PreparedProgramV1 {
            group_id: 7,
            token_buffer: 20,
            result_buffer: 21,
            metadata: [EngineeringTp2PreparedMetadataV1 {
                positions: 22,
                page_table: 23,
                cos: 24,
                sin: 25,
            }; 2],
            steps,
        };
        let input = EngineeringTp2PreparedInputV1 {
            plan_sha256: [9; 32],
            generation: epoch + 1,
            epoch,
            token: 42,
            position: epoch as u32,
            page_table: [0, u32::MAX, u32::MAX, u32::MAX],
            cos_sin: vec![0; 512],
        };
        let mut cursor = [epoch * 688, epoch * 685];
        let mut collectives = Vec::new();
        for step in &program.steps {
            match step {
                EngineeringTp2PreparedStepV1::Rank { rank, .. } => cursor[*rank as usize] += 1,
                EngineeringTp2PreparedStepV1::Collective(request) => {
                    let mut key = request.key;
                    key.epoch = epoch;
                    collectives.push(EngineeringTp2CollectiveReceiptV1 {
                        key,
                        generation: epoch * 72 + collectives.len() as u64 + 1,
                        kernel_dispatches: [2; 2],
                        barrier_packets: [1; 2],
                        packet_counts: [3; 2],
                        completion_values: [0; 6],
                        queue_epochs: [0; 2],
                        first_packet_ids: cursor,
                        frontiers: [[cursor[0] + 3; 2], [cursor[1] + 3; 2]],
                    });
                    for value in &mut cursor {
                        *value += 3;
                    }
                }
            }
        }
        let receipt = EngineeringTp2PreparedReceiptV1 {
            plan_sha256: input.plan_sha256,
            generation: input.generation,
            epoch,
            position: input.position,
            input_token: 42,
            output_token: 12095,
            kernel_counts: [616, 613],
            barrier_counts: [72; 2],
            packet_counts: [688, 685],
            collectives,
        };
        (program, input, receipt)
    }

    #[test]
    fn prepared_receipts_bind_two_tokens_and_last_context_position() {
        for epoch in [0, 1, 35, 63] {
            let (program, input, receipt) = fixture(epoch);
            receipt.validate_for(&program, &input).unwrap();
        }
    }

    #[test]
    fn prepared_receipts_reject_every_incomplete_collective() {
        let (program, input, original) = fixture(1);
        for index in 0..72 {
            for rank in 0..2 {
                let mut receipt = original.clone();
                receipt.collectives[index].frontiers[rank][1] -= 1;
                assert!(receipt.validate_for(&program, &input).is_err());
                let mut receipt = original.clone();
                receipt.collectives[index].first_packet_ids[rank] += 1;
                receipt.collectives[index].frontiers[rank] =
                    [receipt.collectives[index].first_packet_ids[rank] + 3; 2];
                assert!(receipt.validate_for(&program, &input).is_err());
            }
        }
    }

    #[test]
    fn prepared_receipts_reject_identity_counts_output_and_truncation() {
        let (program, input, original) = fixture(0);
        let mutations: [fn(&mut EngineeringTp2PreparedReceiptV1); 12] = [
            |r| r.plan_sha256[0] ^= 1,
            |r| r.generation += 1,
            |r| r.epoch += 1,
            |r| r.position += 1,
            |r| r.input_token += 1,
            |r| r.output_token = 151_936,
            |r| r.kernel_counts[0] -= 1,
            |r| r.barrier_counts[1] -= 1,
            |r| r.packet_counts[0] -= 1,
            |r| {
                r.collectives.pop();
            },
            |r| r.collectives[0].generation += 1,
            |r| r.collectives[71].key.layer = 0,
        ];
        for mutate in mutations {
            let mut receipt = original.clone();
            mutate(&mut receipt);
            assert!(receipt.validate_for(&program, &input).is_err());
        }
    }

    #[test]
    fn prepared_receipts_reject_invalid_input_and_final_rank_accounting() {
        let (mut program, mut input, receipt) = fixture(0);
        input.cos_sin.pop();
        assert!(receipt.validate_for(&program, &input).is_err());
        input.cos_sin.push(0);
        *program.steps.last_mut().unwrap() = ordinary(1);
        assert!(receipt.validate_for(&program, &input).is_err());
        let (program, mut input, receipt) = fixture(0);
        input.epoch = u64::MAX;
        assert!(receipt.validate_for(&program, &input).is_err());
    }
}
