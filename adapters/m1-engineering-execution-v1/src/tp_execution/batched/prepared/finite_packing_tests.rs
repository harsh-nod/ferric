#[path = "finite_packing_authentic_tests.rs"]
mod authentic;

use super::*;
use ferric_spec::{Identity, ModelConfig, TensorDType};

fn model() -> ModelConfig {
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

fn metadata(kind: Qwen3TensorKind) -> Qwen3TensorMetadata {
    use Qwen3TensorKind as Kind;
    let (rank, dimension_0, dimension_1, layer) = match kind {
        Kind::TokenEmbedding | Kind::LanguageModelHead => (2, 151_936, 4096, QWEN3_NO_LAYER),
        Kind::FinalNorm => (1, 4096, 1, QWEN3_NO_LAYER),
        Kind::InputLayerNorm | Kind::PostAttentionLayerNorm => (1, 4096, 1, 0),
        Kind::QueryNorm | Kind::KeyNorm => (1, 128, 1, 0),
        Kind::QueryProjection | Kind::OutputProjection => (2, 4096, 4096, 0),
        Kind::KeyProjection | Kind::ValueProjection => (2, 1024, 4096, 0),
        Kind::GateProjection | Kind::UpProjection => (2, 12_288, 4096, 0),
        Kind::DownProjection => (2, 4096, 12_288, 0),
    };
    Qwen3TensorMetadata {
        role: Qwen3ModelRole::Target8B,
        kind,
        layer,
        dtype: TensorDType::Bf16,
        rank,
        dimension_0,
        dimension_1,
    }
}

// This fixture exercises byte movement, not authenticated-model admission.
fn part(kind: Qwen3TensorKind, rank: u32, source: &[u8]) -> Part<'_> {
    let metadata = metadata(kind);
    let shard = Qwen3TensorParallelPlanV1::new(model(), 2)
        .unwrap()
        .tensor(metadata, rank)
        .unwrap();
    Part {
        metadata,
        digest: Sha256::digest(source).into(),
        source,
        shard,
    }
}

fn bytes(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn collect(upload: &EngineeringTp2FiniteUploadV1<'_>, cap: usize) -> Vec<u8> {
    let mut output = Vec::new();
    upload
        .visit_chunks(cap, |offset, chunk| {
            assert_eq!(offset, output.len());
            assert!(!chunk.is_empty() && chunk.len() <= cap && chunk.len().is_multiple_of(2));
            output.extend_from_slice(chunk);
            Ok(())
        })
        .unwrap();
    output
}

#[test]
fn head_norm_packing_preserves_order_and_all_bf16_bit_patterns() {
    let patterns = [
        0, 0x8000, 1, 0x007f, 0x0080, 0x3f80, 0x7f80, 0xff80, 0x7f81, 0xffc1,
    ];
    let query = bytes(
        &(0..128)
            .map(|i| patterns[i % patterns.len()])
            .collect::<Vec<_>>(),
    );
    let key = bytes(
        &(0..128)
            .map(|i| patterns[(i + 3) % patterns.len()])
            .collect::<Vec<_>>(),
    );
    let expected = [query.as_slice(), key.as_slice()].concat();
    for rank in 0..2 {
        let upload = make_upload(
            EngineeringTp2FiniteUploadKeyV1::PackedHeadNorm { rank, layer: 0 },
            vec![
                part(Qwen3TensorKind::QueryNorm, rank, &query),
                part(Qwen3TensorKind::KeyNorm, rank, &key),
            ],
            512,
        )
        .unwrap();
        assert_eq!(upload.bytes(), 512);
        assert_eq!(upload.sha256(), <[u8; 32]>::from(Sha256::digest(&expected)));
        for cap in [2, 14, 128, 510, MAX_CHUNK] {
            assert_eq!(collect(&upload, cap), expected);
        }
        let sources = upload.sources().collect::<Vec<_>>();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].metadata.kind, Qwen3TensorKind::QueryNorm);
        assert_eq!(sources[1].metadata.kind, Qwen3TensorKind::KeyNorm);
        assert_eq!(sources[0].rows, [0, 128]);
        assert_eq!(sources[0].columns, [0, 1]);
        assert_eq!(
            sources[0].source_sha256,
            <[u8; 32]>::from(Sha256::digest(&query))
        );
    }
}

fn row_marked(rows: usize, columns: usize, salt: u16) -> Vec<u8> {
    let mut source = vec![0; rows * columns * 2];
    for (row, data) in source.chunks_exact_mut(columns * 2).enumerate() {
        let marker = ((row as u16) ^ salt).to_le_bytes();
        for word in data.chunks_exact_mut(2) {
            word.copy_from_slice(&marker);
        }
        data[..2].copy_from_slice(&(salt ^ 0x1111).to_le_bytes());
        data[(columns - 1) * 2..].copy_from_slice(&(salt ^ 0xeeee).to_le_bytes());
    }
    source
}

#[test]
fn qkv_packing_has_exact_rank_rows_and_q_then_k_then_v() {
    let query = row_marked(4096, 4096, 0x1000);
    let key = row_marked(1024, 4096, 0x3000);
    let value = row_marked(1024, 4096, 0x5000);
    for rank in 0..2 {
        let upload = make_upload(
            EngineeringTp2FiniteUploadKeyV1::PackedQkv { rank, layer: 0 },
            vec![
                part(Qwen3TensorKind::QueryProjection, rank, &query),
                part(Qwen3TensorKind::KeyProjection, rank, &key),
                part(Qwen3TensorKind::ValueProjection, rank, &value),
            ],
            3072 * 4096 * 2,
        )
        .unwrap();
        let mut independent_digest = Sha256::new();
        let mut position = 0usize;
        upload
            .visit_chunks(17 * 8192, |offset, chunk| {
                assert_eq!(offset, position);
                for row in chunk.chunks_exact(8192) {
                    let output_row = position / 8192;
                    let (source, local, shard_rows) = match output_row {
                        0..2048 => (&query, output_row, 2048),
                        2048..2560 => (&key, output_row - 2048, 512),
                        _ => (&value, output_row - 2560, 512),
                    };
                    let source_row = rank as usize * shard_rows + local;
                    assert_eq!(row, &source[source_row * 8192..(source_row + 1) * 8192]);
                    independent_digest.update(row);
                    position += row.len();
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(position, upload.bytes());
        assert_eq!(
            upload.sha256(),
            <[u8; 32]>::from(independent_digest.finalize())
        );
        assert_eq!(
            upload.sources().map(|p| p.rows).collect::<Vec<_>>(),
            vec![[rank * 2048, 2048], [rank * 512, 512], [rank * 512, 512]]
        );
    }
}

#[test]
fn original_mlp_o_and_globals_use_existing_exact_nxk_windows() {
    use Qwen3TensorKind as Kind;
    let plan = Qwen3TensorParallelPlanV1::new(model(), 2).unwrap();
    for (kind, rows, columns, row_partitioned) in [
        (Kind::GateProjection, 12_288, 4096, true),
        (Kind::UpProjection, 12_288, 4096, true),
        (Kind::OutputProjection, 4096, 4096, false),
        (Kind::DownProjection, 4096, 12_288, false),
    ] {
        for rank in 0..2 {
            let shard = plan.tensor(metadata(kind), rank).unwrap();
            let expected_rows = if row_partitioned {
                [rank * rows / 2, rows / 2]
            } else {
                [0, rows]
            };
            let expected_columns = if row_partitioned {
                [0, columns]
            } else {
                [rank * columns / 2, columns / 2]
            };
            assert_eq!([shard.rows().start, shard.rows().count], expected_rows);
            assert_eq!(
                [shard.columns().start, shard.columns().count],
                expected_columns
            );
            for row in [0, shard.rows().count - 1] {
                assert_eq!(
                    shard
                        .source_row_bytes(row, u64::from(rows) * u64::from(columns) * 2)
                        .unwrap(),
                    (
                        (u64::from(expected_rows[0] + row) * u64::from(columns)
                            + u64::from(expected_columns[0]))
                            * 2,
                        u64::from(expected_columns[1]) * 2
                    )
                );
            }
        }
    }
    for kind in GLOBAL_KINDS {
        let data = metadata(kind);
        let shard = plan.tensor(data, 0).unwrap();
        assert_eq!(data.layer, QWEN3_NO_LAYER);
        assert_eq!(
            [shard.rows().start, shard.rows().count],
            [0, data.dimension_0]
        );
        assert_eq!(
            [shard.columns().start, shard.columns().count],
            [0, data.dimension_1]
        );
    }
}

#[test]
fn column_shard_copy_preserves_every_original_row_without_transpose() {
    let source = row_marked(4096, 4096, 0x1234);
    for rank in 0..2 {
        let upload = make_upload(
            EngineeringTp2FiniteUploadKeyV1::Source {
                rank,
                id: 10 + u64::from(rank),
                layer: 0,
                kind: Qwen3TensorKind::OutputProjection,
            },
            vec![part(Qwen3TensorKind::OutputProjection, rank, &source)],
            4096 * 2048 * 2,
        )
        .unwrap();
        let mut position = 0;
        upload
            .visit_chunks(11 * 4096, |offset, chunk| {
                assert_eq!(offset, position);
                for row in chunk.chunks_exact(4096) {
                    let original_start = position / 4096 * 8192 + rank as usize * 4096;
                    assert_eq!(row, &source[original_start..original_start + 4096]);
                    position += 4096;
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(position, upload.bytes());
    }
}

#[test]
fn invalid_chunk_sizes_refuse_before_sink_and_copy() {
    let source = vec![0; 256];
    let norm = part(Qwen3TensorKind::QueryNorm, 0, &source);
    for cap in [0, 1, 3, MAX_CHUNK + 1, MAX_CHUNK + 2, usize::MAX] {
        let mut calls = 0;
        assert!(
            stream_parts(&[norm], cap, &mut |_, _| {
                calls += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(calls, 0);
    }
    let mut calls = 0;
    assert!(
        stream_parts(&[], 2, &mut |_, _| {
            calls += 1;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(calls, 0);
    let source = vec![0; 4096 * 4096 * 2];
    let wide = part(Qwen3TensorKind::OutputProjection, 0, &source);
    assert!(
        stream_parts(&[wide], 4094, &mut |_, _| {
            calls += 1;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(calls, 0);
}

#[test]
fn later_malformed_part_refuses_before_any_external_write() {
    let query = vec![0; 256];
    let short_key = vec![0; 254];
    let mut calls = 0;
    assert!(
        stream_parts(
            &[
                part(Qwen3TensorKind::QueryNorm, 0, &query),
                part(Qwen3TensorKind::KeyNorm, 0, &short_key)
            ],
            128,
            &mut |_, _| {
                calls += 1;
                Ok(())
            }
        )
        .is_err()
    );
    assert_eq!(calls, 0);
}

#[test]
fn sink_error_stops_without_retry_or_later_write() {
    let source = vec![0; 256];
    let upload = make_upload(
        EngineeringTp2FiniteUploadKeyV1::Source {
            rank: 0,
            id: 9,
            layer: 0,
            kind: Qwen3TensorKind::QueryNorm,
        },
        vec![part(Qwen3TensorKind::QueryNorm, 0, &source)],
        256,
    )
    .unwrap();
    let mut calls = Vec::new();
    let error = upload
        .visit_chunks(64, |offset, _| {
            calls.push(offset);
            if calls.len() == 2 {
                Err("injected write failure".into())
            } else {
                Ok(())
            }
        })
        .unwrap_err();
    assert_eq!(error, "injected write failure");
    assert_eq!(calls, [0, 64]);
}

#[test]
fn coordinates_kinds_order_and_destination_extents_are_closed() {
    use EngineeringTp2FiniteUploadKeyV1 as Key;
    let source = vec![0; 256];
    let query = part(Qwen3TensorKind::QueryNorm, 0, &source);
    let key = part(Qwen3TensorKind::KeyNorm, 0, &source);
    for upload_key in [
        Key::Source {
            rank: 0,
            id: 0,
            layer: 0,
            kind: Qwen3TensorKind::QueryNorm,
        },
        Key::Source {
            rank: 2,
            id: 1,
            layer: 0,
            kind: Qwen3TensorKind::QueryNorm,
        },
        Key::Source {
            rank: 1,
            id: 1,
            layer: 0,
            kind: Qwen3TensorKind::QueryNorm,
        },
        Key::Source {
            rank: 0,
            id: 1,
            layer: 1,
            kind: Qwen3TensorKind::QueryNorm,
        },
        Key::Source {
            rank: 0,
            id: 1,
            layer: 0,
            kind: Qwen3TensorKind::KeyNorm,
        },
    ] {
        assert!(make_upload(upload_key, vec![query], 256).is_err());
    }
    let packed = Key::PackedHeadNorm { rank: 0, layer: 0 };
    assert!(make_upload(packed, vec![key, query], 512).is_err());
    assert!(make_upload(packed, vec![query], 256).is_err());
    assert!(make_upload(packed, vec![query, key], 510).is_err());
    assert!(make_upload(packed, vec![query, key], 514).is_err());
    assert!(make_upload(Key::PackedQkv { rank: 0, layer: 0 }, vec![query, key], 512).is_err());
}

#[test]
fn shard_planner_rejects_wrong_model_rank_layer_and_shape() {
    let plan = Qwen3TensorParallelPlanV1::new(model(), 2).unwrap();
    assert!(
        plan.tensor(metadata(Qwen3TensorKind::QueryNorm), 2)
            .is_err()
    );
    let original = metadata(Qwen3TensorKind::QueryNorm);
    let mut bad = original;
    bad.layer = 36;
    assert!(plan.tensor(bad, 0).is_err());
    bad = original;
    bad.dimension_0 = 127;
    assert!(plan.tensor(bad, 0).is_err());
    bad = original;
    bad.role = Qwen3ModelRole::Draft06B;
    assert!(plan.tensor(bad, 0).is_err());
}
