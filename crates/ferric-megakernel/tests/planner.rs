use ferric_megakernel::{
    declare_decode_plan, validate_declaration, DeclaredIdentityBindings, DecodePlanDeclaration,
    DecodeShape, Operation, PlanError, PlanResourceLimits, Qwen3Model, RequestDeclaration,
    ScalarType, Storage, TensorRole,
};

fn identities() -> DeclaredIdentityBindings {
    DeclaredIdentityBindings {
        bundle: [1; 32],
        config: [2; 32],
        weights: [3; 32],
        tokenizer: [4; 32],
        graph: [5; 32],
        numerical_policy: [6; 32],
        task_schema: [7; 32],
        scheduler_model: [8; 32],
        fusion_plan: [9; 32],
        persistent_plan: [10; 32],
    }
}

fn shape() -> DecodeShape {
    DecodeShape {
        model: Qwen3Model::Qwen3_06B,
        batch: 1,
        context_capacity: 1_024,
        kv_page_tokens: 16,
    }
}

fn candidate() -> DecodePlanDeclaration {
    declare_decode_plan(shape(), identities(), PlanResourceLimits::default())
        .unwrap()
        .declaration()
        .clone()
}

fn check(declaration: DecodePlanDeclaration) -> Result<(), PlanError> {
    validate_declaration(
        declaration,
        shape(),
        identities(),
        PlanResourceLimits::default(),
    )
    .map(|_| ())
}

#[test]
fn complete_envelope_has_exact_graph_and_deterministic_bytes() {
    for model in [Qwen3Model::Qwen3_06B, Qwen3Model::Qwen3_8B] {
        for batch in [1, 2, 4, 8] {
            for context_capacity in [1_024, 4_096, 8_192] {
                let shape = DecodeShape {
                    model,
                    batch,
                    context_capacity,
                    kv_page_tokens: 16,
                };
                let plan = declare_decode_plan(shape, identities(), PlanResourceLimits::default())
                    .unwrap();
                let d = plan.declaration();
                assert_eq!(
                    d.tasks.len(),
                    if model == Qwen3Model::Qwen3_06B {
                        424
                    } else {
                        544
                    }
                );
                assert_eq!(d.tasks[0].operation, Operation::Embedding);
                assert_eq!(d.tasks.last().unwrap().operation, Operation::Argmax);
                assert_eq!(d.ready_capacity as usize, d.tasks.len());
                let copy = validate_declaration(
                    d.clone(),
                    shape,
                    identities(),
                    PlanResourceLimits::default(),
                )
                .unwrap();
                assert_eq!(plan.canonical_bytes(), copy.canonical_bytes());
                assert_ne!(plan.canonical_bytes(), Vec::<u8>::new());
            }
        }
    }
}

#[test]
fn query_projection_does_not_assume_query_width_equals_hidden() {
    let d = candidate();
    let query = d
        .buffers
        .iter()
        .find(|b| b.role == TensorRole::Query)
        .unwrap();
    let q_weight = d
        .buffers
        .iter()
        .find(|b| b.role == TensorRole::QueryWeight)
        .unwrap();
    let output = d
        .buffers
        .iter()
        .find(|b| b.role == TensorRole::OutputWeight)
        .unwrap();
    assert_eq!(query.dimensions, [1, 16, 128]);
    assert_eq!(q_weight.dimensions, [2_048, 1_024]);
    assert_eq!(output.dimensions, [1_024, 2_048]);
}

#[test]
fn sequential_operator_roster_matches_qwen3_and_tracks_residual_edges() {
    let d = candidate();
    let expected = [
        Operation::InputRmsNorm,
        Operation::QueryProjection,
        Operation::KeyProjection,
        Operation::ValueProjection,
        Operation::QueryRmsNorm,
        Operation::KeyRmsNorm,
        Operation::Rope,
        Operation::KvAppend,
        Operation::Attention,
        Operation::AttentionOutputResidual,
        Operation::PostAttentionRmsNorm,
        Operation::GateProjection,
        Operation::UpProjection,
        Operation::SwiGlu,
        Operation::DownResidual,
    ];
    for layer in 0..28 {
        let tasks = &d.tasks[(1 + layer * 15)..=((layer + 1) * 15)];
        for (task, operation) in tasks.iter().zip(expected) {
            assert_eq!(task.operation, operation);
            assert_eq!(task.layer, Some(u32::try_from(layer).unwrap()));
        }
        assert_eq!(tasks[1].predecessors, tasks[2].predecessors);
        assert_eq!(tasks[1].predecessors, tasks[3].predecessors);
        assert_eq!(tasks[13].predecessors, [tasks[11].id, tasks[12].id]);
        assert!(tasks[14].inputs.contains(&tasks[9].outputs[0]));
    }
}

/// This is a graph-order oracle, not a numerical interpreter or GPU scheduler.
/// Reverse-priority readiness exercises independent Q/K/V and gate/up branches.
#[test]
fn independent_topological_walk_reaches_completion_and_covers_every_task() {
    let d = candidate();
    let mut completed = vec![false; d.tasks.len()];
    let mut order = Vec::new();
    while order.len() < d.tasks.len() {
        let ready = d
            .tasks
            .iter()
            .rev()
            .find(|task| {
                !completed[task.id as usize]
                    && task.predecessors.iter().all(|id| completed[*id as usize])
            })
            .expect("canonical DAG cannot stall");
        completed[ready.id as usize] = true;
        order.push(ready.id);
    }
    assert_eq!(order.last(), Some(&423));
    assert!(order.windows(2).any(|pair| pair[0] > pair[1]));

    // Every operation must contribute to the sole final token decision.
    let mut ancestors = vec![false; d.tasks.len()];
    let mut pending = vec![d.tasks.last().unwrap().id];
    while let Some(id) = pending.pop() {
        if !ancestors[id as usize] {
            ancestors[id as usize] = true;
            pending.extend_from_slice(&d.tasks[id as usize].predecessors);
        }
    }
    assert!(ancestors.iter().all(|ancestor| *ancestor));
}

#[test]
fn norm_and_embedding_weights_are_retained_and_tied_only_for_small_model() {
    for model in [Qwen3Model::Qwen3_06B, Qwen3Model::Qwen3_8B] {
        let plan = declare_decode_plan(
            DecodeShape { model, ..shape() },
            identities(),
            PlanResourceLimits::default(),
        )
        .unwrap();
        let d = plan.declaration();
        let embedding_weight = d.tasks[0].inputs[1];
        let logits = &d.tasks[d.tasks.len() - 2];
        assert_eq!(
            logits.inputs[1] == embedding_weight,
            model.geometry().tied_embeddings
        );
        assert!(d
            .buffers
            .iter()
            .all(|buffer| !buffer.consumers.is_empty() || buffer.role == TensorRole::GreedyTokens));
    }
}

#[test]
fn append_reservations_do_not_alias_committed_prefix_or_workspace() {
    let d = candidate();
    for task in d
        .tasks
        .iter()
        .filter(|task| task.operation == Operation::Attention)
    {
        assert_eq!(task.inputs.len(), 6);
        for id in &task.inputs[1..3] {
            assert_eq!(d.buffers[*id as usize].storage, Storage::ExternalReadOnly);
        }
        for id in &task.inputs[3..5] {
            assert_eq!(
                d.buffers[*id as usize].storage,
                Storage::KvAppendReservation
            );
        }
    }
}

#[test]
fn mutation_suite_rejects_missing_and_stale_identity_declarations() {
    let base = candidate();
    let mut changed = base.clone();
    changed.identities.weights[0] ^= 1;
    assert_eq!(check(changed), Err(PlanError::IdentityMismatch));
    let mut changed = base;
    changed.identities.task_schema = [0; 32];
    assert_eq!(check(changed), Err(PlanError::MissingIdentity(6)));
}

#[test]
fn rejects_independently_unexpected_model_even_when_graph_is_canonical() {
    let d = declare_decode_plan(
        DecodeShape {
            model: Qwen3Model::Qwen3_8B,
            ..shape()
        },
        identities(),
        PlanResourceLimits::default(),
    )
    .unwrap();
    assert_eq!(check(d.declaration().clone()), Err(PlanError::Shape));
}

#[test]
fn rejects_wrong_target_wave_size_and_version() {
    for mutator in [
        |d: &mut DecodePlanDeclaration| d.processor = "gfx942".into(),
        |d: &mut DecodePlanDeclaration| d.wave_size = 32,
        |d: &mut DecodePlanDeclaration| d.version = 2,
    ] {
        let mut changed = candidate();
        mutator(&mut changed);
        assert_eq!(check(changed), Err(PlanError::TargetOrVersion));
    }
}

#[test]
fn rejects_cycles_dangling_edges_duplicates_and_missing_fan_in() {
    let base = candidate();
    let mut cycle = base.clone();
    cycle.tasks[1].predecessors.push(2);
    assert!(matches!(check(cycle), Err(PlanError::Graph(_))));
    let mut dangling = base.clone();
    dangling.tasks[1].inputs[0] = u32::MAX;
    assert!(matches!(check(dangling), Err(PlanError::Graph(_))));
    let mut duplicate = base.clone();
    let duplicate_input = duplicate.tasks[1].inputs[0];
    duplicate.tasks[1].inputs.push(duplicate_input);
    assert!(matches!(check(duplicate), Err(PlanError::Graph(_))));
    let mut missing = base.clone();
    let swiglu = missing
        .tasks
        .iter_mut()
        .find(|task| task.operation == Operation::SwiGlu)
        .unwrap();
    swiglu.predecessors.pop();
    assert!(matches!(check(missing), Err(PlanError::Graph(_))));
    let mut producer = base;
    producer
        .buffers
        .iter_mut()
        .find(|buffer| buffer.producer == Some(1))
        .unwrap()
        .producer = Some(2);
    assert!(matches!(check(producer), Err(PlanError::Graph(_))));
}

#[test]
fn rejects_omitted_or_retagged_operator_even_if_edges_remain_valid() {
    let mut changed = candidate();
    changed.tasks[1].operation = Operation::FinalRmsNorm;
    assert_eq!(check(changed), Err(PlanError::NonCanonicalGraph));
    let mut changed = candidate();
    changed.tasks.pop();
    assert!(check(changed).is_err());
}

#[test]
fn detects_overflow_wrong_shapes_and_overlapping_live_ranges() {
    let mut changed = candidate();
    changed.buffers[0].dimensions = vec![u32::MAX; 4];
    assert_eq!(check(changed), Err(PlanError::ArithmeticOverflow));
    let mut changed = candidate();
    changed.buffers[0].byte_len += 1;
    assert!(matches!(check(changed), Err(PlanError::BufferShape(_))));
    let mut changed = candidate();
    let mut ranges = changed
        .buffers
        .iter_mut()
        .filter(|buffer| matches!(buffer.storage, Storage::Workspace { .. }));
    let first_storage = ranges.next().unwrap().storage;
    ranges.next().unwrap().storage = first_storage;
    assert!(matches!(
        check(changed),
        Err(PlanError::WorkspaceOverlap(_, _))
    ));
    let mut changed = candidate();
    changed
        .buffers
        .iter_mut()
        .find(|buffer| matches!(buffer.storage, Storage::Workspace { .. }))
        .unwrap()
        .storage = Storage::Workspace { offset: u64::MAX };
    assert_eq!(check(changed), Err(PlanError::ArithmeticOverflow));
}

#[test]
fn workspace_and_queue_underallocation_fail_closed() {
    let base = candidate();
    let workspace_bytes = base.workspace_bytes - 1;
    let limits = PlanResourceLimits {
        workspace_bytes,
        ..PlanResourceLimits::default()
    };
    assert_eq!(
        validate_declaration(base.clone(), shape(), identities(), limits).unwrap_err(),
        PlanError::ResourceLimit,
    );
    let limits = PlanResourceLimits {
        ready_tasks: 423,
        ..PlanResourceLimits::default()
    };
    assert_eq!(
        validate_declaration(base, shape(), identities(), limits).unwrap_err(),
        PlanError::ResourceLimit,
    );
}

#[test]
fn unsupported_shapes_and_page_sizes_are_rejected() {
    for batch in [0, 3, 16, u32::MAX] {
        assert_eq!(
            declare_decode_plan(
                DecodeShape { batch, ..shape() },
                identities(),
                PlanResourceLimits::default()
            )
            .unwrap_err(),
            PlanError::Shape,
        );
    }
    for page in [0, 3, 512, u32::MAX] {
        assert_eq!(
            declare_decode_plan(
                DecodeShape {
                    kv_page_tokens: page,
                    ..shape()
                },
                identities(),
                PlanResourceLimits::default()
            )
            .unwrap_err(),
            PlanError::PageSize,
        );
    }
}

fn request(slot: u32, context: u32) -> RequestDeclaration {
    RequestDeclaration {
        slot,
        generation: 1,
        kv_generation: 1,
        token_id: 42,
        committed_context: context,
        position: context,
    }
}

#[test]
fn zero_allocation_patch_binding_accepts_ragged_and_page_boundary_contexts() {
    let plan = declare_decode_plan(
        DecodeShape {
            batch: 4,
            ..shape()
        },
        identities(),
        PlanResourceLimits::default(),
    )
    .unwrap();
    let requests = [
        request(10, 0),
        request(11, 15),
        request(12, 16),
        request(13, 1_023),
    ];
    let bound = plan.bind_step(1, 1, &requests).unwrap();
    assert!(core::ptr::eq(bound.requests().as_ptr(), requests.as_ptr()));
    assert!(core::ptr::eq(bound.plan(), &raw const plan));
    assert_eq!(bound.epoch(), 1);
}

#[test]
fn rejects_stale_epoch_generation_cross_request_and_bad_positions() {
    let plan = declare_decode_plan(
        DecodeShape {
            batch: 2,
            ..shape()
        },
        identities(),
        PlanResourceLimits::default(),
    )
    .unwrap();
    let mut requests = [request(10, 15), request(11, 16)];
    assert_eq!(
        plan.bind_step(1, 2, &requests).unwrap_err(),
        PlanError::Epoch
    );
    assert_eq!(
        plan.bind_step(0, 0, &requests).unwrap_err(),
        PlanError::Epoch
    );
    assert_eq!(
        plan.bind_step(u64::MAX, u64::MAX, &requests).unwrap_err(),
        PlanError::Epoch
    );
    requests[1].slot = 10;
    requests[1].generation = 2;
    assert_eq!(
        plan.bind_step(1, 1, &requests).unwrap_err(),
        PlanError::DuplicateRequestSlot(10)
    );
    requests[1] = request(11, 1_024);
    assert_eq!(
        plan.bind_step(1, 1, &requests).unwrap_err(),
        PlanError::Position(1)
    );
    requests[1] = request(11, 16);
    requests[1].position = 15;
    assert_eq!(
        plan.bind_step(1, 1, &requests).unwrap_err(),
        PlanError::Position(1)
    );
    requests[1] = request(11, 16);
    requests[1].kv_generation = 0;
    assert_eq!(
        plan.bind_step(1, 1, &requests).unwrap_err(),
        PlanError::RequestGeneration(1)
    );
    requests[1] = request(11, 16);
    requests[1].token_id = u32::MAX;
    assert_eq!(
        plan.bind_step(1, 1, &requests).unwrap_err(),
        PlanError::Token(1)
    );
    assert_eq!(
        plan.bind_step(1, 1, &requests[..1]).unwrap_err(),
        PlanError::RequestCount
    );
}

#[test]
fn canonical_commitment_changes_for_shape_and_identity() {
    let first = declare_decode_plan(shape(), identities(), PlanResourceLimits::default()).unwrap();
    let second = declare_decode_plan(
        DecodeShape {
            batch: 2,
            ..shape()
        },
        identities(),
        PlanResourceLimits::default(),
    )
    .unwrap();
    let mut different = identities();
    different.weights = [11; 32];
    let third = declare_decode_plan(shape(), different, PlanResourceLimits::default()).unwrap();
    assert_ne!(first.canonical_bytes(), second.canonical_bytes());
    assert_ne!(first.canonical_bytes(), third.canonical_bytes());
}

#[test]
fn positions_feed_rope_kv_append_and_ragged_attention_in_every_layer() {
    let d = candidate();
    let position = d
        .buffers
        .iter()
        .find(|buffer| buffer.role == TensorRole::Positions)
        .unwrap();
    assert_eq!(position.consumers.len(), 28 * 3);
    for task in d.tasks.iter().filter(|task| {
        matches!(
            task.operation,
            Operation::Rope | Operation::KvAppend | Operation::Attention
        )
    }) {
        assert!(task.inputs.contains(&position.id));
    }
}

#[test]
fn fp32_logits_and_exact_model_constants_are_part_of_the_canonical_plan() {
    let d = candidate();
    assert_eq!(d.geometry.rms_norm_epsilon_bits, 1e-6_f32.to_bits());
    assert_eq!(d.geometry.rope_theta, 1_000_000);
    let logits = d
        .buffers
        .iter()
        .find(|buffer| buffer.role == TensorRole::Logits)
        .unwrap();
    assert_eq!(logits.scalar, ScalarType::F32);
    assert_eq!(logits.byte_len, 151_936 * 4);
    let mut changed = d.clone();
    changed.geometry.rope_theta = 10_000;
    assert_eq!(check(changed), Err(PlanError::NonCanonicalGraph));
    let mut changed = d;
    changed.geometry.rms_norm_epsilon_bits = 1e-5_f32.to_bits();
    assert_eq!(check(changed), Err(PlanError::NonCanonicalGraph));
}

#[test]
fn edge_vector_preflight_precedes_all_buffer_cross_reference_scans() {
    for oversized in 0..3 {
        let mut changed = candidate();
        // This earlier buffer defect must not be reached before every task's
        // edge cardinalities have been bounded, including the final task.
        changed.buffers[0].byte_len = 0;
        let last = changed.tasks.last_mut().unwrap();
        let task_id = last.id;
        match oversized {
            0 => last.inputs = vec![u32::MAX; 128],
            1 => last.outputs = vec![u32::MAX; 128],
            _ => last.predecessors = vec![u32::MAX; 128],
        }
        assert_eq!(check(changed), Err(PlanError::Graph(task_id)));
    }
    let mut changed = candidate();
    changed.buffers[0].byte_len = 0;
    changed.tasks.last_mut().unwrap().id = u32::MAX;
    assert_eq!(check(changed), Err(PlanError::Graph(u32::MAX)));
}
