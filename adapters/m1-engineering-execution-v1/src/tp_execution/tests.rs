//! Exercises the actual driver with a strict recording transport. This is not
//! a GPU emulator or a numerical proof of the kernels. Synthetic GPU outputs
//! let these tests inspect real host reductions, broadcasts and error paths.

use super::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Submit(u32, &'static str),
    Wait(u32, &'static str),
    Kv(u32, u32),
    Attention(u32, u32),
    Close(u32),
}

struct RecordingTransport {
    rank: u32,
    buffers: BTreeMap<u64, Vec<u8>>,
    next: u64,
    pending: Option<EngineeringTpDispatchV1>,
    events: Rc<RefCell<Vec<Event>>>,
    fail_submit: bool,
    fail_wait: bool,
}

impl RecordingTransport {
    fn output(&mut self, command: &EngineeringTpDispatchV1, index: usize, pattern: &[u8]) {
        let EngineeringTpArgumentV1::Buffer {
            id,
            offset,
            elements,
            element_bytes,
            ..
        } = command.arguments[index]
        else {
            panic!("buffer expected");
        };
        assert_eq!(pattern.len(), element_bytes as usize);
        let bytes = self.buffers.get_mut(&id).unwrap();
        for chunk in bytes[offset..offset + elements * element_bytes as usize]
            .chunks_exact_mut(pattern.len())
        {
            chunk.copy_from_slice(pattern);
        }
    }

    fn scalar(command: &EngineeringTpDispatchV1, index: usize) -> u32 {
        let EngineeringTpArgumentV1::U32(value) = command.arguments[index] else {
            panic!("u32 expected");
        };
        value
    }
}

impl EngineeringTpRankTransportV1 for RecordingTransport {
    fn allocate(&mut self, byte_len: usize) -> TpResult<u64> {
        assert!(self.pending.is_none());
        let id = self.next;
        self.next += 1;
        self.buffers.insert(id, vec![0xa5; byte_len]);
        Ok(id)
    }

    fn write(&mut self, buffer: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        assert!(self.pending.is_none());
        self.buffers.get_mut(&buffer).unwrap()[offset..offset + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    fn read(&mut self, buffer: u64, offset: usize, bytes: &mut [u8]) -> TpResult<()> {
        assert!(self.pending.is_none());
        bytes.copy_from_slice(&self.buffers[&buffer][offset..offset + bytes.len()]);
        Ok(())
    }

    fn submit(&mut self, command: &EngineeringTpDispatchV1) -> TpResult<()> {
        assert!(self.pending.is_none());
        assert_eq!(command.workgroup_size, 64);
        assert!(command.grid_workgroups > 0);
        if self.fail_submit && command.kernel == PARTIAL {
            return Err("injected submit failure".into());
        }
        self.events
            .borrow_mut()
            .push(Event::Submit(self.rank, command.kernel));
        self.pending = Some(command.clone());
        Ok(())
    }

    fn wait(&mut self) -> TpResult<()> {
        let command = self.pending.take().expect("one pending request");
        self.events
            .borrow_mut()
            .push(Event::Wait(self.rank, command.kernel));
        if self.fail_wait && command.kernel == PARTIAL {
            return Err("injected completion failure".into());
        }
        match command.kernel {
            EMBEDDING => self.output(&command, 2, &0x3f80_u16.to_le_bytes()),
            PARTIAL => self.output(
                &command,
                2,
                &(f32::from(u16::try_from(self.rank + 1).unwrap()) / 1024.0).to_le_bytes(),
            ),
            ARGMAX => self.output(&command, 1, &42_u32.to_le_bytes()),
            KV_APPEND => {
                let position = Self::scalar(&command, 4);
                let capacity = Self::scalar(&command, 5);
                assert!(position < capacity);
                self.events
                    .borrow_mut()
                    .push(Event::Kv(self.rank, position));
            }
            ATTENTION => {
                let count = Self::scalar(&command, 4);
                let capacity = Self::scalar(&command, 5);
                assert!(count > 0 && count <= capacity);
                self.events
                    .borrow_mut()
                    .push(Event::Attention(self.rank, count));
                self.output(&command, 3, &0_u16.to_le_bytes());
            }
            RMSNORM => {
                for index in [1, 3] {
                    assert!(matches!(
                        command.arguments[index],
                        EngineeringTpArgumentV1::Buffer { elements: 0, .. }
                    ));
                }
                self.output(&command, 4, &0_u16.to_le_bytes());
            }
            GEMV | LM_HEAD | SWIGLU => self.output(&command, 2, &0_u16.to_le_bytes()),
            ROPE => {
                self.output(&command, 4, &0_u16.to_le_bytes());
                self.output(&command, 5, &0_u16.to_le_bytes());
            }
            other => panic!("unexpected kernel {other}"),
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

fn fixture(world: u32) -> EngineeringTpExecutionV1<RecordingTransport> {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut transports = (0..world)
        .map(|rank| RecordingTransport {
            rank,
            buffers: BTreeMap::new(),
            next: 1,
            pending: None,
            events: events.clone(),
            fail_submit: false,
            fail_wait: false,
        })
        .collect::<Vec<_>>();
    let model = target();
    let plan = Qwen3TensorParallelPlanV1::new(model, world).unwrap();
    let mut ranks = Vec::new();
    for (index, transport) in transports.iter_mut().enumerate() {
        let mut rank = allocate_rank(transport, &plan, u32::try_from(index).unwrap(), 2).unwrap();
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
    }
    EngineeringTpExecutionV1 {
        transports,
        ranks,
        plan,
        sequence: TensorParallelSequenceV1::new(2, model.vocabulary_size).unwrap(),
        collective: Qwen3TensorParallelCollectiveStateV1::new(&plan, 0, 0),
        capacity: 2,
        row_capacity: 1,
        large_kv: false,
        hidden: vec![0; 4096],
        reduction: ReductionWorkspace::default(),
        sequences: None,
        timing: crate::host_timing::HostTiming::default(),
        closed: false,
    }
}

#[test]
fn actual_driver_runs_every_rank_and_broadcasts_real_reductions() {
    for world in [1, 2, 8] {
        let mut driver = fixture(world);
        assert_eq!(driver.step(1).unwrap(), 42);
        assert_eq!(driver.position(), 1);
        let counts = driver.dispatch_counts();
        assert_eq!(counts[0], 544);
        assert!(counts[1..].iter().all(|count| *count == 540));
        let values = (0..world)
            .map(|rank| [f32::from(u16::try_from(rank + 1).unwrap()) / 1024.0])
            .collect::<Vec<_>>();
        let partials = values
            .iter()
            .enumerate()
            .map(|(rank, values)| HostStagedPartialV1 {
                rank: u32::try_from(rank).unwrap(),
                values,
            })
            .collect::<Vec<_>>();
        let mut expected = vec![0x3f80];
        for _ in 0..72 {
            expected = reduce_residual_bf16_v1(world, &partials, &expected).unwrap();
        }
        assert!(driver.hidden.iter().all(|value| *value == expected[0]));
        for (rank, transport) in driver.ranks.iter().zip(&driver.transports) {
            let hidden = decode_bf16(&transport.buffers[&rank.hidden.id]).unwrap();
            assert_eq!(hidden, driver.hidden);
        }
        let events = driver.transports[0].events.borrow();
        for window in events.windows((world * 2) as usize) {
            if window[0] != Event::Submit(0, PARTIAL) {
                continue;
            }
            for rank in 0..world {
                assert_eq!(window[rank as usize], Event::Submit(rank, PARTIAL));
                assert_eq!(window[(world + rank) as usize], Event::Wait(rank, PARTIAL));
            }
        }
    }
}

#[test]
fn reset_restarts_kv_prefix_without_reallocating_or_changing_dispatch_counts() {
    let mut driver = fixture(2);
    assert_eq!(driver.step(1).unwrap(), 42);
    assert_eq!(driver.step(42).unwrap(), 42);
    assert!(driver.step(42).is_err());
    let allocations = driver.transports.iter().map(|r| r.next).collect::<Vec<_>>();
    let counts = driver.dispatch_counts();
    driver.reset_sequence().unwrap();
    assert_eq!(driver.position(), 0);
    assert_eq!(driver.dispatch_counts(), counts);
    assert_eq!(driver.step(1).unwrap(), 42);
    assert_eq!(
        driver.transports.iter().map(|r| r.next).collect::<Vec<_>>(),
        allocations
    );
    let events = driver.transports[0].events.borrow();
    let positions = events
        .iter()
        .filter_map(|event| match event {
            Event::Kv(0, position) => Some(*position),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(positions, [vec![0; 36], vec![1; 36], vec![0; 36]].concat());
    let counts = events
        .iter()
        .filter_map(|event| match event {
            Event::Attention(0, count) => Some(*count),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(counts, [vec![1; 36], vec![2; 36], vec![1; 36]].concat());
}

#[test]
fn partial_submission_failure_drains_prior_ranks_and_poison_prevents_retry() {
    let mut driver = fixture(8);
    driver.transports[3].fail_submit = true;
    assert!(driver.step(1).unwrap_err().contains("injected"));
    assert!(driver.transports.iter().all(|rank| rank.pending.is_none()));
    assert!(driver.step(1).is_err());
    assert!(driver.reset_sequence().is_err());
    assert_eq!(driver.position(), 0);
    driver.close().unwrap();
    let events = driver.transports[0].events.borrow();
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::Close(_)))
            .count(),
        8
    );
}

#[test]
fn completion_failure_still_waits_every_submitted_peer_and_closes_group() {
    let mut driver = fixture(8);
    driver.transports[3].fail_wait = true;
    assert!(driver.step(1).unwrap_err().contains("completion"));
    assert!(driver.transports.iter().all(|rank| rank.pending.is_none()));
    assert!(driver.reset_sequence().is_err());
    assert!(driver.step(1).is_err());
    driver.close().unwrap();
    let events = driver.transports[0].events.borrow();
    for rank in 0..8 {
        assert!(events.contains(&Event::Wait(rank, PARTIAL)));
        assert!(events.contains(&Event::Close(rank)));
    }
}

#[test]
fn byte_geometry_and_rope_tables_are_exact() {
    assert_eq!(section_bytes(&[1, 2, 3], (1, 2)).unwrap(), [2, 3]);
    assert!(section_bytes(&[1, 2, 3], (u64::MAX, 2)).is_err());
    assert!(decode_bf16(&[1]).is_err());
    assert!(decode_bf16(&[0x80, 0x7f]).is_err());
    let (cos, sin) = rope_bytes(0, 1_000_000);
    assert_eq!(cos.len(), 256);
    assert!(
        cos.chunks_exact(4)
            .all(|bits| bits == 1.0_f32.to_le_bytes())
    );
    assert!(
        sin.chunks_exact(4)
            .all(|bits| bits == 0.0_f32.to_le_bytes())
    );
}
