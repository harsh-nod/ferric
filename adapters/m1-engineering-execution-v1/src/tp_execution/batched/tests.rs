//! Runs the actual batched host driver against a recording byte transport.
//! Synthetic weights and outputs are not GPU kernel emulation or numerical
//! evidence. These tests exercise scheduling, active extents, ownership handoff,
//! host reductions, and terminal failure behavior without a physical device.

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
}

struct Recording {
    rank: u32,
    buffers: BTreeMap<u64, Vec<u8>>,
    next: u64,
    pending: Option<EngineeringTpDispatchV1>,
    events: Rc<RefCell<Vec<Event>>>,
    commands: Vec<EngineeringTpDispatchV1>,
    reads: Vec<(u64, usize)>,
    writes: Vec<(u64, usize)>,
    failure: Option<Failure>,
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
    fn allocate(&mut self, byte_len: usize) -> TpResult<u64> {
        assert!(self.pending.is_none());
        let id = self.next;
        self.next += 1;
        self.buffers.insert(id, vec![0xa5; byte_len]);
        Ok(id)
    }

    fn write(&mut self, id: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        assert!(self.pending.is_none());
        if self.failure == Some(Failure::Write) {
            return Err("injected metadata write failure".into());
        }
        self.buffers.get_mut(&id).unwrap()[offset..offset + bytes.len()].copy_from_slice(bytes);
        self.writes.push((id, bytes.len()));
        Ok(())
    }

    fn read(&mut self, id: u64, offset: usize, bytes: &mut [u8]) -> TpResult<()> {
        assert!(self.pending.is_none());
        bytes.copy_from_slice(&self.buffers[&id][offset..offset + bytes.len()]);
        self.reads.push((id, bytes.len()));
        Ok(())
    }

    fn submit(&mut self, command: &EngineeringTpDispatchV1) -> TpResult<()> {
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
        if self.failure == Some(Failure::Submit) && command.kernel == PARTIAL {
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
        self.events
            .borrow_mut()
            .push(Event::Wait(self.rank, command.kernel));
        if self.failure == Some(Failure::Wait) && command.kernel == PARTIAL {
            return Err("injected partial completion failure".into());
        }
        match command.kernel {
            EMBEDDING => self.output(&command, 2, scalar(&command, 3), 4096, |row| {
                u16::try_from(exact_f32(row + 1).to_bits() >> 16)
                    .unwrap()
                    .to_le_bytes()
                    .to_vec()
            }),
            GEMM => self.output(
                &command,
                2,
                scalar(&command, 3),
                scalar(&command, 4),
                |_| vec![0; 2],
            ),
            PARTIAL => {
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
                12_288 / scalar(&command, 4),
                |_| vec![0; 2],
            ),
            ROPE => {
                let rows = scalar(&command, 7);
                let world = scalar(&command, 8);
                self.output(&command, 5, rows, 4096 / world, |_| vec![0; 2]);
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
            ATTENTION => {
                let rows = scalar(&command, 6);
                let world = scalar(&command, 7);
                let context = scalar(&command, 10);
                let positions = self.u32_values(buffer(&command, 3).0, rows as usize);
                assert_eq!(context, positions.iter().max().unwrap() + 1);
                assert_eq!(command.grid_workgroups, rows * 32 / world);
                self.output(&command, 5, rows, 4096 / world, |_| vec![0; 2]);
            }
            ARGMAX => {
                let bad = self.failure == Some(Failure::BadChoice);
                self.output(&command, 1, scalar(&command, 2), 1, |row| {
                    (if bad { 151_936 } else { 42 + row })
                        .to_le_bytes()
                        .to_vec()
                });
            }
            other => panic!("unexpected batch kernel {other}"),
        }
        Ok(())
    }

    fn close(&mut self) -> TpResult<()> {
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
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut transports = (0..world)
        .map(|rank| Recording {
            rank,
            buffers: BTreeMap::new(),
            next: 1,
            pending: None,
            events: events.clone(),
            commands: Vec::new(),
            reads: Vec::new(),
            writes: Vec::new(),
            failure: None,
        })
        .collect::<Vec<_>>();
    let model = target();
    let plan = Qwen3TensorParallelPlanV1::new(model, world).unwrap();
    let mut ranks = Vec::new();
    let mut positions = Vec::new();
    let mut page_tables = Vec::new();
    for (index, transport) in transports.iter_mut().enumerate() {
        let mut rank =
            allocate_rank_storage(transport, &plan, u32::try_from(index).unwrap(), 64, 16).unwrap();
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
        positions.push(allocate_tensor(transport, 16, 4).unwrap());
        page_tables.push(allocate_tensor(transport, 16 * 4, 4).unwrap());
    }
    let inner = EngineeringTpExecutionV1 {
        transports,
        ranks,
        plan,
        sequence: TensorParallelSequenceV1::new(64, model.vocabulary_size).unwrap(),
        collective: Qwen3TensorParallelCollectiveStateV1::new(&plan, 0, 0),
        capacity: 64,
        hidden: vec![0; 4096 * 16],
        closed: false,
    };
    EngineeringTpBatchExecutionV2 {
        inner,
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
