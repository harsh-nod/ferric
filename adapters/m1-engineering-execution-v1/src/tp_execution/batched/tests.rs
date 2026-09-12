//! Runs the actual batched host driver against a recording byte transport.
//! Synthetic weights and outputs are not GPU kernel emulation or numerical
//! evidence. These tests exercise scheduling, active extents, ownership handoff,
//! host reductions, and terminal failure behavior without a physical device.

mod argmax_v11;
mod draft;
mod large_kv;
mod ordered_batches;
mod speculative;

use super::super::{
    EngineeringTpBufferAccessV1, HostStagedPartialV1, Qwen3TensorParallelCollectiveStateV1,
    Qwen3TensorParallelPlanV1, RMSNORM, TensorParallelSequenceV1, allocate_rank_storage,
    reduce_residual_bf16_v1,
};
use super::*;
use crate::tp_paged::{EngineeringTpPageRowV1, EngineeringTpPagedLimitsV1};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Event {
    SequenceSubmit(u32, usize),
    SequenceWait(u32, usize),
    OrderedSubmit(u32, usize),
    OrderedWait(u32, usize),
    Submit(u32, &'static str),
    Wait(u32, &'static str),
    Close(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Failure {
    Submit,
    Wait,
    Write,
    BadChoice,
    NonfinitePartial,
    ResidualWait,
    Close,
    RuntimeSnapshot,
    RuntimeCounter,
    OrderedSubmit,
    OrderedWait,
    PreparePackets,
    ArgmaxSubmit,
    ArgmaxWait,
}

struct Recording {
    rank: u32,
    buffers: BTreeMap<u64, Vec<u8>>,
    next: u64,
    pending: Option<EngineeringTpDispatchV1>,
    pending_sequence: Option<Vec<EngineeringTpDispatchV1>>,
    pending_ordered: Option<Vec<EngineeringTpDispatchV1>>,
    ordered_supported: bool,
    sequences_supported: bool,
    events: Rc<RefCell<Vec<Event>>>,
    commands: Vec<EngineeringTpDispatchV1>,
    reads: Vec<(u64, usize)>,
    writes: Vec<(u64, usize)>,
    failure: Option<Failure>,
    choice_override: Option<Vec<u32>>,
    rollover_supported: bool,
    queue_packets: u64,
    queue_epochs: u64,
    packet_preparations: Vec<u64>,
    argmax_v11_loaded: Option<[u8; 32]>,
    argmax_peer: Option<(u32, u32, u32)>,
}

fn scalar(command: &EngineeringTpDispatchV1, index: usize) -> u32 {
    let EngineeringTpArgumentV1::U32(value) = command.arguments[index] else {
        panic!("u32 argument expected");
    };
    value
}

fn exact_f32(value: u32) -> f32 {
    f32::from(u16::try_from(value).unwrap())
}

fn buffer(command: &EngineeringTpDispatchV1, index: usize) -> (u64, usize, usize, u32) {
    let EngineeringTpArgumentV1::Buffer {
        id,
        offset,
        elements,
        element_bytes,
        ..
    } = command.arguments[index]
    else {
        panic!("buffer argument expected");
    };
    (id, offset, elements, element_bytes)
}

impl Recording {
    fn output(
        &mut self,
        command: &EngineeringTpDispatchV1,
        index: usize,
        rows: u32,
        columns: u32,
        value: impl Fn(u32) -> Vec<u8>,
    ) {
        let (id, offset, elements, element_bytes) = buffer(command, index);
        let width = columns as usize * element_bytes as usize;
        assert!(rows as usize * columns as usize <= elements);
        assert!(matches!(
            command.arguments[index],
            EngineeringTpArgumentV1::Buffer {
                access: EngineeringTpBufferAccessV1::Write,
                ..
            }
        ));
        for row in 0..rows {
            let pattern = value(row);
            assert_eq!(pattern.len(), element_bytes as usize);
            let start = offset + row as usize * width;
            for element in self.buffers.get_mut(&id).unwrap()[start..start + width]
                .chunks_exact_mut(element_bytes as usize)
            {
                element.copy_from_slice(&pattern);
            }
        }
    }

    fn u32_values(&self, id: u64, count: usize) -> Vec<u32> {
        self.buffers[&id][..count * 4]
            .chunks_exact(4)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
            .collect()
    }
}

impl EngineeringTpRankTransportV1 for Recording {
    fn peer_group_rank(&self) -> Option<(u32, u32, u32)> {
        self.argmax_peer
    }
    fn require_loaded_image(&mut self, image: [u8; 32], kernels: &[&str]) -> TpResult<()> {
        if self.buffers.is_empty()
            && self.argmax_v11_loaded == Some(image)
            && kernels == crate::tp_artifact::ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11
        {
            Ok(())
        } else {
            Err("recording image was not loaded".into())
        }
    }
    fn supports_queue_rollover(&self) -> bool {
        self.rollover_supported
    }

    fn prepare_packets(&mut self, count: u64) -> TpResult<()> {
        self.packet_preparations.push(count);
        if self.failure == Some(Failure::PreparePackets) {
            return Err("injected queue preparation failure".into());
        }
        let limit = fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1;
        if self.rollover_supported && self.queue_packets + count > limit {
            assert!(self.pending.is_none());
            self.queue_epochs += 1;
            self.queue_packets = 0;
        }
        Ok(())
    }

    fn runtime_diagnostic_snapshot(&mut self) -> TpResult<serde_json::Value> {
        if self.failure == Some(Failure::RuntimeSnapshot) {
            return Err("injected runtime snapshot failure".into());
        }
        assert!(
            self.pending.is_none()
                && self.pending_sequence.is_none()
                && self.pending_ordered.is_none()
        );
        Ok(
            serde_json::json!({"counters":{"dispatches":self.commands.len()
            + usize::from(self.failure == Some(Failure::RuntimeCounter))}}),
        )
    }
    fn supports_sequences(&self) -> bool {
        self.sequences_supported
    }

    fn submit_sequence(&mut self, commands: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        assert!(self.pending.is_none() && self.pending_sequence.is_none());
        assert!((1..=16).contains(&commands.len()));
        self.events
            .borrow_mut()
            .push(Event::SequenceSubmit(self.rank, commands.len()));
        self.pending_sequence = Some(commands.to_vec());
        Ok(())
    }

    fn wait_sequence(&mut self, count: usize) -> TpResult<()> {
        let commands = self.pending_sequence.take().expect("pending sequence");
        assert_eq!(commands.len(), count);
        self.events
            .borrow_mut()
            .push(Event::SequenceWait(self.rank, count));
        for command in commands {
            self.submit(&command)?;
            self.wait()?;
        }
        Ok(())
    }

    fn supports_ordered_batches(&self) -> bool {
        self.ordered_supported
    }

    fn submit_ordered_batch(&mut self, commands: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        assert!(
            self.pending.is_none()
                && self.pending_sequence.is_none()
                && self.pending_ordered.is_none()
        );
        assert!((1..=16).contains(&commands.len()));
        self.events
            .borrow_mut()
            .push(Event::OrderedSubmit(self.rank, commands.len()));
        if self.failure == Some(Failure::OrderedSubmit) {
            return Err("injected ordered publication failure".into());
        }
        self.pending_ordered = Some(commands.to_vec());
        Ok(())
    }

    fn wait_ordered_batch(&mut self, count: usize) -> TpResult<()> {
        let commands = self.pending_ordered.take().expect("pending ordered batch");
        assert_eq!(commands.len(), count);
        self.events
            .borrow_mut()
            .push(Event::OrderedWait(self.rank, count));
        if self.failure == Some(Failure::OrderedWait) {
            return Err("injected ordered aggregate completion failure".into());
        }
        for command in commands {
            self.submit(&command)?;
            self.wait()?;
        }
        Ok(())
    }

    fn allocate(&mut self, byte_len: usize) -> TpResult<u64> {
        assert!(
            self.pending.is_none()
                && self.pending_sequence.is_none()
                && self.pending_ordered.is_none()
        );
        let id = self.next;
        self.next += 1;
        self.buffers.insert(id, vec![0xa5; byte_len]);
        Ok(id)
    }

    fn write(&mut self, id: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        assert!(self.pending_sequence.is_none() && self.pending_ordered.is_none());
        assert!(self.pending.is_none());
        if self.failure == Some(Failure::Write) {
            return Err("injected metadata write failure".into());
        }
        self.buffers.get_mut(&id).unwrap()[offset..offset + bytes.len()].copy_from_slice(bytes);
        self.writes.push((id, bytes.len()));
        Ok(())
    }

    fn read(&mut self, id: u64, offset: usize, bytes: &mut [u8]) -> TpResult<()> {
        assert!(self.pending_sequence.is_none() && self.pending_ordered.is_none());
        assert!(self.pending.is_none());
        bytes.copy_from_slice(&self.buffers[&id][offset..offset + bytes.len()]);
        self.reads.push((id, bytes.len()));
        Ok(())
    }

    fn submit(&mut self, command: &EngineeringTpDispatchV1) -> TpResult<()> {
        if self.failure == Some(Failure::ArgmaxSubmit)
            && command.kernel == crate::tp_artifact::ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11[0]
        {
            return Err("injected argmax submit failure".into());
        }
        assert!(self.pending.is_none());
        assert_eq!(command.workgroup_size, 64);
        assert!(command.grid_workgroups > 0);
        for argument in &command.arguments {
            if let EngineeringTpArgumentV1::Buffer {
                id,
                offset,
                elements,
                element_bytes,
                ..
            } = argument
            {
                assert!(*offset <= self.buffers[id].len());
                assert!(elements * *element_bytes as usize <= self.buffers[id].len() - offset);
            }
        }
        if self.failure == Some(Failure::Submit)
            && matches!(
                command.kernel,
                PARTIAL
                    | "ferric_qwen3_draft_batch32_gemm_partial_bf16_f32_v10"
                    | "ferric_qwen3_draft_batch32_mfma_gemm_partial_f32_v10"
            )
        {
            return Err("injected partial submit failure".into());
        }
        self.events
            .borrow_mut()
            .push(Event::Submit(self.rank, command.kernel));
        self.commands.push(command.clone());
        self.pending = Some(command.clone());
        Ok(())
    }

    fn wait(&mut self) -> TpResult<()> {
        let command = self.pending.take().expect("one submitted request");
        if self.failure == Some(Failure::ArgmaxWait)
            && command.kernel == crate::tp_artifact::ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11[0]
        {
            return Err("injected argmax completion failure".into());
        }
        self.events
            .borrow_mut()
            .push(Event::Wait(self.rank, command.kernel));
        // Reuse only the synthetic byte writer; retain the real v5 names/grids in receipts.
        let is_draft = command.kernel.starts_with("ferric_qwen3_draft_batch32_");
        let hidden = if is_draft { 1024 } else { 4096 };
        let query = if is_draft { 2048 } else { 4096 };
        let intermediate = if is_draft { 3072 } else { 12_288 };
        let kernel = match command.kernel {
            "ferric_qwen3_draft_batch32_rmsnorm_v10" => RMSNORM,
            "ferric_qwen3_draft_batch32_embedding_bf16_v10"
            | "ferric_qwen3_tp_batch32_embedding_bf16_v5" => EMBEDDING,
            "ferric_qwen3_tp_batch32_gemm_bf16_f32_bf16_v5"
            | "ferric_qwen3_tp_batch32_wave_gemv_bf16_v5"
            | "ferric_qwen3_draft_batch32_gemm_bf16_f32_bf16_v10"
            | "ferric_qwen3_draft_batch32_mfma_gemm_bf16_v10"
            | "ferric_qwen3_tp_batch32_mfma_gemm_bf16_v5" => GEMM,
            "ferric_qwen3_tp_batch32_gemm_partial_bf16_f32_v5"
            | "ferric_qwen3_tp_batch32_wave_gemv_partial_f32_v5"
            | "ferric_qwen3_draft_batch32_gemm_partial_bf16_f32_v10"
            | "ferric_qwen3_draft_batch32_mfma_gemm_partial_f32_v10"
            | "ferric_qwen3_tp_batch32_mfma_gemm_partial_f32_v5" => PARTIAL,
            "ferric_qwen3_draft_batch32_swiglu_bf16_f32_v10"
            | "ferric_qwen3_tp_batch32_swiglu_bf16_f32_v5" => SWIGLU,
            "ferric_qwen3_draft_batch32_rope_v10" | "ferric_qwen3_tp_batch32_rope_v5" => ROPE,
            "ferric_qwen3_tp_batch32_paged_kv_append_v5"
            | "ferric_qwen3_draft_batch32_paged_kv_append_v10"
            | "ferric_qwen3_tp_batch32_large_kv_append_v9" => APPEND,
            "ferric_qwen3_tp_batch32_paged_gqa_bf16_f32_v5"
            | "ferric_qwen3_draft_batch32_paged_gqa_bf16_f32_v10"
            | "ferric_qwen3_tp_batch32_large_kv_paged_gqa_bf16_f32_v9"
            | "ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5" => ATTENTION,
            "ferric_qwen3_tp_batch32_argmax_bf16_v5" => ARGMAX,
            "ferric_qwen3_draft_batch32_head_bf16_f32_v10"
            | "ferric_qwen3_tp_batch32_head_bf16_f32_v8" => FP32_HEAD,
            "ferric_qwen3_draft_batch32_mfma_head_f32_v10"
            | "ferric_qwen3_tp_batch32_mfma_head_f32_v8" => FP32_MFMA_HEAD,
            "ferric_qwen3_draft_batch32_argmax_f32_v10"
            | "ferric_qwen3_tp_batch32_wave_argmax_f32_v11"
            | "ferric_qwen3_tp_batch32_argmax_f32_v8" => FP32_ARGMAX,
            "ferric_qwen3_draft_batch32_residual_bf16_v10"
            | "ferric_qwen3_tp_batch32_residual_bf16_v5" => {
                "ferric_qwen3_tp_batch_residual_bf16_v3"
            }
            name => name,
        };
        if self.failure == Some(Failure::Wait) && kernel == PARTIAL {
            return Err("injected partial completion failure".into());
        }
        match kernel {
            "ferric_qwen3_tp_batch_residual_bf16_v3" => {
                if self.failure == Some(Failure::ResidualWait) {
                    return Err("injected residual completion failure".into());
                }
                let elements = scalar(&command, 3) as usize * hidden as usize;
                let input = buffer(&command, 0);
                let residual = buffer(&command, 1);
                let output = buffer(&command, 2);
                assert_ne!(input.0, residual.0);
                assert_ne!(input.0, output.0);
                assert_ne!(residual.0, output.0);
                assert_eq!(
                    (input.2, residual.2, output.2),
                    (elements, elements, elements)
                );
                assert_eq!((input.3, residual.3, output.3), (4, 2, 2));
                let partial = self.buffers[&input.0][..elements * 4]
                    .chunks_exact(4)
                    .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
                    .collect::<Vec<_>>();
                let residual = self.buffers[&residual.0][..elements * 2]
                    .chunks_exact(2)
                    .map(|bytes| u16::from_le_bytes(bytes.try_into().unwrap()))
                    .collect::<Vec<_>>();
                let result = reduce_residual_bf16_v1(
                    1,
                    &[HostStagedPartialV1 {
                        rank: 0,
                        values: &partial,
                    }],
                    &residual,
                )?;
                for (bytes, value) in self.buffers.get_mut(&output.0).unwrap()[..elements * 2]
                    .chunks_exact_mut(2)
                    .zip(result)
                {
                    bytes.copy_from_slice(&value.to_le_bytes());
                }
            }
            EMBEDDING => self.output(&command, 2, scalar(&command, 3), hidden, |row| {
                u16::try_from(exact_f32(row + 1).to_bits() >> 16)
                    .unwrap()
                    .to_le_bytes()
                    .to_vec()
            }),
            GEMM | "ferric_qwen3_tp_wave_gemv_bf16_v3" | "ferric_qwen3_tp_mfma_gemm_bf16_v3" => {
                self.output(
                    &command,
                    2,
                    scalar(&command, 3),
                    scalar(&command, 4),
                    |_| vec![0; 2],
                );
            }
            FP32_HEAD | FP32_MFMA_HEAD => self.output(
                &command,
                2,
                scalar(&command, 3),
                scalar(&command, 4),
                |_| vec![0; 4],
            ),
            PARTIAL
            | "ferric_qwen3_tp_wave_gemv_partial_f32_v3"
            | "ferric_qwen3_tp_mfma_gemm_partial_f32_v3" => {
                let rank = self.rank;
                let nonfinite = self.failure == Some(Failure::NonfinitePartial);
                self.output(
                    &command,
                    2,
                    scalar(&command, 3),
                    scalar(&command, 4),
                    |row| {
                        let value = if nonfinite {
                            f32::NAN
                        } else {
                            exact_f32((rank + 1) * (row + 1)) / 1024.0
                        };
                        value.to_le_bytes().to_vec()
                    },
                );
            }
            RMSNORM => {
                let rows = scalar(&command, 5);
                let width = scalar(&command, 6);
                assert_eq!(buffer(&command, 0).2, rows as usize * width as usize);
                assert_eq!(buffer(&command, 4).2, rows as usize * width as usize);
                for index in [1, 3] {
                    let (id, offset, elements, bytes) = buffer(&command, index);
                    assert_eq!((offset, elements, bytes), (0, 0, 2));
                    assert!(self.buffers[&id].len() >= 2);
                }
                assert_eq!(command.grid_workgroups, rows);
                self.output(&command, 4, rows, width, |_| vec![0; 2]);
            }
            SWIGLU => self.output(
                &command,
                2,
                scalar(&command, 3),
                intermediate / scalar(&command, 4),
                |_| vec![0; 2],
            ),
            ROPE => {
                let rows = scalar(&command, 7);
                let world = scalar(&command, 8);
                self.output(&command, 5, rows, query / world, |_| vec![0; 2]);
                self.output(&command, 6, rows, 1024 / world, |_| vec![0; 2]);
            }
            APPEND => {
                let rows = scalar(&command, 6) as usize;
                let stride = scalar(&command, 8) as usize;
                let pages = scalar(&command, 9);
                let positions = self.u32_values(buffer(&command, 2).0, rows);
                let table = self.u32_values(buffer(&command, 3).0, rows * stride);
                let mut slots = std::collections::BTreeSet::new();
                for (row, position) in positions.into_iter().enumerate() {
                    let logical = position as usize / 16;
                    assert!(logical < stride);
                    let page = table[row * stride + logical];
                    assert!(page < pages);
                    assert!(slots.insert((page, position % 16)));
                }
                assert_eq!(command.grid_workgroups, 1);
            }
            ATTENTION | "ferric_qwen3_tp_wave_paged_gqa_bf16_v3" => {
                let rows = scalar(&command, 6);
                let world = scalar(&command, 7);
                let context = scalar(&command, 10);
                let positions = self.u32_values(buffer(&command, 3).0, rows as usize);
                assert_eq!(context, positions.iter().max().unwrap() + 1);
                assert_eq!(command.grid_workgroups, rows * (query / 128) / world);
                self.output(&command, 5, rows, query / world, |_| vec![0; 2]);
            }
            ARGMAX | FP32_ARGMAX => {
                let bad = self.failure == Some(Failure::BadChoice);
                let choices = self.choice_override.clone();
                self.output(&command, 1, scalar(&command, 2), 1, |row| {
                    (if bad {
                        151_936
                    } else {
                        choices
                            .as_ref()
                            .map_or(42 + row, |values| values[row as usize])
                    })
                    .to_le_bytes()
                    .to_vec()
                });
            }
            other => panic!("unexpected batch kernel {other}"),
        }
        self.queue_packets += 1;
        Ok(())
    }

    fn close(&mut self) -> TpResult<()> {
        if self.failure == Some(Failure::Close) {
            return Err("injected close failure".into());
        }
        self.pending = None;
        self.events.borrow_mut().push(Event::Close(self.rank));
        Ok(())
    }
}

fn target() -> ModelConfig {
    use ferric_spec::Identity;
    ModelConfig {
        role: Qwen3ModelRole::Target8B,
        model_id: Identity::new([1; 32]),
        config_id: Identity::new([2; 32]),
        vocabulary_size: 151_936,
        layers: 36,
        hidden_size: 4096,
        intermediate_size: 12_288,
        query_heads: 32,
        kv_heads: 8,
        head_dim: 128,
        max_position_embeddings: 40_960,
        rope_theta: 1_000_000,
        tie_word_embeddings: false,
    }
}

fn pool() -> EngineeringTpPagedPoolV1 {
    EngineeringTpPagedPoolV1::new(
        EngineeringTpPoolScopeV1 {
            model: [1; 32],
            session: [2; 32],
        },
        EngineeringTpPagedLimitsV1::new(64, 32, 4, 100).unwrap(),
    )
    .unwrap()
}

fn fixture(
    world: u32,
    pool: &EngineeringTpPagedPoolV1,
) -> EngineeringTpBatchExecutionV2<Recording> {
    fixture_for_model(world, pool, target())
}

fn fixture_for_model(
    world: u32,
    pool: &EngineeringTpPagedPoolV1,
    model: ModelConfig,
) -> EngineeringTpBatchExecutionV2<Recording> {
    let row_capacity = pool.row_capacity();
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut transports = (0..world)
        .map(|rank| Recording {
            argmax_v11_loaded: None,
            argmax_peer: None,
            rank,
            buffers: BTreeMap::new(),
            next: 1,
            pending: None,
            pending_sequence: None,
            pending_ordered: None,
            ordered_supported: true,
            sequences_supported: true,
            events: events.clone(),
            commands: Vec::new(),
            reads: Vec::new(),
            writes: Vec::new(),
            failure: None,
            choice_override: None,
            rollover_supported: false,
            queue_packets: 0,
            queue_epochs: 0,
            packet_preparations: Vec::new(),
        })
        .collect::<Vec<_>>();
    let plan = Qwen3TensorParallelPlanV1::new(model, world).unwrap();
    let mut ranks = Vec::new();
    let mut positions = Vec::new();
    let mut page_tables = Vec::new();
    for (index, transport) in transports.iter_mut().enumerate() {
        let mut rank = allocate_rank_storage(
            transport,
            &plan,
            u32::try_from(index).unwrap(),
            64,
            u32::try_from(row_capacity).unwrap(),
        )
        .unwrap();
        // Weight intake is tested separately; this fixture does not emulate a
        // dense model or assert that synthetic outputs are GPU computations.
        let placeholder = allocate_tensor(transport, 1, 2).unwrap();
        for layer in &mut rank.layers {
            for kind in [
                Qwen3TensorKind::InputLayerNorm,
                Qwen3TensorKind::PostAttentionLayerNorm,
                Qwen3TensorKind::QueryNorm,
                Qwen3TensorKind::KeyNorm,
                Qwen3TensorKind::QueryProjection,
                Qwen3TensorKind::KeyProjection,
                Qwen3TensorKind::ValueProjection,
                Qwen3TensorKind::OutputProjection,
                Qwen3TensorKind::GateProjection,
                Qwen3TensorKind::UpProjection,
                Qwen3TensorKind::DownProjection,
            ] {
                layer.weights.push((kind, placeholder));
            }
        }
        if index == 0 {
            rank.globals = vec![
                (Qwen3TensorKind::TokenEmbedding, placeholder),
                (Qwen3TensorKind::FinalNorm, placeholder),
                (Qwen3TensorKind::LanguageModelHead, placeholder),
            ];
        }
        ranks.push(rank);
        positions.push(allocate_tensor(transport, row_capacity, 4).unwrap());
        page_tables.push(allocate_tensor(transport, row_capacity * 4, 4).unwrap());
    }
    let inner = EngineeringTpExecutionV1 {
        transports,
        ranks,
        plan,
        sequence: TensorParallelSequenceV1::new(64, model.vocabulary_size).unwrap(),
        collective: Qwen3TensorParallelCollectiveStateV1::new(&plan, 0, 0),
        capacity: 64,
        row_capacity: u32::try_from(row_capacity).unwrap(),
        large_kv: pool.large_kv_binding().is_some(),
        draft_v10: false,
        hidden: vec![0; model.hidden_size as usize * row_capacity],
        reduction: super::super::ReductionWorkspace::default(),
        sequences: None,
        ordered_batches: None,
        timing: crate::host_timing::HostTiming::default(),
        closed: false,
    };
    EngineeringTpBatchExecutionV2 {
        inner,
        row_capacity,
        positions,
        page_tables,
        scope: pool.scope(),
        pool_identity: pool.identity(),
        context_tokens: 64,
        physical_pages: 4,
        table_stride: 4,
        last_batch: 0,
        completed_batches: 0,
        poisoned: false,
        prune_output_head: false,
        projection: super::super::projection::ProjectionPolicy::default(),
        projection_configured: false,
        wave_attention: false,
        numerical: None,
        head_profile_configured: false,
        fp32_logits: None,
        fp32_argmax_v11: None,
        admitted_argmax_v11: None,
    }
}

#[test]
fn resident_weight_bytes_counts_rank_local_allocations_not_aliases_or_workspaces() {
    for world in [1, 2, 8] {
        let mut driver = fixture(world, &pool());
        assert_eq!(
            driver.resident_weight_bytes().unwrap(),
            u64::from(world) * 2
        );
        let extra = allocate_tensor(&mut driver.inner.transports[0], 7, 4).unwrap();
        driver.inner.ranks[0]
            .globals
            .push((Qwen3TensorKind::FinalNorm, extra));
        assert_eq!(
            driver.resident_weight_bytes().unwrap(),
            u64::from(world) * 2 + 28
        );
        driver.projection.bytes = 4096;
        assert_eq!(
            driver.resident_weight_bytes().unwrap(),
            u64::from(world) * 2 + 28
        );
        driver.inner.ranks[0].globals.last_mut().unwrap().1.elements = 8;
        driver.inner.ranks[0].layers[0]
            .weights
            .push((Qwen3TensorKind::InputLayerNorm, extra));
        assert!(driver.resident_weight_bytes().is_err());
    }
}

#[test]
fn resident_weight_bytes_rejects_overflow() {
    let mut driver = fixture(1, &pool());
    driver.inner.ranks[0].globals[0].1.elements = usize::MAX;
    driver.inner.ranks[0].globals[0].1.element_bytes = 4;
    assert!(driver.resident_weight_bytes().is_err());
}

#[test]
fn runtime_snapshots_bind_completed_dispatch_counts_and_poison_on_failure() {
    let mut pool = pool();
    let mut driver = fixture(2, &pool);
    let before = driver.runtime_diagnostic_snapshot().unwrap();
    assert_eq!(before.len(), 2);
    assert_eq!(before[0]["counters"]["dispatches"], 0);
    let prepared = prepare(&mut pool, 3);
    driver.execute_selected(&prepared, &[0, 1, 2]).unwrap();
    let after = driver.runtime_diagnostic_snapshot().unwrap();
    assert_eq!(after[0]["counters"]["dispatches"], 544);
    assert_eq!(after[1]["counters"]["dispatches"], 540);
    driver.close().unwrap();
    assert!(driver.runtime_diagnostic_snapshot().is_err());
    for failure in [Failure::RuntimeSnapshot, Failure::RuntimeCounter] {
        let mut driver = fixture(2, &pool);
        driver.inner.transports[1].failure = Some(failure);
        assert!(driver.runtime_diagnostic_snapshot().is_err());
        assert!(driver.poisoned);
        assert!(driver.runtime_diagnostic_snapshot().is_err());
        driver.close().unwrap();
    }
    let mut mismatched = fixture(2, &pool);
    mismatched.inner.transports.pop();
    assert!(mismatched.runtime_diagnostic_snapshot().is_err());
    assert!(mismatched.poisoned);
    mismatched.close().unwrap();
}

#[test]
fn diagnostic_driver_freezes_policy_and_rejects_failed_close() {
    let path = std::env::temp_dir().join(format!("ferric-numerical-driver-{}", std::process::id()));
    let mut identity = serde_json::json!({"tensor_parallel":1,"projection":"baseline","output_head_pruning":false,"device_unique_id":1});
    for key in [
        "controller_sha256",
        "worker_sha256",
        "artifact_hsaco_id",
        "artifact_manifest_id",
        "artifact_handoff_id",
        "model_bundle_id",
        "requests_sha256",
        "session_id",
    ] {
        identity[key] = serde_json::json!("ab".repeat(32));
    }
    let capture =
        EngineeringTpNumericalCaptureV1::new(&path, 1, 0, NumericalRole::Query, identity).unwrap();
    let mut driver = fixture(1, &pool());
    driver.configure_numerical_capture(capture).unwrap();
    assert!(driver.configure_output_head_pruning(true).is_err());
    assert!(driver.configure_wave_attention(true).is_err());
    assert!(
        driver
            .configure_reduction(EngineeringTpReductionModeV3::HostStagedV1)
            .is_err()
    );
    assert!(driver.configure_dispatch_sequences(true).is_err());
    assert!(driver.finish_numerical_capture().is_err());
    driver.inner.transports[0].failure = Some(Failure::Close);
    assert!(driver.close().is_err());
    assert!(driver.poisoned);
    driver.inner.transports[0].failure = None;
    driver.close().unwrap();
    assert!(driver.finish_numerical_capture().is_err());
    assert!(!path.join("manifest.json").exists());
    drop(driver);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn diagnostic_projection_roster_matches_every_actual_selected_dispatch() {
    for mfma in [false, true] {
        for prune in [false, true] {
            let mut pool = pool();
            let mut driver = fixture(1, &pool);
            driver.configure_output_head_pruning(prune).unwrap();
            if mfma {
                let original =
                    driver.inner.ranks[0].layers[0].weight(Qwen3TensorKind::QueryProjection);
                let transposed = allocate_tensor(&mut driver.inner.transports[0], 1, 2).unwrap();
                driver.projection =
                    super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
                        original.id,
                        transposed,
                    );
            }
            let batch = prepare(&mut pool, 4);
            pool.begin_submission(&batch).unwrap();
            driver.execute_selected(&batch, &[1, 3]).unwrap();
            let rank = &driver.inner.ranks[0];
            let projections = driver.inner.transports[0]
                .commands
                .iter()
                .filter(|command| command.kernel.contains("gemm"))
                .collect::<Vec<_>>();
            for (index, role) in [
                NumericalRole::Query,
                NumericalRole::Key,
                NumericalRole::Value,
                NumericalRole::AttentionOutput,
                NumericalRole::Gate,
                NumericalRole::Up,
                NumericalRole::Down,
            ]
            .into_iter()
            .enumerate()
            {
                let (captured, original) =
                    numerical_projection_command(rank, &driver.projection, 0, role, 4);
                assert_eq!(&captured, projections[index]);
                assert_eq!(
                    original,
                    rank.layers[0]
                        .weight(Qwen3TensorKind::QueryProjection)
                        .read()
                );
            }
            let head_rows = if prune { 2 } else { 4 };
            let captured = driver.projection.command(
                0,
                GEMM,
                rank.normalized,
                rank.global(Qwen3TensorKind::LanguageModelHead),
                rank.logits,
                [head_rows, 151_936, 4096, 1, 6],
            );
            assert_eq!(&captured, *projections.last().unwrap());
            assert_eq!(
                driver.inner.transports[0].u32_values(rank.token.id, 4),
                if prune {
                    vec![1, 3, 0, 2]
                } else {
                    vec![0, 1, 2, 3]
                }
            );
        }
    }
}

#[test]
fn v7_head_only_changes_final_kernels_and_dtype_with_exact_row_pruning() {
    for fp32 in [false, true] {
        for mfma in [false, true] {
            for prune in [false, true] {
                let mut pool = pool();
                let mut driver = fixture(1, &pool);
                driver.configure_output_head_pruning(prune).unwrap();
                if mfma {
                    let original = driver.inner.ranks[0].global(Qwen3TensorKind::LanguageModelHead);
                    let transposed =
                        allocate_tensor(&mut driver.inner.transports[0], 1, 2).unwrap();
                    driver.projection =
                        super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
                            original.id,
                            transposed,
                        );
                }
                let original_logits = driver.inner.ranks[0].logits;
                let before = driver.inner.transports[0].buffers.len();
                driver.configure_head_precision_v7(fp32).unwrap();
                assert_eq!(
                    driver.fp32_head_workspace_bytes(),
                    if fp32 { 9_723_904 } else { 0 }
                );
                assert_eq!(
                    driver.inner.transports[0].buffers.len(),
                    before + usize::from(fp32)
                );
                assert_eq!(driver.inner.ranks[0].logits.id, original_logits.id);
                assert!(driver.configure_head_precision_v7(fp32).is_err());
                assert!(driver.configure_wave_attention(true).is_err());
                assert!(driver.configure_dispatch_sequences(true).is_err());
                let batch = prepare(&mut pool, 4);
                pool.begin_submission(&batch).unwrap();
                let result = driver.execute_selected(&batch, &[1, 3]).unwrap();
                assert_eq!(
                    result.choices,
                    if prune { vec![42, 43] } else { vec![43, 45] }
                );
                let commands = &driver.inner.transports[0].commands;
                assert_eq!(commands.len(), 544);
                let head = &commands[commands.len() - 2];
                let argmax = &commands[commands.len() - 1];
                let head_rows = if prune { 2 } else { 4 };
                assert_eq!(
                    head.kernel,
                    if fp32 {
                        if mfma { FP32_MFMA_HEAD } else { FP32_HEAD }
                    } else if mfma {
                        "ferric_qwen3_tp_mfma_gemm_bf16_v3"
                    } else {
                        GEMM
                    }
                );
                assert_eq!(argmax.kernel, if fp32 { FP32_ARGMAX } else { ARGMAX });
                assert_eq!(head.grid_workgroups, 9496);
                assert_eq!(argmax.grid_workgroups, head_rows);
                assert_eq!(buffer(head, 2).3, if fp32 { 4 } else { 2 });
                assert_eq!(buffer(head, 2), buffer(argmax, 0));
                assert_eq!(buffer(head, 2).2, 16 * 151_936);
                assert_eq!(
                    driver.inner.transports[0].u32_values(driver.inner.ranks[0].token.id, 4),
                    if prune {
                        vec![1, 3, 0, 2]
                    } else {
                        vec![0, 1, 2, 3]
                    }
                );
                if fp32 {
                    let tensor = driver.fp32_logits.unwrap();
                    let storage = &driver.inner.transports[0].buffers[&tensor.id];
                    assert!(
                        storage[head_rows as usize * 151_936 * 4..]
                            .iter()
                            .all(|&byte| byte == 0xa5)
                    );
                }
            }
        }
    }
}

#[test]
fn v7_head_rejects_unsupported_geometry_modes_and_lifecycle() {
    for world in [2, 8] {
        assert!(
            fixture(world, &pool())
                .configure_head_precision_v7(true)
                .is_err()
        );
    }
    let mut driver = fixture(1, &pool());
    driver.wave_attention = true;
    assert!(driver.configure_head_precision_v7(true).is_err());
    driver.wave_attention = false;
    driver.inner.sequences = Some(vec![vec![]]);
    assert!(driver.configure_head_precision_v7(true).is_err());
    driver.inner.sequences = None;
    for mode in [
        super::super::EngineeringTpProjectionModeV3::Wave,
        super::super::EngineeringTpProjectionModeV3::Auto,
    ] {
        driver.projection.mode = mode;
        assert!(driver.configure_head_precision_v7(true).is_err());
    }
    driver.projection.mode = super::super::EngineeringTpProjectionModeV3::Baseline;
    driver.row_capacity = 32;
    assert!(driver.configure_head_precision_v7(true).is_err());
    driver.row_capacity = 16;
    driver.last_batch = 1;
    assert!(driver.configure_head_precision_v7(true).is_err());
    driver.last_batch = 0;
    driver.poisoned = true;
    assert!(driver.configure_head_precision_v7(true).is_err());
    driver.poisoned = false;
    driver.inner.closed = true;
    assert!(driver.configure_head_precision_v7(true).is_err());
}

#[test]
fn fp32_candidate_preserves_every_non_head_dispatch_and_prefill_only_skip() {
    let mut recorded = Vec::new();
    for fp32 in [false, true] {
        let mut pool = pool();
        let mut driver = fixture(1, &pool);
        driver.configure_head_precision_v7(fp32).unwrap();
        let batch = prepare(&mut pool, 3);
        pool.begin_submission(&batch).unwrap();
        driver.execute(&batch).unwrap();
        recorded.push(driver.inner.transports[0].commands.clone());
    }
    assert_eq!(&recorded[0][..542], &recorded[1][..542]);
    let mut pool = pool();
    let mut driver = fixture(1, &pool);
    driver.configure_output_head_pruning(true).unwrap();
    driver.configure_head_precision_v7(true).unwrap();
    let batch = prepare(&mut pool, 3);
    pool.begin_submission(&batch).unwrap();
    assert!(
        driver
            .execute_selected(&batch, &[])
            .unwrap()
            .choices
            .is_empty()
    );
    assert_eq!(driver.inner.transports[0].commands.len(), 541);
    assert!(
        !driver.inner.transports[0].commands.iter().any(|command| [
            FP32_HEAD,
            FP32_MFMA_HEAD,
            FP32_ARGMAX
        ]
        .contains(&command.kernel))
    );
    let logits = driver.fp32_logits.unwrap();
    assert!(
        driver.inner.transports[0].buffers[&logits.id]
            .iter()
            .all(|&byte| byte == 0xa5)
    );
}

fn wide_pool() -> EngineeringTpPagedPoolV1 {
    EngineeringTpPagedPoolV1::new_wide32(
        pool().scope(),
        EngineeringTpPagedLimitsV1::new(64, 32, 4, 100).unwrap(),
    )
    .unwrap()
}

#[test]
fn v8_head_routes_real_row_tiles_and_preserves_non_head_dispatches() {
    for rows in [1, 16, 17, 31, 32] {
        for mfma in [false, true] {
            let mut recorded = Vec::new();
            for fp32 in [false, true] {
                let mut pool = wide_pool();
                let mut driver = fixture(1, &pool);
                if mfma {
                    let original = driver.inner.ranks[0].global(Qwen3TensorKind::LanguageModelHead);
                    let transposed =
                        allocate_tensor(&mut driver.inner.transports[0], 1, 2).unwrap();
                    driver.projection =
                        super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
                            original.id,
                            transposed,
                        );
                }
                let original_logits = driver.inner.ranks[0].logits;
                let before = driver.inner.transports[0].buffers.len();
                driver.configure_head_precision_v8(fp32).unwrap();
                assert_eq!(
                    driver.fp32_head_workspace_bytes(),
                    if fp32 { 19_447_808 } else { 0 }
                );
                assert_eq!(driver.inner.ranks[0].logits.id, original_logits.id);
                assert_eq!(
                    driver.inner.transports[0].buffers.len(),
                    before + usize::from(fp32)
                );
                let batch = prepare(&mut pool, rows);
                pool.begin_submission(&batch).unwrap();
                let result = driver.execute(&batch).unwrap();
                assert_eq!(result.choices.len(), rows as usize);
                let commands = &driver.inner.transports[0].commands;
                assert_eq!(commands.len(), 544);
                let head = &commands[542];
                let argmax = &commands[543];
                assert_eq!(
                    head.kernel,
                    match (fp32, mfma) {
                        (true, false) => "ferric_qwen3_tp_batch32_head_bf16_f32_v8",
                        (true, true) => "ferric_qwen3_tp_batch32_mfma_head_f32_v8",
                        (false, false) => "ferric_qwen3_tp_batch32_gemm_bf16_f32_bf16_v5",
                        (false, true) => "ferric_qwen3_tp_batch32_mfma_gemm_bf16_v5",
                    }
                );
                assert_eq!(
                    argmax.kernel,
                    if fp32 {
                        "ferric_qwen3_tp_batch32_argmax_f32_v8"
                    } else {
                        "ferric_qwen3_tp_batch32_argmax_bf16_v5"
                    }
                );
                assert_eq!(head.grid_workgroups, rows.div_ceil(16) * 9496);
                assert_eq!(argmax.grid_workgroups, rows);
                assert_eq!(scalar(head, 3), rows);
                assert_eq!(buffer(head, 2), buffer(argmax, 0));
                assert_eq!(buffer(head, 2).2, 32 * 151_936);
                assert_eq!(buffer(head, 2).3, if fp32 { 4 } else { 2 });
                if fp32 {
                    let logits = driver.fp32_logits.unwrap();
                    assert_eq!(
                        driver.inner.transports[0].buffers[&logits.id].len(),
                        19_447_808
                    );
                    assert!(
                        driver.inner.transports[0].buffers[&logits.id]
                            [rows as usize * 151_936 * 4..]
                            .iter()
                            .all(|&byte| byte == 0xa5)
                    );
                }
                recorded.push(commands.clone());
                pool.commit_batch(&batch, result.completion).unwrap();
                driver.close().unwrap();
            }
            assert_eq!(&recorded[0][..542], &recorded[1][..542]);
        }
    }
}

#[test]
fn v8_head_pruning_uses_selected_rows_and_prefill_only_skips_the_head() {
    for selected in [vec![], vec![16, 30]] {
        let mut pool = wide_pool();
        let mut driver = fixture(1, &pool);
        driver.configure_output_head_pruning(true).unwrap();
        driver.configure_head_precision_v8(true).unwrap();
        let batch = prepare(&mut pool, 32);
        pool.begin_submission(&batch).unwrap();
        let result = driver.execute_selected(&batch, &selected).unwrap();
        assert_eq!(
            result.choices,
            if selected.is_empty() {
                vec![]
            } else {
                vec![42, 43]
            }
        );
        let commands = &driver.inner.transports[0].commands;
        assert_eq!(commands.len(), if selected.is_empty() { 541 } else { 544 });
        if selected.is_empty() {
            assert!(
                !commands
                    .iter()
                    .any(|command| command.kernel.ends_with("_v8"))
            );
        } else {
            let head = &commands[542];
            assert_eq!(head.kernel, "ferric_qwen3_tp_batch32_head_bf16_f32_v8");
            assert_eq!(head.grid_workgroups, 9496);
            assert_eq!(scalar(head, 3), 2);
            assert_eq!(
                driver.inner.transports[0].u32_values(driver.inner.ranks[0].token.id, 2),
                [16, 30]
            );
        }
        let logits = driver.fp32_logits.unwrap();
        assert!(
            driver.inner.transports[0].buffers[&logits.id][selected.len() * 151_936 * 4..]
                .iter()
                .all(|&byte| byte == 0xa5)
        );
    }
}

#[test]
fn v8_head_scope_and_configuration_freeze_do_not_broaden_v7() {
    assert!(
        fixture(1, &pool())
            .configure_head_precision_v8(true)
            .is_err()
    );
    assert!(
        fixture(1, &wide_pool())
            .configure_head_precision_v7(true)
            .is_err()
    );
    for world in [2, 8] {
        assert!(
            fixture(world, &wide_pool())
                .configure_head_precision_v8(true)
                .is_err()
        );
    }
    for mode in [
        super::super::EngineeringTpProjectionModeV3::Wave,
        super::super::EngineeringTpProjectionModeV3::Auto,
    ] {
        let mut driver = fixture(1, &wide_pool());
        driver.projection.mode = mode;
        assert!(driver.configure_head_precision_v8(true).is_err());
    }
    let mut driver = fixture(1, &wide_pool());
    driver.inner.sequences = Some(vec![vec![]]);
    assert!(driver.configure_head_precision_v8(true).is_err());
    driver.inner.sequences = None;
    driver.last_batch = 1;
    assert!(driver.configure_head_precision_v8(true).is_err());
    driver.last_batch = 0;
    driver.poisoned = true;
    assert!(driver.configure_head_precision_v8(true).is_err());
    driver.poisoned = false;
    driver.inner.closed = true;
    assert!(driver.configure_head_precision_v8(true).is_err());
    driver.inner.closed = false;
    driver.configure_head_precision_v8(true).unwrap();
    assert!(driver.configure_head_precision_v8(true).is_err());
    assert!(driver.configure_head_precision_v7(true).is_err());
    assert!(driver.configure_wave_attention(true).is_err());
    assert!(driver.configure_dispatch_sequences(true).is_err());
}

#[test]
fn v8_head_with_wave_attention_preserves_exact_dispatches_and_freezes_policy() {
    for fp32 in [false, true] {
        for mfma in [false, true] {
            let mut pool = wide_pool();
            let mut driver = fixture(1, &pool);
            if mfma {
                let original = driver.inner.ranks[0].global(Qwen3TensorKind::LanguageModelHead);
                let transposed = allocate_tensor(&mut driver.inner.transports[0], 1, 2).unwrap();
                driver.projection =
                    super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
                        original.id,
                        transposed,
                    );
            }
            driver.configure_wave_attention(true).unwrap();
            driver.configure_head_precision_v8(fp32).unwrap();
            assert!(driver.configure_wave_attention(false).is_err());
            assert!(driver.configure_wave_attention(true).is_err());
            assert!(driver.configure_dispatch_sequences(true).is_err());
            assert!(driver.configure_head_precision_v8(fp32).is_err());
            let batch = prepare(&mut pool, 32);
            pool.begin_submission(&batch).unwrap();
            assert_eq!(driver.execute(&batch).unwrap().choices.len(), 32);
            let commands = &driver.inner.transports[0].commands;
            assert_eq!(commands.len(), 544);
            let attention = commands
                .iter()
                .filter(|command| {
                    command.kernel == "ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5"
                })
                .collect::<Vec<_>>();
            assert_eq!(attention.len(), 36);
            assert!(
                attention
                    .iter()
                    .all(|command| command.grid_workgroups == 32 * 32)
            );
            assert_eq!(
                commands[543].kernel,
                if fp32 {
                    "ferric_qwen3_tp_batch32_argmax_f32_v8"
                } else {
                    "ferric_qwen3_tp_batch32_argmax_bf16_v5"
                }
            );
        }
        let mut legacy = fixture(1, &self::pool());
        legacy.configure_wave_attention(true).unwrap();
        assert!(legacy.configure_head_precision_v7(fp32).is_err());
        legacy.configure_wave_attention(false).unwrap();
        legacy.configure_head_precision_v7(fp32).unwrap();
        legacy.configure_wave_attention(false).unwrap();
        assert!(legacy.configure_wave_attention(true).is_err());
    }
}

fn prepare(pool: &mut EngineeringTpPagedPoolV1, rows: u32) -> EngineeringTpPreparedBatchV1 {
    let prompt = (0..rows).collect::<Vec<_>>();
    let sequence = pool
        .open_sequence(pool.scope(), &prompt, 0)
        .unwrap()
        .sequence();
    let selected = prompt
        .into_iter()
        .enumerate()
        .map(|(position, token)| EngineeringTpPageRowV1 {
            sequence,
            token,
            position: u32::try_from(position).unwrap(),
        })
        .collect::<Vec<_>>();
    pool.reserve_batch(&selected).unwrap()
}

#[test]
fn wide_driver_uses_one_real_grid_per_root_with_exact_active_extents() {
    use super::super::EngineeringTpProjectionModeV3;
    for (world, rows, wave, prune) in [
        (8, 17, false, false),
        (8, 32, true, true),
        (1, 31, false, true),
    ] {
        let limits = EngineeringTpPagedLimitsV1::new(64, 32, 4, 100).unwrap();
        let mut pool = EngineeringTpPagedPoolV1::new_wide32(pool().scope(), limits).unwrap();
        let mut driver = fixture(world, &pool);
        driver.configure_output_head_pruning(prune).unwrap();
        driver.configure_dispatch_sequences(wave).unwrap();
        if wave {
            driver.projection.mode = EngineeringTpProjectionModeV3::Wave;
            driver.configure_wave_attention(true).unwrap();
        }
        if world == 1 {
            driver
                .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
                .unwrap();
        }
        let batch = prepare(&mut pool, rows);
        pool.begin_submission(&batch).unwrap();
        let result = driver
            .execute_selected(&batch, &[(rows - 1) as usize])
            .unwrap();
        assert_eq!(result.choices.len(), 1);
        assert_eq!(driver.dispatch_counts(), driver.expected_dispatch_counts(1));
        for transport in &driver.inner.transports {
            for command in &transport.commands {
                assert!(
                    command.kernel == RMSNORM
                        || command.kernel.starts_with("ferric_qwen3_tp_batch32_")
                );
                if command.kernel.contains("_gemm_") {
                    assert_eq!(
                        command.grid_workgroups,
                        scalar(command, 3).div_ceil(16) * (scalar(command, 4) / 16)
                    );
                } else if command.kernel.contains("_wave_gemv_") {
                    assert_eq!(
                        command.grid_workgroups,
                        scalar(command, 3) * scalar(command, 4)
                    );
                }
            }
            assert_eq!(transport.commands.iter().filter(|command| command.kernel == "ferric_qwen3_tp_batch32_paged_kv_append_v5").count(), 36);
        }
        pool.commit_batch(&batch, result.completion).unwrap();
        pool.check_invariants().unwrap();
        driver.close().unwrap();
    }
}

#[test]
fn selected_output_rows_move_together_with_positions_tokens_and_page_tables() {
    for world in [1, 8] {
        let mut pool = pool();
        let mut driver = fixture(world, &pool);
        driver.configure_output_head_pruning(true).unwrap();
        let batch = prepare(&mut pool, 4);
        pool.begin_submission(&batch).unwrap();
        let result = driver.execute_selected(&batch, &[1, 3]).unwrap();
        assert_eq!(result.choices, [42, 43]);
        assert_eq!(driver.dispatch_counts(), driver.expected_dispatch_counts(2));
        for (index, transport) in driver.inner.transports.iter().enumerate() {
            assert_eq!(
                transport.u32_values(driver.positions[index].id, 4),
                [1, 3, 0, 2]
            );
            let table = transport.u32_values(driver.page_tables[index].id, 16);
            for (gpu_row, source_row) in [1, 3, 0, 2].into_iter().enumerate() {
                assert_eq!(
                    table[gpu_row * 4],
                    batch.rows()[source_row].physical_pages()[0]
                );
            }
        }
        let transport = &driver.inner.transports[0];
        assert_eq!(
            transport.u32_values(driver.inner.ranks[0].token.id, 4),
            [1, 3, 0, 2]
        );
        let head = transport
            .commands
            .iter()
            .find(|command| command.kernel == GEMM && scalar(command, 7) == 6)
            .unwrap();
        assert_eq!(scalar(head, 3), 2);
        assert!(
            transport
                .commands
                .iter()
                .filter(|command| command.kernel == ATTENTION)
                .all(|command| scalar(command, 6) == 4)
        );
        assert!(
            transport
                .reads
                .contains(&(driver.inner.ranks[0].choice.id, 8))
        );
        assert!(driver.configure_output_head_pruning(false).is_err());
        pool.commit_batch(&batch, result.completion).unwrap();
    }
}

#[test]
fn sequences_preserve_collective_barriers_counts_and_submit_all_ranks_first() {
    for world in [1, 2, 8] {
        let mut pool = pool();
        let mut driver = fixture(world, &pool);
        driver.configure_dispatch_sequences(true).unwrap();
        driver.configure_output_head_pruning(true).unwrap();
        driver
            .configure_reduction(EngineeringTpReductionModeV3::HostStagedReuseV3)
            .unwrap();
        let batch = prepare(&mut pool, 3);
        pool.begin_submission(&batch).unwrap();
        let result = driver.execute_selected(&batch, &[2]).unwrap();
        assert_eq!(result.choices, [42]);
        assert_eq!(driver.dispatch_counts(), driver.expected_dispatch_counts(1));
        let events = driver.inner.transports[0].events.borrow();
        let sequence_events = events
            .iter()
            .filter(|event| matches!(event, Event::SequenceSubmit(..) | Event::SequenceWait(..)))
            .collect::<Vec<_>>();
        assert_eq!(sequence_events.len(), 72 * world as usize * 2);
        for (segment, group) in sequence_events.chunks(world as usize * 2).enumerate() {
            let count = if segment % 2 == 0 { 10 } else { 5 };
            for rank in 0..world {
                assert_eq!(*group[rank as usize], Event::SequenceSubmit(rank, count));
                assert_eq!(
                    *group[world as usize + rank as usize],
                    Event::SequenceWait(rank, count)
                );
            }
        }
        drop(events);
        pool.commit_batch(&batch, result.completion).unwrap();
        assert!(driver.configure_dispatch_sequences(false).is_err());
        driver.close().unwrap();
    }
}

#[test]
fn wave_modes_select_all_projection_attention_roots_without_host_transposes() {
    let mut pool = pool();
    let mut driver = fixture(8, &pool);
    driver.projection.mode = super::super::EngineeringTpProjectionModeV3::Wave;
    driver.configure_wave_attention(true).unwrap();
    driver.configure_dispatch_sequences(true).unwrap();
    let batch = prepare(&mut pool, 3);
    pool.begin_submission(&batch).unwrap();
    let result = driver.execute(&batch).unwrap();
    assert_eq!(result.choices, [42, 43, 44]);
    assert_eq!(driver.transposed_weight_bytes(), 0);
    assert_eq!(driver.dispatch_counts(), driver.expected_dispatch_counts(3));
    for transport in &driver.inner.transports {
        assert!(
            !transport
                .commands
                .iter()
                .any(|command| matches!(command.kernel, GEMM | PARTIAL | ATTENTION))
        );
        let attention = transport
            .commands
            .iter()
            .filter(|command| command.kernel == "ferric_qwen3_tp_wave_paged_gqa_bf16_v3")
            .count();
        assert_eq!(attention, 36);
        for command in &transport.commands {
            if command.kernel.contains("_wave_gemv_") {
                assert_eq!(
                    command.grid_workgroups,
                    scalar(command, 3) * scalar(command, 4)
                );
            }
        }
    }
    pool.commit_batch(&batch, result.completion).unwrap();
    assert!(driver.configure_wave_attention(false).is_err());
    driver.close().unwrap();
}

#[test]
fn failed_sequence_drains_submitted_ranks_and_cannot_publish_or_resume() {
    let mut pool = pool();
    let mut driver = fixture(8, &pool);
    driver.configure_dispatch_sequences(true).unwrap();
    driver.inner.transports[3].failure = Some(Failure::Wait);
    let batch = prepare(&mut pool, 3);
    pool.begin_submission(&batch).unwrap();
    assert!(driver.execute(&batch).is_err());
    assert!(driver.execute(&batch).is_err());
    assert!(driver.poisoned);
    driver.close().unwrap();
    assert!(driver.inner.closed);
    let events = driver.inner.transports[0].events.borrow();
    for rank in 0..8 {
        assert!(events.contains(&Event::SequenceWait(rank, 10)));
        assert!(events.contains(&Event::Close(rank)));
    }
    assert!(
        driver
            .inner
            .transports
            .iter()
            .all(|rank| rank.pending_sequence.is_none())
    );
}

#[test]
fn device_tp1_has_no_hidden_or_partial_host_copies_and_matches_host_result() {
    for rows in [1, 3, 16] {
        let mut baseline_pool = pool();
        let mut baseline = fixture(1, &baseline_pool);
        let baseline_batch = prepare(&mut baseline_pool, rows);
        baseline_pool.begin_submission(&baseline_batch).unwrap();
        let baseline_result = baseline.execute(&baseline_batch).unwrap();

        let mut device_pool = pool();
        let mut device = fixture(1, &device_pool);
        device
            .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
            .unwrap();
        let device_batch = prepare(&mut device_pool, rows);
        device_pool.begin_submission(&device_batch).unwrap();
        let result = device.execute(&device_batch).unwrap();
        assert_eq!(result.choices, baseline_result.choices);
        assert_eq!(device.dispatch_counts(), [616]);
        let transport = &device.inner.transports[0];
        assert_eq!(
            transport.reads,
            [(device.inner.ranks[0].choice.id, rows as usize * 4)]
        );
        let residual_commands = transport
            .commands
            .iter()
            .filter(|command| command.kernel == "ferric_qwen3_tp_batch_residual_bf16_v3")
            .collect::<Vec<_>>();
        assert_eq!(residual_commands.len(), 72);
        for command in residual_commands {
            for argument in &command.arguments[..3] {
                let EngineeringTpArgumentV1::Buffer { id, .. } = argument else {
                    unreachable!()
                };
                assert!(!transport.writes.iter().any(|(written, _)| written == id));
            }
        }
        let hidden = device.inner.ranks[0].hidden;
        let bytes = &transport.buffers[&hidden.id];
        let expected = &baseline.inner.hidden;
        for (actual, expected) in bytes[..rows as usize * 4096 * 2]
            .chunks_exact(2)
            .zip(expected)
        {
            assert_eq!(u16::from_le_bytes(actual.try_into().unwrap()), *expected);
        }
        assert!(
            bytes[rows as usize * 4096 * 2..]
                .iter()
                .all(|&byte| byte == 0xa5)
        );
        device_pool
            .commit_batch(&device_batch, result.completion)
            .unwrap();
        assert!(
            device
                .configure_reduction(EngineeringTpReductionModeV3::HostStagedV1)
                .is_err()
        );
        device.close().unwrap();
    }
}

#[test]
fn prefill_only_batch_skips_final_norm_head_and_argmax_but_commits_all_kv_rows() {
    let mut pool = pool();
    let mut driver = fixture(8, &pool);
    driver.configure_output_head_pruning(true).unwrap();
    let batch = prepare(&mut pool, 16);
    pool.begin_submission(&batch).unwrap();
    let result = driver.execute_selected(&batch, &[]).unwrap();
    assert!(result.choices.is_empty());
    assert_eq!(
        driver.dispatch_counts(),
        [541, 540, 540, 540, 540, 540, 540, 540]
    );
    let transport = &driver.inner.transports[0];
    assert!(
        !transport
            .commands
            .iter()
            .any(|command| command.kernel == ARGMAX
                || (command.kernel == GEMM && scalar(command, 7) == 6))
    );
    assert!(
        !transport
            .reads
            .iter()
            .any(|(id, _)| *id == driver.inner.ranks[0].choice.id)
    );
    assert_eq!(
        transport
            .commands
            .iter()
            .filter(|command| command.kernel == APPEND)
            .count(),
        36
    );
    assert_eq!(
        transport
            .commands
            .iter()
            .filter(|command| command.kernel == RMSNORM)
            .count(),
        144
    );
    pool.commit_batch(&batch, result.completion).unwrap();
}

#[test]
fn output_selection_rejects_invalid_indices_before_submission_and_preserves_baseline() {
    let mut pool = pool();
    let mut driver = fixture(1, &pool);
    let batch = prepare(&mut pool, 4);
    pool.begin_submission(&batch).unwrap();
    for selection in [&[4][..], &[1, 1][..], &[3, 1][..]] {
        assert!(driver.execute_selected(&batch, selection).is_err());
        assert!(driver.inner.transports[0].commands.is_empty());
    }
    let result = driver.execute_selected(&batch, &[1, 3]).unwrap();
    assert_eq!(result.choices, [43, 45]);
    let head = driver.inner.transports[0]
        .commands
        .iter()
        .find(|command| command.kernel == GEMM && scalar(command, 7) == 6)
        .unwrap();
    assert_eq!(scalar(head, 3), 4);
    pool.commit_batch(&batch, result.completion).unwrap();
}

#[test]
fn reused_host_workspace_preserves_tp1_tp2_tp8_values_dispatches_and_copy_extents() {
    for world in [1, 2, 8] {
        let mut baseline_pool = pool();
        let mut baseline = fixture(world, &baseline_pool);
        let baseline_batch = prepare(&mut baseline_pool, 3);
        baseline_pool.begin_submission(&baseline_batch).unwrap();
        let baseline_result = baseline.execute(&baseline_batch).unwrap();

        let mut reused_pool = pool();
        let mut reused = fixture(world, &reused_pool);
        reused
            .configure_reduction(EngineeringTpReductionModeV3::HostStagedReuseV3)
            .unwrap();
        let reused_batch = prepare(&mut reused_pool, 3);
        reused_pool.begin_submission(&reused_batch).unwrap();
        let result = reused.execute(&reused_batch).unwrap();
        assert_eq!(result.choices, baseline_result.choices);
        assert_eq!(reused.inner.hidden, baseline.inner.hidden);
        assert_eq!(reused.dispatch_counts(), baseline.dispatch_counts());
        for (left, right) in reused
            .inner
            .transports
            .iter()
            .zip(&baseline.inner.transports)
        {
            assert_eq!(left.reads, right.reads);
            assert_eq!(left.writes, right.writes);
            assert_eq!(left.commands, right.commands);
            assert_eq!(left.buffers, right.buffers);
        }
        reused_pool
            .commit_batch(&reused_batch, result.completion)
            .unwrap();
        reused.close().unwrap();
    }
}

#[test]
fn device_mode_rejects_peer_worlds_without_allocating_or_dispatching() {
    for world in [2, 8] {
        let pool = pool();
        let mut driver = fixture(world, &pool);
        let counts = driver
            .inner
            .transports
            .iter()
            .map(|t| t.next)
            .collect::<Vec<_>>();
        assert!(
            driver
                .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
                .unwrap_err()
                .contains("peer collectives are unsupported")
        );
        assert_eq!(
            driver.reduction_mode(),
            EngineeringTpReductionModeV3::HostStagedV1
        );
        assert_eq!(
            driver
                .inner
                .transports
                .iter()
                .map(|t| t.next)
                .collect::<Vec<_>>(),
            counts
        );
        assert!(
            driver
                .inner
                .transports
                .iter()
                .all(|t| t.commands.is_empty())
        );
        driver.close().unwrap();
    }
}

#[test]
fn optimized_reduction_failures_poison_without_successful_completion() {
    for (mode, failure) in [
        (
            EngineeringTpReductionModeV3::HostStagedReuseV3,
            Failure::NonfinitePartial,
        ),
        (
            EngineeringTpReductionModeV3::DeviceTp1V3,
            Failure::NonfinitePartial,
        ),
        (
            EngineeringTpReductionModeV3::DeviceTp1V3,
            Failure::ResidualWait,
        ),
    ] {
        let mut pool = pool();
        let mut driver = fixture(1, &pool);
        driver.configure_reduction(mode).unwrap();
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare(&mut pool, 3);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute(&batch).is_err());
        assert_eq!(driver.completed_batches(), 0);
        assert!(driver.poisoned);
        assert!(driver.execute(&batch).err().unwrap().contains("poisoned"));
        driver.close().unwrap();
    }
}

#[test]
fn device_extra_packets_and_destination_alias_are_rejected_before_submission() {
    let mut pool = pool();
    let mut driver = fixture(1, &pool);
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    let batch = prepare(&mut pool, 1);
    driver.completed_batches = fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1 / 616;
    assert!(driver.execute(&batch).err().unwrap().contains("ring"));
    assert!(driver.inner.transports[0].commands.is_empty());
    assert!(driver.inner.transports[0].reads.is_empty());
    assert!(driver.inner.transports[0].writes.is_empty());
    driver.inner.reduction =
        super::super::ReductionWorkspace::DeviceTp1(driver.inner.ranks[0].hidden);
    assert!(
        driver
            .inner
            .reduce_device_tp1(0, Qwen3TensorParallelCollectiveV1::AttentionOutputSum)
            .unwrap_err()
            .contains("ownership")
    );
    assert!(driver.inner.transports[0].commands.is_empty());
    driver.close().unwrap();
}

#[test]
fn reused_host_arithmetic_keeps_rank_order_and_fails_before_broadcast_on_nonfinite() {
    let cases = [
        (
            [
                16_777_216.0_f32,
                1.0,
                -16_777_216.0,
                2.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
            0_u16,
        ),
        ([1.0, 0.003_906_25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0),
        ([0.0; 8], 0x8000),
        ([f32::MAX; 8], 0),
        ([f32::NAN; 8], 0),
        ([0.0; 8], 0x7fc0),
    ];
    for (values, residual) in cases {
        let pool = pool();
        let mut driver = fixture(8, &pool);
        driver
            .configure_reduction(EngineeringTpReductionModeV3::HostStagedReuseV3)
            .unwrap();
        driver.inner.hidden = vec![residual; 4096];
        for (rank, transport) in driver.inner.ranks.iter().zip(&mut driver.inner.transports) {
            for bytes in
                transport.buffers.get_mut(&rank.partial.id).unwrap()[..4096 * 4].chunks_exact_mut(4)
            {
                bytes.copy_from_slice(&values[rank.geometry.rank as usize].to_le_bytes());
            }
        }
        let vectors = values.map(|value| [value]);
        let partials = vectors
            .iter()
            .enumerate()
            .map(|(rank, values)| HostStagedPartialV1 {
                rank: u32::try_from(rank).unwrap(),
                values,
            })
            .collect::<Vec<_>>();
        let expected = reduce_residual_bf16_v1(8, &partials, &[residual]);
        let result = driver
            .inner
            .reduce_host_reused(0, Qwen3TensorParallelCollectiveV1::AttentionOutputSum);
        match expected {
            Ok(expected) => {
                result.unwrap();
                assert!(
                    driver
                        .inner
                        .hidden
                        .iter()
                        .all(|value| *value == expected[0])
                );
            }
            Err(expected) => {
                assert_eq!(result.unwrap_err(), expected);
                assert!(
                    driver
                        .inner
                        .transports
                        .iter()
                        .all(|transport| transport.writes.is_empty())
                );
                assert!(driver.inner.hidden.iter().all(|value| *value == residual));
            }
        }
        driver.close().unwrap();
    }
}

#[test]
fn real_multirow_schedule_submits_all_ranks_before_waiting_and_preserves_tails() {
    for world in [1, 2, 8] {
        let mut pool = pool();
        let mut driver = fixture(world, &pool);
        let batch = prepare(&mut pool, 3);
        pool.begin_submission(&batch).unwrap();
        let result = driver.execute(&batch).unwrap();
        assert_eq!(result.choices, [42, 43, 44]);
        pool.commit_batch(&batch, result.completion).unwrap();
        assert_eq!(driver.completed_batches(), 1);
        assert_eq!(driver.dispatch_counts()[0], 544);
        assert!(
            driver.dispatch_counts()[1..]
                .iter()
                .all(|&count| count == 540)
        );
        for (rank_index, rank) in driver.inner.ranks.iter().enumerate() {
            let transport = &driver.inner.transports[rank_index];
            assert_eq!(
                transport.u32_values(driver.positions[rank_index].id, 3),
                [0, 1, 2]
            );
            let table = transport.u32_values(driver.page_tables[rank_index].id, 12);
            for (index, row) in batch.rows().iter().enumerate() {
                assert_eq!(table[index * 4], row.physical_pages()[0]);
                assert_eq!(&table[index * 4 + 1..index * 4 + 4], &[u32::MAX; 3]);
            }
            for tensor in [rank.hidden, rank.normalized, rank.partial] {
                let active = 3 * 4096 * tensor.element_bytes as usize;
                assert!(
                    transport.buffers[&tensor.id][active..]
                        .iter()
                        .all(|&byte| byte == 0xa5)
                );
            }
            assert_eq!(
                transport
                    .reads
                    .iter()
                    .filter(|(id, _)| *id == rank.partial.id)
                    .count(),
                72
            );
            assert!(
                transport
                    .reads
                    .iter()
                    .filter(|(id, _)| *id == rank.partial.id)
                    .all(|(_, bytes)| *bytes == 3 * 4096 * 4)
            );
            assert!(
                transport
                    .writes
                    .iter()
                    .filter(|(id, _)| *id == rank.hidden.id)
                    .all(|(_, bytes)| *bytes == 3 * 4096 * 2)
            );
            assert_eq!(
                transport
                    .commands
                    .iter()
                    .filter(|command| command.kernel == APPEND)
                    .count(),
                36
            );
            for command in transport
                .commands
                .iter()
                .filter(|command| command.kernel == GEMM || command.kernel == PARTIAL)
            {
                assert_eq!(scalar(command, 3), 3);
                assert_eq!(command.grid_workgroups, scalar(command, 4) / 16);
            }
        }
        let values = (0..world)
            .map(|rank| {
                (0..3)
                    .flat_map(|row| vec![exact_f32((rank + 1) * (row + 1)) / 1024.0; 4096])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let partials = values
            .iter()
            .enumerate()
            .map(|(rank, values)| HostStagedPartialV1 {
                rank: u32::try_from(rank).unwrap(),
                values,
            })
            .collect::<Vec<_>>();
        let mut expected = (1..=3_u32)
            .flat_map(|value| vec![u16::try_from(exact_f32(value).to_bits() >> 16).unwrap(); 4096])
            .collect::<Vec<_>>();
        for _ in 0..72 {
            expected = reduce_residual_bf16_v1(world, &partials, &expected).unwrap();
        }
        assert_eq!(driver.inner.hidden, expected);
        let events = driver.inner.transports[0].events.borrow();
        let mut stages = 0;
        for (index, event) in events.iter().enumerate() {
            if *event != Event::Submit(0, PARTIAL) {
                continue;
            }
            stages += 1;
            for rank in 0..world {
                assert_eq!(events[index + rank as usize], Event::Submit(rank, PARTIAL));
                assert_eq!(
                    events[index + world as usize + rank as usize],
                    Event::Wait(rank, PARTIAL)
                );
            }
        }
        assert_eq!(stages, 72);
    }
}

#[test]
fn host_timing_preserves_driver_choices_counts_and_transport_order() {
    let mut observations = Vec::new();
    for enabled in [false, true] {
        let mut pool = pool();
        let mut driver = fixture(2, &pool);
        let timing = if enabled {
            crate::host_timing::HostTiming::enabled()
        } else {
            crate::host_timing::HostTiming::default()
        };
        driver.configure_host_timing(timing.clone()).unwrap();
        let batch = prepare(&mut pool, 3);
        pool.begin_submission(&batch).unwrap();
        let result = driver.execute(&batch).unwrap();
        assert_eq!(result.choices, [42, 43, 44]);
        pool.commit_batch(&batch, result.completion).unwrap();
        assert!(driver.configure_host_timing(timing.clone()).is_err());
        driver.close().unwrap();
        observations.push((
            driver.dispatch_counts(),
            driver.inner.transports[0].events.borrow().clone(),
        ));
        if enabled {
            let snapshot = timing.snapshot();
            assert_eq!(snapshot["incomplete"], false);
            assert_eq!(snapshot["active_records"], 0);
            assert_attention_operation_spans(&snapshot, batch.id(), 36);
            let records = snapshot["records"].as_array().unwrap();
            for (label, count) in [
                ("batch", 1),
                ("attention", 36),
                ("feed_forward", 36),
                ("collective_attention", 36),
                ("collective_feed_forward", 36),
                ("output_readback", 1),
            ] {
                let record = records
                    .iter()
                    .find(|record| record["label"] == label)
                    .unwrap();
                assert_eq!(record["batch"], batch.id());
                assert_eq!(record["count"], count);
                assert_eq!(record["phase"], label);
            }
        } else {
            assert!(timing.snapshot().is_null());
        }
    }
    assert_eq!(observations[0], observations[1]);
}

fn assert_attention_operation_spans(snapshot: &serde_json::Value, batch: u64, count: u64) {
    let records = snapshot["records"].as_array().unwrap();
    for label in [
        "attention_input_norm",
        "attention_qkv_projection",
        "attention_qk_norm",
        "attention_rope",
        "attention_kv_append",
        "attention_gqa_math",
        "attention_output_projection",
    ] {
        let matching = records
            .iter()
            .filter(|record| record["label"] == label)
            .collect::<Vec<_>>();
        assert_eq!(matching.len(), 1, "{label}");
        let record = matching[0];
        assert_eq!(record["batch"], batch);
        assert_eq!(record["phase"], "attention");
        assert_eq!(record["category"], "span");
        assert!(record["rank"].is_null());
        assert_eq!(record["count"], count);
    }
    let dispatch = records
        .iter()
        .find(|record| record["phase"] == "attention" && record["label"] == "dispatch_each")
        .unwrap();
    assert_eq!(dispatch["count"], count * 10);
}

#[test]
fn attention_operation_spans_restore_phase_after_submit_and_wait_failures() {
    for failure in [Failure::Submit, Failure::Wait] {
        let mut pool = pool();
        let mut driver = fixture(1, &pool);
        let timing = crate::host_timing::HostTiming::enabled();
        driver.configure_host_timing(timing.clone()).unwrap();
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute(&batch).is_err());
        pool.quarantine_batch(&batch).unwrap();
        driver.close().unwrap();
        drop(timing.span("after_failed_attention_batch", None));
        let snapshot = timing.snapshot();
        assert_eq!(snapshot["incomplete"], false);
        assert_eq!(snapshot["active_records"], 0);
        assert_attention_operation_spans(&snapshot, batch.id(), 1);
        let restored = snapshot["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| record["label"] == "after_failed_attention_batch")
            .unwrap();
        assert_eq!(restored["phase"], "controller");
        assert!(restored["batch"].is_null());
    }
}

#[test]
fn foreign_metadata_and_conservative_ring_boundary_reject_before_any_io() {
    let mut owned = pool();
    let mut driver = fixture(2, &owned);
    let mut foreign = pool();
    let alien = prepare(&mut foreign, 1);
    assert!(driver.execute(&alien).err().unwrap().contains("foreign"));
    let batch = prepare(&mut owned, 1);
    driver.completed_batches = fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1 / 544;
    assert!(driver.execute(&batch).err().unwrap().contains("ring"));
    assert_eq!(driver.last_batch, 0);
    assert!(!driver.poisoned);
    assert!(
        driver
            .inner
            .transports
            .iter()
            .all(|rank| rank.commands.is_empty() && rank.writes.is_empty())
    );
    driver.completed_batches = 0;
    owned.begin_submission(&batch).unwrap();
    let result = driver.execute(&batch).unwrap();
    owned.commit_batch(&batch, result.completion).unwrap();
    let counts = driver.dispatch_counts();
    assert!(driver.execute(&batch).err().unwrap().contains("stale"));
    assert_eq!(driver.dispatch_counts(), counts);
    driver.close().unwrap();
    assert!(driver.execute(&batch).err().unwrap().contains("closed"));
}

#[test]
fn partial_submit_and_wait_failures_drain_every_submitted_rank_and_poison() {
    for fail_on_wait in [false, true] {
        let mut pool = pool();
        let mut driver = fixture(8, &pool);
        driver.inner.transports[3].failure = Some(if fail_on_wait {
            Failure::Wait
        } else {
            Failure::Submit
        });
        let batch = prepare(&mut pool, 2);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute(&batch).err().unwrap().contains("injected"));
        assert!(driver.poisoned);
        assert_eq!(driver.completed_batches(), 0);
        assert!(
            driver
                .inner
                .transports
                .iter()
                .all(|rank| rank.pending.is_none())
        );
        assert_eq!(
            pool.committed_position(batch.rows()[0].sequence()).unwrap(),
            0
        );
        pool.quarantine_batch(&batch).unwrap();
        assert_eq!(pool.stats().quarantined_pages, 4);
        let counts = driver.dispatch_counts();
        assert!(driver.execute(&batch).is_err());
        assert_eq!(driver.dispatch_counts(), counts);
        driver.close().unwrap();
        let events = driver.inner.transports[0].events.borrow();
        for rank in 0..if fail_on_wait { 8 } else { 3 } {
            assert!(events.contains(&Event::Wait(rank, PARTIAL)));
        }
        for rank in 0..8 {
            assert!(events.contains(&Event::Close(rank)));
        }
    }
}

#[test]
fn metadata_copy_nonfinite_reduction_and_invalid_choice_fail_terminally() {
    for failure in 0..3 {
        let mut pool = pool();
        let mut driver = fixture(2, &pool);
        match failure {
            0 => driver.inner.transports[0].failure = Some(Failure::Write),
            1 => driver.inner.transports[1].failure = Some(Failure::NonfinitePartial),
            2 => driver.inner.transports[0].failure = Some(Failure::BadChoice),
            _ => unreachable!(),
        }
        let batch = prepare(&mut pool, 2);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute(&batch).is_err());
        assert!(driver.poisoned);
        assert_eq!(driver.completed_batches(), 0);
        assert!(driver.execute(&batch).err().unwrap().contains("poisoned"));
        pool.quarantine_batch(&batch).unwrap();
        driver.close().unwrap();
    }
}

#[test]
fn later_mixed_request_batch_keeps_row_identity_and_absolute_positions() {
    let mut pool = pool();
    let mut driver = fixture(2, &pool);
    let first = prepare(&mut pool, 16);
    let continued = first.rows()[0].sequence();
    pool.begin_submission(&first).unwrap();
    let result = driver.execute(&first).unwrap();
    assert_eq!(result.choices, (42..58).collect::<Vec<_>>());
    pool.commit_batch(&first, result.completion).unwrap();
    let fresh = pool
        .open_sequence(pool.scope(), &[99], 0)
        .unwrap()
        .sequence();
    let second = pool
        .reserve_batch(&[
            EngineeringTpPageRowV1 {
                sequence: fresh,
                token: 99,
                position: 0,
            },
            EngineeringTpPageRowV1 {
                sequence: continued,
                token: 100,
                position: 16,
            },
        ])
        .unwrap();
    assert_ne!(
        second.rows()[0].writable_physical_page(),
        second.rows()[1].writable_physical_page()
    );
    pool.begin_submission(&second).unwrap();
    let result = driver.execute(&second).unwrap();
    assert_eq!(result.choices, [42, 43]);
    pool.commit_batch(&second, result.completion).unwrap();
    assert_eq!(pool.committed_position(fresh).unwrap(), 1);
    assert_eq!(pool.committed_position(continued).unwrap(), 17);
    assert_eq!(driver.completed_batches(), 2);
    assert_eq!(driver.dispatch_counts(), [1088, 1080]);
    for (rank, transport) in driver.inner.transports.iter().enumerate() {
        assert_eq!(transport.u32_values(driver.positions[rank].id, 2), [0, 16]);
        let table = transport.u32_values(driver.page_tables[rank].id, 8);
        assert_eq!(table[0], second.rows()[0].physical_pages()[0]);
        assert_eq!(&table[4..6], second.rows()[1].physical_pages());
        let attention = transport
            .commands
            .iter()
            .rev()
            .find(|command| command.kernel == ATTENTION)
            .unwrap();
        assert_eq!(scalar(attention, 10), 17);
    }
}

#[test]
fn token_scope_geometry_and_counter_boundaries_reject_without_submission() {
    let mut pool = pool();
    let mut driver = fixture(1, &pool);
    let sequence = pool
        .open_sequence(pool.scope(), &[u32::MAX], 0)
        .unwrap()
        .sequence();
    let invalid = pool
        .reserve_batch(&[EngineeringTpPageRowV1 {
            sequence,
            token: u32::MAX,
            position: 0,
        }])
        .unwrap();
    assert!(driver.execute(&invalid).err().unwrap().contains("geometry"));
    pool.abort_batch(&invalid).unwrap();
    pool.cancel_sequence(sequence).unwrap();
    let batch = prepare(&mut pool, 1);
    for boundary in 0..4 {
        match boundary {
            0 => driver.scope.session[0] ^= 1,
            1 => driver.context_tokens = 63,
            2 => driver.physical_pages = 3,
            3 => driver.table_stride = 3,
            _ => unreachable!(),
        }
        assert!(driver.execute(&batch).err().unwrap().contains("foreign"));
        driver.scope = pool.scope();
        driver.context_tokens = 64;
        driver.physical_pages = 4;
        driver.table_stride = 4;
    }
    driver.completed_batches = u64::MAX;
    assert!(driver.execute(&batch).err().unwrap().contains("overflow"));
    assert!(!driver.poisoned);
    assert_eq!(driver.last_batch, 0);
    assert!(driver.inner.transports[0].commands.is_empty());
    assert!(driver.inner.transports[0].writes.is_empty());
}
