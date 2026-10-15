fn wave_recipe_owner_floor(owner: &ProductionPreRankedKirOwnerV1) -> usize {
    owner.executable_storage().retained_storage() + owner.assert_origin_storage().payload_storage()
}

fn wave_recipe_pure_scalar_helper_request() -> InertSemanticMirRequestV1 {
    let seed = retained_request_with_store_role(true, 0, 4096, false, None)
        .admit(SemanticMirLimitsV1::default())
        .unwrap();
    let old = &seed.functions()[0];
    let source = old.source();
    let mut types = seed.types().to_vec();
    let ty = add_type(
        &mut types,
        scalar_layout(SemanticBackendPrimitiveV1::integer(false, 64, 8), 8),
        SemanticTypeShapeV1::Scalar(SemanticScalarTypeV1::Integer {
            signed: false,
            bits: 64,
        }),
    );
    let place =
        |index| SemanticPlaceV1::new(SemanticLocalIdV1::from_index(index), vec![], ty).unwrap();
    let assign = |destination, value| {
        SemanticStatementV1::new(
            source,
            SemanticStatementKindV1::Assign(SemanticAssignmentV1::new(
                place(destination),
                SemanticRvalueV1::new(ty, value),
            )),
        )
    };
    let local = |identity, role| {
        SemanticLocalDeclV1::new(
            SemanticLocalIdentityV1::from_sha256([identity; 32]),
            ty,
            role,
            source,
        )
    };
    let mut locals = old.locals().to_vec();
    let argument = u32::try_from(locals.len()).unwrap();
    locals.push(local(230, SemanticLocalRoleV1::Temporary));
    let output = u32::try_from(locals.len()).unwrap();
    locals.push(local(231, SemanticLocalRoleV1::Temporary));
    let mut statements = old.blocks()[0].statements().to_vec();
    statements.push(assign(
        argument,
        SemanticRvalueKindV1::Use(SemanticOperandV1::Constant(SemanticConstantV1::new(
            ty,
            SemanticConstantValueV1::Scalar(SemanticScalarValueV1::new(7, 8).unwrap()),
        ))),
    ));
    let root = SemanticFunctionDeclV1::new(
        old.identity(),
        old.role(),
        old.item_definition_identity(),
        old.monomorphization_identity(),
        old.generic_type_arguments_identity(),
        old.const_generic_arguments_identity(),
        source,
        old.abi().clone(),
        locals,
        old.entry(),
        vec![
            SemanticBasicBlockV1::new(
                old.blocks()[0].identity(),
                source,
                statements,
                SemanticTerminatorV1::new(
                    source,
                    SemanticTerminatorKindV1::Call(
                        SemanticDirectCallV1::new_callable(
                            SemanticCallableIdV1::from_index(1),
                            vec![SemanticOperandV1::Copy(place(argument))],
                            Some(SemanticCallDestinationV1::new(
                                place(output),
                                SemanticControlFlowEdgeV1::new(
                                    SemanticEdgeRoleV1::CallReturn,
                                    SemanticBlockIdV1::from_index(1),
                                ),
                            )),
                            SemanticUnwindActionV1::Unreachable,
                        )
                        .unwrap(),
                    ),
                ),
            )
            .unwrap(),
            SemanticBasicBlockV1::new(
                SemanticBlockIdentityV1::from_sha256([232; 32]),
                source,
                vec![],
                SemanticTerminatorV1::new(source, SemanticTerminatorKindV1::Return),
            )
            .unwrap(),
        ],
    )
    .unwrap()
    .with_kernel_entry(old.kernel_entry().unwrap().clone());
    let value = SemanticAbiValueV1::new(
        ty,
        SemanticAbiPassModeV1::Direct(
            SemanticAbiValueAttributesV1::new(
                SemanticAbiRegularAttributesV1::new(false, None, false, false, false, true),
                SemanticAbiExtensionV1::None,
                0,
                None,
            )
            .unwrap(),
        ),
    );
    let abi = SemanticFunctionAbiV1::from_rustc(
        SemanticAbiIdentityV1::from_sha256([241; 32]),
        old.abi().layout_identity(),
        SemanticCanonAbiV1::Rust,
        SemanticExternAbiV1::Rust,
        false,
        false,
        1,
        vec![SemanticAbiArgumentV1::source(value.clone())],
        value,
    )
    .unwrap()
    .with_source_argument_ownership(vec![SemanticSourceArgumentOwnershipV1::ByValue])
    .unwrap();
    let helper = SemanticFunctionDeclV1::new(
        SemanticFunctionIdentityV1::from_sha256([233; 32]),
        SemanticFunctionRoleV1::InternalHelper,
        SemanticItemDefinitionIdentityV1::from_sha256([234; 32]),
        SemanticMonomorphizationIdentityV1::from_sha256([235; 32]),
        SemanticGenericTypeArgumentsIdentityV1::from_sha256([236; 32]),
        SemanticConstGenericArgumentsIdentityV1::from_sha256([237; 32]),
        source,
        abi,
        vec![
            local(238, SemanticLocalRoleV1::Return),
            local(239, SemanticLocalRoleV1::Argument(0)),
        ],
        SemanticBlockIdV1::from_index(0),
        vec![
            SemanticBasicBlockV1::new(
                SemanticBlockIdentityV1::from_sha256([240; 32]),
                source,
                vec![assign(
                    0,
                    SemanticRvalueKindV1::Use(SemanticOperandV1::Copy(place(1))),
                )],
                SemanticTerminatorV1::new(source, SemanticTerminatorKindV1::Return),
            )
            .unwrap(),
        ],
    )
    .unwrap();
    InertSemanticMirRequestV1::new_with_callables(
        seed.target(),
        types,
        seed.allocations().to_vec(),
        seed.statics().to_vec(),
        seed.vtables().to_vec(),
        vec![root, helper],
        (0..2)
            .map(|index| SemanticCallableDeclV1::defined(SemanticFunctionIdV1::from_index(index)))
            .collect(),
        seed.roots().to_vec(),
    )
    .unwrap()
}

#[test]
fn wave_recipe_genuine_pure_helper_is_refused_by_local_effect_preflight() {
    let owner = materialize_retained_request(wave_recipe_pure_scalar_helper_request()).unwrap();
    assert!(!owner.grants_artifact_or_launch_authority());
    let root = SemanticFunctionIdV1::from_index(0);
    let rows = owner.correspondence.lowered_functions();
    let entry = rows
        .iter()
        .find(|row| {
            row.correspondence_owner() == root
                && row.semantic_function() == root
                && row.role() == SemanticKirFunctionRoleV1::KernelEntry
        })
        .unwrap();
    let module = owner.executable().module();
    let function = module
        .functions
        .iter()
        .find(|function| &function.id == entry.kernel_ir_function())
        .unwrap();
    let body = function.body.as_ref().unwrap();
    let calls = body
        .blocks
        .iter()
        .flat_map(|block| {
            block
                .operations
                .iter()
                .enumerate()
                .filter_map(move |(index, operation)| {
                    matches!(operation.kind, OperationKind::Call { .. })
                        .then_some((block.id.0, index, operation))
                })
        })
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 1);
    let (block, operation, call) = calls[0];
    let OperationKind::Call { callee, .. } = &call.kind else {
        unreachable!()
    };
    let helper = module
        .functions
        .iter()
        .find(|function| &function.id == callee)
        .unwrap();
    assert_eq!(helper.signature.parameters, [Type::Scalar(ScalarType::U64)]);
    assert_eq!(helper.signature.results, [Type::Scalar(ScalarType::U64)]);
    assert!(
        owner
            .empty_effect_helpers()
            .iter()
            .any(|row| row.correspondence_owner() == root
                && row.semantic_function() == SemanticFunctionIdV1::from_index(1)
                && row.kernel_ir_function() == callee)
    );
    // A complete transitive helper summary is not a local call contract.
    let effects = fe2o3_kernel_ir::analyze_interprocedural_effects_v1(module).unwrap();
    assert!(effects.function(callee).unwrap().is_complete_and_pure());
    assert!(call.memory_effects().is_empty());
    assert!(!call.has_complete_effect_summary());
    assert!(
        body.blocks
            .iter()
            .flat_map(|block| &block.operations)
            .all(|candidate| std::ptr::eq(candidate, call)
                || candidate.has_complete_effect_summary())
    );
    let floor = wave_recipe_owner_floor(&owner) + 11;
    for public_replay in [false, true] {
        let mut work = fe2o3_kernel_ir::CanonicalKernelIrWorkBudgetV1::new(4_000_000);
        let mut budget = fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1::new(
            &mut work, 16_000_000,
        );
        budget.charge_work(7).unwrap();
        budget.reserve_storage(floor).unwrap();
        let error = if public_replay {
            match owner.replay_wave_task_recipe_for_root_v1(root, &mut budget) {
                Err(error) => error,
                Ok(_) => panic!("the real pure helper must still fail local effect preflight"),
            }
        } else {
            wave_recipe_index_storage_v1(&owner, body, &mut budget).unwrap_err()
        };
        assert!(
            matches!(error, ProductionSemanticKirErrorV1::WaveRecipeIncompleteOperationEffects {
            block: actual_block, operation: actual_operation,
            kind: ProductionWaveRecipeIncompleteEffectKindV1::Call,
        } if actual_block == block && actual_operation == operation)
        );
        assert_eq!(budget.storage(), floor);
        assert!(budget.work() > 7 && budget.work() < 4_000_000);
        assert_eq!(budget.remaining_work(), 4_000_000 - budget.work());
    }
}

#[test]
fn wave_recipe_keeps_actual_read_store_pointer_and_source_ordinals() {
    let owner = materialize_retained_request(retained_request_with_store_role(
        true,
        4095,
        4096,
        false,
        Some(Role::Normalized),
    ))
    .unwrap();
    let mut work = fe2o3_kernel_ir::CanonicalKernelIrWorkBudgetV1::new(4_000_000);
    let mut budget =
        fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1::new(&mut work, 16_000_000);
    let floor = wave_recipe_owner_floor(&owner);
    budget.reserve_storage(floor).unwrap();
    let root = SemanticFunctionIdV1::from_index(0);
    let view = owner
        .replay_wave_task_recipe_for_root_v1(root, &mut budget)
        .unwrap()
        .unwrap();
    assert!(!view.grants_artifact_or_launch_authority());
    assert_eq!(view.selected_root(), root);
    assert_eq!(view.selected_function(), root);
    assert_eq!(view.function_ordinal(), 0);
    assert!(std::ptr::eq(view.executable(), owner.executable()));
    assert!(std::ptr::eq(
        view.function(),
        &owner.executable().module().functions[0]
    ));
    assert_eq!(view.roots().len(), 6);
    assert_eq!(view.accesses().len(), 2);
    assert_eq!(view.memory_census().len(), 2);
    assert_eq!(view.effect_operations().len(), 2);
    for (ordinal, row) in view.memory_census().iter().copied().enumerate() {
        assert_eq!(row.source_site(), Some((0, Some(ordinal as u32), 0)));
        assert_eq!(
            view.effect_source_site(row.location()),
            Some((0, Some(ordinal as u32)))
        );
        assert_eq!(row.memory_space(), dialect_kernel::MemorySpaceAttr::Global);
        assert_eq!(row.atomic_contract(), None);
        let ProductionWaveTaskMemoryClassV1::Global(global) = row.class() else {
            panic!("global source effects must retain their entry component");
        };
        assert_eq!(global, *view.access_at(0, Some(ordinal as u32), 0).unwrap());
        let operation = view.operation(row.location()).unwrap();
        let accesses = kir_memory_accesses_v1(operation);
        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].0, row.pointer());
        assert_eq!(accesses[0].1, row.access());
    }
    let input = view.memory_census()[0];
    let gep = view
        .function()
        .body
        .as_ref()
        .unwrap()
        .blocks
        .iter()
        .flat_map(|block| &block.operations)
        .find(|op| op.results.iter().any(|result| result.id == input.pointer()))
        .unwrap();
    let OperationKind::GetElementPointer { offset, .. } = gep.kind else {
        panic!("the actual nonzero input index must remain in KIR");
    };
    assert!(
        view.function()
            .body
            .as_ref()
            .unwrap()
            .blocks
            .iter()
            .flat_map(|block| &block.operations)
            .any(|op| op.results.iter().any(|result| result.id == offset)
                && matches!(op.kind, OperationKind::Constant(Constant::Index(4095))))
    );
    let retained = view.retained_storage();
    assert_eq!(budget.storage(), floor + retained);
    drop(view);
    budget.release_storage(retained).unwrap();
    assert_eq!(budget.storage(), floor);
}

#[test]
fn wave_recipe_keeps_ordinary_roots_separate_and_rejects_other_source_selection() {
    let wave = retained_owner(true);
    let ordinary = retained_owner(false);
    let root = SemanticFunctionIdV1::from_index(0);
    let mut work = fe2o3_kernel_ir::CanonicalKernelIrWorkBudgetV1::new(4_000_000);
    let mut budget =
        fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1::new(&mut work, 16_000_000);
    budget
        .reserve_storage(wave_recipe_owner_floor(&wave).max(wave_recipe_owner_floor(&ordinary)))
        .unwrap();
    let floor = budget.storage();
    assert!(
        ordinary
            .replay_wave_task_recipe_for_root_v1(root, &mut budget)
            .unwrap()
            .is_none()
    );
    assert_eq!(budget.storage(), floor);
    let view = wave
        .replay_wave_task_recipe_for_root_v1(root, &mut budget)
        .unwrap()
        .unwrap();
    assert!(
        view.validate_source_selection(ordinary.semantic_ssa().source_semantic(), root, root)
            .is_err()
    );
    assert!(
        view.validate_source_selection(
            wave.semantic_ssa().source_semantic(),
            root,
            SemanticFunctionIdV1::from_index(1)
        )
        .is_err()
    );
    assert!(
        wave.replay_wave_task_recipe_for_root_v1(SemanticFunctionIdV1::from_index(1), &mut budget,)
            .is_err()
    );
}

#[test]
fn wave_recipe_shared_budget_exact_and_one_short_preserve_owner_floor() {
    let owner = retained_owner(true);
    let root = SemanticFunctionIdV1::from_index(0);
    let owner_floor = wave_recipe_owner_floor(&owner);
    let floor = owner_floor + 11;
    let mut work = fe2o3_kernel_ir::CanonicalKernelIrWorkBudgetV1::new(4_000_000);
    let mut budget =
        fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1::new(&mut work, 16_000_000);
    budget.charge_work(7).unwrap();
    budget.reserve_storage(floor).unwrap();
    let view = owner
        .replay_wave_task_recipe_for_root_v1(root, &mut budget)
        .unwrap()
        .unwrap();
    let exact_work = budget.work();
    let exact_storage = budget.peak_storage();
    let retained = view.retained_storage();
    drop(view);
    budget.release_storage(retained).unwrap();
    assert_eq!(budget.storage(), floor);
    for (work_limit, storage_limit, succeeds) in [
        (exact_work, exact_storage, true),
        (exact_work - 1, exact_storage, false),
        (exact_work, exact_storage - 1, false),
    ] {
        let mut work = fe2o3_kernel_ir::CanonicalKernelIrWorkBudgetV1::new(work_limit);
        let mut budget = fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1::new(
            &mut work,
            storage_limit,
        );
        budget.charge_work(7).unwrap();
        budget.reserve_storage(floor).unwrap();
        let result = owner.replay_wave_task_recipe_for_root_v1(root, &mut budget);
        assert_eq!(result.is_ok(), succeeds);
        if let Ok(Some(view)) = result {
            let retained = view.retained_storage();
            drop(view);
            budget.release_storage(retained).unwrap();
        }
        assert_eq!(budget.storage(), floor);
        assert!(budget.work() >= 7 && budget.work() <= work_limit);
        assert_eq!(budget.remaining_work(), work_limit - budget.work());
    }
    let mut work = fe2o3_kernel_ir::CanonicalKernelIrWorkBudgetV1::new(exact_work);
    let mut budget = fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1::new(
        &mut work,
        exact_storage,
    );
    budget.reserve_storage(owner_floor - 1).unwrap();
    assert!(
        owner
            .replay_wave_task_recipe_for_root_v1(root, &mut budget)
            .is_err()
    );
    assert_eq!(budget.storage(), owner_floor - 1);
    assert_eq!(budget.work(), 0);
}

#[test]
fn wave_recipe_malformed_correlation_charges_work_without_losing_incoming_floors() {
    let mut owner = retained_owner(true);
    let original = owner.correspondence.parameter_component_bindings.to_vec();
    owner.correspondence.parameter_component_bindings = original[1..].to_vec().into_boxed_slice();
    let floor = wave_recipe_owner_floor(&owner) + 11;
    let mut work = fe2o3_kernel_ir::CanonicalKernelIrWorkBudgetV1::new(4_000_000);
    let mut budget =
        fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1::new(&mut work, 16_000_000);
    budget.charge_work(7).unwrap();
    budget.reserve_storage(floor).unwrap();
    let root = SemanticFunctionIdV1::from_index(0);
    assert!(
        owner
            .replay_wave_task_recipe_for_root_v1(root, &mut budget)
            .is_err()
    );
    assert!(budget.work() > 7);
    assert!(budget.work() < 4_000_000);
    assert_eq!(budget.storage(), floor);
    assert_eq!(budget.remaining_work(), 4_000_000 - budget.work());
}

#[derive(Clone, Copy)]
enum WaveRecipeBindingCase {
    Exact,
    SameTypeOriginSwap,
    WrongSourceOrdinal,
    MissingMemoryRow,
    DuplicateMemoryRow,
}

#[test]
fn wave_recipe_preflight_rejects_empty_summaries_from_unknown_effects() {
    let owner = retained_owner(true);
    // These are inert negative operations, not an admitted assembly identity.
    // Empty declared effects must never make either operation appear complete.
    let cases = [
        OperationKind::Call {
            callee: FunctionId::new("unknown_effects"),
            arguments: vec![],
        },
        OperationKind::InlineAssembly(fe2o3_kernel_ir::InlineAssembly {
            target: fe2o3_kernel_ir::InlineAssemblyTarget::AmdGpuGfx942,
            source: fe2o3_kernel_ir::AssemblySourceIdentity::new(
                [0; 32], [0; 32], [0; 32], [0; 32],
            ),
            mnemonic: "s_nop 0".into(),
            operands: vec![],
            options: BTreeSet::from([fe2o3_kernel_ir::AssemblyOption::NoMemory]),
            declared_effects: BTreeSet::new(),
        }),
    ];
    for kind in cases {
        let expected_kind = match &kind {
            OperationKind::Call { .. } => ProductionWaveRecipeIncompleteEffectKindV1::Call,
            OperationKind::InlineAssembly(_) => {
                ProductionWaveRecipeIncompleteEffectKindV1::InlineAssembly
            }
            _ => unreachable!("the negative fixture has two closed operation kinds"),
        };
        let operation = Operation::new(vec![], kind);
        assert!(operation.memory_effects().is_empty());
        assert!(!operation.has_complete_effect_summary());
        let mut body = owner.executable().module().functions[0]
            .body
            .clone()
            .unwrap();
        let expected_block = body.blocks[0].id.0;
        let expected_operation = body.blocks[0].operations.len();
        body.blocks[0].operations.push(operation);
        let mut work = fe2o3_kernel_ir::CanonicalKernelIrWorkBudgetV1::new(4_000_000);
        let mut budget = fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1::new(
            &mut work, 16_000_000,
        );
        budget.charge_work(7).unwrap();
        let floor = wave_recipe_owner_floor(&owner) + 11;
        budget.reserve_storage(floor).unwrap();
        assert!(matches!(
            wave_recipe_index_storage_v1(&owner, &body, &mut budget),
            Err(ProductionSemanticKirErrorV1::WaveRecipeIncompleteOperationEffects {
                block, operation, kind,
            }) if block == expected_block && operation == expected_operation && kind == expected_kind
        ));
        assert_eq!(budget.storage(), floor);
        assert!(budget.work() > 7);
    }
}

#[test]
fn wave_recipe_incomplete_effect_diagnostic_is_bounded_and_keeps_error_chain() {
    use std::error::Error;
    for (kind, label) in [
        (ProductionWaveRecipeIncompleteEffectKindV1::Call, "call"),
        (
            ProductionWaveRecipeIncompleteEffectKindV1::InlineAssembly,
            "inline-assembly",
        ),
        (ProductionWaveRecipeIncompleteEffectKindV1::Other, "other"),
    ] {
        let error = ProductionSemanticKirErrorV1::WaveRecipeIncompleteOperationEffects {
            block: u32::MAX,
            operation: usize::MAX,
            kind,
        };
        assert_eq!(
            error.to_string(),
            format!(
                "wave recipe operation lacks a complete local effect summary: block {}; operation {}; kind {label}",
                u32::MAX,
                usize::MAX,
            )
        );
        assert!(error.to_string().len() < 160);
        assert!(error.source().is_none());
    }
    assert_eq!(
        ProductionSemanticKirErrorV1::CorrespondenceMismatch.to_string(),
        "semantic-to-Kernel-IR correspondence no longer matches its exact owner",
    );
}

#[test]
fn wave_recipe_incomplete_effect_diagnostic_preserves_first_failure_and_work_limit() {
    let owner = retained_owner(true);
    let mut body = owner.executable().module().functions[0]
        .body
        .clone()
        .unwrap();
    let block = body.blocks[0].id.0;
    let first = body.blocks[0].operations.len();
    for name in ["first_unknown", "second_unknown"] {
        body.blocks[0].operations.push(Operation::new(
            vec![],
            OperationKind::Call {
                callee: FunctionId::new(name),
                arguments: vec![],
            },
        ));
    }
    let floor = wave_recipe_owner_floor(&owner) + 11;
    let mut work = fe2o3_kernel_ir::CanonicalKernelIrWorkBudgetV1::new(4_000_000);
    let mut budget =
        fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1::new(&mut work, 16_000_000);
    budget.charge_work(7).unwrap();
    budget.reserve_storage(floor).unwrap();
    let error = wave_recipe_index_storage_v1(&owner, &body, &mut budget).unwrap_err();
    assert!(matches!(error,
        ProductionSemanticKirErrorV1::WaveRecipeIncompleteOperationEffects {
            block: actual_block, operation, kind: ProductionWaveRecipeIncompleteEffectKindV1::Call,
        } if actual_block == block && operation == first
    ));
    let required_work = budget.work();
    assert!(required_work > 7);
    assert_eq!(budget.storage(), floor);
    for (limit, diagnostic) in [(required_work, true), (required_work - 1, false)] {
        let mut work = fe2o3_kernel_ir::CanonicalKernelIrWorkBudgetV1::new(limit);
        let mut budget = fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1::new(
            &mut work, 16_000_000,
        );
        budget.charge_work(7).unwrap();
        budget.reserve_storage(floor).unwrap();
        let error = wave_recipe_index_storage_v1(&owner, &body, &mut budget).unwrap_err();
        if diagnostic {
            assert!(matches!(error,
                ProductionSemanticKirErrorV1::WaveRecipeIncompleteOperationEffects {
                    block: actual_block, operation,
                    kind: ProductionWaveRecipeIncompleteEffectKindV1::Call,
                } if actual_block == block && operation == first
            ));
        } else {
            assert!(matches!(
                error,
                ProductionSemanticKirErrorV1::MirPlironTranslation(
                    ProductionMirPlironTranslationErrorV1::ResourceLimit,
                )
            ));
        }
        assert_eq!(budget.storage(), floor);
        assert!(budget.work() <= limit);
        assert_eq!(budget.remaining_work(), limit - budget.work());
    }
}

fn wave_recipe_small_conditional_input(
    owner: &ProductionPreRankedKirOwnerV1,
    case: WaveRecipeBindingCase,
) -> Result<
    (
        ProductionRankedSemanticChecksV1,
        Vec<ProductionRankedAccessSourceV1>,
    ),
    fe2o3_pliron::ProductionWaveTaskConditionalCompileErrorV1,
> {
    use fe2o3_pliron::{
        ProductionConstructionV1, ProductionRankedBlockV1, ProductionRankedKernelV1,
        ProductionRankedTerminatorV1, ProductionSessionLimitsV1,
        ProductionWaveTaskRankedEffectBindingV1,
        compile_ranked_kernel_for_conditional_wave_tasks_v1,
    };
    let mut work = fe2o3_kernel_ir::CanonicalKernelIrWorkBudgetV1::new(4_000_000);
    let mut budget =
        fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1::new(&mut work, 16_000_000);
    budget
        .reserve_storage(wave_recipe_owner_floor(owner))
        .unwrap();
    let view = owner
        .replay_wave_task_recipe_for_root_v1(SemanticFunctionIdV1::from_index(0), &mut budget)
        .unwrap()
        .unwrap();
    let mut operations = vec![ProductionRankedOperationV1::ExecutionLayout {
        grid_identity: 1,
        global_extents: [64, 1, 1],
        workgroup_extents: [64, 1, 1],
        subgroup_size: 64,
        full_physical_workgroups: true,
    }];
    let mut sources = Vec::new();
    let mut bindings = Vec::new();
    for (index, row) in view.memory_census().iter().enumerate() {
        let ProductionWaveTaskMemoryClassV1::Global(global) = row.class() else {
            panic!("this small integration fixture has only original global effects");
        };
        let (block, statement, mut ordinal) = row.source_site().unwrap();
        let mut origin = global.root().allocation_origin();
        if index == 0 {
            match case {
                WaveRecipeBindingCase::SameTypeOriginSwap => {
                    origin = view
                        .roots()
                        .iter()
                        .find(|root| root.role() == Role::NormWeight)
                        .unwrap()
                        .allocation_origin();
                }
                WaveRecipeBindingCase::WrongSourceOrdinal => ordinal += 1,
                _ => {}
            }
        }
        let ranked_operation = u32::try_from(operations.len()).unwrap();
        operations.push(ProductionRankedOperationV1::AllocationEffect {
            kind: row.access(),
            memory_space: row.memory_space(),
            allocation_origin: origin,
            noalias_class: 0,
        });
        sources.push(ProductionRankedAccessSourceV1::new(
            block,
            statement,
            ordinal,
            0,
            ranked_operation,
        ));
        bindings.push(ProductionWaveTaskRankedEffectBindingV1::new(
            Some((block, statement, ordinal)),
            row.location(),
            row.operation_access_ordinal(),
            Some((0, ranked_operation)),
        ));
    }
    match case {
        WaveRecipeBindingCase::MissingMemoryRow => {
            bindings.pop();
        }
        WaveRecipeBindingCase::DuplicateMemoryRow => bindings.push(bindings[0]),
        _ => {}
    }
    let kernel = ProductionRankedKernelV1::new(
        view.function().id.as_str(),
        0,
        vec![ProductionRankedBlockV1::new(
            operations,
            ProductionRankedTerminatorV1::Return,
        )],
    )
    .unwrap();
    let input = compile_ranked_kernel_for_conditional_wave_tasks_v1(
        ProductionConstructionV1::ranked_kernel("wave_recipe_custody", kernel).unwrap(),
        ProductionSessionLimitsV1::default(),
        view.executable(),
        view.function_ordinal(),
        &bindings,
        &[],
        &mut budget,
    );
    let retained = view.retained_storage();
    drop(view);
    budget.release_storage(retained).unwrap();
    input.map(|input| {
        (
            ProductionRankedSemanticChecksV1::ConditionalWaveTasks(input),
            sources,
        )
    })
}

fn wave_recipe_two_global_owner(index: u64) -> ProductionPreRankedKirOwnerV1 {
    materialize_retained_request(retained_request_with_store_role(
        true,
        index,
        index + 1,
        false,
        Some(Role::Normalized),
    ))
    .unwrap()
}

#[test]
fn wave_recipe_conditional_join_replays_exact_source_and_every_memory_effect() {
    let owner = wave_recipe_two_global_owner(4095);
    let (checks, sources) =
        wave_recipe_small_conditional_input(&owner, WaveRecipeBindingCase::Exact).unwrap();
    assert!(checks.ordinary().is_none());
    assert!(!checks.all_mandatory_reports_are_clean());
    validate_retained_conditional_ranked_owner_v1(
        owner.semantic_ssa(),
        owner.executable(),
        &owner.correspondence,
        SemanticFunctionIdV1::from_index(0),
        &checks,
        &sources,
        &[],
        owner.limits,
    )
    .unwrap();
    let result = validate_mir_pliron_translation_with_semantic_v1(
        Some(owner.semantic_ssa().source_semantic()),
        owner.executable().module(),
        &owner.correspondence,
        owner.executable().module().kernels[0].id.as_str(),
        &checks,
        &sources,
        &[],
        owner.limits.max_operations,
    )
    .unwrap();
    assert_eq!(result.memory_effects, 2);
    assert_eq!(result.conservative_ranked_effects, 2);
    let other = wave_recipe_two_global_owner(4094);
    assert!(
        validate_retained_conditional_ranked_owner_v1(
            other.semantic_ssa(),
            other.executable(),
            &other.correspondence,
            SemanticFunctionIdV1::from_index(0),
            &checks,
            &sources,
            &[],
            other.limits,
        )
        .is_err()
    );
}

#[test]
fn wave_recipe_conditional_join_rejects_wrong_roots_ordinals_and_incomplete_census() {
    let owner = wave_recipe_two_global_owner(4095);
    for case in [
        WaveRecipeBindingCase::SameTypeOriginSwap,
        WaveRecipeBindingCase::WrongSourceOrdinal,
    ] {
        let (checks, sources) = wave_recipe_small_conditional_input(&owner, case).unwrap();
        assert!(
            validate_retained_conditional_ranked_owner_v1(
                owner.semantic_ssa(),
                owner.executable(),
                &owner.correspondence,
                SemanticFunctionIdV1::from_index(0),
                &checks,
                &sources,
                &[],
                owner.limits,
            )
            .is_err()
        );
    }
    for case in [
        WaveRecipeBindingCase::MissingMemoryRow,
        WaveRecipeBindingCase::DuplicateMemoryRow,
    ] {
        assert!(wave_recipe_small_conditional_input(&owner, case).is_err());
    }
}
