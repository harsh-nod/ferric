#[test]
#[ignore = "requires an independently retained actual V39 source capture and its digest"]
fn actual_mlp_v39_reaches_formal_archive_and_outer_lineage_replay() {
    use crate::production_ranked_projection_v1 as rp;
    use fe2o3_artifacts::{BlockSize, Dimensions, LaunchContract};
    use fe2o3_kernel_ir::{
        CanonicalKernelIrVerificationResourceBudgetV1 as Budget,
        CanonicalKernelIrWorkBudgetV1 as Work, SemanticWaveTaskProfileV3 as Profile,
    };
    use fe2o3_lower_mir_kernel::{
        ProductionPreRankedKirOwnerV1 as Pre, ProductionSemanticKirOwnerV1 as Kir,
    };
    use fe2o3_mir_model::semantic_mir_v1::*;
    use sha2::{Digest as _, Sha256};

    let path = std::env::var("FE2O3_MLP_TILE_V2_SEMANTIC_CAPTURE").unwrap();
    let expected_sha = std::env::var("FE2O3_MLP_TILE_V2_SEMANTIC_CAPTURE_SHA256").unwrap();
    assert_eq!(expected_sha.len(), 64);
    assert!(expected_sha.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(expected_sha.bytes().any(|byte| byte != b'0'));
    assert!(std::fs::metadata(&path).unwrap().len() <= 16 * 1024 * 1024);
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(
        Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        expected_sha
    );
    let semantic =
        AdmittedInertSemanticMirV1::decode_exact_v39_canonical(&bytes, Default::default()).unwrap();
    assert_eq!(semantic.canonical_encoding(), bytes);
    assert!(
        AdmittedInertSemanticMirV1::decode_exact_v37_canonical(&bytes, Default::default()).is_err()
    );
    assert!(
        AdmittedInertSemanticMirV1::decode_exact_v38_canonical(&bytes, Default::default()).is_err()
    );
    assert_eq!(semantic.roots().len(), 1);
    let root = semantic.roots()[0];
    let selected = semantic.select_kernel_body_for_root_v1(root).unwrap();
    assert_eq!(
        semantic.functions()[selected.body().index() as usize]
            .abi()
            .source_argument_ownership(),
        [SemanticSourceArgumentOwnershipV1::WaveMlpTileStorage64OwnerV2]
    );
    let abi = semantic.functions()[selected.body().index() as usize].abi();
    assert_eq!(abi.source_input_types().len(), 1);
    let storage = &semantic.types()[abi.source_input_types()[0].index() as usize];
    assert_eq!(storage.layout().size_bytes(), Some(88));
    assert_eq!(storage.layout().alignment_bytes(), 8);
    let SemanticTypeShapeV1::Aggregate(fields) = storage.shape() else {
        panic!("actual MLP storage must retain eleven roots and two markers");
    };
    assert_eq!(fields.fields().len(), 13);
    let entry = semantic.functions()[root.index() as usize]
        .kernel_entry()
        .unwrap();
    let symbol = std::str::from_utf8(entry.export_symbol().as_bytes())
        .unwrap()
        .to_owned();
    assert_eq!(symbol, "ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2");
    let binding = *entry.kernel_binding_identity().as_bytes();
    let launch = LaunchContract::new(
        1,
        BlockSize::Exact(Dimensions::new(64, 1, 1).unwrap()),
        Dimensions::new(64, 1, 1).unwrap(),
        512,
        0,
    )
    .unwrap();
    let input = rp::ProductionRankedRootInputV1::new(&symbol, binding, &launch);
    let mir =
        fe2o3_pliron::ProductionSemanticMirOwnerV1::try_new(semantic, Default::default()).unwrap();
    let ssa = fe2o3_pliron::ProductionSemanticSsaOwnerV1::try_new(mir, Default::default()).unwrap();
    let roster =
        rp::source_launch_roster_for_ranked_inputs_v1(&ssa, std::slice::from_ref(&input)).unwrap();
    let mut work = Work::new(
        usize::try_from(crate::production_canonical_phase_policy_v1::WORK_LIMIT).unwrap(),
    );
    let mut budget = Budget::new(
        &mut work,
        crate::production_canonical_phase_policy_v1::STORAGE_LIMIT,
    );
    eprintln!("stage=pre-ranked status=starting");
    let owner =
        Pre::try_materialize_with_budget(ssa, roster, Default::default(), &mut budget).unwrap();
    let floor = owner
        .executable_storage()
        .retained_storage()
        .checked_add(owner.assert_origin_storage().payload_storage())
        .unwrap();
    budget.reserve_storage(floor).unwrap();
    let recipe = owner
        .replay_wave_task_recipe_for_root_v1(root, &mut budget)
        .unwrap()
        .unwrap();
    assert_eq!(
        recipe.roots().iter().map(|r| r.role()).collect::<Vec<_>>(),
        Profile::MlpTilesV2.roles()
    );
    assert_eq!(recipe.roots().len(), 11);
    assert_eq!(Profile::MlpTilesV2.carrier_bytes(), 88);
    for role in Profile::MlpTilesV2.roles() {
        assert!(
            recipe
                .accesses()
                .iter()
                .any(|access| access.root().role() == *role)
        );
    }
    eprintln!(
        "stage=recipe status=complete roots={} accesses={} memory={} effects={}",
        recipe.roots().len(),
        recipe.accesses().len(),
        recipe.memory_census().len(),
        recipe.effect_operations().len()
    );
    let retained = recipe.retained_storage();
    drop(recipe);
    budget.release_storage(retained).unwrap();

    // This source registers no safe-Rust numerical reference. Empty bindings
    // grant no numerical refinement; all conditional runtime requirements remain.
    let references =
        crate::reference_effect_v1::AuthenticatedReferenceEffectBindingsV1::new(vec![]);
    eprintln!("stage=ranked status=starting");
    let program =
        rp::project_and_verify_ranked_materialized_semantic_mir_v1(owner, &[input], &references)
            .unwrap();
    eprintln!(
        "stage=ranked status=complete roots={} bytes={}",
        program.root_count(),
        program.roots()[0].ranked_ir().len()
    );
    let (receipt, ranked) = program
        .into_verified_roster_receipt()
        .unwrap()
        .into_module_verified_receipt()
        .unwrap();
    let lowered = Kir::try_attach_materialized_ranked_checks(receipt).unwrap();
    lowered.verify_equivalence().unwrap();
    // The actual MLP body must retain its authenticated exp terminal;
    // declaration-only fixtures are not accepted as MLP source evidence.
    assert!(
        lowered
            .module()
            .functions
            .iter()
            .filter(|function| function.role == fe2o3_kernel_ir::FunctionRole::KernelEntry)
            .filter_map(|function| function.body.as_ref())
            .flat_map(|body| body.blocks.iter())
            .flat_map(|block| block.operations.iter())
            .any(|operation| {
                let fe2o3_kernel_ir::OperationKind::Call { callee, .. } = &operation.kind else {
                    return false;
                };
                matches!(
                    fe2o3_kernel_ir::FloatOperation::from_intrinsic_id(callee),
                    Some(fe2o3_kernel_ir::FloatOperation::F32Math {
                        function: fe2o3_kernel_ir::F32MathFunction::Exp,
                        implementation: fe2o3_kernel_ir::F32MathImplementation::OcmlAbiV1,
                        ..
                    })
                )
            })
    );
    assert_eq!(lowered.conditional_wave_task_reports().count(), 1);
    let (_, report) = lowered.conditional_wave_task_reports().next().unwrap();
    assert_eq!(report.unresolved_requirements().requirements().len(), 8);
    assert_eq!(
        wave_source_profile_v3(lowered.semantic().semantic()),
        Ok(Profile::MlpTilesV2)
    );
    assert_eq!(
        wave_source_profile_v2(lowered.semantic().semantic()),
        Err("QKV post ownership has no six-root formal profile")
    );
    let formal = CompilerFormalMemoryV1::from_lowered(
        lowered,
        fe2o3_amd_target::ProductionAmdTargetProfileV1::Gfx950,
    )
    .unwrap();
    formal.verify_equivalence().unwrap();
    assert_eq!(formal.raw_witness_extent(), 4096);
    assert!(
        formal
            .require_ordinary_handoff("MLP remains conditional")
            .is_err()
    );
    let CompilerFormalMemoryV1::ConditionalWaveMlpTilesV2(owner) = formal else {
        panic!("actual MLP capture must retain its distinct formal owner");
    };
    assert_eq!(owner.roots().len(), 11);
    assert_eq!(
        fe2o3_kernel_ir::SemanticWaveTaskRoleV1::MlpTileStateV2.elements(),
        548
    );
    assert_eq!(owner.unresolved_requirements().len(), 8);
    assert!(!owner.grants_artifact_or_launch_authority());
    assert!(!owner.grants_compiler_refinement_authority());
    eprintln!("stage=formal status=complete roots=11 unresolved=8");
    crate::production_semantic_lineage_v3::replay_actual_mlp_tile_lineage_v2(&owner, ranked)
        .unwrap();
}
