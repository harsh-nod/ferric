mod builder_fixture {
    use super::*;
    use fe2o3_hsaco::{ArgumentAccess, ExplicitValueKind, InspectedKernel};
    use fe2o3_kfd::engineering_wire::{BufferAccessV1, ExplicitArgumentV1, KernelMetadataV1};
    use ferric_m1_engineering_execution_v1::tp_artifact::{
        ENGINEERING_TP_BATCH_EXPORTS_V2, ENGINEERING_TP_GRAPH_ARGMAX_ROOT_V22,
        ENGINEERING_TP_PEER_TP2_EXPORTS_V18, ENGINEERING_TP_PERFORMANCE_EXPORTS_V3,
    };
    use serde::{Deserialize, Serialize};
    use std::io::Write;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Fixture {
        schema: String,
        #[serde(default)]
        geometry: Option<String>,
        profile: String,
        program: FixtureProgram,
        buffers: Vec<protocol::BufferBinding>,
        rope_fixtures: Vec<FixtureRope>,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct FixtureRope {
        position: u32,
        bytes: Vec<u8>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FixtureProgram {
        group_id: u64,
        token_buffer: u64,
        result_buffer: u64,
        metadata: [protocol::RankMetadata; 2],
        steps: Vec<FixtureStep>,
    }

    #[derive(Deserialize)]
    #[serde(tag = "step", rename_all = "snake_case", deny_unknown_fields)]
    enum FixtureStep {
        Rank {
            rank: u32,
            dispatch: FixtureDispatch,
        },
        Collective {
            key: FixtureKey,
            rows: u32,
            producers: [FixtureDispatch; 2],
            consumers: [FixtureDispatch; 2],
        },
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FixtureKey {
        group_id: u64,
        model_role: String,
        epoch: u64,
        layer: u32,
        operation: dep::Operation,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FixtureDispatch {
        kernel: String,
        grid_workgroups: u32,
        workgroup_size: u16,
        arguments: Vec<FixtureArgument>,
    }

    #[derive(Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    enum FixtureArgument {
        Buffer {
            id: u64,
            offset: usize,
            elements: usize,
            element_bytes: u32,
            access: BufferAccessV1,
        },
        U32 {
            value: u32,
        },
        F32Bits {
            value: u32,
        },
    }

    fn dispatch(value: FixtureDispatch) -> EngineeringTpDispatchV1 {
        let kernel = ENGINEERING_TP_BATCH_EXPORTS_V2
            .into_iter()
            .chain(ENGINEERING_TP_PERFORMANCE_EXPORTS_V3)
            .chain(ENGINEERING_TP_PEER_TP2_EXPORTS_V18)
            .chain([ENGINEERING_TP_GRAPH_ARGMAX_ROOT_V22])
            .chain(ferric_m1_engineering_execution_v1::tp_artifact::ENGINEERING_TP_GRAPH_SPLIT_ATTENTION_ROOTS_V1)
            .chain(ferric_m1_engineering_execution_v1::tp_artifact::ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15)
            .find(|name| *name == value.kernel)
            .expect("fixture root must belong to the exact admitted roster");
        EngineeringTpDispatchV1 {
            kernel,
            grid_workgroups: value.grid_workgroups,
            workgroup_size: value.workgroup_size,
            arguments: value
                .arguments
                .into_iter()
                .map(|argument| match argument {
                    FixtureArgument::Buffer {
                        id,
                        offset,
                        elements,
                        element_bytes,
                        access,
                    } => Arg::Buffer {
                        id,
                        offset,
                        elements,
                        element_bytes,
                        access: match access {
                            BufferAccessV1::Read => Access::Read,
                            BufferAccessV1::Write => Access::Write,
                            BufferAccessV1::ReadWrite => Access::ReadWrite,
                        },
                    },
                    FixtureArgument::U32 { value } => Arg::U32(value),
                    FixtureArgument::F32Bits { value } => Arg::F32(f32::from_bits(value)),
                })
                .collect(),
        }
    }

    fn program(value: FixtureProgram) -> Program {
        Program {
            group_id: value.group_id,
            token_buffer: value.token_buffer,
            result_buffer: value.result_buffer,
            metadata: value.metadata.map(|item| EngineeringTp2PreparedMetadataV1 {
                positions: item.positions,
                page_table: item.page_table,
                cos: item.cos,
                sin: item.sin,
            }),
            steps: value
                .steps
                .into_iter()
                .map(|step| match step {
                    FixtureStep::Rank {
                        rank,
                        dispatch: item,
                    } => Step::Rank {
                        rank,
                        dispatch: dispatch(item),
                    },
                    FixtureStep::Collective {
                        key,
                        rows,
                        producers,
                        consumers,
                    } => {
                        assert_eq!(key.model_role, "target8b");
                        Step::Collective(EngineeringTp2CollectiveRequestV1 {
                            key: Qwen3TensorParallelCollectiveKeyV1 {
                                group_id: key.group_id,
                                model_role: Qwen3ModelRole::Target8B,
                                epoch: key.epoch,
                                layer: key.layer,
                                operation: match key.operation {
                                    dep::Operation::AttentionOutputSum => {
                                        Qwen3TensorParallelCollectiveV1::AttentionOutputSum
                                    }
                                    dep::Operation::FeedForwardDownSum => {
                                        Qwen3TensorParallelCollectiveV1::FeedForwardDownSum
                                    }
                                },
                            },
                            rows,
                            producers: producers.map(dispatch),
                            consumers: consumers.map(dispatch),
                        })
                    }
                })
                .collect(),
        }
    }

    fn metadata(value: &InspectedKernel, image: [u8; 32]) -> KernelMetadataV1 {
        KernelMetadataV1 {
            symbol: value.name().into(),
            object_sha256: image,
            kernarg_bytes: u32::try_from(value.kernarg_segment_size()).unwrap(),
            kernarg_alignment: u32::try_from(value.kernarg_segment_alignment()).unwrap(),
            group_segment_bytes: u32::try_from(value.group_segment_fixed_size()).unwrap(),
            private_segment_bytes: u32::try_from(value.private_segment_fixed_size()).unwrap(),
            wavefront_size: value.wavefront_size(),
            implicit_argument_offset: value
                .implicit_argument_offset()
                .map(|offset| u32::try_from(offset).unwrap()),
            implicit_argument_bytes: u32::try_from(value.implicit_argument_size()).unwrap(),
            explicit_arguments: value
                .explicit_arguments()
                .iter()
                .map(|argument| ExplicitArgumentV1 {
                    offset: u32::try_from(argument.offset()).unwrap(),
                    bytes: u32::try_from(argument.size()).unwrap(),
                    global_buffer: argument.value_kind() == ExplicitValueKind::GlobalBuffer,
                    pointee_alignment: argument
                        .pointee_alignment()
                        .map(|alignment| u32::try_from(alignment).unwrap()),
                    access: argument.access().map(|access| match access {
                        ArgumentAccess::ReadOnly => BufferAccessV1::Read,
                        ArgumentAccess::WriteOnly => BufferAccessV1::Write,
                        ArgumentAccess::ReadWrite => BufferAccessV1::ReadWrite,
                    }),
                })
                .collect(),
        }
    }

    #[test]
    fn prepared_builder_fixture_argument_schema_is_closed_and_preserves_float_bits() {
        let argument: FixtureArgument =
            serde_json::from_str(r#"{"kind":"f32_bits","value":2147483648}"#).unwrap();
        let value = dispatch(FixtureDispatch {
            kernel: ENGINEERING_TP_BATCH_EXPORTS_V2[0].into(),
            grid_workgroups: 1,
            workgroup_size: 64,
            arguments: vec![argument],
        });
        let Arg::F32(number) = value.arguments[0] else {
            panic!("f32 argument");
        };
        assert_eq!(number.to_bits(), 0x8000_0000);
        for invalid in [
            r#"{"kind":"u32","value":1,"address":3}"#,
            r#"{"kind":"pointer","value":1}"#,
            r#"{"kind":"u32","value":4294967296}"#,
        ] {
            assert!(serde_json::from_str::<FixtureArgument>(invalid).is_err());
        }
    }

    #[test]
    #[ignore = "CPU-only actual builder/artifact packing requires pinned fixture paths"]
    fn pack_prepared_actual_builder_fixture_with_admitted_artifacts() {
        pack_actual_builder_fixture(PreparedMode::Interpreter);
    }

    #[test]
    #[ignore = "CPU-only actual builder/artifact packing requires pinned fixture paths"]
    fn pack_native_program_actual_builder_fixture_with_admitted_artifacts() {
        pack_actual_builder_fixture(PreparedMode::NativeProgram);
    }

    #[test]
    #[ignore = "requires explicit actual-builder fixture and admitted main/peer images; CPU only"]
    fn actual_builder_packs_queued_graph_source_without_serial_receipts() {
        pack_actual_builder_fixture(PreparedMode::Graph(
            scope::graph::ExecutionMode::TransactionFences,
        ));
    }

    #[test]
    #[ignore = "CPU-only actual V22 builder packing requires pinned three-image fixture paths"]
    fn actual_builder_packs_v22_graph_with_admitted_sidecar() {
        let profile = protocol::KernelProfile::parse(
            &std::env::var("FERRIC_TP2_GRAPH_KERNEL_PROFILE").unwrap(),
        )
        .unwrap();
        assert!(profile.has_v22() && !profile.has_v15());
        pack_actual_builder_fixture(PreparedMode::GraphOptions(
            scope::graph::ExecutionMode::TransactionFences,
            profile,
            scope::graph::MetadataUploadMode::SeparateWrites,
        ));
    }

    #[test]
    #[ignore = "CPU-only wave-stack packing requires actual builder plus four admitted images"]
    fn actual_builder_packs_wave_stack_with_both_rank_norm_sidecars() {
        let profile = protocol::KernelProfile::parse(
            &std::env::var("FERRIC_TP2_GRAPH_KERNEL_PROFILE").unwrap(),
        )
        .unwrap();
        assert!(profile.has_v15());
        pack_actual_builder_fixture(PreparedMode::GraphOptions(
            scope::graph::ExecutionMode::TransactionFences,
            profile,
            scope::graph::MetadataUploadMode::SeparateWrites,
        ));
    }

    #[test]
    #[ignore = "CPU-only long graph packing requires actual context2304 builder and admitted images"]
    fn actual_builder_packs_context2304_with_admitted_artifacts() {
        assert_eq!(
            std::env::var("FERRIC_TP2_GRAPH_GEOMETRY").unwrap(),
            "long2304"
        );
        let profile = protocol::KernelProfile::parse(
            &std::env::var("FERRIC_TP2_GRAPH_KERNEL_PROFILE").unwrap(),
        )
        .unwrap();
        pack_actual_builder_fixture(PreparedMode::LongGraph(
            scope::graph::ExecutionMode::TransactionFences,
            profile,
            scope::graph::MetadataUploadMode::SeparateWrites,
        ));
    }

    #[test]
    fn parent_graph_kernel_axes_match_the_worker_wire_contract() {
        assert_eq!(GraphKernels::ALL.len(), protocol::KernelProfile::ALL.len());
        assert_eq!(GraphGeometry::ALL.len(), protocol::GraphGeometry::ALL.len());
        for parent in GraphKernels::ALL {
            let wire = protocol::KernelProfile::parse(parent.argument()).unwrap();
            assert_eq!(parent.has_v22(), wire.has_v22());
            assert_eq!(parent.has_v15(), wire.has_v15());
            assert_eq!(parent.wave_argmax(), wire.wave_argmax());
            assert_eq!(parent.wave_hidden_norm(), wire.wave_hidden_norm());
            assert_eq!(parent.wave_attention(), wire.wave_attention());
            assert_eq!(parent.split_attention(), wire.split_attention());
            assert_eq!(parent.steps(), wire.steps());
            assert_eq!(parent.rank_dispatches(), wire.rank_dispatches());
            assert_eq!(parent.kernel_counts(), wire.kernel_counts());
            assert_eq!(
                parent.graph_packet_counts(),
                wire.graph_packet_counts().map(u64::from)
            );
            assert_eq!(parent.wave_kv(), wire.wave_kv());
            assert_eq!(parent.wave_mlp(), wire.wave_mlp());
            assert_eq!(
                parent
                    .loaded_kernel_counts()
                    .into_iter()
                    .map(|count| usize::try_from(count).unwrap())
                    .sum::<usize>(),
                wire.descriptors()
            );
            for geometry in GraphGeometry::ALL {
                let wire_geometry = protocol::GraphGeometry::parse(geometry.argument()).unwrap();
                assert_eq!(geometry.context_tokens(), wire_geometry.context_tokens());
                assert_eq!(geometry.pages(), wire_geometry.pages());
                assert_eq!(
                    geometry.attention_splits(),
                    wire_geometry.attention_splits()
                );
                assert_eq!(
                    geometry.attention_scratch_values(),
                    wire_geometry.attention_scratch_values()
                );
                assert_eq!(
                    geometry.program_profile(parent),
                    wire_geometry.program_profile(wire)
                );
            }
        }
    }

    fn pack_actual_builder_fixture(mode: PreparedMode) {
        let input =
            std::env::var_os("FERRIC_PREPARED_PROGRAM_FIXTURE").expect("builder fixture input");
        let output = std::env::var_os("FERRIC_PREPARED_WIRE_FIXTURE")
            .expect("wire fixture create-new output");
        let rope_output = std::env::var_os("FERRIC_PREPARED_ROPE_FIXTURE")
            .expect("real RoPE fixture create-new output");
        let main_path =
            std::env::var_os("FERRIC_PREPARED_MAIN_ARTIFACT").expect("admitted main artifact");
        let peer_path =
            std::env::var_os("FERRIC_PREPARED_PEER_ARTIFACT").expect("admitted peer artifact");
        let bytes = std::fs::read(input).unwrap();
        assert!(bytes.len() <= protocol::MAX_PROGRAM_BYTES);
        let fixture: Fixture = serde_json::from_slice(&bytes).unwrap();
        let long = mode.geometry() == scope::graph::GraphGeometry::Long2304;
        assert_eq!(
            fixture.schema,
            if long {
                "ferric-prepared-parent-builder-context2304-fixture-v2"
            } else {
                "ferric-prepared-parent-builder-fixture-v1"
            }
        );
        assert_eq!(fixture.geometry.as_deref(), long.then_some("long2304"));
        assert_eq!(
            fixture.profile,
            mode.geometry().program_profile(mode.kernel_profile())
        );
        assert_eq!(
            fixture
                .rope_fixtures
                .iter()
                .map(|item| item.position)
                .collect::<Vec<_>>(),
            if long {
                vec![0, 1, 17, 63, 64, 173, 174, 2047, 2302, 2303]
            } else {
                vec![0, 1, 17, 63]
            }
        );
        let program = program(fixture.program);
        for item in &fixture.rope_fixtures {
            let mut page_table = vec![u32::MAX; usize::try_from(mode.geometry().pages()).unwrap()];
            for (index, page) in page_table
                .iter_mut()
                .enumerate()
                .take(usize::try_from(item.position / 16 + 1).unwrap())
            {
                *page = u32::try_from(index).unwrap();
            }
            let input = GraphInput {
                geometry: graph_geometry(mode),
                plan_sha256: [0x51; 32],
                generation: u64::from(item.position) + 1,
                epoch: u64::from(item.position),
                token: 42,
                position: item.position,
                page_table: page_table.clone(),
                cos_sin: item.bytes.clone(),
            };
            let registered = Registration {
                program: program.clone(),
                hash: input.plan_sha256,
                generation: input.generation,
                pages: page_table,
                geometry: graph_geometry(mode),
            };
            validate_graph_input(&registered, &input).unwrap();
        }
        let main = EngineeringTpArtifactV1::open_performance(
            Path::new(&main_path),
            &ferric_qwen3_tp_perf_kernels_device_v3::compiler_expectation_roster_v3(),
            true,
        )
        .unwrap();
        let peer = EngineeringTpArtifactV1::open_peer_tp2_v18(
            Path::new(&peer_path),
            &ferric_qwen3_tp_peer_tp2_kernels_device_v18::compiler_expectation_roster_v18(),
        )
        .unwrap();
        let v22 = mode.kernel_profile().has_v22().then(|| {
            EngineeringTpArtifactV1::open_graph_bf16_argmax_v22(Path::new(
                &std::env::var_os("FERRIC_PREPARED_V22_ARTIFACT").unwrap(),
            ))
            .unwrap()
        });
        let v15 = mode.kernel_profile().has_v15().then(|| {
            EngineeringTpArtifactV1::open_graph_wave_rmsnorm_v15(Path::new(
                &std::env::var_os("FERRIC_PREPARED_V15_ARTIFACT").unwrap(),
            ))
            .unwrap()
        });
        let split = mode.kernel_profile().split_attention().then(|| {
            EngineeringTpArtifactV1::open_graph_split_attention_v1(Path::new(
                &std::env::var_os("FERRIC_PREPARED_SPLIT_ATTENTION_ARTIFACT").unwrap(),
            ))
            .unwrap()
        });
        assert_eq!(hex(&digest(main.bytes())), protocol::MAIN_IMAGE);
        assert_eq!(hex(&digest(peer.bytes())), protocol::PEER_IMAGE);
        assert_eq!(main.inspection().hsaco().kernels().len(), 15);
        assert_eq!(peer.inspection().hsaco().kernels().len(), 2);
        let mut ranks = fake_with_mode("normal", mode).unwrap();
        let packed = {
            let mut connection = ranks[0].connection.borrow_mut();
            assert!(
                fixture
                    .buffers
                    .windows(2)
                    .all(|pair| pair[0].id < pair[1].id)
            );
            for buffer in &fixture.buffers {
                assert!(buffer.id > 0 && buffer.rank < 2 && buffer.bytes > 0);
                assert!(
                    connection
                        .capacities
                        .insert(buffer.id, usize::try_from(buffer.bytes).unwrap())
                        .is_none()
                );
            }
            connection.catalog.buffers = fixture.buffers;
            let mut id = 1;
            for rank in 0..2 {
                for artifact in [&main, &peer] {
                    let image = digest(artifact.bytes());
                    for kernel in artifact.inspection().hsaco().kernels() {
                        let actual = metadata(kernel, image);
                        assert!(metadata_matches(kernel, &actual, image));
                        connection.catalog.kernels.push(protocol::KernelBinding {
                            id,
                            rank: u32::try_from(rank).unwrap(),
                            metadata: actual,
                        });
                        assert!(
                            connection.kernels[rank]
                                .insert(
                                    kernel.name().into(),
                                    LoadedKernel {
                                        id,
                                        image,
                                        metadata: kernel.clone(),
                                    }
                                )
                                .is_none()
                        );
                        id += 1;
                    }
                }
            }
            if let Some(artifact) = &v22 {
                assert_eq!(id, 35);
                assert!(artifact.is_graph_bf16_argmax_v22());
                let image = digest(artifact.bytes());
                let kernel = &artifact.inspection().hsaco().kernels()[0];
                let actual = metadata(kernel, image);
                assert!(metadata_matches(kernel, &actual, image));
                // Fixture IDs are local CPU bindings, never native-load receipt claims.
                connection.catalog.kernels.push(protocol::KernelBinding {
                    id,
                    rank: 0,
                    metadata: actual,
                });
                assert!(
                    connection.kernels[0]
                        .insert(
                            kernel.name().into(),
                            LoadedKernel {
                                id,
                                image,
                                metadata: kernel.clone(),
                            }
                        )
                        .is_none()
                );
                id += 1;
            }
            if let Some(artifact) = &v15 {
                assert_eq!(id, 36);
                assert!(artifact.is_graph_wave_rmsnorm_v15());
                for rank in 0..2 {
                    let image = digest(artifact.bytes());
                    let kernel = &artifact.inspection().hsaco().kernels()[0];
                    let actual = metadata(kernel, image);
                    assert!(metadata_matches(kernel, &actual, image));
                    connection.catalog.kernels.push(protocol::KernelBinding {
                        id,
                        rank: u32::try_from(rank).unwrap(),
                        metadata: actual,
                    });
                    assert!(
                        connection.kernels[rank]
                            .insert(
                                kernel.name().into(),
                                LoadedKernel {
                                    id,
                                    image,
                                    metadata: kernel.clone(),
                                }
                            )
                            .is_none()
                    );
                    id += 1;
                }
            }
            if let Some(artifact) = &split {
                assert_eq!(id, 38);
                let image = digest(artifact.bytes());
                assert_eq!(hex(&image), protocol::SPLIT_ATTENTION_IMAGE);
                for rank in 0..2 {
                    for root in protocol::SPLIT_ATTENTION_ROOTS {
                        let kernel = artifact
                            .inspection()
                            .hsaco()
                            .kernels()
                            .iter()
                            .find(|kernel| kernel.name() == root)
                            .unwrap();
                        let actual = metadata(kernel, image);
                        assert!(metadata_matches(kernel, &actual, image));
                        connection.catalog.kernels.push(protocol::KernelBinding {
                            id,
                            rank: u32::try_from(rank).unwrap(),
                            metadata: actual,
                        });
                        assert!(
                            connection.kernels[rank]
                                .insert(
                                    root.into(),
                                    LoadedKernel {
                                        id,
                                        image,
                                        metadata: kernel.clone(),
                                    }
                                )
                                .is_none()
                        );
                        id += 1;
                    }
                }
            }
            assert_eq!(
                usize::try_from(id - 1).unwrap(),
                mode.kernel_profile().descriptors()
            );
            let packed = connection.pack_program(&program).unwrap();
            // Metadata-only fixture: no Setup, Register, Execute, or native receipt was sent.
            assert_eq!(connection.next_request, 1);
            assert_eq!(connection.next_native_request, 1);
            assert_eq!(connection.phase, Phase::Setup);
            packed
        };
        assert_eq!(packed.steps.len(), mode.kernel_profile().steps());
        assert_eq!(packed.native_source_sha256, mode.source());
        assert_eq!(
            packed.catalog.kernels.len(),
            mode.kernel_profile().descriptors()
        );
        let packed_bytes = if long {
            serde_json::to_vec(&scope::graph::LongProgram {
                geometry: mode.geometry(),
                program: packed.clone(),
            })
            .unwrap()
        } else {
            serde_json::to_vec(&packed).unwrap()
        };
        assert!(packed_bytes.len() <= protocol::MAX_PROGRAM_BYTES);
        eprintln!(
            "actual builder packed program: {} bytes, {} buffers, {} steps, {} kernels",
            packed_bytes.len(),
            packed.catalog.buffers.len(),
            packed.steps.len(),
            packed.catalog.kernels.len()
        );
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .unwrap();
        file.write_all(&packed_bytes).unwrap();
        file.sync_all().unwrap();
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(rope_output)
            .unwrap();
        file.write_all(&serde_json::to_vec(&fixture.rope_fixtures).unwrap())
            .unwrap();
        file.sync_all().unwrap();
        ranks[0].close().unwrap();
        ranks[1].close().unwrap();
    }
}
