//! Tests actual driver transitions with host-simulated collective outputs.
//! These are not GPU arithmetic, visibility, or performance evidence.

use super::*;
use crate::tp_execution::{
    EngineeringTpBufferAccessV1, HostStagedPartialV1, Qwen3TensorParallelCollectiveStateV1,
    Qwen3TensorParallelPlanV1, TensorParallelSequenceV1, allocate_rank_storage,
    reduce_residual_bf16_v1,
};
use ferric_spec::{Identity, ModelConfig, Qwen3ModelRole};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

const COPY32: &str = "ferric_qwen3_tp_batch32_peer_copy_bf16_v6";
const REDUCE32: &str = "ferric_qwen3_tp_batch32_peer_ordered_residual_bf16_v6";

#[derive(Default)]
struct Memory {
    next: u64,
    buffers: BTreeMap<u64, (u32, bool, Vec<u8>)>,
    events: Vec<(bool, u32, &'static str)>,
    commands: Vec<(u32, EngineeringTpDispatchV1)>,
    closed: Vec<u32>,
}

struct Transport {
    memory: Rc<RefCell<Memory>>,
    rank: u32,
    world: u32,
    pid: u32,
    pending: Option<EngineeringTpDispatchV1>,
    fail_submit: bool,
    fail_wait: bool,
}

fn tensor(command: &EngineeringTpDispatchV1, index: usize) -> (u64, usize, usize) {
    let EngineeringTpArgumentV1::Buffer {
        id,
        elements,
        element_bytes,
        offset: 0,
        ..
    } = command.arguments[index]
    else {
        panic!("slice argument required");
    };
    (id, elements, element_bytes as usize)
}

impl Transport {
    fn alloc(&mut self, bytes: usize, shared: bool) -> u64 {
        let mut memory = self.memory.borrow_mut();
        memory.next += 1;
        let id = memory.next;
        memory
            .buffers
            .insert(id, (self.rank, shared, vec![0xa5; bytes]));
        id
    }
}

impl EngineeringTpRankTransportV1 for Transport {
    fn peer_group_rank(&self) -> Option<(u32, u32, u32)> {
        Some((self.pid, self.rank, self.world))
    }
    fn allocate(&mut self, bytes: usize) -> TpResult<u64> {
        Ok(self.alloc(bytes, false))
    }
    fn allocate_peer_readable(&mut self, bytes: usize) -> TpResult<u64> {
        Ok(self.alloc(bytes, true))
    }
    fn read(&mut self, _: u64, _: usize, _: &mut [u8]) -> TpResult<()> {
        panic!("peer reduction read host tensors")
    }
    fn write(&mut self, _: u64, _: usize, _: &[u8]) -> TpResult<()> {
        panic!("peer reduction wrote host tensors")
    }
    fn submit(&mut self, command: &EngineeringTpDispatchV1) -> TpResult<()> {
        assert!(self.pending.is_none());
        if self.fail_submit {
            return Err("injected peer submission failure".into());
        }
        let mut memory = self.memory.borrow_mut();
        for argument in &command.arguments {
            if let EngineeringTpArgumentV1::Buffer {
                id,
                offset,
                elements,
                element_bytes,
                access,
            } = argument
            {
                let (owner, shared, bytes) = &memory.buffers[id];
                assert!(*offset + *elements * *element_bytes as usize <= bytes.len());
                assert!(
                    *owner == self.rank || *shared && *access == EngineeringTpBufferAccessV1::Read
                );
            }
        }
        memory.events.push((true, self.rank, command.kernel));
        memory.commands.push((self.rank, command.clone()));
        self.pending = Some(command.clone());
        Ok(())
    }
    fn wait(&mut self) -> TpResult<()> {
        let command = self.pending.take().unwrap();
        let mut memory = self.memory.borrow_mut();
        memory.events.push((false, self.rank, command.kernel));
        if self.fail_wait {
            return Err("injected peer completion failure".into());
        }
        let (output, values) = match command.kernel {
            COPY | COPY32 => {
                let input = tensor(&command, 0);
                let output = tensor(&command, 1);
                assert_ne!(input.0, output.0);
                (output, memory.buffers[&input.0].2[..input.1 * 2].to_vec())
            }
            REDUCE | REDUCE32 => {
                let EngineeringTpArgumentV1::U32(world) = command.arguments[11] else {
                    panic!()
                };
                let partials = (0..world as usize)
                    .map(|index| {
                        let input = tensor(&command, index);
                        assert_eq!(input.2, 4);
                        memory.buffers[&input.0].2[..input.1 * 4]
                            .chunks_exact(4)
                            .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                for index in world as usize..8 {
                    assert_eq!(tensor(&command, index).1, 0);
                }
                let residual = tensor(&command, 8);
                let output = tensor(&command, 9);
                let residual = memory.buffers[&residual.0].2[..residual.1 * 2]
                    .chunks_exact(2)
                    .map(|bytes| u16::from_le_bytes(bytes.try_into().unwrap()))
                    .collect::<Vec<_>>();
                let parts = partials
                    .iter()
                    .enumerate()
                    .map(|(rank, values)| HostStagedPartialV1 {
                        rank: u32::try_from(rank).unwrap(),
                        values,
                    })
                    .collect::<Vec<_>>();
                let values = reduce_residual_bf16_v1(world, &parts, &residual)?;
                (
                    output,
                    values.into_iter().flat_map(u16::to_le_bytes).collect(),
                )
            }
            _ => panic!("unexpected peer kernel"),
        };
        memory.buffers.get_mut(&output.0).unwrap().2[..values.len()].copy_from_slice(&values);
        Ok(())
    }
    fn close(&mut self) -> TpResult<()> {
        self.memory.borrow_mut().closed.push(self.rank);
        self.pending = None;
        Ok(())
    }
}

fn fixture(world: u32) -> EngineeringTpExecutionV1<Transport> {
    fixture_with_capacity(world, 16)
}

fn fixture_with_capacity(world: u32, row_capacity: u32) -> EngineeringTpExecutionV1<Transport> {
    let model = ModelConfig {
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
    };
    let plan = Qwen3TensorParallelPlanV1::new(model, world).unwrap();
    let memory = Rc::new(RefCell::new(Memory::default()));
    let mut transports = (0..world)
        .map(|rank| Transport {
            memory: Rc::clone(&memory),
            rank,
            world,
            pid: 123,
            pending: None,
            fail_submit: false,
            fail_wait: false,
        })
        .collect::<Vec<_>>();
    let ranks = transports
        .iter_mut()
        .enumerate()
        .map(|(rank, transport)| {
            allocate_rank_storage(
                transport,
                &plan,
                u32::try_from(rank).unwrap(),
                1,
                row_capacity,
            )
            .unwrap()
        })
        .collect();
    EngineeringTpExecutionV1 {
        transports,
        ranks,
        plan,
        sequence: TensorParallelSequenceV1::new(64, model.vocabulary_size).unwrap(),
        collective: Qwen3TensorParallelCollectiveStateV1::new(&plan, 0, 0),
        capacity: 64,
        row_capacity,
        large_kv: false,
        draft_v10: false,
        hidden: vec![0; 4096 * row_capacity as usize],
        reduction: ReductionWorkspace::Baseline,
        sequences: None,
        ordered_batches: None,
        timing: crate::host_timing::HostTiming::default(),
        closed: false,
    }
}

fn initialize(execution: &mut EngineeringTpExecutionV1<Transport>, rows: usize) {
    execution.hidden.resize(rows * 4096, 0);
    let mut memory = execution.transports[0].memory.borrow_mut();
    for bytes in memory
        .buffers
        .get_mut(&execution.ranks[0].hidden.id)
        .unwrap()
        .2[..rows * 8192]
        .chunks_exact_mut(2)
    {
        bytes.copy_from_slice(&0x3f80_u16.to_le_bytes());
    }
    for (index, rank) in execution.ranks.iter().enumerate() {
        let value = [
            16_777_216.0_f32,
            1.0,
            -16_777_216.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ][index];
        for bytes in
            memory.buffers.get_mut(&rank.partial.id).unwrap().2[..rows * 16384].chunks_exact_mut(4)
        {
            bytes.copy_from_slice(&value.to_le_bytes());
        }
    }
}

#[test]
fn exact_rank_order_single_residual_active_extents_and_completed_embedding_without_host_copies() {
    for world in [2, 8] {
        for (capacity, rows) in [
            (16, 1),
            (16, 3),
            (16, 16),
            (32, 1),
            (32, 17),
            (32, 31),
            (32, 32),
        ] {
            let mut execution = fixture_with_capacity(world, capacity);
            execution
                .configure_reduction(super::super::EngineeringTpReductionModeV3::DevicePeerV4)
                .unwrap();
            initialize(&mut execution, rows);
            execution.initialize_hidden_from_embedding().unwrap();
            execution
                .reduce(0, Qwen3TensorParallelCollectiveV1::AttentionOutputSum)
                .unwrap();
            let memory = execution.transports[0].memory.borrow();
            let expected = if world == 8 { 0x3f80_u16 } else { 0x4b80_u16 };
            for rank in &execution.ranks {
                let bytes = &memory.buffers[&rank.hidden.id].2;
                assert!(
                    bytes[..rows * 8192]
                        .chunks_exact(2)
                        .all(|b| b == expected.to_le_bytes())
                );
                assert!(bytes[rows * 8192..].iter().all(|&b| b == 0xa5));
                assert_eq!(rank.dispatches, if rank.geometry.rank == 0 { 1 } else { 2 });
            }
            let (copy, reduce) = if capacity == 32 {
                (COPY32, REDUCE32)
            } else {
                (COPY, REDUCE)
            };
            assert_eq!(memory.commands.len(), 2 * world as usize - 1);
            for (rank, command) in &memory.commands {
                assert_eq!(command.grid_workgroups, u32::try_from(rows).unwrap() * 64);
                assert!(command.kernel == copy || command.kernel == reduce);
                if command.kernel == copy {
                    assert_ne!(*rank, 0);
                    assert_eq!(tensor(command, 0).1, rows * 4096);
                    assert_eq!(tensor(command, 1).1, rows * 4096);
                } else {
                    for index in 0..world as usize {
                        assert_eq!(tensor(command, index).1, rows * 4096);
                    }
                    assert_eq!(tensor(command, 8).1, rows * 4096);
                    assert_eq!(tensor(command, 9).1, rows * 4096);
                }
            }
            let reductions = memory
                .events
                .iter()
                .filter(|event| event.2 == reduce)
                .copied()
                .collect::<Vec<_>>();
            assert!(reductions[..world as usize].iter().all(|event| event.0));
            assert!(reductions[world as usize..].iter().all(|event| !event.0));
            assert_eq!(
                execution.collective.expected().operation,
                Qwen3TensorParallelCollectiveV1::FeedForwardDownSum
            );
        }
    }
}

#[test]
fn group_mismatch_and_world_one_reject_before_allocating_peer_buffers() {
    for world in [1, 2, 8] {
        let mut execution = fixture(world);
        if world != 1 {
            execution.transports[1].pid = 124;
        }
        let before = execution.transports[0].memory.borrow().next;
        assert!(
            execution
                .configure_reduction(super::super::EngineeringTpReductionModeV3::DevicePeerV4)
                .is_err()
        );
        assert_eq!(before, execution.transports[0].memory.borrow().next);
    }
}

#[test]
fn partial_submit_or_wait_failure_drains_prior_ranks_without_swapping_hidden() {
    for (capacity, rows) in [(16, 1), (32, 32)] {
        for submission in [false, true] {
            let mut execution = fixture_with_capacity(8, capacity);
            execution
                .configure_reduction(super::super::EngineeringTpReductionModeV3::DevicePeerV4)
                .unwrap();
            initialize(&mut execution, rows);
            execution.initialize_hidden_from_embedding().unwrap();
            let previous = execution
                .ranks
                .iter()
                .map(|rank| rank.hidden.id)
                .collect::<Vec<_>>();
            execution.transports[3].fail_submit = submission;
            execution.transports[3].fail_wait = !submission;
            assert!(
                execution
                    .reduce(0, Qwen3TensorParallelCollectiveV1::AttentionOutputSum)
                    .is_err()
            );
            assert_eq!(
                previous,
                execution
                    .ranks
                    .iter()
                    .map(|rank| rank.hidden.id)
                    .collect::<Vec<_>>()
            );
            assert!(
                execution
                    .transports
                    .iter()
                    .all(|transport| transport.pending.is_none())
            );
            assert_eq!(
                execution.collective.expected().operation,
                Qwen3TensorParallelCollectiveV1::AttentionOutputSum
            );
            execution.close().unwrap();
            assert_eq!(
                execution.transports[0].memory.borrow().closed,
                (0..8).collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn nonfinite_partial_rejects_without_publishing_any_rank_hidden_state() {
    let mut execution = fixture(2);
    execution
        .configure_reduction(super::super::EngineeringTpReductionModeV3::DevicePeerV4)
        .unwrap();
    initialize(&mut execution, 1);
    execution.initialize_hidden_from_embedding().unwrap();
    let old = execution
        .ranks
        .iter()
        .map(|rank| rank.hidden.id)
        .collect::<Vec<_>>();
    execution.transports[0]
        .memory
        .borrow_mut()
        .buffers
        .get_mut(&execution.ranks[1].partial.id)
        .unwrap()
        .2[..4]
        .copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(
        execution
            .reduce(0, Qwen3TensorParallelCollectiveV1::AttentionOutputSum)
            .is_err()
    );
    assert_eq!(
        old,
        execution
            .ranks
            .iter()
            .map(|rank| rank.hidden.id)
            .collect::<Vec<_>>()
    );
    assert!(
        execution
            .transports
            .iter()
            .all(|rank| rank.pending.is_none())
    );
}
