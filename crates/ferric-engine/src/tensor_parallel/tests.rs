use super::*;
use ferric_spec::{Identity, TensorDType, QWEN3_NO_LAYER};

fn model(role: Qwen3ModelRole) -> ModelConfig {
    let target = role == Qwen3ModelRole::Target8B;
    ModelConfig {
        role,
        model_id: Identity::new([1; 32]),
        config_id: Identity::new([2; 32]),
        vocabulary_size: 151_936,
        layers: if target { 36 } else { 28 },
        hidden_size: if target { 4_096 } else { 1_024 },
        intermediate_size: if target { 12_288 } else { 3_072 },
        query_heads: if target { 32 } else { 16 },
        kv_heads: 8,
        head_dim: 128,
        max_position_embeddings: 40_960,
        rope_theta: 1_000_000,
        tie_word_embeddings: !target,
    }
}

fn tensor(config: ModelConfig, kind: Qwen3TensorKind) -> Qwen3TensorMetadata {
    use Qwen3TensorKind as Kind;
    let hidden = config.hidden_size;
    let queries = config.query_heads * config.head_dim;
    let kv = config.kv_heads * config.head_dim;
    let intermediate = config.intermediate_size;
    let (rank, dimension_0, dimension_1, layer) = match kind {
        Kind::LanguageModelHead | Kind::TokenEmbedding => (2, 151_936, hidden, QWEN3_NO_LAYER),
        Kind::FinalNorm => (1, hidden, 1, QWEN3_NO_LAYER),
        Kind::InputLayerNorm | Kind::PostAttentionLayerNorm => (1, hidden, 1, 0),
        Kind::QueryNorm | Kind::KeyNorm => (1, 128, 1, 0),
        Kind::QueryProjection => (2, queries, hidden, 0),
        Kind::KeyProjection | Kind::ValueProjection => (2, kv, hidden, 0),
        Kind::OutputProjection => (2, hidden, queries, 0),
        Kind::GateProjection | Kind::UpProjection => (2, intermediate, hidden, 0),
        Kind::DownProjection => (2, hidden, intermediate, 0),
    };
    Qwen3TensorMetadata {
        role: config.role,
        kind,
        layer,
        dtype: TensorDType::Bf16,
        rank,
        dimension_0,
        dimension_1,
    }
}

const ROLES: [Qwen3ModelRole; 2] = [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B];

#[test]
fn tensor_parallel_rank_partitions_cover_both_models_at_one_two_eight() {
    for role in ROLES {
        let config = model(role);
        for world in [1, 2, 8] {
            let plan = Qwen3TensorParallelPlanV1::new(config, world).unwrap();
            let mut next_query = 0;
            let mut next_kv = 0;
            let mut next_intermediate = 0;
            for rank in 0..world {
                let shard = plan.rank(rank).unwrap();
                assert_eq!(shard.query_heads.start, next_query);
                assert_eq!(shard.kv_heads.start, next_kv);
                assert_eq!(shard.intermediate.start, next_intermediate);
                assert_eq!(shard.query_channels.start, shard.query_heads.start * 128);
                assert_eq!(shard.kv_channels.count, shard.kv_heads.count * 128);
                assert_eq!(
                    shard.query_heads.count / shard.kv_heads.count,
                    config.query_heads / config.kv_heads
                );
                let queries_per_kv_head = config.query_heads / config.kv_heads;
                for local_query in 0..shard.query_heads.count {
                    assert_eq!(
                        (shard.query_heads.start + local_query) / queries_per_kv_head,
                        shard.kv_heads.start + local_query / queries_per_kv_head
                    );
                }
                next_query += shard.query_heads.count;
                next_kv += shard.kv_heads.count;
                next_intermediate += shard.intermediate.count;
            }
            assert_eq!(next_query, config.query_heads);
            assert_eq!(next_kv, config.kv_heads);
            assert_eq!(next_intermediate, config.intermediate_size);
            assert_eq!(plan.rank(world), Err(TensorParallelErrorV1::InvalidRank));
        }
    }
}

#[test]
fn tensor_parallel_rejects_unsupported_models_worlds_and_extents() {
    for world in [0, 3, 4, 7, 16, u32::MAX] {
        assert_eq!(
            Qwen3TensorParallelPlanV1::new(model(ROLES[0]), world),
            Err(TensorParallelErrorV1::UnsupportedWorldSize)
        );
    }
    let mut config = model(ROLES[0]);
    config.kv_heads = 7;
    assert_eq!(
        Qwen3TensorParallelPlanV1::new(config, 8),
        Err(TensorParallelErrorV1::InvalidModel)
    );
    config = model(ROLES[0]);
    config.model_id = Identity::new([0; 32]);
    assert_eq!(
        Qwen3TensorParallelPlanV1::new(config, 8),
        Err(TensorParallelErrorV1::InvalidModel)
    );
    for extent in [0, 1, 7, 9, u32::MAX] {
        assert_eq!(
            tensor_parallel_range_v1(extent, 8, 0),
            Err(TensorParallelErrorV1::NondivisibleGeometry)
        );
    }
    let large = tensor_parallel_range_v1(u32::MAX - 7, 8, 7).unwrap();
    assert_eq!(large.start + large.count, u32::MAX - 7);
}

#[test]
fn tensor_parallel_all_tensor_rectangles_have_correct_axis_and_source_bytes() {
    use Qwen3TensorKind as Kind;
    let kinds = [
        Kind::LanguageModelHead,
        Kind::TokenEmbedding,
        Kind::FinalNorm,
        Kind::InputLayerNorm,
        Kind::PostAttentionLayerNorm,
        Kind::QueryNorm,
        Kind::KeyNorm,
        Kind::QueryProjection,
        Kind::KeyProjection,
        Kind::ValueProjection,
        Kind::OutputProjection,
        Kind::GateProjection,
        Kind::UpProjection,
        Kind::DownProjection,
    ];
    for role in ROLES {
        let config = model(role);
        for world in [1, 2, 8] {
            let plan = Qwen3TensorParallelPlanV1::new(config, world).unwrap();
            for kind in kinds {
                let metadata = tensor(config, kind);
                let source_bytes =
                    u64::from(metadata.dimension_0) * u64::from(metadata.dimension_1) * 2;
                for rank in 0..world {
                    let shard = plan.tensor(metadata, rank).unwrap();
                    assert_eq!(shard.role(), role);
                    assert_eq!(shard.rank(), rank);
                    assert_eq!(shard.world_size(), world);
                    match kind {
                        Kind::QueryProjection
                        | Kind::KeyProjection
                        | Kind::ValueProjection
                        | Kind::GateProjection
                        | Kind::UpProjection => {
                            assert_eq!(shard.mode(), TensorParallelMatrixModeV1::ColumnParallel);
                            assert_eq!(shard.rows().count * world, metadata.dimension_0);
                            assert_eq!(shard.rows().start, rank * shard.rows().count);
                            assert_eq!(shard.columns().count, metadata.dimension_1);
                        }
                        Kind::OutputProjection | Kind::DownProjection => {
                            assert_eq!(shard.mode(), TensorParallelMatrixModeV1::RowParallelSum);
                            assert_eq!(shard.columns().count * world, metadata.dimension_1);
                            assert_eq!(shard.columns().start, rank * shard.columns().count);
                            assert_eq!(shard.rows().count, metadata.dimension_0);
                        }
                        _ => {
                            assert_eq!(shard.mode(), TensorParallelMatrixModeV1::Replicated);
                            assert_eq!(shard.rows().count, metadata.dimension_0);
                            assert_eq!(shard.columns().count, metadata.dimension_1);
                        }
                    }
                    for row in [0, shard.rows().count - 1] {
                        let (offset, bytes) = shard.source_row_bytes(row, source_bytes).unwrap();
                        assert_eq!(bytes, u64::from(shard.columns().count) * 2);
                        assert_eq!(
                            offset,
                            (u64::from(shard.rows().start + row) * u64::from(metadata.dimension_1)
                                + u64::from(shard.columns().start))
                                * 2
                        );
                        assert!(offset + bytes <= source_bytes);
                    }
                    assert_eq!(
                        shard.source_row_bytes(0, source_bytes - 1),
                        Err(TensorParallelErrorV1::InvalidSourceLength)
                    );
                    assert!(shard
                        .source_row_bytes(shard.rows().count, source_bytes)
                        .is_err());
                }
            }
        }
    }
}

#[test]
fn tensor_parallel_metadata_role_shape_and_layer_are_checked() {
    let config = model(ROLES[0]);
    let plan = Qwen3TensorParallelPlanV1::new(config, 8).unwrap();
    let draft = tensor(model(ROLES[1]), Qwen3TensorKind::KeyProjection);
    assert_eq!(
        plan.tensor(draft, 0),
        Err(TensorParallelErrorV1::ModelRoleMismatch)
    );
    let mut metadata = tensor(config, Qwen3TensorKind::QueryProjection);
    metadata.dimension_0 -= 1;
    assert_eq!(
        plan.tensor(metadata, 0),
        Err(TensorParallelErrorV1::InvalidTensor)
    );
    metadata = tensor(config, Qwen3TensorKind::QueryProjection);
    metadata.layer = config.layers;
    assert_eq!(
        plan.tensor(metadata, 0),
        Err(TensorParallelErrorV1::InvalidTensor)
    );
    metadata = tensor(config, Qwen3TensorKind::QueryProjection);
    assert_eq!(
        plan.tensor(metadata, 8),
        Err(TensorParallelErrorV1::InvalidRank)
    );
}

#[test]
fn tensor_parallel_collectives_require_all_ranks_in_layer_order() {
    for role in ROLES {
        let config = model(role);
        for world in [1, 2, 8] {
            let plan = Qwen3TensorParallelPlanV1::new(config, world).unwrap();
            let mut state = Qwen3TensorParallelCollectiveStateV1::new(&plan, 101, 17);
            for epoch in [17, 18] {
                for layer in 0..config.layers {
                    for operation in [
                        Qwen3TensorParallelCollectiveV1::AttentionOutputSum,
                        Qwen3TensorParallelCollectiveV1::FeedForwardDownSum,
                    ] {
                        let key = Qwen3TensorParallelCollectiveKeyV1 {
                            group_id: 101,
                            model_role: role,
                            epoch,
                            layer,
                            operation,
                        };
                        assert_eq!(state.expected(), key);
                        for rank in (0..world).rev() {
                            let before = state;
                            assert_eq!(state.advance(), Err(TensorParallelErrorV1::MissingRanks));
                            assert_eq!(state, before);
                            state.arrive(rank, key).unwrap();
                            let arrived = state;
                            assert_eq!(
                                state.arrive(rank, key),
                                Err(TensorParallelErrorV1::DuplicateRank)
                            );
                            assert_eq!(state, arrived);
                        }
                        state.advance().unwrap();
                    }
                }
            }
            assert_eq!(state.expected().epoch, 19);
            assert_eq!(state.expected().layer, 0);
        }
    }
}

#[test]
fn tensor_parallel_collective_mismatches_are_fail_atomic() {
    let plan = Qwen3TensorParallelPlanV1::new(model(ROLES[0]), 8).unwrap();
    let mut state = Qwen3TensorParallelCollectiveStateV1::new(&plan, 101, 9);
    let expected = state.expected();
    let bad_keys = [
        Qwen3TensorParallelCollectiveKeyV1 {
            group_id: 102,
            ..expected
        },
        Qwen3TensorParallelCollectiveKeyV1 {
            model_role: ROLES[1],
            ..expected
        },
        Qwen3TensorParallelCollectiveKeyV1 {
            epoch: 8,
            ..expected
        },
        Qwen3TensorParallelCollectiveKeyV1 {
            epoch: 10,
            ..expected
        },
        Qwen3TensorParallelCollectiveKeyV1 {
            layer: 1,
            ..expected
        },
        Qwen3TensorParallelCollectiveKeyV1 {
            layer: 36,
            ..expected
        },
        Qwen3TensorParallelCollectiveKeyV1 {
            operation: Qwen3TensorParallelCollectiveV1::FeedForwardDownSum,
            ..expected
        },
    ];
    state.arrive(3, expected).unwrap();
    let before = state;
    for key in bad_keys {
        assert!(state.arrive(0, key).is_err());
        assert_eq!(state, before);
    }
    assert_eq!(
        state.arrive(8, expected),
        Err(TensorParallelErrorV1::InvalidRank)
    );
    assert_eq!(state, before);
}

#[test]
fn tensor_parallel_epoch_overflow_preserves_final_barrier() {
    let config = model(ROLES[1]);
    let plan = Qwen3TensorParallelPlanV1::new(config, 2).unwrap();
    let mut state = Qwen3TensorParallelCollectiveStateV1::new(&plan, 101, u64::MAX);
    for ordinal in 0..config.layers * 2 {
        let key = state.expected();
        state.arrive(0, key).unwrap();
        state.arrive(1, key).unwrap();
        if ordinal + 1 == config.layers * 2 {
            let before = state;
            assert_eq!(
                state.advance(),
                Err(TensorParallelErrorV1::ArithmeticOverflow)
            );
            assert_eq!(state, before);
        } else {
            state.advance().unwrap();
        }
    }
}

#[test]
fn tensor_parallel_bf16_row_copy_uses_column_offset_and_preserves_failed_destination() {
    let config = model(ROLES[1]);
    let plan = Qwen3TensorParallelPlanV1::new(config, 8).unwrap();
    let metadata = tensor(config, Qwen3TensorKind::OutputProjection);
    let shard = plan.tensor(metadata, 3).unwrap();
    let source_length =
        usize::try_from(u64::from(metadata.dimension_0) * u64::from(metadata.dimension_1) * 2)
            .unwrap();
    let source: Vec<u8> = (0..source_length)
        .map(|index| u8::try_from(index % 251).unwrap())
        .collect();
    let row_length = usize::try_from(shard.columns().count * 2).unwrap();
    let mut destination = vec![0xA5; row_length];
    for row in [0, 1, shard.rows().count - 1] {
        shard
            .copy_bf16_row_into(&source, row, &mut destination)
            .unwrap();
        let (offset, length) = shard.source_row_bytes(row, source.len() as u64).unwrap();
        let offset = usize::try_from(offset).unwrap();
        let length = usize::try_from(length).unwrap();
        assert_eq!(destination, source[offset..offset + length]);
    }
    let before = destination.clone();
    assert_eq!(
        shard.copy_bf16_row_into(&source[..source.len() - 1], 0, &mut destination),
        Err(TensorParallelErrorV1::InvalidSourceLength)
    );
    assert_eq!(destination, before);
    assert!(shard
        .copy_bf16_row_into(&source, shard.rows().count, &mut destination)
        .is_err());
    assert_eq!(destination, before);
    destination.push(0xA5);
    let before = destination.clone();
    assert_eq!(
        shard.copy_bf16_row_into(&source, 0, &mut destination),
        Err(TensorParallelErrorV1::InvalidDestinationLength)
    );
    assert_eq!(destination, before);
}

#[test]
fn tensor_parallel_bf16_projection_shards_reconstruct_original_without_overlap() {
    use Qwen3TensorKind as Kind;
    for role in ROLES {
        let config = model(role);
        for kind in [
            Kind::QueryProjection,
            Kind::KeyProjection,
            Kind::ValueProjection,
            Kind::OutputProjection,
            Kind::GateProjection,
            Kind::UpProjection,
            Kind::DownProjection,
        ] {
            let metadata = tensor(config, kind);
            let source_length = usize::try_from(
                u64::from(metadata.dimension_0) * u64::from(metadata.dimension_1) * 2,
            )
            .unwrap();
            let source: Vec<u8> = (0..source_length)
                .map(|index| u8::try_from(index % 251).unwrap())
                .collect();
            for world in [1, 2, 8] {
                let plan = Qwen3TensorParallelPlanV1::new(config, world).unwrap();
                let mut reconstructed = vec![0xFF; source_length];
                let mut copied = 0;
                for rank in 0..world {
                    let shard = plan.tensor(metadata, rank).unwrap();
                    let mut row = vec![0; usize::try_from(shard.columns().count * 2).unwrap()];
                    for local_row in 0..shard.rows().count {
                        shard
                            .copy_bf16_row_into(&source, local_row, &mut row)
                            .unwrap();
                        let (offset, length) = shard
                            .source_row_bytes(local_row, u64::try_from(source_length).unwrap())
                            .unwrap();
                        let start = usize::try_from(offset).unwrap();
                        let end = start + usize::try_from(length).unwrap();
                        assert!(reconstructed[start..end].iter().all(|value| *value == 0xFF));
                        reconstructed[start..end].copy_from_slice(&row);
                        copied += row.len();
                    }
                }
                assert_eq!(copied, source_length);
                assert_eq!(reconstructed, source);
            }
        }
    }
}
