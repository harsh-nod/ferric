use super::*;
use crate::tp_execution::EngineeringTpDispatchV1;
use ferric_engine::tensor_parallel::Qwen3TensorParallelPlanV1;
use ferric_spec::{
    Identity, ModelConfig, Qwen3ModelRole, Qwen3TensorKind, Qwen3TensorMetadata, TensorDType,
};

#[derive(Default)]
struct Recording {
    bytes: Vec<u8>,
    writes: Vec<(u64, usize, usize)>,
    attempts: usize,
    fail_at: Option<usize>,
}

impl EngineeringTpRankTransportV1 for Recording {
    fn allocate(&mut self, _: usize) -> TpResult<u64> {
        Ok(7)
    }
    fn write(&mut self, buffer: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        self.attempts += 1;
        if self.fail_at == Some(self.attempts) {
            return Err("recorded transfer failure".into());
        }
        assert_eq!(offset, self.bytes.len());
        self.writes.push((buffer, offset, bytes.len()));
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn read(&mut self, _: u64, _: usize, _: &mut [u8]) -> TpResult<()> {
        Err("not a read fixture".into())
    }
    fn submit(&mut self, _: &EngineeringTpDispatchV1) -> TpResult<()> {
        Err("not a GPU fixture".into())
    }
    fn wait(&mut self) -> TpResult<()> {
        Err("not a GPU fixture".into())
    }
    fn close(&mut self) -> TpResult<()> {
        Ok(())
    }
}

struct WithChunk {
    recording: Recording,
    chunk: usize,
}

impl EngineeringTpRankTransportV1 for WithChunk {
    fn setup_upload_chunk_bytes(&self) -> usize {
        self.chunk
    }
    fn allocate(&mut self, bytes: usize) -> TpResult<u64> {
        self.recording.allocate(bytes)
    }
    fn write(&mut self, buffer: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        self.recording.write(buffer, offset, bytes)
    }
    fn read(&mut self, buffer: u64, offset: usize, bytes: &mut [u8]) -> TpResult<()> {
        self.recording.read(buffer, offset, bytes)
    }
    fn submit(&mut self, dispatch: &EngineeringTpDispatchV1) -> TpResult<()> {
        self.recording.submit(dispatch)
    }
    fn wait(&mut self) -> TpResult<()> {
        self.recording.wait()
    }
    fn close(&mut self) -> TpResult<()> {
        self.recording.close()
    }
}

fn pattern(bytes: usize) -> Vec<u8> {
    (0..bytes)
        .map(|index| {
            u8::try_from((index.wrapping_mul(17) ^ (index >> 7) ^ (index >> 19)) % 256).unwrap()
        })
        .collect()
}

fn qwen8b() -> ModelConfig {
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

#[test]
fn legacy_default_keeps_one_mib_chunks_and_exact_tail_offsets() {
    let mut transport = Recording::default();
    assert_eq!(transport.setup_upload_chunk_bytes(), 1 << 20);
    let data = pattern((4 << 20) + 66);
    write_contiguous(&mut transport, 7, &data).unwrap();
    assert_eq!(transport.bytes, data);
    assert_eq!(
        transport.writes,
        vec![
            (7, 0, 1 << 20),
            (7, 1 << 20, 1 << 20),
            (7, 2 << 20, 1 << 20),
            (7, 3 << 20, 1 << 20),
            (7, 4 << 20, 66),
        ]
    );
}

#[test]
fn graph_sized_contiguous_transfer_preserves_every_byte_and_tail() {
    let mut transport = WithChunk {
        recording: Recording::default(),
        chunk: 4 << 20,
    };
    let data = pattern((4 << 20) + 66);
    write_contiguous(&mut transport, 7, &data).unwrap();
    assert_eq!(transport.recording.bytes, data);
    assert_eq!(
        transport.recording.writes,
        [(7, 0, 4 << 20), (7, 4 << 20, 66)]
    );
}

#[test]
fn invalid_capabilities_reject_before_any_transfer() {
    for chunk in [0, 1, 3, (4 << 20) + 2, usize::MAX] {
        let mut transport = WithChunk {
            recording: Recording::default(),
            chunk,
        };
        assert!(write_contiguous(&mut transport, 7, &[1, 2]).is_err());
        assert_eq!(transport.recording.attempts, 0);
        assert!(transport.recording.bytes.is_empty());
    }
}

#[test]
fn invalid_shard_capability_rejects_before_source_copy_or_write() {
    let model = qwen8b();
    let metadata = Qwen3TensorMetadata {
        role: model.role,
        kind: Qwen3TensorKind::KeyProjection,
        layer: 0,
        dtype: TensorDType::Bf16,
        rank: 2,
        dimension_0: 1024,
        dimension_1: 4096,
    };
    let plan = Qwen3TensorParallelPlanV1::new(model, 2).unwrap();
    let shard = plan.tensor(metadata, 1).unwrap();
    for chunk in [0, 1, 3, (4 << 20) + 2, usize::MAX] {
        let mut transport = WithChunk {
            recording: Recording::default(),
            chunk,
        };
        assert_eq!(
            write_shard(&mut transport, 7, &[], &shard),
            Err("invalid setup upload chunk capability".into())
        );
        assert_eq!(transport.recording.attempts, 0);
        assert!(transport.recording.bytes.is_empty());
    }
}

#[test]
fn shard_write_failure_returns_immediately_without_retry() {
    let model = qwen8b();
    let metadata = Qwen3TensorMetadata {
        role: model.role,
        kind: Qwen3TensorKind::KeyProjection,
        layer: 0,
        dtype: TensorDType::Bf16,
        rank: 2,
        dimension_0: 1024,
        dimension_1: 4096,
    };
    let plan = Qwen3TensorParallelPlanV1::new(model, 2).unwrap();
    let shard = plan.tensor(metadata, 1).unwrap();
    let source = pattern(1024 * 4096 * 2);
    let mut transport = Recording {
        fail_at: Some(1),
        ..Recording::default()
    };
    assert_eq!(
        write_shard(&mut transport, 7, &source, &shard),
        Err("recorded transfer failure".into())
    );
    assert_eq!(transport.attempts, 1);
    assert!(transport.bytes.is_empty());
    assert!(transport.writes.is_empty());
}

#[test]
fn empty_and_small_transfers_do_not_add_padding_or_empty_writes() {
    let mut transport = WithChunk {
        recording: Recording::default(),
        chunk: 4 << 20,
    };
    write_contiguous(&mut transport, 7, &[]).unwrap();
    assert_eq!(transport.recording.attempts, 0);
    write_contiguous(&mut transport, 7, &[0x80, 0x7f]).unwrap();
    assert_eq!(transport.recording.bytes, [0x80, 0x7f]);
    assert_eq!(transport.recording.writes, [(7, 0, 2)]);
}

#[test]
fn production_qwen8b_rank_one_shard_matches_independent_source_slice() {
    let model = qwen8b();
    let metadata = Qwen3TensorMetadata {
        role: model.role,
        kind: Qwen3TensorKind::KeyProjection,
        layer: 0,
        dtype: TensorDType::Bf16,
        rank: 2,
        dimension_0: 1024,
        dimension_1: 4096,
    };
    let plan = Qwen3TensorParallelPlanV1::new(model, 2).unwrap();
    let shard = plan.tensor(metadata, 1).unwrap();
    let source = pattern(1024 * 4096 * 2);
    // Rank one owns the upper four KV heads: rows 512..1024, all 4096 columns.
    let expected = &source[512 * 4096 * 2..];
    let mut baseline = Recording::default();
    write_shard(&mut baseline, 7, &source, &shard).unwrap();
    let mut graph = WithChunk {
        recording: Recording::default(),
        chunk: 4 << 20,
    };
    write_shard(&mut graph, 7, &source, &shard).unwrap();
    assert_eq!(baseline.bytes, expected);
    assert_eq!(graph.recording.bytes, expected);
    assert_eq!(baseline.writes.len(), 4);
    assert_eq!(graph.recording.writes, [(7, 0, 4 << 20)]);
}

#[test]
fn production_column_shard_keeps_row_order_and_partial_final_chunks() {
    let model = qwen8b();
    let metadata = Qwen3TensorMetadata {
        role: model.role,
        kind: Qwen3TensorKind::DownProjection,
        layer: 0,
        dtype: TensorDType::Bf16,
        rank: 2,
        dimension_0: 4096,
        dimension_1: 12_288,
    };
    let plan = Qwen3TensorParallelPlanV1::new(model, 8).unwrap();
    let shard = plan.tensor(metadata, 3).unwrap();
    let source = pattern(4096 * 12_288 * 2);
    let expected = source
        .chunks_exact(12_288 * 2)
        .flat_map(|row| row[4608 * 2..6144 * 2].iter().copied())
        .collect::<Vec<_>>();
    let mut baseline = Recording::default();
    write_shard(&mut baseline, 7, &source, &shard).unwrap();
    let mut graph = WithChunk {
        recording: Recording::default(),
        chunk: 4 << 20,
    };
    write_shard(&mut graph, 7, &source, &shard).unwrap();
    assert_eq!(baseline.bytes, expected);
    assert_eq!(graph.recording.bytes, expected);
    assert_eq!(baseline.writes.len(), 13);
    assert_eq!(graph.recording.writes.len(), 4);
    assert_eq!(baseline.writes.last().unwrap().2, 4 * 3072);
    assert_eq!(graph.recording.writes.last().unwrap().2, 3072);
    assert!(
        graph
            .recording
            .writes
            .iter()
            .all(|(_, offset, len)| offset % 3072 == 0 && len % 3072 == 0)
    );
}

#[test]
fn transfer_failure_stops_without_retry_or_size_fallback() {
    let mut transport = Recording {
        fail_at: Some(2),
        ..Recording::default()
    };
    let data = pattern((3 << 20) + 8);
    assert_eq!(
        write_contiguous(&mut transport, 7, &data),
        Err("recorded transfer failure".into())
    );
    assert_eq!(transport.attempts, 2);
    assert_eq!(transport.bytes, data[..1 << 20]);
    assert_eq!(transport.writes, [(7, 0, 1 << 20)]);
}
