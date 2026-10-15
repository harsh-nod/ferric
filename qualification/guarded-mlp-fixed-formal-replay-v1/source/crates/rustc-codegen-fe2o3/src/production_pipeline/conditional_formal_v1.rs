/// Move-only distinction between ordinary complete admission and conditional
/// custody. No conversion from the conditional variant to the ordinary owner.
pub(crate) enum CompilerFormalMemoryV1 {
    ConditionalMultiwaveJoinV1(
        fe2o3_lower_mir_kernel::ProductionConditionalMultiwaveJoinFormalMemoryOwnerV1,
    ),
    Ordinary(fe2o3_lower_mir_kernel::ProductionFormalMemoryOwnerV1),
    ConditionalFiniteJoin(
        fe2o3_lower_mir_kernel::ProductionConditionalFiniteJoinFormalMemoryOwnerV1,
    ),
    ConditionalWaveTasks(fe2o3_lower_mir_kernel::ProductionConditionalWaveTaskFormalMemoryOwnerV1),
    ConditionalWaveQkvTasksV2(
        fe2o3_lower_mir_kernel::ProductionConditionalWaveQkvTaskFormalMemoryOwnerV2,
    ),
    ConditionalWaveQkvPostTasksV3(
        fe2o3_lower_mir_kernel::ProductionConditionalWaveQkvPostTaskFormalMemoryOwnerV3,
    ),
    ConditionalWaveQkvAttentionTasksV4(
        fe2o3_lower_mir_kernel::ProductionConditionalWaveQkvAttentionTaskFormalMemoryOwnerV4,
    ),
    ConditionalWaveQkvAttentionOutputTasksV5(
        fe2o3_lower_mir_kernel::ProductionConditionalWaveQkvAttentionOutputTaskFormalMemoryOwnerV5,
    ),
    ConditionalWaveQkvAttentionOutputTilesV6(
        fe2o3_lower_mir_kernel::ProductionConditionalWaveQkvAttentionOutputTileFormalMemoryOwnerV6,
    ),
    ConditionalWaveMlpTasksV1(
        fe2o3_lower_mir_kernel::ProductionConditionalWaveMlpTaskFormalMemoryOwnerV1,
    ),
    ConditionalWaveMlpTilesV2(
        fe2o3_lower_mir_kernel::ProductionConditionalWaveMlpTileFormalMemoryOwnerV2,
    ),
}

/// The selected semantic body, not an archive header or pointer type, owns the profile.
pub(crate) fn wave_source_profile_v2(
    semantic: &fe2o3_mir_model::semantic_mir_v1::AdmittedInertSemanticMirV1,
) -> Result<fe2o3_kernel_ir::SemanticWaveTaskProfileV2, &'static str> {
    use fe2o3_kernel_ir::SemanticWaveTaskProfileV2 as Profile;
    use fe2o3_mir_model::semantic_mir_v1::SemanticSourceArgumentOwnershipV1 as Owner;
    let error = "one exact wave-task source profile required";
    let [root] = semantic.roots() else {
        return Err(error);
    };
    let selected = semantic
        .select_kernel_body_for_root_v1(*root)
        .ok_or(error)?;
    let body = semantic
        .functions()
        .get(selected.body().index() as usize)
        .ok_or(error)?;
    let owners = body.abi().source_argument_ownership();
    if owners.contains(&Owner::WaveQkvPostTaskStorage64OwnerV3)
        || owners.contains(&Owner::WaveQkvAttentionTaskStorage64OwnerV4)
        || owners.contains(&Owner::WaveQkvAttentionOutputTaskStorage64OwnerV5)
        || owners.contains(&Owner::WaveQkvAttentionOutputTileStorage64OwnerV6)
        || owners.contains(&Owner::WaveMlpTaskStorage64OwnerV1)
        || owners.contains(&Owner::WaveMlpTileStorage64OwnerV2)
        || owners.contains(&Owner::MultiwaveJoinStorage128OwnerV1)
    {
        return Err("QKV post ownership has no six-root formal profile");
    }
    match owners.first() {
        Some(Owner::WaveTaskStorage64OwnerV1)
            if !owners.contains(&Owner::WaveQkvTaskStorage64OwnerV2) =>
        {
            Ok(Profile::KeyV1)
        }
        Some(Owner::WaveQkvTaskStorage64OwnerV2)
            if !owners.contains(&Owner::WaveTaskStorage64OwnerV1) =>
        {
            Ok(Profile::QkvV2)
        }
        _ => Err(error),
    }
}

/// Extends source dispatch without broadening the legacy six-root profile API.
pub(crate) fn wave_source_profile_v3(
    semantic: &fe2o3_mir_model::semantic_mir_v1::AdmittedInertSemanticMirV1,
) -> Result<fe2o3_kernel_ir::SemanticWaveTaskProfileV3, &'static str> {
    use fe2o3_mir_model::semantic_mir_v1::SemanticSourceArgumentOwnershipV1 as Owner;
    let error = "one exact wave-task source profile required";
    let [root] = semantic.roots() else {
        return Err(error);
    };
    let selected = semantic
        .select_kernel_body_for_root_v1(*root)
        .ok_or(error)?;
    let body = semantic
        .functions()
        .get(selected.body().index() as usize)
        .ok_or(error)?;
    let owners = body.abi().source_argument_ownership();
    if owners == [Owner::MultiwaveJoinStorage128OwnerV1] {
        return Ok(fe2o3_kernel_ir::SemanticWaveTaskProfileV3::MultiwaveJoinV1);
    }
    if owners.contains(&Owner::MultiwaveJoinStorage128OwnerV1) {
        return Err(error);
    }
    if owners == [Owner::WaveMlpTaskStorage64OwnerV1] {
        return Ok(fe2o3_kernel_ir::SemanticWaveTaskProfileV3::MlpV1);
    }
    if owners == [Owner::WaveMlpTileStorage64OwnerV2] {
        return Ok(fe2o3_kernel_ir::SemanticWaveTaskProfileV3::MlpTilesV2);
    }
    if owners.contains(&Owner::WaveMlpTaskStorage64OwnerV1) {
        return Err(error);
    }
    if owners.contains(&Owner::WaveMlpTileStorage64OwnerV2) {
        return Err(error);
    }
    if owners == [Owner::WaveQkvAttentionOutputTaskStorage64OwnerV5] {
        return Ok(fe2o3_kernel_ir::SemanticWaveTaskProfileV3::QkvAttentionOutputV5);
    }
    if owners.contains(&Owner::WaveQkvAttentionOutputTaskStorage64OwnerV5) {
        return Err(error);
    }
    if owners == [Owner::WaveQkvAttentionOutputTileStorage64OwnerV6] {
        return Ok(fe2o3_kernel_ir::SemanticWaveTaskProfileV3::QkvAttentionOutputTilesV6);
    }
    if owners.contains(&Owner::WaveQkvAttentionOutputTileStorage64OwnerV6) {
        return Err(error);
    }
    if owners.first() == Some(&Owner::WaveQkvAttentionTaskStorage64OwnerV4)
        && !owners.contains(&Owner::WaveTaskStorage64OwnerV1)
        && !owners.contains(&Owner::WaveQkvTaskStorage64OwnerV2)
        && !owners.contains(&Owner::WaveQkvPostTaskStorage64OwnerV3)
    {
        return Ok(fe2o3_kernel_ir::SemanticWaveTaskProfileV3::QkvAttentionV4);
    }
    if owners.first() == Some(&Owner::WaveQkvPostTaskStorage64OwnerV3)
        && !owners.contains(&Owner::WaveTaskStorage64OwnerV1)
        && !owners.contains(&Owner::WaveQkvTaskStorage64OwnerV2)
        && !owners.contains(&Owner::WaveQkvAttentionTaskStorage64OwnerV4)
    {
        return Ok(fe2o3_kernel_ir::SemanticWaveTaskProfileV3::QkvPostV3);
    }
    wave_source_profile_v2(semantic).map(Into::into)
}

/// One existing compiler phase ledger, shared by the complete formal action.
/// A new ledger is never created for an individual operation or diagnostic row.
pub(crate) fn with_wave_formal_budget_v1<T>(
    retained_storage: usize,
    action: impl FnOnce(
        &mut fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1<'_>,
    ) -> Result<T, fe2o3_lower_mir_kernel::ProductionWaveTaskFormalMemoryErrorV1>,
) -> Result<T, fe2o3_lower_mir_kernel::ProductionWaveTaskFormalMemoryErrorV1> {
    with_wave_formal_phase_budget_v1(
        retained_storage,
        fe2o3_lower_mir_kernel::ProductionWaveTaskFormalMemoryErrorV1::Budget,
        action,
    )
}

/// The same phase policy also covers the lossless archive retained by lineage.
/// The error adapter preserves each caller's exact resource diagnostic.
pub(crate) fn with_wave_formal_phase_budget_v1<T, E>(
    retained_storage: usize,
    resource: impl Fn(fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceErrorV1) -> E,
    action: impl FnOnce(
        &mut fe2o3_kernel_ir::CanonicalKernelIrVerificationResourceBudgetV1<'_>,
    ) -> Result<T, E>,
) -> Result<T, E> {
    use fe2o3_kernel_ir::{
        CanonicalKernelIrVerificationResourceBudgetV1,
        CanonicalKernelIrVerificationResourceErrorV1, CanonicalKernelIrWorkBudgetV1,
    };
    let limit = usize::try_from(crate::production_canonical_phase_policy_v1::WORK_LIMIT)
        .map_err(|_| resource(CanonicalKernelIrVerificationResourceErrorV1::Arithmetic))?;
    let mut work = CanonicalKernelIrWorkBudgetV1::new(limit);
    let mut budget = CanonicalKernelIrVerificationResourceBudgetV1::new(
        &mut work,
        crate::production_canonical_phase_policy_v1::STORAGE_LIMIT,
    );
    budget.reserve_storage(retained_storage).map_err(resource)?;
    action(&mut budget)
}

#[cfg(test)]
mod conditional_formal_tests {
    use super::*;

    include!("wave_task_formal_v1_tests.rs");
    include!("wave_qkv_post_capture_v3_tests.rs");
    include!("wave_qkv_attention_capture_v4_tests.rs");
    include!("wave_qkv_attention_output_capture_v5_tests.rs");
    include!("wave_qkv_attention_output_formal_v5_tests.rs");
    include!("wave_mlp_formal_v1_tests.rs");
    include!("wave_mlp_tile_formal_v2_tests.rs");
    include!("multiwave_join_formal_v1_tests.rs");
    include!("wave_mlp_capture_v1_tests.rs");
    include!("wave_mlp_tile_capture_v2_tests.rs");
    include!("multiwave_join_capture_v1_tests.rs");
    include!("prefix_tile_formal_v6_tests.rs");
    include!("prefix_tile_capture_v6_tests.rs");
    include!("wave_qkv_post_formal_v3_tests.rs");

    #[test]
    fn conditional_formal_pipeline_keeps_v2_and_protected_handoffs_closed() {
        let (lowered, _) =
            crate::production_ranked_projection_v1::conditional_finite_join_owner_fixture_v1();
        let formal = CompilerFormalMemoryV1::from_lowered(
            lowered,
            fe2o3_amd_target::ProductionAmdTargetProfileV1::Gfx950,
        )
        .unwrap();
        assert!(matches!(
            &formal,
            CompilerFormalMemoryV1::ConditionalFiniteJoin(_)
        ));
        formal.verify_equivalence().unwrap();
        assert_eq!(formal.raw_kernel_count(), 1);
        assert_eq!(formal.raw_witness_extent(), 256);
        for policy in [
            "V2-only extraction drops conditional runtime premises",
            "protected publication lacks conditional proof admission",
        ] {
            assert!(matches!(formal.require_ordinary_handoff(policy),
                Err(ProductionPipelineError::ConditionalFormalPolicy(actual)) if actual == policy));
        }
    }

    #[test]
    fn conditional_formal_pipeline_rejects_non_gfx950_target() {
        let (lowered, _) =
            crate::production_ranked_projection_v1::conditional_finite_join_owner_fixture_v1();
        assert!(matches!(
            CompilerFormalMemoryV1::from_lowered(
                lowered,
                fe2o3_amd_target::ProductionAmdTargetProfileV1::Gfx942
            ),
            Err(ProductionPipelineError::ConditionalFormalPolicy(
                "only gfx950 conditional extraction is supported"
            ))
        ));
    }
}

impl CompilerFormalMemoryV1 {
    fn from_lowered(
        lowered: fe2o3_lower_mir_kernel::ProductionSemanticKirOwnerV1,
        target: fe2o3_amd_target::ProductionAmdTargetProfileV1,
    ) -> Result<Self, ProductionPipelineError> {
        let conditional = lowered.conditional_finite_join_reports().next().is_some();
        let wave = lowered.conditional_wave_task_reports().next().is_some();
        if wave {
            if conditional {
                return Err(ProductionPipelineError::ConditionalFormalPolicy(
                    "wave-task and finite-join custody cannot be combined",
                ));
            }
            if target != fe2o3_amd_target::ProductionAmdTargetProfileV1::Gfx950 {
                return Err(ProductionPipelineError::ConditionalFormalPolicy(
                    "only gfx950 conditional extraction is supported",
                ));
            }
            let floor = fe2o3_lower_mir_kernel::ProductionConditionalWaveTaskFormalMemoryOwnerV1::source_storage(&lowered)
                .map_err(ProductionPipelineError::WaveTaskFormalMemory)?;
            let profile = wave_source_profile_v3(lowered.semantic().semantic())
                .map_err(ProductionPipelineError::ConditionalFormalPolicy)?;
            if profile == fe2o3_kernel_ir::SemanticWaveTaskProfileV3::QkvAttentionOutputV5 {
                return with_wave_formal_budget_v1(floor, |budget| {
                    fe2o3_lower_mir_kernel::ProductionConditionalWaveQkvAttentionOutputTaskFormalMemoryOwnerV5::try_retain_with_budget(lowered, budget)
                }).map(Self::ConditionalWaveQkvAttentionOutputTasksV5).map_err(ProductionPipelineError::WaveTaskFormalMemory);
            }
            if profile == fe2o3_kernel_ir::SemanticWaveTaskProfileV3::QkvAttentionOutputTilesV6 {
                return with_wave_formal_budget_v1(floor, |budget| {
                    fe2o3_lower_mir_kernel::ProductionConditionalWaveQkvAttentionOutputTileFormalMemoryOwnerV6::try_retain_with_budget(lowered, budget)
                }).map(Self::ConditionalWaveQkvAttentionOutputTilesV6).map_err(ProductionPipelineError::WaveTaskFormalMemory);
            }
            if profile == fe2o3_kernel_ir::SemanticWaveTaskProfileV3::MlpV1 {
                return with_wave_formal_budget_v1(floor, |budget| {
                    fe2o3_lower_mir_kernel::ProductionConditionalWaveMlpTaskFormalMemoryOwnerV1::try_retain_with_budget(lowered, budget)
                }).map(Self::ConditionalWaveMlpTasksV1).map_err(ProductionPipelineError::WaveTaskFormalMemory);
            }
            if profile == fe2o3_kernel_ir::SemanticWaveTaskProfileV3::MlpTilesV2 {
                return with_wave_formal_budget_v1(floor, |budget| {
                    fe2o3_lower_mir_kernel::ProductionConditionalWaveMlpTileFormalMemoryOwnerV2::try_retain_with_budget(lowered, budget)
                }).map(Self::ConditionalWaveMlpTilesV2).map_err(ProductionPipelineError::WaveTaskFormalMemory);
            }
            if profile == fe2o3_kernel_ir::SemanticWaveTaskProfileV3::MultiwaveJoinV1 {
                return with_wave_formal_budget_v1(floor, |budget| {
                    fe2o3_lower_mir_kernel::ProductionConditionalMultiwaveJoinFormalMemoryOwnerV1::try_retain_with_budget(lowered, budget)
                }).map(Self::ConditionalMultiwaveJoinV1).map_err(ProductionPipelineError::WaveTaskFormalMemory);
            }
            if profile == fe2o3_kernel_ir::SemanticWaveTaskProfileV3::QkvAttentionV4 {
                return with_wave_formal_budget_v1(floor, |budget| {
                    fe2o3_lower_mir_kernel::ProductionConditionalWaveQkvAttentionTaskFormalMemoryOwnerV4::try_retain_with_budget(lowered, budget)
                }).map(Self::ConditionalWaveQkvAttentionTasksV4).map_err(ProductionPipelineError::WaveTaskFormalMemory);
            }
            if profile == fe2o3_kernel_ir::SemanticWaveTaskProfileV3::QkvPostV3 {
                return with_wave_formal_budget_v1(floor, |budget| {
                    fe2o3_lower_mir_kernel::ProductionConditionalWaveQkvPostTaskFormalMemoryOwnerV3::try_retain_with_budget(lowered, budget)
                }).map(Self::ConditionalWaveQkvPostTasksV3).map_err(ProductionPipelineError::WaveTaskFormalMemory);
            }
            if profile == fe2o3_kernel_ir::SemanticWaveTaskProfileV3::QkvV2 {
                return with_wave_formal_budget_v1(floor, |budget| {
                    fe2o3_lower_mir_kernel::ProductionConditionalWaveQkvTaskFormalMemoryOwnerV2::try_retain_with_budget(lowered, budget)
                }).map(Self::ConditionalWaveQkvTasksV2).map_err(ProductionPipelineError::WaveTaskFormalMemory);
            }
            with_wave_formal_budget_v1(floor, |budget| {
                fe2o3_lower_mir_kernel::ProductionConditionalWaveTaskFormalMemoryOwnerV1::try_retain_with_budget(
                    lowered, budget,
                )
            })
            .map(Self::ConditionalWaveTasks)
            .map_err(ProductionPipelineError::WaveTaskFormalMemory)
        } else if conditional {
            if target != fe2o3_amd_target::ProductionAmdTargetProfileV1::Gfx950 {
                return Err(ProductionPipelineError::ConditionalFormalPolicy(
                    "only gfx950 conditional extraction is supported",
                ));
            }
            fe2o3_lower_mir_kernel::ProductionConditionalFiniteJoinFormalMemoryOwnerV1::try_retain(
                lowered,
            )
            .map(Self::ConditionalFiniteJoin)
            .map_err(ProductionPipelineError::ConditionalFormalMemory)
        } else {
            fe2o3_lower_mir_kernel::ProductionFormalMemoryOwnerV1::try_admit(lowered)
                .map(Self::Ordinary)
                .map_err(ProductionPipelineError::FormalMemoryAdmission)
        }
    }

    pub(crate) fn semantic_kir(&self) -> &fe2o3_lower_mir_kernel::ProductionSemanticKirOwnerV1 {
        match self {
            Self::Ordinary(owner) => owner.semantic_kir(),
            Self::ConditionalFiniteJoin(owner) => owner.semantic_kir(),
            Self::ConditionalWaveTasks(owner) => owner.semantic_kir(),
            Self::ConditionalWaveQkvTasksV2(owner) => owner.semantic_kir(),
            Self::ConditionalWaveQkvPostTasksV3(owner) => owner.semantic_kir(),
            Self::ConditionalWaveQkvAttentionTasksV4(owner) => owner.semantic_kir(),
            Self::ConditionalWaveQkvAttentionOutputTasksV5(owner) => owner.semantic_kir(),
            Self::ConditionalWaveQkvAttentionOutputTilesV6(owner) => owner.semantic_kir(),
            Self::ConditionalWaveMlpTasksV1(owner) => owner.semantic_kir(),
            Self::ConditionalWaveMlpTilesV2(owner) => owner.semantic_kir(),
            Self::ConditionalMultiwaveJoinV1(owner) => owner.semantic_kir(),
        }
    }

    pub(crate) fn verify_equivalence(&self) -> Result<(), ProductionPipelineError> {
        match self {
            Self::Ordinary(owner) => owner
                .verify_equivalence()
                .map_err(ProductionPipelineError::FormalMemoryAdmission),
            Self::ConditionalFiniteJoin(owner) => owner
                .verify_equivalence()
                .map_err(ProductionPipelineError::ConditionalFormalMemory),
            Self::ConditionalWaveTasks(owner) => {
                with_wave_formal_budget_v1(owner.retained_storage(), |budget| {
                    owner.verify_equivalence_with_budget(budget)
                })
                .map_err(ProductionPipelineError::WaveTaskFormalMemory)
            }
            Self::ConditionalWaveQkvTasksV2(owner) => {
                with_wave_formal_budget_v1(owner.retained_storage(), |budget| {
                    owner.verify_equivalence_with_budget(budget)
                })
                .map_err(ProductionPipelineError::WaveTaskFormalMemory)
            }
            Self::ConditionalWaveQkvPostTasksV3(owner) => {
                with_wave_formal_budget_v1(owner.retained_storage(), |budget| {
                    owner.verify_equivalence_with_budget(budget)
                })
                .map_err(ProductionPipelineError::WaveTaskFormalMemory)
            }
            Self::ConditionalWaveQkvAttentionTasksV4(owner) => {
                with_wave_formal_budget_v1(owner.retained_storage(), |budget| {
                    owner.verify_equivalence_with_budget(budget)
                })
                .map_err(ProductionPipelineError::WaveTaskFormalMemory)
            }
            Self::ConditionalWaveQkvAttentionOutputTasksV5(owner) => {
                with_wave_formal_budget_v1(owner.retained_storage(), |budget| {
                    owner.verify_equivalence_with_budget(budget)
                })
                .map_err(ProductionPipelineError::WaveTaskFormalMemory)
            }
            Self::ConditionalWaveQkvAttentionOutputTilesV6(owner) => {
                with_wave_formal_budget_v1(owner.retained_storage(), |budget| {
                    owner.verify_equivalence_with_budget(budget)
                })
                .map_err(ProductionPipelineError::WaveTaskFormalMemory)
            }
            Self::ConditionalWaveMlpTasksV1(owner) => {
                with_wave_formal_budget_v1(owner.retained_storage(), |budget| {
                    owner.verify_equivalence_with_budget(budget)
                })
                .map_err(ProductionPipelineError::WaveTaskFormalMemory)
            }
            Self::ConditionalWaveMlpTilesV2(owner) => {
                with_wave_formal_budget_v1(owner.retained_storage(), |budget| {
                    owner.verify_equivalence_with_budget(budget)
                })
                .map_err(ProductionPipelineError::WaveTaskFormalMemory)
            }
            Self::ConditionalMultiwaveJoinV1(owner) => {
                with_wave_formal_budget_v1(owner.retained_storage(), |budget| {
                    owner.verify_equivalence_with_budget(budget)
                })
                .map_err(ProductionPipelineError::WaveTaskFormalMemory)
            }
        }
    }

    fn require_ordinary_handoff(
        &self,
        detail: &'static str,
    ) -> Result<(), ProductionPipelineError> {
        match self {
            Self::Ordinary(_) => Ok(()),
            Self::ConditionalFiniteJoin(_)
            | Self::ConditionalWaveTasks(_)
            | Self::ConditionalWaveQkvTasksV2(_)
            | Self::ConditionalWaveQkvPostTasksV3(_)
            | Self::ConditionalWaveQkvAttentionTasksV4(_)
            | Self::ConditionalWaveMlpTasksV1(_)
            | Self::ConditionalWaveMlpTilesV2(_)
            | Self::ConditionalMultiwaveJoinV1(_)
            | Self::ConditionalWaveQkvAttentionOutputTasksV5(_)
            | Self::ConditionalWaveQkvAttentionOutputTilesV6(_) => {
                Err(ProductionPipelineError::ConditionalFormalPolicy(detail))
            }
        }
    }

    // These are descriptive raw-obligation statistics, never ordinary reports.
    fn raw_kernel_count(&self) -> usize {
        match self {
            Self::Ordinary(owner) => owner.kernels().len(),
            Self::ConditionalFiniteJoin(owner) => owner.semantic_kir().module().kernels.len(),
            Self::ConditionalWaveTasks(owner) => owner.semantic_kir().module().kernels.len(),
            Self::ConditionalWaveQkvTasksV2(owner) => owner.semantic_kir().module().kernels.len(),
            Self::ConditionalWaveQkvPostTasksV3(owner) => {
                owner.semantic_kir().module().kernels.len()
            }
            Self::ConditionalWaveQkvAttentionTasksV4(owner) => {
                owner.semantic_kir().module().kernels.len()
            }
            Self::ConditionalWaveQkvAttentionOutputTasksV5(owner) => {
                owner.semantic_kir().module().kernels.len()
            }
            Self::ConditionalWaveQkvAttentionOutputTilesV6(owner) => {
                owner.semantic_kir().module().kernels.len()
            }
            Self::ConditionalWaveMlpTasksV1(owner) => owner.semantic_kir().module().kernels.len(),
            Self::ConditionalWaveMlpTilesV2(owner) => owner.semantic_kir().module().kernels.len(),
            Self::ConditionalMultiwaveJoinV1(owner) => owner.semantic_kir().module().kernels.len(),
        }
    }

    fn raw_kernel_matches(&self, ordinal: usize, symbol: &str) -> bool {
        match self {
            Self::Ordinary(owner) => owner
                .kernels()
                .get(ordinal)
                .is_some_and(|kernel| kernel.obligations().kernel().as_str() == symbol),
            Self::ConditionalFiniteJoin(owner) => {
                ordinal == 0 && owner.raw_obligations().kernel().as_str() == symbol
            }
            Self::ConditionalWaveTasks(owner) => {
                ordinal == 0 && owner.raw_obligations().kernel().as_str() == symbol
            }
            Self::ConditionalWaveQkvTasksV2(owner) => {
                ordinal == 0 && owner.raw_obligations().kernel().as_str() == symbol
            }
            Self::ConditionalWaveQkvPostTasksV3(owner) => {
                ordinal == 0 && owner.raw_obligations().kernel().as_str() == symbol
            }
            Self::ConditionalWaveQkvAttentionTasksV4(owner) => {
                ordinal == 0 && owner.raw_obligations().kernel().as_str() == symbol
            }
            Self::ConditionalWaveQkvAttentionOutputTasksV5(owner) => {
                ordinal == 0 && owner.raw_obligations().kernel().as_str() == symbol
            }
            Self::ConditionalWaveQkvAttentionOutputTilesV6(owner) => {
                ordinal == 0 && owner.raw_obligations().kernel().as_str() == symbol
            }
            Self::ConditionalWaveMlpTasksV1(owner) => {
                ordinal == 0 && owner.raw_obligations().kernel().as_str() == symbol
            }
            Self::ConditionalWaveMlpTilesV2(owner) => {
                ordinal == 0 && owner.raw_obligations().kernel().as_str() == symbol
            }
            Self::ConditionalMultiwaveJoinV1(owner) => {
                ordinal == 0 && owner.raw_obligations().kernel().as_str() == symbol
            }
        }
    }

    fn raw_witness_extent(&self) -> u64 {
        match self {
            Self::Ordinary(owner) => owner.witness_extent(),
            Self::ConditionalFiniteJoin(owner) => owner.witness_extents()[0],
            Self::ConditionalWaveTasks(owner) => owner.witness_extents().into_iter().product(),
            Self::ConditionalWaveQkvTasksV2(owner) => owner.witness_extents().into_iter().product(),
            Self::ConditionalWaveQkvPostTasksV3(owner) => {
                owner.witness_extents().into_iter().product()
            }
            Self::ConditionalWaveQkvAttentionTasksV4(owner) => {
                owner.witness_extents().into_iter().product()
            }
            Self::ConditionalWaveQkvAttentionOutputTasksV5(owner) => {
                owner.witness_extents().into_iter().product()
            }
            Self::ConditionalWaveQkvAttentionOutputTilesV6(owner) => {
                owner.witness_extents().into_iter().product()
            }
            Self::ConditionalWaveMlpTasksV1(owner) => owner.witness_extents().into_iter().product(),
            Self::ConditionalWaveMlpTilesV2(owner) => owner.witness_extents().into_iter().product(),
            Self::ConditionalMultiwaveJoinV1(owner) => {
                owner.witness_extents().into_iter().product()
            }
        }
    }

    fn sum_raw_obligations(
        &self,
        count: fn(&fe2o3_kernel_ir::FormalMemoryObligations) -> usize,
    ) -> usize {
        match self {
            Self::Ordinary(owner) => owner
                .kernels()
                .iter()
                .map(|kernel| count(kernel.obligations()))
                .sum(),
            Self::ConditionalFiniteJoin(owner) => count(owner.raw_obligations()),
            Self::ConditionalWaveTasks(owner) => count(owner.raw_obligations()),
            Self::ConditionalWaveQkvTasksV2(owner) => count(owner.raw_obligations()),
            Self::ConditionalWaveQkvPostTasksV3(owner) => count(owner.raw_obligations()),
            Self::ConditionalWaveQkvAttentionTasksV4(owner) => count(owner.raw_obligations()),
            Self::ConditionalWaveQkvAttentionOutputTasksV5(owner) => count(owner.raw_obligations()),
            Self::ConditionalWaveQkvAttentionOutputTilesV6(owner) => count(owner.raw_obligations()),
            Self::ConditionalWaveMlpTasksV1(owner) => count(owner.raw_obligations()),
            Self::ConditionalWaveMlpTilesV2(owner) => count(owner.raw_obligations()),
            Self::ConditionalMultiwaveJoinV1(owner) => count(owner.raw_obligations()),
        }
    }
}
