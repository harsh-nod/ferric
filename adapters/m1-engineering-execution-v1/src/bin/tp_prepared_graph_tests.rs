// CPU protocol fixtures, not executable program admission or GPU evidence.
fn graph_receipt_fixture(
    epoch: u64,
    policy: GraphPolicy,
) -> (Program, Input, scope::graph::GraphExecutionReceipt) {
    use scope::graph;
    let (mut program, input, _) = receipt_fixture(epoch);
    for (index, name) in [
        "ferric_qwen3_tp_batch_embedding_bf16_v2",
        "ferric_qwen3_tp_peer_copy_bf16_v4",
    ]
    .into_iter()
    .enumerate()
    {
        let Step::Rank { dispatch, .. } = &mut program.steps[index] else {
            unreachable!()
        };
        dispatch.kernel = name;
    }
    let first = [epoch * 759, epoch * 757];
    let collectives = (0..72)
        .map(|index| {
            let producer0 = 10 + 21 * (index / 2) + 8 * (index % 2);
            let producers = [first[0] + producer0, first[1] + producer0 + 1];
            graph::CollectiveBinding {
                identity: dep::Identity {
                    request_id: epoch * 72 + index + 1,
                    generation: epoch * 72 + index + 1,
                    group_id: program.group_id,
                    model_role: dep::ModelRole::Target8b,
                    epoch,
                    layer: (index / 2) as u32,
                    operation: if index % 2 == 0 {
                        dep::Operation::AttentionOutputSum
                    } else {
                        dep::Operation::FeedForwardDownSum
                    },
                },
                before_producers: (index > 0).then_some(producers.map(|value| value - 1)),
                producers,
                after_producers: producers.map(|value| value + 1),
                consumers: producers.map(|value| value + 2),
            }
        })
        .collect();
    let receipt = graph::GraphExecutionReceipt {
        id: 50,
        plan_sha256: input.plan_sha256,
        generation: input.generation,
        epoch,
        position: input.position,
        input_token: input.token,
        output_token: 12_095,
        graph: graph::NativeGraphCompletion {
            program_id: input.generation,
            group_id: program.group_id,
            epoch,
            unique_ids: [1, 2],
            graph_api_calls: 1,
            logical_steps: 1013,
            rank_dispatches: 941,
            kernel_counts: [616, 613],
            barrier_counts: [143, 144],
            packet_counts: [759, 757],
            ranks: std::array::from_fn(|rank| graph::RankCompletion {
                unique_id: rank as u64 + 1,
                queue_epoch: 0,
                first_packet: first[rank],
                next_packet: first[rank] + u64::from(graph::PACKET_COUNTS[rank]),
                final_write: first[rank] + u64::from(graph::PACKET_COUNTS[rank]),
                final_read: first[rank] + u64::from(graph::PACKET_COUNTS[rank]),
                completion_values: vec![0; graph::PACKET_COUNTS[rank] as usize],
            }),
            embedding_copy_packets: [first[0], first[1], first[1] + 1],
            collectives,
        },
        policy: match policy {
            GraphPolicy::FiniteRequestAdmissionCache => {
                graph::PolicyEvidence::FiniteRequestAdmissionCache {
                    full_boundaries: 0,
                    graph_operational_boundaries: 9,
                    token_boundaries: 4,
                    reset_only_rounds: 1,
                    loop_checks: 9_111,
                    token_ordinal: input.generation,
                    provisional: true,
                    completed: None,
                }
            }
            GraphPolicy::QueuedBaseline => graph::PolicyEvidence::QueuedBaseline {},
            GraphPolicy::TransactionFences => graph::PolicyEvidence::TransactionFences {
                full_boundaries: 2,
                operational_boundaries: 9,
                reset_only_rounds: 1,
                loop_checks: 9_111,
            },
            GraphPolicy::TransactionFencesAdmissionCache => {
                graph::PolicyEvidence::TransactionFencesAdmissionCache {
                    full_boundaries: 2,
                    operational_boundaries: 9,
                    reset_only_rounds: 1,
                    loop_checks: 9_111,
                }
            }
            GraphPolicy::TransactionFencesAdmissionCacheScopedObservations => {
                graph::PolicyEvidence::TransactionFencesAdmissionCacheScopedObservations {
                    full_boundaries: 2,
                    operational_boundaries: 9,
                    reset_only_rounds: 1,
                    loop_checks: 9_111,
                }
            }
            GraphPolicy::ClosedTokenAdmissionCache => {
                graph::PolicyEvidence::ClosedTokenAdmissionCache {
                    full_boundaries: 2,
                    graph_operational_boundaries: 9,
                    token_boundaries: 4,
                    reset_only_rounds: 1,
                    loop_checks: 9_111,
                }
            }
        },
    };
    (program, input, receipt)
}

#[test]
fn graph_ready_binds_every_launch_field_and_closed_discriminator() {
    use scope::graph;
    for policy in graph::ExecutionMode::LEGACY {
        let ready = scope::graph_ready_fixture([1, 2], 30, policy);
        assert!(scope::Reply::Graph(ready.clone()).valid_ready(
            PreparedMode::Graph(policy),
            [1, 2],
            30
        ));
        for other in graph::ExecutionMode::PRE_FINITE
            .into_iter()
            .filter(|other| *other != policy)
        {
            let mut substituted = serde_json::to_value(&ready).unwrap();
            substituted["currentness"] = other.currentness().into();
            let substituted = serde_json::from_value(substituted).unwrap();
            assert!(!scope::Reply::Graph(substituted).valid_ready(
                PreparedMode::Graph(policy),
                [1, 2],
                30
            ));
        }
        for field in [
            "protocol",
            "mode",
            "execution_mode",
            "profile",
            "unique_ids",
            "process_id",
            "authority",
            "currentness",
            "control_allocation_flags",
            "native_source_sha256",
            "native_program_deadline_ms",
            "native_queued_deadline_ms",
        ] {
            let mut value = serde_json::to_value(&ready).unwrap();
            value[field] = match field {
                "protocol"
                | "process_id"
                | "control_allocation_flags"
                | "native_program_deadline_ms"
                | "native_queued_deadline_ms" => serde_json::json!(0),
                "unique_ids" => serde_json::json!([2, 1]),
                "execution_mode" => serde_json::json!(match policy {
                    graph::ExecutionMode::FiniteRequestAdmissionCache => "queued-baseline",
                    graph::ExecutionMode::QueuedBaseline => "transaction-fences",
                    graph::ExecutionMode::TransactionFences => "queued-baseline",
                    graph::ExecutionMode::TransactionFencesAdmissionCache => "transaction-fences",
                    graph::ExecutionMode::TransactionFencesAdmissionCacheScopedObservations =>
                        "transaction-fences-admission-cache",
                    graph::ExecutionMode::ClosedTokenAdmissionCache => "transaction-fences",
                }),
                _ => serde_json::json!("wrong"),
            };
            let changed = serde_json::from_value(value).unwrap();
            assert!(
                !scope::Reply::Graph(changed).valid_ready(PreparedMode::Graph(policy), [1, 2], 30),
                "{field}"
            );
        }
        let mut bytes = Vec::new();
        graph::write_response(&mut bytes, &ready).unwrap();
        assert!(
            scope::read_response(&mut bytes.as_slice(), PreparedMode::Graph(policy))
                .unwrap()
                .is_some()
        );
        assert!(scope::read_response(&mut bytes.as_slice(), PreparedMode::Interpreter).is_err());
        assert!(scope::read_response(&mut bytes.as_slice(), PreparedMode::NativeProgram).is_err());
        assert!(scope::Reply::Graph(ready).into_response().is_err());
    }
}

#[test]
fn graph_mapping_preserves_actual_native_fields_and_never_fabricates_serial_receipts() {
    for policy in GraphPolicy::PRE_FINITE {
        for epoch in [0, 1, 15, 16, 35, 63] {
            let (program, input, actual) = graph_receipt_fixture(epoch, policy);
            let typed =
                scope::validate_graph_receipt(actual.clone(), &program, &input, [1, 2], 50, policy)
                    .unwrap();
            assert_eq!(
                typed.completion.ranks[1].completion_values,
                actual.graph.ranks[1].completion_values
            );
            assert_eq!(
                typed.completion.collectives[71].consumers,
                actual.graph.collectives[71].consumers
            );
            assert_eq!(typed.output_token, actual.output_token);
            assert!(
                scope::Reply::Graph(scope::graph::Response::Executed {
                    receipt: actual.clone()
                })
                .into_response()
                .is_err()
            );
            for changed in 0..5 {
                let mut bad = actual.clone();
                match changed {
                    0 => bad.id += 1,
                    1 => bad.graph.unique_ids.swap(0, 1),
                    2 => bad.graph.ranks[0].unique_id = 99,
                    3 => bad.graph.ranks[1].completion_values[756] = 1,
                    4 => bad.graph.collectives[71].identity.request_id += 1,
                    _ => unreachable!(),
                }
                assert!(
                    scope::validate_graph_receipt(bad, &program, &input, [1, 2], 50, policy)
                        .is_err()
                );
            }
            let opposite = match policy {
                GraphPolicy::FiniteRequestAdmissionCache => GraphPolicy::QueuedBaseline,
                GraphPolicy::QueuedBaseline => GraphPolicy::TransactionFences,
                GraphPolicy::TransactionFences => GraphPolicy::QueuedBaseline,
                GraphPolicy::TransactionFencesAdmissionCache => GraphPolicy::TransactionFences,
                GraphPolicy::TransactionFencesAdmissionCacheScopedObservations => {
                    GraphPolicy::TransactionFencesAdmissionCache
                }
                GraphPolicy::ClosedTokenAdmissionCache => {
                    GraphPolicy::TransactionFencesAdmissionCache
                }
            };
            assert!(
                scope::validate_graph_receipt(actual, &program, &input, [1, 2], 50, opposite)
                    .is_err()
            );
        }
    }
}

#[test]
fn graph_capability_and_shared_close_preserve_immutable_mode_in_real_ipc() {
    for mode in scope::graph::ExecutionMode::PRE_FINITE {
        let selected = if mode.closed_token() {
            PreparedMode::GraphOptions(
                mode,
                scope::graph::KernelProfile::Baseline,
                MetadataUploadMode::Batched,
            )
        } else {
            PreparedMode::Graph(mode)
        };
        let policy = selected.graph_policy().unwrap();
        let mut ranks = fake_with_mode("normal", selected).unwrap();
        assert!(ranks[0].supports_prepared_peer_graph(policy));
        assert!(ranks[1].supports_peer_dependency_collectives());
        assert!(!ranks[0].supports_prepared_peer());
        ranks[0].allocate(4).unwrap();
        ranks[0].close().unwrap();
        assert!(!ranks[1].supports_prepared_peer_graph(policy));
        ranks[1].close().unwrap();
        assert!(ranks[0].connection.borrow().exited);
        assert_eq!(ranks[0].connection.borrow().phase, Phase::Closed);
        assert!(fake_with_mode("bad_ready", selected).is_err());
    }
}

const GRAPH_EXECUTE_FAKE: &str = r"
import json, os, struct, sys
mode = sys.argv[1]
ready = json.loads(sys.argv[2]); long = 'response' in ready
(ready['response'] if long else ready)['process_id'] = os.getpid()
receipt = json.loads(sys.argv[3])
def send(value):
    if long and 'geometry' not in value and mode != 'short-envelope':
        value = {'geometry':'short64' if mode == 'wrong-geometry' else 'long2304','response':value}
    data = json.dumps(value).encode()
    sys.stdout.buffer.write(struct.pack('<I', len(data)) + data); sys.stdout.buffer.flush()
send(ready)
while True:
    prefix = sys.stdin.buffer.read(4)
    if not prefix: break
    header = json.loads(sys.stdin.buffer.read(struct.unpack('<I', prefix)[0]))
    if long: header = header['request']
    operation = header['prepared_long_graph_op' if long else 'prepared_op']
    if operation == 'close':
        if mode == 'close-eof': break
        send({'prepared_graph_op':'closed', 'id':header['id'] + (1 if mode == 'close-id' else 0)})
        if mode == 'close-extra': send({'prepared_graph_op':'closed', 'id':header['id']})
        if mode == 'close-malformed': sys.stdout.buffer.write(struct.pack('<I', 1) + b'!'); sys.stdout.buffer.flush()
        if mode == 'close-truncated-prefix': sys.stdout.buffer.write(b'\x01'); sys.stdout.buffer.flush()
        if mode == 'close-truncated-body': sys.stdout.buffer.write(struct.pack('<I', 8) + b'{'); sys.stdout.buffer.flush()
        if mode == 'close-exit': sys.exit(1)
        break
    assert operation == 'execute'
    assert len(sys.stdin.buffer.read(512)) == 512
    if mode == 'disconnect': break
    if mode == 'fatal':
        send({'prepared_graph_op':'fatal', 'id':receipt['id'], 'message':'injected'}); break
    if mode == 'wrong-id': receipt['id'] += 1
    if mode == 'last-slot': receipt['graph']['ranks'][1]['completion_values'][-1] = 1
    if mode == 'stale-read': receipt['graph']['ranks'][1]['final_read'] -= 1
    if mode == 'last-binding': receipt['graph']['collectives'][-1]['consumers'][1] += 1
    if mode == 'wrong-device': receipt['graph']['unique_ids'] = [2,1]
    if mode == 'wrong-policy': receipt['policy'] = {'execution_mode':'queued-baseline'}
    if mode == 'token-full': receipt['policy']['full_boundaries'] = 1
    if mode == 'token-graph': receipt['policy']['graph_operational_boundaries'] = 8
    if mode == 'token-boundaries': receipt['policy']['token_boundaries'] = 3
    if mode == 'token-reset': receipt['policy']['reset_only_rounds'] = receipt['policy']['loop_checks'] + 1
    if mode == 'token-loop': receipt['policy']['loop_checks'] += 1
    if mode == 'token-provisional': receipt['policy']['provisional'] = True
    if mode == 'token-request': receipt['policy']['request_completion'] = {}
    if mode == 'token-old-policy': receipt['policy'] = {'execution_mode':'transaction-fences-admission-cache','full_boundaries':2,'operational_boundaries':9,'reset_only_rounds':1,'loop_checks':9111}
    value = {'prepared_graph_op':'executed','receipt':receipt}
    if mode == 'wrong-wire': value = {'prepared_op':'executed','receipt':receipt}
    send(value)
";

fn graph_execute_fake(mode: &str, policy: GraphPolicy, epoch: u64) -> (Connection, Input) {
    let (connection, input) =
        graph_geometry_execute_fake(mode, policy, epoch, GraphGeometry::Short64);
    (connection, input.short_input().unwrap())
}

fn graph_geometry_execute_fake(
    mode: &str,
    policy: GraphPolicy,
    epoch: u64,
    geometry: GraphGeometry,
) -> (Connection, GraphInput) {
    let (program, short, mut receipt) = graph_receipt_fixture(epoch, policy);
    if policy.finite_request() {
        let budget = scope::graph::FiniteBudget::for_geometry(match geometry {
            GraphGeometry::Short64 => scope::graph::GraphGeometry::Short64,
            GraphGeometry::Long2304 => scope::graph::GraphGeometry::Long2304,
        });
        if short.generation == u64::from(budget.forwards) {
            receipt.policy = scope::graph::PolicyEvidence::FiniteRequestAdmissionCache {
                full_boundaries: 1,
                graph_operational_boundaries: 9,
                token_boundaries: 4,
                reset_only_rounds: 1,
                loop_checks: 9_111,
                token_ordinal: short.generation,
                provisional: false,
                completed: Some(scope::graph::FiniteCompleted {
                    group_id: program.group_id,
                    forwards: budget.forwards,
                    final_labels: [
                        short.generation,
                        short.generation * 72 + 1,
                        short.generation * 72,
                    ],
                    next_packets: receipt.graph.ranks.each_ref().map(|rank| rank.next_packet),
                    full_boundaries: 2,
                }),
            };
        }
    }
    let mut input = GraphInput::from(&short);
    input.geometry = geometry;
    input.page_table = (0..geometry.pages())
        .map(|page| {
            if u64::from(page) <= epoch / 16 {
                page
            } else {
                u32::MAX
            }
        })
        .collect();
    let wire_mode = match policy {
        GraphPolicy::FiniteRequestAdmissionCache => {
            scope::graph::ExecutionMode::FiniteRequestAdmissionCache
        }
        GraphPolicy::QueuedBaseline => scope::graph::ExecutionMode::QueuedBaseline,
        GraphPolicy::TransactionFences => scope::graph::ExecutionMode::TransactionFences,
        GraphPolicy::TransactionFencesAdmissionCache => {
            scope::graph::ExecutionMode::TransactionFencesAdmissionCache
        }
        GraphPolicy::TransactionFencesAdmissionCacheScopedObservations => {
            scope::graph::ExecutionMode::TransactionFencesAdmissionCacheScopedObservations
        }
        GraphPolicy::ClosedTokenAdmissionCache => {
            scope::graph::ExecutionMode::ClosedTokenAdmissionCache
        }
    };
    let profile = if policy.finite_request() {
        scope::graph::KernelProfile::WaveStackNormAttentionKvMlp
    } else {
        scope::graph::KernelProfile::Baseline
    };
    let metadata = if policy.decode_token() {
        scope::graph::MetadataUploadMode::Batched
    } else {
        scope::graph::MetadataUploadMode::SeparateWrites
    };
    let (selected, ready) = if geometry == GraphGeometry::Long2304 {
        (
            PreparedMode::LongGraph(wire_mode, profile, metadata),
            serde_json::to_value(scope::long_graph_ready_fixture(
                [1, 2],
                0,
                wire_mode,
                profile,
                metadata,
            ))
            .unwrap(),
        )
    } else if policy.decode_token() {
        (
            PreparedMode::GraphOptions(wire_mode, profile, metadata),
            serde_json::to_value(scope::graph_ready_fixture_with_options(
                [1, 2],
                0,
                wire_mode,
                profile,
                metadata,
            ))
            .unwrap(),
        )
    } else {
        (
            PreparedMode::Graph(wire_mode),
            serde_json::to_value(scope::graph_ready_fixture([1, 2], 0, wire_mode)).unwrap(),
        )
    };
    let child = Command::new("python3")
        .args([
            "-u",
            "-c",
            GRAPH_EXECUTE_FAKE,
            mode,
            &serde_json::to_string(&ready).unwrap(),
            &serde_json::to_string(&receipt).unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut connection = Connection::connect_mode(
        child,
        [1, 2],
        Duration::from_millis(500),
        HostTiming::default(),
        selected,
    )
    .unwrap();
    let mut pages = vec![u32::MAX; usize::try_from(geometry.pages()).unwrap()];
    if epoch > 0 {
        for (index, page) in pages
            .iter_mut()
            .enumerate()
            .take((epoch as usize - 1) / 16 + 1)
        {
            *page = index as u32;
        }
    }
    connection.registration = Some(Registration {
        program,
        hash: input.plan_sha256,
        generation: input.generation,
        pages,
        geometry,
    });
    connection.phase = Phase::Ready;
    connection.next_request = receipt.id;
    (connection, input)
}

#[test]
fn graph_context2304_real_ipc_commits_only_geometry_bound_complete_drains() {
    for policy in GraphPolicy::PRE_FINITE {
        for epoch in [64, 173, 2302] {
            let (mut connection, input) =
                graph_geometry_execute_fake("normal", policy, epoch, GraphGeometry::Long2304);
            connection.execute_geometry_graph(&input).unwrap();
            let registered = connection.registration.as_ref().unwrap();
            assert_eq!(registered.generation, input.generation + 1);
            assert_eq!(registered.pages, input.page_table);
            assert_eq!(connection.phase, Phase::Ready);
            connection.terminate().unwrap();
        }
    }
    for mode in [
        "last-slot",
        "stale-read",
        "wrong-wire",
        "wrong-geometry",
        "short-envelope",
        "wrong-id",
        "fatal",
        "disconnect",
    ] {
        let (mut connection, input) = graph_geometry_execute_fake(
            mode,
            GraphPolicy::TransactionFencesAdmissionCache,
            173,
            GraphGeometry::Long2304,
        );
        let pages = connection.registration.as_ref().unwrap().pages.clone();
        assert!(connection.execute_geometry_graph(&input).is_err(), "{mode}");
        assert_eq!(connection.phase, Phase::Terminal);
        assert!(connection.exited);
        assert_eq!(
            connection.registration.as_ref().unwrap().generation,
            input.generation
        );
        assert_eq!(connection.registration.as_ref().unwrap().pages, pages);
        assert!(connection.execute_geometry_graph(&input).is_err());
    }
}

#[test]
fn scoped_parent_transport_rejects_failures_without_commit_or_retry_in_both_geometries() {
    for geometry in [GraphGeometry::Short64, GraphGeometry::Long2304] {
        for fault in ["last-slot", "stale-read", "wrong-id", "fatal", "disconnect"] {
            let (mut connection, input) = graph_geometry_execute_fake(
                fault,
                GraphPolicy::TransactionFencesAdmissionCacheScopedObservations,
                0,
                geometry,
            );
            let pages = connection.registration.as_ref().unwrap().pages.clone();
            assert!(connection.execute_geometry_graph(&input).is_err());
            assert_eq!(connection.phase, Phase::Terminal);
            assert!(connection.exited);
            assert_eq!(
                connection.registration.as_ref().unwrap().generation,
                input.generation
            );
            assert_eq!(connection.registration.as_ref().unwrap().pages, pages);
            assert!(connection.execute_geometry_graph(&input).is_err());
        }
    }
}

#[test]
fn graph_context2304_parent_rejects_page_extent_ownership_and_progression_before_ipc() {
    let (program, short, _) = graph_receipt_fixture(2302, GraphPolicy::TransactionFences);
    let mut input = GraphInput::from(&short);
    input.geometry = GraphGeometry::Long2304;
    input.page_table = (0..144).collect();
    let registered = Registration {
        program,
        hash: input.plan_sha256,
        generation: input.generation,
        pages: (0..144).collect(),
        geometry: GraphGeometry::Long2304,
    };
    validate_graph_input(&registered, &input).unwrap();
    for mutation in 0..8 {
        let mut wrong = input.clone();
        match mutation {
            0 => wrong.geometry = GraphGeometry::Short64,
            1 => {
                wrong.page_table.pop();
            }
            2 => wrong.page_table.push(u32::MAX),
            3 => wrong.page_table[143] = 144,
            4 => wrong.page_table[143] = 142,
            5 => wrong.page_table.swap(0, 1),
            6 => {
                wrong.position = 2304;
                wrong.epoch = 2304;
                wrong.generation = 2305;
            }
            _ => wrong.cos_sin[..4].copy_from_slice(&f32::NAN.to_le_bytes()),
        }
        assert!(
            validate_graph_input(&registered, &wrong).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn graph_real_ipc_commits_generation_and_page_transition_only_after_full_validation() {
    for policy in GraphPolicy::PRE_FINITE {
        for epoch in [0, 15, 16, 17, 63] {
            let (mut connection, input) = graph_execute_fake("normal", policy, epoch);
            let receipt = connection.execute_graph(&input).unwrap();
            assert_eq!(receipt.output_token, 12_095);
            let registered = connection.registration.as_ref().unwrap();
            assert_eq!(registered.generation, input.generation + 1);
            assert_eq!(registered.pages, input.page_table);
            assert_eq!(connection.phase, Phase::Ready);
            let id = connection.next_request;
            assert!(
                matches!(connection.exchange(protocol::Request::Close { id }, vec![]).unwrap().0, protocol::Response::Closed { id: actual } if actual == id)
            );
            connection.await_exit(true).unwrap();
            assert!(connection.exited);
        }
    }
}

#[test]
fn graph_real_ipc_rejects_late_substitution_without_host_commit_and_reaps_child() {
    for mode in [
        "wrong-id",
        "last-slot",
        "last-binding",
        "wrong-device",
        "wrong-policy",
        "wrong-wire",
        "disconnect",
        "fatal",
    ] {
        let (mut connection, input) = graph_execute_fake(mode, GraphPolicy::TransactionFences, 16);
        let before = connection.registration.as_ref().unwrap().pages.clone();
        assert!(connection.execute_graph(&input).is_err(), "{mode}");
        assert!(connection.exited, "{mode}");
        assert_eq!(connection.phase, Phase::Terminal);
        let registered = connection.registration.as_ref().unwrap();
        assert_eq!(registered.generation, input.generation);
        assert_eq!(registered.pages, before);
        assert!(connection.execute_graph(&input).is_err());
    }
}
